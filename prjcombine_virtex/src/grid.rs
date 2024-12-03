use prjcombine_int::grid::{ColId, RowId, SimpleIoCoord};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use unnamed_entity::{EntityId, EntityIds};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum GridKind {
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
pub struct Grid {
    pub kind: GridKind,
    pub columns: usize,
    pub cols_bram: BTreeSet<ColId>,
    pub cols_clkv: Vec<(ColId, ColId, ColId)>,
    pub rows: usize,
    pub cfg_io: BTreeMap<SharedCfgPin, SimpleIoCoord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DisabledPart {
    // Virtex-E: primary DLLs are disabled
    PrimaryDlls,
    // Virtex-E: a BRAM column is disabled
    Bram(ColId),
}

impl Grid {
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

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "kind": match self.kind {
                GridKind::Virtex => "virtex",
                GridKind::VirtexE => "virtexe",
                GridKind::VirtexEM => "virtexem",
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
