use std::collections::BTreeSet;

use prjcombine_interconnect::grid::{ColId, DieId, EdgeIoCoord, ExpandedGrid, RowId, TileCoord};
use prjcombine_xilinx_bitstream::{BitTile, BitstreamGeom};
use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};

use crate::chip::{Chip, DisabledPart};

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

    pub fn tile_bits(&self, tcrd: TileCoord) -> Vec<BitTile> {
        let tile = &self[tcrd];
        let kind = self.db.tile_classes.key(tile.class).as_str();
        if matches!(kind, "LBRAM" | "RBRAM" | "MBRAM") {
            vec![
                self.btile_main(tcrd.col, tcrd.row),
                self.btile_main(tcrd.col, tcrd.row + 1),
                self.btile_main(tcrd.col, tcrd.row + 2),
                self.btile_main(tcrd.col, tcrd.row + 3),
                self.btile_bram(tcrd.col, tcrd.row),
            ]
        } else if kind.starts_with("CLKB") || kind.starts_with("CLKT") {
            if tcrd.row == self.chip.row_s() {
                vec![self.btile_spine(tcrd.row), self.btile_spine(tcrd.row + 1)]
            } else {
                vec![self.btile_spine(tcrd.row), self.btile_spine(tcrd.row - 1)]
            }
        } else if matches!(kind, "CLKV.CLKV" | "CLKV.GCLKV") {
            vec![self.btile_clkv(tcrd.col, tcrd.row)]
        } else if kind.starts_with("DLL") || matches!(kind, "BRAM_BOT" | "BRAM_TOP") {
            vec![self.btile_main(tcrd.col, tcrd.row)]
        } else {
            Vec::from_iter(
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
