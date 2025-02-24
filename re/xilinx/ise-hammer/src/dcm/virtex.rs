use std::collections::BTreeMap;

use bitvec::prelude::*;

use prjcombine_interconnect::dir::Dir;
use prjcombine_re_collector::{xlat_bit, xlat_bool, xlat_enum};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::tiledb::{TileBit, TileItem, TileItemKind};
use prjcombine_xilinx_bitstream::Reg;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits, TileKV},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ExpandedDevice::Virtex(edev) = backend.edev else {
        unreachable!()
    };
    for tile in [
        "DLL.BOT", "DLL.TOP", "DLLP.BOT", "DLLP.TOP", "DLLS.BOT", "DLLS.TOP",
    ] {
        let Some(ctx) = FuzzCtx::try_new(session, backend, tile, "DLL", TileBits::Main(0, 1))
        else {
            continue;
        };
        fuzz_one_extras!(ctx, "PRESENT", "1", [
            (global_mutex_site "DLL")
        ], [
            (mode "DLL")
        ], vec![
            ExtraFeature::new(
                ExtraFeatureKind::MainFixed(edev.chip.col_lio(), edev.chip.row_tio()),
                "CNR.TL",
                "MISC",
                "DLL_ENABLE",
                "1"
            ),
        ]);
        fuzz_enum!(ctx, "RSTMUX", ["0", "1", "RST", "RST_B"], [
            (mode "DLL"),
            (global_mutex "DLL", "USE"),
            (pin "RST")
        ]);
        fuzz_one!(ctx, "HIGH_FREQUENCY", "1", [
            (mode "DLL"),
            (global_mutex "DLL", "USE")
        ], [
            (attr "HIGH_FREQ_ATTR", "HIGH_FREQUENCY")
        ]);
        fuzz_enum!(ctx, "DUTY_ATTR", ["FALSE", "TRUE"], [
            (mode "DLL"),
            (global_mutex "DLL", "USE")
        ]);
        for attr in ["JF_ZD1_ATTR", "JF_ZD2_ATTR"] {
            fuzz_enum!(ctx, attr, [
                "0X80", "0XC0", "0XE0", "0XF0", "0XF8", "0XFC", "0XFE", "0XFF"
            ], [
                (mode "DLL"),
                (global_mutex "DLL", "USE")
            ]);
        }
        fuzz_enum!(ctx, "DIVIDE_ATTR", [
            "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16"
        ], [
            (mode "DLL"),
            (global_mutex "DLL", "USE")
        ]);
        for i in 1..8 {
            fuzz_one!(ctx, "DIVIDE_ATTR", format!("{i}_5.LOW"), [
                (mode "DLL"),
                (global_mutex "DLL", "USE"),
                (attr "HIGH_FREQ_ATTR", "")
            ], [
                (attr "DIVIDE_ATTR", format!("{i}_5"))
            ]);
            fuzz_one!(ctx, "DIVIDE_ATTR", format!("{i}_5.HIGH"), [
                (mode "DLL"),
                (global_mutex "DLL", "USE"),
                (attr "HIGH_FREQ_ATTR", "HIGH_FREQUENCY")
            ], [
                (attr "DIVIDE_ATTR", format!("{i}_5"))
            ]);
        }
        for (attr, opt) in [
            ("CLK_FEEDBACK_2X", "IDLL*FB2X"),
            ("CFG_O_14", "IDLL*CFG_O_14"),
            ("LVL1_MUX_20", "IDLL*_ILVL1_MUX_20"),
            ("LVL1_MUX_21", "IDLL*_ILVL1_MUX_21"),
            ("LVL1_MUX_22", "IDLL*_ILVL1_MUX_22"),
            ("LVL1_MUX_23", "IDLL*_ILVL1_MUX_23"),
            ("LVL1_MUX_24", "IDLL*_ILVL1_MUX_24"),
        ] {
            for val in ["0", "1"] {
                // value "0" is apparently buggy and affects other DLLs than the one we're
                // aiming for, sometimes.
                //
                // have I mentioned I hate ISE?
                if attr == "LVL1_MUX_21" && val == "0" {
                    continue;
                }
                fuzz_one!(ctx, attr, val, [
                    (mode "DLL"),
                    (global_mutex "DLL", "USE"),
                    (pin_node_mutex_shared "CLKIN"),
                    (pin_node_mutex_shared "CLKFB")
                ], [
                    (global_dll opt, val)
                ]);
            }
        }
        for (attr, opt) in [("TESTDLL", "TESTDLL*"), ("TESTZD2OSC", "TESTZD2OSC*")] {
            for val in ["NO", "YES"] {
                fuzz_one!(ctx, attr, val, [
                    (mode "DLL"),
                    (global_mutex "DLL", "USE")
                ], [
                    (global_dll opt, val)
                ]);
            }
        }

        if !(tile.starts_with("DLLS") && backend.device.name.contains('v')) {
            let ctx = FuzzCtx::new(session, backend, tile, "DLL", TileBits::Null);

            if tile.ends_with("BOT") {
                fuzz_one_extras!(ctx, "STARTUP_ATTR", "STARTUP_WAIT", [
                    (global_mutex_site "DLL"),
                    (mode "DLL"),
                    (special TileKV::DeviceSide(Dir::W))
                ], [
                    (attr "STARTUP_ATTR", "STARTUP_WAIT")
                ], vec![
                    ExtraFeature::new(
                        ExtraFeatureKind::Reg(Reg::Cor0),
                        "REG.COR",
                        "STARTUP",
                        "DLL_WAIT_BL",
                        "1"
                    ),
                ]);
                fuzz_one_extras!(ctx, "STARTUP_ATTR", "STARTUP_WAIT", [
                    (global_mutex_site "DLL"),
                    (mode "DLL"),
                    (special TileKV::DeviceSide(Dir::E))
                ], [
                    (attr "STARTUP_ATTR", "STARTUP_WAIT")
                ], vec![
                    ExtraFeature::new(
                        ExtraFeatureKind::Reg(Reg::Cor0),
                        "REG.COR",
                        "STARTUP",
                        "DLL_WAIT_BR",
                        "1"
                    ),
                ]);
            } else {
                fuzz_one_extras!(ctx, "STARTUP_ATTR", "STARTUP_WAIT", [
                    (global_mutex_site "DLL"),
                    (mode "DLL"),
                    (special TileKV::DeviceSide(Dir::W))
                ], [
                    (attr "STARTUP_ATTR", "STARTUP_WAIT")
                ], vec![
                    ExtraFeature::new(
                        ExtraFeatureKind::Reg(Reg::Cor0),
                        "REG.COR",
                        "STARTUP",
                        "DLL_WAIT_TL",
                        "1"
                    ),
                ]);
                fuzz_one_extras!(ctx, "STARTUP_ATTR", "STARTUP_WAIT", [
                    (global_mutex_site "DLL"),
                    (mode "DLL"),
                    (special TileKV::DeviceSide(Dir::E))
                ], [
                    (attr "STARTUP_ATTR", "STARTUP_WAIT")
                ], vec![
                    ExtraFeature::new(
                        ExtraFeatureKind::Reg(Reg::Cor0),
                        "REG.COR",
                        "STARTUP",
                        "DLL_WAIT_TR",
                        "1"
                    ),
                ]);
            }
        }
    }
    let ctx = FuzzCtx::new_fake_tile(session, backend, "NULL", "NULL", TileBits::Null);
    for val in ["90", "180", "270", "360"] {
        let extras = vec![ExtraFeature::new(
            crate::fgen::ExtraFeatureKind::AllDcms,
            "DLL",
            "DLL",
            "TEST_OSC",
            val,
        )];
        fuzz_one_extras!(ctx, "TEST_OSC", val, [], [(global_opt "TESTOSC", val)], extras);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let test_osc = ctx.extract_enum("DLL", "DLL", "TEST_OSC", &["90", "180", "270", "360"]);
    for tile in [
        "DLL.BOT", "DLL.TOP", "DLLP.BOT", "DLLP.TOP", "DLLS.BOT", "DLLS.TOP",
    ] {
        if !ctx.has_tile(tile) {
            continue;
        }
        let bel = "DLL";

        let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "1");

        let item = ctx.extract_enum_bool_wide(tile, bel, "DUTY_ATTR", "FALSE", "TRUE");
        present.apply_bitvec_diff(&item, &bitvec![1; 4], &bitvec![0; 4]);
        ctx.tiledb.insert(tile, bel, "DUTY_CYCLE_CORRECTION", item);

        ctx.collect_bit(tile, bel, "HIGH_FREQUENCY", "1");

        let d0 = ctx.state.get_diff(tile, bel, "RSTMUX", "RST");
        assert_eq!(d0, ctx.state.get_diff(tile, bel, "RSTMUX", "1"));
        let d1 = ctx.state.get_diff(tile, bel, "RSTMUX", "RST_B");
        assert_eq!(d1, ctx.state.get_diff(tile, bel, "RSTMUX", "0"));
        let item = xlat_bool(d0, d1);
        ctx.insert_int_inv(&[tile], tile, bel, "RST", item);

        let item_jf2 =
            TileItem::from_bitvec((0..8).map(|bit| TileBit::new(0, 17, bit)).collect(), false);
        let item_jf1 =
            TileItem::from_bitvec((8..16).map(|bit| TileBit::new(0, 17, bit)).collect(), false);
        for (attr, item, base) in [
            ("JF_ZD2_ATTR", &item_jf2, 0x80),
            ("JF_ZD1_ATTR", &item_jf1, 0xc0),
        ] {
            for val in [0x80, 0xc0, 0xe0, 0xf0, 0xf8, 0xfc, 0xfe, 0xff] {
                let mut diff = ctx.state.get_diff(tile, bel, attr, format!("0X{val:02X}"));
                diff.apply_bitvec_diff_int(item, val, base);
                diff.assert_empty();
            }
            present.apply_bitvec_diff_int(item, base, 0xf0);
        }
        ctx.tiledb.insert(tile, bel, "FACTORY_JF2", item_jf2);
        ctx.tiledb.insert(tile, bel, "FACTORY_JF1", item_jf1);

        let clkdv_count_max =
            TileItem::from_bitvec((4..8).map(|bit| TileBit::new(0, 18, bit)).collect(), false);
        let clkdv_count_fall =
            TileItem::from_bitvec((8..12).map(|bit| TileBit::new(0, 18, bit)).collect(), false);
        let clkdv_count_fall_2 = TileItem::from_bitvec(
            (12..16).map(|bit| TileBit::new(0, 18, bit)).collect(),
            false,
        );
        let clkdv_phase_rise =
            TileItem::from_bitvec((1..3).map(|bit| TileBit::new(0, 16, bit)).collect(), false);
        let clkdv_phase_fall =
            TileItem::from_bitvec((3..5).map(|bit| TileBit::new(0, 16, bit)).collect(), false);
        let clkdv_mode = TileItem {
            bits: vec![TileBit::new(0, 16, 15)],
            kind: TileItemKind::Enum {
                values: BTreeMap::from_iter([
                    ("HALF".to_string(), bitvec![0]),
                    ("INT".to_string(), bitvec![1]),
                ]),
            },
        };
        for i in 2..=16 {
            let mut diff = ctx.state.get_diff(tile, bel, "DIVIDE_ATTR", format!("{i}"));
            diff.apply_bitvec_diff_int(&clkdv_count_max, i - 1, 1);
            diff.apply_bitvec_diff_int(&clkdv_count_fall, (i - 1) / 2, 0);
            diff.apply_bitvec_diff_int(&clkdv_phase_fall, (i % 2) * 2, 0);
            diff.assert_empty();
        }
        for i in 1..=7 {
            let mut diff = ctx
                .state
                .get_diff(tile, bel, "DIVIDE_ATTR", format!("{i}_5.LOW"));
            diff.apply_enum_diff(&clkdv_mode, "HALF", "INT");
            diff.apply_bitvec_diff_int(&clkdv_count_max, 2 * i, 1);
            diff.apply_bitvec_diff_int(&clkdv_count_fall, i / 2, 0);
            diff.apply_bitvec_diff_int(&clkdv_count_fall_2, 3 * i / 2 + 1, 0);
            diff.apply_bitvec_diff_int(&clkdv_phase_fall, (i % 2) * 2 + 1, 0);
            diff.assert_empty();
            let mut diff = ctx
                .state
                .get_diff(tile, bel, "DIVIDE_ATTR", format!("{i}_5.HIGH"));
            diff.apply_enum_diff(&clkdv_mode, "HALF", "INT");
            diff.apply_bitvec_diff_int(&clkdv_count_max, 2 * i, 1);
            diff.apply_bitvec_diff_int(&clkdv_count_fall, (i - 1) / 2, 0);
            diff.apply_bitvec_diff_int(&clkdv_count_fall_2, (3 * i).div_ceil(2), 0);
            diff.apply_bitvec_diff_int(&clkdv_phase_fall, (i % 2) * 2, 0);
            diff.assert_empty();
        }
        present.apply_bitvec_diff_int(&clkdv_count_max, 1, 0);
        present.apply_enum_diff(&clkdv_mode, "INT", "HALF");
        ctx.tiledb
            .insert(tile, bel, "CLKDV_COUNT_MAX", clkdv_count_max);
        ctx.tiledb
            .insert(tile, bel, "CLKDV_COUNT_FALL", clkdv_count_fall);
        ctx.tiledb
            .insert(tile, bel, "CLKDV_COUNT_FALL_2", clkdv_count_fall_2);
        ctx.tiledb
            .insert(tile, bel, "CLKDV_PHASE_RISE", clkdv_phase_rise);
        ctx.tiledb
            .insert(tile, bel, "CLKDV_PHASE_FALL", clkdv_phase_fall);
        ctx.tiledb.insert(tile, bel, "CLKDV_MODE", clkdv_mode);

        ctx.collect_enum_bool(tile, bel, "CFG_O_14", "0", "1");
        ctx.collect_enum_bool(tile, bel, "LVL1_MUX_20", "0", "1");
        ctx.collect_bit(tile, bel, "LVL1_MUX_21", "1");
        ctx.collect_enum_bool(tile, bel, "LVL1_MUX_22", "0", "1");
        ctx.collect_enum_bool(tile, bel, "LVL1_MUX_23", "0", "1");
        ctx.collect_enum_bool(tile, bel, "LVL1_MUX_24", "0", "1");
        ctx.collect_enum_bool(tile, bel, "TESTZD2OSC", "NO", "YES");
        ctx.collect_enum_bool_wide(tile, bel, "TESTDLL", "NO", "YES");
        let item = xlat_enum(vec![
            ("1X", ctx.state.get_diff(tile, bel, "CLK_FEEDBACK_2X", "0")),
            ("2X", ctx.state.get_diff(tile, bel, "CLK_FEEDBACK_2X", "1")),
        ]);
        ctx.tiledb.insert(tile, bel, "CLK_FEEDBACK", item);

        present.apply_bit_diff(ctx.tiledb.item(tile, bel, "CFG_O_14"), true, false);
        if ctx.device.name.ends_with('e') {
            ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(present));
        } else {
            present.assert_empty();
        }

        ctx.tiledb.insert(tile, bel, "TEST_OSC", test_osc.clone());
    }
    ctx.collect_bit("CNR.TL", "MISC", "DLL_ENABLE", "1");
    let tile = "REG.COR";
    let bel = "STARTUP";
    ctx.collect_bit(tile, bel, "DLL_WAIT_BL", "1");
    ctx.collect_bit(tile, bel, "DLL_WAIT_BR", "1");
    ctx.collect_bit(tile, bel, "DLL_WAIT_TL", "1");
    ctx.collect_bit(tile, bel, "DLL_WAIT_TR", "1");
}
