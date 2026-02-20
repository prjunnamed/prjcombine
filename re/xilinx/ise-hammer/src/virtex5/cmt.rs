use prjcombine_entity::EntityPartVec;
use prjcombine_interconnect::{
    db::{BelAttributeEnum, BelInfo, SwitchBoxItem, WireSlotIdExt},
    grid::TileCoord,
};
use prjcombine_re_collector::diff::{
    Diff, DiffKey, OcdMode, extract_bitvec_val_part, xlat_bit, xlat_bit_wide_bi, xlat_bitvec,
    xlat_bitvec_sparse_u32, xlat_enum_attr, xlat_enum_raw,
};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{bits, bitvec::BitVec, bsdata::TileBit};
use prjcombine_virtex4::defs::{
    bcls::{self, DCM_V5 as DCM, PLL_V5 as PLL},
    bslots, devdata, enums, tslots,
    virtex5::{tables::PLL_MULT, tcls, wires},
};

use crate::{
    backend::{IseBackend, MultiValue, PinFromKind},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::{BaseIntPip, FuzzIntPip},
        props::{
            extra::{ExtraKeyFixed, ExtraTile},
            mutex::{WireMutexExclusive, WireMutexShared},
            relation::TileRelation,
        },
    },
    virtex4::specials,
};

#[derive(Copy, Clone, Debug)]
struct HclkCmt;

impl TileRelation for HclkCmt {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let chip = edev.chips[tcrd.die];
        let row = chip.row_hclk(tcrd.row);
        Some(tcrd.with_row(row).tile(tslots::HCLK_BEL))
    }
}

pub fn add_fuzzers<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
    skip_dcm: bool,
    skip_pll: bool,
    devdata_only: bool,
) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::CMT);

    if devdata_only {
        let mut bctx = ctx.bel(bslots::PLL);
        bctx.build()
            .global_xy("PLLADV_*_USE_CALC", "NO")
            .related_tile_mutex_exclusive(HclkCmt, "ENABLE")
            .prop(ExtraTile::new(
                HclkCmt,
                ExtraKeyFixed::new(DiffKey::BelAttrBit(
                    tcls::HCLK_CMT,
                    bslots::HCLK_CMT_DRP,
                    bcls::HCLK_CMT_DRP::DRP_MASK,
                    0,
                    true,
                )),
            ))
            .test_bel_special(specials::PRESENT)
            .mode("PLL_ADV")
            .commit();
        return;
    }

    if !skip_dcm {
        for i in 0..2 {
            let mut bctx = ctx.bel(bslots::DCM[i]);
            let mode = "DCM_ADV";
            bctx.build()
                .related_tile_mutex_exclusive(HclkCmt, "ENABLE")
                .prop(ExtraTile::new(
                    HclkCmt,
                    ExtraKeyFixed::new(DiffKey::BelAttrBit(
                        tcls::HCLK_CMT,
                        bslots::HCLK_CMT_DRP,
                        bcls::HCLK_CMT_DRP::DRP_MASK,
                        0,
                        true,
                    )),
                ))
                .test_bel_special(specials::PRESENT)
                .mode(mode)
                .commit();

            for pin in [
                DCM::PSEN,
                DCM::PSINCDEC,
                DCM::RST,
                DCM::SKEWCLKIN1,
                DCM::SKEWCLKIN2,
                DCM::SKEWIN,
                DCM::SKEWRST,
            ] {
                bctx.mode(mode)
                    .mutex("MODE", "INV")
                    .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                    .test_bel_input_inv_auto(pin);
            }
            for (attr, pin) in [
                (DCM::OUT_CLK0_ENABLE, "CLK0"),
                (DCM::OUT_CLK90_ENABLE, "CLK90"),
                (DCM::OUT_CLK180_ENABLE, "CLK180"),
                (DCM::OUT_CLK270_ENABLE, "CLK270"),
                (DCM::OUT_CLK2X_ENABLE, "CLK2X"),
                (DCM::OUT_CLK2X180_ENABLE, "CLK2X180"),
                (DCM::OUT_CLKDV_ENABLE, "CLKDV"),
                (DCM::OUT_CLKFX_ENABLE, "CLKFX"),
                (DCM::OUT_CLKFX180_ENABLE, "CLKFX180"),
                (DCM::OUT_CONCUR_ENABLE, "CONCUR"),
            ] {
                bctx.mode(mode)
                    .mutex("MODE", "PIN")
                    .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                    .mutex("PIN", pin)
                    .test_bel_attr_bits(attr)
                    .pin(pin)
                    .commit();
            }

            bctx.mode(mode)
                .mutex("MODE", "PIN")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .pin_from("CLKFB", PinFromKind::Bufg)
                .test_bel_special(specials::DCM_CLKFB_ENABLE)
                .pin("CLKFB")
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "PIN")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .pin_from("CLKIN", PinFromKind::Bufg)
                .test_bel_special(specials::DCM_CLKIN_ENABLE)
                .pin("CLKIN")
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "PIN")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .pin("CLKIN")
                .pin("CLKFB")
                .pin_from("CLKFB", PinFromKind::Bufg)
                .test_bel_special(specials::DCM_CLKIN_IOB)
                .pin_from("CLKIN", PinFromKind::Bufg, PinFromKind::Iob)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "PIN")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .pin("CLKIN")
                .pin("CLKFB")
                .pin_from("CLKIN", PinFromKind::Bufg)
                .test_bel_special(specials::DCM_CLKFB_IOB)
                .pin_from("CLKFB", PinFromKind::Bufg, PinFromKind::Iob)
                .commit();

            for (attr, pin) in [
                ("DCM_OPTINV_RST", DCM::RST),
                ("DCM_OPTINV_PSINCDEC", DCM::PSINCDEC),
                ("DCM_OPTINV_PSEN", DCM::PSEN),
                ("DCM_OPTINV_SKEW_IN", DCM::SKEWIN),
                ("DCM_OPTINV_SKEW_RST", DCM::SKEWRST),
                ("MUX_INV_PLL_CLK", DCM::SKEWCLKIN1),
                ("MUX_INV_TEST_CLK", DCM::SKEWCLKIN2),
            ] {
                for (val, vname) in [(false, "FALSE"), (true, "TRUE")] {
                    bctx.mode(mode)
                        .mutex("MODE", "ATTR")
                        .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                        .global("GTS_CYCLE", "1")
                        .global("DONE_CYCLE", "1")
                        .global("LCK_CYCLE", "NOWAIT")
                        .test_bel_input_inv(pin, val)
                        .attr(attr, vname)
                        .commit();
                }
            }

            for attr in [
                DCM::DCM_CLKDV_CLKFX_ALIGNMENT,
                DCM::DCM_COM_PWC_FB_EN,
                DCM::DCM_COM_PWC_REF_EN,
                DCM::DCM_EXT_FB_EN,
                DCM::DCM_LOCK_HIGH_B,
                DCM::DCM_PLL_RST_DCM,
                DCM::DCM_POWERDOWN_COMMON_EN_B,
                DCM::DCM_REG_PWRD_CFG,
                DCM::DCM_SCANMODE,
                DCM::DCM_UNUSED_TAPS_POWERDOWN,
                DCM::DCM_USE_REG_READY,
                DCM::DCM_VREG_ENABLE,
                DCM::DCM_WAIT_PLL,
                DCM::DFS_CFG_BYPASS,
                DCM::DFS_EARLY_LOCK,
                DCM::DFS_EN,
                DCM::DFS_EN_RELRST_B,
                DCM::DFS_FAST_UPDATE,
                DCM::DFS_MPW_LOW,
                DCM::DFS_MPW_HIGH,
                DCM::DFS_OSC_ON_FX,
                DCM::DFS_OUTPUT_PSDLY_ON_CONCUR,
                DCM::DFS_PWRD_CLKIN_STOP_B,
                DCM::DFS_PWRD_CLKIN_STOP_STICKY_B,
                DCM::DFS_PWRD_REPLY_TIMES_OUT_B,
                DCM::DFS_REF_ON_FX,
                DCM::DFS_SYNC_TO_DLL,
                DCM::DLL_PERIOD_LOCK_BY1,
                DCM::DLL_PWRD_STICKY_B,
                DCM::DLL_PWRD_ON_SCANMODE_B,
                DCM::DLL_CLKFB_STOPPED_PWRD_EN_B,
                DCM::DLL_CLKIN_STOPPED_PWRD_EN_B,
                DCM::DLL_ZD1_PWC_EN,
                DCM::DLL_PHASE_SHIFT_LOCK_BY1,
                DCM::DLL_ETPP_HOLD,
                DCM::DLL_ZD2_PWC_EN,
                DCM::DLL_ZD2_EN,
                DCM::DLL_FDBKLOST_EN,
                DCM::DLL_DESKEW_LOCK_BY1,
                DCM::DLL_ZD1_EN,
                DCM::DLL_ZD2_JF_OVERFLOW_HOLD,
                DCM::DLL_ZD1_JF_OVERFLOW_HOLD,
                DCM::CLKIN_DIVIDE_BY_2,
                DCM::STARTUP_WAIT,
            ] {
                bctx.mode(mode)
                    .mutex("MODE", "ATTR")
                    .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                    .global("GTS_CYCLE", "1")
                    .global("DONE_CYCLE", "1")
                    .global("LCK_CYCLE", "NOWAIT")
                    .test_bel_attr_bool_auto(attr, "FALSE", "TRUE");
            }
            for (spec, attr) in [
                (specials::DCM_DUTY_CYCLE_CORRECTION, "DUTY_CYCLE_CORRECTION"),
                (specials::DCM_INPUTMUX_EN, "DCM_INPUTMUX_EN"),
            ] {
                for val in ["FALSE", "TRUE"] {
                    bctx.mode(mode)
                        .null_bits()
                        .mutex("MODE", "ATTR")
                        .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                        .global("GTS_CYCLE", "1")
                        .global("DONE_CYCLE", "1")
                        .global("LCK_CYCLE", "NOWAIT")
                        .test_bel_special(spec)
                        .attr(attr, val)
                        .commit();
                }
            }

            for attr in [
                DCM::DCM_CLKFB_IODLY_MUXINSEL,
                DCM::DCM_CLKFB_IODLY_MUXOUT_SEL,
                DCM::DCM_CLKIN_IODLY_MUXOUT_SEL,
            ] {
                bctx.mode(mode)
                    .mutex("MODE", "ATTR")
                    .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                    .test_bel_attr_auto(attr);
            }
            bctx.mode(mode)
                .mutex("MODE", "PIN")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .pin("CLKIN")
                .pin("CLKFB")
                .pin_from("CLKIN", PinFromKind::Iob)
                .pin_from("CLKFB", PinFromKind::Bufg)
                .test_bel_attr_auto(DCM::DCM_CLKIN_IODLY_MUXINSEL);

            bctx.mode(mode)
                .mutex("MODE", "ATTR")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .test_bel_attr_bool_auto(DCM::DCM_CLKLOST_EN, "DISABLE", "ENABLE");

            for val in 1..8 {
                bctx.mode(mode)
                    .mutex("MODE", "ATTR")
                    .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                    .test_bel_attr_bitvec_u32(DCM::DFS_AVE_FREQ_SAMPLE_INTERVAL, val)
                    .attr("DFS_AVE_FREQ_SAMPLE_INTERVAL", val.to_string())
                    .commit();
            }

            for (val, vname) in [
                (enums::DCM_DFS_AVE_FREQ_GAIN::_0P125, "0.125"),
                (enums::DCM_DFS_AVE_FREQ_GAIN::_0P25, "0.25"),
                (enums::DCM_DFS_AVE_FREQ_GAIN::_0P5, "0.5"),
                (enums::DCM_DFS_AVE_FREQ_GAIN::_1P0, "1.0"),
                (enums::DCM_DFS_AVE_FREQ_GAIN::_2P0, "2.0"),
                (enums::DCM_DFS_AVE_FREQ_GAIN::_4P0, "4.0"),
                (enums::DCM_DFS_AVE_FREQ_GAIN::_8P0, "8.0"),
            ] {
                bctx.mode(mode)
                    .mutex("MODE", "ATTR")
                    .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                    .test_bel_attr_val(DCM::DFS_AVE_FREQ_GAIN, val)
                    .attr("DFS_AVE_FREQ_GAIN", vname)
                    .commit();
            }
            bctx.mode(mode)
                .mutex("MODE", "ATTR")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .test_bel_attr_auto(DCM::DFS_FREQUENCY_MODE);
            bctx.mode(mode)
                .mutex("MODE", "ATTR")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .test_bel_attr_subset_rename(
                    "DLL_FREQUENCY_MODE",
                    DCM::DLL_FREQUENCY_MODE,
                    &[
                        enums::DCM_DLL_FREQUENCY_MODE::LOW,
                        enums::DCM_DLL_FREQUENCY_MODE::HIGH,
                    ],
                );
            bctx.mode(mode)
                .mutex("MODE", "ATTR")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .attr("DLL_FREQUENCY_MODE", "")
                .test_bel_attr_auto(DCM::DLL_PHASE_SHIFT_CALIBRATION);
            bctx.mode(mode)
                .mutex("MODE", "ATTR")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .test_bel_attr_auto(DCM::DLL_SYNTH_CLOCK_SPEED);
            bctx.mode(mode)
                .mutex("MODE", "ATTR")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .attr("DFS_EARLY_LOCK", "")
                .attr("DFS_HARDSYNC_B", "")
                .test_bel_attr_subset_rename(
                    "DFS_OSCILLATOR_MODE",
                    DCM::DFS_OSCILLATOR_MODE,
                    &[
                        enums::DCM_DFS_OSCILLATOR_MODE::PHASE_FREQ_LOCK,
                        enums::DCM_DFS_OSCILLATOR_MODE::AVE_FREQ_LOCK,
                    ],
                );
            for (spec, val) in [
                (specials::DCM_VREF_SOURCE_VDD, "VDD"),
                (specials::DCM_VREF_SOURCE_VBG_DLL, "VBG_DLL"),
                (specials::DCM_VREF_SOURCE_VBG, "VBG"),
            ] {
                bctx.mode(mode)
                    .mutex("MODE", "ATTR")
                    .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                    .attr("DCM_VBG_PD", "")
                    .attr("DCM_VBG_SEL", "")
                    .attr("DCM_PERFORMANCE_MODE", "")
                    .test_bel_special(spec)
                    .attr("DCM_VREF_SOURCE", val)
                    .commit();
            }
            for (spec, val) in [
                (specials::DCM_MAX_SPEED, "MAX_SPEED"),
                (specials::DCM_MAX_RANGE, "MAX_RANGE"),
            ] {
                bctx.mode(mode)
                    .mutex("MODE", "ATTR")
                    .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                    .attr("DCM_VBG_PD", "")
                    .attr("DCM_VBG_SEL", "")
                    .attr("DCM_VREF_SOURCE", "VBG_DLL")
                    .test_bel_special(spec)
                    .attr("DCM_PERFORMANCE_MODE", val)
                    .commit();
            }

            for attr in [
                DCM::DCM_COMMON_MSB_SEL,
                DCM::DCM_COM_PWC_FB_TAP,
                DCM::DCM_COM_PWC_REF_TAP,
                DCM::DCM_TRIM_CAL,
                DCM::DCM_VBG_PD,
                DCM::DCM_VBG_SEL,
                DCM::DCM_VSPLY_VALID_ACC,
                DCM::DFS_CUSTOM_FAST_SYNC,
                DCM::DFS_HARDSYNC_B,
                DCM::DFS_JF_LOWER_LIMIT,
                DCM::DFS_HF_TRIM_CAL,
                DCM::DFS_SYNTH_CLOCK_SPEED,
                DCM::DFS_SYNTH_FAST_SYNCH,
                DCM::DFS_TAPTRIM,
                DCM::DFS_TWEAK,
                DCM::DLL_TAPINIT_CTL,
                DCM::DLL_TEST_MUX_SEL,
                DCM::DLL_ZD1_PHASE_SEL_INIT,
                DCM::DLL_ZD1_PWC_TAP,
                DCM::DLL_ZD1_TAP_INIT,
                DCM::DLL_ZD2_PWC_TAP,
                DCM::DLL_ZD2_TAP_INIT,
            ] {
                bctx.mode(mode)
                    .mutex("MODE", "ATTR")
                    .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                    .test_bel_attr_multi(attr, MultiValue::Bin);
            }
            for (spec, attr, width) in [
                (specials::DCM_DLL_CLK_EN, "DLL_CLK_EN", 7),
                (specials::DCM_DFS_CLK_EN, "DFS_CLK_EN", 3),
            ] {
                bctx.mode(mode)
                    .mutex("MODE", "ATTR")
                    .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                    .test_bel_special_bits(spec)
                    .multi_attr(attr, MultiValue::Bin, width);
            }
            for attr in [
                DCM::DESKEW_ADJUST,
                DCM::DLL_DESKEW_MAXTAP,
                DCM::DLL_DESKEW_MINTAP,
                DCM::DLL_DEAD_TIME,
                DCM::DLL_LIVE_TIME,
                DCM::DLL_SETTLE_TIME,
                DCM::DLL_PHASE_SHIFT_LFC,
            ] {
                bctx.mode(mode)
                    .mutex("MODE", "ATTR")
                    .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                    .test_bel_attr_multi(attr, MultiValue::Dec(0));
            }
            bctx.mode(mode)
                .mutex("MODE", "ATTR")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .attr("DLL_FREQUENCY_MODE", "")
                .test_bel_attr_multi(DCM::FACTORY_JF, MultiValue::Hex(0));
            bctx.mode(mode)
                .mutex("MODE", "ATTR")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .test_bel_attr_bits(DCM::CLKFX_DIVIDE)
                .multi_attr("CLKFX_DIVIDE", MultiValue::Dec(1), 5);
            for val in 2..=32 {
                bctx.mode(mode)
                    .mutex("MODE", "ATTR")
                    .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                    .test_bel_attr_u32(DCM::CLKFX_MULTIPLY, val - 1)
                    .attr("CLKFX_MULTIPLY", val.to_string())
                    .commit();
            }

            for val in 2..=16 {
                bctx.mode(mode)
                    .mutex("MODE", "PIN")
                    .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                    .test_bel_special_u32(specials::DCM_CLKDV_DIVIDE_INT, val)
                    .attr("CLKDV_DIVIDE", format!("{val}.0"))
                    .commit();
            }

            for (spec, dll_mode) in [
                (specials::DCM_CLKDV_DIVIDE_HALF_LOW, "LOW"),
                (specials::DCM_CLKDV_DIVIDE_HALF_HIGH, "HIGH"),
            ] {
                for val in 1..=7 {
                    bctx.mode(mode)
                        .mutex("MODE", "PIN")
                        .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                        .attr("DLL_FREQUENCY_MODE", dll_mode)
                        .test_bel_special_u32(spec, val)
                        .attr("CLKDV_DIVIDE", format!("{val}.5"))
                        .commit();
                }
            }

            for (spec, val) in [
                (specials::DCM_CLKOUT_PHASE_SHIFT_NONE, "NONE"),
                (specials::DCM_CLKOUT_PHASE_SHIFT_FIXED, "FIXED"),
                (
                    specials::DCM_CLKOUT_PHASE_SHIFT_VARIABLE_POSITIVE,
                    "VARIABLE_POSITIVE",
                ),
                (
                    specials::DCM_CLKOUT_PHASE_SHIFT_VARIABLE_CENTER,
                    "VARIABLE_CENTER",
                ),
                (specials::DCM_CLKOUT_PHASE_SHIFT_DIRECT, "DIRECT"),
            ] {
                bctx.mode(mode)
                    .mutex("MODE", "ATTR")
                    .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                    .attr("PHASE_SHIFT", "1")
                    .attr("DLL_ZD2_EN", "")
                    .no_pin("CLK0")
                    .no_pin("CLK90")
                    .no_pin("CLK180")
                    .no_pin("CLK270")
                    .no_pin("CLK2X")
                    .no_pin("CLK2X180")
                    .no_pin("CLKDV")
                    .test_bel_special(spec)
                    .attr("CLKOUT_PHASE_SHIFT", val)
                    .commit();
                bctx.mode(mode)
                    .mutex("MODE", "ATTR")
                    .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                    .attr("PHASE_SHIFT", "-1")
                    .attr("DLL_ZD2_EN", "")
                    .no_pin("CLK0")
                    .no_pin("CLK90")
                    .no_pin("CLK180")
                    .no_pin("CLK270")
                    .no_pin("CLK2X")
                    .no_pin("CLK2X180")
                    .no_pin("CLKDV")
                    .test_bel_special_special(spec, specials::DCM_NEG)
                    .attr("CLKOUT_PHASE_SHIFT", val)
                    .commit();
            }
            bctx.mode(mode)
                .mutex("MODE", "ATTR")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .test_bel_attr_multi(DCM::PHASE_SHIFT, MultiValue::Dec(0));
            bctx.mode(mode)
                .mutex("MODE", "ATTR")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .attr("CLKOUT_PHASE_SHIFT", "NONE")
                .test_bel_special(specials::DCM_PHASE_SHIFT_N1)
                .attr("PHASE_SHIFT", "-1")
                .commit();

            for (dst, odst) in [
                (
                    wires::IMUX_DCM_CLKIN[i].cell(0),
                    wires::IMUX_DCM_CLKFB[i].cell(0),
                ),
                (
                    wires::IMUX_DCM_CLKFB[i].cell(0),
                    wires::IMUX_DCM_CLKIN[i].cell(0),
                ),
            ] {
                let mux = &backend.edev.db_index.tile_classes[tcls::CMT].muxes[&dst];
                for &src in mux.src.keys() {
                    let mut builder = bctx
                        .mode(mode)
                        .global_mutex("HCLK_CMT", "USE")
                        .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                        .attr("CLK_FEEDBACK", "1X")
                        .prop(WireMutexExclusive::new(dst))
                        .prop(WireMutexShared::new(src.tw))
                        .prop(WireMutexExclusive::new(odst));
                    if wires::HCLK_CMT.contains(src.wire) || wires::GIOB_CMT.contains(src.wire) {
                        builder = builder.prop(BaseIntPip::new(odst, src.tw));
                    }
                    if matches!(
                        src.wire,
                        wires::OMUX_PLL_SKEWCLKIN1 | wires::OMUX_PLL_SKEWCLKIN2
                    ) {
                        builder = builder.bel_unused(bslots::PLL);
                    }
                    builder
                        .test_routing(dst, src)
                        .prop(FuzzIntPip::new(dst, src.tw))
                        .commit();
                }
            }
        }
    }
    if !skip_pll {
        let mut bctx = ctx.bel(bslots::PLL);
        let mode = "PLL_ADV";
        bctx.build()
            .global_xy("PLLADV_*_USE_CALC", "NO")
            .related_tile_mutex_exclusive(HclkCmt, "ENABLE")
            .prop(ExtraTile::new(
                HclkCmt,
                ExtraKeyFixed::new(DiffKey::BelAttrBit(
                    tcls::HCLK_CMT,
                    bslots::HCLK_CMT_DRP,
                    bcls::HCLK_CMT_DRP::DRP_MASK,
                    0,
                    true,
                )),
            ))
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();

        for pin in [
            PLL::CLKBRST,
            PLL::CLKINSEL,
            PLL::ENOUTSYNC,
            PLL::MANPDLF,
            PLL::MANPULF,
            PLL::REL,
            PLL::RST,
            PLL::SKEWCLKIN1,
            PLL::SKEWCLKIN2,
            PLL::SKEWRST,
            PLL::SKEWSTB,
        ] {
            bctx.mode(mode)
                .mutex("MODE", "INV")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .global_xy("PLLADV_*_USE_CALC", "NO")
                .test_bel_input_inv_auto(pin);
        }

        for attr in [
            PLL::PLL_CLKBURST_ENABLE,
            PLL::PLL_CP_BIAS_TRIP_SHIFT,
            PLL::PLL_DIRECT_PATH_CNTRL,
            PLL::PLL_EN_DLY,
            PLL::PLL_INC_FLOCK,
            PLL::PLL_INC_SLOCK,
            PLL::PLL_LOCK_CNT_RST_FAST,
            PLL::PLL_MAN_LF_EN,
            PLL::PLL_NBTI_EN,
            PLL::PLL_PMCD_MODE,
            PLL::PLL_PWRD_CFG,
            PLL::PLL_SEL_SLIPD,
            PLL::PLL_UNLOCK_CNT_RST_FAST,
            PLL::PLL_VLFHIGH_DIS,
            PLL::PLL_EN_TCLK0,
            PLL::PLL_EN_TCLK1,
            PLL::PLL_EN_TCLK2,
            PLL::PLL_EN_TCLK3,
            PLL::PLL_EN_TCLK4,
            PLL::PLL_EN_VCO0,
            PLL::PLL_EN_VCO1,
            PLL::PLL_EN_VCO2,
            PLL::PLL_EN_VCO3,
            PLL::PLL_EN_VCO4,
            PLL::PLL_EN_VCO5,
            PLL::PLL_EN_VCO6,
            PLL::PLL_EN_VCO7,
            PLL::PLL_EN_VCO_DIV1,
            PLL::PLL_EN_VCO_DIV6,
            PLL::PLL_CLKOUT0_EN,
            PLL::PLL_CLKOUT1_EN,
            PLL::PLL_CLKOUT2_EN,
            PLL::PLL_CLKOUT3_EN,
            PLL::PLL_CLKOUT4_EN,
            PLL::PLL_CLKOUT5_EN,
            PLL::PLL_CLKFBOUT_EN,
            PLL::PLL_CLKOUT0_EDGE,
            PLL::PLL_CLKOUT1_EDGE,
            PLL::PLL_CLKOUT2_EDGE,
            PLL::PLL_CLKOUT3_EDGE,
            PLL::PLL_CLKOUT4_EDGE,
            PLL::PLL_CLKOUT5_EDGE,
            PLL::PLL_CLKFBOUT_EDGE,
            PLL::PLL_CLKFBOUT2_EDGE,
            PLL::PLL_DIVCLK_EDGE,
            PLL::PLL_CLKOUT0_NOCOUNT,
            PLL::PLL_CLKOUT1_NOCOUNT,
            PLL::PLL_CLKOUT2_NOCOUNT,
            PLL::PLL_CLKOUT3_NOCOUNT,
            PLL::PLL_CLKOUT4_NOCOUNT,
            PLL::PLL_CLKOUT5_NOCOUNT,
            PLL::PLL_CLKFBOUT_NOCOUNT,
            PLL::PLL_CLKFBOUT2_NOCOUNT,
            PLL::PLL_DIVCLK_NOCOUNT,
        ] {
            bctx.mode(mode)
                .mutex("MODE", "TEST")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .global_xy("PLLADV_*_USE_CALC", "NO")
                .test_bel_attr_bool_auto(attr, "FALSE", "TRUE");
        }
        for attr in [
            PLL::PLL_UNLOCK_CNT,
            PLL::PLL_AVDD_COMP_SET,
            PLL::PLL_DVDD_COMP_SET,
            PLL::PLL_INTFB,
            PLL::PLL_RES,
            PLL::PLL_CP,
            PLL::PLL_CP_RES,
            PLL::PLL_LFHF,
            PLL::PLL_AVDD_VBG_PD,
            PLL::PLL_DVDD_VBG_PD,
            PLL::PLL_AVDD_VBG_SEL,
            PLL::PLL_DVDD_VBG_SEL,
            PLL::PLL_LF_NEN,
            PLL::PLL_LF_PEN,
            PLL::PLL_PFD_CNTRL,
            PLL::PLL_CLKCNTRL,
            PLL::PLL_TCK4_SEL,
            PLL::PLL_PFD_DLY,
            PLL::PLL_CLKBURST_CNT,
            PLL::PLL_LOCK_CNT,
            PLL::PLL_CLK0MX,
            PLL::PLL_CLK1MX,
            PLL::PLL_CLK2MX,
            PLL::PLL_CLK3MX,
            PLL::PLL_CLK4MX,
            PLL::PLL_CLK5MX,
            PLL::PLL_CLKFBMX,
            PLL::CLKOUT0_DESKEW_ADJUST,
            PLL::CLKOUT1_DESKEW_ADJUST,
            PLL::CLKOUT2_DESKEW_ADJUST,
            PLL::CLKOUT3_DESKEW_ADJUST,
            PLL::CLKOUT4_DESKEW_ADJUST,
            PLL::CLKOUT5_DESKEW_ADJUST,
            PLL::CLKFBOUT_DESKEW_ADJUST,
        ] {
            bctx.mode(mode)
                .mutex("MODE", "TEST")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .global_xy("PLLADV_*_USE_CALC", "NO")
                .test_bel_attr_multi(attr, MultiValue::Dec(0));
        }
        for attr in [
            PLL::PLL_EN_CNTRL,
            PLL::PLL_FLOCK,
            PLL::PLL_IN_DLY_MX_SEL,
            PLL::PLL_IN_DLY_SET,
            PLL::PLL_LOCK_FB_P1,
            PLL::PLL_LOCK_FB_P2,
            PLL::PLL_LOCK_REF_P1,
            PLL::PLL_LOCK_REF_P2,
            PLL::PLL_MISC,
            PLL::PLL_CLKOUT0_DT,
            PLL::PLL_CLKOUT0_HT,
            PLL::PLL_CLKOUT0_LT,
            PLL::PLL_CLKOUT1_DT,
            PLL::PLL_CLKOUT1_HT,
            PLL::PLL_CLKOUT1_LT,
            PLL::PLL_CLKOUT2_DT,
            PLL::PLL_CLKOUT2_HT,
            PLL::PLL_CLKOUT2_LT,
            PLL::PLL_CLKOUT3_DT,
            PLL::PLL_CLKOUT3_HT,
            PLL::PLL_CLKOUT3_LT,
            PLL::PLL_CLKOUT4_DT,
            PLL::PLL_CLKOUT4_HT,
            PLL::PLL_CLKOUT4_LT,
            PLL::PLL_CLKOUT5_DT,
            PLL::PLL_CLKOUT5_HT,
            PLL::PLL_CLKOUT5_LT,
            PLL::PLL_CLKFBOUT_DT,
            PLL::PLL_CLKFBOUT_HT,
            PLL::PLL_CLKFBOUT_LT,
            PLL::PLL_CLKFBOUT2_DT,
            PLL::PLL_CLKFBOUT2_HT,
            PLL::PLL_CLKFBOUT2_LT,
            PLL::PLL_DIVCLK_DT,
            PLL::PLL_DIVCLK_HT,
            PLL::PLL_DIVCLK_LT,
            PLL::PLL_CLKOUT0_PM,
            PLL::PLL_CLKOUT1_PM,
            PLL::PLL_CLKOUT2_PM,
            PLL::PLL_CLKOUT3_PM,
            PLL::PLL_CLKOUT4_PM,
            PLL::PLL_CLKOUT5_PM,
            PLL::PLL_CLKFBOUT_PM,
        ] {
            bctx.mode(mode)
                .mutex("MODE", "TEST")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .global_xy("PLLADV_*_USE_CALC", "NO")
                .test_bel_attr_multi(attr, MultiValue::Bin);
        }

        for (spec, val) in [
            (
                specials::PLL_COMPENSATION_SOURCE_SYNCHRONOUS,
                "SOURCE_SYNCHRONOUS",
            ),
            (
                specials::PLL_COMPENSATION_SYSTEM_SYNCHRONOUS,
                "SYSTEM_SYNCHRONOUS",
            ),
            (specials::PLL_COMPENSATION_PLL2DCM, "PLL2DCM"),
            (specials::PLL_COMPENSATION_DCM2PLL, "DCM2PLL"),
            (specials::PLL_COMPENSATION_EXTERNAL, "EXTERNAL"),
            (specials::PLL_COMPENSATION_INTERNAL, "INTERNAL"),
        ] {
            bctx.mode(mode)
                .mutex("MODE", "COMP")
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .global_xy("PLLADV_*_USE_CALC", "YES")
                .test_bel_special(spec)
                .attr("COMPENSATION", val)
                .commit();
        }

        for mult in 1..=64 {
            for (spec, bandwidth) in [
                (specials::PLL_TABLES_LOW, "LOW"),
                (specials::PLL_TABLES_HIGH, "HIGH"),
            ] {
                bctx.mode(mode)
                    .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                    .mutex("MODE", "CALC")
                    .global_xy("PLLADV_*_USE_CALC", "NO")
                    .test_bel_special_u32(spec, mult)
                    .attr_diff("CLKFBOUT_MULT", "0", mult.to_string())
                    .attr_diff("BANDWIDTH", "LOW", bandwidth)
                    .commit();
            }
        }

        let dst = wires::IMUX_PLL_CLKFB.cell(0);
        let odst = wires::IMUX_PLL_CLKIN1.cell(0);
        let mux = &backend.edev.db_index.tile_classes[tcls::CMT].muxes[&dst];
        for &src in mux.src.keys() {
            let mut builder = bctx
                .mode(mode)
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .mutex("MODE", "CALC")
                .prop(WireMutexExclusive::new(dst))
                .prop(WireMutexExclusive::new(odst))
                .prop(WireMutexShared::new(src.tw));
            if wires::HCLK_CMT.contains(src.wire) || wires::GIOB_CMT.contains(src.wire) {
                builder = builder
                    .global_mutex("HCLK_CMT", "USE")
                    .prop(BaseIntPip::new(odst, src.tw));
            } else {
                builder = builder.prop(BaseIntPip::new(odst, wires::OUT_PLL_CLKFBDCM.cell(0)));
            }
            let mut builder = builder
                .test_routing(dst, src)
                .prop(FuzzIntPip::new(dst, src.tw));
            if !wires::HCLK_CMT.contains(src.wire)
                && !wires::GIOB_CMT.contains(src.wire)
                && src.wire != wires::OUT_PLL_CLKFBDCM
            {
                builder = builder.pip("CLKFBIN", "CLKFB_ALT");
            }
            builder.commit();
        }

        let dst = wires::IMUX_PLL_CLKIN1.cell(0);
        let odst = wires::IMUX_PLL_CLKFB.cell(0);
        for src in wires::GIOB_CMT
            .into_iter()
            .chain(wires::HCLK_CMT)
            .chain([
                wires::OUT_PLL_CLKFBDCM,
                wires::OMUX_DCM_SKEWCLKIN1[0],
                wires::OMUX_DCM_SKEWCLKIN1[1],
            ])
            .map(|w| w.cell(0))
            .chain([wires::IMUX_CLK[0].cell(3)])
        {
            let mut builder = bctx
                .mode(mode)
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .mutex("MODE", "CALC")
                .prop(WireMutexExclusive::new(dst))
                .prop(WireMutexExclusive::new(odst))
                .prop(WireMutexShared::new(src));
            if wires::HCLK_CMT.contains(src.wire) || wires::GIOB_CMT.contains(src.wire) {
                builder = builder
                    .global_mutex("HCLK_CMT", "USE")
                    .prop(BaseIntPip::new(odst, src));
            } else {
                builder = builder.prop(BaseIntPip::new(odst, wires::OUT_PLL_CLKFBDCM.cell(0)));
            }
            let mut builder = builder
                .test_routing(dst, src.pos())
                .prop(FuzzIntPip::new(dst, src));
            if !wires::HCLK_CMT.contains(src.wire)
                && !wires::GIOB_CMT.contains(src.wire)
                && src.wire != wires::OUT_PLL_CLKFBDCM
            {
                builder = builder.pip("CLKIN1", "CLKIN_ALT");
            }
            builder.commit();
        }

        let dst = wires::IMUX_PLL_CLKIN2.cell(0);
        let src = wires::GIOB_CMT[5].cell(0);
        bctx.mode(mode)
            .global_mutex("HCLK_CMT", "USE")
            .related_tile_mutex(HclkCmt, "ENABLE", "USE")
            .mutex("MODE", "CALC")
            .prop(WireMutexExclusive::new(dst))
            .prop(WireMutexExclusive::new(odst))
            .prop(WireMutexShared::new(src))
            .prop(BaseIntPip::new(odst, src))
            .test_bel_attr_bits(PLL::CLKINSEL_MODE_DYNAMIC)
            .prop(FuzzIntPip::new(dst, src))
            .commit();

        for w in [wires::OMUX_PLL_SKEWCLKIN1, wires::OMUX_PLL_SKEWCLKIN2] {
            let dst = w.cell(0);
            let mux = &backend.edev.db_index.tile_classes[tcls::CMT].muxes[&dst];
            for &src in mux.src.keys() {
                bctx.mode(mode)
                    .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                    .prop(WireMutexExclusive::new(dst))
                    .test_routing(dst, src)
                    .prop(FuzzIntPip::new(dst, src.tw))
                    .commit();
            }
        }
    }
    {
        let mut bctx = ctx.bel(bslots::SPEC_INT);
        let dst = wires::OUT_CMT[10].cell(0);
        let mux = &backend.edev.db_index.tile_classes[tcls::CMT].muxes[&dst];
        for &src in mux.src.keys() {
            bctx.build()
                .related_tile_mutex(HclkCmt, "ENABLE", "USE")
                .bel_mode(bslots::PLL, "PLL_ADV")
                .prop(WireMutexExclusive::new(dst))
                .prop(WireMutexShared::new(src.tw))
                .test_routing(dst, src)
                .prop(FuzzIntPip::new(dst, src.tw))
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, skip_dcm: bool, skip_pll: bool, devdata_only: bool) {
    let tcid = tcls::CMT;
    if devdata_only {
        let bslot = bslots::PLL;
        let mut enable = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
        let dly_val = extract_bitvec_val_part(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_IN_DLY_SET),
            &bits![0; 9],
            &mut enable,
        );
        ctx.insert_devdata_bitvec(devdata::PLL_IN_DLY_SET, dly_val);

        {
            let tcid = tcls::HCLK_CMT;
            let bslot = bslots::HCLK_CMT_DRP;
            let bit = xlat_bit(ctx.get_diff_attr_bool(tcid, bslot, bcls::HCLK_CMT_DRP::DRP_MASK));
            for tcid in [tcls::HCLK_CMT, tcls::HCLK_IO_CMT_N, tcls::HCLK_IO_CMT_S] {
                ctx.insert_bel_attr_bool(tcid, bslot, bcls::HCLK_CMT_DRP::DRP_MASK, bit);
            }
        }

        return;
    }
    ctx.collect_mux_ocd(tcid, wires::OUT_CMT[10].cell(0), OcdMode::BitOrderDrpV5);

    if !skip_dcm {
        for i in 0..2 {
            let bslot = bslots::DCM[i];
            fn dcm_drp_bit(which: usize, reg: usize, bit: usize) -> TileBit {
                let reg = reg & 0x3f;
                let tile = which * 7 + (reg >> 3);
                let frame = match bit & 3 {
                    0 | 3 => 29,
                    1 | 2 => 28,
                    _ => unreachable!(),
                };
                let bit = (bit >> 1) | (reg & 7) << 3;
                TileBit::new(tile, frame, bit)
            }
            let mut drp = vec![];
            for reg in 0x40..0x58 {
                for bit in 0..16 {
                    drp.push(dcm_drp_bit(i, reg, bit).pos());
                }
            }
            ctx.insert_bel_attr_bitvec(tcid, bslot, DCM::DRP, drp);

            for pin in [
                DCM::PSEN,
                DCM::PSINCDEC,
                DCM::RST,
                DCM::SKEWCLKIN1,
                DCM::SKEWCLKIN2,
                DCM::SKEWIN,
                DCM::SKEWRST,
            ] {
                ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
            }
            for attr in [
                DCM::DCM_CLKDV_CLKFX_ALIGNMENT,
                DCM::DCM_COM_PWC_FB_EN,
                DCM::DCM_COM_PWC_REF_EN,
                DCM::DCM_EXT_FB_EN,
                DCM::DCM_LOCK_HIGH_B,
                DCM::DCM_PLL_RST_DCM,
                DCM::DCM_POWERDOWN_COMMON_EN_B,
                DCM::DCM_REG_PWRD_CFG,
                DCM::DCM_SCANMODE,
                DCM::DCM_USE_REG_READY,
                DCM::DCM_VREG_ENABLE,
                DCM::DCM_WAIT_PLL,
                DCM::DFS_CFG_BYPASS,
                DCM::DFS_EARLY_LOCK,
                DCM::DFS_EN,
                DCM::DFS_EN_RELRST_B,
                DCM::DFS_FAST_UPDATE,
                DCM::DFS_MPW_LOW,
                DCM::DFS_MPW_HIGH,
                DCM::DFS_OSC_ON_FX,
                DCM::DFS_OUTPUT_PSDLY_ON_CONCUR,
                DCM::DFS_PWRD_CLKIN_STOP_B,
                DCM::DFS_PWRD_CLKIN_STOP_STICKY_B,
                DCM::DFS_PWRD_REPLY_TIMES_OUT_B,
                DCM::DFS_REF_ON_FX,
                DCM::DFS_SYNC_TO_DLL,
                DCM::DLL_CLKFB_STOPPED_PWRD_EN_B,
                DCM::DLL_CLKIN_STOPPED_PWRD_EN_B,
                DCM::DLL_DESKEW_LOCK_BY1,
                DCM::DLL_ETPP_HOLD,
                DCM::DLL_FDBKLOST_EN,
                DCM::DLL_PERIOD_LOCK_BY1,
                DCM::DLL_PHASE_SHIFT_LOCK_BY1,
                DCM::DLL_PWRD_STICKY_B,
                DCM::DLL_PWRD_ON_SCANMODE_B,
                DCM::DLL_ZD1_EN,
                DCM::DLL_ZD1_JF_OVERFLOW_HOLD,
                DCM::DLL_ZD1_PWC_EN,
                DCM::DLL_ZD2_EN,
                DCM::DLL_ZD2_JF_OVERFLOW_HOLD,
                DCM::DLL_ZD2_PWC_EN,
                DCM::CLKIN_DIVIDE_BY_2,
                DCM::STARTUP_WAIT,
                DCM::DCM_CLKLOST_EN,
            ] {
                ctx.collect_bel_attr_bi(tcid, bslot, attr);
            }

            let bits = xlat_bit_wide_bi(
                ctx.get_diff_attr_bool_bi(tcid, bslot, DCM::DCM_UNUSED_TAPS_POWERDOWN, false),
                ctx.get_diff_attr_bool_bi(tcid, bslot, DCM::DCM_UNUSED_TAPS_POWERDOWN, true),
            );
            ctx.insert_bel_attr_bitvec(tcid, bslot, DCM::DCM_UNUSED_TAPS_POWERDOWN, bits);
            ctx.collect_bel_attr(tcid, bslot, DCM::DFS_FREQUENCY_MODE);
            ctx.collect_bel_attr(tcid, bslot, DCM::DLL_SYNTH_CLOCK_SPEED);
            ctx.collect_bel_attr_sparse(tcid, bslot, DCM::DFS_AVE_FREQ_SAMPLE_INTERVAL, 1..8);
            ctx.collect_bel_attr_default(
                tcid,
                bslot,
                DCM::DFS_AVE_FREQ_GAIN,
                enums::DCM_DFS_AVE_FREQ_GAIN::NONE,
            );
            ctx.collect_bel_attr(tcid, bslot, DCM::DLL_PHASE_SHIFT_CALIBRATION);
            for attr in [
                DCM::DCM_COMMON_MSB_SEL,
                DCM::DCM_COM_PWC_FB_TAP,
                DCM::DCM_COM_PWC_REF_TAP,
                DCM::DCM_TRIM_CAL,
                DCM::DCM_VBG_PD,
                DCM::DCM_VBG_SEL,
                DCM::DCM_VSPLY_VALID_ACC,
                DCM::DFS_CUSTOM_FAST_SYNC,
                DCM::DFS_HARDSYNC_B,
                DCM::DFS_JF_LOWER_LIMIT,
                DCM::DFS_HF_TRIM_CAL,
                DCM::DFS_SYNTH_CLOCK_SPEED,
                DCM::DFS_SYNTH_FAST_SYNCH,
                DCM::DFS_TAPTRIM,
                DCM::DFS_TWEAK,
                DCM::DLL_TAPINIT_CTL,
                DCM::DLL_TEST_MUX_SEL,
                DCM::DLL_ZD1_PHASE_SEL_INIT,
                DCM::DLL_ZD1_PWC_TAP,
                DCM::DLL_ZD1_TAP_INIT,
                DCM::DLL_ZD2_PWC_TAP,
                DCM::DLL_ZD2_TAP_INIT,
                DCM::DLL_DESKEW_MAXTAP,
                DCM::DLL_DESKEW_MINTAP,
                DCM::DLL_DEAD_TIME,
                DCM::DLL_LIVE_TIME,
                DCM::DLL_SETTLE_TIME,
                DCM::DLL_PHASE_SHIFT_LFC,
                DCM::FACTORY_JF,
                DCM::DESKEW_ADJUST,
                DCM::PHASE_SHIFT,
            ] {
                ctx.collect_bel_attr(tcid, bslot, attr);
            }
            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_PHASE_SHIFT_N1);
            diff.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, DCM::PHASE_SHIFT), 1, 0);
            ctx.insert_bel_attr_bool(tcid, bslot, DCM::PHASE_SHIFT_NEGATIVE, xlat_bit(diff));

            for attr in [
                DCM::DCM_CLKFB_IODLY_MUXINSEL,
                DCM::DCM_CLKIN_IODLY_MUXINSEL,
                DCM::DCM_CLKFB_IODLY_MUXOUT_SEL,
                DCM::DCM_CLKIN_IODLY_MUXOUT_SEL,
            ] {
                ctx.collect_bel_attr(tcid, bslot, attr);
            }

            let clkfx_d = xlat_bitvec(ctx.get_diffs_attr_bits(tcid, bslot, DCM::CLKFX_DIVIDE, 5));
            let clkfx_m = xlat_bitvec_sparse_u32(
                (1..32)
                    .map(|val| {
                        (
                            val,
                            ctx.get_diff_attr_u32(tcid, bslot, DCM::CLKFX_MULTIPLY, val),
                        )
                    })
                    .collect(),
            );
            let clkfx_divide: Vec<_> = (0..8).map(|bit| dcm_drp_bit(i, 0x50, bit).pos()).collect();
            let clkfx_multiply: Vec<_> = (0..8)
                .map(|bit| dcm_drp_bit(i, 0x50, bit + 8).pos())
                .collect();
            assert_eq!(clkfx_d, clkfx_divide[0..5]);
            assert_eq!(clkfx_m, clkfx_multiply[0..5]);
            ctx.insert_bel_attr_bitvec(tcid, bslot, DCM::CLKFX_DIVIDE, clkfx_divide);
            ctx.insert_bel_attr_bitvec(tcid, bslot, DCM::CLKFX_MULTIPLY, clkfx_multiply);

            let mut diff_low = ctx.get_diff_attr_val(
                tcid,
                bslot,
                DCM::DLL_FREQUENCY_MODE,
                enums::DCM_DLL_FREQUENCY_MODE::LOW,
            );
            let mut diff_high = ctx.get_diff_attr_val(
                tcid,
                bslot,
                DCM::DLL_FREQUENCY_MODE,
                enums::DCM_DLL_FREQUENCY_MODE::HIGH,
            );
            diff_low.apply_bitvec_diff_int(
                ctx.bel_attr_bitvec(tcid, bslot, DCM::FACTORY_JF),
                0xc080,
                0,
            );
            diff_high.apply_bitvec_diff_int(
                ctx.bel_attr_bitvec(tcid, bslot, DCM::FACTORY_JF),
                0xf0f0,
                0,
            );
            diff_high.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, DCM::DLL_PHASE_SHIFT_CALIBRATION),
                enums::DCM_DLL_PHASE_SHIFT_CALIBRATION::AUTO_ZD2,
                enums::DCM_DLL_PHASE_SHIFT_CALIBRATION::AUTO_DPS,
            );
            ctx.insert_bel_attr_enum(
                tcid,
                bslot,
                DCM::DLL_FREQUENCY_MODE,
                xlat_enum_attr(vec![
                    (enums::DCM_DLL_FREQUENCY_MODE::LOW, diff_low),
                    (enums::DCM_DLL_FREQUENCY_MODE::HIGH, diff_high),
                ]),
            );

            for (attr, bits) in [
                (DCM::CLKDV_COUNT_MAX, 0..4),
                (DCM::CLKDV_COUNT_FALL, 4..8),
                (DCM::CLKDV_COUNT_FALL_2, 8..12),
                (DCM::CLKDV_PHASE_RISE, 12..14),
                (DCM::CLKDV_PHASE_FALL, 14..16),
            ] {
                let bits = Vec::from_iter(bits.map(|bit| dcm_drp_bit(i, 0x53, bit).pos()));
                ctx.insert_bel_attr_bitvec(tcid, bslot, attr, bits);
            }
            ctx.insert_bel_attr_enum(
                tcid,
                bslot,
                DCM::CLKDV_MODE,
                BelAttributeEnum {
                    bits: vec![dcm_drp_bit(i, 0x54, 0)],
                    values: EntityPartVec::from_iter([
                        (enums::DCM_CLKDV_MODE::HALF, bits![0]),
                        (enums::DCM_CLKDV_MODE::INT, bits![1]),
                    ]),
                },
            );

            let clkdv_count_max = ctx
                .bel_attr_bitvec(tcid, bslot, DCM::CLKDV_COUNT_MAX)
                .to_vec();
            let clkdv_count_fall = ctx
                .bel_attr_bitvec(tcid, bslot, DCM::CLKDV_COUNT_FALL)
                .to_vec();
            let clkdv_count_fall_2 = ctx
                .bel_attr_bitvec(tcid, bslot, DCM::CLKDV_COUNT_FALL_2)
                .to_vec();
            let clkdv_phase_fall = ctx
                .bel_attr_bitvec(tcid, bslot, DCM::CLKDV_PHASE_FALL)
                .to_vec();
            let clkdv_mode = ctx.bel_attr_enum(tcid, bslot, DCM::CLKDV_MODE).clone();
            for val in 2..=16 {
                let mut diff = ctx.get_diff_bel_special_u32(
                    tcid,
                    bslot,
                    specials::DCM_CLKDV_DIVIDE_INT,
                    val as u32,
                );
                diff.apply_bitvec_diff_int(&clkdv_count_max, val - 1, 1);
                diff.apply_bitvec_diff_int(&clkdv_count_fall, (val - 1) / 2, 0);
                diff.apply_bitvec_diff_int(&clkdv_phase_fall, (val % 2) * 2, 0);
                diff.assert_empty();
            }
            for val in 1..=7 {
                let mut diff = ctx.get_diff_bel_special_u32(
                    tcid,
                    bslot,
                    specials::DCM_CLKDV_DIVIDE_HALF_LOW,
                    val as u32,
                );
                diff.apply_enum_diff(
                    &clkdv_mode,
                    enums::DCM_CLKDV_MODE::HALF,
                    enums::DCM_CLKDV_MODE::INT,
                );
                diff.apply_bitvec_diff_int(&clkdv_count_max, 2 * val, 1);
                diff.apply_bitvec_diff_int(&clkdv_count_fall, val / 2, 0);
                diff.apply_bitvec_diff_int(&clkdv_count_fall_2, 3 * val / 2 + 1, 0);
                diff.apply_bitvec_diff_int(&clkdv_phase_fall, (val % 2) * 2 + 1, 0);
                diff.assert_empty();
                let mut diff = ctx.get_diff_bel_special_u32(
                    tcid,
                    bslot,
                    specials::DCM_CLKDV_DIVIDE_HALF_HIGH,
                    val as u32,
                );
                diff.apply_enum_diff(
                    &clkdv_mode,
                    enums::DCM_CLKDV_MODE::HALF,
                    enums::DCM_CLKDV_MODE::INT,
                );
                diff.apply_bitvec_diff_int(&clkdv_count_max, 2 * val, 1);
                diff.apply_bitvec_diff_int(&clkdv_count_fall, (val - 1) / 2, 0);
                diff.apply_bitvec_diff_int(&clkdv_count_fall_2, (3 * val).div_ceil(2), 0);
                diff.apply_bitvec_diff_int(&clkdv_phase_fall, (val % 2) * 2, 0);
                diff.assert_empty();
            }

            for (attr, diff) in [
                DCM::OUT_CLKFX_ENABLE,
                DCM::OUT_CLKFX180_ENABLE,
                DCM::OUT_CONCUR_ENABLE,
            ]
            .into_iter()
            .zip(ctx.get_diffs_bel_special_bits(
                tcid,
                bslot,
                specials::DCM_DFS_CLK_EN,
                3,
            )) {
                let mut diff2 = ctx.get_diff_attr_bool(tcid, bslot, attr);
                diff2.apply_bit_diff(ctx.bel_attr_bit(tcid, bslot, DCM::DFS_EN), true, false);
                assert_eq!(diff, diff2);
                ctx.insert_bel_attr_bool(tcid, bslot, attr, xlat_bit(diff));
            }

            for (attr, diff) in [
                DCM::OUT_CLK0_ENABLE,
                DCM::OUT_CLK90_ENABLE,
                DCM::OUT_CLK180_ENABLE,
                DCM::OUT_CLK270_ENABLE,
                DCM::OUT_CLK2X_ENABLE,
                DCM::OUT_CLK2X180_ENABLE,
                DCM::OUT_CLKDV_ENABLE,
            ]
            .into_iter()
            .zip(ctx.get_diffs_bel_special_bits(
                tcid,
                bslot,
                specials::DCM_DLL_CLK_EN,
                7,
            )) {
                let mut diff2 = ctx.get_diff_attr_bool(tcid, bslot, attr);
                diff2.apply_bit_diff(ctx.bel_attr_bit(tcid, bslot, DCM::DLL_ZD2_EN), true, false);
                assert_eq!(diff, diff2);
                ctx.insert_bel_attr_bool(tcid, bslot, attr, xlat_bit(diff));
            }

            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_CLKIN_ENABLE);
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, DCM::DCM_CLKLOST_EN),
                true,
                false,
            );
            diff.assert_empty();
            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_CLKFB_ENABLE);
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, DCM::DLL_FDBKLOST_EN),
                true,
                false,
            );
            diff.apply_bit_diff(ctx.bel_attr_bit(tcid, bslot, DCM::DLL_ZD1_EN), true, false);
            diff.assert_empty();

            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_MAX_RANGE);
            diff.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, DCM::DCM_VBG_SEL), 9, 0);
            diff.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, DCM::DCM_VBG_PD), 2, 0);
            diff.assert_empty();
            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_MAX_SPEED);
            diff.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, DCM::DCM_VBG_SEL), 0xc, 0);
            diff.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, DCM::DCM_VBG_PD), 3, 0);
            diff.assert_empty();
            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_VREF_SOURCE_VBG);
            diff.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, DCM::DCM_VBG_SEL), 1, 0);
            diff.assert_empty();
            ctx.get_diff_bel_special(tcid, bslot, specials::DCM_VREF_SOURCE_VBG_DLL)
                .assert_empty();
            ctx.get_diff_bel_special(tcid, bslot, specials::DCM_VREF_SOURCE_VDD)
                .assert_empty();

            let mut diff = ctx.get_diff_attr_val(
                tcid,
                bslot,
                DCM::DFS_OSCILLATOR_MODE,
                enums::DCM_DFS_OSCILLATOR_MODE::AVE_FREQ_LOCK,
            );
            diff.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, DCM::DFS_HARDSYNC_B), 3, 0);
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, DCM::DFS_EARLY_LOCK),
                true,
                false,
            );
            let item = xlat_enum_attr(vec![
                (
                    enums::DCM_DFS_OSCILLATOR_MODE::PHASE_FREQ_LOCK,
                    ctx.get_diff_attr_val(
                        tcid,
                        bslot,
                        DCM::DFS_OSCILLATOR_MODE,
                        enums::DCM_DFS_OSCILLATOR_MODE::PHASE_FREQ_LOCK,
                    ),
                ),
                (enums::DCM_DFS_OSCILLATOR_MODE::AVE_FREQ_LOCK, diff),
            ]);
            ctx.insert_bel_attr_enum(tcid, bslot, DCM::DFS_OSCILLATOR_MODE, item);

            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_CLKIN_IOB);
            diff.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, DCM::DCM_CLKFB_IODLY_MUXOUT_SEL),
                enums::DCM_IODLY_MUX::DELAY_LINE,
                enums::DCM_IODLY_MUX::PASS,
            );
            diff.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, DCM::DCM_CLKIN_IODLY_MUXINSEL),
                enums::DCM_IODLY_MUX::PASS,
                enums::DCM_IODLY_MUX::DELAY_LINE,
            );
            diff.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, DCM::DCM_CLKFB_IODLY_MUXINSEL),
                enums::DCM_IODLY_MUX::DELAY_LINE,
                enums::DCM_IODLY_MUX::PASS,
            );
            diff.assert_empty();

            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_CLKFB_IOB);
            diff.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, DCM::DCM_CLKIN_IODLY_MUXOUT_SEL),
                enums::DCM_IODLY_MUX::DELAY_LINE,
                enums::DCM_IODLY_MUX::PASS,
            );
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, DCM::DCM_EXT_FB_EN),
                true,
                false,
            );
            diff.assert_empty();

            let diff = ctx
                .peek_diff_bel_special(tcid, bslot, specials::DCM_CLKOUT_PHASE_SHIFT_NONE)
                .clone();
            ctx.insert_bel_attr_enum(
                tcid,
                bslot,
                DCM::PS_MODE,
                xlat_enum_attr(vec![
                    (enums::DCM_PS_MODE::CLKFB, Diff::default()),
                    (enums::DCM_PS_MODE::CLKIN, diff),
                ]),
            );
            for spec in [
                specials::DCM_CLKOUT_PHASE_SHIFT_NONE,
                specials::DCM_CLKOUT_PHASE_SHIFT_FIXED,
                specials::DCM_CLKOUT_PHASE_SHIFT_VARIABLE_POSITIVE,
                specials::DCM_CLKOUT_PHASE_SHIFT_VARIABLE_CENTER,
                specials::DCM_CLKOUT_PHASE_SHIFT_DIRECT,
            ] {
                let mut d = ctx.get_diff_bel_special(tcid, bslot, spec);
                let mut dn = ctx.get_diff_bel_special_special(tcid, bslot, spec, specials::DCM_NEG);
                let item = ctx.bel_attr_enum(tcid, bslot, DCM::PS_MODE);
                d.apply_enum_diff(item, enums::DCM_PS_MODE::CLKIN, enums::DCM_PS_MODE::CLKFB);
                if spec != specials::DCM_CLKOUT_PHASE_SHIFT_FIXED {
                    dn.apply_enum_diff(item, enums::DCM_PS_MODE::CLKIN, enums::DCM_PS_MODE::CLKFB);
                }
                assert_eq!(d, dn);
                if spec != specials::DCM_CLKOUT_PHASE_SHIFT_NONE
                    && spec != specials::DCM_CLKOUT_PHASE_SHIFT_DIRECT
                {
                    let item = ctx.bel_attr_bit(tcid, bslot, DCM::DLL_ZD2_EN);
                    d.apply_bit_diff(item, true, false);
                }
                match spec {
                    specials::DCM_CLKOUT_PHASE_SHIFT_NONE => d.assert_empty(),
                    specials::DCM_CLKOUT_PHASE_SHIFT_FIXED
                    | specials::DCM_CLKOUT_PHASE_SHIFT_VARIABLE_POSITIVE => {
                        ctx.insert_bel_attr_bool(tcid, bslot, DCM::PS_ENABLE, xlat_bit(d))
                    }
                    specials::DCM_CLKOUT_PHASE_SHIFT_VARIABLE_CENTER => {
                        d.apply_bit_diff(
                            ctx.bel_attr_bit(tcid, bslot, DCM::PS_ENABLE),
                            true,
                            false,
                        );
                        ctx.insert_bel_attr_bool(tcid, bslot, DCM::PS_CENTERED, xlat_bit(d));
                    }
                    specials::DCM_CLKOUT_PHASE_SHIFT_DIRECT => {
                        d.apply_bit_diff(
                            ctx.bel_attr_bit(tcid, bslot, DCM::PS_ENABLE),
                            true,
                            false,
                        );
                        ctx.insert_bel_attr_bool(tcid, bslot, DCM::PS_DIRECT, xlat_bit(d));
                    }
                    _ => unreachable!(),
                }
            }

            let ref_src = wires::IMUX_CLK[0].cell(i * 7).pos();
            let (_, _, diff) = Diff::split(
                ctx.peek_diff_routing(tcid, wires::IMUX_DCM_CLKIN[i].cell(0), ref_src)
                    .clone(),
                ctx.peek_diff_routing(tcid, wires::IMUX_DCM_CLKFB[i].cell(0), ref_src)
                    .clone(),
            );
            let en = xlat_bit(diff);
            for dst in [
                wires::IMUX_DCM_CLKIN[i].cell(0),
                wires::IMUX_DCM_CLKFB[i].cell(0),
            ] {
                let mut diffs = vec![];
                let mux = &ctx.edev.db_index.tile_classes[tcls::CMT].muxes[&dst];
                for &src in mux.src.keys() {
                    let mut diff = ctx.get_diff_routing(tcid, dst, src);
                    if wires::HCLK_CMT.contains(src.wire) || wires::GIOB_CMT.contains(src.wire) {
                        // ok
                    } else {
                        diff.apply_bit_diff(en, true, false);
                    }
                    diffs.push((Some(src), diff));
                }
                ctx.insert_mux(tcid, dst, xlat_enum_raw(diffs, OcdMode::BitOrderDrpV5));
            }
            ctx.insert_bel_attr_bool(tcid, bslot, DCM::CLKIN_CLKFB_ENABLE, en);

            let mut enable = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
            enable.apply_enum_diff_raw(
                ctx.sb_mux(tcid, wires::OUT_CMT[10].cell(0)),
                &None,
                &Some(wires::IMUX_DCM_CLKIN[1].cell(0).pos()),
            );
            enable.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, DCM::CLKDV_MODE),
                enums::DCM_CLKDV_MODE::INT,
                enums::DCM_CLKDV_MODE::HALF,
            );
            enable.apply_bitvec_diff_int(
                ctx.bel_attr_bitvec(tcid, bslot, DCM::CLKDV_COUNT_MAX),
                1,
                0,
            );
            enable.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, DCM::DCM_CLKIN_IODLY_MUXINSEL),
                enums::DCM_IODLY_MUX::DELAY_LINE,
                enums::DCM_IODLY_MUX::PASS,
            );
            enable.apply_bitvec_diff_int(
                ctx.bel_attr_bitvec(tcid, bslot, DCM::CLKFX_MULTIPLY),
                1,
                0,
            );
            enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, DCM::CLKFX_DIVIDE), 1, 0);
            enable.assert_empty();
        }
    }
    if !skip_pll {
        let bslot = bslots::PLL;
        fn pll_drp_bit(reg: usize, bit: usize) -> TileBit {
            let tile = 3 + (reg >> 3);
            let frame = match bit & 3 {
                0 | 3 => 29,
                1 | 2 => 28,
                _ => unreachable!(),
            };
            let bit = (bit >> 1) | (reg & 7) << 3;
            TileBit::new(tile, frame, bit)
        }
        let mut drp = vec![];
        for reg in 0..0x20 {
            for bit in 0..16 {
                drp.push(pll_drp_bit(reg, bit).pos());
            }
        }
        ctx.insert_bel_attr_bitvec(tcid, bslot, PLL::DRP, drp);

        for pin in [
            PLL::CLKBRST,
            PLL::CLKINSEL,
            PLL::ENOUTSYNC,
            PLL::MANPDLF,
            PLL::MANPULF,
            PLL::REL,
            PLL::RST,
            PLL::SKEWCLKIN1,
            PLL::SKEWCLKIN2,
            PLL::SKEWRST,
            PLL::SKEWSTB,
        ] {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }

        for attr in [
            PLL::PLL_CLKBURST_ENABLE,
            PLL::PLL_CP_BIAS_TRIP_SHIFT,
            PLL::PLL_DIRECT_PATH_CNTRL,
            PLL::PLL_EN_DLY,
            PLL::PLL_INC_FLOCK,
            PLL::PLL_INC_SLOCK,
            PLL::PLL_LOCK_CNT_RST_FAST,
            PLL::PLL_MAN_LF_EN,
            PLL::PLL_NBTI_EN,
            PLL::PLL_PMCD_MODE,
            PLL::PLL_PWRD_CFG,
            PLL::PLL_SEL_SLIPD,
            PLL::PLL_UNLOCK_CNT_RST_FAST,
            PLL::PLL_VLFHIGH_DIS,
            PLL::PLL_EN_TCLK0,
            PLL::PLL_EN_TCLK1,
            PLL::PLL_EN_TCLK2,
            PLL::PLL_EN_TCLK3,
            PLL::PLL_EN_TCLK4,
            PLL::PLL_EN_VCO0,
            PLL::PLL_EN_VCO1,
            PLL::PLL_EN_VCO2,
            PLL::PLL_EN_VCO3,
            PLL::PLL_EN_VCO4,
            PLL::PLL_EN_VCO5,
            PLL::PLL_EN_VCO6,
            PLL::PLL_EN_VCO7,
            PLL::PLL_EN_VCO_DIV1,
            PLL::PLL_EN_VCO_DIV6,
            PLL::PLL_CLKOUT0_EN,
            PLL::PLL_CLKOUT1_EN,
            PLL::PLL_CLKOUT2_EN,
            PLL::PLL_CLKOUT3_EN,
            PLL::PLL_CLKOUT4_EN,
            PLL::PLL_CLKOUT5_EN,
            PLL::PLL_CLKFBOUT_EN,
            PLL::PLL_CLKOUT0_EDGE,
            PLL::PLL_CLKOUT1_EDGE,
            PLL::PLL_CLKOUT2_EDGE,
            PLL::PLL_CLKOUT3_EDGE,
            PLL::PLL_CLKOUT4_EDGE,
            PLL::PLL_CLKOUT5_EDGE,
            PLL::PLL_CLKFBOUT_EDGE,
            PLL::PLL_CLKFBOUT2_EDGE,
            PLL::PLL_DIVCLK_EDGE,
            PLL::PLL_CLKOUT0_NOCOUNT,
            PLL::PLL_CLKOUT1_NOCOUNT,
            PLL::PLL_CLKOUT2_NOCOUNT,
            PLL::PLL_CLKOUT3_NOCOUNT,
            PLL::PLL_CLKOUT4_NOCOUNT,
            PLL::PLL_CLKOUT5_NOCOUNT,
            PLL::PLL_CLKFBOUT_NOCOUNT,
            PLL::PLL_CLKFBOUT2_NOCOUNT,
            PLL::PLL_DIVCLK_NOCOUNT,
        ] {
            ctx.collect_bel_attr_bi(tcid, bslot, attr);
        }
        for attr in [
            PLL::PLL_AVDD_COMP_SET,
            PLL::PLL_AVDD_VBG_PD,
            PLL::PLL_AVDD_VBG_SEL,
            PLL::PLL_CLKBURST_CNT,
            PLL::PLL_CLK0MX,
            PLL::PLL_CLK1MX,
            PLL::PLL_CLK2MX,
            PLL::PLL_CLK3MX,
            PLL::PLL_CLK4MX,
            PLL::PLL_CLK5MX,
            PLL::PLL_CLKFBMX,
            PLL::PLL_CLKCNTRL,
            PLL::PLL_CP,
            PLL::PLL_CP_RES,
            PLL::PLL_DVDD_COMP_SET,
            PLL::PLL_DVDD_VBG_PD,
            PLL::PLL_DVDD_VBG_SEL,
            PLL::PLL_INTFB,
            PLL::PLL_LFHF,
            PLL::PLL_LF_NEN,
            PLL::PLL_LF_PEN,
            PLL::PLL_LOCK_CNT,
            PLL::PLL_PFD_CNTRL,
            PLL::PLL_PFD_DLY,
            PLL::PLL_RES,
            PLL::PLL_TCK4_SEL,
            PLL::PLL_UNLOCK_CNT,
            PLL::CLKOUT0_DESKEW_ADJUST,
            PLL::CLKOUT1_DESKEW_ADJUST,
            PLL::CLKOUT2_DESKEW_ADJUST,
            PLL::CLKOUT3_DESKEW_ADJUST,
            PLL::CLKOUT4_DESKEW_ADJUST,
            PLL::CLKOUT5_DESKEW_ADJUST,
            PLL::CLKFBOUT_DESKEW_ADJUST,
            PLL::PLL_CLKOUT0_DT,
            PLL::PLL_CLKOUT0_HT,
            PLL::PLL_CLKOUT0_LT,
            PLL::PLL_CLKOUT0_PM,
            PLL::PLL_CLKOUT1_DT,
            PLL::PLL_CLKOUT1_HT,
            PLL::PLL_CLKOUT1_LT,
            PLL::PLL_CLKOUT1_PM,
            PLL::PLL_CLKOUT2_DT,
            PLL::PLL_CLKOUT2_HT,
            PLL::PLL_CLKOUT2_LT,
            PLL::PLL_CLKOUT2_PM,
            PLL::PLL_CLKOUT3_DT,
            PLL::PLL_CLKOUT3_HT,
            PLL::PLL_CLKOUT3_LT,
            PLL::PLL_CLKOUT3_PM,
            PLL::PLL_CLKOUT4_DT,
            PLL::PLL_CLKOUT4_HT,
            PLL::PLL_CLKOUT4_LT,
            PLL::PLL_CLKOUT4_PM,
            PLL::PLL_CLKOUT5_DT,
            PLL::PLL_CLKOUT5_HT,
            PLL::PLL_CLKOUT5_LT,
            PLL::PLL_CLKOUT5_PM,
            PLL::PLL_CLKFBOUT_DT,
            PLL::PLL_CLKFBOUT_HT,
            PLL::PLL_CLKFBOUT_LT,
            PLL::PLL_CLKFBOUT_PM,
            PLL::PLL_CLKFBOUT2_DT,
            PLL::PLL_CLKFBOUT2_HT,
            PLL::PLL_CLKFBOUT2_LT,
            PLL::PLL_DIVCLK_DT,
            PLL::PLL_DIVCLK_HT,
            PLL::PLL_DIVCLK_LT,
            PLL::PLL_EN_CNTRL,
            PLL::PLL_FLOCK,
            PLL::PLL_IN_DLY_MX_SEL,
            PLL::PLL_IN_DLY_SET,
            PLL::PLL_LOCK_FB_P1,
            PLL::PLL_LOCK_FB_P2,
            PLL::PLL_LOCK_REF_P1,
            PLL::PLL_LOCK_REF_P2,
            PLL::PLL_MISC,
        ] {
            ctx.collect_bel_attr(tcid, bslot, attr);
        }

        ctx.collect_bel_attr(tcid, bslot, PLL::CLKINSEL_MODE_DYNAMIC);
        let item = xlat_bit(
            ctx.peek_diff_routing(
                tcid,
                wires::IMUX_PLL_CLKIN1.cell(0),
                wires::GIOB_CMT[5].cell(0).pos(),
            )
            .clone(),
        );
        ctx.insert_bel_attr_bool(tcid, bslot, PLL::CLKINSEL_STATIC_VAL, item);
        ctx.collect_mux_ocd(tcid, wires::IMUX_PLL_CLKFB.cell(0), OcdMode::BitOrderDrpV5);

        let BelInfo::SwitchBox(ref sb) = ctx.edev.db[tcid].bels[bslots::SPEC_INT] else {
            unreachable!()
        };
        let pair_mux = sb
            .items
            .iter()
            .find_map(|item| {
                if let SwitchBoxItem::PairMux(pm) = item {
                    Some(pm)
                } else {
                    None
                }
            })
            .unwrap();

        let mut diffs = vec![];
        for &srcs in pair_mux.src.keys() {
            if let Some(src0) = srcs[0] {
                diffs.push((
                    srcs,
                    ctx.get_diff_routing(tcid, wires::IMUX_PLL_CLKIN1.cell(0), src0),
                ));
            }
            if let Some(src1) = srcs[1] {
                let mut diff = ctx.get_diff_routing(tcid, wires::IMUX_PLL_CLKIN1.cell(0), src1);
                diff.apply_bit_diff(
                    ctx.bel_attr_bit(tcid, bslot, PLL::CLKINSEL_STATIC_VAL),
                    true,
                    false,
                );
                diffs.push((srcs, diff));
            }
        }
        ctx.insert_pairmux(
            tcid,
            pair_mux.dst,
            xlat_enum_raw(diffs, OcdMode::BitOrderDrpV5),
        );

        ctx.collect_mux_ocd(
            tcid,
            wires::OMUX_PLL_SKEWCLKIN1.cell(0),
            OcdMode::BitOrderDrpV5,
        );
        ctx.collect_mux_ocd(
            tcid,
            wires::OMUX_PLL_SKEWCLKIN2.cell(0),
            OcdMode::BitOrderDrpV5,
        );

        for mult in 1..=64 {
            let row = ctx.edev.db[PLL_MULT]
                .rows
                .get(&format!("_{mult}"))
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
                for (attr, field, width) in [
                    (PLL::PLL_CP, field_cp, 4),
                    (PLL::PLL_RES, field_res, 4),
                    (PLL::PLL_LFHF, field_lfhf, 2),
                ] {
                    let val = extract_bitvec_val_part(
                        ctx.bel_attr_bitvec(tcid, bslot, attr),
                        &BitVec::repeat(false, width),
                        &mut diff,
                    );
                    ctx.insert_table_bitvec(PLL_MULT, row, field, val);
                }
                for attr in [
                    PLL::PLL_CLKFBOUT_NOCOUNT,
                    PLL::PLL_CLKFBOUT_LT,
                    PLL::PLL_CLKFBOUT_HT,
                    PLL::PLL_CLKFBOUT_EDGE,
                ] {
                    diff.discard_polbits(ctx.bel_attr_bitvec(tcid, bslot, attr));
                }
                diff.assert_empty();
            }
        }
        let mut enable = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_RES), 0xb, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CP), 2, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_LFHF), 3, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_DIVCLK_EDGE), 1, 0);
        enable.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKFBOUT_EDGE),
            1,
            0,
        );
        enable.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKOUT0_EDGE),
            1,
            0,
        );
        enable.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKOUT1_EDGE),
            1,
            0,
        );
        enable.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKOUT2_EDGE),
            1,
            0,
        );
        enable.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKOUT3_EDGE),
            1,
            0,
        );
        enable.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKOUT4_EDGE),
            1,
            0,
        );
        enable.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKOUT5_EDGE),
            1,
            0,
        );
        enable.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_DIVCLK_NOCOUNT),
            1,
            0,
        );
        enable.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKFBOUT_NOCOUNT),
            1,
            0,
        );
        enable.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKOUT0_NOCOUNT),
            1,
            0,
        );
        enable.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKOUT1_NOCOUNT),
            1,
            0,
        );
        enable.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKOUT2_NOCOUNT),
            1,
            0,
        );
        enable.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKOUT3_NOCOUNT),
            1,
            0,
        );
        enable.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKOUT4_NOCOUNT),
            1,
            0,
        );
        enable.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKOUT5_NOCOUNT),
            1,
            0,
        );
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_DIVCLK_LT), 1, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKFBOUT_LT), 1, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKOUT0_LT), 1, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKOUT1_LT), 1, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKOUT2_LT), 1, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKOUT3_LT), 1, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKOUT4_LT), 1, 0);
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_CLKOUT5_LT), 1, 0);
        let dly_val = extract_bitvec_val_part(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_IN_DLY_SET),
            &bits![0; 9],
            &mut enable,
        );
        enable.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_EN_DLY), 1, 0);
        enable.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_IN_DLY_MX_SEL),
            8,
            0,
        );
        enable.apply_bit_diff(ctx.bel_input_inv(tcid, bslot, PLL::REL), true, false);
        enable.apply_enum_diff_raw(
            ctx.sb_mux(tcid, wires::OMUX_PLL_SKEWCLKIN1.cell(0)),
            &None,
            &Some(wires::OUT_PLL_CLKOUTDCM[0].cell(0).pos()),
        );
        enable.apply_enum_diff_raw(
            ctx.sb_mux(tcid, wires::OMUX_PLL_SKEWCLKIN2.cell(0)),
            &None,
            &Some(wires::OUT_PLL_CLKOUTDCM[0].cell(0).pos()),
        );
        enable.apply_enum_diff_raw(
            ctx.sb_mux(tcid, wires::OUT_CMT[10].cell(0)),
            &None,
            &Some(wires::IMUX_DCM_CLKIN[1].cell(0).pos()),
        );
        ctx.insert_bel_attr_bool(tcid, bslot, PLL::PLL_EN, xlat_bit(enable));

        ctx.get_diff_bel_special(tcid, bslot, specials::PLL_COMPENSATION_SYSTEM_SYNCHRONOUS)
            .assert_empty();
        let mut diff =
            ctx.get_diff_bel_special(tcid, bslot, specials::PLL_COMPENSATION_SOURCE_SYNCHRONOUS);
        diff.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_IN_DLY_SET),
            &bits![0; 9],
            &dly_val,
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::PLL_COMPENSATION_EXTERNAL);
        diff.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_IN_DLY_SET),
            &bits![0; 9],
            &dly_val,
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::PLL_COMPENSATION_INTERNAL);
        diff.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_INTFB), 2, 0);
        diff.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_IN_DLY_SET),
            &bits![0; 9],
            &dly_val,
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::PLL_COMPENSATION_DCM2PLL);
        diff.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_IN_DLY_SET),
            &bits![0; 9],
            &dly_val,
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::PLL_COMPENSATION_PLL2DCM);
        diff.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslot, PLL::PLL_IN_DLY_SET),
            &bits![0; 9],
            &dly_val,
        );
        diff.assert_empty();
        ctx.insert_devdata_bitvec(devdata::PLL_IN_DLY_SET, dly_val);
    }
    {
        let tcid = tcls::HCLK_CMT;
        let bslot = bslots::HCLK_CMT_DRP;
        let bit = xlat_bit(ctx.get_diff_attr_bool(tcid, bslot, bcls::HCLK_CMT_DRP::DRP_MASK));
        for tcid in [tcls::HCLK_CMT, tcls::HCLK_IO_CMT_N, tcls::HCLK_IO_CMT_S] {
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::HCLK_CMT_DRP::DRP_MASK, bit);
        }
    }
}
