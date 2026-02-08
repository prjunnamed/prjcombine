use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::BelAttributeEnum,
    grid::{CellCoord, DieId},
};
use prjcombine_re_collector::diff::{Diff, xlat_bit, xlat_bitvec_sparse_u32};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_spartan6::defs::{bcls, bslots, enums, tcls, tslots};
use prjcombine_types::{bits, bsdata::TileBit};

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
    let global = CellCoord::new(DieId::from_idx(0), edev.chip.col_w(), edev.chip.row_s())
        .tile(tslots::GLOBAL);
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
            .test_bel_attr_auto(bcls::OCT_CAL::ACCESS_MODE);
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
        for (val, vname) in &backend.edev.db[enums::STARTUP_CLOCK].values {
            bctx.mode("STARTUP")
                .pin("CLK")
                .extra_fixed_bel_attr_val(global, bslots::GLOBAL, bcls::GLOBAL::STARTUP_CLOCK, val)
                .null_bits()
                .test_bel_special(specials::STARTUP_CLOCK_DUMMY)
                .global("STARTUPCLK", vname)
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

    let mut ctx = FuzzCtx::new(session, backend, tcls::GLOBAL);
    let mut bctx = ctx.bel(bslots::GLOBAL);

    // COR1
    bctx.build()
        .test_global_attr_bool_rename("DRIVEDONE", bcls::GLOBAL::DRIVE_DONE, "NO", "YES");
    bctx.build()
        .test_global_attr_bool_rename("DONEPIPE", bcls::GLOBAL::DONE_PIPE, "NO", "YES");
    bctx.build().test_global_attr_bool_rename(
        "DRIVE_AWAKE",
        bcls::GLOBAL::DRIVE_AWAKE,
        "NO",
        "YES",
    );
    bctx.build()
        .test_global_attr_bool_rename("CRC", bcls::GLOBAL::CRC_ENABLE, "DISABLE", "ENABLE");
    bctx.build()
        .test_bel_attr_bits(bcls::GLOBAL::VRDSEL)
        .multi_global("VRDSEL", MultiValue::Bin, 3);
    for (val, vname) in [(false, "0"), (true, "1")] {
        for i in 0..4 {
            bctx.build()
                .test_bel_attr_bits_base_bi(bcls::GLOBAL::SEND_VGG, i, val)
                .global(format!("SEND_VGG{i}"), vname)
                .commit();
        }
    }
    bctx.build().test_global_attr_bool_rename(
        "VGG_SENDMAX",
        bcls::GLOBAL::VGG_SENDMAX,
        "NO",
        "YES",
    );
    bctx.build().test_global_attr_bool_rename(
        "VGG_ENABLE_OFFCHIP",
        bcls::GLOBAL::VGG_ENABLE_OFFCHIP,
        "NO",
        "YES",
    );

    // COR2
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
            .test_bel_attr_val(bcls::GLOBAL::GWE_CYCLE, val)
            .global("GWE_CYCLE", vname)
            .commit();
        bctx.build()
            .global("LCK_CYCLE", "NOWAIT")
            .test_bel_attr_val(bcls::GLOBAL::GTS_CYCLE, val)
            .global("GTS_CYCLE", vname)
            .commit();
        if val != enums::STARTUP_CYCLE::DONE && val != enums::STARTUP_CYCLE::KEEP {
            bctx.build()
                .test_bel_attr_val(bcls::GLOBAL::DONE_CYCLE, val)
                .global("DONE_CYCLE", vname)
                .commit();
        }
    }
    for (val, vname) in [
        (enums::STARTUP_CYCLE::_1, "1"),
        (enums::STARTUP_CYCLE::_2, "2"),
        (enums::STARTUP_CYCLE::_3, "3"),
        (enums::STARTUP_CYCLE::_4, "4"),
        (enums::STARTUP_CYCLE::_5, "5"),
        (enums::STARTUP_CYCLE::_6, "6"),
        (enums::STARTUP_CYCLE::NOWAIT, "NOWAIT"),
    ] {
        bctx.build()
            .global("GTS_CYCLE", "1")
            .test_bel_attr_val(bcls::GLOBAL::LOCK_CYCLE, val)
            .global("LCK_CYCLE", vname)
            .commit();
    }

    bctx.build()
        .test_global_attr_bool_rename("BPI_DIV8", bcls::GLOBAL::BPI_DIV8, "NO", "YES");
    bctx.build()
        .test_global_attr_bool_rename("BPI_DIV16", bcls::GLOBAL::BPI_DIV16, "NO", "YES");
    bctx.build().test_global_attr_bool_rename(
        "RESET_ON_ERR",
        bcls::GLOBAL::RESET_ON_ERR,
        "NO",
        "YES",
    );
    bctx.build().test_global_attr_bool_rename(
        "DISABLE_VRD_REG",
        bcls::GLOBAL::DISABLE_VRD_REG,
        "NO",
        "YES",
    );

    // CTL
    bctx.build()
        .test_global_attr_bool_rename("GTS_USR_B", bcls::GLOBAL::GTS_USR_B, "NO", "YES");
    bctx.build().test_global_attr_bool_rename(
        "MULTIBOOTMODE",
        bcls::GLOBAL::MULTIBOOT_ENABLE,
        "NO",
        "YES",
    );
    if edev.chip.has_encrypt {
        bctx.build()
            .global_mutex("BRAM", "NOPE")
            .test_global_attr_bool_rename("ENCRYPT", bcls::GLOBAL::ENCRYPT, "NO", "YES");
    }
    bctx.build()
        .test_global_attr_rename("ENCRYPTKEYSELECT", bcls::GLOBAL::ENCRYPT_KEY_SELECT);
    bctx.build()
        .test_global_attr_rename("SECURITY", bcls::GLOBAL::SECURITY);
    bctx.build().test_global_attr_bool_rename(
        "POST_CRC_INIT_FLAG",
        bcls::GLOBAL::POST_CRC_INIT_FLAG,
        "DISABLE",
        "ENABLE",
    );
    // persist not fuzzed — too much effort

    // CCLK_FREQ
    // CONFIGRATE too annoying.
    for i in 0..10 {
        bctx.build()
            .global("EXTMASTERCCLK_EN", "YES")
            .test_bel_attr_u32(bcls::GLOBAL::CONFIG_RATE_DIV, 1 << i)
            .global_diff("EXTMASTERCCLK_DIVIDE", "6", (1 << i).to_string())
            .commit();
    }
    bctx.build()
        .test_bel_attr_bits(bcls::GLOBAL::EXT_CCLK_ENABLE)
        .global_diff("EXTMASTERCCLK_EN", "NO", "YES")
        .global("EXTMASTERCCLK_DIVIDE", "200")
        .commit();
    for val in 0..4 {
        bctx.build()
            .test_bel_attr_bitvec_u32(bcls::GLOBAL::CCLK_DLY, val)
            .global("CCLK_DLY", val.to_string())
            .commit();
        bctx.build()
            .test_bel_attr_bitvec_u32(bcls::GLOBAL::CCLK_SEP, val)
            .global("CCLK_SEP", val.to_string())
            .commit();
        bctx.build()
            .null_bits()
            .test_bel_special(specials::CLK_SWITCH_OPT)
            .global("CLK_SWITCH_OPT", val.to_string())
            .commit();
    }

    // HC_OPT
    bctx.build()
        .test_global_attr_bool_rename("BRAM_SKIP", bcls::GLOBAL::BRAM_SKIP, "NO", "YES");
    bctx.build()
        .test_global_attr_bool_rename("TWO_ROUND", bcls::GLOBAL::TWO_ROUND, "NO", "YES");
    for val in 1..16 {
        bctx.build()
            .test_bel_attr_bitvec_u32(bcls::GLOBAL::HC_CYCLE, val)
            .global("HC_CYCLE", val.to_string())
            .commit();
    }

    // POWERDOWN
    bctx.build()
        .test_global_attr_rename("SW_CLK", bcls::GLOBAL::SW_CLK);
    bctx.build()
        .test_global_attr_bool_rename("EN_SUSPEND", bcls::GLOBAL::EN_SUSPEND, "NO", "YES");
    bctx.build().test_global_attr_bool_rename(
        "SUSPEND_FILTER",
        bcls::GLOBAL::SUSPEND_FILTER,
        "NO",
        "YES",
    );
    bctx.build()
        .test_global_attr_bool_rename("EN_SW_GSR", bcls::GLOBAL::EN_SW_GSR, "NO", "YES");
    bctx.build().test_global_attr_bool_rename(
        "MULTIPIN_WAKEUP",
        bcls::GLOBAL::MULTIPIN_WAKEUP,
        "NO",
        "YES",
    );
    for val in 1..8 {
        bctx.build()
            .test_bel_attr_bitvec_u32(bcls::GLOBAL::WAKE_DELAY1, val)
            .global("WAKE_DELAY1", val.to_string())
            .commit();
    }
    for val in 1..32 {
        bctx.build()
            .test_bel_attr_bitvec_u32(bcls::GLOBAL::WAKE_DELAY2, val)
            .global("WAKE_DELAY2", val.to_string())
            .commit();
    }

    for i in 0..10 {
        bctx.build()
            .test_bel_attr_bits_base(bcls::GLOBAL::SW_GWE_CYCLE, i)
            .global_diff("SW_GWE_CYCLE", (0x3ff ^ (1 << i)).to_string(), "1023")
            .commit();
    }
    for i in 0..10 {
        bctx.build()
            .test_bel_attr_bits_base(bcls::GLOBAL::SW_GTS_CYCLE, i)
            .global_diff("SW_GTS_CYCLE", (0x3ff ^ (1 << i)).to_string(), "1023")
            .commit();
    }
    bctx.build()
        .test_bel_attr_bits(bcls::GLOBAL::WAKEUP_MASK)
        .multi_global("WAKEUP_MASK", MultiValue::HexPrefix, 8);

    // MODE
    bctx.build().test_global_attr_bool_rename(
        "NEXT_CONFIG_NEW_MODE",
        bcls::GLOBAL::NEXT_CONFIG_NEW_MODE,
        "NO",
        "YES",
    );
    bctx.build()
        .test_bel_attr_bits(bcls::GLOBAL::NEXT_CONFIG_BOOT_MODE)
        .multi_global("NEXT_CONFIG_BOOT_MODE", MultiValue::Bin, 3);

    // SEU_OPT
    bctx.build().test_global_attr_bool_rename(
        "POST_CRC_KEEP",
        bcls::GLOBAL::POST_CRC_KEEP,
        "NO",
        "YES",
    );
    bctx.build()
        .test_global_attr_bool_rename("POST_CRC_SEL", bcls::GLOBAL::POST_CRC_SEL, "0", "1");
    bctx.build()
        .global("POST_CRC_SEL", "0")
        .test_global_attr_bool_rename("POST_CRC_ONESHOT", bcls::GLOBAL::POST_CRC_ONESHOT, "0", "1");

    // TESTMODE
    bctx.build().test_global_attr_bool_rename(
        "TESTMODE_EN",
        bcls::GLOBAL::TESTMODE_EN,
        "NO",
        "YES",
    );
    bctx.build().test_global_attr_bool_rename(
        "ICAP_BYPASS",
        bcls::GLOBAL::ICAP_BYPASS,
        "NO",
        "YES",
    );
    bctx.build()
        .test_global_attr_bool_rename("VGG_TEST", bcls::GLOBAL::VGG_TEST, "NO", "YES");
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
        let tcid = tcls::GLOBAL;
        let bslot = bslots::GLOBAL;

        // COR1
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::STARTUP_CLOCK);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::DRIVE_DONE);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::DONE_PIPE);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::DRIVE_AWAKE);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::CRC_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::VRDSEL);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::SEND_VGG);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::VGG_SENDMAX);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::VGG_ENABLE_OFFCHIP);

        // COR2
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            bcls::GLOBAL::GWE_CYCLE,
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
            bcls::GLOBAL::GTS_CYCLE,
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
            bcls::GLOBAL::DONE_CYCLE,
            &[
                enums::STARTUP_CYCLE::_1,
                enums::STARTUP_CYCLE::_2,
                enums::STARTUP_CYCLE::_3,
                enums::STARTUP_CYCLE::_4,
                enums::STARTUP_CYCLE::_5,
                enums::STARTUP_CYCLE::_6,
            ],
        );

        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            bcls::GLOBAL::LOCK_CYCLE,
            &[
                enums::STARTUP_CYCLE::_1,
                enums::STARTUP_CYCLE::_2,
                enums::STARTUP_CYCLE::_3,
                enums::STARTUP_CYCLE::_4,
                enums::STARTUP_CYCLE::_5,
                enums::STARTUP_CYCLE::_6,
                enums::STARTUP_CYCLE::NOWAIT,
            ],
        );
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::BPI_DIV8);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::BPI_DIV16);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::RESET_ON_ERR);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::DISABLE_VRD_REG);

        // CTL
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::GTS_USR_B);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::ENCRYPT_KEY_SELECT);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::POST_CRC_INIT_FLAG);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::MULTIBOOT_ENABLE);
        if edev.chip.has_encrypt {
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::ENCRYPT);
        }
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::SECURITY);
        // too much trouble to deal with in normal ways.
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::GLOBAL::PERSIST,
            TileBit::new(2, 0, 3).pos(),
        );

        // CCLK_FREQ
        let mut diffs = vec![(6, Diff::default())];
        for i in 0..10 {
            diffs.push((
                1 << i,
                ctx.get_diff_attr_u32(tcid, bslot, bcls::GLOBAL::CONFIG_RATE_DIV, 1 << i),
            ));
        }
        let mut bits = xlat_bitvec_sparse_u32(diffs);
        bits[1].inv ^= true;
        bits[2].inv ^= true;
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::GLOBAL::CONFIG_RATE_DIV, bits);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::EXT_CCLK_ENABLE);
        ctx.collect_bel_attr_sparse(tcid, bslot, bcls::GLOBAL::CCLK_DLY, 0..4);
        ctx.collect_bel_attr_sparse(tcid, bslot, bcls::GLOBAL::CCLK_SEP, 0..4);

        // HC_OPT
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::BRAM_SKIP);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::TWO_ROUND);
        ctx.collect_bel_attr_sparse(tcid, bslot, bcls::GLOBAL::HC_CYCLE, 1..16);
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::GLOBAL::INIT_SKIP,
            TileBit::new(0, 0, 6).pos(),
        );

        // POWERDOWN
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::SW_CLK);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::EN_SUSPEND);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::EN_SW_GSR);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::MULTIPIN_WAKEUP);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::SUSPEND_FILTER);
        ctx.collect_bel_attr_sparse(tcid, bslot, bcls::GLOBAL::WAKE_DELAY1, 1..8);
        ctx.collect_bel_attr_sparse(tcid, bslot, bcls::GLOBAL::WAKE_DELAY2, 1..32);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::SW_GWE_CYCLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::SW_GTS_CYCLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::WAKEUP_MASK);

        // MODE
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::NEXT_CONFIG_NEW_MODE);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::NEXT_CONFIG_BOOT_MODE);
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            bcls::GLOBAL::SPI_BUSWIDTH,
            BelAttributeEnum {
                bits: vec![TileBit::new(0, 0, 11), TileBit::new(0, 0, 12)],
                values: [
                    (enums::SPI_BUSWIDTH::_1, bits![0, 0]),
                    (enums::SPI_BUSWIDTH::_2, bits![1, 0]),
                    (enums::SPI_BUSWIDTH::_4, bits![0, 1]),
                ]
                .into_iter()
                .collect(),
            },
        );

        // these have annoying requirements to fuzz.
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::GLOBAL::NEXT_CONFIG_ADDR,
            (0..16)
                .map(|bit| TileBit::new(9, 0, bit).pos())
                .chain((0..16).map(|bit| TileBit::new(10, 0, bit).pos()))
                .collect(),
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::GLOBAL::GOLDEN_CONFIG_ADDR,
            (0..16)
                .map(|bit| TileBit::new(11, 0, bit).pos())
                .chain((0..16).map(|bit| TileBit::new(12, 0, bit).pos()))
                .collect(),
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::GLOBAL::FAILSAFE_USER,
            (0..16).map(|bit| TileBit::new(13, 0, bit).pos()).collect(),
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::GLOBAL::TIMER_CFG,
            (0..16).map(|bit| TileBit::new(16, 0, bit).pos()).collect(),
        );

        // SEU_OPT
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::POST_CRC_KEEP);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::POST_CRC_ONESHOT);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::POST_CRC_SEL);

        // too much effort to include in the automatic fuzzer
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::GLOBAL::POST_CRC_EN,
            TileBit::new(14, 0, 0).pos(),
        );
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::GLOBAL::GLUTMASK,
            TileBit::new(14, 0, 1).pos(),
        );

        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::GLOBAL::POST_CRC_FREQ_DIV,
            (4..14).map(|bit| TileBit::new(14, 0, bit).pos()).collect(),
        );

        // TESTMODE
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::VGG_TEST);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::ICAP_BYPASS);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::TESTMODE_EN);
    }
}
