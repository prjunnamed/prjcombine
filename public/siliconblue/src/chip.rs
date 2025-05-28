use std::collections::{BTreeMap, BTreeSet};

use jzon::JsonValue;
use prjcombine_interconnect::{
    db::TileCellId,
    dir::{Dir, DirH, DirV},
    grid::{ColId, DieId, EdgeIoCoord, BelCoord, RowId, TileIobId},
};
use serde::{Deserialize, Serialize};
use unnamed_entity::{EntityId, EntityIds, EntityVec};

use crate::bels;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

    pub fn has_io_we(self) -> bool {
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

    pub fn has_actual_io_we(self) -> bool {
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

    pub fn has_colbuf(self) -> bool {
        !matches!(
            self,
            Self::Ice65L04 | Self::Ice65P04 | Self::Ice65L08 | Self::Ice40P03
        )
    }

    pub fn is_ultra(self) -> bool {
        matches!(self, Self::Ice40T04 | Self::Ice40T01 | Self::Ice40T05)
    }

    pub fn has_multi_pullup(self) -> bool {
        matches!(self, Self::Ice40T01 | Self::Ice40T05)
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    SpiSo,
    SpiSi,
    SpiSck,
    SpiCsB,
    CbSel0,
    CbSel1,
}

impl std::fmt::Display for SharedCfgPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SharedCfgPin::SpiSo => write!(f, "SPI_SO"),
            SharedCfgPin::SpiSi => write!(f, "SPI_SI"),
            SharedCfgPin::SpiSck => write!(f, "SPI_SCK"),
            SharedCfgPin::SpiCsB => write!(f, "SPI_CS_B"),
            SharedCfgPin::CbSel0 => write!(f, "CBSEL0"),
            SharedCfgPin::CbSel1 => write!(f, "CBSEL1"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExtraNode {
    pub io: BTreeMap<ExtraNodeIo, EdgeIoCoord>,
    pub tiles: EntityVec<TileCellId, (ColId, RowId)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ExtraNodeLoc {
    GbFabric(usize),
    GbIo(usize),
    LatchIo(Dir),
    Warmboot,
    Pll(DirV),
    PllStub(DirV),
    Spi(DirH),
    I2c(DirH),
    I2cFifo(DirH),
    LsOsc,
    HsOsc,
    LfOsc,
    HfOsc,
    Trim,
    IoI3c(EdgeIoCoord),
    IrDrv,
    RgbDrv,
    Ir500Drv,
    RgbaDrv,
    LedDrvCur,
    LeddIp,
    LeddaIp,
    IrIp,
    Mac16(ColId, RowId),
    Mac16Trim(ColId, RowId),
    SpramPair(DirH),
    FilterPair,
    SmcClk,
}

impl ExtraNodeLoc {
    pub fn node_kind(self) -> String {
        match self {
            ExtraNodeLoc::GbFabric(_) => "GB_FABRIC".to_string(),
            ExtraNodeLoc::LatchIo(_) => "IO_LATCH".to_string(),
            ExtraNodeLoc::IoI3c(crd) => {
                let iob = crd.iob();
                format!("IO_I3C_{iob}")
            }
            ExtraNodeLoc::Mac16(..) => "MAC16".to_string(),
            ExtraNodeLoc::Mac16Trim(..) => "MAC16_TRIM".to_string(),
            ExtraNodeLoc::SpramPair(_) => "SPRAM".to_string(),
            _ => self.to_string(),
        }
    }
}

impl std::fmt::Display for ExtraNodeLoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtraNodeLoc::GbFabric(idx) => write!(f, "GB{idx}_FABRIC"),
            ExtraNodeLoc::GbIo(idx) => write!(f, "GB{idx}_IO"),
            ExtraNodeLoc::LatchIo(edge) => write!(f, "LATCH_IO_{edge}"),
            ExtraNodeLoc::Warmboot => write!(f, "WARMBOOT"),
            ExtraNodeLoc::Pll(edge) => write!(f, "PLL_{edge}"),
            ExtraNodeLoc::PllStub(edge) => write!(f, "PLL_STUB_{edge}"),
            ExtraNodeLoc::Spi(edge) => write!(f, "SPI_{edge}"),
            ExtraNodeLoc::I2c(edge) => write!(f, "I2C_{edge}"),
            ExtraNodeLoc::I2cFifo(edge) => write!(f, "I2C_FIFO_{edge}"),
            ExtraNodeLoc::LsOsc => write!(f, "LSOSC"),
            ExtraNodeLoc::HsOsc => write!(f, "HSOSC"),
            ExtraNodeLoc::LfOsc => write!(f, "LFOSC"),
            ExtraNodeLoc::HfOsc => write!(f, "HFOSC"),
            ExtraNodeLoc::Trim => write!(f, "TRIM"),
            ExtraNodeLoc::IoI3c(crd) => write!(f, "IO_I3C_{crd}"),
            ExtraNodeLoc::IrDrv => write!(f, "IR_DRV"),
            ExtraNodeLoc::RgbDrv => write!(f, "RGB_DRV"),
            ExtraNodeLoc::Ir500Drv => write!(f, "IR500_DRV"),
            ExtraNodeLoc::RgbaDrv => write!(f, "RGBA_DRV"),
            ExtraNodeLoc::LedDrvCur => write!(f, "LED_DRV_CUR"),
            ExtraNodeLoc::LeddIp => write!(f, "LEDD_IP"),
            ExtraNodeLoc::LeddaIp => write!(f, "LEDDA_IP"),
            ExtraNodeLoc::IrIp => write!(f, "IR_IP"),
            ExtraNodeLoc::Mac16(col, row) => write!(f, "MAC16_X{col}Y{row}"),
            ExtraNodeLoc::Mac16Trim(col, row) => write!(f, "MAC16_TRIM_X{col}Y{row}"),
            ExtraNodeLoc::SpramPair(edge) => write!(f, "SPRAM_{edge}"),
            ExtraNodeLoc::FilterPair => write!(f, "FILTER"),
            ExtraNodeLoc::SmcClk => write!(f, "SMCCLK"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ExtraNodeIo {
    GbIn,
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
}

impl std::fmt::Display for ExtraNodeIo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtraNodeIo::GbIn => write!(f, "GB_IN"),
            ExtraNodeIo::PllA => write!(f, "PLL_A"),
            ExtraNodeIo::PllB => write!(f, "PLL_B"),
            ExtraNodeIo::SpiCopi => write!(f, "SPI_COPI"),
            ExtraNodeIo::SpiCipo => write!(f, "SPI_CIPO"),
            ExtraNodeIo::SpiSck => write!(f, "SPI_SCK"),
            ExtraNodeIo::SpiCsB0 => write!(f, "SPI_CS_B0"),
            ExtraNodeIo::SpiCsB1 => write!(f, "SPI_CS_B1"),
            ExtraNodeIo::I2cScl => write!(f, "I2C_SCL"),
            ExtraNodeIo::I2cSda => write!(f, "I2C_SDA"),
            ExtraNodeIo::RgbLed0 => write!(f, "RGB_LED0"),
            ExtraNodeIo::RgbLed1 => write!(f, "RGB_LED1"),
            ExtraNodeIo::RgbLed2 => write!(f, "RGB_LED2"),
            ExtraNodeIo::IrLed => write!(f, "IR_LED"),
            ExtraNodeIo::BarcodeLed => write!(f, "BARCODE_LED"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Chip {
    pub kind: ChipKind,
    pub columns: usize,
    pub col_bio_split: ColId,
    pub cols_bram: BTreeSet<ColId>,
    pub rows: usize,
    pub row_mid: RowId,
    // (hclk row, start row, end row)
    pub rows_colbuf: Vec<(RowId, RowId, RowId)>,
    pub cfg_io: BTreeMap<SharedCfgPin, EdgeIoCoord>,
    pub io_iob: BTreeMap<EdgeIoCoord, EdgeIoCoord>,
    pub io_od: BTreeSet<EdgeIoCoord>,
    pub extra_nodes: BTreeMap<ExtraNodeLoc, ExtraNode>,
}

impl Chip {
    pub fn col_w(&self) -> ColId {
        ColId::from_idx(0)
    }

    pub fn col_e(&self) -> ColId {
        ColId::from_idx(self.columns - 1)
    }

    pub fn row_s(&self) -> RowId {
        RowId::from_idx(0)
    }

    pub fn row_n(&self) -> RowId {
        RowId::from_idx(self.rows - 1)
    }

    pub fn col_mid(&self) -> ColId {
        ColId::from_idx(self.columns / 2)
    }

    pub fn columns(&self) -> EntityIds<ColId> {
        EntityIds::new(self.columns)
    }

    pub fn rows(&self) -> EntityIds<RowId> {
        EntityIds::new(self.rows)
    }

    pub fn get_io_bank(&self, io: EdgeIoCoord) -> u32 {
        match io {
            EdgeIoCoord::N(_, _) => 0,
            EdgeIoCoord::E(_, _) => 1,
            EdgeIoCoord::S(col, _) => {
                if col < self.col_bio_split {
                    2
                } else if self.kind.has_io_we() {
                    4
                } else {
                    1
                }
            }
            EdgeIoCoord::W(_, _) => 3,
        }
    }

    pub fn get_io_loc(&self, io: EdgeIoCoord) -> BelCoord {
        let (col, row, iob) = match io {
            EdgeIoCoord::N(col, iob) => (col, self.row_n(), iob),
            EdgeIoCoord::E(row, iob) => (self.col_e(), row, iob),
            EdgeIoCoord::S(col, iob) => (col, self.row_s(), iob),
            EdgeIoCoord::W(row, iob) => (self.col_w(), row, iob),
        };
        let slot = bels::IO[iob.to_idx()];
        (DieId::from_idx(0), (col, row), slot)
    }

    pub fn get_io_crd(&self, bel: BelCoord) -> EdgeIoCoord {
        let (_, (col, row), slot) = bel;
        let iob = TileIobId::from_idx(bels::IO.iter().position(|&x| x == slot).unwrap());
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

    pub fn io_has_lvds(&self, crd: EdgeIoCoord) -> bool {
        let iob = match crd {
            EdgeIoCoord::N(_, iob) => iob,
            EdgeIoCoord::E(_, iob) => iob,
            EdgeIoCoord::S(_, iob) => iob,
            EdgeIoCoord::W(_, iob) => iob,
        };
        if iob.to_idx() != 0 {
            return false;
        }
        if self.kind == ChipKind::Ice65L01 {
            false
        } else if self.kind.has_actual_io_we() {
            crd.edge() == Dir::W
        } else if self.kind == ChipKind::Ice40R04 {
            crd.edge() == Dir::N
        } else {
            !self.io_od.contains(&crd)
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
        } else if self.kind.has_io_we() && (col == self.col_w() || col == self.col_e()) {
            18
        } else {
            54
        }
    }
}

impl From<&ExtraNode> for JsonValue {
    fn from(node: &ExtraNode) -> Self {
        jzon::object! {
            io: jzon::object::Object::from_iter(node.io.iter().map(|(slot, io)| (slot.to_string(), io.to_string()))),
            tiles: Vec::from_iter(node.tiles.values().map(|(col, row)| jzon::array![col.to_idx(), row.to_idx()])),
        }
    }
}

impl From<&Chip> for JsonValue {
    fn from(chip: &Chip) -> Self {
        jzon::object! {
            kind: chip.kind.to_string(),
            columns: chip.columns,
            col_bio_split: chip.col_bio_split.to_idx(),
            cols_bram: Vec::from_iter(chip.cols_bram.iter().map(|col| col.to_idx())),
            rows: chip.rows,
            row_mid: chip.row_mid.to_idx(),
            rows_colbuf: Vec::from_iter(chip.rows_colbuf.iter().map(|(row_mid, row_start, row_end)| {
                jzon::array![row_mid.to_idx(), row_start.to_idx(), row_end.to_idx()]
            })),
            cfg_io: jzon::object::Object::from_iter(chip.cfg_io.iter().map(|(k, io)| {
                (k.to_string(), io.to_string())
            })),
            io_iob: jzon::object::Object::from_iter(chip.io_iob.iter().map(|(&k, &v)| (k.to_string(), v.to_string()))),
            io_od: Vec::from_iter(chip.io_od.iter().map(|crd| crd.to_string())),
            extra_nodes: jzon::object::Object::from_iter(chip.extra_nodes.iter().map(|(&k, v)| (k.to_string(), v))),
        }
    }
}

impl std::fmt::Display for Chip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tKIND: {k:?}", k = self.kind)?;
        writeln!(f, "\tDIMS: {c}×{r}", c = self.columns, r = self.rows)?;
        writeln!(f, "\tBIO SPLIT COLUMN: {c}", c = self.col_bio_split)?;
        if !self.cols_bram.is_empty() {
            write!(f, "\tBRAM COLUMNS:")?;
            for &col in &self.cols_bram {
                write!(f, " {col}")?;
            }
            writeln!(f)?;
        }
        writeln!(f, "\tROW MID: {r}", r = self.row_mid)?;
        if !self.rows_colbuf.is_empty() {
            writeln!(f, "\tROWS COLBUF:")?;
            for &(row_mid, row_bot, row_top) in &self.rows_colbuf {
                writeln!(f, "\t\t{row_mid}: {row_bot}..{row_top}")?;
            }
        }
        for (&loc, node) in &self.extra_nodes {
            writeln!(f, "\tEXTRA {loc}:")?;
            for (slot, io) in &node.io {
                writeln!(f, "\t\tIO {slot}: {io}")?;
            }
            for (tile, (col, row)) in &node.tiles {
                writeln!(f, "\t\tTILE {tile}: X{col}Y{row}")?;
            }
        }
        writeln!(f, "\tIOB:")?;
        for (&io, &iob) in &self.io_iob {
            writeln!(f, "\t\t{io}: {iob}")?;
        }
        if !self.io_od.is_empty() {
            writeln!(f, "\tIO_OD:")?;
            for &io in &self.io_od {
                writeln!(f, "\t\t{io}")?;
            }
        }
        writeln!(f, "\tCFG PINS:")?;
        for (k, v) in &self.cfg_io {
            writeln!(f, "\t\t{k}: {v}",)?;
        }
        Ok(())
    }
}
