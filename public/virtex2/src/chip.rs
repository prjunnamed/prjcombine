use jzon::JsonValue;
use prjcombine_interconnect::db::TileCellId;
use prjcombine_interconnect::dir::{Dir, DirH};
use prjcombine_interconnect::grid::{ColId, Coord, DieId, EdgeIoCoord, BelCoord, RowId, TileIobId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use unnamed_entity::{EntityId, EntityVec};

use crate::bels;
use crate::iob::{
    IobKind, IobTileData, get_iob_data_e, get_iob_data_n, get_iob_data_s, get_iob_data_w,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ChipKind {
    Virtex2,
    Virtex2P,
    Virtex2PX,
    Spartan3,
    FpgaCore,
    Spartan3E,
    Spartan3A,
    Spartan3ADsp,
}

impl ChipKind {
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

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Chip {
    pub kind: ChipKind,
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
    pub cfg_io: BTreeMap<SharedCfgPin, EdgeIoCoord>,
    pub dci_io: BTreeMap<u32, (EdgeIoCoord, EdgeIoCoord)>,
    pub dci_io_alt: BTreeMap<u32, (EdgeIoCoord, EdgeIoCoord)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Column {
    pub kind: ColumnKind,
    pub io: ColumnIoKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ColumnKind {
    Io,
    Clb,
    Bram,
    BramCont(u8),
    Dsp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
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
    DoubleRightClk(u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum RowIoKind {
    None,
    Single,
    Double(u8),
    Triple(u8),
    Quad(u8),
    DoubleBot(u8),
    DoubleTop(u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Dcms {
    Two,
    Four,
    Eight,
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

impl std::fmt::Display for SharedCfgPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SharedCfgPin::Data(i) => write!(f, "D{i}"),
            SharedCfgPin::Addr(i) => write!(f, "A{i}"),
            SharedCfgPin::CsiB => write!(f, "CSI_B"),
            SharedCfgPin::CsoB => write!(f, "CSO_B"),
            SharedCfgPin::RdWrB => write!(f, "RDWR_B"),
            SharedCfgPin::Dout => write!(f, "DOUT"),
            SharedCfgPin::InitB => write!(f, "INIT_B"),
            SharedCfgPin::Cclk => write!(f, "CCLK"),
            SharedCfgPin::M0 => write!(f, "M0"),
            SharedCfgPin::M1 => write!(f, "M1"),
            SharedCfgPin::M2 => write!(f, "M2"),
            SharedCfgPin::Ldc0 => write!(f, "LDC0"),
            SharedCfgPin::Ldc1 => write!(f, "LDC1"),
            SharedCfgPin::Ldc2 => write!(f, "LDC2"),
            SharedCfgPin::Hdc => write!(f, "HDC"),
            SharedCfgPin::HswapEn => write!(f, "HSWAP_EN"),
            SharedCfgPin::Awake => write!(f, "AWAKE"),
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum DcmPairKind {
    Bot,
    BotSingle,
    Top,
    TopSingle,
    // S3E
    Left,
    Right,
    // S3A
    Bram,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct DcmPair {
    pub kind: DcmPairKind,
    pub col: ColId,
    pub row: RowId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IoDiffKind {
    P(TileIobId),
    N(TileIobId),
    None,
}

#[derive(Clone, Copy, Debug)]
pub struct IoInfo {
    pub coord: EdgeIoCoord,
    pub bank: u32,
    pub diff: IoDiffKind,
    pub pad_kind: Option<IobKind>,
}

impl Chip {
    pub fn col_w(&self) -> ColId {
        self.columns.first_id().unwrap()
    }

    pub fn col_e(&self) -> ColId {
        self.columns.last_id().unwrap()
    }

    pub fn row_s(&self) -> RowId {
        self.rows.first_id().unwrap()
    }

    pub fn row_n(&self) -> RowId {
        self.rows.last_id().unwrap()
    }

    pub fn row_mid(&self) -> RowId {
        RowId::from_idx(self.rows.len() / 2)
    }

    pub fn bram_row(&self, row: RowId) -> Option<usize> {
        if let Some((b, t)) = self.rows_ram {
            if row <= b || row >= t {
                None
            } else {
                Some((row.to_idx() - (b.to_idx() + 1)) % 4)
            }
        } else {
            if row == self.row_s() || row == self.row_n() {
                None
            } else {
                Some((row.to_idx() - 1) % 4)
            }
        }
    }

    pub fn get_dcm_pairs(&self) -> Vec<DcmPair> {
        let mut res = vec![];
        if let Some(dcms) = self.dcms {
            if dcms == Dcms::Two {
                if self.kind == ChipKind::Spartan3E {
                    res.extend([
                        DcmPair {
                            kind: DcmPairKind::BotSingle,
                            col: self.col_clk,
                            row: self.row_s() + 1,
                        },
                        DcmPair {
                            kind: DcmPairKind::TopSingle,
                            col: self.col_clk,
                            row: self.row_n() - 1,
                        },
                    ]);
                } else {
                    res.extend([DcmPair {
                        kind: DcmPairKind::Top,
                        col: self.col_clk,
                        row: self.row_n() - 1,
                    }]);
                }
            } else {
                res.extend([
                    DcmPair {
                        kind: DcmPairKind::Bot,
                        col: self.col_clk,
                        row: self.row_s() + 1,
                    },
                    DcmPair {
                        kind: DcmPairKind::Top,
                        col: self.col_clk,
                        row: self.row_n() - 1,
                    },
                ]);
            }
            if dcms == Dcms::Eight {
                if self.kind == ChipKind::Spartan3E {
                    res.extend([
                        DcmPair {
                            kind: DcmPairKind::Left,
                            col: self.col_w() + 9,
                            row: self.row_mid(),
                        },
                        DcmPair {
                            kind: DcmPairKind::Right,
                            col: self.col_e() - 9,
                            row: self.row_mid(),
                        },
                    ]);
                } else {
                    res.extend([
                        DcmPair {
                            kind: DcmPairKind::Bram,
                            col: self.col_w() + 3,
                            row: self.row_mid(),
                        },
                        DcmPair {
                            kind: DcmPairKind::Bram,
                            col: self.col_e() - 6,
                            row: self.row_mid(),
                        },
                    ]);
                }
            }
        }
        res
    }

    pub fn get_io_info(&self, io: EdgeIoCoord) -> IoInfo {
        let bank = match self.kind {
            ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX | ChipKind::Spartan3 => {
                match io {
                    EdgeIoCoord::N(col, _) => {
                        if col < self.col_clk {
                            0
                        } else {
                            1
                        }
                    }
                    EdgeIoCoord::E(row, _) => {
                        if row < self.row_mid() {
                            3
                        } else {
                            2
                        }
                    }
                    EdgeIoCoord::S(col, _) => {
                        if col < self.col_clk {
                            5
                        } else {
                            4
                        }
                    }
                    EdgeIoCoord::W(row, _) => {
                        if row < self.row_mid() {
                            6
                        } else {
                            7
                        }
                    }
                }
            }
            ChipKind::FpgaCore => 0,
            ChipKind::Spartan3E | ChipKind::Spartan3A | ChipKind::Spartan3ADsp => match io {
                EdgeIoCoord::N(_, _) => 0,
                EdgeIoCoord::E(_, _) => 1,
                EdgeIoCoord::S(_, _) => 2,
                EdgeIoCoord::W(_, _) => 3,
            },
        };
        let iob = io.iob();
        let diff = match self.kind {
            ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => {
                let is_single = if let EdgeIoCoord::S(col, _) | EdgeIoCoord::N(col, _) = io {
                    matches!(
                        self.columns[col].io,
                        ColumnIoKind::SingleLeftAlt | ColumnIoKind::SingleRightAlt
                    )
                } else {
                    false
                };
                if is_single {
                    match iob.to_idx() {
                        0 => IoDiffKind::None,
                        1 => IoDiffKind::P(TileIobId::from_idx(2)),
                        2 => IoDiffKind::N(TileIobId::from_idx(1)),
                        3 => IoDiffKind::None,
                        _ => unreachable!(),
                    }
                } else {
                    match iob.to_idx() {
                        0 => IoDiffKind::P(TileIobId::from_idx(1)),
                        1 => IoDiffKind::N(TileIobId::from_idx(0)),
                        2 => IoDiffKind::P(TileIobId::from_idx(3)),
                        3 => IoDiffKind::N(TileIobId::from_idx(2)),
                        _ => unreachable!(),
                    }
                }
            }
            ChipKind::Spartan3 => {
                if matches!(io, EdgeIoCoord::W(..)) {
                    match iob.to_idx() {
                        0 => IoDiffKind::N(TileIobId::from_idx(1)),
                        1 => IoDiffKind::P(TileIobId::from_idx(0)),
                        2 => IoDiffKind::None,
                        _ => unreachable!(),
                    }
                } else {
                    match iob.to_idx() {
                        0 => IoDiffKind::P(TileIobId::from_idx(1)),
                        1 => IoDiffKind::N(TileIobId::from_idx(0)),
                        2 => IoDiffKind::None,
                        _ => unreachable!(),
                    }
                }
            }
            ChipKind::FpgaCore => IoDiffKind::None,
            ChipKind::Spartan3E => match iob.to_idx() {
                0 => IoDiffKind::P(TileIobId::from_idx(1)),
                1 => IoDiffKind::N(TileIobId::from_idx(0)),
                2 => IoDiffKind::None,
                _ => unreachable!(),
            },
            ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                if matches!(io, EdgeIoCoord::N(..) | EdgeIoCoord::W(..)) {
                    match iob.to_idx() {
                        0 => IoDiffKind::N(TileIobId::from_idx(1)),
                        1 => IoDiffKind::P(TileIobId::from_idx(0)),
                        2 => IoDiffKind::None,
                        _ => unreachable!(),
                    }
                } else {
                    match iob.to_idx() {
                        0 => IoDiffKind::P(TileIobId::from_idx(1)),
                        1 => IoDiffKind::N(TileIobId::from_idx(0)),
                        2 => IoDiffKind::None,
                        _ => unreachable!(),
                    }
                }
            }
        };
        let mut pad_kind = None;
        let (_, (col, row), _) = self.get_io_loc(io);
        if let Some((data, tidx)) = self.get_iob_tile_data((col, row)) {
            for &iob_data in &data.iobs {
                if iob_data.tile == tidx && iob_data.iob == iob {
                    pad_kind = Some(iob_data.kind);
                }
            }
        }
        IoInfo {
            coord: io,
            bank,
            diff,
            pad_kind,
        }
    }

    pub fn get_bonded_ios(&self) -> Vec<EdgeIoCoord> {
        let mut res = vec![];
        for col in self.columns.ids() {
            let row = self.row_n();
            if let Some((data, tidx)) = self.get_iob_tile_data((col, row)) {
                for &iob in &data.iobs {
                    if iob.tile == tidx {
                        res.push(EdgeIoCoord::N(col, TileIobId::from_idx(iob.iob.to_idx())));
                    }
                }
            }
        }
        for row in self.rows.ids().rev() {
            let col = self.col_e();
            if let Some((data, tidx)) = self.get_iob_tile_data((col, row)) {
                for &iob in &data.iobs {
                    if iob.tile == tidx {
                        res.push(EdgeIoCoord::E(row, TileIobId::from_idx(iob.iob.to_idx())));
                    }
                }
            }
        }
        for col in self.columns.ids().rev() {
            let row = self.row_s();
            if let Some((data, tidx)) = self.get_iob_tile_data((col, row)) {
                for &iob in &data.iobs {
                    if iob.tile == tidx {
                        res.push(EdgeIoCoord::S(col, TileIobId::from_idx(iob.iob.to_idx())));
                    }
                }
            }
        }
        for row in self.rows.ids() {
            let col = self.col_w();
            if let Some((data, tidx)) = self.get_iob_tile_data((col, row)) {
                for &iob in &data.iobs {
                    if iob.tile == tidx {
                        res.push(EdgeIoCoord::W(row, TileIobId::from_idx(iob.iob.to_idx())));
                    }
                }
            }
        }
        res
    }

    pub fn get_io_loc(&self, io: EdgeIoCoord) -> BelCoord {
        let (col, row, iob) = match io {
            EdgeIoCoord::N(col, iob) => (col, self.row_n(), iob),
            EdgeIoCoord::E(row, iob) => (self.col_e(), row, iob),
            EdgeIoCoord::S(col, iob) => (col, self.row_s(), iob),
            EdgeIoCoord::W(row, iob) => (self.col_w(), row, iob),
        };
        let slot = if self.kind == ChipKind::FpgaCore {
            if iob.to_idx() < 4 {
                bels::IBUF[iob.to_idx()]
            } else {
                bels::OBUF[iob.to_idx() - 4]
            }
        } else {
            bels::IO[iob.to_idx()]
        };
        (DieId::from_idx(0), (col, row), slot)
    }

    pub fn get_io_crd(&self, bel: BelCoord) -> EdgeIoCoord {
        let (_, (col, row), slot) = bel;
        let iob = TileIobId::from_idx(if self.kind == ChipKind::FpgaCore {
            bels::IBUF
                .iter()
                .position(|&x| x == slot)
                .unwrap_or_else(|| bels::OBUF.iter().position(|&x| x == slot).unwrap() + 4)
        } else {
            bels::IO.iter().position(|&x| x == slot).unwrap()
        });
        if col == self.col_w() {
            EdgeIoCoord::W(row, iob)
        } else if col == self.col_e() {
            EdgeIoCoord::E(row, iob)
        } else if row == self.row_s() {
            EdgeIoCoord::S(col, iob)
        } else if row == self.row_n() {
            EdgeIoCoord::N(col, iob)
        } else {
            unreachable!()
        }
    }

    pub fn get_clk_io(&self, edge: Dir, idx: usize) -> Option<EdgeIoCoord> {
        if self.kind.is_virtex2() {
            match edge {
                Dir::S => {
                    if self.kind == ChipKind::Virtex2PX && matches!(idx, 6 | 7) {
                        return None;
                    }
                    if idx < 4 {
                        Some(EdgeIoCoord::S(self.col_clk, TileIobId::from_idx(idx)))
                    } else if idx < 8 {
                        Some(EdgeIoCoord::S(
                            self.col_clk - 1,
                            TileIobId::from_idx(idx - 4),
                        ))
                    } else {
                        None
                    }
                }
                Dir::N => {
                    if self.kind == ChipKind::Virtex2PX && matches!(idx, 4 | 5) {
                        return None;
                    }
                    if idx < 4 {
                        Some(EdgeIoCoord::N(self.col_clk, TileIobId::from_idx(idx)))
                    } else if idx < 8 {
                        Some(EdgeIoCoord::N(
                            self.col_clk - 1,
                            TileIobId::from_idx(idx - 4),
                        ))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else if matches!(self.kind, ChipKind::Spartan3 | ChipKind::FpgaCore) {
            match edge {
                Dir::S => {
                    if idx < 2 {
                        Some(EdgeIoCoord::S(self.col_clk, TileIobId::from_idx(idx)))
                    } else if idx < 4 {
                        Some(EdgeIoCoord::S(
                            self.col_clk - 1,
                            TileIobId::from_idx(idx - 2),
                        ))
                    } else {
                        None
                    }
                }
                Dir::N => {
                    if idx < 2 {
                        Some(EdgeIoCoord::N(self.col_clk, TileIobId::from_idx(idx)))
                    } else if idx < 4 {
                        Some(EdgeIoCoord::N(
                            self.col_clk - 1,
                            TileIobId::from_idx(idx - 2),
                        ))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else if self.kind == ChipKind::Spartan3E {
            match (edge, idx) {
                (Dir::S, 0 | 1) => Some(EdgeIoCoord::S(self.col_clk, TileIobId::from_idx(idx % 2))),
                (Dir::S, 2 | 3) => Some(EdgeIoCoord::S(
                    self.col_clk + 1,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::S, 4 | 5) => Some(EdgeIoCoord::S(
                    self.col_clk - 3,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::S, 6 | 7) => Some(EdgeIoCoord::S(
                    self.col_clk - 1,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::N, 0 | 1) => Some(EdgeIoCoord::N(
                    self.col_clk + 2,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::N, 2 | 3) => Some(EdgeIoCoord::N(self.col_clk, TileIobId::from_idx(idx % 2))),
                (Dir::N, 4 | 5) => Some(EdgeIoCoord::N(
                    self.col_clk - 1,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::N, 6 | 7) => Some(EdgeIoCoord::N(
                    self.col_clk - 2,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::W, 0 | 1) => Some(EdgeIoCoord::W(
                    self.row_mid() + 3,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::W, 2 | 3) => Some(EdgeIoCoord::W(
                    self.row_mid() + 1,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::W, 4 | 5) => Some(EdgeIoCoord::W(
                    self.row_mid() - 1,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::W, 6 | 7) => Some(EdgeIoCoord::W(
                    self.row_mid() - 3,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::E, 0 | 1) => {
                    Some(EdgeIoCoord::E(self.row_mid(), TileIobId::from_idx(idx % 2)))
                }
                (Dir::E, 2 | 3) => Some(EdgeIoCoord::E(
                    self.row_mid() + 2,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::E, 4 | 5) => Some(EdgeIoCoord::E(
                    self.row_mid() - 4,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::E, 6 | 7) => Some(EdgeIoCoord::E(
                    self.row_mid() - 2,
                    TileIobId::from_idx(idx % 2),
                )),
                _ => None,
            }
        } else {
            match (edge, idx) {
                (Dir::S, 0 | 1) => Some(EdgeIoCoord::S(self.col_clk, TileIobId::from_idx(idx % 2))),
                (Dir::S, 2 | 3) => Some(EdgeIoCoord::S(
                    self.col_clk + 1,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::S, 4 | 5) => Some(EdgeIoCoord::S(
                    self.col_clk - 2,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::S, 6 | 7) => Some(EdgeIoCoord::S(
                    self.col_clk - 1,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::N, 0 | 1) => Some(EdgeIoCoord::N(
                    self.col_clk + 1,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::N, 2 | 3) => Some(EdgeIoCoord::N(self.col_clk, TileIobId::from_idx(idx % 2))),
                (Dir::N, 4 | 5) => Some(EdgeIoCoord::N(
                    self.col_clk - 1,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::N, 6 | 7) => Some(EdgeIoCoord::N(
                    self.col_clk - 2,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::W, 0 | 1) => Some(EdgeIoCoord::W(
                    self.row_mid() + 2,
                    TileIobId::from_idx((idx % 2) ^ 1),
                )),
                (Dir::W, 2 | 3) => Some(EdgeIoCoord::W(
                    self.row_mid() + 1,
                    TileIobId::from_idx((idx % 2) ^ 1),
                )),
                (Dir::W, 4 | 5) => Some(EdgeIoCoord::W(
                    self.row_mid() - 1,
                    TileIobId::from_idx((idx % 2) ^ 1),
                )),
                (Dir::W, 6 | 7) => Some(EdgeIoCoord::W(
                    self.row_mid() - 2,
                    TileIobId::from_idx((idx % 2) ^ 1),
                )),
                (Dir::E, 0 | 1) => {
                    Some(EdgeIoCoord::E(self.row_mid(), TileIobId::from_idx(idx % 2)))
                }
                (Dir::E, 2 | 3) => Some(EdgeIoCoord::E(
                    self.row_mid() + 1,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::E, 4 | 5) => Some(EdgeIoCoord::E(
                    self.row_mid() - 3,
                    TileIobId::from_idx(idx % 2),
                )),
                (Dir::E, 6 | 7) => Some(EdgeIoCoord::E(
                    self.row_mid() - 2,
                    TileIobId::from_idx(idx % 2),
                )),
                _ => None,
            }
        }
    }

    pub fn get_pci_io(&self, edge: DirH) -> [EdgeIoCoord; 2] {
        match self.kind {
            ChipKind::Spartan3E => match edge {
                DirH::W => [
                    EdgeIoCoord::W(self.row_mid() + 1, TileIobId::from_idx(1)),
                    EdgeIoCoord::W(self.row_mid() - 1, TileIobId::from_idx(0)),
                ],
                DirH::E => [
                    EdgeIoCoord::E(self.row_mid(), TileIobId::from_idx(0)),
                    EdgeIoCoord::E(self.row_mid() - 2, TileIobId::from_idx(1)),
                ],
            },
            ChipKind::Spartan3A | ChipKind::Spartan3ADsp => match edge {
                DirH::W => [
                    EdgeIoCoord::W(self.row_mid() + 1, TileIobId::from_idx(0)),
                    EdgeIoCoord::W(self.row_mid() - 2, TileIobId::from_idx(1)),
                ],
                DirH::E => [
                    EdgeIoCoord::E(self.row_mid() + 1, TileIobId::from_idx(0)),
                    EdgeIoCoord::E(self.row_mid() - 2, TileIobId::from_idx(1)),
                ],
            },
            _ => unreachable!(),
        }
    }

    pub fn get_iob_tile_data(&self, coord: Coord) -> Option<(IobTileData, TileCellId)> {
        if coord.0 == self.col_w() {
            let kind = self.rows[coord.1];
            if kind == RowIoKind::None {
                None
            } else {
                Some(get_iob_data_w(self.kind, kind))
            }
        } else if coord.0 == self.col_e() {
            let kind = self.rows[coord.1];
            if kind == RowIoKind::None {
                None
            } else {
                Some(get_iob_data_e(self.kind, kind))
            }
        } else if coord.1 == self.row_s() {
            let kind = self.columns[coord.0].io;
            if kind == ColumnIoKind::None {
                None
            } else {
                Some(get_iob_data_s(self.kind, kind))
            }
        } else if coord.1 == self.row_n() {
            let kind = self.columns[coord.0].io;
            if kind == ColumnIoKind::None {
                None
            } else {
                Some(get_iob_data_n(self.kind, kind))
            }
        } else {
            unreachable!()
        }
    }
}

impl From<&Chip> for JsonValue {
    fn from(chip: &Chip) -> Self {
        jzon::object! {
            kind: match chip.kind {
                ChipKind::Virtex2 => "virtex2",
                ChipKind::Virtex2P => "virtex2p",
                ChipKind::Virtex2PX => "virtex2px",
                ChipKind::Spartan3 => "spartan3",
                ChipKind::Spartan3E => "spartan3e",
                ChipKind::Spartan3A => "spartan3a",
                ChipKind::Spartan3ADsp => "spartan3adsp",
                ChipKind::FpgaCore => "fpgacore",
            },
            columns: Vec::from_iter(chip.columns.values().map(|column| {
                jzon::object! {
                    kind: match column.kind {
                        ColumnKind::Io => "IO".to_string(),
                        ColumnKind::Clb => "CLB".to_string(),
                        ColumnKind::Bram => "BRAM".to_string(),
                        ColumnKind::BramCont(i) => format!("BRAM_CONT:{i}"),
                        ColumnKind::Dsp => "DSP".to_string(),
                    },
                    io: match column.io {
                        ColumnIoKind::None => JsonValue::Null,
                        ColumnIoKind::Single => "SINGLE".into(),
                        ColumnIoKind::Double(i) => format!("DOUBLE:{i}").into(),
                        ColumnIoKind::Triple(i) => format!("TRIPLE:{i}").into(),
                        ColumnIoKind::Quad(i) => format!("QUAD:{i}").into(),
                        ColumnIoKind::SingleLeft => "SINGLE_LEFT".into(),
                        ColumnIoKind::SingleRight => "SINGLE_RIGHT".into(),
                        ColumnIoKind::SingleLeftAlt => "SINGLE_LEFT_ALT".into(),
                        ColumnIoKind::SingleRightAlt => "SINGLE_RIGHT_ALT".into(),
                        ColumnIoKind::DoubleLeft(i) => format!("DOUBLE_LEFT:{i}").into(),
                        ColumnIoKind::DoubleRight(i) => format!("DOUBLE_RIGHT:{i}").into(),
                        ColumnIoKind::DoubleRightClk(i) => format!("DOUBLE_RIGHT_CLK:{i}").into(),
                    },
                }
            })),
            cols_clkv: chip.cols_clkv.map(|(col_l, col_r)| jzon::array![col_l.to_idx(), col_r.to_idx()]),
            "cols_gt": Vec::from_iter(chip.cols_gt.iter().map(|(col, (bank_b, bank_t))| jzon::object! {
                column: col.to_idx(),
                bank_b: *bank_b,
                bank_t: *bank_t,
            })),
            rows: Vec::from_iter(chip.rows.values().map(|io| match io {
                RowIoKind::None => JsonValue::Null,
                RowIoKind::Single => "SINGLE".into(),
                RowIoKind::Double(i) => format!("DOUBLE:{i}").into(),
                RowIoKind::Triple(i) => format!("TRIPLE:{i}").into(),
                RowIoKind::Quad(i) => format!("QUAD:{i}").into(),
                RowIoKind::DoubleBot(i) => format!("DOUBLE_BOT:{i}").into(),
                RowIoKind::DoubleTop(i) => format!("DOUBLE_TOP:{i}").into(),
            })),
            rows_ram: chip.rows_ram.map(|(row_b, row_t)| jzon::array![row_b.to_idx(), row_t.to_idx()]),
            rows_hclk: Vec::from_iter(chip.rows_hclk.iter().map(|(row_mid, row_start, row_end)| {
                jzon::array![row_mid.to_idx(), row_start.to_idx(), row_end.to_idx()]
            })),
            row_pci: chip.row_pci.map(|row| row.to_idx()),
            holes_ppc: Vec::from_iter(chip.holes_ppc.iter().map(|(col, row)| jzon::array![col.to_idx(), row.to_idx()])),
            dcms: match chip.dcms {
                None => JsonValue::Null,
                Some(dcms) => match dcms {
                    Dcms::Two => 2,
                    Dcms::Four => 4,
                    Dcms::Eight => 8,
                }.into()
            },
            has_ll: chip.has_ll,
            cfg_io: jzon::object::Object::from_iter(chip.cfg_io.iter().map(|(k, io)| {
                (k.to_string(), io.to_string())
            })),
            dci_io: jzon::object::Object::from_iter(chip.dci_io.iter().map(|(k, (io_a, io_b))| {
                (k.to_string(), jzon::object! {
                    vrp: io_a.to_string(),
                    vrn: io_b.to_string(),
                })
            })),
            dci_io_alt: jzon::object::Object::from_iter(chip.dci_io_alt.iter().map(|(k, (io_a, io_b))| {
                (k.to_string(), jzon::object! {
                    vrp: io_a.to_string(),
                    vrn: io_b.to_string(),
                })
            })),
        }
    }
}

impl std::fmt::Display for Chip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tKIND: {k:?}", k = self.kind)?;
        writeln!(f, "\tCOLS:")?;
        for (col, cd) in &self.columns {
            if let Some((cl, cr)) = self.cols_clkv {
                if col == cl {
                    writeln!(f, "\t\t--- clock left spine")?;
                }
                if col == cr {
                    writeln!(f, "\t\t--- clock right spine")?;
                }
            }
            if col == self.col_clk {
                writeln!(f, "\t\t--- clock spine")?;
            }
            write!(f, "\t\tX{c}: ", c = col.to_idx())?;
            match cd.kind {
                ColumnKind::Io => write!(f, "IO")?,
                ColumnKind::Clb => write!(f, "CLB   ")?,
                ColumnKind::Bram => write!(f, "BRAM  ")?,
                ColumnKind::BramCont(i) => write!(f, "BRAM.{i}")?,
                ColumnKind::Dsp => write!(f, "DSP   ")?,
            }
            match cd.io {
                ColumnIoKind::None => (),
                ColumnIoKind::Single => write!(f, " IO: 1")?,
                ColumnIoKind::Double(i) => write!(f, " IO: 2.{i}")?,
                ColumnIoKind::Triple(i) => write!(f, " IO: 3.{i}")?,
                ColumnIoKind::Quad(i) => write!(f, " IO: 4.{i}")?,
                ColumnIoKind::SingleLeft => write!(f, " IO: 1L")?,
                ColumnIoKind::SingleRight => write!(f, " IO: 1R")?,
                ColumnIoKind::SingleLeftAlt => write!(f, " IO: 1LA")?,
                ColumnIoKind::SingleRightAlt => write!(f, " IO: 1RA")?,
                ColumnIoKind::DoubleLeft(i) => write!(f, " IO: 2L.{i}")?,
                ColumnIoKind::DoubleRight(i) => write!(f, " IO: 2R.{i}")?,
                ColumnIoKind::DoubleRightClk(i) => write!(f, " IO: 2R.CLK.{i}")?,
            }
            if let Some(&(bb, bt)) = self.cols_gt.get(&col) {
                write!(f, " GT: BOT {bb} TOP {bt}")?;
            }
            writeln!(f,)?;
        }
        let mut clkv_idx = 0;
        writeln!(f, "\tROWS:")?;
        for (row, rd) in &self.rows {
            if row == self.rows_hclk[clkv_idx].0 {
                writeln!(f, "\t\t--- clock row")?;
            }
            if row == self.rows_hclk[clkv_idx].2 {
                writeln!(f, "\t\t--- clock break")?;
                clkv_idx += 1;
            }
            if Some(row) == self.row_pci {
                writeln!(f, "\t\t--- PCI row")?;
            }
            if row == self.row_mid() {
                writeln!(f, "\t\t--- spine row")?;
            }
            write!(f, "\t\tY{r}: ", r = row.to_idx())?;
            match rd {
                RowIoKind::None => (),
                RowIoKind::Single => write!(f, " IO: 1")?,
                RowIoKind::Double(i) => write!(f, " IO: 2.{i}")?,
                RowIoKind::Triple(i) => write!(f, " IO: 3.{i}")?,
                RowIoKind::Quad(i) => write!(f, " IO: 4.{i}")?,
                RowIoKind::DoubleBot(i) => write!(f, " IO: 2B.{i}")?,
                RowIoKind::DoubleTop(i) => write!(f, " IO: 2T.{i}")?,
            }
            if let Some((rb, rt)) = self.rows_ram {
                if row == rb {
                    write!(f, " BRAM BOT TERM")?;
                }
                if row == rt {
                    write!(f, " BRAM TOP TERM")?;
                }
            }
            writeln!(f,)?;
        }
        for &(col, row) in &self.holes_ppc {
            writeln!(
                f,
                "\tPPC: X{xl}:X{xr} Y{yb}:Y{yt}",
                xl = col.to_idx(),
                xr = col.to_idx() + 10,
                yb = row.to_idx(),
                yt = row.to_idx() + 16
            )?;
        }
        if let Some(dcms) = self.dcms {
            writeln!(f, "\tDCMS: {dcms:?}")?;
        }
        if self.has_ll {
            writeln!(f, "\tHAS LL SPLITTERS")?;
        }
        writeln!(f, "\tHAS_SMALL_INT: {v:?}", v = self.has_small_int)?;
        writeln!(f, "\tCFG PINS:")?;
        for (k, v) in &self.cfg_io {
            writeln!(f, "\t\t{k:?}: {v}")?;
        }
        if !self.dci_io.is_empty() {
            writeln!(f, "\tDCI:")?;
            for k in 0..8 {
                writeln!(f, "\t\t{k}:")?;
                if let Some(&(vp, vn)) = self.dci_io.get(&k) {
                    writeln!(f, "\t\t\tVP: {vp}")?;
                    writeln!(f, "\t\t\tVN: {vn}")?;
                }
                if let Some(&(vp, vn)) = self.dci_io_alt.get(&k) {
                    writeln!(f, "\t\t\tALT VP: {vp}")?;
                    writeln!(f, "\t\t\tALT VN: {vn}")?;
                }
            }
        }
        Ok(())
    }
}
