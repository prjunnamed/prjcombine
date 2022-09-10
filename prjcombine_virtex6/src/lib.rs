#![allow(clippy::comparison_chain)]

use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ColId, ExpandedDieRefMut, ExpandedGrid, Rect, RowId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DisabledPart {
    Emac(RowId),
    GtxRow(usize),
    SysMon,
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
pub enum GtPin {
    RxP(u8),
    RxN(u8),
    TxP(u8),
    TxN(u8),
    ClkP(u8),
    ClkN(u8),
    AVttRCal,
    RRef,
    RBias,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtRegion {
    All,
    S,
    N,
    L,
    R,
    LS,
    RS,
    LN,
    RN,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtxRegionPin {
    AVtt,
    AVcc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GthRegionPin {
    AVtt,
    AGnd,
    AVcc,
    AVccRx,
    AVccPll,
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
    GtxRegion(GtRegion, GtxRegionPin),
    GthRegion(GtRegion, GthRegionPin),
    Dxp,
    Dxn,
    Vfs,
    SysMon(SysMonPin),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
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
        matches!(
            (self.bank, self.row.to_idx() % 40),
            (24 | 34, 36..=39) | (25 | 35, 0..=3)
        )
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
    pub fn get_cfg(&self) -> Option<SharedCfgPin> {
        match (self.bank, self.row.to_idx() % 40) {
            (24, 6) => Some(SharedCfgPin::CsoB),
            (24, 7) => Some(SharedCfgPin::Rs(0)),
            (24, 8) => Some(SharedCfgPin::Rs(1)),
            (24, 9) => Some(SharedCfgPin::FweB),
            (24, 10) => Some(SharedCfgPin::FoeB),
            (24, 11) => Some(SharedCfgPin::FcsB),
            (24, 12) => Some(SharedCfgPin::Data(0)),
            (24, 13) => Some(SharedCfgPin::Data(1)),
            (24, 14) => Some(SharedCfgPin::Data(2)),
            (24, 15) => Some(SharedCfgPin::Data(3)),
            (24, 24) => Some(SharedCfgPin::Data(4)),
            (24, 25) => Some(SharedCfgPin::Data(5)),
            (24, 26) => Some(SharedCfgPin::Data(6)),
            (24, 27) => Some(SharedCfgPin::Data(7)),
            (24, 28) => Some(SharedCfgPin::Data(8)),
            (24, 29) => Some(SharedCfgPin::Data(9)),
            (24, 30) => Some(SharedCfgPin::Data(10)),
            (24, 31) => Some(SharedCfgPin::Data(11)),
            (24, 32) => Some(SharedCfgPin::Data(12)),
            (24, 33) => Some(SharedCfgPin::Data(13)),
            (24, 34) => Some(SharedCfgPin::Data(14)),
            (24, 35) => Some(SharedCfgPin::Data(15)),
            (34, 2) => Some(SharedCfgPin::Addr(16)),
            (34, 3) => Some(SharedCfgPin::Addr(17)),
            (34, 4) => Some(SharedCfgPin::Addr(18)),
            (34, 5) => Some(SharedCfgPin::Addr(19)),
            (34, 6) => Some(SharedCfgPin::Addr(20)),
            (34, 7) => Some(SharedCfgPin::Addr(21)),
            (34, 8) => Some(SharedCfgPin::Addr(22)),
            (34, 9) => Some(SharedCfgPin::Addr(23)),
            (34, 10) => Some(SharedCfgPin::Addr(24)),
            (34, 11) => Some(SharedCfgPin::Addr(25)),
            (34, 12) => Some(SharedCfgPin::Data(16)),
            (34, 13) => Some(SharedCfgPin::Data(17)),
            (34, 14) => Some(SharedCfgPin::Data(18)),
            (34, 15) => Some(SharedCfgPin::Data(19)),
            (34, 24) => Some(SharedCfgPin::Data(20)),
            (34, 25) => Some(SharedCfgPin::Data(21)),
            (34, 26) => Some(SharedCfgPin::Data(22)),
            (34, 27) => Some(SharedCfgPin::Data(23)),
            (34, 28) => Some(SharedCfgPin::Data(24)),
            (34, 29) => Some(SharedCfgPin::Data(25)),
            (34, 30) => Some(SharedCfgPin::Data(26)),
            (34, 31) => Some(SharedCfgPin::Data(27)),
            (34, 32) => Some(SharedCfgPin::Data(28)),
            (34, 33) => Some(SharedCfgPin::Data(29)),
            (34, 34) => Some(SharedCfgPin::Data(30)),
            (34, 35) => Some(SharedCfgPin::Data(31)),
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
    pub fn get_pads(&self, grid: &Grid) -> Vec<(String, String, GtPin)> {
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
                res.push((
                    format!("OPAD_X{}Y{}", opx, opy + 2 * (3 - b)),
                    format!("MGTTXN{}_{}", b, self.bank),
                    GtPin::TxN(b as u8),
                ));
                res.push((
                    format!("OPAD_X{}Y{}", opx, opy + 2 * (3 - b) + 1),
                    format!("MGTTXP{}_{}", b, self.bank),
                    GtPin::TxP(b as u8),
                ));
                res.push((
                    format!("IPAD_X{}Y{}", ipx, ipy + 6 + 2 * (3 - b)),
                    format!("MGTRXN{}_{}", b, self.bank),
                    GtPin::RxN(b as u8),
                ));
                res.push((
                    format!("IPAD_X{}Y{}", ipx, ipy + 6 + 2 * (3 - b) + 1),
                    format!("MGTRXP{}_{}", b, self.bank),
                    GtPin::RxP(b as u8),
                ));
            }
            res.push((
                format!("IPAD_X{}Y{}", ipx, ipy - 9),
                format!("MGTREFCLKN_{}", self.bank),
                GtPin::ClkN(0),
            ));
            res.push((
                format!("IPAD_X{}Y{}", ipx, ipy - 8),
                format!("MGTREFCLKP_{}", self.bank),
                GtPin::ClkP(0),
            ));
        } else {
            let opy = self.gy * 8;
            let ipy = self.gy * 24;
            for b in 0..4 {
                res.push((
                    format!("OPAD_X{}Y{}", opx, opy + 2 * b),
                    format!("MGTTXN{}_{}", b, self.bank),
                    GtPin::TxN(b as u8),
                ));
                res.push((
                    format!("OPAD_X{}Y{}", opx, opy + 2 * b + 1),
                    format!("MGTTXP{}_{}", b, self.bank),
                    GtPin::TxP(b as u8),
                ));
                res.push((
                    format!("IPAD_X{}Y{}", ipx, ipy + 6 * b),
                    format!("MGTRXN{}_{}", b, self.bank),
                    GtPin::RxN(b as u8),
                ));
                res.push((
                    format!("IPAD_X{}Y{}", ipx, ipy + 6 * b + 1),
                    format!("MGTRXP{}_{}", b, self.bank),
                    GtPin::RxP(b as u8),
                ));
            }
            for b in 0..2 {
                res.push((
                    format!("IPAD_X{}Y{}", ipx, ipy + 10 - 2 * b),
                    format!("MGTREFCLK{}P_{}", b, self.bank),
                    GtPin::ClkP(b as u8),
                ));
                res.push((
                    format!("IPAD_X{}Y{}", ipx, ipy + 11 - 2 * b),
                    format!("MGTREFCLK{}N_{}", b, self.bank),
                    GtPin::ClkN(b as u8),
                ));
            }
        }
        res
    }
}

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub disabled: &'a BTreeSet<DisabledPart>,
    pub egrid: ExpandedGrid<'a>,
}

struct Expander<'a, 'b> {
    grid: &'b Grid,
    db: &'a IntDb,
    disabled: &'b BTreeSet<DisabledPart>,
    die: ExpandedDieRefMut<'a, 'b>,
    tiexlut: EntityVec<ColId, usize>,
    rxlut: EntityVec<ColId, usize>,
    site_holes: Vec<Rect>,
    int_holes: Vec<Rect>,
    hard_skip: BTreeSet<RowId>,
}

impl<'a, 'b> Expander<'a, 'b> {
    fn fill_rxlut(&mut self) {
        let mut rx = 0;
        for (col, &kind) in &self.grid.columns {
            if self.grid.cols_vbrk.contains(&col) {
                rx += 1;
            }
            self.rxlut.push(rx);
            match kind {
                ColumnKind::ClbLL | ColumnKind::ClbLM => rx += 2,
                ColumnKind::Bram | ColumnKind::Dsp => rx += 3,
                ColumnKind::Io => {
                    if col.to_idx() == 0 {
                        rx += 5;
                    } else {
                        rx += 4;
                    }
                }
                ColumnKind::Gt => rx += 4,
                ColumnKind::Cmt => rx += 4,
            }
        }
    }

    fn fill_tiexlut(&mut self) {
        let mut tie_x = 0;
        for &kind in self.grid.columns.values() {
            self.tiexlut.push(tie_x);
            tie_x += 1;
            if kind == ColumnKind::Dsp {
                tie_x += 1;
            }
        }
    }

    fn fill_int(&mut self) {
        for (col, &kind) in &self.grid.columns {
            let tie_x = self.tiexlut[col];
            for row in self.die.rows() {
                let x = col.to_idx();
                let y = row.to_idx();
                self.die
                    .fill_tile((col, row), "INT", "INT", format!("INT_X{x}Y{y}"));
                let tile = &mut self.die[(col, row)];
                tile.nodes[0].tie_name = Some(format!("TIEOFF_X{tie_x}Y{y}"));
                match kind {
                    ColumnKind::ClbLL => (),
                    ColumnKind::ClbLM => (),
                    ColumnKind::Io if col < self.grid.col_cfg => {
                        tile.add_xnode(
                            self.db.get_node("INTF"),
                            &[&format!("IOI_L_INT_INTERFACE_X{x}Y{y}")],
                            self.db.get_node_naming("INTF.IOI_L"),
                            &[(col, row)],
                        );
                    }
                    ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::Io | ColumnKind::Cmt => {
                        tile.add_xnode(
                            self.db.get_node("INTF"),
                            &[&format!("INT_INTERFACE_X{x}Y{y}")],
                            self.db.get_node_naming("INTF"),
                            &[(col, row)],
                        );
                    }
                    ColumnKind::Gt => {
                        if x == 0 {
                            tile.add_xnode(
                                self.db.get_node("INTF.DELAY"),
                                &[&format!("GT_L_INT_INTERFACE_X{x}Y{y}")],
                                self.db.get_node_naming("INTF.GT_L"),
                                &[(col, row)],
                            );
                        } else {
                            tile.add_xnode(
                                self.db.get_node("INTF.DELAY"),
                                &[&format!("GTX_INT_INTERFACE_X{x}Y{y}")],
                                self.db.get_node_naming("INTF.GTX"),
                                &[(col, row)],
                            );
                        }
                    }
                }
            }
        }
    }

    fn fill_cfg(&mut self) {
        let row_b = RowId::from_idx(self.grid.reg_cfg * 40 - 40);
        let row_t = RowId::from_idx(self.grid.reg_cfg * 40 + 40);
        self.die.nuke_rect(self.grid.col_cfg - 6, row_b, 6, 80);
        for dx in 0..6 {
            let col = self.grid.col_cfg - 6 + dx;
            if row_b.to_idx() != 0 {
                self.die.fill_term_anon((col, row_b - 1), "TERM.N");
            }
            if row_t.to_idx() != self.grid.regs * 40 {
                self.die.fill_term_anon((col, row_t), "TERM.S");
            }
        }
        self.site_holes.push(Rect {
            col_l: self.grid.col_cfg - 6,
            col_r: self.grid.col_cfg,
            row_b,
            row_t,
        });
        self.int_holes.push(Rect {
            col_l: self.grid.col_cfg - 6,
            col_r: self.grid.col_cfg,
            row_b,
            row_t,
        });
        let crds: [_; 80] = core::array::from_fn(|dy| (self.grid.col_cfg, row_b + dy));
        let ry = row_b.to_idx() + 11 + row_b.to_idx() / 20;
        let rx = self.rxlut[self.grid.col_cfg] - 2;
        let name0 = format!("CFG_CENTER_0_X{rx}Y{ry}");
        let name1 = format!("CFG_CENTER_1_X{rx}Y{ry}", ry = ry + 21);
        let name2 = format!("CFG_CENTER_2_X{rx}Y{ry}", ry = ry + 42);
        let name3 = format!("CFG_CENTER_3_X{rx}Y{ry}", ry = ry + 63);
        let node = self.die[crds[40]].add_xnode(
            self.db.get_node("CFG"),
            &[&name0, &name1, &name2, &name3],
            self.db.get_node_naming("CFG"),
            &crds,
        );
        node.add_bel(0, "BSCAN_X0Y0".to_string());
        node.add_bel(1, "BSCAN_X0Y1".to_string());
        node.add_bel(2, "BSCAN_X0Y2".to_string());
        node.add_bel(3, "BSCAN_X0Y3".to_string());
        node.add_bel(4, "ICAP_X0Y0".to_string());
        node.add_bel(5, "ICAP_X0Y1".to_string());
        node.add_bel(6, "PMV_X0Y0".to_string());
        node.add_bel(7, "PMV_X0Y1".to_string());
        node.add_bel(8, "STARTUP_X0Y0".to_string());
        node.add_bel(9, "CAPTURE_X0Y0".to_string());
        node.add_bel(10, "FRAME_ECC".to_string());
        node.add_bel(11, "EFUSE_USR_X0Y0".to_string());
        node.add_bel(12, "USR_ACCESS_X0Y0".to_string());
        node.add_bel(13, "DNA_PORT_X0Y0".to_string());
        node.add_bel(14, "DCIRESET_X0Y0".to_string());
        node.add_bel(15, "CFG_IO_ACCESS_X0Y0".to_string());
        node.add_bel(16, "SYSMON_X0Y0".to_string());
        let ipx = if self.grid.has_left_gt() { 1 } else { 0 };
        let mut ipy = if self.grid.has_gt() {
            self.grid.reg_cfg * 24 + 6
        } else {
            0
        };
        for i in 0..self.grid.reg_cfg {
            if self.disabled.contains(&DisabledPart::GtxRow(i)) {
                ipy -= 24;
            }
        }
        node.add_bel(17, format!("IPAD_X{ipx}Y{y}", y = ipy));
        node.add_bel(18, format!("IPAD_X{ipx}Y{y}", y = ipy + 1));
    }

    fn fill_btterm(&mut self) {
        let row_b = self.die.rows().next().unwrap();
        let row_t = self.die.rows().next_back().unwrap();
        for col in self.die.cols() {
            if !self.die[(col, row_b)].nodes.is_empty() {
                self.die.fill_term_anon((col, row_b), "TERM.S.HOLE");
            }
            if !self.die[(col, row_t)].nodes.is_empty() {
                self.die.fill_term_anon((col, row_t), "TERM.N.HOLE");
            }
        }
        let col_l = self.die.cols().next().unwrap();
        let col_r = self.die.cols().next_back().unwrap();
        for row in self.die.rows() {
            self.die.fill_term_anon((col_l, row), "TERM.W");
            self.die.fill_term_anon((col_r, row), "TERM.E");
        }
    }

    fn fill_clb(&mut self) {
        let mut sx = 0;
        for (col, &cd) in &self.grid.columns {
            let kind = match cd {
                ColumnKind::ClbLL => "CLBLL",
                ColumnKind::ClbLM => "CLBLM",
                _ => continue,
            };
            'a: for row in self.die.rows() {
                let tile = &mut self.die[(col, row)];
                for hole in &self.site_holes {
                    if hole.contains(col, row) {
                        continue 'a;
                    }
                }
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("{kind}_X{x}Y{y}");
                let node = tile.add_xnode(
                    self.db.get_node(kind),
                    &[&name],
                    self.db.get_node_naming(kind),
                    &[(col, row)],
                );
                node.add_bel(0, format!("SLICE_X{sx}Y{y}"));
                node.add_bel(1, format!("SLICE_X{sx}Y{y}", sx = sx + 1));
            }
            sx += 2;
        }
    }

    fn fill_hard(&mut self) {
        if let Some(ref hard) = self.grid.col_hard {
            let col = hard.col;
            let x = col.to_idx();
            let mut ey = 0;
            for &br in &hard.rows_emac {
                for dy in 0..10 {
                    let row = br + dy;
                    let y = row.to_idx();
                    let tile = &mut self.die[(col, row)];
                    tile.nodes.truncate(1);
                    tile.add_xnode(
                        self.db.get_node("INTF.DELAY"),
                        &[&format!("EMAC_INT_INTERFACE_X{x}Y{y}")],
                        self.db.get_node_naming("INTF.EMAC"),
                        &[(col, row)],
                    );
                }
                self.hard_skip.insert(br);
                self.hard_skip.insert(br + 5);
                if self.disabled.contains(&DisabledPart::Emac(br)) {
                    continue;
                }
                let x = hard.col.to_idx();
                let y = br.to_idx();
                let crds: [_; 10] = core::array::from_fn(|dy| (hard.col, br + dy));
                let name = format!("EMAC_X{x}Y{y}");
                let node = self.die[crds[0]].add_xnode(
                    self.db.get_node("EMAC"),
                    &[&name],
                    self.db.get_node_naming("EMAC"),
                    &crds,
                );
                node.add_bel(0, format!("TEMAC_X0Y{ey}"));
                ey += 1;
            }

            for (py, &br) in hard.rows_pcie.iter().enumerate() {
                self.die.nuke_rect(col - 1, br, 2, 20);
                self.site_holes.push(Rect {
                    col_l: col - 3,
                    col_r: col + 1,
                    row_b: br,
                    row_t: br + 20,
                });
                self.int_holes.push(Rect {
                    col_l: col - 1,
                    col_r: col + 1,
                    row_b: br,
                    row_t: br + 20,
                });
                for dy in 0..20 {
                    let row = br + dy;
                    let y = row.to_idx();
                    self.die[(col - 3, row)].add_xnode(
                        self.db.get_node("INTF.DELAY"),
                        &[&format!("PCIE_INT_INTERFACE_L_X{xx}Y{y}", xx = x - 3)],
                        self.db.get_node_naming("INTF.PCIE_L"),
                        &[(col - 3, row)],
                    );
                    self.die[(col - 2, row)].add_xnode(
                        self.db.get_node("INTF.DELAY"),
                        &[&format!("PCIE_INT_INTERFACE_R_X{xx}Y{y}", xx = x - 2)],
                        self.db.get_node_naming("INTF.PCIE_R"),
                        &[(col - 2, row)],
                    );
                }
                if br.to_idx() != 0 {
                    self.die.fill_term_anon((col - 1, br - 1), "TERM.N");
                    self.die.fill_term_anon((col, br - 1), "TERM.N");
                }
                self.die.fill_term_anon((col - 1, br + 20), "TERM.S");
                self.die.fill_term_anon((col, br + 20), "TERM.S");

                for dy in [0, 5, 10, 15] {
                    self.hard_skip.insert(br + dy);
                }
                let x = hard.col.to_idx() - 2;
                let y = br.to_idx();
                let mut crds = vec![];
                for dy in 0..20 {
                    crds.push((hard.col - 3, br + dy));
                }
                for dy in 0..20 {
                    crds.push((hard.col - 2, br + dy));
                }
                let name = format!("PCIE_X{x}Y{y}", y = y + 10);
                let node = self.die[crds[0]].add_xnode(
                    self.db.get_node("PCIE"),
                    &[&name],
                    self.db.get_node_naming("PCIE"),
                    &crds,
                );
                node.add_bel(0, format!("PCIE_X0Y{py}"));
            }
        }
    }

    fn fill_bram_dsp(&mut self) {
        let mut bx = 0;
        let mut dx = 0;
        for (col, &cd) in &self.grid.columns {
            let kind = match cd {
                ColumnKind::Bram => "BRAM",
                ColumnKind::Dsp => "DSP",
                _ => continue,
            };
            for row in self.die.rows() {
                if row.to_idx() % 5 != 0 {
                    continue;
                }
                if let Some(ref hard) = self.grid.col_hard {
                    if hard.col == col && self.hard_skip.contains(&row) {
                        continue;
                    }
                }
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("{kind}_X{x}Y{y}");
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node(kind),
                    &[&name],
                    self.db.get_node_naming(kind),
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
                    node.add_bel(1, format!("RAMB18_X{bx}Y{sy}", sy = y / 5 * 2));
                    node.add_bel(2, format!("RAMB18_X{bx}Y{sy}", sy = y / 5 * 2 + 1));
                } else {
                    node.add_bel(0, format!("DSP48_X{dx}Y{sy}", sy = y / 5 * 2));
                    node.add_bel(1, format!("DSP48_X{dx}Y{sy}", sy = y / 5 * 2 + 1));
                    let tx = self.tiexlut[col] + 1;
                    node.add_bel(2, format!("TIEOFF_X{tx}Y{y}"));
                }
                if kind == "BRAM" && row.to_idx() % 40 == 20 {
                    let mut hy = y - 1;
                    if let Some(ref hard) = self.grid.col_hard {
                        if hard.col == col && hard.rows_pcie.contains(&(row - 20)) {
                            hy = y;
                        }
                    }
                    let name_h = format!("HCLK_BRAM_X{x}Y{hy}");
                    let name_1 = format!("BRAM_X{x}Y{y}", y = y + 5);
                    let name_2 = format!("BRAM_X{x}Y{y}", y = y + 10);
                    let coords: Vec<_> = (0..15).map(|dy| (col, row + dy)).collect();
                    let node = self.die[(col, row)].add_xnode(
                        self.db.get_node("PMVBRAM"),
                        &[&name_h, &name, &name_1, &name_2],
                        self.db.get_node_naming("PMVBRAM"),
                        &coords,
                    );
                    node.add_bel(0, format!("PMVBRAM_X{bx}Y{sy}", sy = y / 40));
                }
            }
            if cd == ColumnKind::Bram {
                bx += 1;
            } else {
                dx += 1;
            }
        }
    }

    fn fill_io(&mut self) {
        let mut iox = 0;
        for (i, &col) in self.grid.cols_io.iter().enumerate() {
            let hclk_tk = match i {
                0 | 3 => "HCLK_OUTER_IOI",
                1 | 2 => "HCLK_INNER_IOI",
                _ => unreachable!(),
            };
            let hclk_naming = match i {
                0 => "HCLK_IOI.OL",
                1 => "HCLK_IOI.IL",
                2 => "HCLK_IOI.IR",
                3 => "HCLK_IOI.OR",
                _ => unreachable!(),
            };
            let ioi_tk = match i {
                0 | 1 => "LIOI",
                2 | 3 => "RIOI",
                _ => unreachable!(),
            };
            let iob_tk = match i {
                0 => "LIOB",
                1 => {
                    if self.grid.cols_io[0].is_none() {
                        "LIOB"
                    } else {
                        "LIOB_FT"
                    }
                }
                2 | 3 => "RIOB",
                _ => unreachable!(),
            };
            if let Some(col) = col {
                for row in self.die.rows() {
                    if row.to_idx() % 2 == 0 {
                        let name_ioi =
                            format!("{ioi_tk}_X{x}Y{y}", x = col.to_idx(), y = row.to_idx());
                        let name_iob =
                            format!("{iob_tk}_X{x}Y{y}", x = col.to_idx(), y = row.to_idx());
                        let node = self.die[(col, row)].add_xnode(
                            self.db.get_node("IO"),
                            &[&name_ioi, &name_iob],
                            self.db.get_node_naming(ioi_tk),
                            &[(col, row), (col, row + 1)],
                        );
                        node.add_bel(0, format!("ILOGIC_X{iox}Y{y}", y = row.to_idx() + 1));
                        node.add_bel(1, format!("ILOGIC_X{iox}Y{y}", y = row.to_idx()));
                        node.add_bel(2, format!("OLOGIC_X{iox}Y{y}", y = row.to_idx() + 1));
                        node.add_bel(3, format!("OLOGIC_X{iox}Y{y}", y = row.to_idx()));
                        node.add_bel(4, format!("IODELAY_X{iox}Y{y}", y = row.to_idx() + 1));
                        node.add_bel(5, format!("IODELAY_X{iox}Y{y}", y = row.to_idx()));
                        node.add_bel(6, format!("IOB_X{iox}Y{y}", y = row.to_idx() + 1));
                        node.add_bel(7, format!("IOB_X{iox}Y{y}", y = row.to_idx()));
                    }

                    if row.to_idx() % 40 == 20 {
                        let hx = if i < 2 && col.to_idx() != 0 {
                            col.to_idx() - 1
                        } else {
                            col.to_idx()
                        };
                        let name = format!("{hclk_tk}_X{hx}Y{y}", y = row.to_idx() - 1);
                        let name_ioi_s =
                            format!("{ioi_tk}_X{x}Y{y}", x = col.to_idx(), y = row.to_idx() - 2);
                        let name_ioi_n =
                            format!("{ioi_tk}_X{x}Y{y}", x = col.to_idx(), y = row.to_idx());
                        let node = self.die[(col, row)].add_xnode(
                            self.db.get_node("HCLK_IOI"),
                            &[&name, &name_ioi_s, &name_ioi_n],
                            self.db.get_node_naming(hclk_naming),
                            &[(col, row - 1), (col, row)],
                        );
                        let hy = row.to_idx() / 40;
                        node.add_bel(0, format!("BUFIODQS_X{iox}Y{y}", y = hy * 4 + 2));
                        node.add_bel(1, format!("BUFIODQS_X{iox}Y{y}", y = hy * 4 + 3));
                        node.add_bel(2, format!("BUFIODQS_X{iox}Y{y}", y = hy * 4));
                        node.add_bel(3, format!("BUFIODQS_X{iox}Y{y}", y = hy * 4 + 1));
                        node.add_bel(4, format!("BUFR_X{iox}Y{y}", y = hy * 2 + 1));
                        node.add_bel(5, format!("BUFR_X{iox}Y{y}", y = hy * 2));
                        node.add_bel(6, format!("BUFO_X{iox}Y{y}", y = hy * 2 + 1));
                        node.add_bel(7, format!("BUFO_X{iox}Y{y}", y = hy * 2));
                        node.add_bel(8, format!("IDELAYCTRL_X{iox}Y{hy}"));
                        node.add_bel(9, format!("DCI_X{iox}Y{hy}"));
                    }
                }
                iox += 1;
            }
        }
    }

    fn fill_cmt(&mut self) {
        let col = self.grid.col_cfg;
        let x = col.to_idx();
        let mut pmvy = 0;
        for reg in 0..self.grid.regs {
            let row_hclk = RowId::from_idx(reg * 40 + 20);
            let crds: [_; 40] = core::array::from_fn(|dy| (col, row_hclk - 20 + dy));
            let name_b = format!("CMT_X{x}Y{y}", y = row_hclk.to_idx() - 9);
            let name_t = format!("CMT_X{x}Y{y}", y = row_hclk.to_idx() + 9);
            let bt = if reg < self.grid.reg_cfg {
                "BOT"
            } else {
                "TOP"
            };
            let name_h = format!("HCLK_CMT_{bt}_X{x}Y{y}", y = row_hclk.to_idx() - 1);
            let node = self.die[(col, row_hclk)].add_xnode(
                self.db.get_node("CMT"),
                &[&name_b, &name_t, &name_h],
                self.db.get_node_naming(if reg < self.grid.reg_cfg {
                    "CMT.BOT"
                } else {
                    "CMT.TOP"
                }),
                &crds,
            );
            for i in 0..2 {
                for j in 0..12 {
                    node.add_bel(i * 12 + j, format!("BUFHCE_X{i}Y{y}", y = reg * 12 + j));
                }
            }
            node.add_bel(24, format!("MMCM_ADV_X0Y{y}", y = reg * 2));
            node.add_bel(25, format!("MMCM_ADV_X0Y{y}", y = reg * 2 + 1));
            node.add_bel(26, format!("PPR_FRAME_X0Y{reg}"));

            let row = row_hclk - 20;
            let y = row.to_idx();
            if reg < self.grid.reg_cfg - 1 {
                let name = format!("CMT_PMVA_BELOW_X{x}Y{y}");
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node("PMVIOB"),
                    &[&name],
                    self.db.get_node_naming("CMT_PMVA_BELOW"),
                    &[(col, row), (col, row + 1)],
                );
                node.add_bel(0, format!("PMVIOB_X0Y{pmvy}"));
                pmvy += 1;
            } else if reg == self.grid.reg_cfg - 1 {
                // CMT_PMVB, empty
            } else if reg == self.grid.reg_cfg {
                let name = format!("CMT_BUFG_TOP_X{x}Y{y}");
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node("CMT_BUFG_TOP"),
                    &[&name, &name_b],
                    self.db.get_node_naming("CMT_BUFG_TOP"),
                    &[(col, row), (col, row + 1), (col, row + 2)],
                );
                for i in 0..16 {
                    node.add_bel(i, format!("BUFGCTRL_X0Y{y}", y = i + 16));
                }
            } else {
                let name = format!("CMT_PMVB_BUF_ABOVE_X{x}Y{y}");
                self.die[(col, row)].add_xnode(
                    self.db.get_node("GCLK_BUF"),
                    &[&name],
                    self.db.get_node_naming("GCLK_BUF"),
                    &[],
                );
            }

            let row = row_hclk + 18;
            let y = row.to_idx();
            if reg < self.grid.reg_cfg - 1 {
                let name = format!("CMT_PMVB_BUF_BELOW_X{x}Y{y}");
                self.die[(col, row + 2)].add_xnode(
                    self.db.get_node("GCLK_BUF"),
                    &[&name],
                    self.db.get_node_naming("GCLK_BUF"),
                    &[],
                );
            } else if reg == self.grid.reg_cfg - 1 {
                let name = format!("CMT_BUFG_BOT_X{x}Y{y}");
                let node = self.die[(col, row + 2)].add_xnode(
                    self.db.get_node("CMT_BUFG_BOT"),
                    &[&name, &name_t],
                    self.db.get_node_naming("CMT_BUFG_BOT"),
                    &[(col, row - 1), (col, row), (col, row + 1)],
                );
                for i in 0..16 {
                    node.add_bel(i, format!("BUFGCTRL_X0Y{i}"));
                }
            } else {
                let name = format!("CMT_PMVA_X{x}Y{y}");
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node("PMVIOB"),
                    &[&name],
                    self.db.get_node_naming("CMT_PMVA"),
                    &[(col, row), (col, row + 1)],
                );
                node.add_bel(0, format!("PMVIOB_X0Y{pmvy}"));
                pmvy += 1;
            }
        }
    }

    fn fill_gt(&mut self) {
        let mut gx = 0;
        for (col, &cd) in &self.grid.columns {
            if cd != ColumnKind::Gt {
                continue;
            }
            let is_l = col.to_idx() == 0;
            let ipx = if is_l { 0 } else { 1 + gx };

            let mut gy = 0;
            let mut gthy = 0;
            let mut hy = 0;
            for row in self.die.rows() {
                let reg = row.to_idx() / 40;
                if reg >= self.grid.reg_gth_start {
                    if row.to_idx() % 40 == 20 {
                        let name_b = if is_l {
                            format!(
                                "GTH_L_BOT_X{x}Y{y}",
                                x = col.to_idx(),
                                y = row.to_idx() - 10
                            )
                        } else {
                            format!("GTH_BOT_X{x}Y{y}", x = col.to_idx(), y = row.to_idx() - 10)
                        };
                        let name_t = if is_l {
                            format!(
                                "GTH_L_TOP_X{x}Y{y}",
                                x = col.to_idx(),
                                y = row.to_idx() + 10
                            )
                        } else {
                            format!("GTH_TOP_X{x}Y{y}", x = col.to_idx(), y = row.to_idx() + 10)
                        };
                        let name_h = if is_l {
                            format!(
                                "HCLK_GTH_LEFT_X{x}Y{y}",
                                x = col.to_idx(),
                                y = row.to_idx() - 1
                            )
                        } else {
                            format!(
                                "HCLK_GTH_X{x}Y{y}",
                                x = self.rxlut[col] + 2,
                                y = row.to_idx() + row.to_idx() / 20
                            )
                        };
                        let crds: [_; 40] = core::array::from_fn(|dy| (col, row - 20 + dy));
                        let node = self.die[(col, row)].add_xnode(
                            self.db.get_node("GTH"),
                            &[&name_b, &name_t, &name_h],
                            self.db
                                .get_node_naming(if is_l { "GTH.L" } else { "GTH.R" }),
                            &crds,
                        );
                        for i in 0..8 {
                            node.add_bel(
                                i,
                                format!("IPAD_X{ipx}Y{y}", y = gy * 6 + gthy * 12 + 6 + (7 - i)),
                            );
                        }
                        for i in 0..8 {
                            node.add_bel(
                                8 + i,
                                format!("OPAD_X{gx}Y{y}", y = (gy + gthy) * 8 + (7 - i)),
                            );
                        }
                        node.add_bel(16, format!("IPAD_X{ipx}Y{y}", y = gy * 6 - 8 + gthy * 12));
                        node.add_bel(17, format!("IPAD_X{ipx}Y{y}", y = gy * 6 - 9 + gthy * 12));
                        node.add_bel(18, format!("GTHE1_QUAD_X{gx}Y{gthy}"));
                        node.add_bel(19, format!("IBUFDS_GTHE1_X{gx}Y{y}", y = gthy * 2 + 1));
                        gthy += 1;
                    }
                } else {
                    if self.disabled.contains(&DisabledPart::GtxRow(reg)) {
                        continue;
                    }
                    if row.to_idx() % 40 == 20 {
                        let crds: [_; 10] = core::array::from_fn(|dy| (col, row - 10 + dy));
                        let tk = if is_l { "HCLK_GTX_LEFT" } else { "HCLK_GTX" };
                        let name = if is_l {
                            format!("{tk}_X{x}Y{y}", x = col.to_idx(), y = row.to_idx() - 1)
                        } else {
                            format!(
                                "{tk}_X{x}Y{y}",
                                x = self.rxlut[col] + 2,
                                y = row.to_idx() + row.to_idx() / 20
                            )
                        };
                        let tk_gt = if is_l { "GTX_LEFT" } else { "GTX" };
                        let name_gt =
                            format!("{tk_gt}_X{x}Y{y}", x = col.to_idx(), y = row.to_idx() - 10);
                        let node = self.die[(col, row)].add_xnode(
                            self.db.get_node("HCLK_GTX"),
                            &[&name, &name_gt],
                            self.db.get_node_naming(tk),
                            &crds,
                        );
                        node.add_bel(0, format!("IPAD_X{ipx}Y{y}", y = gy * 6 - 2));
                        node.add_bel(1, format!("IPAD_X{ipx}Y{y}", y = gy * 6 - 1));
                        node.add_bel(2, format!("IPAD_X{ipx}Y{y}", y = gy * 6 - 4));
                        node.add_bel(3, format!("IPAD_X{ipx}Y{y}", y = gy * 6 - 3));
                        node.add_bel(4, format!("IBUFDS_GTXE1_X{gx}Y{y}", y = hy * 2));
                        node.add_bel(5, format!("IBUFDS_GTXE1_X{gx}Y{y}", y = hy * 2 + 1));
                        hy += 1;
                    }
                    if row.to_idx() % 10 == 0 {
                        let crds: [_; 10] = core::array::from_fn(|dy| (col, row + dy));
                        let tk = if is_l { "GTX_LEFT" } else { "GTX" };
                        let name = format!("{tk}_X{x}Y{y}", x = col.to_idx(), y = row.to_idx());
                        let node = self.die[(col, row)].add_xnode(
                            self.db.get_node("GTX"),
                            &[&name],
                            self.db.get_node_naming(tk),
                            &crds,
                        );
                        node.add_bel(0, format!("IPAD_X{ipx}Y{y}", y = gy * 6 + 1));
                        node.add_bel(1, format!("IPAD_X{ipx}Y{y}", y = gy * 6));
                        node.add_bel(2, format!("OPAD_X{gx}Y{y}", y = gy * 2 + 1));
                        node.add_bel(3, format!("OPAD_X{gx}Y{y}", y = gy * 2));
                        node.add_bel(4, format!("GTXE1_X{gx}Y{gy}"));
                        gy += 1;
                    }
                }
            }

            gx += 1;
        }
    }

    fn fill_hclk(&mut self) {
        for col in self.die.cols() {
            for row in self.die.rows() {
                let crow = RowId::from_idx(if row.to_idx() % 40 < 20 {
                    row.to_idx() / 40 * 40 + 19
                } else {
                    row.to_idx() / 40 * 40 + 20
                });
                self.die[(col, row)].clkroot = (col, crow);
                if row.to_idx() % 40 == 20 {
                    let mut skip_b = false;
                    let mut skip_t = false;
                    for hole in &self.int_holes {
                        if hole.contains(col, row) {
                            skip_t = true;
                        }
                        if hole.contains(col, row - 1) {
                            skip_b = true;
                        }
                    }
                    if skip_t && skip_b {
                        continue;
                    }
                    let mut naming = "HCLK";
                    let mut name = format!("HCLK_X{x}Y{y}", x = col.to_idx(), y = row.to_idx() - 1);
                    if col == self.grid.cols_qbuf.0 || col == self.grid.cols_qbuf.1 {
                        naming = "HCLK.QBUF";
                        name =
                            format!("HCLK_QBUF_X{x}Y{y}", x = col.to_idx(), y = row.to_idx() - 1);
                    }
                    if skip_b {
                        name = format!("HCLK_X{x}Y{y}", x = col.to_idx(), y = row.to_idx());
                    }
                    let node = self.die[(col, row)].add_xnode(
                        self.db.get_node("HCLK"),
                        &[&name],
                        self.db.get_node_naming(naming),
                        &[(col, row - 1), (col, row)],
                    );
                    node.add_bel(
                        0,
                        format!(
                            "GLOBALSIG_X{x}Y{y}",
                            x = col.to_idx(),
                            y = row.to_idx() / 40
                        ),
                    );
                    if naming == "HCLK.QBUF" {
                        self.die[(col, row)].add_xnode(
                            self.db.get_node("HCLK_QBUF"),
                            &[&name],
                            self.db.get_node_naming("HCLK_QBUF"),
                            &[],
                        );
                    }
                    if self.grid.cols_mgt_buf.contains(&col) {
                        let is_l = col < self.grid.col_cfg;
                        let tk = if is_l {
                            "HCLK_CLBLM_MGT_LEFT"
                        } else {
                            "HCLK_CLB"
                        };
                        let name = format!("{tk}_X{x}Y{y}", x = col.to_idx(), y = row.to_idx() - 1);
                        self.die[(col, row)].add_xnode(
                            self.db.get_node("MGT_BUF"),
                            &[&name],
                            self.db
                                .get_node_naming(if is_l { "MGT_BUF.L" } else { "MGT_BUF.R" }),
                            &[],
                        );
                    }
                }
            }
        }
    }
}

impl Grid {
    pub fn row_hclk(&self, row: RowId) -> RowId {
        RowId::from_idx(row.to_idx() / 40 * 40 + 20)
    }

    pub fn row_bufg(&self) -> RowId {
        RowId::from_idx(self.reg_cfg * 40)
    }

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
            if disabled.contains(&DisabledPart::GtxRow(i)) {
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

    pub fn has_gt(&self) -> bool {
        self.columns.values().any(|&x| x == ColumnKind::Gt)
    }

    pub fn has_left_gt(&self) -> bool {
        *self.columns.first().unwrap() == ColumnKind::Gt
    }

    pub fn get_sysmon_pads(&self, disabled: &BTreeSet<DisabledPart>) -> Vec<(String, SysMonPin)> {
        let mut res = Vec::new();
        if self.col_hard.is_none() {
            res.push(("IPAD_X0Y0".to_string(), SysMonPin::VP));
            res.push(("IPAD_X0Y1".to_string(), SysMonPin::VN));
        } else {
            let mut ipy = 6;
            for i in 0..self.reg_cfg {
                if !disabled.contains(&DisabledPart::GtxRow(i)) {
                    ipy += 24;
                }
            }
            if self.has_left_gt() {
                res.push((format!("IPAD_X1Y{}", ipy), SysMonPin::VP));
                res.push((format!("IPAD_X1Y{}", ipy + 1), SysMonPin::VN));
            } else {
                res.push((format!("IPAD_X0Y{}", ipy), SysMonPin::VP));
                res.push((format!("IPAD_X0Y{}", ipy + 1), SysMonPin::VN));
            }
        }
        res
    }

    pub fn expand_grid<'a>(
        &'a self,
        db: &'a IntDb,
        disabled: &'a BTreeSet<DisabledPart>,
    ) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        egrid.tie_kind = Some("TIEOFF".to_string());
        egrid.tie_pin_gnd = Some("HARD0".to_string());
        egrid.tie_pin_vcc = Some("HARD1".to_string());
        let (_, die) = egrid.add_die(self.columns.len(), self.regs * 40);

        let mut expander = Expander {
            grid: self,
            db,
            disabled,
            die,
            tiexlut: EntityVec::new(),
            rxlut: EntityVec::new(),
            int_holes: vec![],
            site_holes: vec![],
            hard_skip: BTreeSet::new(),
        };

        expander.fill_tiexlut();
        expander.fill_rxlut();
        expander.fill_int();
        expander.fill_cfg();
        expander.fill_hard();
        expander.fill_btterm();
        expander.die.fill_main_passes();
        expander.fill_clb();
        expander.fill_bram_dsp();
        expander.fill_io();
        expander.fill_cmt();
        expander.fill_gt();
        expander.fill_hclk();

        ExpandedDevice {
            grid: self,
            disabled,
            egrid,
        }
    }
}
