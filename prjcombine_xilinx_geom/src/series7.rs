use std::collections::BTreeSet;
use serde::{Serialize, Deserialize};
use super::{CfgPin, ExtraDie, SysMonPin, GtPin, PsPin, ColId};
use prjcombine_entity::EntityVec;

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
    pub rows: u32,
    pub row_cfg: u32,
    pub row_clk: u32,
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
    pub rows: Vec<Option<IoKind>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GtColumn {
    pub col: ColId,
    pub rows: Vec<Option<GtKind>>,
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
    pub row: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Io {
    pub col: ColId,
    pub row: u32,
    pub ioc: u32,
    pub iox: u32,
    pub bank: u32,
    pub kind: IoKind,
}

impl Io {
    pub fn iob_name(&self) -> String {
        let x = self.iox;
        let y = self.row;
        format!("IOB_X{x}Y{y}")
    }
    pub fn is_mrcc(&self) -> bool {
        matches!(self.row % 50, 23..=26)
    }
    pub fn is_srcc(&self) -> bool {
        matches!(self.row % 50, 21 | 22 | 27 | 28)
    }
    pub fn is_dqs(&self) -> bool {
        matches!(self.row % 50, 7 | 8 | 19 | 20 | 31 | 32 | 43 | 44)
    }
    pub fn is_vref(&self) -> bool {
        matches!(self.row % 50, 11 | 37)
    }
    pub fn is_vrp(&self) -> bool {
        self.kind == IoKind::Hpio && matches!(self.row % 50, 0)
    }
    pub fn is_vrn(&self) -> bool {
        self.kind == IoKind::Hpio && matches!(self.row % 50, 49)
    }
    pub fn get_cfg(&self, has_14: bool) -> Option<CfgPin> {
        match (self.bank, self.row % 50) {
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
            match (self.bank, self.row % 50) {
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
            match (self.bank, self.row % 50) {
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
            match (self.bank, self.row % 50) {
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

pub fn get_io(grids: &[Grid], grid_master: usize) -> Vec<Io> {
    let mut res = Vec::new();
    let row_cfg: u32 = grids[grid_master].row_cfg + grids[..grid_master].iter().map(|x| x.rows).sum::<u32>();
    for ioc in 0..2 {
        let iox = if grids[0].cols_io[0].is_none() {0} else {ioc};
        let mut row_base = 0;
        for grid in grids {
            if let Some(ref col) = grid.cols_io[ioc as usize] {
                for (j, &kind) in col.rows.iter().enumerate() {
                    if let Some(kind) = kind {
                        let row = row_base + j as u32;
                        let bank = 15 + row - row_cfg + ioc * 20;
                        for k in 0..50 {
                            res.push(Io {
                                col: col.col,
                                row: row * 50 + k,
                                ioc,
                                iox,
                                bank,
                                kind,
                            });
                        }
                    }
                }
            }
            row_base += grid.rows;
        }
    }
    res
}

fn get_iopad_y(grids: &[Grid], extras: &[ExtraDie], is_7k70t: bool) -> Vec<(u32, u32, u32, u32, u32)> {
    let mut res = Vec::new();
    let mut ipy = 0;
    let mut opy = 0;
    let mut gy = 0;
    if extras.contains(&ExtraDie::GtzBottom) {
        ipy += 6;
        opy += 2;
    }
    for grid in grids {
        for j in 0..grid.rows {
            let mut has_gt = false;
            if let Some(ref col) = grid.cols_gt[0] {
                has_gt |= col.rows[j as usize].is_some();
            }
            if let Some(ref col) = grid.cols_gt[1] {
                has_gt |= col.rows[j as usize].is_some();
            }
            for hole in &grid.holes {
                if matches!(hole.kind, HoleKind::GtpLeft | HoleKind::GtpRight) && hole.row == j * 50 {
                    has_gt = true;
                }
            }
            if has_gt {
                if grid.row_cfg == j && !is_7k70t {
                    res.push((gy, opy, ipy, ipy + 24, ipy + 18));
                    ipy += 36;
                } else {
                    res.push((gy, opy, ipy, ipy + 18, 0));
                    ipy += 30;
                }
                gy += 1;
                opy += 8;
            } else {
                if grid.row_cfg == j && !is_7k70t {
                    res.push((0, 0, 0, 0, ipy));
                    ipy += 6;
                } else {
                    res.push((0, 0, 0, 0, 0));
                }
            }
        }
    }
    if is_7k70t {
        res[grids[0].row_cfg as usize].4 = ipy + 6;
    }
    res
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Gt {
    pub col: ColId,
    pub row: u32,
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

pub fn get_gt(grids: &[Grid], grid_master: usize, extras: &[ExtraDie], is_7k70t: bool) -> Vec<Gt> {
    let iopad_y = get_iopad_y(grids, extras, is_7k70t);
    let row_cfg: u32 = grids[grid_master].row_cfg + grids[..grid_master].iter().map(|x| x.rows).sum::<u32>();
    let mut res = Vec::new();
    let mut row_base = 0;
    let has_gtz = !extras.is_empty();
    for grid in grids {
        let has_left_gt = grid.cols_gt[0].is_some();
        for gtc in 0..2 {
            let gx: u32 = if has_left_gt { gtc } else { 0 };
            let opx: u32 = if has_gtz { gtc * 2 } else if has_left_gt { gtc } else { 0 };
            let ipx: u32 = if has_gtz { gtc * 3 } else if has_left_gt { gtc * 2 } else if !is_7k70t { 1 } else { 0 };
            if let Some(ref col) = grid.cols_gt[gtc as usize] {
                for (j, &kind) in col.rows.iter().enumerate() {
                    if let Some(kind) = kind {
                        let row = row_base + j as u32;
                        let bank = if kind == GtKind::Gtp {
                            if grid.has_ps {
                                112
                            } else if row == 0 {
                                213
                            } else {
                                216
                            }
                        } else {
                            15 + row - row_cfg + [200, 100][gtc as usize]
                        };
                        let (gy, opy, ipy_l, ipy_h, _) = iopad_y[row as usize];
                        res.push(Gt {
                            col: col.col,
                            row: row * 50,
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
            let row = row_base + hole.row / 50;
            let bank = if row == 0 {13} else {16} + [200, 100][gtc as usize];
            let (gy, opy, ipy_l, ipy_h, _) = iopad_y[row as usize];
            res.push(Gt {
                col: hole.col,
                row: row * 50,
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
        row_base += grid.rows;
    }
    res
}

pub fn get_sysmon_pads(grids: &[Grid], extras: &[ExtraDie], is_7k70t: bool) -> Vec<(String, u32, SysMonPin)> {
    let iopad_y = get_iopad_y(grids, extras, is_7k70t);
    let mut res = Vec::new();
    let mut row_base = 0;
    for (i, grid) in grids.iter().enumerate() {
        if grid.row_cfg == grid.rows {
            continue;
        }
        let ipx = if grid.cols_gt[0].is_some() { 1 } else { 0 };
        let ipy = iopad_y[(row_base + grid.row_cfg) as usize].4;
        res.push((format!("IPAD_X{}Y{}", ipx, ipy), i as u32, SysMonPin::VP));
        res.push((format!("IPAD_X{}Y{}", ipx, ipy+1), i as u32, SysMonPin::VN));
        row_base += grid.rows;
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

pub fn get_ps_pads(grids: &[Grid]) -> Vec<(String, u32, PsPin)> {
    let mut res = Vec::new();
    if grids[0].has_ps {
        // XXX
        res.push((format!("IOPAD_X1Y1"), 502, PsPin::DdrWeB));
        res.push((format!("IOPAD_X1Y2"), 502, PsPin::DdrVrN));
        res.push((format!("IOPAD_X1Y3"), 502, PsPin::DdrVrP));
        for i in 0..13 {
            res.push((format!("IOPAD_X1Y{}", 4 + i), 502, PsPin::DdrA(i)));
        }
        res.push((format!("IOPAD_X1Y17"), 502, PsPin::DdrA(14)));
        res.push((format!("IOPAD_X1Y18"), 502, PsPin::DdrA(13)));
        for i in 0..3 {
            res.push((format!("IOPAD_X1Y{}", 19 + i), 502, PsPin::DdrBa(i)));
        }
        res.push((format!("IOPAD_X1Y22"), 502, PsPin::DdrCasB));
        res.push((format!("IOPAD_X1Y23"), 502, PsPin::DdrCke(0)));
        res.push((format!("IOPAD_X1Y24"), 502, PsPin::DdrCkN(0)));
        res.push((format!("IOPAD_X1Y25"), 502, PsPin::DdrCkP(0)));
        res.push((format!("IOPAD_X1Y26"), 500, PsPin::Clk));
        res.push((format!("IOPAD_X1Y27"), 502, PsPin::DdrCsB(0)));
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
        res.push((format!("IOPAD_X1Y72"), 502, PsPin::DdrDrstB));
        for i in 0..54 {
            res.push((format!("IOPAD_X1Y{}", 77 + i), if i < 16 {500} else {501}, PsPin::Mio(i)));
        }
        res.push((format!("IOPAD_X1Y131"), 502, PsPin::DdrOdt(0)));
        res.push((format!("IOPAD_X1Y132"), 500, PsPin::PorB));
        res.push((format!("IOPAD_X1Y133"), 502, PsPin::DdrRasB));
        res.push((format!("IOPAD_X1Y134"), 501, PsPin::SrstB));
    }
    res
}
