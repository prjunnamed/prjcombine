use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
};

use prjcombine_int::grid::{ColId, RowId};
use serde::{Deserialize, Serialize};
use unnamed_entity::{entity_id, EntityId};

entity_id! {
    pub id TileIobId u8;
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
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub columns: usize,
    pub rows: usize,
    pub cols_bidi: BTreeSet<ColId>,
    pub rows_bidi: BTreeSet<RowId>,
    pub cfg_io: BTreeMap<SharedCfgPin, IoCoord>,
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

    pub fn row_mid(&self) -> RowId {
        RowId::from_idx(self.rows / 2 - 1)
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
}

impl Display for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tKIND: Xc2000")?;
        writeln!(f, "\tDIMS: {c}×{r}", c = self.columns, r = self.rows)?;
        write!(f, "\tBIDI COLS:")?;
        for &col in &self.cols_bidi {
            write!(f, " {col}")?;
        }
        writeln!(f)?;
        write!(f, "\tBIDI ROWS:")?;
        for &row in &self.rows_bidi {
            write!(f, " {row}")?;
        }
        writeln!(f)?;
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
