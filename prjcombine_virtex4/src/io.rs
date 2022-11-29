use prjcombine_entity::EntityId;
use prjcombine_int::grid::{ColId, RowId};
use serde::{Deserialize, Serialize};

use crate::bond::SharedCfgPin;
use crate::ExpandedDevice;

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
        let y = self.row.to_idx() * 2 + self.bel as usize;
        format!("IOB_X{x}Y{y}")
    }
    pub fn is_cc(&self) -> bool {
        matches!(self.row.to_idx() % 16, 7 | 8)
    }
    pub fn is_lc(&self) -> bool {
        matches!(self.row.to_idx() % 16, 7 | 8) || self.ioc == 1
    }
    pub fn is_gc(&self) -> bool {
        matches!(self.bank, 3 | 4) || (matches!(self.bank, 1 | 2) && matches!(self.bbel, 18..=33))
    }
    pub fn is_vref(&self) -> bool {
        self.row.to_idx() % 8 == 4 && self.bel == 0
    }
    pub fn is_vr(&self) -> bool {
        match self.bank {
            1 => self.bbel / 2 == 18,
            2 => self.bbel / 2 == 23,
            3 => self.bbel / 2 == 2,
            4 => self.bbel / 2 == 7,
            _ => self.row.to_idx() % 32 == 9,
        }
    }
    pub fn get_cfg(&self) -> Option<SharedCfgPin> {
        if !matches!(self.bank, 1 | 2) {
            return None;
        }
        if self.bbel > 17 {
            return None;
        }
        if self.bank == 2 {
            Some(SharedCfgPin::Data(
                (self.row.to_idx() % 8 * 2 + self.bel as usize) as u8,
            ))
        } else {
            Some(SharedCfgPin::Data(
                (self.row.to_idx() % 8 * 2 + self.bel as usize + 16) as u8,
            ))
        }
    }
}

impl ExpandedDevice<'_> {
    pub fn get_io(&self) -> Vec<Io> {
        let lbanks: &[u32] = match self.grid.regs {
            4 => &[7, 5],
            6 => &[7, 9, 5],
            8 => &[7, 11, 9, 5],
            10 => &[7, 11, 13, 9, 5],
            12 => &[7, 11, 15, 13, 9, 5],
            _ => unreachable!(),
        };
        let rbanks: &[u32] = match self.grid.regs {
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
            for j in 0..32 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.col_lio.unwrap(),
                        row: RowId::from_idx(i * 32 + j),
                        ioc: 0,
                        bel: k,
                        bank: b,
                        bbel: (32 - j as u32) * 2 + k,
                    });
                }
            }
        }
        // center column
        // bank 4
        let base = self.row_dcmiob.unwrap();
        for j in 0..8 {
            for k in 0..2 {
                res.push(Io {
                    col: self.col_cfg,
                    row: base + j,
                    ioc: 1,
                    bel: k,
                    bank: 4,
                    bbel: (8 - j as u32) * 2 + k,
                });
            }
        }
        let regs_cfg_io = (self.grid.row_bufg() - 8 - self.row_dcmiob.unwrap()) / 16;
        // bank 2
        if regs_cfg_io > 1 {
            let base = self.row_dcmiob.unwrap() + 8;
            for j in 0..16 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.col_cfg,
                        row: base + j,
                        ioc: 1,
                        bel: k,
                        bank: 2,
                        bbel: (8 + 16 - (j as u32 ^ 8)) * 2 + k,
                    });
                }
            }
        }
        if regs_cfg_io > 2 {
            let base = self.grid.row_reg_bot(self.grid.reg_cfg - 2);
            for j in 0..16 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.col_cfg,
                        row: base + j,
                        ioc: 1,
                        bel: k,
                        bank: 2,
                        bbel: (24 + 16 - (j as u32 ^ 8)) * 2 + k,
                    });
                }
            }
        }
        let base = self.grid.row_reg_bot(self.grid.reg_cfg - 1);
        for j in 0..8 {
            for k in 0..2 {
                res.push(Io {
                    col: self.col_cfg,
                    row: base + j,
                    ioc: 1,
                    bel: k,
                    bank: 2,
                    bbel: (8 - j as u32) * 2 + k,
                });
            }
        }
        // bank 1
        let base = self.grid.row_bufg() + 8;
        for j in 0..8 {
            for k in 0..2 {
                res.push(Io {
                    col: self.col_cfg,
                    row: base + j,
                    ioc: 1,
                    bel: k,
                    bank: 1,
                    bbel: (8 - j as u32) * 2 + k,
                });
            }
        }
        if regs_cfg_io > 2 {
            let base = self.grid.row_bufg() + 16;
            for j in 0..16 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.col_cfg,
                        row: base + j,
                        ioc: 1,
                        bel: k,
                        bank: 1,
                        bbel: (24 + 16 - j as u32) * 2 + k,
                    });
                }
            }
        }
        if regs_cfg_io > 1 {
            let base = self.row_iobdcm.unwrap() - 24;
            for j in 0..16 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.col_cfg,
                        row: base + j,
                        ioc: 1,
                        bel: k,
                        bank: 1,
                        bbel: (8 + 16 - j as u32) * 2 + k,
                    });
                }
            }
        }
        // bank 3
        let base = self.row_iobdcm.unwrap() - 8;
        for j in 0..8 {
            for k in 0..2 {
                res.push(Io {
                    col: self.col_cfg,
                    row: base + j,
                    ioc: 1,
                    bel: k,
                    bank: 3,
                    bbel: (8 - j as u32) * 2 + k,
                });
            }
        }
        // right column
        for (i, b) in rbanks.iter().copied().enumerate() {
            for j in 0..32 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.col_rio.unwrap(),
                        row: RowId::from_idx(i * 32 + j),
                        ioc: 2,
                        bel: k,
                        bank: b,
                        bbel: (32 - j as u32) * 2 + k,
                    });
                }
            }
        }
        res
    }
}
