use prjcombine_re_fpga_hammer::diff::{OcdMode, extract_bitvec_val, xlat_bit, xlat_enum_ocd};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
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
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    let mut ctx = FuzzCtx::new_null(session, backend);
    for (bel, attr, vals) in [
        ("MISC", "M0PIN", &["PULLNONE", "PULLDOWN", "PULLUP"][..]),
        ("MISC", "M1PIN", &["PULLNONE", "PULLDOWN", "PULLUP"][..]),
        ("MISC", "M2PIN", &["PULLNONE", "PULLDOWN", "PULLUP"][..]),
        ("MISC", "TDIPIN", &["PULLNONE", "PULLDOWN", "PULLUP"][..]),
        ("MISC", "TDOPIN", &["PULLNONE", "PULLDOWN", "PULLUP"][..]),
        ("MISC", "TMSPIN", &["PULLNONE", "PULLDOWN", "PULLUP"][..]),
        ("MISC", "TCKPIN", &["PULLNONE", "PULLDOWN", "PULLUP"][..]),
        ("MISC", "PROGPIN", &["PULLNONE", "PULLUP"][..]),
        ("MISC", "INITPIN", &["PULLNONE", "PULLUP"][..]),
        ("MISC", "DONEPIN", &["PULLNONE", "PULLUP"][..]),
        ("MISC", "CCLKPIN", &["PULLNONE", "PULLUP"][..]),
        ("STARTUP", "GTS_SYNC", &["NO", "YES"][..]),
        ("STARTUP", "GSR_SYNC", &["NO", "YES"][..]),
    ] {
        for &val in vals {
            ctx.build()
                .extra_tiles_by_kind("CFG", bel)
                .test_manual(bel, attr, val)
                .global(attr, val)
                .commit();
        }
    }
    ctx.build()
        .extra_tiles_by_kind("CFG", "BSCAN_COMMON")
        .test_manual("BSCAN_COMMON", "USERID", "")
        .multi_global("USERID", MultiValue::HexPrefix, 32);

    let mut ctx = FuzzCtx::new(session, backend, "CFG");
    for i in 0..4 {
        let mut bctx = ctx.bel(defs::bslots::BSCAN[i]);
        bctx.test_manual("ENABLE", "1").mode("BSCAN").commit();
        bctx.mode("BSCAN")
            .global_mutex_here("DISABLE_JTAG")
            .test_enum("DISABLE_JTAG", &["FALSE", "TRUE"]);
    }

    if edev.chips.len() == 1 && !edev.chips.first().unwrap().has_ps {
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
            .test_manual("ENABLE", "1")
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
    {
        let mut bctx = ctx.bel(defs::bslots::STARTUP);
        if edev.chips.len() == 1 {
            bctx.build()
                .null_bits()
                .test_manual("PRESENT", "1")
                .mode("STARTUP")
                .commit();
            for val in ["CCLK", "USERCLK", "JTAGCLK"] {
                bctx.mode("STARTUP")
                    .null_bits()
                    .extra_tile_reg(Reg::Cor0, "REG.COR", "STARTUP")
                    .pin("CLK")
                    .test_manual("STARTUPCLK", val)
                    .global("STARTUPCLK", val)
                    .commit();
            }
        }
        bctx.mode("STARTUP")
            .no_pin("GSR")
            .test_manual("PIN.GTS", "1")
            .pin("GTS")
            .commit();
        bctx.mode("STARTUP")
            .no_pin("GTS")
            .test_manual("PIN.GSR", "1")
            .pin("GSR")
            .commit();
        bctx.mode("STARTUP")
            .test_manual("PIN.USRCCLKO", "1")
            .pin("USRCCLKO")
            .commit();
        if edev.chips.first().unwrap().regs > 1 {
            bctx.mode("STARTUP")
                .global("ENCRYPT", "YES")
                .test_manual("PIN.KEYCLEARB", "1")
                .pin("KEYCLEARB")
                .commit();
        }
        bctx.mode("STARTUP")
            .test_enum("PROG_USR", &["FALSE", "TRUE"]);
    }
    if edev.chips.len() == 1 {
        let mut bctx = ctx.bel(defs::bslots::CAPTURE);
        bctx.build()
            .null_bits()
            .test_manual("PRESENT", "1")
            .mode("CAPTURE")
            .commit();
        for val in ["FALSE", "TRUE"] {
            bctx.mode("CAPTURE")
                .null_bits()
                .extra_tile_reg(Reg::Cor0, "REG.COR", "CAPTURE")
                .test_manual("ONESHOT", val)
                .attr("ONESHOT", val)
                .commit();
        }
    }
    if edev.chips.len() == 1 {
        let mut bctx = ctx.bel(defs::bslots::CFG_IO_ACCESS);
        bctx.build()
            .no_global("CFGIOACCESS_TDO")
            .extra_tile_reg(Reg::Cor1, "REG.COR1", "CFG_IO_ACCESS")
            .test_manual("ENABLE", "1")
            .mode("CFG_IO_ACCESS")
            .commit();
        bctx.mode("CFG_IO_ACCESS")
            .extra_tile_reg(Reg::Cor1, "REG.COR1", "CFG_IO_ACCESS")
            .test_manual("TDO", "UNCONNECTED")
            .global("CFGIOACCESS_TDO", "UNCONNECTED")
            .commit();
    }
    if edev.chips.len() == 1 {
        let mut bctx = ctx.bel(defs::bslots::FRAME_ECC);
        bctx.build()
            .null_bits()
            .extra_tile_reg(Reg::Ctl0, "REG.CTL", "FRAME_ECC")
            .no_global("GLUTMASK_B")
            .test_manual("ENABLE", "1")
            .mode("FRAME_ECC")
            .commit();
        for val in ["FAR", "EFAR"] {
            bctx.mode("FRAME_ECC")
                .null_bits()
                .extra_tile_reg(Reg::Ctl0, "REG.CTL", "FRAME_ECC")
                .test_manual("FARSRC", val)
                .attr("FARSRC", val)
                .commit();
        }
    }
    {
        let mut bctx = ctx.bel(defs::bslots::DCIRESET);
        bctx.test_manual("ENABLE", "1").mode("DCIRESET").commit();
    }
    {
        let mut bctx = ctx.bel(defs::bslots::DNA_PORT);
        bctx.test_manual("ENABLE", "1").mode("DNA_PORT").commit();
    }

    let mut ctx = FuzzCtx::new_null(session, backend);
    {
        let reg = Reg::Cor0;
        let reg_name = "REG.COR";
        let bel = "STARTUP";
        for val in ["0", "1", "2", "3", "4", "5", "6", "NOWAIT"] {
            ctx.build()
                .global_mutex("GLOBAL_DCI", "NO")
                .test_reg(reg, reg_name, bel, "MATCH_CYCLE", val)
                .global("MATCH_CYCLE", val)
                .commit();
        }
        for (attr, opt, vals) in [
            (
                "GWE_CYCLE",
                "GWE_CYCLE",
                &["1", "2", "3", "4", "5", "6", "DONE", "KEEP"][..],
            ),
            (
                "GTS_CYCLE",
                "GTS_CYCLE",
                &["1", "2", "3", "4", "5", "6", "DONE", "KEEP"][..],
            ),
            (
                "DONE_CYCLE",
                "DONE_CYCLE",
                &["1", "2", "3", "4", "5", "6", "KEEP"][..],
            ),
            (
                "LCK_CYCLE",
                "LCK_CYCLE",
                &["0", "1", "2", "3", "4", "5", "6", "NOWAIT"][..],
            ),
            ("DRIVE_DONE", "DRIVEDONE", &["NO", "YES"][..]),
            ("DONE_PIPE", "DONEPIPE", &["NO", "YES"][..]),
            (
                "CONFIG_RATE",
                "CONFIGRATE",
                &[
                    "3", "6", "9", "12", "16", "22", "26", "33", "40", "50", "66",
                ],
            ),
            (
                "DONE_SIGNALS_POWERDOWN",
                "DONESIGNALSPOWERDOWN",
                &["DISABLE", "ENABLE"][..],
            ),
        ] {
            if edev.chips.first().unwrap().has_ps && attr == "CONFIG_RATE" {
                continue;
            }

            for &val in vals {
                ctx.test_reg(reg, reg_name, bel, attr, val)
                    .global(opt, val)
                    .commit();
            }
        }
    }
    {
        let reg = Reg::Cor1;
        let reg_name = "REG.COR1";
        let bel = "MISC";
        for val in ["NO", "YES"] {
            ctx.build()
                .global("ENCRYPT", "NO")
                .global("GLUTMASK_B", "0")
                .test_reg(reg, reg_name, bel, "POST_CRC_EN", val)
                .global("POST_CRC_EN", val)
                .commit();
        }
        for val in ["CFG_CLK", "INTERNAL"] {
            ctx.build()
                .no_global("POST_CRC_FREQ")
                .test_reg(reg, reg_name, bel, "POST_CRC_CLK", val)
                .global("POST_CRC_CLK", val)
                .commit();
        }

        if !edev.chips.first().unwrap().has_ps {
            for val in ["1", "2", "3", "4"] {
                ctx.build()
                    .global("BPI_PAGE_SIZE", "8")
                    .test_reg(reg, reg_name, bel, "BPI_1ST_READ_CYCLE", val)
                    .global("BPI_1ST_READ_CYCLE", val)
                    .commit();
            }
        }
        for (attr, opt, vals) in [
            ("BPI_PAGE_SIZE", "BPI_PAGE_SIZE", &["1", "4", "8"][..]),
            (
                "POST_CRC_FREQ",
                "POST_CRC_FREQ",
                &["1", "2", "3", "6", "13", "25", "50"],
            ),
            ("POST_CRC_RECONFIG", "POST_CRC_RECONFIG", &["NO", "YES"]),
            ("POST_CRC_KEEP", "POST_CRC_KEEP", &["NO", "YES"]),
            ("POST_CRC_CORRECT", "POST_CRC_CORRECT", &["NO", "YES"]),
            ("POST_CRC_SEL", "POST_CRC_SEL", &["0", "1"]),
            (
                "POST_CRC_INIT_FLAG",
                "POST_CRC_INIT_FLAG",
                &["DISABLE", "ENABLE"],
            ),
            (
                "SYSMON_PARTIAL_RECONFIG",
                "XADCPARTIALRECONFIG",
                &["DISABLE", "ENABLE"],
            ),
            ("TRIM_BITSTREAM", "TRIM_BITSTREAM", &["DISABLE", "ENABLE"]),
        ] {
            if edev.chips.first().unwrap().has_ps && attr == "BPI_PAGE_SIZE" {
                continue;
            }

            for &val in vals {
                ctx.test_reg(reg, reg_name, bel, attr, val)
                    .global(opt, val)
                    .commit();
            }
        }
    }
    {
        let reg = Reg::Ctl0;
        let reg_name = "REG.CTL";
        let bel = "MISC";
        for val in ["DISABLE", "ENABLE"] {
            ctx.build()
                .no_global("NEXT_CONFIG_REBOOT")
                .test_reg(reg, reg_name, bel, "CONFIG_FALLBACK", val)
                .global("CONFIGFALLBACK", val)
                .commit();
        }
        for (attr, opt, vals) in [
            ("GTS_USR_B", "GTS_USR_B", &["0", "1"][..]),
            ("SEC_ALL", "SECALL", &["NO", "YES"]),
            ("SEC_ERROR", "SECERROR", &["NO", "YES"]),
            ("SEC_STATUS", "SECSTATUS", &["NO", "YES"]),
            ("SECURITY", "SECURITY", &["NONE", "LEVEL1", "LEVEL2"]),
            (
                "ENCRYPT_KEY_SELECT",
                "ENCRYPTKEYSELECT",
                &["BBRAM", "EFUSE"],
            ),
            (
                "OVERTEMP_POWERDOWN",
                "OVERTEMPPOWERDOWN",
                &["DISABLE", "ENABLE"],
            ),
            (
                "INIT_SIGNALS_ERROR",
                "INITSIGNALSERROR",
                &["DISABLE", "ENABLE"],
            ),
            ("SELECTMAP_ABORT", "SELECTMAPABORT", &["DISABLE", "ENABLE"]),
            ("PERSIST", "PERSIST", &["NO", "CTLREG"]),
        ] {
            if edev.chips.first().unwrap().has_ps && attr == "SELECTMAP_ABORT" {
                continue;
            }

            for &val in vals {
                ctx.test_reg(reg, reg_name, bel, attr, val)
                    .global(opt, val)
                    .commit();
            }
        }
    }
    {
        let reg = Reg::Ctl1;
        let reg_name = "REG.CTL1";
        let bel = "MISC";
        for val in ["DISABLE", "TEST0", "TEST1"] {
            ctx.build()
                .no_global("EN_VTEST")
                .no_global("VGG_TEST")
                .test_reg(reg, reg_name, bel, "MODE_PIN_TEST", val)
                .global("MODEPINTEST", val)
                .commit();
        }
        for (attr, opt, vals) in [
            (
                "ICAP_ENCRYPTION",
                "ICAP_ENCRYPTION",
                &["DISABLE", "ENABLE"][..],
            ),
            ("DIS_VGG_REG", "DIS_VGG_REG", &["NO", "YES"]),
            ("ENABLE_VGG_CLAMP", "ENABLE_VGG_CLAMP", &["NO", "YES"]),
        ] {
            for &val in vals {
                ctx.test_reg(reg, reg_name, bel, attr, val)
                    .global(opt, val)
                    .commit();
            }
        }
        for (opt, width) in [
            ("VGG_SEL", 5),
            ("VGG_NEG_GAIN_SEL", 5),
            ("VGG_POS_GAIN_SEL", 1),
        ] {
            ctx.test_reg(reg, reg_name, bel, opt, "")
                .multi_global(opt, MultiValue::Bin, width);
        }
    }
    if !edev.chips.first().unwrap().has_ps {
        let reg = Reg::Bspi;
        let reg_name = "REG.BSPI";
        let bel = "MISC";
        for (attr, opt, vals) in [
            (
                "BPI_SYNC_MODE",
                "BPI_SYNC_MODE",
                &["DISABLE", "TYPE1", "TYPE2"],
            ),
            ("SPI_BUSWIDTH", "SPI_BUSWIDTH", &["1", "2", "4"]),
        ] {
            for &val in vals {
                ctx.build()
                    .global("SPI_OPCODE", "0x12")
                    .test_reg(reg, reg_name, bel, attr, val)
                    .global(opt, val)
                    .commit();
            }
        }
        ctx.build()
            .global("BPI_SYNC_MODE", "TYPE1")
            .test_reg(reg, reg_name, bel, "SPI_OPCODE", "")
            .multi_global("SPI_OPCODE", MultiValue::HexPrefix, 8);
    }
    if !edev.chips.first().unwrap().has_ps {
        let reg = Reg::WbStar;
        let reg_name = "REG.WBSTAR";
        let bel = "MISC";
        for val in ["DISABLE", "ENABLE"] {
            ctx.test_reg(reg, reg_name, bel, "REVISION_SELECT_TRISTATE", val)
                .global("REVISIONSELECT_TRISTATE", val)
                .commit();
        }
        ctx.build()
            .global("NEXT_CONFIG_REBOOT", "DISABLE")
            .test_reg(reg, reg_name, bel, "NEXT_CONFIG_ADDR", "")
            .multi_global("NEXT_CONFIG_ADDR", MultiValue::HexPrefix, 29);
        ctx.build()
            .global("NEXT_CONFIG_REBOOT", "DISABLE")
            .test_reg(reg, reg_name, bel, "REVISION_SELECT", "")
            .multi_global("REVISIONSELECT", MultiValue::Bin, 2);
    }

    if edev.chips.first().unwrap().regs != 1 {
        ctx.build()
            .extra_tile_reg(Reg::Ctl0, "REG.CTL", "MISC")
            .extra_tile_reg(Reg::Ctl1, "REG.CTL1", "MISC")
            .no_global("VGG_SEL")
            .no_global("VGG_POS_GAIN_SEL")
            .no_global("VGG_NEG_GAIN_SEL")
            .test_manual("MISC", "ENCRYPT", "YES")
            .global("ENCRYPT", "YES")
            .commit();
    }

    if !edev.chips.first().unwrap().has_ps {
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

    for (reg_name, reg, attr, width, anchor, anchor_val) in [
        (
            "REG.TESTMODE",
            Reg::Testmode,
            "TEST_REF_SEL",
            3,
            "TEST_VGG_ENABLE",
            "1",
        ),
        (
            "REG.TESTMODE",
            Reg::Testmode,
            "TEST_VGG_SEL",
            4,
            "TEST_VGG_ENABLE",
            "1",
        ),
        (
            "REG.TESTMODE",
            Reg::Testmode,
            "TEST_NEG_SLOPE_VGG",
            1,
            "TEST_VGG_ENABLE",
            "1",
        ),
        (
            "REG.TESTMODE",
            Reg::Testmode,
            "TEST_VGG_ENABLE",
            1,
            "TEST_NEG_SLOPE_VGG",
            "1",
        ),
        ("REG.TRIM0", Reg::Trim0, "MPD_SEL", 3, "MPD_OVERRIDE", "1"),
        (
            "REG.TRIM0",
            Reg::Trim0,
            "TRIM_SPARE",
            2,
            "MPD_OVERRIDE",
            "1",
        ),
        (
            "REG.TRIM0",
            Reg::Trim0,
            "MPD_DIS_OVERRIDE",
            1,
            "MPD_OVERRIDE",
            "1",
        ),
        (
            "REG.TRIM0",
            Reg::Trim0,
            "MPD_OVERRIDE",
            1,
            "MPD_DIS_OVERRIDE",
            "1",
        ),
        (
            "REG.TRIM1",
            Reg::Trim1,
            "VGGSEL",
            6,
            "VBG_FLAT_SEL",
            "111111",
        ),
        (
            "REG.TRIM1",
            Reg::Trim1,
            "VGGSEL2",
            6,
            "VBG_FLAT_SEL",
            "111111",
        ),
        (
            "REG.TRIM1",
            Reg::Trim1,
            "VBG_FLAT_SEL",
            6,
            "VGGSEL",
            "111111",
        ),
        (
            "REG.TRIM2",
            Reg::Trim2,
            "VGG_TRIM_BOT",
            12,
            "VGG_TRIM_TOP",
            "111111111111",
        ),
        (
            "REG.TRIM2",
            Reg::Trim2,
            "VGG_TRIM_TOP",
            12,
            "VGG_TRIM_BOT",
            "111111111111",
        ),
    ] {
        ctx.build()
            .global(anchor, anchor_val)
            .test_reg(reg, reg_name, "MISC", attr, "")
            .multi_global(attr, MultiValue::Bin, width);
    }

    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, "SYSMON") {
        let mut bctx = ctx.bel(defs::bslots::SYSMON);
        bctx.build()
            .extra_tile_attr(
                Delta::new(0, 0, "HCLK"),
                "HCLK",
                "DRP_MASK_ABOVE_L",
                "SYSMON",
            )
            .test_manual("ENABLE", "1")
            .mode("XADC")
            .commit();
        bctx.mode("XADC").test_inv("DCLK");
        bctx.mode("XADC").test_inv("CONVSTCLK");
        for i in 0x40..0x60 {
            bctx.mode("XADC")
                .global_mutex("SYSMON", "SYSMON")
                .test_multi_attr_hex(format!("INIT_{i:02X}"), 16);
        }
        for attr in [
            "SYSMON_TEST_A",
            "SYSMON_TEST_B",
            "SYSMON_TEST_C",
            "SYSMON_TEST_D",
            "SYSMON_TEST_E",
        ] {
            bctx.mode("XADC")
                .global_mutex("SYSMON", "SYSMON")
                .test_multi_attr_hex(attr, 16);
        }
        let mut ctx = FuzzCtx::new_null(session, backend);
        for (attr, vals) in [
            ("JTAG_XADC", &["ENABLE", "DISABLE", "STATUSONLY"][..]),
            ("XADCPOWERDOWN", &["ENABLE", "DISABLE"][..]),
            ("XADCENHANCEDLINEARITY", &["ON", "OFF"][..]),
        ] {
            for &val in vals {
                ctx.build()
                    .global_mutex("SYSMON", "OPT")
                    .extra_tiles_by_bel(defs::bslots::SYSMON, "SYSMON")
                    .test_manual("SYSMON", attr, val)
                    .global(attr, val)
                    .commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };
    let tile = "CFG";
    {
        let bel = "MISC";
        for (attr, vals) in [
            ("M0PIN", &["PULLNONE", "PULLDOWN", "PULLUP"][..]),
            ("M1PIN", &["PULLNONE", "PULLDOWN", "PULLUP"][..]),
            ("M2PIN", &["PULLNONE", "PULLDOWN", "PULLUP"][..]),
            ("TDIPIN", &["PULLNONE", "PULLDOWN", "PULLUP"][..]),
            ("TDOPIN", &["PULLNONE", "PULLDOWN", "PULLUP"][..]),
            ("TMSPIN", &["PULLNONE", "PULLDOWN", "PULLUP"][..]),
            ("TCKPIN", &["PULLNONE", "PULLDOWN", "PULLUP"][..]),
            ("PROGPIN", &["PULLNONE", "PULLUP"][..]),
            ("INITPIN", &["PULLNONE", "PULLUP"][..]),
            ("DONEPIN", &["PULLNONE", "PULLUP"][..]),
            ("CCLKPIN", &["PULLNONE", "PULLUP"][..]),
        ] {
            ctx.collect_enum(tile, bel, attr, vals);
        }
    }
    for i in 0..4 {
        let bel = &format!("BSCAN[{i}]");
        ctx.collect_bit(tile, bel, "ENABLE", "1");
        let item = ctx.extract_enum_bool_wide(tile, bel, "DISABLE_JTAG", "FALSE", "TRUE");
        ctx.insert(tile, "BSCAN_COMMON", "DISABLE_JTAG", item);
    }
    {
        let bel = "BSCAN_COMMON";
        ctx.collect_bitvec(tile, bel, "USERID", "");
    }
    if edev.chips.len() == 1 && !edev.chips.first().unwrap().has_ps {
        for bel in ["ICAP[0]", "ICAP[1]"] {
            ctx.collect_bit_wide(tile, bel, "ENABLE", "1");
            // ???
            ctx.get_diff(tile, bel, "ICAP_AUTO_SWITCH", "DISABLE")
                .assert_empty();
            ctx.get_diff(tile, bel, "ICAP_AUTO_SWITCH", "ENABLE")
                .assert_empty();
        }

        let item0 = ctx.extract_enum(tile, "ICAP[0]", "ICAP_WIDTH", &["X8", "X16", "X32"]);
        let item1 = ctx.extract_enum(tile, "ICAP[1]", "ICAP_WIDTH", &["X8", "X16", "X32"]);
        assert_eq!(item0, item1);
        ctx.insert(tile, "ICAP_COMMON", "ICAP_WIDTH", item0);
    }

    {
        let bel = "STARTUP";
        ctx.collect_enum_bool(tile, bel, "GSR_SYNC", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "GTS_SYNC", "NO", "YES");
        let item0 = ctx.extract_bit_wide(tile, bel, "PIN.GSR", "1");
        let item1 = ctx.extract_bit_wide(tile, bel, "PIN.GTS", "1");
        assert_eq!(item0, item1);
        ctx.insert(tile, bel, "GTS_GSR_ENABLE", item0);
        ctx.collect_enum_bool_wide(tile, bel, "PROG_USR", "FALSE", "TRUE");
        let item = ctx.extract_bit_wide(tile, bel, "PIN.USRCCLKO", "1");
        ctx.insert(tile, bel, "USRCCLK_ENABLE", item);
        if edev.chips.first().unwrap().regs > 1 {
            let item = ctx.extract_bit_wide(tile, bel, "PIN.KEYCLEARB", "1");
            ctx.insert(tile, bel, "KEY_CLEAR_ENABLE", item);
        }
    }
    {
        let bel = "DCIRESET";
        ctx.collect_bit_wide(tile, bel, "ENABLE", "1");
    }
    if edev.chips.len() == 1 {
        let bel = "CFG_IO_ACCESS";
        ctx.collect_bit_wide(tile, bel, "ENABLE", "1");
        ctx.get_diff(tile, bel, "TDO", "UNCONNECTED").assert_empty();
    }
    {
        let bel = "DNA_PORT";
        ctx.collect_bit_wide(tile, bel, "ENABLE", "1");
    }

    {
        let tile = "REG.COR";
        let bel = "STARTUP";
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
        ctx.collect_enum(
            tile,
            bel,
            "MATCH_CYCLE",
            &["0", "1", "2", "3", "4", "5", "6", "NOWAIT"],
        );
        if edev.chips.len() == 1 {
            ctx.collect_enum(tile, bel, "STARTUPCLK", &["CCLK", "USERCLK", "JTAGCLK"]);
        }
        if !edev.chips.first().unwrap().has_ps {
            ctx.collect_enum_ocd(
                tile,
                bel,
                "CONFIG_RATE",
                &[
                    "3", "6", "9", "12", "16", "22", "26", "33", "40", "50", "66",
                ],
                OcdMode::BitOrder,
            );
        }
        ctx.collect_enum_bool(tile, bel, "DRIVE_DONE", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "DONE_PIPE", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "DONE_SIGNALS_POWERDOWN", "DISABLE", "ENABLE");
        let bel = "CAPTURE";
        if edev.chips.len() == 1 {
            ctx.collect_enum_bool(tile, bel, "ONESHOT", "FALSE", "TRUE");
        }
        ctx.insert(
            tile,
            bel,
            "EXTMASTERCCLK_EN",
            TileItem::from_bit(TileBit::new(0, 0, 26), false),
        );
        ctx.insert(
            tile,
            bel,
            "EXTMASTERCCLK_DIV",
            TileItem {
                bits: vec![TileBit::new(0, 0, 21), TileBit::new(0, 0, 22)],
                kind: TileItemKind::Enum {
                    values: [
                        ("8".to_string(), bits![0, 0]),
                        ("4".to_string(), bits![1, 0]),
                        ("2".to_string(), bits![0, 1]),
                        ("1".to_string(), bits![1, 1]),
                    ]
                    .into_iter()
                    .collect(),
                },
            },
        );
    }
    {
        let tile = "REG.COR1";
        let bel = "MISC";

        if !edev.chips.first().unwrap().has_ps {
            ctx.collect_enum(tile, bel, "BPI_PAGE_SIZE", &["1", "4", "8"]);
            ctx.collect_enum(tile, bel, "BPI_1ST_READ_CYCLE", &["1", "2", "3", "4"]);
        }
        ctx.collect_enum(tile, bel, "POST_CRC_CLK", &["CFG_CLK", "INTERNAL"]);
        let mut diffs = vec![];
        for val in ["1", "2", "3", "6", "13", "25", "50"] {
            let mut diff = ctx.get_diff(tile, bel, "POST_CRC_FREQ", val);
            diff.apply_enum_diff(ctx.item(tile, bel, "POST_CRC_CLK"), "INTERNAL", "CFG_CLK");
            diffs.push((val, diff));
        }
        ctx.insert(
            tile,
            bel,
            "POST_CRC_FREQ",
            xlat_enum_ocd(diffs, OcdMode::BitOrder),
        );

        ctx.collect_enum_bool(tile, bel, "POST_CRC_EN", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "POST_CRC_RECONFIG", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "POST_CRC_KEEP", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "POST_CRC_CORRECT", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "POST_CRC_SEL", "0", "1");
        ctx.collect_enum_bool(tile, bel, "POST_CRC_INIT_FLAG", "DISABLE", "ENABLE");
        ctx.collect_enum_bool(tile, bel, "SYSMON_PARTIAL_RECONFIG", "DISABLE", "ENABLE");
        ctx.collect_enum_bool(tile, bel, "TRIM_BITSTREAM", "DISABLE", "ENABLE");
        ctx.insert(
            tile,
            bel,
            "PERSIST_DEASSERT_AT_DESYNC",
            TileItem::from_bit(TileBit::new(0, 0, 17), false),
        );
        let item = ctx.extract_bit(tile, "CFG_IO_ACCESS", "ENABLE", "1");
        let item2 = xlat_bit(!ctx.get_diff(tile, "CFG_IO_ACCESS", "TDO", "UNCONNECTED"));
        assert_eq!(item, item2);
        ctx.insert(tile, "MISC", "CFG_IO_ACCESS_TDO", item);
        ctx.insert(
            tile,
            bel,
            "TRIM_REG",
            TileItem::from_bitvec(vec![TileBit::new(0, 0, 10), TileBit::new(0, 0, 11)], false),
        );
    }
    {
        let tile = "REG.CTL";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "GTS_USR_B", "0", "1");
        ctx.collect_enum(tile, bel, "SECURITY", &["NONE", "LEVEL1", "LEVEL2"]);
        ctx.collect_enum_bool(tile, bel, "OVERTEMP_POWERDOWN", "DISABLE", "ENABLE");
        ctx.collect_enum_bool(tile, bel, "CONFIG_FALLBACK", "DISABLE", "ENABLE");
        ctx.collect_enum_bool(tile, bel, "INIT_SIGNALS_ERROR", "DISABLE", "ENABLE");
        if !edev.chips.first().unwrap().has_ps {
            ctx.collect_enum_bool(tile, bel, "SELECTMAP_ABORT", "DISABLE", "ENABLE");
        }
        ctx.collect_enum(tile, bel, "ENCRYPT_KEY_SELECT", &["BBRAM", "EFUSE"]);
        ctx.collect_enum_bool(tile, bel, "SEC_ALL", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "SEC_ERROR", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "SEC_STATUS", "NO", "YES");
        if edev.chips.first().unwrap().regs > 1 {
            ctx.collect_bit(tile, bel, "ENCRYPT", "YES");
        }
        ctx.collect_enum_bool(tile, bel, "PERSIST", "NO", "CTLREG");
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
        let item = ctx.extract_bit(tile, bel, "ENABLE", "1");
        ctx.insert(tile, "MISC", "GLUTMASK", item);
        ctx.collect_enum(tile, bel, "FARSRC", &["FAR", "EFAR"]);
    }
    {
        let tile = "REG.CTL1";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "ICAP_ENCRYPTION", "DISABLE", "ENABLE");
        ctx.collect_enum_bool(tile, bel, "DIS_VGG_REG", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "ENABLE_VGG_CLAMP", "NO", "YES");
        ctx.collect_enum(tile, bel, "MODE_PIN_TEST", &["DISABLE", "TEST0", "TEST1"]);
        ctx.collect_bitvec(tile, bel, "VGG_SEL", "");
        ctx.collect_bitvec(tile, bel, "VGG_NEG_GAIN_SEL", "");
        ctx.collect_bitvec(tile, bel, "VGG_POS_GAIN_SEL", "");
        if edev.chips.first().unwrap().regs > 1 {
            let mut diff = ctx.get_diff(tile, bel, "ENCRYPT", "YES");
            diff.apply_bitvec_diff_int(ctx.item(tile, bel, "VGG_POS_GAIN_SEL"), 1, 0);
            diff.apply_bitvec_diff_int(ctx.item(tile, bel, "VGG_NEG_GAIN_SEL"), 0xf, 0);
            diff.apply_bitvec_diff_int(ctx.item(tile, bel, "VGG_SEL"), 0xf, 0);
            diff.assert_empty();
        }
    }
    if !edev.chips.first().unwrap().has_ps {
        let tile = "REG.BSPI";
        let bel = "MISC";
        ctx.collect_enum(tile, bel, "SPI_BUSWIDTH", &["1", "2", "4"]);
        ctx.collect_bitvec(tile, bel, "SPI_OPCODE", "");
        let mut item =
            TileItem::from_bitvec((12..28).map(|i| TileBit::new(0, 0, i)).collect(), false);
        ctx.get_diff(tile, bel, "BPI_SYNC_MODE", "DISABLE")
            .assert_empty();
        let type1 = extract_bitvec_val(
            &item,
            &bits![0; 16],
            ctx.get_diff(tile, bel, "BPI_SYNC_MODE", "TYPE1"),
        );
        let type2 = extract_bitvec_val(
            &item,
            &bits![0; 16],
            ctx.get_diff(tile, bel, "BPI_SYNC_MODE", "TYPE2"),
        );
        item.kind = TileItemKind::Enum {
            values: [
                ("NONE".to_string(), bits![0; 16]),
                ("TYPE1".to_string(), type1),
                ("TYPE2".to_string(), type2),
            ]
            .into_iter()
            .collect(),
        };
        ctx.insert(tile, bel, "BPI_SYNC_MODE", item);
    }
    if !edev.chips.first().unwrap().has_ps {
        let tile = "REG.WBSTAR";
        let bel = "MISC";
        ctx.collect_bitvec(tile, bel, "NEXT_CONFIG_ADDR", "");
        ctx.collect_bitvec(tile, bel, "REVISION_SELECT", "");
        ctx.collect_enum_bool(tile, bel, "REVISION_SELECT_TRISTATE", "DISABLE", "ENABLE");
    }
    if !edev.chips.first().unwrap().has_ps {
        let tile = "REG.TIMER";
        let bel = "MISC";
        ctx.collect_bitvec(tile, bel, "TIMER", "");
        ctx.collect_bit(tile, bel, "TIMER_CFG", "1");
        ctx.collect_bit(tile, bel, "TIMER_USR", "1");
    }
    {
        let tile = "REG.TESTMODE";
        let bel = "MISC";
        for attr in [
            "TEST_REF_SEL",
            "TEST_VGG_SEL",
            "TEST_NEG_SLOPE_VGG",
            "TEST_VGG_ENABLE",
        ] {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
    }
    {
        let tile = "REG.TRIM0";
        let bel = "MISC";
        for attr in ["MPD_SEL", "TRIM_SPARE", "MPD_DIS_OVERRIDE", "MPD_OVERRIDE"] {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
    }
    {
        let tile = "REG.TRIM1";
        let bel = "MISC";
        for attr in ["VGGSEL", "VGGSEL2", "VBG_FLAT_SEL"] {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
    }
    {
        let tile = "REG.TRIM2";
        let bel = "MISC";
        for attr in ["VGG_TRIM_BOT", "VGG_TRIM_TOP"] {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
    }

    if ctx.has_tile("SYSMON") {
        let tile = "SYSMON";
        let bel = "SYSMON";
        ctx.get_diff(tile, bel, "ENABLE", "1").assert_empty();
        ctx.collect_inv(tile, bel, "CONVSTCLK");
        ctx.collect_inv(tile, bel, "DCLK");
        for i in 0x40..0x60 {
            ctx.collect_bitvec(tile, bel, &format!("INIT_{i:02X}"), "");
        }
        ctx.collect_bitvec(tile, bel, "SYSMON_TEST_A", "");
        ctx.collect_bitvec(tile, bel, "SYSMON_TEST_B", "");
        ctx.collect_bitvec(tile, bel, "SYSMON_TEST_C", "");
        ctx.collect_bitvec(tile, bel, "SYSMON_TEST_D", "");
        ctx.collect_bitvec(tile, bel, "SYSMON_TEST_E", "");

        ctx.get_diff(tile, bel, "JTAG_XADC", "ENABLE")
            .assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "JTAG_XADC", "DISABLE");
        diff.apply_bitvec_diff_int(ctx.item(tile, bel, "SYSMON_TEST_E"), 7, 0);
        diff.assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "JTAG_XADC", "STATUSONLY");
        diff.apply_bitvec_diff_int(ctx.item(tile, bel, "SYSMON_TEST_E"), 0xc8, 0);
        diff.assert_empty();

        let mut diff = ctx.get_diff(tile, bel, "XADCENHANCEDLINEARITY", "ON");
        diff.apply_bitvec_diff_int(ctx.item(tile, bel, "SYSMON_TEST_C"), 0x10, 0);
        diff.assert_empty();
        ctx.get_diff(tile, bel, "XADCENHANCEDLINEARITY", "OFF")
            .assert_empty();

        let mut diff = ctx.get_diff(tile, bel, "XADCPOWERDOWN", "ENABLE");
        diff.apply_bitvec_diff_int(ctx.item(tile, bel, "INIT_42"), 0x30, 0);
        diff.assert_empty();
        ctx.get_diff(tile, bel, "XADCPOWERDOWN", "DISABLE")
            .assert_empty();

        let tile = "HCLK";
        let bel = "HCLK";
        ctx.collect_bit(tile, bel, "DRP_MASK_ABOVE_L", "SYSMON");
    }
}
