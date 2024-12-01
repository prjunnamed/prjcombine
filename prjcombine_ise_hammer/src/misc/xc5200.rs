use bitvec::prelude::*;
use prjcombine_collector::xlat_enum;
use prjcombine_hammer::Session;
use prjcombine_types::tiledb::TileItemKind;
use prjcombine_xilinx_geom::ExpandedDevice;

use crate::{
    backend::IseBackend, diff::CollectorCtx, fgen::TileBits, fuzz::FuzzCtx, fuzz_enum, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ExpandedDevice::Xc2000(edev) = backend.edev else {
        unreachable!()
    };

    let ctx = FuzzCtx::new_fake_bel(session, backend, "CNR.BL", "MISC", TileBits::MainAuto);
    for val in ["ENABLE", "ENLL", "NE7", "DISABLE"] {
        fuzz_one!(ctx, "SCAN_TEST", val, [], [(global_opt "SCANTEST", val)]);
    }
    for val in ["ENABLE", "DISABLE"] {
        fuzz_one!(ctx, "READ_ABORT", val, [], [(global_opt "READABORT", val)]);
        fuzz_one!(ctx, "READ_CAPTURE", val, [], [(global_opt "READCAPTURE", val)]);
    }
    let ctx = FuzzCtx::new(session, backend, "CNR.BL", "BUFG", TileBits::Null);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "CLK")]);
    let ctx = FuzzCtx::new(session, backend, "CNR.BL", "RDBK", TileBits::Null);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "RDBK")]);
    let ctx = FuzzCtx::new(session, backend, "CNR.BL", "RDBK", TileBits::MainAuto);
    for val in ["CCLK", "RDBK"] {
        fuzz_one!(ctx, "READ_CLK", val, [(mode "RDBK"), (pin "CK")], [(global_opt "READCLK", val)]);
    }

    let ctx = FuzzCtx::new_fake_bel(session, backend, "CNR.TL", "MISC", TileBits::MainAuto);
    for val in ["ENABLE", "DISABLE"] {
        fuzz_one!(ctx, "BS_RECONFIG", val, [], [(global_opt "BSRECONFIG", val)]);
        fuzz_one!(ctx, "BS_READBACK", val, [], [(global_opt "BSREADBACK", val)]);
    }
    for val in ["TTL", "CMOS"] {
        fuzz_one!(ctx, "INPUT", val, [], [(global_opt "INPUT", val)]);
    }
    let ctx = FuzzCtx::new(session, backend, "CNR.BL", "BUFG", TileBits::Null);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "CLK")]);
    let ctx = FuzzCtx::new(session, backend, "CNR.TL", "BSCAN", TileBits::MainAuto);
    fuzz_one!(ctx, "ENABLE", "1", [], [(mode "BSCAN")]);

    let ctx = FuzzCtx::new_fake_bel(session, backend, "CNR.BR", "MISC", TileBits::MainAuto);
    for val in ["PULLUP", "PULLNONE"] {
        fuzz_one!(ctx, "PROGPIN", val, [], [(global_opt "PROGPIN", val)]);
        fuzz_one!(ctx, "DONEPIN", val, [], [(global_opt "DONEPIN", val)]);
    }
    for val in ["OFF", "ON"] {
        fuzz_one!(ctx, "TCTEST", val, [], [(global_opt "TCTEST", val)]);
    }
    let ctx = FuzzCtx::new(session, backend, "CNR.BR", "BUFG", TileBits::Null);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "CLK")]);
    let mut ctx = FuzzCtx::new(session, backend, "CNR.BR", "STARTUP", TileBits::Null);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "STARTUP")]);
    ctx.bits = TileBits::MainAuto;
    fuzz_enum!(ctx, "GRMUX", ["GR", "GRNOT"], [(mode "STARTUP"), (pin "GR")]);
    fuzz_enum!(ctx, "GTSMUX", ["GTS", "GTSNOT"], [(mode "STARTUP"), (pin "GTS")]);
    for (val, phase) in [("CCLK", "C4"), ("USERCLK", "U3")] {
        fuzz_one!(ctx, "STARTUP_CLK", val, [
            (mode "STARTUP"), (pin "CLK"),
            (global_opt "SYNCTODONE", "NO"),
            (global_opt "DONEACTIVE", "C1")
        ], [
            (global_opt_diff "GSRINACTIVE", "C4", phase),
            (global_opt_diff "OUTPUTSACTIVE", "C4", phase),
            (global_opt_diff "STARTUPCLK", "CCLK", val)
        ]);
    }
    for (val, phase) in [("NO", "C4"), ("YES", "DI_PLUS_1")] {
        fuzz_one!(ctx, "SYNC_TO_DONE", val, [
            (global_opt "STARTUPCLK", "CCLK"),
            (global_opt "DONEACTIVE", "C1")
        ], [
            (global_opt_diff "GSRINACTIVE", "C4", phase),
            (global_opt_diff "OUTPUTSACTIVE", "C4", phase),
            (global_opt_diff "SYNCTODONE", "NO", val)
        ]);
    }
    for val in ["C1", "C2", "C3", "C4"] {
        fuzz_one!(ctx, "DONE_ACTIVE", val, [
            (global_opt "SYNCTODONE", "NO"),
            (global_opt "STARTUPCLK", "CCLK")
        ], [(global_opt_diff "DONEACTIVE", "C1", val)]);
    }
    for val in ["U2", "U3", "U4"] {
        fuzz_one!(ctx, "DONE_ACTIVE", val, [
            (mode "STARTUP"),
            (pin "CLK"),
            (global_opt "SYNCTODONE", "NO"),
            (global_opt "STARTUPCLK", "USERCLK")
        ], [(global_opt_diff "DONEACTIVE", "C1", val)]);
    }
    for (attr, opt) in [
        ("OUTPUTS_ACTIVE", "OUTPUTSACTIVE"),
        ("GSR_INACTIVE", "GSRINACTIVE"),
    ] {
        for val in ["C2", "C3", "C4"] {
            fuzz_one!(ctx, attr, val, [
                (global_opt "SYNCTODONE", "NO"),
                (global_opt "STARTUPCLK", "CCLK")
            ], [(global_opt_diff opt, "C4", val)]);
        }
        for val in ["U2", "U3", "U4"] {
            fuzz_one!(ctx, attr, val, [
                (mode "STARTUP"),
                (pin "CLK"),
                (global_opt "SYNCTODONE", "NO"),
                (global_opt "STARTUPCLK", "USERCLK")
            ], [(global_opt_diff opt, "U3", val)]);
        }
        for val in ["DI", "DI_PLUS_1", "DI_PLUS_2"] {
            fuzz_one!(ctx, attr, val, [
                (mode "STARTUP"),
                (pin "CLK"),
                (global_opt "SYNCTODONE", "YES"),
                (global_opt "STARTUPCLK", "USERCLK")
            ], [(global_opt_diff opt, "DI_PLUS_1", val)]);
        }
    }

    for val in ["ENABLE", "DISABLE"] {
        fuzz_one!(ctx, "CRC", val, [], [(global_opt "CRC", val)]);
    }
    for val in ["SLOW", "MED", "FAST"] {
        fuzz_one!(ctx, "CONFIG_RATE", val, [], [(global_opt "CONFIGRATE", val)]);
    }
    // for val in ["LENGTH", "DONE"] {
    //     fuzz_one!(ctx, "LC_ALIGNMENT", val, [], [(global_opt "LC_ALIGNMENT", val)]);
    // }

    let ctx = FuzzCtx::new_fake_bel(session, backend, "CNR.TR", "MISC", TileBits::MainAuto);
    for val in ["OFF", "ON"] {
        fuzz_one!(ctx, "TAC", val, [], [(global_opt "TAC", val)]);
        fuzz_one!(ctx, "TLC", val, [], [(global_opt "TLC", val)]);
    }
    let ctx = FuzzCtx::new(session, backend, "CNR.TR", "BUFG", TileBits::Null);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "CLK")]);

    // pins located in TR, config in BR.
    let mut ctx = FuzzCtx::new(session, backend, "CNR.TR", "OSC", TileBits::Null);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "OSC")]);
    ctx.tile_name = "CNR.BR".to_string();
    ctx.bits = TileBits::Raw(vec![
        edev.btile_main(edev.grid.col_rio(), edev.grid.row_bio())
    ]);
    fuzz_enum!(ctx, "OSC1_ATTR", ["4", "16", "64", "256"], [(mode "OSC")]);
    fuzz_enum!(ctx, "OSC2_ATTR", ["2", "8", "32", "128", "1024", "4096", "16384", "65536"], [(mode "OSC")]);
    fuzz_enum!(ctx, "CMUX", ["CCLK", "USERCLK"], [(mode "OSC"), (pin "C")]);
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    {
        let tile = "CNR.BL";
        let bel = "MISC";
        ctx.collect_enum(
            tile,
            bel,
            "SCAN_TEST",
            &["ENABLE", "ENLL", "NE7", "DISABLE"],
        );
        ctx.collect_enum_bool(tile, bel, "READ_ABORT", "DISABLE", "ENABLE");
        ctx.collect_enum_bool(tile, bel, "READ_CAPTURE", "DISABLE", "ENABLE");
        let bel = "RDBK";
        ctx.collect_enum(tile, bel, "READ_CLK", &["CCLK", "RDBK"]);
    }

    {
        let tile = "CNR.TL";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "BS_RECONFIG", "DISABLE", "ENABLE");
        ctx.collect_enum_bool(tile, bel, "BS_READBACK", "DISABLE", "ENABLE");
        ctx.collect_enum(tile, bel, "INPUT", &["CMOS", "TTL"]);
        let bel = "BSCAN";
        ctx.collect_bit(tile, bel, "ENABLE", "1");
    }

    {
        let tile = "CNR.BR";
        let bel = "MISC";
        let item = ctx.extract_enum(tile, bel, "PROGPIN", &["PULLUP", "PULLNONE"]);
        ctx.tiledb.insert(tile, "PROG", "PULL", item);
        let item = ctx.extract_enum(tile, bel, "DONEPIN", &["PULLUP", "PULLNONE"]);
        ctx.tiledb.insert(tile, "DONE", "PULL", item);
        ctx.collect_enum_bool(tile, bel, "TCTEST", "OFF", "ON");
        let bel = "STARTUP";
        ctx.collect_enum_bool(tile, bel, "CRC", "DISABLE", "ENABLE");
        ctx.collect_enum(tile, bel, "CONFIG_RATE", &["SLOW", "MED", "FAST"]);
        let item = ctx.extract_enum_bool(tile, bel, "GRMUX", "GR", "GRNOT");
        ctx.tiledb.insert(tile, bel, "INV.GR", item);
        let item = ctx.extract_enum_bool(tile, bel, "GTSMUX", "GTS", "GTSNOT");
        ctx.tiledb.insert(tile, bel, "INV.GTS", item);
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
            let mut item = xlat_enum(vec![
                ("Q3", ctx.state.get_diff(tile, bel, attr, "DI_PLUS_1")),
                ("Q1Q4", ctx.state.get_diff(tile, bel, attr, "DI_PLUS_2")),
                ("Q2", ctx.state.get_diff(tile, bel, attr, "C3")),
                ("Q3", ctx.state.get_diff(tile, bel, attr, "C4")),
                ("Q1Q4", ctx.state.get_diff(tile, bel, attr, "C2")),
                ("Q2", ctx.state.get_diff(tile, bel, attr, "U2")),
                ("Q3", ctx.state.get_diff(tile, bel, attr, "U3")),
                ("Q1Q4", ctx.state.get_diff(tile, bel, attr, "U4")),
                ("DONE_IN", ctx.state.get_diff(tile, bel, attr, "DI")),
            ]);
            if attr == "GSR_INACTIVE" {
                // sigh. DI has identical value to DI_PLUS_2, which is obviously bogus.
                // not *completely* sure this is the right fixup, but it seems to be the most
                // likely option.
                let TileItemKind::Enum { ref mut values } = item.kind else {
                    unreachable!()
                };
                values.insert("DONE_IN".to_string(), bitvec![0; 2]);
            }
            ctx.tiledb.insert(tile, bel, attr, item);
        }
        ctx.collect_enum(tile, bel, "STARTUP_CLK", &["CCLK", "USERCLK"]);
        ctx.collect_enum_bool(tile, bel, "SYNC_TO_DONE", "NO", "YES");
        let bel = "OSC";
        let mut diffs = vec![];
        for i in [2, 4, 6, 8] {
            diffs.push((
                format!("D{i}"),
                ctx.state
                    .get_diff(tile, bel, "OSC1_ATTR", format!("{}", 1 << i)),
            ));
        }
        ctx.tiledb.insert(tile, bel, "OSC1", xlat_enum(diffs));
        let mut diffs = vec![];
        for i in [1, 3, 5, 7, 10, 12, 14, 16] {
            diffs.push((
                format!("D{i}"),
                ctx.state
                    .get_diff(tile, bel, "OSC2_ATTR", format!("{}", 1 << i)),
            ));
        }
        ctx.tiledb.insert(tile, bel, "OSC2", xlat_enum(diffs));
        ctx.collect_enum(tile, bel, "CMUX", &["CCLK", "USERCLK"]);
    }
    {
        let tile = "CNR.TR";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "TAC", "OFF", "ON");
        ctx.collect_enum_bool(tile, bel, "TLC", "OFF", "ON");
    }
}
