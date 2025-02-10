use std::collections::{BTreeMap, BTreeSet};

use enum_map::EnumMap;
use prjcombine_int::{
    db::{BelId, Dir},
    grid::{ColId, EdgeIoCoord, RowId, TileIobId},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use unnamed_entity::{EntityId, EntityIds};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GridKind {
    Ice65L01,
    Ice65L04,
    Ice65L08,
    Ice65P04,
    Ice40P01,
    Ice40P08,
    Ice40P03,
    Ice40MX,
    Ice40R04,
    Ice40T04,
    Ice40T01,
    Ice40T05,
}

impl GridKind {
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
                | Self::Ice40MX
                | Self::Ice40R04
                | Self::Ice40T04
                | Self::Ice40T01
                | Self::Ice40T05
        )
    }

    pub fn has_ice40_bramv2(self) -> bool {
        matches!(
            self,
            Self::Ice40P08
                | Self::Ice40MX
                | Self::Ice40R04
                | Self::Ice40T04
                | Self::Ice40T01
                | Self::Ice40T05
        )
    }

    pub fn has_lrio(self) -> bool {
        matches!(
            self,
            Self::Ice65L01
                | Self::Ice65L04
                | Self::Ice65L08
                | Self::Ice65P04
                | Self::Ice40P01
                | Self::Ice40P08
                | Self::Ice40P03
                | Self::Ice40MX
                | Self::Ice40R04
        )
    }

    pub fn has_actual_lrio(self) -> bool {
        matches!(
            self,
            Self::Ice65L01
                | Self::Ice65L04
                | Self::Ice65L08
                | Self::Ice65P04
                | Self::Ice40P01
                | Self::Ice40P08
                | Self::Ice40P03
                | Self::Ice40MX
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    SpiSo,
    SpiSi,
    SpiSck,
    SpiSsB,
    CbSel0,
    CbSel1,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pll {
    pub pad: EdgeIoCoord,
    pub pad_b: EdgeIoCoord,
    pub tiles: Vec<(ColId, RowId)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Grid {
    pub kind: GridKind,
    pub columns: usize,
    pub col_bio_split: ColId,
    pub cols_bram: BTreeSet<ColId>,
    pub rows: usize,
    pub row_mid: RowId,
    // (hclk row, start row, end row)
    pub rows_colbuf: Vec<(RowId, RowId, RowId)>,
    pub cfg_io: BTreeMap<SharedCfgPin, EdgeIoCoord>,
    pub io_latch: EnumMap<Dir, Option<(ColId, RowId)>>,
    pub pll: EnumMap<Dir, Option<Pll>>,
    pub gbin_io: [Option<EdgeIoCoord>; 8],
    pub gbin_fabric: [(ColId, RowId); 8],
    pub warmboot: Option<Vec<(ColId, RowId)>>,
}

impl Grid {
    pub fn col_lio(&self) -> ColId {
        ColId::from_idx(0)
    }

    pub fn col_rio(&self) -> ColId {
        ColId::from_idx(self.columns - 1)
    }

    pub fn row_bio(&self) -> RowId {
        RowId::from_idx(0)
    }

    pub fn row_tio(&self) -> RowId {
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
            EdgeIoCoord::T(_, _) => 0,
            EdgeIoCoord::R(_, _) => 1,
            EdgeIoCoord::B(col, _) => {
                if col < self.col_bio_split {
                    2
                } else if self.kind.has_lrio() {
                    4
                } else {
                    1
                }
            }
            EdgeIoCoord::L(_, _) => 3,
        }
    }

    pub fn get_io_loc(&self, io: EdgeIoCoord) -> (ColId, RowId, BelId) {
        let (col, row, iob) = match io {
            EdgeIoCoord::T(col, iob) => (col, self.row_tio(), iob),
            EdgeIoCoord::R(row, iob) => (self.col_rio(), row, iob),
            EdgeIoCoord::B(col, iob) => (col, self.row_bio(), iob),
            EdgeIoCoord::L(row, iob) => (self.col_lio(), row, iob),
        };
        let bel = BelId::from_idx(iob.to_idx());
        (col, row, bel)
    }

    pub fn get_io_crd(&self, col: ColId, row: RowId, bel: BelId) -> EdgeIoCoord {
        let iob = TileIobId::from_idx(bel.to_idx());
        if col == self.col_lio() {
            EdgeIoCoord::L(row, iob)
        } else if col == self.col_rio() {
            EdgeIoCoord::R(row, iob)
        } else if row == self.row_bio() {
            EdgeIoCoord::B(col, iob)
        } else if row == self.row_tio() {
            EdgeIoCoord::T(col, iob)
        } else {
            unreachable!()
        }
    }

    pub fn has_int_at(&self, col: ColId, row: RowId) -> bool {
        if col != self.col_lio() && col != self.col_rio() {
            return false;
        }
        if self.kind == GridKind::Ice40T01 {
            row - self.row_bio() <= 3 || self.row_tio() - row <= 3
        } else {
            true
        }
    }

    pub fn btile_width(&self, col: ColId) -> usize {
        if self.cols_bram.contains(&col) {
            42
        } else if self.kind.has_lrio() && (col == self.col_lio() || col == self.col_rio()) {
            18
        } else {
            54
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "kind": match self.kind {
                GridKind::Ice65L01 => "ice65l01",
                GridKind::Ice65L04 => "ice65l04",
                GridKind::Ice65L08 => "ice65l08",
                GridKind::Ice65P04 => "ice65p04",
                GridKind::Ice40P01 => "ice40p01",
                GridKind::Ice40P08 => "ice40p08",
                GridKind::Ice40P03 => "ice40p03",
                GridKind::Ice40MX => "ice40mx",
                GridKind::Ice40R04 => "ice40r04",
                GridKind::Ice40T04 => "ice40t04",
                GridKind::Ice40T01 => "ice40t01",
                GridKind::Ice40T05 => "ice40t05",
            },
            "columns": self.columns,
            "col_bio_split": self.col_bio_split,
            "cols_bram": self.cols_bram,
            "rows": self.rows,
            "row_mid": self.row_mid,
            "rows_colbuf": self.rows_colbuf,
            "cfg_io": serde_json::Map::from_iter(self.cfg_io.iter().map(|(k, io)| {
                (match k {
                    SharedCfgPin::SpiSo => "SPI_SO",
                    SharedCfgPin::SpiSi => "SPI_SI",
                    SharedCfgPin::SpiSck => "SPI_SCK",
                    SharedCfgPin::SpiSsB => "SPI_SS_B",
                    SharedCfgPin::CbSel0 => "CBSEL0",
                    SharedCfgPin::CbSel1 => "CBSEL1",
                }.to_string(), io.to_string().into())
            })),
            "io_latch": self.io_latch,
            // TODO: pll
            "gbin_io": self.gbin_io,
            "gbin_fabric": self.gbin_fabric,
            "warmboot": self.warmboot,
        })
    }
}

impl std::fmt::Display for Grid {
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
        for (dir, &crd) in &self.io_latch {
            if let Some((col, row)) = crd {
                writeln!(f, "\tIO LATCH {dir}: X{col}Y{row}")?;
            }
        }
        for (idx, crd) in self.gbin_io.into_iter().enumerate() {
            if let Some(crd) = crd {
                writeln!(f, "\tGB {idx} IO: {crd}")?;
            }
        }
        for (idx, (col, row)) in self.gbin_fabric.into_iter().enumerate() {
            writeln!(f, "\tGB {idx} FABRIC: X{col}Y{row}")?;
        }
        if let Some(ref tiles) = self.warmboot {
            write!(f, "\tWARMBOOT:")?;
            for &(col, row) in tiles {
                write!(f, " X{col}Y{row}")?;
            }
            writeln!(f)?;
        }
        // TODO: PLL
        writeln!(f, "\tCFG PINS:")?;
        for (k, v) in &self.cfg_io {
            writeln!(f, "\t\t{k:?}: {v}",)?;
        }
        Ok(())
    }
}
