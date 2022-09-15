use prjcombine_entity::EntityId;
use prjcombine_int::grid::{ColId, RowId};
use serde::{Deserialize, Serialize};

use crate::{ColumnKind, Grid, GtPin, SharedCfgPin, SysMonPin};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Io {
    pub col: ColId,
    pub row: RowId,
    pub bel: u32,
    pub ioc: u32,
    pub bank: u32,
    pub bbel: u32,
}

impl Io {
    pub fn iob_name(&self) -> String {
        let x = self.ioc;
        let y = self.row.to_idx() as u32 * 2 + self.bel;
        format!("IOB_X{x}Y{y}")
    }
    pub fn is_cc(&self) -> bool {
        matches!(self.row.to_idx() % 20, 8..=11)
    }
    pub fn is_gc(&self) -> bool {
        matches!(self.bank, 3 | 4)
    }
    pub fn is_vref(&self) -> bool {
        self.row.to_idx() % 10 == 5 && self.bel == 0
    }
    pub fn is_vr(&self) -> bool {
        match self.bank {
            1 | 2 => false,
            3 => self.row.to_idx() % 10 == 7,
            4 => self.row.to_idx() % 10 == 2,
            _ => self.row.to_idx() % 20 == 7,
        }
    }
    pub fn sm_pair(&self) -> Option<u32> {
        match (self.bank, self.row.to_idx() % 20) {
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
    pub fn get_cfg(&self) -> Option<SharedCfgPin> {
        match (self.bank, self.row.to_idx() % 20, self.bel) {
            (4, 16, 0) => Some(SharedCfgPin::Data(8)),
            (4, 16, 1) => Some(SharedCfgPin::Data(9)),
            (4, 17, 0) => Some(SharedCfgPin::Data(10)),
            (4, 17, 1) => Some(SharedCfgPin::Data(11)),
            (4, 18, 0) => Some(SharedCfgPin::Data(12)),
            (4, 18, 1) => Some(SharedCfgPin::Data(13)),
            (4, 19, 0) => Some(SharedCfgPin::Data(14)),
            (4, 19, 1) => Some(SharedCfgPin::Data(15)),
            (2, 0, 0) => Some(SharedCfgPin::Data(0)),
            (2, 0, 1) => Some(SharedCfgPin::Data(1)),
            (2, 1, 0) => Some(SharedCfgPin::Data(2)),
            (2, 1, 1) => Some(SharedCfgPin::Data(3)),
            (2, 2, 0) => Some(SharedCfgPin::Data(4)),
            (2, 2, 1) => Some(SharedCfgPin::Data(5)),
            (2, 3, 0) => Some(SharedCfgPin::Data(6)),
            (2, 3, 1) => Some(SharedCfgPin::Data(7)),
            (2, 4, 0) => Some(SharedCfgPin::CsoB),
            (2, 4, 1) => Some(SharedCfgPin::FweB),
            (2, 5, 0) => Some(SharedCfgPin::FoeB),
            (2, 5, 1) => Some(SharedCfgPin::FcsB),
            (2, 6, 0) => Some(SharedCfgPin::Addr(20)),
            (2, 6, 1) => Some(SharedCfgPin::Addr(21)),
            (2, 7, 0) => Some(SharedCfgPin::Addr(22)),
            (2, 7, 1) => Some(SharedCfgPin::Addr(23)),
            (2, 8, 0) => Some(SharedCfgPin::Addr(24)),
            (2, 8, 1) => Some(SharedCfgPin::Addr(25)),
            (2, 9, 0) => Some(SharedCfgPin::Rs(0)),
            (2, 9, 1) => Some(SharedCfgPin::Rs(1)),
            (1, 10, 0) => Some(SharedCfgPin::Data(16)),
            (1, 10, 1) => Some(SharedCfgPin::Data(17)),
            (1, 11, 0) => Some(SharedCfgPin::Data(18)),
            (1, 11, 1) => Some(SharedCfgPin::Data(19)),
            (1, 12, 0) => Some(SharedCfgPin::Data(20)),
            (1, 12, 1) => Some(SharedCfgPin::Data(21)),
            (1, 13, 0) => Some(SharedCfgPin::Data(22)),
            (1, 13, 1) => Some(SharedCfgPin::Data(23)),
            (1, 14, 0) => Some(SharedCfgPin::Data(24)),
            (1, 14, 1) => Some(SharedCfgPin::Data(25)),
            (1, 15, 0) => Some(SharedCfgPin::Data(26)),
            (1, 15, 1) => Some(SharedCfgPin::Data(27)),
            (1, 16, 0) => Some(SharedCfgPin::Data(28)),
            (1, 16, 1) => Some(SharedCfgPin::Data(29)),
            (1, 17, 0) => Some(SharedCfgPin::Data(30)),
            (1, 17, 1) => Some(SharedCfgPin::Data(31)),
            (1, 18, 0) => Some(SharedCfgPin::Addr(16)),
            (1, 18, 1) => Some(SharedCfgPin::Addr(17)),
            (1, 19, 0) => Some(SharedCfgPin::Addr(18)),
            (1, 19, 1) => Some(SharedCfgPin::Addr(19)),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Gt {
    pub col: ColId,
    pub row: RowId,
    pub gtc: u32,
    pub bank: u32,
    pub is_gtx: bool,
}

impl Gt {
    pub fn get_pads(&self, grid: &Grid) -> Vec<(String, String, GtPin)> {
        let reg = self.row.to_idx() / 20;
        let ipy = if reg < grid.reg_cfg {
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
            (
                format!("IPAD_X{}Y{}", ipx, ipy),
                format!("MGTRXN0_{}", self.bank),
                GtPin::RxN(0),
            ),
            (
                format!("IPAD_X{}Y{}", ipx, ipy + 1),
                format!("MGTRXP0_{}", self.bank),
                GtPin::RxP(0),
            ),
            (
                format!("IPAD_X{}Y{}", ipx, ipy + 2),
                format!("MGTRXN1_{}", self.bank),
                GtPin::RxN(1),
            ),
            (
                format!("IPAD_X{}Y{}", ipx, ipy + 3),
                format!("MGTRXP1_{}", self.bank),
                GtPin::RxP(1),
            ),
            (
                format!("IPAD_X{}Y{}", ipx, ipy + 4),
                format!("MGTREFCLKN_{}", self.bank),
                GtPin::ClkN,
            ),
            (
                format!("IPAD_X{}Y{}", ipx, ipy + 5),
                format!("MGTREFCLKP_{}", self.bank),
                GtPin::ClkP,
            ),
            (
                format!("OPAD_X{}Y{}", opx, opy),
                format!("MGTTXN0_{}", self.bank),
                GtPin::TxN(0),
            ),
            (
                format!("OPAD_X{}Y{}", opx, opy + 1),
                format!("MGTTXP0_{}", self.bank),
                GtPin::TxP(0),
            ),
            (
                format!("OPAD_X{}Y{}", opx, opy + 2),
                format!("MGTTXN1_{}", self.bank),
                GtPin::TxN(1),
            ),
            (
                format!("OPAD_X{}Y{}", opx, opy + 3),
                format!("MGTTXP1_{}", self.bank),
                GtPin::TxP(1),
            ),
        ]
    }
}

impl Grid {
    pub fn get_io(&self) -> Vec<Io> {
        let mut res = Vec::new();
        // left column
        for i in 0..self.regs {
            let bank = if i < self.reg_cfg {
                13 + (self.reg_cfg - i - 1) * 4
            } else {
                11 + (i - self.reg_cfg) * 4
            };
            for j in 0..20 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.cols_io[0].unwrap(),
                        row: RowId::from_idx(i * 20 + j),
                        ioc: 0,
                        bel: k,
                        bank: bank as u32,
                        bbel: (19 - j as u32) * 2 + k,
                    });
                }
            }
        }
        // center column
        // bottom banks
        if self.reg_cfg > 3 {
            for i in 0..(self.reg_cfg - 3) {
                let bank = 6 + (self.reg_cfg - 4 - i) * 2;
                for j in 0..20 {
                    for k in 0..2 {
                        res.push(Io {
                            col: self.cols_io[1].unwrap(),
                            row: RowId::from_idx(i * 20 + j),
                            ioc: 1,
                            bel: k,
                            bank: bank as u32,
                            bbel: (19 - j as u32) * 2 + k,
                        });
                    }
                }
            }
        }
        // special banks 4, 2, 1, 3
        for (bank, base) in [
            (4, self.reg_cfg * 20 - 30),
            (2, self.reg_cfg * 20 - 20),
            (1, self.reg_cfg * 20 + 10),
            (3, self.reg_cfg * 20 + 20),
        ] {
            for j in 0..10 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.cols_io[1].unwrap(),
                        row: RowId::from_idx(base + j),
                        ioc: 1,
                        bel: k,
                        bank,
                        bbel: (9 - j as u32) * 2 + k,
                    });
                }
            }
        }
        // top banks
        if (self.regs - self.reg_cfg) > 3 {
            for i in (self.reg_cfg + 3)..self.regs {
                let bank = 5 + (i - self.reg_cfg - 3) * 2;
                for j in 0..20 {
                    for k in 0..2 {
                        res.push(Io {
                            col: self.cols_io[1].unwrap(),
                            row: RowId::from_idx(i * 20 + j),
                            ioc: 1,
                            bel: k,
                            bank: bank as u32,
                            bbel: (19 - j as u32) * 2 + k,
                        });
                    }
                }
            }
        }
        // right column
        if let Some(col) = self.cols_io[2] {
            for i in 0..self.regs {
                let bank = if i < self.reg_cfg {
                    14 + (self.reg_cfg - i - 1) * 4
                } else {
                    12 + (i - self.reg_cfg) * 4
                };
                for j in 0..20 {
                    for k in 0..2 {
                        res.push(Io {
                            col,
                            row: RowId::from_idx(i * 20 + j),
                            ioc: 2,
                            bel: k,
                            bank: bank as u32,
                            bbel: (19 - j as u32) * 2 + k,
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
            for i in 0..self.regs {
                let bank = if i < self.reg_cfg {
                    113 + (self.reg_cfg - i - 1) * 4
                } else {
                    111 + (i - self.reg_cfg) * 4
                };
                res.push(Gt {
                    col: self.columns.first_id().unwrap(),
                    row: RowId::from_idx(i * 20),
                    gtc: 0,
                    bank: bank as u32,
                    is_gtx: true,
                });
            }
        }
        if self.col_hard.is_some() {
            let is_gtx = *self.columns.last().unwrap() == ColumnKind::Gtx;
            for i in 0..self.regs {
                let bank = if i < self.reg_cfg {
                    114 + (self.reg_cfg - i - 1) * 4
                } else {
                    112 + (i - self.reg_cfg) * 4
                };
                res.push(Gt {
                    col: self.columns.last_id().unwrap(),
                    row: RowId::from_idx(i * 20),
                    gtc: 1,
                    bank: bank as u32,
                    is_gtx,
                });
            }
        }
        res
    }

    pub fn get_sysmon_pads(&self) -> Vec<(String, SysMonPin)> {
        let mut res = Vec::new();
        if self.has_left_gt() {
            let ipy = 6 * self.reg_cfg;
            res.push((format!("IPAD_X1Y{}", ipy), SysMonPin::VP));
            res.push((format!("IPAD_X1Y{}", ipy + 1), SysMonPin::VN));
        } else if self.col_hard.is_some() {
            let ipy = 6 * self.reg_cfg;
            res.push((format!("IPAD_X0Y{}", ipy), SysMonPin::VP));
            res.push((format!("IPAD_X0Y{}", ipy + 1), SysMonPin::VN));
        } else {
            res.push(("IPAD_X0Y0".to_string(), SysMonPin::VP));
            res.push(("IPAD_X0Y1".to_string(), SysMonPin::VN));
        }
        res
    }
}
