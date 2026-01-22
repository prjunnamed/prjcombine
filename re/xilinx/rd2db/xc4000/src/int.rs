use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{
        BelInfo, BelSlotId, IntDb, ProgBuf, SwitchBoxItem, TileClassId, TileWireCoord, WireSlotId,
    },
    dir::Dir,
};
use prjcombine_re_xilinx_naming::db::{NamingDb, TileClassNamingId};
use prjcombine_re_xilinx_rawdump::{Coord, Part};
use prjcombine_types::bsdata::PolTileBit;
use prjcombine_xc2000::xc4000::{self as defs, bslots, wires, xc4000::tcls};

use prjcombine_re_xilinx_rd2db_interconnect::{ExtrBelInfo, IntBuilder, PipMode};

const BOT_KINDS: [&str; 4] = ["BOT", "BOTS", "BOTSL", "BOTRR"];
const TOP_KINDS: [&str; 4] = ["TOP", "TOPS", "TOPSL", "TOPRR"];
const LEFT_KINDS: [&str; 8] = [
    "LEFT", "LEFTS", "LEFTT", "LEFTSB", "LEFTF", "LEFTSF", "LEFTF1", "LEFTSF1",
];
const RT_KINDS: [&str; 8] = ["RT", "RTS", "RTSB", "RTT", "RTF", "RTF1", "RTSF", "RTSF1"];

mod xc4000e_wires;

fn fill_tie_wires(builder: &mut IntBuilder) {
    builder.wire_names(
        wires::TIE_0,
        &["CENTER_TIE", "LR_TIE", "TVIBRK_TIE", "LHIBRK_TIE"],
    );
    for k in BOT_KINDS {
        builder.extra_name(format!("{k}_PULLDN"), wires::TIE_0);
    }
    for k in RT_KINDS {
        builder.extra_name(format!("{k}_TIE"), wires::TIE_0);
    }
}

fn fill_single_wires(builder: &mut IntBuilder) {
    for i in 0..8 {
        let ii = i + 1;
        builder.wire_names(wires::SINGLE_H[i], &[format!("CENTER_H{ii}R")]);
        for k in BOT_KINDS.into_iter().chain(RT_KINDS).chain(["LR"]) {
            builder.extra_name(format!("{k}_H{ii}R"), wires::SINGLE_H[i]);
        }
        for k in LEFT_KINDS.into_iter().chain(["LL"]) {
            builder.extra_name(format!("{k}_H{ii}"), wires::SINGLE_H[i]);
        }
        builder.wire_names(wires::SINGLE_H_E[i], &[format!("CENTER_H{ii}")]);
        for k in BOT_KINDS.into_iter().chain(RT_KINDS).chain(["LR"]) {
            builder.extra_name(format!("{k}_H{ii}"), wires::SINGLE_H_E[i]);
        }
    }

    for i in 0..8 {
        let ii = i + 1;
        builder.wire_names(wires::SINGLE_V[i], &[format!("CENTER_V{ii}")]);
        for k in BOT_KINDS
            .into_iter()
            .chain(TOP_KINDS)
            .chain(RT_KINDS)
            .chain(["LR", "UR"])
        {
            builder.extra_name(format!("{k}_V{ii}"), wires::SINGLE_V[i]);
        }
        builder.wire_names(wires::SINGLE_V_S[i], &[format!("CENTER_V{ii}T")]);
        for k in BOT_KINDS.into_iter().chain(RT_KINDS).chain(["LR"]) {
            builder.extra_name(format!("{k}_V{ii}T"), wires::SINGLE_V_S[i]);
        }
    }
}

fn fill_double_wires(builder: &mut IntBuilder) {
    for (hv, rt0, rt2, w0, w1, w2) in [
        (
            'H',
            "R",
            "",
            wires::DOUBLE_H0,
            wires::DOUBLE_H1,
            wires::DOUBLE_H2,
        ),
        (
            'V',
            "",
            "T",
            wires::DOUBLE_V0,
            wires::DOUBLE_V1,
            wires::DOUBLE_V2,
        ),
    ] {
        for i in 0..2 {
            let ii = [2, 3][i];
            builder.wire_names(w0[i], &[format!("CENTER_D{hv}{ii}{rt0}")]);
            for k in BOT_KINDS.into_iter().chain(RT_KINDS).chain(["LR"]) {
                builder.extra_name(format!("{k}_D{hv}{ii}{rt0}"), w0[i]);
            }
            if hv == 'H' {
                for k in LEFT_KINDS.into_iter().chain(["LL"]) {
                    builder.extra_name(format!("{k}_D{hv}{ii}", ii = [1, 4][i]), w0[i]);
                }
            }
            if hv == 'V' {
                for k in TOP_KINDS.into_iter().chain(["UR"]) {
                    builder.extra_name(format!("{k}_D{hv}{ii}"), w0[i]);
                }
            }
            let ii = [1, 4][i];
            builder.wire_names(w1[i], &[format!("CENTER_D{hv}{ii}")]);
            for k in BOT_KINDS.into_iter().chain(RT_KINDS).chain(["LR"]) {
                builder.extra_name(format!("{k}_D{hv}{ii}"), w1[i]);
            }
            if hv == 'V' {
                for k in TOP_KINDS.into_iter().chain(["UR"]) {
                    builder.extra_name(format!("{k}_D{hv}{ii}"), w1[i]);
                }
            } else {
                for k in LEFT_KINDS.into_iter().chain(["LL"]) {
                    builder.extra_name(format!("{k}_D{hv}{ii}", ii = [2, 3][i]), w1[i]);
                }
            }
            let ii = [2, 3][i];
            builder.wire_names(w2[i], &[format!("CENTER_D{hv}{ii}{rt2}")]);
            for k in BOT_KINDS.into_iter().chain(RT_KINDS).chain(["LR"]) {
                builder.extra_name(format!("{k}_D{hv}{ii}{rt2}"), w2[i]);
            }
        }
    }
}

fn fill_io_double_wires(builder: &mut IntBuilder) {
    for i in 0..4 {
        let ia = i * 2 + 1;
        let ib = i * 2 + 2;
        for k in BOT_KINDS {
            builder.extra_name(format!("{k}_BDH{ia}"), wires::DOUBLE_IO_S0[i]);
            builder.extra_name(format!("{k}_BDH{ib}"), wires::DOUBLE_IO_S1[i]);
            builder.extra_name(format!("{k}_BDH{ia}L"), wires::DOUBLE_IO_S2[i]);
        }
        for k in TOP_KINDS {
            builder.extra_name(format!("{k}_DH{ia}L"), wires::DOUBLE_IO_N0[i]);
            builder.extra_name(format!("{k}_DH{ib}"), wires::DOUBLE_IO_N1[i]);
            builder.extra_name(format!("{k}_DH{ia}"), wires::DOUBLE_IO_N2[i]);
        }
        for k in LEFT_KINDS {
            builder.extra_name(format!("{k}_LDV{ia}"), wires::DOUBLE_IO_W0[i]);
            builder.extra_name(format!("{k}_LDV{ib}"), wires::DOUBLE_IO_W1[i]);
            builder.extra_name(format!("{k}_LDV{ia}T"), wires::DOUBLE_IO_W2[i]);
        }
        for k in RT_KINDS {
            builder.extra_name(format!("{k}_RDV{ia}T"), wires::DOUBLE_IO_E0[i]);
            builder.extra_name(format!("{k}_RDV{ib}"), wires::DOUBLE_IO_E1[i]);
            builder.extra_name(format!("{k}_RDV{ia}"), wires::DOUBLE_IO_E2[i]);
        }
        builder.extra_name(format!("LL_D{ib}B"), wires::DOUBLE_IO_S0[i]);
        builder.extra_name(format!("LL_D{ia}"), wires::DOUBLE_IO_W1[i]);
        builder.extra_name(format!("LL_D{ib}"), wires::DOUBLE_IO_W2[i]);
        builder.extra_name(format!("UL_D{ia}"), wires::DOUBLE_IO_N1[i]);
        builder.extra_name(format!("UL_D{ib}"), wires::DOUBLE_IO_N2[i]);
        builder.extra_name(format!("LR_RDV{ib}"), wires::DOUBLE_IO_E0[i]);
        builder.extra_name(format!("LR_RDV{ia}"), wires::DOUBLE_IO_E1[i]);
        builder.extra_name(format!("LR_BDH{ib}"), wires::DOUBLE_IO_S1[i]);
        builder.extra_name(format!("LR_BDH{ia}"), wires::DOUBLE_IO_S2[i]);
        builder.extra_name(format!("UR_D{ia}L"), wires::DOUBLE_IO_N0[i]);
        builder.extra_name(format!("UR_D{ib}"), wires::DOUBLE_IO_E1[i]);
        builder.extra_name(format!("UR_D{ia}"), wires::DOUBLE_IO_E2[i]);
    }

    for (i, n) in ["DMUX_OUTER", "DMUX_INNER"].into_iter().enumerate() {
        builder.wire_names(wires::DBUF_IO_H[i], &[format!("LR_B{n}")]);
        for k in BOT_KINDS.into_iter().chain(TOP_KINDS).chain(["UR"]) {
            builder.extra_name(format!("{k}_{n}"), wires::DBUF_IO_H[i]);
        }
    }
    for (i, n) in ["DMUX_OUTER", "DMUX_INNER"].into_iter().enumerate() {
        builder.wire_names(wires::DBUF_IO_V[i], &[format!("LR_R{n}")]);
        for k in LEFT_KINDS.into_iter().chain(RT_KINDS).chain(["LL"]) {
            builder.extra_name(format!("{k}_{n}"), wires::DBUF_IO_V[i]);
        }
    }
}

fn fill_quad_wires(builder: &mut IntBuilder) {
    if matches!(&*builder.rd.family, "xc4000e" | "spartanxl") {
        return;
    }

    for (hv, rt0, rt4, w0, w1, w2, w3, w4) in [
        (
            'H',
            "R",
            "",
            wires::QUAD_H0,
            wires::QUAD_H1,
            wires::QUAD_H2,
            wires::QUAD_H3,
            wires::QUAD_H4,
        ),
        (
            'V',
            "",
            "T",
            wires::QUAD_V0,
            wires::QUAD_V1,
            wires::QUAD_V2,
            wires::QUAD_V3,
            wires::QUAD_V4,
        ),
    ] {
        for i in 0..3 {
            let ii = 4 * i + 4;
            builder.wire_names(w0[i], &[format!("CENTER_Q{hv}{ii}{rt0}")]);
            for k in BOT_KINDS
                .into_iter()
                .chain(TOP_KINDS)
                .chain(RT_KINDS)
                .chain(["LR", "UR"])
            {
                builder.extra_name(format!("{k}_Q{hv}{ii}{rt0}"), w0[i]);
            }
            for k in LEFT_KINDS.into_iter().chain(["LL"]) {
                builder.extra_name(format!("{k}_Q{hv}{ii}"), w0[i]);
            }
            for (j, w) in [(1, w1), (2, w2), (3, w3)] {
                let ii = if hv == 'H' { 4 * i + 4 - j } else { 4 * i + j };
                builder.wire_names(w[i], &[format!("CENTER_Q{hv}{ii}")]);
                for k in BOT_KINDS
                    .into_iter()
                    .chain(TOP_KINDS)
                    .chain(LEFT_KINDS)
                    .chain(RT_KINDS)
                    .chain(["LL", "LR", "UR"])
                {
                    builder.extra_name(format!("{k}_Q{hv}{ii}"), w[i]);
                }
            }
            builder.wire_names(w4[i], &[format!("CENTER_Q{hv}{ii}{rt4}")]);
            for k in BOT_KINDS
                .into_iter()
                .chain(TOP_KINDS)
                .chain(RT_KINDS)
                .chain(["LR", "UR"])
            {
                builder.extra_name(format!("{k}_Q{hv}{ii}{rt4}"), w4[i]);
            }
        }
    }

    for i in 0..3 {
        let ii = i * 4 + 4;
        builder.wire_names(wires::QBUF[i], &[format!("CENTER_QBUF{ii}")]);
        for k in BOT_KINDS.into_iter().chain(RT_KINDS).chain(["LR"]) {
            builder.extra_name(format!("{k}_QBUF{ii}"), wires::QBUF[i]);
        }
    }
}

fn fill_octal_wires(builder: &mut IntBuilder) {
    if builder.rd.family != "xc4000xv" {
        return;
    }

    builder.wire_names(wires::OCTAL_H[0], &["VHIBRK_OH1R", "LHIBRK_OH8"]);
    for j in 1..8 {
        let ii = 9 - j;
        builder.wire_names(
            wires::OCTAL_H[j],
            &[
                format!("VHIBRK_OH{ii}"),
                format!("RHIBRK_OH{ii}"),
                format!("LHIBRK_OH{ii}", ii = ii - 1),
            ],
        );
    }
    builder.wire_names(wires::OCTAL_H[8], &["VHIBRK_OH1", "RHIBRK_OH1"]);

    builder.wire_names(wires::OCTAL_V[0], &["VHIBRK_OV8B", "TVIBRK_OV8"]);
    for j in 1..8 {
        builder.wire_names(
            wires::OCTAL_V[j],
            &[
                format!("VHIBRK_OV{j}"),
                format!("TVIBRK_OV{j}"),
                format!("BVIBRK_OV{ii}", ii = if j == 1 { 8 } else { j - 1 }),
            ],
        );
    }
    builder.wire_names(wires::OCTAL_V[8], &["VHIBRK_OV8T", "BVIBRK_OV7"]);
}

fn fill_io_octal_wires(builder: &mut IntBuilder) {
    if matches!(&*builder.rd.family, "xc4000e" | "spartanxl") {
        return;
    }

    for k in BOT_KINDS {
        builder.extra_name(format!("{k}_OH8R"), wires::OCTAL_IO_S[0]);
        for i in 0..7 {
            builder.extra_name(format!("{k}_OH{ii}", ii = 7 - i), wires::OCTAL_IO_S[1 + i]);
        }
        builder.extra_name(format!("{k}_OH8"), wires::OCTAL_IO_S[8]);
    }
    for k in TOP_KINDS {
        builder.extra_name(format!("{k}_OH8"), wires::OCTAL_IO_N[0]);
        for i in 1..8 {
            builder.extra_name(format!("{k}_OH{i}"), wires::OCTAL_IO_N[i]);
        }
        builder.extra_name(format!("{k}_OH8R"), wires::OCTAL_IO_N[8]);
    }
    for k in LEFT_KINDS {
        builder.extra_name(format!("{k}_OV8"), wires::OCTAL_IO_W[0]);
        for i in 1..8 {
            builder.extra_name(format!("{k}_OV{i}"), wires::OCTAL_IO_W[i]);
        }
        builder.extra_name(format!("{k}_OV8T"), wires::OCTAL_IO_W[8]);
    }

    for k in RT_KINDS {
        builder.extra_name(format!("{k}_OV8T"), wires::OCTAL_IO_E[0]);
        for i in 0..7 {
            builder.extra_name(format!("{k}_OV{ii}", ii = 7 - i), wires::OCTAL_IO_E[1 + i]);
        }
        builder.extra_name(format!("{k}_OV8"), wires::OCTAL_IO_E[8]);
    }
    for i in 1..8 {
        builder.extra_name(format!("LR_O{i}"), wires::OCTAL_IO_E[i]);
    }
    builder.extra_name("LR_O8T", wires::OCTAL_IO_E[0]);
    for i in 1..8 {
        builder.extra_name(format!("UR_O{i}"), wires::OCTAL_IO_N[i]);
    }
    builder.extra_name("UR_O8", wires::OCTAL_IO_N[0]);
    for i in 0..7 {
        builder.extra_name(format!("LL_O{ii}", ii = 7 - i), wires::OCTAL_IO_S[i]);
    }
    builder.extra_name("LL_O8", wires::OCTAL_IO_S[7]);
    for i in 1..8 {
        builder.extra_name(format!("UL_O{i}"), wires::OCTAL_IO_W[i]);
    }
    builder.extra_name("UL_O8", wires::OCTAL_IO_W[0]);
}

fn fill_long_wires(builder: &mut IntBuilder) {
    for i in 0..6 {
        let ii = i + 1;
        builder.wire_names(wires::LONG_H[i], &[format!("CENTER_HLL{ii}")]);
        for k in BOT_KINDS
            .into_iter()
            .chain(TOP_KINDS)
            .chain(LEFT_KINDS)
            .chain(RT_KINDS)
            .chain(["LL", "UL", "LR", "UR"])
        {
            builder.extra_name(format!("{k}_HLL{ii}"), wires::LONG_H[i]);
        }
        if matches!(&*builder.rd.family, "xc4000xla" | "xc4000xv" | "spartanxl")
            && matches!(i, 2 | 3)
        {
            builder.mark_permabuf(wires::LONG_H_BUF[i]);
            builder.wire_names(wires::LONG_H_BUF[i], &[format!("CENTER_HLL{ii}_LOC")]);
            for k in LEFT_KINDS.into_iter().chain(RT_KINDS) {
                builder.extra_name(format!("{k}_HLL{ii}_LOC"), wires::LONG_H_BUF[i]);
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
        builder.wire_names(wires::LONG_V[i], &[format!("CENTER_VLL{ii}")]);
        for k in BOT_KINDS
            .into_iter()
            .chain(TOP_KINDS)
            .chain(RT_KINDS)
            .chain(["LR", "UR"])
        {
            builder.extra_name(format!("{k}_VLL{ii}"), wires::LONG_V[i]);
        }
        if matches!(i, 7 | 9) {
            for (n, sub) in [
                (format!("HVBRK_VLL{ii}_EXCL"), 0),
                (format!("HVBRK_VLL{ii}T_EXCL"), 1),
                (format!("RHVBRK_VLL{ii}_EXCL"), 1),
                (format!("RHVBRK_VLL{ii}B_EXCL"), 0),
                (format!("RVRBRK_VLL{ii}_EXCL"), 1),
                (format!("RVRBRK_VLL{ii}B_EXCL"), 0),
            ] {
                builder.alt_name_sub(n, sub, wires::LONG_V[i]);
            }
        }
    }
    for i in 0..4 {
        let ii = i + 1;
        for k in BOT_KINDS.into_iter().chain(["LL", "LR"]) {
            builder.extra_name(format!("{k}_BHLL{ii}"), wires::LONG_IO_H[i]);
        }
        for k in TOP_KINDS.into_iter().chain(["UL", "UR"]) {
            builder.extra_name(format!("{k}_THLL{ii}"), wires::LONG_IO_H[i]);
        }
        if !matches!(&*builder.rd.family, "xc4000e" | "spartanxl") {
            for k in BOT_KINDS.into_iter().chain(["LR"]) {
                builder.alt_name(format!("{k}_BHLL{ii}_EXCL"), wires::LONG_IO_H[i]);
            }
            for k in TOP_KINDS.into_iter().chain(["UR"]) {
                builder.alt_name(format!("{k}_THLL{ii}_EXCL"), wires::LONG_IO_H[i]);
            }
        }
    }
    for i in 0..4 {
        let ii = i + 1;
        for k in LEFT_KINDS.into_iter().chain(["LL", "UL"]) {
            builder.extra_name(format!("{k}_LVLL{ii}"), wires::LONG_IO_V[i]);
        }
        for k in RT_KINDS.into_iter().chain(["LR", "UR"]) {
            builder.extra_name(format!("{k}_RVLL{ii}"), wires::LONG_IO_V[i]);
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
        builder.wire_names(
            wires::DEC_H[i],
            &[
                format!("LL_BTX{ii}"),
                format!("LR_BTX{ii}"),
                format!("UL_TTX{tii}"),
                format!("UR_TTX{tii}"),
            ],
        );
        for k in BOT_KINDS {
            builder.extra_name(format!("{k}_TX{ii}"), wires::DEC_H[i]);
        }
        for k in TOP_KINDS {
            builder.extra_name(format!("{k}_TTX{tii}"), wires::DEC_H[i]);
        }
    }
    for i in 0..4 {
        let ii = 4 - i;
        let lii = i + 1;
        builder.wire_names(
            wires::DEC_V[i],
            &[
                format!("LL_LTX{lii}"),
                format!("UL_LTX{lii}"),
                format!("LR_RTX{ii}"),
                format!("UR_RTX{ii}"),
            ],
        );
        for k in LEFT_KINDS {
            builder.extra_name(format!("{k}_LTX{lii}"), wires::DEC_V[i]);
        }

        for k in RT_KINDS {
            builder.extra_name(format!("{k}_RTX{ii}"), wires::DEC_V[i]);
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
        builder.wire_names(wires::GCLK[i], &[format!("CENTER_K{ii}")]);
        for k in BOT_KINDS
            .into_iter()
            .chain(TOP_KINDS)
            .chain(LEFT_KINDS)
            .chain(RT_KINDS)
            .chain(["LL", "UL", "LR", "UR"])
        {
            builder.extra_name(format!("{k}_K{ii}"), wires::GCLK[i]);
        }
    }

    if builder.rd.family == "spartanxl" {
        for (i, name) in [
            "UL_PRI_CLOCK",
            "LL_SEC_CLOCK",
            "LL_PRI_CLOCK",
            "LR_SEC_CLK",
            "LR_PRI_CLK",
            "UR_CLOCK_6",
            "UR_PRI_CLK",
            "UL_SEC_CLOCK",
        ]
        .into_iter()
        .enumerate()
        {
            builder.extra_name(name, wires::BUFGLS[i]);
        }

        for tkn in ["CLKL", "CLKR", "CLKH"] {
            for i in 0..4 {
                builder.extra_name(format!("{tkn}_K{ii}", ii = i + 1), wires::GCLK[i]);
            }
            for i in 0..8 {
                builder.extra_name(format!("{tkn}_CLOCK_{ii}", ii = i + 1), wires::BUFGLS[i]);
            }
        }
    }
    if !matches!(&*builder.rd.family, "xc4000e" | "spartanxl") {
        if builder.rd.family == "xc4000xv" {
            for (i, (name_a, name_b, name_c)) in [
                ("UL_BUFGLS_1_B", "BCCBRK_BUFGLS_1T", "TCCBRK_BUFGLS_1B"),
                ("LL_BUFGLS_2_T", "BCCBRK_BUFGLS_2T", "TCCBRK_BUFGLS_2B"),
                ("LL_BUFGLS_3_R", "BCCBRK_BUFGLS_3", "TCCBRK_BUFGLS_3"),
                ("LR_BUFGLS_4_L", "BCCBRK_BUFGLS_4", "TCCBRK_BUFGLS_4"),
                ("LR_BUFGLS_5_T", "BCCBRK_BUFGLS_5T", "TCCBRK_BUFGLS_5B"),
                ("UR_BUFGLS_6_B", "BCCBRK_BUFGLS_6T", "TCCBRK_BUFGLS_6B"),
                ("UR_BUFGLS_7_L", "BCCBRK_BUFGLS_7", "TCCBRK_BUFGLS_7"),
                ("UL_BUFGLS_8_R", "BCCBRK_BUFGLS_8", "TCCBRK_BUFGLS_8"),
            ]
            .into_iter()
            .enumerate()
            {
                builder.extra_name(name_a, wires::BUFGLS[i]);
                builder.extra_name(name_b, wires::BUFGLS[i]);
                builder.extra_name(name_c, wires::BUFGLS[i]);
            }
        } else {
            for (i, name) in [
                "CLKC_BUFGLS_1",
                "CLKC_BUFGLS_2",
                "LL_BUFGLS_3_R",
                "LR_BUFGLS_4_L",
                "CLKC_BUFGLS_5",
                "CLKC_BUFGLS_6",
                "UR_BUFGLS_7_L",
                "UL_BUFGLS_8_R",
            ]
            .into_iter()
            .enumerate()
            {
                builder.extra_name(name, wires::BUFGLS[i]);
            }
            for (i, (name_o, name_i)) in [
                ("TVBRKC_BUFGLS_1_H", "TVBRKC_BUFGLS_1"),
                ("TVBRKC_BUFGLS_2", "TVBRKC_BUFGLS_2_B"),
                ("TVBRKC_BUFGLS_3_H", "TVBRKC_BUFGLS_3"),
                ("TVBRKC_BUFGLS_4", "TVBRKC_BUFGLS_4_B"),
                ("TVBRKC_BUFGLS_5", "TVBRKC_BUFGLS_5_B"),
                ("TVBRKC_BUFGLS_6", "TVBRKC_BUFGLS_6_B"),
                ("TVBRKC_BUFGLS_7_H", "TVBRKC_BUFGLS_7"),
                ("TVBRKC_BUFGLS_8_H", "TVBRKC_BUFGLS_8"),
            ]
            .into_iter()
            .enumerate()
            {
                builder.extra_name(name_o, wires::BUFGLS_H[i]);
                builder.extra_name(name_i, wires::BUFGLS[i]);
            }
            for i in 0..8 {
                builder.extra_name(format!("HVBRKC_BUFGLS_{ii}", ii = i + 1), wires::BUFGLS[i]);
                builder.extra_name(
                    format!("HVBRKC_BUFGLS_{ii}_H", ii = i + 1),
                    wires::BUFGLS_H[i],
                );
            }
        }

        builder.wire_names(wires::VCLK, &["CENTER_KX"]);
        for k in BOT_KINDS
            .into_iter()
            .chain(TOP_KINDS)
            .chain(RT_KINDS)
            .chain(["LL", "LR", "UR"])
        {
            builder.extra_name(format!("{k}_KX"), wires::VCLK);
        }

        builder.wire_names(wires::ECLK_V, &["LL_KX", "UL_KX", "LR_LRKX", "UR_URKX"]);
        for k in RT_KINDS {
            builder.extra_name(format!("{k}_R_KX"), wires::ECLK_V);
        }
        for k in LEFT_KINDS {
            builder.extra_name(format!("{k}_L_KX"), wires::ECLK_V);
        }

        builder.wire_names(wires::ECLK_H, &["LR_FCLK", "UR_FCLK", "LL_FCLK", "UL_FCLK"]);
        for k in BOT_KINDS.into_iter().chain(TOP_KINDS) {
            builder.extra_name(format!("{k}_FCLK"), wires::ECLK_H);
        }

        builder.wire_names(
            wires::BUFGE_H,
            &[
                "LR_BUFGE_4_L",
                "UR_BUFGE_7_L",
                "LL_BUFGE_3_R",
                "UL_BUFGE_8_R",
            ],
        );
        for k in BOT_KINDS {
            builder.extra_name(format!("{k}_BUFGE_3_4"), wires::BUFGE_H);
        }
        for k in TOP_KINDS {
            builder.extra_name(format!("{k}_BUFGE_7_8"), wires::BUFGE_H);
        }

        builder.wire_names(
            wires::BUFGE_V[0],
            &[
                "LR_BUFGE_5_6",
                "LL_BUFGE_1_2",
                "LHVBRK_BUFGE_2",
                "LVLBRK_BUFGE_2",
                "RHVBRK_BUFGE_5",
                "RVRBRK_BUFGE_5",
            ],
        );
        builder.wire_names(
            wires::BUFGE_V[1],
            &[
                "UR_BUFGE_5_6",
                "UL_BUFGE_1_2",
                "LHVBRK_BUFGE_1",
                "LVLBRK_BUFGE_1",
                "RHVBRK_BUFGE_6",
                "RVRBRK_BUFGE_6",
            ],
        );

        for i in 0..8 {
            let ii = i + 1;
            for n in [
                format!("HVBRK_BUFGLS_{ii}"),
                format!("LHVBRK_BUFGLS_{ii}"),
                format!("LVLBRK_BUFGLS_{ii}"),
                format!("RHVBRK_BUFGLS_{ii}"),
                format!("RVRBRK_BUFGLS_{ii}"),
            ] {
                builder.extra_name_sub(n, 1, wires::BUFGLS_H[i]);
            }
        }
    }
}

fn fill_imux_wires(builder: &mut IntBuilder) -> (Vec<WireSlotId>, Vec<TileWireCoord>) {
    let mut imux_wires = vec![];
    let mut imux_nw = vec![];
    for (w, pin, opin) in [
        (wires::IMUX_CLB_F1, "F1", "O_2"),
        (wires::IMUX_CLB_G1, "G1", "O_1"),
        (wires::IMUX_CLB_C1, "C1", "TXIN2"),
    ] {
        builder.wire_names(w, &[format!("CENTER_{pin}")]);
        imux_wires.push(w);
        imux_nw.push(TileWireCoord::new_idx(0, w));
        for &k in &RT_KINDS {
            builder.extra_name(format!("{k}_{opin}"), w);
        }
    }
    for (w, ww, pin, opin) in [
        (wires::IMUX_CLB_F2, wires::IMUX_CLB_F2_N, "F2", "O_1"),
        (wires::IMUX_CLB_G2, wires::IMUX_CLB_G2_N, "G2", "O_2"),
        (wires::IMUX_CLB_C2, wires::IMUX_CLB_C2_N, "C2", "TXIN2"),
    ] {
        builder.wire_names(w, &[format!("CENTER_{pin}T")]);
        builder.wire_names(ww, &[format!("CENTER_{pin}")]);
        imux_wires.push(w);
        imux_nw.push(TileWireCoord::new_idx(0, w));
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
    for (w, ww, pin, opin) in [
        (wires::IMUX_CLB_F3, wires::IMUX_CLB_F3_W, "F3", "O_2"),
        (wires::IMUX_CLB_G3, wires::IMUX_CLB_G3_W, "G3", "O_1"),
        (wires::IMUX_CLB_C3, wires::IMUX_CLB_C3_W, "C3", "TXIN2"),
    ] {
        builder.wire_names(w, &[format!("CENTER_{pin}L")]);
        builder.wire_names(ww, &[format!("CENTER_{pin}")]);
        imux_wires.push(w);
        imux_nw.push(TileWireCoord::new_idx(0, w));
        imux_wires.push(ww);
        for &k in &RT_KINDS {
            builder.extra_name(format!("{k}_{pin}L"), w);
        }
        for &k in &LEFT_KINDS {
            builder.extra_name(format!("{k}_{opin}"), ww);
        }
    }
    for (w, pin, opin, xname) in [
        (wires::IMUX_CLB_F4, "F4", "O_1", "LR_HZ1"),
        (wires::IMUX_CLB_G4, "G4", "O_2", "LR_HZ3"),
        (wires::IMUX_CLB_C4, "C4", "TXIN2", "LR_HZ2"),
    ] {
        builder.wire_names(w, &[format!("CENTER_{pin}")]);
        imux_wires.push(w);
        imux_nw.push(TileWireCoord::new_idx(0, w));
        for k in RT_KINDS {
            builder.extra_name(format!("{k}_{pin}"), w);
        }
        builder.extra_name(xname, w);
        for k in BOT_KINDS {
            builder.extra_name(format!("{k}_{opin}"), w);
        }
    }
    {
        builder.wire_names(wires::IMUX_CLB_K, &["CENTER_K"]);
        imux_wires.push(wires::IMUX_CLB_K);
        imux_nw.push(TileWireCoord::new_idx(0, wires::IMUX_CLB_K));
    }

    for i in 0..2 {
        let ii = i + 3;
        for (w, pin) in [(wires::IMUX_TBUF_I[i], "I"), (wires::IMUX_TBUF_T[i], "TS")] {
            builder.wire_names(w, &[format!("CENTER_TBUF{ii}{pin}")]);
            for k in LEFT_KINDS.into_iter().chain(RT_KINDS) {
                builder.extra_name(format!("{k}_TBUF{ii}{pin}"), w);
            }
            imux_wires.push(w);
            imux_nw.push(TileWireCoord::new_idx(0, w));
        }
    }

    for i in 0..2 {
        for (w, pin) in [
            (wires::IMUX_IO_O1[i], "O1"),
            (wires::IMUX_IO_OK[i], "OK"),
            (wires::IMUX_IO_IK[i], "IK"),
            (wires::IMUX_IO_T[i], "TS"),
        ] {
            let apin = if pin == "O1" { "CE" } else { pin };
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
            imux_nw.push(TileWireCoord::new_idx(0, w));
        }
    }

    if builder.rd.family != "xc4000e" {
        for k in BOT_KINDS {
            builder.extra_name(format!("{k}_COUT"), wires::IMUX_CIN);
        }
        imux_wires.push(wires::IMUX_CIN);
        imux_nw.push(TileWireCoord::new_idx(0, wires::IMUX_CIN));
    }

    for (w, pin) in [
        (wires::IMUX_STARTUP_CLK, "CLK"),
        (wires::IMUX_STARTUP_GSR, "GSR"),
        (wires::IMUX_STARTUP_GTS, "GTS"),
    ] {
        builder.wire_names(w, &[format!("LR_STUP_{pin}")]);
    }
    builder.wire_names(wires::IMUX_READCLK_I, &["LR_RDCLK_I"]);

    builder.wire_names(
        wires::IMUX_BUFG_H,
        &["LL_BUFG_I_3", "UL_BUFG_I_8", "LR_BUFG_I_4", "UR_BUFG7MUX"],
    );
    builder.wire_names(
        wires::IMUX_BUFG_V,
        &["LL_BUFG_I_2", "UL_BUFG_I_1", "LR_BUFG_I_5", "UR_CLKIN"],
    );

    for (w, xn) in [
        (wires::IMUX_TDO_O, "UR_TDO_1"),
        (wires::IMUX_TDO_T, "UR_TDO_2"),
        (wires::IMUX_RDBK_TRIG, "LL_RDBK_TRIG"),
        (wires::IMUX_BSCAN_TDO1, "UL_BSCAN2"),
        (wires::IMUX_BSCAN_TDO2, "UL_BSCAN6"),
    ] {
        builder.wire_names(w, &[xn]);
        imux_wires.push(w);
        imux_nw.push(TileWireCoord::new_idx(0, w));
    }

    (imux_wires, imux_nw)
}

fn fill_out_wires(builder: &mut IntBuilder) {
    for (w, wh, wv, pin) in [
        (
            wires::OUT_CLB_X,
            wires::OUT_CLB_X_H,
            wires::OUT_CLB_X_V,
            "FX",
        ),
        (
            wires::OUT_CLB_XQ,
            wires::OUT_CLB_XQ_H,
            wires::OUT_CLB_XQ_V,
            "FXQ",
        ),
        (
            wires::OUT_CLB_Y,
            wires::OUT_CLB_Y_H,
            wires::OUT_CLB_Y_V,
            "GY",
        ),
        (
            wires::OUT_CLB_YQ,
            wires::OUT_CLB_YQ_H,
            wires::OUT_CLB_YQ_V,
            "GYQ",
        ),
    ] {
        builder.wire_names(w, &[format!("CENTER_{pin}")]);
        builder.mark_permabuf(wh);
        builder.mark_permabuf(wv);
        if builder.rd.family != "xc4000e" {
            builder.wire_names(wh, &[&format!("CENTER_{pin}_HORIZ")]);
            builder.wire_names(wv, &[&format!("CENTER_{pin}_VERT")]);
        }
    }
    for (w, pin) in [(wires::OUT_CLB_X_S, "FX"), (wires::OUT_CLB_XQ_S, "FXQ")] {
        builder.wire_names(w, &[format!("CENTER_{pin}T")]);
        for k in BOT_KINDS {
            builder.extra_name(format!("{k}_{pin}T"), w);
        }
    }
    for (w, pin) in [(wires::OUT_CLB_Y_E, "GY"), (wires::OUT_CLB_YQ_E, "GYQ")] {
        builder.wire_names(w, &[format!("CENTER_{pin}L")]);
        for k in RT_KINDS {
            builder.extra_name(format!("{k}_{pin}L"), w);
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
        for (w, we, pin) in [
            (wires::OUT_IO_SN_I1, wires::OUT_IO_SN_I1_E1, "I1"),
            (wires::OUT_IO_SN_I2, wires::OUT_IO_SN_I2_E1, "I2"),
        ] {
            for k in BOT_KINDS.into_iter().chain(TOP_KINDS) {
                builder.extra_name(format!("{k}_{pin}_{ii}"), w[i]);
            }
            if i == 1 {
                builder.wire_names(we, &[format!("LR_L{pin}_{ii}"), format!("UR_{pin}_{ii}")]);
                for k in BOT_KINDS.into_iter().chain(TOP_KINDS) {
                    builder.extra_name(format!("{k}_{pin}_{ii}L"), we);
                }
                if pin == "I1" {
                    builder.extra_name("UL_BSCAN5", w[i]);
                    builder.extra_name("TOPSL_BSCAN5", we);
                    builder.extra_name("LL_MD2_I", w[i]);
                } else {
                    builder.extra_name("UL_BSCAN1", w[i]);
                    builder.extra_name("TOPSL_BSCAN1", we);
                    builder.extra_name("LL_RDBK_RIP", w[i]);
                }
            }
        }
    }

    for i in 0..2 {
        let ii = i + 1;
        for (w, ws, pin) in [
            (wires::OUT_IO_WE_I1, wires::OUT_IO_WE_I1_S1, "I1"),
            (wires::OUT_IO_WE_I2, wires::OUT_IO_WE_I2_S1, "I2"),
        ] {
            for k in RT_KINDS.into_iter().chain(LEFT_KINDS) {
                builder.extra_name(format!("{k}_{pin}_{ii}"), w[i]);
            }
            if i == 1 {
                builder.wire_names(ws, &[format!("LL_{pin}_{ii}"), format!("LR_T{pin}_{ii}")]);
                for k in RT_KINDS.into_iter().chain(LEFT_KINDS) {
                    builder.extra_name(format!("{k}_{pin}_{ii}T"), ws);
                }
                if pin == "I1" {
                    builder.extra_name("UL_BSCAN3", w[i]);
                    builder.extra_name("LEFTT_BSCAN3", ws);
                    builder.extra_name("UR_OSC1", w[i]);
                    builder.extra_name("RTT_OSC2", ws);
                } else {
                    builder.extra_name("UL_BSCAN4", w[i]);
                    builder.extra_name("LEFTT_BSCAN4", ws);
                    builder.extra_name("UR_OSC_OUT", w[i]);
                    builder.extra_name("RTT_OSC1", ws);
                }
            }
        }
    }

    builder.wire_names(
        wires::OUT_IO_CLKIN,
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
    builder.wire_names(wires::OUT_IO_CLKIN_W, &["UL_CLKIN_TOP", "LL_CLKIN_R"]);
    builder.wire_names(wires::OUT_IO_CLKIN_E, &["LR_CLKIN_LEFT", "UR_BUFG7MUX_L"]);
    builder.wire_names(wires::OUT_IO_CLKIN_S, &["LL_CLKIN_TOP", "LR_CLKIN_TOP"]);
    builder.wire_names(wires::OUT_IO_CLKIN_N, &["UR_BUFG6MUX_B", "UL_CLKIN_LEFT"]);

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

    builder.wire_names(wires::OUT_OSC_MUX1, &["UR_OSC_IN"]);

    for (w, pin) in [
        (wires::OUT_STARTUP_DONEIN, "DONEIN"),
        (wires::OUT_STARTUP_Q1Q4, "Q1Q4"),
        (wires::OUT_STARTUP_Q2, "Q2"),
        (wires::OUT_STARTUP_Q3, "Q3"),
    ] {
        builder.wire_names(w, &[format!("LR_STUP_{pin}")]);
    }

    if builder.rd.family != "xc4000e" {
        for k in TOP_KINDS {
            builder.extra_name(format!("{k}_COUTB"), wires::OUT_COUT);
        }
        builder.wire_names(wires::OUT_COUT_E, &["UR_COUT"]);
        for k in TOP_KINDS {
            builder.extra_name(format!("{k}_COUTL"), wires::OUT_COUT_E);
        }
    }

    builder.wire_names(wires::OUT_UPDATE_O, &["UR_UPDATE"]);
    if builder.rd.family != "spartanxl" {
        builder.wire_names(wires::OUT_MD0_I, &["LL_MD0_I"]);
    }
    builder.wire_names(wires::OUT_RDBK_DATA, &["LL_RDBK_DATA"]);

    if !matches!(&*builder.rd.family, "xc4000e" | "spartanxl") {
        builder.wire_names(
            wires::OUT_BUFGE_H,
            &["LL_BUFGE_3", "UL_BUFGE_7_8", "LR_BUFGE_4", "UR_BUFGE_7_8"],
        );
        builder.wire_names(
            wires::OUT_BUFGE_V,
            &["LL_BUFGE_2", "UL_BUFGE_1X", "LR_BUFGE_5", "UR_BUFGE_6X"],
        );

        for n in [
            "LHVBRK_FCLK_OUT",
            "LVLBRK_FCLK_OUT",
            "RHVBRK_FCLK_OUT",
            "RVRBRK_FCLK_OUT",
        ] {
            builder.extra_name_sub(n, 1, wires::OUT_BUFF);
        }
    }
}

fn fill_xc4000e_wirenames(builder: &mut IntBuilder) {
    for (name, wire) in xc4000e_wires::xc4000e_wires() {
        builder.extra_name(name, wire);
    }
}

fn extract_clb(
    builder: &mut IntBuilder,
    imux_wires: &[WireSlotId],
    imux_nw: &[TileWireCoord],
    force_names: &[(usize, String, WireSlotId)],
) {
    let is_xv = builder.rd.family == "xc4000xv";

    for &crd in builder.rd.tiles_by_kind_name("CENTER") {
        let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
        let xy_n = builder.walk_to_int(crd, Dir::N, true).unwrap();

        let lx = if is_xv { 2 } else { 1 };
        let tcid = if crd.y == 1 {
            if crd.x == lx {
                tcls::CLB_SW
            } else if xy_e.x == builder.rd.width - 1 {
                tcls::CLB_SE
            } else {
                tcls::CLB_S
            }
        } else if xy_n.y == builder.rd.height - 1 {
            if crd.x == lx {
                tcls::CLB_NW
            } else if xy_e.x == builder.rd.width - 1 {
                tcls::CLB_NE
            } else {
                tcls::CLB_N
            }
        } else {
            if crd.x == lx {
                tcls::CLB_W
            } else if xy_e.x == builder.rd.width - 1 {
                tcls::CLB_E
            } else {
                tcls::CLB
            }
        };
        let mut naming = "CLB".to_string();
        for xy in [xy_n, xy_e] {
            let kind = builder.rd.tile_kinds.key(builder.rd.tiles[&xy].kind);
            if kind != "CENTER" {
                write!(naming, "_{kind}").unwrap();
            }
        }

        let mut bel = builder
            .bel_single(bslots::CLB, "CLB")
            .pin_name_only("CIN", 0);
        if builder.rd.family == "xc4000e" {
            bel = bel
                .pin_name_only("COUT", 0)
                .extra_wire("CIN_S", &["CENTER_SEG_38"])
                .extra_wire("CIN_N", &["CENTER_SEG_56"]);
        } else {
            bel = bel.pin_name_only("COUT", 1);
        }
        let mut bels = vec![bel];
        for i in 0..2 {
            bels.push(builder.bel_indexed(bslots::TBUF[i], "TBUF", [2, 1][i]));
        }

        let mut cur_imux_nw = BTreeSet::from_iter(imux_nw.iter().copied());
        if builder.rd.family == "spartanxl"
            && matches!(tcid, tcls::CLB_N | tcls::CLB_NW | tcls::CLB_NE)
        {
            cur_imux_nw.remove(&TileWireCoord::new_idx(0, wires::IMUX_CLB_C2));
        }
        if builder.rd.family == "spartanxl"
            && matches!(tcid, tcls::CLB_W | tcls::CLB_SW | tcls::CLB_NW)
        {
            cur_imux_nw.remove(&TileWireCoord::new_idx(0, wires::IMUX_CLB_C3));
        }

        let mut xn = builder
            .xtile_id(tcid, &naming, crd)
            .num_cells(3)
            .raw_tile_single(xy_n, 1)
            .raw_tile_single(xy_e, 2)
            .extract_muxes(bslots::INT)
            .optin_muxes_tile(&cur_imux_nw)
            .skip_muxes(imux_wires)
            .skip_muxes(&[wires::LONG_H[2], wires::LONG_H[3]])
            .bels(bels);
        for &(rti, ref name, wire) in force_names {
            xn = xn.force_name(rti, name, TileWireCoord::new_idx(0, wire));
        }
        for (wt, wf) in [
            (wires::IMUX_CLB_F2, wires::IMUX_IO_O1[0]),
            (wires::IMUX_CLB_G2, wires::IMUX_IO_O1[1]),
        ] {
            let wt = TileWireCoord::new_idx(0, wt);
            let wf = TileWireCoord::new_idx(1, wf);
            xn = xn.force_skip_pip(wt, wf);
        }
        if is_xv {
            xn = xn
                .raw_tile(crd.delta(-1, 0))
                .raw_tile(crd.delta(0, 1))
                .raw_tile(crd.delta(-1, 1))
                .extract_muxes_rt(bslots::INT, 3)
                .extract_muxes_rt(bslots::INT, 4)
                .extract_muxes_rt(bslots::INT, 5);
        }
        xn.extract();
    }

    let naming = builder.ndb.get_tile_class_naming("CLB");
    builder.inject_int_type_naming("CENTER", naming);
}

fn extract_bot(
    builder: &mut IntBuilder,
    imux_wires: &[WireSlotId],
    imux_nw: &[TileWireCoord],
    force_names: &[(usize, String, WireSlotId)],
) {
    let is_xv = builder.rd.family == "xc4000xv";
    let is_xl = !matches!(&*builder.rd.family, "xc4000e" | "spartanxl");
    for (tcid, tkn) in [
        (tcls::IO_S0, "BOT"),
        (tcls::IO_S0_E, "BOTRR"),
        (tcls::IO_S1, "BOTS"),
        (tcls::IO_S1_W, "BOTSL"),
    ] {
        let mut found_naming = None;
        for &crd in builder.rd.tiles_by_kind_name(tkn) {
            let xy_n = builder.walk_to_int(crd, Dir::N, true).unwrap();
            let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
            let xy_w = builder.walk_to_int(crd, Dir::W, true).unwrap();
            let kind_e = builder.rd.tile_kinds.key(builder.rd.tiles[&xy_e].kind);
            let naming = format!("{tkn}_{kind_e}");
            let mut bels = vec![];
            for i in 0..2 {
                bels.push(
                    builder
                        .bel_indexed(bslots::IO[i], "IOB", i + 1)
                        .pin_rename("EC", "O1")
                        .pin_rename("O", "O2"),
                )
            }
            if builder.rd.family != "spartanxl" {
                for i in 0..3 {
                    bels.push(builder.bel_indexed(bslots::DEC[i], "DEC", i + 1))
                }
            }
            let cout_names: Vec<_> = BOT_KINDS.into_iter().map(|k| format!("{k}_COUT")).collect();
            if builder.rd.family != "xc4000e" {
                bels.push(
                    builder
                        .bel_virtual(bslots::CIN)
                        .extra_int_in("I", &cout_names),
                );
            }
            let mut xn = builder
                .xtile_id(tcid, &naming, crd)
                .num_cells(4)
                .raw_tile_single(xy_n, 1)
                .raw_tile_single(xy_e, 2)
                .raw_tile_single(xy_w, 3)
                .extract_muxes(bslots::INT)
                .optin_muxes_tile(imux_nw)
                .skip_muxes(imux_wires)
                .bels(bels);
            for &(rti, ref name, wire) in force_names {
                xn = xn.force_name(rti, name, TileWireCoord::new_idx(0, wire));
            }
            if is_xl && tkn == "BOTSL" {
                xn = xn
                    .force_name(3, "LL_FCLK", TileWireCoord::new_idx(0, wires::ECLK_H))
                    .force_name(
                        3,
                        "LL_BHLL1",
                        TileWireCoord::new_idx(0, wires::LONG_IO_H[0]),
                    )
                    .force_name(
                        3,
                        "LL_BHLL2",
                        TileWireCoord::new_idx(0, wires::LONG_IO_H[1]),
                    )
                    .force_name(
                        3,
                        "LL_BHLL4",
                        TileWireCoord::new_idx(0, wires::LONG_IO_H[3]),
                    )
                    .optin_muxes_tile(&[TileWireCoord::new_idx(0, wires::ECLK_H)]);
            }
            for (wt, wf) in [
                (wires::IMUX_CLB_F4, wires::IMUX_IO_O1[0]),
                (wires::IMUX_CLB_G4, wires::IMUX_IO_O1[1]),
            ] {
                let wt = TileWireCoord::new_idx(0, wt);
                let wf = TileWireCoord::new_idx(0, wf);
                xn = xn.force_skip_pip(wt, wf);
            }
            if is_xv {
                xn = xn
                    .raw_tile(crd.delta(-1, 0))
                    .extract_muxes_rt(bslots::INT, 4);
            }
            xn.extract();
            found_naming = Some(naming);
        }
        let naming = builder.ndb.get_tile_class_naming(&found_naming.unwrap());
        builder.inject_int_type_naming(tkn, naming);
    }
}

fn extract_top(builder: &mut IntBuilder, imux_wires: &[WireSlotId], imux_nw: &[TileWireCoord]) {
    let is_xv = builder.rd.family == "xc4000xv";
    let is_xl = !matches!(&*builder.rd.family, "xc4000e" | "spartanxl");
    for (tcid, tkn) in [
        (tcls::IO_N0, "TOP"),
        (tcls::IO_N0_E, "TOPRR"),
        (tcls::IO_N1, "TOPS"),
        (tcls::IO_N1_W, "TOPSL"),
    ] {
        let mut found_naming = None;
        for &crd in builder.rd.tiles_by_kind_name(tkn) {
            let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
            let xy_w = builder.walk_to_int(crd, Dir::W, true).unwrap();
            let kind_e = builder.rd.tile_kinds.key(builder.rd.tiles[&xy_e].kind);
            let naming = format!("{tkn}_{kind_e}");

            let mut bels = vec![];
            for i in 0..2 {
                bels.push(
                    builder
                        .bel_indexed(bslots::IO[i], "IOB", i + 1)
                        .pin_rename("EC", "O1")
                        .pin_rename("O", "O2"),
                )
            }
            if builder.rd.family != "spartanxl" {
                for i in 0..3 {
                    bels.push(builder.bel_indexed(bslots::DEC[i], "DEC", i + 1))
                }
            }
            let cout_names: Vec<_> = TOP_KINDS
                .into_iter()
                .map(|k| format!("{k}_COUTB"))
                .collect();
            if builder.rd.family != "xc4000e" {
                bels.push(
                    builder
                        .bel_virtual(bslots::COUT)
                        .extra_int_out("O", &cout_names),
                );
            }
            let mut xn = builder
                .xtile_id(tcid, &naming, crd)
                .num_cells(3)
                .raw_tile_single(xy_e, 1)
                .raw_tile_single(xy_w, 2)
                .extract_muxes(bslots::INT)
                .optin_muxes_tile(imux_nw)
                .skip_muxes(imux_wires)
                .bels(bels);
            if is_xv {
                xn = xn
                    .raw_tile(crd.delta(-1, 0))
                    .extract_muxes_rt(bslots::INT, 3);
            }
            if is_xl && tkn == "TOPSL" {
                xn = xn
                    .force_name(2, "UL_FCLK", TileWireCoord::new_idx(0, wires::ECLK_H))
                    .force_name(
                        2,
                        "UL_THLL1",
                        TileWireCoord::new_idx(0, wires::LONG_IO_H[0]),
                    )
                    .force_name(
                        2,
                        "UL_THLL2",
                        TileWireCoord::new_idx(0, wires::LONG_IO_H[1]),
                    )
                    .force_name(
                        2,
                        "UL_THLL4",
                        TileWireCoord::new_idx(0, wires::LONG_IO_H[3]),
                    )
                    .optin_muxes_tile(&[TileWireCoord::new_idx(0, wires::ECLK_H)]);
            }
            xn.extract();
            found_naming = Some(naming);
        }
        let naming = builder.ndb.get_tile_class_naming(&found_naming.unwrap());
        builder.inject_int_type_naming(tkn, naming);
    }
}

fn extract_rt(builder: &mut IntBuilder, imux_wires: &[WireSlotId], imux_nw: &[TileWireCoord]) {
    let is_xv = builder.rd.family == "xc4000xv";
    let is_e = builder.rd.family == "xc4000e";
    let is_xl = !matches!(&*builder.rd.family, "xc4000e" | "spartanxl");
    for (tcid, tkn) in [
        (tcls::IO_E0, "RT"),
        (tcls::IO_E0_N, "RTT"),
        (tcls::IO_E0_F1, "RTF"),
        (tcls::IO_E0_F0, "RTF1"),
        (tcls::IO_E1, "RTS"),
        (tcls::IO_E1_S, "RTSB"),
        (tcls::IO_E1_F1, "RTSF"),
        (tcls::IO_E1_F0, "RTSF1"),
    ] {
        let mut found_naming = None;
        for &crd in builder.rd.tiles_by_kind_name(tkn) {
            let xy_s = builder.walk_to_int(crd, Dir::S, true).unwrap();
            let xy_n = builder.walk_to_int(crd, Dir::N, true).unwrap();
            let kind_s = builder.rd.tile_kinds.key(builder.rd.tiles[&xy_s].kind);
            let naming = format!("{tkn}_{kind_s}");

            let mut bels = vec![];
            for i in 0..2 {
                let mut bel = builder
                    .bel_indexed(bslots::IO[i], "IOB", i + 1)
                    .pin_rename("EC", "O1")
                    .pin_rename("O", "O2");
                if matches!(&*builder.rd.family, "xc4000xla" | "xc4000xv")
                    && (tkn.ends_with('F') || tkn.ends_with("F1"))
                {
                    bel = bel.pin_name_only("CLKIN", 0);
                }
                bels.push(bel)
            }
            for i in 0..2 {
                bels.push(builder.bel_indexed(bslots::TBUF[i], "TBUF", [2, 1][i]));
            }
            for i in 0..2 {
                bels.push(builder.bel_indexed(bslots::PULLUP_TBUF[i], "PULLUP", [2, 1][i]));
            }
            if builder.rd.family != "spartanxl" {
                for i in 0..3 {
                    bels.push(builder.bel_indexed(bslots::DEC[i], "DEC", i + 1))
                }
            }

            let mut xn = builder
                .xtile_id(tcid, &naming, crd)
                .num_cells(3)
                .raw_tile_single(xy_s, 1)
                .raw_tile_single(xy_n, 2)
                .extract_muxes(bslots::INT)
                .optin_muxes_tile(imux_nw)
                .skip_muxes(imux_wires)
                .bels(bels);
            if is_e {
                xn = xn
                    .skip_muxes(&[wires::LONG_H[2], wires::LONG_H[3]])
                    .force_pip(
                        TileWireCoord::new_idx(0, wires::LONG_H[2]),
                        TileWireCoord::new_idx(0, wires::SINGLE_V[3]),
                    )
                    .force_pip(
                        TileWireCoord::new_idx(0, wires::LONG_H[3]),
                        TileWireCoord::new_idx(0, wires::SINGLE_V[4]),
                    );
            }
            for (wt, wf) in [
                (wires::IMUX_CLB_G1, wires::IMUX_IO_O1[0]),
                (wires::IMUX_CLB_F1, wires::IMUX_IO_O1[1]),
            ] {
                let wt = TileWireCoord::new_idx(0, wt);
                let wf = TileWireCoord::new_idx(0, wf);
                xn = xn.force_skip_pip(wt, wf);
            }
            if is_xv {
                xn = xn
                    .raw_tile(crd.delta(0, 1))
                    .extract_muxes_rt(bslots::INT, 3);
            }
            if is_xl && tkn == "RTT" {
                xn = xn
                    .force_name(2, "UR_URKX", TileWireCoord::new_idx(0, wires::ECLK_V))
                    .optin_muxes_tile(&[TileWireCoord::new_idx(0, wires::ECLK_V)]);
            }
            xn.extract();
            found_naming = Some(naming);
        }

        if let Some(naming) = found_naming {
            let naming = builder.ndb.get_tile_class_naming(&naming);
            builder.inject_int_type_naming(tkn, naming);
        }
    }
}

fn extract_left(builder: &mut IntBuilder, imux_wires: &[WireSlotId], imux_nw: &[TileWireCoord]) {
    let is_xv = builder.rd.family == "xc4000xv";
    let is_xl = !matches!(&*builder.rd.family, "xc4000e" | "spartanxl");
    for (tcid, tkn) in [
        (tcls::IO_W0, "LEFT"),
        (tcls::IO_W0_N, "LEFTT"),
        (tcls::IO_W0_F1, "LEFTF"),
        (tcls::IO_W0_F0, "LEFTF1"),
        (tcls::IO_W1, "LEFTS"),
        (tcls::IO_W1_S, "LEFTSB"),
        (tcls::IO_W1_F1, "LEFTSF"),
        (tcls::IO_W1_F0, "LEFTSF1"),
    ] {
        let mut found_naming = None;
        for &crd in builder.rd.tiles_by_kind_name(tkn) {
            let xy_s = builder.walk_to_int(crd, Dir::S, true).unwrap();
            let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
            let xy_n = builder.walk_to_int(crd, Dir::N, true).unwrap();
            let kind_s = builder.rd.tile_kinds.key(builder.rd.tiles[&xy_s].kind);
            let naming = format!("{tkn}_{kind_s}");

            let mut bels = vec![];
            for i in 0..2 {
                let mut bel = builder
                    .bel_indexed(bslots::IO[i], "IOB", i + 1)
                    .pin_rename("EC", "O1")
                    .pin_rename("O", "O2");
                if matches!(&*builder.rd.family, "xc4000xla" | "xc4000xv")
                    && (tkn.ends_with('F') || tkn.ends_with("F1"))
                {
                    bel = bel.pin_name_only("CLKIN", 0);
                }
                bels.push(bel)
            }
            for i in 0..2 {
                bels.push(builder.bel_indexed(bslots::TBUF[i], "TBUF", [2, 1][i]));
            }
            for i in 0..2 {
                bels.push(builder.bel_indexed(bslots::PULLUP_TBUF[i], "PULLUP", [2, 1][i]));
            }
            if builder.rd.family != "spartanxl" {
                for i in 0..3 {
                    bels.push(builder.bel_indexed(bslots::DEC[i], "DEC", i + 1))
                }
            }
            if builder.rd.family == "xc4000ex" {
                bels.push(builder.bel_virtual(bslots::MISC_W));
            }

            let mut xn = builder
                .xtile_id(tcid, &naming, crd)
                .num_cells(4)
                .raw_tile_single(xy_s, 1)
                .raw_tile_single(xy_e, 2)
                .raw_tile_single(xy_n, 3)
                .extract_muxes(bslots::INT)
                .optin_muxes_tile(imux_nw)
                .skip_muxes(imux_wires)
                .skip_muxes(&[wires::LONG_H[2], wires::LONG_H[3]])
                .bels(bels);
            if is_xv {
                xn = xn
                    .raw_tile(crd.delta(0, 1))
                    .extract_muxes_rt(bslots::INT, 4);
            }
            if is_xl && tkn == "LEFTT" {
                xn = xn
                    .force_name(3, "UL_KX", TileWireCoord::new_idx(0, wires::ECLK_V))
                    .optin_muxes_tile(&[TileWireCoord::new_idx(0, wires::ECLK_V)]);
            }
            xn.extract();
            found_naming = Some(naming);
        }

        if let Some(naming) = found_naming {
            let naming = builder.ndb.get_tile_class_naming(&naming);
            builder.inject_int_type_naming(tkn, naming);
        }
    }
}

fn bel_bufg(builder: &mut IntBuilder, slot: BelSlotId, idx: usize) -> ExtrBelInfo {
    let rd = builder.rd;
    match rd.family.as_str() {
        "spartanxl" => builder.bel_indexed(slot, "BUFGLS", idx),
        "xc4000e" => builder.bel_single(
            slot,
            if idx.is_multiple_of(2) {
                "BUFGP"
            } else {
                "BUFGS"
            },
        ),
        _ => builder
            .bel_indexed(slot, "BUFG", idx)
            .pin_rename("O", "O_BUFG")
            .pins_name_only(&["O_BUFG"])
            .sub_indexed(rd, "BUFGE", idx)
            .pin_rename("I", "I_BUFGE")
            .pins_name_only(&["I_BUFGE"])
            .pin_rename("O", "O_BUFGE")
            .sub_indexed(rd, "BUFGLS", idx)
            .pin_rename("I", "I_BUFGLS")
            .pins_name_only(&["I_BUFGLS"]),
    }
}

fn extract_lr(builder: &mut IntBuilder, imux_wires: &[WireSlotId], imux_nw: &[TileWireCoord]) {
    let clkc = *builder.rd.tiles_by_kind_name("CLKC").first().unwrap();
    for &crd in builder.rd.tiles_by_kind_name("LR") {
        let mut bels = vec![
            bel_bufg(builder, bslots::BUFG_H, 3),
            bel_bufg(builder, bslots::BUFG_V, 4),
        ];
        if builder.rd.family != "spartanxl" {
            for i in 0..4 {
                bels.push(builder.bel_indexed(bslots::PULLUP_DEC_H[i], "PULLUP", (i ^ 7) + 1));
            }
            for i in 0..4 {
                bels.push(builder.bel_indexed(bslots::PULLUP_DEC_V[i], "PULLUP", (i ^ 3) + 1));
            }
        }
        let misc = if builder.rd.family == "xc4000e" {
            builder
                .bel_single(bslots::MISC_SE, "COUT")
                .pin_name_only("I", 0)
        } else {
            builder.bel_virtual(bslots::MISC_SE)
        };
        bels.extend([
            builder.bel_single(bslots::STARTUP, "STARTUP"),
            builder.bel_single(bslots::READCLK, "RDCLK"),
            misc,
        ]);

        builder
            .xtile_id(tcls::CNR_SE, "CNR_SE", crd)
            .raw_tile(clkc)
            .extract_muxes(bslots::INT)
            .optin_muxes_tile(imux_nw)
            .skip_muxes(imux_wires)
            .skip_muxes(wires::BUFGLS.as_slice())
            .bels(bels)
            .extract();
    }
}

fn extract_ur(builder: &mut IntBuilder, imux_wires: &[WireSlotId], imux_nw: &[TileWireCoord]) {
    let clkc = *builder.rd.tiles_by_kind_name("CLKC").first().unwrap();
    let is_xl = !matches!(&*builder.rd.family, "xc4000e" | "spartanxl");
    for &crd in builder.rd.tiles_by_kind_name("UR") {
        let mut bels = vec![
            bel_bufg(builder, bslots::BUFG_H, 2),
            bel_bufg(builder, bslots::BUFG_V, 1),
        ];
        if builder.rd.family != "spartanxl" {
            for i in 0..4 {
                bels.push(builder.bel_indexed(bslots::PULLUP_DEC_H[i], "PULLUP", i + 1));
            }
            for i in 0..4 {
                bels.push(builder.bel_indexed(bslots::PULLUP_DEC_V[i], "PULLUP", i + 5));
            }
        }
        let misc = if builder.rd.family == "xc4000e" {
            builder
                .bel_single(bslots::MISC_NE, "COUT")
                .pin_name_only("I", 0)
        } else {
            builder.bel_virtual(bslots::MISC_NE)
        };
        bels.extend([
            builder.bel_single(bslots::UPDATE, "UPDATE"),
            builder
                .bel_single(bslots::OSC, "OSC")
                .pins_name_only(&["F15", "F490", "F16K", "F500K"])
                .extra_int_out("OUT0", &["UR_SEG_4", "UR_OSC_OUT"])
                .extra_int_out("OUT1", &["UR_SEG_44", "UR_OSC_IN"]),
            builder.bel_single(bslots::TDO, "TDO"),
            misc,
        ]);
        let xy_s = builder.walk_to_int(crd, Dir::S, true).unwrap();

        let mut xn = builder
            .xtile_id(tcls::CNR_NE, "CNR_NE", crd)
            .num_cells(2)
            .raw_tile_single(xy_s, 1)
            .raw_tile(clkc)
            .extract_muxes(bslots::INT)
            .optin_muxes_tile(imux_nw)
            .skip_muxes(imux_wires)
            .skip_muxes(wires::BUFGLS.as_slice());
        if is_xl {
            xn = xn.skip_muxes(&[wires::ECLK_V]);
        }
        xn.bels(bels).extract();
    }
}

fn extract_ll(builder: &mut IntBuilder, imux_wires: &[WireSlotId], imux_nw: &[TileWireCoord]) {
    let clkc = *builder.rd.tiles_by_kind_name("CLKC").first().unwrap();
    let is_xl = !matches!(&*builder.rd.family, "xc4000e" | "spartanxl");
    for &crd in builder.rd.tiles_by_kind_name("LL") {
        let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
        let mut bels = vec![
            bel_bufg(builder, bslots::BUFG_H, 6),
            bel_bufg(builder, bslots::BUFG_V, 5),
        ];
        if builder.rd.family == "spartanxl" {
            bels.extend([
                builder.bel_virtual(bslots::MD0),
                builder.bel_virtual(bslots::MD1),
                builder.bel_virtual(bslots::MD2),
            ]);
        } else {
            for i in 0..4 {
                bels.push(builder.bel_indexed(bslots::PULLUP_DEC_H[i], "PULLUP", (i ^ 7) + 1));
            }
            for i in 0..4 {
                bels.push(builder.bel_indexed(bslots::PULLUP_DEC_V[i], "PULLUP", (i ^ 3) + 1));
            }
            bels.extend([
                builder.bel_single(bslots::MD0, "MD0"),
                builder.bel_single(bslots::MD1, "MD1"),
                builder.bel_single(bslots::MD2, "MD2"),
            ]);
        }
        let misc = if builder.rd.family == "xc4000e" {
            builder
                .bel_single(bslots::MISC_SW, "CIN")
                .pin_name_only("O", 1)
        } else {
            builder.bel_virtual(bslots::MISC_SW)
        };
        bels.extend([builder.bel_single(bslots::RDBK, "RDBK"), misc]);

        let mut xn = builder
            .xtile_id(tcls::CNR_SW, "CNR_SW", crd)
            .num_cells(2)
            .raw_tile_single(xy_e, 1)
            .raw_tile(clkc)
            .extract_muxes(bslots::INT)
            .optin_muxes_tile(imux_nw)
            .skip_muxes(imux_wires)
            .skip_muxes(wires::BUFGLS.as_slice());
        if is_xl {
            xn = xn.skip_muxes(&[wires::ECLK_H]);
        }
        xn.bels(bels).extract();
    }
}

fn extract_ul(builder: &mut IntBuilder, imux_wires: &[WireSlotId], imux_nw: &[TileWireCoord]) {
    let clkc = *builder.rd.tiles_by_kind_name("CLKC").first().unwrap();
    let is_xl = !matches!(&*builder.rd.family, "xc4000e" | "spartanxl");
    for &crd in builder.rd.tiles_by_kind_name("UL") {
        let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
        let xy_s = builder.walk_to_int(crd, Dir::S, true).unwrap();
        let xy_se = builder.walk_to_int(xy_s, Dir::E, true).unwrap();
        let mut bels = vec![
            bel_bufg(builder, bslots::BUFG_H, 7),
            bel_bufg(builder, bslots::BUFG_V, 0),
        ];
        if builder.rd.family != "spartanxl" {
            for i in 0..4 {
                bels.push(builder.bel_indexed(bslots::PULLUP_DEC_H[i], "PULLUP", i + 1));
            }
            for i in 0..4 {
                bels.push(builder.bel_indexed(bslots::PULLUP_DEC_V[i], "PULLUP", i + 5));
            }
        }
        let misc = if builder.rd.family == "xc4000e" {
            builder
                .bel_single(bslots::MISC_NW, "CIN")
                .pin_name_only("O", 1)
        } else {
            builder.bel_virtual(bslots::MISC_NW)
        };
        bels.extend([builder.bel_single(bslots::BSCAN, "BSCAN"), misc]);

        let mut xn = builder
            .xtile_id(tcls::CNR_NW, "CNR_NW", crd)
            .num_cells(4)
            .raw_tile_single(xy_e, 1)
            .raw_tile_single(xy_s, 2)
            .raw_tile_single(xy_se, 3)
            .raw_tile(clkc)
            .extract_muxes(bslots::INT)
            .optin_muxes_tile(imux_nw)
            .skip_muxes(imux_wires)
            .skip_muxes(wires::BUFGLS.as_slice());
        if is_xl {
            xn = xn.skip_muxes(&[wires::ECLK_H, wires::ECLK_V]);
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
                write!(naming, "_{kind}").unwrap();
            }
        }
        builder.ndb.get_tile_class_naming(&naming)
    } else if tkn.starts_with("BOT") || tkn.starts_with("TOP") {
        let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
        let kind_e = builder.rd.tile_kinds.key(builder.rd.tiles[&xy_e].kind);
        let naming = format!("{tkn}_{kind_e}");
        builder.ndb.get_tile_class_naming(&naming)
    } else if tkn.starts_with("LEFT") || tkn.starts_with("RT") {
        let xy_s = builder.walk_to_int(crd, Dir::S, true).unwrap();
        let kind_s = builder.rd.tile_kinds.key(builder.rd.tiles[&xy_s].kind);
        let naming = format!("{tkn}_{kind_s}");
        builder.ndb.get_tile_class_naming(&naming)
    } else {
        builder.ndb.get_tile_class_naming(tkn)
    }
}

fn extract_llh(builder: &mut IntBuilder) {
    let is_sxl = builder.rd.family == "spartanxl";
    for (tcid, naming, tkn) in [
        (tcls::LLH_IO_S, "LLH_IO_S", "CLKB"),
        (tcls::LLH_IO_N, "LLH_IO_N", "CLKT"),
        (
            tcls::LLH_CLB,
            "LLH_CLB_S",
            if builder.rd.family == "spartanxl" {
                "CLKVC"
            } else {
                "CLKVB"
            },
        ),
        (
            tcls::LLH_CLB_S,
            "LLH_CLB_S",
            if builder.rd.family == "spartanxl" {
                "CLKVC"
            } else {
                "CLKVB"
            },
        ),
        (tcls::LLH_CLB, "LLH_CLB_N", "CLKV"),
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
                        .bel_virtual(bslots::TBUF_SPLITTER[0])
                        .extra_int_inout("W", &["CLKV_HLL3", "CLKVC_HLL3"])
                        .extra_int_inout("E", &["CLKV_HLL3R", "CLKVC_HLL3R"])
                        .extra_wire("W_EXCL", &["CLKV_HLL3_EXCL", "CLKVC_HLL3_EXCL"])
                        .extra_wire("E_EXCL", &["CLKV_HLL3R_EXCL", "CLKVC_HLL3R_EXCL"]),
                    builder
                        .bel_virtual(bslots::TBUF_SPLITTER[1])
                        .extra_int_inout("W", &["CLKV_HLL4", "CLKVC_HLL4"])
                        .extra_int_inout("E", &["CLKV_HLL4R", "CLKVC_HLL4R"])
                        .extra_wire("W_EXCL", &["CLKV_HLL4_EXCL", "CLKVC_HLL4_EXCL"])
                        .extra_wire("E_EXCL", &["CLKV_HLL4R_EXCL", "CLKVC_HLL4R_EXCL"]),
                ]);
            }

            let mut xn = builder
                .xtile_id(tcid, naming, crd)
                .num_cells(2)
                .ref_single(xy_w, 0, naming_w)
                .ref_single(xy_e, 1, naming_e)
                .extract_muxes(bslots::LLH)
                .bels(bels);
            if has_splitter {
                xn = xn.skip_muxes(&[wires::LONG_H[2], wires::LONG_H[3]]);
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
    for (tcid, naming, tkn) in [
        (tcls::LLV_IO_W, "LLV_IO_W", "CLKL"),
        (tcls::LLV_IO_E, "LLV_IO_E", "CLKR"),
        (tcls::LLV_CLB, "LLV_CLB", "CLKH"),
    ] {
        if let Some(&crd) = builder.rd.tiles_by_kind_name(tkn).first() {
            let xy_s = builder.walk_to_int(crd, Dir::S, true).unwrap();
            let xy_n = builder.walk_to_int(crd, Dir::N, true).unwrap();
            let naming_s = get_tile_naming(builder, xy_s);
            let naming_n = get_tile_naming(builder, xy_n);
            let mut xt = builder
                .xtile_id(tcid, naming, crd)
                .num_cells(2)
                .ref_single(xy_s, 0, naming_s)
                .ref_single(xy_n, 1, naming_n)
                .extract_muxes(bslots::LLV)
                .skip_muxes(&clk_wires)
                .optin_muxes(wires::GCLK.as_slice());
            if tcid == tcls::LLV_IO_E {
                let bel = xt.builder.bel_virtual(bslots::MISC_E);
                xt = xt.bel(bel);
            }
            xt.extract();
        }
    }
}

fn extract_llhq(builder: &mut IntBuilder) {
    for (tcid, naming, tkn) in [
        (tcls::LLHQ_CLB, "LLHQ_CLB", "VHBRK"),
        (tcls::LLHQ_CLB_S, "LLHQ_CLB", "VHBRK"),
        (tcls::LLHQ_CLB_N, "LLHQ_CLB", "VHBRK"),
        (tcls::LLHQ_CLB, "LLHQ_CLB_O", "VHBRKV"),
        (tcls::LLHQ_CLB_S, "LLHQ_CLB_O", "VHBRKV"),
        (tcls::LLHQ_CLB_N, "LLHQ_CLB_O", "VHBRKV"),
        (tcls::LLHQ_CLB, "LLHQ_CLB_I", "VHBRKVC"),
        (tcls::LLHQ_IO_S, "LLHQ_IO_S", "BVHBRK"),
        (tcls::LLHQ_IO_N, "LLHQ_IO_N", "THRBRK"),
    ] {
        if let Some(&crd) = builder.rd.tiles_by_kind_name(tkn).first() {
            let xy_w = builder.walk_to_int(crd, Dir::W, true).unwrap();
            let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
            let naming_w = get_tile_naming(builder, xy_w);
            let naming_e = get_tile_naming(builder, xy_e);
            let mut bels = vec![];
            if naming.starts_with("LLHQ_CLB") {
                bels.extend([
                    builder
                        .bel_indexed(bslots::PULLUP_TBUF_W[0], "PULLUP", 4)
                        .pin_int_nudge("O", TileWireCoord::new_idx(0, wires::LONG_H[2])),
                    builder
                        .bel_indexed(bslots::PULLUP_TBUF_E[0], "PULLUP", 2)
                        .pin_int_nudge("O", TileWireCoord::new_idx(1, wires::LONG_H[2])),
                    builder
                        .bel_indexed(bslots::PULLUP_TBUF_W[1], "PULLUP", 3)
                        .pin_int_nudge("O", TileWireCoord::new_idx(0, wires::LONG_H[3])),
                    builder
                        .bel_indexed(bslots::PULLUP_TBUF_E[1], "PULLUP", 1)
                        .pin_int_nudge("O", TileWireCoord::new_idx(1, wires::LONG_H[3])),
                ]);
            }
            builder
                .xtile_id(tcid, naming, crd)
                .num_cells(2)
                .ref_single(xy_w, 0, naming_w)
                .ref_single(xy_e, 1, naming_e)
                .extract_muxes(bslots::LLH)
                .bels(bels)
                .extract();
        }
    }
}

fn extract_llhc(builder: &mut IntBuilder) {
    for (tcid, naming, tkn) in [
        (tcls::LLHC_CLB, "LLHC_CLB.O", "CLKV"),
        (tcls::LLHC_CLB_S, "LLHC_CLB_O", "CLKV"),
        (tcls::LLHC_CLB, "LLHC_CLB_I", "CLKVC"),
        (tcls::LLHC_IO_S, "LLHC_IO_S", "CLKB"),
        (tcls::LLHC_IO_N, "LLHC_IO_N", "CLKT"),
    ] {
        if let Some(&crd) = builder.rd.tiles_by_kind_name(tkn).first() {
            let xy_w = builder.walk_to_int(crd, Dir::W, true).unwrap();
            let xy_e = builder.walk_to_int(crd, Dir::E, true).unwrap();
            let naming_w = get_tile_naming(builder, xy_w);
            let naming_e = get_tile_naming(builder, xy_e);
            let mut bels = vec![];
            match tcid {
                tcls::LLHC_CLB | tcls::LLHC_CLB_S => {
                    bels.extend([
                        builder.bel_indexed(bslots::PULLUP_TBUF_W[0], "PULLUP", 2),
                        builder.bel_indexed(bslots::PULLUP_TBUF_E[0], "PULLUP", 4),
                        builder.bel_indexed(bslots::PULLUP_TBUF_W[1], "PULLUP", 1),
                        builder.bel_indexed(bslots::PULLUP_TBUF_E[1], "PULLUP", 3),
                        builder
                            .bel_virtual(bslots::TBUF_SPLITTER[0])
                            .extra_int_inout("W", &["CLKV_HLL3", "CLKVC_HLL3"])
                            .extra_int_inout("E", &["CLKV_HLL3R", "CLKVC_HLL3R"])
                            .extra_wire("W_EXCL", &["CLKV_HLL3_EXCL", "CLKVC_HLL3_EXCL"])
                            .extra_wire("E_EXCL", &["CLKV_HLL3R_EXCL", "CLKVC_HLL3R_EXCL"]),
                        builder
                            .bel_virtual(bslots::TBUF_SPLITTER[1])
                            .extra_int_inout("W", &["CLKV_HLL4", "CLKVC_HLL4"])
                            .extra_int_inout("E", &["CLKV_HLL4R", "CLKVC_HLL4R"])
                            .extra_wire("W_EXCL", &["CLKV_HLL4_EXCL", "CLKVC_HLL4_EXCL"])
                            .extra_wire("E_EXCL", &["CLKV_HLL4R_EXCL", "CLKVC_HLL4R_EXCL"]),
                    ]);
                }
                tcls::LLHC_IO_S => {
                    bels.extend([
                        builder.bel_indexed(bslots::PULLUP_DEC_W[0], "PULLUP", 4),
                        builder.bel_indexed(bslots::PULLUP_DEC_E[0], "PULLUP", 5),
                        builder.bel_indexed(bslots::PULLUP_DEC_W[1], "PULLUP", 3),
                        builder.bel_indexed(bslots::PULLUP_DEC_E[1], "PULLUP", 6),
                        builder.bel_indexed(bslots::PULLUP_DEC_W[2], "PULLUP", 2),
                        builder.bel_indexed(bslots::PULLUP_DEC_E[2], "PULLUP", 7),
                        builder.bel_indexed(bslots::PULLUP_DEC_W[3], "PULLUP", 1),
                        builder.bel_indexed(bslots::PULLUP_DEC_E[3], "PULLUP", 8),
                    ]);
                }
                tcls::LLHC_IO_N => {
                    bels.extend([
                        builder.bel_indexed(bslots::PULLUP_DEC_W[0], "PULLUP", 1),
                        builder.bel_indexed(bslots::PULLUP_DEC_E[0], "PULLUP", 8),
                        builder.bel_indexed(bslots::PULLUP_DEC_W[1], "PULLUP", 2),
                        builder.bel_indexed(bslots::PULLUP_DEC_E[1], "PULLUP", 7),
                        builder.bel_indexed(bslots::PULLUP_DEC_W[2], "PULLUP", 3),
                        builder.bel_indexed(bslots::PULLUP_DEC_E[2], "PULLUP", 6),
                        builder.bel_indexed(bslots::PULLUP_DEC_W[3], "PULLUP", 4),
                        builder.bel_indexed(bslots::PULLUP_DEC_E[3], "PULLUP", 5),
                    ]);
                }
                _ => unreachable!(),
            }
            let mut xn = builder
                .xtile_id(tcid, naming, crd)
                .num_cells(2)
                .ref_single(xy_w, 0, naming_w)
                .ref_single(xy_e, 1, naming_e)
                .extract_muxes(bslots::LLH)
                .bels(bels);
            if naming.starts_with("LLHC_CLB") {
                xn = xn.skip_muxes(&[wires::LONG_H[2], wires::LONG_H[3]]);
            }
            xn.extract();
        }
    }
}

fn extract_llvc(builder: &mut IntBuilder) {
    for (tcid, naming, tkn) in [
        (tcls::LLVC_IO_W, "LLVC_IO_W", "CLKL"),
        (tcls::LLVC_IO_E, "LLVC_IO_E", "CLKR"),
        (tcls::LLVC_CLB, "LLVC_CLB", "CLKH"),
    ] {
        if let Some(&crd) = builder.rd.tiles_by_kind_name(tkn).first() {
            let xy_s = builder.walk_to_int(crd, Dir::S, true).unwrap();
            let xy_n = builder.walk_to_int(crd, Dir::N, true).unwrap();
            let naming_s = get_tile_naming(builder, xy_s);
            let naming_n = get_tile_naming(builder, xy_n);
            let mut bels = vec![];
            match tcid {
                tcls::LLVC_IO_W | tcls::LLVC_IO_E => {
                    bels.extend([
                        builder.bel_indexed(bslots::PULLUP_DEC_S[0], "PULLUP", 10),
                        builder.bel_indexed(bslots::PULLUP_DEC_N[0], "PULLUP", 3),
                        builder.bel_indexed(bslots::PULLUP_DEC_S[1], "PULLUP", 9),
                        builder.bel_indexed(bslots::PULLUP_DEC_N[1], "PULLUP", 4),
                        builder.bel_indexed(bslots::PULLUP_DEC_S[2], "PULLUP", 8),
                        builder.bel_indexed(bslots::PULLUP_DEC_N[2], "PULLUP", 5),
                        builder.bel_indexed(bslots::PULLUP_DEC_S[3], "PULLUP", 7),
                        builder.bel_indexed(bslots::PULLUP_DEC_N[3], "PULLUP", 6),
                    ]);
                }
                _ => (),
            }
            let mut xt = builder
                .xtile_id(tcid, naming, crd)
                .num_cells(2)
                .ref_single(xy_s, 0, naming_s)
                .ref_single(xy_n, 1, naming_n)
                .extract_muxes(bslots::LLV)
                .bels(bels);
            if tcid == tcls::LLVC_IO_E {
                let bel = xt.builder.bel_virtual(bslots::MISC_E);
                xt = xt.bel(bel);
            }
            xt.extract();
        }
    }
}

fn extract_llvq(builder: &mut IntBuilder) {
    for (tcid, naming, tkn) in [
        (tcls::LLVQ_CLB, "LLVQ_CLB", "HVBRK"),
        (tcls::LLVQ_IO_SW, "LLVQ_IO_SW", "LHVBRK"),
        (tcls::LLVQ_IO_NW, "LLVQ_IO_NW", "LVLBRK"),
        (tcls::LLVQ_IO_SE, "LLVQ_IO_SE_L", "RHVBRK"),
        (tcls::LLVQ_IO_SE, "LLVQ_IO_SE_S", "RHVBRKS"),
        (tcls::LLVQ_IO_NE, "LLVQ_IO_NE_L", "RVRBRK"),
        (tcls::LLVQ_IO_NE, "LLVQ_IO_NE_S", "RVRBRKS"),
    ] {
        if let Some(&crd) = builder.rd.tiles_by_kind_name(tkn).first() {
            let xy_s = builder.walk_to_int(crd, Dir::S, true).unwrap();
            let xy_n = builder.walk_to_int(crd, Dir::N, true).unwrap();
            let naming_s = get_tile_naming(builder, xy_s);
            let naming_n = get_tile_naming(builder, xy_n);
            let mut bels = vec![];
            if tcid != tcls::LLVQ_CLB {
                bels.push(
                    builder
                        .bel_single(bslots::BUFF, "BUFF")
                        .pin_name_only("I", 1),
                );
            }
            builder
                .xtile_id(tcid, naming, crd)
                .num_cells(2)
                .ref_single(xy_s, 0, naming_s)
                .ref_single(xy_n, 1, naming_n)
                .extract_muxes(bslots::LLV)
                .bels(bels)
                .extract();
        }
    }
}

fn extract_clkqc(builder: &mut IntBuilder) {
    let hvbrk = builder.ndb.get_tile_class_naming("LLVQ_CLB");
    for (naming, tkn) in [("CLKQC_S", "HVBRKC"), ("CLKQC_N", "TVBRKC")] {
        if let Some(&crd) = builder.rd.tiles_by_kind_name(tkn).first() {
            builder
                .xtile_id(tcls::CLKQC, naming, crd)
                .ref_xlat(crd.delta(1, 0), &[None, Some(0)], hvbrk)
                .extract_muxes(bslots::CLKQC)
                .extract();
        }
    }
}

fn extract_clkq(builder: &mut IntBuilder) {
    let hvbrk = builder.ndb.get_tile_class_naming("LLVQ_CLB");
    for (naming, tkn) in [("CLKQ_S", "BCCBRK"), ("CLKQ_N", "TCCBRK")] {
        if let Some(&crd) = builder.rd.tiles_by_kind_name(tkn).first() {
            builder
                .xtile_id(tcls::CLKQ, naming, crd)
                .num_cells(2)
                .ref_xlat(crd.delta(-1, 0), &[None, Some(0)], hvbrk)
                .ref_xlat(crd.delta(2, 0), &[None, Some(1)], hvbrk)
                .extract_muxes(bslots::CLKQ)
                .extract();
        }
    }
}

pub fn make_int_db(rd: &Part) -> (IntDb, NamingDb) {
    let mut builder = IntBuilder::new(
        rd,
        bincode::decode_from_slice(
            match rd.family.as_str() {
                "xc4000e" => defs::xc4000e::INIT,
                "xc4000ex" => defs::xc4000ex::INIT,
                "xc4000xv" => defs::xc4000xv::INIT,
                "xc4000xla" => defs::xc4000xla::INIT,
                "spartanxl" => defs::spartanxl::INIT,
                _ => unreachable!(),
            },
            bincode::config::standard(),
        )
        .unwrap()
        .0,
    );

    fill_tie_wires(&mut builder);
    fill_single_wires(&mut builder);
    fill_double_wires(&mut builder);
    fill_io_double_wires(&mut builder);
    fill_quad_wires(&mut builder);
    fill_octal_wires(&mut builder);
    fill_io_octal_wires(&mut builder);
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
        builder.inject_int_type(tkn);
    }

    let mut force_names = vec![];
    for (pin, tpin) in [("F2", "O_1"), ("G2", "O_2"), ("C2", "TXIN2")] {
        let w = builder.db.get_wire(&format!("IMUX_CLB_{pin}"));
        force_names.push((1, format!("CENTER_{pin}"), w));
        // force_names.push((2, format!("CENTER_{pin}L"), w));
        for kind in TOP_KINDS {
            force_names.push((1, format!("{kind}_{tpin}"), w));
        }
    }
    if builder.rd.family == "xc4000e" {
        for (name, wire) in xc4000e_wires::xc4000e_wires() {
            let xwire = match wire {
                wires::IMUX_CLB_F2_N => wires::IMUX_CLB_F2,
                wires::IMUX_CLB_G2_N => wires::IMUX_CLB_G2,
                wires::IMUX_CLB_C2_N => wires::IMUX_CLB_C2,
                _ => continue,
            };
            force_names.push((1, name.to_string(), xwire));
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
            extract_clkqc(&mut builder);
        } else {
            extract_clkq(&mut builder);
        }
    }

    for (&(tcid, bslot), pips) in &mut builder.pips {
        let tcname = builder.db.tile_classes.key(tcid);
        for (&(wt, wf), mode) in &mut pips.pips {
            let wtn = builder.db.wires.key(wt.wire);
            let wfn = builder.db.wires.key(wf.wire);
            if *mode == PipMode::PermaBuf {
                continue;
            }
            if wtn.starts_with("BUFGE") || wires::BUFGLS_H.contains(wt.wire) {
                *mode = PipMode::PermaBuf;
            } else if wtn.starts_with("SINGLE")
                || wtn.starts_with("DOUBLE")
                || wtn.starts_with("IO.DOUBLE")
                || wtn.starts_with("QUAD")
                || wtn.starts_with("DEC")
                || (wtn.starts_with("LONG")
                    && bslot != bslots::INT
                    && builder.rd.family == "xc4000e")
            {
                *mode = PipMode::Pass;
            } else if wtn.starts_with("LONG")
                && wfn.starts_with("LONG")
                && !wtn.ends_with("EXCL")
                && !wfn.ends_with("EXCL")
                && bslot != bslots::INT
            {
                if !matches!(builder.rd.family.as_str(), "xc4000e" | "spartanxl")
                    && tcname.starts_with("LLVQ")
                    && ((wt.wire == wires::LONG_V[9] && wt.cell.to_idx() == 0)
                        || (wt.wire == wires::LONG_V[7] && wt.cell.to_idx() == 1))
                {
                    *mode = PipMode::Mux;
                } else {
                    *mode = PipMode::Buf;
                }
            } else if wtn.starts_with("LONG_IO") && wfn.starts_with("QUAD") {
                *mode = PipMode::Pass;
            } else if wtn.starts_with("OCTAL_IO")
                && wfn.starts_with("OCTAL_IO")
                && builder.rd.family != "xc4000xv"
            {
                *mode = PipMode::Buf;
            } else if wtn.starts_with("OCTAL_IO") && wfn.starts_with("SINGLE") {
                *mode = PipMode::Pass;
            } else if wtn.starts_with("LONG")
                && !wtn.starts_with("LONG_IO")
                && (wfn.starts_with("SINGLE") || wf.wire == wires::OUT_COUT_E)
            {
                *mode = PipMode::Buf;
            } else if wt.wire == wires::IMUX_CIN && builder.rd.family == "spartanxl" {
                *mode = PipMode::PermaBuf;
            } else if wtn.starts_with("OCTAL") {
                *mode = PipMode::Buf;
            }
        }
    }

    let (mut intdb, mut naming) = builder.build();

    if rd.family == "spartanxl" {
        for tcid in [tcls::LLV_CLB, tcls::LLV_IO_W, tcls::LLV_IO_E] {
            let tcls = &mut intdb.tile_classes[tcid];
            let BelInfo::SwitchBox(ref mut sb) = tcls.bels[bslots::LLV] else {
                unreachable!()
            };
            for item in &mut sb.items {
                let SwitchBoxItem::Mux(mux) = item else {
                    continue;
                };
                let srcs = std::mem::take(&mut mux.src);
                mux.src = srcs
                    .into_iter()
                    .map(|(mut src, bits)| {
                        let idx = wires::BUFGLS.index_of(src.wire).unwrap();
                        src.wire = wires::BUFGLS_H[idx];
                        (src, bits)
                    })
                    .collect();
            }
            for idx in 0..8 {
                sb.items.push(SwitchBoxItem::ProgBuf(ProgBuf {
                    dst: TileWireCoord::new_idx(0, wires::BUFGLS_H[idx]),
                    src: TileWireCoord::new_idx(0, wires::BUFGLS[idx]).pos(),
                    bit: PolTileBit::DUMMY,
                }));
            }
            sb.items.sort();
        }
        for key in ["LLV_CLB", "LLV_IO_W", "LLV_IO_E"] {
            let ntcls = naming.tile_class_namings.get_mut(key).unwrap().1;
            for i in 0..8 {
                let name = ntcls.wires[&TileWireCoord::new_idx(0, wires::BUFGLS[i])].clone();
                ntcls
                    .wires
                    .insert(TileWireCoord::new_idx(0, wires::BUFGLS_H[i]), name);
            }
        }
    }
    fn is_octal(w: TileWireCoord) -> bool {
        wires::OCTAL_H.contains(w.wire)
            || wires::OCTAL_V.contains(w.wire)
            || wires::OCTAL_IO_W.contains(w.wire)
            || wires::OCTAL_IO_E.contains(w.wire)
            || wires::OCTAL_IO_S.contains(w.wire)
            || wires::OCTAL_IO_N.contains(w.wire)
    }
    if rd.family == "xc4000xv" {
        for tcls in intdb.tile_classes.values_mut() {
            if !tcls.bels.contains_id(bslots::IO[0]) {
                continue;
            }
            let mut obuf_outs = BTreeSet::new();
            let mut obuf_ins = BTreeSet::new();
            let BelInfo::SwitchBox(ref mut sb) = tcls.bels[bslots::INT] else {
                unreachable!()
            };
            sb.items.retain(|item| {
                if let SwitchBoxItem::ProgBuf(buf) = item
                    && is_octal(buf.dst)
                    && (is_octal(buf.src.tw) || buf.src.wire == wires::TIE_0)
                {
                    if buf.src.wire != wires::TIE_0 {
                        obuf_outs.insert(buf.dst);
                        obuf_ins.insert(buf.src);
                    }
                    false
                } else {
                    true
                }
            });
            let obuf = TileWireCoord::new_idx(0, wires::OBUF);
            for dst in obuf_outs {
                sb.items.push(SwitchBoxItem::ProgBuf(ProgBuf {
                    dst,
                    src: obuf.pos(),
                    bit: PolTileBit::DUMMY,
                }));
            }
            for src in obuf_ins {
                sb.items.push(SwitchBoxItem::ProgBuf(ProgBuf {
                    dst: obuf,
                    src,
                    bit: PolTileBit::DUMMY,
                }));
            }
            sb.items.sort();
        }
    }

    let mut injected_specials: BTreeMap<(TileClassId, TileWireCoord), Vec<_>> = BTreeMap::new();
    for tcid in [
        tcls::CLB,
        tcls::CLB_W,
        tcls::CLB_E,
        tcls::CLB_S,
        tcls::CLB_SW,
        tcls::CLB_SE,
        tcls::CLB_N,
        tcls::CLB_NW,
        tcls::CLB_NE,
    ] {
        injected_specials
            .entry((tcid, TileWireCoord::new_idx(0, wires::IMUX_CLB_F4)))
            .or_default()
            .push(TileWireCoord::new_idx(0, wires::SPECIAL_CLB_CIN).pos());
    }
    for tcid in [
        tcls::CLB,
        tcls::CLB_E,
        tcls::CLB_S,
        tcls::CLB_SE,
        tcls::CLB_N,
        tcls::CLB_NE,
        tcls::IO_E0,
        tcls::IO_E0_N,
        tcls::IO_E0_F0,
        tcls::IO_E0_F1,
        tcls::IO_E1,
        tcls::IO_E1_S,
        tcls::IO_E1_F0,
        tcls::IO_E1_F1,
    ] {
        injected_specials
            .entry((tcid, TileWireCoord::new_idx(0, wires::IMUX_CLB_G3)))
            .or_default()
            .push(TileWireCoord::new_idx(0, wires::SPECIAL_CLB_CIN).pos());
    }
    for tcid in [
        tcls::CLB,
        tcls::CLB_W,
        tcls::CLB_E,
        tcls::CLB_S,
        tcls::CLB_SW,
        tcls::CLB_SE,
        tcls::IO_S0,
        tcls::IO_S0_E,
        tcls::IO_S1,
        tcls::IO_S1_W,
    ] {
        injected_specials
            .entry((tcid, TileWireCoord::new_idx(0, wires::IMUX_CLB_G2)))
            .or_default()
            .push(TileWireCoord::new_idx(0, wires::SPECIAL_CLB_COUT0).pos());
    }
    for (tcid, _, tcls) in &intdb.tile_classes {
        for idx in 0..2 {
            if tcls.bels.contains_id(bslots::TBUF[idx]) {
                injected_specials
                    .entry((tcid, TileWireCoord::new_idx(0, wires::IMUX_TBUF_I[idx])))
                    .or_default()
                    .push(TileWireCoord::new_idx(0, wires::TIE_0).pos());
                injected_specials
                    .entry((tcid, TileWireCoord::new_idx(0, wires::IMUX_TBUF_T[idx])))
                    .or_default()
                    .push(TileWireCoord::new_idx(0, wires::TIE_0).pos());
                injected_specials
                    .entry((tcid, TileWireCoord::new_idx(0, wires::IMUX_TBUF_T[idx])))
                    .or_default()
                    .push(TileWireCoord::new_idx(0, wires::TIE_1).pos());
            }
        }
        if tcls.bels.contains_id(bslots::IO[0]) {
            for idx in 0..2 {
                injected_specials
                    .entry((tcid, TileWireCoord::new_idx(0, wires::IMUX_IO_O1[idx])))
                    .or_default()
                    .push(TileWireCoord::new_idx(0, wires::TIE_0).pos());
                injected_specials
                    .entry((tcid, TileWireCoord::new_idx(0, wires::IMUX_IO_T[idx])))
                    .or_default()
                    .push(TileWireCoord::new_idx(0, wires::TIE_0).pos());
            }
        }
    }

    for (tcid, _, tcls) in &mut intdb.tile_classes {
        for bel in tcls.bels.values_mut() {
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
            for item in &mut sb.items {
                let SwitchBoxItem::Mux(mux) = item else {
                    continue;
                };
                let Some(new_srcs) = injected_specials.get(&(tcid, mux.dst)) else {
                    continue;
                };
                for &src in new_srcs {
                    mux.src.insert(src, Default::default());
                }
            }
        }
    }

    (intdb, naming)
}
