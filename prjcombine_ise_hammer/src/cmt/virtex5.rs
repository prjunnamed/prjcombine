use std::collections::BTreeMap;

use bitvec::prelude::*;
use prjcombine_collector::{extract_bitvec_val_part, xlat_bit, xlat_enum, Diff, OcdMode};
use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_types::tiledb::{TileBit, TileItem, TileItemKind};
use unnamed_entity::EntityId;

use crate::{
    backend::{IseBackend, PinFromKind},
    diff::CollectorCtx,
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits, TileRelation},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_enum_suffix, fuzz_inv, fuzz_multi_attr_bin, fuzz_multi_attr_dec,
    fuzz_multi_attr_dec_delta, fuzz_multi_attr_hex, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(
    session: &mut Session<IseBackend<'a>>,
    backend: &IseBackend<'a>,
    skip_dcm: bool,
    skip_pll: bool,
    devdata_only: bool,
) {
    let hclk_cmt = backend.egrid.db.get_node("HCLK_CMT");
    let bel_dcm0 = BelId::from_idx(0);
    let bel_dcm1 = BelId::from_idx(1);
    let bel_pll = BelId::from_idx(2);
    let bel_cmt = BelId::from_idx(3);

    if devdata_only {
        let ctx = FuzzCtx::new(session, backend, "CMT", "PLL", TileBits::MainAuto);
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::Hclk(0, 0),
            "HCLK_CMT",
            "HCLK_CMT",
            "DRP_MASK",
            "1",
        )];
        fuzz_one_extras!(ctx, "ENABLE", "1", [
            (global_xy "PLLADV_*_USE_CALC", "NO")
        ], [
            (related TileRelation::Hclk(hclk_cmt),
                (tile_mutex "ENABLE")),
            (mode "PLL_ADV")
        ], extras);
        return;
    }

    if !skip_dcm {
        for i in 0..2 {
            let ctx = FuzzCtx::new(
                session,
                backend,
                "CMT",
                format!("DCM{i}"),
                TileBits::MainAuto,
            );
            let extras = vec![ExtraFeature::new(
                ExtraFeatureKind::Hclk(0, 0),
                "HCLK_CMT",
                "HCLK_CMT",
                "DRP_MASK",
                "1",
            )];
            fuzz_one_extras!(ctx, "ENABLE", "1", [
            ], [
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE")),
                (mode "DCM_ADV")
            ], extras);

            for pin in [
                "PSEN",
                "PSINCDEC",
                "RST",
                "SKEWCLKIN1",
                "SKEWCLKIN2",
                "SKEWIN",
                "SKEWRST",
            ] {
                fuzz_inv!(ctx, pin, [
                    (mutex "MODE", "INV"),
                    (related TileRelation::Hclk(hclk_cmt),
                        (tile_mutex "ENABLE", "USE")),
                    (mode "DCM_ADV")
                ]);
            }
            for pin in [
                "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX",
                "CLKFX180", "CONCUR",
            ] {
                fuzz_one!(ctx, pin, "1", [
                    (mutex "MODE", "PIN"),
                    (related TileRelation::Hclk(hclk_cmt),
                        (tile_mutex "ENABLE", "USE")),
                    (mode "DCM_ADV"),
                    (mutex "PIN", pin)
                ], [
                    (pin pin)
                ]);
            }

            fuzz_one!(ctx, "CLKFB_ENABLE", "1", [
                (mutex "MODE", "PIN"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV"),
                (pin_from "CLKFB", PinFromKind::Bufg)
            ], [
                (pin "CLKFB")
            ]);
            fuzz_one!(ctx, "CLKIN_ENABLE", "1", [
                (mutex "MODE", "PIN"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV"),
                (pin_from "CLKIN", PinFromKind::Bufg)
            ], [
                (pin "CLKIN")
            ]);
            fuzz_one!(ctx, "CLKIN_IOB", "1", [
                (mutex "MODE", "PIN"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV"),
                (pin "CLKIN"),
                (pin "CLKFB"),
                (pin_from "CLKFB", PinFromKind::Bufg)
            ], [
                (pin_from "CLKIN", PinFromKind::Bufg, PinFromKind::Iob)
            ]);
            fuzz_one!(ctx, "CLKFB_IOB", "1", [
                (mutex "MODE", "PIN"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV"),
                (pin "CLKIN"),
                (pin "CLKFB"),
                (pin_from "CLKIN", PinFromKind::Bufg)
            ], [
                (pin_from "CLKFB", PinFromKind::Bufg, PinFromKind::Iob)
            ]);

            for attr in [
                "DCM_CLKDV_CLKFX_ALIGNMENT",
                "DCM_COM_PWC_FB_EN",
                "DCM_COM_PWC_REF_EN",
                "DCM_EXT_FB_EN",
                "DCM_INPUTMUX_EN",
                "DCM_LOCK_HIGH_B",
                "DCM_OPTINV_RST",
                "DCM_OPTINV_PSINCDEC",
                "DCM_OPTINV_SKEW_IN",
                "DCM_OPTINV_SKEW_RST",
                "DCM_OPTINV_PSEN",
                "DCM_PLL_RST_DCM",
                "DCM_POWERDOWN_COMMON_EN_B",
                "DCM_REG_PWRD_CFG",
                "DCM_SCANMODE",
                "DCM_UNUSED_TAPS_POWERDOWN",
                "DCM_USE_REG_READY",
                "DCM_VREG_ENABLE",
                "DCM_WAIT_PLL",
                "DFS_CFG_BYPASS",
                "DFS_EARLY_LOCK",
                "DFS_EN",
                "DFS_EN_RELRST_B",
                "DFS_FAST_UPDATE",
                "DFS_MPW_LOW",
                "DFS_MPW_HIGH",
                "DFS_OSC_ON_FX",
                "DFS_OUTPUT_PSDLY_ON_CONCUR",
                "DFS_PWRD_CLKIN_STOP_B",
                "DFS_PWRD_CLKIN_STOP_STICKY_B",
                "DFS_PWRD_REPLY_TIMES_OUT_B",
                "DFS_REF_ON_FX",
                "DFS_SYNC_TO_DLL",
                "DLL_PERIOD_LOCK_BY1",
                "DLL_PWRD_STICKY_B",
                "DLL_PWRD_ON_SCANMODE_B",
                "DLL_CLKFB_STOPPED_PWRD_EN_B",
                "DLL_CLKIN_STOPPED_PWRD_EN_B",
                "DLL_ZD1_PWC_EN",
                "DLL_PHASE_SHIFT_LOCK_BY1",
                "DLL_ETPP_HOLD",
                "DLL_ZD2_PWC_EN",
                "DLL_ZD2_EN",
                "DLL_FDBKLOST_EN",
                "DLL_DESKEW_LOCK_BY1",
                "DLL_ZD1_EN",
                "DLL_ZD2_JF_OVERFLOW_HOLD",
                "DLL_ZD1_JF_OVERFLOW_HOLD",
                "CLKIN_DIVIDE_BY_2",
                "DUTY_CYCLE_CORRECTION",
                "MUX_INV_PLL_CLK",
                "MUX_INV_TEST_CLK",
                "STARTUP_WAIT",
            ] {
                fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [
                    (mutex "MODE", "ATTR"),
                    (related TileRelation::Hclk(hclk_cmt),
                        (tile_mutex "ENABLE", "USE")),
                    (global_opt "GTS_CYCLE", "1"),
                    (global_opt "DONE_CYCLE", "1"),
                    (global_opt "LCK_CYCLE", "NOWAIT"),
                    (mode "DCM_ADV")
                ]);
            }

            for attr in [
                "DCM_CLKFB_IODLY_MUXINSEL",
                "DCM_CLKFB_IODLY_MUXOUT_SEL",
                "DCM_CLKIN_IODLY_MUXOUT_SEL",
            ] {
                fuzz_enum!(ctx, attr, ["PASS", "DELAY_LINE"], [
                    (mutex "MODE", "ATTR"),
                    (related TileRelation::Hclk(hclk_cmt),
                        (tile_mutex "ENABLE", "USE")),
                    (mode "DCM_ADV")
                ]);
            }
            fuzz_enum!(ctx, "DCM_CLKIN_IODLY_MUXINSEL", ["PASS", "DELAY_LINE"], [
                (mutex "MODE", "PIN"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV"),
                (pin "CLKIN"),
                (pin "CLKFB"),
                (pin_from "CLKIN", PinFromKind::Iob),
                (pin_from "CLKFB", PinFromKind::Bufg)
            ]);

            fuzz_enum!(ctx, "DCM_CLKLOST_EN", ["DISABLE", "ENABLE"], [
                (mutex "MODE", "ATTR"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV")
            ]);

            fuzz_enum!(ctx, "DFS_AVE_FREQ_SAMPLE_INTERVAL", ["1", "2", "3", "4", "5", "6", "7"], [
                (mutex "MODE", "ATTR"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV")
            ]);
            fuzz_enum!(ctx, "DFS_AVE_FREQ_GAIN", ["0.125", "0.25", "0.5", "1.0", "2.0", "4.0", "8.0"], [
                (mutex "MODE", "ATTR"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV")
            ]);
            fuzz_enum!(ctx, "DFS_FREQUENCY_MODE", ["LOW", "HIGH"], [
                (mutex "MODE", "ATTR"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV")
            ]);
            fuzz_enum!(ctx, "DLL_FREQUENCY_MODE", ["LOW", "HIGH"], [
                (mutex "MODE", "ATTR"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV")
            ]);
            fuzz_enum!(ctx, "DLL_PHASE_SHIFT_CALIBRATION", ["MASK", "CONFIG", "AUTO_ZD2", "AUTO_DPS"], [
                (mutex "MODE", "ATTR"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV"),
                (attr "DLL_FREQUENCY_MODE", "")
            ]);
            fuzz_enum!(ctx, "DLL_SYNTH_CLOCK_SPEED", ["VDD", "QUARTER", "HALF", "NORMAL"], [
                (mutex "MODE", "ATTR"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV")
            ]);
            fuzz_enum!(ctx, "DFS_OSCILLATOR_MODE", ["PHASE_FREQ_LOCK", "AVE_FREQ_LOCK"], [
                (mutex "MODE", "ATTR"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV"),
                (attr "DFS_EARLY_LOCK", ""),
                (attr "DFS_HARDSYNC_B", "")
            ]);
            fuzz_enum!(ctx, "DCM_VREF_SOURCE", ["VDD", "VBG_DLL", "VBG"], [
                (mutex "MODE", "ATTR"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV"),
                (attr "DCM_VBG_PD", ""),
                (attr "DCM_VBG_SEL", ""),
                (attr "DCM_PERFORMANCE_MODE", "")
            ]);
            fuzz_enum!(ctx, "DCM_PERFORMANCE_MODE", ["MAX_SPEED", "MAX_RANGE"], [
                (mutex "MODE", "ATTR"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV"),
                (attr "DCM_VBG_PD", ""),
                (attr "DCM_VBG_SEL", ""),
                (attr "DCM_VREF_SOURCE", "VBG_DLL")
            ]);

            for (attr, width) in [
                ("DCM_COMMON_MSB_SEL", 2),
                ("DCM_COM_PWC_FB_TAP", 3),
                ("DCM_COM_PWC_REF_TAP", 3),
                ("DCM_TRIM_CAL", 3),
                ("DCM_VBG_PD", 2),
                ("DCM_VBG_SEL", 4),
                ("DCM_VSPLY_VALID_ACC", 2),
                ("DFS_CLK_EN", 3),
                ("DFS_CUSTOM_FAST_SYNC", 4),
                ("DFS_HARDSYNC_B", 2),
                ("DFS_JF_LOWER_LIMIT", 4),
                ("DFS_HF_TRIM_CAL", 3),
                ("DFS_SYNTH_CLOCK_SPEED", 3),
                ("DFS_SYNTH_FAST_SYNCH", 2),
                ("DFS_TAPTRIM", 11),
                ("DFS_TWEAK", 8),
                ("DLL_CLK_EN", 7),
                ("DLL_TAPINIT_CTL", 3),
                ("DLL_TEST_MUX_SEL", 2),
                ("DLL_ZD1_PHASE_SEL_INIT", 2),
                ("DLL_ZD1_PWC_TAP", 3),
                ("DLL_ZD1_TAP_INIT", 8),
                ("DLL_ZD2_PWC_TAP", 3),
                ("DLL_ZD2_TAP_INIT", 7),
            ] {
                fuzz_multi_attr_bin!(ctx, attr, width, [
                    (mutex "MODE", "ATTR"),
                    (related TileRelation::Hclk(hclk_cmt),
                        (tile_mutex "ENABLE", "USE")),
                    (mode "DCM_ADV")
                ]);
            }
            for (attr, width) in [
                ("DESKEW_ADJUST", 5),
                ("DLL_DESKEW_MAXTAP", 8),
                ("DLL_DESKEW_MINTAP", 8),
                ("DLL_DEAD_TIME", 8),
                ("DLL_LIVE_TIME", 8),
                ("DLL_SETTLE_TIME", 8),
                ("DLL_PHASE_SHIFT_LFC", 9),
            ] {
                fuzz_multi_attr_dec!(ctx, attr, width, [
                    (mutex "MODE", "ATTR"),
                    (related TileRelation::Hclk(hclk_cmt),
                        (tile_mutex "ENABLE", "USE")),
                    (mode "DCM_ADV")
                ]);
            }
            fuzz_multi_attr_hex!(ctx, "FACTORY_JF", 16, [
                (mutex "MODE", "ATTR"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV"),
                (attr "DLL_FREQUENCY_MODE", "")
            ]);
            fuzz_multi_attr_dec_delta!(ctx, "CLKFX_DIVIDE", 5, 1, [
                (mutex "MODE", "ATTR"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV")
            ]);
            for val in 2..=32 {
                fuzz_one!(ctx, "CLKFX_MULTIPLY", format!("{val}"), [
                    (mutex "MODE", "ATTR"),
                    (related TileRelation::Hclk(hclk_cmt),
                        (tile_mutex "ENABLE", "USE")),
                    (mode "DCM_ADV")
                ], [
                    (attr "CLKFX_MULTIPLY", format!("{val}"))
                ]);
            }

            fuzz_enum!(ctx, "CLKDV_DIVIDE", ["2.0", "3.0", "4.0", "5.0", "6.0", "7.0", "8.0", "9.0", "10.0", "11.0", "12.0", "13.0", "14.0", "15.0", "16.0"], [
                (mutex "MODE", "PIN"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV")
            ]);
            for dll_mode in ["LOW", "HIGH"] {
                for val in ["1.5", "2.5", "3.5", "4.5", "5.5", "6.5", "7.5"] {
                    fuzz_one!(
                        ctx,
                        "CLKDV_DIVIDE",
                        format!("{val}.{dll_mode}"), [
                            (mutex "MODE", "PIN"),
                            (related TileRelation::Hclk(hclk_cmt),
                                (tile_mutex "ENABLE", "USE")),
                            (mode "DCM_ADV"),
                            (attr "DLL_FREQUENCY_MODE", dll_mode)
                        ], [
                            (attr "CLKDV_DIVIDE", val)
                        ]
                    );
                }
            }

            fuzz_enum!(ctx, "CLKOUT_PHASE_SHIFT", ["NONE", "FIXED", "VARIABLE_POSITIVE", "VARIABLE_CENTER", "DIRECT"], [
                (mutex "MODE", "ATTR"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV"),
                (attr "PHASE_SHIFT", "1"),
                (attr "DLL_ZD2_EN", ""),
                (nopin "CLK0"),
                (nopin "CLK90"),
                (nopin "CLK180"),
                (nopin "CLK270"),
                (nopin "CLK2X"),
                (nopin "CLK2X180"),
                (nopin "CLKDV")
            ]);
            fuzz_enum_suffix!(ctx, "CLKOUT_PHASE_SHIFT", "NEG", [
                "NONE",
                "FIXED",
                "VARIABLE_POSITIVE",
                "VARIABLE_CENTER",
                "DIRECT",
            ], [
                (mutex "MODE", "ATTR"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV"),
                (attr "PHASE_SHIFT", "-1"),
                (attr "DLL_ZD2_EN", ""),
                (nopin "CLK0"),
                (nopin "CLK90"),
                (nopin "CLK180"),
                (nopin "CLK270"),
                (nopin "CLK2X"),
                (nopin "CLK2X180"),
                (nopin "CLKDV")
            ]);
            fuzz_multi_attr_dec!(ctx, "PHASE_SHIFT", 10, [
                (mutex "MODE", "ATTR"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV")
            ]);
            fuzz_one!(ctx, "PHASE_SHIFT", "-1", [
                (mutex "MODE", "ATTR"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mode "DCM_ADV"),
                (attr "CLKOUT_PHASE_SHIFT", "NONE")
            ], [
                (attr "PHASE_SHIFT", "-1")
            ]);

            for (pin, opin) in [("CLKIN", "CLKFB"), ("CLKFB", "CLKIN")] {
                for i in 0..10 {
                    fuzz_one!(ctx, format!("MUX.{pin}"), format!("HCLK{i}"), [
                        (global_mutex "HCLK_CMT", "USE"),
                        (related TileRelation::Hclk(hclk_cmt),
                            (tile_mutex "ENABLE", "USE")),
                        (mode "DCM_ADV"),
                        (attr "CLK_FEEDBACK", "1X"),
                        (mutex "MUX.CLK", format!("{pin}.HCLK{i}")),
                        (pip (bel_pin bel_cmt, format!("HCLK{i}")), (pin opin))
                    ], [
                        (pip (bel_pin bel_cmt, format!("HCLK{i}")), (pin pin))
                    ]);
                }
                for i in 0..10 {
                    fuzz_one!(ctx, format!("MUX.{pin}"), format!("GIOB{i}"), [
                        (global_mutex "HCLK_CMT", "USE"),
                        (related TileRelation::Hclk(hclk_cmt),
                            (tile_mutex "ENABLE", "USE")),
                        (mode "DCM_ADV"),
                        (attr "CLK_FEEDBACK", "1X"),
                        (mutex "MUX.CLK", format!("{pin}.GIOB{i}")),
                        (pip (bel_pin bel_cmt, format!("GIOB{i}")), (pin opin))
                    ], [
                        (pip (bel_pin bel_cmt, format!("GIOB{i}")), (pin pin))
                    ]);
                }
                for i in 0..3 {
                    fuzz_one!(ctx, format!("MUX.{pin}"), format!("CKINT{i}"), [
                        (global_mutex "HCLK_CMT", "USE"),
                        (related TileRelation::Hclk(hclk_cmt),
                            (tile_mutex "ENABLE", "USE")),
                        (mode "DCM_ADV"),
                        (attr "CLK_FEEDBACK", "1X"),
                        (mutex "MUX.CLK", format!("{pin}.CKINT{i}"))
                    ], [
                        (pip (pin format!("CKINT{i}")), (pin pin))
                    ]);
                }
                fuzz_one!(ctx, format!("MUX.{pin}"), "CLK_FROM_PLL", [
                    (global_mutex "HCLK_CMT", "USE"),
                    (related TileRelation::Hclk(hclk_cmt),
                        (tile_mutex "ENABLE", "USE")),
                    (mode "DCM_ADV"),
                    (attr "CLK_FEEDBACK", "1X"),
                    (bel_unused bel_pll),
                    (mutex "MUX.CLK", format!("{pin}.CLK_FROM_PLL"))
                ], [
                    (pip (bel_pin bel_pll, format!("CLK_TO_DCM{i}")), (pin pin))
                ]);
            }

            for (idx, pin) in [
                "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX",
                "CLKFX180", "CONCUR",
            ]
            .into_iter()
            .enumerate()
            {
                let idx = i * 18 + idx;
                fuzz_one!(ctx, "MUX.CLK_TO_PLL", pin, [
                    (mutex "MUX.CLK_TO_PLL", pin)
                ], [
                    (pip (bel_pin bel_cmt, format!("OUT{idx}")), (pin "MUXED_CLK"))
                ]);
                fuzz_one!(ctx, "MUX.SKEWCLKIN2", pin, [
                    (mutex "MUX.SKEWCLKIN2", pin)
                ], [
                    (pip (bel_pin bel_cmt, format!("OUT{idx}_TEST")), (pin "SKEWCLKIN2"))
                ]);
            }
        }
    }
    if !skip_pll {
        let ctx = FuzzCtx::new(session, backend, "CMT", "PLL", TileBits::MainAuto);
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::Hclk(0, 0),
            "HCLK_CMT",
            "HCLK_CMT",
            "DRP_MASK",
            "1",
        )];
        fuzz_one_extras!(ctx, "ENABLE", "1", [
            (global_xy "PLLADV_*_USE_CALC", "NO")
        ], [
            (related TileRelation::Hclk(hclk_cmt),
                (tile_mutex "ENABLE")),
            (mode "PLL_ADV")
        ], extras);

        for pin in [
            "CLKBRST",
            "CLKINSEL",
            "ENOUTSYNC",
            "MANPDLF",
            "MANPULF",
            "REL",
            "RST",
            "SKEWCLKIN1",
            "SKEWCLKIN2",
            "SKEWRST",
            "SKEWSTB",
        ] {
            fuzz_inv!(ctx, pin, [
                (mutex "MODE", "INV"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (global_xy "PLLADV_*_USE_CALC", "NO"),
                (mode "PLL_ADV")
            ]);
        }

        for attr in [
            "PLL_CLKBURST_ENABLE",
            "PLL_CP_BIAS_TRIP_SHIFT",
            "PLL_DIRECT_PATH_CNTRL",
            "PLL_EN_DLY",
            "PLL_INC_FLOCK",
            "PLL_INC_SLOCK",
            "PLL_LOCK_CNT_RST_FAST",
            "PLL_MAN_LF_EN",
            "PLL_NBTI_EN",
            "PLL_PMCD_MODE",
            "PLL_PWRD_CFG",
            "PLL_SEL_SLIPD",
            "PLL_UNLOCK_CNT_RST_FAST",
            "PLL_VLFHIGH_DIS",
            "PLL_EN_TCLK0",
            "PLL_EN_TCLK1",
            "PLL_EN_TCLK2",
            "PLL_EN_TCLK3",
            "PLL_EN_TCLK4",
            "PLL_EN_VCO0",
            "PLL_EN_VCO1",
            "PLL_EN_VCO2",
            "PLL_EN_VCO3",
            "PLL_EN_VCO4",
            "PLL_EN_VCO5",
            "PLL_EN_VCO6",
            "PLL_EN_VCO7",
            "PLL_EN_VCO_DIV1",
            "PLL_EN_VCO_DIV6",
            "PLL_CLKOUT0_EN",
            "PLL_CLKOUT1_EN",
            "PLL_CLKOUT2_EN",
            "PLL_CLKOUT3_EN",
            "PLL_CLKOUT4_EN",
            "PLL_CLKOUT5_EN",
            "PLL_CLKFBOUT_EN",
            "PLL_CLKOUT0_EDGE",
            "PLL_CLKOUT1_EDGE",
            "PLL_CLKOUT2_EDGE",
            "PLL_CLKOUT3_EDGE",
            "PLL_CLKOUT4_EDGE",
            "PLL_CLKOUT5_EDGE",
            "PLL_CLKFBOUT_EDGE",
            "PLL_CLKFBOUT2_EDGE",
            "PLL_DIVCLK_EDGE",
            "PLL_CLKOUT0_NOCOUNT",
            "PLL_CLKOUT1_NOCOUNT",
            "PLL_CLKOUT2_NOCOUNT",
            "PLL_CLKOUT3_NOCOUNT",
            "PLL_CLKOUT4_NOCOUNT",
            "PLL_CLKOUT5_NOCOUNT",
            "PLL_CLKFBOUT_NOCOUNT",
            "PLL_CLKFBOUT2_NOCOUNT",
            "PLL_DIVCLK_NOCOUNT",
        ] {
            fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [
                (mutex "MODE", "TEST"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (global_xy "PLLADV_*_USE_CALC", "NO"),
                (mode "PLL_ADV")
            ]);
        }
        for (attr, width) in [
            ("PLL_UNLOCK_CNT", 4),
            ("PLL_AVDD_COMP_SET", 2),
            ("PLL_DVDD_COMP_SET", 2),
            ("PLL_INTFB", 2),
            ("PLL_RES", 4),
            ("PLL_CP", 4),
            ("PLL_CP_RES", 2),
            ("PLL_LFHF", 2),
            ("PLL_AVDD_VBG_PD", 2),
            ("PLL_DVDD_VBG_PD", 2),
            ("PLL_AVDD_VBG_SEL", 4),
            ("PLL_DVDD_VBG_SEL", 4),
            ("PLL_LF_NEN", 2),
            ("PLL_LF_PEN", 2),
            ("PLL_PFD_CNTRL", 4),
            ("PLL_CLKCNTRL", 1),
            ("PLL_TCK4_SEL", 1),
            ("PLL_PFD_DLY", 2),
            ("PLL_CLKBURST_CNT", 3),
            ("PLL_LOCK_CNT", 6),
            ("PLL_CLK0MX", 2),
            ("PLL_CLK1MX", 2),
            ("PLL_CLK2MX", 2),
            ("PLL_CLK3MX", 2),
            ("PLL_CLK4MX", 2),
            ("PLL_CLK5MX", 2),
            ("PLL_CLKFBMX", 2),
            ("CLKOUT0_DESKEW_ADJUST", 5),
            ("CLKOUT1_DESKEW_ADJUST", 5),
            ("CLKOUT2_DESKEW_ADJUST", 5),
            ("CLKOUT3_DESKEW_ADJUST", 5),
            ("CLKOUT4_DESKEW_ADJUST", 5),
            ("CLKOUT5_DESKEW_ADJUST", 5),
            ("CLKFBOUT_DESKEW_ADJUST", 5),
        ] {
            fuzz_multi_attr_dec!(ctx, attr, width, [
                (mutex "MODE", "TEST"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (global_xy "PLLADV_*_USE_CALC", "NO"),
                (mode "PLL_ADV")
            ]);
        }
        for (attr, width) in [
            ("PLL_EN_CNTRL", 78),
            ("PLL_FLOCK", 6),
            ("PLL_IN_DLY_MX_SEL", 5),
            ("PLL_IN_DLY_SET", 9),
            ("PLL_LOCK_FB_P1", 5),
            ("PLL_LOCK_FB_P2", 5),
            ("PLL_LOCK_REF_P1", 5),
            ("PLL_LOCK_REF_P2", 5),
            ("PLL_MISC", 4),
        ] {
            fuzz_multi_attr_bin!(ctx, attr, width, [
                (mutex "MODE", "TEST"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (global_xy "PLLADV_*_USE_CALC", "NO"),
                (mode "PLL_ADV")
            ]);
        }

        for out in [
            "CLKOUT0",
            "CLKOUT1",
            "CLKOUT2",
            "CLKOUT3",
            "CLKOUT4",
            "CLKOUT5",
            "CLKFBOUT",
            "CLKFBOUT2",
            "DIVCLK",
        ] {
            for at in ["DT", "HT", "LT"] {
                fuzz_multi_attr_bin!(ctx, format!("PLL_{out}_{at}"), 6, [
                    (mutex "MODE", "TEST"),
                    (related TileRelation::Hclk(hclk_cmt),
                        (tile_mutex "ENABLE", "USE")),
                    (global_xy "PLLADV_*_USE_CALC", "NO"),
                    (mode "PLL_ADV")
                ]);
            }
        }
        for out in [
            "CLKOUT0", "CLKOUT1", "CLKOUT2", "CLKOUT3", "CLKOUT4", "CLKOUT5", "CLKFBOUT",
        ] {
            fuzz_multi_attr_bin!(ctx, format!("PLL_{out}_PM"), 3, [
                (mutex "MODE", "TEST"),
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (global_xy "PLLADV_*_USE_CALC", "NO"),
                (mode "PLL_ADV")
            ]);
        }
        fuzz_enum!(ctx, "COMPENSATION", ["SOURCE_SYNCHRONOUS", "SYSTEM_SYNCHRONOUS", "PLL2DCM", "DCM2PLL", "EXTERNAL", "INTERNAL"], [
            (mutex "MODE", "COMP"),
            (related TileRelation::Hclk(hclk_cmt),
                (tile_mutex "ENABLE", "USE")),
            (global_xy "PLLADV_*_USE_CALC", "YES"),
            (mode "PLL_ADV")
        ]);

        for mult in 1..=64 {
            for bandwidth in ["LOW", "HIGH"] {
                fuzz_one!(ctx, "TABLES", format!("{mult}.{bandwidth}"), [
                    (related TileRelation::Hclk(hclk_cmt),
                        (tile_mutex "ENABLE", "USE")),
                    (mutex "MODE", "CALC"),
                    (global_xy "PLLADV_*_USE_CALC", "NO"),
                    (mode "PLL_ADV")
                ], [
                    (attr_diff "CLKFBOUT_MULT", "0", format!("{mult}")),
                    (attr_diff "BANDWIDTH", "LOW", bandwidth)
                ]);
            }
        }

        for (pin, opin) in [("CLKIN1", "CLKFBIN"), ("CLKFBIN", "CLKIN1")] {
            for i in 0..10 {
                fuzz_one!(ctx, format!("MUX.{pin}"), format!("HCLK{i}"), [
                    (global_mutex "HCLK_CMT", "USE"),
                    (related TileRelation::Hclk(hclk_cmt),
                        (tile_mutex "ENABLE", "USE")),
                    (mutex "MODE", "CALC"),
                    (mode "PLL_ADV"),
                    (mutex "MUX.CLK", format!("{pin}.HCLK{i}")),
                    (pip (bel_pin bel_cmt, format!("HCLK{i}")), (pin opin))
                ], [
                    (pip (bel_pin bel_cmt, format!("HCLK{i}")), (pin pin))
                ]);
            }
            for i in 0..10 {
                fuzz_one!(ctx, format!("MUX.{pin}"), format!("GIOB{i}"), [
                    (global_mutex "HCLK_CMT", "USE"),
                    (related TileRelation::Hclk(hclk_cmt),
                        (tile_mutex "ENABLE", "USE")),
                    (mutex "MODE", "CALC"),
                    (mode "PLL_ADV"),
                    (mutex "MUX.CLK", format!("{pin}.GIOB{i}")),
                    (pip (bel_pin bel_cmt, format!("GIOB{i}")), (pin opin))
                ], [
                    (pip (bel_pin bel_cmt, format!("GIOB{i}")), (pin pin))
                ]);
            }
            fuzz_one!(ctx, format!("MUX.{pin}"), "CLKFBDCM", [
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (mutex "MODE", "CALC"),
                (mode "PLL_ADV"),
                (mutex "MUX.CLK", format!("{pin}.CLKFBDCM")),
                (pip (pin "CLKFBDCM"), (pin opin))
            ], [
                (pip (pin "CLKFBDCM"), (pin pin))
            ]);
        }
        fuzz_one!(ctx, "MUX.CLKIN1", "CKINT0", [
            (related TileRelation::Hclk(hclk_cmt),
                (tile_mutex "ENABLE", "USE")),
            (mutex "MODE", "CALC"),
            (mode "PLL_ADV"),
            (mutex "MUX.CLK", "CLKIN1.CKINT0"),
            (pip (pin "CLKFBDCM"), (pin "CLKFBIN"))
        ], [
            (pip (pin "CKINT0"), (pin "CLK_DCM_MUX")),
            (pip (pin "CLK_DCM_MUX"), (pin "CLKIN1"))
        ]);
        fuzz_one!(ctx, "MUX.CLKIN1", "CLK_FROM_DCM0", [
            (related TileRelation::Hclk(hclk_cmt),
                (tile_mutex "ENABLE", "USE")),
            (mutex "MODE", "CALC"),
            (mode "PLL_ADV"),
            (mutex "MUX.CLK", "CLKIN1.CLK_FROM_DCM0"),
            (pip (pin "CLKFBDCM"), (pin "CLKFBIN"))
        ], [
            (pip (bel_pin bel_dcm0, "MUXED_CLK"), (pin "CLK_DCM_MUX")),
            (pip (pin "CLK_DCM_MUX"), (pin "CLKIN1"))
        ]);
        fuzz_one!(ctx, "MUX.CLKIN1", "CLK_FROM_DCM1", [
            (related TileRelation::Hclk(hclk_cmt),
                (tile_mutex "ENABLE", "USE")),
            (mutex "MODE", "CALC"),
            (mode "PLL_ADV"),
            (mutex "MUX.CLK", "CLKIN1.CLK_FROM_DCM1"),
            (pip (pin "CLKFBDCM"), (pin "CLKFBIN"))
        ], [
            (pip (bel_pin bel_dcm1, "MUXED_CLK"), (pin "CLK_DCM_MUX")),
            (pip (pin "CLK_DCM_MUX"), (pin "CLKIN1"))
        ]);
        fuzz_one!(ctx, "MUX.CLKFBIN", "CKINT1", [
            (related TileRelation::Hclk(hclk_cmt),
                (tile_mutex "ENABLE", "USE")),
            (mutex "MODE", "CALC"),
            (mode "PLL_ADV"),
            (mutex "MUX.CLK", "CLKFBIN.CKINT1"),
            (pip (pin "CLKFBDCM"), (pin "CLKIN1"))
        ], [
            (pip (pin "CKINT1"), (pin "CLK_FB_FROM_DCM")),
            (pip (pin "CLK_FB_FROM_DCM"), (pin "CLKFBIN"))
        ]);
        fuzz_one!(ctx, "MUX.CLKFBIN", "CLKFBOUT", [
            (related TileRelation::Hclk(hclk_cmt),
                (tile_mutex "ENABLE", "USE")),
            (mutex "MODE", "CALC"),
            (mode "PLL_ADV"),
            (mutex "MUX.CLK", "CLKFBIN.CLKFBOUT"),
            (pip (pin "CLKFBDCM"), (pin "CLKIN1"))
        ], [
            (pip (bel_pin bel_cmt, "OUT11"), (pin "CLK_FB_FROM_DCM")),
            (pip (pin "CLK_FB_FROM_DCM"), (pin "CLKFBIN"))
        ]);
        fuzz_one!(ctx, "CLKINSEL_MODE", "DYNAMIC", [
            (global_mutex "HCLK_CMT", "USE"),
            (related TileRelation::Hclk(hclk_cmt),
                (tile_mutex "ENABLE", "USE")),
            (mutex "MODE", "CALC"),
            (mode "PLL_ADV"),
            (mutex "MUX.CLK", "CLKIN2"),
            (pip (bel_pin bel_cmt, "GIOB5"), (pin "CLKFBIN"))
        ], [
            (pip (bel_pin bel_cmt, "GIOB5"), (pin "CLKIN2"))
        ]);

        for opin in ["CLK_TO_DCM0", "CLK_TO_DCM1"] {
            for i in 0..6 {
                fuzz_one!(ctx, format!("MUX.{opin}"), format!("CLKOUTDCM{i}"), [
                    (related TileRelation::Hclk(hclk_cmt),
                        (tile_mutex "ENABLE", "USE")),
                    (mode "PLL_ADV"),
                    (mutex format!("MUX.{opin}"), format!("CLKOUTDCM{i}"))
                ], [
                    (pip (pin format!("CLKOUTDCM{i}")), (pin opin))
                ]);
            }
        }
        fuzz_one!(ctx, "MUX.CLK_TO_DCM1", "CLKFBDCM", [
            (related TileRelation::Hclk(hclk_cmt),
                (tile_mutex "ENABLE", "USE")),
            (mode "PLL_ADV"),
            (mutex "MUX.CLK_TO_DCM1", "CLKFBDCM")
        ], [
            (pip (pin "CLKFBDCM_TEST"), (pin "CLK_TO_DCM1"))
        ]);
    }
    {
        let ctx = FuzzCtx::new(session, backend, "CMT", "CMT", TileBits::MainAuto);
        for (name, bel, pin) in [
            ("DCM0_CLKFB", bel_dcm0, "CLKFB_TEST"),
            ("DCM0_CLKIN", bel_dcm0, "CLKIN_TEST"),
            ("DCM1_CLKFB", bel_dcm1, "CLKFB_TEST"),
            ("DCM1_CLKIN", bel_dcm1, "CLKIN_TEST"),
            ("PLL_CLKIN", bel_pll, "CLKIN1_TEST"),
            ("PLL_CLKINFB", bel_pll, "CLKINFB_TEST"),
        ] {
            fuzz_one!(ctx, "MUX.OUT10", name, [
                (related TileRelation::Hclk(hclk_cmt),
                    (tile_mutex "ENABLE", "USE")),
                (bel_mode bel_pll, "PLL_ADV"),
                (mutex "MUX.OUT10", name)
            ], [
                (pip (bel_pin bel, pin), (pin "OUT10"))
            ]);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, skip_dcm: bool, skip_pll: bool, devdata_only: bool) {
    let tile = "CMT";
    if devdata_only {
        let bel = "PLL";
        let mut enable = ctx.state.get_diff(tile, bel, "ENABLE", "1");
        let dly_val = extract_bitvec_val_part(
            ctx.tiledb.item(tile, bel, "PLL_IN_DLY_SET"),
            &bitvec![0; 9],
            &mut enable,
        );
        ctx.insert_device_data("PLL:PLL_IN_DLY_SET", dly_val);
        let tile = "HCLK_CMT";
        let bel = "HCLK_CMT";
        ctx.collect_bit(tile, bel, "DRP_MASK", "1");
        return;
    }
    {
        let bel = "CMT";
        ctx.collect_enum_default_ocd(
            tile,
            bel,
            "MUX.OUT10",
            &[
                "DCM0_CLKFB",
                "DCM0_CLKIN",
                "DCM1_CLKFB",
                "DCM1_CLKIN",
                "PLL_CLKIN",
                "PLL_CLKINFB",
            ],
            "NONE",
            OcdMode::Mux,
        );
    }
    if !skip_dcm {
        for i in 0..2 {
            let bel = &format!("DCM{i}");
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
            for reg in 0x40..0x58 {
                ctx.tiledb.insert(
                    tile,
                    bel,
                    format!("DRP{reg:02X}"),
                    TileItem::from_bitvec(
                        (0..16).map(|bit| dcm_drp_bit(i, reg, bit)).collect(),
                        false,
                    ),
                );
            }

            for pin in [
                "PSEN",
                "PSINCDEC",
                "RST",
                "SKEWCLKIN1",
                "SKEWCLKIN2",
                "SKEWIN",
                "SKEWRST",
            ] {
                ctx.collect_inv(tile, bel, pin);
            }
            for attr in [
                "DCM_CLKDV_CLKFX_ALIGNMENT",
                "DCM_COM_PWC_FB_EN",
                "DCM_COM_PWC_REF_EN",
                "DCM_EXT_FB_EN",
                "DCM_LOCK_HIGH_B",
                "DCM_PLL_RST_DCM",
                "DCM_POWERDOWN_COMMON_EN_B",
                "DCM_REG_PWRD_CFG",
                "DCM_SCANMODE",
                "DCM_USE_REG_READY",
                "DCM_VREG_ENABLE",
                "DCM_WAIT_PLL",
                "DFS_CFG_BYPASS",
                "DFS_EARLY_LOCK",
                "DFS_EN",
                "DFS_EN_RELRST_B",
                "DFS_FAST_UPDATE",
                "DFS_MPW_LOW",
                "DFS_MPW_HIGH",
                "DFS_OSC_ON_FX",
                "DFS_OUTPUT_PSDLY_ON_CONCUR",
                "DFS_PWRD_CLKIN_STOP_B",
                "DFS_PWRD_CLKIN_STOP_STICKY_B",
                "DFS_PWRD_REPLY_TIMES_OUT_B",
                "DFS_REF_ON_FX",
                "DFS_SYNC_TO_DLL",
                "DLL_CLKFB_STOPPED_PWRD_EN_B",
                "DLL_CLKIN_STOPPED_PWRD_EN_B",
                "DLL_DESKEW_LOCK_BY1",
                "DLL_ETPP_HOLD",
                "DLL_FDBKLOST_EN",
                "DLL_PERIOD_LOCK_BY1",
                "DLL_PHASE_SHIFT_LOCK_BY1",
                "DLL_PWRD_STICKY_B",
                "DLL_PWRD_ON_SCANMODE_B",
                "DLL_ZD1_EN",
                "DLL_ZD1_JF_OVERFLOW_HOLD",
                "DLL_ZD1_PWC_EN",
                "DLL_ZD2_EN",
                "DLL_ZD2_JF_OVERFLOW_HOLD",
                "DLL_ZD2_PWC_EN",
                "CLKIN_DIVIDE_BY_2",
                "STARTUP_WAIT",
            ] {
                ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
            }
            for (attr, pin) in [
                ("DCM_OPTINV_RST", "RST"),
                ("DCM_OPTINV_PSINCDEC", "PSINCDEC"),
                ("DCM_OPTINV_PSEN", "PSEN"),
                ("DCM_OPTINV_SKEW_IN", "SKEWIN"),
                ("DCM_OPTINV_SKEW_RST", "SKEWRST"),
                ("MUX_INV_PLL_CLK", "SKEWCLKIN1"),
                ("MUX_INV_TEST_CLK", "SKEWCLKIN2"),
            ] {
                let item = ctx.extract_enum_bool(tile, bel, attr, "FALSE", "TRUE");
                ctx.tiledb.insert(tile, bel, format!("INV.{pin}"), item);
            }
            ctx.collect_enum_bool(tile, bel, "DCM_CLKLOST_EN", "DISABLE", "ENABLE");
            ctx.collect_enum_bool_wide(tile, bel, "DCM_UNUSED_TAPS_POWERDOWN", "FALSE", "TRUE");
            ctx.collect_enum(tile, bel, "DFS_FREQUENCY_MODE", &["HIGH", "LOW"]);
            ctx.collect_enum(
                tile,
                bel,
                "DLL_SYNTH_CLOCK_SPEED",
                &["NORMAL", "HALF", "QUARTER", "VDD"],
            );
            ctx.collect_enum_int(tile, bel, "DFS_AVE_FREQ_SAMPLE_INTERVAL", 1..8, 0);
            ctx.collect_enum(
                tile,
                bel,
                "DFS_AVE_FREQ_GAIN",
                &["0.125", "0.25", "0.5", "1.0", "2.0", "4.0", "8.0"],
            );
            ctx.collect_enum(
                tile,
                bel,
                "DLL_PHASE_SHIFT_CALIBRATION",
                &["AUTO_DPS", "CONFIG", "MASK", "AUTO_ZD2"],
            );
            for attr in [
                "DCM_COMMON_MSB_SEL",
                "DCM_COM_PWC_FB_TAP",
                "DCM_COM_PWC_REF_TAP",
                "DCM_TRIM_CAL",
                "DCM_VBG_PD",
                "DCM_VBG_SEL",
                "DCM_VSPLY_VALID_ACC",
                "DFS_CUSTOM_FAST_SYNC",
                "DFS_HARDSYNC_B",
                "DFS_JF_LOWER_LIMIT",
                "DFS_HF_TRIM_CAL",
                "DFS_SYNTH_CLOCK_SPEED",
                "DFS_SYNTH_FAST_SYNCH",
                "DFS_TAPTRIM",
                "DFS_TWEAK",
                "DLL_TAPINIT_CTL",
                "DLL_TEST_MUX_SEL",
                "DLL_ZD1_PHASE_SEL_INIT",
                "DLL_ZD1_PWC_TAP",
                "DLL_ZD1_TAP_INIT",
                "DLL_ZD2_PWC_TAP",
                "DLL_ZD2_TAP_INIT",
                "DLL_DESKEW_MAXTAP",
                "DLL_DESKEW_MINTAP",
                "DLL_DEAD_TIME",
                "DLL_LIVE_TIME",
                "DLL_SETTLE_TIME",
                "DLL_PHASE_SHIFT_LFC",
                "FACTORY_JF",
                "DESKEW_ADJUST",
                "PHASE_SHIFT",
            ] {
                ctx.collect_bitvec(tile, bel, attr, "");
            }
            let mut diff = ctx.state.get_diff(tile, bel, "PHASE_SHIFT", "-1");
            diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PHASE_SHIFT"), 1, 0);
            ctx.tiledb
                .insert(tile, bel, "PHASE_SHIFT_NEGATIVE", xlat_bit(diff));

            for attr in [
                "DCM_CLKFB_IODLY_MUXINSEL",
                "DCM_CLKIN_IODLY_MUXINSEL",
                "DCM_CLKFB_IODLY_MUXOUT_SEL",
                "DCM_CLKIN_IODLY_MUXOUT_SEL",
            ] {
                ctx.collect_enum(tile, bel, attr, &["PASS", "DELAY_LINE"]);
            }

            let clkfx_d = ctx.extract_bitvec(tile, bel, "CLKFX_DIVIDE", "");
            let clkfx_m = ctx.extract_enum_int(tile, bel, "CLKFX_MULTIPLY", 1..32, 1);
            let clkfx_divide: Vec<_> = (0..8).map(|bit| dcm_drp_bit(i, 0x50, bit)).collect();
            let clkfx_multiply: Vec<_> = (0..8).map(|bit| dcm_drp_bit(i, 0x50, bit + 8)).collect();
            assert_eq!(clkfx_d.bits, clkfx_divide[0..5]);
            assert_eq!(clkfx_m.bits, clkfx_multiply[0..5]);
            ctx.tiledb.insert(
                tile,
                bel,
                "CLKFX_DIVIDE",
                TileItem::from_bitvec(clkfx_divide, false),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "CLKFX_MULTIPLY",
                TileItem::from_bitvec(clkfx_multiply, false),
            );

            let mut diff_low = ctx.state.get_diff(tile, bel, "DLL_FREQUENCY_MODE", "LOW");
            let mut diff_high = ctx.state.get_diff(tile, bel, "DLL_FREQUENCY_MODE", "HIGH");
            diff_low.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "FACTORY_JF"), 0xc080, 0);
            diff_high.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "FACTORY_JF"), 0xf0f0, 0);
            diff_high.apply_enum_diff(
                ctx.tiledb.item(tile, bel, "DLL_PHASE_SHIFT_CALIBRATION"),
                "AUTO_ZD2",
                "AUTO_DPS",
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "DLL_FREQUENCY_MODE",
                xlat_enum(vec![("LOW", diff_low), ("HIGH", diff_high)]),
            );

            ctx.state
                .get_diff(tile, bel, "DUTY_CYCLE_CORRECTION", "FALSE")
                .assert_empty();
            ctx.state
                .get_diff(tile, bel, "DUTY_CYCLE_CORRECTION", "TRUE")
                .assert_empty();
            ctx.state
                .get_diff(tile, bel, "DCM_INPUTMUX_EN", "FALSE")
                .assert_empty();
            ctx.state
                .get_diff(tile, bel, "DCM_INPUTMUX_EN", "TRUE")
                .assert_empty();

            for (attr, bits) in [
                ("CLKDV_COUNT_MAX", 0..4),
                ("CLKDV_COUNT_FALL", 4..8),
                ("CLKDV_COUNT_FALL_2", 8..12),
                ("CLKDV_PHASE_RISE", 12..14),
                ("CLKDV_PHASE_FALL", 14..16),
            ] {
                let bits = Vec::from_iter(bits.map(|bit| dcm_drp_bit(i, 0x53, bit)));
                ctx.tiledb
                    .insert(tile, bel, attr, TileItem::from_bitvec(bits, false));
            }
            ctx.tiledb.insert(
                tile,
                bel,
                "CLKDV_MODE",
                TileItem {
                    bits: vec![dcm_drp_bit(i, 0x54, 0)],
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
                let mut diff =
                    ctx.collector
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
                diff.apply_enum_diff(clkdv_mode, "HALF", "INT");
                diff.apply_bitvec_diff_int(clkdv_count_max, 2 * i, 1);
                diff.apply_bitvec_diff_int(clkdv_count_fall, (i - 1) / 2, 0);
                diff.apply_bitvec_diff_int(clkdv_count_fall_2, (3 * i + 1) / 2, 0);
                diff.apply_bitvec_diff_int(clkdv_phase_fall, (i % 2) * 2, 0);
                diff.assert_empty();
            }

            for (pin, diff) in ["CLKFX", "CLKFX180", "CONCUR"]
                .into_iter()
                .zip(ctx.state.get_diffs(tile, bel, "DFS_CLK_EN", ""))
            {
                let mut diff2 = ctx.state.get_diff(tile, bel, pin, "1");
                diff2.apply_bit_diff(ctx.tiledb.item(tile, bel, "DFS_EN"), true, false);
                assert_eq!(diff, diff2);
                ctx.tiledb
                    .insert(tile, bel, format!("ENABLE.{pin}"), xlat_bit(diff));
            }

            for (pin, diff) in [
                "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV",
            ]
            .into_iter()
            .zip(ctx.state.get_diffs(tile, bel, "DLL_CLK_EN", ""))
            {
                let mut diff2 = ctx.state.get_diff(tile, bel, pin, "1");
                diff2.apply_bit_diff(ctx.tiledb.item(tile, bel, "DLL_ZD2_EN"), true, false);
                assert_eq!(diff, diff2);
                ctx.tiledb
                    .insert(tile, bel, format!("ENABLE.{pin}"), xlat_bit(diff));
            }

            let mut diff = ctx.state.get_diff(tile, bel, "CLKIN_ENABLE", "1");
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "DCM_CLKLOST_EN"), true, false);
            diff.assert_empty();
            let mut diff = ctx.state.get_diff(tile, bel, "CLKFB_ENABLE", "1");
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "DLL_FDBKLOST_EN"), true, false);
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "DLL_ZD1_EN"), true, false);
            diff.assert_empty();

            let mut diff = ctx
                .state
                .get_diff(tile, bel, "DCM_PERFORMANCE_MODE", "MAX_RANGE");
            diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DCM_VBG_SEL"), 9, 0);
            diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DCM_VBG_PD"), 2, 0);
            diff.assert_empty();
            let mut diff = ctx
                .state
                .get_diff(tile, bel, "DCM_PERFORMANCE_MODE", "MAX_SPEED");
            diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DCM_VBG_SEL"), 0xc, 0);
            diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DCM_VBG_PD"), 3, 0);
            diff.assert_empty();
            let mut diff = ctx.state.get_diff(tile, bel, "DCM_VREF_SOURCE", "VBG");
            diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DCM_VBG_SEL"), 1, 0);
            diff.assert_empty();
            ctx.state
                .get_diff(tile, bel, "DCM_VREF_SOURCE", "VBG_DLL")
                .assert_empty();
            ctx.state
                .get_diff(tile, bel, "DCM_VREF_SOURCE", "VDD")
                .assert_empty();

            let mut diff = ctx
                .state
                .get_diff(tile, bel, "DFS_OSCILLATOR_MODE", "AVE_FREQ_LOCK");
            diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DFS_HARDSYNC_B"), 3, 0);
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "DFS_EARLY_LOCK"), true, false);
            let item = xlat_enum(vec![
                (
                    "PHASE_FREQ_LOCK",
                    ctx.state
                        .get_diff(tile, bel, "DFS_OSCILLATOR_MODE", "PHASE_FREQ_LOCK"),
                ),
                ("AVE_FREQ_LOCK", diff),
            ]);
            ctx.tiledb.insert(tile, bel, "DFS_OSCILLATOR_MODE", item);

            let mut diff = ctx.state.get_diff(tile, bel, "CLKIN_IOB", "1");
            diff.apply_enum_diff(
                ctx.tiledb.item(tile, bel, "DCM_CLKFB_IODLY_MUXOUT_SEL"),
                "DELAY_LINE",
                "PASS",
            );
            diff.apply_enum_diff(
                ctx.tiledb.item(tile, bel, "DCM_CLKIN_IODLY_MUXINSEL"),
                "PASS",
                "DELAY_LINE",
            );
            diff.apply_enum_diff(
                ctx.tiledb.item(tile, bel, "DCM_CLKFB_IODLY_MUXINSEL"),
                "DELAY_LINE",
                "PASS",
            );
            diff.assert_empty();

            let mut diff = ctx.state.get_diff(tile, bel, "CLKFB_IOB", "1");
            diff.apply_enum_diff(
                ctx.tiledb.item(tile, bel, "DCM_CLKIN_IODLY_MUXOUT_SEL"),
                "DELAY_LINE",
                "PASS",
            );
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "DCM_EXT_FB_EN"), true, false);
            diff.assert_empty();

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
                let item = ctx.tiledb.item(tile, bel, "PS_MODE");
                d.apply_enum_diff(item, "CLKIN", "CLKFB");
                if val != "FIXED" {
                    dn.apply_enum_diff(item, "CLKIN", "CLKFB");
                }
                assert_eq!(d, dn);
                if val != "NONE" && val != "DIRECT" {
                    let item = ctx.tiledb.item(tile, bel, "DLL_ZD2_EN");
                    d.apply_bit_diff(item, true, false);
                }
                match val {
                    "NONE" => d.assert_empty(),
                    "FIXED" | "VARIABLE_POSITIVE" => {
                        ctx.tiledb.insert(tile, bel, "PS_ENABLE", xlat_bit(d))
                    }
                    "VARIABLE_CENTER" => {
                        d.apply_bit_diff(ctx.tiledb.item(tile, bel, "PS_ENABLE"), true, false);
                        ctx.tiledb.insert(tile, bel, "PS_CENTERED", xlat_bit(d));
                    }
                    "DIRECT" => {
                        d.apply_bit_diff(ctx.tiledb.item(tile, bel, "PS_ENABLE"), true, false);
                        ctx.tiledb.insert(tile, bel, "PS_DIRECT", xlat_bit(d));
                    }
                    _ => unreachable!(),
                }
            }

            let (_, _, diff) = Diff::split(
                ctx.state
                    .peek_diff(tile, bel, "MUX.CLKIN", "CKINT0")
                    .clone(),
                ctx.state
                    .peek_diff(tile, bel, "MUX.CLKFB", "CKINT0")
                    .clone(),
            );
            let en = xlat_bit(diff);
            for attr in ["MUX.CLKIN", "MUX.CLKFB"] {
                let mut diffs = vec![];
                for i in 0..10 {
                    diffs.push((
                        format!("GIOB{i}"),
                        ctx.state.get_diff(tile, bel, attr, format!("GIOB{i}")),
                    ));
                }
                for i in 0..10 {
                    diffs.push((
                        format!("HCLK{i}"),
                        ctx.state.get_diff(tile, bel, attr, format!("HCLK{i}")),
                    ));
                }
                for i in 0..3 {
                    let mut diff = ctx.state.get_diff(tile, bel, attr, format!("CKINT{i}"));
                    diff.apply_bit_diff(&en, true, false);
                    diffs.push((format!("CKINT{i}"), diff));
                }
                let mut diff = ctx.state.get_diff(tile, bel, attr, "CLK_FROM_PLL");
                diff.apply_bit_diff(&en, true, false);
                diffs.push(("CLK_FROM_PLL".to_string(), diff));
                ctx.tiledb.insert(tile, bel, attr, xlat_enum(diffs));
            }
            ctx.tiledb.insert(tile, bel, "CLKIN_CLKFB_ENABLE", en);

            for attr in ["MUX.CLK_TO_PLL", "MUX.SKEWCLKIN2"] {
                ctx.collect_enum(
                    tile,
                    bel,
                    attr,
                    &[
                        "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX",
                        "CLKFX180", "CONCUR",
                    ],
                );
            }

            let mut enable = ctx.state.get_diff(tile, bel, "ENABLE", "1");
            enable.apply_enum_diff(
                ctx.tiledb.item(tile, "CMT", "MUX.OUT10"),
                "NONE",
                "DCM1_CLKIN",
            );
            enable.apply_enum_diff(ctx.tiledb.item(tile, bel, "CLKDV_MODE"), "INT", "HALF");
            enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKDV_COUNT_MAX"), 1, 0);
            enable.apply_enum_diff(
                ctx.tiledb.item(tile, bel, "DCM_CLKIN_IODLY_MUXINSEL"),
                "DELAY_LINE",
                "PASS",
            );
            enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKFX_MULTIPLY"), 1, 0);
            enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKFX_DIVIDE"), 1, 0);
            enable.assert_empty();
        }
    }
    if !skip_pll {
        let bel = "PLL";
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
        for reg in 0..0x20 {
            ctx.tiledb.insert(
                tile,
                bel,
                format!("DRP{reg:02X}"),
                TileItem::from_bitvec((0..16).map(|bit| pll_drp_bit(reg, bit)).collect(), false),
            );
        }

        for pin in [
            "CLKBRST",
            "CLKINSEL",
            "ENOUTSYNC",
            "MANPDLF",
            "MANPULF",
            "REL",
            "RST",
            "SKEWCLKIN1",
            "SKEWCLKIN2",
            "SKEWRST",
            "SKEWSTB",
        ] {
            ctx.collect_inv(tile, bel, pin);
        }

        for attr in [
            "PLL_CLKBURST_ENABLE",
            "PLL_CP_BIAS_TRIP_SHIFT",
            "PLL_DIRECT_PATH_CNTRL",
            "PLL_EN_DLY",
            "PLL_INC_FLOCK",
            "PLL_INC_SLOCK",
            "PLL_LOCK_CNT_RST_FAST",
            "PLL_MAN_LF_EN",
            "PLL_NBTI_EN",
            "PLL_PMCD_MODE",
            "PLL_PWRD_CFG",
            "PLL_SEL_SLIPD",
            "PLL_UNLOCK_CNT_RST_FAST",
            "PLL_VLFHIGH_DIS",
            "PLL_EN_TCLK0",
            "PLL_EN_TCLK1",
            "PLL_EN_TCLK2",
            "PLL_EN_TCLK3",
            "PLL_EN_TCLK4",
            "PLL_EN_VCO0",
            "PLL_EN_VCO1",
            "PLL_EN_VCO2",
            "PLL_EN_VCO3",
            "PLL_EN_VCO4",
            "PLL_EN_VCO5",
            "PLL_EN_VCO6",
            "PLL_EN_VCO7",
            "PLL_EN_VCO_DIV1",
            "PLL_EN_VCO_DIV6",
            "PLL_CLKOUT0_EN",
            "PLL_CLKOUT1_EN",
            "PLL_CLKOUT2_EN",
            "PLL_CLKOUT3_EN",
            "PLL_CLKOUT4_EN",
            "PLL_CLKOUT5_EN",
            "PLL_CLKFBOUT_EN",
            "PLL_CLKOUT0_EDGE",
            "PLL_CLKOUT1_EDGE",
            "PLL_CLKOUT2_EDGE",
            "PLL_CLKOUT3_EDGE",
            "PLL_CLKOUT4_EDGE",
            "PLL_CLKOUT5_EDGE",
            "PLL_CLKFBOUT_EDGE",
            "PLL_CLKFBOUT2_EDGE",
            "PLL_DIVCLK_EDGE",
            "PLL_CLKOUT0_NOCOUNT",
            "PLL_CLKOUT1_NOCOUNT",
            "PLL_CLKOUT2_NOCOUNT",
            "PLL_CLKOUT3_NOCOUNT",
            "PLL_CLKOUT4_NOCOUNT",
            "PLL_CLKOUT5_NOCOUNT",
            "PLL_CLKFBOUT_NOCOUNT",
            "PLL_CLKFBOUT2_NOCOUNT",
            "PLL_DIVCLK_NOCOUNT",
        ] {
            ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
        }
        for attr in [
            "PLL_AVDD_COMP_SET",
            "PLL_AVDD_VBG_PD",
            "PLL_AVDD_VBG_SEL",
            "PLL_CLKBURST_CNT",
            "PLL_CLK0MX",
            "PLL_CLK1MX",
            "PLL_CLK2MX",
            "PLL_CLK3MX",
            "PLL_CLK4MX",
            "PLL_CLK5MX",
            "PLL_CLKFBMX",
            "PLL_CLKCNTRL",
            "PLL_CP",
            "PLL_CP_RES",
            "PLL_DVDD_COMP_SET",
            "PLL_DVDD_VBG_PD",
            "PLL_DVDD_VBG_SEL",
            "PLL_INTFB",
            "PLL_LFHF",
            "PLL_LF_NEN",
            "PLL_LF_PEN",
            "PLL_LOCK_CNT",
            "PLL_PFD_CNTRL",
            "PLL_PFD_DLY",
            "PLL_RES",
            "PLL_TCK4_SEL",
            "PLL_UNLOCK_CNT",
            "CLKOUT0_DESKEW_ADJUST",
            "CLKOUT1_DESKEW_ADJUST",
            "CLKOUT2_DESKEW_ADJUST",
            "CLKOUT3_DESKEW_ADJUST",
            "CLKOUT4_DESKEW_ADJUST",
            "CLKOUT5_DESKEW_ADJUST",
            "CLKFBOUT_DESKEW_ADJUST",
            "PLL_CLKOUT0_DT",
            "PLL_CLKOUT0_HT",
            "PLL_CLKOUT0_LT",
            "PLL_CLKOUT0_PM",
            "PLL_CLKOUT1_DT",
            "PLL_CLKOUT1_HT",
            "PLL_CLKOUT1_LT",
            "PLL_CLKOUT1_PM",
            "PLL_CLKOUT2_DT",
            "PLL_CLKOUT2_HT",
            "PLL_CLKOUT2_LT",
            "PLL_CLKOUT2_PM",
            "PLL_CLKOUT3_DT",
            "PLL_CLKOUT3_HT",
            "PLL_CLKOUT3_LT",
            "PLL_CLKOUT3_PM",
            "PLL_CLKOUT4_DT",
            "PLL_CLKOUT4_HT",
            "PLL_CLKOUT4_LT",
            "PLL_CLKOUT4_PM",
            "PLL_CLKOUT5_DT",
            "PLL_CLKOUT5_HT",
            "PLL_CLKOUT5_LT",
            "PLL_CLKOUT5_PM",
            "PLL_CLKFBOUT_DT",
            "PLL_CLKFBOUT_HT",
            "PLL_CLKFBOUT_LT",
            "PLL_CLKFBOUT_PM",
            "PLL_CLKFBOUT2_DT",
            "PLL_CLKFBOUT2_HT",
            "PLL_CLKFBOUT2_LT",
            "PLL_DIVCLK_DT",
            "PLL_DIVCLK_HT",
            "PLL_DIVCLK_LT",
            "PLL_EN_CNTRL",
            "PLL_FLOCK",
            "PLL_IN_DLY_MX_SEL",
            "PLL_IN_DLY_SET",
            "PLL_LOCK_FB_P1",
            "PLL_LOCK_FB_P2",
            "PLL_LOCK_REF_P1",
            "PLL_LOCK_REF_P2",
            "PLL_MISC",
        ] {
            ctx.collect_bitvec(tile, bel, attr, "");
        }

        ctx.collect_enum_default(tile, bel, "CLKINSEL_MODE", &["DYNAMIC"], "STATIC");
        let item = xlat_bit(
            ctx.state
                .peek_diff(tile, bel, "MUX.CLKIN1", "GIOB5")
                .clone(),
        );
        ctx.tiledb.insert(tile, bel, "CLKINSEL_STATIC", item);
        ctx.collect_enum(
            tile,
            bel,
            "MUX.CLKFBIN",
            &[
                "GIOB0", "GIOB1", "GIOB2", "HCLK0", "HCLK1", "HCLK2", "GIOB3", "GIOB4", "HCLK3",
                "HCLK4", "GIOB5", "GIOB6", "GIOB7", "HCLK5", "HCLK6", "HCLK7", "GIOB8", "GIOB9",
                "HCLK8", "HCLK9", "CLKFBOUT", "CLKFBDCM", "CKINT1",
            ],
        );
        let mut diffs = vec![];
        for (v0, v1) in [
            ("GIOB0", "GIOB5"),
            ("GIOB1", "GIOB6"),
            ("GIOB2", "GIOB7"),
            ("HCLK0", "HCLK5"),
            ("HCLK1", "HCLK6"),
            ("HCLK2", "HCLK7"),
            ("GIOB3", "GIOB8"),
            ("GIOB4", "GIOB9"),
            ("HCLK3", "HCLK8"),
            ("HCLK4", "HCLK9"),
            ("NONE", "CLKFBDCM"),
            ("CLK_FROM_DCM0", "NONE"),
            ("CLK_FROM_DCM1", "NONE"),
            ("CKINT0", "NONE"),
        ] {
            let name = format!("{v0}_{v1}");
            if v0 != "NONE" {
                diffs.push((
                    name.clone(),
                    ctx.state.get_diff(tile, bel, "MUX.CLKIN1", v0),
                ));
            }
            if v1 != "NONE" {
                let mut diff = ctx.state.get_diff(tile, bel, "MUX.CLKIN1", v1);
                diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "CLKINSEL_STATIC"), true, false);
                diffs.push((name, diff));
            }
        }
        ctx.tiledb.insert(tile, bel, "MUX.CLKIN", xlat_enum(diffs));

        ctx.collect_enum_default(
            tile,
            bel,
            "MUX.CLK_TO_DCM0",
            &[
                "CLKOUTDCM0",
                "CLKOUTDCM1",
                "CLKOUTDCM2",
                "CLKOUTDCM3",
                "CLKOUTDCM4",
                "CLKOUTDCM5",
            ],
            "NONE",
        );
        ctx.collect_enum_default(
            tile,
            bel,
            "MUX.CLK_TO_DCM1",
            &[
                "CLKOUTDCM0",
                "CLKOUTDCM1",
                "CLKOUTDCM2",
                "CLKOUTDCM3",
                "CLKOUTDCM4",
                "CLKOUTDCM5",
                "CLKFBDCM",
            ],
            "NONE",
        );

        for mult in 1..=64 {
            for bandwidth in ["LOW", "HIGH"] {
                let mut diff =
                    ctx.state
                        .get_diff(tile, bel, "TABLES", format!("{mult}.{bandwidth}"));
                for (attr, width) in [("PLL_CP", 4), ("PLL_RES", 4), ("PLL_LFHF", 2)] {
                    let val = extract_bitvec_val_part(
                        ctx.tiledb.item(tile, bel, attr),
                        &BitVec::repeat(false, width),
                        &mut diff,
                    );
                    let mut ival = 0;
                    for (i, v) in val.into_iter().enumerate() {
                        if v {
                            ival |= 1 << i;
                        }
                    }
                    ctx.tiledb
                        .insert_misc_data(format!("PLL:{attr}:{bandwidth}:{mult}"), ival);
                }
                for attr in [
                    "PLL_CLKFBOUT_NOCOUNT",
                    "PLL_CLKFBOUT_LT",
                    "PLL_CLKFBOUT_HT",
                    "PLL_CLKFBOUT_EDGE",
                ] {
                    diff.discard_bits(ctx.tiledb.item(tile, bel, attr));
                }
                diff.assert_empty();
            }
        }
        let mut enable = ctx.state.get_diff(tile, bel, "ENABLE", "1");
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_RES"), 0xb, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CP"), 2, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_LFHF"), 3, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_DIVCLK_EDGE"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKFBOUT_EDGE"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT0_EDGE"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT1_EDGE"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT2_EDGE"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT3_EDGE"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT4_EDGE"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT5_EDGE"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_DIVCLK_NOCOUNT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKFBOUT_NOCOUNT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT0_NOCOUNT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT1_NOCOUNT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT2_NOCOUNT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT3_NOCOUNT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT4_NOCOUNT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT5_NOCOUNT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_DIVCLK_LT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKFBOUT_LT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT0_LT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT1_LT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT2_LT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT3_LT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT4_LT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT5_LT"), 1, 0);
        let dly_val = extract_bitvec_val_part(
            ctx.tiledb.item(tile, bel, "PLL_IN_DLY_SET"),
            &bitvec![0; 9],
            &mut enable,
        );
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_EN_DLY"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_IN_DLY_MX_SEL"), 8, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "INV.REL"), 1, 0);
        enable.apply_enum_diff(
            ctx.tiledb.item(tile, bel, "MUX.CLK_TO_DCM0"),
            "NONE",
            "CLKOUTDCM0",
        );
        enable.apply_enum_diff(
            ctx.tiledb.item(tile, bel, "MUX.CLK_TO_DCM1"),
            "NONE",
            "CLKOUTDCM0",
        );
        enable.apply_enum_diff(
            ctx.tiledb.item(tile, "CMT", "MUX.OUT10"),
            "NONE",
            "DCM1_CLKIN",
        );
        ctx.tiledb.insert(tile, bel, "PLL_EN", xlat_bit(enable));

        ctx.state
            .get_diff(tile, bel, "COMPENSATION", "SYSTEM_SYNCHRONOUS")
            .assert_empty();
        let mut diff = ctx
            .state
            .get_diff(tile, bel, "COMPENSATION", "SOURCE_SYNCHRONOUS");
        diff.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, "PLL_IN_DLY_SET"),
            &bitvec![0; 9],
            &dly_val,
        );
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "COMPENSATION", "EXTERNAL");
        diff.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, "PLL_IN_DLY_SET"),
            &bitvec![0; 9],
            &dly_val,
        );
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "COMPENSATION", "INTERNAL");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_INTFB"), 2, 0);
        diff.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, "PLL_IN_DLY_SET"),
            &bitvec![0; 9],
            &dly_val,
        );
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "COMPENSATION", "DCM2PLL");
        diff.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, "PLL_IN_DLY_SET"),
            &bitvec![0; 9],
            &dly_val,
        );
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "COMPENSATION", "PLL2DCM");
        diff.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, "PLL_IN_DLY_SET"),
            &bitvec![0; 9],
            &dly_val,
        );
        diff.assert_empty();
        ctx.insert_device_data("PLL:PLL_IN_DLY_SET", dly_val);
    }
    {
        let tile = "HCLK_CMT";
        let bel = "HCLK_CMT";
        ctx.collect_bit(tile, bel, "DRP_MASK", "1");
    }
}
