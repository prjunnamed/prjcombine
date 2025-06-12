use assert_matches::assert_matches;
use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::grid::{
    CellCoord, ColId, DieId, ExpandedDieRefMut, ExpandedGrid, Rect, RowId, TileIobId,
};
use prjcombine_xilinx_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo, FrameMaskMode,
};
use std::collections::BTreeSet;
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::bond::SharedCfgPad;
use crate::chip::{Chip, ColumnKind, DisabledPart, GtKind};

use crate::expanded::{DieFrameGeom, ExpandedDevice, IoCoord, REGION_HCLK, REGION_LEAF};
use crate::gtz::GtzDb;

struct Expander<'a, 'b> {
    chip: &'b Chip,
    die: ExpandedDieRefMut<'a, 'b>,
    site_holes: Vec<Rect>,
    int_holes: Vec<Rect>,
    frame_info: Vec<FrameInfo>,
    frames: DieFrameGeom,
    col_cfg: ColId,
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
        for &(bc, br) in &self.chip.holes_ppc {
            self.int_holes.push(Rect {
                col_l: bc + 1,
                col_r: bc + 13,
                row_b: br,
                row_t: br + 40,
            });
            self.site_holes.push(Rect {
                col_l: bc,
                col_r: bc + 14,
                row_b: br,
                row_t: br + 40,
            });
        }
        if let Some(ref hard) = self.chip.col_hard {
            let col = hard.col;
            for &row in &hard.rows_emac {
                self.site_holes.push(Rect {
                    col_l: col,
                    col_r: col + 1,
                    row_b: row,
                    row_t: row + 10,
                });
            }
            for &row in &hard.rows_pcie {
                self.site_holes.push(Rect {
                    col_l: col,
                    col_r: col + 1,
                    row_b: row,
                    row_t: row + 40,
                });
            }
        }
    }

    fn fill_int(&mut self) {
        for (col, &kind) in &self.chip.columns {
            for row in self.die.rows() {
                if self.is_int_hole(col, row) {
                    continue;
                }
                self.die.add_tile((col, row), "INT", &[(col, row)]);
                if self.is_site_hole(col, row) {
                    continue;
                }
                match kind {
                    ColumnKind::ClbLL => (),
                    ColumnKind::ClbLM => (),
                    ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::Io | ColumnKind::Cfg => {
                        self.die.add_tile((col, row), "INTF", &[(col, row)]);
                    }
                    ColumnKind::Gt => {
                        self.die.add_tile((col, row), "INTF.DELAY", &[(col, row)]);
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    fn fill_ppc(&mut self) {
        for &(bc, br) in &self.chip.holes_ppc {
            let col_l: ColId = bc;
            let col_r: ColId = bc + 13;
            for dy in 0..40 {
                let row: RowId = br + dy;
                self.die
                    .fill_conn_pair((col_l, row), (col_r, row), "PPC.E", "PPC.W");
                self.die
                    .add_tile((col_l, row), "INTF.DELAY", &[(col_l, row)]);
                self.die
                    .add_tile((col_r, row), "INTF.DELAY", &[(col_r, row)]);
            }
            let row_b: RowId = br - 1;
            let row_t: RowId = br + 40;
            for dx in 1..13 {
                let col: ColId = bc + dx;
                self.die.fill_conn_term((col, row_b), "TERM.N.PPC");
                self.die.fill_conn_term((col, row_t), "TERM.S.PPC");
            }
            let mut crds = vec![];
            for dy in 0..40 {
                crds.push((col_l, br + dy));
            }
            for dy in 0..40 {
                crds.push((col_r, br + dy));
            }
            self.die.add_tile((bc, br), "PPC", &crds);
        }
    }

    fn fill_terms(&mut self) {
        let row_b = self.die.rows().next().unwrap();
        let row_t = self.die.rows().next_back().unwrap();
        for col in self.die.cols() {
            self.die.fill_conn_term((col, row_b), "TERM.S.HOLE");
            self.die.fill_conn_term((col, row_t), "TERM.N.HOLE");
            self.die.fill_conn_pair(
                (col, row_t - 1),
                (col, row_t),
                "MAIN.NHOLE.N",
                "MAIN.NHOLE.S",
            );
        }
        let col_l = self.die.cols().next().unwrap();
        let col_r = self.die.cols().next_back().unwrap();
        for row in self.die.rows() {
            self.die.fill_conn_term((col_l, row), "TERM.W");
            if self.chip.columns[col_r] == ColumnKind::Gt {
                self.die.fill_conn_term((col_r, row), "TERM.E");
            } else {
                self.die.fill_conn_term((col_r, row), "TERM.E.HOLE");
            }
        }
    }

    fn fill_int_bufs(&mut self) {
        let col_l = self.die.cols().next().unwrap();
        let col_r = self.die.cols().next_back().unwrap();
        for (col, &cd) in &self.chip.columns {
            if !matches!(cd, ColumnKind::Io | ColumnKind::Cfg) || col == col_l || col == col_r {
                continue;
            }
            for row in self.die.rows() {
                self.die
                    .fill_conn_pair((col, row), (col + 1, row), "INT_BUFS.E", "INT_BUFS.W");
            }
        }
    }

    fn fill_clb(&mut self) {
        for (col, &cd) in &self.chip.columns {
            let kind = match cd {
                ColumnKind::ClbLL => "CLBLL",
                ColumnKind::ClbLM => "CLBLM",
                _ => continue,
            };
            for row in self.die.rows() {
                if self.is_site_hole(col, row) {
                    continue;
                }
                self.die.add_tile((col, row), kind, &[(col, row)]);
            }
        }
    }

    fn fill_hard(&mut self) {
        if let Some(ref hard) = self.chip.col_hard {
            let col = hard.col;
            for &row in &hard.rows_emac {
                for dy in 0..10 {
                    let row: RowId = row + dy;
                    self.die.add_tile((col, row), "INTF.DELAY", &[(col, row)]);
                }
                let crds: Vec<_> = (0..10).map(|dy| (col, row + dy)).collect();
                self.die.add_tile(crds[0], "EMAC", &crds);
            }
            for &row in &hard.rows_pcie {
                for dy in 0..40 {
                    let row: RowId = row + dy;
                    self.die.add_tile((col, row), "INTF.DELAY", &[(col, row)]);
                }
                let crds: Vec<_> = (0..40).map(|dy| (col, row + dy)).collect();
                self.die.add_tile(crds[0], "PCIE", &crds);
            }
        }
    }

    fn fill_bram_dsp(&mut self) {
        for (col, &cd) in &self.chip.columns {
            let kind = match cd {
                ColumnKind::Bram => "BRAM",
                ColumnKind::Dsp => "DSP",
                _ => continue,
            };
            for row in self.die.rows() {
                if row.to_idx() % 5 != 0 {
                    continue;
                }
                if self.is_site_hole(col, row) {
                    continue;
                }
                self.die.add_tile(
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
                if kind == "BRAM" && row.to_idx() % 20 == 10 {
                    if self.chip.cols_mgt_buf.contains(&col) {
                        self.die.add_tile((col, row), "HCLK_BRAM_MGT", &[]);
                    } else {
                        self.die.add_tile(
                            (col, row),
                            "PMVBRAM",
                            &[
                                (col, row),
                                (col, row + 1),
                                (col, row + 2),
                                (col, row + 3),
                                (col, row + 4),
                            ],
                        );
                    }
                }
            }
        }
    }

    fn fill_cfg(&mut self) {
        let col = self.col_cfg;
        let row = self.chip.row_reg_hclk(self.chip.reg_cfg - 1);
        self.site_holes.push(Rect {
            col_l: col,
            col_r: col + 1,
            row_b: row,
            row_t: row + 20,
        });
        let crds: [_; 20] = core::array::from_fn(|i| (col, row + i));
        self.die.add_tile((col, row), "CFG", &crds);
    }

    fn fill_cmt(&mut self) {
        let col = self.col_cfg;
        for row in self.chip.get_cmt_rows() {
            self.site_holes.push(Rect {
                col_l: col,
                col_r: col + 1,
                row_b: row,
                row_t: row + 10,
            });
            let crds: [_; 10] = core::array::from_fn(|i| (col, row + i));
            self.die.add_tile((col, row), "CMT", &crds);

            let kind = if row < self.chip.row_bufg() {
                "CLK_CMT_B"
            } else {
                "CLK_CMT_T"
            };
            self.die.add_tile((col, row), kind, &[]);
        }
    }

    fn fill_io(&mut self) {
        let row_ioi_cmt = if self.chip.reg_cfg.to_idx() == 1 {
            RowId::from_idx(0)
        } else {
            self.chip.row_bufg() - 30
        };
        let row_cmt_ioi = if self.chip.reg_cfg.to_idx() == self.chip.regs - 1 {
            RowId::from_idx(self.chip.regs * 20)
        } else {
            self.chip.row_bufg() + 30
        };
        let row_bot_cmt = if self.chip.reg_cfg.to_idx() < 3 {
            RowId::from_idx(0)
        } else {
            self.chip.row_bufg() - 60
        };
        let row_top_cmt = if (self.chip.regs - self.chip.reg_cfg.to_idx()) < 3 {
            RowId::from_idx(self.chip.regs * 20)
        } else {
            self.chip.row_bufg() + 60
        };
        for (col, &cd) in &self.chip.columns {
            if !matches!(cd, ColumnKind::Io | ColumnKind::Cfg) {
                continue;
            }
            for row in self.die.rows() {
                let cell = CellCoord::new(self.die.die, col, row);
                let is_cfg = col == self.col_cfg;
                if !self.is_site_hole(col, row) {
                    self.die.add_tile((col, row), "IO", &[(col, row)]);
                    self.io.extend([
                        IoCoord {
                            cell,
                            iob: TileIobId::from_idx(0),
                        },
                        IoCoord {
                            cell,
                            iob: TileIobId::from_idx(1),
                        },
                    ]);
                }

                if row.to_idx() % 20 == 10 {
                    if is_cfg {
                        self.die.add_tile((col, row), "CLK_HROW", &[]);

                        if row == self.chip.row_bufg() - 10 {
                            self.die.add_tile(
                                (col, row),
                                "HCLK_IOI_BOTCEN",
                                &[(col, row - 2), (col, row - 1)],
                            );
                        } else if row == self.chip.row_bufg() + 10 {
                            self.die.add_tile(
                                (col, row),
                                "HCLK_IOI_TOPCEN",
                                &[(col, row), (col, row + 1)],
                            );
                        } else if row == row_ioi_cmt {
                            self.die.add_tile(
                                (col, row),
                                "HCLK_IOI_CMT",
                                &[(col, row), (col, row + 1)],
                            );

                            self.die.add_tile((col, row), "HCLK_CMT", &[]);

                            self.die.add_tile((col, row), "CLK_IOB_B", &[]);
                        } else if row == row_cmt_ioi {
                            self.die.add_tile(
                                (col, row),
                                "HCLK_CMT_IOI",
                                &[(col, row - 2), (col, row - 1)],
                            );

                            self.die.add_tile((col, row), "HCLK_CMT", &[]);

                            self.die.add_tile((col, row - 10), "CLK_IOB_T", &[]);
                        } else if (row >= row_bot_cmt && row < row_ioi_cmt)
                            || (row >= row_cmt_ioi && row < row_top_cmt)
                        {
                            self.die.add_tile((col, row), "HCLK_CMT", &[]);
                        } else {
                            self.die.add_tile(
                                (col, row),
                                "HCLK_IOI_CENTER",
                                &[(col, row - 2), (col, row - 1), (col, row)],
                            );

                            if row < self.chip.row_bufg() {
                                self.die.add_tile((col, row), "CLK_MGT_B", &[]);
                            } else {
                                self.die.add_tile((col, row - 10), "CLK_MGT_T", &[]);
                            }
                        }
                    } else {
                        self.die.add_tile(
                            (col, row),
                            "HCLK_IOI",
                            &[(col, row - 2), (col, row - 1), (col, row), (col, row + 1)],
                        );
                    }
                }
            }
        }
    }

    fn fill_gt(&mut self) {
        for gtc in &self.chip.cols_gt {
            let col = gtc.col;
            for row in self.die.rows() {
                if row.to_idx() % 20 != 0 {
                    continue;
                }
                let reg = self.chip.row_to_reg(row);
                let kind = match gtc.regs[reg] {
                    Some(GtKind::Gtp) => "GTP",
                    Some(GtKind::Gtx) => "GTX",
                    _ => continue,
                };
                let crds: [_; 20] = core::array::from_fn(|i| (col, row + i));
                self.die.add_tile((col, row), kind, &crds);
                self.gt.push((self.die.die, col, row));
            }
        }
    }

    fn fill_hclk(&mut self) {
        for col in self.die.cols() {
            let col_hrow = if col <= self.col_cfg {
                self.col_cfg
            } else {
                self.col_cfg + 1
            };
            for row in self.die.rows() {
                let crow = self.chip.row_hclk(row);
                self.die[(col, row)].region_root[REGION_HCLK] = (col_hrow, crow);
                self.die[(col, row)].region_root[REGION_LEAF] = (col, crow);

                if row.to_idx() % 20 == 10 {
                    if self.is_int_hole(col, row) {
                        continue;
                    }
                    self.die.add_tile((col, row), "HCLK", &[(col, row)]);
                }
            }
        }
    }

    fn fill_frame_info(&mut self) {
        let mut regs: Vec<_> = self.chip.regs().collect();
        regs.sort_by_key(|&reg| {
            let rreg = reg - self.chip.reg_cfg;
            (rreg < 0, rreg.abs())
        });
        for _ in 0..self.chip.regs {
            self.frames.col_frame.push(EntityVec::new());
            self.frames.col_width.push(EntityVec::new());
            self.frames.bram_frame.push(EntityPartVec::new());
            self.frames.spine_frame.push(0);
        }
        for &reg in &regs {
            let mut major = 0;
            for (col, &cd) in &self.chip.columns {
                self.frames.col_frame[reg].push(self.frame_info.len());
                let width = match cd {
                    ColumnKind::ClbLL => 36,
                    ColumnKind::ClbLM => 36,
                    ColumnKind::Bram => 30,
                    ColumnKind::Dsp => 28,
                    ColumnKind::Io | ColumnKind::Cfg => 54,
                    ColumnKind::Gt => 32,
                    _ => unreachable!(),
                };
                self.frames.col_width[reg].push(width as usize);
                for minor in 0..width {
                    let mut mask_mode = [FrameMaskMode::None; 2];
                    if cd == ColumnKind::Gt && matches!(minor, 30 | 31) {
                        mask_mode[0] = FrameMaskMode::DrpHclk(28, 12);
                        mask_mode[1] = FrameMaskMode::DrpHclk(28, 12);
                    }
                    if cd == ColumnKind::Cfg && matches!(minor, 28 | 29) {
                        if reg + 3 == self.chip.reg_cfg || reg == self.chip.reg_cfg + 2 {
                            mask_mode[0] = FrameMaskMode::DrpHclk(27, 15);
                            mask_mode[1] = FrameMaskMode::DrpHclk(27, 15);
                        } else if reg + 2 == self.chip.reg_cfg {
                            mask_mode[0] = FrameMaskMode::DrpHclk(27, 15);
                        } else if reg == self.chip.reg_cfg + 1 {
                            mask_mode[1] = FrameMaskMode::DrpHclk(27, 15);
                        }
                    }
                    if cd == ColumnKind::Cfg && matches!(minor, 28..32) && reg == self.chip.reg_cfg
                    {
                        mask_mode[0] = FrameMaskMode::DrpHclk(27, 15);
                    }
                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 0,
                            region: (reg - self.chip.reg_cfg) as i32,
                            major,
                            minor,
                        },
                        mask_mode: mask_mode.into_iter().collect(),
                    });
                }
                major += 1;
                if col == self.col_cfg {
                    self.frames.spine_frame[reg] = self.frame_info.len();
                    for minor in 0..4 {
                        self.frame_info.push(FrameInfo {
                            addr: FrameAddr {
                                typ: 0,
                                region: (reg - self.chip.reg_cfg) as i32,
                                major,
                                minor,
                            },
                            mask_mode: [FrameMaskMode::None; 2].into_iter().collect(),
                        });
                    }
                    major += 1;
                }
            }
        }
        for &reg in &regs {
            let mut major = 0;
            for (col, &cd) in &self.chip.columns {
                if cd != ColumnKind::Bram {
                    continue;
                }
                self.frames.bram_frame[reg].insert(col, self.frame_info.len());
                for minor in 0..128 {
                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 1,
                            region: (reg - self.chip.reg_cfg) as i32,
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
    chips: &EntityVec<DieId, &'a Chip>,
    disabled: &BTreeSet<DisabledPart>,
    db: &'a IntDb,
    gdb: &'a GtzDb,
) -> ExpandedDevice<'a> {
    let mut egrid = ExpandedGrid::new(db);
    assert_eq!(chips.len(), 1);
    let chip = chips.first().unwrap();
    let col_cfg = chip
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
    let cols_io: Vec<_> = chip
        .columns
        .iter()
        .filter_map(|(col, &cd)| {
            if cd == ColumnKind::Io {
                Some(col)
            } else {
                None
            }
        })
        .collect();
    assert_matches!(cols_io.len(), 1 | 2);
    let col_lgt = chip
        .cols_gt
        .iter()
        .find(|gtc| gtc.col < col_cfg)
        .map(|x| x.col);
    let col_rgt = chip
        .cols_gt
        .iter()
        .find(|gtc| gtc.col > col_cfg)
        .map(|x| x.col);
    let (_, die) = egrid.add_die(chip.columns.len(), chip.regs * 20);

    let mut expander = Expander {
        chip,
        die,
        int_holes: vec![],
        site_holes: vec![],
        frame_info: vec![],
        frames: DieFrameGeom {
            col_frame: EntityVec::new(),
            col_width: EntityVec::new(),
            bram_frame: EntityVec::new(),
            spine_frame: EntityVec::new(),
        },
        col_cfg,
        io: vec![],
        gt: vec![],
    };

    expander.fill_holes();
    expander.fill_int();
    expander.fill_ppc();
    expander.fill_terms();
    expander.fill_int_bufs();
    expander.die.fill_main_passes();
    expander.fill_clb();
    expander.fill_hard();
    expander.fill_bram_dsp();
    expander.fill_cfg();
    expander.fill_cmt();
    expander.fill_io();
    expander.fill_gt();
    expander.fill_hclk();
    expander.fill_frame_info();

    let int_holes = expander.int_holes;
    let site_holes = expander.site_holes;
    let frames = expander.frames;
    let io = expander.io;
    let gt = expander.gt;
    let die_bs_geom = DieBitstreamGeom {
        frame_len: 64 * 20 + 32,
        frame_info: expander.frame_info,
        bram_frame_len: 0,
        bram_frame_info: vec![],
        iob_frame_len: 0,
    };
    let bs_geom = BitstreamGeom {
        kind: DeviceKind::Virtex5,
        die: [die_bs_geom].into_iter().collect(),
        die_order: vec![expander.die.die],
        has_gtz_bot: false,
        has_gtz_top: false,
    };

    let cfg_io = [
        (6, 0, SharedCfgPad::Data(8)),
        (6, 1, SharedCfgPad::Data(9)),
        (7, 0, SharedCfgPad::Data(10)),
        (7, 1, SharedCfgPad::Data(11)),
        (8, 0, SharedCfgPad::Data(12)),
        (8, 1, SharedCfgPad::Data(13)),
        (9, 0, SharedCfgPad::Data(14)),
        (9, 1, SharedCfgPad::Data(15)),
        (10, 0, SharedCfgPad::Data(0)),
        (10, 1, SharedCfgPad::Data(1)),
        (11, 0, SharedCfgPad::Data(2)),
        (11, 1, SharedCfgPad::Data(3)),
        (12, 0, SharedCfgPad::Data(4)),
        (12, 1, SharedCfgPad::Data(5)),
        (13, 0, SharedCfgPad::Data(6)),
        (13, 1, SharedCfgPad::Data(7)),
        (14, 0, SharedCfgPad::CsoB),
        (14, 1, SharedCfgPad::FweB),
        (15, 0, SharedCfgPad::FoeB),
        (15, 1, SharedCfgPad::FcsB),
        (16, 0, SharedCfgPad::Addr(20)),
        (16, 1, SharedCfgPad::Addr(21)),
        (17, 0, SharedCfgPad::Addr(22)),
        (17, 1, SharedCfgPad::Addr(23)),
        (18, 0, SharedCfgPad::Addr(24)),
        (18, 1, SharedCfgPad::Addr(25)),
        (19, 0, SharedCfgPad::Rs(0)),
        (19, 1, SharedCfgPad::Rs(1)),
        (40, 0, SharedCfgPad::Data(16)),
        (40, 1, SharedCfgPad::Data(17)),
        (41, 0, SharedCfgPad::Data(18)),
        (41, 1, SharedCfgPad::Data(19)),
        (42, 0, SharedCfgPad::Data(20)),
        (42, 1, SharedCfgPad::Data(21)),
        (43, 0, SharedCfgPad::Data(22)),
        (43, 1, SharedCfgPad::Data(23)),
        (44, 0, SharedCfgPad::Data(24)),
        (44, 1, SharedCfgPad::Data(25)),
        (45, 0, SharedCfgPad::Data(26)),
        (45, 1, SharedCfgPad::Data(27)),
        (46, 0, SharedCfgPad::Data(28)),
        (46, 1, SharedCfgPad::Data(29)),
        (47, 0, SharedCfgPad::Data(30)),
        (47, 1, SharedCfgPad::Data(31)),
        (48, 0, SharedCfgPad::Addr(16)),
        (48, 1, SharedCfgPad::Addr(17)),
        (49, 0, SharedCfgPad::Addr(18)),
        (49, 1, SharedCfgPad::Addr(19)),
    ]
    .into_iter()
    .map(|(dy, iob, pin)| {
        (
            pin,
            IoCoord {
                cell: CellCoord {
                    die: DieId::from_idx(0),
                    col: col_cfg,
                    row: chip.row_reg_bot(chip.reg_cfg) - 30 + dy,
                },
                iob: TileIobId::from_idx(iob),
            },
        )
    })
    .collect();

    egrid.finish();
    ExpandedDevice {
        kind: chip.kind,
        chips: chips.clone(),
        interposer: None,
        disabled: disabled.clone(),
        egrid,
        gdb,
        int_holes: [int_holes].into_iter().collect(),
        site_holes: [site_holes].into_iter().collect(),
        bs_geom,
        frames: [frames].into_iter().collect(),
        col_cfg,
        col_clk: col_cfg,
        col_lio: Some(cols_io[0]),
        col_rio: cols_io.get(1).copied(),
        col_lcio: None,
        col_rcio: None,
        col_lgt,
        col_rgt,
        col_mgt: None,
        row_dcmiob: None,
        row_iobdcm: None,
        io,
        gt,
        gtz: Default::default(),
        cfg_io,
        banklut: EntityVec::new(),
    }
}
