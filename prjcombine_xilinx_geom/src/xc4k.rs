use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};
use crate::{CfgPin, BelCoord};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GridKind {
    // plain, D (no memory)
    Xc4000,
    Xc4000A,
    Xc4000H,
    // E, L, Spartan
    Xc4000E,
    // EX, XL
    Xc4000Ex,
    Xc4000Xla,
    Xc4000Xv,
    SpartanXl,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub kind: GridKind,
    pub columns: u32,
    pub rows: u32,
    pub cfg_io: BTreeMap<CfgPin, BelCoord>,
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
            for bel in [0, 1] {
                res.push(Io {
                    coord: BelCoord {
                        col: c,
                        row: self.rows - 1,
                        bel,
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
                    coord: BelCoord {
                        col: self.columns - 1,
                        row: r,
                        bel,
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
                    coord: BelCoord {
                        col: c,
                        row: 0,
                        bel,
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
                    coord: BelCoord {
                        col: 0,
                        row: r,
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
