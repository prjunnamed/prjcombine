#![allow(clippy::comparison_chain)]

use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ColId, DieId, ExpandedDieRefMut, ExpandedGrid, Rect, RowId};
use prjcombine_virtex_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo, FrameMaskMode,
};
use std::collections::{BTreeSet, HashSet};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::bond::SharedCfgPin;
use crate::expanded::{DieFrameGeom, ExpandedDevice, IoCoord, TileIobId};
use crate::grid::{ColumnKind, DisabledPart, Grid, GtKind};

struct Expander<'a, 'b> {
    grid: &'b Grid,
    disabled: &'b BTreeSet<DisabledPart>,
    die: ExpandedDieRefMut<'a, 'b>,
    site_holes: Vec<Rect>,
    int_holes: Vec<Rect>,
    hard_skip: HashSet<RowId>,
    frame_info: Vec<FrameInfo>,
    frames: DieFrameGeom,
    col_cfg: ColId,
    col_lio: Option<ColId>,
    col_rio: Option<ColId>,
    col_lcio: Option<ColId>,
    col_rcio: Option<ColId>,
    io: Vec<IoCoord>,
    gt: Vec<(DieId, ColId, RowId)>,
}

impl Expander<'_, '_> {
    fn is_site_hole(&self, col: ColId, row: RowId) -> bool {
        for hole in &self.site_holes {
            if hole.contains(col, row) {
                return true;
            }
        }
        false
    }

    fn is_int_hole(&self, col: ColId, row: RowId) -> bool {
        for hole in &self.int_holes {
            if hole.contains(col, row) {
                return true;
            }
        }
        false
    }

    fn fill_holes(&mut self) {
        let row_b = self.grid.row_reg_bot(self.grid.reg_cfg - 1);
        let row_t = self.grid.row_reg_bot(self.grid.reg_cfg + 1);
        self.site_holes.push(Rect {
            col_l: self.col_cfg - 6,
            col_r: self.col_cfg,
            row_b,
            row_t,
        });
        self.int_holes.push(Rect {
            col_l: self.col_cfg - 6,
            col_r: self.col_cfg,
            row_b,
            row_t,
        });
        if let Some(ref hard) = self.grid.col_hard {
            let col = hard.col;
            for &row in &hard.rows_pcie {
                self.site_holes.push(Rect {
                    col_l: col - 3,
                    col_r: col + 1,
                    row_b: row,
                    row_t: row + 20,
                });
                self.int_holes.push(Rect {
                    col_l: col - 1,
                    col_r: col + 1,
                    row_b: row,
                    row_t: row + 20,
                });
            }
            for &row in &hard.rows_emac {
                self.site_holes.push(Rect {
                    col_l: col,
                    col_r: col + 1,
                    row_b: row,
                    row_t: row + 10,
                });
            }
        }
    }

    fn fill_int(&mut self) {
        for (col, &kind) in &self.grid.columns {
            for row in self.die.rows() {
                if self.is_int_hole(col, row) {
                    continue;
                }
                self.die.add_xnode((col, row), "INT", &[(col, row)]);
                if self.is_site_hole(col, row) {
                    continue;
                }
                match kind {
                    ColumnKind::ClbLL => (),
                    ColumnKind::ClbLM => (),
                    ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::Io | ColumnKind::Cfg => {
                        self.die.add_xnode((col, row), "INTF", &[(col, row)]);
                    }
                    ColumnKind::Gt => {
                        self.die.add_xnode((col, row), "INTF.DELAY", &[(col, row)]);
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    fn fill_cfg(&mut self) {
        let row_b = self.grid.row_reg_bot(self.grid.reg_cfg - 1);
        let row_t = self.grid.row_reg_bot(self.grid.reg_cfg + 1);
        for dx in 0..6 {
            let col = self.col_cfg - 6 + dx;
            if row_b.to_idx() != 0 {
                self.die.fill_term((col, row_b - 1), "TERM.N");
            }
            if row_t.to_idx() != self.grid.regs * 40 {
                self.die.fill_term((col, row_t), "TERM.S");
            }
        }
        let crds: [_; 80] = core::array::from_fn(|dy| (self.col_cfg, row_b + dy));
        self.die.add_xnode(crds[40], "CFG", &crds);
    }

    fn fill_btterm(&mut self) {
        let row_b = self.die.rows().next().unwrap();
        let row_t = self.die.rows().next_back().unwrap();
        for col in self.die.cols() {
            if !self.is_int_hole(col, row_b) {
                self.die.fill_term((col, row_b), "TERM.S.HOLE");
            }
            if !self.is_int_hole(col, row_t) {
                self.die.fill_term((col, row_t), "TERM.N.HOLE");
            }
        }
        let col_l = self.die.cols().next().unwrap();
        let col_r = self.die.cols().next_back().unwrap();
        for row in self.die.rows() {
            self.die.fill_term((col_l, row), "TERM.W");
            self.die.fill_term((col_r, row), "TERM.E");
        }
    }

    fn fill_clb(&mut self) {
        for (col, &cd) in &self.grid.columns {
            let kind = match cd {
                ColumnKind::ClbLL => "CLBLL",
                ColumnKind::ClbLM => "CLBLM",
                _ => continue,
            };
            for row in self.die.rows() {
                if self.is_site_hole(col, row) {
                    continue;
                }
                self.die.add_xnode((col, row), kind, &[(col, row)]);
            }
        }
    }

    fn fill_hard(&mut self) {
        if let Some(ref hard) = self.grid.col_hard {
            let col = hard.col;
            for &br in &hard.rows_emac {
                for dy in 0..10 {
                    let row: RowId = br + dy;
                    self.die.add_xnode((col, row), "INTF.DELAY", &[(col, row)]);
                }
                self.hard_skip.insert(br);
                self.hard_skip.insert(br + 5);
                if self.disabled.contains(&DisabledPart::Emac(br)) {
                    continue;
                }
                let crds: [_; 10] = core::array::from_fn(|dy| (hard.col, br + dy));
                self.die.add_xnode(crds[0], "EMAC", &crds);
            }

            for &br in &hard.rows_pcie {
                for dy in 0..20 {
                    let row: RowId = br + dy;
                    self.die
                        .add_xnode((col - 3, row), "INTF.DELAY", &[(col - 3, row)]);
                    self.die
                        .add_xnode((col - 2, row), "INTF.DELAY", &[(col - 2, row)]);
                }
                if br.to_idx() != 0 {
                    self.die.fill_term((col - 1, br - 1), "TERM.N");
                    self.die.fill_term((col, br - 1), "TERM.N");
                }
                self.die.fill_term((col - 1, br + 20), "TERM.S");
                self.die.fill_term((col, br + 20), "TERM.S");

                for dy in [0, 5, 10, 15] {
                    self.hard_skip.insert(br + dy);
                }
                let mut crds = vec![];
                for dy in 0..20 {
                    crds.push((hard.col - 3, br + dy));
                }
                for dy in 0..20 {
                    crds.push((hard.col - 2, br + dy));
                }
                self.die.add_xnode(crds[0], "PCIE", &crds);
            }
        }
    }

    fn fill_bram_dsp(&mut self) {
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
                self.die.add_xnode(
                    (col, row),
                    kind,
                    &[
                        (col, row),
                        (col, row + 1),
                        (col, row + 2),
                        (col, row + 3),
                        (col, row + 4),
                    ],
                );
                if kind == "BRAM" && row.to_idx() % 40 == 20 {
                    let coords: Vec<_> = (0..15).map(|dy| (col, row + dy)).collect();
                    self.die.add_xnode((col, row), "PMVBRAM", &coords);
                }
            }
        }
    }

    fn fill_io(&mut self) {
        for col in [self.col_lio, self.col_lcio, self.col_rcio, self.col_rio]
            .into_iter()
            .flatten()
        {
            for row in self.die.rows() {
                if row.to_idx() % 2 == 0 {
                    self.die
                        .add_xnode((col, row), "IO", &[(col, row), (col, row + 1)]);
                    self.io.extend([
                        IoCoord {
                            die: self.die.die,
                            col,
                            row,
                            iob: TileIobId::from_idx(0),
                        },
                        IoCoord {
                            die: self.die.die,
                            col,
                            row,
                            iob: TileIobId::from_idx(1),
                        },
                    ]);
                }

                if row.to_idx() % 40 == 20 {
                    self.die
                        .add_xnode((col, row), "HCLK_IOI", &[(col, row - 1), (col, row)]);
                }
            }
        }
    }

    fn fill_cmt(&mut self) {
        let col = self.col_cfg;
        for reg in self.grid.regs() {
            let row_hclk = self.grid.row_reg_hclk(reg);
            let crds: [_; 40] = core::array::from_fn(|dy| (col, row_hclk - 20 + dy));
            self.die.add_xnode((col, row_hclk), "CMT", &crds);
            let row: RowId = row_hclk - 20;
            if reg < self.grid.reg_cfg - 1 {
                self.die
                    .add_xnode((col, row), "PMVIOB", &[(col, row), (col, row + 1)]);
            } else if reg == self.grid.reg_cfg - 1 {
                // CMT_PMVB, empty
            } else if reg == self.grid.reg_cfg {
                self.die.add_xnode(
                    (col, row),
                    "CMT_BUFG_TOP",
                    &[(col, row), (col, row + 1), (col, row + 2)],
                );
            } else {
                self.die.add_xnode((col, row), "GCLK_BUF", &[]);
            }

            let row: RowId = row_hclk + 18;
            if reg < self.grid.reg_cfg - 1 {
                self.die.add_xnode((col, row + 2), "GCLK_BUF", &[]);
            } else if reg == self.grid.reg_cfg - 1 {
                self.die.add_xnode(
                    (col, row + 2),
                    "CMT_BUFG_BOT",
                    &[(col, row - 1), (col, row), (col, row + 1)],
                );
            } else {
                self.die
                    .add_xnode((col, row), "PMVIOB", &[(col, row), (col, row + 1)]);
            }
        }
    }

    fn fill_gt(&mut self) {
        for gtc in &self.grid.cols_gt {
            let col = gtc.col;
            for reg in self.grid.regs() {
                if self.disabled.contains(&DisabledPart::GtxRow(reg)) {
                    continue;
                }
                let row = self.grid.row_reg_hclk(reg);
                let crds: [_; 40] = core::array::from_fn(|dy| (col, row - 20 + dy));
                let kind = gtc.regs[reg].unwrap();
                match kind {
                    GtKind::Gtx => {
                        self.die.add_xnode((col, row), "GTX", &crds);
                    }
                    GtKind::Gth => {
                        self.die.add_xnode((col, row), "GTH", &crds);
                    }
                    _ => unreachable!(),
                }
                self.gt.push((self.die.die, col, row));
            }
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
                    self.die
                        .add_xnode((col, row), "HCLK", &[(col, row - 1), (col, row)]);
                    if col == self.grid.cols_qbuf.unwrap().0
                        || col == self.grid.cols_qbuf.unwrap().1
                    {
                        self.die.add_xnode((col, row), "HCLK_QBUF", &[]);
                    }
                    if self.grid.cols_mgt_buf.contains(&col) {
                        self.die.add_xnode((col, row), "MGT_BUF", &[]);
                    }
                }
            }
        }
    }

    fn fill_frame_info(&mut self) {
        let mut regs: Vec<_> = self.grid.regs().collect();
        regs.sort_by_key(|&reg| {
            let rreg = reg - self.grid.reg_cfg;
            (rreg < 0, rreg.abs())
        });
        for _ in 0..self.grid.regs {
            self.frames.col_frame.push(EntityVec::new());
            self.frames.col_width.push(EntityVec::new());
            self.frames.bram_frame.push(EntityPartVec::new());
        }
        for &reg in &regs {
            for (col, &cd) in &self.grid.columns {
                self.frames.col_frame[reg].push(self.frame_info.len());
                let width = match cd {
                    ColumnKind::ClbLL => 36,
                    ColumnKind::ClbLM => 36,
                    ColumnKind::Bram => 28,
                    ColumnKind::Dsp => 28,
                    ColumnKind::Io => 44,
                    ColumnKind::Cfg => 38,
                    ColumnKind::Gt => 30,
                    _ => unreachable!(),
                };
                self.frames.col_width[reg].push(width as usize);
                for minor in 0..width {
                    let mut mask_mode = [FrameMaskMode::None; 2];
                    if cd == ColumnKind::Gt && matches!(minor, 28 | 29) {
                        mask_mode[0] = FrameMaskMode::DrpHclk(24, 13);
                        mask_mode[1] = FrameMaskMode::DrpHclk(25, 13);
                    }
                    if cd == ColumnKind::Cfg && matches!(minor, 26 | 27) {
                        mask_mode[0] = FrameMaskMode::CmtDrpHclk(24, 13);
                        mask_mode[1] = FrameMaskMode::CmtDrpHclk(25, 13);
                    }
                    if cd == ColumnKind::Cfg && matches!(minor, 34 | 35) && reg == self.grid.reg_cfg
                    {
                        mask_mode[0] = FrameMaskMode::DrpHclk(23, 13);
                        mask_mode[1] = FrameMaskMode::DrpHclk(23, 13);
                    }
                    if let Some(ref hard) = self.grid.col_hard {
                        if col == hard.col
                            && hard.rows_pcie.contains(&self.grid.row_reg_bot(reg))
                            && matches!(minor, 26 | 27)
                        {
                            mask_mode[0] = FrameMaskMode::DrpHclk(24, 13);
                        }
                    }

                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 0,
                            region: (reg - self.grid.reg_cfg) as i32,
                            major: col.to_idx() as u32,
                            minor,
                        },
                        mask_mode: mask_mode.into_iter().collect(),
                    });
                }
            }
        }
        for &reg in &regs {
            let mut major = 0;
            for (col, &cd) in &self.grid.columns {
                if cd != ColumnKind::Bram {
                    continue;
                }
                self.frames.bram_frame[reg].insert(col, self.frame_info.len());
                for minor in 0..128 {
                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 1,
                            region: (reg - self.grid.reg_cfg) as i32,
                            major,
                            minor,
                        },
                        mask_mode: [FrameMaskMode::All; 2].into_iter().collect(),
                    });
                }
                major += 1;
            }
        }
    }
}

pub fn expand_grid<'a>(
    grids: &EntityVec<DieId, &'a Grid>,
    disabled: &BTreeSet<DisabledPart>,
    db: &'a IntDb,
) -> ExpandedDevice<'a> {
    let mut egrid = ExpandedGrid::new(db);
    assert_eq!(grids.len(), 1);
    let grid = grids.first().unwrap();
    let (_, die) = egrid.add_die(grid.columns.len(), grid.regs * 40);

    let col_cfg = grid
        .columns
        .iter()
        .find_map(|(col, &cd)| {
            if cd == ColumnKind::Cfg {
                Some(col)
            } else {
                None
            }
        })
        .unwrap();
    let cols_lio: Vec<_> = grid
        .columns
        .iter()
        .filter_map(|(col, &cd)| {
            if cd == ColumnKind::Io && col < col_cfg {
                Some(col)
            } else {
                None
            }
        })
        .collect();
    let (col_lio, col_lcio) = match *cols_lio {
        [lc] => (None, Some(lc)),
        [l, lc] => (Some(l), Some(lc)),
        _ => unreachable!(),
    };
    let cols_rio: Vec<_> = grid
        .columns
        .iter()
        .filter_map(|(col, &cd)| {
            if cd == ColumnKind::Io && col > col_cfg {
                Some(col)
            } else {
                None
            }
        })
        .collect();
    let (col_rio, col_rcio) = match *cols_rio {
        [rc] => (None, Some(rc)),
        [rc, r] => (Some(r), Some(rc)),
        _ => unreachable!(),
    };
    let col_lgt = grid
        .cols_gt
        .iter()
        .find(|gtc| gtc.col < col_cfg)
        .map(|x| x.col);
    let col_rgt = grid
        .cols_gt
        .iter()
        .find(|gtc| gtc.col > col_cfg)
        .map(|x| x.col);

    let mut expander = Expander {
        grid,
        disabled,
        die,
        int_holes: vec![],
        site_holes: vec![],
        hard_skip: HashSet::new(),
        frame_info: vec![],
        frames: DieFrameGeom {
            col_frame: EntityVec::new(),
            col_width: EntityVec::new(),
            bram_frame: EntityVec::new(),
            spine_frame: EntityVec::new(),
        },
        col_cfg,
        col_lio,
        col_rio,
        col_lcio,
        col_rcio,
        io: vec![],
        gt: vec![],
    };

    expander.fill_holes();
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
    expander.fill_frame_info();

    let int_holes = expander.int_holes;
    let site_holes = expander.site_holes;
    let frames = expander.frames;
    let io = expander.io;
    let gt = expander.gt;
    let die_bs_geom = DieBitstreamGeom {
        frame_len: 64 * 40 + 32,
        frame_info: expander.frame_info,
        bram_frame_len: 0,
        bram_frame_info: vec![],
        iob_frame_len: 0,
    };
    let bs_geom = BitstreamGeom {
        kind: DeviceKind::Virtex6,
        die: [die_bs_geom].into_iter().collect(),
        die_order: vec![expander.die.die],
    };

    let lcio = col_lcio.unwrap();
    let rcio = col_rcio.unwrap();
    let cfg_io = [
        (lcio, 6, 0, SharedCfgPin::CsoB),
        (lcio, 6, 1, SharedCfgPin::Rs(0)),
        (lcio, 8, 0, SharedCfgPin::Rs(1)),
        (lcio, 8, 1, SharedCfgPin::FweB),
        (lcio, 10, 0, SharedCfgPin::FoeB),
        (lcio, 10, 1, SharedCfgPin::FcsB),
        (lcio, 12, 0, SharedCfgPin::Data(0)),
        (lcio, 12, 1, SharedCfgPin::Data(1)),
        (lcio, 14, 0, SharedCfgPin::Data(2)),
        (lcio, 14, 1, SharedCfgPin::Data(3)),
        (lcio, 24, 0, SharedCfgPin::Data(4)),
        (lcio, 24, 1, SharedCfgPin::Data(5)),
        (lcio, 26, 0, SharedCfgPin::Data(6)),
        (lcio, 26, 1, SharedCfgPin::Data(7)),
        (lcio, 28, 0, SharedCfgPin::Data(8)),
        (lcio, 28, 1, SharedCfgPin::Data(9)),
        (lcio, 30, 0, SharedCfgPin::Data(10)),
        (lcio, 30, 1, SharedCfgPin::Data(11)),
        (lcio, 32, 0, SharedCfgPin::Data(12)),
        (lcio, 32, 1, SharedCfgPin::Data(13)),
        (lcio, 34, 0, SharedCfgPin::Data(14)),
        (lcio, 34, 1, SharedCfgPin::Data(15)),
        (rcio, 2, 0, SharedCfgPin::Addr(16)),
        (rcio, 2, 1, SharedCfgPin::Addr(17)),
        (rcio, 4, 0, SharedCfgPin::Addr(18)),
        (rcio, 4, 1, SharedCfgPin::Addr(19)),
        (rcio, 6, 0, SharedCfgPin::Addr(20)),
        (rcio, 6, 1, SharedCfgPin::Addr(21)),
        (rcio, 8, 0, SharedCfgPin::Addr(22)),
        (rcio, 8, 1, SharedCfgPin::Addr(23)),
        (rcio, 10, 0, SharedCfgPin::Addr(24)),
        (rcio, 10, 1, SharedCfgPin::Addr(25)),
        (rcio, 12, 0, SharedCfgPin::Data(16)),
        (rcio, 12, 1, SharedCfgPin::Data(17)),
        (rcio, 14, 0, SharedCfgPin::Data(18)),
        (rcio, 14, 1, SharedCfgPin::Data(19)),
        (rcio, 24, 0, SharedCfgPin::Data(20)),
        (rcio, 24, 1, SharedCfgPin::Data(21)),
        (rcio, 26, 0, SharedCfgPin::Data(22)),
        (rcio, 26, 1, SharedCfgPin::Data(23)),
        (rcio, 28, 0, SharedCfgPin::Data(24)),
        (rcio, 28, 1, SharedCfgPin::Data(25)),
        (rcio, 30, 0, SharedCfgPin::Data(26)),
        (rcio, 30, 1, SharedCfgPin::Data(27)),
        (rcio, 32, 0, SharedCfgPin::Data(28)),
        (rcio, 32, 1, SharedCfgPin::Data(29)),
        (rcio, 34, 0, SharedCfgPin::Data(30)),
        (rcio, 34, 1, SharedCfgPin::Data(31)),
    ]
    .into_iter()
    .map(|(col, dy, iob, pin)| {
        (
            pin,
            IoCoord {
                die: DieId::from_idx(0),
                col,
                row: grid.row_reg_bot(grid.reg_cfg) - 40 + dy,
                iob: TileIobId::from_idx(iob),
            },
        )
    })
    .collect();

    egrid.finish();
    ExpandedDevice {
        kind: grid.kind,
        grids: grids.clone(),
        interposer: None,
        disabled: disabled.clone(),
        int_holes: [int_holes].into_iter().collect(),
        site_holes: [site_holes].into_iter().collect(),
        egrid,
        bs_geom,
        frames: [frames].into_iter().collect(),
        col_cfg,
        col_clk: col_cfg,
        col_lio,
        col_rio,
        col_lcio,
        col_rcio,
        col_lgt,
        col_rgt,
        col_mgt: None,
        row_dcmiob: None,
        row_iobdcm: None,
        io,
        gt,
        gtz: vec![],
        cfg_io,
        banklut: EntityVec::new(),
    }
}
