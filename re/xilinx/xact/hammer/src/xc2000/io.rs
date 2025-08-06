use prjcombine_re_fpga_hammer::xlat_enum;
use prjcombine_re_hammer::Session;
use prjcombine_types::bsdata::{TileBit, TileItem};

use crate::{backend::XactBackend, collector::CollectorCtx, fbuild::FuzzCtx};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    for (_, tcname, tcls) in &backend.egrid.db.tile_classes {
        if !tcname.starts_with("CLB") {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, tcname);
        for slot in tcls.bels.ids() {
            let slot_name = backend.egrid.db.bel_slots.key(slot);
            if !slot_name.starts_with("IO") {
                continue;
            }
            let mut bctx = ctx.bel(slot);
            bctx.mode("IO").test_enum("I", &["PAD", "Q"]);
            bctx.mode("IO").test_enum("BUF", &["ON"]);
        }
        if tcname == "CLB.BR" {
            ctx.test_global("DONE", "DONEPAD", &["PULLUP", "NOPULLUP"]);
            ctx.test_global("MISC", "REPROGRAM", &["ENABLE", "DISABLE"]);
        }
        if tcname == "CLB.BL" {
            ctx.test_global("MISC", "READ", &["COMMAND", "ONCE", "DISABLE"]);
        }
        if tcname == "CLB.TL" {
            ctx.test_global("MISC", "INPUT", &["TTL", "CMOS"]);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for (_, tcname, tcls) in &ctx.edev.egrid.db.tile_classes {
        if !tcname.starts_with("CLB") {
            continue;
        }
        for slot in tcls.bels.ids() {
            let bel = ctx.edev.egrid.db.bel_slots.key(slot);
            if !bel.starts_with("IO") {
                continue;
            }
            ctx.collect_enum(tcname, bel, "I", &["PAD", "Q"]);
        }
        if tcname == "CLB.BR" {
            let bel = "MISC";
            ctx.collect_enum_bool(tcname, bel, "REPROGRAM", "DISABLE", "ENABLE");
            ctx.tiledb.insert(
                tcname,
                bel,
                "TLC",
                TileItem::from_bit(TileBit::new(0, 0, 2), true),
            );
            let bel = "DONE";
            let item = xlat_enum(vec![
                ("PULLUP", ctx.state.get_diff(tcname, bel, "DONEPAD", "PULLUP")),
                (
                    "PULLNONE",
                    ctx.state.get_diff(tcname, bel, "DONEPAD", "NOPULLUP"),
                ),
            ]);
            ctx.tiledb.insert(tcname, bel, "PULL", item);
        }
        if tcname == "CLB.BL" {
            let bel = "MISC";
            ctx.collect_enum(tcname, bel, "READ", &["COMMAND", "ONCE", "DISABLE"]);
        }
        if tcname == "CLB.TL" {
            let bel = "MISC";
            ctx.collect_enum(tcname, bel, "INPUT", &["TTL", "CMOS"]);
        }
        if tcname == "CLB.TR" {
            let bel = "MISC";
            ctx.tiledb.insert(
                tcname,
                bel,
                "TAC",
                TileItem::from_bit(TileBit::new(0, 8, 8), true),
            );
        }
        if tcname == "CLB.MR" {
            let bel = "MISC";
            ctx.tiledb.insert(
                tcname,
                bel,
                "TLC",
                TileItem::from_bit(TileBit::new(0, 0, 1), true),
            );
        }
    }
}
