use std::collections::BTreeSet;
use serde::{Serialize, Deserialize};
use crate::{CfgPin, DisabledPart};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GridKind {
    Ultrascale,
    UltrascalePlus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub kind: GridKind,
    pub columns: Vec<ColumnKind>,
    pub cols_vbrk: BTreeSet<u32>,
    pub cols_fsr_gap: BTreeSet<u32>,
    pub col_cfg: HardColumn,
    pub col_hard: Option<HardColumn>,
    pub cols_io: Vec<IoColumn>,
    pub rows: u32,
    pub ps: Option<Ps>,
    pub has_hbm: bool,
    pub is_dmc: bool,
    pub is_alt_cfg: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKind {
    CleL,
    CleLDcg10,
    CleM,
    CleMClkBuf,
    CleMLaguna,
    Bram,
    BramAuxClmp,
    BramBramClmp,
    BramTd,
    Dsp,
    DspClkBuf,
    Uram,
    Hard,
    Io,
    Gt,
    Sdfec,
    DfeB,
    DfeC,
    DfeDF,
    DfeE,
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
    pub col: u32,
    pub rows: Vec<HardRowKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum IoRowKind {
    None,
    Hpio,
    Hrio,
    Gth,
    Gty,
    Gtm,
    HsAdc,
    HsDac,
    RfAdc,
    RfDac,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IoColumn {
    pub col: u32,
    pub rows: Vec<IoRowKind>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Ps {
    pub col: u32,
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
    pub col: u32,
    pub row: u32,
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
                    // XXX
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

pub fn get_io(grids: &[Grid], grid_master: usize, disabled: &BTreeSet<DisabledPart>) -> Vec<Io> {
    let mut res = Vec::new();
    let mut io_has_io: Vec<_> = grids[0].cols_io.iter().map(|_| false).collect();
    let mut hard_has_io = false;
    let mut cfg_has_io = false;
    let mut row_has_hprio = Vec::new();
    let mut row_has_hdio = Vec::new();
    let mut row_base = 0;
    let mut row_cfg = None;
    for (gi, grid) in grids.iter().enumerate() {
        for _ in 0..grid.rows {
            row_has_hdio.push(false);
            row_has_hprio.push(false);
        }
        for (i, c) in grid.cols_io.iter().enumerate() {
            for (j, &kind) in c.rows.iter().enumerate() {
                let row = row_base + (j as u32);
                if disabled.contains(&DisabledPart::Region(row)) {
                    continue
                }
                if matches!(kind, IoRowKind::Hpio | IoRowKind::Hrio) {
                    io_has_io[i] = true;
                    row_has_hprio[row as usize] = true;
                }
            }
        }
        if let Some(ref c) = grid.col_hard {
            for (j, &kind) in c.rows.iter().enumerate() {
                let row = row_base + (j as u32);
                if disabled.contains(&DisabledPart::Region(row)) {
                    continue
                }
                if matches!(kind, HardRowKind::Hdio | HardRowKind::HdioAms) {
                    hard_has_io = true;
                    row_has_hdio[row as usize] = true;
                }
            }
        }
        for (j, &kind) in grid.col_cfg.rows.iter().enumerate() {
            let row = row_base + (j as u32);
            if disabled.contains(&DisabledPart::Region(row)) {
                continue
            }
            if matches!(kind, HardRowKind::Hdio | HardRowKind::HdioAms) {
                cfg_has_io = true;
                row_has_hdio[row as usize] = true;
            }
            if gi == grid_master && kind == HardRowKind::Cfg {
                row_cfg = Some(row);
            }
        }
        row_base += grid.rows;
    }
    let row_cfg = row_cfg.unwrap();
    let mut ioy: u32 = 0;
    let mut row_ioy = Vec::new();
    for (has_hprio, has_hdio) in row_has_hprio.into_iter().zip(row_has_hdio.into_iter()) {
        if has_hprio {
            row_ioy.push((ioy, ioy + 26));
            ioy += 52;
        } else if has_hdio {
            row_ioy.push((ioy, ioy + 12));
            ioy += 24;
        } else {
            row_ioy.push((ioy, ioy));
        }
    }
    let mut iox_io = Vec::new();
    let mut iox_hard = 0;
    let mut iox_cfg = 0;
    let mut iox = 0;
    let mut prev_col = 0;
    for (i, &has_io) in io_has_io.iter().enumerate() {
        if hard_has_io && grids[0].col_hard.as_ref().unwrap().col > prev_col && grids[0].col_hard.as_ref().unwrap().col < grids[0].cols_io[i].col {
            iox_hard = iox;
            iox += 1;
        }
        if cfg_has_io && grids[0].col_cfg.col > prev_col && grids[0].col_cfg.col < grids[0].cols_io[i].col {
            iox_cfg = iox;
            iox += 1;
        }
        iox_io.push(iox);
        if has_io {
            iox += 1;
        }
        prev_col = grids[0].cols_io[i].col;
    }
    let iox_spec = if iox_io.len() > 1 && io_has_io[iox_io.len() - 2] {
        iox_io[iox_io.len() - 2]
    } else {
        assert!(io_has_io[iox_io.len() - 1]);
        iox_io[iox_io.len() - 1]
    };
    row_base = 0;
    for grid in grids.iter() {
        // HPIO/HRIO
        for (i, c) in grid.cols_io.iter().enumerate() {
            for (j, &kind) in c.rows.iter().enumerate() {
                let row = row_base + j as u32;
                if disabled.contains(&DisabledPart::Region(row)) {
                    continue
                }
                if matches!(kind, IoRowKind::Hpio | IoRowKind::Hrio) {
                    for bel in 0..52 {
                        let mut bank = 65 + row - row_cfg + iox_io[i] * 20 - iox_spec * 20;
                        if bank == 64 && kind == IoRowKind::Hrio {
                            if bel < 26 {
                                bank = 94;
                            } else {
                                bank = 84;
                            }
                        }
                        if i == 0 && iox_io[i] != iox_spec && grids[0].kind == GridKind::UltrascalePlus && !hard_has_io {
                            bank -= 20;
                        }
                        res.push(Io {
                            col: c.col,
                            row,
                            bel,
                            iox: iox_io[i],
                            ioy: row_ioy[row as usize].0 + bel,
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
            for (j, &kind) in c.rows.iter().enumerate() {
                let row = row_base + j as u32;
                if disabled.contains(&DisabledPart::Region(row)) {
                    continue
                }
                if matches!(kind, HardRowKind::Hdio | HardRowKind::HdioAms) {
                    let bank = 65 + row - row_cfg + iox_hard * 20 - iox_spec * 20;
                    for bel in 0..24 {
                        res.push(Io {
                            col: c.col,
                            row,
                            bel,
                            iox: iox_hard,
                            ioy: if bel < 12 { row_ioy[row as usize].0 + bel } else { row_ioy[row as usize].1 + bel - 12 },
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
        for (j, &kind) in grid.col_cfg.rows.iter().enumerate() {
            let row = row_base + j as u32;
            if disabled.contains(&DisabledPart::Region(row)) {
                continue
            }
            if matches!(kind, HardRowKind::Hdio | HardRowKind::HdioAms) {
                let bank = 65 + row - row_cfg + iox_cfg * 20 - iox_spec * 20;
                for bel in 0..24 {
                    res.push(Io {
                        col: grid.col_cfg.col,
                        row,
                        bel,
                        iox: iox_cfg,
                        ioy: if bel < 12 { row_ioy[row as usize].0 + bel } else { row_ioy[row as usize].1 + bel - 12 },
                        bank,
                        kind: IoKind::Hdio,
                        grid_kind: grid.kind,
                        is_hdio_ams: kind == HardRowKind::HdioAms,
                        is_alt_cfg: grid.is_alt_cfg,
                    });
                }
            }
        }
        row_base += grid.rows;
    }
    res
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Gt {
    pub col: u32,
    pub row: u32,
    pub gx: u32,
    pub gy: u32,
    pub bank: u32,
    pub kind: IoRowKind,
}

pub fn get_gt(grids: &[Grid], grid_master: usize, disabled: &BTreeSet<DisabledPart>) -> Vec<Gt> {
    let mut res = Vec::new();
    for kind in [
        IoRowKind::Gth,
        IoRowKind::Gty,
        IoRowKind::Gtm,
        IoRowKind::HsAdc,
        IoRowKind::HsDac,
        IoRowKind::RfAdc,
        IoRowKind::RfDac,
    ] {
        let mut col_has_gt: Vec<_> = grids[0].cols_io.iter().map(|_| false).collect();
        let mut row_has_gt = Vec::new();
        let mut row_base = 0;
        let mut row_cfg = None;
        for (gi, grid) in grids.iter().enumerate() {
            for _ in 0..grid.rows {
                row_has_gt.push(false);
            }
            for (i, c) in grid.cols_io.iter().enumerate() {
                for (j, &rkind) in c.rows.iter().enumerate() {
                    let row = row_base + (j as u32);
                    if disabled.contains(&DisabledPart::Region(row)) {
                        continue
                    }
                    if kind == rkind {
                        col_has_gt[i] = true;
                        row_has_gt[row as usize] = true;
                    }
                }
            }
            for (j, &rkind) in grid.col_cfg.rows.iter().enumerate() {
                let row = row_base + (j as u32);
                if disabled.contains(&DisabledPart::Region(row)) {
                    continue
                }
                if gi == grid_master && rkind == HardRowKind::Cfg {
                    row_cfg = Some(row);
                }
            }
            row_base += grid.rows;
        }
        let row_cfg = row_cfg.unwrap();
        let mut gy: u32 = 0;
        let mut row_gy = Vec::new();
        for has_gt in row_has_gt {
            row_gy.push(gy);
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
        row_base = 0;
        for grid in grids.iter() {
            for (i, c) in grid.cols_io.iter().enumerate() {
                for (j, &rkind) in c.rows.iter().enumerate() {
                    let row = row_base + j as u32;
                    if disabled.contains(&DisabledPart::Region(row)) {
                        continue
                    }
                    if kind != rkind {
                        continue
                    }
                    let mut bank = 125 + row - row_cfg;
                    if i != 0 {
                        bank += 100;
                    }
                    res.push(Gt {
                        col: c.col,
                        row,
                        gx: col_gx[i],
                        gy: row_gy[row as usize],
                        bank,
                        kind,
                    });
                }
            }
            row_base += grid.rows;
        }
    }
    res
}
