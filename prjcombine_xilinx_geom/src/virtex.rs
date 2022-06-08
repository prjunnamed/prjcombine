use std::collections::{BTreeSet, BTreeMap};
use serde::{Serialize, Deserialize};
use crate::{CfgPin, BelCoord};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GridKind {
    Virtex,
    VirtexE,
    VirtexEM,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub kind: GridKind,
    pub columns: u32,
    pub cols_bram: Vec<u32>,
    pub cols_clkv: Vec<(u32, u32)>,
    pub rows: u32,
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
    pub fn get_io(&self) -> Vec<Io> {
        let mut res = Vec::new();
        let mut ctr = 1;
        // top
        for c in 1..(self.columns - 1) {
            for bel in [2, 1] {
                res.push(Io {
                    coord: BelCoord {
                        col: c,
                        row: self.rows - 1,
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
                        col: self.columns - 1,
                        row: r,
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
                        col: c,
                        row: 0,
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
                        col: 0,
                        row: r,
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
}
