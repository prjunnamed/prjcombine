use std::collections::BTreeMap;

use prjcombine_int::grid::{ColId, RowId};
use serde::{Deserialize, Serialize};
use unnamed_entity::{entity_id, EntityId};

entity_id! {
    pub id TileIobId u8;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub columns: usize,
    pub rows: usize,
    pub cfg_io: BTreeMap<SharedCfgPin, IoCoord>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    Tck,
    Tdi,
    Tms,
    Tdo,
    M0,
    M1,
    M2,
    Addr(u8),
    Data(u8),
    Ldc,
    Hdc,
    InitB,
    Cs0B,
    RsB,
    Dout,
    BusyB,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct IoCoord {
    pub col: ColId,
    pub row: RowId,
    pub iob: TileIobId,
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
}
