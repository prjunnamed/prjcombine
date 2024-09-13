use prjcombine_int::db::{Dir, IntDb, IntfWireInNaming, NodeTileId, TermInfo, WireKind};
use prjcombine_rawdump::{Coord, Part};
use unnamed_entity::EntityId;

use prjcombine_rdintb::IntBuilder;

pub fn make_int_db(rd: &Part) -> IntDb {
    let mut builder = IntBuilder::new("virtex5", rd);

    builder.wire("PULLUP", WireKind::TiePullup, &["KEEP1_WIRE"]);
    builder.wire("GND", WireKind::Tie0, &["GND_WIRE"]);
    builder.wire("VCC", WireKind::Tie1, &["VCC_WIRE"]);

    for i in 0..10 {
        builder.wire(
            format!("HCLK{i}"),
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
        let int_w_xy = builder.walk_to_int(xy_l, Dir::W, false).unwrap();
        let int_e_xy = builder.walk_to_int(xy_l, Dir::E, false).unwrap();
        builder.extract_pass_tile(
            "INT_BUFS.W",
            Dir::W,
            int_e_xy,
            Some(xy_r),
            Some(xy_l),
            Some("INT_BUFS.W"),
            None,
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
        let int_w_xy = builder.walk_to_int(xy_l, Dir::W, false).unwrap();
        let int_e_xy = builder.walk_to_int(xy_l, Dir::E, false).unwrap();
        builder.extract_pass_tile(
            "PPC.W",
            Dir::W,
            int_e_xy,
            Some(xy_r),
            Some(xy_l),
            Some("PPC.W"),
            None,
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
            let int_xy = xy.delta(-1, 0);
            builder.extract_xnode_bels(
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
            );
        }
    }

    let intf = builder.db.get_node_naming("INTF");

    if let Some(&xy) = rd.tiles_by_kind_name("BRAM").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        for dy in 0..5 {
            int_xy.push(xy.delta(-2, dy));
            intf_xy.push((xy.delta(-1, dy), intf));
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
        for dy in 0..5 {
            int_xy.push(xy.delta(-2, 1 + dy));
            intf_xy.push((xy.delta(-1, 1 + dy), intf));
        }
        let bram_xy = xy.delta(0, 1);
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
        for dy in 0..5 {
            int_xy.push(xy.delta(-2, dy));
            intf_xy.push((xy.delta(-1, dy), intf));
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
        let intf_emac = builder.db.get_node_naming("INTF.EMAC");
        for dy in 0..10 {
            int_xy.push(xy.delta(-2, dy));
            intf_xy.push((xy.delta(-1, dy), intf_emac));
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
        let intf_pcie = builder.db.get_node_naming("INTF.PCIE");
        for by in [-11, 0, 11, 22] {
            for dy in 0..10 {
                int_xy.push(xy.delta(-2, by + dy));
                intf_xy.push((xy.delta(-1, by + dy), intf_pcie));
            }
        }
        builder.extract_xnode_bels_intf(
            "PCIE",
            xy,
            &[xy.delta(0, 22)],
            &int_xy,
            &intf_xy,
            "PCIE",
            &[builder.bel_xy("PCIE", "PCIE", 0, 0)],
        );
    }

    if let Some((_, intf)) = builder.db.node_namings.get_mut("INTF.PPC_R") {
        intf.intf_wires_in.insert(
            (NodeTileId::from_idx(0), clk[0]),
            IntfWireInNaming::Buf {
                name_out: "PPC_R_INT_INTERFACE_FB_CLK_B0".to_string(),
                name_in: "INT_INTERFACE_CLK_B0".to_string(),
            },
        );
        intf.intf_wires_in.insert(
            (NodeTileId::from_idx(0), clk[1]),
            IntfWireInNaming::Buf {
                name_out: "PPC_R_INT_INTERFACE_FB_CLK_B1".to_string(),
                name_in: "INT_INTERFACE_CLK_B1".to_string(),
            },
        );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("PPC_B").iter().next() {
        let ppc_t_xy = xy.delta(0, 22);
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let intf_ppc_l = builder.db.get_node_naming("INTF.PPC_L");
        let intf_ppc_r = builder.db.get_node_naming("INTF.PPC_R");
        for by in [-10, 1, 12, 23] {
            for dy in 0..10 {
                int_xy.push(xy.delta(-11, by + dy));
                intf_xy.push((xy.delta(-10, by + dy), intf_ppc_l));
            }
        }
        for by in [-10, 1, 12, 23] {
            for dy in 0..10 {
                int_xy.push(xy.delta(20, by + dy));
                intf_xy.push((xy.delta(21, by + dy), intf_ppc_r));
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

    if let Some(&xy) = rd.tiles_by_kind_name("CFG_CENTER").iter().next() {
        let mut bels = vec![];
        for i in 0..32 {
            bels.push(
                builder
                    .bel_xy(format!("BUFGCTRL{i}"), "BUFGCTRL", 0, i)
                    .raw_tile(1)
                    .pins_name_only(&["I0", "I1", "O"])
                    .extra_wire("GCLK", &[format!("CLK_BUFGMUX_GCLKP{i}")])
                    .extra_wire(
                        "GFB",
                        &[if i < 16 {
                            format!("CLK_BUFGMUX_GFB_BOT_{i}")
                        } else {
                            format!("CLK_BUFGMUX_GFB_TOP_{i}", i = i - 16)
                        }],
                    )
                    .extra_int_out("I0MUX", &[format!("CLK_BUFGMUX_PREMUX0_CLK{i}")])
                    .extra_int_out("I1MUX", &[format!("CLK_BUFGMUX_PREMUX1_CLK{i}")])
                    .extra_int_in(
                        "CKINT0",
                        &[format!(
                            "CLK_BUFGMUX_CKINT0_{ii}",
                            ii = if i < 16 { i } else { i ^ 15 }
                        )],
                    )
                    .extra_int_in(
                        "CKINT1",
                        &[format!(
                            "CLK_BUFGMUX_CKINT1_{ii}",
                            ii = if i < 16 { i } else { i ^ 15 }
                        )],
                    )
                    .extra_wire(
                        "MUXBUS0",
                        &[if i < 16 {
                            format!("CLK_BUFGMUX_MUXED_IN_CLKB_P{ii}", ii = i * 2)
                        } else {
                            format!("CLK_BUFGMUX_MUXED_IN_CLKT_P{ii}", ii = i * 2 - 32)
                        }],
                    )
                    .extra_wire(
                        "MUXBUS1",
                        &[if i < 16 {
                            format!("CLK_BUFGMUX_MUXED_IN_CLKB_P{ii}", ii = i * 2 + 1)
                        } else {
                            format!("CLK_BUFGMUX_MUXED_IN_CLKT_P{ii}", ii = i * 2 - 32 + 1)
                        }],
                    ),
            );
        }
        let mut bel_mgtclk_b = builder.bel_virtual("BUFG_MGTCLK_B").raw_tile(1);
        let mut bel_mgtclk_t = builder.bel_virtual("BUFG_MGTCLK_T").raw_tile(1);
        for i in 0..5 {
            for lr in ['L', 'R'] {
                let ii = if lr == 'L' { 4 - i } else { i };
                bel_mgtclk_b = bel_mgtclk_b
                    .extra_wire(
                        format!("MGT_O_{lr}{i}"),
                        &[format!("CLK_BUFGMUX_{lr}MGT_CLK_BOT{ii}")],
                    )
                    .extra_wire_force(
                        format!("MGT_I_{lr}{i}"),
                        format!("CLK_BUFGMUX_MGT_CLKP_{lr}BOT{ii}"),
                    );
                bel_mgtclk_t = bel_mgtclk_t
                    .extra_wire(
                        format!("MGT_O_{lr}{i}"),
                        &[format!("CLK_BUFGMUX_{lr}MGT_CLK_TOP{ii}")],
                    )
                    .extra_wire_force(
                        format!("MGT_I_{lr}{i}"),
                        format!("CLK_BUFGMUX_MGT_CLKP_{lr}TOP{ii}"),
                    );
            }
        }
        let mut bel_sysmon = builder
            .bel_xy("SYSMON", "SYSMON", 0, 0)
            .pins_name_only(&["VP", "VN"]);
        for i in 0..16 {
            bel_sysmon = bel_sysmon
                .pin_name_only(&format!("VAUXP{i}"), 1)
                .pin_name_only(&format!("VAUXN{i}"), 1);
        }
        bels.extend([
            builder.bel_xy("BSCAN0", "BSCAN", 0, 0),
            builder.bel_xy("BSCAN1", "BSCAN", 0, 1),
            builder.bel_xy("BSCAN2", "BSCAN", 0, 2),
            builder.bel_xy("BSCAN3", "BSCAN", 0, 3),
            builder.bel_xy("ICAP0", "ICAP", 0, 0),
            builder.bel_xy("ICAP1", "ICAP", 0, 1),
            builder.bel_single("PMV", "PMV"),
            builder.bel_single("STARTUP", "STARTUP"),
            builder.bel_single("JTAGPPC", "JTAGPPC"),
            builder.bel_single("FRAME_ECC", "FRAME_ECC"),
            builder.bel_single("DCIRESET", "DCIRESET"),
            builder.bel_single("CAPTURE", "CAPTURE"),
            builder.bel_single("USR_ACCESS", "USR_ACCESS_SITE"),
            builder.bel_single("KEY_CLEAR", "KEY_CLEAR"),
            builder.bel_single("EFUSE_USR", "EFUSE_USR"),
            bel_sysmon,
            builder
                .bel_xy("IPAD.VP", "IPAD", 0, 0)
                .pins_name_only(&["O"]),
            builder
                .bel_xy("IPAD.VN", "IPAD", 0, 1)
                .pins_name_only(&["O"]),
            bel_mgtclk_b,
            bel_mgtclk_t,
        ]);
        let mut xn = builder
            .xnode("CFG", "CFG", xy)
            .raw_tile(xy.delta(1, 0))
            .num_tiles(20);
        for i in 0..10 {
            xn = xn.ref_int(xy.delta(-4, -10 + (i as i32)), i);
            xn = xn.ref_single(xy.delta(-3, -10 + (i as i32)), i, intf);
        }
        for i in 0..10 {
            xn = xn.ref_int(xy.delta(-4, 1 + (i as i32)), i + 10);
            xn = xn.ref_single(xy.delta(-3, 1 + (i as i32)), i + 10, intf);
        }
        for bel in bels {
            xn = xn.bel(bel);
        }
        xn.extract();
    }

    for tkn in ["LIOB", "LIOB_MON", "CIOB", "RIOB"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let ioi_xy = xy.delta(if tkn == "LIOB" { 4 } else { -1 }, 0);
            let int_xy = builder.walk_to_int(ioi_xy, Dir::W, false).unwrap();
            let intf_xy = int_xy.delta(1, 0);
            let bel_ilogic0 = builder
                .bel_xy("ILOGIC0", "ILOGIC", 0, 0)
                .pins_name_only(&[
                    "SHIFTIN1",
                    "SHIFTIN2",
                    "SHIFTOUT1",
                    "SHIFTOUT2",
                    "D",
                    "DDLY",
                    "TFB",
                    "OFB",
                    "CLK",
                    "CLKB",
                    "OCLK",
                ])
                .extra_wire("I_IOB", &["IOI_IBUF1"]);
            let bel_ilogic1 = builder
                .bel_xy("ILOGIC1", "ILOGIC", 0, 1)
                .pins_name_only(&[
                    "SHIFTIN1",
                    "SHIFTIN2",
                    "SHIFTOUT1",
                    "SHIFTOUT2",
                    "D",
                    "DDLY",
                    "TFB",
                    "OFB",
                    "CLK",
                    "CLKB",
                    "OCLK",
                ])
                .extra_wire("I_IOB", &["IOI_IBUF0"])
                .extra_wire_force("CLKOUT", "IOI_I_2GCLK0");
            let bel_ologic0 = builder
                .bel_xy("OLOGIC0", "OLOGIC", 0, 0)
                .pins_name_only(&[
                    "SHIFTIN1",
                    "SHIFTIN2",
                    "SHIFTOUT1",
                    "SHIFTOUT2",
                    "CLK",
                    "CLKDIV",
                    "OQ",
                ])
                .extra_wire("T_IOB", &["IOI_T1"])
                .extra_wire("O_IOB", &["IOI_O1"])
                .extra_int_out("CLKMUX", &["IOI_OCLKP_1"])
                .extra_int_out("CLKDIVMUX", &["IOI_OCLKDIV1"])
                .extra_int_in("CKINT", &["IOI_IMUX_B4"])
                .extra_int_in("CKINT_DIV", &["IOI_IMUX_B1"]);
            let bel_ologic1 = builder
                .bel_xy("OLOGIC1", "OLOGIC", 0, 1)
                .pins_name_only(&[
                    "SHIFTIN1",
                    "SHIFTIN2",
                    "SHIFTOUT1",
                    "SHIFTOUT2",
                    "CLK",
                    "CLKDIV",
                    "OQ",
                ])
                .extra_wire("T_IOB", &["IOI_T0"])
                .extra_wire("O_IOB", &["IOI_O0"])
                .extra_int_out("CLKMUX", &["IOI_OCLKP_0"])
                .extra_int_out("CLKDIVMUX", &["IOI_OCLKDIV0"])
                .extra_int_in("CKINT", &["IOI_IMUX_B10"])
                .extra_int_in("CKINT_DIV", &["IOI_IMUX_B7"]);
            let bel_iodelay0 = builder
                .bel_xy("IODELAY0", "IODELAY", 0, 0)
                .pins_name_only(&["IDATAIN", "ODATAIN", "T", "DATAOUT"]);
            let bel_iodelay1 = builder
                .bel_xy("IODELAY1", "IODELAY", 0, 1)
                .pins_name_only(&["IDATAIN", "ODATAIN", "T", "DATAOUT"]);

            let mut bel_iob0 = builder
                .bel_xy("IOB0", "IOB", 0, 0)
                .raw_tile(1)
                .pins_name_only(&["I", "O", "T", "PADOUT", "DIFFI_IN", "DIFFO_OUT", "DIFFO_IN"]);
            let mut bel_iob1 = builder
                .bel_xy("IOB1", "IOB", 0, 1)
                .raw_tile(1)
                .pins_name_only(&["I", "O", "T", "PADOUT", "DIFFI_IN", "DIFFO_OUT", "DIFFO_IN"]);
            match tkn {
                "LIOB" => {
                    bel_iob0 = bel_iob0.extra_wire_force("MONITOR", "LIOB_MONITOR_N");
                    bel_iob1 = bel_iob1.extra_wire_force("MONITOR", "LIOB_MONITOR_P");
                }
                "LIOB_MON" => {
                    bel_iob0 = bel_iob0.extra_wire_force("MONITOR", "LIOB_MON_MONITOR_N");
                    bel_iob1 = bel_iob1.extra_wire_force("MONITOR", "LIOB_MON_MONITOR_P");
                }
                _ => (),
            }
            let mut bel_ioi_clk = builder
                .bel_virtual("IOI_CLK")
                .extra_int_in("CKINT0", &["IOI_IMUX_B5"])
                .extra_int_in("CKINT1", &["IOI_IMUX_B11"])
                .extra_wire("ICLK0", &["IOI_ICLKP_1"])
                .extra_wire("ICLK1", &["IOI_ICLKP_0"]);
            for i in 0..4 {
                bel_ioi_clk =
                    bel_ioi_clk.extra_wire(format!("IOCLK{i}"), &[format!("IOI_IOCLKP{i}")]);
            }
            for i in 0..4 {
                bel_ioi_clk =
                    bel_ioi_clk.extra_wire(format!("RCLK{i}"), &[format!("IOI_RCLK_FORIO_P{i}")]);
            }
            for i in 0..10 {
                bel_ioi_clk =
                    bel_ioi_clk.extra_wire(format!("HCLK{i}"), &[format!("IOI_LEAF_GCLK_P{i}")]);
            }
            builder
                .xnode("IO", tkn, ioi_xy)
                .raw_tile(xy)
                .ref_int(int_xy, 0)
                .ref_single(intf_xy, 0, intf)
                .bel(bel_ilogic0)
                .bel(bel_ilogic1)
                .bel(bel_ologic0)
                .bel(bel_ologic1)
                .bel(bel_iodelay0)
                .bel(bel_iodelay1)
                .bel(bel_iob0)
                .bel(bel_iob1)
                .bel(bel_ioi_clk)
                .extract();
        }
    }

    for tkn in ["CMT_BOT", "CMT_TOP"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bels = vec![];
            for i in 0..2 {
                bels.push(
                    builder
                        .bel_xy(format!("DCM{i}"), "DCM_ADV", 0, i)
                        .pins_name_only(&[
                            "CLK0",
                            "CLK90",
                            "CLK180",
                            "CLK270",
                            "CLK2X",
                            "CLK2X180",
                            "CLKDV",
                            "CLKFX",
                            "CLKFX180",
                            "CONCUR",
                            "CLKIN",
                            "CLKFB",
                            "SKEWCLKIN1",
                            "SKEWCLKIN2",
                        ])
                        .extra_int_in("CKINT0", &[format!("CMT_DCM_{i}_SE_CLK_IN0")])
                        .extra_int_in("CKINT1", &[format!("CMT_DCM_{i}_SE_CLK_IN1")])
                        .extra_int_in("CKINT2", &[format!("CMT_DCM_{i}_SE_CLK_IN2")])
                        .extra_wire("CLKIN_TEST", &[format!("CMT_DCM_{i}_CLKIN_TEST")])
                        .extra_wire("CLKFB_TEST", &[format!("CMT_DCM_{i}_CLKFB_TEST")])
                        .extra_wire("MUXED_CLK", &[format!("CMT_DCM_{i}_MUXED_CLK")]),
                );
            }
            bels.push(
                builder
                    .bel_xy("PLL", "PLL_ADV", 0, 0)
                    .pins_name_only(&[
                        "CLKIN1",
                        "CLKIN2",
                        "CLKFBIN",
                        "SKEWCLKIN1",
                        "SKEWCLKIN2",
                        "CLKOUT0",
                        "CLKOUT1",
                        "CLKOUT2",
                        "CLKOUT3",
                        "CLKOUT4",
                        "CLKOUT5",
                        "CLKFBOUT",
                        "CLKOUTDCM0",
                        "CLKOUTDCM1",
                        "CLKOUTDCM2",
                        "CLKOUTDCM3",
                        "CLKOUTDCM4",
                        "CLKOUTDCM5",
                        "CLKFBDCM",
                    ])
                    .extra_int_in("CKINT0", &["CMT_PLL_SE_CLK_IN0"])
                    .extra_int_in("CKINT1", &["CMT_PLL_SE_CLK_IN1"])
                    .extra_wire("CLKIN1_TEST", &["CMT_PLL_CLKIN1_TEST"])
                    .extra_wire("CLKINFB_TEST", &["CMT_PLL_CLKINFB_TEST"])
                    .extra_wire("CLKFBDCM_TEST", &["CMT_PLL_CLKFBDCM_TEST"])
                    .extra_wire("CLK_DCM_MUX", &["CMT_PLL_CLK_DCM_MUX"])
                    .extra_wire("CLK_FB_FROM_DCM", &["CMT_PLL_CLK_FB_FROM_DCM"])
                    .extra_wire("CLK_TO_DCM0", &["CMT_PLL_CLK_TO_DCM0"])
                    .extra_wire("CLK_TO_DCM1", &["CMT_PLL_CLK_TO_DCM1"]),
            );
            let mut bel = builder.bel_virtual("CMT");
            for i in 0..10 {
                bel = bel
                    .extra_wire(format!("GIOB{i}"), &[format!("CMT_GIOB{i}")])
                    .extra_wire(
                        format!("HCLK{i}"),
                        &[
                            format!("CMT_BUFG{i}"),
                            format!("CMT_BUFG{i}_BOT"),
                            format!("CMT_BUFG{i}_TOP"),
                        ],
                    );
                if i < 5 {
                    continue;
                }
                if i == 5 && tkn == "CMT_BOT" {
                    continue;
                }
                if i == 6 && tkn == "CMT_TOP" {
                    continue;
                }
                bel = bel.extra_wire(
                    format!("HCLK{i}_TO_CLKIN2"),
                    &[format!("CMT_BUFG{i}_TO_CLKIN2")],
                );
            }
            for i in 0..28 {
                if i == 10 {
                    bel = bel.extra_int_out(format!("OUT{i}"), &[format!("CMT_CLK_{i:02}")]);
                } else {
                    bel = bel.extra_wire(format!("OUT{i}"), &[format!("CMT_CLK_{i:02}")]);
                }
                if !(10..18).contains(&i) {
                    bel = bel.extra_wire(format!("OUT{i}_TEST"), &[format!("CMT_CLK_{i:02}_TEST")]);
                }
            }
            bels.push(bel);
            let mut xn = builder.xnode("CMT", tkn, xy).num_tiles(10);
            for i in 0..10 {
                xn = xn.ref_int(xy.delta(-3, i as i32), i);
                xn = xn.ref_single(xy.delta(-2, i as i32), i, intf);
            }
            for bel in bels {
                xn = xn.bel(bel);
            }
            xn.extract();
        }
    }

    for tkn in ["CLK_HROW", "CLK_HROW_MGT"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual("CLK_HROW");
            for i in 0..32 {
                bel = bel.extra_wire(format!("GCLK{i}"), &[format!("CLK_HROW_GCLK_BUF{i}")]);
            }
            for i in 0..10 {
                bel = bel.extra_wire(format!("HCLK_L{i}"), &[format!("CLK_HROW_HCLKL_P{i}")]);
                bel = bel.extra_wire(format!("HCLK_R{i}"), &[format!("CLK_HROW_HCLKR_P{i}")]);
            }
            for i in 0..5 {
                bel = bel
                    .extra_wire_force(format!("MGT_I_L{i}"), format!("CLK_HROW_MGT_CLK_P{i}_LEFT"))
                    .extra_wire_force(format!("MGT_I_R{i}"), format!("CLK_HROW_MGT_CLK_P{i}"))
                    .extra_wire_force(format!("MGT_O_L{i}"), format!("CLK_HROW_MGT_CLKV{i}_LEFT"))
                    .extra_wire_force(format!("MGT_O_R{i}"), format!("CLK_HROW_MGT_CLKV{i}"));
            }
            builder
                .xnode("CLK_HROW", "CLK_HROW", xy)
                .num_tiles(0)
                .bel(bel)
                .extract();
        }
    }

    for tkn in ["HCLK", "HCLK_GT3", "HCLK_GTX", "HCLK_GTX_LEFT"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual("HCLK");
            for i in 0..10 {
                bel = bel
                    .extra_wire(format!("HCLK_I{i}"), &[format!("HCLK_G_HCLK_P{i}")])
                    .extra_int_out(format!("HCLK_O{i}"), &[format!("HCLK_LEAF_GCLK{i}")]);
            }
            for i in 0..4 {
                bel = bel
                    .extra_wire(format!("RCLK_I{i}"), &[format!("HCLK_RCLK{i}")])
                    .extra_int_out(format!("RCLK_O{i}"), &[format!("HCLK_LEAF_RCLK{i}")]);
            }
            let bel_gsig = builder.bel_xy("GLOBALSIG", "GLOBALSIG", 0, 0);
            builder
                .xnode("HCLK", "HCLK", xy)
                .ref_int(xy.delta(0, 1), 0)
                .bel(bel_gsig)
                .bel(bel)
                .extract();
        }
    }

    for (tkn, kind, has_bufr, has_io_s, has_io_n, has_rclk) in [
        ("HCLK_IOI", "HCLK_IOI", true, true, true, true),
        (
            "HCLK_IOI_CENTER",
            "HCLK_IOI_CENTER",
            false,
            true,
            true,
            true,
        ),
        ("HCLK_CMT_IOI", "HCLK_CMT_IOI", false, true, false, true),
        (
            "HCLK_IOI_BOTCEN",
            "HCLK_IOI_BOTCEN",
            false,
            true,
            false,
            false,
        ),
        (
            "HCLK_IOI_BOTCEN_MGT",
            "HCLK_IOI_BOTCEN",
            false,
            true,
            false,
            false,
        ),
        ("HCLK_IOI_CMT", "HCLK_IOI_CMT", false, false, true, false),
        (
            "HCLK_IOI_CMT_MGT",
            "HCLK_IOI_CMT",
            false,
            false,
            true,
            false,
        ),
        (
            "HCLK_IOI_TOPCEN",
            "HCLK_IOI_TOPCEN",
            false,
            false,
            true,
            false,
        ),
        (
            "HCLK_IOI_TOPCEN_MGT",
            "HCLK_IOI_TOPCEN",
            false,
            false,
            true,
            false,
        ),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let int_x = builder
                .walk_to_int(xy.delta(0, -1), Dir::W, false)
                .unwrap()
                .x;
            let mut bels = vec![];
            if has_io_n {
                for i in 0..2 {
                    bels.push(
                        builder
                            .bel_xy(
                                format!("BUFIO{i}"),
                                "BUFIO",
                                0,
                                if has_io_s { i ^ 2 } else { i },
                            )
                            .pin_name_only("I", 1)
                            .pins_name_only(&["O"]),
                    )
                }
            }
            if has_io_s {
                for i in 2..4 {
                    bels.push(
                        builder
                            .bel_xy(format!("BUFIO{i}"), "BUFIO", 0, i ^ 3)
                            .pin_name_only("I", 1)
                            .pins_name_only(&["O"]),
                    )
                }
            }
            if has_bufr {
                for i in 0..2 {
                    bels.push(
                        builder
                            .bel_xy(format!("BUFR{i}"), "BUFR", 0, i)
                            .pins_name_only(&["O", "I"]),
                    )
                }
            }
            bels.push(
                builder
                    .bel_xy("IDELAYCTRL", "IDELAYCTRL", 0, 0)
                    .pins_name_only(&["REFCLK"]),
            );
            bels.push(builder.bel_xy("DCI", "DCI", 0, 0));
            let mut bel_ioclk = builder.bel_virtual("IOCLK");
            for i in 0..4 {
                bel_ioclk = bel_ioclk
                    .extra_wire(format!("RCLK_I{i}"), &[format!("HCLK_IOI_RCLK{i}")])
                    .extra_wire(format!("RCLK_O{i}"), &[format!("HCLK_IOI_RCLK_FORIO_P{i}")]);
            }
            for i in 0..4 {
                if (has_io_s && i >= 2) || (has_io_n && i < 2) {
                    bel_ioclk =
                        bel_ioclk.extra_wire(format!("IOCLK{i}"), &[format!("HCLK_IOI_IOCLKP{i}")]);
                }
            }
            for i in 0..10 {
                bel_ioclk = bel_ioclk
                    .extra_wire(format!("HCLK_I{i}"), &[format!("HCLK_IOI_G_HCLK_P{i}")])
                    .extra_wire(format!("HCLK_O{i}"), &[format!("HCLK_IOI_LEAF_GCLK_P{i}")]);
            }
            if has_rclk {
                bel_ioclk = bel_ioclk
                    .extra_wire("VRCLK0", &["HCLK_IOI_VRCLK0"])
                    .extra_wire("VRCLK1", &["HCLK_IOI_VRCLK1"])
                    .extra_wire("VRCLK_S0", &["HCLK_IOI_VRCLK_S0"])
                    .extra_wire("VRCLK_S1", &["HCLK_IOI_VRCLK_S1"])
                    .extra_wire("VRCLK_N0", &["HCLK_IOI_VRCLK_N0"])
                    .extra_wire("VRCLK_N1", &["HCLK_IOI_VRCLK_N1"]);
            }
            bels.push(bel_ioclk);
            if has_bufr {
                bels.push(
                    builder
                        .bel_virtual("RCLK")
                        .extra_wire("MGT0", &["HCLK_IOI_MGT_CLK_P0"])
                        .extra_wire("MGT1", &["HCLK_IOI_MGT_CLK_P1"])
                        .extra_wire("MGT2", &["HCLK_IOI_MGT_CLK_P2"])
                        .extra_wire("MGT3", &["HCLK_IOI_MGT_CLK_P3"])
                        .extra_wire("MGT4", &["HCLK_IOI_MGT_CLK_P4"])
                        .extra_int_in("CKINT0", &["HCLK_IOI_INT_RCLKMUX_B_N"])
                        .extra_int_in("CKINT1", &["HCLK_IOI_INT_RCLKMUX_B_S"]),
                );
            }

            let mut xn = builder.xnode(kind, kind, xy);
            let mut t = 0;
            if has_io_s {
                xn = xn
                    .raw_tile(xy.delta(0, -2))
                    .ref_int(
                        Coord {
                            x: int_x,
                            y: xy.y - 2,
                        },
                        0,
                    )
                    .ref_single(
                        Coord {
                            x: int_x + 1,
                            y: xy.y - 2,
                        },
                        0,
                        intf,
                    )
                    .raw_tile(xy.delta(0, -1))
                    .ref_int(
                        Coord {
                            x: int_x,
                            y: xy.y - 1,
                        },
                        1,
                    )
                    .ref_single(
                        Coord {
                            x: int_x + 1,
                            y: xy.y - 1,
                        },
                        1,
                        intf,
                    );
                t = 2;
            }
            if has_io_n {
                xn = xn
                    .raw_tile(xy.delta(0, 1))
                    .ref_int(
                        Coord {
                            x: int_x,
                            y: xy.y + 1,
                        },
                        t,
                    )
                    .ref_single(
                        Coord {
                            x: int_x + 1,
                            y: xy.y + 1,
                        },
                        t,
                        intf,
                    );
                if !has_io_s || has_bufr {
                    xn = xn
                        .raw_tile(xy.delta(0, 2))
                        .ref_int(
                            Coord {
                                x: int_x,
                                y: xy.y + 2,
                            },
                            t + 1,
                        )
                        .ref_single(
                            Coord {
                                x: int_x + 1,
                                y: xy.y + 2,
                            },
                            t + 1,
                            intf,
                        );
                }
            }
            for bel in bels {
                xn = xn.bel(bel);
            }
            xn.extract();
        }
    }

    for tkn in [
        "HCLK_IOB_CMT_BOT",
        "HCLK_IOB_CMT_BOT_MGT",
        "HCLK_IOB_CMT_MID",
        "HCLK_IOB_CMT_MID_MGT",
        "HCLK_IOB_CMT_TOP",
        "HCLK_IOB_CMT_TOP_MGT",
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel_hclk = builder.bel_virtual("HCLK_CMT_HCLK");
            for i in 0..10 {
                bel_hclk = bel_hclk
                    .extra_wire(format!("HCLK_I{i}"), &[format!("HCLK_IOB_CMT_GCLK_B{i}")])
                    .extra_wire(format!("HCLK_O{i}"), &[format!("HCLK_IOB_CMT_BUFG{i}")]);
            }
            let mut bel_giob = builder.bel_virtual("HCLK_CMT_GIOB").raw_tile(1);
            for i in 0..10 {
                bel_giob = bel_giob
                    .extra_wire(format!("GIOB_I{i}"), &[format!("CLK_HROW_CLK_METAL9_{i}")])
                    .extra_wire(
                        format!("GIOB_O{i}"),
                        &[format!("CLK_HROW_CLK_H_METAL9_{i}")],
                    );
            }
            builder
                .xnode("HCLK_CMT", "HCLK_CMT", xy)
                .num_tiles(0)
                .raw_tile(xy.delta(1, 0))
                .bel(bel_hclk)
                .bel(bel_giob)
                .extract();
        }
    }

    for tkn in ["CLK_IOB_B", "CLK_IOB_T"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual("CLK_IOB");
            for i in 0..10 {
                bel = bel.extra_wire(format!("PAD{i}"), &[format!("CLK_IOB_PAD_CLK{i}")]);
                bel = bel.extra_wire(format!("PAD_BUF{i}"), &[format!("CLK_IOB_CLK_BUF{i}")]);
                bel = bel.extra_wire(format!("GIOB{i}"), &[format!("CLK_IOB_IOB_CLKP{i}")]);
            }
            for i in 0..5 {
                bel = bel.extra_wire(
                    format!("MGT_L{i}"),
                    &[
                        format!("CLK_IOB_B_CLK_BUF{ii}", ii = 15 + i),
                        format!("CLK_IOB_T_CLK_BUF{ii}", ii = 15 + i),
                    ],
                );
                bel = bel.extra_wire(
                    format!("MGT_R{i}"),
                    &[
                        format!("CLK_IOB_B_CLK_BUF{ii}", ii = 14 - i),
                        format!("CLK_IOB_T_CLK_BUF{ii}", ii = 14 - i),
                    ],
                );
            }
            for i in 0..32 {
                bel = bel.extra_wire(format!("MUXBUS_I{i}"), &[format!("CLK_IOB_MUXED_CLKIN{i}")]);
                bel = bel.extra_wire(
                    format!("MUXBUS_O{i}"),
                    &[format!("CLK_IOB_MUXED_CLKOUT{i}")],
                );
            }
            builder.xnode(tkn, tkn, xy).num_tiles(0).bel(bel).extract();
        }
    }

    for (tkn, node) in [
        ("CLK_CMT_BOT", "CLK_CMT_B"),
        ("CLK_CMT_BOT_MGT", "CLK_CMT_B"),
        ("CLK_CMT_TOP", "CLK_CMT_T"),
        ("CLK_CMT_TOP_MGT", "CLK_CMT_T"),
    ] {
        let bt = if node == "CLK_CMT_B" { 'B' } else { 'T' };
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual("CLK_CMT");
            for i in 0..28 {
                bel = bel.extra_wire(format!("CMT_CLK{i}"), &[format!("CLK_CMT_CMT_CLK_{i:02}")]);
            }
            for i in 0..5 {
                bel = bel.extra_wire_force(
                    format!("MGT_L{i}"),
                    format!("CLK_IOB_{bt}_CLK_BUF{ii}", ii = 15 + i),
                );
                bel = bel.extra_wire(
                    format!("MGT_R{i}"),
                    &[
                        format!("CLK_IOB_B_CLK_BUF{ii}", ii = 14 - i),
                        format!("CLK_IOB_T_CLK_BUF{ii}", ii = 14 - i),
                    ],
                );
            }
            for i in 0..32 {
                bel = bel.extra_wire(format!("MUXBUS_I{i}"), &[format!("CLK_IOB_MUXED_CLKIN{i}")]);
                bel = bel.extra_wire(
                    format!("MUXBUS_O{i}"),
                    &[format!("CLK_IOB_MUXED_CLKOUT{i}")],
                );
            }
            builder
                .xnode(node, node, xy)
                .num_tiles(0)
                .bel(bel)
                .extract();
        }
    }

    for (tkn, node) in [
        ("CLK_MGT_BOT", "CLK_MGT_B"),
        ("CLK_MGT_BOT_MGT", "CLK_MGT_B"),
        ("CLK_MGT_TOP", "CLK_MGT_T"),
        ("CLK_MGT_TOP_MGT", "CLK_MGT_T"),
    ] {
        let bt = if node == "CLK_MGT_B" { 'B' } else { 'T' };
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual("CLK_MGT");
            for i in 0..5 {
                bel =
                    bel.extra_wire_force(format!("MGT_L{i}"), format!("CLK_MGT_{bt}_CLK{i}_LEFT"));
                bel = bel.extra_wire(
                    format!("MGT_R{i}"),
                    &[format!("CLK_MGT_B_CLK{i}"), format!("CLK_MGT_T_CLK{i}")],
                );
            }
            for i in 0..32 {
                bel = bel.extra_wire(format!("MUXBUS_I{i}"), &[format!("CLK_IOB_MUXED_CLKIN{i}")]);
                bel = bel.extra_wire(
                    format!("MUXBUS_O{i}"),
                    &[format!("CLK_IOB_MUXED_CLKOUT{i}")],
                );
            }
            builder
                .xnode(node, node, xy)
                .num_tiles(0)
                .bel(bel)
                .extract();
        }
    }

    for tkn in ["HCLK_BRAM_MGT", "HCLK_BRAM_MGT_LEFT"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual("HCLK_BRAM_MGT");
            for i in 0..5 {
                bel = bel
                    .extra_wire(format!("MGT_I{i}"), &[format!("HCLK_BRAM_MGT_CLK_IN_P{i}")])
                    .extra_wire(
                        format!("MGT_O{i}"),
                        &[format!("HCLK_BRAM_MGT_CLK_OUT_P{i}")],
                    );
            }
            builder
                .xnode("HCLK_BRAM_MGT", "HCLK_BRAM_MGT", xy)
                .num_tiles(0)
                .bel(bel)
                .extract();
        }
    }

    for (tkn, kind) in [("GT3", "GTP"), ("GTX", "GTX"), ("GTX_LEFT", "GTX")] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let int_dx = if tkn == "GTX_LEFT" { 2 } else { -3 };
            let intf_gt = builder.db.get_node_naming(if tkn == "GTX_LEFT" {
                "INTF.GTX_LEFT"
            } else {
                "INTF.GTP"
            });
            let gtkind = if kind == "GTP" {
                "GTP_DUAL"
            } else {
                "GTX_DUAL"
            };
            let bels = [
                builder
                    .bel_xy(gtkind, gtkind, 0, 0)
                    .pins_name_only(&[
                        "RXP0", "RXN0", "RXP1", "RXN1", "TXP0", "TXN0", "TXP1", "TXN1", "CLKIN",
                    ])
                    .extra_wire("CLKOUT_NORTH_S", &["GT3_CLKOUT_NORTH"])
                    .extra_wire("CLKOUT_NORTH", &["GT3_CLKOUT_NORTH_N"])
                    .extra_wire("CLKOUT_SOUTH", &["GT3_CLKOUT_SOUTH"])
                    .extra_wire("CLKOUT_SOUTH_N", &["GT3_CLKOUT_SOUTH_N"])
                    .extra_wire("MGT0", &["GT3_MGT_CLK_P0"])
                    .extra_wire("MGT1", &["GT3_MGT_CLK_P1"])
                    .extra_wire("MGT2", &["GT3_MGT_CLK_P2"])
                    .extra_wire("MGT3", &["GT3_MGT_CLK_P3"])
                    .extra_wire("MGT4", &["GT3_MGT_CLK_P4"])
                    .extra_int_in(
                        "GREFCLK",
                        &["GT3_GREFCLK", "GTX_GREFCLK", "GTX_LEFT_GREFCLK"],
                    ),
                builder
                    .bel_xy("BUFDS", "BUFDS", 0, 0)
                    .pins_name_only(&["IP", "IN", "O"]),
                builder.bel_xy("CRC64_0", "CRC64", 0, 0),
                builder.bel_xy("CRC64_1", "CRC64", 0, 1),
                builder.bel_xy("CRC32_0", "CRC32", 0, 0),
                builder.bel_xy("CRC32_1", "CRC32", 0, 1),
                builder.bel_xy("CRC32_2", "CRC32", 0, 2),
                builder.bel_xy("CRC32_3", "CRC32", 0, 3),
                builder
                    .bel_xy("IPAD.RXP0", "IPAD", 0, 1)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy("IPAD.RXN0", "IPAD", 0, 0)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy("IPAD.RXP1", "IPAD", 0, 3)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy("IPAD.RXN1", "IPAD", 0, 2)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy("IPAD.CLKP", "IPAD", 0, 5)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy("IPAD.CLKN", "IPAD", 0, 4)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy("OPAD.TXP0", "OPAD", 0, 1)
                    .pins_name_only(&["I"]),
                builder
                    .bel_xy("OPAD.TXN0", "OPAD", 0, 0)
                    .pins_name_only(&["I"]),
                builder
                    .bel_xy("OPAD.TXP1", "OPAD", 0, 3)
                    .pins_name_only(&["I"]),
                builder
                    .bel_xy("OPAD.TXN1", "OPAD", 0, 2)
                    .pins_name_only(&["I"]),
            ];

            let mut xn = builder.xnode(kind, tkn, xy).num_tiles(20);
            for i in 0..10 {
                xn = xn.ref_int(xy.delta(int_dx, -10 + i as i32), i).ref_single(
                    xy.delta(int_dx + 1, -10 + i as i32),
                    i,
                    intf_gt,
                );
            }
            for i in 0..10 {
                xn = xn
                    .ref_int(xy.delta(int_dx, 1 + i as i32), i + 10)
                    .ref_single(xy.delta(int_dx + 1, 1 + i as i32), i + 10, intf_gt);
            }
            for bel in bels {
                xn = xn.bel(bel);
            }
            xn.extract();
        }
    }

    builder.build()
}
