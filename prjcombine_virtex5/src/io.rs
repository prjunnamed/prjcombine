use prjcombine_entity::EntityId;
use prjcombine_int::grid::{ColId, RowId};
use serde::{Deserialize, Serialize};

use crate::{ExpandedDevice, SharedCfgPin};

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

impl ExpandedDevice<'_> {
    pub fn get_io(&self) -> Vec<Io> {
        let mut res = Vec::new();
        // left column
        for reg in self.grid.regs() {
            let bank = if reg < self.grid.reg_cfg {
                13 + (self.grid.reg_cfg - reg - 1) * 4
            } else {
                11 + (reg - self.grid.reg_cfg) * 4
            };
            for j in 0..20 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.col_lio.unwrap(),
                        row: self.grid.row_reg_bot(reg) + j,
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
        if self.grid.reg_cfg.to_idx() > 3 {
            for reg in self.grid.regs() {
                if reg >= self.grid.reg_cfg - 3 {
                    continue;
                }
                let bank = 6 + (self.grid.reg_cfg - 4 - reg) * 2;
                for j in 0..20 {
                    for k in 0..2 {
                        res.push(Io {
                            col: self.col_cfg,
                            row: self.grid.row_reg_bot(reg) + j,
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
        let row_ioi_cmt = if self.grid.reg_cfg.to_idx() == 1 {
            RowId::from_idx(0)
        } else {
            self.grid.row_bufg() - 30
        };
        for (bank, base) in [
            (4, row_ioi_cmt),
            (2, self.grid.row_bufg() - 20),
            (1, self.grid.row_bufg() + 10),
            (3, self.grid.row_bufg() + 20),
        ] {
            if bank == 4 && self.grid.reg_cfg.to_idx() == 1 {
                continue;
            }
            for j in 0..10 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.col_cfg,
                        row: base + j,
                        ioc: 1,
                        bel: k,
                        bank,
                        bbel: (9 - j as u32) * 2 + k,
                    });
                }
            }
        }
        // top banks
        if (self.grid.regs - self.grid.reg_cfg.to_idx()) > 3 {
            for reg in self.grid.regs() {
                if reg < self.grid.reg_cfg + 3 {
                    continue;
                }
                let bank = 5 + (reg - self.grid.reg_cfg - 3) * 2;
                for j in 0..20 {
                    for k in 0..2 {
                        res.push(Io {
                            col: self.col_cfg,
                            row: self.grid.row_reg_bot(reg) + j,
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
        if let Some(col) = self.col_rio {
            for reg in self.grid.regs() {
                let bank = if reg < self.grid.reg_cfg {
                    14 + (self.grid.reg_cfg - reg - 1) * 4
                } else {
                    12 + (reg - self.grid.reg_cfg) * 4
                };
                for j in 0..20 {
                    for k in 0..2 {
                        res.push(Io {
                            col,
                            row: self.grid.row_reg_bot(reg) + j,
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
}
