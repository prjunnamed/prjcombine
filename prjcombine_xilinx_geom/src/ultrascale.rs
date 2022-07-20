use std::collections::BTreeSet;
use serde::{Serialize, Deserialize};
use crate::{CfgPin, DisabledPart, ColId, RowId, SlrId, int, eint};
use ndarray::Array2;
use prjcombine_entity::{EntityVec, EntityId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GridKind {
    Ultrascale,
    UltrascalePlus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ColSide {
    Left,
    Right,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub kind: GridKind,
    pub columns: EntityVec<ColId, Column>,
    pub cols_vbrk: BTreeSet<ColId>,
    pub cols_fsr_gap: BTreeSet<ColId>,
    pub col_cfg: HardColumn,
    pub col_hard: Option<HardColumn>,
    pub cols_io: Vec<IoColumn>,
    pub regs: usize,
    pub ps: Option<Ps>,
    pub has_hbm: bool,
    pub is_dmc: bool,
    pub is_alt_cfg: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKindLeft {
    CleL,
    CleM,
    CleMClkBuf,
    CleMLaguna,
    Bram,
    BramAuxClmp,
    BramBramClmp,
    BramTd,
    Uram,
    Hard,
    Io,
    Gt,
    Sdfec,
    DfeC,
    DfeDF,
    DfeE,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKindRight {
    CleL,
    CleLDcg10,
    Dsp,
    DspClkBuf,
    Uram,
    Hard,
    Io,
    Gt,
    DfeB,
    DfeC,
    DfeDF,
    DfeE,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub l: ColumnKindLeft,
    pub r: ColumnKindRight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HardRowKind {
    None,
    Cfg,
    Ams,
    Hdio,
    HdioAms,
    Pcie,
    PciePlus,
    Cmac,
    Ilkn,
    DfeA,
    DfeG,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HardColumn {
    pub col: ColId,
    pub regs: Vec<HardRowKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum IoRowKind {
    None,
    Hpio,
    Hrio,
    Gth,
    Gty,
    Gtm,
    Gtf,
    HsAdc,
    HsDac,
    RfAdc,
    RfDac,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IoColumn {
    pub col: ColId,
    pub side: ColSide,
    pub regs: Vec<IoRowKind>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Ps {
    pub col: ColId,
    pub has_vcu: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IoKind {
    Hpio,
    Hrio,
    Hdio,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Io {
    pub col: ColId,
    pub side: Option<ColSide>,
    pub reg: u32,
    pub bel: u32,
    pub iox: u32,
    pub ioy: u32,
    pub bank: u32,
    pub kind: IoKind,
    pub grid_kind: GridKind,
    pub is_hdio_ams: bool,
    pub is_alt_cfg: bool,
}

impl Io {
    pub fn iob_name(&self) -> String {
        let x = self.iox;
        let y = self.ioy;
        format!("IOB_X{x}Y{y}")
    }
    pub fn is_vrp(&self) -> bool {
        self.kind == IoKind::Hpio && self.bel == 12
    }
    pub fn is_dbc(&self) -> bool {
        self.kind != IoKind::Hdio && matches!(self.bel, 0 | 1 | 6 | 7 | 39 | 40 | 45 | 46)
    }
    pub fn is_qbc(&self) -> bool {
        self.kind != IoKind::Hdio && matches!(self.bel, 13 | 14 | 19 | 20 | 26 | 27 | 32 | 33)
    }
    pub fn is_gc(&self) -> bool {
        if self.kind == IoKind::Hdio {
            matches!(self.bel, 8..=15)
        } else {
            matches!(self.bel, 21 | 22 | 23 | 24 | 26 | 27 | 28 | 29)
        }
    }
    pub fn sm_pair(&self) -> Option<u32> {
        if self.kind == IoKind::Hdio {
            if self.is_hdio_ams {
                match self.bel {
                    0 | 1 => Some(11),
                    2 | 3 => Some(10),
                    4 | 5 => Some(9),
                    6 | 7 => Some(8),
                    8 | 9 => Some(7),
                    10 | 11 => Some(6),
                    12 | 13 => Some(5),
                    14 | 15 => Some(4),
                    16 | 17 => Some(3),
                    18 | 19 => Some(2),
                    20 | 21 => Some(1),
                    22 | 23 => Some(0),
                    _ => None,
                }
            } else {
                match self.bel {
                    0 | 1 => Some(15),
                    2 | 3 => Some(14),
                    4 | 5 => Some(13),
                    6 | 7 => Some(12),
                    16 | 17 => Some(11),
                    18 | 19 => Some(10),
                    20 | 21 => Some(9),
                    22 | 23 => Some(8),
                    _ => None,
                }
            }
        } else {
            match self.bel {
                4 | 5 => Some(15),
                6 | 7 => Some(7),
                8 | 9 => Some(14),
                10 | 11 => Some(6),
                13 | 14 => Some(13),
                15 | 16 => Some(5),
                17 | 18 => Some(12),
                19 | 20 => Some(4),
                30 | 31 => Some(11),
                32 | 33 => Some(3),
                34 | 35 => Some(10),
                36 | 37 => Some(2),
                39 | 40 => Some(9),
                41 | 42 => Some(1),
                43 | 44 => Some(8),
                45 | 46 => Some(0),
                _ => None,
            }
        }
    }
    pub fn get_cfg(&self) -> Option<CfgPin> {
        if !self.is_alt_cfg {
            match (self.bank, self.bel) {
                (65, 0) => Some(CfgPin::Rs(0)),
                (65, 1) => Some(CfgPin::Rs(1)),
                (65, 2) => Some(CfgPin::FoeB),
                (65, 3) => Some(CfgPin::FweB),
                (65, 4) => Some(CfgPin::Addr(26)),
                (65, 5) => Some(CfgPin::Addr(27)),
                (65, 6) => Some(CfgPin::Addr(24)),
                (65, 7) => Some(CfgPin::Addr(25)),
                (65, 8) => Some(CfgPin::Addr(22)),
                (65, 9) => Some(CfgPin::Addr(23)),
                (65, 10) => Some(CfgPin::Addr(20)),
                (65, 11) => Some(CfgPin::Addr(21)),
                (65, 12) => Some(CfgPin::Addr(28)),
                (65, 13) => Some(CfgPin::Addr(18)),
                (65, 14) => Some(CfgPin::Addr(19)),
                (65, 15) => Some(CfgPin::Addr(16)),
                (65, 16) => Some(CfgPin::Addr(17)),
                (65, 17) => Some(CfgPin::Data(30)),
                (65, 18) => Some(CfgPin::Data(31)),
                (65, 19) => Some(CfgPin::Data(28)),
                (65, 20) => Some(CfgPin::Data(29)),
                (65, 21) => Some(CfgPin::Data(26)),
                (65, 22) => Some(CfgPin::Data(27)),
                (65, 23) => Some(CfgPin::Data(24)),
                (65, 24) => Some(CfgPin::Data(25)),
                (65, 25) => Some(if self.grid_kind == GridKind::Ultrascale {CfgPin::PerstN1} else {CfgPin::SmbAlert}),
                (65, 26) => Some(CfgPin::Data(22)),
                (65, 27) => Some(CfgPin::Data(23)),
                (65, 28) => Some(CfgPin::Data(20)),
                (65, 29) => Some(CfgPin::Data(21)),
                (65, 30) => Some(CfgPin::Data(18)),
                (65, 31) => Some(CfgPin::Data(19)),
                (65, 32) => Some(CfgPin::Data(16)),
                (65, 33) => Some(CfgPin::Data(17)),
                (65, 34) => Some(CfgPin::Data(14)),
                (65, 35) => Some(CfgPin::Data(15)),
                (65, 36) => Some(CfgPin::Data(12)),
                (65, 37) => Some(CfgPin::Data(13)),
                (65, 38) => Some(CfgPin::CsiB),
                (65, 39) => Some(CfgPin::Data(10)),
                (65, 40) => Some(CfgPin::Data(11)),
                (65, 41) => Some(CfgPin::Data(8)),
                (65, 42) => Some(CfgPin::Data(9)),
                (65, 43) => Some(CfgPin::Data(6)),
                (65, 44) => Some(CfgPin::Data(7)),
                (65, 45) => Some(CfgPin::Data(4)),
                (65, 46) => Some(CfgPin::Data(5)),
                (65, 47) => Some(CfgPin::I2cSclk),
                (65, 48) => Some(CfgPin::I2cSda),
                (65, 49) => Some(CfgPin::UserCclk),
                (65, 50) => Some(CfgPin::Dout),
                (65, 51) => Some(CfgPin::PerstN0),
                _ => None,
            }
        } else {
            match (self.bank, self.bel) {
                (65, 0) => Some(CfgPin::Rs(1)),
                (65, 1) => Some(CfgPin::FweB),
                (65, 2) => Some(CfgPin::Rs(0)),
                (65, 3) => Some(CfgPin::FoeB),
                (65, 4) => Some(CfgPin::Addr(28)),
                (65, 5) => Some(CfgPin::Addr(26)),
                (65, 6) => Some(CfgPin::SmbAlert),
                (65, 7) => Some(CfgPin::Addr(27)),
                (65, 8) => Some(CfgPin::Addr(24)),
                (65, 9) => Some(CfgPin::Addr(22)),
                (65, 10) => Some(CfgPin::Addr(25)),
                (65, 11) => Some(CfgPin::Addr(23)),
                (65, 12) => Some(CfgPin::Addr(20)),
                (65, 13) => Some(CfgPin::Addr(18)),
                (65, 14) => Some(CfgPin::Addr(16)),
                (65, 15) => Some(CfgPin::Addr(19)),
                (65, 16) => Some(CfgPin::Addr(17)),
                (65, 17) => Some(CfgPin::Data(30)),
                (65, 18) => Some(CfgPin::Data(28)),
                (65, 19) => Some(CfgPin::Data(31)),
                (65, 20) => Some(CfgPin::Data(29)),
                (65, 21) => Some(CfgPin::Data(26)),
                (65, 22) => Some(CfgPin::Data(24)),
                (65, 23) => Some(CfgPin::Data(27)),
                (65, 24) => Some(CfgPin::Data(25)),
                (65, 25) => Some(CfgPin::Addr(21)),
                (65, 26) => Some(CfgPin::CsiB),
                (65, 27) => Some(CfgPin::Data(22)),
                (65, 28) => Some(CfgPin::UserCclk),
                (65, 29) => Some(CfgPin::Data(23)),
                (65, 30) => Some(CfgPin::Data(20)),
                (65, 31) => Some(CfgPin::Data(18)),
                (65, 32) => Some(CfgPin::Data(21)),
                (65, 33) => Some(CfgPin::Data(19)),
                (65, 34) => Some(CfgPin::Data(16)),
                (65, 35) => Some(CfgPin::Data(14)),
                (65, 36) => Some(CfgPin::Data(17)),
                (65, 37) => Some(CfgPin::Data(15)),
                (65, 38) => Some(CfgPin::Data(12)),
                (65, 39) => Some(CfgPin::Data(10)),
                (65, 40) => Some(CfgPin::Data(8)),
                (65, 41) => Some(CfgPin::Data(11)),
                (65, 42) => Some(CfgPin::Data(9)),
                (65, 43) => Some(CfgPin::Data(6)),
                (65, 44) => Some(CfgPin::Data(4)),
                (65, 45) => Some(CfgPin::Data(7)),
                (65, 46) => Some(CfgPin::Data(5)),
                (65, 47) => Some(CfgPin::I2cSclk),
                (65, 48) => Some(CfgPin::Dout),
                (65, 49) => Some(CfgPin::I2cSda),
                (65, 50) => Some(CfgPin::PerstN0),
                (65, 51) => Some(CfgPin::Data(13)),
                _ => None,
            }
        }
    }
}

pub fn get_io(grids: &EntityVec<SlrId, Grid>, grid_master: SlrId, disabled: &BTreeSet<DisabledPart>) -> Vec<Io> {
    let mut res = Vec::new();
    let mut io_has_io: Vec<_> = grids[grid_master].cols_io.iter().map(|_| false).collect();
    let mut hard_has_io = false;
    let mut cfg_has_io = false;
    let mut reg_has_hprio = Vec::new();
    let mut reg_has_hdio = Vec::new();
    let mut reg_base = 0;
    let mut reg_cfg = None;
    for (gi, grid) in grids {
        for _ in 0..grid.regs {
            reg_has_hdio.push(false);
            reg_has_hprio.push(false);
        }
        for (i, c) in grid.cols_io.iter().enumerate() {
            for (j, &kind) in c.regs.iter().enumerate() {
                if disabled.contains(&DisabledPart::Region(gi, j as u32)) {
                    continue
                }
                if matches!(kind, IoRowKind::Hpio | IoRowKind::Hrio) {
                    io_has_io[i] = true;
                    reg_has_hprio[reg_base + j] = true;
                }
            }
        }
        if let Some(ref c) = grid.col_hard {
            for (j, &kind) in c.regs.iter().enumerate() {
                if disabled.contains(&DisabledPart::Region(gi, j as u32)) {
                    continue
                }
                if matches!(kind, HardRowKind::Hdio | HardRowKind::HdioAms) {
                    hard_has_io = true;
                    reg_has_hdio[reg_base + j] = true;
                }
            }
        }
        for (j, &kind) in grid.col_cfg.regs.iter().enumerate() {
            if disabled.contains(&DisabledPart::Region(gi, j as u32)) {
                continue
            }
            if matches!(kind, HardRowKind::Hdio | HardRowKind::HdioAms) {
                cfg_has_io = true;
                reg_has_hdio[reg_base + j] = true;
            }
            if gi == grid_master && kind == HardRowKind::Cfg {
                reg_cfg = Some(reg_base + j);
            }
        }
        reg_base += grid.regs;
    }
    let reg_cfg = reg_cfg.unwrap();
    let mut ioy: u32 = 0;
    let mut reg_ioy = Vec::new();
    for (has_hprio, has_hdio) in reg_has_hprio.into_iter().zip(reg_has_hdio.into_iter()) {
        if has_hprio {
            reg_ioy.push((ioy, ioy + 26));
            ioy += 52;
        } else if has_hdio {
            reg_ioy.push((ioy, ioy + 12));
            ioy += 24;
        } else {
            reg_ioy.push((ioy, ioy));
        }
    }
    let mut iox_io = Vec::new();
    let mut iox_hard = 0;
    let mut iox_cfg = 0;
    let mut iox = 0;
    let mut prev_col = grids[grid_master].columns.first_id().unwrap();
    for (i, &has_io) in io_has_io.iter().enumerate() {
        if hard_has_io && grids[grid_master].col_hard.as_ref().unwrap().col > prev_col && grids[grid_master].col_hard.as_ref().unwrap().col < grids[grid_master].cols_io[i].col {
            iox_hard = iox;
            iox += 1;
        }
        if cfg_has_io && grids[grid_master].col_cfg.col > prev_col && grids[grid_master].col_cfg.col < grids[grid_master].cols_io[i].col {
            iox_cfg = iox;
            iox += 1;
        }
        iox_io.push(iox);
        if has_io {
            iox += 1;
        }
        prev_col = grids[grid_master].cols_io[i].col;
    }
    let iox_spec = if iox_io.len() > 1 && io_has_io[iox_io.len() - 2] {
        iox_io[iox_io.len() - 2]
    } else {
        assert!(io_has_io[iox_io.len() - 1]);
        iox_io[iox_io.len() - 1]
    };
    reg_base = 0;
    for (gi, grid) in grids {
        // HPIO/HRIO
        for (i, c) in grid.cols_io.iter().enumerate() {
            for (j, &kind) in c.regs.iter().enumerate() {
                let reg = reg_base + j;
                if disabled.contains(&DisabledPart::Region(gi, j as u32)) {
                    continue
                }
                if matches!(kind, IoRowKind::Hpio | IoRowKind::Hrio) {
                    for bel in 0..52 {
                        let mut bank = (65 + reg - reg_cfg) as u32 + iox_io[i] * 20 - iox_spec * 20;
                        if bank == 64 && kind == IoRowKind::Hrio {
                            if bel < 26 {
                                bank = 94;
                            } else {
                                bank = 84;
                            }
                        }
                        if i == 0 && iox_io[i] != iox_spec && grids[grid_master].kind == GridKind::UltrascalePlus && !hard_has_io {
                            bank -= 20;
                        }
                        res.push(Io {
                            col: c.col,
                            side: Some(c.side),
                            reg: reg as u32,
                            bel,
                            iox: iox_io[i],
                            ioy: reg_ioy[reg].0 + bel,
                            bank,
                            kind: match kind {
                                IoRowKind::Hpio => IoKind::Hpio,
                                IoRowKind::Hrio => IoKind::Hrio,
                                _ => unreachable!(),
                            },
                            grid_kind: grid.kind,
                            is_hdio_ams: false,
                            is_alt_cfg: grid.is_alt_cfg,
                        });
                    }
                }
            }
        }
        // HDIO
        if let Some(ref c) = grid.col_hard {
            for (j, &kind) in c.regs.iter().enumerate() {
                let reg = reg_base + j;
                if disabled.contains(&DisabledPart::Region(gi, j as u32)) {
                    continue
                }
                if matches!(kind, HardRowKind::Hdio | HardRowKind::HdioAms) {
                    let bank = (65 + reg - reg_cfg) as u32 + iox_hard * 20 - iox_spec * 20;
                    for bel in 0..24 {
                        res.push(Io {
                            col: c.col,
                            side: None,
                            reg: reg as u32,
                            bel,
                            iox: iox_hard,
                            ioy: if bel < 12 { reg_ioy[reg].0 + bel } else { reg_ioy[reg].1 + bel - 12 },
                            bank,
                            kind: IoKind::Hdio,
                            grid_kind: grid.kind,
                            is_hdio_ams: kind == HardRowKind::HdioAms,
                            is_alt_cfg: grid.is_alt_cfg,
                        });
                    }
                }
            }
        }
        for (j, &kind) in grid.col_cfg.regs.iter().enumerate() {
            let reg = reg_base + j;
            if disabled.contains(&DisabledPart::Region(gi, j as u32)) {
                continue
            }
            if matches!(kind, HardRowKind::Hdio | HardRowKind::HdioAms) {
                let bank = (65 + reg - reg_cfg) as u32 + iox_cfg * 20 - iox_spec * 20;
                for bel in 0..24 {
                    res.push(Io {
                        col: grid.col_cfg.col,
                        side: None,
                        reg: reg as u32,
                        bel,
                        iox: iox_cfg,
                        ioy: if bel < 12 { reg_ioy[reg].0 + bel } else { reg_ioy[reg].1 + bel - 12 },
                        bank,
                        kind: IoKind::Hdio,
                        grid_kind: grid.kind,
                        is_hdio_ams: kind == HardRowKind::HdioAms,
                        is_alt_cfg: grid.is_alt_cfg,
                    });
                }
            }
        }
        reg_base += grid.regs;
    }
    res
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Gt {
    pub col: ColId,
    pub side: ColSide,
    pub reg: u32,
    pub gx: u32,
    pub gy: u32,
    pub bank: u32,
    pub kind: IoRowKind,
}

pub fn get_gt(grids: &EntityVec<SlrId, Grid>, grid_master: SlrId, disabled: &BTreeSet<DisabledPart>) -> Vec<Gt> {
    let mut res = Vec::new();
    for kind in [
        IoRowKind::Gth,
        IoRowKind::Gty,
        IoRowKind::Gtm,
        IoRowKind::Gtf,
        IoRowKind::HsAdc,
        IoRowKind::HsDac,
        IoRowKind::RfAdc,
        IoRowKind::RfDac,
    ] {
        let mut col_has_gt: Vec<_> = grids[grid_master].cols_io.iter().map(|_| false).collect();
        let mut reg_has_gt = Vec::new();
        let mut reg_base = 0;
        let mut reg_cfg = None;
        for (gi, grid) in grids {
            for _ in 0..grid.regs {
                reg_has_gt.push(false);
            }
            for (i, c) in grid.cols_io.iter().enumerate() {
                for (j, &rkind) in c.regs.iter().enumerate() {
                    let reg = reg_base + j;
                    if disabled.contains(&DisabledPart::Region(gi, j as u32)) {
                        continue
                    }
                    if kind == rkind {
                        col_has_gt[i] = true;
                        reg_has_gt[reg] = true;
                    }
                }
            }
            for (j, &rkind) in grid.col_cfg.regs.iter().enumerate() {
                let reg = reg_base + j;
                if disabled.contains(&DisabledPart::Region(gi, j as u32)) {
                    continue
                }
                if gi == grid_master && rkind == HardRowKind::Cfg {
                    reg_cfg = Some(reg);
                }
            }
            reg_base += grid.regs;
        }
        let reg_cfg = reg_cfg.unwrap();
        let mut gy: u32 = 0;
        let mut reg_gy = Vec::new();
        for has_gt in reg_has_gt {
            reg_gy.push(gy);
            if has_gt {
                gy += 1;
            }
        }
        let mut col_gx = Vec::new();
        let mut gx = 0;
        for has_gt in col_has_gt {
            col_gx.push(gx);
            if has_gt {
                gx += 1;
            }
        }
        reg_base = 0;
        for (gi, grid) in grids {
            for (i, c) in grid.cols_io.iter().enumerate() {
                for (j, &rkind) in c.regs.iter().enumerate() {
                    let reg = reg_base + j;
                    if disabled.contains(&DisabledPart::Region(gi, j as u32)) {
                        continue
                    }
                    if kind != rkind {
                        continue
                    }
                    let mut bank = (125 + reg - reg_cfg) as u32;
                    if i != 0 {
                        bank += 100;
                    }
                    res.push(Gt {
                        col: c.col,
                        side: c.side,
                        reg: reg as u32,
                        gx: col_gx[i],
                        gy: reg_gy[reg],
                        bank,
                        kind,
                    });
                }
            }
            reg_base += grid.regs;
        }
    }
    res
}

pub fn expand_grid<'a>(grids: &EntityVec<SlrId, &Grid>, grid_master: SlrId, disabled: &BTreeSet<DisabledPart>, db: &'a int::IntDb) -> eint::ExpandedGrid<'a> {
    let mut egrid = eint::ExpandedGrid::new(db);
    let mut yb = 0;
    for (slrid, grid) in grids {
        let mut reg_skip_bot = 0;
        let mut reg_skip_top = 0;
        for i in 0..grid.regs {
            if disabled.contains(&DisabledPart::Region(slrid, i as u32)) {
                reg_skip_bot += 1;
            } else {
                break;
            }
        }
        for i in (0..grid.regs).rev() {
            if disabled.contains(&DisabledPart::Region(slrid, i as u32)) {
                reg_skip_top += 1;
            } else {
                break;
            }
        }
        if grid.kind == GridKind::Ultrascale && reg_skip_bot != 0 {
            yb += 1;
        }
        let row_skip = reg_skip_bot * 60;
        egrid.tiles.push(Array2::default([grid.regs * 60, grid.columns.len()]));
        let mut slr = egrid.slr_mut(slrid);
        for (col, &cd) in &grid.columns {
            let x = col.to_idx();
            for row in slr.rows() {
                let y = if row.to_idx() < row_skip {
                    0
                } else {
                    yb + row.to_idx() - row_skip
                };
                slr.fill_tile((col, row), "INT", "NODE.INT", format!("INT_X{x}Y{y}"));
                match cd.l {
                    ColumnKindLeft::CleL | ColumnKindLeft::CleM | ColumnKindLeft::CleMClkBuf | ColumnKindLeft::CleMLaguna => (),
                    ColumnKindLeft::Bram | ColumnKindLeft::BramTd | ColumnKindLeft::BramAuxClmp | ColumnKindLeft::BramBramClmp | ColumnKindLeft::Uram => {
                        let kind = if grid.kind == GridKind::Ultrascale {"INT_INTERFACE_L"} else {"INT_INTF_L"};
                        slr.tile_mut((col, row)).intfs.push(eint::ExpandedTileIntf {
                            kind: db.get_intf("INTF.W"),
                            name: format!("{kind}_X{x}Y{y}"),
                            naming_int: db.get_naming("INTF.W"),
                            naming_buf: None,
                            naming_site: Some(db.get_naming("INTF.W.SITE")),
                            naming_delay: None,
                        });
                    }
                    ColumnKindLeft::Gt | ColumnKindLeft::Io => {
                        let cio = grid.cols_io.iter().find(|x| x.col == col && x.side == ColSide::Left).unwrap();
                        let rk = cio.regs[row.to_idx() / 60];
                        match (grid.kind, rk) {
                            (_, IoRowKind::None) => (),
                            (GridKind::Ultrascale, IoRowKind::Hpio | IoRowKind::Hrio) => {
                                let kind = "INT_INT_INTERFACE_XIPHY_FT";
                                slr.tile_mut((col, row)).intfs.push(eint::ExpandedTileIntf {
                                    kind: db.get_intf("INTF.W.DELAY"),
                                    name: format!("{kind}_X{x}Y{y}"),
                                    naming_int: db.get_naming("INTF.W.IO"),
                                    naming_buf: None,
                                    naming_site: Some(db.get_naming("INTF.W.IO.SITE")),
                                    naming_delay: Some(db.get_naming("INTF.W.IO.DELAY")),
                                });
                            }
                            (GridKind::UltrascalePlus, IoRowKind::Hpio | IoRowKind::Hrio) => {
                                let kind = if col.to_idx() == 0 {"INT_INTF_LEFT_TERM_IO_FT"} else if matches!(row.to_idx() % 15, 0 | 1 | 13 | 14) {"INT_INTF_L_CMT"} else {"INT_INTF_L_IO"};
                                slr.tile_mut((col, row)).intfs.push(eint::ExpandedTileIntf {
                                    kind: db.get_intf("INTF.W.IO"),
                                    name: format!("{kind}_X{x}Y{y}"),
                                    naming_int: db.get_naming("INTF.W.IO"),
                                    naming_buf: None,
                                    naming_site: Some(db.get_naming("INTF.W.IO.SITE")),
                                    naming_delay: Some(db.get_naming("INTF.W.IO.DELAY")),
                                });
                            }
                            _ => {
                                let kind = if grid.kind == GridKind::Ultrascale {"INT_INT_INTERFACE_GT_LEFT_FT"} else {"INT_INTF_L_TERM_GT"};
                                slr.tile_mut((col, row)).intfs.push(eint::ExpandedTileIntf {
                                    kind: db.get_intf("INTF.W.DELAY"),
                                    name: format!("{kind}_X{x}Y{y}"),
                                    naming_int: db.get_naming("INTF.W.GT"),
                                    naming_buf: None,
                                    naming_site: Some(db.get_naming("INTF.W.GT.SITE")),
                                    naming_delay: Some(db.get_naming("INTF.W.GT.DELAY")),
                                });
                            }
                        }
                    }
                    ColumnKindLeft::Hard | ColumnKindLeft::Sdfec | ColumnKindLeft::DfeC | ColumnKindLeft::DfeDF | ColumnKindLeft::DfeE => {
                        let kind = if grid.kind == GridKind::Ultrascale {"INT_INTERFACE_PCIE_L"} else {"INT_INTF_L_PCIE4"};
                        slr.tile_mut((col, row)).intfs.push(eint::ExpandedTileIntf {
                            kind: db.get_intf("INTF.W.DELAY"),
                            name: format!("{kind}_X{x}Y{y}"),
                            naming_int: db.get_naming("INTF.W.PCIE"),
                            naming_buf: None,
                            naming_site: Some(db.get_naming("INTF.W.PCIE.SITE")),
                            naming_delay: Some(db.get_naming("INTF.W.PCIE.DELAY")),
                        });
                    }
                }
                match cd.r {
                    ColumnKindRight::CleL | ColumnKindRight::CleLDcg10 => (),
                    ColumnKindRight::Dsp | ColumnKindRight::DspClkBuf | ColumnKindRight::Uram => {
                        let kind = if grid.kind == GridKind::Ultrascale {"INT_INTERFACE_R"} else {"INT_INTF_R"};
                        slr.tile_mut((col, row)).intfs.push(eint::ExpandedTileIntf {
                            kind: db.get_intf("INTF.E"),
                            name: format!("{kind}_X{x}Y{y}"),
                            naming_int: db.get_naming("INTF.E"),
                            naming_buf: None,
                            naming_site: Some(db.get_naming("INTF.E.SITE")),
                            naming_delay: None,
                        });
                    }
                    ColumnKindRight::Gt | ColumnKindRight::Io => {
                        let cio = grid.cols_io.iter().find(|x| x.col == col && x.side == ColSide::Right).unwrap();
                        let rk = cio.regs[row.to_idx() / 60];
                        match (grid.kind, rk) {
                            (_, IoRowKind::None) => (),
                            (GridKind::Ultrascale, IoRowKind::Hpio | IoRowKind::Hrio) => unreachable!(),
                            (GridKind::UltrascalePlus, IoRowKind::Hpio | IoRowKind::Hrio) => {
                                let kind = "INT_INTF_RIGHT_TERM_IO";
                                slr.tile_mut((col, row)).intfs.push(eint::ExpandedTileIntf {
                                    kind: db.get_intf("INTF.E.IO"),
                                    name: format!("{kind}_X{x}Y{y}"),
                                    naming_int: db.get_naming("INTF.E.IO"),
                                    naming_buf: None,
                                    naming_site: Some(db.get_naming("INTF.E.IO.SITE")),
                                    naming_delay: Some(db.get_naming("INTF.E.IO.DELAY")),
                                });
                            }
                            _ => {
                                let kind = if grid.kind == GridKind::Ultrascale {"INT_INTERFACE_GT_R"} else {"INT_INTF_R_TERM_GT"};
                                slr.tile_mut((col, row)).intfs.push(eint::ExpandedTileIntf {
                                    kind: db.get_intf("INTF.E.DELAY"),
                                    name: format!("{kind}_X{x}Y{y}"),
                                    naming_int: db.get_naming("INTF.E.GT"),
                                    naming_buf: None,
                                    naming_site: Some(db.get_naming("INTF.E.GT.SITE")),
                                    naming_delay: Some(db.get_naming("INTF.E.GT.DELAY")),
                                });
                            }
                        }
                    }
                    ColumnKindRight::Hard | ColumnKindRight::DfeB | ColumnKindRight::DfeC | ColumnKindRight::DfeDF | ColumnKindRight::DfeE => {
                        let kind = if grid.kind == GridKind::Ultrascale {"INT_INTERFACE_PCIE_R"} else {"INT_INTF_R_PCIE4"};
                        slr.tile_mut((col, row)).intfs.push(eint::ExpandedTileIntf {
                            kind: db.get_intf("INTF.E.DELAY"),
                            name: format!("{kind}_X{x}Y{y}"),
                            naming_int: db.get_naming("INTF.E.PCIE"),
                            naming_buf: None,
                            naming_site: Some(db.get_naming("INTF.E.PCIE.SITE")),
                            naming_delay: Some(db.get_naming("INTF.E.PCIE.DELAY")),
                        });
                    }
                }
            }
        }

        if grid.kind == GridKind::UltrascalePlus {
            for (col, &cd) in &grid.columns {
                if cd.l == ColumnKindLeft::Io && col.to_idx() != 0 {
                    let term_e = db.get_term("IO.E");
                    let term_w = db.get_term("IO.W");
                    for row in slr.rows() {
                        slr.fill_term_pair_anon((col - 1, row), (col, row), term_e, term_w);
                    }
                }
            }
        }

        if let Some(ref ps) = grid.ps {
            let height = if ps.has_vcu {240} else {180};
            let width = ps.col.to_idx();
            slr.nuke_rect(ColId(0), RowId(0), width, height);
            if height != grid.regs * 60 {
                let row_t = RowId::from_idx(height);
                for dx in 0..width {
                    let col = ColId::from_idx(dx);
                    slr.fill_term_anon((col, row_t), "S");
                }
            }
            let x = ps.col.to_idx();
            for dy in 0..height {
                let row = RowId::from_idx(dy);
                let y = if row.to_idx() < row_skip {
                    0
                } else {
                    yb + row.to_idx() - row_skip
                };
                slr.fill_term_anon((ps.col, row), "W");
                slr.tile_mut((ps.col, row)).intfs.insert(0, eint::ExpandedTileIntf {
                    kind: db.get_intf("INTF.W.IO"),
                    name: format!("INT_INTF_LEFT_TERM_PSS_X{x}Y{y}"),
                    naming_int: db.get_naming("INTF.PSS"),
                    naming_buf: None,
                    naming_site: Some(db.get_naming("INTF.PSS.SITE")),
                    naming_delay: Some(db.get_naming("INTF.PSS.DELAY")),
                });
            }
        }

        slr.nuke_rect(ColId(0), RowId(0), grid.columns.len(), reg_skip_bot * 60);
        slr.nuke_rect(ColId(0), RowId::from_idx((grid.regs - reg_skip_top) * 60), grid.columns.len(), reg_skip_top * 60);

        let col_l = slr.cols().next().unwrap();
        let col_r = slr.cols().next_back().unwrap();
        let row_b = slr.rows().next().unwrap();
        let row_t = slr.rows().next_back().unwrap();
        for col in slr.cols() {
            if slr[(col, row_b)].is_some() {
                slr.fill_term_anon((col, row_b), "S");
            }
            if slr[(col, row_t)].is_some() {
                slr.fill_term_anon((col, row_t), "N");
            }
        }
        for row in slr.rows() {
            if slr[(col_l, row)].is_some() {
                slr.fill_term_anon((col_l, row), "W");
            }
            if slr[(col_r, row)].is_some() {
                slr.fill_term_anon((col_r, row), "E");
            }
        }

        slr.fill_main_passes();

        yb += slr.rows().len() - reg_skip_bot * 60 - reg_skip_top * 60;
    }

    egrid
}
