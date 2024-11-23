use prjcombine_int::grid::{ColId, RowId};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Display};
use unnamed_entity::{entity_id, EntityId};

entity_id! {
    pub id TileIobId u8;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GridKind {
    // plain, D (no memory)
    Xc4000,
    Xc4000A,
    Xc4000H,
    // E, L, Spartan
    Xc4000E,
    // EX, XL
    Xc4000Ex,
    Xc4000Xla,
    Xc4000Xv,
    SpartanXl,
}

impl GridKind {
    pub fn is_xl(self) -> bool {
        matches!(self, Self::Xc4000Ex | Self::Xc4000Xla | Self::Xc4000Xv)
    }
    pub fn is_clb_xl(self) -> bool {
        matches!(
            self,
            Self::SpartanXl | Self::Xc4000Ex | Self::Xc4000Xla | Self::Xc4000Xv
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub kind: GridKind,
    pub columns: usize,
    pub rows: usize,
    pub cfg_io: BTreeMap<SharedCfgPin, IoCoord>,
    pub is_buff_large: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    Tck,
    Tdi,
    Tms,
    Addr(u8),
    Data(u8),
    Ldc,
    Hdc,
    InitB,
    Cs0B,
    Cs1B,
    Dout,
    RclkB,
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

    pub fn row_bio(&self) -> RowId {
        RowId::from_idx(0)
    }

    pub fn row_tio(&self) -> RowId {
        RowId::from_idx(self.rows - 1)
    }

    pub fn col_mid(&self) -> ColId {
        ColId::from_idx(self.columns / 2)
    }

    pub fn row_mid(&self) -> RowId {
        RowId::from_idx(self.rows / 2)
    }

    pub fn col_ql(&self) -> ColId {
        ColId::from_idx((self.columns + 2) / 4)
    }

    pub fn col_qr(&self) -> ColId {
        ColId::from_idx(3 * self.columns / 4)
    }

    pub fn row_qb(&self) -> RowId {
        RowId::from_idx((self.rows + 2) / 4)
    }

    pub fn row_qt(&self) -> RowId {
        RowId::from_idx(3 * self.rows / 4)
    }

    pub fn btile_height_main(&self, row: RowId) -> usize {
        if row == self.row_bio() {
            match self.kind {
                GridKind::Xc4000 | GridKind::Xc4000H | GridKind::Xc4000E | GridKind::SpartanXl => {
                    13
                }
                GridKind::Xc4000A => 10,
                GridKind::Xc4000Ex | GridKind::Xc4000Xla => 16,
                GridKind::Xc4000Xv => 17,
            }
        } else if row == self.row_tio() {
            match self.kind {
                GridKind::Xc4000 | GridKind::Xc4000H | GridKind::Xc4000E | GridKind::SpartanXl => 7,
                GridKind::Xc4000A => 6,
                GridKind::Xc4000Ex | GridKind::Xc4000Xla => 8,
                GridKind::Xc4000Xv => 9,
            }
        } else {
            match self.kind {
                GridKind::Xc4000 | GridKind::Xc4000H | GridKind::Xc4000E | GridKind::SpartanXl => {
                    10
                }
                GridKind::Xc4000A => 10,
                GridKind::Xc4000Ex | GridKind::Xc4000Xla => 12,
                GridKind::Xc4000Xv => 13,
            }
        }
    }

    pub fn btile_height_clk(&self) -> usize {
        match self.kind {
            GridKind::Xc4000 | GridKind::Xc4000A | GridKind::Xc4000H | GridKind::Xc4000E => 1,
            GridKind::Xc4000Ex | GridKind::Xc4000Xla | GridKind::Xc4000Xv | GridKind::SpartanXl => {
                2
            }
        }
    }

    pub fn btile_height_brk(&self) -> usize {
        2
    }

    pub fn btile_width_main(&self, col: ColId) -> usize {
        if col == self.col_lio() {
            match self.kind {
                GridKind::Xc4000 | GridKind::Xc4000H | GridKind::Xc4000E | GridKind::SpartanXl => {
                    26
                }
                GridKind::Xc4000A => 21,
                GridKind::Xc4000Ex | GridKind::Xc4000Xla | GridKind::Xc4000Xv => 27,
            }
        } else if col == self.col_rio() {
            match self.kind {
                GridKind::Xc4000 | GridKind::Xc4000H | GridKind::Xc4000E | GridKind::SpartanXl => {
                    41
                }
                GridKind::Xc4000A => 32,
                GridKind::Xc4000Ex | GridKind::Xc4000Xla | GridKind::Xc4000Xv => 52,
            }
        } else {
            match self.kind {
                GridKind::Xc4000 | GridKind::Xc4000H | GridKind::Xc4000E | GridKind::SpartanXl => {
                    36
                }
                GridKind::Xc4000A => 32,
                GridKind::Xc4000Ex | GridKind::Xc4000Xla | GridKind::Xc4000Xv => 47,
            }
        }
    }

    pub fn btile_width_clk(&self) -> usize {
        match self.kind {
            GridKind::Xc4000 | GridKind::Xc4000H | GridKind::Xc4000A | GridKind::Xc4000E => 1,
            GridKind::Xc4000Ex | GridKind::Xc4000Xla | GridKind::Xc4000Xv | GridKind::SpartanXl => {
                2
            }
        }
    }

    pub fn btile_width_brk(&self) -> usize {
        1
    }
}

impl Display for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tKIND: {k:?}", k = self.kind)?;
        writeln!(f, "\tDIMS: {c}×{r}", c = self.columns, r = self.rows)?;
        writeln!(f, "\tIS BUFF LARGE: {v}", v = self.is_buff_large)?;
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
