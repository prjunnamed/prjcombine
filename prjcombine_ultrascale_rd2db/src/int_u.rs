use prjcombine_int::db::{Dir, IntDb, WireKind};
use prjcombine_rawdump::{Coord, Part};
use prjcombine_ultrascale::grid::DeviceNaming;

use prjcombine_rdintb::IntBuilder;

pub fn make_int_db(rd: &Part, dev_naming: &DeviceNaming) -> IntDb {
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

    for i in 0..2 {
        for j in 0..16 {
            builder.mux_out(
                format!("RCLK.IMUX.CE.{i}.{j}"),
                &[format!("CLK_BUFCE_LEAF_X16_{i}_CE_INT{j}")],
            );
        }
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
            let mut bels = vec![];
            for i in 0..2 {
                let ud = ['D', 'U'][i];
                let mut bel =
                    builder.bel_xy(format!("BUFCE_LEAF_X16_{ud}"), "BUFCE_LEAF_X16", 0, i as u8);
                for j in 0..16 {
                    bel = bel.pin_name_only(&format!("CLK_IN{j}"), 0);
                }
                bels.push(bel);
            }
            let mut bel = builder
                .bel_virtual("RCLK_INT")
                .extra_wire("VCC", &["VCC_WIRE"]);
            for i in 0..24 {
                bel = bel.extra_wire(format!("HDISTR{i}"), &[format!("CLK_HDISTR_FT0_{i}")]);
            }
            bels.push(bel);
            builder
                .xnode("RCLK_INT", "RCLK_INT", xy)
                .num_tiles(2)
                .ref_int(xy.delta(0, 1), 0)
                .ref_int(xy.delta(0, -1), 1)
                .extract_muxes()
                .bels(bels)
                .extract();
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

    if let Some(&xy) = rd.tiles_by_kind_name("LAGUNA_TILE").iter().next() {
        let mut bels = vec![];
        for i in 0..4 {
            let mut bel = builder.bel_xy(format!("LAGUNA{i}"), "LAGUNA", i >> 1, i & 1);
            for j in 0..6 {
                bel = bel
                    .pin_name_only(&format!("RXQ{j}"), 0)
                    .pin_name_only(&format!("RXD{j}"), 0)
                    .pin_name_only(&format!("TXQ{j}"), 0)
                    .extra_int_out(format!("RXOUT{j}"), &[format!("RXD{ii}", ii = i * 6 + j)])
                    .extra_wire(
                        format!("TXOUT{j}"),
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
                    .extra_wire(format!("UBUMP{j}"), &[format!("UBUMP{ii}", ii = i * 6 + j)]);
            }
            bels.push(bel);
        }
        bels.push(
            builder
                .bel_virtual("LAGUNA_EXTRA")
                .extra_wire("UBUMP", &["UBUMP_EXTRA"])
                .extra_wire("RXD", &["LAG_IOBUF_ATOM_16_RXO"])
                .extra_wire("TXOUT", &["VCC_WIRE0"]),
        );
        bels.push(
            builder
                .bel_virtual("VCC.LAGUNA")
                .extra_wire("VCC", &["VCC_WIRE"]),
        );
        builder.extract_xnode_bels("LAGUNA", xy, &[], &[xy.delta(1, 0)], "LAGUNA", &bels);
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
            builder.bel_xy("ABUS_SWITCH.CFG", "ABUS_SWITCH", 0, 0),
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
                .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i + 30)
                .ref_single(int_l_xy.delta(1, (i + i / 30) as i32), i, intf_l)
                .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i + 30, intf_r)
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
                .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i + 30)
                .ref_single(int_l_xy.delta(1, (i + i / 30) as i32), i, intf_l)
                .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i + 30, intf_r)
        }
        xn.bel(bel).extract();
    }

    for tkn in [
        "PCIE",
        "CMAC_CMAC_FT",
        "ILMAC_ILMAC_FT",
        "CFG_CFG",
        "RCLK_AMS_CFGIO",
        "RCLK_CLEM_CLKBUF_L",
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual("RCLK_HROUTE_SPLITTER");
            for i in 0..24 {
                bel = bel
                    .extra_wire(format!("HROUTE{i}_L"), &[format!("CLK_HROUTE_L{i}")])
                    .extra_wire(format!("HROUTE{i}_R"), &[format!("CLK_HROUTE_R{i}")]);
            }
            let bel_vcc = builder
                .bel_virtual("VCC.RCLK_HROUTE_SPLITTER")
                .extra_wire("VCC", &["VCC_WIRE"]);
            builder
                .xnode("RCLK_HROUTE_SPLITTER_L", "RCLK_HROUTE_SPLITTER", xy)
                .num_tiles(0)
                .bel(bel)
                .bel(bel_vcc)
                .extract();
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("RCLK_DSP_CLKBUF_L").iter().next() {
        let mut bel = builder.bel_virtual("RCLK_SPLITTER");
        for i in 0..24 {
            bel = bel
                .extra_wire(format!("HDISTR{i}_L"), &[format!("CLK_HDISTR_L{i}")])
                .extra_wire(format!("HDISTR{i}_R"), &[format!("CLK_HDISTR_R{i}")])
                .extra_wire(format!("HROUTE{i}_L"), &[format!("CLK_HROUTE_L{i}")])
                .extra_wire(format!("HROUTE{i}_R"), &[format!("CLK_HROUTE_R{i}")]);
        }
        let bel_vcc = builder
            .bel_virtual("VCC.RCLK_SPLITTER")
            .extra_wire("VCC", &["VCC_WIRE"]);
        builder
            .xnode("RCLK_SPLITTER", "RCLK_SPLITTER", xy)
            .num_tiles(0)
            .bel(bel)
            .bel(bel_vcc)
            .extract();
    }

    for (tkn, lr) in [
        ("RCLK_CLEL_L", 'L'),
        ("RCLK_CLEL_R", 'L'),
        ("RCLK_CLEL_R_L", 'R'),
        ("RCLK_CLEL_R_R", 'R'),
        ("RCLK_CLE_M_L", 'L'),
        ("RCLK_CLE_M_R", 'L'),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let is_alt = dev_naming.rclk_alt_pins[tkn];
            let rclk_int = builder.db.get_node_naming("RCLK_INT");
            let kind = if lr == 'L' {
                "RCLK_V_SINGLE_L"
            } else {
                "RCLK_V_SINGLE_R"
            };
            let int_xy = xy.delta(if lr == 'L' { 1 } else { -1 }, 0);
            let bels = vec![
                builder
                    .bel_xy(format!("BUFCE_ROW_{lr}0"), "BUFCE_ROW", 0, 0)
                    .pins_name_only(&["CLK_IN", "CLK_OUT", "CLK_OUT_OPT_DLY"])
                    .extra_wire("VDISTR_B", &["CLK_VDISTR_BOT"])
                    .extra_wire("VDISTR_T", &["CLK_VDISTR_TOP"])
                    .extra_wire("VROUTE_B", &["CLK_VROUTE_BOT"])
                    .extra_wire("VROUTE_T", &["CLK_VROUTE_TOP"])
                    .extra_wire("HROUTE", &["CLK_HROUTE_CORE_OPT"])
                    .extra_wire("VDISTR_B_MUX", &["CLK_CMT_MUX_3TO1_0_CLK_OUT"])
                    .extra_wire("VDISTR_T_MUX", &["CLK_CMT_MUX_3TO1_1_CLK_OUT"])
                    .extra_wire("VROUTE_B_MUX", &["CLK_CMT_MUX_3TO1_2_CLK_OUT"])
                    .extra_wire("VROUTE_T_MUX", &["CLK_CMT_MUX_3TO1_3_CLK_OUT"])
                    .extra_wire("HROUTE_MUX", &["CLK_CMT_MUX_2TO1_1_CLK_OUT"]),
                builder
                    .bel_xy(format!("GCLK_TEST_BUF_{lr}0"), "GCLK_TEST_BUFE3", 0, 0)
                    .pin_name_only("CLK_OUT", 0)
                    .pin_name_only("CLK_IN", usize::from(is_alt)),
                builder
                    .bel_virtual(format!("VCC.RCLK_V_{lr}"))
                    .extra_wire("VCC", &["VCC_WIRE"]),
            ];
            builder
                .xnode(
                    kind,
                    &if is_alt {
                        format!("{kind}.ALT")
                    } else {
                        kind.to_string()
                    },
                    xy,
                )
                .ref_xlat(int_xy, &[Some(0), None], rclk_int)
                .bels(bels)
                .extract();
        }
    }

    for (tkn, lr) in [
        ("RCLK_BRAM_L", 'L'),
        ("RCLK_BRAM_R", 'L'),
        ("RCLK_RCLK_BRAM_L_AUXCLMP_FT", 'L'),
        ("RCLK_RCLK_BRAM_L_BRAMCLMP_FT", 'L'),
        ("RCLK_DSP_L", 'R'),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let is_alt = dev_naming.rclk_alt_pins[tkn];
            let rclk_int = builder.db.get_node_naming("RCLK_INT");
            let kind = if lr == 'L' {
                "RCLK_V_DOUBLE_L"
            } else {
                "RCLK_V_DOUBLE_R"
            };
            let int_xy = xy.delta(if lr == 'L' { 2 } else { -2 }, 0);
            let mut bels = vec![];
            for i in 0..2 {
                bels.push(
                    builder
                        .bel_xy(format!("BUFCE_ROW_{lr}{i}"), "BUFCE_ROW", i, 0)
                        .pins_name_only(&["CLK_IN", "CLK_OUT", "CLK_OUT_OPT_DLY"])
                        .extra_wire("VDISTR_B", &[format!("CLK_VDISTR_BOT{i}")])
                        .extra_wire("VDISTR_T", &[format!("CLK_VDISTR_TOP{i}")])
                        .extra_wire("VROUTE_B", &[format!("CLK_VROUTE_BOT{i}")])
                        .extra_wire("VROUTE_T", &[format!("CLK_VROUTE_TOP{i}")])
                        .extra_wire("HROUTE", &[format!("CLK_HROUTE_CORE_OPT{i}")])
                        .extra_wire(
                            "VDISTR_B_MUX",
                            &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4)],
                        )
                        .extra_wire(
                            "VDISTR_T_MUX",
                            &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4 + 1)],
                        )
                        .extra_wire(
                            "VROUTE_B_MUX",
                            &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4 + 2)],
                        )
                        .extra_wire(
                            "VROUTE_T_MUX",
                            &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4 + 3)],
                        )
                        .extra_wire(
                            "HROUTE_MUX",
                            &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = i * 2 + 1)],
                        ),
                );
            }
            for i in 0..2 {
                bels.push(
                    builder
                        .bel_xy(format!("GCLK_TEST_BUF_{lr}{i}"), "GCLK_TEST_BUFE3", i, 0)
                        .pin_name_only("CLK_OUT", 0)
                        .pin_name_only("CLK_IN", usize::from(is_alt)),
                );
            }
            bels.push(
                builder
                    .bel_virtual(format!("VCC.RCLK_V_{lr}"))
                    .extra_wire("VCC", &["VCC_WIRE"]),
            );
            builder
                .xnode(
                    kind,
                    &if is_alt {
                        format!("{kind}.ALT")
                    } else {
                        kind.to_string()
                    },
                    xy,
                )
                .ref_xlat(int_xy, &[Some(0), None], rclk_int)
                .bels(bels)
                .extract();
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("XIPHY_L").iter().next() {
        let int_r_xy = builder.walk_to_int(xy, Dir::E).unwrap();
        let intf_r = builder.db.get_node_naming("INTF.W.IO");
        let rclk_int = builder.db.get_node_naming("RCLK_INT");
        let mut bels = vec![];
        for i in 0..24 {
            bels.push(
                builder
                    .bel_xy(format!("BUFCE_ROW_IO{i}"), "BUFCE_ROW", 0, i)
                    .pins_name_only(&["CLK_IN", "CLK_OUT", "CLK_OUT_OPT_DLY"]),
            );
        }
        for i in 0..24 {
            bels.push(
                builder
                    .bel_xy(format!("GCLK_TEST_BUF_IO{i}"), "GCLK_TEST_BUFE3", 0, i)
                    .pins_name_only(&["CLK_IN", "CLK_OUT"]),
            );
        }
        for i in 0..24 {
            bels.push(
                builder
                    .bel_xy(format!("BUFGCE{i}"), "BUFGCE", 0, i)
                    .pins_name_only(&["CLK_OUT"])
                    .pin_name_only("CLK_IN", usize::from(matches!(i, 5 | 11 | 17 | 23)))
                    .extra_wire(
                        "CLK_IN_MUX_HROUTE",
                        &[format!("CLK_LEAF_MUX_{ii}_CLK_LEAF", ii = i * 2 + 1)],
                    )
                    .extra_wire(
                        "CLK_IN_MUX_PLL_CKINT",
                        &[format!(
                            "CLK_CMT_MUX_3TO1_{ii}_CLK_OUT",
                            ii = i % 3 + i / 3 * 5
                        )],
                    )
                    .extra_wire(
                        "CLK_IN_MUX_TEST",
                        &[format!("CLK_CMT_MUX_4TO1_{i}_CLK_OUT")],
                    )
                    .extra_int_in("CLK_IN_CKINT", &[format!("CLB2CMT_CLK_INT{i}")]),
            );
        }
        for i in 0..8 {
            bels.push(
                builder
                    .bel_xy(format!("BUFGCTRL{i}"), "BUFGCTRL", 0, i)
                    .pins_name_only(&["CLK_I0", "CLK_I1", "CLK_OUT"]),
            );
        }
        for i in 0..4 {
            bels.push(
                builder
                    .bel_xy(format!("BUFGCE_DIV{i}"), "BUFGCE_DIV", 0, i)
                    .pins_name_only(&["CLK_IN", "CLK_OUT"]),
            );
        }
        for i in 0..2 {
            bels.push(
                builder
                    .bel_xy(format!("PLL{i}"), "PLLE3_ADV", 0, i)
                    .pins_name_only(&[
                        "CLKOUT0",
                        "CLKOUT0B",
                        "CLKOUT1",
                        "CLKOUT1B",
                        "CLKFBOUT",
                        "TMUXOUT",
                        "CLKOUTPHY_P",
                        "CLKIN",
                        "CLKFBIN",
                    ])
                    .extra_wire(
                        "CLKFBIN_MUX_HDISTR",
                        &[format!("CLK_LEAF_MUX_{ii}_CLK_LEAF", ii = 56 + i * 2)],
                    )
                    .extra_wire(
                        "CLKFBIN_MUX_BUFCE_ROW_DLY",
                        &[format!("CLK_LEAF_MUX_{ii}_CLK_LEAF", ii = 57 + i * 2)],
                    )
                    .extra_wire(
                        "CLKIN_MUX_MMCM",
                        &[format!("CLK_CMT_MUX_4TO1_{ii}_CLK_OUT", ii = 24 + i)],
                    )
                    .extra_wire(
                        "CLKIN_MUX_HDISTR",
                        &[format!("CLK_LEAF_MUX_{ii}_CLK_LEAF", ii = 60 + i * 3)],
                    )
                    .extra_wire(
                        "CLKIN_MUX_HROUTE",
                        &[format!("CLK_LEAF_MUX_{ii}_CLK_LEAF", ii = 61 + i * 3)],
                    )
                    .extra_wire(
                        "CLKIN_MUX_BUFCE_ROW_DLY",
                        &[format!("CLK_LEAF_MUX_{ii}_CLK_LEAF", ii = 62 + i * 3)],
                    ),
            );
        }
        bels.push(
            builder
                .bel_xy("MMCM", "MMCME3_ADV", 0, 0)
                .pins_name_only(&[
                    "CLKOUT0",
                    "CLKOUT0B",
                    "CLKOUT1",
                    "CLKOUT1B",
                    "CLKOUT2",
                    "CLKOUT2B",
                    "CLKOUT3",
                    "CLKOUT3B",
                    "CLKOUT4",
                    "CLKOUT5",
                    "CLKOUT6",
                    "CLKFBOUT",
                    "CLKFBOUTB",
                    "TMUXOUT",
                    "CLKIN1",
                    "CLKIN2",
                    "CLKFBIN",
                ])
                .extra_wire("CLKFBIN_MUX_HDISTR", &["CLK_LEAF_MUX_48_CLK_LEAF"])
                .extra_wire("CLKFBIN_MUX_BUFCE_ROW_DLY", &["CLK_LEAF_MUX_49_CLK_LEAF"])
                .extra_wire("CLKFBIN_MUX_DUMMY0", &["VCC_WIRE51"])
                .extra_wire("CLKFBIN_MUX_DUMMY1", &["VCC_WIRE52"])
                .extra_wire("CLKIN1_MUX_HDISTR", &["CLK_LEAF_MUX_50_CLK_LEAF"])
                .extra_wire("CLKIN1_MUX_HROUTE", &["CLK_LEAF_MUX_51_CLK_LEAF"])
                .extra_wire("CLKIN1_MUX_BUFCE_ROW_DLY", &["CLK_LEAF_MUX_52_CLK_LEAF"])
                .extra_wire("CLKIN1_MUX_DUMMY0", &["GND_WIRE0"])
                .extra_wire("CLKIN2_MUX_HDISTR", &["CLK_LEAF_MUX_53_CLK_LEAF"])
                .extra_wire("CLKIN2_MUX_HROUTE", &["CLK_LEAF_MUX_54_CLK_LEAF"])
                .extra_wire("CLKIN2_MUX_BUFCE_ROW_DLY", &["CLK_LEAF_MUX_55_CLK_LEAF"])
                .extra_wire("CLKIN2_MUX_DUMMY0", &["GND_WIRE1"]),
        );
        bels.push(builder.bel_xy("ABUS_SWITCH.CMT", "ABUS_SWITCH", 0, 0));

        // XIPHY
        for i in 0..52 {
            let mut bel = builder
                .bel_xy(format!("BITSLICE_RX_TX{i}"), "BITSLICE_RX_TX", 0, i)
                .pins_name_only(&[
                    "TX_CLK",
                    "TX_OCLK",
                    "TX_DIV2_CLK",
                    "TX_DIV4_CLK",
                    "TX_DDR_CLK",
                    "TX_CTRL_CLK",
                    "TX_CTRL_CE",
                    "TX_CTRL_INC",
                    "TX_CTRL_LD",
                    "TX_OCLKDIV",
                    "TX_TBYTE_IN",
                    "TX_WL_TRAIN",
                    "TX_MUX_360_N_SEL",
                    "TX_MUX_360_P_SEL",
                    "TX_MUX_720_P0_SEL",
                    "TX_MUX_720_P1_SEL",
                    "TX_MUX_720_P2_SEL",
                    "TX_MUX_720_P3_SEL",
                    "TX_VTC_READY",
                    "TX_TOGGLE_DIV2_SEL",
                    "TX_BS_RESET",
                    "TX_REGRST_B",
                    "TX_RST_B",
                    "TX_Q",
                    "RX_CLK_C",
                    "RX_CLK_C_B",
                    "RX_CLK_P",
                    "RX_CLK_N",
                    "RX_CTRL_CLK",
                    "RX_CTRL_CE",
                    "RX_CTRL_INC",
                    "RX_CTRL_LD",
                    "RX_RST_B",
                    "RX_CLKDIV",
                    "RX_DCC0",
                    "RX_DCC1",
                    "RX_DCC2",
                    "RX_DCC3",
                    "RX_VTC_READY",
                    "RX_RESET_B",
                    "RX_BS_RESET",
                    "RX_DQS_OUT",
                    "TX2RX_CASC_IN",
                    "TX2RX_CASC_OUT",
                    "RX2TX_CASC_RETURN_IN",
                    "PHY2CLB_FIFO_WRCLK",
                    "CLB2PHY_FIFO_CLK",
                    "CTL2BS_FIFO_BYPASS",
                    "CTL2BS_RX_RECALIBRATE_EN",
                    "CTL2BS_TX_DDR_PHASE_SEL",
                    "CTL2BS_DYNAMIC_MODE_EN",
                    "BS2CTL_IDELAY_DELAY_FORMAT",
                    "BS2CTL_ODELAY_DELAY_FORMAT",
                    "BS2CTL_TX_DDR_PHASE_SEL",
                    "BS2CTL_RX_P0_DQ_OUT",
                    "BS2CTL_RX_N0_DQ_OUT",
                    "BS2CTL_RX_DDR_EN_DQS",
                ])
                .pin_name_only("RX_CLK", 1)
                .pin_name_only("RX_D", 1)
                .extra_wire(
                    "DYN_DCI_OUT",
                    &[match i {
                        0..=11 => format!("DYNAMIC_DCI_TS_BOT{i}"),
                        12 => "DYNAMIC_DCI_TS_BOT_VR1".to_string(),
                        13..=24 => format!("DYNAMIC_DCI_TS_BOT{ii}", ii = i - 1),
                        25 => "DYNAMIC_DCI_TS_BOT_VR2".to_string(),
                        26..=37 => format!("DYNAMIC_DCI_TS_TOP{ii}", ii = i - 26),
                        38 => "DYNAMIC_DCI_TS_TOP_VR1".to_string(),
                        39..=50 => format!("DYNAMIC_DCI_TS_TOP{ii}", ii = i - 27),
                        51 => "DYNAMIC_DCI_TS_TOP_VR2".to_string(),
                        _ => unreachable!(),
                    }],
                )
                .extra_int_in(
                    "DYN_DCI_OUT_INT",
                    &[match i {
                        0..=11 => format!("CLB2PHY_DYNAMIC_DCI_TS_BOT{i}"),
                        12 => "CLB2PHY_DYNAMIC_DCI_TS_BOT_VR1".to_string(),
                        13..=24 => format!("CLB2PHY_DYNAMIC_DCI_TS_BOT{ii}", ii = i - 1),
                        25 => "CLB2PHY_DYNAMIC_DCI_TS_BOT_VR2".to_string(),
                        26..=37 => format!("CLB2PHY_DYNAMIC_DCI_TS_TOP{ii}", ii = i - 26),
                        38 => "CLB2PHY_DYNAMIC_DCI_TS_TOP_VR1".to_string(),
                        39..=50 => format!("CLB2PHY_DYNAMIC_DCI_TS_TOP{ii}", ii = i - 27),
                        51 => "CLB2PHY_DYNAMIC_DCI_TS_TOP_VR2".to_string(),
                        _ => unreachable!(),
                    }],
                );
            for i in 0..18 {
                bel = bel.pins_name_only(&[
                    format!("BS2CTL_IDELAY_FIXED_DLY_RATIO{i}"),
                    format!("BS2CTL_ODELAY_FIXED_DLY_RATIO{i}"),
                ]);
            }
            for i in 0..9 {
                bel = bel.pins_name_only(&[
                    format!("BS2CTL_RX_CNTVALUEOUT{i}"),
                    format!("BS2CTL_TX_CNTVALUEOUT{i}"),
                    format!("RX_CTRL_DLY{i}"),
                    format!("TX_CTRL_DLY{i}"),
                ]);
            }
            bels.push(bel);
        }
        for i in 0..8 {
            let mut bel = builder
                .bel_xy(format!("BITSLICE_TX{i}"), "BITSLICE_TX", 0, i)
                .pins_name_only(&[
                    "CLK",
                    "DIV2_CLK",
                    "DIV4_CLK",
                    "DDR_CLK",
                    "CTRL_CLK",
                    "CTRL_CE",
                    "CTRL_INC",
                    "CTRL_LD",
                    "TX_MUX_360_N_SEL",
                    "TX_MUX_360_P_SEL",
                    "TX_MUX_720_P0_SEL",
                    "TX_MUX_720_P1_SEL",
                    "TX_MUX_720_P2_SEL",
                    "TX_MUX_720_P3_SEL",
                    "TOGGLE_DIV2_SEL",
                    "D0",
                    "D1",
                    "D2",
                    "D3",
                    "D4",
                    "D5",
                    "D6",
                    "D7",
                    "Q",
                    "RST_B",
                    "REGRST_B",
                    "BS_RESET",
                    "CDATAIN0",
                    "CDATAIN1",
                    "CDATAOUT",
                    "CTL2BS_TX_DDR_PHASE_SEL",
                    "CTL2BS_DYNAMIC_MODE_EN",
                    "BS2CTL_TX_DDR_PHASE_SEL",
                    "FORCE_OE_B",
                    "VTC_READY",
                ]);
            for i in 0..9 {
                bel =
                    bel.pins_name_only(&[format!("BS2CTL_CNTVALUEOUT{i}"), format!("CTRL_DLY{i}")]);
            }
            bels.push(bel);
        }
        for i in 0..8 {
            let mut bel = builder
                .bel_xy(format!("BITSLICE_CONTROL{i}"), "BITSLICE_CONTROL", 0, i)
                .pins_name_only(&[
                    "PDQS_GT_IN",
                    "NDQS_GT_IN",
                    "PDQS_GT_OUT",
                    "NDQS_GT_OUT",
                    "FORCE_OE_B",
                    "PLL_CLK",
                    "PLL_CLK_EN",
                    "REFCLK_DFD",
                    "CLK_TO_EXT_SOUTH",
                    "CLK_TO_EXT_NORTH",
                    "CLB2PHY_CTRL_RST_B",
                    "LOCAL_DIV_CLK",
                    "BS_RESET_TRI",
                    "TRISTATE_ODELAY_CE_OUT",
                    "TRISTATE_ODELAY_INC_OUT",
                    "TRISTATE_ODELAY_LD_OUT",
                    "TRISTATE_VTC_READY",
                    "SCAN_INT",
                    "RIU2CLB_VALID",
                    "CLK_STOP",
                    "CLK_FROM_EXT",
                ]);
            for i in 0..7 {
                bel = bel.pins_name_only(&[
                    format!("RX_DCC{i:02}_0"),
                    format!("RX_DCC{i:02}_1"),
                    format!("RX_DCC{i:02}_2"),
                    format!("RX_DCC{i:02}_3"),
                    format!("RX_PDQ{i}_IN"),
                    format!("RX_NDQ{i}_IN"),
                    format!("IDELAY_CTRL_CLK{i}"),
                    format!("IDELAY_CE_OUT{i}"),
                    format!("IDELAY_INC_OUT{i}"),
                    format!("IDELAY_LD_OUT{i}"),
                    format!("FIXED_IDELAY{i:02}"),
                    format!("ODELAY_CE_OUT{i}"),
                    format!("ODELAY_INC_OUT{i}"),
                    format!("ODELAY_LD_OUT{i}"),
                    format!("FIXED_ODELAY{i:02}"),
                    format!("VTC_READY_IDELAY{i:02}"),
                    format!("VTC_READY_ODELAY{i:02}"),
                    format!("WL_TRAIN{i}"),
                    format!("DYN_DCI_OUT{i}"),
                    format!("DQS_IN{i}"),
                    format!("RX_BS_RESET{i}"),
                    format!("TX_BS_RESET{i}"),
                    format!("PDQS_OUT{i}"),
                    format!("NDQS_OUT{i}"),
                    format!("REFCLK_EN{i}"),
                    format!("IFIFO_BYPASS{i}"),
                    format!("BS2CTL_RIU_BS_DQS_EN{i}"),
                ]);
                for j in 0..9 {
                    bel = bel.pins_name_only(&[
                        format!("IDELAY{i:02}_IN{j}"),
                        format!("IDELAY{i:02}_OUT{j}"),
                        format!("ODELAY{i:02}_IN{j}"),
                        format!("ODELAY{i:02}_OUT{j}"),
                    ]);
                }
                for j in 0..18 {
                    bel = bel.pins_name_only(&[
                        format!("FIXDLYRATIO_IDELAY{i:02}_{j}"),
                        format!("FIXDLYRATIO_ODELAY{i:02}_{j}"),
                    ]);
                }
            }
            for i in 0..8 {
                bel = bel.pins_name_only(&[
                    format!("ODELAY_CTRL_CLK{i}"),
                    format!("DYNAMIC_MODE_EN{i}"),
                    format!("EN_DIV_DLY_OE{i}"),
                    format!("TOGGLE_DIV2_SEL{i}"),
                    format!("TX_DATA_PHASE{i}"),
                    format!("BS2CTL_RIU_TX_DATA_PHASE{i}"),
                    format!("DIV2_CLK_OUT{i}"),
                    format!("DIV_CLK_OUT{i}"),
                    format!("DDR_CLK_OUT{i}"),
                    format!("PH02_DIV2_360_{i}"),
                    format!("PH13_DIV2_360_{i}"),
                    format!("PH0_DIV_720_{i}"),
                    format!("PH1_DIV_720_{i}"),
                    format!("PH2_DIV_720_{i}"),
                    format!("PH3_DIV_720_{i}"),
                ]);
            }
            for i in 0..9 {
                bel = bel.pins_name_only(&[
                    format!("TRISTATE_ODELAY_IN{i}"),
                    format!("TRISTATE_ODELAY_OUT{i}"),
                ]);
            }
            for i in 0..16 {
                bel = bel.pins_name_only(&[format!("RIU2CLB_RD_DATA{i}")]);
            }
            bels.push(bel);
        }
        for i in 0..8 {
            bels.push(
                builder
                    .bel_xy(format!("PLL_SELECT{i}"), "PLL_SELECT_SITE", 0, i ^ 1)
                    .pins_name_only(&["REFCLK_DFD", "Z", "PLL_CLK_EN"])
                    .pin_name_only("D0", 1)
                    .pin_name_only("D1", 1),
            );
        }
        for i in 0..4 {
            let mut bel = builder
                .bel_xy(format!("RIU_OR{i}"), "RIU_OR", 0, i)
                .pins_name_only(&["RIU_RD_VALID_LOW", "RIU_RD_VALID_UPP"]);
            for i in 0..16 {
                bel = bel.pins_name_only(&[
                    format!("RIU_RD_DATA_LOW{i}"),
                    format!("RIU_RD_DATA_UPP{i}"),
                ]);
            }
            bels.push(bel);
        }
        for i in 0..4 {
            let mut bel = builder
                .bel_xy(format!("XIPHY_FEEDTHROUGH{i}"), "XIPHY_FEEDTHROUGH", i, 0)
                .pins_name_only(&[
                    "CLB2PHY_CTRL_RST_B_LOW_SMX",
                    "CLB2PHY_CTRL_RST_B_UPP_SMX",
                    "CLB2PHY_TRISTATE_ODELAY_RST_B_SMX0",
                    "CLB2PHY_TRISTATE_ODELAY_RST_B_SMX1",
                    "CLB2PHY_TXBIT_TRI_RST_B_SMX0",
                    "CLB2PHY_TXBIT_TRI_RST_B_SMX1",
                    "SCAN_INT_LOWER",
                    "SCAN_INT_UPPER",
                    "DIV_CLK_OUT_LOW",
                    "DIV_CLK_OUT_UPP",
                    "XIPHY_CLK_STOP_CTRL_LOW",
                    "XIPHY_CLK_STOP_CTRL_UPP",
                    "RCLK2PHY_CLKDR",
                    "RCLK2PHY_SHIFTDR",
                ]);
            for i in 0..13 {
                bel = bel.pins_name_only(&[
                    format!("CLB2PHY_TXBIT_RST_B_SMX{i}"),
                    format!("CLB2PHY_RXBIT_RST_B_SMX{i}"),
                    format!("CLB2PHY_FIFO_CLK_SMX{i}"),
                    format!("CLB2PHY_IDELAY_RST_B_SMX{i}"),
                    format!("CLB2PHY_ODELAY_RST_B_SMX{i}"),
                ]);
            }
            for i in 0..6 {
                bel = bel.pins_name_only(&[format!("CTL2BS_REFCLK_EN_LOW_SMX{i}")]);
            }
            for i in 0..7 {
                bel = bel.pins_name_only(&[
                    format!("CTL2BS_REFCLK_EN_LOW{i}"),
                    format!("CTL2BS_REFCLK_EN_UPP{i}"),
                    format!("CTL2BS_REFCLK_EN_UPP_SMX{i}"),
                ]);
            }
            bels.push(bel);
        }

        let mut bel = builder.bel_virtual("CMT");
        for i in 0..4 {
            bel = bel.extra_wire(format!("CCIO{i}"), &[format!("IOB2CLK_CCIO{i}")]);
        }
        for i in 0..24 {
            let dummy_base = [
                0, 3, 36, 53, 56, 59, 62, 65, 68, 71, 6, 9, 12, 15, 18, 21, 24, 27, 30, 33, 39, 42,
                45, 48,
            ][i];
            bel = bel
                .extra_wire(format!("VDISTR{i}_B"), &[format!("CLK_VDISTR_BOT{i}")])
                .extra_wire(format!("VDISTR{i}_T"), &[format!("CLK_VDISTR_TOP{i}")])
                .extra_wire(format!("HROUTE{i}_L"), &[format!("CLK_HROUTE_0_{i}")])
                .extra_wire(format!("HROUTE{i}_R"), &[format!("CLK_HROUTE_1_{i}")])
                .extra_wire(format!("HDISTR{i}_L"), &[format!("CLK_HDISTR_0_{i}")])
                .extra_wire(format!("HDISTR{i}_R"), &[format!("CLK_HDISTR_1_{i}")])
                .extra_wire(
                    format!("HDISTR{i}_L_MUX"),
                    &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 1 + i * 8)],
                )
                .extra_wire(
                    format!("HDISTR{i}_R_MUX"),
                    &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = i * 8)],
                )
                .extra_wire(
                    format!("HDISTR{i}_OUT_MUX"),
                    &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 4 + i * 8)],
                )
                .extra_wire(
                    format!("HROUTE{i}_L_MUX"),
                    &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 3 + i * 8)],
                )
                .extra_wire(
                    format!("HROUTE{i}_R_MUX"),
                    &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 2 + i * 8)],
                )
                .extra_wire(
                    format!("VDISTR{i}_B_MUX"),
                    &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 6 + i * 8)],
                )
                .extra_wire(
                    format!("VDISTR{i}_T_MUX"),
                    &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 7 + i * 8)],
                )
                .extra_wire(
                    format!("OUT_MUX{i}"),
                    &[format!("CLK_CMT_MUX_16_ENC_{i}_CLK_OUT")],
                )
                .extra_wire(
                    format!("OUT_MUX{i}_DUMMY0"),
                    &[format!("VCC_WIRE{ii}", ii = dummy_base)],
                )
                .extra_wire(
                    format!("OUT_MUX{i}_DUMMY1"),
                    &[format!("VCC_WIRE{ii}", ii = dummy_base + 1)],
                )
                .extra_wire(
                    format!("OUT_MUX{i}_DUMMY2"),
                    &[format!("VCC_WIRE{ii}", ii = dummy_base + 2)],
                );
        }
        for i in 0..6 {
            bel = bel
                .extra_wire(
                    format!("XIPHY_CLK{i}_B"),
                    &[format!("CLK_LEAF_MUX_XIPHY_{ii}_CLK_LEAF", ii = i + 6)],
                )
                .extra_wire(
                    format!("XIPHY_CLK{i}_T"),
                    &[format!("CLK_LEAF_MUX_XIPHY_{i}_CLK_LEAF")],
                );
        }
        bels.push(bel);
        bels.push(
            builder
                .bel_virtual("VCC.CMT")
                .extra_wire("VCC", &["VCC_WIRE"]),
        );
        let mut xn = builder
            .xnode("XIPHY", "XIPHY", xy)
            .num_tiles(60)
            .ref_single(int_r_xy.delta(0, 30), 30, rclk_int);
        for i in 0..60 {
            xn = xn
                .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i)
                .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i, intf_r)
        }
        xn.bels(bels).extract();
    }

    if let Some(&xy) = rd.tiles_by_kind_name("HPIO_L").iter().next() {
        let int_r_xy = builder.walk_to_int(xy, Dir::E).unwrap();
        let intf_r = builder.db.get_node_naming("INTF.W.IO");
        let mut bels = vec![];
        for i in 0..26 {
            let mut bel = builder
                .bel_xy(format!("HPIOB{i}"), "IOB", 0, i)
                .pins_name_only(&[
                    "I",
                    "OUTB_B_IN",
                    "OUTB_B",
                    "TSTATE_IN",
                    "TSTATE_OUT",
                    "CTLE_IN",
                    "DOUT",
                    "IO",
                    "LVDS_TRUE",
                    "PAD_RES",
                    "O_B",
                    "TSTATEB",
                    "DYNAMIC_DCI_TS",
                    "VREF",
                ])
                .pin_name_only("SWITCH_OUT", usize::from(matches!(i, 4..=11 | 13..=20)))
                .pin_name_only("OP", 1)
                .pin_name_only("TSP", 1)
                .pin_dummy("TSDI");
            if matches!(i, 12 | 25) {
                bel = bel.pin_dummy("IO");
            }
            bels.push(bel);
        }
        for i in 0..12 {
            bels.push(
                builder
                    .bel_xy(format!("HPIODIFFIN{i}"), "HPIOBDIFFINBUF", 0, i)
                    .pins_name_only(&[
                        "LVDS_TRUE",
                        "LVDS_COMP",
                        "PAD_RES_0",
                        "PAD_RES_1",
                        "VREF",
                        "CTLE_IN_1",
                    ]),
            );
        }
        for i in 0..12 {
            bels.push(
                builder
                    .bel_xy(format!("HPIODIFFOUT{i}"), "HPIOBDIFFOUTBUF", 0, i)
                    .pins_name_only(&["AOUT", "BOUT", "O_B", "TSTATEB"]),
            );
        }
        bels.push(
            builder
                .bel_xy("HPIO_VREF", "HPIO_VREF_SITE", 0, 0)
                .pins_name_only(&["VREF1", "VREF2"]),
        );
        let mut xn = builder.xnode("HPIO", "HPIO", xy).num_tiles(30);
        for i in 0..30 {
            xn = xn
                .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i)
                .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i, intf_r)
        }
        xn.bels(bels).extract();
    }

    if let Some(&xy) = rd.tiles_by_kind_name("HRIO_L").iter().next() {
        let int_r_xy = builder.walk_to_int(xy, Dir::E).unwrap();
        let intf_r = builder.db.get_node_naming("INTF.W.IO");
        let mut bels = vec![];
        for i in 0..26 {
            let mut bel = builder
                .bel_xy(format!("HRIOB{i}"), "IOB", 0, i)
                .pins_name_only(&[
                    "DOUT",
                    "OUTB_B_IN",
                    "OUTB_B",
                    "TSTATEIN",
                    "TSTATEOUT",
                    "IO",
                    "TSDI",
                    "TMDS_IBUF_OUT",
                    "DRIVER_BOT_IBUF",
                    "O_B",
                    "TSTATEB",
                    "DYNAMIC_DCI_TS",
                ])
                .pin_name_only("SWITCH_OUT", usize::from(matches!(i, 4..=11 | 13..=20)))
                .pin_name_only("OP", 1)
                .pin_name_only("TSP", 1)
                .pin_dummy("TSDI");
            if matches!(i, 12 | 25) {
                bel = bel.pin_dummy("IO");
            }
            bels.push(bel);
        }
        for i in 0..12 {
            bels.push(
                builder
                    .bel_xy(format!("HRIODIFFIN{i}"), "HRIODIFFINBUF", 0, i)
                    .pins_name_only(&[
                        "LVDS_IBUF_OUT",
                        "LVDS_IBUF_OUT_B",
                        "LVDS_IN_P",
                        "LVDS_IN_N",
                    ]),
            );
        }
        for i in 0..12 {
            bels.push(
                builder
                    .bel_xy(format!("HRIODIFFOUT{i}"), "HRIODIFFOUTBUF", 0, i)
                    .pins_name_only(&["AOUT", "BOUT", "O_B", "TSTATEB"]),
            );
        }
        let mut xn = builder.xnode("HRIO", "HRIO", xy).num_tiles(30);
        for i in 0..30 {
            xn = xn
                .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i)
                .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i, intf_r)
        }
        xn.bels(bels).extract();
    }

    if let Some(&xy) = rd.tiles_by_kind_name("RCLK_HPIO_L").iter().next() {
        let int_r_xy = builder.walk_to_int(xy.delta(0, -30), Dir::E).unwrap();
        let intf_r = builder.db.get_node_naming("INTF.W.IO");
        let mut bels = vec![];
        for i in 0..5 {
            bels.push(builder.bel_xy(format!("ABUS_SWITCH.HPIO{i}"), "ABUS_SWITCH", i, 0));
        }
        bels.push(builder.bel_xy("HPIO_ZMATCH_BLK_HCLK", "HPIO_ZMATCH_BLK_HCLK", 0, 0));
        let mut xn = builder.xnode("RCLK_HPIO", "RCLK_HPIO", xy).num_tiles(60);
        for i in 0..60 {
            xn = xn
                .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i)
                .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i, intf_r)
        }
        xn.bels(bels).extract();
    }

    if let Some(&xy) = rd.tiles_by_kind_name("RCLK_HRIO_L").iter().next() {
        let mut bels = vec![];
        for i in 0..8 {
            bels.push(builder.bel_xy(format!("ABUS_SWITCH.HRIO{i}"), "ABUS_SWITCH", i, 0));
        }
        builder
            .xnode("RCLK_HRIO", "RCLK_HRIO", xy)
            .num_tiles(0)
            .bels(bels)
            .extract();
    }

    for (tkn, kind, is_l) in [
        ("GTH_QUAD_LEFT_FT", "GTH_L", true),
        ("GTY_QUAD_LEFT_FT", "GTY_L", true),
        ("GTH_R", "GTH_R", false),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let int_xy = builder
                .walk_to_int(xy, if is_l { Dir::E } else { Dir::W })
                .unwrap();
            let intf = builder
                .db
                .get_node_naming(if is_l { "INTF.W.GT" } else { "INTF.E.GT" });
            let rclk_int = builder.db.get_node_naming("RCLK_INT");
            let gtk = &kind[..3];
            let mut bels = vec![];
            for i in 0..24 {
                let bi = [
                    (0, 5, 60),
                    (135, 140, 145),
                    (215, 220, 230),
                    (235, 240, 245),
                    (250, 255, 260),
                    (265, 270, 275),
                    (285, 290, 295),
                    (300, 305, 310),
                    (315, 320, 325),
                    (330, 340, 345),
                    (115, 170, 225),
                    (280, 335, 350),
                    (355, 10, 15),
                    (20, 25, 30),
                    (35, 40, 45),
                    (50, 55, 65),
                    (70, 75, 80),
                    (85, 90, 95),
                    (100, 105, 110),
                    (120, 125, 130),
                    (150, 155, 160),
                    (165, 175, 180),
                    (185, 190, 195),
                    (200, 205, 210),
                ][i];
                let mut bel = builder
                    .bel_xy(format!("BUFG_GT{i}"), "BUFG_GT", 0, i as u8)
                    .pins_name_only(&["CLK_IN", "CLK_OUT", "CE", "RST_PRE_OPTINV"]);
                for j in 0..5 {
                    bel = bel
                        .extra_wire(
                            format!("CE_MUX_DUMMY{j}"),
                            &[format!("VCC_WIRE{ii}", ii = bi.0 + j)],
                        )
                        .extra_wire(
                            format!("CLK_IN_MUX_DUMMY{j}"),
                            &[format!("VCC_WIRE{ii}", ii = bi.1 + j)],
                        )
                        .extra_wire(
                            format!("RST_MUX_DUMMY{j}"),
                            &[format!("VCC_WIRE{ii}", ii = bi.2 + j)],
                        );
                }
                bels.push(bel);
            }
            for i in 0..11 {
                let mut bel = builder
                    .bel_xy(format!("BUFG_GT_SYNC{i}"), "BUFG_GT_SYNC", 0, i)
                    .pins_name_only(&["CE_OUT", "RST_OUT"]);
                if i != 10 {
                    bel = bel.pins_name_only(&["CLK_IN"]);
                }
                bels.push(bel);
            }
            for i in 0..4 {
                bels.push(builder.bel_xy(format!("ABUS_SWITCH.GT{i}"), "ABUS_SWITCH", 0, i));
            }

            for i in 0..4 {
                bels.push(
                    builder
                        .bel_xy(
                            format!("{gtk}_CHANNEL{i}"),
                            &format!("{gtk}E3_CHANNEL"),
                            0,
                            i,
                        )
                        .pins_name_only(&[
                            "MGTREFCLK0",
                            "MGTREFCLK1",
                            "NORTHREFCLK0",
                            "NORTHREFCLK1",
                            "SOUTHREFCLK0",
                            "SOUTHREFCLK1",
                            "QDCMREFCLK0_INT",
                            "QDCMREFCLK1_INT",
                            "QDPLL0CLK0P_INT",
                            "QDPLL1CLK0P_INT",
                            "RING_OSC_CLK_INT",
                            "RXRECCLKOUT",
                            "RXRECCLK_INT",
                            "TXOUTCLK_INT",
                        ]),
                );
            }
            bels.push(
                builder
                    .bel_xy(format!("{gtk}_COMMON"), &format!("{gtk}E3_COMMON"), 0, 0)
                    .pins_name_only(&[
                        "RXRECCLK0",
                        "RXRECCLK1",
                        "RXRECCLK2",
                        "RXRECCLK3",
                        "QDCMREFCLK_INT_0",
                        "QDCMREFCLK_INT_1",
                        "QDPLLCLK0P_0",
                        "QDPLLCLK0P_1",
                        "COM0_REFCLKOUT0",
                        "COM0_REFCLKOUT1",
                        "COM0_REFCLKOUT2",
                        "COM0_REFCLKOUT3",
                        "COM0_REFCLKOUT4",
                        "COM0_REFCLKOUT5",
                        "COM2_REFCLKOUT0",
                        "COM2_REFCLKOUT1",
                        "COM2_REFCLKOUT2",
                        "COM2_REFCLKOUT3",
                        "COM2_REFCLKOUT4",
                        "COM2_REFCLKOUT5",
                        "MGTREFCLK0",
                        "MGTREFCLK1",
                        "REFCLK2HROW0",
                        "REFCLK2HROW1",
                        "SARC_CLK0",
                        "SARC_CLK1",
                        "SARC_CLK2",
                        "SARC_CLK3",
                    ])
                    .extra_wire("CLKOUT_NORTH0", &["CLKOUT_NORTH0"])
                    .extra_wire("CLKOUT_NORTH1", &["CLKOUT_NORTH1"])
                    .extra_wire("CLKOUT_SOUTH0", &["CLKOUT_SOUTH0"])
                    .extra_wire("CLKOUT_SOUTH1", &["CLKOUT_SOUTH1"])
                    .extra_wire(
                        "NORTHREFCLK0",
                        &[
                            "GTH_CHANNEL_BLH_0_NORTHREFCLK0",
                            "GTY_CHANNEL_BLH_1_NORTHREFCLK0",
                        ],
                    )
                    .extra_wire(
                        "NORTHREFCLK1",
                        &[
                            "GTH_CHANNEL_BLH_0_NORTHREFCLK1",
                            "GTY_CHANNEL_BLH_1_NORTHREFCLK1",
                        ],
                    )
                    .extra_wire(
                        "SOUTHREFCLK0",
                        &[
                            "GTH_CHANNEL_BLH_0_SOUTHREFCLK0",
                            "GTY_CHANNEL_BLH_1_SOUTHREFCLK0",
                        ],
                    )
                    .extra_wire(
                        "SOUTHREFCLK1",
                        &[
                            "GTH_CHANNEL_BLH_0_SOUTHREFCLK1",
                            "GTY_CHANNEL_BLH_1_SOUTHREFCLK1",
                        ],
                    ),
            );
            let mut bel = builder.bel_virtual(if is_l { "RCLK_GT_L" } else { "RCLK_GT_R" });
            for i in 0..24 {
                if is_l {
                    bel = bel
                        .extra_wire(format!("HDISTR{i}_R"), &[format!("CLK_HDISTR_FT1_{i}")])
                        .extra_wire(format!("HROUTE{i}_R"), &[format!("CLK_HROUTE_FT1_{i}")]);
                } else {
                    bel = bel
                        .extra_wire(format!("HDISTR{i}_L"), &[format!("CLK_HDISTR_FT0_{i}")])
                        .extra_wire(format!("HROUTE{i}_L"), &[format!("CLK_HROUTE_FT0_{i}")])
                }
            }
            bels.push(bel);
            bels.push(
                builder
                    .bel_virtual("VCC.GT")
                    .extra_wire("VCC", &["VCC_WIRE"]),
            );

            let mut xn = builder.xnode(kind, kind, xy).num_tiles(60).ref_single(
                int_xy.delta(0, 30),
                30,
                rclk_int,
            );
            for i in 0..60 {
                xn = xn
                    .ref_int(int_xy.delta(0, (i + i / 30) as i32), i)
                    .ref_single(
                        int_xy.delta(if is_l { -1 } else { 1 }, (i + i / 30) as i32),
                        i,
                        intf,
                    )
            }
            xn.bels(bels).extract();
        }
    }

    builder.build()
}
