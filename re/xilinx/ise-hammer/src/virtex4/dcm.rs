use std::collections::BTreeMap;

use prjcombine_re_collector::{
    diff::{Diff, OcdMode},
    legacy::{xlat_bit_legacy, xlat_bitvec_legacy, xlat_enum_legacy, xlat_enum_legacy_ocd},
};
use prjcombine_re_hammer::Session;
use prjcombine_types::{
    bits,
    bsdata::{TileBit, TileItem, TileItemKind},
};
use prjcombine_virtex4::defs::{self, bslots, virtex4::tcls};

use crate::{
    backend::{IseBackend, PinFromKind},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, "DCM");
    let mut bctx = ctx.bel(defs::bslots::DCM[0]);
    let mode = "DCM_ADV";

    bctx.build()
        .test_manual_legacy("PRESENT", "1")
        .mode(mode)
        .commit();
    for pin in [
        "DEN",
        "DWE",
        "DI0",
        "DI1",
        "DI2",
        "DI3",
        "DI4",
        "DI5",
        "DI6",
        "DI7",
        "DI8",
        "DI9",
        "DI10",
        "DI11",
        "DI12",
        "DI13",
        "DI14",
        "DI15",
        "DADDR0",
        "DADDR1",
        "DADDR2",
        "DADDR3",
        "DADDR4",
        "DADDR5",
        "DADDR6",
        // DCLK?
        "RST",
        // PSCLK?
        "PSEN",
        "PSINCDEC",
        "CTLMODE",
        "CTLSEL0",
        "CTLSEL1",
        "CTLSEL2",
        "CTLOSC1",
        "CTLOSC2",
        "CTLGO",
        "FREEZE_DLL",
        "FREEZE_DFS",
    ] {
        bctx.mode(mode).test_inv(pin);
    }

    for pin in [
        "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX", "CLKFX180",
        "CONCUR",
    ] {
        bctx.mode(mode)
            .mutex("PIN", pin)
            .test_manual_legacy(pin, "1")
            .pin(pin)
            .commit();
    }
    bctx.mode(mode)
        .pin_from("CLKFB", PinFromKind::Bufg)
        .test_manual_legacy("CLKFB_ENABLE", "1")
        .pin("CLKFB")
        .commit();
    bctx.mode(mode)
        .pin_from("CLKIN", PinFromKind::Bufg)
        .test_manual_legacy("CLKIN_ENABLE", "1")
        .pin("CLKIN")
        .commit();
    bctx.mode(mode)
        .global_mutex("DCM", "USE")
        .pin("CLKIN")
        .pin("CLKFB")
        .pin_from("CLKFB", PinFromKind::Bufg)
        .test_manual_legacy("CLKIN_IOB", "1")
        .pin_from("CLKIN", PinFromKind::Bufg, PinFromKind::Iob)
        .commit();
    bctx.mode(mode)
        .global_mutex("DCM", "USE")
        .pin("CLKIN")
        .pin("CLKFB")
        .pin_from("CLKIN", PinFromKind::Bufg)
        .test_manual_legacy("CLKFB_IOB", "1")
        .pin_from("CLKFB", PinFromKind::Bufg, PinFromKind::Iob)
        .commit();

    bctx.mode(mode).test_multi_attr_dec("BGM_VLDLY", 3);
    bctx.mode(mode).test_multi_attr_dec("BGM_LDLY", 3);
    bctx.mode(mode).test_multi_attr_dec("BGM_SDLY", 3);
    bctx.mode(mode).test_multi_attr_dec("BGM_VSDLY", 3);
    bctx.mode(mode).test_multi_attr_dec("BGM_SAMPLE_LEN", 3);
    bctx.mode(mode).test_enum_legacy(
        "BGM_MODE",
        &["BG_SNAPSHOT", "ABS_FREQ_SNAPSHOT", "ABS_FREQ_REF"],
    );
    bctx.mode(mode)
        .test_enum_legacy("BGM_CONFIG_REF_SEL", &["DCLK", "CLKIN"]);
    bctx.mode(mode).test_enum_legacy(
        "BGM_VADJ",
        &[
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
        ],
    );
    bctx.mode(mode)
        .test_multi_attr_dec_delta("BGM_MULTIPLY", 6, 1);
    bctx.mode(mode)
        .test_multi_attr_dec_delta("BGM_DIVIDE", 6, 1);

    bctx.mode(mode)
        .test_enum_legacy("DCM_CLKDV_CLKFX_ALIGNMENT", &["FALSE", "TRUE"]);
    bctx.mode(mode)
        .test_enum_legacy("DCM_LOCK_HIGH", &["FALSE", "TRUE"]);
    bctx.mode(mode)
        .test_enum_legacy("DCM_VREG_ENABLE", &["FALSE", "TRUE"]);
    bctx.mode(mode)
        .no_pin("CLKFB")
        .test_enum_legacy("DCM_EXT_FB_EN", &["FALSE", "TRUE"]);
    bctx.mode(mode)
        .test_enum_legacy("DCM_UNUSED_TAPS_POWERDOWN", &["FALSE", "TRUE"]);
    bctx.mode(mode)
        .test_enum_legacy("DCM_PERFORMANCE_MODE", &["MAX_SPEED", "MAX_RANGE"]);
    for val in [
        "VDD",
        "VBG_DLL",
        "VBG",
        "BGM_SNAP",
        "BGM_ABS_SNAP",
        "BGM_ABS_REF",
    ] {
        bctx.mode(mode)
            .attr("DCM_PERFORMANCE_MODE", "MAX_RANGE")
            .test_manual_legacy("DCM_VREF_SOURCE.MAX_RANGE", val)
            .attr_diff("DCM_VREF_SOURCE", "VDD", val)
            .commit();
        bctx.mode(mode)
            .attr("DCM_PERFORMANCE_MODE", "MAX_SPEED")
            .test_manual_legacy("DCM_VREF_SOURCE.MAX_SPEED", val)
            .attr_diff("DCM_VREF_SOURCE", "VDD", val)
            .commit();
    }
    bctx.mode(mode)
        .global("GTS_CYCLE", "1")
        .global("DONE_CYCLE", "1")
        .global("LCK_CYCLE", "NOWAIT")
        .test_enum_legacy("STARTUP_WAIT", &["FALSE", "TRUE"]);
    bctx.mode(mode)
        .test_enum_legacy("CLKIN_DIVIDE_BY_2", &["FALSE", "TRUE"]);
    bctx.mode(mode).test_enum_legacy("PMCD_SYNC", &["FALSE", "TRUE"]);
    bctx.mode(mode)
        .no_pin("CLKFB")
        .test_enum_legacy("CLK_FEEDBACK", &["NONE", "1X", "2X"]);
    for val in ["NONE", "1X", "2X"] {
        bctx.mode(mode)
            .pin("CLKFB")
            .pin_from("CLKFB", PinFromKind::Bufg)
            .test_manual_legacy("CLK_FEEDBACK.CLKFB", val)
            .attr("CLK_FEEDBACK", val)
            .commit();
    }
    bctx.mode(mode)
        .attr("PHASE_SHIFT", "1")
        .no_pin("CLK0")
        .no_pin("CLK90")
        .no_pin("CLK180")
        .no_pin("CLK270")
        .no_pin("CLK2X")
        .no_pin("CLK2X180")
        .no_pin("CLKDV")
        .test_enum_legacy(
            "CLKOUT_PHASE_SHIFT",
            &[
                "NONE",
                "FIXED",
                "VARIABLE_POSITIVE",
                "VARIABLE_CENTER",
                "DIRECT",
            ],
        );
    for val in [
        "NONE",
        "FIXED",
        "VARIABLE_POSITIVE",
        "VARIABLE_CENTER",
        "DIRECT",
    ] {
        bctx.mode(mode)
            .attr("PHASE_SHIFT", "-1")
            .no_pin("CLK0")
            .no_pin("CLK90")
            .no_pin("CLK180")
            .no_pin("CLK270")
            .no_pin("CLK2X")
            .no_pin("CLK2X180")
            .no_pin("CLKDV")
            .test_manual_legacy("CLKOUT_PHASE_SHIFT.NEG", val)
            .attr("CLKOUT_PHASE_SHIFT", val)
            .commit();
    }
    for val in [
        "NONE",
        "FIXED",
        "VARIABLE_POSITIVE",
        "VARIABLE_CENTER",
        "DIRECT",
    ] {
        bctx.mode(mode)
            .mutex("PIN", "NONE")
            .attr("PHASE_SHIFT", "1")
            .pin("CLK0")
            .test_manual_legacy("CLKOUT_PHASE_SHIFT.DLL", val)
            .attr("CLKOUT_PHASE_SHIFT", val)
            .commit();
    }
    bctx.mode(mode).test_multi_attr_dec("DESKEW_ADJUST", 5);
    bctx.mode(mode)
        .test_multi_attr_bin("DCM_PULSE_WIDTH_CORRECTION_LOW", 5);
    bctx.mode(mode)
        .test_multi_attr_bin("DCM_PULSE_WIDTH_CORRECTION_HIGH", 5);
    bctx.mode(mode).test_multi_attr_bin("DCM_VBG_PD", 2);
    bctx.mode(mode)
        .attr("DCM_VREF_SOURCE", "VDD")
        .test_multi_attr_bin("DCM_VBG_SEL", 4);
    bctx.mode(mode)
        .test_multi_attr_bin("DCM_VREG_PHASE_MARGIN", 3);
    bctx.mode(mode).test_multi_attr_dec("PHASE_SHIFT", 10);
    bctx.mode(mode)
        .attr("CLKOUT_PHASE_SHIFT", "NONE")
        .test_manual_legacy("PHASE_SHIFT", "-1")
        .attr("PHASE_SHIFT", "-1")
        .commit();

    bctx.mode(mode)
        .test_enum_legacy("DLL_FREQUENCY_MODE", &["LOW", "HIGH", "HIGH_SER"]);
    bctx.mode(mode)
        .attr("CLKOUT_PHASE_SHIFT", "NONE")
        .test_enum_legacy(
            "DLL_PHASE_SHIFT_CALIBRATION",
            &["AUTO_DPS", "CONFIG", "MASK", "AUTO_ZD2"],
        );
    bctx.mode(mode)
        .test_enum_legacy("DLL_CONTROL_CLOCK_SPEED", &["QUARTER", "HALF"]);
    bctx.mode(mode)
        .test_enum_legacy("DLL_PHASE_DETECTOR_MODE", &["LEVEL", "ENHANCED"]);
    bctx.mode(mode)
        .test_enum_legacy("DLL_PHASE_DETECTOR_AUTO_RESET", &["FALSE", "TRUE"]);
    bctx.mode(mode)
        .test_enum_legacy("DLL_PERIOD_LOCK_BY1", &["FALSE", "TRUE"]);
    bctx.mode(mode)
        .test_enum_legacy("DLL_DESKEW_LOCK_BY1", &["FALSE", "TRUE"]);
    bctx.mode(mode)
        .test_enum_legacy("DLL_PHASE_SHIFT_LOCK_BY1", &["FALSE", "TRUE"]);
    bctx.mode(mode)
        .test_enum_legacy("DLL_CTL_SEL_CLKIN_DIV2", &["FALSE", "TRUE"]);
    bctx.mode(mode)
        .test_enum_legacy("DUTY_CYCLE_CORRECTION", &["FALSE", "TRUE"]);
    bctx.mode(mode).test_multi_attr_dec("DLL_PD_DLY_SEL", 3);
    bctx.mode(mode).test_multi_attr_dec("DLL_DEAD_TIME", 8);
    bctx.mode(mode).test_multi_attr_dec("DLL_LIVE_TIME", 8);
    bctx.mode(mode).test_multi_attr_dec("DLL_DESKEW_MINTAP", 8);
    bctx.mode(mode).test_multi_attr_dec("DLL_DESKEW_MAXTAP", 8);
    bctx.mode(mode)
        .test_multi_attr_dec("DLL_PHASE_SHIFT_LFC", 8);
    bctx.mode(mode)
        .test_multi_attr_dec("DLL_PHASE_SHIFT_HFC", 8);
    bctx.mode(mode).test_multi_attr_dec("DLL_SETTLE_TIME", 8);
    bctx.mode(mode).test_multi_attr_bin("DLL_SPARE", 16);
    bctx.mode(mode).test_multi_attr_bin("DLL_TEST_MUX_SEL", 2);
    bctx.mode(mode)
        .attr("DLL_FREQUENCY_MODE", "")
        .test_multi_attr_hex_legacy("FACTORY_JF", 16);
    bctx.mode(mode).test_enum_legacy(
        "CLKDV_DIVIDE",
        &[
            "2.0", "3.0", "4.0", "5.0", "6.0", "7.0", "8.0", "9.0", "10.0", "11.0", "12.0", "13.0",
            "14.0", "15.0", "16.0",
        ],
    );
    for dll_mode in ["LOW", "HIGH", "HIGH_SER"] {
        for val in ["1.5", "2.5", "3.5", "4.5", "5.5", "6.5", "7.5"] {
            bctx.mode(mode)
                .global_mutex("DCM", "USE")
                .attr("DLL_FREQUENCY_MODE", dll_mode)
                .test_manual_legacy("CLKDV_DIVIDE", format!("{val}.{dll_mode}"))
                .attr("CLKDV_DIVIDE", val)
                .commit();
        }
    }

    bctx.mode(mode)
        .test_enum_legacy("DFS_FREQUENCY_MODE", &["LOW", "HIGH"]);
    bctx.mode(mode)
        .test_enum_legacy("DFS_EN_RELRST", &["FALSE", "TRUE"]);
    bctx.mode(mode)
        .test_enum_legacy("DFS_NON_STOP", &["FALSE", "TRUE"]);
    bctx.mode(mode)
        .test_enum_legacy("DFS_EXTEND_RUN_TIME", &["FALSE", "TRUE"]);
    bctx.mode(mode)
        .test_enum_legacy("DFS_EXTEND_HALT_TIME", &["FALSE", "TRUE"]);
    bctx.mode(mode)
        .test_enum_legacy("DFS_EXTEND_FLUSH_TIME", &["FALSE", "TRUE"]);
    bctx.mode(mode)
        .attr("DFS_OSCILLATOR_MODE", "")
        .test_enum_legacy("DFS_EARLY_LOCK", &["FALSE", "TRUE"]);
    bctx.mode(mode)
        .test_enum_legacy("DFS_SKIP_FINE", &["FALSE", "TRUE"]);
    bctx.mode(mode)
        .test_enum_legacy("DFS_COARSE_SEL", &["LEVEL", "LEGACY"]);
    bctx.mode(mode)
        .test_enum_legacy("DFS_TP_SEL", &["LEVEL", "LEGACY"]);
    bctx.mode(mode)
        .test_enum_legacy("DFS_FINE_SEL", &["LEVEL", "LEGACY"]);
    bctx.mode(mode).test_enum_legacy(
        "DFS_AVE_FREQ_GAIN",
        &["0.125", "0.25", "0.5", "1.0", "2.0", "4.0", "8.0"],
    );
    bctx.mode(mode).test_enum_legacy(
        "DFS_AVE_FREQ_SAMPLE_INTERVAL",
        &["1", "2", "3", "4", "5", "6", "7"],
    );
    bctx.mode(mode).test_enum_legacy(
        "DFS_AVE_FREQ_ADJ_INTERVAL",
        &[
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
        ],
    );
    bctx.mode(mode).test_enum_legacy("DFS_TRACKMODE", &["0", "1"]);
    bctx.mode(mode)
        .mutex("PIN", "NONE")
        .pin("CLK0")
        .pin("CLKFX")
        .test_enum_legacy(
            "DFS_OSCILLATOR_MODE",
            &["PHASE_FREQ_LOCK", "FREQ_LOCK", "AVE_FREQ_LOCK"],
        );
    bctx.mode(mode).test_multi_attr_bin("DFS_COIN_WINDOW", 2);
    bctx.mode(mode)
        .attr("DFS_OSCILLATOR_MODE", "")
        .test_multi_attr_bin("DFS_HARDSYNC", 2);
    bctx.mode(mode).test_multi_attr_bin("DFS_SPARE", 16);
    bctx.mode(mode)
        .test_multi_attr_dec_delta("CLKFX_DIVIDE", 5, 1);
    for val in 2..=32 {
        bctx.mode(mode)
            .test_manual_legacy("CLKFX_MULTIPLY", format!("{val}"))
            .attr("CLKFX_MULTIPLY", format!("{val}"))
            .commit();
    }

    for (pin, opin) in [("CLKIN", "CLKFB"), ("CLKFB", "CLKIN")] {
        for rpin in [pin.to_string(), format!("{pin}_TEST")] {
            for i in 0..8 {
                bctx.mode(mode)
                    .global_mutex("HCLK_DCM", "USE")
                    .pin(pin)
                    .pin(opin)
                    .mutex(format!("{pin}_OUT"), &rpin)
                    .mutex(format!("{pin}_IN"), format!("HCLK{i}"))
                    .mutex(format!("{opin}_OUT"), "HOLD")
                    .mutex(format!("{opin}_IN"), format!("HCLK{i}"))
                    .pip(opin, format!("HCLK{i}"))
                    .test_manual_legacy(&rpin, format!("HCLK{i}"))
                    .pip(&rpin, format!("HCLK{i}"))
                    .commit();
            }
            for i in 0..16 {
                bctx.mode(mode)
                    .global_mutex("HCLK_DCM", "USE")
                    .pin(pin)
                    .pin(opin)
                    .mutex(format!("{pin}_OUT"), &rpin)
                    .mutex(format!("{pin}_IN"), format!("GIOB{i}"))
                    .mutex(format!("{opin}_OUT"), "HOLD")
                    .mutex(format!("{opin}_IN"), format!("GIOB{i}"))
                    .pip(opin, format!("GIOB{i}"))
                    .test_manual_legacy(&rpin, format!("GIOB{i}"))
                    .pip(&rpin, format!("GIOB{i}"))
                    .commit();
            }
            for i in 0..4 {
                bctx.mode(mode)
                    .global_mutex("HCLK_DCM", "USE")
                    .pin(pin)
                    .pin(opin)
                    .mutex(format!("{pin}_OUT"), &rpin)
                    .mutex(format!("{pin}_IN"), format!("MGT{i}"))
                    .mutex(format!("{opin}_OUT"), "HOLD")
                    .mutex(format!("{opin}_IN"), format!("MGT{i}"))
                    .pip(opin, format!("MGT{i}"))
                    .test_manual_legacy(&rpin, format!("MGT{i}"))
                    .pip(&rpin, format!("MGT{i}"))
                    .commit();
            }
            for i in 0..2 {
                bctx.mode(mode)
                    .pin(pin)
                    .pin(opin)
                    .mutex(format!("{pin}_OUT"), &rpin)
                    .mutex(format!("{pin}_IN"), format!("BUSOUT{i}"))
                    .mutex(format!("{opin}_OUT"), "HOLD")
                    .mutex(format!("{opin}_IN"), format!("BUSOUT{i}"))
                    .pip(opin, format!("BUSOUT{i}"))
                    .test_manual_legacy(&rpin, format!("BUSOUT{i}"))
                    .pip(&rpin, format!("BUSOUT{i}"))
                    .commit();
            }
            for i in 0..4 {
                bctx.mode(mode)
                    .pin(pin)
                    .mutex(format!("{pin}_OUT"), &rpin)
                    .mutex(format!("{pin}_IN"), format!("CKINT{i}"))
                    .mutex(format!("CKINT{i}"), &rpin)
                    .test_manual_legacy(&rpin, format!("CKINT{i}"))
                    .pip(&rpin, format!("CKINT{i}"))
                    .commit();
            }
        }
    }

    for i in 0..24 {
        bctx.build()
            .mutex(format!("BUSOUT{i}"), format!("BUSIN{i}"))
            .test_manual_legacy(format!("MUX.BUSOUT{i}"), "PASS")
            .pip(format!("BUSOUT{i}"), format!("BUSIN{i}"))
            .commit();
        for inp in [
            "CLK0_BUF",
            "CLK90_BUF",
            "CLK180_BUF",
            "CLK270_BUF",
            "CLK2X_BUF",
            "CLK2X180_BUF",
            "CLKDV_BUF",
            "CLKFX_BUF",
            "CLKFX180_BUF",
            "CONCUR_BUF",
            "LOCKED_BUF",
            "CLK_IN0",
        ] {
            let sname = inp.strip_suffix("_BUF").unwrap_or(inp);
            bctx.build()
                .mutex(format!("BUSOUT{i}"), inp)
                .test_manual_legacy(format!("MUX.BUSOUT{i}"), sname)
                .pip(format!("BUSOUT{i}"), inp)
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::DCM;
    let bslot = bslots::DCM[0];
    let tile = "DCM";
    let bel = "DCM[0]";

    let mut present = ctx.get_diff_legacy(tile, bel, "PRESENT", "1");

    fn reg_bit(addr: usize, bit: usize) -> TileBit {
        TileBit::new(
            (addr >> 2) & 3,
            20 - (addr >> 4 & 1),
            bit + 1 + (addr & 3) * 20,
        )
    }

    for addr in 0x40..0x60 {
        let reg_mask_bit = reg_bit(addr, 17);
        assert_eq!(present.bits.remove(&reg_mask_bit), Some(true));
        ctx.insert(
            tile,
            bel,
            format!("DRP{addr:02X}_MASK"),
            TileItem::from_bit_inv(reg_mask_bit, false),
        );
        ctx.insert(
            tile,
            bel,
            format!("DRP{addr:02X}"),
            TileItem::from_bitvec_inv(Vec::from_iter((0..16).map(|bit| reg_bit(addr, bit))), false),
        );
    }

    for pin in ["RST", "CTLMODE", "FREEZE_DLL", "FREEZE_DFS", "DEN", "DWE"] {
        ctx.collect_int_inv_legacy(&[tcls::INT; 4], tcid, bslot, pin, false);
    }

    for pin in [
        "DI0", "DI1", "DI2", "DI3", "DI4", "DI5", "DI6", "DI7", "DI8", "DI9", "DI10", "DI11",
        "DI12", "DI13", "DI14", "DI15", "DADDR0", "DADDR1", "DADDR2", "DADDR3", "DADDR4", "DADDR5",
        "DADDR6", "PSEN", "PSINCDEC", "CTLSEL0", "CTLSEL1", "CTLSEL2", "CTLOSC1", "CTLOSC2",
        "CTLGO",
    ] {
        ctx.collect_inv(tile, bel, pin);
    }

    let diff = ctx.get_diff_legacy(tile, bel, "CLK2X", "1");
    for pin in ["CLK2X180", "CLKDV", "CLK90", "CLK180", "CLK270"] {
        assert_eq!(diff, ctx.get_diff_legacy(tile, bel, pin, "1"));
    }
    let diff_0 = ctx.get_diff_legacy(tile, bel, "CLK0", "1");
    let diff_0 = diff_0.combine(&!&diff);
    ctx.insert(tile, bel, "ENABLE.CLK0", xlat_bit_legacy(diff_0));
    // ???
    ctx.insert(
        tile,
        bel,
        "ENABLE.CLK90",
        TileItem::from_bit_inv(reg_bit(0x4e, 1), false),
    );
    ctx.insert(
        tile,
        bel,
        "ENABLE.CLK180",
        TileItem::from_bit_inv(reg_bit(0x4e, 2), false),
    );
    ctx.insert(
        tile,
        bel,
        "ENABLE.CLK270",
        TileItem::from_bit_inv(reg_bit(0x4e, 3), false),
    );
    ctx.insert(
        tile,
        bel,
        "ENABLE.CLK2X",
        TileItem::from_bit_inv(reg_bit(0x4e, 4), false),
    );
    ctx.insert(
        tile,
        bel,
        "ENABLE.CLK2X180",
        TileItem::from_bit_inv(reg_bit(0x4e, 5), false),
    );
    ctx.insert(
        tile,
        bel,
        "ENABLE.CLKDV",
        TileItem::from_bit_inv(reg_bit(0x4e, 6), false),
    );
    ctx.insert(
        tile,
        bel,
        "ENABLE.CLKFX180",
        TileItem::from_bit_inv(reg_bit(0x51, 8), false),
    );
    ctx.insert(
        tile,
        bel,
        "ENABLE.CLKFX",
        TileItem::from_bit_inv(reg_bit(0x51, 9), false),
    );
    ctx.insert(
        tile,
        bel,
        "ENABLE.CONCUR",
        TileItem::from_bit_inv(reg_bit(0x51, 10), false),
    );

    ctx.insert(tile, bel, "DLL_ZD2_EN", xlat_bit_legacy(diff));
    let diff = ctx.get_diff_legacy(tile, bel, "CLKFX", "1");
    for pin in ["CLKFX180", "CONCUR"] {
        assert_eq!(diff, ctx.get_diff_legacy(tile, bel, pin, "1"));
    }
    ctx.insert(tile, bel, "DFS_ENABLE", xlat_bit_legacy(diff));

    ctx.collect_bitvec_legacy(tile, bel, "BGM_VLDLY", "");
    ctx.collect_bitvec_legacy(tile, bel, "BGM_LDLY", "");
    ctx.collect_bitvec_legacy(tile, bel, "BGM_SDLY", "");
    ctx.collect_bitvec_legacy(tile, bel, "BGM_VSDLY", "");
    ctx.collect_bitvec_legacy(tile, bel, "BGM_SAMPLE_LEN", "");
    ctx.collect_enum_legacy_ocd(
        tile,
        bel,
        "BGM_MODE",
        &["BG_SNAPSHOT", "ABS_FREQ_SNAPSHOT", "ABS_FREQ_REF"],
        OcdMode::BitOrder,
    );
    ctx.collect_enum_legacy(tile, bel, "BGM_CONFIG_REF_SEL", &["DCLK", "CLKIN"]);
    ctx.collect_enum_legacy_int(tile, bel, "BGM_VADJ", 1..16, 0);
    ctx.collect_bitvec_legacy(tile, bel, "BGM_MULTIPLY", "");
    ctx.collect_bitvec_legacy(tile, bel, "BGM_DIVIDE", "");

    ctx.collect_bit_bi_legacy(tile, bel, "DCM_CLKDV_CLKFX_ALIGNMENT", "FALSE", "TRUE");
    ctx.collect_bit_bi_legacy(tile, bel, "DCM_LOCK_HIGH", "FALSE", "TRUE");
    ctx.collect_bit_bi_legacy(tile, bel, "DCM_VREG_ENABLE", "FALSE", "TRUE");
    ctx.collect_bit_bi_legacy(tile, bel, "DCM_EXT_FB_EN", "FALSE", "TRUE");
    ctx.collect_bit_bi_legacy(tile, bel, "DCM_UNUSED_TAPS_POWERDOWN", "FALSE", "TRUE");
    ctx.collect_enum_legacy(
        tile,
        bel,
        "DCM_PERFORMANCE_MODE",
        &["MAX_SPEED", "MAX_RANGE"],
    );
    ctx.collect_bit_bi_legacy(tile, bel, "STARTUP_WAIT", "FALSE", "TRUE");
    ctx.collect_bit_bi_legacy(tile, bel, "CLKIN_DIVIDE_BY_2", "FALSE", "TRUE");
    ctx.collect_bit_bi_legacy(tile, bel, "PMCD_SYNC", "FALSE", "TRUE");
    ctx.collect_bitvec_legacy(tile, bel, "DESKEW_ADJUST", "");
    ctx.collect_bitvec_legacy(tile, bel, "DCM_PULSE_WIDTH_CORRECTION_LOW", "");
    ctx.collect_bitvec_legacy(tile, bel, "DCM_PULSE_WIDTH_CORRECTION_HIGH", "");
    ctx.collect_bitvec_legacy(tile, bel, "DCM_VBG_PD", "");
    ctx.collect_bitvec_legacy(tile, bel, "DCM_VBG_SEL", "");
    ctx.collect_bitvec_legacy(tile, bel, "DCM_VREG_PHASE_MARGIN", "");
    ctx.collect_bitvec_legacy(tile, bel, "PHASE_SHIFT", "");
    let mut diff = ctx.get_diff_legacy(tile, bel, "PHASE_SHIFT", "-1");
    diff.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "PHASE_SHIFT"), 1, 0);
    ctx.insert(tile, bel, "PHASE_SHIFT_NEGATIVE", xlat_bit_legacy(diff));

    let mut diffs = vec![];
    for val in [
        "VDD",
        "VBG_DLL",
        "VBG",
        "BGM_SNAP",
        "BGM_ABS_SNAP",
        "BGM_ABS_REF",
    ] {
        let mut diff_mr = ctx.get_diff_legacy(tile, bel, "DCM_VREF_SOURCE.MAX_RANGE", val);
        let mut diff_ms = ctx.get_diff_legacy(tile, bel, "DCM_VREF_SOURCE.MAX_SPEED", val);
        if val == "VBG" {
            diff_mr.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "DCM_VBG_SEL"), 0x1, 0);
            diff_ms.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "DCM_VBG_SEL"), 0x1, 0);
        } else if val != "VDD" {
            diff_mr.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "DCM_VBG_SEL"), 0x5, 0);
            diff_ms.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "DCM_VBG_SEL"), 0x9, 0);
        }
        assert_eq!(diff_mr, diff_ms);
        if matches!(val, "VDD" | "VBG" | "VBG_DLL") {
            diffs.push(("VDD_VBG", diff_mr));
        } else {
            diffs.push((val, diff_mr));
        }
    }
    ctx.insert(
        tile,
        bel,
        "DCM_VREF_SOURCE",
        xlat_enum_legacy_ocd(diffs, OcdMode::BitOrder),
    );

    ctx.collect_enum_legacy(
        tile,
        bel,
        "DLL_PHASE_SHIFT_CALIBRATION",
        &["MASK", "CONFIG", "AUTO_ZD2", "AUTO_DPS"],
    );
    ctx.collect_enum_legacy(tile, bel, "DLL_CONTROL_CLOCK_SPEED", &["QUARTER", "HALF"]);
    ctx.collect_enum_legacy(tile, bel, "DLL_PHASE_DETECTOR_MODE", &["LEVEL", "ENHANCED"]);
    ctx.collect_bit_bi_legacy(tile, bel, "DLL_PHASE_DETECTOR_AUTO_RESET", "FALSE", "TRUE");
    ctx.collect_bit_bi_legacy(tile, bel, "DLL_PERIOD_LOCK_BY1", "FALSE", "TRUE");
    ctx.collect_bit_bi_legacy(tile, bel, "DLL_DESKEW_LOCK_BY1", "FALSE", "TRUE");
    ctx.collect_bit_bi_legacy(tile, bel, "DLL_PHASE_SHIFT_LOCK_BY1", "FALSE", "TRUE");
    ctx.collect_bit_bi_legacy(tile, bel, "DLL_CTL_SEL_CLKIN_DIV2", "FALSE", "TRUE");
    ctx.collect_bit_wide_bi_legacy(tile, bel, "DUTY_CYCLE_CORRECTION", "FALSE", "TRUE");
    ctx.collect_bitvec_legacy(tile, bel, "DLL_PD_DLY_SEL", "");
    ctx.collect_bitvec_legacy(tile, bel, "DLL_DEAD_TIME", "");
    ctx.collect_bitvec_legacy(tile, bel, "DLL_LIVE_TIME", "");
    ctx.collect_bitvec_legacy(tile, bel, "DLL_DESKEW_MINTAP", "");
    ctx.collect_bitvec_legacy(tile, bel, "DLL_DESKEW_MAXTAP", "");
    ctx.collect_bitvec_legacy(tile, bel, "DLL_PHASE_SHIFT_LFC", "");
    ctx.collect_bitvec_legacy(tile, bel, "DLL_PHASE_SHIFT_HFC", "");
    ctx.collect_bitvec_legacy(tile, bel, "DLL_SETTLE_TIME", "");
    ctx.collect_bitvec_legacy(tile, bel, "DLL_SPARE", "");
    ctx.collect_bitvec_legacy(tile, bel, "DLL_TEST_MUX_SEL", "");
    ctx.collect_bitvec_legacy(tile, bel, "FACTORY_JF", "");
    let mut diffs = vec![];
    for val in ["LOW", "HIGH", "HIGH_SER"] {
        let mut diff = ctx.get_diff_legacy(tile, bel, "DLL_FREQUENCY_MODE", val);
        diff.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "FACTORY_JF"), 0xf0f0, 0);
        diffs.push((val, diff));
    }
    ctx.insert(tile, bel, "DLL_FREQUENCY_MODE", xlat_enum_legacy(diffs));

    let diff = ctx
        .peek_diff_legacy(tile, bel, "CLKOUT_PHASE_SHIFT", "NONE")
        .clone();
    ctx.insert(
        tile,
        bel,
        "PS_MODE",
        xlat_enum_legacy(vec![("CLKFB", Diff::default()), ("CLKIN", diff)]),
    );
    for val in [
        "NONE",
        "FIXED",
        "VARIABLE_POSITIVE",
        "VARIABLE_CENTER",
        "DIRECT",
    ] {
        let mut d = ctx.get_diff_legacy(tile, bel, "CLKOUT_PHASE_SHIFT", val);
        let mut dn = ctx.get_diff_legacy(tile, bel, "CLKOUT_PHASE_SHIFT.NEG", val);
        let mut dd = ctx.get_diff_legacy(tile, bel, "CLKOUT_PHASE_SHIFT.DLL", val);
        let item = ctx.item(tile, bel, "PS_MODE");
        d.apply_enum_diff_legacy(item, "CLKIN", "CLKFB");
        dd.apply_enum_diff_legacy(item, "CLKIN", "CLKFB");
        if val != "FIXED" {
            dn.apply_enum_diff_legacy(item, "CLKIN", "CLKFB");
        }
        if val != "NONE" && val != "DIRECT" {
            let item = ctx.item(tile, bel, "DLL_ZD2_EN");
            d.apply_bit_diff_legacy(item, true, false);
            dn.apply_bit_diff_legacy(item, true, false);
        }
        assert_eq!(d, dn);
        assert_eq!(d, dd);
        match val {
            "NONE" => d.assert_empty(),
            "FIXED" | "VARIABLE_POSITIVE" => ctx.insert(tile, bel, "PS_ENABLE", xlat_bit_legacy(d)),
            "VARIABLE_CENTER" => {
                d.apply_bit_diff_legacy(ctx.item(tile, bel, "PS_ENABLE"), true, false);
                ctx.insert(tile, bel, "PS_CENTERED", xlat_bit_legacy(d));
            }
            "DIRECT" => {
                d.apply_bit_diff_legacy(ctx.item(tile, bel, "PS_ENABLE"), true, false);
                d.apply_enum_diff_legacy(
                    ctx.item(tile, bel, "DLL_PHASE_SHIFT_CALIBRATION"),
                    "AUTO_ZD2",
                    "AUTO_DPS",
                );
                ctx.insert(tile, bel, "PS_DIRECT", xlat_bit_legacy(d));
            }
            _ => unreachable!(),
        }
    }

    for (attr, bits) in [
        ("CLKDV_PHASE_FALL", 0..2),
        ("CLKDV_PHASE_RISE", 2..4),
        ("CLKDV_COUNT_MAX", 4..8),
        ("CLKDV_COUNT_FALL_2", 8..12),
        ("CLKDV_COUNT_FALL", 12..16),
    ] {
        let bits = Vec::from_iter(bits.map(|bit| reg_bit(0x4d, bit)));
        ctx.insert(tile, bel, attr, TileItem::from_bitvec_inv(bits, false));
    }
    ctx.insert(
        tile,
        bel,
        "CLKDV_MODE",
        TileItem {
            bits: vec![reg_bit(0x4c, 15)],
            kind: TileItemKind::Enum {
                values: BTreeMap::from_iter([
                    ("HALF".to_string(), bits![0]),
                    ("INT".to_string(), bits![1]),
                ]),
            },
        },
    );

    let clkdv_count_max = ctx.data.bsdata.item(tile, bel, "CLKDV_COUNT_MAX").clone();
    let clkdv_count_fall = ctx.data.bsdata.item(tile, bel, "CLKDV_COUNT_FALL").clone();
    let clkdv_count_fall_2 = ctx
        .data
        .bsdata
        .item(tile, bel, "CLKDV_COUNT_FALL_2")
        .clone();
    let clkdv_phase_fall = ctx.data.bsdata.item(tile, bel, "CLKDV_PHASE_FALL").clone();
    let clkdv_mode = ctx.data.bsdata.item(tile, bel, "CLKDV_MODE").clone();
    for i in 2..=16 {
        let mut diff = ctx.get_diff_legacy(tile, bel, "CLKDV_DIVIDE", format!("{i}.0"));
        diff.apply_bitvec_diff_int_legacy(&clkdv_count_max, i - 1, 1);
        diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall, (i - 1) / 2, 0);
        diff.apply_bitvec_diff_int_legacy(&clkdv_phase_fall, (i % 2) * 2, 0);
        diff.assert_empty();
    }
    for i in 1..=7 {
        let mut diff = ctx.get_diff_legacy(tile, bel, "CLKDV_DIVIDE", format!("{i}.5.LOW"));
        diff.apply_enum_diff_legacy(&clkdv_mode, "HALF", "INT");
        diff.apply_bitvec_diff_int_legacy(&clkdv_count_max, 2 * i, 1);
        diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall, i / 2, 0);
        diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall_2, 3 * i / 2 + 1, 0);
        diff.apply_bitvec_diff_int_legacy(&clkdv_phase_fall, (i % 2) * 2 + 1, 0);
        diff.assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "CLKDV_DIVIDE", format!("{i}.5.HIGH"));
        assert_eq!(
            diff,
            ctx.get_diff_legacy(tile, bel, "CLKDV_DIVIDE", format!("{i}.5.HIGH_SER"))
        );
        diff.apply_enum_diff_legacy(&clkdv_mode, "HALF", "INT");
        diff.apply_bitvec_diff_int_legacy(&clkdv_count_max, 2 * i, 1);
        diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall, (i - 1) / 2, 0);
        diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall_2, (3 * i).div_ceil(2), 0);
        diff.apply_bitvec_diff_int_legacy(&clkdv_phase_fall, (i % 2) * 2, 0);
        diff.assert_empty();
    }

    ctx.collect_enum_legacy(tile, bel, "DFS_FREQUENCY_MODE", &["LOW", "HIGH"]);
    ctx.collect_bit_bi_legacy(tile, bel, "DFS_EN_RELRST", "FALSE", "TRUE");
    ctx.collect_bit_bi_legacy(tile, bel, "DFS_NON_STOP", "FALSE", "TRUE");
    ctx.collect_bit_bi_legacy(tile, bel, "DFS_EXTEND_RUN_TIME", "FALSE", "TRUE");
    ctx.collect_bit_bi_legacy(tile, bel, "DFS_EXTEND_HALT_TIME", "FALSE", "TRUE");
    ctx.collect_bit_bi_legacy(tile, bel, "DFS_EXTEND_FLUSH_TIME", "FALSE", "TRUE");
    ctx.collect_bit_bi_legacy(tile, bel, "DFS_EARLY_LOCK", "FALSE", "TRUE");
    ctx.collect_bit_bi_legacy(tile, bel, "DFS_SKIP_FINE", "FALSE", "TRUE");
    ctx.collect_enum_legacy(tile, bel, "DFS_COARSE_SEL", &["LEVEL", "LEGACY"]);
    ctx.collect_enum_legacy(tile, bel, "DFS_TP_SEL", &["LEVEL", "LEGACY"]);
    ctx.collect_enum_legacy(tile, bel, "DFS_FINE_SEL", &["LEVEL", "LEGACY"]);
    ctx.collect_enum_legacy_ocd(
        tile,
        bel,
        "DFS_AVE_FREQ_GAIN",
        &["0.125", "0.25", "0.5", "1.0", "2.0", "4.0", "8.0"],
        OcdMode::BitOrder,
    );
    ctx.collect_enum_legacy_int(tile, bel, "DFS_AVE_FREQ_SAMPLE_INTERVAL", 1..8, 0);
    ctx.collect_enum_legacy_int(tile, bel, "DFS_AVE_FREQ_ADJ_INTERVAL", 1..16, 0);
    ctx.collect_bit_bi_legacy(tile, bel, "DFS_TRACKMODE", "0", "1");
    ctx.collect_bitvec_legacy(tile, bel, "DFS_COIN_WINDOW", "");
    ctx.collect_bitvec_legacy(tile, bel, "DFS_HARDSYNC", "");
    ctx.collect_bitvec_legacy(tile, bel, "DFS_SPARE", "");
    ctx.collect_bitvec_legacy(tile, bel, "CLKFX_DIVIDE", "");
    ctx.collect_enum_legacy_int(tile, bel, "CLKFX_MULTIPLY", 1..32, 1);

    let mut diffs = vec![("PHASE_FREQ_LOCK", Diff::default())];
    for val in ["FREQ_LOCK", "AVE_FREQ_LOCK"] {
        let mut diff = ctx.get_diff_legacy(tile, bel, "DFS_OSCILLATOR_MODE", val);
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "DFS_EARLY_LOCK"), true, false);
        diff.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "DFS_HARDSYNC"), 3, 0);
        diffs.push((val, diff));
    }
    ctx.insert(tile, bel, "DFS_OSCILLATOR_MODE", xlat_enum_legacy(diffs));
    let item = xlat_bitvec_legacy(vec![ctx.get_diff_legacy(
        tile,
        bel,
        "DFS_OSCILLATOR_MODE",
        "PHASE_FREQ_LOCK",
    )]);
    ctx.insert(tile, bel, "DFS_FEEDBACK", item);

    ctx.collect_bit_legacy(tile, bel, "CLKIN_IOB", "1");
    let mut diff = ctx.get_diff_legacy(tile, bel, "CLKFB_IOB", "1");
    diff.apply_bit_diff_legacy(ctx.item(tile, bel, "DCM_EXT_FB_EN"), true, false);
    ctx.insert(tile, bel, "CLKFB_IOB", xlat_bit_legacy(diff));
    ctx.collect_bit_legacy(tile, bel, "CLKIN_ENABLE", "1");
    ctx.collect_bit_legacy(tile, bel, "CLKFB_ENABLE", "1");

    let dn = ctx.get_diff_legacy(tile, bel, "CLK_FEEDBACK", "NONE");
    assert_eq!(
        dn,
        ctx.get_diff_legacy(tile, bel, "CLK_FEEDBACK.CLKFB", "NONE")
    );
    let d1 = ctx.get_diff_legacy(tile, bel, "CLK_FEEDBACK", "1X");
    let df = ctx
        .get_diff_legacy(tile, bel, "CLK_FEEDBACK.CLKFB", "1X")
        .combine(&!&d1);
    let d2 = ctx.get_diff_legacy(tile, bel, "CLK_FEEDBACK", "2X");
    assert_eq!(
        df,
        ctx.get_diff_legacy(tile, bel, "CLK_FEEDBACK.CLKFB", "2X")
            .combine(&!&d2)
    );
    ctx.insert(tile, bel, "CLKFB_FEEDBACK", xlat_bit_legacy(df));
    ctx.insert(
        tile,
        bel,
        "CLK_FEEDBACK",
        xlat_enum_legacy(vec![("1X", d1), ("2X", d2), ("NONE", dn)]),
    );

    present.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "DCM_VBG_SEL"), 1, 0);
    present.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "CLKDV_COUNT_MAX"), 1, 0);
    present.apply_enum_diff_legacy(ctx.item(tile, bel, "CLKDV_MODE"), "INT", "HALF");
    present.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE.CLK90"), true, false);
    present.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE.CLK180"), true, false);
    present.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE.CLK270"), true, false);
    present.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE.CLK2X"), true, false);
    present.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE.CLK2X180"), true, false);
    present.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE.CLKDV"), true, false);
    present.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE.CLKFX180"), true, false);
    present.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE.CLKFX"), true, false);
    present.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE.CONCUR"), true, false);

    ctx.insert(tile, bel, "UNK_ALWAYS_SET", xlat_bit_legacy(present));

    for pin in ["CLKIN", "CLKFB"] {
        let mut diffs = vec![];
        for i in 0..8 {
            let diff = ctx.get_diff_legacy(tile, bel, pin, format!("HCLK{i}"));
            assert_eq!(
                diff,
                ctx.get_diff_legacy(tile, bel, format!("{pin}_TEST"), format!("HCLK{i}"))
            );
            diffs.push((format!("HCLK{i}"), diff));
        }
        for i in 0..16 {
            let diff = ctx.get_diff_legacy(tile, bel, pin, format!("GIOB{i}"));
            assert_eq!(
                diff,
                ctx.get_diff_legacy(tile, bel, format!("{pin}_TEST"), format!("GIOB{i}"))
            );
            diffs.push((format!("GIOB{i}"), diff));
        }
        for i in 0..4 {
            let diff = ctx.get_diff_legacy(tile, bel, pin, format!("MGT{i}"));
            assert_eq!(
                diff,
                ctx.get_diff_legacy(tile, bel, format!("{pin}_TEST"), format!("MGT{i}"))
            );
            diffs.push((format!("MGT{i}"), diff));
        }
        for i in 0..2 {
            let diff = ctx.get_diff_legacy(tile, bel, pin, format!("BUSOUT{i}"));
            assert_eq!(
                diff,
                ctx.get_diff_legacy(tile, bel, format!("{pin}_TEST"), format!("BUSOUT{i}"))
            );
            diffs.push((format!("BUSOUT{i}"), diff));
        }
        for i in 0..4 {
            let diff = ctx.get_diff_legacy(tile, bel, pin, format!("CKINT{i}"));
            let mut diff_test =
                ctx.get_diff_legacy(tile, bel, format!("{pin}_TEST"), format!("CKINT{i}"));
            let item = ctx.item_int_inv_legacy(&[tcls::INT; 4], tcid, bslot, &format!("CKINT{i}"));
            diff_test.apply_bit_diff(item, false, true);
            assert_eq!(diff, diff_test);
            diffs.push((format!("CKINT{i}"), diff));
        }
        ctx.insert(
            tile,
            bel,
            format!("MUX.{pin}"),
            xlat_enum_legacy_ocd(diffs, OcdMode::Mux),
        );
    }
    for i in 0..24 {
        ctx.collect_enum_legacy_ocd(
            tile,
            bel,
            &format!("MUX.BUSOUT{i}"),
            &[
                "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX",
                "CLKFX180", "CONCUR", "LOCKED", "CLK_IN0", "PASS",
            ],
            OcdMode::Mux,
        );
    }
}
