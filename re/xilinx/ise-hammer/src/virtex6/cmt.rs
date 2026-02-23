use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::{db::WireSlotIdExt, dir::DirH, grid::TileCoord};
use prjcombine_re_collector::{
    diff::{Diff, DiffKey, OcdMode, xlat_bit, xlat_enum_raw},
    legacy::{extract_bitvec_val_part_legacy, xlat_bit_legacy},
};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{
    bits,
    bitvec::BitVec,
    bsdata::{BitRectId, TileBit, TileItem},
};
use prjcombine_virtex4::defs::{
    self,
    bcls::{self, BUFHCE, PLL_V6 as PLL},
    bslots,
    virtex6::{tcls, wires},
};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::{BaseIntPip, FuzzIntPip},
        props::{
            mutex::{WireMutexExclusive, WireMutexShared},
            relation::TileRelation,
        },
    },
    virtex4::specials,
};

#[derive(Clone, Copy, Debug)]
struct HclkIoiInnerSide(DirH);

impl TileRelation for HclkIoiInnerSide {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let col = match self.0 {
            DirH::W => edev.col_io_iw.unwrap(),
            DirH::E => edev.col_io_ie.unwrap(),
        };
        let row = edev.chips[tcrd.die].row_hclk(tcrd.row);
        Some(tcrd.with_cr(col, row).tile(defs::tslots::HCLK_BEL))
    }
}

pub fn add_fuzzers<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
    devdata_only: bool,
) {
    let tcid = tcls::CMT;
    let mut ctx = FuzzCtx::new(session, backend, tcid);
    if devdata_only {
        for i in 0..2 {
            let mut bctx = ctx.bel(bslots::PLL[i]);
            bctx.mode("MMCM_ADV")
                .mutex("MODE", "COMP")
                .global_xy("MMCMADV_*_USE_CALC", "NO")
                .attr("HROW_DLY_SET", "000")
                .test_enum_legacy("COMPENSATION", &["ZHOLD"]);
        }
        return;
    }
    for slots in [bslots::BUFHCE_W, bslots::BUFHCE_E] {
        for i in 0..12 {
            let mut bctx = ctx.bel(slots[i]);
            let mode = "BUFHCE";
            bctx.build()
                .test_bel_attr_bits(BUFHCE::ENABLE)
                .mode(mode)
                .commit();
            bctx.mode(mode).test_bel_input_inv_auto(BUFHCE::CE);
            bctx.mode(mode)
                .test_bel_attr_bool_auto(BUFHCE::INIT_OUT, "0", "1");
        }
    }
    for i in 0..2 {
        let mut bctx = ctx.bel(bslots::PLL[i]);
        let mode = "MMCM_ADV";

        bctx.build()
            .global_xy("MMCMADV_*_USE_CALC", "NO")
            .test_manual_legacy("ENABLE", "1")
            .mode(mode)
            .commit();

        for pin in [
            PLL::RST,
            PLL::PWRDWN,
            PLL::PSINCDEC,
            PLL::PSEN,
            PLL::CLKINSEL,
        ] {
            bctx.mode(mode)
                .mutex("MODE", "PIN")
                .test_bel_input_inv_auto(pin);
        }

        for attr in [
            "CASC_LOCK_EN",
            "CLKBURST_ENABLE",
            "CLKBURST_REPEAT",
            "CLKFBOUT_EN",
            "CLKOUT0_EN",
            "CLKOUT1_EN",
            "CLKOUT2_EN",
            "CLKOUT3_EN",
            "CLKOUT4_EN",
            "CLKOUT5_EN",
            "CLKOUT6_EN",
            "DIRECT_PATH_CNTRL",
            "CLOCK_HOLD",
            "EN_VCO_DIV1",
            "EN_VCO_DIV6",
            "HVLF_STEP",
            "HVLF_CNT_TEST_EN",
            "IN_DLY_EN",
            "STARTUP_WAIT",
            "VLF_HIGH_DIS_B",
            "VLF_HIGH_PWDN_B",
        ] {
            bctx.mode(mode)
                .global_xy("MMCMADV_*_USE_CALC", "NO")
                .mutex("MODE", "TEST")
                .test_enum_legacy(attr, &["FALSE", "TRUE"]);
        }
        bctx.mode(mode)
            .global_xy("MMCMADV_*_USE_CALC", "NO")
            .mutex("MODE", "TEST")
            .attr("CLKOUT6_EN", "TRUE")
            .attr("CLKOUT4_USE_FINE_PS", "")
            .attr("CLKOUT4_MX", "")
            .test_enum_legacy("CLKOUT4_CASCADE", &["FALSE", "TRUE"]);
        bctx.mode(mode)
            .global_xy("MMCMADV_*_USE_CALC", "NO")
            .mutex("MODE", "TEST")
            .attr("STARTUP_WAIT", "FALSE")
            .test_enum_legacy("GTS_WAIT", &["FALSE", "TRUE"]);
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
                .global_xy("MMCMADV_*_USE_CALC", "NO")
                .mutex("MODE", "TEST")
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
                .global_xy("MMCMADV_*_USE_CALC", "NO")
                .mutex("MODE", "TEST")
                .attr("CLKOUT5_EN", "TRUE")
                .attr("CLKOUT6_EN", "TRUE")
                .attr("INTERP_EN", "00000000")
                .test_enum_legacy(attr, &["FALSE", "TRUE"]);
        }

        for (attr, width) in [
            ("ANALOG_MISC", 4),
            ("AVDD_COMP_SET", 2),
            ("AVDD_VBG_PD", 3),
            ("AVDD_VBG_SEL", 4),
            ("CLKFBIN_HT", 6),
            ("CLKFBIN_LT", 6),
            ("CLKFBOUT_DT", 6),
            ("CLKFBOUT_FRAC", 3),
            ("CLKFBOUT_HT", 6),
            ("CLKFBOUT_LT", 6),
            ("CLKFBOUT_MX", 2),
            ("CLKFBOUT_FRAC", 3),
            ("CLKOUT0_DT", 6),
            ("CLKOUT0_HT", 6),
            ("CLKOUT0_LT", 6),
            ("CLKOUT0_MX", 2),
            ("CLKOUT0_FRAC", 3),
            ("CLKOUT1_DT", 6),
            ("CLKOUT1_HT", 6),
            ("CLKOUT1_LT", 6),
            ("CLKOUT1_MX", 2),
            ("CLKOUT2_DT", 6),
            ("CLKOUT2_HT", 6),
            ("CLKOUT2_LT", 6),
            ("CLKOUT2_MX", 2),
            ("CLKOUT3_DT", 6),
            ("CLKOUT3_HT", 6),
            ("CLKOUT3_LT", 6),
            ("CLKOUT3_MX", 2),
            ("CLKOUT4_DT", 6),
            ("CLKOUT4_HT", 6),
            ("CLKOUT4_LT", 6),
            ("CLKOUT4_MX", 2),
            ("CLKOUT5_DT", 6),
            ("CLKOUT5_HT", 6),
            ("CLKOUT5_LT", 6),
            ("CLKOUT5_MX", 2),
            ("CLKOUT6_DT", 6),
            ("CLKOUT6_HT", 6),
            ("CLKOUT6_LT", 6),
            ("CLKOUT6_MX", 2),
            ("CONTROL_0", 16),
            ("CONTROL_1", 16),
            ("CONTROL_2", 16),
            ("CONTROL_3", 16),
            ("CONTROL_4", 16),
            ("CONTROL_5", 16),
            ("CP_BIAS_TRIP_SET", 1),
            ("CP_RES", 2),
            ("DIVCLK_HT", 6),
            ("DIVCLK_LT", 6),
            ("DVDD_COMP_SET", 2),
            ("DVDD_VBG_PD", 3),
            ("DVDD_VBG_SEL", 4),
            ("INTERP_EN", 8),
            ("IN_DLY_MX_CVDD", 6),
            ("IN_DLY_MX_DVDD", 6),
            ("LF_NEN", 2),
            ("LF_PEN", 2),
            ("MAN_LF", 3),
            ("PFD", 7),
            ("TMUX_MUX_SEL", 2),
        ] {
            bctx.mode(mode)
                .global_xy("MMCMADV_*_USE_CALC", "NO")
                .mutex("MODE", "TEST")
                .attr("CLKOUT0_DIVIDE_F", "1.5")
                .attr("CLKFBOUT_MULT_F", "1.5")
                .test_multi_attr_bin_legacy(attr, width);
        }
        for (attr, width) in [
            ("CLKFBOUT_PM", 3),
            ("CLKOUT0_PM", 3),
            ("CLKOUT1_PM", 3),
            ("CLKOUT2_PM", 3),
            ("CLKOUT3_PM", 3),
            ("CLKOUT4_PM", 3),
            ("CLKOUT5_PM", 3),
            ("CLKOUT6_PM", 3),
            ("FINE_PS_FRAC", 6),
        ] {
            bctx.mode(mode)
                .global_xy("MMCMADV_*_USE_CALC", "NO")
                .mutex("MODE", "TEST")
                .attr("INTERP_EN", "00000000")
                .test_multi_attr_bin_legacy(attr, width);
        }
        for (attr, width) in [
            ("CLKBURST_CNT", 4),
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
        ] {
            bctx.mode(mode)
                .global_xy("MMCMADV_*_USE_CALC", "NO")
                .mutex("MODE", "TEST")
                .test_multi_attr_dec_legacy(attr, width);
        }

        for mult in 1..=64 {
            for bandwidth in ["LOW", "HIGH"] {
                bctx.mode(mode)
                    .mutex("MODE", "CALC")
                    .global_xy("MMCMADV_*_USE_CALC", "NO")
                    .test_manual_legacy("TABLES", format!("{mult}.{bandwidth}"))
                    .attr("CLKFBOUT_MULT_F", format!("{mult}"))
                    .attr("BANDWIDTH", bandwidth)
                    .commit();
            }
        }
        bctx.mode(mode)
            .mutex("MODE", "COMP")
            .global_xy("MMCMADV_*_USE_CALC", "NO")
            .attr("HROW_DLY_SET", "000")
            .test_enum_legacy(
                "COMPENSATION",
                &["ZHOLD", "EXTERNAL", "INTERNAL", "BUF_IN", "CASCADE"],
            );
    }
    {
        let mut bctx = ctx.bel(bslots::PPR_FRAME);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("PPR_FRAME")
            .commit();
    }

    for wires in [wires::IMUX_BUFHCE_W, wires::IMUX_BUFHCE_E] {
        for i in 0..12 {
            let dst = wires[i].cell(20);
            let odst = wires[if i < 6 { i + 6 } else { i - 6 }].cell(20);
            let mux = &backend.edev.db_index[tcid].muxes[&dst];
            for &src in mux.src.keys() {
                let mut builder = ctx
                    .build()
                    .prop(WireMutexExclusive::new(dst))
                    .prop(WireMutexShared::new(src.tw));
                if wires::CCIO_CMT_W.contains(src.wire) || wires::CCIO_CMT_E.contains(src.wire) {
                    builder = builder
                        .tile_mutex("CCIO", "USE")
                        .prop(WireMutexExclusive::new(odst))
                        .prop(BaseIntPip::new(odst, src.tw))
                        .prop(FuzzIntPip::new(dst, src.tw));
                } else if wires::GCLK_CMT.contains(src.wire) {
                    builder = builder
                        .global_mutex("GCLK", "USE")
                        .prop(WireMutexExclusive::new(odst))
                        .prop(BaseIntPip::new(odst, src.tw))
                        .prop(FuzzIntPip::new(dst, src.tw));
                } else if wires::BUFH_INT_W.contains(src.wire)
                    || wires::BUFH_INT_E.contains(src.wire)
                {
                    builder = builder
                        .tile_mutex("BUFHCE_CKINT", "USE")
                        .prop(WireMutexExclusive::new(odst))
                        .prop(BaseIntPip::new(odst, src.tw))
                        .prop(FuzzIntPip::new(dst, src.tw));
                } else {
                    builder = builder.prop(FuzzIntPip::new(dst, src.tw));
                }
                builder.test_routing(dst, src).commit();
            }
        }
    }
    {
        for (wt, wf) in [
            (wires::IMUX_BUFHCE_W[0], wires::BUFH_INT_W),
            (wires::IMUX_BUFHCE_E[0], wires::BUFH_INT_E),
        ] {
            for wf in wf {
                let dst = wt.cell(20);
                let src = wf.cell(20);
                let far_src = backend.edev.db_index[tcid].pips_bwd[&src]
                    .iter()
                    .next()
                    .copied()
                    .unwrap();
                ctx.build()
                    .tile_mutex("BUFHCE_CKINT", "TEST")
                    .prop(WireMutexExclusive::new(dst))
                    .prop(WireMutexExclusive::new(src))
                    .prop(WireMutexShared::new(far_src.tw))
                    .test_routing(dst, far_src)
                    .prop(FuzzIntPip::new(dst, src))
                    .commit();
            }
        }
    }
    {
        let dst = wires::IMUX_BUFHCE_W[0].cell(20);
        for wf in wires::CCIO_CMT_W.into_iter().chain(wires::CCIO_CMT_E) {
            let src = wf.cell(20);
            let far_src = backend.edev.db_index[tcid].pips_bwd[&src]
                .iter()
                .next()
                .copied()
                .unwrap();
            ctx.build()
                .tile_mutex("CCIO", "TEST")
                .prop(WireMutexExclusive::new(dst))
                .prop(WireMutexExclusive::new(src))
                .prop(WireMutexShared::new(far_src.tw))
                .test_routing(dst, far_src)
                .prop(FuzzIntPip::new(dst, src))
                .commit();
        }
    }
    for (lr, wt, wf) in [
        ('L', wires::BUFH_TEST_W, wires::BUFH_TEST_W_IN),
        ('R', wires::BUFH_TEST_E, wires::BUFH_TEST_E_IN),
    ] {
        let dst = wt.cell(20);
        let src = wf.cell(20);
        let mut bctx = ctx.bel(bslots::SPEC_INT);
        bctx.build()
            .prop(WireMutexExclusive::new(dst))
            .prop(WireMutexShared::new(src))
            .test_raw(DiffKey::RoutingInv(tcid, dst, false))
            .pip(
                format!("BUFH_TEST_{lr}_NOINV"),
                format!("BUFH_TEST_{lr}_PRE"),
            )
            .commit();
        bctx.build()
            .prop(WireMutexExclusive::new(dst))
            .prop(WireMutexShared::new(src))
            .test_raw(DiffKey::RoutingInv(tcid, dst, true))
            .pip(format!("BUFH_TEST_{lr}_INV"), format!("BUFH_TEST_{lr}_PRE"))
            .commit();
    }
    for i in 0..32 {
        let mut bctx = ctx.bel(bslots::SPEC_INT);
        let wire_pin = wires::IMUX_BUFHCE_W[i % 12].cell(20);
        let dst = wires::GCLK_TEST[i].cell(20);
        let src = wires::GCLK_CMT[i].cell(20);
        bctx.build()
            .global_mutex("GCLK", "USE")
            .prop(WireMutexExclusive::new(dst))
            .prop(WireMutexShared::new(src))
            .prop(WireMutexExclusive::new(wire_pin))
            .prop(BaseIntPip::new(wire_pin, src))
            .test_routing(dst, src.pos())
            .pip(format!("GCLK{i}_NOINV"), format!("GCLK{i}"))
            .commit();
        bctx.build()
            .global_mutex("GCLK", "USE")
            .prop(WireMutexExclusive::new(dst))
            .prop(WireMutexShared::new(src))
            .prop(WireMutexExclusive::new(wire_pin))
            .prop(BaseIntPip::new(wire_pin, src))
            .test_routing(dst, src.neg())
            .pip(format!("GCLK{i}_INV"), format!("GCLK{i}"))
            .commit();
    }
    for (side, wt, wp, do_far) in [
        (
            DirH::W,
            wires::BUFH_TEST_W_IN,
            wires::IMUX_MMCM_CLKIN1_HCLK_W[0],
            true,
        ),
        (
            DirH::W,
            wires::IMUX_MMCM_CLKIN1_HCLK_W[0],
            wires::IMUX_MMCM_CLKIN1_HCLK_W[1],
            false,
        ),
        (
            DirH::W,
            wires::IMUX_MMCM_CLKIN1_HCLK_W[1],
            wires::IMUX_MMCM_CLKIN1_HCLK_W[0],
            false,
        ),
        (
            DirH::W,
            wires::IMUX_MMCM_CLKIN2_HCLK_W[0],
            wires::IMUX_MMCM_CLKIN2_HCLK_W[1],
            false,
        ),
        (
            DirH::W,
            wires::IMUX_MMCM_CLKIN2_HCLK_W[1],
            wires::IMUX_MMCM_CLKIN2_HCLK_W[0],
            false,
        ),
        (
            DirH::W,
            wires::IMUX_MMCM_CLKFB_HCLK_W[0],
            wires::IMUX_MMCM_CLKFB_HCLK_W[1],
            false,
        ),
        (
            DirH::W,
            wires::IMUX_MMCM_CLKFB_HCLK_W[1],
            wires::IMUX_MMCM_CLKFB_HCLK_W[0],
            false,
        ),
        (
            DirH::E,
            wires::BUFH_TEST_E_IN,
            wires::IMUX_MMCM_CLKIN1_HCLK_E[0],
            true,
        ),
        (
            DirH::E,
            wires::IMUX_MMCM_CLKIN1_HCLK_E[0],
            wires::IMUX_MMCM_CLKIN1_HCLK_E[1],
            false,
        ),
        (
            DirH::E,
            wires::IMUX_MMCM_CLKIN1_HCLK_E[1],
            wires::IMUX_MMCM_CLKIN1_HCLK_E[0],
            false,
        ),
        (
            DirH::E,
            wires::IMUX_MMCM_CLKIN2_HCLK_E[0],
            wires::IMUX_MMCM_CLKIN2_HCLK_E[1],
            false,
        ),
        (
            DirH::E,
            wires::IMUX_MMCM_CLKIN2_HCLK_E[1],
            wires::IMUX_MMCM_CLKIN2_HCLK_E[0],
            false,
        ),
        (
            DirH::E,
            wires::IMUX_MMCM_CLKFB_HCLK_E[0],
            wires::IMUX_MMCM_CLKFB_HCLK_E[1],
            false,
        ),
        (
            DirH::E,
            wires::IMUX_MMCM_CLKFB_HCLK_E[1],
            wires::IMUX_MMCM_CLKFB_HCLK_E[0],
            false,
        ),
    ] {
        let dst = wt.cell(20);
        let odst = wp.cell(20);
        for i in 0..12 {
            let src = match side {
                DirH::W => wires::HCLK_CMT_W[i].cell(20),
                DirH::E => wires::HCLK_CMT_E[i].cell(20),
            };
            let far_src = backend.edev.db_index[tcid].pips_bwd[&src]
                .iter()
                .next()
                .copied()
                .unwrap();

            ctx.build()
                .global_mutex("HCLK", "USE")
                .row_mutex("BUFH_TEST", "NOPE")
                .prop(WireMutexExclusive::new(dst))
                .prop(WireMutexExclusive::new(odst))
                .prop(BaseIntPip::new(odst, src))
                .test_routing(dst, src.pos())
                .prop(FuzzIntPip::new(dst, src))
                .commit();

            if do_far {
                ctx.build()
                    .global_mutex("HCLK", "TEST")
                    .row_mutex("BUFH_TEST", "NOPE")
                    .prop(WireMutexExclusive::new(dst))
                    .test_routing(dst, far_src)
                    .prop(FuzzIntPip::new(dst, src))
                    .commit();
            }
        }
        for i in 0..6 {
            let src = match side {
                DirH::W => wires::RCLK_CMT_W[i].cell(20),
                DirH::E => wires::RCLK_CMT_E[i].cell(20),
            };
            let far_src = backend.edev.db_index[tcid].pips_bwd[&src]
                .iter()
                .next()
                .copied()
                .unwrap();

            ctx.build()
                .global_mutex("RCLK", "USE")
                .row_mutex("BUFH_TEST", "NOPE")
                .prop(WireMutexExclusive::new(dst))
                .prop(WireMutexExclusive::new(odst))
                .prop(BaseIntPip::new(odst, src))
                .test_routing(dst, src.pos())
                .prop(FuzzIntPip::new(dst, src))
                .commit();

            if do_far {
                ctx.build()
                    .global_mutex("RCLK", format!("TEST{i}"))
                    .row_mutex("BUFH_TEST", "NOPE")
                    .prop(WireMutexExclusive::new(dst))
                    .prop(WireMutexExclusive::new(src))
                    .prop(WireMutexExclusive::new(far_src.tw))
                    .extra_tile_routing_special(
                        HclkIoiInnerSide(side),
                        wires::RCLK_ROW[i].cell(4),
                        specials::PRESENT,
                    )
                    .test_routing(dst, far_src)
                    .prop(FuzzIntPip::new(dst, src))
                    .commit();
            }
        }
    }
    for wires in [
        wires::IMUX_MMCM_CLKIN1_IO,
        wires::IMUX_MMCM_CLKIN2_IO,
        wires::IMUX_MMCM_CLKFB_IO,
        wires::IMUX_MMCM_CLKIN1_MGT,
        wires::IMUX_MMCM_CLKIN2_MGT,
    ] {
        for i in 0..2 {
            let dst = wires[i].cell(20);
            let odst = wires[i ^ 1].cell(20);
            let mux = &backend.edev.db_index[tcid].muxes[&dst];
            for &src in mux.src.keys() {
                let mut builder = ctx
                    .build()
                    .prop(WireMutexExclusive::new(dst))
                    .prop(WireMutexExclusive::new(odst))
                    .prop(WireMutexShared::new(src.tw));
                if wires::GIOB_CMT.contains(src.wire) {
                    builder = builder.global_mutex("GIO", "USE");
                } else if wires::CCIO_CMT_W.contains(src.wire)
                    || wires::CCIO_CMT_E.contains(src.wire)
                {
                    builder = builder.tile_mutex("CCIO", "USE");
                } else {
                    builder = builder.row_mutex("MGT", "USE");
                }
                builder
                    .prop(BaseIntPip::new(odst, src.tw))
                    .test_routing(dst, src)
                    .prop(FuzzIntPip::new(dst, src.tw))
                    .commit();
            }
        }
    }
    for wf in wires::MGT_CMT_W.into_iter().chain(wires::MGT_CMT_E) {
        let far_dst = wires::IMUX_MMCM_CLKIN1_MGT[0].cell(20);
        let dst = wf.cell(20);
        let src = backend.edev.db_index[tcid].pips_bwd[&dst]
            .iter()
            .next()
            .copied()
            .unwrap();
        ctx.build()
            .row_mutex("MGT", "TEST")
            .prop(WireMutexExclusive::new(far_dst))
            .prop(WireMutexExclusive::new(dst))
            .test_routing(far_dst, src)
            .prop(FuzzIntPip::new(far_dst, dst))
            .commit();
    }

    {
        for i in 0..32 {
            let dst = wires::IMUX_BUFG_O[i].cell(20);
            let odst = wires::IMUX_BUFG_O[i ^ 1].cell(20);
            let mux = &backend.edev.db_index[tcid].muxes[&dst];
            for &src in mux.src.keys() {
                let mut builder = ctx
                    .build()
                    .prop(WireMutexExclusive::new(dst))
                    .prop(WireMutexShared::new(src.tw));
                if wires::CCIO_CMT_W.contains(src.wire) || wires::CCIO_CMT_E.contains(src.wire) {
                    builder = builder
                        .tile_mutex("CCIO", "USE")
                        .prop(WireMutexExclusive::new(odst))
                        .prop(BaseIntPip::new(odst, src.tw))
                        .prop(FuzzIntPip::new(dst, src.tw));
                } else if wires::MGT_CMT_W.contains(src.wire) || wires::MGT_CMT_E.contains(src.wire)
                {
                    builder = builder
                        .tile_mutex("MGT", "USE")
                        .prop(WireMutexExclusive::new(odst))
                        .prop(BaseIntPip::new(odst, src.tw))
                        .prop(FuzzIntPip::new(dst, src.tw));
                } else if wires::RCLK_CMT_W.contains(src.wire)
                    || wires::RCLK_CMT_E.contains(src.wire)
                {
                    builder = builder
                        .global_mutex("RCLK", "USE")
                        .prop(WireMutexExclusive::new(odst))
                        .prop(BaseIntPip::new(odst, src.tw))
                        .prop(FuzzIntPip::new(dst, src.tw));
                } else {
                    builder = builder.prop(FuzzIntPip::new(dst, src.tw));
                }
                builder.test_routing(dst, src).commit();
            }
        }
    }
    for (wf, wm) in [
        (wires::OUT_MMCM_S, wires::OMUX_MMCM_PERF_S),
        (wires::OUT_MMCM_N, wires::OMUX_MMCM_PERF_N),
    ] {
        for i in 0..4 {
            let mid = wm[i].cell(20);
            for c in [44, 52] {
                for (wt, x) in [(wires::PERF_ROW, 0), (wires::PERF_ROW_OUTER, 1)] {
                    let dst = wt[i ^ x].cell(c);
                    for j in [0, 2, 4, 6] {
                        let src = wf[j].cell(20);
                        ctx.build()
                            .prop(WireMutexExclusive::new(dst))
                            .prop(WireMutexExclusive::new(mid))
                            .prop(WireMutexShared::new(src))
                            .test_routing(dst, src.pos())
                            .prop(FuzzIntPip::new(dst, mid))
                            .prop(FuzzIntPip::new(mid, src))
                            .commit();
                    }
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let tcid = tcls::CMT;
    let tile = "CMT";
    if devdata_only {
        for bel in ["PLL[0]", "PLL[1]"] {
            let mut diff = ctx.get_diff_legacy(tile, bel, "COMPENSATION", "ZHOLD");
            let dly_val = extract_bitvec_val_part_legacy(
                ctx.item_legacy(tile, bel, "IN_DLY_SET"),
                &bits![0; 5],
                &mut diff,
            );
            ctx.insert_device_data_legacy("MMCM:IN_DLY_SET", dly_val);
        }
        return;
    }
    for slots in [bslots::BUFHCE_W, bslots::BUFHCE_E] {
        for i in 0..12 {
            let bslot = slots[i];
            ctx.collect_bel_attr(tcid, bslot, BUFHCE::ENABLE);
            ctx.collect_bel_input_inv_bi(tcid, bslot, BUFHCE::CE);
            ctx.collect_bel_attr_bi(tcid, bslot, BUFHCE::INIT_OUT);
        }
    }
    for i in 0..2 {
        let bslot = bslots::PLL[i];
        let bel = &format!("PLL[{i}]");

        fn mmcm_drp_bit(which: usize, reg: usize, bit: usize) -> TileBit {
            let tile = if which == 0 {
                17 - (reg >> 3)
            } else {
                22 + (reg >> 3)
            };
            let frame = 26 + (bit & 1);
            let bit = (bit >> 1) | (reg & 7) << 3;
            let bit = if which == 0 { bit ^ 0x3f } else { bit };
            TileBit::new(tile, frame, bit)
        }
        let mut drp = vec![];
        for reg in 0..0x80 {
            for bit in 0..16 {
                drp.push(mmcm_drp_bit(i, reg, bit).pos());
            }
        }
        ctx.insert_bel_attr_bitvec(tcid, bslot, PLL::DRP, drp);

        for pin in [
            PLL::RST,
            PLL::PWRDWN,
            PLL::CLKINSEL,
            PLL::PSEN,
            PLL::PSINCDEC,
        ] {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }

        for attr in [
            "CASC_LOCK_EN",
            "CLKBURST_ENABLE",
            "CLKBURST_REPEAT",
            "CLKFBOUT_EN",
            "CLKOUT0_EN",
            "CLKOUT1_EN",
            "CLKOUT2_EN",
            "CLKOUT3_EN",
            "CLKOUT4_EN",
            "CLKOUT5_EN",
            "CLKOUT6_EN",
            "CLKFBOUT_USE_FINE_PS",
            "CLKOUT0_USE_FINE_PS",
            "CLKOUT1_USE_FINE_PS",
            "CLKOUT2_USE_FINE_PS",
            "CLKOUT3_USE_FINE_PS",
            "CLKOUT4_USE_FINE_PS",
            "CLKOUT5_USE_FINE_PS",
            "CLKOUT6_USE_FINE_PS",
            "CLKFBOUT_FRAC_EN",
            "CLKOUT0_FRAC_EN",
            "CLKOUT4_CASCADE",
            "CLOCK_HOLD",
            "DIRECT_PATH_CNTRL",
            "EN_VCO_DIV1",
            "EN_VCO_DIV6",
            "HVLF_STEP",
            "HVLF_CNT_TEST_EN",
            "IN_DLY_EN",
            "STARTUP_WAIT",
            "VLF_HIGH_DIS_B",
            "VLF_HIGH_PWDN_B",
        ] {
            ctx.collect_bit_bi_legacy(tile, bel, attr, "FALSE", "TRUE");
        }
        for attr in [
            "ANALOG_MISC",
            "AVDD_COMP_SET",
            "AVDD_VBG_PD",
            "AVDD_VBG_SEL",
            "CLKBURST_CNT",
            "CLKFBIN_HT",
            "CLKFBIN_LT",
            "CLKFBOUT_DT",
            "CLKFBOUT_HT",
            "CLKFBOUT_LT",
            "CLKFBOUT_MX",
            "CLKFBOUT_PM",
            "CLKFBOUT_FRAC",
            "CLKOUT0_DT",
            "CLKOUT0_HT",
            "CLKOUT0_LT",
            "CLKOUT0_MX",
            "CLKOUT0_PM",
            "CLKOUT0_FRAC",
            "CLKOUT1_DT",
            "CLKOUT1_HT",
            "CLKOUT1_LT",
            "CLKOUT1_MX",
            "CLKOUT1_PM",
            "CLKOUT2_DT",
            "CLKOUT2_HT",
            "CLKOUT2_LT",
            "CLKOUT2_MX",
            "CLKOUT2_PM",
            "CLKOUT3_DT",
            "CLKOUT3_HT",
            "CLKOUT3_LT",
            "CLKOUT3_MX",
            "CLKOUT3_PM",
            "CLKOUT4_DT",
            "CLKOUT4_HT",
            "CLKOUT4_LT",
            "CLKOUT4_MX",
            "CLKOUT4_PM",
            "CLKOUT5_DT",
            "CLKOUT5_HT",
            "CLKOUT5_LT",
            "CLKOUT5_MX",
            "CLKOUT5_PM",
            "CLKOUT6_DT",
            "CLKOUT6_HT",
            "CLKOUT6_LT",
            "CLKOUT6_MX",
            "CLKOUT6_PM",
            "CONTROL_0",
            "CONTROL_1",
            "CONTROL_2",
            "CONTROL_3",
            "CONTROL_4",
            "CONTROL_5",
            "CP",
            "CP_BIAS_TRIP_SET",
            "CP_RES",
            "DIVCLK_HT",
            "DIVCLK_LT",
            "DVDD_COMP_SET",
            "DVDD_VBG_PD",
            "DVDD_VBG_SEL",
            "FINE_PS_FRAC",
            "HROW_DLY_SET",
            "HVLF_CNT_TEST",
            "INTERP_EN",
            "IN_DLY_MX_CVDD",
            "IN_DLY_MX_DVDD",
            "LF_NEN",
            "LF_PEN",
            "LFHF",
            "MAN_LF",
            "LOCK_CNT",
            "LOCK_FB_DLY",
            "LOCK_REF_DLY",
            "LOCK_SAT_HIGH",
            "PFD",
            "RES",
            "TMUX_MUX_SEL",
            "UNLOCK_CNT",
        ] {
            ctx.collect_bitvec_legacy(tile, bel, attr, "");
        }

        for (addr, name) in [(0x16, "DIVCLK"), (0x17, "CLKFBIN")] {
            ctx.insert_legacy(
                tile,
                bel,
                format!("{name}_NOCOUNT"),
                TileItem::from_bit_inv(mmcm_drp_bit(i, addr, 12), false),
            );
            ctx.insert_legacy(
                tile,
                bel,
                format!("{name}_EDGE"),
                TileItem::from_bit_inv(mmcm_drp_bit(i, addr, 13), false),
            );
        }
        for (addr, name) in [
            (0x07, "CLKOUT5"),
            (0x09, "CLKOUT0"),
            (0x13, "CLKOUT6"),
            (0x15, "CLKFBOUT"),
        ] {
            ctx.insert_legacy(
                tile,
                bel,
                format!("{name}_FRAC_WF"),
                TileItem::from_bit_inv(mmcm_drp_bit(i, addr, 10), false),
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
            ctx.insert_legacy(
                tile,
                bel,
                format!("{name}_NOCOUNT"),
                TileItem::from_bit_inv(mmcm_drp_bit(i, addr, 6), false),
            );
            ctx.insert_legacy(
                tile,
                bel,
                format!("{name}_EDGE"),
                TileItem::from_bit_inv(mmcm_drp_bit(i, addr, 7), false),
            );
        }

        ctx.insert_legacy(
            tile,
            bel,
            "SYNTH_CLK_DIV",
            TileItem::from_bitvec_inv(
                vec![mmcm_drp_bit(i, 0x02, 0), mmcm_drp_bit(i, 0x02, 1)],
                false,
            ),
        );
        ctx.insert_legacy(
            tile,
            bel,
            "IN_DLY_SET",
            TileItem::from_bitvec_inv(
                vec![
                    mmcm_drp_bit(i, 0x05, 10),
                    mmcm_drp_bit(i, 0x05, 11),
                    mmcm_drp_bit(i, 0x05, 12),
                    mmcm_drp_bit(i, 0x05, 13),
                    mmcm_drp_bit(i, 0x05, 14),
                ],
                false,
            ),
        );

        ctx.get_diff_legacy(tile, bel, "GTS_WAIT", "FALSE")
            .assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "GTS_WAIT", "TRUE");
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "SYNTH_CLK_DIV"), 1, 0);
        ctx.insert_legacy(tile, bel, "GTS_WAIT", xlat_bit_legacy(diff));

        ctx.insert_legacy(
            tile,
            bel,
            "MMCM_EN",
            TileItem::from_bit_inv(mmcm_drp_bit(i, 0x74, 0), false),
        );

        let mut enable = ctx.get_diff_legacy(tile, bel, "ENABLE", "1");
        enable.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "MMCM_EN"), true, false);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "RES"), 0xf, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CP"), 0x5, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "INTERP_EN"), 0x10, 0);
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
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKOUT6_HT"), 1, 0);
        enable.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "CLKOUT6_LT"), 0x3f, 0);
        assert_eq!(enable.bits.len(), 1);
        let drp_mask = enable.filter_rects(&EntityVec::from_iter([BitRectId::from_idx(40)]));
        assert_eq!(drp_mask.bits.len(), 1);
        ctx.insert_bel_attr_bool(
            tcls::HCLK,
            bslots::HCLK_DRP,
            [bcls::HCLK_DRP_V6::DRP_MASK_S, bcls::HCLK_DRP_V6::DRP_MASK_N][i],
            xlat_bit(drp_mask),
        );

        let mut diff = ctx.get_diff_legacy(tile, bel, "COMPENSATION", "BUF_IN");
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IN_DLY_MX_DVDD"), 0x31, 0);
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IN_DLY_MX_CVDD"), 0x12, 0);
        diff.assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "COMPENSATION", "CASCADE");
        diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "CASC_LOCK_EN"), true, false);
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IN_DLY_MX_DVDD"), 0x0a, 0);
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
        let dly_val = extract_bitvec_val_part_legacy(
            ctx.item_legacy(tile, bel, "IN_DLY_SET"),
            &bits![0; 5],
            &mut diff,
        );
        ctx.insert_device_data_legacy("MMCM:IN_DLY_SET", dly_val);
        diff.assert_empty();

        for mult in 1..=64 {
            for bandwidth in ["LOW", "HIGH"] {
                let mut diff =
                    ctx.get_diff_legacy(tile, bel, "TABLES", format!("{mult}.{bandwidth}"));
                for (attr, base) in [
                    ("CP", bits![1, 0, 1, 0]),
                    ("RES", bits![1, 1, 1, 1]),
                    ("LFHF", bits![0, 0]),
                ] {
                    let val = extract_bitvec_val_part_legacy(
                        ctx.item_legacy(tile, bel, attr),
                        &base,
                        &mut diff,
                    );
                    let mut ival = 0;
                    for (i, v) in val.into_iter().enumerate() {
                        if v {
                            ival |= 1 << i;
                        }
                    }
                    ctx.insert_misc_data_legacy(format!("MMCM:{attr}:{bandwidth}:{mult}"), ival);
                }
                for (attr, width) in [
                    ("LOCK_REF_DLY", 5),
                    ("LOCK_FB_DLY", 5),
                    ("LOCK_CNT", 10),
                    ("LOCK_SAT_HIGH", 10),
                    ("UNLOCK_CNT", 10),
                ] {
                    let val = extract_bitvec_val_part_legacy(
                        ctx.item_legacy(tile, bel, attr),
                        &BitVec::repeat(false, width),
                        &mut diff,
                    );
                    let mut ival = 0;
                    for (i, v) in val.into_iter().enumerate() {
                        if v {
                            ival |= 1 << i;
                        }
                    }
                    ctx.insert_misc_data_legacy(format!("MMCM:{attr}:{mult}"), ival);
                }
                diff.discard_bits_legacy(ctx.item_legacy(tile, bel, "CLKFBOUT_NOCOUNT"));
                diff.discard_bits_legacy(ctx.item_legacy(tile, bel, "CLKFBOUT_EDGE"));
                diff.discard_bits_legacy(ctx.item_legacy(tile, bel, "CLKFBOUT_LT"));
                diff.discard_bits_legacy(ctx.item_legacy(tile, bel, "CLKFBOUT_HT"));
                diff.assert_empty();
            }
        }
    }

    for wires in [wires::IMUX_BUFHCE_W, wires::IMUX_BUFHCE_E] {
        for i in 0..12 {
            ctx.collect_mux(tcid, wires[i].cell(20));
        }
    }
    {
        for (wt, wf) in [
            (wires::IMUX_BUFHCE_W[0], wires::BUFH_INT_W),
            (wires::IMUX_BUFHCE_E[0], wires::BUFH_INT_E),
        ] {
            for wf in wf {
                let dst = wt.cell(20);
                let src = wf.cell(20);
                let far_src = ctx.edev.db_index[tcid].pips_bwd[&src]
                    .iter()
                    .next()
                    .copied()
                    .unwrap();
                let mut diff = ctx.get_diff_routing(tcid, dst, far_src);
                diff.apply_enum_diff_raw(ctx.sb_mux(tcid, dst), &Some(src.pos()), &None);
                ctx.insert_progbuf(tcid, src, far_src, xlat_bit(diff));
            }
        }
    }
    {
        let dst = wires::IMUX_BUFHCE_W[0].cell(20);
        for wf in wires::CCIO_CMT_W.into_iter().chain(wires::CCIO_CMT_E) {
            let src = wf.cell(20);
            let far_src = ctx.edev.db_index[tcid].pips_bwd[&src]
                .iter()
                .next()
                .copied()
                .unwrap();
            let mut diff = ctx.get_diff_routing(tcid, dst, far_src);
            diff.apply_enum_diff_raw(ctx.sb_mux(tcid, dst), &Some(src.pos()), &None);
            ctx.insert_progbuf(tcid, src, far_src, xlat_bit(diff));
        }
    }

    ctx.collect_inv_bi(tcid, wires::BUFH_TEST_W.cell(20));
    ctx.collect_inv_bi(tcid, wires::BUFH_TEST_E.cell(20));
    ctx.collect_mux(tcid, wires::BUFH_TEST_W_IN.cell(20));
    ctx.collect_mux(tcid, wires::BUFH_TEST_E_IN.cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKIN1_HCLK_W[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKIN1_HCLK_W[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKIN2_HCLK_W[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKIN2_HCLK_W[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKFB_HCLK_W[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKFB_HCLK_W[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKIN1_HCLK_E[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKIN1_HCLK_E[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKIN2_HCLK_E[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKIN2_HCLK_E[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKFB_HCLK_E[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKFB_HCLK_E[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKIN1_IO[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKIN1_IO[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKIN2_IO[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKIN2_IO[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKFB_IO[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKFB_IO[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKIN1_MGT[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKIN1_MGT[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKIN2_MGT[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_MMCM_CLKIN2_MGT[1].cell(20));

    for i in 0..32 {
        let dst = wires::GCLK_TEST[i].cell(20);
        let buf = wires::GCLK_TEST_IN[i].cell(20);
        let src = wires::GCLK_CMT[i].cell(20);

        let diff_buf = ctx.get_diff_routing(tcid, dst, src.pos());
        let mut diff_inv = ctx
            .get_diff_routing(tcid, dst, src.neg())
            .combine(&!&diff_buf);
        ctx.insert_progbuf(tcid, buf, src.pos(), xlat_bit(diff_buf));
        // FUCKERY MURDER HORSESHIT ISE
        match i {
            6 | 14 => {
                assert_eq!(diff_inv.bits.len(), 2);
                let diff_n = diff_inv.split_bits_by(|bit| bit.frame.to_idx() == 31);
                ctx.insert_inv(tcid, dst, xlat_bit(diff_inv));
                ctx.insert_inv(tcid, wires::GCLK_TEST[i + 1].cell(20), xlat_bit(diff_n));
            }
            7 | 15 => {
                diff_inv.assert_empty();
            }
            _ => {
                ctx.insert_inv(tcid, dst, xlat_bit(diff_inv));
            }
        }
    }
    for (wt, wf) in [
        (wires::BUFH_TEST_W_IN, wires::HCLK_CMT_W),
        (wires::BUFH_TEST_E_IN, wires::HCLK_CMT_E),
    ] {
        let far_dst = wt.cell(20);
        for i in 0..12 {
            let dst = wf[i].cell(20);
            let src = ctx.edev.db_index[tcid].pips_bwd[&dst]
                .iter()
                .next()
                .copied()
                .unwrap();
            let mut diff = ctx.get_diff_routing(tcid, far_dst, src);
            diff.apply_enum_diff_raw(ctx.sb_mux(tcid, far_dst), &Some(dst.pos()), &None);
            ctx.insert_progbuf(tcid, dst, src, xlat_bit(diff));
        }
    }
    for (wt, wf) in [
        (wires::BUFH_TEST_W_IN, wires::RCLK_CMT_W),
        (wires::BUFH_TEST_E_IN, wires::RCLK_CMT_E),
    ] {
        let far_dst = wt.cell(20);
        for i in 0..6 {
            let dst = wf[i].cell(20);
            let src = ctx.edev.db_index[tcid].pips_bwd[&dst]
                .iter()
                .next()
                .copied()
                .unwrap();
            let mut diff = ctx.get_diff_routing(tcid, far_dst, src);
            diff.apply_enum_diff_raw(ctx.sb_mux(tcid, far_dst), &Some(dst.pos()), &None);
            ctx.insert_progbuf(tcid, dst, src, xlat_bit(diff));
        }
    }
    for wf in wires::MGT_CMT_W.into_iter().chain(wires::MGT_CMT_E) {
        let far_dst = wires::IMUX_MMCM_CLKIN1_MGT[0].cell(20);
        let dst = wf.cell(20);
        let src = ctx.edev.db_index[tcid].pips_bwd[&dst]
            .iter()
            .next()
            .copied()
            .unwrap();
        let mut diff = ctx.get_diff_routing(tcid, far_dst, src);
        diff.apply_enum_diff_raw(ctx.sb_mux(tcid, far_dst), &Some(dst.pos()), &None);
        ctx.insert_progbuf(tcid, dst, src, xlat_bit(diff));
    }
    for i in 0..32 {
        ctx.collect_mux(tcid, wires::IMUX_BUFG_O[i].cell(20));
    }
    {
        let tcid = tcls::HCLK_IO;
        let diffs: [_; 6] = core::array::from_fn(|i| {
            ctx.get_diff_routing_special(tcid, wires::RCLK_ROW[i].cell(4), specials::PRESENT)
        });
        let mut all = Diff::default();
        for diff in &diffs {
            for (&k, &v) in &diff.bits {
                all.bits.insert(k, v);
            }
        }
        for i in 0..6 {
            let diff = all.combine(&!&diffs[i]);
            ctx.insert_pass(
                tcid,
                wires::RCLK_ROW[i].cell(4),
                wires::PULLUP.cell(4),
                xlat_bit(diff),
            );
        }
    }

    for (wf, wm) in [
        (wires::OUT_MMCM_S, wires::OMUX_MMCM_PERF_S),
        (wires::OUT_MMCM_N, wires::OMUX_MMCM_PERF_N),
    ] {
        for i in 0..4 {
            let mid = wm[i].cell(20);
            for c in [44, 52] {
                let mut diffs = vec![];
                let dst_i = wires::PERF_ROW[i].cell(c);
                let dst_o = wires::PERF_ROW_OUTER[i ^ 1].cell(c);
                for j in [0, 2, 4, 6] {
                    let src = wf[j].cell(20);
                    let diff_i = ctx.get_diff_routing(tcid, dst_i, src.pos());
                    let diff_o = ctx.get_diff_routing(tcid, dst_o, src.pos());
                    let (diff_i, diff_o, diff) = Diff::split(diff_i, diff_o);
                    diffs.push((Some(src.pos()), diff));
                    ctx.insert_progbuf(tcid, dst_i, mid.pos(), xlat_bit(diff_i));
                    ctx.insert_progbuf(tcid, dst_o, mid.pos(), xlat_bit(diff_o));
                }
                diffs.push((None, Diff::default()));
                ctx.insert_mux(tcid, mid, xlat_enum_raw(diffs, OcdMode::ValueOrder));
            }
        }
    }
}
