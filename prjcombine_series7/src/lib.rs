#![allow(clippy::collapsible_else_if)]

use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ColId, DieId, ExpandedGrid, Rect, RowId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

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
    pub die: DieId,
    pub reg_base: usize,
    pub ioc: u32,
    pub iox: u32,
    pub bank: u32,
    pub kind: IoKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    // ×32; high 16 bits are also low 16 bits of Addr
    // 0 doubles as MOSI
    // 1 doubles as DIN
    Data(u8),
    Addr(u8), // ×29 total, but 0-15 are represented as Data(16-31)
    CsiB,
    Dout, // doubles as CSO_B
    RdWrB,
    EmCclk,
    PudcB,
    Rs(u8), // ×2
    AdvB,
    FweB,
    FoeB,
    FcsB,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum CfgPin {
    Tck,
    Tdi,
    Tdo,
    Tms,
    ProgB,
    Done,
    M0,
    M1,
    M2,
    Cclk,
    InitB,
    CfgBvs,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtPin {
    RxP(u8),
    RxN(u8),
    TxP(u8),
    TxN(u8),
    ClkP(u8),
    ClkN(u8),
    RRef,
    AVttRCal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtRegionPin {
    AVtt,
    AVcc,
    VccAux,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtzPin {
    RxP(u8),
    RxN(u8),
    TxP(u8),
    TxN(u8),
    ClkP(u8),
    ClkN(u8),
    AGnd,
    AVcc,
    VccH,
    VccL,
    ObsClkP,
    ObsClkN,
    ThermIn,
    ThermOut,
    SenseAGnd,
    SenseGnd,
    SenseGndL,
    SenseAVcc,
    SenseVcc,
    SenseVccL,
    SenseVccH,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SysMonPin {
    VP,
    VN,
    AVdd,
    AVss,
    VRefP,
    VRefN,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PsPin {
    Mio(u32),
    Clk,
    PorB,
    SrstB,
    DdrDq(u32),
    DdrDm(u32),
    DdrDqsP(u32),
    DdrDqsN(u32),
    DdrA(u32),
    DdrBa(u32),
    DdrVrP,
    DdrVrN,
    DdrCkP(u32),
    DdrCkN(u32),
    DdrCke(u32),
    DdrOdt(u32),
    DdrDrstB,
    DdrCsB(u32),
    DdrRasB,
    DdrCasB,
    DdrWeB,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BondPin {
    // bank, pin within bank
    Io(u32, u32),
    Nc,
    Gnd,
    VccInt,
    VccAux,
    VccBram,
    VccO(u32),
    VccBatt,
    VccAuxIo(u32),
    RsvdGnd,
    Cfg(CfgPin),
    Gt(u32, GtPin),
    Gtz(u32, GtzPin),
    GtRegion(u32, GtRegionPin),
    Dxp,
    Dxn,
    SysMon(DieId, SysMonPin),
    VccPsInt,
    VccPsAux,
    VccPsPll,
    PsVref(u32, u32),
    PsIo(u32, PsPin),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DisabledPart {
    Gtp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ExtraDie {
    GtzTop,
    GtzBottom,
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
    let reg_cfg: usize = grids[grid_master].reg_cfg
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
                for (j, &kind) in col.regs.iter().enumerate() {
                    if let Some(kind) = kind {
                        let bank = (15 + reg_base + j - reg_cfg) as u32 + ioc * 20;
                        for k in 0..50 {
                            let row = RowId::from_idx(j * 50 + k);
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
        for j in 0..grid.regs {
            let mut has_gt = false;
            if let Some(ref col) = grid.cols_gt[0] {
                has_gt |= col.regs[j].is_some();
            }
            if let Some(ref col) = grid.cols_gt[1] {
                has_gt |= col.regs[j].is_some();
            }
            for hole in &grid.holes {
                if matches!(hole.kind, HoleKind::GtpLeft | HoleKind::GtpRight)
                    && hole.row == RowId::from_idx(j * 50)
                {
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
    let reg_cfg: usize = grids[grid_master].reg_cfg
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
            let bank = if reg == 0 { 13 } else { 16 } + [200, 100][gtc as usize];
            let (gy, opy, ipy_l, ipy_h, _) = iopad_y[reg];
            res.push(Gt {
                col: hole.col,
                row: hole.row,
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
                kind: GtKind::Gtp,
            });
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
        if grid.reg_cfg == grid.regs {
            continue;
        }
        let ipx = if grid.cols_gt[0].is_some() { 1 } else { 0 };
        let ipy = iopad_y[reg_base + grid.reg_cfg].4;
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

pub struct ExpandedDevice<'a> {
    pub grids: EntityVec<DieId, &'a Grid>,
    pub grid_master: DieId,
    pub egrid: ExpandedGrid<'a>,
    pub disabled: BTreeSet<DisabledPart>,
    pub extras: Vec<ExtraDie>,
}

pub fn expand_grid<'a>(
    grids: &EntityVec<DieId, &'a Grid>,
    grid_master: DieId,
    extras: &[ExtraDie],
    disabled: &BTreeSet<DisabledPart>,
    db: &'a IntDb,
) -> ExpandedDevice<'a> {
    let mut egrid = ExpandedGrid::new(db);
    egrid.tie_kind = Some("TIEOFF".to_string());
    egrid.tie_pin_gnd = Some("HARD0".to_string());
    egrid.tie_pin_vcc = Some("HARD1".to_string());
    let mut yb = 0;
    let mut ryb = 0;
    let mut syb = 0;
    let mut tie_yb = 0;
    let mut pyb = 0;
    if extras.iter().any(|&x| x == ExtraDie::GtzBottom) {
        yb = 1;
        ryb = 2;
    }
    for &grid in grids.values() {
        let (_, mut die) = egrid.add_die(grid.columns.len(), grid.regs * 50);
        let mut x = 0;
        let mut tie_x = 0;
        let mut xlut = EntityVec::new();
        let mut tiexlut = EntityVec::new();
        let mut rxlut = EntityVec::new();
        let mut rx = 0;
        for (col, &kind) in &grid.columns {
            xlut.push(x);
            if grid.has_ps && grid.regs == 2 && col.to_idx() == 18 {
                rx -= 19;
            }
            if grid.cols_vbrk.contains(&col) {
                rx += 1;
            }
            if kind == ColumnKind::Bram && col.to_idx() == 0 {
                rx += 1;
            }
            rxlut.push(rx);
            match kind {
                ColumnKind::ClbLL | ColumnKind::ClbLM => rx += 2,
                ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::Clk | ColumnKind::Cfg => rx += 3,
                ColumnKind::Io => {
                    if col == die.cols().next().unwrap() || col == die.cols().next_back().unwrap() {
                        rx += 5;
                    } else {
                        rx += 4;
                    }
                }
                ColumnKind::Gt | ColumnKind::Cmt => rx += 4,
            }
            if grid.regs == 2 && grid.has_ps && col.to_idx() < 18 {
                tiexlut.push(tie_x);
                continue;
            }
            if grid.regs <= 2 && col < grid.col_cfg && col >= grid.col_cfg - 6 {
                tiexlut.push(tie_x);
                continue;
            }
            let lr = ['L', 'R'][col.to_idx() % 2];
            if lr == 'L' && kind == ColumnKind::Dsp {
                tie_x += 1;
            }
            tiexlut.push(tie_x);
            for row in die.rows() {
                let y = yb + row.to_idx();
                let tie_y = tie_yb + row.to_idx();
                die.fill_tile(
                    (col, row),
                    "INT",
                    &format!("INT.{lr}"),
                    format!("INT_{lr}_X{x}Y{y}"),
                );
                die[(col, row)].nodes[0].tie_name = Some(format!("TIEOFF_X{tie_x}Y{tie_y}"));
                match kind {
                    ColumnKind::ClbLL => (),
                    ColumnKind::ClbLM => (),
                    ColumnKind::Io => {
                        die[(col, row)].add_intf(
                            db.get_intf("INTF"),
                            format!("IO_INT_INTERFACE_{lr}_X{x}Y{y}"),
                            db.get_intf_naming(&format!("INTF.{lr}")),
                        );
                    }
                    ColumnKind::Bram => {
                        die[(col, row)].add_intf(
                            db.get_intf("INTF.BRAM"),
                            format!("BRAM_INT_INTERFACE_{lr}_X{x}Y{y}"),
                            db.get_intf_naming(&format!("INTF.{lr}")),
                        );
                    }
                    ColumnKind::Dsp | ColumnKind::Cmt | ColumnKind::Cfg | ColumnKind::Clk => {
                        die[(col, row)].add_intf(
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

        let mut holes = Vec::new();

        let row_cb = RowId::from_idx(grid.reg_cfg * 50 - 50);
        let row_ct = RowId::from_idx(grid.reg_cfg * 50 + 50);
        if grid.regs == 1 {
            die.nuke_rect(grid.col_cfg - 6, row_cb, 6, 50);
            holes.push(Rect {
                col_l: grid.col_cfg - 6,
                col_r: grid.col_cfg,
                row_b: row_cb,
                row_t: row_cb + 50,
            });
        } else {
            die.nuke_rect(grid.col_cfg - 6, row_cb, 6, 100);
            holes.push(Rect {
                col_l: grid.col_cfg - 6,
                col_r: grid.col_cfg,
                row_b: row_cb,
                row_t: row_cb + 100,
            });
            for dx in 0..6 {
                let col = grid.col_cfg - 6 + dx;
                if row_cb.to_idx() != 0 {
                    die.fill_term_anon((col, row_cb - 1), "TERM.N");
                }
                if row_ct.to_idx() != grid.regs * 50 {
                    die.fill_term_anon((col, row_ct), "TERM.S");
                }
            }
        }

        let col_l = die.cols().next().unwrap();
        let col_r = die.cols().next_back().unwrap();
        let row_b = die.rows().next().unwrap();
        let row_t = die.rows().next_back().unwrap();
        if grid.has_ps {
            die.nuke_rect(col_l, row_t - 99, 18, 100);
            holes.push(Rect {
                col_l,
                col_r: col_l + 19,
                row_b: row_t - 99,
                row_t: row_t + 1,
            });
            if grid.regs != 2 {
                let row = row_t - 100;
                for dx in 0..18 {
                    let col = col_l + dx;
                    die.fill_term_anon((col, row), "TERM.N");
                }
            }
            let col = col_l + 18;
            for dy in 0..100 {
                let row = row_t - 99 + dy;
                die.fill_term_anon((col, row), "TERM.W");
                let y = yb + row.to_idx();
                let x = xlut[col];
                die[(col, row)].add_intf(
                    db.get_intf("INTF"),
                    format!("INT_INTERFACE_PSS_L_X{x}Y{y}"),
                    db.get_intf_naming("INTF.PSS"),
                );
            }
        }

        let has_pcie2_left = grid.holes.iter().any(|x| x.kind == HoleKind::Pcie2Left);
        let mut ply = 0;
        let mut pry = 0;
        for hole in &grid.holes {
            match hole.kind {
                HoleKind::Pcie2Left | HoleKind::Pcie2Right => {
                    die.nuke_rect(hole.col + 1, hole.row, 2, 25);
                    holes.push(Rect {
                        col_l: hole.col,
                        col_r: hole.col + 4,
                        row_b: hole.row,
                        row_t: hole.row + 25,
                    });
                    for dx in 1..3 {
                        let col = hole.col + dx;
                        if hole.row.to_idx() != 0 {
                            die.fill_term_anon((col, hole.row - 1), "TERM.N");
                        }
                        die.fill_term_anon((col, hole.row + 25), "TERM.S");
                    }
                    let col_l = hole.col;
                    let col_r = hole.col + 3;
                    let xl = xlut[col_l];
                    let xr = xlut[col_r];
                    for dy in 0..25 {
                        let row = hole.row + dy;
                        let y = yb + row.to_idx();
                        let tile_l = &mut die[(col_l, row)];
                        tile_l.intfs.clear();
                        tile_l.add_intf(
                            db.get_intf("INTF.DELAY"),
                            format!("PCIE_INT_INTERFACE_R_X{xl}Y{y}"),
                            db.get_intf_naming("INTF.PCIE_R"),
                        );
                        let tile_r = &mut die[(col_r, row)];
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
                    if disabled.contains(&DisabledPart::Gtp) {
                        continue;
                    }
                    let mut crds = vec![];
                    for dy in 0..25 {
                        crds.push((hole.col, hole.row + dy));
                    }
                    for dy in 0..25 {
                        crds.push((hole.col + 3, hole.row + dy));
                    }
                    let kind;
                    let tkb;
                    let tkt;
                    let sx;
                    let sy;
                    match hole.kind {
                        HoleKind::Pcie2Left => {
                            tkb = "PCIE_BOT_LEFT";
                            tkt = "PCIE_TOP_LEFT";
                            kind = "PCIE_L";
                            sy = pyb + ply;
                            ply += 1;
                            sx = 0;
                        }
                        HoleKind::Pcie2Right => {
                            tkb = "PCIE_BOT";
                            tkt = "PCIE_TOP";
                            kind = "PCIE_R";
                            sy = pyb + pry;
                            pry += 1;
                            sx = if has_pcie2_left { 1 } else { 0 };
                        }
                        _ => unreachable!(),
                    }
                    let x = rxlut[hole.col] + 2;
                    let y = ryb + hole.row.to_idx() + hole.row.to_idx() / 25 + 1;
                    let name_b = format!("{tkb}_X{x}Y{y}", y = y + 10);
                    let name_t = format!("{tkt}_X{x}Y{y}", y = y + 20);
                    let node = die[crds[0]].add_xnode(
                        db.get_node(kind),
                        &[&name_b, &name_t],
                        db.get_node_naming(kind),
                        &crds,
                    );
                    node.add_bel(0, format!("PCIE_X{sx}Y{sy}"));
                }
                HoleKind::Pcie3 => {
                    die.nuke_rect(hole.col + 1, hole.row, 4, 50);
                    holes.push(Rect {
                        col_l: hole.col,
                        col_r: hole.col + 6,
                        row_b: hole.row,
                        row_t: hole.row + 50,
                    });
                    for dx in 1..5 {
                        let col = hole.col + dx;
                        die.fill_term_anon((col, hole.row - 1), "TERM.N");
                        die.fill_term_anon((col, hole.row + 50), "TERM.S");
                    }
                    let col_l = hole.col;
                    let col_r = hole.col + 5;
                    let xl = xlut[col_l];
                    let xr = xlut[col_r];
                    for dy in 0..50 {
                        let row = hole.row + dy;
                        let y = yb + row.to_idx();
                        let tile_l = &mut die[(col_l, row)];
                        tile_l.intfs.clear();
                        tile_l.add_intf(
                            db.get_intf("INTF.DELAY"),
                            format!("PCIE3_INT_INTERFACE_R_X{xl}Y{y}"),
                            db.get_intf_naming("INTF.PCIE3_R"),
                        );
                        let tile_r = &mut die[(col_r, row)];
                        tile_r.intfs.clear();
                        tile_r.add_intf(
                            db.get_intf("INTF.DELAY"),
                            format!("PCIE3_INT_INTERFACE_L_X{xr}Y{y}"),
                            db.get_intf_naming("INTF.PCIE3_L"),
                        );
                    }
                    let mut crds = vec![];
                    for dy in 0..50 {
                        crds.push((hole.col, hole.row + dy));
                    }
                    for dy in 0..50 {
                        crds.push((hole.col + 5, hole.row + dy));
                    }
                    let x = rxlut[hole.col] + 2;
                    let y = ryb + hole.row.to_idx() + hole.row.to_idx() / 25 + 1;
                    let name_b = format!("PCIE3_BOT_RIGHT_X{x}Y{y}", y = y + 7);
                    let name = format!("PCIE3_RIGHT_X{x}Y{y}", y = y + 26);
                    let name_t = format!("PCIE3_TOP_RIGHT_X{x}Y{y}", y = y + 43);
                    let node = die[crds[0]].add_xnode(
                        db.get_node("PCIE3"),
                        &[&name, &name_b, &name_t],
                        db.get_node_naming("PCIE3"),
                        &crds,
                    );
                    node.add_bel(0, format!("PCIE3_X0Y{sy}", sy = pyb + pry));
                    pry += 1;
                }
                HoleKind::GtpLeft => {
                    die.nuke_rect(hole.col + 1, hole.row, 18, 50);
                    holes.push(Rect {
                        col_l: hole.col,
                        col_r: hole.col + 19,
                        row_b: hole.row,
                        row_t: hole.row + 50,
                    });
                    for dx in 1..19 {
                        let col = hole.col + dx;
                        if hole.row.to_idx() != 0 {
                            die.fill_term_anon((col, hole.row - 1), "TERM.N");
                        }
                        if hole.row.to_idx() + 50 != grid.regs * 50 {
                            die.fill_term_anon((col, hole.row + 50), "TERM.S");
                        }
                    }
                    let col_l = hole.col;
                    let col_r = hole.col + 19;
                    let xl = xlut[col_l];
                    for dy in 0..50 {
                        let row = hole.row + dy;
                        let y = yb + row.to_idx();
                        let tile = &mut die[(col_l, row)];
                        tile.intfs.clear();
                        tile.add_intf(
                            db.get_intf("INTF.DELAY"),
                            format!("GTP_INT_INTERFACE_R_X{xl}Y{y}"),
                            db.get_intf_naming("INTF.GTP_R"),
                        );
                        die.fill_term_anon((col_l, row), "TERM.E");
                        die.fill_term_anon((col_r, row), "TERM.W");
                    }
                }
                HoleKind::GtpRight => {
                    die.nuke_rect(hole.col - 18, hole.row, 18, 50);
                    holes.push(Rect {
                        col_l: hole.col - 18,
                        col_r: hole.col + 1,
                        row_b: hole.row,
                        row_t: hole.row + 50,
                    });
                    for dx in 1..19 {
                        let col = hole.col - 19 + dx;
                        if hole.row.to_idx() != 0 {
                            die.fill_term_anon((col, hole.row - 1), "TERM.N");
                        }
                        if hole.row.to_idx() + 50 != grid.regs * 50 {
                            die.fill_term_anon((col, hole.row + 50), "TERM.S");
                        }
                    }
                    let col_l = hole.col - 19;
                    let col_r = hole.col;
                    let xr = xlut[col_r];
                    for dy in 0..50 {
                        let row = hole.row + dy;
                        let y = yb + row.to_idx();
                        let tile = &mut die[(col_r, row)];
                        tile.intfs.clear();
                        tile.add_intf(
                            db.get_intf("INTF.DELAY"),
                            format!("GTP_INT_INTERFACE_L_X{xr}Y{y}"),
                            db.get_intf_naming("INTF.GTP_L"),
                        );
                        die.fill_term_anon((col_l, row), "TERM.E");
                        die.fill_term_anon((col_r, row), "TERM.W");
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
                        let tile = &mut die[(gtcol.col, row)];
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
                        die.nuke_rect(gtcol.col + 1, br, 6, 50);
                        holes.push(Rect {
                            col_l: gtcol.col,
                            col_r: gtcol.col + 7,
                            row_b: br,
                            row_t: br + 50,
                        });
                        if reg != 0 && gtcol.regs[reg - 1].is_none() {
                            for dx in 1..7 {
                                die.fill_term_anon((gtcol.col + dx, br - 1), "TERM.N");
                            }
                        }
                        if reg != grid.regs - 1 && gtcol.regs[reg + 1].is_none() {
                            for dx in 1..7 {
                                die.fill_term_anon((gtcol.col + dx, br + 50), "TERM.S");
                            }
                        }
                        for dy in 0..50 {
                            die.fill_term_anon((gtcol.col, br + dy), "TERM.E");
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
                        let tile = &mut die[(gtcol.col, row)];
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

        for col in die.cols() {
            if !die[(col, row_b)].nodes.is_empty() {
                if grid.has_no_tbuturn {
                    die.fill_term_anon((col, row_b), "TERM.S.HOLE");
                } else {
                    die.fill_term_anon((col, row_b), "TERM.S");
                }
            }
            if !die[(col, row_t)].nodes.is_empty() {
                if grid.has_no_tbuturn {
                    die.fill_term_anon((col, row_t), "TERM.N.HOLE");
                } else {
                    die.fill_term_anon((col, row_t), "TERM.N");
                }
            }
        }
        for row in die.rows() {
            if !die[(col_l, row)].nodes.is_empty() {
                die.fill_term_anon((col_l, row), "TERM.W");
            }
            if !die[(col_r, row)].nodes.is_empty() {
                die.fill_term_anon((col_r, row), "TERM.E");
            }
        }
        for reg in 1..grid.regs {
            let row_s = RowId::from_idx(reg * 50 - 1);
            let row_n = RowId::from_idx(reg * 50);
            let term_s = db.get_term("BRKH.S");
            let term_n = db.get_term("BRKH.N");
            let naming_s = db.get_term_naming("BRKH.S");
            let naming_n = db.get_term_naming("BRKH.N");
            for col in die.cols() {
                if !die[(col, row_s)].nodes.is_empty() && !die[(col, row_n)].nodes.is_empty() {
                    let x = xlut[col];
                    let y = yb + row_s.to_idx();
                    die.fill_term_pair_buf(
                        (col, row_s),
                        (col, row_n),
                        term_n,
                        term_s,
                        format!("BRKH_INT_X{x}Y{y}"),
                        naming_s,
                        naming_n,
                    );
                }
            }
        }

        die.fill_main_passes();

        let mut sx = 0;
        for (col, &cd) in &grid.columns {
            let (kind, naming) = match (cd, col.to_idx() % 2) {
                (ColumnKind::ClbLL, 0) => ("CLBLL", "CLBLL_L"),
                (ColumnKind::ClbLL, 1) => ("CLBLL", "CLBLL_R"),
                (ColumnKind::ClbLM, 0) => ("CLBLM", "CLBLM_L"),
                (ColumnKind::ClbLM, 1) => ("CLBLM", "CLBLM_R"),
                _ => continue,
            };
            let mut found = false;
            'a: for row in die.rows() {
                let tile = &mut die[(col, row)];
                for &hole in &holes {
                    if col >= hole.col_l
                        && col < hole.col_r
                        && row >= hole.row_b
                        && row < hole.row_t
                    {
                        continue 'a;
                    }
                }
                let x = xlut[col];
                let y = yb + row.to_idx();
                let sy = syb + row.to_idx();
                let name = format!("{naming}_X{x}Y{y}");
                let node = tile.add_xnode(
                    db.get_node(kind),
                    &[&name],
                    db.get_node_naming(naming),
                    &[(col, row)],
                );
                node.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                node.add_bel(1, format!("SLICE_X{sx}Y{sy}", sx = sx + 1));
                found = true;
            }
            if found {
                sx += 2;
            }
        }

        let mut bx = 0;
        let mut dx = 0;
        for (col, &cd) in &grid.columns {
            let (kind, naming) = match cd {
                ColumnKind::Bram => ("BRAM", ["BRAM_L", "BRAM_R"][col.to_idx() % 2]),
                ColumnKind::Dsp => ("DSP", ["DSP_L", "DSP_R"][col.to_idx() % 2]),
                _ => continue,
            };
            let mut found = false;
            'a: for row in die.rows() {
                if row.to_idx() % 5 != 0 {
                    continue;
                }
                for &hole in &holes {
                    if col >= hole.col_l
                        && col < hole.col_r
                        && row >= hole.row_b
                        && row < hole.row_t
                    {
                        continue 'a;
                    }
                }
                if col.to_idx() == 0 && (row.to_idx() < 5 || row.to_idx() >= die.rows().len() - 5) {
                    continue;
                }
                found = true;
                let x = xlut[col];
                let y = yb + row.to_idx();
                let sy = (syb + row.to_idx()) / 5;
                let name = format!("{naming}_X{x}Y{y}");
                let node = die[(col, row)].add_xnode(
                    db.get_node(kind),
                    &[&name],
                    db.get_node_naming(naming),
                    &[
                        (col, row),
                        (col, row + 1),
                        (col, row + 2),
                        (col, row + 3),
                        (col, row + 4),
                    ],
                );
                if cd == ColumnKind::Bram {
                    node.add_bel(0, format!("RAMB36_X{bx}Y{sy}", sy = sy));
                    node.add_bel(1, format!("RAMB18_X{bx}Y{sy}", sy = sy * 2));
                    node.add_bel(2, format!("RAMB18_X{bx}Y{sy}", sy = sy * 2 + 1));
                } else {
                    node.add_bel(0, format!("DSP48_X{dx}Y{sy}", sy = sy * 2));
                    node.add_bel(1, format!("DSP48_X{dx}Y{sy}", sy = sy * 2 + 1));
                    let tx = if naming == "DSP_L" {
                        tiexlut[col] - 1
                    } else {
                        tiexlut[col] + 1
                    };
                    let ty = tie_yb + row.to_idx();
                    node.add_bel(2, format!("TIEOFF_X{tx}Y{ty}"));
                }
                if kind == "BRAM" && row.to_idx() % 50 == 25 {
                    let hx = if naming == "BRAM_L" {
                        rxlut[col]
                    } else {
                        rxlut[col] + 2
                    };
                    let hy = ryb + row.to_idx() + row.to_idx() / 25;
                    let name_h = format!("HCLK_BRAM_X{hx}Y{hy}");
                    let name_1 = format!("{naming}_X{x}Y{y}", y = y + 5);
                    let name_2 = format!("{naming}_X{x}Y{y}", y = y + 10);
                    let coords: Vec<_> = (0..15).map(|dy| (col, row + dy)).collect();
                    let node = die[(col, row)].add_xnode(
                        db.get_node("PMVBRAM"),
                        &[&name_h, &name, &name_1, &name_2],
                        db.get_node_naming("PMVBRAM"),
                        &coords,
                    );
                    node.add_bel(0, format!("PMVBRAM_X{bx}Y{sy}", sy = sy / 10));
                }
            }
            if found {
                if cd == ColumnKind::Bram {
                    bx += 1;
                } else {
                    dx += 1;
                }
            }
        }

        yb += die.rows().len();
        syb += die.rows().len();
        ryb += die.rows().len() + grid.regs * 2 + 1;
        tie_yb += die.rows().len();
        pyb += pry;
    }

    let lvb6 = db
        .wires
        .iter()
        .find_map(|(k, v)| if v.name == "LVB.6" { Some(k) } else { None })
        .unwrap();
    let mut xdie_wires = HashMap::new();
    for i in 1..grids.len() {
        let dieid_s = DieId::from_idx(i - 1);
        let dieid_n = DieId::from_idx(i);
        let die_s = egrid.die(dieid_s);
        let die_n = egrid.die(dieid_n);
        for col in die_s.cols() {
            for dy in 0..49 {
                let row_s = die_s.rows().next_back().unwrap() - 49 + dy;
                let row_n = die_n.rows().next().unwrap() + 1 + dy;
                if !die_s[(col, row_s)].nodes.is_empty() && !die_n[(col, row_n)].nodes.is_empty() {
                    xdie_wires.insert((dieid_n, (col, row_n), lvb6), (dieid_s, (col, row_s), lvb6));
                }
            }
        }
    }
    egrid.xdie_wires = xdie_wires;

    ExpandedDevice {
        grids: grids.clone(),
        grid_master,
        egrid,
        extras: extras.to_vec(),
        disabled: disabled.clone(),
    }
}

impl<'a> ExpandedDevice<'a> {
    pub fn adjust_ise(&mut self) {
        for (die, &grid) in &self.grids {
            if grid.has_no_tbuturn {
                let (w, _) = self
                    .egrid
                    .db
                    .wires
                    .iter()
                    .find(|(_, w)| w.name == "LVB.6")
                    .unwrap();
                for col in grid.columns.ids() {
                    for i in 0..6 {
                        let row = RowId::from_idx(i);
                        self.egrid.blackhole_wires.insert((die, (col, row), w));
                    }
                    for i in 0..6 {
                        let row = RowId::from_idx(grid.regs * 50 - 6 + i);
                        self.egrid.blackhole_wires.insert((die, (col, row), w));
                    }
                }
            }
        }
    }

    pub fn adjust_vivado(&mut self) {
        let lvb6 = self
            .egrid
            .db
            .wires
            .iter()
            .find_map(|(k, v)| if v.name == "LVB.6" { Some(k) } else { None })
            .unwrap();
        let mut cursed_wires = HashSet::new();
        for i in 1..self.grids.len() {
            let dieid_s = DieId::from_idx(i - 1);
            let dieid_n = DieId::from_idx(i);
            let die_s = self.egrid.die(dieid_s);
            let die_n = self.egrid.die(dieid_n);
            for col in die_s.cols() {
                let row_s = die_s.rows().next_back().unwrap() - 49;
                let row_n = die_n.rows().next().unwrap() + 1;
                if !die_s[(col, row_s)].nodes.is_empty() && !die_n[(col, row_n)].nodes.is_empty() {
                    cursed_wires.insert((dieid_s, (col, row_s), lvb6));
                }
            }
        }
        self.egrid.cursed_wires = cursed_wires;
    }
}
