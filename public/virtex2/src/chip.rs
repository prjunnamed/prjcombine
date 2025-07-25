use bincode::{Decode, Encode};
use jzon::JsonValue;
use prjcombine_interconnect::db::CellSlotId;
use prjcombine_interconnect::dir::{Dir, DirH, DirHV, DirV};
use prjcombine_interconnect::grid::{
    BelCoord, CellCoord, ColId, DieId, EdgeIoCoord, RowId, TileCoord, TileIobId,
};
use std::collections::BTreeMap;
use unnamed_entity::{EntityId, EntityVec};

use crate::iob::{
    IobKind, IobTileData, get_iob_data_e, get_iob_data_n, get_iob_data_s, get_iob_data_w,
};
use crate::{bels, tslots};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
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

impl std::fmt::Display for ChipKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChipKind::Virtex2 => write!(f, "virtex2"),
            ChipKind::Virtex2P => write!(f, "virtex2p"),
            ChipKind::Virtex2PX => write!(f, "virtex2px"),
            ChipKind::Spartan3 => write!(f, "spartan3"),
            ChipKind::Spartan3E => write!(f, "spartan3e"),
            ChipKind::Spartan3A => write!(f, "spartan3a"),
            ChipKind::Spartan3ADsp => write!(f, "spartan3adsp"),
            ChipKind::FpgaCore => write!(f, "fpgacore"),
        }
    }
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

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
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
    pub cfg_io: BTreeMap<SharedCfgPad, EdgeIoCoord>,
    pub dci_io: BTreeMap<u32, (EdgeIoCoord, EdgeIoCoord)>,
    pub dci_io_alt: BTreeMap<u32, (EdgeIoCoord, EdgeIoCoord)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Column {
    pub kind: ColumnKind,
    pub io: ColumnIoKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum ColumnKind {
    Io,
    Clb,
    Bram,
    BramCont(u8),
    Dsp,
}

impl std::fmt::Display for ColumnKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ColumnKind::Io => write!(f, "IO"),
            ColumnKind::Clb => write!(f, "CLB"),
            ColumnKind::Bram => write!(f, "BRAM"),
            ColumnKind::BramCont(i) => write!(f, "BRAM_CONT:{i}"),
            ColumnKind::Dsp => write!(f, "DSP"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
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

impl std::fmt::Display for ColumnIoKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ColumnIoKind::None => write!(f, "NONE"),
            ColumnIoKind::Single => write!(f, "SINGLE"),
            ColumnIoKind::Double(i) => write!(f, "DOUBLE:{i}"),
            ColumnIoKind::Triple(i) => write!(f, "TRIPLE:{i}"),
            ColumnIoKind::Quad(i) => write!(f, "QUAD:{i}"),
            ColumnIoKind::SingleLeft => write!(f, "SINGLE_LEFT"),
            ColumnIoKind::SingleRight => write!(f, "SINGLE_RIGHT"),
            ColumnIoKind::SingleLeftAlt => write!(f, "SINGLE_LEFT_ALT"),
            ColumnIoKind::SingleRightAlt => write!(f, "SINGLE_RIGHT_ALT"),
            ColumnIoKind::DoubleLeft(i) => write!(f, "DOUBLE_LEFT:{i}"),
            ColumnIoKind::DoubleRight(i) => write!(f, "DOUBLE_RIGHT:{i}"),
            ColumnIoKind::DoubleRightClk(i) => write!(f, "DOUBLE_RIGHT_CLK:{i}"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum RowIoKind {
    None,
    Single,
    Double(u8),
    Triple(u8),
    Quad(u8),
    DoubleBot(u8),
    DoubleTop(u8),
}

impl std::fmt::Display for RowIoKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            RowIoKind::None => write!(f, "NONE"),
            RowIoKind::Single => write!(f, "SINGLE"),
            RowIoKind::Double(i) => write!(f, "DOUBLE:{i}"),
            RowIoKind::Triple(i) => write!(f, "TRIPLE:{i}"),
            RowIoKind::Quad(i) => write!(f, "QUAD:{i}"),
            RowIoKind::DoubleBot(i) => write!(f, "DOUBLE_BOT:{i}"),
            RowIoKind::DoubleTop(i) => write!(f, "DOUBLE_TOP:{i}"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum Dcms {
    Two,
    Four,
    Eight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum SharedCfgPad {
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

impl std::fmt::Display for SharedCfgPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SharedCfgPad::Data(i) => write!(f, "D{i}"),
            SharedCfgPad::Addr(i) => write!(f, "A{i}"),
            SharedCfgPad::CsiB => write!(f, "CSI_B"),
            SharedCfgPad::CsoB => write!(f, "CSO_B"),
            SharedCfgPad::RdWrB => write!(f, "RDWR_B"),
            SharedCfgPad::Dout => write!(f, "DOUT"),
            SharedCfgPad::InitB => write!(f, "INIT_B"),
            SharedCfgPad::Cclk => write!(f, "CCLK"),
            SharedCfgPad::M0 => write!(f, "M0"),
            SharedCfgPad::M1 => write!(f, "M1"),
            SharedCfgPad::M2 => write!(f, "M2"),
            SharedCfgPad::Ldc0 => write!(f, "LDC0"),
            SharedCfgPad::Ldc1 => write!(f, "LDC1"),
            SharedCfgPad::Ldc2 => write!(f, "LDC2"),
            SharedCfgPad::Hdc => write!(f, "HDC"),
            SharedCfgPad::HswapEn => write!(f, "HSWAP_EN"),
            SharedCfgPad::Awake => write!(f, "AWAKE"),
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Encode, Decode)]
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

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Encode, Decode)]
pub struct DcmPair {
    pub kind: DcmPairKind,
    pub cell: CellCoord,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Encode, Decode)]
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
    pub fn col_edge(&self, dir: DirH) -> ColId {
        match dir {
            DirH::W => self.col_w(),
            DirH::E => self.col_e(),
        }
    }

    pub fn row_edge(&self, dir: DirV) -> RowId {
        match dir {
            DirV::S => self.row_s(),
            DirV::N => self.row_n(),
        }
    }

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

    pub fn is_row_io(&self, row: RowId) -> bool {
        row == self.row_s() || row == self.row_n()
    }

    pub fn is_col_io(&self, col: ColId) -> bool {
        col == self.col_w() || col == self.col_e()
    }

    pub fn corner(&self, dir: DirHV) -> TileCoord {
        CellCoord::new(
            DieId::from_idx(0),
            self.col_edge(dir.h),
            self.row_edge(dir.v),
        )
        .tile(tslots::BEL)
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
        let die = DieId::from_idx(0);
        let mut res = vec![];
        if let Some(dcms) = self.dcms {
            if dcms == Dcms::Two {
                if self.kind == ChipKind::Spartan3E {
                    res.extend([
                        DcmPair {
                            kind: DcmPairKind::BotSingle,
                            cell: CellCoord::new(die, self.col_clk, self.row_s() + 1),
                        },
                        DcmPair {
                            kind: DcmPairKind::TopSingle,
                            cell: CellCoord::new(die, self.col_clk, self.row_n() - 1),
                        },
                    ]);
                } else {
                    res.extend([DcmPair {
                        kind: DcmPairKind::Top,
                        cell: CellCoord::new(die, self.col_clk, self.row_n() - 1),
                    }]);
                }
            } else {
                res.extend([
                    DcmPair {
                        kind: DcmPairKind::Bot,
                        cell: CellCoord::new(die, self.col_clk, self.row_s() + 1),
                    },
                    DcmPair {
                        kind: DcmPairKind::Top,
                        cell: CellCoord::new(die, self.col_clk, self.row_n() - 1),
                    },
                ]);
            }
            if dcms == Dcms::Eight {
                if self.kind == ChipKind::Spartan3E {
                    res.extend([
                        DcmPair {
                            kind: DcmPairKind::Left,
                            cell: CellCoord::new(die, self.col_w() + 9, self.row_mid()),
                        },
                        DcmPair {
                            kind: DcmPairKind::Right,
                            cell: CellCoord::new(die, self.col_e() - 9, self.row_mid()),
                        },
                    ]);
                } else {
                    res.extend([
                        DcmPair {
                            kind: DcmPairKind::Bram,
                            cell: CellCoord::new(die, self.col_w() + 3, self.row_mid()),
                        },
                        DcmPair {
                            kind: DcmPairKind::Bram,
                            cell: CellCoord::new(die, self.col_e() - 6, self.row_mid()),
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
        let bel = self.get_io_loc(io);
        if let Some((data, tidx)) = self.get_iob_tile_data(bel.cell) {
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
        let die = DieId::from_idx(0);
        for col in self.columns.ids() {
            let row = self.row_n();
            if let Some((data, tidx)) = self.get_iob_tile_data(CellCoord::new(die, col, row)) {
                for &iob in &data.iobs {
                    if iob.tile == tidx {
                        res.push(EdgeIoCoord::N(col, TileIobId::from_idx(iob.iob.to_idx())));
                    }
                }
            }
        }
        for row in self.rows.ids().rev() {
            let col = self.col_e();
            if let Some((data, tidx)) = self.get_iob_tile_data(CellCoord::new(die, col, row)) {
                for &iob in &data.iobs {
                    if iob.tile == tidx {
                        res.push(EdgeIoCoord::E(row, TileIobId::from_idx(iob.iob.to_idx())));
                    }
                }
            }
        }
        for col in self.columns.ids().rev() {
            let row = self.row_s();
            if let Some((data, tidx)) = self.get_iob_tile_data(CellCoord::new(die, col, row)) {
                for &iob in &data.iobs {
                    if iob.tile == tidx {
                        res.push(EdgeIoCoord::S(col, TileIobId::from_idx(iob.iob.to_idx())));
                    }
                }
            }
        }
        for row in self.rows.ids() {
            let col = self.col_w();
            if let Some((data, tidx)) = self.get_iob_tile_data(CellCoord::new(die, col, row)) {
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
        CellCoord::new(DieId::from_idx(0), col, row).bel(slot)
    }

    pub fn get_io_crd(&self, bel: BelCoord) -> EdgeIoCoord {
        let iob = TileIobId::from_idx(if self.kind == ChipKind::FpgaCore {
            bels::IBUF
                .iter()
                .position(|&x| x == bel.slot)
                .unwrap_or_else(|| bels::OBUF.iter().position(|&x| x == bel.slot).unwrap() + 4)
        } else {
            bels::IO.iter().position(|&x| x == bel.slot).unwrap()
        });
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

    pub fn get_iob_tile_data(&self, cell: CellCoord) -> Option<(IobTileData, CellSlotId)> {
        if cell.col == self.col_w() {
            let kind = self.rows[cell.row];
            if kind == RowIoKind::None {
                None
            } else {
                Some(get_iob_data_w(self.kind, kind))
            }
        } else if cell.col == self.col_e() {
            let kind = self.rows[cell.row];
            if kind == RowIoKind::None {
                None
            } else {
                Some(get_iob_data_e(self.kind, kind))
            }
        } else if cell.row == self.row_s() {
            let kind = self.columns[cell.col].io;
            if kind == ColumnIoKind::None {
                None
            } else {
                Some(get_iob_data_s(self.kind, kind))
            }
        } else if cell.row == self.row_n() {
            let kind = self.columns[cell.col].io;
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
            kind: chip.kind.to_string(),
            columns: Vec::from_iter(chip.columns.values().map(|column| {
                jzon::object! {
                    kind: column.kind.to_string(),
                    io: if column.io == ColumnIoKind::None {
                        JsonValue::Null
                    } else {
                        column.io.to_string().into()
                    },
                }
            })),
            cols_clkv: chip.cols_clkv.map(|(col_l, col_r)| jzon::array![col_l.to_idx(), col_r.to_idx()]),
            cols_gt: Vec::from_iter(chip.cols_gt.iter().map(|(col, (bank_b, bank_t))| jzon::object! {
                column: col.to_idx(),
                bank_b: *bank_b,
                bank_t: *bank_t,
            })),
            rows: Vec::from_iter(chip.rows.values().map(|&io| if io == RowIoKind::None {
                JsonValue::Null
            } else {
                io.to_string().into()
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
        writeln!(f, "\tKIND: {k}", k = self.kind)?;
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
            write!(f, "\t\t{col}: ")?;
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
            write!(f, "\t\t{row}: ")?;
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
                "\tPPC: {col_l}:{col_r} {row_b}:{row_t}",
                col_l = col,
                col_r = col + 10,
                row_b = row,
                row_t = row + 16
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
