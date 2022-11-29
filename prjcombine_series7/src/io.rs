#![allow(clippy::bool_to_int_with_if)]

use prjcombine_entity::EntityId;
use prjcombine_int::grid::{ColId, DieId, RowId};
use serde::{Deserialize, Serialize};

use crate::{ExpandedDevice, ExtraDie, GtzPin, IoKind, PsPin, SharedCfgPin};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Io {
    pub col: ColId,
    pub row: RowId,
    pub die: DieId,
    pub reg_base: usize,
    pub ioc: u32,
    pub iox: u32,
    pub bank: u32,
    pub kind: IoKind,
}

impl Io {
    pub fn iob_name(&self) -> String {
        let x = self.iox;
        let y = self.row.to_idx() + self.reg_base * 50;
        format!("IOB_X{x}Y{y}")
    }
    pub fn is_mrcc(&self) -> bool {
        matches!(self.row.to_idx() % 50, 23..=26)
    }
    pub fn is_srcc(&self) -> bool {
        matches!(self.row.to_idx() % 50, 21 | 22 | 27 | 28)
    }
    pub fn is_dqs(&self) -> bool {
        matches!(self.row.to_idx() % 50, 7 | 8 | 19 | 20 | 31 | 32 | 43 | 44)
    }
    pub fn is_vref(&self) -> bool {
        matches!(self.row.to_idx() % 50, 11 | 37)
    }
    pub fn is_vrp(&self) -> bool {
        self.kind == IoKind::Hpio && matches!(self.row.to_idx() % 50, 0)
    }
    pub fn is_vrn(&self) -> bool {
        self.kind == IoKind::Hpio && matches!(self.row.to_idx() % 50, 49)
    }
    pub fn get_cfg(&self, has_14: bool) -> Option<SharedCfgPin> {
        match (self.bank, self.row.to_idx() % 50) {
            (14, 1) => Some(SharedCfgPin::Data(16)),
            (14, 2) => Some(SharedCfgPin::Data(17)),
            (14, 3) => Some(SharedCfgPin::Data(18)),
            (14, 4) => Some(SharedCfgPin::Data(19)),
            (14, 5) => Some(SharedCfgPin::Data(20)),
            (14, 6) => Some(SharedCfgPin::Data(21)),
            (14, 7) => Some(SharedCfgPin::Data(22)),
            (14, 9) => Some(SharedCfgPin::Data(23)),
            (14, 10) => Some(SharedCfgPin::Data(24)),
            (14, 11) => Some(SharedCfgPin::Data(25)),
            (14, 12) => Some(SharedCfgPin::Data(26)),
            (14, 13) => Some(SharedCfgPin::Data(27)),
            (14, 14) => Some(SharedCfgPin::Data(28)),
            (14, 15) => Some(SharedCfgPin::Data(29)),
            (14, 16) => Some(SharedCfgPin::Data(30)),
            (14, 17) => Some(SharedCfgPin::Data(31)),
            (14, 18) => Some(SharedCfgPin::CsiB),
            (14, 19) => Some(SharedCfgPin::CsoB),
            (14, 20) => Some(SharedCfgPin::RdWrB),
            (14, 29) => Some(SharedCfgPin::Data(15)),
            (14, 30) => Some(SharedCfgPin::Data(14)),
            (14, 31) => Some(SharedCfgPin::Data(13)),
            (14, 33) => Some(SharedCfgPin::Data(12)),
            (14, 34) => Some(SharedCfgPin::Data(11)),
            (14, 35) => Some(SharedCfgPin::Data(10)),
            (14, 36) => Some(SharedCfgPin::Data(9)),
            (14, 37) => Some(SharedCfgPin::Data(8)),
            (14, 38) => Some(SharedCfgPin::FcsB),
            (14, 39) => Some(SharedCfgPin::Data(7)),
            (14, 40) => Some(SharedCfgPin::Data(6)),
            (14, 41) => Some(SharedCfgPin::Data(5)),
            (14, 42) => Some(SharedCfgPin::Data(4)),
            (14, 43) => Some(SharedCfgPin::EmCclk),
            (14, 44) => Some(SharedCfgPin::PudcB),
            (14, 45) => Some(SharedCfgPin::Data(3)),
            (14, 46) => Some(SharedCfgPin::Data(2)),
            (14, 47) => Some(SharedCfgPin::Data(1)),
            (14, 48) => Some(SharedCfgPin::Data(0)),
            (15, 1) => Some(SharedCfgPin::Rs(0)),
            (15, 2) => Some(SharedCfgPin::Rs(1)),
            (15, 3) => Some(SharedCfgPin::FweB),
            (15, 4) => Some(SharedCfgPin::FoeB),
            (15, 5) => Some(SharedCfgPin::Addr(16)),
            (15, 6) => Some(SharedCfgPin::Addr(17)),
            (15, 7) => Some(SharedCfgPin::Addr(18)),
            (15, 9) => Some(SharedCfgPin::Addr(19)),
            (15, 10) => Some(SharedCfgPin::Addr(20)),
            (15, 11) => Some(SharedCfgPin::Addr(21)),
            (15, 12) => Some(SharedCfgPin::Addr(22)),
            (15, 13) => Some(SharedCfgPin::Addr(23)),
            (15, 14) => Some(SharedCfgPin::Addr(24)),
            (15, 15) => Some(SharedCfgPin::Addr(25)),
            (15, 16) => Some(SharedCfgPin::Addr(26)),
            (15, 17) => Some(SharedCfgPin::Addr(27)),
            (15, 18) => Some(SharedCfgPin::Addr(28)),
            (15, 19) => Some(SharedCfgPin::AdvB),
            (34, 44) if !has_14 => Some(SharedCfgPin::PudcB),
            _ => None,
        }
    }
}

impl ExpandedDevice<'_> {
    pub fn get_io(&self) -> Vec<Io> {
        let mut res = Vec::new();
        let reg_cfg: usize = self.grids[self.grid_master].reg_cfg.to_idx()
            + self
                .grids
                .iter()
                .filter(|&(k, _)| k < self.grid_master)
                .map(|(_, x)| x.regs)
                .sum::<usize>();
        let mut reg_base = 0;
        for (die, grid) in &self.grids {
            for (iox, col) in grid.cols_io.iter().enumerate() {
                let ioc = if col.col < self.col_clk { 0 } else { 1 };
                for (reg, &kind) in &col.regs {
                    if let Some(kind) = kind {
                        let bank = (15 + reg_base + reg.to_idx() - reg_cfg) as u32 + ioc * 20;
                        for k in 0..50 {
                            let row = grid.row_reg_bot(reg) + k;
                            res.push(Io {
                                col: col.col,
                                row,
                                die,
                                reg_base,
                                ioc,
                                iox: iox as u32,
                                bank,
                                kind,
                            });
                        }
                    }
                }
            }
            reg_base += grid.regs;
        }
        res
    }

    pub fn get_gtz_pads(&self) -> Vec<(String, String, u32, GtzPin)> {
        let mut res = Vec::new();
        let has_gtz_bot = self.extras.contains(&ExtraDie::GtzBottom);
        let has_gtz_top = self.extras.contains(&ExtraDie::GtzTop);
        if has_gtz_bot {
            let ipy = 0;
            let opy = 0;
            for b in 0..8 {
                res.push((
                    format!("IPAD_X2Y{}", ipy + 4 + 2 * b),
                    format!("MGTZRXN{b}_400"),
                    400,
                    GtzPin::RxN(b as u8),
                ));
                res.push((
                    format!("IPAD_X2Y{}", ipy + 5 + 2 * b),
                    format!("MGTZRXP{b}_400"),
                    400,
                    GtzPin::RxP(b as u8),
                ));
                res.push((
                    format!("OPAD_X1Y{}", opy + 2 * b),
                    format!("MGTZTXN{b}_400"),
                    400,
                    GtzPin::TxN(b as u8),
                ));
                res.push((
                    format!("OPAD_X1Y{}", opy + 1 + 2 * b),
                    format!("MGTZTXP{b}_400"),
                    400,
                    GtzPin::TxP(b as u8),
                ));
            }
            for b in 0..2 {
                res.push((
                    format!("IPAD_X2Y{}", ipy + 2 * b),
                    format!("MGTZREFCLK{b}N_400"),
                    400,
                    GtzPin::ClkN(b as u8),
                ));
                res.push((
                    format!("IPAD_X2Y{}", ipy + 1 + 2 * b),
                    format!("MGTZREFCLK{b}P_400"),
                    400,
                    GtzPin::ClkP(b as u8),
                ));
            }
        }
        if has_gtz_top {
            let ipy = if has_gtz_bot { 20 } else { 0 };
            let opy = if has_gtz_bot { 16 } else { 0 };
            for b in 0..8 {
                res.push((
                    format!("IPAD_X2Y{}", ipy + 4 + 2 * b),
                    format!("MGTZRXN{b}_300"),
                    300,
                    GtzPin::RxN(b as u8),
                ));
                res.push((
                    format!("IPAD_X2Y{}", ipy + 5 + 2 * b),
                    format!("MGTZRXP{b}_300"),
                    300,
                    GtzPin::RxP(b as u8),
                ));
                res.push((
                    format!("OPAD_X1Y{}", opy + 2 * b),
                    format!("MGTZTXN{b}_300"),
                    300,
                    GtzPin::TxN(b as u8),
                ));
                res.push((
                    format!("OPAD_X1Y{}", opy + 1 + 2 * b),
                    format!("MGTZTXP{b}_300"),
                    300,
                    GtzPin::TxP(b as u8),
                ));
            }
            for b in 0..2 {
                res.push((
                    format!("IPAD_X2Y{}", ipy + 2 * b),
                    format!("MGTZREFCLK{b}N_300"),
                    300,
                    GtzPin::ClkN(b as u8),
                ));
                res.push((
                    format!("IPAD_X2Y{}", ipy + 1 + 2 * b),
                    format!("MGTZREFCLK{b}P_300"),
                    300,
                    GtzPin::ClkP(b as u8),
                ));
            }
        }
        res
    }

    pub fn get_ps_pads(&self) -> Vec<(String, u32, PsPin)> {
        let mut res = Vec::new();
        if self.grids.first().unwrap().has_ps {
            res.push(("IOPAD_X1Y1".to_string(), 502, PsPin::DdrWeB));
            res.push(("IOPAD_X1Y2".to_string(), 502, PsPin::DdrVrN));
            res.push(("IOPAD_X1Y3".to_string(), 502, PsPin::DdrVrP));
            for i in 0..13 {
                res.push((format!("IOPAD_X1Y{}", 4 + i), 502, PsPin::DdrA(i)));
            }
            res.push(("IOPAD_X1Y17".to_string(), 502, PsPin::DdrA(14)));
            res.push(("IOPAD_X1Y18".to_string(), 502, PsPin::DdrA(13)));
            for i in 0..3 {
                res.push((format!("IOPAD_X1Y{}", 19 + i), 502, PsPin::DdrBa(i)));
            }
            res.push(("IOPAD_X1Y22".to_string(), 502, PsPin::DdrCasB));
            res.push(("IOPAD_X1Y23".to_string(), 502, PsPin::DdrCke(0)));
            res.push(("IOPAD_X1Y24".to_string(), 502, PsPin::DdrCkN(0)));
            res.push(("IOPAD_X1Y25".to_string(), 502, PsPin::DdrCkP(0)));
            res.push(("IOPAD_X1Y26".to_string(), 500, PsPin::Clk));
            res.push(("IOPAD_X1Y27".to_string(), 502, PsPin::DdrCsB(0)));
            for i in 0..4 {
                res.push((format!("IOPAD_X1Y{}", 28 + i), 502, PsPin::DdrDm(i)));
            }
            for i in 0..32 {
                res.push((format!("IOPAD_X1Y{}", 32 + i), 502, PsPin::DdrDq(i)));
            }
            for i in 0..4 {
                res.push((format!("IOPAD_X1Y{}", 64 + i), 502, PsPin::DdrDqsN(i)));
            }
            for i in 0..4 {
                res.push((format!("IOPAD_X1Y{}", 68 + i), 502, PsPin::DdrDqsP(i)));
            }
            res.push(("IOPAD_X1Y72".to_string(), 502, PsPin::DdrDrstB));
            for i in 0..54 {
                res.push((
                    format!("IOPAD_X1Y{}", 77 + i),
                    if i < 16 { 500 } else { 501 },
                    PsPin::Mio(i),
                ));
            }
            res.push(("IOPAD_X1Y131".to_string(), 502, PsPin::DdrOdt(0)));
            res.push(("IOPAD_X1Y132".to_string(), 500, PsPin::PorB));
            res.push(("IOPAD_X1Y133".to_string(), 502, PsPin::DdrRasB));
            res.push(("IOPAD_X1Y134".to_string(), 501, PsPin::SrstB));
        }
        res
    }
}
