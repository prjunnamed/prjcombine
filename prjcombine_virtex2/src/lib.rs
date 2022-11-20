#![allow(clippy::collapsible_else_if)]
#![allow(clippy::bool_to_int_with_if)]

use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};
use prjcombine_int::db::{BelId, BelInfo, BelNaming};
use prjcombine_int::grid::{ColId, Coord, DieId, ExpandedGrid, ExpandedTileNode, RowId};
use prjcombine_virtex_bitstream::BitstreamGeom;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod expand;

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
    pub fn is_spartan3ea(self) -> bool {
        matches!(self, Self::Spartan3E | Self::Spartan3A | Self::Spartan3ADsp)
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
    pub bs_geom: BitstreamGeom,
    pub clkv_frame: usize,
    pub spine_frame: usize,
    pub lterm_frame: usize,
    pub rterm_frame: usize,
    pub col_frame: EntityVec<ColId, usize>,
    pub bram_frame: EntityPartVec<ColId, usize>,
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
