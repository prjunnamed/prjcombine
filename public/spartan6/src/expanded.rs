use prjcombine_interconnect::{
    db::RegionSlotId, dir::{Dir, DirMap}, grid::{ColId, DieId, ExpandedGrid, NodeLoc, Rect, RowId}
};
use prjcombine_xilinx_bitstream::{BitTile, BitstreamGeom};
use std::collections::{BTreeSet, HashMap};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::chip::{Chip, DisabledPart, RegId};

pub const REGION_HCLK: RegionSlotId = RegionSlotId::from_idx_const(0);
pub const REGION_LEAF: RegionSlotId = RegionSlotId::from_idx_const(1);

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
    pub iob_frame: HashMap<(ColId, RowId), usize>,
    pub reg_frame: DirMap<usize>,
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
        let reg = self.chip.row_to_reg(row);
        let rd = row - self.chip.row_reg_bot(reg);
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
        let reg = self.chip.row_to_reg(row);
        let rd = row - self.chip.row_reg_bot(reg);
        let bit = 64 * (rd as usize) + if rd < 8 { 0 } else { 16 };
        BitTile::Main(DieId::from_idx(0), self.spine_frame[reg], 4, bit, 64, false)
    }

    pub fn btile_hclk(&self, col: ColId, row: RowId) -> BitTile {
        let reg = self.chip.row_to_reg(row);
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
        let reg = self.chip.row_to_reg(row);
        let rd: usize = (row - self.chip.row_reg_bot(reg)).try_into().unwrap();
        BitTile::Bram(DieId::from_idx(0), self.bram_frame[reg][col] + rd / 4)
    }

    pub fn btile_reg(&self, dir: Dir) -> BitTile {
        BitTile::Iob(DieId::from_idx(0), self.reg_frame[dir], 384)
    }

    pub fn btile_iob(&self, col: ColId, row: RowId) -> BitTile {
        BitTile::Iob(DieId::from_idx(0), self.iob_frame[&(col, row)], 128)
    }

    pub fn node_bits(&self, nloc: NodeLoc) -> Vec<BitTile> {
        let (_, col, row, _) = nloc;
        let node = self.egrid.tile(nloc);
        let kind = self.egrid.db.tile_classes.key(node.class).as_str();
        if kind == "BRAM" {
            vec![
                self.btile_main(col, row),
                self.btile_main(col, row + 1),
                self.btile_main(col, row + 2),
                self.btile_main(col, row + 3),
                self.btile_bram(col, row),
            ]
        } else if kind == "HCLK" {
            vec![self.btile_hclk(col, row)]
        } else if kind == "REG_L" {
            vec![self.btile_reg(Dir::W)]
        } else if kind == "REG_R" {
            vec![self.btile_reg(Dir::E)]
        } else if kind == "REG_B" {
            vec![self.btile_reg(Dir::S)]
        } else if kind == "REG_T" {
            vec![self.btile_reg(Dir::N)]
        } else if kind == "HCLK_ROW" {
            vec![self.btile_spine(row - 1)]
        } else if kind.starts_with("PLL_BUFPLL") || kind.starts_with("DCM_BUFPLL") {
            vec![self.btile_spine(row - 7)]
        } else if kind == "IOB" {
            vec![self.btile_iob(col, row)]
        } else if matches!(kind, "CMT_DCM" | "CMT_PLL") {
            let mut res = vec![];
            for i in 0..16 {
                res.push(self.btile_main(col, row - 8 + i));
            }
            for i in 0..16 {
                res.push(self.btile_spine(row - 8 + i));
            }
            res
        } else {
            Vec::from_iter(
                node.cells
                    .values()
                    .map(|&(col, row)| self.btile_main(col, row)),
            )
        }
    }
}
