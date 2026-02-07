use prjcombine_re_collector::{
    diff::{OcdMode, xlat_bit},
    legacy::{concat_bitvec_legacy, xlat_bit_legacy},
};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_spartan6::defs::{bcls, bslots, enums, tcls};
use prjcombine_types::{
    bits,
    bsdata::{TileBit, TileItem, TileItemKind},
};
use prjcombine_xilinx_bitstream::Reg;

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
    spartan6::specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Spartan6(edev) = backend.edev else {
        unreachable!()
    };
    for (tcid, n) in [
        (tcls::CNR_SW, "BL"),
        (tcls::CNR_NW, "TL"),
        (tcls::CNR_SE, "BR"),
        (tcls::CNR_NE, "TR"),
    ] {
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        for (bslot, vh) in [(bslots::MISR_CNR_V, 'V'), (bslots::MISR_CNR_H, 'H')] {
            let mut bctx = ctx.bel(bslot);
            bctx.build()
                .global("ENABLEMISR", "Y")
                .global("MISRRESET", "N")
                .test_bel_attr_bits(bcls::MISR::ENABLE)
                .global(format!("MISR_{n}{vh}_EN"), "Y")
                .commit();
            bctx.build()
                .global("ENABLEMISR", "Y")
                .global("MISRRESET", "Y")
                .test_bel_attr_bits(bcls::MISR::RESET)
                .global(format!("MISR_{n}{vh}_EN"), "Y")
                .commit();
        }
    }
    for (tcid, bslot) in [
        (tcls::CNR_NW, bslots::OCT_CAL[0]),
        (tcls::CNR_SE, bslots::OCT_CAL[1]),
        (tcls::CNR_SW, bslots::OCT_CAL[2]),
        (tcls::CNR_SW, bslots::OCT_CAL[3]),
        (tcls::CNR_NW, bslots::OCT_CAL[4]),
        (tcls::CNR_NE, bslots::OCT_CAL[5]),
    ] {
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        let mut bctx = ctx.bel(bslot);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("OCT_CALIBRATE")
            .commit();
        bctx.mode("OCT_CALIBRATE")
            .test_bel_attr(bcls::OCT_CAL::ACCESS_MODE);
        for (val, vname) in [
            (enums::OCT_CAL_VREF_VALUE::_0P25, "0.25"),
            (enums::OCT_CAL_VREF_VALUE::_0P5, "0.5"),
            (enums::OCT_CAL_VREF_VALUE::_0P75, "0.75"),
        ] {
            bctx.mode("OCT_CALIBRATE")
                .null_bits()
                .test_bel_attr_val(bcls::OCT_CAL::VREF_VALUE, val)
                .attr("VREF_VALUE", vname)
                .commit();
        }
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::CNR_SW);
        let mut bctx = ctx.bel(bslots::MISC_SW);
        bctx.build()
            .test_bel_attr_bits(bcls::MISC_SW::LEAKER_SLOPE_OPTIONS)
            .multi_global("LEAKERSLOPEOPTIONS", MultiValue::Dec(0), 4);
        bctx.build()
            .test_bel_attr_bits(bcls::MISC_SW::LEAKER_GAIN_OPTIONS)
            .multi_global("LEAKERGAINOPTIONS", MultiValue::Dec(0), 4);
        bctx.build()
            .test_bel_attr_bits(bcls::MISC_SW::VGG_SLOPE_OPTIONS)
            .multi_global("VGGSLOPEOPTIONS", MultiValue::Dec(0), 4);
        bctx.build()
            .test_bel_attr_bits(bcls::MISC_SW::VBG_SLOPE_OPTIONS)
            .multi_global("VBGSLOPEOPTIONS", MultiValue::Dec(0), 4);
        bctx.build()
            .test_bel_attr_bits(bcls::MISC_SW::VGG_TEST_OPTIONS)
            .multi_global("VGGTESTOPTIONS", MultiValue::Dec(0), 3);
        bctx.build()
            .test_bel_attr_bits(bcls::MISC_SW::VGG_COMP_OPTION)
            .multi_global("VGGCOMPOPTION", MultiValue::Dec(0), 1);
        for (val, vname) in [
            (enums::IOB_PULL::NONE, "PULLNONE"),
            (enums::IOB_PULL::PULLUP, "PULLUP"),
        ] {
            bctx.build()
                .test_bel_attr_val(bcls::MISC_SW::PROG_PULL, val)
                .global("PROGPIN", vname)
                .commit();
        }
        for (val, vname) in [
            (enums::IOB_PULL::NONE, "PULLNONE"),
            (enums::IOB_PULL::PULLUP, "PULLUP"),
            (enums::IOB_PULL::PULLDOWN, "PULLDOWN"),
        ] {
            bctx.build()
                .test_bel_attr_val(bcls::MISC_SW::MISO2_PULL, val)
                .global("MISO2PIN", vname)
                .commit();
        }
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::CNR_NW);
        let mut bctx = ctx.bel(bslots::MISC_NW);
        for (attr, opt) in [
            (bcls::MISC_NW::M2_PULL, "M2PIN"),
            (bcls::MISC_NW::SELECTHS_PULL, "SELECTHSPIN"),
        ] {
            for (val, vname) in [
                (enums::IOB_PULL::NONE, "PULLNONE"),
                (enums::IOB_PULL::PULLUP, "PULLUP"),
                (enums::IOB_PULL::PULLDOWN, "PULLDOWN"),
            ] {
                bctx.build()
                    .test_bel_attr_val(attr, val)
                    .global(opt, vname)
                    .commit();
            }
        }
        let mut bctx = ctx.bel(bslots::DNA_PORT);
        bctx.build()
            .test_bel_attr_bits(bcls::DNA_PORT::ENABLE)
            .mode("DNA_PORT")
            .commit();
        for (val, vname) in [
            (enums::DNA_PORT_OPTIONS::READ, "READ"),
            (enums::DNA_PORT_OPTIONS::PROGRAM, "PROGRAM"),
            (enums::DNA_PORT_OPTIONS::ANALOG_READ, "ANALOGREAD"),
        ] {
            bctx.build()
                .test_bel_attr_val(bcls::DNA_PORT::OPTIONS, val)
                .global("DNAOPTIONS", vname)
                .commit();
        }
        let mut bctx = ctx.bel(bslots::PMV);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("PMV")
            .commit();
        bctx.mode("PMV")
            .test_bel_attr_bits(bcls::PMV::PSLEW)
            .multi_attr("PSLEW", MultiValue::Dec(0), 4);
        bctx.mode("PMV")
            .test_bel_attr_bits(bcls::PMV::NSLEW)
            .multi_attr("NSLEW", MultiValue::Dec(0), 4);
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::CNR_SE);
        let mut bctx = ctx.bel(bslots::MISC_SE);
        for (attr, opt) in [
            (bcls::MISC_SE::CCLK2_PULL, "CCLK2PIN"),
            (bcls::MISC_SE::MOSI2_PULL, "MOSI2PIN"),
            (bcls::MISC_SE::CMP_CS_PULL, "SS_BPIN"),
            (bcls::MISC_SE::DONE_PULL, "DONEPIN"),
        ] {
            for (val, vname) in [
                (enums::IOB_PULL::NONE, "PULLNONE"),
                (enums::IOB_PULL::PULLUP, "PULLUP"),
                (enums::IOB_PULL::PULLDOWN, "PULLDOWN"),
            ] {
                if attr == bcls::MISC_SE::DONE_PULL && val == enums::IOB_PULL::PULLDOWN {
                    continue;
                }
                bctx.build()
                    .test_bel_attr_val(attr, val)
                    .global(opt, vname)
                    .commit();
            }
        }
        let mut bctx = ctx.bel(bslots::ICAP);
        bctx.build()
            .test_bel_attr_bits(bcls::ICAP::ENABLE)
            .mode("ICAP")
            .commit();
        let mut bctx = ctx.bel(bslots::SPI_ACCESS);
        bctx.build()
            .test_bel_attr_bits(bcls::SPI_ACCESS::ENABLE)
            .mode("SPI_ACCESS")
            .commit();

        let mut bctx = ctx.bel(bslots::SUSPEND_SYNC);
        bctx.build()
            .test_bel_attr_bits(bcls::SUSPEND_SYNC::ENABLE)
            .mode("SUSPEND_SYNC")
            .commit();
        let mut bctx = ctx.bel(bslots::POST_CRC_INTERNAL);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("POST_CRC_INTERNAL")
            .commit();
        let mut bctx = ctx.bel(bslots::STARTUP);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("STARTUP")
            .commit();
        bctx.mode("STARTUP").test_global_attr_bool_rename(
            "GTS_SYNC",
            bcls::STARTUP::GTS_SYNC,
            "NO",
            "YES",
        );
        bctx.mode("STARTUP").test_global_attr_bool_rename(
            "GSR_SYNC",
            bcls::STARTUP::GSR_SYNC,
            "NO",
            "YES",
        );
        bctx.mode("STARTUP")
            .no_pin("GSR")
            .test_bel_attr_bits(bcls::STARTUP::USER_GTS_GSR_ENABLE)
            .pin("GTS")
            .commit();
        bctx.mode("STARTUP")
            .no_pin("GTS")
            .test_bel_attr_bits(bcls::STARTUP::USER_GTS_GSR_ENABLE)
            .pin("GSR")
            .commit();
        bctx.mode("STARTUP")
            .test_bel_attr_bits(bcls::STARTUP::CFGCLK_ENABLE)
            .pin("CFGCLK")
            .commit();
        bctx.mode("STARTUP")
            .test_bel_attr_bits(bcls::STARTUP::CFGMCLK_ENABLE)
            .pin("CFGMCLK")
            .commit();
        bctx.mode("STARTUP")
            .test_bel_attr_bits(bcls::STARTUP::KEYCLEARB_ENABLE)
            .pin("KEYCLEARB")
            .commit();
        for val in ["CCLK", "USERCLK", "JTAGCLK"] {
            bctx.mode("STARTUP")
                .pin("CLK")
                .extra_tile_reg_attr_legacy(Reg::Cor1, "REG.COR1", "STARTUP", "STARTUPCLK", val)
                .null_bits()
                .test_manual_legacy("STARTUPCLK", val)
                .global("STARTUPCLK", val)
                .commit();
        }

        let mut bctx = ctx.bel(bslots::SLAVE_SPI);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("SLAVE_SPI")
            .commit();
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::CNR_NE);
        let mut bctx = ctx.bel(bslots::MISC_NE);
        for (attr, opt) in [
            (bcls::MISC_NE::TCK_PULL, "TCKPIN"),
            (bcls::MISC_NE::TMS_PULL, "TMSPIN"),
            (bcls::MISC_NE::TDI_PULL, "TDIPIN"),
            (bcls::MISC_NE::TDO_PULL, "TDOPIN"),
            (bcls::MISC_NE::CSO2_PULL, "CSO2PIN"),
        ] {
            for (val, vname) in [
                (enums::IOB_PULL::NONE, "PULLNONE"),
                (enums::IOB_PULL::PULLUP, "PULLUP"),
                (enums::IOB_PULL::PULLDOWN, "PULLDOWN"),
            ] {
                bctx.build()
                    .test_bel_attr_val(attr, val)
                    .global(opt, vname)
                    .commit();
            }
        }
        for (val, vname) in [(false, "0"), (true, "1")] {
            bctx.build()
                .bel_mode(bslots::BSCAN[0], "BSCAN")
                .test_bel_attr_bits_bi(bcls::MISC_NE::JTAG_TEST, val)
                .bel_attr(bslots::BSCAN[0], "JTAG_TEST", vname)
                .commit();
        }
        bctx.build()
            .test_bel_attr_bits(bcls::MISC_NE::USERCODE)
            .multi_global("USERID", MultiValue::HexPrefix, 32);

        for i in 0..4 {
            let mut bctx = ctx.bel(bslots::BSCAN[i]);
            bctx.build()
                .test_bel_attr_bits(bcls::BSCAN::ENABLE)
                .mode("BSCAN")
                .commit();
            if i != 0 {
                for val in ["0", "1"] {
                    bctx.build()
                        .null_bits()
                        .mode("BSCAN")
                        .test_bel_special(specials::BSCAN_JTAG_TEST_DUMMY)
                        .attr("JTAG_TEST", val)
                        .commit();
                }
            }
        }
    }

    let mut ctx = FuzzCtx::new_null(session, backend);
    {
        let reg = Reg::Cor1;
        let reg_name = "REG.COR1";
        // "STARTUP",
        for val in ["NO", "YES"] {
            ctx.test_reg_legacy(reg, reg_name, "STARTUP", "DRIVE_DONE", val)
                .global("DRIVEDONE", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, "STARTUP", "DONE_PIPE", val)
                .global("DONEPIPE", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, "STARTUP", "DRIVE_AWAKE", val)
                .global("DRIVE_AWAKE", val)
                .commit();
        }
        for val in ["DISABLE", "ENABLE"] {
            ctx.test_reg_legacy(reg, reg_name, "STARTUP", "CRC", val)
                .global("CRC", val)
                .commit();
        }
        ctx.test_reg_legacy(reg, reg_name, "STARTUP", "VRDSEL", "")
            .multi_global("VRDSEL", MultiValue::Bin, 3);
        for val in ["0", "1"] {
            for opt in ["SEND_VGG0", "SEND_VGG1", "SEND_VGG2", "SEND_VGG3"] {
                ctx.test_reg_legacy(reg, reg_name, "MISC", opt, val)
                    .global(opt, val)
                    .commit();
            }
        }
        for val in ["NO", "YES"] {
            for opt in ["VGG_SENDMAX", "VGG_ENABLE_OFFCHIP"] {
                ctx.test_reg_legacy(reg, reg_name, "MISC", opt, val)
                    .global(opt, val)
                    .commit();
            }
        }
    }

    {
        let reg = Reg::Cor2;
        let reg_name = "REG.COR2";
        for val in ["1", "2", "3", "4", "5", "6", "DONE", "KEEP"] {
            ctx.test_reg_legacy(reg, reg_name, "STARTUP", "GWE_CYCLE", val)
                .global("GWE_CYCLE", val)
                .commit();
            ctx.build()
                .global("LCK_CYCLE", "NOWAIT")
                .test_reg(reg, reg_name, "STARTUP", "GTS_CYCLE", val)
                .global("GTS_CYCLE", val)
                .commit();
        }
        for val in ["1", "2", "3", "4", "5", "6"] {
            ctx.test_reg_legacy(reg, reg_name, "STARTUP", "DONE_CYCLE", val)
                .global("DONE_CYCLE", val)
                .commit();
        }
        for val in ["1", "2", "3", "4", "5", "6", "NOWAIT"] {
            ctx.build()
                .global("GTS_CYCLE", "1")
                .test_reg(reg, reg_name, "STARTUP", "LCK_CYCLE", val)
                .global("LCK_CYCLE", val)
                .commit();
        }
        for val in ["NO", "YES"] {
            ctx.test_reg_legacy(reg, reg_name, "STARTUP", "BPI_DIV8", val)
                .global("BPI_DIV8", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, "STARTUP", "BPI_DIV16", val)
                .global("BPI_DIV16", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, "STARTUP", "RESET_ON_ERR", val)
                .global("RESET_ON_ERR", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, "STARTUP", "DISABLE_VRD_REG", val)
                .global("DISABLE_VRD_REG", val)
                .commit();
        }
    }

    {
        let reg = Reg::Ctl0;
        let reg_name = "REG.CTL";
        for val in ["NO", "YES"] {
            ctx.test_reg_legacy(reg, reg_name, "MISC", "GTS_USR_B", val)
                .global("GTS_USR_B", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, "MISC", "MULTIBOOT_ENABLE", val)
                .global("MULTIBOOTMODE", val)
                .commit();
            if edev.chip.has_encrypt {
                ctx.build()
                    .global_mutex("BRAM", "NOPE")
                    .test_reg(reg, reg_name, "MISC", "ENCRYPT", val)
                    .global("ENCRYPT", val)
                    .commit();
            }
        }
        for val in ["EFUSE", "BBRAM"] {
            ctx.test_reg_legacy(reg, reg_name, "MISC", "ENCRYPT_KEY_SELECT", val)
                .global("ENCRYPTKEYSELECT", val)
                .commit();
        }
        for val in ["DISABLE", "ENABLE"] {
            ctx.test_reg_legacy(reg, reg_name, "MISC", "POST_CRC_INIT_FLAG", val)
                .global("POST_CRC_INIT_FLAG", val)
                .commit();
        }
        // persist not fuzzed — too much effort
        for val in ["NONE", "LEVEL1", "LEVEL2", "LEVEL3"] {
            ctx.test_reg_legacy(reg, reg_name, "MISC", "SECURITY", val)
                .global("SECURITY", val)
                .commit();
        }
    }

    {
        let reg = Reg::CclkFrequency;
        let reg_name = "REG.CCLK_FREQ";
        for val in ["2", "1", "4", "6", "10", "12", "16", "22", "26"] {
            ctx.build()
                .global("EXTMASTERCCLK_EN", "NO")
                .test_reg(reg, reg_name, "STARTUP", "CONFIG_RATE", val)
                .global("CONFIGRATE", val)
                .commit();
        }
        for val in ["1", "2", "4", "8", "16", "32", "64", "128", "256", "512"] {
            ctx.build()
                .global("EXTMASTERCCLK_EN", "YES")
                .test_reg(reg, reg_name, "STARTUP", "EXTMASTERCCLK_DIVIDE", val)
                .global("EXTMASTERCCLK_DIVIDE", val)
                .commit();
        }
        for val in ["NO", "YES"] {
            ctx.test_reg_legacy(reg, reg_name, "STARTUP", "EXTMASTERCCLK_EN", val)
                .global("EXTMASTERCCLK_EN", val)
                .commit();
        }
        for val in ["0", "1", "2", "3"] {
            ctx.test_reg_legacy(reg, reg_name, "STARTUP", "CCLK_DLY", val)
                .global("CCLK_DLY", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, "STARTUP", "CCLK_SEP", val)
                .global("CCLK_SEP", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, "STARTUP", "CLK_SWITCH_OPT", val)
                .global("CLK_SWITCH_OPT", val)
                .commit();
        }
    }

    {
        let reg = Reg::HcOpt;
        let reg_name = "REG.HC_OPT";
        for val in ["NO", "YES"] {
            ctx.test_reg_legacy(reg, reg_name, "MISC", "BRAM_SKIP", val)
                .global("BRAM_SKIP", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, "MISC", "TWO_ROUND", val)
                .global("TWO_ROUND", val)
                .commit();
        }
        for i in 1..16 {
            let val = format!("{i}");
            ctx.test_reg_legacy(reg, reg_name, "MISC", "HC_CYCLE", &val)
                .global("HC_CYCLE", &val)
                .commit();
        }
    }

    {
        let reg = Reg::Powerdown;
        let reg_name = "REG.POWERDOWN";
        for val in ["STARTUPCLK", "INTERNALCLK"] {
            ctx.test_reg_legacy(reg, reg_name, "MISC", "SW_CLK", val)
                .global("SW_CLK", val)
                .commit();
        }
        for val in ["NO", "YES"] {
            ctx.test_reg_legacy(reg, reg_name, "MISC", "EN_SUSPEND", val)
                .global("EN_SUSPEND", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, "MISC", "SUSPEND_FILTER", val)
                .global("SUSPEND_FILTER", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, "MISC", "EN_SW_GSR", val)
                .global("EN_SW_GSR", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, "MISC", "MULTIPIN_WAKEUP", val)
                .global("MULTIPIN_WAKEUP", val)
                .commit();
        }
        for i in 1..8 {
            let val = format!("{i}");
            ctx.test_reg_legacy(reg, reg_name, "MISC", "WAKE_DELAY1", &val)
                .global("WAKE_DELAY1", val)
                .commit();
        }
        for i in 1..32 {
            let val = format!("{i}");
            ctx.test_reg_legacy(reg, reg_name, "MISC", "WAKE_DELAY2", &val)
                .global("WAKE_DELAY2", val)
                .commit();
        }
    }

    {
        let reg = Reg::PuGwe;
        let reg_name = "REG.PU_GWE";
        for val in ["1", "2", "4", "8", "16", "32", "64", "128", "256", "512"] {
            ctx.test_reg_legacy(reg, reg_name, "MISC", "SW_GWE_CYCLE", val)
                .global("SW_GWE_CYCLE", val)
                .commit();
        }
    }

    {
        let reg = Reg::PuGts;
        let reg_name = "REG.PU_GTS";
        for val in ["1", "2", "4", "8", "16", "32", "64", "128", "256", "512"] {
            ctx.test_reg_legacy(reg, reg_name, "MISC", "SW_GTS_CYCLE", val)
                .global("SW_GTS_CYCLE", val)
                .commit();
        }
    }

    {
        let reg = Reg::EyeMask;
        let reg_name = "REG.EYE_MASK";
        ctx.test_reg_legacy(reg, reg_name, "MISC", "WAKEUP_MASK", "")
            .multi_global("WAKEUP_MASK", MultiValue::HexPrefix, 8);
    }

    {
        let reg = Reg::Mode;
        let reg_name = "REG.MODE";
        for val in ["NO", "YES"] {
            ctx.test_reg_legacy(reg, reg_name, "MISC", "NEXT_CONFIG_NEW_MODE", val)
                .global("NEXT_CONFIG_NEW_MODE", val)
                .commit();
        }
        ctx.test_reg_legacy(reg, reg_name, "MISC", "NEXT_CONFIG_BOOT_MODE", "")
            .multi_global("NEXT_CONFIG_BOOT_MODE", MultiValue::Bin, 3);
    }

    {
        let reg = Reg::SeuOpt;
        let reg_name = "REG.SEU_OPT";
        for val in ["NO", "YES"] {
            ctx.test_reg_legacy(reg, reg_name, "MISC", "POST_CRC_KEEP", val)
                .global("POST_CRC_KEEP", val)
                .commit();
        }
        for val in ["0", "1"] {
            ctx.test_reg_legacy(reg, reg_name, "MISC", "POST_CRC_SEL", val)
                .global("POST_CRC_SEL", val)
                .commit();
            ctx.build()
                .global("POST_CRC_SEL", "0")
                .test_reg(reg, reg_name, "MISC", "POST_CRC_ONESHOT", val)
                .global("POST_CRC_ONESHOT", val)
                .commit();
        }
        for val in [
            "1", "2", "4", "6", "10", "12", "16", "22", "26", "33", "40", "50", "66",
        ] {
            ctx.test_reg_legacy(reg, reg_name, "MISC", "POST_CRC_FREQ", val)
                .global("POST_CRC_FREQ", val)
                .commit();
        }
    }

    {
        let reg = Reg::Testmode;
        let reg_name = "REG.TESTMODE";
        for val in ["NO", "YES"] {
            ctx.test_reg_legacy(reg, reg_name, "MISC", "TESTMODE_EN", val)
                .global("TESTMODE_EN", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, "MISC", "ICAP_BYPASS", val)
                .global("ICAP_BYPASS", val)
                .commit();
            ctx.test_reg_legacy(reg, reg_name, "MISC", "VGG_TEST", val)
                .global("VGG_TEST", val)
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Spartan6(edev) = ctx.edev else {
        unreachable!()
    };
    for (tcid, bslot) in [
        (tcls::CNR_NW, bslots::OCT_CAL[0]),
        (tcls::CNR_SE, bslots::OCT_CAL[1]),
        (tcls::CNR_SW, bslots::OCT_CAL[2]),
        (tcls::CNR_SW, bslots::OCT_CAL[3]),
        (tcls::CNR_NW, bslots::OCT_CAL[4]),
        (tcls::CNR_NE, bslots::OCT_CAL[5]),
    ] {
        ctx.collect_bel_attr(tcid, bslot, bcls::OCT_CAL::ACCESS_MODE);
    }

    for tcid in [tcls::CNR_SW, tcls::CNR_NW, tcls::CNR_SE, tcls::CNR_NE] {
        for bslot in [bslots::MISR_CNR_H, bslots::MISR_CNR_V] {
            ctx.collect_bel_attr(tcid, bslot, bcls::MISR::ENABLE);
            let mut diff = ctx.get_diff_attr_bool(tcid, bslot, bcls::MISR::RESET);
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::MISR::ENABLE),
                true,
                false,
            );
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::MISR::RESET, xlat_bit(diff));
        }
    }

    {
        let tcid = tcls::CNR_SW;
        let bslot = bslots::MISC_SW;
        ctx.collect_bel_attr(tcid, bslot, bcls::MISC_SW::LEAKER_SLOPE_OPTIONS);
        ctx.collect_bel_attr(tcid, bslot, bcls::MISC_SW::LEAKER_GAIN_OPTIONS);
        ctx.collect_bel_attr(tcid, bslot, bcls::MISC_SW::VGG_SLOPE_OPTIONS);
        ctx.collect_bel_attr(tcid, bslot, bcls::MISC_SW::VBG_SLOPE_OPTIONS);
        ctx.collect_bel_attr(tcid, bslot, bcls::MISC_SW::VGG_TEST_OPTIONS);
        ctx.collect_bel_attr(tcid, bslot, bcls::MISC_SW::VGG_COMP_OPTION);
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            bcls::MISC_SW::PROG_PULL,
            &[enums::IOB_PULL::PULLUP, enums::IOB_PULL::NONE],
        );
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            bcls::MISC_SW::MISO2_PULL,
            &[
                enums::IOB_PULL::PULLUP,
                enums::IOB_PULL::NONE,
                enums::IOB_PULL::PULLDOWN,
            ],
        );
    }

    {
        let tcid = tcls::CNR_NW;
        let bslot = bslots::MISC_NW;
        for attr in [bcls::MISC_NW::M2_PULL, bcls::MISC_NW::SELECTHS_PULL] {
            ctx.collect_bel_attr_subset(
                tcid,
                bslot,
                attr,
                &[
                    enums::IOB_PULL::PULLUP,
                    enums::IOB_PULL::NONE,
                    enums::IOB_PULL::PULLDOWN,
                ],
            );
        }
    }
    {
        let tcid = tcls::CNR_NW;
        let bslot = bslots::DNA_PORT;
        ctx.collect_bel_attr(tcid, bslot, bcls::DNA_PORT::ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::DNA_PORT::OPTIONS);
    }
    {
        let tcid = tcls::CNR_NW;
        let bslot = bslots::PMV;
        ctx.collect_bel_attr(tcid, bslot, bcls::PMV::PSLEW);
        ctx.collect_bel_attr(tcid, bslot, bcls::PMV::NSLEW);
    }

    {
        let tcid = tcls::CNR_SE;
        let bslot = bslots::MISC_SE;
        for attr in [
            bcls::MISC_SE::CCLK2_PULL,
            bcls::MISC_SE::MOSI2_PULL,
            bcls::MISC_SE::CMP_CS_PULL,
        ] {
            ctx.collect_bel_attr_subset(
                tcid,
                bslot,
                attr,
                &[
                    enums::IOB_PULL::PULLUP,
                    enums::IOB_PULL::NONE,
                    enums::IOB_PULL::PULLDOWN,
                ],
            );
        }
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            bcls::MISC_SE::DONE_PULL,
            &[enums::IOB_PULL::PULLUP, enums::IOB_PULL::NONE],
        );
        ctx.collect_bel_attr(tcid, bslots::ICAP, bcls::ICAP::ENABLE);
        ctx.collect_bel_attr(tcid, bslots::SUSPEND_SYNC, bcls::SUSPEND_SYNC::ENABLE);
        ctx.collect_bel_attr(tcid, bslots::SPI_ACCESS, bcls::SPI_ACCESS::ENABLE);
    }
    {
        let tcid = tcls::CNR_SE;
        let bslot = bslots::STARTUP;
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::STARTUP::GTS_SYNC);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::STARTUP::GSR_SYNC);
        ctx.collect_bel_attr(tcid, bslot, bcls::STARTUP::CFGCLK_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::STARTUP::CFGMCLK_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::STARTUP::KEYCLEARB_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::STARTUP::USER_GTS_GSR_ENABLE);
    }

    {
        let tcid = tcls::CNR_NE;
        let bslot = bslots::MISC_NE;
        for attr in [
            bcls::MISC_NE::TCK_PULL,
            bcls::MISC_NE::TMS_PULL,
            bcls::MISC_NE::TDI_PULL,
            bcls::MISC_NE::TDO_PULL,
            bcls::MISC_NE::CSO2_PULL,
        ] {
            ctx.collect_bel_attr_subset(
                tcid,
                bslot,
                attr,
                &[
                    enums::IOB_PULL::PULLUP,
                    enums::IOB_PULL::NONE,
                    enums::IOB_PULL::PULLDOWN,
                ],
            );
        }
        ctx.collect_bel_attr(tcid, bslot, bcls::MISC_NE::USERCODE);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::MISC_NE::JTAG_TEST);
        for bslot in bslots::BSCAN {
            ctx.collect_bel_attr(tcid, bslot, bcls::BSCAN::ENABLE);
        }
    }

    {
        let tile = "REG.COR1";
        let bel = "STARTUP";
        ctx.collect_enum_legacy(tile, bel, "STARTUPCLK", &["CCLK", "USERCLK", "JTAGCLK"]);
        ctx.collect_bit_bi_legacy(tile, bel, "DRIVE_DONE", "NO", "YES");
        ctx.collect_bit_bi_legacy(tile, bel, "DONE_PIPE", "NO", "YES");
        ctx.collect_bit_bi_legacy(tile, bel, "DRIVE_AWAKE", "NO", "YES");
        ctx.collect_bit_bi_legacy(tile, bel, "CRC", "DISABLE", "ENABLE");
        ctx.collect_bitvec_legacy(tile, bel, "VRDSEL", "");
        let bel = "MISC";
        ctx.collect_bit_bi_legacy(tile, bel, "VGG_SENDMAX", "NO", "YES");
        ctx.collect_bit_bi_legacy(tile, bel, "VGG_ENABLE_OFFCHIP", "NO", "YES");
        let item0 = ctx.extract_bit_bi_legacy(tile, bel, "SEND_VGG0", "0", "1");
        let item1 = ctx.extract_bit_bi_legacy(tile, bel, "SEND_VGG1", "0", "1");
        let item2 = ctx.extract_bit_bi_legacy(tile, bel, "SEND_VGG2", "0", "1");
        let item3 = ctx.extract_bit_bi_legacy(tile, bel, "SEND_VGG3", "0", "1");
        let item = concat_bitvec_legacy([item0, item1, item2, item3]);
        ctx.insert_legacy(tile, bel, "SEND_VGG", item);

        let tile = "REG.COR2";
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
        ctx.collect_enum_legacy(tile, bel, "DONE_CYCLE", &["1", "2", "3", "4", "5", "6"]);
        ctx.collect_enum_legacy(
            tile,
            bel,
            "LCK_CYCLE",
            &["1", "2", "3", "4", "5", "6", "NOWAIT"],
        );
        ctx.collect_bit_bi_legacy(tile, bel, "BPI_DIV8", "NO", "YES");
        ctx.collect_bit_bi_legacy(tile, bel, "BPI_DIV16", "NO", "YES");
        ctx.collect_bit_bi_legacy(tile, bel, "RESET_ON_ERR", "NO", "YES");
        ctx.collect_bit_bi_legacy(tile, bel, "DISABLE_VRD_REG", "NO", "YES");
    }

    {
        let tile = "REG.CTL";
        let bel = "MISC";
        ctx.collect_bit_bi_legacy(tile, bel, "GTS_USR_B", "NO", "YES");
        ctx.collect_enum_legacy(tile, bel, "ENCRYPT_KEY_SELECT", &["BBRAM", "EFUSE"]);
        ctx.collect_bit_bi_legacy(tile, bel, "POST_CRC_INIT_FLAG", "DISABLE", "ENABLE");
        ctx.collect_bit_bi_legacy(tile, bel, "MULTIBOOT_ENABLE", "NO", "YES");
        if edev.chip.has_encrypt {
            ctx.collect_bit_bi_legacy(tile, bel, "ENCRYPT", "NO", "YES");
        }
        ctx.collect_enum_legacy(
            tile,
            bel,
            "SECURITY",
            &["NONE", "LEVEL1", "LEVEL2", "LEVEL3"],
        );
        // too much trouble to deal with in normal ways.
        ctx.insert_legacy(
            tile,
            bel,
            "PERSIST",
            TileItem::from_bit_inv(TileBit::new(0, 0, 3), false),
        );
    }

    {
        let tile = "REG.CCLK_FREQ";
        let bel = "STARTUP";
        // it's just 400 / val. boring.
        let _ = ctx.extract_enum_legacy_ocd(
            tile,
            bel,
            "CONFIG_RATE",
            &["2", "1", "4", "6", "10", "12", "16", "22", "26"],
            OcdMode::BitOrder,
        );
        let item =
            TileItem::from_bitvec_inv((0..10).map(|bit| TileBit::new(0, 0, bit)).collect(), false);
        for i in 0..10 {
            let val = 1 << i;
            let mut diff = ctx.get_diff_legacy(tile, bel, "EXTMASTERCCLK_DIVIDE", val.to_string());
            diff.apply_bitvec_diff_int_legacy(&item, val, 1);
            diff.assert_empty();
        }
        ctx.get_diff_legacy(tile, bel, "EXTMASTERCCLK_EN", "NO")
            .assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "EXTMASTERCCLK_EN", "YES");
        diff.apply_bitvec_diff_int_legacy(&item, 1, 0xc8);
        ctx.insert_legacy(tile, bel, "CCLK_DIVISOR", item);
        ctx.insert_legacy(tile, bel, "EXT_CCLK_ENABLE", xlat_bit_legacy(diff));
        ctx.collect_enum_legacy_int(tile, bel, "CCLK_DLY", 0..4, 0);
        ctx.collect_enum_legacy_int(tile, bel, "CCLK_SEP", 0..4, 0);
        for val in ["0", "1", "2", "3"] {
            ctx.get_diff_legacy(tile, bel, "CLK_SWITCH_OPT", val)
                .assert_empty();
        }
    }

    {
        let tile = "REG.HC_OPT";
        let bel = "MISC";
        ctx.collect_bit_bi_legacy(tile, bel, "BRAM_SKIP", "NO", "YES");
        ctx.collect_bit_bi_legacy(tile, bel, "TWO_ROUND", "NO", "YES");
        ctx.collect_enum_legacy_int(tile, bel, "HC_CYCLE", 1..16, 0);
        ctx.insert_legacy(
            tile,
            bel,
            "INIT_SKIP",
            TileItem::from_bit_inv(TileBit::new(0, 0, 6), false),
        );
    }

    {
        let tile = "REG.POWERDOWN";
        let bel = "MISC";
        ctx.collect_enum_legacy(tile, bel, "SW_CLK", &["STARTUPCLK", "INTERNALCLK"]);
        ctx.collect_bit_bi_legacy(tile, bel, "EN_SUSPEND", "NO", "YES");
        ctx.collect_bit_bi_legacy(tile, bel, "EN_SW_GSR", "NO", "YES");
        ctx.collect_bit_bi_legacy(tile, bel, "MULTIPIN_WAKEUP", "NO", "YES");
        ctx.collect_bit_bi_legacy(tile, bel, "SUSPEND_FILTER", "NO", "YES");
        ctx.collect_enum_legacy_int(tile, bel, "WAKE_DELAY1", 1..8, 0);
        ctx.collect_enum_legacy_int(tile, bel, "WAKE_DELAY2", 1..32, 0);
    }

    {
        let tile = "REG.PU_GWE";
        let bel = "MISC";
        let item =
            TileItem::from_bitvec_inv((0..10).map(|bit| TileBit::new(0, 0, bit)).collect(), false);
        for i in 0..10 {
            let val = 1 << i;
            let mut diff = ctx.get_diff_legacy(tile, bel, "SW_GWE_CYCLE", val.to_string());
            diff.apply_bitvec_diff_int_legacy(&item, val, 5);
            diff.assert_empty();
        }
        ctx.insert_legacy(tile, bel, "SW_GWE_CYCLE", item);
    }
    {
        let tile = "REG.PU_GTS";
        let bel = "MISC";
        let item =
            TileItem::from_bitvec_inv((0..10).map(|bit| TileBit::new(0, 0, bit)).collect(), false);
        for i in 0..10 {
            let val = 1 << i;
            let mut diff = ctx.get_diff_legacy(tile, bel, "SW_GTS_CYCLE", val.to_string());
            diff.apply_bitvec_diff_int_legacy(&item, val, 4);
            diff.assert_empty();
        }
        ctx.insert_legacy(tile, bel, "SW_GTS_CYCLE", item);
    }

    {
        let tile = "REG.EYE_MASK";
        let bel = "MISC";
        ctx.collect_bitvec_legacy(tile, bel, "WAKEUP_MASK", "");
    }

    {
        let tile = "REG.MODE";
        let bel = "MISC";
        ctx.collect_bit_bi_legacy(tile, bel, "NEXT_CONFIG_NEW_MODE", "NO", "YES");
        ctx.insert_legacy(
            tile,
            bel,
            "SPI_BUSWIDTH",
            TileItem {
                bits: vec![TileBit::new(0, 0, 11), TileBit::new(0, 0, 12)],
                kind: TileItemKind::Enum {
                    values: [
                        ("1".to_string(), bits![0, 0]),
                        ("2".to_string(), bits![1, 0]),
                        ("4".to_string(), bits![0, 1]),
                    ]
                    .into_iter()
                    .collect(),
                },
            },
        );
        ctx.collect_bitvec_legacy(tile, bel, "NEXT_CONFIG_BOOT_MODE", "");
    }

    // these have annoying requirements to fuzz.
    ctx.insert_legacy(
        "REG.GENERAL12",
        "MISC",
        "NEXT_CONFIG_ADDR",
        TileItem::from_bitvec_inv(
            (0..16)
                .map(|bit| TileBit::new(0, 0, bit))
                .chain((0..16).map(|bit| TileBit::new(1, 0, bit)))
                .collect(),
            false,
        ),
    );
    ctx.insert_legacy(
        "REG.GENERAL34",
        "MISC",
        "GOLDEN_CONFIG_ADDR",
        TileItem::from_bitvec_inv(
            (0..16)
                .map(|bit| TileBit::new(0, 0, bit))
                .chain((0..16).map(|bit| TileBit::new(1, 0, bit)))
                .collect(),
            false,
        ),
    );
    ctx.insert_legacy(
        "REG.GENERAL5",
        "MISC",
        "FAILSAFE_USER",
        TileItem::from_bitvec_inv((0..16).map(|bit| TileBit::new(0, 0, bit)).collect(), false),
    );
    ctx.insert_legacy(
        "REG.TIMER",
        "MISC",
        "TIMER_CFG",
        TileItem::from_bitvec_inv((0..16).map(|bit| TileBit::new(0, 0, bit)).collect(), false),
    );

    {
        let tile = "REG.SEU_OPT";
        let bel = "MISC";
        ctx.collect_bit_bi_legacy(tile, bel, "POST_CRC_KEEP", "NO", "YES");
        ctx.collect_bit_bi_legacy(tile, bel, "POST_CRC_ONESHOT", "0", "1");
        ctx.collect_bit_bi_legacy(tile, bel, "POST_CRC_SEL", "0", "1");

        // too much effort to include in the automatic fuzzer
        ctx.insert_legacy(
            tile,
            bel,
            "POST_CRC_EN",
            TileItem::from_bit_inv(TileBit::new(0, 0, 0), false),
        );
        ctx.insert_legacy(
            tile,
            bel,
            "GLUTMASK",
            TileItem::from_bit_inv(TileBit::new(0, 0, 1), false),
        );

        // again, don't care.
        let _ = ctx.extract_enum_legacy_ocd(
            tile,
            bel,
            "POST_CRC_FREQ",
            &[
                "1", "2", "4", "6", "10", "12", "16", "22", "26", "33", "40", "50", "66",
            ],
            OcdMode::BitOrder,
        );
        ctx.insert_legacy(
            tile,
            bel,
            "POST_CRC_FREQ",
            TileItem::from_bitvec_inv((4..14).map(|bit| TileBit::new(0, 0, bit)).collect(), false),
        );
    }

    {
        let tile = "REG.TESTMODE";
        let bel = "MISC";
        ctx.collect_bit_bi_legacy(tile, bel, "VGG_TEST", "NO", "YES");
        ctx.collect_bit_bi_legacy(tile, bel, "ICAP_BYPASS", "NO", "YES");
        ctx.collect_bit_bi_legacy(tile, bel, "TESTMODE_EN", "NO", "YES");
    }
}
