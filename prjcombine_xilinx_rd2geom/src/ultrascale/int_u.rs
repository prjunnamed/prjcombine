use prjcombine_xilinx_geom::int::{Dir, IntDb, WireKind};
use prjcombine_xilinx_rawdump::{Coord, Part};

use crate::intb::IntBuilder;

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

    builder.build()
}
