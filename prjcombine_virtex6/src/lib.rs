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
    GtxRow(u32),
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
            if disabled.contains(&DisabledPart::GtxRow(i as u32)) {
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
            res.push(("IPAD_X0Y0".to_string(), SysMonPin::VP));
            res.push(("IPAD_X0Y1".to_string(), SysMonPin::VN));
        } else {
            let mut ipy = 6;
            for i in 0..self.reg_cfg {
                if !disabled.contains(&DisabledPart::GtxRow(i as u32)) {
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
        &self,
        db: &'a IntDb,
        disabled: &BTreeSet<DisabledPart>,
    ) -> ExpandedGrid<'a> {
        let mut egrid = ExpandedGrid::new(db);
        egrid.tie_kind = Some("TIEOFF".to_string());
        egrid.tie_pin_gnd = Some("HARD0".to_string());
        egrid.tie_pin_vcc = Some("HARD1".to_string());
        let (_, mut grid) = egrid.add_die(self.columns.len(), self.regs * 40);

        let mut tie_x = 0;
        let mut tiexlut = EntityVec::new();
        for (col, &kind) in &self.columns {
            tiexlut.push(tie_x);
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
                        tile.add_xnode(
                            db.get_node("INTF"),
                            &[&format!("IOI_L_INT_INTERFACE_X{x}Y{y}")],
                            db.get_node_naming("INTF.IOI_L"),
                            &[(col, row)],
                        );
                    }
                    ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::Io | ColumnKind::Cmt => {
                        tile.add_xnode(
                            db.get_node("INTF"),
                            &[&format!("INT_INTERFACE_X{x}Y{y}")],
                            db.get_node_naming("INTF"),
                            &[(col, row)],
                        );
                    }
                    ColumnKind::Gt => {
                        if x == 0 {
                            tile.add_xnode(
                                db.get_node("INTF.DELAY"),
                                &[&format!("GT_L_INT_INTERFACE_X{x}Y{y}")],
                                db.get_node_naming("INTF.GT_L"),
                                &[(col, row)],
                            );
                        } else {
                            tile.add_xnode(
                                db.get_node("INTF.DELAY"),
                                &[&format!("GTX_INT_INTERFACE_X{x}Y{y}")],
                                db.get_node_naming("INTF.GTX"),
                                &[(col, row)],
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
                grid.fill_term_anon((col, row_b - 1), "TERM.N");
            }
            if row_t.to_idx() != self.regs * 40 {
                grid.fill_term_anon((col, row_t), "TERM.S");
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
                grid.nuke_rect(col - 1, br, 2, 20);
                for dy in 0..20 {
                    let row = br + dy;
                    let y = row.to_idx();
                    grid[(col - 3, row)].add_xnode(
                        db.get_node("INTF.DELAY"),
                        &[&format!("PCIE_INT_INTERFACE_L_X{xx}Y{y}", xx = x - 3)],
                        db.get_node_naming("INTF.PCIE_L"),
                        &[(col - 3, row)],
                    );
                    grid[(col - 2, row)].add_xnode(
                        db.get_node("INTF.DELAY"),
                        &[&format!("PCIE_INT_INTERFACE_R_X{xx}Y{y}", xx = x - 2)],
                        db.get_node_naming("INTF.PCIE_R"),
                        &[(col - 2, row)],
                    );
                }
                if br.to_idx() != 0 {
                    grid.fill_term_anon((col - 1, br - 1), "TERM.N");
                    grid.fill_term_anon((col, br - 1), "TERM.N");
                }
                grid.fill_term_anon((col - 1, br + 20), "TERM.S");
                grid.fill_term_anon((col, br + 20), "TERM.S");
            }
        }

        let row_b = grid.rows().next().unwrap();
        let row_t = grid.rows().next_back().unwrap();
        for col in grid.cols() {
            if !grid[(col, row_b)].nodes.is_empty() {
                grid.fill_term_anon((col, row_b), "TERM.S.HOLE");
            }
            if !grid[(col, row_t)].nodes.is_empty() {
                grid.fill_term_anon((col, row_t), "TERM.N.HOLE");
            }
        }
        let col_l = grid.cols().next().unwrap();
        let col_r = grid.cols().next_back().unwrap();
        for row in grid.rows() {
            grid.fill_term_anon((col_l, row), "TERM.W");
            grid.fill_term_anon((col_r, row), "TERM.E");
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
            let mut ey = 0;
            for &row in &hard.rows_emac {
                hard_skip.insert(row);
                hard_skip.insert(row + 5);
                if disabled.contains(&DisabledPart::Emac(row)) {
                    continue;
                }
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
                node.add_bel(0, format!("TEMAC_X0Y{ey}"));
                ey += 1;
            }
            for (i, &row) in hard.rows_pcie.iter().enumerate() {
                for dy in [0, 5, 10, 15] {
                    hard_skip.insert(row + dy);
                }
                let x = hard.col.to_idx() - 2;
                let y = row.to_idx();
                let mut crds = vec![];
                for dy in 0..20 {
                    crds.push((hard.col - 3, row + dy));
                }
                for dy in 0..20 {
                    crds.push((hard.col - 2, row + dy));
                }
                let name = format!("PCIE_X{x}Y{y}", y = y + 10);
                let node = grid[crds[0]].add_xnode(
                    db.get_node("PCIE"),
                    &[&name],
                    db.get_node_naming("PCIE"),
                    &crds,
                );
                node.add_bel(0, format!("PCIE_X0Y{i}"));
            }
        }

        let mut bx = 0;
        let mut dx = 0;
        for (col, &cd) in &self.columns {
            let kind = match cd {
                ColumnKind::Bram => "BRAM",
                ColumnKind::Dsp => "DSP",
                _ => continue,
            };
            for row in grid.rows() {
                if row.to_idx() % 5 != 0 {
                    continue;
                }
                if let Some(ref hard) = self.col_hard {
                    if hard.col == col && hard_skip.contains(&row) {
                        continue;
                    }
                }
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("{kind}_X{x}Y{y}");
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
                    node.add_bel(1, format!("RAMB18_X{bx}Y{sy}", sy = y / 5 * 2));
                    node.add_bel(2, format!("RAMB18_X{bx}Y{sy}", sy = y / 5 * 2 + 1));
                } else {
                    node.add_bel(0, format!("DSP48_X{dx}Y{sy}", sy = y / 5 * 2));
                    node.add_bel(1, format!("DSP48_X{dx}Y{sy}", sy = y / 5 * 2 + 1));
                    let tx = tiexlut[col] + 1;
                    node.add_bel(2, format!("TIEOFF_X{tx}Y{y}"));
                }
                if kind == "BRAM" && row.to_idx() % 40 == 20 {
                    let mut hy = y - 1;
                    if let Some(ref hard) = self.col_hard {
                        if hard.col == col && hard.rows_pcie.contains(&(row - 20)) {
                            hy = y;
                        }
                    }
                    let name_h = format!("HCLK_BRAM_X{x}Y{hy}");
                    let name_1 = format!("BRAM_X{x}Y{y}", y = y + 5);
                    let name_2 = format!("BRAM_X{x}Y{y}", y = y + 10);
                    let coords: Vec<_> = (0..15).map(|dy| (col, row + dy)).collect();
                    let node = grid[(col, row)].add_xnode(
                        db.get_node("PMVBRAM"),
                        &[&name_h, &name, &name_1, &name_2],
                        db.get_node_naming("PMVBRAM"),
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

        for col in grid.cols() {
            for row in grid.rows() {
                let crow = RowId::from_idx(if row.to_idx() % 40 < 20 {
                    row.to_idx() / 40 * 40 + 19
                } else {
                    row.to_idx() / 40 * 40 + 20
                });
                grid[(col, row)].clkroot = (col, crow);
            }
        }

        egrid
    }
}