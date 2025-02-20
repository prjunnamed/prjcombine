use prjcombine_collector::{xlat_bit, xlat_bit_wide, xlat_enum_ocd, Diff, OcdMode};
use prjcombine_hammer::Session;
use prjcombine_interconnect::{db::BelId, grid::DieId};
use prjcombine_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{BelFuzzKV, BelRelation, ExtraFeature, ExtraFeatureKind, TileBits, TileRelation},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    {
        let ctx = FuzzCtx::new(session, backend, "HCLK", "HCLK", TileBits::Hclk);
        for i in 0..10 {
            fuzz_one!(ctx, format!("BUF.HCLK{i}"), "1", [], [
                (pip (pin format!("HCLK_I{i}")), (pin format!("HCLK_O{i}")))
            ]);
        }
        for i in 0..4 {
            fuzz_one!(ctx, format!("BUF.RCLK{i}"), "1", [
                (related TileRelation::Rclk,
                    (tile_mutex "RCLK_MODE", "USE")),
                (pip
                    (related_pin BelRelation::Rclk, "VRCLK0"),
                    (related_pin BelRelation::Rclk, format!("RCLK_I{i}")))
            ], [
                (pip (pin format!("RCLK_I{i}")), (pin format!("RCLK_O{i}")))
            ]);
        }
    }
    {
        let ctx = FuzzCtx::new(session, backend, "CLK_HROW", "CLK_HROW", TileBits::ClkHrow);
        for lr in ['L', 'R'] {
            for i in 0..10 {
                for j in 0..32 {
                    fuzz_one!(ctx, format!("MUX.HCLK_{lr}{i}"), format!("GCLK{j}"), [
                        (tile_mutex format!("IN_HCLK_{lr}{i}"), format!("GCLK{j}")),
                        (tile_mutex format!("OUT_GCLK{j}"), format!("HCLK_{lr}{i}"))
                    ], [
                        (pip (pin format!("GCLK{j}")), (pin format!("HCLK_{lr}{i}")))
                    ]);
                }
            }
        }
    }
    for (tile, bel) in [
        ("CLK_IOB_B", "CLK_IOB"),
        ("CLK_IOB_T", "CLK_IOB"),
        ("CLK_CMT_B", "CLK_CMT"),
        ("CLK_CMT_T", "CLK_CMT"),
        ("CLK_MGT_B", "CLK_MGT"),
        ("CLK_MGT_T", "CLK_MGT"),
    ] {
        let hclk_cmt = backend.egrid.db.get_node("HCLK_CMT");
        let Some(ctx) = FuzzCtx::try_new(session, backend, tile, bel, TileBits::Spine(0, 10))
        else {
            continue;
        };
        for i in 0..32 {
            let mux = format!("MUX.MUXBUS{i}");
            let mout = format!("MUXBUS_O{i}");
            let min = format!("MUXBUS_I{i}");
            for lr in ['L', 'R'] {
                if lr == 'L' && edev.col_lgt.is_none() && bel != "CLK_IOB" {
                    continue;
                }
                for j in 0..5 {
                    fuzz_one!(ctx, &mux, format!("MGT_{lr}{j}"), [
                        (tile_mutex &mux, format!("MGT_{lr}{j}")),
                        (tile_mutex format!("MGT_{lr}{j}"), &mout)
                    ], [
                        (pip (pin format!("MGT_{lr}{j}")), (pin &mout))
                    ]);
                }
            }
            if bel == "CLK_CMT" {
                for j in 0..28 {
                    fuzz_one!(ctx, &mux, format!("CMT_CLK{j}"), [
                        (related TileRelation::Hclk(hclk_cmt),
                            (tile_mutex "ENABLE", "NOPE")),
                        (tile_mutex &mux, format!("CMT_CLK{j}"))
                    ], [
                        (pip (pin format!("CMT_CLK{j}")), (pin &mout))
                    ]);
                }
            }
            if bel == "CLK_IOB" {
                for j in 0..10 {
                    fuzz_one!(ctx, &mux, format!("GIOB{j}"), [
                        (tile_mutex &mux, format!("GIOB{j}"))
                    ], [
                        (pip (pin format!("PAD_BUF{j}")), (pin &mout))
                    ]);
                }
            }
            fuzz_one!(
                ctx,
                &mux,
                "PASS",
                [(tile_mutex & mux, "PASS")],
                [(pip(pin & min), (pin & mout))]
            );
        }
        if bel == "CLK_IOB" {
            for i in 0..10 {
                fuzz_one!(ctx, format!("BUF.GIOB{i}"), "1", [], [
                    (pip (pin format!("PAD_BUF{i}")), (pin format!("GIOB{i}")))
                ]);
            }
        }
    }
    for tile in [
        "HCLK_IOI",
        "HCLK_IOI_CENTER",
        "HCLK_IOI_TOPCEN",
        "HCLK_IOI_BOTCEN",
        "HCLK_IOI_CMT",
        "HCLK_CMT_IOI",
    ] {
        let node_kind = backend.egrid.db.get_node(tile);
        let node_data = &backend.egrid.db.nodes[node_kind];

        for i in 0..4 {
            if let Some(ctx) =
                FuzzCtx::try_new(session, backend, tile, format!("BUFIO{i}"), TileBits::Hclk)
            {
                fuzz_one!(ctx, "PRESENT", "1", [], [(mode "BUFIO")]);
                fuzz_one!(ctx, "ENABLE", "1", [
                    (mode "BUFIO"),
                    (tile_mutex "BUFIO", format!("TEST_BUFIO{i}"))
                ], [(pin "O")]);
            }
        }
        for i in 0..4 {
            if let Some(ctx) =
                FuzzCtx::try_new(session, backend, tile, format!("BUFR{i}"), TileBits::Hclk)
            {
                fuzz_one!(ctx, "ENABLE", "1", [], [(mode "BUFR")]);
                fuzz_enum!(ctx, "BUFR_DIVIDE", ["BYPASS", "1", "2", "3", "4", "5", "6", "7", "8"], [(mode "BUFR")]);
                let obel_rclk = node_data.bels.get("RCLK").unwrap().0;
                for pin in ["MGT0", "MGT1", "MGT2", "MGT3", "MGT4", "CKINT0", "CKINT1"] {
                    fuzz_one!(ctx, "MUX.I", pin, [
                        (mode "BUFR"),
                        (mutex "MUX.I", pin)
                    ], [
                        (pip (bel_pin obel_rclk, pin), (pin "I"))
                    ]);
                }
                for obel_name in ["BUFIO0", "BUFIO1", "BUFIO2", "BUFIO3"] {
                    let obel = node_data.bels.get(obel_name).unwrap().0;
                    fuzz_one!(ctx, "MUX.I", obel_name, [
                        (mode "BUFR"),
                        (mutex "MUX.I", obel_name)
                    ], [
                        (pip (bel_pin_far obel, "I"), (pin "I"))
                    ]);
                }
            }
        }
        if let Some(ctx) = FuzzCtx::try_new(session, backend, tile, "IOCLK", TileBits::Hclk) {
            for i in 0..10 {
                fuzz_one!(ctx, format!("BUF.HCLK{i}"), "1", [], [
                    (pip (pin format!("HCLK_I{i}")), (pin format!("HCLK_O{i}")))
                ]);
            }
            for i in 0..4 {
                fuzz_one!(ctx, &format!("BUF.RCLK{i}"), "1", [
                    (related TileRelation::Rclk,
                        (tile_mutex "RCLK_MODE", "USE")),
                    (pip
                        (related_pin BelRelation::Rclk, "VRCLK0"),
                        (related_pin BelRelation::Rclk, format!("RCLK_I{i}")))
                ], [
                    (pip (pin format!("RCLK_I{i}")), (pin format!("RCLK_O{i}")))
                ]);
            }
            if tile == "HCLK_IOI" {
                for i in 0..4 {
                    for inp in [
                        "VRCLK0", "VRCLK1", "VRCLK_S0", "VRCLK_S1", "VRCLK_N0", "VRCLK_N1",
                    ] {
                        let mut extras = vec![];
                        for otile in [
                            "HCLK_IOI_CENTER",
                            "HCLK_IOI_TOPCEN",
                            "HCLK_IOI_BOTCEN",
                            "HCLK_IOI_CMT",
                            "HCLK_CMT_IOI",
                        ] {
                            let onode = backend.egrid.db.get_node(otile);
                            if !backend.egrid.node_index[onode].is_empty() {
                                extras.push(ExtraFeature::new(
                                    ExtraFeatureKind::HclkIoiCenter(otile),
                                    otile,
                                    "IOCLK",
                                    format!("ENABLE.RCLK{i}"),
                                    "1",
                                ));
                            }
                        }
                        fuzz_one_extras!(ctx, format!("MUX.RCLK{i}"), inp, [
                            (tile_mutex "RCLK_MODE", "TEST"),
                            (mutex format!("MUX.RCLK{i}"), inp)
                        ], [
                            (pip (pin inp), (pin format!("RCLK_I{i}")))
                        ], extras);
                    }
                }
            }
        }
        if let Some(ctx) = FuzzCtx::try_new(session, backend, tile, "IDELAYCTRL", TileBits::Hclk) {
            let obel = node_data.bels.get("IOCLK").unwrap().0;
            for i in 0..10 {
                fuzz_one!(ctx, "MUX.REFCLK", format!("HCLK{i}"), [
                    (mutex "MUX.REFCLK", format!("HCLK{i}"))
                ], [
                    (pip (bel_pin obel, format!("HCLK_O{i}")), (pin "REFCLK"))
                ]);
            }
            fuzz_one_extras!(ctx, "MODE", "DEFAULT_ONLY", [
                (global_opt "LEGIDELAY", "DISABLE"),
                (mode "")
            ], [
                (bel_special BelFuzzKV::AllIodelay("DEFAULT"))
            ], vec![
                ExtraFeature::new(
                    ExtraFeatureKind::AllBankIoi,
                    "IO",
                    "IODELAY_BOTH",
                    "IDELAYCTRL_MODE",
                    "DEFAULT_ONLY",
                ),
            ]);
            fuzz_one_extras!(ctx, "MODE", "FULL", [
                (global_opt "LEGIDELAY", "DISABLE")
            ], [
                (bel_special BelFuzzKV::AllIodelay("FIXED")),
                (mode "IDELAYCTRL")
            ], vec![
                ExtraFeature::new(
                    ExtraFeatureKind::AllBankIoi,
                    "IO",
                    "IODELAY_BOTH",
                    "IDELAYCTRL_MODE",
                    "FULL",
                ),
            ]);
        }
    }
    {
        let ctx = FuzzCtx::new_force_bel(
            session,
            backend,
            "HCLK_CMT",
            "HCLK_CMT",
            TileBits::Hclk,
            BelId::from_idx(0),
        );
        for i in 0..10 {
            fuzz_one!(ctx, format!("BUF.HCLK{i}"), "1", [
                (global_mutex "HCLK_CMT", "TEST")
            ], [
                (pip (pin format!("HCLK_I{i}")), (pin format!("HCLK_O{i}")))
            ]);
        }
        let ctx = FuzzCtx::new_force_bel(
            session,
            backend,
            "HCLK_CMT",
            "HCLK_CMT",
            TileBits::Hclk,
            BelId::from_idx(1),
        );
        for i in 0..10 {
            fuzz_one!(ctx, format!("BUF.GIOB{i}"), "1", [
                (global_mutex "HCLK_CMT", "TEST")
            ], [
                (pip (pin format!("GIOB_I{i}")), (pin format!("GIOB_O{i}")))
            ]);
        }
    }
    if let Some(ctx) = FuzzCtx::try_new(
        session,
        backend,
        "HCLK_BRAM_MGT",
        "HCLK_BRAM_MGT",
        TileBits::Hclk,
    ) {
        for i in 0..5 {
            let mut extras = vec![];
            let cols_mgt_buf = &edev.grids[DieId::from_idx(0)].cols_mgt_buf;
            let num_l = cols_mgt_buf
                .iter()
                .copied()
                .filter(|&col| col < edev.col_clk)
                .count();
            let num_r = cols_mgt_buf
                .iter()
                .copied()
                .filter(|&col| col > edev.col_clk)
                .count();
            if num_l > 1 || num_r > 1 {
                extras.push(ExtraFeature::new(
                    ExtraFeatureKind::HclkBramMgtPrev,
                    "HCLK_BRAM_MGT",
                    "HCLK_BRAM_MGT",
                    format!("BUF.MGT{i}"),
                    "1",
                ));
            }
            fuzz_one_extras!(ctx, format!("BUF.MGT{i}"), "1", [
                // overzealous, but I don't care
                (global_mutex_site "HCLK_MGT")
            ], [
                (pip (pin format!("MGT_I{i}")), (pin format!("MGT_O{i}")))
            ], extras);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };
    {
        let tile = "HCLK";
        let bel = "HCLK";
        for i in 0..10 {
            ctx.collect_bit(tile, bel, &format!("BUF.HCLK{i}"), "1");
        }
        for i in 0..4 {
            ctx.collect_bit(tile, bel, &format!("BUF.RCLK{i}"), "1");
        }
    }
    {
        let tile = "CLK_HROW";
        let bel = "CLK_HROW";
        let mut inp_diffs = vec![];
        for i in 0..32 {
            let diff_l = ctx
                .state
                .peek_diff(tile, bel, "MUX.HCLK_L0", format!("GCLK{i}"))
                .clone();
            let diff_r = ctx
                .state
                .peek_diff(tile, bel, "MUX.HCLK_R0", format!("GCLK{i}"))
                .clone();
            let (_, _, diff) = Diff::split(diff_l, diff_r);
            inp_diffs.push(diff);
        }
        for i in 0..10 {
            for lr in ['L', 'R'] {
                let mut inps = vec![("NONE".to_string(), Diff::default())];
                for j in 0..32 {
                    let mut diff = ctx.state.get_diff(
                        tile,
                        bel,
                        format!("MUX.HCLK_{lr}{i}"),
                        format!("GCLK{j}"),
                    );
                    diff = diff.combine(&!&inp_diffs[j]);
                    inps.push((format!("GCLK{j}"), diff));
                }
                ctx.tiledb.insert(
                    tile,
                    bel,
                    format!("MUX.HCLK_{lr}{i}"),
                    xlat_enum_ocd(inps, OcdMode::Mux),
                );
            }
        }
        for (i, diff) in inp_diffs.into_iter().enumerate() {
            ctx.tiledb
                .insert(tile, bel, format!("BUF.GCLK{i}"), xlat_bit(diff));
        }
    }
    for (tile, bel) in [
        ("CLK_IOB_B", "CLK_IOB"),
        ("CLK_IOB_T", "CLK_IOB"),
        ("CLK_CMT_B", "CLK_CMT"),
        ("CLK_CMT_T", "CLK_CMT"),
        ("CLK_MGT_B", "CLK_MGT"),
        ("CLK_MGT_T", "CLK_MGT"),
    ] {
        if !ctx.has_tile(tile) {
            continue;
        }
        for i in 0..5 {
            for lr in ['L', 'R'] {
                if lr == 'L' && edev.col_lgt.is_none() && bel != "CLK_IOB" {
                    continue;
                }
                let diff_a = ctx
                    .state
                    .peek_diff(tile, bel, "MUX.MUXBUS0", format!("MGT_{lr}{i}"))
                    .clone();
                let diff_b = ctx
                    .state
                    .peek_diff(tile, bel, "MUX.MUXBUS1", format!("MGT_{lr}{i}"))
                    .clone();
                let (_, _, diff) = Diff::split(diff_a, diff_b);
                ctx.tiledb
                    .insert(tile, bel, format!("BUF.MGT_{lr}{i}"), xlat_bit(diff));
            }
        }
        for i in 0..32 {
            let mut diffs = vec![
                ("NONE".to_string(), Diff::default()),
                (
                    "PASS".to_string(),
                    ctx.state
                        .get_diff(tile, bel, format!("MUX.MUXBUS{i}"), "PASS"),
                ),
            ];
            for j in 0..5 {
                for lr in ['L', 'R'] {
                    if lr == 'L' && edev.col_lgt.is_none() && bel != "CLK_IOB" {
                        continue;
                    }
                    let mut diff = ctx.state.get_diff(
                        tile,
                        bel,
                        format!("MUX.MUXBUS{i}"),
                        format!("MGT_{lr}{j}"),
                    );
                    diff.apply_bit_diff(
                        ctx.tiledb.item(tile, bel, &format!("BUF.MGT_{lr}{j}")),
                        true,
                        false,
                    );
                    diffs.push((format!("MGT_{lr}{j}"), diff));
                }
            }
            if bel == "CLK_CMT" {
                for j in 0..28 {
                    diffs.push((
                        format!("CMT_CLK{j}"),
                        ctx.state.get_diff(
                            tile,
                            bel,
                            format!("MUX.MUXBUS{i}"),
                            format!("CMT_CLK{j}"),
                        ),
                    ));
                }
            }
            if bel == "CLK_IOB" {
                for j in 0..10 {
                    diffs.push((
                        format!("GIOB{j}"),
                        ctx.state
                            .get_diff(tile, bel, format!("MUX.MUXBUS{i}"), format!("GIOB{j}")),
                    ));
                }
            }
            ctx.tiledb.insert(
                tile,
                bel,
                format!("MUX.MUXBUS{i}"),
                xlat_enum_ocd(diffs, OcdMode::Mux),
            );
        }
        if bel == "CLK_IOB" {
            for i in 0..10 {
                ctx.collect_bit_wide(tile, bel, &format!("BUF.GIOB{i}"), "1");
            }
        }
    }
    for tile in [
        "HCLK_IOI",
        "HCLK_IOI_CENTER",
        "HCLK_IOI_TOPCEN",
        "HCLK_IOI_BOTCEN",
        "HCLK_IOI_CMT",
        "HCLK_CMT_IOI",
    ] {
        let node_kind = edev.egrid.db.get_node(tile);
        let node_data = &edev.egrid.db.nodes[node_kind];

        if !ctx.has_tile(tile) {
            continue;
        }
        let mut diffs = vec![];
        for i in 0..4 {
            let bel = format!("BUFIO{i}");
            if !node_data.bels.contains_key(&bel) {
                continue;
            }
            ctx.state
                .get_diff(tile, &bel, "PRESENT", "1")
                .assert_empty();
            let diff = ctx.state.get_diff(tile, &bel, "ENABLE", "1");
            diffs.push((bel, diff));
        }
        let (_, _, enable) = Diff::split(diffs[0].1.clone(), diffs[1].1.clone());
        for (bel, mut diff) in diffs {
            diff = diff.combine(&!&enable);
            ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(diff));
        }
        ctx.tiledb
            .insert(tile, "IOCLK", "IOCLK_ENABLE", xlat_bit_wide(enable));

        if tile == "HCLK_IOI" {
            for i in 0..2 {
                let bel = format!("BUFR{i}");
                let bel = &bel;
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
                        "CKINT0", "CKINT1", "BUFIO0", "BUFIO1", "BUFIO2", "BUFIO3", "MGT0", "MGT1",
                        "MGT2", "MGT3", "MGT4",
                    ],
                    "NONE",
                );
            }
            for i in 0..4 {
                let item = ctx.extract_enum_default_ocd(
                    tile,
                    "IOCLK",
                    &format!("MUX.RCLK{i}"),
                    &[
                        "VRCLK0", "VRCLK1", "VRCLK_S0", "VRCLK_S1", "VRCLK_N0", "VRCLK_N1",
                    ],
                    "NONE",
                    OcdMode::Mux,
                );
                ctx.tiledb
                    .insert(tile, "RCLK", format!("MUX.RCLK{i}"), item);
            }
        } else {
            for i in 0..4 {
                ctx.collect_bit(tile, "IOCLK", &format!("ENABLE.RCLK{i}"), "1");
            }
        }
        {
            let bel = "IOCLK";
            for i in 0..10 {
                ctx.collect_bit(tile, bel, &format!("BUF.HCLK{i}"), "1");
            }
            for i in 0..4 {
                ctx.collect_bit(tile, bel, &format!("BUF.RCLK{i}"), "1");
            }
        }
        {
            let bel = "IDELAYCTRL";
            ctx.collect_enum_default(
                tile,
                bel,
                "MUX.REFCLK",
                &[
                    "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7",
                    "HCLK8", "HCLK9",
                ],
                "NONE",
            );
            ctx.collect_enum_default(tile, bel, "MODE", &["FULL", "DEFAULT_ONLY"], "NONE");
        }
    }
    {
        let tile = "IO";
        let bel = "IODELAY_BOTH";
        // don't worry about it kitten
        ctx.state
            .get_diff(tile, bel, "IDELAYCTRL_MODE", "DEFAULT_ONLY");
        ctx.state.get_diff(tile, bel, "IDELAYCTRL_MODE", "FULL");
    }
    {
        let tile = "HCLK_CMT";
        let bel = "HCLK_CMT";
        for i in 0..10 {
            ctx.collect_bit(tile, bel, &format!("BUF.HCLK{i}"), "1");
            ctx.collect_bit(tile, bel, &format!("BUF.GIOB{i}"), "1");
        }
    }
    if ctx.has_tile("HCLK_BRAM_MGT") {
        let tile = "HCLK_BRAM_MGT";
        let bel = "HCLK_BRAM_MGT";
        for i in 0..5 {
            ctx.collect_bit(tile, bel, &format!("BUF.MGT{i}"), "1");
        }
    }
}
