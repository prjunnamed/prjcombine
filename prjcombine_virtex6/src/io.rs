use prjcombine_entity::EntityId;
use prjcombine_int::grid::{ColId, RowId};
use serde::{Deserialize, Serialize};

use crate::{ExpandedDevice, SharedCfgPin};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Io {
    pub col: ColId,
    pub row: RowId,
    pub ioc: u32,
    pub iox: u32,
    pub bank: u32,
    pub bbel: u32,
}

impl Io {
    pub fn iob_name(&self) -> String {
        let x = self.iox;
        let y = self.row.to_idx();
        format!("IOB_X{x}Y{y}")
    }
    pub fn is_mrcc(&self) -> bool {
        matches!(self.row.to_idx() % 40, 18..=21)
    }
    pub fn is_srcc(&self) -> bool {
        matches!(self.row.to_idx() % 40, 16 | 17 | 22 | 23)
    }
    pub fn is_gc(&self) -> bool {
        matches!(
            (self.bank, self.row.to_idx() % 40),
            (24 | 34, 36..=39) | (25 | 35, 0..=3)
        )
    }
    pub fn is_vref(&self) -> bool {
        self.row.to_idx() % 20 == 10
    }
    pub fn is_vr(&self) -> bool {
        match self.bank {
            34 => matches!(self.row.to_idx() % 40, 0 | 1),
            24 => matches!(self.row.to_idx() % 40, 4 | 5),
            15 | 25 | 35 => matches!(self.row.to_idx() % 40, 6 | 7),
            _ => matches!(self.row.to_idx() % 40, 14 | 15),
        }
    }
    pub fn get_cfg(&self) -> Option<SharedCfgPin> {
        match (self.bank, self.row.to_idx() % 40) {
            (24, 6) => Some(SharedCfgPin::CsoB),
            (24, 7) => Some(SharedCfgPin::Rs(0)),
            (24, 8) => Some(SharedCfgPin::Rs(1)),
            (24, 9) => Some(SharedCfgPin::FweB),
            (24, 10) => Some(SharedCfgPin::FoeB),
            (24, 11) => Some(SharedCfgPin::FcsB),
            (24, 12) => Some(SharedCfgPin::Data(0)),
            (24, 13) => Some(SharedCfgPin::Data(1)),
            (24, 14) => Some(SharedCfgPin::Data(2)),
            (24, 15) => Some(SharedCfgPin::Data(3)),
            (24, 24) => Some(SharedCfgPin::Data(4)),
            (24, 25) => Some(SharedCfgPin::Data(5)),
            (24, 26) => Some(SharedCfgPin::Data(6)),
            (24, 27) => Some(SharedCfgPin::Data(7)),
            (24, 28) => Some(SharedCfgPin::Data(8)),
            (24, 29) => Some(SharedCfgPin::Data(9)),
            (24, 30) => Some(SharedCfgPin::Data(10)),
            (24, 31) => Some(SharedCfgPin::Data(11)),
            (24, 32) => Some(SharedCfgPin::Data(12)),
            (24, 33) => Some(SharedCfgPin::Data(13)),
            (24, 34) => Some(SharedCfgPin::Data(14)),
            (24, 35) => Some(SharedCfgPin::Data(15)),
            (34, 2) => Some(SharedCfgPin::Addr(16)),
            (34, 3) => Some(SharedCfgPin::Addr(17)),
            (34, 4) => Some(SharedCfgPin::Addr(18)),
            (34, 5) => Some(SharedCfgPin::Addr(19)),
            (34, 6) => Some(SharedCfgPin::Addr(20)),
            (34, 7) => Some(SharedCfgPin::Addr(21)),
            (34, 8) => Some(SharedCfgPin::Addr(22)),
            (34, 9) => Some(SharedCfgPin::Addr(23)),
            (34, 10) => Some(SharedCfgPin::Addr(24)),
            (34, 11) => Some(SharedCfgPin::Addr(25)),
            (34, 12) => Some(SharedCfgPin::Data(16)),
            (34, 13) => Some(SharedCfgPin::Data(17)),
            (34, 14) => Some(SharedCfgPin::Data(18)),
            (34, 15) => Some(SharedCfgPin::Data(19)),
            (34, 24) => Some(SharedCfgPin::Data(20)),
            (34, 25) => Some(SharedCfgPin::Data(21)),
            (34, 26) => Some(SharedCfgPin::Data(22)),
            (34, 27) => Some(SharedCfgPin::Data(23)),
            (34, 28) => Some(SharedCfgPin::Data(24)),
            (34, 29) => Some(SharedCfgPin::Data(25)),
            (34, 30) => Some(SharedCfgPin::Data(26)),
            (34, 31) => Some(SharedCfgPin::Data(27)),
            (34, 32) => Some(SharedCfgPin::Data(28)),
            (34, 33) => Some(SharedCfgPin::Data(29)),
            (34, 34) => Some(SharedCfgPin::Data(30)),
            (34, 35) => Some(SharedCfgPin::Data(31)),
            _ => None,
        }
    }
}

impl ExpandedDevice<'_> {
    pub fn get_io(&self) -> Vec<Io> {
        let mut res = Vec::new();
        let mut iox = 0;
        for (ioc, col) in [self.col_lio, self.col_lcio, self.col_rcio, self.col_rio]
            .into_iter()
            .enumerate()
        {
            if let Some(col) = col {
                for reg in self.grid.regs() {
                    let bank = (reg - self.grid.reg_cfg + 15) as usize + ioc * 10;
                    for k in 0..40 {
                        res.push(Io {
                            col,
                            row: self.grid.row_reg_bot(reg) + k,
                            ioc: ioc as u32,
                            iox,
                            bank: bank as u32,
                            bbel: 39 - k as u32,
                        });
                    }
                }
                iox += 1;
            }
        }
        res
    }
}
