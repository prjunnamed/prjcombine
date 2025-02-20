#![allow(clippy::too_many_arguments)]

use enum_map::EnumMap;
use prjcombine_interconnect::{
    db::{NodeKind, NodeKindId},
    grid::{ColId, DieId, NodeLoc, RowId},
};
use prjcombine_versal::{
    expanded::ExpandedDevice,
    grid::{BramKind, CleKind, ColSide, ColumnKind, HardRowKind, InterposerKind, RegId, RightKind},
};
use prjcombine_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use serde::{Deserialize, Serialize};
use std::{cmp::max, collections::BTreeMap};
use unnamed_entity::{entity_id, EntityId, EntityPartVec, EntityVec};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceNaming {
    pub die: EntityVec<DieId, DieNaming>,
    pub is_dsp_v2: bool,
    pub is_vnoc2_scan_offset: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DieNaming {
    pub hdio: BTreeMap<(ColId, RegId), HdioNaming>,
    pub sysmon_sat_vnoc: BTreeMap<(ColId, RegId), (u32, u32)>,
    pub sysmon_sat_gt: BTreeMap<(ColId, RegId), (u32, u32)>,
    pub dpll_gt: BTreeMap<(ColId, RegId), (u32, u32)>,
    pub vnoc2: BTreeMap<(ColId, RegId), VNoc2Naming>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HdioNaming {
    pub iob_xy: (u32, u32),
    pub dpll_xy: (u32, u32),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VNoc2Naming {
    pub nsu_xy: (u32, u32),
    pub nmu_xy: (u32, u32),
    pub nps_xy: (u32, u32),
    pub scan_xy: (u32, u32),
}

pub const BUFDIV_LEAF_SWZ_A: [u32; 32] = [
    3, 2, 1, 0, 8, 9, 10, 11, 19, 18, 17, 16, 24, 25, 26, 27, 4, 5, 6, 7, 15, 14, 13, 12, 20, 21,
    22, 23, 31, 30, 29, 28,
];

pub const BUFDIV_LEAF_SWZ_B: [u32; 32] = [
    7, 6, 5, 4, 12, 13, 14, 15, 23, 22, 21, 20, 28, 29, 30, 31, 0, 1, 2, 3, 11, 10, 9, 8, 16, 17,
    18, 19, 27, 26, 25, 24,
];

pub const BUFDIV_LEAF_SWZ_AH: [u32; 32] = [
    35, 34, 33, 32, 40, 41, 42, 43, 51, 50, 49, 48, 56, 57, 58, 59, 36, 37, 38, 39, 47, 46, 45, 44,
    52, 53, 54, 55, 63, 62, 61, 60,
];

pub const BUFDIV_LEAF_SWZ_BH: [u32; 32] = [
    39, 38, 37, 36, 44, 45, 46, 47, 55, 54, 53, 52, 60, 61, 62, 63, 32, 33, 34, 35, 43, 42, 41, 40,
    48, 49, 50, 51, 59, 58, 57, 56,
];

entity_id! {
    id EColId u32, delta;
}

struct BelGrid {
    mirror_square: bool,
    xlut: EnumMap<ColSide, EntityVec<DieId, EntityPartVec<ColId, i32>>>,
    ylut: EntityVec<DieId, EntityPartVec<RowId, i32>>,
}

impl BelGrid {
    fn name(
        &self,
        prefix: &str,
        die: DieId,
        col: ColId,
        side: ColSide,
        row: RowId,
        dx: i32,
        dy: i32,
    ) -> String {
        self.name_mult(prefix, die, col, side, row, 1, dx, 1, dy)
    }

    fn name_mult(
        &self,
        prefix: &str,
        die: DieId,
        col: ColId,
        side: ColSide,
        row: RowId,
        mx: i32,
        dx: i32,
        my: i32,
        dy: i32,
    ) -> String {
        self.name_manual(
            prefix,
            die,
            (mx * self.xlut[side][die][col] + dx) as u32,
            (my * self.ylut[die][row] + dy) as u32,
        )
    }

    fn name_manual(&self, prefix: &str, die: DieId, x: u32, y: u32) -> String {
        if self.mirror_square {
            format!("{prefix}_S{die}X{x}Y{y}")
        } else {
            format!("{prefix}_X{x}Y{y}")
        }
    }
}

fn make_grid(
    edev: &ExpandedDevice,
    f_l: impl Fn(NodeKindId, &str, &NodeKind) -> bool,
    f_r: impl Fn(NodeKindId, &str, &NodeKind) -> bool,
    n_l: (i32, i32),
    n_r: (i32, i32),
) -> BelGrid {
    make_grid_complex(edev, f_l, f_r, |_, _, _, _| n_l, |_, _, _, _| n_r)
}

fn make_grid_complex(
    edev: &ExpandedDevice,
    f_l: impl Fn(NodeKindId, &str, &NodeKind) -> bool,
    f_r: impl Fn(NodeKindId, &str, &NodeKind) -> bool,
    n_l: impl Fn(NodeKindId, &str, &NodeKind, NodeLoc) -> (i32, i32),
    n_r: impl Fn(NodeKindId, &str, &NodeKind, NodeLoc) -> (i32, i32),
) -> BelGrid {
    if edev.interposer.kind == InterposerKind::MirrorSquare {
        let mut res = BelGrid {
            mirror_square: true,
            xlut: EnumMap::from_fn(|_| EntityVec::new()),
            ylut: EntityVec::new(),
        };
        for die in edev.grids.ids() {
            let mut cols = BTreeMap::new();
            let mut rows = BTreeMap::new();
            for (kind, name, node) in &edev.egrid.db.nodes {
                for side in [ColSide::Left, ColSide::Right] {
                    let ok = match side {
                        ColSide::Left => f_l(kind, name, node),
                        ColSide::Right => f_r(kind, name, node),
                    };
                    if ok {
                        for &nloc in &edev.egrid.node_index[kind] {
                            if nloc.0 != die {
                                continue;
                            }
                            let (n_x, n_y) = match side {
                                ColSide::Left => n_l(kind, name, node, nloc),
                                ColSide::Right => n_r(kind, name, node, nloc),
                            };
                            let v_c = cols.entry((nloc.1, side)).or_default();
                            *v_c = max(*v_c, n_x);
                            let v_r = rows.entry(nloc.2).or_default();
                            *v_r = max(*v_r, n_y);
                        }
                    }
                }
            }
            let mut xlut = EnumMap::from_fn(|_| EntityPartVec::new());
            let mut ylut = EntityPartVec::new();
            let mut x = 0;
            for ((col, side), num) in cols {
                xlut[side].insert(col, x);
                x += num;
            }
            let mut y = 0;
            for (row, num) in rows {
                ylut.insert(row, y);
                y += num;
            }
            for (k, v) in xlut {
                res.xlut[k].push(v);
            }
            res.ylut.push(ylut);
        }
        res
    } else {
        let mut cols: EntityVec<_, _> = edev.grids.ids().map(|_| BTreeMap::new()).collect();
        let mut rows = BTreeMap::new();
        for (kind, name, node) in &edev.egrid.db.nodes {
            for side in [ColSide::Left, ColSide::Right] {
                let ok = match side {
                    ColSide::Left => f_l(kind, name, node),
                    ColSide::Right => f_r(kind, name, node),
                };
                if ok {
                    for &nloc in &edev.egrid.node_index[kind] {
                        let (n_x, n_y) = match side {
                            ColSide::Left => n_l(kind, name, node, nloc),
                            ColSide::Right => n_r(kind, name, node, nloc),
                        };
                        let v_c = cols[nloc.0].entry((nloc.1, side)).or_default();
                        *v_c = max(*v_c, n_x);
                        let v_r = rows.entry((nloc.0, nloc.2)).or_default();
                        *v_r = max(*v_r, n_y);
                    }
                }
            }
        }
        let mut xlut: EnumMap<_, EntityVec<_, _>> =
            EnumMap::from_fn(|_| edev.grids.ids().map(|_| EntityPartVec::new()).collect());
        let mut x = 0;
        for die in edev.grids.ids() {
            let mut lx = 0;
            for (&(col, side), &num) in &cols[die] {
                if col >= edev.col_cfrm[die] {
                    continue;
                }
                xlut[side][die].insert(col, lx);
                lx += num;
            }
            x = max(x, lx);
        }
        for die in edev.grids.ids() {
            let mut lx = x;
            for (&(col, side), &num) in &cols[die] {
                if col < edev.col_cfrm[die] {
                    continue;
                }
                xlut[side][die].insert(col, lx);
                lx += num;
            }
        }
        let mut y = 0;
        let mut ylut: EntityVec<_, _> = edev.grids.ids().map(|_| EntityPartVec::new()).collect();
        for ((die, row), num) in rows {
            ylut[die].insert(row, y);
            y += num;
        }
        BelGrid {
            mirror_square: false,
            xlut,
            ylut,
        }
    }
}

pub struct ExpandedNamedDevice<'a> {
    pub edev: &'a ExpandedDevice<'a>,
    pub ngrid: ExpandedGridNaming<'a>,
}

pub fn name_device<'a>(
    edev: &'a ExpandedDevice<'a>,
    ndb: &'a NamingDb,
    dev_naming: &DeviceNaming,
) -> ExpandedNamedDevice<'a> {
    let egrid = &edev.egrid;
    let int_grid = make_grid(
        edev,
        |_, node, _| node == "INT",
        |_, _, _| false,
        (1, 1),
        (0, 0),
    );
    let bufdiv_grid = make_grid_complex(
        edev,
        |_, node, _| {
            matches!(
                node,
                "RCLK_CLE" | "RCLK_CLE.HALF" | "RCLK_INTF.W" | "RCLK_INTF.W.HALF"
            )
        },
        |_, node, _| matches!(node, "RCLK_INTF.E" | "RCLK_INTF.E.HALF"),
        |_, _, _, nloc| {
            let grid = edev.grids[nloc.0];
            let cd = &grid.columns[nloc.1];
            match cd.l {
                ColumnKind::Dsp => (0, 0),
                ColumnKind::Hard => {
                    let hc = grid.get_col_hard(nloc.1).unwrap();
                    let reg = grid.row_to_reg(nloc.2);
                    if matches!(
                        hc.regs[reg],
                        HardRowKind::DcmacT | HardRowKind::HscT | HardRowKind::IlknT
                    ) {
                        (1, 32)
                    } else {
                        (0, 0)
                    }
                }
                _ => (1, 32),
            }
        },
        |_, _, _, nloc| {
            let grid = edev.grids[nloc.0];
            let cd = &grid.columns[nloc.1];
            match cd.r {
                ColumnKind::Dsp => (1, 64),
                ColumnKind::Hard => {
                    let hc = grid.get_col_hard(nloc.1 + 1).unwrap();
                    let reg = grid.row_to_reg(nloc.2);
                    if matches!(
                        hc.regs[reg],
                        HardRowKind::DcmacT | HardRowKind::HscT | HardRowKind::IlknT
                    ) {
                        (1, 32)
                    } else {
                        (1, 64)
                    }
                }
                _ => (1, 32),
            }
        },
    );
    let iri_grid = make_grid_complex(
        edev,
        |_, node, _| {
            matches!(
                node,
                "INTF.W"
                    | "INTF.W.TERM.GT"
                    | "INTF.W.HDIO"
                    | "INTF.W.HB"
                    | "INTF.W.TERM.PSS"
                    | "INTF.W.PSS"
                    | "INTF.BLI_CLE.BOT.W.0"
                    | "INTF.BLI_CLE.TOP.W.0"
            )
        },
        |_, node, _| {
            matches!(
                node,
                "INTF.E"
                    | "INTF.E.TERM.GT"
                    | "INTF.E.HDIO"
                    | "INTF.E.HB"
                    | "INTF.BLI_CLE.BOT.E.0"
                    | "INTF.BLI_CLE.TOP.E.0"
            )
        },
        |_, node, _, _| {
            if node.starts_with("INTF.BLI_CLE") {
                (1, 16)
            } else {
                (1, 4)
            }
        },
        |_, node, _, _| {
            if node.starts_with("INTF.BLI_CLE") {
                (1, 16)
            } else {
                (1, 4)
            }
        },
    );
    let rclk_dfx_grid = make_grid_complex(
        edev,
        |_, node, _| node == "RCLK_DFX.W",
        |_, node, _| node == "RCLK_DFX.E",
        |_, _, _, _| (1, 1),
        |_, _, _, nloc| {
            let grid = edev.grids[nloc.0];
            if grid.columns[nloc.1].r == ColumnKind::Bram(BramKind::MaybeClkBufNoPd) {
                (2, 1)
            } else {
                (1, 1)
            }
        },
    );
    let slice_grid = make_grid(
        edev,
        |_, node, _| matches!(node, "CLE_L" | "CLE_L.VR"),
        |_, node, _| matches!(node, "CLE_R" | "CLE_R.VR"),
        (2, 1),
        (2, 1),
    );
    let dsp_grid = make_grid(
        edev,
        |_, _, _| false,
        |_, node, _| node == "DSP",
        (0, 0),
        (1, 1),
    );
    let bram_grid = make_grid(
        edev,
        |_, node, _| node == "BRAM_L",
        |_, node, _| node == "BRAM_R",
        (1, 1),
        (1, 1),
    );
    let uram_grid = make_grid(
        edev,
        |_, node, _| matches!(node, "URAM" | "URAM_DELAY"),
        |_, _, _| false,
        (1, 1),
        (0, 0),
    );
    let uram_delay_grid = make_grid(
        edev,
        |_, node, _| matches!(node, "URAM_DELAY"),
        |_, _, _| false,
        (1, 1),
        (0, 0),
    );
    let pcie4_grid = make_grid(
        edev,
        |_, node, _| node == "PCIE4",
        |_, _, _| false,
        (1, 1),
        (0, 0),
    );
    let pcie5_grid = make_grid(
        edev,
        |_, node, _| node == "PCIE5",
        |_, _, _| false,
        (1, 1),
        (0, 0),
    );
    let mrmac_grid = make_grid(
        edev,
        |_, node, _| node == "MRMAC",
        |_, _, _| false,
        (1, 1),
        (0, 0),
    );
    let sdfec_grid = make_grid(
        edev,
        |_, node, _| node == "SDFEC",
        |_, _, _| false,
        (1, 1),
        (0, 0),
    );
    let dfe_cfc_bot_grid = make_grid(
        edev,
        |_, node, _| node == "DFE_CFC_BOT",
        |_, _, _| false,
        (1, 1),
        (0, 0),
    );
    let dfe_cfc_top_grid = make_grid(
        edev,
        |_, node, _| node == "DFE_CFC_TOP",
        |_, _, _| false,
        (1, 1),
        (0, 0),
    );
    let dcmac_grid = make_grid(
        edev,
        |_, node, _| node == "DCMAC",
        |_, _, _| false,
        (1, 1),
        (0, 0),
    );
    let ilkn_grid = make_grid(
        edev,
        |_, node, _| node == "ILKN",
        |_, _, _| false,
        (1, 1),
        (0, 0),
    );
    let hsc_grid = make_grid(
        edev,
        |_, node, _| node == "HSC",
        |_, _, _| false,
        (1, 1),
        (0, 0),
    );
    let hdio_grid = make_grid(
        edev,
        |_, node, _| node == "HDIO",
        |_, _, _| false,
        (1, 1),
        (0, 0),
    );
    let vnoc_grid = make_grid(
        edev,
        |_, node, _| node == "VNOC",
        |_, _, _| false,
        (1, 1),
        (0, 0),
    );
    let misr_grid = make_grid(
        edev,
        |_, node, _| node == "MISR",
        |_, _, _| false,
        (1, 1),
        (0, 0),
    );
    let vdu_grid = make_grid(
        edev,
        |_, node, _| node == "VDU.E",
        |_, _, _| false,
        (1, 1),
        (0, 0),
    );
    let bfr_b_grid = make_grid(
        edev,
        |_, node, _| node == "BFR_B.E",
        |_, _, _| false,
        (1, 1),
        (0, 0),
    );

    let mut ngrid = ExpandedGridNaming::new(ndb, egrid);

    for die in egrid.dies() {
        let grid = edev.grids[die.die];
        for col in die.cols() {
            for row in die.rows() {
                let reg = grid.row_to_reg(row);
                for (layer, node) in &die[(col, row)].nodes {
                    let nloc = (die.die, col, row, layer);
                    let kind = egrid.db.nodes.key(node.kind);
                    match &kind[..] {
                        "INT" => {
                            ngrid.name_node(
                                nloc,
                                "INT",
                                [int_grid.name("INT", die.die, col, ColSide::Left, row, 0, 0)],
                            );
                        }
                        "RCLK" => {
                            let lr = if col < edev.col_cfrm[die.die] {
                                'L'
                            } else {
                                'R'
                            };
                            let vr = if grid.is_vr { "_VR" } else { "" };
                            ngrid.name_node(
                                nloc,
                                "RCLK",
                                [int_grid.name(
                                    &format!("RCLK_INT_{lr}{vr}_FT"),
                                    die.die,
                                    col,
                                    ColSide::Left,
                                    if reg.to_idx() % 2 == 1 { row - 1 } else { row },
                                    0,
                                    0,
                                )],
                            );
                        }
                        "CLE_BC" | "CLE_BC.SLL" | "CLE_BC.SLL2" => {
                            let tk = match &kind[..] {
                                "CLE_BC" => "CLE_BC_CORE",
                                "CLE_BC.SLL" => "SLL",
                                "CLE_BC.SLL2" => "SLL2",
                                _ => unreachable!(),
                            };
                            let bump_cur = col >= edev.col_cfrm[die.die]
                                && grid.cols_vbrk.contains(&(col + 1));
                            let bump_prev = col.to_idx() > 0
                                && matches!(grid.columns[col - 1].r, ColumnKind::Cle(_))
                                && (col - 1) >= edev.col_cfrm[die.die]
                                && grid.cols_vbrk.contains(&col);
                            ngrid.name_node(
                                nloc,
                                kind,
                                [int_grid.name(
                                    &if bump_prev && !bump_cur {
                                        format!("{tk}_1")
                                    } else {
                                        tk.to_string()
                                    },
                                    die.die,
                                    if bump_cur { col + 1 } else { col },
                                    ColSide::Left,
                                    row,
                                    0,
                                    0,
                                )],
                            );
                        }
                        "RCLK_CLE" | "RCLK_CLE.HALF" => {
                            let ColumnKind::Cle(cle_kind) = grid.columns[col].l else {
                                unreachable!()
                            };
                            let naming = &if cle_kind == CleKind::Plain {
                                if grid.is_vr {
                                    format!("{kind}.VR")
                                } else {
                                    kind.to_string()
                                }
                            } else {
                                format!("{kind}.LAG")
                            };
                            let nnode = ngrid.name_node(
                                nloc,
                                naming,
                                [int_grid.name(
                                    if cle_kind == CleKind::Plain {
                                        if grid.is_vr {
                                            "RCLK_CLE_VR_CORE"
                                        } else {
                                            "RCLK_CLE_CORE"
                                        }
                                    } else {
                                        "RCLK_CLE_LAG_CORE"
                                    },
                                    die.die,
                                    col - 1,
                                    ColSide::Left,
                                    if reg.to_idx() % 2 == 1 { row - 1 } else { row },
                                    0,
                                    0,
                                )],
                            );
                            let swz = if cle_kind == CleKind::Plain && !grid.is_vr {
                                BUFDIV_LEAF_SWZ_A
                            } else {
                                BUFDIV_LEAF_SWZ_B
                            };
                            for (i, dy) in swz.into_iter().enumerate() {
                                nnode.add_bel(
                                    i,
                                    bufdiv_grid.name(
                                        if grid.is_vr {
                                            "BUFDIV_LEAF_ULVT"
                                        } else {
                                            "BUFDIV_LEAF"
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        row,
                                        0,
                                        dy as i32,
                                    ),
                                );
                            }
                        }
                        "INTF.W" | "INTF.W.TERM.GT" | "INTF.W.HDIO" | "INTF.W.HB"
                        | "INTF.W.PSS" | "INTF.W.TERM.PSS" => {
                            let ocf = if col < edev.col_cfrm[die.die] {
                                "LOCF"
                            } else {
                                "ROCF"
                            };
                            let bt = if grid.is_reg_top(reg) { 'T' } else { 'B' };
                            let name = int_grid.name(
                                &match &kind[..] {
                                    "INTF.W" => format!("INTF_{ocf}_{bt}L_TILE"),
                                    "INTF.W.TERM.GT" => format!("INTF_GT_{bt}L_TILE"),
                                    "INTF.W.HDIO" => format!("INTF_HDIO_{ocf}_{bt}L_TILE"),
                                    "INTF.W.HB" => format!("INTF_HB_{ocf}_{bt}L_TILE"),
                                    "INTF.W.PSS" => format!("INTF_CFRM_{bt}L_TILE"),
                                    "INTF.W.TERM.PSS" => format!("INTF_PSS_{bt}L_TILE"),
                                    _ => unreachable!(),
                                },
                                die.die,
                                col,
                                ColSide::Left,
                                row,
                                0,
                                0,
                            );
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            for i in 0..4 {
                                nnode.iri_names.push(iri_grid.name(
                                    "IRI_QUAD",
                                    die.die,
                                    col,
                                    ColSide::Left,
                                    row,
                                    0,
                                    i,
                                ));
                            }
                        }
                        "INTF.E" | "INTF.E.TERM.GT" | "INTF.E.HDIO" | "INTF.E.HB" => {
                            let ocf = if col < edev.col_cfrm[die.die] {
                                "LOCF"
                            } else {
                                "ROCF"
                            };
                            let bt = if grid.is_reg_top(reg) { 'T' } else { 'B' };
                            let name = int_grid.name(
                                &match &kind[..] {
                                    "INTF.E" => format!("INTF_{ocf}_{bt}R_TILE"),
                                    "INTF.E.TERM.GT" => format!("INTF_GT_{bt}R_TILE"),
                                    "INTF.E.HDIO" => format!("INTF_HDIO_{ocf}_{bt}R_TILE"),
                                    "INTF.E.HB" => format!("INTF_HB_{ocf}_{bt}R_TILE"),
                                    _ => unreachable!(),
                                },
                                die.die,
                                col,
                                ColSide::Left,
                                row,
                                0,
                                0,
                            );
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            for i in 0..4 {
                                nnode.iri_names.push(iri_grid.name(
                                    "IRI_QUAD",
                                    die.die,
                                    col,
                                    ColSide::Right,
                                    row,
                                    0,
                                    i,
                                ));
                            }
                        }
                        "INTF.BLI_CLE.BOT.W.0"
                        | "INTF.BLI_CLE.BOT.W.1"
                        | "INTF.BLI_CLE.BOT.W.2"
                        | "INTF.BLI_CLE.BOT.W.3"
                        | "INTF.BLI_CLE.TOP.W.0"
                        | "INTF.BLI_CLE.TOP.W.1"
                        | "INTF.BLI_CLE.TOP.W.2"
                        | "INTF.BLI_CLE.TOP.W.3" => {
                            let (dy, srow) = match &kind[..] {
                                "INTF.BLI_CLE.BOT.W.0" => (12, row),
                                "INTF.BLI_CLE.BOT.W.1" => (8, row - 1),
                                "INTF.BLI_CLE.BOT.W.2" => (4, row - 2),
                                "INTF.BLI_CLE.BOT.W.3" => (0, row - 3),
                                "INTF.BLI_CLE.TOP.W.0" => (0, row),
                                "INTF.BLI_CLE.TOP.W.1" => (4, row - 1),
                                "INTF.BLI_CLE.TOP.W.2" => (8, row - 2),
                                "INTF.BLI_CLE.TOP.W.3" => (12, row - 3),
                                _ => unreachable!(),
                            };
                            let name = int_grid.name(
                                if kind.contains("BOT") {
                                    "BLI_CLE_BOT_CORE"
                                } else {
                                    "BLI_CLE_TOP_CORE"
                                },
                                die.die,
                                col,
                                ColSide::Left,
                                srow,
                                0,
                                0,
                            );
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            for i in 0..4 {
                                nnode.iri_names.push(iri_grid.name(
                                    "IRI_QUAD",
                                    die.die,
                                    col,
                                    ColSide::Left,
                                    srow,
                                    0,
                                    dy + i,
                                ));
                            }
                        }
                        "INTF.BLI_CLE.BOT.E.0"
                        | "INTF.BLI_CLE.BOT.E.1"
                        | "INTF.BLI_CLE.BOT.E.2"
                        | "INTF.BLI_CLE.BOT.E.3"
                        | "INTF.BLI_CLE.TOP.E.0"
                        | "INTF.BLI_CLE.TOP.E.1"
                        | "INTF.BLI_CLE.TOP.E.2"
                        | "INTF.BLI_CLE.TOP.E.3" => {
                            let (dy, srow) = match &kind[..] {
                                "INTF.BLI_CLE.BOT.E.0" => (12, row),
                                "INTF.BLI_CLE.BOT.E.1" => (8, row - 1),
                                "INTF.BLI_CLE.BOT.E.2" => (4, row - 2),
                                "INTF.BLI_CLE.BOT.E.3" => (0, row - 3),
                                "INTF.BLI_CLE.TOP.E.0" => (0, row),
                                "INTF.BLI_CLE.TOP.E.1" => (4, row - 1),
                                "INTF.BLI_CLE.TOP.E.2" => (8, row - 2),
                                "INTF.BLI_CLE.TOP.E.3" => (12, row - 3),
                                _ => unreachable!(),
                            };
                            let name = int_grid.name(
                                if kind.contains("BOT") {
                                    if matches!(grid.columns[col].l, ColumnKind::Cle(_))
                                        && grid.columns[col].has_bli_bot_l
                                    {
                                        "BLI_CLE_BOT_CORE_1"
                                    } else {
                                        "BLI_CLE_BOT_CORE"
                                    }
                                } else {
                                    if matches!(grid.columns[col].l, ColumnKind::Cle(_))
                                        && grid.columns[col].has_bli_top_l
                                    {
                                        "BLI_CLE_TOP_CORE_1"
                                    } else {
                                        "BLI_CLE_TOP_CORE"
                                    }
                                },
                                die.die,
                                col,
                                ColSide::Left,
                                srow,
                                0,
                                0,
                            );
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            for i in 0..4 {
                                nnode.iri_names.push(iri_grid.name(
                                    "IRI_QUAD",
                                    die.die,
                                    col,
                                    ColSide::Right,
                                    srow,
                                    0,
                                    dy + i,
                                ));
                            }
                        }
                        "RCLK_INTF.W" | "RCLK_INTF.W.HALF" => {
                            let srow = if reg.to_idx() % 2 == 1 { row - 1 } else { row };
                            let (subkind, name, swz, wide) = match grid.columns[col].l {
                                ColumnKind::Dsp => (
                                    if grid.is_vr { "DSP.VR" } else { "DSP" },
                                    int_grid.name(
                                        if grid.is_vr {
                                            "RCLK_DSP_VR_CORE"
                                        } else {
                                            "RCLK_DSP_CORE"
                                        },
                                        die.die,
                                        col - 1,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    if grid.is_vr {
                                        BUFDIV_LEAF_SWZ_BH
                                    } else {
                                        BUFDIV_LEAF_SWZ_AH
                                    },
                                    true,
                                ),
                                ColumnKind::Bram(BramKind::Plain) => (
                                    if grid.is_vr { "BRAM.VR" } else { "BRAM" },
                                    int_grid.name(
                                        if grid.is_vr {
                                            "RCLK_BRAM_VR_CORE"
                                        } else {
                                            "RCLK_BRAM_CORE"
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    if grid.is_vr {
                                        BUFDIV_LEAF_SWZ_B
                                    } else {
                                        BUFDIV_LEAF_SWZ_A
                                    },
                                    false,
                                ),
                                ColumnKind::Uram => (
                                    if grid.is_vr { "URAM.VR" } else { "URAM" },
                                    int_grid.name(
                                        if grid.is_vr {
                                            "RCLK_URAM_VR_CORE"
                                        } else {
                                            "RCLK_URAM_CORE"
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    if grid.is_vr {
                                        BUFDIV_LEAF_SWZ_B
                                    } else {
                                        BUFDIV_LEAF_SWZ_A
                                    },
                                    false,
                                ),
                                ColumnKind::Gt => (
                                    if grid.is_vr { "GT.VR" } else { "GT" },
                                    int_grid.name(
                                        if grid.is_vr {
                                            "RCLK_INTF_TERM_LEFT_VR_CORE"
                                        } else {
                                            "RCLK_INTF_TERM_LEFT_CORE"
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    if grid.is_vr {
                                        BUFDIV_LEAF_SWZ_B
                                    } else {
                                        BUFDIV_LEAF_SWZ_A
                                    },
                                    false,
                                ),
                                ColumnKind::Cfrm => (
                                    if grid.is_vr { "CFRM.VR" } else { "CFRM" },
                                    int_grid.name(
                                        if grid.is_vr {
                                            "RCLK_INTF_OPT_VR_CORE"
                                        } else {
                                            "RCLK_INTF_OPT_CORE"
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    if grid.is_vr {
                                        BUFDIV_LEAF_SWZ_B
                                    } else {
                                        BUFDIV_LEAF_SWZ_A
                                    },
                                    false,
                                ),
                                ColumnKind::VNoc | ColumnKind::VNoc2 | ColumnKind::VNoc4 => (
                                    if grid.is_vr { "VNOC.VR" } else { "VNOC" },
                                    int_grid.name(
                                        if grid.is_vr {
                                            "RCLK_INTF_L_VR_CORE"
                                        } else {
                                            "RCLK_INTF_L_CORE"
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    BUFDIV_LEAF_SWZ_B,
                                    false,
                                ),
                                ColumnKind::Hard => {
                                    let hc = grid.get_col_hard(col).unwrap();
                                    if reg.to_idx() % 2 == 0 {
                                        if hc.regs[reg] == HardRowKind::Hdio {
                                            (
                                                if grid.is_vr { "HDIO.VR" } else { "HDIO" },
                                                int_grid.name(
                                                    if grid.is_vr {
                                                        "RCLK_HDIO_VR_CORE"
                                                    } else {
                                                        "RCLK_HDIO_CORE"
                                                    },
                                                    die.die,
                                                    col - 1,
                                                    ColSide::Left,
                                                    srow,
                                                    0,
                                                    0,
                                                ),
                                                if grid.is_vr {
                                                    BUFDIV_LEAF_SWZ_BH
                                                } else {
                                                    BUFDIV_LEAF_SWZ_AH
                                                },
                                                true,
                                            )
                                        } else {
                                            (
                                                if grid.is_vr { "HB.VR" } else { "HB" },
                                                int_grid.name(
                                                    if grid.is_vr {
                                                        "RCLK_HB_VR_CORE"
                                                    } else {
                                                        "RCLK_HB_CORE"
                                                    },
                                                    die.die,
                                                    col - 1,
                                                    ColSide::Left,
                                                    srow,
                                                    0,
                                                    0,
                                                ),
                                                if grid.is_vr {
                                                    BUFDIV_LEAF_SWZ_BH
                                                } else {
                                                    BUFDIV_LEAF_SWZ_AH
                                                },
                                                true,
                                            )
                                        }
                                    } else {
                                        if hc.regs[reg] == HardRowKind::Hdio {
                                            (
                                                if grid.is_vr { "HDIO.VR" } else { "HDIO" },
                                                int_grid.name(
                                                    if grid.is_vr {
                                                        "RCLK_HDIO_VR_CORE"
                                                    } else {
                                                        "RCLK_HDIO_CORE"
                                                    },
                                                    die.die,
                                                    col - 1,
                                                    ColSide::Left,
                                                    srow,
                                                    0,
                                                    0,
                                                ),
                                                if grid.is_vr {
                                                    BUFDIV_LEAF_SWZ_BH
                                                } else {
                                                    BUFDIV_LEAF_SWZ_AH
                                                },
                                                true,
                                            )
                                        } else if hc.regs[reg - 1] == HardRowKind::Hdio {
                                            (
                                                if grid.is_vr { "HB_HDIO.VR" } else { "HB_HDIO" },
                                                int_grid.name(
                                                    if grid.is_vr {
                                                        "RCLK_HB_HDIO_VR_CORE"
                                                    } else {
                                                        "RCLK_HB_HDIO_CORE"
                                                    },
                                                    die.die,
                                                    col - 1,
                                                    ColSide::Left,
                                                    srow,
                                                    0,
                                                    0,
                                                ),
                                                BUFDIV_LEAF_SWZ_BH,
                                                true,
                                            )
                                        } else if hc.regs[reg - 1] == HardRowKind::DfeCfcB {
                                            (
                                                "SDFEC",
                                                int_grid.name(
                                                    "RCLK_SDFEC_CORE",
                                                    die.die,
                                                    col - 1,
                                                    ColSide::Left,
                                                    srow,
                                                    0,
                                                    0,
                                                ),
                                                BUFDIV_LEAF_SWZ_BH,
                                                true,
                                            )
                                        } else if matches!(
                                            hc.regs[reg],
                                            HardRowKind::DcmacT
                                                | HardRowKind::HscT
                                                | HardRowKind::IlknT
                                        ) {
                                            (
                                                if grid.is_vr { "HB_FULL.VR" } else { "HB_FULL" },
                                                int_grid.name(
                                                    if grid.is_vr {
                                                        "RCLK_HB_FULL_R_VR_CORE"
                                                    } else {
                                                        "RCLK_HB_FULL_R_CORE"
                                                    },
                                                    die.die,
                                                    col,
                                                    ColSide::Left,
                                                    srow,
                                                    0,
                                                    0,
                                                ),
                                                BUFDIV_LEAF_SWZ_B,
                                                false,
                                            )
                                        } else {
                                            (
                                                if grid.is_vr { "HB.VR" } else { "HB" },
                                                int_grid.name(
                                                    if grid.is_vr {
                                                        "RCLK_HB_VR_CORE"
                                                    } else {
                                                        "RCLK_HB_CORE"
                                                    },
                                                    die.die,
                                                    col - 1,
                                                    ColSide::Left,
                                                    srow,
                                                    0,
                                                    0,
                                                ),
                                                if grid.is_vr {
                                                    BUFDIV_LEAF_SWZ_BH
                                                } else {
                                                    BUFDIV_LEAF_SWZ_AH
                                                },
                                                true,
                                            )
                                        }
                                    }
                                }
                                _ => unreachable!(),
                            };
                            let nnode = ngrid.name_node(nloc, &format!("{kind}.{subkind}"), [name]);
                            for (i, dy) in swz.into_iter().enumerate() {
                                nnode.add_bel(
                                    i,
                                    if wide {
                                        bufdiv_grid.name(
                                            if grid.is_vr {
                                                "BUFDIV_LEAF_ULVT"
                                            } else {
                                                "BUFDIV_LEAF"
                                            },
                                            die.die,
                                            col - 1,
                                            ColSide::Right,
                                            row,
                                            0,
                                            dy as i32,
                                        )
                                    } else {
                                        bufdiv_grid.name(
                                            if grid.is_vr {
                                                "BUFDIV_LEAF_ULVT"
                                            } else {
                                                "BUFDIV_LEAF"
                                            },
                                            die.die,
                                            col,
                                            ColSide::Left,
                                            row,
                                            0,
                                            dy as i32,
                                        )
                                    },
                                );
                            }
                        }
                        "RCLK_INTF.E" | "RCLK_INTF.E.HALF" => {
                            let srow = if reg.to_idx() % 2 == 1 { row - 1 } else { row };
                            let (subkind, name, swz) = match grid.columns[col].r {
                                ColumnKind::Dsp => (
                                    if grid.is_vr { "DSP.VR" } else { "DSP" },
                                    int_grid.name(
                                        if grid.is_vr {
                                            "RCLK_DSP_VR_CORE"
                                        } else {
                                            "RCLK_DSP_CORE"
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    if grid.is_vr {
                                        BUFDIV_LEAF_SWZ_B
                                    } else {
                                        BUFDIV_LEAF_SWZ_A
                                    },
                                ),
                                ColumnKind::Bram(BramKind::Plain) => (
                                    if grid.is_vr { "BRAM.VR" } else { "BRAM" },
                                    int_grid.name(
                                        if grid.is_vr {
                                            "RCLK_BRAM_VR_CORE"
                                        } else {
                                            "RCLK_BRAM_CORE"
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    if grid.is_vr {
                                        BUFDIV_LEAF_SWZ_B
                                    } else {
                                        BUFDIV_LEAF_SWZ_A
                                    },
                                ),
                                ColumnKind::Bram(BramKind::ClkBuf) => (
                                    if grid.is_vr {
                                        "BRAM.CLKBUF.VR"
                                    } else {
                                        "BRAM.CLKBUF"
                                    },
                                    int_grid.name(
                                        if grid.is_vr {
                                            "RCLK_BRAM_CLKBUF_VR_CORE"
                                        } else {
                                            "RCLK_BRAM_CLKBUF_CORE"
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    if grid.is_vr {
                                        BUFDIV_LEAF_SWZ_B
                                    } else {
                                        BUFDIV_LEAF_SWZ_A
                                    },
                                ),
                                ColumnKind::Bram(BramKind::ClkBufNoPd) => (
                                    if grid.is_vr {
                                        "BRAM.CLKBUF.NOPD.VR"
                                    } else {
                                        "BRAM.CLKBUF.NOPD"
                                    },
                                    int_grid.name(
                                        if grid.is_vr {
                                            "RCLK_BRAM_CLKBUF_NOPD_VR_CORE"
                                        } else {
                                            "RCLK_BRAM_CLKBUF_NOPD_CORE"
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    BUFDIV_LEAF_SWZ_B,
                                ),
                                ColumnKind::Bram(BramKind::MaybeClkBufNoPd) => (
                                    if row.to_idx() < grid.get_ps_height() {
                                        if grid.is_vr {
                                            "BRAM.VR"
                                        } else {
                                            "BRAM"
                                        }
                                    } else {
                                        if grid.is_vr {
                                            "BRAM.CLKBUF.NOPD.VR"
                                        } else {
                                            "BRAM.CLKBUF.NOPD"
                                        }
                                    },
                                    int_grid.name(
                                        if row.to_idx() < grid.get_ps_height() {
                                            if grid.is_vr {
                                                "RCLK_BRAM_VR_CORE"
                                            } else {
                                                "RCLK_BRAM_CORE"
                                            }
                                        } else {
                                            if grid.is_vr {
                                                "RCLK_BRAM_CLKBUF_NOPD_VR_CORE"
                                            } else {
                                                "RCLK_BRAM_CLKBUF_NOPD_CORE"
                                            }
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    if grid.is_vr || row.to_idx() >= grid.get_ps_height() {
                                        BUFDIV_LEAF_SWZ_B
                                    } else {
                                        BUFDIV_LEAF_SWZ_A
                                    },
                                ),
                                ColumnKind::Uram => (
                                    if grid.is_vr { "URAM.VR" } else { "URAM" },
                                    int_grid.name(
                                        if grid.is_vr {
                                            "RCLK_URAM_VR_CORE"
                                        } else {
                                            "RCLK_URAM_CORE"
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    if grid.is_vr {
                                        BUFDIV_LEAF_SWZ_B
                                    } else {
                                        BUFDIV_LEAF_SWZ_A
                                    },
                                ),
                                ColumnKind::Gt => {
                                    if reg.to_idx() == 1 && matches!(grid.right, RightKind::Term2) {
                                        (
                                            if grid.is_vr { "GT.ALT.VR" } else { "GT.ALT" },
                                            int_grid.name(
                                                if grid.is_vr {
                                                    "RCLK_INTF_TERM2_RIGHT_VR_CORE"
                                                } else {
                                                    "RCLK_INTF_TERM2_RIGHT_CORE"
                                                },
                                                die.die,
                                                col,
                                                ColSide::Left,
                                                srow,
                                                0,
                                                0,
                                            ),
                                            BUFDIV_LEAF_SWZ_B,
                                        )
                                    } else {
                                        (
                                            if grid.is_vr { "GT.VR" } else { "GT" },
                                            int_grid.name(
                                                if grid.is_vr {
                                                    "RCLK_INTF_TERM_RIGHT_VR_CORE"
                                                } else {
                                                    "RCLK_INTF_TERM_RIGHT_CORE"
                                                },
                                                die.die,
                                                col,
                                                ColSide::Left,
                                                srow,
                                                0,
                                                0,
                                            ),
                                            if grid.is_vr {
                                                BUFDIV_LEAF_SWZ_B
                                            } else {
                                                BUFDIV_LEAF_SWZ_A
                                            },
                                        )
                                    }
                                }
                                ColumnKind::VNoc | ColumnKind::VNoc2 | ColumnKind::VNoc4 => (
                                    if grid.is_vr { "VNOC.VR" } else { "VNOC" },
                                    int_grid.name(
                                        if grid.is_vr {
                                            "RCLK_INTF_R_VR_CORE"
                                        } else {
                                            "RCLK_INTF_R_CORE"
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    BUFDIV_LEAF_SWZ_B,
                                ),
                                ColumnKind::Hard => {
                                    let hc = grid.get_col_hard(col + 1).unwrap();
                                    if reg.to_idx() % 2 == 0 {
                                        if hc.regs[reg] == HardRowKind::Hdio {
                                            (
                                                if grid.is_vr { "HDIO.VR" } else { "HDIO" },
                                                int_grid.name(
                                                    if grid.is_vr {
                                                        "RCLK_HDIO_VR_CORE"
                                                    } else {
                                                        "RCLK_HDIO_CORE"
                                                    },
                                                    die.die,
                                                    col,
                                                    ColSide::Left,
                                                    srow,
                                                    0,
                                                    0,
                                                ),
                                                if grid.is_vr {
                                                    BUFDIV_LEAF_SWZ_B
                                                } else {
                                                    BUFDIV_LEAF_SWZ_A
                                                },
                                            )
                                        } else {
                                            (
                                                if grid.is_vr { "HB.VR" } else { "HB" },
                                                int_grid.name(
                                                    if grid.is_vr {
                                                        "RCLK_HB_VR_CORE"
                                                    } else {
                                                        "RCLK_HB_CORE"
                                                    },
                                                    die.die,
                                                    col,
                                                    ColSide::Left,
                                                    srow,
                                                    0,
                                                    0,
                                                ),
                                                if grid.is_vr {
                                                    BUFDIV_LEAF_SWZ_B
                                                } else {
                                                    BUFDIV_LEAF_SWZ_A
                                                },
                                            )
                                        }
                                    } else {
                                        if hc.regs[reg] == HardRowKind::Hdio {
                                            (
                                                if grid.is_vr { "HDIO.VR" } else { "HDIO" },
                                                int_grid.name(
                                                    if grid.is_vr {
                                                        "RCLK_HDIO_VR_CORE"
                                                    } else {
                                                        "RCLK_HDIO_CORE"
                                                    },
                                                    die.die,
                                                    col,
                                                    ColSide::Left,
                                                    srow,
                                                    0,
                                                    0,
                                                ),
                                                if grid.is_vr {
                                                    BUFDIV_LEAF_SWZ_B
                                                } else {
                                                    BUFDIV_LEAF_SWZ_A
                                                },
                                            )
                                        } else if hc.regs[reg - 1] == HardRowKind::Hdio {
                                            (
                                                if grid.is_vr { "HB_HDIO.VR" } else { "HB_HDIO" },
                                                int_grid.name(
                                                    if grid.is_vr {
                                                        "RCLK_HB_HDIO_VR_CORE"
                                                    } else {
                                                        "RCLK_HB_HDIO_CORE"
                                                    },
                                                    die.die,
                                                    col,
                                                    ColSide::Left,
                                                    srow,
                                                    0,
                                                    0,
                                                ),
                                                BUFDIV_LEAF_SWZ_B,
                                            )
                                        } else if hc.regs[reg - 1] == HardRowKind::DfeCfcB {
                                            (
                                                "SDFEC",
                                                int_grid.name(
                                                    "RCLK_SDFEC_CORE",
                                                    die.die,
                                                    col,
                                                    ColSide::Left,
                                                    srow,
                                                    0,
                                                    0,
                                                ),
                                                BUFDIV_LEAF_SWZ_B,
                                            )
                                        } else if matches!(
                                            hc.regs[reg],
                                            HardRowKind::DcmacT
                                                | HardRowKind::HscT
                                                | HardRowKind::IlknT
                                        ) {
                                            (
                                                if grid.is_vr { "HB_FULL.VR" } else { "HB_FULL" },
                                                int_grid.name(
                                                    if grid.is_vr {
                                                        "RCLK_HB_FULL_L_VR_CORE"
                                                    } else {
                                                        "RCLK_HB_FULL_L_CORE"
                                                    },
                                                    die.die,
                                                    col,
                                                    ColSide::Left,
                                                    srow,
                                                    0,
                                                    0,
                                                ),
                                                BUFDIV_LEAF_SWZ_B,
                                            )
                                        } else {
                                            (
                                                if grid.is_vr { "HB.VR" } else { "HB" },
                                                int_grid.name(
                                                    if grid.is_vr {
                                                        "RCLK_HB_VR_CORE"
                                                    } else {
                                                        "RCLK_HB_CORE"
                                                    },
                                                    die.die,
                                                    col,
                                                    ColSide::Left,
                                                    srow,
                                                    0,
                                                    0,
                                                ),
                                                if grid.is_vr {
                                                    BUFDIV_LEAF_SWZ_B
                                                } else {
                                                    BUFDIV_LEAF_SWZ_A
                                                },
                                            )
                                        }
                                    }
                                }
                                _ => unreachable!(),
                            };
                            let nnode = ngrid.name_node(nloc, &format!("{kind}.{subkind}"), [name]);
                            for (i, dy) in swz.into_iter().enumerate() {
                                nnode.add_bel(
                                    i,
                                    bufdiv_grid.name(
                                        if grid.is_vr {
                                            "BUFDIV_LEAF_ULVT"
                                        } else {
                                            "BUFDIV_LEAF"
                                        },
                                        die.die,
                                        col,
                                        ColSide::Right,
                                        row,
                                        0,
                                        dy as i32,
                                    ),
                                );
                            }
                        }
                        "RCLK_DFX.W" => {
                            let srow = if reg.to_idx() % 2 == 1 { row - 1 } else { row };
                            let (subkind, name) = match grid.columns[col].l {
                                ColumnKind::Dsp => (
                                    "DSP",
                                    int_grid.name(
                                        if grid.is_vr {
                                            "RCLK_DSP_VR_CORE"
                                        } else {
                                            "RCLK_DSP_CORE"
                                        },
                                        die.die,
                                        col - 1,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                ),
                                ColumnKind::Bram(BramKind::Plain) => (
                                    "BRAM",
                                    int_grid.name(
                                        if grid.is_vr {
                                            "RCLK_BRAM_VR_CORE"
                                        } else {
                                            "RCLK_BRAM_CORE"
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                ),
                                ColumnKind::Uram => (
                                    "URAM",
                                    int_grid.name(
                                        if grid.is_vr {
                                            "RCLK_URAM_VR_CORE"
                                        } else {
                                            "RCLK_URAM_CORE"
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                ),
                                _ => unreachable!(),
                            };
                            let vr = if grid.is_vr { ".VR" } else { "" };
                            let nnode =
                                ngrid.name_node(nloc, &format!("{kind}.{subkind}{vr}"), [name]);
                            nnode.add_bel(
                                0,
                                rclk_dfx_grid.name("RCLK", die.die, col, ColSide::Left, row, 0, 0),
                            );
                        }
                        "RCLK_DFX.E" => {
                            let srow = if reg.to_idx() % 2 == 1 { row - 1 } else { row };
                            let (subkind, name, dx) = match grid.columns[col].r {
                                ColumnKind::Bram(BramKind::Plain) => (
                                    "BRAM",
                                    int_grid.name(
                                        if grid.is_vr {
                                            "RCLK_BRAM_VR_CORE"
                                        } else {
                                            "RCLK_BRAM_CORE"
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    0,
                                ),
                                ColumnKind::Bram(BramKind::ClkBuf) => (
                                    "BRAM.CLKBUF",
                                    int_grid.name(
                                        if grid.is_vr {
                                            "RCLK_BRAM_CLKBUF_VR_CORE"
                                        } else {
                                            "RCLK_BRAM_CLKBUF_CORE"
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    0,
                                ),
                                ColumnKind::Bram(BramKind::ClkBufNoPd) => (
                                    "BRAM.CLKBUF.NOPD",
                                    int_grid.name(
                                        if grid.is_vr {
                                            "RCLK_BRAM_CLKBUF_NOPD_VR_CORE"
                                        } else {
                                            "RCLK_BRAM_CLKBUF_NOPD_CORE"
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    0,
                                ),
                                ColumnKind::Bram(BramKind::MaybeClkBufNoPd) => (
                                    if row.to_idx() < grid.get_ps_height() {
                                        "BRAM"
                                    } else {
                                        "BRAM.CLKBUF.NOPD"
                                    },
                                    int_grid.name(
                                        if row.to_idx() < grid.get_ps_height() {
                                            if grid.is_vr {
                                                "RCLK_BRAM_VR_CORE"
                                            } else {
                                                "RCLK_BRAM_CORE"
                                            }
                                        } else {
                                            if grid.is_vr {
                                                "RCLK_BRAM_CLKBUF_NOPD_VR_CORE"
                                            } else {
                                                "RCLK_BRAM_CLKBUF_NOPD_CORE"
                                            }
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    if row.to_idx() < grid.get_ps_height() {
                                        0
                                    } else {
                                        1
                                    },
                                ),
                                ColumnKind::Uram => (
                                    "URAM",
                                    int_grid.name(
                                        if grid.is_vr {
                                            "RCLK_URAM_VR_CORE"
                                        } else {
                                            "RCLK_URAM_CORE"
                                        },
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    0,
                                ),
                                _ => unreachable!(),
                            };
                            let vr = if grid.is_vr { ".VR" } else { "" };
                            let nnode =
                                ngrid.name_node(nloc, &format!("{kind}.{subkind}{vr}"), [name]);
                            nnode.add_bel(
                                0,
                                rclk_dfx_grid.name(
                                    "RCLK",
                                    die.die,
                                    col,
                                    ColSide::Right,
                                    row,
                                    dx,
                                    0,
                                ),
                            );
                        }
                        "RCLK_HDIO" | "RCLK_HB_HDIO" => {
                            let srow = if reg.to_idx() % 2 == 1 { row - 1 } else { row };
                            let naming = if grid.is_vr {
                                format!("{kind}.VR")
                            } else {
                                kind.to_string()
                            };
                            let name = int_grid.name(
                                &if grid.is_vr {
                                    format!("{kind}_VR_CORE")
                                } else {
                                    format!("{kind}_CORE")
                                },
                                die.die,
                                col - 1,
                                ColSide::Left,
                                srow,
                                0,
                                0,
                            );
                            ngrid.name_node(nloc, &naming, [name]);
                        }
                        "CLE_L" | "CLE_L.VR" => {
                            let tkn = if !grid.is_vr {
                                "CLE_E_CORE"
                            } else {
                                "CLE_E_VR_CORE"
                            };
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [int_grid.name(tkn, die.die, col, ColSide::Left, row, 0, 0)],
                            );
                            for i in 0..2 {
                                nnode.add_bel(
                                    i,
                                    slice_grid.name(
                                        "SLICE",
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        row,
                                        i as i32,
                                        0,
                                    ),
                                );
                            }
                        }
                        "CLE_R" | "CLE_R.VR" => {
                            let tkn = if !grid.is_vr {
                                "CLE_W_CORE"
                            } else {
                                "CLE_W_VR_CORE"
                            };
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [int_grid.name(tkn, die.die, col, ColSide::Left, row, 0, 0)],
                            );
                            for i in 0..2 {
                                nnode.add_bel(
                                    i,
                                    slice_grid.name(
                                        "SLICE",
                                        die.die,
                                        col,
                                        ColSide::Right,
                                        row,
                                        i as i32,
                                        0,
                                    ),
                                );
                            }
                        }
                        "DSP" => {
                            let ocf = if col < edev.col_cfrm[die.die] {
                                "LOCF"
                            } else {
                                "ROCF"
                            };
                            let bt = if grid.is_reg_top(reg) { 'T' } else { 'B' };
                            let name = int_grid.name(
                                &format!("DSP_{ocf}_{bt}_TILE"),
                                die.die,
                                col,
                                ColSide::Left,
                                row,
                                0,
                                0,
                            );
                            let nnode = ngrid.name_node(
                                nloc,
                                if dev_naming.is_dsp_v2 {
                                    "DSP.V2"
                                } else {
                                    "DSP.V1"
                                },
                                [name],
                            );
                            for i in 0..2 {
                                nnode.add_bel(
                                    i,
                                    dsp_grid.name_mult(
                                        "DSP",
                                        die.die,
                                        col,
                                        ColSide::Right,
                                        row,
                                        2,
                                        i as i32,
                                        1,
                                        0,
                                    ),
                                );
                            }
                            nnode.add_bel(
                                2,
                                dsp_grid.name(
                                    "DSP58_CPLX",
                                    die.die,
                                    col,
                                    ColSide::Right,
                                    row,
                                    0,
                                    0,
                                ),
                            );
                        }
                        "BRAM_L" | "BRAM_R" => {
                            let lr = if kind == "BRAM_L" { 'L' } else { 'R' };
                            let ocf = if col < edev.col_cfrm[die.die] {
                                "LOCF"
                            } else {
                                "ROCF"
                            };
                            let bt = if grid.is_reg_top(reg) { 'T' } else { 'B' };
                            let name = int_grid.name(
                                &format!("BRAM_{ocf}_{bt}{lr}_TILE"),
                                die.die,
                                col,
                                ColSide::Left,
                                row,
                                0,
                                0,
                            );
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            nnode.add_bel(
                                0,
                                bram_grid.name(
                                    "RAMB36",
                                    die.die,
                                    col,
                                    if lr == 'L' {
                                        ColSide::Left
                                    } else {
                                        ColSide::Right
                                    },
                                    row,
                                    0,
                                    0,
                                ),
                            );
                            for i in 0..2 {
                                nnode.add_bel(
                                    1 + i,
                                    bram_grid.name_mult(
                                        "RAMB18",
                                        die.die,
                                        col,
                                        if lr == 'L' {
                                            ColSide::Left
                                        } else {
                                            ColSide::Right
                                        },
                                        row,
                                        1,
                                        0,
                                        2,
                                        i as i32,
                                    ),
                                );
                            }
                        }
                        "URAM" | "URAM_DELAY" => {
                            let ocf = if col < edev.col_cfrm[die.die] {
                                "LOCF"
                            } else {
                                "ROCF"
                            };
                            let bt = if grid.is_reg_top(reg) { 'T' } else { 'B' };
                            let name = int_grid.name(
                                &format!("{kind}_{ocf}_{bt}L_TILE"),
                                die.die,
                                col,
                                ColSide::Left,
                                row,
                                0,
                                0,
                            );
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            nnode.add_bel(
                                0,
                                uram_grid.name("URAM288", die.die, col, ColSide::Left, row, 0, 0),
                            );
                            if kind == "URAM_DELAY" {
                                nnode.add_bel(
                                    1,
                                    uram_delay_grid.name(
                                        "URAM_CAS_DLY",
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        row,
                                        0,
                                        0,
                                    ),
                                );
                            }
                        }
                        "PCIE4" | "PCIE5" | "MRMAC" | "SDFEC" | "DFE_CFC_BOT" | "DFE_CFC_TOP" => {
                            let (tk, bk, bel_grid) = match &kind[..] {
                                "PCIE4" => ("PCIEB", "PCIE40", &pcie4_grid),
                                "PCIE5" => ("PCIEB5", "PCIE50", &pcie5_grid),
                                "MRMAC" => ("MRMAC", "MRMAC", &mrmac_grid),
                                "SDFEC" => ("SDFECA", "SDFEC_A", &sdfec_grid),
                                "DFE_CFC_BOT" => ("DFE_CFC", "DFE_CFC_BOT", &dfe_cfc_bot_grid),
                                "DFE_CFC_TOP" => ("DFE_CFC", "DFE_CFC_TOP", &dfe_cfc_top_grid),
                                _ => unreachable!(),
                            };
                            let bt = if grid.is_reg_top(reg) { "TOP" } else { "BOT" };
                            let name = int_grid.name(
                                &format!("{tk}_{bt}_TILE"),
                                die.die,
                                col - 1,
                                ColSide::Left,
                                row,
                                0,
                                0,
                            );
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            nnode.add_bel(
                                0,
                                bel_grid.name(bk, die.die, col, ColSide::Left, row, 0, 0),
                            );
                        }
                        "DCMAC" | "ILKN" | "HSC" => {
                            let (bk, bel_grid) = match &kind[..] {
                                "DCMAC" => ("DCMAC", &dcmac_grid),
                                "ILKN" => ("ILKNF", &ilkn_grid),
                                "HSC" => ("HSC", &hsc_grid),
                                _ => unreachable!(),
                            };
                            let name = int_grid.name(
                                &format!("{kind}_TILE"),
                                die.die,
                                col - 1,
                                ColSide::Left,
                                row,
                                0,
                                0,
                            );
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            nnode.add_bel(
                                0,
                                bel_grid.name(bk, die.die, col, ColSide::Left, row, 0, 0),
                            );
                        }
                        "HDIO" => {
                            let name = int_grid.name(
                                if grid.is_reg_top(reg) {
                                    "HDIO_TILE"
                                } else {
                                    "HDIO_BOT_TILE"
                                },
                                die.die,
                                col - 1,
                                ColSide::Left,
                                row,
                                0,
                                0,
                            );
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let naming = &dev_naming.die[die.die].hdio[&(col, reg)];
                            for i in 0..11 {
                                nnode.add_bel(
                                    i,
                                    hdio_grid.name_mult(
                                        "HDIOLOGIC",
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        row,
                                        1,
                                        0,
                                        11,
                                        i as i32,
                                    ),
                                );
                            }
                            for i in 0..11 {
                                nnode.add_bel(
                                    11 + i,
                                    hdio_grid.name_manual(
                                        "IOB",
                                        die.die,
                                        naming.iob_xy.0,
                                        naming.iob_xy.1 + i as u32,
                                    ),
                                );
                            }
                            for i in 0..4 {
                                nnode.add_bel(
                                    22 + i,
                                    hdio_grid.name_mult(
                                        "BUFGCE_HDIO",
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        row,
                                        1,
                                        0,
                                        4,
                                        i as i32,
                                    ),
                                );
                            }
                            nnode.add_bel(
                                26,
                                hdio_grid.name_manual(
                                    "DPLL",
                                    die.die,
                                    naming.dpll_xy.0,
                                    naming.dpll_xy.1,
                                ),
                            );
                            nnode.add_bel(
                                27,
                                hdio_grid.name("HDIO_BIAS", die.die, col, ColSide::Left, row, 0, 0),
                            );
                            nnode.add_bel(
                                28,
                                hdio_grid.name(
                                    "RPI_HD_APB",
                                    die.die,
                                    col,
                                    ColSide::Left,
                                    row,
                                    0,
                                    0,
                                ),
                            );
                            nnode.add_bel(
                                29,
                                hdio_grid.name(
                                    "HDLOGIC_APB",
                                    die.die,
                                    col,
                                    ColSide::Left,
                                    row,
                                    0,
                                    0,
                                ),
                            );
                        }
                        "VNOC" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [
                                    int_grid.name(
                                        "NOC_NSU512_TOP",
                                        die.die,
                                        col - 1,
                                        ColSide::Left,
                                        row + 7,
                                        0,
                                        0,
                                    ),
                                    int_grid.name(
                                        "NOC_NPS_VNOC_TOP",
                                        die.die,
                                        col - 1,
                                        ColSide::Left,
                                        row + 15,
                                        0,
                                        0,
                                    ),
                                    int_grid.name(
                                        "NOC_NPS_VNOC_TOP",
                                        die.die,
                                        col - 1,
                                        ColSide::Left,
                                        row + 23,
                                        0,
                                        0,
                                    ),
                                    int_grid.name(
                                        "NOC_NMU512_TOP",
                                        die.die,
                                        col - 1,
                                        ColSide::Left,
                                        row + 31,
                                        0,
                                        0,
                                    ),
                                ],
                            );
                            nnode.add_bel(
                                0,
                                vnoc_grid.name(
                                    "NOC_NSU512",
                                    die.die,
                                    col,
                                    ColSide::Left,
                                    row,
                                    0,
                                    0,
                                ),
                            );
                            nnode.add_bel(
                                1,
                                vnoc_grid.name_mult(
                                    "NOC_NPS_VNOC",
                                    die.die,
                                    col,
                                    ColSide::Left,
                                    row,
                                    1,
                                    0,
                                    2,
                                    0,
                                ),
                            );
                            nnode.add_bel(
                                2,
                                vnoc_grid.name_mult(
                                    "NOC_NPS_VNOC",
                                    die.die,
                                    col,
                                    ColSide::Left,
                                    row,
                                    1,
                                    0,
                                    2,
                                    1,
                                ),
                            );
                            nnode.add_bel(
                                3,
                                vnoc_grid.name(
                                    "NOC_NMU512",
                                    die.die,
                                    col,
                                    ColSide::Left,
                                    row,
                                    0,
                                    0,
                                ),
                            );
                        }
                        "VNOC2" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [
                                    int_grid.name(
                                        "NOC2_NSU512_VNOC_TILE",
                                        die.die,
                                        col - 1,
                                        ColSide::Left,
                                        row + 7,
                                        0,
                                        0,
                                    ),
                                    int_grid.name(
                                        "NOC2_NPS5555_TOP",
                                        die.die,
                                        col - 1,
                                        ColSide::Left,
                                        row + 11,
                                        0,
                                        0,
                                    ),
                                    int_grid.name(
                                        "NOC2_NPS5555_TOP",
                                        die.die,
                                        col - 1,
                                        ColSide::Left,
                                        row + 14,
                                        0,
                                        0,
                                    ),
                                    int_grid.name(
                                        "NOC2_NMU512_VNOC_TILE",
                                        die.die,
                                        col - 1,
                                        ColSide::Left,
                                        row + 16,
                                        0,
                                        0,
                                    ),
                                    int_grid.name(
                                        "NOC2_SCAN_TOP",
                                        die.die,
                                        if dev_naming.is_vnoc2_scan_offset {
                                            col
                                        } else {
                                            col - 1
                                        },
                                        ColSide::Left,
                                        row + 7,
                                        0,
                                        0,
                                    ),
                                ],
                            );
                            let naming = &dev_naming.die[die.die].vnoc2[&(col, reg)];
                            nnode.add_bel(
                                0,
                                vnoc_grid.name_manual(
                                    "NOC2_NSU512",
                                    die.die,
                                    naming.nsu_xy.0,
                                    naming.nsu_xy.1,
                                ),
                            );
                            nnode.add_bel(
                                1,
                                vnoc_grid.name_manual(
                                    "NOC2_NPS5555",
                                    die.die,
                                    naming.nps_xy.0,
                                    naming.nps_xy.1,
                                ),
                            );
                            nnode.add_bel(
                                2,
                                vnoc_grid.name_manual(
                                    "NOC2_NPS5555",
                                    die.die,
                                    naming.nps_xy.0,
                                    naming.nps_xy.1 + 1,
                                ),
                            );
                            nnode.add_bel(
                                3,
                                vnoc_grid.name_manual(
                                    "NOC2_NMU512",
                                    die.die,
                                    naming.nmu_xy.0,
                                    naming.nmu_xy.1,
                                ),
                            );
                            nnode.add_bel(
                                4,
                                vnoc_grid.name_manual(
                                    "NOC2_SCAN",
                                    die.die,
                                    naming.scan_xy.0,
                                    naming.scan_xy.1,
                                ),
                            );
                        }
                        "VNOC4" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [
                                    int_grid.name(
                                        "NOC2_NSU512_VNOC4_TILE",
                                        die.die,
                                        col - 1,
                                        ColSide::Left,
                                        row + 7,
                                        0,
                                        0,
                                    ),
                                    int_grid.name(
                                        "NOC2_NPS6X_TOP",
                                        die.die,
                                        col - 1,
                                        ColSide::Left,
                                        row + 11,
                                        0,
                                        0,
                                    ),
                                    int_grid.name(
                                        "NOC2_NPS6X_TOP",
                                        die.die,
                                        col - 1,
                                        ColSide::Left,
                                        row + 14,
                                        0,
                                        0,
                                    ),
                                    int_grid.name(
                                        "NOC2_NMU512_VNOC4_TILE",
                                        die.die,
                                        col - 1,
                                        ColSide::Left,
                                        row + 16,
                                        0,
                                        0,
                                    ),
                                    int_grid.name(
                                        "NOC2_SCAN_TOP",
                                        die.die,
                                        col - 1,
                                        ColSide::Left,
                                        row + 7,
                                        0,
                                        0,
                                    ),
                                ],
                            );
                            let naming = &dev_naming.die[die.die].vnoc2[&(col, reg)];
                            nnode.add_bel(
                                0,
                                vnoc_grid.name_manual(
                                    "NOC2_NSU512",
                                    die.die,
                                    naming.nsu_xy.0,
                                    naming.nsu_xy.1,
                                ),
                            );
                            nnode.add_bel(
                                1,
                                vnoc_grid.name_manual(
                                    "NOC2_NPS6X",
                                    die.die,
                                    naming.nps_xy.0,
                                    naming.nps_xy.1,
                                ),
                            );
                            nnode.add_bel(
                                2,
                                vnoc_grid.name_manual(
                                    "NOC2_NPS6X",
                                    die.die,
                                    naming.nps_xy.0,
                                    naming.nps_xy.1 + 1,
                                ),
                            );
                            nnode.add_bel(
                                3,
                                vnoc_grid.name_manual(
                                    "NOC2_NMU512",
                                    die.die,
                                    naming.nmu_xy.0,
                                    naming.nmu_xy.1,
                                ),
                            );
                            nnode.add_bel(
                                4,
                                vnoc_grid.name_manual(
                                    "NOC2_SCAN",
                                    die.die,
                                    naming.scan_xy.0,
                                    naming.scan_xy.1,
                                ),
                            );
                        }
                        "MISR" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [int_grid.name(
                                    "MISR_TILE",
                                    die.die,
                                    col,
                                    ColSide::Left,
                                    if reg.to_idx() % 2 == 0 { row } else { row - 1 },
                                    0,
                                    0,
                                )],
                            );
                            nnode.add_bel(
                                0,
                                misr_grid.name("MISR", die.die, col, ColSide::Left, row, 0, 0),
                            );
                        }
                        "SYSMON_SAT.VNOC" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [int_grid.name(
                                    "AMS_SAT_VNOC_TILE",
                                    die.die,
                                    col - 1,
                                    ColSide::Left,
                                    row + 39,
                                    0,
                                    0,
                                )],
                            );
                            let (sx, sy) = dev_naming.die[die.die].sysmon_sat_vnoc[&(col, reg)];
                            nnode.add_bel(0, vnoc_grid.name_manual("SYSMON_SAT", die.die, sx, sy));
                        }
                        "SYSMON_SAT.LGT" | "SYSMON_SAT.RGT" => {
                            let bt = if grid.is_reg_top(reg) { "TOP" } else { "BOT" };
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [int_grid.name(
                                    &format!("AMS_SAT_GT_{bt}_TILE"),
                                    die.die,
                                    col,
                                    ColSide::Left,
                                    row + 19,
                                    0,
                                    0,
                                )],
                            );
                            let (sx, sy) = dev_naming.die[die.die].sysmon_sat_gt[&(col, reg)];
                            nnode.add_bel(0, vnoc_grid.name_manual("SYSMON_SAT", die.die, sx, sy));
                        }
                        "DPLL.LGT" | "DPLL.RGT" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [int_grid.name(
                                    "CMT_DPLL",
                                    die.die,
                                    col,
                                    ColSide::Left,
                                    row + 7,
                                    0,
                                    0,
                                )],
                            );
                            let (sx, sy) = dev_naming.die[die.die].dpll_gt[&(col, reg)];
                            nnode.add_bel(0, vnoc_grid.name_manual("DPLL", die.die, sx, sy));
                        }
                        "BFR_B.E" => {
                            let bt = if grid.is_reg_top(reg) { "TOP" } else { "BOT" };
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [int_grid.name(
                                    &format!("BFR_TILE_B_{bt}_CORE"),
                                    die.die,
                                    col,
                                    ColSide::Left,
                                    row,
                                    0,
                                    0,
                                )],
                            );
                            nnode.add_bel(
                                0,
                                bfr_b_grid.name("BFR_B", die.die, col, ColSide::Left, row, 0, 0),
                            );
                        }
                        "VDU.E" => {
                            let nnode =
                                ngrid.name_node(
                                    nloc,
                                    kind,
                                    [int_grid.name(
                                        "VDU_CORE",
                                        die.die,
                                        col,
                                        ColSide::Left,
                                        row,
                                        0,
                                        0,
                                    )],
                                );
                            nnode.add_bel(
                                0,
                                vdu_grid.name("VDU", die.die, col, ColSide::Left, row, 0, 0),
                            );
                        }

                        _ => panic!("how to {kind}"),
                    }
                }
            }
        }
    }

    ExpandedNamedDevice { edev, ngrid }
}
