use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::grid::{ColId, DieId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::{ColSide, DisabledPart, Grid, GridKind, HardRowKind, IoKind, IoRowKind, SharedCfgPin};

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
    pub fn get_cfg(&self) -> Option<SharedCfgPin> {
        if !self.is_alt_cfg {
            match (self.bank, self.bel) {
                (65, 0) => Some(SharedCfgPin::Rs(0)),
                (65, 1) => Some(SharedCfgPin::Rs(1)),
                (65, 2) => Some(SharedCfgPin::FoeB),
                (65, 3) => Some(SharedCfgPin::FweB),
                (65, 4) => Some(SharedCfgPin::Addr(26)),
                (65, 5) => Some(SharedCfgPin::Addr(27)),
                (65, 6) => Some(SharedCfgPin::Addr(24)),
                (65, 7) => Some(SharedCfgPin::Addr(25)),
                (65, 8) => Some(SharedCfgPin::Addr(22)),
                (65, 9) => Some(SharedCfgPin::Addr(23)),
                (65, 10) => Some(SharedCfgPin::Addr(20)),
                (65, 11) => Some(SharedCfgPin::Addr(21)),
                (65, 12) => Some(SharedCfgPin::Addr(28)),
                (65, 13) => Some(SharedCfgPin::Addr(18)),
                (65, 14) => Some(SharedCfgPin::Addr(19)),
                (65, 15) => Some(SharedCfgPin::Addr(16)),
                (65, 16) => Some(SharedCfgPin::Addr(17)),
                (65, 17) => Some(SharedCfgPin::Data(30)),
                (65, 18) => Some(SharedCfgPin::Data(31)),
                (65, 19) => Some(SharedCfgPin::Data(28)),
                (65, 20) => Some(SharedCfgPin::Data(29)),
                (65, 21) => Some(SharedCfgPin::Data(26)),
                (65, 22) => Some(SharedCfgPin::Data(27)),
                (65, 23) => Some(SharedCfgPin::Data(24)),
                (65, 24) => Some(SharedCfgPin::Data(25)),
                (65, 25) => Some(if self.grid_kind == GridKind::Ultrascale {
                    SharedCfgPin::PerstN1
                } else {
                    SharedCfgPin::SmbAlert
                }),
                (65, 26) => Some(SharedCfgPin::Data(22)),
                (65, 27) => Some(SharedCfgPin::Data(23)),
                (65, 28) => Some(SharedCfgPin::Data(20)),
                (65, 29) => Some(SharedCfgPin::Data(21)),
                (65, 30) => Some(SharedCfgPin::Data(18)),
                (65, 31) => Some(SharedCfgPin::Data(19)),
                (65, 32) => Some(SharedCfgPin::Data(16)),
                (65, 33) => Some(SharedCfgPin::Data(17)),
                (65, 34) => Some(SharedCfgPin::Data(14)),
                (65, 35) => Some(SharedCfgPin::Data(15)),
                (65, 36) => Some(SharedCfgPin::Data(12)),
                (65, 37) => Some(SharedCfgPin::Data(13)),
                (65, 38) => Some(SharedCfgPin::CsiB),
                (65, 39) => Some(SharedCfgPin::Data(10)),
                (65, 40) => Some(SharedCfgPin::Data(11)),
                (65, 41) => Some(SharedCfgPin::Data(8)),
                (65, 42) => Some(SharedCfgPin::Data(9)),
                (65, 43) => Some(SharedCfgPin::Data(6)),
                (65, 44) => Some(SharedCfgPin::Data(7)),
                (65, 45) => Some(SharedCfgPin::Data(4)),
                (65, 46) => Some(SharedCfgPin::Data(5)),
                (65, 47) => Some(SharedCfgPin::I2cSclk),
                (65, 48) => Some(SharedCfgPin::I2cSda),
                (65, 49) => Some(SharedCfgPin::EmCclk),
                (65, 50) => Some(SharedCfgPin::Dout),
                (65, 51) => Some(SharedCfgPin::PerstN0),
                _ => None,
            }
        } else {
            match (self.bank, self.bel) {
                (65, 0) => Some(SharedCfgPin::Rs(1)),
                (65, 1) => Some(SharedCfgPin::FweB),
                (65, 2) => Some(SharedCfgPin::Rs(0)),
                (65, 3) => Some(SharedCfgPin::FoeB),
                (65, 4) => Some(SharedCfgPin::Addr(28)),
                (65, 5) => Some(SharedCfgPin::Addr(26)),
                (65, 6) => Some(SharedCfgPin::SmbAlert),
                (65, 7) => Some(SharedCfgPin::Addr(27)),
                (65, 8) => Some(SharedCfgPin::Addr(24)),
                (65, 9) => Some(SharedCfgPin::Addr(22)),
                (65, 10) => Some(SharedCfgPin::Addr(25)),
                (65, 11) => Some(SharedCfgPin::Addr(23)),
                (65, 12) => Some(SharedCfgPin::Addr(20)),
                (65, 13) => Some(SharedCfgPin::Addr(18)),
                (65, 14) => Some(SharedCfgPin::Addr(16)),
                (65, 15) => Some(SharedCfgPin::Addr(19)),
                (65, 16) => Some(SharedCfgPin::Addr(17)),
                (65, 17) => Some(SharedCfgPin::Data(30)),
                (65, 18) => Some(SharedCfgPin::Data(28)),
                (65, 19) => Some(SharedCfgPin::Data(31)),
                (65, 20) => Some(SharedCfgPin::Data(29)),
                (65, 21) => Some(SharedCfgPin::Data(26)),
                (65, 22) => Some(SharedCfgPin::Data(24)),
                (65, 23) => Some(SharedCfgPin::Data(27)),
                (65, 24) => Some(SharedCfgPin::Data(25)),
                (65, 25) => Some(SharedCfgPin::Addr(21)),
                (65, 26) => Some(SharedCfgPin::CsiB),
                (65, 27) => Some(SharedCfgPin::Data(22)),
                (65, 28) => Some(SharedCfgPin::EmCclk),
                (65, 29) => Some(SharedCfgPin::Data(23)),
                (65, 30) => Some(SharedCfgPin::Data(20)),
                (65, 31) => Some(SharedCfgPin::Data(18)),
                (65, 32) => Some(SharedCfgPin::Data(21)),
                (65, 33) => Some(SharedCfgPin::Data(19)),
                (65, 34) => Some(SharedCfgPin::Data(16)),
                (65, 35) => Some(SharedCfgPin::Data(14)),
                (65, 36) => Some(SharedCfgPin::Data(17)),
                (65, 37) => Some(SharedCfgPin::Data(15)),
                (65, 38) => Some(SharedCfgPin::Data(12)),
                (65, 39) => Some(SharedCfgPin::Data(10)),
                (65, 40) => Some(SharedCfgPin::Data(8)),
                (65, 41) => Some(SharedCfgPin::Data(11)),
                (65, 42) => Some(SharedCfgPin::Data(9)),
                (65, 43) => Some(SharedCfgPin::Data(6)),
                (65, 44) => Some(SharedCfgPin::Data(4)),
                (65, 45) => Some(SharedCfgPin::Data(7)),
                (65, 46) => Some(SharedCfgPin::Data(5)),
                (65, 47) => Some(SharedCfgPin::I2cSclk),
                (65, 48) => Some(SharedCfgPin::Dout),
                (65, 49) => Some(SharedCfgPin::I2cSda),
                (65, 50) => Some(SharedCfgPin::PerstN0),
                (65, 51) => Some(SharedCfgPin::Data(13)),
                _ => None,
            }
        }
    }
}

pub fn get_io(
    grids: &EntityVec<DieId, Grid>,
    grid_master: DieId,
    disabled: &BTreeSet<DisabledPart>,
) -> Vec<Io> {
    let mut res = Vec::new();
    let mut io_has_io: Vec<_> = grids[grid_master].cols_io.iter().map(|_| false).collect();
    let mut hard_has_io = [false, false];
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
            for (reg, &kind) in c.regs.iter() {
                if disabled.contains(&DisabledPart::Region(gi, reg)) {
                    continue;
                }
                if matches!(kind, IoRowKind::Hpio | IoRowKind::Hrio) {
                    io_has_io[i] = true;
                    reg_has_hprio[reg_base + reg.to_idx()] = true;
                }
            }
        }
        for (i, hc) in grid.cols_hard.iter().enumerate() {
            for (reg, &kind) in hc.regs.iter() {
                if disabled.contains(&DisabledPart::Region(gi, reg)) {
                    continue;
                }
                if matches!(kind, HardRowKind::Hdio | HardRowKind::HdioAms) {
                    hard_has_io[i] = true;
                    reg_has_hdio[reg_base + reg.to_idx()] = true;
                }
                if gi == grid_master && kind == HardRowKind::Cfg {
                    reg_cfg = Some(reg_base + reg.to_idx());
                }
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
    let mut iox_hard = [0, 0];
    let mut iox = 0;
    let mut prev_col = grids[grid_master].columns.first_id().unwrap();
    for (i, &has_io) in io_has_io.iter().enumerate() {
        for (j, hc) in grids[grid_master].cols_hard.iter().enumerate() {
            if hard_has_io[j] && hc.col > prev_col && hc.col < grids[grid_master].cols_io[i].col {
                iox_hard[j] = iox;
                iox += 1;
            }
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
            for (reg, &kind) in c.regs.iter() {
                if disabled.contains(&DisabledPart::Region(gi, reg)) {
                    continue;
                }
                let reg = reg_base + reg.to_idx();
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
                        if i == 0
                            && iox_io[i] != iox_spec
                            && grids[grid_master].kind == GridKind::UltrascalePlus
                            && ((grids[grid_master].cols_hard.len() == 1) || !hard_has_io[0])
                        {
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
        for (i, hc) in grid.cols_hard.iter().enumerate() {
            for (reg, &kind) in hc.regs.iter() {
                if disabled.contains(&DisabledPart::Region(gi, reg)) {
                    continue;
                }
                let reg = reg_base + reg.to_idx();
                if matches!(kind, HardRowKind::Hdio | HardRowKind::HdioAms) {
                    let bank = (65 + reg - reg_cfg) as u32 + iox_hard[i] * 20 - iox_spec * 20;
                    for bel in 0..24 {
                        res.push(Io {
                            col: hc.col,
                            side: None,
                            reg: reg as u32,
                            bel,
                            iox: iox_hard[i],
                            ioy: if bel < 12 {
                                reg_ioy[reg].0 + bel
                            } else {
                                reg_ioy[reg].1 + bel - 12
                            },
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

pub fn get_gt(
    grids: &EntityVec<DieId, Grid>,
    grid_master: DieId,
    disabled: &BTreeSet<DisabledPart>,
) -> Vec<Gt> {
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
            #[allow(clippy::same_item_push)]
            for _ in 0..grid.regs {
                reg_has_gt.push(false);
            }
            for (i, c) in grid.cols_io.iter().enumerate() {
                for (reg, &rkind) in c.regs.iter() {
                    if disabled.contains(&DisabledPart::Region(gi, reg)) {
                        continue;
                    }
                    let reg = reg_base + reg.to_idx();
                    if kind == rkind {
                        col_has_gt[i] = true;
                        reg_has_gt[reg] = true;
                    }
                }
            }
            for (reg, &rkind) in grid.cols_hard.last().unwrap().regs.iter() {
                if disabled.contains(&DisabledPart::Region(gi, reg)) {
                    continue;
                }
                let reg = reg_base + reg.to_idx();
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
                for (reg, &rkind) in c.regs.iter() {
                    if disabled.contains(&DisabledPart::Region(gi, reg)) {
                        continue;
                    }
                    let reg = reg_base + reg.to_idx();
                    if kind != rkind {
                        continue;
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
