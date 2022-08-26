use enum_map::EnumMap;
use prjcombine_entity::{EntityId, EntityPartVec};
use prjcombine_rawdump::{Coord, Part, TkWire};
use prjcombine_xilinx_geom::int::{Dir, IntDb, NodeTileId, TermInfo, TermKind, WireKind};
use std::collections::HashMap;

use crate::intb::IntBuilder;

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
        (Dir::W, "INTF_HDIO_LOCF_BL_TILE", "INTF.W.HB", false),
        (Dir::W, "INTF_HDIO_LOCF_TL_TILE", "INTF.W.HB", false),
        (Dir::E, "INTF_HDIO_LOCF_BR_TILE", "INTF.E.HB", false),
        (Dir::E, "INTF_HDIO_LOCF_TR_TILE", "INTF.E.HB", false),
        (Dir::W, "INTF_HDIO_ROCF_BL_TILE", "INTF.W.HB", false),
        (Dir::W, "INTF_HDIO_ROCF_TL_TILE", "INTF.W.HB", false),
        (Dir::E, "INTF_HDIO_ROCF_BR_TILE", "INTF.E.HB", false),
        (Dir::E, "INTF_HDIO_ROCF_TR_TILE", "INTF.E.HB", false),
        (Dir::W, "INTF_CFRM_BL_TILE", "INTF.W", false),
        (Dir::W, "INTF_CFRM_TL_TILE", "INTF.W", false),
        (Dir::W, "INTF_PSS_BL_TILE", "INTF.W.TERM", true),
        (Dir::W, "INTF_PSS_TL_TILE", "INTF.W.TERM", true),
        (Dir::W, "INTF_GT_BL_TILE", "INTF.W.TERM", true),
        (Dir::W, "INTF_GT_TL_TILE", "INTF.W.TERM", true),
        (Dir::E, "INTF_GT_BR_TILE", "INTF.E.TERM", true),
        (Dir::E, "INTF_GT_TR_TILE", "INTF.E.TERM", true),
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
        (Dir::E, "LONG", 3, 6),
        (Dir::N, "LONG", 7, 7),
        (Dir::E, "LONG", 5, 10),
        (Dir::N, "LONG", 12, 12),
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
            builder.extra_name_tile_sub("CLE_BC_CORE", format!("LOGIC_OUTS_{we}{i}"), sub, cw);
            builder.extra_name_tile_sub("SLL", format!("LOGIC_OUTS_{we}{i}"), sub, cw);
            if ew == Dir::E {
                logic_outs_e.insert(cw, TermInfo::PassNear(w));
            } else {
                logic_outs_w.insert(cw, TermInfo::PassNear(w));
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

    for (sub, ew) in [Dir::W, Dir::E].into_iter().enumerate() {
        let lr = match ew {
            Dir::E => 'L',
            Dir::W => 'R',
            _ => unreachable!(),
        };
        for i in 0..13 {
            let w = builder.mux_out(format!("CLE.IMUX.{ew}.CTRL.{i}"), &[""]);
            builder.extra_name_sub(format!("CTRL_{lr}{i}"), sub, w);
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
        for i in 0..20 {
            for j in 0..2 {
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
            builder.extract_xnode(name, xy, &[], &[int_xy], name, &[], &[]);
        }
    }

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

    builder.build()
}
