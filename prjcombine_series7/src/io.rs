#![allow(clippy::bool_to_int_with_if)]

use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::grid::{ColId, DieId, RowId};
use serde::{Deserialize, Serialize};

use crate::{ExtraDie, Grid, GtKind, GtPin, GtzPin, IoKind, PsPin, SharedCfgPin, SysMonPin};

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
            (14, 19) => Some(SharedCfgPin::Dout),
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
    pub fn sm_pair(&self, has_15: bool, has_35: bool) -> Option<u32> {
        if !has_35 {
            // Kintex, some Artix
            match (self.bank, self.row.to_idx() % 50) {
                (15, 25 | 26) => Some(5),
                (15, 27 | 28) => Some(12),
                (15, 29 | 30) => Some(4),
                (15, 31 | 32) => Some(11),
                (15, 33 | 34) => Some(3),
                (15, 35 | 36) => Some(10),
                (15, 39 | 40) => Some(2),
                (15, 41 | 42) => Some(9),
                (15, 43 | 44) => Some(1),
                (15, 45 | 46) => Some(8),
                (15, 47 | 48) => Some(0),
                _ => None,
            }
        } else if !has_15 {
            // Zynq
            match (self.bank, self.row.to_idx() % 50) {
                (35, 1 | 2) => Some(15),
                (35, 5 | 6) => Some(7),
                (35, 7 | 8) => Some(14),
                (35, 9 | 10) => Some(6),
                (35, 13 | 14) => Some(13),
                (35, 15 | 16) => Some(5),
                (35, 19 | 20) => Some(12),
                (35, 21 | 22) => Some(4),
                (35, 29 | 30) => Some(11),
                (35, 31 | 32) => Some(3),
                (35, 33 | 34) => Some(10),
                (35, 35 | 36) => Some(2),
                (35, 39 | 40) => Some(9),
                (35, 43 | 44) => Some(1),
                (35, 45 | 46) => Some(8),
                (35, 47 | 48) => Some(0),
                _ => None,
            }
        } else {
            // Virtex, most Artix
            match (self.bank, self.row.to_idx() % 50) {
                (15, 29 | 30) => Some(11),
                (15, 31 | 32) => Some(3),
                (15, 33 | 34) => Some(10),
                (15, 35 | 36) => Some(2),
                (15, 39 | 40) => Some(9),
                (15, 43 | 44) => Some(1),
                (15, 45 | 46) => Some(8),
                (15, 47 | 48) => Some(0),
                (35, 29 | 30) => Some(15),
                (35, 31 | 32) => Some(7),
                (35, 33 | 34) => Some(14),
                (35, 35 | 36) => Some(6),
                (35, 39 | 40) => Some(13),
                (35, 43 | 44) => Some(5),
                (35, 45 | 46) => Some(12),
                (35, 47 | 48) => Some(4),
                _ => None,
            }
        }
    }
}

pub fn get_io(grids: &EntityVec<DieId, Grid>, grid_master: DieId) -> Vec<Io> {
    let mut res = Vec::new();
    let reg_cfg: usize = grids[grid_master].reg_cfg.to_idx()
        + grids
            .iter()
            .filter(|&(k, _)| k < grid_master)
            .map(|(_, x)| x.regs)
            .sum::<usize>();
    for ioc in 0..2 {
        let iox = if grids[grid_master].cols_io[0].is_none() {
            0
        } else {
            ioc
        };
        let mut reg_base = 0;
        for (die, grid) in grids {
            if let Some(ref col) = grid.cols_io[ioc as usize] {
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
                                iox,
                                bank,
                                kind,
                            });
                        }
                    }
                }
            }
            reg_base += grid.regs;
        }
    }
    res
}

fn get_iopad_y(
    grids: &EntityVec<DieId, Grid>,
    extras: &[ExtraDie],
    is_7k70t: bool,
) -> Vec<(u32, u32, u32, u32, u32)> {
    let mut res = Vec::new();
    let mut ipy = 0;
    let mut opy = 0;
    let mut gy = 0;
    if extras.contains(&ExtraDie::GtzBottom) {
        ipy += 6;
        opy += 2;
    }
    for grid in grids.values() {
        for reg in grid.regs() {
            let mut has_gt = false;
            if let Some(ref col) = grid.cols_gt[0] {
                has_gt |= col.regs[reg].is_some();
            }
            if let Some(ref col) = grid.cols_gt[1] {
                has_gt |= col.regs[reg].is_some();
            }
            if let Some((ref lcol, ref rcol)) = grid.cols_gtp_mid {
                has_gt |= lcol.regs[reg].is_some();
                has_gt |= rcol.regs[reg].is_some();
            }
            if has_gt {
                if grid.reg_cfg == reg && !is_7k70t {
                    res.push((gy, opy, ipy, ipy + 24, ipy + 18));
                    ipy += 36;
                } else {
                    res.push((gy, opy, ipy, ipy + 18, 0));
                    ipy += 30;
                }
                gy += 1;
                opy += 8;
            } else {
                if grid.reg_cfg == reg && !is_7k70t {
                    res.push((0, 0, 0, 0, ipy));
                    ipy += 6;
                } else {
                    res.push((0, 0, 0, 0, 0));
                }
            }
        }
    }
    if is_7k70t {
        res[grids.first().unwrap().reg_cfg.to_idx()].4 = ipy + 6;
    }
    res
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Gt {
    pub col: ColId,
    pub row: RowId,
    pub reg_base: usize,
    pub die: DieId,
    pub gx: u32,
    pub gy: u32,
    pub ipx: u32,
    pub ipy_l: u32,
    pub ipy_h: u32,
    pub opx: u32,
    pub opy: u32,
    pub bank: u32,
    pub kind: GtKind,
}

impl Gt {
    pub fn get_pads(&self) -> Vec<(String, String, GtPin)> {
        let mut res = Vec::new();
        let l = match self.kind {
            GtKind::Gtp => "P",
            GtKind::Gtx => "X",
            GtKind::Gth => "H",
        };
        for b in 0..4 {
            res.push((
                format!("OPAD_X{}Y{}", self.opx, self.opy + 2 * b),
                format!("MGT{l}TXN{}_{}", b, self.bank),
                GtPin::TxN(b as u8),
            ));
            res.push((
                format!("OPAD_X{}Y{}", self.opx, self.opy + 2 * b + 1),
                format!("MGT{l}TXP{}_{}", b, self.bank),
                GtPin::TxP(b as u8),
            ));
        }
        for b in 0..2 {
            res.push((
                format!("IPAD_X{}Y{}", self.ipx, self.ipy_l + 6 * b),
                format!("MGT{l}RXN{}_{}", b, self.bank),
                GtPin::RxN(b as u8),
            ));
            res.push((
                format!("IPAD_X{}Y{}", self.ipx, self.ipy_l + 6 * b + 1),
                format!("MGT{l}RXP{}_{}", b, self.bank),
                GtPin::RxP(b as u8),
            ));
        }
        for b in 2..4 {
            res.push((
                format!("IPAD_X{}Y{}", self.ipx, self.ipy_h + 6 * (b - 2)),
                format!("MGT{l}RXN{}_{}", b, self.bank),
                GtPin::RxN(b as u8),
            ));
            res.push((
                format!("IPAD_X{}Y{}", self.ipx, self.ipy_h + 6 * (b - 2) + 1),
                format!("MGT{l}RXP{}_{}", b, self.bank),
                GtPin::RxP(b as u8),
            ));
        }
        for b in 0..2 {
            res.push((
                format!("IPAD_X{}Y{}", self.ipx, self.ipy_l + 8 + 2 * b),
                format!("MGTREFCLK{}P_{}", b, self.bank),
                GtPin::ClkP(b as u8),
            ));
            res.push((
                format!("IPAD_X{}Y{}", self.ipx, self.ipy_l + 9 + 2 * b),
                format!("MGTREFCLK{}N_{}", b, self.bank),
                GtPin::ClkN(b as u8),
            ));
        }
        res
    }
}

pub fn get_gt(
    grids: &EntityVec<DieId, Grid>,
    grid_master: DieId,
    extras: &[ExtraDie],
    is_7k70t: bool,
) -> Vec<Gt> {
    let iopad_y = get_iopad_y(grids, extras, is_7k70t);
    let reg_cfg: usize = grids[grid_master].reg_cfg.to_idx()
        + grids
            .iter()
            .filter(|&(k, _)| k < grid_master)
            .map(|(_, x)| x.regs)
            .sum::<usize>();
    let mut res = Vec::new();
    let mut reg_base = 0;
    let has_gtz = !extras.is_empty();
    for (die, grid) in grids {
        let has_left_gt = grid.cols_gt[0].is_some();
        for gtc in 0..2 {
            let gx: u32 = if has_left_gt { gtc } else { 0 };
            let opx: u32 = if has_gtz {
                gtc * 2
            } else if has_left_gt {
                gtc
            } else {
                0
            };
            let ipx: u32 = if has_gtz {
                gtc * 3
            } else if has_left_gt {
                gtc * 2
            } else if !is_7k70t {
                1
            } else {
                0
            };
            if let Some(ref col) = grid.cols_gt[gtc as usize] {
                for (reg, &kind) in &col.regs {
                    if let Some(kind) = kind {
                        let areg = reg_base + reg.to_idx();
                        let bank = if kind == GtKind::Gtp {
                            if grid.has_ps {
                                112
                            } else if areg == 0 {
                                213
                            } else {
                                216
                            }
                        } else {
                            (15 + areg - reg_cfg + [200, 100][gtc as usize]) as u32
                        };
                        let (gy, opy, ipy_l, ipy_h, _) = iopad_y[areg];
                        let row = grid.row_reg_bot(reg);
                        res.push(Gt {
                            col: col.col,
                            row,
                            die,
                            reg_base,
                            gx,
                            gy,
                            opx,
                            opy,
                            ipx,
                            ipy_l,
                            ipy_h,
                            bank,
                            kind,
                        });
                    }
                }
            }
        }
        if let Some((ref lcol, ref rcol)) = grid.cols_gtp_mid {
            for (gtc, col) in [(0, lcol), (1, rcol)] {
                let gx = gtc;
                let opx = gtc;
                let ipx = gtc + 1;
                for (reg, &kind) in &col.regs {
                    if let Some(kind) = kind {
                        let areg = reg_base + reg.to_idx();
                        let bank = if areg == 0 { 13 } else { 16 } + [200, 100][gtc as usize];
                        let (gy, opy, ipy_l, ipy_h, _) = iopad_y[areg];
                        let row = grid.row_reg_bot(reg);
                        res.push(Gt {
                            col: col.col,
                            row,
                            reg_base,
                            die,
                            gx,
                            gy,
                            opx,
                            opy,
                            ipx,
                            ipy_l,
                            ipy_h,
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

pub fn get_sysmon_pads(
    grids: &EntityVec<DieId, Grid>,
    extras: &[ExtraDie],
    is_7k70t: bool,
) -> Vec<(String, DieId, SysMonPin)> {
    let iopad_y = get_iopad_y(grids, extras, is_7k70t);
    let mut res = Vec::new();
    let mut reg_base = 0;
    for (i, grid) in grids {
        if grid.reg_cfg.to_idx() == grid.regs {
            continue;
        }
        let ipx = if grid.cols_gt[0].is_some() { 1 } else { 0 };
        let ipy = iopad_y[reg_base + grid.reg_cfg.to_idx()].4;
        res.push((format!("IPAD_X{}Y{}", ipx, ipy), i, SysMonPin::VP));
        res.push((format!("IPAD_X{}Y{}", ipx, ipy + 1), i, SysMonPin::VN));
        reg_base += grid.regs;
    }
    res
}

pub fn get_gtz_pads(extras: &[ExtraDie]) -> Vec<(String, String, u32, GtzPin)> {
    let mut res = Vec::new();
    let has_gtz_bot = extras.contains(&ExtraDie::GtzBottom);
    let has_gtz_top = extras.contains(&ExtraDie::GtzTop);
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

pub fn get_ps_pads(grids: &EntityVec<DieId, Grid>) -> Vec<(String, u32, PsPin)> {
    let mut res = Vec::new();
    if grids.first().unwrap().has_ps {
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
