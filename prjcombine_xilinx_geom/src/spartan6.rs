use std::collections::{BTreeSet, BTreeMap};
use serde::{Serialize, Deserialize};
use crate::{CfgPin, BelCoord, GtPin, DisabledPart};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub columns: Vec<ColumnKind>,
    pub cols_bio: Vec<ColumnIoKind>,
    pub cols_tio: Vec<ColumnIoKind>,
    pub col_clk: u32,
    pub cols_clk_fold: Option<(u32, u32)>,
    pub cols_reg_buf: (u32, u32),
    pub rows: u32,
    pub rows_lio: Vec<bool>,
    pub rows_rio: Vec<bool>,
    pub rows_midbuf: (u32, u32),
    pub rows_hclkbuf: (u32, u32),
    pub rows_bufio_split: (u32, u32),
    pub rows_bank_split: Option<(u32, u32)>,
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
pub enum Gts {
    None,
    Single(u32),
    Double(u32, u32),
    Quad(u32, u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McbIo {
    pub row: u32,
    pub bel: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Mcb {
    pub row_mcb: u32,
    pub row_mui: [u32; 8],
    pub iop_dq: [u32; 8],
    pub iop_dqs: [u32; 2],
    pub io_dm: [McbIo; 2],
    pub iop_clk: u32,
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
        for (cs, kind) in self.cols_tio.iter().copied().enumerate() {
            let col = cs as u32;
            let row_o = self.rows * 16 - 1;
            let row_i = self.rows * 16 - 2;
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
        for (rs, present) in self.rows_rio.iter().copied().enumerate().rev() {
            let col = self.columns.len() as u32 - 1;
            let row = rs as u32;
            let bank = if let Some((_, sr)) = self.rows_bank_split {
                if row >= sr {
                    5
                } else {
                    1
                }
            } else {
                1
            };
            if present {
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
        }
        // bot
        for (cs, kind) in self.cols_bio.iter().copied().enumerate().rev() {
            let col = cs as u32;
            if matches!(kind, ColumnIoKind::Outer | ColumnIoKind::Both) {
                for bel in [0, 1] {
                    res.push(Io {
                        bank: 2,
                        coord: BelCoord {
                            col,
                            row: 0,
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
                            row: 1,
                            bel,
                        },
                        name: format!("PAD{ctr}"),
                    });
                    ctr += 1;
                }
            }
        }
        // right
        for (rs, present) in self.rows_lio.iter().copied().enumerate() {
            let row = rs as u32;
            let bank = if let Some((sl, _)) = self.rows_bank_split {
                if row >= sl {
                    4
                } else {
                    3
                }
            } else {
                3
            };
            if present {
                for bel in [0, 1] {
                    res.push(Io {
                        bank,
                        coord: BelCoord {
                            col: 0,
                            row,
                            bel,
                        },
                        name: format!("PAD{ctr}"),
                    });
                    ctr += 1;
                }
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
