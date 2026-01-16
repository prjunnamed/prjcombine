use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelKind, TileWireCoord},
    grid::{CellCoord, DieId},
};
use prjcombine_re_fpga_hammer::{Diff, xlat_enum};
use prjcombine_re_hammer::Session;
use prjcombine_xc2000::xc2000::{
    bslots, tslots,
    xc3000::{bcls, wires},
};

use crate::{
    backend::{Key, Value, XactBackend},
    collector::CollectorCtx,
    fbuild::FuzzCtx,
    props::BaseBelNoConfig,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    let grid = backend.edev.chip;
    for (_, tcname, tcls) in &backend.edev.db.tile_classes {
        if tcls.slot != tslots::MAIN {
            continue;
        }
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcname) else {
            continue;
        };
        for slot in tcls.bels.ids() {
            if backend.edev.db.bel_slots[slot].kind != BelKind::Class(bcls::IO) {
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
        if tcname.starts_with("CLB_SE") {
            ctx.test_global("DONE", "DONEPAD", &["PULLUP", "NOPULLUP"]);
            ctx.test_global("MISC_SE", "REPROGRAM", &["ENABLE", "DISABLE"]);
            ctx.test_global("MISC_SE", "DONETIME", &["BEFORE", "AFTER"]);
            ctx.test_global("MISC_SE", "RESETTIME", &["BEFORE", "AFTER"]);
            let tcrd =
                CellCoord::new(DieId::from_idx(0), grid.col_e(), grid.row_s()).tile(tslots::MAIN);
            let wt = TileWireCoord::new_idx(0, wires::IMUX_BUFG);
            let wf = TileWireCoord::new_idx(0, wires::OUT_OSC);
            let crd = backend.ngrid.int_pip(tcrd, wt, wf);
            let rwt = backend.edev.resolve_tile_wire(tcrd, wt).unwrap();
            let rwf = backend.edev.resolve_tile_wire(tcrd, wf).unwrap();
            for val in ["ENABLE", "DIV2"] {
                ctx.build()
                    .raw(Key::WireMutex(rwt), "OSC_SPECIAL")
                    .raw(Key::WireMutex(rwf), "OSC_SPECIAL")
                    .test_manual("OSC", "MODE", val)
                    .global_diff("XTALOSC", "DISABLE", val)
                    .raw_diff(Key::Pip(crd), None, Value::FromPin("OSC", "O".into()))
                    .raw_diff(
                        Key::BlockPin("ACLK", "I".into()),
                        None,
                        Value::FromPin("OSC", "O".into()),
                    )
                    .prop(BaseBelNoConfig::new(
                        bslots::IO_S[1],
                        "IN".into(),
                        "I".into(),
                    ))
                    .prop(BaseBelNoConfig::new(
                        bslots::IO_E[0],
                        "IN".into(),
                        "I".into(),
                    ))
                    .commit();
            }
        }
        if tcname.starts_with("CLB_SW") {
            ctx.test_global("MISC_SW", "READ", &["COMMAND", "ONCE", "DISABLE"]);
        }
        if tcname.starts_with("CLB_NW") {
            ctx.test_global("MISC_NW", "INPUT", &["TTL", "CMOS"]);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for (tcid, tcname, tcls) in &ctx.edev.db.tile_classes {
        if tcls.slot != tslots::MAIN {
            continue;
        }
        if !ctx.has_tile_id(tcid) {
            continue;
        }
        for slot in tcls.bels.ids() {
            let bel = ctx.edev.db.bel_slots.key(slot).as_str();
            if ctx.edev.db.bel_slots[slot].kind != BelKind::Class(bcls::IO) {
                continue;
            }
            ctx.state.get_diff(tcname, bel, "IN", "IQ").assert_empty();
            let item = ctx.extract_bit(tcname, bel, "TRI", "NOT");
            ctx.tiledb.insert(tcname, bel, "INV.T", item);
            let item = ctx.extract_bit(tcname, bel, "OUT", "NOT");
            ctx.tiledb.insert(tcname, bel, "INV.O", item);
            let item = xlat_enum(vec![
                ("SLOW", Diff::default()),
                ("FAST", ctx.state.get_diff(tcname, bel, "OUT", "FAST")),
            ]);
            ctx.tiledb.insert(tcname, bel, "SLEW", item);
            let item = xlat_enum(vec![
                ("O", Diff::default()),
                ("OFF", ctx.state.get_diff(tcname, bel, "OUT", "OQ")),
            ]);
            ctx.tiledb.insert(tcname, bel, "MUX.O", item);
            let item = ctx.extract_enum_bool(tcname, bel, "IN", "FF", "LATCH");
            ctx.tiledb.insert(tcname, bel, "IFF_LATCH", item);
        }
        if tcname.starts_with("CLB_SE") {
            let bel = "MISC_SE";
            ctx.collect_enum_bool_wide_mixed(tcname, bel, "REPROGRAM", "DISABLE", "ENABLE");
            ctx.collect_enum(tcname, bel, "DONETIME", &["BEFORE", "AFTER"]);
            ctx.collect_enum(tcname, bel, "RESETTIME", &["BEFORE", "AFTER"]);
            let bel = "DONE";
            let item = xlat_enum(vec![
                (
                    "PULLUP",
                    ctx.state.get_diff(tcname, bel, "DONEPAD", "PULLUP"),
                ),
                (
                    "PULLNONE",
                    ctx.state.get_diff(tcname, bel, "DONEPAD", "NOPULLUP"),
                ),
            ]);
            ctx.tiledb.insert(tcname, bel, "PULL", item);
            let bel = "OSC";
            let mut diffs = vec![("DISABLE", Diff::default())];
            for val in ["ENABLE", "DIV2"] {
                let mut diff = ctx.state.get_diff(tcname, bel, "MODE", val);
                diff.discard_bits(ctx.tiledb.item(tcname, "INT", "MUX.IMUX_BUFG"));
                diff.apply_bit_diff(ctx.tiledb.item(tcname, "IO_S[1]", "PULLUP"), false, true);
                diff.apply_bit_diff(ctx.tiledb.item(tcname, "IO_E[0]", "PULLUP"), false, true);
                diffs.push((val, diff));
            }
            ctx.tiledb.insert(tcname, bel, "MODE", xlat_enum(diffs));
        }
        if tcname.starts_with("CLB_SW") {
            let bel = "MISC_SW";
            ctx.collect_enum(tcname, bel, "READ", &["COMMAND", "ONCE", "DISABLE"]);
        }
        if tcname.starts_with("CLB_NW") {
            let bel = "MISC_NW";
            ctx.collect_enum(tcname, bel, "INPUT", &["TTL", "CMOS"]);
        }
    }
}
