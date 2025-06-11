use prjcombine_interconnect::{
    db::TileCellId,
    grid::{DieId, LayerId},
};
use prjcombine_re_fpga_hammer::{Diff, xlat_enum};
use prjcombine_re_hammer::Session;
use prjcombine_types::bsdata::{TileBit, TileItem};
use prjcombine_xc2000::bels::xc2000 as bels;
use unnamed_entity::EntityId;

use crate::{
    backend::{Key, Value, XactBackend},
    collector::CollectorCtx,
    fbuild::FuzzCtx,
    props::BaseBelNoConfig,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    let grid = backend.edev.chip;
    for (_, tile, node) in &backend.egrid.db.tile_classes {
        if !tile.starts_with("CLB") {
            continue;
        }
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        for slot in node.bels.ids() {
            let slot_name = backend.egrid.db.bel_slots.key(slot).as_str();
            if !slot_name.starts_with("IO") {
                continue;
            }
            let mut bctx = ctx.bel(slot);
            bctx.mode("IO").cfg("IN", "I").test_cfg("IN", "IQ");
            bctx.mode("IO")
                .cfg("IN", "I")
                .test_enum("IN", &["FF", "LATCH"]);
            bctx.mode("IO")
                .cfg("IN", "I")
                .mutex("TRI", "GND")
                .test_manual("OUT", "OQ")
                .cfg_diff("OUT", "O", "OQ")
                .commit();
            bctx.mode("IO")
                .cfg("IN", "I")
                .mutex("TRI", "GND")
                .cfg("OUT", "O")
                .test_cfg("OUT", "NOT");
            bctx.mode("IO")
                .cfg("IN", "I")
                .mutex("TRI", "GND")
                .cfg("OUT", "O")
                .test_cfg("OUT", "FAST");
            bctx.mode("IO")
                .cfg("IN", "I")
                .mutex("TRI", "T")
                .cfg("OUT", "O")
                .cfg("TRI", "T")
                .test_cfg("TRI", "NOT");
        }
        if tile.starts_with("CLB.BR") {
            ctx.test_global("DONE", "DONEPAD", &["PULLUP", "NOPULLUP"]);
            ctx.test_global("MISC", "REPROGRAM", &["ENABLE", "DISABLE"]);
            ctx.test_global("MISC", "DONETIME", &["BEFORE", "AFTER"]);
            ctx.test_global("MISC", "RESETTIME", &["BEFORE", "AFTER"]);
            let nloc = (
                DieId::from_idx(0),
                grid.col_e(),
                grid.row_s(),
                LayerId::from_idx(0),
            );
            let wt = (
                TileCellId::from_idx(0),
                backend.egrid.db.get_wire("IMUX.BUFG"),
            );
            let wf = (
                TileCellId::from_idx(0),
                backend.egrid.db.get_wire("OUT.OSC"),
            );
            let crd = backend.ngrid.int_pip(nloc, wt, wf);
            let rwt = backend.egrid.resolve_tile_wire_nobuf(nloc, wt).unwrap();
            let rwf = backend.egrid.resolve_tile_wire_nobuf(nloc, wf).unwrap();
            for val in ["ENABLE", "DIV2"] {
                ctx.build()
                    .raw(Key::NodeMutex(rwt), "OSC_SPECIAL")
                    .raw(Key::NodeMutex(rwf), "OSC_SPECIAL")
                    .test_manual("OSC", "MODE", val)
                    .global_diff("XTALOSC", "DISABLE", val)
                    .raw_diff(Key::Pip(crd), None, Value::FromPin("OSC", "O".into()))
                    .raw_diff(
                        Key::BlockPin("ACLK", "I".into()),
                        None,
                        Value::FromPin("OSC", "O".into()),
                    )
                    .prop(BaseBelNoConfig::new(bels::IO_S1, "IN".into(), "I".into()))
                    .prop(BaseBelNoConfig::new(bels::IO_E0, "IN".into(), "I".into()))
                    .commit();
            }
        }
        if tile.starts_with("CLB.BL") {
            ctx.test_global("MISC", "READ", &["COMMAND", "ONCE", "DISABLE"]);
        }
        if tile.starts_with("CLB.TL") {
            ctx.test_global("MISC", "INPUT", &["TTL", "CMOS"]);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for (_, tile, node) in &ctx.edev.egrid.db.tile_classes {
        if tile == "LLV.RS" {
            let bel = "MISC";
            ctx.tiledb.insert(
                tile,
                bel,
                "TLC",
                TileItem::from_bit(TileBit::new(0, 0, 0), true),
            );
        } else if tile == "LLV.R" {
            let bel = "MISC";
            ctx.tiledb.insert(
                tile,
                bel,
                "TLC",
                TileItem::from_bit(TileBit::new(1, 0, 0), true),
            );
        }
        if !tile.starts_with("CLB") {
            continue;
        }
        if !ctx.has_tile(tile) {
            continue;
        }
        for slot in node.bels.ids() {
            let bel = ctx.edev.egrid.db.bel_slots.key(slot).as_str();
            if !bel.starts_with("IO") {
                continue;
            }
            ctx.state.get_diff(tile, bel, "IN", "IQ").assert_empty();
            let item = ctx.extract_bit(tile, bel, "TRI", "NOT");
            ctx.tiledb.insert(tile, bel, "INV.T", item);
            let item = ctx.extract_bit(tile, bel, "OUT", "NOT");
            ctx.tiledb.insert(tile, bel, "INV.O", item);
            let item = xlat_enum(vec![
                ("SLOW", Diff::default()),
                ("FAST", ctx.state.get_diff(tile, bel, "OUT", "FAST")),
            ]);
            ctx.tiledb.insert(tile, bel, "SLEW", item);
            let item = xlat_enum(vec![
                ("O", Diff::default()),
                ("OFF", ctx.state.get_diff(tile, bel, "OUT", "OQ")),
            ]);
            ctx.tiledb.insert(tile, bel, "MUX.O", item);
            let item = ctx.extract_enum_bool(tile, bel, "IN", "FF", "LATCH");
            ctx.tiledb.insert(tile, bel, "IFF_LATCH", item);
        }
        for (prefix, bel, what, frame, bit) in [
            ("CLB.BR", "IO_S0", "I", 28, 1),
            ("CLB.BR", "IO_S0", "IFF", 25, 0),
            ("CLB.BR", "IO_S1", "I", 20, 1),
            ("CLB.BR", "IO_S1", "IFF", 24, 1),
            ("CLB.B.", "IO_S0", "I", 14, 1),
            ("CLB.B.", "IO_S0", "IFF", 11, 0),
            ("CLB.B.", "IO_S1", "I", 6, 1),
            ("CLB.B.", "IO_S1", "IFF", 10, 1),
            ("CLB.BL", "IO_S0", "I", 14, 1),
            ("CLB.BL", "IO_S0", "IFF", 11, 0),
            ("CLB.BL", "IO_S1", "I", 6, 1),
            ("CLB.BL", "IO_S1", "IFF", 10, 1),
            ("CLB.TR", "IO_N0", "I", 28, 8),
            ("CLB.TR", "IO_N0", "IFF", 25, 9),
            ("CLB.TR", "IO_N1", "I", 20, 8),
            ("CLB.TR", "IO_N1", "IFF", 24, 8),
            ("CLB.T.", "IO_N0", "I", 14, 8),
            ("CLB.T.", "IO_N0", "IFF", 11, 9),
            ("CLB.T.", "IO_N1", "I", 6, 8),
            ("CLB.T.", "IO_N1", "IFF", 10, 8),
            ("CLB.TS.", "IO_N0", "I", 14, 8),
            ("CLB.TS.", "IO_N0", "IFF", 11, 9),
            ("CLB.TS.", "IO_N1", "I", 6, 8),
            ("CLB.TS.", "IO_N1", "IFF", 10, 8),
            ("CLB.TL", "IO_N0", "I", 14, 8),
            ("CLB.TL", "IO_N0", "IFF", 11, 9),
            ("CLB.TL", "IO_N1", "I", 6, 8),
            ("CLB.TL", "IO_N1", "IFF", 10, 8),
            ("CLB.BR", "IO_E0", "I", 13, 7),
            ("CLB.BR", "IO_E0", "IFF", 9, 10),
            ("CLB.BR", "IO_E1", "I", 5, 6),
            ("CLB.BR", "IO_E1", "IFF", 6, 6),
            ("CLB.R.", "IO_E0", "I", 13, 2),
            ("CLB.R.", "IO_E0", "IFF", 9, 5),
            ("CLB.R.", "IO_E1", "I", 5, 1),
            ("CLB.R.", "IO_E1", "IFF", 6, 1),
            ("CLB.TR", "IO_E0", "I", 13, 2),
            ("CLB.TR", "IO_E0", "IFF", 8, 4),
            ("CLB.TR", "IO_E1", "I", 5, 1),
            ("CLB.TR", "IO_E1", "IFF", 6, 1),
            ("CLB.BL", "IO_W0", "I", 2, 11),
            ("CLB.BL", "IO_W0", "IFF", 8, 11),
            ("CLB.BL", "IO_W1", "I", 9, 8),
            ("CLB.BL", "IO_W1", "IFF", 22, 6),
            ("CLB.L.", "IO_W0", "I", 2, 6),
            ("CLB.L.", "IO_W0", "IFF", 8, 6),
            ("CLB.L.", "IO_W1", "I", 9, 3),
            ("CLB.L.", "IO_W1", "IFF", 22, 1),
            ("CLB.TL", "IO_W0", "I", 10, 7),
            ("CLB.TL", "IO_W0", "IFF", 12, 5),
            ("CLB.TL", "IO_W1", "I", 9, 3),
            ("CLB.TL", "IO_W1", "IFF", 22, 1),
        ] {
            if tile.starts_with(prefix) {
                ctx.tiledb.insert(
                    tile,
                    bel,
                    format!("READBACK_{what}"),
                    TileItem::from_bit(TileBit::new(0, frame, bit), true),
                );
            }
        }
        if tile.starts_with("CLB.BR") {
            let bel = "MISC";
            ctx.collect_enum_bool_wide_mixed(tile, bel, "REPROGRAM", "DISABLE", "ENABLE");
            ctx.collect_enum(tile, bel, "DONETIME", &["BEFORE", "AFTER"]);
            ctx.collect_enum(tile, bel, "RESETTIME", &["BEFORE", "AFTER"]);
            ctx.tiledb.insert(
                tile,
                bel,
                "TLC",
                TileItem::from_bit(TileBit::new(0, 1, 0), true),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "SLOWOSC_HALT",
                TileItem::from_bit(TileBit::new(0, 5, 0), false),
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
            let bel = "OSC";
            let mut diffs = vec![("DISABLE", Diff::default())];
            for val in ["ENABLE", "DIV2"] {
                let mut diff = ctx.state.get_diff(tile, bel, "MODE", val);
                diff.discard_bits(ctx.tiledb.item(tile, "INT", "MUX.IMUX.BUFG"));
                diff.apply_bit_diff(ctx.tiledb.item(tile, "IO_S1", "PULLUP"), false, true);
                diff.apply_bit_diff(ctx.tiledb.item(tile, "IO_E0", "PULLUP"), false, true);
                diffs.push((val, diff));
            }
            ctx.tiledb.insert(tile, bel, "MODE", xlat_enum(diffs));
        }
        if tile.starts_with("CLB.BL") {
            let bel = "MISC";
            ctx.collect_enum(tile, bel, "READ", &["COMMAND", "ONCE", "DISABLE"]);
        }
        if tile.starts_with("CLB.TL") {
            let bel = "MISC";
            ctx.collect_enum(tile, bel, "INPUT", &["TTL", "CMOS"]);
        }
        if tile.starts_with("CLB.TR") {
            let bel = "MISC";
            ctx.tiledb.insert(
                tile,
                bel,
                "TAC",
                TileItem::from_bit(TileBit::new(0, 0, 5), true),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "POR",
                TileItem::from_bit(TileBit::new(0, 11, 9), true),
            );
        }
    }
}
