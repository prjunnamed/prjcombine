use prjcombine_re_fpga_hammer::{OcdMode, xlat_bit, xlat_bitvec};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{
    bits,
    bsdata::{TileBit, TileItem, TileItemKind},
};
use prjcombine_virtex4::bels;
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

    for i in 0..32 {
        let mut bctx = ctx.bel(bels::BUFGCTRL[i]);
        bctx.build()
            .test_manual("PRESENT", "1")
            .mode("BUFGCTRL")
            .commit();
        for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
            bctx.mode("BUFGCTRL").test_inv(pin);
        }
        bctx.mode("BUFGCTRL")
            .test_enum("PRESELECT_I0", &["FALSE", "TRUE"]);
        bctx.mode("BUFGCTRL")
            .test_enum("PRESELECT_I1", &["FALSE", "TRUE"]);
        bctx.mode("BUFGCTRL")
            .test_enum("CREATE_EDGE", &["FALSE", "TRUE"]);
        bctx.mode("BUFGCTRL").test_enum("INIT_OUT", &["0", "1"]);

        for j in 0..2 {
            for val in ["CKINT0", "CKINT1"] {
                bctx.build()
                    .mutex(format!("MUX.I{j}"), val)
                    .test_manual(format!("MUX.I{j}"), val)
                    .pip(format!("I{j}MUX"), val)
                    .commit();
            }
            bctx.build()
                .mutex(format!("MUX.I{j}"), "MUXBUS")
                .test_manual(format!("MUX.I{j}"), "MUXBUS")
                .pip(format!("I{j}MUX"), format!("MUXBUS{j}"))
                .commit();
            for k in 0..16 {
                let obel = bels::BUFGCTRL[if i < 16 { k } else { k + 16 }];
                let val = format!("GFB{k}");
                bctx.build()
                    .mutex(format!("MUX.I{j}"), &val)
                    .test_manual(format!("MUX.I{j}"), val)
                    .pip(format!("I{j}MUX"), (obel, "GFB"))
                    .commit();
            }
            for k in 0..5 {
                for lr in ['L', 'R'] {
                    let val = format!("MGT_{lr}{k}");
                    let pin = format!("MGT_O_{lr}{k}");
                    let obel = if i < 16 {
                        bels::BUFG_MGTCLK_S
                    } else {
                        bels::BUFG_MGTCLK_N
                    };
                    bctx.build()
                        .mutex(format!("MUX.I{j}"), &val)
                        .test_manual(format!("MUX.I{j}"), &val)
                        .pip(format!("I{j}MUX"), (obel, pin))
                        .commit();
                }
            }
        }
        bctx.build()
            .test_manual("I0_FABRIC_OUT", "1")
            .pin_pips("I0MUX")
            .commit();
        bctx.build()
            .test_manual("I1_FABRIC_OUT", "1")
            .pin_pips("I1MUX")
            .commit();
    }
    for i in 0..4 {
        let mut bctx = ctx.bel(bels::BSCAN[i]);
        bctx.test_manual("ENABLE", "1").mode("BSCAN").commit();
    }
    ctx.test_manual("BSCAN_COMMON", "USERID", "")
        .multi_global("USERID", MultiValue::HexPrefix, 32);
    for i in 0..2 {
        let mut bctx = ctx.bel(bels::ICAP[i]);
        bctx.build()
            .test_manual("ENABLE", "1")
            .mode("ICAP")
            .commit();
        bctx.mode("ICAP")
            .global_mutex_here("ICAP")
            .test_enum("ICAP_WIDTH", &["X8", "X16", "X32"]);
    }
    {
        let mut bctx = ctx.bel(bels::PMV0);
        bctx.build()
            .null_bits()
            .test_manual("PRESENT", "1")
            .mode("PMV")
            .commit();
    }
    {
        let mut bctx = ctx.bel(bels::STARTUP);
        bctx.build()
            .null_bits()
            .test_manual("PRESENT", "1")
            .mode("STARTUP")
            .commit();
        for val in ["CCLK", "USERCLK", "JTAGCLK"] {
            bctx.mode("STARTUP")
                .null_bits()
                .pin("CLK")
                .extra_tile_reg(Reg::Cor0, "REG.COR", "STARTUP")
                .test_manual("STARTUPCLK", val)
                .global("STARTUPCLK", val)
                .commit();
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
        for attr in ["GSR_SYNC", "GTS_SYNC"] {
            for val in ["YES", "NO"] {
                bctx.test_manual(attr, val).global(attr, val).commit();
            }
        }
    }
    {
        let mut bctx = ctx.bel(bels::JTAGPPC);
        bctx.build()
            .null_bits()
            .test_manual("PRESENT", "1")
            .mode("JTAGPPC")
            .commit();
        bctx.mode("JTAGPPC")
            .test_enum("NUM_PPC", &["0", "1", "2", "3", "4"]);
    }
    {
        let mut bctx = ctx.bel(bels::FRAME_ECC);
        bctx.build()
            .null_bits()
            .test_manual("PRESENT", "1")
            .mode("FRAME_ECC")
            .commit();
    }
    {
        let mut bctx = ctx.bel(bels::DCIRESET);
        bctx.build()
            .test_manual("ENABLE", "1")
            .mode("DCIRESET")
            .commit();
    }
    {
        let mut bctx = ctx.bel(bels::CAPTURE);
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
    {
        let mut bctx = ctx.bel(bels::USR_ACCESS);
        bctx.build()
            .null_bits()
            .test_manual("PRESENT", "1")
            .mode("USR_ACCESS")
            .commit();
    }
    {
        let mut bctx = ctx.bel(bels::KEY_CLEAR);
        bctx.build()
            .null_bits()
            .test_manual("PRESENT", "1")
            .mode("KEY_CLEAR")
            .commit();
    }
    {
        let mut bctx = ctx.bel(bels::EFUSE_USR);
        bctx.build()
            .null_bits()
            .test_manual("PRESENT", "1")
            .mode("EFUSE_USR")
            .commit();
    }
    {
        let mut bctx = ctx.bel(bels::SYSMON);
        bctx.build()
            .null_bits()
            .extra_tile_attr(
                Delta::new(0, 20, "HCLK_IOI_TOPCEN"),
                "SYSMON",
                "ENABLE",
                "1",
            )
            .test_manual("ENABLE", "1")
            .mode("SYSMON")
            .commit();
        bctx.mode("SYSMON").test_inv("DCLK");
        bctx.mode("SYSMON").test_inv("CONVSTCLK");
        for i in 0x40..0x58 {
            bctx.mode("SYSMON")
                .test_multi_attr_hex(format!("INIT_{i:02X}"), 16);
        }
        for attr in [
            "SYSMON_TEST_A",
            "SYSMON_TEST_B",
            "SYSMON_TEST_C",
            "SYSMON_TEST_D",
            "SYSMON_TEST_E",
        ] {
            bctx.mode("SYSMON").test_multi_attr_hex(attr, 16);
        }
        bctx.build()
            .attr("SYSMON_TEST_A", "")
            .test_manual("JTAG_SYSMON", "DISABLE")
            .global("JTAG_SYSMON", "DISABLE")
            .commit();
    }

    for bel in [bels::BUFG_MGTCLK_S, bels::BUFG_MGTCLK_N] {
        let mut bctx = ctx.bel(bel);
        for i in 0..5 {
            for lr in ['L', 'R'] {
                if lr == 'L' && edev.col_lgt.is_none() {
                    continue;
                }
                bctx.build()
                    .test_manual(format!("BUF.MGT_{lr}{i}"), "1")
                    .pip(format!("MGT_O_{lr}{i}"), format!("MGT_I_{lr}{i}"))
                    .commit();
            }
        }
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
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };
    let tile = "CFG";
    let bel = "MISC";
    for attr in ["CCLKPIN", "DONEPIN", "PROGPIN", "INITPIN"] {
        ctx.collect_enum(tile, bel, attr, &["PULLUP", "PULLNONE"]);
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
        ctx.collect_enum(tile, bel, attr, &["PULLUP", "PULLDOWN", "PULLNONE"]);
    }

    for i in 0..32 {
        let bel = format!("BUFGCTRL{i}");
        let bel = &bel;
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
            ctx.collect_inv(tile, bel, pin);
        }
        ctx.collect_enum_bool(tile, bel, "PRESELECT_I0", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "PRESELECT_I1", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "CREATE_EDGE", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "INIT_OUT", "0", "1");

        for attr in ["MUX.I0", "MUX.I1"] {
            ctx.collect_enum_default_ocd(
                tile,
                bel,
                attr,
                &[
                    "MUXBUS", "CKINT0", "CKINT1", "GFB0", "GFB1", "GFB2", "GFB3", "GFB4", "GFB5",
                    "GFB6", "GFB7", "GFB8", "GFB9", "GFB10", "GFB11", "GFB12", "GFB13", "GFB14",
                    "GFB15", "MGT_L0", "MGT_L1", "MGT_L2", "MGT_L3", "MGT_L4", "MGT_R0", "MGT_R1",
                    "MGT_R2", "MGT_R3", "MGT_R4",
                ],
                "NONE",
                OcdMode::Mux,
            );
        }

        ctx.collect_bit(tile, bel, "I0_FABRIC_OUT", "1");
        ctx.collect_bit(tile, bel, "I1_FABRIC_OUT", "1");
    }

    for bel in [
        "BSCAN0", "BSCAN1", "BSCAN2", "BSCAN3", "DCIRESET", "ICAP0", "ICAP1",
    ] {
        ctx.collect_bit(tile, bel, "ENABLE", "1");
    }
    let bel = "BSCAN_COMMON";
    let item = xlat_bitvec(ctx.state.get_diffs(tile, bel, "USERID", ""));
    ctx.tiledb.insert(tile, bel, "USERID", item);

    let bel = "STARTUP";
    ctx.collect_enum_bool(tile, bel, "GSR_SYNC", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "GTS_SYNC", "NO", "YES");
    let item0 = ctx.extract_bit(tile, bel, "PIN.GSR", "1");
    let item1 = ctx.extract_bit(tile, bel, "PIN.GTS", "1");
    assert_eq!(item0, item1);
    ctx.tiledb.insert(tile, bel, "GTS_GSR_ENABLE", item0);
    let item = ctx.extract_bit(tile, bel, "PIN.USRCCLKO", "1");
    ctx.tiledb.insert(tile, bel, "USRCCLK_ENABLE", item);

    let item0 = ctx.extract_enum(tile, "ICAP0", "ICAP_WIDTH", &["X8", "X16", "X32"]);
    let item1 = ctx.extract_enum(tile, "ICAP1", "ICAP_WIDTH", &["X8", "X16", "X32"]);
    assert_eq!(item0, item1);
    ctx.tiledb.insert(tile, "ICAP_COMMON", "ICAP_WIDTH", item0);

    {
        let bel = "JTAGPPC";
        ctx.collect_enum(tile, bel, "NUM_PPC", &["0", "1", "2", "3", "4"]);
    }

    {
        let bel = "SYSMON";
        ctx.collect_inv(tile, bel, "CONVSTCLK");
        ctx.collect_inv(tile, bel, "DCLK");
        for i in 0x40..0x58 {
            ctx.collect_bitvec(tile, bel, &format!("INIT_{i:02X}"), "");
        }
        ctx.collect_bitvec(tile, bel, "SYSMON_TEST_A", "");
        ctx.collect_bitvec(tile, bel, "SYSMON_TEST_B", "");
        ctx.collect_bitvec(tile, bel, "SYSMON_TEST_C", "");
        ctx.collect_bitvec(tile, bel, "SYSMON_TEST_D", "");
        ctx.collect_bitvec(tile, bel, "SYSMON_TEST_E", "");

        let mut diff = ctx.state.get_diff(tile, bel, "JTAG_SYSMON", "DISABLE");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "SYSMON_TEST_A"), 2, 0);
        diff.assert_empty();
    }

    {
        let tile = "HCLK_IOI_TOPCEN";
        let bel = "SYSMON";
        ctx.collect_bit(tile, bel, "ENABLE", "1");
    }

    for bel in ["BUFG_MGTCLK_S", "BUFG_MGTCLK_N"] {
        for i in 0..5 {
            for lr in ['L', 'R'] {
                if lr == 'L' && edev.col_lgt.is_none() {
                    continue;
                }
                ctx.collect_bit(tile, bel, &format!("BUF.MGT_{lr}{i}"), "1");
            }
        }
    }

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
    ctx.collect_enum(tile, bel, "STARTUPCLK", &["CCLK", "USERCLK", "JTAGCLK"]);
    ctx.collect_enum_ocd(
        tile,
        bel,
        "CONFIG_RATE",
        &[
            "2", "6", "9", "13", "17", "20", "24", "27", "31", "35", "38", "42", "46", "49", "53",
            "56", "60",
        ],
        OcdMode::BitOrder,
    );
    ctx.collect_enum_bool(tile, bel, "DRIVE_DONE", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "DONE_PIPE", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "CRC", "DISABLE", "ENABLE");
    let bel = "CAPTURE";
    ctx.collect_enum_bool(tile, bel, "ONESHOT", "FALSE", "TRUE");

    let tile = "REG.COR1";
    let bel = "MISC";
    ctx.collect_enum(tile, bel, "BPI_PAGE_SIZE", &["1", "4", "8"]);
    ctx.collect_enum(tile, bel, "BPI_1ST_READ_CYCLE", &["1", "2", "3", "4"]);
    ctx.collect_enum_bool(tile, bel, "POST_CRC_EN", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "POST_CRC_NO_PIN", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "POST_CRC_RECONFIG", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "RETAIN_CONFIG_STATUS", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "POST_CRC_SEL", "0", "1");

    let tile = "REG.CTL";
    let bel = "MISC";
    ctx.collect_enum(tile, bel, "SECURITY", &["NONE", "LEVEL1", "LEVEL2"]);
    ctx.collect_enum_bool(tile, bel, "ENCRYPT", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "OVERTEMP_POWERDOWN", "DISABLE", "ENABLE");
    ctx.collect_enum_bool(tile, bel, "CONFIG_FALLBACK", "DISABLE", "ENABLE");
    ctx.collect_enum_bool(tile, bel, "SELECTMAP_ABORT", "DISABLE", "ENABLE");
    ctx.collect_enum_bool(tile, bel, "GLUTMASK", "1", "0");
    ctx.collect_enum(tile, bel, "ENCRYPT_KEY_SELECT", &["BBRAM", "EFUSE"]);
    ctx.collect_bitvec(tile, bel, "VBG_SEL", "");
    ctx.collect_bitvec(tile, bel, "VBG_DLL_SEL", "");
    ctx.collect_bitvec(tile, bel, "VGG_SEL", "");
    // these are too much trouble to deal with the normal way.
    for (attr, bit) in [("GTS_USR_B", 0), ("PERSIST", 3)] {
        ctx.tiledb.insert(
            tile,
            bel,
            attr,
            TileItem {
                bits: vec![TileBit {
                    tile: 0,
                    frame: 0,
                    bit,
                }],
                kind: TileItemKind::BitVec { invert: bits![0] },
            },
        );
    }
    ctx.tiledb.insert(
        tile,
        bel,
        "ICAP_SELECT",
        TileItem {
            bits: vec![TileBit {
                tile: 0,
                frame: 0,
                bit: 30,
            }],
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
    ctx.collect_bitvec(tile, bel, "TIMER", "");
    ctx.collect_bit(tile, bel, "TIMER_CFG", "1");
    ctx.collect_bit(tile, bel, "TIMER_USR", "1");

    let tile = "REG.TESTMODE";
    let bel = "MISC";
    let mut diff = ctx.state.get_diff(tile, bel, "DD_OVERRIDE", "YES");
    diff.bits.remove(&TileBit::new(1, 0, 0));
    ctx.tiledb.insert(tile, bel, "DD_OVERRIDE", xlat_bit(diff));
}
