use prjcombine_entity::EntityId;
use prjcombine_interconnect::{db::BelAttributeEnum, grid::DieId};
use prjcombine_re_collector::diff::{OcdMode, xlat_bit_wide, xlat_bit_wide_bi};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{bits, bsdata::TileBit};
use prjcombine_virtex4::defs::{
    bcls::{self, GLOBAL},
    bslots, enums, tslots,
    virtex6::tcls,
};

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
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };

    let global = edev.tile_cfg(DieId::from_idx(0)).tile(tslots::GLOBAL);

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
        let mut bctx = ctx.bel(bslots::BSCAN[i]);
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
        let mut bctx = ctx.bel(bslots::ICAP[1]);
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

        let mut bctx = ctx.bel(bslots::ICAP[0]);
        bctx.build()
            .bel_mode(bslots::ICAP[1], "ICAP")
            .test_bel_attr_bits(bcls::ICAP_V6::ENABLE_TR)
            .mode("ICAP")
            .commit();
        bctx.mode("ICAP")
            .bel_mode(bslots::ICAP[1], "ICAP")
            .global_mutex_here("ICAP")
            .test_bel(bslots::MISC_CFG)
            .test_bel_attr_auto(bcls::MISC_CFG::ICAP_WIDTH);
        for val in ["DISABLE", "ENABLE"] {
            bctx.mode("ICAP")
                .null_bits()
                .bel_mode(bslots::ICAP[1], "ICAP")
                .global_mutex_here("ICAP")
                .test_bel_special(specials::ICAP_AUTO_SWITCH)
                .attr("ICAP_AUTO_SWITCH", val)
                .commit();
        }
    }

    for i in 0..2 {
        let mut bctx = ctx.bel(bslots::PMV_CFG[i]);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("PMV")
            .commit();
    }

    {
        let mut bctx = ctx.bel(bslots::STARTUP);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("STARTUP")
            .commit();
        for (val, vname) in [
            (enums::STARTUP_CLOCK::CCLK, "CCLK"),
            (enums::STARTUP_CLOCK::USERCLK, "USERCLK"),
            (enums::STARTUP_CLOCK::JTAGCLK, "JTAGCLK"),
        ] {
            bctx.mode("STARTUP")
                .null_bits()
                .pin("CLK")
                .extra_fixed_bel_attr_val(global, bslots::GLOBAL, GLOBAL::STARTUP_CLOCK, val)
                .test_bel_special_val(specials::STARTUP_CLOCK, val)
                .global("STARTUPCLK", vname)
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
        let mut bctx = ctx.bel(bslots::FRAME_ECC);
        bctx.build()
            .null_bits()
            .extra_fixed_bel_attr_bits(global, bslots::GLOBAL, GLOBAL::GLUTMASK)
            .no_global("GLUTMASK_B")
            .test_bel_special(specials::PRESENT)
            .mode("FRAME_ECC")
            .commit();
        for (val, vname) in &backend.edev.db[enums::FARSRC].values {
            bctx.mode("FRAME_ECC")
                .null_bits()
                .extra_fixed_bel_attr_val(global, bslots::GLOBAL, GLOBAL::FARSRC, val)
                .test_bel_special(specials::MISC_CFG)
                .attr("FARSRC", vname)
                .commit();
        }
    }

    {
        let mut bctx = ctx.bel(bslots::DCIRESET);
        bctx.build()
            .test_bel_attr_bits(bcls::DCIRESET::ENABLE_TR)
            .mode("DCIRESET")
            .commit();
    }

    {
        let mut bctx = ctx.bel(bslots::CAPTURE);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("CAPTURE")
            .commit();
        for val in [false, true] {
            bctx.mode("CAPTURE")
                .null_bits()
                .extra_fixed_bel_attr_bits_bi(global, bslots::GLOBAL, GLOBAL::CAPTURE_ONESHOT, val)
                .test_bel_special(specials::CAPTURE_ONESHOT)
                .attr("ONESHOT", if val { "TRUE" } else { "FALSE" })
                .commit();
        }
    }

    {
        let mut bctx = ctx.bel(bslots::USR_ACCESS);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("USR_ACCESS")
            .commit();
    }

    {
        let mut bctx = ctx.bel(bslots::EFUSE_USR);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("EFUSE_USR")
            .commit();
    }

    {
        let mut bctx = ctx.bel(bslots::DNA_PORT);
        bctx.build()
            .test_bel_attr_bits(bcls::DNA_PORT::ENABLE_TR)
            .mode("DNA_PORT")
            .commit();
    }

    {
        let mut bctx = ctx.bel(bslots::CFG_IO_ACCESS);
        bctx.build()
            .test_bel_attr_bits(bcls::CFG_IO_ACCESS_V6::ENABLE_TR)
            .mode("CFG_IO_ACCESS")
            .commit();
    }

    {
        let mut bctx = ctx.bel(bslots::SYSMON);
        bctx.build()
            .null_bits()
            .extra_tile_attr_bits(
                Delta::new(0, 20, tcls::HCLK),
                bslots::HCLK_DRP[0],
                bcls::HCLK_DRP::DRP_MASK_SYSMON,
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
                .test_bel_attr_bits_base(bcls::SYSMON_V5::V5_INIT, base)
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

    let mut ctx = FuzzCtx::new(session, backend, tcls::GLOBAL);
    {
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
            bctx.build()
                .global_mutex("GLOBAL_DCI", "NO")
                .test_bel_attr_val(GLOBAL::MATCH_CYCLE, val)
                .global("MATCH_CYCLE", vname)
                .commit();
        }
        bctx.build()
            .test_global_attr_bool_rename("DRIVEDONE", GLOBAL::DRIVE_DONE, "NO", "YES");
        bctx.build()
            .test_global_attr_bool_rename("DONEPIPE", GLOBAL::DONE_PIPE, "NO", "YES");
        bctx.build()
            .test_global_attr_rename("CONFIGRATE", GLOBAL::CONFIG_RATE_V6);
        bctx.build().test_global_attr_bool_rename(
            "DONESIGNALSPOWERDOWN",
            GLOBAL::POWERDOWN_STATUS,
            "DISABLE",
            "ENABLE",
        );

        // COR1
        bctx.build()
            .test_global_attr_rename("BPI_PAGE_SIZE", GLOBAL::BPI_PAGE_SIZE);
        bctx.build()
            .global("BPI_PAGE_SIZE", "8")
            .test_global_attr_rename("BPI_1ST_READ_CYCLE", GLOBAL::BPI_1ST_READ_CYCLE);
        bctx.build()
            .global("GLUTMASK_B", "0")
            .test_global_attr_bool_rename("POST_CRC_EN", GLOBAL::POST_CRC_EN, "NO", "YES");
        bctx.build().test_global_attr_bool_rename(
            "POST_CRC_RECONFIG",
            GLOBAL::POST_CRC_RECONFIG,
            "NO",
            "YES",
        );
        bctx.build().test_global_attr_bool_rename(
            "POST_CRC_KEEP",
            GLOBAL::POST_CRC_KEEP,
            "NO",
            "YES",
        );
        bctx.build().test_global_attr_bool_rename(
            "POST_CRC_CORRECT",
            GLOBAL::POST_CRC_CORRECT,
            "NO",
            "YES",
        );
        bctx.build()
            .test_global_attr_bool_rename("POST_CRC_SEL", GLOBAL::POST_CRC_SEL, "0", "1");
        bctx.build()
            .test_global_attr_bool_rename("FUSE_NO_CDR", GLOBAL::FUSE_NO_CDR, "0", "1");
        bctx.build().test_global_attr_bool_rename(
            "SYSMONPARTIALRECONFIG",
            GLOBAL::SYSMON_PARTIAL_RECONFIG,
            "DISABLE",
            "ENABLE",
        );
        bctx.build().test_global_attr_bool_rename(
            "POST_CRC_INIT_FLAG",
            GLOBAL::POST_CRC_NO_PIN,
            "ENABLE",
            "DISABLE",
        );
        bctx.build().test_global_attr_bool_rename(
            "FALLBACKPULSEFWE",
            GLOBAL::FALLBACK_PULSE_FWE,
            "NO",
            "YES",
        );
        for (val, vname) in &backend.edev.db[enums::POST_CRC_FREQ].values {
            bctx.build()
                .test_bel_attr_val(GLOBAL::POST_CRC_FREQ, val)
                .global_diff_none("POST_CRC_CLK", "INTERNAL")
                .global("POST_CRC_FREQ", vname.strip_prefix('_').unwrap())
                .commit();
        }
        bctx.build()
            .no_global("POST_CRC_FREQ")
            .test_global_attr_rename("POST_CRC_CLK", GLOBAL::POST_CRC_CLK);

        // CTL
        // persist not fuzzed — too much effort
        bctx.build()
            .test_global_attr_rename("SECURITY", GLOBAL::SECURITY);
        bctx.build()
            .test_global_attr_rename("ENCRYPTKEYSELECT", GLOBAL::ENCRYPT_KEY_SELECT);
        bctx.build().test_global_attr_bool_rename(
            "OVERTEMPPOWERDOWN",
            GLOBAL::OVERTEMP_POWERDOWN,
            "DISABLE",
            "ENABLE",
        );
        bctx.build().test_global_attr_bool_rename(
            "CONFIGFALLBACK",
            GLOBAL::CONFIG_FALLBACK,
            "DISABLE",
            "ENABLE",
        );
        bctx.build().test_global_attr_bool_rename(
            "SELECTMAPABORT",
            GLOBAL::SELECTMAP_ABORT,
            "DISABLE",
            "ENABLE",
        );
        bctx.build().test_global_attr_bool_rename(
            "INITSIGNALSERROR",
            GLOBAL::INIT_SIGNALS_ERROR,
            "DISABLE",
            "ENABLE",
        );
        bctx.build()
            .test_global_attr_bool_rename("GTS_USR_B", GLOBAL::GTS_USR_B, "0", "1");

        bctx.build()
            .test_global_attr_bool_rename("ENCRYPT", GLOBAL::ENCRYPT, "NO", "YES");
        bctx.build()
            .test_global_attr_bool_rename("SECALL", GLOBAL::SEC_ALL, "NO", "YES");
        bctx.build()
            .test_global_attr_bool_rename("SECERROR", GLOBAL::SEC_ERROR, "NO", "YES");
        bctx.build()
            .test_global_attr_bool_rename("SECSTATUS", GLOBAL::SEC_STATUS, "NO", "YES");

        // CTL1
        bctx.build().test_global_attr_bool_rename(
            "ICAP_ENCRYPTION",
            GLOBAL::ICAP_ENCRYPTION,
            "DISABLE",
            "ENABLE",
        );
        bctx.build()
            .test_global_attr_bool_rename("VGG_TEST", GLOBAL::VGG_TEST, "NO", "YES");
        bctx.build()
            .test_global_attr_bool_rename("EN_VTEST", GLOBAL::EN_VTEST, "NO", "YES");
        bctx.build()
            .test_global_attr_bool_rename("DIS_VGG_REG", GLOBAL::DIS_VGG_REG, "NO", "YES");
        bctx.build().test_global_attr_bool_rename(
            "ENABLE_VGG_CLAMP",
            GLOBAL::ENABLE_VGG_CLAMP,
            "NO",
            "YES",
        );
        bctx.build()
            .test_global_attr_bool_rename("VGG_OPT_DRV", GLOBAL::VGG_OPT_DRV, "0", "1");
        bctx.build()
            .test_global_attr_bool_rename("VGG_V4_OPT", GLOBAL::VGG_V4_OPT, "0", "1");

        for (attr, val, vname) in [
            (GLOBAL::VGG_TEST, false, "DISABLE"),
            (GLOBAL::VGG_TEST, true, "TEST0"),
            (GLOBAL::EN_VTEST, true, "TEST1"),
        ] {
            bctx.build()
                .no_global("EN_VTEST")
                .no_global("VGG_TEST")
                .test_bel_attr_bits_bi(attr, val)
                .global("MODEPINTEST", vname)
                .commit();
        }

        for (attr, opt, width) in [
            (GLOBAL::VGG_SEL, "VGG_SEL", 5),
            (GLOBAL::VGG_SEL2, "VGG_SEL2", 5),
        ] {
            bctx.build()
                .test_bel_attr_bits(attr)
                .multi_global(opt, MultiValue::Bin, width);
        }

        // TIMER
        bctx.build()
            .no_global("TIMER_USR")
            .test_bel_attr_bits(GLOBAL::TIMER_CFG)
            .global("TIMER_CFG", "0")
            .commit();
        bctx.build()
            .no_global("TIMER_CFG")
            .test_bel_attr_bits(GLOBAL::TIMER_USR)
            .global("TIMER_USR", "0")
            .commit();
        bctx.build()
            .no_global("TIMER_USR")
            .test_bel_attr_bits(GLOBAL::TIMER)
            .multi_global("TIMER_CFG", MultiValue::Hex(0), 24);

        // TESTMODE
        {
            bctx.build()
                .test_bel_attr_bits(GLOBAL::FUSE_SHADOW)
                .multi_global("FUSE_SHADOW", MultiValue::Bin, 1);
        }

        // TRIM0
        {
            bctx.build()
                .test_bel_attr_bits(GLOBAL::MPD_SEL)
                .multi_global("MPD_SEL", MultiValue::Bin, 3);
        }

        // TRIM1
        {
            bctx.build()
                .test_bel_attr_bits(GLOBAL::V6_VBG_SEL)
                .multi_global("VBG_SEL", MultiValue::Bin, 6);
            bctx.build()
                .test_bel_attr_bits(GLOBAL::VBG_VGG_FLAST_SEL)
                .multi_global("VBGVGGFLASTSEL", MultiValue::Bin, 6);
            bctx.build()
                .test_bel_attr_bits(GLOBAL::VBG_VGG_NEG_SEL)
                .multi_global("VBGVGGNEGSEL", MultiValue::Bin, 6);
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
        ctx.collect_bel_attr(tcid, bslot, bcls::SYSMON_V5::V5_INIT);
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
        let bslot = bslots::HCLK_DRP[0];
        ctx.collect_bel_attr(tcid, bslot, bcls::HCLK_DRP::DRP_MASK_SYSMON);
    }

    {
        let tcid = tcls::GLOBAL;
        let bslot = bslots::GLOBAL;

        // COR
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
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            GLOBAL::MATCH_CYCLE,
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
        ctx.collect_bel_attr_ocd(tcid, bslot, GLOBAL::CONFIG_RATE_V6, OcdMode::BitOrder);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::DRIVE_DONE);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::DONE_PIPE);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::POWERDOWN_STATUS);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::CAPTURE_ONESHOT);
        // CRC disable gone — now done through altering bitstream structure instead (use an RCRC command instead of checking CRC)

        // COR1
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::BPI_PAGE_SIZE);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::BPI_1ST_READ_CYCLE);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::POST_CRC_EN);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::POST_CRC_RECONFIG);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::POST_CRC_KEEP);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::POST_CRC_CORRECT);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::POST_CRC_SEL);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::FUSE_NO_CDR);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::POST_CRC_NO_PIN);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::SYSMON_PARTIAL_RECONFIG);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::FALLBACK_PULSE_FWE);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::POST_CRC_FREQ);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::POST_CRC_CLK);

        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            GLOBAL::PERSIST_DEASSERT_AT_DESYNCH,
            TileBit::new(1, 0, 17).pos(),
        );

        // CTL
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::GTS_USR_B);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::SECURITY);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::OVERTEMP_POWERDOWN);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::CONFIG_FALLBACK);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::INIT_SIGNALS_ERROR);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::SELECTMAP_ABORT);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::ENCRYPT_KEY_SELECT);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::SEC_ALL);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::SEC_ERROR);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::SEC_STATUS);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::ENCRYPT);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::GLUTMASK);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::FARSRC);
        // these are too much trouble to deal with the normal way.
        ctx.insert_bel_attr_bool(tcid, bslot, GLOBAL::PERSIST, TileBit::new(2, 0, 3).pos());
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            GLOBAL::ICAP_SELECT,
            BelAttributeEnum {
                bits: vec![TileBit::new(2, 0, 30)],
                values: [
                    (enums::ICAP_SELECT::TOP, bits![0]),
                    (enums::ICAP_SELECT::BOTTOM, bits![1]),
                ]
                .into_iter()
                .collect(),
            },
        );

        // CTL1
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::ICAP_ENCRYPTION);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::VGG_TEST);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::EN_VTEST);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::DIS_VGG_REG);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::ENABLE_VGG_CLAMP);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::VGG_OPT_DRV);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::VGG_V4_OPT);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::VGG_SEL);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::VGG_SEL2);

        // TIMER
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::TIMER);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::TIMER_CFG);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::TIMER_USR);

        // WBSTAR
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::GLOBAL::V5_NEXT_CONFIG_ADDR,
            (0..26).map(|i| TileBit::new(5, 0, i).pos()).collect(),
        );
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::GLOBAL::REVISION_SELECT_TRISTATE,
            TileBit::new(5, 0, 26).neg(),
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::GLOBAL::REVISION_SELECT,
            (27..29).map(|i| TileBit::new(5, 0, i).pos()).collect(),
        );

        // TESTMODE
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::FUSE_SHADOW);

        // TRIM0
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::MPD_SEL);

        // TRIM1
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::V6_VBG_SEL);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::VBG_VGG_FLAST_SEL);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::VBG_VGG_NEG_SEL);
    }
}
