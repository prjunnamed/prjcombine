use std::{collections::BTreeMap, fmt::Display};

use prjcombine_int::grid::{ColId, RowId};
use serde::{Deserialize, Serialize};
use unnamed_entity::{entity_id, EntityId};

entity_id! {
    pub id TileIobId u8;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GridKind {
    Xc3000,
    Xc3000A,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct IoCoord {
    pub col: ColId,
    pub row: RowId,
    pub iob: TileIobId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    Addr(u8),
    Data(u8),
    Ldc,
    Hdc,
    RclkB,
    Dout,
    M2,
    InitB,
    Cs0B,
    Cs1B,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub kind: GridKind,
    pub columns: usize,
    pub rows: usize,
    pub is_small: bool,
    pub cfg_io: BTreeMap<SharedCfgPin, IoCoord>,
}

impl Grid {
    pub fn col_lio(&self) -> ColId {
        ColId::from_idx(0)
    }

    pub fn col_rio(&self) -> ColId {
        ColId::from_idx(self.columns - 1)
    }

    pub fn col_mid(&self) -> ColId {
        ColId::from_idx(self.columns / 2)
    }

    pub fn row_bio(&self) -> RowId {
        RowId::from_idx(0)
    }

    pub fn row_tio(&self) -> RowId {
        RowId::from_idx(self.rows - 1)
    }

    pub fn row_mid(&self) -> RowId {
        RowId::from_idx(self.rows / 2)
    }

    pub fn io_xtl1(&self) -> IoCoord {
        IoCoord {
            col: self.col_rio(),
            row: self.row_bio(),
            iob: TileIobId::from_idx(1),
        }
    }

    pub fn io_xtl2(&self) -> IoCoord {
        IoCoord {
            col: self.col_rio(),
            row: self.row_bio(),
            iob: TileIobId::from_idx(2),
        }
    }

    pub fn io_tclk(&self) -> IoCoord {
        IoCoord {
            col: self.col_lio(),
            row: self.row_tio(),
            iob: TileIobId::from_idx(2),
        }
    }
}

impl Display for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tKIND: {:?}", self.kind)?;
        writeln!(f, "\tDIMS: {c}×{r}", c = self.columns, r = self.rows)?;
        writeln!(f, "\tSMALL: {}", self.is_small)?;
        writeln!(f, "\tCFG PINS:")?;
        for (k, v) in &self.cfg_io {
            writeln!(
                f,
                "\t\t{k:?}: IOB_X{x}Y{y}B{b}",
                x = v.col,
                y = v.row,
                b = v.iob
            )?;
        }
        Ok(())
    }
}
