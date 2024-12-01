use prjcombine_collector::xlat_enum;
use prjcombine_hammer::Session;
use prjcombine_types::tiledb::{TileBit, TileItem};

use crate::{backend::XactBackend, collector::CollectorCtx, fbuild::FuzzCtx};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    for (_, tile, node) in &backend.egrid.db.nodes {
        if !tile.starts_with("CLB") {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, tile);
        for bel in node.bels.keys() {
            if !bel.contains("IOB") {
                continue;
            }
            let mut bctx = ctx.bel(bel);
            bctx.mode("IO").test_enum("I", &["PAD", "Q"]);
            bctx.mode("IO").test_enum("BUF", &["ON"]);
        }
        if tile == "CLB.BR" {
            ctx.test_global("DONE", "DONEPAD", &["PULLUP", "NOPULLUP"]);
            ctx.test_global("MISC", "REPROGRAM", &["ENABLE", "DISABLE"]);
        }
        if tile == "CLB.BL" {
            ctx.test_global("MISC", "READ", &["COMMAND", "ONCE", "DISABLE"]);
        }
        if tile == "CLB.TL" {
            ctx.test_global("MISC", "INPUT", &["TTL", "CMOS"]);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for (_, tile, node) in &ctx.edev.egrid.db.nodes {
        if !tile.starts_with("CLB") {
            continue;
        }
        for bel in node.bels.keys() {
            if !bel.contains("IOB") {
                continue;
            }
            ctx.collect_enum(tile, bel, "I", &["PAD", "Q"]);
        }
        if tile == "CLB.BR" {
            let bel = "MISC";
            ctx.collect_enum_bool(tile, bel, "REPROGRAM", "DISABLE", "ENABLE");
            ctx.tiledb.insert(
                tile,
                bel,
                "TLC",
                TileItem::from_bit(TileBit::new(0, 0, 2), true),
            );
            let bel = "DONE";
            let item = xlat_enum(vec![
                ("PULLUP", ctx.state.get_diff(tile, bel, "DONEPAD", "PULLUP")),
                (
                    "PULLNONE",
                    ctx.state.get_diff(tile, bel, "DONEPAD", "NOPULLUP"),
                ),
            ]);
            ctx.tiledb.insert(tile, bel, "PULL", item);
        }
        if tile == "CLB.BL" {
            let bel = "MISC";
            ctx.collect_enum(tile, bel, "READ", &["COMMAND", "ONCE", "DISABLE"]);
        }
        if tile == "CLB.TL" {
            let bel = "MISC";
            ctx.collect_enum(tile, bel, "INPUT", &["TTL", "CMOS"]);
        }
        if tile == "CLB.TR" {
            let bel = "MISC";
            ctx.tiledb.insert(
                tile,
                bel,
                "TAC",
                TileItem::from_bit(TileBit::new(0, 8, 8), true),
            );
        }
        if tile == "CLB.MR" {
            let bel = "MISC";
            ctx.tiledb.insert(
                tile,
                bel,
                "TLC",
                TileItem::from_bit(TileBit::new(0, 0, 1), true),
            );
        }
    }
}
