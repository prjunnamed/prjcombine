use std::collections::BTreeSet;
use serde::{Serialize, Deserialize};
use super::{GtPin, SysMonPin, CfgPin};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub columns: Vec<ColumnKind>,
    pub cols_vbrk: BTreeSet<u32>,
    pub cols_io: [u32; 3],
    pub rows: u32,
    pub has_bot_sysmon: bool,
    pub has_top_sysmon: bool,
    pub rows_cfg_io: u32,
    pub ccm: u32,
    pub row_cfg: u32,
    pub holes_ppc: Vec<(u32, u32)>,
    pub has_bram_fx: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKind {
    Clb,
    Bram,
    Dsp,
    Io,
    Gt,
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
        matches!(self.row % 16, 7 | 8)
    }
    pub fn is_lc(&self) -> bool {
        matches!(self.row % 16, 7 | 8) || self.ioc == 1
    }
    pub fn is_gc(&self) -> bool {
        matches!(self.bank, 3 | 4) || (matches!(self.bank, 1 | 2) && matches!(self.bbel, 18..=33))
    }
    pub fn is_vref(&self) -> bool {
        self.row % 8 == 4 && self.bel == 0
    }
    pub fn is_vr(&self) -> bool {
        match self.bank {
            1 => self.bbel / 2 == 18,
            2 => self.bbel / 2 == 23,
            3 => self.bbel / 2 == 2,
            4 => self.bbel / 2 == 7,
            _ => self.row % 32 == 9,
        }
    }
    pub fn get_cfg(&self) -> Option<CfgPin> {
        if !matches!(self.bank, 1 | 2) {
            return None;
        }
        if self.bbel > 17 {
            return None;
        }
        if self.bank == 2 {
            Some(CfgPin::Data((self.row % 8 * 2 + self.bel) as u8))
        } else {
            Some(CfgPin::Data((self.row % 8 * 2 + self.bel + 16) as u8))
        }
    }
    pub fn sm_pair(&self, grid: &Grid) -> Option<(u32, u32)> {
        if grid.has_bot_sysmon {
            match (self.bank, self.row % 32) {
                (7, 0) => return Some((0, 1)),
                (7, 1) => return Some((0, 2)),
                (7, 2) => return Some((0, 3)),
                (7, 3) => return Some((0, 4)),
                (7, 5) => return Some((0, 5)),
                (7, 6) => return Some((0, 6)),
                (7, 7) => return Some((0, 7)),
                _ => (),
            }
        }
        if grid.has_top_sysmon {
            match (self.bank, self.row % 32) {
                (5, 24) => return Some((1, 1)),
                (5, 25) => return Some((1, 2)),
                (5, 26) => return Some((1, 3)),
                (5, 27) => return Some((1, 4)),
                (5, 29) => return Some((1, 5)),
                (5, 30) => return Some((1, 6)),
                (5, 31) => return Some((1, 7)),
                _ => (),
            }
        }
        None
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Gt {
    pub col: u32,
    pub row: u32,
    pub gtc: u32,
    pub bank: u32,
}

impl Gt {
    pub fn get_pads(&self, grid: &Grid) -> Vec<(String, String, GtPin, u32)> {
        let reg = self.row / 32;
        let (ipx, ipy);
        if grid.has_bot_sysmon {
            ipy = 2 + reg * 6;
            ipx = self.gtc * 2;
        } else {
            ipy = reg * 6;
            ipx = self.gtc;
        }
        let opy = reg * 4;
        let opx = self.gtc;
        vec![
            (format!("IPAD_X{}Y{}", ipx, ipy), format!("RXPPADB_{}", self.bank), GtPin::RxP, 0),
            (format!("IPAD_X{}Y{}", ipx, ipy+1), format!("RXNPADB_{}", self.bank), GtPin::RxN, 0),
            (format!("IPAD_X{}Y{}", ipx, ipy+2), format!("MGTCLK_N_{}", self.bank), GtPin::ClkN, 0),
            (format!("IPAD_X{}Y{}", ipx, ipy+3), format!("MGTCLK_P_{}", self.bank), GtPin::ClkP, 0),
            (format!("IPAD_X{}Y{}", ipx, ipy+4), format!("RXPPADA_{}", self.bank), GtPin::RxP, 1),
            (format!("IPAD_X{}Y{}", ipx, ipy+5), format!("RXNPADA_{}", self.bank), GtPin::RxN, 1),
            (format!("OPAD_X{}Y{}", opx, opy), format!("TXPPADB_{}", self.bank), GtPin::TxP, 0),
            (format!("OPAD_X{}Y{}", opx, opy+1), format!("TXNPADB_{}", self.bank), GtPin::TxN, 0),
            (format!("OPAD_X{}Y{}", opx, opy+2), format!("TXPPADA_{}", self.bank), GtPin::TxP, 1),
            (format!("OPAD_X{}Y{}", opx, opy+3), format!("TXNPADA_{}", self.bank), GtPin::TxN, 1),
        ]
    }
}

impl Grid {
    pub fn get_io(&self) -> Vec<Io> {
        let lbanks: &[u32] = match self.rows {
            4 => &[7, 5],
            6 => &[7, 9, 5],
            8 => &[7, 11, 9, 5],
            10 => &[7, 11, 13, 9, 5],
            12 => &[7, 11, 15, 13, 9, 5],
            _ => unreachable!(),
        };
        let rbanks: &[u32] = match self.rows {
            4 => &[8, 6],
            6 => &[8, 10, 6],
            8 => &[8, 12, 10, 6],
            10 => &[8, 12, 14, 10, 6],
            12 => &[8, 12, 16, 14, 10, 6],
            _ => unreachable!(),
        };
        let mut res = Vec::new();
        // left column
        for (i, b) in lbanks.iter().copied().enumerate() {
            let i = i as u32;
            for j in 0..32 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.cols_io[0],
                        row: i * 32 + j,
                        ioc: 0,
                        bel: k,
                        bank: b,
                        bbel: (32 - j) * 2 + k,
                    });
                }
            }
        }
        // center column
        // bank 4
        let base = (self.row_cfg - self.rows_cfg_io) * 16 - 8;
        for j in 0..8 {
            for k in 0..2 {
                res.push(Io {
                    col: self.cols_io[1],
                    row: base + j,
                    ioc: 1,
                    bel: k,
                    bank: 4,
                    bbel: (8 - j) * 2 + k,
                });
            }
        }
        // bank 2
        if self.rows_cfg_io > 1 {
            let base = (self.row_cfg - self.rows_cfg_io) * 16;
            for j in 0..16 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.cols_io[1],
                        row: base + j,
                        ioc: 1,
                        bel: k,
                        bank: 2,
                        bbel: (8 + 16 - (j ^ 8)) * 2 + k,
                    });
                }
            }
        }
        if self.rows_cfg_io > 2 {
            let base = self.row_cfg * 16 - 32;
            for j in 0..16 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.cols_io[1],
                        row: base + j,
                        ioc: 1,
                        bel: k,
                        bank: 2,
                        bbel: (24 + 16 - (j ^ 8)) * 2 + k,
                    });
                }
            }
        }
        let base = self.row_cfg * 16 - 16;
        for j in 0..8 {
            for k in 0..2 {
                res.push(Io {
                    col: self.cols_io[1],
                    row: base + j,
                    ioc: 1,
                    bel: k,
                    bank: 2,
                    bbel: (8 - j) * 2 + k,
                });
            }
        }
        // bank 1
        let base = self.row_cfg * 16 + 8;
        for j in 0..8 {
            for k in 0..2 {
                res.push(Io {
                    col: self.cols_io[1],
                    row: base + j,
                    ioc: 1,
                    bel: k,
                    bank: 1,
                    bbel: (8 - j) * 2 + k,
                });
            }
        }
        if self.rows_cfg_io > 2 {
            let base = self.row_cfg * 16 + 16;
            for j in 0..16 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.cols_io[1],
                        row: base + j,
                        ioc: 1,
                        bel: k,
                        bank: 1,
                        bbel: (24 + 16 - j) * 2 + k,
                    });
                }
            }
        }
        if self.rows_cfg_io > 1 {
            let base = (self.row_cfg + self.rows_cfg_io) * 16 - 16;
            for j in 0..16 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.cols_io[1],
                        row: base + j,
                        ioc: 1,
                        bel: k,
                        bank: 1,
                        bbel: (8 + 16 - j) * 2 + k,
                    });
                }
            }
        }
        // bank 3
        let base = (self.row_cfg + self.rows_cfg_io) * 16;
        for j in 0..8 {
            for k in 0..2 {
                res.push(Io {
                    col: self.cols_io[1],
                    row: base + j,
                    ioc: 1,
                    bel: k,
                    bank: 3,
                    bbel: (8 - j) * 2 + k,
                });
            }
        }
        // right column
        for (i, b) in rbanks.iter().copied().enumerate() {
            let i = i as u32;
            for j in 0..32 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.cols_io[2],
                        row: i * 32 + j,
                        ioc: 2,
                        bel: k,
                        bank: b,
                        bbel: (32 - j) * 2 + k,
                    });
                }
            }
        }
        res
    }

    pub fn get_gt(&self) -> Vec<Gt> {
        let mut res = Vec::new();
        if self.columns[0] == ColumnKind::Gt {
            let lbanks: &[u32] = match self.rows {
                4 => &[105, 102],
                6 => &[105, 103, 102],
                8 => &[106, 105, 103, 102],
                10 => &[106, 105, 103, 102, 101],
                12 => &[106, 105, 104, 103, 102, 101],
                _ => unreachable!(),
            };
            for (i, b) in lbanks.iter().copied().enumerate() {
                res.push(Gt {
                    col: 0,
                    row: i as u32 * 32,
                    gtc: 0,
                    bank: b,
                });
            }
        }
        if *self.columns.last().unwrap() == ColumnKind::Gt {
            let rbanks: &[u32] = match self.rows {
                4 => &[110, 113],
                6 => &[110, 112, 113],
                8 => &[109, 110, 112, 113],
                10 => &[109, 110, 112, 113, 114],
                12 => &[109, 110, 111, 112, 113, 114],
                _ => unreachable!(),
            };
            for (i, b) in rbanks.iter().copied().enumerate() {
                res.push(Gt {
                    col: self.columns.len() as u32 - 1,
                    row: i as u32 * 32,
                    gtc: 1,
                    bank: b,
                });
            }
        }
        res
    }

    pub fn get_sysmon_pads(&self) -> Vec<(String, u32, SysMonPin)> {
        let mut res = Vec::new();
        let has_gt = self.columns[0] == ColumnKind::Gt;
        if has_gt {
            if self.has_bot_sysmon {
                res.push((format!("IPAD_X1Y0"), 0, SysMonPin::VP));
                res.push((format!("IPAD_X1Y1"), 0, SysMonPin::VN));
            }
            if self.has_top_sysmon {
                let ipy = self.rows * 3;
                res.push((format!("IPAD_X1Y{}", ipy), 1, SysMonPin::VP));
                res.push((format!("IPAD_X1Y{}", ipy+1), 1, SysMonPin::VN));
            }
        } else {
            if self.has_bot_sysmon {
                res.push((format!("IPAD_X0Y0"), 0, SysMonPin::VP));
                res.push((format!("IPAD_X0Y1"), 0, SysMonPin::VN));
            }
            if self.has_top_sysmon {
                res.push((format!("IPAD_X0Y2"), 1, SysMonPin::VP));
                res.push((format!("IPAD_X0Y3"), 1, SysMonPin::VN));
            }
        }
        res
    }
}
