use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};
use prjcombine_interconnect::{
    dir::{Dir, DirMap},
    grid::{CellCoord, ColId, DieId, ExpandedGrid, Rect, RowId, TileCoord},
};
use prjcombine_types::bsdata::BitRectId;
use prjcombine_xilinx_bitstream::{BitRect, BitstreamGeom};
use std::collections::{BTreeSet, HashMap};

use crate::chip::{Chip, DisabledPart, RegId};

pub struct ExpandedDevice<'a> {
    pub chip: &'a Chip,
    pub disabled: BTreeSet<DisabledPart>,
    pub egrid: ExpandedGrid<'a>,
    pub site_holes: Vec<Rect>,
    pub bs_geom: BitstreamGeom,
    pub col_frame: EntityVec<RegId, EntityVec<ColId, usize>>,
    pub col_width: EntityVec<ColId, usize>,
    pub spine_frame: EntityVec<RegId, usize>,
    pub bram_frame: EntityVec<RegId, EntityPartVec<ColId, usize>>,
    pub iob_frame: HashMap<CellCoord, usize>,
    pub reg_frame: DirMap<usize>,
}

impl ExpandedDevice<'_> {
    pub fn in_site_hole(&self, cell: CellCoord) -> bool {
        for hole in &self.site_holes {
            if hole.contains(cell) {
                return true;
            }
        }
        false
    }

    pub fn btile_main(&self, col: ColId, row: RowId) -> BitRect {
        let reg = self.chip.row_to_reg(row);
        let rd = row - self.chip.row_reg_bot(reg);
        let bit = 64 * (rd as usize) + if rd < 8 { 0 } else { 16 };
        BitRect::Main(
            DieId::from_idx(0),
            self.col_frame[reg][col],
            self.col_width[col],
            bit,
            64,
            false,
        )
    }

    pub fn btile_spine(&self, row: RowId) -> BitRect {
        let reg = self.chip.row_to_reg(row);
        let rd = row - self.chip.row_reg_bot(reg);
        let bit = 64 * (rd as usize) + if rd < 8 { 0 } else { 16 };
        BitRect::Main(DieId::from_idx(0), self.spine_frame[reg], 4, bit, 64, false)
    }

    pub fn btile_hclk(&self, col: ColId, row: RowId) -> BitRect {
        let reg = self.chip.row_to_reg(row);
        BitRect::Main(
            DieId::from_idx(0),
            self.col_frame[reg][col],
            self.col_width[col],
            64 * 8,
            16,
            false,
        )
    }

    pub fn btile_bram(&self, col: ColId, row: RowId) -> BitRect {
        let reg = self.chip.row_to_reg(row);
        let rd: usize = (row - self.chip.row_reg_bot(reg)).try_into().unwrap();
        BitRect::Bram(DieId::from_idx(0), self.bram_frame[reg][col] + rd / 4)
    }

    pub fn btile_reg(&self, dir: Dir) -> BitRect {
        BitRect::Iob(DieId::from_idx(0), self.reg_frame[dir], 384)
    }

    pub fn btile_iob(&self, cell: CellCoord) -> BitRect {
        BitRect::Iob(DieId::from_idx(0), self.iob_frame[&cell], 128)
    }

    pub fn tile_bits(&self, tcrd: TileCoord) -> EntityVec<BitRectId, BitRect> {
        let tile = &self[tcrd];
        let kind = self.db.tile_classes.key(tile.class).as_str();
        if kind == "BRAM" {
            EntityVec::from_iter([
                self.btile_main(tcrd.col, tcrd.row),
                self.btile_main(tcrd.col, tcrd.row + 1),
                self.btile_main(tcrd.col, tcrd.row + 2),
                self.btile_main(tcrd.col, tcrd.row + 3),
                self.btile_bram(tcrd.col, tcrd.row),
            ])
        } else if kind == "HCLK" {
            EntityVec::from_iter([self.btile_hclk(tcrd.col, tcrd.row)])
        } else if kind == "REG_L" {
            EntityVec::from_iter([self.btile_reg(Dir::W)])
        } else if kind == "REG_R" {
            EntityVec::from_iter([self.btile_reg(Dir::E)])
        } else if kind == "REG_B" {
            EntityVec::from_iter([self.btile_reg(Dir::S)])
        } else if kind == "REG_T" {
            EntityVec::from_iter([self.btile_reg(Dir::N)])
        } else if kind == "HCLK_ROW" {
            EntityVec::from_iter([self.btile_spine(tcrd.row - 1)])
        } else if kind.starts_with("PLL_BUFPLL") || kind.starts_with("DCM_BUFPLL") {
            EntityVec::from_iter([self.btile_spine(tcrd.row - 7)])
        } else if kind == "IOB" {
            EntityVec::from_iter([self.btile_iob(tcrd.cell)])
        } else if matches!(kind, "CMT_DCM" | "CMT_PLL") {
            let mut res = EntityVec::new();
            for i in 0..16 {
                res.push(self.btile_main(tcrd.col, tcrd.row - 8 + i));
            }
            for i in 0..16 {
                res.push(self.btile_spine(tcrd.row - 8 + i));
            }
            res
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
