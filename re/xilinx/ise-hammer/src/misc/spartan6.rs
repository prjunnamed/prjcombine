use bitvec::prelude::*;
use prjcombine_re_collector::{OcdMode, concat_bitvec, xlat_bit};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::tiledb::{TileBit, TileItem, TileItemKind};
use prjcombine_xilinx_bitstream::Reg;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{ExtraFeature, TileBits},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_multi, fuzz_multi_attr_dec, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ExpandedDevice::Spartan6(edev) = backend.edev else {
        unreachable!()
    };
    for (tile, n) in [("LL", "BL"), ("UL", "TL"), ("LR", "BR"), ("UR", "TR")] {
        for vh in ['V', 'H'] {
            let ctx = FuzzCtx::new_fake_bel(session, backend, tile, "MISC", TileBits::MainAuto);
            fuzz_one!(ctx, format!("MISR_{vh}_ENABLE"), "1", [
                (global_opt "ENABLEMISR", "Y"),
                (global_opt "MISRRESET", "N")
            ], [
                (global_opt format!("MISR_{n}{vh}_EN"), "Y")
            ]);
            fuzz_one!(ctx, format!("MISR_{vh}_ENABLE_RESET"), "1", [
                (global_opt "ENABLEMISR", "Y"),
                (global_opt "MISRRESET", "Y")
            ], [
                (global_opt format!("MISR_{n}{vh}_EN"), "Y")
            ]);
        }
    }

    let ctx = FuzzCtx::new_fake_bel(session, backend, "LL", "MISC", TileBits::MainAuto);
    fuzz_multi!(ctx, "LEAKER_SLOPE_OPTIONS", "", 4, [], (global_dec "LEAKERSLOPEOPTIONS"));
    fuzz_multi!(ctx, "LEAKER_GAIN_OPTIONS", "", 4, [], (global_dec "LEAKERGAINOPTIONS"));
    fuzz_multi!(ctx, "VGG_SLOPE_OPTIONS", "", 4, [], (global_dec "VGGSLOPEOPTIONS"));
    fuzz_multi!(ctx, "VBG_SLOPE_OPTIONS", "", 4, [], (global_dec "VBGSLOPEOPTIONS"));
    fuzz_multi!(ctx, "VGG_TEST_OPTIONS", "", 3, [], (global_dec "VGGTESTOPTIONS"));
    fuzz_multi!(ctx, "VGG_COMP_OPTION", "", 1, [], (global_dec "VGGCOMPOPTION"));
    for val in ["PULLUP", "PULLNONE"] {
        fuzz_one!(ctx, "PROGPIN", val, [], [(global_opt "PROGPIN", val)]);
    }
    for val in ["PULLUP", "PULLNONE", "PULLDOWN"] {
        fuzz_one!(ctx, "MISO2PIN", val, [], [(global_opt "MISO2PIN", val)]);
    }
    for bel in ["OCT_CAL2", "OCT_CAL3"] {
        let ctx = FuzzCtx::new(session, backend, "LL", bel, TileBits::MainAuto);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "OCT_CALIBRATE")]);
        fuzz_enum!(ctx, "ACCESS_MODE", ["STATIC", "USER"], [(mode "OCT_CALIBRATE")]);
        fuzz_enum!(ctx, "VREF_VALUE", ["0.25", "0.5", "0.75"], [(mode "OCT_CALIBRATE")]);
    }

    let ctx = FuzzCtx::new_fake_bel(session, backend, "UL", "MISC", TileBits::MainAuto);
    for val in ["READ", "PROGRAM"] {
        fuzz_one!(ctx, "DNA_OPTIONS", val, [], [(global_opt "DNAOPTIONS", val)]);
    }
    fuzz_one!(ctx, "DNA_OPTIONS", "ANALOG_READ", [], [(global_opt "DNAOPTIONS", "ANALOGREAD")]);
    for val in ["PULLUP", "PULLNONE", "PULLDOWN"] {
        for opt in ["M2PIN", "SELECTHSPIN"] {
            fuzz_one!(ctx, opt, val, [], [(global_opt opt, val)]);
        }
    }
    let ctx = FuzzCtx::new(session, backend, "UL", "DNA_PORT", TileBits::MainAuto);
    fuzz_one!(ctx, "ENABLE", "1", [], [(mode "DNA_PORT")]);
    let ctx = FuzzCtx::new(session, backend, "UL", "PMV", TileBits::MainAuto);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "PMV")]);
    fuzz_multi_attr_dec!(ctx, "PSLEW", 4, [(mode "PMV")]);
    fuzz_multi_attr_dec!(ctx, "NSLEW", 4, [(mode "PMV")]);
    for bel in ["OCT_CAL0", "OCT_CAL4"] {
        let ctx = FuzzCtx::new(session, backend, "UL", bel, TileBits::MainAuto);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "OCT_CALIBRATE")]);
        fuzz_enum!(ctx, "ACCESS_MODE", ["STATIC", "USER"], [(mode "OCT_CALIBRATE")]);
        fuzz_enum!(ctx, "VREF_VALUE", ["0.25", "0.5", "0.75"], [(mode "OCT_CALIBRATE")]);
    }

    let ctx = FuzzCtx::new_fake_bel(session, backend, "LR", "MISC", TileBits::MainAuto);
    for val in ["PULLUP", "PULLNONE", "PULLDOWN"] {
        for opt in ["CCLK2PIN", "MOSI2PIN", "SS_BPIN"] {
            fuzz_one!(ctx, opt, val, [], [(global_opt opt, val)]);
        }
    }
    for val in ["PULLUP", "PULLNONE"] {
        fuzz_one!(ctx, "DONEPIN", val, [], [(global_opt "DONEPIN", val)]);
    }
    let ctx = FuzzCtx::new(session, backend, "LR", "OCT_CAL1", TileBits::MainAuto);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "OCT_CALIBRATE")]);
    fuzz_enum!(ctx, "ACCESS_MODE", ["STATIC", "USER"], [(mode "OCT_CALIBRATE")]);
    fuzz_enum!(ctx, "VREF_VALUE", ["0.25", "0.5", "0.75"], [(mode "OCT_CALIBRATE")]);
    let ctx = FuzzCtx::new(session, backend, "LR", "ICAP", TileBits::MainAuto);
    fuzz_one!(ctx, "ENABLE", "1", [], [(mode "ICAP")]);
    let ctx = FuzzCtx::new(session, backend, "LR", "SPI_ACCESS", TileBits::MainAuto);
    fuzz_one!(ctx, "ENABLE", "1", [], [(mode "SPI_ACCESS")]);

    let ctx = FuzzCtx::new(session, backend, "LR", "SUSPEND_SYNC", TileBits::MainAuto);
    fuzz_one!(ctx, "ENABLE", "1", [], [(mode "SUSPEND_SYNC")]);
    let ctx = FuzzCtx::new(
        session,
        backend,
        "LR",
        "POST_CRC_INTERNAL",
        TileBits::MainAuto,
    );
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "POST_CRC_INTERNAL")]);
    let ctx = FuzzCtx::new(session, backend, "LR", "STARTUP", TileBits::MainAuto);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "STARTUP")]);
    for attr in ["GTS_SYNC", "GSR_SYNC"] {
        for val in ["NO", "YES"] {
            fuzz_one!(ctx, attr, val, [(mode "STARTUP")], [(global_opt attr, val)]);
        }
    }
    fuzz_one!(ctx, "PIN.GTS", "1", [(mode "STARTUP"), (nopin "GSR")], [(pin "GTS")]);
    fuzz_one!(ctx, "PIN.GSR", "1", [(mode "STARTUP"), (nopin "GTS")], [(pin "GSR")]);
    fuzz_one!(ctx, "PIN.CFGCLK", "1", [(mode "STARTUP")], [(pin "CFGCLK")]);
    fuzz_one!(ctx, "PIN.CFGMCLK", "1", [(mode "STARTUP")], [(pin "CFGMCLK")]);
    fuzz_one!(ctx, "PIN.KEYCLEARB", "1", [(mode "STARTUP")], [(pin "KEYCLEARB")]);
    for val in ["CCLK", "USERCLK", "JTAGCLK"] {
        let extras = vec![ExtraFeature::new(
            crate::fgen::ExtraFeatureKind::Reg(Reg::Cor1),
            "REG.COR1",
            "STARTUP",
            "STARTUPCLK",
            val,
        )];
        fuzz_one_extras!(ctx, "STARTUPCLK", val, [
            (mode "STARTUP"),
            (pin "CLK")
        ], [
            (global_opt "STARTUPCLK", val)
        ], extras);
    }

    let ctx = FuzzCtx::new(session, backend, "LR", "SLAVE_SPI", TileBits::MainAuto);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "SLAVE_SPI")]);

    let ctx = FuzzCtx::new_fake_bel(session, backend, "UR", "MISC", TileBits::MainAuto);
    for val in ["PULLUP", "PULLNONE", "PULLDOWN"] {
        for opt in ["TCKPIN", "TDIPIN", "TMSPIN", "TDOPIN", "CSO2PIN"] {
            fuzz_one!(ctx, opt, val, [], [(global_opt opt, val)]);
        }
    }
    let ctx = FuzzCtx::new(session, backend, "UR", "OCT_CAL5", TileBits::MainAuto);
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "OCT_CALIBRATE")]);
    fuzz_enum!(ctx, "ACCESS_MODE", ["STATIC", "USER"], [(mode "OCT_CALIBRATE")]);
    fuzz_enum!(ctx, "VREF_VALUE", ["0.25", "0.5", "0.75"], [(mode "OCT_CALIBRATE")]);
    for i in 0..4 {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "UR",
            format!("BSCAN{i}"),
            TileBits::MainAuto,
        );
        fuzz_one!(ctx, "ENABLE", "1", [], [(mode "BSCAN")]);
        fuzz_enum!(ctx, "JTAG_TEST", ["0", "1"], [(mode "BSCAN")]);
    }
    let ctx = FuzzCtx::new_fake_bel(session, backend, "UR", "BSCAN_COMMON", TileBits::MainAuto);
    fuzz_multi!(ctx, "USERID", "", 32, [], (global_hex_prefix "USERID"));

    let mut ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.COR1",
        "STARTUP",
        TileBits::Reg(Reg::Cor1),
    );
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "DRIVE_DONE", val, [], [(global_opt "DRIVEDONE", val)]);
        fuzz_one!(ctx, "DONE_PIPE", val, [], [(global_opt "DONEPIPE", val)]);
        fuzz_one!(ctx, "DRIVE_AWAKE", val, [], [(global_opt "DRIVE_AWAKE", val)]);
    }
    for val in ["DISABLE", "ENABLE"] {
        fuzz_one!(ctx, "CRC", val, [], [(global_opt "CRC", val)]);
    }
    fuzz_multi!(ctx, "VRDSEL", "", 3, [], (global_bin "VRDSEL"));
    ctx.bel_name = "MISC".to_string();
    for val in ["0", "1"] {
        for opt in ["SEND_VGG0", "SEND_VGG1", "SEND_VGG2", "SEND_VGG3"] {
            fuzz_one!(ctx, opt, val, [], [(global_opt opt, val)]);
        }
    }
    for val in ["NO", "YES"] {
        for opt in ["VGG_SENDMAX", "VGG_ENABLE_OFFCHIP"] {
            fuzz_one!(ctx, opt, val, [], [(global_opt opt, val)]);
        }
    }

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.COR2",
        "STARTUP",
        TileBits::Reg(Reg::Cor2),
    );
    for val in ["1", "2", "3", "4", "5", "6", "DONE", "KEEP"] {
        fuzz_one!(ctx, "GWE_CYCLE", val, [], [(global_opt "GWE_CYCLE", val)]);
        fuzz_one!(ctx, "GTS_CYCLE", val, [(global_opt "LCK_CYCLE", "NOWAIT")], [(global_opt "GTS_CYCLE", val)]);
    }
    for val in ["1", "2", "3", "4", "5", "6"] {
        fuzz_one!(ctx, "DONE_CYCLE", val, [], [(global_opt "DONE_CYCLE", val)]);
    }
    for val in ["1", "2", "3", "4", "5", "6", "NOWAIT"] {
        fuzz_one!(ctx, "LCK_CYCLE", val, [(global_opt "GTS_CYCLE", "1")], [(global_opt "LCK_CYCLE", val)]);
    }
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "BPI_DIV8", val, [], [(global_opt "BPI_DIV8", val)]);
        fuzz_one!(ctx, "BPI_DIV16", val, [], [(global_opt "BPI_DIV16", val)]);
        fuzz_one!(ctx, "RESET_ON_ERR", val, [], [(global_opt "RESET_ON_ERR", val)]);
        fuzz_one!(ctx, "DISABLE_VRD_REG", val, [], [(global_opt "DISABLE_VRD_REG", val)]);
    }

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.CTL",
        "MISC",
        TileBits::Reg(Reg::Ctl0),
    );
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "GTS_USR_B", val, [], [(global_opt "GTS_USR_B", val)]);
        fuzz_one!(ctx, "MULTIBOOT_ENABLE", val, [], [(global_opt "MULTIBOOTMODE", val)]);
        if edev.grid.has_encrypt {
            fuzz_one!(ctx, "ENCRYPT", val, [(global_mutex "BRAM", "NOPE")], [(global_opt "ENCRYPT", val)]);
        }
    }
    for val in ["EFUSE", "BBRAM"] {
        fuzz_one!(ctx, "ENCRYPT_KEY_SELECT", val, [], [(global_opt "ENCRYPTKEYSELECT", val)]);
    }
    for val in ["DISABLE", "ENABLE"] {
        fuzz_one!(ctx, "POST_CRC_INIT_FLAG", val, [], [(global_opt "POST_CRC_INIT_FLAG", val)]);
    }
    // persist not fuzzed — too much effort
    for val in ["NONE", "LEVEL1", "LEVEL2", "LEVEL3"] {
        fuzz_one!(ctx, "SECURITY", val, [], [(global_opt "SECURITY", val)]);
    }

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.CCLK_FREQ",
        "STARTUP",
        TileBits::Reg(Reg::CclkFrequency),
    );
    for val in ["2", "1", "4", "6", "10", "12", "16", "22", "26"] {
        fuzz_one!(ctx, "CONFIG_RATE", val, [
            (global_opt "EXTMASTERCCLK_EN", "NO")
        ], [
            (global_opt "CONFIGRATE", val)
        ]);
    }
    for val in ["1", "2", "4", "8", "16", "32", "64", "128", "256", "512"] {
        fuzz_one!(ctx, "EXTMASTERCCLK_DIVIDE", val, [
            (global_opt "EXTMASTERCCLK_EN", "YES")
        ], [
            (global_opt "EXTMASTERCCLK_DIVIDE", val)
        ]);
    }
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "EXTMASTERCCLK_EN", val, [], [(global_opt "EXTMASTERCCLK_EN", val)]);
    }
    for val in ["0", "1", "2", "3"] {
        fuzz_one!(ctx, "CCLK_DLY", val, [], [(global_opt "CCLK_DLY", val)]);
        fuzz_one!(ctx, "CCLK_SEP", val, [], [(global_opt "CCLK_SEP", val)]);
        fuzz_one!(ctx, "CLK_SWITCH_OPT", val, [], [(global_opt "CLK_SWITCH_OPT", val)]);
    }

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.HC_OPT",
        "MISC",
        TileBits::Reg(Reg::HcOpt),
    );
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "BRAM_SKIP", val, [], [(global_opt "BRAM_SKIP", val)]);
        fuzz_one!(ctx, "TWO_ROUND", val, [], [(global_opt "TWO_ROUND", val)]);
    }
    for i in 1..16 {
        let val = format!("{i}");
        fuzz_one!(ctx, "HC_CYCLE", &val, [], [(global_opt "HC_CYCLE", &val)]);
    }

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.POWERDOWN",
        "MISC",
        TileBits::Reg(Reg::Powerdown),
    );
    for val in ["STARTUPCLK", "INTERNALCLK"] {
        fuzz_one!(ctx, "SW_CLK", val, [], [(global_opt "SW_CLK", val)]);
    }
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "EN_SUSPEND", val, [], [(global_opt "EN_SUSPEND", val)]);
        fuzz_one!(ctx, "SUSPEND_FILTER", val, [], [(global_opt "SUSPEND_FILTER", val)]);
        fuzz_one!(ctx, "EN_SW_GSR", val, [], [(global_opt "EN_SW_GSR", val)]);
        fuzz_one!(ctx, "MULTIPIN_WAKEUP", val, [], [(global_opt "MULTIPIN_WAKEUP", val)]);
    }
    for i in 1..8 {
        let val = format!("{i}");
        fuzz_one!(ctx, "WAKE_DELAY1", &val, [], [(global_opt "WAKE_DELAY1", val)]);
    }
    for i in 1..32 {
        let val = format!("{i}");
        fuzz_one!(ctx, "WAKE_DELAY2", &val, [], [(global_opt "WAKE_DELAY2", val)]);
    }

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.PU_GWE",
        "MISC",
        TileBits::Reg(Reg::PuGwe),
    );
    for val in ["1", "2", "4", "8", "16", "32", "64", "128", "256", "512"] {
        fuzz_one!(ctx, "SW_GWE_CYCLE", val, [], [(global_opt "SW_GWE_CYCLE", val)]);
    }

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.PU_GTS",
        "MISC",
        TileBits::Reg(Reg::PuGts),
    );
    for val in ["1", "2", "4", "8", "16", "32", "64", "128", "256", "512"] {
        fuzz_one!(ctx, "SW_GTS_CYCLE", val, [], [(global_opt "SW_GTS_CYCLE", val)]);
    }

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.EYE_MASK",
        "MISC",
        TileBits::Reg(Reg::EyeMask),
    );
    fuzz_multi!(ctx, "WAKEUP_MASK", "", 8, [], (global_hex_prefix "WAKEUP_MASK"));

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.MODE",
        "MISC",
        TileBits::Reg(Reg::Mode),
    );
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "NEXT_CONFIG_NEW_MODE", val, [], [(global_opt "NEXT_CONFIG_NEW_MODE", val)]);
    }
    fuzz_multi!(ctx, "NEXT_CONFIG_BOOT_MODE", "", 3, [], (global_bin "NEXT_CONFIG_BOOT_MODE"));

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.SEU_OPT",
        "MISC",
        TileBits::Reg(Reg::SeuOpt),
    );
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "POST_CRC_KEEP", val, [], [(global_opt "POST_CRC_KEEP", val)]);
    }
    for val in ["0", "1"] {
        fuzz_one!(ctx, "POST_CRC_SEL", val, [], [(global_opt "POST_CRC_SEL", val)]);
        fuzz_one!(ctx, "POST_CRC_ONESHOT", val, [
            (global_opt "POST_CRC_SEL", "0")
        ], [(global_opt "POST_CRC_ONESHOT", val)]);
    }
    for val in [
        "1", "2", "4", "6", "10", "12", "16", "22", "26", "33", "40", "50", "66",
    ] {
        fuzz_one!(ctx, "POST_CRC_FREQ", val, [], [(global_opt "POST_CRC_FREQ", val)]);
    }

    let ctx = FuzzCtx::new_fake_tile(
        session,
        backend,
        "REG.TESTMODE",
        "MISC",
        TileBits::Reg(Reg::Testmode),
    );
    for val in ["NO", "YES"] {
        fuzz_one!(ctx, "TESTMODE_EN", val, [], [(global_opt "TESTMODE_EN", val)]);
        fuzz_one!(ctx, "ICAP_BYPASS", val, [], [(global_opt "ICAP_BYPASS", val)]);
        fuzz_one!(ctx, "VGG_TEST", val, [], [(global_opt "VGG_TEST", val)]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Spartan6(edev) = ctx.edev else {
        unreachable!()
    };
    for (tile, bel) in [
        ("LL", "OCT_CAL2"),
        ("LL", "OCT_CAL3"),
        ("UL", "OCT_CAL0"),
        ("UL", "OCT_CAL4"),
        ("LR", "OCT_CAL1"),
        ("UR", "OCT_CAL5"),
    ] {
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        ctx.state
            .get_diff(tile, bel, "VREF_VALUE", "0.25")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "VREF_VALUE", "0.5")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "VREF_VALUE", "0.75")
            .assert_empty();
        ctx.collect_enum(tile, bel, "ACCESS_MODE", &["STATIC", "USER"]);
    }

    {
        let tile = "LL";
        let bel = "MISC";
        ctx.collect_bitvec(tile, bel, "LEAKER_SLOPE_OPTIONS", "");
        ctx.collect_bitvec(tile, bel, "LEAKER_GAIN_OPTIONS", "");
        ctx.collect_bitvec(tile, bel, "VGG_SLOPE_OPTIONS", "");
        ctx.collect_bitvec(tile, bel, "VBG_SLOPE_OPTIONS", "");
        ctx.collect_bitvec(tile, bel, "VGG_TEST_OPTIONS", "");
        ctx.collect_bitvec(tile, bel, "VGG_COMP_OPTION", "");
        ctx.collect_enum(tile, bel, "PROGPIN", &["PULLUP", "PULLNONE"]);
        ctx.collect_enum(tile, bel, "MISO2PIN", &["PULLUP", "PULLNONE", "PULLDOWN"]);
    }
    for tile in ["LL", "UL", "LR", "UR"] {
        let bel = "MISC";
        ctx.collect_bit(tile, bel, "MISR_H_ENABLE", "1");
        ctx.collect_bit(tile, bel, "MISR_V_ENABLE", "1");
        let mut diff = ctx.state.get_diff(tile, bel, "MISR_H_ENABLE_RESET", "1");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "MISR_H_ENABLE"), true, false);
        ctx.tiledb.insert(tile, bel, "MISR_H_RESET", xlat_bit(diff));
        let mut diff = ctx.state.get_diff(tile, bel, "MISR_V_ENABLE_RESET", "1");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "MISR_V_ENABLE"), true, false);
        ctx.tiledb.insert(tile, bel, "MISR_V_RESET", xlat_bit(diff));
    }

    {
        let tile = "UL";
        let bel = "MISC";
        ctx.collect_enum(
            tile,
            bel,
            "DNA_OPTIONS",
            &["READ", "PROGRAM", "ANALOG_READ"],
        );
        ctx.collect_enum(tile, bel, "M2PIN", &["PULLUP", "PULLNONE", "PULLDOWN"]);
        ctx.collect_enum(
            tile,
            bel,
            "SELECTHSPIN",
            &["PULLUP", "PULLNONE", "PULLDOWN"],
        );
    }
    {
        let tile = "UL";
        let bel = "DNA_PORT";
        ctx.collect_bit(tile, bel, "ENABLE", "1");
    }
    {
        let tile = "UL";
        let bel = "PMV";
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        ctx.collect_bitvec(tile, bel, "PSLEW", "");
        ctx.collect_bitvec(tile, bel, "NSLEW", "");
    }

    {
        let tile = "LR";
        let bel = "MISC";
        ctx.collect_enum(tile, bel, "CCLK2PIN", &["PULLUP", "PULLNONE", "PULLDOWN"]);
        ctx.collect_enum(tile, bel, "MOSI2PIN", &["PULLUP", "PULLNONE", "PULLDOWN"]);
        ctx.collect_enum(tile, bel, "SS_BPIN", &["PULLUP", "PULLNONE", "PULLDOWN"]);
        ctx.collect_enum(tile, bel, "DONEPIN", &["PULLUP", "PULLNONE"]);
        ctx.collect_bit(tile, "ICAP", "ENABLE", "1");
        ctx.collect_bit(tile, "SUSPEND_SYNC", "ENABLE", "1");
        ctx.collect_bit(tile, "SPI_ACCESS", "ENABLE", "1");
        ctx.state
            .get_diff(tile, "SLAVE_SPI", "PRESENT", "1")
            .assert_empty();
        ctx.state
            .get_diff(tile, "POST_CRC_INTERNAL", "PRESENT", "1")
            .assert_empty();
    }
    {
        let tile = "LR";
        let bel = "STARTUP";
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        ctx.collect_enum_bool(tile, bel, "GTS_SYNC", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "GSR_SYNC", "NO", "YES");
        ctx.collect_bit(tile, bel, "PIN.CFGCLK", "1");
        ctx.collect_bit(tile, bel, "PIN.CFGMCLK", "1");
        ctx.collect_bit(tile, bel, "PIN.KEYCLEARB", "1");
        let item = ctx.extract_bit(tile, bel, "PIN.GTS", "1");
        ctx.tiledb.insert(tile, bel, "GTS_GSR_ENABLE", item);
        let item = ctx.extract_bit(tile, bel, "PIN.GSR", "1");
        ctx.tiledb.insert(tile, bel, "GTS_GSR_ENABLE", item);
        for val in ["CCLK", "USERCLK", "JTAGCLK"] {
            ctx.state
                .get_diff(tile, bel, "STARTUPCLK", val)
                .assert_empty();
        }
    }

    {
        let tile = "UR";
        let bel = "MISC";
        ctx.collect_enum(tile, bel, "TCKPIN", &["PULLUP", "PULLNONE", "PULLDOWN"]);
        ctx.collect_enum(tile, bel, "TDIPIN", &["PULLUP", "PULLNONE", "PULLDOWN"]);
        ctx.collect_enum(tile, bel, "TMSPIN", &["PULLUP", "PULLNONE", "PULLDOWN"]);
        ctx.collect_enum(tile, bel, "TDOPIN", &["PULLUP", "PULLNONE", "PULLDOWN"]);
        ctx.collect_enum(tile, bel, "CSO2PIN", &["PULLUP", "PULLNONE", "PULLDOWN"]);
        ctx.collect_bit(tile, "BSCAN0", "ENABLE", "1");
        ctx.collect_bit(tile, "BSCAN1", "ENABLE", "1");
        ctx.collect_bit(tile, "BSCAN2", "ENABLE", "1");
        ctx.collect_bit(tile, "BSCAN3", "ENABLE", "1");
        ctx.collect_bitvec(tile, "BSCAN_COMMON", "USERID", "");
        let item = ctx.extract_enum_bool(tile, "BSCAN0", "JTAG_TEST", "0", "1");
        ctx.tiledb.insert(tile, "BSCAN_COMMON", "JTAG_TEST", item);
        for bel in ["BSCAN1", "BSCAN2", "BSCAN3"] {
            ctx.state
                .get_diff(tile, bel, "JTAG_TEST", "0")
                .assert_empty();
            ctx.state
                .get_diff(tile, bel, "JTAG_TEST", "1")
                .assert_empty();
        }
    }

    {
        let tile = "REG.COR1";
        let bel = "STARTUP";
        ctx.collect_enum(tile, bel, "STARTUPCLK", &["CCLK", "USERCLK", "JTAGCLK"]);
        ctx.collect_enum_bool(tile, bel, "DRIVE_DONE", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "DONE_PIPE", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "DRIVE_AWAKE", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "CRC", "DISABLE", "ENABLE");
        ctx.collect_bitvec(tile, bel, "VRDSEL", "");
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "VGG_SENDMAX", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "VGG_ENABLE_OFFCHIP", "NO", "YES");
        let item0 = ctx.extract_enum_bool(tile, bel, "SEND_VGG0", "0", "1");
        let item1 = ctx.extract_enum_bool(tile, bel, "SEND_VGG1", "0", "1");
        let item2 = ctx.extract_enum_bool(tile, bel, "SEND_VGG2", "0", "1");
        let item3 = ctx.extract_enum_bool(tile, bel, "SEND_VGG3", "0", "1");
        let item = concat_bitvec([item0, item1, item2, item3]);
        ctx.tiledb.insert(tile, bel, "SEND_VGG", item);

        let tile = "REG.COR2";
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
        ctx.collect_enum(tile, bel, "DONE_CYCLE", &["1", "2", "3", "4", "5", "6"]);
        ctx.collect_enum(
            tile,
            bel,
            "LCK_CYCLE",
            &["1", "2", "3", "4", "5", "6", "NOWAIT"],
        );
        ctx.collect_enum_bool(tile, bel, "BPI_DIV8", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "BPI_DIV16", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "RESET_ON_ERR", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "DISABLE_VRD_REG", "NO", "YES");
    }

    {
        let tile = "REG.CTL";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "GTS_USR_B", "NO", "YES");
        ctx.collect_enum(tile, bel, "ENCRYPT_KEY_SELECT", &["BBRAM", "EFUSE"]);
        ctx.collect_enum_bool(tile, bel, "POST_CRC_INIT_FLAG", "DISABLE", "ENABLE");
        ctx.collect_enum_bool(tile, bel, "MULTIBOOT_ENABLE", "NO", "YES");
        if edev.grid.has_encrypt {
            ctx.collect_enum_bool(tile, bel, "ENCRYPT", "NO", "YES");
        }
        ctx.collect_enum(
            tile,
            bel,
            "SECURITY",
            &["NONE", "LEVEL1", "LEVEL2", "LEVEL3"],
        );
        // too much trouble to deal with in normal ways.
        ctx.tiledb.insert(
            tile,
            bel,
            "PERSIST",
            TileItem::from_bit(TileBit::new(0, 0, 3), false),
        );
    }

    {
        let tile = "REG.CCLK_FREQ";
        let bel = "STARTUP";
        // it's just 400 / val. boring.
        let _ = ctx.extract_enum_ocd(
            tile,
            bel,
            "CONFIG_RATE",
            &["2", "1", "4", "6", "10", "12", "16", "22", "26"],
            OcdMode::BitOrder,
        );
        let item =
            TileItem::from_bitvec((0..10).map(|bit| TileBit::new(0, 0, bit)).collect(), false);
        for i in 0..10 {
            let val = 1 << i;
            let mut diff = ctx
                .state
                .get_diff(tile, bel, "EXTMASTERCCLK_DIVIDE", val.to_string());
            diff.apply_bitvec_diff_int(&item, val, 1);
            diff.assert_empty();
        }
        ctx.state
            .get_diff(tile, bel, "EXTMASTERCCLK_EN", "NO")
            .assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "EXTMASTERCCLK_EN", "YES");
        diff.apply_bitvec_diff_int(&item, 1, 0xc8);
        ctx.tiledb.insert(tile, bel, "CCLK_DIVISOR", item);
        ctx.tiledb
            .insert(tile, bel, "EXT_CCLK_ENABLE", xlat_bit(diff));
        ctx.collect_enum_int(tile, bel, "CCLK_DLY", 0..4, 0);
        ctx.collect_enum_int(tile, bel, "CCLK_SEP", 0..4, 0);
        for val in ["0", "1", "2", "3"] {
            ctx.state
                .get_diff(tile, bel, "CLK_SWITCH_OPT", val)
                .assert_empty();
        }
    }

    {
        let tile = "REG.HC_OPT";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "BRAM_SKIP", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "TWO_ROUND", "NO", "YES");
        ctx.collect_enum_int(tile, bel, "HC_CYCLE", 1..16, 0);
        ctx.tiledb.insert(
            tile,
            bel,
            "INIT_SKIP",
            TileItem::from_bit(TileBit::new(0, 0, 6), false),
        );
    }

    {
        let tile = "REG.POWERDOWN";
        let bel = "MISC";
        ctx.collect_enum(tile, bel, "SW_CLK", &["STARTUPCLK", "INTERNALCLK"]);
        ctx.collect_enum_bool(tile, bel, "EN_SUSPEND", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "EN_SW_GSR", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "MULTIPIN_WAKEUP", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "SUSPEND_FILTER", "NO", "YES");
        ctx.collect_enum_int(tile, bel, "WAKE_DELAY1", 1..8, 0);
        ctx.collect_enum_int(tile, bel, "WAKE_DELAY2", 1..32, 0);
    }

    {
        let tile = "REG.PU_GWE";
        let bel = "MISC";
        let item =
            TileItem::from_bitvec((0..10).map(|bit| TileBit::new(0, 0, bit)).collect(), false);
        for i in 0..10 {
            let val = 1 << i;
            let mut diff = ctx
                .state
                .get_diff(tile, bel, "SW_GWE_CYCLE", val.to_string());
            diff.apply_bitvec_diff_int(&item, val, 5);
            diff.assert_empty();
        }
        ctx.tiledb.insert(tile, bel, "SW_GWE_CYCLE", item);
    }
    {
        let tile = "REG.PU_GTS";
        let bel = "MISC";
        let item =
            TileItem::from_bitvec((0..10).map(|bit| TileBit::new(0, 0, bit)).collect(), false);
        for i in 0..10 {
            let val = 1 << i;
            let mut diff = ctx
                .state
                .get_diff(tile, bel, "SW_GTS_CYCLE", val.to_string());
            diff.apply_bitvec_diff_int(&item, val, 4);
            diff.assert_empty();
        }
        ctx.tiledb.insert(tile, bel, "SW_GTS_CYCLE", item);
    }

    {
        let tile = "REG.EYE_MASK";
        let bel = "MISC";
        ctx.collect_bitvec(tile, bel, "WAKEUP_MASK", "");
    }

    {
        let tile = "REG.MODE";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "NEXT_CONFIG_NEW_MODE", "NO", "YES");
        ctx.tiledb.insert(
            tile,
            bel,
            "SPI_BUSWIDTH",
            TileItem {
                bits: vec![TileBit::new(0, 0, 11), TileBit::new(0, 0, 12)],
                kind: TileItemKind::Enum {
                    values: [
                        ("1".to_string(), bitvec![0, 0]),
                        ("2".to_string(), bitvec![1, 0]),
                        ("4".to_string(), bitvec![0, 1]),
                    ]
                    .into_iter()
                    .collect(),
                },
            },
        );
        ctx.collect_bitvec(tile, bel, "NEXT_CONFIG_BOOT_MODE", "");
    }

    // these have annoying requirements to fuzz.
    ctx.tiledb.insert(
        "REG.GENERAL12",
        "MISC",
        "NEXT_CONFIG_ADDR",
        TileItem::from_bitvec(
            (0..16)
                .map(|bit| TileBit::new(0, 0, bit))
                .chain((0..16).map(|bit| TileBit::new(1, 0, bit)))
                .collect(),
            false,
        ),
    );
    ctx.tiledb.insert(
        "REG.GENERAL34",
        "MISC",
        "GOLDEN_CONFIG_ADDR",
        TileItem::from_bitvec(
            (0..16)
                .map(|bit| TileBit::new(0, 0, bit))
                .chain((0..16).map(|bit| TileBit::new(1, 0, bit)))
                .collect(),
            false,
        ),
    );
    ctx.tiledb.insert(
        "REG.GENERAL5",
        "MISC",
        "FAILSAFE_USER",
        TileItem::from_bitvec((0..16).map(|bit| TileBit::new(0, 0, bit)).collect(), false),
    );
    ctx.tiledb.insert(
        "REG.TIMER",
        "MISC",
        "TIMER_CFG",
        TileItem::from_bitvec((0..16).map(|bit| TileBit::new(0, 0, bit)).collect(), false),
    );

    {
        let tile = "REG.SEU_OPT";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "POST_CRC_KEEP", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "POST_CRC_ONESHOT", "0", "1");
        ctx.collect_enum_bool(tile, bel, "POST_CRC_SEL", "0", "1");

        // too much effort to include in the automatic fuzzer
        ctx.tiledb.insert(
            tile,
            bel,
            "POST_CRC_EN",
            TileItem::from_bit(TileBit::new(0, 0, 0), false),
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "GLUTMASK",
            TileItem::from_bit(TileBit::new(0, 0, 1), false),
        );

        // again, don't care.
        let _ = ctx.extract_enum_ocd(
            tile,
            bel,
            "POST_CRC_FREQ",
            &[
                "1", "2", "4", "6", "10", "12", "16", "22", "26", "33", "40", "50", "66",
            ],
            OcdMode::BitOrder,
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "POST_CRC_FREQ",
            TileItem::from_bitvec((4..14).map(|bit| TileBit::new(0, 0, bit)).collect(), false),
        );
    }

    {
        let tile = "REG.TESTMODE";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "VGG_TEST", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "ICAP_BYPASS", "NO", "YES");
        ctx.collect_enum_bool(tile, bel, "TESTMODE_EN", "NO", "YES");
    }
}
