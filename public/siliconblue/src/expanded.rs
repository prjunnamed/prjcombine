use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::grid::{ColId, ExpandedGrid, RowId, TileCoord};
use prjcombine_types::bsdata::BitRectId;

use crate::{
    bitstream::{BitPos, BitRect},
    chip::Chip,
    defs,
};

pub struct ExpandedDevice<'a> {
    pub chip: &'a Chip,
    pub egrid: ExpandedGrid<'a>,
    pub col_bit: EntityVec<ColId, usize>,
    pub frame_width: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum BitOwner {
    Null,
    Main(ColId, RowId),
    Bram(ColId, RowId),
    Clock(usize),
    Pll(usize),
    Speed,
    CReg,
}

impl ExpandedDevice<'_> {
    pub fn btile_main(&self, col: ColId, row: RowId) -> BitRect {
        let mut bank = 0;
        if col >= self.chip.col_mid() {
            bank |= 2;
        }
        let frame;
        if row < self.chip.row_mid {
            frame = row.to_idx() * 16;
        } else {
            frame = (self.chip.rows - 1 - row.to_idx()) * 16;
            bank |= 1;
        }
        BitRect::Main(
            bank,
            frame,
            16,
            self.col_bit[col],
            self.chip.btile_width(col),
        )
    }

    pub fn btile_bram(&self, col: ColId, row: RowId) -> BitRect {
        let mut bank = 0;
        if col >= self.chip.col_mid() {
            bank |= 2;
        }
        let bit;
        if row < self.chip.row_mid {
            bit = (row.to_idx() - 1) / 2 * 16;
        } else {
            bit = (row.to_idx() - self.chip.row_mid.to_idx()) / 2 * 16;
            bank |= 1;
        }
        BitRect::Bram(bank, bit)
    }

    pub fn btile_pll(&self) -> [BitRect; 2] {
        [
            BitRect::Main(0, 0, 16, self.frame_width - 2, 2),
            BitRect::Main(2, 0, 16, self.frame_width - 2, 2),
        ]
    }

    pub fn btile_clock(&self) -> [BitRect; 2] {
        [
            BitRect::Main(
                0,
                self.chip.row_mid.to_idx() * 16 - 16,
                16,
                self.frame_width - 2,
                2,
            ),
            BitRect::Main(
                1,
                (self.chip.rows - self.chip.row_mid.to_idx()) * 16 - 16,
                16,
                self.frame_width - 2,
                2,
            ),
        ]
    }

    pub fn classify_bit(&self, bit: BitPos) -> Option<(BitRect, BitOwner)> {
        match bit {
            BitPos::Main(bank, frame, bit) => {
                let row = frame / 0x10;
                let row = if (bank & 1) == 0 {
                    self.chip.row_s() + row
                } else {
                    self.chip.row_n() - row
                };
                if let Some(col) = self.chip.columns().into_iter().find(|&col| {
                    bit >= self.col_bit[col]
                        && bit < self.col_bit[col] + self.chip.btile_width(col)
                        && if (bank & 2) == 0 {
                            col < self.chip.col_mid()
                        } else {
                            col >= self.chip.col_mid()
                        }
                }) {
                    Some((self.btile_main(col, row), BitOwner::Main(col, row)))
                } else {
                    if frame < 16 {
                        if (bank & 1) == 0 {
                            Some((self.btile_pll()[bank / 2], BitOwner::Pll(bank / 2)))
                        } else {
                            None
                        }
                    } else {
                        if bank < 2 {
                            Some((self.btile_clock()[bank], BitOwner::Clock(bank)))
                        } else {
                            None
                        }
                    }
                }
            }
            BitPos::Bram(bank, _, bit) => {
                let row = bit / 0x10;
                let row = if (bank & 1) == 0 {
                    RowId::from_idx(1 + 2 * row)
                } else {
                    self.chip.row_mid + 2 * row
                };
                let col = if (bank & 2) == 0 {
                    self.chip
                        .cols_bram
                        .iter()
                        .copied()
                        .find(|&col| col < self.chip.col_mid())
                        .unwrap()
                } else {
                    self.chip
                        .cols_bram
                        .iter()
                        .copied()
                        .find(|&col| col >= self.chip.col_mid())
                        .unwrap()
                };
                Some((self.btile_bram(col, row), BitOwner::Bram(col, row)))
            }
            BitPos::Speed(_) => Some((BitRect::Speed, BitOwner::Speed)),
            BitPos::CReg(_) => Some((BitRect::CReg, BitOwner::CReg)),
        }
    }

    pub fn tile_bits(&self, tcrd: TileCoord) -> EntityVec<BitRectId, BitRect> {
        let tile = &self[tcrd];
        let tcls = &self.db.tile_classes[tile.class];
        if tcls.bitrects.is_empty() {
            EntityVec::new()
        } else if tcls.slot == defs::tslots::GLOBALS {
            EntityVec::from_iter([BitRect::CReg, BitRect::Speed])
        } else if tcls.bels.contains_id(defs::bslots::BRAM) {
            EntityVec::from_iter([
                self.btile_main(tcrd.col, tcrd.row),
                self.btile_main(tcrd.col, tcrd.row + 1),
                self.btile_bram(tcrd.col, tcrd.row),
            ])
        } else if tcls.bels.contains_id(defs::bslots::GB_ROOT) {
            EntityVec::from_iter(self.btile_clock())
        } else if tcls.bels.contains_id(defs::bslots::PLL65) {
            EntityVec::from_iter(self.btile_pll())
        } else if tcls.bels.contains_id(defs::bslots::PLL40) {
            EntityVec::from_iter(
                tile.cells
                    .values()
                    .take(tile.cells.len() - 2)
                    .map(|&cell| self.btile_main(cell.col, cell.row)),
            )
        } else {
            EntityVec::from_iter(
                tile.cells
                    .values()
                    .map(|&cell| self.btile_main(cell.col, cell.row)),
            )
        }
    }
}

impl<'a> std::ops::Deref for ExpandedDevice<'a> {
    type Target = ExpandedGrid<'a>;

    fn deref(&self) -> &Self::Target {
        &self.egrid
    }
}
