use prjcombine_re_collector::{xlat_bit, xlat_enum_ocd, Diff, OcdMode};
use prjcombine_re_hammer::Session;
use prjcombine_interconnect::db::{BelId, Dir};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits, TileKV, TileRelation},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(
    session: &mut Session<IseBackend<'a>>,
    backend: &IseBackend<'a>,
    bali_only: bool,
) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    let int = backend.egrid.db.get_node("INT");
    let hclk = backend.egrid.db.get_node("HCLK");
    if !bali_only {
        let ctx = FuzzCtx::new(session, backend, "HCLK", "HCLK_L", TileBits::DoubleHclk);
        for i in 6..12 {
            for ud in ['U', 'D'] {
                for j in 0..12 {
                    fuzz_one!(ctx, format!("MUX.LCLK{i}_{ud}"), format!("HCLK{j}"), [
                        (tile_mutex "MODE", "TEST"),
                        (global_mutex "HCLK", "USE"),
                        (tile_mutex format!("MUX.LCLK{i}_{ud}_L"), format!("HCLK{j}")),
                        (tile_mutex format!("HCLK{j}"), format!("MUX.LCLK{i}_{ud}_L")),
                        (related TileRelation::Delta(0, -1, int), (nop)),
                        (related TileRelation::Delta(-2, -1, int), (nop)),
                        (related TileRelation::Delta(2, -1, int), (nop)),
                        (related TileRelation::Delta(0, 1, int), (nop)),
                        (related TileRelation::Delta(-2, 1, int), (nop)),
                        (related TileRelation::Delta(2, 1, int), (nop)),
                        (related TileRelation::Delta(-2, 0, hclk), (tile_mutex "MODE", "PIN_L")),
                        (related TileRelation::Delta(-2, 0, hclk),
                            (pip
                                (pin if j < 8 {
                                    format!("HCLK{j}_I")
                                } else {
                                    format!("HCLK{j}")
                                }),
                                (pin format!("LCLK{i}_{ud}")))),
                        (related TileRelation::Delta(2, 0, hclk), (tile_mutex "MODE", "PIN_R")),
                        (related TileRelation::Delta(2, 0, hclk),
                            (pip
                                (pin if j < 8 {
                                    format!("HCLK{j}_I")
                                } else {
                                    format!("HCLK{j}")
                                }),
                                (pin format!("LCLK{i}_{ud}"))))
                    ], [
                        (pip
                            (pin if j < 8 {
                                format!("HCLK{j}_I")
                            } else {
                                format!("HCLK{j}")
                            }),
                            (pin format!("LCLK{i}_{ud}")))
                    ]);
                }
                for j in 0..4 {
                    fuzz_one!(ctx, format!("MUX.LCLK{i}_{ud}"), format!("RCLK{j}"), [
                        (tile_mutex "MODE", "TEST"),
                        (global_mutex "RCLK", "USE"),
                        (tile_mutex format!("MUX.LCLK{i}_{ud}_L"), format!("RCLK{j}")),
                        (tile_mutex format!("RCLK{j}"), format!("MUX.LCLK{i}_{ud}_L")),
                        (related TileRelation::Delta(0, -1, int), (nop)),
                        (related TileRelation::Delta(-2, -1, int), (nop)),
                        (related TileRelation::Delta(2, -1, int), (nop)),
                        (related TileRelation::Delta(0, 1, int), (nop)),
                        (related TileRelation::Delta(-2, 1, int), (nop)),
                        (related TileRelation::Delta(2, 1, int), (nop)),
                        (related TileRelation::Delta(-2, 0, hclk), (tile_mutex "MODE", "PIN_L")),
                        (related TileRelation::Delta(-2, 0, hclk),
                            (pip
                                (pin format!("RCLK{j}")),
                                (pin format!("LCLK{i}_{ud}")))),
                        (related TileRelation::Delta(2, 0, hclk), (tile_mutex "MODE", "PIN_R")),
                        (related TileRelation::Delta(2, 0, hclk),
                            (pip
                                (pin format!("RCLK{j}")),
                                (pin format!("LCLK{i}_{ud}"))))
                    ], [
                        (pip (pin format!("RCLK{j}")), (pin format!("LCLK{i}_{ud}")))
                    ]);
                }
            }
        }
    }
    if !bali_only {
        let ctx = FuzzCtx::new(session, backend, "HCLK", "HCLK_R", TileBits::DoubleHclk);
        for i in 0..6 {
            for ud in ['U', 'D'] {
                for j in 0..12 {
                    fuzz_one!(ctx, format!("MUX.LCLK{i}_{ud}"), format!("HCLK{j}"), [
                        (tile_mutex "MODE", "TEST"),
                        (global_mutex "HCLK", "USE"),
                        (tile_mutex format!("MUX.LCLK{i}_{ud}_R"), format!("HCLK{j}")),
                        (tile_mutex format!("HCLK{j}"), format!("MUX.LCLK{i}_{ud}_R")),
                        (related TileRelation::Delta(0, -1, int), (nop)),
                        (related TileRelation::Delta(-2, -1, int), (nop)),
                        (related TileRelation::Delta(2, -1, int), (nop)),
                        (related TileRelation::Delta(0, 1, int), (nop)),
                        (related TileRelation::Delta(-2, 1, int), (nop)),
                        (related TileRelation::Delta(2, 1, int), (nop)),
                        (related TileRelation::Delta(-2, 0, hclk), (tile_mutex "MODE", "PIN_L")),
                        (related TileRelation::Delta(-2, 0, hclk),
                            (pip
                                (pin if j < 8 {
                                    format!("HCLK{j}")
                                } else {
                                    format!("HCLK{j}_I")
                                }),
                                (pin format!("LCLK{i}_{ud}")))),
                        (related TileRelation::Delta(2, 0, hclk), (tile_mutex "MODE", "PIN_R")),
                        (related TileRelation::Delta(2, 0, hclk),
                            (pip
                                (pin if j < 8 {
                                    format!("HCLK{j}")
                                } else {
                                    format!("HCLK{j}_I")
                                }),
                                (pin format!("LCLK{i}_{ud}"))))
                    ], [
                        (pip
                            (pin if j < 8 {
                                format!("HCLK{j}")
                            } else {
                                format!("HCLK{j}_I")
                            }),
                            (pin format!("LCLK{i}_{ud}")))
                    ]);
                }
                for j in 0..4 {
                    fuzz_one!(ctx, format!("MUX.LCLK{i}_{ud}"), format!("RCLK{j}"), [
                        (tile_mutex "MODE", "TEST"),
                        (global_mutex "RCLK", "USE"),
                        (tile_mutex format!("MUX.LCLK{i}_{ud}_R"), format!("RCLK{j}")),
                        (tile_mutex format!("RCLK{j}"), format!("MUX.LCLK{i}_{ud}_R")),
                        (related TileRelation::Delta(0, -1, int), (nop)),
                        (related TileRelation::Delta(-2, -1, int), (nop)),
                        (related TileRelation::Delta(2, -1, int), (nop)),
                        (related TileRelation::Delta(0, 1, int), (nop)),
                        (related TileRelation::Delta(-2, 1, int), (nop)),
                        (related TileRelation::Delta(2, 1, int), (nop)),
                        (related TileRelation::Delta(-2, 0, hclk), (tile_mutex "MODE", "PIN_L")),
                        (related TileRelation::Delta(-2, 0, hclk),
                            (pip
                                (pin format!("RCLK{j}_I")),
                                (pin format!("LCLK{i}_{ud}")))),
                        (related TileRelation::Delta(2, 0, hclk), (tile_mutex "MODE", "PIN_R")),
                        (related TileRelation::Delta(2, 0, hclk),
                            (pip
                                (pin format!("RCLK{j}_I")),
                                (pin format!("LCLK{i}_{ud}"))))
                    ], [
                        (pip (pin format!("RCLK{j}_I")), (pin format!("LCLK{i}_{ud}")))
                    ]);
                }
            }
        }
    }
    let clk_bufg_rebuf = backend.egrid.db.get_node("CLK_BUFG_REBUF");
    let clk_bali_rebuf = backend.egrid.db.get_node("CLK_BALI_REBUF");
    if !bali_only {
        for i in 0..16 {
            let ctx = FuzzCtx::new(
                session,
                backend,
                "CLK_BUFG",
                format!("BUFGCTRL{}", i),
                TileBits::MainAuto,
            );
            fuzz_one!(ctx, "ENABLE", "1", [], [(mode "BUFGCTRL")]);
            for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
                fuzz_inv!(ctx, pin, [(mode "BUFGCTRL")]);
            }
            fuzz_enum!(ctx, "PRESELECT_I0", ["FALSE", "TRUE"], [(mode "BUFGCTRL")]);
            fuzz_enum!(ctx, "PRESELECT_I1", ["FALSE", "TRUE"], [(mode "BUFGCTRL")]);
            fuzz_enum!(ctx, "CREATE_EDGE", ["FALSE", "TRUE"], [(mode "BUFGCTRL")]);
            fuzz_enum!(ctx, "INIT_OUT", ["0", "1"], [(mode "BUFGCTRL")]);
            fuzz_one!(ctx, "ENABLE.FB", "1", [
                (tile_mutex "FB", "TEST")
            ], [
                (pip (pin "O"), (pin "FB"))
            ]);
            if edev.grids.first().unwrap().regs == 1 {
                let extras = vec![ExtraFeature::new(
                    ExtraFeatureKind::ClkRebuf(Dir::S, clk_bufg_rebuf),
                    "CLK_BUFG_REBUF",
                    "CLK_REBUF",
                    format!("ENABLE.GCLK{i}_U"),
                    "1",
                )];
                fuzz_one_extras!(ctx, "ENABLE.GCLK", "1", [
                (global_mutex "GCLK", "TEST")
            ], [
                (pip (pin "O"), (pin "GCLK"))
            ], extras);
            } else {
                let extras = vec![
                    ExtraFeature::new(
                        ExtraFeatureKind::ClkRebuf(Dir::S, clk_bufg_rebuf),
                        "CLK_BUFG_REBUF",
                        "CLK_REBUF",
                        format!("ENABLE.GCLK{i}_U"),
                        "1",
                    ),
                    ExtraFeature::new(
                        ExtraFeatureKind::ClkRebuf(Dir::N, clk_bufg_rebuf),
                        "CLK_BUFG_REBUF",
                        "CLK_REBUF",
                        format!("ENABLE.GCLK{i}_D"),
                        "1",
                    ),
                ];
                fuzz_one_extras!(ctx, "ENABLE.GCLK", "1", [
                (global_mutex "GCLK", "TEST"),
                (special TileKV::HclkSide(Dir::N))
            ], [
                (pip (pin "O"), (pin "GCLK"))
            ], extras);
                let extras = vec![
                    ExtraFeature::new(
                        ExtraFeatureKind::ClkRebuf(Dir::S, clk_bufg_rebuf),
                        "CLK_BUFG_REBUF",
                        "CLK_REBUF",
                        format!("ENABLE.GCLK{ii}_U", ii = i + 16),
                        "1",
                    ),
                    ExtraFeature::new(
                        ExtraFeatureKind::ClkRebuf(Dir::N, clk_bufg_rebuf),
                        "CLK_BUFG_REBUF",
                        "CLK_REBUF",
                        format!("ENABLE.GCLK{ii}_D", ii = i + 16),
                        "1",
                    ),
                ];
                fuzz_one_extras!(ctx, "ENABLE.GCLK", "1", [
                (global_mutex "GCLK", "TEST"),
                (special TileKV::HclkSide(Dir::S))
            ], [
                (pip (pin "O"), (pin "GCLK"))
            ], extras);
            }
            // ISE bug causes pips to be reversed?
            fuzz_one!(ctx, "TEST_I0", "1", [
                (mutex "MUX.I0", "FB_TEST")
            ], [
                (pip (pin "FB_TEST0"), (pin "I0"))
            ]);
            fuzz_one!(ctx, "TEST_I1", "1", [
                (mutex "MUX.I1", "FB_TEST")
            ], [
                (pip (pin "FB_TEST1"), (pin "I1"))
            ]);
            for j in 0..2 {
                fuzz_one!(ctx, format!("MUX.I{j}"), "CASCI", [
                    (mutex format!("MUX.I{j}"), "CASCI")
                ], [
                    (pip (pin format!("CASCI{j}")), (pin format!("I{j}")))
                ]);
                for k in 0..2 {
                    fuzz_one!(ctx, format!("MUX.I{j}"), format!("CKINT{k}"), [
                        (mutex format!("MUX.I{j}"), format!("CKINT{k}"))
                    ], [
                        (pip (pin format!("CKINT{k}")), (pin format!("I{j}")))
                    ]);
                }

                let obel_prev = BelId::from_idx((i + 15) % 16);
                fuzz_one!(ctx, format!("MUX.I{j}"), "FB_PREV", [
                    (tile_mutex "FB", "USE"),
                    (mutex format!("MUX.I{j}"), "FB_PREV"),
                    (pip (bel_pin obel_prev, "O"), (bel_pin obel_prev, "FB"))
                ], [
                    (pip (bel_pin obel_prev, "FB"), (pin format!("I{j}")))
                ]);
                let obel_next = BelId::from_idx((i + 1) % 16);
                fuzz_one!(ctx, format!("MUX.I{j}"), "FB_NEXT", [
                    (tile_mutex "FB", "USE"),
                    (mutex format!("MUX.I{j}"), "FB_NEXT"),
                    (pip (bel_pin obel_next, "O"), (bel_pin obel_next, "FB"))
                ], [
                    (pip (bel_pin obel_next, "FB"), (pin format!("I{j}")))
                ]);
            }
        }
    }
    if !bali_only {
        let bel_clk_hrow = BelId::from_idx(58);
        for i in 0..32 {
            let ctx = FuzzCtx::new(
                session,
                backend,
                "CLK_HROW",
                format!("GCLK_TEST_BUF.HROW_GCLK{i}"),
                TileBits::ClkHrow,
            );
            fuzz_one!(ctx, "ENABLE", "1", [], [(mode "GCLK_TEST_BUF")]);
            fuzz_enum!(ctx, "GCLK_TEST_ENABLE", ["FALSE", "TRUE"], [(mode "GCLK_TEST_BUF")]);
            fuzz_enum!(ctx, "INVERT_INPUT", ["FALSE", "TRUE"], [(mode "GCLK_TEST_BUF")]);
        }
        for lr in ['L', 'R'] {
            for i in 0..12 {
                let ctx = FuzzCtx::new(
                    session,
                    backend,
                    "CLK_HROW",
                    format!("BUFHCE_{lr}{i}"),
                    TileBits::ClkHrow,
                );
                fuzz_one!(ctx, "ENABLE", "1", [], [(mode "BUFHCE")]);
                fuzz_inv!(ctx, "CE", [(mode "BUFHCE")]);
                fuzz_enum!(ctx, "INIT_OUT", ["0", "1"], [(mode "BUFHCE")]);
                fuzz_enum!(ctx, "CE_TYPE", ["SYNC", "ASYNC"], [(mode "BUFHCE")]);

                let ckints = if (lr == 'R' && i < 6) || (lr == 'L' && i >= 6) {
                    0..2
                } else {
                    2..4
                };
                for j in ckints {
                    fuzz_one!(ctx, "MUX.I", format!("CKINT{j}"), [
                        (tile_mutex format!("CKINT{j}"), format!("BUFHCE_{lr}{i}")),
                        (mutex "MUX.I", format!("CKINT{j}"))
                    ], [
                        (pip (bel_pin bel_clk_hrow, format!("BUFHCE_CKINT{j}")), (pin "I"))
                    ]);
                }
                for olr in ['L', 'R'] {
                    fuzz_one!(ctx, "MUX.I", format!("HCLK_TEST_{olr}"), [
                        (mutex "MUX.I", format!("HCLK_TEST_{olr}"))
                    ], [
                        (pip (bel_pin bel_clk_hrow, format!("HCLK_TEST_OUT_{olr}")), (pin "I"))
                    ]);
                    for j in 0..14 {
                        fuzz_one!(ctx, "MUX.I", format!("HIN{j}_{olr}"), [
                            (tile_mutex format!("HIN{j}_{olr}"), format!("BUFHCE_{lr}{i}")),
                            (mutex "MUX.I", format!("HIN{j}_{olr}"))
                        ], [
                            (pip (bel_pin bel_clk_hrow, format!("HIN{j}_{olr}")), (pin "I"))
                        ]);
                    }
                }
                for j in 0..32 {
                    let extras = vec![
                        ExtraFeature::new(
                            ExtraFeatureKind::ClkRebuf(Dir::S, clk_bufg_rebuf),
                            "CLK_BUFG_REBUF",
                            "CLK_REBUF",
                            format!("ENABLE.GCLK{j}_U"),
                            "1",
                        ),
                        ExtraFeature::new(
                            ExtraFeatureKind::ClkRebuf(Dir::N, clk_bufg_rebuf),
                            "CLK_BUFG_REBUF",
                            "CLK_REBUF",
                            format!("ENABLE.GCLK{j}_D"),
                            "1",
                        ),
                    ];
                    fuzz_one_extras!(ctx, "MUX.I", format!("GCLK{j}"), [
                    (global_mutex "GCLK", "TEST"),
                    (related TileRelation::Delta(0, -13, clk_bufg_rebuf), (nop)),
                    (related TileRelation::Delta(0, 11, clk_bufg_rebuf), (nop)),
                    (tile_mutex format!("GCLK{j}"), format!("BUFHCE_{lr}{i}")),
                    (mutex "MUX.I", format!("GCLK{j}"))
                ], [
                    (pip (bel_pin bel_clk_hrow, format!("GCLK{j}")), (pin "I"))
                ], extras);
                }
            }
            let ctx = FuzzCtx::new(
                session,
                backend,
                "CLK_HROW",
                format!("GCLK_TEST_BUF.HROW_BUFH_{lr}"),
                TileBits::ClkHrow,
            );
            fuzz_one!(ctx, "ENABLE", "1", [], [(mode "GCLK_TEST_BUF")]);
            fuzz_enum!(ctx, "GCLK_TEST_ENABLE", ["FALSE", "TRUE"], [(mode "GCLK_TEST_BUF")]);
            fuzz_enum!(ctx, "INVERT_INPUT", ["FALSE", "TRUE"], [(mode "GCLK_TEST_BUF")]);
            for j in 0..14 {
                fuzz_one!(ctx, "MUX.I", format!("HIN{j}"), [
                    (tile_mutex format!("HIN{j}_{lr}"), "TEST"),
                    (mutex "MUX.I", format!("HIN{j}"))
                ], [
                    (pip (bel_pin bel_clk_hrow, format!("HIN{j}_{lr}")), (bel_pin bel_clk_hrow, format!("HCLK_TEST_IN_{lr}")))
                ]);
            }
        }
        {
            let ctx = FuzzCtx::new(session, backend, "CLK_HROW", "CLK_HROW", TileBits::ClkHrow);
            let cmt = backend.egrid.db.get_node("CMT");
            let has_lio = backend.egrid.node_index[cmt]
                .iter()
                .any(|loc| loc.1 <= edev.col_clk);
            let has_rio = backend.egrid.node_index[cmt]
                .iter()
                .any(|loc| loc.1 > edev.col_clk);
            for i in 0..32 {
                fuzz_one!(ctx, format!("MUX.CASCO{i}"), "CASCI", [
                    (mutex "CASCO", "CASCO"),
                    (mutex format!("MUX.CASCO{i}"), "CASCI")
                ], [
                    (pip (pin format!("CASCI{i}")), (pin format!("CASCO{i}")))
                ]);
                for j in [i, i ^ 1] {
                    fuzz_one!(ctx, format!("MUX.CASCO{i}"), format!("GCLK_TEST{j}"), [
                        (mutex format!("MUX.CASCO{i}"), format!("GCLK_TEST{j}"))
                    ], [
                        (pip (pin format!("GCLK{j}_TEST_OUT")), (pin format!("GCLK_TEST{i}")))
                    ]);
                }
                for lr in ['L', 'R'] {
                    fuzz_one!(ctx, format!("MUX.CASCO{i}"), format!("HCLK_TEST_{lr}"), [
                        (mutex "CASCO", "CASCO"),
                        (mutex format!("MUX.CASCO{i}"), format!("HCLK_TEST_{lr}"))
                    ], [
                        (pip (pin format!("HCLK_TEST_OUT_{lr}")), (pin format!("CASCO{i}")))
                    ]);
                    for j in 0..14 {
                        fuzz_one!(ctx, format!("MUX.CASCO{i}"), format!("HIN{j}_{lr}"), [
                            (mutex "CASCO", "CASCO"),
                            (tile_mutex format!("HIN{j}_{lr}"), format!("CASCO{i}")),
                            (mutex format!("MUX.CASCO{i}"), format!("HIN{j}_{lr}"))
                        ], [
                            (pip (pin format!("HIN{j}_{lr}")), (pin format!("CASCO{i}")))
                        ]);
                    }
                    for j in 0..4 {
                        fuzz_one!(ctx, format!("MUX.CASCO{i}"), format!("RCLK{j}_{lr}"), [
                            (mutex "CASCO", "CASCO"),
                            (global_mutex "RCLK", "USE"),
                            (mutex format!("MUX.CASCO{i}"), format!("RCLK{j}_{lr}")),
                            (mutex format!("MUX.CASCO{}", i ^ 1), format!("RCLK{j}_{lr}")),
                            (pip (pin format!("RCLK{j}_{lr}")), (pin format!("CASCO{}", i ^ 1)))
                        ], [
                            (pip (pin format!("RCLK{j}_{lr}")), (pin format!("CASCO{i}")))
                        ]);
                    }
                }
                fuzz_one!(ctx, format!("GCLK{i}_TEST_IN"), "1", [
                    (mutex "CASCO", "TEST_IN"),
                    (global_mutex "GCLK", "USE"),
                    (tile_mutex format!("GCLK{i}"), "BUFHCE_L0"),
                    (bel_mutex BelId::from_idx(32), "MUX.I", format!("GCLK{i}")),
                    (pip (pin format!("GCLK{i}")), (bel_pin BelId::from_idx(32), "I"))
                ], [
                    (pip (pin format!("GCLK{i}")), (pin format!("GCLK{i}_TEST_IN")))
                ]);
            }
            for lr in ['L', 'R'] {
                for j in 0..4 {
                    let mut extras = vec![];
                    if lr == 'L' {
                        if has_lio {
                            extras.push(ExtraFeature::new(
                                ExtraFeatureKind::CmtDir(Dir::W),
                                "CMT",
                                "HCLK_CMT",
                                format!("ENABLE.RCLK{j}"),
                                "HROW",
                            ));
                        }
                    } else {
                        if has_rio {
                            extras.push(ExtraFeature::new(
                                ExtraFeatureKind::CmtDir(Dir::E),
                                "CMT",
                                "HCLK_CMT",
                                format!("ENABLE.RCLK{j}"),
                                "HROW",
                            ));
                        }
                    }
                    fuzz_one_extras!(ctx, "MUX.CASCO0", format!("RCLK{j}_{lr}.EXCL"), [
                        (mutex "CASCO", "CASCO"),
                        (global_mutex "RCLK", "TEST_HROW"),
                        (mutex "MUX.CASCO0", format!("RCLK{j}_{lr}"))
                    ], [
                        (pip (pin format!("RCLK{j}_{lr}")), (pin "CASCO0"))
                    ], extras);
                }
            }
        }
    }
    for (tile, bts) in [("CLK_BUFG_REBUF", 2), ("CLK_BALI_REBUF", 16)] {
        let Some(ctx) =
            FuzzCtx::try_new(session, backend, tile, "CLK_REBUF", TileBits::Main(0, bts))
        else {
            continue;
        };
        if tile == "CLK_BUFG_REBUF" && bali_only {
            continue;
        }
        for i in 0..32 {
            let bel_d = BelId::from_idx(i / 2);
            let bel_u = BelId::from_idx(16 + i / 2);
            if i % 2 == 0 {
                if edev.grids.values().any(|grid| grid.regs > 1) {
                    fuzz_one!(ctx, format!("BUF.GCLK{i}_D"), "1", [
                        (global_mutex "GCLK", "REBUF_D0"),
                        (pip (pin format!("GCLK{i}_D")), (bel_pin bel_d, "CLKIN")),
                        (pip (bel_pin bel_u, "CLKOUT"), (pin format!("GCLK{i}_U")))
                    ], [
                        (pip (pin format!("GCLK{i}_U")), (pin format!("GCLK{i}_D")))
                    ]);
                    fuzz_one!(ctx, format!("BUF.GCLK{i}_U"), "1", [
                        (global_mutex "GCLK", "REBUF_U0"),
                        (special TileKV::HclkSide(Dir::S)),
                        (related TileRelation::ClkRebuf(Dir::S),
                            (pip (bel_pin bel_u, "CLKOUT"), (pin format!("GCLK{i}_U")))),
                        (related TileRelation::ClkRebuf(Dir::N),
                            (pip (pin format!("GCLK{i}_D")), (bel_pin bel_d, "CLKIN")))
                    ], [
                        (pip (pin format!("GCLK{i}_D")), (pin format!("GCLK{i}_U")))
                    ]);
                }
                if tile == "CLK_BALI_REBUF" {
                    let extras = vec![ExtraFeature::new(
                        ExtraFeatureKind::ClkRebuf(Dir::S, clk_bali_rebuf),
                        "CLK_BALI_REBUF",
                        "CLK_REBUF",
                        format!("ENABLE.GCLK{i}_U"),
                        "1",
                    )];
                    fuzz_one_extras!(ctx, format!("ENABLE.GCLK{i}_D"), "1", [
                        (global_mutex "GCLK", "REBUF_BALI"),
                        (special TileKV::HclkSide(Dir::S))
                    ], [
                        (pip (pin format!("GCLK{i}_D")), (bel_pin bel_d, "CLKIN"))
                    ], extras);
                }
            } else {
                if edev.grids.values().any(|grid| grid.regs > 1) {
                    fuzz_one!(ctx, format!("BUF.GCLK{i}_U"), "1", [
                        (global_mutex "GCLK", "REBUF_U1"),
                        (pip (pin format!("GCLK{i}_U")), (bel_pin bel_u, "CLKIN")),
                        (pip (bel_pin bel_d, "CLKOUT"), (pin format!("GCLK{i}_D")))
                    ], [
                        (pip (pin format!("GCLK{i}_D")), (pin format!("GCLK{i}_U")))
                    ]);
                    fuzz_one!(ctx, format!("BUF.GCLK{i}_D"), "1", [
                        (global_mutex "GCLK", "REBUF_D1"),
                        (special TileKV::HclkSide(Dir::N)),
                        (related TileRelation::ClkRebuf(Dir::N),
                            (pip (bel_pin bel_d, "CLKOUT"), (pin format!("GCLK{i}_D")))),
                        (related TileRelation::ClkRebuf(Dir::S),
                            (pip (pin format!("GCLK{i}_U")), (bel_pin bel_u, "CLKIN")))
                    ], [
                        (pip (pin format!("GCLK{i}_U")), (pin format!("GCLK{i}_D")))
                    ]);
                }
                if tile == "CLK_BALI_REBUF" {
                    let extras = vec![ExtraFeature::new(
                        ExtraFeatureKind::ClkRebuf(Dir::S, clk_bali_rebuf),
                        "CLK_BALI_REBUF",
                        "CLK_REBUF",
                        format!("ENABLE.GCLK{i}_U"),
                        "1",
                    )];
                    fuzz_one_extras!(ctx, format!("ENABLE.GCLK{i}_D"), "1", [
                        (global_mutex "GCLK", "REBUF_BALI"),
                        (special TileKV::HclkSide(Dir::S))
                    ], [
                        (pip (bel_pin bel_d, "CLKOUT"), (pin format!("GCLK{i}_D")))
                    ], extras);
                }
            }
        }
        for i in 0..16 {
            for ud in ['D', 'U'] {
                let ctx = FuzzCtx::new(
                    session,
                    backend,
                    tile,
                    format!("GCLK_TEST_BUF.REBUF_{ud}{i}"),
                    TileBits::Main(0, bts),
                );
                fuzz_one!(ctx, "ENABLE", "1", [], [(mode "GCLK_TEST_BUF")]);
                fuzz_enum!(ctx, "GCLK_TEST_ENABLE", ["FALSE", "TRUE"], [(mode "GCLK_TEST_BUF")]);
                fuzz_enum!(ctx, "INVERT_INPUT", ["FALSE", "TRUE"], [(mode "GCLK_TEST_BUF")]);
            }
        }
    }
    if !bali_only {
        let node_cmt = backend.egrid.db.get_node("CMT");
        let bel_cmt = BelId::from_idx(18);
        for (tile, bel_hclk_ioi) in [
            ("HCLK_IOI_HR", BelId::from_idx(9)),
            ("HCLK_IOI_HP", BelId::from_idx(10)),
        ] {
            for i in 0..4 {
                let ctx = FuzzCtx::new(session, backend, tile, format!("BUFIO{i}"), TileBits::Hclk);
                fuzz_one!(ctx, "ENABLE", "1", [], [(mode "BUFIO")]);
                fuzz_enum!(ctx, "DELAY_BYPASS", ["FALSE", "TRUE"], [(mode "BUFIO")]);
                fuzz_one!(ctx, "MUX.I", "CCIO", [
                    (mutex "MUX.I", "CCIO"),
                    (related TileRelation::ColPair(0, node_cmt),
                        (tile_mutex "CCIO", "USE_IO")),
                    (related TileRelation::ColPair(0, node_cmt),
                        (bel_mutex bel_cmt, format!("MUX.FREQ_BB{i}"), format!("CCIO{i}"))),
                    (related TileRelation::ColPair(0, node_cmt),
                        (pip (bel_pin bel_cmt, format!("CCIO{i}")), (bel_pin bel_cmt, format!("FREQ_BB{i}_MUX"))))
                ], [
                    (pip
                        (bel_pin bel_hclk_ioi, format!("IOCLK_IN{i}_PAD")),
                        (bel_pin bel_hclk_ioi, format!("IOCLK_IN{i}")))
                ]);
                fuzz_one!(ctx, "MUX.I", "PERF", [
                    (mutex "MUX.I", "PERF"),
                    (related TileRelation::ColPair(0, node_cmt),
                        (tile_mutex "PERF", "USE_IO")),
                    (related TileRelation::ColPair(0, node_cmt),
                        (pip (bel_pin bel_cmt, "PHASER_IN_RCLK0"), (bel_pin bel_cmt, format!("PERF{i}"))))
                ], [
                    (pip
                        (bel_pin bel_hclk_ioi, format!("IOCLK_IN{i}_PERF")),
                        (bel_pin bel_hclk_ioi, format!("IOCLK_IN{i}")))
                ]);
            }
            for i in 0..4 {
                let ctx = FuzzCtx::new(session, backend, tile, format!("BUFR{i}"), TileBits::Hclk);
                fuzz_one!(ctx, "ENABLE", "1", [], [(mode "BUFR"), (attr "BUFR_DIVIDE", "BYPASS")]);
                fuzz_enum!(ctx, "BUFR_DIVIDE", [
                "BYPASS", "1", "2", "3", "4", "5", "6", "7", "8",
            ], [(mode "BUFR")]);
                for j in 0..4 {
                    fuzz_one!(ctx, "MUX.I", format!("CKINT{j}"), [
                        (mutex "MUX.I", format!("CKINT{j}"))
                    ], [
                        (pip (bel_pin bel_hclk_ioi, format!("BUFR_CKINT{j}")), (pin "I"))
                    ]);
                    fuzz_one!(ctx, "MUX.I", format!("BUFIO{j}_I"), [
                        (mutex "MUX.I", format!("BUFIO{j}_I"))
                    ], [
                        (pip (bel_pin bel_hclk_ioi, format!("IOCLK_IN{j}_BUFR")), (pin "I"))
                    ]);
                }
            }
            {
                let ctx = FuzzCtx::new(session, backend, tile, "IDELAYCTRL", TileBits::Hclk);
                for i in 0..6 {
                    for ud in ['U', 'D'] {
                        fuzz_one!(ctx, "MUX.REFCLK", format!("HCLK_IO_{ud}{i}"), [
                            (mutex "MUX.REFCLK", format!("HCLK_IO_{ud}{i}"))
                        ], [
                            (pip (bel_pin bel_hclk_ioi, format!("HCLK_IO_{ud}{i}")), (pin "REFCLK"))
                        ]);
                    }
                }
                fuzz_one!(ctx, "PRESENT", "1", [], [(mode "IDELAYCTRL")]);
                fuzz_enum!(ctx, "HIGH_PERFORMANCE_MODE", ["FALSE", "TRUE"], [(mode "IDELAYCTRL")]);
                fuzz_one!(ctx, "MODE", "DEFAULT", [
                    (tile_mutex "IDELAYCTRL", "TEST"),
                    (mode "IDELAYCTRL")
                ], [
                    (attr "IDELAYCTRL_EN", "DEFAULT"),
                    (attr "BIAS_MODE", "2")
                ]);
                fuzz_one!(ctx, "MODE", "FULL_0", [
                    (tile_mutex "IDELAYCTRL", "TEST"),
                    (mode "IDELAYCTRL")
                ], [
                    (attr "IDELAYCTRL_EN", "ENABLE"),
                    (attr "BIAS_MODE", "0")
                ]);
                fuzz_one!(ctx, "MODE", "FULL_1", [
                    (tile_mutex "IDELAYCTRL", "TEST"),
                    (mode "IDELAYCTRL")
                ], [
                    (attr "IDELAYCTRL_EN", "ENABLE"),
                    (attr "BIAS_MODE", "1")
                ]);
            }
            {
                let ctx = FuzzCtx::new(session, backend, tile, "HCLK_IOI", TileBits::Hclk);
                for i in 0..6 {
                    for ud in ['U', 'D'] {
                        for j in 0..12 {
                            fuzz_one!(ctx, format!("MUX.HCLK_IO_{ud}{i}"), format!("HCLK{j}"), [
                                (mutex format!("MUX.HCLK_IO_{ud}{i}"), format!("HCLK{j}")),
                                (mutex format!("HCLK{j}"), format!("MUX.HCLK_IO_{ud}{i}"))
                            ], [
                                (pip (pin format!("HCLK{j}_BUF")), (pin format!("HCLK_IO_{ud}{i}")))
                            ]);
                        }
                    }
                }
                for i in 0..4 {
                    let li = i % 2;
                    let ud = if i < 2 { 'U' } else { 'D' };
                    fuzz_one!(ctx, format!("BUF.RCLK{i}"), "1", [
                        (global_mutex "RCLK", "USE"),
                        (related TileRelation::ColPair(0, node_cmt),
                            (bel_mutex bel_cmt, format!("MUX.LCLK{li}_{ud}"), format!("RCLK{i}"))),
                        (related TileRelation::ColPair(0, node_cmt),
                            (pip
                                (bel_pin bel_cmt, format!("RCLK{i}")),
                                (bel_pin bel_cmt, format!("LCLK{li}_CMT_{ud}"))))
                    ], [
                        (pip (pin format!("RCLK{i}")), (pin format!("RCLK{i}_IO")))
                    ]);
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, bali_only: bool) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };

    if !bali_only {
        let tile = "HCLK";
        let bel = "HCLK";
        for i in 0..12 {
            let (_, _, diff) = Diff::split(
                ctx.state
                    .peek_diff(tile, "HCLK_R", "MUX.LCLK0_D", format!("HCLK{i}"))
                    .clone(),
                ctx.state
                    .peek_diff(tile, "HCLK_R", "MUX.LCLK0_U", format!("HCLK{i}"))
                    .clone(),
            );
            ctx.tiledb
                .insert(tile, bel, format!("ENABLE.HCLK{i}"), xlat_bit(diff));
        }
        for i in 0..4 {
            let (_, _, diff) = Diff::split(
                ctx.state
                    .peek_diff(tile, "HCLK_R", "MUX.LCLK0_D", format!("RCLK{i}"))
                    .clone(),
                ctx.state
                    .peek_diff(tile, "HCLK_R", "MUX.LCLK0_U", format!("RCLK{i}"))
                    .clone(),
            );
            ctx.tiledb
                .insert(tile, bel, format!("ENABLE.RCLK{i}"), xlat_bit(diff));
        }
        for i in 0..12 {
            let sbel = if i < 6 { "HCLK_R" } else { "HCLK_L" };
            for ud in ['U', 'D'] {
                let mux = &format!("MUX.LCLK{i}_{ud}");
                let mut diffs = vec![("NONE".to_string(), Diff::default())];
                for i in 0..12 {
                    let val = format!("HCLK{i}");
                    let mut diff = ctx.state.get_diff(tile, sbel, mux, &val);
                    diff.apply_bit_diff(
                        ctx.tiledb.item(tile, bel, &format!("ENABLE.HCLK{i}")),
                        true,
                        false,
                    );
                    diffs.push((val, diff));
                }
                for i in 0..4 {
                    let val = format!("RCLK{i}");
                    let mut diff = ctx.state.get_diff(tile, sbel, mux, &val);
                    diff.apply_bit_diff(
                        ctx.tiledb.item(tile, bel, &format!("ENABLE.RCLK{i}")),
                        true,
                        false,
                    );
                    diffs.push((val, diff));
                }
                ctx.tiledb
                    .insert(tile, bel, mux, xlat_enum_ocd(diffs, OcdMode::Mux));
            }
        }
    }
    if !bali_only {
        let tile = "CLK_BUFG";
        for i in 0..16 {
            let bel = &format!("BUFGCTRL{}", i);
            for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
                ctx.collect_inv(tile, bel, pin);
            }
            for attr in ["PRESELECT_I0", "PRESELECT_I1", "CREATE_EDGE"] {
                ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
            }
            ctx.collect_enum_bool(tile, bel, "INIT_OUT", "0", "1");

            for attr in ["MUX.I0", "MUX.I1"] {
                ctx.collect_enum_ocd(
                    tile,
                    bel,
                    attr,
                    &["CASCI", "CKINT0", "CKINT1", "FB_PREV", "FB_NEXT"],
                    OcdMode::Mux,
                );
            }
            ctx.collect_bit(tile, bel, "ENABLE.FB", "1");
            ctx.collect_bit(tile, bel, "ENABLE", "1");
            ctx.collect_bit(tile, bel, "ENABLE.GCLK", "1");
            ctx.collect_bit(tile, bel, "TEST_I0", "1");
            ctx.collect_bit(tile, bel, "TEST_I1", "1");
        }
    }
    if !bali_only {
        let tile = "CLK_HROW";
        let bel = "CLK_HROW";
        for i in 0..32 {
            let (_, _, diff) = Diff::split(
                ctx.state
                    .peek_diff(tile, "BUFHCE_L0", "MUX.I", format!("GCLK{i}"))
                    .clone(),
                ctx.state
                    .peek_diff(tile, "BUFHCE_R0", "MUX.I", format!("GCLK{i}"))
                    .clone(),
            );
            ctx.tiledb
                .insert(tile, bel, format!("ENABLE.GCLK{i}"), xlat_bit(diff));
        }
        for lr in ['L', 'R'] {
            for i in 0..14 {
                let (_, _, diff) = Diff::split(
                    ctx.state
                        .peek_diff(tile, "BUFHCE_L0", "MUX.I", format!("HIN{i}_{lr}"))
                        .clone(),
                    ctx.state
                        .peek_diff(tile, "BUFHCE_R0", "MUX.I", format!("HIN{i}_{lr}"))
                        .clone(),
                );
                ctx.tiledb
                    .insert(tile, bel, format!("ENABLE.HIN{i}_{lr}"), xlat_bit(diff));
            }
        }
        for (pin, sbel_a, sbel_b) in [
            ("CKINT0", "BUFHCE_R0", "BUFHCE_R1"),
            ("CKINT1", "BUFHCE_R0", "BUFHCE_R1"),
            ("CKINT2", "BUFHCE_L0", "BUFHCE_L1"),
            ("CKINT3", "BUFHCE_L0", "BUFHCE_L1"),
        ] {
            let (_, _, diff) = Diff::split(
                ctx.state.peek_diff(tile, sbel_a, "MUX.I", pin).clone(),
                ctx.state.peek_diff(tile, sbel_b, "MUX.I", pin).clone(),
            );
            ctx.tiledb
                .insert(tile, bel, format!("ENABLE.{pin}"), xlat_bit(diff));
        }
        for lr in ['L', 'R'] {
            for i in 0..12 {
                let bel = &format!("BUFHCE_{lr}{i}");
                ctx.collect_bit(tile, bel, "ENABLE", "1");
                ctx.collect_inv(tile, bel, "CE");
                ctx.collect_enum_bool(tile, bel, "INIT_OUT", "0", "1");
                ctx.collect_enum(tile, bel, "CE_TYPE", &["SYNC", "ASYNC"]);
                let mut diffs = vec![];
                for j in 0..32 {
                    let mut diff = ctx.state.get_diff(tile, bel, "MUX.I", format!("GCLK{j}"));
                    diff.apply_bit_diff(
                        ctx.tiledb
                            .item(tile, "CLK_HROW", &format!("ENABLE.GCLK{j}")),
                        true,
                        false,
                    );
                    diffs.push((format!("GCLK{j}"), diff));
                }
                for lr in ['L', 'R'] {
                    for j in 0..14 {
                        let mut diff =
                            ctx.state
                                .get_diff(tile, bel, "MUX.I", format!("HIN{j}_{lr}"));
                        diff.apply_bit_diff(
                            ctx.tiledb
                                .item(tile, "CLK_HROW", &format!("ENABLE.HIN{j}_{lr}")),
                            true,
                            false,
                        );
                        diffs.push((format!("HIN{j}_{lr}"), diff));
                    }
                    let diff = ctx
                        .state
                        .get_diff(tile, bel, "MUX.I", format!("HCLK_TEST_{lr}"));
                    diffs.push((format!("HCLK_TEST_{lr}"), diff));
                }
                let ckints = if (lr == 'R' && i < 6) || (lr == 'L' && i >= 6) {
                    0..2
                } else {
                    2..4
                };
                for j in ckints {
                    let mut diff = ctx.state.get_diff(tile, bel, "MUX.I", format!("CKINT{j}"));
                    diff.apply_bit_diff(
                        ctx.tiledb
                            .item(tile, "CLK_HROW", &format!("ENABLE.CKINT{j}")),
                        true,
                        false,
                    );
                    diffs.push((format!("CKINT{j}"), diff));
                }
                diffs.push(("NONE".to_string(), Diff::default()));
                ctx.tiledb
                    .insert(tile, bel, "MUX.I", xlat_enum_ocd(diffs, OcdMode::Mux));
            }
        }
        for lr in ['L', 'R'] {
            let sbel = &format!("GCLK_TEST_BUF.HROW_BUFH_{lr}");
            ctx.state.get_diff(tile, sbel, "ENABLE", "1").assert_empty();
            ctx.state
                .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "FALSE")
                .assert_empty();
            ctx.state
                .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "TRUE")
                .assert_empty();
            let item = ctx.extract_enum_bool(tile, sbel, "INVERT_INPUT", "FALSE", "TRUE");
            ctx.tiledb
                .insert(tile, bel, format!("INV.HCLK_TEST_{lr}"), item);
            let mut diffs = vec![("NONE".to_string(), Diff::default())];
            for j in 0..14 {
                let mut diff = ctx.state.get_diff(tile, sbel, "MUX.I", format!("HIN{j}"));
                diff.apply_bit_diff(
                    ctx.tiledb
                        .item(tile, "CLK_HROW", &format!("ENABLE.HIN{j}_{lr}")),
                    true,
                    false,
                );
                diffs.push((format!("HIN{j}"), diff));
            }
            ctx.tiledb.insert(
                tile,
                bel,
                format!("MUX.HCLK_TEST_{lr}"),
                xlat_enum_ocd(diffs, OcdMode::Mux),
            );
        }
        for i in 0..32 {
            let sbel = &format!("GCLK_TEST_BUF.HROW_GCLK{i}");
            let item = ctx.extract_bit(tile, sbel, "ENABLE", "1");
            ctx.tiledb
                .insert(tile, bel, format!("ENABLE.GCLK_TEST{i}"), item);
            ctx.state
                .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "FALSE")
                .assert_empty();
            ctx.state
                .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "TRUE")
                .assert_empty();
            let item = ctx.extract_enum_bool(tile, sbel, "INVERT_INPUT", "FALSE", "TRUE");
            ctx.tiledb
                .insert(tile, bel, format!("INV.GCLK_TEST{i}"), item);

            let mut diffs = vec![("NONE".to_string(), Diff::default())];
            for val in ["CASCI", "HCLK_TEST_L", "HCLK_TEST_R"] {
                diffs.push((
                    val.to_string(),
                    ctx.state.get_diff(tile, bel, format!("MUX.CASCO{i}"), val),
                ));
            }
            for lr in ['L', 'R'] {
                for j in 0..14 {
                    let mut diff = ctx.state.get_diff(
                        tile,
                        bel,
                        format!("MUX.CASCO{i}"),
                        format!("HIN{j}_{lr}"),
                    );
                    diff.apply_bit_diff(
                        ctx.tiledb.item(tile, bel, &format!("ENABLE.HIN{j}_{lr}")),
                        true,
                        false,
                    );
                    diffs.push((format!("HIN{j}_{lr}"), diff));
                }
                for j in 0..4 {
                    let diff = ctx.state.get_diff(
                        tile,
                        bel,
                        format!("MUX.CASCO{i}"),
                        format!("RCLK{j}_{lr}"),
                    );
                    if i == 0 {
                        let xdiff = ctx
                            .state
                            .get_diff(
                                tile,
                                bel,
                                format!("MUX.CASCO{i}"),
                                format!("RCLK{j}_{lr}.EXCL"),
                            )
                            .combine(&!&diff);
                        ctx.tiledb.insert(
                            tile,
                            bel,
                            format!("ENABLE.RCLK{j}_{lr}"),
                            xlat_bit(xdiff),
                        );
                    }
                    diffs.push((format!("RCLK{j}_{lr}"), diff));
                }
            }
            for j in [i, i ^ 1] {
                let mut diff = ctx
                    .state
                    .peek_diff(tile, bel, format!("GCLK{j}_TEST_IN"), "1")
                    .clone();
                diff.bits
                    .retain(|&bit, _| diffs.iter().any(|(_, odiff)| odiff.bits.contains_key(&bit)));
                diff = diff.combine(&ctx.state.get_diff(
                    tile,
                    bel,
                    format!("MUX.CASCO{i}"),
                    format!("GCLK_TEST{j}"),
                ));
                diffs.push((format!("GCLK_TEST{j}"), diff));
            }
            ctx.tiledb.insert(
                tile,
                bel,
                format!("MUX.CASCO{i}"),
                xlat_enum_ocd(diffs, OcdMode::Mux),
            );
        }
        for i in 0..32 {
            // slurped above bit by bit
            ctx.state
                .get_diff(tile, bel, format!("GCLK{i}_TEST_IN"), "1");
        }
    }
    for tile in ["CLK_BUFG_REBUF", "CLK_BALI_REBUF"] {
        if bali_only && tile == "CLK_BUFG_REBUF" {
            continue;
        }
        if !ctx.has_tile(tile) {
            continue;
        }
        let bel = "CLK_REBUF";
        for i in 0..16 {
            let sbel = &format!("GCLK_TEST_BUF.REBUF_D{i}");
            let item = if tile == "CLK_BUFG_REBUF" {
                ctx.state
                    .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "FALSE")
                    .assert_empty();
                ctx.state
                    .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "TRUE")
                    .assert_empty();
                ctx.extract_bit(tile, sbel, "ENABLE", "1")
            } else {
                ctx.state
                    .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "FALSE")
                    .assert_empty();
                ctx.state.get_diff(tile, sbel, "ENABLE", "1").assert_empty();
                ctx.extract_bit(tile, sbel, "GCLK_TEST_ENABLE", "TRUE")
            };
            let s = i * 2;
            let d = i * 2 + 1;
            ctx.tiledb
                .insert(tile, bel, format!("BUF.GCLK{d}_D.GCLK{s}_D"), item);
            let item = ctx.extract_enum_bool(tile, sbel, "INVERT_INPUT", "FALSE", "TRUE");
            ctx.tiledb
                .insert(tile, bel, format!("INV.GCLK{d}_D.GCLK{s}_D"), item);

            let sbel = &format!("GCLK_TEST_BUF.REBUF_U{i}");
            let item = if tile == "CLK_BUFG_REBUF" {
                ctx.state
                    .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "FALSE")
                    .assert_empty();
                ctx.state
                    .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "TRUE")
                    .assert_empty();
                ctx.extract_bit(tile, sbel, "ENABLE", "1")
            } else {
                ctx.state
                    .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "FALSE")
                    .assert_empty();
                ctx.state.get_diff(tile, sbel, "ENABLE", "1").assert_empty();
                ctx.extract_bit(tile, sbel, "GCLK_TEST_ENABLE", "TRUE")
            };
            let s = i * 2 + 1;
            let d = i * 2;
            ctx.tiledb
                .insert(tile, bel, format!("BUF.GCLK{d}_U.GCLK{s}_U"), item);
            let item = ctx.extract_enum_bool(tile, sbel, "INVERT_INPUT", "FALSE", "TRUE");
            ctx.tiledb
                .insert(tile, bel, format!("INV.GCLK{d}_U.GCLK{s}_U"), item);
        }
        for i in 0..32 {
            ctx.collect_bit(tile, bel, &format!("ENABLE.GCLK{i}_D"), "1");
            ctx.collect_bit(tile, bel, &format!("ENABLE.GCLK{i}_U"), "1");
            if edev.grids.values().any(|grid| grid.regs > 1) {
                ctx.collect_bit(tile, bel, &format!("BUF.GCLK{i}_D"), "1");
                ctx.collect_bit(tile, bel, &format!("BUF.GCLK{i}_U"), "1");
            }
        }
    }
    if !bali_only {
        let tile = "CMT";
        let bel = "HCLK_CMT";
        for i in 0..4 {
            ctx.collect_bit(tile, bel, &format!("ENABLE.RCLK{i}"), "HROW");
        }
        for tile in ["HCLK_IOI_HP", "HCLK_IOI_HR"] {
            for i in 0..4 {
                let bel = &format!("BUFIO{i}");
                ctx.collect_bit(tile, bel, "ENABLE", "1");
                ctx.collect_enum_bool(tile, bel, "DELAY_BYPASS", "FALSE", "TRUE");
                ctx.collect_enum(tile, bel, "MUX.I", &["CCIO", "PERF"]);
            }
            for i in 0..4 {
                let bel = &format!("BUFR{i}");
                ctx.collect_bit(tile, bel, "ENABLE", "1");
                ctx.collect_enum(
                    tile,
                    bel,
                    "BUFR_DIVIDE",
                    &["BYPASS", "1", "2", "3", "4", "5", "6", "7", "8"],
                );
                ctx.collect_enum_default(
                    tile,
                    bel,
                    "MUX.I",
                    &[
                        "BUFIO0_I", "BUFIO1_I", "BUFIO2_I", "BUFIO3_I", "CKINT0", "CKINT1",
                        "CKINT2", "CKINT3",
                    ],
                    "NONE",
                );
            }
            {
                let bel = "IDELAYCTRL";
                ctx.collect_enum_default(
                    tile,
                    bel,
                    "MUX.REFCLK",
                    &[
                        "HCLK_IO_D0",
                        "HCLK_IO_D1",
                        "HCLK_IO_D2",
                        "HCLK_IO_D3",
                        "HCLK_IO_D4",
                        "HCLK_IO_D5",
                        "HCLK_IO_U0",
                        "HCLK_IO_U1",
                        "HCLK_IO_U2",
                        "HCLK_IO_U3",
                        "HCLK_IO_U4",
                        "HCLK_IO_U5",
                    ],
                    "NONE",
                );
                ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
                ctx.collect_enum_bool(tile, bel, "HIGH_PERFORMANCE_MODE", "FALSE", "TRUE");
                ctx.collect_enum_default(
                    tile,
                    bel,
                    "MODE",
                    &["DEFAULT", "FULL_0", "FULL_1"],
                    "NONE",
                );
            }
            {
                let bel = "HCLK_IOI";
                for i in 0..4 {
                    ctx.collect_bit(tile, bel, &format!("BUF.RCLK{i}"), "1");
                }
                for i in 0..12 {
                    let (_, _, diff) = Diff::split(
                        ctx.state
                            .peek_diff(tile, bel, "MUX.HCLK_IO_D0", format!("HCLK{i}"))
                            .clone(),
                        ctx.state
                            .peek_diff(tile, bel, "MUX.HCLK_IO_U0", format!("HCLK{i}"))
                            .clone(),
                    );
                    ctx.tiledb
                        .insert(tile, bel, format!("ENABLE.HCLK{i}"), xlat_bit(diff));
                }
                for i in 0..6 {
                    for ud in ['U', 'D'] {
                        let mux = &format!("MUX.HCLK_IO_{ud}{i}");
                        let mut diffs = vec![("NONE".to_string(), Diff::default())];
                        for i in 0..12 {
                            let val = format!("HCLK{i}");
                            let mut diff = ctx.state.get_diff(tile, bel, mux, &val);
                            diff.apply_bit_diff(
                                ctx.tiledb.item(tile, bel, &format!("ENABLE.HCLK{i}")),
                                true,
                                false,
                            );
                            diffs.push((val, diff));
                        }
                        ctx.tiledb
                            .insert(tile, bel, mux, xlat_enum_ocd(diffs, OcdMode::Mux));
                    }
                }
            }
        }
    }
}
