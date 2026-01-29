use prjcombine_re_collector::{
    diff::OcdMode,
    legacy::{xlat_bit_legacy, xlat_bitvec_legacy, xlat_enum_legacy_ocd},
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
        props::relation::Delta,
    },
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, "CFG");
    for attr in ["CCLKPIN", "DONEPIN", "PROGPIN", "INITPIN"] {
        for val in ["PULLUP", "PULLNONE"] {
            ctx.test_manual("MISC", attr, val)
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
            ctx.test_manual("MISC", attr, val)
                .global(attr, val)
                .commit();
        }
    }

    for i in 0..4 {
        let mut bctx = ctx.bel(defs::bslots::BSCAN[i]);
        bctx.test_manual("ENABLE", "1").mode("BSCAN").commit();
        bctx.mode("BSCAN")
            .global_mutex_here("DISABLE_JTAG")
            .test_enum("DISABLE_JTAG", &["FALSE", "TRUE"]);
    }
    ctx.test_manual("BSCAN_COMMON", "USERID", "")
        .multi_global("USERID", MultiValue::HexPrefix, 32);

    {
        let mut bctx = ctx.bel(defs::bslots::ICAP[1]);
        bctx.test_manual("ENABLE", "1").mode("ICAP").commit();
        bctx.mode("ICAP")
            .global_mutex_here("ICAP")
            .test_enum("ICAP_WIDTH", &["X8", "X16", "X32"]);
        bctx.mode("ICAP")
            .global_mutex_here("ICAP")
            .test_enum("ICAP_AUTO_SWITCH", &["DISABLE", "ENABLE"]);

        let mut bctx = ctx.bel(defs::bslots::ICAP[0]);
        bctx.build()
            .bel_mode(defs::bslots::ICAP[1], "ICAP")
            .test_manual_legacy("ENABLE", "1")
            .mode("ICAP")
            .commit();
        bctx.mode("ICAP")
            .bel_mode(defs::bslots::ICAP[1], "ICAP")
            .global_mutex_here("ICAP")
            .test_enum("ICAP_WIDTH", &["X8", "X16", "X32"]);
        bctx.mode("ICAP")
            .bel_mode(defs::bslots::ICAP[1], "ICAP")
            .global_mutex_here("ICAP")
            .test_enum("ICAP_AUTO_SWITCH", &["DISABLE", "ENABLE"]);
    }

    for i in 0..2 {
        let mut bctx = ctx.bel(defs::bslots::PMV_CFG[i]);
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
                .extra_tile_reg(Reg::Cor0, "REG.COR", "STARTUP")
                .pin("CLK")
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
        bctx.mode("STARTUP")
            .global("ENCRYPT", "YES")
            .test_manual_legacy("PIN.KEYCLEARB", "1")
            .pin("KEYCLEARB")
            .commit();
        for attr in ["GSR_SYNC", "GTS_SYNC"] {
            for val in ["YES", "NO"] {
                bctx.test_manual(attr, val).global(attr, val).commit();
            }
        }
        bctx.mode("STARTUP")
            .test_enum("PROG_USR", &["FALSE", "TRUE"]);
    }

    {
        let mut bctx = ctx.bel(defs::bslots::FRAME_ECC);
        bctx.build()
            .null_bits()
            .extra_tile_reg(Reg::Ctl0, "REG.CTL", "FRAME_ECC")
            .no_global("GLUTMASK_B")
            .test_manual_legacy("ENABLE", "1")
            .mode("FRAME_ECC")
            .commit();
        bctx.mode("FRAME_ECC")
            .null_bits()
            .extra_tile_reg(Reg::Ctl0, "REG.CTL", "FRAME_ECC")
            .test_enum("FARSRC", &["FAR", "EFAR"]);
    }

    {
        let mut bctx = ctx.bel(defs::bslots::DCIRESET);
        bctx.test_manual("ENABLE", "1").mode("DCIRESET").commit();
    }

    {
        let mut bctx = ctx.bel(defs::bslots::CAPTURE);
        bctx.build()
            .null_bits()
            .test_manual_legacy("PRESENT", "1")
            .mode("CAPTURE")
            .commit();
        bctx.mode("CAPTURE")
            .null_bits()
            .extra_tile_reg(Reg::Cor0, "REG.COR", "CAPTURE")
            .test_enum("ONESHOT", &["FALSE", "TRUE"]);
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
        let mut bctx = ctx.bel(defs::bslots::EFUSE_USR);
        bctx.build()
            .null_bits()
            .test_manual_legacy("PRESENT", "1")
            .mode("EFUSE_USR")
            .commit();
    }

    {
        let mut bctx = ctx.bel(defs::bslots::DNA_PORT);
        bctx.test_manual("ENABLE", "1").mode("DNA_PORT").commit();
    }

    {
        let mut bctx = ctx.bel(defs::bslots::CFG_IO_ACCESS);
        bctx.test_manual("ENABLE", "1")
            .mode("CFG_IO_ACCESS")
            .commit();
    }

    {
        let mut bctx = ctx.bel(defs::bslots::SYSMON);
        bctx.build()
            .null_bits()
            .extra_tile_attr(Delta::new(0, 20, "HCLK"), "HCLK", "DRP_MASK_SYSMON", "1")
            .test_manual_legacy("ENABLE", "1")
            .mode("SYSMON")
            .commit();
        bctx.mode("SYSMON").test_inv("DCLK");
        bctx.mode("SYSMON").test_inv("CONVSTCLK");
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
            .attr("SYSMON_TEST_E", "")
            .test_manual_legacy("JTAG_SYSMON", "DISABLE")
            .global("JTAG_SYSMON", "DISABLE")
            .commit();
    }

    let mut ctx = FuzzCtx::new_null(session, backend);

    {
        let reg_name = "REG.COR";
        let bel = "STARTUP";
        let reg = Reg::Cor0;
        for val in ["1", "2", "3", "4", "5", "6", "DONE", "KEEP"] {
            ctx.test_reg(reg, reg_name, bel, "GWE_CYCLE", val)
                .global("GWE_CYCLE", val)
                .commit();
            ctx.test_reg(reg, reg_name, bel, "GTS_CYCLE", val)
                .global("GTS_CYCLE", val)
                .commit();
        }
        for val in ["1", "2", "3", "4", "5", "6", "KEEP"] {
            ctx.test_reg(reg, reg_name, bel, "DONE_CYCLE", val)
                .global("DONE_CYCLE", val)
                .commit();
        }
        for val in ["0", "1", "2", "3", "4", "5", "6", "NOWAIT"] {
            ctx.test_reg(reg, reg_name, bel, "LCK_CYCLE", val)
                .global("LCK_CYCLE", val)
                .commit();
            ctx.build()
                .global_mutex("GLOBAL_DCI", "NO")
                .test_reg(reg, reg_name, bel, "MATCH_CYCLE", val)
                .global("MATCH_CYCLE", val)
                .commit();
        }
        for val in ["NO", "YES"] {
            ctx.test_reg(reg, reg_name, bel, "DRIVE_DONE", val)
                .global("DRIVEDONE", val)
                .commit();
            ctx.test_reg(reg, reg_name, bel, "DONE_PIPE", val)
                .global("DONEPIPE", val)
                .commit();
        }
        for val in [
            "2", "4", "6", "10", "12", "16", "22", "26", "33", "40", "50", "66",
        ] {
            ctx.test_reg(reg, reg_name, bel, "CONFIG_RATE", val)
                .global("CONFIGRATE", val)
                .commit();
        }
        for val in ["DISABLE", "ENABLE"] {
            ctx.test_reg(reg, reg_name, bel, "DONE_SIGNALS_POWERDOWN", val)
                .global("DONESIGNALSPOWERDOWN", val)
                .commit();
        }
    }

    {
        let reg_name = "REG.COR1";
        let bel = "MISC";
        let reg = Reg::Cor1;
        for val in ["1", "4", "8"] {
            ctx.test_reg(reg, reg_name, bel, "BPI_PAGE_SIZE", val)
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
                .global("GLUTMASK_B", "0")
                .test_reg(reg, reg_name, bel, "POST_CRC_EN", val)
                .global("POST_CRC_EN", val)
                .commit();
            ctx.test_reg(reg, reg_name, bel, "POST_CRC_RECONFIG", val)
                .global("POST_CRC_RECONFIG", val)
                .commit();
            ctx.test_reg(reg, reg_name, bel, "POST_CRC_KEEP", val)
                .global("POST_CRC_KEEP", val)
                .commit();
            ctx.test_reg(reg, reg_name, bel, "POST_CRC_CORRECT", val)
                .global("POST_CRC_CORRECT", val)
                .commit();
        }
        for opt in ["POST_CRC_SEL", "FUSE_NO_CDR"] {
            for val in ["0", "1"] {
                ctx.test_reg(reg, reg_name, bel, opt, val)
                    .global(opt, val)
                    .commit();
            }
        }
        for val in ["1", "2", "3", "6", "13", "25", "50"] {
            ctx.test_reg(reg, reg_name, bel, "POST_CRC_FREQ", val)
                .global("POST_CRC_FREQ", val)
                .commit();
        }
        for val in ["CFG_CLK", "INTERNAL"] {
            ctx.build()
                .no_global("POST_CRC_FREQ")
                .test_reg(reg, reg_name, bel, "POST_CRC_CLK", val)
                .global("POST_CRC_CLK", val)
                .commit();
        }
        for val in ["DISABLE", "ENABLE"] {
            ctx.test_reg(reg, reg_name, bel, "SYSMON_PARTIAL_RECONFIG", val)
                .global("SYSMONPARTIALRECONFIG", val)
                .commit();
            ctx.test_reg(reg, reg_name, bel, "POST_CRC_INIT_FLAG", val)
                .global("POST_CRC_INIT_FLAG", val)
                .commit();
        }
        for val in ["NO", "YES"] {
            ctx.test_reg(reg, reg_name, bel, "FALLBACK_PULSE_FWE", val)
                .global("FALLBACKPULSEFWE", val)
                .commit();
        }
    }

    {
        let reg_name = "REG.CTL";
        let bel = "MISC";
        let reg = Reg::Ctl0;
        // persist not fuzzed — too much effort
        for val in ["NONE", "LEVEL1", "LEVEL2"] {
            ctx.test_reg(reg, reg_name, bel, "SECURITY", val)
                .global("SECURITY", val)
                .commit();
        }
        for val in ["BBRAM", "EFUSE"] {
            ctx.test_reg(reg, reg_name, bel, "ENCRYPT_KEY_SELECT", val)
                .global("ENCRYPTKEYSELECT", val)
                .commit();
        }
        for (attr, opt) in [
            ("OVERTEMP_POWERDOWN", "OVERTEMPPOWERDOWN"),
            ("CONFIG_FALLBACK", "CONFIGFALLBACK"),
            ("INIT_SIGNALS_ERROR", "INITSIGNALSERROR"),
            ("SELECTMAP_ABORT", "SELECTMAPABORT"),
        ] {
            for val in ["DISABLE", "ENABLE"] {
                ctx.test_reg(reg, reg_name, bel, attr, val)
                    .global(opt, val)
                    .commit();
            }
        }
        for val in ["0", "1"] {
            ctx.test_reg(reg, reg_name, bel, "GTS_USR_B", val)
                .global("GTS_USR_B", val)
                .commit();
        }
        for val in ["NO", "YES"] {
            ctx.test_reg(reg, reg_name, bel, "SEC_ALL", val)
                .global("SECALL", val)
                .commit();
            ctx.test_reg(reg, reg_name, bel, "SEC_ERROR", val)
                .global("SECERROR", val)
                .commit();
            ctx.test_reg(reg, reg_name, bel, "SEC_STATUS", val)
                .global("SECSTATUS", val)
                .commit();
            ctx.test_reg(reg, reg_name, bel, "ENCRYPT", val)
                .global("ENCRYPT", val)
                .commit();
        }
    }

    {
        let reg_name = "REG.CTL1";
        let bel = "MISC";
        let reg = Reg::Ctl1;
        for (attr, opt) in [("ICAP_ENCRYPTION", "ICAP_ENCRYPTION")] {
            for val in ["DISABLE", "ENABLE"] {
                ctx.test_reg(reg, reg_name, bel, attr, val)
                    .global(opt, val)
                    .commit();
            }
        }
        for opt in ["VGG_TEST", "EN_VTEST", "DIS_VGG_REG", "ENABLE_VGG_CLAMP"] {
            for val in ["NO", "YES"] {
                ctx.test_reg(reg, reg_name, bel, opt, val)
                    .global(opt, val)
                    .commit();
            }
        }
        for opt in ["VGG_OPT_DRV", "VGG_V4_OPT"] {
            for val in ["0", "1"] {
                ctx.test_reg(reg, reg_name, bel, opt, val)
                    .global(opt, val)
                    .commit();
            }
        }
        for val in ["DISABLE", "TEST0", "TEST1"] {
            ctx.build()
                .no_global("EN_VTEST")
                .no_global("VGG_TEST")
                .test_reg(reg, reg_name, bel, "MODE_PIN_TEST", val)
                .global("MODEPINTEST", val)
                .commit();
        }

        for (opt, width) in [("VGG_SEL", 5), ("VGG_SEL2", 5)] {
            ctx.build()
                .test_reg(reg, reg_name, bel, opt, "")
                .multi_global(opt, MultiValue::Bin, width);
        }
    }

    {
        let reg_name = "REG.UNK1C";
        let bel = "MISC";
        let reg = Reg::Unk1C;
        ctx.test_reg(reg, reg_name, bel, "VBG_SEL", "")
            .multi_global("VBG_SEL", MultiValue::Bin, 6);
        ctx.test_reg(reg, reg_name, bel, "VBG_VGG_FLAST_SEL", "")
            .multi_global("VBGVGGFLASTSEL", MultiValue::Bin, 6);
        ctx.test_reg(reg, reg_name, bel, "VBG_VGG_NEG_SEL", "")
            .multi_global("VBGVGGNEGSEL", MultiValue::Bin, 6);
    }

    {
        let reg_name = "REG.TRIM";
        let bel = "MISC";
        let reg = Reg::Trim0;
        ctx.test_reg(reg, reg_name, bel, "MPD_SEL", "")
            .multi_global("MPD_SEL", MultiValue::Bin, 3);
    }

    {
        let reg_name = "REG.TESTMODE";
        let bel = "MISC";
        let reg = Reg::Testmode;
        ctx.build()
            .extra_tile_reg_present(reg, reg_name, bel)
            .test_manual(bel, "FUSE_SHADOW", "")
            .multi_global("FUSE_SHADOW", MultiValue::Bin, 1);
    }

    {
        let reg_name = "REG.TIMER";
        let bel = "MISC";
        let reg = Reg::Timer;
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
        let reg_name = "FAKE.IGNORE_CRC";
        let bel = "MISC";
        let reg = Reg::FakeIgnoreCrc;
        for val in ["DISABLE", "ENABLE"] {
            ctx.build()
                .extra_tile_reg_present(reg, reg_name, bel)
                .test_manual(bel, "CRC", val)
                .global("CRC", val)
                .commit();
        }
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

    for bel in ["BSCAN[0]", "BSCAN[1]", "BSCAN[2]", "BSCAN[3]"] {
        ctx.collect_bit_legacy(tile, bel, "ENABLE", "1");
        let item = ctx.extract_bit_wide_bi_legacy(tile, bel, "DISABLE_JTAG", "FALSE", "TRUE");
        ctx.insert(tile, "BSCAN_COMMON", "DISABLE_JTAG", item);
    }
    let bel = "BSCAN_COMMON";
    let item = xlat_bitvec_legacy(ctx.get_diffs_legacy(tile, bel, "USERID", ""));
    ctx.insert(tile, bel, "USERID", item);

    for bel in [
        "ICAP[0]",
        "ICAP[1]",
        "DCIRESET",
        "DNA_PORT",
        "CFG_IO_ACCESS",
    ] {
        ctx.collect_bit_wide_legacy(tile, bel, "ENABLE", "1");
    }
    for bel in ["ICAP[0]", "ICAP[1]"] {
        // ???
        ctx.get_diff_legacy(tile, bel, "ICAP_AUTO_SWITCH", "DISABLE")
            .assert_empty();
        ctx.get_diff_legacy(tile, bel, "ICAP_AUTO_SWITCH", "ENABLE")
            .assert_empty();
    }

    let bel = "STARTUP";
    ctx.collect_bit_bi_legacy(tile, bel, "GSR_SYNC", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "GTS_SYNC", "NO", "YES");
    let item0 = ctx.extract_bit_wide_legacy(tile, bel, "PIN.GSR", "1");
    let item1 = ctx.extract_bit_wide_legacy(tile, bel, "PIN.GTS", "1");
    assert_eq!(item0, item1);
    ctx.insert(tile, bel, "GTS_GSR_ENABLE", item0);
    ctx.collect_bit_wide_bi_legacy(tile, bel, "PROG_USR", "FALSE", "TRUE");
    let item = ctx.extract_bit_wide_legacy(tile, bel, "PIN.USRCCLKO", "1");
    ctx.insert(tile, bel, "USRCCLK_ENABLE", item);
    let item = ctx.extract_bit_wide_legacy(tile, bel, "PIN.KEYCLEARB", "1");
    ctx.insert(tile, bel, "KEY_CLEAR_ENABLE", item);

    let item0 = ctx.extract_enum_legacy(tile, "ICAP[0]", "ICAP_WIDTH", &["X8", "X16", "X32"]);
    let item1 = ctx.extract_enum_legacy(tile, "ICAP[1]", "ICAP_WIDTH", &["X8", "X16", "X32"]);
    assert_eq!(item0, item1);
    ctx.insert(tile, "ICAP_COMMON", "ICAP_WIDTH", item0);

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
        diff.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "SYSMON_TEST_E"), 7, 0);
        diff.assert_empty();
    }

    {
        let tile = "HCLK";
        let bel = "HCLK";
        ctx.collect_bit_legacy(tile, bel, "DRP_MASK_SYSMON", "1");
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
            "2", "4", "6", "10", "12", "16", "22", "26", "33", "40", "50", "66",
        ],
        OcdMode::BitOrder,
    );
    ctx.collect_bit_bi_legacy(tile, bel, "DRIVE_DONE", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "DONE_PIPE", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "DONE_SIGNALS_POWERDOWN", "DISABLE", "ENABLE");
    let bel = "CAPTURE";
    ctx.collect_bit_bi_legacy(tile, bel, "ONESHOT", "FALSE", "TRUE");

    let tile = "REG.COR1";
    let bel = "MISC";
    ctx.collect_enum_legacy(tile, bel, "BPI_PAGE_SIZE", &["1", "4", "8"]);
    ctx.collect_enum_legacy(tile, bel, "BPI_1ST_READ_CYCLE", &["1", "2", "3", "4"]);
    ctx.collect_enum_legacy(tile, bel, "POST_CRC_CLK", &["CFG_CLK", "INTERNAL"]);
    let mut diffs = vec![];
    for val in ["1", "2", "3", "6", "13", "25", "50"] {
        let mut diff = ctx.get_diff_legacy(tile, bel, "POST_CRC_FREQ", val);
        diff.apply_enum_diff_legacy(ctx.item(tile, bel, "POST_CRC_CLK"), "INTERNAL", "CFG_CLK");
        diffs.push((val, diff));
    }
    ctx.insert(
        tile,
        bel,
        "POST_CRC_FREQ",
        xlat_enum_legacy_ocd(diffs, OcdMode::BitOrder),
    );

    ctx.collect_bit_bi_legacy(tile, bel, "POST_CRC_EN", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "POST_CRC_RECONFIG", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "POST_CRC_KEEP", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "POST_CRC_CORRECT", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "POST_CRC_SEL", "0", "1");
    ctx.collect_bit_bi_legacy(tile, bel, "FUSE_NO_CDR", "0", "1");
    ctx.collect_bit_bi_legacy(tile, bel, "POST_CRC_INIT_FLAG", "DISABLE", "ENABLE");
    ctx.collect_bit_bi_legacy(tile, bel, "SYSMON_PARTIAL_RECONFIG", "DISABLE", "ENABLE");
    ctx.collect_bit_bi_legacy(tile, bel, "FALLBACK_PULSE_FWE", "NO", "YES");
    ctx.insert(
        tile,
        bel,
        "PERSIST_DEASSERT_AT_DESYNC",
        TileItem::from_bit_inv(TileBit::new(0, 0, 17), false),
    );

    let tile = "REG.CTL";
    let bel = "MISC";
    ctx.collect_bit_bi_legacy(tile, bel, "GTS_USR_B", "0", "1");
    ctx.collect_enum_legacy(tile, bel, "SECURITY", &["NONE", "LEVEL1", "LEVEL2"]);
    ctx.collect_bit_bi_legacy(tile, bel, "OVERTEMP_POWERDOWN", "DISABLE", "ENABLE");
    ctx.collect_bit_bi_legacy(tile, bel, "CONFIG_FALLBACK", "DISABLE", "ENABLE");
    ctx.collect_bit_bi_legacy(tile, bel, "INIT_SIGNALS_ERROR", "DISABLE", "ENABLE");
    ctx.collect_bit_bi_legacy(tile, bel, "SELECTMAP_ABORT", "DISABLE", "ENABLE");
    ctx.collect_enum_legacy(tile, bel, "ENCRYPT_KEY_SELECT", &["BBRAM", "EFUSE"]);
    ctx.collect_bit_bi_legacy(tile, bel, "SEC_ALL", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "SEC_ERROR", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "SEC_STATUS", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "ENCRYPT", "NO", "YES");
    // these are too much trouble to deal with the normal way.
    ctx.insert(
        tile,
        bel,
        "PERSIST",
        TileItem {
            bits: vec![TileBit::new(0, 0, 3)],
            kind: TileItemKind::BitVec { invert: bits![0] },
        },
    );
    ctx.insert(
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
    let bel = "FRAME_ECC";
    let item = ctx.extract_bit_legacy(tile, bel, "ENABLE", "1");
    ctx.insert(tile, "MISC", "GLUTMASK", item);
    ctx.collect_enum_legacy(tile, bel, "FARSRC", &["FAR", "EFAR"]);

    let tile = "REG.CTL1";
    let bel = "MISC";
    ctx.collect_bit_bi_legacy(tile, bel, "ICAP_ENCRYPTION", "DISABLE", "ENABLE");
    ctx.collect_bit_bi_legacy(tile, bel, "VGG_TEST", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "EN_VTEST", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "DIS_VGG_REG", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "ENABLE_VGG_CLAMP", "NO", "YES");
    ctx.collect_bit_bi_legacy(tile, bel, "VGG_OPT_DRV", "0", "1");
    ctx.collect_bit_bi_legacy(tile, bel, "VGG_V4_OPT", "0", "1");
    ctx.get_diff_legacy(tile, bel, "MODE_PIN_TEST", "DISABLE")
        .assert_empty();
    let mut diff = ctx.get_diff_legacy(tile, bel, "MODE_PIN_TEST", "TEST0");
    diff.apply_bit_diff_legacy(ctx.item(tile, bel, "VGG_TEST"), true, false);
    diff.assert_empty();
    let mut diff = ctx.get_diff_legacy(tile, bel, "MODE_PIN_TEST", "TEST1");
    diff.apply_bit_diff_legacy(ctx.item(tile, bel, "EN_VTEST"), true, false);
    diff.assert_empty();
    ctx.collect_bitvec_legacy(tile, bel, "VGG_SEL", "");
    ctx.collect_bitvec_legacy(tile, bel, "VGG_SEL2", "");

    let tile = "REG.TIMER";
    let bel = "MISC";
    ctx.collect_bitvec_legacy(tile, bel, "TIMER", "");
    ctx.collect_bit_legacy(tile, bel, "TIMER_CFG", "1");
    ctx.collect_bit_legacy(tile, bel, "TIMER_USR", "1");

    let tile = "REG.TESTMODE";
    let bel = "MISC";
    let mut diff = ctx.get_diff_legacy(tile, bel, "FUSE_SHADOW", "");
    diff.bits.remove(&TileBit::new(1, 0, 0));
    ctx.insert(tile, bel, "FUSE_SHADOW", xlat_bit_legacy(diff));

    let tile = "REG.TRIM";
    let bel = "MISC";
    ctx.collect_bitvec_legacy(tile, bel, "MPD_SEL", "");

    let tile = "REG.UNK1C";
    let bel = "MISC";
    ctx.collect_bitvec_legacy(tile, bel, "VBG_SEL", "");
    ctx.collect_bitvec_legacy(tile, bel, "VBG_VGG_FLAST_SEL", "");
    ctx.collect_bitvec_legacy(tile, bel, "VBG_VGG_NEG_SEL", "");

    let tile = "FAKE.IGNORE_CRC";
    let bel = "MISC";
    ctx.get_diff_legacy(tile, bel, "CRC", "ENABLE")
        .assert_empty();
    let diff = ctx.get_diff_legacy(tile, bel, "CRC", "DISABLE");
    assert_eq!(diff.bits.len(), 2);
    assert!(diff.bits[&TileBit::new(0, 0, 0)]);
    assert!(diff.bits[&TileBit::new(1, 0, 0)]);
}
