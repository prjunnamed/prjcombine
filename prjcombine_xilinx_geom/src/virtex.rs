use std::collections::{BTreeSet, BTreeMap};
use serde::{Serialize, Deserialize};
use crate::{CfgPin, BelCoord, ColId, RowId, eint, int};
use ndarray::Array2;
use prjcombine_entity::EntityId;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GridKind {
    Virtex,
    VirtexE,
    VirtexEM,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub kind: GridKind,
    pub columns: usize,
    pub cols_bram: Vec<ColId>,
    pub cols_clkv: Vec<(ColId, ColId)>,
    pub rows: usize,
    pub vref: BTreeSet<BelCoord>,
    pub cfg_io: BTreeMap<CfgPin, BelCoord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Io {
    pub bank: u32,
    pub coord: BelCoord,
    pub name: String,
}

impl Grid {
    pub fn row_mid(&self) -> RowId {
        RowId::from_idx(self.rows as usize / 2)
    }

    pub fn get_io(&self) -> Vec<Io> {
        let mut res = Vec::new();
        let mut ctr = 1;
        // top
        for c in 1..(self.columns - 1) {
            for bel in [2, 1] {
                res.push(Io {
                    coord: BelCoord {
                        col: ColId::from_idx(c as usize),
                        row: RowId::from_idx(self.rows as usize - 1),
                        bel,
                    },
                    bank: if c < self.columns / 2 { 0 } else { 1 },
                    name: format!("PAD{ctr}"),
                });
                ctr += 1;
            }
        }
        // right
        for r in (1..(self.rows - 1)).rev() {
            for bel in [1, 2, 3] {
                res.push(Io {
                    coord: BelCoord {
                        col: ColId::from_idx(self.columns as usize - 1),
                        row: RowId::from_idx(r as usize),
                        bel,
                    },
                    bank: if r < self.rows / 2 { 3 } else { 2 },
                    name: format!("PAD{ctr}"),
                });
                ctr += 1;
            }
        }
        // bottom
        for c in (1..(self.columns - 1)).rev() {
            for bel in [1, 2] {
                res.push(Io {
                    coord: BelCoord {
                        col: ColId::from_idx(c as usize),
                        row: RowId::from_idx(0),
                        bel,
                    },
                    bank: if c < self.columns / 2 { 5 } else { 4 },
                    name: format!("PAD{ctr}"),
                });
                ctr += 1;
            }
        }
        // left
        for r in 1..(self.rows - 1) {
            for bel in [3, 2, 1] {
                res.push(Io {
                    coord: BelCoord {
                        col: ColId::from_idx(0),
                        row: RowId::from_idx(r as usize),
                        bel,
                    },
                    bank: if r < self.rows / 2 { 6 } else { 7 },
                    name: format!("PAD{ctr}"),
                });
                ctr += 1;
            }
        }
        res
    }

    pub fn expand_grid<'a>(&self, db: &'a int::IntDb) -> eint::ExpandedGrid<'a> {
        let mut grid = eint::ExpandedGrid {
            db,
            tie_kind: None,
            tie_pin_pullup: None,
            tie_pin_gnd: None,
            tie_pin_vcc: None,
            tiles: Array2::default([self.rows, self.columns]),
        };
        let col_l = grid.cols().next().unwrap();
        let col_r = grid.cols().next_back().unwrap();
        let row_b = grid.rows().next().unwrap();
        let row_t = grid.rows().next_back().unwrap();
        for col in grid.cols() {
            if col == col_l {
                for row in grid.rows() {
                    if row == row_b {
                        grid.fill_tile((col, row), "CNR.BL", "NODE.CNR.BL", "BL".to_string());
                    } else if row == row_t {
                        grid.fill_tile((col, row), "CNR.TL", "NODE.CNR.TL", "TL".to_string());
                    } else {
                        let r = row_t.to_idx() - row.to_idx();
                        grid.fill_tile((col, row), "IO.L", "NODE.IO.L", format!("LR{r}"));
                    }
                }
            } else if col == col_r {
                for row in grid.rows() {
                    if row == row_b {
                        grid.fill_tile((col, row), "CNR.BR", "NODE.CNR.BR", "BR".to_string());
                    } else if row == row_t {
                        grid.fill_tile((col, row), "CNR.TR", "NODE.CNR.TR", "TR".to_string());
                    } else {
                        let r = row_t.to_idx() - row.to_idx();
                        grid.fill_tile((col, row), "IO.R", "NODE.IO.R", format!("RR{r}"));
                    }
                }
            } else {
                let c = col.to_idx();
                for row in grid.rows() {
                    if row == row_b {
                        grid.fill_tile((col, row), "IO.B", "NODE.IO.B", format!("BC{c}"));
                    } else if row == row_t {
                        grid.fill_tile((col, row), "IO.T", "NODE.IO.T", format!("TC{c}"));
                    } else {
                        let r = row_t.to_idx() - row.to_idx();
                        grid.fill_tile((col, row), "CLB", "NODE.CLB", format!("R{r}C{c}"));
                    }
                }
            }
        }
        grid.fill_main_passes();
        grid
    }
}
