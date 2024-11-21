use prjcombine_int::{
    db::IntDb,
    grid::{ColId, ExpandedGrid, RowId},
};
use unnamed_entity::EntityId;

use crate::{expanded::ExpandedDevice, grid::Grid};

impl Grid {
    pub fn expand_grid<'a>(&'a self, db: &'a IntDb) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        let (_, mut grid) = egrid.add_die(self.columns, self.rows);

        for col in grid.cols() {
            if col == self.col_lio() {
                for row in grid.rows() {
                    if row == self.row_bio() {
                        grid.add_xnode((col, row), "CLB.BL", &[(col, row), (col + 1, row)]);
                    } else if row == self.row_tio() {
                        grid.add_xnode(
                            (col, row),
                            "CLB.TL",
                            &[(col, row), (col, row - 1), (col + 1, row)],
                        );
                    } else if row == self.row_mid() {
                        grid.add_xnode((col, row), "CLB.ML", &[(col, row), (col, row - 1)]);
                    } else {
                        grid.add_xnode((col, row), "CLB.L", &[(col, row), (col, row - 1)]);
                    }
                }
            } else if col == self.col_rio() {
                for row in grid.rows() {
                    if row == self.row_bio() {
                        grid.add_xnode((col, row), "CLB.BR", &[(col, row)]);
                    } else if row == self.row_tio() {
                        grid.add_xnode((col, row), "CLB.TR", &[(col, row), (col, row - 1)]);
                    } else if row == self.row_mid() {
                        grid.add_xnode((col, row), "CLB.MR", &[(col, row), (col, row - 1)]);
                    } else {
                        grid.add_xnode((col, row), "CLB.R", &[(col, row), (col, row - 1)]);
                    }
                }
            } else {
                for row in grid.rows() {
                    if row == self.row_bio() {
                        grid.add_xnode((col, row), "CLB.B", &[(col, row), (col + 1, row)]);
                    } else if row == self.row_tio() {
                        grid.add_xnode((col, row), "CLB.T", &[(col, row), (col + 1, row)]);
                    } else {
                        grid.add_xnode((col, row), "CLB", &[(col, row)]);
                    }
                }
            }
        }
        for col in grid.cols() {
            for row in grid.rows() {
                grid[(col, row)].clkroot = (ColId::from_idx(0), RowId::from_idx(0));
            }
        }

        grid.fill_main_passes();

        egrid.finish();

        ExpandedDevice { grid: self, egrid }
    }
}
