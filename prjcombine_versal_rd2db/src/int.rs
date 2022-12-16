use enum_map::EnumMap;
use prjcombine_entity::{EntityId, EntityPartVec};
use prjcombine_int::db::{Dir, IntDb, NodeTileId, TermInfo, TermKind, WireKind};
use prjcombine_rawdump::{Coord, Part, TkWire};
use prjcombine_versal::expand::{
    BUFDIV_LEAF_SWZ_A, BUFDIV_LEAF_SWZ_AH, BUFDIV_LEAF_SWZ_B, BUFDIV_LEAF_SWZ_BH,
};
use std::collections::HashMap;

use prjcombine_rdintb::IntBuilder;

pub fn make_int_db(rd: &Part) -> IntDb {
    let mut builder = IntBuilder::new("versal", rd);
    let mut term_wires: EnumMap<Dir, EntityPartVec<_, _>> = Default::default();
    let intf_kinds = [
        (Dir::W, "INTF_LOCF_BL_TILE", "INTF.W", false),
        (Dir::W, "INTF_LOCF_TL_TILE", "INTF.W", false),
        (Dir::E, "INTF_LOCF_BR_TILE", "INTF.E", false),
        (Dir::E, "INTF_LOCF_TR_TILE", "INTF.E", false),
        (Dir::W, "INTF_ROCF_BL_TILE", "INTF.W", false),
        (Dir::W, "INTF_ROCF_TL_TILE", "INTF.W", false),
        (Dir::E, "INTF_ROCF_BR_TILE", "INTF.E", false),
        (Dir::E, "INTF_ROCF_TR_TILE", "INTF.E", false),
        (Dir::W, "INTF_HB_LOCF_BL_TILE", "INTF.W.HB", false),
        (Dir::W, "INTF_HB_LOCF_TL_TILE", "INTF.W.HB", false),
        (Dir::E, "INTF_HB_LOCF_BR_TILE", "INTF.E.HB", false),
        (Dir::E, "INTF_HB_LOCF_TR_TILE", "INTF.E.HB", false),
        (Dir::W, "INTF_HB_ROCF_BL_TILE", "INTF.W.HB", false),
        (Dir::W, "INTF_HB_ROCF_TL_TILE", "INTF.W.HB", false),
        (Dir::E, "INTF_HB_ROCF_BR_TILE", "INTF.E.HB", false),
        (Dir::E, "INTF_HB_ROCF_TR_TILE", "INTF.E.HB", false),
        (Dir::W, "INTF_HDIO_LOCF_BL_TILE", "INTF.W.HDIO", false),
        (Dir::W, "INTF_HDIO_LOCF_TL_TILE", "INTF.W.HDIO", false),
        (Dir::E, "INTF_HDIO_LOCF_BR_TILE", "INTF.E.HDIO", false),
        (Dir::E, "INTF_HDIO_LOCF_TR_TILE", "INTF.E.HDIO", false),
        (Dir::W, "INTF_HDIO_ROCF_BL_TILE", "INTF.W.HDIO", false),
        (Dir::W, "INTF_HDIO_ROCF_TL_TILE", "INTF.W.HDIO", false),
        (Dir::E, "INTF_HDIO_ROCF_BR_TILE", "INTF.E.HDIO", false),
        (Dir::E, "INTF_HDIO_ROCF_TR_TILE", "INTF.E.HDIO", false),
        (Dir::W, "INTF_CFRM_BL_TILE", "INTF.W.PSS", false),
        (Dir::W, "INTF_CFRM_TL_TILE", "INTF.W.PSS", false),
        (Dir::W, "INTF_PSS_BL_TILE", "INTF.W.TERM.PSS", true),
        (Dir::W, "INTF_PSS_TL_TILE", "INTF.W.TERM.PSS", true),
        (Dir::W, "INTF_GT_BL_TILE", "INTF.W.TERM.GT", true),
        (Dir::W, "INTF_GT_TL_TILE", "INTF.W.TERM.GT", true),
        (Dir::E, "INTF_GT_BR_TILE", "INTF.E.TERM.GT", true),
        (Dir::E, "INTF_GT_TR_TILE", "INTF.E.TERM.GT", true),
    ];
    let bli_cle_intf_kinds = [
        (Dir::E, "BLI_CLE_BOT_CORE", "INTF.BLI_CLE.BOT.E", false),
        (Dir::E, "BLI_CLE_TOP_CORE", "INTF.BLI_CLE.TOP.E", true),
        (Dir::W, "BLI_CLE_BOT_CORE_MY", "INTF.BLI_CLE.BOT.W", false),
        (Dir::W, "BLI_CLE_TOP_CORE_MY", "INTF.BLI_CLE.TOP.W", true),
    ];

    builder.wire("VCC", WireKind::Tie1, &["VCC_WIRE"]);

    for (iq, q) in ['E', 'N', 'S', 'W'].into_iter().enumerate() {
        for (ih, h) in ['E', 'W'].into_iter().enumerate() {
            for i in 0..32 {
                match (q, i) {
                    ('E', 0 | 2) | ('W', 0 | 2) | ('N', 0) => {
                        let w = builder.mux_out(
                            format!("SDQNODE.{q}.{h}.{i}"),
                            &[format!("OUT_{q}NODE_{h}_{i}")],
                        );
                        builder.branch(
                            w,
                            Dir::S,
                            format!("SDQNODE.{q}.{h}.{i}.S"),
                            &[format!("IN_{q}NODE_{h}_BLS_{i}")],
                        );
                    }
                    ('E', 29 | 31) | ('W', 31) | ('S', 31) => {
                        let w = builder.mux_out(
                            format!("SDQNODE.{q}.{h}.{i}"),
                            &[format!("OUT_{q}NODE_{h}_{i}")],
                        );
                        builder.branch(
                            w,
                            Dir::N,
                            format!("SDQNODE.{q}.{h}.{i}.N"),
                            &[format!("IN_{q}NODE_{h}_BLN_{i}")],
                        );
                    }
                    _ => {
                        // TODO not the true permutation
                        let a = [0, 11, 1, 2, 3, 4, 5, 6, 7, 8, 9, 13, 14, 15, 10, 12][i >> 1];
                        let aa = a + ih * 16 + iq * 32;
                        let b = i & 1;
                        builder.mux_out(
                            format!("SDQNODE.{q}.{h}.{i}"),
                            &[format!("INT_NODE_SDQ_ATOM_{aa}_INT_OUT{b}")],
                        );
                    }
                }
            }
        }
    }

    for i in 0..48 {
        builder.mux_out(
            format!("SDQ_RED.{i}"),
            &[format!("INT_SDQ_RED_ATOM_{i}_INT_OUT0")],
        );
    }
    for (fwd, name, l, ll, num) in [
        (Dir::E, "SNG", 1, 1, 16),
        (Dir::N, "SNG", 1, 1, 16),
        (Dir::E, "DBL", 1, 2, 8),
        (Dir::N, "DBL", 2, 2, 8),
        (Dir::E, "QUAD", 2, 4, 8),
        (Dir::N, "QUAD", 4, 4, 8),
    ] {
        let bwd = !fwd;
        for ew_f in [Dir::E, Dir::W] {
            let ew_b = if fwd == Dir::E { !ew_f } else { ew_f };
            for i in 0..num {
                if ll == 1 && fwd == Dir::E && ew_f == Dir::W {
                    continue;
                }
                let mut w_f = builder.mux_out(
                    format!("{name}.{fwd}.{ew_f}.{i}.0"),
                    &[format!("OUT_{fwd}{fwd}{ll}_{ew_f}_BEG{i}")],
                );
                let mut w_b = builder.mux_out(
                    format!("{name}.{bwd}.{ew_b}.{i}.0"),
                    &[format!("OUT_{bwd}{bwd}{ll}_{ew_b}_BEG{i}")],
                );
                match (fwd, ew_f, ll) {
                    (Dir::E, Dir::E, 1) => {
                        let ii = i;
                        builder.extra_name(format!("IF_HBUS_EBUS{ii}"), w_f);
                        builder.extra_name(format!("IF_HBUS_W_EBUS{ii}"), w_f);
                        builder.extra_name(format!("IF_HBUS_WBUS{ii}"), w_b);
                        builder.extra_name(format!("IF_HBUS_E_WBUS{ii}"), w_b);
                    }
                    (Dir::E, Dir::W, 2) => {
                        let ii = i + 24;
                        builder.extra_name(format!("IF_HBUS_EBUS{ii}"), w_f);
                        builder.extra_name(format!("IF_HBUS_W_EBUS{ii}"), w_f);
                        let ii = i + 16;
                        builder.extra_name(format!("IF_HBUS_WBUS{ii}"), w_b);
                        builder.extra_name(format!("IF_HBUS_E_WBUS{ii}"), w_b);
                    }
                    _ => (),
                }
                if bwd == Dir::W && i == 0 && ll == 1 {
                    let w =
                        builder.branch(w_b, Dir::S, format!("{name}.{bwd}.{ew_b}.{i}.0.S"), &[""]);
                    builder.extra_name_tile_sub("CLE_BC_CORE", "BNODE_TAP0", 1, w);
                    builder.extra_name_tile_sub("SLL", "BNODE_TAP0", 1, w);
                }
                for j in 1..l {
                    let n_f =
                        builder.branch(w_f, fwd, format!("{name}.{fwd}.{ew_f}.{i}.{j}"), &[""]);
                    let n_b =
                        builder.branch(w_b, bwd, format!("{name}.{bwd}.{ew_b}.{i}.{j}"), &[""]);
                    match (fwd, ew_f, ll, j) {
                        (Dir::E, Dir::W, 4, 1) => {
                            let ii = i + 40;
                            builder.extra_name(format!("IF_HBUS_WBUS{ii}"), n_b);
                            builder.extra_name(format!("IF_HBUS_E_WBUS{ii}"), n_b);
                            let ii = i + 56;
                            builder.extra_name(format!("IF_HBUS_EBUS{ii}"), n_f);
                            builder.extra_name(format!("IF_HBUS_W_EBUS{ii}"), n_f);
                            if i == 0 {
                                let w = builder.branch(
                                    n_f,
                                    Dir::S,
                                    format!("{name}.{fwd}.{ew_f}.{i}.{j}.S"),
                                    &[""],
                                );
                                for (dir, tkn, _, _) in intf_kinds {
                                    if dir == Dir::E {
                                        builder.extra_name_tile(tkn, "IF_LBC_N_BNODE_SOUTHBUS", w);
                                    }
                                }
                            }
                        }
                        (Dir::N, Dir::E, 2, 1) => {
                            let ii = i + 32;
                            builder.extra_name(format!("IF_VBUS_S_NBUS{ii}"), n_f);
                        }
                        (Dir::N, Dir::W, 2, 1) => {
                            let ii = i + 48;
                            builder.extra_name(format!("IF_VBUS_S_NBUS{ii}"), n_f);
                        }
                        (Dir::N, Dir::E, 4, 1) => {
                            let ii = i + 64;
                            builder.extra_name(format!("IF_VBUS_S_NBUS{ii}"), n_f);
                        }
                        (Dir::N, Dir::W, 4, 1) => {
                            let ii = i + 96;
                            builder.extra_name(format!("IF_VBUS_S_NBUS{ii}"), n_f);
                        }
                        _ => (),
                    }
                    term_wires[fwd].insert(n_b, TermInfo::PassNear(w_f));
                    term_wires[bwd].insert(n_f, TermInfo::PassNear(w_b));
                    w_f = n_f;
                    w_b = n_b;
                }
                let e_f = builder.branch(
                    w_f,
                    fwd,
                    format!("{name}.{fwd}.{ew_f}.{i}.{l}"),
                    &[format!("IN_{fwd}{fwd}{ll}_{ew_f}_END{i}")],
                );
                let e_b = builder.branch(
                    w_b,
                    bwd,
                    format!("{name}.{bwd}.{ew_b}.{i}.{l}"),
                    &[format!("IN_{bwd}{bwd}{ll}_{ew_b}_END{i}")],
                );
                match (fwd, ew_f, ll) {
                    (Dir::N, _, 1) => {
                        for (dir, tkn, _, _) in intf_kinds {
                            if dir == ew_f {
                                let ii = i;
                                builder.extra_name_tile(tkn, format!("IF_INT_VSINGLE{ii}"), e_b);
                                let ii = i + 16;
                                builder.extra_name_tile(tkn, format!("IF_INT_VSINGLE{ii}"), e_f);
                            }
                        }
                    }
                    (Dir::E, Dir::E, 2) => {
                        let ii = i + 16;
                        for (_, tkn, _, term) in intf_kinds {
                            if !term {
                                builder.extra_name_tile(tkn, format!("IF_HBUS_EBUS{ii}"), e_f);
                                builder.extra_name_tile(tkn, format!("IF_HBUS_W_EBUS{ii}"), e_f);
                            } else {
                                builder.extra_name_tile(tkn, format!("IF_HBUS_EBUS{ii}"), e_b);
                            }
                        }
                        let ii = i + 24;
                        for (_, tkn, _, term) in intf_kinds {
                            if !term {
                                builder.extra_name_tile(tkn, format!("IF_HBUS_WBUS{ii}"), e_b);
                                builder.extra_name_tile(tkn, format!("IF_HBUS_E_WBUS{ii}"), e_b);
                            } else {
                                builder.extra_name_tile(tkn, format!("IF_HBUS_WBUS{ii}"), e_f);
                            }
                        }
                    }
                    (Dir::E, Dir::E, 4) => {
                        let ii = i + 40;
                        for (_, tkn, _, term) in intf_kinds {
                            if !term {
                                builder.extra_name_tile(tkn, format!("IF_HBUS_EBUS{ii}"), e_f);
                                builder.extra_name_tile(tkn, format!("IF_HBUS_W_EBUS{ii}"), e_f);
                            } else {
                                builder.extra_name_tile(tkn, format!("IF_HBUS_EBUS{ii}"), e_b);
                            }
                        }
                        if i == 0 {
                            let w = builder.branch(
                                e_f,
                                Dir::S,
                                format!("{name}.{fwd}.{ew_f}.{i}.{l}.S"),
                                &[""],
                            );
                            for (dir, tkn, _, _) in intf_kinds {
                                if dir == Dir::W {
                                    builder.extra_name_tile(tkn, "IF_LBC_N_BNODE_SOUTHBUS", w);
                                }
                            }
                        }
                        let ii = i + 56;
                        for (_, tkn, _, term) in intf_kinds {
                            if !term {
                                builder.extra_name_tile(tkn, format!("IF_HBUS_WBUS{ii}"), e_b);
                                builder.extra_name_tile(tkn, format!("IF_HBUS_E_WBUS{ii}"), e_b);
                            } else {
                                builder.extra_name_tile(tkn, format!("IF_HBUS_WBUS{ii}"), e_f);
                            }
                        }
                    }
                    _ => (),
                }
                term_wires[fwd].insert(e_b, TermInfo::PassNear(w_f));
                term_wires[bwd].insert(e_f, TermInfo::PassNear(w_b));
            }
        }
    }

    for (fwd, name, l, ll) in [
        (Dir::E, "LONG.6", 3, 6),
        (Dir::N, "LONG.7", 7, 7),
        (Dir::E, "LONG.10", 5, 10),
        (Dir::N, "LONG.12", 12, 12),
    ] {
        let bwd = !fwd;
        for i in 0..8 {
            let mut w_f = builder.mux_out(
                format!("{name}.{fwd}.{i}.0"),
                &[format!("OUT_{fwd}{fwd}{ll}_BEG{i}")],
            );
            let mut w_b = builder.mux_out(
                format!("{name}.{bwd}.{i}.0"),
                &[format!("OUT_{bwd}{bwd}{ll}_BEG{i}")],
            );
            for j in 1..l {
                let n_f = builder.branch(w_f, fwd, format!("{name}.{fwd}.{i}.{j}"), &[""]);
                let n_b = builder.branch(w_b, bwd, format!("{name}.{bwd}.{i}.{j}"), &[""]);
                term_wires[fwd].insert(n_b, TermInfo::PassNear(w_f));
                term_wires[bwd].insert(n_f, TermInfo::PassNear(w_b));
                w_f = n_f;
                w_b = n_b;
            }
            let e_f = builder.branch(
                w_f,
                fwd,
                format!("{name}.{fwd}.{i}.{l}"),
                &[format!("IN_{fwd}{fwd}{ll}_END{i}")],
            );
            let e_b = builder.branch(
                w_b,
                bwd,
                format!("{name}.{bwd}.{i}.{l}"),
                &[format!("IN_{bwd}{bwd}{ll}_END{i}")],
            );
            term_wires[fwd].insert(e_b, TermInfo::PassNear(w_f));
            term_wires[bwd].insert(e_f, TermInfo::PassNear(w_b));
            if i == 0 && fwd == Dir::E && ll == 6 {
                builder.branch(
                    e_f,
                    Dir::S,
                    format!("{name}.{fwd}.{i}.{l}.S"),
                    &[format!("IN_{fwd}{fwd}{ll}_BLS_{i}")],
                );
            }
            if i == 7 && fwd == Dir::E && ll == 10 {
                builder.branch(
                    e_f,
                    Dir::N,
                    format!("{name}.{fwd}.{i}.{l}.N"),
                    &[format!("IN_{fwd}{fwd}{ll}_BLN_{i}")],
                );
            }
        }
    }

    for i in 0..128 {
        for j in 0..2 {
            builder.mux_out(
                format!("INODE.{i}.{j}"),
                &[format!("INT_NODE_IMUX_ATOM_{i}_INT_OUT{j}")],
            );
        }
    }

    for ew in ['E', 'W'] {
        for i in 0..96 {
            builder.mux_out(format!("IMUX.{ew}.IMUX.{i}"), &[format!("IMUX_B_{ew}{i}")]);
        }
    }

    let mut bounces = Vec::new();
    for ew in ['E', 'W'] {
        for i in 0..32 {
            let w = builder.mux_out(format!("IMUX.{ew}.BOUNCE.{i}"), &[""]);
            builder.extra_name_tile("INT", format!("BOUNCE_{ew}{i}"), w);
            bounces.push(w);
        }
    }

    let mut bnodes = Vec::new();
    for dir in [Dir::E, Dir::W] {
        for i in 0..64 {
            let w = builder.wire(
                format!("BNODE.{dir}.{i}"),
                WireKind::Branch(dir),
                &[format!("BNODE_{dir}{i}")],
            );
            bnodes.push(w);
        }
    }

    let mut logic_outs_w = EntityPartVec::new();
    let mut logic_outs_e = EntityPartVec::new();
    for (sub, ew) in [Dir::E, Dir::W].into_iter().enumerate() {
        let we = !ew;
        for i in 0..48 {
            let w = builder.logic_out(format!("OUT.{ew}.{i}"), &[""]);
            builder.extra_name_tile("INT", format!("LOGIC_OUTS_{ew}{i}"), w);
            match (ew, i) {
                (Dir::E, 1 | 4 | 5) | (Dir::W, 4 | 5) => {
                    builder.branch(
                        w,
                        Dir::S,
                        format!("OUT.{ew}.{i}.S"),
                        &[format!("IN_LOGIC_OUTS_{ew}_BLS_{i}")],
                    );
                }
                _ => (),
            }
            let cw = builder.wire(format!("CLE.OUT.{ew}.{i}"), WireKind::Branch(ew), &[""]);
            builder.test_mux_pass(cw);
            builder.extra_name_tile_sub("CLE_BC_CORE", format!("LOGIC_OUTS_{we}{i}"), sub, cw);
            builder.extra_name_tile_sub("SLL", format!("LOGIC_OUTS_{we}{i}"), sub, cw);
            if ew == Dir::E {
                logic_outs_e.insert(cw, TermInfo::PassNear(w));
            } else {
                logic_outs_w.insert(cw, TermInfo::PassNear(w));
            }
        }
    }

    for ew in [Dir::E, Dir::W] {
        let w = builder.test_out(format!("TEST.{ew}.TMR_DFT"), &[""]);

        for (dir, tkn, _, _) in intf_kinds {
            if dir == ew {
                builder.extra_name_tile(tkn, "INTF_MUX2_TMR_GREEN_TMR_DFT", w);
            }
        }
        for (dir, tkn, _, _) in bli_cle_intf_kinds {
            if dir == ew {
                for i in 0..4 {
                    builder.extra_name_tile(tkn, format!("INTF_MUX2_TMR_GREEN_{i}_TMR_DFT"), w);
                }
            }
        }
    }

    let mut bnode_outs = Vec::new();
    for (sub, ew) in [Dir::E, Dir::W].into_iter().enumerate() {
        let we = !ew;
        for i in 0..32 {
            let w = builder.mux_out(format!("CLE.BNODE.{ew}.{i}"), &[""]);
            builder.extra_name_sub(format!("BNODE_OUTS_{we}{i}"), sub, w);
            bnode_outs.push(w);
        }
    }

    for (sub, ew) in [Dir::E, Dir::W].into_iter().enumerate() {
        let we = !ew;
        for i in 0..12 {
            let w = builder.mux_out(format!("CLE.CNODE.{ew}.{i}"), &[""]);
            builder.extra_name_sub(format!("CNODE_OUTS_{we}{i}"), sub, w);
            bnode_outs.push(w);
        }
    }

    for (sub, ew) in [Dir::E, Dir::W].into_iter().enumerate() {
        let lr = match ew {
            Dir::E => 'L',
            Dir::W => 'R',
            _ => unreachable!(),
        };
        for i in 0..13 {
            let w = builder.mux_out(format!("CLE.IMUX.{ew}.CTRL.{i}"), &[""]);
            builder.extra_name_sub(format!("CTRL_{lr}_B{i}"), sub, w);
            builder.extra_name_tile(
                match ew {
                    Dir::E => "CLE_W_CORE",
                    Dir::W => "CLE_E_CORE",
                    _ => unreachable!(),
                },
                match i {
                    0 => "CLE_SLICEL_TOP_0_CLK",
                    1 => "CLE_SLICEM_TOP_1_CLK",
                    2 => "CLE_SLICEL_TOP_0_RST",
                    3 => "CLE_SLICEM_TOP_1_RST",
                    4 => "CLE_SLICEL_TOP_0_CKEN1",
                    5 => "CLE_SLICEL_TOP_0_CKEN2",
                    6 => "CLE_SLICEL_TOP_0_CKEN3",
                    7 => "CLE_SLICEL_TOP_0_CKEN4",
                    8 => "CLE_SLICEM_TOP_1_WE",
                    9 => "CLE_SLICEM_TOP_1_CKEN1",
                    10 => "CLE_SLICEM_TOP_1_CKEN2",
                    11 => "CLE_SLICEM_TOP_1_CKEN3",
                    12 => "CLE_SLICEM_TOP_1_CKEN4",
                    _ => unreachable!(),
                },
                w,
            );
        }
    }

    for ew in [Dir::E, Dir::W] {
        for i in 0..4 {
            for j in 1..4 {
                let w = builder.wire(
                    format!("BLI_CLE.IMUX.{ew}.IRI{i}.FAKE_CE{j}"),
                    WireKind::Tie0,
                    &[""],
                );
                for (dir, tkn, _, _) in bli_cle_intf_kinds {
                    if dir == ew {
                        let idxs = match (i, j) {
                            (0, 1) => [24, 30, 39, 45],
                            (0, 2) => [25, 31, 40, 46],
                            (0, 3) => [26, 32, 41, 47],
                            (1, 1) => [0, 6, 15, 21],
                            (1, 2) => [1, 7, 16, 22],
                            (1, 3) => [2, 8, 17, 23],
                            (2, 1) => [27, 33, 36, 42],
                            (2, 2) => [28, 34, 37, 43],
                            (2, 3) => [29, 35, 38, 44],
                            (3, 1) => [3, 9, 12, 18],
                            (3, 2) => [4, 10, 13, 19],
                            (3, 3) => [5, 11, 14, 20],
                            _ => unreachable!(),
                        };
                        for idx in idxs {
                            builder.extra_name_tile(tkn, format!("GND_WIRE{idx}"), w);
                        }
                    }
                }
            }
        }
    }

    for i in 0..16 {
        let w = builder.wire(format!("CLE.GCLK.{i}"), WireKind::ClkOut(32 + i), &[""]);
        builder.extra_name_sub(format!("GCLK_B{i}"), 1, w);
    }

    for ew in [Dir::W, Dir::E] {
        for i in 0..4 {
            let rg = match i % 2 {
                0 => "RED",
                1 => "GREEN",
                _ => unreachable!(),
            };
            let w = builder.mux_out(format!("INTF.{ew}.IMUX.IRI{i}.CLK"), &[""]);
            for (dir, tkn, _, _) in intf_kinds {
                if dir == ew {
                    builder.extra_name_tile(tkn, format!("INTF_IRI_QUADRANT_{rg}_{i}_CLK"), w);
                }
            }
            let w = builder.mux_out(format!("INTF.{ew}.IMUX.IRI{i}.RST"), &[""]);
            for (dir, tkn, _, _) in intf_kinds {
                if dir == ew {
                    builder.extra_name_tile(tkn, format!("INTF_IRI_QUADRANT_{rg}_{i}_RST"), w);
                }
            }
            for j in 0..4 {
                let w = builder.mux_out(format!("INTF.{ew}.IMUX.IRI{i}.CE{j}"), &[""]);
                for (dir, tkn, _, _) in intf_kinds {
                    if dir == ew {
                        builder.extra_name_tile(
                            tkn,
                            format!("INTF_IRI_QUADRANT_{rg}_{i}_CE{j}"),
                            w,
                        );
                    }
                }
            }
        }
    }

    for ew in [Dir::W, Dir::E] {
        for i in 0..12 {
            for j in 0..2 {
                let w = builder.mux_out(format!("INTF.{ew}.CNODE.{i}.{j}"), &[""]);
                for (dir, tkn, _, _) in intf_kinds {
                    if dir == ew {
                        builder.extra_name_tile(tkn, format!("INTF_CNODE_ATOM_{i}_INT_OUT{j}"), w);
                    }
                }
            }
        }
    }

    for (b, ew) in [Dir::W, Dir::E].into_iter().enumerate() {
        for i in 0..16 {
            let w = builder.wire(
                format!("INTF.{ew}.GCLK.{i}"),
                WireKind::ClkOut(b * 16 + i),
                &[""],
            );
            for (dir, tkn, _, _) in intf_kinds {
                if dir == ew {
                    builder.extra_name_tile(tkn, format!("IF_GCLK_GCLK_B{i}"), w);
                }
            }
        }
    }

    for i in 0..40 {
        for j in 0..2 {
            builder.mux_out(
                format!("RCLK.INODE.{i}.{j}"),
                &[format!("INT_NODE_IMUX_ATOM_RCLK_{i}_INT_OUT{j}")],
            );
        }
    }

    for ew in [Dir::W, Dir::E] {
        for i in 0..2 {
            for j in 0..20 {
                builder.mux_out(
                    format!("RCLK.IMUX.{ew}.{i}.{j}"),
                    &[format!("IF_INT2COE_{ew}_INT_RCLK_TO_CLK_B_{i}_{j}")],
                );
            }
        }
    }

    builder.extract_main_passes();

    let t = builder.db.terms.get("MAIN.W").unwrap().1.clone();
    builder.db.terms.insert("CLE.W".to_string(), t);
    let t = builder.db.terms.get("MAIN.E").unwrap().1.clone();
    builder.db.terms.insert("CLE.E".to_string(), t);

    builder.node_type("INT", "INT", "INT");

    for tkn in ["CLE_BC_CORE", "SLL"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let xy_l = builder.walk_to_int(xy, Dir::W).unwrap();
            let xy_r = builder.walk_to_int(xy, Dir::E).unwrap();
            builder.extract_xnode("CLE_BC", xy, &[], &[xy_l, xy_r], "CLE_BC", &[], &[]);
            let tile = &rd.tiles[&xy];
            let tk = &rd.tile_kinds[tile.kind];
            let naming = builder.db.get_node_naming("CLE_BC");
            let int_naming = builder.db.get_node_naming("INT");
            for (int_xy, t, dir, tname) in [
                (xy_l, NodeTileId::from_idx(0), Dir::E, "CLE.E"),
                (xy_r, NodeTileId::from_idx(1), Dir::W, "CLE.W"),
            ] {
                let naming = &builder.db.node_namings[naming];
                let mut nodes = HashMap::new();
                for &w in &bnode_outs {
                    if let Some(n) = naming.wires.get(&(t, w)) {
                        let n = rd.wires.get(n).unwrap();
                        if let &TkWire::Connected(idx) = tk.wires.get(&n).unwrap().1 {
                            nodes.insert(tile.conn_wires[idx], w);
                        }
                    }
                }
                let mut wires = EntityPartVec::new();
                let int_tile = &rd.tiles[&int_xy];
                let int_tk = &rd.tile_kinds[int_tile.kind];
                let int_naming = &builder.db.node_namings[int_naming];
                for &w in &bounces {
                    if let Some(n) = int_naming.wires.get(&(NodeTileId::from_idx(0), w)) {
                        let n = rd.wires.get(n).unwrap();
                        if let &TkWire::Connected(idx) = int_tk.wires.get(&n).unwrap().1 {
                            nodes.insert(int_tile.conn_wires[idx], w);
                        }
                    }
                }
                for &w in &bnodes {
                    if let Some(n) = int_naming.wires.get(&(NodeTileId::from_idx(0), w)) {
                        let n = rd.wires.get(n).unwrap();
                        if let &TkWire::Connected(idx) = int_tk.wires.get(&n).unwrap().1 {
                            if let Some(&cw) = nodes.get(&int_tile.conn_wires[idx]) {
                                wires.insert(w, TermInfo::PassNear(cw));
                            }
                        }
                    }
                }
                builder.insert_term_merge(tname, TermKind { dir, wires });
            }
        }
    }

    let t = builder.db.terms.get("CLE.W").unwrap().1.clone();
    builder.db.terms.insert("CLE.BLI.W".to_string(), t);
    let t = builder.db.terms.get("CLE.E").unwrap().1.clone();
    builder.db.terms.insert("CLE.BLI.E".to_string(), t);
    builder.insert_term_merge(
        "CLE.W",
        TermKind {
            dir: Dir::W,
            wires: logic_outs_w,
        },
    );
    builder.insert_term_merge(
        "CLE.E",
        TermKind {
            dir: Dir::E,
            wires: logic_outs_e,
        },
    );

    for (dir, tkn, name, _) in intf_kinds {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = builder.walk_to_int(xy, !dir).unwrap();
            builder
                .xnode(name, name, xy)
                .ref_int(int_xy, 0)
                .extract_muxes()
                .extract_intfs(true)
                .iris(&[
                    ("IRI_QUAD", 0, 0),
                    ("IRI_QUAD", 0, 1),
                    ("IRI_QUAD", 0, 2),
                    ("IRI_QUAD", 0, 3),
                ])
                .extract();
        }
    }
    let cle_bc = builder.db.get_node_naming("CLE_BC");
    for (dir, tkn, name, is_top) in bli_cle_intf_kinds {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = builder.walk_to_int(xy, !dir).unwrap();
            let cle_xy = xy.delta(if dir == Dir::E { 1 } else { -1 }, 0);
            for i in 0..4 {
                let iriy = if is_top {
                    4 * i as u8
                } else {
                    4 * (3 - i) as u8
                };
                builder
                    .xnode(format!("{name}.{i}"), format!("{name}.{i}"), xy)
                    .ref_int(int_xy.delta(0, i), 0)
                    .ref_xlat(
                        cle_xy.delta(0, i),
                        if dir == Dir::E {
                            &[Some(0), None]
                        } else {
                            &[None, Some(0)]
                        },
                        cle_bc,
                    )
                    .extract_intfs(true)
                    .iris(&[
                        ("IRI_QUAD", 0, iriy),
                        ("IRI_QUAD", 0, iriy + 1),
                        ("IRI_QUAD", 0, iriy + 2),
                        ("IRI_QUAD", 0, iriy + 3),
                    ])
                    .extract();
            }
        }
    }

    term_wires[Dir::N].insert(
        builder.db.get_wire("LONG.6.E.0.3.S"),
        TermInfo::PassNear(builder.db.get_wire("LONG.10.E.7.5")),
    );
    term_wires[Dir::N].insert(
        builder.db.get_wire("OUT.E.1.S"),
        TermInfo::PassNear(builder.db.get_wire("LONG.10.E.7.5")),
    );
    for (dir, wires) in term_wires {
        builder.insert_term_merge(&format!("TERM.{dir}"), TermKind { dir, wires });
    }
    builder.extract_term_conn("TERM.W", Dir::W, "INTF_GT_BL_TILE", &[]);
    builder.extract_term_conn("TERM.W", Dir::W, "INTF_GT_TL_TILE", &[]);
    builder.extract_term_conn("TERM.W", Dir::W, "INTF_PSS_BL_TILE", &[]);
    builder.extract_term_conn("TERM.W", Dir::W, "INTF_PSS_TL_TILE", &[]);
    builder.extract_term_conn("TERM.E", Dir::E, "INTF_GT_BR_TILE", &[]);
    builder.extract_term_conn("TERM.E", Dir::E, "INTF_GT_TR_TILE", &[]);
    builder.extract_term_conn("TERM.S", Dir::S, "TERM_B_INT_TILE", &[]);
    builder.extract_term_conn("TERM.N", Dir::N, "TERM_T_INT_TILE", &[]);

    for tkn in ["RCLK_INT_L_FT", "RCLK_INT_R_FT"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = Coord {
                x: xy.x,
                y: xy.y + 1,
            };
            let mut int_xy_b = Coord {
                x: xy.x,
                y: xy.y - 1,
            };
            if rd.tile_kinds.key(rd.tiles[&int_xy_b].kind) != "INT" {
                int_xy_b.y -= 1;
                if rd.tile_kinds.key(rd.tiles[&int_xy_b].kind) != "INT" {
                    continue;
                }
            }
            builder.extract_xnode("RCLK", xy, &[], &[int_xy], "RCLK", &[], &[]);
        }
    }
    let rclk_int = builder.db.get_node_naming("RCLK");

    for (tkn, kind, key0, key1) in [
        ("CLE_W_CORE", "CLE_R", "SLICE_L0", "SLICE_L1"),
        ("CLE_E_CORE", "CLE_L", "SLICE_R0", "SLICE_R1"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let int_xy = if kind == "CLE_R" {
                builder.walk_to_int(xy, Dir::W).unwrap()
            } else {
                builder.walk_to_int(xy, Dir::E).unwrap()
            };
            builder.extract_xnode_bels(
                kind,
                xy,
                &[],
                &[int_xy],
                kind,
                &[
                    builder
                        .bel_xy(key0, "SLICE", 0, 0)
                        .pin_name_only("LAG_W1", 1)
                        .pin_name_only("LAG_W2", 1)
                        .pin_name_only("LAG_E1", 1)
                        .pin_name_only("LAG_E2", 1)
                        .pin_name_only("LAG_S", 1)
                        .pin_name_only("LAG_N", 1)
                        .pin_name_only("CIN", 1)
                        .pin_name_only("COUT", 1),
                    builder
                        .bel_xy(key1, "SLICE", 1, 0)
                        .pin_name_only("LAG_W1", 1)
                        .pin_name_only("LAG_W2", 1)
                        .pin_name_only("LAG_E1", 1)
                        .pin_name_only("LAG_E2", 1)
                        .pin_name_only("LAG_S", 1)
                        .pin_name_only("LAG_N", 1)
                        .pin_name_only("SRL_IN_B", 1)
                        .pin_name_only("SRL_OUT_B", 1)
                        .pin_name_only("CIN", 1)
                        .pin_name_only("COUT", 1),
                ],
            );
        }
    }

    for (dir, tkn) in [
        (Dir::E, "BRAM_LOCF_BR_TILE"),
        (Dir::E, "BRAM_LOCF_TR_TILE"),
        (Dir::E, "BRAM_ROCF_BR_TILE"),
        (Dir::E, "BRAM_ROCF_TR_TILE"),
        (Dir::W, "BRAM_ROCF_BL_TILE"),
        (Dir::W, "BRAM_ROCF_TL_TILE"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let (kind, key_f, key_h0, key_h1, intf) = match dir {
                Dir::E => ("BRAM_R", "BRAM_R_F", "BRAM_R_H0", "BRAM_R_H1", "INTF.E"),
                Dir::W => ("BRAM_L", "BRAM_L_F", "BRAM_L_H0", "BRAM_L_H1", "INTF.W"),
                _ => unreachable!(),
            };
            let intf = builder.db.get_node_naming(intf);
            let intf_xy = if dir == Dir::E {
                xy.delta(-1, 0)
            } else {
                xy.delta(1, 0)
            };
            let mut bel_f = builder
                .bel_xy(key_f, "RAMB36", 0, 0)
                .pin_name_only("CASINSBITERR", 1)
                .pin_name_only("CASINDBITERR", 1)
                .pin_name_only("CASOUTSBITERR", 1)
                .pin_name_only("CASOUTDBITERR", 1);
            for ab in ['A', 'B'] {
                for i in 0..32 {
                    bel_f = bel_f
                        .pin_name_only(&format!("CASDIN{ab}_{i}_"), 1)
                        .pin_name_only(&format!("CASDOUT{ab}_{i}_"), 1);
                }
                for i in 0..4 {
                    bel_f = bel_f
                        .pin_name_only(&format!("CASDINP{ab}_{i}_"), 1)
                        .pin_name_only(&format!("CASDOUTP{ab}_{i}_"), 1);
                }
            }
            let mut bel_h0 = builder.bel_xy(key_h0, "RAMB18", 0, 0);
            let mut bel_h1 = builder.bel_xy(key_h1, "RAMB18", 0, 1);
            for ab in ['A', 'B'] {
                for i in 0..16 {
                    bel_h0 = bel_h0
                        .pin_name_only(&format!("CASDIN{ab}_{i}_"), 0)
                        .pin_name_only(&format!("CASDOUT{ab}_{i}_"), 0);
                    bel_h1 = bel_h1
                        .pin_name_only(&format!("CASDIN{ab}_{i}_"), 0)
                        .pin_name_only(&format!("CASDOUT{ab}_{i}_"), 0);
                }
                for i in 0..2 {
                    bel_h0 = bel_h0
                        .pin_name_only(&format!("CASDINP{ab}_{i}_"), 0)
                        .pin_name_only(&format!("CASDOUTP{ab}_{i}_"), 0);
                    bel_h1 = bel_h1
                        .pin_name_only(&format!("CASDINP{ab}_{i}_"), 0)
                        .pin_name_only(&format!("CASDOUTP{ab}_{i}_"), 0);
                }
            }
            let bels = [bel_f, bel_h0, bel_h1];
            let mut xn = builder.xnode(kind, kind, xy).num_tiles(4);
            for i in 0..4 {
                xn = xn.ref_single(intf_xy.delta(0, i as i32), i, intf)
            }
            xn.bels(bels).extract();
        }
    }

    for (tkn, kind) in [
        ("URAM_LOCF_BL_TILE", "URAM"),
        ("URAM_LOCF_TL_TILE", "URAM"),
        ("URAM_ROCF_BL_TILE", "URAM"),
        ("URAM_ROCF_TL_TILE", "URAM"),
        ("URAM_DELAY_LOCF_TL_TILE", "URAM_DELAY"),
        ("URAM_DELAY_ROCF_TL_TILE", "URAM_DELAY"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let intf = builder.db.get_node_naming("INTF.W");
            let intf_xy = xy.delta(1, 0);
            let mut bels = vec![builder.bel_xy("URAM", "URAM288", 0, 0)];
            if kind == "URAM_DELAY" {
                bels.push(builder.bel_xy("URAM_CAS_DLY", "URAM_CAS_DLY", 0, 0));
            }
            let bels: Vec<_> = bels
                .into_iter()
                .map(|mut bel| {
                    for ab in ['A', 'B'] {
                        bel = bel
                            .pin_name_only(&format!("CAS_IN_EN_{ab}"), 1)
                            .pin_name_only(&format!("CAS_OUT_EN_{ab}"), 1)
                            .pin_name_only(&format!("CAS_IN_SBITERR_{ab}"), 1)
                            .pin_name_only(&format!("CAS_OUT_SBITERR_{ab}"), 1)
                            .pin_name_only(&format!("CAS_IN_DBITERR_{ab}"), 1)
                            .pin_name_only(&format!("CAS_OUT_DBITERR_{ab}"), 1)
                            .pin_name_only(&format!("CAS_IN_RDACCESS_{ab}"), 1)
                            .pin_name_only(&format!("CAS_OUT_RDACCESS_{ab}"), 1)
                            .pin_name_only(&format!("CAS_IN_RDB_WR_{ab}"), 1)
                            .pin_name_only(&format!("CAS_OUT_RDB_WR_{ab}"), 1);
                        for i in 0..72 {
                            bel = bel
                                .pin_name_only(&format!("CAS_IN_DIN_{ab}_{i}_"), 1)
                                .pin_name_only(&format!("CAS_IN_DOUT_{ab}_{i}_"), 1)
                                .pin_name_only(&format!("CAS_OUT_DIN_{ab}_{i}_"), 1)
                                .pin_name_only(&format!("CAS_OUT_DOUT_{ab}_{i}_"), 1);
                        }
                        for i in 0..26 {
                            bel = bel
                                .pin_name_only(&format!("CAS_IN_ADDR_{ab}_{i}_"), 1)
                                .pin_name_only(&format!("CAS_OUT_ADDR_{ab}_{i}_"), 1);
                        }
                        for i in 0..9 {
                            bel = bel
                                .pin_name_only(&format!("CAS_IN_BWE_{ab}_{i}_"), 1)
                                .pin_name_only(&format!("CAS_OUT_BWE_{ab}_{i}_"), 1);
                        }
                    }
                    bel
                })
                .collect();
            let mut xn = builder.xnode(kind, kind, xy).num_tiles(4);
            for i in 0..4 {
                xn = xn.ref_single(intf_xy.delta(0, i as i32), i, intf)
            }
            xn.bels(bels).extract();
        }
    }

    for tkn in ["DSP_ROCF_B_TILE", "DSP_ROCF_T_TILE"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bels = vec![];
            for i in 0..2 {
                let mut bel = builder
                    .bel_xy(format!("DSP{i}"), "DSP", i, 0)
                    .pin_name_only("MULTSIGNIN", 1)
                    .pin_name_only("MULTSIGNOUT", 1)
                    .pin_name_only("CARRYCASCIN", 1)
                    .pin_name_only("CARRYCASCOUT", 1)
                    .pin_name_only("CONJ_CPLX_OUT", 1)
                    .pin_name_only("CONJ_CPLX_MULT_IN", 0)
                    .pin_name_only("CONJ_CPLX_PREADD_IN", 0);
                for i in 0..34 {
                    bel = bel
                        .pin_name_only(&format!("ACIN_{i}_"), 1)
                        .pin_name_only(&format!("ACOUT_{i}_"), 1);
                }
                for i in 0..32 {
                    bel = bel
                        .pin_name_only(&format!("BCIN_{i}_"), 1)
                        .pin_name_only(&format!("BCOUT_{i}_"), 1);
                }
                for i in 0..58 {
                    bel = bel
                        .pin_name_only(&format!("PCIN_{i}_"), 1)
                        .pin_name_only(&format!("PCOUT_{i}_"), 1);
                }
                for i in 0..10 {
                    bel = bel
                        .pin_name_only(&format!("AD_CPLX_{i}_"), 0)
                        .pin_name_only(&format!("AD_DATA_CPLX_{i}_"), 1);
                }
                for i in 0..18 {
                    bel = bel
                        .pin_name_only(&format!("A_TO_D_CPLX_{i}_"), 1)
                        .pin_name_only(&format!("D_FROM_A_CPLX_{i}_"), 1)
                        .pin_name_only(&format!("A_CPLX_{i}_"), 1)
                        .pin_name_only(&format!("B2B1_CPLX_{i}_"), 1);
                }
                for i in 0..37 {
                    bel = bel
                        .pin_name_only(&format!("U_CPLX_{i}_"), 0)
                        .pin_name_only(&format!("V_CPLX_{i}_"), 0);
                }
                bels.push(bel);
            }
            let mut bel = builder
                .bel_xy("DSP_CPLX", "DSP58_CPLX", 0, 0)
                .pin_name_only("CONJ_DSP_L_IN", 0)
                .pin_name_only("CONJ_DSP_R_IN", 0)
                .pin_name_only("CONJ_DSP_L_MULT_OUT", 1)
                .pin_name_only("CONJ_DSP_R_MULT_OUT", 1)
                .pin_name_only("CONJ_DSP_L_PREADD_OUT", 1)
                .pin_name_only("CONJ_DSP_R_PREADD_OUT", 1);
            for i in 0..10 {
                bel = bel
                    .pin_name_only(&format!("AD_CPLX_DSPL_{i}_"), 1)
                    .pin_name_only(&format!("AD_CPLX_DSPR_{i}_"), 1)
                    .pin_name_only(&format!("AD_DATA_CPLX_DSPL_{i}_"), 0)
                    .pin_name_only(&format!("AD_DATA_CPLX_DSPR_{i}_"), 0);
            }
            for i in 0..18 {
                bel = bel
                    .pin_name_only(&format!("A_CPLX_L_{i}_"), 0)
                    .pin_name_only(&format!("B2B1_CPLX_L_{i}_"), 0)
                    .pin_name_only(&format!("B2B1_CPLX_R_{i}_"), 0);
            }
            for i in 0..37 {
                bel = bel
                    .pin_name_only(&format!("U_CPLX_{i}_"), 1)
                    .pin_name_only(&format!("V_CPLX_{i}_"), 1);
            }
            bels.push(bel);
            let intf_l = builder.db.get_node_naming("INTF.E");
            let intf_r = builder.db.get_node_naming("INTF.W");
            builder
                .xnode("DSP", "DSP", xy)
                .num_tiles(4)
                .ref_single(xy.delta(-1, 0), 0, intf_l)
                .ref_single(xy.delta(-1, 1), 1, intf_l)
                .ref_single(xy.delta(2, 0), 2, intf_r)
                .ref_single(xy.delta(2, 1), 3, intf_r)
                .bels(bels)
                .extract();
        }
    }

    for (kind, tkn, sk, is_large) in [
        ("PCIE4", "PCIEB_BOT_TILE", "PCIE40", false),
        ("PCIE4", "PCIEB_TOP_TILE", "PCIE40", false),
        ("PCIE5", "PCIEB5_BOT_TILE", "PCIE50", false),
        ("PCIE5", "PCIEB5_TOP_TILE", "PCIE50", false),
        ("MRMAC", "MRMAC_BOT_TILE", "MRMAC", false),
        ("MRMAC", "MRMAC_TOP_TILE", "MRMAC", false),
        ("DCMAC", "DCMAC_TILE", "DCMAC", true),
        ("ILKN", "ILKN_TILE", "ILKNF", true),
        ("HSC", "HSC_TILE", "HSC", true),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bel = builder.bel_xy(kind, sk, 0, 0);
            let intf_l = builder.db.get_node_naming("INTF.E.HB");
            let intf_r = builder.db.get_node_naming("INTF.W.HB");
            let height = if is_large { 96 } else { 48 };
            let mut xn = builder.xnode(kind, kind, xy).num_tiles(height * 2);
            for i in 0..height {
                xn = xn
                    .ref_single(xy.delta(-1, (i + i / 4) as i32), i, intf_l)
                    .ref_single(xy.delta(1, (i + i / 4) as i32), i + height, intf_r)
            }
            xn.bel(bel).extract();
        }
    }

    for (tkn, naming_f, naming_h, swz) in [
        (
            "RCLK_CLE_CORE",
            "RCLK_CLE",
            "RCLK_CLE.HALF",
            BUFDIV_LEAF_SWZ_A,
        ),
        (
            "RCLK_CLE_LAG_CORE",
            "RCLK_CLE.LAG",
            "RCLK_CLE.HALF.LAG",
            BUFDIV_LEAF_SWZ_B,
        ),
    ] {
        let mut done_full = false;
        let mut done_half = false;
        for &xy in rd.tiles_by_kind_name(tkn) {
            let td = &rd.tiles[&xy.delta(0, -1)];
            let is_full = rd.tile_kinds.key(td.kind) == "CLE_W_CORE";
            if is_full {
                if done_full {
                    continue;
                }
                done_full = true;
            } else {
                if done_half {
                    continue;
                }
                done_half = true;
            }
            let mut bels = vec![];
            for (i, &y) in swz.iter().enumerate() {
                let mut bel = builder
                    .bel_xy(&format!("BUFDIV_LEAF.CLE.{i}"), "BUFDIV_LEAF", 0, y as u8)
                    .pin_name_only("I", 1)
                    .pin_name_only("O_CASC", 1);
                if i != 0 {
                    bel = bel.pin_name_only("I_CASC", 0);
                }
                if !is_full && i < 16 {
                    bel = bel.pin_name_only("O", 1);
                }
                bels.push(bel);
            }
            let mut bel = builder.bel_virtual("RCLK_HDISTR_LOC.CLE");
            for i in 0..24 {
                bel = bel.extra_wire(
                    format!("HDISTR_LOC{i}"),
                    &[format!("IF_HCLK_CLK_HDISTR_LOC{i}")],
                );
            }
            bels.push(bel);
            bels.push(
                builder
                    .bel_virtual("VCC.RCLK_CLE")
                    .extra_wire("VCC", &["VCC_WIRE"]),
            );
            let kind = if is_full { "RCLK_CLE" } else { "RCLK_CLE.HALF" };
            let naming = if is_full { naming_f } else { naming_h };
            let int_r_xy = builder
                .walk_to_int(xy.delta(0, 1), Dir::E)
                .unwrap()
                .delta(0, -1);
            let mut xn = builder
                .xnode(kind, naming, xy)
                .num_tiles(if is_full { 2 } else { 1 })
                .ref_single(int_r_xy, 0, rclk_int)
                .ref_xlat(xy.delta(1, 1), &[None, Some(0)], cle_bc);
            if is_full {
                xn = xn.ref_xlat(xy.delta(1, -1), &[None, Some(1)], cle_bc);
            }
            xn.bels(bels).extract();
        }
    }

    for (dir, naming, tkn, intf_dx, swz, has_dfx) in [
        (Dir::E, "DSP", "RCLK_DSP_CORE", 0, BUFDIV_LEAF_SWZ_A, false),
        (Dir::W, "DSP", "RCLK_DSP_CORE", 3, BUFDIV_LEAF_SWZ_AH, true),
        (Dir::E, "HB", "RCLK_HB_CORE", 0, BUFDIV_LEAF_SWZ_A, false),
        (Dir::W, "HB", "RCLK_HB_CORE", 2, BUFDIV_LEAF_SWZ_AH, false),
        (
            Dir::E,
            "HDIO",
            "RCLK_HDIO_CORE",
            0,
            BUFDIV_LEAF_SWZ_A,
            false,
        ),
        (
            Dir::W,
            "HDIO",
            "RCLK_HDIO_CORE",
            2,
            BUFDIV_LEAF_SWZ_AH,
            false,
        ),
        (
            Dir::E,
            "HB_HDIO",
            "RCLK_HB_HDIO_CORE",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::W,
            "HB_HDIO",
            "RCLK_HB_HDIO_CORE",
            2,
            BUFDIV_LEAF_SWZ_BH,
            false,
        ),
        (
            Dir::W,
            "VNOC",
            "RCLK_INTF_L_CORE",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::E,
            "VNOC",
            "RCLK_INTF_R_CORE",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::W,
            "CFRM",
            "RCLK_INTF_OPT_CORE",
            0,
            BUFDIV_LEAF_SWZ_A,
            false,
        ),
        (
            Dir::W,
            "GT",
            "RCLK_INTF_TERM_LEFT_CORE",
            1,
            BUFDIV_LEAF_SWZ_A,
            false,
        ),
        (
            Dir::E,
            "GT",
            "RCLK_INTF_TERM_RIGHT_CORE",
            0,
            BUFDIV_LEAF_SWZ_A,
            false,
        ),
        (
            Dir::E,
            "GT.ALT",
            "RCLK_INTF_TERM2_RIGHT_CORE",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::W,
            "BRAM",
            "RCLK_BRAM_CORE_MY",
            1,
            BUFDIV_LEAF_SWZ_A,
            true,
        ),
        (
            Dir::W,
            "URAM",
            "RCLK_URAM_CORE_MY",
            1,
            BUFDIV_LEAF_SWZ_A,
            true,
        ),
        (Dir::E, "BRAM", "RCLK_BRAM_CORE", 0, BUFDIV_LEAF_SWZ_A, true),
        (
            Dir::E,
            "BRAM.CLKBUF",
            "RCLK_BRAM_CLKBUF_CORE",
            0,
            BUFDIV_LEAF_SWZ_A,
            true,
        ),
        (
            Dir::W,
            "HB_FULL",
            "RCLK_HB_FULL_R_CORE",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::E,
            "HB_FULL",
            "RCLK_HB_FULL_L_CORE",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
    ] {
        let mut done_full = false;
        let mut done_half = false;
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = builder
                .walk_to_int(xy.delta(0, 1), !dir)
                .unwrap()
                .delta(0, -1);
            if int_xy.x.abs_diff(xy.x) > 5 {
                continue;
            }
            if rd.tile_kinds.key(rd.tiles[&int_xy.delta(0, 1)].kind) != "INT" {
                continue;
            }
            let td = &rd.tiles[&int_xy.delta(0, -1)];
            let is_full = rd.tile_kinds.key(td.kind) == "INT";
            if is_full {
                if done_full {
                    continue;
                }
                done_full = true;
            } else {
                if done_half {
                    continue;
                }
                done_half = true;
            }
            let mut bels = vec![];
            for (i, &y) in swz.iter().enumerate() {
                let mut bel = builder
                    .bel_xy(&format!("BUFDIV_LEAF.{dir}.{i}"), "BUFDIV_LEAF", 0, y as u8)
                    .pin_name_only("I", 1)
                    .pin_name_only("O_CASC", 1);
                if i != 0 {
                    bel = bel.pin_name_only("I_CASC", 0);
                }
                if !is_full && i < 16 {
                    bel = bel.pin_name_only("O", 1);
                }
                bels.push(bel);
            }
            let mut bel = builder.bel_virtual(format!("RCLK_HDISTR_LOC.{dir}"));
            for i in 0..24 {
                bel = bel.extra_wire(
                    format!("HDISTR_LOC{i}"),
                    &[
                        format!("IF_HCLK_CLK_HDISTR_LOC{i}"),
                        format!("IF_HCLK_L_CLK_HDISTR_LOC{i}"),
                    ],
                );
            }
            bels.push(bel);
            bels.push(
                builder
                    .bel_virtual(format!("VCC.RCLK_INTF.{dir}"))
                    .extra_wire("VCC", &["VCC_WIRE"]),
            );
            let intf = builder
                .db
                .get_node_naming(if dir == Dir::E { "INTF.E" } else { "INTF.W" });
            let half = if is_full { "" } else { ".HALF" };
            let mut xn = builder
                .xnode(
                    &format!("RCLK_INTF.{dir}{half}"),
                    &format!("RCLK_INTF.{dir}{half}.{naming}"),
                    xy,
                )
                .num_tiles(if is_full { 2 } else { 1 })
                .ref_single(int_xy, 0, rclk_int)
                .ref_single(xy.delta(intf_dx, 1), 0, intf);
            if is_full {
                xn = xn.ref_single(xy.delta(intf_dx, -1), 1, intf);
            }
            xn.bels(bels).extract();
            if has_dfx {
                let bel = builder.bel_xy(format!("RCLK_DFX_TEST.{dir}"), "RCLK", 0, 0);
                builder
                    .xnode(
                        &format!("RCLK_DFX.{dir}"),
                        &format!("RCLK_DFX.{dir}.{naming}"),
                        xy,
                    )
                    .ref_single(int_xy, 0, rclk_int)
                    .bel(bel)
                    .extract();
            }
        }
    }

    // XXX RCLK_HDIO
    // XXX RCLK_CLKBUF

    builder.build()
}
