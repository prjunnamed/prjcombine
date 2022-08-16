use std::collections::{BTreeSet, HashMap};
use serde::{Serialize, Deserialize};
use super::{CfgPin, ExtraDie, SysMonPin, GtPin, PsPin, ColId, RowId, SlrId, int, eint};
use ndarray::Array2;
use prjcombine_entity::{EntityVec, EntityId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GridKind {
    Artix,
    Kintex,
    Virtex,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub kind: GridKind,
    pub columns: EntityVec<ColId, ColumnKind>,
    pub cols_vbrk: BTreeSet<ColId>,
    pub col_cfg: ColId,
    pub col_clk: ColId,
    pub cols_io: [Option<IoColumn>; 2],
    pub cols_gt: [Option<GtColumn>; 2],
    pub regs: usize,
    pub reg_cfg: usize,
    pub reg_clk: usize,
    pub holes: Vec<Hole>,
    pub has_ps: bool,
    pub has_slr: bool,
    pub has_no_tbuturn: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKind {
    ClbLL,
    ClbLM,
    Bram,
    Dsp,
    Io,
    Cmt,
    Gt,
    Cfg,
    Clk,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IoColumn {
    pub col: ColId,
    pub regs: Vec<Option<IoKind>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GtColumn {
    pub col: ColId,
    pub regs: Vec<Option<GtKind>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtKind {
    Gtp,
    Gtx,
    Gth,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IoKind {
    Hpio,
    Hrio,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HoleKind {
    Pcie2Left,
    Pcie2Right,
    Pcie3,
    GtpLeft,
    GtpRight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Hole {
    pub kind: HoleKind,
    pub col: ColId,
    pub row: RowId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Io {
    pub col: ColId,
    pub row: RowId,
    pub slr: SlrId,
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
    pub fn get_cfg(&self, has_14: bool) -> Option<CfgPin> {
        match (self.bank, self.row.to_idx() % 50) {
            (14, 1) => Some(CfgPin::Data(16)),
            (14, 2) => Some(CfgPin::Data(17)),
            (14, 3) => Some(CfgPin::Data(18)),
            (14, 4) => Some(CfgPin::Data(19)),
            (14, 5) => Some(CfgPin::Data(20)),
            (14, 6) => Some(CfgPin::Data(21)),
            (14, 7) => Some(CfgPin::Data(22)),
            (14, 9) => Some(CfgPin::Data(23)),
            (14, 10) => Some(CfgPin::Data(24)),
            (14, 11) => Some(CfgPin::Data(25)),
            (14, 12) => Some(CfgPin::Data(26)),
            (14, 13) => Some(CfgPin::Data(27)),
            (14, 14) => Some(CfgPin::Data(28)),
            (14, 15) => Some(CfgPin::Data(29)),
            (14, 16) => Some(CfgPin::Data(30)),
            (14, 17) => Some(CfgPin::Data(31)),
            (14, 18) => Some(CfgPin::CsiB),
            (14, 19) => Some(CfgPin::Dout),
            (14, 20) => Some(CfgPin::RdWrB),
            (14, 29) => Some(CfgPin::Data(15)),
            (14, 30) => Some(CfgPin::Data(14)),
            (14, 31) => Some(CfgPin::Data(13)),
            (14, 33) => Some(CfgPin::Data(12)),
            (14, 34) => Some(CfgPin::Data(11)),
            (14, 35) => Some(CfgPin::Data(10)),
            (14, 36) => Some(CfgPin::Data(9)),
            (14, 37) => Some(CfgPin::Data(8)),
            (14, 38) => Some(CfgPin::FcsB),
            (14, 39) => Some(CfgPin::Data(7)),
            (14, 40) => Some(CfgPin::Data(6)),
            (14, 41) => Some(CfgPin::Data(5)),
            (14, 42) => Some(CfgPin::Data(4)),
            (14, 43) => Some(CfgPin::UserCclk),
            (14, 44) => Some(CfgPin::HswapEn),
            (14, 45) => Some(CfgPin::Data(3)),
            (14, 46) => Some(CfgPin::Data(2)),
            (14, 47) => Some(CfgPin::Data(1)),
            (14, 48) => Some(CfgPin::Data(0)),
            (15, 1) => Some(CfgPin::Rs(0)),
            (15, 2) => Some(CfgPin::Rs(1)),
            (15, 3) => Some(CfgPin::FweB),
            (15, 4) => Some(CfgPin::FoeB),
            (15, 5) => Some(CfgPin::Addr(16)),
            (15, 6) => Some(CfgPin::Addr(17)),
            (15, 7) => Some(CfgPin::Addr(18)),
            (15, 9) => Some(CfgPin::Addr(19)),
            (15, 10) => Some(CfgPin::Addr(20)),
            (15, 11) => Some(CfgPin::Addr(21)),
            (15, 12) => Some(CfgPin::Addr(22)),
            (15, 13) => Some(CfgPin::Addr(23)),
            (15, 14) => Some(CfgPin::Addr(24)),
            (15, 15) => Some(CfgPin::Addr(25)),
            (15, 16) => Some(CfgPin::Addr(26)),
            (15, 17) => Some(CfgPin::Addr(27)),
            (15, 18) => Some(CfgPin::Addr(28)),
            (15, 19) => Some(CfgPin::AdvB),
            (34, 44) if !has_14 => Some(CfgPin::HswapEn),
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

pub fn get_io(grids: &EntityVec<SlrId, Grid>, grid_master: SlrId) -> Vec<Io> {
    let mut res = Vec::new();
    let reg_cfg: usize = grids[grid_master].reg_cfg + grids.iter().filter(|&(k, _)| k < grid_master).map(|(_, x)| x.regs).sum::<usize>();
    for ioc in 0..2 {
        let iox = if grids[grid_master].cols_io[0].is_none() {0} else {ioc};
        let mut reg_base = 0;
        for (slr, grid) in grids {
            if let Some(ref col) = grid.cols_io[ioc as usize] {
                for (j, &kind) in col.regs.iter().enumerate() {
                    if let Some(kind) = kind {
                        let bank = (15 + reg_base + j - reg_cfg) as u32 + ioc * 20;
                        for k in 0..50 {
                            let row = RowId::from_idx(j * 50 + k);
                            res.push(Io {
                                col: col.col,
                                row,
                                slr,
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

fn get_iopad_y(grids: &EntityVec<SlrId, Grid>, extras: &[ExtraDie], is_7k70t: bool) -> Vec<(u32, u32, u32, u32, u32)> {
    let mut res = Vec::new();
    let mut ipy = 0;
    let mut opy = 0;
    let mut gy = 0;
    if extras.contains(&ExtraDie::GtzBottom) {
        ipy += 6;
        opy += 2;
    }
    for grid in grids.values() {
        for j in 0..grid.regs {
            let mut has_gt = false;
            if let Some(ref col) = grid.cols_gt[0] {
                has_gt |= col.regs[j].is_some();
            }
            if let Some(ref col) = grid.cols_gt[1] {
                has_gt |= col.regs[j].is_some();
            }
            for hole in &grid.holes {
                if matches!(hole.kind, HoleKind::GtpLeft | HoleKind::GtpRight) && hole.row == RowId::from_idx(j * 50) {
                    has_gt = true;
                }
            }
            if has_gt {
                if grid.reg_cfg == j && !is_7k70t {
                    res.push((gy, opy, ipy, ipy + 24, ipy + 18));
                    ipy += 36;
                } else {
                    res.push((gy, opy, ipy, ipy + 18, 0));
                    ipy += 30;
                }
                gy += 1;
                opy += 8;
            } else {
                if grid.reg_cfg == j && !is_7k70t {
                    res.push((0, 0, 0, 0, ipy));
                    ipy += 6;
                } else {
                    res.push((0, 0, 0, 0, 0));
                }
            }
        }
    }
    if is_7k70t {
        res[grids.first().unwrap().reg_cfg].4 = ipy + 6;
    }
    res
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Gt {
    pub col: ColId,
    pub row: RowId,
    pub reg_base: usize,
    pub slr: SlrId,
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
    pub fn get_pads(&self) -> Vec<(String, String, GtPin, u32)> {
        let mut res = Vec::new();
        let l = match self.kind {
            GtKind::Gtp => "P",
            GtKind::Gtx => "X",
            GtKind::Gth => "H",
        };
        for b in 0..4 {
            res.push((format!("OPAD_X{}Y{}", self.opx, self.opy + 2 * b), format!("MGT{l}TXN{}_{}", b, self.bank), GtPin::TxN, b));
            res.push((format!("OPAD_X{}Y{}", self.opx, self.opy + 2 * b + 1), format!("MGT{l}TXP{}_{}", b, self.bank), GtPin::TxP, b));
        }
        for b in 0..2 {
            res.push((format!("IPAD_X{}Y{}", self.ipx, self.ipy_l + 6 * b), format!("MGT{l}RXN{}_{}", b, self.bank), GtPin::RxN, b));
            res.push((format!("IPAD_X{}Y{}", self.ipx, self.ipy_l + 6 * b + 1), format!("MGT{l}RXP{}_{}", b, self.bank), GtPin::RxP, b));
        }
        for b in 2..4 {
            res.push((format!("IPAD_X{}Y{}", self.ipx, self.ipy_h + 6 * (b - 2)), format!("MGT{l}RXN{}_{}", b, self.bank), GtPin::RxN, b));
            res.push((format!("IPAD_X{}Y{}", self.ipx, self.ipy_h + 6 * (b - 2) + 1), format!("MGT{l}RXP{}_{}", b, self.bank), GtPin::RxP, b));
        }
        for b in 0..2 {
            res.push((format!("IPAD_X{}Y{}", self.ipx, self.ipy_l + 8 + 2 * b), format!("MGTREFCLK{}P_{}", b, self.bank), GtPin::ClkP, b));
            res.push((format!("IPAD_X{}Y{}", self.ipx, self.ipy_l + 9 + 2 * b), format!("MGTREFCLK{}N_{}", b, self.bank), GtPin::ClkN, b));
        }
        res
    }
}

pub fn get_gt(grids: &EntityVec<SlrId, Grid>, grid_master: SlrId, extras: &[ExtraDie], is_7k70t: bool) -> Vec<Gt> {
    let iopad_y = get_iopad_y(grids, extras, is_7k70t);
    let reg_cfg: usize = grids[grid_master].reg_cfg + grids.iter().filter(|&(k, _)| k < grid_master).map(|(_, x)| x.regs).sum::<usize>();
    let mut res = Vec::new();
    let mut reg_base = 0;
    let has_gtz = !extras.is_empty();
    for (slr, grid) in grids {
        let has_left_gt = grid.cols_gt[0].is_some();
        for gtc in 0..2 {
            let gx: u32 = if has_left_gt { gtc } else { 0 };
            let opx: u32 = if has_gtz { gtc * 2 } else if has_left_gt { gtc } else { 0 };
            let ipx: u32 = if has_gtz { gtc * 3 } else if has_left_gt { gtc * 2 } else if !is_7k70t { 1 } else { 0 };
            if let Some(ref col) = grid.cols_gt[gtc as usize] {
                for (j, &kind) in col.regs.iter().enumerate() {
                    if let Some(kind) = kind {
                        let reg = reg_base + j;
                        let bank = if kind == GtKind::Gtp {
                            if grid.has_ps {
                                112
                            } else if reg == 0 {
                                213
                            } else {
                                216
                            }
                        } else {
                            (15 + reg - reg_cfg + [200, 100][gtc as usize]) as u32
                        };
                        let (gy, opy, ipy_l, ipy_h, _) = iopad_y[reg as usize];
                        let row = RowId::from_idx(j * 50);
                        res.push(Gt {
                            col: col.col,
                            row,
                            slr,
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
        for hole in &grid.holes {
            let gtc: u32 = match hole.kind {
                HoleKind::GtpLeft => 0,
                HoleKind::GtpRight => 1,
                _ => continue,
            };
            let gx = gtc;
            let opx = gtc;
            let ipx = gtc + 1;
            let reg = reg_base + hole.row.to_idx() / 50;
            let bank = if reg == 0 {13} else {16} + [200, 100][gtc as usize];
            let (gy, opy, ipy_l, ipy_h, _) = iopad_y[reg];
            res.push(Gt {
                col: hole.col,
                row: hole.row,
                reg_base,
                slr,
                gx,
                gy,
                opx,
                opy,
                ipx,
                ipy_l,
                ipy_h,
                bank,
                kind: GtKind::Gtp,
            });
        }
        reg_base += grid.regs;
    }
    res
}

pub fn get_sysmon_pads(grids: &EntityVec<SlrId, Grid>, extras: &[ExtraDie], is_7k70t: bool) -> Vec<(String, u32, SysMonPin)> {
    let iopad_y = get_iopad_y(grids, extras, is_7k70t);
    let mut res = Vec::new();
    let mut reg_base = 0;
    for (i, grid) in grids {
        if grid.reg_cfg == grid.regs {
            continue;
        }
        let ipx = if grid.cols_gt[0].is_some() { 1 } else { 0 };
        let ipy = iopad_y[reg_base + grid.reg_cfg].4;
        res.push((format!("IPAD_X{}Y{}", ipx, ipy), i.to_idx() as u32, SysMonPin::VP));
        res.push((format!("IPAD_X{}Y{}", ipx, ipy+1), i.to_idx() as u32, SysMonPin::VN));
        reg_base += grid.regs;
    }
    res
}

pub fn get_gtz_pads(extras: &[ExtraDie]) -> Vec<(String, String, u32, GtPin, u32)> {
    let mut res = Vec::new();
    let has_gtz_bot = extras.contains(&ExtraDie::GtzBottom);
    let has_gtz_top = extras.contains(&ExtraDie::GtzTop);
    if has_gtz_bot {
        let ipy = 0;
        let opy = 0;
        for b in 0..8 {
            res.push((format!("IPAD_X2Y{}", ipy + 4 + 2 * b), format!("MGTZRXN{b}_400"), 400, GtPin::RxN, b));
            res.push((format!("IPAD_X2Y{}", ipy + 5 + 2 * b), format!("MGTZRXP{b}_400"), 400, GtPin::RxP, b));
            res.push((format!("OPAD_X1Y{}", opy + 2 * b), format!("MGTZTXN{b}_400"), 400, GtPin::TxN, b));
            res.push((format!("OPAD_X1Y{}", opy + 1 + 2 * b), format!("MGTZTXP{b}_400"), 400, GtPin::TxP, b));
        }
        for b in 0..2 {
            res.push((format!("IPAD_X2Y{}", ipy + 2 * b), format!("MGTZREFCLK{b}N_400"), 400, GtPin::ClkN, b));
            res.push((format!("IPAD_X2Y{}", ipy + 1 + 2 * b), format!("MGTZREFCLK{b}P_400"), 400, GtPin::ClkP, b));
        }
    }
    if has_gtz_top {
        let ipy = if has_gtz_bot {20} else {0};
        let opy = if has_gtz_bot {16} else {0};
        for b in 0..8 {
            res.push((format!("IPAD_X2Y{}", ipy + 4 + 2 * b), format!("MGTZRXN{b}_300"), 300, GtPin::RxN, b));
            res.push((format!("IPAD_X2Y{}", ipy + 5 + 2 * b), format!("MGTZRXP{b}_300"), 300, GtPin::RxP, b));
            res.push((format!("OPAD_X1Y{}", opy + 2 * b), format!("MGTZTXN{b}_300"), 300, GtPin::TxN, b));
            res.push((format!("OPAD_X1Y{}", opy + 1 + 2 * b), format!("MGTZTXP{b}_300"), 300, GtPin::TxP, b));
        }
        for b in 0..2 {
            res.push((format!("IPAD_X2Y{}", ipy + 2 * b), format!("MGTZREFCLK{b}N_300"), 300, GtPin::ClkN, b));
            res.push((format!("IPAD_X2Y{}", ipy + 1 + 2 * b), format!("MGTZREFCLK{b}P_300"), 300, GtPin::ClkP, b));
        }
    }
    res
}

pub fn get_ps_pads(grids: &EntityVec<SlrId, Grid>) -> Vec<(String, u32, PsPin)> {
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
            res.push((format!("IOPAD_X1Y{}", 77 + i), if i < 16 {500} else {501}, PsPin::Mio(i)));
        }
        res.push(("IOPAD_X1Y131".to_string(), 502, PsPin::DdrOdt(0)));
        res.push(("IOPAD_X1Y132".to_string(), 500, PsPin::PorB));
        res.push(("IOPAD_X1Y133".to_string(), 502, PsPin::DdrRasB));
        res.push(("IOPAD_X1Y134".to_string(), 501, PsPin::SrstB));
    }
    res
}

pub fn expand_grid<'a>(grids: &EntityVec<SlrId, &Grid>, grid_master: SlrId, extras: &[ExtraDie], db: &'a int::IntDb) -> eint::ExpandedGrid<'a> {
    let mut egrid = eint::ExpandedGrid::new(db);
    egrid.tie_kind = Some("TIEOFF".to_string());
    egrid.tie_pin_gnd = Some("HARD0".to_string());
    egrid.tie_pin_vcc = Some("HARD1".to_string());
    let mut yb = 0;
    let mut tie_yb = 0;
    if extras.iter().any(|&x| x == ExtraDie::GtzBottom) {
        yb += 1;
    }
    for (slrid, grid) in grids {
        egrid.tiles.push(Array2::default([grid.regs * 50, grid.columns.len()]));
        let mut slr = egrid.slr_mut(slrid);
        let mut x = 0;
        let mut tie_x = 0;
        let mut xlut = EntityVec::new();
        for (col, &kind) in &grid.columns {
            xlut.push(x);
            if grid.regs == 2 && grid.has_ps && col.to_idx() < 18 {
                continue;
            }
            if grid.regs <= 2 && col < grid.col_cfg && col >= grid.col_cfg - 6 {
                continue;
            }
            let lr = ['L', 'R'][col.to_idx() % 2];
            if lr == 'L' && kind == ColumnKind::Dsp {
                tie_x += 1;
            }
            for row in slr.rows() {
                let y = yb + row.to_idx();
                let tie_y = tie_yb + row.to_idx();
                slr.fill_tile((col, row), "INT", &format!("INT.{lr}"), format!("INT_{lr}_X{x}Y{y}"));
                slr[(col, row)].nodes[0].tie_name = Some(format!("TIEOFF_X{tie_x}Y{tie_y}"));
                match kind {
                    ColumnKind::ClbLL => (),
                    ColumnKind::ClbLM => (),
                    ColumnKind::Io => {
                        slr[(col, row)].add_intf(
                            db.get_intf("INTF"),
                            format!("IO_INT_INTERFACE_{lr}_X{x}Y{y}"),
                            db.get_intf_naming(&format!("INTF.{lr}")),
                        );
                    }
                    ColumnKind::Bram => {
                        slr[(col, row)].add_intf(
                            db.get_intf("INTF.BRAM"),
                            format!("BRAM_INT_INTERFACE_{lr}_X{x}Y{y}"),
                            db.get_intf_naming(&format!("INTF.{lr}")),
                        );
                    }
                    ColumnKind::Dsp | ColumnKind::Cmt | ColumnKind::Cfg | ColumnKind::Clk => {
                        slr[(col, row)].add_intf(
                            db.get_intf("INTF"),
                            format!("INT_INTERFACE_{lr}_X{x}Y{y}"),
                            db.get_intf_naming(&format!("INTF.{lr}")),
                        );
                    }
                    ColumnKind::Gt => (),
                }
            }
            x += 1;
            tie_x += 1;
            if lr == 'R' && kind == ColumnKind::Dsp {
                tie_x += 1;
            }
        }

        let row_cb = RowId::from_idx(grid.reg_cfg * 50 - 50);
        let row_ct = RowId::from_idx(grid.reg_cfg * 50 + 50);
        if grid.regs == 1 {
            slr.nuke_rect(grid.col_cfg - 6, row_cb, 6, 50);
        } else {
            slr.nuke_rect(grid.col_cfg - 6, row_cb, 6, 100);
            for dx in 0..6 {
                let col = grid.col_cfg - 6 + dx;
                if row_cb.to_idx() != 0 {
                    slr.fill_term_anon((col, row_cb - 1), "N");
                }
                if row_ct.to_idx() != grid.regs * 50 {
                    slr.fill_term_anon((col, row_ct), "S");
                }
            }
        }

        let col_l = slr.cols().next().unwrap();
        let col_r = slr.cols().next_back().unwrap();
        let row_b = slr.rows().next().unwrap();
        let row_t = slr.rows().next_back().unwrap();
        if grid.has_ps {
            slr.nuke_rect(grid.columns.first_id().unwrap(), row_t - 99, 18, 100);
            if grid.regs != 2 {
                let row = row_t - 100;
                for dx in 0..18 {
                    let col = col_l + dx;
                    slr.fill_term_anon((col, row), "N");
                }
            }
            let col = col_l + 18;
            for dy in 0..100 {
                let row = row_t - 99 + dy;
                slr.fill_term_anon((col, row), "W");
                let y = yb + row.to_idx();
                let x = xlut[col];
                slr[(col, row)].add_intf(
                    db.get_intf("INTF"),
                    format!("INT_INTERFACE_PSS_L_X{x}Y{y}"),
                    db.get_intf_naming("INTF.PSS"),
                );
            }
        }

        for hole in &grid.holes {
            match hole.kind {
                HoleKind::Pcie2Left | HoleKind::Pcie2Right => {
                    slr.nuke_rect(hole.col + 1, hole.row, 2, 25);
                    for dx in 1..3 {
                        let col = hole.col + dx;
                        if hole.row.to_idx() != 0 {
                            slr.fill_term_anon((col, hole.row - 1), "N");
                        }
                        slr.fill_term_anon((col, hole.row + 25), "S");
                    }
                    let col_l = hole.col;
                    let col_r = hole.col + 3;
                    let xl = xlut[col_l];
                    let xr = xlut[col_r];
                    for dy in 0..25 {
                        let row = hole.row + dy;
                        let y = yb + row.to_idx();
                        let tile_l = &mut slr[(col_l, row)];
                        tile_l.intfs.clear();
                        tile_l.add_intf(
                            db.get_intf("INTF.DELAY"),
                            format!("PCIE_INT_INTERFACE_R_X{xl}Y{y}"),
                            db.get_intf_naming("INTF.PCIE_R"),
                        );
                        let tile_r = &mut slr[(col_r, row)];
                        tile_r.intfs.clear();
                        if hole.kind == HoleKind::Pcie2Left {
                            tile_r.add_intf(
                                db.get_intf("INTF.DELAY"),
                                format!("PCIE_INT_INTERFACE_LEFT_L_X{xr}Y{y}"),
                                db.get_intf_naming("INTF.PCIE_LEFT_L"),
                            );
                        } else {
                            tile_r.add_intf(
                                db.get_intf("INTF.DELAY"),
                                format!("PCIE_INT_INTERFACE_L_X{xr}Y{y}"),
                                db.get_intf_naming("INTF.PCIE_L"),
                            );
                        }
                    }
                }
                HoleKind::Pcie3 => {
                    slr.nuke_rect(hole.col + 1, hole.row, 4, 50);
                    for dx in 1..5 {
                        let col = hole.col + dx;
                        slr.fill_term_anon((col, hole.row - 1), "N");
                        slr.fill_term_anon((col, hole.row + 50), "S");
                    }
                    let col_l = hole.col;
                    let col_r = hole.col + 5;
                    let xl = xlut[col_l];
                    let xr = xlut[col_r];
                    for dy in 0..50 {
                        let row = hole.row + dy;
                        let y = yb + row.to_idx();
                        let tile_l = &mut slr[(col_l, row)];
                        tile_l.intfs.clear();
                        tile_l.add_intf(
                            db.get_intf("INTF.DELAY"),
                            format!("PCIE3_INT_INTERFACE_R_X{xl}Y{y}"),
                            db.get_intf_naming("INTF.PCIE3_R"),
                        );
                        let tile_r = &mut slr[(col_r, row)];
                        tile_r.intfs.clear();
                        tile_r.add_intf(
                            db.get_intf("INTF.DELAY"),
                            format!("PCIE3_INT_INTERFACE_L_X{xr}Y{y}"),
                            db.get_intf_naming("INTF.PCIE3_L"),
                        );
                    }
                }
                HoleKind::GtpLeft => {
                    slr.nuke_rect(hole.col + 1, hole.row, 18, 50);
                    for dx in 1..19 {
                        let col = hole.col + dx;
                        if hole.row.to_idx() != 0 {
                            slr.fill_term_anon((col, hole.row - 1), "N");
                        }
                        if hole.row.to_idx() + 50 != grid.regs * 50 {
                            slr.fill_term_anon((col, hole.row + 50), "S");
                        }
                    }
                    let col_l = hole.col;
                    let col_r = hole.col + 19;
                    let xl = xlut[col_l];
                    for dy in 0..50 {
                        let row = hole.row + dy;
                        let y = yb + row.to_idx();
                        let tile = &mut slr[(col_l, row)];
                        tile.intfs.clear();
                        tile.add_intf(
                            db.get_intf("INTF.DELAY"),
                            format!("GTP_INT_INTERFACE_R_X{xl}Y{y}"),
                            db.get_intf_naming("INTF.GTP_R"),
                        );
                        slr.fill_term_anon((col_l, row), "E");
                        slr.fill_term_anon((col_r, row), "W");
                    }
                }
                HoleKind::GtpRight => {
                    slr.nuke_rect(hole.col - 18, hole.row, 18, 50);
                    for dx in 1..19 {
                        let col = hole.col - 19 + dx;
                        if hole.row.to_idx() != 0 {
                            slr.fill_term_anon((col, hole.row - 1), "N");
                        }
                        if hole.row.to_idx() + 50 != grid.regs * 50 {
                            slr.fill_term_anon((col, hole.row + 50), "S");
                        }
                    }
                    let col_l = hole.col - 19;
                    let col_r = hole.col;
                    let xr = xlut[col_r];
                    for dy in 0..50 {
                        let row = hole.row + dy;
                        let y = yb + row.to_idx();
                        let tile = &mut slr[(col_r, row)];
                        tile.intfs.clear();
                        tile.add_intf(
                            db.get_intf("INTF.DELAY"),
                            format!("GTP_INT_INTERFACE_L_X{xr}Y{y}"),
                            db.get_intf_naming("INTF.GTP_L"),
                        );
                        slr.fill_term_anon((col_l, row), "E");
                        slr.fill_term_anon((col_r, row), "W");
                    }
                }
            }
        }

        if let Some(ref gtcol) = grid.cols_gt[0] {
            for (reg, &kind) in gtcol.regs.iter().enumerate() {
                if let Some(kind) = kind {
                    let br = RowId::from_idx(reg * 50);
                    let x = xlut[gtcol.col];
                    for dy in 0..50 {
                        let row = br + dy;
                        let y = yb + row.to_idx();
                        let t = match kind {
                            GtKind::Gtp => unreachable!(),
                            GtKind::Gtx => "GTX",
                            GtKind::Gth => "GTH",
                        };
                        let tile = &mut slr[(gtcol.col, row)];
                        tile.intfs.clear();
                        tile.add_intf(
                            db.get_intf("INTF.DELAY"),
                            format!("{t}_INT_INTERFACE_L_X{x}Y{y}"),
                            db.get_intf_naming(&format!("INTF.{t}_L")),
                        );
                    }
                }
            }
        }

        if let Some(ref gtcol) = grid.cols_gt[1] {
            let need_holes = grid.columns[gtcol.col] != ColumnKind::Gt;
            for (reg, &kind) in gtcol.regs.iter().enumerate() {
                if let Some(kind) = kind {
                    let br = RowId::from_idx(reg * 50);
                    if need_holes {
                        slr.nuke_rect(gtcol.col + 1, br, 6, 50);
                        if reg != 0 && gtcol.regs[reg - 1].is_none() {
                            for dx in 1..7 {
                                slr.fill_term_anon((gtcol.col + dx, br - 1), "N");
                            }
                        }
                        if reg != grid.regs - 1 && gtcol.regs[reg + 1].is_none() {
                            for dx in 1..7 {
                                slr.fill_term_anon((gtcol.col + dx, br + 50), "S");
                            }
                        }
                        for dy in 0..50 {
                            slr.fill_term_anon((gtcol.col, br + dy), "E");
                        }
                    }
                    let x = xlut[gtcol.col];
                    for dy in 0..50 {
                        let row = br + dy;
                        let y = yb + row.to_idx();
                        let t = match kind {
                            GtKind::Gtp => "GTP",
                            GtKind::Gtx => "GTX",
                            GtKind::Gth => "GTH",
                        };
                        let tile = &mut slr[(gtcol.col, row)];
                        tile.intfs.clear();
                        tile.add_intf(
                            db.get_intf("INTF.DELAY"),
                            format!("{t}_INT_INTERFACE_X{x}Y{y}"),
                            db.get_intf_naming(&format!("INTF.{t}")),
                        );
                    }
                }
            }
        }

        for col in slr.cols() {
            if !slr[(col, row_b)].nodes.is_empty() {
                if grid.has_no_tbuturn {
                    slr.fill_term_anon((col, row_b), "S.HOLE");
                } else {
                    slr.fill_term_anon((col, row_b), "S");
                }
            }
            if !slr[(col, row_t)].nodes.is_empty() {
                if grid.has_no_tbuturn {
                    slr.fill_term_anon((col, row_t), "N.HOLE");
                } else {
                    slr.fill_term_anon((col, row_t), "N");
                }
            }
        }
        for row in slr.rows() {
            if !slr[(col_l, row)].nodes.is_empty() {
                slr.fill_term_anon((col_l, row), "W");
            }
            if !slr[(col_r, row)].nodes.is_empty() {
                slr.fill_term_anon((col_r, row), "E");
            }
        }
        for reg in 1..grid.regs {
            let row_s = RowId::from_idx(reg * 50 - 1);
            let row_n = RowId::from_idx(reg * 50);
            let term_s = db.get_term("BRKH.S");
            let term_n = db.get_term("BRKH.N");
            let naming_s = db.get_term_naming("BRKH.S");
            let naming_n = db.get_term_naming("BRKH.N");
            for col in slr.cols() {
                if !slr[(col, row_s)].nodes.is_empty() && !slr[(col, row_n)].nodes.is_empty() {
                    let x = xlut[col];
                    let y = yb + row_s.to_idx();
                    slr.fill_term_pair_buf((col, row_s), (col, row_n), term_n, term_s, format!("BRKH_INT_X{x}Y{y}"), naming_s, naming_n);
                }
            }
        }

        slr.fill_main_passes();

        yb += slr.rows().len();
        tie_yb += slr.rows().len();
    }

    let lvb6 = db.wires.iter().find_map(|(k, v)| if v.name == "LVB.6" {Some(k)} else {None}).unwrap();
    let mut slr_wires = HashMap::new();
    for i in 1..grids.len() {
        let slrid_s = SlrId::from_idx(i - 1);
        let slrid_n = SlrId::from_idx(i);
        let slr_s = egrid.slr(slrid_s);
        let slr_n = egrid.slr(slrid_n);
        for col in slr_s.cols() {
            for dy in 0..49 {
                let row_s = slr_s.rows().next_back().unwrap() - 49 + dy;
                let row_n = slr_n.rows().next().unwrap() + 1 + dy;
                if !slr_s[(col, row_s)].nodes.is_empty() && !slr_n[(col, row_n)].nodes.is_empty() {
                    slr_wires.insert((slrid_n, (col, row_n), lvb6), (slrid_s, (col, row_s), lvb6));
                }
            }
        }
    }
    egrid.slr_wires = slr_wires;

    egrid
}
