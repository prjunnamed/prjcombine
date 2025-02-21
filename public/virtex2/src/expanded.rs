use prjcombine_interconnect::grid::{ColId, DieId, ExpandedGrid, Rect, RowId};
use prjcombine_xilinx_bitstream::{BitTile, BitstreamGeom};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::chip::{Chip, ChipKind};

pub struct ExpandedDevice<'a> {
    pub chip: &'a Chip,
    pub egrid: ExpandedGrid<'a>,
    pub bs_geom: BitstreamGeom,
    pub holes: Vec<Rect>,
    pub clkv_frame: usize,
    pub spine_frame: usize,
    pub lterm_frame: usize,
    pub rterm_frame: usize,
    pub col_frame: EntityVec<ColId, usize>,
    pub bram_frame: EntityPartVec<ColId, usize>,
}

impl ExpandedDevice<'_> {
    pub fn is_in_hole(&self, col: ColId, row: RowId) -> bool {
        for hole in &self.holes {
            if hole.contains(col, row) {
                return true;
            }
        }
        false
    }

    pub fn btile_main(&self, col: ColId, row: RowId) -> BitTile {
        let (width, height) = if self.chip.kind.is_virtex2() {
            (22, 80)
        } else {
            (19, 64)
        };
        let bit = 16 + height * row.to_idx();
        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            width,
            bit,
            height,
            false,
        )
    }

    pub fn btile_bram(&self, col: ColId, row: RowId) -> BitTile {
        let (width, height, height_single) = if self.chip.kind.is_virtex2() {
            (64, 80 * 4, 80)
        } else {
            (19 * 4, 64 * 4, 64)
        };
        let bit = 16 + height_single * row.to_idx();
        BitTile::Main(
            DieId::from_idx(0),
            self.bram_frame[col],
            width,
            bit,
            height,
            false,
        )
    }

    pub fn btile_lrterm(&self, col: ColId, row: RowId) -> BitTile {
        let (width, height) = if self.chip.kind.is_virtex2() {
            (4, 80)
        } else {
            (2, 64)
        };
        let bit = 16 + height * row.to_idx();
        let frame = if col == self.chip.col_left() {
            self.lterm_frame
        } else if col == self.chip.col_right() {
            self.rterm_frame
        } else {
            unreachable!()
        };
        BitTile::Main(DieId::from_idx(0), frame, width, bit, height, false)
    }

    pub fn btile_btterm(&self, col: ColId, row: RowId) -> BitTile {
        let (width, height) = if self.chip.kind.is_virtex2() {
            (22, 80)
        } else {
            (19, 64)
        };
        let bit = if row == self.chip.row_bot() {
            if self.chip.kind.is_virtex2() {
                4
            } else if !self.chip.kind.is_spartan3a() {
                7
            } else {
                0
            }
        } else if row == self.chip.row_top() {
            16 + height * self.chip.rows.len()
        } else {
            unreachable!()
        };
        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            width,
            bit,
            if self.chip.kind.is_virtex2() {
                12
            } else if !self.chip.kind.is_spartan3a() {
                5
            } else {
                6
            },
            false,
        )
    }

    pub fn btile_spine(&self, row: RowId) -> BitTile {
        let (width, height) = if self.chip.kind.is_virtex2() {
            (4, 80)
        } else if self.chip.has_ll || self.chip.kind.is_spartan3a() {
            (2, 64)
        } else {
            (1, 64)
        };
        let bit = 16 + height * row.to_idx();
        BitTile::Main(
            DieId::from_idx(0),
            self.spine_frame,
            width,
            bit,
            height,
            false,
        )
    }

    pub fn btile_clkv(&self, col: ColId, row: RowId) -> BitTile {
        assert!(!self.chip.kind.is_virtex2());
        let bit = 16 + 64 * row.to_idx();
        BitTile::Main(
            DieId::from_idx(0),
            self.clkv_frame + if col < self.chip.col_clk { 0 } else { 1 },
            1,
            bit,
            64,
            false,
        )
    }

    pub fn btile_btspine(&self, row: RowId) -> BitTile {
        let (width, height) = if self.chip.kind.is_virtex2() {
            (4, 80)
        } else if self.chip.has_ll || self.chip.kind.is_spartan3a() {
            (2, 64)
        } else {
            (1, 64)
        };
        let bit = if row == self.chip.row_bot() {
            0
        } else if row == self.chip.row_top() {
            16 + height * self.chip.rows.len()
        } else {
            unreachable!()
        };
        BitTile::Main(DieId::from_idx(0), self.spine_frame, width, bit, 16, false)
    }

    pub fn btile_llv_b(&self, col: ColId) -> BitTile {
        assert_eq!(self.chip.kind, ChipKind::Spartan3E);
        assert!(self.chip.has_ll);
        let bit = self.chip.rows_hclk.len() / 2;
        BitTile::Main(DieId::from_idx(0), self.col_frame[col], 19, bit, 1, false)
    }

    pub fn btile_llv_t(&self, col: ColId) -> BitTile {
        assert_eq!(self.chip.kind, ChipKind::Spartan3E);
        assert!(self.chip.has_ll);
        let bit = 16 + self.chip.rows.len() * 64 + 11 + self.chip.rows_hclk.len() / 2;
        BitTile::Main(DieId::from_idx(0), self.col_frame[col], 19, bit, 2, false)
    }

    pub fn btile_llv(&self, col: ColId) -> BitTile {
        assert!(self.chip.kind.is_spartan3a());
        assert!(self.chip.has_ll);
        let bit = 16 + self.chip.rows.len() * 64 + 8;
        BitTile::Main(DieId::from_idx(0), self.col_frame[col], 19, bit, 3, false)
    }

    pub fn btile_hclk(&self, col: ColId, row: RowId) -> BitTile {
        let (width, height) = if self.chip.kind.is_virtex2() {
            (22, 80)
        } else {
            (19, 64)
        };
        let hclk_idx = self
            .chip
            .rows_hclk
            .iter()
            .position(|&(hrow, _, _)| hrow == row)
            .unwrap();
        let bit = if row <= self.chip.row_mid() {
            if self.chip.kind.is_spartan3a() {
                11 + hclk_idx
            } else {
                hclk_idx
            }
        } else {
            let hclk_idx = self.chip.rows_hclk.len() - hclk_idx - 1;
            if self.chip.kind.is_spartan3a() || self.chip.has_ll {
                16 + height * self.chip.rows.len() + 11 + hclk_idx
            } else {
                16 + height * self.chip.rows.len() + 12 + hclk_idx
            }
        };
        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            width,
            bit,
            1,
            false,
        )
    }
}
