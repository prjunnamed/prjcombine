use prjcombine_interconnect::grid::{ColId, ExpandedGrid, RowId};
use unnamed_entity::{EntityId, EntityVec};

use crate::{
    bitstream::{BitPos, BitTile},
    grid::Grid,
};

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub egrid: ExpandedGrid<'a>,
    pub col_bit: EntityVec<ColId, usize>,
    pub frame_width: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum BitOwner {
    Main(ColId, RowId),
    Bram(ColId, RowId),
    Clock(usize),
    Speed,
    CReg,
}

impl ExpandedDevice<'_> {
    pub fn btile_main(&self, col: ColId, row: RowId) -> BitTile {
        let mut bank = 0;
        if col >= self.grid.col_mid() {
            bank |= 2;
        }
        let frame;
        if row < self.grid.row_mid {
            frame = row.to_idx() * 16;
        } else {
            frame = (self.grid.rows - 1 - row.to_idx()) * 16;
            bank |= 1;
        }
        BitTile::Main(
            bank,
            frame,
            16,
            self.col_bit[col],
            self.grid.btile_width(col),
        )
    }

    pub fn btile_bram(&self, col: ColId, row: RowId) -> BitTile {
        let mut bank = 0;
        if col >= self.grid.col_mid() {
            bank |= 2;
        }
        let bit;
        if row < self.grid.row_mid {
            bit = (row.to_idx() - 1) / 2 * 16;
        } else {
            bit = (row.to_idx() - self.grid.row_mid.to_idx()) / 2 * 16;
            bank |= 1;
        }
        BitTile::Bram(bank, bit)
    }

    pub fn btile_clock(&self) -> [BitTile; 2] {
        [
            BitTile::Main(
                0,
                self.grid.row_mid.to_idx() * 16 - 16,
                16,
                self.frame_width - 2,
                2,
            ),
            BitTile::Main(
                1,
                (self.grid.rows - self.grid.row_mid.to_idx()) * 16 - 16,
                16,
                self.frame_width - 2,
                2,
            ),
        ]
    }

    pub fn classify_bit(&self, bit: BitPos) -> Option<(BitTile, BitOwner)> {
        match bit {
            BitPos::Main(bank, frame, bit) => {
                let row = frame / 0x10;
                let row = if (bank & 1) == 0 {
                    self.grid.row_bio() + row
                } else {
                    self.grid.row_tio() - row
                };
                if let Some(col) = self.grid.columns().find(|&col| {
                    bit >= self.col_bit[col]
                        && bit < self.col_bit[col] + self.grid.btile_width(col)
                        && if (bank & 2) == 0 {
                            col < self.grid.col_mid()
                        } else {
                            col >= self.grid.col_mid()
                        }
                }) {
                    Some((self.btile_main(col, row), BitOwner::Main(col, row)))
                } else if bank < 2 {
                    Some((self.btile_clock()[bank], BitOwner::Clock(bank)))
                } else {
                    None
                }
            }
            BitPos::Bram(bank, _, bit) => {
                let row = bit / 0x10;
                let row = if (bank & 1) == 0 {
                    RowId::from_idx(1 + 2 * row)
                } else {
                    self.grid.row_mid + 2 * row
                };
                let col = if (bank & 2) == 0 {
                    self.grid
                        .cols_bram
                        .iter()
                        .copied()
                        .find(|&col| col < self.grid.col_mid())
                        .unwrap()
                } else {
                    self.grid
                        .cols_bram
                        .iter()
                        .copied()
                        .find(|&col| col >= self.grid.col_mid())
                        .unwrap()
                };
                Some((self.btile_bram(col, row), BitOwner::Bram(col, row)))
            }
            BitPos::Speed(_) => Some((BitTile::Speed, BitOwner::Speed)),
            BitPos::CReg(_) => Some((BitTile::CReg, BitOwner::CReg)),
        }
    }
}
