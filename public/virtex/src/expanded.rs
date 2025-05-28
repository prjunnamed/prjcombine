use std::collections::BTreeSet;

use prjcombine_interconnect::{db::RegionSlotId, grid::{ColId, DieId, EdgeIoCoord, ExpandedGrid, NodeLoc, RowId}};
use prjcombine_xilinx_bitstream::{BitTile, BitstreamGeom};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::chip::{Chip, DisabledPart};

pub const REGION_GLOBAL: RegionSlotId = RegionSlotId::from_idx_const(0);
pub const REGION_LEAF: RegionSlotId = RegionSlotId::from_idx_const(1);

#[derive(Copy, Clone, Debug)]
pub struct Io {
    pub bank: u32,
    pub coord: EdgeIoCoord,
}

pub struct ExpandedDevice<'a> {
    pub chip: &'a Chip,
    pub egrid: ExpandedGrid<'a>,
    pub bs_geom: BitstreamGeom,
    pub spine_frame: usize,
    pub col_frame: EntityVec<ColId, usize>,
    pub bram_frame: EntityPartVec<ColId, usize>,
    pub clkv_frame: EntityPartVec<ColId, usize>,
    pub disabled: BTreeSet<DisabledPart>,
}

impl ExpandedDevice<'_> {
    pub fn btile_main(&self, col: ColId, row: RowId) -> BitTile {
        let width = if col == self.chip.col_w() || col == self.chip.col_e() {
            54
        } else if self.chip.cols_bram.contains(&col) {
            27
        } else {
            48
        };
        let height = 18;

        let bit = height * row.to_idx();
        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            width,
            bit,
            height,
            false,
        )
    }

    pub fn btile_spine(&self, row: RowId) -> BitTile {
        let width = 8;
        let height = 18;

        let bit = height * row.to_idx();
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
        let height = 18;

        let bit = height * row.to_idx();
        BitTile::Main(
            DieId::from_idx(0),
            self.clkv_frame[col],
            1,
            bit,
            height,
            false,
        )
    }

    pub fn btile_bram(&self, col: ColId, row: RowId) -> BitTile {
        let width = 64;
        let height = 18;

        let bit = height * row.to_idx();
        BitTile::Main(
            DieId::from_idx(0),
            self.bram_frame[col],
            width,
            bit,
            height * 4,
            false,
        )
    }

    pub fn node_bits(&self, nloc: NodeLoc) -> Vec<BitTile> {
        let (_, col, row, _) = nloc;
        let node = self.egrid.tile(nloc);
        let kind = self.egrid.db.tile_classes.key(node.class).as_str();
        if matches!(kind, "LBRAM" | "RBRAM" | "MBRAM") {
            vec![
                self.btile_main(col, row),
                self.btile_main(col, row + 1),
                self.btile_main(col, row + 2),
                self.btile_main(col, row + 3),
                self.btile_bram(col, row),
            ]
        } else if kind.starts_with("CLKB") || kind.starts_with("CLKT") {
            if row == self.chip.row_s() {
                vec![self.btile_spine(row), self.btile_spine(row + 1)]
            } else {
                vec![self.btile_spine(row), self.btile_spine(row - 1)]
            }
        } else if matches!(kind, "CLKV.CLKV" | "CLKV.GCLKV") {
            vec![self.btile_clkv(col, row)]
        } else if kind.starts_with("DLL") || matches!(kind, "BRAM_BOT" | "BRAM_TOP") {
            vec![self.btile_main(col, row)]
        } else {
            Vec::from_iter(
                node.cells
                    .values()
                    .map(|&(col, row)| self.btile_main(col, row)),
            )
        }
    }
}
