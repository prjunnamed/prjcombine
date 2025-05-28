use prjcombine_interconnect::db::TileCellId;
use prjcombine_re_fpga_hammer::{OcdMode, xlat_bitvec, xlat_bool, xlat_enum_int};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::{TileBit, TileItem};
use prjcombine_virtex::{bels, chip::ChipKind};
use prjcombine_xilinx_bitstream::Reg;
use unnamed_entity::EntityId;

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::mutex::NodeMutexExclusive,
    },
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    for tile in ["CLKL", "CLKR"] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        let mut bctx = ctx.bel(bels::PCILOGIC);
        bctx.test_manual("PRESENT", "1").mode("PCILOGIC").commit();
        bctx.mode("PCILOGIC")
            .pin("I1")
            .test_enum("I1MUX", &["0", "1", "I1", "I1_B"]);
        bctx.mode("PCILOGIC")
            .pin("I2")
            .test_enum("I2MUX", &["0", "1", "I2", "I2_B"]);
    }
    let mut ctx = FuzzCtx::new_null(session, backend);
    for val in ["00", "01", "10", "11"] {
        ctx.build()
            .extra_tiles_by_bel(bels::PCILOGIC, "PCILOGIC")
            .test_manual("PCILOGIC", "PCI_DELAY", val)
            .global("PCIDELAY", val)
            .commit();
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, "CNR.BL");
        for attr in ["M0PIN", "M1PIN", "M2PIN"] {
            for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
                ctx.test_manual("MISC", attr, val)
                    .global(attr, val)
                    .commit();
            }
        }
        for attr in ["POWERDOWNPIN", "PDSTATUSPIN"] {
            for val in ["PULLUP", "PULLNONE"] {
                ctx.test_manual("MISC", attr, val)
                    .global(attr, val)
                    .commit();
            }
        }
        for val in ["NO", "YES"] {
            ctx.test_manual("MISC", "DRIVE_PD_STATUS", val)
                .global("DRIVEPDSTATUS", val)
                .commit();
        }
        for val in ["100US", "200US", "400US"] {
            ctx.test_manual("MISC", "POWERUP_DELAY", val)
                .global("POWERUPDELAY", val)
                .commit();
        }

        let mut bctx = ctx.bel(bels::CAPTURE);
        bctx.test_manual("PRESENT", "1").mode("CAPTURE").commit();
        bctx.mode("CAPTURE")
            .pin("CLK")
            .test_enum("CLKINV", &["0", "1"]);
        bctx.mode("CAPTURE")
            .pin("CAP")
            .test_enum("CAPMUX", &["0", "1", "CAP", "CAP_B"]);
        bctx.mode("CAPTURE")
            .extra_tile_reg_attr(Reg::Cor0, "REG.COR", "CAPTURE", "ONESHOT", "1")
            .test_manual("ONESHOT", "1")
            .attr("ONESHOT_ATTR", "ONE_SHOT")
            .commit();
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, "CNR.TL");
        for attr in ["TMSPIN", "TCKPIN"] {
            for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
                ctx.test_manual("MISC", attr, val)
                    .global(attr, val)
                    .commit();
            }
        }
        for val in ["INTOSC", "USERCLK", "CCLK"] {
            ctx.test_manual("MISC", "POWERUP_CLK", val)
                .global("POWERUPCLK", val)
                .commit();
        }
        for attr in ["IBCLK_N2", "IBCLK_N4", "IBCLK_N8", "IBCLK_N16", "IBCLK_N32"] {
            for val in ["0", "1"] {
                ctx.test_manual("MISC", attr, val)
                    .global(attr, val)
                    .commit();
            }
        }

        let mut bctx = ctx.bel(bels::STARTUP);
        bctx.test_manual("PRESENT", "1").mode("STARTUP").commit();
        bctx.mode("STARTUP")
            .pin("CLK")
            .test_enum("CLKINV", &["0", "1"]);
        bctx.mode("STARTUP")
            .pin("GWE")
            .test_enum("GWEMUX", &["0", "1", "GWE", "GWE_B"]);
        bctx.mode("STARTUP")
            .pin("GTS")
            .test_enum("GTSMUX", &["0", "1", "GTS", "GTS_B"]);
        bctx.mode("STARTUP")
            .pin("GSR")
            .test_enum("GSRMUX", &["0", "1", "GSR", "GSR_B"]);
        let wire_gwe = (
            TileCellId::from_idx(0),
            backend.egrid.db.get_wire("IMUX.STARTUP.GWE"),
        );
        let wire_gts = (
            TileCellId::from_idx(0),
            backend.egrid.db.get_wire("IMUX.STARTUP.GTS"),
        );
        let wire_gsr = (
            TileCellId::from_idx(0),
            backend.egrid.db.get_wire("IMUX.STARTUP.GSR"),
        );
        bctx.mode("STARTUP")
            .no_pin("GTS")
            .no_pin("GWE")
            .test_manual("GSR", "1")
            .prop(NodeMutexExclusive::new(wire_gwe))
            .prop(NodeMutexExclusive::new(wire_gts))
            .prop(NodeMutexExclusive::new(wire_gsr))
            .pin("GSR")
            .attr("GSRMUX", "GSR_B")
            .commit();
        bctx.mode("STARTUP")
            .no_pin("GSR")
            .no_pin("GWE")
            .test_manual("GTS", "1")
            .prop(NodeMutexExclusive::new(wire_gwe))
            .prop(NodeMutexExclusive::new(wire_gts))
            .prop(NodeMutexExclusive::new(wire_gsr))
            .pin("GTS")
            .attr("GTSMUX", "GTS_B")
            .commit();
        bctx.mode("STARTUP")
            .no_pin("GTS")
            .no_pin("GSR")
            .test_manual("GWE", "1")
            .prop(NodeMutexExclusive::new(wire_gwe))
            .prop(NodeMutexExclusive::new(wire_gts))
            .prop(NodeMutexExclusive::new(wire_gsr))
            .pin("GWE")
            .attr("GWEMUX", "GWE")
            .commit();
        for val in ["NO", "YES"] {
            bctx.test_manual("GWE_SYNC", val)
                .global("GWE_SYNC", val)
                .commit();
            bctx.test_manual("GTS_SYNC", val)
                .global("GTS_SYNC", val)
                .commit();
            bctx.test_manual("GSR_SYNC", val)
                .global("GSR_SYNC", val)
                .commit();
        }
        for val in ["CCLK", "USERCLK", "JTAGCLK"] {
            bctx.mode("STARTUP")
                .pin("CLK")
                .extra_tile_reg_attr(Reg::Cor0, "REG.COR", "STARTUP", "STARTUPCLK", val)
                .test_manual("STARTUPCLK", val)
                .global("STARTUPCLK", val)
                .commit();
        }

        let mut bctx = ctx.bel(bels::BSCAN);
        bctx.test_manual("PRESENT", "1").mode("BSCAN").commit();
        bctx.mode("BSCAN")
            .pin("TDO1")
            .test_enum("TDO1MUX", &["0", "1", "TDO1", "TDO1_B"]);
        bctx.mode("BSCAN")
            .pin("TDO2")
            .test_enum("TDO2MUX", &["0", "1", "TDO2", "TDO2_B"]);
        bctx.test_manual("USERID", "")
            .multi_global("USERID", MultiValue::HexPrefix, 32);
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, "CNR.BR");
        for attr in ["DONEPIN", "PROGPIN"] {
            for val in ["PULLUP", "PULLNONE"] {
                ctx.test_manual("MISC", attr, val)
                    .global(attr, val)
                    .commit();
            }
        }
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, "CNR.TR");
        for attr in ["TDIPIN", "TDOPIN"] {
            for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
                ctx.test_manual("MISC", attr, val)
                    .global(attr, val)
                    .commit();
            }
        }
        for val in ["PULLUP", "PULLNONE"] {
            ctx.test_manual("MISC", "CCLKPIN", val)
                .global("CCLKPIN", val)
                .commit();
        }
    }

    let mut ctx = FuzzCtx::new_null(session, backend);
    for attr in ["GSR_CYCLE", "GWE_CYCLE", "GTS_CYCLE"] {
        for val in ["1", "2", "3", "4", "5", "6", "DONE", "KEEP"] {
            ctx.test_reg(Reg::Cor0, "REG.COR", "STARTUP", attr, val)
                .global(attr, val)
                .commit();
        }
    }
    for val in ["1", "2", "3", "4", "5", "6", "KEEP"] {
        ctx.test_reg(Reg::Cor0, "REG.COR", "STARTUP", "DONE_CYCLE", val)
            .global("DONE_CYCLE", val)
            .commit();
    }
    for val in ["0", "1", "2", "3", "4", "5", "6", "NOWAIT"] {
        ctx.test_reg(Reg::Cor0, "REG.COR", "STARTUP", "LCK_CYCLE", val)
            .global("LCK_CYCLE", val)
            .commit();
    }
    for val in ["NO", "YES"] {
        ctx.test_reg(Reg::Cor0, "REG.COR", "STARTUP", "DRIVE_DONE", val)
            .global("DRIVEDONE", val)
            .commit();
        ctx.test_reg(Reg::Cor0, "REG.COR", "STARTUP", "DONE_PIPE", val)
            .global("DONEPIPE", val)
            .commit();
    }
    for val in [
        "4", "5", "7", "8", "9", "10", "13", "15", "20", "26", "30", "34", "41", "51", "55", "60",
        "130",
    ] {
        ctx.test_reg(Reg::Cor0, "REG.COR", "STARTUP", "CONFIG_RATE", val)
            .global("CONFIGRATE", val)
            .commit();
    }

    let mut ctx = FuzzCtx::new_null(session, backend);

    // persist not fuzzed — too much effort
    for val in ["NONE", "LEVEL1", "LEVEL2"] {
        ctx.test_reg(Reg::Ctl0, "REG.CTL", "MISC", "SECURITY", val)
            .global("SECURITY", val)
            .commit();
    }
    for val in ["0", "1"] {
        for attr in ["DISPMP1", "DISPMP2"] {
            ctx.test_reg(Reg::Ctl0, "REG.CTL", "MISC", attr, val)
                .global(attr, val)
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex(edev) = ctx.edev else {
        unreachable!()
    };
    for tile in ["CLKL", "CLKR"] {
        let bel = "PCILOGIC";
        let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "1");
        for (pinmux, pin, pin_b) in [("I1MUX", "I1", "I1_B"), ("I2MUX", "I2", "I2_B")] {
            // this is different from other virtex muxes!
            let d0 = ctx.state.get_diff(tile, bel, pinmux, pin);
            assert_eq!(d0, ctx.state.get_diff(tile, bel, pinmux, "0"));
            let d1 = ctx.state.get_diff(tile, bel, pinmux, pin_b);
            assert_eq!(d1, ctx.state.get_diff(tile, bel, pinmux, "1"));
            let item = xlat_bool(d0, d1);
            present.discard_bits(&item);
            ctx.insert_int_inv(&[tile], tile, bel, pin, item);
        }
        present.assert_empty();
        if edev.chip.kind == ChipKind::Virtex {
            let d0 = ctx.state.get_diff(tile, bel, "PCI_DELAY", "00");
            let d1 = ctx.state.get_diff(tile, bel, "PCI_DELAY", "01");
            let d2 = ctx.state.get_diff(tile, bel, "PCI_DELAY", "10");
            let d3 = ctx.state.get_diff(tile, bel, "PCI_DELAY", "11");
            // bug? bug.
            assert_eq!(d0, d1);
            ctx.tiledb.insert(
                tile,
                bel,
                "PCI_DELAY",
                xlat_enum_int(vec![(0, d0), (2, d2), (3, d3)]),
            );
        } else {
            for val in ["00", "01", "10", "11"] {
                ctx.state
                    .get_diff(tile, bel, "PCI_DELAY", val)
                    .assert_empty();
            }
        }
    }
    {
        let tile = "CNR.BL";
        let bel = "MISC";
        ctx.collect_enum(tile, bel, "M0PIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        ctx.collect_enum(tile, bel, "M1PIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        ctx.collect_enum(tile, bel, "M2PIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        if edev.chip.kind == ChipKind::Virtex && ctx.device.name.contains("2s") {
            ctx.collect_enum(tile, bel, "POWERDOWNPIN", &["PULLUP", "PULLNONE"]);
            ctx.collect_enum(tile, bel, "PDSTATUSPIN", &["PULLUP", "PULLNONE"]);
            ctx.collect_enum(tile, bel, "POWERUP_DELAY", &["100US", "200US", "400US"]);
            ctx.collect_enum_bool(tile, bel, "DRIVE_PD_STATUS", "NO", "YES");
        } else {
            for (attr, val) in [
                ("POWERDOWNPIN", "PULLUP"),
                ("POWERDOWNPIN", "PULLNONE"),
                ("PDSTATUSPIN", "PULLUP"),
                ("PDSTATUSPIN", "PULLNONE"),
                ("POWERUP_DELAY", "100US"),
                ("POWERUP_DELAY", "200US"),
                ("POWERUP_DELAY", "400US"),
                ("DRIVE_PD_STATUS", "YES"),
                ("DRIVE_PD_STATUS", "NO"),
            ] {
                ctx.state.get_diff(tile, bel, attr, val).assert_empty();
            }
        }

        let bel = "CAPTURE";
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        let d0 = ctx.state.get_diff(tile, bel, "CAPMUX", "CAP");
        assert_eq!(d0, ctx.state.get_diff(tile, bel, "CAPMUX", "1"));
        let d1 = ctx.state.get_diff(tile, bel, "CAPMUX", "CAP_B");
        assert_eq!(d1, ctx.state.get_diff(tile, bel, "CAPMUX", "0"));
        let item = xlat_bool(d0, d1);
        ctx.insert_int_inv(&[tile], tile, bel, "CAP", item);
        let item = ctx.extract_enum_bool(tile, bel, "CLKINV", "1", "0");
        ctx.insert_int_inv(&[tile], tile, bel, "CLK", item);
        ctx.state.get_diff(tile, bel, "ONESHOT", "1").assert_empty();
    }
    {
        let tile = "CNR.TL";
        let bel = "MISC";
        ctx.collect_enum(tile, bel, "TMSPIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        ctx.collect_enum(tile, bel, "TCKPIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        let item = xlat_bitvec(vec![
            !ctx.state.get_diff(tile, bel, "IBCLK_N2", "0"),
            !ctx.state.get_diff(tile, bel, "IBCLK_N4", "0"),
            !ctx.state.get_diff(tile, bel, "IBCLK_N8", "0"),
            !ctx.state.get_diff(tile, bel, "IBCLK_N16", "0"),
            !ctx.state.get_diff(tile, bel, "IBCLK_N32", "0"),
        ]);
        ctx.tiledb.insert(tile, bel, "BCLK_DIV2", item);
        for attr in ["IBCLK_N2", "IBCLK_N4", "IBCLK_N8", "IBCLK_N16", "IBCLK_N32"] {
            ctx.state.get_diff(tile, bel, attr, "1").assert_empty();
        }
        if edev.chip.kind == ChipKind::Virtex && ctx.device.name.contains("2s") {
            ctx.collect_enum(tile, bel, "POWERUP_CLK", &["USERCLK", "INTOSC", "CCLK"]);
        } else {
            for (attr, val) in [
                ("POWERUP_CLK", "USERCLK"),
                ("POWERUP_CLK", "INTOSC"),
                ("POWERUP_CLK", "CCLK"),
            ] {
                ctx.state.get_diff(tile, bel, attr, val).assert_empty();
            }
        }

        let bel = "STARTUP";
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        for attr in ["GWE_SYNC", "GSR_SYNC", "GTS_SYNC"] {
            ctx.collect_enum_bool(tile, bel, attr, "NO", "YES");
        }
        for (pinmux, pin, pin_b) in [
            ("GWEMUX", "GWE", "GWE_B"),
            ("GTSMUX", "GTS", "GTS_B"),
            ("GSRMUX", "GSR", "GSR_B"),
        ] {
            let d0 = ctx.state.get_diff(tile, bel, pinmux, pin);
            assert_eq!(d0, ctx.state.get_diff(tile, bel, pinmux, "1"));
            let d1 = ctx.state.get_diff(tile, bel, pinmux, pin_b);
            assert_eq!(d1, ctx.state.get_diff(tile, bel, pinmux, "0"));
            let item = xlat_bool(d0, d1);
            ctx.insert_int_inv(&[tile], tile, bel, pin, item);
        }
        let item = ctx.extract_enum_bool(tile, bel, "CLKINV", "1", "0");
        ctx.insert_int_inv(&[tile], tile, bel, "CLK", item);
        let item = ctx.extract_bit(tile, bel, "GSR", "1");
        ctx.tiledb.insert(tile, bel, "GSR_GTS_GWE_ENABLE", item);
        let item = ctx.extract_bit(tile, bel, "GWE", "1");
        ctx.tiledb.insert(tile, bel, "GSR_GTS_GWE_ENABLE", item);
        let item = ctx.extract_bit(tile, bel, "GTS", "1");
        ctx.tiledb.insert(tile, bel, "GSR_GTS_GWE_ENABLE", item);
        for val in ["JTAGCLK", "CCLK", "USERCLK"] {
            ctx.state
                .get_diff(tile, bel, "STARTUPCLK", val)
                .assert_empty();
        }

        let bel = "BSCAN";
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        for (pinmux, pin, pin_b) in [("TDO1MUX", "TDO1", "TDO1_B"), ("TDO2MUX", "TDO2", "TDO2_B")] {
            let d0 = ctx.state.get_diff(tile, bel, pinmux, pin);
            assert_eq!(d0, ctx.state.get_diff(tile, bel, pinmux, "1"));
            let d1 = ctx.state.get_diff(tile, bel, pinmux, pin_b);
            assert_eq!(d1, ctx.state.get_diff(tile, bel, pinmux, "0"));
            let item = xlat_bool(d0, d1);
            ctx.insert_int_inv(&[tile], tile, bel, pin, item);
        }
        ctx.collect_bitvec(tile, bel, "USERID", "");
    }
    {
        let tile = "CNR.BR";
        let bel = "MISC";
        ctx.collect_enum(tile, bel, "DONEPIN", &["PULLUP", "PULLNONE"]);
        ctx.collect_enum(tile, bel, "PROGPIN", &["PULLUP", "PULLNONE"]);
    }
    {
        let tile = "CNR.TR";
        let bel = "MISC";
        ctx.collect_enum(tile, bel, "CCLKPIN", &["PULLUP", "PULLNONE"]);
        ctx.collect_enum(tile, bel, "TDIPIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        ctx.collect_enum(tile, bel, "TDOPIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
    }
    {
        let tile = "REG.COR";
        let bel = "STARTUP";
        ctx.collect_enum(
            tile,
            bel,
            "GSR_CYCLE",
            &["1", "2", "3", "4", "5", "6", "DONE", "KEEP"],
        );
        ctx.collect_enum(
            tile,
            bel,
            "GWE_CYCLE",
            &["1", "2", "3", "4", "5", "6", "DONE", "KEEP"],
        );
        ctx.collect_enum(
            tile,
            bel,
            "GTS_CYCLE",
            &["1", "2", "3", "4", "5", "6", "DONE", "KEEP"],
        );
        ctx.collect_enum(
            tile,
            bel,
            "DONE_CYCLE",
            &["1", "2", "3", "4", "5", "6", "KEEP"],
        );
        ctx.collect_enum(
            tile,
            bel,
            "LCK_CYCLE",
            &["0", "1", "2", "3", "4", "5", "6", "NOWAIT"],
        );
        ctx.collect_enum(tile, bel, "STARTUPCLK", &["CCLK", "USERCLK", "JTAGCLK"]);
        ctx.collect_enum_ocd(
            tile,
            bel,
            "CONFIG_RATE",
            &[
                "4", "5", "7", "8", "9", "10", "13", "15", "20", "26", "30", "34", "41", "51",
                "55", "60", "130",
            ],
            OcdMode::BitOrder,
        );
        ctx.collect_enum_bool(tile, bel, "DRIVE_DONE", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "DONE_PIPE", "NO", "YES");
        ctx.tiledb.insert(
            tile,
            bel,
            "SHUTDOWN",
            TileItem::from_bit(TileBit::new(0, 0, 15), false),
        );

        let bel = "CAPTURE";
        ctx.collect_bit(tile, bel, "ONESHOT", "1");

        let tile = "REG.CTL";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "DISPMP1", "0", "1");
        ctx.collect_enum_bool(tile, bel, "DISPMP2", "0", "1");
        ctx.collect_enum(tile, bel, "SECURITY", &["NONE", "LEVEL1", "LEVEL2"]);
        // these are too much trouble to deal with the normal way.
        ctx.tiledb.insert(
            tile,
            bel,
            "PERSIST",
            TileItem::from_bit(TileBit::new(0, 0, 6), false),
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "GTS_USR_B",
            TileItem::from_bit(TileBit::new(0, 0, 0), false),
        );
    }
}
