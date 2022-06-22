use serde::{Serialize, Deserialize};
use crate::{BelCoord, ColId, RowId};
use prjcombine_entity::EntityId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub columns: u32,
    pub rows: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Io {
    pub coord: BelCoord,
    pub name: String,
}

impl Grid {
    pub fn get_io(&self) -> Vec<Io> {
        let mut res = Vec::new();
        let mut ctr = 1;
        // top
        for c in 1..(self.columns - 1) {
            for bel in [3, 2, 1, 0] {
                res.push(Io {
                    coord: BelCoord {
                        col: ColId::from_idx(c as usize),
                        row: RowId::from_idx(self.rows as usize - 1),
                        bel,
                    },
                    name: format!("PAD{ctr}"),
                });
                ctr += 1;
            }
        }
        // right
        for r in (1..(self.rows - 1)).rev() {
            for bel in [3, 2, 1, 0] {
                res.push(Io {
                    coord: BelCoord {
                        col: ColId::from_idx(self.columns as usize - 1),
                        row: RowId::from_idx(r as usize),
                        bel,
                    },
                    name: format!("PAD{ctr}"),
                });
                ctr += 1;
            }
        }
        // bottom
        for c in (1..(self.columns - 1)).rev() {
            for bel in [0, 1, 2, 3] {
                res.push(Io {
                    coord: BelCoord {
                        col: ColId::from_idx(c as usize),
                        row: RowId::from_idx(0),
                        bel,
                    },
                    name: format!("PAD{ctr}"),
                });
                ctr += 1;
            }
        }
        // left
        for r in 1..(self.rows - 1) {
            for bel in [0, 1, 2, 3] {
                res.push(Io {
                    coord: BelCoord {
                        col: ColId::from_idx(0),
                        row: RowId::from_idx(r as usize),
                        bel,
                    },
                    name: format!("PAD{ctr}"),
                });
                ctr += 1;
            }
        }
        res
    }
}

