use bitvec::prelude::*;
use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_types::{TileBit, TileItem, TileItemKind};
use prjcombine_virtex_bitstream::Reg;
use prjcombine_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::{xlat_bit, xlat_bitvec, CollectorCtx, OcdMode},
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_multi, fuzz_multi_attr_hex, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    let ctx = FuzzCtx::new_fake_bel(session, backend, "CFG", "MISC", TileBits::Cfg);
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

    for i in 0..32 {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "CFG",
            format!("BUFGCTRL{i}"),
            TileBits::Cfg,
        );
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "BUFGCTRL")]);
        for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
            fuzz_inv!(ctx, pin, [(mode "BUFGCTRL")]);
        }
        fuzz_enum!(ctx, "PRESELECT_I0", ["FALSE", "TRUE"], [(mode "BUFGCTRL")]);
        fuzz_enum!(ctx, "PRESELECT_I1", ["FALSE", "TRUE"], [(mode "BUFGCTRL")]);
        fuzz_enum!(ctx, "CREATE_EDGE", ["FALSE", "TRUE"], [(mode "BUFGCTRL")]);
        fuzz_enum!(ctx, "INIT_OUT", ["0", "1"], [(mode "BUFGCTRL")]);

        for j in 0..2 {
            for val in ["CKINT0", "CKINT1"] {
                fuzz_one!(ctx, format!("MUX.I{j}"), val, [
                    (mutex format!("MUX.I{j}"), val)
                ], [
                    (pip (pin val), (pin format!("I{j}MUX")))
                ]);
            }
            fuzz_one!(ctx, format!("MUX.I{j}"), "MUXBUS", [
                (mutex format!("MUX.I{j}"), "MUXBUS")
            ], [
                (pip (pin format!("MUXBUS{j}")), (pin format!("I{j}MUX")))
            ]);
            for k in 0..16 {
                let kk = if i < 16 { k } else { k + 16 };
                let obel = BelId::from_idx(kk);
                let val = format!("GFB{k}");
                fuzz_one!(ctx, format!("MUX.I{j}"), val.clone(), [
                    (mutex format!("MUX.I{j}"), val)
                ], [
                    (pip (bel_pin obel, "GFB"), (pin format!("I{j}MUX")))
                ]);
            }
            for k in 0..5 {
                for lr in ['L', 'R'] {
                    let val = format!("MGT_{lr}{k}");
                    let pin = format!("MGT_O_{lr}{k}");
                    let obel = BelId::from_idx(50 + i / 16);
                    fuzz_one!(ctx, format!("MUX.I{j}"), &val, [
                        (mutex format!("MUX.I{j}"), &val)
                    ], [
                        (pip (bel_pin obel, pin), (pin format!("I{j}MUX")))
                    ]);
                }
            }
        }
        fuzz_one!(ctx, "I0_FABRIC_OUT", "1", [
        ], [
            (pin_pips "I0MUX")
        ]);
        fuzz_one!(ctx, "I1_FABRIC_OUT", "1", [
        ], [
            (pin_pips "I1MUX")
        ]);
    }
    for i in 0..4 {
        let ctx = FuzzCtx::new(session, backend, "CFG", format!("BSCAN{i}"), TileBits::Cfg);
        fuzz_one!(ctx, "ENABLE", "1", [], [(mode "BSCAN")]);
    }
    let ctx = FuzzCtx::new_fake_bel(session, backend, "CFG", "BSCAN_COMMON", TileBits::Cfg);
    fuzz_multi!(ctx, "USERID", "", 32, [], (global_hex_prefix "USERID"));
    for i in 0..2 {
        let ctx = FuzzCtx::new(session, backend, "CFG", format!("ICAP{i}"), TileBits::Cfg);
        fuzz_one!(ctx, "ENABLE", "1", [], [(mode "ICAP")]);
        fuzz_enum!(ctx, "ICAP_WIDTH", ["X8", "X16", "X32"], [
            (mode "ICAP"),
            (global_mutex_site "ICAP")
        ]);
    }
    {
        let ctx = FuzzCtx::new(session, backend, "CFG", "PMV", TileBits::Null);
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
        let ctx = FuzzCtx::new(session, backend, "CFG", "STARTUP", TileBits::Cfg);
        fuzz_one!(ctx, "PIN.GTS", "1", [(mode "STARTUP"), (nopin "GSR")], [(pin "GTS")]);
        fuzz_one!(ctx, "PIN.GSR", "1", [(mode "STARTUP"), (nopin "GTS")], [(pin "GSR")]);
        fuzz_one!(ctx, "PIN.USRCCLKO", "1", [(mode "STARTUP")], [(pin "USRCCLKO")]);
        for attr in ["GSR_SYNC", "GTS_SYNC"] {
            for val in ["YES", "NO"] {
                fuzz_one!(ctx, attr, val, [], [(global_opt attr, val)]);
            }
        }
    }
    {
        let ctx = FuzzCtx::new(session, backend, "CFG", "JTAGPPC", TileBits::Null);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "JTAGPPC")]);
        let ctx = FuzzCtx::new(session, backend, "CFG", "JTAGPPC", TileBits::Cfg);
        fuzz_enum!(ctx, "NUM_PPC", ["0", "1", "2", "3", "4"], [(mode "JTAGPPC")]);
    }
    {
        let ctx = FuzzCtx::new(session, backend, "CFG", "FRAME_ECC", TileBits::Null);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "FRAME_ECC")]);
    }
    {
        let ctx = FuzzCtx::new(session, backend, "CFG", "DCIRESET", TileBits::Cfg);
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
        let ctx = FuzzCtx::new(session, backend, "CFG", "KEY_CLEAR", TileBits::Null);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "KEY_CLEAR")]);
    }
    {
        let ctx = FuzzCtx::new(session, backend, "CFG", "EFUSE_USR", TileBits::Null);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "EFUSE_USR")]);
    }
    {
        let ctx = FuzzCtx::new(session, backend, "CFG", "SYSMON", TileBits::Null);
        fuzz_one_extras!(ctx, "ENABLE", "1", [], [(mode "SYSMON")], vec![
            ExtraFeature::new(ExtraFeatureKind::Hclk(0, 20), "HCLK_IOI_TOPCEN", "SYSMON", "ENABLE", "1"),
        ]);
        let ctx = FuzzCtx::new(session, backend, "CFG", "SYSMON", TileBits::Cfg);
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
            (attr "SYSMON_TEST_A", "")
        ], [
            (global_opt "JTAG_SYSMON", "DISABLE")
        ]);
    }

    for bel in ["BUFG_MGTCLK_B", "BUFG_MGTCLK_T"] {
        let ctx = FuzzCtx::new(session, backend, "CFG", bel, TileBits::Cfg);
        for i in 0..5 {
            for lr in ['L', 'R'] {
                if lr == 'L' && edev.col_lgt.is_none() {
                    continue;
                }
                fuzz_one!(ctx, format!("BUF.MGT_{lr}{i}"), "1", [], [
                    (pip (pin format!("MGT_I_{lr}{i}")), (pin format!("MGT_O_{lr}{i}")))
                ]);
            }
        }
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
        "2", "6", "9", "13", "17", "20", "24", "27", "31", "35", "38", "42", "46", "49", "53",
        "56", "60",
    ] {
        fuzz_one!(ctx, "CONFIG_RATE", val, [], [(global_opt "CONFIGRATE", val)]);
    }
    for val in ["DISABLE", "ENABLE"] {
        fuzz_one!(ctx, "CRC", val, [], [(global_opt "CRC", val)]);
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
        fuzz_one!(ctx, "POST_CRC_EN", val, [], [(global_opt "POST_CRC_EN", val)]);
        fuzz_one!(ctx, "POST_CRC_NO_PIN", val, [], [(global_opt "POST_CRC_NO_PIN", val)]);
        fuzz_one!(ctx, "POST_CRC_RECONFIG", val, [], [(global_opt "POST_CRC_RECONFIG", val)]);
        fuzz_one!(ctx, "RETAIN_CONFIG_STATUS", val, [], [(global_opt "RETAINCONFIGSTATUS", val)]);
    }
    for val in ["0", "1"] {
        fuzz_one!(ctx, "POST_CRC_SEL", val, [], [(global_opt "POST_CRC_SEL", val)]);
    }

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.CTL",
        "MISC",
        TileBits::Reg(Reg::Ctl0),
    );
    // persist not fuzzed — too much effort
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "ENCRYPT", val, [
            (global_opt "CONFIGFALLBACK", "DISABLE")
        ], [
            (global_opt "ENCRYPT", val)
        ]);
    }
    for val in ["NONE", "LEVEL1", "LEVEL2"] {
        fuzz_one!(ctx, "SECURITY", val, [], [(global_opt "SECURITY", val)]);
    }
    for val in ["BBRAM", "EFUSE"] {
        fuzz_one!(ctx, "ENCRYPT_KEY_SELECT", val, [], [(global_opt "ENCRYPTKEYSELECT", val)]);
    }
    for (attr, opt) in [
        ("OVERTEMP_POWERDOWN", "OVERTEMPPOWERDOWN"),
        ("CONFIG_FALLBACK", "CONFIGFALLBACK"),
        ("SELECTMAP_ABORT", "SELECTMAPABORT"),
    ] {
        for val in ["DISABLE", "ENABLE"] {
            fuzz_one!(ctx, attr, val, [], [(global_opt opt, val)]);
        }
    }
    for val in ["0", "1"] {
        fuzz_one!(ctx, "GLUTMASK", val, [], [(global_opt "GLUTMASK_B", val)]);
    }
    for opt in ["VBG_SEL", "VBG_DLL_SEL", "VGG_SEL"] {
        fuzz_multi!(ctx, opt, "", 5, [], (global_bin opt));
    }

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
        "REG.TESTMODE",
        "MISC",
        TileBits::RegPresent(Reg::Testmode),
    );
    fuzz_one!(ctx, "DD_OVERRIDE", "YES", [], [(global_opt "DD_OVERRIDE", "YES")]);
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

    for bel in ["BUFG_MGTCLK_B", "BUFG_MGTCLK_T"] {
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
                kind: TileItemKind::BitVec { invert: bitvec![0] },
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
                    ("TOP".to_string(), bitvec![0]),
                    ("BOTTOM".to_string(), bitvec![1]),
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
