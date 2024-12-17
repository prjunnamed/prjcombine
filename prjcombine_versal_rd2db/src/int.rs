use enum_map::EnumMap;
use prjcombine_int::db::{Dir, IntDb, NodeTileId, TermInfo, TermKind, WireKind};
use prjcombine_rawdump::{Part, TkWire};
use prjcombine_versal_naming::DeviceNaming;
use prjcombine_versal_naming::{
    BUFDIV_LEAF_SWZ_A, BUFDIV_LEAF_SWZ_AH, BUFDIV_LEAF_SWZ_B, BUFDIV_LEAF_SWZ_BH,
};
use prjcombine_xilinx_naming::db::NamingDb;
use std::collections::HashMap;
use unnamed_entity::{EntityId, EntityPartVec};

use prjcombine_rdintb::IntBuilder;

pub fn make_int_db(rd: &Part, dev_naming: &DeviceNaming) -> (IntDb, NamingDb) {
    let mut builder = IntBuilder::new(rd);
    let crd = rd.tiles_by_kind_name("INT").first().unwrap();
    let tile = &rd.tiles[crd];
    if tile.name.contains("_S") {
        builder.set_mirror_square();
    }
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
                    builder.extra_name_tile_sub("CLE_BC_CORE_MX", "BNODE_TAP0", 1, w);
                    builder.extra_name_tile_sub("SLL", "BNODE_TAP0", 1, w);
                    builder.extra_name_tile_sub("SLL2", "BNODE_TAP0", 1, w);
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

    for i in 0..6 {
        let w = builder.mux_out(format!("IMUX.LAG{i}"), &[""]);
        builder.extra_name_tile_sub("SLL", format!("LAG_CASCOUT_TXI{i}"), 1, w);
        builder.extra_name_tile_sub("SLL2", format!("LAG_CASCOUT_TXI{i}"), 1, w);
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
            builder.extra_name_tile_sub("CLE_BC_CORE_MX", format!("LOGIC_OUTS_{we}{i}"), sub, cw);
            builder.extra_name_tile_sub("SLL", format!("LOGIC_OUTS_{we}{i}"), sub, cw);
            builder.extra_name_tile_sub("SLL2", format!("LOGIC_OUTS_{we}{i}"), sub, cw);
            if ew == Dir::E {
                logic_outs_e.insert(cw, TermInfo::PassNear(w));
            } else {
                logic_outs_w.insert(cw, TermInfo::PassNear(w));
            }
        }
    }

    for i in 0..6 {
        let w = builder.logic_out(format!("OUT.LAG{i}"), &[""]);
        builder.extra_name_tile_sub("CLE_BC_CORE", format!("VCC_WIRE{i}"), 1, w);
        builder.extra_name_tile_sub("CLE_BC_CORE_MX", format!("VCC_WIRE{i}"), 1, w);
        builder.extra_name_tile_sub("SLL", format!("LAG_OUT{i}"), 1, w);
        builder.extra_name_tile_sub("SLL2", format!("LAG_OUT{i}"), 1, w);
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
            builder.extra_name_tile(
                match ew {
                    Dir::E => "CLE_W_VR_CORE",
                    Dir::W => "CLE_E_VR_CORE",
                    _ => unreachable!(),
                },
                match i {
                    0 => "CLE_SLICEL_VR_TOP_0_CLK",
                    1 => "CLE_SLICEM_VR_TOP_1_CLK",
                    2 => "CLE_SLICEL_VR_TOP_0_RST",
                    3 => "CLE_SLICEM_VR_TOP_1_RST",
                    4 => "CLE_SLICEL_VR_TOP_0_CKEN1",
                    5 => "CLE_SLICEL_VR_TOP_0_CKEN2",
                    6 => "CLE_SLICEL_VR_TOP_0_CKEN3",
                    7 => "CLE_SLICEL_VR_TOP_0_CKEN4",
                    8 => "CLE_SLICEM_VR_TOP_1_WE",
                    9 => "CLE_SLICEM_VR_TOP_1_CKEN1",
                    10 => "CLE_SLICEM_VR_TOP_1_CKEN2",
                    11 => "CLE_SLICEM_VR_TOP_1_CKEN3",
                    12 => "CLE_SLICEM_VR_TOP_1_CKEN4",
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

    for (kind, tkn) in [
        ("CLE_BC", "CLE_BC_CORE"),
        ("CLE_BC", "CLE_BC_CORE_MX"),
        ("CLE_BC.SLL", "SLL"),
        ("CLE_BC.SLL2", "SLL2"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let td = &rd.tiles[&builder.delta(xy, 0, -1)];
            if rd.tile_kinds.key(td.kind) != tkn {
                continue;
            }
            let tu = &rd.tiles[&builder.delta(xy, 0, 1)];
            if rd.tile_kinds.key(tu.kind) != tkn {
                continue;
            }
            let xy_l = builder.walk_to_int(xy, Dir::W, false).unwrap();
            let xy_r = builder.walk_to_int(xy, Dir::E, false).unwrap();
            let mut bels = vec![];
            if kind != "CLE_BC" {
                let mut bel = builder.bel_virtual("LAGUNA");
                for i in 0..6 {
                    bel = bel
                        .extra_int_in(format!("IN{i}"), &[format!("LAG_CASCOUT_TXI{i}")])
                        .extra_int_out(format!("OUT{i}"), &[format!("LAG_OUT{i}")])
                        .extra_wire(format!("UBUMP{i}"), &[format!("UBUMP{i}")]);
                }
                bels.push(bel);
            }
            builder.extract_xnode(kind, xy, &[], &[xy_l, xy_r], kind, &bels, &[]);
            let tile = &rd.tiles[&xy];
            let tk = &rd.tile_kinds[tile.kind];
            let naming = builder.ndb.get_node_naming("CLE_BC");
            let int_naming = builder.ndb.get_node_naming("INT");
            for (int_xy, t, dir, tname) in [
                (xy_l, NodeTileId::from_idx(0), Dir::E, "CLE.E"),
                (xy_r, NodeTileId::from_idx(1), Dir::W, "CLE.W"),
            ] {
                let naming = &builder.ndb.node_namings[naming];
                let mut nodes = HashMap::new();
                for &w in &bnode_outs {
                    if let Some(n) = naming.wires.get(&(t, w)) {
                        let n = rd.wires.get(n).unwrap();
                        if let &TkWire::Connected(idx) = tk.wires.get(&n).unwrap().1 {
                            nodes.insert(tile.conn_wires[idx], w);
                        }
                    }
                }
                let int_tile = &rd.tiles[&int_xy];
                let int_tk = &rd.tile_kinds[int_tile.kind];
                let int_naming = &builder.ndb.node_namings[int_naming];
                for &w in &bounces {
                    if let Some(n) = int_naming.wires.get(&(NodeTileId::from_idx(0), w)) {
                        let n = rd.wires.get(n).unwrap();
                        if let &TkWire::Connected(idx) = int_tk.wires.get(&n).unwrap().1 {
                            nodes.insert(int_tile.conn_wires[idx], w);
                        }
                    }
                }
                let mut wires = EntityPartVec::new();
                for &w in &bnodes {
                    if builder.db.wires[w] != WireKind::Branch(dir) {
                        continue;
                    }
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
            break;
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
            let td = &rd.tiles[&builder.delta(xy, 0, -1)];
            if rd.tile_kinds.key(td.kind) != tkn {
                continue;
            }
            let tu = &rd.tiles[&builder.delta(xy, 0, 1)];
            if rd.tile_kinds.key(tu.kind) != tkn {
                continue;
            }
            let int_xy = builder.walk_to_int(xy, !dir, false).unwrap();
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
            break;
        }
    }
    let cle_bc = builder.ndb.get_node_naming("CLE_BC");
    for (dir, tkn, name, is_top) in bli_cle_intf_kinds {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let int_xy = builder.walk_to_int(xy, !dir, false).unwrap();
            let cle_xy = builder.delta(xy, if dir == Dir::E { 1 } else { -1 }, 0);
            for i in 0..4 {
                let iriy = if is_top {
                    4 * i as u8
                } else {
                    4 * (3 - i) as u8
                };
                let cur_int_xy = builder.delta(int_xy, 0, i);
                let cur_cle_xy = builder.delta(cle_xy, 0, i);
                builder
                    .xnode(format!("{name}.{i}"), format!("{name}.{i}"), xy)
                    .ref_int(cur_int_xy, 0)
                    .ref_xlat(
                        cur_cle_xy,
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
        builder.db.wires.get("LONG.6.E.0.3.S").unwrap().0,
        TermInfo::PassNear(builder.db.wires.get("LONG.10.E.7.5").unwrap().0),
    );
    term_wires[Dir::N].insert(
        builder.db.wires.get("OUT.E.1.S").unwrap().0,
        TermInfo::PassNear(builder.db.wires.get("LONG.10.E.7.5").unwrap().0),
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

    for tkn in [
        "RCLK_INT_L_FT",
        "RCLK_INT_R_FT",
        "RCLK_INT_L_VR_FT",
        "RCLK_INT_R_VR_FT",
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = builder.delta(xy, 0, 1);
            let mut int_xy_b = builder.delta(xy, 0, -1);
            if rd.tile_kinds.key(rd.tiles[&int_xy_b].kind) != "INT" {
                int_xy_b = builder.delta(int_xy_b, 0, -1);
                if rd.tile_kinds.key(rd.tiles[&int_xy_b].kind) != "INT" {
                    continue;
                }
            }
            builder.extract_xnode("RCLK", xy, &[], &[int_xy], "RCLK", &[], &[]);
            break;
        }
    }
    let rclk_int = builder.ndb.get_node_naming("RCLK");

    for (tkn, kind, key0, key1) in [
        ("CLE_W_CORE", "CLE_R", "SLICE_L0", "SLICE_L1"),
        ("CLE_E_CORE", "CLE_L", "SLICE_R0", "SLICE_R1"),
        ("CLE_W_VR_CORE", "CLE_R.VR", "SLICE_L0", "SLICE_L1"),
        ("CLE_E_VR_CORE", "CLE_L.VR", "SLICE_R0", "SLICE_R1"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let int_xy = if kind == "CLE_R" || kind == "CLE_R.VR" {
                builder.walk_to_int(xy, Dir::W, false).unwrap()
            } else {
                builder.walk_to_int(xy, Dir::E, false).unwrap()
            };
            let bel_slicel = builder
                .bel_xy(key0, "SLICE", 0, 0)
                .pin_name_only("CIN", 1)
                .pin_name_only("COUT", 1);
            let bel_slicem = builder
                .bel_xy(key1, "SLICE", 1, 0)
                .pin_name_only("SRL_IN_B", 1)
                .pin_name_only("SRL_OUT_B", 1)
                .pin_name_only("CIN", 1)
                .pin_name_only("COUT", 1);
            let cle_bc_xy = builder.delta(xy, if kind.starts_with("CLE_R") { 1 } else { -1 }, 0);
            let cle_bc_kind = rd.tiles[&cle_bc_xy].kind;
            let cle_bc_naming = match &rd.tile_kinds.key(cle_bc_kind)[..] {
                "CLE_BC_CORE" | "CLE_BC_CORE_MX" => "CLE_BC",
                "SLL" => "CLE_BC.SLL",
                "SLL2" => "CLE_BC.SLL2",
                _ => unreachable!(),
            };
            let cle_bc_naming = builder.ndb.get_node_naming(cle_bc_naming);
            let mut xn = builder
                .xnode(kind, kind, xy)
                .num_tiles(if kind.starts_with("CLE_R") { 2 } else { 1 })
                .bel(bel_slicel)
                .bel(bel_slicem)
                .ref_int(int_xy, 0);
            if kind.starts_with("CLE_R") {
                xn = xn.ref_xlat(cle_bc_xy, &[None, Some(1)], cle_bc_naming);
            } else {
                xn = xn.ref_xlat(cle_bc_xy, &[None, Some(0)], cle_bc_naming);
            }
            xn.extract();
        }
    }

    for (dir, tkn) in [
        (Dir::E, "BRAM_LOCF_BR_TILE"),
        (Dir::E, "BRAM_LOCF_TR_TILE"),
        (Dir::W, "BRAM_LOCF_BL_TILE"),
        (Dir::W, "BRAM_LOCF_TL_TILE"),
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
            let intf = builder.ndb.get_node_naming(intf);
            let intf_xy = if dir == Dir::E {
                builder.delta(xy, -1, 0)
            } else {
                builder.delta(xy, 1, 0)
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
                let cur_intf_xy = xn.builder.delta(intf_xy, 0, i as i32);
                xn = xn.ref_single(cur_intf_xy, i, intf)
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
            let intf = builder.ndb.get_node_naming("INTF.W");
            let intf_xy = builder.delta(xy, 1, 0);
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
                let cur_intf_xy = xn.builder.delta(intf_xy, 0, i as i32);
                xn = xn.ref_single(cur_intf_xy, i, intf)
            }
            xn.bels(bels).extract();
        }
    }

    for tkn in [
        "DSP_LOCF_B_TILE",
        "DSP_LOCF_T_TILE",
        "DSP_ROCF_B_TILE",
        "DSP_ROCF_T_TILE",
    ] {
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
            let intf_l = builder.ndb.get_node_naming("INTF.E");
            let intf_r = builder.ndb.get_node_naming("INTF.W");
            let naming = if dev_naming.is_dsp_v2 {
                "DSP.V2"
            } else {
                "DSP.V1"
            };
            let intf0_xy = builder.delta(xy, -1, 0);
            let intf1_xy = builder.delta(xy, -1, 1);
            let intf2_xy = builder.delta(xy, 2, 0);
            let intf3_xy = builder.delta(xy, 2, 1);
            builder
                .xnode("DSP", naming, xy)
                .num_tiles(4)
                .ref_single(intf0_xy, 0, intf_l)
                .ref_single(intf1_xy, 1, intf_l)
                .ref_single(intf2_xy, 2, intf_r)
                .ref_single(intf3_xy, 3, intf_r)
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
        ("DFE_CFC_BOT", "DFE_CFC_BOT_TILE", "DFE_CFC_BOT", false),
        ("DFE_CFC_TOP", "DFE_CFC_TOP_TILE", "DFE_CFC_TOP", false),
        ("SDFEC", "SDFECA_TOP_TILE", "SDFEC_A", false),
        ("DCMAC", "DCMAC_TILE", "DCMAC", true),
        ("ILKN", "ILKN_TILE", "ILKNF", true),
        ("HSC", "HSC_TILE", "HSC", true),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bel = builder.bel_xy(kind, sk, 0, 0);
            let intf_l = builder.ndb.get_node_naming("INTF.E.HB");
            let intf_r = builder.ndb.get_node_naming("INTF.W.HB");
            let height = if is_large { 96 } else { 48 };
            let mut xn = builder.xnode(kind, kind, xy).num_tiles(height * 2);
            for i in 0..height {
                let intf_l_xy = xn.builder.delta(xy, -1, (i + i / 4) as i32);
                let intf_r_xy = xn.builder.delta(xy, 1, (i + i / 4) as i32);
                xn = xn
                    .ref_single(intf_l_xy, i, intf_l)
                    .ref_single(intf_r_xy, i + height, intf_r)
            }
            xn.bel(bel).extract();
        }
    }

    for tkn in ["HDIO_TILE", "HDIO_BOT_TILE"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bels = vec![];
            for i in 0..11 {
                bels.push(
                    builder
                        .bel_xy(format!("HDIOLOGIC{i}"), "HDIOLOGIC", 0, i)
                        .pin_name_only("TFFM_Q", 1)
                        .pin_name_only("TFFS_Q", 1)
                        .pin_name_only("OPFFM_Q", 1)
                        .pin_name_only("OPFFS_Q", 1)
                        .pin_name_only("IPFFM_D", 0)
                        .pin_name_only("IPFFS_D", 0),
                );
            }
            for i in 0..11 {
                bels.push(
                    builder
                        .bel_xy(format!("HDIOB{i}"), "IOB", 0, i)
                        .pin_name_only("RXOUT_M", 1)
                        .pin_name_only("RXOUT_S", 1)
                        .pin_name_only("OP_M", 0)
                        .pin_name_only("OP_S", 0)
                        .pin_name_only("TRISTATE_M", 0)
                        .pin_name_only("TRISTATE_S", 0),
                );
            }
            for i in 0..4 {
                let mut bel = builder
                    .bel_xy(format!("BUFGCE_HDIO{i}"), "BUFGCE_HDIO", 0, i)
                    .pin_name_only("O", 1)
                    .pin_name_only("I", 1);
                for j in 0..8 {
                    bel = bel.extra_wire(
                        format!("I_DUMMY{j}"),
                        &[format!("VCC_WIRE{k}", k = i * 8 + j)],
                    );
                }
                bels.push(bel);
            }
            bels.push(
                builder
                    .bel_xy("DPLL.HDIO", "DPLL", 0, 0)
                    .pin_name_only("CLKIN", 1)
                    .extra_int_in("CLKIN_INT", &["IF_COE_W24_CTRL14"])
                    .extra_wire("CLKIN_RCLK", &["IF_RCLK_CLK_TO_DPLL"])
                    .pin_name_only("CLKIN_DESKEW", 1)
                    .extra_wire("CLKIN_DESKEW_DUMMY0", &["VCC_WIRE32"])
                    .extra_wire("CLKIN_DESKEW_DUMMY1", &["VCC_WIRE33"])
                    .pin_name_only("CLKOUT0", 1)
                    .pin_name_only("CLKOUT1", 1)
                    .pin_name_only("CLKOUT2", 1)
                    .pin_name_only("CLKOUT3", 1)
                    .pin_name_only("TMUXOUT", 1),
            );
            bels.push(builder.bel_xy("HDIO_BIAS", "HDIO_BIAS", 0, 0));
            bels.push(builder.bel_xy("RPI_HD_APB", "RPI_HD_APB", 0, 0));
            bels.push(builder.bel_xy("HDLOGIC_APB", "HDLOGIC_APB", 0, 0));
            bels.push(
                builder
                    .bel_virtual("VCC.HDIO")
                    .extra_wire("VCC", &["VCC_WIRE"]),
            );
            let intf_l = builder.ndb.get_node_naming("INTF.E.HB");
            let intf_r = builder.ndb.get_node_naming("INTF.W.HB");
            let mut xn = builder.xnode("HDIO", "HDIO", xy).num_tiles(96);
            for i in 0..48 {
                let intf_l_xy = xn.builder.delta(xy, -1, (i + i / 4) as i32);
                let intf_r_xy = xn.builder.delta(xy, 1, (i + i / 4) as i32);
                xn = xn
                    .ref_single(intf_l_xy, i, intf_l)
                    .ref_single(intf_r_xy, i + 48, intf_r)
            }
            xn.bels(bels).extract();
        }
    }

    for (tkn, naming_f, naming_h, bkind, swz) in [
        (
            "RCLK_CLE_CORE",
            "RCLK_CLE",
            "RCLK_CLE.HALF",
            "BUFDIV_LEAF",
            BUFDIV_LEAF_SWZ_A,
        ),
        (
            "RCLK_CLE_VR_CORE",
            "RCLK_CLE.VR",
            "RCLK_CLE.HALF.VR",
            "BUFDIV_LEAF_ULVT",
            BUFDIV_LEAF_SWZ_B,
        ),
        (
            "RCLK_CLE_LAG_CORE",
            "RCLK_CLE.LAG",
            "RCLK_CLE.HALF.LAG",
            "BUFDIV_LEAF",
            BUFDIV_LEAF_SWZ_B,
        ),
    ] {
        let mut done_full = false;
        let mut done_half = false;
        for &xy in rd.tiles_by_kind_name(tkn) {
            let td = &rd.tiles[&builder.delta(xy, 0, -1)];
            let is_full = matches!(
                &rd.tile_kinds.key(td.kind)[..],
                "CLE_W_CORE" | "CLE_W_VR_CORE"
            );
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
                    .bel_xy(format!("BUFDIV_LEAF.CLE.{i}"), bkind, 0, y as u8)
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
            let int_r_xy = builder.delta(
                builder
                    .walk_to_int(builder.delta(xy, 0, 1), Dir::E, false)
                    .unwrap(),
                0,
                -1,
            );
            let int_u_xy = builder.delta(xy, 1, 1);
            let int_d_xy = builder.delta(xy, 1, -1);
            let mut xn = builder
                .xnode(kind, naming, xy)
                .num_tiles(if is_full { 2 } else { 1 })
                .ref_single(int_r_xy, 0, rclk_int)
                .ref_xlat(int_u_xy, &[None, Some(0)], cle_bc);
            if is_full {
                xn = xn.ref_xlat(int_d_xy, &[None, Some(1)], cle_bc);
            }
            xn.bels(bels).extract();
        }
    }

    for (dir, naming, tkn, bkind, intf_dx, swz, has_dfx) in [
        (
            Dir::E,
            "DSP",
            "RCLK_DSP_CORE",
            "BUFDIV_LEAF",
            0,
            BUFDIV_LEAF_SWZ_A,
            false,
        ),
        (
            Dir::W,
            "DSP",
            "RCLK_DSP_CORE",
            "BUFDIV_LEAF",
            3,
            BUFDIV_LEAF_SWZ_AH,
            true,
        ),
        (
            Dir::E,
            "DSP.VR",
            "RCLK_DSP_VR_CORE",
            "BUFDIV_LEAF_ULVT",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::W,
            "DSP.VR",
            "RCLK_DSP_VR_CORE",
            "BUFDIV_LEAF_ULVT",
            3,
            BUFDIV_LEAF_SWZ_BH,
            true,
        ),
        (
            Dir::E,
            "HB",
            "RCLK_HB_CORE",
            "BUFDIV_LEAF",
            0,
            BUFDIV_LEAF_SWZ_A,
            false,
        ),
        (
            Dir::E,
            "HB.VR",
            "RCLK_HB_VR_CORE",
            "BUFDIV_LEAF_ULVT",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::W,
            "HB",
            "RCLK_HB_CORE",
            "BUFDIV_LEAF",
            2,
            BUFDIV_LEAF_SWZ_AH,
            false,
        ),
        (
            Dir::W,
            "HB.VR",
            "RCLK_HB_VR_CORE",
            "BUFDIV_LEAF_ULVT",
            2,
            BUFDIV_LEAF_SWZ_BH,
            false,
        ),
        (
            Dir::E,
            "SDFEC",
            "RCLK_SDFEC_CORE",
            "BUFDIV_LEAF_ULVT",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::W,
            "SDFEC",
            "RCLK_SDFEC_CORE",
            "BUFDIV_LEAF_ULVT",
            2,
            BUFDIV_LEAF_SWZ_BH,
            false,
        ),
        (
            Dir::E,
            "HDIO",
            "RCLK_HDIO_CORE",
            "BUFDIV_LEAF",
            0,
            BUFDIV_LEAF_SWZ_A,
            false,
        ),
        (
            Dir::E,
            "HDIO.VR",
            "RCLK_HDIO_VR_CORE",
            "BUFDIV_LEAF_ULVT",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::W,
            "HDIO",
            "RCLK_HDIO_CORE",
            "BUFDIV_LEAF",
            2,
            BUFDIV_LEAF_SWZ_AH,
            false,
        ),
        (
            Dir::W,
            "HDIO.VR",
            "RCLK_HDIO_VR_CORE",
            "BUFDIV_LEAF_ULVT",
            2,
            BUFDIV_LEAF_SWZ_BH,
            false,
        ),
        (
            Dir::E,
            "HB_HDIO",
            "RCLK_HB_HDIO_CORE",
            "BUFDIV_LEAF",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::E,
            "HB_HDIO.VR",
            "RCLK_HB_HDIO_VR_CORE",
            "BUFDIV_LEAF_ULVT",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::W,
            "HB_HDIO",
            "RCLK_HB_HDIO_CORE",
            "BUFDIV_LEAF",
            2,
            BUFDIV_LEAF_SWZ_BH,
            false,
        ),
        (
            Dir::W,
            "HB_HDIO.VR",
            "RCLK_HB_HDIO_VR_CORE",
            "BUFDIV_LEAF_ULVT",
            2,
            BUFDIV_LEAF_SWZ_BH,
            false,
        ),
        (
            Dir::W,
            "VNOC",
            "RCLK_INTF_L_CORE",
            "BUFDIV_LEAF",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::W,
            "VNOC.VR",
            "RCLK_INTF_L_VR_CORE",
            "BUFDIV_LEAF_ULVT",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::E,
            "VNOC",
            "RCLK_INTF_R_CORE",
            "BUFDIV_LEAF",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::E,
            "VNOC.VR",
            "RCLK_INTF_R_VR_CORE",
            "BUFDIV_LEAF_ULVT",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::W,
            "CFRM",
            "RCLK_INTF_OPT_CORE",
            "BUFDIV_LEAF",
            0,
            BUFDIV_LEAF_SWZ_A,
            false,
        ),
        (
            Dir::W,
            "CFRM.VR",
            "RCLK_INTF_OPT_VR_CORE",
            "BUFDIV_LEAF_ULVT",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::W,
            "GT",
            "RCLK_INTF_TERM_LEFT_CORE",
            "BUFDIV_LEAF",
            1,
            BUFDIV_LEAF_SWZ_A,
            false,
        ),
        (
            Dir::W,
            "GT.VR",
            "RCLK_INTF_TERM_LEFT_VR_CORE",
            "BUFDIV_LEAF_ULVT",
            1,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::E,
            "GT",
            "RCLK_INTF_TERM_RIGHT_CORE",
            "BUFDIV_LEAF",
            0,
            BUFDIV_LEAF_SWZ_A,
            false,
        ),
        (
            Dir::E,
            "GT.VR",
            "RCLK_INTF_TERM_RIGHT_VR_CORE",
            "BUFDIV_LEAF_ULVT",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::E,
            "GT.ALT",
            "RCLK_INTF_TERM2_RIGHT_CORE",
            "BUFDIV_LEAF",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::W,
            "BRAM",
            "RCLK_BRAM_CORE_MY",
            "BUFDIV_LEAF",
            1,
            BUFDIV_LEAF_SWZ_A,
            true,
        ),
        (
            Dir::W,
            "BRAM.VR",
            "RCLK_BRAM_VR_CORE_MY",
            "BUFDIV_LEAF_ULVT",
            1,
            BUFDIV_LEAF_SWZ_B,
            true,
        ),
        (
            Dir::W,
            "URAM",
            "RCLK_URAM_CORE_MY",
            "BUFDIV_LEAF",
            1,
            BUFDIV_LEAF_SWZ_A,
            true,
        ),
        (
            Dir::W,
            "URAM.VR",
            "RCLK_URAM_VR_CORE_MY",
            "BUFDIV_LEAF_ULVT",
            1,
            BUFDIV_LEAF_SWZ_B,
            true,
        ),
        (
            Dir::E,
            "BRAM",
            "RCLK_BRAM_CORE",
            "BUFDIV_LEAF",
            0,
            BUFDIV_LEAF_SWZ_A,
            true,
        ),
        (
            Dir::E,
            "BRAM.VR",
            "RCLK_BRAM_VR_CORE",
            "BUFDIV_LEAF_ULVT",
            0,
            BUFDIV_LEAF_SWZ_B,
            true,
        ),
        (
            Dir::E,
            "BRAM.CLKBUF",
            "RCLK_BRAM_CLKBUF_CORE",
            "BUFDIV_LEAF",
            0,
            BUFDIV_LEAF_SWZ_A,
            true,
        ),
        (
            Dir::E,
            "BRAM.CLKBUF.VR",
            "RCLK_BRAM_CLKBUF_VR_CORE",
            "BUFDIV_LEAF_ULVT",
            0,
            BUFDIV_LEAF_SWZ_B,
            true,
        ),
        (
            Dir::E,
            "BRAM.CLKBUF.NOPD",
            "RCLK_BRAM_CLKBUF_NOPD_CORE",
            "BUFDIV_LEAF",
            0,
            BUFDIV_LEAF_SWZ_B,
            true,
        ),
        (
            Dir::E,
            "BRAM.CLKBUF.NOPD.VR",
            "RCLK_BRAM_CLKBUF_NOPD_VR_CORE",
            "BUFDIV_LEAF_ULVT",
            0,
            BUFDIV_LEAF_SWZ_B,
            true,
        ),
        (
            Dir::W,
            "HB_FULL",
            "RCLK_HB_FULL_R_CORE",
            "BUFDIV_LEAF",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
        (
            Dir::E,
            "HB_FULL",
            "RCLK_HB_FULL_L_CORE",
            "BUFDIV_LEAF",
            0,
            BUFDIV_LEAF_SWZ_B,
            false,
        ),
    ] {
        let mut done_full = false;
        let mut done_half = false;
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = builder.delta(
                builder
                    .walk_to_int(builder.delta(xy, 0, 1), !dir, false)
                    .unwrap(),
                0,
                -1,
            );
            if int_xy.x.abs_diff(xy.x) > 5 {
                continue;
            }
            if rd
                .tile_kinds
                .key(rd.tiles[&builder.delta(int_xy, 0, 1)].kind)
                != "INT"
            {
                continue;
            }
            let td = &rd.tiles[&builder.delta(int_xy, 0, -1)];
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
                    .bel_xy(format!("BUFDIV_LEAF.{dir}.{i}"), bkind, 0, y as u8)
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
                .ndb
                .get_node_naming(if dir == Dir::E { "INTF.E" } else { "INTF.W" });
            let half = if is_full { "" } else { ".HALF" };
            let intf_u_xy = builder.delta(xy, intf_dx, 1);
            let intf_d_xy = builder.delta(xy, intf_dx, -1);
            let mut xn = builder
                .xnode(
                    format!("RCLK_INTF.{dir}{half}"),
                    format!("RCLK_INTF.{dir}{half}.{naming}"),
                    xy,
                )
                .num_tiles(if is_full { 2 } else { 1 })
                .ref_single(int_xy, 0, rclk_int)
                .ref_single(intf_u_xy, 0, intf);
            if is_full {
                xn = xn.ref_single(intf_d_xy, 1, intf);
            }
            xn.bels(bels).extract();
            if has_dfx {
                let bel = builder.bel_xy(format!("RCLK_DFX_TEST.{dir}"), "RCLK", 0, 0);
                builder
                    .xnode(
                        format!("RCLK_DFX.{dir}"),
                        format!("RCLK_DFX.{dir}.{naming}"),
                        xy,
                    )
                    .ref_single(int_xy, 0, rclk_int)
                    .bel(bel)
                    .extract();
            }
        }
    }

    for (tkn, naming) in [
        ("RCLK_HDIO_CORE", "RCLK_HDIO"),
        ("RCLK_HDIO_VR_CORE", "RCLK_HDIO.VR"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bel_dpll = builder
                .bel_virtual("RCLK_HDIO_DPLL")
                .extra_wire("OUT_S", &["IF_RCLK_BOT_CLK_TO_DPLL"])
                .extra_wire("OUT_N", &["IF_RCLK_TOP_CLK_TO_DPLL"]);
            let mut bel_hdio = builder.bel_virtual("RCLK_HDIO");
            for i in 0..4 {
                bel_hdio = bel_hdio
                    .extra_wire(
                        format!("BUFGCE_OUT_S{i}"),
                        &[format!("IF_RCLK_BOT_CLK_FROM_BUFG{i}")],
                    )
                    .extra_wire(
                        format!("BUFGCE_OUT_N{i}"),
                        &[format!("IF_RCLK_TOP_CLK_FROM_BUFG{i}")],
                    );
            }
            let swz = [
                0, 3, 4, 5, 6, 7, 8, 9, 10, 11, 1, 2, 12, 15, 16, 17, 18, 19, 20, 21, 22, 23, 13,
                14,
            ];
            for (i, si) in swz.into_iter().enumerate() {
                bel_hdio = bel_hdio
                    .extra_wire(
                        format!("HDISTR{i}"),
                        &[
                            format!("IF_HCLK_CLK_HDISTR{i}"),
                            match i {
                                0..8 => format!("CLK_HDISTR_LSB{i}"),
                                8..12 | 20..24 => format!("CLK_CMT_DRVR_TRI_ULVT_{si}_CLK_OUT_B"),
                                12..20 => format!("CLK_HDISTR_MSB{ii}", ii = i - 12),
                                _ => unreachable!(),
                            },
                        ],
                    )
                    .extra_wire(
                        format!("HDISTR{i}_MUX"),
                        &[
                            format!("CLK_CMT_MUX_8TO1_{si}_CLK_OUT"),
                            format!("CLK_CMT_MUX_8TO1_ULVT_{si}_CLK_OUT"),
                        ],
                    );
            }
            for i in 0..12 {
                bel_hdio = bel_hdio
                    .extra_wire(format!("HROUTE{i}"), &[format!("IF_HCLK_CLK_HROUTE{i}")])
                    .extra_wire(
                        format!("HROUTE{i}_MUX"),
                        &[
                            format!("CLK_CMT_MUX_8TO1_{si}_CLK_OUT", si = 24 + i),
                            format!("CLK_CMT_MUX_8TO1_ULVT_{si}_CLK_OUT", si = 24 + i),
                        ],
                    );
            }
            builder
                .xnode("RCLK_HDIO", naming, xy)
                .num_tiles(0)
                .bel(bel_hdio)
                .bel(bel_dpll)
                .extract();
        }
    }

    for (tkn, naming) in [
        ("RCLK_HB_HDIO_CORE", "RCLK_HB_HDIO"),
        ("RCLK_HB_HDIO_VR_CORE", "RCLK_HB_HDIO.VR"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bel_dpll = builder
                .bel_virtual("RCLK_HDIO_DPLL")
                .extra_wire("OUT_S", &["IF_RCLK_BOT_CLK_TO_DPLL"])
                .extra_wire(
                    "OUT_N",
                    &[
                        "CLK_CMT_MUX_24_ENC_1_CLK_OUT",
                        "CLK_CMT_MUX_24_ENC_ULVT_1_CLK_OUT",
                    ],
                );
            let mut bel_hdio = builder.bel_virtual("RCLK_HB_HDIO");
            for i in 0..4 {
                bel_hdio = bel_hdio.extra_wire(
                    format!("BUFGCE_OUT_S{i}"),
                    &[format!("IF_RCLK_BOT_CLK_FROM_BUFG{i}")],
                );
            }
            let swz = [
                0, 3, 4, 5, 6, 7, 8, 9, 10, 11, 1, 2, 12, 15, 16, 17, 18, 19, 20, 21, 22, 23, 13,
                14,
            ];
            for (i, si) in swz.into_iter().enumerate() {
                bel_hdio = bel_hdio
                    .extra_wire(
                        format!("HDISTR{i}"),
                        &[
                            format!("IF_HCLK_CLK_HDISTR{i}"),
                            match i {
                                0..8 => format!("CLK_HDISTR_LSB{i}"),
                                8..12 | 20..24 => format!("CLK_CMT_DRVR_TRI_ULVT_{si}_CLK_OUT_B"),
                                12..20 => format!("CLK_HDISTR_MSB{ii}", ii = i - 12),
                                _ => unreachable!(),
                            },
                        ],
                    )
                    .extra_wire(
                        format!("HDISTR{i}_MUX"),
                        &[
                            format!("CLK_CMT_MUX_8TO1_{si}_CLK_OUT"),
                            format!("CLK_CMT_MUX_8TO1_ULVT_{si}_CLK_OUT"),
                        ],
                    );
                let b = [
                    0, 92, 120, 124, 128, 132, 136, 140, 8, 12, 4, 48, 16, 28, 32, 36, 40, 44, 52,
                    56, 60, 64, 20, 24,
                ][i];
                for j in 0..4 {
                    bel_hdio = bel_hdio.extra_wire(
                        format!("HDISTR{i}_MUX_DUMMY{j}"),
                        &[format!("GND_WIRE{k}", k = b + j)],
                    );
                }
            }
            for i in 0..12 {
                bel_hdio = bel_hdio
                    .extra_wire(format!("HROUTE{i}"), &[format!("IF_HCLK_CLK_HROUTE{i}")])
                    .extra_wire(
                        format!("HROUTE{i}_MUX"),
                        &[
                            format!("CLK_CMT_MUX_8TO1_{si}_CLK_OUT", si = 24 + i),
                            format!("CLK_CMT_MUX_8TO1_ULVT_{si}_CLK_OUT", si = 24 + i),
                        ],
                    );
                let b = [68, 72, 76, 80, 84, 88, 96, 100, 104, 108, 112, 116][i];
                for j in 0..4 {
                    bel_hdio = bel_hdio.extra_wire(
                        format!("HROUTE{i}_MUX_DUMMY{j}"),
                        &[format!("GND_WIRE{k}", k = b + j)],
                    );
                }
            }
            builder
                .xnode("RCLK_HB_HDIO", naming, xy)
                .num_tiles(0)
                .bel(bel_hdio)
                .bel(bel_dpll)
                .extract();
        }
    }

    // XXX RCLK_CLKBUF

    if let Some(&xy) = rd.tiles_by_kind_name("AMS_SAT_VNOC_TILE").iter().next() {
        let bel = builder.bel_xy("SYSMON_SAT.VNOC", "SYSMON_SAT", 0, 0);
        let intf_l = builder.ndb.get_node_naming("INTF.E");
        let mut xn = builder
            .xnode("SYSMON_SAT.VNOC", "SYSMON_SAT.VNOC", xy)
            .num_tiles(96);
        for i in 0..48 {
            let intf_xy = xn.builder.delta(xy, -1, -49 + (i + i / 4) as i32);
            xn = xn.ref_single(intf_xy, i, intf_l)
        }
        xn.bel(bel).extract();
    }

    for (kind, dpll_kind, tkn, intf_kind, int_dir) in [
        (
            "SYSMON_SAT.LGT",
            "DPLL.LGT",
            "AMS_SAT_GT_BOT_TILE_MY",
            "INTF.W.TERM.GT",
            Dir::E,
        ),
        (
            "SYSMON_SAT.LGT",
            "DPLL.LGT",
            "AMS_SAT_GT_TOP_TILE_MY",
            "INTF.W.TERM.GT",
            Dir::E,
        ),
        (
            "SYSMON_SAT.RGT",
            "DPLL.RGT",
            "AMS_SAT_GT_BOT_TILE",
            "INTF.E.TERM.GT",
            Dir::W,
        ),
        (
            "SYSMON_SAT.RGT",
            "DPLL.RGT",
            "AMS_SAT_GT_TOP_TILE",
            "INTF.E.TERM.GT",
            Dir::W,
        ),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bel = builder.bel_xy(kind, "SYSMON_SAT", 0, 0);
            let intf = builder.ndb.get_node_naming(intf_kind);
            let base_xy = builder.delta(xy, 0, -24);
            let int_xy = builder.walk_to_int(base_xy, int_dir, true).unwrap();
            let intf_xy = builder.delta(int_xy, if int_dir == Dir::E { -1 } else { 1 }, 0);
            let mut xn = builder.xnode(kind, kind, xy).num_tiles(48);
            for i in 0..48 {
                let intf_xy = xn.builder.delta(intf_xy, 0, (i + i / 4) as i32);
                xn = xn.ref_single(intf_xy, i, intf)
            }
            xn.bel(bel).extract();
            let bel = builder
                .bel_xy(dpll_kind, "DPLL", 0, 0)
                .pin_name_only("CLKIN", 1)
                .pin_name_only("CLKIN_DESKEW", 1)
                .pin_name_only("CLKOUT0", 1)
                .pin_name_only("CLKOUT1", 1)
                .pin_name_only("CLKOUT2", 1)
                .pin_name_only("CLKOUT3", 1)
                .pin_name_only("TMUXOUT", 1);
            let dpll_xy = builder.delta(xy, 0, -15);
            let mut xn = builder.xnode(dpll_kind, dpll_kind, dpll_xy).num_tiles(48);
            for i in 0..48 {
                let intf_xy = xn.builder.delta(intf_xy, 0, (i + i / 4) as i32);
                xn = xn.ref_single(intf_xy, i, intf)
            }
            xn.bel(bel).extract();
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("MISR_TILE").iter().next() {
        let bel = builder.bel_xy("MISR", "MISR", 0, 0);
        let intf_r = builder.ndb.get_node_naming("INTF.W");
        let mut xn = builder.xnode("MISR", "MISR", xy).num_tiles(96);
        for i in 0..48 {
            let intf_xy = xn.builder.delta(xy, 1, 1 + (i + i / 4) as i32);
            xn = xn.ref_single(intf_xy, i + 48, intf_r)
        }
        xn.bel(bel).extract();
    }

    if let Some(&nsu_xy) = rd.tiles_by_kind_name("NOC_NSU512_TOP").iter().next() {
        let nps_a_xy = builder.delta(nsu_xy, 0, 9);
        let nps_b_xy = builder.delta(nsu_xy, 0, 19);
        let nmu_xy = builder.delta(nsu_xy, 0, 29);
        let intf_l = builder.ndb.get_node_naming("INTF.E");
        let intf_r = builder.ndb.get_node_naming("INTF.W");
        let bels = [
            builder
                .bel_xy("VNOC_NSU512", "NOC_NSU512", 0, 0)
                .pin_name_only("TO_NOC", 1)
                .pin_name_only("FROM_NOC", 1),
            builder
                .bel_xy("VNOC_NPS_A", "NOC_NPS_VNOC", 0, 0)
                .raw_tile(1)
                .pin_name_only("IN_0", 1)
                .pin_name_only("IN_1", 1)
                .pin_name_only("IN_2", 1)
                .pin_name_only("IN_3", 1)
                .pin_name_only("OUT_0", 1)
                .pin_name_only("OUT_1", 1)
                .pin_name_only("OUT_2", 1)
                .pin_name_only("OUT_3", 1),
            builder
                .bel_xy("VNOC_NPS_B", "NOC_NPS_VNOC", 0, 0)
                .raw_tile(2)
                .pin_name_only("IN_0", 1)
                .pin_name_only("IN_1", 1)
                .pin_name_only("IN_2", 1)
                .pin_name_only("IN_3", 1)
                .pin_name_only("OUT_0", 1)
                .pin_name_only("OUT_1", 1)
                .pin_name_only("OUT_2", 1)
                .pin_name_only("OUT_3", 1),
            builder
                .bel_xy("VNOC_NMU512", "NOC_NMU512", 0, 0)
                .raw_tile(3)
                .pin_name_only("TO_NOC", 1)
                .pin_name_only("FROM_NOC", 1),
        ];
        let mut xn = builder
            .xnode("VNOC", "VNOC", nsu_xy)
            .num_tiles(96)
            .raw_tile(nps_a_xy)
            .raw_tile(nps_b_xy)
            .raw_tile(nmu_xy);
        for i in 0..48 {
            let intf_l_xy = xn.builder.delta(nsu_xy, -1, -9 + (i + i / 4) as i32);
            let intf_r_xy = xn.builder.delta(nsu_xy, 2, -9 + (i + i / 4) as i32);
            xn = xn
                .ref_single(intf_l_xy, i, intf_l)
                .ref_single(intf_r_xy, i + 48, intf_r)
        }
        xn.bels(bels).extract();
    }

    if let Some(&nsu_xy) = rd.tiles_by_kind_name("NOC2_NSU512_VNOC_TILE").iter().next() {
        let mut nps_a_xy = builder.delta(nsu_xy, 0, 4);
        if rd.tile_kinds.key(rd.tiles[&nps_a_xy].kind) == "NULL" {
            nps_a_xy = builder.delta(nps_a_xy, -1, 0);
        }
        let nps_b_xy = builder.delta(nps_a_xy, 0, 4);
        let nmu_xy = builder.delta(nps_a_xy, 0, 7);
        let scan_xy = builder.delta(nsu_xy, 1, 0);
        let intf_l = builder.ndb.get_node_naming("INTF.E");
        let intf_r = builder.ndb.get_node_naming("INTF.W");
        let mut bel_scan = builder.bel_xy("VNOC2_SCAN", "NOC2_SCAN", 0, 0).raw_tile(4);
        for i in 6..15 {
            bel_scan = bel_scan
                .pin_name_only(&format!("NOC2_SCAN_CHNL_FROM_PL_{i}_"), 1)
                .pin_name_only(&format!("NOC2_SCAN_CHNL_TO_PL_{i}_"), 1);
        }
        for i in 5..14 {
            bel_scan = bel_scan.pin_name_only(&format!("NOC2_SCAN_CHNL_MASK_FROM_PL_{i}_"), 1);
        }
        let bels = [
            builder
                .bel_xy("VNOC2_NSU512", "NOC2_NSU512", 0, 0)
                .pin_name_only("TO_NOC", 1)
                .pin_name_only("FROM_NOC", 1),
            builder
                .bel_xy("VNOC2_NPS_A", "NOC2_NPS5555", 0, 0)
                .raw_tile(1)
                .pin_name_only("IN_0", 1)
                .pin_name_only("IN_1", 1)
                .pin_name_only("IN_2", 1)
                .pin_name_only("IN_3", 1)
                .pin_name_only("OUT_0", 1)
                .pin_name_only("OUT_1", 1)
                .pin_name_only("OUT_2", 1)
                .pin_name_only("OUT_3", 1),
            builder
                .bel_xy("VNOC2_NPS_B", "NOC2_NPS5555", 0, 0)
                .raw_tile(2)
                .pin_name_only("IN_0", 1)
                .pin_name_only("IN_1", 1)
                .pin_name_only("IN_2", 1)
                .pin_name_only("IN_3", 1)
                .pin_name_only("OUT_0", 1)
                .pin_name_only("OUT_1", 1)
                .pin_name_only("OUT_2", 1)
                .pin_name_only("OUT_3", 1),
            builder
                .bel_xy("VNOC2_NMU512", "NOC2_NMU512", 0, 0)
                .raw_tile(3)
                .pin_name_only("TO_NOC", 1)
                .pin_name_only("FROM_NOC", 1),
            bel_scan,
        ];
        let mut xn = builder
            .xnode("VNOC2", "VNOC2", nsu_xy)
            .num_tiles(96)
            .raw_tile(nps_a_xy)
            .raw_tile(nps_b_xy)
            .raw_tile(nmu_xy)
            .raw_tile(scan_xy);
        for i in 0..48 {
            let intf_l_xy = xn.builder.delta(nps_a_xy, -1, -13 + (i + i / 4) as i32);
            let intf_r_xy = xn.builder.delta(nps_a_xy, 3, -13 + (i + i / 4) as i32);

            xn = xn
                .ref_single(intf_l_xy, i, intf_l)
                .ref_single(intf_r_xy, i + 48, intf_r)
        }
        xn.bels(bels).extract();
    }

    if let Some(&nsu_xy) = rd
        .tiles_by_kind_name("NOC2_NSU512_VNOC4_TILE")
        .iter()
        .next()
    {
        let nps_a_xy = builder.delta(nsu_xy, -1, 4);
        let nps_b_xy = builder.delta(nps_a_xy, 0, 4);
        let nmu_xy = builder.delta(nps_a_xy, 0, 7);
        let scan_xy = builder.delta(nsu_xy, 1, 0);
        let intf_l = builder.ndb.get_node_naming("INTF.E");
        let intf_r = builder.ndb.get_node_naming("INTF.W");
        let mut bel_scan = builder.bel_xy("VNOC4_SCAN", "NOC2_SCAN", 0, 0).raw_tile(4);
        for i in 7..15 {
            bel_scan = bel_scan
                .pin_name_only(&format!("NOC2_SCAN_CHNL_FROM_PL_{i}_"), 1)
                .pin_name_only(&format!("NOC2_SCAN_CHNL_TO_PL_{i}_"), 1);
        }
        for i in 7..14 {
            bel_scan = bel_scan.pin_name_only(&format!("NOC2_SCAN_CHNL_MASK_FROM_PL_{i}_"), 1);
        }
        let bels = [
            builder
                .bel_xy("VNOC4_NSU512", "NOC2_NSU512", 0, 0)
                .pin_name_only("TO_NOC", 1)
                .pin_name_only("FROM_NOC", 1),
            builder
                .bel_xy("VNOC4_NPS_A", "NOC2_NPS6X", 0, 0)
                .raw_tile(1)
                .pin_name_only("IN_0", 1)
                .pin_name_only("IN_1", 1)
                .pin_name_only("IN_2", 1)
                .pin_name_only("IN_3", 1)
                .pin_name_only("IN_4", 1)
                .pin_name_only("IN_5", 1)
                .pin_name_only("OUT_0", 1)
                .pin_name_only("OUT_1", 1)
                .pin_name_only("OUT_2", 1)
                .pin_name_only("OUT_3", 1)
                .pin_name_only("OUT_4", 1)
                .pin_name_only("OUT_5", 1),
            builder
                .bel_xy("VNOC4_NPS_B", "NOC2_NPS6X", 0, 0)
                .raw_tile(2)
                .pin_name_only("IN_0", 1)
                .pin_name_only("IN_1", 1)
                .pin_name_only("IN_2", 1)
                .pin_name_only("IN_3", 1)
                .pin_name_only("IN_4", 1)
                .pin_name_only("IN_5", 1)
                .pin_name_only("OUT_0", 1)
                .pin_name_only("OUT_1", 1)
                .pin_name_only("OUT_2", 1)
                .pin_name_only("OUT_3", 1)
                .pin_name_only("OUT_4", 1)
                .pin_name_only("OUT_5", 1),
            builder
                .bel_xy("VNOC4_NMU512", "NOC2_NMU512", 0, 0)
                .raw_tile(3)
                .pin_name_only("TO_NOC", 1)
                .pin_name_only("FROM_NOC", 1),
            bel_scan,
        ];
        let mut xn = builder
            .xnode("VNOC4", "VNOC4", nsu_xy)
            .num_tiles(96)
            .raw_tile(nps_a_xy)
            .raw_tile(nps_b_xy)
            .raw_tile(nmu_xy)
            .raw_tile(scan_xy);
        for i in 0..48 {
            let intf_l_xy = xn.builder.delta(nps_a_xy, -1, -13 + (i + i / 4) as i32);
            let intf_r_xy = xn.builder.delta(nps_a_xy, 5, -13 + (i + i / 4) as i32);

            xn = xn
                .ref_single(intf_l_xy, i, intf_l)
                .ref_single(intf_r_xy, i + 48, intf_r)
        }
        xn.bels(bels).extract();
    }

    builder.build()
}
