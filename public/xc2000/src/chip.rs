use std::collections::{BTreeMap, BTreeSet};

use bincode::{Decode, Encode};
use itertools::Itertools;
use prjcombine_entity::{EntityId, EntityRange};
use prjcombine_interconnect::{
    dir::{DirH, DirHV, DirV},
    grid::{BelCoord, ColId, DieId, DieIdExt, EdgeIoCoord, RowId, TileCoord, TileIobId},
};

use crate::{xc2000, xc3000, xc4000, xc5200};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum SharedCfgPad {
    Addr(u8),
    Data(u8),
    Ldc,
    Hdc,
    RclkB,
    Dout,
    M2, // dedicated on XC4000
    // XC3000+
    InitB,
    Cs0B,
    Cs1B,
    // XC4000+
    Tck,
    Tdi,
    Tms,
    // XC5200 only
    Tdo,
    M0,
    M1,
}

impl std::fmt::Display for SharedCfgPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SharedCfgPad::Addr(i) => write!(f, "A{i}"),
            SharedCfgPad::Data(i) => write!(f, "D{i}"),
            SharedCfgPad::Ldc => write!(f, "LDC"),
            SharedCfgPad::Hdc => write!(f, "HDC"),
            SharedCfgPad::RclkB => write!(f, "RCLK_B"),
            SharedCfgPad::Dout => write!(f, "DOUT"),
            SharedCfgPad::M2 => write!(f, "M2"),
            SharedCfgPad::InitB => write!(f, "INIT_B"),
            SharedCfgPad::Cs0B => write!(f, "CS0_B"),
            SharedCfgPad::Cs1B => write!(f, "CS1_B"),
            SharedCfgPad::Tck => write!(f, "TCK"),
            SharedCfgPad::Tdi => write!(f, "TDI"),
            SharedCfgPad::Tms => write!(f, "TMS"),
            SharedCfgPad::Tdo => write!(f, "TDO"),
            SharedCfgPad::M0 => write!(f, "M0"),
            SharedCfgPad::M1 => write!(f, "M1"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum ChipKind {
    Xc2000,
    Xc3000,
    Xc3000A,
    // plain, D (no memory)
    Xc4000,
    Xc4000A,
    Xc4000H,
    // E, L, Spartan
    Xc4000E,
    // EX, XL
    Xc4000Ex,
    Xc4000Xla,
    Xc4000Xv,
    SpartanXl,
    Xc5200,
}

impl std::fmt::Display for ChipKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChipKind::Xc2000 => write!(f, "xc2000"),
            ChipKind::Xc3000 => write!(f, "xc3000"),
            ChipKind::Xc3000A => write!(f, "xc3000a"),
            ChipKind::Xc4000 => write!(f, "xc4000"),
            ChipKind::Xc4000A => write!(f, "xc4000a"),
            ChipKind::Xc4000H => write!(f, "xc4000h"),
            ChipKind::Xc4000E => write!(f, "xc4000e"),
            ChipKind::Xc4000Ex => write!(f, "xc4000ex"),
            ChipKind::Xc4000Xla => write!(f, "xc4000xla"),
            ChipKind::Xc4000Xv => write!(f, "xc4000xv"),
            ChipKind::SpartanXl => write!(f, "spartanxl"),
            ChipKind::Xc5200 => write!(f, "xc5200"),
        }
    }
}

impl ChipKind {
    pub fn is_xc3000(self) -> bool {
        matches!(self, Self::Xc3000 | Self::Xc3000A)
    }
    pub fn is_xc4000(self) -> bool {
        matches!(
            self,
            Self::Xc4000
                | Self::Xc4000A
                | Self::Xc4000H
                | Self::Xc4000E
                | Self::Xc4000Ex
                | Self::Xc4000Xla
                | Self::Xc4000Xv
                | Self::SpartanXl
        )
    }
    pub fn is_xl(self) -> bool {
        matches!(self, Self::Xc4000Ex | Self::Xc4000Xla | Self::Xc4000Xv)
    }
    pub fn is_clb_xl(self) -> bool {
        matches!(
            self,
            Self::SpartanXl | Self::Xc4000Ex | Self::Xc4000Xla | Self::Xc4000Xv
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Chip {
    pub kind: ChipKind,
    pub columns: usize,
    pub rows: usize,
    // XC3000 only
    pub is_small: bool,
    // XC4000X only
    pub is_buff_large: bool,
    // XC2000 only
    pub cols_bidi: BTreeSet<ColId>,
    pub rows_bidi: BTreeSet<RowId>,
    pub cfg_io: BTreeMap<SharedCfgPad, EdgeIoCoord>,
    pub unbonded_io: BTreeSet<EdgeIoCoord>,
}

impl Chip {
    // always single chip
    pub const DIE: DieId = DieId::from_idx_const(0);

    pub fn col_w(&self) -> ColId {
        ColId::from_idx(0)
    }

    pub fn col_e(&self) -> ColId {
        ColId::from_idx(self.columns - 1)
    }

    pub fn col_mid(&self) -> ColId {
        ColId::from_idx(self.columns / 2)
    }

    pub fn col_edge(&self, edge: DirH) -> ColId {
        match edge {
            DirH::W => self.col_w(),
            DirH::E => self.col_e(),
        }
    }

    pub fn row_s(&self) -> RowId {
        RowId::from_idx(0)
    }

    pub fn row_n(&self) -> RowId {
        RowId::from_idx(self.rows - 1)
    }

    pub fn row_mid(&self) -> RowId {
        RowId::from_idx(self.rows / 2)
    }

    pub fn row_edge(&self, edge: DirV) -> RowId {
        match edge {
            DirV::S => self.row_s(),
            DirV::N => self.row_n(),
        }
    }

    pub fn corner(&self, side: DirHV) -> TileCoord {
        Self::DIE
            .cell(self.col_edge(side.h), self.row_edge(side.v))
            .tile(match self.kind {
                ChipKind::Xc2000 => xc2000::tslots::MAIN,
                ChipKind::Xc3000 | ChipKind::Xc3000A => xc3000::tslots::MAIN,
                ChipKind::Xc4000
                | ChipKind::Xc4000A
                | ChipKind::Xc4000H
                | ChipKind::Xc4000E
                | ChipKind::Xc4000Ex
                | ChipKind::Xc4000Xla
                | ChipKind::Xc4000Xv
                | ChipKind::SpartanXl => xc4000::tslots::MAIN,
                ChipKind::Xc5200 => xc5200::tslots::MAIN,
            })
    }

    pub fn col_q(&self, side: DirH) -> ColId {
        match side {
            DirH::W => ColId::from_idx((self.columns + 2) / 4),
            DirH::E => ColId::from_idx(3 * self.columns / 4),
        }
    }

    pub fn row_q(&self, side: DirV) -> RowId {
        match side {
            DirV::S => RowId::from_idx((self.rows + 2) / 4),
            DirV::N => RowId::from_idx(3 * self.rows / 4),
        }
    }

    pub fn col_side_of_mid(&self, col: ColId) -> DirH {
        if col < self.col_mid() {
            DirH::W
        } else {
            DirH::E
        }
    }

    pub fn row_side_of_mid(&self, row: RowId) -> DirV {
        if row < self.row_mid() {
            DirV::S
        } else {
            DirV::N
        }
    }

    pub fn columns(&self) -> EntityRange<ColId> {
        EntityRange::new(0, self.columns)
    }

    pub fn rows(&self) -> EntityRange<RowId> {
        EntityRange::new(0, self.rows)
    }

    pub fn get_io_crd(&self, bel: BelCoord) -> EdgeIoCoord {
        match self.kind {
            ChipKind::Xc2000 => {
                if let Some(iob) = xc2000::bslots::IO_W.index_of(bel.slot) {
                    assert_eq!(bel.col, self.col_w());
                    EdgeIoCoord::W(bel.row, TileIobId::from_idx(iob))
                } else if let Some(iob) = xc2000::bslots::IO_E.index_of(bel.slot) {
                    assert_eq!(bel.col, self.col_e());
                    EdgeIoCoord::E(bel.row, TileIobId::from_idx(iob))
                } else if let Some(iob) = xc2000::bslots::IO_S.index_of(bel.slot) {
                    assert_eq!(bel.row, self.row_s());
                    EdgeIoCoord::S(bel.col, TileIobId::from_idx(iob))
                } else if let Some(iob) = xc2000::bslots::IO_N.index_of(bel.slot) {
                    assert_eq!(bel.row, self.row_n());
                    EdgeIoCoord::N(bel.col, TileIobId::from_idx(iob))
                } else {
                    unreachable!()
                }
            }
            ChipKind::Xc3000 | ChipKind::Xc3000A => {
                if let Some(iob) = xc3000::bslots::IO_W.index_of(bel.slot) {
                    assert_eq!(bel.col, self.col_w());
                    EdgeIoCoord::W(bel.row, TileIobId::from_idx(iob))
                } else if let Some(iob) = xc3000::bslots::IO_E.index_of(bel.slot) {
                    assert_eq!(bel.col, self.col_e());
                    EdgeIoCoord::E(bel.row, TileIobId::from_idx(iob))
                } else if let Some(iob) = xc3000::bslots::IO_S.index_of(bel.slot) {
                    assert_eq!(bel.row, self.row_s());
                    EdgeIoCoord::S(bel.col, TileIobId::from_idx(iob))
                } else if let Some(iob) = xc3000::bslots::IO_N.index_of(bel.slot) {
                    assert_eq!(bel.row, self.row_n());
                    EdgeIoCoord::N(bel.col, TileIobId::from_idx(iob))
                } else {
                    unreachable!()
                }
            }
            ChipKind::Xc4000
            | ChipKind::Xc4000A
            | ChipKind::Xc4000H
            | ChipKind::Xc4000E
            | ChipKind::Xc4000Ex
            | ChipKind::Xc4000Xla
            | ChipKind::Xc4000Xv
            | ChipKind::SpartanXl
            | ChipKind::Xc5200 => {
                let iob = match self.kind {
                    ChipKind::Xc4000H => TileIobId::from_idx(
                        xc4000::bslots::HIO
                            .iter()
                            .position(|&x| x == bel.slot)
                            .unwrap(),
                    ),
                    ChipKind::Xc5200 => TileIobId::from_idx(
                        xc5200::bslots::IO
                            .iter()
                            .position(|&x| x == bel.slot)
                            .unwrap(),
                    ),
                    _ => TileIobId::from_idx(
                        xc4000::bslots::IO
                            .iter()
                            .position(|&x| x == bel.slot)
                            .unwrap(),
                    ),
                };
                if bel.col == self.col_w() {
                    EdgeIoCoord::W(bel.row, iob)
                } else if bel.col == self.col_e() {
                    EdgeIoCoord::E(bel.row, iob)
                } else if bel.row == self.row_s() {
                    EdgeIoCoord::S(bel.col, iob)
                } else if bel.row == self.row_n() {
                    EdgeIoCoord::N(bel.col, iob)
                } else {
                    unreachable!()
                }
            }
        }
    }

    pub fn get_io_loc(&self, io: EdgeIoCoord) -> BelCoord {
        match self.kind {
            ChipKind::Xc2000 => match io {
                EdgeIoCoord::N(col, iob) => Self::DIE
                    .cell(col, self.row_n())
                    .bel(xc2000::bslots::IO_N[iob.to_idx()]),
                EdgeIoCoord::E(row, iob) => Self::DIE
                    .cell(self.col_e(), row)
                    .bel(xc2000::bslots::IO_E[iob.to_idx()]),
                EdgeIoCoord::S(col, iob) => Self::DIE
                    .cell(col, self.row_s())
                    .bel(xc2000::bslots::IO_S[iob.to_idx()]),
                EdgeIoCoord::W(row, iob) => Self::DIE
                    .cell(self.col_w(), row)
                    .bel(xc2000::bslots::IO_W[iob.to_idx()]),
            },
            ChipKind::Xc3000 | ChipKind::Xc3000A => match io {
                EdgeIoCoord::N(col, iob) => Self::DIE
                    .cell(col, self.row_n())
                    .bel(xc3000::bslots::IO_N[iob.to_idx()]),
                EdgeIoCoord::E(row, iob) => Self::DIE
                    .cell(self.col_e(), row)
                    .bel(xc3000::bslots::IO_E[iob.to_idx()]),
                EdgeIoCoord::S(col, iob) => Self::DIE
                    .cell(col, self.row_s())
                    .bel(xc3000::bslots::IO_S[iob.to_idx()]),
                EdgeIoCoord::W(row, iob) => Self::DIE
                    .cell(self.col_w(), row)
                    .bel(xc3000::bslots::IO_W[iob.to_idx()]),
            },
            ChipKind::Xc4000
            | ChipKind::Xc4000A
            | ChipKind::Xc4000H
            | ChipKind::Xc4000E
            | ChipKind::Xc4000Ex
            | ChipKind::Xc4000Xla
            | ChipKind::Xc4000Xv
            | ChipKind::SpartanXl
            | ChipKind::Xc5200 => {
                let (col, row, iob) = match io {
                    EdgeIoCoord::N(col, iob) => (col, self.row_n(), iob),
                    EdgeIoCoord::E(row, iob) => (self.col_e(), row, iob),
                    EdgeIoCoord::S(col, iob) => (col, self.row_s(), iob),
                    EdgeIoCoord::W(row, iob) => (self.col_w(), row, iob),
                };
                let slot = match self.kind {
                    ChipKind::Xc4000H => xc4000::bslots::HIO[iob.to_idx()],
                    ChipKind::Xc5200 => xc5200::bslots::IO[iob.to_idx()],
                    _ => xc4000::bslots::IO[iob.to_idx()],
                };
                Self::DIE.cell(col, row).bel(slot)
            }
        }
    }

    pub fn get_bonded_ios(&self) -> Vec<EdgeIoCoord> {
        let mut res = vec![];
        match self.kind {
            ChipKind::Xc2000 => {
                for col in self.columns() {
                    for iob in [0, 1] {
                        res.push(EdgeIoCoord::N(col, TileIobId::from_idx(iob)));
                    }
                }
                for row in self.rows().rev() {
                    if row == self.row_s() {
                        res.push(EdgeIoCoord::E(row, TileIobId::from_idx(0)));
                    } else if row == self.row_n() || row == self.row_mid() - 1 {
                        res.push(EdgeIoCoord::E(row, TileIobId::from_idx(1)));
                    } else {
                        for iob in [0, 1] {
                            res.push(EdgeIoCoord::E(row, TileIobId::from_idx(iob)));
                        }
                    }
                }
                for col in self.columns().rev() {
                    for iob in [1, 0] {
                        res.push(EdgeIoCoord::S(col, TileIobId::from_idx(iob)));
                    }
                }
                for row in self.rows() {
                    if row == self.row_s() {
                        res.push(EdgeIoCoord::W(row, TileIobId::from_idx(0)));
                    } else if row == self.row_n() || row == self.row_mid() - 1 {
                        res.push(EdgeIoCoord::W(row, TileIobId::from_idx(1)));
                    } else {
                        for iob in [1, 0] {
                            res.push(EdgeIoCoord::W(row, TileIobId::from_idx(iob)));
                        }
                    }
                }
            }
            ChipKind::Xc3000 | ChipKind::Xc3000A => {
                for col in self.columns() {
                    for iob in [0, 1] {
                        res.push(EdgeIoCoord::N(col, TileIobId::from_idx(iob)));
                    }
                }
                for row in self.rows().rev() {
                    for iob in [0, 1] {
                        res.push(EdgeIoCoord::E(row, TileIobId::from_idx(iob)));
                    }
                }
                for col in self.columns().rev() {
                    for iob in [1, 0] {
                        res.push(EdgeIoCoord::S(col, TileIobId::from_idx(iob)));
                    }
                }
                for row in self.rows() {
                    for iob in [1, 0] {
                        res.push(EdgeIoCoord::W(row, TileIobId::from_idx(iob)));
                    }
                }
            }
            ChipKind::Xc4000
            | ChipKind::Xc4000A
            | ChipKind::Xc4000H
            | ChipKind::Xc4000E
            | ChipKind::Xc4000Ex
            | ChipKind::Xc4000Xla
            | ChipKind::Xc4000Xv
            | ChipKind::SpartanXl => {
                let iobs = if self.kind == ChipKind::Xc4000H {
                    0..4
                } else {
                    0..2
                };
                for col in self.columns() {
                    if col == self.col_w() || col == self.col_e() {
                        continue;
                    }
                    for iob in iobs.clone() {
                        res.push(EdgeIoCoord::N(col, TileIobId::from_idx(iob)));
                    }
                }
                for row in self.rows().rev() {
                    if row == self.row_s() || row == self.row_n() {
                        continue;
                    }
                    for iob in iobs.clone() {
                        res.push(EdgeIoCoord::E(row, TileIobId::from_idx(iob)));
                    }
                }
                for col in self.columns().rev() {
                    if col == self.col_w() || col == self.col_e() {
                        continue;
                    }
                    for iob in iobs.clone().rev() {
                        res.push(EdgeIoCoord::S(col, TileIobId::from_idx(iob)));
                    }
                }
                for row in self.rows() {
                    if row == self.row_s() || row == self.row_n() {
                        continue;
                    }
                    for iob in iobs.clone().rev() {
                        res.push(EdgeIoCoord::W(row, TileIobId::from_idx(iob)));
                    }
                }
            }
            ChipKind::Xc5200 => {
                for col in self.columns() {
                    if col == self.col_w() || col == self.col_e() {
                        continue;
                    }
                    for iob in [3, 2, 1, 0] {
                        res.push(EdgeIoCoord::N(col, TileIobId::from_idx(iob)));
                    }
                }
                for row in self.rows().rev() {
                    if row == self.row_s() || row == self.row_n() {
                        continue;
                    }
                    for iob in [3, 2, 1, 0] {
                        res.push(EdgeIoCoord::E(row, TileIobId::from_idx(iob)));
                    }
                }
                for col in self.columns().rev() {
                    if col == self.col_w() || col == self.col_e() {
                        continue;
                    }
                    for iob in [0, 1, 2, 3] {
                        res.push(EdgeIoCoord::S(col, TileIobId::from_idx(iob)));
                    }
                }
                for row in self.rows() {
                    if row == self.row_s() || row == self.row_n() {
                        continue;
                    }
                    for iob in [0, 1, 2, 3] {
                        res.push(EdgeIoCoord::W(row, TileIobId::from_idx(iob)));
                    }
                }
            }
        }
        res
    }

    pub fn io_xtl1(&self) -> EdgeIoCoord {
        EdgeIoCoord::S(self.col_e(), TileIobId::from_idx(1))
    }

    pub fn io_xtl2(&self) -> EdgeIoCoord {
        EdgeIoCoord::E(self.row_s(), TileIobId::from_idx(0))
    }

    pub fn bel_clock_io(&self, idx: usize) -> BelCoord {
        match self.kind {
            ChipKind::Xc2000 => unreachable!(),
            ChipKind::Xc3000 | ChipKind::Xc3000A => [
                Self::DIE
                    .cell(self.col_w(), self.row_n())
                    .bel(xc3000::bslots::IO_W[0]),
                Self::DIE
                    .cell(self.col_e(), self.row_s())
                    .bel(xc3000::bslots::IO_E[0]),
            ][idx],
            ChipKind::Xc4000
            | ChipKind::Xc4000A
            | ChipKind::Xc4000E
            | ChipKind::Xc4000Ex
            | ChipKind::Xc4000Xla
            | ChipKind::Xc4000Xv
            | ChipKind::SpartanXl => {
                [
                    Self::DIE
                        .cell(self.col_w(), self.row_n() - 1)
                        .bel(xc4000::bslots::IO[0]),
                    Self::DIE
                        .cell(self.col_w(), self.row_s() + 1)
                        .bel(xc4000::bslots::IO[1]),
                    Self::DIE
                        .cell(self.col_w() + 1, self.row_s())
                        .bel(xc4000::bslots::IO[0]),
                    Self::DIE
                        .cell(self.col_e() - 1, self.row_s())
                        .bel(xc4000::bslots::IO[1]),
                    Self::DIE
                        .cell(self.col_e(), self.row_s() + 1)
                        .bel(xc4000::bslots::IO[0]), // skip one
                    Self::DIE
                        .cell(self.col_e(), self.row_n() - 1)
                        .bel(xc4000::bslots::IO[0]),
                    Self::DIE
                        .cell(self.col_e() - 1, self.row_n())
                        .bel(xc4000::bslots::IO[0]), // skip one
                    Self::DIE
                        .cell(self.col_w() + 1, self.row_n())
                        .bel(xc4000::bslots::IO[0]),
                ][idx]
            }
            ChipKind::Xc4000H => {
                [
                    Self::DIE
                        .cell(self.col_w(), self.row_n() - 1)
                        .bel(xc4000::bslots::HIO[0]),
                    Self::DIE
                        .cell(self.col_w(), self.row_s() + 1)
                        .bel(xc4000::bslots::HIO[3]),
                    Self::DIE
                        .cell(self.col_w() + 1, self.row_s())
                        .bel(xc4000::bslots::HIO[0]),
                    Self::DIE
                        .cell(self.col_e() - 1, self.row_s())
                        .bel(xc4000::bslots::HIO[3]),
                    Self::DIE
                        .cell(self.col_e(), self.row_s() + 1)
                        .bel(xc4000::bslots::HIO[2]), // skip one
                    Self::DIE
                        .cell(self.col_e(), self.row_n() - 1)
                        .bel(xc4000::bslots::HIO[0]),
                    Self::DIE
                        .cell(self.col_e() - 1, self.row_n())
                        .bel(xc4000::bslots::HIO[2]), // skip one
                    Self::DIE
                        .cell(self.col_w() + 1, self.row_n())
                        .bel(xc4000::bslots::HIO[0]),
                ][idx]
            }
            ChipKind::Xc5200 => {
                [
                    Self::DIE
                        .cell(self.col_w(), self.row_n() - 1)
                        .bel(xc5200::bslots::IO[1]), // skip two (2×unbonded)
                    Self::DIE
                        .cell(self.col_w() + 1, self.row_s())
                        .bel(xc5200::bslots::IO[2]), // skip one (M2)
                    Self::DIE
                        .cell(self.col_e(), self.row_s() + 1)
                        .bel(xc5200::bslots::IO[1]), // skip one (D7)
                    Self::DIE
                        .cell(self.col_e() - 2, self.row_n())
                        .bel(xc5200::bslots::IO[2]), // skip five (TDO, 2×unbonded, A0, unbonded)
                ][idx]
            }
        }
    }

    pub fn bel_buff_io(&self, which: DirHV) -> BelCoord {
        assert!(matches!(
            self.kind,
            ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv
        ));
        match (which, self.is_buff_large) {
            (DirHV::SW, _) => Self::DIE
                .cell(self.col_w(), self.row_q(DirV::S))
                .bel(xc4000::bslots::IO[1]),
            (DirHV::NW, _) => Self::DIE
                .cell(self.col_w(), self.row_q(DirV::N) - 1)
                .bel(xc4000::bslots::IO[0]),
            (DirHV::SE, false) => Self::DIE
                .cell(self.col_e(), self.row_q(DirV::S))
                .bel(xc4000::bslots::IO[1]),
            (DirHV::SE, true) => Self::DIE
                .cell(self.col_e(), self.row_q(DirV::S) + 1)
                .bel(xc4000::bslots::IO[1]),
            (DirHV::NE, false) => Self::DIE
                .cell(self.col_e(), self.row_q(DirV::N) - 1)
                .bel(xc4000::bslots::IO[0]),
            (DirHV::NE, true) => Self::DIE
                .cell(self.col_e(), self.row_q(DirV::N) - 2)
                .bel(xc4000::bslots::IO[0]),
        }
    }

    pub fn btile_height_main(&self, row: RowId) -> usize {
        if row == self.row_s() {
            match self.kind {
                ChipKind::Xc2000 => 12,
                ChipKind::Xc3000 | ChipKind::Xc3000A => 13,
                ChipKind::Xc4000 | ChipKind::Xc4000H | ChipKind::Xc4000E | ChipKind::SpartanXl => {
                    13
                }
                ChipKind::Xc4000A => 10,
                ChipKind::Xc4000Ex | ChipKind::Xc4000Xla => 16,
                ChipKind::Xc4000Xv => 17,
                ChipKind::Xc5200 => 28,
            }
        } else if row == self.row_n() {
            match self.kind {
                ChipKind::Xc2000 => 9,
                ChipKind::Xc3000 | ChipKind::Xc3000A => 10,
                ChipKind::Xc4000 | ChipKind::Xc4000H | ChipKind::Xc4000E | ChipKind::SpartanXl => 7,
                ChipKind::Xc4000A => 6,
                ChipKind::Xc4000Ex | ChipKind::Xc4000Xla => 8,
                ChipKind::Xc4000Xv => 9,
                ChipKind::Xc5200 => 28,
            }
        } else {
            match self.kind {
                ChipKind::Xc2000 => 8,
                ChipKind::Xc3000 | ChipKind::Xc3000A => 8,
                ChipKind::Xc4000 | ChipKind::Xc4000H | ChipKind::Xc4000E | ChipKind::SpartanXl => {
                    10
                }
                ChipKind::Xc4000A => 10,
                ChipKind::Xc4000Ex | ChipKind::Xc4000Xla => 12,
                ChipKind::Xc4000Xv => 13,
                ChipKind::Xc5200 => 34,
            }
        }
    }

    pub fn btile_height_clk(&self) -> usize {
        match self.kind {
            ChipKind::Xc2000 => unreachable!(),
            ChipKind::Xc3000 | ChipKind::Xc3000A => 1,
            ChipKind::Xc4000 | ChipKind::Xc4000A | ChipKind::Xc4000H | ChipKind::Xc4000E => 1,
            ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl => {
                2
            }
            ChipKind::Xc5200 => 4,
        }
    }

    pub fn btile_height_brk(&self) -> usize {
        if self.kind == ChipKind::Xc2000 { 1 } else { 2 }
    }

    pub fn btile_width_main(&self, col: ColId) -> usize {
        if col == self.col_w() {
            match self.kind {
                ChipKind::Xc2000 => 21,
                ChipKind::Xc3000 | ChipKind::Xc3000A => 29,
                ChipKind::Xc4000 | ChipKind::Xc4000H | ChipKind::Xc4000E | ChipKind::SpartanXl => {
                    26
                }
                ChipKind::Xc4000A => 21,
                ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv => 27,
                ChipKind::Xc5200 => 7,
            }
        } else if col == self.col_e() {
            match self.kind {
                ChipKind::Xc2000 => 27,
                ChipKind::Xc3000 | ChipKind::Xc3000A => 36,
                ChipKind::Xc4000 | ChipKind::Xc4000H | ChipKind::Xc4000E | ChipKind::SpartanXl => {
                    41
                }
                ChipKind::Xc4000A => 32,
                ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv => 52,
                ChipKind::Xc5200 => 8,
            }
        } else {
            match self.kind {
                ChipKind::Xc2000 => 18,
                ChipKind::Xc3000 | ChipKind::Xc3000A => 22,
                ChipKind::Xc4000 | ChipKind::Xc4000H | ChipKind::Xc4000E | ChipKind::SpartanXl => {
                    36
                }
                ChipKind::Xc4000A => 32,
                ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv => 47,
                ChipKind::Xc5200 => 12,
            }
        }
    }

    pub fn btile_width_clk(&self) -> usize {
        match self.kind {
            ChipKind::Xc2000 => unreachable!(),
            ChipKind::Xc3000 | ChipKind::Xc3000A => unreachable!(),
            ChipKind::Xc4000 | ChipKind::Xc4000H | ChipKind::Xc4000A | ChipKind::Xc4000E => 1,
            ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl => {
                2
            }
            ChipKind::Xc5200 => 1,
        }
    }

    pub fn btile_width_brk(&self) -> usize {
        if self.kind == ChipKind::Xc2000 { 2 } else { 1 }
    }

    pub fn bel_bufg(&self, idx: usize) -> BelCoord {
        assert!(self.kind.is_xc4000());
        let (col, row, slot) = [
            (self.col_w(), self.row_n(), xc4000::bslots::BUFG_V),
            (self.col_w(), self.row_s(), xc4000::bslots::BUFG_V),
            (self.col_w(), self.row_s(), xc4000::bslots::BUFG_H),
            (self.col_e(), self.row_s(), xc4000::bslots::BUFG_H),
            (self.col_e(), self.row_s(), xc4000::bslots::BUFG_V),
            (self.col_e(), self.row_n(), xc4000::bslots::BUFG_V),
            (self.col_e(), self.row_n(), xc4000::bslots::BUFG_H),
            (self.col_w(), self.row_n(), xc4000::bslots::BUFG_H),
        ][idx];
        Self::DIE.cell(col, row).bel(slot)
    }
}

impl Chip {
    pub fn dump(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(o, "\tkind {};", self.kind)?;
        writeln!(o, "\tcolumns {};", self.columns)?;
        writeln!(o, "\trows {};", self.rows)?;
        if self.is_small {
            writeln!(o, "\tsmall;")?;
        }
        if self.is_buff_large {
            writeln!(o, "\tbuff_large;")?;
        }
        if !self.cols_bidi.is_empty() {
            writeln!(
                o,
                "\tcols_bidi {};",
                self.cols_bidi.iter().map(|x| x.to_string()).join(", ")
            )?;
        }
        if !self.rows_bidi.is_empty() {
            writeln!(
                o,
                "\trows_bidi {};",
                self.rows_bidi.iter().map(|x| x.to_string()).join(", ")
            )?;
        }
        for (k, v) in &self.cfg_io {
            writeln!(o, "\tcfg_io {k} = {v};")?;
        }
        if !self.unbonded_io.is_empty() {
            writeln!(
                o,
                "\tunbonded_io {};",
                self.unbonded_io.iter().map(|x| x.to_string()).join(", ")
            )?;
        }
        Ok(())
    }
}
