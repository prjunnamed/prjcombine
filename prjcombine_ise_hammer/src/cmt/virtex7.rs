use bitvec::prelude::*;
use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_types::TileItem;
use prjcombine_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::{FeatureBit, IseBackend},
    diff::{
        extract_bitvec_val_part, xlat_bit, xlat_bit_wide, xlat_bitvec, xlat_enum, xlat_enum_ocd,
        CollectorCtx, Diff, OcdMode,
    },
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits, TileKV, TileRelation},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_multi_attr_bin, fuzz_multi_attr_dec, fuzz_multi_attr_hex, fuzz_one,
    fuzz_one_extras,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ExpandedDevice::Virtex4(ref edev) = backend.edev else {
        unreachable!()
    };
    let node_cmt = backend.egrid.db.get_node("CMT");
    let bel_pc = BelId::from_idx(9);
    let bel_mmcm = BelId::from_idx(10);
    let bel_pll = BelId::from_idx(11);
    let bel_a = BelId::from_idx(14);
    let bel_b = BelId::from_idx(15);
    let bel_d = BelId::from_idx(17);
    let bel_hclk = BelId::from_idx(18);
    {
        let ctx = FuzzCtx::new(session, backend, "CMT_FIFO", "IN_FIFO", TileBits::MainAuto);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "IN_FIFO")]);

        fuzz_enum!(ctx, "ALMOST_EMPTY_VALUE", ["1", "2"], [(mode "IN_FIFO")]);
        fuzz_enum!(ctx, "ALMOST_FULL_VALUE", ["1", "2"], [(mode "IN_FIFO")]);
        fuzz_enum!(ctx, "ARRAY_MODE", ["ARRAY_MODE_4_X_8", "ARRAY_MODE_4_X_4"], [(mode "IN_FIFO")]);
        fuzz_enum!(ctx, "SLOW_RD_CLK", ["FALSE", "TRUE"], [(mode "IN_FIFO")]);
        fuzz_enum!(ctx, "SLOW_WR_CLK", ["FALSE", "TRUE"], [(mode "IN_FIFO")]);
        fuzz_enum!(ctx, "SYNCHRONOUS_MODE", ["FALSE", "TRUE"], [(mode "IN_FIFO")]);
        fuzz_multi_attr_bin!(ctx, "SPARE", 4, [(mode "IN_FIFO")]);

        fuzz_one!(ctx, "MUX.WRCLK", "PHASER", [
            (mutex "MUX.WRCLK", "PHASER")
        ], [
            (pip (pin "PHASER_WRCLK"), (pin "WRCLK"))
        ]);
        fuzz_one!(ctx, "MUX.WRCLK", "INT", [
            (mutex "MUX.WRCLK", "INT")
        ], [
            (pin_pips "WRCLK")
        ]);
        fuzz_one!(ctx, "MUX.WREN", "PHASER", [
            (mutex "MUX.WREN", "PHASER")
        ], [
            (pip (pin "PHASER_WREN"), (pin "WREN"))
        ]);
        fuzz_one!(ctx, "MUX.WREN", "INT", [
            (mutex "MUX.WREN", "INT")
        ], [
            (pin_pips "WREN")
        ]);
    }
    {
        let ctx = FuzzCtx::new(session, backend, "CMT_FIFO", "OUT_FIFO", TileBits::MainAuto);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "OUT_FIFO")]);

        fuzz_enum!(ctx, "ALMOST_EMPTY_VALUE", ["1", "2"], [(mode "OUT_FIFO")]);
        fuzz_enum!(ctx, "ALMOST_FULL_VALUE", ["1", "2"], [(mode "OUT_FIFO")]);
        fuzz_enum!(ctx, "ARRAY_MODE", ["ARRAY_MODE_8_X_4", "ARRAY_MODE_4_X_4"], [(mode "OUT_FIFO")]);
        fuzz_enum!(ctx, "SLOW_RD_CLK", ["FALSE", "TRUE"], [(mode "OUT_FIFO")]);
        fuzz_enum!(ctx, "SLOW_WR_CLK", ["FALSE", "TRUE"], [(mode "OUT_FIFO")]);
        fuzz_enum!(ctx, "SYNCHRONOUS_MODE", ["FALSE", "TRUE"], [(mode "OUT_FIFO")]);
        fuzz_enum!(ctx, "OUTPUT_DISABLE", ["FALSE", "TRUE"], [(mode "OUT_FIFO")]);
        fuzz_multi_attr_bin!(ctx, "SPARE", 4, [(mode "OUT_FIFO")]);

        fuzz_one!(ctx, "MUX.RDCLK", "PHASER", [
            (mutex "MUX.RDCLK", "PHASER")
        ], [
            (pip (pin "PHASER_RDCLK"), (pin "RDCLK"))
        ]);
        fuzz_one!(ctx, "MUX.RDCLK", "INT", [
            (mutex "MUX.RDCLK", "INT")
        ], [
            (pin_pips "RDCLK")
        ]);
        fuzz_one!(ctx, "MUX.RDEN", "PHASER", [
            (mutex "MUX.RDEN", "PHASER")
        ], [
            (pip (pin "PHASER_RDEN"), (pin "RDEN"))
        ]);
        fuzz_one!(ctx, "MUX.RDEN", "INT", [
            (mutex "MUX.RDEN", "INT")
        ], [
            (pin_pips "RDEN")
        ]);
    }
    for i in 0..4 {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "CMT",
            format!("PHASER_IN{i}"),
            TileBits::Cmt,
        );
        fuzz_inv!(ctx, "RST", [(mode "PHASER_IN_ADV")]);
        for attr in [
            "BURST_MODE",
            "EN_ISERDES_RST",
            "EN_TEST_RING",
            "HALF_CYCLE_ADJ",
            "ICLK_TO_RCLK_BYPASS",
            "DQS_BIAS_MODE",
            "PHASER_IN_EN",
            "SYNC_IN_DIV_RST",
            "UPDATE_NONACTIVE",
            "WR_CYCLES",
        ] {
            fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [(mode "PHASER_IN_ADV")]);
        }
        fuzz_enum!(ctx, "CLKOUT_DIV", [
            "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
        ], [(mode "PHASER_IN_ADV")]);
        fuzz_enum!(ctx, "CTL_MODE", ["HARD", "SOFT"], [(mode "PHASER_IN_ADV")]);
        fuzz_enum!(ctx, "FREQ_REF_DIV", ["NONE", "DIV2", "DIV4"], [(mode "PHASER_IN")]);
        fuzz_enum!(ctx, "OUTPUT_CLK_SRC", [
            "PHASE_REF", "DELAYED_MEM_REF", "DELAYED_PHASE_REF", "DELAYED_REF", "FREQ_REF", "MEM_REF",
        ], [(mode "PHASER_IN_ADV")]);
        fuzz_enum!(ctx, "PD_REVERSE", [
            "1", "2", "3", "4", "5", "6", "7", "8",
        ], [(mode "PHASER_IN_ADV")]);
        fuzz_enum!(ctx, "STG1_PD_UPDATE", [
            "2", "3", "4", "5", "6", "7", "8", "9",
        ], [(mode "PHASER_IN_ADV")]);
        for (attr, width) in [
            ("CLKOUT_DIV_ST", 4),
            ("DQS_AUTO_RECAL", 1),
            ("DQS_FIND_PATTERN", 3),
            ("RD_ADDR_INIT", 2),
            ("REG_OPT_1", 1),
            ("REG_OPT_2", 1),
            ("REG_OPT_4", 1),
            ("RST_SEL", 1),
            ("SEL_OUT", 1),
            ("TEST_BP", 1),
        ] {
            fuzz_multi_attr_bin!(ctx, attr, width, [(mode "PHASER_IN_ADV")]);
        }
        for (attr, width) in [("FINE_DELAY", 6), ("SEL_CLK_OFFSET", 3)] {
            fuzz_multi_attr_dec!(ctx, attr, width, [(mode "PHASER_IN_ADV")]);
        }
        fuzz_one!(ctx, "MUX.PHASEREFCLK", "DQS_PAD", [
            (mode "PHASER_IN_ADV"),
            (mutex "MUX.PHASEREFCLK", "DQS_PAD")
        ], [
            (pip (pin "DQS_PAD"), (pin_far "PHASEREFCLK"))
        ]);
        let bel_cmt = BelId::from_idx(if i < 2 { 15 } else { 16 });
        for pin in [
            "MRCLK0", "MRCLK1", "MRCLK0_S", "MRCLK1_S", "MRCLK0_N", "MRCLK1_N",
        ] {
            fuzz_one!(ctx, "MUX.PHASEREFCLK", pin, [
                (mode "PHASER_IN_ADV"),
                (mutex "MUX.PHASEREFCLK", pin)
            ], [
                (pip (bel_pin bel_cmt, pin), (pin_far "PHASEREFCLK"))
            ]);
        }
    }
    for i in 0..4 {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "CMT",
            format!("PHASER_OUT{i}"),
            TileBits::Cmt,
        );
        fuzz_inv!(ctx, "RST", [(mode "PHASER_OUT_ADV")]);
        for attr in [
            "COARSE_BYPASS",
            "DATA_CTL_N",
            "DATA_RD_CYCLES",
            "EN_OSERDES_RST",
            "EN_TEST_RING",
            "OCLKDELAY_INV",
            "PHASER_OUT_EN",
            "SYNC_IN_DIV_RST",
        ] {
            fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [(mode "PHASER_OUT_ADV")]);
        }
        fuzz_enum!(ctx, "CLKOUT_DIV", [
            "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
        ], [(mode "PHASER_OUT_ADV")]);
        fuzz_enum!(ctx, "CTL_MODE", ["HARD", "SOFT"], [(mode "PHASER_OUT_ADV")]);
        fuzz_enum!(ctx, "OUTPUT_CLK_SRC", [
            "PHASE_REF", "DELAYED_PHASE_REF", "DELAYED_REF", "FREQ_REF",
        ], [
            (mode "PHASER_OUT_ADV"),
            (attr "STG1_BYPASS", "PHASE_REF")
        ]);
        fuzz_enum!(ctx, "STG1_BYPASS", [
            "PHASE_REF", "FREQ_REF",
        ], [
            (mode "PHASER_OUT_ADV"),
            (attr "OUTPUT_CLK_SRC", "PHASE_REF")
        ]);
        for (attr, width) in [("CLKOUT_DIV_ST", 4), ("TEST_OPT", 11)] {
            fuzz_multi_attr_bin!(ctx, attr, width, [(mode "PHASER_OUT_ADV")]);
        }
        fuzz_multi_attr_bin!(ctx, "PO", 3, [(mode "PHASER_OUT_ADV"), (attr "TEST_OPT", "")]);
        for (attr, width) in [("COARSE_DELAY", 6), ("FINE_DELAY", 6), ("OCLK_DELAY", 6)] {
            fuzz_multi_attr_dec!(ctx, attr, width, [(mode "PHASER_OUT_ADV")]);
        }
        let bel_cmt = BelId::from_idx(if i < 2 { 15 } else { 16 });
        for pin in [
            "MRCLK0", "MRCLK1", "MRCLK0_S", "MRCLK1_S", "MRCLK0_N", "MRCLK1_N",
        ] {
            fuzz_one!(ctx, "MUX.PHASEREFCLK", pin, [
                (mode "PHASER_OUT_ADV"),
                (mutex "MUX.PHASEREFCLK", pin)
            ], [
                (pip (bel_pin bel_cmt, pin), (pin_far "PHASEREFCLK"))
            ]);
        }
    }
    {
        let ctx = FuzzCtx::new(session, backend, "CMT", "PHASER_REF", TileBits::Cmt);
        for pin in ["RST", "PWRDWN"] {
            fuzz_inv!(ctx, pin, [(mode "PHASER_REF")]);
        }
        for attr in ["PHASER_REF_EN", "SEL_SLIPD", "SUP_SEL_AREG"] {
            fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [(mode "PHASER_REF")]);
        }
        for (attr, width) in [
            ("AVDD_COMP_SET", 3),
            ("AVDD_VBG_PD", 3),
            ("AVDD_VBG_SEL", 4),
            ("CP", 4),
            ("CP_BIAS_TRIP_SET", 1),
            ("CP_RES", 2),
            ("LF_NEN", 2),
            ("LF_PEN", 2),
            ("MAN_LF", 3),
            ("PFD", 7),
            ("PHASER_REF_MISC", 3),
            ("SEL_LF_HIGH", 3),
            ("TMUX_MUX_SEL", 2),
        ] {
            fuzz_multi_attr_bin!(ctx, attr, width, [(mode "PHASER_REF")]);
        }
        for (attr, width) in [
            ("CONTROL_0", 16),
            ("CONTROL_1", 16),
            ("CONTROL_2", 16),
            ("CONTROL_3", 16),
            ("CONTROL_4", 16),
            ("CONTROL_5", 16),
        ] {
            fuzz_multi_attr_hex!(ctx, attr, width, [(mode "PHASER_REF")]);
        }
        for (attr, width) in [("LOCK_CNT", 10), ("LOCK_FB_DLY", 5), ("LOCK_REF_DLY", 5)] {
            fuzz_multi_attr_dec!(ctx, attr, width, [(mode "PHASER_REF")]);
        }
    }
    {
        let ctx = FuzzCtx::new(session, backend, "CMT", "PHY_CONTROL", TileBits::Cmt);
        for attr in [
            "BURST_MODE",
            "DATA_CTL_A_N",
            "DATA_CTL_B_N",
            "DATA_CTL_C_N",
            "DATA_CTL_D_N",
            "DISABLE_SEQ_MATCH",
            "MULTI_REGION",
            "PHY_COUNT_ENABLE",
            "SYNC_MODE",
        ] {
            fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [(mode "PHY_CONTROL")]);
        }
        fuzz_enum!(ctx, "CLK_RATIO", ["1", "2", "4", "8"], [(mode "PHY_CONTROL")]);
        for (attr, width) in [
            ("RD_DURATION_0", 6),
            ("RD_DURATION_1", 6),
            ("RD_DURATION_2", 6),
            ("RD_DURATION_3", 6),
            ("RD_CMD_OFFSET_0", 6),
            ("RD_CMD_OFFSET_1", 6),
            ("RD_CMD_OFFSET_2", 6),
            ("RD_CMD_OFFSET_3", 6),
            ("WR_DURATION_0", 6),
            ("WR_DURATION_1", 6),
            ("WR_DURATION_2", 6),
            ("WR_DURATION_3", 6),
            ("WR_CMD_OFFSET_0", 6),
            ("WR_CMD_OFFSET_1", 6),
            ("WR_CMD_OFFSET_2", 6),
            ("WR_CMD_OFFSET_3", 6),
            ("CMD_OFFSET", 6),
            ("DI_DURATION", 3),
            ("DO_DURATION", 3),
            ("CO_DURATION", 3),
            ("FOUR_WINDOW_CLOCKS", 6),
            ("EVENTS_DELAY", 6),
            ("AO_TOGGLE", 4),
        ] {
            fuzz_multi_attr_dec!(ctx, attr, width, [(mode "PHY_CONTROL")]);
        }
        for (attr, width) in [("AO_WRLVL_EN", 4), ("SPARE", 1)] {
            fuzz_multi_attr_bin!(ctx, attr, width, [(mode "PHY_CONTROL")]);
        }
    }
    for bel in ["MMCM", "PLL"] {
        let ctx = FuzzCtx::new(session, backend, "CMT", bel, TileBits::Cmt);
        let use_calc = if bel == "MMCM" {
            "MMCMADV_*_USE_CALC"
        } else {
            "PLLADV_*_USE_CALC"
        };
        let mode = if bel == "MMCM" {
            "MMCME2_ADV"
        } else {
            "PLLE2_ADV"
        };
        fuzz_one!(ctx, "ENABLE", "1", [
            (global_xy use_calc, "NO")
        ], [
            (mode mode)
        ]);
        for pin in ["CLKINSEL", "PSEN", "PSINCDEC", "PWRDWN", "RST"] {
            if matches!(pin, "PSEN" | "PSINCDEC") && bel == "PLL" {
                continue;
            }
            fuzz_inv!(ctx, pin, [
                (mutex "MODE", "INV"),
                (mode mode)
            ]);
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
            fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [
                (mutex "MODE", "TEST"),
                (global_xy use_calc, "NO"),
                (mode mode)
            ]);
        }
        if bel == "MMCM" {
            for attr in [
                "SEL_SLIPD",
                "CLKBURST_ENABLE",
                "CLKBURST_REPEAT",
                "INTERP_TEST",
                "CLKOUT6_EN",
                "CLKOUT6_NOCOUNT",
            ] {
                fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [
                    (mutex "MODE", "TEST"),
                    (global_xy use_calc, "NO"),
                    (mode mode)
                ]);
            }
            fuzz_enum!(ctx, "CLKOUT4_CASCADE", ["FALSE", "TRUE"], [
                (mutex "MODE", "TEST"),
                (global_xy use_calc, "NO"),
                (mode mode),
                (attr "CLKOUT6_EN", "TRUE"),
                (attr "CLKOUT4_USE_FINE_PS", ""),
                (attr "CLKOUT4_MX", "")
            ]);
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
                fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [
                    (mutex "MODE", "TEST"),
                    (global_xy use_calc, "NO"),
                    (mode mode),
                    (attr "CLKFBOUT_MX", ""),
                    (attr "CLKOUT0_MX", ""),
                    (attr "CLKOUT1_MX", ""),
                    (attr "CLKOUT2_MX", ""),
                    (attr "CLKOUT3_MX", ""),
                    (attr "CLKOUT4_MX", ""),
                    (attr "CLKOUT5_MX", ""),
                    (attr "CLKOUT6_MX", ""),
                    (attr "INTERP_EN", "00000000")
                ]);
            }
            for attr in ["CLKOUT0_FRAC_EN", "CLKFBOUT_FRAC_EN"] {
                fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [
                    (mutex "MODE", "TEST"),
                    (global_xy use_calc, "NO"),
                    (mode mode),
                    (attr "CLKOUT5_EN", "TRUE"),
                    (attr "CLKOUT6_EN", "TRUE"),
                    (attr "INTERP_EN", "00000000")
                ]);
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
            fuzz_multi_attr_bin!(ctx, attr, width, [
                (mutex "MODE", "TEST"),
                (global_xy use_calc, "NO"),
                (mode mode)
            ]);
        }
        if bel == "MMCM" {
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
                fuzz_multi_attr_bin!(ctx, attr, width, [
                    (mutex "MODE", "TEST"),
                    (global_xy use_calc, "NO"),
                    (mode mode),
                    (attr "INTERP_EN", "00000000")
                ]);
            }
            fuzz_multi_attr_bin!(ctx, "INTERP_EN", 8, [
                (mutex "MODE", "TEST"),
                (global_xy use_calc, "NO"),
                (mode mode)
            ]);
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
                fuzz_multi_attr_bin!(ctx, attr, width, [
                    (mutex "MODE", "TEST"),
                    (global_xy use_calc, "NO"),
                    (mode mode)
                ]);
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
            fuzz_multi_attr_dec!(ctx, attr, width, [
                (mutex "MODE", "TEST"),
                (global_xy use_calc, "NO"),
                (mode mode)
            ]);
        }
        if bel == "MMCM" {
            fuzz_multi_attr_dec!(ctx, "CLKBURST_CNT", 4, [
                (mutex "MODE", "TEST"),
                (global_xy use_calc, "NO"),
                (mode mode)
            ]);
            fuzz_enum!(ctx, "SS_EN", ["FALSE", "TRUE"], [
                (mutex "MODE", "TEST_SS"),
                (global_xy use_calc, "NO"),
                (mode mode),
                (attr "INTERP_EN", "00000000"),
                (attr "CLKFBOUT_LT", "000000"),
                (attr "CLKFBOUT_HT", "000000"),
                (attr "CLKFBOUT_DT", "000000"),
                (attr "CLKFBOUT_FRAC_EN", "FALSE"),
                (attr "CLKOUT2_EN", "FALSE"),
                (attr "CLKOUT2_MX", "00"),
                (attr "CLKOUT3_EN", "FALSE")
            ]);
        }

        for mult in 1..=64 {
            if bel == "MMCM" {
                for bandwidth in ["LOW", "HIGH"] {
                    fuzz_one!(ctx, "TABLES", format!("{mult}.{bandwidth}"), [
                        (mutex "MODE", "CALC"),
                        (global_xy use_calc, "NO"),
                        (attr "SS_EN", "FALSE"),
                        (mode mode)
                    ], [
                        (attr "CLKFBOUT_MULT_F", format!("{mult}")),
                        (attr "BANDWIDTH", bandwidth)
                    ]);
                }
                fuzz_one!(ctx, "TABLES", format!("{mult}.SS"), [
                    (mutex "MODE", "CALC"),
                    (global_xy use_calc, "NO"),
                    (mode mode),
                    (attr "SS_EN", "TRUE"),
                    (attr "INTERP_EN", "00000000"),
                    (attr "CLKFBOUT_LT", "000000"),
                    (attr "CLKFBOUT_HT", "000000"),
                    (attr "CLKFBOUT_DT", "000000"),
                    (attr "CLKFBOUT_FRAC_EN", "FALSE"),
                    (attr "CLKOUT2_EN", "FALSE"),
                    (attr "CLKOUT2_MX", "00"),
                    (attr "CLKOUT3_EN", "FALSE")
                ], [
                    (attr "CLKFBOUT_MULT_F", format!("{mult}")),
                    (attr "BANDWIDTH", "LOW")
                ]);
            } else {
                for bandwidth in ["LOW", "HIGH"] {
                    fuzz_one!(ctx, "TABLES", format!("{mult}.{bandwidth}"), [
                        (mutex "MODE", "CALC"),
                        (global_xy use_calc, "NO"),
                        (mode mode)
                    ], [
                        (attr "CLKFBOUT_MULT", format!("{mult}")),
                        (attr "BANDWIDTH", bandwidth)
                    ]);
                }
            }
        }
        fuzz_enum!(ctx, "COMPENSATION", ["ZHOLD", "EXTERNAL", "INTERNAL", "BUF_IN"], [
            (mutex "MODE", "COMP"),
            (global_xy use_calc, "NO"),
            (mode mode),
            (attr "HROW_DLY_SET", "000")
        ]);

        fuzz_one!(ctx, "DRP_MASK", "1", [(mode mode)], [(pin "DWE")]);

        for pin in ["CLKIN1", "CLKIN2", "CLKFBIN"] {
            fuzz_one!(ctx, format!("MUX.{pin}"), format!("{pin}_CKINT"), [
                (mutex format!("MUX.{pin}"), format!("{pin}_CKINT"))
            ], [
                (pip (pin format!("{pin}_CKINT")), (pin pin))
            ]);
            fuzz_one!(ctx, format!("MUX.{pin}"), format!("{pin}_HCLK"), [
                (mutex format!("MUX.{pin}"), format!("{pin}_HCLK"))
            ], [
                (pip (pin format!("{pin}_HCLK")), (pin pin))
            ]);
            for i in 0..4 {
                fuzz_one!(ctx, format!("MUX.{pin}"), format!("FREQ_BB{i}"), [
                    (mutex format!("MUX.{pin}"), format!("FREQ_BB{i}"))
                ], [
                    (pip (pin format!("FREQ_BB{i}_IN")), (pin pin))
                ]);
            }
            let opin = if pin == "CLKFBIN" {
                "CLKIN1"
            } else {
                "CLKFBIN"
            };
            for i in 0..4 {
                fuzz_one!(ctx, format!("MUX.{pin}_HCLK"), format!("PHASER_REF_BOUNCE{i}"), [
                    (tile_mutex "CCIO", "USE"),
                    (mutex format!("MUX.{pin}_HCLK"), format!("CCIO{i}")),
                    (mutex format!("MUX.{opin}_HCLK"), format!("CCIO{i}")),
                    (pip
                        (bel_pin bel_hclk, format!("CCIO{i}")),
                        (bel_pin bel_hclk, format!("{bel}_{opin}")))
                ], [
                    (pip
                        (bel_pin bel_hclk, format!("CCIO{i}")),
                        (bel_pin bel_hclk, format!("{bel}_{pin}")))
                ]);
            }
            for i in 0..4 {
                fuzz_one!(ctx, format!("MUX.{pin}_HCLK"), format!("PHASER_REF_BOUNCE{i}"), [
                    (tile_mutex "PHASER_REF_BOUNCE", "USE"),
                    (mutex format!("MUX.{pin}_HCLK"), format!("PHASER_REF_BOUNCE{i}")),
                    (mutex format!("MUX.{opin}_HCLK"), format!("PHASER_REF_BOUNCE{i}")),
                    (pip
                        (bel_pin bel_hclk, format!("PHASER_REF_BOUNCE{i}")),
                        (bel_pin bel_hclk, format!("{bel}_{opin}")))
                ], [
                    (pip
                        (bel_pin bel_hclk, format!("PHASER_REF_BOUNCE{i}")),
                        (bel_pin bel_hclk, format!("{bel}_{pin}")))
                ]);
            }
            for i in 0..12 {
                fuzz_one!(ctx, format!("MUX.{pin}_HCLK"), format!("HCLK{i}"), [
                    (global_mutex "HCLK", "USE"),
                    (mutex format!("MUX.{pin}_HCLK"), format!("HCLK{i}")),
                    (mutex format!("MUX.{opin}_HCLK"), format!("HCLK{i}")),
                    (pip
                        (bel_pin bel_hclk, format!("HCLK{i}")),
                        (bel_pin bel_hclk, format!("{bel}_{opin}")))
                ], [
                    (pip
                        (bel_pin bel_hclk, format!("HCLK{i}")),
                        (bel_pin bel_hclk, format!("{bel}_{pin}")))
                ]);
            }
            for i in 0..4 {
                fuzz_one!(ctx, format!("MUX.{pin}_HCLK"), format!("RCLK{i}"), [
                    (global_mutex "RCLK", "USE"),
                    (mutex format!("MUX.{pin}_HCLK"), format!("RCLK{i}")),
                    (mutex format!("MUX.{opin}_HCLK"), format!("RCLK{i}")),
                    (pip
                        (bel_pin bel_hclk, format!("RCLK{i}")),
                        (bel_pin bel_hclk, format!("{bel}_{opin}")))
                ], [
                    (pip
                        (bel_pin bel_hclk, format!("RCLK{i}")),
                        (bel_pin bel_hclk, format!("{bel}_{pin}")))
                ]);
            }
            for i in 4..14 {
                fuzz_one!(ctx, format!("MUX.{pin}_HCLK"), format!("HIN{i}"), [
                    (tile_mutex "HIN", "USE"),
                    (mutex format!("MUX.{pin}_HCLK"), format!("HIN{i}")),
                    (mutex format!("MUX.{opin}_HCLK"), format!("HIN{i}")),
                    (pip
                        (bel_pin bel_hclk, format!("HIN{i}")),
                        (bel_pin bel_hclk, format!("{bel}_{opin}")))
                ], [
                    (pip
                        (bel_pin bel_hclk, format!("HIN{i}")),
                        (bel_pin bel_hclk, format!("{bel}_{pin}")))
                ]);
            }
        }
        fuzz_one!(ctx, "MUX.CLKFBIN", "CLKFBOUT", [
            (mutex "MUX.CLKFBIN", "CLKFBOUT")
        ], [
            (pip (pin "CLKFB"), (pin "CLKFBIN"))
        ]);

        for i in 0..4 {
            fuzz_one!(ctx, format!("BUF.CLKOUT{i}_FREQ_BB"), "1", [], [
                (pip (pin format!("CLKOUT{i}")), (pin format!("FREQ_BB_OUT{i}")))
            ]);
        }

        if bel == "MMCM" {
            for i in 0..4 {
                for pin in ["CLKOUT0", "CLKOUT1", "CLKOUT2", "CLKOUT3", "CLKFBOUT"] {
                    fuzz_one!(ctx, format!("MUX.PERF{i}"), pin, [
                        (tile_mutex "PERF", "TEST"),
                        (mutex format!("MUX.PERF{i}"), pin),
                        (pip
                            (bel_pin bel_hclk, format!("MMCM_PERF{i}")),
                            (bel_pin bel_hclk, format!("PERF{i}")))
                    ], [
                        (pip (pin pin), (pin format!("PERF{i}")))
                    ]);
                }
            }
        }
    }
    for i in 0..2 {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "CMT",
            format!("BUFMRCE{i}"),
            TileBits::Cmt,
        );
        fuzz_one!(ctx, "ENABLE", "1", [], [(mode "BUFMRCE")]);
        fuzz_inv!(ctx, "CE", [(mode "BUFMRCE")]);
        fuzz_enum!(ctx, "INIT_OUT", ["0", "1"], [(mode "BUFMRCE")]);
        fuzz_enum!(ctx, "CE_TYPE", ["SYNC", "ASYNC"], [(mode "BUFMRCE")]);
        let bel_other = BelId::from_idx(12 + (i ^ 1));
        for j in 4..14 {
            fuzz_one!(ctx, "MUX.I", format!("HIN{j}"), [
                (tile_mutex "HIN", "USE"),
                (mutex "MUX.I", format!("HIN{j}")),
                (bel_mutex bel_other, "MUX.I", format!("HIN{j}")),
                (pip (bel_pin bel_hclk, format!("HIN{j}")), (bel_pin bel_other, "I"))
            ], [
                (pip (bel_pin bel_hclk, format!("HIN{j}")), (pin "I"))
            ]);
        }
        for j in 0..2 {
            fuzz_one!(ctx, "MUX.I", format!("CKINT{j}"), [
                (tile_mutex "CKINT", "USE"),
                (mutex "MUX.I", format!("CKINT{j}")),
                (bel_mutex bel_other, "MUX.I", format!("CKINT{j}")),
                (pip (bel_pin bel_hclk, format!("CKINT{j}")), (bel_pin bel_other, "I"))
            ], [
                (pip (bel_pin bel_hclk, format!("CKINT{j}")), (pin "I"))
            ]);
        }
        let ccio = i * 3;
        fuzz_one!(ctx, "MUX.I", format!("CCIO{ccio}"), [
            (tile_mutex "CCIO", "USE"),
            (mutex "MUX.I", format!("CCIO{ccio}")),
            (special TileKV::TouchHout(0)),
            (bel_mutex bel_hclk, "MUX.HOUT0", format!("CCIO{ccio}")),
            (pip (bel_pin bel_hclk, format!("CCIO{ccio}")), (bel_pin bel_hclk, "HOUT0"))
        ], [
            (pip (bel_pin bel_hclk, format!("CCIO{ccio}")), (pin "I"))
        ]);
    }
    if edev.grids.first().unwrap().regs > 1 {
        let mut ctx = FuzzCtx::new(session, backend, "CMT", "CMT_A", TileBits::Cmt);
        ctx.bel_name = "CMT_BOT".to_string();
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::Cmt(-50),
            "CMT",
            "CMT_TOP",
            "ENABLE.SYNC_BB_N",
            "1",
        )];
        fuzz_one_extras!(ctx, "BUF.SYNC_BB.D", "1", [
                (tile_mutex "SYNC_BB", "DRIVE"),
                (pip (bel_pin_far bel_pc, "PHYCTLEMPTY"), (bel_pin bel_pc, "SYNC_BB")),
                (related TileRelation::Delta(0, -50, node_cmt),
                    (tile_mutex "SYNC_BB", "TEST_SOURCE_DUMMY"))
            ], [
                (pip (pin "SYNC_BB"), (pin "SYNC_BB_S"))
            ], extras);
        fuzz_one!(ctx, "BUF.SYNC_BB.U", "1", [
            (tile_mutex "SYNC_BB", "TEST_SOURCE_U"),
            (related TileRelation::Delta(0, -50, node_cmt),
                (tile_mutex "SYNC_BB", "DRIVE")),
            (related TileRelation::Delta(0, -50, node_cmt),
                (pip (bel_pin_far bel_pc, "PHYCTLEMPTY"), (bel_pin bel_pc, "SYNC_BB"))),
            (related TileRelation::Delta(0, -50, node_cmt),
                (pip (bel_pin bel_d, "SYNC_BB"), (bel_pin bel_d, "SYNC_BB_N"))),
            (pip (bel_pin bel_pc, "SYNC_BB"), (bel_pin_far bel_pc, "PHYCTLMSTREMPTY"))
        ], [
            (pip (pin "SYNC_BB_S"), (pin "SYNC_BB"))
        ]);
        for i in 0..4 {
            let extras = vec![ExtraFeature::new(
                ExtraFeatureKind::Cmt(-50),
                "CMT",
                "CMT_TOP",
                format!("ENABLE.FREQ_BB{i}_N"),
                "1",
            )];
            fuzz_one_extras!(ctx, format!("BUF.FREQ_BB{i}.D"), "1", [
                (tile_mutex "FREQ_BB", "DRIVE"),
                (pip (bel_pin bel_b, format!("FREQ_BB{i}_MUX")), (bel_pin bel_b, format!("FREQ_BB{i}"))),
                (related TileRelation::Delta(0, -50, node_cmt),
                    (tile_mutex "FREQ_BB", "TEST_SOURCE_DUMMY"))
            ], [
                (pip (pin format!("FREQ_BB{i}")), (pin format!("FREQ_BB{i}_S")))
            ], extras);
            fuzz_one!(ctx, format!("BUF.FREQ_BB{i}.U"), "1", [
                (tile_mutex "FREQ_BB", "TEST_SOURCE_U"),
                (related TileRelation::Delta(0, -50, node_cmt),
                    (tile_mutex "FREQ_BB", "DRIVE")),
                (related TileRelation::Delta(0, -50, node_cmt),
                    (pip (bel_pin bel_b, format!("FREQ_BB{i}_MUX")), (bel_pin bel_b, format!("FREQ_BB{i}")))),
                (related TileRelation::Delta(0, -50, node_cmt),
                    (pip (bel_pin bel_d, format!("FREQ_BB{i}")), (bel_pin bel_d, format!("FREQ_BB{i}_N")))),
                (pip (bel_pin bel_a, format!("FREQ_BB{i}")), (bel_pin bel_mmcm, format!("FREQ_BB{i}_IN")))
            ], [
                (pip (pin format!("FREQ_BB{i}_S")), (pin format!("FREQ_BB{i}")))
            ]);
        }
    }
    if edev.grids.first().unwrap().regs > 1 {
        let mut ctx = FuzzCtx::new(session, backend, "CMT", "CMT_D", TileBits::Cmt);
        ctx.bel_name = "CMT_TOP".to_string();
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::Cmt(50),
            "CMT",
            "CMT_BOT",
            "ENABLE.SYNC_BB_S",
            "1",
        )];
        fuzz_one_extras!(ctx, "BUF.SYNC_BB.U", "1", [
                (tile_mutex "SYNC_BB", "DRIVE"),
                (pip (bel_pin_far bel_pc, "PHYCTLEMPTY"), (bel_pin bel_pc, "SYNC_BB")),
                (related TileRelation::Delta(0, 50, node_cmt), 
                    (tile_mutex "SYNC_BB", "TEST_SOURCE_DUMMY"))
            ], [
                (pip (pin "SYNC_BB"), (pin "SYNC_BB_N"))
            ], extras);
        fuzz_one!(ctx, "BUF.SYNC_BB.D", "1", [
            (tile_mutex "SYNC_BB", "TEST_SOURCE_D"),
            (related TileRelation::Delta(0, 50, node_cmt),
                (tile_mutex "SYNC_BB", "DRIVE")),
            (related TileRelation::Delta(0, 50, node_cmt),
                (pip (bel_pin_far bel_pc, "PHYCTLEMPTY"), (bel_pin bel_pc, "SYNC_BB"))),
            (related TileRelation::Delta(0, 50, node_cmt),
                (pip (bel_pin bel_a, "SYNC_BB"), (bel_pin bel_a, "SYNC_BB_S"))),
            (pip (bel_pin bel_pc, "SYNC_BB"), (bel_pin_far bel_pc, "PHYCTLMSTREMPTY"))
        ], [
            (pip (pin "SYNC_BB_N"), (pin "SYNC_BB"))
        ]);
        for i in 0..4 {
            let extras = vec![ExtraFeature::new(
                ExtraFeatureKind::Cmt(50),
                "CMT",
                "CMT_BOT",
                format!("ENABLE.FREQ_BB{i}_S"),
                "1",
            )];
            fuzz_one_extras!(ctx, format!("BUF.FREQ_BB{i}.U"), "1", [
                (tile_mutex "FREQ_BB", "DRIVE"),
                (pip (bel_pin bel_b, format!("FREQ_BB{i}_MUX")), (bel_pin bel_b, format!("FREQ_BB{i}"))),
                (related TileRelation::Delta(0, 50, node_cmt),
                    (tile_mutex "FREQ_BB", "TEST_SOURCE_DUMMY"))
            ], [
                (pip (pin format!("FREQ_BB{i}")), (pin format!("FREQ_BB{i}_N")))
            ], extras);
            fuzz_one!(ctx, format!("BUF.FREQ_BB{i}.D"), "1", [
                (tile_mutex "FREQ_BB", "TEST_SOURCE_D"),
                (related TileRelation::Delta(0, 50, node_cmt),
                    (tile_mutex "FREQ_BB", "DRIVE")),
                (related TileRelation::Delta(0, 50, node_cmt),
                    (pip (bel_pin bel_b, format!("FREQ_BB{i}_MUX")), (bel_pin bel_b, format!("FREQ_BB{i}")))),
                (related TileRelation::Delta(0, 50, node_cmt),
                    (pip (bel_pin bel_a, format!("FREQ_BB{i}")), (bel_pin bel_a, format!("FREQ_BB{i}_S")))),
                (pip (bel_pin bel_a, format!("FREQ_BB{i}")), (bel_pin bel_mmcm, format!("FREQ_BB{i}_IN")))
            ], [
                (pip (pin format!("FREQ_BB{i}_N")), (pin format!("FREQ_BB{i}")))
            ]);
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, "CMT", "CMT_B", TileBits::Cmt);
        ctx.bel_name = "CMT_BOT".to_string();
        for i in 0..4 {
            fuzz_one!(ctx, format!("ENABLE.FREQ_BB{i}"), "1", [
                (tile_mutex "FREQ_BB", "TEST")
            ], [
                (pip (bel_pin bel_a, format!("FREQ_BB{i}")), (bel_pin bel_mmcm, format!("FREQ_BB{i}_IN")))
            ]);
            for j in 0..4 {
                fuzz_one!(ctx, format!("MUX.FREQ_BB{i}"), format!("MMCM_CLKOUT{j}"), [
                    (tile_mutex "FREQ_BB", "DRIVE_MMCM"),
                    (mutex format!("MUX.FREQ_BB{i}"), format!("MMCM_CLKOUT{j}")),
                    (pip (bel_pin bel_mmcm, format!("CLKOUT{j}")), (bel_pin bel_mmcm, format!("FREQ_BB_OUT{j}"))),
                    (pip (bel_pin bel_a, format!("FREQ_BB{i}")), (bel_pin bel_mmcm, format!("FREQ_BB{i}_IN")))
                ], [
                    (pip (pin format!("MMCM_FREQ_BB{j}")), (pin format!("FREQ_BB{i}_MUX"))),
                    (pip (pin format!("FREQ_BB{i}_MUX")), (pin format!("FREQ_BB{i}")))
                ]);
            }
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, "CMT", "CMT_C", TileBits::Cmt);
        ctx.bel_name = "CMT_TOP".to_string();
        fuzz_one!(ctx, "DRIVE.SYNC_BB", "1", [
            (tile_mutex "SYNC_BB", "USE"),
            (pip (bel_pin bel_pc, "SYNC_BB"), (bel_pin_far bel_pc, "PHYCTLMSTREMPTY"))
        ], [
            (pip (bel_pin_far bel_pc, "PHYCTLEMPTY"), (bel_pin bel_pc, "SYNC_BB"))
        ]);
        if edev.grids.first().unwrap().regs > 1 {
            fuzz_one!(ctx, "ENABLE.SYNC_BB", "BOT", [
                (tile_mutex "SYNC_BB", "TEST"),
                (no_related TileRelation::Delta(0, -50, node_cmt)),
                (related TileRelation::Delta(0, 50, node_cmt), (nop))
            ], [
                (pip (bel_pin bel_pc, "SYNC_BB"), (bel_pin_far bel_pc, "PHYCTLMSTREMPTY"))
            ]);
            fuzz_one!(ctx, "ENABLE.SYNC_BB", "TOP", [
                (tile_mutex "SYNC_BB", "TEST"),
                (no_related TileRelation::Delta(0, 50, node_cmt)),
                (related TileRelation::Delta(0, -50, node_cmt), (nop))
            ], [
                (pip (bel_pin bel_pc, "SYNC_BB"), (bel_pin_far bel_pc, "PHYCTLMSTREMPTY"))
            ]);
        }
        for i in 0..4 {
            for j in 0..4 {
                fuzz_one!(ctx, format!("MUX.FREQ_BB{i}"), format!("PLL_CLKOUT{j}"), [
                    (tile_mutex "FREQ_BB", "DRIVE_PLL"),
                    (mutex format!("MUX.FREQ_BB{i}"), format!("PLL_CLKOUT{j}")),
                    (pip (bel_pin bel_pll, format!("CLKOUT{j}")), (bel_pin bel_pll, format!("FREQ_BB_OUT{j}"))),
                    (pip (bel_pin bel_a, format!("FREQ_BB{i}")), (bel_pin bel_mmcm, format!("FREQ_BB{i}_IN")))
                ], [
                    (pip (pin format!("PLL_FREQ_BB{j}")), (pin format!("FREQ_BB{i}_MUX"))),
                    (pip (pin format!("FREQ_BB{i}_MUX")), (pin format!("FREQ_BB{i}")))
                ]);
            }
        }
        for (i, pin) in ["FREQREFCLK", "MEMREFCLK", "SYNCIN"]
            .into_iter()
            .enumerate()
        {
            for j in 0..4 {
                fuzz_one!(ctx, format!("MUX.{pin}"), format!("FREQ_BB{j}"), [
                    (mutex format!("MUX.{pin}"), format!("FREQ_BB{j}"))
                ], [
                    (pip (pin format!("FREQ_BB{j}_REF")), (pin pin))
                ]);
            }
            fuzz_one!(ctx, format!("MUX.{pin}"), format!("PLL_CLKOUT{i}"), [
                (mutex format!("MUX.{pin}"), "PLL"),
                (pip (bel_pin bel_pll, format!("CLKOUT{i}")), (bel_pin bel_pll, format!("FREQ_BB_OUT{i}")))
            ], [
                (pip (pin format!("PLL_FREQ_BB{i}")), (pin pin))
            ]);
        }
    }
    {
        let ctx = FuzzCtx::new(session, backend, "CMT", "HCLK_CMT", TileBits::Cmt);
        for i in 0..4 {
            let mut extras = vec![];
            for tile in ["HCLK_IOI_HP", "HCLK_IOI_HR"] {
                let node = backend.egrid.db.get_node(tile);
                if !backend.egrid.node_index[node].is_empty() {
                    extras.push(ExtraFeature::new(
                        ExtraFeatureKind::HclkIoiHere(node),
                        tile,
                        "HCLK_IOI",
                        format!("ENABLE.PERF{i}"),
                        "1",
                    ));
                }
            }
            for j in [i, i ^ 1] {
                fuzz_one_extras!(ctx, format!("MUX.PERF{i}"), format!("MMCM_PERF{j}"), [
                    (tile_mutex "PERF", "USE"),
                    (mutex format!("MUX.PERF{i}"), format!("MMCM_PERF{j}")),
                    (mutex format!("MMCM_PERF{j}"), format!("PERF{i}"))
                ], [
                    (pip (pin format!("MMCM_PERF{j}")), (pin format!("PERF{i}")))
                ], extras.clone());
            }
            for j in 0..4 {
                fuzz_one_extras!(ctx, format!("MUX.PERF{i}"), format!("PHASER_IN_RCLK{j}"), [
                    (tile_mutex "PERF", "USE"),
                    (mutex format!("MUX.PERF{i}"), format!("PHASER_IN_RCLK{j}")),
                    (mutex format!("PHASER_IN_RCLK{j}"), format!("PERF{i}"))
                ], [
                    (pip (pin format!("PHASER_IN_RCLK{j}")), (pin format!("PERF{i}")))
                ], extras.clone());
            }
        }
        for i in 0..4 {
            for pin in ["CLKOUT", "TMUXOUT"] {
                fuzz_one!(ctx, format!("MUX.PHASER_REF_BOUNCE{i}"), pin, [
                    (mutex format!("MUX.PHASER_REF_BOUNCE{i}"), pin)
                ], [
                    (pip
                        (pin format!("PHASER_REF_{pin}")),
                        (pin format!("PHASER_REF_BOUNCE{i}")))
                ]);
            }
        }
        for i in 0..2 {
            let oi = i ^ 1;
            for ud in ['U', 'D'] {
                for j in 0..12 {
                    fuzz_one!(ctx, format!("MUX.LCLK{i}_CMT_{ud}"), format!("HCLK{j}"), [
                        (global_mutex "HCLK", "USE"),
                        (mutex format!("MUX.LCLK{i}_{ud}"), format!("HCLK{j}")),
                        (mutex format!("MUX.LCLK{oi}_{ud}"), format!("HCLK{j}")),
                        (pip (pin format!("HCLK{j}")), (pin format!("LCLK{oi}_CMT_{ud}")))
                    ], [
                        (pip (pin format!("HCLK{j}")), (pin format!("LCLK{i}_CMT_{ud}")))
                    ]);
                }
                for j in 0..4 {
                    fuzz_one!(ctx, format!("MUX.LCLK{i}_CMT_{ud}"), format!("RCLK{j}"), [
                        (global_mutex "RCLK", "USE"),
                        (mutex format!("MUX.LCLK{i}_{ud}"), format!("RCLK{j}")),
                        (mutex format!("MUX.LCLK{oi}_{ud}"), format!("RCLK{j}")),
                        (pip (pin format!("RCLK{j}")), (pin format!("LCLK{oi}_CMT_{ud}")))
                    ], [
                        (pip (pin format!("RCLK{j}")), (pin format!("LCLK{i}_CMT_{ud}")))
                    ]);
                }
            }
        }
        for i in 0..14 {
            let oi = i ^ 1;
            for j in 0..12 {
                fuzz_one!(ctx, format!("MUX.HOUT{i}"), format!("HCLK{j}"), [
                    (global_mutex "HCLK", "USE"),
                    (special TileKV::TouchHout(i)),
                    (special TileKV::TouchHout(oi)),
                    (mutex format!("MUX.HOUT{i}"), format!("HCLK{j}")),
                    (mutex format!("MUX.HOUT{oi}"), format!("HCLK{j}")),
                    (pip (pin format!("HCLK{j}")), (pin format!("HOUT{oi}")))
                ], [
                    (pip (pin format!("HCLK{j}")), (pin format!("HOUT{i}")))
                ]);
                if i == 0 {
                    fuzz_one!(ctx, format!("MUX.HOUT{i}"), format!("HCLK{j}.EXCL"), [
                        (global_mutex "HCLK", "TEST"),
                        (special TileKV::TouchHout(i)),
                        (mutex format!("MUX.HOUT{i}"), format!("HCLK{j}"))
                    ], [
                        (pip (pin format!("HCLK{j}")), (pin format!("HOUT{i}")))
                    ]);
                }
            }
            for j in 0..4 {
                fuzz_one!(ctx, format!("MUX.HOUT{i}"), format!("PHASER_REF_BOUNCE{j}"), [
                    (tile_mutex "CCIO", "USE"),
                    (special TileKV::TouchHout(i)),
                    (special TileKV::TouchHout(oi)),
                    (mutex format!("MUX.HOUT{i}"), format!("CCIO{j}")),
                    (mutex format!("MUX.HOUT{oi}"), format!("CCIO{j}")),
                    (pip (pin format!("CCIO{j}")), (pin format!("HOUT{oi}")))
                ], [
                    (pip (pin format!("CCIO{j}")), (pin format!("HOUT{i}")))
                ]);
                if i == 0 {
                    fuzz_one!(ctx, format!("MUX.HOUT{i}"), format!("CCIO{j}.EXCL"), [
                        (tile_mutex "CCIO", "TEST"),
                        (special TileKV::TouchHout(i)),
                        (mutex format!("MUX.HOUT{i}"), format!("CCIO{j}"))
                    ], [
                        (pip (pin format!("CCIO{j}")), (pin format!("HOUT{i}")))
                    ]);
                }
            }
            for j in 0..4 {
                fuzz_one!(ctx, format!("MUX.HOUT{i}"), format!("PHASER_REF_BOUNCE{j}"), [
                    (tile_mutex "PHASER_REF_BOUNCE", "USE"),
                    (special TileKV::TouchHout(i)),
                    (special TileKV::TouchHout(oi)),
                    (mutex format!("MUX.HOUT{i}"), format!("PHASER_REF_BOUNCE{j}")),
                    (mutex format!("MUX.HOUT{oi}"), format!("PHASER_REF_BOUNCE{j}")),
                    (pip (pin format!("PHASER_REF_BOUNCE{j}")), (pin format!("HOUT{oi}")))
                ], [
                    (pip (pin format!("PHASER_REF_BOUNCE{j}")), (pin format!("HOUT{i}")))
                ]);
            }
            for j in 4..14 {
                fuzz_one!(ctx, format!("MUX.HOUT{i}"), format!("HIN{j}"), [
                    (tile_mutex "HIN", "USE"),
                    (special TileKV::TouchHout(i)),
                    (special TileKV::TouchHout(oi)),
                    (mutex format!("MUX.HOUT{i}"), format!("HIN{j}")),
                    (mutex format!("MUX.HOUT{oi}"), format!("HIN{j}")),
                    (pip (pin format!("HIN{j}")), (pin format!("HOUT{oi}")))
                ], [
                    (pip (pin format!("HIN{j}")), (pin format!("HOUT{i}")))
                ]);
                if i == 0 {
                    fuzz_one!(ctx, format!("MUX.HOUT{i}"), format!("HIN{j}.EXCL"), [
                        (tile_mutex "HIN", "TEST"),
                        (special TileKV::TouchHout(i)),
                        (mutex format!("MUX.HOUT{i}"), format!("HIN{j}"))
                    ], [
                        (pip (pin format!("HIN{j}")), (pin format!("HOUT{i}")))
                    ]);
                }
            }
            for (j, pin) in [
                "CLKOUT0",
                "CLKOUT0B",
                "CLKOUT1",
                "CLKOUT1B",
                "CLKOUT2",
                "CLKOUT2B",
                "CLKOUT3",
                "CLKOUT3B",
                "CLKOUT4",
                "CLKOUT5",
                "CLKOUT6",
                "CLKFBOUT",
                "CLKFBOUTB",
                "TMUXOUT",
            ]
            .into_iter()
            .enumerate()
            {
                fuzz_one!(ctx, format!("MUX.HOUT{i}"), format!("MMCM_{pin}"), [
                    (special TileKV::TouchHout(i)),
                    (mutex format!("MUX.HOUT{i}"), format!("MMCM_{pin}"))
                ], [
                    (pip (pin format!("MMCM_OUT{j}")), (pin format!("HOUT{i}")))
                ]);
            }
            for (j, pin) in [
                "CLKOUT0", "CLKOUT1", "CLKOUT2", "CLKOUT3", "CLKOUT4", "CLKOUT5", "CLKFBOUT",
                "TMUXOUT",
            ]
            .into_iter()
            .enumerate()
            {
                fuzz_one!(ctx, format!("MUX.HOUT{i}"), format!("PLL_{pin}"), [
                    (special TileKV::TouchHout(i)),
                    (mutex format!("MUX.HOUT{i}"), format!("PLL_{pin}"))
                ], [
                    (pip (pin format!("PLL_OUT{j}")), (pin format!("HOUT{i}")))
                ]);
            }
        }
        for i in 0..4 {
            fuzz_one!(ctx, format!("ENABLE.CKINT{i}"), "1", [
                (tile_mutex "CKINT", "TEST")
            ], [
                (pin_pips format!("CKINT{i}"))
            ]);
        }
        for i in 0..4 {
            fuzz_one!(ctx, format!("MUX.FREQ_BB{i}"), format!("CKINT{i}"), [
                (tile_mutex "CKINT", "USE"),
                (mutex format!("MUX.FREQ_BB{i}"), format!("CKINT{i}")),
                (pin_pips format!("CKINT{i}"))
            ], [
                (pip (pin format!("CKINT{i}")), (pin format!("FREQ_BB{i}_MUX")))
            ]);
            fuzz_one!(ctx, format!("MUX.FREQ_BB{i}"), format!("CCIO{i}"), [
                (tile_mutex "CCIO", "TEST_FREQ_BB"),
                (mutex format!("MUX.FREQ_BB{i}"), format!("CCIO{i}"))
            ], [
                (pip (pin format!("CCIO{i}")), (pin format!("FREQ_BB{i}_MUX")))
            ]);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(ref edev) = ctx.edev else {
        unreachable!()
    };
    {
        let tile = "CMT_FIFO";
        let bel = "IN_FIFO";
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        ctx.collect_enum_default(tile, bel, "ALMOST_EMPTY_VALUE", &["1", "2"], "NONE");
        ctx.collect_enum_default(tile, bel, "ALMOST_FULL_VALUE", &["1", "2"], "NONE");
        ctx.collect_enum(
            tile,
            bel,
            "ARRAY_MODE",
            &["ARRAY_MODE_4_X_8", "ARRAY_MODE_4_X_4"],
        );
        ctx.collect_enum_bool(tile, bel, "SLOW_RD_CLK", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "SLOW_WR_CLK", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "SYNCHRONOUS_MODE", "FALSE", "TRUE");
        ctx.collect_bitvec(tile, bel, "SPARE", "");
        ctx.collect_enum(tile, bel, "MUX.WRCLK", &["INT", "PHASER"]);
        ctx.collect_enum(tile, bel, "MUX.WREN", &["INT", "PHASER"]);
    }
    {
        let tile = "CMT_FIFO";
        let bel = "OUT_FIFO";
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        ctx.collect_enum_default(tile, bel, "ALMOST_EMPTY_VALUE", &["1", "2"], "NONE");
        ctx.collect_enum_default(tile, bel, "ALMOST_FULL_VALUE", &["1", "2"], "NONE");
        ctx.collect_enum(
            tile,
            bel,
            "ARRAY_MODE",
            &["ARRAY_MODE_8_X_4", "ARRAY_MODE_4_X_4"],
        );
        ctx.collect_enum_bool(tile, bel, "SLOW_RD_CLK", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "SLOW_WR_CLK", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "SYNCHRONOUS_MODE", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "OUTPUT_DISABLE", "FALSE", "TRUE");
        ctx.collect_bitvec(tile, bel, "SPARE", "");
        ctx.collect_enum(tile, bel, "MUX.RDCLK", &["INT", "PHASER"]);
        ctx.collect_enum(tile, bel, "MUX.RDEN", &["INT", "PHASER"]);
    }
    for i in 0..4 {
        let tile = "CMT";
        let bel = &format!("PHASER_IN{i}");
        ctx.collect_inv(tile, bel, "RST");
        for attr in [
            "BURST_MODE",
            "DQS_BIAS_MODE",
            "EN_ISERDES_RST",
            "EN_TEST_RING",
            "HALF_CYCLE_ADJ",
            "ICLK_TO_RCLK_BYPASS",
            "PHASER_IN_EN",
            "SYNC_IN_DIV_RST",
            "UPDATE_NONACTIVE",
            "WR_CYCLES",
        ] {
            ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
        }
        ctx.collect_enum_default_ocd(
            tile,
            bel,
            "CLKOUT_DIV",
            &[
                "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
            ],
            "NONE",
            OcdMode::BitOrderDrpV6,
        );
        ctx.collect_enum(tile, bel, "CTL_MODE", &["HARD", "SOFT"]);
        ctx.collect_enum(tile, bel, "FREQ_REF_DIV", &["NONE", "DIV2", "DIV4"]);
        ctx.collect_enum_ocd(
            tile,
            bel,
            "OUTPUT_CLK_SRC",
            &[
                "PHASE_REF",
                "DELAYED_MEM_REF",
                "DELAYED_PHASE_REF",
                "DELAYED_REF",
                "FREQ_REF",
                "MEM_REF",
            ],
            OcdMode::BitOrderDrpV6,
        );
        ctx.collect_enum(
            tile,
            bel,
            "PD_REVERSE",
            &["1", "2", "3", "4", "5", "6", "7", "8"],
        );
        ctx.collect_enum(
            tile,
            bel,
            "STG1_PD_UPDATE",
            &["2", "3", "4", "5", "6", "7", "8", "9"],
        );
        for attr in [
            "CLKOUT_DIV_ST",
            "DQS_AUTO_RECAL",
            "DQS_FIND_PATTERN",
            "RD_ADDR_INIT",
            "REG_OPT_1",
            "REG_OPT_2",
            "REG_OPT_4",
            "RST_SEL",
            "SEL_OUT",
            "TEST_BP",
            "FINE_DELAY",
            "SEL_CLK_OFFSET",
        ] {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        ctx.collect_enum_ocd(
            tile,
            bel,
            "MUX.PHASEREFCLK",
            &[
                "DQS_PAD", "MRCLK0", "MRCLK0_S", "MRCLK0_N", "MRCLK1", "MRCLK1_S", "MRCLK1_N",
            ],
            OcdMode::BitOrderDrpV6,
        )
    }
    for i in 0..4 {
        let tile = "CMT";
        let bel = &format!("PHASER_OUT{i}");

        ctx.collect_inv(tile, bel, "RST");
        for attr in [
            "COARSE_BYPASS",
            "DATA_CTL_N",
            "DATA_RD_CYCLES",
            "EN_OSERDES_RST",
            "EN_TEST_RING",
            "OCLKDELAY_INV",
            "PHASER_OUT_EN",
            "SYNC_IN_DIV_RST",
        ] {
            ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
        }
        ctx.collect_enum_default_ocd(
            tile,
            bel,
            "CLKOUT_DIV",
            &[
                "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
            ],
            "NONE",
            OcdMode::BitOrderDrpV6,
        );
        ctx.collect_enum(tile, bel, "CTL_MODE", &["HARD", "SOFT"]);
        ctx.collect_enum_ocd(
            tile,
            bel,
            "OUTPUT_CLK_SRC",
            &["PHASE_REF", "DELAYED_PHASE_REF", "DELAYED_REF", "FREQ_REF"],
            OcdMode::BitOrderDrpV6,
        );
        ctx.collect_enum(tile, bel, "STG1_BYPASS", &["PHASE_REF", "FREQ_REF"]);
        for attr in ["CLKOUT_DIV_ST", "COARSE_DELAY", "FINE_DELAY", "OCLK_DELAY"] {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        let diffs = ctx.state.get_diffs(tile, bel, "TEST_OPT", "");
        let diffs_po = ctx.state.get_diffs(tile, bel, "PO", "");
        assert_eq!(&diffs[6..9], &diffs_po[..]);
        ctx.tiledb.insert(tile, bel, "TEST_OPT", xlat_bitvec(diffs));

        ctx.collect_enum_default_ocd(
            tile,
            bel,
            "MUX.PHASEREFCLK",
            &[
                "MRCLK0", "MRCLK0_S", "MRCLK0_N", "MRCLK1", "MRCLK1_S", "MRCLK1_N",
            ],
            "NONE",
            OcdMode::BitOrderDrpV6,
        );
    }
    {
        let tile = "CMT";
        let bel = "PHASER_REF";
        ctx.collect_inv(tile, bel, "RST");
        ctx.collect_inv(tile, bel, "PWRDWN");
        for attr in ["PHASER_REF_EN", "SEL_SLIPD", "SUP_SEL_AREG"] {
            ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
        }
        for attr in [
            "AVDD_COMP_SET",
            "AVDD_VBG_PD",
            "AVDD_VBG_SEL",
            "CP",
            "CP_BIAS_TRIP_SET",
            "CP_RES",
            "LF_NEN",
            "LF_PEN",
            "MAN_LF",
            "PFD",
            "PHASER_REF_MISC",
            "SEL_LF_HIGH",
            "TMUX_MUX_SEL",
            "CONTROL_0",
            "CONTROL_1",
            "CONTROL_2",
            "CONTROL_3",
            "CONTROL_4",
            "CONTROL_5",
            "LOCK_CNT",
            "LOCK_FB_DLY",
            "LOCK_REF_DLY",
        ] {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
    }
    {
        let tile = "CMT";
        let bel = "PHY_CONTROL";
        for attr in [
            "BURST_MODE",
            "DATA_CTL_A_N",
            "DATA_CTL_B_N",
            "DATA_CTL_C_N",
            "DATA_CTL_D_N",
            "DISABLE_SEQ_MATCH",
            "MULTI_REGION",
            "PHY_COUNT_ENABLE",
            "SYNC_MODE",
        ] {
            ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
        }
        ctx.collect_enum(tile, bel, "CLK_RATIO", &["1", "2", "4", "8"]);
        for attr in [
            "RD_DURATION_0",
            "RD_DURATION_1",
            "RD_DURATION_2",
            "RD_DURATION_3",
            "RD_CMD_OFFSET_0",
            "RD_CMD_OFFSET_1",
            "RD_CMD_OFFSET_2",
            "RD_CMD_OFFSET_3",
            "WR_DURATION_0",
            "WR_DURATION_1",
            "WR_DURATION_2",
            "WR_DURATION_3",
            "WR_CMD_OFFSET_0",
            "WR_CMD_OFFSET_1",
            "WR_CMD_OFFSET_2",
            "WR_CMD_OFFSET_3",
            "CMD_OFFSET",
            "DI_DURATION",
            "DO_DURATION",
            "CO_DURATION",
            "FOUR_WINDOW_CLOCKS",
            "EVENTS_DELAY",
            "AO_TOGGLE",
            "AO_WRLVL_EN",
            "SPARE",
        ] {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
    }
    for bel in ["MMCM", "PLL"] {
        let tile = "CMT";

        fn drp_bit(which: &'static str, reg: usize, bit: usize) -> FeatureBit {
            if which == "MMCM" {
                let tile = 15 - (reg >> 3);
                let frame = 29 - (bit & 1);
                let bit = 63 - ((bit >> 1) | (reg & 7) << 3);
                FeatureBit::new(tile, frame, bit)
            } else {
                let tile = 37 + (reg >> 3);
                let frame = 28 + (bit & 1);
                let bit = (bit >> 1) | (reg & 7) << 3;
                FeatureBit::new(tile, frame, bit)
            }
        }
        for reg in 0..(if bel == "MMCM" { 0x80 } else { 0x68 }) {
            ctx.tiledb.insert(
                tile,
                bel,
                format!("DRP{reg:02X}"),
                TileItem::from_bitvec((0..16).map(|bit| drp_bit(bel, reg, bit)).collect(), false),
            );
        }

        for pin in ["CLKINSEL", "PWRDWN", "RST"] {
            ctx.collect_inv(tile, bel, pin);
        }
        if bel == "MMCM" {
            for pin in ["PSEN", "PSINCDEC"] {
                ctx.collect_inv(tile, bel, pin);
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
            ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
        }
        if bel == "MMCM" {
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
                ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
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
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        if bel == "MMCM" {
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
                ctx.collect_bitvec(tile, bel, attr, "");
            }
            // THIS PIECE OF SHIT ACTUALLY CORRUPTS ITS OWN MEMORY TRYING TO COMPUTE THIS FUCKING ATTRIBUTE
            let mut diffs = ctx.state.get_diffs(tile, bel, "SPARE_ANALOG", "");
            assert!(diffs[1].bits.is_empty());
            diffs[1].bits.insert(FeatureBit::new(7, 28, 30), true);
            ctx.tiledb
                .insert(tile, bel, "SPARE_ANALOG", xlat_bitvec(diffs));
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
                ctx.collect_bitvec(tile, bel, attr, "");
            }
        }
        for (addr, name) in [(0x16, "DIVCLK"), (0x17, "CLKFBIN")] {
            ctx.tiledb.insert(
                tile,
                bel,
                format!("{name}_EDGE"),
                TileItem::from_bit(drp_bit(bel, addr, 13), false),
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
            if name == "CLKOUT6" && bel == "PLL" {
                continue;
            }
            ctx.tiledb.insert(
                tile,
                bel,
                format!("{name}_EDGE"),
                TileItem::from_bit(drp_bit(bel, addr, 7), false),
            );
        }
        if bel == "MMCM" {
            for (reg, bit, attr) in [
                (0x07, 10, "CLKOUT0_FRAC_WF_FALL"),
                (0x09, 10, "CLKOUT0_FRAC_WF_RISE"),
                (0x13, 10, "CLKFBOUT_FRAC_WF_FALL"),
                (0x15, 10, "CLKFBOUT_FRAC_WF_RISE"),
            ] {
                ctx.tiledb.insert(
                    tile,
                    bel,
                    attr,
                    TileItem::from_bit(drp_bit(bel, reg, bit), false),
                );
            }
            for (addr, name) in [(0x09, "CLKOUT0"), (0x15, "CLKFBOUT")] {
                ctx.tiledb.insert(
                    tile,
                    bel,
                    format!("{name}_FRAC"),
                    TileItem::from_bitvec(
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

        if bel == "MMCM" {
            ctx.tiledb.insert(
                tile,
                bel,
                "MMCM_EN",
                TileItem::from_bit(drp_bit(bel, 0x74, 0), false),
            );
        } else {
            ctx.tiledb.insert(
                tile,
                bel,
                "PLL_EN",
                TileItem::from_bit(drp_bit(bel, 0x5c, 0), false),
            );
        }

        let mut enable = ctx.state.get_diff(tile, bel, "ENABLE", "1");
        enable.apply_bit_diff(
            ctx.tiledb.item(tile, bel, &format!("{bel}_EN")),
            true,
            false,
        );
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKFBIN_HT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKFBIN_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DIVCLK_HT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DIVCLK_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKFBOUT_HT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKFBOUT_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT0_HT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT0_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT1_HT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT1_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT2_HT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT2_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT3_HT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT3_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT4_HT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT4_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT5_HT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT5_LT"), 0x3f, 0);
        if bel == "MMCM" {
            enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "INTERP_EN"), 0x10, 0);
            enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "SS_STEPS_INIT"), 4, 0);
            enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "SS_STEPS"), 7, 0);
            enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT6_HT"), 1, 0);
            enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT6_LT"), 0x3f, 0);
        }
        enable.assert_empty();

        let modes = if bel == "MMCM" {
            &["LOW", "HIGH", "SS"][..]
        } else {
            &["LOW", "HIGH"][..]
        };
        for mode in modes {
            for mult in 1..=64 {
                let mut diff = ctx
                    .state
                    .get_diff(tile, bel, "TABLES", format!("{mult}.{mode}"));
                for attr in ["CP", "RES", "LFHF"] {
                    let item = ctx.tiledb.item(tile, bel, attr);
                    let base = BitVec::repeat(false, item.bits.len());
                    let val = extract_bitvec_val_part(item, &base, &mut diff);
                    let mut ival = 0;
                    for (i, v) in val.into_iter().enumerate() {
                        if v {
                            ival |= 1 << i;
                        }
                    }
                    ctx.tiledb
                        .insert_misc_data(format!("{bel}:{attr}:{mode}:{mult}"), ival);
                }
                for attr in [
                    "LOCK_REF_DLY",
                    "LOCK_FB_DLY",
                    "LOCK_CNT",
                    "LOCK_SAT_HIGH",
                    "UNLOCK_CNT",
                ] {
                    let item = ctx.tiledb.item(tile, bel, attr);
                    let base = BitVec::repeat(false, item.bits.len());
                    let val = extract_bitvec_val_part(item, &base, &mut diff);
                    let mut ival = 0;
                    for (i, v) in val.into_iter().enumerate() {
                        if v {
                            ival |= 1 << i;
                        }
                    }
                    ctx.tiledb
                        .insert_misc_data(format!("{bel}:{attr}:{mult}"), ival);
                }
                diff.discard_bits(ctx.tiledb.item(tile, bel, "CLKFBOUT_NOCOUNT"));
                diff.discard_bits(ctx.tiledb.item(tile, bel, "CLKFBOUT_EDGE"));
                diff.discard_bits(ctx.tiledb.item(tile, bel, "CLKFBOUT_LT"));
                diff.discard_bits(ctx.tiledb.item(tile, bel, "CLKFBOUT_HT"));
                if bel == "MMCM" {
                    diff.discard_bits(ctx.tiledb.item(tile, bel, "CLKFBOUT_PM_RISE"));
                    diff.discard_bits(ctx.tiledb.item(tile, bel, "CLKFBOUT_PM_FALL"));
                    diff.discard_bits(ctx.tiledb.item(tile, bel, "CLKFBOUT_FRAC_WF_RISE"));
                    diff.discard_bits(ctx.tiledb.item(tile, bel, "CLKFBOUT_FRAC_WF_FALL"));
                }
                diff.assert_empty();
            }
        }

        let mut diff = ctx.state.get_diff(tile, bel, "COMPENSATION", "BUF_IN");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IN_DLY_MX_DVDD"), 0x31, 0);
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IN_DLY_MX_CVDD"), 0x12, 0);
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "COMPENSATION", "EXTERNAL");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IN_DLY_MX_DVDD"), 0x31, 0);
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IN_DLY_MX_CVDD"), 0x12, 0);
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "COMPENSATION", "INTERNAL");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IN_DLY_MX_DVDD"), 0x2f, 0);
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IN_DLY_MX_CVDD"), 0x12, 0);
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "COMPENSATION", "ZHOLD");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IN_DLY_MX_DVDD"), 0x01, 0);
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IN_DLY_MX_CVDD"), 0x18, 0);
        diff.assert_empty();

        for i in 0..4 {
            ctx.collect_bit(tile, bel, &format!("BUF.CLKOUT{i}_FREQ_BB"), "1");
        }
        for pin in ["CLKFBIN", "CLKIN1", "CLKIN2"] {
            ctx.collect_enum_ocd(
                tile,
                bel,
                &format!("MUX.{pin}"),
                &[
                    "FREQ_BB0".to_string(),
                    "FREQ_BB1".to_string(),
                    "FREQ_BB2".to_string(),
                    "FREQ_BB3".to_string(),
                    format!("{pin}_HCLK"),
                    format!("{pin}_CKINT"),
                ],
                OcdMode::BitOrderDrpV6,
            );
            let mut vals = vec![];
            for i in 0..12 {
                vals.push(format!("HCLK{i}"));
            }
            for i in 0..4 {
                vals.push(format!("RCLK{i}"));
            }
            for i in 4..14 {
                vals.push(format!("HIN{i}"));
            }
            for i in 0..4 {
                vals.push(format!("PHASER_REF_BOUNCE{i}"));
            }
            ctx.collect_enum_default_ocd(
                tile,
                bel,
                &format!("MUX.{pin}_HCLK"),
                &vals,
                "NONE",
                OcdMode::Mux,
            );
        }
        if bel == "MMCM" {
            for i in 0..4 {
                ctx.collect_enum_default_ocd(
                    tile,
                    bel,
                    &format!("MUX.PERF{i}"),
                    &["CLKOUT0", "CLKOUT1", "CLKOUT2", "CLKOUT3", "CLKFBOUT"],
                    "NONE",
                    OcdMode::BitOrderDrpV6,
                );
            }
        }

        ctx.state
            .get_diff(tile, bel, "MUX.CLKFBIN", "CLKFBOUT")
            .assert_empty();

        let item = ctx.extract_bit(tile, bel, "DRP_MASK", "1");
        assert_eq!(item.bits.len(), 1);
        assert_eq!(item.bits[0].tile, 50);
        let mut item_l = item.clone();
        let mut item_r = item;
        item_l.bits[0].tile = 0;
        item_r.bits[0].tile = 1;
        if bel == "PLL" {
            ctx.tiledb
                .insert("HCLK", "HCLK", "DRP_MASK_ABOVE_L", item_l);
            ctx.tiledb
                .insert("HCLK", "HCLK", "DRP_MASK_ABOVE_R", item_r);
        } else {
            ctx.tiledb
                .insert("HCLK", "HCLK", "DRP_MASK_BELOW_L", item_l);
            ctx.tiledb
                .insert("HCLK", "HCLK", "DRP_MASK_BELOW_R", item_r);
        }
    }
    for i in 0..2 {
        let tile = "CMT";
        let bel = &format!("BUFMRCE{i}");
        ctx.collect_bit(tile, bel, "ENABLE", "1");
        ctx.collect_inv(tile, bel, "CE");
        ctx.collect_enum_bool(tile, bel, "INIT_OUT", "0", "1");
        ctx.collect_enum(tile, bel, "CE_TYPE", &["SYNC", "ASYNC"]);
        ctx.collect_enum_default_ocd(
            tile,
            bel,
            "MUX.I",
            &[
                ["CCIO0", "CCIO3"][i],
                "HIN4",
                "HIN5",
                "HIN6",
                "HIN7",
                "HIN8",
                "HIN9",
                "HIN10",
                "HIN11",
                "HIN12",
                "HIN13",
                "CKINT0",
                "CKINT1",
            ],
            "NONE",
            OcdMode::Mux,
        );
    }
    {
        let tile = "CMT";
        let bel = "CMT_BOT";
        for i in 0..4 {
            ctx.collect_enum_default(
                tile,
                bel,
                &format!("MUX.FREQ_BB{i}"),
                &[
                    "MMCM_CLKOUT0",
                    "MMCM_CLKOUT1",
                    "MMCM_CLKOUT2",
                    "MMCM_CLKOUT3",
                ],
                "NONE",
            );
            let mut diff = ctx
                .state
                .get_diff(tile, bel, format!("ENABLE.FREQ_BB{i}"), "1");
            let diff_bot = diff.split_bits_by(|bit| bit.tile < 24);
            let diff_top = diff.split_bits_by(|bit| bit.tile > 25 && bit.tile != 50);
            ctx.tiledb.insert(
                tile,
                "CMT_BOT",
                format!("ENABLE.FREQ_BB{i}"),
                xlat_bit_wide(diff_bot),
            );
            ctx.tiledb.insert(
                tile,
                "CMT_TOP",
                format!("ENABLE.FREQ_BB{i}"),
                xlat_bit_wide(diff_top),
            );
            ctx.tiledb.insert(
                tile,
                "HCLK_CMT",
                format!("ENABLE.FREQ_BB{i}"),
                xlat_bit_wide(diff),
            );
        }
        if edev.grids.first().unwrap().regs > 1 {
            ctx.collect_bit(tile, bel, "ENABLE.SYNC_BB_S", "1");
            ctx.collect_bit(tile, bel, "BUF.SYNC_BB.U", "1");
            let mut diff = ctx.state.get_diff(tile, bel, "BUF.SYNC_BB.D", "1");
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE.SYNC_BB_S"), true, false);
            ctx.tiledb
                .insert(tile, bel, "BUF.SYNC_BB.D", xlat_bit(diff));
            for i in 0..4 {
                ctx.collect_bit_wide(tile, bel, &format!("ENABLE.FREQ_BB{i}_S"), "1");
                ctx.collect_bit_wide(tile, bel, &format!("BUF.FREQ_BB{i}.U"), "1");
                let mut diff = ctx
                    .state
                    .get_diff(tile, bel, format!("BUF.FREQ_BB{i}.D"), "1");
                diff.apply_bitvec_diff_int(
                    ctx.tiledb.item(tile, bel, &format!("ENABLE.FREQ_BB{i}_S")),
                    3,
                    0,
                );
                ctx.tiledb
                    .insert(tile, bel, format!("BUF.FREQ_BB{i}.D"), xlat_bit_wide(diff));
            }
        }
    }
    {
        let tile = "CMT";
        let bel = "CMT_TOP";
        ctx.collect_bit(tile, bel, "DRIVE.SYNC_BB", "1");
        for i in 0..4 {
            ctx.collect_enum_default(
                tile,
                bel,
                &format!("MUX.FREQ_BB{i}"),
                &["PLL_CLKOUT0", "PLL_CLKOUT1", "PLL_CLKOUT2", "PLL_CLKOUT3"],
                "NONE",
            );
        }
        for (pin, pll_clkout) in [
            ("FREQREFCLK", "PLL_CLKOUT0"),
            ("MEMREFCLK", "PLL_CLKOUT1"),
            ("SYNCIN", "PLL_CLKOUT2"),
        ] {
            ctx.collect_enum(
                tile,
                bel,
                &format!("MUX.{pin}"),
                &["FREQ_BB0", "FREQ_BB1", "FREQ_BB2", "FREQ_BB3", pll_clkout],
            );
        }
        if edev.grids.first().unwrap().regs > 1 {
            ctx.collect_bit(tile, bel, "ENABLE.SYNC_BB_N", "1");
            ctx.collect_bit(tile, bel, "BUF.SYNC_BB.D", "1");
            let mut diff = ctx.state.get_diff(tile, bel, "BUF.SYNC_BB.U", "1");
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE.SYNC_BB_N"), true, false);
            ctx.tiledb
                .insert(tile, bel, "BUF.SYNC_BB.U", xlat_bit(diff));
            for i in 0..4 {
                ctx.collect_bit_wide(tile, bel, &format!("ENABLE.FREQ_BB{i}_N"), "1");
                ctx.collect_bit_wide(tile, bel, &format!("BUF.FREQ_BB{i}.D"), "1");
                let mut diff = ctx
                    .state
                    .get_diff(tile, bel, format!("BUF.FREQ_BB{i}.U"), "1");
                diff.apply_bitvec_diff_int(
                    ctx.tiledb.item(tile, bel, &format!("ENABLE.FREQ_BB{i}_N")),
                    3,
                    0,
                );
                ctx.tiledb
                    .insert(tile, bel, format!("BUF.FREQ_BB{i}.U"), xlat_bit_wide(diff));
            }
            let diff_bot = ctx.state.get_diff(tile, bel, "ENABLE.SYNC_BB", "BOT");
            let diff_top = ctx.state.get_diff(tile, bel, "ENABLE.SYNC_BB", "TOP");
            let (diff_bot, diff_top, diff_com) = Diff::split(diff_bot, diff_top);
            ctx.tiledb
                .insert(tile, "CMT_BOT", "ENABLE.SYNC_BB", xlat_bit(diff_top));
            ctx.tiledb.insert(
                tile,
                "CMT_TOP",
                "ENABLE.SYNC_BB",
                xlat_bit_wide(diff_bot.combine(&diff_com)),
            );
        }
    }
    {
        let tile = "CMT";
        let bel = "HCLK_CMT";
        ctx.collect_bit(tile, bel, "ENABLE.CKINT0", "1");
        ctx.collect_bit(tile, bel, "ENABLE.CKINT1", "1");
        ctx.collect_bit(tile, bel, "ENABLE.CKINT2", "1");
        ctx.collect_bit(tile, bel, "ENABLE.CKINT3", "1");
        for i in 0..12 {
            let diff = ctx
                .state
                .get_diff(tile, bel, "MUX.HOUT0", format!("HCLK{i}.EXCL"))
                .combine(
                    &!ctx
                        .state
                        .peek_diff(tile, bel, "MUX.HOUT0", format!("HCLK{i}")),
                );
            ctx.tiledb
                .insert(tile, bel, format!("ENABLE.HCLK{i}"), xlat_bit(diff));
        }
        for i in 4..14 {
            let diff = ctx
                .state
                .get_diff(tile, bel, "MUX.HOUT0", format!("HIN{i}.EXCL"))
                .combine(
                    &!ctx
                        .state
                        .peek_diff(tile, bel, "MUX.HOUT0", format!("HIN{i}")),
                );
            ctx.tiledb
                .insert(tile, bel, format!("ENABLE.HIN{i}"), xlat_bit(diff));
        }
        for i in 0..4 {
            let mut diffs_pref = vec![
                ("NONE".to_string(), Diff::default()),
                (
                    "CLKOUT".to_string(),
                    ctx.state
                        .get_diff(tile, bel, format!("MUX.PHASER_REF_BOUNCE{i}"), "CLKOUT"),
                ),
                (
                    "TMUXOUT".to_string(),
                    ctx.state
                        .get_diff(tile, bel, format!("MUX.PHASER_REF_BOUNCE{i}"), "TMUXOUT"),
                ),
            ];
            let diff_pref_ccio = ctx
                .state
                .get_diff(tile, bel, "MUX.HOUT0", format!("CCIO{i}.EXCL"))
                .combine(&!ctx.state.peek_diff(
                    tile,
                    bel,
                    "MUX.HOUT0",
                    format!("PHASER_REF_BOUNCE{i}"),
                ));
            let mut diffs_fbb = vec![
                ("NONE".to_string(), Diff::default()),
                (
                    format!("CKINT{i}"),
                    ctx.state
                        .get_diff(tile, bel, format!("MUX.FREQ_BB{i}"), format!("CKINT{i}")),
                ),
            ];
            let diff_fbb_ccio =
                ctx.state
                    .get_diff(tile, bel, format!("MUX.FREQ_BB{i}"), format!("CCIO{i}"));
            let (diff_pref_ccio, diff_fbb_ccio, diff_en_ccio) =
                Diff::split(diff_pref_ccio, diff_fbb_ccio);
            diffs_pref.push((format!("CCIO{i}"), diff_pref_ccio));
            diffs_fbb.push((format!("CCIO{i}"), diff_fbb_ccio));
            ctx.tiledb.insert(
                tile,
                bel,
                format!("MUX.PHASER_REF_BOUNCE{i}"),
                xlat_enum(diffs_pref),
            );
            ctx.tiledb
                .insert(tile, bel, format!("MUX.FREQ_BB{i}"), xlat_enum(diffs_fbb));
            ctx.tiledb
                .insert(tile, bel, format!("ENABLE.CCIO{i}"), xlat_bit(diff_en_ccio));
        }
        for i in 0..14 {
            let mut vals = vec![];
            for j in 0..12 {
                vals.push(format!("HCLK{j}"));
            }
            for j in 4..14 {
                vals.push(format!("HIN{j}"));
            }
            for j in 0..4 {
                vals.push(format!("PHASER_REF_BOUNCE{j}"));
            }
            for pin in [
                "CLKOUT0",
                "CLKOUT0B",
                "CLKOUT1",
                "CLKOUT1B",
                "CLKOUT2",
                "CLKOUT2B",
                "CLKOUT3",
                "CLKOUT3B",
                "CLKOUT4",
                "CLKOUT5",
                "CLKOUT6",
                "CLKFBOUT",
                "CLKFBOUTB",
                "TMUXOUT",
            ] {
                vals.push(format!("MMCM_{pin}"));
            }
            for pin in [
                "CLKOUT0", "CLKOUT1", "CLKOUT2", "CLKOUT3", "CLKOUT4", "CLKOUT5", "CLKFBOUT",
                "TMUXOUT",
            ] {
                vals.push(format!("PLL_{pin}"));
            }
            ctx.collect_enum_default_ocd(
                tile,
                bel,
                &format!("MUX.HOUT{i}"),
                &vals,
                "NONE",
                OcdMode::Mux,
            );
        }
        for i in 0..2 {
            for ud in ['U', 'D'] {
                let mut vals = vec![];
                for j in 0..4 {
                    vals.push(format!("RCLK{j}"));
                }
                for j in 0..12 {
                    vals.push(format!("HCLK{j}"));
                }
                ctx.collect_enum_default_ocd(
                    tile,
                    bel,
                    &format!("MUX.LCLK{i}_CMT_{ud}"),
                    &vals,
                    "NONE",
                    OcdMode::Mux,
                );
            }
        }
        for i in 0..4 {
            let oi = i ^ 1;
            let diff_a = ctx
                .state
                .peek_diff(tile, bel, format!("MUX.PERF{i}"), format!("MMCM_PERF{i}"))
                .clone();
            let diff_b = ctx
                .state
                .peek_diff(tile, bel, format!("MUX.PERF{oi}"), format!("MMCM_PERF{i}"))
                .clone();
            let (_, _, diff) = Diff::split(diff_a, diff_b);
            ctx.tiledb
                .insert(tile, "MMCM", format!("ENABLE.PERF{i}"), xlat_bit(diff));
            let diff_a = ctx
                .state
                .peek_diff(
                    tile,
                    bel,
                    format!("MUX.PERF{i}"),
                    format!("PHASER_IN_RCLK{i}"),
                )
                .clone();
            let diff_b = ctx
                .state
                .peek_diff(
                    tile,
                    bel,
                    format!("MUX.PERF{oi}"),
                    format!("PHASER_IN_RCLK{i}"),
                )
                .clone();
            let (_, _, diff) = Diff::split(diff_a, diff_b);
            ctx.tiledb
                .insert(tile, format!("PHASER_IN{i}"), "ENABLE.RCLK", xlat_bit(diff));
        }
        for i in 0..4 {
            let mut diffs = vec![("NONE".to_string(), Diff::default())];
            for j in 0..4 {
                let mut diff = ctx.state.get_diff(
                    tile,
                    bel,
                    format!("MUX.PERF{i}"),
                    format!("PHASER_IN_RCLK{j}"),
                );
                diff.apply_bit_diff(
                    ctx.tiledb
                        .item(tile, &format!("PHASER_IN{j}"), "ENABLE.RCLK"),
                    true,
                    false,
                );
                diffs.push((format!("PHASER_IN_RCLK{j}"), diff));
            }
            for j in [i, i ^ 1] {
                let mut diff =
                    ctx.state
                        .get_diff(tile, bel, format!("MUX.PERF{i}"), format!("MMCM_PERF{j}"));
                diff.apply_bit_diff(
                    ctx.tiledb.item(tile, "MMCM", &format!("ENABLE.PERF{j}")),
                    true,
                    false,
                );
                diffs.push((format!("MMCM_PERF{j}"), diff));
            }
            ctx.tiledb.insert(
                tile,
                bel,
                format!("MUX.PERF{i}"),
                xlat_enum_ocd(diffs, OcdMode::Mux),
            );
        }
    }
    for tile in ["HCLK_IOI_HR", "HCLK_IOI_HP"] {
        if ctx.has_tile(tile) {
            let bel = "HCLK_IOI";
            ctx.collect_bit(tile, bel, "ENABLE.PERF0", "1");
            ctx.collect_bit(tile, bel, "ENABLE.PERF1", "1");
            ctx.collect_bit(tile, bel, "ENABLE.PERF2", "1");
            ctx.collect_bit(tile, bel, "ENABLE.PERF3", "1");
        }
    }
}
