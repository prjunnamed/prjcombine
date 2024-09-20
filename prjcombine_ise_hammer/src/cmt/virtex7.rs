use prjcombine_hammer::Session;

use crate::{
    backend::IseBackend, diff::CollectorCtx, fgen::TileBits, fuzz::FuzzCtx, fuzz_enum,
    fuzz_multi_attr_bin, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
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
    // TODO: pll
    // TODO: mmcm
    // TODO: phy
    // TODO: misc pips
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
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
}
