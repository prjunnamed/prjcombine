use prjcombine_entity::EntityId;
use prjcombine_int::db::{Dir, IntDb, IntfWireInNaming, NodeTileId, TermInfo, WireKind};
use prjcombine_rawdump::{Coord, Part};

use prjcombine_rdintb::IntBuilder;

pub fn make_int_db(rd: &Part) -> IntDb {
    let mut builder = IntBuilder::new("virtex5", rd);

    builder.wire("PULLUP", WireKind::TiePullup, &["KEEP1_WIRE"]);
    builder.wire("GND", WireKind::Tie0, &["GND_WIRE"]);
    builder.wire("VCC", WireKind::Tie1, &["VCC_WIRE"]);

    for i in 0..10 {
        builder.wire(
            format!("GCLK{i}"),
            WireKind::ClkOut(i),
            &[format!("GCLK{i}")],
        );
    }
    for i in 0..4 {
        builder.wire(
            format!("RCLK{i}"),
            WireKind::ClkOut(10 + i),
            &[format!("RCLK{i}")],
        );
    }

    for (name, da, db, dbeg, dend, dmid) in [
        ("EL", Dir::E, Dir::E, None, None, None),
        ("ER", Dir::E, Dir::E, Some((0, Dir::S)), None, None),
        ("EN", Dir::E, Dir::N, None, None, None),
        ("ES", Dir::E, Dir::S, None, None, None),
        ("WL", Dir::W, Dir::W, Some((0, Dir::S)), None, None),
        ("WR", Dir::W, Dir::W, Some((2, Dir::N)), None, None),
        ("WN", Dir::W, Dir::N, None, Some((0, Dir::S)), None),
        ("WS", Dir::W, Dir::S, None, None, Some((0, Dir::S))),
        ("NL", Dir::N, Dir::N, Some((0, Dir::S)), None, None),
        ("NR", Dir::N, Dir::N, Some((2, Dir::N)), None, None),
        ("NE", Dir::N, Dir::E, None, None, Some((2, Dir::N))),
        ("NW", Dir::N, Dir::W, None, Some((2, Dir::N)), None),
        ("SL", Dir::S, Dir::S, Some((2, Dir::N)), None, None),
        ("SR", Dir::S, Dir::S, None, None, None),
        ("SE", Dir::S, Dir::E, None, None, None),
        ("SW", Dir::S, Dir::W, None, None, None),
    ] {
        for i in 0..3 {
            let beg;
            if let Some((xi, dbeg)) = dbeg {
                if xi == i {
                    let beg_x = builder.mux_out(
                        format!("DBL.{name}{i}.0.{dbeg}"),
                        &[&format!("{name}2BEG_{dbeg}{i}")],
                    );
                    beg = builder.branch(
                        beg_x,
                        !dbeg,
                        format!("DBL.{name}{i}.0"),
                        &[format!("{name}2BEG{i}")],
                    );
                } else {
                    beg = builder.mux_out(format!("DBL.{name}{i}.0"), &[format!("{name}2BEG{i}")]);
                }
            } else {
                beg = builder.mux_out(format!("DBL.{name}{i}.0"), &[format!("{name}2BEG{i}")]);
            }
            let mid = builder.branch(
                beg,
                da,
                format!("DBL.{name}{i}.1"),
                &[format!("{name}2MID{i}")],
            );
            if let Some((xi, dmid)) = dmid {
                if xi == i {
                    let mid_buf = builder.buf(
                        mid,
                        format!("DBL.{name}{i}.1.BUF"),
                        &[format!("{name}2MID_FAKE{i}")],
                    );
                    builder.branch(
                        mid_buf,
                        dmid,
                        format!("DBL.{name}{i}.1.{dmid}"),
                        &[format!("{name}2MID_{dmid}{i}")],
                    );
                }
            }
            let end = builder.branch(
                mid,
                db,
                format!("DBL.{name}{i}.2"),
                &[format!("{name}2END{i}")],
            );
            if let Some((xi, dend)) = dend {
                if xi == i {
                    builder.branch(
                        end,
                        dend,
                        format!("DBL.{name}{i}.2.{dend}"),
                        &[format!("{name}2END_{dend}{i}")],
                    );
                }
            }
        }
    }

    for (name, da, db, dbeg, dend, dmid) in [
        ("EL", Dir::E, Dir::E, None, None, None),
        ("ER", Dir::E, Dir::E, None, None, None),
        ("EN", Dir::E, Dir::N, None, None, None),
        ("ES", Dir::E, Dir::S, None, None, None),
        ("WL", Dir::W, Dir::W, Some((0, Dir::S)), None, None),
        ("WR", Dir::W, Dir::W, None, None, None),
        ("WN", Dir::W, Dir::N, None, Some((0, Dir::S)), None),
        ("WS", Dir::W, Dir::S, None, None, Some((0, Dir::S))),
        ("NL", Dir::N, Dir::N, None, None, None),
        ("NR", Dir::N, Dir::N, Some((2, Dir::N)), None, None),
        ("NE", Dir::N, Dir::E, None, None, Some((2, Dir::N))),
        ("NW", Dir::N, Dir::W, None, Some((2, Dir::N)), None),
        ("SL", Dir::S, Dir::S, None, None, None),
        ("SR", Dir::S, Dir::S, None, None, None),
        ("SE", Dir::S, Dir::E, None, None, None),
        ("SW", Dir::S, Dir::W, None, None, None),
    ] {
        for i in 0..3 {
            let beg;
            if let Some((xi, dbeg)) = dbeg {
                if xi == i {
                    let beg_x = builder.mux_out(
                        format!("PENT.{name}{i}.0.{dbeg}"),
                        &[&format!("{name}5BEG_{dbeg}{i}")],
                    );
                    beg = builder.branch(
                        beg_x,
                        !dbeg,
                        format!("PENT.{name}{i}.0"),
                        &[format!("{name}5BEG{i}")],
                    );
                } else {
                    beg = builder.mux_out(format!("PENT.{name}{i}.0"), &[format!("{name}5BEG{i}")]);
                }
            } else {
                beg = builder.mux_out(format!("PENT.{name}{i}.0"), &[format!("{name}5BEG{i}")]);
            }
            let a = builder.branch(
                beg,
                da,
                format!("PENT.{name}{i}.1"),
                &[format!("{name}5A{i}")],
            );
            let b = builder.branch(
                a,
                da,
                format!("PENT.{name}{i}.2"),
                &[format!("{name}5B{i}")],
            );
            let mid = builder.branch(
                b,
                da,
                format!("PENT.{name}{i}.3"),
                &[format!("{name}5MID{i}")],
            );
            if let Some((xi, dmid)) = dmid {
                if xi == i {
                    let mid_buf = builder.buf(
                        mid,
                        format!("PENT.{name}{i}.3.BUF"),
                        &[format!("{name}5MID_FAKE{i}")],
                    );
                    builder.branch(
                        mid_buf,
                        dmid,
                        format!("PENT.{name}{i}.3.{dmid}"),
                        &[format!("{name}5MID_{dmid}{i}")],
                    );
                }
            }
            let c = builder.branch(
                mid,
                db,
                format!("PENT.{name}{i}.4"),
                &[format!("{name}5C{i}")],
            );
            let end = builder.branch(
                c,
                db,
                format!("PENT.{name}{i}.5"),
                &[format!("{name}5END{i}")],
            );
            if let Some((xi, dend)) = dend {
                if xi == i {
                    builder.branch(
                        end,
                        dend,
                        format!("PENT.{name}{i}.5.{dend}"),
                        &[format!("{name}5END_{dend}{i}")],
                    );
                }
            }
        }
    }

    // The long wires.
    let mid = builder.wire("LH.9", WireKind::MultiOut, &["LH9"]);
    let mut prev = mid;
    let mut lh_all = vec![mid];
    for i in (0..9).rev() {
        prev = builder.multi_branch(prev, Dir::E, format!("LH.{i}"), &[format!("LH{i}")]);
        lh_all.push(prev);
    }
    let mut prev = mid;
    let mut lh_bh_e = Vec::new();
    for i in 10..19 {
        prev = builder.multi_branch(prev, Dir::W, format!("LH.{i}"), &[format!("LH{i}")]);
        lh_bh_e.push(prev);
        lh_all.push(prev);
    }
    let mid = builder.wire("LV.9", WireKind::MultiOut, &["LV9"]);
    let mut prev = mid;
    let mut lv_bh_n = Vec::new();
    for i in (0..9).rev() {
        prev = builder.multi_branch(prev, Dir::S, format!("LV.{i}"), &[format!("LV{i}")]);
        lv_bh_n.push(prev);
    }
    let mut prev = mid;
    let mut lv_bh_s = Vec::new();
    for i in 10..19 {
        prev = builder.multi_branch(prev, Dir::N, format!("LV.{i}"), &[format!("LV{i}")]);
        lv_bh_s.push(prev);
    }

    // The control inputs.
    for i in 0..2 {
        builder.mux_out(format!("IMUX.GFAN{i}"), &[format!("GFAN{i}")]);
    }
    let mut clk = vec![];
    for i in 0..2 {
        let w = builder.mux_out(format!("IMUX.CLK{i}"), &[format!("CLK_B{i}")]);
        clk.push(w);
    }
    for i in 0..4 {
        let w = builder.mux_out(format!("IMUX.CTRL{i}"), &[format!("CTRL{i}")]);
        builder.buf(w, format!("IMUX.CTRL{i}.SITE"), &[format!("CTRL_B{i}")]);
        let b = builder.buf(
            w,
            format!("IMUX.CTRL{i}.BOUNCE"),
            &[format!("CTRL_BOUNCE{i}")],
        );
        let dir = match i {
            0 => Dir::S,
            3 => Dir::N,
            _ => continue,
        };
        builder.branch(
            b,
            dir,
            format!("IMUX.CTRL{i}.BOUNCE.{dir}"),
            &[format!("CTRL_BOUNCE_{dir}{i}")],
        );
    }
    for i in 0..8 {
        let w = builder.mux_out(format!("IMUX.BYP{i}"), &[format!("BYP{i}")]);
        builder.buf(w, format!("IMUX.BYP{i}.SITE"), &[format!("BYP_B{i}")]);
        let b = builder.buf(
            w,
            format!("IMUX.BYP{i}.BOUNCE"),
            &[format!("BYP_BOUNCE{i}")],
        );
        let dir = match i {
            0 | 4 => Dir::S,
            3 | 7 => Dir::N,
            _ => continue,
        };
        builder.branch(
            b,
            dir,
            format!("IMUX.BYP{i}.BOUNCE.{dir}"),
            &[format!("BYP_BOUNCE_{dir}{i}")],
        );
    }
    for i in 0..8 {
        let w = builder.mux_out(format!("IMUX.FAN{i}"), &[format!("FAN{i}")]);
        builder.buf(w, format!("IMUX.FAN{i}.SITE"), &[format!("FAN_B{i}")]);
        let b = builder.buf(
            w,
            format!("IMUX.FAN{i}.BOUNCE"),
            &[format!("FAN_BOUNCE{i}")],
        );
        let dir = match i {
            0 => Dir::S,
            7 => Dir::N,
            _ => continue,
        };
        builder.branch(
            b,
            dir,
            format!("IMUX.FAN{i}.BOUNCE.{dir}"),
            &[format!("FAN_BOUNCE_{dir}{i}")],
        );
    }
    for i in 0..48 {
        builder.mux_out(format!("IMUX.IMUX{i}"), &[format!("IMUX_B{i}")]);
    }

    for i in 0..24 {
        let w = builder.logic_out(format!("OUT{i}"), &[format!("LOGIC_OUTS{i}")]);
        let dir = match i {
            15 | 17 => Dir::N,
            12 | 18 => Dir::S,
            _ => continue,
        };
        builder.branch(
            w,
            dir,
            format!("OUT{i}.{dir}.DBL"),
            &[format!("LOGIC_OUTS_{dir}{i}")],
        );
        builder.branch(
            w,
            dir,
            format!("OUT{i}.{dir}.PENT"),
            &[format!("LOGIC_OUTS_{dir}1_{i}")],
        );
    }

    for i in 0..4 {
        builder.test_out(
            format!("TEST{i}"),
            &[
                format!("INT_INTERFACE_BLOCK_INPS_B{i}"),
                format!("PPC_L_INT_INTERFACE_BLOCK_INPS_B{i}"),
                format!("PPC_R_INT_INTERFACE_BLOCK_INPS_B{i}"),
                format!("GTX_LEFT_INT_INTERFACE_BLOCK_INPS_B{i}"),
            ],
        );
    }

    builder.extract_main_passes();

    builder.node_type("INT", "INT", "INT");

    builder.extract_term_buf("TERM.W", Dir::W, "L_TERM_INT", "TERM.W", &[]);
    builder.extract_term_buf("TERM.W", Dir::W, "GTX_L_TERM_INT", "TERM.W", &[]);
    builder.extract_term_buf("TERM.E", Dir::E, "R_TERM_INT", "TERM.E", &[]);
    builder.make_blackhole_term("TERM.E.HOLE", Dir::E, &lh_bh_e);
    builder.make_blackhole_term("TERM.S.HOLE", Dir::S, &lv_bh_s);
    builder.make_blackhole_term("TERM.N.HOLE", Dir::N, &lv_bh_n);
    let forced = [
        (
            builder.find_wire("PENT.NW2.5.N"),
            builder.find_wire("PENT.WN0.5"),
        ),
        (
            builder.find_wire("PENT.WN0.5"),
            builder.find_wire("PENT.WS2.4"),
        ),
    ];
    builder.extract_term_buf("TERM.S.PPC", Dir::S, "PPC_T_TERM", "TERM.S.PPC", &forced);
    let forced = [
        (
            builder.find_wire("PENT.NR2.0"),
            builder.find_wire("PENT.WL0.0.S"),
        ),
        (
            builder.find_wire("PENT.SL0.1"),
            builder.find_wire("PENT.NR2.0"),
        ),
    ];
    builder.extract_term_buf("TERM.N.PPC", Dir::N, "PPC_B_TERM", "TERM.N.PPC", &forced);

    for &xy_l in rd.tiles_by_kind_name("INT_BUFS_L") {
        let mut xy_r = xy_l;
        while !matches!(
            &rd.tile_kinds.key(rd.tiles[&xy_r].kind)[..],
            "INT_BUFS_R" | "INT_BUFS_R_MON"
        ) {
            xy_r.x += 1;
        }
        if xy_l.y < 10 || xy_l.y >= rd.height - 10 {
            // wheeee.
            continue;
        }
        let int_w_xy = builder.walk_to_int(xy_l, Dir::W).unwrap();
        let int_e_xy = builder.walk_to_int(xy_l, Dir::E).unwrap();
        builder.extract_pass_tile(
            "INT_BUFS.W",
            Dir::W,
            int_e_xy,
            Some(xy_r),
            Some(xy_l),
            Some("INT_BUFS.W"),
            None,
            int_w_xy,
            &lh_all,
        );
        builder.extract_pass_tile(
            "INT_BUFS.E",
            Dir::E,
            int_w_xy,
            Some(xy_l),
            Some(xy_r),
            Some("INT_BUFS.E"),
            None,
            int_e_xy,
            &lh_all,
        );
    }
    for &xy_l in rd.tiles_by_kind_name("L_TERM_PPC") {
        let mut xy_r = xy_l;
        while rd.tile_kinds.key(rd.tiles[&xy_r].kind) != "R_TERM_PPC" {
            xy_r.x += 1;
        }
        let int_w_xy = builder.walk_to_int(xy_l, Dir::W).unwrap();
        let int_e_xy = builder.walk_to_int(xy_l, Dir::E).unwrap();
        builder.extract_pass_tile(
            "PPC.W",
            Dir::W,
            int_e_xy,
            Some(xy_r),
            Some(xy_l),
            Some("PPC.W"),
            None,
            int_w_xy,
            &lh_all,
        );
        builder.extract_pass_tile(
            "PPC.E",
            Dir::E,
            int_w_xy,
            Some(xy_l),
            Some(xy_r),
            Some("PPC.E"),
            None,
            int_e_xy,
            &lh_all,
        );
    }

    builder.extract_intf("INTF", Dir::E, "INT_INTERFACE", "INTF", true);
    for (n, tkn) in [
        ("GTX_LEFT", "GTX_LEFT_INT_INTERFACE"),
        ("GTP", "GTP_INT_INTERFACE"),
        ("EMAC", "EMAC_INT_INTERFACE"),
        ("PCIE", "PCIE_INT_INTERFACE"),
        ("PPC_L", "PPC_L_INT_INTERFACE"),
        ("PPC_R", "PPC_R_INT_INTERFACE"),
    ] {
        builder.extract_intf("INTF.DELAY", Dir::E, tkn, format!("INTF.{n}"), true);
    }

    let mps = builder.db.terms.get("MAIN.S").unwrap().1.clone();
    builder.db.terms.insert("MAIN.NHOLE.S".to_string(), mps);
    let mut mpn = builder.db.terms.get("MAIN.N").unwrap().1.clone();
    for w in lv_bh_n {
        mpn.wires.insert(w, TermInfo::BlackHole);
    }
    builder.db.terms.insert("MAIN.NHOLE.N".to_string(), mpn);

    for tkn in ["CLBLL", "CLBLM"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let int_xy = Coord {
                x: xy.x - 1,
                y: xy.y,
            };
            builder.extract_xnode(
                tkn,
                xy,
                &[],
                &[int_xy],
                tkn,
                &[
                    builder
                        .bel_xy("SLICE0", "SLICE", 0, 0)
                        .pin_name_only("CIN", 0)
                        .pin_name_only("COUT", 1),
                    builder
                        .bel_xy("SLICE1", "SLICE", 1, 0)
                        .pin_name_only("CIN", 0)
                        .pin_name_only("COUT", 1),
                ],
                &[],
            );
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("BRAM").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let n = builder.db.get_node_naming("INTF");
        for dy in 0..5 {
            int_xy.push(Coord {
                x: xy.x - 2,
                y: xy.y + dy,
            });
            intf_xy.push((
                Coord {
                    x: xy.x - 1,
                    y: xy.y + dy,
                },
                n,
            ));
        }
        builder.extract_xnode_bels_intf(
            "BRAM",
            xy,
            &[],
            &int_xy,
            &intf_xy,
            "BRAM",
            &[builder
                .bel_xy("BRAM", "RAMB36", 0, 0)
                .pins_name_only(&[
                    "CASCADEOUTLATA",
                    "CASCADEOUTLATB",
                    "CASCADEOUTREGA",
                    "CASCADEOUTREGB",
                ])
                .pin_name_only("CASCADEINLATA", 1)
                .pin_name_only("CASCADEINLATB", 1)
                .pin_name_only("CASCADEINREGA", 1)
                .pin_name_only("CASCADEINREGB", 1)],
        );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("HCLK_BRAM").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let n = builder.db.get_node_naming("INTF");
        for dy in 0..5 {
            int_xy.push(Coord {
                x: xy.x - 2,
                y: xy.y + 1 + dy,
            });
            intf_xy.push((
                Coord {
                    x: xy.x - 1,
                    y: xy.y + 1 + dy,
                },
                n,
            ));
        }
        let bram_xy = Coord {
            x: xy.x,
            y: xy.y + 1,
        };
        builder.extract_xnode_bels_intf(
            "PMVBRAM",
            xy,
            &[bram_xy],
            &int_xy,
            &intf_xy,
            "PMVBRAM",
            &[builder.bel_xy("PMVBRAM", "PMVBRAM", 0, 0)],
        );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("DSP").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let n = builder.db.get_node_naming("INTF");
        for dy in 0..5 {
            int_xy.push(Coord {
                x: xy.x - 2,
                y: xy.y + dy,
            });
            intf_xy.push((
                Coord {
                    x: xy.x - 1,
                    y: xy.y + dy,
                },
                n,
            ));
        }

        let mut bels_dsp = vec![];
        for i in 0..2 {
            let mut bel = builder.bel_xy(format!("DSP{i}"), "DSP48", 0, i);
            let buf_cnt = match i {
                0 => 0,
                1 => 1,
                _ => unreachable!(),
            };
            bel = bel.pin_name_only("MULTSIGNIN", 0);
            bel = bel.pin_name_only("MULTSIGNOUT", buf_cnt);
            bel = bel.pin_name_only("CARRYCASCIN", 0);
            bel = bel.pin_name_only("CARRYCASCOUT", buf_cnt);
            for j in 0..30 {
                bel = bel.pin_name_only(&format!("ACIN{j}"), 0);
                bel = bel.pin_name_only(&format!("ACOUT{j}"), buf_cnt);
            }
            for j in 0..18 {
                bel = bel.pin_name_only(&format!("BCIN{j}"), 0);
                bel = bel.pin_name_only(&format!("BCOUT{j}"), buf_cnt);
            }
            for j in 0..48 {
                bel = bel.pin_name_only(&format!("PCIN{j}"), 0);
                bel = bel.pin_name_only(&format!("PCOUT{j}"), buf_cnt);
            }
            bels_dsp.push(bel);
        }
        builder.extract_xnode_bels_intf("DSP", xy, &[], &int_xy, &intf_xy, "DSP", &bels_dsp);
    }

    if let Some(&xy) = rd.tiles_by_kind_name("EMAC").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let n = builder.db.get_node_naming("INTF.EMAC");
        for dy in 0..10 {
            int_xy.push(Coord {
                x: xy.x - 2,
                y: xy.y + dy,
            });
            intf_xy.push((
                Coord {
                    x: xy.x - 1,
                    y: xy.y + dy,
                },
                n,
            ));
        }
        builder.extract_xnode_bels_intf(
            "EMAC",
            xy,
            &[],
            &int_xy,
            &intf_xy,
            "EMAC",
            &[builder.bel_xy("EMAC", "TEMAC", 0, 0)],
        );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("PCIE_B").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let n = builder.db.get_node_naming("INTF.PCIE");
        for by in [xy.y - 11, xy.y, xy.y + 11, xy.y + 22] {
            for dy in 0..10 {
                int_xy.push(Coord {
                    x: xy.x - 2,
                    y: by + dy,
                });
                intf_xy.push((
                    Coord {
                        x: xy.x - 1,
                        y: by + dy,
                    },
                    n,
                ));
            }
        }
        builder.extract_xnode_bels_intf(
            "PCIE",
            xy,
            &[Coord {
                x: xy.x,
                y: xy.y + 22,
            }],
            &int_xy,
            &intf_xy,
            "PCIE",
            &[builder.bel_xy("PCIE", "PCIE", 0, 0)],
        );
    }

    if let Some((_, intf)) = builder.db.node_namings.get_mut("INTF.PPC_R") {
        intf.intf_wires_in.insert(
            (NodeTileId::from_idx(0), clk[0]),
            IntfWireInNaming::Buf(
                "PPC_R_INT_INTERFACE_FB_CLK_B0".to_string(),
                "INT_INTERFACE_CLK_B0".to_string(),
            ),
        );
        intf.intf_wires_in.insert(
            (NodeTileId::from_idx(0), clk[1]),
            IntfWireInNaming::Buf(
                "PPC_R_INT_INTERFACE_FB_CLK_B1".to_string(),
                "INT_INTERFACE_CLK_B1".to_string(),
            ),
        );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("PPC_B").iter().next() {
        let ppc_t_xy = Coord {
            x: xy.x,
            y: xy.y + 22,
        };
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let nl = builder.db.get_node_naming("INTF.PPC_L");
        let nr = builder.db.get_node_naming("INTF.PPC_R");
        for by in [xy.y - 10, xy.y + 1, xy.y + 12, xy.y + 23] {
            for dy in 0..10 {
                int_xy.push(Coord {
                    x: xy.x - 11,
                    y: by + dy,
                });
                intf_xy.push((
                    Coord {
                        x: xy.x - 10,
                        y: by + dy,
                    },
                    nl,
                ));
            }
        }
        for by in [xy.y - 10, xy.y + 1, xy.y + 12, xy.y + 23] {
            for dy in 0..10 {
                int_xy.push(Coord {
                    x: xy.x + 20,
                    y: by + dy,
                });
                intf_xy.push((
                    Coord {
                        x: xy.x + 21,
                        y: by + dy,
                    },
                    nr,
                ));
            }
        }

        builder.extract_xnode_bels_intf(
            "PPC",
            xy,
            &[ppc_t_xy],
            &int_xy,
            &intf_xy,
            "PPC",
            &[builder.bel_xy("PPC", "PPC440", 0, 0)],
        );
    }

    builder.build()
}
