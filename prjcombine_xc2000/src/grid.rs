use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
};

use prjcombine_int::grid::{ColId, RowId, SimpleIoCoord, TileIobId};
use serde::{Deserialize, Serialize};
use unnamed_entity::EntityId;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    Addr(u8),
    Data(u8),
    Ldc,
    Hdc,
    RclkB,
    Dout,
    M2, // dedicated on XC4000
    // XC3000+
    InitB,
    Cs0B,
    Cs1B,
    // XC4000+
    Tck,
    Tdi,
    Tms,
    // XC5200 only
    Tdo,
    M0,
    M1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GridKind {
    Xc2000,
    Xc3000,
    Xc3000A,
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
    Xc5200,
}

impl GridKind {
    pub fn is_xc3000(self) -> bool {
        matches!(self, Self::Xc3000 | Self::Xc3000A)
    }
    pub fn is_xc4000(self) -> bool {
        matches!(
            self,
            Self::Xc4000
                | Self::Xc4000A
                | Self::Xc4000H
                | Self::Xc4000E
                | Self::Xc4000Ex
                | Self::Xc4000Xla
                | Self::Xc4000Xv
                | Self::SpartanXl
        )
    }
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
    // XC3000 only
    pub is_small: bool,
    // XC4000X only
    pub is_buff_large: bool,
    // XC2000 only
    pub cols_bidi: BTreeSet<ColId>,
    pub rows_bidi: BTreeSet<RowId>,
    pub cfg_io: BTreeMap<SharedCfgPin, SimpleIoCoord>,
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

    pub fn io_xtl1(&self) -> SimpleIoCoord {
        SimpleIoCoord {
            col: self.col_rio(),
            row: self.row_bio(),
            iob: TileIobId::from_idx(1),
        }
    }

    pub fn io_xtl2(&self) -> SimpleIoCoord {
        SimpleIoCoord {
            col: self.col_rio(),
            row: self.row_bio(),
            iob: TileIobId::from_idx(2),
        }
    }

    pub fn io_tclk(&self) -> SimpleIoCoord {
        assert!(self.kind.is_xc3000());
        SimpleIoCoord {
            col: self.col_lio(),
            row: self.row_tio(),
            iob: TileIobId::from_idx(2),
        }
    }

    pub fn btile_height_main(&self, row: RowId) -> usize {
        if row == self.row_bio() {
            match self.kind {
                GridKind::Xc2000 => todo!(),
                GridKind::Xc3000 | GridKind::Xc3000A => todo!(),
                GridKind::Xc4000 | GridKind::Xc4000H | GridKind::Xc4000E | GridKind::SpartanXl => {
                    13
                }
                GridKind::Xc4000A => 10,
                GridKind::Xc4000Ex | GridKind::Xc4000Xla => 16,
                GridKind::Xc4000Xv => 17,
                GridKind::Xc5200 => 28,
            }
        } else if row == self.row_tio() {
            match self.kind {
                GridKind::Xc2000 => todo!(),
                GridKind::Xc3000 | GridKind::Xc3000A => todo!(),
                GridKind::Xc4000 | GridKind::Xc4000H | GridKind::Xc4000E | GridKind::SpartanXl => 7,
                GridKind::Xc4000A => 6,
                GridKind::Xc4000Ex | GridKind::Xc4000Xla => 8,
                GridKind::Xc4000Xv => 9,
                GridKind::Xc5200 => 28,
            }
        } else {
            match self.kind {
                GridKind::Xc2000 => todo!(),
                GridKind::Xc3000 | GridKind::Xc3000A => todo!(),
                GridKind::Xc4000 | GridKind::Xc4000H | GridKind::Xc4000E | GridKind::SpartanXl => {
                    10
                }
                GridKind::Xc4000A => 10,
                GridKind::Xc4000Ex | GridKind::Xc4000Xla => 12,
                GridKind::Xc4000Xv => 13,
                GridKind::Xc5200 => 34,
            }
        }
    }

    pub fn btile_height_clk(&self) -> usize {
        match self.kind {
            GridKind::Xc2000 => unreachable!(),
            GridKind::Xc3000 | GridKind::Xc3000A => unreachable!(),
            GridKind::Xc4000 | GridKind::Xc4000A | GridKind::Xc4000H | GridKind::Xc4000E => 1,
            GridKind::Xc4000Ex | GridKind::Xc4000Xla | GridKind::Xc4000Xv | GridKind::SpartanXl => {
                2
            }
            GridKind::Xc5200 => 4,
        }
    }

    pub fn btile_height_brk(&self) -> usize {
        2
    }

    pub fn btile_width_main(&self, col: ColId) -> usize {
        if col == self.col_lio() {
            match self.kind {
                GridKind::Xc2000 => todo!(),
                GridKind::Xc3000 | GridKind::Xc3000A => todo!(),
                GridKind::Xc4000 | GridKind::Xc4000H | GridKind::Xc4000E | GridKind::SpartanXl => {
                    26
                }
                GridKind::Xc4000A => 21,
                GridKind::Xc4000Ex | GridKind::Xc4000Xla | GridKind::Xc4000Xv => 27,
                GridKind::Xc5200 => 7,
            }
        } else if col == self.col_rio() {
            match self.kind {
                GridKind::Xc2000 => todo!(),
                GridKind::Xc3000 | GridKind::Xc3000A => todo!(),
                GridKind::Xc4000 | GridKind::Xc4000H | GridKind::Xc4000E | GridKind::SpartanXl => {
                    41
                }
                GridKind::Xc4000A => 32,
                GridKind::Xc4000Ex | GridKind::Xc4000Xla | GridKind::Xc4000Xv => 52,
                GridKind::Xc5200 => 8,
            }
        } else {
            match self.kind {
                GridKind::Xc2000 => todo!(),
                GridKind::Xc3000 | GridKind::Xc3000A => todo!(),
                GridKind::Xc4000 | GridKind::Xc4000H | GridKind::Xc4000E | GridKind::SpartanXl => {
                    36
                }
                GridKind::Xc4000A => 32,
                GridKind::Xc4000Ex | GridKind::Xc4000Xla | GridKind::Xc4000Xv => 47,
                GridKind::Xc5200 => 12,
            }
        }
    }

    pub fn btile_width_clk(&self) -> usize {
        match self.kind {
            GridKind::Xc2000 => unreachable!(),
            GridKind::Xc3000 | GridKind::Xc3000A => unreachable!(),
            GridKind::Xc4000 | GridKind::Xc4000H | GridKind::Xc4000A | GridKind::Xc4000E => 1,
            GridKind::Xc4000Ex | GridKind::Xc4000Xla | GridKind::Xc4000Xv | GridKind::SpartanXl => {
                2
            }
            GridKind::Xc5200 => 1,
        }
    }

    pub fn btile_width_brk(&self) -> usize {
        1
    }
}

impl Display for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tKIND: {:?}", self.kind)?;
        writeln!(f, "\tDIMS: {c}×{r}", c = self.columns, r = self.rows)?;
        writeln!(f, "\tSMALL: {}", self.is_small)?;
        writeln!(f, "\tBUFF LARGE: {v}", v = self.is_buff_large)?;
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
