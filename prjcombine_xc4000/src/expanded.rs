use prjcombine_int::grid::{ColId, DieId, ExpandedGrid, RowId};
use prjcombine_virtex_bitstream::{BitTile, BitstreamGeom};
use unnamed_entity::{EntityId, EntityVec};

use crate::grid::{Grid, IoCoord, TileIobId};

#[derive(Debug)]
pub struct Io {
    pub name: String,
    pub crd: IoCoord,
}

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub egrid: ExpandedGrid<'a>,
    pub bs_geom: BitstreamGeom,
    pub spine_frame: usize,
    pub quarter_frame: Option<(usize, usize)>,
    pub col_frame: EntityVec<ColId, usize>,
    pub spine_framebit: usize,
    pub quarter_framebit: Option<(usize, usize)>,
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
            for iob in [0, 1] {
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
            for iob in [0, 1] {
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
            for iob in [1, 0] {
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
            for iob in [1, 0] {
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
        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            self.grid.btile_width_main(col),
            self.row_framebit[row],
            self.grid.btile_height_main(row),
            false,
        )
    }

    pub fn btile_llv(&self, col: ColId, row: RowId) -> BitTile {
        let (bit, height) = if row == self.grid.row_mid() {
            (self.spine_framebit, self.grid.btile_height_clk())
        } else if row == self.grid.row_qb() {
            (
                self.quarter_framebit.unwrap().0,
                self.grid.btile_height_brk(),
            )
        } else if row == self.grid.row_qt() {
            (
                self.quarter_framebit.unwrap().1,
                self.grid.btile_height_brk(),
            )
        } else {
            unreachable!()
        };
        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            self.grid.btile_width_main(col),
            bit,
            height,
            false,
        )
    }

    pub fn btile_llh(&self, col: ColId, row: RowId) -> BitTile {
        let (frame, width) = if col == self.grid.col_mid() {
            (self.spine_frame, self.grid.btile_width_clk())
        } else if col == self.grid.col_ql() {
            (self.quarter_frame.unwrap().0, self.grid.btile_width_brk())
        } else if col == self.grid.col_qr() {
            (self.quarter_frame.unwrap().1, self.grid.btile_width_brk())
        } else {
            unreachable!()
        };
        BitTile::Main(
            DieId::from_idx(0),
            frame,
            width,
            self.row_framebit[row],
            self.grid.btile_height_main(row),
            false,
        )
    }
}
