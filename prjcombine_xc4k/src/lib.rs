use prjcombine_entity::EntityId;
use prjcombine_int::db::BelId;
use prjcombine_int::grid::{ColId, RowId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    pub cfg_io: BTreeMap<SharedCfgPin, IoCoord>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    Tck,
    Tdi,
    Tms,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct IoCoord {
    pub col: ColId,
    pub row: RowId,
    pub bel: BelId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CfgPin {
    Tdo,
    Cclk,
    Done,
    ProgB,
    PwrdwnB,
    M0,
    M1,
    M2,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BondPin {
    Io(IoCoord),
    Nc,
    Gnd,
    VccInt,
    VccO,
    Cfg(CfgPin),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
}

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
                        bel: BelId::from_idx(bel),
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
                        bel: BelId::from_idx(bel),
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
                        bel: BelId::from_idx(bel),
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
                        bel: BelId::from_idx(bel),
                    },
                    name: format!("PAD{ctr}"),
                });
                ctr += 1;
            }
        }
        res
    }
}
