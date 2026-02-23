use prjcombine_re_collector::{
    diff::{OcdMode, xlat_bit_wide, xlat_bit_wide_bi},
    legacy::{xlat_bit_legacy, xlat_enum_legacy_ocd},
};
use prjcombine_re_hammer::Session;
use prjcombine_types::{
    bits,
    bsdata::{TileBit, TileItem, TileItemKind},
};
use prjcombine_virtex4::defs::{self, bcls, bslots, enums, virtex6::tcls};
use prjcombine_xilinx_bitstream::Reg;

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::relation::Delta,
    },
    virtex4::specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::CFG);

    let mut bctx = ctx.bel(bslots::MISC_CFG);
    for (attr, opt) in [
        (bcls::MISC_CFG::CCLK_PULL, "CCLKPIN"),
        (bcls::MISC_CFG::DONE_PULL, "DONEPIN"),
        (bcls::MISC_CFG::PROG_PULL, "PROGPIN"),
        (bcls::MISC_CFG::INIT_PULL, "INITPIN"),
    ] {
        for (val, vname) in [
            (enums::IOB_PULL::PULLUP, "PULLUP"),
            (enums::IOB_PULL::NONE, "PULLNONE"),
        ] {
            bctx.build()
                .test_bel_attr_val(attr, val)
                .global(opt, vname)
                .commit();
        }
    }
    for (attr, opt) in [
        (bcls::MISC_CFG::HSWAPEN_PULL, "HSWAPENPIN"),
        (bcls::MISC_CFG::M0_PULL, "M0PIN"),
        (bcls::MISC_CFG::M1_PULL, "M1PIN"),
        (bcls::MISC_CFG::M2_PULL, "M2PIN"),
        (bcls::MISC_CFG::CS_PULL, "CSPIN"),
        (bcls::MISC_CFG::DIN_PULL, "DINPIN"),
        (bcls::MISC_CFG::BUSY_PULL, "BUSYPIN"),
        (bcls::MISC_CFG::RDWR_PULL, "RDWRPIN"),
        (bcls::MISC_CFG::TCK_PULL, "TCKPIN"),
        (bcls::MISC_CFG::TDI_PULL, "TDIPIN"),
        (bcls::MISC_CFG::TDO_PULL, "TDOPIN"),
        (bcls::MISC_CFG::TMS_PULL, "TMSPIN"),
    ] {
        for (val, vname) in [
            (enums::IOB_PULL::PULLUP, "PULLUP"),
            (enums::IOB_PULL::PULLDOWN, "PULLDOWN"),
            (enums::IOB_PULL::NONE, "PULLNONE"),
        ] {
            bctx.build()
                .test_bel_attr_val(attr, val)
                .global(opt, vname)
                .commit();
        }
    }
    bctx.build()
        .test_bel_attr_bits(bcls::MISC_CFG::USERCODE)
        .multi_global("USERID", MultiValue::HexPrefix, 32);

    for i in 0..4 {
        let mut bctx = ctx.bel(defs::bslots::BSCAN[i]);
        bctx.build()
            .test_bel_attr_bits(bcls::BSCAN::ENABLE)
            .mode("BSCAN")
            .commit();
        bctx.mode("BSCAN")
            .global_mutex_here("DISABLE_JTAG")
            .test_bel(bslots::MISC_CFG)
            .test_bel_attr_bool_rename(
                "DISABLE_JTAG",
                bcls::MISC_CFG::DISABLE_JTAG_TR,
                "FALSE",
                "TRUE",
            );
    }

    {
        let mut bctx = ctx.bel(defs::bslots::ICAP[1]);
        bctx.build()
            .test_bel_attr_bits(bcls::ICAP_V6::ENABLE_TR)
            .mode("ICAP")
            .commit();
        bctx.mode("ICAP")
            .global_mutex_here("ICAP")
            .test_bel(bslots::MISC_CFG)
            .test_bel_attr_auto(bcls::MISC_CFG::ICAP_WIDTH);
        for val in ["DISABLE", "ENABLE"] {
            bctx.mode("ICAP")
                .null_bits()
                .global_mutex_here("ICAP")
                .test_bel_special(specials::ICAP_AUTO_SWITCH)
                .attr("ICAP_AUTO_SWITCH", val)
                .commit();
        }

        let mut bctx = ctx.bel(defs::bslots::ICAP[0]);
        bctx.build()
            .bel_mode(defs::bslots::ICAP[1], "ICAP")
            .test_bel_attr_bits(bcls::ICAP_V6::ENABLE_TR)
            .mode("ICAP")
            .commit();
        bctx.mode("ICAP")
            .bel_mode(defs::bslots::ICAP[1], "ICAP")
            .global_mutex_here("ICAP")
            .test_bel(bslots::MISC_CFG)
            .test_bel_attr_auto(bcls::MISC_CFG::ICAP_WIDTH);
        for val in ["DISABLE", "ENABLE"] {
            bctx.mode("ICAP")
                .null_bits()
                .bel_mode(defs::bslots::ICAP[1], "ICAP")
                .global_mutex_here("ICAP")
                .test_bel_special(specials::ICAP_AUTO_SWITCH)
                .attr("ICAP_AUTO_SWITCH", val)
                .commit();
        }
    }

    for i in 0..2 {
        let mut bctx = ctx.bel(defs::bslots::PMV_CFG[i]);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("PMV")
            .commit();
    }

    {
        let mut bctx = ctx.bel(defs::bslots::STARTUP);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
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
            .test_bel_attr_bits(bcls::STARTUP::USER_GTS_GSR_ENABLE_TR)
            .pin("GTS")
            .commit();
        bctx.mode("STARTUP")
            .no_pin("GTS")
            .test_bel_attr_bits(bcls::STARTUP::USER_GTS_GSR_ENABLE_TR)
            .pin("GSR")
            .commit();
        bctx.mode("STARTUP")
            .test_bel_attr_bits(bcls::STARTUP::USRCCLK_ENABLE_TR)
            .pin("USRCCLKO")
            .commit();
        bctx.mode("STARTUP")
            .global("ENCRYPT", "YES")
            .test_bel_attr_bits(bcls::STARTUP::KEY_CLEAR_ENABLE_TR)
            .pin("KEYCLEARB")
            .commit();
        for attr in [bcls::STARTUP::GSR_SYNC, bcls::STARTUP::GTS_SYNC] {
            bctx.build().test_global_attr_bool_rename(
                backend.edev.db[bcls::STARTUP].attributes.key(attr),
                attr,
                "NO",
                "YES",
            );
        }
        bctx.mode("STARTUP").test_bel_attr_bool_rename(
            "PROG_USR",
            bcls::STARTUP::PROG_USR_TR,
            "FALSE",
            "TRUE",
        );
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
            .test_enum_legacy("FARSRC", &["FAR", "EFAR"]);
    }

    {
        let mut bctx = ctx.bel(defs::bslots::DCIRESET);
        bctx.build()
            .test_bel_attr_bits(bcls::DCIRESET::ENABLE_TR)
            .mode("DCIRESET")
            .commit();
    }

    {
        let mut bctx = ctx.bel(defs::bslots::CAPTURE);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("CAPTURE")
            .commit();
        bctx.mode("CAPTURE")
            .null_bits()
            .extra_tile_reg(Reg::Cor0, "REG.COR", "CAPTURE")
            .test_enum_legacy("ONESHOT", &["FALSE", "TRUE"]);
    }

    {
        let mut bctx = ctx.bel(defs::bslots::USR_ACCESS);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("USR_ACCESS")
            .commit();
    }

    {
        let mut bctx = ctx.bel(defs::bslots::EFUSE_USR);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("EFUSE_USR")
            .commit();
    }

    {
        let mut bctx = ctx.bel(defs::bslots::DNA_PORT);
        bctx.build()
            .test_bel_attr_bits(bcls::DNA_PORT::ENABLE_TR)
            .mode("DNA_PORT")
            .commit();
    }

    {
        let mut bctx = ctx.bel(defs::bslots::CFG_IO_ACCESS);
        bctx.build()
            .test_bel_attr_bits(bcls::CFG_IO_ACCESS_V6::ENABLE_TR)
            .mode("CFG_IO_ACCESS")
            .commit();
    }

    {
        let mut bctx = ctx.bel(defs::bslots::SYSMON);
        bctx.build()
            .null_bits()
            .extra_tile_attr_bits(
                Delta::new(0, 20, tcls::HCLK),
                bslots::HCLK_DRP,
                bcls::HCLK_DRP_V6::DRP_MASK_SYSMON,
            )
            .test_bel_special(specials::PRESENT)
            .mode("SYSMON")
            .commit();
        bctx.mode("SYSMON")
            .test_bel_input_inv_auto(bcls::SYSMON_V5::DCLK);
        bctx.mode("SYSMON")
            .test_bel_input_inv_auto(bcls::SYSMON_V5::CONVSTCLK);
        for i in 0x40..0x58 {
            let base = (i - 0x40) * 0x10;
            bctx.mode("SYSMON")
                .test_bel_attr_bits_base(bcls::SYSMON_V5::INIT, base)
                .multi_attr(format!("INIT_{i:02X}"), MultiValue::Hex(0), 16);
        }
        for attr in [
            bcls::SYSMON_V5::SYSMON_TEST_A,
            bcls::SYSMON_V5::SYSMON_TEST_B,
            bcls::SYSMON_V5::SYSMON_TEST_C,
            bcls::SYSMON_V5::SYSMON_TEST_D,
            bcls::SYSMON_V5::SYSMON_TEST_E,
        ] {
            bctx.mode("SYSMON")
                .test_bel_attr_multi(attr, MultiValue::Hex(0));
        }
        bctx.build()
            .attr("SYSMON_TEST_E", "")
            .test_bel_special(specials::JTAG_SYSMON_DISABLE)
            .global("JTAG_SYSMON", "DISABLE")
            .commit();
    }

    let mut ctx = FuzzCtx::new_null(session, backend);

    {
        let reg_name = "REG.COR";
        let bel = "STARTUP";
        let reg = Reg::Cor0;
        for val in ["1", "2", "3", "4", "5", "6", "DONE", "KEEP"] {
            ctx.test_reg_legacy(reg, reg_name, bel, "GWE_CYCLE", val)
                .global("GWE_CYCLE", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, bel, "GTS_CYCLE", val)
                .global("GTS_CYCLE", val)
                .commit();
        }
        for val in ["1", "2", "3", "4", "5", "6", "KEEP"] {
            ctx.test_reg_legacy(reg, reg_name, bel, "DONE_CYCLE", val)
                .global("DONE_CYCLE", val)
                .commit();
        }
        for val in ["0", "1", "2", "3", "4", "5", "6", "NOWAIT"] {
            ctx.test_reg_legacy(reg, reg_name, bel, "LCK_CYCLE", val)
                .global("LCK_CYCLE", val)
                .commit();
            ctx.build()
                .global_mutex("GLOBAL_DCI", "NO")
                .test_reg(reg, reg_name, bel, "MATCH_CYCLE", val)
                .global("MATCH_CYCLE", val)
                .commit();
        }
        for val in ["NO", "YES"] {
            ctx.test_reg_legacy(reg, reg_name, bel, "DRIVE_DONE", val)
                .global("DRIVEDONE", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, bel, "DONE_PIPE", val)
                .global("DONEPIPE", val)
                .commit();
        }
        for val in [
            "2", "4", "6", "10", "12", "16", "22", "26", "33", "40", "50", "66",
        ] {
            ctx.test_reg_legacy(reg, reg_name, bel, "CONFIG_RATE", val)
                .global("CONFIGRATE", val)
                .commit();
        }
        for val in ["DISABLE", "ENABLE"] {
            ctx.test_reg_legacy(reg, reg_name, bel, "DONE_SIGNALS_POWERDOWN", val)
                .global("DONESIGNALSPOWERDOWN", val)
                .commit();
        }
    }

    {
        let reg_name = "REG.COR1";
        let bel = "MISC";
        let reg = Reg::Cor1;
        for val in ["1", "4", "8"] {
            ctx.test_reg_legacy(reg, reg_name, bel, "BPI_PAGE_SIZE", val)
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
            ctx.test_reg_legacy(reg, reg_name, bel, "POST_CRC_RECONFIG", val)
                .global("POST_CRC_RECONFIG", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, bel, "POST_CRC_KEEP", val)
                .global("POST_CRC_KEEP", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, bel, "POST_CRC_CORRECT", val)
                .global("POST_CRC_CORRECT", val)
                .commit();
        }
        for opt in ["POST_CRC_SEL", "FUSE_NO_CDR"] {
            for val in ["0", "1"] {
                ctx.test_reg_legacy(reg, reg_name, bel, opt, val)
                    .global(opt, val)
                    .commit();
            }
        }
        for val in ["1", "2", "3", "6", "13", "25", "50"] {
            ctx.test_reg_legacy(reg, reg_name, bel, "POST_CRC_FREQ", val)
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
            ctx.test_reg_legacy(reg, reg_name, bel, "SYSMON_PARTIAL_RECONFIG", val)
                .global("SYSMONPARTIALRECONFIG", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, bel, "POST_CRC_INIT_FLAG", val)
                .global("POST_CRC_INIT_FLAG", val)
                .commit();
        }
        for val in ["NO", "YES"] {
            ctx.test_reg_legacy(reg, reg_name, bel, "FALLBACK_PULSE_FWE", val)
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
            ctx.test_reg_legacy(reg, reg_name, bel, "SECURITY", val)
                .global("SECURITY", val)
                .commit();
        }
        for val in ["BBRAM", "EFUSE"] {
            ctx.test_reg_legacy(reg, reg_name, bel, "ENCRYPT_KEY_SELECT", val)
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
                ctx.test_reg_legacy(reg, reg_name, bel, attr, val)
                    .global(opt, val)
                    .commit();
            }
        }
        for val in ["0", "1"] {
            ctx.test_reg_legacy(reg, reg_name, bel, "GTS_USR_B", val)
                .global("GTS_USR_B", val)
                .commit();
        }
        for val in ["NO", "YES"] {
            ctx.test_reg_legacy(reg, reg_name, bel, "SEC_ALL", val)
                .global("SECALL", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, bel, "SEC_ERROR", val)
                .global("SECERROR", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, bel, "SEC_STATUS", val)
                .global("SECSTATUS", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, bel, "ENCRYPT", val)
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
                ctx.test_reg_legacy(reg, reg_name, bel, attr, val)
                    .global(opt, val)
                    .commit();
            }
        }
        for opt in ["VGG_TEST", "EN_VTEST", "DIS_VGG_REG", "ENABLE_VGG_CLAMP"] {
            for val in ["NO", "YES"] {
                ctx.test_reg_legacy(reg, reg_name, bel, opt, val)
                    .global(opt, val)
                    .commit();
            }
        }
        for opt in ["VGG_OPT_DRV", "VGG_V4_OPT"] {
            for val in ["0", "1"] {
                ctx.test_reg_legacy(reg, reg_name, bel, opt, val)
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
        ctx.test_reg_legacy(reg, reg_name, bel, "VBG_SEL", "")
            .multi_global("VBG_SEL", MultiValue::Bin, 6);
        ctx.test_reg_legacy(reg, reg_name, bel, "VBG_VGG_FLAST_SEL", "")
            .multi_global("VBGVGGFLASTSEL", MultiValue::Bin, 6);
        ctx.test_reg_legacy(reg, reg_name, bel, "VBG_VGG_NEG_SEL", "")
            .multi_global("VBGVGGNEGSEL", MultiValue::Bin, 6);
    }

    {
        let reg_name = "REG.TRIM";
        let bel = "MISC";
        let reg = Reg::Trim0;
        ctx.test_reg_legacy(reg, reg_name, bel, "MPD_SEL", "")
            .multi_global("MPD_SEL", MultiValue::Bin, 3);
    }

    {
        let reg_name = "REG.TESTMODE";
        let bel = "MISC";
        let reg = Reg::Testmode;
        ctx.build()
            .extra_tile_reg_present(reg, reg_name, bel)
            .test_manual_legacy(bel, "FUSE_SHADOW", "")
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
                .test_manual_legacy(bel, "CRC", val)
                .global("CRC", val)
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::CFG;

    {
        let bslot = bslots::MISC_CFG;
        for attr in [
            bcls::MISC_CFG::CCLK_PULL,
            bcls::MISC_CFG::DONE_PULL,
            bcls::MISC_CFG::PROG_PULL,
            bcls::MISC_CFG::INIT_PULL,
        ] {
            ctx.collect_bel_attr_subset(
                tcid,
                bslot,
                attr,
                &[enums::IOB_PULL::NONE, enums::IOB_PULL::PULLUP],
            );
        }
        for attr in [
            bcls::MISC_CFG::HSWAPEN_PULL,
            bcls::MISC_CFG::M0_PULL,
            bcls::MISC_CFG::M1_PULL,
            bcls::MISC_CFG::M2_PULL,
            bcls::MISC_CFG::CS_PULL,
            bcls::MISC_CFG::DIN_PULL,
            bcls::MISC_CFG::BUSY_PULL,
            bcls::MISC_CFG::RDWR_PULL,
            bcls::MISC_CFG::TCK_PULL,
            bcls::MISC_CFG::TDI_PULL,
            bcls::MISC_CFG::TDO_PULL,
            bcls::MISC_CFG::TMS_PULL,
        ] {
            ctx.collect_bel_attr_subset(
                tcid,
                bslot,
                attr,
                &[
                    enums::IOB_PULL::NONE,
                    enums::IOB_PULL::PULLUP,
                    enums::IOB_PULL::PULLDOWN,
                ],
            );
        }
        ctx.collect_bel_attr(tcid, bslot, bcls::MISC_CFG::USERCODE);
        ctx.collect_bel_attr(tcid, bslot, bcls::MISC_CFG::ICAP_WIDTH);
        let bits = xlat_bit_wide_bi(
            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::MISC_CFG::DISABLE_JTAG_TR, false),
            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::MISC_CFG::DISABLE_JTAG_TR, true),
        );
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::MISC_CFG::DISABLE_JTAG_TR, bits);
    }

    for bslot in bslots::BSCAN {
        ctx.collect_bel_attr(tcid, bslot, bcls::BSCAN::ENABLE);
    }

    for bslot in bslots::ICAP {
        let bits = xlat_bit_wide(ctx.get_diff_attr_bool(tcid, bslot, bcls::ICAP_V6::ENABLE_TR));
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::ICAP_V6::ENABLE_TR, bits);
    }
    {
        let bslot = bslots::DNA_PORT;
        let bits = xlat_bit_wide(ctx.get_diff_attr_bool(tcid, bslot, bcls::DNA_PORT::ENABLE_TR));
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DNA_PORT::ENABLE_TR, bits);
    }
    {
        let bslot = bslots::CFG_IO_ACCESS;
        let bits =
            xlat_bit_wide(ctx.get_diff_attr_bool(tcid, bslot, bcls::CFG_IO_ACCESS_V6::ENABLE_TR));
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::CFG_IO_ACCESS_V6::ENABLE_TR, bits);
    }
    {
        let bslot = bslots::DCIRESET;
        let bits = xlat_bit_wide(ctx.get_diff_attr_bool(tcid, bslot, bcls::DCIRESET::ENABLE_TR));
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCIRESET::ENABLE_TR, bits);
    }

    {
        let bslot = bslots::STARTUP;
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::STARTUP::GSR_SYNC);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::STARTUP::GTS_SYNC);
        for attr in [
            bcls::STARTUP::USER_GTS_GSR_ENABLE_TR,
            bcls::STARTUP::USRCCLK_ENABLE_TR,
            bcls::STARTUP::KEY_CLEAR_ENABLE_TR,
        ] {
            let bits = xlat_bit_wide(ctx.get_diff_attr_bool(tcid, bslot, attr));
            ctx.insert_bel_attr_bitvec(tcid, bslot, attr, bits);
        }
        let bits = xlat_bit_wide_bi(
            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::STARTUP::PROG_USR_TR, false),
            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::STARTUP::PROG_USR_TR, true),
        );
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::STARTUP::PROG_USR_TR, bits);
    }

    {
        let bslot = bslots::SYSMON;
        ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::SYSMON_V5::CONVSTCLK);
        ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::SYSMON_V5::DCLK);
        ctx.collect_bel_attr(tcid, bslot, bcls::SYSMON_V5::INIT);
        ctx.collect_bel_attr(tcid, bslot, bcls::SYSMON_V5::SYSMON_TEST_A);
        ctx.collect_bel_attr(tcid, bslot, bcls::SYSMON_V5::SYSMON_TEST_B);
        ctx.collect_bel_attr(tcid, bslot, bcls::SYSMON_V5::SYSMON_TEST_C);
        ctx.collect_bel_attr(tcid, bslot, bcls::SYSMON_V5::SYSMON_TEST_D);
        ctx.collect_bel_attr(tcid, bslot, bcls::SYSMON_V5::SYSMON_TEST_E);

        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::JTAG_SYSMON_DISABLE);
        diff.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::SYSMON_V5::SYSMON_TEST_E),
            7,
            0,
        );
        diff.assert_empty();
    }

    {
        let tcid = tcls::HCLK;
        let bslot = bslots::HCLK_DRP;
        ctx.collect_bel_attr(tcid, bslot, bcls::HCLK_DRP_V6::DRP_MASK_SYSMON);
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
        diff.apply_enum_diff_legacy(
            ctx.item_legacy(tile, bel, "POST_CRC_CLK"),
            "INTERNAL",
            "CFG_CLK",
        );
        diffs.push((val, diff));
    }
    ctx.insert_legacy(
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
    ctx.insert_legacy(
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
    ctx.insert_legacy(tile, "MISC", "GLUTMASK", item);
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
    diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "VGG_TEST"), true, false);
    diff.assert_empty();
    let mut diff = ctx.get_diff_legacy(tile, bel, "MODE_PIN_TEST", "TEST1");
    diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "EN_VTEST"), true, false);
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
    ctx.insert_legacy(tile, bel, "FUSE_SHADOW", xlat_bit_legacy(diff));

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
