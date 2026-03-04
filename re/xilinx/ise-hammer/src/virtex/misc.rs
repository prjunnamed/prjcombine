use prjcombine_interconnect::db::TileWireCoord;
use prjcombine_re_collector::{
    diff::OcdMode,
    legacy::{xlat_bit_bi_legacy, xlat_bitvec_legacy, xlat_bitvec_sparse_legacy},
};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::TileBit;
use prjcombine_virtex::{
    chip::ChipKind,
    defs::{bcls::GLOBAL, bslots, enums, tcls, wires},
};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::mutex::WireMutexExclusive,
    },
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    for tcid in [tcls::PCI_W, tcls::PCI_E] {
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        let mut bctx = ctx.bel(bslots::PCILOGIC);
        bctx.test_manual_legacy("PRESENT", "1")
            .mode("PCILOGIC")
            .commit();
        bctx.mode("PCILOGIC")
            .pin("I1")
            .test_enum_legacy("I1MUX", &["0", "1", "I1", "I1_B"]);
        bctx.mode("PCILOGIC")
            .pin("I2")
            .test_enum_legacy("I2MUX", &["0", "1", "I2", "I2_B"]);
    }
    let mut ctx = FuzzCtx::new_null(session, backend);
    for val in ["00", "01", "10", "11"] {
        ctx.build()
            .extra_tiles_by_bel_legacy(bslots::PCILOGIC, "PCILOGIC")
            .test_manual_legacy("PCILOGIC", "PCI_DELAY", val)
            .global("PCIDELAY", val)
            .commit();
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::CNR_SW);
        for attr in ["M0PIN", "M1PIN", "M2PIN"] {
            for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
                ctx.test_manual_legacy("MISC", attr, val)
                    .global(attr, val)
                    .commit();
            }
        }
        for attr in ["POWERDOWNPIN", "PDSTATUSPIN"] {
            for val in ["PULLUP", "PULLNONE"] {
                ctx.test_manual_legacy("MISC", attr, val)
                    .global(attr, val)
                    .commit();
            }
        }
        for val in ["NO", "YES"] {
            ctx.test_manual_legacy("MISC", "DRIVE_PD_STATUS", val)
                .global("DRIVEPDSTATUS", val)
                .commit();
        }
        for val in ["100US", "200US", "400US"] {
            ctx.test_manual_legacy("MISC", "POWERUP_DELAY", val)
                .global("POWERUPDELAY", val)
                .commit();
        }

        let mut bctx = ctx.bel(bslots::CAPTURE);
        bctx.test_manual_legacy("PRESENT", "1")
            .mode("CAPTURE")
            .commit();
        bctx.mode("CAPTURE")
            .pin("CLK")
            .test_enum_legacy("CLKINV", &["0", "1"]);
        bctx.mode("CAPTURE")
            .pin("CAP")
            .test_enum_legacy("CAPMUX", &["0", "1", "CAP", "CAP_B"]);
        bctx.mode("CAPTURE")
            .extra_tiles_by_bel_attr_bits(bslots::GLOBAL, GLOBAL::CAPTURE_ONESHOT)
            .test_manual_legacy("ONESHOT", "1")
            .attr("ONESHOT_ATTR", "ONE_SHOT")
            .commit();
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::CNR_NW);
        for attr in ["TMSPIN", "TCKPIN"] {
            for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
                ctx.test_manual_legacy("MISC", attr, val)
                    .global(attr, val)
                    .commit();
            }
        }
        for val in ["INTOSC", "USERCLK", "CCLK"] {
            ctx.test_manual_legacy("MISC", "POWERUP_CLK", val)
                .global("POWERUPCLK", val)
                .commit();
        }
        for attr in ["IBCLK_N2", "IBCLK_N4", "IBCLK_N8", "IBCLK_N16", "IBCLK_N32"] {
            for val in ["0", "1"] {
                ctx.test_manual_legacy("MISC", attr, val)
                    .global(attr, val)
                    .commit();
            }
        }

        let mut bctx = ctx.bel(bslots::STARTUP);
        bctx.test_manual_legacy("PRESENT", "1")
            .mode("STARTUP")
            .commit();
        bctx.mode("STARTUP")
            .pin("CLK")
            .test_enum_legacy("CLKINV", &["0", "1"]);
        bctx.mode("STARTUP")
            .pin("GWE")
            .test_enum_legacy("GWEMUX", &["0", "1", "GWE", "GWE_B"]);
        bctx.mode("STARTUP")
            .pin("GTS")
            .test_enum_legacy("GTSMUX", &["0", "1", "GTS", "GTS_B"]);
        bctx.mode("STARTUP")
            .pin("GSR")
            .test_enum_legacy("GSRMUX", &["0", "1", "GSR", "GSR_B"]);
        let wire_gwe = TileWireCoord::new_idx(0, wires::IMUX_STARTUP_GWE);
        let wire_gts = TileWireCoord::new_idx(0, wires::IMUX_STARTUP_GTS);
        let wire_gsr = TileWireCoord::new_idx(0, wires::IMUX_STARTUP_GSR);
        bctx.mode("STARTUP")
            .no_pin("GTS")
            .no_pin("GWE")
            .test_manual_legacy("GSR", "1")
            .prop(WireMutexExclusive::new(wire_gwe))
            .prop(WireMutexExclusive::new(wire_gts))
            .prop(WireMutexExclusive::new(wire_gsr))
            .pin("GSR")
            .attr("GSRMUX", "GSR_B")
            .commit();
        bctx.mode("STARTUP")
            .no_pin("GSR")
            .no_pin("GWE")
            .test_manual_legacy("GTS", "1")
            .prop(WireMutexExclusive::new(wire_gwe))
            .prop(WireMutexExclusive::new(wire_gts))
            .prop(WireMutexExclusive::new(wire_gsr))
            .pin("GTS")
            .attr("GTSMUX", "GTS_B")
            .commit();
        bctx.mode("STARTUP")
            .no_pin("GTS")
            .no_pin("GSR")
            .test_manual_legacy("GWE", "1")
            .prop(WireMutexExclusive::new(wire_gwe))
            .prop(WireMutexExclusive::new(wire_gts))
            .prop(WireMutexExclusive::new(wire_gsr))
            .pin("GWE")
            .attr("GWEMUX", "GWE")
            .commit();
        for val in ["NO", "YES"] {
            bctx.test_manual_legacy("GWE_SYNC", val)
                .global("GWE_SYNC", val)
                .commit();
            bctx.test_manual_legacy("GTS_SYNC", val)
                .global("GTS_SYNC", val)
                .commit();
            bctx.test_manual_legacy("GSR_SYNC", val)
                .global("GSR_SYNC", val)
                .commit();
        }
        for (val, vname) in [
            (enums::STARTUP_CLOCK::CCLK, "CCLK"),
            (enums::STARTUP_CLOCK::USERCLK, "USERCLK"),
            (enums::STARTUP_CLOCK::JTAGCLK, "JTAGCLK"),
        ] {
            bctx.mode("STARTUP")
                .pin("CLK")
                .extra_tiles_by_bel_attr_val(bslots::GLOBAL, GLOBAL::STARTUP_CLOCK, val)
                .test_manual_legacy("STARTUPCLK", vname)
                .global("STARTUPCLK", vname)
                .commit();
        }

        let mut bctx = ctx.bel(bslots::BSCAN);
        bctx.test_manual_legacy("PRESENT", "1")
            .mode("BSCAN")
            .commit();
        bctx.mode("BSCAN")
            .pin("TDO1")
            .test_enum_legacy("TDO1MUX", &["0", "1", "TDO1", "TDO1_B"]);
        bctx.mode("BSCAN")
            .pin("TDO2")
            .test_enum_legacy("TDO2MUX", &["0", "1", "TDO2", "TDO2_B"]);
        bctx.test_manual_legacy("USERID", "")
            .multi_global("USERID", MultiValue::HexPrefix, 32);
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::CNR_SE);
        for attr in ["DONEPIN", "PROGPIN"] {
            for val in ["PULLUP", "PULLNONE"] {
                ctx.test_manual_legacy("MISC", attr, val)
                    .global(attr, val)
                    .commit();
            }
        }
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::CNR_NE);
        for attr in ["TDIPIN", "TDOPIN"] {
            for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
                ctx.test_manual_legacy("MISC", attr, val)
                    .global(attr, val)
                    .commit();
            }
        }
        for val in ["PULLUP", "PULLNONE"] {
            ctx.test_manual_legacy("MISC", "CCLKPIN", val)
                .global("CCLKPIN", val)
                .commit();
        }
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::GLOBAL);
        let mut bctx = ctx.bel(bslots::GLOBAL);
        // COR
        for (val, vname) in [
            (enums::STARTUP_CYCLE::_1, "1"),
            (enums::STARTUP_CYCLE::_2, "2"),
            (enums::STARTUP_CYCLE::_3, "3"),
            (enums::STARTUP_CYCLE::_4, "4"),
            (enums::STARTUP_CYCLE::_5, "5"),
            (enums::STARTUP_CYCLE::_6, "6"),
            (enums::STARTUP_CYCLE::DONE, "DONE"),
            (enums::STARTUP_CYCLE::KEEP, "KEEP"),
        ] {
            bctx.build()
                .test_bel_attr_val(GLOBAL::GSR_CYCLE, val)
                .global("GSR_CYCLE", vname)
                .commit();
            bctx.build()
                .test_bel_attr_val(GLOBAL::GWE_CYCLE, val)
                .global("GWE_CYCLE", vname)
                .commit();
            bctx.build()
                .test_bel_attr_val(GLOBAL::GTS_CYCLE, val)
                .global("GTS_CYCLE", vname)
                .commit();
            if val != enums::STARTUP_CYCLE::DONE {
                bctx.build()
                    .test_bel_attr_val(GLOBAL::DONE_CYCLE, val)
                    .global("DONE_CYCLE", vname)
                    .commit();
            }
        }
        for (val, vname) in [
            (enums::STARTUP_CYCLE::_0, "0"),
            (enums::STARTUP_CYCLE::_1, "1"),
            (enums::STARTUP_CYCLE::_2, "2"),
            (enums::STARTUP_CYCLE::_3, "3"),
            (enums::STARTUP_CYCLE::_4, "4"),
            (enums::STARTUP_CYCLE::_5, "5"),
            (enums::STARTUP_CYCLE::_6, "6"),
            (enums::STARTUP_CYCLE::NOWAIT, "NOWAIT"),
        ] {
            bctx.build()
                .test_bel_attr_val(GLOBAL::LOCK_CYCLE, val)
                .global("LCK_CYCLE", vname)
                .commit();
        }
        bctx.build()
            .test_global_attr_bool_rename("DRIVEDONE", GLOBAL::DRIVE_DONE, "NO", "YES");
        bctx.build()
            .test_global_attr_bool_rename("DONEPIPE", GLOBAL::DONE_PIPE, "NO", "YES");
        bctx.build()
            .test_global_attr_rename("CONFIGRATE", GLOBAL::CONFIG_RATE);

        // CTL
        bctx.build()
            .test_global_attr_rename("SECURITY", GLOBAL::SECURITY);
        bctx.build()
            .test_global_attr_bool_rename("DISPMP1", GLOBAL::DISPMP1, "0", "1");
        bctx.build()
            .test_global_attr_bool_rename("DISPMP2", GLOBAL::DISPMP2, "0", "1");
        // persist not fuzzed — too much effort
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex(edev) = ctx.edev else {
        unreachable!()
    };
    for tile in ["PCI_W", "PCI_E"] {
        let bel = "PCILOGIC";
        let mut present = ctx.get_diff_legacy(tile, bel, "PRESENT", "1");
        for (pinmux, pin, pin_b) in [("I1MUX", "I1", "I1_B"), ("I2MUX", "I2", "I2_B")] {
            // this is different from other virtex muxes!
            let d0 = ctx.get_diff_legacy(tile, bel, pinmux, pin);
            assert_eq!(d0, ctx.get_diff_legacy(tile, bel, pinmux, "0"));
            let d1 = ctx.get_diff_legacy(tile, bel, pinmux, pin_b);
            assert_eq!(d1, ctx.get_diff_legacy(tile, bel, pinmux, "1"));
            let item = xlat_bit_bi_legacy(d0, d1);
            present.discard_bits_legacy(&item);
            ctx.insert_legacy(tile, bel, format!("INV.{pin}"), item);
        }
        present.assert_empty();
        if edev.chip.kind == ChipKind::Virtex {
            let d0 = ctx.get_diff_legacy(tile, bel, "PCI_DELAY", "00");
            let d1 = ctx.get_diff_legacy(tile, bel, "PCI_DELAY", "01");
            let d2 = ctx.get_diff_legacy(tile, bel, "PCI_DELAY", "10");
            let d3 = ctx.get_diff_legacy(tile, bel, "PCI_DELAY", "11");
            // bug? bug.
            assert_eq!(d0, d1);
            ctx.insert_legacy(
                tile,
                bel,
                "PCI_DELAY",
                xlat_bitvec_sparse_legacy(vec![(0, d0), (2, d2), (3, d3)]),
            );
        } else {
            for val in ["00", "01", "10", "11"] {
                ctx.get_diff_legacy(tile, bel, "PCI_DELAY", val)
                    .assert_empty();
            }
        }
    }
    {
        let tile = "CNR_SW";
        let bel = "MISC";
        ctx.collect_enum_legacy(tile, bel, "M0PIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        ctx.collect_enum_legacy(tile, bel, "M1PIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        ctx.collect_enum_legacy(tile, bel, "M2PIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        if edev.chip.kind == ChipKind::Virtex && ctx.device.name.contains("2s") {
            ctx.collect_enum_legacy(tile, bel, "POWERDOWNPIN", &["PULLUP", "PULLNONE"]);
            ctx.collect_enum_legacy(tile, bel, "PDSTATUSPIN", &["PULLUP", "PULLNONE"]);
            ctx.collect_enum_legacy(tile, bel, "POWERUP_DELAY", &["100US", "200US", "400US"]);
            ctx.collect_bit_bi_legacy(tile, bel, "DRIVE_PD_STATUS", "NO", "YES");
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
                ctx.get_diff_legacy(tile, bel, attr, val).assert_empty();
            }
        }

        let bel = "CAPTURE";
        ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
            .assert_empty();
        let d0 = ctx.get_diff_legacy(tile, bel, "CAPMUX", "CAP");
        assert_eq!(d0, ctx.get_diff_legacy(tile, bel, "CAPMUX", "1"));
        let d1 = ctx.get_diff_legacy(tile, bel, "CAPMUX", "CAP_B");
        assert_eq!(d1, ctx.get_diff_legacy(tile, bel, "CAPMUX", "0"));
        let item = xlat_bit_bi_legacy(d0, d1);
        ctx.insert_legacy(tile, bel, "INV.CAP", item);
        let item = ctx.extract_bit_bi_legacy(tile, bel, "CLKINV", "1", "0");
        ctx.insert_legacy(tile, bel, "INV.CLK", item);
        ctx.get_diff_legacy(tile, bel, "ONESHOT", "1")
            .assert_empty();
    }
    {
        let tile = "CNR_NW";
        let bel = "MISC";
        ctx.collect_enum_legacy(tile, bel, "TMSPIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        ctx.collect_enum_legacy(tile, bel, "TCKPIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        let item = xlat_bitvec_legacy(vec![
            !ctx.get_diff_legacy(tile, bel, "IBCLK_N2", "0"),
            !ctx.get_diff_legacy(tile, bel, "IBCLK_N4", "0"),
            !ctx.get_diff_legacy(tile, bel, "IBCLK_N8", "0"),
            !ctx.get_diff_legacy(tile, bel, "IBCLK_N16", "0"),
            !ctx.get_diff_legacy(tile, bel, "IBCLK_N32", "0"),
        ]);
        ctx.insert_legacy(tile, bel, "BCLK_DIV2", item);
        for attr in ["IBCLK_N2", "IBCLK_N4", "IBCLK_N8", "IBCLK_N16", "IBCLK_N32"] {
            ctx.get_diff_legacy(tile, bel, attr, "1").assert_empty();
        }
        if edev.chip.kind == ChipKind::Virtex && ctx.device.name.contains("2s") {
            ctx.collect_enum_legacy(tile, bel, "POWERUP_CLK", &["USERCLK", "INTOSC", "CCLK"]);
        } else {
            for (attr, val) in [
                ("POWERUP_CLK", "USERCLK"),
                ("POWERUP_CLK", "INTOSC"),
                ("POWERUP_CLK", "CCLK"),
            ] {
                ctx.get_diff_legacy(tile, bel, attr, val).assert_empty();
            }
        }

        let bel = "STARTUP";
        ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
            .assert_empty();
        for attr in ["GWE_SYNC", "GSR_SYNC", "GTS_SYNC"] {
            ctx.collect_bit_bi_legacy(tile, bel, attr, "NO", "YES");
        }
        for (pinmux, pin, pin_b) in [
            ("GWEMUX", "GWE", "GWE_B"),
            ("GTSMUX", "GTS", "GTS_B"),
            ("GSRMUX", "GSR", "GSR_B"),
        ] {
            let d0 = ctx.get_diff_legacy(tile, bel, pinmux, pin);
            assert_eq!(d0, ctx.get_diff_legacy(tile, bel, pinmux, "1"));
            let d1 = ctx.get_diff_legacy(tile, bel, pinmux, pin_b);
            assert_eq!(d1, ctx.get_diff_legacy(tile, bel, pinmux, "0"));
            let item = xlat_bit_bi_legacy(d0, d1);
            ctx.insert_legacy(tile, bel, format!("INV.{pin}"), item);
        }
        let item = ctx.extract_bit_bi_legacy(tile, bel, "CLKINV", "1", "0");
        ctx.insert_legacy(tile, bel, "INV.CLK", item);
        let item = ctx.extract_bit_legacy(tile, bel, "GSR", "1");
        ctx.insert_legacy(tile, bel, "GSR_GTS_GWE_ENABLE", item);
        let item = ctx.extract_bit_legacy(tile, bel, "GWE", "1");
        ctx.insert_legacy(tile, bel, "GSR_GTS_GWE_ENABLE", item);
        let item = ctx.extract_bit_legacy(tile, bel, "GTS", "1");
        ctx.insert_legacy(tile, bel, "GSR_GTS_GWE_ENABLE", item);
        for val in ["JTAGCLK", "CCLK", "USERCLK"] {
            ctx.get_diff_legacy(tile, bel, "STARTUPCLK", val)
                .assert_empty();
        }

        let bel = "BSCAN";
        ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
            .assert_empty();
        for (pinmux, pin, pin_b) in [("TDO1MUX", "TDO1", "TDO1_B"), ("TDO2MUX", "TDO2", "TDO2_B")] {
            let d0 = ctx.get_diff_legacy(tile, bel, pinmux, pin);
            assert_eq!(d0, ctx.get_diff_legacy(tile, bel, pinmux, "1"));
            let d1 = ctx.get_diff_legacy(tile, bel, pinmux, pin_b);
            assert_eq!(d1, ctx.get_diff_legacy(tile, bel, pinmux, "0"));
            let item = xlat_bit_bi_legacy(d0, d1);
            ctx.insert_legacy(tile, bel, format!("INV.{pin}"), item);
        }
        ctx.collect_bitvec_legacy(tile, bel, "USERID", "");
    }
    {
        let tile = "CNR_SE";
        let bel = "MISC";
        ctx.collect_enum_legacy(tile, bel, "DONEPIN", &["PULLUP", "PULLNONE"]);
        ctx.collect_enum_legacy(tile, bel, "PROGPIN", &["PULLUP", "PULLNONE"]);
    }
    {
        let tile = "CNR_NE";
        let bel = "MISC";
        ctx.collect_enum_legacy(tile, bel, "CCLKPIN", &["PULLUP", "PULLNONE"]);
        ctx.collect_enum_legacy(tile, bel, "TDIPIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
        ctx.collect_enum_legacy(tile, bel, "TDOPIN", &["PULLDOWN", "PULLUP", "PULLNONE"]);
    }

    {
        let tcid = tcls::GLOBAL;
        let bslot = bslots::GLOBAL;

        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            GLOBAL::GSR_CYCLE,
            &[
                enums::STARTUP_CYCLE::_1,
                enums::STARTUP_CYCLE::_2,
                enums::STARTUP_CYCLE::_3,
                enums::STARTUP_CYCLE::_4,
                enums::STARTUP_CYCLE::_5,
                enums::STARTUP_CYCLE::_6,
                enums::STARTUP_CYCLE::DONE,
                enums::STARTUP_CYCLE::KEEP,
            ],
        );
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            GLOBAL::GWE_CYCLE,
            &[
                enums::STARTUP_CYCLE::_1,
                enums::STARTUP_CYCLE::_2,
                enums::STARTUP_CYCLE::_3,
                enums::STARTUP_CYCLE::_4,
                enums::STARTUP_CYCLE::_5,
                enums::STARTUP_CYCLE::_6,
                enums::STARTUP_CYCLE::DONE,
                enums::STARTUP_CYCLE::KEEP,
            ],
        );
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            GLOBAL::GTS_CYCLE,
            &[
                enums::STARTUP_CYCLE::_1,
                enums::STARTUP_CYCLE::_2,
                enums::STARTUP_CYCLE::_3,
                enums::STARTUP_CYCLE::_4,
                enums::STARTUP_CYCLE::_5,
                enums::STARTUP_CYCLE::_6,
                enums::STARTUP_CYCLE::DONE,
                enums::STARTUP_CYCLE::KEEP,
            ],
        );
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            GLOBAL::DONE_CYCLE,
            &[
                enums::STARTUP_CYCLE::_1,
                enums::STARTUP_CYCLE::_2,
                enums::STARTUP_CYCLE::_3,
                enums::STARTUP_CYCLE::_4,
                enums::STARTUP_CYCLE::_5,
                enums::STARTUP_CYCLE::_6,
                enums::STARTUP_CYCLE::KEEP,
            ],
        );
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            GLOBAL::LOCK_CYCLE,
            &[
                enums::STARTUP_CYCLE::_0,
                enums::STARTUP_CYCLE::_1,
                enums::STARTUP_CYCLE::_2,
                enums::STARTUP_CYCLE::_3,
                enums::STARTUP_CYCLE::_4,
                enums::STARTUP_CYCLE::_5,
                enums::STARTUP_CYCLE::_6,
                enums::STARTUP_CYCLE::NOWAIT,
            ],
        );
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::STARTUP_CLOCK);
        ctx.insert_bel_attr_bool(tcid, bslot, GLOBAL::SHUTDOWN, TileBit::new(0, 0, 15).pos());
        ctx.collect_bel_attr_ocd(tcid, bslot, GLOBAL::CONFIG_RATE, OcdMode::BitOrder);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::DRIVE_DONE);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::DONE_PIPE);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::CAPTURE_ONESHOT);

        // CTL
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::DISPMP1);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::DISPMP2);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::SECURITY);
        // these are too much trouble to deal with the normal way.
        ctx.insert_bel_attr_bool(tcid, bslot, GLOBAL::PERSIST, TileBit::new(1, 0, 6).pos());
        ctx.insert_bel_attr_bool(tcid, bslot, GLOBAL::GTS_USR_B, TileBit::new(1, 0, 0).pos());
    }
}
