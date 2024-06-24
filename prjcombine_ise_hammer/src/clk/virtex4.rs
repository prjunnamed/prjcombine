use prjcombine_hammer::Session;
use prjcombine_int::db::{BelId, Dir};
use prjcombine_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::{xlat_bit_wide, xlat_bitvec, xlat_enum_ocd, CollectorCtx, Diff, OcdMode},
    fgen::{BelRelation, TileBits, TileRelation},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    for tile in ["CLK_IOB_B", "CLK_IOB_T"] {
        let node_kind = backend.egrid.db.get_node(tile);
        let ctx = FuzzCtx {
            session,
            node_kind,
            bits: TileBits::ClkIob,
            tile_name: tile,
            bel: BelId::from_idx(0),
            bel_name: "CLK_IOB",
        };
        let pad_buf: Vec<_> = (0..16).map(|i| &*format!("PAD_BUF{i}").leak()).collect();
        let giob: Vec<_> = (0..16).map(|i| &*format!("GIOB{i}").leak()).collect();
        for i in 0..16 {
            fuzz_one!(ctx, giob[i], "1", [
                (global_mutex "GIOB", "TEST"),
                (tile_mutex "GIOB_TEST", giob[i])
            ], [
                (pip (pin pad_buf[i]), (pin giob[i]))
            ]);
        }
        for i in 0..32 {
            let mux = &*format!("MUXBUS{i}").leak();
            let mout = &*format!("MUXBUS_O{i}").leak();
            let min = &*format!("MUXBUS_I{i}").leak();
            for j in 0..16 {
                fuzz_one!(ctx, mux, giob[j], [
                    (global_mutex "CLK_IOB_MUXBUS", "TEST"),
                    (tile_mutex mout, giob[j])
                ], [
                    (pip (pin pad_buf[j]), (pin mout))
                ]);
            }
            let obel = BelId::from_idx(0);
            fuzz_one!(ctx, mux, "PASS", [
                (global_mutex "CLK_IOB_MUXBUS", "TEST"),
                (tile_mutex mout, min),
                (related TileRelation::ClkDcm,
                    (pip (bel_pin obel, "DCM0"), (bel_pin obel, mout))),
                (related TileRelation::ClkDcm,
                    (tile_mutex "MUXBUS", "USE"))
            ], [
                (pip (pin min), (pin mout))
            ]);
        }
    }
    for tile in ["CLK_DCM_B", "CLK_DCM_T"] {
        let node_kind = backend.egrid.db.get_node(tile);
        let ctx = FuzzCtx {
            session,
            node_kind,
            bits: TileBits::Spine(8),
            tile_name: tile,
            bel: BelId::from_idx(0),
            bel_name: "CLK_DCM",
        };
        let dcm: Vec<_> = (0..24).map(|i| &*format!("DCM{i}").leak()).collect();
        for i in 0..32 {
            let mux = &*format!("MUXBUS{i}").leak();
            let mout = &*format!("MUXBUS_O{i}").leak();
            let min = &*format!("MUXBUS_I{i}").leak();
            for j in 0..24 {
                fuzz_one!(ctx, mux, dcm[j], [
                    (tile_mutex "MUXBUS", "TEST"),
                    (tile_mutex mout, dcm[j])
                ], [
                    (pip (pin dcm[j]), (pin mout))
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
                fuzz_one!(ctx, mux, "PASS", [
                    (tile_mutex "MUXBUS", "TEST"),
                    (tile_mutex mout, min),
                    (related TileRelation::ClkDcm,
                        (pip (bel_pin obel, "DCM0"), (bel_pin obel, mout))),
                    (related TileRelation::ClkDcm,
                        (tile_mutex "MUXBUS", "USE"))
                ], [
                    (pip (pin min), (pin mout))
                ]);
            }
        }
    }
    {
        let tile = "CLK_HROW";
        let node_kind = backend.egrid.db.get_node(tile);
        let ctx = FuzzCtx {
            session,
            node_kind,
            bits: TileBits::ClkHrow,
            tile_name: tile,
            bel: BelId::from_idx(0),
            bel_name: "CLK_HROW",
        };
        let gclk_i: Vec<_> = (0..32).map(|i| &*format!("GCLK_I{i}").leak()).collect();
        for lr in ['L', 'R'] {
            for i in 0..8 {
                let gclk_o = &*format!("GCLK_O_{lr}{i}").leak();
                for j in 0..32 {
                    let bel_bufg = BelId::from_idx(j);
                    fuzz_one!(ctx, gclk_o, gclk_i[j], [
                        (global_mutex "BUFGCTRL_OUT", "USE"),
                        (tile_mutex "MODE", "TEST"),
                        (tile_mutex "IN", gclk_i[j]),
                        (tile_mutex "OUT", gclk_o),
                        (related TileRelation::Cfg,
                            (pip (bel_pin bel_bufg, "O"), (bel_pin bel_bufg, "GCLK")))
                    ], [
                        (pip (pin gclk_i[j]), (pin gclk_o))
                    ]);
                }
            }
        }
    }
    {
        let tile = "HCLK";
        let node_kind = backend.egrid.db.get_node(tile);
        let ctx = FuzzCtx {
            session,
            node_kind,
            bits: TileBits::Hclk,
            tile_name: tile,
            bel: BelId::from_idx(1),
            bel_name: "HCLK",
        };
        for i in 0..8 {
            let gclk_i = &*format!("GCLK_I{i}").leak();
            let gclk_o = &*format!("GCLK_O{i}").leak();
            let gclk_o_l = &*format!("GCLK_O_L{i}").leak();
            let gclk_o_r = &*format!("GCLK_O_R{i}").leak();
            let hclk = &*format!("HCLK{i}").leak();
            let obel = BelId::from_idx(0);
            fuzz_one!(ctx, hclk, "1", [
                (global_mutex "BUFGCTRL_OUT", "USE"),
                (related TileRelation::ClkHrow,
                    (tile_mutex "MODE", "USE")),
                (related TileRelation::ClkHrow,
                    (pip (bel_pin obel, "GCLK_I0"), (bel_pin obel, gclk_o_l))),
                (related TileRelation::ClkHrow,
                    (pip (bel_pin obel, "GCLK_I0"), (bel_pin obel, gclk_o_r)))
            ], [
                (pip (pin gclk_i), (pin gclk_o))
            ]);
        }
        for i in 0..2 {
            let rclk_i = &*format!("RCLK_I{i}").leak();
            let rclk_o = &*format!("RCLK_O{i}").leak();
            let rclk = &*format!("RCLK{i}").leak();
            fuzz_one!(ctx, rclk, "1", [
                (related TileRelation::Rclk,
                    (tile_mutex "RCLK_MODE", "USE")),
                (pip
                    (related_pin BelRelation::Rclk, "VRCLK0"),
                    (related_pin BelRelation::Rclk, rclk))
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
            let ctx = FuzzCtx {
                session,
                node_kind,
                bits: TileBits::Hclk,
                tile_name: tile,
                bel,
                bel_name: "RCLK",
            };
            for opin in ["RCLK0", "RCLK1"] {
                for ipin in [
                    "VRCLK0", "VRCLK1", "VRCLK_S0", "VRCLK_S1", "VRCLK_N0", "VRCLK_N1",
                ] {
                    fuzz_one!(ctx, opin, ipin, [
                        (tile_mutex "RCLK_MODE", "TEST"),
                        (tile_mutex opin, ipin)
                    ], [
                        (pip (pin ipin), (pin opin))
                    ]);
                }
            }
            let obel_rclk = bel;
            for bel_name in ["BUFR0", "BUFR1"] {
                let bel = node_data.bels.get(bel_name).unwrap().0;
                let ctx = FuzzCtx {
                    session,
                    node_kind,
                    bits: TileBits::Hclk,
                    tile_name: tile,
                    bel,
                    bel_name,
                };
                fuzz_one!(ctx, "PRESENT", "1", [], [(mode "BUFR")]);
                fuzz_one!(ctx, "ENABLE", "1", [(mode "BUFR")], [(pin "O")]);
                fuzz_enum!(ctx, "BUFR_DIVIDE", ["BYPASS", "1", "2", "3", "4", "5", "6", "7", "8"], [(mode "BUFR")]);
                for ckint in ["CKINT0", "CKINT1"] {
                    fuzz_one!(ctx, "IMUX", ckint, [
                        (mode "BUFR"),
                        (mutex "IMUX", ckint)
                    ], [
                        (pip (bel_pin obel_rclk, ckint), (pin "I"))
                    ]);
                }
                for obel_name in ["BUFIO0", "BUFIO1"] {
                    let obel = node_data.bels.get(obel_name).unwrap().0;
                    fuzz_one!(ctx, "IMUX", obel_name, [
                        (mode "BUFR"),
                        (mutex "IMUX", obel_name)
                    ], [
                        (pip (bel_pin obel, "O"), (pin "I"))
                    ]);
                }
            }
        }
        {
            let ctx = FuzzCtx {
                session,
                node_kind,
                bits: TileBits::Hclk,
                tile_name: tile,
                bel: node_data.bels.get("IOCLK").unwrap().0,
                bel_name: "IOCLK",
            };
            for i in 0..8 {
                let gclk_i = &*format!("GCLK_I{i}").leak();
                let gclk_o = &*format!("GCLK_O{i}").leak();
                let gclk_o_l = &*format!("GCLK_O_L{i}").leak();
                let gclk_o_r = &*format!("GCLK_O_R{i}").leak();
                let hclk = &*format!("HCLK{i}").leak();
                let obel = BelId::from_idx(0);
                fuzz_one!(ctx, hclk, "1", [
                    (global_mutex "BUFGCTRL_OUT", "USE"),
                    (related TileRelation::ClkHrow,
                        (tile_mutex "MODE", "USE")),
                    (related TileRelation::ClkHrow,
                        (pip (bel_pin obel, "GCLK_I0"), (bel_pin obel, gclk_o_l))),
                    (related TileRelation::ClkHrow,
                        (pip (bel_pin obel, "GCLK_I0"), (bel_pin obel, gclk_o_r)))
                ], [
                    (pip (pin gclk_i), (pin gclk_o))
                ]);
            }
            for i in 0..2 {
                let rclk_i = &*format!("RCLK_I{i}").leak();
                let rclk_o = &*format!("RCLK_O{i}").leak();
                let rclk = &*format!("RCLK{i}").leak();
                fuzz_one!(ctx, rclk, "1", [
                    (related TileRelation::Rclk,
                        (tile_mutex "RCLK_MODE", "USE")),
                    (pip
                        (related_pin BelRelation::Rclk, "VRCLK0"),
                        (related_pin BelRelation::Rclk, rclk))
                ], [
                    (pip (pin rclk_i), (pin rclk_o))
                ]);
            }

            for (obel_name, vioclk) in [("BUFIO0", "VIOCLK0"), ("BUFIO1", "VIOCLK1")] {
                let obel = node_data.bels.get(obel_name).unwrap().0;
                fuzz_one!(ctx, vioclk, "1", [
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
                    edev.grids[edev.grid_master].row_bufg() - edev.row_dcmiob.unwrap() > 24,
                ),
                _ => unreachable!(),
            };
            if has_s {
                for (o, i, oi, oo) in [
                    ("IOCLK_N0", "VIOCLK_N0", "VIOCLK0", "IOCLK0"),
                    ("IOCLK_N1", "VIOCLK_N1", "VIOCLK1", "IOCLK1"),
                ] {
                    fuzz_one!(ctx, o, "1", [
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
                    fuzz_one!(ctx, o, "1", [
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
            let ctx = FuzzCtx {
                session,
                node_kind,
                bits: TileBits::Hclk,
                tile_name: tile,
                bel: node_data.bels.get("IDELAYCTRL").unwrap().0,
                bel_name: "IDELAYCTRL",
            };
            fuzz_one!(ctx, "ENABLE", "1", [], [(mode "IDELAYCTRL")]);
            let obel = node_data.bels.get("IOCLK").unwrap().0;
            for i in 0..8 {
                let hclk = &*format!("HCLK{i}").leak();
                let gclk_o = &*format!("GCLK_O{i}").leak();
                fuzz_one!(ctx, "REFCLK", hclk, [
                    (mutex "REFCLK", hclk)
                ], [
                    (pip (bel_pin obel, gclk_o), (pin "REFCLK"))
                ]);
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
            let giob = &*format!("GIOB{i}").leak();
            let diff = ctx.state.get_diff(tile, bel, giob, "1");
            let [diff, diff_term] = &diff.split_tiles(&[
                &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
                &[16],
            ])[..] else {
                unreachable!()
            };
            ctx.tiledb
                .insert(tile, bel, giob, xlat_bit_wide(diff.clone()));
            ctx.tiledb.insert(
                term,
                "CLK_TERM",
                "GIOB_ENABLE",
                xlat_bitvec(vec![diff_term.clone()]),
            );
        }
        for i in 0..32 {
            let mux = &*format!("MUXBUS{i}").leak();
            let mut vals = vec![("NONE", Diff::default())];
            for j in 0..16 {
                let giob = &*format!("GIOB{j}").leak();
                vals.push((giob, ctx.state.get_diff(tile, bel, mux, giob)));
            }
            vals.push(("PASS", ctx.state.get_diff(tile, bel, mux, "PASS")));
            ctx.tiledb
                .insert(tile, bel, mux, xlat_enum_ocd(vals, OcdMode::Mux));
        }
    }
    for tile in ["CLK_DCM_B", "CLK_DCM_T"] {
        let bel = "CLK_DCM";
        for i in 0..32 {
            let mux = &*format!("MUXBUS{i}").leak();
            let mut vals = vec![("NONE", Diff::default())];
            for j in 0..24 {
                let giob = &*format!("DCM{j}").leak();
                vals.push((giob, ctx.state.get_diff(tile, bel, mux, giob)));
            }
            let has_other = if tile == "CLK_DCM_T" {
                edev.grids
                    .values()
                    .any(|grid| grid.regs - grid.reg_clk.to_idx() > 2)
            } else {
                edev.grids.values().any(|grid| grid.reg_clk.to_idx() > 2)
            };
            if has_other {
                vals.push(("PASS", ctx.state.get_diff(tile, bel, mux, "PASS")));
            }
            ctx.tiledb
                .insert(tile, bel, mux, xlat_enum_ocd(vals, OcdMode::Mux));
        }
    }
    {
        let tile = "CLK_HROW";
        let bel = "CLK_HROW";
        let gclk_i: Vec<_> = (0..32).map(|i| &*format!("GCLK_I{i}").leak()).collect();
        let gclk_o_l: Vec<_> = (0..8).map(|i| &*format!("GCLK_O_L{i}").leak()).collect();
        let gclk_o_r: Vec<_> = (0..8).map(|i| &*format!("GCLK_O_R{i}").leak()).collect();
        let mut inp_diffs = vec![];
        for i in 0..32 {
            let diff_l = ctx
                .state
                .peek_diff(tile, bel, gclk_o_l[0], gclk_i[i])
                .clone();
            let diff_r = ctx
                .state
                .peek_diff(tile, bel, gclk_o_r[0], gclk_i[i])
                .clone();
            let (_, _, diff) = Diff::split(diff_l, diff_r);
            inp_diffs.push(diff);
        }
        for (gclk_o, ttile, ttidx) in [(gclk_o_l, "HCLK_TERM_L", 3), (gclk_o_r, "HCLK_TERM_R", 4)] {
            for i in 0..8 {
                let mut inps = vec![("NONE", Diff::default())];
                for j in 0..32 {
                    let mut diff = ctx.state.get_diff(tile, bel, gclk_o[i], gclk_i[j]);
                    diff = diff.combine(&!&inp_diffs[j]);
                    let [diff, diff_term] = &diff.split_tiles(&[&[0, 1, 2], &[ttidx]])[..] else {
                        unreachable!()
                    };
                    inps.push((gclk_i[j], diff.clone()));
                    ctx.tiledb.insert(
                        ttile,
                        "HCLK_TERM",
                        "HCLK_ENABLE",
                        xlat_bit_wide(diff_term.clone()),
                    );
                }
                ctx.tiledb
                    .insert(tile, bel, gclk_o[i], xlat_enum_ocd(inps, OcdMode::Mux));
            }
        }
        for (i, diff) in inp_diffs.into_iter().enumerate() {
            ctx.tiledb.insert(
                tile,
                bel,
                format!("GCLK_I{i}_ENABLE"),
                xlat_bitvec(vec![diff]),
            );
        }
    }
    {
        let tile = "HCLK";
        let bel = "HCLK";
        for i in 0..8 {
            ctx.collect_bit(tile, bel, &*format!("HCLK{i}").leak(), "1");
        }
        for i in 0..2 {
            ctx.collect_bit(tile, bel, &*format!("RCLK{i}").leak(), "1");
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
            for mux in ["RCLK0", "RCLK1"] {
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
                ctx.collect_enum(tile, bel, "IMUX", &["CKINT0", "CKINT1", "BUFIO0", "BUFIO1"]);
            }
        }
        {
            let bel = "IOCLK";
            for i in 0..8 {
                ctx.collect_bit(tile, bel, &*format!("HCLK{i}").leak(), "1");
            }
            for i in 0..2 {
                ctx.collect_bit(tile, bel, &*format!("RCLK{i}").leak(), "1");
            }
            let diff0 = ctx.state.get_diff(tile, bel, "VIOCLK0", "1");
            let diff1 = ctx.state.get_diff(tile, bel, "VIOCLK1", "1");
            let (diff0, diff1, diffc) = Diff::split(diff0, diff1);
            ctx.tiledb
                .insert(tile, bel, "VIOCLK0", xlat_bitvec(vec![diff0]));
            ctx.tiledb
                .insert(tile, bel, "VIOCLK1", xlat_bitvec(vec![diff1]));
            ctx.tiledb
                .insert(tile, bel, "VIOCLK_ENABLE", xlat_bit_wide(diffc));
            let (has_s, has_n) = match tile {
                "HCLK_IOIS_DCI" | "HCLK_IOIS_LVDS" => (true, true),
                "HCLK_DCMIOB" | "HCLK_CENTER_ABOVE_CFG" => (false, true),
                "HCLK_IOBDCM" => (true, false),
                "HCLK_CENTER" => (
                    true,
                    edev.grids[edev.grid_master].row_bufg() - edev.row_dcmiob.unwrap() > 24,
                ),
                _ => unreachable!(),
            };
            if has_s {
                ctx.collect_bit(tile, bel, "IOCLK_N0", "1");
                ctx.collect_bit(tile, bel, "IOCLK_N1", "1");
            }
            if has_n {
                ctx.collect_bit(tile, bel, "IOCLK_S0", "1");
                ctx.collect_bit(tile, bel, "IOCLK_S1", "1");
            }
        }
        {
            let bel = "IDELAYCTRL";
            ctx.collect_bit(tile, bel, "ENABLE", "1");
            ctx.collect_enum(
                tile,
                bel,
                "REFCLK",
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
        for attr in ["IOCLK_N0", "IOCLK_N1", "IOCLK_S0", "IOCLK_S1"] {
            let item = ctx.tiledb.item("HCLK_IOIS_LVDS", "IOCLK", attr);
            ctx.tiledb.insert(tile, "IOCLK", attr, item.clone());
        }
    }
}
