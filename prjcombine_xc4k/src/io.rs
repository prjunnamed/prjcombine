use prjcombine_entity::EntityId;
use prjcombine_int::grid::{ColId, RowId};
use serde::{Deserialize, Serialize};

use crate::grid::{Grid, TileIobId, IoCoord};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Io {
    pub coord: IoCoord,
    pub name: String,
}

impl Grid {
    pub fn get_io(&self) -> Vec<Io> {
        let mut res = Vec::new();
        let mut ctr = 1;
        // top
        for c in 1..(self.columns - 1) {
            for bel in [0, 1] {
                res.push(Io {
                    coord: IoCoord {
                        col: ColId::from_idx(c as usize),
                        row: RowId::from_idx(self.rows as usize - 1),
                        iob: TileIobId::from_idx(bel),
                    },
                    name: format!("PAD{ctr}"),
                });
                ctr += 1;
            }
        }
        // right
        for r in (1..(self.rows - 1)).rev() {
            for bel in [0, 1] {
                res.push(Io {
                    coord: IoCoord {
                        col: ColId::from_idx(self.columns as usize - 1),
                        row: RowId::from_idx(r as usize),
                        iob: TileIobId::from_idx(bel),
                    },
                    name: format!("PAD{ctr}"),
                });
                ctr += 1;
            }
        }
        // bottom
        for c in (1..(self.columns - 1)).rev() {
            for bel in [1, 0] {
                res.push(Io {
                    coord: IoCoord {
                        col: ColId::from_idx(c as usize),
                        row: RowId::from_idx(0),
                        iob: TileIobId::from_idx(bel),
                    },
                    name: format!("PAD{ctr}"),
                });
                ctr += 1;
            }
        }
        // left
        for r in 1..(self.rows - 1) {
            for bel in [1, 0] {
                res.push(Io {
                    coord: IoCoord {
                        col: ColId::from_idx(0),
                        row: RowId::from_idx(r as usize),
                        iob: TileIobId::from_idx(bel),
                    },
                    name: format!("PAD{ctr}"),
                });
                ctr += 1;
            }
        }
        res
    }
}
