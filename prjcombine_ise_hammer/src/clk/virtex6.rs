use prjcombine_hammer::Session;

use crate::{
    backend::IseBackend,
    diff::{xlat_bit, xlat_enum_ocd, CollectorCtx, Diff, OcdMode},
    fgen::{BelRelation, TileBits},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_one,
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

    let tile = "PMVIOB";
    let bel = "PMVIOB";
    ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
    ctx.collect_enum_bool(tile, bel, "HYS_IN", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "HSLEW4_IN", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, bel, "PSLEW4_IN", "FALSE", "TRUE");
}
