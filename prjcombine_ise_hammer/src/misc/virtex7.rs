use bitvec::prelude::*;
use prjcombine_collector::{extract_bitvec_val, xlat_bit, xlat_enum_ocd, OcdMode};
use prjcombine_hammer::Session;
use prjcombine_types::tiledb::{TileBit, TileItem, TileItemKind};
use prjcombine_virtex_bitstream::Reg;
use prjcombine_xilinx_geom::ExpandedDevice;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_multi_attr_hex, fuzz_multi_extras, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ExpandedDevice::Virtex4(ref edev) = backend.edev else {
        unreachable!()
    };
    let ctx = FuzzCtx::new_fake_tile(session, backend, "NULL", "NULL", TileBits::Null);
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
            fuzz_one_extras!(ctx, attr, val, [], [
                (global_opt attr, val)
            ], vec![
                ExtraFeature::new(
                    ExtraFeatureKind::AllCfg,
                    "CFG",
                    bel,
                    attr,
                    val,
                ),
            ]);
        }
    }
    fuzz_multi_extras!(ctx, "USERID", "", 32, [], (global_hex_prefix "USERID"), vec![
        ExtraFeature::new(
            ExtraFeatureKind::AllCfg,
            "CFG",
            "BSCAN_COMMON",
            "USERID",
            "",
        ),
    ]);

    for i in 0..4 {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "CFG",
            format!("BSCAN{i}"),
            TileBits::MainAuto,
        );
        fuzz_one!(ctx, "ENABLE", "1", [], [(mode "BSCAN")]);
        fuzz_enum!(ctx, "DISABLE_JTAG", ["FALSE", "TRUE"], [
            (global_mutex_site "DISABLE_JTAG"),
            (mode "BSCAN")
        ]);
    }

    if edev.grids.len() == 1 && !edev.grids.first().unwrap().has_ps {
        let ctx = FuzzCtx::new(session, backend, "CFG", "ICAP1", TileBits::MainAuto);
        let obel_top = ctx.bel;
        fuzz_one!(ctx, "ENABLE", "1", [], [(mode "ICAP")]);
        fuzz_enum!(ctx, "ICAP_WIDTH", ["X8", "X16", "X32"], [
            (mode "ICAP"),
            (global_mutex_site "ICAP")
        ]);
        fuzz_enum!(ctx, "ICAP_AUTO_SWITCH", ["DISABLE", "ENABLE"], [
            (mode "ICAP"),
            (global_mutex_site "ICAP")
        ]);

        let ctx = FuzzCtx::new(session, backend, "CFG", "ICAP0", TileBits::MainAuto);
        fuzz_one!(ctx, "ENABLE", "1", [
            (bel_mode obel_top, "ICAP")
        ], [(mode "ICAP")]);
        fuzz_enum!(ctx, "ICAP_WIDTH", ["X8", "X16", "X32"], [
            (bel_mode obel_top, "ICAP"),
            (mode "ICAP"),
            (global_mutex_site "ICAP")
        ]);
        fuzz_enum!(ctx, "ICAP_AUTO_SWITCH", ["DISABLE", "ENABLE"], [
            (bel_mode obel_top, "ICAP"),
            (mode "ICAP"),
            (global_mutex_site "ICAP")
        ]);
    }
    {
        if edev.grids.len() == 1 {
            let ctx = FuzzCtx::new(session, backend, "CFG", "STARTUP", TileBits::Null);
            fuzz_one!(ctx, "PRESENT", "1", [], [(mode "STARTUP")]);
            for val in ["CCLK", "USERCLK", "JTAGCLK"] {
                fuzz_one_extras!(ctx, "STARTUPCLK", val, [
                    (mode "STARTUP"),
                    (pin "CLK")
                ], [
                    (global_opt "STARTUPCLK", val)
                ], vec![
                    ExtraFeature::new(ExtraFeatureKind::Reg(Reg::Cor0), "REG.COR", "STARTUP", "STARTUPCLK", val)
                ]);
            }
        }
        let ctx = FuzzCtx::new(session, backend, "CFG", "STARTUP", TileBits::MainAuto);
        fuzz_one!(ctx, "PIN.GTS", "1", [(mode "STARTUP"), (nopin "GSR")], [(pin "GTS")]);
        fuzz_one!(ctx, "PIN.GSR", "1", [(mode "STARTUP"), (nopin "GTS")], [(pin "GSR")]);
        fuzz_one!(ctx, "PIN.USRCCLKO", "1", [(mode "STARTUP")], [(pin "USRCCLKO")]);
        if edev.grids.first().unwrap().regs > 1 {
            fuzz_one!(ctx, "PIN.KEYCLEARB", "1", [
                (mode "STARTUP"),
                (global_opt "ENCRYPT", "YES")
            ], [
                (pin "KEYCLEARB")
            ]);
        }
        fuzz_enum!(ctx, "PROG_USR", ["FALSE", "TRUE"], [(mode "STARTUP")]);
    }
    if edev.grids.len() == 1 {
        let ctx = FuzzCtx::new(session, backend, "CFG", "CAPTURE", TileBits::Null);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "CAPTURE")]);
        for val in ["FALSE", "TRUE"] {
            fuzz_one_extras!(ctx, "ONESHOT", val, [
                (mode "CAPTURE")
            ], [
                (attr "ONESHOT", val)
            ], vec![
                ExtraFeature::new(ExtraFeatureKind::Reg(Reg::Cor0), "REG.COR", "CAPTURE", "ONESHOT", val)
            ]);
        }
    }
    if edev.grids.len() == 1 {
        let ctx = FuzzCtx::new(session, backend, "CFG", "CFG_IO_ACCESS", TileBits::MainAuto);
        fuzz_one_extras!(ctx, "ENABLE", "1", [
            (no_global_opt "CFGIOACCESS_TDO")
        ], [
            (mode "CFG_IO_ACCESS")
        ], vec![
            ExtraFeature::new(
                ExtraFeatureKind::Reg(Reg::Cor1),
                "REG.COR1",
                "CFG_IO_ACCESS",
                "ENABLE",
                "1",
            ),
        ]);
        fuzz_one_extras!(ctx, "TDO", "UNCONNECTED", [
            (mode "CFG_IO_ACCESS")
        ], [
            (global_opt "CFGIOACCESS_TDO", "UNCONNECTED")
        ], vec![
            ExtraFeature::new(
                ExtraFeatureKind::Reg(Reg::Cor1),
                "REG.COR1",
                "CFG_IO_ACCESS",
                "TDO",
                "UNCONNECTED",
            ),
        ]);
    }
    if edev.grids.len() == 1 {
        let ctx = FuzzCtx::new(session, backend, "CFG", "FRAME_ECC", TileBits::Null);
        fuzz_one_extras!(ctx, "PRESENT", "1", [
            (no_global_opt "GLUTMASK_B")
        ], [
            (mode "FRAME_ECC")
        ], vec![
            ExtraFeature::new(ExtraFeatureKind::Reg(Reg::Ctl0), "REG.CTL", "FRAME_ECC", "ENABLE", "1"),
        ]);
        for val in ["FAR", "EFAR"] {
            fuzz_one_extras!(ctx, "FARSRC", "1", [
                (mode "FRAME_ECC")
            ], [
                (attr "FARSRC", val)
            ], vec![
                ExtraFeature::new(ExtraFeatureKind::Reg(Reg::Ctl0), "REG.CTL", "FRAME_ECC", "FARSRC", val),
            ]);
        }
    }
    {
        let ctx = FuzzCtx::new(session, backend, "CFG", "DCIRESET", TileBits::MainAuto);
        fuzz_one!(ctx, "ENABLE", "1", [], [(mode "DCIRESET")]);
    }
    {
        let ctx = FuzzCtx::new(session, backend, "CFG", "DNA_PORT", TileBits::MainAuto);
        fuzz_one!(ctx, "ENABLE", "1", [], [(mode "DNA_PORT")]);
    }

    let ctx = FuzzCtx::new_fake_tile(session, backend, "NULL", "NULL", TileBits::Null);
    for (reg, bel, attr, opt, vals) in [
        // COR
        (
            Reg::Cor0,
            "STARTUP",
            "GWE_CYCLE",
            "GWE_CYCLE",
            &["1", "2", "3", "4", "5", "6", "DONE", "KEEP"][..],
        ),
        (
            Reg::Cor0,
            "STARTUP",
            "GTS_CYCLE",
            "GTS_CYCLE",
            &["1", "2", "3", "4", "5", "6", "DONE", "KEEP"][..],
        ),
        (
            Reg::Cor0,
            "STARTUP",
            "DONE_CYCLE",
            "DONE_CYCLE",
            &["1", "2", "3", "4", "5", "6", "KEEP"][..],
        ),
        (
            Reg::Cor0,
            "STARTUP",
            "LCK_CYCLE",
            "LCK_CYCLE",
            &["0", "1", "2", "3", "4", "5", "6", "NOWAIT"][..],
        ),
        (
            Reg::Cor0,
            "STARTUP",
            "MATCH_CYCLE",
            "MATCH_CYCLE",
            &["0", "1", "2", "3", "4", "5", "6", "NOWAIT"][..],
        ),
        (
            Reg::Cor0,
            "STARTUP",
            "DRIVE_DONE",
            "DRIVEDONE",
            &["NO", "YES"][..],
        ),
        (
            Reg::Cor0,
            "STARTUP",
            "DONE_PIPE",
            "DONEPIPE",
            &["NO", "YES"][..],
        ),
        (
            Reg::Cor0,
            "STARTUP",
            "CONFIG_RATE",
            "CONFIGRATE",
            &[
                "3", "6", "9", "12", "16", "22", "26", "33", "40", "50", "66",
            ],
        ),
        (
            Reg::Cor0,
            "STARTUP",
            "DONE_SIGNALS_POWERDOWN",
            "DONESIGNALSPOWERDOWN",
            &["DISABLE", "ENABLE"][..],
        ),
        // COR1
        (
            Reg::Cor1,
            "MISC",
            "BPI_PAGE_SIZE",
            "BPI_PAGE_SIZE",
            &["1", "4", "8"],
        ),
        (
            Reg::Cor1,
            "MISC",
            "BPI_1ST_READ_CYCLE",
            "BPI_1ST_READ_CYCLE",
            &["1", "2", "3", "4"],
        ),
        (
            Reg::Cor1,
            "MISC",
            "POST_CRC_CLK",
            "POST_CRC_CLK",
            &["CFG_CLK", "INTERNAL"],
        ),
        (
            Reg::Cor1,
            "MISC",
            "POST_CRC_FREQ",
            "POST_CRC_FREQ",
            &["1", "2", "3", "6", "13", "25", "50"],
        ),
        (
            Reg::Cor1,
            "MISC",
            "POST_CRC_EN",
            "POST_CRC_EN",
            &["NO", "YES"],
        ),
        (
            Reg::Cor1,
            "MISC",
            "POST_CRC_RECONFIG",
            "POST_CRC_RECONFIG",
            &["NO", "YES"],
        ),
        (
            Reg::Cor1,
            "MISC",
            "POST_CRC_KEEP",
            "POST_CRC_KEEP",
            &["NO", "YES"],
        ),
        (
            Reg::Cor1,
            "MISC",
            "POST_CRC_CORRECT",
            "POST_CRC_CORRECT",
            &["NO", "YES"],
        ),
        (
            Reg::Cor1,
            "MISC",
            "POST_CRC_SEL",
            "POST_CRC_SEL",
            &["0", "1"],
        ),
        (
            Reg::Cor1,
            "MISC",
            "POST_CRC_INIT_FLAG",
            "POST_CRC_INIT_FLAG",
            &["DISABLE", "ENABLE"],
        ),
        (
            Reg::Cor1,
            "MISC",
            "XADC_PARTIAL_RECONFIG",
            "XADCPARTIALRECONFIG",
            &["DISABLE", "ENABLE"],
        ),
        (
            Reg::Cor1,
            "MISC",
            "TRIM_BITSTREAM",
            "TRIM_BITSTREAM",
            &["DISABLE", "ENABLE"],
        ),
        // CTL
        (Reg::Ctl0, "MISC", "GTS_USR_B", "GTS_USR_B", &["0", "1"]),
        (Reg::Ctl0, "MISC", "SEC_ALL", "SECALL", &["NO", "YES"]),
        (Reg::Ctl0, "MISC", "SEC_ERROR", "SECERROR", &["NO", "YES"]),
        (Reg::Ctl0, "MISC", "SEC_STATUS", "SECSTATUS", &["NO", "YES"]),
        (
            Reg::Ctl0,
            "MISC",
            "SECURITY",
            "SECURITY",
            &["NONE", "LEVEL1", "LEVEL2"],
        ),
        (
            Reg::Ctl0,
            "MISC",
            "ENCRYPT_KEY_SELECT",
            "ENCRYPTKEYSELECT",
            &["BBRAM", "EFUSE"],
        ),
        (
            Reg::Ctl0,
            "MISC",
            "OVERTEMP_POWERDOWN",
            "OVERTEMPPOWERDOWN",
            &["DISABLE", "ENABLE"],
        ),
        (
            Reg::Ctl0,
            "MISC",
            "CONFIG_FALLBACK",
            "CONFIGFALLBACK",
            &["DISABLE", "ENABLE"],
        ),
        (
            Reg::Ctl0,
            "MISC",
            "INIT_SIGNALS_ERROR",
            "INITSIGNALSERROR",
            &["DISABLE", "ENABLE"],
        ),
        (
            Reg::Ctl0,
            "MISC",
            "SELECTMAP_ABORT",
            "SELECTMAPABORT",
            &["DISABLE", "ENABLE"],
        ),
        (Reg::Ctl0, "MISC", "PERSIST", "PERSIST", &["NO", "CTLREG"]),
        // CTL1
        (
            Reg::Ctl1,
            "MISC",
            "ICAP_ENCRYPTION",
            "ICAP_ENCRYPTION",
            &["DISABLE", "ENABLE"],
        ),
        (
            Reg::Ctl1,
            "MISC",
            "DIS_VGG_REG",
            "DIS_VGG_REG",
            &["NO", "YES"],
        ),
        (
            Reg::Ctl1,
            "MISC",
            "ENABLE_VGG_CLAMP",
            "ENABLE_VGG_CLAMP",
            &["NO", "YES"],
        ),
        (
            Reg::Ctl1,
            "MISC",
            "MODE_PIN_TEST",
            "MODEPINTEST",
            &["DISABLE", "TEST0", "TEST1"],
        ),
        // BSPI
        (
            Reg::Bspi,
            "MISC",
            "BPI_SYNC_MODE",
            "BPI_SYNC_MODE",
            &["DISABLE", "TYPE1", "TYPE2"],
        ),
        (
            Reg::Bspi,
            "MISC",
            "SPI_BUSWIDTH",
            "SPI_BUSWIDTH",
            &["1", "2", "4"],
        ),
        // WBSTAR
        (
            Reg::WbStar,
            "MISC",
            "REVISION_SELECT_TRISTATE",
            "REVISIONSELECT_TRISTATE",
            &["DISABLE", "ENABLE"],
        ),
    ] {
        if edev.grids.first().unwrap().has_ps
            && matches!(
                attr,
                "SELECTMAP_ABORT"
                    | "CONFIG_RATE"
                    | "BPI_PAGE_SIZE"
                    | "BPI_1ST_READ_CYCLE"
                    | "BPI_SYNC_MODE"
                    | "SPI_32BIT_ADDR"
                    | "SPI_BUSWIDTH"
                    | "REVISION_SELECT_TRISTATE"
            )
        {
            continue;
        }
        for &val in vals {
            let extras = vec![ExtraFeature::new(
                ExtraFeatureKind::Reg(reg),
                match reg {
                    Reg::Cor0 => "REG.COR",
                    Reg::Cor1 => "REG.COR1",
                    Reg::Ctl0 => "REG.CTL",
                    Reg::Ctl1 => "REG.CTL1",
                    Reg::Bspi => "REG.BSPI",
                    Reg::WbStar => "REG.WBSTAR",
                    _ => unreachable!(),
                },
                bel,
                attr,
                val,
            )];
            match attr {
                "MATCH_CYCLE" => {
                    fuzz_one_extras!(ctx, attr, val, [
                        (global_mutex "GLOBAL_DCI", "NO")
                    ], [
                        (global_opt opt, val)
                    ], extras);
                }
                "POST_CRC_EN" => {
                    fuzz_one_extras!(ctx, attr, val, [
                        (global_opt "ENCRYPT", "NO"),
                        (global_opt "GLUTMASK_B", "0")
                    ], [
                        (global_opt opt, val)
                    ], extras);
                }
                "POST_CRC_CLK" => {
                    fuzz_one_extras!(ctx, attr, val, [
                        (no_global_opt "POST_CRC_FREQ")
                    ], [
                        (global_opt opt, val)
                    ], extras);
                }
                "BPI_1ST_READ_CYCLE" => {
                    fuzz_one_extras!(ctx, attr, val, [
                        (global_opt "BPI_PAGE_SIZE", "8")
                    ], [
                        (global_opt opt, val)
                    ], extras);
                }
                "BPI_SYNC_MODE" | "SPI_32BIT_ADDR" | "SPI_BUSWIDTH" => {
                    fuzz_one_extras!(ctx, attr, val, [
                        (global_opt "SPI_OPCODE", "0x12")
                    ], [
                        (global_opt opt, val)
                    ], extras);
                }
                "MODE_PIN_TEST" => {
                    fuzz_one_extras!(ctx, attr, val, [
                        (no_global_opt "EN_VTEST"),
                        (no_global_opt "VGG_TEST")
                    ], [
                        (global_opt opt, val)
                    ], extras);
                }
                "CONFIG_FALLBACK" => {
                    fuzz_one_extras!(ctx, attr, val, [
                        (no_global_opt "NEXT_CONFIG_REBOOT")
                    ], [
                        (global_opt opt, val)
                    ], extras);
                }
                _ => {
                    fuzz_one_extras!(ctx, attr, val, [], [
                        (global_opt opt, val)
                    ], extras);
                }
            }
        }
    }

    if edev.grids.first().unwrap().regs != 1 {
        let extras = vec![
            ExtraFeature::new(
                ExtraFeatureKind::Reg(Reg::Ctl0),
                "REG.CTL",
                "MISC",
                "ENCRYPT",
                "YES",
            ),
            ExtraFeature::new(
                ExtraFeatureKind::Reg(Reg::Ctl1),
                "REG.CTL1",
                "MISC",
                "ENCRYPT",
                "YES",
            ),
        ];
        fuzz_one_extras!(ctx, "ENCRYPT", "YES", [
                (no_global_opt "VGG_SEL"),
                (no_global_opt "VGG_POS_GAIN_SEL"),
                (no_global_opt "VGG_NEG_GAIN_SEL")
            ], [
                (global_opt "ENCRYPT", "YES")
            ], extras);
    }

    for (opt, width) in [
        ("VGG_SEL", 5),
        ("VGG_NEG_GAIN_SEL", 5),
        ("VGG_POS_GAIN_SEL", 1),
    ] {
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::Reg(Reg::Ctl1),
            "REG.CTL1",
            "MISC",
            opt,
            "",
        )];
        fuzz_multi_extras!(ctx, opt, "", width, [], (global_bin opt), extras);
    }
    if !edev.grids.first().unwrap().has_ps {
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::Reg(Reg::Bspi),
            "REG.BSPI",
            "MISC",
            "SPI_OPCODE",
            "",
        )];
        fuzz_multi_extras!(ctx, "SPI_OPCODE", "", 8, [
            (global_opt "BPI_SYNC_MODE", "TYPE1")
        ], (global_hex_prefix "SPI_OPCODE"), extras);
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::Reg(Reg::WbStar),
            "REG.WBSTAR",
            "MISC",
            "NEXT_CONFIG_ADDR",
            "",
        )];
        fuzz_multi_extras!(ctx, "NEXT_CONFIG_ADDR", "", 29, [
            (global_opt "NEXT_CONFIG_REBOOT", "DISABLE")
        ], (global_hex_prefix "NEXT_CONFIG_ADDR"), extras);
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::Reg(Reg::WbStar),
            "REG.WBSTAR",
            "MISC",
            "REVISION_SELECT",
            "",
        )];
        fuzz_multi_extras!(ctx, "REVISION_SELECT", "", 2, [
            (global_opt "NEXT_CONFIG_REBOOT", "DISABLE")
        ], (global_bin "REVISIONSELECT"), extras);
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::Reg(Reg::Timer),
            "REG.TIMER",
            "MISC",
            "TIMER_CFG",
            "1",
        )];
        fuzz_one_extras!(ctx, "TIMER_CFG", "1", [
            (no_global_opt "TIMER_USR")
        ], [(global_opt "TIMER_CFG", "0")], extras);
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::Reg(Reg::Timer),
            "REG.TIMER",
            "MISC",
            "TIMER_USR",
            "1",
        )];
        fuzz_one_extras!(ctx, "TIMER_USR", "1", [
            (no_global_opt "TIMER_CFG")
        ], [(global_opt "TIMER_USR", "0")], extras);
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::Reg(Reg::Timer),
            "REG.TIMER",
            "MISC",
            "TIMER",
            "",
        )];
        fuzz_multi_extras!(ctx, "TIMER", "", 24, [
            (no_global_opt "TIMER_USR")
        ], (global_hex "TIMER_CFG"), extras);
    }

    for (tile, reg, attr, width, anchor, anchor_val) in [
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
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::Reg(reg),
            tile,
            "MISC",
            attr,
            "",
        )];
        fuzz_multi_extras!(ctx, attr, "", width, [
            (global_opt anchor, anchor_val)
        ], (global_bin attr), extras);
    }

    if let Some(ctx) = FuzzCtx::try_new(session, backend, "XADC", "XADC", TileBits::MainAuto) {
        fuzz_one_extras!(ctx, "ENABLE", "1", [], [(mode "XADC")], vec![
            ExtraFeature::new(
                ExtraFeatureKind::HclkPair(0, 0),
                "HCLK",
                "HCLK",
                "DRP_MASK_ABOVE_L",
                "XADC"
            ),
        ]);
        fuzz_inv!(ctx, "DCLK", [(mode "XADC")]);
        fuzz_inv!(ctx, "CONVSTCLK", [(mode "XADC")]);
        for i in 0x40..0x60 {
            fuzz_multi_attr_hex!(ctx, format!("INIT_{i:02X}"), 16, [
                (global_mutex "XADC", "XADC"),
                (mode "XADC")
            ]);
        }
        for attr in [
            "SYSMON_TEST_A",
            "SYSMON_TEST_B",
            "SYSMON_TEST_C",
            "SYSMON_TEST_D",
            "SYSMON_TEST_E",
        ] {
            fuzz_multi_attr_hex!(ctx, attr, 16, [
                (global_mutex "XADC", "XADC"),
                (mode "XADC")
            ]);
        }
        let ctx = FuzzCtx::new_fake_tile(session, backend, "NULL", "NULL", TileBits::Null);
        for (attr, vals) in [
            ("JTAG_XADC", &["ENABLE", "DISABLE", "STATUSONLY"][..]),
            ("XADCPOWERDOWN", &["ENABLE", "DISABLE"][..]),
            ("XADCENHANCEDLINEARITY", &["ON", "OFF"][..]),
        ] {
            for &val in vals {
                fuzz_one_extras!(ctx, attr, val, [
                    (global_mutex "XADC", "OPT")
                ], [
                    (global_opt attr, val)
                ], vec![
                    ExtraFeature::new(
                        ExtraFeatureKind::AllXadc,
                        "XADC",
                        "XADC",
                        attr,
                        val,
                    ),
                ]);
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(ref edev) = ctx.edev else {
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
        let bel = &format!("BSCAN{i}");
        ctx.collect_bit(tile, bel, "ENABLE", "1");
        let item = ctx.extract_enum_bool_wide(tile, bel, "DISABLE_JTAG", "FALSE", "TRUE");
        ctx.tiledb
            .insert(tile, "BSCAN_COMMON", "DISABLE_JTAG", item);
    }
    {
        let bel = "BSCAN_COMMON";
        ctx.collect_bitvec(tile, bel, "USERID", "");
    }
    if edev.grids.len() == 1 && !edev.grids.first().unwrap().has_ps {
        for bel in ["ICAP0", "ICAP1"] {
            ctx.collect_bit_wide(tile, bel, "ENABLE", "1");
            // ???
            ctx.state
                .get_diff(tile, bel, "ICAP_AUTO_SWITCH", "DISABLE")
                .assert_empty();
            ctx.state
                .get_diff(tile, bel, "ICAP_AUTO_SWITCH", "ENABLE")
                .assert_empty();
        }

        let item0 = ctx.extract_enum(tile, "ICAP0", "ICAP_WIDTH", &["X8", "X16", "X32"]);
        let item1 = ctx.extract_enum(tile, "ICAP1", "ICAP_WIDTH", &["X8", "X16", "X32"]);
        assert_eq!(item0, item1);
        ctx.tiledb.insert(tile, "ICAP_COMMON", "ICAP_WIDTH", item0);
    }

    {
        let bel = "STARTUP";
        ctx.collect_enum_bool(tile, bel, "GSR_SYNC", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "GTS_SYNC", "NO", "YES");
        let item0 = ctx.extract_bit_wide(tile, bel, "PIN.GSR", "1");
        let item1 = ctx.extract_bit_wide(tile, bel, "PIN.GTS", "1");
        assert_eq!(item0, item1);
        ctx.tiledb.insert(tile, bel, "GTS_GSR_ENABLE", item0);
        ctx.collect_enum_bool_wide(tile, bel, "PROG_USR", "FALSE", "TRUE");
        let item = ctx.extract_bit_wide(tile, bel, "PIN.USRCCLKO", "1");
        ctx.tiledb.insert(tile, bel, "USRCCLK_ENABLE", item);
        if edev.grids.first().unwrap().regs > 1 {
            let item = ctx.extract_bit_wide(tile, bel, "PIN.KEYCLEARB", "1");
            ctx.tiledb.insert(tile, bel, "KEY_CLEAR_ENABLE", item);
        }
    }
    {
        let bel = "DCIRESET";
        ctx.collect_bit_wide(tile, bel, "ENABLE", "1");
    }
    if edev.grids.len() == 1 {
        let bel = "CFG_IO_ACCESS";
        ctx.collect_bit_wide(tile, bel, "ENABLE", "1");
        ctx.state
            .get_diff(tile, bel, "TDO", "UNCONNECTED")
            .assert_empty();
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
        if edev.grids.len() == 1 {
            ctx.collect_enum(tile, bel, "STARTUPCLK", &["CCLK", "USERCLK", "JTAGCLK"]);
        }
        if !edev.grids.first().unwrap().has_ps {
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
        if edev.grids.len() == 1 {
            ctx.collect_enum_bool(tile, bel, "ONESHOT", "FALSE", "TRUE");
        }
        ctx.tiledb.insert(
            tile,
            bel,
            "EXTMASTERCCLK_EN",
            TileItem::from_bit(TileBit::new(0, 0, 26), false),
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "EXTMASTERCCLK_DIV",
            TileItem {
                bits: vec![TileBit::new(0, 0, 21), TileBit::new(0, 0, 22)],
                kind: TileItemKind::Enum {
                    values: [
                        ("8".to_string(), bitvec![0, 0]),
                        ("4".to_string(), bitvec![1, 0]),
                        ("2".to_string(), bitvec![0, 1]),
                        ("1".to_string(), bitvec![1, 1]),
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

        if !edev.grids.first().unwrap().has_ps {
            ctx.collect_enum(tile, bel, "BPI_PAGE_SIZE", &["1", "4", "8"]);
            ctx.collect_enum(tile, bel, "BPI_1ST_READ_CYCLE", &["1", "2", "3", "4"]);
        }
        ctx.collect_enum(tile, bel, "POST_CRC_CLK", &["CFG_CLK", "INTERNAL"]);
        let mut diffs = vec![];
        for val in ["1", "2", "3", "6", "13", "25", "50"] {
            let mut diff = ctx.state.get_diff(tile, bel, "POST_CRC_FREQ", val);
            diff.apply_enum_diff(
                ctx.tiledb.item(tile, bel, "POST_CRC_CLK"),
                "INTERNAL",
                "CFG_CLK",
            );
            diffs.push((val, diff));
        }
        ctx.tiledb.insert(
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
        ctx.collect_enum_bool(tile, bel, "XADC_PARTIAL_RECONFIG", "DISABLE", "ENABLE");
        ctx.collect_enum_bool(tile, bel, "TRIM_BITSTREAM", "DISABLE", "ENABLE");
        ctx.tiledb.insert(
            tile,
            bel,
            "PERSIST_DEASSERT_AT_DESYNC",
            TileItem::from_bit(TileBit::new(0, 0, 17), false),
        );
        let item = ctx.extract_bit(tile, "CFG_IO_ACCESS", "ENABLE", "1");
        let item2 = xlat_bit(
            !ctx.state
                .get_diff(tile, "CFG_IO_ACCESS", "TDO", "UNCONNECTED"),
        );
        assert_eq!(item, item2);
        ctx.tiledb.insert(tile, "MISC", "CFG_IO_ACCESS_TDO", item);
        ctx.tiledb.insert(
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
        if !edev.grids.first().unwrap().has_ps {
            ctx.collect_enum_bool(tile, bel, "SELECTMAP_ABORT", "DISABLE", "ENABLE");
        }
        ctx.collect_enum(tile, bel, "ENCRYPT_KEY_SELECT", &["BBRAM", "EFUSE"]);
        ctx.collect_enum_bool(tile, bel, "SEC_ALL", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "SEC_ERROR", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "SEC_STATUS", "NO", "YES");
        if edev.grids.first().unwrap().regs > 1 {
            ctx.collect_bit(tile, bel, "ENCRYPT", "YES");
        }
        ctx.collect_enum_bool(tile, bel, "PERSIST", "NO", "CTLREG");
        ctx.tiledb.insert(
            tile,
            bel,
            "ICAP_SELECT",
            TileItem {
                bits: vec![TileBit::new(0, 0, 30)],
                kind: TileItemKind::Enum {
                    values: [
                        ("TOP".to_string(), bitvec![0]),
                        ("BOTTOM".to_string(), bitvec![1]),
                    ]
                    .into_iter()
                    .collect(),
                },
            },
        );
        let bel = "FRAME_ECC";
        let item = ctx.extract_bit(tile, bel, "ENABLE", "1");
        ctx.tiledb.insert(tile, "MISC", "GLUTMASK", item);
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
        if edev.grids.first().unwrap().regs > 1 {
            let mut diff = ctx.state.get_diff(tile, bel, "ENCRYPT", "YES");
            diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "VGG_POS_GAIN_SEL"), 1, 0);
            diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "VGG_NEG_GAIN_SEL"), 0xf, 0);
            diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "VGG_SEL"), 0xf, 0);
            diff.assert_empty();
        }
    }
    if !edev.grids.first().unwrap().has_ps {
        let tile = "REG.BSPI";
        let bel = "MISC";
        ctx.collect_enum(tile, bel, "SPI_BUSWIDTH", &["1", "2", "4"]);
        ctx.collect_bitvec(tile, bel, "SPI_OPCODE", "");
        let mut item =
            TileItem::from_bitvec((12..28).map(|i| TileBit::new(0, 0, i)).collect(), false);
        ctx.state
            .get_diff(tile, bel, "BPI_SYNC_MODE", "DISABLE")
            .assert_empty();
        let type1 = extract_bitvec_val(
            &item,
            &bitvec![0; 16],
            ctx.state.get_diff(tile, bel, "BPI_SYNC_MODE", "TYPE1"),
        );
        let type2 = extract_bitvec_val(
            &item,
            &bitvec![0; 16],
            ctx.state.get_diff(tile, bel, "BPI_SYNC_MODE", "TYPE2"),
        );
        item.kind = TileItemKind::Enum {
            values: [
                ("NONE".to_string(), bitvec![0; 16]),
                ("TYPE1".to_string(), type1),
                ("TYPE2".to_string(), type2),
            ]
            .into_iter()
            .collect(),
        };
        ctx.tiledb.insert(tile, bel, "BPI_SYNC_MODE", item);
    }
    if !edev.grids.first().unwrap().has_ps {
        let tile = "REG.WBSTAR";
        let bel = "MISC";
        ctx.collect_bitvec(tile, bel, "NEXT_CONFIG_ADDR", "");
        ctx.collect_bitvec(tile, bel, "REVISION_SELECT", "");
        ctx.collect_enum_bool(tile, bel, "REVISION_SELECT_TRISTATE", "DISABLE", "ENABLE");
    }
    if !edev.grids.first().unwrap().has_ps {
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

    if ctx.has_tile("XADC") {
        let tile = "XADC";
        let bel = "XADC";
        ctx.state.get_diff(tile, bel, "ENABLE", "1").assert_empty();
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

        ctx.state
            .get_diff(tile, bel, "JTAG_XADC", "ENABLE")
            .assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "JTAG_XADC", "DISABLE");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "SYSMON_TEST_E"), 7, 0);
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "JTAG_XADC", "STATUSONLY");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "SYSMON_TEST_E"), 0xc8, 0);
        diff.assert_empty();

        let mut diff = ctx.state.get_diff(tile, bel, "XADCENHANCEDLINEARITY", "ON");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "SYSMON_TEST_C"), 0x10, 0);
        diff.assert_empty();
        ctx.state
            .get_diff(tile, bel, "XADCENHANCEDLINEARITY", "OFF")
            .assert_empty();

        let mut diff = ctx.state.get_diff(tile, bel, "XADCPOWERDOWN", "ENABLE");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "INIT_42"), 0x30, 0);
        diff.assert_empty();
        ctx.state
            .get_diff(tile, bel, "XADCPOWERDOWN", "DISABLE")
            .assert_empty();

        let tile = "HCLK";
        let bel = "HCLK";
        ctx.collect_bit(tile, bel, "DRP_MASK_ABOVE_L", "XADC");
    }
}
