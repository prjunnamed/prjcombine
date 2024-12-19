use prjcombine_collector::xlat_enum;
use prjcombine_hammer::Session;
use prjcombine_xc2000::grid::GridKind;

use crate::{backend::XactBackend, collector::CollectorCtx, fbuild::FuzzCtx};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    let grid = backend.edev.grid;
    for tile in backend.egrid.db.nodes.keys() {
        if !tile.starts_with("IO") {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, tile);
        if grid.kind != GridKind::Xc4000H {
            for bel in ["IOB0", "IOB1"] {
                let mut bctx = ctx.bel(bel);
                bctx.mode("IO")
                    .mutex("CLK", "O")
                    .cfg("OUT", "OK")
                    .test_enum("OUT", &["SET", "RESET"]);
                bctx.mode("IO").mutex("CLK", "O").test_cfg("OUT", "OK");
                bctx.mode("IO")
                    .mutex("CLK", "O")
                    .cfg("OUT", "OK")
                    .test_cfg("OUT", "OKNOT");
                bctx.mode("IO")
                    .mutex("CLK", "I")
                    .cfg("INFF", "IK")
                    .test_enum("INFF", &["SET", "RESET"]);
                bctx.mode("IO").mutex("CLK", "I").test_cfg("INFF", "IK");
                bctx.mode("IO")
                    .mutex("CLK", "I")
                    .cfg("INFF", "IK")
                    .test_cfg("INFF", "IKNOT");
                bctx.mode("IO")
                    .mutex("CLK", "I")
                    .cfg("INFF", "IK")
                    .test_cfg("INFF", "DELAY");
                bctx.mode("IO")
                    .mutex("CLK", "I")
                    .cfg("INFF", "IK")
                    .test_enum("PAD", &["PULLDOWN", "PULLUP"]);
                if grid.kind != GridKind::Xc4000A {
                    bctx.mode("IO")
                        .mutex("CLK", "I")
                        .cfg("INFF", "IK")
                        .test_cfg("PAD", "FAST");
                } else {
                    bctx.mode("IO")
                        .mutex("CLK", "I")
                        .cfg("INFF", "IK")
                        .test_enum("OSPEED", &["SLOW", "MEDSLOW", "MEDFAST", "FAST"]);
                }
                bctx.mode("IO")
                    .mutex("CLK", "O")
                    .cfg("OUT", "OK")
                    .mutex("OUT", "O")
                    .cfg("OUT", "O")
                    .cfg("TRI", "T")
                    .test_cfg("TRI", "NOT");
                bctx.mode("IO")
                    .mutex("CLK", "I")
                    .cfg("INFF", "IK")
                    .test_enum("I1", &["I", "IQ", "IQL"]);
                bctx.mode("IO")
                    .mutex("CLK", "I")
                    .cfg("INFF", "IK")
                    .test_enum("I2", &["I", "IQ", "IQL"]);
                bctx.mode("IO")
                    .mutex("CLK", "I")
                    .cfg("INFF", "IK")
                    .test_enum("RDBK", &["I1", "I2", "OQ"]);
            }
        } else {
            for bel in ["HIOB0", "HIOB1", "HIOB2", "HIOB3"] {
                let mut bctx = ctx.bel(bel);
                bctx.mode("IO")
                    .bonded_io()
                    .cfg("IN", "I")
                    .test_enum("PAD", &["PULLDOWN", "PULLUP"]);
                bctx.mode("IO")
                    .bonded_io()
                    .cfg("IN", "I")
                    .cfg("OUT", "O")
                    .test_enum("TRI", &["TS", "TP"]);
                bctx.mode("IO")
                    .bonded_io()
                    .cfg("IN", "I")
                    .cfg("OUT", "O")
                    .test_cfg("TRI", "NOT");
                bctx.mode("IO").bonded_io().test_cfg("IN", "I");
                bctx.mode("IO")
                    .bonded_io()
                    .cfg("IN", "I")
                    .test_cfg("IN", "NOT");
                bctx.mode("IO")
                    .bonded_io()
                    .cfg("IN", "I")
                    .test_enum("IN", &["CMOS", "TTL"]);
                bctx.mode("IO")
                    .bonded_io()
                    .cfg("IN", "I")
                    .test_cfg("OUT", "O");
                bctx.mode("IO")
                    .bonded_io()
                    .cfg("IN", "I")
                    .cfg("OUT", "O")
                    .test_cfg("OUT", "NOT");
                bctx.mode("IO")
                    .bonded_io()
                    .cfg("IN", "I")
                    .cfg("OUT", "O")
                    .test_enum("OUT", &["CMOS", "TTL"]);
                bctx.mode("IO")
                    .bonded_io()
                    .cfg("IN", "I")
                    .cfg("OUT", "O")
                    .test_enum("OUT", &["CAP", "RES"]);
                bctx.mode("IO")
                    .bonded_io()
                    .cfg("IN", "I")
                    .test_cfg("RDBK", "I");
            }
        }
        for bel in ["DEC0", "DEC1", "DEC2"] {
            let mut bctx = ctx.bel(bel);
            if grid.kind != GridKind::Xc4000A {
                for pin in ["O1", "O2", "O3", "O4"] {
                    bctx.mode("DECODER")
                        .pin_mutex_exclusive(pin)
                        .test_manual(format!("{pin}_P"), "1")
                        .pip_pin(pin, pin)
                        .commit();
                    bctx.mode("DECODER")
                        .pin_mutex_exclusive(pin)
                        .test_manual(format!("{pin}_N"), "1")
                        .pip_pin(pin, pin)
                        .cfg(pin, "NOT")
                        .commit();
                }
            } else {
                for pin in ["O1", "O2"] {
                    bctx.mode("DECODER")
                        .pin_mutex_exclusive(pin)
                        .test_manual(format!("{pin}_P"), "1")
                        .pip_pin(pin, pin)
                        .commit();
                    bctx.mode("DECODER")
                        .pin_mutex_exclusive(pin)
                        .test_manual(format!("{pin}_N"), "1")
                        .pip_pin(pin, pin)
                        .cfg(pin, "NOT")
                        .commit();
                }
            }
        }
        if tile.starts_with("IO.L") || tile.starts_with("IO.R") {
            for bel in ["PULLUP.TBUF0", "PULLUP.TBUF1"] {
                let mut bctx = ctx.bel(bel);
                bctx.build()
                    .pin_mutex_exclusive("O")
                    .test_manual("ENABLE", "1")
                    .pip_pin("O", "O")
                    .commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let grid = ctx.edev.grid;
    for tile in ctx.edev.egrid.db.nodes.keys() {
        if !tile.starts_with("IO") {
            continue;
        }
        if grid.kind != GridKind::Xc4000H {
            for bel in ["IOB0", "IOB1"] {
                let item =
                    ctx.extract_enum_default(tile, bel, "PAD", &["PULLUP", "PULLDOWN"], "NONE");
                ctx.tiledb.insert(tile, bel, "PULL", item);
                if grid.kind != GridKind::Xc4000A {
                    let item = ctx.extract_enum_default(tile, bel, "PAD", &["FAST"], "SLOW");
                    ctx.tiledb.insert(tile, bel, "SLEW", item);
                } else {
                    let item = ctx.extract_enum(
                        tile,
                        bel,
                        "OSPEED",
                        &["SLOW", "MEDSLOW", "MEDFAST", "FAST"],
                    );
                    ctx.tiledb.insert(tile, bel, "SLEW", item);
                }
                let item = ctx.extract_enum_default(tile, bel, "INFF", &["DELAY"], "I");
                ctx.tiledb.insert(tile, bel, "IFF_D", item);

                let item = ctx.extract_enum_bool(tile, bel, "INFF", "RESET", "SET");
                ctx.tiledb.insert(tile, bel, "IFF_SRVAL", item);
                let item = ctx.extract_enum_bool(tile, bel, "OUT", "RESET", "SET");
                ctx.tiledb.insert(tile, bel, "OFF_SRVAL", item);
                let item = ctx.extract_bit(tile, bel, "INFF", "IKNOT");
                ctx.tiledb.insert(tile, bel, "INV.IFF_CLK", item);
                let item = ctx.extract_bit(tile, bel, "OUT", "OKNOT");
                ctx.tiledb.insert(tile, bel, "INV.OFF_CLK", item);

                let item = ctx.extract_enum(tile, bel, "I1", &["I", "IQ", "IQL"]);
                ctx.tiledb.insert(tile, bel, "I1MUX", item);
                let item = ctx.extract_enum(tile, bel, "I2", &["I", "IQ", "IQL"]);
                ctx.tiledb.insert(tile, bel, "I2MUX", item);

                let item = ctx.extract_bit(tile, bel, "TRI", "NOT");
                ctx.tiledb.insert(tile, bel, "INV.T", item);

                let item = ctx.extract_bit(tile, bel, "RDBK", "I1");
                ctx.tiledb.insert(tile, bel, "READBACK_I1", item);
                let item = ctx.extract_bit(tile, bel, "RDBK", "I2");
                ctx.tiledb.insert(tile, bel, "READBACK_I2", item);
                let item = ctx.extract_bit(tile, bel, "RDBK", "OQ");
                ctx.tiledb.insert(tile, bel, "READBACK_OFF", item);

                let mut diff = ctx.state.get_diff(tile, bel, "INFF", "IK");
                diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "PULL"), "NONE", "PULLUP");
                diff.assert_empty();
                let mut diff = ctx.state.get_diff(tile, bel, "OUT", "OK");
                diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "PULL"), "NONE", "PULLUP");
                diff.assert_empty();
            }
        } else {
            for bel in ["HIOB0", "HIOB1", "HIOB2", "HIOB3"] {
                let item =
                    ctx.extract_enum_default(tile, bel, "PAD", &["PULLUP", "PULLDOWN"], "NONE");
                ctx.tiledb.insert(tile, bel, "PULL", item);

                let item = ctx.extract_enum(tile, bel, "IN", &["CMOS", "TTL"]);
                ctx.tiledb.insert(tile, bel, "ISTD", item);
                let item = ctx.extract_enum(tile, bel, "OUT", &["CMOS", "TTL"]);
                ctx.tiledb.insert(tile, bel, "OSTD", item);
                let item = ctx.extract_enum(tile, bel, "OUT", &["CAP", "RES"]);
                ctx.tiledb.insert(tile, bel, "OMODE", item);

                let item = ctx.extract_bit(tile, bel, "IN", "NOT");
                ctx.tiledb.insert(tile, bel, "INV.I", item);
                let item = ctx.extract_bit(tile, bel, "OUT", "NOT");
                ctx.tiledb.insert(tile, bel, "INV.O", item);
                let item = ctx.extract_bit(tile, bel, "TRI", "NOT");
                ctx.tiledb.insert(tile, bel, "INV.T", item);

                let item = xlat_enum(vec![
                    ("T1", ctx.state.get_diff(tile, bel, "TRI", "TP")),
                    ("T2", ctx.state.get_diff(tile, bel, "TRI", "TS")),
                ]);
                ctx.tiledb.insert(tile, bel, "MUX.T", item);

                let item = ctx.extract_bit(tile, bel, "RDBK", "I");
                ctx.tiledb.insert(tile, bel, "READBACK_I", item);

                let mut diff = ctx.state.get_diff(tile, bel, "IN", "I");
                diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "PULL"), "NONE", "PULLUP");
                diff.assert_empty();
                let mut diff = ctx.state.get_diff(tile, bel, "OUT", "O");
                diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "OSTD"), "TTL", "CMOS");
                diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.T"), false, true);
                diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "OMODE"), "RES", "CAP");
                diff.assert_empty();
            }
        }
        for bel in ["DEC0", "DEC1", "DEC2"] {
            if grid.kind != GridKind::Xc4000A {
                for pin in ["O1", "O2", "O3", "O4"] {
                    ctx.collect_bit(tile, bel, &format!("{pin}_P"), "1");
                    ctx.collect_bit(tile, bel, &format!("{pin}_N"), "1");
                }
            } else {
                for pin in ["O1", "O2"] {
                    ctx.collect_bit(tile, bel, &format!("{pin}_P"), "1");
                    ctx.collect_bit(tile, bel, &format!("{pin}_N"), "1");
                }
            }
        }
        if tile.starts_with("IO.L") || tile.starts_with("IO.R") {
            for bel in ["PULLUP.TBUF0", "PULLUP.TBUF1"] {
                ctx.collect_bit(tile, bel, "ENABLE", "1");
            }
        }
    }
}
