use bitvec::vec::BitVec;
use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_types::TileItem;
use unnamed_entity::EntityId;

use crate::{
    backend::{FeatureBit, IseBackend},
    diff::{extract_bitvec_val_part, xlat_bitvec, xlat_enum_ocd, CollectorCtx, OcdMode},
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits, TileRelation},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_multi_attr_bin, fuzz_multi_attr_dec, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ctx = FuzzCtx::new(session, backend, "CMT_PLL", "PLL", TileBits::Cmt);
    let extras = vec![ExtraFeature::new(
        ExtraFeatureKind::AllOtherDcms,
        "CMT_DCM",
        "CMT",
        "PRESENT_ANY_PLL",
        "1",
    )];
    fuzz_one_extras!(ctx, "PRESENT", "PLL", [
        (global_mutex "CMT", "PRESENT_PLL"),
        (global_mutex_site "CMT_PRESENT")
    ], [
        (mode "PLL_ADV"),
        (global_xy "PLLADV_*_USE_CALC", "NO")
    ], extras.clone());
    for pin in [
        "CLKBRST",
        "CLKINSEL",
        "ENOUTSYNC",
        "MANPDLF",
        "MANPULF",
        "RST",
        "SKEWCLKIN1",
        "SKEWCLKIN2",
        "SKEWRST",
        "SKEWSTB",
    ] {
        fuzz_inv!(ctx, pin, [
            (global_mutex "CMT", "INV"),
            (global_xy "PLLADV_*_USE_CALC", "NO"),
            (mode "PLL_ADV")
        ]);
    }
    let obel_tie = BelId::from_idx(1);
    fuzz_inv!(ctx, "REL", [
        (global_mutex "CMT", "INV"),
        (global_xy "PLLADV_*_USE_CALC", "NO"),
        (mode "PLL_ADV"),
        (pip (bel_pin obel_tie, "HARD1"), (pin "REL"))
    ]);

    for attr in [
        "PLL_EN",
        "PLL_EN_DLY",
        "PLL_NBTI_EN",
        "PLL_MAN_LF_EN",
        "PLL_CLAMP_BYPASS",
        "PLL_DIRECT_PATH_CNTRL",
        "PLL_VLFHIGH_DIS",
        "PLL_PWRD_CFG",
        "PLL_TEST_IN_WINDOW",
        "PLL_REG_INPUT",
        "PLL_CLK_LOST_DETECT",
        "PLL_SEL_SLIPD",
        "PLL_CP_BIAS_TRIP_SHIFT",
        "PLL_CLKBURST_ENABLE",
        "PLL_EN_TCLK0",
        "PLL_EN_TCLK1",
        "PLL_EN_TCLK2",
        "PLL_EN_TCLK3",
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
        "PLL_DIVCLK_EDGE",
        "PLL_CLKOUT0_EDGE",
        "PLL_CLKOUT1_EDGE",
        "PLL_CLKOUT2_EDGE",
        "PLL_CLKOUT3_EDGE",
        "PLL_CLKOUT4_EDGE",
        "PLL_CLKOUT5_EDGE",
        "PLL_CLKFBOUT_EDGE",
        "PLL_CLKFBOUT2_EDGE",
        "PLL_DIVCLK_NOCOUNT",
        "PLL_CLKOUT0_NOCOUNT",
        "PLL_CLKOUT1_NOCOUNT",
        "PLL_CLKOUT2_NOCOUNT",
        "PLL_CLKOUT3_NOCOUNT",
        "PLL_CLKOUT4_NOCOUNT",
        "PLL_CLKOUT5_NOCOUNT",
        "PLL_CLKFBOUT_NOCOUNT",
        "PLL_CLKFBOUT2_NOCOUNT",
    ] {
        fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [
            (global_mutex "CMT", "TEST"),
            (global_xy "PLLADV_*_USE_CALC", "NO"),
            (mode "PLL_ADV")
        ]);
    }

    for (attr, width) in [
        ("PLL_CLKCNTRL", 1),
        ("PLL_PFD_DLY", 2),
        ("PLL_CLK0MX", 2),
        ("PLL_CLK1MX", 2),
        ("PLL_CLK2MX", 2),
        ("PLL_CLK3MX", 2),
        ("PLL_CLK4MX", 2),
        ("PLL_CLK5MX", 2),
        ("PLL_CLKFBMX", 2),
        ("PLL_LFHF", 2),
        ("PLL_CP_RES", 2),
        ("PLL_EN_LEAKAGE", 2),
        ("PLL_IO_CLKSRC", 2),
        ("PLL_ADD_LEAKAGE", 2),
        ("PLL_VDD_SEL", 2),
        ("PLL_SKEW_CNTRL", 2),
        ("PLL_AVDD_COMP_SET", 2),
        ("PLL_DVDD_COMP_SET", 2),
        ("PLL_INTFB", 2),
        ("PLL_CLKBURST_CNT", 3),
        ("PLL_CLAMP_REF_SEL", 3),
        ("PLL_PFD_CNTRL", 4),
        ("PLL_CP", 4),
        ("PLL_RES", 4),
        ("PLL_LOCK_REF_DLY", 5),
        ("PLL_LOCK_FB_DLY", 5),
        ("PLL_LOCK_CNT", 10),
        ("PLL_LOCK_SAT_HIGH", 10),
        ("PLL_UNLOCK_CNT", 10),
    ] {
        fuzz_multi_attr_dec!(ctx, attr, width, [
            (global_mutex "CMT", "TEST"),
            (global_xy "PLLADV_*_USE_CALC", "NO"),
            (mode "PLL_ADV")
        ]);
    }
    fuzz_multi_attr_dec!(ctx, "PLL_CP_REPL", 4, [
        (global_mutex "CMT", "CP_REPL"),
        (global_xy "PLLADV_*_USE_CALC", "YES"),
        (mode "PLL_ADV")
    ]);
    for (attr, width) in [
        ("PLL_EN_CNTRL", 85),
        ("PLL_MISC", 4),
        ("PLL_IN_DLY_MX_SEL", 5),
        ("PLL_IN_DLY_SET", 9),
        ("PLL_OPT_INV", 6),
    ] {
        fuzz_multi_attr_bin!(ctx, attr, width, [
            (global_mutex "CMT", "TEST"),
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
                (global_mutex "CMT", "TEST"),
                (global_xy "PLLADV_*_USE_CALC", "NO"),
                (mode "PLL_ADV")
            ]);
        }
    }
    for out in [
        "CLKOUT0", "CLKOUT1", "CLKOUT2", "CLKOUT3", "CLKOUT4", "CLKOUT5", "CLKFBOUT",
    ] {
        fuzz_multi_attr_bin!(ctx, format!("PLL_{out}_PM"), 3, [
            (global_mutex "CMT", "TEST"),
            (global_xy "PLLADV_*_USE_CALC", "NO"),
            (mode "PLL_ADV")
        ]);
    }
    fuzz_enum!(ctx, "COMPENSATION", ["SOURCE_SYNCHRONOUS", "SYSTEM_SYNCHRONOUS", "PLL2DCM", "DCM2PLL", "EXTERNAL", "INTERNAL"], [
        (global_mutex "CMT", "CALC"),
        (global_xy "PLLADV_*_USE_CALC", "NO"),
        (mode "PLL_ADV")
    ]);

    for mult in 1..=64 {
        for bandwidth in ["LOW", "HIGH"] {
            fuzz_one!(ctx, "TABLES", format!("{mult}.{bandwidth}"), [
                (global_mutex "CMT", "CALC"),
                (global_xy "PLLADV_*_USE_CALC", "NO"),
                (mode "PLL_ADV")
            ], [
                (attr_diff "CLKFBOUT_MULT", "0", format!("{mult}")),
                (attr_diff "BANDWIDTH", "LOW", bandwidth)
            ]);
        }
    }

    let obel_dcm0 = BelId::from_idx(0);
    let obel_dcm1 = BelId::from_idx(1);
    let obel_dcm_cmt = BelId::from_idx(2);
    let bel_cmt = BelId::from_idx(2);
    let relation_dcm = TileRelation::Delta(0, -16, backend.egrid.db.get_node("CMT_DCM"));

    for (opin, val, ipin) in [
        ("CLKIN1", "CKINT0", "CLKIN1_CKINT0"),
        ("CLKIN2", "CKINT0", "CLKIN2_CKINT0"),
        ("CLKIN2", "CKINT1", "CLKIN2_CKINT1"),
        ("CLKIN2", "CLK_FROM_DCM0", "CLK_FROM_DCM0"),
        ("CLKIN2", "CLK_FROM_DCM1", "CLK_FROM_DCM1"),
    ] {
        fuzz_one!(ctx, format!("MUX.{opin}"), val, [
            (mode "PLL_ADV"),
            (global_xy "PLLADV_*_USE_CALC", "NO"),
            (global_mutex "CMT", format!("TEST_{opin}")),
            (mutex "CLKIN_IN", ipin),
            (pip (pin "CLKFBIN_CKINT0"), (pin_far "CLKFBIN"))
        ], [
            (pip (pin ipin), (pin_far opin))
        ]);
    }

    for btlr in ["BT", "LR"] {
        for j in 0..8 {
            let opin = if j < 4 { "CLKIN1" } else { "CLKIN2" };
            fuzz_one!(ctx, format!("MUX.{opin}"), format!("BUFIO2_{btlr}{j}"), [
                (mode "PLL_ADV"),
                (global_xy "PLLADV_*_USE_CALC", "NO"),
                (global_mutex "CMT", format!("TEST_{opin}")),
                (mutex "CLKIN_IN", format!("BUFIO2_{btlr}{j}")),
                (pip (pin "CLKFBIN_CKINT0"), (pin_far "CLKFBIN")),
                (global_mutex "BUFIO2_CMT_OUT", "USE"),
                (related relation_dcm,
                    (pip (bel_pin obel_dcm_cmt, format!("BUFIO2_{btlr}{j}")),
                        (bel_pin obel_dcm0, "CLKIN")))
            ], [
                (pip (bel_pin bel_cmt, format!("BUFIO2_{btlr}{j}")), (pin_far opin))
            ]);
        }
    }

    fuzz_one!(ctx, "CLKINSEL_MODE", "DYNAMIC", [
        (mode "PLL_ADV"),
        (global_xy "PLLADV_*_USE_CALC", "NO"),
        (global_mutex "CMT", "TEST_CLKIN1_BOTH"),
        (mutex "CLKIN_IN", "CLKIN1_CKINT0"),
        (pip (pin "CLKFBIN_CKINT0"), (pin_far "CLKFBIN")),
        (pip (pin "CLKIN2_CKINT0"), (pin_far "CLKIN2"))
    ], [
        (pip (pin "CLKIN1_CKINT0"), (pin_far "CLKIN1"))
    ]);

    for (val, pin) in [
        ("CKINT0", "CLKFBIN_CKINT0"),
        ("CKINT1", "CLKFBIN_CKINT1"),
        ("CLKOUT0", "CLKOUT0"),
        ("CLKFBDCM", "CLKFBDCM"),
    ] {
        fuzz_one!(ctx, "MUX.CLKFBIN", val, [
            (mode "PLL_ADV"),
            (global_xy "PLLADV_*_USE_CALC", "NO"),
            (global_mutex "CMT", "TEST_CLKFBIN"),
            (mutex "CLKIN_IN", pin),
            (pip (pin "CLKIN1_CKINT0"), (pin_far "CLKIN1"))
        ], [
            (pip (pin pin), (pin_far "CLKFBIN"))
        ]);
    }
    fuzz_one!(ctx, "MUX.CLKFBIN", "CLKFBOUT", [
        (mode "PLL_ADV"),
        (global_mutex "CMT", "TEST_CLKFBIN"),
        (mutex "CLKIN_IN", "CLKFBOUT"),
        (pip (pin "CLKIN1_CKINT0"), (pin_far "CLKIN1"))
    ], [
        (pip (pin_far "CLKFBOUT"), (pin_far "CLKFBIN"))
    ]);
    for btlr in ["BT", "LR"] {
        for j in 0..8 {
            fuzz_one!(ctx, format!("MUX.CLKFBIN"), format!("BUFIO2FB_{btlr}{j}"), [
                (mode "PLL_ADV"),
                (global_mutex "CMT", "TEST_CLKFBIN"),
                (mutex "CLKIN_IN", format!("BUFIO2FB_{btlr}{j}")),
                (pip (pin "CLKIN1_CKINT0"), (pin_far "CLKIN1")),
                (global_mutex "BUFIO2_CMT_OUT", "USE"),
                (related relation_dcm,
                    (pip (bel_pin obel_dcm_cmt, format!("BUFIO2FB_{btlr}{j}")),
                        (bel_pin obel_dcm0, "CLKFB")))
            ], [
                (pip (bel_pin bel_cmt, format!("BUFIO2FB_{btlr}{j}")), (pin_far "CLKFBIN"))
            ]);
        }
    }

    for i in 0..2 {
        for (inp, ipin) in [
            ("CLKOUT0", "CLKOUTDCM0"),
            ("CLKOUT1", "CLKOUTDCM1"),
            ("CLKOUT2", "CLKOUTDCM2"),
            ("CLKOUT3", "CLKOUTDCM3"),
            ("CLKOUT4", "CLKOUTDCM4"),
            ("CLKOUT5", "CLKOUTDCM5"),
            ("CLKFBOUT", "CLKFBDCM_TEST"),
        ] {
            if i == 0 && inp == "CLKFBOUT" {
                continue;
            }
            let extras = vec![ExtraFeature::new(
                ExtraFeatureKind::PllDcm,
                "CMT_DCM",
                format!("DCM{i}"),
                "BUF.CLK_FROM_PLL",
                "1",
            )];
            fuzz_one_extras!(ctx, format!("MUX.CLK_TO_DCM{i}"), inp, [
                (global_mutex "CMT", "MUX_PLL"),
                (mutex "CLK_TO_DCM_IN", inp),
                (mode "PLL_ADV")
            ], [
                (pip (pin ipin), (pin format!("CLK_TO_DCM{i}")))
            ], extras);
        }
    }

    for inp in [
        "CLKIN1",
        "CLKFBIN",
        "DCM0_CLKIN",
        "DCM1_CLKIN",
        "DCM0_CLKFB",
        "DCM1_CLKFB",
    ] {
        fuzz_one!(ctx, "MUX.TEST_CLK", inp, [
            (global_mutex "CMT", "MUX_PLL"),
            (mutex "TEST_CLK", inp),
            (related relation_dcm,
                (pip (bel_pin obel_dcm0, "CLKIN_CKINT0"), (bel_pin obel_dcm0, "CLKIN"))),
            (related relation_dcm,
                (pip (bel_pin obel_dcm1, "CLKIN_CKINT0"), (bel_pin obel_dcm1, "CLKIN"))),
            (related relation_dcm,
                (pip (bel_pin obel_dcm0, "CLKFB_CKINT0"), (bel_pin obel_dcm0, "CLKFB"))),
            (related relation_dcm,
                (pip (bel_pin obel_dcm1, "CLKFB_CKINT0"), (bel_pin obel_dcm1, "CLKFB"))),
            (related relation_dcm, (tile_mutex "CLKIN_BEL", "PLL"))
        ], [
            (pip (pin format!("{inp}_TEST")), (pin "TEST_CLK"))
        ]);
    }

    let ctx = FuzzCtx::new(session, backend, "CMT_PLL", "CMT", TileBits::Cmt);
    for i in 0..16 {
        fuzz_one!(ctx, format!("MUX.CASC{i}"), "PASS", [
            (mutex format!("MUX.CASC{i}"), "PASS")
        ], [
            (pip (pin format!("CASC{i}_I")), (pin format!("CASC{i}_O")))
        ]);
        fuzz_one!(ctx, format!("MUX.CASC{i}"), "HCLK", [
            (mutex format!("MUX.CASC{i}"), "HCLK")
        ], [
            (pip (pin format!("HCLK{i}_BUF")), (pin format!("CASC{i}_O")))
        ]);
        fuzz_one!(ctx, format!("MUX.HCLK{i}"), "CKINT", [
            (mutex format!("MUX.HCLK{i}"), "CKINT")
        ], [
            (pip (pin format!("HCLK{i}_CKINT")), (pin format!("HCLK{i}")))
        ]);
        let bel_pll = BelId::from_idx(0);
        for out in [
            "CLKOUT0",
            "CLKOUT1",
            "CLKOUT2",
            "CLKOUT3",
            "CLKOUT4",
            "CLKOUT5",
            "TEST_CLK_OUT",
        ] {
            fuzz_one!(ctx, format!("MUX.HCLK{i}"), format!("PLL_{out}"), [
                (global_mutex "CMT", "MUX_PLL_HCLK"),
                (mutex format!("MUX.HCLK{i}"), format!("PLL_{out}"))
            ], [
                (pip (bel_pin bel_pll, out), (pin format!("HCLK{i}")))
            ]);
        }
        fuzz_one!(ctx, format!("MUX.HCLK{i}"), "PLL_CLKFBOUT", [
            (global_mutex "CMT", "MUX_PLL_HCLK"),
            (mutex format!("MUX.HCLK{i}"), "PLL_CLKFBOUT")
        ], [
            (pip (bel_pin_far bel_pll, "CLKFBOUT"), (pin format!("HCLK{i}")))
        ]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, skip_dcm: bool) {
    let tile = "CMT_PLL";
    let bel = "PLL";

    fn reg_bit(addr: usize, bit: usize) -> FeatureBit {
        let slot = match addr {
            0..6 => 22 + addr,
            6..0x1c => 36 + (addr - 6),
            0x1c.. => 59 + (addr - 0x1c),
        };
        FeatureBit {
            tile: slot / 4,
            frame: 30,
            bit: (slot % 4) * 16 + bit,
        }
    }

    for addr in 0..0x20 {
        ctx.tiledb.insert(
            tile,
            bel,
            format!("DRP{addr:02X}"),
            TileItem::from_bitvec(Vec::from_iter((0..16).map(|bit| reg_bit(addr, bit))), false),
        );
    }

    let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "PLL");

    for pin in [
        "CLKBRST",
        "CLKINSEL",
        "ENOUTSYNC",
        "MANPDLF",
        "MANPULF",
        "RST",
        "SKEWRST",
        "SKEWSTB",
        "REL",
    ] {
        ctx.collect_inv(tile, bel, pin);
    }

    // hm.
    for pin in ["SKEWCLKIN1", "SKEWCLKIN2"] {
        ctx.state
            .get_diff(tile, bel, format!("{pin}INV"), pin)
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, format!("{pin}INV"), format!("{pin}_B"))
            .assert_empty();
    }

    for attr in [
        "PLL_EN_DLY",
        "PLL_NBTI_EN",
        "PLL_MAN_LF_EN",
        "PLL_CLAMP_BYPASS",
        "PLL_DIRECT_PATH_CNTRL",
        "PLL_VLFHIGH_DIS",
        "PLL_PWRD_CFG",
        "PLL_TEST_IN_WINDOW",
        "PLL_REG_INPUT",
        "PLL_CLK_LOST_DETECT",
        "PLL_SEL_SLIPD",
        "PLL_CP_BIAS_TRIP_SHIFT",
        "PLL_CLKBURST_ENABLE",
        "PLL_EN_TCLK0",
        "PLL_EN_TCLK1",
        "PLL_EN_TCLK2",
        "PLL_EN_TCLK3",
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
        "PLL_DIVCLK_EDGE",
        "PLL_CLKOUT0_EDGE",
        "PLL_CLKOUT1_EDGE",
        "PLL_CLKOUT2_EDGE",
        "PLL_CLKOUT3_EDGE",
        "PLL_CLKOUT4_EDGE",
        "PLL_CLKOUT5_EDGE",
        "PLL_CLKFBOUT_EDGE",
        "PLL_CLKFBOUT2_EDGE",
        "PLL_DIVCLK_NOCOUNT",
        "PLL_CLKOUT0_NOCOUNT",
        "PLL_CLKOUT1_NOCOUNT",
        "PLL_CLKOUT2_NOCOUNT",
        "PLL_CLKOUT3_NOCOUNT",
        "PLL_CLKOUT4_NOCOUNT",
        "PLL_CLKOUT5_NOCOUNT",
        "PLL_CLKFBOUT_NOCOUNT",
        "PLL_CLKFBOUT2_NOCOUNT",
    ] {
        ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
    }

    for attr in [
        "PLL_PFD_DLY",
        "PLL_CLK0MX",
        "PLL_CLK1MX",
        "PLL_CLK2MX",
        "PLL_CLK3MX",
        "PLL_CLK4MX",
        "PLL_CLK5MX",
        "PLL_CLKFBMX",
        "PLL_LFHF",
        "PLL_CP_RES",
        "PLL_EN_LEAKAGE",
        "PLL_ADD_LEAKAGE",
        "PLL_VDD_SEL",
        "PLL_AVDD_COMP_SET",
        "PLL_DVDD_COMP_SET",
        "PLL_INTFB",
        "PLL_CLKBURST_CNT",
        "PLL_CLAMP_REF_SEL",
        "PLL_PFD_CNTRL",
        "PLL_RES",
        "PLL_LOCK_REF_DLY",
        "PLL_LOCK_FB_DLY",
        "PLL_LOCK_CNT",
        "PLL_LOCK_SAT_HIGH",
        "PLL_UNLOCK_CNT",
        "PLL_EN_CNTRL",
        "PLL_IN_DLY_MX_SEL",
        "PLL_IN_DLY_SET",
        "PLL_CP_REPL",
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
        "PLL_DIVCLK_HT",
        "PLL_DIVCLK_LT",
    ] {
        ctx.collect_bitvec(tile, bel, attr, "");
    }

    for attr in ["PLL_MISC", "PLL_CP", "PLL_DIVCLK_DT"] {
        for diff in ctx.state.get_diffs(tile, bel, attr, "") {
            diff.assert_empty();
        }
    }

    // sigh. bug. again. murder me with a rusty spoon.
    ctx.tiledb.insert(
        tile,
        bel,
        "PLL_CP",
        TileItem::from_bitvec(
            vec![
                reg_bit(0x18, 13),
                reg_bit(0x18, 10),
                reg_bit(0x18, 11),
                reg_bit(0x18, 9),
            ],
            false,
        ),
    );
    ctx.state
        .get_diff(tile, bel, "PLL_EN", "FALSE")
        .assert_empty();
    ctx.state
        .get_diff(tile, bel, "PLL_EN", "TRUE")
        .assert_empty();

    ctx.tiledb.insert(
        tile,
        bel,
        "PLL_EN",
        TileItem::from_bit(reg_bit(0x1a, 8), false),
    );

    ctx.tiledb.insert(
        tile,
        bel,
        "PLL_DIVCLK_EN",
        TileItem::from_bit(reg_bit(0x16, 0), false),
    );

    ctx.collect_enum_ocd(
        tile,
        bel,
        "MUX.TEST_CLK",
        &[
            "DCM1_CLKIN",
            "DCM1_CLKFB",
            "DCM0_CLKIN",
            "DCM0_CLKFB",
            "CLKIN1",
            "CLKFBIN",
        ],
        OcdMode::Mux,
    );

    ctx.collect_enum_default_ocd(
        tile,
        bel,
        "MUX.CLK_TO_DCM0",
        &[
            "CLKOUT0", "CLKOUT1", "CLKOUT2", "CLKOUT3", "CLKOUT4", "CLKOUT5",
        ],
        "NONE",
        OcdMode::BitOrder,
    );
    ctx.collect_enum_default_ocd(
        tile,
        bel,
        "MUX.CLK_TO_DCM1",
        &[
            "CLKOUT0", "CLKOUT1", "CLKOUT2", "CLKOUT3", "CLKOUT4", "CLKOUT5", "CLKFBOUT",
        ],
        "NONE",
        OcdMode::BitOrder,
    );

    ctx.collect_enum_ocd(
        tile,
        bel,
        "MUX.CLKFBIN",
        &[
            "BUFIO2FB_BT0",
            "BUFIO2FB_BT1",
            "BUFIO2FB_BT2",
            "BUFIO2FB_BT3",
            "BUFIO2FB_BT4",
            "BUFIO2FB_BT5",
            "BUFIO2FB_BT6",
            "BUFIO2FB_BT7",
            "BUFIO2FB_LR0",
            "BUFIO2FB_LR1",
            "BUFIO2FB_LR2",
            "BUFIO2FB_LR3",
            "BUFIO2FB_LR4",
            "BUFIO2FB_LR5",
            "BUFIO2FB_LR6",
            "BUFIO2FB_LR7",
            "CLKFBOUT",
            "CKINT0",
            "CKINT1",
            "CLKFBDCM",
        ],
        OcdMode::Mux,
    );

    ctx.collect_enum_default(tile, bel, "CLKINSEL_MODE", &["DYNAMIC"], "STATIC");

    // ????
    ctx.state
        .get_diff(tile, bel, "MUX.CLKFBIN", "CLKOUT0")
        .assert_empty();
    let mut diffs = vec![];
    for (val, val1, val2) in [
        ("BUFIO2_BT0_4", "BUFIO2_BT0", "BUFIO2_BT4"),
        ("BUFIO2_BT1_5", "BUFIO2_BT1", "BUFIO2_BT5"),
        ("BUFIO2_BT2_6", "BUFIO2_BT2", "BUFIO2_BT6"),
        ("BUFIO2_BT3_7", "BUFIO2_BT3", "BUFIO2_BT7"),
        ("BUFIO2_LR0_4", "BUFIO2_LR0", "BUFIO2_LR4"),
        ("BUFIO2_LR1_5", "BUFIO2_LR1", "BUFIO2_LR5"),
        ("BUFIO2_LR2_6", "BUFIO2_LR2", "BUFIO2_LR6"),
        ("BUFIO2_LR3_7", "BUFIO2_LR3", "BUFIO2_LR7"),
        ("CKINT0", "CKINT0", "CKINT0"),
    ] {
        let diff1 = ctx.state.get_diff(tile, bel, "MUX.CLKIN1", val1);
        let diff2 = ctx.state.get_diff(tile, bel, "MUX.CLKIN2", val2);
        let diff2 = diff2.combine(&!&diff1);
        diffs.push((val, diff1));
        ctx.tiledb
            .insert(tile, bel, "CLKINSEL_STATIC", xlat_bitvec(vec![diff2]));
    }
    for val in ["CKINT1", "CLK_FROM_DCM0", "CLK_FROM_DCM1"] {
        let mut diff = ctx.state.get_diff(tile, bel, "MUX.CLKIN2", val);
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "CLKINSEL_STATIC"), true, false);
        diffs.push((val, diff));
    }
    ctx.tiledb
        .insert(tile, bel, "MUX.CLKIN", xlat_enum_ocd(diffs, OcdMode::Mux));

    let diff = ctx.state.get_diff(tile, bel, "PLL_CLKCNTRL", "");
    ctx.tiledb
        .insert(tile, bel, "CLKIN_CLKFBIN_USED", xlat_bitvec(vec![!diff]));

    let mut diffs = ctx.state.get_diffs(tile, bel, "PLL_IO_CLKSRC", "");
    diffs[0].apply_enum_diff(
        ctx.tiledb.item(tile, bel, "CLKINSEL_MODE"),
        "DYNAMIC",
        "STATIC",
    );
    diffs[1].apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.CLKINSEL"), false, true);
    for diff in diffs {
        diff.assert_empty();
    }

    let mut diffs = ctx.state.get_diffs(tile, bel, "PLL_OPT_INV", "");
    diffs[0].apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.RST"), true, false);
    diffs[1].apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.MANPDLF"), true, false);
    diffs[2].apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.MANPULF"), true, false);
    diffs[3].apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.REL"), false, true);
    diffs[4].apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.CLKBRST"), true, false);
    diffs[5].apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.ENOUTSYNC"), true, false);
    for diff in diffs {
        diff.assert_empty();
    }

    let mut diffs = ctx.state.get_diffs(tile, bel, "PLL_SKEW_CNTRL", "");
    diffs[0].apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.SKEWSTB"), true, false);
    diffs[1].apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.SKEWRST"), true, false);
    for diff in diffs {
        diff.assert_empty();
    }

    // um?
    present.apply_enum_diff(
        ctx.tiledb.item(tile, bel, "MUX.CLKFBIN"),
        "BUFIO2FB_LR7",
        "BUFIO2FB_BT0",
    );
    present.apply_enum_diff(
        ctx.tiledb.item(tile, bel, "MUX.CLKIN"),
        "BUFIO2_LR3_7",
        "BUFIO2_BT0_4",
    );
    present.apply_bit_diff(ctx.tiledb.item(tile, bel, "CLKINSEL_STATIC"), true, false);
    present.apply_enum_diff(
        ctx.tiledb.item(tile, bel, "MUX.CLK_TO_DCM0"),
        "NONE",
        "CLKOUT0",
    );
    present.apply_enum_diff(
        ctx.tiledb.item(tile, bel, "MUX.CLK_TO_DCM1"),
        "NONE",
        "CLKOUT0",
    );
    present.apply_bit_diff(
        ctx.tiledb.item(tile, bel, "PLL_CLKOUT0_NOCOUNT"),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.tiledb.item(tile, bel, "PLL_CLKOUT1_NOCOUNT"),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.tiledb.item(tile, bel, "PLL_CLKOUT2_NOCOUNT"),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.tiledb.item(tile, bel, "PLL_CLKOUT3_NOCOUNT"),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.tiledb.item(tile, bel, "PLL_CLKOUT4_NOCOUNT"),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.tiledb.item(tile, bel, "PLL_CLKOUT5_NOCOUNT"),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.tiledb.item(tile, bel, "PLL_CLKFBOUT_NOCOUNT"),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.tiledb.item(tile, bel, "PLL_DIVCLK_NOCOUNT"),
        true,
        false,
    );
    present.apply_bit_diff(ctx.tiledb.item(tile, bel, "PLL_EN_DLY"), false, true);
    present.apply_bit_diff(ctx.tiledb.item(tile, bel, "PLL_EN"), true, false);
    present.apply_bit_diff(ctx.tiledb.item(tile, bel, "PLL_DIVCLK_EN"), true, false);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_DIVCLK_LT"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_DIVCLK_HT"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT0_LT"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT0_HT"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT1_LT"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT1_HT"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT2_LT"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT2_HT"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT3_LT"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT3_HT"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT4_LT"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT4_HT"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT5_LT"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKOUT5_HT"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKFBOUT_LT"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CLKFBOUT_HT"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_IN_DLY_SET"), 0x11, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_IN_DLY_MX_SEL"), 0xa, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_LOCK_CNT"), 0x3e8, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_UNLOCK_CNT"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_LOCK_SAT_HIGH"), 0x3e9, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_LOCK_REF_DLY"), 0x9, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_LOCK_FB_DLY"), 0x7, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_LFHF"), 3, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_RES"), 11, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CP"), 2, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_CP_REPL"), 2, 0);

    ctx.tiledb
        .insert(tile, bel, "ENABLE", xlat_bitvec(vec![present]));

    ctx.state
        .get_diff(tile, bel, "COMPENSATION", "SYSTEM_SYNCHRONOUS")
        .assert_empty();
    let mut diff = ctx
        .state
        .get_diff(tile, bel, "COMPENSATION", "SOURCE_SYNCHRONOUS");
    diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_IN_DLY_SET"), 0, 0x11);
    diff.assert_empty();
    let mut diff = ctx.state.get_diff(tile, bel, "COMPENSATION", "EXTERNAL");
    diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_IN_DLY_SET"), 0, 0x11);
    diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_IN_DLY_MX_SEL"), 0, 0xa);
    diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "PLL_EN_DLY"), true, false);
    diff.assert_empty();
    let mut diff = ctx.state.get_diff(tile, bel, "COMPENSATION", "INTERNAL");
    diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_INTFB"), 2, 0);
    diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_IN_DLY_SET"), 0, 0x11);
    diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_IN_DLY_MX_SEL"), 0, 0xa);
    diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "PLL_EN_DLY"), true, false);
    diff.assert_empty();
    let mut diff = ctx.state.get_diff(tile, bel, "COMPENSATION", "DCM2PLL");
    diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_IN_DLY_SET"), 0, 0x11);
    diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_IN_DLY_MX_SEL"), 0, 0xa);
    diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "PLL_EN_DLY"), true, false);
    diff.assert_empty();
    let mut diff = ctx.state.get_diff(tile, bel, "COMPENSATION", "PLL2DCM");
    diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_IN_DLY_SET"), 0, 0x11);
    diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "PLL_IN_DLY_MX_SEL"), 0, 0xa);
    diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "PLL_EN_DLY"), true, false);
    diff.assert_empty();

    for mult in 1..=64 {
        for bandwidth in ["LOW", "HIGH"] {
            let mut diff = ctx
                .state
                .get_diff(tile, bel, "TABLES", format!("{mult}.{bandwidth}"));
            for (attr, width) in [
                ("PLL_LOCK_REF_DLY", 5),
                ("PLL_LOCK_FB_DLY", 5),
                ("PLL_LOCK_CNT", 10),
                ("PLL_LOCK_SAT_HIGH", 10),
                ("PLL_UNLOCK_CNT", 10),
            ] {
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
                    .insert_misc_data(format!("PLL:{attr}:{mult}"), ival);
            }
            for (attr, width) in [
                ("PLL_CP_REPL", 4),
                ("PLL_CP", 4),
                ("PLL_RES", 4),
                ("PLL_LFHF", 2),
            ] {
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

    let bel = "CMT";
    for i in 0..16 {
        ctx.collect_enum_default_ocd(
            tile,
            bel,
            &format!("MUX.HCLK{i}"),
            &[
                "CKINT",
                "PLL_CLKOUT0",
                "PLL_CLKOUT1",
                "PLL_CLKOUT2",
                "PLL_CLKOUT3",
                "PLL_CLKOUT4",
                "PLL_CLKOUT5",
                "PLL_CLKFBOUT",
                "PLL_TEST_CLK_OUT",
            ],
            "NONE",
            OcdMode::Mux,
        );
        ctx.collect_enum(tile, bel, &format!("MUX.CASC{i}"), &["PASS", "HCLK"]);
    }

    let tile = "CMT_DCM";
    let bel = "CMT";
    let mut diff = ctx.state.get_diff(tile, bel, "PRESENT_ANY_PLL", "1");
    if !skip_dcm {
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "BG"), 0x520, 1);
        diff.assert_empty();
    }
    for bel in ["DCM0", "DCM1"] {
        ctx.collect_bit(tile, bel, "BUF.CLK_FROM_PLL", "1");
    }
}
