use bincode::{Decode, Encode};
use itertools::Itertools;
use prjcombine_entity::{EntityId, EntityRange};
use prjcombine_interconnect::{
    dir::{DirH, DirHV, DirV},
    grid::{BelCoord, ColId, DieId, DieIdExt, EdgeIoCoord, RowId, TileCoord, TileIobId},
};
use std::collections::{BTreeMap, BTreeSet};

use crate::defs::{self, tslots};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum ChipKind {
    Virtex,
    VirtexE,
    VirtexEM,
}

impl std::fmt::Display for ChipKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChipKind::Virtex => write!(f, "virtex"),
            ChipKind::VirtexE => write!(f, "virtexe"),
            ChipKind::VirtexEM => write!(f, "virtexem"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum SharedCfgPad {
    Data(u8), // ×8
    CsB,
    InitB,
    RdWrB,
    Dout,
}

impl std::fmt::Display for SharedCfgPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SharedCfgPad::Data(i) => write!(f, "D{i}"),
            SharedCfgPad::CsB => write!(f, "CS_B"),
            SharedCfgPad::RdWrB => write!(f, "RDWR_B"),
            SharedCfgPad::Dout => write!(f, "DOUT"),
            SharedCfgPad::InitB => write!(f, "INIT_B"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Chip {
    pub kind: ChipKind,
    pub columns: usize,
    pub cols_bram: BTreeSet<ColId>,
    pub cols_clkv: Vec<(ColId, ColId, ColId)>,
    pub rows: usize,
    pub cfg_io: BTreeMap<SharedCfgPad, EdgeIoCoord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
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
    // always single chip
    pub const DIE: DieId = DieId::from_idx_const(0);

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

    pub fn col_edge(&self, dir: DirH) -> ColId {
        match dir {
            DirH::W => self.col_w(),
            DirH::E => self.col_e(),
        }
    }

    pub fn row_s(&self) -> RowId {
        RowId::from_idx(0)
    }

    pub fn row_n(&self) -> RowId {
        RowId::from_idx(self.rows - 1)
    }

    pub fn row_edge(&self, dir: DirV) -> RowId {
        match dir {
            DirV::S => self.row_s(),
            DirV::N => self.row_n(),
        }
    }

    pub fn is_row_io(&self, row: RowId) -> bool {
        row == self.row_s() || row == self.row_n()
    }

    pub fn columns(&self) -> EntityRange<ColId> {
        EntityRange::new(0, self.columns)
    }

    pub fn rows(&self) -> EntityRange<RowId> {
        EntityRange::new(0, self.rows)
    }

    pub fn corner(&self, side: DirHV) -> TileCoord {
        Self::DIE
            .cell(self.col_edge(side.h), self.row_edge(side.v))
            .tile(tslots::MAIN)
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
        let slot = defs::bslots::IO[iob.to_idx()];
        Self::DIE.cell(col, row).bel(slot)
    }

    pub fn get_io_crd(&self, bel: BelCoord) -> EdgeIoCoord {
        let iob = TileIobId::from_idx(defs::bslots::IO.index_of(bel.slot).unwrap());
        if bel.col == self.col_w() {
            EdgeIoCoord::W(bel.row, iob)
        } else if bel.col == self.col_e() {
            EdgeIoCoord::E(bel.row, iob)
        } else if bel.row == self.row_s() {
            EdgeIoCoord::S(bel.col, iob)
        } else if bel.row == self.row_n() {
            EdgeIoCoord::N(bel.col, iob)
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

    pub fn bel_pci(&self, side: DirH) -> BelCoord {
        Self::DIE
            .cell(self.col_edge(side), self.row_clk())
            .bel(defs::bslots::PCILOGIC)
    }
}

impl Chip {
    pub fn dump(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(o, "\tkind {};", self.kind)?;
        writeln!(o, "\tcolumns {};", self.columns)?;
        writeln!(o, "\trows {};", self.rows)?;
        writeln!(
            o,
            "\tcols_bram {};",
            self.cols_bram.iter().map(|x| x.to_string()).join(", ")
        )?;
        for &(col_hclk, col_start, col_end) in &self.cols_clkv {
            writeln!(o, "\tcol_clkv {col_hclk} = {col_start}..{col_end};")?;
        }
        for (k, v) in &self.cfg_io {
            writeln!(o, "\tcfg_io {k} = {v};")?;
        }
        Ok(())
    }
}
