use prjcombine_entity::EntityId;
use prjcombine_re_collector::{
    diff::{Diff, OcdMode},
    legacy::{
        extract_bitvec_val_part_legacy, xlat_bit_legacy, xlat_bit_wide_legacy, xlat_bitvec_legacy,
        xlat_enum_legacy, xlat_enum_legacy_ocd,
    },
};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{
    bitvec::BitVec,
    bsdata::{BitRectId, TileBit, TileItem},
};
use prjcombine_virtex4::defs::{self, virtex7::tcls};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::{
            DynProp,
            extra::{ExtraKeyLegacyAttr, ExtraTileMaybe},
            pip::PinFar,
            relation::Delta,
        },
    },
};

use super::{clk::ColPair, gt::TouchHout};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    let mut ctx = FuzzCtx::new(session, backend, tcls::CMT_FIFO);
    {
        let mut bctx = ctx.bel(defs::bslots::IN_FIFO);
        let mode = "IN_FIFO";
        bctx.test_manual_legacy("PRESENT", "1").mode(mode).commit();

        bctx.mode(mode)
            .test_enum_legacy("ALMOST_EMPTY_VALUE", &["1", "2"]);
        bctx.mode(mode)
            .test_enum_legacy("ALMOST_FULL_VALUE", &["1", "2"]);
        bctx.mode(mode)
            .test_enum_legacy("ARRAY_MODE", &["ARRAY_MODE_4_X_8", "ARRAY_MODE_4_X_4"]);
        bctx.mode(mode)
            .test_enum_legacy("SLOW_RD_CLK", &["FALSE", "TRUE"]);
        bctx.mode(mode)
            .test_enum_legacy("SLOW_WR_CLK", &["FALSE", "TRUE"]);
        bctx.mode(mode)
            .test_enum_legacy("SYNCHRONOUS_MODE", &["FALSE", "TRUE"]);
        bctx.mode(mode).test_multi_attr_bin_legacy("SPARE", 4);

        bctx.build()
            .mutex("MUX.WRCLK", "PHASER")
            .test_manual_legacy("MUX.WRCLK", "PHASER")
            .pip("WRCLK", "PHASER_WRCLK")
            .commit();
        bctx.build()
            .mutex("MUX.WRCLK", "INT")
            .test_manual_legacy("MUX.WRCLK", "INT")
            .pin_pips("WRCLK")
            .commit();
        bctx.build()
            .mutex("MUX.WREN", "PHASER")
            .test_manual_legacy("MUX.WREN", "PHASER")
            .pip("WREN", "PHASER_WREN")
            .commit();
        bctx.build()
            .mutex("MUX.WREN", "INT")
            .test_manual_legacy("MUX.WREN", "INT")
            .pin_pips("WREN")
            .commit();
    }
    {
        let mut bctx = ctx.bel(defs::bslots::OUT_FIFO);
        let mode = "OUT_FIFO";
        bctx.test_manual_legacy("PRESENT", "1").mode(mode).commit();

        bctx.mode(mode)
            .test_enum_legacy("ALMOST_EMPTY_VALUE", &["1", "2"]);
        bctx.mode(mode)
            .test_enum_legacy("ALMOST_FULL_VALUE", &["1", "2"]);
        bctx.mode(mode)
            .test_enum_legacy("ARRAY_MODE", &["ARRAY_MODE_8_X_4", "ARRAY_MODE_4_X_4"]);
        bctx.mode(mode)
            .test_enum_legacy("SLOW_RD_CLK", &["FALSE", "TRUE"]);
        bctx.mode(mode)
            .test_enum_legacy("SLOW_WR_CLK", &["FALSE", "TRUE"]);
        bctx.mode(mode)
            .test_enum_legacy("SYNCHRONOUS_MODE", &["FALSE", "TRUE"]);
        bctx.mode(mode)
            .test_enum_legacy("OUTPUT_DISABLE", &["FALSE", "TRUE"]);
        bctx.mode(mode).test_multi_attr_bin_legacy("SPARE", 4);

        bctx.build()
            .mutex("MUX.RDCLK", "PHASER")
            .test_manual_legacy("MUX.RDCLK", "PHASER")
            .pip("RDCLK", "PHASER_RDCLK")
            .commit();
        bctx.build()
            .mutex("MUX.RDCLK", "INT")
            .test_manual_legacy("MUX.RDCLK", "INT")
            .pin_pips("RDCLK")
            .commit();
        bctx.build()
            .mutex("MUX.RDEN", "PHASER")
            .test_manual_legacy("MUX.RDEN", "PHASER")
            .pip("RDEN", "PHASER_RDEN")
            .commit();
        bctx.build()
            .mutex("MUX.RDEN", "INT")
            .test_manual_legacy("MUX.RDEN", "INT")
            .pin_pips("RDEN")
            .commit();
    }

    let mut ctx = FuzzCtx::new(session, backend, tcls::CMT);

    for i in 0..4 {
        let mut bctx = ctx.bel(defs::bslots::PHASER_IN[i]);
        bctx.mode("PHASER_IN_ADV").test_inv_legacy("RST");
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
            bctx.mode("PHASER_IN_ADV")
                .test_enum_legacy(attr, &["FALSE", "TRUE"]);
        }
        bctx.mode("PHASER_IN_ADV").test_enum_legacy(
            "CLKOUT_DIV",
            &[
                "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
            ],
        );
        bctx.mode("PHASER_IN_ADV")
            .test_enum_legacy("CTL_MODE", &["HARD", "SOFT"]);
        bctx.mode("PHASER_IN")
            .test_enum_legacy("FREQ_REF_DIV", &["NONE", "DIV2", "DIV4"]);
        bctx.mode("PHASER_IN_ADV").test_enum_legacy(
            "OUTPUT_CLK_SRC",
            &[
                "PHASE_REF",
                "DELAYED_MEM_REF",
                "DELAYED_PHASE_REF",
                "DELAYED_REF",
                "FREQ_REF",
                "MEM_REF",
            ],
        );
        bctx.mode("PHASER_IN_ADV")
            .test_enum_legacy("PD_REVERSE", &["1", "2", "3", "4", "5", "6", "7", "8"]);
        bctx.mode("PHASER_IN_ADV")
            .test_enum_legacy("STG1_PD_UPDATE", &["2", "3", "4", "5", "6", "7", "8", "9"]);
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
            bctx.mode("PHASER_IN_ADV")
                .test_multi_attr_bin_legacy(attr, width);
        }
        for (attr, width) in [("FINE_DELAY", 6), ("SEL_CLK_OFFSET", 3)] {
            bctx.mode("PHASER_IN_ADV")
                .test_multi_attr_dec_legacy(attr, width);
        }
        bctx.mode("PHASER_IN_ADV")
            .mutex("MUX.PHASEREFCLK", "DQS_PAD")
            .test_manual_legacy("MUX.PHASEREFCLK", "DQS_PAD")
            .pip((PinFar, "PHASEREFCLK"), "DQS_PAD")
            .commit();
        let bel_cmt = if i < 2 {
            defs::bslots::CMT_B
        } else {
            defs::bslots::CMT_C
        };
        for pin in [
            "MRCLK0", "MRCLK1", "MRCLK0_S", "MRCLK1_S", "MRCLK0_N", "MRCLK1_N",
        ] {
            bctx.mode("PHASER_IN_ADV")
                .mutex("MUX.PHASEREFCLK", pin)
                .test_manual_legacy("MUX.PHASEREFCLK", pin)
                .pip((PinFar, "PHASEREFCLK"), (bel_cmt, pin))
                .commit();
        }
    }
    for i in 0..4 {
        let mut bctx = ctx.bel(defs::bslots::PHASER_OUT[i]);
        bctx.mode("PHASER_OUT_ADV").test_inv_legacy("RST");
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
            bctx.mode("PHASER_OUT_ADV")
                .test_enum_legacy(attr, &["FALSE", "TRUE"]);
        }
        bctx.mode("PHASER_OUT_ADV").test_enum_legacy(
            "CLKOUT_DIV",
            &[
                "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
            ],
        );
        bctx.mode("PHASER_OUT_ADV")
            .test_enum_legacy("CTL_MODE", &["HARD", "SOFT"]);
        bctx.mode("PHASER_OUT_ADV")
            .attr("STG1_BYPASS", "PHASE_REF")
            .test_enum_legacy(
                "OUTPUT_CLK_SRC",
                &["PHASE_REF", "DELAYED_PHASE_REF", "DELAYED_REF", "FREQ_REF"],
            );
        bctx.mode("PHASER_OUT_ADV")
            .attr("OUTPUT_CLK_SRC", "PHASE_REF")
            .test_enum_legacy("STG1_BYPASS", &["PHASE_REF", "FREQ_REF"]);
        for (attr, width) in [("CLKOUT_DIV_ST", 4), ("TEST_OPT", 11)] {
            bctx.mode("PHASER_OUT_ADV")
                .test_multi_attr_bin_legacy(attr, width);
        }
        bctx.mode("PHASER_OUT_ADV")
            .attr("TEST_OPT", "")
            .test_multi_attr_bin_legacy("PO", 3);
        for (attr, width) in [("COARSE_DELAY", 6), ("FINE_DELAY", 6), ("OCLK_DELAY", 6)] {
            bctx.mode("PHASER_OUT_ADV")
                .test_multi_attr_dec_legacy(attr, width);
        }
        let bel_cmt = if i < 2 {
            defs::bslots::CMT_B
        } else {
            defs::bslots::CMT_C
        };
        for pin in [
            "MRCLK0", "MRCLK1", "MRCLK0_S", "MRCLK1_S", "MRCLK0_N", "MRCLK1_N",
        ] {
            bctx.mode("PHASER_OUT_ADV")
                .mutex("MUX.PHASEREFCLK", pin)
                .test_manual_legacy("MUX.PHASEREFCLK", pin)
                .pip((PinFar, "PHASEREFCLK"), (bel_cmt, pin))
                .commit();
        }
    }
    {
        let mut bctx = ctx.bel(defs::bslots::PHASER_REF);
        for pin in ["RST", "PWRDWN"] {
            bctx.mode("PHASER_REF").test_inv_legacy(pin);
        }
        for attr in ["PHASER_REF_EN", "SEL_SLIPD", "SUP_SEL_AREG"] {
            bctx.mode("PHASER_REF")
                .test_enum_legacy(attr, &["FALSE", "TRUE"]);
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
            bctx.mode("PHASER_REF")
                .test_multi_attr_bin_legacy(attr, width);
        }
        for (attr, width) in [
            ("CONTROL_0", 16),
            ("CONTROL_1", 16),
            ("CONTROL_2", 16),
            ("CONTROL_3", 16),
            ("CONTROL_4", 16),
            ("CONTROL_5", 16),
        ] {
            bctx.mode("PHASER_REF")
                .test_multi_attr_hex_legacy(attr, width);
        }
        for (attr, width) in [("LOCK_CNT", 10), ("LOCK_FB_DLY", 5), ("LOCK_REF_DLY", 5)] {
            bctx.mode("PHASER_REF")
                .test_multi_attr_dec_legacy(attr, width);
        }
    }
    {
        let mut bctx = ctx.bel(defs::bslots::PHY_CONTROL);
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
            bctx.mode("PHY_CONTROL")
                .test_enum_legacy(attr, &["FALSE", "TRUE"]);
        }
        bctx.mode("PHY_CONTROL")
            .test_enum_legacy("CLK_RATIO", &["1", "2", "4", "8"]);
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
            bctx.mode("PHY_CONTROL")
                .test_multi_attr_dec_legacy(attr, width);
        }
        for (attr, width) in [("AO_WRLVL_EN", 4), ("SPARE", 1)] {
            bctx.mode("PHY_CONTROL")
                .test_multi_attr_bin_legacy(attr, width);
        }
    }
    for (bel, bel_name) in [
        (defs::bslots::PLL[0], "MMCM"),
        (defs::bslots::PLL[1], "PLL"),
    ] {
        let mut bctx = ctx.bel(bel);
        let use_calc = if bel == defs::bslots::PLL[0] {
            "MMCMADV_*_USE_CALC"
        } else {
            "PLLADV_*_USE_CALC"
        };
        let mode = if bel == defs::bslots::PLL[0] {
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
            if matches!(pin, "PSEN" | "PSINCDEC") && bel == defs::bslots::PLL[1] {
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
        if bel == defs::bslots::PLL[0] {
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
        if bel == defs::bslots::PLL[0] {
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
        if bel == defs::bslots::PLL[0] {
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
            if bel == defs::bslots::PLL[0] {
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
            .test_manual_legacy("DRP_MASK", "1")
            .pin("DWE")
            .commit();

        for pin in ["CLKIN1", "CLKIN2", "CLKFBIN"] {
            bctx.build()
                .mutex(format!("MUX.{pin}"), format!("{pin}_CKINT"))
                .test_manual_legacy(format!("MUX.{pin}"), format!("{pin}_CKINT"))
                .pip(pin, format!("{pin}_CKINT"))
                .commit();
            bctx.build()
                .mutex(format!("MUX.{pin}"), format!("{pin}_HCLK"))
                .test_manual_legacy(format!("MUX.{pin}"), format!("{pin}_HCLK"))
                .pip(pin, format!("{pin}_HCLK"))
                .commit();
            for i in 0..4 {
                bctx.build()
                    .mutex(format!("MUX.{pin}"), format!("FREQ_BB{i}"))
                    .test_manual_legacy(format!("MUX.{pin}"), format!("FREQ_BB{i}"))
                    .pip(pin, format!("FREQ_BB{i}_IN"))
                    .commit();
            }
            let opin = if pin == "CLKFBIN" {
                "CLKIN1"
            } else {
                "CLKFBIN"
            };
            for i in 0..4 {
                bctx.build()
                    .tile_mutex("CCIO", "USE")
                    .mutex(format!("MUX.{pin}_HCLK"), format!("CCIO{i}"))
                    .mutex(format!("MUX.{opin}_HCLK"), format!("CCIO{i}"))
                    .pip(
                        (defs::bslots::HCLK_CMT, format!("{bel_name}_{opin}")),
                        (defs::bslots::HCLK_CMT, format!("CCIO{i}")),
                    )
                    .test_manual_legacy(format!("MUX.{pin}_HCLK"), format!("PHASER_REF_BOUNCE{i}"))
                    .pip(
                        (defs::bslots::HCLK_CMT, format!("{bel_name}_{pin}")),
                        (defs::bslots::HCLK_CMT, format!("CCIO{i}")),
                    )
                    .commit();
            }
            for i in 0..4 {
                bctx.build()
                    .tile_mutex("PHASER_REF_BOUNCE", "USE")
                    .mutex(format!("MUX.{pin}_HCLK"), format!("PHASER_REF_BOUNCE{i}"))
                    .mutex(format!("MUX.{opin}_HCLK"), format!("PHASER_REF_BOUNCE{i}"))
                    .pip(
                        (defs::bslots::HCLK_CMT, format!("{bel_name}_{opin}")),
                        (defs::bslots::HCLK_CMT, format!("PHASER_REF_BOUNCE{i}")),
                    )
                    .test_manual_legacy(format!("MUX.{pin}_HCLK"), format!("PHASER_REF_BOUNCE{i}"))
                    .pip(
                        (defs::bslots::HCLK_CMT, format!("{bel_name}_{pin}")),
                        (defs::bslots::HCLK_CMT, format!("PHASER_REF_BOUNCE{i}")),
                    )
                    .commit();
            }
            for i in 0..12 {
                bctx.build()
                    .global_mutex("HCLK", "USE")
                    .mutex(format!("MUX.{pin}_HCLK"), format!("HCLK{i}"))
                    .mutex(format!("MUX.{opin}_HCLK"), format!("HCLK{i}"))
                    .pip(
                        (defs::bslots::HCLK_CMT, format!("{bel_name}_{opin}")),
                        (defs::bslots::HCLK_CMT, format!("HCLK{i}")),
                    )
                    .test_manual_legacy(format!("MUX.{pin}_HCLK"), format!("HCLK{i}"))
                    .pip(
                        (defs::bslots::HCLK_CMT, format!("{bel_name}_{pin}")),
                        (defs::bslots::HCLK_CMT, format!("HCLK{i}")),
                    )
                    .commit();
            }
            for i in 0..4 {
                bctx.build()
                    .global_mutex("RCLK", "USE")
                    .mutex(format!("MUX.{pin}_HCLK"), format!("RCLK{i}"))
                    .mutex(format!("MUX.{opin}_HCLK"), format!("RCLK{i}"))
                    .pip(
                        (defs::bslots::HCLK_CMT, format!("{bel_name}_{opin}")),
                        (defs::bslots::HCLK_CMT, format!("RCLK{i}")),
                    )
                    .test_manual_legacy(format!("MUX.{pin}_HCLK"), format!("RCLK{i}"))
                    .pip(
                        (defs::bslots::HCLK_CMT, format!("{bel_name}_{pin}")),
                        (defs::bslots::HCLK_CMT, format!("RCLK{i}")),
                    )
                    .commit();
            }
            for i in 4..14 {
                bctx.build()
                    .tile_mutex("HIN", "USE")
                    .mutex(format!("MUX.{pin}_HCLK"), format!("HIN{i}"))
                    .mutex(format!("MUX.{opin}_HCLK"), format!("HIN{i}"))
                    .pip(
                        (defs::bslots::HCLK_CMT, format!("{bel_name}_{opin}")),
                        (defs::bslots::HCLK_CMT, format!("HIN{i}")),
                    )
                    .test_manual_legacy(format!("MUX.{pin}_HCLK"), format!("HIN{i}"))
                    .pip(
                        (defs::bslots::HCLK_CMT, format!("{bel_name}_{pin}")),
                        (defs::bslots::HCLK_CMT, format!("HIN{i}")),
                    )
                    .commit();
            }
        }
        bctx.build()
            .mutex("MUX.CLKFBIN", "CLKFBOUT")
            .test_manual_legacy("MUX.CLKFBIN", "CLKFBOUT")
            .pip("CLKFBIN", "CLKFB")
            .commit();

        for i in 0..4 {
            bctx.test_manual_legacy(format!("BUF.CLKOUT{i}_FREQ_BB"), "1")
                .pip(format!("FREQ_BB_OUT{i}"), format!("CLKOUT{i}"))
                .commit();
        }

        if bel == defs::bslots::PLL[0] {
            for i in 0..4 {
                for pin in ["CLKOUT0", "CLKOUT1", "CLKOUT2", "CLKOUT3", "CLKFBOUT"] {
                    bctx.build()
                        .tile_mutex("PERF", "TEST")
                        .mutex(format!("MUX.PERF{i}"), pin)
                        .pip(
                            (defs::bslots::HCLK_CMT, format!("PERF{i}")),
                            (defs::bslots::HCLK_CMT, format!("MMCM_PERF{i}")),
                        )
                        .test_manual_legacy(format!("MUX.PERF{i}"), pin)
                        .pip(format!("PERF{i}"), pin)
                        .commit();
                }
            }
        }
    }
    for i in 0..2 {
        let mut bctx = ctx.bel(defs::bslots::BUFMRCE[i]);
        bctx.test_manual_legacy("ENABLE", "1")
            .mode("BUFMRCE")
            .commit();
        bctx.mode("BUFMRCE").test_inv_legacy("CE");
        bctx.mode("BUFMRCE")
            .test_enum_legacy("INIT_OUT", &["0", "1"]);
        bctx.mode("BUFMRCE")
            .test_enum_legacy("CE_TYPE", &["SYNC", "ASYNC"]);
        let bel_other = defs::bslots::BUFMRCE[i ^ 1];
        for j in 4..14 {
            bctx.build()
                .tile_mutex("HIN", "USE")
                .mutex("MUX.I", format!("HIN{j}"))
                .bel_mutex(bel_other, "MUX.I", format!("HIN{j}"))
                .pip(
                    (bel_other, "I"),
                    (defs::bslots::HCLK_CMT, format!("HIN{j}")),
                )
                .test_manual_legacy("MUX.I", format!("HIN{j}"))
                .pip("I", (defs::bslots::HCLK_CMT, format!("HIN{j}")))
                .commit();
        }
        for j in 0..2 {
            bctx.build()
                .tile_mutex("CKINT", "USE")
                .mutex("MUX.I", format!("CKINT{j}"))
                .bel_mutex(bel_other, "MUX.I", format!("CKINT{j}"))
                .pip(
                    (bel_other, "I"),
                    (defs::bslots::HCLK_CMT, format!("CKINT{j}")),
                )
                .test_manual_legacy("MUX.I", format!("CKINT{j}"))
                .pip("I", (defs::bslots::HCLK_CMT, format!("CKINT{j}")))
                .commit();
        }
        let ccio = i * 3;
        bctx.build()
            .tile_mutex("CCIO", "USE")
            .mutex("MUX.I", format!("CCIO{ccio}"))
            .prop(TouchHout(0))
            .bel_mutex(defs::bslots::HCLK_CMT, "MUX.HOUT0", format!("CCIO{ccio}"))
            .pip(
                (defs::bslots::HCLK_CMT, "HOUT0"),
                (defs::bslots::HCLK_CMT, format!("CCIO{ccio}")),
            )
            .test_manual_legacy("MUX.I", format!("CCIO{ccio}"))
            .pip("I", (defs::bslots::HCLK_CMT, format!("CCIO{ccio}")))
            .commit();
    }
    if edev.chips.first().unwrap().regs > 1 {
        let mut bctx = ctx.bel(defs::bslots::CMT_A);
        bctx.build()
            .force_bel_name("CMT_BOT")
            .tile_mutex("SYNC_BB", "DRIVE")
            .pip(
                (defs::bslots::PHY_CONTROL, "SYNC_BB"),
                (PinFar, defs::bslots::PHY_CONTROL, "PHYCTLEMPTY"),
            )
            .related_tile_mutex(
                Delta::new(0, -50, tcls::CMT),
                "SYNC_BB",
                "TEST_SOURCE_DUMMY",
            )
            .extra_tile_attr_legacy(
                Delta::new(0, -50, tcls::CMT),
                "CMT_TOP",
                "ENABLE.SYNC_BB_N",
                "1",
            )
            .test_manual_legacy("BUF.SYNC_BB.D", "1")
            .pip("SYNC_BB_S", "SYNC_BB")
            .commit();
        bctx.build()
            .force_bel_name("CMT_BOT")
            .tile_mutex("SYNC_BB", "TEST_SOURCE_U")
            .related_tile_mutex(Delta::new(0, -50, tcls::CMT), "SYNC_BB", "DRIVE")
            .related_pip(
                Delta::new(0, -50, tcls::CMT),
                (defs::bslots::PHY_CONTROL, "SYNC_BB"),
                (PinFar, defs::bslots::PHY_CONTROL, "PHYCTLEMPTY"),
            )
            .related_pip(
                Delta::new(0, -50, tcls::CMT),
                (defs::bslots::CMT_D, "SYNC_BB_N"),
                (defs::bslots::CMT_D, "SYNC_BB"),
            )
            .pip(
                (PinFar, defs::bslots::PHY_CONTROL, "PHYCTLMSTREMPTY"),
                (defs::bslots::PHY_CONTROL, "SYNC_BB"),
            )
            .test_manual_legacy("BUF.SYNC_BB.U", "1")
            .pip("SYNC_BB", "SYNC_BB_S")
            .commit();
        for i in 0..4 {
            bctx.build()
                .force_bel_name("CMT_BOT")
                .tile_mutex("FREQ_BB", "DRIVE")
                .pip(
                    (defs::bslots::CMT_B, format!("FREQ_BB{i}")),
                    (defs::bslots::CMT_B, format!("FREQ_BB{i}_MUX")),
                )
                .related_tile_mutex(
                    Delta::new(0, -50, tcls::CMT),
                    "FREQ_BB",
                    "TEST_SOURCE_DUMMY",
                )
                .extra_tile_attr_legacy(
                    Delta::new(0, -50, tcls::CMT),
                    "CMT_TOP",
                    format!("ENABLE.FREQ_BB{i}_N"),
                    "1",
                )
                .test_manual_legacy(format!("BUF.FREQ_BB{i}.D"), "1")
                .pip(format!("FREQ_BB{i}_S"), format!("FREQ_BB{i}"))
                .commit();
            bctx.build()
                .force_bel_name("CMT_BOT")
                .tile_mutex("FREQ_BB", "TEST_SOURCE_U")
                .related_tile_mutex(Delta::new(0, -50, tcls::CMT), "FREQ_BB", "DRIVE")
                .related_pip(
                    Delta::new(0, -50, tcls::CMT),
                    (defs::bslots::CMT_B, format!("FREQ_BB{i}")),
                    (defs::bslots::CMT_B, format!("FREQ_BB{i}_MUX")),
                )
                .related_pip(
                    Delta::new(0, -50, tcls::CMT),
                    (defs::bslots::CMT_D, format!("FREQ_BB{i}_N")),
                    (defs::bslots::CMT_D, format!("FREQ_BB{i}")),
                )
                .pip(
                    (defs::bslots::PLL[0], format!("FREQ_BB{i}_IN")),
                    (defs::bslots::CMT_A, format!("FREQ_BB{i}")),
                )
                .test_manual_legacy(format!("BUF.FREQ_BB{i}.U"), "1")
                .pip(format!("FREQ_BB{i}"), format!("FREQ_BB{i}_S"))
                .commit();
        }
    }
    if edev.chips.first().unwrap().regs > 1 {
        let mut bctx = ctx.bel(defs::bslots::CMT_D);
        bctx.build()
            .force_bel_name("CMT_TOP")
            .tile_mutex("SYNC_BB", "DRIVE")
            .pip(
                (defs::bslots::PHY_CONTROL, "SYNC_BB"),
                (PinFar, defs::bslots::PHY_CONTROL, "PHYCTLEMPTY"),
            )
            .related_tile_mutex(Delta::new(0, 50, tcls::CMT), "SYNC_BB", "TEST_SOURCE_DUMMY")
            .extra_tile_attr_legacy(
                Delta::new(0, 50, tcls::CMT),
                "CMT_BOT",
                "ENABLE.SYNC_BB_S",
                "1",
            )
            .test_manual_legacy("BUF.SYNC_BB.U", "1")
            .pip("SYNC_BB_N", "SYNC_BB")
            .commit();
        bctx.build()
            .force_bel_name("CMT_TOP")
            .tile_mutex("SYNC_BB", "TEST_SOURCE_D")
            .related_tile_mutex(Delta::new(0, 50, tcls::CMT), "SYNC_BB", "DRIVE")
            .related_pip(
                Delta::new(0, 50, tcls::CMT),
                (defs::bslots::PHY_CONTROL, "SYNC_BB"),
                (PinFar, defs::bslots::PHY_CONTROL, "PHYCTLEMPTY"),
            )
            .related_pip(
                Delta::new(0, 50, tcls::CMT),
                (defs::bslots::CMT_A, "SYNC_BB_S"),
                (defs::bslots::CMT_A, "SYNC_BB"),
            )
            .pip(
                (PinFar, defs::bslots::PHY_CONTROL, "PHYCTLMSTREMPTY"),
                (defs::bslots::PHY_CONTROL, "SYNC_BB"),
            )
            .test_manual_legacy("BUF.SYNC_BB.D", "1")
            .pip("SYNC_BB", "SYNC_BB_N")
            .commit();
        for i in 0..4 {
            bctx.build()
                .force_bel_name("CMT_TOP")
                .tile_mutex("FREQ_BB", "DRIVE")
                .pip(
                    (defs::bslots::CMT_B, format!("FREQ_BB{i}")),
                    (defs::bslots::CMT_B, format!("FREQ_BB{i}_MUX")),
                )
                .related_tile_mutex(Delta::new(0, 50, tcls::CMT), "FREQ_BB", "TEST_SOURCE_DUMMY")
                .extra_tile_attr_legacy(
                    Delta::new(0, 50, tcls::CMT),
                    "CMT_BOT",
                    format!("ENABLE.FREQ_BB{i}_S"),
                    "1",
                )
                .test_manual_legacy(format!("BUF.FREQ_BB{i}.U"), "1")
                .pip(format!("FREQ_BB{i}_N"), format!("FREQ_BB{i}"))
                .commit();
            bctx.build()
                .force_bel_name("CMT_TOP")
                .tile_mutex("FREQ_BB", "TEST_SOURCE_D")
                .related_tile_mutex(Delta::new(0, 50, tcls::CMT), "FREQ_BB", "DRIVE")
                .related_pip(
                    Delta::new(0, 50, tcls::CMT),
                    (defs::bslots::CMT_B, format!("FREQ_BB{i}")),
                    (defs::bslots::CMT_B, format!("FREQ_BB{i}_MUX")),
                )
                .related_pip(
                    Delta::new(0, 50, tcls::CMT),
                    (defs::bslots::CMT_A, format!("FREQ_BB{i}_S")),
                    (defs::bslots::CMT_A, format!("FREQ_BB{i}")),
                )
                .pip(
                    (defs::bslots::PLL[0], format!("FREQ_BB{i}_IN")),
                    (defs::bslots::CMT_A, format!("FREQ_BB{i}")),
                )
                .test_manual_legacy(format!("BUF.FREQ_BB{i}.D"), "1")
                .pip(format!("FREQ_BB{i}"), format!("FREQ_BB{i}_N"))
                .commit();
        }
    }
    {
        let mut bctx = ctx.bel(defs::bslots::CMT_B);
        for i in 0..4 {
            bctx.build()
                .force_bel_name("CMT_BOT")
                .tile_mutex("FREQ_BB", "TEST")
                .test_manual_legacy(format!("ENABLE.FREQ_BB{i}"), "1")
                .pip(
                    (defs::bslots::PLL[0], format!("FREQ_BB{i}_IN")),
                    (defs::bslots::CMT_A, format!("FREQ_BB{i}")),
                )
                .commit();
            for j in 0..4 {
                bctx.build()
                    .force_bel_name("CMT_BOT")
                    .tile_mutex("FREQ_BB", "DRIVE_MMCM")
                    .mutex(format!("MUX.FREQ_BB{i}"), format!("MMCM_CLKOUT{j}"))
                    .pip(
                        (defs::bslots::PLL[0], format!("FREQ_BB_OUT{j}")),
                        (defs::bslots::PLL[0], format!("CLKOUT{j}")),
                    )
                    .pip(
                        (defs::bslots::PLL[0], format!("FREQ_BB{i}_IN")),
                        (defs::bslots::CMT_A, format!("FREQ_BB{i}")),
                    )
                    .test_manual_legacy(format!("MUX.FREQ_BB{i}"), format!("MMCM_CLKOUT{j}"))
                    .pip(format!("FREQ_BB{i}_MUX"), format!("MMCM_FREQ_BB{j}"))
                    .pip(format!("FREQ_BB{i}"), format!("FREQ_BB{i}_MUX"))
                    .commit();
            }
        }
    }
    {
        let mut bctx = ctx.bel(defs::bslots::CMT_C);
        bctx.build()
            .force_bel_name("CMT_TOP")
            .tile_mutex("SYNC_BB", "USE")
            .pip(
                (PinFar, defs::bslots::PHY_CONTROL, "PHYCTLMSTREMPTY"),
                (defs::bslots::PHY_CONTROL, "SYNC_BB"),
            )
            .test_manual_legacy("DRIVE.SYNC_BB", "1")
            .pip(
                (defs::bslots::PHY_CONTROL, "SYNC_BB"),
                (PinFar, defs::bslots::PHY_CONTROL, "PHYCTLEMPTY"),
            )
            .commit();
        if edev.chips.first().unwrap().regs > 1 {
            bctx.build()
                .force_bel_name("CMT_TOP")
                .tile_mutex("SYNC_BB", "TEST")
                .no_related(Delta::new(0, -50, tcls::CMT))
                .has_related(Delta::new(0, 50, tcls::CMT))
                .test_manual_legacy("ENABLE.SYNC_BB", "BOT")
                .pip(
                    (PinFar, defs::bslots::PHY_CONTROL, "PHYCTLMSTREMPTY"),
                    (defs::bslots::PHY_CONTROL, "SYNC_BB"),
                )
                .commit();
            bctx.build()
                .force_bel_name("CMT_TOP")
                .tile_mutex("SYNC_BB", "TEST")
                .no_related(Delta::new(0, 50, tcls::CMT))
                .has_related(Delta::new(0, -50, tcls::CMT))
                .test_manual_legacy("ENABLE.SYNC_BB", "TOP")
                .pip(
                    (PinFar, defs::bslots::PHY_CONTROL, "PHYCTLMSTREMPTY"),
                    (defs::bslots::PHY_CONTROL, "SYNC_BB"),
                )
                .commit();
        }
        for i in 0..4 {
            for j in 0..4 {
                bctx.build()
                    .force_bel_name("CMT_TOP")
                    .tile_mutex("FREQ_BB", "DRIVE_PLL")
                    .mutex(format!("MUX.FREQ_BB{i}"), format!("PLL_CLKOUT{j}"))
                    .pip(
                        (defs::bslots::PLL[1], format!("FREQ_BB_OUT{j}")),
                        (defs::bslots::PLL[1], format!("CLKOUT{j}")),
                    )
                    .pip(
                        (defs::bslots::PLL[0], format!("FREQ_BB{i}_IN")),
                        (defs::bslots::CMT_A, format!("FREQ_BB{i}")),
                    )
                    .test_manual_legacy(format!("MUX.FREQ_BB{i}"), format!("PLL_CLKOUT{j}"))
                    .pip(format!("FREQ_BB{i}_MUX"), format!("PLL_FREQ_BB{j}"))
                    .pip(format!("FREQ_BB{i}"), format!("FREQ_BB{i}_MUX"))
                    .commit();
            }
        }
        for (i, pin) in ["FREQREFCLK", "MEMREFCLK", "SYNCIN"]
            .into_iter()
            .enumerate()
        {
            for j in 0..4 {
                bctx.build()
                    .force_bel_name("CMT_TOP")
                    .mutex(format!("MUX.{pin}"), format!("FREQ_BB{j}"))
                    .test_manual_legacy(format!("MUX.{pin}"), format!("FREQ_BB{j}"))
                    .pip(pin, format!("FREQ_BB{j}_REF"))
                    .commit();
            }
            bctx.build()
                .force_bel_name("CMT_TOP")
                .mutex(format!("MUX.{pin}"), "PLL")
                .pip(
                    (defs::bslots::PLL[1], format!("FREQ_BB_OUT{i}")),
                    (defs::bslots::PLL[1], format!("CLKOUT{i}")),
                )
                .test_manual_legacy(format!("MUX.{pin}"), format!("PLL_CLKOUT{i}"))
                .pip(pin, format!("PLL_FREQ_BB{i}"))
                .commit();
        }
    }
    {
        let mut bctx = ctx.bel(defs::bslots::HCLK_CMT);
        for i in 0..4 {
            let mut props: Vec<Box<DynProp>> = vec![];
            for tcid in [tcls::HCLK_IO_HP, tcls::HCLK_IO_HR] {
                if !backend.edev.tile_index[tcid].is_empty() {
                    props.push(Box::new(ExtraTileMaybe::new(
                        ColPair(tcid),
                        ExtraKeyLegacyAttr::new(
                            "HCLK_IO".into(),
                            format!("ENABLE.PERF{i}"),
                            "1".into(),
                        ),
                    )));
                }
            }
            for j in [i, i ^ 1] {
                bctx.build()
                    .tile_mutex("PERF", "USE")
                    .mutex(format!("MUX.PERF{i}"), format!("MMCM_PERF{j}"))
                    .mutex(format!("MMCM_PERF{j}"), format!("PERF{i}"))
                    .props(props.clone())
                    .test_manual_legacy(format!("MUX.PERF{i}"), format!("MMCM_PERF{j}"))
                    .pip(format!("PERF{i}"), format!("MMCM_PERF{j}"))
                    .commit();
            }
            for j in 0..4 {
                bctx.build()
                    .tile_mutex("PERF", "USE")
                    .mutex(format!("MUX.PERF{i}"), format!("PHASER_IN_RCLK{j}"))
                    .mutex(format!("PHASER_IN_RCLK{j}"), format!("PERF{i}"))
                    .props(props.clone())
                    .test_manual_legacy(format!("MUX.PERF{i}"), format!("PHASER_IN_RCLK{j}"))
                    .pip(format!("PERF{i}"), format!("PHASER_IN_RCLK{j}"))
                    .commit();
            }
        }
        for i in 0..4 {
            for pin in ["CLKOUT", "TMUXOUT"] {
                bctx.build()
                    .mutex(format!("MUX.PHASER_REF_BOUNCE{i}"), pin)
                    .test_manual_legacy(format!("MUX.PHASER_REF_BOUNCE{i}"), pin)
                    .pip(format!("PHASER_REF_BOUNCE{i}"), format!("PHASER_REF_{pin}"))
                    .commit();
            }
        }
        for i in 0..2 {
            let oi = i ^ 1;
            for ud in ['U', 'D'] {
                for j in 0..12 {
                    bctx.build()
                        .global_mutex("HCLK", "USE")
                        .mutex(format!("MUX.LCLK{i}_{ud}"), format!("HCLK{j}"))
                        .mutex(format!("MUX.LCLK{oi}_{ud}"), format!("HCLK{j}"))
                        .pip(format!("LCLK{oi}_CMT_{ud}"), format!("HCLK{j}"))
                        .test_manual_legacy(format!("MUX.LCLK{i}_CMT_{ud}"), format!("HCLK{j}"))
                        .pip(format!("LCLK{i}_CMT_{ud}"), format!("HCLK{j}"))
                        .commit();
                }
                for j in 0..4 {
                    bctx.build()
                        .global_mutex("RCLK", "USE")
                        .mutex(format!("MUX.LCLK{i}_{ud}"), format!("RCLK{j}"))
                        .mutex(format!("MUX.LCLK{oi}_{ud}"), format!("RCLK{j}"))
                        .pip(format!("LCLK{oi}_CMT_{ud}"), format!("RCLK{j}"))
                        .test_manual_legacy(format!("MUX.LCLK{i}_CMT_{ud}"), format!("RCLK{j}"))
                        .pip(format!("LCLK{i}_CMT_{ud}"), format!("RCLK{j}"))
                        .commit();
                }
            }
        }
        for i in 0..14 {
            let oi = i ^ 1;
            for j in 0..12 {
                bctx.build()
                    .global_mutex("HCLK", "USE")
                    .prop(TouchHout(i))
                    .prop(TouchHout(oi))
                    .mutex(format!("MUX.HOUT{i}"), format!("HCLK{j}"))
                    .mutex(format!("MUX.HOUT{oi}"), format!("HCLK{j}"))
                    .pip(format!("HOUT{oi}"), format!("HCLK{j}"))
                    .test_manual_legacy(format!("MUX.HOUT{i}"), format!("HCLK{j}"))
                    .pip(format!("HOUT{i}"), format!("HCLK{j}"))
                    .commit();
                if i == 0 {
                    bctx.build()
                        .global_mutex("HCLK", "TEST")
                        .prop(TouchHout(i))
                        .mutex(format!("MUX.HOUT{i}"), format!("HCLK{j}"))
                        .test_manual_legacy(format!("MUX.HOUT{i}"), format!("HCLK{j}.EXCL"))
                        .pip(format!("HOUT{i}"), format!("HCLK{j}"))
                        .commit();
                }
            }
            for j in 0..4 {
                bctx.build()
                    .tile_mutex("CCIO", "USE")
                    .prop(TouchHout(i))
                    .prop(TouchHout(oi))
                    .mutex(format!("MUX.HOUT{i}"), format!("CCIO{j}"))
                    .mutex(format!("MUX.HOUT{oi}"), format!("CCIO{j}"))
                    .pip(format!("HOUT{oi}"), format!("CCIO{j}"))
                    .test_manual_legacy(format!("MUX.HOUT{i}"), format!("PHASER_REF_BOUNCE{j}"))
                    .pip(format!("HOUT{i}"), format!("CCIO{j}"))
                    .commit();
                if i == 0 {
                    bctx.build()
                        .tile_mutex("CCIO", "TEST")
                        .prop(TouchHout(i))
                        .mutex(format!("MUX.HOUT{i}"), format!("CCIO{j}"))
                        .test_manual_legacy(format!("MUX.HOUT{i}"), format!("CCIO{j}.EXCL"))
                        .pip(format!("HOUT{i}"), format!("CCIO{j}"))
                        .commit();
                }
            }
            for j in 0..4 {
                bctx.build()
                    .tile_mutex("PHASER_REF_BOUNCE", "USE")
                    .prop(TouchHout(i))
                    .prop(TouchHout(oi))
                    .mutex(format!("MUX.HOUT{i}"), format!("PHASER_REF_BOUNCE{j}"))
                    .mutex(format!("MUX.HOUT{oi}"), format!("PHASER_REF_BOUNCE{j}"))
                    .pip(format!("HOUT{oi}"), format!("PHASER_REF_BOUNCE{j}"))
                    .test_manual_legacy(format!("MUX.HOUT{i}"), format!("PHASER_REF_BOUNCE{j}"))
                    .pip(format!("HOUT{i}"), format!("PHASER_REF_BOUNCE{j}"))
                    .commit();
            }
            for j in 4..14 {
                bctx.build()
                    .tile_mutex("HIN", "USE")
                    .prop(TouchHout(i))
                    .prop(TouchHout(oi))
                    .mutex(format!("MUX.HOUT{i}"), format!("HIN{j}"))
                    .mutex(format!("MUX.HOUT{oi}"), format!("HIN{j}"))
                    .pip(format!("HOUT{oi}"), format!("HIN{j}"))
                    .test_manual_legacy(format!("MUX.HOUT{i}"), format!("HIN{j}"))
                    .pip(format!("HOUT{i}"), format!("HIN{j}"))
                    .commit();
                if i == 0 {
                    bctx.build()
                        .tile_mutex("HIN", "TEST")
                        .prop(TouchHout(i))
                        .mutex(format!("MUX.HOUT{i}"), format!("HIN{j}"))
                        .test_manual_legacy(format!("MUX.HOUT{i}"), format!("HIN{j}.EXCL"))
                        .pip(format!("HOUT{i}"), format!("HIN{j}"))
                        .commit();
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
                bctx.build()
                    .prop(TouchHout(i))
                    .mutex(format!("MUX.HOUT{i}"), format!("MMCM_{pin}"))
                    .test_manual_legacy(format!("MUX.HOUT{i}"), format!("MMCM_{pin}"))
                    .pip(format!("HOUT{i}"), format!("MMCM_OUT{j}"))
                    .commit();
            }
            for (j, pin) in [
                "CLKOUT0", "CLKOUT1", "CLKOUT2", "CLKOUT3", "CLKOUT4", "CLKOUT5", "CLKFBOUT",
                "TMUXOUT",
            ]
            .into_iter()
            .enumerate()
            {
                bctx.build()
                    .prop(TouchHout(i))
                    .mutex(format!("MUX.HOUT{i}"), format!("PLL_{pin}"))
                    .test_manual_legacy(format!("MUX.HOUT{i}"), format!("PLL_{pin}"))
                    .pip(format!("HOUT{i}"), format!("PLL_OUT{j}"))
                    .commit();
            }
        }
        for i in 0..4 {
            bctx.build()
                .tile_mutex("CKINT", "TEST")
                .test_manual_legacy(format!("ENABLE.CKINT{i}"), "1")
                .pin_pips(format!("CKINT{i}"))
                .commit();
        }
        for i in 0..4 {
            bctx.build()
                .tile_mutex("CKINT", "USE")
                .mutex(format!("MUX.FREQ_BB{i}"), format!("CKINT{i}"))
                .pin_pips(format!("CKINT{i}"))
                .test_manual_legacy(format!("MUX.FREQ_BB{i}"), format!("CKINT{i}"))
                .pip(format!("FREQ_BB{i}_MUX"), format!("CKINT{i}"))
                .commit();
            bctx.build()
                .tile_mutex("CCIO", "TEST_FREQ_BB")
                .mutex(format!("MUX.FREQ_BB{i}"), format!("CCIO{i}"))
                .test_manual_legacy(format!("MUX.FREQ_BB{i}"), format!("CCIO{i}"))
                .pip(format!("FREQ_BB{i}_MUX"), format!("CCIO{i}"))
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };
    {
        let tile = "CMT_FIFO";
        let bel = "IN_FIFO";
        ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
            .assert_empty();
        ctx.collect_enum_default_legacy(tile, bel, "ALMOST_EMPTY_VALUE", &["1", "2"], "NONE");
        ctx.collect_enum_default_legacy(tile, bel, "ALMOST_FULL_VALUE", &["1", "2"], "NONE");
        ctx.collect_enum_legacy(
            tile,
            bel,
            "ARRAY_MODE",
            &["ARRAY_MODE_4_X_8", "ARRAY_MODE_4_X_4"],
        );
        ctx.collect_bit_bi_legacy(tile, bel, "SLOW_RD_CLK", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "SLOW_WR_CLK", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "SYNCHRONOUS_MODE", "FALSE", "TRUE");
        ctx.collect_bitvec_legacy(tile, bel, "SPARE", "");
        ctx.collect_enum_legacy(tile, bel, "MUX.WRCLK", &["INT", "PHASER"]);
        ctx.collect_enum_legacy(tile, bel, "MUX.WREN", &["INT", "PHASER"]);
    }
    {
        let tile = "CMT_FIFO";
        let bel = "OUT_FIFO";
        ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
            .assert_empty();
        ctx.collect_enum_default_legacy(tile, bel, "ALMOST_EMPTY_VALUE", &["1", "2"], "NONE");
        ctx.collect_enum_default_legacy(tile, bel, "ALMOST_FULL_VALUE", &["1", "2"], "NONE");
        ctx.collect_enum_legacy(
            tile,
            bel,
            "ARRAY_MODE",
            &["ARRAY_MODE_8_X_4", "ARRAY_MODE_4_X_4"],
        );
        ctx.collect_bit_bi_legacy(tile, bel, "SLOW_RD_CLK", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "SLOW_WR_CLK", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "SYNCHRONOUS_MODE", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "OUTPUT_DISABLE", "FALSE", "TRUE");
        ctx.collect_bitvec_legacy(tile, bel, "SPARE", "");
        ctx.collect_enum_legacy(tile, bel, "MUX.RDCLK", &["INT", "PHASER"]);
        ctx.collect_enum_legacy(tile, bel, "MUX.RDEN", &["INT", "PHASER"]);
    }
    for i in 0..4 {
        let tile = "CMT";
        let bel = &format!("PHASER_IN[{i}]");
        ctx.collect_inv_legacy(tile, bel, "RST");
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
            ctx.collect_bit_bi_legacy(tile, bel, attr, "FALSE", "TRUE");
        }
        ctx.collect_enum_default_legacy_ocd(
            tile,
            bel,
            "CLKOUT_DIV",
            &[
                "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
            ],
            "NONE",
            OcdMode::BitOrderDrpV6,
        );
        ctx.collect_enum_legacy(tile, bel, "CTL_MODE", &["HARD", "SOFT"]);
        ctx.collect_enum_legacy(tile, bel, "FREQ_REF_DIV", &["NONE", "DIV2", "DIV4"]);
        ctx.collect_enum_legacy_ocd(
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
        ctx.collect_enum_legacy(
            tile,
            bel,
            "PD_REVERSE",
            &["1", "2", "3", "4", "5", "6", "7", "8"],
        );
        ctx.collect_enum_legacy(
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
            ctx.collect_bitvec_legacy(tile, bel, attr, "");
        }
        ctx.collect_enum_legacy_ocd(
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
        let bel = &format!("PHASER_OUT[{i}]");

        ctx.collect_inv_legacy(tile, bel, "RST");
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
            ctx.collect_bit_bi_legacy(tile, bel, attr, "FALSE", "TRUE");
        }
        ctx.collect_enum_default_legacy_ocd(
            tile,
            bel,
            "CLKOUT_DIV",
            &[
                "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
            ],
            "NONE",
            OcdMode::BitOrderDrpV6,
        );
        ctx.collect_enum_legacy(tile, bel, "CTL_MODE", &["HARD", "SOFT"]);
        ctx.collect_enum_legacy_ocd(
            tile,
            bel,
            "OUTPUT_CLK_SRC",
            &["PHASE_REF", "DELAYED_PHASE_REF", "DELAYED_REF", "FREQ_REF"],
            OcdMode::BitOrderDrpV6,
        );
        ctx.collect_enum_legacy(tile, bel, "STG1_BYPASS", &["PHASE_REF", "FREQ_REF"]);
        for attr in ["CLKOUT_DIV_ST", "COARSE_DELAY", "FINE_DELAY", "OCLK_DELAY"] {
            ctx.collect_bitvec_legacy(tile, bel, attr, "");
        }
        let diffs = ctx.get_diffs_legacy(tile, bel, "TEST_OPT", "");
        let diffs_po = ctx.get_diffs_legacy(tile, bel, "PO", "");
        assert_eq!(&diffs[6..9], &diffs_po[..]);
        ctx.insert_legacy(tile, bel, "TEST_OPT", xlat_bitvec_legacy(diffs));

        ctx.collect_enum_default_legacy_ocd(
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
        ctx.collect_inv_legacy(tile, bel, "RST");
        ctx.collect_inv_legacy(tile, bel, "PWRDWN");
        for attr in ["PHASER_REF_EN", "SEL_SLIPD", "SUP_SEL_AREG"] {
            ctx.collect_bit_bi_legacy(tile, bel, attr, "FALSE", "TRUE");
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
            ctx.collect_bitvec_legacy(tile, bel, attr, "");
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
            ctx.collect_bit_bi_legacy(tile, bel, attr, "FALSE", "TRUE");
        }
        ctx.collect_enum_legacy(tile, bel, "CLK_RATIO", &["1", "2", "4", "8"]);
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
            ctx.collect_bitvec_legacy(tile, bel, attr, "");
        }
    }
    for bel in ["PLL[0]", "PLL[1]"] {
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

        for i in 0..4 {
            ctx.collect_bit_legacy(tile, bel, &format!("BUF.CLKOUT{i}_FREQ_BB"), "1");
        }
        for pin in ["CLKFBIN", "CLKIN1", "CLKIN2"] {
            ctx.collect_enum_legacy_ocd(
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
            ctx.collect_enum_default_legacy_ocd(
                tile,
                bel,
                &format!("MUX.{pin}_HCLK"),
                &vals,
                "NONE",
                OcdMode::Mux,
            );
        }
        if bel == "PLL[0]" {
            for i in 0..4 {
                ctx.collect_enum_default_legacy_ocd(
                    tile,
                    bel,
                    &format!("MUX.PERF{i}"),
                    &["CLKOUT0", "CLKOUT1", "CLKOUT2", "CLKOUT3", "CLKFBOUT"],
                    "NONE",
                    OcdMode::BitOrderDrpV6,
                );
            }
        }

        ctx.get_diff_legacy(tile, bel, "MUX.CLKFBIN", "CLKFBOUT")
            .assert_empty();

        let item = ctx.extract_bit_legacy(tile, bel, "DRP_MASK", "1");
        assert_eq!(item.bits.len(), 1);
        assert_eq!(item.bits[0].rect.to_idx(), 50);
        let mut item_l = item.clone();
        let mut item_r = item;
        item_l.bits[0].rect = BitRectId::from_idx(0);
        item_r.bits[0].rect = BitRectId::from_idx(1);
        if bel == "PLL[1]" {
            ctx.insert_legacy("HCLK", "HCLK", "DRP_MASK_ABOVE_L", item_l);
            ctx.insert_legacy("HCLK", "HCLK", "DRP_MASK_ABOVE_R", item_r);
        } else {
            ctx.insert_legacy("HCLK", "HCLK", "DRP_MASK_BELOW_L", item_l);
            ctx.insert_legacy("HCLK", "HCLK", "DRP_MASK_BELOW_R", item_r);
        }
    }
    for i in 0..2 {
        let tile = "CMT";
        let bel = &format!("BUFMRCE[{i}]");
        ctx.collect_bit_legacy(tile, bel, "ENABLE", "1");
        ctx.collect_inv_legacy(tile, bel, "CE");
        ctx.collect_bit_bi_legacy(tile, bel, "INIT_OUT", "0", "1");
        ctx.collect_enum_legacy(tile, bel, "CE_TYPE", &["SYNC", "ASYNC"]);
        ctx.collect_enum_default_legacy_ocd(
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
            ctx.collect_enum_default_legacy(
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
            let mut diff = ctx.get_diff_legacy(tile, bel, format!("ENABLE.FREQ_BB{i}"), "1");
            let diff_bot = diff.split_bits_by(|bit| bit.rect.to_idx() < 24);
            let diff_top =
                diff.split_bits_by(|bit| bit.rect.to_idx() > 25 && bit.rect.to_idx() != 50);
            ctx.insert_legacy(
                tile,
                "CMT_BOT",
                format!("ENABLE.FREQ_BB{i}"),
                xlat_bit_wide_legacy(diff_bot),
            );
            ctx.insert_legacy(
                tile,
                "CMT_TOP",
                format!("ENABLE.FREQ_BB{i}"),
                xlat_bit_wide_legacy(diff_top),
            );
            ctx.insert_legacy(
                tile,
                "HCLK_CMT",
                format!("ENABLE.FREQ_BB{i}"),
                xlat_bit_wide_legacy(diff),
            );
        }
        if edev.chips.first().unwrap().regs > 1 {
            ctx.collect_bit_legacy(tile, bel, "ENABLE.SYNC_BB_S", "1");
            ctx.collect_bit_legacy(tile, bel, "BUF.SYNC_BB.U", "1");
            let mut diff = ctx.get_diff_legacy(tile, bel, "BUF.SYNC_BB.D", "1");
            diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "ENABLE.SYNC_BB_S"), true, false);
            ctx.insert_legacy(tile, bel, "BUF.SYNC_BB.D", xlat_bit_legacy(diff));
            for i in 0..4 {
                ctx.collect_bit_wide_legacy(tile, bel, &format!("ENABLE.FREQ_BB{i}_S"), "1");
                ctx.collect_bit_wide_legacy(tile, bel, &format!("BUF.FREQ_BB{i}.U"), "1");
                let mut diff = ctx.get_diff_legacy(tile, bel, format!("BUF.FREQ_BB{i}.D"), "1");
                diff.apply_bitvec_diff_int_legacy(
                    ctx.item_legacy(tile, bel, &format!("ENABLE.FREQ_BB{i}_S")),
                    3,
                    0,
                );
                ctx.insert_legacy(
                    tile,
                    bel,
                    format!("BUF.FREQ_BB{i}.D"),
                    xlat_bit_wide_legacy(diff),
                );
            }
        }
    }
    {
        let tile = "CMT";
        let bel = "CMT_TOP";
        ctx.collect_bit_legacy(tile, bel, "DRIVE.SYNC_BB", "1");
        for i in 0..4 {
            ctx.collect_enum_default_legacy(
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
            ctx.collect_enum_legacy(
                tile,
                bel,
                &format!("MUX.{pin}"),
                &["FREQ_BB0", "FREQ_BB1", "FREQ_BB2", "FREQ_BB3", pll_clkout],
            );
        }
        if edev.chips.first().unwrap().regs > 1 {
            ctx.collect_bit_legacy(tile, bel, "ENABLE.SYNC_BB_N", "1");
            ctx.collect_bit_legacy(tile, bel, "BUF.SYNC_BB.D", "1");
            let mut diff = ctx.get_diff_legacy(tile, bel, "BUF.SYNC_BB.U", "1");
            diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "ENABLE.SYNC_BB_N"), true, false);
            ctx.insert_legacy(tile, bel, "BUF.SYNC_BB.U", xlat_bit_legacy(diff));
            for i in 0..4 {
                ctx.collect_bit_wide_legacy(tile, bel, &format!("ENABLE.FREQ_BB{i}_N"), "1");
                ctx.collect_bit_wide_legacy(tile, bel, &format!("BUF.FREQ_BB{i}.D"), "1");
                let mut diff = ctx.get_diff_legacy(tile, bel, format!("BUF.FREQ_BB{i}.U"), "1");
                diff.apply_bitvec_diff_int_legacy(
                    ctx.item_legacy(tile, bel, &format!("ENABLE.FREQ_BB{i}_N")),
                    3,
                    0,
                );
                ctx.insert_legacy(
                    tile,
                    bel,
                    format!("BUF.FREQ_BB{i}.U"),
                    xlat_bit_wide_legacy(diff),
                );
            }
            let diff_bot = ctx.get_diff_legacy(tile, bel, "ENABLE.SYNC_BB", "BOT");
            let diff_top = ctx.get_diff_legacy(tile, bel, "ENABLE.SYNC_BB", "TOP");
            let (diff_bot, diff_top, diff_com) = Diff::split(diff_bot, diff_top);
            ctx.insert_legacy(tile, "CMT_BOT", "ENABLE.SYNC_BB", xlat_bit_legacy(diff_top));
            ctx.insert_legacy(
                tile,
                "CMT_TOP",
                "ENABLE.SYNC_BB",
                xlat_bit_wide_legacy(diff_bot.combine(&diff_com)),
            );
        }
    }
    {
        let tile = "CMT";
        let bel = "HCLK_CMT";
        ctx.collect_bit_legacy(tile, bel, "ENABLE.CKINT0", "1");
        ctx.collect_bit_legacy(tile, bel, "ENABLE.CKINT1", "1");
        ctx.collect_bit_legacy(tile, bel, "ENABLE.CKINT2", "1");
        ctx.collect_bit_legacy(tile, bel, "ENABLE.CKINT3", "1");
        for i in 0..12 {
            let diff = ctx
                .get_diff_legacy(tile, bel, "MUX.HOUT0", format!("HCLK{i}.EXCL"))
                .combine(&!ctx.peek_diff_legacy(tile, bel, "MUX.HOUT0", format!("HCLK{i}")));
            ctx.insert_legacy(tile, bel, format!("ENABLE.HCLK{i}"), xlat_bit_legacy(diff));
        }
        for i in 4..14 {
            let diff = ctx
                .get_diff_legacy(tile, bel, "MUX.HOUT0", format!("HIN{i}.EXCL"))
                .combine(&!ctx.peek_diff_legacy(tile, bel, "MUX.HOUT0", format!("HIN{i}")));
            ctx.insert_legacy(tile, bel, format!("ENABLE.HIN{i}"), xlat_bit_legacy(diff));
        }
        for i in 0..4 {
            let mut diffs_pref = vec![
                ("NONE".to_string(), Diff::default()),
                (
                    "CLKOUT".to_string(),
                    ctx.get_diff_legacy(tile, bel, format!("MUX.PHASER_REF_BOUNCE{i}"), "CLKOUT"),
                ),
                (
                    "TMUXOUT".to_string(),
                    ctx.get_diff_legacy(tile, bel, format!("MUX.PHASER_REF_BOUNCE{i}"), "TMUXOUT"),
                ),
            ];
            let diff_pref_ccio = ctx
                .get_diff_legacy(tile, bel, "MUX.HOUT0", format!("CCIO{i}.EXCL"))
                .combine(&!ctx.peek_diff_legacy(
                    tile,
                    bel,
                    "MUX.HOUT0",
                    format!("PHASER_REF_BOUNCE{i}"),
                ));
            let mut diffs_fbb = vec![
                ("NONE".to_string(), Diff::default()),
                (
                    format!("CKINT{i}"),
                    ctx.get_diff_legacy(tile, bel, format!("MUX.FREQ_BB{i}"), format!("CKINT{i}")),
                ),
            ];
            let diff_fbb_ccio =
                ctx.get_diff_legacy(tile, bel, format!("MUX.FREQ_BB{i}"), format!("CCIO{i}"));
            let (diff_pref_ccio, diff_fbb_ccio, diff_en_ccio) =
                Diff::split(diff_pref_ccio, diff_fbb_ccio);
            diffs_pref.push((format!("CCIO{i}"), diff_pref_ccio));
            diffs_fbb.push((format!("CCIO{i}"), diff_fbb_ccio));
            ctx.insert_legacy(
                tile,
                bel,
                format!("MUX.PHASER_REF_BOUNCE{i}"),
                xlat_enum_legacy(diffs_pref),
            );
            ctx.insert_legacy(
                tile,
                bel,
                format!("MUX.FREQ_BB{i}"),
                xlat_enum_legacy(diffs_fbb),
            );
            ctx.insert_legacy(
                tile,
                bel,
                format!("ENABLE.CCIO{i}"),
                xlat_bit_legacy(diff_en_ccio),
            );
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
            ctx.collect_enum_default_legacy_ocd(
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
                ctx.collect_enum_default_legacy_ocd(
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
                .peek_diff_legacy(tile, bel, format!("MUX.PERF{i}"), format!("MMCM_PERF{i}"))
                .clone();
            let diff_b = ctx
                .peek_diff_legacy(tile, bel, format!("MUX.PERF{oi}"), format!("MMCM_PERF{i}"))
                .clone();
            let (_, _, diff) = Diff::split(diff_a, diff_b);
            ctx.insert_legacy(
                tile,
                "PLL[0]",
                format!("ENABLE.PERF{i}"),
                xlat_bit_legacy(diff),
            );
            let diff_a = ctx
                .peek_diff_legacy(
                    tile,
                    bel,
                    format!("MUX.PERF{i}"),
                    format!("PHASER_IN_RCLK{i}"),
                )
                .clone();
            let diff_b = ctx
                .peek_diff_legacy(
                    tile,
                    bel,
                    format!("MUX.PERF{oi}"),
                    format!("PHASER_IN_RCLK{i}"),
                )
                .clone();
            let (_, _, diff) = Diff::split(diff_a, diff_b);
            ctx.insert_legacy(
                tile,
                format!("PHASER_IN[{i}]"),
                "ENABLE.RCLK",
                xlat_bit_legacy(diff),
            );
        }
        for i in 0..4 {
            let mut diffs = vec![("NONE".to_string(), Diff::default())];
            for j in 0..4 {
                let mut diff = ctx.get_diff_legacy(
                    tile,
                    bel,
                    format!("MUX.PERF{i}"),
                    format!("PHASER_IN_RCLK{j}"),
                );
                diff.apply_bit_diff_legacy(
                    ctx.item_legacy(tile, &format!("PHASER_IN[{j}]"), "ENABLE.RCLK"),
                    true,
                    false,
                );
                diffs.push((format!("PHASER_IN_RCLK{j}"), diff));
            }
            for j in [i, i ^ 1] {
                let mut diff =
                    ctx.get_diff_legacy(tile, bel, format!("MUX.PERF{i}"), format!("MMCM_PERF{j}"));
                diff.apply_bit_diff_legacy(
                    ctx.item_legacy(tile, "PLL[0]", &format!("ENABLE.PERF{j}")),
                    true,
                    false,
                );
                diffs.push((format!("MMCM_PERF{j}"), diff));
            }
            ctx.insert_legacy(
                tile,
                bel,
                format!("MUX.PERF{i}"),
                xlat_enum_legacy_ocd(diffs, OcdMode::Mux),
            );
        }
    }
    for tile in ["HCLK_IO_HR", "HCLK_IO_HP"] {
        if ctx.has_tile_legacy(tile) {
            let bel = "HCLK_IO";
            ctx.collect_bit_legacy(tile, bel, "ENABLE.PERF0", "1");
            ctx.collect_bit_legacy(tile, bel, "ENABLE.PERF1", "1");
            ctx.collect_bit_legacy(tile, bel, "ENABLE.PERF2", "1");
            ctx.collect_bit_legacy(tile, bel, "ENABLE.PERF3", "1");
        }
    }
}
