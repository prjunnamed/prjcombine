use prjcombine_interconnect::{
    db::{BelInfo, SwitchBoxItem},
    grid::TileCoord,
};
use prjcombine_re_fpga_hammer::{
    Diff, FuzzerFeature, FuzzerProp, OcdMode, xlat_bit, xlat_enum_ocd,
};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::TileBit;
use prjcombine_xilinx_bitstream::BitTile;

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::FuzzCtx,
        int::{
            BaseIntPip, DriveLLH, DriveLLV, FuzzIntPip, WireIntDistinct, WireIntDstFilter,
            WireIntSrcFilter,
        },
        props::{
            DynProp,
            mutex::{IntMutex, WireMutexExclusive},
        },
    },
};

#[derive(Clone, Debug)]
struct AllColumnIo;

impl<'b> FuzzerProp<'b, IseBackend<'b>> for AllColumnIo {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Xc2000(edev) = backend.edev else {
            unreachable!()
        };
        let id = fuzzer.info.features.pop().unwrap().id;
        for row in backend.edev.rows(tcrd.die) {
            if row == edev.chip.row_s() || row == edev.chip.row_n() {
                continue;
            }
            fuzzer.info.features.push(FuzzerFeature {
                id: id.clone(),
                tiles: vec![BitTile::Null, edev.btile_main(tcrd.col, row)],
            });
        }
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.edev.db;
    for (tcid, tcname, tcls) in &intdb.tile_classes {
        let mut ctx = FuzzCtx::new(session, backend, tcname);
        let tcls_index = &backend.edev.db_index[tcid];
        for (&wire_to, ins) in &tcls_index.pips_bwd {
            let mux_name = if tcls.cells.len() == 1 {
                format!("MUX.{}", intdb.wires.key(wire_to.wire))
            } else {
                format!("MUX.{:#}.{}", wire_to.cell, intdb.wires.key(wire_to.wire))
            };
            for &wire_from in ins {
                let wire_from = wire_from.tw;
                let wire_from_name = intdb.wires.key(wire_from.wire);
                let in_name = if tcls.cells.len() == 1 {
                    wire_from_name.to_string()
                } else {
                    format!("{:#}.{}", wire_from.cell, wire_from_name)
                };
                if (tcname == "IO.B" || tcname == "IO.T")
                    && mux_name.contains("IMUX.IO")
                    && mux_name.ends_with('O')
                    && in_name.contains("OMUX")
                {
                    continue;
                }
                let mut builder = ctx
                    .build()
                    .test_manual("INT", &mux_name, &in_name)
                    .prop(WireIntDistinct::new(wire_to, wire_from))
                    .prop(WireIntDstFilter::new(wire_to))
                    .prop(WireIntSrcFilter::new(wire_from))
                    .prop(IntMutex::new("MAIN".to_string()))
                    .prop(WireMutexExclusive::new(wire_to))
                    .prop(WireMutexExclusive::new(wire_from))
                    .prop(FuzzIntPip::new(wire_to, wire_from));

                if let Some(rev) = tcls_index.pips_fwd.get(&wire_to)
                    && rev.contains(&wire_from.pos())
                {
                    if tcname.starts_with("CLK") || tcname.starts_with("CNR") {
                        if wire_from_name.starts_with("LONG.H") {
                            builder = builder.prop(DriveLLH::new(wire_from));
                        } else if wire_from_name.starts_with("LONG.V") {
                            builder = builder.prop(DriveLLV::new(wire_from));
                        } else {
                            panic!("AM HOUSECAT {tcname} {mux_name} {in_name}");
                        }
                    } else {
                        let mut wire_help = None;
                        for &help in &tcls_index.pips_bwd[&wire_from] {
                            if tcls_index.pips_fwd[&wire_from].contains(&help) {
                                continue;
                            }
                            // println!("HELP {} <- {} <- {}", intdb.wires.key(wire_to.1), intdb.wires.key(wire_from.1), intdb.wires.key(help.1));
                            wire_help = Some(help.tw);
                            break;
                        }
                        if let Some(wire_help) = wire_help {
                            builder = builder
                                .prop(BaseIntPip::new(wire_from, wire_help))
                                .prop(WireMutexExclusive::new(wire_from))
                                .prop(WireMutexExclusive::new(wire_help));
                        } else {
                            let mut wire_help_a = None;
                            let mut wire_help_b = None;
                            'help_ab: for &help_a in &tcls_index.pips_bwd[&wire_from] {
                                let help_a = help_a.tw;
                                if help_a == wire_to {
                                    continue;
                                }
                                if let Some(helpmux_a) = tcls_index.pips_bwd.get(&help_a) {
                                    for &help_b in helpmux_a {
                                        let help_b = help_b.tw;
                                        if help_b == wire_to || help_b == wire_from {
                                            continue;
                                        }
                                        if let Some(helpmux_b) = tcls_index.pips_bwd.get(&help_b)
                                            && helpmux_b.contains(&help_a.pos())
                                        {
                                            continue;
                                        }
                                        wire_help_a = Some(help_a);
                                        wire_help_b = Some(help_b);
                                        break 'help_ab;
                                    }
                                }
                            }
                            if let (Some(wire_help_a), Some(wire_help_b)) =
                                (wire_help_a, wire_help_b)
                            {
                                builder = builder
                                    .prop(BaseIntPip::new(wire_from, wire_help_a))
                                    .prop(BaseIntPip::new(wire_help_a, wire_help_b))
                                    .prop(WireMutexExclusive::new(wire_from))
                                    .prop(WireMutexExclusive::new(wire_help_a))
                                    .prop(WireMutexExclusive::new(wire_help_b));
                            }
                        }
                    }
                }

                if mux_name.contains("LONG.V2") && (tcname == "CLKL" || tcname == "CLKR") {
                    builder = builder.prop(AllColumnIo);
                }

                builder.commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let intdb = ctx.edev.db;
    for (_, tcname, tcls) in &intdb.tile_classes {
        if !ctx.has_tile(tcname) {
            continue;
        }
        for (bslot, bel) in &tcls.bels {
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
            let bel = intdb.bel_slots.key(bslot);
            for item in &sb.items {
                match item {
                    SwitchBoxItem::Mux(mux) => {
                        let out_name = if tcls.cells.len() == 1 {
                            intdb.wires.key(mux.dst.wire).to_string()
                        } else {
                            format!("{:#}.{}", mux.dst.cell, intdb.wires.key(mux.dst.wire))
                        };
                        let mux_name = format!("MUX.{out_name}");

                        let mut inps = vec![];
                        let mut got_empty = false;
                        for &wire_from in &mux.src {
                            let wire_from = wire_from.tw;
                            let in_name = if tcls.cells.len() == 1 {
                                intdb.wires.key(wire_from.wire).to_string()
                            } else {
                                format!("{:#}.{}", wire_from.cell, intdb.wires.key(wire_from.wire))
                            };
                            if (tcname == "IO.B" || tcname == "IO.T")
                                && mux_name.contains("IMUX.IO")
                                && mux_name.ends_with('O')
                                && in_name.contains("OMUX")
                            {
                                continue;
                            }
                            let diff = ctx.state.get_diff(tcname, "INT", &mux_name, &in_name);
                            if diff.bits.is_empty() {
                                got_empty = true;
                            }
                            inps.push((in_name.to_string(), diff));
                        }
                        for (rtile, rwire, rbel, rattr) in [
                            ("CNR.BR", "IMUX.STARTUP.GTS", "STARTUP", "ENABLE.GTS"),
                            ("CNR.BR", "IMUX.STARTUP.GRST", "STARTUP", "ENABLE.GR"),
                        ] {
                            if tcname == rtile && out_name == rwire {
                                let mut common = inps[0].1.clone();
                                for (_, diff) in &inps {
                                    common.bits.retain(|bit, _| diff.bits.contains_key(bit));
                                }
                                assert_eq!(common.bits.len(), 1);
                                for (_, diff) in &mut inps {
                                    *diff = diff.combine(&!&common);
                                    if diff.bits.is_empty() {
                                        got_empty = true;
                                    }
                                }
                                assert!(got_empty);
                                ctx.tiledb.insert(tcname, rbel, rattr, xlat_bit(common));
                            }
                        }
                        if !got_empty {
                            inps.push(("NONE".to_string(), Diff::default()));
                        }
                        let item = xlat_enum_ocd(inps, OcdMode::Mux);
                        if item.bits.is_empty() {
                            println!("UMMM MUX {tcname} {mux_name} is empty");
                        }
                        ctx.tiledb.insert(tcname, bel, mux_name, item);
                    }
                    SwitchBoxItem::Pass(pass) => {
                        let out_name = if tcls.cells.len() == 1 {
                            intdb.wires.key(pass.dst.wire).to_string()
                        } else {
                            format!("{:#}.{}", pass.dst.cell, intdb.wires.key(pass.dst.wire))
                        };
                        let in_name = if tcls.cells.len() == 1 {
                            intdb.wires.key(pass.src.wire).to_string()
                        } else {
                            format!("{:#}.{}", pass.src.cell, intdb.wires.key(pass.src.wire))
                        };
                        let mut diff =
                            ctx.state
                                .get_diff(tcname, "INT", format!("MUX.{out_name}"), &in_name);
                        // HORSEFUCKERS PISS SHIT FUCK
                        match (&tcname[..], &out_name[..], &in_name[..]) {
                            ("CNR.BR", "LONG.V0", "OUT.STARTUP.DONEIN") => {
                                assert_eq!(diff.bits.len(), 2);
                                assert_eq!(diff.bits.remove(&TileBit::new(0, 6, 20)), Some(false));
                            }
                            ("CNR.BR", "LONG.V1", "OUT.STARTUP.DONEIN") => {
                                assert_eq!(diff.bits.len(), 0);
                                diff.bits.insert(TileBit::new(0, 6, 20), false);
                            }
                            _ => (),
                        }
                        let item = xlat_bit(diff);
                        let name = format!("PASS.{out_name}.{in_name}");
                        ctx.tiledb.insert(tcname, bel, name, item);
                    }
                    SwitchBoxItem::BiPass(pass) => {
                        let a_name = intdb.wires.key(pass.a.wire);
                        let b_name = intdb.wires.key(pass.b.wire);
                        let name = if tcls.cells.len() == 1 {
                            format!("BIPASS.{a_name}.{b_name}")
                        } else {
                            format!(
                                "BIPASS.{a_cell:#}.{a_name}.{b_cell:#}.{b_name}",
                                a_cell = pass.a.cell,
                                b_cell = pass.b.cell,
                            )
                        };
                        for (wdst, wsrc) in [(pass.a, pass.b), (pass.b, pass.a)] {
                            let out_name = if tcls.cells.len() == 1 {
                                intdb.wires.key(wdst.wire).to_string()
                            } else {
                                format!("{:#}.{}", wdst.cell, intdb.wires.key(wdst.wire))
                            };
                            let in_name = if tcls.cells.len() == 1 {
                                intdb.wires.key(wsrc.wire).to_string()
                            } else {
                                format!("{:#}.{}", wsrc.cell, intdb.wires.key(wsrc.wire))
                            };
                            let diff = ctx.state.get_diff(
                                tcname,
                                "INT",
                                format!("MUX.{out_name}"),
                                &in_name,
                            );
                            let item = xlat_bit(diff);
                            ctx.tiledb.insert(tcname, bel, &name, item);
                        }
                    }
                    SwitchBoxItem::PermaBuf(buf) => {
                        let out_name = if tcls.cells.len() == 1 {
                            intdb.wires.key(buf.dst.wire).to_string()
                        } else {
                            format!("{:#}.{}", buf.dst.cell, intdb.wires.key(buf.dst.wire))
                        };
                        let in_name = if tcls.cells.len() == 1 {
                            intdb.wires.key(buf.src.wire).to_string()
                        } else {
                            format!("{:#}.{}", buf.src.cell, intdb.wires.key(buf.src.wire))
                        };
                        let diff =
                            ctx.state
                                .get_diff(tcname, "INT", format!("MUX.{out_name}"), &in_name);
                        diff.assert_empty();
                    }

                    _ => unreachable!(),
                }
            }
        }
    }
}
