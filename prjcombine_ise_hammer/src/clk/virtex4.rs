use prjcombine_collector::{xlat_bit, xlat_bit_wide, xlat_enum_ocd, Diff, OcdMode};
use prjcombine_hammer::Session;
use prjcombine_interconnect::{
    db::{BelId, Dir},
    grid::DieId,
};
use prjcombine_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{BelRelation, ExtraFeature, ExtraFeatureKind, TileBits, TileKV, TileRelation},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    for tile in ["CLK_IOB_B", "CLK_IOB_T"] {
        let ctx = FuzzCtx::new(session, backend, tile, "CLK_IOB", TileBits::ClkIob);
        let giob: Vec<_> = (0..16).map(|i| format!("GIOB{i}")).collect();
        for i in 0..16 {
            fuzz_one!(ctx, format!("BUF.GIOB{i}"), "1", [
                (global_mutex "GIOB", "TEST"),
                (tile_mutex "GIOB_TEST", &giob[i])
            ], [
                (pip (pin format!("PAD_BUF{i}")), (pin &giob[i]))
            ]);
        }
        for i in 0..32 {
            let mux = format!("MUX.MUXBUS{i}");
            let mout = format!("MUXBUS_O{i}");
            let min = format!("MUXBUS_I{i}");
            for j in 0..16 {
                fuzz_one!(ctx, &mux, &giob[j], [
                    (global_mutex "CLK_IOB_MUXBUS", "TEST"),
                    (tile_mutex &mout, &giob[j])
                ], [
                    (pip (pin format!("PAD_BUF{j}")), (pin &mout))
                ]);
            }
            let obel = BelId::from_idx(0);
            fuzz_one!(ctx, &mux, "PASS", [
                (global_mutex "CLK_IOB_MUXBUS", "TEST"),
                (tile_mutex &mout, &min),
                (related TileRelation::ClkDcm,
                    (pip (bel_pin obel, "DCM0"), (bel_pin obel, &mout))),
                (related TileRelation::ClkDcm,
                    (tile_mutex "MUXBUS", "USE"))
            ], [
                (pip (pin &min), (pin &mout))
            ]);
        }
    }
    for tile in ["CLK_DCM_B", "CLK_DCM_T"] {
        let ctx = FuzzCtx::new(session, backend, tile, "CLK_DCM", TileBits::Spine(0, 8));
        let dcm: Vec<_> = (0..24).map(|i| format!("DCM{i}")).collect();
        for i in 0..32 {
            let mux = format!("MUX.MUXBUS{i}");
            let mout = format!("MUXBUS_O{i}");
            let min = format!("MUXBUS_I{i}");
            for j in 0..24 {
                fuzz_one!(ctx, &mux, &dcm[j], [
                    (tile_mutex "MUXBUS", "TEST"),
                    (tile_mutex &mout, &dcm[j])
                ], [
                    (pip (pin &dcm[j]), (pin &mout))
                ]);
            }
            let has_other = if tile == "CLK_DCM_T" {
                edev.grids
                    .values()
                    .any(|grid| grid.regs - grid.reg_clk.to_idx() > 2)
            } else {
                edev.grids.values().any(|grid| grid.reg_clk.to_idx() > 2)
            };
            if has_other {
                let obel = BelId::from_idx(0);
                fuzz_one!(ctx, &mux, "PASS", [
                    (tile_mutex "MUXBUS", "TEST"),
                    (tile_mutex &mout, &min),
                    (related TileRelation::ClkDcm,
                        (pip (bel_pin obel, "DCM0"), (bel_pin obel, &mout))),
                    (related TileRelation::ClkDcm,
                        (tile_mutex "MUXBUS", "USE"))
                ], [
                    (pip (pin &min), (pin &mout))
                ]);
            }
        }
    }
    {
        let ctx = FuzzCtx::new(session, backend, "CLK_HROW", "CLK_HROW", TileBits::ClkHrow);
        let gclk: Vec<_> = (0..32).map(|i| format!("GCLK{i}")).collect();
        for lr in ['L', 'R'] {
            for i in 0..8 {
                let hclk = format!("HCLK_{lr}{i}");
                for j in 0..32 {
                    let bel_bufg = BelId::from_idx(j);
                    fuzz_one!(ctx, format!("MUX.HCLK_{lr}{i}"), &gclk[j], [
                        (global_mutex "BUFGCTRL_OUT", "USE"),
                        (tile_mutex "MODE", "TEST"),
                        (tile_mutex "IN", &gclk[j]),
                        (tile_mutex "OUT", &hclk),
                        (related TileRelation::Cfg,
                            (pip (bel_pin bel_bufg, "O"), (bel_pin bel_bufg, "GCLK")))
                    ], [
                        (pip (pin &gclk[j]), (pin &hclk))
                    ]);
                }
            }
        }
    }
    {
        let ctx = FuzzCtx::new(session, backend, "HCLK", "HCLK", TileBits::Hclk);
        for i in 0..8 {
            let hclk_i = format!("HCLK_I{i}");
            let hclk_o = format!("HCLK_O{i}");
            let hclk_l = format!("HCLK_L{i}");
            let hclk_r = format!("HCLK_R{i}");
            let obel = BelId::from_idx(0);
            fuzz_one!(ctx, format!("BUF.HCLK{i}"), "1", [
                (global_mutex "BUFGCTRL_OUT", "USE"),
                (related TileRelation::ClkHrow(0),
                    (tile_mutex "MODE", "USE")),
                (related TileRelation::ClkHrow(0),
                    (pip (bel_pin obel, "GCLK0"), (bel_pin obel, hclk_l))),
                (related TileRelation::ClkHrow(0),
                    (pip (bel_pin obel, "GCLK0"), (bel_pin obel, hclk_r)))
            ], [
                (pip (pin hclk_i), (pin hclk_o))
            ]);
        }
        for i in 0..2 {
            let rclk_i = format!("RCLK_I{i}");
            let rclk_o = format!("RCLK_O{i}");
            fuzz_one!(ctx, format!("BUF.RCLK{i}"), "1", [
                (related TileRelation::Rclk,
                    (tile_mutex "RCLK_MODE", "USE")),
                (pip
                    (related_pin BelRelation::Rclk, "VRCLK0"),
                    (related_pin BelRelation::Rclk, format!("RCLK{i}")))
            ], [
                (pip (pin rclk_i), (pin rclk_o))
            ]);
        }
    }
    for tile in [
        "HCLK_IOIS_LVDS",
        "HCLK_IOIS_DCI",
        "HCLK_DCMIOB",
        "HCLK_IOBDCM",
        "HCLK_CENTER",
        "HCLK_CENTER_ABOVE_CFG",
    ] {
        let node_kind = backend.egrid.db.get_node(tile);
        let node_data = &backend.egrid.db.nodes[node_kind];
        if let Some((bel, _)) = node_data.bels.get("RCLK") {
            let ctx = FuzzCtx::new(session, backend, tile, "RCLK", TileBits::Hclk);
            for opin in ["RCLK0", "RCLK1"] {
                for ipin in [
                    "VRCLK0", "VRCLK1", "VRCLK_S0", "VRCLK_S1", "VRCLK_N0", "VRCLK_N1",
                ] {
                    fuzz_one!(ctx, format!("MUX.{opin}"), ipin, [
                        (tile_mutex "RCLK_MODE", "TEST"),
                        (tile_mutex opin, ipin)
                    ], [
                        (pip (pin ipin), (pin opin))
                    ]);
                }
            }
            let obel_rclk = bel;
            for bel in ["BUFR0", "BUFR1"] {
                let ctx = FuzzCtx::new(session, backend, tile, bel, TileBits::Hclk);
                fuzz_one!(ctx, "PRESENT", "1", [], [(mode "BUFR")]);
                fuzz_one!(ctx, "ENABLE", "1", [(mode "BUFR")], [(pin "O")]);
                fuzz_enum!(ctx, "BUFR_DIVIDE", ["BYPASS", "1", "2", "3", "4", "5", "6", "7", "8"], [(mode "BUFR")]);
                for ckint in ["CKINT0", "CKINT1"] {
                    fuzz_one!(ctx, "MUX.I", ckint, [
                        (mode "BUFR"),
                        (mutex "MUX.I", ckint)
                    ], [
                        (pip (bel_pin obel_rclk, ckint), (pin "I"))
                    ]);
                }
                for obel_name in ["BUFIO0", "BUFIO1"] {
                    let obel = node_data.bels.get(obel_name).unwrap().0;
                    fuzz_one!(ctx, "MUX.I", obel_name, [
                        (mode "BUFR"),
                        (mutex "MUX.I", obel_name)
                    ], [
                        (pip (bel_pin obel, "O"), (pin "I"))
                    ]);
                }
            }
        }
        {
            let ctx = FuzzCtx::new(session, backend, tile, "IOCLK", TileBits::Hclk);
            for i in 0..8 {
                let hclk_i = format!("HCLK_I{i}");
                let hclk_o = format!("HCLK_O{i}");
                let hclk_l = format!("HCLK_L{i}");
                let hclk_r = format!("HCLK_R{i}");
                let obel = BelId::from_idx(0);
                fuzz_one!(ctx, format!("BUF.HCLK{i}"), "1", [
                    (global_mutex "BUFGCTRL_OUT", "USE"),
                    (related TileRelation::ClkHrow(0),
                        (tile_mutex "MODE", "USE")),
                    (related TileRelation::ClkHrow(0),
                        (pip (bel_pin obel, "GCLK0"), (bel_pin obel, hclk_l))),
                    (related TileRelation::ClkHrow(0),
                        (pip (bel_pin obel, "GCLK0"), (bel_pin obel, hclk_r)))
                ], [
                    (pip (pin hclk_i), (pin hclk_o))
                ]);
            }
            for i in 0..2 {
                let rclk_i = format!("RCLK_I{i}");
                let rclk_o = format!("RCLK_O{i}");
                fuzz_one!(ctx, format!("BUF.RCLK{i}"), "1", [
                    (related TileRelation::Rclk,
                        (tile_mutex "RCLK_MODE", "USE")),
                    (pip
                        (related_pin BelRelation::Rclk, "VRCLK0"),
                        (related_pin BelRelation::Rclk, format!("RCLK{i}")))
                ], [
                    (pip (pin rclk_i), (pin rclk_o))
                ]);
            }

            for (obel_name, vioclk) in [("BUFIO0", "VIOCLK0"), ("BUFIO1", "VIOCLK1")] {
                let obel = node_data.bels.get(obel_name).unwrap().0;
                fuzz_one!(ctx, format!("BUF.{vioclk}"), "1", [
                    (tile_mutex "VIOCLK", vioclk)
                ], [
                    (pip (bel_pin obel, "O"), (pin vioclk))
                ]);
            }
            let (has_s, has_n) = match tile {
                "HCLK_IOIS_DCI" | "HCLK_IOIS_LVDS" => (true, true),
                "HCLK_DCMIOB" | "HCLK_CENTER_ABOVE_CFG" => (false, true),
                "HCLK_IOBDCM" => (true, false),
                "HCLK_CENTER" => (
                    true,
                    edev.grids[DieId::from_idx(0)].row_bufg() - edev.row_dcmiob.unwrap() > 24,
                ),
                _ => unreachable!(),
            };
            if has_s {
                for (o, i, oi, oo) in [
                    ("IOCLK_N0", "VIOCLK_N0", "VIOCLK0", "IOCLK0"),
                    ("IOCLK_N1", "VIOCLK_N1", "VIOCLK1", "IOCLK1"),
                ] {
                    fuzz_one!(ctx, format!("BUF.{o}"), "1", [
                        (related TileRelation::Ioclk(Dir::S), (tile_mutex "VIOCLK", "USE")),
                        (pip
                            (related_pin BelRelation::Ioclk(Dir::S), oi),
                            (related_pin BelRelation::Ioclk(Dir::S), oo))
                    ], [
                        (pip (pin i), (pin o))
                    ]);
                }
            }
            if has_n {
                for (o, i, oi, oo) in [
                    ("IOCLK_S0", "VIOCLK_S0", "VIOCLK0", "IOCLK0"),
                    ("IOCLK_S1", "VIOCLK_S1", "VIOCLK1", "IOCLK1"),
                ] {
                    fuzz_one!(ctx, format!("BUF.{o}"), "1", [
                        (related TileRelation::Ioclk(Dir::N), (tile_mutex "VIOCLK", "USE")),
                        (pip
                            (related_pin BelRelation::Ioclk(Dir::N), oi),
                            (related_pin BelRelation::Ioclk(Dir::N), oo))
                    ], [
                        (pip (pin i), (pin o))
                    ]);
                }
            }
        }
        {
            let ctx = FuzzCtx::new(session, backend, tile, "IDELAYCTRL", TileBits::Hclk);
            fuzz_one!(ctx, "ENABLE", "1", [], [(mode "IDELAYCTRL")]);
            let obel = node_data.bels.get("IOCLK").unwrap().0;
            for i in 0..8 {
                let hclk = format!("HCLK{i}");
                let hclk_o = format!("HCLK_O{i}");
                fuzz_one!(ctx, "MUX.REFCLK", &hclk, [
                    (mutex "REFCLK", &hclk)
                ], [
                    (pip (bel_pin obel, hclk_o), (pin "REFCLK"))
                ]);
            }
        }
    }

    let ccm = backend.egrid.db.get_node("CCM");
    let num_ccms = backend.egrid.node_index[ccm].len();
    let sysmon = backend.egrid.db.get_node("SYSMON");
    let has_hclk_dcm = !backend.egrid.node_index[sysmon].is_empty();
    let has_gt = edev.col_lgt.is_some();
    for (tile, bel) in [
        ("HCLK_DCM", "HCLK_DCM"),
        ("HCLK_DCMIOB", "HCLK_DCM_S"),
        ("HCLK_IOBDCM", "HCLK_DCM_N"),
    ] {
        if tile == "HCLK_DCM" && !has_hclk_dcm {
            continue;
        }
        let ctx = FuzzCtx::new(session, backend, tile, bel, TileBits::Hclk);
        for dir in [Dir::S, Dir::N] {
            if dir == Dir::S && bel == "HCLK_DCM_N" {
                continue;
            }
            if dir == Dir::N && bel == "HCLK_DCM_S" {
                continue;
            }
            let ud = match dir {
                Dir::S => 'D',
                Dir::N => 'U',
                _ => unreachable!(),
            };
            for i in 0..16 {
                let mut extras = vec![];
                if tile == "HCLK_DCM" || num_ccms < 4 {
                    extras.push(ExtraFeature::new(
                        ExtraFeatureKind::HclkDcm(dir),
                        "DCM",
                        "DCM",
                        format!("ENABLE.GIOB{i}"),
                        "1",
                    ));
                }
                if tile != "HCLK_DCM" && num_ccms != 0 {
                    extras.push(ExtraFeature::new(
                        ExtraFeatureKind::HclkCcm(dir),
                        "CCM",
                        "CCM",
                        format!("ENABLE.GIOB{i}"),
                        "1",
                    ))
                }
                fuzz_one_extras!(ctx, format!("BUF.GIOB_{ud}{i}"), "1", [
                    (global_mutex "HCLK_DCM", "TEST"),
                    (tile_mutex "HCLK_DCM", format!("GIOB_O_{ud}{i}"))
                ], [
                    (pip (pin format!("GIOB_I{i}")), (pin format!("GIOB_O_{ud}{i}")))
                ], extras);
            }
            for i in 0..8 {
                let mut extras = vec![];
                if tile == "HCLK_DCM" || num_ccms < 4 {
                    extras.push(ExtraFeature::new(
                        ExtraFeatureKind::HclkDcm(dir),
                        "DCM",
                        "DCM",
                        format!("ENABLE.HCLK{i}"),
                        "1",
                    ));
                }
                if tile != "HCLK_DCM" && num_ccms != 0 {
                    extras.push(ExtraFeature::new(
                        ExtraFeatureKind::HclkCcm(dir),
                        "CCM",
                        "CCM",
                        format!("ENABLE.HCLK{i}"),
                        "1",
                    ))
                }
                let obel = BelId::from_idx(0);
                fuzz_one_extras!(ctx, format!("BUF.HCLK_{ud}{i}"), "1", [
                    (global_mutex "HCLK_DCM", "TEST"),
                    (tile_mutex "HCLK_DCM", format!("HCLK_O_{ud}{i}")),
                    (global_mutex "BUFGCTRL_OUT", "USE"),
                    (related TileRelation::ClkHrow(0),
                        (tile_mutex "MODE", "USE")),
                    (related TileRelation::ClkHrow(0),
                        (pip (bel_pin obel, "GCLK0"), (bel_pin obel, format!("HCLK_L{i}")))),
                    (special TileKV::HclkHasDcm(dir))
                ], [
                    (pip (pin format!("HCLK_I{i}")), (pin format!("HCLK_O_{ud}{i}")))
                ], extras);
            }
            if has_gt || tile == "HCLK_DCM" {
                for i in 0..4 {
                    let mut extras = vec![];
                    if tile == "HCLK_DCM" || num_ccms < 4 {
                        extras.push(ExtraFeature::new(
                            ExtraFeatureKind::HclkDcm(dir),
                            "DCM",
                            "DCM",
                            format!("ENABLE.MGT{i}"),
                            "1",
                        ));
                    }
                    if tile != "HCLK_DCM" && num_ccms != 0 {
                        extras.push(ExtraFeature::new(
                            ExtraFeatureKind::HclkCcm(dir),
                            "CCM",
                            "CCM",
                            format!("ENABLE.MGT{i}"),
                            "1",
                        ))
                    }
                    if tile == "HCLK_DCM" {
                        fuzz_one_extras!(ctx, format!("BUF.MGT_{ud}{i}"), "1", [
                            (global_mutex "HCLK_DCM", "TEST"),
                            (tile_mutex "HCLK_DCM", format!("MGT_O_{ud}{i}")),
                            (special TileKV::HclkHasDcm(Dir::S)),
                            (special TileKV::HclkHasDcm(Dir::N))
                        ], [
                            (pip (pin format!("MGT{i}")), (pin format!("MGT_O_{ud}{i}")))
                        ], extras);
                    } else {
                        if !edev.grids[DieId::from_idx(0)].cols_vbrk.is_empty() {
                            extras.push(ExtraFeature::new(
                                ExtraFeatureKind::MgtRepeater(
                                    if i < 2 { Dir::W } else { Dir::E },
                                    None,
                                ),
                                "HCLK_MGT_REPEATER",
                                "HCLK_MGT_REPEATER",
                                format!("BUF.MGT{idx}.DCM", idx = i % 2),
                                "1",
                            ));
                        }
                        fuzz_one_extras!(ctx, format!("BUF.MGT_{ud}{i}"), "1", [
                            (global_mutex "MGT_OUT", "USE"),
                            (global_mutex "HCLK_DCM", "TEST"),
                            (tile_mutex "HCLK_DCM", format!("MGT_O_{ud}{i}"))
                        ], [
                            (pip (pin format!("MGT_I{i}")), (pin format!("MGT_O_{ud}{i}")))
                        ], extras);
                    }
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };
    for (tile, term) in [("CLK_IOB_B", "CLK_TERM_B"), ("CLK_IOB_T", "CLK_TERM_T")] {
        let bel = "CLK_IOB";
        for i in 0..16 {
            let giob = format!("BUF.GIOB{i}");
            let diff = ctx.state.get_diff(tile, bel, &giob, "1");
            let [diff, diff_term] = &diff.split_tiles(&[
                &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
                &[16],
            ])[..] else {
                unreachable!()
            };
            ctx.tiledb
                .insert(tile, bel, giob, xlat_bit_wide(diff.clone()));
            ctx.tiledb
                .insert(term, "CLK_TERM", "GIOB_ENABLE", xlat_bit(diff_term.clone()));
        }
        for i in 0..32 {
            let mux = format!("MUX.MUXBUS{i}");
            let mut vals = vec![("NONE".to_string(), Diff::default())];
            for j in 0..16 {
                let giob = format!("GIOB{j}");
                vals.push((giob.clone(), ctx.state.get_diff(tile, bel, &mux, &giob)));
            }
            vals.push((
                "PASS".to_string(),
                ctx.state.get_diff(tile, bel, &mux, "PASS"),
            ));
            ctx.tiledb
                .insert(tile, bel, mux, xlat_enum_ocd(vals, OcdMode::Mux));
        }
    }
    for tile in ["CLK_DCM_B", "CLK_DCM_T"] {
        let bel = "CLK_DCM";
        for i in 0..32 {
            let mux = format!("MUX.MUXBUS{i}");
            let mut vals = vec![("NONE".to_string(), Diff::default())];
            for j in 0..24 {
                let dcm = format!("DCM{j}");
                vals.push((dcm.clone(), ctx.state.get_diff(tile, bel, &mux, &dcm)));
            }
            let has_other = if tile == "CLK_DCM_T" {
                edev.grids
                    .values()
                    .any(|grid| grid.regs - grid.reg_clk.to_idx() > 2)
            } else {
                edev.grids.values().any(|grid| grid.reg_clk.to_idx() > 2)
            };
            if has_other {
                vals.push((
                    "PASS".to_string(),
                    ctx.state.get_diff(tile, bel, &mux, "PASS"),
                ));
            }
            ctx.tiledb
                .insert(tile, bel, mux, xlat_enum_ocd(vals, OcdMode::Mux));
        }
    }
    {
        let tile = "CLK_HROW";
        let bel = "CLK_HROW";
        let gclk: Vec<_> = (0..32).map(|i| format!("GCLK{i}")).collect();
        let hclk_l: Vec<_> = (0..8).map(|i| format!("MUX.HCLK_L{i}")).collect();
        let hclk_r: Vec<_> = (0..8).map(|i| format!("MUX.HCLK_R{i}")).collect();
        let mut inp_diffs = vec![];
        for i in 0..32 {
            let diff_l = ctx.state.peek_diff(tile, bel, &hclk_l[0], &gclk[i]).clone();
            let diff_r = ctx.state.peek_diff(tile, bel, &hclk_r[0], &gclk[i]).clone();
            let (_, _, diff) = Diff::split(diff_l, diff_r);
            inp_diffs.push(diff);
        }
        for (hclk, ttile, ttidx) in [(hclk_l, "HCLK_TERM_L", 3), (hclk_r, "HCLK_TERM_R", 4)] {
            for i in 0..8 {
                let mut inps = vec![("NONE", Diff::default())];
                for j in 0..32 {
                    let mut diff = ctx.state.get_diff(tile, bel, &hclk[i], &gclk[j]);
                    diff = diff.combine(&!&inp_diffs[j]);
                    let [diff, diff_term] = &diff.split_tiles(&[&[0, 1, 2], &[ttidx]])[..] else {
                        unreachable!()
                    };
                    inps.push((&gclk[j], diff.clone()));
                    ctx.tiledb.insert(
                        ttile,
                        "HCLK_TERM",
                        "HCLK_ENABLE",
                        xlat_bit_wide(diff_term.clone()),
                    );
                }
                ctx.tiledb
                    .insert(tile, bel, &hclk[i], xlat_enum_ocd(inps, OcdMode::Mux));
            }
        }
        for (i, diff) in inp_diffs.into_iter().enumerate() {
            ctx.tiledb
                .insert(tile, bel, format!("BUF.GCLK{i}"), xlat_bit(diff));
        }
    }
    {
        let tile = "HCLK";
        let bel = "HCLK";
        for i in 0..8 {
            ctx.collect_bit(tile, bel, &format!("BUF.HCLK{i}"), "1");
        }
        for i in 0..2 {
            ctx.collect_bit(tile, bel, &format!("BUF.RCLK{i}"), "1");
        }
    }
    for tile in [
        "HCLK_IOIS_LVDS",
        "HCLK_IOIS_DCI",
        "HCLK_DCMIOB",
        "HCLK_IOBDCM",
        "HCLK_CENTER",
        "HCLK_CENTER_ABOVE_CFG",
    ] {
        let node_kind = ctx.edev.egrid().db.get_node(tile);
        let node_data = &ctx.edev.egrid().db.nodes[node_kind];
        if node_data.bels.contains_key("RCLK") {
            let bel = "RCLK";
            for mux in ["MUX.RCLK0", "MUX.RCLK1"] {
                ctx.collect_enum_default_ocd(
                    tile,
                    bel,
                    mux,
                    &[
                        "VRCLK_N0", "VRCLK0", "VRCLK_S0", "VRCLK_N1", "VRCLK1", "VRCLK_S1",
                    ],
                    "NONE",
                    OcdMode::Mux,
                );
            }
            for bel in ["BUFR0", "BUFR1"] {
                ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
                ctx.collect_bit(tile, bel, "ENABLE", "1");
                ctx.collect_enum(
                    tile,
                    bel,
                    "BUFR_DIVIDE",
                    &["BYPASS", "1", "2", "3", "4", "5", "6", "7", "8"],
                );
                ctx.collect_enum(
                    tile,
                    bel,
                    "MUX.I",
                    &["CKINT0", "CKINT1", "BUFIO0", "BUFIO1"],
                );
            }
        }
        {
            let bel = "IOCLK";
            for i in 0..8 {
                ctx.collect_bit(tile, bel, &format!("BUF.HCLK{i}"), "1");
            }
            for i in 0..2 {
                ctx.collect_bit(tile, bel, &format!("BUF.RCLK{i}"), "1");
            }
            let diff0 = ctx.state.get_diff(tile, bel, "BUF.VIOCLK0", "1");
            let diff1 = ctx.state.get_diff(tile, bel, "BUF.VIOCLK1", "1");
            let (diff0, diff1, diffc) = Diff::split(diff0, diff1);
            ctx.tiledb.insert(tile, bel, "BUF.VIOCLK0", xlat_bit(diff0));
            ctx.tiledb.insert(tile, bel, "BUF.VIOCLK1", xlat_bit(diff1));
            ctx.tiledb
                .insert(tile, bel, "VIOCLK_ENABLE", xlat_bit_wide(diffc));
            let (has_s, has_n) = match tile {
                "HCLK_IOIS_DCI" | "HCLK_IOIS_LVDS" => (true, true),
                "HCLK_DCMIOB" | "HCLK_CENTER_ABOVE_CFG" => (false, true),
                "HCLK_IOBDCM" => (true, false),
                "HCLK_CENTER" => (
                    true,
                    edev.grids[DieId::from_idx(0)].row_bufg() - edev.row_dcmiob.unwrap() > 24,
                ),
                _ => unreachable!(),
            };
            if has_s {
                ctx.collect_bit(tile, bel, "BUF.IOCLK_N0", "1");
                ctx.collect_bit(tile, bel, "BUF.IOCLK_N1", "1");
            }
            if has_n {
                ctx.collect_bit(tile, bel, "BUF.IOCLK_S0", "1");
                ctx.collect_bit(tile, bel, "BUF.IOCLK_S1", "1");
            }
        }
        {
            let bel = "IDELAYCTRL";
            ctx.collect_bit(tile, bel, "ENABLE", "1");
            ctx.collect_enum(
                tile,
                bel,
                "MUX.REFCLK",
                &[
                    "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7",
                ],
            )
        }
    }
    for tile in [
        "HCLK_IOIS_LVDS",
        "HCLK_IOIS_DCI",
        "HCLK_DCMIOB",
        "HCLK_IOBDCM",
        "HCLK_CENTER",
        "HCLK_CENTER_ABOVE_CFG",
    ] {
        for attr in [
            "BUF.IOCLK_N0",
            "BUF.IOCLK_N1",
            "BUF.IOCLK_S0",
            "BUF.IOCLK_S1",
        ] {
            let item = ctx.tiledb.item("HCLK_IOIS_LVDS", "IOCLK", attr).clone();
            ctx.tiledb.insert(tile, "IOCLK", attr, item);
        }
    }
    {
        let tile = "HCLK_DCM";
        let bel = "HCLK_DCM";
        let (_, _, common) = Diff::split(
            ctx.state.peek_diff(tile, bel, "BUF.MGT_D0", "1").clone(),
            ctx.state.peek_diff(tile, bel, "BUF.GIOB_D0", "1").clone(),
        );
        let (_, _, hclk_giob) = Diff::split(
            ctx.state.peek_diff(tile, bel, "BUF.HCLK_D0", "1").clone(),
            ctx.state.peek_diff(tile, bel, "BUF.GIOB_D0", "1").clone(),
        );
        let (_, _, common_mgt) = Diff::split(
            ctx.state.peek_diff(tile, bel, "BUF.MGT_D0", "1").clone(),
            ctx.state.peek_diff(tile, bel, "BUF.MGT_D1", "1").clone(),
        );
        for ud in ['U', 'D'] {
            for i in 0..8 {
                let diff = ctx
                    .state
                    .get_diff(tile, bel, format!("BUF.HCLK_{ud}{i}"), "1");
                let diff = diff.combine(&!&hclk_giob);
                ctx.tiledb
                    .insert(tile, bel, format!("BUF.HCLK_{ud}{i}"), xlat_bit(diff));
            }
            for i in 0..16 {
                let diff = ctx
                    .state
                    .get_diff(tile, bel, format!("BUF.GIOB_{ud}{i}"), "1");
                let diff = diff.combine(&!&hclk_giob);
                ctx.tiledb
                    .insert(tile, bel, format!("BUF.GIOB_{ud}{i}"), xlat_bit(diff));
            }
            for i in 0..4 {
                let diff = ctx
                    .state
                    .get_diff(tile, bel, format!("BUF.MGT_{ud}{i}"), "1");
                let diff = diff.combine(&!&common_mgt);
                ctx.tiledb
                    .insert(tile, bel, format!("BUF.MGT_{ud}{i}"), xlat_bit(diff));
            }
        }
        let hclk_giob = hclk_giob.combine(&!&common);
        let common_mgt = common_mgt.combine(&!&common);
        ctx.tiledb
            .insert(tile, bel, "COMMON", xlat_bit_wide(common));
        ctx.tiledb
            .insert(tile, bel, "COMMON_HCLK_GIOB", xlat_bit_wide(hclk_giob));
        ctx.tiledb
            .insert(tile, bel, "COMMON_MGT", xlat_bit_wide(common_mgt));
    }
    for (tile, bel, ud) in [
        ("HCLK_DCMIOB", "HCLK_DCM_S", 'D'),
        ("HCLK_IOBDCM", "HCLK_DCM_N", 'U'),
    ] {
        let (_, _, common) = Diff::split(
            ctx.state
                .peek_diff(tile, bel, format!("BUF.HCLK_{ud}0"), "1")
                .clone(),
            ctx.state
                .peek_diff(tile, bel, format!("BUF.GIOB_{ud}0"), "1")
                .clone(),
        );
        for i in 0..8 {
            let diff = ctx
                .state
                .get_diff(tile, bel, format!("BUF.HCLK_{ud}{i}"), "1");
            let diff = diff.combine(&!&common);
            ctx.tiledb
                .insert(tile, bel, format!("BUF.HCLK_{ud}{i}"), xlat_bit(diff));
        }
        for i in 0..16 {
            let diff = ctx
                .state
                .get_diff(tile, bel, format!("BUF.GIOB_{ud}{i}"), "1");
            let diff = diff.combine(&!&common);
            ctx.tiledb
                .insert(tile, bel, format!("BUF.GIOB_{ud}{i}"), xlat_bit(diff));
        }
        if edev.col_lgt.is_some() {
            let (_, _, common_mgt) = Diff::split(
                ctx.state
                    .peek_diff(tile, bel, format!("BUF.MGT_{ud}0"), "1")
                    .clone(),
                ctx.state
                    .peek_diff(tile, bel, format!("BUF.MGT_{ud}1"), "1")
                    .clone(),
            );
            for i in 0..4 {
                let diff = ctx
                    .state
                    .get_diff(tile, bel, format!("BUF.MGT_{ud}{i}"), "1");
                let diff = diff.combine(&!&common_mgt);
                ctx.tiledb
                    .insert(tile, bel, format!("BUF.MGT_{ud}{i}"), xlat_bit(diff));
            }
            let common_mgt = common_mgt.combine(&!&common);
            ctx.tiledb
                .insert(tile, bel, "COMMON_MGT", xlat_bit_wide(common_mgt));
        }
        ctx.tiledb
            .insert(tile, bel, "COMMON", xlat_bit_wide(common));
    }
    {
        let tile = "DCM";
        let bel = "DCM";
        for i in 0..16 {
            ctx.collect_bit(tile, bel, &format!("ENABLE.GIOB{i}"), "1");
        }
        for i in 0..8 {
            ctx.collect_bit(tile, bel, &format!("ENABLE.HCLK{i}"), "1");
        }
        for i in 0..4 {
            ctx.collect_bit(tile, bel, &format!("ENABLE.MGT{i}"), "1");
        }
    }
    let ccm = edev.egrid.db.get_node("CCM");
    let num_ccms = edev.egrid.node_index[ccm].len();
    if num_ccms != 0 {
        let tile = "CCM";
        let bel = "CCM";
        for i in 0..16 {
            ctx.collect_bit(tile, bel, &format!("ENABLE.GIOB{i}"), "1");
        }
        for i in 0..8 {
            ctx.collect_bit(tile, bel, &format!("ENABLE.HCLK{i}"), "1");
        }
        if edev.col_lgt.is_some() {
            for i in 0..4 {
                ctx.collect_bit(tile, bel, &format!("ENABLE.MGT{i}"), "1");
            }
        }
    }
    if edev.col_lgt.is_some() && !edev.grids[DieId::from_idx(0)].cols_vbrk.is_empty() {
        let tile = "HCLK_MGT_REPEATER";
        let bel = "HCLK_MGT_REPEATER";
        let item = ctx.extract_bit(tile, bel, "BUF.MGT0.DCM", "1");
        ctx.tiledb.insert(tile, bel, "BUF.MGT0", item);
        let item = ctx.extract_bit(tile, bel, "BUF.MGT1.DCM", "1");
        ctx.tiledb.insert(tile, bel, "BUF.MGT1", item);
    }
}
