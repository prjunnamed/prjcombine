use prjcombine_int::grid::{DieId, ExpandedGrid};
use unnamed_entity::EntityId;

use crate::grid::{Grid, IoCoord, TileIobId};

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub egrid: ExpandedGrid<'a>,
}

impl ExpandedDevice<'_> {
    pub fn get_bonded_ios(&self) -> Vec<IoCoord> {
        let mut res = vec![];
        let die = self.egrid.die(DieId::from_idx(0));
        for col in die.cols() {
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
                res.push(IoCoord {
                    col: self.grid.col_rio(),
                    row,
                    iob: TileIobId::from_idx(2),
                });
            } else if row == self.grid.row_mid() {
                res.push(IoCoord {
                    col: self.grid.col_rio(),
                    row,
                    iob: TileIobId::from_idx(0),
                });
            } else {
                for iob in [0, 1] {
                    res.push(IoCoord {
                        col: self.grid.col_rio(),
                        row,
                        iob: TileIobId::from_idx(iob),
                    });
                }
            }
        }
        for col in die.cols().rev() {
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
                res.push(IoCoord {
                    col: self.grid.col_lio(),
                    row,
                    iob: TileIobId::from_idx(2),
                });
            } else if row == self.grid.row_mid() {
                res.push(IoCoord {
                    col: self.grid.col_lio(),
                    row,
                    iob: TileIobId::from_idx(0),
                });
            } else {
                for iob in [1, 0] {
                    res.push(IoCoord {
                        col: self.grid.col_lio(),
                        row,
                        iob: TileIobId::from_idx(iob),
                    });
                }
            }
        }
        res
    }
}
