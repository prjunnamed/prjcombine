use prjcombine_interconnect::grid::{DieId, LayerId};
use prjcombine_re_collector::xlat_enum;
use prjcombine_re_hammer::Session;
use unnamed_entity::EntityId;

use crate::{backend::XactBackend, collector::CollectorCtx, fbuild::FuzzCtx};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, "CNR.BL");
    ctx.test_cfg5200("MISC", "SCANTEST", &["ENABLE", "ENLL", "NE7", "DISABLE"]);
    ctx.test_global("MISC", "READABORT", &["DISABLE", "ENABLE"]);
    ctx.test_global("MISC", "READCAPTURE", &["DISABLE", "ENABLE"]);
    ctx.test_global("RDBK", "READCLK", &["CCLK", "RDBK"]);

    let mut ctx = FuzzCtx::new(session, backend, "CNR.TL");
    ctx.test_global("MISC", "BSRECONFIG", &["DISABLE", "ENABLE"]);
    ctx.test_global("MISC", "BSREADBACK", &["DISABLE", "ENABLE"]);
    ctx.test_global("MISC", "INPUT", &["TTL", "CMOS"]);
    let mut bctx = ctx.bel("BSCAN");
    bctx.mode("BSCAN").test_cfg("BSCAN", "USED");

    let mut ctx = FuzzCtx::new(session, backend, "CNR.BR");
    ctx.test_cfg5200("MISC", "TCTEST", &["ON", "OFF"]);
    ctx.test_global("PROG", "PROGPIN", &["PULLUP", "NOPULLUP"]);
    ctx.test_global("DONE", "DONEPIN", &["PULLUP", "NOPULLUP"]);
    ctx.test_global("STARTUP", "CRC", &["DISABLE", "ENABLE"]);
    ctx.test_global("STARTUP", "CONFIGRATE", &["SLOW", "MED", "FAST"]);
    ctx.build()
        .bel_out("OSC", "CK")
        .test_global("OSC", "OSCCLK", &["CCLK", "USERCLK"]);

    ctx.build()
        .global("SYNCTODONE", "NO")
        .global("DONEACTIVE", "C1")
        .test_manual("STARTUP", "STARTUP_CLK", "USERCLK")
        .global_diff("GSRINACTIVE", "C4", "U3")
        .global_diff("OUTPUTSACTIVE", "C4", "U3")
        .global_diff("STARTUPCLK", "CCLK", "USERCLK")
        .bel_out("STARTUP", "CK")
        .commit();
    ctx.build()
        .global("STARTUPCLK", "CCLK")
        .global("DONEACTIVE", "C1")
        .test_manual("STARTUP", "SYNC_TO_DONE", "YES")
        .global_diff("GSRINACTIVE", "C4", "DI_PLUS_1")
        .global_diff("OUTPUTSACTIVE", "C4", "DI_PLUS_1")
        .global_diff("SYNCTODONE", "NO", "YES")
        .commit();

    for val in ["C1", "C2", "C3", "C4"] {
        ctx.build()
            .global("STARTUPCLK", "CCLK")
            .global("SYNCTODONE", "NO")
            .global("GSRINACTIVE", "C4")
            .global("OUTPUTSACTIVE", "C4")
            .test_manual("STARTUP", "DONE_ACTIVE", val)
            .global_diff("DONEACTIVE", "C1", val)
            .commit();
    }
    for val in ["U2", "U3", "U4"] {
        ctx.build()
            .global("STARTUPCLK", "USERCLK")
            .global("SYNCTODONE", "NO")
            .global("GSRINACTIVE", "U3")
            .global("OUTPUTSACTIVE", "U3")
            .bel_out("STARTUP", "CK")
            .test_manual("STARTUP", "DONE_ACTIVE", val)
            .global_diff("DONEACTIVE", "C1", val)
            .commit();
    }
    for (attr, opt, oopt) in [
        ("OUTPUTS_ACTIVE", "OUTPUTSACTIVE", "GSRINACTIVE"),
        ("GSR_INACTIVE", "GSRINACTIVE", "OUTPUTSACTIVE"),
    ] {
        for val in ["C2", "C3", "C4"] {
            ctx.build()
                .global("STARTUPCLK", "CCLK")
                .global("SYNCTODONE", "NO")
                .global("DONEACTIVE", "C1")
                .global(oopt, "C4")
                .test_manual("STARTUP", attr, val)
                .global_diff(opt, "C4", val)
                .commit();
        }
        for val in ["U2", "U3", "U4"] {
            ctx.build()
                .global("STARTUPCLK", "USERCLK")
                .global("SYNCTODONE", "NO")
                .global("DONEACTIVE", "C1")
                .global(oopt, "U3")
                .bel_out("STARTUP", "CK")
                .test_manual("STARTUP", attr, val)
                .global_diff(opt, "U3", val)
                .commit();
        }
        for val in ["DI", "DI_PLUS_1", "DI_PLUS_2"] {
            ctx.build()
                .global("STARTUPCLK", "USERCLK")
                .global("SYNCTODONE", "YES")
                .global("DONEACTIVE", "C1")
                .global(oopt, "DI_PLUS_1")
                .bel_out("STARTUP", "CK")
                .test_manual("STARTUP", attr, val)
                .global_diff(opt, "DI_PLUS_1", val)
                .commit();
        }
    }

    let mut bctx = ctx.bel("STARTUP");
    bctx.mode("STARTUP").test_cfg("GCLR", "NOT");
    bctx.mode("STARTUP").test_cfg("GTS", "NOT");

    let mut ctx = FuzzCtx::new(session, backend, "CNR.TR");
    ctx.test_cfg5200("MISC", "TLC", &["ON", "OFF"]);
    ctx.test_cfg5200("MISC", "TAC", &["ON", "OFF"]);
    let mut bctx = ctx.bel("OSC");
    let cnr_br = (
        DieId::from_idx(0),
        backend.edev.chip.col_rio(),
        backend.edev.chip.row_bio(),
        LayerId::from_idx(0),
    );
    for val in ["D2", "D4", "D6", "D8"] {
        bctx.mode("OSC")
            .extra_tile(cnr_br, "OSC", "OSC1", val)
            .mutex("OSC1", val)
            .test_cfg("OSC1", val);
    }
    for val in ["D1", "D3", "D5", "D7", "D10", "D12", "D14", "D16"] {
        bctx.mode("OSC")
            .extra_tile(cnr_br, "OSC", "OSC2", val)
            .mutex("OSC2", val)
            .test_cfg("OSC2", val);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "CNR.BL";
    let bel = "MISC";
    let item = ctx.extract_enum_bool(tile, bel, "READABORT", "DISABLE", "ENABLE");
    ctx.tiledb.insert(tile, bel, "READ_ABORT", item);
    let item = ctx.extract_enum_bool(tile, bel, "READCAPTURE", "DISABLE", "ENABLE");
    ctx.tiledb.insert(tile, bel, "READ_CAPTURE", item);
    let item = ctx.extract_enum(tile, bel, "SCANTEST", &["ENABLE", "ENLL", "NE7", "DISABLE"]);
    ctx.tiledb.insert(tile, bel, "SCAN_TEST", item);

    let bel = "RDBK";
    let item = ctx.extract_enum(tile, bel, "READCLK", &["RDBK", "CCLK"]);
    ctx.tiledb.insert(tile, bel, "READ_CLK", item);

    let tile = "CNR.TL";
    let bel = "MISC";
    let item = ctx.extract_enum_bool(tile, bel, "BSRECONFIG", "DISABLE", "ENABLE");
    ctx.tiledb.insert(tile, bel, "BS_RECONFIG", item);
    let item = ctx.extract_enum_bool(tile, bel, "BSREADBACK", "DISABLE", "ENABLE");
    ctx.tiledb.insert(tile, bel, "BS_READBACK", item);
    ctx.collect_enum(tile, bel, "INPUT", &["CMOS", "TTL"]);

    let bel = "BSCAN";
    let item = ctx.extract_bit(tile, bel, "BSCAN", "USED");
    ctx.tiledb.insert(tile, bel, "ENABLE", item);

    let tile = "CNR.BR";
    let bel = "MISC";
    ctx.collect_enum_bool(tile, bel, "TCTEST", "OFF", "ON");

    let bel = "STARTUP";
    ctx.collect_enum_bool(tile, bel, "CRC", "DISABLE", "ENABLE");
    let item = ctx.extract_enum(tile, bel, "CONFIGRATE", &["SLOW", "MED", "FAST"]);
    ctx.tiledb.insert(tile, bel, "CONFIG_RATE", item);
    let item = ctx.extract_bit(tile, bel, "GTS", "NOT");
    ctx.tiledb.insert(tile, bel, "INV.GTS", item);
    let item = ctx.extract_bit(tile, bel, "GCLR", "NOT");
    ctx.tiledb.insert(tile, bel, "INV.GR", item);
    let item = xlat_enum(vec![
        ("Q0", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "C1")),
        ("Q2", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "C3")),
        ("Q3", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "C4")),
        ("Q1Q4", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "C2")),
        ("Q2", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "U2")),
        ("Q3", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "U3")),
        ("Q1Q4", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "U4")),
    ]);
    ctx.tiledb.insert(tile, bel, "DONE_ACTIVE", item);
    for attr in ["OUTPUTS_ACTIVE", "GSR_INACTIVE"] {
        let item = xlat_enum(vec![
            ("DONE_IN", ctx.state.get_diff(tile, bel, attr, "DI")),
            ("Q3", ctx.state.get_diff(tile, bel, attr, "DI_PLUS_1")),
            ("Q1Q4", ctx.state.get_diff(tile, bel, attr, "DI_PLUS_2")),
            ("Q2", ctx.state.get_diff(tile, bel, attr, "C3")),
            ("Q3", ctx.state.get_diff(tile, bel, attr, "C4")),
            ("Q1Q4", ctx.state.get_diff(tile, bel, attr, "C2")),
            ("Q2", ctx.state.get_diff(tile, bel, attr, "U2")),
            ("Q3", ctx.state.get_diff(tile, bel, attr, "U3")),
            ("Q1Q4", ctx.state.get_diff(tile, bel, attr, "U4")),
        ]);
        ctx.tiledb.insert(tile, bel, attr, item);
    }
    ctx.collect_enum_default(tile, bel, "STARTUP_CLK", &["USERCLK"], "CCLK");
    ctx.collect_bit(tile, bel, "SYNC_TO_DONE", "YES");

    let bel = "DONE";
    let item = xlat_enum(vec![
        ("PULLUP", ctx.state.get_diff(tile, bel, "DONEPIN", "PULLUP")),
        (
            "PULLNONE",
            ctx.state.get_diff(tile, bel, "DONEPIN", "NOPULLUP"),
        ),
    ]);
    ctx.tiledb.insert(tile, bel, "PULL", item);
    let bel = "PROG";
    let item = xlat_enum(vec![
        ("PULLUP", ctx.state.get_diff(tile, bel, "PROGPIN", "PULLUP")),
        (
            "PULLNONE",
            ctx.state.get_diff(tile, bel, "PROGPIN", "NOPULLUP"),
        ),
    ]);
    ctx.tiledb.insert(tile, bel, "PULL", item);

    let bel = "OSC";
    ctx.collect_enum(tile, bel, "OSC1", &["D2", "D4", "D6", "D8"]);
    ctx.collect_enum(
        tile,
        bel,
        "OSC2",
        &["D1", "D3", "D5", "D7", "D10", "D12", "D14", "D16"],
    );
    let item = ctx.extract_enum(tile, bel, "OSCCLK", &["CCLK", "USERCLK"]);
    ctx.tiledb.insert(tile, bel, "CMUX", item);

    let tile = "CNR.TR";
    let bel = "MISC";
    ctx.collect_enum_bool(tile, bel, "TLC", "OFF", "ON");
    ctx.collect_enum_bool(tile, bel, "TAC", "OFF", "ON");
    let bel = "OSC";
    for val in ["D2", "D4", "D6", "D8"] {
        ctx.state.get_diff(tile, bel, "OSC1", val).assert_empty();
    }
    for val in ["D1", "D3", "D5", "D7", "D10", "D12", "D14", "D16"] {
        ctx.state.get_diff(tile, bel, "OSC2", val).assert_empty();
    }
}
