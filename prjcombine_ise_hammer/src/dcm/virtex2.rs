use std::collections::BTreeMap;

use bitvec::prelude::*;
use prjcombine_hammer::Session;
use prjcombine_int::db::Dir;
use prjcombine_types::{TileItem, TileItemKind};
use prjcombine_virtex2::grid::{ColumnKind, GridKind};
use prjcombine_xilinx_geom::ExpandedDevice;

use crate::{
    backend::{FeatureBit, IseBackend, PinFromKind},
    diff::{
        extract_bitvec_val, extract_bitvec_val_part, xlat_bit, xlat_bit_wide, xlat_bitvec,
        xlat_bool, xlat_enum, CollectorCtx, Diff,
    },
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits, TileKV},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_multi, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(
    session: &mut Session<IseBackend<'a>>,
    backend: &IseBackend<'a>,
    devdata_only: bool,
) {
    let ExpandedDevice::Virtex2(edev) = backend.edev else {
        unreachable!()
    };
    let tile = match edev.grid.kind {
        GridKind::Virtex2 => "DCM.V2",
        GridKind::Virtex2P | GridKind::Virtex2PX => "DCM.V2P",
        GridKind::Spartan3 => "DCM.S3",
        _ => unreachable!(),
    };

    if devdata_only {
        let ctx = FuzzCtx::new(session, backend, tile, "DCM", TileBits::Dcm);
        let mut extras = vec![];
        if edev.grid.kind == GridKind::Spartan3 {
            extras.extend([ExtraFeature::new(
                ExtraFeatureKind::DcmLL,
                "LL.S3",
                "MISC",
                "DCM_ENABLE",
                "1",
            )]);
        }
        fuzz_one_extras!(ctx, "ENABLE", "1", [
            (global_mutex "DCM_OPT", "NO"),
            (special TileKV::DeviceSide(Dir::S)),
            (special TileKV::DeviceSide(Dir::W))
        ], [
            (mode "DCM")
        ], extras.clone());
        return;
    }

    let ctx = FuzzCtx::new_fake_tile(session, backend, "NULL", "NULL", TileBits::Null);
    for val in ["90", "180", "270", "360"] {
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::AllDcms,
            tile,
            "DCM",
            "TEST_OSC",
            val,
        )];
        fuzz_one_extras!(ctx, "TEST_OSC", val, [], [(global_opt "TESTOSC", val)], extras);
    }

    let ctx = FuzzCtx::new(session, backend, tile, "DCM", TileBits::Dcm);
    let mut extras = vec![];
    if edev.grid.kind == GridKind::Spartan3 {
        extras.extend([
            ExtraFeature::new(ExtraFeatureKind::DcmLL, "LL.S3", "MISC", "DCM_ENABLE", "1"),
            ExtraFeature::new(ExtraFeatureKind::DcmUL, "UL.S3", "MISC", "DCM_ENABLE", "1"),
        ]);
        if edev.grid.columns[edev.grid.columns.last_id().unwrap() - 3].kind == ColumnKind::Bram {
            extras.extend([
                ExtraFeature::new(ExtraFeatureKind::DcmLR, "LR.S3", "MISC", "DCM_ENABLE", "1"),
                ExtraFeature::new(ExtraFeatureKind::DcmUR, "UR.S3", "MISC", "DCM_ENABLE", "1"),
            ]);
        }
    }
    fuzz_one_extras!(ctx, "ENABLE", "1", [
        (global_mutex "DCM_OPT", "NO")
    ], [
        (mode "DCM")
    ], extras.clone());
    fuzz_one_extras!(ctx, "ENABLE", "OPT_BASE", [
        (global_mutex "DCM_OPT", "YES"),
        (global_opt "VBG_SEL0", "0"),
        (global_opt "VBG_SEL1", "0"),
        (global_opt "VBG_SEL2", "0"),
        (global_opt "VBG_PD0", "0"),
        (global_opt "VBG_PD1", "0")
    ], [
        (mode "DCM")
    ], extras.clone());

    for opt in ["VBG_SEL0", "VBG_SEL1", "VBG_SEL2", "VBG_PD0", "VBG_PD1"] {
        fuzz_one_extras!(ctx, "ENABLE", opt, [
            (global_mutex "DCM_OPT", "YES"),
            (global_opt "VBG_SEL0", if opt == "VBG_SEL0" {"1"} else {"0"}),
            (global_opt "VBG_SEL1", if opt == "VBG_SEL1" {"1"} else {"0"}),
            (global_opt "VBG_SEL2", if opt == "VBG_SEL2" {"1"} else {"0"}),
            (global_opt "VBG_PD0", if opt == "VBG_PD0" {"1"} else {"0"}),
            (global_opt "VBG_PD1", if opt == "VBG_PD1" {"1"} else {"0"})
        ], [
            (mode "DCM")
        ], extras.clone());
    }

    for pin in ["RST", "PSCLK", "PSEN", "PSINCDEC", "DSSEN"] {
        fuzz_inv!(ctx, pin, [
            (mode "DCM"),
            (global_mutex "PSCLK", "DCM"),
            (mutex "MODE", "SIMPLE")
        ]);
    }
    for pin in [
        "CTLMODE",
        "CTLSEL0",
        "CTLSEL1",
        "CTLSEL2",
        "CTLOSC1",
        "CTLOSC2",
        "CTLGO",
        "STSADRS0",
        "STSADRS1",
        "STSADRS2",
        "STSADRS3",
        "STSADRS4",
        "FREEZEDFS",
        "FREEZEDLL",
    ] {
        if pin == "STSADRS4" && edev.grid.kind == GridKind::Virtex2 {
            continue;
        }
        fuzz_inv!(ctx, pin, [
            (mode "DCM"),
            (mutex "MODE", "SIMPLE"),
            (mutex "INV", pin)
        ]);
    }

    for pin in [
        "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX", "CLKFX180",
        "CONCUR",
    ] {
        fuzz_one!(ctx, pin, "1", [
            (mode "DCM"),
            (mutex "MODE", "PINS"),
            (mutex "PIN", pin),
            (nopin "CLKFB")
        ], [
            (pin pin)
        ]);
        fuzz_one!(ctx, pin, "1.CLKFB", [
            (mode "DCM"),
            (mutex "MODE", "PINS"),
            (mutex "PIN", pin),
            (pin "CLKFB")
        ], [
            (pin pin)
        ]);
        if pin != "CLKFX" && pin != "CLKFX180" && pin != "CONCUR" {
            fuzz_one!(ctx, pin, "1.CLKFX", [
                (mode "DCM"),
                (mutex "MODE", "PINS"),
                (mutex "PIN", format!("{pin}.CLKFX")),
                (pin "CLKFX"),
                (pin "CLKFB")
            ], [
                (pin pin)
            ]);
        }
    }
    fuzz_one!(ctx, "CLKFB", "1", [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE")
    ], [
        (pin "CLKFB")
    ]);
    fuzz_one!(ctx, "CLKIN_IOB", "1", [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE"),
        (pin "CLKIN"),
        (pin "CLKFB"),
        (pin_from "CLKFB", PinFromKind::Bufg)
    ], [
        (pin_from "CLKIN", PinFromKind::Bufg, PinFromKind::Iob)
    ]);
    fuzz_one!(ctx, "CLKFB_IOB", "1", [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE"),
        (pin "CLKIN"),
        (pin "CLKFB"),
        (pin_from "CLKIN", PinFromKind::Bufg)
    ], [
        (pin_from "CLKFB", PinFromKind::Bufg, PinFromKind::Iob)
    ]);
    for pin in [
        "STATUS0", "STATUS1", "STATUS2", "STATUS3", "STATUS4", "STATUS5", "STATUS6", "STATUS7",
    ] {
        fuzz_one!(ctx, pin, "1", [
            (mode "DCM"),
            (mutex "MODE", "SIMPLE")
        ], [
            (pin pin)
        ]);
    }

    fuzz_enum!(ctx, "DLL_FREQUENCY_MODE", ["LOW", "HIGH"], [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE")
    ]);
    fuzz_enum!(ctx, "DFS_FREQUENCY_MODE", ["LOW", "HIGH"], [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE")
    ]);
    fuzz_enum!(ctx, "STARTUP_WAIT", ["STARTUP_WAIT"], [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE"),
        (global_opt "GTS_CYCLE", "1"),
        (global_opt "DONE_CYCLE", "1"),
        (global_opt "LCK_CYCLE", "NOWAIT")
    ]);
    fuzz_enum!(ctx, "DUTY_CYCLE_CORRECTION", ["FALSE", "TRUE"], [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE")
    ]);
    fuzz_enum!(ctx, "FACTORY_JF1", ["0X80", "0XC0", "0XE0", "0XF0", "0XF8", "0XFC", "0XFE", "0XFF"], [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE")
    ]);
    fuzz_enum!(ctx, "FACTORY_JF2", ["0X80", "0XC0", "0XE0", "0XF0", "0XF8", "0XFC", "0XFE", "0XFF"], [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE")
    ]);
    fuzz_multi!(ctx, "DESKEW_ADJUST", "", 4, [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE")
    ], (attr_dec "DESKEW_ADJUST"));
    fuzz_enum!(ctx, "CLKIN_DIVIDE_BY_2", ["CLKIN_DIVIDE_BY_2"], [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE")
    ]);
    fuzz_enum!(ctx, "VERY_HIGH_FREQUENCY", ["VERY_HIGH_FREQUENCY"], [
        (mode "DCM"),
        (attr "DUTY_CYCLE_CORRECTION", "#OFF"),
        (mutex "MODE", "SIMPLE"),
        (pin "CLK0")
    ]);
    fuzz_enum!(ctx, "DSS_MODE", ["NONE", "SPREAD_2", "SPREAD_4", "SPREAD_6", "SPREAD_8"], [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE"),
        (attr "CLKOUT_PHASE_SHIFT", "NONE")
    ]);
    fuzz_enum!(ctx, "CLK_FEEDBACK", ["1X", "2X"], [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE")
    ]);
    fuzz_enum!(ctx, "CLKOUT_PHASE_SHIFT", ["NONE", "FIXED", "VARIABLE"], [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE"),
        (attr "PHASE_SHIFT", "1"),
        (pin "CLK0")
    ]);
    fuzz_one!(ctx, "CLKOUT_PHASE_SHIFT", "FIXED.NEG", [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE"),
        (attr "PHASE_SHIFT", "-1"),
        (pin "CLK0")
    ], [
        (attr "CLKOUT_PHASE_SHIFT", "FIXED")
    ]);
    fuzz_one!(ctx, "CLKOUT_PHASE_SHIFT", "VARIABLE.NEG", [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE"),
        (attr "PHASE_SHIFT", "-1"),
        (pin "CLK0")
    ], [
        (attr "CLKOUT_PHASE_SHIFT", "VARIABLE")
    ]);
    fuzz_multi!(ctx, "CLKFX_MULTIPLY", "", 12, [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE")
    ], (attr_dec_delta "CLKFX_MULTIPLY", 1));
    fuzz_multi!(ctx, "CLKFX_DIVIDE", "", 12, [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE")
    ], (attr_dec_delta "CLKFX_DIVIDE", 1));

    fuzz_multi!(ctx, "PHASE_SHIFT", "", 8, [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE"),
        (attr "CLKOUT_PHASE_SHIFT", "FIXED")
    ], (attr_dec "PHASE_SHIFT"));
    fuzz_one!(ctx, "PHASE_SHIFT", "-255.FIXED", [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE"),
        (attr "CLKOUT_PHASE_SHIFT", "FIXED")
    ], [
        (attr "PHASE_SHIFT", "-255")
    ]);
    fuzz_one!(ctx, "PHASE_SHIFT", "-255.VARIABLE", [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE"),
        (attr "CLKOUT_PHASE_SHIFT", "VARIABLE")
    ], [
        (attr "PHASE_SHIFT", "-255")
    ]);

    fuzz_enum!(ctx, "CLKDV_DIVIDE", ["2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16"], [
        (mode "DCM"),
        (mutex "MODE", "SIMPLE")
    ]);
    for dll_mode in ["LOW", "HIGH"] {
        for val in ["1_5", "2_5", "3_5", "4_5", "5_5", "6_5", "7_5"] {
            fuzz_one!(
                ctx,
                "CLKDV_DIVIDE",
                format!("{val}.{dll_mode}"), [
                    (mode "DCM"),
                    (mutex "MODE", "SIMPLE"),
                    (attr "DLL_FREQUENCY_MODE", dll_mode)
                ], [
                    (attr "CLKDV_DIVIDE", val)
                ]
            );
        }
    }

    fuzz_multi!(ctx, "DLLC", "", 32, [
        (mode "DCM"),
        (mutex "MODE", "LL_DLLC"),
        (no_global_opt "TESTOSC"),
        (pin "STATUS1"),
        (pin "STATUS7")
    ], (attr_hex "LL_HEX_DLLC"));
    fuzz_multi!(ctx, "DLLS", "", 32, [
        (mode "DCM"),
        (mutex "MODE", "LL_DLLS")
    ], (attr_hex "LL_HEX_DLLS"));
    fuzz_multi!(ctx, "DFS", "", 32, [
        (mode "DCM"),
        (mutex "MODE", "LL_DFS")
    ], (attr_hex "LL_HEX_DFS"));
    fuzz_multi!(ctx, "COM", "", 32, [
        (mode "DCM"),
        (mutex "MODE", "LL_COM")
    ], (attr_hex "LL_HEX_COM"));
    fuzz_multi!(ctx, "MISC", "", 32, [
        (mode "DCM"),
        (mutex "MODE", "LL_MISC")
    ], (attr_hex "LL_HEX_MISC"));
    for val in ["0", "1", "2", "3"] {
        fuzz_one!(ctx, "COIN_WINDOW", val, [
            (mode "DCM"),
            (mutex "MODE", "GLOBALS")
        ], [
            (global_xy "COINWINDOW_*", val)
        ]);
        fuzz_one!(ctx, "SEL_PL_DLY", val, [
            (mode "DCM"),
            (mutex "MODE", "GLOBALS")
        ], [
            (global_xy "SELPLDLY_*", val)
        ]);
    }
    for val in ["0", "1"] {
        fuzz_one!(ctx, "EN_OSC_COARSE", val, [
            (mode "DCM"),
            (mutex "MODE", "GLOBALS")
        ], [
            (global_xy "ENOSCCOARSE_*", val)
        ]);
        fuzz_one!(ctx, "EN_DUMMY_OSC", val, [
            (mode "DCM"),
            (mutex "MODE", "GLOBALS"),
            (global_xy "NONSTOP_*", "0")
        ], [
            (global_xy "ENDUMMYOSC_*", val)
        ]);
        fuzz_one!(ctx, "PL_CENTERED", val, [
            (mode "DCM"),
            (mutex "MODE", "GLOBALS")
        ], [
            (global_xy "PLCENTERED_*", val)
        ]);
        fuzz_one!(ctx, "NON_STOP", val, [
            (mode "DCM"),
            (mutex "MODE", "GLOBALS"),
            (global_xy "ENDUMMYOSC_*", "0")
        ], [
            (global_xy "NONSTOP_*", val)
        ]);
        fuzz_one!(ctx, "ZD2_BY1", val, [
            (mode "DCM"),
            (mutex "MODE", "GLOBALS"),
            (mutex "ZD2", "PLAIN")
        ], [
            (global_xy "ZD2_BY1_*", val)
        ]);
        if edev.grid.kind.is_virtex2() {
            fuzz_one!(ctx, "PS_CENTERED", val, [
                (mode "DCM"),
                (mutex "MODE", "GLOBALS")
            ], [
                (global_xy "CENTERED_*", val)
            ]);
            fuzz_one!(ctx, "ZD2_HF_BY1", val, [
                (mode "DCM"),
                (mutex "MODE", "GLOBALS"),
                (mutex "ZD2", "HF")
            ], [
                (global_xy "ZD2_HF_BY1_*", val)
            ]);
        }
        if edev.grid.kind != GridKind::Virtex2 {
            fuzz_one!(ctx, "ZD1_BY1", val, [
                (mode "DCM"),
                (mutex "MODE", "GLOBALS")
            ], [
                (global_xy "ZD1_BY1_*", val)
            ]);
            fuzz_one!(ctx, "RESET_PS_SEL", val, [
                (mode "DCM"),
                (mutex "MODE", "GLOBALS")
            ], [
                (global_xy "RESETPS_SEL_*", val)
            ]);
        }
        if edev.grid.kind == GridKind::Spartan3 {
            fuzz_one!(ctx, "SPLY_IDC0", val, [
                (mode "DCM"),
                (mutex "MODE", "GLOBALS")
            ], [
                (global_xy "SPLY_IDC0_*", val)
            ]);
            fuzz_one!(ctx, "SPLY_IDC1", val, [
                (mode "DCM"),
                (mutex "MODE", "GLOBALS")
            ], [
                (global_xy "SPLY_IDC1_*", val)
            ]);
            fuzz_one!(ctx, "EXTENDED_FLUSH_TIME", val, [
                (mode "DCM"),
                (mutex "MODE", "GLOBALS")
            ], [
                (global_xy "EXTENDEDFLUSHTIME_*", val)
            ]);
            fuzz_one!(ctx, "EXTENDED_HALT_TIME", val, [
                (mode "DCM"),
                (mutex "MODE", "GLOBALS")
            ], [
                (global_xy "EXTENDEDHALTTIME_*", val)
            ]);
            fuzz_one!(ctx, "EXTENDED_RUN_TIME", val, [
                (mode "DCM"),
                (mutex "MODE", "GLOBALS")
            ], [
                (global_xy "EXTENDEDRUNTIME_*", val)
            ]);
            for i in 0..=8 {
                fuzz_one!(ctx, format!("CFG_DLL_PS{i}"), val, [
                    (mode "DCM"),
                    (mutex "MODE", "GLOBALS")
                ], [
                    (global_xy format!("CFG_DLL_PS{i}_*"), val)
                ]);
            }
            for i in 0..=2 {
                fuzz_one!(ctx, format!("CFG_DLL_LP{i}"), val, [
                    (mode "DCM"),
                    (mutex "MODE", "GLOBALS")
                ], [
                    (global_xy format!("CFG_DLL_LP{i}_*"), val)
                ]);
            }
            for i in 0..=1 {
                fuzz_one!(ctx, format!("SEL_HSYNC_B{i}"), val, [
                    (mode "DCM"),
                    (mutex "MODE", "GLOBALS")
                ], [
                    (global_xy format!("SELHSYNC_B{i}_*"), val)
                ]);
            }
            for i in 0..=1 {
                fuzz_one!(ctx, format!("LPON_B_DFS{i}"), val, [
                    (mode "DCM"),
                    (mutex "MODE", "GLOBALS")
                ], [
                    (global_xy format!("LPON_B_DFS{i}_*"), val)
                ]);
            }
            fuzz_one!(ctx, "EN_PWCTL", val, [
                (mode "DCM"),
                (mutex "MODE", "GLOBALS")
            ], [
                (global_xy "ENPWCTL_*", val)
            ]);
            fuzz_one!(ctx, "M1D1", val, [
                (mode "DCM"),
                (mutex "MODE", "GLOBALS")
            ], [
                (global_xy "M1D1_*", val)
            ]);
            fuzz_one!(ctx, "MIS1", val, [
                (mode "DCM"),
                (mutex "MODE", "GLOBALS")
            ], [
                (global_xy "MIS1_*", val)
            ]);
            fuzz_one!(ctx, "EN_RELRST_B", val, [
                (mode "DCM"),
                (mutex "MODE", "GLOBALS")
            ], [
                (global_xy "ENRELRST_B_*", val)
            ]);
            fuzz_one!(ctx, "EN_OLD_OSCCTL", val, [
                (mode "DCM"),
                (mutex "MODE", "GLOBALS")
            ], [
                (global_xy "ENOLDOSCCTL_*", val)
            ]);
            fuzz_one!(ctx, "TRIM_LP_B", val, [
                (mode "DCM"),
                (mutex "MODE", "GLOBALS")
            ], [
                (global_xy "TRIM_LP_B_*", val)
            ]);
            fuzz_one!(ctx, "INVERT_ZD1_CUSTOM", val, [
                (mode "DCM"),
                (mutex "MODE", "GLOBALS")
            ], [
                (global_xy "INVERT_ZD1_CUSTOM_*", val)
            ]);
            for i in 0..=4 {
                fuzz_one!(ctx, format!("VREG_PROBE{i}"), val, [
                    (mode "DCM"),
                    (mutex "MODE", "GLOBALS")
                ], [
                    (global_xy format!("VREG_PROBE{i}_*"), val)
                ]);
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let ExpandedDevice::Virtex2(edev) = ctx.edev else {
        unreachable!()
    };
    let tile = match edev.grid.kind {
        GridKind::Virtex2 => "DCM.V2",
        GridKind::Virtex2P | GridKind::Virtex2PX => "DCM.V2P",
        GridKind::Spartan3 => "DCM.S3",
        _ => unreachable!(),
    };
    let bel = "DCM";

    if devdata_only {
        let mut present = ctx.state.get_diff(tile, bel, "ENABLE", "1");
        let item = ctx.tiledb.item(tile, bel, "DESKEW_ADJUST");
        let val = extract_bitvec_val(
            item,
            &bitvec![0; 4],
            present.split_bits(&item.bits.iter().copied().collect()),
        );
        ctx.tiledb
            .insert_device_data(&ctx.device.name, "DCM:DESKEW_ADJUST", val);
        let vbg_sel = extract_bitvec_val_part(
            ctx.tiledb.item(tile, bel, "VBG_SEL"),
            &bitvec![0; 3],
            &mut present,
        );
        let vbg_pd = extract_bitvec_val_part(
            ctx.tiledb.item(tile, bel, "VBG_PD"),
            &bitvec![0; 2],
            &mut present,
        );
        ctx.tiledb
            .insert_device_data(&ctx.device.name, "DCM:VBG_SEL", vbg_sel);
        ctx.tiledb
            .insert_device_data(&ctx.device.name, "DCM:VBG_PD", vbg_pd);
        if edev.grid.kind == GridKind::Spartan3 {
            ctx.collect_bit("LL.S3", "MISC", "DCM_ENABLE", "1");
        }
        return;
    }

    let mut present = ctx.state.get_diff(tile, bel, "ENABLE", "1");
    let dllc = ctx.state.get_diffs(tile, bel, "DLLC", "");
    let dlls = ctx.state.get_diffs(tile, bel, "DLLS", "");
    let dfs = ctx.state.get_diffs(tile, bel, "DFS", "");
    let mut com = ctx.state.get_diffs(tile, bel, "COM", "");
    let mut misc = ctx.state.get_diffs(tile, bel, "MISC", "");

    // sigh. fixups.
    assert!(com[11].bits.is_empty());
    let com9 = *com[9].bits.keys().next().unwrap();
    let com11 = FeatureBit {
        bit: com9.bit + 2,
        ..com9
    };
    assert_eq!(com[10].bits.remove(&com11), Some(true));
    com[11].bits.insert(com11, true);

    if edev.grid.kind == GridKind::Spartan3 {
        for diff in &misc[12..31] {
            assert!(diff.bits.is_empty());
        }
        misc.truncate(12);
    }

    let dllc = xlat_bitvec(dllc);
    let dlls = xlat_bitvec(dlls);
    let dfs = xlat_bitvec(dfs);
    let com = xlat_bitvec(com);
    let misc = xlat_bitvec(misc);

    let base = ctx.state.get_diff(tile, bel, "ENABLE", "OPT_BASE");
    for (attr, len) in [("VBG_SEL", 3), ("VBG_PD", 2)] {
        let mut diffs = vec![];
        for bit in 0..len {
            diffs.push(
                ctx.state
                    .get_diff(tile, bel, "ENABLE", format!("{attr}{bit}"))
                    .combine(&!&base),
            );
        }
        ctx.tiledb.insert(tile, bel, attr, xlat_bitvec(diffs));
    }
    ctx.collect_enum(tile, bel, "TEST_OSC", &["90", "180", "270", "360"]);

    ctx.collect_enum(tile, bel, "COIN_WINDOW", &["0", "1", "2", "3"]);
    ctx.collect_enum(tile, bel, "SEL_PL_DLY", &["0", "1", "2", "3"]);
    ctx.collect_enum_bool(tile, bel, "EN_OSC_COARSE", "0", "1");
    ctx.collect_enum_bool(tile, bel, "PL_CENTERED", "0", "1");
    if edev.grid.kind.is_virtex2() {
        ctx.state
            .get_diff(tile, bel, "NON_STOP", "0")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "EN_DUMMY_OSC", "1")
            .assert_empty();
        let en_dummy_osc = !ctx.state.get_diff(tile, bel, "EN_DUMMY_OSC", "0");
        let non_stop = ctx.state.get_diff(tile, bel, "NON_STOP", "1");
        let (en_dummy_osc, non_stop, common) = Diff::split(en_dummy_osc, non_stop);
        ctx.tiledb.insert(tile, bel, "NON_STOP", xlat_bit(non_stop));
        ctx.tiledb
            .insert(tile, bel, "EN_DUMMY_OSC", xlat_bit_wide(en_dummy_osc));
        ctx.tiledb
            .insert(tile, bel, "EN_DUMMY_OSC_OR_NON_STOP", xlat_bit(common));
    } else {
        ctx.collect_enum_bool(tile, bel, "EN_DUMMY_OSC", "0", "1");
        ctx.collect_enum_bool(tile, bel, "NON_STOP", "0", "1");
    }
    ctx.collect_enum_bool(tile, bel, "ZD2_BY1", "0", "1");
    if edev.grid.kind.is_virtex2() {
        ctx.collect_enum_bool(tile, bel, "PS_CENTERED", "0", "1");
        let item = ctx.extract_enum_bool(tile, bel, "ZD2_HF_BY1", "0", "1");
        assert_eq!(item, *ctx.tiledb.item(tile, bel, "ZD2_BY1"));
    }
    if edev.grid.kind != GridKind::Virtex2 {
        ctx.collect_enum_bool(tile, bel, "ZD1_BY1", "0", "1");
        ctx.collect_enum_bool(tile, bel, "RESET_PS_SEL", "0", "1");
    }
    if edev.grid.kind == GridKind::Spartan3 {
        ctx.collect_enum_bool(tile, bel, "EXTENDED_FLUSH_TIME", "0", "1");
        ctx.collect_enum_bool(tile, bel, "EXTENDED_HALT_TIME", "0", "1");
        ctx.collect_enum_bool(tile, bel, "EXTENDED_RUN_TIME", "0", "1");
        ctx.collect_enum_bool(tile, bel, "M1D1", "0", "1");
        ctx.collect_enum_bool(tile, bel, "MIS1", "0", "1");
        ctx.collect_enum_bool(tile, bel, "EN_OLD_OSCCTL", "0", "1");
        ctx.collect_enum_bool(tile, bel, "EN_PWCTL", "0", "1");
        ctx.collect_enum_bool(tile, bel, "EN_RELRST_B", "0", "1");
        ctx.collect_enum_bool(tile, bel, "INVERT_ZD1_CUSTOM", "0", "1");
        ctx.collect_enum_bool(tile, bel, "TRIM_LP_B", "0", "1");

        for (attr, len) in [
            ("SPLY_IDC", 2),
            ("VREG_PROBE", 5),
            ("CFG_DLL_PS", 9),
            ("CFG_DLL_LP", 3),
            ("SEL_HSYNC_B", 2),
            ("LPON_B_DFS", 2),
        ] {
            let mut diffs = vec![];
            for i in 0..len {
                let d0 = ctx.state.get_diff(tile, bel, format!("{attr}{i}"), "0");
                let d1 = ctx.state.get_diff(tile, bel, format!("{attr}{i}"), "1");
                if d0.bits.is_empty() {
                    diffs.push(d1);
                } else {
                    diffs.push(!d0);
                    d1.assert_empty();
                }
            }
            ctx.tiledb.insert(tile, bel, attr, xlat_bitvec(diffs));
        }
    }

    let int_tiles = &[match edev.grid.kind {
        GridKind::Virtex2 => "INT.DCM.V2",
        GridKind::Virtex2P | GridKind::Virtex2PX => "INT.DCM.V2P",
        GridKind::Spartan3 => "INT.DCM",
        _ => unreachable!(),
    }];
    ctx.collect_int_inv(int_tiles, tile, bel, "PSCLK", false);
    for pin in ["RST", "PSEN", "PSINCDEC"] {
        ctx.collect_inv(tile, bel, pin);
    }
    if edev.grid.kind == GridKind::Spartan3 {
        ctx.state
            .get_diff(tile, bel, "DSSENINV", "DSSEN")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "DSSENINV", "DSSEN_B")
            .assert_empty();
    } else {
        ctx.collect_inv(tile, bel, "DSSEN");
    }
    for pin in [
        "CTLMODE",
        "CTLSEL0",
        "CTLSEL1",
        "CTLSEL2",
        "CTLOSC1",
        "CTLOSC2",
        "CTLGO",
        "STSADRS0",
        "STSADRS1",
        "STSADRS2",
        "STSADRS3",
        "STSADRS4",
        "FREEZEDFS",
        "FREEZEDLL",
    ] {
        if pin == "STSADRS4" && edev.grid.kind == GridKind::Virtex2 {
            continue;
        }
        let d0 = ctx.state.get_diff(tile, bel, format!("{pin}INV"), pin);
        let d1 = ctx
            .state
            .get_diff(tile, bel, format!("{pin}INV"), format!("{pin}_B"));
        let (d0, d1, dc) = Diff::split(d0, d1);
        ctx.tiledb
            .insert(tile, bel, format!("INV.{pin}"), xlat_bool(d0, d1));
        if edev.grid.kind.is_virtex2() {
            ctx.tiledb.insert(tile, bel, "TEST_ENABLE", xlat_bit(dc));
        } else {
            dc.assert_empty();
        }
    }
    for pin in [
        "STATUS0", "STATUS2", "STATUS3", "STATUS4", "STATUS5", "STATUS6",
    ] {
        ctx.state.get_diff(tile, bel, pin, "1").assert_empty();
    }
    for pin in ["STATUS1", "STATUS7"] {
        ctx.collect_bit(tile, bel, pin, "1");
    }
    let (_, _, en_dll) = Diff::split(
        ctx.state.peek_diff(tile, bel, "CLK0", "1").clone(),
        ctx.state.peek_diff(tile, bel, "CLK90", "1").clone(),
    );
    let (_, _, en_dfs) = Diff::split(
        ctx.state.peek_diff(tile, bel, "CLKFX", "1").clone(),
        ctx.state.peek_diff(tile, bel, "CLKFX180", "1").clone(),
    );
    let vhf = ctx
        .state
        .get_diff(tile, bel, "VERY_HIGH_FREQUENCY", "VERY_HIGH_FREQUENCY");
    assert_eq!(en_dll, !vhf);
    for pin in [
        "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV",
    ] {
        let diff = ctx.state.get_diff(tile, bel, pin, "1");
        let diff_fb = ctx.state.get_diff(tile, bel, pin, "1.CLKFB");
        let diff_fx = ctx.state.get_diff(tile, bel, pin, "1.CLKFX");
        assert_eq!(diff, diff_fb);
        assert_eq!(diff, diff_fx);
        let diff = diff.combine(&!&en_dll);
        ctx.tiledb.insert(tile, bel, pin, xlat_bit(diff));
    }
    for pin in ["CLKFX", "CLKFX180", "CONCUR"] {
        let diff = ctx.state.get_diff(tile, bel, pin, "1");
        let diff_fb = ctx.state.get_diff(tile, bel, pin, "1.CLKFB");
        let diff_fb = diff_fb.combine(&!&diff);
        let diff = diff.combine(&!&en_dfs);
        ctx.tiledb.insert(tile, bel, pin, xlat_bit(diff));
        ctx.tiledb
            .insert(tile, bel, "DFS_FEEDBACK", xlat_bit(diff_fb));
    }
    ctx.tiledb.insert(tile, bel, "DLL_ENABLE", xlat_bit(en_dll));
    ctx.tiledb.insert(tile, bel, "DFS_ENABLE", xlat_bit(en_dfs));
    ctx.collect_bit(tile, bel, "CLKFB", "1");

    ctx.collect_bit(tile, bel, "CLKIN_IOB", "1");
    ctx.collect_bit(tile, bel, "CLKFB_IOB", "1");

    ctx.collect_bitvec(tile, bel, "CLKFX_MULTIPLY", "");
    ctx.collect_bitvec(tile, bel, "CLKFX_DIVIDE", "");
    ctx.collect_bitvec(tile, bel, "DESKEW_ADJUST", "");
    ctx.collect_bitvec(tile, bel, "PHASE_SHIFT", "");
    let mut diff = ctx
        .state
        .get_diff(tile, bel, "PHASE_SHIFT", "-255.VARIABLE");
    diff.apply_bitvec_diff(
        ctx.tiledb.item(tile, bel, "PHASE_SHIFT"),
        &bitvec![1; 8],
        &bitvec![0; 8],
    );
    ctx.tiledb
        .insert(tile, bel, "PHASE_SHIFT_NEGATIVE", xlat_bit(diff));

    ctx.state
        .get_diff(tile, bel, "CLKOUT_PHASE_SHIFT", "NONE")
        .assert_empty();
    let fixed = ctx.state.get_diff(tile, bel, "CLKOUT_PHASE_SHIFT", "FIXED");
    let fixed_n = ctx
        .state
        .get_diff(tile, bel, "CLKOUT_PHASE_SHIFT", "FIXED.NEG");
    let variable = ctx
        .state
        .get_diff(tile, bel, "CLKOUT_PHASE_SHIFT", "VARIABLE");
    let variable_n = ctx
        .state
        .get_diff(tile, bel, "CLKOUT_PHASE_SHIFT", "VARIABLE.NEG");
    assert_eq!(variable, variable_n);
    let fixed_n = fixed_n.combine(&!&fixed);
    let (fixed, variable, en_ps) = Diff::split(fixed, variable);
    ctx.tiledb.insert(tile, bel, "PS_ENABLE", xlat_bit(en_ps));
    ctx.tiledb
        .insert(tile, bel, "PS_CENTERED", xlat_bool(fixed, variable));

    let mut diff = ctx.state.get_diff(tile, bel, "PHASE_SHIFT", "-255.FIXED");
    diff.apply_bitvec_diff(
        ctx.tiledb.item(tile, bel, "PHASE_SHIFT"),
        &bitvec![1; 8],
        &bitvec![0; 8],
    );
    diff.apply_bit_diff(
        ctx.tiledb.item(tile, bel, "PHASE_SHIFT_NEGATIVE"),
        true,
        false,
    );
    assert_eq!(diff, fixed_n);
    if edev.grid.kind != GridKind::Virtex2 {
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "RESET_PS_SEL"), true, false);
    }
    ctx.tiledb.insert(
        tile,
        bel,
        "PS_MODE",
        xlat_enum(vec![("CLKFB", diff), ("CLKIN", Diff::default())]),
    );

    ctx.collect_enum_bool_wide(tile, bel, "DUTY_CYCLE_CORRECTION", "FALSE", "TRUE");
    ctx.collect_bit(tile, bel, "STARTUP_WAIT", "STARTUP_WAIT");
    ctx.collect_bit(tile, bel, "CLKIN_DIVIDE_BY_2", "CLKIN_DIVIDE_BY_2");
    ctx.collect_enum(tile, bel, "CLK_FEEDBACK", &["1X", "2X"]);
    ctx.collect_enum(tile, bel, "DFS_FREQUENCY_MODE", &["LOW", "HIGH"]);
    let low = ctx.state.get_diff(tile, bel, "DLL_FREQUENCY_MODE", "LOW");
    let mut high = ctx.state.get_diff(tile, bel, "DLL_FREQUENCY_MODE", "HIGH");
    if edev.grid.kind.is_virtex2p() {
        high.apply_bit_diff(ctx.tiledb.item(tile, bel, "ZD2_BY1"), true, false);
    }
    ctx.tiledb.insert(
        tile,
        bel,
        "DLL_FREQUENCY_MODE",
        xlat_enum(vec![("LOW", low), ("HIGH", high)]),
    );

    let mut jf1 = ctx.extract_enum(
        tile,
        bel,
        "FACTORY_JF1",
        &[
            "0X80", "0XC0", "0XE0", "0XF0", "0XF8", "0XFC", "0XFE", "0XFF",
        ],
    );
    jf1.bits.reverse();
    assert_eq!(jf1.bits, dlls.bits[8..15]);
    let mut jf2 = ctx.extract_enum(
        tile,
        bel,
        "FACTORY_JF2",
        &[
            "0X80", "0XC0", "0XE0", "0XF0", "0XF8", "0XFC", "0XFE", "0XFF",
        ],
    );
    jf2.bits.reverse();
    assert_eq!(jf2.bits, dlls.bits[0..7]);
    assert_eq!(jf2.kind, jf1.kind);
    let TileItemKind::Enum { values } = jf2.kind else {
        unreachable!()
    };
    assert_eq!(values["0X80"], bitvec![0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(values["0XC0"], bitvec![1, 0, 0, 0, 0, 0, 0]);
    assert_eq!(values["0XE0"], bitvec![1, 1, 0, 0, 0, 0, 0]);
    assert_eq!(values["0XF0"], bitvec![1, 1, 1, 0, 0, 0, 0]);
    assert_eq!(values["0XF8"], bitvec![1, 1, 1, 1, 0, 0, 0]);
    assert_eq!(values["0XFC"], bitvec![1, 1, 1, 1, 1, 0, 0]);
    assert_eq!(values["0XFE"], bitvec![1, 1, 1, 1, 1, 1, 0]);
    assert_eq!(values["0XFF"], bitvec![1, 1, 1, 1, 1, 1, 1]);
    jf1.bits.push(dlls.bits[15]);
    jf2.bits.push(dlls.bits[7]);
    jf1.kind = TileItemKind::BitVec {
        invert: BitVec::repeat(false, 8),
    };
    jf2.kind = TileItemKind::BitVec {
        invert: BitVec::repeat(false, 8),
    };
    ctx.tiledb.insert(tile, bel, "FACTORY_JF1", jf1);
    ctx.tiledb.insert(tile, bel, "FACTORY_JF2", jf2);

    for (attr, bits) in [
        ("CLKDV_COUNT_MAX", &dllc.bits[4..8]),
        ("CLKDV_COUNT_FALL", &dllc.bits[8..12]),
        ("CLKDV_COUNT_FALL_2", &dllc.bits[12..16]),
        ("CLKDV_PHASE_RISE", &dllc.bits[16..18]),
        ("CLKDV_PHASE_FALL", &dllc.bits[18..20]),
    ] {
        ctx.tiledb.insert(
            tile,
            bel,
            attr,
            TileItem {
                bits: bits.to_vec(),
                kind: TileItemKind::BitVec {
                    invert: bitvec![0; bits.len()],
                },
            },
        );
    }
    ctx.tiledb.insert(
        tile,
        bel,
        "CLKDV_MODE",
        TileItem {
            bits: dllc.bits[20..21].to_vec(),
            kind: TileItemKind::Enum {
                values: BTreeMap::from_iter([
                    ("HALF".to_string(), bitvec![0]),
                    ("INT".to_string(), bitvec![1]),
                ]),
            },
        },
    );

    let clkdv_count_max = ctx.tiledb.item(tile, bel, "CLKDV_COUNT_MAX");
    let clkdv_count_fall = ctx.tiledb.item(tile, bel, "CLKDV_COUNT_FALL");
    let clkdv_count_fall_2 = ctx.tiledb.item(tile, bel, "CLKDV_COUNT_FALL_2");
    let clkdv_phase_fall = ctx.tiledb.item(tile, bel, "CLKDV_PHASE_FALL");
    let clkdv_mode = ctx.tiledb.item(tile, bel, "CLKDV_MODE");
    for i in 2..=16 {
        let mut diff = ctx
            .state
            .get_diff(tile, bel, "CLKDV_DIVIDE", format!("{i}"));
        diff.apply_bitvec_diff_int(clkdv_count_max, i - 1, 1);
        diff.apply_bitvec_diff_int(clkdv_count_fall, (i - 1) / 2, 0);
        diff.apply_bitvec_diff_int(clkdv_phase_fall, (i % 2) * 2, 0);
        diff.assert_empty();
    }
    for i in 1..=7 {
        let mut diff = ctx
            .state
            .get_diff(tile, bel, "CLKDV_DIVIDE", format!("{i}_5.LOW"));
        diff.apply_enum_diff(clkdv_mode, "HALF", "INT");
        diff.apply_bitvec_diff_int(clkdv_count_max, 2 * i, 1);
        diff.apply_bitvec_diff_int(clkdv_count_fall, i / 2, 0);
        diff.apply_bitvec_diff_int(clkdv_count_fall_2, 3 * i / 2 + 1, 0);
        diff.apply_bitvec_diff_int(clkdv_phase_fall, (i % 2) * 2 + 1, 0);
        diff.assert_empty();
        let mut diff = ctx
            .state
            .get_diff(tile, bel, "CLKDV_DIVIDE", format!("{i}_5.HIGH"));
        diff.apply_enum_diff(clkdv_mode, "HALF", "INT");
        diff.apply_bitvec_diff_int(clkdv_count_max, 2 * i, 1);
        diff.apply_bitvec_diff_int(clkdv_count_fall, (i - 1) / 2, 0);
        diff.apply_bitvec_diff_int(clkdv_count_fall_2, (3 * i + 1) / 2, 0);
        diff.apply_bitvec_diff_int(clkdv_phase_fall, (i % 2) * 2, 0);
        diff.assert_empty();
    }

    if edev.grid.kind.is_virtex2() {
        ctx.state
            .get_diff(tile, bel, "DSS_MODE", "NONE")
            .assert_empty();
        let mut dss_base = ctx
            .state
            .peek_diff(tile, bel, "DSS_MODE", "SPREAD_2")
            .clone();
        let mut diffs = vec![];
        for val in ["SPREAD_2", "SPREAD_4", "SPREAD_6", "SPREAD_8"] {
            diffs.push((
                val,
                ctx.state
                    .get_diff(tile, bel, "DSS_MODE", val)
                    .combine(&!&dss_base),
            ));
        }
        ctx.tiledb.insert(tile, bel, "DSS_MODE", xlat_enum(diffs));
        dss_base.apply_bit_diff(ctx.tiledb.item(tile, bel, "PS_ENABLE"), true, false);
        dss_base.apply_bit_diff(ctx.tiledb.item(tile, bel, "PS_CENTERED"), true, false);
        ctx.tiledb
            .insert(tile, bel, "DSS_ENABLE", xlat_bit(dss_base));
    } else {
        for val in ["NONE", "SPREAD_2", "SPREAD_4", "SPREAD_6", "SPREAD_8"] {
            ctx.state
                .get_diff(tile, bel, "DSS_MODE", val)
                .assert_empty();
        }
    }

    ctx.tiledb.insert(tile, bel, "DLLC", dllc);
    ctx.tiledb.insert(tile, bel, "DLLS", dlls);
    ctx.tiledb.insert(tile, bel, "DFS", dfs);
    ctx.tiledb.insert(tile, bel, "COM", com);
    ctx.tiledb.insert(tile, bel, "MISC", misc);

    present.apply_bitvec_diff(
        ctx.tiledb.item(tile, bel, "FACTORY_JF2"),
        &bitvec![0, 0, 0, 0, 0, 0, 0, 1],
        &bitvec![0, 0, 0, 0, 0, 0, 0, 0],
    );
    present.apply_bitvec_diff(
        ctx.tiledb.item(tile, bel, "FACTORY_JF1"),
        &bitvec![0, 0, 0, 0, 0, 0, 1, 1],
        &bitvec![0, 0, 0, 0, 0, 0, 0, 0],
    );
    let vbg_sel = extract_bitvec_val_part(
        ctx.tiledb.item(tile, bel, "VBG_SEL"),
        &bitvec![0; 3],
        &mut present,
    );
    let vbg_pd = extract_bitvec_val_part(
        ctx.tiledb.item(tile, bel, "VBG_PD"),
        &bitvec![0; 2],
        &mut present,
    );
    ctx.tiledb
        .insert_device_data(&ctx.device.name, "DCM:VBG_SEL", vbg_sel);
    ctx.tiledb
        .insert_device_data(&ctx.device.name, "DCM:VBG_PD", vbg_pd);
    for attr in ["CLKFX_MULTIPLY", "CLKFX_DIVIDE"] {
        present.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, attr),
            &bitvec![1; 12],
            &bitvec![0; 12],
        );
    }

    let item = ctx.tiledb.item(tile, bel, "DESKEW_ADJUST");
    let val = extract_bitvec_val(
        item,
        &bitvec![0; 4],
        present.split_bits(&item.bits.iter().copied().collect()),
    );
    ctx.tiledb
        .insert_device_data(&ctx.device.name, "DCM:DESKEW_ADJUST", val);

    present.apply_bitvec_diff(
        ctx.tiledb.item(tile, bel, "DUTY_CYCLE_CORRECTION"),
        &bitvec![1; 4],
        &bitvec![0; 4],
    );

    if edev.grid.kind.is_virtex2() {
        present.apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.DSSEN"), false, true);
        present.apply_bit_diff(ctx.tiledb.item(tile, bel, "EN_OSC_COARSE"), true, false);
        present.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, "EN_DUMMY_OSC"),
            &bitvec![1, 1, 1],
            &bitvec![0, 0, 0],
        );
        present.apply_bit_diff(
            ctx.tiledb.item(tile, bel, "EN_DUMMY_OSC_OR_NON_STOP"),
            true,
            false,
        );
        if !edev.grid.kind.is_virtex2p() {
            present.apply_bit_diff(ctx.tiledb.item(tile, bel, "ZD2_BY1"), true, false);
        }
    } else {
        present.apply_bit_diff(ctx.tiledb.item(tile, bel, "EN_PWCTL"), true, false);
        present.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, "SEL_HSYNC_B"),
            &bitvec![0, 1],
            &bitvec![0, 0],
        );
        present.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, "CFG_DLL_PS"),
            &bitvec![0, 1, 1, 0, 1, 0, 0, 1, 0],
            &bitvec![0; 9],
        );
        present.apply_bit_diff(ctx.tiledb.item(tile, bel, "ZD1_BY1"), true, false);
        present.apply_bit_diff(ctx.tiledb.item(tile, bel, "ZD2_BY1"), true, false);
        present.apply_bit_diff(ctx.tiledb.item(tile, bel, "EN_DUMMY_OSC"), true, false);
    }
    present.discard_bits(ctx.tiledb.item(tile, bel, "PS_MODE"));
    if edev.grid.kind == GridKind::Spartan3 {
        present.apply_bit_diff(ctx.tiledb.item(tile, bel, "PS_CENTERED"), true, false);
    }
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKDV_COUNT_MAX"), 1, 0);
    present.apply_enum_diff(ctx.tiledb.item(tile, bel, "CLKDV_MODE"), "INT", "HALF");

    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "MISC"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DFS"), 1 << 26, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "COM"), 0x800a0a, 0);

    present.assert_empty();

    if edev.grid.kind == GridKind::Spartan3 {
        ctx.collect_bit("LL.S3", "MISC", "DCM_ENABLE", "1");
        ctx.collect_bit("UL.S3", "MISC", "DCM_ENABLE", "1");
        if edev.grid.columns[edev.grid.columns.last_id().unwrap() - 3].kind == ColumnKind::Bram {
            ctx.collect_bit("LR.S3", "MISC", "DCM_ENABLE", "1");
            ctx.collect_bit("UR.S3", "MISC", "DCM_ENABLE", "1");
        }
    }
}
