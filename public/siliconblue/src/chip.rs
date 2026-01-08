use std::collections::{BTreeMap, BTreeSet};

use bincode::{Decode, Encode};
use itertools::Itertools;
use prjcombine_entity::{EntityId, EntityRange, EntityVec};
use prjcombine_interconnect::{
    db::{CellSlotId, IntDb, TileClassId},
    dir::{Dir, DirH, DirV},
    grid::{BelCoord, CellCoord, ColId, DieId, RowId, TileCoord},
};
use prjcombine_types::bimap::BiMap;

use crate::defs;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Encode, Decode)]
pub enum ChipKind {
    Ice65L01,
    Ice65L04,
    Ice65L08,
    Ice65P04,
    Ice40P01,
    Ice40P08,
    Ice40P03,
    Ice40M08,
    Ice40M16,
    Ice40R04,
    Ice40T04,
    Ice40T01,
    Ice40T05,
}

impl ChipKind {
    pub fn is_ice65(self) -> bool {
        matches!(
            self,
            Self::Ice65L01 | Self::Ice65L04 | Self::Ice65L08 | Self::Ice65P04
        )
    }

    pub fn is_ice40(self) -> bool {
        matches!(
            self,
            Self::Ice40P01
                | Self::Ice40P08
                | Self::Ice40P03
                | Self::Ice40M08
                | Self::Ice40M16
                | Self::Ice40R04
                | Self::Ice40T04
                | Self::Ice40T01
                | Self::Ice40T05
        )
    }

    pub fn has_latch_global_out(self) -> bool {
        !matches!(self, Self::Ice65L04 | Self::Ice65L08)
    }

    pub fn has_ice40_bramv2(self) -> bool {
        matches!(
            self,
            Self::Ice40P08
                | Self::Ice40M08
                | Self::Ice40M16
                | Self::Ice40R04
                | Self::Ice40T04
                | Self::Ice40T01
                | Self::Ice40T05
        )
    }

    pub fn has_ioi_we(self) -> bool {
        matches!(
            self,
            Self::Ice65L01
                | Self::Ice65L04
                | Self::Ice65L08
                | Self::Ice65P04
                | Self::Ice40P01
                | Self::Ice40P08
                | Self::Ice40P03
                | Self::Ice40M08
                | Self::Ice40M16
                | Self::Ice40R04
        )
    }

    pub fn has_iob_we(self) -> bool {
        matches!(
            self,
            Self::Ice65L01
                | Self::Ice65L04
                | Self::Ice65L08
                | Self::Ice65P04
                | Self::Ice40P01
                | Self::Ice40P08
                | Self::Ice40P03
                | Self::Ice40M08
                | Self::Ice40M16
        )
    }

    pub fn has_vref(self) -> bool {
        matches!(self, Self::Ice65L04 | Self::Ice65P04 | Self::Ice65L08)
    }

    pub fn tile_class_colbuf(self) -> Option<TileClassId> {
        match self {
            Self::Ice65L04 | Self::Ice65P04 | Self::Ice65L08 | Self::Ice40P03 => None,
            Self::Ice65L01 | Self::Ice40P01 => Some(defs::tcls::COLBUF_L01),
            _ => Some(defs::tcls::COLBUF_P08),
        }
    }

    pub fn is_ultra(self) -> bool {
        matches!(self, Self::Ice40T04 | Self::Ice40T01 | Self::Ice40T05)
    }

    pub fn has_multi_pullup(self) -> bool {
        matches!(self, Self::Ice40T01 | Self::Ice40T05)
    }

    pub fn tile_class_plb(self) -> TileClassId {
        match self {
            ChipKind::Ice65L04 | ChipKind::Ice65P04 => defs::tcls::PLB_L04,
            ChipKind::Ice65L08 | ChipKind::Ice65L01 => defs::tcls::PLB_L08,
            ChipKind::Ice40P01
            | ChipKind::Ice40P08
            | ChipKind::Ice40P03
            | ChipKind::Ice40M08
            | ChipKind::Ice40M16
            | ChipKind::Ice40R04
            | ChipKind::Ice40T04
            | ChipKind::Ice40T05
            | ChipKind::Ice40T01 => defs::tcls::PLB_P01,
        }
    }

    pub fn tile_class_gb_root(self) -> TileClassId {
        match self {
            ChipKind::Ice65L04 | ChipKind::Ice65P04 => defs::tcls::GB_ROOT_L04,
            ChipKind::Ice40R04 | ChipKind::Ice40T04 | ChipKind::Ice40T01 | ChipKind::Ice40T05 => {
                defs::tcls::GB_ROOT_R04
            }
            _ => defs::tcls::GB_ROOT_L08,
        }
    }

    pub fn tile_class_bram(self) -> TileClassId {
        match self {
            ChipKind::Ice65L04 | ChipKind::Ice65P04 | ChipKind::Ice65L08 | ChipKind::Ice65L01 => {
                defs::tcls::BRAM_L04
            }
            ChipKind::Ice40P01 => defs::tcls::BRAM_P01,
            ChipKind::Ice40P08
            | ChipKind::Ice40P03
            | ChipKind::Ice40M08
            | ChipKind::Ice40M16
            | ChipKind::Ice40R04
            | ChipKind::Ice40T04
            | ChipKind::Ice40T05
            | ChipKind::Ice40T01 => defs::tcls::BRAM_P08,
        }
    }

    pub fn tile_class_ioi(self, dir: Dir) -> Option<TileClassId> {
        match self {
            ChipKind::Ice65L04 | ChipKind::Ice65P04 => match dir {
                Dir::W => Some(defs::tcls::IOI_W_L04),
                Dir::E => Some(defs::tcls::IOI_E_L04),
                Dir::S => Some(defs::tcls::IOI_S_L04),
                Dir::N => Some(defs::tcls::IOI_N_L04),
            },
            ChipKind::Ice65L08
            | ChipKind::Ice65L01
            | ChipKind::Ice40P01
            | ChipKind::Ice40P08
            | ChipKind::Ice40P03
            | ChipKind::Ice40M08
            | ChipKind::Ice40M16
            | ChipKind::Ice40R04 => match dir {
                Dir::W => Some(defs::tcls::IOI_W_L08),
                Dir::E => Some(defs::tcls::IOI_E_L08),
                Dir::S => Some(defs::tcls::IOI_S_L08),
                Dir::N => Some(defs::tcls::IOI_N_L08),
            },
            ChipKind::Ice40T04 | ChipKind::Ice40T05 | ChipKind::Ice40T01 => match dir {
                Dir::S => Some(defs::tcls::IOI_S_T04),
                Dir::N => Some(defs::tcls::IOI_N_T04),
                _ => None,
            },
        }
    }

    pub fn tile_class_iob(self, dir: Dir) -> Option<TileClassId> {
        match self {
            ChipKind::Ice65L04 => match dir {
                Dir::W => Some(defs::tcls::IOB_W_L04),
                Dir::E => Some(defs::tcls::IOB_E_L04),
                Dir::S => Some(defs::tcls::IOB_S_L04),
                Dir::N => Some(defs::tcls::IOB_N_L04),
            },
            ChipKind::Ice65P04 => match dir {
                Dir::W => Some(defs::tcls::IOB_W_P04),
                Dir::E => Some(defs::tcls::IOB_E_P04),
                Dir::S => Some(defs::tcls::IOB_S_P04),
                Dir::N => Some(defs::tcls::IOB_N_P04),
            },
            ChipKind::Ice65L08 => match dir {
                Dir::W => Some(defs::tcls::IOB_W_L08),
                Dir::E => Some(defs::tcls::IOB_E_L08),
                Dir::S => Some(defs::tcls::IOB_S_L08),
                Dir::N => Some(defs::tcls::IOB_N_L08),
            },
            ChipKind::Ice65L01 => match dir {
                Dir::W => Some(defs::tcls::IOB_W_L01),
                Dir::E => Some(defs::tcls::IOB_E_L01),
                Dir::S => Some(defs::tcls::IOB_S_L01),
                Dir::N => Some(defs::tcls::IOB_N_L01),
            },
            ChipKind::Ice40P01 => match dir {
                Dir::W => Some(defs::tcls::IOB_W_P01),
                Dir::E => Some(defs::tcls::IOB_E_P01),
                Dir::S => Some(defs::tcls::IOB_S_P01),
                Dir::N => Some(defs::tcls::IOB_N_P01),
            },
            ChipKind::Ice40P08 | ChipKind::Ice40M08 | ChipKind::Ice40M16 => match dir {
                Dir::W => Some(defs::tcls::IOB_W_P08),
                Dir::E => Some(defs::tcls::IOB_E_P08),
                Dir::S => Some(defs::tcls::IOB_S_P08),
                Dir::N => Some(defs::tcls::IOB_N_P08),
            },
            ChipKind::Ice40P03 => match dir {
                Dir::W => Some(defs::tcls::IOB_W_P03),
                Dir::E => Some(defs::tcls::IOB_E_P03),
                Dir::S => Some(defs::tcls::IOB_S_P03),
                Dir::N => Some(defs::tcls::IOB_N_P03),
            },
            ChipKind::Ice40R04 => match dir {
                Dir::S => Some(defs::tcls::IOB_S_R04),
                Dir::N => Some(defs::tcls::IOB_N_R04),
                _ => None,
            },
            ChipKind::Ice40T04 => match dir {
                Dir::S => Some(defs::tcls::IOB_S_T04),
                Dir::N => Some(defs::tcls::IOB_N_T04),
                _ => None,
            },
            ChipKind::Ice40T05 => match dir {
                Dir::S => Some(defs::tcls::IOB_S_T05),
                Dir::N => Some(defs::tcls::IOB_N_T05),
                _ => None,
            },
            ChipKind::Ice40T01 => match dir {
                Dir::S => Some(defs::tcls::IOB_S_T01),
                Dir::N => Some(defs::tcls::IOB_N_T01),
                _ => None,
            },
        }
    }
}

impl std::fmt::Display for ChipKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChipKind::Ice65L01 => write!(f, "ice65l01"),
            ChipKind::Ice65L04 => write!(f, "ice65l04"),
            ChipKind::Ice65L08 => write!(f, "ice65l08"),
            ChipKind::Ice65P04 => write!(f, "ice65p04"),
            ChipKind::Ice40P01 => write!(f, "ice40p01"),
            ChipKind::Ice40P08 => write!(f, "ice40p08"),
            ChipKind::Ice40P03 => write!(f, "ice40p03"),
            ChipKind::Ice40M08 => write!(f, "ice40m08"),
            ChipKind::Ice40M16 => write!(f, "ice40m16"),
            ChipKind::Ice40R04 => write!(f, "ice40r04"),
            ChipKind::Ice40T04 => write!(f, "ice40t04"),
            ChipKind::Ice40T01 => write!(f, "ice40t01"),
            ChipKind::Ice40T05 => write!(f, "ice40t05"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Encode, Decode)]
pub struct SpecialTile {
    // has IOI coords
    pub io: BTreeMap<SpecialIoKey, BelCoord>,
    pub cells: EntityVec<CellSlotId, CellCoord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode)]
pub enum SpecialTileKey {
    Globals,
    GbRoot,
    Misc,
    LatchIo(Dir),
    Warmboot,
    Pll(DirV),
    PllStub(DirV),
    Spi(DirH),
    I2c(DirH),
    I2cFifo(DirH),
    LsOsc,
    HsOsc,
    SpramPair(DirH),
}

impl SpecialTileKey {
    pub fn tile_class(self, kind: ChipKind) -> TileClassId {
        match self {
            SpecialTileKey::Globals => defs::tcls::GLOBALS,
            SpecialTileKey::GbRoot => kind.tile_class_gb_root(),
            SpecialTileKey::Misc => match kind {
                ChipKind::Ice40T04 => defs::tcls::MISC_T04,
                ChipKind::Ice40T01 => defs::tcls::MISC_T01,
                ChipKind::Ice40T05 => defs::tcls::MISC_T05,
                _ => unreachable!(),
            },
            SpecialTileKey::LatchIo(_) => defs::tcls::IO_LATCH,
            SpecialTileKey::Pll(dir) => match (dir, kind) {
                (DirV::S, ChipKind::Ice65P04) => defs::tcls::PLL65,
                (DirV::S, ChipKind::Ice40P01) => defs::tcls::PLL40_S_P01,
                (DirV::S, ChipKind::Ice40P08) => defs::tcls::PLL40_S_P08,
                (DirV::N, ChipKind::Ice40P08) => defs::tcls::PLL40_N_P08,
                (DirV::S, ChipKind::Ice40R04) => defs::tcls::PLL40_S_R04,
                (DirV::N, ChipKind::Ice40R04 | ChipKind::Ice40T04 | ChipKind::Ice40T05) => {
                    defs::tcls::PLL40_N_R04
                }
                (DirV::S, ChipKind::Ice40T01) => defs::tcls::PLL40_S_T01,
                _ => unreachable!(),
            },
            SpecialTileKey::PllStub(DirV::S) => defs::tcls::PLL40_S_STUB,
            SpecialTileKey::PllStub(_) => unreachable!(),
            SpecialTileKey::Spi(..) => match kind {
                ChipKind::Ice40R04 => defs::tcls::SPI_R04,
                ChipKind::Ice40T04 => defs::tcls::SPI_T04,
                ChipKind::Ice40T05 => defs::tcls::SPI_T05,
                _ => unreachable!(),
            },
            SpecialTileKey::I2c(..) => match kind {
                ChipKind::Ice40R04 => defs::tcls::I2C_R04,
                ChipKind::Ice40T04 | ChipKind::Ice40T05 => defs::tcls::I2C_T04,
                _ => unreachable!(),
            },
            SpecialTileKey::I2cFifo(..) => defs::tcls::I2C_FIFO,
            SpecialTileKey::SpramPair(_) => defs::tcls::SPRAM,
            SpecialTileKey::Warmboot => defs::tcls::WARMBOOT,
            SpecialTileKey::LsOsc => defs::tcls::LSOSC,
            SpecialTileKey::HsOsc => defs::tcls::HSOSC,
        }
    }
}

impl std::fmt::Display for SpecialTileKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecialTileKey::Globals => write!(f, "GLOBALS"),
            SpecialTileKey::GbRoot => write!(f, "GB_ROOT"),
            SpecialTileKey::Misc => write!(f, "MISC"),
            SpecialTileKey::LatchIo(edge) => write!(f, "LATCH_IO_{edge}"),
            SpecialTileKey::Warmboot => write!(f, "WARMBOOT"),
            SpecialTileKey::Pll(edge) => write!(f, "PLL_{edge}"),
            SpecialTileKey::PllStub(edge) => write!(f, "PLL_STUB_{edge}"),
            SpecialTileKey::Spi(edge) => write!(f, "SPI_{edge}"),
            SpecialTileKey::I2c(edge) => write!(f, "I2C_{edge}"),
            SpecialTileKey::I2cFifo(edge) => write!(f, "I2C_FIFO_{edge}"),
            SpecialTileKey::LsOsc => write!(f, "LSOSC"),
            SpecialTileKey::HsOsc => write!(f, "HSOSC"),
            SpecialTileKey::SpramPair(edge) => write!(f, "SPRAM_{edge}"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum SpecialIoKey {
    CfgSdo(usize),
    CfgSdi(usize),
    CfgSck,
    CfgCsB,
    CbSel0,
    CbSel1,
    JtagTdi,
    JtagTms,
    JtagTck,
    JtagTdo,
    GbIn(usize),
    PllA,
    PllB,
    SpiCopi,
    SpiCipo,
    SpiSck,
    SpiCsB0,
    SpiCsB1,
    I2cScl,
    I2cSda,
    RgbLed0,
    RgbLed1,
    RgbLed2,
    IrLed,
    BarcodeLed,
    I3c0,
    I3c1,
}

impl std::fmt::Display for SpecialIoKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecialIoKey::CfgSdo(i) => write!(f, "CFG_SDO{i}"),
            SpecialIoKey::CfgSdi(i) => write!(f, "CFG_SDI{i}"),
            SpecialIoKey::CfgSck => write!(f, "CFG_SCK"),
            SpecialIoKey::CfgCsB => write!(f, "CFG_CS_B"),
            SpecialIoKey::CbSel0 => write!(f, "CBSEL0"),
            SpecialIoKey::CbSel1 => write!(f, "CBSEL1"),
            SpecialIoKey::JtagTdi => write!(f, "JTAG_TDI"),
            SpecialIoKey::JtagTms => write!(f, "JTAG_TMS"),
            SpecialIoKey::JtagTck => write!(f, "JTAG_TCK"),
            SpecialIoKey::JtagTdo => write!(f, "JTAG_TDO"),
            SpecialIoKey::GbIn(idx) => write!(f, "GB_IN{idx}"),
            SpecialIoKey::PllA => write!(f, "PLL_A"),
            SpecialIoKey::PllB => write!(f, "PLL_B"),
            SpecialIoKey::SpiCopi => write!(f, "SPI_COPI"),
            SpecialIoKey::SpiCipo => write!(f, "SPI_CIPO"),
            SpecialIoKey::SpiSck => write!(f, "SPI_SCK"),
            SpecialIoKey::SpiCsB0 => write!(f, "SPI_CS_B0"),
            SpecialIoKey::SpiCsB1 => write!(f, "SPI_CS_B1"),
            SpecialIoKey::I2cScl => write!(f, "I2C_SCL"),
            SpecialIoKey::I2cSda => write!(f, "I2C_SDA"),
            SpecialIoKey::RgbLed0 => write!(f, "RGB_LED0"),
            SpecialIoKey::RgbLed1 => write!(f, "RGB_LED1"),
            SpecialIoKey::RgbLed2 => write!(f, "RGB_LED2"),
            SpecialIoKey::IrLed => write!(f, "IR_LED"),
            SpecialIoKey::BarcodeLed => write!(f, "BARCODE_LED"),
            SpecialIoKey::I3c0 => write!(f, "I3C0"),
            SpecialIoKey::I3c1 => write!(f, "I3C1"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Encode, Decode)]
pub struct Chip {
    pub kind: ChipKind,
    pub columns: usize,
    pub col_bio_split: ColId,
    pub cols_bram: Vec<ColId>,
    pub rows: usize,
    pub row_mid: RowId,
    // (hclk row, start row, end row)
    pub rows_colbuf: Vec<(RowId, RowId, RowId)>,
    pub rows_mac16: Vec<RowId>,
    pub ioi_iob: BiMap<BelCoord, BelCoord>,
    pub iob_od: BTreeSet<BelCoord>,
    pub special_tiles: BTreeMap<SpecialTileKey, SpecialTile>,
}

impl Chip {
    pub fn col_w(&self) -> ColId {
        ColId::from_idx(0)
    }

    pub fn col_e(&self) -> ColId {
        ColId::from_idx(self.columns - 1)
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

    pub fn row_edge(&self, edge: DirV) -> RowId {
        match edge {
            DirV::S => self.row_s(),
            DirV::N => self.row_n(),
        }
    }

    pub fn col_mid(&self) -> ColId {
        ColId::from_idx(self.columns / 2)
    }

    pub fn columns(&self) -> EntityRange<ColId> {
        EntityRange::new(0, self.columns)
    }

    pub fn rows(&self) -> EntityRange<RowId> {
        EntityRange::new(0, self.rows)
    }

    pub fn globals(&self) -> TileCoord {
        CellCoord::new(DieId::from_idx(0), self.col_w(), self.row_s()).tile(defs::tslots::GLOBALS)
    }

    pub fn get_io_bank(&self, io: BelCoord) -> BelCoord {
        let slot = if io.col == self.col_w() {
            defs::bslots::IO_BANK[3]
        } else if io.col == self.col_e() {
            defs::bslots::IO_BANK[1]
        } else if io.row == self.row_n() {
            defs::bslots::IO_BANK[0]
        } else if io.row == self.row_s() {
            if io.col < self.col_bio_split {
                defs::bslots::IO_BANK[2]
            } else {
                defs::bslots::IO_BANK_SPI
            }
        } else {
            panic!("weird io")
        };
        self.globals().bel(slot)
    }

    pub fn ioi_to_iob(&self, ioi: BelCoord) -> Option<BelCoord> {
        self.ioi_iob.get_left(&ioi).copied()
    }

    pub fn iob_to_ioi(&self, iob: BelCoord) -> Option<BelCoord> {
        self.ioi_iob.get_right(&iob).copied()
    }

    pub fn iob_has_lvds(&self, iob: BelCoord) -> bool {
        let idx = defs::bslots::IOB.index_of(iob.slot).unwrap();
        if idx != 0 {
            return false;
        }
        if self.kind == ChipKind::Ice65L01 {
            false
        } else if self.kind.has_iob_we() {
            iob.col == self.col_w()
        } else if self.kind == ChipKind::Ice40R04 {
            iob.row == self.row_n()
        } else {
            !self.iob_od.contains(&iob)
        }
    }

    pub fn has_int_at(&self, col: ColId, row: RowId) -> bool {
        if col != self.col_w() && col != self.col_e() {
            return false;
        }
        if self.kind == ChipKind::Ice40T01 {
            row - self.row_s() <= 3 || self.row_n() - row <= 3
        } else {
            true
        }
    }

    pub fn btile_width(&self, col: ColId) -> usize {
        if self.cols_bram.contains(&col) {
            42
        } else if self.kind.has_ioi_we() && (col == self.col_w() || col == self.col_e()) {
            18
        } else {
            54
        }
    }

    pub fn special_tile(&self, key: SpecialTileKey) -> TileCoord {
        let spec = &self.special_tiles[&key];
        match key {
            SpecialTileKey::Globals => self.globals(),
            SpecialTileKey::GbRoot => {
                CellCoord::new(DieId::from_idx(0), self.col_mid(), self.row_mid)
                    .tile(defs::tslots::GB_ROOT)
            }
            SpecialTileKey::Misc => self.globals().tile(defs::tslots::BEL),
            SpecialTileKey::Warmboot => {
                CellCoord::new(DieId::from_idx(0), self.col_e(), self.row_s())
                    .tile(defs::tslots::BEL)
            }
            SpecialTileKey::Pll(edge) | SpecialTileKey::PllStub(edge) => {
                CellCoord::new(DieId::from_idx(0), self.col_mid() - 1, self.row_edge(edge))
                    .tile(defs::tslots::BEL)
            }
            SpecialTileKey::LatchIo(_)
            | SpecialTileKey::Spi(_)
            | SpecialTileKey::I2c(_)
            | SpecialTileKey::I2cFifo(_)
            | SpecialTileKey::LsOsc
            | SpecialTileKey::HsOsc
            | SpecialTileKey::SpramPair(_) => spec.cells.first().unwrap().tile(defs::tslots::BEL),
        }
    }

    pub fn get_io_edge(&self, io: BelCoord) -> Dir {
        if io.col == self.col_w() {
            Dir::W
        } else if io.col == self.col_e() {
            Dir::E
        } else if io.row == self.row_n() {
            Dir::N
        } else if io.row == self.row_s() {
            Dir::S
        } else {
            panic!("weird io")
        }
    }

    pub fn bel_pll(&self, edge: DirV) -> BelCoord {
        let cell = CellCoord::new(DieId::from_idx(0), self.col_mid() - 1, self.row_edge(edge));
        cell.bel(if self.kind.is_ice65() {
            defs::bslots::PLL65
        } else {
            defs::bslots::PLL40
        })
    }
}

impl Chip {
    pub fn dump(&self, o: &mut dyn std::io::Write, db: &IntDb) -> std::io::Result<()> {
        writeln!(o, "\tkind {};", self.kind)?;
        writeln!(o, "\tcolumns {};", self.columns)?;
        writeln!(o, "\trows {};", self.rows)?;
        writeln!(o, "\tcol_bio_split {};", self.col_bio_split)?;
        if !self.cols_bram.is_empty() {
            writeln!(
                o,
                "\tcols_bram {};",
                self.cols_bram.iter().map(|x| x.to_string()).join(", ")
            )?;
        }
        writeln!(o, "\trow_mid {};", self.row_mid)?;
        for &(row_hclk, row_start, row_end) in &self.rows_colbuf {
            writeln!(o, "\trow_colbuf {row_hclk} = {row_start}..{row_end};")?;
        }
        if !self.rows_mac16.is_empty() {
            writeln!(
                o,
                "\trows_mac16 {};",
                self.rows_mac16.iter().map(|x| x.to_string()).join(", ")
            )?;
        }
        for (ioi, iob) in &self.ioi_iob {
            writeln!(
                o,
                "\tiob {ioi} = {iob};",
                ioi = ioi.to_string(db),
                iob = iob.to_string(db)
            )?;
        }
        for iob in &self.iob_od {
            writeln!(o, "\tiob_od {iob};", iob = iob.to_string(db))?;
        }
        for (key, spec) in &self.special_tiles {
            writeln!(o, "\tspecial {key} {{")?;
            for v in spec.cells.values() {
                writeln!(o, "\t\tcell {v};")?;
            }
            for (k, v) in &spec.io {
                writeln!(o, "\t\tio {k} = {v};", v = v.to_string(db))?;
            }
            writeln!(o, "\t}}")?;
        }

        Ok(())
    }
}
