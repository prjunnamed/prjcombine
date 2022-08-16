use std::collections::BTreeSet;
use serde::{Serialize, Deserialize};
use super::{CfgPin, DisabledPart, SysMonPin, GtPin, ColId, RowId, int, eint};
use ndarray::Array2;
use prjcombine_entity::{EntityVec, EntityId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub columns: EntityVec<ColId, ColumnKind>,
    pub cols_vbrk: BTreeSet<ColId>,
    pub cols_mgt_buf: BTreeSet<ColId>,
    pub col_cfg: ColId,
    pub cols_qbuf: (ColId, ColId),
    pub cols_io: [Option<ColId>; 4],
    pub col_hard: Option<HardColumn>,
    pub regs: usize,
    pub reg_gth_start: usize,
    pub reg_cfg: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKind {
    ClbLL,
    ClbLM,
    Bram,
    Dsp,
    Io,
    Gt,
    Cmt,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HardColumn {
    pub col: ColId,
    pub rows_emac: Vec<RowId>,
    pub rows_pcie: Vec<RowId>,
}

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
        matches!((self.bank, self.row.to_idx() % 40), (24 | 34, 36..=39) | (25 | 35, 0..=3))
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
    pub fn sm_pair(&self, grid: &Grid) -> Option<u32> {
        let has_ol = grid.cols_io[0].is_some();
        match (self.bank, self.row.to_idx() % 40) {
            (15, 8 | 9) => Some(15),
            (15, 12 | 13) => Some(14),
            (15, 14 | 15) => Some(13),
            (15, 24 | 25) => Some(12),
            (15, 26 | 27) => Some(11),
            (15, 28 | 29) => Some(10),
            (15, 32 | 33) => Some(9),
            (15, 34 | 35) => Some(8),
            (25, 8 | 9) if !has_ol => Some(15),
            (25, 12 | 13) if !has_ol => Some(14),
            (25, 14 | 15) if !has_ol => Some(13),
            (25, 24 | 25) if !has_ol => Some(12),
            (25, 26 | 27) if !has_ol => Some(11),
            (25, 28 | 29) if !has_ol => Some(10),
            (25, 32 | 33) if !has_ol => Some(9),
            (25, 34 | 35) if !has_ol => Some(8),
            (35, 8 | 9) => Some(7),
            (35, 12 | 13) => Some(6),
            (35, 14 | 15) => Some(5),
            (35, 24 | 25) => Some(4),
            (35, 26 | 27) => Some(3),
            (35, 28 | 29) => Some(2),
            (35, 32 | 33) => Some(1),
            (35, 34 | 35) => Some(0),
            _ => None,
        }
    }
    pub fn get_cfg(&self) -> Option<CfgPin> {
        match (self.bank, self.row.to_idx() % 40) {
            (24, 6) => Some(CfgPin::CsoB),
            (24, 7) => Some(CfgPin::Rs(0)),
            (24, 8) => Some(CfgPin::Rs(1)),
            (24, 9) => Some(CfgPin::FweB),
            (24, 10) => Some(CfgPin::FoeB),
            (24, 11) => Some(CfgPin::FcsB),
            (24, 12) => Some(CfgPin::Data(0)),
            (24, 13) => Some(CfgPin::Data(1)),
            (24, 14) => Some(CfgPin::Data(2)),
            (24, 15) => Some(CfgPin::Data(3)),
            (24, 24) => Some(CfgPin::Data(4)),
            (24, 25) => Some(CfgPin::Data(5)),
            (24, 26) => Some(CfgPin::Data(6)),
            (24, 27) => Some(CfgPin::Data(7)),
            (24, 28) => Some(CfgPin::Data(8)),
            (24, 29) => Some(CfgPin::Data(9)),
            (24, 30) => Some(CfgPin::Data(10)),
            (24, 31) => Some(CfgPin::Data(11)),
            (24, 32) => Some(CfgPin::Data(12)),
            (24, 33) => Some(CfgPin::Data(13)),
            (24, 34) => Some(CfgPin::Data(14)),
            (24, 35) => Some(CfgPin::Data(15)),
            (34, 2) => Some(CfgPin::Addr(16)),
            (34, 3) => Some(CfgPin::Addr(17)),
            (34, 4) => Some(CfgPin::Addr(18)),
            (34, 5) => Some(CfgPin::Addr(19)),
            (34, 6) => Some(CfgPin::Addr(20)),
            (34, 7) => Some(CfgPin::Addr(21)),
            (34, 8) => Some(CfgPin::Addr(22)),
            (34, 9) => Some(CfgPin::Addr(23)),
            (34, 10) => Some(CfgPin::Addr(24)),
            (34, 11) => Some(CfgPin::Addr(25)),
            (34, 12) => Some(CfgPin::Data(16)),
            (34, 13) => Some(CfgPin::Data(17)),
            (34, 14) => Some(CfgPin::Data(18)),
            (34, 15) => Some(CfgPin::Data(19)),
            (34, 24) => Some(CfgPin::Data(20)),
            (34, 25) => Some(CfgPin::Data(21)),
            (34, 26) => Some(CfgPin::Data(22)),
            (34, 27) => Some(CfgPin::Data(23)),
            (34, 28) => Some(CfgPin::Data(24)),
            (34, 29) => Some(CfgPin::Data(25)),
            (34, 30) => Some(CfgPin::Data(26)),
            (34, 31) => Some(CfgPin::Data(27)),
            (34, 32) => Some(CfgPin::Data(28)),
            (34, 33) => Some(CfgPin::Data(29)),
            (34, 34) => Some(CfgPin::Data(30)),
            (34, 35) => Some(CfgPin::Data(31)),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Gt {
    pub col: ColId,
    pub row: RowId,
    pub gtc: u32,
    pub gy: u32,
    pub bank: u32,
    pub is_gth: bool,
}

impl Gt {
    pub fn get_pads(&self, grid: &Grid) -> Vec<(String, String, GtPin, u32)> {
        let mut res = Vec::new();
        let (ipx, opx) = if grid.has_left_gt() {
            (self.gtc * 2, self.gtc)
        } else {
            (1, 0)
        };
        if self.is_gth {
            let gthy = self.row.to_idx() / 40 - grid.reg_gth_start;
            let opy = (grid.reg_gth_start * 32 + gthy * 8) as u32;
            let ipy = (grid.reg_gth_start * 24 + gthy * 12) as u32;
            for b in 0..4 {
                res.push((format!("OPAD_X{}Y{}", opx, opy + 2 * (3 - b)), format!("MGTTXN{}_{}", b, self.bank), GtPin::TxN, b));
                res.push((format!("OPAD_X{}Y{}", opx, opy + 2 * (3 - b) + 1), format!("MGTTXP{}_{}", b, self.bank), GtPin::TxP, b));
                res.push((format!("IPAD_X{}Y{}", ipx, ipy + 6 + 2 * (3 - b)), format!("MGTRXN{}_{}", b, self.bank), GtPin::RxN, b));
                res.push((format!("IPAD_X{}Y{}", ipx, ipy + 6 + 2 * (3 - b) + 1), format!("MGTRXP{}_{}", b, self.bank), GtPin::RxP, b));
            }
            res.push((format!("IPAD_X{}Y{}", ipx, ipy - 9), format!("MGTREFCLKN_{}", self.bank), GtPin::ClkN, 0));
            res.push((format!("IPAD_X{}Y{}", ipx, ipy - 8), format!("MGTREFCLKP_{}", self.bank), GtPin::ClkP, 0));
        } else {
            let opy = self.gy * 8;
            let ipy = self.gy * 24;
            for b in 0..4 {
                res.push((format!("OPAD_X{}Y{}", opx, opy + 2 * b), format!("MGTTXN{}_{}", b, self.bank), GtPin::TxN, b));
                res.push((format!("OPAD_X{}Y{}", opx, opy + 2 * b + 1), format!("MGTTXP{}_{}", b, self.bank), GtPin::TxP, b));
                res.push((format!("IPAD_X{}Y{}", ipx, ipy + 6 * b), format!("MGTRXN{}_{}", b, self.bank), GtPin::RxN, b));
                res.push((format!("IPAD_X{}Y{}", ipx, ipy + 6 * b + 1), format!("MGTRXP{}_{}", b, self.bank), GtPin::RxP, b));
            }
            for b in 0..2 {
                res.push((format!("IPAD_X{}Y{}", ipx, ipy + 10 - 2 * b), format!("MGTREFCLK{}P_{}", b, self.bank), GtPin::ClkP, b));
                res.push((format!("IPAD_X{}Y{}", ipx, ipy + 11 - 2 * b), format!("MGTREFCLK{}N_{}", b, self.bank), GtPin::ClkN, b));
            }
        }
        res
    }
}

impl Grid {
    pub fn get_io(&self) -> Vec<Io> {
        let mut res = Vec::new();
        let mut iox = 0;
        for ioc in 0..4 {
            if let Some(col) = self.cols_io[ioc as usize] {
                for j in 0..self.regs {
                    let bank = 15 + j - self.reg_cfg + ioc as usize * 10;
                    for k in 0..40 {
                        res.push(Io {
                            col,
                            row: RowId::from_idx(j * 40 + k),
                            ioc,
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

    pub fn get_gt(&self, disabled: &BTreeSet<DisabledPart>) -> Vec<Gt> {
        let mut res = Vec::new();
        let mut gy = 0;
        for i in 0..self.regs {
            if disabled.contains(&DisabledPart::Virtex6GtxRow(i as u32)) {
                continue;
            }
            let is_gth = i >= self.reg_gth_start;
            if self.has_left_gt() {
                let bank = 105 + i - self.reg_cfg;
                res.push(Gt {
                    col: self.columns.first_id().unwrap(),
                    row: RowId::from_idx(i * 40),
                    gtc: 0,
                    gy,
                    bank: bank as u32,
                    is_gth,
                });
            }
            if self.col_hard.is_some() {
                let bank = 115 + i - self.reg_cfg;
                res.push(Gt {
                    col: self.columns.last_id().unwrap(),
                    row: RowId::from_idx(i * 40),
                    gtc: 1,
                    gy,
                    bank: bank as u32,
                    is_gth,
                });
            }
            gy += 1;
        }
        res
    }

    pub fn has_left_gt(&self) -> bool {
        *self.columns.first().unwrap() == ColumnKind::Gt
    }

    pub fn get_sysmon_pads(&self, disabled: &BTreeSet<DisabledPart>) -> Vec<(String, SysMonPin)> {
        let mut res = Vec::new();
        if self.col_hard.is_none() {
            res.push((format!("IPAD_X0Y0"), SysMonPin::VP));
            res.push((format!("IPAD_X0Y1"), SysMonPin::VN));
        } else {
            let mut ipy = 6;
            for i in 0..self.reg_cfg {
                if !disabled.contains(&DisabledPart::Virtex6GtxRow(i as u32)) {
                    ipy += 24;
                }
            }
            if self.has_left_gt() {
                res.push((format!("IPAD_X1Y{}", ipy), SysMonPin::VP));
                res.push((format!("IPAD_X1Y{}", ipy+1), SysMonPin::VN));
            } else {
                res.push((format!("IPAD_X0Y{}", ipy), SysMonPin::VP));
                res.push((format!("IPAD_X0Y{}", ipy+1), SysMonPin::VN));
            }
        }
        res
    }

    pub fn expand_grid<'a>(&self, db: &'a int::IntDb) -> eint::ExpandedGrid<'a> {
        let mut egrid = eint::ExpandedGrid::new(db);
        egrid.tie_kind = Some("TIEOFF".to_string());
        egrid.tie_pin_gnd = Some("HARD0".to_string());
        egrid.tie_pin_vcc = Some("HARD1".to_string());
        let slrid = egrid.tiles.push(Array2::default([self.regs * 40, self.columns.len()]));
        let mut grid = egrid.slr_mut(slrid);

        let mut tie_x = 0;
        for (col, &kind) in &self.columns {
            for row in grid.rows() {
                let x = col.to_idx();
                let y = row.to_idx();
                grid.fill_tile((col, row), "INT", "INT", format!("INT_X{x}Y{y}"));
                let tile = &mut grid[(col, row)];
                tile.nodes[0].tie_name = Some(format!("TIEOFF_X{tie_x}Y{y}"));
                match kind {
                    ColumnKind::ClbLL => (),
                    ColumnKind::ClbLM => (),
                    ColumnKind::Io if col < self.col_cfg => {
                        tile.add_intf(
                            db.get_intf("INTF"),
                            format!("IOI_L_INT_INTERFACE_X{x}Y{y}"),
                            db.get_intf_naming("INTF.IOI_L"),
                        );
                    }
                    ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::Io | ColumnKind::Cmt => {
                        tile.add_intf(
                            db.get_intf("INTF"),
                            format!("INT_INTERFACE_X{x}Y{y}"),
                            db.get_intf_naming("INTF"),
                        );
                    }
                    ColumnKind::Gt => {
                        if x == 0 {
                            tile.add_intf(
                                db.get_intf("INTF.DELAY"),
                                format!("GT_L_INT_INTERFACE_X{x}Y{y}"),
                                db.get_intf_naming("INTF.GT_L"),
                            );
                        } else {
                            tile.add_intf(
                                db.get_intf("INTF.DELAY"),
                                format!("GTX_INT_INTERFACE_X{x}Y{y}"),
                                db.get_intf_naming("INTF.GTX"),
                            );
                        }
                    }
                }
            }
            tie_x += 1;
            if kind == ColumnKind::Dsp {
                tie_x += 1;
            }
        }

        let row_b = RowId::from_idx(self.reg_cfg * 40 - 40);
        let row_t = RowId::from_idx(self.reg_cfg * 40 + 40);
        grid.nuke_rect(self.col_cfg - 6, row_b, 6, 80);
        for dx in 0..6 {
            let col = self.col_cfg - 6 + dx;
            if row_b.to_idx() != 0 {
                grid.fill_term_anon((col, row_b - 1), "N");
            }
            if row_t.to_idx() != self.regs * 40 {
                grid.fill_term_anon((col, row_t), "S");
            }
        }

        if let Some(ref col_hard) = self.col_hard {
            let col = col_hard.col;
            let x = col.to_idx();
            for &br in &col_hard.rows_emac {
                for dy in 0..10 {
                    let row = br + dy;
                    let y = row.to_idx();
                    let tile = &mut grid[(col, row)];
                    tile.intfs.clear();
                    tile.add_intf(
                        db.get_intf("INTF.DELAY"),
                        format!("EMAC_INT_INTERFACE_X{x}Y{y}"),
                        db.get_intf_naming("INTF.EMAC"),
                    );
                }
            }
            for &br in &col_hard.rows_pcie {
                grid.nuke_rect(col - 1, br, 2, 20);
                for dy in 0..20 {
                    let row = br + dy;
                    let y = row.to_idx();
                    grid[(col - 3, row)].add_intf(
                        db.get_intf("INTF.DELAY"),
                        format!("PCIE_INT_INTERFACE_L_X{xx}Y{y}", xx = x - 3),
                        db.get_intf_naming("INTF.PCIE_L"),
                    );
                    grid[(col - 2, row)].add_intf(
                        db.get_intf("INTF.DELAY"),
                        format!("PCIE_INT_INTERFACE_R_X{xx}Y{y}", xx = x - 2),
                        db.get_intf_naming("INTF.PCIE_R"),
                    );
                }
                if br.to_idx() != 0 {
                    grid.fill_term_anon((col - 1, br - 1), "N");
                    grid.fill_term_anon((col, br - 1), "N");
                }
                grid.fill_term_anon((col - 1, br + 20), "S");
                grid.fill_term_anon((col, br + 20), "S");
            }
        }

        let row_b = grid.rows().next().unwrap();
        let row_t = grid.rows().next_back().unwrap();
        for col in grid.cols() {
            if !grid[(col, row_b)].nodes.is_empty() {
                grid.fill_term_anon((col, row_b), "S.HOLE");
            }
            if !grid[(col, row_t)].nodes.is_empty() {
                grid.fill_term_anon((col, row_t), "N.HOLE");
            }
        }
        let col_l = grid.cols().next().unwrap();
        let col_r = grid.cols().next_back().unwrap();
        for row in grid.rows() {
            grid.fill_term_anon((col_l, row), "W");
            grid.fill_term_anon((col_r, row), "E");
        }

        grid.fill_main_passes();

        egrid
    }
}
