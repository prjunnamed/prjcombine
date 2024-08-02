use std::collections::{BTreeMap, HashSet};

use bitvec::prelude::*;

use prjcombine_hammer::Session;
use prjcombine_types::{TileItem, TileItemKind};
use prjcombine_virtex2::grid::Dcms;
use prjcombine_xilinx_geom::ExpandedDevice;

use crate::{
    backend::{IseBackend, PinFromKind},
    diff::{extract_bitvec_val, xlat_bitvec, CollectorCtx, Diff},
    fgen::{ExtraFeature, TileBits},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_multi, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    for (tile, vreg) in [
        ("DCM.S3E.BL", Some("DCM.S3E.BR")),
        ("DCM.S3E.BR", None),
        ("DCM.S3E.TL", Some("DCM.S3E.TR")),
        ("DCM.S3E.TR", None),
        ("DCM.S3E.LB", Some("DCM.S3E.LT")),
        ("DCM.S3E.LT", None),
        ("DCM.S3E.RB", None),
        ("DCM.S3E.RT", Some("DCM.S3E.RB")),
    ] {
        let vreg_tile = vreg.unwrap_or(tile);
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile, "DCM", TileBits::Dcm) else {
            continue;
        };

        let extras = if let Some(vreg_tile) = vreg {
            vec![ExtraFeature::new(
                crate::fgen::ExtraFeatureKind::DcmVreg,
                vreg_tile,
                "DCM_VREG",
                "ENABLE",
                "1",
            )]
        } else {
            vec![]
        };
        fuzz_one_extras!(ctx, "ENABLE", "1", [
            (global_mutex "DCM", "ENABLE"),
            (global_mutex vreg_tile, tile)
        ], [
            (mode "DCM")
        ], extras.clone());

        if vreg.is_none() {
            fuzz_one!(ctx, "ENABLE", "OPT_BASE", [
                (global_mutex "DCM", "ENABLE_OPT"),
                (global_opt "VBG_SEL0", "0"),
                (global_opt "VBG_SEL1", "0"),
                (global_opt "VBG_SEL2", "0"),
                (global_opt "VBG_SEL3", "0")
            ], [
                (mode "DCM")
            ]);
            for opt in ["VBG_SEL0", "VBG_SEL1", "VBG_SEL2", "VBG_SEL3"] {
                fuzz_one!(ctx, "ENABLE", opt, [
                    (global_mutex "DCM", "ENABLE_OPT"),
                    (global_opt "VBG_SEL0", if opt == "VBG_SEL0" {"1"} else {"0"}),
                    (global_opt "VBG_SEL1", if opt == "VBG_SEL1" {"1"} else {"0"}),
                    (global_opt "VBG_SEL2", if opt == "VBG_SEL2" {"1"} else {"0"}),
                    (global_opt "VBG_SEL3", if opt == "VBG_SEL3" {"1"} else {"0"})
                ], [
                    (mode "DCM")
                ]);
            }
        }

        fuzz_multi!(ctx, "DLL_C", "", 32, [
            (global_mutex "DCM", "CFG"),
            (mode "DCM")
        ], (global_xy_bin "CFG_DLL_C_*"));
        fuzz_multi!(ctx, "DLL_S", "", 32, [
            (global_mutex "DCM", "CFG"),
            (mode "DCM")
        ], (global_xy_bin "CFG_DLL_S_*"));
        fuzz_multi!(ctx, "DFS_C", "", 12, [
            (global_mutex "DCM", "CFG"),
            (mode "DCM")
        ], (global_xy_bin "CFG_DFS_C_*"));
        fuzz_multi!(ctx, "DFS_S", "", 76, [
            (global_mutex "DCM", "CFG"),
            (mode "DCM")
        ], (global_xy_bin "CFG_DFS_S_*"));
        fuzz_multi!(ctx, "INTERFACE", "", 16, [
            (global_mutex "DCM", "CFG"),
            (mode "DCM")
        ], (global_xy_bin "CFG_INTERFACE_*"));
        if vreg.is_none() {
            fuzz_multi!(ctx, "VREG", "", 36, [
                (global_mutex "DCM", "CFG"),
                (mode "DCM")
            ], (global_xy_bin "CFG_REG_*"));
        }
        for pin in [
            "RST",
            "PSCLK",
            "PSEN",
            "PSINCDEC",
            "DSSEN",
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
            fuzz_inv!(ctx, pin, [
                (global_mutex "DCM", "USE"),
                (global_mutex "PSCLK", "DCM"),
                (mode "DCM")
            ]);
        }

        for pin in [
            "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX", "CLKFX180",
            "CONCUR",
        ] {
            fuzz_one!(ctx, pin, "1", [
                (mode "DCM"),
                (global_mutex "DCM", "PINS"),
                (mutex "PIN", pin),
                (nopin "CLKFB")
            ], [
                (pin pin)
            ]);
            fuzz_one!(ctx, pin, "1.CLKFB", [
                (mode "DCM"),
                (global_mutex "DCM", "PINS"),
                (mutex "PIN", pin),
                (pin "CLKFB")
            ], [
                (pin pin)
            ]);
            if pin != "CLKFX" && pin != "CLKFX180" && pin != "CONCUR" {
                fuzz_one!(ctx, pin, "1.CLKFX", [
                    (mode "DCM"),
                    (global_mutex "DCM", "PINS"),
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
            (global_mutex "DCM", "PINS")
        ], [
            (pin "CLKFB")
        ]);
        fuzz_one!(ctx, "CLKIN_IOB", "1", [
            (mode "DCM"),
            (global_mutex "DCM", "USE"),
            (pin "CLKIN"),
            (pin "CLKFB"),
            (pin_from "CLKFB", PinFromKind::Bufg)
        ], [
            (pin_from "CLKIN", PinFromKind::Bufg, PinFromKind::Iob)
        ]);
        fuzz_one!(ctx, "CLKFB_IOB", "1", [
            (mode "DCM"),
            (global_mutex "DCM", "USE"),
            (pin "CLKIN"),
            (pin "CLKFB"),
            (pin_from "CLKIN", PinFromKind::Bufg)
        ], [
            (pin_from "CLKFB", PinFromKind::Bufg, PinFromKind::Iob)
        ]);

        fuzz_enum!(ctx, "DLL_FREQUENCY_MODE", ["LOW", "HIGH"], [
            (mode "DCM"),
            (global_mutex "DCM", "USE")
        ]);
        fuzz_enum!(ctx, "DFS_FREQUENCY_MODE", ["LOW", "HIGH"], [
            (mode "DCM"),
            (global_mutex "DCM", "USE")
        ]);
        fuzz_enum!(ctx, "STARTUP_WAIT", ["STARTUP_WAIT"], [
            (mode "DCM"),
            (global_mutex "DCM", "USE"),
            (global_opt "GTS_CYCLE", "1"),
            (global_opt "DONE_CYCLE", "1"),
            (global_opt "LCK_CYCLE", "NOWAIT")
        ]);
        fuzz_enum!(ctx, "DUTY_CYCLE_CORRECTION", ["FALSE", "TRUE"], [
            (mode "DCM"),
            (global_mutex "DCM", "USE")
        ]);
        fuzz_multi!(ctx, "DESKEW_ADJUST", "", 4, [
            (mode "DCM"),
            (global_mutex "DCM", "USE")
        ], (attr_dec "DESKEW_ADJUST"));
        fuzz_enum!(ctx, "CLKIN_DIVIDE_BY_2", ["CLKIN_DIVIDE_BY_2"], [
            (mode "DCM"),
            (global_mutex "DCM", "USE")
        ]);
        fuzz_enum!(ctx, "CLK_FEEDBACK", ["1X", "2X"], [
            (mode "DCM"),
            (global_mutex "DCM", "USE")
        ]);
        fuzz_multi!(ctx, "CLKFX_MULTIPLY", "", 8, [
            (mode "DCM"),
            (global_mutex "DCM", "USE")
        ], (attr_dec_delta "CLKFX_MULTIPLY", 1));
        fuzz_multi!(ctx, "CLKFX_DIVIDE", "", 8, [
            (mode "DCM"),
            (global_mutex "DCM", "USE")
        ], (attr_dec_delta "CLKFX_DIVIDE", 1));
        fuzz_one!(ctx, "VERY_HIGH_FREQUENCY", "1", [
            (mode "DCM"),
            (global_mutex "DCM", "USE"),
            (pin "CLK0"),
            (nopin "CLKFB")
        ], [
            (attr "VERY_HIGH_FREQUENCY", "VERY_HIGH_FREQUENCY")
        ]);
        fuzz_one!(ctx, "VERY_HIGH_FREQUENCY", "1.CLKFB", [
            (mode "DCM"),
            (global_mutex "DCM", "USE"),
            (pin "CLK0"),
            (pin "CLKFB")
        ], [
            (attr "VERY_HIGH_FREQUENCY", "VERY_HIGH_FREQUENCY")
        ]);

        fuzz_enum!(ctx, "CLKOUT_PHASE_SHIFT", ["NONE", "FIXED", "VARIABLE"], [
            (mode "DCM"),
            (global_mutex "DCM", "USE"),
            (pin "CLK0")
        ]);
        fuzz_multi!(ctx, "PHASE_SHIFT", "", 7, [
            (mode "DCM"),
            (global_mutex "DCM", "USE")
        ], (attr_dec "PHASE_SHIFT"));
        fuzz_one!(ctx, "PHASE_SHIFT", "-1", [
            (mode "DCM"),
            (global_mutex "DCM", "USE")
        ], [
            (attr "PHASE_SHIFT", "-1")
        ]);
        fuzz_one!(ctx, "PHASE_SHIFT", "-255", [
            (mode "DCM"),
            (global_mutex "DCM", "USE")
        ], [
            (attr "PHASE_SHIFT", "-255")
        ]);

        fuzz_enum!(ctx, "CLKDV_DIVIDE", ["2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16"], [
            (mode "DCM"),
            (global_mutex "DCM", "USE")
        ]);
        for dll_mode in ["LOW", "HIGH"] {
            for val in ["1_5", "2_5", "3_5", "4_5", "5_5", "6_5", "7_5"] {
                fuzz_one!(
                    ctx,
                    "CLKDV_DIVIDE",
                    format!("{val}.{dll_mode}"), [
                        (mode "DCM"),
                        (global_mutex "DCM", "USE"),
                        (attr "DLL_FREQUENCY_MODE", dll_mode),
                        (attr "X_CLKIN_PERIOD", "")
                    ], [
                        (attr "CLKDV_DIVIDE", val)
                    ]
                );
            }
        }
        fuzz_enum!(ctx, "X_CLKIN_PERIOD", ["1.0", "4.99", "5.0", "24.99", "25.0", "200.99"], [
            (mode "DCM"),
            (global_mutex "DCM", "USE"),
            (attr "CLKIN_DIVIDE_BY_2", ""),
            (attr "CLKFX_MULTIPLY", ""),
            (attr "CLKFX_DIVIDE", ""),
            (pin "CLK0"),
            (nopin "CLKFX")
        ]);
        if vreg.is_none() {
            fuzz_one!(ctx, "X_CLKIN_PERIOD", "201.0", [
                (mode "DCM"),
                (global_mutex "DCM", "USE_VREG"),
                (pin "CLK0")
            ], [
                (attr "X_CLKIN_PERIOD", "201.0")
            ]);
        }

        // junk
        ctx.bits = TileBits::Null;
        for pin in [
            "STATUS0", "STATUS1", "STATUS2", "STATUS3", "STATUS4", "STATUS5", "STATUS6", "STATUS7",
        ] {
            fuzz_one!(ctx, pin, "1", [
                (mode "DCM"),
                (global_mutex "DCM", "USE")
            ], [
                (pin pin)
            ]);
        }
        fuzz_enum!(ctx, "DSS_MODE", ["NONE", "SPREAD_2", "SPREAD_4", "SPREAD_6", "SPREAD_8"], [
            (mode "DCM"),
            (global_mutex "DCM", "USE")
        ]);
        fuzz_enum!(ctx, "FACTORY_JF1", ["0X80", "0XC0", "0XE0", "0XF0", "0XF8", "0XFC", "0XFE", "0XFF"], [
            (mode "DCM"),
            (global_mutex "DCM", "USE")
        ]);
        fuzz_enum!(ctx, "FACTORY_JF2", ["0X80", "0XC0", "0XE0", "0XF0", "0XF8", "0XFC", "0XFE", "0XFF"], [
            (mode "DCM"),
            (global_mutex "DCM", "USE")
        ]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex2(edev) = ctx.edev else {
        unreachable!()
    };
    for (tile, vreg) in [
        ("DCM.S3E.BL", Some("DCM.S3E.BR")),
        ("DCM.S3E.BR", None),
        ("DCM.S3E.TL", Some("DCM.S3E.TR")),
        ("DCM.S3E.TR", None),
        ("DCM.S3E.LB", Some("DCM.S3E.LT")),
        ("DCM.S3E.LT", None),
        ("DCM.S3E.RB", None),
        ("DCM.S3E.RT", Some("DCM.S3E.RB")),
    ] {
        let bel = "DCM";
        let node = edev.egrid.db.get_node(tile);
        if edev.egrid.node_index[node].is_empty() {
            continue;
        }
        for pin in [
            "RST",
            "PSEN",
            "PSINCDEC",
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
            ctx.collect_inv(tile, bel, pin);
        }
        ctx.collect_int_inv(&["INT.DCM"], tile, bel, "PSCLK", false);
        ctx.state
            .get_diff(tile, bel, "DSSENINV", "DSSEN")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "DSSENINV", "DSSEN_B")
            .assert_empty();

        let mut present = ctx.state.get_diff(tile, bel, "ENABLE", "1");

        // TODO: VREG ENABLE etc
        if vreg.is_none() {
            let base = ctx.state.get_diff(tile, bel, "ENABLE", "OPT_BASE");
            let mut diffs = vec![];
            for bit in 0..4 {
                diffs.push(
                    ctx.state
                        .get_diff(tile, bel, "ENABLE", format!("VBG_SEL{bit}"))
                        .combine(&!&base),
                );
            }
            ctx.tiledb
                .insert(tile, "DCM_VREG", "VBG_SEL", xlat_bitvec(diffs));

            let mut cfg_vreg = ctx.state.get_diffs(tile, bel, "VREG", "");
            for i in 0..16 {
                cfg_vreg[i].assert_empty();
            }
            let mut cfg_vreg = cfg_vreg.split_off(16);
            cfg_vreg.reverse();
            let vreg_bits: HashSet<_> = cfg_vreg
                .iter()
                .flat_map(|x| x.bits.keys().copied())
                .collect();
            ctx.tiledb
                .insert(tile, "DCM_VREG", "VREG", xlat_bitvec(cfg_vreg));

            let mut vreg_enable = present.split_bits(&vreg_bits);
            if edev.grid.kind.is_spartan3a() || edev.grid.dcms != Some(Dcms::Two) {
                let diff = ctx.state.get_diff(tile, "DCM_VREG", "ENABLE", "1");
                assert_eq!(vreg_enable, diff);
            }

            vreg_enable.apply_bitvec_diff(
                ctx.tiledb.item(tile, "DCM_VREG", "VBG_SEL"),
                &bitvec![0, 1, 0, 1],
                &bitvec![0; 4],
            );

            let mut base_vreg = BitVec::repeat(false, 20);
            base_vreg.set(0, true);
            base_vreg.set(6, true);
            vreg_enable.apply_bitvec_diff(
                ctx.tiledb.item(tile, "DCM_VREG", "VREG"),
                &base_vreg,
                &bitvec![0; 20],
            );

            vreg_enable.assert_empty();
        }

        let mut cfg_dll_c = ctx.state.get_diffs(tile, bel, "DLL_C", "");
        let mut cfg_dll_s = ctx.state.get_diffs(tile, bel, "DLL_S", "");
        let mut cfg_dfs_c = ctx.state.get_diffs(tile, bel, "DFS_C", "");
        let mut cfg_dfs_s = ctx.state.get_diffs(tile, bel, "DFS_S", "");
        let mut cfg_interface = ctx.state.get_diffs(tile, bel, "INTERFACE", "");

        for i in 0..9 {
            cfg_dfs_c[i].assert_empty();
        }
        let mut cfg_dfs_c = cfg_dfs_c.split_off(9);
        cfg_dll_c.reverse();
        cfg_dll_s.reverse();
        cfg_dfs_c.reverse();
        cfg_dfs_s.reverse();
        cfg_interface.reverse();
        let cfg_dll_c = xlat_bitvec(cfg_dll_c);
        let cfg_dll_s = xlat_bitvec(cfg_dll_s);
        let cfg_dfs_c = xlat_bitvec(cfg_dfs_c);
        let cfg_dfs_s = xlat_bitvec(cfg_dfs_s);
        let cfg_interface = xlat_bitvec(cfg_interface);

        ctx.collect_enum_bool(tile, bel, "DUTY_CYCLE_CORRECTION", "FALSE", "TRUE");
        ctx.collect_bit(tile, bel, "STARTUP_WAIT", "STARTUP_WAIT");
        ctx.collect_bit(tile, bel, "CLKIN_DIVIDE_BY_2", "CLKIN_DIVIDE_BY_2");
        ctx.collect_enum(tile, bel, "CLK_FEEDBACK", &["1X", "2X"]);
        ctx.collect_enum(tile, bel, "DLL_FREQUENCY_MODE", &["LOW", "HIGH"]);
        ctx.collect_enum(tile, bel, "DFS_FREQUENCY_MODE", &["LOW", "HIGH"]);
        ctx.collect_bitvec(tile, bel, "DESKEW_ADJUST", "");
        ctx.collect_bitvec(tile, bel, "CLKFX_MULTIPLY", "");
        ctx.collect_bitvec(tile, bel, "CLKFX_DIVIDE", "");
        ctx.collect_bit(tile, bel, "CLKIN_IOB", "1");
        ctx.collect_bit(tile, bel, "CLKFB_IOB", "1");

        ctx.state.get_diff(tile, bel, "CLKFB", "1").assert_empty();

        for pin in [
            "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV",
        ] {
            let diff = ctx.state.get_diff(tile, bel, pin, "1");
            let diff_fb = ctx.state.get_diff(tile, bel, pin, "1.CLKFB");
            let diff_fx = ctx.state.get_diff(tile, bel, pin, "1.CLKFX");
            let diff_fx = diff_fx.combine(&!&diff_fb);
            let diff_fb = diff_fb.combine(&!&diff);
            ctx.tiledb.insert(tile, bel, pin, xlat_bitvec(vec![diff]));
            ctx.tiledb
                .insert(tile, bel, "DLL_ENABLE", xlat_bitvec(vec![diff_fb]));
            ctx.tiledb
                .insert(tile, bel, "DFS_FEEDBACK", xlat_bitvec(vec![diff_fx]));
        }

        ctx.state
            .get_diff(tile, bel, "VERY_HIGH_FREQUENCY", "1")
            .assert_empty();
        let diff = ctx
            .state
            .get_diff(tile, bel, "VERY_HIGH_FREQUENCY", "1.CLKFB");
        ctx.tiledb
            .insert(tile, bel, "DLL_ENABLE", xlat_bitvec(vec![!diff]));

        let (_, _, dfs_en) = Diff::split(
            ctx.state.peek_diff(tile, bel, "CLKFX", "1").clone(),
            ctx.state.peek_diff(tile, bel, "CONCUR", "1").clone(),
        );
        for pin in ["CLKFX", "CLKFX180", "CONCUR"] {
            let diff = ctx.state.get_diff(tile, bel, pin, "1");
            let diff_fb = ctx.state.get_diff(tile, bel, pin, "1.CLKFB");
            assert_eq!(diff, diff_fb);
            let diff = diff.combine(&!&dfs_en);
            let pin = if pin == "CONCUR" { pin } else { "CLKFX" };
            ctx.tiledb.insert(tile, bel, pin, xlat_bitvec(vec![diff]));
        }
        ctx.tiledb
            .insert(tile, bel, "DFS_ENABLE", xlat_bitvec(vec![dfs_en]));

        let item = ctx.tiledb.item(tile, bel, "DESKEW_ADJUST");
        let val = extract_bitvec_val(
            item,
            &bitvec![0; 4],
            present.split_bits(&item.bits.iter().copied().collect()),
        );
        ctx.tiledb
            .insert_device_data(&ctx.device.name, "DCM:DESKEW_ADJUST", val);

        let mut diffs = vec![ctx.state.get_diff(tile, bel, "PHASE_SHIFT", "-255")];
        diffs.extend(ctx.state.get_diffs(tile, bel, "PHASE_SHIFT", ""));
        let item = xlat_bitvec(diffs);
        let mut diff = ctx.state.get_diff(tile, bel, "PHASE_SHIFT", "-1");
        diff.apply_bitvec_diff_int(&item, 2, 0);
        ctx.tiledb.insert(tile, bel, "PHASE_SHIFT", item);
        ctx.tiledb
            .insert(tile, bel, "PHASE_SHIFT_NEGATIVE", xlat_bitvec(vec![diff]));

        ctx.state
            .get_diff(tile, bel, "CLKOUT_PHASE_SHIFT", "NONE")
            .assert_empty();
        let diff_f = ctx.state.get_diff(tile, bel, "CLKOUT_PHASE_SHIFT", "FIXED");
        let diff_v = ctx
            .state
            .get_diff(tile, bel, "CLKOUT_PHASE_SHIFT", "VARIABLE");
        let diff_v = diff_v.combine(&!&diff_f);
        ctx.tiledb
            .insert(tile, bel, "PS_ENABLE", xlat_bitvec(vec![diff_f]));
        ctx.tiledb
            .insert(tile, bel, "PS_VARIABLE", xlat_bitvec(vec![diff_v]));

        for (attr, bits) in [
            ("CLKDV_COUNT_MAX", &cfg_dll_c.bits[1..5]),
            ("CLKDV_COUNT_FALL", &cfg_dll_c.bits[5..9]),
            ("CLKDV_COUNT_FALL_2", &cfg_dll_c.bits[9..13]),
            ("CLKDV_PHASE_RISE", &cfg_dll_c.bits[13..15]),
            ("CLKDV_PHASE_FALL", &cfg_dll_c.bits[15..17]),
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
                bits: cfg_dll_c.bits[17..18].to_vec(),
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

        ctx.state
            .get_diff(tile, bel, "X_CLKIN_PERIOD", "1.0")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "X_CLKIN_PERIOD", "4.99")
            .assert_empty();
        let diff_a = ctx.state.get_diff(tile, bel, "X_CLKIN_PERIOD", "5.0");
        assert_eq!(
            diff_a,
            ctx.state.get_diff(tile, bel, "X_CLKIN_PERIOD", "24.99")
        );
        let diff_b = ctx.state.get_diff(tile, bel, "X_CLKIN_PERIOD", "25.0");
        assert_eq!(
            diff_b,
            ctx.state.get_diff(tile, bel, "X_CLKIN_PERIOD", "200.99")
        );
        if vreg.is_none() {
            let diff_c = ctx.state.get_diff(tile, bel, "X_CLKIN_PERIOD", "201.0");
            let mut diff_c = diff_c.combine(&!&diff_b);
            diff_c.apply_bitvec_diff(
                ctx.tiledb.item(tile, "DCM_VREG", "VBG_SEL"),
                &bitvec![0, 1, 1, 0],
                &bitvec![0, 1, 0, 1],
            );
            diff_c.assert_empty();
        }
        let mut diff_b = diff_b.combine(&!&diff_a);
        ctx.tiledb
            .insert(tile, bel, "PERIOD_NOT_HF", xlat_bitvec(vec![!diff_a]));
        ctx.tiledb.insert(
            tile,
            bel,
            "PERIOD_LF",
            TileItem {
                bits: vec![cfg_dll_s.bits[7], cfg_dll_s.bits[17]],
                kind: TileItemKind::BitVec {
                    invert: bitvec![0; 2],
                },
            },
        );
        diff_b.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, "PERIOD_LF"),
            &bitvec![1; 2],
            &bitvec![0; 2],
        );
        diff_b.assert_empty();

        ctx.tiledb.insert(tile, bel, "DLL_C", cfg_dll_c);
        ctx.tiledb.insert(tile, bel, "DLL_S", cfg_dll_s);
        ctx.tiledb.insert(tile, bel, "DFS_C", cfg_dfs_c);
        ctx.tiledb.insert(tile, bel, "DFS_S", cfg_dfs_s);
        ctx.tiledb.insert(tile, bel, "INTERFACE", cfg_interface);

        present.apply_bit_diff(
            ctx.tiledb.item(tile, bel, "DUTY_CYCLE_CORRECTION"),
            true,
            false,
        );
        present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKDV_COUNT_MAX"), 1, 0);
        present.apply_enum_diff(ctx.tiledb.item(tile, bel, "CLKDV_MODE"), "INT", "HALF");
        present.apply_bit_diff(ctx.tiledb.item(tile, bel, "PERIOD_NOT_HF"), true, false);

        let mut base_interface = BitVec::repeat(false, 16);
        base_interface.set(9, true);
        base_interface.set(10, true);
        base_interface.set(13, true);
        present.apply_bitvec_diff(
            ctx.tiledb.item(tile, "DCM", "INTERFACE"),
            &base_interface,
            &bitvec![0; 16],
        );

        let mut base_dfs_s = BitVec::repeat(false, 76);
        base_dfs_s.set(17, true);
        base_dfs_s.set(21, true);
        base_dfs_s.set(32, true);
        base_dfs_s.set(33, true);
        base_dfs_s.set(37, true);
        base_dfs_s.set(41, true);
        base_dfs_s.set(43, true);
        base_dfs_s.set(45, true);
        base_dfs_s.set(52, true);
        base_dfs_s.set(64, true);
        base_dfs_s.set(68, true);
        present.apply_bitvec_diff(
            ctx.tiledb.item(tile, "DCM", "DFS_S"),
            &base_dfs_s,
            &bitvec![0; 76],
        );

        let mut base_dll_s = BitVec::repeat(false, 32);
        base_dll_s.set(0, true);
        base_dll_s.set(6, true);
        present.apply_bitvec_diff(
            ctx.tiledb.item(tile, "DCM", "DLL_S"),
            &base_dll_s,
            &bitvec![0; 32],
        );

        present.assert_empty();
    }
}
