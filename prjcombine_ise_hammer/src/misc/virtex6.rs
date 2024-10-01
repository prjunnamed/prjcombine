use bitvec::prelude::*;
use prjcombine_hammer::Session;
use prjcombine_types::{TileItem, TileItemKind};
use prjcombine_virtex_bitstream::Reg;

use crate::{
    backend::{FeatureBit, IseBackend},
    diff::{xlat_bit, xlat_bitvec, xlat_enum_ocd, CollectorCtx, OcdMode},
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_multi, fuzz_multi_attr_hex, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ctx = FuzzCtx::new_fake_bel(session, backend, "CFG", "MISC", TileBits::MainAuto);
    for attr in ["CCLKPIN", "DONEPIN", "PROGPIN", "INITPIN"] {
        for val in ["PULLUP", "PULLNONE"] {
            fuzz_one!(ctx, attr, val, [], [(global_opt attr, val)]);
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
            fuzz_one!(ctx, attr, val, [], [(global_opt attr, val)]);
        }
    }

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
    let ctx = FuzzCtx::new_fake_bel(session, backend, "CFG", "BSCAN_COMMON", TileBits::MainAuto);
    fuzz_multi!(ctx, "USERID", "", 32, [], (global_hex_prefix "USERID"));

    {
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

    for i in 0..2 {
        let ctx = FuzzCtx::new(session, backend, "CFG", format!("PMV{i}"), TileBits::Null);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "PMV")]);
    }

    {
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
        let ctx = FuzzCtx::new(session, backend, "CFG", "STARTUP", TileBits::MainAuto);
        fuzz_one!(ctx, "PIN.GTS", "1", [(mode "STARTUP"), (nopin "GSR")], [(pin "GTS")]);
        fuzz_one!(ctx, "PIN.GSR", "1", [(mode "STARTUP"), (nopin "GTS")], [(pin "GSR")]);
        fuzz_one!(ctx, "PIN.USRCCLKO", "1", [(mode "STARTUP")], [(pin "USRCCLKO")]);
        fuzz_one!(ctx, "PIN.KEYCLEARB", "1", [
            (mode "STARTUP"),
            (global_opt "ENCRYPT", "YES")
        ], [
            (pin "KEYCLEARB")
        ]);
        for attr in ["GSR_SYNC", "GTS_SYNC"] {
            for val in ["YES", "NO"] {
                fuzz_one!(ctx, attr, val, [], [(global_opt attr, val)]);
            }
        }
        fuzz_enum!(ctx, "PROG_USR", ["FALSE", "TRUE"], [(mode "STARTUP")]);
    }

    {
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

    {
        let ctx = FuzzCtx::new(session, backend, "CFG", "USR_ACCESS", TileBits::Null);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "USR_ACCESS")]);
    }

    {
        let ctx = FuzzCtx::new(session, backend, "CFG", "EFUSE_USR", TileBits::Null);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "EFUSE_USR")]);
    }

    {
        let ctx = FuzzCtx::new(session, backend, "CFG", "DNA_PORT", TileBits::MainAuto);
        fuzz_one!(ctx, "ENABLE", "1", [], [(mode "DNA_PORT")]);
    }

    {
        let ctx = FuzzCtx::new(session, backend, "CFG", "CFG_IO_ACCESS", TileBits::MainAuto);
        fuzz_one!(ctx, "ENABLE", "1", [], [(mode "CFG_IO_ACCESS")]);
    }

    {
        let ctx = FuzzCtx::new(session, backend, "CFG", "SYSMON", TileBits::Null);
        fuzz_one_extras!(ctx, "ENABLE", "1", [], [(mode "SYSMON")], vec![
            ExtraFeature::new(ExtraFeatureKind::Hclk(0, 20), "HCLK", "HCLK", "DRP_MASK_SYSMON", "1"),
        ]);
        let ctx = FuzzCtx::new(session, backend, "CFG", "SYSMON", TileBits::MainAuto);
        fuzz_inv!(ctx, "DCLK", [(mode "SYSMON")]);
        fuzz_inv!(ctx, "CONVSTCLK", [(mode "SYSMON")]);
        for i in 0x40..0x58 {
            fuzz_multi_attr_hex!(ctx, format!("INIT_{i:02X}"), 16, [(mode "SYSMON")]);
        }
        for attr in [
            "SYSMON_TEST_A",
            "SYSMON_TEST_B",
            "SYSMON_TEST_C",
            "SYSMON_TEST_D",
            "SYSMON_TEST_E",
        ] {
            fuzz_multi_attr_hex!(ctx, attr, 16, [(mode "SYSMON")]);
        }
        fuzz_one!(ctx, "JTAG_SYSMON", "DISABLE", [
            (attr "SYSMON_TEST_E", "")
        ], [
            (global_opt "JTAG_SYSMON", "DISABLE")
        ]);
    }

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.COR",
        "STARTUP",
        TileBits::Reg(Reg::Cor0),
    );
    for val in ["1", "2", "3", "4", "5", "6", "DONE", "KEEP"] {
        fuzz_one!(ctx, "GWE_CYCLE", val, [], [(global_opt "GWE_CYCLE", val)]);
        fuzz_one!(ctx, "GTS_CYCLE", val, [], [(global_opt "GTS_CYCLE", val)]);
    }
    for val in ["1", "2", "3", "4", "5", "6", "KEEP"] {
        fuzz_one!(ctx, "DONE_CYCLE", val, [], [(global_opt "DONE_CYCLE", val)]);
    }
    for val in ["0", "1", "2", "3", "4", "5", "6", "NOWAIT"] {
        fuzz_one!(ctx, "LCK_CYCLE", val, [], [(global_opt "LCK_CYCLE", val)]);
        fuzz_one!(ctx, "MATCH_CYCLE", val, [(global_mutex "GLOBAL_DCI", "NO")], [(global_opt "MATCH_CYCLE", val)]);
    }
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "DRIVE_DONE", val, [], [(global_opt "DRIVEDONE", val)]);
        fuzz_one!(ctx, "DONE_PIPE", val, [], [(global_opt "DONEPIPE", val)]);
    }
    for val in [
        "2", "4", "6", "10", "12", "16", "22", "26", "33", "40", "50", "66",
    ] {
        fuzz_one!(ctx, "CONFIG_RATE", val, [], [(global_opt "CONFIGRATE", val)]);
    }
    for val in ["DISABLE", "ENABLE"] {
        fuzz_one!(ctx, "DONE_SIGNALS_POWERDOWN", val, [], [(global_opt "DONESIGNALSPOWERDOWN", val)]);
    }

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.COR1",
        "MISC",
        TileBits::Reg(Reg::Cor1),
    );
    for val in ["1", "4", "8"] {
        fuzz_one!(ctx, "BPI_PAGE_SIZE", val, [], [(global_opt "BPI_PAGE_SIZE", val)]);
    }
    for val in ["1", "2", "3", "4"] {
        fuzz_one!(ctx, "BPI_1ST_READ_CYCLE", val, [
            (global_opt "BPI_PAGE_SIZE", "8")
        ], [
            (global_opt "BPI_1ST_READ_CYCLE", val)
        ]);
    }
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "POST_CRC_EN", val, [
            (global_opt "GLUTMASK_B", "0")
        ], [
            (global_opt "POST_CRC_EN", val)
        ]);
        fuzz_one!(ctx, "POST_CRC_RECONFIG", val, [], [(global_opt "POST_CRC_RECONFIG", val)]);
        fuzz_one!(ctx, "POST_CRC_KEEP", val, [], [(global_opt "POST_CRC_KEEP", val)]);
        fuzz_one!(ctx, "POST_CRC_CORRECT", val, [], [(global_opt "POST_CRC_CORRECT", val)]);
    }
    for opt in ["POST_CRC_SEL", "FUSE_NO_CDR"] {
        for val in ["0", "1"] {
            fuzz_one!(ctx, opt, val, [], [(global_opt opt, val)]);
        }
    }
    for val in ["1", "2", "3", "6", "13", "25", "50"] {
        fuzz_one!(ctx, "POST_CRC_FREQ", val, [], [(global_opt "POST_CRC_FREQ", val)]);
    }
    for val in ["CFG_CLK", "INTERNAL"] {
        fuzz_one!(ctx, "POST_CRC_CLK", val, [(no_global_opt "POST_CRC_FREQ")], [(global_opt "POST_CRC_CLK", val)]);
    }
    for val in ["DISABLE", "ENABLE"] {
        fuzz_one!(ctx, "SYSMON_PARTIAL_RECONFIG", val, [], [(global_opt "SYSMONPARTIALRECONFIG", val)]);
        fuzz_one!(ctx, "POST_CRC_INIT_FLAG", val, [], [(global_opt "POST_CRC_INIT_FLAG", val)]);
    }
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "FALLBACK_PULSE_FWE", val, [], [(global_opt "FALLBACKPULSEFWE", val)]);
    }

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.CTL",
        "MISC",
        TileBits::Reg(Reg::Ctl0),
    );
    // persist not fuzzed — too much effort
    for val in ["NONE", "LEVEL1", "LEVEL2"] {
        fuzz_one!(ctx, "SECURITY", val, [], [(global_opt "SECURITY", val)]);
    }
    for val in ["BBRAM", "EFUSE"] {
        fuzz_one!(ctx, "ENCRYPT_KEY_SELECT", val, [], [(global_opt "ENCRYPTKEYSELECT", val)]);
    }
    for (attr, opt) in [
        ("OVERTEMP_POWERDOWN", "OVERTEMPPOWERDOWN"),
        ("CONFIG_FALLBACK", "CONFIGFALLBACK"),
        ("INIT_SIGNALS_ERROR", "INITSIGNALSERROR"),
        ("SELECTMAP_ABORT", "SELECTMAPABORT"),
    ] {
        for val in ["DISABLE", "ENABLE"] {
            fuzz_one!(ctx, attr, val, [], [(global_opt opt, val)]);
        }
    }
    for val in ["0", "1"] {
        fuzz_one!(ctx, "GTS_USR_B", val, [], [(global_opt "GTS_USR_B", val)]);
    }
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "SEC_ALL", val, [], [(global_opt "SECALL", val)]);
        fuzz_one!(ctx, "SEC_ERROR", val, [], [(global_opt "SECERROR", val)]);
        fuzz_one!(ctx, "SEC_STATUS", val, [], [(global_opt "SECSTATUS", val)]);
        fuzz_one!(ctx, "ENCRYPT", val, [], [(global_opt "ENCRYPT", val)]);
    }

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.CTL1",
        "MISC",
        TileBits::Reg(Reg::Ctl1),
    );
    for (attr, opt) in [("ICAP_ENCRYPTION", "ICAP_ENCRYPTION")] {
        for val in ["DISABLE", "ENABLE"] {
            fuzz_one!(ctx, attr, val, [], [(global_opt opt, val)]);
        }
    }
    for opt in ["VGG_TEST", "EN_VTEST", "DIS_VGG_REG", "ENABLE_VGG_CLAMP"] {
        for val in ["NO", "YES"] {
            fuzz_one!(ctx, opt, val, [], [(global_opt opt, val)]);
        }
    }
    for opt in ["VGG_OPT_DRV", "VGG_V4_OPT"] {
        for val in ["0", "1"] {
            fuzz_one!(ctx, opt, val, [], [(global_opt opt, val)]);
        }
    }
    for val in ["DISABLE", "TEST0", "TEST1"] {
        fuzz_one!(ctx, "MODE_PIN_TEST", val, [
            (no_global_opt "EN_VTEST"),
            (no_global_opt "VGG_TEST")
        ], [
            (global_opt "MODEPINTEST", val)
        ]);
    }

    for (opt, width) in [("VGG_SEL", 5), ("VGG_SEL2", 5)] {
        fuzz_multi!(ctx, opt, "", width, [], (global_bin opt));
    }

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.UNK1C",
        "MISC",
        TileBits::Reg(Reg::Unk1C),
    );
    fuzz_multi!(ctx, "VBG_SEL", "", 6, [], (global_bin "VBG_SEL"));
    fuzz_multi!(ctx, "VBG_VGG_FLAST_SEL", "", 6, [], (global_bin "VBGVGGFLASTSEL"));
    fuzz_multi!(ctx, "VBG_VGG_NEG_SEL", "", 6, [], (global_bin "VBGVGGNEGSEL"));

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.TRIM",
        "MISC",
        TileBits::Reg(Reg::Trim0),
    );
    fuzz_multi!(ctx, "MPD_SEL", "", 3, [], (global_bin "MPD_SEL"));

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.TESTMODE",
        "MISC",
        TileBits::RegPresent(Reg::Testmode),
    );
    fuzz_multi!(ctx, "FUSE_SHADOW", "", 1, [], (global_bin "FUSE_SHADOW"));

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.TIMER",
        "MISC",
        TileBits::Reg(Reg::Timer),
    );
    fuzz_one!(ctx, "TIMER_CFG", "1", [
        (no_global_opt "TIMER_USR")
    ], [(global_opt "TIMER_CFG", "0")]);
    fuzz_one!(ctx, "TIMER_USR", "1", [
        (no_global_opt "TIMER_CFG")
    ], [(global_opt "TIMER_USR", "0")]);
    fuzz_multi!(ctx, "TIMER", "", 24, [
        (no_global_opt "TIMER_USR")
    ], (global_hex "TIMER_CFG"));

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "FAKE.IGNORE_CRC",
        "MISC",
        TileBits::RegPresent(Reg::FakeIgnoreCrc),
    );
    for val in ["DISABLE", "ENABLE"] {
        fuzz_one!(ctx, "CRC", val, [], [(global_opt "CRC", val)]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
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

    for bel in ["BSCAN0", "BSCAN1", "BSCAN2", "BSCAN3"] {
        ctx.collect_bit(tile, bel, "ENABLE", "1");
        let item = ctx.extract_enum_bool_wide(tile, bel, "DISABLE_JTAG", "FALSE", "TRUE");
        ctx.tiledb
            .insert(tile, "BSCAN_COMMON", "DISABLE_JTAG", item);
    }
    let bel = "BSCAN_COMMON";
    let item = xlat_bitvec(ctx.state.get_diffs(tile, bel, "USERID", ""));
    ctx.tiledb.insert(tile, bel, "USERID", item);

    for bel in ["ICAP0", "ICAP1", "DCIRESET", "DNA_PORT", "CFG_IO_ACCESS"] {
        ctx.collect_bit_wide(tile, bel, "ENABLE", "1");
    }
    for bel in ["ICAP0", "ICAP1"] {
        // ???
        ctx.state
            .get_diff(tile, bel, "ICAP_AUTO_SWITCH", "DISABLE")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "ICAP_AUTO_SWITCH", "ENABLE")
            .assert_empty();
    }

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
    let item = ctx.extract_bit_wide(tile, bel, "PIN.KEYCLEARB", "1");
    ctx.tiledb.insert(tile, bel, "KEY_CLEAR_ENABLE", item);

    let item0 = ctx.extract_enum(tile, "ICAP0", "ICAP_WIDTH", &["X8", "X16", "X32"]);
    let item1 = ctx.extract_enum(tile, "ICAP1", "ICAP_WIDTH", &["X8", "X16", "X32"]);
    assert_eq!(item0, item1);
    ctx.tiledb.insert(tile, "ICAP_COMMON", "ICAP_WIDTH", item0);

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
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "SYSMON_TEST_E"), 7, 0);
        diff.assert_empty();
    }

    {
        let tile = "HCLK";
        let bel = "HCLK";
        ctx.collect_bit(tile, bel, "DRP_MASK_SYSMON", "1");
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
            "2", "4", "6", "10", "12", "16", "22", "26", "33", "40", "50", "66",
        ],
        OcdMode::BitOrder,
    );
    ctx.collect_enum_bool(tile, bel, "DRIVE_DONE", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "DONE_PIPE", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "DONE_SIGNALS_POWERDOWN", "DISABLE", "ENABLE");
    let bel = "CAPTURE";
    ctx.collect_enum_bool(tile, bel, "ONESHOT", "FALSE", "TRUE");

    let tile = "REG.COR1";
    let bel = "MISC";
    ctx.collect_enum(tile, bel, "BPI_PAGE_SIZE", &["1", "4", "8"]);
    ctx.collect_enum(tile, bel, "BPI_1ST_READ_CYCLE", &["1", "2", "3", "4"]);
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
    ctx.collect_enum_bool(tile, bel, "FUSE_NO_CDR", "0", "1");
    ctx.collect_enum_bool(tile, bel, "POST_CRC_INIT_FLAG", "DISABLE", "ENABLE");
    ctx.collect_enum_bool(tile, bel, "SYSMON_PARTIAL_RECONFIG", "DISABLE", "ENABLE");
    ctx.collect_enum_bool(tile, bel, "FALLBACK_PULSE_FWE", "NO", "YES");
    ctx.tiledb.insert(
        tile,
        bel,
        "PERSIST_DEASSERT_AT_DESYNC",
        TileItem::from_bit(FeatureBit::new(0, 0, 17), false),
    );

    let tile = "REG.CTL";
    let bel = "MISC";
    ctx.collect_enum_bool(tile, bel, "GTS_USR_B", "0", "1");
    ctx.collect_enum(tile, bel, "SECURITY", &["NONE", "LEVEL1", "LEVEL2"]);
    ctx.collect_enum_bool(tile, bel, "OVERTEMP_POWERDOWN", "DISABLE", "ENABLE");
    ctx.collect_enum_bool(tile, bel, "CONFIG_FALLBACK", "DISABLE", "ENABLE");
    ctx.collect_enum_bool(tile, bel, "INIT_SIGNALS_ERROR", "DISABLE", "ENABLE");
    ctx.collect_enum_bool(tile, bel, "SELECTMAP_ABORT", "DISABLE", "ENABLE");
    ctx.collect_enum(tile, bel, "ENCRYPT_KEY_SELECT", &["BBRAM", "EFUSE"]);
    ctx.collect_enum_bool(tile, bel, "SEC_ALL", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "SEC_ERROR", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "SEC_STATUS", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "ENCRYPT", "NO", "YES");
    // these are too much trouble to deal with the normal way.
    ctx.tiledb.insert(
        tile,
        bel,
        "PERSIST",
        TileItem {
            bits: vec![FeatureBit::new(0, 0, 3)],
            kind: TileItemKind::BitVec { invert: bitvec![0] },
        },
    );
    ctx.tiledb.insert(
        tile,
        bel,
        "ICAP_SELECT",
        TileItem {
            bits: vec![FeatureBit::new(0, 0, 30)],
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

    let tile = "REG.CTL1";
    let bel = "MISC";
    ctx.collect_enum_bool(tile, bel, "ICAP_ENCRYPTION", "DISABLE", "ENABLE");
    ctx.collect_enum_bool(tile, bel, "VGG_TEST", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "EN_VTEST", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "DIS_VGG_REG", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "ENABLE_VGG_CLAMP", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "VGG_OPT_DRV", "0", "1");
    ctx.collect_enum_bool(tile, bel, "VGG_V4_OPT", "0", "1");
    ctx.state
        .get_diff(tile, bel, "MODE_PIN_TEST", "DISABLE")
        .assert_empty();
    let mut diff = ctx.state.get_diff(tile, bel, "MODE_PIN_TEST", "TEST0");
    diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "VGG_TEST"), true, false);
    diff.assert_empty();
    let mut diff = ctx.state.get_diff(tile, bel, "MODE_PIN_TEST", "TEST1");
    diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "EN_VTEST"), true, false);
    diff.assert_empty();
    ctx.collect_bitvec(tile, bel, "VGG_SEL", "");
    ctx.collect_bitvec(tile, bel, "VGG_SEL2", "");

    let tile = "REG.TIMER";
    let bel = "MISC";
    ctx.collect_bitvec(tile, bel, "TIMER", "");
    ctx.collect_bit(tile, bel, "TIMER_CFG", "1");
    ctx.collect_bit(tile, bel, "TIMER_USR", "1");

    let tile = "REG.TESTMODE";
    let bel = "MISC";
    let mut diff = ctx.state.get_diff(tile, bel, "FUSE_SHADOW", "");
    diff.bits.remove(&FeatureBit::new(1, 0, 0));
    ctx.tiledb.insert(tile, bel, "FUSE_SHADOW", xlat_bit(diff));

    let tile = "REG.TRIM";
    let bel = "MISC";
    ctx.collect_bitvec(tile, bel, "MPD_SEL", "");

    let tile = "REG.UNK1C";
    let bel = "MISC";
    ctx.collect_bitvec(tile, bel, "VBG_SEL", "");
    ctx.collect_bitvec(tile, bel, "VBG_VGG_FLAST_SEL", "");
    ctx.collect_bitvec(tile, bel, "VBG_VGG_NEG_SEL", "");

    let tile = "FAKE.IGNORE_CRC";
    let bel = "MISC";
    ctx.state
        .get_diff(tile, bel, "CRC", "ENABLE")
        .assert_empty();
    let diff = ctx.state.get_diff(tile, bel, "CRC", "DISABLE");
    assert_eq!(diff.bits.len(), 2);
    assert!(diff.bits[&FeatureBit::new(0, 0, 0)]);
    assert!(diff.bits[&FeatureBit::new(1, 0, 0)]);
}
