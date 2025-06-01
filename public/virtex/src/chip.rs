use jzon::JsonValue;
use prjcombine_interconnect::grid::{BelCoord, ColId, DieId, EdgeIoCoord, RowId, TileIobId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use unnamed_entity::{EntityId, EntityIds};

use crate::bels;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ChipKind {
    Virtex,
    VirtexE,
    VirtexEM,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    Data(u8), // ×8
    CsB,
    InitB,
    RdWrB,
    Dout,
}

impl std::fmt::Display for SharedCfgPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SharedCfgPin::Data(i) => write!(f, "D{i}"),
            SharedCfgPin::CsB => write!(f, "CS_B"),
            SharedCfgPin::RdWrB => write!(f, "RDWR_B"),
            SharedCfgPin::Dout => write!(f, "DOUT"),
            SharedCfgPin::InitB => write!(f, "INIT_B"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Chip {
    pub kind: ChipKind,
    pub columns: usize,
    pub cols_bram: BTreeSet<ColId>,
    pub cols_clkv: Vec<(ColId, ColId, ColId)>,
    pub rows: usize,
    pub cfg_io: BTreeMap<SharedCfgPin, EdgeIoCoord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DisabledPart {
    // Virtex-E: primary DLLs are disabled
    PrimaryDlls,
    // Virtex-E: a BRAM column is disabled
    Bram(ColId),
}

impl std::fmt::Display for DisabledPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisabledPart::PrimaryDlls => write!(f, "PRIMARY_DLLS"),
            DisabledPart::Bram(col) => write!(f, "BRAM_COL:{col}"),
        }
    }
}

impl Chip {
    pub fn row_mid(&self) -> RowId {
        RowId::from_idx(self.rows / 2)
    }

    pub fn row_clk(&self) -> RowId {
        match self.rows % 8 {
            2 => RowId::from_idx(self.rows / 2),
            6 => RowId::from_idx(self.rows / 2 - 2),
            _ => unreachable!(),
        }
    }

    pub fn col_clk(&self) -> ColId {
        ColId::from_idx(self.columns / 2)
    }

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

    pub fn columns(&self) -> EntityIds<ColId> {
        EntityIds::new(self.columns)
    }

    pub fn rows(&self) -> EntityIds<RowId> {
        EntityIds::new(self.rows)
    }

    pub fn get_io_bank(&self, io: EdgeIoCoord) -> u32 {
        match io {
            EdgeIoCoord::N(col, _) => {
                if col < self.col_clk() {
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
                if col < self.col_clk() {
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

    pub fn get_bonded_ios(&self) -> Vec<EdgeIoCoord> {
        let mut res = vec![];
        for col in self.columns() {
            if self.cols_bram.contains(&col) {
                continue;
            }
            if col == self.col_w() || col == self.col_e() {
                continue;
            }
            for iob in [2, 1] {
                res.push(EdgeIoCoord::N(col, TileIobId::from_idx(iob)));
            }
        }
        for row in self.rows().rev() {
            if row == self.row_s() || row == self.row_n() {
                continue;
            }
            for iob in [1, 2, 3] {
                res.push(EdgeIoCoord::E(row, TileIobId::from_idx(iob)));
            }
        }
        for col in self.columns().rev() {
            if self.cols_bram.contains(&col) {
                continue;
            }
            if col == self.col_w() || col == self.col_e() {
                continue;
            }
            for iob in [1, 2] {
                res.push(EdgeIoCoord::S(col, TileIobId::from_idx(iob)));
            }
        }
        for row in self.rows() {
            if row == self.row_s() || row == self.row_n() {
                continue;
            }
            for iob in [3, 2, 1] {
                res.push(EdgeIoCoord::W(row, TileIobId::from_idx(iob)));
            }
        }
        res
    }
}

impl From<&Chip> for JsonValue {
    fn from(chip: &Chip) -> Self {
        jzon::object! {
            kind: match chip.kind {
                ChipKind::Virtex => "virtex",
                ChipKind::VirtexE => "virtexe",
                ChipKind::VirtexEM => "virtexem",
            },
            columns: chip.columns,
            cols_bram: Vec::from_iter(chip.cols_bram.iter().map(|x| x.to_idx())),
            cols_clkv: Vec::from_iter(chip.cols_clkv.iter().map(|(col_mid, col_start, col_end)| {
                jzon::array![col_mid.to_idx(), col_start.to_idx(), col_end.to_idx()]
            })),
            rows: chip.rows,
            cfg_io: jzon::object::Object::from_iter(chip.cfg_io.iter().map(|(k, io)| {
                (k.to_string(), io.to_string())
            })),
        }
    }
}

impl std::fmt::Display for Chip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tKIND: {k:?}", k = self.kind)?;
        writeln!(f, "\tDIMS: {c}×{r}", c = self.columns, r = self.rows)?;
        writeln!(f, "\tCOLS:")?;
        let mut clkv_idx = 0;
        for col in self.columns() {
            if col == self.cols_clkv[clkv_idx].0 {
                writeln!(f, "\t\t--- clock column")?;
            }
            if col == self.cols_clkv[clkv_idx].2 {
                writeln!(f, "\t\t--- clock break")?;
                clkv_idx += 1;
            }
            writeln!(
                f,
                "\t\tX{c}: {kind}",
                c = col.to_idx(),
                kind = if self.cols_bram.contains(&col) {
                    "BRAM"
                } else if col == self.col_w() {
                    "LIO"
                } else if col == self.col_e() {
                    "RIO"
                } else {
                    "CLB"
                }
            )?;
        }
        writeln!(f, "\tCFG PINS:")?;
        for (k, v) in &self.cfg_io {
            writeln!(f, "\t\t{k}: {v}",)?;
        }
        Ok(())
    }
}
