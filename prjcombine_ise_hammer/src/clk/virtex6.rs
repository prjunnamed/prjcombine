use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::{xlat_bit, xlat_enum_ocd, CollectorCtx, Diff, OcdMode},
    fgen::{BelRelation, TileBits},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    {
        let ctx = FuzzCtx::new(session, backend, "HCLK", "HCLK", TileBits::Hclk);
        for i in 0..8 {
            for ud in ['U', 'D'] {
                for j in 0..12 {
                    fuzz_one!(ctx, format!("MUX.OUT_{ud}{i}"), format!("HCLK{j}"), [
                        (global_mutex "BUFH_TEST", format!("USED_HCLK{j}")),
                        (pip
                            (related_pin BelRelation::Rclk, format!("HCLK{j}_L_I")),
                            (related_pin BelRelation::Rclk, "BUFH_TEST_L_PRE")),
                        (pip
                            (related_pin BelRelation::Rclk, format!("HCLK{j}_R_I")),
                            (related_pin BelRelation::Rclk, "BUFH_TEST_R_PRE")),
                        (mutex format!("MUX.OUT_{ud}{i}"), format!("HCLK{j}")),
                        (mutex format!("HCLK{j}"), format!("MUX.OUT_{ud}{i}"))
                    ], [
                        (pip (pin format!("HCLK{j}")), (pin format!("OUT_{ud}{i}")))
                    ]);
                }
                for j in 0..6 {
                    fuzz_one!(ctx, format!("MUX.OUT_{ud}{i}"), format!("RCLK{j}"), [
                        (global_mutex "BUFH_TEST", format!("USED_RCLK{j}")),
                        (pip
                            (related_pin BelRelation::Rclk, format!("RCLK{j}_L_I")),
                            (related_pin BelRelation::Rclk, "BUFH_TEST_L_PRE")),
                        (pip
                            (related_pin BelRelation::Rclk, format!("RCLK{j}_R_I")),
                            (related_pin BelRelation::Rclk, "BUFH_TEST_R_PRE")),
                        (mutex format!("MUX.OUT_{ud}{i}"), format!("RCLK{j}")),
                        (mutex format!("RCLK{j}"), format!("MUX.OUT_{ud}{i}"))
                    ], [
                        (pip (pin format!("RCLK{j}")), (pin format!("OUT_{ud}{i}")))
                    ]);
                }
            }
        }
    }

    for (tile, bits, base) in [
        ("CMT_BUFG_BOT", TileBits::Main(2, 2), 0),
        ("CMT_BUFG_TOP", TileBits::Main(0, 2), 16),
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
                    .peek_diff(tile, bel, "MUX.OUT_D0", format!("HCLK{i}"))
                    .clone(),
                ctx.state
                    .peek_diff(tile, bel, "MUX.OUT_U0", format!("HCLK{i}"))
                    .clone(),
            );
            ctx.tiledb
                .insert(tile, bel, format!("ENABLE.HCLK{i}"), xlat_bit(diff));
        }
        for i in 0..6 {
            let (_, _, diff) = Diff::split(
                ctx.state
                    .peek_diff(tile, bel, "MUX.OUT_D0", format!("RCLK{i}"))
                    .clone(),
                ctx.state
                    .peek_diff(tile, bel, "MUX.OUT_U0", format!("RCLK{i}"))
                    .clone(),
            );
            ctx.tiledb
                .insert(tile, bel, format!("ENABLE.RCLK{i}"), xlat_bit(diff));
        }
        for i in 0..8 {
            for ud in ['U', 'D'] {
                let mux = &format!("MUX.OUT_{ud}{i}");
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
        }
    }

    let tile = "PMVIOB";
    let bel = "PMVIOB";
    ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
    ctx.collect_enum_bool(tile, bel, "HYS_IN", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "HSLEW4_IN", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "PSLEW4_IN", "FALSE", "TRUE");
}
