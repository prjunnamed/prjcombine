use prjcombine_entity::EntityPartVec;
use prjcombine_interconnect::db::{BelAttributeEnum, BelInfo};
use prjcombine_re_collector::diff::{
    Diff, OcdMode, xlat_bit, xlat_bit_wide_bi, xlat_enum_attr, xlat_enum_attr_ocd, xlat_enum_raw,
};
use prjcombine_re_hammer::Session;
use prjcombine_types::{bits, bsdata::TileBit};
use prjcombine_virtex4::defs::{
    self,
    bcls::DCM_V4,
    bslots, enums,
    virtex4::{tcls, wires},
};

use crate::{
    backend::{IseBackend, MultiValue, PinFromKind},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::mutex::{WireMutexExclusive, WireMutexShared},
    },
    virtex4::specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::DCM);
    let mut bctx = ctx.bel(defs::bslots::DCM[0]);
    let mode = "DCM_ADV";

    bctx.build()
        .test_bel_special(specials::PRESENT)
        .mode(mode)
        .commit();
    for pin in [
        DCM_V4::DEN,
        DCM_V4::DWE,
        DCM_V4::DI[0],
        DCM_V4::DI[1],
        DCM_V4::DI[2],
        DCM_V4::DI[3],
        DCM_V4::DI[4],
        DCM_V4::DI[5],
        DCM_V4::DI[6],
        DCM_V4::DI[7],
        DCM_V4::DI[8],
        DCM_V4::DI[9],
        DCM_V4::DI[10],
        DCM_V4::DI[11],
        DCM_V4::DI[12],
        DCM_V4::DI[13],
        DCM_V4::DI[14],
        DCM_V4::DI[15],
        DCM_V4::DADDR[0],
        DCM_V4::DADDR[1],
        DCM_V4::DADDR[2],
        DCM_V4::DADDR[3],
        DCM_V4::DADDR[4],
        DCM_V4::DADDR[5],
        DCM_V4::DADDR[6],
        // DCLK?
        DCM_V4::RST,
        // PSCLK?
        DCM_V4::PSEN,
        DCM_V4::PSINCDEC,
        DCM_V4::CTLMODE,
        DCM_V4::CTLSEL[0],
        DCM_V4::CTLSEL[1],
        DCM_V4::CTLSEL[2],
        DCM_V4::CTLOSC1,
        DCM_V4::CTLOSC2,
        DCM_V4::CTLGO,
        DCM_V4::FREEZE_DLL,
        DCM_V4::FREEZE_DFS,
    ] {
        bctx.mode(mode).test_bel_input_inv_auto(pin);
    }

    for (attr, pin) in [
        (DCM_V4::OUT_CLK0_ENABLE, "CLK0"),
        (DCM_V4::OUT_CLK90_ENABLE, "CLK90"),
        (DCM_V4::OUT_CLK180_ENABLE, "CLK180"),
        (DCM_V4::OUT_CLK270_ENABLE, "CLK270"),
        (DCM_V4::OUT_CLK2X_ENABLE, "CLK2X"),
        (DCM_V4::OUT_CLK2X180_ENABLE, "CLK2X180"),
        (DCM_V4::OUT_CLKDV_ENABLE, "CLKDV"),
        (DCM_V4::OUT_CLKFX_ENABLE, "CLKFX"),
        (DCM_V4::OUT_CLKFX180_ENABLE, "CLKFX180"),
        (DCM_V4::OUT_CONCUR_ENABLE, "CONCUR"),
    ] {
        bctx.mode(mode)
            .mutex("PIN", pin)
            .test_bel_attr_bits(attr)
            .pin(pin)
            .commit();
    }
    bctx.mode(mode)
        .pin_from("CLKFB", PinFromKind::Bufg)
        .test_bel_attr_bits(DCM_V4::CLKFB_ENABLE)
        .pin("CLKFB")
        .commit();
    bctx.mode(mode)
        .pin_from("CLKIN", PinFromKind::Bufg)
        .test_bel_attr_bits(DCM_V4::CLKIN_ENABLE)
        .pin("CLKIN")
        .commit();
    bctx.mode(mode)
        .global_mutex("DCM", "USE")
        .pin("CLKIN")
        .pin("CLKFB")
        .pin_from("CLKFB", PinFromKind::Bufg)
        .test_bel_attr_bits(DCM_V4::CLKIN_IOB)
        .pin_from("CLKIN", PinFromKind::Bufg, PinFromKind::Iob)
        .commit();
    bctx.mode(mode)
        .global_mutex("DCM", "USE")
        .pin("CLKIN")
        .pin("CLKFB")
        .pin_from("CLKIN", PinFromKind::Bufg)
        .test_bel_attr_bits(DCM_V4::CLKFB_IOB)
        .pin_from("CLKFB", PinFromKind::Bufg, PinFromKind::Iob)
        .commit();

    for attr in [
        DCM_V4::BGM_VLDLY,
        DCM_V4::BGM_LDLY,
        DCM_V4::BGM_SDLY,
        DCM_V4::BGM_VSDLY,
        DCM_V4::BGM_SAMPLE_LEN,
        DCM_V4::DLL_PD_DLY_SEL,
        DCM_V4::DLL_DEAD_TIME,
        DCM_V4::DLL_LIVE_TIME,
        DCM_V4::DLL_DESKEW_MINTAP,
        DCM_V4::DLL_DESKEW_MAXTAP,
        DCM_V4::DLL_PHASE_SHIFT_LFC,
        DCM_V4::DLL_PHASE_SHIFT_HFC,
        DCM_V4::DLL_SETTLE_TIME,
        DCM_V4::DESKEW_ADJUST,
        DCM_V4::PHASE_SHIFT,
    ] {
        bctx.mode(mode)
            .test_bel_attr_multi(attr, MultiValue::Dec(0));
    }
    for attr in [
        DCM_V4::DFS_COIN_WINDOW,
        DCM_V4::DFS_SPARE,
        DCM_V4::DCM_PULSE_WIDTH_CORRECTION_LOW,
        DCM_V4::DCM_PULSE_WIDTH_CORRECTION_HIGH,
        DCM_V4::DCM_VBG_PD,
        DCM_V4::DCM_VREG_PHASE_MARGIN,
        DCM_V4::DLL_SPARE,
        DCM_V4::DLL_TEST_MUX_SEL,
    ] {
        bctx.mode(mode).test_bel_attr_multi(attr, MultiValue::Bin);
    }

    for attr in [
        DCM_V4::BGM_MODE,
        DCM_V4::BGM_CONFIG_REF_SEL,
        DCM_V4::DCM_PERFORMANCE_MODE,
        DCM_V4::DFS_FREQUENCY_MODE,
        DCM_V4::DFS_COARSE_SEL,
        DCM_V4::DFS_TP_SEL,
        DCM_V4::DFS_FINE_SEL,
        DCM_V4::DLL_FREQUENCY_MODE,
        DCM_V4::DLL_CONTROL_CLOCK_SPEED,
        DCM_V4::DLL_PHASE_DETECTOR_MODE,
    ] {
        bctx.mode(mode).test_bel_attr_auto(attr);
    }

    for attr in [
        DCM_V4::DCM_CLKDV_CLKFX_ALIGNMENT,
        DCM_V4::DCM_LOCK_HIGH,
        DCM_V4::DCM_VREG_ENABLE,
        DCM_V4::DCM_UNUSED_TAPS_POWERDOWN,
        DCM_V4::CLKIN_DIVIDE_BY_2,
        DCM_V4::PMCD_SYNC,
        DCM_V4::DLL_PHASE_DETECTOR_AUTO_RESET,
        DCM_V4::DLL_PERIOD_LOCK_BY1,
        DCM_V4::DLL_DESKEW_LOCK_BY1,
        DCM_V4::DLL_PHASE_SHIFT_LOCK_BY1,
        DCM_V4::DLL_CTL_SEL_CLKIN_DIV2,
        DCM_V4::DUTY_CYCLE_CORRECTION,
        DCM_V4::DFS_EN_RELRST,
        DCM_V4::DFS_NON_STOP,
        DCM_V4::DFS_EXTEND_RUN_TIME,
        DCM_V4::DFS_EXTEND_HALT_TIME,
        DCM_V4::DFS_EXTEND_FLUSH_TIME,
        DCM_V4::DFS_SKIP_FINE,
    ] {
        bctx.mode(mode)
            .test_bel_attr_bool_auto(attr, "FALSE", "TRUE");
    }
    for val in 1..16 {
        bctx.mode(mode)
            .test_bel_attr_bitvec_u32(DCM_V4::BGM_VADJ, val)
            .attr("BGM_VADJ", val.to_string())
            .commit();
    }
    bctx.mode(mode)
        .test_bel_attr_multi(DCM_V4::BGM_MULTIPLY, MultiValue::Dec(1));
    bctx.mode(mode)
        .test_bel_attr_multi(DCM_V4::BGM_DIVIDE, MultiValue::Dec(1));

    bctx.mode(mode)
        .no_pin("CLKFB")
        .test_bel_attr_bool_auto(DCM_V4::DCM_EXT_FB_EN, "FALSE", "TRUE");
    for (spec, val) in [
        (specials::DCM_VREF_SOURCE_VDD, "VDD"),
        (specials::DCM_VREF_SOURCE_VBG_DLL, "VBG_DLL"),
        (specials::DCM_VREF_SOURCE_VBG, "VBG"),
        (specials::DCM_VREF_SOURCE_BGM_SNAP, "BGM_SNAP"),
        (specials::DCM_VREF_SOURCE_BGM_ABS_SNAP, "BGM_ABS_SNAP"),
        (specials::DCM_VREF_SOURCE_BGM_ABS_REF, "BGM_ABS_REF"),
    ] {
        bctx.mode(mode)
            .attr("DCM_PERFORMANCE_MODE", "MAX_RANGE")
            .test_bel_special_special(spec, specials::DCM_MAX_RANGE)
            .attr_diff("DCM_VREF_SOURCE", "VDD", val)
            .commit();
        bctx.mode(mode)
            .attr("DCM_PERFORMANCE_MODE", "MAX_SPEED")
            .test_bel_special_special(spec, specials::DCM_MAX_SPEED)
            .attr_diff("DCM_VREF_SOURCE", "VDD", val)
            .commit();
    }
    bctx.mode(mode)
        .global("GTS_CYCLE", "1")
        .global("DONE_CYCLE", "1")
        .global("LCK_CYCLE", "NOWAIT")
        .test_bel_attr_bool_auto(DCM_V4::STARTUP_WAIT, "FALSE", "TRUE");

    bctx.mode(mode)
        .no_pin("CLKFB")
        .test_bel_attr_auto(DCM_V4::CLK_FEEDBACK);
    bctx.mode(mode)
        .pin("CLKFB")
        .pin_from("CLKFB", PinFromKind::Bufg)
        .test_bel_attr_special_auto(DCM_V4::CLK_FEEDBACK, specials::DCM_CLKFB);
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
            .attr("PHASE_SHIFT", "1")
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
            .attr("PHASE_SHIFT", "-1")
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
        bctx.mode(mode)
            .mutex("PIN", "NONE")
            .attr("PHASE_SHIFT", "1")
            .pin("CLK0")
            .test_bel_special_special(spec, specials::DCM_DLL)
            .attr("CLKOUT_PHASE_SHIFT", val)
            .commit();
    }
    bctx.mode(mode)
        .attr("DCM_VREF_SOURCE", "VDD")
        .test_bel_attr_multi(DCM_V4::DCM_VBG_SEL, MultiValue::Bin);
    bctx.mode(mode)
        .attr("CLKOUT_PHASE_SHIFT", "NONE")
        .test_bel_special(specials::DCM_PHASE_SHIFT_N1)
        .attr("PHASE_SHIFT", "-1")
        .commit();

    bctx.mode(mode)
        .attr("CLKOUT_PHASE_SHIFT", "NONE")
        .test_bel_attr_auto(DCM_V4::DLL_PHASE_SHIFT_CALIBRATION);

    bctx.mode(mode)
        .attr("DLL_FREQUENCY_MODE", "")
        .test_bel_attr_multi(DCM_V4::FACTORY_JF, MultiValue::Hex(0));
    for val in 2..=16 {
        bctx.mode(mode)
            .test_bel_special_u32(specials::DCM_CLKDV_DIVIDE_INT, val)
            .attr("CLKDV_DIVIDE", format!("{val}.0"))
            .commit();
    }
    for (spec, dll_mode) in [
        (specials::DCM_CLKDV_DIVIDE_HALF_LOW, "LOW"),
        (specials::DCM_CLKDV_DIVIDE_HALF_HIGH, "HIGH"),
        (specials::DCM_CLKDV_DIVIDE_HALF_HIGH, "HIGH_SER"),
    ] {
        for val in 1..=7 {
            bctx.mode(mode)
                .global_mutex("DCM", "USE")
                .attr("DLL_FREQUENCY_MODE", dll_mode)
                .test_bel_special_u32(spec, val)
                .attr("CLKDV_DIVIDE", format!("{val}.5"))
                .commit();
        }
    }

    bctx.mode(mode)
        .attr("DFS_OSCILLATOR_MODE", "")
        .test_bel_attr_bool_auto(DCM_V4::DFS_EARLY_LOCK, "FALSE", "TRUE");
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
            .test_bel_attr_val(DCM_V4::DFS_AVE_FREQ_GAIN, val)
            .attr("DFS_AVE_FREQ_GAIN", vname)
            .commit();
    }
    for val in 1..8 {
        bctx.mode(mode)
            .test_bel_attr_bitvec_u32(DCM_V4::DFS_AVE_FREQ_SAMPLE_INTERVAL, val)
            .attr("DFS_AVE_FREQ_SAMPLE_INTERVAL", val.to_string())
            .commit();
    }
    for val in 1..16 {
        bctx.mode(mode)
            .test_bel_attr_bitvec_u32(DCM_V4::DFS_AVE_FREQ_ADJ_INTERVAL, val)
            .attr("DFS_AVE_FREQ_ADJ_INTERVAL", val.to_string())
            .commit();
    }
    bctx.mode(mode)
        .test_bel_attr_bool_auto(DCM_V4::DFS_TRACKMODE, "0", "1");
    bctx.mode(mode)
        .mutex("PIN", "NONE")
        .pin("CLK0")
        .pin("CLKFX")
        .test_bel_attr_auto(DCM_V4::DFS_OSCILLATOR_MODE);
    bctx.mode(mode)
        .attr("DFS_OSCILLATOR_MODE", "")
        .test_bel_attr_multi(DCM_V4::DFS_HARDSYNC, MultiValue::Bin);

    bctx.mode(mode)
        .test_bel_attr_multi(DCM_V4::CLKFX_DIVIDE, MultiValue::Dec(1));
    for val in 2..=32 {
        bctx.mode(mode)
            .test_bel_attr_bitvec_u32(DCM_V4::CLKFX_MULTIPLY, val - 1)
            .attr("CLKFX_MULTIPLY", val.to_string())
            .commit();
    }

    let muxes = &backend.edev.db_index.tile_classes[tcls::DCM].muxes;

    let BelInfo::Bel(ref bel) = backend.edev.db[tcls::DCM].bels[bslots::DCM[0]] else {
        unreachable!()
    };
    for (pin, pname, opin, opname) in [
        (DCM_V4::CLKIN, "CLKIN", DCM_V4::CLKFB, "CLKFB"),
        (DCM_V4::CLKFB, "CLKFB", DCM_V4::CLKIN, "CLKIN"),
    ] {
        let wire = bel.inputs[pin].wire();
        let odst = bel.inputs[opin].wire();
        let mux = &muxes[&wire];
        for (out, is_test) in [(pname, false), (format!("{pname}_TEST").as_str(), true)] {
            for &src in mux.src.keys() {
                let mut builder = bctx
                    .build()
                    .mode(mode)
                    .pin(pname)
                    .prop(WireMutexExclusive::new(mux.dst));
                if wires::HCLK_DCM.contains(src.wire)
                    || wires::GIOB_DCM.contains(src.wire)
                    || wires::MGT_DCM.contains(src.wire)
                    || wires::DCM_DCM_I.contains(src.wire)
                {
                    if !wires::DCM_DCM_I.contains(src.wire) {
                        builder = builder.global_mutex("HCLK_DCM", "USE");
                    }
                    builder = builder
                        .pin(opname)
                        .prop(WireMutexExclusive::new(odst))
                        .pip(opname, src.tw);
                }
                if wires::IMUX_CLK_OPTINV.contains(src.wire) {
                    builder = builder.prop(WireMutexExclusive::new(src.tw));
                } else {
                    builder = builder.prop(WireMutexShared::new(src.tw));
                }
                if is_test {
                    builder
                        .test_routing_pair_special(mux.dst, src, specials::TEST)
                        .pip(out, src.tw)
                        .commit();
                } else {
                    builder.test_routing(mux.dst, src).pip(out, src.tw).commit();
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::DCM;
    let bslot = bslots::DCM[0];

    let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);

    fn reg_bit(addr: usize, bit: usize) -> TileBit {
        TileBit::new(
            (addr >> 2) & 3,
            20 - (addr >> 4 & 1),
            bit + 1 + (addr & 3) * 20,
        )
    }

    let mut drp = vec![];
    let mut drp_mask = vec![];
    for addr in 0x40..0x60 {
        drp_mask.push(reg_bit(addr, 17).pos());
        drp.extend((0..16).map(|bit| reg_bit(addr, bit).pos()));
    }
    present.apply_bitvec_diff(&drp_mask, &bits![1; 32], &bits![0; 32]);
    ctx.insert_bel_attr_bitvec(tcid, bslot, DCM_V4::DRP, drp);
    ctx.insert_bel_attr_bitvec(tcid, bslot, DCM_V4::DRP_MASK, drp_mask);

    for pin in [
        DCM_V4::RST,
        DCM_V4::CTLMODE,
        DCM_V4::FREEZE_DLL,
        DCM_V4::FREEZE_DFS,
        DCM_V4::DEN,
        DCM_V4::DWE,
    ] {
        ctx.collect_bel_input_inv_int_bi(&[tcls::INT; 4], tcid, bslot, pin);
    }

    for pin in [
        DCM_V4::DI[0],
        DCM_V4::DI[1],
        DCM_V4::DI[2],
        DCM_V4::DI[3],
        DCM_V4::DI[4],
        DCM_V4::DI[5],
        DCM_V4::DI[6],
        DCM_V4::DI[7],
        DCM_V4::DI[8],
        DCM_V4::DI[9],
        DCM_V4::DI[10],
        DCM_V4::DI[11],
        DCM_V4::DI[12],
        DCM_V4::DI[13],
        DCM_V4::DI[14],
        DCM_V4::DI[15],
        DCM_V4::DADDR[0],
        DCM_V4::DADDR[1],
        DCM_V4::DADDR[2],
        DCM_V4::DADDR[3],
        DCM_V4::DADDR[4],
        DCM_V4::DADDR[5],
        DCM_V4::DADDR[6],
        DCM_V4::PSEN,
        DCM_V4::PSINCDEC,
        DCM_V4::CTLSEL[0],
        DCM_V4::CTLSEL[1],
        DCM_V4::CTLSEL[2],
        DCM_V4::CTLOSC1,
        DCM_V4::CTLOSC2,
        DCM_V4::CTLGO,
    ] {
        ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
    }

    let diff = ctx.get_diff_attr_bool(tcid, bslot, DCM_V4::OUT_CLK2X_ENABLE);
    for attr in [
        DCM_V4::OUT_CLK2X180_ENABLE,
        DCM_V4::OUT_CLKDV_ENABLE,
        DCM_V4::OUT_CLK90_ENABLE,
        DCM_V4::OUT_CLK180_ENABLE,
        DCM_V4::OUT_CLK270_ENABLE,
    ] {
        assert_eq!(diff, ctx.get_diff_attr_bool(tcid, bslot, attr));
    }
    let diff_0 = ctx.get_diff_attr_bool(tcid, bslot, DCM_V4::OUT_CLK0_ENABLE);
    let diff_0 = diff_0.combine(&!&diff);
    ctx.insert_bel_attr_bool(tcid, bslot, DCM_V4::OUT_CLK0_ENABLE, xlat_bit(diff_0));
    // ???
    ctx.insert_bel_attr_bool(
        tcid,
        bslot,
        DCM_V4::OUT_CLK90_ENABLE,
        reg_bit(0x4e, 1).pos(),
    );
    ctx.insert_bel_attr_bool(
        tcid,
        bslot,
        DCM_V4::OUT_CLK180_ENABLE,
        reg_bit(0x4e, 2).pos(),
    );
    ctx.insert_bel_attr_bool(
        tcid,
        bslot,
        DCM_V4::OUT_CLK270_ENABLE,
        reg_bit(0x4e, 3).pos(),
    );
    ctx.insert_bel_attr_bool(
        tcid,
        bslot,
        DCM_V4::OUT_CLK2X_ENABLE,
        reg_bit(0x4e, 4).pos(),
    );
    ctx.insert_bel_attr_bool(
        tcid,
        bslot,
        DCM_V4::OUT_CLK2X180_ENABLE,
        reg_bit(0x4e, 5).pos(),
    );
    ctx.insert_bel_attr_bool(
        tcid,
        bslot,
        DCM_V4::OUT_CLKDV_ENABLE,
        reg_bit(0x4e, 6).pos(),
    );
    ctx.insert_bel_attr_bool(
        tcid,
        bslot,
        DCM_V4::OUT_CLKFX180_ENABLE,
        reg_bit(0x51, 8).pos(),
    );
    ctx.insert_bel_attr_bool(
        tcid,
        bslot,
        DCM_V4::OUT_CLKFX_ENABLE,
        reg_bit(0x51, 9).pos(),
    );
    ctx.insert_bel_attr_bool(
        tcid,
        bslot,
        DCM_V4::OUT_CONCUR_ENABLE,
        reg_bit(0x51, 10).pos(),
    );

    ctx.insert_bel_attr_bool(tcid, bslot, DCM_V4::DLL_ZD2_EN, xlat_bit(diff));
    let diff = ctx.get_diff_attr_bool(tcid, bslot, DCM_V4::OUT_CLKFX_ENABLE);
    for attr in [DCM_V4::OUT_CLKFX180_ENABLE, DCM_V4::OUT_CONCUR_ENABLE] {
        assert_eq!(diff, ctx.get_diff_attr_bool(tcid, bslot, attr));
    }
    ctx.insert_bel_attr_bool(tcid, bslot, DCM_V4::DFS_ENABLE, xlat_bit(diff));

    ctx.collect_bel_attr(tcid, bslot, DCM_V4::BGM_VLDLY);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::BGM_LDLY);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::BGM_SDLY);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::BGM_VSDLY);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::BGM_SAMPLE_LEN);
    ctx.collect_bel_attr_ocd(tcid, bslot, DCM_V4::BGM_MODE, OcdMode::BitOrder);
    ctx.collect_bel_attr_ocd(tcid, bslot, DCM_V4::BGM_CONFIG_REF_SEL, OcdMode::BitOrder);
    ctx.collect_bel_attr_sparse(tcid, bslot, DCM_V4::BGM_VADJ, 1..16);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::BGM_MULTIPLY);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::BGM_DIVIDE);

    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::DCM_CLKDV_CLKFX_ALIGNMENT);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::DCM_LOCK_HIGH);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::DCM_VREG_ENABLE);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::DCM_EXT_FB_EN);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::DCM_UNUSED_TAPS_POWERDOWN);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DCM_PERFORMANCE_MODE);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::STARTUP_WAIT);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::CLKIN_DIVIDE_BY_2);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::PMCD_SYNC);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DESKEW_ADJUST);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DCM_PULSE_WIDTH_CORRECTION_LOW);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DCM_PULSE_WIDTH_CORRECTION_HIGH);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DCM_VBG_PD);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DCM_VBG_SEL);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DCM_VREG_PHASE_MARGIN);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::PHASE_SHIFT);
    let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_PHASE_SHIFT_N1);
    diff.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, DCM_V4::PHASE_SHIFT), 1, 0);
    ctx.insert_bel_attr_bool(tcid, bslot, DCM_V4::PHASE_SHIFT_NEGATIVE, xlat_bit(diff));

    let mut diffs = vec![];
    for (val, spec) in [
        (
            enums::DCM_VREF_SOURCE::VDD_VBG,
            specials::DCM_VREF_SOURCE_VDD,
        ),
        (
            enums::DCM_VREF_SOURCE::VDD_VBG,
            specials::DCM_VREF_SOURCE_VBG_DLL,
        ),
        (
            enums::DCM_VREF_SOURCE::VDD_VBG,
            specials::DCM_VREF_SOURCE_VBG,
        ),
        (
            enums::DCM_VREF_SOURCE::BGM_SNAP,
            specials::DCM_VREF_SOURCE_BGM_SNAP,
        ),
        (
            enums::DCM_VREF_SOURCE::BGM_ABS_SNAP,
            specials::DCM_VREF_SOURCE_BGM_ABS_SNAP,
        ),
        (
            enums::DCM_VREF_SOURCE::BGM_ABS_REF,
            specials::DCM_VREF_SOURCE_BGM_ABS_REF,
        ),
    ] {
        let mut diff_mr =
            ctx.get_diff_bel_special_special(tcid, bslot, spec, specials::DCM_MAX_RANGE);
        let mut diff_ms =
            ctx.get_diff_bel_special_special(tcid, bslot, spec, specials::DCM_MAX_SPEED);
        if spec == specials::DCM_VREF_SOURCE_VBG {
            diff_mr.apply_bitvec_diff_int(
                ctx.bel_attr_bitvec(tcid, bslot, DCM_V4::DCM_VBG_SEL),
                0x1,
                0,
            );
            diff_ms.apply_bitvec_diff_int(
                ctx.bel_attr_bitvec(tcid, bslot, DCM_V4::DCM_VBG_SEL),
                0x1,
                0,
            );
        } else if spec != specials::DCM_VREF_SOURCE_VDD {
            diff_mr.apply_bitvec_diff_int(
                ctx.bel_attr_bitvec(tcid, bslot, DCM_V4::DCM_VBG_SEL),
                0x5,
                0,
            );
            diff_ms.apply_bitvec_diff_int(
                ctx.bel_attr_bitvec(tcid, bslot, DCM_V4::DCM_VBG_SEL),
                0x9,
                0,
            );
        }
        assert_eq!(diff_mr, diff_ms);
        diffs.push((val, diff_mr));
    }
    ctx.insert_bel_attr_enum(
        tcid,
        bslot,
        DCM_V4::DCM_VREF_SOURCE,
        xlat_enum_attr_ocd(diffs, OcdMode::BitOrder),
    );

    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DLL_PHASE_SHIFT_CALIBRATION);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DLL_CONTROL_CLOCK_SPEED);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DLL_PHASE_DETECTOR_MODE);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::DLL_PHASE_DETECTOR_AUTO_RESET);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::DLL_PERIOD_LOCK_BY1);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::DLL_DESKEW_LOCK_BY1);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::DLL_PHASE_SHIFT_LOCK_BY1);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::DLL_CTL_SEL_CLKIN_DIV2);
    let bits = xlat_bit_wide_bi(
        ctx.get_diff_attr_bool_bi(tcid, bslot, DCM_V4::DUTY_CYCLE_CORRECTION, false),
        ctx.get_diff_attr_bool_bi(tcid, bslot, DCM_V4::DUTY_CYCLE_CORRECTION, true),
    );
    ctx.insert_bel_attr_bitvec(tcid, bslot, DCM_V4::DUTY_CYCLE_CORRECTION, bits);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DLL_PD_DLY_SEL);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DLL_DEAD_TIME);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DLL_LIVE_TIME);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DLL_DESKEW_MINTAP);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DLL_DESKEW_MAXTAP);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DLL_PHASE_SHIFT_LFC);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DLL_PHASE_SHIFT_HFC);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DLL_SETTLE_TIME);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DLL_SPARE);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DLL_TEST_MUX_SEL);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::FACTORY_JF);
    let mut diffs = vec![];
    for val in [
        enums::DCM_DLL_FREQUENCY_MODE::LOW,
        enums::DCM_DLL_FREQUENCY_MODE::HIGH,
        enums::DCM_DLL_FREQUENCY_MODE::HIGH_SER,
    ] {
        let mut diff = ctx.get_diff_attr_val(tcid, bslot, DCM_V4::DLL_FREQUENCY_MODE, val);
        diff.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, DCM_V4::FACTORY_JF),
            0xf0f0,
            0,
        );
        diffs.push((val, diff));
    }
    ctx.insert_bel_attr_enum(
        tcid,
        bslot,
        DCM_V4::DLL_FREQUENCY_MODE,
        xlat_enum_attr(diffs),
    );

    let diff = ctx
        .peek_diff_bel_special(tcid, bslot, specials::DCM_CLKOUT_PHASE_SHIFT_NONE)
        .clone();
    ctx.insert_bel_attr_enum(
        tcid,
        bslot,
        DCM_V4::PS_MODE,
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
        let mut dd = ctx.get_diff_bel_special_special(tcid, bslot, spec, specials::DCM_DLL);
        let item = ctx.bel_attr_enum(tcid, bslot, DCM_V4::PS_MODE);
        d.apply_enum_diff(item, enums::DCM_PS_MODE::CLKIN, enums::DCM_PS_MODE::CLKFB);
        dd.apply_enum_diff(item, enums::DCM_PS_MODE::CLKIN, enums::DCM_PS_MODE::CLKFB);
        if spec != specials::DCM_CLKOUT_PHASE_SHIFT_FIXED {
            dn.apply_enum_diff(item, enums::DCM_PS_MODE::CLKIN, enums::DCM_PS_MODE::CLKFB);
        }
        if spec != specials::DCM_CLKOUT_PHASE_SHIFT_NONE
            && spec != specials::DCM_CLKOUT_PHASE_SHIFT_DIRECT
        {
            let item = ctx.bel_attr_bit(tcid, bslot, DCM_V4::DLL_ZD2_EN);
            d.apply_bit_diff(item, true, false);
            dn.apply_bit_diff(item, true, false);
        }
        assert_eq!(d, dn);
        assert_eq!(d, dd);
        match spec {
            specials::DCM_CLKOUT_PHASE_SHIFT_NONE => d.assert_empty(),
            specials::DCM_CLKOUT_PHASE_SHIFT_FIXED
            | specials::DCM_CLKOUT_PHASE_SHIFT_VARIABLE_POSITIVE => {
                ctx.insert_bel_attr_bool(tcid, bslot, DCM_V4::PS_ENABLE, xlat_bit(d))
            }
            specials::DCM_CLKOUT_PHASE_SHIFT_VARIABLE_CENTER => {
                d.apply_bit_diff(
                    ctx.bel_attr_bit(tcid, bslot, DCM_V4::PS_ENABLE),
                    true,
                    false,
                );
                ctx.insert_bel_attr_bool(tcid, bslot, DCM_V4::PS_CENTERED, xlat_bit(d));
            }
            specials::DCM_CLKOUT_PHASE_SHIFT_DIRECT => {
                d.apply_bit_diff(
                    ctx.bel_attr_bit(tcid, bslot, DCM_V4::PS_ENABLE),
                    true,
                    false,
                );
                d.apply_enum_diff(
                    ctx.bel_attr_enum(tcid, bslot, DCM_V4::DLL_PHASE_SHIFT_CALIBRATION),
                    enums::DCM_DLL_PHASE_SHIFT_CALIBRATION::AUTO_ZD2,
                    enums::DCM_DLL_PHASE_SHIFT_CALIBRATION::AUTO_DPS,
                );
                ctx.insert_bel_attr_bool(tcid, bslot, DCM_V4::PS_DIRECT, xlat_bit(d));
            }
            _ => unreachable!(),
        }
    }

    for (attr, bits) in [
        (DCM_V4::CLKDV_PHASE_FALL, 0..2),
        (DCM_V4::CLKDV_PHASE_RISE, 2..4),
        (DCM_V4::CLKDV_COUNT_MAX, 4..8),
        (DCM_V4::CLKDV_COUNT_FALL_2, 8..12),
        (DCM_V4::CLKDV_COUNT_FALL, 12..16),
    ] {
        let bits = Vec::from_iter(bits.map(|bit| reg_bit(0x4d, bit).pos()));
        ctx.insert_bel_attr_bitvec(tcid, bslot, attr, bits);
    }
    ctx.insert_bel_attr_enum(
        tcid,
        bslot,
        DCM_V4::CLKDV_MODE,
        BelAttributeEnum {
            bits: vec![reg_bit(0x4c, 15)],
            values: EntityPartVec::from_iter([
                (enums::DCM_CLKDV_MODE::HALF, bits![0]),
                (enums::DCM_CLKDV_MODE::INT, bits![1]),
            ]),
        },
    );

    let clkdv_count_max = ctx
        .bel_attr_bitvec(tcid, bslot, DCM_V4::CLKDV_COUNT_MAX)
        .to_vec();
    let clkdv_count_fall = ctx
        .bel_attr_bitvec(tcid, bslot, DCM_V4::CLKDV_COUNT_FALL)
        .to_vec();
    let clkdv_count_fall_2 = ctx
        .bel_attr_bitvec(tcid, bslot, DCM_V4::CLKDV_COUNT_FALL_2)
        .to_vec();
    let clkdv_phase_fall = ctx
        .bel_attr_bitvec(tcid, bslot, DCM_V4::CLKDV_PHASE_FALL)
        .to_vec();
    let clkdv_mode = ctx.bel_attr_enum(tcid, bslot, DCM_V4::CLKDV_MODE).clone();
    for i in 2..=16 {
        let mut diff =
            ctx.get_diff_bel_special_u32(tcid, bslot, specials::DCM_CLKDV_DIVIDE_INT, i as u32);
        diff.apply_bitvec_diff_int(&clkdv_count_max, i - 1, 1);
        diff.apply_bitvec_diff_int(&clkdv_count_fall, (i - 1) / 2, 0);
        diff.apply_bitvec_diff_int(&clkdv_phase_fall, (i % 2) * 2, 0);
        diff.assert_empty();
    }
    for i in 1..=7 {
        let mut diff = ctx.get_diff_bel_special_u32(
            tcid,
            bslot,
            specials::DCM_CLKDV_DIVIDE_HALF_LOW,
            i as u32,
        );
        diff.apply_enum_diff(
            &clkdv_mode,
            enums::DCM_CLKDV_MODE::HALF,
            enums::DCM_CLKDV_MODE::INT,
        );
        diff.apply_bitvec_diff_int(&clkdv_count_max, 2 * i, 1);
        diff.apply_bitvec_diff_int(&clkdv_count_fall, i / 2, 0);
        diff.apply_bitvec_diff_int(&clkdv_count_fall_2, 3 * i / 2 + 1, 0);
        diff.apply_bitvec_diff_int(&clkdv_phase_fall, (i % 2) * 2 + 1, 0);
        diff.assert_empty();
        let mut diff = ctx.get_diff_bel_special_u32(
            tcid,
            bslot,
            specials::DCM_CLKDV_DIVIDE_HALF_HIGH,
            i as u32,
        );
        diff.apply_enum_diff(
            &clkdv_mode,
            enums::DCM_CLKDV_MODE::HALF,
            enums::DCM_CLKDV_MODE::INT,
        );
        diff.apply_bitvec_diff_int(&clkdv_count_max, 2 * i, 1);
        diff.apply_bitvec_diff_int(&clkdv_count_fall, (i - 1) / 2, 0);
        diff.apply_bitvec_diff_int(&clkdv_count_fall_2, (3 * i).div_ceil(2), 0);
        diff.apply_bitvec_diff_int(&clkdv_phase_fall, (i % 2) * 2, 0);
        diff.assert_empty();
    }

    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DFS_FREQUENCY_MODE);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::DFS_EN_RELRST);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::DFS_NON_STOP);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::DFS_EXTEND_RUN_TIME);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::DFS_EXTEND_HALT_TIME);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::DFS_EXTEND_FLUSH_TIME);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::DFS_EARLY_LOCK);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::DFS_SKIP_FINE);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DFS_COARSE_SEL);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DFS_TP_SEL);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DFS_FINE_SEL);
    ctx.collect_bel_attr_default_ocd(
        tcid,
        bslot,
        DCM_V4::DFS_AVE_FREQ_GAIN,
        enums::DCM_DFS_AVE_FREQ_GAIN::NONE,
        OcdMode::BitOrder,
    );
    ctx.collect_bel_attr_sparse(tcid, bslot, DCM_V4::DFS_AVE_FREQ_SAMPLE_INTERVAL, 1..8);
    ctx.collect_bel_attr_sparse(tcid, bslot, DCM_V4::DFS_AVE_FREQ_ADJ_INTERVAL, 1..16);
    ctx.collect_bel_attr_bi(tcid, bslot, DCM_V4::DFS_TRACKMODE);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DFS_COIN_WINDOW);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DFS_HARDSYNC);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::DFS_SPARE);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::CLKFX_DIVIDE);
    ctx.collect_bel_attr_sparse(tcid, bslot, DCM_V4::CLKFX_MULTIPLY, 1..32);

    let mut diffs = vec![(
        enums::DCM_DFS_OSCILLATOR_MODE::PHASE_FREQ_LOCK,
        Diff::default(),
    )];
    for val in [
        enums::DCM_DFS_OSCILLATOR_MODE::FREQ_LOCK,
        enums::DCM_DFS_OSCILLATOR_MODE::AVE_FREQ_LOCK,
    ] {
        let mut diff = ctx.get_diff_attr_val(tcid, bslot, DCM_V4::DFS_OSCILLATOR_MODE, val);
        diff.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, DCM_V4::DFS_EARLY_LOCK),
            true,
            false,
        );
        diff.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, DCM_V4::DFS_HARDSYNC), 3, 0);
        diffs.push((val, diff));
    }
    ctx.insert_bel_attr_enum(
        tcid,
        bslot,
        DCM_V4::DFS_OSCILLATOR_MODE,
        xlat_enum_attr(diffs),
    );
    let item = xlat_bit(ctx.get_diff_attr_val(
        tcid,
        bslot,
        DCM_V4::DFS_OSCILLATOR_MODE,
        enums::DCM_DFS_OSCILLATOR_MODE::PHASE_FREQ_LOCK,
    ));
    ctx.insert_bel_attr_bool(tcid, bslot, DCM_V4::DFS_FEEDBACK, item);

    ctx.collect_bel_attr(tcid, bslot, DCM_V4::CLKIN_IOB);
    let mut diff = ctx.get_diff_attr_bool(tcid, bslot, DCM_V4::CLKFB_IOB);
    diff.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, DCM_V4::DCM_EXT_FB_EN),
        true,
        false,
    );
    ctx.insert_bel_attr_bool(tcid, bslot, DCM_V4::CLKFB_IOB, xlat_bit(diff));
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::CLKIN_ENABLE);
    ctx.collect_bel_attr(tcid, bslot, DCM_V4::CLKFB_ENABLE);

    let dn = ctx.get_diff_attr_val(
        tcid,
        bslot,
        DCM_V4::CLK_FEEDBACK,
        enums::DCM_CLK_FEEDBACK::NONE,
    );
    assert_eq!(
        dn,
        ctx.get_diff_attr_special_val(
            tcid,
            bslot,
            DCM_V4::CLK_FEEDBACK,
            specials::DCM_CLKFB,
            enums::DCM_CLK_FEEDBACK::NONE
        )
    );
    let d1 = ctx.get_diff_attr_val(
        tcid,
        bslot,
        DCM_V4::CLK_FEEDBACK,
        enums::DCM_CLK_FEEDBACK::_1X,
    );
    let df = ctx
        .get_diff_attr_special_val(
            tcid,
            bslot,
            DCM_V4::CLK_FEEDBACK,
            specials::DCM_CLKFB,
            enums::DCM_CLK_FEEDBACK::_1X,
        )
        .combine(&!&d1);
    let d2 = ctx.get_diff_attr_val(
        tcid,
        bslot,
        DCM_V4::CLK_FEEDBACK,
        enums::DCM_CLK_FEEDBACK::_2X,
    );
    assert_eq!(
        df,
        ctx.get_diff_attr_special_val(
            tcid,
            bslot,
            DCM_V4::CLK_FEEDBACK,
            specials::DCM_CLKFB,
            enums::DCM_CLK_FEEDBACK::_2X
        )
        .combine(&!&d2)
    );
    ctx.insert_bel_attr_bool(tcid, bslot, DCM_V4::CLKFB_FEEDBACK, xlat_bit(df));
    ctx.insert_bel_attr_enum(
        tcid,
        bslot,
        DCM_V4::CLK_FEEDBACK,
        xlat_enum_attr(vec![
            (enums::DCM_CLK_FEEDBACK::_1X, d1),
            (enums::DCM_CLK_FEEDBACK::_2X, d2),
            (enums::DCM_CLK_FEEDBACK::NONE, dn),
        ]),
    );

    present.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, DCM_V4::DCM_VBG_SEL), 1, 0);
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, DCM_V4::CLKDV_COUNT_MAX),
        1,
        0,
    );
    present.apply_enum_diff(
        ctx.bel_attr_enum(tcid, bslot, DCM_V4::CLKDV_MODE),
        enums::DCM_CLKDV_MODE::INT,
        enums::DCM_CLKDV_MODE::HALF,
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, DCM_V4::OUT_CLK90_ENABLE),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, DCM_V4::OUT_CLK180_ENABLE),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, DCM_V4::OUT_CLK270_ENABLE),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, DCM_V4::OUT_CLK2X_ENABLE),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, DCM_V4::OUT_CLK2X180_ENABLE),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, DCM_V4::OUT_CLKDV_ENABLE),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, DCM_V4::OUT_CLKFX180_ENABLE),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, DCM_V4::OUT_CLKFX_ENABLE),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, DCM_V4::OUT_CONCUR_ENABLE),
        true,
        false,
    );

    ctx.insert_bel_attr_bool(tcid, bslot, DCM_V4::UNK_ALWAYS_SET, xlat_bit(present));

    for mux in ctx.edev.db_index.tile_classes[tcid].muxes.values() {
        if wires::IMUX_SPEC.contains(mux.dst.wire) {
            let mut diffs = vec![];
            for &src in mux.src.keys() {
                let diff = ctx.get_diff_routing(tcid, mux.dst, src);
                let mut diff_test =
                    ctx.get_diff_routing_pair_special(tcid, mux.dst, src, specials::TEST);
                if wires::IMUX_CLK_OPTINV.contains(src.wire) {
                    let item = ctx.item_int_inv_raw(&[tcls::INT; 4], src.tw);
                    diff_test.apply_bit_diff(item, false, true);
                }
                assert_eq!(diff, diff_test);
                diffs.push((Some(src), diff));
            }
            ctx.insert_mux(tcid, mux.dst, xlat_enum_raw(diffs, OcdMode::Mux));
        }
    }
}
