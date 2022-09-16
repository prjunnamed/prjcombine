use prjcombine_int::db::{Dir, IntDb, WireKind};
use prjcombine_rawdump::{Coord, Part};

use prjcombine_rdintb::IntBuilder;

pub fn make_int_db(rd: &Part) -> IntDb {
    let mut builder = IntBuilder::new("ultrascale", rd);

    builder.wire("VCC", WireKind::Tie1, &["VCC_WIRE"]);
    builder.wire("GND", WireKind::Tie1, &["GND_WIRE"]);

    for i in 0..16 {
        builder.wire(
            format!("GCLK{i}"),
            WireKind::ClkOut(i),
            &[format!("GCLK_B_0_{i}")],
        );
    }

    for (iq, q) in ["NE", "NW", "SE", "SW"].into_iter().enumerate() {
        for (ih, h) in ["E", "W"].into_iter().enumerate() {
            for i in 0..16 {
                match (iq, i) {
                    (1 | 3, 0) => {
                        let w = builder.mux_out(
                            format!("SDND.{q}.{h}.{i}"),
                            &[format!("SDND{q}_{h}_{i}_FTS")],
                        );
                        builder.branch(
                            w,
                            Dir::S,
                            format!("SDND.{q}.{h}.{i}.S"),
                            &[format!("SDND{q}_{h}_BLS_{i}_FTN")],
                        );
                    }
                    (1 | 3, 15) => {
                        let w = builder.mux_out(
                            format!("SDND.{q}.{h}.{i}"),
                            &[format!("SDND{q}_{h}_{i}_FTN")],
                        );
                        builder.branch(
                            w,
                            Dir::N,
                            format!("SDND.{q}.{h}.{i}.N"),
                            &[format!("SDND{q}_{h}_BLN_{i}_FTS")],
                        );
                    }
                    _ => {
                        let xlat = [0, 7, 8, 9, 10, 11, 12, 13, 14, 15, 1, 2, 3, 4, 5, 6];
                        builder.mux_out(
                            format!("SDND.{q}.{h}.{i}"),
                            &[format!(
                                "INT_NODE_SINGLE_DOUBLE_{n}_INT_OUT",
                                n = iq * 32 + ih * 16 + xlat[i]
                            )],
                        );
                    }
                }
            }
        }
    }
    // Singles.
    for i in 0..8 {
        let beg = builder.mux_out(format!("SNG.E.E.{i}.0"), &[format!("EE1_E_BEG{i}")]);
        let end = builder.branch(
            beg,
            Dir::E,
            format!("SNG.E.E.{i}.1"),
            &[format!("EE1_E_END{i}")],
        );
        if i == 0 {
            builder.branch(
                end,
                Dir::S,
                format!("SNG.E.E.{i}.1.S"),
                &[format!("EE1_E_BLS_{i}_FTN")],
            );
        }
    }
    for i in 0..8 {
        if i == 0 {
            let beg = builder.mux_out(format!("SNG.E.W.{i}.0"), &[format!("EE1_W_{i}_FTS")]);
            builder.branch(
                beg,
                Dir::S,
                format!("SNG.E.W.{i}.0.S"),
                &[format!("EE1_W_BLS_{i}_FTN")],
            );
        } else {
            builder.mux_out(
                format!("SNG.E.W.{i}.0"),
                &[format!("INT_INT_SINGLE_{n}_INT_OUT", n = i + 8)],
            );
        }
    }
    for i in 0..8 {
        builder.mux_out(
            format!("SNG.W.E.{i}.0"),
            &[format!("INT_INT_SINGLE_{n}_INT_OUT", n = i + 48)],
        );
    }
    for i in 0..8 {
        let beg = builder.mux_out(format!("SNG.W.W.{i}.0"), &[format!("WW1_W_BEG{i}")]);
        builder.branch(
            beg,
            Dir::W,
            format!("SNG.W.W.{i}.1"),
            &[format!("WW1_W_END{i}")],
        );
    }
    for dir in [Dir::N, Dir::S] {
        for ew in ["E", "W"] {
            for i in 0..8 {
                let beg = builder.mux_out(
                    format!("SNG.{dir}.{ew}.{i}.0"),
                    &[format!("{dir}{dir}1_{ew}_BEG{i}")],
                );
                let end = builder.branch(
                    beg,
                    dir,
                    format!("SNG.{dir}.{ew}.{i}.1"),
                    &[format!("{dir}{dir}1_{ew}_END{i}")],
                );
                if i == 0 && dir == Dir::S {
                    builder.branch(
                        end,
                        Dir::S,
                        format!("SNG.{dir}.{ew}.{i}.1.S"),
                        &[format!("{dir}{dir}1_{ew}_BLS_{i}_FTN")],
                    );
                }
            }
        }
    }
    // Doubles.
    for dir in [Dir::E, Dir::W] {
        for ew in ["E", "W"] {
            for i in 0..8 {
                let beg = builder.mux_out(
                    format!("DBL.{dir}.{ew}.{i}.0"),
                    &[format!("{dir}{dir}2_{ew}_BEG{i}")],
                );
                let end = builder.branch(
                    beg,
                    dir,
                    format!("DBL.{dir}.{ew}.{i}.1"),
                    &[format!("{dir}{dir}2_{ew}_END{i}")],
                );
                if i == 7 && dir == Dir::E {
                    builder.branch(
                        end,
                        Dir::N,
                        format!("DBL.{dir}.{ew}.{i}.1.N"),
                        &[format!("{dir}{dir}2_{ew}_BLN_{i}_FTS")],
                    );
                }
            }
        }
    }
    for dir in [Dir::N, Dir::S] {
        let ftd = !dir;
        for ew in ["E", "W"] {
            for i in 0..8 {
                let beg = builder.mux_out(
                    format!("DBL.{dir}.{ew}.{i}.0"),
                    &[format!("{dir}{dir}2_{ew}_BEG{i}")],
                );
                let a = builder.branch(
                    beg,
                    dir,
                    format!("DBL.{dir}.{ew}.{i}.1"),
                    &[format!("{dir}{dir}2_{ew}_A_FT{ftd}{i}")],
                );
                let end = builder.branch(
                    a,
                    dir,
                    format!("DBL.{dir}.{ew}.{i}.2"),
                    &[format!("{dir}{dir}2_{ew}_END{i}")],
                );
                if i == 7 && dir == Dir::N {
                    builder.branch(
                        end,
                        Dir::N,
                        format!("DBL.{dir}.{ew}.{i}.2.N"),
                        &[format!("{dir}{dir}2_{ew}_BLN_{i}_FTS")],
                    );
                }
            }
        }
    }

    for (iq, q) in ["NE", "NW", "SE", "SW"].into_iter().enumerate() {
        for (ih, h) in ['E', 'W'].into_iter().enumerate() {
            for i in 0..16 {
                match (q, h, i) {
                    ("NW", 'E', 0) | ("SW", 'E', 0) | ("NW", 'W', 0) | ("NW", 'W', 1) => {
                        let w = builder.mux_out(
                            format!("QLND.{q}.{h}.{i}"),
                            &[format!("QLND{q}_{h}_{i}_FTS")],
                        );
                        builder.branch(
                            w,
                            Dir::S,
                            format!("QLND.{q}.{h}.{i}.S"),
                            &[format!("QLND{q}_{h}_BLS_{i}_FTN")],
                        );
                    }
                    ("NW", 'E', 15) | ("SW", 'E', 15) | ("SE", 'W', 15) => {
                        let w = builder.mux_out(
                            format!("QLND.{q}.{h}.{i}"),
                            &[format!("QLND{q}_{h}_{i}_FTN")],
                        );
                        builder.branch(
                            w,
                            Dir::N,
                            format!("QLND.{q}.{h}.{i}.N"),
                            &[format!("QLND{q}_{h}_BLN_{i}_FTS")],
                        );
                    }
                    _ => {
                        let xlat = [0, 7, 8, 9, 10, 11, 12, 13, 14, 15, 1, 2, 3, 4, 5, 6];
                        builder.mux_out(
                            format!("QLND.{q}.{h}.{i}"),
                            &[format!(
                                "INT_NODE_QUAD_LONG_{n}_INT_OUT",
                                n = iq * 32 + ih * 16 + xlat[i]
                            )],
                        );
                    }
                }
            }
        }
    }
    for (dir, name, l, n, fts, ftn) in [
        (Dir::E, "QUAD", 2, 16, true, true),
        (Dir::W, "QUAD", 2, 16, false, false),
        (Dir::N, "QUAD.4", 4, 8, false, false),
        (Dir::N, "QUAD.5", 5, 8, false, true),
        (Dir::S, "QUAD.4", 4, 8, false, false),
        (Dir::S, "QUAD.5", 5, 8, false, false),
        (Dir::E, "LONG", 6, 8, true, false),
        (Dir::W, "LONG", 6, 8, false, true),
        (Dir::N, "LONG.12", 12, 4, false, false),
        (Dir::N, "LONG.16", 16, 4, false, true),
        (Dir::S, "LONG.12", 12, 4, true, false),
        (Dir::S, "LONG.16", 16, 4, false, false),
    ] {
        let ftd = !dir;
        let ll = if matches!(dir, Dir::E | Dir::W) {
            l * 2
        } else {
            l
        };
        for i in 0..n {
            let mut w = builder.mux_out(
                format!("{name}.{dir}.{i}.0"),
                &[format!("{dir}{dir}{ll}_BEG{i}")],
            );
            for j in 1..l {
                let nn = (b'A' + (j - 1)) as char;
                w = builder.branch(
                    w,
                    dir,
                    format!("{name}.{dir}.{i}.{j}"),
                    &[format!("{dir}{dir}{ll}_{nn}_FT{ftd}{i}")],
                );
            }
            w = builder.branch(
                w,
                dir,
                format!("{name}.{dir}.{i}.{l}"),
                &[format!("{dir}{dir}{ll}_END{i}")],
            );
            if i == 0 && fts {
                builder.branch(
                    w,
                    Dir::S,
                    format!("{name}.{dir}.{i}.{l}.S"),
                    &[format!("{dir}{dir}{ll}_BLS_{i}_FTN")],
                );
            }
            if i == (n - 1) && ftn {
                builder.branch(
                    w,
                    Dir::N,
                    format!("{name}.{dir}.{i}.{l}.N"),
                    &[format!("{dir}{dir}{ll}_BLN_{i}_FTS")],
                );
            }
        }
    }

    for i in 0..16 {
        for j in 0..2 {
            builder.mux_out(
                format!("INT_NODE_GLOBAL.{i}.{j}"),
                &[format!("INT_NODE_GLOBAL_{i}_OUT{j}")],
            );
        }
    }
    for i in 0..8 {
        builder.mux_out(format!("IMUX.E.CTRL.{i}"), &[format!("CTRL_E_B{i}")]);
    }
    for i in 0..10 {
        builder.mux_out(format!("IMUX.W.CTRL.{i}"), &[format!("CTRL_W_B{i}")]);
    }

    for (iq, q) in ["1", "2"].into_iter().enumerate() {
        for (ih, h) in ['E', 'W'].into_iter().enumerate() {
            for i in 0..32 {
                match i {
                    1 | 3 => {
                        let w = builder.mux_out(
                            format!("INODE.{q}.{h}.{i}"),
                            &[format!("INODE_{q}_{h}_{i}_FTS")],
                        );
                        builder.branch(
                            w,
                            Dir::S,
                            format!("INODE.{q}.{h}.{i}.S"),
                            &[format!("INODE_{q}_{h}_BLS_{i}_FTN")],
                        );
                    }
                    28 | 30 => {
                        let w = builder.mux_out(
                            format!("INODE.{q}.{h}.{i}"),
                            &[format!("INODE_{q}_{h}_{i}_FTN")],
                        );
                        builder.branch(
                            w,
                            Dir::N,
                            format!("INODE.{q}.{h}.{i}.N"),
                            &[format!("INODE_{q}_{h}_BLN_{i}_FTS")],
                        );
                    }
                    _ => {
                        let xlat = [
                            0, 11, 22, 25, 26, 27, 28, 29, 30, 31, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
                            12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 23, 24,
                        ];
                        let w = builder.mux_out(format!("INODE.{q}.{h}.{i}"), &[""]);
                        builder.extra_name_tile(
                            "INT",
                            format!("INT_NODE_IMUX_{n}_INT_OUT", n = iq * 64 + ih * 32 + xlat[i]),
                            w,
                        );
                    }
                }
            }
        }
    }

    for ew in ['E', 'W'] {
        for i in 0..16 {
            match i {
                1 | 3 | 5 | 7 | 11 => {
                    let w = builder.mux_out(
                        format!("IMUX.{ew}.BYP.{i}"),
                        &[format!("BOUNCE_{ew}_{i}_FTS")],
                    );
                    builder.branch(
                        w,
                        Dir::S,
                        format!("IMUX.{ew}.BYP.{i}.S"),
                        &[format!("BOUNCE_{ew}_BLS_{i}_FTN")],
                    );
                }
                8 | 10 | 12 | 14 => {
                    let w = builder.mux_out(
                        format!("IMUX.{ew}.BYP.{i}"),
                        &[format!("BOUNCE_{ew}_{i}_FTN")],
                    );
                    builder.branch(
                        w,
                        Dir::N,
                        format!("IMUX.{ew}.BYP.{i}.N"),
                        &[format!("BOUNCE_{ew}_BLN_{i}_FTS")],
                    );
                }
                _ => {
                    builder.mux_out(format!("IMUX.{ew}.BYP.{i}"), &[format!("BYPASS_{ew}{i}")]);
                }
            }
        }
    }
    for ew in ['E', 'W'] {
        for i in 0..48 {
            builder.mux_out(format!("IMUX.{ew}.IMUX.{i}"), &[format!("IMUX_{ew}{i}")]);
        }
    }

    for ew in ['E', 'W'] {
        for i in 0..32 {
            builder.logic_out(format!("OUT.{ew}.{i}"), &[format!("LOGIC_OUTS_{ew}{i}")]);
        }
    }

    for ew in ['E', 'W'] {
        for i in 0..4 {
            let w = builder.test_out(format!("TEST.{ew}.{i}"), &[""]);
            let tiles: &[&str] = if ew == 'W' {
                &[
                    "INT_INTERFACE_L",
                    "INT_INT_INTERFACE_XIPHY_FT",
                    "INT_INTERFACE_PCIE_L",
                    "INT_INT_INTERFACE_GT_LEFT_FT",
                ]
            } else {
                &[
                    "INT_INTERFACE_R",
                    "INT_INTERFACE_PCIE_R",
                    "INT_INTERFACE_GT_R",
                ]
            };
            for &t in tiles {
                builder.extra_name_tile(t, format!("BLOCK_OUTS{i}"), w);
            }
        }
    }

    for i in 0..16 {
        builder.mux_out(
            format!("RCLK.IMUX.CE.{i}"),
            &[format!("CLK_BUFCE_LEAF_X16_0_CE_INT{i}")],
        );
    }
    for i in 0..2 {
        for j in 0..4 {
            builder.mux_out(
                format!("RCLK.IMUX.LEFT.{i}.{j}"),
                &[format!("INT_RCLK_TO_CLK_LEFT_{i}_{j}")],
            );
        }
    }
    for i in 0..2 {
        for j in 0..4 {
            builder.mux_out(
                format!("RCLK.IMUX.RIGHT.{i}.{j}"),
                &[format!("INT_RCLK_TO_CLK_RIGHT_{i}_{j}")],
            );
        }
    }
    for i in 0..48 {
        let w = builder.mux_out(format!("RCLK.INODE.{i}"), &[""]);
        builder.extra_name_tile("RCLK_INT_L", format!("INT_NODE_IMUX_{i}_INT_OUT"), w);
        builder.extra_name_tile("RCLK_INT_R", format!("INT_NODE_IMUX_{i}_INT_OUT"), w);
    }

    builder.extract_main_passes();

    builder.node_type("INT", "INT", "INT");

    builder.extract_term_conn("TERM.W", Dir::W, "INT_TERM_L_IO", &[]);
    builder.extract_term_conn("TERM.W", Dir::W, "INT_INT_INTERFACE_GT_LEFT_FT", &[]);
    builder.extract_term_conn("TERM.E", Dir::E, "INT_INTERFACE_GT_R", &[]);
    builder.extract_term_conn("TERM.S", Dir::S, "INT_TERM_B", &[]);
    builder.extract_term_conn("TERM.N", Dir::N, "INT_TERM_T", &[]);

    for (dir, tkn) in [(Dir::W, "INT_INTERFACE_L"), (Dir::E, "INT_INTERFACE_R")] {
        builder.extract_intf(format!("INTF.{dir}"), dir, tkn, format!("INTF.{dir}"), true);
    }

    for (dir, n, tkn) in [
        (Dir::W, "IO", "INT_INT_INTERFACE_XIPHY_FT"),
        (Dir::W, "PCIE", "INT_INTERFACE_PCIE_L"),
        (Dir::E, "PCIE", "INT_INTERFACE_PCIE_R"),
        (Dir::W, "GT", "INT_INT_INTERFACE_GT_LEFT_FT"),
        (Dir::E, "GT", "INT_INTERFACE_GT_R"),
    ] {
        builder.extract_intf(
            format!("INTF.{dir}.DELAY"),
            dir,
            tkn,
            format!("INTF.{dir}.{n}"),
            true,
        );
    }

    for tkn in ["RCLK_INT_L", "RCLK_INT_R"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = Coord {
                x: xy.x,
                y: xy.y + 1,
            };
            builder.extract_xnode("RCLK", xy, &[], &[int_xy], "RCLK", &[], &[]);
        }
    }

    for (tkn, kind, key) in [
        ("CLEL_L", "CLEL_L", "SLICE_L"),
        ("CLEL_R", "CLEL_R", "SLICE_R"),
        ("CLE_M", "CLEM", "SLICE_L"),
        ("CLE_M_R", "CLEM", "SLICE_L"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let int_xy = if key == "SLICE_L" {
                Coord {
                    x: xy.x + 1,
                    y: xy.y,
                }
            } else {
                Coord {
                    x: xy.x - 1,
                    y: xy.y,
                }
            };
            builder.extract_xnode_bels(
                kind,
                xy,
                &[],
                &[int_xy],
                kind,
                &[builder
                    .bel_xy(key, "SLICE", 0, 0)
                    .pin_name_only("CIN", 1)
                    .pin_name_only("COUT", 0)],
            );
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("BRAM").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let n = builder.db.get_node_naming("INTF.W");
        for dy in 0..5 {
            int_xy.push(Coord {
                x: xy.x + 2,
                y: xy.y + dy,
            });
            intf_xy.push((
                Coord {
                    x: xy.x + 1,
                    y: xy.y + dy,
                },
                n,
            ));
        }
        let mut bel_bram_f = builder
            .bel_xy("BRAM_F", "RAMB36", 0, 0)
            .pin_name_only("CASINSBITERR", 1)
            .pin_name_only("CASINDBITERR", 1)
            .pin_name_only("CASOUTSBITERR", 0)
            .pin_name_only("CASOUTDBITERR", 0)
            .pin_name_only("CASPRVEMPTY", 1)
            .pin_name_only("CASPRVRDEN", 1)
            .pin_name_only("CASNXTEMPTY", 1)
            .pin_name_only("CASNXTRDEN", 1)
            .pin_name_only("CASMBIST12OUT", 0)
            .pin_name_only("ENABLE_BIST", 1)
            .pin_name_only("START_RSR_NEXT", 0);
        let mut bel_bram_h0 = builder
            .bel_xy("BRAM_H0", "RAMB18", 0, 0)
            .pin_name_only("CASPRVEMPTY", 0)
            .pin_name_only("CASPRVRDEN", 0)
            .pin_name_only("CASNXTEMPTY", 0)
            .pin_name_only("CASNXTRDEN", 0);
        let mut bel_bram_h1 = builder.bel_xy("BRAM_H1", "RAMB18", 0, 1);
        for ab in ['A', 'B'] {
            for ul in ['U', 'L'] {
                for i in 0..16 {
                    bel_bram_f = bel_bram_f.pin_name_only(&format!("CASDI{ab}{ul}{i}"), 1);
                    bel_bram_f = bel_bram_f.pin_name_only(&format!("CASDO{ab}{ul}{i}"), 1);
                }
                for i in 0..2 {
                    bel_bram_f = bel_bram_f.pin_name_only(&format!("CASDIP{ab}{ul}{i}"), 1);
                    bel_bram_f = bel_bram_f.pin_name_only(&format!("CASDOP{ab}{ul}{i}"), 1);
                }
            }
            for i in 0..16 {
                bel_bram_h0 = bel_bram_h0.pin_name_only(&format!("CASDI{ab}L{i}"), 0);
                bel_bram_h0 = bel_bram_h0.pin_name_only(&format!("CASDO{ab}L{i}"), 0);
            }
            for i in 0..2 {
                bel_bram_h0 = bel_bram_h0.pin_name_only(&format!("CASDIP{ab}L{i}"), 0);
                bel_bram_h0 = bel_bram_h0.pin_name_only(&format!("CASDOP{ab}L{i}"), 0);
            }
            for i in 0..16 {
                bel_bram_h1 = bel_bram_h1.pin_name_only(&format!("CASDI{ab}U{i}"), 0);
                bel_bram_h1 = bel_bram_h1.pin_name_only(&format!("CASDO{ab}U{i}"), 0);
            }
            for i in 0..2 {
                bel_bram_h1 = bel_bram_h1.pin_name_only(&format!("CASDIP{ab}U{i}"), 0);
                bel_bram_h1 = bel_bram_h1.pin_name_only(&format!("CASDOP{ab}U{i}"), 0);
            }
        }
        builder.extract_xnode_bels_intf(
            "BRAM",
            xy,
            &[],
            &int_xy,
            &intf_xy,
            "BRAM",
            &[bel_bram_f, bel_bram_h0, bel_bram_h1],
        );
    }

    for tkn in [
        "RCLK_BRAM_L",
        "RCLK_BRAM_R",
        "RCLK_RCLK_BRAM_L_AUXCLMP_FT",
        "RCLK_RCLK_BRAM_L_BRAMCLMP_FT",
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let n = builder.db.get_node_naming("INTF.W");
            let int_xy = Coord {
                x: xy.x + 2,
                y: xy.y + 1,
            };
            let intf_xy = (
                Coord {
                    x: xy.x + 1,
                    y: xy.y + 1,
                },
                n,
            );

            let mut bels = vec![];
            for (i, x, y) in [(0, 0, 0), (1, 0, 1), (2, 1, 0), (3, 1, 1)] {
                bels.push(builder.bel_xy(format!("HARD_SYNC{i}"), "HARD_SYNC", x, y));
            }
            builder.extract_xnode_bels_intf(
                "HARD_SYNC",
                xy,
                &[],
                &[int_xy],
                &[intf_xy],
                "HARD_SYNC",
                &bels,
            );
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("DSP").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let n = builder.db.get_node_naming("INTF.E");
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
            let mut bel = builder.bel_xy(format!("DSP{i}"), "DSP48E2", 0, i);
            let buf_cnt = match i {
                0 => 1,
                1 => 0,
                _ => unreachable!(),
            };
            bel = bel.pin_name_only("MULTSIGNIN", buf_cnt);
            bel = bel.pin_name_only("MULTSIGNOUT", 0);
            bel = bel.pin_name_only("CARRYCASCIN", buf_cnt);
            bel = bel.pin_name_only("CARRYCASCOUT", 0);
            for j in 0..30 {
                bel = bel.pin_name_only(&format!("ACIN_B{j}"), buf_cnt);
                bel = bel.pin_name_only(&format!("ACOUT_B{j}"), 0);
            }
            for j in 0..18 {
                bel = bel.pin_name_only(&format!("BCIN_B{j}"), buf_cnt);
                bel = bel.pin_name_only(&format!("BCOUT_B{j}"), 0);
            }
            for j in 0..48 {
                bel = bel.pin_name_only(&format!("PCIN{j}"), buf_cnt);
                bel = bel.pin_name_only(&format!("PCOUT{j}"), 0);
            }
            bels_dsp.push(bel);
        }
        builder.extract_xnode_bels_intf("DSP", xy, &[], &int_xy, &intf_xy, "DSP", &bels_dsp);
    }

    'a: {
        if let Some(&xy) = rd.tiles_by_kind_name("LAGUNA_TILE").iter().next() {
            let tk = &rd.tile_kinds[rd.tiles[&xy].kind];
            if tk.sites.is_empty() {
                break 'a;
            }
            let mut bels = vec![];
            for i in 0..4 {
                let mut bel = builder.bel_xy(&format!("LAGUNA{i}"), "LAGUNA", i >> 1, i & 1);
                for j in 0..6 {
                    bel = bel
                        .pin_name_only(&format!("RXQ{j}"), 0)
                        .pin_name_only(&format!("RXD{j}"), 0)
                        .pin_name_only(&format!("TXQ{j}"), 0)
                        .extra_int_out(&format!("RXOUT{j}"), &[format!("RXD{ii}", ii = i * 6 + j)])
                        .extra_wire(
                            &format!("TXOUT{j}"),
                            &[format!(
                                "LAG_MUX_ATOM_{ii}_TXOUT",
                                ii = match (i, j) {
                                    (0, 0) => 0,
                                    (0, 1) => 11,
                                    (0, 2) => 16,
                                    (0, 3) => 17,
                                    (0, 4) => 18,
                                    (0, 5) => 19,
                                    (1, 0) => 20,
                                    (1, 1) => 21,
                                    (1, 2) => 22,
                                    (1, 3) => 23,
                                    (1, 4) => 1,
                                    (1, 5) => 2,
                                    (2, 0) => 3,
                                    (2, 1) => 4,
                                    (2, 2) => 5,
                                    (2, 3) => 6,
                                    (2, 4) => 7,
                                    (2, 5) => 8,
                                    (3, 0) => 9,
                                    (3, 1) => 10,
                                    (3, 2) => 12,
                                    (3, 3) => 13,
                                    (3, 4) => 14,
                                    (3, 5) => 15,
                                    _ => unreachable!(),
                                }
                            )],
                        )
                        .extra_wire(
                            &format!("UBUMP{j}"),
                            &[format!("UBUMP{ii}", ii = i * 6 + j)],
                        );
                }
                bels.push(bel);
            }
            bels.push(builder.bel_virtual("VCC").extra_wire("VCC", &["VCC_WIRE"]));
            builder.extract_xnode_bels("LAGUNA", xy, &[], &[xy.delta(1, 0)], "LAGUNA", &bels);
        }
    }

    for (kind, tkn, bk) in [
        ("PCIE", "PCIE", "PCIE_3_1"),
        ("CMAC", "CMAC_CMAC_FT", "CMAC_SITE"),
        ("ILKN", "ILMAC_ILMAC_FT", "ILKN_SITE"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let int_l_xy = builder.walk_to_int(xy, Dir::W).unwrap();
            let int_r_xy = builder.walk_to_int(xy, Dir::E).unwrap();
            let intf_l = builder.db.get_node_naming("INTF.E.PCIE");
            let intf_r = builder.db.get_node_naming("INTF.W.PCIE");
            let mut bel = builder.bel_xy(kind, bk, 0, 0);
            if kind == "PCIE" {
                bel = bel.pin_dummy("MCAP_PERST0_B").pin_dummy("MCAP_PERST1_B");
            }
            let mut xn = builder.xnode(kind, kind, xy).num_tiles(120);
            for i in 0..60 {
                xn = xn
                    .ref_int(int_l_xy.delta(0, (i + i / 30) as i32), i)
                    .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i + 60)
                    .ref_single(int_l_xy.delta(1, (i + i / 30) as i32), i, intf_l)
                    .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i + 60, intf_r)
            }
            xn.bel(bel).extract();
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("CFG_CFG").iter().next() {
        let int_l_xy = builder.walk_to_int(xy, Dir::W).unwrap();
        let int_r_xy = builder.walk_to_int(xy, Dir::E).unwrap();
        let intf_l = builder.db.get_node_naming("INTF.E.PCIE");
        let intf_r = builder.db.get_node_naming("INTF.W.PCIE");
        let bels = [
            builder.bel_xy("CFG", "CONFIG_SITE", 0, 0),
            builder.bel_xy("ABUS_SWITCH", "ABUS_SWITCH", 0, 0),
        ];
        let mut xn = builder.xnode("CFG", "CFG", xy).num_tiles(120);
        for i in 0..60 {
            xn = xn
                .ref_int(int_l_xy.delta(0, (i + i / 30) as i32), i)
                .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i + 60)
                .ref_single(int_l_xy.delta(1, (i + i / 30) as i32), i, intf_l)
                .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i + 60, intf_r)
        }
        xn.bels(bels).extract();
    }

    if let Some(&xy) = rd.tiles_by_kind_name("CFGIO_IOB").iter().next() {
        let int_l_xy = builder.walk_to_int(xy, Dir::W).unwrap();
        let int_r_xy = builder.walk_to_int(xy, Dir::E).unwrap();
        let intf_l = builder.db.get_node_naming("INTF.E.PCIE");
        let intf_r = builder.db.get_node_naming("INTF.W.PCIE");
        let bels = [
            builder.bel_xy("PMV", "PMV", 0, 0),
            builder.bel_xy("PMV2", "PMV2", 0, 0),
            builder.bel_xy("PMVIOB", "PMVIOB", 0, 0),
            builder.bel_xy("MTBF3", "MTBF3", 0, 0),
        ];
        let mut xn = builder.xnode("CFGIO", "CFGIO", xy).num_tiles(60);
        for i in 0..30 {
            xn = xn
                .ref_int(int_l_xy.delta(0, (i + i / 30) as i32), i)
                .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i + 60)
                .ref_single(int_l_xy.delta(1, (i + i / 30) as i32), i, intf_l)
                .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i + 60, intf_r)
        }
        xn.bels(bels).extract();
    }

    if let Some(&xy) = rd.tiles_by_kind_name("AMS").iter().next() {
        let int_l_xy = builder.walk_to_int(xy, Dir::W).unwrap();
        let int_r_xy = builder.walk_to_int(xy, Dir::E).unwrap();
        let intf_l = builder.db.get_node_naming("INTF.E.PCIE");
        let intf_r = builder.db.get_node_naming("INTF.W.PCIE");
        let mut bel = builder.bel_xy("SYSMON", "SYSMONE1", 0, 0).pins_name_only(&[
            "I2C_SCLK_IN",
            "I2C_SCLK_TS",
            "I2C_SDA_IN",
            "I2C_SDA_TS",
        ]);
        for i in 0..16 {
            bel = bel.pins_name_only(&[format!("VP_AUX{i}"), format!("VN_AUX{i}")]);
        }
        let mut xn = builder.xnode("AMS", "AMS", xy).num_tiles(60);
        for i in 0..30 {
            xn = xn
                .ref_int(int_l_xy.delta(0, (i + i / 30) as i32), i)
                .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i + 60)
                .ref_single(int_l_xy.delta(1, (i + i / 30) as i32), i, intf_l)
                .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i + 60, intf_r)
        }
        xn.bel(bel).extract();
    }

    builder.build()
}
