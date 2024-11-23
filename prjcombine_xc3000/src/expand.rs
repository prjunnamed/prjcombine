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
        let s = if self.is_small { "S" } else { "" };

        for col in grid.cols() {
            for row in grid.rows() {
                let mut subkind = (row.to_idx() + 2 * (self.columns - 1 - col.to_idx())) % 3;
                if subkind == 1 && col == self.col_rio() && row == self.row_tio() - 1 {
                    // fuck me with the rustiest fork you can find
                    subkind = 3;
                }
                if col == self.col_lio() {
                    if row == self.row_bio() {
                        grid.add_xnode(
                            (col, row),
                            &format!("CLB.BL{s}.{subkind}"),
                            &[(col, row), (col + 1, row), (col, row + 1)],
                        );
                    } else if row == self.row_tio() {
                        grid.add_xnode(
                            (col, row),
                            &format!("CLB.TL{s}.{subkind}"),
                            &[(col, row), (col + 1, row), (col, row - 1)],
                        );
                    } else {
                        grid.add_xnode(
                            (col, row),
                            &format!("CLB.L.{subkind}"),
                            &[(col, row), (col + 1, row), (col, row - 1), (col, row + 1)],
                        );
                    }
                } else if col == self.col_rio() {
                    if row == self.row_bio() {
                        grid.add_xnode(
                            (col, row),
                            &format!("CLB.BR{s}.{subkind}"),
                            &[(col, row), (col, row + 1)],
                        );
                    } else if row == self.row_tio() {
                        grid.add_xnode(
                            (col, row),
                            &format!("CLB.TR.{subkind}"),
                            &[(col, row), (col, row - 1)],
                        );
                    } else {
                        grid.add_xnode(
                            (col, row),
                            &format!("CLB.R.{subkind}"),
                            &[(col, row), (col, row - 1), (col, row + 1)],
                        );
                    }
                } else {
                    if row == self.row_bio() {
                        grid.add_xnode(
                            (col, row),
                            &format!("CLB.B.{subkind}"),
                            &[(col, row), (col + 1, row), (col, row + 1)],
                        );
                    } else if row == self.row_tio() {
                        grid.add_xnode(
                            (col, row),
                            &format!("CLB.T.{subkind}"),
                            &[(col, row), (col + 1, row), (col, row - 1)],
                        );
                    } else {
                        grid.add_xnode(
                            (col, row),
                            &format!("CLB.{subkind}"),
                            &[(col, row), (col + 1, row), (col, row - 1), (col, row + 1)],
                        );
                    }
                }
            }
        }
        {
            let col = self.col_mid();
            let row = self.row_bio();
            grid.fill_term_pair((col - 1, row), (col, row), "LLH.E", "LLH.W");
            grid.add_xnode((col, row), "LLH.B", &[(col - 1, row), (col, row)]);
            let row = self.row_tio();
            grid.fill_term_pair((col - 1, row), (col, row), "LLH.E", "LLH.W");
            grid.add_xnode((col, row), "LLH.T", &[(col - 1, row), (col, row)]);
        }
        if self.is_small {
            let row = self.row_mid();
            let col = self.col_lio();
            grid.fill_term_pair((col, row - 1), (col, row), "LLV.S.N", "LLV.S.S");
            grid.add_xnode((col, row), "LLV.LS", &[(col, row - 1), (col, row)]);
            let col = self.col_rio();
            grid.fill_term_pair((col, row - 1), (col, row), "LLV.S.N", "LLV.S.S");
            grid.add_xnode((col, row), "LLV.RS", &[(col, row - 1), (col, row)]);
        } else {
            let row = self.row_mid();
            for col in grid.cols() {
                let kind = if col == self.col_lio() {
                    "LLV.L"
                } else if col == self.col_rio() {
                    "LLV.R"
                } else {
                    "LLV"
                };
                grid.fill_term_pair((col, row - 1), (col, row), "LLV.N", "LLV.S");
                grid.add_xnode((col, row), kind, &[(col, row - 1), (col, row)]);
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
