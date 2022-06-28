use std::collections::BTreeSet;
use serde::{Serialize, Deserialize};
use super::{GtPin, SysMonPin, CfgPin, ColId, RowId, int, eint};
use ndarray::Array2;
use prjcombine_entity::{EntityVec, EntityId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub columns: EntityVec<ColId, ColumnKind>,
    pub cols_vbrk: BTreeSet<ColId>,
    pub cols_mgt_buf: BTreeSet<ColId>,
    pub col_hard: Option<HardColumn>,
    pub cols_io: [Option<ColId>; 3],
    pub regs: usize,
    pub reg_cfg: usize,
    pub holes_ppc: Vec<(ColId, RowId)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKind {
    ClbLL,
    ClbLM,
    Bram,
    Dsp,
    Io,
    Gtp,
    Gtx,
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
    pub bel: u32,
    pub ioc: u32,
    pub bank: u32,
    pub bbel: u32,
}

impl Io {
    pub fn iob_name(&self) -> String {
        let x = self.ioc;
        let y = self.row.to_idx() as u32 * 2 + self.bel;
        format!("IOB_X{x}Y{y}")
    }
    pub fn is_cc(&self) -> bool {
        matches!(self.row.to_idx() % 20, 8..=11)
    }
    pub fn is_gc(&self) -> bool {
        matches!(self.bank, 3 | 4)
    }
    pub fn is_vref(&self) -> bool {
        self.row.to_idx() % 10 == 5 && self.bel == 0
    }
    pub fn is_vr(&self) -> bool {
        match self.bank {
            1 | 2 => false,
            3 => self.row.to_idx() % 10 == 7,
            4 => self.row.to_idx() % 10 == 2,
            _ => self.row.to_idx() % 20 == 7,
        }
    }
    pub fn sm_pair(&self) -> Option<u32> {
        match (self.bank, self.row.to_idx() % 20) {
            (13, 10) => Some(0),
            (13, 11) => Some(1),
            (13, 12) => Some(2),
            (13, 13) => Some(3),
            (13, 14) => Some(4),
            (13, 16) => Some(5),
            (13, 17) => Some(6),
            (13, 18) => Some(7),
            (13, 19) => Some(8),
            (11, 0) => Some(9),
            (11, 1) => Some(10),
            (11, 2) => Some(11),
            (11, 3) => Some(12),
            (11, 4) => Some(13),
            (11, 8) => Some(14),
            (11, 9) => Some(15),
            _ => None,
        }
    }
    pub fn get_cfg(&self) -> Option<CfgPin> {
        match (self.bank, self.row.to_idx() % 20, self.bel) {
            (4, 16, 0) => Some(CfgPin::Data(8)),
            (4, 16, 1) => Some(CfgPin::Data(9)),
            (4, 17, 0) => Some(CfgPin::Data(10)),
            (4, 17, 1) => Some(CfgPin::Data(11)),
            (4, 18, 0) => Some(CfgPin::Data(12)),
            (4, 18, 1) => Some(CfgPin::Data(13)),
            (4, 19, 0) => Some(CfgPin::Data(14)),
            (4, 19, 1) => Some(CfgPin::Data(15)),
            (2, 0, 0) => Some(CfgPin::Data(0)),
            (2, 0, 1) => Some(CfgPin::Data(1)),
            (2, 1, 0) => Some(CfgPin::Data(2)),
            (2, 1, 1) => Some(CfgPin::Data(3)),
            (2, 2, 0) => Some(CfgPin::Data(4)),
            (2, 2, 1) => Some(CfgPin::Data(5)),
            (2, 3, 0) => Some(CfgPin::Data(6)),
            (2, 3, 1) => Some(CfgPin::Data(7)),
            (2, 4, 0) => Some(CfgPin::CsoB),
            (2, 4, 1) => Some(CfgPin::FweB),
            (2, 5, 0) => Some(CfgPin::FoeB),
            (2, 5, 1) => Some(CfgPin::FcsB),
            (2, 6, 0) => Some(CfgPin::Addr(20)),
            (2, 6, 1) => Some(CfgPin::Addr(21)),
            (2, 7, 0) => Some(CfgPin::Addr(22)),
            (2, 7, 1) => Some(CfgPin::Addr(23)),
            (2, 8, 0) => Some(CfgPin::Addr(24)),
            (2, 8, 1) => Some(CfgPin::Addr(25)),
            (2, 9, 0) => Some(CfgPin::Rs(0)),
            (2, 9, 1) => Some(CfgPin::Rs(1)),
            (1, 10, 0) => Some(CfgPin::Data(16)),
            (1, 10, 1) => Some(CfgPin::Data(17)),
            (1, 11, 0) => Some(CfgPin::Data(18)),
            (1, 11, 1) => Some(CfgPin::Data(19)),
            (1, 12, 0) => Some(CfgPin::Data(20)),
            (1, 12, 1) => Some(CfgPin::Data(21)),
            (1, 13, 0) => Some(CfgPin::Data(22)),
            (1, 13, 1) => Some(CfgPin::Data(23)),
            (1, 14, 0) => Some(CfgPin::Data(24)),
            (1, 14, 1) => Some(CfgPin::Data(25)),
            (1, 15, 0) => Some(CfgPin::Data(26)),
            (1, 15, 1) => Some(CfgPin::Data(27)),
            (1, 16, 0) => Some(CfgPin::Data(28)),
            (1, 16, 1) => Some(CfgPin::Data(29)),
            (1, 17, 0) => Some(CfgPin::Data(30)),
            (1, 17, 1) => Some(CfgPin::Data(31)),
            (1, 18, 0) => Some(CfgPin::Addr(16)),
            (1, 18, 1) => Some(CfgPin::Addr(17)),
            (1, 19, 0) => Some(CfgPin::Addr(18)),
            (1, 19, 1) => Some(CfgPin::Addr(19)),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Gt {
    pub col: ColId,
    pub row: RowId,
    pub gtc: u32,
    pub bank: u32,
    pub is_gtx: bool,
}

impl Gt {
    pub fn get_pads(&self, grid: &Grid) -> Vec<(String, String, GtPin, u32)> {
        let reg = self.row.to_idx() / 20;
        let ipy = if reg < grid.reg_cfg {
            reg * 6
        } else {
            6 + reg * 6
        };
        let opy = reg * 4;
        let (ipx, opx) = if grid.has_left_gt() {
            (self.gtc * 2, self.gtc)
        } else {
            (1, 0)
        };
        vec![
            (format!("IPAD_X{}Y{}", ipx, ipy), format!("MGTRXN0_{}", self.bank), GtPin::RxN, 0),
            (format!("IPAD_X{}Y{}", ipx, ipy+1), format!("MGTRXP0_{}", self.bank), GtPin::RxP, 0),
            (format!("IPAD_X{}Y{}", ipx, ipy+2), format!("MGTRXN1_{}", self.bank), GtPin::RxN, 1),
            (format!("IPAD_X{}Y{}", ipx, ipy+3), format!("MGTRXP1_{}", self.bank), GtPin::RxP, 1),
            (format!("IPAD_X{}Y{}", ipx, ipy+4), format!("MGTREFCLKN_{}", self.bank), GtPin::ClkN, 0),
            (format!("IPAD_X{}Y{}", ipx, ipy+5), format!("MGTREFCLKP_{}", self.bank), GtPin::ClkP, 0),
            (format!("OPAD_X{}Y{}", opx, opy), format!("MGTTXN0_{}", self.bank), GtPin::TxN, 0),
            (format!("OPAD_X{}Y{}", opx, opy+1), format!("MGTTXP0_{}", self.bank), GtPin::TxP, 0),
            (format!("OPAD_X{}Y{}", opx, opy+2), format!("MGTTXN1_{}", self.bank), GtPin::TxN, 1),
            (format!("OPAD_X{}Y{}", opx, opy+3), format!("MGTTXP1_{}", self.bank), GtPin::TxP, 1),
        ]
    }
}

impl Grid {
    pub fn get_io(&self) -> Vec<Io> {
        let mut res = Vec::new();
        // left column
        for i in 0..self.regs {
            let bank = if i < self.reg_cfg {
                13 + (self.reg_cfg - i - 1) * 4
            } else {
                11 + (i - self.reg_cfg) * 4
            };
            for j in 0..20 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.cols_io[0].unwrap(),
                        row: RowId::from_idx(i * 20 + j),
                        ioc: 0,
                        bel: k,
                        bank: bank as u32,
                        bbel: (19 - j as u32) * 2 + k,
                    });
                }
            }
        }
        // center column
        // bottom banks
        if self.reg_cfg > 3 {
            for i in 0..(self.reg_cfg - 3) {
                let bank = 6 + (self.reg_cfg - 4 - i) * 2;
                for j in 0..20 {
                    for k in 0..2 {
                        res.push(Io {
                            col: self.cols_io[1].unwrap(),
                            row: RowId::from_idx(i * 20 + j),
                            ioc: 1,
                            bel: k,
                            bank: bank as u32,
                            bbel: (19 - j as u32) * 2 + k,
                        });
                    }
                }
            }
        }
        // special banks 4, 2, 1, 3
        for (bank, base) in [
            (4, self.reg_cfg * 20 - 30),
            (2, self.reg_cfg * 20 - 20),
            (1, self.reg_cfg * 20 + 10),
            (3, self.reg_cfg * 20 + 20),
        ] {
            for j in 0..10 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.cols_io[1].unwrap(),
                        row: RowId::from_idx(base + j),
                        ioc: 1,
                        bel: k,
                        bank,
                        bbel: (9 - j as u32) * 2 + k,
                    });
                }
            }
        }
        // top banks
        if (self.regs - self.reg_cfg) > 3 {
            for i in (self.reg_cfg + 3)..self.regs {
                let bank = 5 + (i - self.reg_cfg - 3) * 2;
                for j in 0..20 {
                    for k in 0..2 {
                        res.push(Io {
                            col: self.cols_io[1].unwrap(),
                            row: RowId::from_idx(i * 20 + j),
                            ioc: 1,
                            bel: k,
                            bank: bank as u32,
                            bbel: (19 - j as u32) * 2 + k,
                        });
                    }
                }
            }
        }
        // right column
        if let Some(col) = self.cols_io[2] {
            for i in 0..self.regs {
                let bank = if i < self.reg_cfg {
                    14 + (self.reg_cfg - i - 1) * 4
                } else {
                    12 + (i - self.reg_cfg) * 4
                };
                for j in 0..20 {
                    for k in 0..2 {
                        res.push(Io {
                            col,
                            row: RowId::from_idx(i * 20 + j),
                            ioc: 2,
                            bel: k,
                            bank: bank as u32,
                            bbel: (19 - j as u32) * 2 + k,
                        });
                    }
                }
            }
        }
        res
    }

    pub fn get_gt(&self) -> Vec<Gt> {
        let mut res = Vec::new();
        if self.has_left_gt() {
            for i in 0..self.regs {
                let bank = if i < self.reg_cfg {
                    113 + (self.reg_cfg - i - 1) * 4
                } else {
                    111 + (i - self.reg_cfg) * 4
                };
                res.push(Gt {
                    col: self.columns.first_id().unwrap(),
                    row: RowId::from_idx(i * 20),
                    gtc: 0,
                    bank: bank as u32,
                    is_gtx: true,
                });
            }
        }
        if self.col_hard.is_some() {
            let is_gtx = *self.columns.last().unwrap() == ColumnKind::Gtx;
            for i in 0..self.regs {
                let bank = if i < self.reg_cfg {
                    114 + (self.reg_cfg - i - 1) * 4
                } else {
                    112 + (i - self.reg_cfg) * 4
                };
                res.push(Gt {
                    col: self.columns.last_id().unwrap(),
                    row: RowId::from_idx(i * 20),
                    gtc: 1,
                    bank: bank as u32,
                    is_gtx,
                });
            }
        }
        res
    }

    pub fn has_left_gt(&self) -> bool {
        *self.columns.first().unwrap() == ColumnKind::Gtx
    }

    pub fn get_sysmon_pads(&self) -> Vec<(String, SysMonPin)> {
        let mut res = Vec::new();
        if self.has_left_gt() {
            let ipy = 6 * self.reg_cfg;
            res.push((format!("IPAD_X1Y{}", ipy), SysMonPin::VP));
            res.push((format!("IPAD_X1Y{}", ipy+1), SysMonPin::VN));
        } else if self.col_hard.is_some() {
            let ipy = 6 * self.reg_cfg;
            res.push((format!("IPAD_X0Y{}", ipy), SysMonPin::VP));
            res.push((format!("IPAD_X0Y{}", ipy+1), SysMonPin::VN));
        } else {
            res.push((format!("IPAD_X0Y0"), SysMonPin::VP));
            res.push((format!("IPAD_X0Y1"), SysMonPin::VN));
        }
        res
    }

    pub fn expand_grid<'a>(&self, db: &'a int::IntDb) -> eint::ExpandedGrid<'a> {
        let mut egrid = eint::ExpandedGrid {
            db,
            tie_kind: Some("TIEOFF".to_string()),
            tie_pin_pullup: Some("KEEP1".to_string()),
            tie_pin_gnd: Some("HARD0".to_string()),
            tie_pin_vcc: Some("HARD1".to_string()),
            tiles: Default::default(),
        };
        let slrid = egrid.tiles.push(Array2::default([self.regs * 20, self.columns.len()]));
        let mut grid = egrid.slr_mut(slrid);

        for (col, &kind) in &self.columns {
            for row in grid.rows() {
                let x = col.to_idx();
                let y = row.to_idx();
                grid.fill_tile((col, row), "INT", "NODE.INT", format!("INT_X{x}Y{y}"));
                grid.tile_mut((col, row)).tie_name = Some(format!("TIEOFF_X{x}Y{y}"));
                match kind {
                    ColumnKind::ClbLL => (),
                    ColumnKind::ClbLM => (),
                    ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::Io => {
                        grid.tile_mut((col, row)).intf = Some(eint::ExpandedTileIntf {
                            kind: db.get_intf("INTF"),
                            name: format!("INT_INTERFACE_X{x}Y{y}"),
                            naming_int: db.get_naming("INTF"),
                            naming_buf: None,
                            naming_site: Some(db.get_naming("INTF.SITE")),
                            naming_delay: None,
                        });
                    }
                    ColumnKind::Gtp | ColumnKind::Gtx if col.to_idx() != 0 => {
                        grid.tile_mut((col, row)).intf = Some(eint::ExpandedTileIntf {
                            kind: db.get_intf("INTF.DELAY"),
                            name: format!("GTP_INT_INTERFACE_X{x}Y{y}"),
                            naming_int: db.get_naming("INTF.GTP"),
                            naming_buf: None,
                            naming_site: Some(db.get_naming("INTF.GTP.SITE")),
                            naming_delay: Some(db.get_naming("INTF.GTP.DELAY")),
                        });
                    }
                    ColumnKind::Gtp | ColumnKind::Gtx => {
                        grid.tile_mut((col, row)).intf = Some(eint::ExpandedTileIntf {
                            kind: db.get_intf("INTF.DELAY"),
                            name: format!("GTX_LEFT_INT_INTERFACE_X{x}Y{y}"),
                            naming_int: db.get_naming("INTF.GTX_LEFT"),
                            naming_buf: None,
                            naming_site: Some(db.get_naming("INTF.GTX_LEFT.SITE")),
                            naming_delay: Some(db.get_naming("INTF.GTX_LEFT.DELAY")),
                        });
                    }
                }
            }
        }

        if let Some(ref col_hard) = self.col_hard {
            let col = col_hard.col;
            let x = col.to_idx();
            for &br in &col_hard.rows_emac {
                for dy in 0..10 {
                    let row = br + dy;
                    let y = row.to_idx();
                    grid.tile_mut((col, row)).intf = Some(eint::ExpandedTileIntf {
                        kind: db.get_intf("INTF.DELAY"),
                        name: format!("EMAC_INT_INTERFACE_X{x}Y{y}"),
                        naming_int: db.get_naming("INTF.EMAC"),
                        naming_buf: None,
                        naming_site: Some(db.get_naming("INTF.EMAC.SITE")),
                        naming_delay: Some(db.get_naming("INTF.EMAC.DELAY")),
                    });
                }
            }
            for &br in &col_hard.rows_pcie {
                for dy in 0..40 {
                    let row = br + dy;
                    let y = row.to_idx();
                    grid.tile_mut((col, row)).intf = Some(eint::ExpandedTileIntf {
                        kind: db.get_intf("INTF.DELAY"),
                        name: format!("PCIE_INT_INTERFACE_X{x}Y{y}"),
                        naming_int: db.get_naming("INTF.PCIE"),
                        naming_buf: None,
                        naming_site: Some(db.get_naming("INTF.PCIE.SITE")),
                        naming_delay: Some(db.get_naming("INTF.PCIE.DELAY")),
                    });
                }
            }
        }

        for &(bc, br) in &self.holes_ppc {
            grid.nuke_rect(bc + 1, br, 12, 40);
            let col_l = bc;
            let col_r = bc + 13;
            let xl = col_l.to_idx();
            let xr = col_r.to_idx();
            for dy in 0..40 {
                let row = br + dy;
                let y = row.to_idx();
                // sigh.
                let rxr = 53;
                let ry = y / 10 * 11 + y % 10 + 1;
                let tile_l = format!("L_TERM_PPC_X{xl}Y{y}");
                let tile_r = format!("R_TERM_PPC_X{rxr}Y{ry}");
                grid.fill_pass_pair(eint::ExpandedTilePass {
                    target: (col_r, row),
                    kind: db.get_pass("PPC.E"),
                    tile: Some(tile_l.clone()),
                    naming_near: Some(db.get_naming("TERM.PPC.E")),
                    naming_far: Some(db.get_naming("TERM.PPC.E.FAR")),
                    tile_far: Some(tile_r.clone()),
                    naming_far_out: Some(db.get_naming("TERM.PPC.W.OUT")),
                    naming_far_in: Some(db.get_naming("TERM.PPC.W.IN")),
                }, eint::ExpandedTilePass {
                    target: (col_l, row),
                    kind: db.get_pass("PPC.W"),
                    tile: Some(tile_r),
                    naming_near: Some(db.get_naming("TERM.PPC.W")),
                    naming_far: Some(db.get_naming("TERM.PPC.W.FAR")),
                    tile_far: Some(tile_l),
                    naming_far_out: Some(db.get_naming("TERM.PPC.E.OUT")),
                    naming_far_in: Some(db.get_naming("TERM.PPC.E.IN")),
                });
                grid.tile_mut((col_l, row)).intf = Some(eint::ExpandedTileIntf {
                    kind: db.get_intf("INTF.DELAY"),
                    name: format!("PPC_L_INT_INTERFACE_X{xl}Y{y}"),
                    naming_int: db.get_naming("INTF.PPC_L"),
                    naming_buf: None,
                    naming_site: Some(db.get_naming("INTF.PPC_L.SITE")),
                    naming_delay: Some(db.get_naming("INTF.PPC_L.DELAY")),
                });
                grid.tile_mut((col_r, row)).intf = Some(eint::ExpandedTileIntf {
                    kind: db.get_intf("INTF.DELAY"),
                    name: format!("PPC_R_INT_INTERFACE_X{xr}Y{y}"),
                    naming_int: db.get_naming("INTF.PPC_R"),
                    naming_buf: None,
                    naming_site: Some(db.get_naming("INTF.PPC_R.SITE")),
                    naming_delay: Some(db.get_naming("INTF.PPC_R.DELAY")),
                });
            }
            let row_b = br - 1;
            let row_t = br + 40;
            let yb = row_b.to_idx();
            let yt = row_t.to_idx();
            for dx in 1..13 {
                let col = bc + dx;
                let x = col.to_idx();
                grid.fill_term_tile((col, row_b), "N.PPC", "TERM.PPC.N.OUT", Some("TERM.PPC.N.IN"), format!("PPC_B_TERM_X{x}Y{yb}"));
                grid.fill_term_tile((col, row_t), "S.PPC", "TERM.PPC.S.OUT", Some("TERM.PPC.S.IN"), format!("PPC_T_TERM_X{x}Y{yt}"));
            }
        }

        let row_b = grid.rows().next().unwrap();
        let row_t = grid.rows().next_back().unwrap();
        let pass_n = db.get_pass("MAIN.NHOLE.N");
        let pass_s = db.get_pass("MAIN.NHOLE.S");
        for col in grid.cols() {
            grid.fill_term_anon((col, row_b), "S.HOLE");
            grid.fill_term_anon((col, row_t), "N.HOLE");
            grid.fill_pass_anon((col, row_t - 1), (col, row_t), pass_n, pass_s);
        }
        let col_l = grid.cols().next().unwrap();
        let col_r = grid.cols().next_back().unwrap();
        let xl = col_l.to_idx();
        let xr = col_r.to_idx();
        for row in grid.rows() {
            let y = row.to_idx();
            if self.columns[col_l] == ColumnKind::Gtx {
                grid.fill_term_tile((col_l, row), "W", "TERM.W.OUT", Some("TERM.W.IN"), format!("GTX_L_TERM_INT_X{xl}Y{y}"));
            } else {
                grid.fill_term_tile((col_l, row), "W", "TERM.W.OUT", Some("TERM.W.IN"), format!("L_TERM_INT_X{xl}Y{y}"));
            }
            if matches!(self.columns[col_r], ColumnKind::Gtp | ColumnKind::Gtx) {
                grid.fill_term_tile((col_r, row), "E", "TERM.E.OUT", Some("TERM.E.IN"), format!("R_TERM_INT_X{xr}Y{y}"));
            } else {
                grid.fill_term_anon((col_r, row), "E.HOLE");
            }
        }

        let pass_w = db.get_pass("INT_BUFS.W");
        let pass_e = db.get_pass("INT_BUFS.E");
        let naming_w = db.get_naming("INT_BUFS.W");
        let naming_wf = db.get_naming("INT_BUFS.W.FAR");
        let naming_wo = db.get_naming("INT_BUFS.W.OUT");
        let naming_e = db.get_naming("INT_BUFS.E");
        let naming_ef = db.get_naming("INT_BUFS.E.FAR");
        let naming_eo = db.get_naming("INT_BUFS.E.OUT");
        for (col, &cd) in &self.columns {
            if cd != ColumnKind::Io || col == col_l || col == col_r {
                continue;
            }
            for row in grid.rows() {
                let x = col.to_idx();
                let y = row.to_idx();
                let tile_l = format!("INT_BUFS_L_X{x}Y{y}");
                let mon = if self.columns[col_l] == ColumnKind::Gtx {"_MON"} else {""};
                let tile_r = format!("INT_BUFS_R{mon}_X{xx}Y{y}", xx = x + 1);
                grid.fill_pass_pair(eint::ExpandedTilePass {
                    target: (col + 1, row),
                    kind: pass_e,
                    tile: Some(tile_l.clone()),
                    naming_near: Some(naming_e),
                    naming_far: Some(naming_ef),
                    tile_far: Some(tile_r.clone()),
                    naming_far_out: Some(naming_wo),
                    naming_far_in: Some(naming_w),
                }, eint::ExpandedTilePass {
                    target: (col, row),
                    kind: pass_w,
                    tile: Some(tile_r),
                    naming_near: Some(naming_w),
                    naming_far: Some(naming_wf),
                    tile_far: Some(tile_l),
                    naming_far_out: Some(naming_eo),
                    naming_far_in: Some(naming_e),
                });
            }
        }

        grid.fill_main_passes();

        egrid
    }
}
