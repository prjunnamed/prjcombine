use prjcombine_interconnect::{
    db::BelId,
    grid::{ColId, EdgeIoCoord, RowId, TileIobId},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use unnamed_entity::{EntityId, EntityIds};

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

    pub fn columns(&self) -> EntityIds<ColId> {
        EntityIds::new(self.columns)
    }

    pub fn rows(&self) -> EntityIds<RowId> {
        EntityIds::new(self.rows)
    }

    pub fn get_io_bank(&self, io: EdgeIoCoord) -> u32 {
        match io {
            EdgeIoCoord::T(col, _) => {
                if col < self.col_clk() {
                    0
                } else {
                    1
                }
            }
            EdgeIoCoord::R(row, _) => {
                if row < self.row_mid() {
                    3
                } else {
                    2
                }
            }
            EdgeIoCoord::B(col, _) => {
                if col < self.col_clk() {
                    5
                } else {
                    4
                }
            }
            EdgeIoCoord::L(row, _) => {
                if row < self.row_mid() {
                    6
                } else {
                    7
                }
            }
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

    pub fn get_bonded_ios(&self) -> Vec<EdgeIoCoord> {
        let mut res = vec![];
        for col in self.columns() {
            if self.cols_bram.contains(&col) {
                continue;
            }
            if col == self.col_lio() || col == self.col_rio() {
                continue;
            }
            for iob in [2, 1] {
                res.push(EdgeIoCoord::T(col, TileIobId::from_idx(iob)));
            }
        }
        for row in self.rows().rev() {
            if row == self.row_bio() || row == self.row_tio() {
                continue;
            }
            for iob in [1, 2, 3] {
                res.push(EdgeIoCoord::R(row, TileIobId::from_idx(iob)));
            }
        }
        for col in self.columns().rev() {
            if self.cols_bram.contains(&col) {
                continue;
            }
            if col == self.col_lio() || col == self.col_rio() {
                continue;
            }
            for iob in [1, 2] {
                res.push(EdgeIoCoord::B(col, TileIobId::from_idx(iob)));
            }
        }
        for row in self.rows() {
            if row == self.row_bio() || row == self.row_tio() {
                continue;
            }
            for iob in [3, 2, 1] {
                res.push(EdgeIoCoord::L(row, TileIobId::from_idx(iob)));
            }
        }
        res
    }

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "kind": match self.kind {
                ChipKind::Virtex => "virtex",
                ChipKind::VirtexE => "virtexe",
                ChipKind::VirtexEM => "virtexem",
            },
            "columns": self.columns,
            "cols_bram": self.cols_bram,
            "cols_clkv": self.cols_clkv,
            "rows": self.rows,
            "cfg_io": serde_json::Map::from_iter(self.cfg_io.iter().map(|(k, io)| {
                (match k {
                    SharedCfgPin::Data(i) => format!("D{i}"),
                    SharedCfgPin::CsB => "CS_B".to_string(),
                    SharedCfgPin::RdWrB => "RDWR_B".to_string(),
                    SharedCfgPin::Dout => "DOUT".to_string(),
                    SharedCfgPin::InitB => "INIT_B".to_string(),
                }, io.to_string().into())
            }))
        })
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
                } else if col == self.col_lio() {
                    "LIO"
                } else if col == self.col_rio() {
                    "RIO"
                } else {
                    "CLB"
                }
            )?;
        }
        writeln!(f, "\tCFG PINS:")?;
        for (k, v) in &self.cfg_io {
            writeln!(f, "\t\t{k:?}: {v}",)?;
        }
        Ok(())
    }
}
