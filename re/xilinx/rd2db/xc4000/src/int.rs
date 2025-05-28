use std::fmt::Write;

use prjcombine_interconnect::{
    db::{IntDb, TileCellId, TileClassWire, ConnectorWire, ConnectorClass, WireId, WireKind},
    dir::{Dir, DirMap},
};
use prjcombine_re_xilinx_naming::db::{NamingDb, TileClassNamingId};
use prjcombine_re_xilinx_rawdump::{Coord, Part};
use prjcombine_xc2000::bels::xc4000 as bels;
use unnamed_entity::EntityId;

use prjcombine_re_xilinx_rd2db_interconnect::IntBuilder;

const BOT_KINDS: [&str; 4] = ["BOT", "BOTS", "BOTSL", "BOTRR"];
const TOP_KINDS: [&str; 4] = ["TOP", "TOPS", "TOPSL", "TOPRR"];
const LEFT_KINDS: [&str; 8] = [
    "LEFT", "LEFTS", "LEFTT", "LEFTSB", "LEFTF", "LEFTSF", "LEFTF1", "LEFTSF1",
];
const RT_KINDS: [&str; 8] = ["RT", "RTS", "RTSB", "RTT", "RTF", "RTF1", "RTSF", "RTSF1"];

mod xc4000e_wires;

struct CnrTerms {
    term_ll_w: Vec<(WireId, WireId)>,
    term_lr_s: Vec<(WireId, WireId)>,
    term_ul_n: Vec<(WireId, WireId)>,
    term_ur_e: Vec<(WireId, WireId)>,
}

fn fill_tie_wires(builder: &mut IntBuilder) {
    let w = builder.wire(
        "GND",
        WireKind::Tie0,
        &["CENTER_TIE", "LR_TIE", "TVIBRK_TIE", "LHIBRK_TIE"],
    );
    for k in BOT_KINDS {
        builder.extra_name(format!("{k}_PULLDN"), w);
    }
    for k in RT_KINDS {
        builder.extra_name(format!("{k}_TIE"), w);
    }
}

fn fill_single_wires(builder: &mut IntBuilder) {
    for i in 0..8 {
        let ii = i + 1;
        let w = builder.wire(
            format!("SINGLE.H{i}"),
            WireKind::PipOut,
            &[format!("CENTER_H{ii}R")],
        );
        for k in BOT_KINDS.into_iter().chain(RT_KINDS).chain(["LR"]) {
            builder.extra_name(format!("{k}_H{ii}R"), w);
        }
        for k in LEFT_KINDS.into_iter().chain(["LL"]) {
            builder.extra_name(format!("{k}_H{ii}"), w);
        }
        let w = builder.pip_branch(
            w,
            Dir::E,
            format!("SINGLE.H{i}.E"),
            &[format!("CENTER_H{ii}")],
        );
        for k in BOT_KINDS.into_iter().chain(RT_KINDS).chain(["LR"]) {
            builder.extra_name(format!("{k}_H{ii}"), w);
        }
    }

    for i in 0..8 {
        let ii = i + 1;
        let w = builder.wire(
            format!("SINGLE.V{i}"),
            WireKind::PipOut,
            &[format!("CENTER_V{ii}")],
        );
        for k in BOT_KINDS
            .into_iter()
            .chain(TOP_KINDS)
            .chain(RT_KINDS)
            .chain(["LR", "UR"])
        {
            builder.extra_name(format!("{k}_V{ii}"), w);
        }
        let w = builder.pip_branch(
            w,
            Dir::S,
            format!("SINGLE.V{i}.S"),
            &[format!("CENTER_V{ii}T")],
        );
        for k in BOT_KINDS.into_iter().chain(RT_KINDS).chain(["LR"]) {
            builder.extra_name(format!("{k}_V{ii}T"), w);
        }
    }
}

fn fill_double_wires(builder: &mut IntBuilder) {
    for (dir, hv, rt0, rt2) in [(Dir::E, 'H', "R", ""), (Dir::S, 'V', "", "T")] {
        for i in 0..2 {
            let ii = [2, 3][i];
            let w = builder.wire(
                format!("DOUBLE.{hv}{i}.0"),
                WireKind::PipOut,
                &[format!("CENTER_D{hv}{ii}{rt0}")],
            );
            for k in BOT_KINDS.into_iter().chain(RT_KINDS).chain(["LR"]) {
                builder.extra_name(format!("{k}_D{hv}{ii}{rt0}"), w);
            }
            if hv == 'H' {
                for k in LEFT_KINDS.into_iter().chain(["LL"]) {
                    builder.extra_name(format!("{k}_D{hv}{ii}", ii = [1, 4][i]), w);
                }
            }
            if hv == 'V' {
                for k in TOP_KINDS.into_iter().chain(["UR"]) {
                    builder.extra_name(format!("{k}_D{hv}{ii}"), w);
                }
            }
            let ii = [1, 4][i];
            let w = builder.pip_branch(
                w,
                dir,
                format!("DOUBLE.{hv}{i}.1"),
                &[format!("CENTER_D{hv}{ii}")],
            );
            for k in BOT_KINDS.into_iter().chain(RT_KINDS).chain(["LR"]) {
                builder.extra_name(format!("{k}_D{hv}{ii}"), w);
            }
            if hv == 'V' {
                for k in TOP_KINDS.into_iter().chain(["UR"]) {
                    builder.extra_name(format!("{k}_D{hv}{ii}"), w);
                }
            } else {
                for k in LEFT_KINDS.into_iter().chain(["LL"]) {
                    builder.extra_name(format!("{k}_D{hv}{ii}", ii = [2, 3][i]), w);
                }
            }
            let ii = [2, 3][i];
            let w = builder.pip_branch(
                w,
                dir,
                format!("DOUBLE.{hv}{i}.2"),
                &[format!("CENTER_D{hv}{ii}{rt2}")],
            );
            for k in BOT_KINDS.into_iter().chain(RT_KINDS).chain(["LR"]) {
                builder.extra_name(format!("{k}_D{hv}{ii}{rt2}"), w);
            }
        }
    }
}

fn fill_io_double_wires(builder: &mut IntBuilder, cnr_terms: &mut CnrTerms) {
    let bdir = DirMap::from_fn(|dir| match dir {
        Dir::S => Dir::W,
        Dir::E => Dir::S,
        Dir::N => Dir::E,
        Dir::W => Dir::N,
    });

    for i in 0..4 {
        let mut wires = DirMap::from_fn(|_| vec![]);

        for j in 0..3 {
            for dir in Dir::DIRS {
                wires[dir].push(builder.wire(
                    format!("IO.DOUBLE.{i}.{dir}.{j}"),
                    WireKind::PipBranch(builder.term_slots[bdir[dir]]),
                    &[""],
                ));
            }
        }

        for j in 0..2 {
            for dir in Dir::DIRS {
                builder.conn_branch(wires[dir][j], !bdir[dir], wires[dir][j + 1]);
            }
            cnr_terms
                .term_ul_n
                .push((wires[Dir::W][j], wires[Dir::N][j + 1]));
        }
        cnr_terms
            .term_ll_w
            .push((wires[Dir::S][1], wires[Dir::W][1]));

        cnr_terms
            .term_ur_e
            .push((wires[Dir::N][1], wires[Dir::E][1]));

        let ia = i * 2 + 1;
        let ib = i * 2 + 2;
        for k in BOT_KINDS {
            builder.extra_name(format!("{k}_BDH{ia}"), wires[Dir::S][0]);
            builder.extra_name(format!("{k}_BDH{ib}"), wires[Dir::S][1]);
            builder.extra_name(format!("{k}_BDH{ia}L"), wires[Dir::S][2]);
        }
        for k in TOP_KINDS {
            builder.extra_name(format!("{k}_DH{ia}L"), wires[Dir::N][0]);
            builder.extra_name(format!("{k}_DH{ib}"), wires[Dir::N][1]);
            builder.extra_name(format!("{k}_DH{ia}"), wires[Dir::N][2]);
        }
        for k in LEFT_KINDS {
            builder.extra_name(format!("{k}_LDV{ia}"), wires[Dir::W][0]);
            builder.extra_name(format!("{k}_LDV{ib}"), wires[Dir::W][1]);
            builder.extra_name(format!("{k}_LDV{ia}T"), wires[Dir::W][2]);
        }
        for k in RT_KINDS {
            builder.extra_name(format!("{k}_RDV{ia}T"), wires[Dir::E][0]);
            builder.extra_name(format!("{k}_RDV{ib}"), wires[Dir::E][1]);
            builder.extra_name(format!("{k}_RDV{ia}"), wires[Dir::E][2]);
        }
        builder.extra_name(format!("LL_D{ib}B"), wires[Dir::S][0]);
        builder.extra_name(format!("LL_D{ia}"), wires[Dir::W][1]);
        builder.extra_name(format!("LL_D{ib}"), wires[Dir::W][2]);
        builder.extra_name(format!("UL_D{ia}"), wires[Dir::N][1]);
        builder.extra_name(format!("UL_D{ib}"), wires[Dir::N][2]);
        builder.extra_name(format!("LR_RDV{ib}"), wires[Dir::E][0]);
        builder.extra_name(format!("LR_RDV{ia}"), wires[Dir::E][1]);
        builder.extra_name(format!("LR_BDH{ib}"), wires[Dir::S][1]);
        builder.extra_name(format!("LR_BDH{ia}"), wires[Dir::S][2]);
        builder.extra_name(format!("UR_D{ia}L"), wires[Dir::N][0]);
        builder.extra_name(format!("UR_D{ib}"), wires[Dir::E][1]);
        builder.extra_name(format!("UR_D{ia}"), wires[Dir::E][2]);
    }

    for (i, n) in ["DMUX_OUTER", "DMUX_INNER"].into_iter().enumerate() {
        let w = builder.mux_out(format!("IO.DBUF.H{i}"), &[format!("LR_B{n}")]);
        for k in BOT_KINDS.into_iter().chain(TOP_KINDS).chain(["UR"]) {
            builder.extra_name(format!("{k}_{n}"), w);
        }
    }
    for (i, n) in ["DMUX_OUTER", "DMUX_INNER"].into_iter().enumerate() {
        let w = builder.mux_out(format!("IO.DBUF.V{i}"), &[format!("LR_R{n}")]);
        for k in LEFT_KINDS.into_iter().chain(RT_KINDS).chain(["LL"]) {
            builder.extra_name(format!("{k}_{n}"), w);
        }
    }
}

fn fill_quad_wires(builder: &mut IntBuilder) {
    if matches!(&*builder.rd.family, "xc4000e" | "spartanxl") {
        return;
    }

    for (dir, hv, rt0, rt4) in [(Dir::E, 'H', "R", ""), (Dir::S, 'V', "", "T")] {
        for i in 0..3 {
            let ii = 4 * i + 4;
            let mut w = builder.wire(
                format!("QUAD.{hv}{i}.0"),
                WireKind::PipOut,
                &[format!("CENTER_Q{hv}{ii}{rt0}")],
            );
            for k in BOT_KINDS
                .into_iter()
                .chain(TOP_KINDS)
                .chain(RT_KINDS)
                .chain(["LR", "UR"])
            {
                builder.extra_name(format!("{k}_Q{hv}{ii}{rt0}"), w);
            }
            for k in LEFT_KINDS.into_iter().chain(["LL"]) {
                builder.extra_name(format!("{k}_Q{hv}{ii}"), w);
            }
            for j in 1..4 {
                let ii = if hv == 'H' { 4 * i + 4 - j } else { 4 * i + j };
                w = builder.pip_branch(
                    w,
                    dir,
                    format!("QUAD.{hv}{i}.{j}"),
                    &[format!("CENTER_Q{hv}{ii}")],
                );
                for k in BOT_KINDS
                    .into_iter()
                    .chain(TOP_KINDS)
                    .chain(LEFT_KINDS)
                    .chain(RT_KINDS)
                    .chain(["LL", "LR", "UR"])
                {
                    builder.extra_name(format!("{k}_Q{hv}{ii}"), w);
                }
            }
            w = builder.pip_branch(
                w,
                dir,
                format!("QUAD.{hv}{i}.4"),
                &[format!("CENTER_Q{hv}{ii}{rt4}")],
            );
            for k in BOT_KINDS
                .into_iter()
                .chain(TOP_KINDS)
                .chain(RT_KINDS)
                .chain(["LR", "UR"])
            {
                builder.extra_name(format!("{k}_Q{hv}{ii}{rt4}"), w);
            }
        }
    }

    for i in 0..3 {
        let ii = i * 4 + 4;
        let w = builder.mux_out(format!("QBUF.{i}"), &[format!("CENTER_QBUF{ii}")]);
        for k in BOT_KINDS.into_iter().chain(RT_KINDS).chain(["LR"]) {
            builder.extra_name(format!("{k}_QBUF{ii}"), w);
        }
    }
}

fn fill_octal_wires(builder: &mut IntBuilder) {
    if builder.rd.family != "xc4000xv" {
        return;
    }

    let mut w = builder.wire(
        "OCTAL.H.0",
        WireKind::PipOut,
        &["VHIBRK_OH1R", "LHIBRK_OH8"],
    );
    for j in 1..8 {
        let ii = 9 - j;
        w = builder.pip_branch(
            w,
            Dir::E,
            format!("OCTAL.H.{j}"),
            &[
                format!("VHIBRK_OH{ii}"),
                format!("RHIBRK_OH{ii}"),
                format!("LHIBRK_OH{ii}", ii = ii - 1),
            ],
        );
    }
    builder.pip_branch(w, Dir::E, "OCTAL.H.8", &["VHIBRK_OH1", "RHIBRK_OH1"]);

    let mut w = builder.wire(
        "OCTAL.V.0",
        WireKind::PipOut,
        &["VHIBRK_OV8B", "TVIBRK_OV8"],
    );
    for j in 1..8 {
        w = builder.pip_branch(
            w,
            Dir::S,
            format!("OCTAL.V.{j}"),
            &[
                format!("VHIBRK_OV{j}"),
                format!("TVIBRK_OV{j}"),
                format!("BVIBRK_OV{ii}", ii = if j == 1 { 8 } else { j - 1 }),
            ],
        );
    }
    builder.pip_branch(w, Dir::S, "OCTAL.V.8", &["VHIBRK_OV8T", "BVIBRK_OV7"]);
}

fn fill_io_octal_wires(builder: &mut IntBuilder, cnr_terms: &mut CnrTerms) {
    if matches!(&*builder.rd.family, "xc4000e" | "spartanxl") {
        return;
    }

    let mut wires = DirMap::from_fn(|_| vec![]);

    let bdir = DirMap::from_fn(|dir| match dir {
        Dir::S => Dir::W,
        Dir::E => Dir::S,
        Dir::N => Dir::E,
        Dir::W => Dir::N,
    });

    for i in 0..9 {
        for dir in Dir::DIRS {
            wires[dir].push(builder.wire(
                format!("IO.OCTAL.{dir}.{i}"),
                WireKind::PipBranch(builder.term_slots[bdir[dir]]),
                &[""],
            ));
        }
    }

    for i in 0..8 {
        for dir in Dir::DIRS {
            builder.conn_branch(wires[dir][i], !bdir[dir], wires[dir][i + 1]);
        }
        cnr_terms
            .term_ll_w
            .push((wires[Dir::S][i], wires[Dir::W][i + 1]));
        cnr_terms
            .term_lr_s
            .push((wires[Dir::E][i], wires[Dir::S][i + 1]));
        cnr_terms
            .term_ul_n
            .push((wires[Dir::W][i], wires[Dir::N][i + 1]));
        cnr_terms
            .term_ur_e
            .push((wires[Dir::N][i], wires[Dir::E][i + 1]));
    }

    for k in BOT_KINDS {
        builder.extra_name(format!("{k}_OH8R"), wires[Dir::S][0]);
        for i in 0..7 {
            builder.extra_name(format!("{k}_OH{ii}", ii = 7 - i), wires[Dir::S][1 + i]);
        }
        builder.extra_name(format!("{k}_OH8"), wires[Dir::S][8]);
    }
    for k in TOP_KINDS {
        builder.extra_name(format!("{k}_OH8"), wires[Dir::N][0]);
        for i in 1..8 {
            builder.extra_name(format!("{k}_OH{i}"), wires[Dir::N][i]);
        }
        builder.extra_name(format!("{k}_OH8R"), wires[Dir::N][8]);
    }
    for k in LEFT_KINDS {
        builder.extra_name(format!("{k}_OV8"), wires[Dir::W][0]);
        for i in 1..8 {
            builder.extra_name(format!("{k}_OV{i}"), wires[Dir::W][i]);
        }
        builder.extra_name(format!("{k}_OV8T"), wires[Dir::W][8]);
    }

    for k in RT_KINDS {
        builder.extra_name(format!("{k}_OV8T"), wires[Dir::E][0]);
        for i in 0..7 {
            builder.extra_name(format!("{k}_OV{ii}", ii = 7 - i), wires[Dir::E][1 + i]);
        }
        builder.extra_name(format!("{k}_OV8"), wires[Dir::E][8]);
    }
    for i in 1..8 {
        builder.extra_name(format!("LR_O{i}"), wires[Dir::E][i]);
    }
    builder.extra_name("LR_O8T", wires[Dir::E][0]);
    for i in 1..8 {
        builder.extra_name(format!("UR_O{i}"), wires[Dir::N][i]);
    }
    builder.extra_name("UR_O8", wires[Dir::N][0]);
    for i in 0..7 {
        builder.extra_name(format!("LL_O{ii}", ii = 7 - i), wires[Dir::S][i]);
    }
    builder.extra_name("LL_O8", wires[Dir::S][7]);
    for i in 1..8 {
        builder.extra_name(format!("UL_O{i}"), wires[Dir::W][i]);
    }
    builder.extra_name("UL_O8", wires[Dir::W][0]);
}

fn fill_long_wires(builder: &mut IntBuilder) {
    for i in 0..6 {
        let ii = i + 1;
        let w = builder.wire(
            format!("LONG.H{i}"),
            WireKind::MultiBranch(builder.term_slots[Dir::W]),
            &[format!("CENTER_HLL{ii}")],
        );
        builder.conn_branch(w, Dir::E, w);
        for k in BOT_KINDS
            .into_iter()
            .chain(TOP_KINDS)
            .chain(LEFT_KINDS)
            .chain(RT_KINDS)
            .chain(["LL", "UL", "LR", "UR"])
        {
            builder.extra_name(format!("{k}_HLL{ii}"), w);
        }
        if matches!(&*builder.rd.family, "xc4000xla" | "xc4000xv" | "spartanxl")
            && matches!(i, 2 | 3)
        {
            let w = builder.buf(
                w,
                format!("LONG.H{i}.BUF"),
                &[format!("CENTER_HLL{ii}_LOC")],
            );
            for k in LEFT_KINDS.into_iter().chain(RT_KINDS) {
                builder.extra_name(format!("{k}_HLL{ii}_LOC"), w);
            }
        }
    }
    let nvll = if matches!(&*builder.rd.family, "xc4000e" | "spartanxl") {
        6
    } else {
        10
    };
    for i in 0..nvll {
        let ii = i + 1;
        let w = builder.wire(
            format!("LONG.V{i}"),
            WireKind::MultiBranch(builder.term_slots[Dir::S]),
            &[format!("CENTER_VLL{ii}")],
        );
        builder.conn_branch(w, Dir::N, w);
        for k in BOT_KINDS
            .into_iter()
            .chain(TOP_KINDS)
            .chain(RT_KINDS)
            .chain(["LR", "UR"])
        {
            builder.extra_name(format!("{k}_VLL{ii}"), w);
        }
        if matches!(i, 7 | 9) {
            let w = builder.wire(format!("LONG.V{i}.EXCL"), WireKind::PipOut, &[""]);
            for n in [
                format!("HVBRK_VLL{ii}_EXCL"),
                format!("HVBRK_VLL{ii}T_EXCL"),
                format!("RHVBRK_VLL{ii}_EXCL"),
                format!("RHVBRK_VLL{ii}B_EXCL"),
                format!("RVRBRK_VLL{ii}_EXCL"),
                format!("RVRBRK_VLL{ii}B_EXCL"),
            ] {
                builder.extra_name_sub(n, 1, w);
            }
        }
    }
    for i in 0..4 {
        let ii = i + 1;
        let w = builder.wire(
            format!("LONG.IO.H{i}"),
            WireKind::MultiBranch(builder.term_slots[Dir::W]),
            &[""],
        );
        builder.conn_branch(w, Dir::E, w);
        for k in BOT_KINDS.into_iter().chain(["LL", "LR"]) {
            builder.extra_name(format!("{k}_BHLL{ii}"), w);
        }
        for k in TOP_KINDS.into_iter().chain(["UL", "UR"]) {
            builder.extra_name(format!("{k}_THLL{ii}"), w);
        }
        if !matches!(&*builder.rd.family, "xc4000e" | "spartanxl") {
            let w = builder.wire(format!("LONG.IO.H{i}.EXCL"), WireKind::PipOut, &[""]);
            for k in BOT_KINDS.into_iter().chain(["LR"]) {
                builder.extra_name(format!("{k}_BHLL{ii}_EXCL"), w);
            }
            for k in TOP_KINDS.into_iter().chain(["UR"]) {
                builder.extra_name(format!("{k}_THLL{ii}_EXCL"), w);
            }
        }
    }
    for i in 0..4 {
        let ii = i + 1;
        let w = builder.wire(
            format!("LONG.IO.V{i}"),
            WireKind::MultiBranch(builder.term_slots[Dir::S]),
            &[""],
        );
        builder.conn_branch(w, Dir::N, w);
        for k in LEFT_KINDS.into_iter().chain(["LL", "UL"]) {
            builder.extra_name(format!("{k}_LVLL{ii}"), w);
        }
        for k in RT_KINDS.into_iter().chain(["LR", "UR"]) {
            builder.extra_name(format!("{k}_RVLL{ii}"), w);
        }
    }
}

fn fill_dec_wires(builder: &mut IntBuilder) {
    if builder.rd.family == "spartanxl" {
        return;
    }
    for i in 0..4 {
        let ii = i + 1;
        let tii = 4 - i;
        let w = builder.wire(
            format!("DEC.H{i}"),
            WireKind::MultiBranch(builder.term_slots[Dir::W]),
            &[
                format!("LL_BTX{ii}"),
                format!("LR_BTX{ii}"),
                format!("UL_TTX{tii}"),
                format!("UR_TTX{tii}"),
            ],
        );
        builder.conn_branch(w, Dir::E, w);
        for k in BOT_KINDS {
            builder.extra_name(format!("{k}_TX{ii}"), w);
        }
        for k in TOP_KINDS {
            builder.extra_name(format!("{k}_TTX{tii}"), w);
        }
    }
    for i in 0..4 {
        let ii = 4 - i;
        let lii = i + 1;
        let w = builder.wire(
            format!("DEC.V{i}"),
            WireKind::MultiBranch(builder.term_slots[Dir::S]),
            &[
                format!("LL_LTX{lii}"),
                format!("UL_LTX{lii}"),
                format!("LR_RTX{ii}"),
                format!("UR_RTX{ii}"),
            ],
        );
        builder.conn_branch(w, Dir::N, w);
        for k in LEFT_KINDS {
            builder.extra_name(format!("{k}_LTX{lii}"), w);
        }

        for k in RT_KINDS {
            builder.extra_name(format!("{k}_RTX{ii}"), w);
        }
    }
}

fn fill_clk_wires(builder: &mut IntBuilder) {
    let ngclk = if matches!(&*builder.rd.family, "xc4000e" | "spartanxl") {
        4
    } else {
        8
    };
    for i in 0..ngclk {
        let ii = i + 1;
        let w = builder.wire(
            format!("GCLK{i}"),
            WireKind::MultiBranch(builder.term_slots[Dir::S]),
            &[format!("CENTER_K{ii}")],
        );
        builder.conn_branch(w, Dir::N, w);
        for k in BOT_KINDS
            .into_iter()
            .chain(TOP_KINDS)
            .chain(LEFT_KINDS)
            .chain(RT_KINDS)
            .chain(["LL", "UL", "LR", "UR"])
        {
            builder.extra_name(format!("{k}_K{ii}"), w);
        }
    }

    if !matches!(&*builder.rd.family, "xc4000e" | "spartanxl") {
        let w = builder.wire(
            "VCLK",
            WireKind::MultiBranch(builder.term_slots[Dir::S]),
            &["CENTER_KX"],
        );
        builder.conn_branch(w, Dir::N, w);
        for k in BOT_KINDS
            .into_iter()
            .chain(TOP_KINDS)
            .chain(RT_KINDS)
            .chain(["LL", "LR", "UR"])
        {
            builder.extra_name(format!("{k}_KX"), w);
        }

        let w = builder.wire(
            "ECLK.V",
            WireKind::MultiBranch(builder.term_slots[Dir::S]),
            &["LL_KX", "UL_KX", "LR_LRKX", "UR_URKX"],
        );
        builder.conn_branch(w, Dir::N, w);
        for k in RT_KINDS {
            builder.extra_name(format!("{k}_R_KX"), w);
        }
        for k in LEFT_KINDS {
            builder.extra_name(format!("{k}_L_KX"), w);
        }

        let w = builder.wire(
            "ECLK.H",
            WireKind::MultiBranch(builder.term_slots[Dir::W]),
            &["LR_FCLK", "UR_FCLK", "LL_FCLK", "UL_FCLK"],
        );
        builder.conn_branch(w, Dir::E, w);
        for k in BOT_KINDS.into_iter().chain(TOP_KINDS) {
            builder.extra_name(format!("{k}_FCLK"), w);
        }

        let w = builder.wire(
            "BUFGE.H",
            WireKind::MultiBranch(builder.term_slots[Dir::W]),
            &[
                "LR_BUFGE_4_L",
                "UR_BUFGE_7_L",
                "LL_BUFGE_3_R",
                "UL_BUFGE_8_R",
            ],
        );
        builder.conn_branch(w, Dir::E, w);
        for k in BOT_KINDS {
            builder.extra_name(format!("{k}_BUFGE_3_4"), w);
        }
        for k in TOP_KINDS {
            builder.extra_name(format!("{k}_BUFGE_7_8"), w);
        }

        let w = builder.wire(
            "BUFGE.V0",
            WireKind::MultiBranch(builder.term_slots[Dir::S]),
            &[
                "LR_BUFGE_5_6",
                "LL_BUFGE_1_2",
                "LHVBRK_BUFGE_2",
                "LVLBRK_BUFGE_2",
                "RHVBRK_BUFGE_5",
                "RVRBRK_BUFGE_5",
            ],
        );
        builder.conn_branch(w, Dir::N, w);
        let w = builder.wire(
            "BUFGE.V1",
            WireKind::MultiBranch(builder.term_slots[Dir::S]),
            &[
                "UR_BUFGE_5_6",
                "UL_BUFGE_1_2",
                "LHVBRK_BUFGE_1",
                "LVLBRK_BUFGE_1",
                "RHVBRK_BUFGE_6",
                "RVRBRK_BUFGE_6",
            ],
        );
        builder.conn_branch(w, Dir::N, w);

        for i in 0..8 {
            let w = builder.wire(
                format!("BUFGLS.H{i}"),
                WireKind::MultiBranch(builder.term_slots[Dir::W]),
                &[""],
            );
            builder.conn_branch(w, Dir::E, w);
            let ii = i + 1;
            for n in [
                format!("HVBRK_BUFGLS_{ii}"),
                format!("LHVBRK_BUFGLS_{ii}"),
                format!("LVLBRK_BUFGLS_{ii}"),
                format!("RHVBRK_BUFGLS_{ii}"),
                format!("RVRBRK_BUFGLS_{ii}"),
            ] {
                builder.extra_name_sub(n, 1, w);
            }
        }
    }
}

fn fill_imux_wires(builder: &mut IntBuilder) -> (Vec<WireId>, Vec<TileClassWire>) {
    let mut imux_wires = vec![];
    let mut imux_nw = vec![];
    for (pin, opin) in [("F1", "O_2"), ("G1", "O_1"), ("C1", "TXIN2")] {
        let w = builder.mux_out(format!("IMUX.CLB.{pin}"), &[format!("CENTER_{pin}")]);
        imux_wires.push(w);
        imux_nw.push((TileCellId::from_idx(0), w));
        for &k in &RT_KINDS {
            builder.extra_name(format!("{k}_{opin}"), w);
        }
    }
    for (pin, opin) in [("F2", "O_1"), ("G2", "O_2"), ("C2", "TXIN2")] {
        let w = builder.mux_out(format!("IMUX.CLB.{pin}"), &[format!("CENTER_{pin}T")]);
        let ww = builder.branch(
            w,
            Dir::N,
            format!("IMUX.CLB.{pin}.N"),
            &[format!("CENTER_{pin}")],
        );
        imux_wires.push(w);
        imux_nw.push((TileCellId::from_idx(0), w));
        imux_wires.push(ww);
        for k in LEFT_KINDS {
            builder.extra_name(format!("{k}_{pin}R"), w);
        }
        builder.extra_name(format!("LL_{pin}"), w);
        for k in TOP_KINDS {
            builder.extra_name(format!("{k}_{opin}"), ww);
        }
        for k in BOT_KINDS {
            builder.extra_name(format!("{k}_{pin}T"), w);
        }
    }
    for (pin, opin) in [("F3", "O_2"), ("G3", "O_1"), ("C3", "TXIN2")] {
        let w = builder.mux_out(format!("IMUX.CLB.{pin}"), &[format!("CENTER_{pin}L")]);
        let ww = builder.branch(
            w,
            Dir::W,
            format!("IMUX.CLB.{pin}.W"),
            &[format!("CENTER_{pin}")],
        );
        imux_wires.push(w);
        imux_nw.push((TileCellId::from_idx(0), w));
        imux_wires.push(ww);
        for &k in &RT_KINDS {
            builder.extra_name(format!("{k}_{pin}L"), w);
        }
        for &k in &LEFT_KINDS {
            builder.extra_name(format!("{k}_{opin}"), ww);
        }
    }
    for (pin, opin, xname) in [
        ("F4", "O_1", "LR_HZ1"),
        ("G4", "O_2", "LR_HZ3"),
        ("C4", "TXIN2", "LR_HZ2"),
    ] {
        let w = builder.mux_out(format!("IMUX.CLB.{pin}"), &[format!("CENTER_{pin}")]);
        imux_wires.push(w);
        imux_nw.push((TileCellId::from_idx(0), w));
        for k in RT_KINDS {
            builder.extra_name(format!("{k}_{pin}"), w);
        }
        builder.extra_name(xname, w);
        for k in BOT_KINDS {
            builder.extra_name(format!("{k}_{opin}"), w);
        }
    }
    {
        let w = builder.mux_out("IMUX.CLB.K", &["CENTER_K"]);
        imux_wires.push(w);
        imux_nw.push((TileCellId::from_idx(0), w));
    }

    for i in 0..2 {
        let ii = i + 3;
        for pin in ["I", "TS"] {
            let w = builder.mux_out(
                format!("IMUX.TBUF{i}.{pin}"),
                &[format!("CENTER_TBUF{ii}{pin}")],
            );
            for k in LEFT_KINDS.into_iter().chain(RT_KINDS) {
                builder.extra_name(format!("{k}_TBUF{ii}{pin}"), w);
            }
            imux_wires.push(w);
            imux_nw.push((TileCellId::from_idx(0), w));
        }
    }

    for i in 0..2 {
        for pin in ["O1", "OK", "IK", "TS"] {
            let apin = if pin == "O1" { "CE" } else { pin };
            let w = builder.mux_out(format!("IMUX.IOB{i}.{pin}"), &[""]);
            for k in BOT_KINDS {
                let ii = if pin == "TS" { [2, 1][i] } else { i + 1 };
                builder.extra_name(format!("{k}_{apin}_{ii}"), w);
            }
            for k in TOP_KINDS.into_iter().chain(LEFT_KINDS).chain(RT_KINDS) {
                let ii = i + 1;
                builder.extra_name(format!("{k}_{apin}_{ii}"), w);
            }

            match (i, pin) {
                (1, "O1") => builder.extra_name("LL_MD1_O", w),
                (1, "IK") => builder.extra_name("LL_MD1_T", w),
                _ => (),
            }

            imux_wires.push(w);
            imux_nw.push((TileCellId::from_idx(0), w));
        }
    }

    if builder.rd.family != "xc4000e" {
        let w = builder.mux_out("IMUX.BOT.COUT", &[""]);
        for k in BOT_KINDS {
            builder.extra_name(format!("{k}_COUT"), w);
        }
        imux_wires.push(w);
        imux_nw.push((TileCellId::from_idx(0), w));
    }

    for pin in ["CLK", "GSR", "GTS"] {
        builder.mux_out(format!("IMUX.STARTUP.{pin}"), &[format!("LR_STUP_{pin}")]);
    }
    builder.mux_out("IMUX.READCLK.I", &["LR_RDCLK_I"]);

    builder.mux_out(
        "IMUX.BUFG.H",
        &["LL_BUFG_I_3", "UL_BUFG_I_8", "LR_BUFG_I_4", "UR_BUFG7MUX"],
    );
    builder.mux_out(
        "IMUX.BUFG.V",
        &["LL_BUFG_I_2", "UL_BUFG_I_1", "LR_BUFG_I_5", "UR_CLKIN"],
    );

    for (n, xn) in [
        ("IMUX.TDO.O", "UR_TDO_1"),
        ("IMUX.TDO.T", "UR_TDO_2"),
        ("IMUX.RDBK.TRIG", "LL_RDBK_TRIG"),
        ("IMUX.BSCAN.TDO1", "UL_BSCAN2"),
        ("IMUX.BSCAN.TDO2", "UL_BSCAN6"),
    ] {
        let w = builder.mux_out(n, &[xn]);
        imux_wires.push(w);
        imux_nw.push((TileCellId::from_idx(0), w));
    }

    (imux_wires, imux_nw)
}

fn fill_out_wires(builder: &mut IntBuilder) {
    for pin in ["FX", "FXQ"] {
        let mut w = builder.logic_out(format!("OUT.CLB.{pin}"), &[format!("CENTER_{pin}")]);
        if builder.rd.family != "xc4000e" {
            builder.buf(
                w,
                format!("OUT.CLB.{pin}.H"),
                &[&format!("CENTER_{pin}_HORIZ")],
            );
            w = builder.buf(
                w,
                format!("OUT.CLB.{pin}.V"),
                &[&format!("CENTER_{pin}_VERT")],
            );
        }
        let ws = builder.branch(
            w,
            Dir::S,
            format!("OUT.CLB.{pin}.S"),
            &[format!("CENTER_{pin}T")],
        );
        for k in BOT_KINDS {
            builder.extra_name(format!("{k}_{pin}T"), ws);
        }
    }
    for pin in ["GY", "GYQ"] {
        let mut w = builder.logic_out(format!("OUT.CLB.{pin}"), &[format!("CENTER_{pin}")]);
        if builder.rd.family != "xc4000e" {
            builder.buf(
                w,
                format!("OUT.CLB.{pin}.V"),
                &[&format!("CENTER_{pin}_VERT")],
            );
            w = builder.buf(
                w,
                format!("OUT.CLB.{pin}.H"),
                &[&format!("CENTER_{pin}_HORIZ")],
            );
        }
        let we = builder.branch(
            w,
            Dir::E,
            format!("OUT.CLB.{pin}.E"),
            &[format!("CENTER_{pin}L")],
        );
        for k in RT_KINDS {
            builder.extra_name(format!("{k}_{pin}L"), we);
        }
    }

    for i in [3, 4] {
        builder.stub_out(format!("CENTER_TBUF{i}O"));
        for k in LEFT_KINDS.into_iter().chain(RT_KINDS) {
            builder.stub_out(format!("{k}_TBUF{i}O"));
            builder.stub_out(format!("{k}_HLL{i}PU"));
        }
        for j in 1..3 {
            builder.stub_out(format!("VHBRK_PU{j}_HLL{i}"));
        }
        builder.stub_out(format!("CLKV_PU_HLL{i}"));
        builder.stub_out(format!("CLKV_PU_HLL{i}R"));
        builder.stub_out(format!("CLKVC_PU_HLL{i}"));
        builder.stub_out(format!("CLKVC_PU_HLL{i}R"));
        builder.stub_out(format!("VHBRKV_PU_HLL{i}"));
        builder.stub_out(format!("VHBRKV_PU_HLL{i}R"));
        builder.stub_out(format!("VHBRKVC_PU_HLL{i}"));
        builder.stub_out(format!("VHBRKVC_PU_HLL{i}R"));
    }
    for t in ["JB", "BA", "DA", "HA", "JA", "BK", "DK", "HK", "JK"] {
        for i in 1..3 {
            builder.stub_out(format!("TBUF_{t}_{i}_O"));
            builder.stub_out(format!("PULLUP_{t}_{i}_O"));
        }
    }

    for t in ["AA", "AK", "KA", "KK"] {
        for i in 1..9 {
            builder.stub_out(format!("PULLUP_{t}_{i}_O"));
        }
    }

    for i in 0..2 {
        let ii = i + 1;
        for pin in ["I1", "I2"] {
            let w = builder.logic_out(format!("OUT.BT.IOB{i}.{pin}"), &[""]);
            for k in BOT_KINDS.into_iter().chain(TOP_KINDS) {
                builder.extra_name(format!("{k}_{pin}_{ii}"), w);
            }
            if i == 1 {
                let we = builder.branch(
                    w,
                    Dir::E,
                    format!("OUT.BT.IOB{i}.{pin}.E"),
                    &[format!("LR_L{pin}_{ii}"), format!("UR_{pin}_{ii}")],
                );
                for k in BOT_KINDS.into_iter().chain(TOP_KINDS) {
                    builder.extra_name(format!("{k}_{pin}_{ii}L"), we);
                }
                if pin == "I1" {
                    builder.extra_name("UL_BSCAN5", w);
                    builder.extra_name("TOPSL_BSCAN5", we);
                    builder.extra_name("LL_MD2_I", w);
                } else {
                    builder.extra_name("UL_BSCAN1", w);
                    builder.extra_name("TOPSL_BSCAN1", we);
                    builder.extra_name("LL_RDBK_RIP", w);
                }
            }
        }
    }

    for i in 0..2 {
        let ii = i + 1;
        for pin in ["I1", "I2"] {
            let w = builder.logic_out(format!("OUT.LR.IOB{i}.{pin}"), &[""]);
            for k in RT_KINDS.into_iter().chain(LEFT_KINDS) {
                builder.extra_name(format!("{k}_{pin}_{ii}"), w);
            }
            if i == 1 {
                let ws = builder.branch(
                    w,
                    Dir::S,
                    format!("OUT.LR.IOB{i}.{pin}.S"),
                    &[format!("LL_{pin}_{ii}"), format!("LR_T{pin}_{ii}")],
                );
                for k in RT_KINDS.into_iter().chain(LEFT_KINDS) {
                    builder.extra_name(format!("{k}_{pin}_{ii}T"), ws);
                }
                if pin == "I1" {
                    builder.extra_name("UL_BSCAN3", w);
                    builder.extra_name("LEFTT_BSCAN3", ws);
                    builder.extra_name("UR_OSC1", w);
                    builder.extra_name("RTT_OSC2", ws);
                } else {
                    builder.extra_name("UL_BSCAN4", w);
                    builder.extra_name("LEFTT_BSCAN4", ws);
                    builder.extra_name("UR_OSC_OUT", w);
                    builder.extra_name("RTT_OSC1", ws);
                }
            }
        }
    }

    let w = builder.logic_out(
        "OUT.IOB.CLKIN",
        &[
            "BOTRR_CLKIN",
            "BOTSL_CLKIN",
            "LEFTSB_CLKIN",
            "LEFTT_CLKIN",
            "RTSB_CLKIN",
            "RTT_CLKIN",
            "TOPRR_CLKIN",
            "TOPSL_CLKIN",
        ],
    );
    builder.branch(
        w,
        Dir::W,
        "OUT.IOB.CLKIN.W",
        &["UL_CLKIN_TOP", "LL_CLKIN_R"],
    );
    builder.branch(
        w,
        Dir::E,
        "OUT.IOB.CLKIN.E",
        &["LR_CLKIN_LEFT", "UR_BUFG7MUX_L"],
    );
    builder.branch(
        w,
        Dir::S,
        "OUT.IOB.CLKIN.S",
        &["LL_CLKIN_TOP", "LR_CLKIN_TOP"],
    );
    builder.branch(
        w,
        Dir::N,
        "OUT.IOB.CLKIN.N",
        &["UR_BUFG6MUX_B", "UL_CLKIN_LEFT"],
    );

    for i in 1..13 {
        for k in BOT_KINDS
            .into_iter()
            .chain(TOP_KINDS)
            .chain(LEFT_KINDS)
            .chain(RT_KINDS)
        {
            builder.stub_out(format!("{k}_TXM{i}"));
        }
    }

    for t in [
        "KB", "KC", "KD", "KH", "AB", "AC", "AD", "AH", "BA", "DA", "HA", "JA", "BK", "DK", "HK",
        "JK",
    ] {
        for i in 1..4 {
            for j in 1..5 {
                builder.stub_out(format!("DEC_{t}_{i}_O{j}"));
            }
        }
    }

    for pref in [
        "LR_L", "LR_R", "UR_L", "UR_R", "LL_L", "LL_B", "UL_L", "UL_T",
    ] {
        for i in 1..5 {
            builder.stub_out(format!("{pref}_PU{i}"));
        }
    }

    for i in 1..5 {
        builder.stub_out(format!("CLKB_PU_BTX{i}"));
        builder.stub_out(format!("CLKB_PU_BTX{i}R"));
        builder.stub_out(format!("CLKT_PU_TTX{i}"));
        builder.stub_out(format!("CLKT_PU_TTX{i}R"));
        builder.stub_out(format!("CLKL_PU_LTX{i}"));
        builder.stub_out(format!("CLKL_PU_LTX{i}T"));
        builder.stub_out(format!("CLKR_PU_RTX{i}"));
        builder.stub_out(format!("CLKR_PU_RTX{i}T"));
    }

    builder.logic_out("OUT.OSC.MUX1", &["UR_OSC_IN"]);

    for pin in ["DONEIN", "Q1Q4", "Q2", "Q3"] {
        builder.logic_out(format!("OUT.STARTUP.{pin}"), &[format!("LR_STUP_{pin}")]);
    }

    if builder.rd.family != "xc4000e" {
        let w = builder.logic_out("OUT.TOP.COUT", &[""]);
        for k in TOP_KINDS {
            builder.extra_name(format!("{k}_COUTB"), w);
        }
        let w = builder.branch(w, Dir::E, "OUT.TOP.COUT.E", &["UR_COUT"]);
        for k in TOP_KINDS {
            builder.extra_name(format!("{k}_COUTL"), w);
        }
    }

    builder.logic_out("OUT.UPDATE.O", &["UR_UPDATE"]);
    if builder.rd.family != "spartanxl" {
        builder.logic_out("OUT.MD0.I", &["LL_MD0_I"]);
    }
    builder.logic_out("OUT.RDBK.DATA", &["LL_RDBK_DATA"]);

    if !matches!(&*builder.rd.family, "xc4000e" | "spartanxl") {
        builder.logic_out(
            "OUT.BUFGE.H",
            &["LL_BUFGE_3", "UL_BUFGE_7_8", "LR_BUFGE_4", "UR_BUFGE_7_8"],
        );
        builder.logic_out(
            "OUT.BUFGE.V",
            &["LL_BUFGE_2", "UL_BUFGE_1X", "LR_BUFGE_5", "UR_BUFGE_6X"],
        );

        let w = builder.logic_out("OUT.BUFF", &[""]);
        for n in [
            "LHVBRK_FCLK_OUT",
            "LVLBRK_FCLK_OUT",
            "RHVBRK_FCLK_OUT",
            "RVRBRK_FCLK_OUT",
        ] {
            builder.extra_name_sub(n, 1, w);
        }
    }
}

fn fill_xc4000e_wirenames(builder: &mut IntBuilder) {
    for &(name, wire) in xc4000e_wires::XC4000E_WIRES {
        builder.extra_name(name, builder.db.get_wire(wire));
    }
}

fn extract_clb(
    builder: &mut IntBuilder,
    imux_wires: &[WireId],
    imux_nw: &[TileClassWire],
    force_names: &[(usize, String, WireId)],
) {
    let is_xv = builder.rd.family == "xc4000xv";
    let tbuf_wires = [
        builder.db.get_wire("LONG.H2"),
        builder.db.get_wire("LONG.H3"),
    ];

    for &crd in builder.rd.tiles_by_kind_name("CENTER") {
        let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
        let xy_n = builder.walk_to_int(crd, Dir::N, true).unwrap();

        let lx = if is_xv { 2 } else { 1 };
        let kind = if crd.y == 1 {
            if crd.x == lx {
                "CLB.LB"
            } else if xy_e.x == builder.rd.width - 1 {
                "CLB.RB"
            } else {
                "CLB.B"
            }
        } else if xy_n.y == builder.rd.height - 1 {
            if crd.x == lx {
                "CLB.LT"
            } else if xy_e.x == builder.rd.width - 1 {
                "CLB.RT"
            } else {
                "CLB.T"
            }
        } else {
            if crd.x == lx {
                "CLB.L"
            } else if xy_e.x == builder.rd.width - 1 {
                "CLB.R"
            } else {
                "CLB"
            }
        };
        let mut naming = "CLB".to_string();
        for xy in [xy_n, xy_e] {
            let kind = builder.rd.tile_kinds.key(builder.rd.tiles[&xy].kind);
            if kind != "CENTER" {
                write!(naming, ".{kind}").unwrap();
            }
        }

        let mut bel = builder.bel_single(bels::CLB, "CLB").pin_name_only("CIN", 0);
        if builder.rd.family == "xc4000e" {
            bel = bel
                .pin_name_only("COUT", 0)
                .extra_wire("CIN.B", &["CENTER_SEG_38"])
                .extra_wire("CIN.T", &["CENTER_SEG_56"]);
        } else {
            bel = bel.pin_name_only("COUT", 1);
        }
        let mut bels = vec![bel];
        for i in 0..2 {
            bels.push(builder.bel_indexed(bels::TBUF[i], "TBUF", [2, 1][i]));
        }

        let mut xn = builder
            .xnode(kind, &naming, crd)
            .num_tiles(3)
            .raw_tile_single(xy_n, 1)
            .raw_tile_single(xy_e, 2)
            .extract_muxes()
            .optin_muxes_tile(imux_nw)
            .skip_muxes(imux_wires)
            .skip_muxes(&tbuf_wires)
            .bels(bels);
        for &(rti, ref name, wire) in force_names {
            xn = xn.force_name(rti, name, (TileCellId::from_idx(0), wire));
        }
        for (wt, wf) in [
            ("IMUX.CLB.F2", "IMUX.IOB0.O1"),
            ("IMUX.CLB.G2", "IMUX.IOB1.O1"),
        ] {
            let wt = (TileCellId::from_idx(0), xn.builder.db.get_wire(wt));
            let wf = (TileCellId::from_idx(1), xn.builder.db.get_wire(wf));
            xn = xn.force_skip_pip(wt, wf);
        }
        if is_xv {
            xn = xn
                .raw_tile(crd.delta(-1, 0))
                .raw_tile(crd.delta(0, 1))
                .raw_tile(crd.delta(-1, 1))
                .extract_muxes_rt(3)
                .extract_muxes_rt(4)
                .extract_muxes_rt(5);
        }
        xn.extract();
    }

    let naming = builder.ndb.get_tile_class_naming("CLB");
    builder.inject_node_type_naming("CENTER", naming);
}

fn extract_bot(
    builder: &mut IntBuilder,
    imux_wires: &[WireId],
    imux_nw: &[TileClassWire],
    force_names: &[(usize, String, WireId)],
) {
    let is_xv = builder.rd.family == "xc4000xv";
    let eclk_h = builder.db.wires.get("ECLK.H").map(|x| x.0);
    let long_io = [
        builder.db.get_wire("LONG.IO.H0"),
        builder.db.get_wire("LONG.IO.H1"),
        builder.db.get_wire("LONG.IO.H3"),
    ];
    for (nn, tkn) in [
        ("IO.B", "BOT"),
        ("IO.B.R", "BOTRR"),
        ("IO.BS", "BOTS"),
        ("IO.BS.L", "BOTSL"),
    ] {
        let mut found_naming = None;
        for &crd in builder.rd.tiles_by_kind_name(tkn) {
            let xy_n = builder.walk_to_int(crd, Dir::N, true).unwrap();
            let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
            let xy_w = builder.walk_to_int(crd, Dir::W, true).unwrap();
            let kind_e = builder.rd.tile_kinds.key(builder.rd.tiles[&xy_e].kind);
            let naming = format!("{tkn}.{kind_e}");
            let mut bels = vec![];
            for i in 0..2 {
                bels.push(builder.bel_indexed(bels::IO[i], "IOB", i + 1))
            }
            if builder.rd.family != "spartanxl" {
                for i in 0..3 {
                    bels.push(builder.bel_indexed(bels::DEC[i], "DEC", i + 1))
                }
            }
            let cout_names: Vec<_> = BOT_KINDS.into_iter().map(|k| format!("{k}_COUT")).collect();
            if builder.rd.family != "xc4000e" {
                bels.push(
                    builder
                        .bel_virtual(bels::CIN)
                        .extra_int_in("CIN", &cout_names),
                );
            }
            let mut xn = builder
                .xnode(nn, &naming, crd)
                .num_tiles(4)
                .raw_tile_single(xy_n, 1)
                .raw_tile_single(xy_e, 2)
                .raw_tile_single(xy_w, 3)
                .extract_muxes()
                .optin_muxes_tile(imux_nw)
                .skip_muxes(imux_wires)
                .bels(bels);
            for &(rti, ref name, wire) in force_names {
                xn = xn.force_name(rti, name, (TileCellId::from_idx(0), wire));
            }
            if let Some(eclk_h) = eclk_h {
                if tkn == "BOTSL" {
                    xn = xn
                        .force_name(3, "LL_FCLK", (TileCellId::from_idx(0), eclk_h))
                        .force_name(3, "LL_BHLL1", (TileCellId::from_idx(0), long_io[0]))
                        .force_name(3, "LL_BHLL2", (TileCellId::from_idx(0), long_io[1]))
                        .force_name(3, "LL_BHLL4", (TileCellId::from_idx(0), long_io[2]))
                        .optin_muxes_tile(&[(TileCellId::from_idx(0), eclk_h)]);
                }
            }
            for (wt, wf) in [
                ("IMUX.CLB.F4", "IMUX.IOB0.O1"),
                ("IMUX.CLB.G4", "IMUX.IOB1.O1"),
            ] {
                let wt = (TileCellId::from_idx(0), xn.builder.db.get_wire(wt));
                let wf = (TileCellId::from_idx(0), xn.builder.db.get_wire(wf));
                xn = xn.force_skip_pip(wt, wf);
            }
            if is_xv {
                xn = xn.raw_tile(crd.delta(-1, 0)).extract_muxes_rt(4);
            }
            xn.extract();
            found_naming = Some(naming);
        }
        let naming = builder.ndb.get_tile_class_naming(&found_naming.unwrap());
        builder.inject_node_type_naming(tkn, naming);
    }
}

fn extract_top(builder: &mut IntBuilder, imux_wires: &[WireId], imux_nw: &[TileClassWire]) {
    let is_xv = builder.rd.family == "xc4000xv";
    let eclk_h = builder.db.wires.get("ECLK.H").map(|x| x.0);
    let long_io = [
        builder.db.get_wire("LONG.IO.H0"),
        builder.db.get_wire("LONG.IO.H1"),
        builder.db.get_wire("LONG.IO.H3"),
    ];
    for (nn, tkn) in [
        ("IO.T", "TOP"),
        ("IO.T.R", "TOPRR"),
        ("IO.TS", "TOPS"),
        ("IO.TS.L", "TOPSL"),
    ] {
        let mut found_naming = None;
        for &crd in builder.rd.tiles_by_kind_name(tkn) {
            let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
            let xy_w = builder.walk_to_int(crd, Dir::W, true).unwrap();
            let kind_e = builder.rd.tile_kinds.key(builder.rd.tiles[&xy_e].kind);
            let naming = format!("{tkn}.{kind_e}");

            let mut bels = vec![];
            for i in 0..2 {
                bels.push(builder.bel_indexed(bels::IO[i], "IOB", i + 1))
            }
            if builder.rd.family != "spartanxl" {
                for i in 0..3 {
                    bels.push(builder.bel_indexed(bels::DEC[i], "DEC", i + 1))
                }
            }
            let cout_names: Vec<_> = TOP_KINDS
                .into_iter()
                .map(|k| format!("{k}_COUTB"))
                .collect();
            if builder.rd.family != "xc4000e" {
                bels.push(
                    builder
                        .bel_virtual(bels::COUT)
                        .extra_int_out("COUT", &cout_names),
                );
            }
            let mut xn = builder
                .xnode(nn, &naming, crd)
                .num_tiles(3)
                .raw_tile_single(xy_e, 1)
                .raw_tile_single(xy_w, 2)
                .extract_muxes()
                .optin_muxes_tile(imux_nw)
                .skip_muxes(imux_wires)
                .bels(bels);
            if is_xv {
                xn = xn.raw_tile(crd.delta(-1, 0)).extract_muxes_rt(3);
            }
            if let Some(eclk_h) = eclk_h {
                if tkn == "TOPSL" {
                    xn = xn
                        .force_name(2, "UL_FCLK", (TileCellId::from_idx(0), eclk_h))
                        .force_name(2, "UL_THLL1", (TileCellId::from_idx(0), long_io[0]))
                        .force_name(2, "UL_THLL2", (TileCellId::from_idx(0), long_io[1]))
                        .force_name(2, "UL_THLL4", (TileCellId::from_idx(0), long_io[2]))
                        .optin_muxes_tile(&[(TileCellId::from_idx(0), eclk_h)]);
                }
            }
            xn.extract();
            found_naming = Some(naming);
        }
        let naming = builder.ndb.get_tile_class_naming(&found_naming.unwrap());
        builder.inject_node_type_naming(tkn, naming);
    }
}

fn extract_rt(builder: &mut IntBuilder, imux_wires: &[WireId], imux_nw: &[TileClassWire]) {
    let is_xv = builder.rd.family == "xc4000xv";
    let is_e = builder.rd.family == "xc4000e";
    let eclk_v = builder.db.wires.get("ECLK.V").map(|x| x.0);
    let tbuf_wires = [
        builder.db.get_wire("LONG.H2"),
        builder.db.get_wire("LONG.H3"),
    ];
    let single_wires = [
        builder.db.get_wire("SINGLE.V3"),
        builder.db.get_wire("SINGLE.V4"),
    ];
    for (nn, tkn) in [
        ("IO.R", "RT"),
        ("IO.R.T", "RTT"),
        ("IO.R.FB", "RTF"),
        ("IO.R.FT", "RTF1"),
        ("IO.RS", "RTS"),
        ("IO.RS.B", "RTSB"),
        ("IO.RS.FB", "RTSF"),
        ("IO.RS.FT", "RTSF1"),
    ] {
        let mut found_naming = None;
        for &crd in builder.rd.tiles_by_kind_name(tkn) {
            let xy_s = builder.walk_to_int(crd, Dir::S, true).unwrap();
            let xy_n = builder.walk_to_int(crd, Dir::N, true).unwrap();
            let kind_s = builder.rd.tile_kinds.key(builder.rd.tiles[&xy_s].kind);
            let naming = format!("{tkn}.{kind_s}");

            let mut bels = vec![];
            for i in 0..2 {
                let mut bel = builder.bel_indexed(bels::IO[i], "IOB", i + 1);
                if matches!(&*builder.rd.family, "xc4000xla" | "xc4000xv")
                    && (tkn.ends_with('F') || tkn.ends_with("F1"))
                {
                    bel = bel.pin_name_only("CLKIN", 0);
                }
                bels.push(bel)
            }
            for i in 0..2 {
                bels.push(builder.bel_indexed(bels::TBUF[i], "TBUF", [2, 1][i]));
            }
            for i in 0..2 {
                bels.push(builder.bel_indexed(bels::PULLUP_TBUF[i], "PULLUP", [2, 1][i]));
            }
            if builder.rd.family != "spartanxl" {
                for i in 0..3 {
                    bels.push(builder.bel_indexed(bels::DEC[i], "DEC", i + 1))
                }
            }

            let mut xn = builder
                .xnode(nn, &naming, crd)
                .num_tiles(3)
                .raw_tile_single(xy_s, 1)
                .raw_tile_single(xy_n, 2)
                .extract_muxes()
                .optin_muxes_tile(imux_nw)
                .skip_muxes(imux_wires)
                .bels(bels);
            if is_e {
                xn = xn
                    .skip_muxes(&tbuf_wires)
                    .force_pip(
                        (TileCellId::from_idx(0), tbuf_wires[0]),
                        (TileCellId::from_idx(0), single_wires[0]),
                    )
                    .force_pip(
                        (TileCellId::from_idx(0), tbuf_wires[1]),
                        (TileCellId::from_idx(0), single_wires[1]),
                    );
            }
            for (wt, wf) in [
                ("IMUX.CLB.G1", "IMUX.IOB0.O1"),
                ("IMUX.CLB.F1", "IMUX.IOB1.O1"),
            ] {
                let wt = (TileCellId::from_idx(0), xn.builder.db.get_wire(wt));
                let wf = (TileCellId::from_idx(0), xn.builder.db.get_wire(wf));
                xn = xn.force_skip_pip(wt, wf);
            }
            if is_xv {
                xn = xn.raw_tile(crd.delta(0, 1)).extract_muxes_rt(3);
            }
            if let Some(eclk_v) = eclk_v {
                if tkn == "RTT" {
                    xn = xn
                        .force_name(2, "UR_URKX", (TileCellId::from_idx(0), eclk_v))
                        .optin_muxes_tile(&[(TileCellId::from_idx(0), eclk_v)]);
                }
            }
            xn.extract();
            found_naming = Some(naming);
        }

        if let Some(naming) = found_naming {
            let naming = builder.ndb.get_tile_class_naming(&naming);
            builder.inject_node_type_naming(tkn, naming);
        }
    }
}

fn extract_left(builder: &mut IntBuilder, imux_wires: &[WireId], imux_nw: &[TileClassWire]) {
    let is_xv = builder.rd.family == "xc4000xv";
    let eclk_v = builder.db.wires.get("ECLK.V").map(|x| x.0);
    let tbuf_wires = [
        builder.db.get_wire("LONG.H2"),
        builder.db.get_wire("LONG.H3"),
    ];
    for (nn, tkn) in [
        ("IO.L", "LEFT"),
        ("IO.L.T", "LEFTT"),
        ("IO.L.FB", "LEFTF"),
        ("IO.L.FT", "LEFTF1"),
        ("IO.LS", "LEFTS"),
        ("IO.LS.B", "LEFTSB"),
        ("IO.LS.FB", "LEFTSF"),
        ("IO.LS.FT", "LEFTSF1"),
    ] {
        let mut found_naming = None;
        for &crd in builder.rd.tiles_by_kind_name(tkn) {
            let xy_s = builder.walk_to_int(crd, Dir::S, true).unwrap();
            let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
            let xy_n = builder.walk_to_int(crd, Dir::N, true).unwrap();
            let kind_s = builder.rd.tile_kinds.key(builder.rd.tiles[&xy_s].kind);
            let naming = format!("{tkn}.{kind_s}");

            let mut bels = vec![];
            for i in 0..2 {
                let mut bel = builder.bel_indexed(bels::IO[i], "IOB", i + 1);
                if matches!(&*builder.rd.family, "xc4000xla" | "xc4000xv")
                    && (tkn.ends_with('F') || tkn.ends_with("F1"))
                {
                    bel = bel.pin_name_only("CLKIN", 0);
                }
                bels.push(bel)
            }
            for i in 0..2 {
                bels.push(builder.bel_indexed(bels::TBUF[i], "TBUF", [2, 1][i]));
            }
            for i in 0..2 {
                bels.push(builder.bel_indexed(bels::PULLUP_TBUF[i], "PULLUP", [2, 1][i]));
            }
            if builder.rd.family != "spartanxl" {
                for i in 0..3 {
                    bels.push(builder.bel_indexed(bels::DEC[i], "DEC", i + 1))
                }
            }

            let mut xn = builder
                .xnode(nn, &naming, crd)
                .num_tiles(4)
                .raw_tile_single(xy_s, 1)
                .raw_tile_single(xy_e, 2)
                .raw_tile_single(xy_n, 3)
                .extract_muxes()
                .optin_muxes_tile(imux_nw)
                .skip_muxes(imux_wires)
                .skip_muxes(&tbuf_wires)
                .bels(bels);
            if is_xv {
                xn = xn.raw_tile(crd.delta(0, 1)).extract_muxes_rt(4);
            }
            if let Some(eclk_v) = eclk_v {
                if tkn == "LEFTT" {
                    xn = xn
                        .force_name(3, "UL_KX", (TileCellId::from_idx(0), eclk_v))
                        .optin_muxes_tile(&[(TileCellId::from_idx(0), eclk_v)]);
                }
            }
            xn.extract();
            found_naming = Some(naming);
        }

        if let Some(naming) = found_naming {
            let naming = builder.ndb.get_tile_class_naming(&naming);
            builder.inject_node_type_naming(tkn, naming);
        }
    }
}

fn extract_lr(builder: &mut IntBuilder, imux_wires: &[WireId], imux_nw: &[TileClassWire]) {
    for &crd in builder.rd.tiles_by_kind_name("LR") {
        let mut bels = vec![];
        match &*builder.rd.family {
            "spartanxl" => {
                bels.extend([
                    builder
                        .bel_indexed(bels::BUFGLS_H, "BUFGLS", 3)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_indexed(bels::BUFGLS_V, "BUFGLS", 4)
                        .pins_name_only(&["O"]),
                ]);
            }
            "xc4000e" => {
                for i in 0..4 {
                    bels.push(builder.bel_indexed(bels::PULLUP_DEC_H[i], "PULLUP", (i ^ 7) + 1));
                }
                for i in 0..4 {
                    bels.push(builder.bel_indexed(bels::PULLUP_DEC_V[i], "PULLUP", (i ^ 3) + 1));
                }
                bels.extend([
                    builder
                        .bel_single(bels::BUFGLS_H, "BUFGS")
                        .pins_name_only(&["O"]),
                    builder
                        .bel_single(bels::BUFGLS_V, "BUFGP")
                        .pins_name_only(&["O"]),
                    builder
                        .bel_single(bels::COUT, "COUT")
                        .pins_name_only(&["I"]),
                ]);
            }
            _ => {
                for i in 0..4 {
                    bels.push(builder.bel_indexed(bels::PULLUP_DEC_H[i], "PULLUP", (i ^ 7) + 1));
                }
                for i in 0..4 {
                    bels.push(builder.bel_indexed(bels::PULLUP_DEC_V[i], "PULLUP", (i ^ 3) + 1));
                }
                bels.extend([
                    builder
                        .bel_indexed(bels::BUFG_H, "BUFG", 3)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_indexed(bels::BUFG_V, "BUFG", 4)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_indexed(bels::BUFGE_H, "BUFGE", 3)
                        .pins_name_only(&["I"]),
                    builder
                        .bel_indexed(bels::BUFGE_V, "BUFGE", 4)
                        .pins_name_only(&["I"]),
                    builder
                        .bel_indexed(bels::BUFGLS_H, "BUFGLS", 3)
                        .pins_name_only(&["I"])
                        .pin_name_only("O", 1),
                    builder
                        .bel_indexed(bels::BUFGLS_V, "BUFGLS", 4)
                        .pins_name_only(&["I"])
                        .pin_name_only("O", 1),
                ]);
            }
        }
        bels.extend([
            builder.bel_single(bels::STARTUP, "STARTUP"),
            builder.bel_single(bels::READCLK, "RDCLK"),
        ]);

        builder
            .xnode("CNR.BR", "CNR.BR", crd)
            .extract_muxes()
            .optin_muxes_tile(imux_nw)
            .skip_muxes(imux_wires)
            .bels(bels)
            .extract();
    }
}

fn extract_ur(builder: &mut IntBuilder, imux_wires: &[WireId], imux_nw: &[TileClassWire]) {
    let eclk_v = builder.db.wires.get("ECLK.V").map(|x| x.0);
    for &crd in builder.rd.tiles_by_kind_name("UR") {
        let mut bels = vec![];
        match &*builder.rd.family {
            "spartanxl" => {
                bels.extend([
                    builder
                        .bel_indexed(bels::BUFGLS_H, "BUFGLS", 2)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_indexed(bels::BUFGLS_V, "BUFGLS", 1)
                        .pins_name_only(&["O"]),
                ]);
            }
            "xc4000e" => {
                for i in 0..4 {
                    bels.push(builder.bel_indexed(bels::PULLUP_DEC_H[i], "PULLUP", i + 1));
                }
                for i in 0..4 {
                    bels.push(builder.bel_indexed(bels::PULLUP_DEC_V[i], "PULLUP", i + 5));
                }
                bels.extend([
                    builder
                        .bel_single(bels::BUFGLS_H, "BUFGP")
                        .pins_name_only(&["O"]),
                    builder
                        .bel_single(bels::BUFGLS_V, "BUFGS")
                        .pins_name_only(&["O"]),
                    builder
                        .bel_single(bels::COUT, "COUT")
                        .pins_name_only(&["I"]),
                ]);
            }
            _ => {
                for i in 0..4 {
                    bels.push(builder.bel_indexed(bels::PULLUP_DEC_H[i], "PULLUP", i + 1));
                }
                for i in 0..4 {
                    bels.push(builder.bel_indexed(bels::PULLUP_DEC_V[i], "PULLUP", i + 5));
                }
                bels.extend([
                    builder
                        .bel_indexed(bels::BUFG_H, "BUFG", 2)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_indexed(bels::BUFG_V, "BUFG", 1)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_indexed(bels::BUFGE_H, "BUFGE", 2)
                        .pins_name_only(&["I"]),
                    builder
                        .bel_indexed(bels::BUFGE_V, "BUFGE", 1)
                        .pins_name_only(&["I"]),
                    builder
                        .bel_indexed(bels::BUFGLS_H, "BUFGLS", 2)
                        .pins_name_only(&["I"])
                        .pin_name_only("O", 1),
                    builder
                        .bel_indexed(bels::BUFGLS_V, "BUFGLS", 1)
                        .pins_name_only(&["I"])
                        .pin_name_only("O", 1),
                ]);
            }
        }
        bels.extend([
            builder.bel_single(bels::UPDATE, "UPDATE"),
            builder
                .bel_single(bels::OSC, "OSC")
                .pins_name_only(&["F15", "F490", "F16K", "F500K"])
                .extra_int_out("OUT0", &["UR_SEG_4", "UR_OSC_OUT"])
                .extra_int_out("OUT1", &["UR_SEG_44", "UR_OSC_IN"]),
            builder.bel_single(bels::TDO, "TDO"),
        ]);
        let xy_s = builder.walk_to_int(crd, Dir::S, true).unwrap();

        let mut xn = builder
            .xnode("CNR.TR", "CNR.TR", crd)
            .num_tiles(2)
            .raw_tile_single(xy_s, 1)
            .extract_muxes()
            .optin_muxes_tile(imux_nw)
            .skip_muxes(imux_wires);
        if let Some(eclk_v) = eclk_v {
            xn = xn.skip_muxes(&[eclk_v]);
        }
        xn.bels(bels).extract();
    }
}

fn extract_ll(builder: &mut IntBuilder, imux_wires: &[WireId], imux_nw: &[TileClassWire]) {
    let eclk_h = builder.db.wires.get("ECLK.H").map(|x| x.0);
    for &crd in builder.rd.tiles_by_kind_name("LL") {
        let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
        let mut bels = vec![];
        match &*builder.rd.family {
            "spartanxl" => {
                bels.extend([
                    builder
                        .bel_indexed(bels::BUFGLS_H, "BUFGLS", 6)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_indexed(bels::BUFGLS_V, "BUFGLS", 5)
                        .pins_name_only(&["O"]),
                ]);
            }
            "xc4000e" => {
                for i in 0..4 {
                    bels.push(builder.bel_indexed(bels::PULLUP_DEC_H[i], "PULLUP", (i ^ 7) + 1));
                }
                for i in 0..4 {
                    bels.push(builder.bel_indexed(bels::PULLUP_DEC_V[i], "PULLUP", (i ^ 3) + 1));
                }
                bels.extend([
                    builder
                        .bel_single(bels::BUFGLS_H, "BUFGP")
                        .pins_name_only(&["O"]),
                    builder
                        .bel_single(bels::BUFGLS_V, "BUFGS")
                        .pins_name_only(&["O"]),
                    builder.bel_single(bels::CIN, "CIN").pin_name_only("O", 1),
                    builder.bel_single(bels::MD0, "MD0"),
                    builder.bel_single(bels::MD1, "MD1"),
                    builder.bel_single(bels::MD2, "MD2"),
                ]);
            }
            _ => {
                for i in 0..4 {
                    bels.push(builder.bel_indexed(bels::PULLUP_DEC_H[i], "PULLUP", (i ^ 3) + 5));
                }
                for i in 0..4 {
                    bels.push(builder.bel_indexed(bels::PULLUP_DEC_V[i], "PULLUP", (i ^ 3) + 1));
                }

                bels.extend([
                    builder
                        .bel_indexed(bels::BUFG_H, "BUFG", 6)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_indexed(bels::BUFG_V, "BUFG", 5)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_indexed(bels::BUFGE_H, "BUFGE", 6)
                        .pins_name_only(&["I"]),
                    builder
                        .bel_indexed(bels::BUFGE_V, "BUFGE", 5)
                        .pins_name_only(&["I"]),
                    builder
                        .bel_indexed(bels::BUFGLS_H, "BUFGLS", 6)
                        .pins_name_only(&["I"])
                        .pin_name_only("O", 1),
                    builder
                        .bel_indexed(bels::BUFGLS_V, "BUFGLS", 5)
                        .pins_name_only(&["I"])
                        .pin_name_only("O", 1),
                ]);

                bels.extend([
                    builder.bel_single(bels::MD0, "MD0"),
                    builder.bel_single(bels::MD1, "MD1"),
                    builder.bel_single(bels::MD2, "MD2"),
                ]);
            }
        }
        bels.extend([builder.bel_single(bels::RDBK, "RDBK")]);

        let mut xn = builder
            .xnode("CNR.BL", "CNR.BL", crd)
            .num_tiles(2)
            .raw_tile_single(xy_e, 1)
            .extract_muxes()
            .optin_muxes_tile(imux_nw)
            .skip_muxes(imux_wires);
        if let Some(eclk_h) = eclk_h {
            xn = xn.skip_muxes(&[eclk_h]);
        }
        xn.bels(bels).extract();
    }
}

fn extract_ul(builder: &mut IntBuilder, imux_wires: &[WireId], imux_nw: &[TileClassWire]) {
    let eclk_h = builder.db.wires.get("ECLK.H").map(|x| x.0);
    let eclk_v = builder.db.wires.get("ECLK.V").map(|x| x.0);
    for &crd in builder.rd.tiles_by_kind_name("UL") {
        let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
        let xy_s = builder.walk_to_int(crd, Dir::S, true).unwrap();
        let xy_se = builder.walk_to_int(xy_s, Dir::E, true).unwrap();
        let mut bels = vec![];

        match &*builder.rd.family {
            "spartanxl" => {
                bels.extend([
                    builder
                        .bel_indexed(bels::BUFGLS_H, "BUFGLS", 7)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_indexed(bels::BUFGLS_V, "BUFGLS", 0)
                        .pins_name_only(&["O"]),
                ]);
            }
            "xc4000e" => {
                for i in 0..4 {
                    bels.push(builder.bel_indexed(bels::PULLUP_DEC_H[i], "PULLUP", i + 1));
                }
                for i in 0..4 {
                    bels.push(builder.bel_indexed(bels::PULLUP_DEC_V[i], "PULLUP", i + 5));
                }
                bels.extend([
                    builder
                        .bel_single(bels::BUFGLS_H, "BUFGS")
                        .pins_name_only(&["O"]),
                    builder
                        .bel_single(bels::BUFGLS_V, "BUFGP")
                        .pins_name_only(&["O"]),
                    builder.bel_single(bels::CIN, "CIN").pin_name_only("O", 1),
                ]);
            }
            _ => {
                for i in 0..4 {
                    bels.push(builder.bel_indexed(bels::PULLUP_DEC_H[i], "PULLUP", i + 1));
                }
                for i in 0..4 {
                    bels.push(builder.bel_indexed(bels::PULLUP_DEC_V[i], "PULLUP", i + 5));
                }
                bels.extend([
                    builder
                        .bel_indexed(bels::BUFG_H, "BUFG", 7)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_indexed(bels::BUFG_V, "BUFG", 0)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_indexed(bels::BUFGE_H, "BUFGE", 7)
                        .pins_name_only(&["I"]),
                    builder
                        .bel_indexed(bels::BUFGE_V, "BUFGE", 0)
                        .pins_name_only(&["I"]),
                    builder
                        .bel_indexed(bels::BUFGLS_H, "BUFGLS", 7)
                        .pins_name_only(&["I"])
                        .pin_name_only("O", 1),
                    builder
                        .bel_indexed(bels::BUFGLS_V, "BUFGLS", 0)
                        .pins_name_only(&["I"])
                        .pin_name_only("O", 1),
                ]);
            }
        }
        bels.extend([builder.bel_single(bels::BSCAN, "BSCAN")]);

        let mut xn = builder
            .xnode("CNR.TL", "CNR.TL", crd)
            .num_tiles(4)
            .raw_tile_single(xy_e, 1)
            .raw_tile_single(xy_s, 2)
            .raw_tile_single(xy_se, 3)
            .extract_muxes()
            .optin_muxes_tile(imux_nw)
            .skip_muxes(imux_wires);
        if let Some(eclk_h) = eclk_h {
            xn = xn.skip_muxes(&[eclk_h]);
        }
        if let Some(eclk_v) = eclk_v {
            xn = xn.skip_muxes(&[eclk_v]);
        }
        xn.bels(bels).extract();
    }
}

fn get_tile_naming(builder: &IntBuilder, crd: Coord) -> TileClassNamingId {
    let tkn = builder.rd.tile_kinds.key(builder.rd.tiles[&crd].kind);
    if tkn == "CENTER" {
        let xy_s = builder.walk_to_int(crd, Dir::S, true).unwrap();
        let xy_se = builder.walk_to_int(xy_s, Dir::E, true).unwrap();
        let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
        let xy_n = builder.walk_to_int(crd, Dir::N, true).unwrap();
        let mut naming = "CLB".to_string();
        for xy in [xy_s, xy_se, xy_e, xy_n] {
            let kind = builder.rd.tile_kinds.key(builder.rd.tiles[&xy].kind);
            if kind != "CENTER" {
                write!(naming, ".{kind}").unwrap();
            }
        }
        builder.ndb.get_tile_class_naming(&naming)
    } else if tkn.starts_with("BOT") || tkn.starts_with("TOP") {
        let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
        let kind_e = builder.rd.tile_kinds.key(builder.rd.tiles[&xy_e].kind);
        let naming = format!("{tkn}.{kind_e}");
        builder.ndb.get_tile_class_naming(&naming)
    } else if tkn.starts_with("LEFT") || tkn.starts_with("RT") {
        let xy_s = builder.walk_to_int(crd, Dir::S, true).unwrap();
        let kind_s = builder.rd.tile_kinds.key(builder.rd.tiles[&xy_s].kind);
        let naming = format!("{tkn}.{kind_s}");
        builder.ndb.get_tile_class_naming(&naming)
    } else {
        builder.ndb.get_tile_class_naming(tkn)
    }
}

fn extract_llh(builder: &mut IntBuilder) {
    let tbuf_wires = [
        builder.db.get_wire("LONG.H2"),
        builder.db.get_wire("LONG.H3"),
    ];
    let is_sxl = builder.rd.family == "spartanxl";
    for (kind, naming, tkn) in [
        ("LLH.IO.B", "LLH.IO.B", "CLKB"),
        ("LLH.IO.T", "LLH.IO.T", "CLKT"),
        (
            "LLH.CLB",
            "LLH.CLB.B",
            if builder.rd.family == "spartanxl" {
                "CLKVC"
            } else {
                "CLKVB"
            },
        ),
        (
            "LLH.CLB.B",
            "LLH.CLB.B",
            if builder.rd.family == "spartanxl" {
                "CLKVC"
            } else {
                "CLKVB"
            },
        ),
        ("LLH.CLB", "LLH.CLB.T", "CLKV"),
    ] {
        if let Some(&crd) = builder.rd.tiles_by_kind_name(tkn).first() {
            let xy_w = builder.walk_to_int(crd, Dir::W, true).unwrap();
            let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
            let naming_w = get_tile_naming(builder, xy_w);
            let naming_e = get_tile_naming(builder, xy_e);
            let mut bels = vec![];
            let has_splitter = is_sxl && tkn.starts_with("CLKV");
            if has_splitter {
                bels.extend([
                    builder
                        .bel_virtual(bels::TBUF_SPLITTER0)
                        .extra_int_inout("L", &["CLKV_HLL3", "CLKVC_HLL3"])
                        .extra_int_inout("R", &["CLKV_HLL3R", "CLKVC_HLL3R"])
                        .extra_wire("L.EXCL", &["CLKV_HLL3_EXCL", "CLKVC_HLL3_EXCL"])
                        .extra_wire("R.EXCL", &["CLKV_HLL3R_EXCL", "CLKVC_HLL3R_EXCL"]),
                    builder
                        .bel_virtual(bels::TBUF_SPLITTER1)
                        .extra_int_inout("L", &["CLKV_HLL4", "CLKVC_HLL4"])
                        .extra_int_inout("R", &["CLKV_HLL4R", "CLKVC_HLL4R"])
                        .extra_wire("L.EXCL", &["CLKV_HLL4_EXCL", "CLKVC_HLL4_EXCL"])
                        .extra_wire("R.EXCL", &["CLKV_HLL4R_EXCL", "CLKVC_HLL4R_EXCL"]),
                ]);
            }

            let mut xn = builder
                .xnode(kind, naming, crd)
                .num_tiles(2)
                .ref_single(xy_w, 0, naming_w)
                .ref_single(xy_e, 1, naming_e)
                .extract_muxes()
                .bels(bels);
            if has_splitter {
                xn = xn.skip_muxes(&tbuf_wires);
            }
            xn.extract();
        }
    }
}

fn extract_llv(builder: &mut IntBuilder) {
    let mut clk_wires = vec![];
    for (w, wn, _) in &builder.db.wires {
        if wn.starts_with("GCLK") {
            clk_wires.push(w);
        }
    }
    for (kind, tkn) in [
        ("LLV.IO.L", "CLKL"),
        ("LLV.IO.R", "CLKR"),
        ("LLV.CLB", "CLKH"),
    ] {
        if let Some(&crd) = builder.rd.tiles_by_kind_name(tkn).first() {
            let xy_s = builder.walk_to_int(crd, Dir::S, true).unwrap();
            let xy_n = builder.walk_to_int(crd, Dir::N, true).unwrap();
            let naming_s = get_tile_naming(builder, xy_s);
            let naming_n = get_tile_naming(builder, xy_n);
            let bel = builder
                .bel_virtual(bels::CLKH)
                .extra_int_out(
                    "O0",
                    &[
                        "CLKH_SEG_0",
                        "CLKL_SEG_0",
                        "CLKR_SEG_0",
                        "CLKH_K1",
                        "CLKL_K1",
                        "CLKR_K1",
                    ],
                )
                .extra_int_out(
                    "O1",
                    &[
                        "CLKH_SEG_2",
                        "CLKL_SEG_2",
                        "CLKR_SEG_2",
                        "CLKH_K2",
                        "CLKL_K2",
                        "CLKR_K2",
                    ],
                )
                .extra_int_out(
                    "O2",
                    &[
                        "CLKH_SEG_4",
                        "CLKL_SEG_4",
                        "CLKR_SEG_4",
                        "CLKH_K3",
                        "CLKL_K3",
                        "CLKR_K3",
                    ],
                )
                .extra_int_out(
                    "O3",
                    &[
                        "CLKH_SEG_6",
                        "CLKL_SEG_6",
                        "CLKR_SEG_6",
                        "CLKH_K4",
                        "CLKL_K4",
                        "CLKR_K4",
                    ],
                )
                .extra_wire(
                    "I.UL.V",
                    &[
                        "CLKH_SEG_1",
                        "CLKL_SEG_1",
                        "CLKR_SEG_1",
                        "CLKH_CLOCK_1",
                        "CLKL_CLOCK_1",
                        "CLKR_CLOCK_1",
                    ],
                )
                .extra_wire(
                    "I.LL.V",
                    &[
                        "CLKH_SEG_20",
                        "CLKL_SEG_24",
                        "CLKR_SEG_36",
                        "CLKH_CLOCK_2",
                        "CLKL_CLOCK_2",
                        "CLKR_CLOCK_2",
                    ],
                )
                .extra_wire(
                    "I.LL.H",
                    &[
                        "CLKH_SEG_3",
                        "CLKL_SEG_3",
                        "CLKR_SEG_3",
                        "CLKH_CLOCK_3",
                        "CLKL_CLOCK_3",
                        "CLKR_CLOCK_3",
                    ],
                )
                .extra_wire(
                    "I.LR.H",
                    &[
                        "CLKH_SEG_21",
                        "CLKL_SEG_25",
                        "CLKR_SEG_37",
                        "CLKH_CLOCK_4",
                        "CLKL_CLOCK_4",
                        "CLKR_CLOCK_4",
                    ],
                )
                .extra_wire(
                    "I.LR.V",
                    &[
                        "CLKH_SEG_5",
                        "CLKL_SEG_5",
                        "CLKR_SEG_5",
                        "CLKH_CLOCK_5",
                        "CLKL_CLOCK_5",
                        "CLKR_CLOCK_5",
                    ],
                )
                .extra_wire(
                    "I.UR.V",
                    &[
                        "CLKH_SEG_22",
                        "CLKL_SEG_26",
                        "CLKR_SEG_38",
                        "CLKH_CLOCK_6",
                        "CLKL_CLOCK_6",
                        "CLKR_CLOCK_6",
                    ],
                )
                .extra_wire(
                    "I.UR.H",
                    &[
                        "CLKH_SEG_7",
                        "CLKL_SEG_7",
                        "CLKR_SEG_7",
                        "CLKH_CLOCK_7",
                        "CLKL_CLOCK_7",
                        "CLKR_CLOCK_7",
                    ],
                )
                .extra_wire(
                    "I.UL.H",
                    &[
                        "CLKH_SEG_23",
                        "CLKL_SEG_27",
                        "CLKR_SEG_39",
                        "CLKH_CLOCK_8",
                        "CLKL_CLOCK_8",
                        "CLKR_CLOCK_8",
                    ],
                );
            builder
                .xnode(kind, kind, crd)
                .num_tiles(2)
                .ref_single(xy_s, 0, naming_s)
                .ref_single(xy_n, 1, naming_n)
                .extract_muxes()
                .skip_muxes(&clk_wires)
                .bel(bel)
                .extract();
        }
    }
}

fn extract_llhq(builder: &mut IntBuilder) {
    for (kind, naming, tkn) in [
        ("LLHQ.CLB", "LLHQ.CLB", "VHBRK"),
        ("LLHQ.CLB.B", "LLHQ.CLB", "VHBRK"),
        ("LLHQ.CLB.T", "LLHQ.CLB", "VHBRK"),
        ("LLHQ.CLB", "LLHQ.CLB.O", "VHBRKV"),
        ("LLHQ.CLB.B", "LLHQ.CLB.O", "VHBRKV"),
        ("LLHQ.CLB.T", "LLHQ.CLB.O", "VHBRKV"),
        ("LLHQ.CLB", "LLHQ.CLB.I", "VHBRKVC"),
        ("LLHQ.IO.B", "LLHQ.IO.B", "BVHBRK"),
        ("LLHQ.IO.T", "LLHQ.IO.T", "THRBRK"),
    ] {
        if let Some(&crd) = builder.rd.tiles_by_kind_name(tkn).first() {
            let xy_w = builder.walk_to_int(crd, Dir::W, true).unwrap();
            let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
            let naming_w = get_tile_naming(builder, xy_w);
            let naming_e = get_tile_naming(builder, xy_e);
            let mut bels = vec![];
            if kind.starts_with("LLHQ.CLB") {
                bels.extend([
                    builder.bel_indexed(bels::PULLUP_TBUF0_W, "PULLUP", 4),
                    builder.bel_indexed(bels::PULLUP_TBUF0_E, "PULLUP", 2),
                    builder.bel_indexed(bels::PULLUP_TBUF1_W, "PULLUP", 3),
                    builder.bel_indexed(bels::PULLUP_TBUF1_E, "PULLUP", 1),
                ]);
            }
            builder
                .xnode(kind, naming, crd)
                .num_tiles(2)
                .ref_single(xy_w, 0, naming_w)
                .ref_single(xy_e, 1, naming_e)
                .extract_muxes()
                .bels(bels)
                .extract();
        }
    }
}

fn extract_llhc(builder: &mut IntBuilder) {
    let tbuf_wires = [
        builder.db.get_wire("LONG.H2"),
        builder.db.get_wire("LONG.H3"),
    ];

    for (kind, naming, tkn) in [
        ("LLHC.CLB", "LLHC.CLB.O", "CLKV"),
        ("LLHC.CLB.B", "LLHC.CLB.O", "CLKV"),
        ("LLHC.CLB", "LLHC.CLB.I", "CLKVC"),
        ("LLHC.IO.B", "LLHC.IO.B", "CLKB"),
        ("LLHC.IO.T", "LLHC.IO.T", "CLKT"),
    ] {
        if let Some(&crd) = builder.rd.tiles_by_kind_name(tkn).first() {
            let xy_w = builder.walk_to_int(crd, Dir::W, true).unwrap();
            let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
            let naming_w = get_tile_naming(builder, xy_w);
            let naming_e = get_tile_naming(builder, xy_e);
            let mut bels = vec![];
            match kind {
                "LLHC.CLB" | "LLHC.CLB.B" => {
                    bels.extend([
                        builder.bel_indexed(bels::PULLUP_TBUF0_W, "PULLUP", 2),
                        builder.bel_indexed(bels::PULLUP_TBUF0_E, "PULLUP", 4),
                        builder.bel_indexed(bels::PULLUP_TBUF1_W, "PULLUP", 1),
                        builder.bel_indexed(bels::PULLUP_TBUF1_E, "PULLUP", 3),
                        builder
                            .bel_virtual(bels::TBUF_SPLITTER0)
                            .extra_int_inout("L", &["CLKV_HLL3", "CLKVC_HLL3"])
                            .extra_int_inout("R", &["CLKV_HLL3R", "CLKVC_HLL3R"])
                            .extra_wire("L.EXCL", &["CLKV_HLL3_EXCL", "CLKVC_HLL3_EXCL"])
                            .extra_wire("R.EXCL", &["CLKV_HLL3R_EXCL", "CLKVC_HLL3R_EXCL"]),
                        builder
                            .bel_virtual(bels::TBUF_SPLITTER1)
                            .extra_int_inout("L", &["CLKV_HLL4", "CLKVC_HLL4"])
                            .extra_int_inout("R", &["CLKV_HLL4R", "CLKVC_HLL4R"])
                            .extra_wire("L.EXCL", &["CLKV_HLL4_EXCL", "CLKVC_HLL4_EXCL"])
                            .extra_wire("R.EXCL", &["CLKV_HLL4R_EXCL", "CLKVC_HLL4R_EXCL"]),
                    ]);
                }
                "LLHC.IO.B" => {
                    bels.extend([
                        builder.bel_indexed(bels::PULLUP_DEC0_W, "PULLUP", 4),
                        builder.bel_indexed(bels::PULLUP_DEC0_E, "PULLUP", 5),
                        builder.bel_indexed(bels::PULLUP_DEC1_W, "PULLUP", 3),
                        builder.bel_indexed(bels::PULLUP_DEC1_E, "PULLUP", 6),
                        builder.bel_indexed(bels::PULLUP_DEC2_W, "PULLUP", 2),
                        builder.bel_indexed(bels::PULLUP_DEC2_E, "PULLUP", 7),
                        builder.bel_indexed(bels::PULLUP_DEC3_W, "PULLUP", 1),
                        builder.bel_indexed(bels::PULLUP_DEC3_E, "PULLUP", 8),
                    ]);
                }
                "LLHC.IO.T" => {
                    bels.extend([
                        builder.bel_indexed(bels::PULLUP_DEC0_W, "PULLUP", 1),
                        builder.bel_indexed(bels::PULLUP_DEC0_E, "PULLUP", 8),
                        builder.bel_indexed(bels::PULLUP_DEC1_W, "PULLUP", 2),
                        builder.bel_indexed(bels::PULLUP_DEC1_E, "PULLUP", 7),
                        builder.bel_indexed(bels::PULLUP_DEC2_W, "PULLUP", 3),
                        builder.bel_indexed(bels::PULLUP_DEC2_E, "PULLUP", 6),
                        builder.bel_indexed(bels::PULLUP_DEC3_W, "PULLUP", 4),
                        builder.bel_indexed(bels::PULLUP_DEC3_E, "PULLUP", 5),
                    ]);
                }
                _ => unreachable!(),
            }
            let mut xn = builder
                .xnode(kind, naming, crd)
                .num_tiles(2)
                .ref_single(xy_w, 0, naming_w)
                .ref_single(xy_e, 1, naming_e)
                .extract_muxes()
                .bels(bels);
            if kind.starts_with("LLHC.CLB") {
                xn = xn.skip_muxes(&tbuf_wires);
            }
            xn.extract();
        }
    }
}

fn extract_llvc(builder: &mut IntBuilder) {
    for (kind, tkn) in [
        ("LLVC.IO.L", "CLKL"),
        ("LLVC.IO.R", "CLKR"),
        ("LLVC.CLB", "CLKH"),
    ] {
        if let Some(&crd) = builder.rd.tiles_by_kind_name(tkn).first() {
            let xy_s = builder.walk_to_int(crd, Dir::S, true).unwrap();
            let xy_n = builder.walk_to_int(crd, Dir::N, true).unwrap();
            let naming_s = get_tile_naming(builder, xy_s);
            let naming_n = get_tile_naming(builder, xy_n);
            let mut bels = vec![];
            match kind {
                "LLVC.IO.L" | "LLVC.IO.R" => {
                    bels.extend([
                        builder.bel_indexed(bels::PULLUP_DEC0_S, "PULLUP", 10),
                        builder.bel_indexed(bels::PULLUP_DEC0_N, "PULLUP", 3),
                        builder.bel_indexed(bels::PULLUP_DEC1_S, "PULLUP", 9),
                        builder.bel_indexed(bels::PULLUP_DEC1_N, "PULLUP", 4),
                        builder.bel_indexed(bels::PULLUP_DEC2_S, "PULLUP", 8),
                        builder.bel_indexed(bels::PULLUP_DEC2_N, "PULLUP", 5),
                        builder.bel_indexed(bels::PULLUP_DEC3_S, "PULLUP", 7),
                        builder.bel_indexed(bels::PULLUP_DEC3_N, "PULLUP", 6),
                    ]);
                }
                _ => (),
            }
            builder
                .xnode(kind, kind, crd)
                .num_tiles(2)
                .ref_single(xy_s, 0, naming_s)
                .ref_single(xy_n, 1, naming_n)
                .extract_muxes()
                .bels(bels)
                .extract();
        }
    }
}

fn extract_llvq(builder: &mut IntBuilder) {
    for (kind, naming, tkn) in [
        ("LLVQ.CLB", "LLVQ.CLB", "HVBRK"),
        ("LLVQ.IO.L.B", "LLVQ.IO.L.B", "LHVBRK"),
        ("LLVQ.IO.L.T", "LLVQ.IO.L.T", "LVLBRK"),
        ("LLVQ.IO.R.B", "LLVQ.IO.R.B", "RHVBRK"),
        ("LLVQ.IO.R.B", "LLVQ.IO.R.BS", "RHVBRKS"),
        ("LLVQ.IO.R.T", "LLVQ.IO.R.T", "RVRBRK"),
        ("LLVQ.IO.R.T", "LLVQ.IO.R.TS", "RVRBRKS"),
    ] {
        if let Some(&crd) = builder.rd.tiles_by_kind_name(tkn).first() {
            let xy_s = builder.walk_to_int(crd, Dir::S, true).unwrap();
            let xy_n = builder.walk_to_int(crd, Dir::N, true).unwrap();
            let naming_s = get_tile_naming(builder, xy_s);
            let naming_n = get_tile_naming(builder, xy_n);
            let mut bels = vec![];
            if kind != "LLVQ.CLB" {
                bels.push(builder.bel_single(bels::BUFF, "BUFF").pin_name_only("I", 1));
            }
            builder
                .xnode(kind, naming, crd)
                .num_tiles(2)
                .ref_single(xy_s, 0, naming_s)
                .ref_single(xy_n, 1, naming_n)
                .extract_muxes()
                .bels(bels)
                .extract();
        }
    }
}

fn extract_clkc(builder: &mut IntBuilder) {
    if let Some(&crd) = builder.rd.tiles_by_kind_name("CLKC").first() {
        let bel = builder
            .bel_virtual(bels::CLKC)
            .extra_wire("I.LL.V", &["CLKC_BUFGLS_2_H"])
            .extra_wire("I.UL.V", &["CLKC_BUFGLS_1_H"])
            .extra_wire("I.LR.V", &["CLKC_BUFGLS_5_H"])
            .extra_wire("I.UR.V", &["CLKC_BUFGLS_6_H"])
            .extra_wire("O.LL.V", &["CLKC_BUFGLS_2"])
            .extra_wire("O.UL.V", &["CLKC_BUFGLS_1"])
            .extra_wire("O.LR.V", &["CLKC_BUFGLS_5"])
            .extra_wire("O.UR.V", &["CLKC_BUFGLS_6"]);
        builder
            .xnode("CLKC", "CLKC", crd)
            .num_tiles(0)
            .bel(bel)
            .extract();
    }
}

fn extract_clkqc(builder: &mut IntBuilder) {
    let hvbrk = builder.ndb.get_tile_class_naming("LLVQ.CLB");
    for (naming, tkn) in [("CLKQC.B", "HVBRKC"), ("CLKQC.T", "TVBRKC")] {
        if let Some(&crd) = builder.rd.tiles_by_kind_name(tkn).first() {
            let bel = builder
                .bel_virtual(bels::CLKQC)
                .extra_wire("I.LL.H", &["HVBRKC_BUFGLS_3", "TVBRKC_BUFGLS_3"])
                .extra_wire("I.LL.V", &["HVBRKC_BUFGLS_2", "TVBRKC_BUFGLS_2_B"])
                .extra_wire("I.UL.H", &["HVBRKC_BUFGLS_8", "TVBRKC_BUFGLS_8"])
                .extra_wire("I.UL.V", &["HVBRKC_BUFGLS_1", "TVBRKC_BUFGLS_1"])
                .extra_wire("I.LR.H", &["HVBRKC_BUFGLS_4", "TVBRKC_BUFGLS_4_B"])
                .extra_wire("I.LR.V", &["HVBRKC_BUFGLS_5", "TVBRKC_BUFGLS_5_B"])
                .extra_wire("I.UR.H", &["HVBRKC_BUFGLS_7", "TVBRKC_BUFGLS_7"])
                .extra_wire("I.UR.V", &["HVBRKC_BUFGLS_6", "TVBRKC_BUFGLS_6_B"])
                .extra_int_out("O.LL.H", &["HVBRKC_BUFGLS_3_H", "TVBRKC_BUFGLS_3_H"])
                .extra_int_out("O.LL.V", &["HVBRKC_BUFGLS_2_H", "TVBRKC_BUFGLS_2"])
                .extra_int_out("O.UL.H", &["HVBRKC_BUFGLS_8_H", "TVBRKC_BUFGLS_8_H"])
                .extra_int_out("O.UL.V", &["HVBRKC_BUFGLS_1_H", "TVBRKC_BUFGLS_1_H"])
                .extra_int_out("O.LR.H", &["HVBRKC_BUFGLS_4_H", "TVBRKC_BUFGLS_4"])
                .extra_int_out("O.LR.V", &["HVBRKC_BUFGLS_5_H", "TVBRKC_BUFGLS_5"])
                .extra_int_out("O.UR.H", &["HVBRKC_BUFGLS_7_H", "TVBRKC_BUFGLS_7_H"])
                .extra_int_out("O.UR.V", &["HVBRKC_BUFGLS_6_H", "TVBRKC_BUFGLS_6"]);
            builder
                .xnode("CLKQC", naming, crd)
                .ref_xlat(crd.delta(1, 0), &[None, Some(0)], hvbrk)
                .bel(bel)
                .extract();
        }
    }
}

fn extract_clkq(builder: &mut IntBuilder) {
    let hvbrk = builder.ndb.get_tile_class_naming("LLVQ.CLB");
    for (naming, tkn) in [("CLKQ.B", "BCCBRK"), ("CLKQ.T", "TCCBRK")] {
        if let Some(&crd) = builder.rd.tiles_by_kind_name(tkn).first() {
            let bel = builder
                .bel_virtual(bels::CLKQ)
                .extra_wire("I.LL.H", &["BCCBRK_BUFGLS_3", "TCCBRK_BUFGLS_3"])
                .extra_wire("I.LL.V", &["BCCBRK_BUFGLS_2T", "TCCBRK_BUFGLS_2B"])
                .extra_wire("I.UL.H", &["BCCBRK_BUFGLS_8", "TCCBRK_BUFGLS_8"])
                .extra_wire("I.UL.V", &["BCCBRK_BUFGLS_1T", "TCCBRK_BUFGLS_1B"])
                .extra_wire("I.LR.H", &["BCCBRK_BUFGLS_4", "TCCBRK_BUFGLS_4"])
                .extra_wire("I.LR.V", &["BCCBRK_BUFGLS_5T", "TCCBRK_BUFGLS_5B"])
                .extra_wire("I.UR.H", &["BCCBRK_BUFGLS_7", "TCCBRK_BUFGLS_7"])
                .extra_wire("I.UR.V", &["BCCBRK_BUFGLS_6T", "TCCBRK_BUFGLS_6B"])
                .extra_int_out("O.LL.H.L", &["BCCBRK_BUFGLS_3L", "TCCBRK_BUFGLS_3L"])
                .extra_int_out("O.LL.V.L", &["BCCBRK_BUFGLS_2L", "TCCBRK_BUFGLS_2L"])
                .extra_int_out("O.UL.H.L", &["BCCBRK_BUFGLS_8L", "TCCBRK_BUFGLS_8L"])
                .extra_int_out("O.UL.V.L", &["BCCBRK_BUFGLS_1L", "TCCBRK_BUFGLS_1L"])
                .extra_int_out("O.LR.H.L", &["BCCBRK_BUFGLS_4L", "TCCBRK_BUFGLS_4L"])
                .extra_int_out("O.LR.V.L", &["BCCBRK_BUFGLS_5L", "TCCBRK_BUFGLS_5L"])
                .extra_int_out("O.UR.H.L", &["BCCBRK_BUFGLS_7L", "TCCBRK_BUFGLS_7L"])
                .extra_int_out("O.UR.V.L", &["BCCBRK_BUFGLS_6L", "TCCBRK_BUFGLS_6L"])
                .extra_int_out("O.LL.H.R", &["BCCBRK_BUFGLS_3R", "TCCBRK_BUFGLS_3R"])
                .extra_int_out("O.LL.V.R", &["BCCBRK_BUFGLS_2R", "TCCBRK_BUFGLS_2R"])
                .extra_int_out("O.UL.H.R", &["BCCBRK_BUFGLS_8R", "TCCBRK_BUFGLS_8R"])
                .extra_int_out("O.UL.V.R", &["BCCBRK_BUFGLS_1R", "TCCBRK_BUFGLS_1R"])
                .extra_int_out("O.LR.H.R", &["BCCBRK_BUFGLS_4R", "TCCBRK_BUFGLS_4R"])
                .extra_int_out("O.LR.V.R", &["BCCBRK_BUFGLS_5R", "TCCBRK_BUFGLS_5R"])
                .extra_int_out("O.UR.H.R", &["BCCBRK_BUFGLS_7R", "TCCBRK_BUFGLS_7R"])
                .extra_int_out("O.UR.V.R", &["BCCBRK_BUFGLS_6R", "TCCBRK_BUFGLS_6R"]);
            builder
                .xnode("CLKQ", naming, crd)
                .num_tiles(2)
                .ref_xlat(crd.delta(-1, 0), &[None, Some(0)], hvbrk)
                .ref_xlat(crd.delta(2, 0), &[None, Some(1)], hvbrk)
                .bel(bel)
                .extract();
        }
    }
}

pub fn make_int_db(rd: &Part) -> (IntDb, NamingDb) {
    let mut builder = IntBuilder::new(rd);

    for &slot in bels::SLOTS {
        builder.db.bel_slots.insert(slot.into());
    }

    let mut cnr_terms = CnrTerms {
        term_ll_w: vec![],
        term_lr_s: vec![],
        term_ul_n: vec![],
        term_ur_e: vec![],
    };

    fill_tie_wires(&mut builder);
    fill_single_wires(&mut builder);
    fill_double_wires(&mut builder);
    fill_io_double_wires(&mut builder, &mut cnr_terms);
    fill_quad_wires(&mut builder);
    fill_octal_wires(&mut builder);
    fill_io_octal_wires(&mut builder, &mut cnr_terms);
    fill_long_wires(&mut builder);
    fill_dec_wires(&mut builder);
    fill_clk_wires(&mut builder);
    let (imux_wires, imux_nw) = fill_imux_wires(&mut builder);
    fill_out_wires(&mut builder);

    if rd.family == "xc4000e" {
        fill_xc4000e_wirenames(&mut builder);
    }

    for tkn in [
        "CENTER", "BOT", "BOTS", "BOTSL", "BOTRR", "TOP", "TOPS", "TOPSL", "TOPRR", "LEFT",
        "LEFTS", "LEFTT", "LEFTSB", "LEFTF", "LEFTSF", "LEFTF1", "LEFTSF1", "RT", "RTS", "RTSB",
        "RTT", "RTF", "RTF1", "RTSF", "RTSF1", "UL", "UR", "LL", "LR",
    ] {
        builder.inject_node_type(tkn);
    }

    builder.extract_main_passes();

    let mut force_names = vec![];
    for (pin, tpin) in [("F2", "O_1"), ("G2", "O_2"), ("C2", "TXIN2")] {
        let w = builder.db.get_wire(&format!("IMUX.CLB.{pin}"));
        force_names.push((1, format!("CENTER_{pin}"), w));
        // force_names.push((2, format!("CENTER_{pin}L"), w));
        for kind in TOP_KINDS {
            force_names.push((1, format!("{kind}_{tpin}"), w));
        }
    }
    if builder.rd.family == "xc4000e" {
        for &(name, wire) in xc4000e_wires::XC4000E_WIRES {
            let xwire = match wire {
                "IMUX.CLB.F2.N" => "IMUX.CLB.F2",
                "IMUX.CLB.G2.N" => "IMUX.CLB.G2",
                "IMUX.CLB.C2.N" => "IMUX.CLB.C2",
                _ => continue,
            };
            force_names.push((1, name.to_string(), builder.db.get_wire(xwire)));
        }
    }

    extract_clb(&mut builder, &imux_wires, &imux_nw, &force_names);
    extract_bot(&mut builder, &imux_wires, &imux_nw, &force_names);
    extract_top(&mut builder, &imux_wires, &imux_nw);
    extract_rt(&mut builder, &imux_wires, &imux_nw);
    extract_left(&mut builder, &imux_wires, &imux_nw);
    extract_lr(&mut builder, &imux_wires, &imux_nw);
    extract_ur(&mut builder, &imux_wires, &imux_nw);
    extract_ll(&mut builder, &imux_wires, &imux_nw);
    extract_ul(&mut builder, &imux_wires, &imux_nw);

    if matches!(&*rd.family, "xc4000e" | "spartanxl") {
        extract_llh(&mut builder);
        extract_llv(&mut builder);
    } else {
        extract_llhc(&mut builder);
        extract_llhq(&mut builder);
        extract_llvc(&mut builder);
        extract_llvq(&mut builder);
        if rd.family != "xc4000xv" {
            extract_clkc(&mut builder);
            extract_clkqc(&mut builder);
        } else {
            extract_clkq(&mut builder);
        }
    }

    let mut llhq_w = builder.db.conn_classes.get("MAIN.W").unwrap().1.clone();
    let mut llhq_e = builder.db.conn_classes.get("MAIN.E").unwrap().1.clone();
    let mut llhq_io_w = builder.db.conn_classes.get("MAIN.W").unwrap().1.clone();
    let mut llhq_io_e = builder.db.conn_classes.get("MAIN.E").unwrap().1.clone();
    let mut llhc_w = builder.db.conn_classes.get("MAIN.W").unwrap().1.clone();
    let mut llhc_e = builder.db.conn_classes.get("MAIN.E").unwrap().1.clone();
    let mut llvq_s = builder.db.conn_classes.get("MAIN.S").unwrap().1.clone();
    let mut llvq_n = builder.db.conn_classes.get("MAIN.N").unwrap().1.clone();
    let mut llvc_s = builder.db.conn_classes.get("MAIN.S").unwrap().1.clone();
    let mut llvc_n = builder.db.conn_classes.get("MAIN.N").unwrap().1.clone();

    for (w, wn, _) in &builder.db.wires {
        if wn.starts_with("LONG") {
            if wn != "LONG.H2" && wn != "LONG.H3" {
                llhq_w.wires.remove(w);
                llhq_e.wires.remove(w);
            }
            llhq_io_w.wires.remove(w);
            llhq_io_e.wires.remove(w);
            llhc_w.wires.remove(w);
            llhc_e.wires.remove(w);
            llvq_s.wires.remove(w);
            llvq_n.wires.remove(w);
            llvc_s.wires.remove(w);
            llvc_n.wires.remove(w);
        }
        if wn.starts_with("DEC") || wn == "ECLK.H" || wn.starts_with("BUFGE.H") {
            llhc_w.wires.remove(w);
            llhc_e.wires.remove(w);
            llvc_s.wires.remove(w);
            llvc_n.wires.remove(w);
        }
        if !matches!(&*rd.family, "xc4000e" | "spartanxl") {
            if wn.starts_with("GCLK") {
                llvc_s.wires.remove(w);
                llvc_n.wires.remove(w);
            }
            if wn == "VCLK" || wn == "ECLK.V" {
                llvc_s.wires.remove(w);
                llvc_n.wires.remove(w);
                llvq_s.wires.remove(w);
                llvq_n.wires.remove(w);
            }
        }
        if rd.family == "xc4000xv" && wn.starts_with("BUFGLS") {
            llhc_w.wires.remove(w);
            llhc_e.wires.remove(w);
            llhq_w.wires.remove(w);
            llhq_e.wires.remove(w);
        }
    }

    builder.db.conn_classes.insert("LLHC.W".to_owned(), llhc_w);
    builder.db.conn_classes.insert("LLHC.E".to_owned(), llhc_e);
    builder.db.conn_classes.insert("LLVC.S".to_owned(), llvc_s);
    builder.db.conn_classes.insert("LLVC.N".to_owned(), llvc_n);

    if !matches!(&*rd.family, "xc4000e" | "spartanxl") {
        builder.db.conn_classes.insert("LLHQ.W".to_owned(), llhq_w);
        builder.db.conn_classes.insert("LLHQ.E".to_owned(), llhq_e);
        builder.db.conn_classes.insert("LLHQ.IO.W".to_owned(), llhq_io_w);
        builder.db.conn_classes.insert("LLHQ.IO.E".to_owned(), llhq_io_e);
        builder.db.conn_classes.insert("LLVQ.S".to_owned(), llvq_s);
        builder.db.conn_classes.insert("LLVQ.N".to_owned(), llvq_n);
    }

    let mut tclb_n = builder.db.conn_classes.get("MAIN.N").unwrap().1.clone();
    for (wt, wf) in [
        ("OUT.CLB.FX.S", "OUT.BT.IOB0.I2"),
        ("OUT.CLB.FXQ.S", "OUT.BT.IOB1.I2"),
    ] {
        let wt = builder.db.get_wire(wt);
        let wf = builder.db.get_wire(wf);
        tclb_n.wires.insert(wt, ConnectorWire::Pass(wf));
    }
    builder.db.conn_classes.insert("TCLB.N".to_owned(), tclb_n);

    let mut lclb_w = builder.db.conn_classes.get("MAIN.W").unwrap().1.clone();
    for (wt, wf) in [
        ("OUT.CLB.GY.E", "OUT.LR.IOB1.I2"),
        ("OUT.CLB.GYQ.E", "OUT.LR.IOB0.I2"),
    ] {
        let wt = builder.db.get_wire(wt);
        let wf = builder.db.get_wire(wf);
        lclb_w.wires.insert(wt, ConnectorWire::Pass(wf));
    }
    builder.db.conn_classes.insert("LCLB.W".to_owned(), lclb_w);

    for (name, dir, wires) in [
        ("CNR.LL.W", Dir::W, cnr_terms.term_ll_w),
        ("CNR.LR.S", Dir::S, cnr_terms.term_lr_s),
        ("CNR.UL.N", Dir::N, cnr_terms.term_ul_n),
        ("CNR.UR.E", Dir::E, cnr_terms.term_ur_e),
    ] {
        let term = ConnectorClass {
            slot: builder.term_slots[dir],
            wires: wires
                .into_iter()
                .map(|(a, b)| (a, ConnectorWire::Reflect(b)))
                .collect(),
        };
        builder.db.conn_classes.insert_new(name.to_string(), term);
    }

    builder.build()
}
