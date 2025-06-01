use prjcombine_interconnect::grid::DieId;
use prjcombine_re_fpga_hammer::xlat_enum;
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{bits, bsdata::TileItemKind};
use prjcombine_xc2000::bels::xc5200 as bels;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Xc2000(edev) = backend.edev else {
        unreachable!()
    };

    {
        let mut ctx = FuzzCtx::new(session, backend, "CNR.BL");
        for val in ["ENABLE", "ENLL", "NE7", "DISABLE"] {
            ctx.test_manual("MISC", "SCAN_TEST", val)
                .global("SCANTEST", val)
                .commit();
        }
        for val in ["ENABLE", "DISABLE"] {
            ctx.test_manual("MISC", "READ_ABORT", val)
                .global("READABORT", val)
                .commit();
            ctx.test_manual("MISC", "READ_CAPTURE", val)
                .global("READCAPTURE", val)
                .commit();
        }
        let mut bctx = ctx.bel(bels::BUFG);
        bctx.build()
            .null_bits()
            .test_manual("PRESENT", "1")
            .mode("CLK")
            .commit();
        let mut bctx = ctx.bel(bels::RDBK);
        bctx.build()
            .null_bits()
            .test_manual("PRESENT", "1")
            .mode("RDBK")
            .commit();
        let mut bctx = ctx.bel(bels::RDBK);
        for val in ["CCLK", "RDBK"] {
            bctx.mode("RDBK")
                .pin("CK")
                .test_manual("READ_CLK", val)
                .global("READCLK", val)
                .commit();
        }
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, "CNR.TL");
        for val in ["ENABLE", "DISABLE"] {
            ctx.test_manual("MISC", "BS_RECONFIG", val)
                .global("BSRECONFIG", val)
                .commit();
            ctx.test_manual("MISC", "BS_READBACK", val)
                .global("BSREADBACK", val)
                .commit();
        }
        for val in ["TTL", "CMOS"] {
            ctx.test_manual("MISC", "INPUT", val)
                .global("INPUT", val)
                .commit();
        }
        let mut bctx = ctx.bel(bels::BUFG);
        bctx.build()
            .null_bits()
            .test_manual("PRESENT", "1")
            .mode("CLK")
            .commit();
        let mut bctx = ctx.bel(bels::BSCAN);
        bctx.test_manual("ENABLE", "1").mode("BSCAN").commit();
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, "CNR.BR");
        for val in ["PULLUP", "PULLNONE"] {
            ctx.test_manual("MISC", "PROGPIN", val)
                .global("PROGPIN", val)
                .commit();
            ctx.test_manual("MISC", "DONEPIN", val)
                .global("DONEPIN", val)
                .commit();
        }
        for val in ["OFF", "ON"] {
            ctx.test_manual("MISC", "TCTEST", val)
                .global("TCTEST", val)
                .commit();
        }
        let mut bctx = ctx.bel(bels::BUFG);
        bctx.build()
            .null_bits()
            .test_manual("PRESENT", "1")
            .mode("CLK")
            .commit();
        let mut bctx = ctx.bel(bels::STARTUP);
        bctx.build()
            .null_bits()
            .test_manual("PRESENT", "1")
            .mode("STARTUP")
            .commit();
        bctx.mode("STARTUP")
            .pin("GR")
            .test_enum("GRMUX", &["GR", "GRNOT"]);
        bctx.mode("STARTUP")
            .pin("GTS")
            .test_enum("GTSMUX", &["GTS", "GTSNOT"]);
        for (val, phase) in [("CCLK", "C4"), ("USERCLK", "U3")] {
            bctx.mode("STARTUP")
                .pin("CLK")
                .global("SYNCTODONE", "NO")
                .global("DONEACTIVE", "C1")
                .test_manual("STARTUP_CLK", val)
                .global_diff("GSRINACTIVE", "C4", phase)
                .global_diff("OUTPUTSACTIVE", "C4", phase)
                .global_diff("STARTUPCLK", "CCLK", val)
                .commit();
        }
        for (val, phase) in [("NO", "C4"), ("YES", "DI_PLUS_1")] {
            bctx.build()
                .global("STARTUPCLK", "CCLK")
                .global("DONEACTIVE", "C1")
                .test_manual("SYNC_TO_DONE", val)
                .global_diff("GSRINACTIVE", "C4", phase)
                .global_diff("OUTPUTSACTIVE", "C4", phase)
                .global_diff("SYNCTODONE", "NO", val)
                .commit();
        }
        for val in ["C1", "C2", "C3", "C4"] {
            bctx.build()
                .global("SYNCTODONE", "NO")
                .global("STARTUPCLK", "CCLK")
                .test_manual("DONE_ACTIVE", val)
                .global_diff("DONEACTIVE", "C1", val)
                .commit();
        }
        for val in ["U2", "U3", "U4"] {
            bctx.mode("STARTUP")
                .pin("CLK")
                .global("SYNCTODONE", "NO")
                .global("STARTUPCLK", "USERCLK")
                .test_manual("DONE_ACTIVE", val)
                .global_diff("DONEACTIVE", "C1", val)
                .commit();
        }
        for (attr, opt) in [
            ("OUTPUTS_ACTIVE", "OUTPUTSACTIVE"),
            ("GSR_INACTIVE", "GSRINACTIVE"),
        ] {
            for val in ["C2", "C3", "C4"] {
                bctx.build()
                    .global("SYNCTODONE", "NO")
                    .global("STARTUPCLK", "CCLK")
                    .test_manual(attr, val)
                    .global_diff(opt, "C4", val)
                    .commit();
            }
            for val in ["U2", "U3", "U4"] {
                bctx.mode("STARTUP")
                    .pin("CLK")
                    .global("SYNCTODONE", "NO")
                    .global("STARTUPCLK", "USERCLK")
                    .test_manual(attr, val)
                    .global_diff(opt, "U3", val)
                    .commit();
            }
            for val in ["DI", "DI_PLUS_1", "DI_PLUS_2"] {
                bctx.mode("STARTUP")
                    .pin("CLK")
                    .global("SYNCTODONE", "YES")
                    .global("STARTUPCLK", "USERCLK")
                    .test_manual(attr, val)
                    .global_diff(opt, "DI_PLUS_1", val)
                    .commit();
            }
        }

        for val in ["ENABLE", "DISABLE"] {
            bctx.test_manual("CRC", val).global("CRC", val).commit();
        }
        for val in ["SLOW", "MED", "FAST"] {
            bctx.test_manual("CONFIG_RATE", val)
                .global("CONFIGRATE", val)
                .commit();
        }
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, "CNR.TR");
        for val in ["OFF", "ON"] {
            ctx.test_manual("MISC", "TAC", val)
                .global("TAC", val)
                .commit();
            ctx.test_manual("MISC", "TLC", val)
                .global("TLC", val)
                .commit();
        }
        let mut bctx = ctx.bel(bels::BUFG);
        bctx.build()
            .null_bits()
            .test_manual("PRESENT", "1")
            .mode("CLK")
            .commit();

        // pins located in TR, config in BR.
        let mut bctx = ctx.bel(bels::OSC);
        bctx.build()
            .null_bits()
            .test_manual("PRESENT", "1")
            .mode("OSC")
            .commit();
        // TODO
        let die = DieId::from_idx(0);
        let col = edev.chip.col_e();
        let row = edev.chip.row_s();
        let layer = edev
            .egrid
            .find_tile_layer(die, (col, row), |kind| kind == "CNR.BR")
            .unwrap();
        let cnr_br = (die, col, row, layer);
        bctx.mode("OSC")
            .extra_tile_fixed(cnr_br, "OSC")
            .null_bits()
            .test_enum("OSC1_ATTR", &["4", "16", "64", "256"]);
        bctx.mode("OSC")
            .extra_tile_fixed(cnr_br, "OSC")
            .null_bits()
            .test_enum(
                "OSC2_ATTR",
                &["2", "8", "32", "128", "1024", "4096", "16384", "65536"],
            );
        bctx.mode("OSC")
            .extra_tile_fixed(cnr_br, "OSC")
            .null_bits()
            .pin("C")
            .test_enum("CMUX", &["CCLK", "USERCLK"]);
    }
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
                values.insert("DONE_IN".to_string(), bits![0; 2]);
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
