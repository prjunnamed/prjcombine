use prjcombine_re_collector::{xlat_bit, xlat_enum_ocd, Diff, OcdMode};
use prjcombine_re_hammer::Session;
use prjcombine_interconnect::db::BelId;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{BelRelation, ExtraFeature, ExtraFeatureKind, TileBits, TileRelation},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    {
        let ctx = FuzzCtx::new(session, backend, "HCLK", "HCLK", TileBits::Hclk);
        for i in 0..8 {
            for ud in ['U', 'D'] {
                for j in 0..12 {
                    fuzz_one!(ctx, format!("MUX.LCLK{i}_{ud}"), format!("HCLK{j}"), [
                        (global_mutex "HCLK", "USE"),
                        (row_mutex "BUFH_TEST", format!("USED_HCLK{j}")),
                        (pip
                            (related_pin BelRelation::Rclk, format!("HCLK{j}_L_I")),
                            (related_pin BelRelation::Rclk, "BUFH_TEST_L_PRE")),
                        (pip
                            (related_pin BelRelation::Rclk, format!("HCLK{j}_R_I")),
                            (related_pin BelRelation::Rclk, "BUFH_TEST_R_PRE")),
                        (mutex format!("MUX.LCLK{i}_{ud}"), format!("HCLK{j}")),
                        (mutex format!("HCLK{j}"), format!("MUX.LCLK{i}_{ud}"))
                    ], [
                        (pip (pin format!("HCLK{j}")), (pin format!("LCLK{i}_{ud}")))
                    ]);
                }
                for j in 0..6 {
                    fuzz_one!(ctx, format!("MUX.LCLK{i}_{ud}"), format!("RCLK{j}"), [
                        (global_mutex "RCLK", "USE"),
                        (row_mutex "BUFH_TEST", format!("USED_RCLK{j}")),
                        (pip
                            (related_pin BelRelation::Rclk, format!("RCLK{j}_L_I")),
                            (related_pin BelRelation::Rclk, "BUFH_TEST_L_PRE")),
                        (pip
                            (related_pin BelRelation::Rclk, format!("RCLK{j}_R_I")),
                            (related_pin BelRelation::Rclk, "BUFH_TEST_R_PRE")),
                        (mutex format!("MUX.LCLK{i}_{ud}"), format!("RCLK{j}")),
                        (mutex format!("RCLK{j}"), format!("MUX.LCLK{i}_{ud}"))
                    ], [
                        (pip (pin format!("RCLK{j}")), (pin format!("LCLK{i}_{ud}")))
                    ]);
                }
            }
        }
    }

    for (tile, gio, bits, base) in [
        ("CMT_BUFG_BOT", "GIO_BOT", TileBits::Main(2, 2), 0),
        ("CMT_BUFG_TOP", "GIO_TOP", TileBits::Main(0, 2), 16),
    ] {
        let obel_gio = BelId::from_idx(16);
        for i in 0..16 {
            let ctx = FuzzCtx::new(
                session,
                backend,
                tile,
                format!("BUFGCTRL{}", base + i),
                bits.clone(),
            );
            fuzz_one!(ctx, "PRESENT", "1", [], [(mode "BUFGCTRL")]);
            for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
                fuzz_inv!(ctx, pin, [(mode "BUFGCTRL")]);
            }
            fuzz_enum!(ctx, "PRESELECT_I0", ["FALSE", "TRUE"], [(mode "BUFGCTRL")]);
            fuzz_enum!(ctx, "PRESELECT_I1", ["FALSE", "TRUE"], [(mode "BUFGCTRL")]);
            fuzz_enum!(ctx, "CREATE_EDGE", ["FALSE", "TRUE"], [(mode "BUFGCTRL")]);
            fuzz_enum!(ctx, "INIT_OUT", ["0", "1"], [(mode "BUFGCTRL")]);
            fuzz_one!(ctx, "ENABLE.FB", "1", [], [
                (pip (pin "O"), (pin "FB"))
            ]);
            let extras = vec![
                ExtraFeature::new(
                    ExtraFeatureKind::Cmt(-20),
                    "CMT",
                    "CMT",
                    format!("ENABLE.GCLK{}", base + i),
                    "1",
                ),
                ExtraFeature::new(
                    ExtraFeatureKind::Cmt(20),
                    "CMT",
                    "CMT",
                    format!("ENABLE.GCLK{}", base + i),
                    "1",
                ),
            ];
            fuzz_one_extras!(ctx, "ENABLE.GCLK", "1", [
                (global_mutex "GCLK", "TEST")
            ], [
                (pip (pin "O"), (pin "GCLK"))
            ], extras);
            // ISE bug causes pips to be reversed?
            fuzz_one!(ctx, "TEST_I0", "1", [
                (mutex "MUX.I0", "FB_TEST")
            ], [
                (pip (pin "I0_FB_TEST"), (pin "I0"))
            ]);
            fuzz_one!(ctx, "TEST_I1", "1", [
                (mutex "MUX.I1", "FB_TEST")
            ], [
                (pip (pin "I1_FB_TEST"), (pin "I1"))
            ]);
            for j in 0..2 {
                for k in 0..8 {
                    fuzz_one!(ctx, format!("MUX.I{j}"), format!("GIO{k}"), [
                        (mutex format!("MUX.I{j}"), format!("GIO{k}"))
                    ], [
                        (pip (bel_pin obel_gio, format!("GIO{k}_BUFG")), (pin format!("I{j}")))
                    ]);
                }
                fuzz_one!(ctx, format!("MUX.I{j}"), "CASCI", [
                    (mutex format!("MUX.I{j}"), "CASCI")
                ], [
                    (pip (pin format!("I{j}_CASCI")), (pin format!("I{j}")))
                ]);
                fuzz_one!(ctx, format!("MUX.I{j}"), "CKINT", [
                    (mutex format!("MUX.I{j}"), "CKINT")
                ], [
                    (pip (pin format!("I{j}_CKINT")), (pin format!("I{j}")))
                ]);
                let obel_prev = BelId::from_idx((i + 15) % 16);
                fuzz_one!(ctx, format!("MUX.I{j}"), "FB_PREV", [
                    (mutex format!("MUX.I{j}"), "FB_PREV")
                ], [
                    (pip (bel_pin obel_prev, "FB"), (pin format!("I{j}")))
                ]);
                let obel_next = BelId::from_idx((i + 1) % 16);
                fuzz_one!(ctx, format!("MUX.I{j}"), "FB_NEXT", [
                    (mutex format!("MUX.I{j}"), "FB_NEXT")
                ], [
                    (pip (bel_pin obel_next, "FB"), (pin format!("I{j}")))
                ]);
            }
        }
        let ctx = FuzzCtx::new(session, backend, tile, gio, TileBits::Null);
        let gio_base = base / 4;
        for i in gio_base..(gio_base + 4) {
            let extras = vec![
                ExtraFeature::new(
                    ExtraFeatureKind::Cmt(-20),
                    "CMT",
                    "CMT",
                    format!("ENABLE.GIO{}", i),
                    "1",
                ),
                ExtraFeature::new(
                    ExtraFeatureKind::Cmt(20),
                    "CMT",
                    "CMT",
                    format!("ENABLE.GIO{}", i),
                    "1",
                ),
            ];
            fuzz_one_extras!(ctx, format!("ENABLE.GIO{i}"), "1", [
                (global_mutex "GIO", "TEST")
            ], [
                (pip (pin format!("GIO{i}")), (pin format!("GIO{i}_CMT")))
            ], extras);
        }
    }
    let hclk_ioi = backend.egrid.db.get_node("HCLK_IOI");
    for i in 0..4 {
        let bel_hclk_ioi = BelId::from_idx(10);
        let ctx = FuzzCtx::new(
            session,
            backend,
            "HCLK_IOI",
            format!("BUFIODQS{i}"),
            TileBits::Hclk,
        );
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "BUFIODQS")]);
        fuzz_enum!(ctx, "DQSMASK_ENABLE", ["FALSE", "TRUE"], [
            (mode "BUFIODQS")
        ]);
        fuzz_one!(ctx, "MUX.I", format!("PERF{}", i ^ 1), [
            (mutex "MUX.I", "PERF")
        ], [
            (pip
                (bel_pin bel_hclk_ioi, format!("PERF{}_BUF", i ^ 1)),
                (bel_pin bel_hclk_ioi, format!("IOCLK_IN{i}")))
        ]);
        fuzz_one!(ctx, "MUX.I", "CCIO", [
            (mutex "MUX.I", "CCIO")
        ], [
            (pip
                (bel_pin bel_hclk_ioi, format!("IOCLK_PAD{i}")),
                (bel_pin bel_hclk_ioi, format!("IOCLK_IN{i}")))
        ]);
        fuzz_one!(ctx, "ENABLE", "1", [], [
            (pip (pin "O"), (bel_pin bel_hclk_ioi, format!("IOCLK{i}_PRE")))
        ]);
    }
    for i in 0..2 {
        let bel_hclk_ioi = BelId::from_idx(10);
        let bel_other = BelId::from_idx(5 - i);
        let ctx = FuzzCtx::new(
            session,
            backend,
            "HCLK_IOI",
            format!("BUFR{i}"),
            TileBits::Hclk,
        );
        fuzz_one!(ctx, "ENABLE", "1", [
            (global_mutex "RCLK", "USE"),
            (row_mutex "BUFH_TEST", "USED_RCLK0"),
            (pip
                (related_pin BelRelation::Rclk, "RCLK0_L_I"),
                (related_pin BelRelation::Rclk, "BUFH_TEST_L_PRE")),
            (pip
                (related_pin BelRelation::Rclk, "RCLK0_R_I"),
                (related_pin BelRelation::Rclk, "BUFH_TEST_R_PRE"))
        ], [
            (mode "BUFR")
        ]);
        fuzz_enum!(ctx, "BUFR_DIVIDE", ["BYPASS", "1", "2", "3", "4", "5", "6", "7", "8"], [
            (global_mutex "RCLK", "USE"),
            (row_mutex "BUFH_TEST", "USED_RCLK0"),
            (pip
                (related_pin BelRelation::Rclk, "RCLK0_L_I"),
                (related_pin BelRelation::Rclk, "BUFH_TEST_L_PRE")),
            (pip
                (related_pin BelRelation::Rclk, "RCLK0_R_I"),
                (related_pin BelRelation::Rclk, "BUFH_TEST_R_PRE")),
            (mode "BUFR")
        ]);
        for j in 0..10 {
            fuzz_one!(ctx, "MUX.I", format!("MGT{j}"), [
                (row_mutex "MGT", "USE"),
                (mutex "MUX.I", format!("MGT{j}")),
                (bel_mutex bel_other, "MUX.I", format!("MGT{j}")),
                (pip (bel_pin bel_hclk_ioi, format!("MGT{j}")), (bel_pin bel_other, "I"))
            ], [
                (pip (bel_pin bel_hclk_ioi, format!("MGT{j}")), (pin "I"))
            ]);
        }
        for j in 0..4 {
            fuzz_one!(ctx, "MUX.I", format!("BUFIO{j}_I"), [
                (mutex "MUX.I", format!("BUFIO{j}_I"))
            ], [
                (pip (bel_pin bel_hclk_ioi, format!("IOCLK_IN{j}_BUFR")), (pin "I"))
            ]);
        }
        for j in 0..2 {
            fuzz_one!(ctx, "MUX.I", format!("CKINT{j}"), [
                (mutex "MUX.I", format!("CKINT{j}"))
            ], [
                (pip (bel_pin bel_hclk_ioi, format!("BUFR_CKINT{j}")), (pin "I"))
            ]);
        }
    }
    for i in 0..2 {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "HCLK_IOI",
            format!("BUFO{i}"),
            TileBits::Hclk,
        );
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "BUFO")]);
        for (val, pin) in [
            (format!("VOCLK{i}"), "I_PRE"),
            (format!("VOCLK{i}_S"), "VI_S"),
            (format!("VOCLK{i}_N"), "VI_N"),
        ] {
            fuzz_one!(ctx, "MUX.I", val, [
                (mutex "MUX.I", pin),
                (related TileRelation::Delta(0, 40, hclk_ioi), (nop)),
                (related TileRelation::Delta(0, -40, hclk_ioi), (nop))
            ], [
                (pip (pin pin), (pin "I"))
            ]);
        }
    }
    {
        let bel_hclk_ioi = BelId::from_idx(10);
        let ctx = FuzzCtx::new(session, backend, "HCLK_IOI", "IDELAYCTRL", TileBits::Hclk);
        for i in 0..12 {
            fuzz_one!(ctx, "MUX.REFCLK", format!("HCLK{i}"), [
                (mutex "MUX.REFCLK", format!("HCLK{i}"))
            ], [
                (pip (bel_pin bel_hclk_ioi, format!("HCLK{i}_O")), (pin "REFCLK"))
            ]);
        }
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "IDELAYCTRL")]);
        fuzz_enum!(ctx, "RESET_STYLE", ["V4", "V5"], [(mode "IDELAYCTRL")]);
        fuzz_enum!(ctx, "HIGH_PERFORMANCE_MODE", ["FALSE", "TRUE"], [(mode "IDELAYCTRL")]);
        fuzz_one!(ctx, "MODE", "DEFAULT", [
            (tile_mutex "IDELAYCTRL", "TEST"),
            (mode "IDELAYCTRL")
        ], [
            (attr "IDELAYCTRL_EN", "DEFAULT"),
            (attr "BIAS_MODE", "2")
        ]);
        fuzz_one!(ctx, "MODE", "FULL_0", [
            (tile_mutex "IDELAYCTRL", "TEST"),
            (mode "IDELAYCTRL")
        ], [
            (attr "IDELAYCTRL_EN", "ENABLE"),
            (attr "BIAS_MODE", "0")
        ]);
        fuzz_one!(ctx, "MODE", "FULL_1", [
            (tile_mutex "IDELAYCTRL", "TEST"),
            (mode "IDELAYCTRL")
        ], [
            (attr "IDELAYCTRL_EN", "ENABLE"),
            (attr "BIAS_MODE", "1")
        ]);
    }
    {
        let ctx = FuzzCtx::new(session, backend, "HCLK_IOI", "HCLK_IOI", TileBits::Hclk);
        for i in 0..12 {
            fuzz_one!(ctx, format!("BUF.HCLK{i}"), "1", [
                (global_mutex "HCLK", "USE"),
                (row_mutex "BUFH_TEST", format!("USED_HCLK{i}")),
                (pip
                    (related_pin BelRelation::Rclk, format!("HCLK{i}_L_I")),
                    (related_pin BelRelation::Rclk, "BUFH_TEST_L_PRE")),
                (pip
                    (related_pin BelRelation::Rclk, format!("HCLK{i}_R_I")),
                    (related_pin BelRelation::Rclk, "BUFH_TEST_R_PRE"))
            ], [
                (pip (pin format!("HCLK{i}_I")), (pin format!("HCLK{i}_O")))
            ]);
        }
        for i in 0..6 {
            fuzz_one!(ctx, format!("BUF.RCLK{i}"), "1", [
                (global_mutex "RCLK", "USE"),
                (row_mutex "BUFH_TEST", format!("USED_RCLK{i}")),
                (pip
                    (related_pin BelRelation::Rclk, format!("RCLK{i}_L_I")),
                    (related_pin BelRelation::Rclk, "BUFH_TEST_L_PRE")),
                (pip
                    (related_pin BelRelation::Rclk, format!("RCLK{i}_R_I")),
                    (related_pin BelRelation::Rclk, "BUFH_TEST_R_PRE"))
            ], [
                (pip (pin format!("RCLK{i}_I")), (pin format!("RCLK{i}_O")))
            ]);
            for pin in [
                "VRCLK0", "VRCLK1", "VRCLK0_S", "VRCLK1_S", "VRCLK0_N", "VRCLK1_N",
            ] {
                fuzz_one!(ctx, format!("MUX.RCLK{i}"), pin, [
                    (global_mutex "RCLK", "USE"),
                    (row_mutex_site "RCLK_DRIVE"),
                    (mutex format!("MUX.RCLK{i}"), pin),
                    (row_mutex "BUFH_TEST", format!("USED_RCLK{i}")),
                    (pip
                        (related_pin BelRelation::Rclk, format!("RCLK{i}_L_I")),
                        (related_pin BelRelation::Rclk, "BUFH_TEST_L_PRE")),
                    (pip
                        (related_pin BelRelation::Rclk, format!("RCLK{i}_R_I")),
                        (related_pin BelRelation::Rclk, "BUFH_TEST_R_PRE"))
                ], [
                    (pip (pin pin), (pin format!("RCLK{i}_I")))
                ]);
            }
        }
        for i in 0..4 {
            fuzz_one!(ctx, format!("BUF.PERF{i}"), "1", [], [
                (pip (pin format!("PERF{i}")), (pin format!("PERF{i}_BUF")))
            ]);
        }
        for (i, pre) in [
            "IOCLK0_PRE",
            "IOCLK1_PRE",
            "IOCLK2_PRE",
            "IOCLK3_PRE",
            "IOCLK0_PRE_S",
            "IOCLK3_PRE_S",
            "IOCLK0_PRE_N",
            "IOCLK3_PRE_N",
        ]
        .into_iter()
        .enumerate()
        {
            fuzz_one!(ctx, format!("DELAY.IOCLK{i}"), "0", [
                (mutex format!("DELAY.IOCLK{i}"), "0")
            ], [
                (pip (pin pre), (pin format!("IOCLK{i}")))
            ]);
            fuzz_one!(ctx, format!("DELAY.IOCLK{i}"), "1", [
                (mutex format!("DELAY.IOCLK{i}"), "1")
            ], [
                (pip (pin format!("IOCLK{i}_DLY")), (pin format!("IOCLK{i}"))),
                (pip (pin pre), (pin format!("IOCLK{i}_DLY")))
            ]);
        }
        for i in 0..2 {
            let bel_bufo = BelId::from_idx(6 + i);
            fuzz_one!(ctx, format!("BUF.VOCLK{i}"), "1", [], [
                (pip (bel_pin bel_bufo, "I_PRE2"), (bel_pin bel_bufo, "I_PRE"))
            ]);
        }
    }

    {
        let ctx = FuzzCtx::new(session, backend, "PMVIOB", "PMVIOB", TileBits::MainAuto);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "PMVIOB")]);
        fuzz_enum!(ctx, "HSLEW4_IN", ["FALSE", "TRUE"], [(mode "PMVIOB")]);
        fuzz_enum!(ctx, "PSLEW4_IN", ["FALSE", "TRUE"], [(mode "PMVIOB")]);
        fuzz_enum!(ctx, "HYS_IN", ["FALSE", "TRUE"], [(mode "PMVIOB")]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    {
        let tile = "HCLK";
        let bel = "HCLK";
        for i in 0..12 {
            let (_, _, diff) = Diff::split(
                ctx.state
                    .peek_diff(tile, bel, "MUX.LCLK0_D", format!("HCLK{i}"))
                    .clone(),
                ctx.state
                    .peek_diff(tile, bel, "MUX.LCLK0_U", format!("HCLK{i}"))
                    .clone(),
            );
            ctx.tiledb
                .insert(tile, bel, format!("ENABLE.HCLK{i}"), xlat_bit(diff));
        }
        for i in 0..6 {
            let (_, _, diff) = Diff::split(
                ctx.state
                    .peek_diff(tile, bel, "MUX.LCLK0_D", format!("RCLK{i}"))
                    .clone(),
                ctx.state
                    .peek_diff(tile, bel, "MUX.LCLK0_U", format!("RCLK{i}"))
                    .clone(),
            );
            ctx.tiledb
                .insert(tile, bel, format!("ENABLE.RCLK{i}"), xlat_bit(diff));
        }
        for i in 0..8 {
            for ud in ['U', 'D'] {
                let mux = &format!("MUX.LCLK{i}_{ud}");
                let mut diffs = vec![("NONE".to_string(), Diff::default())];
                for i in 0..12 {
                    let val = format!("HCLK{i}");
                    let mut diff = ctx.state.get_diff(tile, bel, mux, &val);
                    diff.apply_bit_diff(
                        ctx.tiledb.item(tile, bel, &format!("ENABLE.HCLK{i}")),
                        true,
                        false,
                    );
                    diffs.push((val, diff));
                }
                for i in 0..6 {
                    let val = format!("RCLK{i}");
                    let mut diff = ctx.state.get_diff(tile, bel, mux, &val);
                    diff.apply_bit_diff(
                        ctx.tiledb.item(tile, bel, &format!("ENABLE.RCLK{i}")),
                        true,
                        false,
                    );
                    diffs.push((val, diff));
                }
                ctx.tiledb
                    .insert(tile, bel, mux, xlat_enum_ocd(diffs, OcdMode::Mux));
            }
        }
    }
    for (tile, base) in [("CMT_BUFG_BOT", 0), ("CMT_BUFG_TOP", 16)] {
        for i in 0..16 {
            let bel = &format!("BUFGCTRL{}", base + i);
            for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
                ctx.collect_inv(tile, bel, pin);
            }
            for attr in ["PRESELECT_I0", "PRESELECT_I1", "CREATE_EDGE"] {
                ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
            }
            ctx.collect_enum_bool(tile, bel, "INIT_OUT", "0", "1");

            // sigh. fucking. ise.
            let mut item = xlat_bit(ctx.state.peek_diff(tile, bel, "MUX.I0", "CASCI").clone());
            assert_eq!(item.bits.len(), 1);
            item.bits[0].bit += 1;
            ctx.tiledb.insert(tile, bel, "TEST_I1", item);
            let mut item = xlat_bit(ctx.state.peek_diff(tile, bel, "MUX.I1", "CASCI").clone());
            assert_eq!(item.bits.len(), 1);
            item.bits[0].bit += 1;
            ctx.tiledb.insert(tile, bel, "TEST_I0", item);

            for attr in ["MUX.I0", "MUX.I1"] {
                ctx.collect_enum_default_ocd(
                    tile,
                    bel,
                    attr,
                    &[
                        "CASCI", "GIO0", "GIO1", "GIO2", "GIO3", "GIO4", "GIO5", "GIO6", "GIO7",
                        "FB_PREV", "FB_NEXT", "CKINT",
                    ],
                    "NONE",
                    OcdMode::Mux,
                );
            }
            ctx.collect_bit(tile, bel, "ENABLE.FB", "1");
            ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
            ctx.state.get_diff(tile, bel, "TEST_I0", "1").assert_empty();
            ctx.state.get_diff(tile, bel, "TEST_I1", "1").assert_empty();
            ctx.state
                .get_diff(tile, bel, "ENABLE.GCLK", "1")
                .assert_empty();
        }
    }
    {
        let tile = "PMVIOB";
        let bel = "PMVIOB";
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        ctx.collect_enum_bool(tile, bel, "HYS_IN", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "HSLEW4_IN", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "PSLEW4_IN", "FALSE", "TRUE");
    }
    for i in 0..4 {
        let tile = "HCLK_IOI";
        let bel = &format!("BUFIODQS{i}");
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        ctx.collect_enum_bool(tile, bel, "DQSMASK_ENABLE", "FALSE", "TRUE");
        ctx.collect_enum(
            tile,
            bel,
            "MUX.I",
            &["CCIO".to_string(), format!("PERF{}", i ^ 1)],
        );
        ctx.collect_bit(tile, bel, "ENABLE", "1");
    }
    for i in 0..2 {
        let tile = "HCLK_IOI";
        let bel = &format!("BUFR{i}");
        ctx.collect_bit(tile, bel, "ENABLE", "1");
        ctx.collect_enum(
            tile,
            bel,
            "BUFR_DIVIDE",
            &["BYPASS", "1", "2", "3", "4", "5", "6", "7", "8"],
        );
        ctx.collect_enum_default(
            tile,
            bel,
            "MUX.I",
            &[
                "BUFIO0_I", "BUFIO1_I", "BUFIO2_I", "BUFIO3_I", "MGT0", "MGT1", "MGT2", "MGT3",
                "MGT4", "MGT5", "MGT6", "MGT7", "MGT8", "MGT9", "CKINT0", "CKINT1",
            ],
            "NONE",
        );
    }
    for i in 0..2 {
        let tile = "HCLK_IOI";
        let bel = &format!("BUFO{i}");
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        ctx.collect_enum(
            tile,
            bel,
            "MUX.I",
            &[
                format!("VOCLK{i}"),
                format!("VOCLK{i}_S"),
                format!("VOCLK{i}_N"),
            ],
        )
    }
    {
        let tile = "HCLK_IOI";
        let bel = "IDELAYCTRL";
        let vals: [_; 12] = core::array::from_fn(|i| format!("HCLK{i}"));
        ctx.collect_enum(tile, bel, "MUX.REFCLK", &vals);
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        ctx.collect_enum_bool(tile, bel, "HIGH_PERFORMANCE_MODE", "FALSE", "TRUE");
        ctx.collect_enum_default(tile, bel, "MODE", &["DEFAULT", "FULL_0", "FULL_1"], "NONE");
        ctx.collect_enum(tile, bel, "RESET_STYLE", &["V4", "V5"]);
    }
    {
        let tile = "HCLK_IOI";
        let bel = "HCLK_IOI";
        for i in 0..12 {
            ctx.collect_bit(tile, bel, &format!("BUF.HCLK{i}"), "1");
        }
        for i in 0..6 {
            ctx.collect_bit(tile, bel, &format!("BUF.RCLK{i}"), "1");
            ctx.collect_enum_default_ocd(
                tile,
                bel,
                &format!("MUX.RCLK{i}"),
                &[
                    "VRCLK0", "VRCLK1", "VRCLK0_S", "VRCLK1_S", "VRCLK0_N", "VRCLK1_N",
                ],
                "NONE",
                OcdMode::Mux,
            );
        }
        for i in 0..4 {
            ctx.collect_bit(tile, bel, &format!("BUF.PERF{i}"), "1");
        }
        for i in 0..2 {
            ctx.collect_bit(tile, bel, &format!("BUF.VOCLK{i}"), "1");
        }
        for i in 0..4 {
            ctx.collect_enum_bool(tile, bel, &format!("DELAY.IOCLK{i}"), "0", "1");
        }
        for i in 4..8 {
            let diff_buf = ctx
                .state
                .get_diff(tile, bel, format!("DELAY.IOCLK{i}"), "0");
            let diff_delay = ctx
                .state
                .get_diff(tile, bel, format!("DELAY.IOCLK{i}"), "1")
                .combine(&!&diff_buf);
            ctx.tiledb
                .insert(tile, bel, format!("BUF.IOCLK{i}"), xlat_bit(diff_buf));
            ctx.tiledb
                .insert(tile, bel, format!("DELAY.IOCLK{i}"), xlat_bit(diff_delay));
        }
    }
    {
        let tile = "CMT";
        let bel = "CMT";
        for i in 0..32 {
            ctx.collect_bit(tile, bel, &format!("ENABLE.GCLK{i}"), "1");
        }
        for i in 0..8 {
            ctx.collect_bit(tile, bel, &format!("ENABLE.GIO{i}"), "1");
        }
    }
}
