use prjcombine_int::grid::{ColId, DieId, ExpandedGrid, RowId};
use prjcombine_virtex_bitstream::{BitTile, BitstreamGeom};
use unnamed_entity::{EntityId, EntityVec};

use crate::grid::{Grid, IoCoord, TileIobId};

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub egrid: ExpandedGrid<'a>,
    pub bs_geom: BitstreamGeom,
    pub spine_frame: usize,
    pub spine_framebit: usize,
    pub col_frame: EntityVec<ColId, usize>,
    pub row_framebit: EntityVec<RowId, usize>,
}

impl<'a> ExpandedDevice<'a> {
    pub fn get_bonded_ios(&'a self) -> Vec<IoCoord> {
        let mut res = vec![];
        let die = self.egrid.die(DieId::from_idx(0));
        for col in die.cols() {
            if col == self.grid.col_lio() || col == self.grid.col_rio() {
                continue;
            }
            for iob in [3, 2, 1, 0] {
                res.push(IoCoord {
                    col,
                    row: self.grid.row_tio(),
                    iob: TileIobId::from_idx(iob),
                });
            }
        }
        for row in die.rows().rev() {
            if row == self.grid.row_bio() || row == self.grid.row_tio() {
                continue;
            }
            for iob in [3, 2, 1, 0] {
                res.push(IoCoord {
                    col: self.grid.col_rio(),
                    row,
                    iob: TileIobId::from_idx(iob),
                });
            }
        }
        for col in die.cols().rev() {
            if col == self.grid.col_lio() || col == self.grid.col_rio() {
                continue;
            }
            for iob in [0, 1, 2, 3] {
                res.push(IoCoord {
                    col,
                    row: self.grid.row_bio(),
                    iob: TileIobId::from_idx(iob),
                });
            }
        }
        for row in die.rows() {
            if row == self.grid.row_bio() || row == self.grid.row_tio() {
                continue;
            }
            for iob in [0, 1, 2, 3] {
                res.push(IoCoord {
                    col: self.grid.col_lio(),
                    row,
                    iob: TileIobId::from_idx(iob),
                });
            }
        }
        res
    }

    pub fn btile_main(&self, col: ColId, row: RowId) -> BitTile {
        let width = if col == self.grid.col_lio() {
            7
        } else if col == self.grid.col_rio() {
            8
        } else {
            12
        };
        let height = if row == self.grid.row_bio() || row == self.grid.row_tio() {
            28
        } else {
            34
        };

        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            width,
            self.row_framebit[row],
            height,
            false,
        )
    }

    pub fn btile_spine(&self, row: RowId) -> BitTile {
        let width = 1;
        let height = if row == self.grid.row_bio() || row == self.grid.row_tio() {
            28
        } else {
            34
        };

        BitTile::Main(
            DieId::from_idx(0),
            self.spine_frame,
            width,
            self.row_framebit[row],
            height,
            false,
        )
    }

    pub fn btile_hclk(&self, col: ColId) -> BitTile {
        let width = if col == self.grid.col_lio() {
            7
        } else if col == self.grid.col_rio() {
            8
        } else {
            12
        };
        let height = 4;

        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            width,
            self.spine_framebit,
            height,
            false,
        )
    }
}
