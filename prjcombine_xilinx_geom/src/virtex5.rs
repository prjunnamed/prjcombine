use std::collections::BTreeSet;
use serde::{Serialize, Deserialize};
use super::{GtPin, SysMonPin, CfgPin, ColId, RowId, int, eint};
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
        let mut egrid = eint::ExpandedGrid::new(db);
        egrid.tie_kind = Some("TIEOFF".to_string());
        egrid.tie_pin_pullup = Some("KEEP1".to_string());
        egrid.tie_pin_gnd = Some("HARD0".to_string());
        egrid.tie_pin_vcc = Some("HARD1".to_string());
        let (_, mut grid) = egrid.add_slr(self.columns.len(), self.regs * 20);

        for (col, &kind) in &self.columns {
            for row in grid.rows() {
                let x = col.to_idx();
                let y = row.to_idx();
                grid.fill_tile((col, row), "INT", "INT", format!("INT_X{x}Y{y}"));
                let tile = &mut grid[(col, row)];
                tile.nodes[0].tie_name = Some(format!("TIEOFF_X{x}Y{y}"));
                match kind {
                    ColumnKind::ClbLL => (),
                    ColumnKind::ClbLM => (),
                    ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::Io => {
                        tile.add_intf(
                            db.get_intf("INTF"),
                            format!("INT_INTERFACE_X{x}Y{y}"),
                            db.get_intf_naming("INTF"),
                        );
                    }
                    ColumnKind::Gtp | ColumnKind::Gtx if col.to_idx() != 0 => {
                        tile.add_intf(
                            db.get_intf("INTF.DELAY"),
                            format!("GTP_INT_INTERFACE_X{x}Y{y}"),
                            db.get_intf_naming("INTF.GTP"),
                        );
                    }
                    ColumnKind::Gtp | ColumnKind::Gtx => {
                        tile.add_intf(
                            db.get_intf("INTF.DELAY"),
                            format!("GTX_LEFT_INT_INTERFACE_X{x}Y{y}"),
                            db.get_intf_naming("INTF.GTX_LEFT"),
                        );
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
                for dy in 0..40 {
                    let row = br + dy;
                    let y = row.to_idx();
                    let tile = &mut grid[(col, row)];
                    tile.intfs.clear();
                    tile.add_intf(
                        db.get_intf("INTF.DELAY"),
                        format!("PCIE_INT_INTERFACE_X{x}Y{y}"),
                        db.get_intf_naming("INTF.PCIE"),
                    );
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
                grid.fill_term_pair_dbuf((col_l, row), (col_r, row), 
                    db.get_term("PPC.E"),
                    db.get_term("PPC.W"),
                    tile_l,
                    tile_r,
                    db.get_term_naming("PPC.E"),
                    db.get_term_naming("PPC.W"),
                );
                let tile = &mut grid[(col_l, row)];
                tile.intfs.clear();
                tile.add_intf(
                    db.get_intf("INTF.DELAY"),
                    format!("PPC_L_INT_INTERFACE_X{xl}Y{y}"),
                    db.get_intf_naming("INTF.PPC_L"),
                );
                let tile = &mut grid[(col_r, row)];
                tile.intfs.clear();
                tile.add_intf(
                    db.get_intf("INTF.DELAY"),
                    format!("PPC_R_INT_INTERFACE_X{xr}Y{y}"),
                    db.get_intf_naming("INTF.PPC_R"),
                );
            }
            let row_b = br - 1;
            let row_t = br + 40;
            let yb = row_b.to_idx();
            let yt = row_t.to_idx();
            for dx in 1..13 {
                let col = bc + dx;
                let x = col.to_idx();
                grid.fill_term_tile((col, row_b), "TERM.N.PPC", "TERM.N.PPC", format!("PPC_B_TERM_X{x}Y{yb}"));
                grid.fill_term_tile((col, row_t), "TERM.S.PPC", "TERM.S.PPC", format!("PPC_T_TERM_X{x}Y{yt}"));
            }
        }

        let row_b = grid.rows().next().unwrap();
        let row_t = grid.rows().next_back().unwrap();
        let term_n = db.get_term("MAIN.NHOLE.N");
        let term_s = db.get_term("MAIN.NHOLE.S");
        for col in grid.cols() {
            grid.fill_term_anon((col, row_b), "TERM.S.HOLE");
            grid.fill_term_anon((col, row_t), "TERM.N.HOLE");
            grid.fill_term_pair_anon((col, row_t - 1), (col, row_t), term_n, term_s);
        }
        let col_l = grid.cols().next().unwrap();
        let col_r = grid.cols().next_back().unwrap();
        let xl = col_l.to_idx();
        let xr = col_r.to_idx();
        for row in grid.rows() {
            let y = row.to_idx();
            if self.columns[col_l] == ColumnKind::Gtx {
                grid.fill_term_tile((col_l, row), "TERM.W", "TERM.W", format!("GTX_L_TERM_INT_X{xl}Y{y}"));
            } else {
                grid.fill_term_tile((col_l, row), "TERM.W", "TERM.W", format!("L_TERM_INT_X{xl}Y{y}"));
            }
            if matches!(self.columns[col_r], ColumnKind::Gtp | ColumnKind::Gtx) {
                grid.fill_term_tile((col_r, row), "TERM.E", "TERM.E", format!("R_TERM_INT_X{xr}Y{y}"));
            } else {
                grid.fill_term_anon((col_r, row), "TERM.E.HOLE");
            }
        }

        let term_w = db.get_term("INT_BUFS.W");
        let term_e = db.get_term("INT_BUFS.E");
        let naming_w = db.get_term_naming("INT_BUFS.W");
        let naming_e = db.get_term_naming("INT_BUFS.E");
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
                grid.fill_term_pair_dbuf(
                    (col, row),
                    (col + 1, row),
                    term_e,
                    term_w,
                    tile_l,
                    tile_r,
                    naming_e,
                    naming_w,
                );
            }
        }

        grid.fill_main_passes();

        for col in grid.cols() {
            for row in grid.rows() {
                let crow = RowId::from_idx(row.to_idx() / 20 * 20 + 10);
                grid[(col, row)].clkroot = (col, crow);
            }
        }

        egrid
    }
}
