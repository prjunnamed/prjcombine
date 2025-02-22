use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
};

use jzon::JsonValue;
use prjcombine_interconnect::{
    db::BelId,
    grid::{ColId, EdgeIoCoord, RowId, TileIobId},
};
use serde::{Deserialize, Serialize};
use unnamed_entity::{EntityId, EntityIds};

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

impl std::fmt::Display for SharedCfgPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SharedCfgPin::Addr(i) => write!(f, "A{i}"),
            SharedCfgPin::Data(i) => write!(f, "D{i}"),
            SharedCfgPin::Ldc => write!(f, "LDC"),
            SharedCfgPin::Hdc => write!(f, "HDC"),
            SharedCfgPin::RclkB => write!(f, "RCLK_B"),
            SharedCfgPin::Dout => write!(f, "DOUT"),
            SharedCfgPin::M2 => write!(f, "M2"),
            SharedCfgPin::InitB => write!(f, "INIT_B"),
            SharedCfgPin::Cs0B => write!(f, "CS0_B"),
            SharedCfgPin::Cs1B => write!(f, "CS1_B"),
            SharedCfgPin::Tck => write!(f, "TCK"),
            SharedCfgPin::Tdi => write!(f, "TDI"),
            SharedCfgPin::Tms => write!(f, "TMS"),
            SharedCfgPin::Tdo => write!(f, "TDO"),
            SharedCfgPin::M0 => write!(f, "M0"),
            SharedCfgPin::M1 => write!(f, "M1"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ChipKind {
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

impl ChipKind {
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

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Chip {
    pub kind: ChipKind,
    pub columns: usize,
    pub rows: usize,
    // XC3000 only
    pub is_small: bool,
    // XC4000X only
    pub is_buff_large: bool,
    // XC2000 only
    pub cols_bidi: BTreeSet<ColId>,
    pub rows_bidi: BTreeSet<RowId>,
    pub cfg_io: BTreeMap<SharedCfgPin, EdgeIoCoord>,
    pub unbonded_io: BTreeSet<EdgeIoCoord>,
}

impl Chip {
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

    pub fn columns(&self) -> EntityIds<ColId> {
        EntityIds::new(self.columns)
    }

    pub fn rows(&self) -> EntityIds<RowId> {
        EntityIds::new(self.rows)
    }

    pub fn get_io_crd(&self, col: ColId, row: RowId, bel: BelId) -> EdgeIoCoord {
        match self.kind {
            ChipKind::Xc2000 | ChipKind::Xc3000 | ChipKind::Xc3000A => {
                let iob = if bel.to_idx() < 3 {
                    TileIobId::from_idx(bel.to_idx() - 1)
                } else {
                    TileIobId::from_idx(bel.to_idx() - 3)
                };
                if row == self.row_bio() && bel.to_idx() < 3 {
                    EdgeIoCoord::S(col, iob)
                } else if row == self.row_tio() && bel.to_idx() < 3 {
                    EdgeIoCoord::N(col, iob)
                } else if col == self.col_lio() {
                    EdgeIoCoord::W(row, iob)
                } else if col == self.col_rio() {
                    EdgeIoCoord::E(row, iob)
                } else {
                    unreachable!()
                }
            }
            ChipKind::Xc4000
            | ChipKind::Xc4000A
            | ChipKind::Xc4000H
            | ChipKind::Xc4000E
            | ChipKind::Xc4000Ex
            | ChipKind::Xc4000Xla
            | ChipKind::Xc4000Xv
            | ChipKind::SpartanXl
            | ChipKind::Xc5200 => {
                let iob = TileIobId::from_idx(bel.to_idx());
                if col == self.col_lio() {
                    EdgeIoCoord::W(row, iob)
                } else if col == self.col_rio() {
                    EdgeIoCoord::E(row, iob)
                } else if row == self.row_bio() {
                    EdgeIoCoord::S(col, iob)
                } else if row == self.row_tio() {
                    EdgeIoCoord::N(col, iob)
                } else {
                    unreachable!()
                }
            }
        }
    }

    pub fn get_io_loc(&self, io: EdgeIoCoord) -> (ColId, RowId, BelId) {
        match self.kind {
            ChipKind::Xc2000 | ChipKind::Xc3000 | ChipKind::Xc3000A => match io {
                EdgeIoCoord::N(col, iob) => {
                    (col, self.row_tio(), BelId::from_idx(1 + iob.to_idx()))
                }
                EdgeIoCoord::E(row, iob) => {
                    let bel = if row == self.row_bio() || row == self.row_tio() {
                        BelId::from_idx(3 + iob.to_idx())
                    } else {
                        BelId::from_idx(1 + iob.to_idx())
                    };
                    (self.col_rio(), row, bel)
                }
                EdgeIoCoord::S(col, iob) => {
                    (col, self.row_bio(), BelId::from_idx(1 + iob.to_idx()))
                }
                EdgeIoCoord::W(row, iob) => {
                    let bel = if row == self.row_bio() || row == self.row_tio() {
                        BelId::from_idx(3 + iob.to_idx())
                    } else {
                        BelId::from_idx(1 + iob.to_idx())
                    };
                    (self.col_lio(), row, bel)
                }
            },
            ChipKind::Xc4000
            | ChipKind::Xc4000A
            | ChipKind::Xc4000H
            | ChipKind::Xc4000E
            | ChipKind::Xc4000Ex
            | ChipKind::Xc4000Xla
            | ChipKind::Xc4000Xv
            | ChipKind::SpartanXl
            | ChipKind::Xc5200 => {
                let (col, row, iob) = match io {
                    EdgeIoCoord::N(col, iob) => (col, self.row_tio(), iob),
                    EdgeIoCoord::E(row, iob) => (self.col_rio(), row, iob),
                    EdgeIoCoord::S(col, iob) => (col, self.row_bio(), iob),
                    EdgeIoCoord::W(row, iob) => (self.col_lio(), row, iob),
                };
                (col, row, BelId::from_idx(iob.to_idx()))
            }
        }
    }

    pub fn get_bonded_ios(&self) -> Vec<EdgeIoCoord> {
        let mut res = vec![];
        match self.kind {
            ChipKind::Xc2000 => {
                for col in self.columns() {
                    for iob in [0, 1] {
                        res.push(EdgeIoCoord::N(col, TileIobId::from_idx(iob)));
                    }
                }
                for row in self.rows().rev() {
                    if row == self.row_bio() || row == self.row_tio() || row == self.row_mid() - 1 {
                        res.push(EdgeIoCoord::E(row, TileIobId::from_idx(0)));
                    } else {
                        for iob in [0, 1] {
                            res.push(EdgeIoCoord::E(row, TileIobId::from_idx(iob)));
                        }
                    }
                }
                for col in self.columns().rev() {
                    for iob in [1, 0] {
                        res.push(EdgeIoCoord::S(col, TileIobId::from_idx(iob)));
                    }
                }
                for row in self.rows() {
                    if row == self.row_bio() || row == self.row_tio() || row == self.row_mid() - 1 {
                        res.push(EdgeIoCoord::W(row, TileIobId::from_idx(0)));
                    } else {
                        for iob in [1, 0] {
                            res.push(EdgeIoCoord::W(row, TileIobId::from_idx(iob)));
                        }
                    }
                }
            }
            ChipKind::Xc3000 | ChipKind::Xc3000A => {
                for col in self.columns() {
                    for iob in [0, 1] {
                        res.push(EdgeIoCoord::N(col, TileIobId::from_idx(iob)));
                    }
                }
                for row in self.rows().rev() {
                    for iob in [0, 1] {
                        res.push(EdgeIoCoord::E(row, TileIobId::from_idx(iob)));
                    }
                }
                for col in self.columns().rev() {
                    for iob in [1, 0] {
                        res.push(EdgeIoCoord::S(col, TileIobId::from_idx(iob)));
                    }
                }
                for row in self.rows() {
                    for iob in [1, 0] {
                        res.push(EdgeIoCoord::W(row, TileIobId::from_idx(iob)));
                    }
                }
            }
            ChipKind::Xc4000
            | ChipKind::Xc4000A
            | ChipKind::Xc4000H
            | ChipKind::Xc4000E
            | ChipKind::Xc4000Ex
            | ChipKind::Xc4000Xla
            | ChipKind::Xc4000Xv
            | ChipKind::SpartanXl => {
                let iobs = if self.kind == ChipKind::Xc4000H {
                    0..4
                } else {
                    0..2
                };
                for col in self.columns() {
                    if col == self.col_lio() || col == self.col_rio() {
                        continue;
                    }
                    for iob in iobs.clone() {
                        res.push(EdgeIoCoord::N(col, TileIobId::from_idx(iob)));
                    }
                }
                for row in self.rows().rev() {
                    if row == self.row_bio() || row == self.row_tio() {
                        continue;
                    }
                    for iob in iobs.clone() {
                        res.push(EdgeIoCoord::E(row, TileIobId::from_idx(iob)));
                    }
                }
                for col in self.columns().rev() {
                    if col == self.col_lio() || col == self.col_rio() {
                        continue;
                    }
                    for iob in iobs.clone().rev() {
                        res.push(EdgeIoCoord::S(col, TileIobId::from_idx(iob)));
                    }
                }
                for row in self.rows() {
                    if row == self.row_bio() || row == self.row_tio() {
                        continue;
                    }
                    for iob in iobs.clone().rev() {
                        res.push(EdgeIoCoord::W(row, TileIobId::from_idx(iob)));
                    }
                }
            }
            ChipKind::Xc5200 => {
                for col in self.columns() {
                    if col == self.col_lio() || col == self.col_rio() {
                        continue;
                    }
                    for iob in [3, 2, 1, 0] {
                        res.push(EdgeIoCoord::N(col, TileIobId::from_idx(iob)));
                    }
                }
                for row in self.rows().rev() {
                    if row == self.row_bio() || row == self.row_tio() {
                        continue;
                    }
                    for iob in [3, 2, 1, 0] {
                        res.push(EdgeIoCoord::E(row, TileIobId::from_idx(iob)));
                    }
                }
                for col in self.columns().rev() {
                    if col == self.col_lio() || col == self.col_rio() {
                        continue;
                    }
                    for iob in [0, 1, 2, 3] {
                        res.push(EdgeIoCoord::S(col, TileIobId::from_idx(iob)));
                    }
                }
                for row in self.rows() {
                    if row == self.row_bio() || row == self.row_tio() {
                        continue;
                    }
                    for iob in [0, 1, 2, 3] {
                        res.push(EdgeIoCoord::W(row, TileIobId::from_idx(iob)));
                    }
                }
            }
        }
        res
    }

    pub fn io_xtl1(&self) -> EdgeIoCoord {
        EdgeIoCoord::S(self.col_rio(), TileIobId::from_idx(1))
    }

    pub fn io_xtl2(&self) -> EdgeIoCoord {
        EdgeIoCoord::E(self.row_bio(), TileIobId::from_idx(0))
    }

    pub fn io_tclk(&self) -> EdgeIoCoord {
        assert!(self.kind.is_xc3000());
        EdgeIoCoord::W(self.row_tio(), TileIobId::from_idx(0))
    }

    pub fn btile_height_main(&self, row: RowId) -> usize {
        if row == self.row_bio() {
            match self.kind {
                ChipKind::Xc2000 => 12,
                ChipKind::Xc3000 | ChipKind::Xc3000A => 13,
                ChipKind::Xc4000 | ChipKind::Xc4000H | ChipKind::Xc4000E | ChipKind::SpartanXl => {
                    13
                }
                ChipKind::Xc4000A => 10,
                ChipKind::Xc4000Ex | ChipKind::Xc4000Xla => 16,
                ChipKind::Xc4000Xv => 17,
                ChipKind::Xc5200 => 28,
            }
        } else if row == self.row_tio() {
            match self.kind {
                ChipKind::Xc2000 => 9,
                ChipKind::Xc3000 | ChipKind::Xc3000A => 10,
                ChipKind::Xc4000 | ChipKind::Xc4000H | ChipKind::Xc4000E | ChipKind::SpartanXl => 7,
                ChipKind::Xc4000A => 6,
                ChipKind::Xc4000Ex | ChipKind::Xc4000Xla => 8,
                ChipKind::Xc4000Xv => 9,
                ChipKind::Xc5200 => 28,
            }
        } else {
            match self.kind {
                ChipKind::Xc2000 => 8,
                ChipKind::Xc3000 | ChipKind::Xc3000A => 8,
                ChipKind::Xc4000 | ChipKind::Xc4000H | ChipKind::Xc4000E | ChipKind::SpartanXl => {
                    10
                }
                ChipKind::Xc4000A => 10,
                ChipKind::Xc4000Ex | ChipKind::Xc4000Xla => 12,
                ChipKind::Xc4000Xv => 13,
                ChipKind::Xc5200 => 34,
            }
        }
    }

    pub fn btile_height_clk(&self) -> usize {
        match self.kind {
            ChipKind::Xc2000 => unreachable!(),
            ChipKind::Xc3000 | ChipKind::Xc3000A => 1,
            ChipKind::Xc4000 | ChipKind::Xc4000A | ChipKind::Xc4000H | ChipKind::Xc4000E => 1,
            ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl => {
                2
            }
            ChipKind::Xc5200 => 4,
        }
    }

    pub fn btile_height_brk(&self) -> usize {
        if self.kind == ChipKind::Xc2000 { 1 } else { 2 }
    }

    pub fn btile_width_main(&self, col: ColId) -> usize {
        if col == self.col_lio() {
            match self.kind {
                ChipKind::Xc2000 => 21,
                ChipKind::Xc3000 | ChipKind::Xc3000A => 29,
                ChipKind::Xc4000 | ChipKind::Xc4000H | ChipKind::Xc4000E | ChipKind::SpartanXl => {
                    26
                }
                ChipKind::Xc4000A => 21,
                ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv => 27,
                ChipKind::Xc5200 => 7,
            }
        } else if col == self.col_rio() {
            match self.kind {
                ChipKind::Xc2000 => 27,
                ChipKind::Xc3000 | ChipKind::Xc3000A => 36,
                ChipKind::Xc4000 | ChipKind::Xc4000H | ChipKind::Xc4000E | ChipKind::SpartanXl => {
                    41
                }
                ChipKind::Xc4000A => 32,
                ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv => 52,
                ChipKind::Xc5200 => 8,
            }
        } else {
            match self.kind {
                ChipKind::Xc2000 => 18,
                ChipKind::Xc3000 | ChipKind::Xc3000A => 22,
                ChipKind::Xc4000 | ChipKind::Xc4000H | ChipKind::Xc4000E | ChipKind::SpartanXl => {
                    36
                }
                ChipKind::Xc4000A => 32,
                ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv => 47,
                ChipKind::Xc5200 => 12,
            }
        }
    }

    pub fn btile_width_clk(&self) -> usize {
        match self.kind {
            ChipKind::Xc2000 => unreachable!(),
            ChipKind::Xc3000 | ChipKind::Xc3000A => unreachable!(),
            ChipKind::Xc4000 | ChipKind::Xc4000H | ChipKind::Xc4000A | ChipKind::Xc4000E => 1,
            ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl => {
                2
            }
            ChipKind::Xc5200 => 1,
        }
    }

    pub fn btile_width_brk(&self) -> usize {
        if self.kind == ChipKind::Xc2000 { 2 } else { 1 }
    }
}

impl From<&Chip> for JsonValue {
    fn from(chip: &Chip) -> Self {
        jzon::object! {
            kind: match chip.kind {
                ChipKind::Xc2000 => "xc2000",
                ChipKind::Xc3000 => "xc3000",
                ChipKind::Xc3000A => "xc3000a",
                ChipKind::Xc4000 => "xc4000",
                ChipKind::Xc4000A => "xc4000a",
                ChipKind::Xc4000H => "xc4000h",
                ChipKind::Xc4000E => "xc4000e",
                ChipKind::Xc4000Ex => "xc4000ex",
                ChipKind::Xc4000Xla => "xc4000xla",
                ChipKind::Xc4000Xv => "xc4000xv",
                ChipKind::SpartanXl => "spartanxl",
                ChipKind::Xc5200 => "xc5200",
            },
            columns: chip.columns,
            rows: chip.rows,
            is_small: chip.is_small,
            is_buff_large: chip.is_buff_large,
            cols_bidi: Vec::from_iter(chip.cols_bidi.iter().map(|col| col.to_idx())),
            rows_bidi: Vec::from_iter(chip.cols_bidi.iter().map(|row| row.to_idx())),
            cfg_io: jzon::object::Object::from_iter(chip.cfg_io.iter().map(|(k, io)| {
                (k.to_string(), io.to_string())
            })),
            unbonded_io: Vec::from_iter(chip.unbonded_io.iter().map(|&io| io.to_string())),
        }
    }
}

impl Display for Chip {
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
            writeln!(f, "\t\t{k}: {v}")?;
        }
        if !self.unbonded_io.is_empty() {
            writeln!(f, "\tUNBONDED IO:")?;
            for &io in &self.unbonded_io {
                writeln!(f, "\t\t{io}")?;
            }
        }
        Ok(())
    }
}
