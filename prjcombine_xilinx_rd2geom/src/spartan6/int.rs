use prjcombine_rawdump::{Coord, Part};
use prjcombine_xilinx_geom::int::{Dir, IntDb, WireKind};

use crate::intb::IntBuilder;

pub fn make_int_db(rd: &Part) -> IntDb {
    let mut builder = IntBuilder::new("spartan6", rd);

    builder.wire("PULLUP", WireKind::TiePullup, &["KEEP1_WIRE"]);
    builder.wire("GND", WireKind::Tie0, &["GND_WIRE"]);
    builder.wire("VCC", WireKind::Tie1, &["VCC_WIRE"]);

    for i in 0..16 {
        builder.wire(
            format!("GCLK{i}"),
            WireKind::ClkOut(i),
            &[format!("GCLK{i}"), format!("GCLK{i}_BRK")],
        );
    }

    for (lr, dir, dend) in [
        ("L", Dir::E, Some((0, Dir::S))),
        ("R", Dir::E, Some((3, Dir::N))),
        ("L", Dir::W, Some((3, Dir::N))),
        ("R", Dir::W, Some((0, Dir::S))),
        ("L", Dir::N, Some((0, Dir::S))),
        ("R", Dir::N, None),
        ("L", Dir::S, None),
        ("R", Dir::S, Some((3, Dir::N))),
    ] {
        for i in 0..4 {
            let b = builder.mux_out(format!("SNG.{dir}{lr}{i}.0"), &[format!("{dir}{lr}1B{i}")]);
            let e = builder.branch(
                b,
                dir,
                format!("SNG.{dir}{lr}{i}.1"),
                &[format!("{dir}{lr}1E{i}")],
            );
            if let Some((xi, dend)) = dend {
                if i == xi {
                    builder.branch(
                        e,
                        dend,
                        format!("SNG.{dir}{lr}{i}.2"),
                        &[format!("{dir}{lr}1E_{dend}{i}")],
                    );
                }
            }
        }
    }

    for (da, db, dend) in [
        (Dir::E, Dir::E, None),
        (Dir::W, Dir::W, Some((3, Dir::N))),
        (Dir::N, Dir::N, Some((0, Dir::S))),
        (Dir::N, Dir::E, Some((0, Dir::S))),
        (Dir::N, Dir::W, Some((0, Dir::S))),
        (Dir::S, Dir::S, Some((3, Dir::N))),
        (Dir::S, Dir::E, None),
        (Dir::S, Dir::W, Some((3, Dir::N))),
    ] {
        for i in 0..4 {
            let b = builder.mux_out(format!("DBL.{da}{db}{i}.0"), &[format!("{da}{db}2B{i}")]);
            let m = builder.branch(
                b,
                da,
                format!("DBL.{da}{db}{i}.1"),
                &[format!("{da}{db}2M{i}")],
            );
            let e = builder.branch(
                m,
                db,
                format!("DBL.{da}{db}{i}.2"),
                &[format!("{da}{db}2E{i}")],
            );
            if let Some((xi, dend)) = dend {
                if i == xi {
                    builder.branch(
                        e,
                        dend,
                        format!("DBL.{da}{db}{i}.3"),
                        &[format!("{da}{db}2E_{dend}{i}")],
                    );
                }
            }
        }
    }

    for (da, db, dend) in [
        (Dir::E, Dir::E, None),
        (Dir::W, Dir::W, Some((0, Dir::S))),
        (Dir::N, Dir::N, None),
        (Dir::N, Dir::E, None),
        (Dir::N, Dir::W, Some((0, Dir::S))),
        (Dir::S, Dir::S, Some((3, Dir::N))),
        (Dir::S, Dir::E, None),
        (Dir::S, Dir::W, Some((3, Dir::N))),
    ] {
        for i in 0..4 {
            let b = builder.mux_out(format!("QUAD.{da}{db}{i}.0"), &[format!("{da}{db}4B{i}")]);
            let a = builder.branch(
                b,
                da,
                format!("QUAD.{da}{db}{i}.1"),
                &[format!("{da}{db}4A{i}")],
            );
            let m = builder.branch(
                a,
                da,
                format!("QUAD.{da}{db}{i}.2"),
                &[format!("{da}{db}4M{i}")],
            );
            let c = builder.branch(
                m,
                db,
                format!("QUAD.{da}{db}{i}.3"),
                &[format!("{da}{db}4C{i}")],
            );
            let e = builder.branch(
                c,
                db,
                format!("QUAD.{da}{db}{i}.4"),
                &[format!("{da}{db}4E{i}")],
            );
            if let Some((xi, dend)) = dend {
                if i == xi {
                    builder.branch(
                        e,
                        dend,
                        format!("QUAD.{da}{db}{i}.5"),
                        &[format!("{da}{db}4E_{dend}{i}")],
                    );
                }
            }
        }
    }

    for i in 0..2 {
        builder.mux_out(
            format!("IMUX.GFAN{i}"),
            &[format!("GFAN{i}"), format!("INT_IOI_GFAN{i}")],
        );
    }
    for i in 0..2 {
        builder.mux_out(
            format!("IMUX.CLK{i}"),
            &[format!("CLK{i}"), format!("INT_TERM_CLK{i}")],
        );
    }
    for i in 0..2 {
        builder.mux_out(
            format!("IMUX.SR{i}"),
            &[format!("SR{i}"), format!("INT_TERM_SR{i}")],
        );
    }
    for i in 0..63 {
        let w = builder.mux_out(
            format!("IMUX.LOGICIN{i}"),
            &[format!("LOGICIN_B{i}"), format!("INT_TERM_LOGICIN_B{i}")],
        );
        let dir = match i {
            20 | 36 | 44 | 62 => Dir::S,
            21 | 28 | 52 | 60 => Dir::N,
            _ => continue,
        };
        let b = builder.buf(
            w,
            format!("IMUX.LOGICIN{i}.BOUNCE"),
            &[format!("LOGICIN{i}")],
        );
        builder.branch(
            b,
            dir,
            format!("IMUX.LOGICIN{i}.BOUNCE.{dir}"),
            &[&format!("LOGICIN_{dir}{i}")],
        );
    }
    builder.mux_out(&"IMUX.LOGICIN63".to_string(), &["FAN_B"]);

    for i in 0..24 {
        builder.logic_out(
            format!("OUT{i}"),
            &[format!("LOGICOUT{i}"), format!("INT_TERM_LOGICOUT{i}")],
        );
    }

    builder.extract_main_passes();

    builder.extract_node("INT", "INT", "INT", &[]);
    builder.extract_node("INT_BRK", "INT", "INT.BRK", &[]);
    builder.extract_node("INT_BRAM", "INT", "INT", &[]);
    builder.extract_node("INT_BRAM_BRK", "INT", "INT.BRK", &[]);
    builder.extract_node("INT_GCLK", "INT", "INT", &[]);
    builder.extract_node("INT_TERM", "INT", "INT.TERM", &[]);
    builder.extract_node("INT_TERM_BRK", "INT", "INT.TERM.BRK", &[]);
    builder.extract_node("IOI_INT", "IOI", "IOI", &[]);
    builder.extract_node("LIOI_INT", "IOI", "IOI", &[]);
    builder.extract_node("LIOI_INT_BRK", "IOI", "IOI.BRK", &[]);

    for tkn in [
        "CNR_TL_LTERM",
        "IOI_LTERM",
        "IOI_LTERM_LOWER_BOT",
        "IOI_LTERM_LOWER_TOP",
        "IOI_LTERM_UPPER_BOT",
        "IOI_LTERM_UPPER_TOP",
    ] {
        builder.extract_term_buf("TERM.W", Dir::W, tkn, "TERM.W", &[]);
    }
    builder.extract_term_buf("TERM.W", Dir::W, "INT_INTERFACE_LTERM", "TERM.W.INTF", &[]);

    for &term_xy in rd.tiles_by_kind_name("INT_LTERM") {
        let int_xy = builder.walk_to_int(term_xy, Dir::E).unwrap();
        // sigh.
        if int_xy.x == term_xy.x + 3 {
            continue;
        }
        builder.extract_term_buf_tile("TERM.W", Dir::W, term_xy, "TERM.W.INTF", int_xy, &[]);
    }
    for tkn in [
        "CNR_TL_RTERM",
        "IOI_RTERM",
        "IOI_RTERM_LOWER_BOT",
        "IOI_RTERM_LOWER_TOP",
        "IOI_RTERM_UPPER_BOT",
        "IOI_RTERM_UPPER_TOP",
    ] {
        builder.extract_term_buf("TERM.E", Dir::E, tkn, "TERM.E", &[]);
    }
    for tkn in ["INT_RTERM", "INT_INTERFACE_RTERM"] {
        builder.extract_term_buf("TERM.E", Dir::E, tkn, "TERM.E.INTF", &[]);
    }
    for tkn in [
        "CNR_BR_BTERM",
        "IOI_BTERM",
        "IOI_BTERM_BUFPLL",
        "CLB_INT_BTERM",
        "DSP_INT_BTERM",
        // NOTE: RAMB_BOT_BTERM is *not* a terminator — it's empty
    ] {
        builder.extract_term_buf("TERM.S", Dir::S, tkn, "TERM.S", &[]);
    }
    for tkn in [
        "CNR_TR_TTERM",
        "IOI_TTERM",
        "IOI_TTERM_BUFPLL",
        "DSP_INT_TTERM",
        "RAMB_TOP_TTERM",
    ] {
        builder.extract_term_buf("TERM.N", Dir::N, tkn, "TERM.N", &[]);
    }

    for (dir, tkn, naming) in [
        (Dir::E, "INT_INTERFACE", "INTF"),
        (Dir::E, "INT_INTERFACE_CARRY", "INTF"),
        (Dir::E, "INT_INTERFACE_REGC", "INTF.REGC"),
        (Dir::W, "INT_INTERFACE_LTERM", "INTF.LTERM"),
        (Dir::E, "INT_INTERFACE_RTERM", "INTF.RTERM"),
        (Dir::E, "LL", "INTF.CNR"),
        (Dir::E, "UL", "INTF.CNR"),
        (Dir::E, "LR_LOWER", "INTF.CNR"),
        (Dir::E, "LR_UPPER", "INTF.CNR"),
        (Dir::E, "UR_LOWER", "INTF.CNR"),
        (Dir::E, "UR_UPPER", "INTF.CNR"),
    ] {
        builder.extract_intf("INTF", dir, tkn, naming, true);
    }
    for tkn in ["INT_INTERFACE_IOI", "INT_INTERFACE_IOI_DCMBOT"] {
        builder.extract_intf("INTF.IOI", Dir::E, tkn, "INTF", true);
    }
    for tkn in ["LIOI", "LIOI_BRK", "RIOI", "RIOI_BRK"] {
        builder.extract_intf("INTF.IOI", Dir::E, tkn, "INTF.IOI", true);
    }

    for tkn in ["CLEXL", "CLEXM"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
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
                        .pins_name_only(&["CIN"])
                        .pin_name_only("COUT", 1),
                    builder.bel_xy("SLICE1", "SLICE", 1, 0),
                ],
                &[],
            );
        }
    }

    builder.build()
}
