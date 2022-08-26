use prjcombine_rawdump::{Coord, Part};
use prjcombine_xilinx_geom::int::{Dir, IntDb, WireKind};

use crate::intb::IntBuilder;

pub fn make_int_db(rd: &Part) -> IntDb {
    let mut builder = IntBuilder::new("virtex4", rd);

    builder.wire("PULLUP", WireKind::TiePullup, &["KEEP1_WIRE"]);
    builder.wire("GND", WireKind::Tie0, &["GND_WIRE"]);
    builder.wire("VCC", WireKind::Tie1, &["VCC_WIRE"]);

    for i in 0..8 {
        builder.wire(
            format!("GCLK{i}"),
            WireKind::ClkOut(i),
            &[format!("GCLK{i}")],
        );
    }
    for i in 0..2 {
        builder.wire(
            format!("RCLK{i}"),
            WireKind::ClkOut(8 + i),
            &[format!("RCLK{i}")],
        );
    }

    for (i, da1, da2, db) in [
        (0, Dir::S, None, None),
        (1, Dir::W, Some(Dir::S), None),
        (2, Dir::E, None, Some(Dir::S)),
        (3, Dir::S, Some(Dir::E), None),
        (4, Dir::S, None, None),
        (5, Dir::S, Some(Dir::W), None),
        (6, Dir::W, None, None),
        (7, Dir::E, Some(Dir::S), None),
        (8, Dir::E, Some(Dir::N), None),
        (9, Dir::W, None, None),
        (10, Dir::N, Some(Dir::W), None),
        (11, Dir::N, None, None),
        (12, Dir::N, Some(Dir::E), None),
        (13, Dir::E, None, Some(Dir::N)),
        (14, Dir::W, Some(Dir::N), None),
        (15, Dir::N, None, None),
    ] {
        let omux = builder.mux_out(format!("OMUX{i}"), &[format!("OMUX{i}")]);
        let omux_da1 = builder.branch(
            omux,
            da1,
            format!("OMUX{i}.{da1}"),
            &[format!("OMUX_{da1}{i}")],
        );
        if let Some(da2) = da2 {
            builder.branch(
                omux_da1,
                da2,
                format!("OMUX{i}.{da1}{da2}"),
                &[format!("OMUX_{da1}{da2}{i}")],
            );
        }
        if let Some(db) = db {
            builder.branch(
                omux,
                db,
                format!("OMUX{i}.{db}"),
                &[format!("OMUX_{db}{i}")],
            );
        }
        if i == 0 {
            builder.branch(omux, Dir::S, "OMUX0.S.ALT".to_string(), &["OUT_S"]);
        }
    }

    for dir in Dir::DIRS {
        for i in 0..10 {
            let beg = builder.mux_out(format!("DBL.{dir}{i}.0"), &[format!("{dir}2BEG{i}")]);
            let mid = builder.branch(
                beg,
                dir,
                format!("DBL.{dir}{i}.1"),
                &[format!("{dir}2MID{i}")],
            );
            let end = builder.branch(
                mid,
                dir,
                format!("DBL.{dir}{i}.2"),
                &[format!("{dir}2END{i}")],
            );
            if matches!(dir, Dir::E | Dir::S) && i < 2 {
                builder.branch(
                    end,
                    Dir::S,
                    format!("DBL.{dir}{i}.3"),
                    &[format!("{dir}2END_S{i}")],
                );
            }
            if matches!(dir, Dir::W | Dir::N) && i >= 8 {
                builder.branch(
                    end,
                    Dir::N,
                    format!("DBL.{dir}{i}.3"),
                    &[format!("{dir}2END_N{i}")],
                );
            }
        }
    }

    for dir in Dir::DIRS {
        for i in 0..10 {
            let mut last = builder.mux_out(format!("HEX.{dir}{i}.0"), &[format!("{dir}6BEG{i}")]);
            for (j, seg) in [
                (1, "A"),
                (2, "B"),
                (3, "MID"),
                (4, "C"),
                (5, "D"),
                (6, "END"),
            ] {
                last = builder.branch(
                    last,
                    dir,
                    format!("HEX.{dir}{i}.{j}"),
                    &[format!("{dir}6{seg}{i}")],
                );
            }
            if matches!(dir, Dir::E | Dir::S) && i < 2 {
                builder.branch(
                    last,
                    Dir::S,
                    format!("HEX.{dir}{i}.7"),
                    &[format!("{dir}6END_S{i}")],
                );
            }
            if matches!(dir, Dir::W | Dir::N) && i >= 8 {
                builder.branch(
                    last,
                    Dir::N,
                    format!("HEX.{dir}{i}.7"),
                    &[format!("{dir}6END_N{i}")],
                );
            }
        }
    }

    // The long wires.
    let mid = builder.wire("LH.12", WireKind::MultiOut, &["LH12"]);
    let mut prev = mid;
    for i in (0..12).rev() {
        prev = builder.multi_branch(prev, Dir::E, format!("LH.{i}"), &[format!("LH{i}")]);
    }
    let mut prev = mid;
    for i in 13..25 {
        prev = builder.multi_branch(prev, Dir::W, format!("LH.{i}"), &[format!("LH{i}")]);
    }
    let mid = builder.wire("LV.12", WireKind::MultiOut, &["LV12"]);
    let mut prev = mid;
    for i in (0..12).rev() {
        prev = builder.multi_branch(prev, Dir::N, format!("LV.{i}"), &[format!("LV{i}")]);
    }
    let mut prev = mid;
    for i in 13..25 {
        prev = builder.multi_branch(prev, Dir::S, format!("LV.{i}"), &[format!("LV{i}")]);
    }

    // The control inputs.
    for i in 0..4 {
        builder.mux_out(format!("IMUX.SR{i}"), &[format!("SR_B{i}")]);
    }
    for i in 0..4 {
        builder.mux_out(format!("IMUX.BOUNCE{i}"), &[format!("BOUNCE{i}")]);
    }
    for i in 0..4 {
        builder.mux_out(
            format!("IMUX.CLK{i}"),
            &[format!("CLK_B{i}"), format!("CLK_B{i}_DCM0")],
        );
    }
    for i in 0..4 {
        builder.mux_out(format!("IMUX.CE{i}"), &[format!("CE_B{i}")]);
    }

    // The data inputs.
    for i in 0..8 {
        let w = builder.mux_out(format!("IMUX.BYP{i}"), &[format!("BYP_INT_B{i}")]);
        builder.buf(
            w,
            format!("IMUX.BYP{i}.BOUNCE"),
            &[format!("BYP_BOUNCE{i}")],
        );
    }

    for i in 0..32 {
        builder.mux_out(format!("IMUX.IMUX{i}"), &[format!("IMUX_B{i}")]);
    }

    for i in 0..8 {
        builder.logic_out(format!("OUT.BEST{i}"), &[format!("BEST_LOGIC_OUTS{i}")]);
    }
    for i in 0..8 {
        builder.logic_out(format!("OUT.SEC{i}"), &[format!("SECONDARY_LOGIC_OUTS{i}")]);
    }
    for i in 0..8 {
        builder.logic_out(format!("OUT.HALF.BOT{i}"), &[format!("HALF_OMUX_BOT{i}")]);
    }
    for i in 0..8 {
        builder.logic_out(format!("OUT.HALF.TOP{i}"), &[format!("HALF_OMUX_TOP{i}")]);
    }

    builder.extract_main_passes();

    builder.node_type("INT", "INT", "INT");
    builder.node_type("INT_SO", "INT", "INT");
    builder.node_type("INT_SO_DCM0", "INT", "INT.DCM0");

    builder.extract_term("TERM.W", None, Dir::W, "L_TERM_INT", "TERM.W");
    builder.extract_term("TERM.E", None, Dir::E, "R_TERM_INT", "TERM.E");
    builder.extract_term("TERM.S", None, Dir::S, "B_TERM_INT", "TERM.S");
    builder.extract_term("TERM.N", None, Dir::N, "T_TERM_INT", "TERM.N");
    for tkn in ["MGT_AL_BOT", "MGT_AL_MID", "MGT_AL", "MGT_BL"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            for (i, delta) in [0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16]
                .into_iter()
                .enumerate()
            {
                let int_xy = Coord {
                    x: xy.x + 1,
                    y: xy.y - 9 + delta,
                };
                builder.extract_term_tile(
                    "TERM.W",
                    None,
                    Dir::W,
                    xy,
                    format!("TERM.W.MGT{i}"),
                    int_xy,
                );
            }
        }
    }
    for tkn in ["MGT_AR_BOT", "MGT_AR_MID", "MGT_AR", "MGT_BR"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            for (i, delta) in [0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16]
                .into_iter()
                .enumerate()
            {
                let int_xy = Coord {
                    x: xy.x - 1,
                    y: xy.y - 9 + delta,
                };
                builder.extract_term_tile(
                    "TERM.E",
                    None,
                    Dir::E,
                    xy,
                    format!("TERM.E.MGT{i}"),
                    int_xy,
                );
            }
        }
    }

    builder.extract_pass_simple("BRKH", Dir::S, "BRKH", &[]);
    builder.extract_pass_buf("CLB_BUFFER", Dir::W, "CLB_BUFFER", "PASS.CLB_BUFFER", &[]);

    builder.stub_out("PB_OMUX11_B5");
    builder.stub_out("PB_OMUX11_B6");

    for &pb_xy in rd.tiles_by_kind_name("PB") {
        let pt_xy = Coord {
            x: pb_xy.x,
            y: pb_xy.y + 18,
        };
        for (i, delta) in [
            0, 1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 13, 14, 15, 16, 17, 18, 19, 20, 22, 23, 24,
        ]
        .into_iter()
        .enumerate()
        {
            let int_w_xy = Coord {
                x: pb_xy.x - 1,
                y: pb_xy.y - 3 + delta,
            };
            let int_e_xy = Coord {
                x: pb_xy.x + 15,
                y: pb_xy.y - 3 + delta,
            };
            let naming_w = format!("TERM.PPC.W{i}");
            let naming_e = format!("TERM.PPC.E{i}");
            let xy = if i < 11 { pb_xy } else { pt_xy };
            builder.extract_pass_tile(
                "PPC.W",
                Dir::W,
                int_e_xy,
                Some(xy),
                Some(xy),
                Some(&naming_w),
                None,
                int_w_xy,
                &[],
            );
            builder.extract_pass_tile(
                "PPC.E",
                Dir::E,
                int_w_xy,
                Some(xy),
                Some(xy),
                Some(&naming_e),
                None,
                int_e_xy,
                &[],
            );
        }
        for (i, delta) in [1, 3, 5, 7, 9, 11, 13].into_iter().enumerate() {
            let int_s_xy = Coord {
                x: pb_xy.x + delta,
                y: pb_xy.y - 4,
            };
            let int_n_xy = Coord {
                x: pb_xy.x + delta,
                y: pb_xy.y + 22,
            };
            let ab = if i < 5 { 'A' } else { 'B' };
            let naming_s = format!("TERM.PPC.S{i}");
            let naming_n = format!("TERM.PPC.N{i}");
            builder.extract_pass_tile(
                format!("PPC{ab}.S"),
                Dir::S,
                int_n_xy,
                Some(pt_xy),
                Some(pb_xy),
                Some(&naming_s),
                None,
                int_s_xy,
                &[],
            );
            builder.extract_pass_tile(
                format!("PPC{ab}.N"),
                Dir::N,
                int_s_xy,
                Some(pb_xy),
                Some(pt_xy),
                Some(&naming_n),
                None,
                int_n_xy,
                &[],
            );
        }
    }

    for (tkn, n, height) in [
        ("BRAM", "BRAM", 4),
        ("DSP", "DSP", 4),
        ("CCM", "CCM", 4),
        ("DCM", "DCM", 4),
        ("DCM_BOT", "DCM", 4),
        ("SYS_MON", "SYSMON", 8),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            for i in 0..height {
                let int_xy = Coord {
                    x: xy.x - 1,
                    y: xy.y + i,
                };
                builder.extract_intf_tile("INTF", xy, int_xy, format!("{n}.{i}"), false);
            }
        }
    }
    for tkn in ["IOIS_LC", "IOIS_NC"] {
        builder.extract_intf("INTF", Dir::E, tkn, "IOIS", false);
    }
    for &xy in rd.tiles_by_kind_name("CFG_CENTER") {
        for i in 0..16 {
            let int_xy = Coord {
                x: xy.x - 1,
                y: if i < 8 {
                    xy.y - 8 + i
                } else {
                    xy.y + 1 + i - 8
                },
            };
            builder.extract_intf_tile("INTF", xy, int_xy, format!("CFG_CENTER.{i}"), false);
        }
    }
    for (dir, tkn) in [
        (Dir::W, "MGT_AL"),
        (Dir::W, "MGT_AL_BOT"),
        (Dir::W, "MGT_AL_MID"),
        (Dir::W, "MGT_BL"),
        (Dir::E, "MGT_AR"),
        (Dir::E, "MGT_AR_BOT"),
        (Dir::E, "MGT_AR_MID"),
        (Dir::E, "MGT_BR"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            for i in 0..16 {
                let int_xy = Coord {
                    x: if dir == Dir::E { xy.x - 1 } else { xy.x + 1 },
                    y: if i < 8 { xy.y - 9 + i } else { xy.y + i - 8 },
                };
                builder.extract_intf_tile("INTF", xy, int_xy, format!("MGT.{i}"), false);
            }
        }
    }
    for &pb_xy in rd.tiles_by_kind_name("PB") {
        let pt_xy = Coord {
            x: pb_xy.x,
            y: pb_xy.y + 18,
        };
        for (i, delta) in [
            0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 23, 24, 25, 26,
        ]
        .into_iter()
        .enumerate()
        {
            let int_w_xy = Coord {
                x: pb_xy.x - 1,
                y: pb_xy.y - 4 + delta,
            };
            let int_e_xy = Coord {
                x: pb_xy.x + 15,
                y: pb_xy.y - 4 + delta,
            };
            let xy = if i < 12 { pb_xy } else { pt_xy };
            builder.extract_intf_tile("INTF", xy, int_w_xy, format!("PPC.L{i}"), false);
            builder.extract_intf_tile("INTF", xy, int_e_xy, format!("PPC.R{i}"), false);
        }
        for (i, delta) in [1, 3, 5, 7, 9, 11, 13].into_iter().enumerate() {
            let int_s_xy = Coord {
                x: pb_xy.x + delta,
                y: pb_xy.y - 4,
            };
            let int_n_xy = Coord {
                x: pb_xy.x + delta,
                y: pb_xy.y + 22,
            };
            builder.extract_intf_tile("INTF", pb_xy, int_s_xy, format!("PPC.B{i}"), false);
            builder.extract_intf_tile("INTF", pt_xy, int_n_xy, format!("PPC.T{i}"), false);
        }
    }

    builder.build()
}
