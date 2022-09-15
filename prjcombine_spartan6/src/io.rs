use prjcombine_entity::EntityId;
use prjcombine_int::db::BelId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::{ColumnIoKind, DisabledPart, Grid, GtPin, Gts, IoCoord};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Io {
    pub bank: u32,
    pub coord: IoCoord,
    pub name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Gt {
    pub gx: u32,
    pub gy: u32,
    pub top: bool,
    pub bank: u32,
}

impl Gt {
    pub fn get_pads(&self) -> Vec<(String, String, GtPin)> {
        let mut res = Vec::new();
        for b in 0..2 {
            res.push((
                format!("OPAD_X{}Y{}", self.gx, self.gy * 4 + 1 - b),
                format!("MGTTXP{}_{}", b, self.bank),
                GtPin::TxP(b as u8),
            ));
            res.push((
                format!("OPAD_X{}Y{}", self.gx, self.gy * 4 + 3 - b),
                format!("MGTTXN{}_{}", b, self.bank),
                GtPin::TxN(b as u8),
            ));
            res.push((
                format!("IPAD_X{}Y{}", self.gx, self.gy * 8 + b),
                format!("MGTRXN{}_{}", b, self.bank),
                GtPin::RxN(b as u8),
            ));
            res.push((
                format!("IPAD_X{}Y{}", self.gx, self.gy * 8 + 2 + b),
                format!("MGTRXP{}_{}", b, self.bank),
                GtPin::RxP(b as u8),
            ));
        }
        for b in 0..2 {
            res.push((
                format!("IPAD_X{}Y{}", self.gx, self.gy * 8 + 4 + 2 * b),
                format!("MGTREFCLK{}N_{}", b, self.bank),
                GtPin::ClkN(b as u8),
            ));
            res.push((
                format!("IPAD_X{}Y{}", self.gx, self.gy * 8 + 5 + 2 * b),
                format!("MGTREFCLK{}P_{}", b, self.bank),
                GtPin::ClkP(b as u8),
            ));
        }
        res
    }
}

impl Grid {
    pub fn get_io(&self) -> Vec<Io> {
        let mut res = Vec::new();
        let mut ctr = 1;
        // top
        for (col, &cd) in self.columns.iter() {
            let row_o = self.rows.last_id().unwrap();
            let row_i = row_o - 1;
            if matches!(cd.tio, ColumnIoKind::Outer | ColumnIoKind::Both) {
                for bel in [0, 1] {
                    res.push(Io {
                        bank: 0,
                        coord: IoCoord {
                            col,
                            row: row_o,
                            bel: BelId::from_idx(bel),
                        },
                        name: format!("PAD{ctr}"),
                    });
                    ctr += 1;
                }
            }
            if matches!(cd.tio, ColumnIoKind::Inner | ColumnIoKind::Both) {
                for bel in [0, 1] {
                    res.push(Io {
                        bank: 0,
                        coord: IoCoord {
                            col,
                            row: row_i,
                            bel: BelId::from_idx(bel),
                        },
                        name: format!("PAD{ctr}"),
                    });
                    ctr += 1;
                }
            }
        }
        // right
        for (row, rd) in self.rows.iter().rev() {
            if !rd.rio {
                continue;
            }
            let col = self.columns.last_id().unwrap();
            let bank = if let Some((_, sr)) = self.rows_bank_split {
                if row >= sr {
                    5
                } else {
                    1
                }
            } else {
                1
            };
            for bel in [0, 1] {
                res.push(Io {
                    bank,
                    coord: IoCoord {
                        col,
                        row,
                        bel: BelId::from_idx(bel),
                    },
                    name: format!("PAD{ctr}"),
                });
                ctr += 1;
            }
        }
        // bot
        for (col, &cd) in self.columns.iter().rev() {
            let row_o = self.rows.first_id().unwrap();
            let row_i = row_o + 1;
            if matches!(cd.bio, ColumnIoKind::Outer | ColumnIoKind::Both) {
                for bel in [0, 1] {
                    res.push(Io {
                        bank: 2,
                        coord: IoCoord {
                            col,
                            row: row_o,
                            bel: BelId::from_idx(bel),
                        },
                        name: format!("PAD{ctr}"),
                    });
                    ctr += 1;
                }
            }
            if matches!(cd.bio, ColumnIoKind::Inner | ColumnIoKind::Both) {
                for bel in [0, 1] {
                    res.push(Io {
                        bank: 2,
                        coord: IoCoord {
                            col,
                            row: row_i,
                            bel: BelId::from_idx(bel),
                        },
                        name: format!("PAD{ctr}"),
                    });
                    ctr += 1;
                }
            }
        }
        // left
        for (row, rd) in self.rows.iter() {
            if !rd.lio {
                continue;
            }
            let col = self.columns.first_id().unwrap();
            let bank = if let Some((sl, _)) = self.rows_bank_split {
                if row >= sl {
                    4
                } else {
                    3
                }
            } else {
                3
            };
            for bel in [0, 1] {
                res.push(Io {
                    bank,
                    coord: IoCoord {
                        col,
                        row,
                        bel: BelId::from_idx(bel),
                    },
                    name: format!("PAD{ctr}"),
                });
                ctr += 1;
            }
        }
        res
    }

    pub fn get_gt(&self, disabled: &BTreeSet<DisabledPart>) -> Vec<Gt> {
        let mut res = Vec::new();
        if !disabled.contains(&DisabledPart::Gtp) {
            match self.gts {
                Gts::Single(_) => {
                    res.push(Gt {
                        gx: 0,
                        gy: 0,
                        top: true,
                        bank: 101,
                    });
                }
                Gts::Double(_, _) => {
                    res.push(Gt {
                        gx: 0,
                        gy: 0,
                        top: true,
                        bank: 101,
                    });
                    res.push(Gt {
                        gx: 1,
                        gy: 0,
                        top: true,
                        bank: 123,
                    });
                }
                Gts::Quad(_, _) => {
                    res.push(Gt {
                        gx: 0,
                        gy: 1,
                        top: true,
                        bank: 101,
                    });
                    res.push(Gt {
                        gx: 1,
                        gy: 1,
                        top: true,
                        bank: 123,
                    });
                    res.push(Gt {
                        gx: 0,
                        gy: 0,
                        top: false,
                        bank: 245,
                    });
                    res.push(Gt {
                        gx: 1,
                        gy: 0,
                        top: false,
                        bank: 267,
                    });
                }
                Gts::None => (),
            }
        }
        res
    }
}
