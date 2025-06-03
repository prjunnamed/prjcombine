use prjcombine_interconnect::{db::WireKind, grid::NodeLoc};
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
            BaseIntPip, DriveLLH, DriveLLV, FuzzIntPip, NodeIntDistinct, NodeIntDstFilter,
            NodeIntSrcFilter,
        },
        props::{
            DynProp,
            mutex::{IntMutex, NodeMutexExclusive},
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
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Xc2000(edev) = backend.edev else {
            unreachable!()
        };
        let id = fuzzer.info.features.pop().unwrap().id;
        let (die, col, _, _) = nloc;
        for row in backend.egrid.die(die).rows() {
            if row == edev.chip.row_s() || row == edev.chip.row_n() {
                continue;
            }
            fuzzer.info.features.push(FuzzerFeature {
                id: id.clone(),
                tiles: vec![BitTile::Null, edev.btile_main(col, row)],
            });
        }
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.egrid.db;
    for (_, name, node) in &intdb.tile_classes {
        if node.muxes.is_empty() {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, name);
        for (&wire_to, mux) in &node.muxes {
            let mux_name = if node.cells.len() == 1 {
                format!("MUX.{}", intdb.wires.key(wire_to.1))
            } else {
                format!("MUX.{:#}.{}", wire_to.0, intdb.wires.key(wire_to.1))
            };
            for &wire_from in &mux.ins {
                let wire_from_name = intdb.wires.key(wire_from.1);
                let in_name = if node.cells.len() == 1 {
                    wire_from_name.to_string()
                } else {
                    format!("{:#}.{}", wire_from.0, wire_from_name)
                };
                if (name == "IO.B" || name == "IO.T")
                    && mux_name.contains("IMUX.IO")
                    && mux_name.ends_with('O')
                    && in_name.contains("OMUX")
                {
                    continue;
                }
                let mut builder = ctx
                    .build()
                    .test_manual("INT", &mux_name, &in_name)
                    .prop(NodeIntDistinct::new(wire_to, wire_from))
                    .prop(NodeIntDstFilter::new(wire_to))
                    .prop(NodeIntSrcFilter::new(wire_from))
                    .prop(IntMutex::new("MAIN".to_string()))
                    .prop(NodeMutexExclusive::new(wire_to))
                    .prop(NodeMutexExclusive::new(wire_from))
                    .prop(FuzzIntPip::new(wire_to, wire_from));

                if let Some(inmux) = node.muxes.get(&wire_from) {
                    if inmux.ins.contains(&wire_to) {
                        if name.starts_with("CLK") || name.starts_with("CNR") {
                            if wire_from_name.starts_with("LONG.H") {
                                builder = builder.prop(DriveLLH::new(wire_from));
                            } else if wire_from_name.starts_with("LONG.V") {
                                builder = builder.prop(DriveLLV::new(wire_from));
                            } else {
                                panic!("AM HOUSECAT {name} {mux_name} {in_name}");
                            }
                        } else {
                            let mut wire_help = None;
                            for &help in &inmux.ins {
                                if let Some(helpmux) = node.muxes.get(&help) {
                                    if helpmux.ins.contains(&wire_from) {
                                        continue;
                                    }
                                }
                                // println!("HELP {} <- {} <- {}", intdb.wires.key(wire_to.1), intdb.wires.key(wire_from.1), intdb.wires.key(help.1));
                                wire_help = Some(help);
                                break;
                            }
                            if let Some(wire_help) = wire_help {
                                builder = builder
                                    .prop(BaseIntPip::new(wire_from, wire_help))
                                    .prop(NodeMutexExclusive::new(wire_from))
                                    .prop(NodeMutexExclusive::new(wire_help));
                            } else {
                                let mut wire_help_a = None;
                                let mut wire_help_b = None;
                                'help_ab: for &help_a in &inmux.ins {
                                    if help_a == wire_to {
                                        continue;
                                    }
                                    if let Some(helpmux_a) = node.muxes.get(&help_a) {
                                        for &help_b in &helpmux_a.ins {
                                            if help_b == wire_to || help_b == wire_from {
                                                continue;
                                            }
                                            if let Some(helpmux_b) = node.muxes.get(&help_b) {
                                                if helpmux_b.ins.contains(&help_a) {
                                                    continue;
                                                }
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
                                        .prop(NodeMutexExclusive::new(wire_from))
                                        .prop(NodeMutexExclusive::new(wire_help_a))
                                        .prop(NodeMutexExclusive::new(wire_help_b));
                                }
                            }
                        }
                    }
                }

                if mux_name.contains("LONG.V2") && (name == "CLKL" || name == "CLKR") {
                    builder = builder.prop(AllColumnIo);
                }

                builder.commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let egrid = ctx.edev.egrid();
    let intdb = egrid.db;
    for (node_kind, tile, node) in &intdb.tile_classes {
        if node.muxes.is_empty() {
            continue;
        }
        if egrid.tile_index[node_kind].is_empty() {
            continue;
        }

        for (&wire_to, mux) in &node.muxes {
            if intdb.wires[wire_to.1] != WireKind::MuxOut {
                let out_name = if node.cells.len() == 1 {
                    intdb.wires.key(wire_to.1).to_string()
                } else {
                    format!("{:#}.{}", wire_to.0, intdb.wires.key(wire_to.1))
                };
                for &wire_from in &mux.ins {
                    let in_name = if node.cells.len() == 1 {
                        intdb.wires.key(wire_from.1).to_string()
                    } else {
                        format!("{:#}.{}", wire_from.0, intdb.wires.key(wire_from.1))
                    };
                    let mut diff =
                        ctx.state
                            .get_diff(tile, "INT", format!("MUX.{out_name}"), &in_name);
                    // HORSEFUCKERS PISS SHIT FUCK
                    match (&tile[..], &out_name[..], &in_name[..]) {
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
                    let mut is_bidi = false;
                    if let Some(omux) = node.muxes.get(&wire_from) {
                        if omux.ins.contains(&wire_to) {
                            is_bidi = true;
                        }
                    }
                    let name = if !is_bidi {
                        format!("PASS.{out_name}.{in_name}")
                    } else if wire_from < wire_to {
                        format!("BIPASS.{in_name}.{out_name}")
                    } else {
                        format!("BIPASS.{out_name}.{in_name}")
                    };
                    ctx.tiledb.insert(tile, "INT", name, item);
                }
            } else {
                let out_name = if node.cells.len() == 1 {
                    intdb.wires.key(wire_to.1).to_string()
                } else {
                    format!("{:#}.{}", wire_to.0, intdb.wires.key(wire_to.1))
                };
                let mux_name = format!("MUX.{out_name}");

                let mut inps = vec![];
                let mut got_empty = false;
                for &wire_from in &mux.ins {
                    let in_name = if node.cells.len() == 1 {
                        intdb.wires.key(wire_from.1).to_string()
                    } else {
                        format!("{:#}.{}", wire_from.0, intdb.wires.key(wire_from.1))
                    };
                    if (tile == "IO.B" || tile == "IO.T")
                        && mux_name.contains("IMUX.IO")
                        && mux_name.ends_with('O')
                        && in_name.contains("OMUX")
                    {
                        continue;
                    }
                    let diff = ctx.state.get_diff(tile, "INT", &mux_name, &in_name);
                    if diff.bits.is_empty() {
                        got_empty = true;
                    }
                    inps.push((in_name.to_string(), diff));
                }
                for (rtile, rwire, rbel, rattr) in [
                    ("CNR.BR", "IMUX.STARTUP.GTS", "STARTUP", "ENABLE.GTS"),
                    ("CNR.BR", "IMUX.STARTUP.GRST", "STARTUP", "ENABLE.GR"),
                ] {
                    if tile == rtile && out_name == rwire {
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
                        ctx.tiledb.insert(tile, rbel, rattr, xlat_bit(common));
                    }
                }
                if !got_empty {
                    inps.push(("NONE".to_string(), Diff::default()));
                }
                let item = xlat_enum_ocd(inps, OcdMode::Mux);
                if item.bits.is_empty() {
                    println!("UMMM MUX {tile} {mux_name} is empty");
                }
                ctx.tiledb.insert(tile, "INT", mux_name, item);
            }
        }
    }
}
