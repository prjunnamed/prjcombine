use prjcombine_re_hammer::Session;

use crate::{backend::XactBackend, collector::CollectorCtx, fbuild::FuzzCtx};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    for tile in ["IO.L", "IO.R", "IO.B", "IO.T"] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        for i in 0..4 {
            let mut bctx = ctx.bel(format!("IOB{i}"));
            bctx.mode("IO")
                .cfg("IN", "I")
                .test_enum("PAD", &["PULLUP", "PULLDOWN"]);
            bctx.mode("IO").cfg("IN", "I").test_cfg("PAD", "FAST");
            bctx.mode("IO").test_cfg("IN", "I");
            bctx.mode("IO").cfg("IN", "I").test_cfg("IN", "NOT");
            bctx.mode("IO").cfg("IN", "I").test_cfg("IN", "DELAY");
            bctx.mode("IO").cfg("IN", "I").test_cfg("OUT", "O");
            bctx.mode("IO")
                .cfg("IN", "I")
                .cfg("OUT", "O")
                .test_cfg("TRI", "T");
            bctx.mode("IO")
                .cfg("IN", "I")
                .cfg("OUT", "O")
                .test_cfg("TRI", "NOT");
            if tile == "IO.L" || tile == "IO.R" {
                bctx.mode("IO")
                    .cfg("IN", "I")
                    .cfg("OUT", "O")
                    .test_cfg("OUT", "NOT");
            }
        }
        if tile == "IO.B" {
            let mut bctx = ctx.bel("SCANTEST");
            bctx.mode("SCANTEST")
                .test_enum("OUT", &["XI", "YI", "ZI", "VI", "SCANPASS"]);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tile in ["IO.L", "IO.R", "IO.B", "IO.T"] {
        for i in 0..4 {
            let bel = &format!("IOB{i}");
            let item = ctx.extract_enum_default(tile, bel, "PAD", &["PULLUP", "PULLDOWN"], "NONE");
            ctx.tiledb.insert(tile, bel, "PULL", item);
            let item = ctx.extract_enum_default(tile, bel, "PAD", &["FAST"], "SLOW");
            ctx.tiledb.insert(tile, bel, "SLEW", item);
            let item = ctx.extract_enum_default(tile, bel, "IN", &["DELAY"], "NODELAY");
            ctx.tiledb.insert(tile, bel, "DELAYMUX", item);
            let item = ctx.extract_bit(tile, bel, "IN", "NOT");
            ctx.tiledb.insert(tile, bel, "INV.I", item);
            let item = ctx.extract_bit(tile, bel, "TRI", "NOT");
            ctx.tiledb.insert(tile, bel, "INV.T", item);
            if tile == "IO.L" || tile == "IO.R" {
                let item = ctx.extract_bit(tile, bel, "OUT", "NOT");
                ctx.tiledb.insert(tile, bel, "INV.O", item);
            }
            let mut diff = ctx.state.get_diff(tile, bel, "IN", "I");
            diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "PULL"), "NONE", "PULLUP");
            diff.assert_empty();
            let mut diff = ctx.state.get_diff(tile, bel, "OUT", "O");
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.T"), false, true);
            diff.assert_empty();
            ctx.state.get_diff(tile, bel, "TRI", "T").assert_empty();
        }
        if tile == "IO.B" {
            ctx.collect_enum(
                tile,
                "SCANTEST",
                "OUT",
                &["SCANPASS", "XI", "YI", "ZI", "VI"],
            );
        }
    }
}
