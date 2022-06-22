use std::collections::{BTreeSet, BTreeMap};
use serde::{Serialize, Deserialize};
use crate::{CfgPin, BelCoord, GtPin, DisabledPart, ColId, RowId};
use prjcombine_entity::EntityVec;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub columns: EntityVec<ColId, ColumnKind>,
    pub cols_bio: EntityVec<ColId, ColumnIoKind>,
    pub cols_tio: EntityVec<ColId, ColumnIoKind>,
    pub col_clk: ColId,
    pub cols_clk_fold: Option<(ColId, ColId)>,
    pub cols_reg_buf: (ColId, ColId),
    pub rows: EntityVec<RowId, Row>,
    pub rows_midbuf: (RowId, RowId),
    pub rows_hclkbuf: (RowId, RowId),
    pub rows_bufio_split: (RowId, RowId),
    pub rows_bank_split: Option<(RowId, RowId)>,
    pub gts: Gts,
    pub mcbs: Vec<Mcb>,
    pub vref: BTreeSet<BelCoord>,
    pub cfg_io: BTreeMap<CfgPin, BelCoord>,
    pub has_encrypt: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKind {
    Io,
    CleXL,
    CleXM,
    CleClk,
    Bram,
    Dsp,
    DspPlus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnIoKind {
    None,
    Both,
    Inner,
    Outer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub lio: bool,
    pub rio: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Gts {
    None,
    Single(ColId),
    Double(ColId, ColId),
    Quad(ColId, ColId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McbIo {
    pub row: RowId,
    pub bel: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Mcb {
    pub row_mcb: RowId,
    pub row_mui: [RowId; 8],
    pub iop_dq: [RowId; 8],
    pub iop_dqs: [RowId; 2],
    pub io_dm: [McbIo; 2],
    pub iop_clk: RowId,
    pub io_addr: [McbIo; 15],
    pub io_ba: [McbIo; 3],
    pub io_ras: McbIo,
    pub io_cas: McbIo,
    pub io_we: McbIo,
    pub io_odt: McbIo,
    pub io_cke: McbIo,
    pub io_reset: McbIo,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Io {
    pub bank: u32,
    pub coord: BelCoord,
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
    pub fn get_pads(&self) -> Vec<(String, String, GtPin, u32)> {
        let mut res = Vec::new();
        for b in 0..2 {
            res.push((format!("OPAD_X{}Y{}", self.gx, self.gy * 4 + 1 - b), format!("MGTTXP{}_{}", b, self.bank), GtPin::TxP, b));
            res.push((format!("OPAD_X{}Y{}", self.gx, self.gy * 4 + 3 - b), format!("MGTTXN{}_{}", b, self.bank), GtPin::TxN, b));
            res.push((format!("IPAD_X{}Y{}", self.gx, self.gy * 8 + b), format!("MGTRXN{}_{}", b, self.bank), GtPin::RxN, b));
            res.push((format!("IPAD_X{}Y{}", self.gx, self.gy * 8 + 2 + b), format!("MGTRXP{}_{}", b, self.bank), GtPin::RxP, b));
        }
        for b in 0..2 {
            res.push((format!("IPAD_X{}Y{}", self.gx, self.gy * 8 + 4 + 2 * b), format!("MGTREFCLK{}N_{}", b, self.bank), GtPin::ClkN, b));
            res.push((format!("IPAD_X{}Y{}", self.gx, self.gy * 8 + 5 + 2 * b), format!("MGTREFCLK{}P_{}", b, self.bank), GtPin::ClkP, b));
        }
        res
    }
}

impl Grid {
    pub fn get_io(&self) -> Vec<Io> {
        let mut res = Vec::new();
        let mut ctr = 1;
        // top
        for (col, &kind) in self.cols_tio.iter() {
            let row_o = self.rows.last_id().unwrap();
            let row_i = row_o - 1;
            if matches!(kind, ColumnIoKind::Outer | ColumnIoKind::Both) {
                for bel in [0, 1] {
                    res.push(Io {
                        bank: 0,
                        coord: BelCoord {
                            col,
                            row: row_o,
                            bel,
                        },
                        name: format!("PAD{ctr}"),
                    });
                    ctr += 1;
                }
            }
            if matches!(kind, ColumnIoKind::Inner | ColumnIoKind::Both) {
                for bel in [0, 1] {
                    res.push(Io {
                        bank: 0,
                        coord: BelCoord {
                            col,
                            row: row_i,
                            bel,
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
                    coord: BelCoord {
                        col,
                        row,
                        bel,
                    },
                    name: format!("PAD{ctr}"),
                });
                ctr += 1;
            }
        }
        // bot
        for (col, &kind) in self.cols_bio.iter().rev() {
            let row_o = self.rows.first_id().unwrap();
            let row_i = row_o + 1;
            if matches!(kind, ColumnIoKind::Outer | ColumnIoKind::Both) {
                for bel in [0, 1] {
                    res.push(Io {
                        bank: 2,
                        coord: BelCoord {
                            col,
                            row: row_o,
                            bel,
                        },
                        name: format!("PAD{ctr}"),
                    });
                    ctr += 1;
                }
            }
            if matches!(kind, ColumnIoKind::Inner | ColumnIoKind::Both) {
                for bel in [0, 1] {
                    res.push(Io {
                        bank: 2,
                        coord: BelCoord {
                            col,
                            row: row_i,
                            bel,
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
                    coord: BelCoord {
                        col,
                        row,
                        bel,
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
        if !disabled.contains(&DisabledPart::Spartan6Gtp) {
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
