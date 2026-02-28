use std::collections::BTreeSet;

use prjcombine_entity::EntityId;
use prjcombine_interconnect::db::{BelAttributeType, WireSlotIdExt};
use prjcombine_re_collector::{
    diff::{Diff, OcdMode, extract_common_diff, xlat_bit, xlat_bit_wide, xlat_enum_raw},
    legacy::{extract_bitvec_val_part_legacy, xlat_bitvec_legacy},
};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{
    bitvec::BitVec,
    bsdata::{BitRectId, TileBit, TileItem},
};
use prjcombine_virtex4::defs::{
    bcls::{self, BUFHCE, IN_FIFO, OUT_FIFO, PHASER_IN, PHASER_OUT, PHASER_REF, PHY_CONTROL},
    bslots, enums,
    virtex7::{tcls, wires},
};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::{BaseIntPip, FuzzIntPip},
        props::{
            DynProp,
            extra::{ExtraKeyRouting, ExtraTileMaybe},
            mutex::WireMutexExclusive,
            pip::{BasePip, FuzzPip, PipWire},
            relation::{Delta, NoopRelation, Related},
        },
    },
    virtex4::specials,
};

use super::{clk::ColPair, gt::TouchHout};

fn add_fuzzers_fifo<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::CMT_FIFO);
    {
        let mut bctx = ctx.bel(bslots::IN_FIFO);
        let mode = "IN_FIFO";
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();

        bctx.mode(mode).test_bel_attr_auto_default(
            IN_FIFO::ALMOST_EMPTY_VALUE,
            enums::IO_FIFO_WATERMARK::NONE,
        );
        bctx.mode(mode)
            .test_bel_attr_auto_default(IN_FIFO::ALMOST_FULL_VALUE, enums::IO_FIFO_WATERMARK::NONE);
        bctx.mode(mode).test_bel_attr_auto(IN_FIFO::ARRAY_MODE);
        bctx.mode(mode)
            .test_bel_attr_bool_auto(IN_FIFO::SLOW_RD_CLK, "FALSE", "TRUE");
        bctx.mode(mode)
            .test_bel_attr_bool_auto(IN_FIFO::SLOW_WR_CLK, "FALSE", "TRUE");
        bctx.mode(mode)
            .test_bel_attr_bool_auto(IN_FIFO::SYNCHRONOUS_MODE, "FALSE", "TRUE");
        bctx.mode(mode)
            .test_bel_attr_multi(IN_FIFO::SPARE, MultiValue::Bin);
    }
    {
        let mut bctx = ctx.bel(bslots::OUT_FIFO);
        let mode = "OUT_FIFO";
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();

        bctx.mode(mode).test_bel_attr_auto_default(
            OUT_FIFO::ALMOST_EMPTY_VALUE,
            enums::IO_FIFO_WATERMARK::NONE,
        );
        bctx.mode(mode).test_bel_attr_auto_default(
            OUT_FIFO::ALMOST_FULL_VALUE,
            enums::IO_FIFO_WATERMARK::NONE,
        );
        bctx.mode(mode).test_bel_attr_auto(OUT_FIFO::ARRAY_MODE);
        bctx.mode(mode)
            .test_bel_attr_bool_auto(OUT_FIFO::SLOW_RD_CLK, "FALSE", "TRUE");
        bctx.mode(mode)
            .test_bel_attr_bool_auto(OUT_FIFO::SLOW_WR_CLK, "FALSE", "TRUE");
        bctx.mode(mode)
            .test_bel_attr_bool_auto(OUT_FIFO::SYNCHRONOUS_MODE, "FALSE", "TRUE");
        bctx.mode(mode)
            .test_bel_attr_bool_auto(OUT_FIFO::OUTPUT_DISABLE, "FALSE", "TRUE");
        bctx.mode(mode)
            .test_bel_attr_multi(OUT_FIFO::SPARE, MultiValue::Bin);
    }
}

fn add_fuzzers_routing<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    let mut ctx = FuzzCtx::new(session, backend, tcls::CMT);

    // IMUX_BUFMRCE
    for o in 0..2 {
        let dst = wires::IMUX_BUFMRCE[o].cell(25);
        let odst = wires::IMUX_BUFMRCE[o ^ 1].cell(25);
        for i in 4..14 {
            let src = wires::HROW_I_CMT[i].cell(25);
            ctx.build()
                .tile_mutex("HIN", "USE")
                .prop(WireMutexExclusive::new(dst))
                .prop(WireMutexExclusive::new(odst))
                .prop(BaseIntPip::new(odst, src))
                .test_routing(dst, src.pos())
                .prop(FuzzIntPip::new(dst, src))
                .commit();
        }
        for i in 0..2 {
            let src = wires::CKINT_CMT[i].cell(25);
            ctx.build()
                .tile_mutex("CKINT", "USE")
                .prop(WireMutexExclusive::new(dst))
                .prop(WireMutexExclusive::new(odst))
                .prop(BaseIntPip::new(odst, src))
                .test_routing(dst, src.pos())
                .prop(FuzzIntPip::new(dst, src))
                .commit();
        }
        let src = wires::CCIO_CMT[o * 3].cell(25);
        let odst = wires::HROW_O[0].cell(25);
        ctx.build()
            .tile_mutex("CCIO", "USE")
            .prop(WireMutexExclusive::new(dst))
            .prop(WireMutexExclusive::new(odst))
            .prop(BaseIntPip::new(odst, src))
            .prop(TouchHout(0))
            .test_routing(dst, src.pos())
            .prop(FuzzIntPip::new(dst, src))
            .commit();
    }

    for w in [
        wires::IMUX_PLL_CLKIN1_HCLK,
        wires::IMUX_PLL_CLKIN2_HCLK,
        wires::IMUX_PLL_CLKFB_HCLK,
        wires::LCLK_CMT_S,
        wires::LCLK_CMT_N,
    ] {
        for o in 0..2 {
            let dst = w[o].cell(25);
            let odst = w[o ^ 1].cell(25);
            for &src in backend.edev.db_index[tcls::CMT].muxes[&dst].src.keys() {
                let mut builder = ctx.build();

                if wires::HCLK_CMT.contains(src.wire) {
                    builder = builder.global_mutex("HCLK", "USE");
                } else if wires::RCLK_CMT.contains(src.wire) {
                    builder = builder.global_mutex("RCLK", "USE");
                } else if wires::HROW_I_CMT.contains(src.wire) {
                    builder = builder.tile_mutex("HIN", "USE");
                } else if let Some(idx) = wires::OMUX_CCIO.index_of(src.wire) {
                    let src_ccio = wires::CCIO_CMT[idx].cell(25);
                    ctx.build()
                        .tile_mutex("CCIO", "USE")
                        .tile_mutex("PHASER_REF_BOUNCE", "CCIO")
                        .prop(WireMutexExclusive::new(dst))
                        .prop(WireMutexExclusive::new(odst))
                        .prop(BaseIntPip::new(odst, src_ccio))
                        .test_routing(dst, src)
                        .prop(FuzzIntPip::new(dst, src_ccio))
                        .commit();
                    builder = ctx.build().tile_mutex("PHASER_REF_BOUNCE", "USE")
                } else {
                    unreachable!()
                }
                builder
                    .prop(WireMutexExclusive::new(dst))
                    .prop(WireMutexExclusive::new(odst))
                    .prop(BaseIntPip::new(odst, src.tw))
                    .test_routing(dst, src)
                    .prop(FuzzIntPip::new(dst, src.tw))
                    .commit();
            }
        }
    }

    // HROW_O
    for o in 0..14 {
        let dst = wires::HROW_O[o].cell(25);
        let odst = wires::HROW_O[o ^ 1].cell(25);

        for &src in backend.edev.db_index[tcls::CMT].muxes[&dst].src.keys() {
            let mut builder = ctx.build().prop(TouchHout(o)).prop(TouchHout(o ^ 1));

            if wires::HCLK_CMT.contains(src.wire) {
                builder = builder.global_mutex("HCLK", "USE");
            } else if wires::HROW_I_CMT.contains(src.wire) {
                builder = builder.tile_mutex("HIN", "USE");
            } else if let Some(idx) = wires::OMUX_CCIO.index_of(src.wire) {
                let src_ccio = wires::CCIO_CMT[idx].cell(25);
                ctx.build()
                    .prop(TouchHout(o))
                    .prop(TouchHout(o ^ 1))
                    .tile_mutex("CCIO", "USE")
                    .tile_mutex("PHASER_REF_BOUNCE", "CCIO")
                    .prop(WireMutexExclusive::new(dst))
                    .prop(WireMutexExclusive::new(odst))
                    .prop(BaseIntPip::new(odst, src_ccio))
                    .test_routing(dst, src)
                    .prop(FuzzIntPip::new(dst, src_ccio))
                    .commit();
                builder = ctx
                    .build()
                    .prop(TouchHout(o))
                    .prop(TouchHout(o ^ 1))
                    .tile_mutex("PHASER_REF_BOUNCE", "USE")
            } else {
                ctx.build()
                    .prop(TouchHout(o))
                    .prop(WireMutexExclusive::new(dst))
                    .test_routing(dst, src)
                    .prop(FuzzIntPip::new(dst, src.tw))
                    .commit();
                continue;
            }
            builder
                .prop(WireMutexExclusive::new(dst))
                .prop(WireMutexExclusive::new(odst))
                .prop(BaseIntPip::new(odst, src.tw))
                .test_routing(dst, src)
                .prop(FuzzIntPip::new(dst, src.tw))
                .commit();

            if o == 0 {
                let mut builder = ctx.build().prop(TouchHout(o));

                if wires::HCLK_CMT.contains(src.wire) {
                    builder = builder.global_mutex("HCLK", "TEST");
                } else if wires::HROW_I_CMT.contains(src.wire) {
                    builder = builder.tile_mutex("HIN", "TEST");
                } else if let Some(idx) = wires::OMUX_CCIO.index_of(src.wire) {
                    let src_ccio = wires::CCIO_CMT[idx].cell(25);
                    ctx.build()
                        .prop(TouchHout(o))
                        .tile_mutex("CCIO", "TEST")
                        .tile_mutex("PHASER_REF_BOUNCE", "CCIO")
                        .prop(WireMutexExclusive::new(dst))
                        .test_routing_pair_special(dst, src_ccio.pos(), specials::CMT_BUF)
                        .prop(FuzzIntPip::new(dst, src_ccio))
                        .commit();
                    continue;
                } else {
                    unreachable!()
                }

                builder
                    .prop(WireMutexExclusive::new(dst))
                    .test_routing_pair_special(dst, src, specials::CMT_BUF)
                    .prop(FuzzIntPip::new(dst, src.tw))
                    .commit();
            }
        }
    }

    for wt in [
        wires::IMUX_PLL_CLKIN1,
        wires::IMUX_PLL_CLKIN2,
        wires::IMUX_PLL_CLKFB,
    ]
    .into_iter()
    .flatten()
    .chain(wires::IMUX_PHASER_IN_PHASEREFCLK)
    .chain(wires::IMUX_PHASER_OUT_PHASEREFCLK)
    {
        let dst = wt.cell(25);
        for &src in backend.edev.db_index[tcls::CMT].muxes[&dst].src.keys() {
            ctx.build()
                .prop(WireMutexExclusive::new(dst))
                .test_routing(dst, src)
                .prop(FuzzIntPip::new(dst, src.tw))
                .commit();
        }
    }

    // OMUX_PLL_PERF
    for i in 0..4 {
        let far_dst = wires::PERF[i].cell(75);
        let far_src = wires::PERF_IN_PLL[i].cell(25);
        let dst = wires::OMUX_PLL_PERF[i].cell(25);
        for &src in backend.edev.db_index[tcls::CMT].muxes[&dst].src.keys() {
            ctx.build()
                .tile_mutex("PERF", "TEST")
                .prop(WireMutexExclusive::new(far_dst))
                .prop(WireMutexExclusive::new(far_src))
                .prop(WireMutexExclusive::new(dst))
                .prop(BaseIntPip::new(far_dst, far_src))
                .test_routing(dst, src)
                .prop(FuzzIntPip::new(dst, src.tw))
                .commit();
        }
    }

    for wt in [
        wires::OUT_PLL_FREQ_BB_S,
        wires::OUT_PLL_FREQ_BB_N,
        wires::CKINT_CMT,
    ]
    .into_iter()
    .flatten()
    {
        let dst = wt.cell(25);
        let src = backend.edev.db_index[tcls::CMT].only_bwd(dst);
        ctx.build()
            .tile_mutex("CKINT", "TEST")
            .test_routing(dst, src)
            .prop(FuzzIntPip::new(dst, src.tw))
            .commit();
    }

    // OMUX_CCIO
    for i in 0..4 {
        let dst = wires::OMUX_CCIO[i].cell(25);
        for wf in [wires::OUT_PHASER_REF_CLKOUT, wires::OUT_PHASER_REF_TMUXOUT] {
            let src = wf.cell(25);
            ctx.build()
                .tile_mutex("PHASER_REF_BOUNCE", "TEST")
                .prop(WireMutexExclusive::new(dst))
                .test_routing(dst, src.pos())
                .prop(FuzzIntPip::new(dst, src))
                .commit();
        }
    }

    // OMUX_HCLK_FREQ_BB
    for i in 0..4 {
        let dst = wires::OMUX_HCLK_FREQ_BB[i].cell(25);
        let src_ckint = wires::CKINT_CMT[i].cell(25);
        let src_ckint_far = backend.edev.db_index[tcls::CMT].only_bwd(src_ckint);
        ctx.build()
            .tile_mutex("FREQ_BB", "DRIVE_HCLK")
            .tile_mutex("CKINT", "USE")
            .prop(WireMutexExclusive::new(dst))
            .prop(BaseIntPip::new(src_ckint, src_ckint_far.tw))
            .test_routing(dst, src_ckint.pos())
            .prop(FuzzIntPip::new(dst, src_ckint))
            .commit();
        let src_ccio = wires::CCIO_CMT[i].cell(25);
        ctx.build()
            .tile_mutex("FREQ_BB", "DRIVE_HCLK")
            .tile_mutex("CCIO", "TEST_FREQ_BB")
            .prop(WireMutexExclusive::new(dst))
            .test_routing(dst, src_ccio.pos())
            .prop(FuzzIntPip::new(dst, src_ccio))
            .commit();
    }

    // OMUX_PLL_FREQ_BB_*
    for o in 0..4 {
        for (wt, wf, wp, step) in [
            (
                wires::OMUX_PLL_FREQ_BB_S,
                wires::OUT_PLL_FREQ_BB_S,
                wires::OUT_PLL_S.as_slice(),
                2,
            ),
            (
                wires::OMUX_PLL_FREQ_BB_N,
                wires::OUT_PLL_FREQ_BB_N,
                wires::OUT_PLL_N.as_slice(),
                1,
            ),
        ] {
            let fbb = wires::CMT_FREQ_BB[o].cell(25);
            let dst = wt[o].cell(25);
            for i in 0..4 {
                let src = wf[i].cell(25);
                let po = wp[i * step].cell(25);
                ctx.build()
                    .tile_mutex("FREQ_BB", "DRIVE_PLL")
                    .prop(WireMutexExclusive::new(fbb))
                    .prop(WireMutexExclusive::new(dst))
                    .prop(BaseIntPip::new(src, po))
                    .prop(BasePip::new(
                        NoopRelation,
                        PipWire::BelPinNear(bslots::SPEC_INT, format!("FREQ_BB{o}_MMCM_IN")),
                        PipWire::BelPinNear(bslots::SPEC_INT, format!("FREQ_BB{o}")),
                    ))
                    .test_routing(dst, src.pos())
                    .prop(FuzzIntPip::new(dst, src))
                    .prop(FuzzIntPip::new(fbb, dst))
                    .commit();
            }
        }
    }

    // FREQ_BB
    for i in 0..4 {
        let fbb = wires::CMT_FREQ_BB[i].cell(25);
        ctx.build()
            .tile_mutex("FREQ_BB", "TEST")
            .test_routing_special(fbb, specials::PRESENT)
            .prop(FuzzPip::new(
                NoopRelation,
                PipWire::BelPinNear(bslots::SPEC_INT, format!("FREQ_BB{i}_MMCM_IN")),
                PipWire::BelPinNear(bslots::SPEC_INT, format!("FREQ_BB{i}")),
            ))
            .commit();
    }
    if edev.chips.first().unwrap().regs > 1 {
        for i in 0..4 {
            let fbb = wires::CMT_FREQ_BB[i].cell(25);
            let fbb_s = wires::CMT_FREQ_BB_S[i].cell(25);
            let fbb_n = wires::CMT_FREQ_BB_N[i].cell(25);
            let omux = wires::OMUX_PLL_FREQ_BB_S[i].cell(25);
            ctx.build()
                .tile_mutex("FREQ_BB", "DRIVE")
                .prop(BaseIntPip::new(fbb, omux))
                .related_tile_mutex(
                    Delta::new(0, -50, tcls::CMT),
                    "FREQ_BB",
                    "TEST_SOURCE_DUMMY",
                )
                .extra_tile_routing_special(Delta::new(0, -50, tcls::CMT), fbb_n, specials::PRESENT)
                .test_routing(fbb_s, fbb.pos())
                .prop(FuzzIntPip::new(fbb_s, fbb))
                .commit();
            ctx.build()
                .tile_mutex("FREQ_BB", "DRIVE")
                .prop(BaseIntPip::new(fbb, omux))
                .related_tile_mutex(Delta::new(0, 50, tcls::CMT), "FREQ_BB", "TEST_SOURCE_DUMMY")
                .extra_tile_routing_special(Delta::new(0, 50, tcls::CMT), fbb_s, specials::PRESENT)
                .test_routing(fbb_n, fbb.pos())
                .prop(FuzzIntPip::new(fbb_n, fbb))
                .commit();

            ctx.build()
                .tile_mutex("FREQ_BB", "TEST_SOURCE_U")
                .related_tile_mutex(Delta::new(0, -50, tcls::CMT), "FREQ_BB", "DRIVE")
                .prop(Related::new(
                    Delta::new(0, -50, tcls::CMT),
                    BaseIntPip::new(fbb, omux),
                ))
                .prop(Related::new(
                    Delta::new(0, -50, tcls::CMT),
                    BaseIntPip::new(fbb_n, fbb),
                ))
                .prop(BasePip::new(
                    NoopRelation,
                    PipWire::BelPinNear(bslots::SPEC_INT, format!("FREQ_BB{i}_MMCM_IN")),
                    PipWire::BelPinNear(bslots::SPEC_INT, format!("FREQ_BB{i}")),
                ))
                .test_routing(fbb, fbb_s.pos())
                .prop(FuzzIntPip::new(fbb, fbb_s))
                .commit();

            ctx.build()
                .tile_mutex("FREQ_BB", "TEST_SOURCE_D")
                .related_tile_mutex(Delta::new(0, 50, tcls::CMT), "FREQ_BB", "DRIVE")
                .prop(Related::new(
                    Delta::new(0, 50, tcls::CMT),
                    BaseIntPip::new(fbb, omux),
                ))
                .prop(Related::new(
                    Delta::new(0, 50, tcls::CMT),
                    BaseIntPip::new(fbb_s, fbb),
                ))
                .prop(BasePip::new(
                    NoopRelation,
                    PipWire::BelPinNear(bslots::SPEC_INT, format!("FREQ_BB{i}_MMCM_IN")),
                    PipWire::BelPinNear(bslots::SPEC_INT, format!("FREQ_BB{i}")),
                ))
                .test_routing(fbb, fbb_n.pos())
                .prop(FuzzIntPip::new(fbb, fbb_n))
                .commit();
        }
    }

    // SYNC_BB
    ctx.build()
        .tile_mutex("SYNC_BB", "USE")
        .prop(BasePip::new(
            NoopRelation,
            PipWire::BelPinNear(bslots::PHY_CONTROL, "PHYCTLMSTREMPTY_FAR".into()),
            PipWire::BelPinNear(bslots::PHY_CONTROL, "SYNC_BB".into()),
        ))
        .test_routing(
            wires::CMT_SYNC_BB.cell(25),
            wires::OUT_PHY_PHYCTLEMPTY.cell(25).pos(),
        )
        .prop(FuzzIntPip::new(
            wires::CMT_SYNC_BB.cell(25),
            wires::OUT_PHY_PHYCTLEMPTY.cell(25),
        ))
        .commit();
    if edev.chips.first().unwrap().regs > 1 {
        ctx.build()
            .tile_mutex("SYNC_BB", "TEST")
            .no_related(Delta::new(0, -50, tcls::CMT))
            .has_related(Delta::new(0, 50, tcls::CMT))
            .test_routing_special(wires::CMT_SYNC_BB.cell(25), specials::CMT_SYNC_BB_BOT)
            .prop(FuzzPip::new(
                NoopRelation,
                PipWire::BelPinNear(bslots::PHY_CONTROL, "PHYCTLMSTREMPTY_FAR".into()),
                PipWire::BelPinNear(bslots::PHY_CONTROL, "SYNC_BB".into()),
            ))
            .commit();
        ctx.build()
            .tile_mutex("SYNC_BB", "TEST")
            .no_related(Delta::new(0, 50, tcls::CMT))
            .has_related(Delta::new(0, -50, tcls::CMT))
            .test_routing_special(wires::CMT_SYNC_BB.cell(25), specials::CMT_SYNC_BB_TOP)
            .prop(FuzzPip::new(
                NoopRelation,
                PipWire::BelPinNear(bslots::PHY_CONTROL, "PHYCTLMSTREMPTY_FAR".into()),
                PipWire::BelPinNear(bslots::PHY_CONTROL, "SYNC_BB".into()),
            ))
            .commit();

        ctx.build()
            .tile_mutex("SYNC_BB", "DRIVE")
            .prop(BaseIntPip::new(
                wires::CMT_SYNC_BB.cell(25),
                wires::OUT_PHY_PHYCTLEMPTY.cell(25),
            ))
            .related_tile_mutex(
                Delta::new(0, -50, tcls::CMT),
                "SYNC_BB",
                "TEST_SOURCE_DUMMY",
            )
            .extra_tile_routing_special(
                Delta::new(0, -50, tcls::CMT),
                wires::CMT_SYNC_BB_N.cell(25),
                specials::PRESENT,
            )
            .test_routing(
                wires::CMT_SYNC_BB_S.cell(25),
                wires::CMT_SYNC_BB.cell(25).pos(),
            )
            .prop(FuzzIntPip::new(
                wires::CMT_SYNC_BB_S.cell(25),
                wires::CMT_SYNC_BB.cell(25),
            ))
            .commit();

        ctx.build()
            .tile_mutex("SYNC_BB", "DRIVE")
            .prop(BaseIntPip::new(
                wires::CMT_SYNC_BB.cell(25),
                wires::OUT_PHY_PHYCTLEMPTY.cell(25),
            ))
            .related_tile_mutex(Delta::new(0, 50, tcls::CMT), "SYNC_BB", "TEST_SOURCE_DUMMY")
            .extra_tile_routing_special(
                Delta::new(0, 50, tcls::CMT),
                wires::CMT_SYNC_BB_S.cell(25),
                specials::PRESENT,
            )
            .test_routing(
                wires::CMT_SYNC_BB_N.cell(25),
                wires::CMT_SYNC_BB.cell(25).pos(),
            )
            .prop(FuzzIntPip::new(
                wires::CMT_SYNC_BB_N.cell(25),
                wires::CMT_SYNC_BB.cell(25),
            ))
            .commit();

        ctx.build()
            .tile_mutex("SYNC_BB", "TEST_SOURCE_U")
            .related_tile_mutex(Delta::new(0, -50, tcls::CMT), "SYNC_BB", "DRIVE")
            .prop(Related::new(
                Delta::new(0, -50, tcls::CMT),
                BaseIntPip::new(
                    wires::CMT_SYNC_BB.cell(25),
                    wires::OUT_PHY_PHYCTLEMPTY.cell(25),
                ),
            ))
            .prop(Related::new(
                Delta::new(0, -50, tcls::CMT),
                BaseIntPip::new(wires::CMT_SYNC_BB_N.cell(25), wires::CMT_SYNC_BB.cell(25)),
            ))
            .prop(BasePip::new(
                NoopRelation,
                PipWire::BelPinNear(bslots::PHY_CONTROL, "PHYCTLMSTREMPTY_FAR".into()),
                PipWire::BelPinNear(bslots::PHY_CONTROL, "SYNC_BB".into()),
            ))
            .test_routing(
                wires::CMT_SYNC_BB.cell(25),
                wires::CMT_SYNC_BB_S.cell(25).pos(),
            )
            .prop(FuzzIntPip::new(
                wires::CMT_SYNC_BB.cell(25),
                wires::CMT_SYNC_BB_S.cell(25),
            ))
            .commit();

        ctx.build()
            .tile_mutex("SYNC_BB", "TEST_SOURCE_D")
            .related_tile_mutex(Delta::new(0, 50, tcls::CMT), "SYNC_BB", "DRIVE")
            .prop(Related::new(
                Delta::new(0, 50, tcls::CMT),
                BaseIntPip::new(
                    wires::CMT_SYNC_BB.cell(25),
                    wires::OUT_PHY_PHYCTLEMPTY.cell(25),
                ),
            ))
            .prop(Related::new(
                Delta::new(0, 50, tcls::CMT),
                BaseIntPip::new(wires::CMT_SYNC_BB_S.cell(25), wires::CMT_SYNC_BB.cell(25)),
            ))
            .prop(BasePip::new(
                NoopRelation,
                PipWire::BelPinNear(bslots::PHY_CONTROL, "PHYCTLMSTREMPTY_FAR".into()),
                PipWire::BelPinNear(bslots::PHY_CONTROL, "SYNC_BB".into()),
            ))
            .test_routing(
                wires::CMT_SYNC_BB.cell(25),
                wires::CMT_SYNC_BB_N.cell(25).pos(),
            )
            .prop(FuzzIntPip::new(
                wires::CMT_SYNC_BB.cell(25),
                wires::CMT_SYNC_BB_N.cell(25),
            ))
            .commit();
    }

    // IMUX_PHASER_REFMUX
    for o in 0..3 {
        let dst = wires::IMUX_PHASER_REFMUX[o].cell(25);
        for &src in backend.edev.db_index[tcls::CMT].muxes[&dst].src.keys() {
            if let Some(idx) = wires::OUT_PLL_FREQ_BB_N.index_of(src.wire) {
                let far_src = wires::OUT_PLL_N[idx].cell(25);
                ctx.build()
                    .prop(WireMutexExclusive::new(dst))
                    .prop(BaseIntPip::new(src.tw, far_src))
                    .test_routing(dst, src)
                    .prop(FuzzIntPip::new(dst, src.tw))
                    .commit();
            } else {
                ctx.build()
                    .prop(WireMutexExclusive::new(dst))
                    .test_routing(dst, src)
                    .prop(FuzzIntPip::new(dst, src.tw))
                    .commit();
            }
        }
    }

    // PERF
    for i in 0..4 {
        let mut props: Vec<Box<DynProp>> = vec![];
        for tcid in [tcls::HCLK_IO_HP, tcls::HCLK_IO_HR] {
            if !backend.edev.tile_index[tcid].is_empty() {
                props.push(Box::new(ExtraTileMaybe::new(
                    ColPair(tcid),
                    ExtraKeyRouting::new(wires::PERF_IO[i].cell(4), wires::PERF[i].cell(4).pos()),
                )));
            }
        }
        let dst = wires::PERF[i].cell(75);
        for j in [i, i ^ 1] {
            let src = wires::PERF_IN_PLL[j].cell(25);
            ctx.build()
                .tile_mutex("PERF", "USE")
                .prop(WireMutexExclusive::new(dst))
                .prop(WireMutexExclusive::new(src))
                .props(props.clone())
                .test_routing(dst, src.pos())
                .prop(FuzzIntPip::new(dst, src))
                .commit();
        }
        for j in 0..4 {
            let src = wires::PERF_IN_PHASER[j].cell(25);
            ctx.build()
                .tile_mutex("PERF", "USE")
                .prop(WireMutexExclusive::new(dst))
                .prop(WireMutexExclusive::new(src))
                .props(props.clone())
                .test_routing(dst, src.pos())
                .prop(FuzzIntPip::new(dst, src))
                .commit();
        }
    }
}

fn add_fuzzers_phaser<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::CMT);

    for i in 0..4 {
        let mut bctx = ctx.bel(bslots::PHASER_IN[i]);
        bctx.mode("PHASER_IN_ADV")
            .test_bel_input_inv_auto(PHASER_IN::RST);
        for attr in [
            PHASER_IN::BURST_MODE,
            PHASER_IN::EN_ISERDES_RST,
            PHASER_IN::EN_TEST_RING,
            PHASER_IN::HALF_CYCLE_ADJ,
            PHASER_IN::ICLK_TO_RCLK_BYPASS,
            PHASER_IN::DQS_BIAS_MODE,
            PHASER_IN::PHASER_IN_EN,
            PHASER_IN::SYNC_IN_DIV_RST,
            PHASER_IN::UPDATE_NONACTIVE,
            PHASER_IN::WR_CYCLES,
        ] {
            bctx.mode("PHASER_IN_ADV")
                .test_bel_attr_bool_auto(attr, "FALSE", "TRUE");
        }
        bctx.mode("PHASER_IN_ADV")
            .test_bel_attr_auto_default(PHASER_IN::CLKOUT_DIV, enums::PHASER_CLKOUT_DIV::NONE);
        bctx.mode("PHASER_IN_ADV")
            .test_bel_attr_auto(PHASER_IN::CTL_MODE);
        bctx.mode("PHASER_IN")
            .test_bel_attr_auto(PHASER_IN::FREQ_REF_DIV);
        bctx.mode("PHASER_IN_ADV")
            .test_bel_attr_auto(PHASER_IN::OUTPUT_CLK_SRC);
        bctx.mode("PHASER_IN_ADV")
            .test_bel_attr_auto(PHASER_IN::PD_REVERSE);
        bctx.mode("PHASER_IN_ADV")
            .test_bel_attr_auto(PHASER_IN::STG1_PD_UPDATE);
        for attr in [
            PHASER_IN::CLKOUT_DIV_ST,
            PHASER_IN::DQS_AUTO_RECAL,
            PHASER_IN::DQS_FIND_PATTERN,
            PHASER_IN::RD_ADDR_INIT,
            PHASER_IN::REG_OPT_1,
            PHASER_IN::REG_OPT_2,
            PHASER_IN::REG_OPT_4,
            PHASER_IN::RST_SEL,
            PHASER_IN::SEL_OUT,
            PHASER_IN::TEST_BP,
        ] {
            bctx.mode("PHASER_IN_ADV")
                .test_bel_attr_multi(attr, MultiValue::Bin);
        }
        for attr in [PHASER_IN::FINE_DELAY, PHASER_IN::SEL_CLK_OFFSET] {
            bctx.mode("PHASER_IN_ADV")
                .test_bel_attr_multi(attr, MultiValue::Dec(0));
        }
    }

    for i in 0..4 {
        let mut bctx = ctx.bel(bslots::PHASER_OUT[i]);
        bctx.mode("PHASER_OUT_ADV")
            .test_bel_input_inv_auto(PHASER_OUT::RST);
        for attr in [
            PHASER_OUT::COARSE_BYPASS,
            PHASER_OUT::DATA_CTL_N,
            PHASER_OUT::DATA_RD_CYCLES,
            PHASER_OUT::EN_OSERDES_RST,
            PHASER_OUT::EN_TEST_RING,
            PHASER_OUT::OCLKDELAY_INV,
            PHASER_OUT::PHASER_OUT_EN,
            PHASER_OUT::SYNC_IN_DIV_RST,
        ] {
            bctx.mode("PHASER_OUT_ADV")
                .test_bel_attr_bool_auto(attr, "FALSE", "TRUE");
        }
        bctx.mode("PHASER_OUT_ADV")
            .test_bel_attr_auto_default(PHASER_OUT::CLKOUT_DIV, enums::PHASER_CLKOUT_DIV::NONE);
        bctx.mode("PHASER_OUT_ADV")
            .test_bel_attr_auto(PHASER_OUT::CTL_MODE);
        bctx.mode("PHASER_OUT_ADV")
            .attr("STG1_BYPASS", "PHASE_REF")
            .test_bel_attr_auto(PHASER_OUT::OUTPUT_CLK_SRC);
        bctx.mode("PHASER_OUT_ADV")
            .attr("OUTPUT_CLK_SRC", "PHASE_REF")
            .test_bel_attr_auto(PHASER_OUT::STG1_BYPASS);
        for attr in [PHASER_OUT::CLKOUT_DIV_ST, PHASER_OUT::TEST_OPT] {
            bctx.mode("PHASER_OUT_ADV")
                .test_bel_attr_multi(attr, MultiValue::Bin);
        }
        bctx.mode("PHASER_OUT_ADV")
            .attr("TEST_OPT", "")
            .test_bel_attr_bits_base(PHASER_OUT::TEST_OPT, 6)
            .multi_attr("PO", MultiValue::Bin, 3);
        for attr in [
            PHASER_OUT::COARSE_DELAY,
            PHASER_OUT::FINE_DELAY,
            PHASER_OUT::OCLK_DELAY,
        ] {
            bctx.mode("PHASER_OUT_ADV")
                .test_bel_attr_multi(attr, MultiValue::Dec(0));
        }
    }

    {
        let mut bctx = ctx.bel(bslots::PHASER_REF);
        for pin in [PHASER_REF::RST, PHASER_REF::PWRDWN] {
            bctx.mode("PHASER_REF").test_bel_input_inv_auto(pin);
        }
        for attr in [
            PHASER_REF::PHASER_REF_EN,
            PHASER_REF::SEL_SLIPD,
            PHASER_REF::SUP_SEL_AREG,
        ] {
            bctx.mode("PHASER_REF")
                .test_bel_attr_bool_auto(attr, "FALSE", "TRUE");
        }
        for attr in [
            PHASER_REF::AVDD_COMP_SET,
            PHASER_REF::AVDD_VBG_PD,
            PHASER_REF::AVDD_VBG_SEL,
            PHASER_REF::CP,
            PHASER_REF::CP_BIAS_TRIP_SET,
            PHASER_REF::CP_RES,
            PHASER_REF::LF_NEN,
            PHASER_REF::LF_PEN,
            PHASER_REF::MAN_LF,
            PHASER_REF::PFD,
            PHASER_REF::PHASER_REF_MISC,
            PHASER_REF::SEL_LF_HIGH,
            PHASER_REF::TMUX_MUX_SEL,
        ] {
            bctx.mode("PHASER_REF")
                .test_bel_attr_multi(attr, MultiValue::Bin);
        }
        for attr in [
            PHASER_REF::CONTROL_0,
            PHASER_REF::CONTROL_1,
            PHASER_REF::CONTROL_2,
            PHASER_REF::CONTROL_3,
            PHASER_REF::CONTROL_4,
            PHASER_REF::CONTROL_5,
        ] {
            bctx.mode("PHASER_REF")
                .test_bel_attr_multi(attr, MultiValue::Hex(0));
        }
        for attr in [
            PHASER_REF::LOCK_CNT,
            PHASER_REF::LOCK_FB_DLY,
            PHASER_REF::LOCK_REF_DLY,
        ] {
            bctx.mode("PHASER_REF")
                .test_bel_attr_multi(attr, MultiValue::Dec(0));
        }
    }
    {
        let mut bctx = ctx.bel(bslots::PHY_CONTROL);
        let mode = "PHY_CONTROL";
        for (aid, _aname, attr) in &backend.edev.db[PHY_CONTROL].attributes {
            if aid == PHY_CONTROL::AO_WRLVL_EN {
                bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Bin);
            } else {
                match attr.typ {
                    BelAttributeType::Bool => {
                        bctx.mode(mode)
                            .test_bel_attr_bool_auto(aid, "FALSE", "TRUE");
                    }
                    BelAttributeType::BitVec(_) => {
                        bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Dec(0));
                    }
                    BelAttributeType::Enum(_) => {
                        bctx.mode(mode).test_bel_attr_auto(aid);
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}

fn add_fuzzers_pll<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::CMT);

    for bslot in bslots::PLL {
        let mut bctx = ctx.bel(bslot);
        let use_calc = if bslot == bslots::PLL[0] {
            "MMCMADV_*_USE_CALC"
        } else {
            "PLLADV_*_USE_CALC"
        };
        let mode = if bslot == bslots::PLL[0] {
            "MMCME2_ADV"
        } else {
            "PLLE2_ADV"
        };
        bctx.build()
            .global_xy(use_calc, "NO")
            .test_manual_legacy("ENABLE", "1")
            .mode(mode)
            .commit();
        for pin in ["CLKINSEL", "PSEN", "PSINCDEC", "PWRDWN", "RST"] {
            if matches!(pin, "PSEN" | "PSINCDEC") && bslot == bslots::PLL[1] {
                continue;
            }
            bctx.mode(mode).mutex("MODE", "INV").test_inv_legacy(pin);
        }
        for attr in [
            "DIRECT_PATH_CNTRL",
            "EN_VCO_DIV1",
            "EN_VCO_DIV6",
            "GTS_WAIT",
            "HVLF_CNT_TEST_EN",
            "IN_DLY_EN",
            "LF_LOW_SEL",
            "SEL_HV_NMOS",
            "SEL_LV_NMOS",
            "STARTUP_WAIT",
            "SUP_SEL_AREG",
            "SUP_SEL_DREG",
            "VLF_HIGH_DIS_B",
            "VLF_HIGH_PWDN_B",
            "DIVCLK_NOCOUNT",
            "CLKFBIN_NOCOUNT",
            "CLKFBOUT_EN",
            "CLKFBOUT_NOCOUNT",
            "CLKOUT0_EN",
            "CLKOUT0_NOCOUNT",
            "CLKOUT1_EN",
            "CLKOUT1_NOCOUNT",
            "CLKOUT2_EN",
            "CLKOUT2_NOCOUNT",
            "CLKOUT3_EN",
            "CLKOUT3_NOCOUNT",
            "CLKOUT4_EN",
            "CLKOUT4_NOCOUNT",
            "CLKOUT5_EN",
            "CLKOUT5_NOCOUNT",
        ] {
            bctx.mode(mode)
                .mutex("MODE", "TEST")
                .global_xy(use_calc, "NO")
                .test_enum_legacy(attr, &["FALSE", "TRUE"]);
        }
        if bslot == bslots::PLL[0] {
            for attr in [
                "SEL_SLIPD",
                "CLKBURST_ENABLE",
                "CLKBURST_REPEAT",
                "INTERP_TEST",
                "CLKOUT6_EN",
                "CLKOUT6_NOCOUNT",
            ] {
                bctx.mode(mode)
                    .mutex("MODE", "TEST")
                    .global_xy(use_calc, "NO")
                    .test_enum_legacy(attr, &["FALSE", "TRUE"]);
            }
            bctx.mode(mode)
                .mutex("MODE", "TEST")
                .global_xy(use_calc, "NO")
                .attr("CLKOUT6_EN", "TRUE")
                .attr("CLKOUT4_USE_FINE_PS", "")
                .attr("CLKOUT4_MX", "")
                .test_enum_legacy("CLKOUT4_CASCADE", &["FALSE", "TRUE"]);
            for attr in [
                "CLKOUT0_USE_FINE_PS",
                "CLKOUT1_USE_FINE_PS",
                "CLKOUT2_USE_FINE_PS",
                "CLKOUT3_USE_FINE_PS",
                "CLKOUT4_USE_FINE_PS",
                "CLKOUT5_USE_FINE_PS",
                "CLKOUT6_USE_FINE_PS",
                "CLKFBOUT_USE_FINE_PS",
            ] {
                bctx.mode(mode)
                    .mutex("MODE", "TEST")
                    .global_xy(use_calc, "NO")
                    .attr("CLKFBOUT_MX", "")
                    .attr("CLKOUT0_MX", "")
                    .attr("CLKOUT1_MX", "")
                    .attr("CLKOUT2_MX", "")
                    .attr("CLKOUT3_MX", "")
                    .attr("CLKOUT4_MX", "")
                    .attr("CLKOUT5_MX", "")
                    .attr("CLKOUT6_MX", "")
                    .attr("INTERP_EN", "00000000")
                    .test_enum_legacy(attr, &["FALSE", "TRUE"]);
            }
            for attr in ["CLKOUT0_FRAC_EN", "CLKFBOUT_FRAC_EN"] {
                bctx.mode(mode)
                    .mutex("MODE", "TEST")
                    .global_xy(use_calc, "NO")
                    .attr("CLKOUT5_EN", "TRUE")
                    .attr("CLKOUT6_EN", "TRUE")
                    .attr("INTERP_EN", "00000000")
                    .test_enum_legacy(attr, &["FALSE", "TRUE"]);
            }
        }
        for (attr, width) in [
            ("CLKFBIN_LT", 6),
            ("CLKFBIN_HT", 6),
            ("DIVCLK_LT", 6),
            ("DIVCLK_HT", 6),
            ("CLKFBOUT_LT", 6),
            ("CLKFBOUT_HT", 6),
            ("CLKFBOUT_DT", 6),
            ("CLKFBOUT_MX", 2),
            ("CLKOUT0_LT", 6),
            ("CLKOUT0_HT", 6),
            ("CLKOUT0_DT", 6),
            ("CLKOUT0_MX", 2),
            ("CLKOUT1_LT", 6),
            ("CLKOUT1_HT", 6),
            ("CLKOUT1_DT", 6),
            ("CLKOUT1_MX", 2),
            ("CLKOUT2_LT", 6),
            ("CLKOUT2_HT", 6),
            ("CLKOUT2_DT", 6),
            ("CLKOUT2_MX", 2),
            ("CLKOUT3_LT", 6),
            ("CLKOUT3_HT", 6),
            ("CLKOUT3_DT", 6),
            ("CLKOUT3_MX", 2),
            ("CLKOUT4_LT", 6),
            ("CLKOUT4_HT", 6),
            ("CLKOUT4_DT", 6),
            ("CLKOUT4_MX", 2),
            ("CLKOUT5_LT", 6),
            ("CLKOUT5_HT", 6),
            ("CLKOUT5_DT", 6),
            ("CLKOUT5_MX", 2),
            ("TMUX_MUX_SEL", 2),
            ("CONTROL_0", 16),
            ("CONTROL_1", 16),
            ("CONTROL_2", 16),
            ("CONTROL_3", 16),
            ("CONTROL_4", 16),
            ("CONTROL_5", 16),
            ("CONTROL_6", 16),
            ("CONTROL_7", 16),
            ("ANALOG_MISC", 4),
            ("CP_BIAS_TRIP_SET", 1),
            ("CP_RES", 2),
            ("EN_CURR_SINK", 2),
            ("AVDD_COMP_SET", 3),
            ("AVDD_VBG_PD", 3),
            ("AVDD_VBG_SEL", 4),
            ("DVDD_COMP_SET", 3),
            ("DVDD_VBG_PD", 3),
            ("DVDD_VBG_SEL", 4),
            ("FREQ_COMP", 2),
            ("IN_DLY_MX_CVDD", 6),
            ("IN_DLY_MX_DVDD", 6),
            ("LF_NEN", 2),
            ("LF_PEN", 2),
            ("MAN_LF", 3),
            ("PFD", 7),
            ("SKEW_FLOP_INV", 4),
            ("SPARE_ANALOG", 5),
            ("SPARE_DIGITAL", 5),
            ("VREF_START", 2),
            ("MVDD_SEL", 2),
            ("SYNTH_CLK_DIV", 2),
        ] {
            bctx.mode(mode)
                .mutex("MODE", "TEST")
                .global_xy(use_calc, "NO")
                .test_multi_attr_bin_legacy(attr, width);
        }
        if bslot == bslots::PLL[0] {
            for (attr, width) in [
                ("SS_STEPS", 3),
                ("SS_STEPS_INIT", 3),
                ("CLKFBOUT_PM_RISE", 3),
                ("CLKFBOUT_PM_FALL", 3),
                ("CLKOUT0_PM_RISE", 3),
                ("CLKOUT0_PM_FALL", 3),
                ("CLKOUT1_PM", 3),
                ("CLKOUT2_PM", 3),
                ("CLKOUT3_PM", 3),
                ("CLKOUT4_PM", 3),
                ("CLKOUT5_PM", 3),
                ("CLKOUT6_PM", 3),
                ("CLKOUT6_LT", 6),
                ("CLKOUT6_HT", 6),
                ("CLKOUT6_DT", 6),
                ("CLKOUT6_MX", 2),
                ("FINE_PS_FRAC", 6),
            ] {
                bctx.mode(mode)
                    .mutex("MODE", "TEST")
                    .global_xy(use_calc, "NO")
                    .attr("INTERP_EN", "00000000")
                    .test_multi_attr_bin_legacy(attr, width);
            }
            bctx.mode(mode)
                .mutex("MODE", "TEST")
                .global_xy(use_calc, "NO")
                .test_multi_attr_bin_legacy("INTERP_EN", 8);
        } else {
            for (attr, width) in [
                ("CLKFBOUT_PM", 3),
                ("CLKOUT0_PM", 3),
                ("CLKOUT1_PM", 3),
                ("CLKOUT2_PM", 3),
                ("CLKOUT3_PM", 3),
                ("CLKOUT4_PM", 3),
                ("CLKOUT5_PM", 3),
            ] {
                bctx.mode(mode)
                    .mutex("MODE", "TEST")
                    .global_xy(use_calc, "NO")
                    .test_multi_attr_bin_legacy(attr, width);
            }
        }
        for (attr, width) in [
            ("CP", 4),
            ("HROW_DLY_SET", 3),
            ("HVLF_CNT_TEST", 6),
            ("LFHF", 2),
            ("LOCK_CNT", 10),
            ("LOCK_FB_DLY", 5),
            ("LOCK_REF_DLY", 5),
            ("LOCK_SAT_HIGH", 10),
            ("RES", 4),
            ("UNLOCK_CNT", 10),
            ("IN_DLY_SET", 6),
        ] {
            bctx.mode(mode)
                .mutex("MODE", "TEST")
                .global_xy(use_calc, "NO")
                .test_multi_attr_dec_legacy(attr, width);
        }
        if bslot == bslots::PLL[0] {
            bctx.mode(mode)
                .mutex("MODE", "TEST")
                .global_xy(use_calc, "NO")
                .test_multi_attr_dec_legacy("CLKBURST_CNT", 4);
            bctx.mode(mode)
                .mutex("MODE", "TEST_SS")
                .global_xy(use_calc, "NO")
                .attr("INTERP_EN", "00000000")
                .attr("CLKFBOUT_LT", "000000")
                .attr("CLKFBOUT_HT", "000000")
                .attr("CLKFBOUT_DT", "000000")
                .attr("CLKFBOUT_FRAC_EN", "FALSE")
                .attr("CLKOUT2_EN", "FALSE")
                .attr("CLKOUT2_MX", "00")
                .attr("CLKOUT3_EN", "FALSE")
                .test_enum_legacy("SS_EN", &["FALSE", "TRUE"]);
        }

        for mult in 1..=64 {
            if bslot == bslots::PLL[0] {
                for bandwidth in ["LOW", "HIGH"] {
                    bctx.mode(mode)
                        .mutex("MODE", "CALC")
                        .global_xy(use_calc, "NO")
                        .attr("SS_EN", "FALSE")
                        .test_manual_legacy("TABLES", format!("{mult}.{bandwidth}"))
                        .attr("CLKFBOUT_MULT_F", format!("{mult}"))
                        .attr("BANDWIDTH", bandwidth)
                        .commit();
                }
                bctx.mode(mode)
                    .mutex("MODE", "CALC")
                    .global_xy(use_calc, "NO")
                    .attr("SS_EN", "TRUE")
                    .attr("INTERP_EN", "00000000")
                    .attr("CLKFBOUT_LT", "000000")
                    .attr("CLKFBOUT_HT", "000000")
                    .attr("CLKFBOUT_DT", "000000")
                    .attr("CLKFBOUT_FRAC_EN", "FALSE")
                    .attr("CLKOUT2_EN", "FALSE")
                    .attr("CLKOUT2_MX", "00")
                    .attr("CLKOUT3_EN", "FALSE")
                    .test_manual_legacy("TABLES", format!("{mult}.SS"))
                    .attr("CLKFBOUT_MULT_F", format!("{mult}"))
                    .attr("BANDWIDTH", "LOW")
                    .commit();
            } else {
                for bandwidth in ["LOW", "HIGH"] {
                    bctx.mode(mode)
                        .mutex("MODE", "CALC")
                        .global_xy(use_calc, "NO")
                        .test_manual_legacy("TABLES", format!("{mult}.{bandwidth}"))
                        .attr("CLKFBOUT_MULT", format!("{mult}"))
                        .attr("BANDWIDTH", bandwidth)
                        .commit();
                }
            }
        }
        bctx.mode(mode)
            .mutex("MODE", "COMP")
            .global_xy(use_calc, "NO")
            .attr("HROW_DLY_SET", "000")
            .test_enum_legacy("COMPENSATION", &["ZHOLD", "EXTERNAL", "INTERNAL", "BUF_IN"]);

        bctx.mode(mode)
            .test_bel_special(specials::DRP_MASK_CMT)
            .pin("DWE")
            .commit();
    }
}

fn add_fuzzers_bufmrce<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::CMT);

    for i in 0..2 {
        let mut bctx = ctx.bel(bslots::BUFMRCE[i]);
        bctx.build()
            .test_bel_attr_bits(BUFHCE::ENABLE)
            .mode("BUFMRCE")
            .commit();
        bctx.mode("BUFMRCE").test_bel_input_inv_auto(BUFHCE::CE);
        bctx.mode("BUFMRCE")
            .test_bel_attr_bool_auto(BUFHCE::INIT_OUT, "0", "1");
        bctx.mode("BUFMRCE").test_bel_attr_auto(BUFHCE::CE_TYPE);
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    add_fuzzers_fifo(session, backend);
    add_fuzzers_routing(session, backend);
    add_fuzzers_phaser(session, backend);
    add_fuzzers_pll(session, backend);
    add_fuzzers_bufmrce(session, backend);
}

fn collect_fuzzers_fifo(ctx: &mut CollectorCtx) {
    let tcid = tcls::CMT_FIFO;
    {
        let bslot = bslots::IN_FIFO;
        ctx.collect_bel_attr_default(
            tcid,
            bslot,
            IN_FIFO::ALMOST_EMPTY_VALUE,
            enums::IO_FIFO_WATERMARK::NONE,
        );
        ctx.collect_bel_attr_default(
            tcid,
            bslot,
            IN_FIFO::ALMOST_FULL_VALUE,
            enums::IO_FIFO_WATERMARK::NONE,
        );
        ctx.collect_bel_attr(tcid, bslot, IN_FIFO::ARRAY_MODE);
        ctx.collect_bel_attr_bi(tcid, bslot, IN_FIFO::SLOW_RD_CLK);
        ctx.collect_bel_attr_bi(tcid, bslot, IN_FIFO::SLOW_WR_CLK);
        ctx.collect_bel_attr_bi(tcid, bslot, IN_FIFO::SYNCHRONOUS_MODE);
        ctx.collect_bel_attr(tcid, bslot, IN_FIFO::SPARE);
    }
    {
        let bslot = bslots::OUT_FIFO;
        ctx.collect_bel_attr_default(
            tcid,
            bslot,
            OUT_FIFO::ALMOST_EMPTY_VALUE,
            enums::IO_FIFO_WATERMARK::NONE,
        );
        ctx.collect_bel_attr_default(
            tcid,
            bslot,
            OUT_FIFO::ALMOST_FULL_VALUE,
            enums::IO_FIFO_WATERMARK::NONE,
        );
        ctx.collect_bel_attr(tcid, bslot, OUT_FIFO::ARRAY_MODE);
        ctx.collect_bel_attr_bi(tcid, bslot, OUT_FIFO::SLOW_RD_CLK);
        ctx.collect_bel_attr_bi(tcid, bslot, OUT_FIFO::SLOW_WR_CLK);
        ctx.collect_bel_attr_bi(tcid, bslot, OUT_FIFO::SYNCHRONOUS_MODE);
        ctx.collect_bel_attr_bi(tcid, bslot, OUT_FIFO::OUTPUT_DISABLE);
        ctx.collect_bel_attr(tcid, bslot, OUT_FIFO::SPARE);
    }
}

fn collect_fuzzers_routing(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };

    let tcid = tcls::CMT;

    for i in 0..2 {
        ctx.collect_mux(tcid, wires::IMUX_BUFMRCE[i].cell(25));
        ctx.collect_mux(tcid, wires::LCLK_CMT_S[i].cell(25));
        ctx.collect_mux(tcid, wires::LCLK_CMT_N[i].cell(25));
        ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN1_HCLK[i].cell(25));
        ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN2_HCLK[i].cell(25));
        ctx.collect_mux(tcid, wires::IMUX_PLL_CLKFB_HCLK[i].cell(25));
        ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN1[i].cell(25));
        ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN2[i].cell(25));
        ctx.collect_mux(tcid, wires::IMUX_PLL_CLKFB[i].cell(25));
    }
    for i in 0..4 {
        ctx.collect_mux(tcid, wires::IMUX_PHASER_IN_PHASEREFCLK[i].cell(25));
        ctx.collect_mux(tcid, wires::IMUX_PHASER_OUT_PHASEREFCLK[i].cell(25));
        ctx.collect_mux_ocd(
            tcid,
            wires::OMUX_PLL_PERF[i].cell(25),
            OcdMode::BitOrderDrpV6,
        );
    }

    let mut diffs_freq_bb = vec![];
    for _ in 0..4 {
        diffs_freq_bb.push(vec![(None, Diff::default())]);
    }

    // OMUX_CCIO and OMUX_HCLK_FREQ_BB (extracted together via special fuckery)
    for i in 0..4 {
        let omux_ccio = wires::OMUX_CCIO[i].cell(25);
        let omux_freq_bb = wires::OMUX_HCLK_FREQ_BB[i].cell(25);
        let mut diffs_occio = vec![(None, Diff::default())];
        for w in [wires::OUT_PHASER_REF_CLKOUT, wires::OUT_PHASER_REF_TMUXOUT] {
            let src = w.cell(25).pos();
            diffs_occio.push((Some(src), ctx.get_diff_routing(tcid, omux_ccio, src)));
        }
        let ccio_cmt = wires::CCIO_CMT[i].cell(25);
        let ckint_cmt = wires::CKINT_CMT[i].cell(25);
        let clkpad = ctx.edev.db_index[tcid].only_bwd(ccio_cmt);
        let diff_occio_ccio = ctx
            .get_diff_routing_pair_special(
                tcid,
                wires::HROW_O[0].cell(25),
                ccio_cmt.pos(),
                specials::CMT_BUF,
            )
            .combine(&!ctx.peek_diff_routing(tcid, wires::HROW_O[0].cell(25), omux_ccio.pos()));
        let mut diffs_ofbb = vec![(
            Some(ckint_cmt.pos()),
            ctx.get_diff_routing(tcid, omux_freq_bb, ckint_cmt.pos()),
        )];
        let diff_fbb_ccio = ctx.get_diff_routing(tcid, omux_freq_bb, ccio_cmt.pos());
        let (diff_occio_ccio, diff_fbb_ccio, diff_en_ccio) =
            Diff::split(diff_occio_ccio, diff_fbb_ccio);
        diffs_occio.push((Some(ccio_cmt.pos()), diff_occio_ccio));
        diffs_ofbb.push((Some(ccio_cmt.pos()), diff_fbb_ccio));
        ctx.insert_mux(tcid, omux_ccio, xlat_enum_raw(diffs_occio, OcdMode::Mux));
        let buf_fbb = extract_common_diff(&mut diffs_ofbb);
        diffs_freq_bb[i].push((Some(omux_freq_bb.pos()), buf_fbb));
        diffs_ofbb.push((None, Diff::default()));
        ctx.insert_mux(tcid, omux_freq_bb, xlat_enum_raw(diffs_ofbb, OcdMode::Mux));
        ctx.insert_progbuf(tcid, ccio_cmt, clkpad.pos(), xlat_bit(diff_en_ccio));
    }

    // OMUX_PLL_FREQ_BB
    for o in 0..4 {
        for (wt, wf) in [
            (wires::OMUX_PLL_FREQ_BB_S, wires::OUT_PLL_FREQ_BB_S),
            (wires::OMUX_PLL_FREQ_BB_N, wires::OUT_PLL_FREQ_BB_N),
        ] {
            let dst = wt[o].cell(25);
            let mut diffs = vec![];
            for i in 0..4 {
                let src = wf[i].cell(25).pos();
                diffs.push((Some(src), ctx.get_diff_routing(tcid, dst, src)));
            }
            diffs_freq_bb[o].push((Some(dst.pos()), extract_common_diff(&mut diffs)));
            ctx.insert_mux(tcid, dst, xlat_enum_raw(diffs, OcdMode::Mux));
        }
    }

    // various buffers grabbed via HROW_O
    let dst0 = wires::HROW_O[0].cell(25);
    for i in 0..12 {
        let dst = wires::HCLK_CMT[i].cell(25);
        let src = wires::HCLK_ROW[i].cell(25);
        let diff = ctx
            .get_diff_routing_pair_special(tcid, dst0, dst.pos(), specials::CMT_BUF)
            .combine(&!ctx.peek_diff_routing(tcid, dst0, dst.pos()));
        ctx.insert_progbuf(tcid, dst, src.pos(), xlat_bit(diff));
    }
    for i in 4..14 {
        let dst = wires::HROW_I_CMT[i].cell(25);
        let src = wires::HROW_I[i].cell(25);
        let diff = ctx
            .get_diff_routing_pair_special(tcid, dst0, dst.pos(), specials::CMT_BUF)
            .combine(&!ctx.peek_diff_routing(tcid, dst0, dst.pos()));
        ctx.insert_progbuf(tcid, dst, src.pos(), xlat_bit(diff));
    }
    // HROW_O itself
    for i in 0..14 {
        ctx.collect_mux(tcid, wires::HROW_O[i].cell(25));
    }

    for wt in [
        wires::OUT_PLL_FREQ_BB_S,
        wires::OUT_PLL_FREQ_BB_N,
        wires::CKINT_CMT,
    ]
    .into_iter()
    .flatten()
    {
        let dst = wt.cell(25);
        let src = ctx.edev.db_index[tcls::CMT].only_bwd(dst);
        ctx.collect_progbuf(tcid, dst, src);
    }

    // FREQ_BB
    for i in 0..4 {
        let w = wires::CMT_FREQ_BB[i].cell(25);
        let bits = xlat_bit_wide(ctx.get_diff_routing_special(tcid, w, specials::PRESENT));
        ctx.insert_support(tcid, BTreeSet::from([w]), bits);
    }
    if edev.chips.first().unwrap().regs > 1 {
        for i in 0..4 {
            let w = wires::CMT_FREQ_BB[i].cell(25);
            for w_sn in [wires::CMT_FREQ_BB_S, wires::CMT_FREQ_BB_N] {
                let w_sn = w_sn[i].cell(25);
                let supp =
                    xlat_bit_wide(ctx.get_diff_routing_special(tcid, w_sn, specials::PRESENT));
                assert_eq!(supp.len(), 2);

                let mut diff = ctx.get_diff_routing(tcid, w_sn, w.pos());
                diff.apply_bitvec_diff_int(&supp, 3, 0);
                ctx.insert_mux(
                    tcid,
                    w_sn,
                    xlat_enum_raw(
                        vec![(None, Diff::default()), (Some(w.pos()), diff)],
                        OcdMode::Mux,
                    ),
                );

                ctx.insert_support(tcid, BTreeSet::from([w_sn]), supp);

                let diff = ctx.get_diff_routing(tcid, w, w_sn.pos());
                diffs_freq_bb[i].push((Some(w_sn.pos()), diff));
            }

            ctx.insert_mux(
                tcid,
                w,
                xlat_enum_raw(diffs_freq_bb[i].clone(), OcdMode::Mux),
            );
        }
    }

    // SYNC_BB
    let sync = wires::CMT_SYNC_BB.cell(25);
    ctx.collect_progbuf(tcid, sync, wires::OUT_PHY_PHYCTLEMPTY.cell(25).pos());
    if edev.chips.first().unwrap().regs > 1 {
        let sync_s = wires::CMT_SYNC_BB_S.cell(25);
        let sync_n = wires::CMT_SYNC_BB_N.cell(25);

        ctx.collect_progbuf(tcid, sync, sync_s.pos());
        ctx.collect_progbuf(tcid, sync, sync_n.pos());

        for w in [sync_s, sync_n] {
            let bit = xlat_bit(ctx.get_diff_routing_special(tcid, w, specials::PRESENT));
            ctx.insert_support(tcid, BTreeSet::from([w]), vec![bit]);

            let mut diff = ctx.get_diff_routing(tcid, w, sync.pos());
            diff.apply_bit_diff(bit, true, false);
            ctx.insert_progbuf(tcid, w, sync.pos(), xlat_bit(diff));
        }

        let diff_bot = ctx.get_diff_routing_special(tcid, sync, specials::CMT_SYNC_BB_BOT);
        let diff_top = ctx.get_diff_routing_special(tcid, sync, specials::CMT_SYNC_BB_TOP);
        let (diff_bot, diff_top, diff_com) = Diff::split(diff_bot, diff_top);
        ctx.insert_support(
            tcid,
            BTreeSet::from([sync]),
            vec![xlat_bit(diff_bot), xlat_bit(diff_com), xlat_bit(diff_top)],
        );
    }

    for i in 0..3 {
        ctx.collect_mux(tcid, wires::IMUX_PHASER_REFMUX[i].cell(25));
    }

    // PERF and associated bufs
    for i in 0..4 {
        let fdst0 = wires::PERF[i].cell(75);
        let fdst1 = wires::PERF[i ^ 1].cell(75);
        let dst = wires::PERF_IN_PLL[i].cell(25);
        let src = wires::OMUX_PLL_PERF[i].cell(25).pos();
        let diff_a = ctx.peek_diff_routing(tcid, fdst0, dst.pos()).clone();
        let diff_b = ctx.peek_diff_routing(tcid, fdst1, dst.pos()).clone();
        let (_, _, diff) = Diff::split(diff_a, diff_b);
        ctx.insert_progbuf(tcid, dst, src, xlat_bit(diff));
    }
    for i in 0..4 {
        let fdst0 = wires::PERF[i].cell(75);
        let fdst1 = wires::PERF[i ^ 1].cell(75);
        let dst = wires::PERF_IN_PHASER[i].cell(25);
        let src = wires::OUT_PHASER_IN_RCLK[i].cell(25).pos();
        let diff_a = ctx.peek_diff_routing(tcid, fdst0, dst.pos()).clone();
        let diff_b = ctx.peek_diff_routing(tcid, fdst1, dst.pos()).clone();
        let (_, _, diff) = Diff::split(diff_a, diff_b);
        ctx.insert_progbuf(tcid, dst, src, xlat_bit(diff));
    }

    for i in 0..4 {
        let dst = wires::PERF[i].cell(75);
        let mut diffs = vec![(None, Diff::default())];
        for &src in ctx.edev.db_index[tcls::CMT].muxes[&dst].src.keys() {
            let fsrc = if let Some(idx) = wires::PERF_IN_PLL.index_of(src.wire) {
                wires::OMUX_PLL_PERF[idx].cell(25).pos()
            } else if let Some(idx) = wires::PERF_IN_PHASER.index_of(src.wire) {
                wires::OUT_PHASER_IN_RCLK[idx].cell(25).pos()
            } else {
                unreachable!()
            };
            let mut diff = ctx.get_diff_routing(tcid, dst, src);
            diff.apply_bit_diff(ctx.sb_progbuf(tcid, src.tw, fsrc), true, false);
            diffs.push((Some(src), diff));
        }
        ctx.insert_mux(tcid, dst, xlat_enum_raw(diffs, OcdMode::Mux));
    }

    for tcid in [tcls::HCLK_IO_HR, tcls::HCLK_IO_HP] {
        if ctx.has_tcls(tcid) {
            for i in 0..4 {
                ctx.collect_progbuf(
                    tcid,
                    wires::PERF_IO[i].cell(4),
                    wires::PERF[i].cell(4).pos(),
                );
            }
        }
    }
}

fn collect_fuzzers_phaser(ctx: &mut CollectorCtx) {
    let tcid = tcls::CMT;

    for i in 0..4 {
        let bslot = bslots::PHASER_IN[i];
        ctx.collect_bel_input_inv_bi(tcid, bslot, PHASER_IN::RST);
        for attr in [
            PHASER_IN::BURST_MODE,
            PHASER_IN::DQS_BIAS_MODE,
            PHASER_IN::EN_ISERDES_RST,
            PHASER_IN::EN_TEST_RING,
            PHASER_IN::HALF_CYCLE_ADJ,
            PHASER_IN::ICLK_TO_RCLK_BYPASS,
            PHASER_IN::PHASER_IN_EN,
            PHASER_IN::SYNC_IN_DIV_RST,
            PHASER_IN::UPDATE_NONACTIVE,
            PHASER_IN::WR_CYCLES,
        ] {
            ctx.collect_bel_attr_bi(tcid, bslot, attr);
        }
        ctx.collect_bel_attr_default_ocd(
            tcid,
            bslot,
            PHASER_IN::CLKOUT_DIV,
            enums::PHASER_CLKOUT_DIV::NONE,
            OcdMode::BitOrderDrpV6,
        );
        ctx.collect_bel_attr_ocd(
            tcid,
            bslot,
            PHASER_IN::OUTPUT_CLK_SRC,
            OcdMode::BitOrderDrpV6,
        );
        for attr in [
            PHASER_IN::CTL_MODE,
            PHASER_IN::FREQ_REF_DIV,
            PHASER_IN::PD_REVERSE,
            PHASER_IN::STG1_PD_UPDATE,
            PHASER_IN::CLKOUT_DIV_ST,
            PHASER_IN::DQS_AUTO_RECAL,
            PHASER_IN::DQS_FIND_PATTERN,
            PHASER_IN::RD_ADDR_INIT,
            PHASER_IN::REG_OPT_1,
            PHASER_IN::REG_OPT_2,
            PHASER_IN::REG_OPT_4,
            PHASER_IN::RST_SEL,
            PHASER_IN::SEL_OUT,
            PHASER_IN::TEST_BP,
            PHASER_IN::FINE_DELAY,
            PHASER_IN::SEL_CLK_OFFSET,
        ] {
            ctx.collect_bel_attr(tcid, bslot, attr);
        }
    }
    for i in 0..4 {
        let bslot = bslots::PHASER_OUT[i];

        ctx.collect_bel_input_inv_bi(tcid, bslot, PHASER_OUT::RST);
        for attr in [
            PHASER_OUT::COARSE_BYPASS,
            PHASER_OUT::DATA_CTL_N,
            PHASER_OUT::DATA_RD_CYCLES,
            PHASER_OUT::EN_OSERDES_RST,
            PHASER_OUT::EN_TEST_RING,
            PHASER_OUT::OCLKDELAY_INV,
            PHASER_OUT::PHASER_OUT_EN,
            PHASER_OUT::SYNC_IN_DIV_RST,
        ] {
            ctx.collect_bel_attr_bi(tcid, bslot, attr);
        }
        ctx.collect_bel_attr_default_ocd(
            tcid,
            bslot,
            PHASER_OUT::CLKOUT_DIV,
            enums::PHASER_CLKOUT_DIV::NONE,
            OcdMode::BitOrderDrpV6,
        );
        ctx.collect_bel_attr_ocd(
            tcid,
            bslot,
            PHASER_OUT::OUTPUT_CLK_SRC,
            OcdMode::BitOrderDrpV6,
        );
        for attr in [
            PHASER_OUT::CTL_MODE,
            PHASER_OUT::STG1_BYPASS,
            PHASER_OUT::CLKOUT_DIV_ST,
            PHASER_OUT::COARSE_DELAY,
            PHASER_OUT::FINE_DELAY,
            PHASER_OUT::OCLK_DELAY,
            PHASER_OUT::TEST_OPT,
        ] {
            ctx.collect_bel_attr(tcid, bslot, attr);
        }
    }
    {
        let bslot = bslots::PHASER_REF;
        ctx.collect_bel_input_inv_bi(tcid, bslot, PHASER_REF::RST);
        ctx.collect_bel_input_inv_bi(tcid, bslot, PHASER_REF::PWRDWN);
        for attr in [
            PHASER_REF::PHASER_REF_EN,
            PHASER_REF::SEL_SLIPD,
            PHASER_REF::SUP_SEL_AREG,
        ] {
            ctx.collect_bel_attr_bi(tcid, bslot, attr);
        }
        for attr in [
            PHASER_REF::AVDD_COMP_SET,
            PHASER_REF::AVDD_VBG_PD,
            PHASER_REF::AVDD_VBG_SEL,
            PHASER_REF::CP,
            PHASER_REF::CP_BIAS_TRIP_SET,
            PHASER_REF::CP_RES,
            PHASER_REF::LF_NEN,
            PHASER_REF::LF_PEN,
            PHASER_REF::MAN_LF,
            PHASER_REF::PFD,
            PHASER_REF::PHASER_REF_MISC,
            PHASER_REF::SEL_LF_HIGH,
            PHASER_REF::TMUX_MUX_SEL,
            PHASER_REF::CONTROL_0,
            PHASER_REF::CONTROL_1,
            PHASER_REF::CONTROL_2,
            PHASER_REF::CONTROL_3,
            PHASER_REF::CONTROL_4,
            PHASER_REF::CONTROL_5,
            PHASER_REF::LOCK_CNT,
            PHASER_REF::LOCK_FB_DLY,
            PHASER_REF::LOCK_REF_DLY,
        ] {
            ctx.collect_bel_attr(tcid, bslot, attr);
        }
    }
    {
        let bslot = bslots::PHY_CONTROL;
        for (aid, _aname, attr) in &ctx.edev.db[PHY_CONTROL].attributes {
            if attr.typ == BelAttributeType::Bool {
                ctx.collect_bel_attr_bi(tcid, bslot, aid);
            } else {
                ctx.collect_bel_attr(tcid, bslot, aid);
            }
        }
    }
}

fn collect_fuzzers_pll(ctx: &mut CollectorCtx) {
    let tcid = tcls::CMT;
    for i in 0..2 {
        let bel = ["PLL[0]", "PLL[1]"][i];
        let bslot = bslots::PLL[i];
        let tile = "CMT";

        fn drp_bit(which: &'static str, reg: usize, bit: usize) -> TileBit {
            if which == "PLL[0]" {
                let tile = 15 - (reg >> 3);
                let frame = 29 - (bit & 1);
                let bit = 63 - ((bit >> 1) | (reg & 7) << 3);
                TileBit::new(tile, frame, bit)
            } else {
                let tile = 37 + (reg >> 3);
                let frame = 28 + (bit & 1);
                let bit = (bit >> 1) | (reg & 7) << 3;
                TileBit::new(tile, frame, bit)
            }
        }
        for reg in 0..(if bel == "PLL[0]" { 0x80 } else { 0x68 }) {
            ctx.insert_legacy(
                tile,
                bel,
                format!("DRP{reg:02X}"),
                TileItem::from_bitvec_inv(
                    (0..16).map(|bit| drp_bit(bel, reg, bit)).collect(),
                    false,
                ),
            );
        }

        for pin in ["CLKINSEL", "PWRDWN", "RST"] {
            ctx.collect_inv_legacy(tile, bel, pin);
        }
        if bel == "PLL[0]" {
            for pin in ["PSEN", "PSINCDEC"] {
                ctx.collect_inv_legacy(tile, bel, pin);
            }
        }
        for attr in [
            "DIRECT_PATH_CNTRL",
            "EN_VCO_DIV1",
            "EN_VCO_DIV6",
            "GTS_WAIT",
            "HVLF_CNT_TEST_EN",
            "IN_DLY_EN",
            "LF_LOW_SEL",
            "SEL_HV_NMOS",
            "SEL_LV_NMOS",
            "STARTUP_WAIT",
            "SUP_SEL_AREG",
            "SUP_SEL_DREG",
            "VLF_HIGH_DIS_B",
            "VLF_HIGH_PWDN_B",
            "DIVCLK_NOCOUNT",
            "CLKFBIN_NOCOUNT",
            "CLKFBOUT_EN",
            "CLKFBOUT_NOCOUNT",
            "CLKOUT0_EN",
            "CLKOUT0_NOCOUNT",
            "CLKOUT1_EN",
            "CLKOUT1_NOCOUNT",
            "CLKOUT2_EN",
            "CLKOUT2_NOCOUNT",
            "CLKOUT3_EN",
            "CLKOUT3_NOCOUNT",
            "CLKOUT4_EN",
            "CLKOUT4_NOCOUNT",
            "CLKOUT5_EN",
            "CLKOUT5_NOCOUNT",
        ] {
            ctx.collect_bit_bi_legacy(tile, bel, attr, "FALSE", "TRUE");
        }
        if bel == "PLL[0]" {
            for attr in [
                "SS_EN",
                "CLKBURST_ENABLE",
                "CLKBURST_REPEAT",
                "INTERP_TEST",
                "CLKOUT6_EN",
                "CLKOUT6_NOCOUNT",
                "SEL_SLIPD",
                "CLKFBOUT_FRAC_EN",
                "CLKOUT0_FRAC_EN",
                "CLKFBOUT_USE_FINE_PS",
                "CLKOUT0_USE_FINE_PS",
                "CLKOUT1_USE_FINE_PS",
                "CLKOUT2_USE_FINE_PS",
                "CLKOUT3_USE_FINE_PS",
                "CLKOUT4_USE_FINE_PS",
                "CLKOUT5_USE_FINE_PS",
                "CLKOUT6_USE_FINE_PS",
                "CLKOUT4_CASCADE",
            ] {
                ctx.collect_bit_bi_legacy(tile, bel, attr, "FALSE", "TRUE");
            }
        }
        for attr in [
            "CLKFBIN_LT",
            "CLKFBIN_HT",
            "DIVCLK_LT",
            "DIVCLK_HT",
            "CLKFBOUT_LT",
            "CLKFBOUT_HT",
            "CLKFBOUT_DT",
            "CLKFBOUT_MX",
            "CLKOUT0_LT",
            "CLKOUT0_HT",
            "CLKOUT0_DT",
            "CLKOUT0_MX",
            "CLKOUT1_LT",
            "CLKOUT1_HT",
            "CLKOUT1_DT",
            "CLKOUT1_MX",
            "CLKOUT2_LT",
            "CLKOUT2_HT",
            "CLKOUT2_DT",
            "CLKOUT2_MX",
            "CLKOUT3_LT",
            "CLKOUT3_HT",
            "CLKOUT3_DT",
            "CLKOUT3_MX",
            "CLKOUT4_LT",
            "CLKOUT4_HT",
            "CLKOUT4_DT",
            "CLKOUT4_MX",
            "CLKOUT5_LT",
            "CLKOUT5_HT",
            "CLKOUT5_DT",
            "CLKOUT5_MX",
            "TMUX_MUX_SEL",
            "CONTROL_0",
            "CONTROL_1",
            "CONTROL_2",
            "CONTROL_3",
            "CONTROL_4",
            "CONTROL_5",
            "CONTROL_6",
            "CONTROL_7",
            "ANALOG_MISC",
            "CP_BIAS_TRIP_SET",
            "CP_RES",
            "EN_CURR_SINK",
            "AVDD_COMP_SET",
            "AVDD_VBG_PD",
            "AVDD_VBG_SEL",
            "DVDD_COMP_SET",
            "DVDD_VBG_PD",
            "DVDD_VBG_SEL",
            "FREQ_COMP",
            "IN_DLY_MX_CVDD",
            "IN_DLY_MX_DVDD",
            "LF_NEN",
            "LF_PEN",
            "MAN_LF",
            "PFD",
            "SKEW_FLOP_INV",
            "SPARE_DIGITAL",
            "VREF_START",
            "MVDD_SEL",
            "SYNTH_CLK_DIV",
            "CP",
            "HROW_DLY_SET",
            "HVLF_CNT_TEST",
            "LFHF",
            "LOCK_CNT",
            "LOCK_FB_DLY",
            "LOCK_REF_DLY",
            "LOCK_SAT_HIGH",
            "RES",
            "UNLOCK_CNT",
            "IN_DLY_SET",
        ] {
            ctx.collect_bitvec_legacy(tile, bel, attr, "");
        }
        if bel == "PLL[0]" {
            for attr in [
                "SS_STEPS",
                "SS_STEPS_INIT",
                "CLKFBOUT_PM_RISE",
                "CLKFBOUT_PM_FALL",
                "CLKOUT0_PM_RISE",
                "CLKOUT0_PM_FALL",
                "CLKOUT1_PM",
                "CLKOUT2_PM",
                "CLKOUT3_PM",
                "CLKOUT4_PM",
                "CLKOUT5_PM",
                "CLKOUT6_PM",
                "CLKOUT6_LT",
                "CLKOUT6_HT",
                "CLKOUT6_DT",
                "CLKOUT6_MX",
                "FINE_PS_FRAC",
                "CLKBURST_CNT",
                "INTERP_EN",
            ] {
                ctx.collect_bitvec_legacy(tile, bel, attr, "");
            }
            // THIS PIECE OF SHIT ACTUALLY CORRUPTS ITS OWN MEMORY TRYING TO COMPUTE THIS FUCKING ATTRIBUTE
            let mut diffs = ctx.get_diffs_legacy(tile, bel, "SPARE_ANALOG", "");
            assert!(diffs[1].bits.is_empty());
            diffs[1].bits.insert(TileBit::new(7, 28, 30), true);
            ctx.insert_legacy(tile, bel, "SPARE_ANALOG", xlat_bitvec_legacy(diffs));
        } else {
            for attr in [
                "CLKFBOUT_PM",
                "CLKOUT0_PM",
                "CLKOUT1_PM",
                "CLKOUT2_PM",
                "CLKOUT3_PM",
                "CLKOUT4_PM",
                "CLKOUT5_PM",
                "SPARE_ANALOG",
            ] {
                ctx.collect_bitvec_legacy(tile, bel, attr, "");
            }
        }
        for (addr, name) in [(0x16, "DIVCLK"), (0x17, "CLKFBIN")] {
            ctx.insert_legacy(
                tile,
                bel,
                format!("{name}_EDGE"),
                TileItem::from_bit_inv(drp_bit(bel, addr, 13), false),
            );
        }
        for (addr, name) in [
            (0x07, "CLKOUT5"),
            (0x09, "CLKOUT0"),
            (0x0b, "CLKOUT1"),
            (0x0d, "CLKOUT2"),
            (0x0f, "CLKOUT3"),
            (0x11, "CLKOUT4"),
            (0x13, "CLKOUT6"),
            (0x15, "CLKFBOUT"),
        ] {
            if name == "CLKOUT6" && bel == "PLL[1]" {
                continue;
            }
            ctx.insert_legacy(
                tile,
                bel,
                format!("{name}_EDGE"),
                TileItem::from_bit_inv(drp_bit(bel, addr, 7), false),
            );
        }
        if bel == "PLL[0]" {
            for (reg, bit, attr) in [
                (0x07, 10, "CLKOUT0_FRAC_WF_FALL"),
                (0x09, 10, "CLKOUT0_FRAC_WF_RISE"),
                (0x13, 10, "CLKFBOUT_FRAC_WF_FALL"),
                (0x15, 10, "CLKFBOUT_FRAC_WF_RISE"),
            ] {
                ctx.insert_legacy(
                    tile,
                    bel,
                    attr,
                    TileItem::from_bit_inv(drp_bit(bel, reg, bit), false),
                );
            }
            for (addr, name) in [(0x09, "CLKOUT0"), (0x15, "CLKFBOUT")] {
                ctx.insert_legacy(
                    tile,
                    bel,
                    format!("{name}_FRAC"),
                    TileItem::from_bitvec_inv(
                        vec![
                            drp_bit(bel, addr, 12),
                            drp_bit(bel, addr, 13),
                            drp_bit(bel, addr, 14),
                        ],
                        false,
                    ),
                );
            }
        }

        if bel == "PLL[0]" {
            ctx.insert_legacy(
                tile,
                bel,
                "MMCM_EN",
                TileItem::from_bit_inv(drp_bit(bel, 0x74, 0), false),
            );
        } else {
            ctx.insert_legacy(
                tile,
                bel,
                "PLL_EN",
                TileItem::from_bit_inv(drp_bit(bel, 0x5c, 0), false),
            );
        }

        let mut enable = ctx.get_diff_legacy(tile, bel, "ENABLE", "1");
        enable.apply_bit_diff_legacy(
            ctx.item_legacy(
                tile,
                bel,
                if bel == "PLL[0]" { "MMCM_EN" } else { "PLL_EN" },
            ),
            true,
            false,
        );
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKFBIN_HT"), 1, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKFBIN_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "DIVCLK_HT"), 1, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "DIVCLK_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKFBOUT_HT"), 1, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKFBOUT_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKOUT0_HT"), 1, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKOUT0_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKOUT1_HT"), 1, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKOUT1_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKOUT2_HT"), 1, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKOUT2_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKOUT3_HT"), 1, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKOUT3_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKOUT4_HT"), 1, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKOUT4_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKOUT5_HT"), 1, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKOUT5_LT"), 0x3f, 0);
        if bel == "PLL[0]" {
            enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "INTERP_EN"), 0x10, 0);
            enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "SS_STEPS_INIT"), 4, 0);
            enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "SS_STEPS"), 7, 0);
            enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKOUT6_HT"), 1, 0);
            enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKOUT6_LT"), 0x3f, 0);
        }
        enable.assert_empty();

        let modes = if bel == "PLL[0]" {
            &["LOW", "HIGH", "SS"][..]
        } else {
            &["LOW", "HIGH"][..]
        };
        let bel_kind = if bel == "PLL[0]" { "MMCM" } else { "PLL" };
        for mode in modes {
            for mult in 1..=64 {
                let mut diff = ctx.get_diff_legacy(tile, bel, "TABLES", format!("{mult}.{mode}"));
                for attr in ["CP", "RES", "LFHF"] {
                    let item = ctx.item_legacy(tile, bel, attr);
                    let base = BitVec::repeat(false, item.bits.len());
                    let val = extract_bitvec_val_part_legacy(item, &base, &mut diff);
                    let mut ival = 0;
                    for (i, v) in val.into_iter().enumerate() {
                        if v {
                            ival |= 1 << i;
                        }
                    }
                    ctx.insert_misc_data_legacy(format!("{bel_kind}:{attr}:{mode}:{mult}"), ival);
                }
                for attr in [
                    "LOCK_REF_DLY",
                    "LOCK_FB_DLY",
                    "LOCK_CNT",
                    "LOCK_SAT_HIGH",
                    "UNLOCK_CNT",
                ] {
                    let item = ctx.item_legacy(tile, bel, attr);
                    let base = BitVec::repeat(false, item.bits.len());
                    let val = extract_bitvec_val_part_legacy(item, &base, &mut diff);
                    let mut ival = 0;
                    for (i, v) in val.into_iter().enumerate() {
                        if v {
                            ival |= 1 << i;
                        }
                    }
                    ctx.insert_misc_data_legacy(format!("{bel_kind}:{attr}:{mult}"), ival);
                }
                diff.discard_bits_legacy(ctx.item_legacy(tile, bel, "CLKFBOUT_NOCOUNT"));
                diff.discard_bits_legacy(ctx.item_legacy(tile, bel, "CLKFBOUT_EDGE"));
                diff.discard_bits_legacy(ctx.item_legacy(tile, bel, "CLKFBOUT_LT"));
                diff.discard_bits_legacy(ctx.item_legacy(tile, bel, "CLKFBOUT_HT"));
                if bel == "PLL[0]" {
                    diff.discard_bits_legacy(ctx.item_legacy(tile, bel, "CLKFBOUT_PM_RISE"));
                    diff.discard_bits_legacy(ctx.item_legacy(tile, bel, "CLKFBOUT_PM_FALL"));
                    diff.discard_bits_legacy(ctx.item_legacy(tile, bel, "CLKFBOUT_FRAC_WF_RISE"));
                    diff.discard_bits_legacy(ctx.item_legacy(tile, bel, "CLKFBOUT_FRAC_WF_FALL"));
                }
                diff.assert_empty();
            }
        }

        let mut diff = ctx.get_diff_legacy(tile, bel, "COMPENSATION", "BUF_IN");
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IN_DLY_MX_DVDD"), 0x31, 0);
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IN_DLY_MX_CVDD"), 0x12, 0);
        diff.assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "COMPENSATION", "EXTERNAL");
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IN_DLY_MX_DVDD"), 0x31, 0);
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IN_DLY_MX_CVDD"), 0x12, 0);
        diff.assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "COMPENSATION", "INTERNAL");
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IN_DLY_MX_DVDD"), 0x2f, 0);
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IN_DLY_MX_CVDD"), 0x12, 0);
        diff.assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "COMPENSATION", "ZHOLD");
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IN_DLY_MX_DVDD"), 0x01, 0);
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IN_DLY_MX_CVDD"), 0x18, 0);
        diff.assert_empty();

        let bit = xlat_bit(ctx.get_diff_bel_special(tcid, bslot, specials::DRP_MASK_CMT));
        assert_eq!(bit.bit.rect.to_idx(), 50);
        let mut bit_w = bit;
        let mut bit_e = bit;
        bit_w.bit.rect = BitRectId::from_idx(0);
        bit_e.bit.rect = BitRectId::from_idx(1);
        if bslot == bslots::PLL[1] {
            ctx.insert_bel_attr_bool(
                tcls::HCLK,
                bslots::HCLK_DRP[0],
                bcls::HCLK_DRP::DRP_MASK_N,
                bit_w,
            );
            ctx.insert_bel_attr_bool(
                tcls::HCLK,
                bslots::HCLK_DRP[1],
                bcls::HCLK_DRP::DRP_MASK_N,
                bit_e,
            );
        } else {
            ctx.insert_bel_attr_bool(
                tcls::HCLK,
                bslots::HCLK_DRP[0],
                bcls::HCLK_DRP::DRP_MASK_S,
                bit_w,
            );
            ctx.insert_bel_attr_bool(
                tcls::HCLK,
                bslots::HCLK_DRP[1],
                bcls::HCLK_DRP::DRP_MASK_S,
                bit_e,
            );
        }
    }
}

fn collect_fuzzers_bufmrce(ctx: &mut CollectorCtx) {
    let tcid = tcls::CMT;

    for i in 0..2 {
        let bslot = bslots::BUFMRCE[i];
        ctx.collect_bel_attr(tcid, bslot, BUFHCE::ENABLE);
        ctx.collect_bel_input_inv_bi(tcid, bslot, BUFHCE::CE);
        ctx.collect_bel_attr_bi(tcid, bslot, BUFHCE::INIT_OUT);
        ctx.collect_bel_attr(tcid, bslot, BUFHCE::CE_TYPE);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    collect_fuzzers_fifo(ctx);
    collect_fuzzers_routing(ctx);
    collect_fuzzers_phaser(ctx);
    collect_fuzzers_pll(ctx);
    collect_fuzzers_bufmrce(ctx);
}
