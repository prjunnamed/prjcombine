use enum_map::EnumMap;
use prjcombine_int::{
    db::Dir,
    grid::{ColId, DieId, ExpandedGrid, Rect, RowId},
};
use prjcombine_virtex_bitstream::{BitTile, BitstreamGeom};
use std::collections::{BTreeSet, HashMap};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::grid::{DisabledPart, Grid, RegId};

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub disabled: BTreeSet<DisabledPart>,
    pub egrid: ExpandedGrid<'a>,
    pub site_holes: Vec<Rect>,
    pub bs_geom: BitstreamGeom,
    pub col_frame: EntityVec<RegId, EntityVec<ColId, usize>>,
    pub col_width: EntityVec<ColId, usize>,
    pub spine_frame: EntityVec<RegId, usize>,
    pub bram_frame: EntityVec<RegId, EntityPartVec<ColId, usize>>,
    pub iob_frame: HashMap<(ColId, RowId), usize>,
    pub reg_frame: EnumMap<Dir, usize>,
}

impl ExpandedDevice<'_> {
    pub fn in_site_hole(&self, col: ColId, row: RowId) -> bool {
        for hole in &self.site_holes {
            if hole.contains(col, row) {
                return true;
            }
        }
        false
    }

    pub fn btile_main(&self, col: ColId, row: RowId) -> BitTile {
        let reg = self.grid.row_to_reg(row);
        let rd = row - self.grid.row_reg_bot(reg);
        let bit = 64 * (rd as usize) + if rd < 8 { 0 } else { 16 };
        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[reg][col],
            self.col_width[col],
            bit,
            64,
            false,
        )
    }

    pub fn btile_spine(&self, row: RowId) -> BitTile {
        let reg = self.grid.row_to_reg(row);
        let rd = row - self.grid.row_reg_bot(reg);
        let bit = 64 * (rd as usize) + if rd < 8 { 0 } else { 16 };
        BitTile::Main(DieId::from_idx(0), self.spine_frame[reg], 4, bit, 64, false)
    }

    pub fn btile_hclk(&self, col: ColId, row: RowId) -> BitTile {
        let reg = self.grid.row_to_reg(row);
        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[reg][col],
            self.col_width[col],
            64 * 8,
            16,
            false,
        )
    }

    pub fn btile_bram(&self, col: ColId, row: RowId) -> BitTile {
        let reg = self.grid.row_to_reg(row);
        let rd: usize = (row - self.grid.row_reg_bot(reg)).try_into().unwrap();
        BitTile::Bram(DieId::from_idx(0), self.bram_frame[reg][col] + rd / 4)
    }

    pub fn btile_reg(&self, dir: Dir) -> BitTile {
        BitTile::Iob(DieId::from_idx(0), self.reg_frame[dir], 384)
    }

    pub fn btile_iob(&self, col: ColId, row: RowId) -> BitTile {
        BitTile::Iob(DieId::from_idx(0), self.iob_frame[&(col, row)], 128)
    }
}
