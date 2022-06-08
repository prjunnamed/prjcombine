use std::collections::BTreeSet;
use serde::{Serialize, Deserialize};
use super::{GtPin, SysMonPin, CfgPin};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub columns: Vec<ColumnKind>,
    pub cols_vbrk: BTreeSet<u32>,
    pub cols_mgt_buf: BTreeSet<u32>,
    pub col_hard: Option<HardColumn>,
    pub cols_io: [Option<u32>; 3],
    pub rows: u32,
    pub row_cfg: u32,
    pub holes_ppc: Vec<(u32, u32)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKind {
    ClbLL,
    ClbLM,
    Bram,
    Dsp,
    Io,
    Gtp,
    Gtx,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HardColumn {
    pub col: u32,
    pub rows_emac: Vec<u32>,
    pub rows_pcie: Vec<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Io {
    pub col: u32,
    pub row: u32,
    pub bel: u32,
    pub ioc: u32,
    pub bank: u32,
    pub bbel: u32,
}

impl Io {
    pub fn iob_name(&self) -> String {
        let x = self.ioc;
        let y = self.row * 2 + self.bel;
        format!("IOB_X{x}Y{y}")
    }
    pub fn is_cc(&self) -> bool {
        matches!(self.row % 20, 8..=11)
    }
    pub fn is_gc(&self) -> bool {
        matches!(self.bank, 3 | 4)
    }
    pub fn is_vref(&self) -> bool {
        self.row % 10 == 5 && self.bel == 0
    }
    pub fn is_vr(&self) -> bool {
        match self.bank {
            1 | 2 => false,
            3 => self.row % 10 == 7,
            4 => self.row % 10 == 2,
            _ => self.row % 20 == 7,
        }
    }
    pub fn sm_pair(&self) -> Option<u32> {
        match (self.bank, self.row % 20) {
            (13, 10) => Some(0),
            (13, 11) => Some(1),
            (13, 12) => Some(2),
            (13, 13) => Some(3),
            (13, 14) => Some(4),
            (13, 16) => Some(5),
            (13, 17) => Some(6),
            (13, 18) => Some(7),
            (13, 19) => Some(8),
            (11, 0) => Some(9),
            (11, 1) => Some(10),
            (11, 2) => Some(11),
            (11, 3) => Some(12),
            (11, 4) => Some(13),
            (11, 8) => Some(14),
            (11, 9) => Some(15),
            _ => None,
        }
    }
    pub fn get_cfg(&self) -> Option<CfgPin> {
        match (self.bank, self.row % 20, self.bel) {
            (4, 16, 0) => Some(CfgPin::Data(8)),
            (4, 16, 1) => Some(CfgPin::Data(9)),
            (4, 17, 0) => Some(CfgPin::Data(10)),
            (4, 17, 1) => Some(CfgPin::Data(11)),
            (4, 18, 0) => Some(CfgPin::Data(12)),
            (4, 18, 1) => Some(CfgPin::Data(13)),
            (4, 19, 0) => Some(CfgPin::Data(14)),
            (4, 19, 1) => Some(CfgPin::Data(15)),
            (2, 0, 0) => Some(CfgPin::Data(0)),
            (2, 0, 1) => Some(CfgPin::Data(1)),
            (2, 1, 0) => Some(CfgPin::Data(2)),
            (2, 1, 1) => Some(CfgPin::Data(3)),
            (2, 2, 0) => Some(CfgPin::Data(4)),
            (2, 2, 1) => Some(CfgPin::Data(5)),
            (2, 3, 0) => Some(CfgPin::Data(6)),
            (2, 3, 1) => Some(CfgPin::Data(7)),
            (2, 4, 0) => Some(CfgPin::CsoB),
            (2, 4, 1) => Some(CfgPin::FweB),
            (2, 5, 0) => Some(CfgPin::FoeB),
            (2, 5, 1) => Some(CfgPin::FcsB),
            (2, 6, 0) => Some(CfgPin::Addr(20)),
            (2, 6, 1) => Some(CfgPin::Addr(21)),
            (2, 7, 0) => Some(CfgPin::Addr(22)),
            (2, 7, 1) => Some(CfgPin::Addr(23)),
            (2, 8, 0) => Some(CfgPin::Addr(24)),
            (2, 8, 1) => Some(CfgPin::Addr(25)),
            (2, 9, 0) => Some(CfgPin::Rs(0)),
            (2, 9, 1) => Some(CfgPin::Rs(1)),
            (1, 10, 0) => Some(CfgPin::Data(16)),
            (1, 10, 1) => Some(CfgPin::Data(17)),
            (1, 11, 0) => Some(CfgPin::Data(18)),
            (1, 11, 1) => Some(CfgPin::Data(19)),
            (1, 12, 0) => Some(CfgPin::Data(20)),
            (1, 12, 1) => Some(CfgPin::Data(21)),
            (1, 13, 0) => Some(CfgPin::Data(22)),
            (1, 13, 1) => Some(CfgPin::Data(23)),
            (1, 14, 0) => Some(CfgPin::Data(24)),
            (1, 14, 1) => Some(CfgPin::Data(25)),
            (1, 15, 0) => Some(CfgPin::Data(26)),
            (1, 15, 1) => Some(CfgPin::Data(27)),
            (1, 16, 0) => Some(CfgPin::Data(28)),
            (1, 16, 1) => Some(CfgPin::Data(29)),
            (1, 17, 0) => Some(CfgPin::Data(30)),
            (1, 17, 1) => Some(CfgPin::Data(31)),
            (1, 18, 0) => Some(CfgPin::Addr(16)),
            (1, 18, 1) => Some(CfgPin::Addr(17)),
            (1, 19, 0) => Some(CfgPin::Addr(18)),
            (1, 19, 1) => Some(CfgPin::Addr(19)),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Gt {
    pub col: u32,
    pub row: u32,
    pub gtc: u32,
    pub bank: u32,
    pub is_gtx: bool,
}

impl Gt {
    pub fn get_pads(&self, grid: &Grid) -> Vec<(String, String, GtPin, u32)> {
        let reg = self.row / 20;
        let ipy = if reg < grid.row_cfg {
            reg * 6
        } else {
            6 + reg * 6
        };
        let opy = reg * 4;
        let (ipx, opx) = if grid.has_left_gt() {
            (self.gtc * 2, self.gtc)
        } else {
            (1, 0)
        };
        vec![
            (format!("IPAD_X{}Y{}", ipx, ipy), format!("MGTRXN0_{}", self.bank), GtPin::RxN, 0),
            (format!("IPAD_X{}Y{}", ipx, ipy+1), format!("MGTRXP0_{}", self.bank), GtPin::RxP, 0),
            (format!("IPAD_X{}Y{}", ipx, ipy+2), format!("MGTRXN1_{}", self.bank), GtPin::RxN, 1),
            (format!("IPAD_X{}Y{}", ipx, ipy+3), format!("MGTRXP1_{}", self.bank), GtPin::RxP, 1),
            (format!("IPAD_X{}Y{}", ipx, ipy+4), format!("MGTREFCLKN_{}", self.bank), GtPin::ClkN, 0),
            (format!("IPAD_X{}Y{}", ipx, ipy+5), format!("MGTREFCLKP_{}", self.bank), GtPin::ClkP, 0),
            (format!("OPAD_X{}Y{}", opx, opy), format!("MGTTXN0_{}", self.bank), GtPin::TxN, 0),
            (format!("OPAD_X{}Y{}", opx, opy+1), format!("MGTTXP0_{}", self.bank), GtPin::TxP, 0),
            (format!("OPAD_X{}Y{}", opx, opy+2), format!("MGTTXN1_{}", self.bank), GtPin::TxN, 1),
            (format!("OPAD_X{}Y{}", opx, opy+3), format!("MGTTXP1_{}", self.bank), GtPin::TxP, 1),
        ]
    }
}

impl Grid {
    pub fn get_io(&self) -> Vec<Io> {
        let mut res = Vec::new();
        // left column
        for i in 0..self.rows {
            let bank = if i < self.row_cfg {
                13 + (self.row_cfg - i - 1) * 4
            } else {
                11 + (i - self.row_cfg) * 4
            };
            for j in 0..20 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.cols_io[0].unwrap(),
                        row: i * 20 + j,
                        ioc: 0,
                        bel: k,
                        bank,
                        bbel: (19 - j) * 2 + k,
                    });
                }
            }
        }
        // center column
        // bottom banks
        if self.row_cfg > 3 {
            for i in 0..(self.row_cfg - 3) {
                let bank = 6 + (self.row_cfg - 4 - i) * 2;
                for j in 0..20 {
                    for k in 0..2 {
                        res.push(Io {
                            col: self.cols_io[1].unwrap(),
                            row: i * 20 + j,
                            ioc: 1,
                            bel: k,
                            bank,
                            bbel: (19 - j) * 2 + k,
                        });
                    }
                }
            }
        }
        // special banks 4, 2, 1, 3
        for (bank, base) in [
            (4, self.row_cfg * 20 - 30),
            (2, self.row_cfg * 20 - 20),
            (1, self.row_cfg * 20 + 10),
            (3, self.row_cfg * 20 + 20),
        ] {
            for j in 0..10 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.cols_io[1].unwrap(),
                        row: base + j,
                        ioc: 1,
                        bel: k,
                        bank,
                        bbel: (9 - j) * 2 + k,
                    });
                }
            }
        }
        // top banks
        if (self.rows - self.row_cfg) > 3 {
            for i in (self.row_cfg + 3)..self.rows {
                let bank = 5 + (i - self.row_cfg - 3) * 2;
                for j in 0..20 {
                    for k in 0..2 {
                        res.push(Io {
                            col: self.cols_io[1].unwrap(),
                            row: i * 20 + j,
                            ioc: 1,
                            bel: k,
                            bank,
                            bbel: (19 - j) * 2 + k,
                        });
                    }
                }
            }
        }
        // right column
        if let Some(col) = self.cols_io[2] {
            for i in 0..self.rows {
                let bank = if i < self.row_cfg {
                    14 + (self.row_cfg - i - 1) * 4
                } else {
                    12 + (i - self.row_cfg) * 4
                };
                for j in 0..20 {
                    for k in 0..2 {
                        res.push(Io {
                            col,
                            row: i * 20 + j,
                            ioc: 2,
                            bel: k,
                            bank,
                            bbel: (19 - j) * 2 + k,
                        });
                    }
                }
            }
        }
        res
    }

    pub fn get_gt(&self) -> Vec<Gt> {
        let mut res = Vec::new();
        if self.has_left_gt() {
            for i in 0..self.rows {
                let bank = if i < self.row_cfg {
                    113 + (self.row_cfg - i - 1) * 4
                } else {
                    111 + (i - self.row_cfg) * 4
                };
                res.push(Gt {
                    col: 0,
                    row: i * 20,
                    gtc: 0,
                    bank,
                    is_gtx: true,
                });
            }
        }
        if self.col_hard.is_some() {
            let is_gtx = *self.columns.last().unwrap() == ColumnKind::Gtx;
            for i in 0..self.rows {
                let bank = if i < self.row_cfg {
                    114 + (self.row_cfg - i - 1) * 4
                } else {
                    112 + (i - self.row_cfg) * 4
                };
                res.push(Gt {
                    col: self.columns.len() as u32 - 1,
                    row: i * 20,
                    gtc: 1,
                    bank,
                    is_gtx,
                });
            }
        }
        res
    }

    pub fn has_left_gt(&self) -> bool {
        self.columns[0] == ColumnKind::Gtx
    }

    pub fn get_sysmon_pads(&self) -> Vec<(String, SysMonPin)> {
        let mut res = Vec::new();
        if self.columns[0] == ColumnKind::Gtx {
            let ipy = 6 * self.row_cfg;
            res.push((format!("IPAD_X1Y{}", ipy), SysMonPin::VP));
            res.push((format!("IPAD_X1Y{}", ipy+1), SysMonPin::VN));
        } else if self.col_hard.is_some() {
            let ipy = 6 * self.row_cfg;
            res.push((format!("IPAD_X0Y{}", ipy), SysMonPin::VP));
            res.push((format!("IPAD_X0Y{}", ipy+1), SysMonPin::VN));
        } else {
            res.push((format!("IPAD_X0Y0"), SysMonPin::VP));
            res.push((format!("IPAD_X0Y1"), SysMonPin::VN));
        }
        res
    }
}
