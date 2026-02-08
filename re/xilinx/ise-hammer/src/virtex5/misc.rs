use prjcombine_re_collector::{
    diff::OcdMode,
    legacy::{xlat_bit_legacy, xlat_bitvec_legacy},
};
use prjcombine_re_hammer::Session;
use prjcombine_types::{
    bits,
    bsdata::{TileBit, TileItem, TileItemKind},
};
use prjcombine_virtex4::defs;
use prjcombine_xilinx_bitstream::Reg;

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::relation::DeltaLegacy,
    },
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new_legacy(session, backend, "CFG");
    for attr in ["CCLKPIN", "DONEPIN", "PROGPIN", "INITPIN"] {
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
        bctx.test_manual_legacy("ENABLE", "1")
            .mode("BSCAN")
            .commit();
    }
    ctx.test_manual_legacy("BSCAN_COMMON", "USERID", "")
        .multi_global("USERID", MultiValue::HexPrefix, 32);
    for i in 0..2 {
        let mut bctx = ctx.bel(defs::bslots::ICAP[i]);
        bctx.build()
            .test_manual_legacy("ENABLE", "1")
            .mode("ICAP")
            .commit();
        bctx.mode("ICAP")
            .global_mutex_here("ICAP")
            .test_enum_legacy("ICAP_WIDTH", &["X8", "X16", "X32"]);
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
        bctx.build()
            .null_bits()
            .test_manual_legacy("PRESENT", "1")
            .mode("STARTUP")
            .commit();
        for val in ["CCLK", "USERCLK", "JTAGCLK"] {
            bctx.mode("STARTUP")
                .null_bits()
                .pin("CLK")
                .extra_tile_reg(Reg::Cor0, "REG.COR", "STARTUP")
                .test_manual_legacy("STARTUPCLK", val)
                .global("STARTUPCLK", val)
                .commit();
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
        for attr in ["GSR_SYNC", "GTS_SYNC"] {
            for val in ["YES", "NO"] {
                bctx.test_manual_legacy(attr, val)
                    .global(attr, val)
                    .commit();
            }
        }
    }
    {
        let mut bctx = ctx.bel(defs::bslots::JTAGPPC);
        bctx.build()
            .null_bits()
            .test_manual_legacy("PRESENT", "1")
            .mode("JTAGPPC")
            .commit();
        bctx.mode("JTAGPPC")
            .test_enum_legacy("NUM_PPC", &["0", "1", "2", "3", "4"]);
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
        bctx.build()
            .test_manual_legacy("ENABLE", "1")
            .mode("DCIRESET")
            .commit();
    }
    {
        let mut bctx = ctx.bel(defs::bslots::CAPTURE);
        bctx.build()
            .null_bits()
            .test_manual_legacy("PRESENT", "1")
            .mode("CAPTURE")
            .commit();
        for val in ["FALSE", "TRUE"] {
            bctx.mode("CAPTURE")
                .null_bits()
                .extra_tile_reg(Reg::Cor0, "REG.COR", "CAPTURE")
                .test_manual_legacy("ONESHOT", val)
                .attr("ONESHOT", val)
                .commit();
        }
    }
    {
        let mut bctx = ctx.bel(defs::bslots::USR_ACCESS);
        bctx.build()
            .null_bits()
            .test_manual_legacy("PRESENT", "1")
            .mode("USR_ACCESS")
            .commit();
    }
    {
        let mut bctx = ctx.bel(defs::bslots::KEY_CLEAR);
        bctx.build()
            .null_bits()
            .test_manual_legacy("PRESENT", "1")
            .mode("KEY_CLEAR")
            .commit();
    }
    {
        let mut bctx = ctx.bel(defs::bslots::EFUSE_USR);
        bctx.build()
            .null_bits()
            .test_manual_legacy("PRESENT", "1")
            .mode("EFUSE_USR")
            .commit();
    }
    {
        let mut bctx = ctx.bel(defs::bslots::SYSMON);
        bctx.build()
            .null_bits()
            .extra_tile_attr_legacy(
                DeltaLegacy::new(0, 20, "HCLK_IO_CFG_N"),
                "SYSMON",
                "ENABLE",
                "1",
            )
            .test_manual_legacy("ENABLE", "1")
            .mode("SYSMON")
            .commit();
        bctx.mode("SYSMON").test_inv_legacy("DCLK");
        bctx.mode("SYSMON").test_inv_legacy("CONVSTCLK");
        for i in 0x40..0x58 {
            bctx.mode("SYSMON")
                .test_multi_attr_hex_legacy(format!("INIT_{i:02X}"), 16);
        }
        for attr in [
            "SYSMON_TEST_A",
            "SYSMON_TEST_B",
            "SYSMON_TEST_C",
            "SYSMON_TEST_D",
            "SYSMON_TEST_E",
        ] {
            bctx.mode("SYSMON").test_multi_attr_hex_legacy(attr, 16);
        }
        bctx.build()
            .attr("SYSMON_TEST_A", "")
            .test_manual_legacy("JTAG_SYSMON", "DISABLE")
            .global("JTAG_SYSMON", "DISABLE")
            .commit();
    }

    let mut ctx = FuzzCtx::new_null(session, backend);
    {
        let reg = Reg::Cor0;
        let reg_name = "REG.COR";
        let bel = "STARTUP";
        for val in ["1", "2", "3", "4", "5", "6", "DONE", "KEEP"] {
            ctx.build()
                .test_reg(reg, reg_name, bel, "GWE_CYCLE", val)
                .global("GWE_CYCLE", val)
                .commit();
            ctx.build()
                .test_reg(reg, reg_name, bel, "GTS_CYCLE", val)
                .global("GTS_CYCLE", val)
                .commit();
        }
        for val in ["1", "2", "3", "4", "5", "6", "KEEP"] {
            ctx.build()
                .test_reg(reg, reg_name, bel, "DONE_CYCLE", val)
                .global("DONE_CYCLE", val)
                .commit();
        }
        for val in ["0", "1", "2", "3", "4", "5", "6", "NOWAIT"] {
            ctx.build()
                .test_reg(reg, reg_name, bel, "LCK_CYCLE", val)
                .global("LCK_CYCLE", val)
                .commit();
            ctx.build()
                .global_mutex("GLOBAL_DCI", "NO")
                .test_reg(reg, reg_name, bel, "MATCH_CYCLE", val)
                .global("MATCH_CYCLE", val)
                .commit();
        }
        for val in ["NO", "YES"] {
            ctx.build()
                .test_reg(reg, reg_name, bel, "DRIVE_DONE", val)
                .global("DRIVEDONE", val)
                .commit();
            ctx.build()
                .test_reg(reg, reg_name, bel, "DONE_PIPE", val)
                .global("DONEPIPE", val)
                .commit();
        }
        for val in [
            "2", "6", "9", "13", "17", "20", "24", "27", "31", "35", "38", "42", "46", "49", "53",
            "56", "60",
        ] {
            ctx.build()
                .test_reg(reg, reg_name, bel, "CONFIG_RATE", val)
                .global("CONFIGRATE", val)
                .commit();
        }
        for val in ["DISABLE", "ENABLE"] {
            ctx.build()
                .test_reg(reg, reg_name, bel, "CRC", val)
                .global("CRC", val)
                .commit();
        }
    }

    {
        let reg = Reg::Cor1;
        let reg_name = "REG.COR1";
        let bel = "MISC";
        for val in ["1", "4", "8"] {
            ctx.build()
                .test_reg(reg, reg_name, bel, "BPI_PAGE_SIZE", val)
                .global("BPI_PAGE_SIZE", val)
                .commit();
        }
        for val in ["1", "2", "3", "4"] {
            ctx.build()
                .global("BPI_PAGE_SIZE", "8")
                .test_reg(reg, reg_name, bel, "BPI_1ST_READ_CYCLE", val)
                .global("BPI_1ST_READ_CYCLE", val)
                .commit();
        }
        for val in ["NO", "YES"] {
            ctx.build()
                .test_reg(reg, reg_name, bel, "POST_CRC_EN", val)
                .global("POST_CRC_EN", val)
                .commit();
            ctx.build()
                .test_reg(reg, reg_name, bel, "POST_CRC_NO_PIN", val)
                .global("POST_CRC_NO_PIN", val)
                .commit();
            ctx.build()
                .test_reg(reg, reg_name, bel, "POST_CRC_RECONFIG", val)
                .global("POST_CRC_RECONFIG", val)
                .commit();
            ctx.build()
                .test_reg(reg, reg_name, bel, "RETAIN_CONFIG_STATUS", val)
                .global("RETAINCONFIGSTATUS", val)
                .commit();
        }
        for val in ["0", "1"] {
            ctx.build()
                .test_reg(reg, reg_name, bel, "POST_CRC_SEL", val)
                .global("POST_CRC_SEL", val)
                .commit();
        }
    }

    {
        let reg = Reg::Ctl0;
        let reg_name = "REG.CTL";
        let bel = "MISC";
        // persist not fuzzed — too much effort
        for val in ["NO", "YES"] {
            ctx.build()
                .global("CONFIGFALLBACK", "DISABLE")
                .test_reg(reg, reg_name, bel, "ENCRYPT", val)
                .global("ENCRYPT", val)
                .commit();
        }
        for val in ["NONE", "LEVEL1", "LEVEL2"] {
            ctx.build()
                .test_reg(reg, reg_name, bel, "SECURITY", val)
                .global("SECURITY", val)
                .commit();
        }
        for val in ["BBRAM", "EFUSE"] {
            ctx.build()
                .test_reg(reg, reg_name, bel, "ENCRYPT_KEY_SELECT", val)
                .global("ENCRYPTKEYSELECT", val)
                .commit();
        }
        for (attr, opt) in [
            ("OVERTEMP_POWERDOWN", "OVERTEMPPOWERDOWN"),
            ("CONFIG_FALLBACK", "CONFIGFALLBACK"),
            ("SELECTMAP_ABORT", "SELECTMAPABORT"),
        ] {
            for val in ["DISABLE", "ENABLE"] {
                ctx.build()
                    .test_reg(reg, reg_name, bel, attr, val)
                    .global(opt, val)
                    .commit();
            }
        }
        for val in ["0", "1"] {
            ctx.build()
                .test_reg(reg, reg_name, bel, "GLUTMASK", val)
                .global("GLUTMASK_B", val)
                .commit();
        }
        for opt in ["VBG_SEL", "VBG_DLL_SEL", "VGG_SEL"] {
            ctx.build()
                .test_reg(reg, reg_name, bel, opt, "")
                .multi_global(opt, MultiValue::Bin, 5);
        }
    }
    {
        let reg = Reg::Timer;
        let reg_name = "REG.TIMER";
        let bel = "MISC";
        ctx.build()
            .no_global("TIMER_USR")
            .test_reg(reg, reg_name, bel, "TIMER_CFG", "1")
            .global("TIMER_CFG", "0")
            .commit();
        ctx.build()
            .no_global("TIMER_CFG")
            .test_reg(reg, reg_name, bel, "TIMER_USR", "1")
            .global("TIMER_USR", "0")
            .commit();
        ctx.build()
            .no_global("TIMER_USR")
            .test_reg(reg, reg_name, bel, "TIMER", "")
            .multi_global("TIMER_CFG", MultiValue::Hex(0), 24);
    }

    {
        let reg = Reg::Testmode;
        let reg_name = "REG.TESTMODE";
        let bel = "MISC";
        ctx.build()
            .test_reg_present(reg, reg_name, bel, "DD_OVERRIDE", "YES")
            .global("DD_OVERRIDE", "YES")
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "CFG";
    let bel = "MISC";
    for attr in ["CCLKPIN", "DONEPIN", "PROGPIN", "INITPIN"] {
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
        "BSCAN[0]", "BSCAN[1]", "BSCAN[2]", "BSCAN[3]", "DCIRESET", "ICAP[0]", "ICAP[1]",
    ] {
        ctx.collect_bit_legacy(tile, bel, "ENABLE", "1");
    }
    let bel = "BSCAN_COMMON";
    let item = xlat_bitvec_legacy(ctx.get_diffs_legacy(tile, bel, "USERID", ""));
    ctx.insert_legacy(tile, bel, "USERID", item);

    let bel = "STARTUP";
    ctx.collect_bit_bi_legacy(tile, bel, "GSR_SYNC", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "GTS_SYNC", "NO", "YES");
    let item0 = ctx.extract_bit_legacy(tile, bel, "PIN.GSR", "1");
    let item1 = ctx.extract_bit_legacy(tile, bel, "PIN.GTS", "1");
    assert_eq!(item0, item1);
    ctx.insert_legacy(tile, bel, "GTS_GSR_ENABLE", item0);
    let item = ctx.extract_bit_legacy(tile, bel, "PIN.USRCCLKO", "1");
    ctx.insert_legacy(tile, bel, "USRCCLK_ENABLE", item);

    let item0 = ctx.extract_enum_legacy(tile, "ICAP[0]", "ICAP_WIDTH", &["X8", "X16", "X32"]);
    let item1 = ctx.extract_enum_legacy(tile, "ICAP[1]", "ICAP_WIDTH", &["X8", "X16", "X32"]);
    assert_eq!(item0, item1);
    ctx.insert_legacy(tile, "ICAP_COMMON", "ICAP_WIDTH", item0);

    {
        let bel = "JTAGPPC";
        ctx.collect_enum_legacy(tile, bel, "NUM_PPC", &["0", "1", "2", "3", "4"]);
    }

    {
        let bel = "SYSMON";
        ctx.collect_inv(tile, bel, "CONVSTCLK");
        ctx.collect_inv(tile, bel, "DCLK");
        for i in 0x40..0x58 {
            ctx.collect_bitvec_legacy(tile, bel, &format!("INIT_{i:02X}"), "");
        }
        ctx.collect_bitvec_legacy(tile, bel, "SYSMON_TEST_A", "");
        ctx.collect_bitvec_legacy(tile, bel, "SYSMON_TEST_B", "");
        ctx.collect_bitvec_legacy(tile, bel, "SYSMON_TEST_C", "");
        ctx.collect_bitvec_legacy(tile, bel, "SYSMON_TEST_D", "");
        ctx.collect_bitvec_legacy(tile, bel, "SYSMON_TEST_E", "");

        let mut diff = ctx.get_diff_legacy(tile, bel, "JTAG_SYSMON", "DISABLE");
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "SYSMON_TEST_A"), 2, 0);
        diff.assert_empty();
    }

    {
        let tile = "HCLK_IO_CFG_N";
        let bel = "SYSMON";
        ctx.collect_bit_legacy(tile, bel, "ENABLE", "1");
    }

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
            "2", "6", "9", "13", "17", "20", "24", "27", "31", "35", "38", "42", "46", "49", "53",
            "56", "60",
        ],
        OcdMode::BitOrder,
    );
    ctx.collect_bit_bi_legacy(tile, bel, "DRIVE_DONE", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "DONE_PIPE", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "CRC", "DISABLE", "ENABLE");
    let bel = "CAPTURE";
    ctx.collect_bit_bi_legacy(tile, bel, "ONESHOT", "FALSE", "TRUE");

    let tile = "REG.COR1";
    let bel = "MISC";
    ctx.collect_enum_legacy(tile, bel, "BPI_PAGE_SIZE", &["1", "4", "8"]);
    ctx.collect_enum_legacy(tile, bel, "BPI_1ST_READ_CYCLE", &["1", "2", "3", "4"]);
    ctx.collect_bit_bi_legacy(tile, bel, "POST_CRC_EN", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "POST_CRC_NO_PIN", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "POST_CRC_RECONFIG", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "RETAIN_CONFIG_STATUS", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "POST_CRC_SEL", "0", "1");

    let tile = "REG.CTL";
    let bel = "MISC";
    ctx.collect_enum_legacy(tile, bel, "SECURITY", &["NONE", "LEVEL1", "LEVEL2"]);
    ctx.collect_bit_bi_legacy(tile, bel, "ENCRYPT", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "OVERTEMP_POWERDOWN", "DISABLE", "ENABLE");
    ctx.collect_bit_bi_legacy(tile, bel, "CONFIG_FALLBACK", "DISABLE", "ENABLE");
    ctx.collect_bit_bi_legacy(tile, bel, "SELECTMAP_ABORT", "DISABLE", "ENABLE");
    ctx.collect_bit_bi_legacy(tile, bel, "GLUTMASK", "1", "0");
    ctx.collect_enum_legacy(tile, bel, "ENCRYPT_KEY_SELECT", &["BBRAM", "EFUSE"]);
    ctx.collect_bitvec_legacy(tile, bel, "VBG_SEL", "");
    ctx.collect_bitvec_legacy(tile, bel, "VBG_DLL_SEL", "");
    ctx.collect_bitvec_legacy(tile, bel, "VGG_SEL", "");
    // these are too much trouble to deal with the normal way.
    for (attr, bit) in [("GTS_USR_B", 0), ("PERSIST", 3)] {
        ctx.insert_legacy(
            tile,
            bel,
            attr,
            TileItem {
                bits: vec![TileBit::new(0, 0, bit)],
                kind: TileItemKind::BitVec { invert: bits![0] },
            },
        );
    }
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

    let tile = "REG.TIMER";
    let bel = "MISC";
    ctx.collect_bitvec_legacy(tile, bel, "TIMER", "");
    ctx.collect_bit_legacy(tile, bel, "TIMER_CFG", "1");
    ctx.collect_bit_legacy(tile, bel, "TIMER_USR", "1");

    let tile = "REG.TESTMODE";
    let bel = "MISC";
    let mut diff = ctx.get_diff_legacy(tile, bel, "DD_OVERRIDE", "YES");
    diff.bits.remove(&TileBit::new(1, 0, 0));
    ctx.insert_legacy(tile, bel, "DD_OVERRIDE", xlat_bit_legacy(diff));
}
