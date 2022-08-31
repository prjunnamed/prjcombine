#![allow(clippy::collapsible_else_if)]

use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::db::{BelId, BelInfo, BelNaming, Dir, IntDb, NodeRawTileId};
use prjcombine_int::grid::{
    ColId, Coord, DieId, ExpandedDieRefMut, ExpandedGrid, ExpandedTileNode, RowId,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GridKind {
    Virtex2,
    Virtex2P,
    Virtex2PX,
    Spartan3,
    Spartan3E,
    Spartan3A,
    Spartan3ADsp,
}

impl GridKind {
    pub fn is_virtex2(self) -> bool {
        matches!(self, Self::Virtex2 | Self::Virtex2P | Self::Virtex2PX)
    }
    pub fn is_virtex2p(self) -> bool {
        matches!(self, Self::Virtex2P | Self::Virtex2PX)
    }
    pub fn is_spartan3a(self) -> bool {
        matches!(self, Self::Spartan3A | Self::Spartan3ADsp)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    Data(u8), // ×8
    CsiB,     // Called CS_B on Virtex 2 and Spartan 3.
    InitB,
    RdWrB,
    Dout,
    // Shared on Spartan 3E, Spartan 3A only; dedicated on Virtex 2, Spartan 3.
    M0,
    M1,
    M2,
    Cclk,
    HswapEn,
    // Spartan 3E, Spartan 3A only.
    CsoB,
    Ldc0,
    Ldc1,
    Ldc2,
    Hdc,
    Addr(u8), // ×20 on 3s100e, ×24 on other Spartan 3E, ×26 on Spartan 3A
    // Spartan 3A only.
    Awake,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub kind: GridKind,
    pub columns: EntityVec<ColId, Column>,
    pub col_clk: ColId,
    // For Spartan 3* other than 3s50a
    pub cols_clkv: Option<(ColId, ColId)>,
    // column -> (bottom bank, top bank)
    pub cols_gt: BTreeMap<ColId, (u32, u32)>,
    pub rows: EntityVec<RowId, RowIoKind>,
    // For Spartan 3E: range of rows containing RAMs
    pub rows_ram: Option<(RowId, RowId)>,
    // (hclk row, start row, end row)
    pub rows_hclk: Vec<(RowId, RowId, RowId)>,
    // For Virtex 2
    pub row_pci: Option<RowId>,
    pub holes_ppc: Vec<(ColId, RowId)>,
    // For Spartan 3E, 3A*
    pub dcms: Option<Dcms>,
    pub has_ll: bool,
    pub has_small_int: bool,
    pub vref: BTreeSet<IoCoord>,
    pub cfg_io: BTreeMap<SharedCfgPin, IoCoord>,
    pub dci_io: BTreeMap<u32, (IoCoord, IoCoord)>,
    pub dci_io_alt: BTreeMap<u32, (IoCoord, IoCoord)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub kind: ColumnKind,
    pub io: ColumnIoKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKind {
    Io,
    Clb,
    Bram,
    BramCont(u8),
    Dsp,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnIoKind {
    None,
    Single,
    Double(u8),
    Triple(u8),
    Quad(u8),
    SingleLeft,
    SingleRight,
    SingleLeftAlt,
    SingleRightAlt,
    DoubleLeft(u8),
    DoubleRight(u8),
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RowIoKind {
    None,
    Single,
    Double(u8),
    Triple(u8),
    Quad(u8),
    DoubleBot(u8),
    DoubleTop(u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Dcms {
    Two,
    Four,
    Eight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IoDiffKind {
    P(BelId),
    N(BelId),
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IoPadKind {
    None,
    Input,
    Io,
    Clk,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Edge {
    Top,
    Bot,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct IoCoord {
    pub col: ColId,
    pub row: RowId,
    pub bel: BelId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtPin {
    RxP,
    RxN,
    TxP,
    TxN,
    GndA,
    VtRx,
    VtTx,
    AVccAuxRx,
    AVccAuxTx,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CfgPin {
    Tck,
    Tdi,
    Tdo,
    Tms,
    Cclk,
    Done,
    ProgB,
    M0,
    M1,
    M2,
    HswapEn,
    PwrdwnB,
    Suspend,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BondPin {
    Io(IoCoord),
    Gt(u32, GtPin),
    Nc,
    Rsvd,
    Gnd,
    VccInt,
    VccAux,
    VccO(u32),
    VccBatt,
    Cfg(CfgPin),
    Dxn,
    Dxp,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
    // device bank -> pkg bank
    pub io_banks: BTreeMap<u32, u32>,
}

#[derive(Clone, Copy, Debug)]
pub struct Io<'a> {
    pub coord: IoCoord,
    pub bank: u32,
    pub diff: IoDiffKind,
    pub pad_kind: IoPadKind,
    pub name: &'a str,
    pub is_vref: bool,
}

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub egrid: ExpandedGrid<'a>,
    pub bonded_ios: Vec<((ColId, RowId), BelId)>,
}

impl Grid {
    pub fn col_left(&self) -> ColId {
        self.columns.first_id().unwrap()
    }

    pub fn col_right(&self) -> ColId {
        self.columns.last_id().unwrap()
    }

    pub fn row_bot(&self) -> RowId {
        self.rows.first_id().unwrap()
    }

    pub fn row_top(&self) -> RowId {
        self.rows.last_id().unwrap()
    }

    pub fn row_mid(&self) -> RowId {
        RowId::from_idx(self.rows.len() / 2)
    }

    pub fn get_clk_io(&self, edge: Edge, idx: usize) -> Option<(Coord, BelId)> {
        if self.kind.is_virtex2() {
            match edge {
                Edge::Bot => {
                    if self.kind == GridKind::Virtex2PX && matches!(idx, 6 | 7) {
                        return None;
                    }
                    if idx < 4 {
                        Some(((self.col_clk, self.row_bot()), BelId::from_idx(idx)))
                    } else if idx < 8 {
                        Some(((self.col_clk - 1, self.row_bot()), BelId::from_idx(idx - 4)))
                    } else {
                        None
                    }
                }
                Edge::Top => {
                    if self.kind == GridKind::Virtex2PX && matches!(idx, 4 | 5) {
                        return None;
                    }
                    if idx < 4 {
                        Some(((self.col_clk, self.row_top()), BelId::from_idx(idx)))
                    } else if idx < 8 {
                        Some(((self.col_clk - 1, self.row_top()), BelId::from_idx(idx - 4)))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else if self.kind == GridKind::Spartan3 {
            match edge {
                Edge::Bot => {
                    if idx < 2 {
                        Some(((self.col_clk, self.row_bot()), BelId::from_idx(idx)))
                    } else if idx < 4 {
                        Some(((self.col_clk - 1, self.row_bot()), BelId::from_idx(idx - 2)))
                    } else {
                        None
                    }
                }
                Edge::Top => {
                    if idx < 2 {
                        Some(((self.col_clk, self.row_top()), BelId::from_idx(idx)))
                    } else if idx < 4 {
                        Some(((self.col_clk - 1, self.row_top()), BelId::from_idx(idx - 2)))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else if self.kind == GridKind::Spartan3E {
            match (edge, idx) {
                (Edge::Bot, 0 | 1) => {
                    Some(((self.col_clk, self.row_bot()), BelId::from_idx(idx % 2)))
                }
                (Edge::Bot, 2 | 3) => {
                    Some(((self.col_clk + 1, self.row_bot()), BelId::from_idx(idx % 2)))
                }
                (Edge::Bot, 4 | 5) => {
                    Some(((self.col_clk - 3, self.row_bot()), BelId::from_idx(idx % 2)))
                }
                (Edge::Bot, 6 | 7) => {
                    Some(((self.col_clk - 1, self.row_bot()), BelId::from_idx(idx % 2)))
                }
                (Edge::Top, 0 | 1) => {
                    Some(((self.col_clk + 2, self.row_top()), BelId::from_idx(idx % 2)))
                }
                (Edge::Top, 2 | 3) => {
                    Some(((self.col_clk, self.row_top()), BelId::from_idx(idx % 2)))
                }
                (Edge::Top, 4 | 5) => {
                    Some(((self.col_clk - 1, self.row_top()), BelId::from_idx(idx % 2)))
                }
                (Edge::Top, 6 | 7) => {
                    Some(((self.col_clk - 2, self.row_top()), BelId::from_idx(idx % 2)))
                }
                (Edge::Left, 0 | 1) => Some((
                    (self.col_left(), self.row_mid() + 3),
                    BelId::from_idx(idx % 2),
                )),
                (Edge::Left, 2 | 3) => Some((
                    (self.col_left(), self.row_mid() + 1),
                    BelId::from_idx(idx % 2),
                )),
                (Edge::Left, 4 | 5) => Some((
                    (self.col_left(), self.row_mid() - 1),
                    BelId::from_idx(idx % 2),
                )),
                (Edge::Left, 6 | 7) => Some((
                    (self.col_left(), self.row_mid() - 3),
                    BelId::from_idx(idx % 2),
                )),
                (Edge::Right, 0 | 1) => {
                    Some(((self.col_right(), self.row_mid()), BelId::from_idx(idx % 2)))
                }
                (Edge::Right, 2 | 3) => Some((
                    (self.col_right(), self.row_mid() + 2),
                    BelId::from_idx(idx % 2),
                )),
                (Edge::Right, 4 | 5) => Some((
                    (self.col_right(), self.row_mid() - 4),
                    BelId::from_idx(idx % 2),
                )),
                (Edge::Right, 6 | 7) => Some((
                    (self.col_right(), self.row_mid() - 2),
                    BelId::from_idx(idx % 2),
                )),
                _ => None,
            }
        } else {
            match (edge, idx) {
                (Edge::Bot, 0 | 1) => {
                    Some(((self.col_clk, self.row_bot()), BelId::from_idx(idx % 2)))
                }
                (Edge::Bot, 2 | 3) => {
                    Some(((self.col_clk + 1, self.row_bot()), BelId::from_idx(idx % 2)))
                }
                (Edge::Bot, 4 | 5) => {
                    Some(((self.col_clk - 2, self.row_bot()), BelId::from_idx(idx % 2)))
                }
                (Edge::Bot, 6 | 7) => {
                    Some(((self.col_clk - 1, self.row_bot()), BelId::from_idx(idx % 2)))
                }
                (Edge::Top, 0 | 1) => {
                    Some(((self.col_clk + 1, self.row_top()), BelId::from_idx(idx % 2)))
                }
                (Edge::Top, 2 | 3) => {
                    Some(((self.col_clk, self.row_top()), BelId::from_idx(idx % 2)))
                }
                (Edge::Top, 4 | 5) => {
                    Some(((self.col_clk - 1, self.row_top()), BelId::from_idx(idx % 2)))
                }
                (Edge::Top, 6 | 7) => {
                    Some(((self.col_clk - 2, self.row_top()), BelId::from_idx(idx % 2)))
                }
                (Edge::Left, 0 | 1) => Some((
                    (self.col_left(), self.row_mid() + 2),
                    BelId::from_idx((idx % 2) ^ 1),
                )),
                (Edge::Left, 2 | 3) => Some((
                    (self.col_left(), self.row_mid() + 1),
                    BelId::from_idx((idx % 2) ^ 1),
                )),
                (Edge::Left, 4 | 5) => Some((
                    (self.col_left(), self.row_mid() - 1),
                    BelId::from_idx((idx % 2) ^ 1),
                )),
                (Edge::Left, 6 | 7) => Some((
                    (self.col_left(), self.row_mid() - 2),
                    BelId::from_idx((idx % 2) ^ 1),
                )),
                (Edge::Right, 0 | 1) => {
                    Some(((self.col_right(), self.row_mid()), BelId::from_idx(idx % 2)))
                }
                (Edge::Right, 2 | 3) => Some((
                    (self.col_right(), self.row_mid() + 1),
                    BelId::from_idx(idx % 2),
                )),
                (Edge::Right, 4 | 5) => Some((
                    (self.col_right(), self.row_mid() - 3),
                    BelId::from_idx(idx % 2),
                )),
                (Edge::Right, 6 | 7) => Some((
                    (self.col_right(), self.row_mid() - 2),
                    BelId::from_idx(idx % 2),
                )),
                _ => None,
            }
        }
    }

    pub fn get_pci_io(&self, edge: Edge) -> [(Coord, BelId); 2] {
        match self.kind {
            GridKind::Spartan3E => match edge {
                Edge::Left => [
                    ((self.col_left(), self.row_mid() + 1), BelId::from_idx(1)),
                    ((self.col_left(), self.row_mid() - 1), BelId::from_idx(0)),
                ],
                Edge::Right => [
                    ((self.col_right(), self.row_mid()), BelId::from_idx(0)),
                    ((self.col_right(), self.row_mid() - 2), BelId::from_idx(1)),
                ],
                _ => unreachable!(),
            },
            GridKind::Spartan3A | GridKind::Spartan3ADsp => match edge {
                Edge::Left => [
                    ((self.col_left(), self.row_mid() + 1), BelId::from_idx(0)),
                    ((self.col_left(), self.row_mid() - 2), BelId::from_idx(1)),
                ],
                Edge::Right => [
                    ((self.col_right(), self.row_mid() + 1), BelId::from_idx(0)),
                    ((self.col_right(), self.row_mid() - 2), BelId::from_idx(1)),
                ],
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    fn fill_term(
        &self,
        die: &mut ExpandedDieRefMut,
        coord: Coord,
        kind: &str,
        naming: &str,
        name: String,
    ) {
        if self.kind.is_virtex2() {
            let kind = die.grid.db.get_node(kind);
            let naming = die.grid.db.get_node_naming(naming);
            die[coord].add_xnode(kind, &[&name], naming, &[coord]);
        }
        die.fill_term_tile(coord, kind, naming, name);
    }

    pub fn expand_grid<'a>(&'a self, db: &'a IntDb) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        egrid.tie_kind = Some("VCC".to_string());
        egrid.tie_pin_pullup = Some("VCCOUT".to_string());

        let (_, mut grid) = egrid.add_die(self.columns.len(), self.rows.len());
        let def_rt = NodeRawTileId::from_idx(0);

        let use_xy = matches!(
            self.kind,
            GridKind::Spartan3E | GridKind::Spartan3A | GridKind::Spartan3ADsp
        );
        let mut rows_brk: BTreeSet<_> = self.rows_hclk.iter().map(|&(_, _, r)| r - 1).collect();
        rows_brk.remove(&self.row_top());
        if self.kind != GridKind::Spartan3ADsp {
            rows_brk.remove(&(self.row_mid() - 1));
        }
        let mut xtmp = 0;
        let xlut = self.columns.map_values(|cd| {
            let res = xtmp;
            if cd.kind == ColumnKind::Dsp {
                xtmp += 2;
            } else {
                xtmp += 1;
            }
            res
        });
        xtmp = 0;
        let clut = self.columns.map_values(|cd| {
            let res = xtmp;
            if cd.kind != ColumnKind::Bram {
                xtmp += 1;
            }
            res
        });
        xtmp = 1;
        let bramclut = self.columns.map_values(|cd| {
            let res = xtmp;
            if cd.kind == ColumnKind::Bram {
                xtmp += 1;
            }
            res
        });

        let cnr_kind = if self.kind.is_virtex2() {
            "INT.CNR"
        } else {
            "INT.CLB"
        };
        let col_l = self.col_left();
        let col_r = self.col_right();
        let row_b = self.row_bot();
        let row_t = self.row_top();
        let xl = xlut[col_l];
        let xr = xlut[col_r];
        let yb = row_b.to_idx();
        let yt = row_t.to_idx();
        if use_xy {
            grid.fill_tile(
                (col_l, row_b),
                cnr_kind,
                "INT.CNR",
                format!("LL_X{xl}Y{yb}"),
            );
            grid.fill_tile(
                (col_r, row_b),
                cnr_kind,
                "INT.CNR",
                format!("LR_X{xr}Y{yb}"),
            );
            grid.fill_tile(
                (col_l, row_t),
                cnr_kind,
                "INT.CNR",
                format!("UL_X{xl}Y{yt}"),
            );
            grid.fill_tile(
                (col_r, row_t),
                cnr_kind,
                "INT.CNR",
                format!("UR_X{xr}Y{yt}"),
            );
            self.fill_term(
                &mut grid,
                (col_l, row_b),
                "TERM.W",
                "TERM.W",
                format!("CNR_LBTERM_X{xl}Y{yb}"),
            );
            self.fill_term(
                &mut grid,
                (col_l, row_t),
                "TERM.W",
                "TERM.W",
                format!("CNR_LTTERM_X{xl}Y{yt}"),
            );
            self.fill_term(
                &mut grid,
                (col_r, row_b),
                "TERM.E",
                "TERM.E",
                format!("CNR_RBTERM_X{xr}Y{yb}"),
            );
            self.fill_term(
                &mut grid,
                (col_r, row_t),
                "TERM.E",
                "TERM.E",
                format!("CNR_RTTERM_X{xr}Y{yt}"),
            );
            self.fill_term(
                &mut grid,
                (col_l, row_b),
                "TERM.S",
                "TERM.S.CNR",
                format!("CNR_BTERM_X{xl}Y{yb}"),
            );
            self.fill_term(
                &mut grid,
                (col_l, row_t),
                "TERM.N",
                "TERM.N.CNR",
                format!("CNR_TTERM_X{xl}Y{yt}"),
            );
            self.fill_term(
                &mut grid,
                (col_r, row_b),
                "TERM.S",
                "TERM.S.CNR",
                format!("CNR_BTERM_X{xr}Y{yb}"),
            );
            self.fill_term(
                &mut grid,
                (col_r, row_t),
                "TERM.N",
                "TERM.N.CNR",
                format!("CNR_TTERM_X{xr}Y{yt}"),
            );
        } else if self.kind.is_virtex2p() {
            grid.fill_tile((col_l, row_b), cnr_kind, "INT.CNR", "LIOIBIOI".to_string());
            grid.fill_tile((col_r, row_b), cnr_kind, "INT.CNR", "RIOIBIOI".to_string());
            grid.fill_tile((col_l, row_t), cnr_kind, "INT.CNR", "LIOITIOI".to_string());
            grid.fill_tile((col_r, row_t), cnr_kind, "INT.CNR", "RIOITIOI".to_string());
            self.fill_term(
                &mut grid,
                (col_l, row_b),
                "TERM.W",
                "TERM.W",
                "LTERMBIOI".to_string(),
            );
            self.fill_term(
                &mut grid,
                (col_l, row_t),
                "TERM.W",
                "TERM.W",
                "LTERMTIOI".to_string(),
            );
            self.fill_term(
                &mut grid,
                (col_r, row_b),
                "TERM.E",
                "TERM.E",
                "RTERMBIOI".to_string(),
            );
            self.fill_term(
                &mut grid,
                (col_r, row_t),
                "TERM.E",
                "TERM.E",
                "RTERMTIOI".to_string(),
            );
            self.fill_term(
                &mut grid,
                (col_l, row_b),
                "TERM.S",
                "TERM.S.CNR",
                "LIOIBTERM".to_string(),
            );
            self.fill_term(
                &mut grid,
                (col_l, row_t),
                "TERM.N",
                "TERM.N.CNR",
                "LIOITTERM".to_string(),
            );
            self.fill_term(
                &mut grid,
                (col_r, row_b),
                "TERM.S",
                "TERM.S.CNR",
                "RIOIBTERM".to_string(),
            );
            self.fill_term(
                &mut grid,
                (col_r, row_t),
                "TERM.N",
                "TERM.N.CNR",
                "RIOITTERM".to_string(),
            );
        } else {
            grid.fill_tile((col_l, row_b), cnr_kind, "INT.CNR", "BL".to_string());
            grid.fill_tile((col_r, row_b), cnr_kind, "INT.CNR", "BR".to_string());
            grid.fill_tile((col_l, row_t), cnr_kind, "INT.CNR", "TL".to_string());
            grid.fill_tile((col_r, row_t), cnr_kind, "INT.CNR", "TR".to_string());
            self.fill_term(
                &mut grid,
                (col_l, row_b),
                "TERM.W",
                "TERM.W",
                "LBTERM".to_string(),
            );
            self.fill_term(
                &mut grid,
                (col_l, row_t),
                "TERM.W",
                "TERM.W",
                "LTTERM".to_string(),
            );
            self.fill_term(
                &mut grid,
                (col_r, row_b),
                "TERM.E",
                "TERM.E",
                "RBTERM".to_string(),
            );
            self.fill_term(
                &mut grid,
                (col_r, row_t),
                "TERM.E",
                "TERM.E",
                "RTTERM".to_string(),
            );
            self.fill_term(
                &mut grid,
                (col_l, row_b),
                "TERM.S",
                "TERM.S.CNR",
                "BLTERM".to_string(),
            );
            self.fill_term(
                &mut grid,
                (col_l, row_t),
                "TERM.N",
                "TERM.N.CNR",
                "TLTERM".to_string(),
            );
            self.fill_term(
                &mut grid,
                (col_r, row_b),
                "TERM.S",
                "TERM.S.CNR",
                "BRTERM".to_string(),
            );
            self.fill_term(
                &mut grid,
                (col_r, row_t),
                "TERM.N",
                "TERM.N.CNR",
                "TRTERM".to_string(),
            );
        }

        if !use_xy {
            for (c, b0, b1) in [
                ((col_l, row_b), 6, 5),
                ((col_r, row_b), 3, 4),
                ((col_l, row_t), 7, 0),
                ((col_r, row_t), 2, 1),
            ] {
                let tile = &mut grid[c];
                let name = tile.nodes[0].names[NodeRawTileId::from_idx(0)].clone();
                let kind = if self.kind == GridKind::Spartan3 && b0 == 2 {
                    "DCI.UR"
                } else {
                    "DCI"
                };
                let node =
                    tile.add_xnode(db.get_node(kind), &[&name], db.get_node_naming(kind), &[c]);
                node.add_bel(0, format!("DCI{b0}"));
                node.add_bel(1, format!("DCI{b1}"));
                if self.kind == GridKind::Spartan3 {
                    node.add_bel(2, format!("DCIRESET{b0}"));
                    node.add_bel(3, format!("DCIRESET{b1}"));
                }
            }
        } else {
            for c in [
                (col_l, row_b),
                (col_r, row_b),
                (col_l, row_t),
                (col_r, row_t),
            ] {
                let tile = &mut grid[c];
                let name = tile.nodes[0].names[NodeRawTileId::from_idx(0)].clone();
                tile.add_xnode(
                    db.get_node("PCI_CE_CNR"),
                    &[&name],
                    db.get_node_naming("PCI_CE_CNR"),
                    &[c],
                );
            }
        }

        {
            let c = (col_r, row_b);
            let tile = &mut grid[c];
            let name = tile.nodes[0].names[NodeRawTileId::from_idx(0)].clone();
            let kind = match self.kind {
                GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => "LR",
                GridKind::Spartan3 => "LR.S3",
                GridKind::Spartan3E => "LR.S3E",
                GridKind::Spartan3A | GridKind::Spartan3ADsp => "LR.S3A",
            };
            let node = tile.add_xnode(db.get_node(kind), &[&name], db.get_node_naming(kind), &[c]);
            node.add_bel(0, "STARTUP".to_string());
            node.add_bel(1, "CAPTURE".to_string());
            node.add_bel(2, "ICAP".to_string());
            if self.kind.is_spartan3a() {
                node.add_bel(3, "SPI_ACCESS".to_string());
            }
        }

        {
            let c = (col_l, row_t);
            let tile = &mut grid[c];
            let name = tile.nodes[0].names[NodeRawTileId::from_idx(0)].clone();
            let node = tile.add_xnode(
                db.get_node("PMV"),
                &[&name],
                db.get_node_naming("PMV"),
                &[c],
            );
            node.add_bel(0, "PMV".to_string());
            if self.kind.is_spartan3a() {
                let node = tile.add_xnode(
                    db.get_node("DNA_PORT"),
                    &[&name],
                    db.get_node_naming("DNA_PORT"),
                    &[c],
                );
                node.add_bel(0, "DNA_PORT".to_string());
            }
        }

        {
            let c = (col_r, row_t);
            let tile = &mut grid[c];
            let name = tile.nodes[0].names[NodeRawTileId::from_idx(0)].clone();
            let kind = match self.kind {
                GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => "BSCAN",
                GridKind::Spartan3 | GridKind::Spartan3E => "UR.S3",
                GridKind::Spartan3A | GridKind::Spartan3ADsp => "UR.S3A",
            };
            let node = tile.add_xnode(db.get_node(kind), &[&name], db.get_node_naming(kind), &[c]);
            node.add_bel(0, "BSCAN".to_string());
            if self.kind.is_virtex2p() {
                let node = tile.add_xnode(
                    db.get_node("JTAGPPC"),
                    &[&name],
                    db.get_node_naming("JTAGPPC"),
                    &[c],
                );
                node.add_bel(0, "JTAGPPC".to_string());
            }
        }

        let mut bonded_ios = vec![];
        {
            let mut ctr_pad = 1;
            let mut ctr_nopad = if use_xy { 0 } else { 1 };
            for (col, &cd) in &self.columns {
                let row = row_t;
                if use_xy {
                    if cd.kind == ColumnKind::Io {
                        continue;
                    }
                } else {
                    if cd.kind != ColumnKind::Clb {
                        continue;
                    }
                }
                let pads: &[usize];
                let ipads: &[usize];
                let mut int_kind;
                let mut int_naming;
                let mut ioi_kind;
                let mut ioi_naming;
                let iobs_kind;
                let iobs: &[usize];
                let mut term = "";
                let mut kind = "";
                match self.kind {
                    GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
                        (pads, iobs_kind) = match cd.io {
                            ColumnIoKind::None => (&[][..], None),
                            ColumnIoKind::SingleLeft | ColumnIoKind::SingleLeftAlt => {
                                (&[2, 1, 0][..], Some(("IOBS.T.L1", 1)))
                            }
                            ColumnIoKind::SingleRight | ColumnIoKind::SingleRightAlt => {
                                (&[3, 2, 1][..], Some(("IOBS.T.R1", 1)))
                            }
                            ColumnIoKind::DoubleLeft(0) => {
                                (&[3, 2, 1, 0][..], Some(("IOBS.T.L2", 2)))
                            }
                            ColumnIoKind::DoubleLeft(1) => (&[1, 0][..], None),
                            ColumnIoKind::DoubleRight(0) => (&[3, 2][..], Some(("IOBS.T.R2", 2))),
                            ColumnIoKind::DoubleRight(1) => (&[3, 2, 1, 0][..], None),
                            _ => unreachable!(),
                        };
                        ipads = &[];
                        int_kind = "INT.IOI";
                        int_naming = "INT.IOI.TB";
                        ioi_kind = "IOI";
                        ioi_naming = "IOI";
                        if matches!(
                            cd.io,
                            ColumnIoKind::SingleLeftAlt | ColumnIoKind::SingleRightAlt
                        ) {
                            ioi_naming = "IOI.TBS";
                        }
                        if self.kind == GridKind::Virtex2PX && col == self.col_clk - 1 {
                            ioi_kind = "IOI.CLK_T";
                            ioi_naming = "IOI.CLK_T";
                            int_kind = "INT.IOI.CLK_T";
                            int_naming = "INT.IOI.CLK_T";
                        }
                        iobs = &[3, 2, 1, 0];
                    }
                    GridKind::Spartan3 => {
                        (pads, iobs_kind) = match cd.io {
                            ColumnIoKind::Double(0) => (&[2, 1, 0][..], Some(("IOBS.S3.T2", 2))),
                            ColumnIoKind::Double(1) => (&[1, 0][..], None),
                            _ => unreachable!(),
                        };
                        ipads = &[];
                        int_kind = "INT.IOI.S3";
                        int_naming = "INT.IOI";
                        ioi_kind = "IOI.S3";
                        ioi_naming = "IOI.S3.T";
                        iobs = &[2, 1, 0];
                    }
                    GridKind::Spartan3E => {
                        (pads, ipads, term, iobs_kind) = match cd.io {
                            ColumnIoKind::Single => {
                                (&[2][..], &[][..], "TTERM1", Some(("IOBS.S3E.T1", 1)))
                            }
                            ColumnIoKind::Double(0) => {
                                (&[1, 0][..], &[][..], "TTERM2", Some(("IOBS.S3E.T2", 2)))
                            }
                            ColumnIoKind::Double(1) => (&[][..], &[2][..], "TTERM", None),
                            ColumnIoKind::Triple(0) => {
                                (&[1, 0][..], &[][..], "TTERM3", Some(("IOBS.S3E.T3", 3)))
                            }
                            ColumnIoKind::Triple(1) => (&[][..], &[2][..], "TTERM", None),
                            ColumnIoKind::Triple(2) => (&[1, 0][..], &[][..], "TTERM", None),
                            ColumnIoKind::Quad(0) => {
                                (&[1, 0][..], &[][..], "TTERM4", Some(("IOBS.S3E.T4", 4)))
                            }
                            ColumnIoKind::Quad(1) => (&[2][..], &[][..], "TTERM", None),
                            ColumnIoKind::Quad(2) => (&[1, 0][..], &[][..], "TTERM", None),
                            ColumnIoKind::Quad(3) => (&[][..], &[1, 0][..], "TTERM", None),
                            _ => unreachable!(),
                        };
                        if cd.io == ColumnIoKind::Quad(0) && cd.kind == ColumnKind::BramCont(2) {
                            term = "TTERM4_BRAM2";
                        }
                        if col == self.col_clk - 2 {
                            term = "TTERMCLK";
                        }
                        if col == self.col_clk - 1 {
                            term = "TTERMCLKA";
                        }
                        if col == self.col_clk {
                            term = "TTERM4CLK";
                        }
                        if col == self.col_clk + 2 {
                            term = "TTERMCLKA";
                        }
                        int_kind = "INT.IOI.S3E";
                        int_naming = "INT.IOI";
                        ioi_kind = "IOI.S3E";
                        ioi_naming = "IOI.S3E.T";
                        iobs = &[2, 1, 0];
                        if ipads.is_empty() {
                            kind = "TIOIS";
                        } else {
                            kind = "TIBUFS";
                        }
                    }
                    GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                        (pads, ipads, term, iobs_kind) = match cd.io {
                            ColumnIoKind::Double(0) => {
                                (&[0, 1][..], &[2][..], "TTERM2", Some(("IOBS.S3A.T2", 2)))
                            }
                            ColumnIoKind::Double(1) => (&[0, 1][..], &[][..], "TTERM", None),
                            _ => unreachable!(),
                        };
                        int_kind = "INT.IOI.S3A.TB";
                        int_naming = "INT.IOI.S3A.TB";
                        if self.kind == GridKind::Spartan3ADsp {
                            ioi_kind = "IOI.S3ADSP.T";
                            ioi_naming = "IOI.S3ADSP.T";
                        } else {
                            ioi_kind = "IOI.S3A.T";
                            ioi_naming = "IOI.S3A.T";
                        }
                        iobs = &[0, 1, 2];
                        if ipads.is_empty() {
                            kind = "TIOIS";
                        } else {
                            kind = "TIOIB";
                        }
                        if col == self.col_clk - 2 {
                            term = "TTERM2CLK";
                        }
                        if col == self.col_clk - 1 {
                            term = "TTERMCLKA";
                        }
                        if col == self.col_clk {
                            term = "TTERM2CLK";
                        }
                        if col == self.col_clk + 1 {
                            term = "TTERMCLKA";
                        }
                        if self.kind == GridKind::Spartan3ADsp {
                            match cd.kind {
                                ColumnKind::BramCont(2) => {
                                    term = "TTERM1";
                                }
                                ColumnKind::Dsp => {
                                    term = "TTERM1_MACC";
                                }
                                _ => (),
                            }
                        }
                    }
                }
                let name;
                let term_name;
                if use_xy {
                    let x = xlut[col];
                    let y = row.to_idx();
                    name = format!("{kind}_X{x}Y{y}");
                    term_name = format!("{term}_X{x}Y{y}");
                } else {
                    let c = clut[col];
                    name = format!("TIOIC{c}");
                    term_name = format!("TTERMC{c}");
                }
                grid.fill_tile((col, row), int_kind, int_naming, name.clone());
                self.fill_term(&mut grid, (col, row), "TERM.N", "TERM.N", term_name);
                let node = grid[(col, row)].add_xnode(
                    db.get_node(ioi_kind),
                    &[&name],
                    db.get_node_naming(ioi_naming),
                    &[(col, row)],
                );
                for &i in iobs {
                    if pads.contains(&i) {
                        bonded_ios.push(((col, row), BelId::from_idx(i)));
                        match (ioi_kind, i) {
                            ("IOI.CLK_T", 0) => node.add_bel(i, "CLKPPAD1".to_string()),
                            ("IOI.CLK_T", 1) => node.add_bel(i, "CLKNPAD1".to_string()),
                            _ => node.add_bel(i, format!("PAD{ctr_pad}")),
                        }
                        ctr_pad += 1;
                    } else if ipads.contains(&i) {
                        bonded_ios.push(((col, row), BelId::from_idx(i)));
                        node.add_bel(i, format!("IPAD{ctr_pad}"));
                        ctr_pad += 1;
                    } else {
                        node.add_bel(i, format!("NOPAD{ctr_nopad}"));
                        ctr_nopad += 1;
                    }
                }
                if let Some((kind, num)) = iobs_kind {
                    let coords: Vec<_> = (0..num).map(|dx| (col + dx, row)).collect();
                    grid[(col, row)].add_xnode(
                        db.get_node(kind),
                        &[&name],
                        db.get_node_naming(kind),
                        &coords,
                    );
                }
                if !self.kind.is_virtex2() {
                    let node = grid[(col, row)].add_xnode(
                        db.get_node("RANDOR"),
                        &[&name],
                        db.get_node_naming("RANDOR.T"),
                        &[(col, row)],
                    );
                    let x = if self.kind == GridKind::Spartan3 {
                        (clut[col] - 1) * 2
                    } else {
                        col.to_idx() - 1
                    };
                    node.add_bel(0, format!("RANDOR_X{x}Y1"));
                }
            }
            for (row, &rd) in self.rows.iter().rev() {
                let col = col_r;
                if row == row_b || row == row_t {
                    continue;
                }
                let pads: &[usize];
                let ipads: &[usize];
                let int_kind;
                let int_naming;
                let ioi_kind;
                let ioi_naming;
                let iobs_kind;
                let iobs: &[usize];
                let mut term = "";
                let mut term_kind = "TERM.E";
                match self.kind {
                    GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
                        (pads, iobs_kind) = match rd {
                            RowIoKind::DoubleBot(0) => (&[3, 2, 1, 0][..], Some(("IOBS.R.B2", 2))),
                            RowIoKind::DoubleBot(1) => (&[3, 2][..], None),
                            RowIoKind::DoubleTop(0) => (&[1, 0][..], Some(("IOBS.R.T2", 2))),
                            RowIoKind::DoubleTop(1) => (&[3, 2, 1, 0][..], None),
                            _ => unreachable!(),
                        };
                        ipads = &[];
                        int_kind = "INT.IOI";
                        int_naming = "INT.IOI.LR";
                        ioi_kind = "IOI";
                        ioi_naming = "IOI";
                        iobs = &[3, 2, 1, 0];
                        term_kind = if row < self.row_pci.unwrap() {
                            "TERM.E.D"
                        } else {
                            "TERM.E.U"
                        };
                    }
                    GridKind::Spartan3 => {
                        pads = &[1, 0];
                        ipads = &[];
                        iobs_kind = Some(("IOBS.S3.R1", 1));
                        int_kind = "INT.IOI.S3";
                        int_naming = "INT.IOI";
                        ioi_kind = "IOI.S3";
                        ioi_naming = "IOI.S3.R";
                        iobs = &[2, 1, 0];
                    }
                    GridKind::Spartan3E => {
                        (pads, ipads, term, iobs_kind) = match rd {
                            RowIoKind::Single => {
                                (&[2][..], &[][..], "RTERM1", Some(("IOBS.S3E.R1", 1)))
                            }
                            RowIoKind::Double(0) => {
                                (&[1, 0][..], &[][..], "RTERM2", Some(("IOBS.S3E.R2", 2)))
                            }
                            RowIoKind::Double(1) => (&[][..], &[][..], "RTERM", None),
                            RowIoKind::Triple(0) => {
                                (&[1, 0][..], &[][..], "RTERM3", Some(("IOBS.S3E.R3", 3)))
                            }
                            RowIoKind::Triple(1) => (&[2][..], &[][..], "RTERM", None),
                            RowIoKind::Triple(2) => (&[][..], &[2][..], "RTERM", None),
                            RowIoKind::Quad(0) => {
                                (&[1, 0][..], &[][..], "RTERM4", Some(("IOBS.S3E.R4", 4)))
                            }
                            RowIoKind::Quad(1) => (&[][..], &[][..], "RTERM", None),
                            RowIoKind::Quad(2) => (&[1, 0][..], &[][..], "RTERM", None),
                            RowIoKind::Quad(3) => (&[][..], &[2][..], "RTERM", None),
                            _ => unreachable!(),
                        };
                        if row == self.row_mid() {
                            term = "RTERM4CLK";
                        }
                        if row == self.row_mid() - 4 {
                            term = "RTERM4CLKB";
                        }
                        if row == self.row_mid() - 2 {
                            term = "RTERMCLKA";
                        }
                        if row == self.row_mid() + 2 {
                            term = "RTERMCLKA";
                        }
                        int_kind = "INT.IOI.S3E";
                        if rows_brk.contains(&row) {
                            int_naming = "INT.IOI.BRK";
                        } else {
                            int_naming = "INT.IOI";
                        }
                        ioi_kind = "IOI.S3E";
                        if row >= self.row_mid() - 4 && row < self.row_mid() + 4 {
                            if ipads.is_empty() {
                                ioi_naming = "IOI.S3E.R.PCI.PCI";
                            } else {
                                ioi_naming = "IOI.S3E.R.PCI";
                            }
                        } else {
                            ioi_naming = "IOI.S3E.R";
                        }
                        iobs = &[2, 1, 0];
                    }
                    GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                        (pads, ipads, term, iobs_kind) = match rd {
                            RowIoKind::Quad(0) => {
                                (&[1, 0][..], &[][..], "RTERM4", Some(("IOBS.S3A.R4", 4)))
                            }
                            RowIoKind::Quad(1) => (&[1, 0][..], &[][..], "RTERM", None),
                            RowIoKind::Quad(2) => (&[1, 0][..], &[][..], "RTERM", None),
                            RowIoKind::Quad(3) => (&[][..], &[1, 0][..], "RTERM", None),
                            _ => unreachable!(),
                        };
                        if row == self.row_mid() {
                            term = "RTERM4CLK";
                        }
                        if row == self.row_mid() - 4 {
                            term = "RTERM4B";
                        }
                        if row == self.row_mid() - 3 {
                            term = "RTERMCLKB";
                        }
                        if row == self.row_mid() - 2 {
                            term = "RTERMCLKA";
                        }
                        if row == self.row_mid() + 1 {
                            term = "RTERMCLKA";
                        }
                        int_kind = "INT.IOI.S3A.LR";
                        if rows_brk.contains(&row) {
                            int_naming = "INT.IOI.S3A.LR.BRK";
                        } else {
                            int_naming = "INT.IOI.S3A.LR";
                        }
                        if self.kind == GridKind::Spartan3ADsp {
                            ioi_kind = "IOI.S3ADSP.R";
                            if row >= self.row_mid() - 4
                                && row < self.row_mid() + 4
                                && ipads.is_empty()
                            {
                                ioi_naming = "IOI.S3ADSP.R.PCI";
                            } else {
                                ioi_naming = "IOI.S3ADSP.R";
                            }
                        } else {
                            ioi_kind = "IOI.S3A.R";
                            if row >= self.row_mid() - 4
                                && row < self.row_mid() + 4
                                && ipads.is_empty()
                            {
                                ioi_naming = "IOI.S3A.R.PCI";
                            } else {
                                ioi_naming = "IOI.S3A.R";
                            }
                        }
                        iobs = &[1, 0];
                    }
                }
                let name;
                let term_name;
                if use_xy {
                    let x = xlut[col];
                    let y = row.to_idx();
                    let brk = if rows_brk.contains(&row) { "_BRK" } else { "" };
                    let clk = if row == self.row_mid() - 1 || row == self.row_mid() {
                        "_CLK"
                    } else {
                        ""
                    };
                    let pci = if row >= self.row_mid() - 4 && row < self.row_mid() + 4 {
                        "_PCI"
                    } else {
                        ""
                    };
                    let kind = if ipads.is_empty() { "RIOIS" } else { "RIBUFS" };
                    name = format!("{kind}{clk}{pci}{brk}_X{x}Y{y}");
                    term_name = format!("{term}_X{x}Y{y}");
                } else {
                    let r = row_t.to_idx() - row.to_idx();
                    name = format!("RIOIR{r}");
                    term_name = format!("RTERMR{r}");
                }
                grid.fill_tile((col, row), int_kind, int_naming, name.clone());
                self.fill_term(&mut grid, (col, row), "TERM.E", term_kind, term_name);
                let node = grid[(col, row)].add_xnode(
                    db.get_node(ioi_kind),
                    &[&name],
                    db.get_node_naming(ioi_naming),
                    &[(col, row)],
                );
                for &i in iobs {
                    if pads.contains(&i) {
                        bonded_ios.push(((col, row), BelId::from_idx(i)));
                        node.add_bel(i, format!("PAD{ctr_pad}"));
                        ctr_pad += 1;
                    } else if ipads.contains(&i) {
                        bonded_ios.push(((col, row), BelId::from_idx(i)));
                        node.add_bel(i, format!("IPAD{ctr_pad}"));
                        ctr_pad += 1;
                    } else {
                        node.add_bel(i, format!("NOPAD{ctr_nopad}"));
                        ctr_nopad += 1;
                    }
                }
                if let Some((kind, num)) = iobs_kind {
                    let coords: Vec<_> = (0..num).map(|dx| (col, row + dx)).collect();
                    grid[(col, row)].add_xnode(
                        db.get_node(kind),
                        &[&name],
                        db.get_node_naming(kind),
                        &coords,
                    );
                }
            }
            for (col, &cd) in self.columns.iter().rev() {
                let row = row_b;
                if use_xy {
                    if cd.kind == ColumnKind::Io {
                        continue;
                    }
                } else {
                    if cd.kind != ColumnKind::Clb {
                        continue;
                    }
                }
                let pads: &[usize];
                let ipads: &[usize];
                let mut int_kind;
                let mut int_naming;
                let mut ioi_kind;
                let mut ioi_naming;
                let iobs_kind;
                let iobs: &[usize];
                let mut term = "";
                let mut kind = "";
                match self.kind {
                    GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
                        (pads, iobs_kind) = match cd.io {
                            ColumnIoKind::None => (&[][..], None),
                            ColumnIoKind::SingleLeft | ColumnIoKind::SingleLeftAlt => {
                                (&[3, 2, 1][..], Some(("IOBS.B.L1", 1)))
                            }
                            ColumnIoKind::SingleRight | ColumnIoKind::SingleRightAlt => {
                                (&[2, 1, 0][..], Some(("IOBS.B.R1", 1)))
                            }
                            ColumnIoKind::DoubleLeft(0) => {
                                (&[3, 2, 1, 0][..], Some(("IOBS.B.L2", 2)))
                            }
                            ColumnIoKind::DoubleRight(0) => (&[1, 0][..], Some(("IOBS.B.R2", 2))),
                            ColumnIoKind::DoubleLeft(1) => (&[3, 2][..], None),
                            ColumnIoKind::DoubleRight(1) => (&[3, 2, 1, 0][..], None),
                            _ => unreachable!(),
                        };
                        ipads = &[];
                        int_kind = "INT.IOI";
                        int_naming = "INT.IOI.TB";
                        ioi_kind = "IOI";
                        ioi_naming = "IOI";
                        if matches!(
                            cd.io,
                            ColumnIoKind::SingleLeftAlt | ColumnIoKind::SingleRightAlt
                        ) {
                            ioi_naming = "IOI.TBS";
                        }
                        if self.kind == GridKind::Virtex2PX && col == self.col_clk - 1 {
                            ioi_kind = "IOI.CLK_B";
                            ioi_naming = "IOI.CLK_B";
                            int_kind = "INT.IOI.CLK_B";
                            int_naming = "INT.IOI.CLK_B";
                        }
                        iobs = &[3, 2, 1, 0];
                    }
                    GridKind::Spartan3 => {
                        (pads, iobs_kind) = match cd.io {
                            ColumnIoKind::Double(0) => (&[1, 0][..], Some(("IOBS.S3.B2", 2))),
                            ColumnIoKind::Double(1) => (&[2, 1, 0][..], None),
                            _ => unreachable!(),
                        };
                        ipads = &[];
                        int_kind = "INT.IOI.S3";
                        int_naming = "INT.IOI";
                        ioi_kind = "IOI.S3";
                        ioi_naming = "IOI.S3.B";
                        iobs = &[2, 1, 0];
                    }
                    GridKind::Spartan3E => {
                        (pads, ipads, term, iobs_kind) = match cd.io {
                            ColumnIoKind::Single => {
                                (&[2][..], &[][..], "BTERM1", Some(("IOBS.S3E.B1", 1)))
                            }
                            ColumnIoKind::Double(0) => {
                                (&[][..], &[2][..], "BTERM2", Some(("IOBS.S3E.B2", 2)))
                            }
                            ColumnIoKind::Double(1) => (&[1, 0][..], &[][..], "BTERM", None),
                            ColumnIoKind::Triple(0) => {
                                (&[1, 0][..], &[][..], "BTERM3", Some(("IOBS.S3E.B3", 3)))
                            }
                            ColumnIoKind::Triple(1) => (&[][..], &[2][..], "BTERM", None),
                            ColumnIoKind::Triple(2) => (&[1, 0][..], &[][..], "BTERM", None),
                            ColumnIoKind::Quad(0) => {
                                (&[][..], &[1, 0][..], "BTERM4", Some(("IOBS.S3E.B4", 4)))
                            }
                            ColumnIoKind::Quad(1) => (&[1, 0][..], &[][..], "BTERM", None),
                            ColumnIoKind::Quad(2) => (&[2][..], &[][..], "BTERM", None),
                            ColumnIoKind::Quad(3) => (&[1, 0][..], &[][..], "BTERM", None),
                            _ => unreachable!(),
                        };
                        if cd.io == ColumnIoKind::Quad(0) && cd.kind == ColumnKind::BramCont(2) {
                            term = "BTERM4_BRAM2";
                        }
                        if col == self.col_clk - 3 {
                            term = "BTERMCLKA";
                        }
                        if col == self.col_clk - 1 {
                            term = "BTERMCLKB";
                        }
                        if col == self.col_clk {
                            term = "BTERM4CLK";
                        }
                        if col == self.col_clk + 1 {
                            term = "BTERMCLK";
                        }
                        int_kind = "INT.IOI.S3E";
                        int_naming = "INT.IOI";
                        ioi_kind = "IOI.S3E";
                        ioi_naming = "IOI.S3E.B";
                        iobs = &[2, 1, 0];
                        if ipads.is_empty() {
                            kind = "BIOIS";
                        } else {
                            kind = "BIBUFS";
                        }
                    }
                    GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                        (pads, ipads, term, iobs_kind) = match cd.io {
                            ColumnIoKind::Double(0) => {
                                (&[1, 0][..], &[2][..], "BTERM2", Some(("IOBS.S3A.B2", 2)))
                            }
                            ColumnIoKind::Double(1) => (&[1, 0][..], &[][..], "BTERM", None),
                            _ => unreachable!(),
                        };
                        int_kind = "INT.IOI.S3A.TB";
                        int_naming = "INT.IOI.S3A.TB";
                        if self.kind == GridKind::Spartan3ADsp {
                            ioi_kind = "IOI.S3ADSP.B";
                            ioi_naming = "IOI.S3ADSP.B";
                        } else {
                            ioi_kind = "IOI.S3A.B";
                            ioi_naming = "IOI.S3A.B";
                        }
                        iobs = &[2, 1, 0];
                        if ipads.is_empty() {
                            kind = "BIOIS";
                        } else {
                            kind = "BIOIB";
                        }
                        if col == self.col_clk - 2 {
                            term = "BTERM2CLK";
                        }
                        if col == self.col_clk - 1 {
                            term = "BTERMCLKB";
                        }
                        if col == self.col_clk {
                            term = "BTERM2CLK";
                        }
                        if col == self.col_clk + 1 {
                            term = "BTERMCLK";
                        }
                        if self.kind == GridKind::Spartan3ADsp {
                            match cd.kind {
                                ColumnKind::BramCont(2) => {
                                    term = "BTERM1";
                                }
                                ColumnKind::Dsp => {
                                    term = "BTERM1_MACC";
                                }
                                _ => (),
                            }
                        }
                    }
                }
                let name;
                let term_name;
                if use_xy {
                    let x = xlut[col];
                    let y = row.to_idx();
                    name = format!("{kind}_X{x}Y{y}");
                    term_name = format!("{term}_X{x}Y{y}");
                } else {
                    let c = clut[col];
                    name = format!("BIOIC{c}");
                    term_name = format!("BTERMC{c}");
                }
                grid.fill_tile((col, row), int_kind, int_naming, name.clone());
                self.fill_term(&mut grid, (col, row), "TERM.S", "TERM.S", term_name);
                let node = grid[(col, row)].add_xnode(
                    db.get_node(ioi_kind),
                    &[&name],
                    db.get_node_naming(ioi_naming),
                    &[(col, row)],
                );
                for &i in iobs {
                    if pads.contains(&i) {
                        bonded_ios.push(((col, row), BelId::from_idx(i)));
                        let mut name = format!("PAD{ctr_pad}");
                        if self.kind == GridKind::Spartan3A && self.cols_clkv.is_none() {
                            // 3s50a special
                            match ctr_pad {
                                94 => name = "PAD96".to_string(),
                                96 => name = "PAD97".to_string(),
                                97 => name = "PAD95".to_string(),
                                _ => (),
                            }
                        }
                        match (ioi_kind, i) {
                            ("IOI.CLK_B", 2) => name = "CLKPPAD2".to_string(),
                            ("IOI.CLK_B", 3) => name = "CLKNPAD2".to_string(),
                            _ => (),
                        }
                        node.add_bel(i, name);
                        ctr_pad += 1;
                    } else if ipads.contains(&i) {
                        bonded_ios.push(((col, row), BelId::from_idx(i)));
                        let mut name = format!("IPAD{ctr_pad}");
                        if self.kind == GridKind::Spartan3A
                            && self.cols_clkv.is_none()
                            && ctr_pad == 95
                        {
                            name = "IPAD94".to_string();
                        }
                        node.add_bel(i, name);
                        ctr_pad += 1;
                    } else {
                        node.add_bel(i, format!("NOPAD{ctr_nopad}"));
                        ctr_nopad += 1;
                    }
                }
                if let Some((kind, num)) = iobs_kind {
                    let coords: Vec<_> = (0..num).map(|dx| (col + dx, row)).collect();
                    grid[(col, row)].add_xnode(
                        db.get_node(kind),
                        &[&name],
                        db.get_node_naming(kind),
                        &coords,
                    );
                }
                if !self.kind.is_virtex2() {
                    let node = grid[(col, row)].add_xnode(
                        db.get_node("RANDOR"),
                        &[&name],
                        db.get_node_naming("RANDOR.B"),
                        &[(col, row)],
                    );
                    let x = if self.kind == GridKind::Spartan3 {
                        (clut[col] - 1) * 2
                    } else {
                        col.to_idx() - 1
                    };
                    node.add_bel(0, format!("RANDOR_X{x}Y0"));
                }
            }
            for (row, &rd) in self.rows.iter() {
                let col = col_l;
                if row == row_b || row == row_t {
                    continue;
                }
                let pads: &[usize];
                let ipads: &[usize];
                let int_kind;
                let int_naming;
                let ioi_kind;
                let ioi_naming;
                let iobs_kind;
                let iobs: &[usize];
                let mut term = "";
                let mut term_kind = "TERM.W";
                match self.kind {
                    GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
                        (pads, iobs_kind) = match rd {
                            RowIoKind::DoubleBot(0) => (&[0, 1, 2, 3][..], Some(("IOBS.L.B2", 2))),
                            RowIoKind::DoubleBot(1) => (&[2, 3][..], None),
                            RowIoKind::DoubleTop(0) => (&[0, 1][..], Some(("IOBS.L.T2", 2))),
                            RowIoKind::DoubleTop(1) => (&[0, 1, 2, 3][..], None),
                            _ => unreachable!(),
                        };
                        ipads = &[];
                        int_kind = "INT.IOI";
                        int_naming = "INT.IOI.LR";
                        ioi_kind = "IOI";
                        ioi_naming = "IOI";
                        iobs = &[0, 1, 2, 3];
                        term_kind = if row < self.row_pci.unwrap() {
                            "TERM.W.D"
                        } else {
                            "TERM.W.U"
                        };
                    }
                    GridKind::Spartan3 => {
                        pads = &[0, 1];
                        ipads = &[];
                        iobs_kind = Some(("IOBS.S3.L1", 1));
                        int_kind = "INT.IOI.S3";
                        int_naming = "INT.IOI";
                        ioi_kind = "IOI.S3";
                        ioi_naming = "IOI.S3.L";
                        iobs = &[0, 1, 2];
                    }
                    GridKind::Spartan3E => {
                        (pads, ipads, term, iobs_kind) = match rd {
                            RowIoKind::Single => {
                                (&[2][..], &[][..], "LTERM1", Some(("IOBS.S3E.L1", 1)))
                            }
                            RowIoKind::Double(0) => {
                                (&[][..], &[][..], "LTERM2", Some(("IOBS.S3E.L2", 2)))
                            }
                            RowIoKind::Double(1) => (&[1, 0][..], &[][..], "LTERM", None),
                            RowIoKind::Triple(0) => {
                                (&[][..], &[2][..], "LTERM3", Some(("IOBS.S3E.L3", 3)))
                            }
                            RowIoKind::Triple(1) => (&[2][..], &[][..], "LTERM", None),
                            RowIoKind::Triple(2) => (&[1, 0][..], &[][..], "LTERM", None),
                            RowIoKind::Quad(0) => {
                                (&[][..], &[2][..], "LTERM4", Some(("IOBS.S3E.L4", 4)))
                            }
                            RowIoKind::Quad(1) => (&[1, 0][..], &[][..], "LTERM", None),
                            RowIoKind::Quad(2) => (&[][..], &[][..], "LTERM", None),
                            RowIoKind::Quad(3) => (&[1, 0][..], &[][..], "LTERM", None),
                            _ => unreachable!(),
                        };
                        if row == self.row_mid() {
                            term = "LTERM4CLK";
                        }
                        if row == self.row_mid() - 4 {
                            term = "LTERM4B";
                        }
                        if row == self.row_mid() - 3 {
                            term = "LTERMCLKA";
                        }
                        if row == self.row_mid() - 1 {
                            term = "LTERMCLK";
                        }
                        if row == self.row_mid() + 1 {
                            term = "LTERMCLKA";
                        }
                        if row == self.row_mid() + 3 {
                            term = "LTERMCLK";
                        }
                        int_kind = "INT.IOI.S3E";
                        if rows_brk.contains(&row) {
                            int_naming = "INT.IOI.BRK";
                        } else {
                            int_naming = "INT.IOI";
                        }
                        ioi_kind = "IOI.S3E";
                        if row >= self.row_mid() - 4 && row < self.row_mid() + 4 {
                            if ipads.is_empty() {
                                ioi_naming = "IOI.S3E.L.PCI.PCI";
                            } else {
                                ioi_naming = "IOI.S3E.L.PCI";
                            }
                        } else {
                            ioi_naming = "IOI.S3E.L";
                        }
                        iobs = &[2, 1, 0];
                    }
                    GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                        (pads, ipads, term, iobs_kind) = match rd {
                            RowIoKind::Quad(0) => {
                                (&[][..], &[0, 1][..], "LTERM4", Some(("IOBS.S3A.L4", 4)))
                            }
                            RowIoKind::Quad(1) => (&[0, 1][..], &[][..], "LTERM", None),
                            RowIoKind::Quad(2) => (&[0, 1][..], &[][..], "LTERM", None),
                            RowIoKind::Quad(3) => (&[0, 1][..], &[][..], "LTERM", None),
                            _ => unreachable!(),
                        };
                        if row == self.row_mid() {
                            term = "LTERM4CLK";
                        }
                        if row == self.row_mid() - 4 {
                            term = "LTERM4B";
                        }
                        if row == self.row_mid() - 2 {
                            term = "LTERMCLKA";
                        }
                        if row == self.row_mid() - 1 {
                            term = "LTERMCLK";
                        }
                        if row == self.row_mid() + 1 {
                            term = "LTERMCLKA";
                        }
                        if row == self.row_mid() + 2 {
                            term = "LTERMCLK";
                        }
                        int_kind = "INT.IOI.S3A.LR";
                        if rows_brk.contains(&row) {
                            int_naming = "INT.IOI.S3A.LR.BRK";
                        } else {
                            int_naming = "INT.IOI.S3A.LR";
                        }
                        if self.kind == GridKind::Spartan3ADsp {
                            ioi_kind = "IOI.S3ADSP.L";
                            if row >= self.row_mid() - 4
                                && row < self.row_mid() + 4
                                && ipads.is_empty()
                            {
                                ioi_naming = "IOI.S3ADSP.L.PCI";
                            } else {
                                ioi_naming = "IOI.S3ADSP.L";
                            }
                        } else {
                            ioi_kind = "IOI.S3A.L";
                            if row >= self.row_mid() - 4
                                && row < self.row_mid() + 4
                                && ipads.is_empty()
                            {
                                ioi_naming = "IOI.S3A.L.PCI";
                            } else {
                                ioi_naming = "IOI.S3A.L";
                            }
                        }
                        iobs = &[0, 1];
                    }
                }
                let name;
                let term_name;
                if use_xy {
                    let x = xlut[col];
                    let y = row.to_idx();
                    let brk = if rows_brk.contains(&row) { "_BRK" } else { "" };
                    let clk = if row == self.row_mid() - 1 || row == self.row_mid() {
                        "_CLK"
                    } else {
                        ""
                    };
                    let pci = if row >= self.row_mid() - 4 && row < self.row_mid() + 4 {
                        "_PCI"
                    } else {
                        ""
                    };
                    let kind = if ipads.is_empty() { "LIOIS" } else { "LIBUFS" };
                    name = format!("{kind}{clk}{pci}{brk}_X{x}Y{y}");
                    term_name = format!("{term}_X{x}Y{y}");
                } else {
                    let r = row_t.to_idx() - row.to_idx();
                    name = format!("LIOIR{r}");
                    term_name = format!("LTERMR{r}");
                }
                grid.fill_tile((col, row), int_kind, int_naming, name.clone());
                self.fill_term(&mut grid, (col, row), "TERM.W", term_kind, term_name);
                let node = grid[(col, row)].add_xnode(
                    db.get_node(ioi_kind),
                    &[&name],
                    db.get_node_naming(ioi_naming),
                    &[(col, row)],
                );
                for &i in iobs {
                    if pads.contains(&i) {
                        bonded_ios.push(((col, row), BelId::from_idx(i)));
                        node.add_bel(i, format!("PAD{ctr_pad}"));
                        ctr_pad += 1;
                    } else if ipads.contains(&i) {
                        bonded_ios.push(((col, row), BelId::from_idx(i)));
                        node.add_bel(i, format!("IPAD{ctr_pad}"));
                        ctr_pad += 1;
                    } else {
                        node.add_bel(i, format!("NOPAD{ctr_nopad}"));
                        ctr_nopad += 1;
                    }
                }
                if let Some((kind, num)) = iobs_kind {
                    let coords: Vec<_> = (0..num).map(|dx| (col, row + dx)).collect();
                    grid[(col, row)].add_xnode(
                        db.get_node(kind),
                        &[&name],
                        db.get_node_naming(kind),
                        &coords,
                    );
                }
            }
        }

        let mut cx = 0;
        for (col, &cd) in self.columns.iter() {
            if self.kind == GridKind::Spartan3E {
                if cd.kind == ColumnKind::Io {
                    continue;
                }
            } else {
                if cd.kind != ColumnKind::Clb {
                    continue;
                }
            }
            for (row, &io) in self.rows.iter() {
                if io == RowIoKind::None {
                    continue;
                }
                let tile = if use_xy {
                    let x = xlut[col];
                    let y = row.to_idx();
                    format!("CLB_X{x}Y{y}")
                } else {
                    let c = clut[col];
                    let r = yt - row.to_idx();
                    format!("R{r}C{c}")
                };
                let naming = if use_xy && rows_brk.contains(&row) {
                    "INT.CLB.BRK"
                } else {
                    "INT.CLB"
                };
                grid.fill_tile((col, row), "INT.CLB", naming, tile.clone());
                let node = grid[(col, row)].add_xnode(
                    db.get_node("CLB"),
                    &[&tile],
                    db.get_node_naming("CLB"),
                    &[(col, row)],
                );
                let sx = 2 * cx;
                let sy = 2 * (row.to_idx() - 1);
                if self.kind.is_virtex2() {
                    node.add_bel(0, format!("SLICE_X{x}Y{y}", x = sx, y = sy));
                    node.add_bel(1, format!("SLICE_X{x}Y{y}", x = sx, y = sy + 1));
                    node.add_bel(2, format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy));
                    node.add_bel(3, format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy + 1));
                    if cx % 2 == 0 {
                        node.add_bel(4, format!("TBUF_X{x}Y{y}", x = sx, y = sy));
                        node.add_bel(5, format!("TBUF_X{x}Y{y}", x = sx, y = sy + 1));
                    } else {
                        node.add_bel(4, format!("TBUF_X{x}Y{y}", x = sx, y = sy + 1));
                        node.add_bel(5, format!("TBUF_X{x}Y{y}", x = sx, y = sy));
                    }
                } else {
                    node.add_bel(0, format!("SLICE_X{x}Y{y}", x = sx, y = sy));
                    node.add_bel(1, format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy));
                    node.add_bel(2, format!("SLICE_X{x}Y{y}", x = sx, y = sy + 1));
                    node.add_bel(3, format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy + 1));
                }
            }
            cx += 1;
        }

        let bram_kind = match self.kind {
            GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => "INT.BRAM",
            GridKind::Spartan3 => "INT.BRAM.S3",
            GridKind::Spartan3E => "INT.BRAM.S3E",
            GridKind::Spartan3A => "INT.BRAM.S3A",
            GridKind::Spartan3ADsp => "INT.BRAM.S3ADSP",
        };
        let mut sx = 0;
        for (col, &cd) in self.columns.iter() {
            if cd.kind != ColumnKind::Bram {
                continue;
            }
            if let Some((b, t)) = self.rows_ram {
                grid.nuke_rect(col, b, 4, t.to_idx() - b.to_idx() + 1);
                for d in 1..4 {
                    let x = xlut[col + d];
                    let yb = b.to_idx();
                    let yt = t.to_idx();
                    grid.fill_term_pair_bounce(
                        (col + d, b - 1),
                        (col + d, t + 1),
                        db.get_term("TERM.BRAM.N"),
                        db.get_term("TERM.BRAM.S"),
                        format!("COB_TERM_B_X{x}Y{yb}"),
                        format!("COB_TERM_T_X{x}Y{yt}"),
                        db.get_term_naming("TERM.BRAM.N"),
                        db.get_term_naming("TERM.BRAM.S"),
                    );
                }
            }
            let mut i = 0;
            for (row, &io) in self.rows.iter() {
                if io == RowIoKind::None {
                    continue;
                }
                if let Some((b, t)) = self.rows_ram {
                    if row <= b || row >= t {
                        continue;
                    }
                }
                let naming = match self.kind {
                    GridKind::Virtex2
                    | GridKind::Virtex2P
                    | GridKind::Virtex2PX
                    | GridKind::Spartan3 => "INT.BRAM",
                    GridKind::Spartan3E | GridKind::Spartan3A => {
                        if rows_brk.contains(&row) {
                            "INT.BRAM.BRK"
                        } else {
                            "INT.BRAM"
                        }
                    }
                    GridKind::Spartan3ADsp => {
                        if rows_brk.contains(&row) {
                            "INT.BRAM.S3ADSP.BRK"
                        } else {
                            "INT.BRAM.S3ADSP"
                        }
                    }
                };
                if use_xy {
                    let x = xlut[col];
                    let y = row.to_idx();
                    let mut md = "";
                    if rows_brk.contains(&row) {
                        md = "_BRK";
                    }
                    if self.kind != GridKind::Spartan3E {
                        if row == row_b + 1 {
                            md = "_BOT";
                        }
                        if row == row_t - 1 {
                            md = "_TOP";
                        }
                        if self.cols_clkv.is_none() && row == row_t - 5 {
                            md = "_TOP";
                        }
                    }
                    grid.fill_tile(
                        (col, row),
                        bram_kind,
                        naming,
                        format!("BRAM{i}_SMALL{md}_X{x}Y{y}"),
                    );
                    if self.kind == GridKind::Spartan3ADsp {
                        let naming_macc = if rows_brk.contains(&row) {
                            "INT.MACC.BRK"
                        } else {
                            "INT.MACC"
                        };
                        let x = xlut[col + 3];
                        grid.fill_tile(
                            (col + 3, row),
                            "INT.BRAM.S3ADSP",
                            naming_macc,
                            format!("MACC{i}_SMALL{md}_X{x}Y{y}"),
                        );
                    }
                } else {
                    let c = bramclut[col];
                    let r = yt - row.to_idx();
                    grid.fill_tile((col, row), bram_kind, naming, format!("BRAMR{r}C{c}"));
                }
                if i == 0 {
                    let is_bot = matches!(self.kind, GridKind::Spartan3A | GridKind::Spartan3ADsp)
                        && row == row_b + 1;
                    let is_top = matches!(self.kind, GridKind::Spartan3A | GridKind::Spartan3ADsp)
                        && (row == row_t - 4 || row == row_t - 8 && col == self.col_clk);
                    let is_brk = rows_brk.contains(&(row + 3));
                    let kind = match self.kind {
                        GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => "BRAM",
                        GridKind::Spartan3 => "BRAM.S3",
                        GridKind::Spartan3E => "BRAM.S3E",
                        GridKind::Spartan3A => "BRAM.S3A",
                        GridKind::Spartan3ADsp => "BRAM.S3ADSP",
                    };
                    let naming = if self.kind == GridKind::Spartan3A {
                        if is_bot {
                            "BRAM.S3A.BOT"
                        } else if is_top {
                            "BRAM.S3A.TOP"
                        } else {
                            "BRAM.S3A"
                        }
                    } else {
                        kind
                    };
                    let name = if use_xy {
                        let x = xlut[col] + 1;
                        let y = row.to_idx();
                        let m = if self.kind == GridKind::Spartan3ADsp {
                            "_3M"
                        } else {
                            ""
                        };
                        if is_bot {
                            format!("BRAMSITE2{m}_BOT_X{x}Y{y}")
                        } else if is_top {
                            format!("BRAMSITE2{m}_TOP_X{x}Y{y}")
                        } else if is_brk {
                            format!("BRAMSITE2{m}_BRK_X{x}Y{y}")
                        } else {
                            format!("BRAMSITE2{m}_X{x}Y{y}")
                        }
                    } else {
                        let c = bramclut[col];
                        let r = yt - row.to_idx();
                        format!("BMR{r}C{c}")
                    };
                    let node = grid[(col, row)].add_xnode(
                        db.get_node(kind),
                        &[&name],
                        db.get_node_naming(naming),
                        &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                    );
                    let mut sy = (row.to_idx() - 1) / 4;
                    if let Some((b, _)) = self.rows_ram {
                        sy = (row.to_idx() - b.to_idx() - 1) / 4;
                    }
                    if self.kind == GridKind::Spartan3A
                        && self.dcms == Some(Dcms::Eight)
                        && row >= self.row_mid()
                    {
                        sy -= 2;
                    }
                    node.add_bel(0, format!("RAMB16_X{sx}Y{sy}"));
                    if self.kind != GridKind::Spartan3ADsp {
                        node.add_bel(1, format!("MULT18X18_X{sx}Y{sy}"));
                    } else {
                        let naming = if is_top { "DSP.TOP" } else { "DSP" };
                        let x = xlut[col] + 4;
                        let y = row.to_idx();
                        let name = if is_bot {
                            format!("MACCSITE2_BOT_X{x}Y{y}")
                        } else if is_top {
                            format!("MACCSITE2_TOP_X{x}Y{y}")
                        } else if is_brk {
                            format!("MACCSITE2_BRK_X{x}Y{y}")
                        } else {
                            format!("MACCSITE2_X{x}Y{y}")
                        };
                        let node = grid[(col + 3, row)].add_xnode(
                            db.get_node("DSP"),
                            &[&name],
                            db.get_node_naming(naming),
                            &[
                                (col + 3, row),
                                (col + 3, row + 1),
                                (col + 3, row + 2),
                                (col + 3, row + 3),
                            ],
                        );
                        node.add_bel(0, format!("DSP48A_X{sx}Y{sy}"));
                    }
                }
                i += 1;
                i %= 4;
            }
            sx += 1;
        }

        if let Some(dcms) = self.dcms {
            let mut dcm_tiles = vec![];
            if dcms != Dcms::Two {
                // col, row, tile_kind, is_horiz
                grid.nuke_rect(self.col_clk - 4, row_b + 1, 4, 4);
                dcm_tiles.push((self.col_clk - 1, row_b + 1, "DCM_BL_CENTER", false));
            }
            if !(self.kind != GridKind::Spartan3E && dcms == Dcms::Two) {
                grid.nuke_rect(self.col_clk, row_b + 1, 4, 4);
                dcm_tiles.push((self.col_clk, row_b + 1, "DCM_BR_CENTER", false));
            }
            if !(self.kind == GridKind::Spartan3E && dcms == Dcms::Two) {
                grid.nuke_rect(self.col_clk - 4, row_t - 4, 4, 4);
                dcm_tiles.push((self.col_clk - 1, row_t - 1, "DCM_TL_CENTER", false));
            }
            {
                grid.nuke_rect(self.col_clk, row_t - 4, 4, 4);
                dcm_tiles.push((self.col_clk, row_t - 1, "DCM_TR_CENTER", false));
            }
            if self.kind == GridKind::Spartan3E && dcms == Dcms::Two {
                grid.nuke_rect(self.col_clk - 1, row_b + 1, 1, 4);
                grid.nuke_rect(self.col_clk - 1, row_t - 4, 1, 4);
                let x = xlut[self.col_clk - 1];
                let y = row_b.to_idx() + 1;
                grid.fill_tile_special(
                    (self.col_clk - 1, row_b + 1),
                    "INT.DCM.S3E.DUMMY",
                    "INT.DCM.S3E.DUMMY",
                    format!("DCMAUX_BL_CENTER_X{x}Y{y}"),
                );
                let y = row_t.to_idx() - 1;
                grid.fill_tile_special(
                    (self.col_clk - 1, row_t - 1),
                    "INT.DCM.S3E.DUMMY",
                    "INT.DCM.S3E.DUMMY",
                    format!("DCMAUX_TL_CENTER_X{x}Y{y}"),
                );
            }
            if dcms == Dcms::Eight {
                if self.kind == GridKind::Spartan3E {
                    grid.nuke_rect(col_l + 9, self.row_mid() - 4, 4, 4);
                    grid.nuke_rect(col_l + 9, self.row_mid(), 4, 4);
                    grid.nuke_rect(col_r - 12, self.row_mid() - 4, 4, 4);
                    grid.nuke_rect(col_r - 12, self.row_mid(), 4, 4);
                    dcm_tiles.push((col_l + 9, self.row_mid(), "DCM_H_TL_CENTER", true));
                    dcm_tiles.push((col_l + 9, self.row_mid() - 1, "DCM_H_BL_CENTER", true));
                    dcm_tiles.push((col_r - 9, self.row_mid(), "DCM_H_TR_CENTER", true));
                    dcm_tiles.push((col_r - 9, self.row_mid() - 1, "DCM_H_BR_CENTER", true));
                } else {
                    for col in [col_l + 3, col_r - 6] {
                        grid.nuke_rect(col, self.row_mid() - 4, 4, 4);
                        grid.nuke_rect(col, self.row_mid(), 4, 4);
                        dcm_tiles.push((col, self.row_mid(), "DCM_SPLY", true));
                        dcm_tiles.push((col, self.row_mid() - 1, "DCM_BGAP", true));
                    }
                }
            }
            let mut dcm_cols: Vec<_> = dcm_tiles.iter().map(|&(col, _, _, _)| col).collect();
            dcm_cols.sort_unstable();
            dcm_cols.dedup();
            let mut dcm_rows: Vec<_> = dcm_tiles.iter().map(|&(_, row, _, _)| row).collect();
            dcm_rows.sort_unstable();
            dcm_rows.dedup();
            for (col, row, tk, is_h) in dcm_tiles {
                let x = xlut[col];
                let y = row.to_idx();
                let name = format!("{tk}_X{x}Y{y}");
                grid.fill_tile_special(
                    (col, row),
                    "INT.DCM",
                    if is_h { "INT.DCM.S3E.H" } else { "INT.DCM.S3E" },
                    name.clone(),
                );
                let dx = dcm_cols.binary_search(&col).unwrap();
                let dy = dcm_rows.binary_search(&row).unwrap();
                let node = grid[(col, row)].add_xnode(
                    db.get_node("DCM.S3E"),
                    &[&name],
                    db.get_node_naming(if is_h {
                        "DCM.S3E.H"
                    } else if col < self.col_clk {
                        "DCM.S3E.L"
                    } else {
                        "DCM.S3E.R"
                    }),
                    &[(col, row)],
                );
                node.add_bel(0, format!("DCM_X{dx}Y{dy}"));
            }
        } else {
            let mut dx = 0;
            for (col, &cd) in self.columns.iter() {
                if cd.kind != ColumnKind::Bram {
                    continue;
                }
                if self.cols_gt.contains_key(&col) {
                    continue;
                }
                let (kind, naming, dcm) = match self.kind {
                    GridKind::Virtex2 => ("INT.DCM.V2", "INT.BRAM_IOIS", "DCM.V2"),
                    GridKind::Virtex2P | GridKind::Virtex2PX => {
                        ("INT.DCM.V2P", "INT.ML_BRAM_IOIS", "DCM.V2P")
                    }
                    GridKind::Spartan3 => {
                        if col == col_l + 3 || col == col_r - 3 {
                            ("INT.DCM", "INT.DCM.S3", "DCM.S3")
                        } else {
                            ("INT.DCM.S3.DUMMY", "INT.DCM.S3.DUMMY", "")
                        }
                    }
                    _ => unreachable!(),
                };
                let c = bramclut[col];
                let name_b = format!("BIOIBRAMC{c}");
                let name_t = format!("TIOIBRAMC{c}");
                grid.fill_tile((col, row_b), kind, naming, name_b.clone());
                grid.fill_tile((col, row_t), kind, naming, name_t.clone());
                self.fill_term(
                    &mut grid,
                    (col, row_b),
                    "TERM.S",
                    "TERM.S",
                    format!("BTERMBRAMC{c}"),
                );
                self.fill_term(
                    &mut grid,
                    (col, row_t),
                    "TERM.N",
                    "TERM.N",
                    format!("TTERMBRAMC{c}"),
                );
                if dcm.is_empty() {
                    continue;
                }
                let node = grid[(col, row_b)].add_xnode(
                    db.get_node(dcm),
                    &[&name_b],
                    db.get_node_naming(dcm),
                    &[(col, row_b)],
                );
                node.add_bel(0, format!("DCM_X{dx}Y0"));
                let node = grid[(col, row_t)].add_xnode(
                    db.get_node(dcm),
                    &[&name_t],
                    db.get_node_naming(dcm),
                    &[(col, row_t)],
                );
                node.add_bel(0, format!("DCM_X{dx}Y1"));
                dx += 1;
            }
        }

        for &(bc, br) in &self.holes_ppc {
            grid.nuke_rect(bc, br, 10, 16);
            let mut ints = vec![];
            // left side
            for d in 0..16 {
                let col = bc;
                let row = br + d;
                let r = yt - row.to_idx();
                let c = clut[col];
                let pref = match d {
                    1 => "PTERMLL",
                    14 => "PTERMUL",
                    _ => "",
                };
                let kind = match d {
                    0 => "INT.PPC.B",
                    15 => "INT.PPC.T",
                    _ => "INT.PPC.L",
                };
                grid.fill_tile_special((col, row), "INT.PPC", kind, format!("{pref}R{r}C{c}"));
                ints.push((col, row));
            }
            // right side
            for d in 0..16 {
                let col = bc + 9;
                let row = br + d;
                let r = yt - row.to_idx();
                let c = clut[col];
                grid.fill_tile_special((col, row), "INT.PPC", "INT.PPC.R", format!("R{r}C{c}"));
                ints.push((col, row));
            }
            // bottom
            for d in 1..9 {
                let col = bc + d;
                let row = br;
                let r = yt - row.to_idx();
                if self.columns[col].kind == ColumnKind::Clb {
                    let c = clut[col];
                    grid.fill_tile_special((col, row), "INT.PPC", "INT.PPC.B", format!("R{r}C{c}"));
                } else {
                    let c = bramclut[col];
                    grid.fill_tile_special(
                        (col, row),
                        "INT.PPC",
                        "INT.PPC.B",
                        format!("PPCINTR{r}BRAMC{c}"),
                    );
                }
                ints.push((col, row));
            }
            // top
            for d in 1..9 {
                let col = bc + d;
                let row = br + 15;
                let r = yt - row.to_idx();
                if self.columns[col].kind == ColumnKind::Clb {
                    let c = clut[col];
                    grid.fill_tile_special((col, row), "INT.PPC", "INT.PPC.T", format!("R{r}C{c}"));
                } else {
                    let c = bramclut[col];
                    grid.fill_tile_special(
                        (col, row),
                        "INT.PPC",
                        "INT.PPC.T",
                        format!("PPCINTR{r}BRAMC{c}"),
                    );
                }
                ints.push((col, row));
            }
            // horiz passes
            for d in 1..15 {
                let col_l = bc;
                let col_r = bc + 9;
                let row = br + d;
                let tile_l = grid[(col_l, row)].nodes[0].names[def_rt].clone();
                let c = bramclut[col_r - 1];
                let r = yt - row.to_idx();
                let tile_r = format!("BMR{r}C{c}");
                grid[(col_l, row)].add_xnode(
                    db.get_node("PPC.E"),
                    &[&tile_l, &tile_r],
                    db.get_node_naming("PPC.E"),
                    &[(col_l, row), (col_r, row)],
                );
                grid[(col_r, row)].add_xnode(
                    db.get_node("PPC.W"),
                    &[&tile_r, &tile_l],
                    db.get_node_naming("PPC.W"),
                    &[(col_r, row), (col_l, row)],
                );
                grid.fill_term_pair_dbuf(
                    (col_l, row),
                    (col_r, row),
                    db.get_term("PPC.E"),
                    db.get_term("PPC.W"),
                    tile_l,
                    tile_r,
                    db.get_term_naming("PPC.E"),
                    db.get_term_naming("PPC.W"),
                );
            }
            // vert passes
            for d in 1..9 {
                let col = bc + d;
                let row_b = br;
                let row_t = br + 15;
                let rb = yt - row_b.to_idx() - 1;
                let rt = yt - row_t.to_idx() + 1;
                let tile_b;
                let tile_t;
                if self.columns[col].kind == ColumnKind::Clb {
                    let c = clut[col];
                    tile_b = format!("PTERMR{rb}C{c}");
                    tile_t = format!("PTERMR{rt}C{c}");
                } else {
                    let c = bramclut[col];
                    tile_b = format!("PTERMBR{rb}BRAMC{c}");
                    tile_t = format!("PTERMTR{rt}BRAMC{c}");
                }
                grid[(col, row_b)].add_xnode(
                    db.get_node("PPC.N"),
                    &[&tile_b, &tile_t],
                    db.get_node_naming("PPC.N"),
                    &[(col, row_b), (col, row_t)],
                );
                grid[(col, row_t)].add_xnode(
                    db.get_node("PPC.S"),
                    &[&tile_t, &tile_b],
                    db.get_node_naming("PPC.S"),
                    &[(col, row_t), (col, row_b)],
                );
                grid.fill_term_pair_dbuf(
                    (col, row_b),
                    (col, row_t),
                    db.get_term("PPC.N"),
                    db.get_term("PPC.S"),
                    tile_b,
                    tile_t,
                    db.get_term_naming("PPC.N"),
                    db.get_term_naming("PPC.S"),
                );
            }
            for dr in 0..16 {
                let row = br + dr;
                for dc in 0..10 {
                    let col = bc + dc;
                    let tile = &mut grid[(col, row)];
                    if tile.nodes.is_empty() {
                        continue;
                    }
                    let name = tile.nodes[0].names[def_rt].clone();
                    let nname = db.node_namings.key(tile.nodes[0].naming);
                    tile.add_intf(db.get_intf("PPC"), name, db.get_intf_naming(&nname[4..]));
                }
            }
            let (kind, name, site) = if bc < self.col_clk {
                ("LBPPC", "PPC_X0Y0", "PPC405_X0Y0")
            } else if self.holes_ppc.len() == 1 {
                ("RBPPC", "PPC_X0Y0", "PPC405_X0Y0")
            } else {
                ("RBPPC", "PPC_X1Y0", "PPC405_X1Y0")
            };
            let node = grid[(bc, br)].add_xnode(
                db.get_node(kind),
                &[name],
                db.get_node_naming(kind),
                &ints,
            );
            node.add_bel(0, site.to_string());
        }

        for (gx, (&col, &(bbank, tbank))) in self.cols_gt.iter().enumerate() {
            if self.kind == GridKind::Virtex2PX {
                grid.nuke_rect(col, row_b, 1, 9);
                grid.nuke_rect(col, row_t - 8, 1, 9);
            } else {
                grid.nuke_rect(col, row_b, 1, 5);
                grid.nuke_rect(col, row_t - 4, 1, 5);
            }
            let c = bramclut[col];
            for row in [row_b, row_t] {
                let bt = if row == row_b { 'B' } else { 'T' };
                let name = format!("{bt}IOIBRAMC{c}");
                grid.fill_tile_special((col, row), "INT.GT.CLKPAD", "INT.GT.CLKPAD", name.clone());
                grid[(col, row)].add_intf(
                    db.get_intf("GT.CLKPAD"),
                    name,
                    db.get_intf_naming("GT.CLKPAD"),
                );
            }
            let n = match self.kind {
                GridKind::Virtex2P => 4,
                GridKind::Virtex2PX => 8,
                _ => unreachable!(),
            };
            for br in [row_b + 1, row_t - n] {
                for d in 0..n {
                    let row = br + d;
                    let r = row_t.to_idx() - row.to_idx();
                    let name = format!("BRAMR{r}C{c}");
                    grid.fill_tile_special((col, row), "INT.PPC", "INT.GT", name.clone());
                    grid[(col, row)].add_intf(
                        db.get_intf(if d % 4 == 0 { "GT.0" } else { "GT.123" }),
                        name,
                        db.get_intf_naming("GT"),
                    );
                }
            }
            let r = row_t.to_idx() - row_b.to_idx() - 1;
            let node_b;
            let node_t;
            if self.kind == GridKind::Virtex2P {
                node_b = grid[(col, row_b)].add_xnode(
                    db.get_node("GIGABIT"),
                    &[&format!("BMR{r}C{c}")],
                    db.get_node_naming("GIGABIT.B"),
                    &[
                        (col, row_b),
                        (col, row_b + 1),
                        (col, row_b + 2),
                        (col, row_b + 3),
                        (col, row_b + 4),
                    ],
                );
                node_b.add_bel(0, format!("GT_X{gx}Y0"));
            } else {
                node_b = grid[(col, row_b)].add_xnode(
                    db.get_node("GIGABIT10"),
                    &[&format!("BMR{r}C{c}")],
                    db.get_node_naming("GIGABIT10.B"),
                    &[
                        (col, row_b),
                        (col, row_b + 1),
                        (col, row_b + 2),
                        (col, row_b + 3),
                        (col, row_b + 4),
                        (col, row_b + 5),
                        (col, row_b + 6),
                        (col, row_b + 7),
                        (col, row_b + 8),
                    ],
                );
                node_b.add_bel(0, format!("GT10_X{gx}Y0"));
            }
            node_b.add_bel(1, format!("RXPPAD{bbank}"));
            node_b.add_bel(2, format!("RXNPAD{bbank}"));
            node_b.add_bel(3, format!("TXPPAD{bbank}"));
            node_b.add_bel(4, format!("TXNPAD{bbank}"));
            if self.kind == GridKind::Virtex2P {
                node_t = grid[(col, row_t)].add_xnode(
                    db.get_node("GIGABIT"),
                    &[&format!("BMR4C{c}")],
                    db.get_node_naming("GIGABIT.T"),
                    &[
                        (col, row_t),
                        (col, row_t - 4),
                        (col, row_t - 3),
                        (col, row_t - 2),
                        (col, row_t - 1),
                    ],
                );
                node_t.add_bel(0, format!("GT_X{gx}Y1"));
            } else {
                node_t = grid[(col, row_t)].add_xnode(
                    db.get_node("GIGABIT10"),
                    &[&format!("BMR8C{c}")],
                    db.get_node_naming("GIGABIT10.T"),
                    &[
                        (col, row_t),
                        (col, row_t - 8),
                        (col, row_t - 7),
                        (col, row_t - 6),
                        (col, row_t - 5),
                        (col, row_t - 4),
                        (col, row_t - 3),
                        (col, row_t - 2),
                        (col, row_t - 1),
                    ],
                );
                node_t.add_bel(0, format!("GT10_X{gx}Y1"));
            }
            node_t.add_bel(1, format!("RXPPAD{tbank}"));
            node_t.add_bel(2, format!("RXNPAD{tbank}"));
            node_t.add_bel(3, format!("TXPPAD{tbank}"));
            node_t.add_bel(4, format!("TXNPAD{tbank}"));
        }

        if self.has_ll {
            for col in self.columns.ids() {
                if matches!(self.columns[col].kind, ColumnKind::BramCont(_)) {
                    continue;
                }
                let mut row_s = self.row_mid() - 1;
                let mut row_n = self.row_mid();
                while grid[(col, row_s)].nodes.is_empty() {
                    row_s -= 1;
                }
                while grid[(col, row_n)].nodes.is_empty() {
                    row_n += 1;
                }
                let mut term_s = db.get_term("LLV.S");
                let mut term_n = db.get_term("LLV.N");
                let mut naming = db.get_node_naming("LLV");
                let mut tile;
                let x = xlut[col];
                let y = self.row_mid().to_idx() - 1;
                if col == col_l || col == col_r {
                    if col == col_l {
                        naming = db.get_node_naming("LLV.CLKL");
                        tile = format!("CLKL_IOIS_LL_X{x}Y{y}");
                    } else {
                        naming = db.get_node_naming("LLV.CLKR");
                        tile = format!("CLKR_IOIS_LL_X{x}Y{y}");
                    }
                    if self.kind != GridKind::Spartan3A {
                        term_s = db.get_term("LLV.CLKLR.S3E.S");
                        term_n = db.get_term("LLV.CLKLR.S3E.N");
                    }
                } else {
                    tile = format!("CLKH_LL_X{x}Y{y}");
                }
                if self.kind == GridKind::Spartan3E {
                    if col == col_l + 9 {
                        tile = format!("CLKLH_DCM_LL_X{x}Y{y}");
                    }
                    if col == col_r - 9 {
                        tile = format!("CLKRH_DCM_LL_X{x}Y{y}");
                    }
                } else {
                    if col == col_l + 3 {
                        tile = format!("CLKLH_DCM_LL_X{x}Y{y}");
                    }
                    if col == col_r - 6 {
                        tile = format!("CLKRH_DCM_LL_X{x}Y{y}");
                    }
                    if [col_l + 1, col_l + 2, col_r - 2, col_r - 1]
                        .into_iter()
                        .any(|x| x == col)
                    {
                        tile = format!("CLKH_DCM_LL_X{x}Y{y}");
                    }
                }
                grid.fill_term_pair_anon((col, row_s), (col, row_n), term_n, term_s);
                grid[(col, row_n)].add_xnode(
                    db.get_node("LLV"),
                    &[&tile],
                    naming,
                    &[(col, row_n), (col, row_s)],
                );
            }
            for row in self.rows.ids() {
                let mut col_l = self.col_clk - 1;
                let mut col_r = self.col_clk;
                while grid[(col_l, row)].nodes.is_empty() {
                    col_l -= 1;
                }
                while grid[(col_r, row)].nodes.is_empty() {
                    col_r += 1;
                }
                let x = xlut[self.col_clk - 1];
                let y = row.to_idx();
                let mut term_w = db.get_term("LLH.W");
                let mut term_e = db.get_term("LLH.E");
                let tile = if row == row_b {
                    format!("CLKB_LL_X{x}Y{y}")
                } else if row == row_t {
                    format!("CLKT_LL_X{x}Y{y}")
                } else if self.kind != GridKind::Spartan3E
                    && [
                        row_b + 2,
                        row_b + 3,
                        row_b + 4,
                        row_t - 4,
                        row_t - 3,
                        row_t - 2,
                    ]
                    .into_iter()
                    .any(|x| x == row)
                {
                    if self.kind == GridKind::Spartan3ADsp {
                        term_w = db.get_term("LLH.DCM.S3ADSP.W");
                        term_e = db.get_term("LLH.DCM.S3ADSP.E");
                    }
                    format!("CLKV_DCM_LL_X{x}Y{y}")
                } else {
                    format!("CLKV_LL_X{x}Y{y}")
                };
                grid.fill_term_pair_anon((col_l, row), (col_r, row), term_e, term_w);
                grid[(col_r, row)].add_xnode(
                    db.get_node("LLH"),
                    &[&tile],
                    db.get_node_naming("LLH"),
                    &[(col_r, row), (col_l, row)],
                );
            }
        }
        if self.kind == GridKind::Spartan3E && !self.has_ll {
            let term_s = db.get_term("CLKLR.S3E.S");
            let term_n = db.get_term("CLKLR.S3E.N");
            for col in [col_l, col_r] {
                grid.fill_term_pair_anon(
                    (col, self.row_mid() - 1),
                    (col, self.row_mid()),
                    term_n,
                    term_s,
                );
            }
        }
        if self.kind == GridKind::Spartan3 && !rows_brk.is_empty() {
            let term_s = db.get_term("BRKH.S3.S");
            let term_n = db.get_term("BRKH.S3.N");
            for &row_s in &rows_brk {
                let row_n = row_s + 1;
                for col in grid.cols() {
                    grid.fill_term_pair_anon((col, row_s), (col, row_n), term_n, term_s);
                }
            }
        }
        if self.kind == GridKind::Spartan3ADsp {
            let dsphole_e = db.get_term("DSPHOLE.E");
            let dsphole_w = db.get_term("DSPHOLE.W");
            let hdcm_e = db.get_term("HDCM.E");
            let hdcm_w = db.get_term("HDCM.W");
            for (col, cd) in &self.columns {
                if cd.kind == ColumnKind::Dsp {
                    for row in [row_b, row_t] {
                        grid.fill_term_pair_anon((col, row), (col + 1, row), dsphole_e, dsphole_w);
                    }
                }
            }
            for col in [col_l + 3, col_r - 6] {
                for row in [self.row_mid() - 1, self.row_mid()] {
                    grid.fill_term_pair_anon((col, row), (col + 4, row), dsphole_e, dsphole_w);
                }
                for row in [
                    self.row_mid() - 4,
                    self.row_mid() - 3,
                    self.row_mid() - 2,
                    self.row_mid() + 1,
                    self.row_mid() + 2,
                    self.row_mid() + 3,
                ] {
                    grid.fill_term_pair_anon((col - 1, row), (col + 4, row), hdcm_e, hdcm_w);
                }
            }
        }
        grid.fill_main_passes();

        if self.kind.is_virtex2() {
            for (col, cd) in &self.columns {
                if !matches!(cd.kind, ColumnKind::Bram) {
                    continue;
                }
                for row in self.rows.ids() {
                    if row.to_idx() % 4 != 1 {
                        continue;
                    }
                    if row.to_idx() == 1 {
                        continue;
                    }
                    if row == row_t {
                        continue;
                    }
                    let et = &mut grid[(col, row)];
                    if et.nodes.is_empty() {
                        continue;
                    }
                    if et.nodes[0].special {
                        continue;
                    }
                    if let Some(ref mut p) = et.terms[Dir::S] {
                        p.naming = Some(db.get_term_naming("BRAM.S"));
                        let c = bramclut[col];
                        let r = row_t.to_idx() - row.to_idx();
                        p.tile = Some(format!("BMR{r}C{c}"));
                    } else {
                        unreachable!();
                    }
                }
            }
        }

        if matches!(self.kind, GridKind::Spartan3A | GridKind::Spartan3ADsp) {
            for (col, cd) in &self.columns {
                if matches!(cd.kind, ColumnKind::BramCont(_)) {
                    grid[(col, row_b)].terms[Dir::N] = None;
                    grid[(col, row_t)].terms[Dir::S] = None;
                }
            }
        }

        xtmp = 0;
        if matches!(
            self.kind,
            GridKind::Spartan3E | GridKind::Spartan3A | GridKind::Spartan3ADsp
        ) {
            xtmp += 1;
        }
        let mut vcc_xlut = EntityVec::new();
        for col in self.columns.ids() {
            vcc_xlut.push(xtmp);
            if col == self.col_clk - 1 {
                xtmp += 2;
            } else {
                xtmp += 1;
            }
        }
        xtmp = 0;
        if matches!(
            self.kind,
            GridKind::Spartan3E | GridKind::Spartan3A | GridKind::Spartan3ADsp
        ) {
            xtmp += 1;
        }
        let mut vcc_ylut = EntityVec::new();
        for row in self.rows.ids() {
            vcc_ylut.push(xtmp);
            if row == self.row_mid() - 1
                && matches!(
                    self.kind,
                    GridKind::Spartan3E | GridKind::Spartan3A | GridKind::Spartan3ADsp
                )
            {
                xtmp += 2;
            } else {
                xtmp += 1;
            }
        }
        for col in self.columns.ids() {
            for row in self.rows.ids() {
                let tile = &mut grid[(col, row)];
                if tile.nodes.is_empty() {
                    continue;
                }
                let x = col.to_idx();
                let y = row.to_idx();
                tile.nodes[0].add_bel(0, format!("RLL_X{x}Y{y}"));
                if db.nodes.key(tile.nodes[0].kind) == "INT.DCM.S3E.DUMMY" {
                    continue;
                }
                let mut x = vcc_xlut[col];
                let mut y = vcc_ylut[row];
                if self.kind == GridKind::Virtex2 {
                    // Look, just..... don't ask me.
                    x = col.to_idx();
                    if col == col_l {
                        if row == row_b {
                            y = self.rows.len() - 2;
                        } else if row == row_t {
                            y = self.rows.len() - 1;
                        } else {
                            y -= 1;
                        }
                    } else if col == col_r {
                        if row == row_b {
                            y = 0;
                            x += 1;
                        } else if row == row_t {
                            y = 1;
                            x += 1;
                        } else {
                            y += 1;
                        }
                    } else if col < self.col_clk {
                        if row == row_b {
                            y = 0;
                        } else if row == row_t {
                            y = 1;
                        } else {
                            y += 1;
                        }
                    } else {
                        if row == row_b {
                            y = 2;
                        } else if row == row_t {
                            y = 3;
                        } else {
                            y += 3;
                            if y >= self.rows.len() {
                                y -= self.rows.len();
                                x += 1;
                            }
                        }
                    }
                }
                tile.nodes[0].tie_name = Some(format!("VCC_X{x}Y{y}"));
            }
        }

        if self.kind.is_virtex2() {
            let (kind_b, kind_t) = match self.kind {
                GridKind::Virtex2 => ("CLKB", "CLKT"),
                GridKind::Virtex2P => ("ML_CLKB", "ML_CLKT"),
                GridKind::Virtex2PX => ("MK_CLKB", "MK_CLKT"),
                _ => unreachable!(),
            };
            let vx = vcc_xlut[self.col_clk] - 1;
            let vyb = row_b.to_idx();
            let node = grid[(self.col_clk - 1, row_b)].add_xnode(
                db.get_node(kind_b),
                &[kind_b],
                db.get_node_naming(kind_b),
                &[(self.col_clk - 1, row_b), (self.col_clk, row_b)],
            );
            node.tie_name = Some(format!("VCC_X{vx}Y{vyb}"));
            node.add_bel(0, "BUFGMUX0P".to_string());
            node.add_bel(1, "BUFGMUX1S".to_string());
            node.add_bel(2, "BUFGMUX2P".to_string());
            node.add_bel(3, "BUFGMUX3S".to_string());
            node.add_bel(4, "BUFGMUX4P".to_string());
            node.add_bel(5, "BUFGMUX5S".to_string());
            node.add_bel(6, "BUFGMUX6P".to_string());
            node.add_bel(7, "BUFGMUX7S".to_string());
            node.add_bel(8, format!("GSIG_X{x}Y0", x = self.col_clk.to_idx()));
            node.add_bel(9, format!("GSIG_X{x}Y0", x = self.col_clk.to_idx() + 1));
            let vyt = if self.kind == GridKind::Virtex2 {
                1
            } else {
                row_t.to_idx()
            };
            let node = grid[(self.col_clk - 1, row_t)].add_xnode(
                db.get_node(kind_t),
                &[kind_t],
                db.get_node_naming(kind_t),
                &[(self.col_clk - 1, row_t), (self.col_clk, row_t)],
            );
            node.tie_name = Some(format!("VCC_X{vx}Y{vyt}"));
            node.add_bel(0, "BUFGMUX0S".to_string());
            node.add_bel(1, "BUFGMUX1P".to_string());
            node.add_bel(2, "BUFGMUX2S".to_string());
            node.add_bel(3, "BUFGMUX3P".to_string());
            node.add_bel(4, "BUFGMUX4S".to_string());
            node.add_bel(5, "BUFGMUX5P".to_string());
            node.add_bel(6, "BUFGMUX6S".to_string());
            node.add_bel(7, "BUFGMUX7P".to_string());
            node.add_bel(8, format!("GSIG_X{x}Y1", x = self.col_clk.to_idx()));
            node.add_bel(9, format!("GSIG_X{x}Y1", x = self.col_clk.to_idx() + 1));

            let rt = row_t.to_idx() - self.row_pci.unwrap().to_idx();
            let rb = row_t.to_idx() - self.row_pci.unwrap().to_idx() + 1;
            let node = grid[(col_l, self.row_pci.unwrap() - 2)].add_xnode(
                db.get_node("REG_L"),
                &[
                    if self.kind == GridKind::Virtex2 {
                        "HMLTERM"
                    } else {
                        "LTERMCLKH"
                    },
                    &format!("LTERMR{rb}"),
                    &format!("LTERMR{rt}"),
                ],
                db.get_node_naming("REG_L"),
                &[
                    (col_l, self.row_pci.unwrap() - 2),
                    (col_l, self.row_pci.unwrap() - 1),
                    (col_l, self.row_pci.unwrap()),
                    (col_l, self.row_pci.unwrap() + 1),
                ],
            );
            node.add_bel(0, "PCILOGIC_X0Y0".to_string());
            let node = grid[(col_r, self.row_pci.unwrap() - 2)].add_xnode(
                db.get_node("REG_R"),
                &[
                    if self.kind == GridKind::Virtex2 {
                        "HMRTERM"
                    } else {
                        "RTERMCLKH"
                    },
                    &format!("RTERMR{rb}"),
                    &format!("RTERMR{rt}"),
                ],
                db.get_node_naming("REG_R"),
                &[
                    (col_r, self.row_pci.unwrap() - 2),
                    (col_r, self.row_pci.unwrap() - 1),
                    (col_r, self.row_pci.unwrap()),
                    (col_r, self.row_pci.unwrap() + 1),
                ],
            );
            node.add_bel(0, "PCILOGIC_X1Y0".to_string());
        } else if self.kind == GridKind::Spartan3 {
            let vyb = 0;
            let vyt = vcc_ylut[row_t];
            let vx = vcc_xlut[self.col_clk] - 1;
            let node = grid[(self.col_clk - 1, row_b)].add_xnode(
                db.get_node("CLKB.S3"),
                &["CLKB"],
                db.get_node_naming("CLKB.S3"),
                &[(self.col_clk - 1, row_b)],
            );
            node.tie_name = Some(format!("VCC_X{vx}Y{vyb}"));
            node.add_bel(0, "BUFGMUX0".to_string());
            node.add_bel(1, "BUFGMUX1".to_string());
            node.add_bel(2, "BUFGMUX2".to_string());
            node.add_bel(3, "BUFGMUX3".to_string());
            node.add_bel(4, format!("GSIG_X{x}Y0", x = self.col_clk.to_idx()));
            let node = grid[(self.col_clk - 1, row_t)].add_xnode(
                db.get_node("CLKT.S3"),
                &["CLKT"],
                db.get_node_naming("CLKT.S3"),
                &[(self.col_clk - 1, row_t)],
            );
            node.tie_name = Some(format!("VCC_X{vx}Y{vyt}"));
            node.add_bel(0, "BUFGMUX4".to_string());
            node.add_bel(1, "BUFGMUX5".to_string());
            node.add_bel(2, "BUFGMUX6".to_string());
            node.add_bel(3, "BUFGMUX7".to_string());
            node.add_bel(4, format!("GSIG_X{x}Y1", x = self.col_clk.to_idx()));
        } else {
            let tile_b;
            let tile_t;
            let buf_b;
            let buf_t;
            let vyb = 0;
            let vyt = vcc_ylut[row_t];
            let x = xlut[self.col_clk - 1];
            let yb = row_b.to_idx();
            let ybb = yb + 1;
            let yt = row_t.to_idx();
            let ybt = yt - 1;
            if self.has_ll {
                tile_b = format!("CLKB_LL_X{x}Y{yb}");
                tile_t = format!("CLKT_LL_X{x}Y{yt}");
                buf_b = format!("CLKV_LL_X{x}Y{ybb}");
                buf_t = format!("CLKV_LL_X{x}Y{ybt}");
            } else {
                tile_b = format!("CLKB_X{x}Y{yb}");
                tile_t = format!("CLKT_X{x}Y{yt}");
                buf_b = format!("CLKV_X{x}Y{ybb}");
                buf_t = format!("CLKV_X{x}Y{ybt}");
            }
            let vx = vcc_xlut[self.col_clk] - 1;
            let kind_b = if self.kind == GridKind::Spartan3E {
                "CLKB.S3E"
            } else {
                "CLKB.S3A"
            };
            let node = grid[(self.col_clk - 1, row_b)].add_xnode(
                db.get_node(kind_b),
                &[&tile_b, &buf_b],
                db.get_node_naming(kind_b),
                &[(self.col_clk - 1, row_b)],
            );
            node.tie_name = Some(format!("VCC_X{vx}Y{vyb}"));
            node.add_bel(0, "BUFGMUX_X2Y1".to_string());
            node.add_bel(1, "BUFGMUX_X2Y0".to_string());
            node.add_bel(2, "BUFGMUX_X1Y1".to_string());
            node.add_bel(3, "BUFGMUX_X1Y0".to_string());
            node.add_bel(4, format!("GLOBALSIG_X{x}Y0", x = xlut[self.col_clk] + 1));
            let kind_t = if self.kind == GridKind::Spartan3E {
                "CLKT.S3E"
            } else {
                "CLKT.S3A"
            };
            let node = grid[(self.col_clk - 1, row_t)].add_xnode(
                db.get_node(kind_t),
                &[&tile_t, &buf_t],
                db.get_node_naming(kind_t),
                &[(self.col_clk - 1, row_t)],
            );
            node.tie_name = Some(format!("VCC_X{vx}Y{vyt}"));
            node.add_bel(0, "BUFGMUX_X2Y11".to_string());
            node.add_bel(1, "BUFGMUX_X2Y10".to_string());
            node.add_bel(2, "BUFGMUX_X1Y11".to_string());
            node.add_bel(3, "BUFGMUX_X1Y10".to_string());
            node.add_bel(
                4,
                format!(
                    "GLOBALSIG_X{x}Y{y}",
                    x = xlut[self.col_clk] + 1,
                    y = self.rows_hclk.len() + 2
                ),
            );

            let vy = vcc_ylut[self.row_mid()] - 1;
            let vxl = 0;
            let vxr = vcc_xlut[col_r] + 1;
            let xl = xlut[col_l];
            let xr = xlut[col_r];
            let y = self.row_mid().to_idx() - 1;
            let tile_l = format!("CLKL_X{xl}Y{y}");
            let tile_r = format!("CLKR_X{xr}Y{y}");
            let tile_l_ioi;
            let tile_r_ioi;
            if self.has_ll {
                tile_l_ioi = format!("CLKL_IOIS_LL_X{xl}Y{y}");
                tile_r_ioi = format!("CLKR_IOIS_LL_X{xr}Y{y}");
            } else if self.cols_clkv.is_none() {
                tile_l_ioi = format!("CLKL_IOIS_50A_X{xl}Y{y}");
                tile_r_ioi = format!("CLKR_IOIS_50A_X{xr}Y{y}");
            } else {
                tile_l_ioi = format!("CLKL_IOIS_X{xl}Y{y}");
                tile_r_ioi = format!("CLKR_IOIS_X{xr}Y{y}");
            }
            let tiles_l: Vec<&str>;
            let tiles_r: Vec<&str>;
            let kind_l;
            let kind_r;
            if self.kind == GridKind::Spartan3E {
                tiles_l = vec![&tile_l];
                tiles_r = vec![&tile_r];
                kind_l = "CLKL.S3E";
                kind_r = "CLKR.S3E";
            } else {
                tiles_l = vec![&tile_l, &tile_l_ioi];
                tiles_r = vec![&tile_r, &tile_r_ioi];
                kind_l = "CLKL.S3A";
                kind_r = "CLKR.S3A";
            }
            let gsy = (self.rows_hclk.len() + 1) / 2 + 1;
            let node = grid[(col_l, self.row_mid() - 1)].add_xnode(
                db.get_node(kind_l),
                &tiles_l,
                db.get_node_naming(kind_l),
                &[(col_l, self.row_mid() - 1), (col_l, self.row_mid())],
            );
            node.add_bel(0, "BUFGMUX_X0Y2".to_string());
            node.add_bel(1, "BUFGMUX_X0Y3".to_string());
            node.add_bel(2, "BUFGMUX_X0Y4".to_string());
            node.add_bel(3, "BUFGMUX_X0Y5".to_string());
            node.add_bel(4, "BUFGMUX_X0Y6".to_string());
            node.add_bel(5, "BUFGMUX_X0Y7".to_string());
            node.add_bel(6, "BUFGMUX_X0Y8".to_string());
            node.add_bel(7, "BUFGMUX_X0Y9".to_string());
            node.add_bel(8, "PCILOGIC_X0Y0".to_string());
            node.add_bel(9, format!("VCC_X{vxl}Y{vy}"));
            node.add_bel(10, format!("GLOBALSIG_X0Y{gsy}"));
            let node = grid[(col_r, self.row_mid() - 1)].add_xnode(
                db.get_node(kind_r),
                &tiles_r,
                db.get_node_naming(kind_r),
                &[(col_r, self.row_mid() - 1), (col_r, self.row_mid())],
            );
            node.add_bel(0, "BUFGMUX_X3Y2".to_string());
            node.add_bel(1, "BUFGMUX_X3Y3".to_string());
            node.add_bel(2, "BUFGMUX_X3Y4".to_string());
            node.add_bel(3, "BUFGMUX_X3Y5".to_string());
            node.add_bel(4, "BUFGMUX_X3Y6".to_string());
            node.add_bel(5, "BUFGMUX_X3Y7".to_string());
            node.add_bel(6, "BUFGMUX_X3Y8".to_string());
            node.add_bel(7, "BUFGMUX_X3Y9".to_string());
            node.add_bel(8, "PCILOGIC_X1Y0".to_string());
            node.add_bel(9, format!("VCC_X{vxr}Y{vy}"));
            node.add_bel(10, format!("GLOBALSIG_X{x}Y{gsy}", x = xlut[col_r] + 3));
        }

        if self.kind.is_virtex2() || self.kind == GridKind::Spartan3 {
            let mut c = 1;
            for (col, &cd) in self.columns.iter() {
                if cd.kind != ColumnKind::Bram {
                    continue;
                }
                if self.kind == GridKind::Spartan3 && (col == col_l + 3 || col == col_r - 3) {
                    c += 1;
                    continue;
                }
                let name_b = format!("BTERMBRAMC{c}");
                let name_t = format!("TTERMBRAMC{c}");
                grid[(col, row_b)].add_xnode(
                    db.get_node("DCMCONN.BOT"),
                    &[&name_b],
                    db.get_node_naming("DCMCONN.BOT"),
                    &[(col, row_b)],
                );
                grid[(col, row_t)].add_xnode(
                    db.get_node("DCMCONN.TOP"),
                    &[&name_t],
                    db.get_node_naming("DCMCONN.TOP"),
                    &[(col, row_t)],
                );
                c += 1;
            }
        }

        if use_xy {
            for &(row, _, _) in &self.rows_hclk {
                let kind = if row > self.row_mid() {
                    "PCI_CE_N"
                } else {
                    "PCI_CE_S"
                };
                for col in [col_l, col_r] {
                    let x = xlut[col];
                    let y = row.to_idx() - 1;
                    let name = if row == self.row_mid() {
                        format!("GCLKH_{kind}_50A_X{x}Y{y}")
                    } else {
                        format!("GCLKH_{kind}_X{x}Y{y}")
                    };
                    grid[(col, row)].add_xnode(
                        db.get_node(kind),
                        &[&name],
                        db.get_node_naming(kind),
                        &[(col, row)],
                    );
                }
            }
            if self.kind == GridKind::Spartan3A {
                if let Some((col_l, col_r)) = self.cols_clkv {
                    for row in [row_b, row_t] {
                        let x = xlut[col_l] - 1;
                        let y = row.to_idx();
                        let name = format!("GCLKV_IOISL_X{x}Y{y}");
                        grid[(col_l, row)].add_xnode(
                            db.get_node("PCI_CE_E"),
                            &[&name],
                            db.get_node_naming("PCI_CE_E"),
                            &[(col_l, row)],
                        );
                        let x = xlut[col_r] - 1;
                        let name = format!("GCLKV_IOISR_X{x}Y{y}");
                        grid[(col_r, row)].add_xnode(
                            db.get_node("PCI_CE_W"),
                            &[&name],
                            db.get_node_naming("PCI_CE_W"),
                            &[(col_r, row)],
                        );
                    }
                }
            }
        }

        for col in grid.cols() {
            for (i, &(row_m, row_b, row_t)) in self.rows_hclk.iter().enumerate() {
                for r in row_b.to_idx()..row_m.to_idx() {
                    let row = RowId::from_idx(r);
                    grid[(col, row)].clkroot = (col, row_m - 1);
                }
                for r in row_m.to_idx()..row_t.to_idx() {
                    let row = RowId::from_idx(r);
                    grid[(col, row)].clkroot = (col, row_m);
                }
                let mut kind = "GCLKH";
                let mut naming = "GCLKH";
                let name = if self.kind.is_virtex2() || self.kind == GridKind::Spartan3 {
                    let mut r = self.rows_hclk.len() - i;
                    if self.columns[col].kind == ColumnKind::Bram {
                        let c = bramclut[col];
                        format!("GCLKHR{r}BRAMC{c}")
                    } else {
                        // *sigh*.
                        if self.kind == GridKind::Virtex2 && grid.cols().len() == 12 {
                            r -= 1;
                        }
                        let c = clut[col];
                        if self.columns[col].kind == ColumnKind::Io && self.kind.is_virtex2p() {
                            if col == self.col_left() {
                                format!("LIOICLKR{r}")
                            } else {
                                format!("RIOICLKR{r}")
                            }
                        } else {
                            format!("GCLKHR{r}C{c}")
                        }
                    }
                } else {
                    let tk = match self.columns[col].kind {
                        ColumnKind::Io => match row_m.cmp(&self.row_mid()) {
                            Ordering::Less => "GCLKH_PCI_CE_S",
                            Ordering::Equal => "GCLKH_PCI_CE_S_50A",
                            Ordering::Greater => "GCLKH_PCI_CE_N",
                        },
                        ColumnKind::BramCont(x) => {
                            if row_m == self.row_mid() {
                                naming = "GCLKH.BRAM";
                                [
                                    "BRAMSITE2_DN_GCLKH",
                                    "BRAM2_GCLKH_FEEDTHRU",
                                    "BRAM2_GCLKH_FEEDTHRUA",
                                ][x as usize - 1]
                            } else if i == 0 {
                                kind = "GCLKH.S";
                                naming = "GCLKH.BRAM.S";
                                if self.kind == GridKind::Spartan3E {
                                    [
                                        "BRAMSITE2_DN_GCLKH",
                                        "BRAM2_DN_GCLKH_FEEDTHRU",
                                        "BRAM2_DN_GCLKH_FEEDTHRUA",
                                    ][x as usize - 1]
                                } else {
                                    [
                                        "BRAMSITE2_DN_GCLKH",
                                        "BRAM2_GCLKH_FEEDTHRU",
                                        "BRAM2_GCLKH_FEEDTHRUA",
                                    ][x as usize - 1]
                                }
                            } else if i == self.rows_hclk.len() - 1 {
                                kind = "GCLKH.N";
                                naming = "GCLKH.BRAM.N";
                                if self.kind == GridKind::Spartan3E {
                                    [
                                        "BRAMSITE2_UP_GCLKH",
                                        "BRAM2_UP_GCLKH_FEEDTHRU",
                                        "BRAM2_UP_GCLKH_FEEDTHRUA",
                                    ][x as usize - 1]
                                } else {
                                    [
                                        "BRAMSITE2_UP_GCLKH",
                                        "BRAM2_GCLKH_FEEDTHRU",
                                        "BRAM2_GCLKH_FEEDTHRUA",
                                    ][x as usize - 1]
                                }
                            } else {
                                kind = "GCLKH.0";
                                naming = "GCLKH.0";
                                if self.kind == GridKind::Spartan3E {
                                    [
                                        "BRAMSITE2_MID_GCLKH",
                                        "BRAM2_MID_GCLKH_FEEDTHRU",
                                        "BRAM2_MID_GCLKH_FEEDTHRUA",
                                    ][x as usize - 1]
                                } else {
                                    [
                                        if self.kind != GridKind::Spartan3ADsp {
                                            "BRAMSITE2_GCLKH"
                                        } else if row_m < self.row_mid() {
                                            "BRAMSITE2_DN_GCLKH"
                                        } else {
                                            "BRAMSITE2_UP_GCLKH"
                                        },
                                        "BRAM2_GCLKH_FEEDTHRU",
                                        "BRAM2_GCLKH_FEEDTHRUA",
                                    ][x as usize - 1]
                                }
                            }
                        }
                        _ => "GCLKH",
                    };
                    let x = xlut[col];
                    let y = row_m.to_idx() - 1;
                    format!("{tk}_X{x}Y{y}")
                };
                let node = grid[(col, row_m - 1)].add_xnode(
                    db.get_node(kind),
                    &[&name],
                    db.get_node_naming(naming),
                    &[(col, row_m - 1), (col, row_m)],
                );
                if self.kind.is_virtex2() || self.kind == GridKind::Spartan3 {
                    let gsx = if col < self.col_clk {
                        col.to_idx()
                    } else if self.kind == GridKind::Spartan3 {
                        col.to_idx() + 1
                    } else {
                        col.to_idx() + 2
                    };
                    let gsy = i;
                    node.add_bel(0, format!("GSIG_X{gsx}Y{gsy}"));
                } else {
                    let gsx = if col < self.col_clk {
                        xlut[col] + 1
                    } else {
                        xlut[col] + 2
                    };
                    let gsy = if row_m <= self.row_mid() {
                        i + 1
                    } else {
                        i + 2
                    };
                    node.add_bel(0, format!("GLOBALSIG_X{gsx}Y{gsy}"));
                    if self.columns[col].kind == ColumnKind::Dsp {
                        let name = format!(
                            "MACC2_GCLKH_FEEDTHRUA_X{x}Y{y}",
                            x = xlut[col] + 1,
                            y = row_m.to_idx() - 1
                        );
                        let node = grid[(col, row_m - 1)].add_xnode(
                            db.get_node("GCLKH.DSP"),
                            &[&name],
                            db.get_node_naming("GCLKH.DSP"),
                            &[(col, row_m - 1), (col, row_m)],
                        );
                        let gsxd = gsx + 1;
                        node.add_bel(0, format!("GLOBALSIG_X{gsxd}Y{gsy}"));
                    }
                }
            }
        }
        for (i, &(row_m, _, _)) in self.rows_hclk.iter().enumerate() {
            if self.kind.is_virtex2() {
                let mut r = self.rows_hclk.len() - i;
                if grid.cols().len() == 12 {
                    r -= 1;
                }
                let name = format!("GCLKCR{r}");
                grid[(self.col_clk, row_m)].add_xnode(
                    db.get_node("GCLKC"),
                    &[&name],
                    db.get_node_naming("GCLKC"),
                    &[(self.col_clk, row_m)],
                );
            } else if let Some((col_cl, col_cr)) = self.cols_clkv {
                let r = self.rows_hclk.len() - i;
                for (lr, col) in [('L', col_cl), ('R', col_cr)] {
                    let name = if self.kind == GridKind::Spartan3 {
                        format!("{lr}CLKVCR{r}")
                    } else {
                        let x = xlut[col] - 1;
                        let y = row_m.to_idx() - 1;
                        format!("GCLKVC_X{x}Y{y}")
                    };
                    grid[(col, row_m)].add_xnode(
                        db.get_node("GCLKVC"),
                        &[&name],
                        db.get_node_naming("GCLKVC"),
                        &[(col, row_m)],
                    );
                }
            }
        }

        {
            let kind = if !self.kind.is_virtex2() && self.cols_clkv.is_none() {
                "CLKC_50A"
            } else {
                "CLKC"
            };
            let name = if use_xy {
                let x = xlut[self.col_clk] - 1;
                let y = self.row_mid().to_idx() - 1;
                if self.kind == GridKind::Spartan3E && self.has_ll {
                    format!("{kind}_LL_X{x}Y{y}")
                } else {
                    format!("{kind}_X{x}Y{y}")
                }
            } else {
                "M".to_string()
            };
            grid[(self.col_clk, self.row_mid())].add_xnode(
                db.get_node(kind),
                &[&name],
                db.get_node_naming(kind),
                &[(self.col_clk, self.row_mid())],
            );
        }

        if let Some((col_cl, col_cr)) = self.cols_clkv {
            if self.kind == GridKind::Spartan3 {
                grid[(col_cl, self.row_mid())].add_xnode(
                    db.get_node("GCLKVM.S3"),
                    &["LGCLKVM"],
                    db.get_node_naming("GCLKVM.S3"),
                    &[(col_cl, self.row_mid())],
                );
                grid[(col_cr, self.row_mid())].add_xnode(
                    db.get_node("GCLKVM.S3"),
                    &["RGCLKVM"],
                    db.get_node_naming("GCLKVM.S3"),
                    &[(col_cr, self.row_mid())],
                );
            } else {
                let xl = xlut[col_cl] - 1;
                let xr = xlut[col_cr] - 1;
                let y = self.row_mid().to_idx() - 1;
                let name_l = format!("GCLKVML_X{xl}Y{y}");
                let name_r = format!("GCLKVMR_X{xr}Y{y}");
                grid[(col_cl, self.row_mid())].add_xnode(
                    db.get_node("GCLKVM.S3E"),
                    &[&name_l],
                    db.get_node_naming("GCLKVML"),
                    &[(col_cl, self.row_mid())],
                );
                grid[(col_cr, self.row_mid())].add_xnode(
                    db.get_node("GCLKVM.S3E"),
                    &[&name_r],
                    db.get_node_naming("GCLKVMR"),
                    &[(col_cr, self.row_mid())],
                );
            }
        }

        ExpandedDevice {
            grid: self,
            egrid,
            bonded_ios,
        }
    }
}

impl<'a> ExpandedDevice<'a> {
    pub fn get_io_node(&'a self, coord: Coord) -> Option<&'a ExpandedTileNode> {
        self.egrid.find_node(DieId::from_idx(0), coord, |x| {
            self.egrid.db.nodes.key(x.kind).starts_with("IOI")
        })
    }

    pub fn get_io_bel(
        &'a self,
        coord: Coord,
        bel: BelId,
    ) -> Option<(&'a ExpandedTileNode, &'a BelInfo, &'a BelNaming, &'a str)> {
        let node = self.get_io_node(coord)?;
        let nk = &self.egrid.db.nodes[node.kind];
        let naming = &self.egrid.db.node_namings[node.naming];
        Some((node, &nk.bels[bel], &naming.bels[bel], &node.bels[bel]))
    }

    pub fn get_io(&'a self, coord: Coord, bel: BelId) -> Io<'a> {
        let (_, _, _, name) = self.get_io_bel(coord, bel).unwrap();
        let bank = match self.grid.kind {
            GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX | GridKind::Spartan3 => {
                if coord.1 == self.grid.row_top() {
                    if coord.0 < self.grid.col_clk {
                        0
                    } else {
                        1
                    }
                } else if coord.0 == self.grid.col_right() {
                    if coord.1 < self.grid.row_mid() {
                        3
                    } else {
                        2
                    }
                } else if coord.1 == self.grid.row_bot() {
                    if coord.0 < self.grid.col_clk {
                        5
                    } else {
                        4
                    }
                } else if coord.0 == self.grid.col_left() {
                    if coord.1 < self.grid.row_mid() {
                        6
                    } else {
                        7
                    }
                } else {
                    unreachable!()
                }
            }
            GridKind::Spartan3E | GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                if coord.1 == self.grid.row_top() {
                    0
                } else if coord.0 == self.grid.col_right() {
                    1
                } else if coord.1 == self.grid.row_bot() {
                    2
                } else if coord.0 == self.grid.col_left() {
                    3
                } else {
                    unreachable!()
                }
            }
        };
        let diff = match self.grid.kind {
            GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
                if matches!(
                    self.grid.columns[coord.0].io,
                    ColumnIoKind::SingleLeftAlt | ColumnIoKind::SingleRightAlt
                ) {
                    match bel.to_idx() {
                        0 => IoDiffKind::None,
                        1 => IoDiffKind::P(BelId::from_idx(2)),
                        2 => IoDiffKind::N(BelId::from_idx(1)),
                        3 => IoDiffKind::None,
                        _ => unreachable!(),
                    }
                } else {
                    match bel.to_idx() {
                        0 => IoDiffKind::P(BelId::from_idx(1)),
                        1 => IoDiffKind::N(BelId::from_idx(0)),
                        2 => IoDiffKind::P(BelId::from_idx(3)),
                        3 => IoDiffKind::N(BelId::from_idx(2)),
                        _ => unreachable!(),
                    }
                }
            }
            GridKind::Spartan3 => {
                if coord.0 == self.grid.col_left() {
                    match bel.to_idx() {
                        0 => IoDiffKind::N(BelId::from_idx(1)),
                        1 => IoDiffKind::P(BelId::from_idx(0)),
                        2 => IoDiffKind::None,
                        _ => unreachable!(),
                    }
                } else {
                    match bel.to_idx() {
                        0 => IoDiffKind::P(BelId::from_idx(1)),
                        1 => IoDiffKind::N(BelId::from_idx(0)),
                        2 => IoDiffKind::None,
                        _ => unreachable!(),
                    }
                }
            }
            GridKind::Spartan3E => match bel.to_idx() {
                0 => IoDiffKind::P(BelId::from_idx(1)),
                1 => IoDiffKind::N(BelId::from_idx(0)),
                2 => IoDiffKind::None,
                _ => unreachable!(),
            },
            GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                if coord.1 == self.grid.row_top() || coord.0 == self.grid.col_left() {
                    match bel.to_idx() {
                        0 => IoDiffKind::N(BelId::from_idx(1)),
                        1 => IoDiffKind::P(BelId::from_idx(0)),
                        2 => IoDiffKind::None,
                        _ => unreachable!(),
                    }
                } else {
                    match bel.to_idx() {
                        0 => IoDiffKind::P(BelId::from_idx(1)),
                        1 => IoDiffKind::N(BelId::from_idx(0)),
                        2 => IoDiffKind::None,
                        _ => unreachable!(),
                    }
                }
            }
        };
        let pad_kind = if name.starts_with("PAD") {
            IoPadKind::Io
        } else if name.starts_with("IPAD") {
            IoPadKind::Input
        } else if name.starts_with("CLK") {
            IoPadKind::Clk
        } else {
            IoPadKind::None
        };
        let coord = IoCoord {
            col: coord.0,
            row: coord.1,
            bel,
        };
        Io {
            coord,
            bank,
            diff,
            pad_kind,
            name,
            is_vref: self.grid.vref.contains(&coord),
        }
    }

    pub fn get_bonded_ios(&'a self) -> Vec<Io<'a>> {
        let mut res = vec![];
        for &(coord, bel) in &self.bonded_ios {
            res.push(self.get_io(coord, bel));
        }
        res
    }
}
