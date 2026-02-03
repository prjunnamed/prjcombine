use prjcombine_entity::EntityVec;
use prjcombine_interconnect::{
    db::{BelInfo, SwitchBoxItem},
    grid::TileCoord,
};
use prjcombine_re_collector::diff::{Diff, DiffKey, OcdMode, xlat_bit, xlat_enum_raw};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::TileBit;
use prjcombine_xc2000::xc5200::{bcls, bslots, tcls, wires};
use prjcombine_xilinx_bitstream::BitRect;

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
            pip::PipInt,
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
        let id = fuzzer.info.features.pop().unwrap().key;
        for row in backend.edev.rows(tcrd.die) {
            if row == edev.chip.row_s() || row == edev.chip.row_n() {
                continue;
            }
            fuzzer.info.features.push(FuzzerFeature {
                key: id.clone(),
                rects: EntityVec::from_iter([BitRect::Null, edev.btile_main(tcrd.col, row)]),
            });
        }
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.edev.db;
    for (tcid, tcname, tcls) in &intdb.tile_classes {
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        let tcls_index = &backend.edev.db_index[tcid];
        for (&wire_to, ins) in &tcls_index.pips_bwd {
            for &wire_from in ins {
                if matches!(wire_from.wire, wires::IMUX_GIN | wires::IMUX_BUFG) {
                    continue;
                }
                if let Some(idx) = wires::IMUX_IO_O_SN.index_of(wire_to.wire) {
                    let mut bctx = ctx.bel(bslots::IO[idx]);
                    let mode = "IOB";
                    if wires::IMUX_IO_O.contains(wire_from.wire) {
                        bctx.mode(mode)
                            .attr("TMUX", "T")
                            .pin("O")
                            .pin("T")
                            .test_raw(DiffKey::Routing(tcid, wire_to, wire_from))
                            .pip("O", (PipInt, 0, wires::TIE_0))
                            .attr("OMUX", if wire_from.inv { "ONOT" } else { "O" })
                            .commit();
                    } else {
                        bctx.mode(mode)
                            .attr("TMUX", "T")
                            .pin("O")
                            .pin("T")
                            .test_raw(DiffKey::Routing(tcid, wire_to, wire_from))
                            .pip("O", (PipInt, 0, wire_from.wire))
                            .attr("OMUX", if wire_from.inv { "ONOT" } else { "O" })
                            .commit();
                    }
                    continue;
                }
                let mut builder = ctx
                    .build()
                    .test_raw(DiffKey::Routing(tcid, wire_to, wire_from))
                    .prop(WireIntDistinct::new(wire_to, wire_from.tw))
                    .prop(WireIntDstFilter::new(wire_to))
                    .prop(WireIntSrcFilter::new(wire_from.tw))
                    .prop(IntMutex::new("MAIN".to_string()))
                    .prop(WireMutexExclusive::new(wire_to))
                    .prop(WireMutexExclusive::new(wire_from.tw))
                    .prop(FuzzIntPip::new(wire_to, wire_from.tw));

                if let Some(rev) = tcls_index.pips_fwd.get(&wire_to)
                    && rev.contains(&wire_from.pos())
                {
                    if !tcls.bels.contains_id(bslots::TBUF[0]) {
                        if wires::LONG_H.contains(wire_from.wire) {
                            builder = builder.prop(DriveLLH::new(wire_from.tw));
                        } else if wires::LONG_V.contains(wire_from.wire) {
                            builder = builder.prop(DriveLLV::new(wire_from.tw));
                        } else {
                            panic!(
                                "AM HOUSECAT {tcname} {dst} {src}",
                                dst = wire_to.to_string(backend.edev.db, &backend.edev.db[tcid]),
                                src = wire_from.to_string(backend.edev.db, &backend.edev.db[tcid])
                            );
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
                                .prop(BaseIntPip::new(wire_from.tw, wire_help))
                                .prop(WireMutexExclusive::new(wire_from.tw))
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
                                        if help_b == wire_to || help_b == wire_from.tw {
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
                                    .prop(BaseIntPip::new(wire_from.tw, wire_help_a))
                                    .prop(BaseIntPip::new(wire_help_a, wire_help_b))
                                    .prop(WireMutexExclusive::new(wire_from.tw))
                                    .prop(WireMutexExclusive::new(wire_help_a))
                                    .prop(WireMutexExclusive::new(wire_help_b));
                            }
                        }
                    }
                }

                if wire_to.wire == wires::LONG_V[2] && matches!(tcid, tcls::LLV_W | tcls::LLV_E) {
                    builder = builder.prop(AllColumnIo);
                }

                builder.commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let intdb = ctx.edev.db;
    for (tcid, tcname, tcls) in &intdb.tile_classes {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        for (bslot, bel) in &tcls.bels {
            if matches!(bslot, bslots::BUFR | bslots::BUFG) {
                continue;
            }
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
            for item in &sb.items {
                match item {
                    SwitchBoxItem::Mux(mux) => {
                        let mut inps = vec![];
                        let mut got_empty = false;
                        for &wire_from in mux.src.keys() {
                            let diff =
                                ctx.get_diff_raw(&DiffKey::Routing(tcid, mux.dst, wire_from));
                            if diff.bits.is_empty() {
                                got_empty = true;
                            }
                            inps.push((Some(wire_from), diff));
                        }
                        for (rtile, rwire, rbel, rattr) in [
                            (
                                tcls::CNR_SE,
                                wires::IMUX_STARTUP_GTS,
                                bslots::STARTUP,
                                bcls::STARTUP::GTS_ENABLE,
                            ),
                            (
                                tcls::CNR_SE,
                                wires::IMUX_STARTUP_GRST,
                                bslots::STARTUP,
                                bcls::STARTUP::GR_ENABLE,
                            ),
                        ] {
                            if tcid == rtile && mux.dst.wire == rwire {
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
                                ctx.insert_bel_attr_bool(tcid, rbel, rattr, xlat_bit(common));
                            }
                        }
                        if !got_empty {
                            inps.push((None, Diff::default()));
                        }
                        let item = xlat_enum_raw(inps, OcdMode::Mux);
                        if item.bits.is_empty() {
                            println!(
                                "UMMM MUX {tcname} {mux_name} is empty",
                                mux_name = mux.dst.to_string(ctx.edev.db, &ctx.edev.db[tcid])
                            );
                        }
                        ctx.insert_mux(tcid, mux.dst, item);
                    }
                    SwitchBoxItem::Pass(pass) => {
                        let mut diff =
                            ctx.get_diff_raw(&DiffKey::Routing(tcid, pass.dst, pass.src.pos()));
                        // HORSEFUCKERS PISS SHIT FUCK
                        match (tcid, pass.src.wire) {
                            (tcls::CNR_SE, wires::OUT_STARTUP_DONEIN)
                                if pass.dst.wire == wires::LONG_V[0] =>
                            {
                                assert_eq!(diff.bits.len(), 2);
                                assert_eq!(diff.bits.remove(&TileBit::new(0, 6, 20)), Some(false));
                            }
                            (tcls::CNR_SE, wires::OUT_STARTUP_DONEIN)
                                if pass.dst.wire == wires::LONG_V[1] =>
                            {
                                assert_eq!(diff.bits.len(), 0);
                                diff.bits.insert(TileBit::new(0, 6, 20), false);
                            }
                            _ => (),
                        }
                        ctx.insert_pass(tcid, pass.dst, pass.src, xlat_bit(diff));
                    }
                    SwitchBoxItem::BiPass(pass) => {
                        ctx.collect_bipass(tcid, pass.a, pass.b);
                    }
                    SwitchBoxItem::PermaBuf(buf) => {
                        let diff = ctx.get_diff_raw(&DiffKey::Routing(tcid, buf.dst, buf.src));
                        diff.assert_empty();
                    }

                    _ => unreachable!(),
                }
            }
        }
    }
}
