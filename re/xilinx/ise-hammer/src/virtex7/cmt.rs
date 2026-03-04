use std::collections::BTreeSet;

use prjcombine_entity::EntityId;
use prjcombine_interconnect::db::{BelAttributeType, WireSlotIdExt};
use prjcombine_re_collector::diff::{
    Diff, OcdMode, extract_bitvec_val_part, extract_common_diff, xlat_bit, xlat_bit_wide,
    xlat_bitvec, xlat_enum_raw,
};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{
    bitvec::BitVec,
    bsdata::{BitRectId, TileBit},
};
use prjcombine_virtex4::defs::{
    bcls::{
        self, BUFHCE, IN_FIFO, OUT_FIFO, PHASER_IN, PHASER_OUT, PHASER_REF, PHY_CONTROL,
        PLL_V6 as PLL,
    },
    bslots, enums,
    virtex7::{tables::PLL_MULT, tcls, wires},
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

    for idx in 0..2 {
        let bslot = bslots::PLL[idx];
        let mut bctx = ctx.bel(bslot);
        let use_calc = if idx == 0 {
            "MMCMADV_*_USE_CALC"
        } else {
            "PLLADV_*_USE_CALC"
        };
        let mode = if idx == 0 { "MMCME2_ADV" } else { "PLLE2_ADV" };
        bctx.build()
            .global_xy(use_calc, "NO")
            .test_bel_attr_bits(PLL::ENABLE)
            .mode(mode)
            .commit();
        for pin in [
            PLL::CLKINSEL,
            PLL::PSEN,
            PLL::PSINCDEC,
            PLL::PWRDWN,
            PLL::RST,
        ] {
            if matches!(pin, PLL::PSEN | PLL::PSINCDEC) && idx == 1 {
                continue;
            }
            bctx.mode(mode)
                .mutex("MODE", "INV")
                .test_bel_input_inv_auto(pin);
        }
        for attr in [
            PLL::DIRECT_PATH_CNTRL,
            PLL::EN_VCO_DIV1,
            PLL::EN_VCO_DIV6,
            PLL::GTS_WAIT,
            PLL::HVLF_CNT_TEST_EN,
            PLL::IN_DLY_EN,
            PLL::LF_LOW_SEL,
            PLL::SEL_HV_NMOS,
            PLL::SEL_LV_NMOS,
            PLL::STARTUP_WAIT,
            PLL::SUP_SEL_AREG,
            PLL::SUP_SEL_DREG,
            PLL::VLF_HIGH_DIS_B,
            PLL::VLF_HIGH_PWDN_B,
            PLL::DIVCLK_NOCOUNT,
            PLL::CLKFBIN_NOCOUNT,
            PLL::CLKFBOUT_EN,
            PLL::CLKFBOUT_NOCOUNT,
            PLL::CLKOUT0_EN,
            PLL::CLKOUT0_NOCOUNT,
            PLL::CLKOUT1_EN,
            PLL::CLKOUT1_NOCOUNT,
            PLL::CLKOUT2_EN,
            PLL::CLKOUT2_NOCOUNT,
            PLL::CLKOUT3_EN,
            PLL::CLKOUT3_NOCOUNT,
            PLL::CLKOUT4_EN,
            PLL::CLKOUT4_NOCOUNT,
            PLL::CLKOUT5_EN,
            PLL::CLKOUT5_NOCOUNT,
        ] {
            bctx.mode(mode)
                .mutex("MODE", "TEST")
                .global_xy(use_calc, "NO")
                .test_bel_attr_bool_auto(attr, "FALSE", "TRUE");
        }
        if idx == 0 {
            for attr in [
                PLL::SEL_SLIPD,
                PLL::CLKBURST_ENABLE,
                PLL::CLKBURST_REPEAT,
                PLL::INTERP_TEST,
                PLL::CLKOUT6_EN,
                PLL::CLKOUT6_NOCOUNT,
            ] {
                bctx.mode(mode)
                    .mutex("MODE", "TEST")
                    .global_xy(use_calc, "NO")
                    .test_bel_attr_bool_auto(attr, "FALSE", "TRUE");
            }
            bctx.mode(mode)
                .mutex("MODE", "TEST")
                .global_xy(use_calc, "NO")
                .attr("CLKOUT6_EN", "TRUE")
                .attr("CLKOUT4_USE_FINE_PS", "")
                .attr("CLKOUT4_MX", "")
                .test_bel_attr_bool_auto(PLL::CLKOUT4_CASCADE, "FALSE", "TRUE");
            for attr in [
                PLL::CLKOUT0_USE_FINE_PS,
                PLL::CLKOUT1_USE_FINE_PS,
                PLL::CLKOUT2_USE_FINE_PS,
                PLL::CLKOUT3_USE_FINE_PS,
                PLL::CLKOUT4_USE_FINE_PS,
                PLL::CLKOUT5_USE_FINE_PS,
                PLL::CLKOUT6_USE_FINE_PS,
                PLL::CLKFBOUT_USE_FINE_PS,
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
                    .test_bel_attr_bool_auto(attr, "FALSE", "TRUE");
            }
            for attr in [PLL::CLKOUT0_FRAC_EN, PLL::CLKFBOUT_FRAC_EN] {
                bctx.mode(mode)
                    .mutex("MODE", "TEST")
                    .global_xy(use_calc, "NO")
                    .attr("CLKOUT5_EN", "TRUE")
                    .attr("CLKOUT6_EN", "TRUE")
                    .attr("INTERP_EN", "00000000")
                    .test_bel_attr_bool_auto(attr, "FALSE", "TRUE");
            }
        }
        for attr in [
            PLL::CLKFBIN_LT,
            PLL::CLKFBIN_HT,
            PLL::DIVCLK_LT,
            PLL::DIVCLK_HT,
            PLL::CLKFBOUT_LT,
            PLL::CLKFBOUT_HT,
            PLL::CLKFBOUT_DT,
            PLL::CLKFBOUT_MX,
            PLL::CLKOUT0_LT,
            PLL::CLKOUT0_HT,
            PLL::CLKOUT0_DT,
            PLL::CLKOUT0_MX,
            PLL::CLKOUT1_LT,
            PLL::CLKOUT1_HT,
            PLL::CLKOUT1_DT,
            PLL::CLKOUT1_MX,
            PLL::CLKOUT2_LT,
            PLL::CLKOUT2_HT,
            PLL::CLKOUT2_DT,
            PLL::CLKOUT2_MX,
            PLL::CLKOUT3_LT,
            PLL::CLKOUT3_HT,
            PLL::CLKOUT3_DT,
            PLL::CLKOUT3_MX,
            PLL::CLKOUT4_LT,
            PLL::CLKOUT4_HT,
            PLL::CLKOUT4_DT,
            PLL::CLKOUT4_MX,
            PLL::CLKOUT5_LT,
            PLL::CLKOUT5_HT,
            PLL::CLKOUT5_DT,
            PLL::CLKOUT5_MX,
            PLL::TMUX_MUX_SEL,
            PLL::CONTROL_0,
            PLL::CONTROL_1,
            PLL::CONTROL_2,
            PLL::CONTROL_3,
            PLL::CONTROL_4,
            PLL::CONTROL_5,
            PLL::CONTROL_6,
            PLL::CONTROL_7,
            PLL::ANALOG_MISC,
            PLL::CP_BIAS_TRIP_SET,
            PLL::CP_RES,
            PLL::EN_CURR_SINK,
            PLL::AVDD_VBG_PD,
            PLL::AVDD_VBG_SEL,
            PLL::DVDD_VBG_PD,
            PLL::DVDD_VBG_SEL,
            PLL::FREQ_COMP,
            PLL::IN_DLY_MX_CVDD,
            PLL::IN_DLY_MX_DVDD,
            PLL::LF_NEN,
            PLL::LF_PEN,
            PLL::MAN_LF,
            PLL::PFD,
            PLL::SKEW_FLOP_INV,
            PLL::SPARE_ANALOG,
            PLL::SPARE_DIGITAL,
            PLL::VREF_START,
            PLL::MVDD_SEL,
            PLL::SYNTH_CLK_DIV,
        ] {
            bctx.mode(mode)
                .mutex("MODE", "TEST")
                .global_xy(use_calc, "NO")
                .test_bel_attr_multi(attr, MultiValue::Bin);
        }
        for (attr, aname, width) in [
            (PLL::V7_AVDD_COMP_SET, "AVDD_COMP_SET", 3),
            (PLL::V7_DVDD_COMP_SET, "DVDD_COMP_SET", 3),
        ] {
            bctx.mode(mode)
                .mutex("MODE", "TEST")
                .global_xy(use_calc, "NO")
                .test_bel_attr_bits(attr)
                .multi_attr(aname, MultiValue::Bin, width);
        }
        if idx == 0 {
            for attr in [
                PLL::SS_STEPS,
                PLL::SS_STEPS_INIT,
                PLL::CLKFBOUT_PM_RISE,
                PLL::CLKFBOUT_PM_FALL,
                PLL::CLKOUT0_PM_RISE,
                PLL::CLKOUT0_PM_FALL,
                PLL::CLKOUT1_PM,
                PLL::CLKOUT2_PM,
                PLL::CLKOUT3_PM,
                PLL::CLKOUT4_PM,
                PLL::CLKOUT5_PM,
                PLL::CLKOUT6_PM,
                PLL::CLKOUT6_LT,
                PLL::CLKOUT6_HT,
                PLL::CLKOUT6_DT,
                PLL::CLKOUT6_MX,
                PLL::FINE_PS_FRAC,
            ] {
                bctx.mode(mode)
                    .mutex("MODE", "TEST")
                    .global_xy(use_calc, "NO")
                    .attr("INTERP_EN", "00000000")
                    .test_bel_attr_multi(attr, MultiValue::Bin);
            }
            bctx.mode(mode)
                .mutex("MODE", "TEST")
                .global_xy(use_calc, "NO")
                .test_bel_attr_multi(PLL::INTERP_EN, MultiValue::Bin);
        } else {
            for attr in [
                PLL::CLKFBOUT_PM,
                PLL::CLKOUT0_PM,
                PLL::CLKOUT1_PM,
                PLL::CLKOUT2_PM,
                PLL::CLKOUT3_PM,
                PLL::CLKOUT4_PM,
                PLL::CLKOUT5_PM,
            ] {
                bctx.mode(mode)
                    .mutex("MODE", "TEST")
                    .global_xy(use_calc, "NO")
                    .test_bel_attr_multi(attr, MultiValue::Bin);
            }
        }
        for attr in [
            PLL::CP,
            PLL::HROW_DLY_SET,
            PLL::HVLF_CNT_TEST,
            PLL::LFHF,
            PLL::LOCK_CNT,
            PLL::LOCK_FB_DLY,
            PLL::LOCK_REF_DLY,
            PLL::LOCK_SAT_HIGH,
            PLL::RES,
            PLL::UNLOCK_CNT,
        ] {
            bctx.mode(mode)
                .mutex("MODE", "TEST")
                .global_xy(use_calc, "NO")
                .test_bel_attr_multi(attr, MultiValue::Dec(0));
        }
        bctx.mode(mode)
            .mutex("MODE", "TEST")
            .global_xy(use_calc, "NO")
            .test_bel_attr_bits(PLL::V7_IN_DLY_SET)
            .multi_attr("IN_DLY_SET", MultiValue::Dec(0), 6);

        if idx == 0 {
            bctx.mode(mode)
                .mutex("MODE", "TEST")
                .global_xy(use_calc, "NO")
                .test_bel_attr_multi(PLL::CLKBURST_CNT, MultiValue::Dec(0));
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
                .test_bel_attr_bool_auto(PLL::SS_EN, "FALSE", "TRUE");
        }

        for mult in 1..=64 {
            if idx == 0 {
                for (spec, bandwidth) in [
                    (specials::PLL_TABLES_LOW, "LOW"),
                    (specials::PLL_TABLES_HIGH, "HIGH"),
                ] {
                    bctx.mode(mode)
                        .mutex("MODE", "CALC")
                        .global_xy(use_calc, "NO")
                        .attr("SS_EN", "FALSE")
                        .test_bel_special_u32(spec, mult)
                        .attr("CLKFBOUT_MULT_F", mult.to_string())
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
                    .test_bel_special_u32(specials::PLL_TABLES_SS, mult)
                    .attr("CLKFBOUT_MULT_F", mult.to_string())
                    .attr("BANDWIDTH", "LOW")
                    .commit();
            } else {
                for (spec, bandwidth) in [
                    (specials::PLL_TABLES_LOW, "LOW"),
                    (specials::PLL_TABLES_HIGH, "HIGH"),
                ] {
                    bctx.mode(mode)
                        .mutex("MODE", "CALC")
                        .global_xy(use_calc, "NO")
                        .test_bel_special_u32(spec, mult)
                        .attr("CLKFBOUT_MULT", mult.to_string())
                        .attr("BANDWIDTH", bandwidth)
                        .commit();
                }
            }
        }
        for (spec, val) in [
            (specials::PLL_COMPENSATION_ZHOLD, "ZHOLD"),
            (specials::PLL_COMPENSATION_EXTERNAL, "EXTERNAL"),
            (specials::PLL_COMPENSATION_INTERNAL, "INTERNAL"),
            (specials::PLL_COMPENSATION_BUF_IN, "BUF_IN"),
        ] {
            bctx.mode(mode)
                .mutex("MODE", "COMP")
                .global_xy(use_calc, "NO")
                .attr("HROW_DLY_SET", "000")
                .test_bel_special(spec)
                .attr("COMPENSATION", val)
                .commit();
        }
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
    for idx in 0..2 {
        let bslot = bslots::PLL[idx];

        fn drp_bit(idx: usize, reg: usize, bit: usize) -> TileBit {
            if idx == 0 {
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

        if idx == 0 {
            let mut drp = vec![];
            for reg in 0..0x80 {
                for bit in 0..16 {
                    drp.push(drp_bit(idx, reg, bit).pos());
                }
            }
            ctx.insert_bel_attr_bitvec(tcid, bslot, PLL::MMCM_DRP, drp);
        } else {
            let mut drp = vec![];
            for reg in 0..0x68 {
                for bit in 0..16 {
                    drp.push(drp_bit(idx, reg, bit).pos());
                }
            }
            ctx.insert_bel_attr_bitvec(tcid, bslot, PLL::PLL_DRP, drp);
        }

        for pin in [PLL::CLKINSEL, PLL::PWRDWN, PLL::RST] {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }
        if idx == 0 {
            for pin in [PLL::PSEN, PLL::PSINCDEC] {
                ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
            }
        }
        for attr in [
            PLL::DIRECT_PATH_CNTRL,
            PLL::EN_VCO_DIV1,
            PLL::EN_VCO_DIV6,
            PLL::GTS_WAIT,
            PLL::HVLF_CNT_TEST_EN,
            PLL::IN_DLY_EN,
            PLL::LF_LOW_SEL,
            PLL::SEL_HV_NMOS,
            PLL::SEL_LV_NMOS,
            PLL::STARTUP_WAIT,
            PLL::SUP_SEL_AREG,
            PLL::SUP_SEL_DREG,
            PLL::VLF_HIGH_DIS_B,
            PLL::VLF_HIGH_PWDN_B,
            PLL::DIVCLK_NOCOUNT,
            PLL::CLKFBIN_NOCOUNT,
            PLL::CLKFBOUT_EN,
            PLL::CLKFBOUT_NOCOUNT,
            PLL::CLKOUT0_EN,
            PLL::CLKOUT0_NOCOUNT,
            PLL::CLKOUT1_EN,
            PLL::CLKOUT1_NOCOUNT,
            PLL::CLKOUT2_EN,
            PLL::CLKOUT2_NOCOUNT,
            PLL::CLKOUT3_EN,
            PLL::CLKOUT3_NOCOUNT,
            PLL::CLKOUT4_EN,
            PLL::CLKOUT4_NOCOUNT,
            PLL::CLKOUT5_EN,
            PLL::CLKOUT5_NOCOUNT,
        ] {
            ctx.collect_bel_attr_bi(tcid, bslot, attr);
        }
        if idx == 0 {
            for attr in [
                PLL::SS_EN,
                PLL::CLKBURST_ENABLE,
                PLL::CLKBURST_REPEAT,
                PLL::INTERP_TEST,
                PLL::CLKOUT6_EN,
                PLL::CLKOUT6_NOCOUNT,
                PLL::SEL_SLIPD,
                PLL::CLKFBOUT_FRAC_EN,
                PLL::CLKOUT0_FRAC_EN,
                PLL::CLKFBOUT_USE_FINE_PS,
                PLL::CLKOUT0_USE_FINE_PS,
                PLL::CLKOUT1_USE_FINE_PS,
                PLL::CLKOUT2_USE_FINE_PS,
                PLL::CLKOUT3_USE_FINE_PS,
                PLL::CLKOUT4_USE_FINE_PS,
                PLL::CLKOUT5_USE_FINE_PS,
                PLL::CLKOUT6_USE_FINE_PS,
                PLL::CLKOUT4_CASCADE,
            ] {
                ctx.collect_bel_attr_bi(tcid, bslot, attr);
            }
        }
        for attr in [
            PLL::CLKFBIN_LT,
            PLL::CLKFBIN_HT,
            PLL::DIVCLK_LT,
            PLL::DIVCLK_HT,
            PLL::CLKFBOUT_LT,
            PLL::CLKFBOUT_HT,
            PLL::CLKFBOUT_DT,
            PLL::CLKFBOUT_MX,
            PLL::CLKOUT0_LT,
            PLL::CLKOUT0_HT,
            PLL::CLKOUT0_DT,
            PLL::CLKOUT0_MX,
            PLL::CLKOUT1_LT,
            PLL::CLKOUT1_HT,
            PLL::CLKOUT1_DT,
            PLL::CLKOUT1_MX,
            PLL::CLKOUT2_LT,
            PLL::CLKOUT2_HT,
            PLL::CLKOUT2_DT,
            PLL::CLKOUT2_MX,
            PLL::CLKOUT3_LT,
            PLL::CLKOUT3_HT,
            PLL::CLKOUT3_DT,
            PLL::CLKOUT3_MX,
            PLL::CLKOUT4_LT,
            PLL::CLKOUT4_HT,
            PLL::CLKOUT4_DT,
            PLL::CLKOUT4_MX,
            PLL::CLKOUT5_LT,
            PLL::CLKOUT5_HT,
            PLL::CLKOUT5_DT,
            PLL::CLKOUT5_MX,
            PLL::TMUX_MUX_SEL,
            PLL::CONTROL_0,
            PLL::CONTROL_1,
            PLL::CONTROL_2,
            PLL::CONTROL_3,
            PLL::CONTROL_4,
            PLL::CONTROL_5,
            PLL::CONTROL_6,
            PLL::CONTROL_7,
            PLL::ANALOG_MISC,
            PLL::CP_BIAS_TRIP_SET,
            PLL::CP_RES,
            PLL::EN_CURR_SINK,
            PLL::V7_AVDD_COMP_SET,
            PLL::AVDD_VBG_PD,
            PLL::AVDD_VBG_SEL,
            PLL::V7_DVDD_COMP_SET,
            PLL::DVDD_VBG_PD,
            PLL::DVDD_VBG_SEL,
            PLL::FREQ_COMP,
            PLL::IN_DLY_MX_CVDD,
            PLL::IN_DLY_MX_DVDD,
            PLL::LF_NEN,
            PLL::LF_PEN,
            PLL::MAN_LF,
            PLL::PFD,
            PLL::SKEW_FLOP_INV,
            PLL::SPARE_DIGITAL,
            PLL::VREF_START,
            PLL::MVDD_SEL,
            PLL::SYNTH_CLK_DIV,
            PLL::CP,
            PLL::HROW_DLY_SET,
            PLL::HVLF_CNT_TEST,
            PLL::LFHF,
            PLL::LOCK_CNT,
            PLL::LOCK_FB_DLY,
            PLL::LOCK_REF_DLY,
            PLL::LOCK_SAT_HIGH,
            PLL::RES,
            PLL::UNLOCK_CNT,
            PLL::V7_IN_DLY_SET,
        ] {
            ctx.collect_bel_attr(tcid, bslot, attr);
        }
        if idx == 0 {
            for attr in [
                PLL::SS_STEPS,
                PLL::SS_STEPS_INIT,
                PLL::CLKFBOUT_PM_RISE,
                PLL::CLKFBOUT_PM_FALL,
                PLL::CLKOUT0_PM_RISE,
                PLL::CLKOUT0_PM_FALL,
                PLL::CLKOUT1_PM,
                PLL::CLKOUT2_PM,
                PLL::CLKOUT3_PM,
                PLL::CLKOUT4_PM,
                PLL::CLKOUT5_PM,
                PLL::CLKOUT6_PM,
                PLL::CLKOUT6_LT,
                PLL::CLKOUT6_HT,
                PLL::CLKOUT6_DT,
                PLL::CLKOUT6_MX,
                PLL::FINE_PS_FRAC,
                PLL::CLKBURST_CNT,
                PLL::INTERP_EN,
            ] {
                ctx.collect_bel_attr(tcid, bslot, attr);
            }
            // THIS PIECE OF SHIT ACTUALLY CORRUPTS ITS OWN MEMORY TRYING TO COMPUTE THIS FUCKING ATTRIBUTE
            let mut diffs = ctx.get_diffs_attr_bits(tcid, bslot, PLL::SPARE_ANALOG, 5);
            assert!(diffs[1].bits.is_empty());
            diffs[1].bits.insert(TileBit::new(7, 28, 30), true);
            ctx.insert_bel_attr_bitvec(tcid, bslot, PLL::SPARE_ANALOG, xlat_bitvec(diffs));
        } else {
            for attr in [
                PLL::CLKFBOUT_PM,
                PLL::CLKOUT0_PM,
                PLL::CLKOUT1_PM,
                PLL::CLKOUT2_PM,
                PLL::CLKOUT3_PM,
                PLL::CLKOUT4_PM,
                PLL::CLKOUT5_PM,
                PLL::SPARE_ANALOG,
            ] {
                ctx.collect_bel_attr(tcid, bslot, attr);
            }
        }
        for (addr, attr) in [(0x16, PLL::DIVCLK_EDGE), (0x17, PLL::CLKFBIN_EDGE)] {
            ctx.insert_bel_attr_bool(tcid, bslot, attr, drp_bit(idx, addr, 13).pos());
        }
        for (addr, attr) in [
            (0x07, PLL::CLKOUT5_EDGE),
            (0x09, PLL::CLKOUT0_EDGE),
            (0x0b, PLL::CLKOUT1_EDGE),
            (0x0d, PLL::CLKOUT2_EDGE),
            (0x0f, PLL::CLKOUT3_EDGE),
            (0x11, PLL::CLKOUT4_EDGE),
            (0x13, PLL::CLKOUT6_EDGE),
            (0x15, PLL::CLKFBOUT_EDGE),
        ] {
            if attr == PLL::CLKOUT6_EDGE && idx == 1 {
                continue;
            }
            ctx.insert_bel_attr_bool(tcid, bslot, attr, drp_bit(idx, addr, 7).pos());
        }
        if idx == 0 {
            for (reg, bit, attr) in [
                (0x07, 10, PLL::CLKOUT0_FRAC_WF_FALL),
                (0x09, 10, PLL::CLKOUT0_FRAC_WF_RISE),
                (0x13, 10, PLL::CLKFBOUT_FRAC_WF_FALL),
                (0x15, 10, PLL::CLKFBOUT_FRAC_WF_RISE),
            ] {
                ctx.insert_bel_attr_bool(tcid, bslot, attr, drp_bit(idx, reg, bit).pos());
            }
            for (addr, attr) in [(0x09, PLL::CLKOUT0_FRAC), (0x15, PLL::CLKFBOUT_FRAC)] {
                ctx.insert_bel_attr_bitvec(
                    tcid,
                    bslot,
                    attr,
                    vec![
                        drp_bit(idx, addr, 12).pos(),
                        drp_bit(idx, addr, 13).pos(),
                        drp_bit(idx, addr, 14).pos(),
                    ],
                );
            }
        }

        if idx == 0 {
            ctx.insert_bel_attr_bool(tcid, bslot, PLL::ENABLE, drp_bit(idx, 0x74, 0).pos());
        } else {
            ctx.insert_bel_attr_bool(tcid, bslot, PLL::ENABLE, drp_bit(idx, 0x5c, 0).pos());
        }

        let mut enable = ctx.get_diff_attr_bool(tcid, bslot, PLL::ENABLE);
        enable.apply_bit_diff(ctx.bel_attr_bit(tcid, bslot, PLL::ENABLE), true, false);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKFBIN_HT), 1, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKFBIN_LT), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::DIVCLK_HT), 1, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::DIVCLK_LT), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKFBOUT_HT), 1, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKFBOUT_LT), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKOUT0_HT), 1, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKOUT0_LT), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKOUT1_HT), 1, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKOUT1_LT), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKOUT2_HT), 1, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKOUT2_LT), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKOUT3_HT), 1, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKOUT3_LT), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKOUT4_HT), 1, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKOUT4_LT), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKOUT5_HT), 1, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKOUT5_LT), 0x3f, 0);
        if idx == 0 {
            enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::INTERP_EN), 0x10, 0);
            enable.apply_bitvec_diff_int(
                ctx.bel_attr_bitvec(tcid, bslot, PLL::SS_STEPS_INIT),
                4,
                0,
            );
            enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::SS_STEPS), 7, 0);
            enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKOUT6_HT), 1, 0);
            enable.apply_bitvec_diff_int(
                ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKOUT6_LT),
                0x3f,
                0,
            );
        }
        enable.assert_empty();

        for (spec, field_cp, field_res, field_lfhf) in [
            (
                specials::PLL_TABLES_LOW,
                PLL_MULT::PLL_CP_LOW,
                PLL_MULT::PLL_RES_LOW,
                PLL_MULT::PLL_LFHF_LOW,
            ),
            (
                specials::PLL_TABLES_HIGH,
                PLL_MULT::PLL_CP_HIGH,
                PLL_MULT::PLL_RES_HIGH,
                PLL_MULT::PLL_LFHF_HIGH,
            ),
            (
                specials::PLL_TABLES_SS,
                PLL_MULT::PLL_CP_SS,
                PLL_MULT::PLL_RES_SS,
                PLL_MULT::PLL_LFHF_SS,
            ),
        ] {
            if idx == 1 && spec == specials::PLL_TABLES_SS {
                continue;
            }
            for mult in 1..=64 {
                let row = ctx.edev.db[PLL_MULT]
                    .rows
                    .get(&if idx == 0 {
                        format!("MMCM_{mult}")
                    } else {
                        format!("PLL_{mult}")
                    })
                    .unwrap()
                    .0;
                let mut diff = ctx.get_diff_bel_special_u32(tcid, bslot, spec, mult);
                for (attr, field) in [
                    (PLL::CP, field_cp),
                    (PLL::RES, field_res),
                    (PLL::LFHF, field_lfhf),
                ] {
                    let item = ctx.bel_attr_bitvec(tcid, bslot, attr);
                    let base = BitVec::repeat(false, item.len());
                    let val = extract_bitvec_val_part(item, &base, &mut diff);
                    ctx.insert_table_bitvec(PLL_MULT, row, field, val);
                }
                for (attr, field) in [
                    (PLL::LOCK_REF_DLY, PLL_MULT::LOCK_REF_DLY),
                    (PLL::LOCK_FB_DLY, PLL_MULT::LOCK_FB_DLY),
                    (PLL::LOCK_CNT, PLL_MULT::LOCK_CNT),
                    (PLL::LOCK_SAT_HIGH, PLL_MULT::LOCK_SAT_HIGH),
                    (PLL::UNLOCK_CNT, PLL_MULT::UNLOCK_CNT),
                ] {
                    let item = ctx.bel_attr_bitvec(tcid, bslot, attr);
                    let base = BitVec::repeat(false, item.len());
                    let val = extract_bitvec_val_part(item, &base, &mut diff);
                    ctx.insert_table_bitvec(PLL_MULT, row, field, val);
                }
                diff.discard_polbits(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKFBOUT_NOCOUNT));
                diff.discard_polbits(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKFBOUT_EDGE));
                diff.discard_polbits(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKFBOUT_LT));
                diff.discard_polbits(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKFBOUT_HT));
                if idx == 0 {
                    diff.discard_polbits(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKFBOUT_PM_RISE));
                    diff.discard_polbits(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKFBOUT_PM_FALL));
                    diff.discard_polbits(ctx.bel_attr_bitvec(
                        tcid,
                        bslot,
                        PLL::CLKFBOUT_FRAC_WF_RISE,
                    ));
                    diff.discard_polbits(ctx.bel_attr_bitvec(
                        tcid,
                        bslot,
                        PLL::CLKFBOUT_FRAC_WF_FALL,
                    ));
                }
                diff.assert_empty();
            }
        }

        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::PLL_COMPENSATION_BUF_IN);
        diff.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::IN_DLY_MX_DVDD),
            0x31,
            0,
        );
        diff.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::IN_DLY_MX_CVDD),
            0x12,
            0,
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::PLL_COMPENSATION_EXTERNAL);
        diff.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::IN_DLY_MX_DVDD),
            0x31,
            0,
        );
        diff.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::IN_DLY_MX_CVDD),
            0x12,
            0,
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::PLL_COMPENSATION_INTERNAL);
        diff.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::IN_DLY_MX_DVDD),
            0x2f,
            0,
        );
        diff.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::IN_DLY_MX_CVDD),
            0x12,
            0,
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::PLL_COMPENSATION_ZHOLD);
        diff.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::IN_DLY_MX_DVDD),
            0x01,
            0,
        );
        diff.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::IN_DLY_MX_CVDD),
            0x18,
            0,
        );
        diff.assert_empty();

        let bit = xlat_bit(ctx.get_diff_bel_special(tcid, bslot, specials::DRP_MASK_CMT));
        assert_eq!(bit.bit.rect.to_idx(), 50);
        let mut bit_w = bit;
        let mut bit_e = bit;
        bit_w.bit.rect = BitRectId::from_idx(0);
        bit_e.bit.rect = BitRectId::from_idx(1);
        if idx == 1 {
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
