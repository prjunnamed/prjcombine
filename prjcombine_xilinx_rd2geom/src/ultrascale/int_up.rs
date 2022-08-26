use prjcombine_rawdump::{Coord, Part};
use prjcombine_xilinx_geom::int::{Dir, IntDb, WireKind};

use enum_map::enum_map;

use crate::intb::IntBuilder;

pub fn make_int_db(rd: &Part) -> IntDb {
    let mut builder = IntBuilder::new("ultrascaleplus", rd);

    let d2n = enum_map!(
        Dir::N => 0,
        Dir::S => 1,
        Dir::E => 2,
        Dir::W => 3,
    );

    builder.wire("VCC", WireKind::Tie1, &["VCC_WIRE"]);

    for i in 0..16 {
        builder.wire(
            format!("GCLK{i}"),
            WireKind::ClkOut(i),
            &[format!("GCLK_B_0_{i}")],
        );
    }

    for (ih, h) in ["E", "W"].into_iter().enumerate() {
        for i in 0..96 {
            match i {
                0 | 2 => {
                    let w = builder.mux_out(
                        format!("SDQNODE.{h}.{i}"),
                        &[format!("SDQNODE_{h}_{i}_FT1")],
                    );
                    builder.branch(
                        w,
                        Dir::S,
                        format!("SDQNODE.{h}.{i}.S"),
                        &[format!("SDQNODE_{h}_BLS_{i}_FT0")],
                    );
                }
                91 | 93 | 95 => {
                    let w = builder.mux_out(
                        format!("SDQNODE.{h}.{i}"),
                        &[format!("SDQNODE_{h}_{i}_FT0")],
                    );
                    builder.branch(
                        w,
                        Dir::N,
                        format!("SDQNODE.{h}.{i}.N"),
                        &[format!("SDQNODE_{h}_BLN_{i}_FT1")],
                    );
                }
                _ => {
                    // TODO not the true permutation
                    let a = [
                        0, 11, 22, 33, 44, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 17,
                        18, 19, 20, 21, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 34, 35, 36, 37, 38,
                        39, 40, 41, 42, 43, 45, 46, 47,
                    ][i >> 1];
                    let aa = a + ih * 48;
                    let b = i & 1;
                    builder.mux_out(
                        format!("SDQNODE.{h}.{i}"),
                        &[format!("INT_NODE_SDQ_{aa}_INT_OUT{b}")],
                    );
                }
            }
        }
    }
    for (dir, name, l, ll, fts, ftn) in [
        (Dir::E, "SNG", 1, 1, false, false),
        (Dir::W, "SNG", 1, 1, false, true),
        (Dir::N, "SNG", 1, 1, false, false),
        (Dir::S, "SNG", 1, 1, false, false),
        (Dir::E, "DBL", 1, 2, false, false),
        (Dir::W, "DBL", 1, 2, true, false),
        (Dir::N, "DBL", 2, 2, false, false),
        (Dir::S, "DBL", 2, 2, false, false),
        (Dir::E, "QUAD", 2, 4, false, false),
        (Dir::W, "QUAD", 2, 4, false, false),
        (Dir::N, "QUAD", 4, 4, false, true),
        (Dir::S, "QUAD", 4, 4, true, false),
    ] {
        let ftd = d2n[!dir];
        for ew in ['E', 'W'] {
            for i in 0..8 {
                match (ll, dir, ew) {
                    (1, Dir::E, 'W') => {
                        let (a, b) = [
                            (60, 1),
                            (4, 0),
                            (61, 1),
                            (5, 0),
                            (62, 1),
                            (6, 0),
                            (63, 1),
                            (7, 0),
                        ][i];
                        builder.mux_out(
                            format!("{name}.{dir}.{ew}.{i}.0"),
                            &[format!("INT_INT_SDQ_{a}_INT_OUT{b}")],
                        );
                    }
                    (1, Dir::W, 'E') => {
                        if i == 7 {
                            let w = builder.mux_out(
                                format!("{name}.{dir}.{ew}.{i}.0"),
                                &[format!("{dir}{dir}{ll}_{ew}_{i}_FT0")],
                            );
                            builder.branch(
                                w,
                                Dir::N,
                                format!("{name}.{dir}.{ew}.{i}.{l}.N"),
                                &[format!("{dir}{dir}{ll}_{ew}_BLN_{i}_FT1")],
                            );
                        } else {
                            let (a, b) = [
                                (72, 0),
                                (32, 1),
                                (73, 0),
                                (33, 1),
                                (74, 0),
                                (34, 1),
                                (75, 0),
                            ][i];
                            builder.mux_out(
                                format!("{name}.{dir}.{ew}.{i}.0"),
                                &[format!("INT_INT_SDQ_{a}_INT_OUT{b}")],
                            );
                        }
                    }
                    _ => {
                        let mut w = builder.mux_out(
                            format!("{name}.{dir}.{ew}.{i}.0"),
                            &[format!("{dir}{dir}{ll}_{ew}_BEG{i}")],
                        );
                        for j in 1..l {
                            let nn = (b'A' + (j - 1)) as char;
                            w = builder.branch(
                                w,
                                dir,
                                format!("{name}.{dir}.{ew}.{i}.{j}"),
                                &[format!("{dir}{dir}{ll}_{ew}_{nn}_FT{ftd}_{i}")],
                            );
                        }
                        w = builder.branch(
                            w,
                            dir,
                            format!("{name}.{dir}.{ew}.{i}.{l}"),
                            &[format!("{dir}{dir}{ll}_{ew}_END{i}")],
                        );
                        if i == 0 && fts {
                            builder.branch(
                                w,
                                Dir::S,
                                format!("{name}.{dir}.{ew}.{i}.{l}.S"),
                                &[format!("{dir}{dir}{ll}_{ew}_BLS_{i}_FT0")],
                            );
                        }
                        if i == 7 && ftn {
                            builder.branch(
                                w,
                                Dir::N,
                                format!("{name}.{dir}.{ew}.{i}.{l}.N"),
                                &[format!("{dir}{dir}{ll}_{ew}_BLN_{i}_FT1")],
                            );
                        }
                    }
                }
            }
        }
    }

    for (dir, name, l, fts, ftn) in [
        (Dir::E, "LONG", 6, true, true),
        (Dir::W, "LONG", 6, false, false),
        (Dir::N, "LONG", 12, false, false),
        (Dir::S, "LONG", 12, false, false),
    ] {
        let ftd = d2n[!dir];
        for i in 0..8 {
            let mut w = builder.mux_out(
                format!("{name}.{dir}.{i}.0"),
                &[format!("{dir}{dir}12_BEG{i}")],
            );
            for j in 1..l {
                let nn = (b'A' + (j - 1)) as char;
                w = builder.branch(
                    w,
                    dir,
                    format!("{name}.{dir}.{i}.{j}"),
                    &[format!("{dir}{dir}12_{nn}_FT{ftd}_{i}")],
                );
            }
            w = builder.branch(
                w,
                dir,
                format!("{name}.{dir}.{i}.{l}"),
                &[format!("{dir}{dir}12_END{i}")],
            );
            if i == 0 && fts {
                builder.branch(
                    w,
                    Dir::S,
                    format!("{name}.{dir}.{i}.{l}.S"),
                    &[format!("{dir}{dir}12_BLS_{i}_FT0")],
                );
            }
            if i == 7 && ftn {
                builder.branch(
                    w,
                    Dir::N,
                    format!("{name}.{dir}.{i}.{l}.N"),
                    &[format!("{dir}{dir}12_BLN_{i}_FT1")],
                );
            }
        }
    }

    for i in 0..16 {
        for j in 0..2 {
            builder.mux_out(
                format!("INT_NODE_GLOBAL.{i}.{j}"),
                &[format!("INT_NODE_GLOBAL_{i}_INT_OUT{j}")],
            );
        }
    }
    for i in 0..8 {
        builder.mux_out(format!("IMUX.E.CTRL.{i}"), &[format!("CTRL_E{i}")]);
    }
    for i in 0..10 {
        builder.mux_out(format!("IMUX.W.CTRL.{i}"), &[format!("CTRL_W{i}")]);
    }

    for (ih, h) in ['E', 'W'].into_iter().enumerate() {
        for i in 0..64 {
            match i {
                1 | 3 | 5 | 9 => {
                    let w =
                        builder.mux_out(format!("INODE.{h}.{i}"), &[format!("INODE_{h}_{i}_FT1")]);
                    builder.branch(
                        w,
                        Dir::S,
                        format!("INODE.{h}.{i}.S"),
                        &[format!("INODE_{h}_BLS_{i}_FT0")],
                    );
                }
                54 | 58 | 60 | 62 => {
                    let w =
                        builder.mux_out(format!("INODE.{h}.{i}"), &[format!("INODE_{h}_{i}_FT0")]);
                    builder.branch(
                        w,
                        Dir::N,
                        format!("INODE.{h}.{i}.N"),
                        &[format!("INODE_{h}_BLN_{i}_FT1")],
                    );
                }
                _ => {
                    // TODO not the true permutation
                    let a = [
                        0, 11, 22, 30, 31, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 17,
                        18, 19, 20, 21, 23, 24, 25, 26, 27, 28, 29,
                    ][i >> 1];
                    let aa = a + ih * 32;
                    let b = i & 1;
                    let w = builder.mux_out(format!("INODE.{h}.{i}"), &[""]);
                    builder.extra_name_tile("INT", format!("INT_NODE_IMUX_{aa}_INT_OUT{b}"), w);
                }
            }
        }
    }

    for ew in ['E', 'W'] {
        for i in 0..16 {
            match i {
                0 | 2 => {
                    let w = builder.mux_out(
                        format!("IMUX.{ew}.BYP.{i}"),
                        &[format!("BOUNCE_{ew}_{i}_FT1")],
                    );
                    builder.branch(
                        w,
                        Dir::S,
                        format!("IMUX.{ew}.BYP.{i}.S"),
                        &[format!("BOUNCE_{ew}_BLS_{i}_FT0")],
                    );
                }
                13 | 15 => {
                    let w = builder.mux_out(
                        format!("IMUX.{ew}.BYP.{i}"),
                        &[format!("BOUNCE_{ew}_{i}_FT0")],
                    );
                    builder.branch(
                        w,
                        Dir::N,
                        format!("IMUX.{ew}.BYP.{i}.N"),
                        &[format!("BOUNCE_{ew}_BLN_{i}_FT1")],
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

    for i in 0..32 {
        builder.mux_out(
            format!("RCLK.IMUX.CE.{i}"),
            &[format!("CLK_LEAF_SITES_{i}_CE_INT")],
        );
    }
    builder.mux_out("RCLK.IMUX.ENSEL_PROG", &["CLK_LEAF_SITES_0_ENSEL_PROG"]);
    builder.mux_out("RCLK.IMUX.CLK_CASC_IN", &["CLK_LEAF_SITES_0_CLK_CASC_IN"]);
    for i in 0..2 {
        for j in 0..4 {
            builder.mux_out(
                format!("RCLK.IMUX.LEFT.{i}.{j}"),
                &[format!("INT_RCLK_TO_CLK_LEFT_{i}_{j}")],
            );
        }
    }
    for i in 0..2 {
        for j in 0..3 {
            builder.mux_out(
                format!("RCLK.IMUX.RIGHT.{i}.{j}"),
                &[format!("INT_RCLK_TO_CLK_RIGHT_{i}_{j}")],
            );
        }
    }
    for i in 0..2 {
        for j in 0..24 {
            let w = builder.mux_out(format!("RCLK.INODE.{i}.{j}"), &[""]);
            builder.extra_name_tile("RCLK_INT_L", format!("INT_NODE_IMUX_{j}_INT_OUT{i}"), w);
            builder.extra_name_tile("RCLK_INT_R", format!("INT_NODE_IMUX_{j}_INT_OUT{i}"), w);
        }
    }
    for i in 0..48 {
        let w = builder.wire(format!("RCLK.GND.{i}"), WireKind::Tie0, &[""]);
        builder.extra_name_tile("RCLK_INT_L", format!("GND_WIRE{i}"), w);
        builder.extra_name_tile("RCLK_INT_R", format!("GND_WIRE{i}"), w);
    }

    builder.extract_main_passes();

    builder.node_type("INT", "INT", "INT");

    builder.extract_term_conn("TERM.W", Dir::W, "INT_INTF_L_TERM_GT", &[]);
    builder.extract_term_conn("TERM.W", Dir::W, "INT_INTF_LEFT_TERM_PSS", &[]);
    builder.extract_term_conn("TERM.W", Dir::W, "INT_INTF_LEFT_TERM_IO_FT", &[]);
    builder.extract_term_conn("TERM.E", Dir::E, "INT_INTF_R_TERM_GT", &[]);
    builder.extract_term_conn("TERM.E", Dir::E, "INT_INTF_RIGHT_TERM_IO", &[]);
    builder.extract_term_conn("TERM.S", Dir::S, "INT_TERM_B", &[]);
    builder.extract_term_conn("TERM.S", Dir::S, "INT_TERM_P", &[]);
    builder.extract_term_conn("TERM.S", Dir::S, "INT_INT_TERM_H_FT", &[]);
    builder.extract_term_conn("TERM.N", Dir::N, "INT_TERM_T", &[]);

    for (dir, tkn) in [(Dir::W, "INT_INTF_L"), (Dir::E, "INT_INTF_R")] {
        builder.extract_intf(format!("INTF.{dir}"), dir, tkn, format!("INTF.{dir}"), true);
    }

    builder.extract_intf(
        "INTF.W.IO",
        Dir::W,
        "INT_INTF_LEFT_TERM_PSS",
        "INTF.PSS",
        true,
    );
    for (dir, tkn) in [
        (Dir::W, "INT_INTF_LEFT_TERM_IO_FT"),
        (Dir::W, "INT_INTF_L_CMT"),
        (Dir::W, "INT_INTF_L_IO"),
        (Dir::E, "INT_INTF_RIGHT_TERM_IO"),
    ] {
        builder.extract_intf(
            format!("INTF.{dir}.IO"),
            dir,
            tkn,
            format!("INTF.{dir}.IO"),
            true,
        );
    }

    for (dir, n, tkn) in [
        (Dir::W, "PCIE", "INT_INTF_L_PCIE4"),
        (Dir::E, "PCIE", "INT_INTF_R_PCIE4"),
        (Dir::W, "GT", "INT_INTF_L_TERM_GT"),
        (Dir::E, "GT", "INT_INTF_R_TERM_GT"),
    ] {
        builder.extract_intf(
            format!("INTF.{dir}.DELAY"),
            dir,
            tkn,
            format!("INTF.{dir}.{n}"),
            true,
        );
    }

    builder.extract_pass_simple("IO", Dir::W, "INT_IBRK_FSR2IO", &[]);

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
