use std::collections::BTreeMap;

use bitvec::prelude::*;
use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_types::{TileItem, TileItemKind};
use unnamed_entity::EntityId;

use crate::{
    backend::{IseBackend, PinFromKind},
    diff::{xlat_bitvec, xlat_enum_default, xlat_enum_ocd, CollectorCtx, Diff, OcdMode},
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits, TileRelation},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_enum_suffix, fuzz_inv, fuzz_multi, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let bel_cmt = BelId::from_idx(2);
    for i in 0..2 {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "CMT_DCM",
            format!("DCM{i}"),
            TileBits::Cmt,
        );
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::AllOtherDcms,
            "CMT_DCM",
            "CMT",
            "PRESENT_ANY_DCM",
            "1",
        )];
        fuzz_one_extras!(ctx, "PRESENT", "DCM", [
            (global_mutex "CMT", format!("PRESENT")),
            (global_mutex_site "CMT_PRESENT")
        ], [(mode "DCM")], extras.clone());
        fuzz_one_extras!(ctx, "PRESENT", "DCM_CLKGEN", [
            (global_mutex "CMT", format!("PRESENT")),
            (global_mutex_site "CMT_PRESENT")
        ], [(mode "DCM_CLKGEN")], extras.clone());

        fuzz_multi!(ctx, "DLL_C", "", 32, [
            (global_mutex "CMT", "CFG"),
            (mode "DCM")
        ], (global_xy_bin "CFG_DLL_C_*"));
        fuzz_multi!(ctx, "DLL_S", "", 32, [
            (global_mutex "CMT", "CFG"),
            (mode "DCM")
        ], (global_xy_bin "CFG_DLL_S_*"));
        fuzz_multi!(ctx, "DFS_C", "", 3, [
            (global_mutex "CMT", "CFG"),
            (mode "DCM")
        ], (global_xy_bin "CFG_DFS_C_*"));
        fuzz_multi!(ctx, "DFS_S", "", 87, [
            (global_mutex "CMT", "CFG"),
            (mode "DCM")
        ], (global_xy_bin "CFG_DFS_S_*"));
        fuzz_multi!(ctx, "INTERFACE", "", 40, [
            (global_mutex "CMT", "CFG"),
            (mode "DCM")
        ], (global_xy_bin "CFG_INTERFACE_*"));
        fuzz_multi!(ctx, "OPT_INV", "", 20, [
            (global_mutex "CMT", "CFG"),
            (mode "DCM")
        ], (global_xy_bin "CFG_OPT_INV_*"));
        fuzz_multi!(ctx, "REG", "", 9, [
            (global_mutex "CMT", format!("CFG_DCM{i}")),
            (mode "DCM")
        ], (global_xy_bin "CFG_REG_*"));
        fuzz_multi!(ctx, "BG", "", 11, [
            (global_mutex "CMT", format!("CFG_DCM{i}")),
            (mode "DCM")
        ], (global_xy_bin "CFG_BG_*"));

        let obel_dcm = BelId::from_idx(i ^ 1);
        for opin in ["CLKIN", "CLKIN_TEST"] {
            for (val, pin) in [
                ("CKINT0", "CLKIN_CKINT0"),
                ("CKINT1", "CLKIN_CKINT1"),
                ("CLK_FROM_PLL", "CLK_FROM_PLL"),
            ] {
                let related_pll = TileRelation::Delta(0, 16, backend.egrid.db.get_node("CMT_PLL"));
                let bel_pll = BelId::from_idx(0);
                fuzz_one!(ctx, format!("MUX.{opin}"), val, [
                    (mode "DCM"),
                    (global_mutex "CMT", "TEST"),
                    (mutex "CLKIN_OUT", opin),
                    (mutex "CLKIN_IN", pin),
                    (tile_mutex "CLKIN_BEL", format!("DCM{i}")),
                    (related related_pll,
                        (pip (bel_pin bel_pll, "CLKOUTDCM0"), (bel_pin bel_pll, format!("CLK_TO_DCM{i}")))
                    ),
                    (related related_pll,
                        (mutex "CLK_TO_DCM", "USE")
                    )
                ], [
                    (pip (pin pin), (pin opin))
                ]);
            }
            for btlr in ["BT", "LR"] {
                for j in 0..8 {
                    fuzz_one!(ctx, format!("MUX.{opin}"), format!("BUFIO2_{btlr}{j}"), [
                        (mode "DCM"),
                        (global_mutex "CMT", "TEST"),
                        (global_mutex "BUFIO2_CMT_OUT", "USE"),
                        (mutex "CLKIN_OUT", opin),
                        (mutex "CLKIN_IN", format!("BUFIO2_{btlr}{j}")),
                        (tile_mutex "CLKIN_BEL", format!("DCM{i}")),
                        (pip (bel_pin bel_cmt, format!("BUFIO2_{btlr}{j}")), (bel_pin obel_dcm, opin))
                    ], [
                        (pip (bel_pin bel_cmt, format!("BUFIO2_{btlr}{j}")), (pin opin))
                    ]);
                }
            }
        }
        for opin in ["CLKFB", "CLKFB_TEST"] {
            for (val, pin) in [("CKINT0", "CLKFB_CKINT0"), ("CKINT1", "CLKFB_CKINT1")] {
                fuzz_one!(ctx, format!("MUX.{opin}"), val, [
                    (mode "DCM"),
                    (global_mutex "CMT", "TEST"),
                    (mutex "CLKIN_OUT", opin),
                    (mutex "CLKIN_IN", pin),
                    (tile_mutex "CLKIN_BEL", format!("DCM{i}"))
                ], [
                    (pip (pin pin), (pin opin))
                ]);
            }
            for btlr in ["BT", "LR"] {
                for j in 0..8 {
                    fuzz_one!(ctx, format!("MUX.{opin}"), format!("BUFIO2FB_{btlr}{j}"), [
                        (mode "DCM"),
                        (global_mutex "CMT", "TEST"),
                        (global_mutex "BUFIO2_CMT_OUT", "USE"),
                        (mutex "CLKIN_OUT", opin),
                        (mutex "CLKIN_IN", format!("BUFIO2FB_{btlr}{j}")),
                        (tile_mutex "CLKIN_BEL", format!("DCM{i}")),
                        (pip (bel_pin bel_cmt, format!("BUFIO2FB_{btlr}{j}")), (bel_pin obel_dcm, opin))
                    ], [
                        (pip (bel_pin bel_cmt, format!("BUFIO2FB_{btlr}{j}")), (pin opin))
                    ]);
                }
            }
        }

        for out in [
            "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX", "CLKFX180",
            "CONCUR",
        ] {
            fuzz_one!(ctx, "MUX.CLK_TO_PLL", out, [
                (global_mutex "CMT", "TEST"),
                (mode "DCM"),
                (pin "CLKDV"),
                (mutex "CLK_TO_PLL", out)
            ], [
                (pip (pin out), (pin "CLK_TO_PLL"))
            ]);
            fuzz_one!(ctx, "MUX.SKEWCLKIN2", out, [
                (global_mutex "CMT", "TEST"),
                (mode "DCM"),
                (pin "CLKDV"),
                (mutex "SKEWCLKIN2", out)
            ], [
                (pip (pin format!("{out}_TEST")), (pin "SKEWCLKIN2"))

            ]);
        }

        for pin in [
            "PSCLK", "PSEN", "PSINCDEC", "RST", "SKEWIN", "CTLGO", "CTLSEL0", "CTLSEL1", "CTLSEL2",
            "SKEWRST",
        ] {
            fuzz_inv!(ctx, pin, [(global_mutex "CMT", "TEST"), (mode "DCM")]);
        }
        for pin in ["PROGCLK", "PROGEN", "PROGDATA", "RST"] {
            fuzz_one!(ctx, format!("{pin}INV.DCM_CLKGEN"), "0", [
                (global_mutex "CMT", "TEST"),
                (mode "DCM_CLKGEN"),
                (pin pin)
            ], [
                (attr format!("{pin}INV"), pin)
            ]);
            fuzz_one!(ctx, format!("{pin}INV.DCM_CLKGEN"), "1", [
                (global_mutex "CMT", "TEST"),
                (mode "DCM_CLKGEN"),
                (pin pin)
            ], [
                (attr format!("{pin}INV"), format!("{pin}_B"))
            ]);
        }
        for pin in [
            "FREEZEDLL",
            "FREEZEDFS",
            "CTLMODE",
            "CTLOSC1",
            "CTLOSC2",
            "STSADRS0",
            "STSADRS1",
            "STSADRS2",
            "STSADRS3",
            "STSADRS4",
        ] {
            fuzz_one!(ctx, format!("PIN.{pin}"), "1", [
                (global_mutex "CMT", "TEST"),
                (mode "DCM")
            ], [
                (pin pin),
                (pin_pips pin)
            ]);
        }

        for pin in [
            "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX", "CLKFX180",
            "CONCUR",
        ] {
            fuzz_one!(ctx, pin, "1", [
                (mode "DCM"),
                (global_mutex "CMT", "PINS"),
                (mutex "PIN", pin),
                (nopin "CLKFB")
            ], [
                (pin pin)
            ]);
            fuzz_one!(ctx, pin, "1.CLKFB", [
                (mode "DCM"),
                (global_mutex "CMT", "PINS"),
                (mutex "PIN", pin),
                (pin "CLKFB")
            ], [
                (pin pin)
            ]);
            if pin != "CLKFX" && pin != "CLKFX180" && pin != "CONCUR" {
                fuzz_one!(ctx, pin, "1.CLKFX", [
                    (mode "DCM"),
                    (global_mutex "CMT", "PINS"),
                    (mutex "PIN", format!("{pin}.CLKFX")),
                    (pin "CLKFX"),
                    (pin "CLKFB")
                ], [
                    (pin pin)
                ]);
            }
        }
        fuzz_one!(ctx, "CLKFB", "1", [
            (mode "DCM"),
            (global_mutex "CMT", "PINS")
        ], [
            (pin "CLKFB")
        ]);
        fuzz_one!(ctx, "CLKIN_IOB", "1", [
            (mode "DCM"),
            (global_mutex "CMT", "USE"),
            (global_opt "GLUTMASK", "NO"),
            (pin "CLKIN"),
            (pin "CLKFB"),
            (pin_from "CLKFB", PinFromKind::Bufg)
        ], [
            (pin_from "CLKIN", PinFromKind::Bufg, PinFromKind::Iob)
        ]);
        fuzz_one!(ctx, "CLKFB_IOB", "1", [
            (mode "DCM"),
            (global_mutex "CMT", "USE"),
            (global_opt "GLUTMASK", "NO"),
            (pin "CLKIN"),
            (pin "CLKFB"),
            (pin_from "CLKIN", PinFromKind::Bufg)
        ], [
            (pin_from "CLKFB", PinFromKind::Bufg, PinFromKind::Iob)
        ]);

        fuzz_one!(ctx, "PIN.PROGCLK", "1", [
            (mode "DCM_CLKGEN"),
            (global_mutex "CMT", "PINS"),
            (nopin "PROGEN"),
            (nopin "PROGDATA")
        ], [
            (pin "PROGCLK")
        ]);
        fuzz_one!(ctx, "PIN.PROGEN", "1", [
            (mode "DCM_CLKGEN"),
            (global_mutex "CMT", "PINS"),
            (nopin "PROGCLK"),
            (nopin "PROGDATA")
        ], [
            (pin "PROGEN")
        ]);
        fuzz_one!(ctx, "PIN.PROGDATA", "1", [
            (mode "DCM_CLKGEN"),
            (global_mutex "CMT", "PINS"),
            (nopin "PROGCLK"),
            (nopin "PROGEN")
        ], [
            (pin "PROGDATA")
        ]);

        fuzz_enum!(ctx, "DSS_MODE", ["NONE", "SPREAD_2", "SPREAD_4", "SPREAD_6", "SPREAD_8"], [
            (global_mutex "CMT", "TEST"),
            (mode "DCM")
        ]);
        fuzz_enum!(ctx, "DLL_FREQUENCY_MODE", ["LOW", "HIGH"], [
            (mode "DCM"),
            (global_mutex "CMT", "USE")
        ]);
        fuzz_enum!(ctx, "DFS_FREQUENCY_MODE", ["LOW", "HIGH"], [
            (mode "DCM"),
            (global_mutex "CMT", "USE")
        ]);
        fuzz_enum!(ctx, "STARTUP_WAIT", ["FALSE", "TRUE"], [
            (global_mutex "CMT", "USE"),
            (mode "DCM"),
            (global_opt "GTS_CYCLE", "1"),
            (global_opt "DONE_CYCLE", "1"),
            (global_opt "LCK_CYCLE", "NOWAIT")
        ]);
        fuzz_enum_suffix!(ctx, "STARTUP_WAIT", "CLKGEN", ["FALSE", "TRUE"], [
            (global_mutex "CMT", "USE"),
            (mode "DCM_CLKGEN"),
            (global_opt "GTS_CYCLE", "1"),
            (global_opt "DONE_CYCLE", "1"),
            (global_opt "LCK_CYCLE", "NOWAIT")
        ]);
        fuzz_enum!(ctx, "DUTY_CYCLE_CORRECTION", ["FALSE", "TRUE"], [
            (mode "DCM"),
            (global_mutex "CMT", "USE")
        ]);
        fuzz_multi!(ctx, "DESKEW_ADJUST", "", 4, [
            (mode "DCM"),
            (global_mutex "CMT", "USE")
        ], (attr_dec "DESKEW_ADJUST"));
        fuzz_enum!(ctx, "CLKIN_DIVIDE_BY_2", ["FALSE", "TRUE"], [
            (mode "DCM"),
            (global_mutex "CMT", "USE")
        ]);
        fuzz_enum!(ctx, "CLK_FEEDBACK", ["NONE", "1X", "2X"], [
            (mode "DCM"),
            (global_mutex "CMT", "USE")
        ]);
        fuzz_multi!(ctx, "CLKFX_MULTIPLY", "", 8, [
            (mode "DCM"),
            (global_mutex "CMT", "USE")
        ], (attr_dec_delta "CLKFX_MULTIPLY", 1));
        fuzz_multi!(ctx, "CLKFX_DIVIDE", "", 8, [
            (mode "DCM"),
            (global_mutex "CMT", "USE")
        ], (attr_dec_delta "CLKFX_DIVIDE", 1));
        fuzz_multi!(ctx, "CLKFX_MULTIPLY.CLKGEN", "", 8, [
            (mode "DCM_CLKGEN"),
            (global_mutex "CMT", "USE")
        ], (attr_dec_delta "CLKFX_MULTIPLY", 1));
        fuzz_multi!(ctx, "CLKFX_DIVIDE.CLKGEN", "", 8, [
            (mode "DCM_CLKGEN"),
            (global_mutex "CMT", "USE")
        ], (attr_dec_delta "CLKFX_DIVIDE", 1));
        fuzz_enum!(ctx, "VERY_HIGH_FREQUENCY", ["FALSE", "TRUE"], [
            (mode "DCM"),
            (global_mutex "CMT", "USE"),
            (pin "CLK0"),
            (nopin "CLKFB")
        ]);

        fuzz_enum!(ctx, "CLKOUT_PHASE_SHIFT", ["NONE", "FIXED", "VARIABLE"], [
            (mode "DCM"),
            (global_mutex "CMT", "USE"),
            (pin "CLK0")
        ]);
        fuzz_multi!(ctx, "PHASE_SHIFT", "", 7, [
            (mode "DCM"),
            (global_mutex "CMT", "USE")
        ], (attr_dec "PHASE_SHIFT"));
        fuzz_one!(ctx, "PHASE_SHIFT", "-1", [
            (mode "DCM"),
            (global_mutex "CMT", "USE")
        ], [
            (attr "PHASE_SHIFT", "-1")
        ]);
        fuzz_one!(ctx, "PHASE_SHIFT", "-255", [
            (mode "DCM"),
            (global_mutex "CMT", "USE")
        ], [
            (attr "PHASE_SHIFT", "-255")
        ]);

        fuzz_enum!(ctx, "CLKDV_DIVIDE", ["2.0", "3.0", "4.0", "5.0", "6.0", "7.0", "8.0", "9.0", "10.0", "11.0", "12.0", "13.0", "14.0", "15.0", "16.0"], [
            (mode "DCM"),
            (global_mutex "CMT", "USE")
        ]);
        for dll_mode in ["LOW", "HIGH"] {
            for val in ["1.5", "2.5", "3.5", "4.5", "5.5", "6.5", "7.5"] {
                fuzz_one!(
                    ctx,
                    "CLKDV_DIVIDE",
                    format!("{val}.{dll_mode}"), [
                        (mode "DCM"),
                        (global_mutex "CMT", "USE"),
                        (attr "DLL_FREQUENCY_MODE", dll_mode),
                        (attr "CLKIN_PERIOD", "")
                    ], [
                        (attr "CLKDV_DIVIDE", val)
                    ]
                );
            }
        }

        fuzz_enum!(ctx, "CLKFXDV_DIVIDE", ["2", "4", "8", "16", "32"], [
            (mode "DCM_CLKGEN"),
            (global_mutex "CMT", "USE")
        ]);
        fuzz_enum!(ctx, "DFS_BANDWIDTH", ["LOW", "HIGH", "OPTIMIZED"], [
            (mode "DCM_CLKGEN"),
            (global_mutex "CMT", "USE")
        ]);
        fuzz_enum!(ctx, "PROG_MD_BANDWIDTH", ["LOW", "HIGH", "OPTIMIZED"], [
            (mode "DCM_CLKGEN"),
            (global_mutex "CMT", "USE")
        ]);

        fuzz_enum!(ctx, "SPREAD_SPECTRUM", [
            "NONE",
            "CENTER_LOW_SPREAD",
            "CENTER_HIGH_SPREAD",
            "VIDEO_LINK_M0",
            "VIDEO_LINK_M1",
            "VIDEO_LINK_M2",
        ], [
            (mode "DCM_CLKGEN"),
            (global_mutex "CMT", "USE"),
            (nopin "PROGCLK"),
            (nopin "PROGEN"),
            (nopin "PROGDATA")
        ]);

        // TODO: CLKIN_PERIOD?
    }

    let ctx = FuzzCtx::new(session, backend, "CMT_DCM", "CMT", TileBits::Cmt);
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
        for j in 0..2 {
            let bel_dcm = BelId::from_idx(j);
            for out in [
                "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX",
                "CLKFX180", "CONCUR",
            ] {
                fuzz_one!(ctx, format!("MUX.HCLK{i}"), format!("DCM{j}_{out}"), [
                    (mutex format!("MUX.HCLK{i}"), format!("DCM{j}_{out}"))
                ], [
                    (pip (bel_pin bel_dcm, format!("{out}_OUT")), (pin format!("HCLK{i}")))
                ]);
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "CMT_DCM";
    for bel in ["DCM0", "DCM1"] {
        let mut present_dcm = ctx.state.get_diff(tile, bel, "PRESENT", "DCM");
        let mut present_dcm_clkgen = ctx.state.get_diff(tile, bel, "PRESENT", "DCM_CLKGEN");

        let mut cfg_interface = ctx.state.get_diffs(tile, bel, "INTERFACE", "");
        cfg_interface.reverse();
        let mut cfg_dll_c = ctx.state.get_diffs(tile, bel, "DLL_C", "");
        cfg_dll_c.reverse();
        let cfg_dll_c = xlat_bitvec(cfg_dll_c);
        for attr in ["DLL_S", "DFS_C", "DFS_S"] {
            let mut diffs = ctx.state.get_diffs(tile, bel, attr, "");
            diffs.reverse();
            ctx.tiledb.insert(tile, bel, attr, xlat_bitvec(diffs));
        }
        for attr in ["REG", "BG"] {
            let mut diffs = ctx.state.get_diffs(tile, bel, attr, "");
            diffs.reverse();
            ctx.tiledb.insert(tile, "CMT", attr, xlat_bitvec(diffs));
        }
        let mut cfg_opt_inv = ctx.state.get_diffs(tile, bel, "OPT_INV", "");
        cfg_opt_inv.reverse();
        ctx.tiledb
            .insert(tile, bel, "OPT_INV", xlat_bitvec(cfg_opt_inv[..3].to_vec()));
        for pin in [
            "PSEN", "PSINCDEC", "RST", "SKEWIN", "CTLGO", "CTLSEL0", "CTLSEL1", "CTLSEL2",
            "SKEWRST",
        ] {
            ctx.collect_inv(tile, bel, pin);
        }
        for (hwpin, pin) in [("PSEN", "PROGEN"), ("PSINCDEC", "PROGDATA"), ("RST", "RST")] {
            let item = ctx.extract_enum_bool(tile, bel, &format!("{pin}INV.DCM_CLKGEN"), "0", "1");
            ctx.tiledb.insert(tile, bel, format!("INV.{hwpin}"), item);
        }
        for pin in [
            "FREEZEDLL",
            "FREEZEDFS",
            "CTLMODE",
            "CTLOSC1",
            "CTLOSC2",
            "STSADRS0",
            "STSADRS1",
            "STSADRS2",
            "STSADRS3",
            "STSADRS4",
        ] {
            let diff = ctx.state.get_diff(tile, bel, format!("PIN.{pin}"), "1");
            present_dcm = present_dcm.combine(&diff);
            present_dcm_clkgen = present_dcm.combine(&diff);
            ctx.tiledb
                .insert(tile, bel, format!("INV.{pin}"), xlat_bitvec(vec![!diff]));
        }

        // hrm. concerning.
        ctx.state
            .get_diff(tile, bel, "PSCLKINV", "PSCLK")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "PSCLKINV", "PSCLK_B")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "PROGCLKINV.DCM_CLKGEN", "0")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "PROGCLKINV.DCM_CLKGEN", "1")
            .assert_empty();

        let (_, _, clkin_clkfb_enable) = Diff::split(
            ctx.state
                .peek_diff(tile, bel, "MUX.CLKIN", "CKINT0")
                .clone(),
            ctx.state
                .peek_diff(tile, bel, "MUX.CLKFB", "CKINT0")
                .clone(),
        );
        let mut diffs = vec![];
        for out in ["CLKIN", "CLKIN_TEST"] {
            for val in [
                "BUFIO2_LR0",
                "BUFIO2_LR1",
                "BUFIO2_LR2",
                "BUFIO2_LR3",
                "BUFIO2_LR4",
                "BUFIO2_LR5",
                "BUFIO2_LR6",
                "BUFIO2_LR7",
                "BUFIO2_BT0",
                "BUFIO2_BT1",
                "BUFIO2_BT2",
                "BUFIO2_BT3",
                "BUFIO2_BT4",
                "BUFIO2_BT5",
                "BUFIO2_BT6",
                "BUFIO2_BT7",
                "CKINT0",
                "CKINT1",
                "CLK_FROM_PLL",
            ] {
                let mut diff = ctx.state.get_diff(tile, bel, format!("MUX.{out}"), val);
                diff = diff.combine(&!&clkin_clkfb_enable);
                diffs.push((val.to_string(), diff));
            }
        }
        ctx.tiledb
            .insert(tile, bel, "MUX.CLKIN", xlat_enum_ocd(diffs, OcdMode::Mux));
        let mut diffs = vec![];
        for out in ["CLKFB", "CLKFB_TEST"] {
            for val in [
                "BUFIO2FB_LR0",
                "BUFIO2FB_LR1",
                "BUFIO2FB_LR2",
                "BUFIO2FB_LR3",
                "BUFIO2FB_LR4",
                "BUFIO2FB_LR5",
                "BUFIO2FB_LR6",
                "BUFIO2FB_LR7",
                "BUFIO2FB_BT0",
                "BUFIO2FB_BT1",
                "BUFIO2FB_BT2",
                "BUFIO2FB_BT3",
                "BUFIO2FB_BT4",
                "BUFIO2FB_BT5",
                "BUFIO2FB_BT6",
                "BUFIO2FB_BT7",
                "CKINT0",
                "CKINT1",
            ] {
                let mut diff = ctx.state.get_diff(tile, bel, format!("MUX.{out}"), val);
                diff = diff.combine(&!&clkin_clkfb_enable);
                diffs.push((val.to_string(), diff));
            }
        }
        ctx.tiledb
            .insert(tile, bel, "MUX.CLKFB", xlat_enum_ocd(diffs, OcdMode::Mux));
        ctx.tiledb.insert(
            tile,
            bel,
            "CLKIN_CLKFB_ENABLE",
            xlat_bitvec(vec![clkin_clkfb_enable]),
        );
        ctx.collect_enum(
            tile,
            bel,
            "MUX.CLK_TO_PLL",
            &[
                "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX",
                "CLKFX180", "CONCUR",
            ],
        );
        ctx.collect_enum(
            tile,
            bel,
            "MUX.SKEWCLKIN2",
            &[
                "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX",
                "CLKFX180", "CONCUR",
            ],
        );

        ctx.collect_enum_bool(tile, bel, "DUTY_CYCLE_CORRECTION", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "CLKIN_DIVIDE_BY_2", "FALSE", "TRUE");
        ctx.collect_enum(tile, bel, "CLK_FEEDBACK", &["1X", "2X"]);
        ctx.collect_enum(tile, bel, "DLL_FREQUENCY_MODE", &["LOW", "HIGH"]);
        ctx.collect_enum(tile, bel, "DFS_FREQUENCY_MODE", &["LOW", "HIGH"]);
        ctx.collect_bitvec(tile, bel, "DESKEW_ADJUST", "");
        ctx.collect_bitvec(tile, bel, "CLKFX_MULTIPLY", "");
        ctx.collect_bitvec(tile, bel, "CLKFX_DIVIDE", "");
        let item = ctx.extract_bitvec(tile, bel, "CLKFX_MULTIPLY.CLKGEN", "");
        ctx.tiledb.insert(tile, bel, "CLKFX_MULTIPLY", item);
        let item = ctx.extract_bitvec(tile, bel, "CLKFX_DIVIDE.CLKGEN", "");
        ctx.tiledb.insert(tile, bel, "CLKFX_DIVIDE", item);
        ctx.collect_bit(tile, bel, "CLKIN_IOB", "1");
        ctx.collect_bit(tile, bel, "CLKFB_IOB", "1");
        ctx.collect_enum_bool(tile, bel, "STARTUP_WAIT", "FALSE", "TRUE");
        let item = ctx.extract_enum_bool(tile, bel, "STARTUP_WAIT.CLKGEN", "FALSE", "TRUE");
        ctx.tiledb.insert(tile, bel, "STARTUP_WAIT", item);
        let item = ctx.extract_bit(tile, bel, "CLK_FEEDBACK", "NONE");
        ctx.tiledb.insert(tile, bel, "NO_FEEDBACK", item);

        ctx.state.get_diff(tile, bel, "CLKFB", "1").assert_empty();

        let (_, _, dll_en) = Diff::split(
            ctx.state.peek_diff(tile, bel, "CLK0", "1").clone(),
            ctx.state.peek_diff(tile, bel, "CLK180", "1").clone(),
        );

        for pin in [
            "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV",
        ] {
            let diff = ctx.state.get_diff(tile, bel, pin, "1");
            let diff_fb = ctx.state.get_diff(tile, bel, pin, "1.CLKFB");
            assert_eq!(diff, diff_fb);
            let diff_fx = ctx.state.get_diff(tile, bel, pin, "1.CLKFX");
            let diff_fx = diff_fx.combine(&!&diff);
            let mut diff = diff.combine(&!&dll_en);
            // hrm.
            if ctx.device.name.ends_with('l') && pin == "CLKDV" {
                diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DLL_S"), 0, 0x40);
            }
            ctx.tiledb.insert(tile, bel, pin, xlat_bitvec(vec![diff]));
            ctx.tiledb
                .insert(tile, bel, "DFS_FEEDBACK", xlat_bitvec(vec![diff_fx]));
        }
        ctx.tiledb
            .insert(tile, bel, "DLL_ENABLE", xlat_bitvec(vec![dll_en]));

        ctx.state
            .get_diff(tile, bel, "VERY_HIGH_FREQUENCY", "FALSE")
            .assert_empty();
        let diff = ctx.state.get_diff(tile, bel, "VERY_HIGH_FREQUENCY", "TRUE");
        ctx.tiledb
            .insert(tile, bel, "DLL_ENABLE", xlat_bitvec(vec![!diff]));

        for attr in ["PIN.PROGCLK", "PIN.PROGEN", "PIN.PROGDATA"] {
            let item = ctx.extract_bit(tile, bel, attr, "1");
            ctx.tiledb.insert(tile, bel, "PROG_ENABLE", item);
        }

        let (_, _, dfs_en) = Diff::split(
            ctx.state.peek_diff(tile, bel, "CLKFX", "1").clone(),
            ctx.state.peek_diff(tile, bel, "CONCUR", "1").clone(),
        );
        for pin in ["CLKFX", "CLKFX180", "CONCUR"] {
            let diff = ctx.state.get_diff(tile, bel, pin, "1");
            let diff_fb = ctx.state.get_diff(tile, bel, pin, "1.CLKFB");
            assert_eq!(diff, diff_fb);
            let diff = diff.combine(&!&dfs_en);
            let pin = if pin == "CONCUR" { pin } else { "CLKFX" };
            ctx.tiledb.insert(tile, bel, pin, xlat_bitvec(vec![diff]));
        }
        ctx.tiledb
            .insert(tile, bel, "DFS_ENABLE", xlat_bitvec(vec![dfs_en]));

        let mut diffs = vec![ctx.state.get_diff(tile, bel, "PHASE_SHIFT", "-255")];
        diffs.extend(ctx.state.get_diffs(tile, bel, "PHASE_SHIFT", ""));
        let item = xlat_bitvec(diffs);
        let mut diff = ctx.state.get_diff(tile, bel, "PHASE_SHIFT", "-1");
        diff.apply_bitvec_diff_int(&item, 2, 0);
        ctx.tiledb.insert(tile, bel, "PHASE_SHIFT", item);
        ctx.tiledb
            .insert(tile, bel, "PHASE_SHIFT_NEGATIVE", xlat_bitvec(vec![diff]));

        ctx.collect_enum(
            tile,
            bel,
            "CLKOUT_PHASE_SHIFT",
            &["NONE", "FIXED", "VARIABLE"],
        );

        let mut diffs = vec![];
        for val in [
            "NONE",
            "CENTER_LOW_SPREAD",
            "CENTER_HIGH_SPREAD",
            "VIDEO_LINK_M0",
            "VIDEO_LINK_M1",
            "VIDEO_LINK_M2",
        ] {
            let mut diff = ctx.state.get_diff(tile, bel, "SPREAD_SPECTRUM", val);
            if val.starts_with("VIDEO_LINK") {
                diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "PROG_ENABLE"), true, false);
            }
            diffs.push((val.to_string(), diff));
        }
        ctx.tiledb.insert(
            tile,
            bel,
            "SPREAD_SPECTRUM",
            xlat_enum_default(diffs, "DCM"),
        );

        for (attr, bits) in [
            ("CLKDV_COUNT_MAX", &cfg_dll_c.bits[1..5]),
            ("CLKDV_COUNT_FALL", &cfg_dll_c.bits[5..9]),
            ("CLKDV_COUNT_FALL_2", &cfg_dll_c.bits[9..13]),
            ("CLKDV_PHASE_RISE", &cfg_dll_c.bits[13..15]),
            ("CLKDV_PHASE_FALL", &cfg_dll_c.bits[15..17]),
        ] {
            ctx.tiledb.insert(
                tile,
                bel,
                attr,
                TileItem {
                    bits: bits.to_vec(),
                    kind: TileItemKind::BitVec {
                        invert: bitvec![0; bits.len()],
                    },
                },
            );
        }
        ctx.tiledb.insert(
            tile,
            bel,
            "CLKDV_MODE",
            TileItem {
                bits: cfg_dll_c.bits[17..18].to_vec(),
                kind: TileItemKind::Enum {
                    values: BTreeMap::from_iter([
                        ("HALF".to_string(), bitvec![0]),
                        ("INT".to_string(), bitvec![1]),
                    ]),
                },
            },
        );

        let clkdv_count_max = ctx.tiledb.item(tile, bel, "CLKDV_COUNT_MAX");
        let clkdv_count_fall = ctx.tiledb.item(tile, bel, "CLKDV_COUNT_FALL");
        let clkdv_count_fall_2 = ctx.tiledb.item(tile, bel, "CLKDV_COUNT_FALL_2");
        let clkdv_phase_fall = ctx.tiledb.item(tile, bel, "CLKDV_PHASE_FALL");
        let clkdv_mode = ctx.tiledb.item(tile, bel, "CLKDV_MODE");
        for i in 2..=16 {
            let mut diff = ctx
                .state
                .get_diff(tile, bel, "CLKDV_DIVIDE", format!("{i}.0"));
            diff.apply_bitvec_diff_int(clkdv_count_max, i - 1, 1);
            diff.apply_bitvec_diff_int(clkdv_count_fall, (i - 1) / 2, 0);
            diff.apply_bitvec_diff_int(clkdv_phase_fall, (i % 2) * 2, 0);
            diff.assert_empty();
        }
        for i in 1..=7 {
            let mut diff = ctx
                .state
                .get_diff(tile, bel, "CLKDV_DIVIDE", format!("{i}.5.LOW"));
            diff.apply_enum_diff(clkdv_mode, "HALF", "INT");
            diff.apply_bitvec_diff_int(clkdv_count_max, 2 * i, 1);
            diff.apply_bitvec_diff_int(clkdv_count_fall, i / 2, 0);
            diff.apply_bitvec_diff_int(clkdv_count_fall_2, 3 * i / 2 + 1, 0);
            diff.apply_bitvec_diff_int(clkdv_phase_fall, (i % 2) * 2 + 1, 0);
            diff.assert_empty();
            let mut diff = ctx
                .state
                .get_diff(tile, bel, "CLKDV_DIVIDE", format!("{i}.5.HIGH"));
            diff.apply_enum_diff(clkdv_mode, "HALF", "INT");
            diff.apply_bitvec_diff_int(clkdv_count_max, 2 * i, 1);
            diff.apply_bitvec_diff_int(clkdv_count_fall, (i - 1) / 2, 0);
            diff.apply_bitvec_diff_int(clkdv_count_fall_2, (3 * i + 1) / 2, 0);
            diff.apply_bitvec_diff_int(clkdv_phase_fall, (i % 2) * 2, 0);
            diff.assert_empty();
        }

        for val in ["NONE", "SPREAD_2", "SPREAD_4", "SPREAD_6", "SPREAD_8"] {
            ctx.state
                .get_diff(tile, bel, "DSS_MODE", val)
                .assert_empty();
        }
        for val in ["LOW", "HIGH", "OPTIMIZED"] {
            ctx.state
                .get_diff(tile, bel, "DFS_BANDWIDTH", val)
                .assert_empty();
            ctx.state
                .get_diff(tile, bel, "PROG_MD_BANDWIDTH", val)
                .assert_empty();
        }
        let mut item = ctx.extract_enum(tile, bel, "CLKFXDV_DIVIDE", &["32", "16", "8", "4", "2"]);
        assert_eq!(item.bits.len(), 3);
        let TileItemKind::Enum { ref mut values } = item.kind else {
            unreachable!()
        };
        values.insert("NONE".into(), bitvec![0; 3]);
        ctx.tiledb.insert(tile, bel, "CLKFXDV_DIVIDE", item);

        ctx.tiledb.insert(tile, bel, "DLL_C", cfg_dll_c);

        present_dcm.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKDV_COUNT_MAX"), 1, 0);
        present_dcm.apply_enum_diff(ctx.tiledb.item(tile, bel, "CLKDV_MODE"), "INT", "HALF");
        present_dcm_clkgen.apply_bitvec_diff_int(
            ctx.tiledb.item(tile, bel, "CLKDV_COUNT_MAX"),
            1,
            0,
        );
        present_dcm_clkgen.apply_enum_diff(ctx.tiledb.item(tile, bel, "CLKDV_MODE"), "INT", "HALF");
        present_dcm.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DESKEW_ADJUST"), 11, 0);
        present_dcm_clkgen.apply_bitvec_diff_int(
            ctx.tiledb.item(tile, bel, "DESKEW_ADJUST"),
            11,
            0,
        );
        present_dcm.apply_enum_diff(ctx.tiledb.item(tile, bel, "CLKFXDV_DIVIDE"), "2", "NONE");
        present_dcm_clkgen.apply_enum_diff(
            ctx.tiledb.item(tile, bel, "CLKFXDV_DIVIDE"),
            "2",
            "NONE",
        );
        present_dcm.apply_bit_diff(
            ctx.tiledb.item(tile, bel, "DUTY_CYCLE_CORRECTION"),
            true,
            false,
        );
        present_dcm_clkgen.apply_bit_diff(
            ctx.tiledb.item(tile, bel, "DUTY_CYCLE_CORRECTION"),
            true,
            false,
        );
        present_dcm.apply_bitvec_diff(
            ctx.tiledb.item(tile, "CMT", "REG"),
            &bitvec![1, 1, 0, 0, 0, 0, 1, 0, 1],
            &bitvec![0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        present_dcm_clkgen.apply_bitvec_diff(
            ctx.tiledb.item(tile, "CMT", "REG"),
            &bitvec![1, 1, 0, 0, 0, 0, 1, 0, 1],
            &bitvec![0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        present_dcm.apply_bitvec_diff(
            ctx.tiledb.item(tile, "CMT", "BG"),
            &bitvec![0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 1],
            &bitvec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        present_dcm_clkgen.apply_bitvec_diff(
            ctx.tiledb.item(tile, "CMT", "BG"),
            &bitvec![0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 1],
            &bitvec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );

        // ???
        present_dcm_clkgen.apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.STSADRS4"), false, true);

        let mut base_dfs_s = BitVec::repeat(false, 87);
        base_dfs_s.set(17, true);
        base_dfs_s.set(21, true);
        base_dfs_s.set(32, true);
        base_dfs_s.set(33, true);
        base_dfs_s.set(37, true);
        base_dfs_s.set(41, true);
        base_dfs_s.set(43, true);
        base_dfs_s.set(45, true);
        base_dfs_s.set(52, true);
        base_dfs_s.set(64, true);
        base_dfs_s.set(68, true);
        base_dfs_s.set(76, true);
        base_dfs_s.set(77, true);
        present_dcm.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, "DFS_S"),
            &base_dfs_s,
            &bitvec![0; 87],
        );
        present_dcm_clkgen.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, "DFS_S"),
            &base_dfs_s,
            &bitvec![0; 87],
        );

        let mut base_dll_s = BitVec::repeat(false, 32);
        base_dll_s.set(0, true);
        base_dll_s.set(6, true);
        base_dll_s.set(13, true); // period not hf
        present_dcm.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, "DLL_S"),
            &base_dll_s,
            &bitvec![0; 32],
        );
        present_dcm_clkgen.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, "DLL_S"),
            &base_dll_s,
            &bitvec![0; 32],
        );

        present_dcm = present_dcm.combine(&!&cfg_interface[9]);
        present_dcm = present_dcm.combine(&!&cfg_interface[10]);
        present_dcm = present_dcm.combine(&!&cfg_interface[12]);
        present_dcm = present_dcm.combine(&!&cfg_interface[13]);
        present_dcm_clkgen = present_dcm_clkgen.combine(&!&cfg_interface[9]);
        present_dcm_clkgen = present_dcm_clkgen.combine(&!&cfg_interface[10]);
        present_dcm_clkgen = present_dcm_clkgen.combine(&!&cfg_interface[12]);
        present_dcm_clkgen = present_dcm_clkgen.combine(&!&cfg_interface[13]);

        assert_eq!(present_dcm.bits.len(), 1);
        assert_eq!(present_dcm, present_dcm_clkgen);
        cfg_interface[18].assert_empty();
        cfg_interface[18] = present_dcm;
        ctx.tiledb
            .insert(tile, bel, "INTERFACE", xlat_bitvec(cfg_interface));
    }

    let bel = "CMT";
    let mut diff = ctx.state.get_diff(tile, bel, "PRESENT_ANY_DCM", "1");
    diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "BG"), 0, 1);
    diff.assert_empty();

    for i in 0..16 {
        ctx.collect_enum_default_ocd(
            tile,
            bel,
            &format!("MUX.HCLK{i}"),
            &[
                "CKINT",
                "DCM0_CLK0",
                "DCM0_CLK90",
                "DCM0_CLK180",
                "DCM0_CLK270",
                "DCM0_CLK2X",
                "DCM0_CLK2X180",
                "DCM0_CLKDV",
                "DCM0_CLKFX",
                "DCM0_CLKFX180",
                "DCM0_CONCUR",
                "DCM1_CLK0",
                "DCM1_CLK90",
                "DCM1_CLK180",
                "DCM1_CLK270",
                "DCM1_CLK2X",
                "DCM1_CLK2X180",
                "DCM1_CLKDV",
                "DCM1_CLKFX",
                "DCM1_CLKFX180",
                "DCM1_CONCUR",
            ],
            "NONE",
            OcdMode::Mux,
        );
        ctx.collect_enum(tile, bel, &format!("MUX.CASC{i}"), &["PASS", "HCLK"]);
    }
}
