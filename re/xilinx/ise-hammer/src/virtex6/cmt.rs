use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::{db::WireSlotIdExt, dir::DirH, grid::TileCoord};
use prjcombine_re_collector::diff::{
    Diff, DiffKey, OcdMode, extract_bitvec_val_part, xlat_bit, xlat_enum_raw,
};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{
    bits,
    bitvec::BitVec,
    bsdata::{BitRectId, TileBit},
};
use prjcombine_virtex4::defs::{
    self,
    bcls::{self, BUFHCE, PLL_V6 as PLL},
    bslots, devdata,
    virtex6::{tables::PLL_MULT, tcls, wires},
};

use crate::{
    backend::{IseBackend, MultiValue},
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
                .test_bel_special(specials::PLL_COMPENSATION_ZHOLD)
                .attr("COMPENSATION", "ZHOLD")
                .commit();
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
            .test_bel_attr_bits(PLL::ENABLE)
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
            PLL::CASC_LOCK_EN,
            PLL::CLKBURST_ENABLE,
            PLL::CLKBURST_REPEAT,
            PLL::CLKFBOUT_EN,
            PLL::CLKOUT0_EN,
            PLL::CLKOUT1_EN,
            PLL::CLKOUT2_EN,
            PLL::CLKOUT3_EN,
            PLL::CLKOUT4_EN,
            PLL::CLKOUT5_EN,
            PLL::CLKOUT6_EN,
            PLL::DIRECT_PATH_CNTRL,
            PLL::CLOCK_HOLD,
            PLL::EN_VCO_DIV1,
            PLL::EN_VCO_DIV6,
            PLL::HVLF_STEP,
            PLL::HVLF_CNT_TEST_EN,
            PLL::IN_DLY_EN,
            PLL::STARTUP_WAIT,
            PLL::VLF_HIGH_DIS_B,
            PLL::VLF_HIGH_PWDN_B,
        ] {
            bctx.mode(mode)
                .global_xy("MMCMADV_*_USE_CALC", "NO")
                .mutex("MODE", "TEST")
                .test_bel_attr_bool_auto(attr, "FALSE", "TRUE");
        }
        bctx.mode(mode)
            .global_xy("MMCMADV_*_USE_CALC", "NO")
            .mutex("MODE", "TEST")
            .attr("CLKOUT6_EN", "TRUE")
            .attr("CLKOUT4_USE_FINE_PS", "")
            .attr("CLKOUT4_MX", "")
            .test_bel_attr_bool_auto(PLL::CLKOUT4_CASCADE, "FALSE", "TRUE");
        bctx.mode(mode)
            .global_xy("MMCMADV_*_USE_CALC", "NO")
            .mutex("MODE", "TEST")
            .attr("STARTUP_WAIT", "FALSE")
            .test_bel_attr_bool_auto(PLL::GTS_WAIT, "FALSE", "TRUE");
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
                .test_bel_attr_bool_auto(attr, "FALSE", "TRUE");
        }
        for attr in [PLL::CLKOUT0_FRAC_EN, PLL::CLKFBOUT_FRAC_EN] {
            bctx.mode(mode)
                .global_xy("MMCMADV_*_USE_CALC", "NO")
                .mutex("MODE", "TEST")
                .attr("CLKOUT5_EN", "TRUE")
                .attr("CLKOUT6_EN", "TRUE")
                .attr("INTERP_EN", "00000000")
                .test_bel_attr_bool_auto(attr, "FALSE", "TRUE");
        }

        for attr in [
            PLL::ANALOG_MISC,
            PLL::AVDD_VBG_PD,
            PLL::AVDD_VBG_SEL,
            PLL::CLKFBIN_HT,
            PLL::CLKFBIN_LT,
            PLL::CLKFBOUT_DT,
            PLL::CLKFBOUT_HT,
            PLL::CLKFBOUT_LT,
            PLL::CLKFBOUT_MX,
            PLL::CLKFBOUT_FRAC,
            PLL::CLKOUT0_DT,
            PLL::CLKOUT0_HT,
            PLL::CLKOUT0_LT,
            PLL::CLKOUT0_MX,
            PLL::CLKOUT0_FRAC,
            PLL::CLKOUT1_DT,
            PLL::CLKOUT1_HT,
            PLL::CLKOUT1_LT,
            PLL::CLKOUT1_MX,
            PLL::CLKOUT2_DT,
            PLL::CLKOUT2_HT,
            PLL::CLKOUT2_LT,
            PLL::CLKOUT2_MX,
            PLL::CLKOUT3_DT,
            PLL::CLKOUT3_HT,
            PLL::CLKOUT3_LT,
            PLL::CLKOUT3_MX,
            PLL::CLKOUT4_DT,
            PLL::CLKOUT4_HT,
            PLL::CLKOUT4_LT,
            PLL::CLKOUT4_MX,
            PLL::CLKOUT5_DT,
            PLL::CLKOUT5_HT,
            PLL::CLKOUT5_LT,
            PLL::CLKOUT5_MX,
            PLL::CLKOUT6_DT,
            PLL::CLKOUT6_HT,
            PLL::CLKOUT6_LT,
            PLL::CLKOUT6_MX,
            PLL::CONTROL_0,
            PLL::CONTROL_1,
            PLL::CONTROL_2,
            PLL::CONTROL_3,
            PLL::CONTROL_4,
            PLL::CONTROL_5,
            PLL::CP_BIAS_TRIP_SET,
            PLL::CP_RES,
            PLL::DIVCLK_HT,
            PLL::DIVCLK_LT,
            PLL::DVDD_VBG_PD,
            PLL::DVDD_VBG_SEL,
            PLL::INTERP_EN,
            PLL::IN_DLY_MX_CVDD,
            PLL::IN_DLY_MX_DVDD,
            PLL::LF_NEN,
            PLL::LF_PEN,
            PLL::MAN_LF,
            PLL::PFD,
            PLL::TMUX_MUX_SEL,
        ] {
            bctx.mode(mode)
                .global_xy("MMCMADV_*_USE_CALC", "NO")
                .mutex("MODE", "TEST")
                .attr("CLKOUT0_DIVIDE_F", "1.5")
                .attr("CLKFBOUT_MULT_F", "1.5")
                .test_bel_attr_multi(attr, MultiValue::Bin);
        }
        for (attr, aname, width) in [
            (PLL::V6_AVDD_COMP_SET, "AVDD_COMP_SET", 2),
            (PLL::V6_DVDD_COMP_SET, "DVDD_COMP_SET", 2),
        ] {
            bctx.mode(mode)
                .global_xy("MMCMADV_*_USE_CALC", "NO")
                .mutex("MODE", "TEST")
                .attr("CLKOUT0_DIVIDE_F", "1.5")
                .attr("CLKFBOUT_MULT_F", "1.5")
                .test_bel_attr_bits(attr)
                .multi_attr(aname, MultiValue::Bin, width);
        }
        for attr in [
            PLL::CLKFBOUT_PM,
            PLL::CLKOUT0_PM,
            PLL::CLKOUT1_PM,
            PLL::CLKOUT2_PM,
            PLL::CLKOUT3_PM,
            PLL::CLKOUT4_PM,
            PLL::CLKOUT5_PM,
            PLL::CLKOUT6_PM,
            PLL::FINE_PS_FRAC,
        ] {
            bctx.mode(mode)
                .global_xy("MMCMADV_*_USE_CALC", "NO")
                .mutex("MODE", "TEST")
                .attr("INTERP_EN", "00000000")
                .test_bel_attr_multi(attr, MultiValue::Bin);
        }
        for attr in [
            PLL::CLKBURST_CNT,
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
                .global_xy("MMCMADV_*_USE_CALC", "NO")
                .mutex("MODE", "TEST")
                .test_bel_attr_multi(attr, MultiValue::Dec(0));
        }

        for mult in 1..=64 {
            for (spec, bandwidth) in [
                (specials::PLL_TABLES_LOW, "LOW"),
                (specials::PLL_TABLES_HIGH, "HIGH"),
            ] {
                bctx.mode(mode)
                    .mutex("MODE", "CALC")
                    .global_xy("MMCMADV_*_USE_CALC", "NO")
                    .test_bel_special_u32(spec, mult)
                    .attr("CLKFBOUT_MULT_F", mult.to_string())
                    .attr("BANDWIDTH", bandwidth)
                    .commit();
            }
        }
        for (spec, val) in [
            (specials::PLL_COMPENSATION_ZHOLD, "ZHOLD"),
            (specials::PLL_COMPENSATION_EXTERNAL, "EXTERNAL"),
            (specials::PLL_COMPENSATION_INTERNAL, "INTERNAL"),
            (specials::PLL_COMPENSATION_BUF_IN, "BUF_IN"),
            (specials::PLL_COMPENSATION_CASCADE, "CASCADE"),
        ] {
            bctx.mode(mode)
                .mutex("MODE", "COMP")
                .global_xy("MMCMADV_*_USE_CALC", "NO")
                .attr("HROW_DLY_SET", "000")
                .test_bel_special(spec)
                .attr("COMPENSATION", val)
                .commit();
        }
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
            wires::IMUX_PLL_CLKIN1_HCLK_W[0],
            true,
        ),
        (
            DirH::W,
            wires::IMUX_PLL_CLKIN1_HCLK_W[0],
            wires::IMUX_PLL_CLKIN1_HCLK_W[1],
            false,
        ),
        (
            DirH::W,
            wires::IMUX_PLL_CLKIN1_HCLK_W[1],
            wires::IMUX_PLL_CLKIN1_HCLK_W[0],
            false,
        ),
        (
            DirH::W,
            wires::IMUX_PLL_CLKIN2_HCLK_W[0],
            wires::IMUX_PLL_CLKIN2_HCLK_W[1],
            false,
        ),
        (
            DirH::W,
            wires::IMUX_PLL_CLKIN2_HCLK_W[1],
            wires::IMUX_PLL_CLKIN2_HCLK_W[0],
            false,
        ),
        (
            DirH::W,
            wires::IMUX_PLL_CLKFB_HCLK_W[0],
            wires::IMUX_PLL_CLKFB_HCLK_W[1],
            false,
        ),
        (
            DirH::W,
            wires::IMUX_PLL_CLKFB_HCLK_W[1],
            wires::IMUX_PLL_CLKFB_HCLK_W[0],
            false,
        ),
        (
            DirH::E,
            wires::BUFH_TEST_E_IN,
            wires::IMUX_PLL_CLKIN1_HCLK_E[0],
            true,
        ),
        (
            DirH::E,
            wires::IMUX_PLL_CLKIN1_HCLK_E[0],
            wires::IMUX_PLL_CLKIN1_HCLK_E[1],
            false,
        ),
        (
            DirH::E,
            wires::IMUX_PLL_CLKIN1_HCLK_E[1],
            wires::IMUX_PLL_CLKIN1_HCLK_E[0],
            false,
        ),
        (
            DirH::E,
            wires::IMUX_PLL_CLKIN2_HCLK_E[0],
            wires::IMUX_PLL_CLKIN2_HCLK_E[1],
            false,
        ),
        (
            DirH::E,
            wires::IMUX_PLL_CLKIN2_HCLK_E[1],
            wires::IMUX_PLL_CLKIN2_HCLK_E[0],
            false,
        ),
        (
            DirH::E,
            wires::IMUX_PLL_CLKFB_HCLK_E[0],
            wires::IMUX_PLL_CLKFB_HCLK_E[1],
            false,
        ),
        (
            DirH::E,
            wires::IMUX_PLL_CLKFB_HCLK_E[1],
            wires::IMUX_PLL_CLKFB_HCLK_E[0],
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
        wires::IMUX_PLL_CLKIN1_IO,
        wires::IMUX_PLL_CLKIN2_IO,
        wires::IMUX_PLL_CLKFB_IO,
        wires::IMUX_PLL_CLKIN1_MGT,
        wires::IMUX_PLL_CLKIN2_MGT,
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
        let far_dst = wires::IMUX_PLL_CLKIN1_MGT[0].cell(20);
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
        (wires::OUT_PLL_S, wires::OMUX_PLL_PERF_S),
        (wires::OUT_PLL_N, wires::OMUX_PLL_PERF_N),
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
    if devdata_only {
        for idx in 0..2 {
            let bslot = bslots::PLL[idx];
            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::PLL_COMPENSATION_ZHOLD);
            let dly_val = extract_bitvec_val_part(
                ctx.bel_attr_bitvec(tcid, bslot, PLL::V6_IN_DLY_SET),
                &bits![0; 5],
                &mut diff,
            );
            ctx.insert_devdata_bitvec(devdata::PLL_V6_IN_DLY_SET, dly_val);
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
        ctx.insert_bel_attr_bitvec(tcid, bslot, PLL::MMCM_DRP, drp);

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
            PLL::CASC_LOCK_EN,
            PLL::CLKBURST_ENABLE,
            PLL::CLKBURST_REPEAT,
            PLL::CLKFBOUT_EN,
            PLL::CLKOUT0_EN,
            PLL::CLKOUT1_EN,
            PLL::CLKOUT2_EN,
            PLL::CLKOUT3_EN,
            PLL::CLKOUT4_EN,
            PLL::CLKOUT5_EN,
            PLL::CLKOUT6_EN,
            PLL::CLKFBOUT_USE_FINE_PS,
            PLL::CLKOUT0_USE_FINE_PS,
            PLL::CLKOUT1_USE_FINE_PS,
            PLL::CLKOUT2_USE_FINE_PS,
            PLL::CLKOUT3_USE_FINE_PS,
            PLL::CLKOUT4_USE_FINE_PS,
            PLL::CLKOUT5_USE_FINE_PS,
            PLL::CLKOUT6_USE_FINE_PS,
            PLL::CLKFBOUT_FRAC_EN,
            PLL::CLKOUT0_FRAC_EN,
            PLL::CLKOUT4_CASCADE,
            PLL::CLOCK_HOLD,
            PLL::DIRECT_PATH_CNTRL,
            PLL::EN_VCO_DIV1,
            PLL::EN_VCO_DIV6,
            PLL::HVLF_STEP,
            PLL::HVLF_CNT_TEST_EN,
            PLL::IN_DLY_EN,
            PLL::STARTUP_WAIT,
            PLL::VLF_HIGH_DIS_B,
            PLL::VLF_HIGH_PWDN_B,
        ] {
            ctx.collect_bel_attr_bi(tcid, bslot, attr);
        }
        for attr in [
            PLL::ANALOG_MISC,
            PLL::V6_AVDD_COMP_SET,
            PLL::AVDD_VBG_PD,
            PLL::AVDD_VBG_SEL,
            PLL::CLKBURST_CNT,
            PLL::CLKFBIN_HT,
            PLL::CLKFBIN_LT,
            PLL::CLKFBOUT_DT,
            PLL::CLKFBOUT_HT,
            PLL::CLKFBOUT_LT,
            PLL::CLKFBOUT_MX,
            PLL::CLKFBOUT_PM,
            PLL::CLKFBOUT_FRAC,
            PLL::CLKOUT0_DT,
            PLL::CLKOUT0_HT,
            PLL::CLKOUT0_LT,
            PLL::CLKOUT0_MX,
            PLL::CLKOUT0_PM,
            PLL::CLKOUT0_FRAC,
            PLL::CLKOUT1_DT,
            PLL::CLKOUT1_HT,
            PLL::CLKOUT1_LT,
            PLL::CLKOUT1_MX,
            PLL::CLKOUT1_PM,
            PLL::CLKOUT2_DT,
            PLL::CLKOUT2_HT,
            PLL::CLKOUT2_LT,
            PLL::CLKOUT2_MX,
            PLL::CLKOUT2_PM,
            PLL::CLKOUT3_DT,
            PLL::CLKOUT3_HT,
            PLL::CLKOUT3_LT,
            PLL::CLKOUT3_MX,
            PLL::CLKOUT3_PM,
            PLL::CLKOUT4_DT,
            PLL::CLKOUT4_HT,
            PLL::CLKOUT4_LT,
            PLL::CLKOUT4_MX,
            PLL::CLKOUT4_PM,
            PLL::CLKOUT5_DT,
            PLL::CLKOUT5_HT,
            PLL::CLKOUT5_LT,
            PLL::CLKOUT5_MX,
            PLL::CLKOUT5_PM,
            PLL::CLKOUT6_DT,
            PLL::CLKOUT6_HT,
            PLL::CLKOUT6_LT,
            PLL::CLKOUT6_MX,
            PLL::CLKOUT6_PM,
            PLL::CONTROL_0,
            PLL::CONTROL_1,
            PLL::CONTROL_2,
            PLL::CONTROL_3,
            PLL::CONTROL_4,
            PLL::CONTROL_5,
            PLL::CP,
            PLL::CP_BIAS_TRIP_SET,
            PLL::CP_RES,
            PLL::DIVCLK_HT,
            PLL::DIVCLK_LT,
            PLL::V6_DVDD_COMP_SET,
            PLL::DVDD_VBG_PD,
            PLL::DVDD_VBG_SEL,
            PLL::FINE_PS_FRAC,
            PLL::HROW_DLY_SET,
            PLL::HVLF_CNT_TEST,
            PLL::INTERP_EN,
            PLL::IN_DLY_MX_CVDD,
            PLL::IN_DLY_MX_DVDD,
            PLL::LF_NEN,
            PLL::LF_PEN,
            PLL::LFHF,
            PLL::MAN_LF,
            PLL::LOCK_CNT,
            PLL::LOCK_FB_DLY,
            PLL::LOCK_REF_DLY,
            PLL::LOCK_SAT_HIGH,
            PLL::PFD,
            PLL::RES,
            PLL::TMUX_MUX_SEL,
            PLL::UNLOCK_CNT,
        ] {
            ctx.collect_bel_attr(tcid, bslot, attr);
        }

        for (addr, nocount, edge) in [
            (0x16, PLL::DIVCLK_NOCOUNT, PLL::DIVCLK_EDGE),
            (0x17, PLL::CLKFBIN_NOCOUNT, PLL::CLKFBIN_EDGE),
        ] {
            ctx.insert_bel_attr_bool(tcid, bslot, nocount, mmcm_drp_bit(i, addr, 12).pos());
            ctx.insert_bel_attr_bool(tcid, bslot, edge, mmcm_drp_bit(i, addr, 13).pos());
        }
        for (addr, attr) in [
            (0x07, PLL::CLKOUT5_FRAC_WF),
            (0x09, PLL::CLKOUT0_FRAC_WF),
            (0x13, PLL::CLKOUT6_FRAC_WF),
            (0x15, PLL::CLKFBOUT_FRAC_WF),
        ] {
            ctx.insert_bel_attr_bool(tcid, bslot, attr, mmcm_drp_bit(i, addr, 10).pos());
        }
        for (addr, nocount, edge) in [
            (0x07, PLL::CLKOUT5_NOCOUNT, PLL::CLKOUT5_EDGE),
            (0x09, PLL::CLKOUT0_NOCOUNT, PLL::CLKOUT0_EDGE),
            (0x0b, PLL::CLKOUT1_NOCOUNT, PLL::CLKOUT1_EDGE),
            (0x0d, PLL::CLKOUT2_NOCOUNT, PLL::CLKOUT2_EDGE),
            (0x0f, PLL::CLKOUT3_NOCOUNT, PLL::CLKOUT3_EDGE),
            (0x11, PLL::CLKOUT4_NOCOUNT, PLL::CLKOUT4_EDGE),
            (0x13, PLL::CLKOUT6_NOCOUNT, PLL::CLKOUT6_EDGE),
            (0x15, PLL::CLKFBOUT_NOCOUNT, PLL::CLKFBOUT_EDGE),
        ] {
            ctx.insert_bel_attr_bool(tcid, bslot, nocount, mmcm_drp_bit(i, addr, 6).pos());
            ctx.insert_bel_attr_bool(tcid, bslot, edge, mmcm_drp_bit(i, addr, 7).pos());
        }

        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            PLL::SYNTH_CLK_DIV,
            vec![
                mmcm_drp_bit(i, 0x02, 0).pos(),
                mmcm_drp_bit(i, 0x02, 1).pos(),
            ],
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            PLL::V6_IN_DLY_SET,
            vec![
                mmcm_drp_bit(i, 0x05, 10).pos(),
                mmcm_drp_bit(i, 0x05, 11).pos(),
                mmcm_drp_bit(i, 0x05, 12).pos(),
                mmcm_drp_bit(i, 0x05, 13).pos(),
                mmcm_drp_bit(i, 0x05, 14).pos(),
            ],
        );

        ctx.get_diff_attr_bool_bi(tcid, bslot, PLL::GTS_WAIT, false)
            .assert_empty();
        let mut diff = ctx.get_diff_attr_bool_bi(tcid, bslot, PLL::GTS_WAIT, true);
        diff.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::SYNTH_CLK_DIV), 1, 0);
        ctx.insert_bel_attr_bool(tcid, bslot, PLL::GTS_WAIT, xlat_bit(diff));

        ctx.insert_bel_attr_bool(tcid, bslot, PLL::ENABLE, mmcm_drp_bit(i, 0x74, 0).pos());

        let mut enable = ctx.get_diff_attr_bool(tcid, bslot, PLL::ENABLE);
        enable.apply_bit_diff(ctx.bel_attr_bit(tcid, bslot, PLL::ENABLE), true, false);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::RES), 0xf, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CP), 0x5, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::INTERP_EN), 0x10, 0);
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
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKOUT6_HT), 1, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKOUT6_LT), 0x3f, 0);
        assert_eq!(enable.bits.len(), 1);
        let drp_mask = enable.filter_rects(&EntityVec::from_iter([BitRectId::from_idx(40)]));
        assert_eq!(drp_mask.bits.len(), 1);
        ctx.insert_bel_attr_bool(
            tcls::HCLK,
            bslots::HCLK_DRP[0],
            [bcls::HCLK_DRP::DRP_MASK_S, bcls::HCLK_DRP::DRP_MASK_N][i],
            xlat_bit(drp_mask),
        );

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
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::PLL_COMPENSATION_CASCADE);
        diff.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, PLL::CASC_LOCK_EN),
            true,
            false,
        );
        diff.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::IN_DLY_MX_DVDD),
            0x0a,
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
        let dly_val = extract_bitvec_val_part(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::V6_IN_DLY_SET),
            &bits![0; 5],
            &mut diff,
        );
        ctx.insert_devdata_bitvec(devdata::PLL_V6_IN_DLY_SET, dly_val);
        diff.assert_empty();

        for mult in 1..=64 {
            let row = ctx.edev.db[PLL_MULT]
                .rows
                .get(&format!("MMCM_{mult}"))
                .unwrap()
                .0;
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
            ] {
                let mut diff = ctx.get_diff_bel_special_u32(tcid, bslot, spec, mult);
                for (attr, field, base) in [
                    (PLL::CP, field_cp, bits![1, 0, 1, 0]),
                    (PLL::RES, field_res, bits![1, 1, 1, 1]),
                    (PLL::LFHF, field_lfhf, bits![0, 0]),
                ] {
                    let val = extract_bitvec_val_part(
                        ctx.bel_attr_bitvec(tcid, bslot, attr),
                        &base,
                        &mut diff,
                    );
                    ctx.insert_table_bitvec(PLL_MULT, row, field, val);
                }
                for (attr, field, width) in [
                    (PLL::LOCK_REF_DLY, PLL_MULT::LOCK_REF_DLY, 5),
                    (PLL::LOCK_FB_DLY, PLL_MULT::LOCK_FB_DLY, 5),
                    (PLL::LOCK_CNT, PLL_MULT::LOCK_CNT, 10),
                    (PLL::LOCK_SAT_HIGH, PLL_MULT::LOCK_SAT_HIGH, 10),
                    (PLL::UNLOCK_CNT, PLL_MULT::UNLOCK_CNT, 10),
                ] {
                    let val = extract_bitvec_val_part(
                        ctx.bel_attr_bitvec(tcid, bslot, attr),
                        &BitVec::repeat(false, width),
                        &mut diff,
                    );
                    ctx.insert_table_bitvec(PLL_MULT, row, field, val);
                }
                diff.discard_polbits(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKFBOUT_NOCOUNT));
                diff.discard_polbits(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKFBOUT_EDGE));
                diff.discard_polbits(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKFBOUT_LT));
                diff.discard_polbits(ctx.bel_attr_bitvec(tcid, bslot, PLL::CLKFBOUT_HT));
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
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN1_HCLK_W[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN1_HCLK_W[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN2_HCLK_W[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN2_HCLK_W[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKFB_HCLK_W[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKFB_HCLK_W[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN1_HCLK_E[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN1_HCLK_E[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN2_HCLK_E[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN2_HCLK_E[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKFB_HCLK_E[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKFB_HCLK_E[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN1_IO[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN1_IO[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN2_IO[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN2_IO[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKFB_IO[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKFB_IO[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN1_MGT[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN1_MGT[1].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN2_MGT[0].cell(20));
    ctx.collect_mux(tcid, wires::IMUX_PLL_CLKIN2_MGT[1].cell(20));

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
        let far_dst = wires::IMUX_PLL_CLKIN1_MGT[0].cell(20);
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
        (wires::OUT_PLL_S, wires::OMUX_PLL_PERF_S),
        (wires::OUT_PLL_N, wires::OMUX_PLL_PERF_N),
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
