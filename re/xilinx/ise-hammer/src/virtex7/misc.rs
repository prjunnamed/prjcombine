use prjcombine_interconnect::{db::BelAttributeEnum, grid::TileCoord};
use prjcombine_re_collector::diff::{
    OcdMode, extract_bitvec_val, xlat_bit, xlat_bit_wide, xlat_bit_wide_bi,
};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{bits, bsdata::TileBit};
use prjcombine_virtex4::defs::{
    bcls::{self, GLOBAL, SYSMON_V5 as SYSMON},
    bslots, enums, tslots,
    virtex7::tcls,
};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::relation::{Delta, TileRelation},
    },
    virtex4::specials,
};

#[derive(Debug, Clone, Copy)]
struct TileGlobal;

impl TileRelation for TileGlobal {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        Some(edev.tile_cfg(tcrd.die).tile(tslots::GLOBAL))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };

    let mut ctx = FuzzCtx::new_null(session, backend);
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
            ctx.build()
                .extra_tiles_by_bel_attr_val(bslots::MISC_CFG, attr, val)
                .test_global_special(specials::MISC_CFG)
                .global(opt, vname)
                .commit();
        }
    }
    for (attr, opt) in [
        (bcls::MISC_CFG::M0_PULL, "M0PIN"),
        (bcls::MISC_CFG::M1_PULL, "M1PIN"),
        (bcls::MISC_CFG::M2_PULL, "M2PIN"),
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
            ctx.build()
                .extra_tiles_by_bel_attr_val(bslots::MISC_CFG, attr, val)
                .test_global_special(specials::MISC_CFG)
                .global(opt, vname)
                .commit();
        }
    }

    for (attr, opt) in [
        (bcls::STARTUP::GTS_SYNC, "GTS_SYNC"),
        (bcls::STARTUP::GSR_SYNC, "GSR_SYNC"),
    ] {
        for (val, vname) in [(false, "NO"), (true, "YES")] {
            ctx.build()
                .extra_tiles_by_bel_attr_bits_bi(bslots::STARTUP, attr, val)
                .test_global_special(specials::MISC_CFG)
                .global(opt, vname)
                .commit();
        }
    }
    ctx.build()
        .extra_tiles_by_bel_attr_bits(bslots::MISC_CFG, bcls::MISC_CFG::USERCODE)
        .test_global_special(specials::MISC_CFG)
        .multi_global("USERID", MultiValue::HexPrefix, 32);

    let mut ctx = FuzzCtx::new(session, backend, tcls::CFG);
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

    if edev.chips.len() == 1 && !edev.chips.first().unwrap().has_ps {
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
    {
        let mut bctx = ctx.bel(bslots::STARTUP);
        if edev.chips.len() == 1 {
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
                    .extra_tiles_by_bel_attr_val(bslots::GLOBAL, GLOBAL::STARTUP_CLOCK, val)
                    .pin("CLK")
                    .test_bel_special_val(specials::STARTUP_CLOCK, val)
                    .global("STARTUPCLK", vname)
                    .commit();
            }
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
        if edev.chips.first().unwrap().regs > 1 {
            bctx.mode("STARTUP")
                .global("ENCRYPT", "YES")
                .test_bel_attr_bits(bcls::STARTUP::KEY_CLEAR_ENABLE_TR)
                .pin("KEYCLEARB")
                .commit();
        }
        bctx.mode("STARTUP").test_bel_attr_bool_rename(
            "PROG_USR",
            bcls::STARTUP::PROG_USR_TR,
            "FALSE",
            "TRUE",
        );
    }
    if edev.chips.len() == 1 {
        let mut bctx = ctx.bel(bslots::CAPTURE);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("CAPTURE")
            .commit();
        for val in [false, true] {
            bctx.mode("CAPTURE")
                .null_bits()
                .extra_tile_attr_bits_bi(TileGlobal, bslots::GLOBAL, GLOBAL::CAPTURE_ONESHOT, val)
                .test_bel_special(specials::CAPTURE_ONESHOT)
                .attr("ONESHOT", if val { "TRUE" } else { "FALSE" })
                .commit();
        }
    }
    if edev.chips.len() == 1 {
        let mut bctx = ctx.bel(bslots::CFG_IO_ACCESS);
        bctx.build()
            .no_global("CFGIOACCESS_TDO")
            .extra_tiles_by_bel_attr_bits(bslots::GLOBAL, GLOBAL::CFG_IO_ACCESS_TDO)
            .test_bel_attr_bits(bcls::CFG_IO_ACCESS_V7::ENABLE_TR)
            .mode("CFG_IO_ACCESS")
            .commit();
        bctx.mode("CFG_IO_ACCESS")
            .null_bits()
            .extra_tiles_by_bel_attr_bits(bslots::GLOBAL, GLOBAL::CFG_IO_ACCESS_TDO)
            .test_bel_special(specials::CFGIOACCESS_TDO_UNCONNECTED)
            .global_diff_none("CFGIOACCESS_TDO", "UNCONNECTED")
            .commit();
    }
    if edev.chips.len() == 1 {
        let mut bctx = ctx.bel(bslots::FRAME_ECC);
        bctx.build()
            .null_bits()
            .extra_tile_attr_bits(TileGlobal, bslots::GLOBAL, GLOBAL::GLUTMASK)
            .no_global("GLUTMASK_B")
            .test_bel_special(specials::PRESENT)
            .mode("FRAME_ECC")
            .commit();
        for (val, vname) in &backend.edev.db[enums::FARSRC].values {
            bctx.mode("FRAME_ECC")
                .null_bits()
                .extra_tile_attr_val(TileGlobal, bslots::GLOBAL, GLOBAL::FARSRC, val)
                .test_bel(bslots::GLOBAL)
                .test_bel_attr_val(GLOBAL::FARSRC, val)
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
        let mut bctx = ctx.bel(bslots::DNA_PORT);
        bctx.build()
            .test_bel_attr_bits(bcls::DNA_PORT::ENABLE_TR)
            .mode("DNA_PORT")
            .commit();
    }

    let mut ctx = FuzzCtx::new_null(session, backend);
    {
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
            ctx.build()
                .extra_tiles_by_bel_attr_val(bslots::GLOBAL, GLOBAL::GWE_CYCLE, val)
                .test_global_special(specials::MISC_CFG)
                .global("GWE_CYCLE", vname)
                .commit();
            ctx.build()
                .extra_tiles_by_bel_attr_val(bslots::GLOBAL, GLOBAL::GTS_CYCLE, val)
                .test_global_special(specials::MISC_CFG)
                .global("GTS_CYCLE", vname)
                .commit();
            if val != enums::STARTUP_CYCLE::DONE {
                ctx.build()
                    .extra_tiles_by_bel_attr_val(bslots::GLOBAL, GLOBAL::DONE_CYCLE, val)
                    .test_global_special(specials::MISC_CFG)
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
            ctx.build()
                .extra_tiles_by_bel_attr_val(bslots::GLOBAL, GLOBAL::LOCK_CYCLE, val)
                .test_global_special(specials::MISC_CFG)
                .global("LCK_CYCLE", vname)
                .commit();
            ctx.build()
                .global_mutex("GLOBAL_DCI", "NO")
                .extra_tiles_by_bel_attr_val(bslots::GLOBAL, GLOBAL::MATCH_CYCLE, val)
                .test_global_special(specials::MISC_CFG)
                .global("MATCH_CYCLE", vname)
                .commit();
        }

        for (attr, opt, val0, val1) in [
            (GLOBAL::DRIVE_DONE, "DRIVEDONE", "NO", "YES"),
            (GLOBAL::DONE_PIPE, "DONEPIPE", "NO", "YES"),
            (
                GLOBAL::POWERDOWN_STATUS,
                "DONESIGNALSPOWERDOWN",
                "DISABLE",
                "ENABLE",
            ),
        ] {
            for (val, vname) in [(false, val0), (true, val1)] {
                ctx.build()
                    .extra_tiles_by_bel_attr_bits_bi(bslots::GLOBAL, attr, val)
                    .test_global_special(specials::MISC_CFG)
                    .global(opt, vname)
                    .commit();
            }
        }

        if !edev.chips.first().unwrap().has_ps {
            for (val, vname) in &backend.edev.db[enums::CONFIG_RATE_V7].values {
                ctx.build()
                    .extra_tiles_by_bel_attr_val(bslots::GLOBAL, GLOBAL::CONFIG_RATE_V7, val)
                    .test_global_special(specials::MISC_CFG)
                    .global("CONFIGRATE", vname.strip_prefix('_').unwrap())
                    .commit();
            }
        }

        // COR1
        for (val, vname) in [(false, "NO"), (true, "YES")] {
            ctx.build()
                .global("ENCRYPT", "NO")
                .global("GLUTMASK_B", "0")
                .extra_tiles_by_bel_attr_bits_bi(bslots::GLOBAL, GLOBAL::POST_CRC_EN, val)
                .test_global_special(specials::MISC_CFG)
                .global("POST_CRC_EN", vname)
                .commit();
        }
        for (val, vname) in &backend.edev.db[enums::POST_CRC_CLK].values {
            ctx.build()
                .no_global("POST_CRC_FREQ")
                .extra_tiles_by_bel_attr_val(bslots::GLOBAL, GLOBAL::POST_CRC_CLK, val)
                .test_global_special(specials::MISC_CFG)
                .global("POST_CRC_CLK", vname)
                .commit();
        }
        if !edev.chips.first().unwrap().has_ps {
            for (val, vname) in &backend.edev.db[enums::BPI_1ST_READ_CYCLE].values {
                let vname = vname.strip_prefix('_').unwrap();
                ctx.build()
                    .global("BPI_PAGE_SIZE", "8")
                    .extra_tiles_by_bel_attr_val(bslots::GLOBAL, GLOBAL::BPI_1ST_READ_CYCLE, val)
                    .test_global_special(specials::MISC_CFG)
                    .global("BPI_1ST_READ_CYCLE", vname)
                    .commit();
            }
            for (val, vname) in &backend.edev.db[enums::BPI_PAGE_SIZE].values {
                let vname = vname.strip_prefix('_').unwrap();
                ctx.build()
                    .extra_tiles_by_bel_attr_val(bslots::GLOBAL, GLOBAL::BPI_PAGE_SIZE, val)
                    .test_global_special(specials::MISC_CFG)
                    .global("BPI_PAGE_SIZE", vname)
                    .commit();
            }
        }
        for (val, vname) in &backend.edev.db[enums::POST_CRC_FREQ].values {
            let vname = vname.strip_prefix('_').unwrap();
            ctx.build()
                .extra_tiles_by_bel_attr_val(bslots::GLOBAL, GLOBAL::POST_CRC_FREQ, val)
                .test_global_special(specials::MISC_CFG)
                .global_diff_none("POST_CRC_CLK", "INTERNAL")
                .global("POST_CRC_FREQ", vname)
                .commit();
        }
        for (attr, opt, val0, val1) in [
            (GLOBAL::POST_CRC_RECONFIG, "POST_CRC_RECONFIG", "NO", "YES"),
            (GLOBAL::POST_CRC_KEEP, "POST_CRC_KEEP", "NO", "YES"),
            (GLOBAL::POST_CRC_CORRECT, "POST_CRC_CORRECT", "NO", "YES"),
            (GLOBAL::POST_CRC_SEL, "POST_CRC_SEL", "0", "1"),
            (
                GLOBAL::POST_CRC_NO_PIN,
                "POST_CRC_INIT_FLAG",
                "ENABLE",
                "DISABLE",
            ),
            (
                GLOBAL::SYSMON_PARTIAL_RECONFIG,
                "XADCPARTIALRECONFIG",
                "DISABLE",
                "ENABLE",
            ),
            (
                GLOBAL::TRIM_BITSTREAM,
                "TRIM_BITSTREAM",
                "DISABLE",
                "ENABLE",
            ),
        ] {
            for (val, vname) in [(false, val0), (true, val1)] {
                ctx.build()
                    .extra_tiles_by_bel_attr_bits_bi(bslots::GLOBAL, attr, val)
                    .test_global_special(specials::MISC_CFG)
                    .global(opt, vname)
                    .commit();
            }
        }

        // CTL
        for (val, vname) in [(false, "DISABLE"), (true, "ENABLE")] {
            ctx.build()
                .no_global("NEXT_CONFIG_REBOOT")
                .extra_tiles_by_bel_attr_bits_bi(bslots::GLOBAL, GLOBAL::CONFIG_FALLBACK, val)
                .test_global_special(specials::MISC_CFG)
                .global("CONFIGFALLBACK", vname)
                .commit();
        }
        for (val, vname) in &backend.edev.db[enums::SECURITY].values {
            ctx.build()
                .extra_tiles_by_bel_attr_val(bslots::GLOBAL, GLOBAL::SECURITY, val)
                .test_global_special(specials::MISC_CFG)
                .global("SECURITY", vname)
                .commit();
        }
        for (val, vname) in &backend.edev.db[enums::ENCRYPT_KEY_SELECT].values {
            ctx.build()
                .extra_tiles_by_bel_attr_val(bslots::GLOBAL, GLOBAL::ENCRYPT_KEY_SELECT, val)
                .test_global_special(specials::MISC_CFG)
                .global("ENCRYPTKEYSELECT", vname)
                .commit();
        }
        for (attr, opt, val0, val1) in [
            (GLOBAL::GTS_USR_B, "GTS_USR_B", "0", "1"),
            (GLOBAL::SEC_ALL, "SECALL", "NO", "YES"),
            (GLOBAL::SEC_ERROR, "SECERROR", "NO", "YES"),
            (GLOBAL::SEC_STATUS, "SECSTATUS", "NO", "YES"),
            (
                GLOBAL::OVERTEMP_POWERDOWN,
                "OVERTEMPPOWERDOWN",
                "DISABLE",
                "ENABLE",
            ),
            (
                GLOBAL::INIT_SIGNALS_ERROR,
                "INITSIGNALSERROR",
                "DISABLE",
                "ENABLE",
            ),
            (
                GLOBAL::SELECTMAP_ABORT,
                "SELECTMAPABORT",
                "DISABLE",
                "ENABLE",
            ),
            (GLOBAL::PERSIST, "PERSIST", "NO", "CTLREG"),
        ] {
            if edev.chips.first().unwrap().has_ps && attr == GLOBAL::SELECTMAP_ABORT {
                continue;
            }

            for (val, vname) in [(false, val0), (true, val1)] {
                ctx.build()
                    .extra_tiles_by_bel_attr_bits_bi(bslots::GLOBAL, attr, val)
                    .test_global_special(specials::MISC_CFG)
                    .global(opt, vname)
                    .commit();
            }
        }

        // CTL1
        ctx.build()
            .no_global("EN_VTEST")
            .no_global("VGG_TEST")
            .extra_tiles_by_bel_attr_bits(bslots::GLOBAL, GLOBAL::VGG_TEST)
            .test_global_special(specials::MISC_CFG)
            .global("MODEPINTEST", "TEST0")
            .commit();
        ctx.build()
            .no_global("EN_VTEST")
            .no_global("VGG_TEST")
            .extra_tiles_by_bel_attr_bits(bslots::GLOBAL, GLOBAL::EN_VTEST)
            .test_global_special(specials::MISC_CFG)
            .global("MODEPINTEST", "TEST1")
            .commit();
        for (attr, opt, val0, val1) in [
            (
                GLOBAL::ICAP_ENCRYPTION,
                "ICAP_ENCRYPTION",
                "DISABLE",
                "ENABLE",
            ),
            (GLOBAL::DIS_VGG_REG, "DIS_VGG_REG", "NO", "YES"),
            (GLOBAL::ENABLE_VGG_CLAMP, "ENABLE_VGG_CLAMP", "NO", "YES"),
        ] {
            for (val, vname) in [(false, val0), (true, val1)] {
                ctx.build()
                    .extra_tiles_by_bel_attr_bits_bi(bslots::GLOBAL, attr, val)
                    .test_global_special(specials::MISC_CFG)
                    .global(opt, vname)
                    .commit();
            }
        }
        for (attr, opt, width) in [
            (GLOBAL::VGG_SEL, "VGG_SEL", 5),
            (GLOBAL::VGG_NEG_GAIN_SEL, "VGG_NEG_GAIN_SEL", 5),
            (GLOBAL::VGG_POS_GAIN_SEL, "VGG_POS_GAIN_SEL", 1),
        ] {
            ctx.build()
                .extra_tiles_by_bel_attr_bits(bslots::GLOBAL, attr)
                .test_global_special(specials::MISC_CFG)
                .multi_global(opt, MultiValue::Bin, width);
        }

        // CTL, but affects bits in CTL1
        if edev.chips.first().unwrap().regs > 1 && !edev.chips.first().unwrap().has_ps {
            ctx.build()
                .extra_tiles_by_bel_attr_bits(bslots::GLOBAL, GLOBAL::ENCRYPT)
                .no_global("VGG_SEL")
                .no_global("VGG_POS_GAIN_SEL")
                .no_global("VGG_NEG_GAIN_SEL")
                .test_global_special(specials::MISC_CFG)
                .global("ENCRYPT", "YES")
                .commit();
        }

        // BSPI
        if !edev.chips.first().unwrap().has_ps {
            for (val, vname) in &backend.edev.db[enums::BPI_SYNC_MODE].values {
                ctx.build()
                    .global("SPI_OPCODE", "0x12")
                    .extra_tiles_by_bel_attr_val(bslots::GLOBAL, GLOBAL::BPI_SYNC_MODE, val)
                    .test_global_special(specials::MISC_CFG)
                    .global(
                        "BPI_SYNC_MODE",
                        if val == enums::BPI_SYNC_MODE::NONE {
                            "DISABLE"
                        } else {
                            vname
                        },
                    )
                    .commit();
            }
            for (val, vname) in &backend.edev.db[enums::SPI_BUSWIDTH].values {
                ctx.build()
                    .global("SPI_OPCODE", "0x12")
                    .extra_tiles_by_bel_attr_val(bslots::GLOBAL, GLOBAL::SPI_BUSWIDTH, val)
                    .test_global_special(specials::MISC_CFG)
                    .global("SPI_BUSWIDTH", vname.strip_prefix('_').unwrap())
                    .commit();
            }

            ctx.build()
                .global("BPI_SYNC_MODE", "TYPE1")
                .extra_tiles_by_bel_attr_bits(bslots::GLOBAL, GLOBAL::SPI_OPCODE)
                .test_global_special(specials::MISC_CFG)
                .multi_global("SPI_OPCODE", MultiValue::HexPrefix, 8);
        }

        // WBSTAR
        if !edev.chips.first().unwrap().has_ps {
            for (val, vname) in [(false, "DISABLE"), (true, "ENABLE")] {
                ctx.build()
                    .extra_tiles_by_bel_attr_bits_bi(
                        bslots::GLOBAL,
                        GLOBAL::REVISION_SELECT_TRISTATE,
                        val,
                    )
                    .test_global_special(specials::MISC_CFG)
                    .global("REVISIONSELECT_TRISTATE", vname)
                    .commit();
            }
            ctx.build()
                .global("NEXT_CONFIG_REBOOT", "DISABLE")
                .extra_tiles_by_bel_attr_bits(bslots::GLOBAL, GLOBAL::V7_NEXT_CONFIG_ADDR)
                .test_global_special(specials::MISC_CFG)
                .multi_global("NEXT_CONFIG_ADDR", MultiValue::HexPrefix, 29);
            ctx.build()
                .global("NEXT_CONFIG_REBOOT", "DISABLE")
                .extra_tiles_by_bel_attr_bits(bslots::GLOBAL, GLOBAL::REVISION_SELECT)
                .test_global_special(specials::MISC_CFG)
                .multi_global("REVISIONSELECT", MultiValue::Bin, 2);
        }

        // TIMER
        if !edev.chips.first().unwrap().has_ps {
            ctx.build()
                .no_global("TIMER_USR")
                .extra_tiles_by_bel_attr_bits(bslots::GLOBAL, GLOBAL::TIMER_CFG)
                .test_global_special(specials::MISC_CFG)
                .global("TIMER_CFG", "0")
                .commit();
            ctx.build()
                .no_global("TIMER_CFG")
                .extra_tiles_by_bel_attr_bits(bslots::GLOBAL, GLOBAL::TIMER_USR)
                .test_global_special(specials::MISC_CFG)
                .global("TIMER_USR", "0")
                .commit();
            ctx.build()
                .no_global("TIMER_USR")
                .extra_tiles_by_bel_attr_bits(bslots::GLOBAL, GLOBAL::TIMER)
                .test_global_special(specials::MISC_CFG)
                .multi_global("TIMER_CFG", MultiValue::Hex(0), 24);
        }
    }

    for (attr, opt, width, anchor, anchor_val) in [
        // TESTMODE
        (
            GLOBAL::TEST_REF_SEL,
            "TEST_REF_SEL",
            3,
            "TEST_VGG_ENABLE",
            "1",
        ),
        (
            GLOBAL::TEST_VGG_SEL,
            "TEST_VGG_SEL",
            4,
            "TEST_VGG_ENABLE",
            "1",
        ),
        (
            GLOBAL::TEST_NEG_SLOPE_VGG,
            "TEST_NEG_SLOPE_VGG",
            1,
            "TEST_VGG_ENABLE",
            "1",
        ),
        (
            GLOBAL::TEST_VGG_ENABLE,
            "TEST_VGG_ENABLE",
            1,
            "TEST_NEG_SLOPE_VGG",
            "1",
        ),
        // TRIM0
        (GLOBAL::MPD_SEL, "MPD_SEL", 3, "MPD_OVERRIDE", "1"),
        (GLOBAL::TRIM_SPARE, "TRIM_SPARE", 2, "MPD_OVERRIDE", "1"),
        (
            GLOBAL::MPD_DIS_OVERRIDE,
            "MPD_DIS_OVERRIDE",
            1,
            "MPD_OVERRIDE",
            "1",
        ),
        (
            GLOBAL::MPD_OVERRIDE,
            "MPD_OVERRIDE",
            1,
            "MPD_DIS_OVERRIDE",
            "1",
        ),
        // TRIM 1
        (GLOBAL::VGGSEL, "VGGSEL", 6, "VBG_FLAT_SEL", "111111"),
        (GLOBAL::VGGSEL2, "VGGSEL2", 6, "VBG_FLAT_SEL", "111111"),
        (GLOBAL::VBG_FLAT_SEL, "VBG_FLAT_SEL", 6, "VGGSEL", "111111"),
        // TRIM2
        (
            GLOBAL::VGG_TRIM_BOT,
            "VGG_TRIM_BOT",
            12,
            "VGG_TRIM_TOP",
            "111111111111",
        ),
        (
            GLOBAL::VGG_TRIM_TOP,
            "VGG_TRIM_TOP",
            12,
            "VGG_TRIM_BOT",
            "111111111111",
        ),
    ] {
        ctx.build()
            .global(anchor, anchor_val)
            .extra_tiles_by_bel_attr_bits(bslots::GLOBAL, attr)
            .test_global_special(specials::MISC_CFG)
            .multi_global(opt, MultiValue::Bin, width);
    }

    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::SYSMON) {
        let mut bctx = ctx.bel(bslots::SYSMON);
        bctx.build()
            .null_bits()
            .extra_tile_bel_special(
                Delta::new(0, 0, tcls::HCLK),
                bslots::HCLK_DRP[0],
                specials::DRP_MASK_SYSMON,
            )
            .test_bel_special(specials::PRESENT)
            .mode("XADC")
            .commit();
        bctx.mode("XADC").test_bel_input_inv_auto(SYSMON::DCLK);
        bctx.mode("XADC").test_bel_input_inv_auto(SYSMON::CONVSTCLK);
        for i in 0x40..0x60 {
            bctx.mode("XADC")
                .global_mutex("SYSMON", "SYSMON")
                .test_bel_attr_bits_base(SYSMON::V7_INIT, (i - 0x40) * 16)
                .multi_attr(format!("INIT_{i:02X}"), MultiValue::Hex(0), 16);
        }
        for attr in [
            SYSMON::SYSMON_TEST_A,
            SYSMON::SYSMON_TEST_B,
            SYSMON::SYSMON_TEST_C,
            SYSMON::SYSMON_TEST_D,
            SYSMON::SYSMON_TEST_E,
        ] {
            bctx.mode("XADC")
                .global_mutex("SYSMON", "SYSMON")
                .test_bel_attr_multi(attr, MultiValue::Hex(0));
        }
        let mut ctx = FuzzCtx::new_null(session, backend);
        for (attr, vals) in [
            (
                "JTAG_XADC",
                &[
                    ("ENABLE", specials::SYSMON_JTAG_XADC_ENABLE),
                    ("DISABLE", specials::SYSMON_JTAG_XADC_DISABLE),
                    ("STATUSONLY", specials::SYSMON_JTAG_XADC_STATUSONLY),
                ][..],
            ),
            (
                "XADCPOWERDOWN",
                &[
                    ("ENABLE", specials::SYSMON_XADCPOWERDOWN_ENABLE),
                    ("DISABLE", specials::SYSMON_XADCPOWERDOWN_DISABLE),
                ][..],
            ),
            (
                "XADCENHANCEDLINEARITY",
                &[
                    ("ON", specials::SYSMON_XADCENHANCEDLINEARITY_ON),
                    ("OFF", specials::SYSMON_XADCENHANCEDLINEARITY_OFF),
                ][..],
            ),
        ] {
            for &(val, spec) in vals {
                ctx.build()
                    .global_mutex("SYSMON", "OPT")
                    .extra_tiles_by_bel_special(bslots::SYSMON, spec)
                    .test_global_special(spec)
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
            bcls::MISC_CFG::M0_PULL,
            bcls::MISC_CFG::M1_PULL,
            bcls::MISC_CFG::M2_PULL,
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
        if edev.chips.len() == 1 && !edev.chips.first().unwrap().has_ps {
            ctx.collect_bel_attr(tcid, bslot, bcls::MISC_CFG::ICAP_WIDTH);
        }
        let bits = xlat_bit_wide_bi(
            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::MISC_CFG::DISABLE_JTAG_TR, false),
            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::MISC_CFG::DISABLE_JTAG_TR, true),
        );
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::MISC_CFG::DISABLE_JTAG_TR, bits);
    }
    for bslot in bslots::BSCAN {
        ctx.collect_bel_attr(tcid, bslot, bcls::BSCAN::ENABLE);
    }

    if edev.chips.len() == 1 && !edev.chips.first().unwrap().has_ps {
        for bslot in bslots::ICAP {
            let bits = xlat_bit_wide(ctx.get_diff_attr_bool(tcid, bslot, bcls::ICAP_V6::ENABLE_TR));
            ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::ICAP_V6::ENABLE_TR, bits);
        }
    }

    {
        let bslot = bslots::STARTUP;
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::STARTUP::GSR_SYNC);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::STARTUP::GTS_SYNC);
        for attr in [
            bcls::STARTUP::USER_GTS_GSR_ENABLE_TR,
            bcls::STARTUP::USRCCLK_ENABLE_TR,
        ] {
            let bits = xlat_bit_wide(ctx.get_diff_attr_bool(tcid, bslot, attr));
            ctx.insert_bel_attr_bitvec(tcid, bslot, attr, bits);
        }
        let bits = xlat_bit_wide_bi(
            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::STARTUP::PROG_USR_TR, false),
            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::STARTUP::PROG_USR_TR, true),
        );
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::STARTUP::PROG_USR_TR, bits);
        if edev.chips.first().unwrap().regs > 1 {
            let bits = xlat_bit_wide(ctx.get_diff_attr_bool(
                tcid,
                bslot,
                bcls::STARTUP::KEY_CLEAR_ENABLE_TR,
            ));
            ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::STARTUP::KEY_CLEAR_ENABLE_TR, bits);
        }
    }
    {
        let bslot = bslots::DCIRESET;
        let bits = xlat_bit_wide(ctx.get_diff_attr_bool(tcid, bslot, bcls::DCIRESET::ENABLE_TR));
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCIRESET::ENABLE_TR, bits);
    }
    if edev.chips.len() == 1 {
        let bslot = bslots::CFG_IO_ACCESS;
        let bits =
            xlat_bit_wide(ctx.get_diff_attr_bool(tcid, bslot, bcls::CFG_IO_ACCESS_V7::ENABLE_TR));
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::CFG_IO_ACCESS_V7::ENABLE_TR, bits);
    }
    {
        let bslot = bslots::DNA_PORT;
        let bits = xlat_bit_wide(ctx.get_diff_attr_bool(tcid, bslot, bcls::DNA_PORT::ENABLE_TR));
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DNA_PORT::ENABLE_TR, bits);
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
        if edev.chips.len() == 1 {
            ctx.collect_bel_attr(tcid, bslot, GLOBAL::STARTUP_CLOCK);
            ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::CAPTURE_ONESHOT);
        }
        if !edev.chips.first().unwrap().has_ps {
            ctx.collect_bel_attr_ocd(tcid, bslot, GLOBAL::CONFIG_RATE_V7, OcdMode::BitOrder);
        }
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::DRIVE_DONE);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::DONE_PIPE);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::POWERDOWN_STATUS);

        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            GLOBAL::EXTMASTERCCLK_EN,
            TileBit::new(0, 0, 26).pos(),
        );
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            GLOBAL::EXTMASTERCCLK_DIV,
            BelAttributeEnum {
                bits: vec![TileBit::new(0, 0, 21), TileBit::new(0, 0, 22)],
                values: [
                    (enums::EXTMASTERCCLK_DIV::_8, bits![0, 0]),
                    (enums::EXTMASTERCCLK_DIV::_4, bits![1, 0]),
                    (enums::EXTMASTERCCLK_DIV::_2, bits![0, 1]),
                    (enums::EXTMASTERCCLK_DIV::_1, bits![1, 1]),
                ]
                .into_iter()
                .collect(),
            },
        );

        // COR1
        if !edev.chips.first().unwrap().has_ps {
            ctx.collect_bel_attr(tcid, bslot, GLOBAL::BPI_PAGE_SIZE);
            ctx.collect_bel_attr(tcid, bslot, GLOBAL::BPI_1ST_READ_CYCLE);
        }
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::POST_CRC_CLK);
        ctx.collect_bel_attr_ocd(tcid, bslot, GLOBAL::POST_CRC_FREQ, OcdMode::BitOrder);

        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::POST_CRC_EN);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::POST_CRC_RECONFIG);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::POST_CRC_KEEP);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::POST_CRC_CORRECT);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::POST_CRC_SEL);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::POST_CRC_NO_PIN);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::SYSMON_PARTIAL_RECONFIG);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::TRIM_BITSTREAM);
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            GLOBAL::PERSIST_DEASSERT_AT_DESYNCH,
            TileBit::new(1, 0, 17).pos(),
        );
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::CFG_IO_ACCESS_TDO);
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            GLOBAL::TRIM_REG,
            vec![TileBit::new(1, 0, 10).pos(), TileBit::new(1, 0, 11).pos()],
        );

        // CTL
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::GTS_USR_B);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::SECURITY);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::OVERTEMP_POWERDOWN);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::CONFIG_FALLBACK);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::INIT_SIGNALS_ERROR);
        if !edev.chips.first().unwrap().has_ps {
            ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::SELECTMAP_ABORT);
        }
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::ENCRYPT_KEY_SELECT);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::SEC_ALL);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::SEC_ERROR);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::SEC_STATUS);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::PERSIST);
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
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::GLUTMASK);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::FARSRC);

        // CTL1
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::ICAP_ENCRYPTION);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::DIS_VGG_REG);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::ENABLE_VGG_CLAMP);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::VGG_TEST);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::EN_VTEST);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::VGG_SEL);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::VGG_NEG_GAIN_SEL);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::VGG_POS_GAIN_SEL);
        // CTL, but need to clean up diff bits from CTL1
        if edev.chips.first().unwrap().regs > 1 && !edev.chips.first().unwrap().has_ps {
            let mut diff = ctx.get_diff_attr_bool(tcid, bslot, GLOBAL::ENCRYPT);
            diff.apply_bitvec_diff_int(
                ctx.bel_attr_bitvec(tcid, bslot, GLOBAL::VGG_POS_GAIN_SEL),
                1,
                0,
            );
            diff.apply_bitvec_diff_int(
                ctx.bel_attr_bitvec(tcid, bslot, GLOBAL::VGG_NEG_GAIN_SEL),
                0xf,
                0,
            );
            diff.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, GLOBAL::VGG_SEL), 0xf, 0);
            ctx.insert_bel_attr_bool(tcid, bslot, GLOBAL::ENCRYPT, xlat_bit(diff));
        }

        // BSPI
        if !edev.chips.first().unwrap().has_ps {
            ctx.collect_bel_attr(tcid, bslot, GLOBAL::SPI_BUSWIDTH);
            ctx.collect_bel_attr(tcid, bslot, GLOBAL::SPI_OPCODE);
            let bits = Vec::from_iter((12..28).map(|i| TileBit::new(10, 0, i).pos()));
            ctx.get_diff_attr_val(
                tcid,
                bslot,
                GLOBAL::BPI_SYNC_MODE,
                enums::BPI_SYNC_MODE::NONE,
            )
            .assert_empty();
            let type1 = extract_bitvec_val(
                &bits,
                &bits![0; 16],
                ctx.get_diff_attr_val(
                    tcid,
                    bslot,
                    GLOBAL::BPI_SYNC_MODE,
                    enums::BPI_SYNC_MODE::TYPE1,
                ),
            );
            let type2 = extract_bitvec_val(
                &bits,
                &bits![0; 16],
                ctx.get_diff_attr_val(
                    tcid,
                    bslot,
                    GLOBAL::BPI_SYNC_MODE,
                    enums::BPI_SYNC_MODE::TYPE2,
                ),
            );
            let item = BelAttributeEnum {
                bits: bits.iter().map(|bit| bit.bit).collect(),
                values: [
                    (enums::BPI_SYNC_MODE::NONE, bits![0; 16]),
                    (enums::BPI_SYNC_MODE::TYPE1, type1),
                    (enums::BPI_SYNC_MODE::TYPE2, type2),
                ]
                .into_iter()
                .collect(),
            };
            ctx.insert_bel_attr_enum(tcid, bslot, GLOBAL::BPI_SYNC_MODE, item);
        }

        // WBSTAR
        if !edev.chips.first().unwrap().has_ps {
            ctx.collect_bel_attr(tcid, bslot, GLOBAL::V7_NEXT_CONFIG_ADDR);
            ctx.collect_bel_attr(tcid, bslot, GLOBAL::REVISION_SELECT);
            ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::REVISION_SELECT_TRISTATE);
        }

        // TIMER
        if !edev.chips.first().unwrap().has_ps {
            ctx.collect_bel_attr(tcid, bslot, GLOBAL::TIMER);
            ctx.collect_bel_attr(tcid, bslot, GLOBAL::TIMER_CFG);
            ctx.collect_bel_attr(tcid, bslot, GLOBAL::TIMER_USR);
        }

        // TESTMODE
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::TEST_REF_SEL);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::TEST_VGG_SEL);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::TEST_NEG_SLOPE_VGG);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::TEST_VGG_ENABLE);

        // TRIM0
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::MPD_SEL);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::TRIM_SPARE);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::MPD_DIS_OVERRIDE);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::MPD_OVERRIDE);

        // TRIM1
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::VGGSEL);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::VGGSEL2);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::VBG_FLAT_SEL);

        // TRIM2
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::VGG_TRIM_BOT);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::VGG_TRIM_TOP);
    }

    if ctx.has_tcls(tcls::SYSMON) {
        let tcid = tcls::SYSMON;
        let bslot = bslots::SYSMON;
        ctx.collect_bel_input_inv_bi(tcid, bslot, SYSMON::CONVSTCLK);
        ctx.collect_bel_input_inv_bi(tcid, bslot, SYSMON::DCLK);
        ctx.collect_bel_attr(tcid, bslot, SYSMON::V7_INIT);
        ctx.collect_bel_attr(tcid, bslot, SYSMON::SYSMON_TEST_A);
        ctx.collect_bel_attr(tcid, bslot, SYSMON::SYSMON_TEST_B);
        ctx.collect_bel_attr(tcid, bslot, SYSMON::SYSMON_TEST_C);
        ctx.collect_bel_attr(tcid, bslot, SYSMON::SYSMON_TEST_D);
        ctx.collect_bel_attr(tcid, bslot, SYSMON::SYSMON_TEST_E);

        ctx.get_diff_bel_special(tcid, bslot, specials::SYSMON_JTAG_XADC_ENABLE)
            .assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::SYSMON_JTAG_XADC_DISABLE);
        diff.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, SYSMON::SYSMON_TEST_E),
            7,
            0,
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::SYSMON_JTAG_XADC_STATUSONLY);
        diff.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, SYSMON::SYSMON_TEST_E),
            0xc8,
            0,
        );
        diff.assert_empty();

        let mut diff =
            ctx.get_diff_bel_special(tcid, bslot, specials::SYSMON_XADCENHANCEDLINEARITY_ON);
        diff.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, SYSMON::SYSMON_TEST_C),
            0x10,
            0,
        );
        diff.assert_empty();
        ctx.get_diff_bel_special(tcid, bslot, specials::SYSMON_XADCENHANCEDLINEARITY_OFF)
            .assert_empty();

        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::SYSMON_XADCPOWERDOWN_ENABLE);
        diff.apply_bitvec_diff_int(
            &ctx.bel_attr_bitvec(tcid, bslot, SYSMON::V7_INIT)[2 * 16..3 * 16],
            0x30,
            0,
        );
        diff.assert_empty();
        ctx.get_diff_bel_special(tcid, bslot, specials::SYSMON_XADCPOWERDOWN_DISABLE)
            .assert_empty();

        let tcid = tcls::HCLK;
        let bslot = bslots::HCLK_DRP[0];
        let bit = xlat_bit(ctx.get_diff_bel_special(tcid, bslot, specials::DRP_MASK_SYSMON));
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::HCLK_DRP::DRP_MASK_N, bit);
    }
}
