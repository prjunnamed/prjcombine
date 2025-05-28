use prjcombine_interconnect::{
    db::RegionSlotId,
    grid::{ColId, ExpandedGrid, NodeLoc, RowId},
};
use unnamed_entity::{EntityId, EntityVec};

use crate::{
    bitstream::{BitPos, BitTile},
    chip::Chip,
};

pub const REGION_GLOBAL: RegionSlotId = RegionSlotId::from_idx_const(0);

pub struct ExpandedDevice<'a> {
    pub chip: &'a Chip,
    pub egrid: ExpandedGrid<'a>,
    pub col_bit: EntityVec<ColId, usize>,
    pub frame_width: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum BitOwner {
    Main(ColId, RowId),
    Bram(ColId, RowId),
    Clock(usize),
    Pll(usize),
    Speed,
    CReg,
}

impl ExpandedDevice<'_> {
    pub fn btile_main(&self, col: ColId, row: RowId) -> BitTile {
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
        BitTile::Main(
            bank,
            frame,
            16,
            self.col_bit[col],
            self.chip.btile_width(col),
        )
    }

    pub fn btile_bram(&self, col: ColId, row: RowId) -> BitTile {
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
        BitTile::Bram(bank, bit)
    }

    pub fn btile_pll(&self) -> [BitTile; 2] {
        [
            BitTile::Main(0, 0, 16, self.frame_width - 2, 2),
            BitTile::Main(2, 0, 16, self.frame_width - 2, 2),
        ]
    }

    pub fn btile_clock(&self) -> [BitTile; 2] {
        [
            BitTile::Main(
                0,
                self.chip.row_mid.to_idx() * 16 - 16,
                16,
                self.frame_width - 2,
                2,
            ),
            BitTile::Main(
                1,
                (self.chip.rows - self.chip.row_mid.to_idx()) * 16 - 16,
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
                    self.chip.row_s() + row
                } else {
                    self.chip.row_n() - row
                };
                if let Some(col) = self.chip.columns().find(|&col| {
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
                        if bank % 2 == 0 {
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
            BitPos::Speed(_) => Some((BitTile::Speed, BitOwner::Speed)),
            BitPos::CReg(_) => Some((BitTile::CReg, BitOwner::CReg)),
        }
    }

    pub fn node_bits(&self, nloc: NodeLoc) -> Vec<BitTile> {
        let (_, col, row, _) = nloc;
        let node = self.egrid.tile(nloc);
        let kind = self.egrid.db.tile_classes.key(node.class).as_str();
        if kind == "BRAM" {
            vec![
                self.btile_main(col, row),
                self.btile_main(col, row + 1),
                self.btile_bram(col, row),
            ]
        } else if kind == "GB_OUT" {
            self.btile_clock().to_vec()
        } else if kind == "PLL_S" && self.chip.kind.is_ice65() {
            self.btile_pll().to_vec()
        } else {
            Vec::from_iter(
                node.cells
                    .values()
                    .map(|&(col, row)| self.btile_main(col, row)),
            )
        }
    }
}
