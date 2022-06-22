use std::collections::{BTreeSet, BTreeMap};
use serde::{Serialize, Deserialize};
use crate::{CfgPin, BelCoord, ColId, RowId};
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
    pub columns: u32,
    pub cols_bram: Vec<ColId>,
    pub cols_clkv: Vec<(ColId, ColId)>,
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
}
