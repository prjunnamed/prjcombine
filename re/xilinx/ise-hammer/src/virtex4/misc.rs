use prjcombine_interconnect::grid::TileCoord;
use prjcombine_re_collector::{
    diff::OcdMode,
    legacy::{xlat_bitvec_legacy, xlat_enum_legacy_ocd},
};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{
    bits,
    bsdata::{TileBit, TileItem, TileItemKind},
};
use prjcombine_virtex4::defs::{self, bslots, virtex4::tcls};
use prjcombine_xilinx_bitstream::Reg;

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::relation::TileRelation,
    },
};

#[derive(Clone, Copy, Debug)]
struct HclkDcm;

impl TileRelation for HclkDcm {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        Some(
            tcrd.cell
                .with_cr(edev.col_clk, edev.chips[tcrd.die].row_hclk(tcrd.row))
                .tile(defs::tslots::HCLK_BEL),
        )
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    {
        let mut ctx = FuzzCtx::new_legacy(session, backend, "CFG");
        for val in ["0", "1", "2", "3"] {
            ctx.test_manual_legacy("MISC", "PROBESEL", val)
                .global("PROBESEL", val)
                .commit();
        }
        for attr in ["CCLKPIN", "DONEPIN", "POWERDOWNPIN", "PROGPIN", "INITPIN"] {
            for val in ["PULLUP", "PULLNONE"] {
                ctx.test_manual_legacy("MISC", attr, val)
                    .global(attr, val)
                    .commit();
            }
        }
        for attr in [
            "HSWAPENPIN",
            "M0PIN",
            "M1PIN",
            "M2PIN",
            "CSPIN",
            "DINPIN",
            "BUSYPIN",
            "RDWRPIN",
            "TCKPIN",
            "TDIPIN",
            "TDOPIN",
            "TMSPIN",
        ] {
            for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
                ctx.test_manual_legacy("MISC", attr, val)
                    .global(attr, val)
                    .commit();
            }
        }

        for i in 0..4 {
            let mut bctx = ctx.bel(defs::bslots::BSCAN[i]);
            bctx.test_manual_legacy("PRESENT", "1")
                .mode("BSCAN")
                .commit();
        }
        ctx.test_manual_legacy("BSCAN_COMMON", "USERID", "")
            .multi_global("USERID", MultiValue::HexPrefix, 32);

        for i in 0..2 {
            let mut bctx = ctx.bel(defs::bslots::ICAP[i]);
            let obel = defs::bslots::ICAP[i ^ 1];
            bctx.build()
                .bel_unused(obel)
                .test_manual_legacy("PRESENT", "1")
                .mode("ICAP")
                .commit();
            bctx.mode("ICAP").test_inv_legacy("CLK");
            bctx.mode("ICAP").test_inv_legacy("CE");
            bctx.mode("ICAP").test_inv_legacy("WRITE");
            bctx.mode("ICAP")
                .bel_unused(obel)
                .test_enum_legacy("ICAP_WIDTH", &["X8", "X32"]);
        }

        {
            let mut bctx = ctx.bel(defs::bslots::PMV_CFG[0]);
            bctx.build()
                .null_bits()
                .test_manual_legacy("PRESENT", "1")
                .mode("PMV")
                .commit();
        }

        {
            let mut bctx = ctx.bel(defs::bslots::STARTUP);
            bctx.test_manual_legacy("PRESENT", "1")
                .mode("STARTUP")
                .commit();
            for pin in [
                "CLK",
                "GTS",
                "GSR",
                "USRCCLKTS",
                "USRCCLKO",
                "USRDONETS",
                "USRDONEO",
            ] {
                bctx.mode("STARTUP").test_inv_legacy(pin);
            }
            bctx.mode("STARTUP")
                .no_pin("GSR")
                .test_manual_legacy("PIN.GTS", "1")
                .pin("GTS")
                .commit();
            bctx.mode("STARTUP")
                .no_pin("GTS")
                .test_manual_legacy("PIN.GSR", "1")
                .pin("GSR")
                .commit();
            bctx.mode("STARTUP")
                .test_manual_legacy("PIN.USRCCLKO", "1")
                .pin("USRCCLKO")
                .commit();
            for attr in ["GSR_SYNC", "GWE_SYNC", "GTS_SYNC"] {
                for val in ["YES", "NO"] {
                    bctx.test_manual_legacy(attr, val)
                        .global(attr, val)
                        .commit();
                }
            }
            for val in ["CCLK", "USERCLK", "JTAGCLK"] {
                bctx.mode("STARTUP")
                    .pin("CLK")
                    .null_bits()
                    .extra_tile_reg(Reg::Cor0, "REG.COR", "STARTUP")
                    .test_manual_legacy("STARTUPCLK", val)
                    .global("STARTUPCLK", val)
                    .commit();
            }
        }

        {
            let mut bctx = ctx.bel(defs::bslots::JTAGPPC);
            bctx.test_manual_legacy("PRESENT", "1")
                .mode("JTAGPPC")
                .commit();
        }

        {
            let mut bctx = ctx.bel(defs::bslots::FRAME_ECC);
            bctx.build()
                .null_bits()
                .test_manual_legacy("PRESENT", "1")
                .mode("FRAME_ECC")
                .commit();
        }

        {
            let mut bctx = ctx.bel(defs::bslots::DCIRESET);
            bctx.test_manual_legacy("PRESENT", "1")
                .mode("DCIRESET")
                .commit();
        }

        {
            let mut bctx = ctx.bel(defs::bslots::CAPTURE);
            bctx.test_manual_legacy("PRESENT", "1")
                .mode("CAPTURE")
                .commit();
            bctx.mode("CAPTURE").test_inv_legacy("CLK");
            bctx.mode("CAPTURE").test_inv_legacy("CAP");
            bctx.mode("CAPTURE")
                .null_bits()
                .extra_tile_reg(Reg::Cor0, "REG.COR", "CAPTURE")
                .test_enum_legacy("ONESHOT", &["FALSE", "TRUE"]);
        }

        {
            let mut bctx = ctx.bel(defs::bslots::USR_ACCESS);
            bctx.build()
                .null_bits()
                .test_manual_legacy("PRESENT", "1")
                .mode("USR_ACCESS")
                .commit();
        }
    }

    {
        let mut ctx = FuzzCtx::new_null(session, backend);
        for val in ["1", "2", "3", "4", "5", "6", "DONE", "KEEP"] {
            ctx.test_reg_legacy(Reg::Cor0, "REG.COR", "STARTUP", "GWE_CYCLE", val)
                .global("GWE_CYCLE", val)
                .commit();
            ctx.test_reg_legacy(Reg::Cor0, "REG.COR", "STARTUP", "GTS_CYCLE", val)
                .global("GTS_CYCLE", val)
                .commit();
        }
        for val in ["1", "2", "3", "4", "5", "6", "KEEP"] {
            ctx.test_reg_legacy(Reg::Cor0, "REG.COR", "STARTUP", "DONE_CYCLE", val)
                .global("DONE_CYCLE", val)
                .commit();
        }
        for val in ["0", "1", "2", "3", "4", "5", "6", "NOWAIT"] {
            ctx.test_reg_legacy(Reg::Cor0, "REG.COR", "STARTUP", "LCK_CYCLE", val)
                .global("LCK_CYCLE", val)
                .commit();
            ctx.build()
                .global_mutex("GLOBAL_DCI", "NO")
                .test_reg(Reg::Cor0, "REG.COR", "STARTUP", "MATCH_CYCLE", val)
                .global("MATCH_CYCLE", val)
                .commit();
        }
        for val in ["NO", "YES"] {
            ctx.test_reg_legacy(Reg::Cor0, "REG.COR", "STARTUP", "DRIVE_DONE", val)
                .global("DRIVEDONE", val)
                .commit();
            ctx.test_reg_legacy(Reg::Cor0, "REG.COR", "STARTUP", "DONE_PIPE", val)
                .global("DONEPIPE", val)
                .commit();
        }
        for val in [
            "4", "5", "7", "8", "9", "10", "13", "15", "20", "26", "30", "34", "41", "51", "55",
            "60", "130",
        ] {
            ctx.test_reg_legacy(Reg::Cor0, "REG.COR", "STARTUP", "CONFIG_RATE", val)
                .global("CONFIGRATE", val)
                .commit();
        }
        for val in ["DISABLE", "ENABLE"] {
            ctx.test_reg_legacy(Reg::Cor0, "REG.COR", "STARTUP", "CRC", val)
                .global("CRC", val)
                .commit();
            ctx.test_reg_legacy(Reg::Cor0, "REG.COR", "STARTUP", "DCM_SHUTDOWN", val)
                .global("DCMSHUTDOWN", val)
                .commit();
            ctx.test_reg_legacy(Reg::Cor0, "REG.COR", "STARTUP", "POWERDOWN_STATUS", val)
                .global("POWERDOWNSTATUS", val)
                .commit();
        }
    }

    {
        let mut ctx = FuzzCtx::new_null(session, backend);
        for val in ["NO", "YES"] {
            ctx.test_reg_legacy(Reg::Ctl0, "REG.CTL", "MISC", "GTS_USR_B", val)
                .global("GTS_USR_B", val)
                .commit();
            ctx.test_reg_legacy(Reg::Ctl0, "REG.CTL", "MISC", "VGG_TEST", val)
                .global("VGG_TEST", val)
                .commit();
            ctx.test_reg_legacy(Reg::Ctl0, "REG.CTL", "MISC", "EN_VTEST", val)
                .global("EN_VTEST", val)
                .commit();
            ctx.test_reg_legacy(Reg::Ctl0, "REG.CTL", "MISC", "ENCRYPT", val)
                .global("ENCRYPT", val)
                .commit();
        }
        // persist not fuzzed — too much effort
        for val in ["NONE", "LEVEL1", "LEVEL2"] {
            ctx.test_reg_legacy(Reg::Ctl0, "REG.CTL", "MISC", "SECURITY", val)
                .global("SECURITY", val)
                .commit();
        }
    }

    {
        // TODO: more crap
        let mut ctx = FuzzCtx::new_null(session, backend);
        for val in ["NO", "YES"] {
            ctx.test_manual_legacy("NULL", "DISABLE_BANDGAP", val)
                .global("DISABLEBANDGAP", val)
                .commit();
        }
        for val in ["DISABLE", "ENABLE"] {
            ctx.test_manual_legacy("NULL", "DCI_SHUTDOWN", val)
                .global("DCISHUTDOWN", val)
                .commit();
        }
    }

    if let Some(mut ctx) = FuzzCtx::try_new_legacy(session, backend, "SYSMON") {
        let mut bctx = ctx.bel(defs::bslots::SYSMON);
        let mode = "MONITOR";
        bctx.test_manual_legacy("PRESENT", "1").mode(mode).commit();
        for i in 0x40..0x70 {
            bctx.mode(mode)
                .test_multi_attr_hex_legacy(format!("INIT_{i:02X}"), 16);
        }
        bctx.mode(mode)
            .global_mutex("MONITOR_GLOBAL", "NONE")
            .test_enum_legacy("MONITOR_MODE", &["TEST", "MONITOR", "ADC"]);
        for pin in [
            "DEN",
            // DCLK?
            "DWE",
            "SCANTESTENA",
            "SCANTESTENB",
            // SCANMEMCLK?
            "SCANMEMWE",
            "ROMTESTENABLE",
            "RST",
            "CONVST",
            // SCLK[AB]?
            "SEA",
            "SEB",
        ] {
            bctx.mode(mode).test_inv_legacy(pin);
        }
        for (attr, len) in [
            ("DCLK_DIVID_2", 1),
            ("LW_DIVID_2_4", 1),
            ("MCCLK_DIVID", 8),
            ("OVER_TEMPERATURE", 10),
            ("OVER_TEMPERATURE_OFF", 1),
            ("OVER_TEMPERATURE_DELAY", 8),
            ("BLOCK_ENABLE", 5),
            ("DCLK_MISSING", 10),
            ("FEATURE_ENABLE", 8),
            ("PROM_DATA", 8),
        ] {
            bctx.mode(mode)
                .global_mutex_here("MONITOR_GLOBAL")
                .attr("MONITOR_MODE", "ADC")
                .test_manual_legacy(attr, "")
                .multi_global(format!("ADC_{attr}"), MultiValue::Bin, len);
        }
        for out in ["CONVST", "CONVST_TEST"] {
            bctx.build()
                .mutex("CONVST_OUT", out)
                .mutex("CONVST_IN", "INT_CLK")
                .test_manual_legacy(out, "INT_CLK")
                .pip(out, "CONVST_INT_CLK")
                .commit();
            bctx.build()
                .mutex("CONVST_OUT", out)
                .mutex("CONVST_IN", "INT_IMUX")
                .test_manual_legacy(out, "INT_IMUX")
                .pip(out, "CONVST_INT_IMUX")
                .commit();
            for i in 0..16 {
                bctx.build()
                    .mutex("CONVST_OUT", out)
                    .mutex("CONVST_IN", format!("GIOB{i}"))
                    .related_tile_mutex(HclkDcm, "HCLK_DCM", "USE")
                    .related_pip(
                        HclkDcm,
                        (defs::bslots::HCLK_DCM, format!("GIOB_O_D{i}")),
                        (defs::bslots::HCLK_DCM, format!("GIOB_I{i}")),
                    )
                    .related_pip(
                        HclkDcm,
                        (defs::bslots::HCLK_DCM, format!("GIOB_O_U{i}")),
                        (defs::bslots::HCLK_DCM, format!("GIOB_I{i}")),
                    )
                    .test_manual_legacy(out, format!("GIOB{i}"))
                    .pip(out, format!("GIOB{i}"))
                    .commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::CFG;
    let tile = "CFG";
    let bel = "MISC";
    ctx.collect_enum_default_legacy(tile, bel, "PROBESEL", &["0", "1", "2", "3"], "NONE");
    for attr in ["CCLKPIN", "DONEPIN", "POWERDOWNPIN", "PROGPIN", "INITPIN"] {
        ctx.collect_enum_legacy(tile, bel, attr, &["PULLUP", "PULLNONE"]);
    }
    for attr in [
        "HSWAPENPIN",
        "M0PIN",
        "M1PIN",
        "M2PIN",
        "CSPIN",
        "DINPIN",
        "BUSYPIN",
        "RDWRPIN",
        "TCKPIN",
        "TDIPIN",
        "TDOPIN",
        "TMSPIN",
    ] {
        ctx.collect_enum_legacy(tile, bel, attr, &["PULLUP", "PULLDOWN", "PULLNONE"]);
    }

    for bel in [
        "BSCAN[0]", "BSCAN[1]", "BSCAN[2]", "BSCAN[3]", "JTAGPPC", "DCIRESET", "ICAP[0]", "ICAP[1]",
    ] {
        let item = ctx.extract_bit_legacy(tile, bel, "PRESENT", "1");
        ctx.insert_legacy(tile, bel, "ENABLE", item);
    }

    let bel = "BSCAN_COMMON";
    let item = xlat_bitvec_legacy(ctx.get_diffs_legacy(tile, bel, "USERID", ""));
    ctx.insert_legacy(tile, bel, "USERID", item);

    let bel = "STARTUP";
    let bslot = bslots::STARTUP;
    ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
        .assert_empty();
    ctx.collect_bit_bi_legacy(tile, bel, "GSR_SYNC", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "GWE_SYNC", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "GTS_SYNC", "NO", "YES");
    for pin in [
        "CLK",
        "GSR",
        "USRDONETS",
        "USRDONEO",
        "USRCCLKTS",
        "USRCCLKO",
    ] {
        ctx.collect_int_inv_legacy(&[tcls::INT; 16], tcid, bslot, pin, false);
    }
    ctx.collect_int_inv_legacy(&[tcls::INT; 16], tcid, bslot, "GTS", true);
    let item0 = ctx.extract_bit_legacy(tile, bel, "PIN.GSR", "1");
    let item1 = ctx.extract_bit_legacy(tile, bel, "PIN.GTS", "1");
    assert_eq!(item0, item1);
    ctx.insert_legacy(tile, "STARTUP", "GTS_GSR_ENABLE", item0);
    let item = ctx.extract_bit_legacy(tile, bel, "PIN.USRCCLKO", "1");
    ctx.insert_legacy(tile, "STARTUP", "USRCCLK_ENABLE", item);

    let item0 = ctx.extract_enum_legacy(tile, "ICAP[0]", "ICAP_WIDTH", &["X8", "X32"]);
    let item1 = ctx.extract_enum_legacy(tile, "ICAP[1]", "ICAP_WIDTH", &["X8", "X32"]);
    assert_eq!(item0, item1);
    ctx.insert_legacy(tile, "ICAP_COMMON", "ICAP_WIDTH", item0);
    for bslot in bslots::ICAP {
        for pin in ["CLK", "CE", "WRITE"] {
            ctx.collect_int_inv_legacy(&[tcls::INT; 16], tcid, bslot, pin, false);
        }
    }

    let bel = "CAPTURE";
    let bslot = bslots::CAPTURE;
    ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
        .assert_empty();
    ctx.collect_int_inv_legacy(&[tcls::INT; 16], tcid, bslot, "CLK", false);
    ctx.collect_int_inv_legacy(&[tcls::INT; 16], tcid, bslot, "CAP", true);

    // config regs

    let tile = "REG.COR";
    let bel = "STARTUP";
    ctx.collect_enum_legacy(
        tile,
        bel,
        "GWE_CYCLE",
        &["1", "2", "3", "4", "5", "6", "DONE", "KEEP"],
    );
    ctx.collect_enum_legacy(
        tile,
        bel,
        "GTS_CYCLE",
        &["1", "2", "3", "4", "5", "6", "DONE", "KEEP"],
    );
    ctx.collect_enum_legacy(
        tile,
        bel,
        "DONE_CYCLE",
        &["1", "2", "3", "4", "5", "6", "KEEP"],
    );
    ctx.collect_enum_legacy(
        tile,
        bel,
        "LCK_CYCLE",
        &["0", "1", "2", "3", "4", "5", "6", "NOWAIT"],
    );
    ctx.collect_enum_legacy(
        tile,
        bel,
        "MATCH_CYCLE",
        &["0", "1", "2", "3", "4", "5", "6", "NOWAIT"],
    );
    ctx.collect_enum_legacy(tile, bel, "STARTUPCLK", &["CCLK", "USERCLK", "JTAGCLK"]);
    ctx.collect_enum_legacy_ocd(
        tile,
        bel,
        "CONFIG_RATE",
        &[
            "4", "5", "7", "8", "9", "10", "13", "15", "20", "26", "30", "34", "41", "51", "55",
            "60", "130",
        ],
        OcdMode::BitOrder,
    );
    ctx.collect_bit_bi_legacy(tile, bel, "DRIVE_DONE", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "DONE_PIPE", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "DCM_SHUTDOWN", "DISABLE", "ENABLE");
    ctx.collect_bit_bi_legacy(tile, bel, "POWERDOWN_STATUS", "DISABLE", "ENABLE");
    ctx.collect_bit_bi_legacy(tile, bel, "CRC", "DISABLE", "ENABLE");
    let bel = "CAPTURE";
    ctx.collect_bit_bi_legacy(tile, bel, "ONESHOT", "FALSE", "TRUE");

    let tile = "REG.CTL";
    let bel = "MISC";
    ctx.collect_bit_bi_legacy(tile, bel, "GTS_USR_B", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "VGG_TEST", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "EN_VTEST", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "ENCRYPT", "NO", "YES");
    ctx.collect_enum_legacy(tile, bel, "SECURITY", &["NONE", "LEVEL1", "LEVEL2"]);
    // these are too much trouble to deal with the normal way.
    ctx.insert_legacy(
        tile,
        bel,
        "PERSIST",
        TileItem {
            bits: vec![TileBit::new(0, 0, 3)],
            kind: TileItemKind::BitVec { invert: bits![0] },
        },
    );
    ctx.insert_legacy(
        tile,
        bel,
        "GLUTMASK",
        TileItem {
            bits: vec![TileBit::new(0, 0, 8)],
            kind: TileItemKind::BitVec { invert: bits![1] },
        },
    );
    ctx.insert_legacy(
        tile,
        bel,
        "ICAP_SELECT",
        TileItem {
            bits: vec![TileBit::new(0, 0, 30)],
            kind: TileItemKind::Enum {
                values: [
                    ("TOP".to_string(), bits![0]),
                    ("BOTTOM".to_string(), bits![1]),
                ]
                .into_iter()
                .collect(),
            },
        },
    );

    if ctx.has_tcls(tcls::SYSMON) {
        let tcid = tcls::SYSMON;
        let tile = "SYSMON";
        let bslot = bslots::SYSMON;
        let bel = "SYSMON";
        ctx.collect_enum_legacy(tile, bel, "MONITOR_MODE", &["TEST", "MONITOR", "ADC"]);
        for i in 0x40..0x70 {
            ctx.collect_bitvec_legacy(tile, bel, &format!("INIT_{i:02X}"), "");
        }
        for pin in [
            "DEN",
            "DWE",
            "SCANTESTENA",
            "SCANTESTENB",
            "SCANMEMWE",
            "ROMTESTENABLE",
            "RST",
            "SEA",
            "SEB",
        ] {
            ctx.collect_int_inv_legacy(&[tcls::INT; 8], tcid, bslot, pin, false);
        }
        ctx.collect_inv_legacy(tile, bel, "CONVST");
        let mut present = ctx.get_diff_legacy(tile, bel, "PRESENT", "1");
        for (attr, val) in [
            ("DCLK_DIVID_2", 0),
            ("LW_DIVID_2_4", 0),
            ("MCCLK_DIVID", 0xc8),
            ("OVER_TEMPERATURE", 0x31e),
            ("OVER_TEMPERATURE_OFF", 0),
            ("OVER_TEMPERATURE_DELAY", 0),
            ("BLOCK_ENABLE", 0x1e),
            ("DCLK_MISSING", 0x320),
            ("FEATURE_ENABLE", 0),
            ("PROM_DATA", 0),
        ] {
            ctx.collect_bitvec_legacy(tile, bel, attr, "");
            present.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, attr), val, 0);
        }
        present.assert_empty();

        let mut diffs = vec![];
        let diff = ctx.get_diff_legacy(tile, bel, "CONVST", "INT_IMUX");
        assert_eq!(
            diff,
            ctx.get_diff_legacy(tile, bel, "CONVST_TEST", "INT_IMUX")
        );
        diffs.push(("INT_IMUX".to_string(), diff));
        let mut diff = ctx.get_diff_legacy(tile, bel, "CONVST", "INT_CLK");
        assert_eq!(
            diff,
            ctx.get_diff_legacy(tile, bel, "CONVST_TEST", "INT_CLK")
        );
        let item = ctx.item_int_inv_legacy(&[tcls::INT; 8], tcid, bslot, "CONVST_INT_CLK");
        diff.apply_bit_diff(item, false, true);
        diffs.push(("INT_CLK".to_string(), diff));
        for i in 0..16 {
            let diff = ctx.get_diff_legacy(tile, bel, "CONVST", format!("GIOB{i}"));
            assert_eq!(
                diff,
                ctx.get_diff_legacy(tile, bel, "CONVST_TEST", format!("GIOB{i}"))
            );
            diffs.push((format!("GIOB{i}"), diff));
        }
        ctx.insert_legacy(
            tile,
            bel,
            "MUX.CONVST",
            xlat_enum_legacy_ocd(diffs, OcdMode::Mux),
        );
    }
}
