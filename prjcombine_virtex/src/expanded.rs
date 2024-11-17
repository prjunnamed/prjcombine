use std::collections::BTreeSet;

use prjcombine_int::grid::{ColId, DieId, ExpandedGrid, RowId};
use prjcombine_virtex_bitstream::{BitTile, BitstreamGeom};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::grid::{DisabledPart, Grid, IoCoord, TileIobId};

#[derive(Copy, Clone, Debug)]
pub struct Io {
    pub bank: u32,
    pub coord: IoCoord,
}

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub egrid: ExpandedGrid<'a>,
    pub bs_geom: BitstreamGeom,
    pub spine_frame: usize,
    pub col_frame: EntityVec<ColId, usize>,
    pub bram_frame: EntityPartVec<ColId, usize>,
    pub clkv_frame: EntityPartVec<ColId, usize>,
    pub disabled: BTreeSet<DisabledPart>,
}

impl<'a> ExpandedDevice<'a> {
    pub fn get_io_bank(&'a self, coord: IoCoord) -> u32 {
        if coord.row == self.grid.row_tio() {
            if coord.col < self.grid.col_clk() {
                0
            } else {
                1
            }
        } else if coord.col == self.grid.col_rio() {
            if coord.row < self.grid.row_mid() {
                3
            } else {
                2
            }
        } else if coord.row == self.grid.row_bio() {
            if coord.col < self.grid.col_clk() {
                5
            } else {
                4
            }
        } else if coord.col == self.grid.col_lio() {
            if coord.row < self.grid.row_mid() {
                6
            } else {
                7
            }
        } else {
            unreachable!()
        }
    }

    pub fn get_bonded_ios(&'a self) -> Vec<IoCoord> {
        let mut res = vec![];
        let die = self.egrid.die(DieId::from_idx(0));
        for col in die.cols() {
            let row = self.grid.row_tio();
            if self.grid.cols_bram.contains(&col) {
                continue;
            }
            if col == self.grid.col_lio() || col == self.grid.col_rio() {
                continue;
            }
            for iob in [2, 1] {
                res.push(IoCoord {
                    col,
                    row,
                    iob: TileIobId::from_idx(iob),
                });
            }
        }
        for row in die.rows().rev() {
            let col = self.grid.col_rio();
            if row == self.grid.row_bio() || row == self.grid.row_tio() {
                continue;
            }
            for iob in [1, 2, 3] {
                res.push(IoCoord {
                    col,
                    row,
                    iob: TileIobId::from_idx(iob),
                });
            }
        }
        for col in die.cols().rev() {
            let row = self.grid.row_bio();
            if self.grid.cols_bram.contains(&col) {
                continue;
            }
            if col == self.grid.col_lio() || col == self.grid.col_rio() {
                continue;
            }
            for iob in [1, 2] {
                res.push(IoCoord {
                    col,
                    row,
                    iob: TileIobId::from_idx(iob),
                });
            }
        }
        for row in die.rows() {
            let col = self.grid.col_lio();
            if row == self.grid.row_bio() || row == self.grid.row_tio() {
                continue;
            }
            for iob in [3, 2, 1] {
                res.push(IoCoord {
                    col,
                    row,
                    iob: TileIobId::from_idx(iob),
                });
            }
        }
        res
    }

    pub fn btile_main(&self, col: ColId, row: RowId) -> BitTile {
        let width = if col == self.grid.col_lio() || col == self.grid.col_rio() {
            54
        } else if self.grid.cols_bram.contains(&col) {
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
}
