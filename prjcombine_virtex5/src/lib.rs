use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ColId, ExpandedGrid, RowId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    // ×32; high 16 bits are also low 16 bits of Addr
    // 0-2 double as FS
    Data(u8),
    Addr(u8), // ×26 total, but 0-15 are represented as Data(16-31)
    Rs(u8),   // ×2
    CsoB,
    FweB,
    FoeB, // doubles as MOSI
    FcsB,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CfgPin {
    Tck,
    Tdi,
    Tdo,
    Tms,
    Cclk,
    Done,
    ProgB,
    InitB,
    RdWrB,
    CsiB,
    Din,
    Dout,
    M0,
    M1,
    M2,
    HswapEn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SysMonPin {
    VP,
    VN,
    AVss,
    AVdd,
    VRefP,
    VRefN,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtPin {
    RxP(u8),
    RxN(u8),
    TxP(u8),
    TxN(u8),
    ClkP,
    ClkN,
    AVcc,
    AVccPll,
    VtRx,
    VtTx,
    RRef,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtRegion {
    All,
    L,
    R,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtRegionPin {
    AVttRxC,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BondPin {
    // bank, pin within bank
    Io(u32, u32),
    Nc,
    Gnd,
    Rsvd,
    VccInt,
    VccAux,
    VccO(u32),
    VccBatt,
    Cfg(CfgPin),
    Gt(u32, GtPin),
    GtRegion(GtRegion, GtRegionPin),
    Dxp,
    Dxn,
    SysMon(SysMonPin),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
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
    pub fn get_cfg(&self) -> Option<SharedCfgPin> {
        match (self.bank, self.row.to_idx() % 20, self.bel) {
            (4, 16, 0) => Some(SharedCfgPin::Data(8)),
            (4, 16, 1) => Some(SharedCfgPin::Data(9)),
            (4, 17, 0) => Some(SharedCfgPin::Data(10)),
            (4, 17, 1) => Some(SharedCfgPin::Data(11)),
            (4, 18, 0) => Some(SharedCfgPin::Data(12)),
            (4, 18, 1) => Some(SharedCfgPin::Data(13)),
            (4, 19, 0) => Some(SharedCfgPin::Data(14)),
            (4, 19, 1) => Some(SharedCfgPin::Data(15)),
            (2, 0, 0) => Some(SharedCfgPin::Data(0)),
            (2, 0, 1) => Some(SharedCfgPin::Data(1)),
            (2, 1, 0) => Some(SharedCfgPin::Data(2)),
            (2, 1, 1) => Some(SharedCfgPin::Data(3)),
            (2, 2, 0) => Some(SharedCfgPin::Data(4)),
            (2, 2, 1) => Some(SharedCfgPin::Data(5)),
            (2, 3, 0) => Some(SharedCfgPin::Data(6)),
            (2, 3, 1) => Some(SharedCfgPin::Data(7)),
            (2, 4, 0) => Some(SharedCfgPin::CsoB),
            (2, 4, 1) => Some(SharedCfgPin::FweB),
            (2, 5, 0) => Some(SharedCfgPin::FoeB),
            (2, 5, 1) => Some(SharedCfgPin::FcsB),
            (2, 6, 0) => Some(SharedCfgPin::Addr(20)),
            (2, 6, 1) => Some(SharedCfgPin::Addr(21)),
            (2, 7, 0) => Some(SharedCfgPin::Addr(22)),
            (2, 7, 1) => Some(SharedCfgPin::Addr(23)),
            (2, 8, 0) => Some(SharedCfgPin::Addr(24)),
            (2, 8, 1) => Some(SharedCfgPin::Addr(25)),
            (2, 9, 0) => Some(SharedCfgPin::Rs(0)),
            (2, 9, 1) => Some(SharedCfgPin::Rs(1)),
            (1, 10, 0) => Some(SharedCfgPin::Data(16)),
            (1, 10, 1) => Some(SharedCfgPin::Data(17)),
            (1, 11, 0) => Some(SharedCfgPin::Data(18)),
            (1, 11, 1) => Some(SharedCfgPin::Data(19)),
            (1, 12, 0) => Some(SharedCfgPin::Data(20)),
            (1, 12, 1) => Some(SharedCfgPin::Data(21)),
            (1, 13, 0) => Some(SharedCfgPin::Data(22)),
            (1, 13, 1) => Some(SharedCfgPin::Data(23)),
            (1, 14, 0) => Some(SharedCfgPin::Data(24)),
            (1, 14, 1) => Some(SharedCfgPin::Data(25)),
            (1, 15, 0) => Some(SharedCfgPin::Data(26)),
            (1, 15, 1) => Some(SharedCfgPin::Data(27)),
            (1, 16, 0) => Some(SharedCfgPin::Data(28)),
            (1, 16, 1) => Some(SharedCfgPin::Data(29)),
            (1, 17, 0) => Some(SharedCfgPin::Data(30)),
            (1, 17, 1) => Some(SharedCfgPin::Data(31)),
            (1, 18, 0) => Some(SharedCfgPin::Addr(16)),
            (1, 18, 1) => Some(SharedCfgPin::Addr(17)),
            (1, 19, 0) => Some(SharedCfgPin::Addr(18)),
            (1, 19, 1) => Some(SharedCfgPin::Addr(19)),
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
    pub fn get_pads(&self, grid: &Grid) -> Vec<(String, String, GtPin)> {
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
            (
                format!("IPAD_X{}Y{}", ipx, ipy),
                format!("MGTRXN0_{}", self.bank),
                GtPin::RxN(0),
            ),
            (
                format!("IPAD_X{}Y{}", ipx, ipy + 1),
                format!("MGTRXP0_{}", self.bank),
                GtPin::RxP(0),
            ),
            (
                format!("IPAD_X{}Y{}", ipx, ipy + 2),
                format!("MGTRXN1_{}", self.bank),
                GtPin::RxN(1),
            ),
            (
                format!("IPAD_X{}Y{}", ipx, ipy + 3),
                format!("MGTRXP1_{}", self.bank),
                GtPin::RxP(1),
            ),
            (
                format!("IPAD_X{}Y{}", ipx, ipy + 4),
                format!("MGTREFCLKN_{}", self.bank),
                GtPin::ClkN,
            ),
            (
                format!("IPAD_X{}Y{}", ipx, ipy + 5),
                format!("MGTREFCLKP_{}", self.bank),
                GtPin::ClkP,
            ),
            (
                format!("OPAD_X{}Y{}", opx, opy),
                format!("MGTTXN0_{}", self.bank),
                GtPin::TxN(0),
            ),
            (
                format!("OPAD_X{}Y{}", opx, opy + 1),
                format!("MGTTXP0_{}", self.bank),
                GtPin::TxP(0),
            ),
            (
                format!("OPAD_X{}Y{}", opx, opy + 2),
                format!("MGTTXN1_{}", self.bank),
                GtPin::TxN(1),
            ),
            (
                format!("OPAD_X{}Y{}", opx, opy + 3),
                format!("MGTTXP1_{}", self.bank),
                GtPin::TxP(1),
            ),
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
            res.push((format!("IPAD_X1Y{}", ipy + 1), SysMonPin::VN));
        } else if self.col_hard.is_some() {
            let ipy = 6 * self.reg_cfg;
            res.push((format!("IPAD_X0Y{}", ipy), SysMonPin::VP));
            res.push((format!("IPAD_X0Y{}", ipy + 1), SysMonPin::VN));
        } else {
            res.push(("IPAD_X0Y0".to_string(), SysMonPin::VP));
            res.push(("IPAD_X0Y1".to_string(), SysMonPin::VN));
        }
        res
    }

    pub fn expand_grid<'a>(&self, db: &'a IntDb) -> ExpandedGrid<'a> {
        let mut egrid = ExpandedGrid::new(db);
        egrid.tie_kind = Some("TIEOFF".to_string());
        egrid.tie_pin_pullup = Some("KEEP1".to_string());
        egrid.tie_pin_gnd = Some("HARD0".to_string());
        egrid.tie_pin_vcc = Some("HARD1".to_string());
        let (_, mut grid) = egrid.add_die(self.columns.len(), self.regs * 20);

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
                        tile.add_xnode(
                            db.get_node("INTF"),
                            &[&format!("INT_INTERFACE_X{x}Y{y}")],
                            db.get_node_naming("INTF"),
                            &[(col, row)],
                        );
                    }
                    ColumnKind::Gtp | ColumnKind::Gtx if col.to_idx() != 0 => {
                        tile.add_xnode(
                            db.get_node("INTF.DELAY"),
                            &[&format!("GTP_INT_INTERFACE_X{x}Y{y}")],
                            db.get_node_naming("INTF.GTP"),
                            &[(col, row)],
                        );
                    }
                    ColumnKind::Gtp | ColumnKind::Gtx => {
                        tile.add_xnode(
                            db.get_node("INTF.DELAY"),
                            &[&format!("GTX_LEFT_INT_INTERFACE_X{x}Y{y}")],
                            db.get_node_naming("INTF.GTX_LEFT"),
                            &[(col, row)],
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
                    tile.nodes.truncate(1);
                    tile.add_xnode(
                        db.get_node("INTF.DELAY"),
                        &[&format!("EMAC_INT_INTERFACE_X{x}Y{y}")],
                        db.get_node_naming("INTF.EMAC"),
                        &[(col, row)],
                    );
                }
            }
            for &br in &col_hard.rows_pcie {
                for dy in 0..40 {
                    let row = br + dy;
                    let y = row.to_idx();
                    let tile = &mut grid[(col, row)];
                    tile.nodes.truncate(1);
                    tile.add_xnode(
                        db.get_node("INTF.DELAY"),
                        &[&format!("PCIE_INT_INTERFACE_X{x}Y{y}")],
                        db.get_node_naming("INTF.PCIE"),
                        &[(col, row)],
                    );
                }
            }
        }

        for (py, &(bc, br)) in self.holes_ppc.iter().enumerate() {
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
                grid.fill_term_pair_dbuf(
                    (col_l, row),
                    (col_r, row),
                    db.get_term("PPC.E"),
                    db.get_term("PPC.W"),
                    tile_l,
                    tile_r,
                    db.get_term_naming("PPC.E"),
                    db.get_term_naming("PPC.W"),
                );
                let tile = &mut grid[(col_l, row)];
                tile.nodes.truncate(1);
                tile.add_xnode(
                    db.get_node("INTF.DELAY"),
                    &[&format!("PPC_L_INT_INTERFACE_X{xl}Y{y}")],
                    db.get_node_naming("INTF.PPC_L"),
                    &[(col_l, row)],
                );
                let tile = &mut grid[(col_r, row)];
                tile.nodes.truncate(1);
                tile.add_xnode(
                    db.get_node("INTF.DELAY"),
                    &[&format!("PPC_R_INT_INTERFACE_X{xr}Y{y}")],
                    db.get_node_naming("INTF.PPC_R"),
                    &[(col_r, row)],
                );
            }
            let row_b = br - 1;
            let row_t = br + 40;
            let yb = row_b.to_idx();
            let yt = row_t.to_idx();
            for dx in 1..13 {
                let col = bc + dx;
                let x = col.to_idx();
                grid.fill_term_tile(
                    (col, row_b),
                    "TERM.N.PPC",
                    "TERM.N.PPC",
                    format!("PPC_B_TERM_X{x}Y{yb}"),
                );
                grid.fill_term_tile(
                    (col, row_t),
                    "TERM.S.PPC",
                    "TERM.S.PPC",
                    format!("PPC_T_TERM_X{x}Y{yt}"),
                );
            }
            let mut crds = vec![];
            for dy in 0..40 {
                crds.push((col_l, br + dy));
            }
            for dy in 0..40 {
                crds.push((col_r, br + dy));
            }
            let yb = br.to_idx() / 10 * 11 + 11;
            let yt = br.to_idx() / 10 * 11 + 33;
            let tile_pb = format!("PPC_B_X36Y{yb}");
            let tile_pt = format!("PPC_T_X36Y{yt}");
            let node = grid[(bc, br)].add_xnode(
                db.get_node("PPC"),
                &[&tile_pb, &tile_pt],
                db.get_node_naming("PPC"),
                &crds,
            );
            node.add_bel(0, format!("PPC440_X0Y{py}"));
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
                grid.fill_term_tile(
                    (col_l, row),
                    "TERM.W",
                    "TERM.W",
                    format!("GTX_L_TERM_INT_X{xl}Y{y}"),
                );
            } else {
                grid.fill_term_tile(
                    (col_l, row),
                    "TERM.W",
                    "TERM.W",
                    format!("L_TERM_INT_X{xl}Y{y}"),
                );
            }
            if matches!(self.columns[col_r], ColumnKind::Gtp | ColumnKind::Gtx) {
                grid.fill_term_tile(
                    (col_r, row),
                    "TERM.E",
                    "TERM.E",
                    format!("R_TERM_INT_X{xr}Y{y}"),
                );
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
                let mon = if self.columns[col_l] == ColumnKind::Gtx {
                    "_MON"
                } else {
                    ""
                };
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

        let mut sx = 0;
        for (col, &cd) in &self.columns {
            let kind = match cd {
                ColumnKind::ClbLL => "CLBLL",
                ColumnKind::ClbLM => "CLBLM",
                _ => continue,
            };
            for row in grid.rows() {
                let tile = &mut grid[(col, row)];
                if tile.nodes.len() != 1 {
                    continue;
                }
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("{kind}_X{x}Y{y}");
                let node = tile.add_xnode(
                    db.get_node(kind),
                    &[&name],
                    db.get_node_naming(kind),
                    &[(col, row)],
                );
                node.add_bel(0, format!("SLICE_X{sx}Y{y}"));
                node.add_bel(1, format!("SLICE_X{sx}Y{y}", sx = sx + 1));
            }
            sx += 2;
        }

        let mut hard_skip = BTreeSet::new();
        if let Some(ref hard) = self.col_hard {
            for (i, &row) in hard.rows_emac.iter().enumerate() {
                hard_skip.insert(row);
                hard_skip.insert(row + 5);
                let x = hard.col.to_idx();
                let y = row.to_idx();
                let crds: Vec<_> = (0..10).map(|dy| (hard.col, row + dy)).collect();
                let name = format!("EMAC_X{x}Y{y}");
                let node = grid[crds[0]].add_xnode(
                    db.get_node("EMAC"),
                    &[&name],
                    db.get_node_naming("EMAC"),
                    &crds,
                );
                node.add_bel(0, format!("TEMAC_X0Y{i}"));
            }
            for (i, &row) in hard.rows_pcie.iter().enumerate() {
                for dy in [0, 5, 10, 15, 20, 25, 30, 35] {
                    hard_skip.insert(row + dy);
                }
                let x = hard.col.to_idx();
                let y = row.to_idx();
                let crds: Vec<_> = (0..40).map(|dy| (hard.col, row + dy)).collect();
                let name_b = format!("PCIE_B_X{x}Y{y}", y = y + 10);
                let name_t = format!("PCIE_T_X{x}Y{y}", y = y + 30);
                let node = grid[crds[0]].add_xnode(
                    db.get_node("PCIE"),
                    &[&name_b, &name_t],
                    db.get_node_naming("PCIE"),
                    &crds,
                );
                node.add_bel(0, format!("PCIE_X0Y{i}"));
            }
        }

        let mut px = 0;
        let mut bx = 0;
        let mut dx = 0;
        for (col, &cd) in &self.columns {
            let kind = match cd {
                ColumnKind::Bram => "BRAM",
                ColumnKind::Dsp => "DSP",
                _ => continue,
            };
            let mut tk = kind;
            if let Some(ref hard) = self.col_hard {
                if hard.col == col {
                    tk = "PCIE_BRAM";
                }
            }
            'a: for row in grid.rows() {
                if row.to_idx() % 5 != 0 {
                    continue;
                }
                for &(bc, br) in &self.holes_ppc {
                    if col >= bc && col < bc + 14 && row >= br && row < br + 40 {
                        continue 'a;
                    }
                }
                if let Some(ref hard) = self.col_hard {
                    if hard.col == col && hard_skip.contains(&row) {
                        continue;
                    }
                }
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("{tk}_X{x}Y{y}");
                let node = grid[(col, row)].add_xnode(
                    db.get_node(kind),
                    &[&name],
                    db.get_node_naming(kind),
                    &[
                        (col, row),
                        (col, row + 1),
                        (col, row + 2),
                        (col, row + 3),
                        (col, row + 4),
                    ],
                );
                if cd == ColumnKind::Bram {
                    node.add_bel(0, format!("RAMB36_X{bx}Y{sy}", sy = y / 5));
                } else {
                    node.add_bel(0, format!("DSP48_X{dx}Y{sy}", sy = y / 5 * 2));
                    node.add_bel(1, format!("DSP48_X{dx}Y{sy}", sy = y / 5 * 2 + 1));
                }
                if kind == "BRAM" && row.to_idx() % 20 == 10 && !self.cols_mgt_buf.contains(&col) {
                    let name_h = format!("HCLK_{tk}_X{x}Y{y}", y = y - 1);
                    let node = grid[(col, row)].add_xnode(
                        db.get_node("PMVBRAM"),
                        &[&name_h, &name],
                        db.get_node_naming("PMVBRAM"),
                        &[
                            (col, row),
                            (col, row + 1),
                            (col, row + 2),
                            (col, row + 3),
                            (col, row + 4),
                        ],
                    );
                    node.add_bel(0, format!("PMVBRAM_X{px}Y{sy}", sy = y / 20));
                }
            }
            if cd == ColumnKind::Bram {
                bx += 1;
                if !self.cols_mgt_buf.contains(&col) {
                    px += 1;
                }
            } else {
                dx += 1;
            }
        }

        for col in grid.cols() {
            for row in grid.rows() {
                let crow = RowId::from_idx(row.to_idx() / 20 * 20 + 10);
                grid[(col, row)].clkroot = (col, crow);
            }
        }

        egrid
    }
}