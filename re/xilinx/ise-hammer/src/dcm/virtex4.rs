use std::collections::BTreeMap;

use bitvec::prelude::*;
use prjcombine_re_collector::{Diff, OcdMode, xlat_bit, xlat_bitvec, xlat_enum, xlat_enum_ocd};
use prjcombine_re_hammer::Session;
use prjcombine_types::tiledb::{TileBit, TileItem, TileItemKind};

use crate::{
    backend::{IseBackend, PinFromKind},
    diff::CollectorCtx,
    fgen::TileBits,
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_multi_attr_bin, fuzz_multi_attr_dec, fuzz_multi_attr_dec_delta,
    fuzz_multi_attr_hex, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ctx = FuzzCtx::new(session, backend, "DCM", "DCM", TileBits::MainAuto);

    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "DCM_ADV")]);
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
        fuzz_inv!(ctx, pin, [(mode "DCM_ADV")]);
    }

    for pin in [
        "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX", "CLKFX180",
        "CONCUR",
    ] {
        fuzz_one!(ctx, pin, "1", [
            (mode "DCM_ADV"),
            (mutex "PIN", pin)
        ], [
            (pin pin)
        ]);
    }
    fuzz_one!(ctx, "CLKFB_ENABLE", "1", [
        (mode "DCM_ADV"),
        (pin_from "CLKFB", PinFromKind::Bufg)
    ], [
        (pin "CLKFB")
    ]);
    fuzz_one!(ctx, "CLKIN_ENABLE", "1", [
        (mode "DCM_ADV"),
        (pin_from "CLKIN", PinFromKind::Bufg)
    ], [
        (pin "CLKIN")
    ]);
    fuzz_one!(ctx, "CLKIN_IOB", "1", [
        (mode "DCM_ADV"),
        (global_mutex "DCM", "USE"),
        (pin "CLKIN"),
        (pin "CLKFB"),
        (pin_from "CLKFB", PinFromKind::Bufg)
    ], [
        (pin_from "CLKIN", PinFromKind::Bufg, PinFromKind::Iob)
    ]);
    fuzz_one!(ctx, "CLKFB_IOB", "1", [
        (mode "DCM_ADV"),
        (global_mutex "DCM", "USE"),
        (pin "CLKIN"),
        (pin "CLKFB"),
        (pin_from "CLKIN", PinFromKind::Bufg)
    ], [
        (pin_from "CLKFB", PinFromKind::Bufg, PinFromKind::Iob)
    ]);

    fuzz_multi_attr_dec!(ctx, "BGM_VLDLY", 3, [(mode "DCM_ADV")]);
    fuzz_multi_attr_dec!(ctx, "BGM_LDLY", 3, [(mode "DCM_ADV")]);
    fuzz_multi_attr_dec!(ctx, "BGM_SDLY", 3, [(mode "DCM_ADV")]);
    fuzz_multi_attr_dec!(ctx, "BGM_VSDLY", 3, [(mode "DCM_ADV")]);
    fuzz_multi_attr_dec!(ctx, "BGM_SAMPLE_LEN", 3, [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "BGM_MODE", ["BG_SNAPSHOT", "ABS_FREQ_SNAPSHOT", "ABS_FREQ_REF"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "BGM_CONFIG_REF_SEL", ["DCLK", "CLKIN"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "BGM_VADJ", ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15"], [(mode "DCM_ADV")]);
    fuzz_multi_attr_dec_delta!(ctx, "BGM_MULTIPLY", 6, 1, [(mode "DCM_ADV")]);
    fuzz_multi_attr_dec_delta!(ctx, "BGM_DIVIDE", 6, 1, [(mode "DCM_ADV")]);

    fuzz_enum!(ctx, "DCM_CLKDV_CLKFX_ALIGNMENT", ["FALSE", "TRUE"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DCM_LOCK_HIGH", ["FALSE", "TRUE"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DCM_VREG_ENABLE", ["FALSE", "TRUE"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DCM_EXT_FB_EN", ["FALSE", "TRUE"], [(mode "DCM_ADV"), (nopin "CLKFB")]);
    fuzz_enum!(ctx, "DCM_UNUSED_TAPS_POWERDOWN", ["FALSE", "TRUE"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DCM_PERFORMANCE_MODE", ["MAX_SPEED", "MAX_RANGE"], [(mode "DCM_ADV")]);
    for val in [
        "VDD",
        "VBG_DLL",
        "VBG",
        "BGM_SNAP",
        "BGM_ABS_SNAP",
        "BGM_ABS_REF",
    ] {
        fuzz_one!(ctx, "DCM_VREF_SOURCE.MAX_RANGE", val, [
            (mode "DCM_ADV"),
            (attr "DCM_PERFORMANCE_MODE", "MAX_RANGE")
        ], [
            (attr_diff "DCM_VREF_SOURCE", "VDD", val)
        ]);
        fuzz_one!(ctx, "DCM_VREF_SOURCE.MAX_SPEED", val, [
            (mode "DCM_ADV"),
            (attr "DCM_PERFORMANCE_MODE", "MAX_SPEED")
        ], [
            (attr_diff "DCM_VREF_SOURCE", "VDD", val)
        ]);
    }
    fuzz_enum!(ctx, "STARTUP_WAIT", ["FALSE", "TRUE"], [
        (mode "DCM_ADV"),
        (global_opt "GTS_CYCLE", "1"),
        (global_opt "DONE_CYCLE", "1"),
        (global_opt "LCK_CYCLE", "NOWAIT")
    ]);
    fuzz_enum!(ctx, "CLKIN_DIVIDE_BY_2", ["FALSE", "TRUE"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "PMCD_SYNC", ["FALSE", "TRUE"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "CLK_FEEDBACK", ["NONE", "1X", "2X"], [(mode "DCM_ADV"), (nopin "CLKFB")]);
    for val in ["NONE", "1X", "2X"] {
        fuzz_one!(ctx, "CLK_FEEDBACK.CLKFB", val, [
            (mode "DCM_ADV"),
            (pin "CLKFB"),
            (pin_from "CLKFB", PinFromKind::Bufg)
        ], [
            (attr "CLK_FEEDBACK", val)
        ]);
    }
    fuzz_enum!(ctx, "CLKOUT_PHASE_SHIFT", ["NONE", "FIXED", "VARIABLE_POSITIVE", "VARIABLE_CENTER", "DIRECT"], [
        (mode "DCM_ADV"),
        (attr "PHASE_SHIFT", "1"),
        (nopin "CLK0"),
        (nopin "CLK90"),
        (nopin "CLK180"),
        (nopin "CLK270"),
        (nopin "CLK2X"),
        (nopin "CLK2X180"),
        (nopin "CLKDV")
    ]);
    for val in [
        "NONE",
        "FIXED",
        "VARIABLE_POSITIVE",
        "VARIABLE_CENTER",
        "DIRECT",
    ] {
        fuzz_one!(ctx, "CLKOUT_PHASE_SHIFT.NEG", val, [
            (mode "DCM_ADV"),
            (attr "PHASE_SHIFT", "-1"),
            (nopin "CLK0"),
            (nopin "CLK90"),
            (nopin "CLK180"),
            (nopin "CLK270"),
            (nopin "CLK2X"),
            (nopin "CLK2X180"),
            (nopin "CLKDV")
        ], [
            (attr "CLKOUT_PHASE_SHIFT", val)
        ]);
    }
    for val in [
        "NONE",
        "FIXED",
        "VARIABLE_POSITIVE",
        "VARIABLE_CENTER",
        "DIRECT",
    ] {
        fuzz_one!(ctx, "CLKOUT_PHASE_SHIFT.DLL", val, [
            (mode "DCM_ADV"),
            (mutex "PIN", "NONE"),
            (attr "PHASE_SHIFT", "1"),
            (pin "CLK0")
        ], [
            (attr "CLKOUT_PHASE_SHIFT", val)
        ]);
    }
    fuzz_multi_attr_dec!(ctx, "DESKEW_ADJUST", 5, [(mode "DCM_ADV")]);
    fuzz_multi_attr_bin!(ctx, "DCM_PULSE_WIDTH_CORRECTION_LOW", 5, [(mode "DCM_ADV")]);
    fuzz_multi_attr_bin!(ctx, "DCM_PULSE_WIDTH_CORRECTION_HIGH", 5, [(mode "DCM_ADV")]);
    fuzz_multi_attr_bin!(ctx, "DCM_VBG_PD", 2, [(mode "DCM_ADV")]);
    fuzz_multi_attr_bin!(ctx, "DCM_VBG_SEL", 4, [
        (mode "DCM_ADV"),
        (attr "DCM_VREF_SOURCE", "VDD")
    ]);
    fuzz_multi_attr_bin!(ctx, "DCM_VREG_PHASE_MARGIN", 3, [(mode "DCM_ADV")]);
    fuzz_multi_attr_dec!(ctx, "PHASE_SHIFT", 10, [(mode "DCM_ADV")]);
    fuzz_one!(ctx, "PHASE_SHIFT", "-1", [
        (mode "DCM_ADV"),
        (attr "CLKOUT_PHASE_SHIFT", "NONE")
    ], [
        (attr "PHASE_SHIFT", "-1")
    ]);

    fuzz_enum!(ctx, "DLL_FREQUENCY_MODE", ["LOW", "HIGH", "HIGH_SER"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DLL_PHASE_SHIFT_CALIBRATION", ["AUTO_DPS", "CONFIG", "MASK", "AUTO_ZD2"], [
        (mode "DCM_ADV"),
        (attr "CLKOUT_PHASE_SHIFT", "NONE")
    ]);
    fuzz_enum!(ctx, "DLL_CONTROL_CLOCK_SPEED", ["QUARTER", "HALF"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DLL_PHASE_DETECTOR_MODE", ["LEVEL", "ENHANCED"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DLL_PHASE_DETECTOR_AUTO_RESET", ["FALSE", "TRUE"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DLL_PERIOD_LOCK_BY1", ["FALSE", "TRUE"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DLL_DESKEW_LOCK_BY1", ["FALSE", "TRUE"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DLL_PHASE_SHIFT_LOCK_BY1", ["FALSE", "TRUE"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DLL_CTL_SEL_CLKIN_DIV2", ["FALSE", "TRUE"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DUTY_CYCLE_CORRECTION", ["FALSE", "TRUE"], [(mode "DCM_ADV")]);
    fuzz_multi_attr_dec!(ctx, "DLL_PD_DLY_SEL", 3, [(mode "DCM_ADV")]);
    fuzz_multi_attr_dec!(ctx, "DLL_DEAD_TIME", 8, [(mode "DCM_ADV")]);
    fuzz_multi_attr_dec!(ctx, "DLL_LIVE_TIME", 8, [(mode "DCM_ADV")]);
    fuzz_multi_attr_dec!(ctx, "DLL_DESKEW_MINTAP", 8, [(mode "DCM_ADV")]);
    fuzz_multi_attr_dec!(ctx, "DLL_DESKEW_MAXTAP", 8, [(mode "DCM_ADV")]);
    fuzz_multi_attr_dec!(ctx, "DLL_PHASE_SHIFT_LFC", 8, [(mode "DCM_ADV")]);
    fuzz_multi_attr_dec!(ctx, "DLL_PHASE_SHIFT_HFC", 8, [(mode "DCM_ADV")]);
    fuzz_multi_attr_dec!(ctx, "DLL_SETTLE_TIME", 8, [(mode "DCM_ADV")]);
    fuzz_multi_attr_bin!(ctx, "DLL_SPARE", 16, [(mode "DCM_ADV")]);
    fuzz_multi_attr_bin!(ctx, "DLL_TEST_MUX_SEL", 2, [(mode "DCM_ADV")]);
    fuzz_multi_attr_hex!(ctx, "FACTORY_JF", 16, [
        (mode "DCM_ADV"),
        (attr "DLL_FREQUENCY_MODE", "")
    ]);
    fuzz_enum!(ctx, "CLKDV_DIVIDE", ["2.0", "3.0", "4.0", "5.0", "6.0", "7.0", "8.0", "9.0", "10.0", "11.0", "12.0", "13.0", "14.0", "15.0", "16.0"], [
        (mode "DCM_ADV")
    ]);
    for dll_mode in ["LOW", "HIGH", "HIGH_SER"] {
        for val in ["1.5", "2.5", "3.5", "4.5", "5.5", "6.5", "7.5"] {
            fuzz_one!(
                ctx,
                "CLKDV_DIVIDE",
                format!("{val}.{dll_mode}"), [
                    (mode "DCM_ADV"),
                    (global_mutex "DCM", "USE"),
                    (attr "DLL_FREQUENCY_MODE", dll_mode)
                ], [
                    (attr "CLKDV_DIVIDE", val)
                ]
            );
        }
    }

    fuzz_enum!(ctx, "DFS_FREQUENCY_MODE", ["LOW", "HIGH"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DFS_EN_RELRST", ["FALSE", "TRUE"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DFS_NON_STOP", ["FALSE", "TRUE"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DFS_EXTEND_RUN_TIME", ["FALSE", "TRUE"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DFS_EXTEND_HALT_TIME", ["FALSE", "TRUE"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DFS_EXTEND_FLUSH_TIME", ["FALSE", "TRUE"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DFS_EARLY_LOCK", ["FALSE", "TRUE"], [
        (mode "DCM_ADV"),
        (attr "DFS_OSCILLATOR_MODE", "")
    ]);
    fuzz_enum!(ctx, "DFS_SKIP_FINE", ["FALSE", "TRUE"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DFS_COARSE_SEL", ["LEVEL", "LEGACY"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DFS_TP_SEL", ["LEVEL", "LEGACY"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DFS_FINE_SEL", ["LEVEL", "LEGACY"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DFS_AVE_FREQ_GAIN", ["0.125", "0.25", "0.5", "1.0", "2.0", "4.0", "8.0"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DFS_AVE_FREQ_SAMPLE_INTERVAL", ["1", "2", "3", "4", "5", "6", "7"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DFS_AVE_FREQ_ADJ_INTERVAL", ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DFS_TRACKMODE", ["0", "1"], [(mode "DCM_ADV")]);
    fuzz_enum!(ctx, "DFS_OSCILLATOR_MODE", ["PHASE_FREQ_LOCK", "FREQ_LOCK", "AVE_FREQ_LOCK"], [
        (mode "DCM_ADV"),
        (mutex "PIN", "NONE"),
        (pin "CLK0"),
        (pin "CLKFX")
    ]);
    fuzz_multi_attr_bin!(ctx, "DFS_COIN_WINDOW", 2, [(mode "DCM_ADV")]);
    fuzz_multi_attr_bin!(ctx, "DFS_HARDSYNC", 2, [
        (mode "DCM_ADV"),
        (attr "DFS_OSCILLATOR_MODE", "")
    ]);
    fuzz_multi_attr_bin!(ctx, "DFS_SPARE", 16, [(mode "DCM_ADV")]);
    fuzz_multi_attr_dec_delta!(ctx, "CLKFX_DIVIDE", 5, 1, [(mode "DCM_ADV")]);
    for val in 2..=32 {
        fuzz_one!(ctx, "CLKFX_MULTIPLY", format!("{val}"), [
            (mode "DCM_ADV")
        ], [
            (attr "CLKFX_MULTIPLY", format!("{val}"))
        ]);
    }

    for (pin, opin) in [("CLKIN", "CLKFB"), ("CLKFB", "CLKIN")] {
        for rpin in [pin.to_string(), format!("{pin}_TEST")] {
            for i in 0..8 {
                fuzz_one!(ctx, &rpin, format!("HCLK{i}"), [
                    (global_mutex "HCLK_DCM", "USE"),
                    (mode "DCM_ADV"),
                    (pin pin),
                    (pin opin),
                    (mutex format!("{pin}_OUT"), &rpin),
                    (mutex format!("{pin}_IN"), format!("HCLK{i}")),
                    (mutex format!("{opin}_OUT"), "HOLD"),
                    (mutex format!("{opin}_IN"), format!("HCLK{i}")),
                    (pip (pin format!("HCLK{i}")), (pin opin))
                ], [
                    (pip (pin format!("HCLK{i}")), (pin rpin))
                ]);
            }
            for i in 0..16 {
                fuzz_one!(ctx, &rpin, format!("GIOB{i}"), [
                    (global_mutex "HCLK_DCM", "USE"),
                    (mode "DCM_ADV"),
                    (pin pin),
                    (pin opin),
                    (mutex format!("{pin}_OUT"), &rpin),
                    (mutex format!("{pin}_IN"), format!("GIOB{i}")),
                    (mutex format!("{opin}_OUT"), "HOLD"),
                    (mutex format!("{opin}_IN"), format!("GIOB{i}")),
                    (pip (pin format!("GIOB{i}")), (pin opin))
                ], [
                    (pip (pin format!("GIOB{i}")), (pin rpin))
                ]);
            }
            for i in 0..4 {
                fuzz_one!(ctx, &rpin, format!("MGT{i}"), [
                    (global_mutex "HCLK_DCM", "USE"),
                    (mode "DCM_ADV"),
                    (pin pin),
                    (pin opin),
                    (mutex format!("{pin}_OUT"), &rpin),
                    (mutex format!("{pin}_IN"), format!("MGT{i}")),
                    (mutex format!("{opin}_OUT"), "HOLD"),
                    (mutex format!("{opin}_IN"), format!("MGT{i}")),
                    (pip (pin format!("MGT{i}")), (pin opin))
                ], [
                    (pip (pin format!("MGT{i}")), (pin rpin))
                ]);
            }
            for i in 0..2 {
                fuzz_one!(ctx, &rpin, format!("BUSOUT{i}"), [
                    (mode "DCM_ADV"),
                    (pin pin),
                    (pin opin),
                    (mutex format!("{pin}_OUT"), &rpin),
                    (mutex format!("{pin}_IN"), format!("BUSOUT{i}")),
                    (mutex format!("{opin}_OUT"), "HOLD"),
                    (mutex format!("{opin}_IN"), format!("BUSOUT{i}")),
                    (pip (pin format!("BUSOUT{i}")), (pin opin))
                ], [
                    (pip (pin format!("BUSOUT{i}")), (pin rpin))
                ]);
            }
            for i in 0..4 {
                fuzz_one!(ctx, &rpin, format!("CKINT{i}"), [
                    (mode "DCM_ADV"),
                    (pin pin),
                    (mutex format!("{pin}_OUT"), &rpin),
                    (mutex format!("{pin}_IN"), format!("CKINT{i}")),
                    (mutex format!("CKINT{i}"), &rpin)
                ], [
                    (pip (pin format!("CKINT{i}")), (pin rpin))
                ]);
            }
        }
    }

    for i in 0..24 {
        fuzz_one!(ctx, format!("MUX.BUSOUT{i}"), "PASS", [
            (mutex format!("BUSOUT{i}"), format!("BUSIN{i}"))
        ], [
            (pip (pin format!("BUSIN{i}")), (pin format!("BUSOUT{i}")))
        ]);
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
            fuzz_one!(ctx, format!("MUX.BUSOUT{i}"), sname, [
                (mutex format!("BUSOUT{i}"), inp)
            ], [
                (pip (pin inp), (pin format!("BUSOUT{i}")))
            ]);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "DCM";
    let bel = "DCM";

    let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "1");

    fn reg_bit(addr: usize, bit: usize) -> TileBit {
        TileBit {
            tile: (addr >> 2) & 3,
            frame: 20 - (addr >> 4 & 1),
            bit: bit + 1 + (addr & 3) * 20,
        }
    }

    for addr in 0x40..0x60 {
        let reg_mask_bit = reg_bit(addr, 17);
        assert_eq!(present.bits.remove(&reg_mask_bit), Some(true));
        ctx.tiledb.insert(
            tile,
            bel,
            format!("DRP{addr:02X}_MASK"),
            TileItem::from_bit(reg_mask_bit, false),
        );
        ctx.tiledb.insert(
            tile,
            bel,
            format!("DRP{addr:02X}"),
            TileItem::from_bitvec(Vec::from_iter((0..16).map(|bit| reg_bit(addr, bit))), false),
        );
    }

    for pin in ["RST", "CTLMODE", "FREEZE_DLL", "FREEZE_DFS", "DEN", "DWE"] {
        ctx.collect_int_inv(&["INT"; 4], tile, bel, pin, false);
    }

    for pin in [
        "DI0", "DI1", "DI2", "DI3", "DI4", "DI5", "DI6", "DI7", "DI8", "DI9", "DI10", "DI11",
        "DI12", "DI13", "DI14", "DI15", "DADDR0", "DADDR1", "DADDR2", "DADDR3", "DADDR4", "DADDR5",
        "DADDR6", "PSEN", "PSINCDEC", "CTLSEL0", "CTLSEL1", "CTLSEL2", "CTLOSC1", "CTLOSC2",
        "CTLGO",
    ] {
        ctx.collect_inv(tile, bel, pin);
    }

    let diff = ctx.state.get_diff(tile, bel, "CLK2X", "1");
    for pin in ["CLK2X180", "CLKDV", "CLK90", "CLK180", "CLK270"] {
        assert_eq!(diff, ctx.state.get_diff(tile, bel, pin, "1"));
    }
    let diff_0 = ctx.state.get_diff(tile, bel, "CLK0", "1");
    let diff_0 = diff_0.combine(&!&diff);
    ctx.tiledb
        .insert(tile, bel, "ENABLE.CLK0", xlat_bit(diff_0));
    // ???
    ctx.tiledb.insert(
        tile,
        bel,
        "ENABLE.CLK90",
        TileItem::from_bit(reg_bit(0x4e, 1), false),
    );
    ctx.tiledb.insert(
        tile,
        bel,
        "ENABLE.CLK180",
        TileItem::from_bit(reg_bit(0x4e, 2), false),
    );
    ctx.tiledb.insert(
        tile,
        bel,
        "ENABLE.CLK270",
        TileItem::from_bit(reg_bit(0x4e, 3), false),
    );
    ctx.tiledb.insert(
        tile,
        bel,
        "ENABLE.CLK2X",
        TileItem::from_bit(reg_bit(0x4e, 4), false),
    );
    ctx.tiledb.insert(
        tile,
        bel,
        "ENABLE.CLK2X180",
        TileItem::from_bit(reg_bit(0x4e, 5), false),
    );
    ctx.tiledb.insert(
        tile,
        bel,
        "ENABLE.CLKDV",
        TileItem::from_bit(reg_bit(0x4e, 6), false),
    );
    ctx.tiledb.insert(
        tile,
        bel,
        "ENABLE.CLKFX180",
        TileItem::from_bit(reg_bit(0x51, 8), false),
    );
    ctx.tiledb.insert(
        tile,
        bel,
        "ENABLE.CLKFX",
        TileItem::from_bit(reg_bit(0x51, 9), false),
    );
    ctx.tiledb.insert(
        tile,
        bel,
        "ENABLE.CONCUR",
        TileItem::from_bit(reg_bit(0x51, 10), false),
    );

    ctx.tiledb.insert(tile, bel, "DLL_ZD2_EN", xlat_bit(diff));
    let diff = ctx.state.get_diff(tile, bel, "CLKFX", "1");
    for pin in ["CLKFX180", "CONCUR"] {
        assert_eq!(diff, ctx.state.get_diff(tile, bel, pin, "1"));
    }
    ctx.tiledb.insert(tile, bel, "DFS_ENABLE", xlat_bit(diff));

    ctx.collect_bitvec(tile, bel, "BGM_VLDLY", "");
    ctx.collect_bitvec(tile, bel, "BGM_LDLY", "");
    ctx.collect_bitvec(tile, bel, "BGM_SDLY", "");
    ctx.collect_bitvec(tile, bel, "BGM_VSDLY", "");
    ctx.collect_bitvec(tile, bel, "BGM_SAMPLE_LEN", "");
    ctx.collect_enum_ocd(
        tile,
        bel,
        "BGM_MODE",
        &["BG_SNAPSHOT", "ABS_FREQ_SNAPSHOT", "ABS_FREQ_REF"],
        OcdMode::BitOrder,
    );
    ctx.collect_enum(tile, bel, "BGM_CONFIG_REF_SEL", &["DCLK", "CLKIN"]);
    ctx.collect_enum_int(tile, bel, "BGM_VADJ", 1..16, 0);
    ctx.collect_bitvec(tile, bel, "BGM_MULTIPLY", "");
    ctx.collect_bitvec(tile, bel, "BGM_DIVIDE", "");

    ctx.collect_enum_bool(tile, bel, "DCM_CLKDV_CLKFX_ALIGNMENT", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "DCM_LOCK_HIGH", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "DCM_VREG_ENABLE", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "DCM_EXT_FB_EN", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "DCM_UNUSED_TAPS_POWERDOWN", "FALSE", "TRUE");
    ctx.collect_enum(
        tile,
        bel,
        "DCM_PERFORMANCE_MODE",
        &["MAX_SPEED", "MAX_RANGE"],
    );
    ctx.collect_enum_bool(tile, bel, "STARTUP_WAIT", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "CLKIN_DIVIDE_BY_2", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "PMCD_SYNC", "FALSE", "TRUE");
    ctx.collect_bitvec(tile, bel, "DESKEW_ADJUST", "");
    ctx.collect_bitvec(tile, bel, "DCM_PULSE_WIDTH_CORRECTION_LOW", "");
    ctx.collect_bitvec(tile, bel, "DCM_PULSE_WIDTH_CORRECTION_HIGH", "");
    ctx.collect_bitvec(tile, bel, "DCM_VBG_PD", "");
    ctx.collect_bitvec(tile, bel, "DCM_VBG_SEL", "");
    ctx.collect_bitvec(tile, bel, "DCM_VREG_PHASE_MARGIN", "");
    ctx.collect_bitvec(tile, bel, "PHASE_SHIFT", "");
    let mut diff = ctx.state.get_diff(tile, bel, "PHASE_SHIFT", "-1");
    diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PHASE_SHIFT"), 1, 0);
    ctx.tiledb
        .insert(tile, bel, "PHASE_SHIFT_NEGATIVE", xlat_bit(diff));

    let mut diffs = vec![];
    for val in [
        "VDD",
        "VBG_DLL",
        "VBG",
        "BGM_SNAP",
        "BGM_ABS_SNAP",
        "BGM_ABS_REF",
    ] {
        let mut diff_mr = ctx
            .state
            .get_diff(tile, bel, "DCM_VREF_SOURCE.MAX_RANGE", val);
        let mut diff_ms = ctx
            .state
            .get_diff(tile, bel, "DCM_VREF_SOURCE.MAX_SPEED", val);
        if val == "VBG" {
            diff_mr.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DCM_VBG_SEL"), 0x1, 0);
            diff_ms.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DCM_VBG_SEL"), 0x1, 0);
        } else if val != "VDD" {
            diff_mr.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DCM_VBG_SEL"), 0x5, 0);
            diff_ms.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DCM_VBG_SEL"), 0x9, 0);
        }
        assert_eq!(diff_mr, diff_ms);
        if matches!(val, "VDD" | "VBG" | "VBG_DLL") {
            diffs.push(("VDD_VBG", diff_mr));
        } else {
            diffs.push((val, diff_mr));
        }
    }
    ctx.tiledb.insert(
        tile,
        bel,
        "DCM_VREF_SOURCE",
        xlat_enum_ocd(diffs, OcdMode::BitOrder),
    );

    ctx.collect_enum(
        tile,
        bel,
        "DLL_PHASE_SHIFT_CALIBRATION",
        &["MASK", "CONFIG", "AUTO_ZD2", "AUTO_DPS"],
    );
    ctx.collect_enum(tile, bel, "DLL_CONTROL_CLOCK_SPEED", &["QUARTER", "HALF"]);
    ctx.collect_enum(tile, bel, "DLL_PHASE_DETECTOR_MODE", &["LEVEL", "ENHANCED"]);
    ctx.collect_enum_bool(tile, bel, "DLL_PHASE_DETECTOR_AUTO_RESET", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "DLL_PERIOD_LOCK_BY1", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "DLL_DESKEW_LOCK_BY1", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "DLL_PHASE_SHIFT_LOCK_BY1", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "DLL_CTL_SEL_CLKIN_DIV2", "FALSE", "TRUE");
    ctx.collect_enum_bool_wide(tile, bel, "DUTY_CYCLE_CORRECTION", "FALSE", "TRUE");
    ctx.collect_bitvec(tile, bel, "DLL_PD_DLY_SEL", "");
    ctx.collect_bitvec(tile, bel, "DLL_DEAD_TIME", "");
    ctx.collect_bitvec(tile, bel, "DLL_LIVE_TIME", "");
    ctx.collect_bitvec(tile, bel, "DLL_DESKEW_MINTAP", "");
    ctx.collect_bitvec(tile, bel, "DLL_DESKEW_MAXTAP", "");
    ctx.collect_bitvec(tile, bel, "DLL_PHASE_SHIFT_LFC", "");
    ctx.collect_bitvec(tile, bel, "DLL_PHASE_SHIFT_HFC", "");
    ctx.collect_bitvec(tile, bel, "DLL_SETTLE_TIME", "");
    ctx.collect_bitvec(tile, bel, "DLL_SPARE", "");
    ctx.collect_bitvec(tile, bel, "DLL_TEST_MUX_SEL", "");
    ctx.collect_bitvec(tile, bel, "FACTORY_JF", "");
    let mut diffs = vec![];
    for val in ["LOW", "HIGH", "HIGH_SER"] {
        let mut diff = ctx.state.get_diff(tile, bel, "DLL_FREQUENCY_MODE", val);
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "FACTORY_JF"), 0xf0f0, 0);
        diffs.push((val, diff));
    }
    ctx.tiledb
        .insert(tile, bel, "DLL_FREQUENCY_MODE", xlat_enum(diffs));

    let diff = ctx
        .state
        .peek_diff(tile, bel, "CLKOUT_PHASE_SHIFT", "NONE")
        .clone();
    ctx.tiledb.insert(
        tile,
        bel,
        "PS_MODE",
        xlat_enum(vec![("CLKFB", Diff::default()), ("CLKIN", diff)]),
    );
    for val in [
        "NONE",
        "FIXED",
        "VARIABLE_POSITIVE",
        "VARIABLE_CENTER",
        "DIRECT",
    ] {
        let mut d = ctx.state.get_diff(tile, bel, "CLKOUT_PHASE_SHIFT", val);
        let mut dn = ctx.state.get_diff(tile, bel, "CLKOUT_PHASE_SHIFT.NEG", val);
        let mut dd = ctx.state.get_diff(tile, bel, "CLKOUT_PHASE_SHIFT.DLL", val);
        let item = ctx.tiledb.item(tile, bel, "PS_MODE");
        d.apply_enum_diff(item, "CLKIN", "CLKFB");
        dd.apply_enum_diff(item, "CLKIN", "CLKFB");
        if val != "FIXED" {
            dn.apply_enum_diff(item, "CLKIN", "CLKFB");
        }
        if val != "NONE" && val != "DIRECT" {
            let item = ctx.tiledb.item(tile, bel, "DLL_ZD2_EN");
            d.apply_bit_diff(item, true, false);
            dn.apply_bit_diff(item, true, false);
        }
        assert_eq!(d, dn);
        assert_eq!(d, dd);
        match val {
            "NONE" => d.assert_empty(),
            "FIXED" | "VARIABLE_POSITIVE" => ctx.tiledb.insert(tile, bel, "PS_ENABLE", xlat_bit(d)),
            "VARIABLE_CENTER" => {
                d.apply_bit_diff(ctx.tiledb.item(tile, bel, "PS_ENABLE"), true, false);
                ctx.tiledb.insert(tile, bel, "PS_CENTERED", xlat_bit(d));
            }
            "DIRECT" => {
                d.apply_bit_diff(ctx.tiledb.item(tile, bel, "PS_ENABLE"), true, false);
                d.apply_enum_diff(
                    ctx.tiledb.item(tile, bel, "DLL_PHASE_SHIFT_CALIBRATION"),
                    "AUTO_ZD2",
                    "AUTO_DPS",
                );
                ctx.tiledb.insert(tile, bel, "PS_DIRECT", xlat_bit(d));
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
        ctx.tiledb
            .insert(tile, bel, attr, TileItem::from_bitvec(bits, false));
    }
    ctx.tiledb.insert(
        tile,
        bel,
        "CLKDV_MODE",
        TileItem {
            bits: vec![reg_bit(0x4c, 15)],
            kind: TileItemKind::Enum {
                values: BTreeMap::from_iter([
                    ("HALF".to_string(), bitvec![0]),
                    ("INT".to_string(), bitvec![1]),
                ]),
            },
        },
    );

    let clkdv_count_max = ctx.collector.tiledb.item(tile, bel, "CLKDV_COUNT_MAX");
    let clkdv_count_fall = ctx.collector.tiledb.item(tile, bel, "CLKDV_COUNT_FALL");
    let clkdv_count_fall_2 = ctx.collector.tiledb.item(tile, bel, "CLKDV_COUNT_FALL_2");
    let clkdv_phase_fall = ctx.collector.tiledb.item(tile, bel, "CLKDV_PHASE_FALL");
    let clkdv_mode = ctx.collector.tiledb.item(tile, bel, "CLKDV_MODE");
    for i in 2..=16 {
        let mut diff = ctx
            .collector
            .state
            .get_diff(tile, bel, "CLKDV_DIVIDE", format!("{i}.0"));
        diff.apply_bitvec_diff_int(clkdv_count_max, i - 1, 1);
        diff.apply_bitvec_diff_int(clkdv_count_fall, (i - 1) / 2, 0);
        diff.apply_bitvec_diff_int(clkdv_phase_fall, (i % 2) * 2, 0);
        diff.assert_empty();
    }
    for i in 1..=7 {
        let mut diff =
            ctx.collector
                .state
                .get_diff(tile, bel, "CLKDV_DIVIDE", format!("{i}.5.LOW"));
        diff.apply_enum_diff(clkdv_mode, "HALF", "INT");
        diff.apply_bitvec_diff_int(clkdv_count_max, 2 * i, 1);
        diff.apply_bitvec_diff_int(clkdv_count_fall, i / 2, 0);
        diff.apply_bitvec_diff_int(clkdv_count_fall_2, 3 * i / 2 + 1, 0);
        diff.apply_bitvec_diff_int(clkdv_phase_fall, (i % 2) * 2 + 1, 0);
        diff.assert_empty();
        let mut diff =
            ctx.collector
                .state
                .get_diff(tile, bel, "CLKDV_DIVIDE", format!("{i}.5.HIGH"));
        assert_eq!(
            diff,
            ctx.collector
                .state
                .get_diff(tile, bel, "CLKDV_DIVIDE", format!("{i}.5.HIGH_SER"))
        );
        diff.apply_enum_diff(clkdv_mode, "HALF", "INT");
        diff.apply_bitvec_diff_int(clkdv_count_max, 2 * i, 1);
        diff.apply_bitvec_diff_int(clkdv_count_fall, (i - 1) / 2, 0);
        diff.apply_bitvec_diff_int(clkdv_count_fall_2, (3 * i).div_ceil(2), 0);
        diff.apply_bitvec_diff_int(clkdv_phase_fall, (i % 2) * 2, 0);
        diff.assert_empty();
    }

    ctx.collect_enum(tile, bel, "DFS_FREQUENCY_MODE", &["LOW", "HIGH"]);
    ctx.collect_enum_bool(tile, bel, "DFS_EN_RELRST", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "DFS_NON_STOP", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "DFS_EXTEND_RUN_TIME", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "DFS_EXTEND_HALT_TIME", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "DFS_EXTEND_FLUSH_TIME", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "DFS_EARLY_LOCK", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "DFS_SKIP_FINE", "FALSE", "TRUE");
    ctx.collect_enum(tile, bel, "DFS_COARSE_SEL", &["LEVEL", "LEGACY"]);
    ctx.collect_enum(tile, bel, "DFS_TP_SEL", &["LEVEL", "LEGACY"]);
    ctx.collect_enum(tile, bel, "DFS_FINE_SEL", &["LEVEL", "LEGACY"]);
    ctx.collect_enum_ocd(
        tile,
        bel,
        "DFS_AVE_FREQ_GAIN",
        &["0.125", "0.25", "0.5", "1.0", "2.0", "4.0", "8.0"],
        OcdMode::BitOrder,
    );
    ctx.collect_enum_int(tile, bel, "DFS_AVE_FREQ_SAMPLE_INTERVAL", 1..8, 0);
    ctx.collect_enum_int(tile, bel, "DFS_AVE_FREQ_ADJ_INTERVAL", 1..16, 0);
    ctx.collect_enum_bool(tile, bel, "DFS_TRACKMODE", "0", "1");
    ctx.collect_bitvec(tile, bel, "DFS_COIN_WINDOW", "");
    ctx.collect_bitvec(tile, bel, "DFS_HARDSYNC", "");
    ctx.collect_bitvec(tile, bel, "DFS_SPARE", "");
    ctx.collect_bitvec(tile, bel, "CLKFX_DIVIDE", "");
    ctx.collect_enum_int(tile, bel, "CLKFX_MULTIPLY", 1..32, 1);

    let mut diffs = vec![("PHASE_FREQ_LOCK", Diff::default())];
    for val in ["FREQ_LOCK", "AVE_FREQ_LOCK"] {
        let mut diff = ctx.state.get_diff(tile, bel, "DFS_OSCILLATOR_MODE", val);
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "DFS_EARLY_LOCK"), true, false);
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DFS_HARDSYNC"), 3, 0);
        diffs.push((val, diff));
    }
    ctx.tiledb
        .insert(tile, bel, "DFS_OSCILLATOR_MODE", xlat_enum(diffs));
    let item = xlat_bitvec(vec![ctx.state.get_diff(
        tile,
        bel,
        "DFS_OSCILLATOR_MODE",
        "PHASE_FREQ_LOCK",
    )]);
    ctx.tiledb.insert(tile, bel, "DFS_FEEDBACK", item);

    ctx.collect_bit(tile, bel, "CLKIN_IOB", "1");
    let mut diff = ctx.state.get_diff(tile, bel, "CLKFB_IOB", "1");
    diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "DCM_EXT_FB_EN"), true, false);
    ctx.tiledb.insert(tile, bel, "CLKFB_IOB", xlat_bit(diff));
    ctx.collect_bit(tile, bel, "CLKIN_ENABLE", "1");
    ctx.collect_bit(tile, bel, "CLKFB_ENABLE", "1");

    let dn = ctx.state.get_diff(tile, bel, "CLK_FEEDBACK", "NONE");
    assert_eq!(
        dn,
        ctx.state.get_diff(tile, bel, "CLK_FEEDBACK.CLKFB", "NONE")
    );
    let d1 = ctx.state.get_diff(tile, bel, "CLK_FEEDBACK", "1X");
    let df = ctx
        .state
        .get_diff(tile, bel, "CLK_FEEDBACK.CLKFB", "1X")
        .combine(&!&d1);
    let d2 = ctx.state.get_diff(tile, bel, "CLK_FEEDBACK", "2X");
    assert_eq!(
        df,
        ctx.state
            .get_diff(tile, bel, "CLK_FEEDBACK.CLKFB", "2X")
            .combine(&!&d2)
    );
    ctx.tiledb.insert(tile, bel, "CLKFB_FEEDBACK", xlat_bit(df));
    ctx.tiledb.insert(
        tile,
        bel,
        "CLK_FEEDBACK",
        xlat_enum(vec![("1X", d1), ("2X", d2), ("NONE", dn)]),
    );

    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DCM_VBG_SEL"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKDV_COUNT_MAX"), 1, 0);
    present.apply_enum_diff(ctx.tiledb.item(tile, bel, "CLKDV_MODE"), "INT", "HALF");
    present.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE.CLK90"), true, false);
    present.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE.CLK180"), true, false);
    present.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE.CLK270"), true, false);
    present.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE.CLK2X"), true, false);
    present.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE.CLK2X180"), true, false);
    present.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE.CLKDV"), true, false);
    present.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE.CLKFX180"), true, false);
    present.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE.CLKFX"), true, false);
    present.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE.CONCUR"), true, false);

    ctx.tiledb
        .insert(tile, bel, "UNK_ALWAYS_SET", xlat_bit(present));

    for pin in ["CLKIN", "CLKFB"] {
        let mut diffs = vec![];
        for i in 0..8 {
            let diff = ctx.state.get_diff(tile, bel, pin, format!("HCLK{i}"));
            assert_eq!(
                diff,
                ctx.state
                    .get_diff(tile, bel, format!("{pin}_TEST"), format!("HCLK{i}"))
            );
            diffs.push((format!("HCLK{i}"), diff));
        }
        for i in 0..16 {
            let diff = ctx.state.get_diff(tile, bel, pin, format!("GIOB{i}"));
            assert_eq!(
                diff,
                ctx.state
                    .get_diff(tile, bel, format!("{pin}_TEST"), format!("GIOB{i}"))
            );
            diffs.push((format!("GIOB{i}"), diff));
        }
        for i in 0..4 {
            let diff = ctx.state.get_diff(tile, bel, pin, format!("MGT{i}"));
            assert_eq!(
                diff,
                ctx.state
                    .get_diff(tile, bel, format!("{pin}_TEST"), format!("MGT{i}"))
            );
            diffs.push((format!("MGT{i}"), diff));
        }
        for i in 0..2 {
            let diff = ctx.state.get_diff(tile, bel, pin, format!("BUSOUT{i}"));
            assert_eq!(
                diff,
                ctx.state
                    .get_diff(tile, bel, format!("{pin}_TEST"), format!("BUSOUT{i}"))
            );
            diffs.push((format!("BUSOUT{i}"), diff));
        }
        for i in 0..4 {
            let diff = ctx.state.get_diff(tile, bel, pin, format!("CKINT{i}"));
            let mut diff_test =
                ctx.state
                    .get_diff(tile, bel, format!("{pin}_TEST"), format!("CKINT{i}"));
            let item = ctx.item_int_inv(&["INT"; 4], tile, bel, &format!("CKINT{i}"));
            diff_test.apply_bit_diff(&item, false, true);
            assert_eq!(diff, diff_test);
            diffs.push((format!("CKINT{i}"), diff));
        }
        ctx.tiledb.insert(
            tile,
            bel,
            format!("MUX.{pin}"),
            xlat_enum_ocd(diffs, OcdMode::Mux),
        );
    }
    for i in 0..24 {
        ctx.collect_enum_ocd(
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
