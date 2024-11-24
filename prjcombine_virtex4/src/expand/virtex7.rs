use bimap::BiHashMap;
use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ColId, DieId, ExpandedDieRefMut, ExpandedGrid, Rect, RowId, TileIobId};
use prjcombine_virtex_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo, FrameMaskMode,
};
use std::collections::BTreeSet;
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::bond::SharedCfgPin;
use crate::expanded::{DieFrameGeom, ExpandedDevice, Gtz, IoCoord};
use crate::grid::{ColumnKind, DisabledPart, Grid, GtKind, GtzLoc, Interposer, IoKind, Pcie2Kind};

struct DieExpander<'a, 'b, 'c> {
    grid: &'b Grid,
    die: ExpandedDieRefMut<'a, 'b>,
    site_holes: Vec<Rect>,
    int_holes: Vec<Rect>,
    frame_info: Vec<FrameInfo>,
    frames: DieFrameGeom,
    col_cfg: ColId,
    col_clk: ColId,
    io: &'c mut Vec<IoCoord>,
    gt: &'c mut Vec<(DieId, ColId, RowId)>,
}

impl DieExpander<'_, '_, '_> {
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
        let row_cm = self.grid.row_reg_bot(self.grid.reg_cfg);
        let row_cb = row_cm - 50;
        let row_ct = row_cm + 50;
        if self.grid.regs == 1 {
            self.int_holes.push(Rect {
                col_l: self.col_cfg - 6,
                col_r: self.col_cfg,
                row_b: row_cb,
                row_t: row_cm,
            });
            self.site_holes.push(Rect {
                col_l: self.col_cfg - 6,
                col_r: self.col_cfg,
                row_b: row_cb,
                row_t: row_cm,
            });
        } else {
            self.int_holes.push(Rect {
                col_l: self.col_cfg - 6,
                col_r: self.col_cfg,
                row_b: row_cb,
                row_t: row_ct,
            });
            self.site_holes.push(Rect {
                col_l: self.col_cfg - 6,
                col_r: self.col_cfg,
                row_b: row_cb,
                row_t: row_ct,
            });
        }
        if self.grid.has_ps {
            let col_l = self.die.cols().next().unwrap();
            let row_t = self.die.rows().next_back().unwrap();
            let row_pb = row_t - 99;
            self.int_holes.push(Rect {
                col_l,
                col_r: col_l + 18,
                row_b: row_pb,
                row_t: row_pb + 100,
            });
            self.site_holes.push(Rect {
                col_l,
                col_r: col_l + 19,
                row_b: row_pb,
                row_t: row_pb + 100,
            });
        }
        for pcie2 in &self.grid.holes_pcie2 {
            self.site_holes.push(Rect {
                col_l: pcie2.col,
                col_r: pcie2.col + 4,
                row_b: pcie2.row,
                row_t: pcie2.row + 25,
            });
            self.int_holes.push(Rect {
                col_l: pcie2.col + 1,
                col_r: pcie2.col + 3,
                row_b: pcie2.row,
                row_t: pcie2.row + 25,
            });
        }
        for &(bc, br) in &self.grid.holes_pcie3 {
            self.int_holes.push(Rect {
                col_l: bc + 1,
                col_r: bc + 5,
                row_b: br,
                row_t: br + 50,
            });
            self.site_holes.push(Rect {
                col_l: bc,
                col_r: bc + 6,
                row_b: br,
                row_t: br + 50,
            });
        }
        for gtcol in &self.grid.cols_gt {
            let is_l = gtcol.col < self.col_clk;
            let is_m = if is_l {
                gtcol.col.to_idx() != 0
            } else {
                self.grid.columns.len() - gtcol.col.to_idx() > 7
            };
            for (reg, &kind) in &gtcol.regs {
                let br = self.grid.row_reg_bot(reg);
                if kind.is_some() {
                    if is_m {
                        if is_l {
                            self.int_holes.push(Rect {
                                col_l: gtcol.col + 1,
                                col_r: gtcol.col + 19,
                                row_b: br,
                                row_t: br + 50,
                            });
                            self.site_holes.push(Rect {
                                col_l: gtcol.col,
                                col_r: gtcol.col + 19,
                                row_b: br,
                                row_t: br + 50,
                            });
                        } else {
                            self.int_holes.push(Rect {
                                col_l: gtcol.col - 18,
                                col_r: gtcol.col,
                                row_b: br,
                                row_t: br + 50,
                            });
                            self.site_holes.push(Rect {
                                col_l: gtcol.col - 18,
                                col_r: gtcol.col + 1,
                                row_b: br,
                                row_t: br + 50,
                            });
                        }
                    } else if !is_l && gtcol.col != self.grid.columns.last_id().unwrap() {
                        self.site_holes.push(Rect {
                            col_l: gtcol.col,
                            col_r: gtcol.col + 7,
                            row_b: br,
                            row_t: br + 50,
                        });
                        self.int_holes.push(Rect {
                            col_l: gtcol.col + 1,
                            col_r: gtcol.col + 7,
                            row_b: br,
                            row_t: br + 50,
                        });
                    }
                }
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
                    ColumnKind::Bram => {
                        self.die.add_xnode((col, row), "INTF.BRAM", &[(col, row)]);
                    }
                    ColumnKind::Dsp
                    | ColumnKind::Cmt
                    | ColumnKind::Cfg
                    | ColumnKind::Clk
                    | ColumnKind::Io => {
                        self.die.add_xnode((col, row), "INTF", &[(col, row)]);
                    }
                    ColumnKind::Gt => (),
                }
            }
        }
    }

    fn fill_cfg(&mut self) {
        let row_cm = self.grid.row_reg_bot(self.grid.reg_cfg);
        let row_cb: RowId = row_cm - 50;
        let row_ct: RowId = row_cm + 50;
        if self.grid.regs != 1 {
            for dx in 0..6 {
                let col = self.col_cfg - 6 + dx;
                if row_cb.to_idx() != 0 {
                    self.die.fill_term((col, row_cb - 1), "TERM.N");
                }
                if row_ct.to_idx() != self.grid.regs * 50 {
                    self.die.fill_term((col, row_ct), "TERM.S");
                }
            }
        }

        let crds: [_; 50] = core::array::from_fn(|dy| (self.col_cfg, row_cb + dy));
        self.die.add_xnode((self.col_cfg, row_cb), "CFG", &crds);

        if self.grid.regs != 1 {
            let row_m = row_cm + 25;
            let crds: [_; 25] = core::array::from_fn(|dy| (self.col_cfg, row_m + dy));
            self.die.add_xnode((self.col_cfg, row_m), "XADC", &crds);
        }
    }

    fn fill_ps(&mut self) {
        if self.grid.has_ps {
            let col_l = self.die.cols().next().unwrap();
            let row_t = self.die.rows().next_back().unwrap();
            let row_pb = row_t - 99;
            if self.grid.regs != 2 {
                for dx in 0..18 {
                    let col = col_l + dx;
                    self.die.fill_term((col, row_pb - 1), "TERM.N");
                }
            }
            let col = col_l + 18;
            for dy in 0..100 {
                let row = row_pb + dy;
                self.die.fill_term((col, row), "TERM.W");
                self.die.add_xnode((col, row), "INTF", &[(col, row)]);
            }

            let crds: [_; 100] = core::array::from_fn(|dy| (col, row_pb + dy));
            self.die.add_xnode((col, row_pb + 50), "PS", &crds);
        }
    }

    fn fill_pcie2(&mut self) {
        for pcie2 in &self.grid.holes_pcie2 {
            for dx in 1..3 {
                let col = pcie2.col + dx;
                if pcie2.row.to_idx() != 0 {
                    self.die.fill_term((col, pcie2.row - 1), "TERM.N");
                }
                self.die.fill_term((col, pcie2.row + 25), "TERM.S");
            }
            let col_l = pcie2.col;
            let col_r = pcie2.col + 3;
            for dy in 0..25 {
                let row = pcie2.row + dy;
                self.die
                    .add_xnode((col_l, row), "INTF.DELAY", &[(col_l, row)]);
                self.die
                    .add_xnode((col_r, row), "INTF.DELAY", &[(col_r, row)]);
            }
            let mut crds = vec![];
            match pcie2.kind {
                Pcie2Kind::Left => {
                    for dy in 0..25 {
                        crds.push((pcie2.col + 3, pcie2.row + dy));
                    }
                    for dy in 0..25 {
                        crds.push((pcie2.col, pcie2.row + dy));
                    }
                }
                Pcie2Kind::Right => {
                    for dy in 0..25 {
                        crds.push((pcie2.col, pcie2.row + dy));
                    }
                    for dy in 0..25 {
                        crds.push((pcie2.col + 3, pcie2.row + dy));
                    }
                }
            }
            self.die.add_xnode(crds[0], "PCIE", &crds);
        }
    }

    fn fill_pcie3(&mut self) {
        for &(bc, br) in &self.grid.holes_pcie3 {
            for dx in 1..5 {
                let col = bc + dx;
                self.die.fill_term((col, br - 1), "TERM.N");
                self.die.fill_term((col, br + 50), "TERM.S");
            }
            let col_l = bc;
            let col_r = bc + 5;
            for dy in 0..50 {
                let row = br + dy;
                self.die
                    .add_xnode((col_l, row), "INTF.DELAY", &[(col_l, row)]);
                self.die
                    .add_xnode((col_r, row), "INTF.DELAY", &[(col_r, row)]);
            }
            let mut crds = vec![];
            for dy in 0..50 {
                crds.push((bc, br + dy));
            }
            for dy in 0..50 {
                crds.push((bc + 5, br + dy));
            }
            self.die.add_xnode(crds[0], "PCIE3", &crds);
        }
    }

    fn fill_gt(&mut self) {
        for gtcol in &self.grid.cols_gt {
            let is_l = gtcol.col < self.col_clk;
            for (reg, &kind) in &gtcol.regs {
                let br = self.grid.row_reg_bot(reg);
                if let Some(kind) = kind {
                    let sk = match kind {
                        GtKind::Gtp => "GTP",
                        GtKind::Gtx => "GTX",
                        GtKind::Gth => "GTH",
                    };
                    if gtcol.is_middle {
                        assert_eq!(kind, GtKind::Gtp);
                        if is_l {
                            for dx in 1..19 {
                                let col = gtcol.col + dx;
                                if br.to_idx() != 0 {
                                    self.die.fill_term((col, br - 1), "TERM.N");
                                }
                                if br.to_idx() + 50 != self.grid.regs * 50 {
                                    self.die.fill_term((col, br + 50), "TERM.S");
                                }
                            }
                            let col_l = gtcol.col;
                            let col_r = gtcol.col + 19;
                            for dy in 0..50 {
                                let row = br + dy;
                                self.die
                                    .add_xnode((col_l, row), "INTF.DELAY", &[(col_l, row)]);
                                self.die.fill_term((col_l, row), "TERM.E");
                                self.die.fill_term((col_r, row), "TERM.W");
                            }
                        } else {
                            for dx in 1..19 {
                                let col = gtcol.col - 19 + dx;
                                if br.to_idx() != 0 {
                                    self.die.fill_term((col, br - 1), "TERM.N");
                                }
                                if br.to_idx() + 50 != self.grid.regs * 50 {
                                    self.die.fill_term((col, br + 50), "TERM.S");
                                }
                            }
                            let col_l = gtcol.col - 19;
                            let col_r = gtcol.col;
                            for dy in 0..50 {
                                let row = br + dy;
                                self.die
                                    .add_xnode((col_r, row), "INTF.DELAY", &[(col_r, row)]);
                                self.die.fill_term((col_l, row), "TERM.E");
                                self.die.fill_term((col_r, row), "TERM.W");
                            }
                        }
                    } else if is_l {
                        for dy in 0..50 {
                            let row = br + dy;
                            self.die
                                .add_xnode((gtcol.col, row), "INTF.DELAY", &[(gtcol.col, row)]);
                        }
                    } else {
                        if gtcol.col != self.grid.columns.last_id().unwrap() {
                            if reg.to_idx() != 0 && gtcol.regs[reg - 1].is_none() {
                                for dx in 1..7 {
                                    self.die.fill_term((gtcol.col + dx, br - 1), "TERM.N");
                                }
                            }
                            if reg.to_idx() != self.grid.regs - 1 && gtcol.regs[reg + 1].is_none() {
                                for dx in 1..7 {
                                    self.die.fill_term((gtcol.col + dx, br + 50), "TERM.S");
                                }
                            }
                            for dy in 0..50 {
                                self.die.fill_term((gtcol.col, br + dy), "TERM.E");
                            }
                        }
                        for dy in 0..50 {
                            let row = br + dy;
                            self.die
                                .add_xnode((gtcol.col, row), "INTF.DELAY", &[(gtcol.col, row)]);
                        }
                    }
                    let ksuf = if gtcol.is_middle { "_MID" } else { "" };
                    for dy in [0, 11, 28, 39] {
                        let row = br + dy;
                        let crds: [_; 11] = core::array::from_fn(|dy| (gtcol.col, row + dy));
                        self.die
                            .add_xnode((gtcol.col, row), &format!("{sk}_CHANNEL{ksuf}"), &crds);
                    }
                    let row = br + 22;
                    let crds: [_; 6] = core::array::from_fn(|dy| (gtcol.col, row + dy));
                    self.die
                        .add_xnode((gtcol.col, row + 3), &format!("{sk}_COMMON{ksuf}"), &crds);

                    self.gt.push((self.die.die, gtcol.col, row + 3));
                }
                if br.to_idx() != 0
                    && (kind.is_some() || gtcol.regs[reg - 1].is_some())
                    && !gtcol.is_middle
                {
                    self.die.add_xnode((gtcol.col, br), "BRKH_GTX", &[]);
                }
            }
        }
    }

    fn fill_terms(&mut self) {
        let col_l = self.die.cols().next().unwrap();
        let col_r = self.die.cols().next_back().unwrap();
        let row_b = self.die.rows().next().unwrap();
        let row_t = self.die.rows().next_back().unwrap();
        for col in self.die.cols() {
            if !self.is_int_hole(col, row_b) {
                if self.grid.has_no_tbuturn {
                    self.die.fill_term((col, row_b), "TERM.S.HOLE");
                } else {
                    self.die.fill_term((col, row_b), "TERM.S");
                }
            }
            if !self.is_int_hole(col, row_t) {
                if self.grid.has_no_tbuturn {
                    self.die.fill_term((col, row_t), "TERM.N.HOLE");
                } else {
                    self.die.fill_term((col, row_t), "TERM.N");
                }
            }
        }
        for row in self.die.rows() {
            if !self.is_int_hole(col_l, row) {
                self.die.fill_term((col_l, row), "TERM.W");
            }
            if !self.is_int_hole(col_r, row) {
                self.die.fill_term((col_r, row), "TERM.E");
            }
        }
        for reg in 1..self.grid.regs {
            let row_s = RowId::from_idx(reg * 50 - 1);
            let row_n = RowId::from_idx(reg * 50);
            for col in self.die.cols() {
                if !self.is_int_hole(col, row_s) && !self.is_int_hole(col, row_n) {
                    self.die
                        .fill_term_pair((col, row_s), (col, row_n), "BRKH.N", "BRKH.S");
                }
            }
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

    fn fill_bram_dsp(&mut self) {
        let col = self.grid.columns.first_id().unwrap();
        if self.grid.columns[col] == ColumnKind::Bram {
            self.site_holes.extend([
                Rect {
                    col_l: col,
                    col_r: col + 1,
                    row_b: RowId::from_idx(0),
                    row_t: RowId::from_idx(5),
                },
                Rect {
                    col_l: col,
                    col_r: col + 1,
                    row_b: RowId::from_idx(self.die.rows().len() - 5),
                    row_t: RowId::from_idx(self.die.rows().len()),
                },
            ]);
        }
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
                if self.is_site_hole(col, row) {
                    continue;
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
                if cd == ColumnKind::Bram && row.to_idx() % 50 == 25 {
                    let coords: Vec<_> = (0..15).map(|dy| (col, row + dy)).collect();
                    self.die.add_xnode((col, row), "PMVBRAM", &coords);
                }
            }
            if cd == ColumnKind::Bram {
                for row in self.die.rows() {
                    if row.to_idx() % 50 != 25 {
                        continue;
                    }
                    if self.is_site_hole(col, row - 1) {
                        continue;
                    }
                    if !self.is_site_hole(col, row) {
                        continue;
                    }
                    self.die.add_xnode((col, row), "PMVBRAM_NC", &[]);
                }
            }
        }
    }

    fn fill_io(&mut self) {
        for iocol in self.grid.cols_io.iter() {
            let col = iocol.col;
            for row in self.die.rows() {
                let reg = self.grid.row_to_reg(row);
                if let Some(kind) = iocol.regs[reg] {
                    if matches!(row.to_idx() % 50, 0 | 49) {
                        self.die.add_xnode(
                            (col, row),
                            if row.to_idx() % 50 == 0 {
                                if kind == IoKind::Hpio {
                                    "IO_HP_BOT"
                                } else {
                                    "IO_HR_BOT"
                                }
                            } else {
                                if kind == IoKind::Hpio {
                                    "IO_HP_TOP"
                                } else {
                                    "IO_HR_TOP"
                                }
                            },
                            &[(col, row)],
                        );
                        self.io.push(IoCoord {
                            die: self.die.die,
                            col,
                            row,
                            iob: TileIobId::from_idx(0),
                        });
                    } else if row.to_idx() % 2 == 1 {
                        self.die.add_xnode(
                            (col, row),
                            if kind == IoKind::Hpio {
                                "IO_HP_PAIR"
                            } else {
                                "IO_HR_PAIR"
                            },
                            &[(col, row), (col, row + 1)],
                        );
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

                    if row.to_idx() % 50 == 25 {
                        let crds: [_; 8] = core::array::from_fn(|dy| (col, row - 4 + dy));
                        self.die.add_xnode(
                            (col, row),
                            match kind {
                                IoKind::Hpio => "HCLK_IOI_HP",
                                IoKind::Hrio => "HCLK_IOI_HR",
                            },
                            &crds,
                        );
                    }
                }
            }
        }
    }

    fn fill_cmt(&mut self) {
        for (col, &cd) in &self.grid.columns {
            if cd != ColumnKind::Cmt {
                continue;
            }
            for reg in self.grid.regs() {
                let row = self.grid.row_reg_hclk(reg);
                if self.is_site_hole(col, row) {
                    continue;
                }
                let crds: [_; 50] = core::array::from_fn(|dy| (col, row - 25 + dy));
                self.die.add_xnode((col, row), "CMT", &crds);

                for row in [row - 24, row - 12, row, row + 12] {
                    let crds: [_; 12] = core::array::from_fn(|dy| (col, row + dy));
                    self.die.add_xnode((col, row), "CMT_FIFO", &crds);
                }
            }
        }
    }

    fn fill_clk(&mut self) {
        let col = self.col_clk;
        for reg in self.grid.regs() {
            let row_h = self.grid.row_reg_hclk(reg);
            if self.grid.has_slr && reg.to_idx() == 0 {
                self.die.add_xnode((col, row_h - 21), "CLK_BALI_REBUF", &[]);
            } else {
                self.die.add_xnode((col, row_h - 13), "CLK_BUFG_REBUF", &[]);
            }

            self.die
                .add_xnode((col, row_h), "CLK_HROW", &[(col, row_h - 1), (col, row_h)]);

            if self.grid.has_slr && reg.to_idx() == self.grid.regs - 1 {
                self.die.add_xnode((col, row_h + 5), "CLK_BALI_REBUF", &[]);
            } else {
                self.die.add_xnode((col, row_h + 11), "CLK_BUFG_REBUF", &[]);
            }
        }

        let row = self.grid.row_bufg() - 4;
        let crds: [_; 4] = core::array::from_fn(|dy| (col, row + dy));
        self.die.add_xnode((col, row), "CLK_BUFG", &crds);
        if self.grid.reg_clk.to_idx() != self.grid.regs {
            let row = self.grid.row_bufg();
            let crds: [_; 4] = core::array::from_fn(|dy| (col, row + dy));
            self.die.add_xnode((col, row), "CLK_BUFG", &crds);
        }

        let pmv_base = if self.grid.regs == 1 { 0 } else { 1 };
        for (kind, dy) in [
            ("CLK_PMV", pmv_base + 3),
            ("CLK_PMVIOB", 17),
            ("CLK_PMV2_SVT", 32),
            ("CLK_PMV2", 41),
            ("CLK_MTBF2", 45),
        ] {
            let row = self.grid.row_bufg() - 50 + dy;
            self.die.add_xnode((col, row), kind, &[(col, row)]);
        }
    }

    fn fill_hclk(&mut self) {
        for col in self.die.cols() {
            if col.to_idx() % 2 != 0 {
                continue;
            }
            for row in self.die.rows() {
                if row.to_idx() % 50 == 25 {
                    let hole_bot = self.is_int_hole(col, row - 1);
                    let hole_top = self.is_int_hole(col, row);
                    if hole_bot && hole_top {
                        continue;
                    }
                    self.die.add_xnode((col, row), "HCLK", &[]);
                }

                if self.is_int_hole(col, row) {
                    continue;
                }
                self.die
                    .add_xnode((col, row), "INT_LCLK", &[(col, row), (col + 1, row)]);
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
                if let Some(gtcol) = self.grid.get_col_gt(col) {
                    if gtcol.regs[reg].is_some()
                        && (gtcol.col == self.grid.columns.last_id().unwrap()
                            || gtcol.col == self.grid.columns.last_id().unwrap() - 6)
                    {
                        self.frames.col_width[reg].push(32);
                        for minor in 0..32 {
                            let mut mask_mode = [FrameMaskMode::None; 2];
                            if matches!(minor, 28..32) {
                                mask_mode = [
                                    FrameMaskMode::DrpHclk(24, 13),
                                    FrameMaskMode::DrpHclk(25, 13),
                                ];
                            }
                            self.frame_info.push(FrameInfo {
                                addr: FrameAddr {
                                    typ: 0,
                                    region: if self.grid.regs == 1 {
                                        0
                                    } else {
                                        (reg - self.grid.reg_cfg) as i32
                                    },
                                    major: col.to_idx() as u32,
                                    minor,
                                },
                                mask_mode: mask_mode.into_iter().collect(),
                            });
                        }
                        break;
                    }
                }
                let width = match cd {
                    ColumnKind::ClbLL => 36,
                    ColumnKind::ClbLM => 36,
                    ColumnKind::Bram => 28,
                    ColumnKind::Dsp => 28,
                    ColumnKind::Io => 42,
                    ColumnKind::Cmt => 30,
                    ColumnKind::Cfg => 30,
                    ColumnKind::Clk => 30,
                    ColumnKind::Gt => 32,
                };
                self.frames.col_width[reg].push(width as usize);
                for minor in 0..width {
                    let mut mask_mode = [FrameMaskMode::None; 2];
                    for gt in &self.grid.cols_gt {
                        if gt.col == col && gt.regs[reg].is_some() && matches!(minor, 28..32) {
                            mask_mode = [
                                FrameMaskMode::DrpHclk(24, 13),
                                FrameMaskMode::DrpHclk(25, 13),
                            ];
                        }
                    }
                    if cd == ColumnKind::Cmt && matches!(minor, 28..30) {
                        mask_mode = [
                            FrameMaskMode::CmtDrpHclk(24, 13),
                            FrameMaskMode::CmtDrpHclk(25, 13),
                        ];
                    }
                    if cd == ColumnKind::Cfg && matches!(minor, 28..30) && reg == self.grid.reg_cfg
                    {
                        mask_mode[1] = FrameMaskMode::DrpHclk(25, 13);
                    }
                    for hole in &self.grid.holes_pcie2 {
                        match hole.kind {
                            Pcie2Kind::Left => {
                                if self.grid.row_reg_bot(reg) == hole.row
                                    && col == hole.col + 3
                                    && matches!(minor, 28..30)
                                {
                                    mask_mode[0] = FrameMaskMode::PcieLeftDrpHclk(24, 13);
                                }
                            }
                            Pcie2Kind::Right => {
                                if self.grid.row_reg_bot(reg) == hole.row
                                    && col == hole.col
                                    && matches!(minor, 28..30)
                                {
                                    mask_mode[0] = FrameMaskMode::DrpHclk(24, 13);
                                }
                            }
                        }
                    }
                    for &(hcol, hrow) in &self.grid.holes_pcie3 {
                        if self.grid.row_reg_hclk(reg) == hrow + 50
                            && col == hcol + 4
                            && matches!(minor, 28..30)
                        {
                            mask_mode[0] = FrameMaskMode::DrpHclk(24, 13);
                        }
                        if self.grid.row_reg_hclk(reg) == hrow
                            && col == hcol + 4
                            && matches!(minor, 28..30)
                        {
                            mask_mode[1] = FrameMaskMode::DrpHclk(24, 13);
                        }
                    }
                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 0,
                            region: if self.grid.regs == 1 {
                                0
                            } else {
                                (reg - self.grid.reg_cfg) as i32
                            },
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
                if let Some(gtcol) = self.grid.get_col_gt(col) {
                    if gtcol.col != self.grid.columns.last_id().unwrap()
                        && gtcol.regs[reg].is_some()
                    {
                        break;
                    }
                }
                self.frames.bram_frame[reg].insert(col, self.frame_info.len());
                for minor in 0..128 {
                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 1,
                            region: if self.grid.regs == 1 {
                                0
                            } else {
                                (reg - self.grid.reg_cfg) as i32
                            },
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
    interposer: &'a Interposer,
    disabled: &BTreeSet<DisabledPart>,
    db: &'a IntDb,
) -> ExpandedDevice<'a> {
    let mut egrid = ExpandedGrid::new(db);
    let pgrid = &grids[interposer.primary];
    let mut bank = (15
        - grids
            .iter()
            .filter_map(|(die, grid)| {
                if die < interposer.primary {
                    Some(grid.regs)
                } else {
                    None
                }
            })
            .sum::<usize>()) as u32;
    let mut frames = EntityVec::new();
    let mut die_bs_geom = EntityVec::new();

    let col_cfg = pgrid
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
    let col_clk = pgrid
        .columns
        .iter()
        .find_map(|(col, &cd)| {
            if cd == ColumnKind::Clk {
                Some(col)
            } else {
                None
            }
        })
        .unwrap();
    let col_lio = pgrid.columns.iter().find_map(|(col, &cd)| {
        if cd == ColumnKind::Io && col < col_cfg {
            Some(col)
        } else {
            None
        }
    });
    let col_rio = pgrid.columns.iter().find_map(|(col, &cd)| {
        if cd == ColumnKind::Io && col > col_cfg {
            Some(col)
        } else {
            None
        }
    });
    let mut col_mgt = None;
    let mut col_lgt = None;
    let mut col_rgt = None;
    if pgrid.cols_gt.len() == 2 && pgrid.cols_gt[0].col.to_idx() != 0 {
        col_mgt = Some((pgrid.cols_gt[0].col, pgrid.cols_gt[1].col));
    } else {
        col_lgt = pgrid.cols_gt.iter().find_map(|gtcol| {
            if gtcol.col < col_cfg {
                Some(gtcol.col)
            } else {
                None
            }
        });
        col_rgt = pgrid.cols_gt.iter().find_map(|gtcol| {
            if gtcol.col > col_cfg {
                Some(gtcol.col)
            } else {
                None
            }
        });
    }

    let mut io = vec![];
    let mut gt = vec![];

    let mut int_holes = EntityVec::new();
    let mut site_holes = EntityVec::new();
    let mut banklut = EntityVec::new();
    for &grid in grids.values() {
        let (_, die) = egrid.add_die(grid.columns.len(), grid.regs * 50);

        let mut de = DieExpander {
            grid,
            die,
            site_holes: Vec::new(),
            int_holes: Vec::new(),
            frame_info: vec![],
            frames: DieFrameGeom {
                col_frame: EntityVec::new(),
                col_width: EntityVec::new(),
                bram_frame: EntityVec::new(),
                spine_frame: EntityVec::new(),
            },
            col_cfg,
            col_clk,
            io: &mut io,
            gt: &mut gt,
        };

        de.fill_holes();
        de.fill_int();
        de.fill_cfg();
        de.fill_ps();
        de.fill_pcie2();
        de.fill_pcie3();
        de.fill_gt();
        de.fill_terms();
        de.die.fill_main_passes();
        de.fill_clb();
        de.fill_bram_dsp();
        de.fill_io();
        de.fill_cmt();
        de.fill_clk();
        de.fill_hclk();
        de.fill_frame_info();

        frames.push(de.frames);
        die_bs_geom.push(DieBitstreamGeom {
            frame_len: 50 * 64 + 32,
            frame_info: de.frame_info,
            bram_frame_len: 0,
            bram_frame_info: vec![],
            iob_frame_len: 0,
        });
        int_holes.push(de.int_holes);
        site_holes.push(de.site_holes);
        banklut.push(bank);
        bank += grid.regs as u32;
    }

    let lvb6 = db.wires.get("LVB.6").unwrap().0;
    for (die, &grid) in grids {
        if grid.has_no_tbuturn {
            for col in grid.columns.ids() {
                for i in 0..6 {
                    let row = RowId::from_idx(i);
                    egrid.blackhole_wires.insert((die, (col, row), lvb6));
                }
                for i in 0..6 {
                    let row = RowId::from_idx(grid.regs * 50 - 6 + i);
                    egrid.blackhole_wires.insert((die, (col, row), lvb6));
                }
            }
        }
    }

    let mut xdie_wires = BiHashMap::new();
    for i in 1..grids.len() {
        let dieid_s = DieId::from_idx(i - 1);
        let dieid_n = DieId::from_idx(i);
        let die_s = egrid.die(dieid_s);
        let die_n = egrid.die(dieid_n);
        for col in die_s.cols() {
            for dy in 0..49 {
                let row_s = die_s.rows().next_back().unwrap() - 49 + dy;
                let row_n = die_n.rows().next().unwrap() + 1 + dy;
                if !die_s[(col, row_s)].nodes.is_empty() && !die_n[(col, row_n)].nodes.is_empty() {
                    xdie_wires.insert((dieid_n, (col, row_n), lvb6), (dieid_s, (col, row_s), lvb6));
                }
            }
        }
    }
    egrid.xdie_wires = xdie_wires;

    let mut die_order = vec![];
    die_order.push(interposer.primary);
    for die in grids.ids() {
        if die != interposer.primary {
            die_order.push(die);
        }
    }

    let bs_geom = BitstreamGeom {
        kind: DeviceKind::Virtex7,
        die: die_bs_geom,
        die_order,
    };

    let mut cfg_io = BiHashMap::new();
    if pgrid.has_ps {
        cfg_io.insert(
            SharedCfgPin::PudcB,
            IoCoord {
                die: interposer.primary,
                col: col_rio.unwrap(),
                row: pgrid.row_reg_bot(pgrid.reg_cfg) - 50 + 43,
                iob: TileIobId::from_idx(1),
            },
        );
    } else {
        cfg_io.extend(
            [
                (1, 0, SharedCfgPin::Data(16)),
                (1, 1, SharedCfgPin::Data(17)),
                (3, 0, SharedCfgPin::Data(18)),
                (3, 1, SharedCfgPin::Data(19)),
                (5, 0, SharedCfgPin::Data(20)),
                (5, 1, SharedCfgPin::Data(21)),
                (7, 0, SharedCfgPin::Data(22)),
                (9, 0, SharedCfgPin::Data(23)),
                (9, 1, SharedCfgPin::Data(24)),
                (11, 0, SharedCfgPin::Data(25)),
                (11, 1, SharedCfgPin::Data(26)),
                (13, 0, SharedCfgPin::Data(27)),
                (13, 1, SharedCfgPin::Data(28)),
                (15, 0, SharedCfgPin::Data(29)),
                (15, 1, SharedCfgPin::Data(30)),
                (17, 0, SharedCfgPin::Data(31)),
                (17, 1, SharedCfgPin::CsiB),
                (19, 0, SharedCfgPin::CsoB),
                (19, 1, SharedCfgPin::RdWrB),
                (29, 0, SharedCfgPin::Data(15)),
                (29, 1, SharedCfgPin::Data(14)),
                (31, 0, SharedCfgPin::Data(13)),
                (33, 0, SharedCfgPin::Data(12)),
                (33, 1, SharedCfgPin::Data(11)),
                (35, 0, SharedCfgPin::Data(10)),
                (35, 1, SharedCfgPin::Data(9)),
                (37, 0, SharedCfgPin::Data(8)),
                (37, 1, SharedCfgPin::FcsB),
                (39, 0, SharedCfgPin::Data(7)),
                (39, 1, SharedCfgPin::Data(6)),
                (41, 0, SharedCfgPin::Data(5)),
                (41, 1, SharedCfgPin::Data(4)),
                (43, 0, SharedCfgPin::EmCclk),
                (43, 1, SharedCfgPin::PudcB),
                (45, 0, SharedCfgPin::Data(3)),
                (45, 1, SharedCfgPin::Data(2)),
                (47, 0, SharedCfgPin::Data(1)),
                (47, 1, SharedCfgPin::Data(0)),
                (51, 0, SharedCfgPin::Rs(0)),
                (51, 1, SharedCfgPin::Rs(1)),
                (53, 0, SharedCfgPin::FweB),
                (53, 1, SharedCfgPin::FoeB),
                (55, 0, SharedCfgPin::Addr(16)),
                (55, 1, SharedCfgPin::Addr(17)),
                (57, 0, SharedCfgPin::Addr(18)),
                (59, 0, SharedCfgPin::Addr(19)),
                (59, 1, SharedCfgPin::Addr(20)),
                (61, 0, SharedCfgPin::Addr(21)),
                (61, 1, SharedCfgPin::Addr(22)),
                (63, 0, SharedCfgPin::Addr(23)),
                (63, 1, SharedCfgPin::Addr(24)),
                (65, 0, SharedCfgPin::Addr(25)),
                (65, 1, SharedCfgPin::Addr(26)),
                (67, 0, SharedCfgPin::Addr(27)),
                (67, 1, SharedCfgPin::Addr(28)),
                (69, 0, SharedCfgPin::AdvB),
            ]
            .into_iter()
            .map(|(dy, iob, pin)| {
                (
                    pin,
                    IoCoord {
                        die: interposer.primary,
                        col: col_lio.unwrap(),
                        row: pgrid.row_reg_bot(pgrid.reg_cfg) - 50 + dy,
                        iob: TileIobId::from_idx(iob),
                    },
                )
            }),
        );
    }

    let mut gtz = vec![];
    if interposer.gtz_bot {
        let ipy = 0;
        let opy = 0;
        gtz.push(Gtz {
            loc: GtzLoc::Bottom,
            bank: 400,
            pads_rx: (0..8)
                .map(|i| {
                    (
                        format!("IPAD_X2Y{}", ipy + 5 + 2 * i),
                        format!("IPAD_X2Y{}", ipy + 4 + 2 * i),
                    )
                })
                .collect(),
            pads_tx: (0..8)
                .map(|i| {
                    (
                        format!("OPAD_X1Y{}", opy + 1 + 2 * i),
                        format!("OPAD_X1Y{}", opy + 2 * i),
                    )
                })
                .collect(),
            pads_clk: (0..2)
                .map(|i| {
                    (
                        format!("IPAD_X2Y{}", ipy + 1 + 2 * i),
                        format!("IPAD_X2Y{}", ipy + 2 * i),
                    )
                })
                .collect(),
        });
    }
    if interposer.gtz_top {
        let ipy = if interposer.gtz_bot { 20 } else { 0 };
        let opy = if interposer.gtz_bot { 16 } else { 0 };
        gtz.push(Gtz {
            loc: GtzLoc::Bottom,
            bank: 300,
            pads_rx: (0..8)
                .map(|i| {
                    (
                        format!("IPAD_X2Y{}", ipy + 5 + 2 * i),
                        format!("IPAD_X2Y{}", ipy + 4 + 2 * i),
                    )
                })
                .collect(),
            pads_tx: (0..8)
                .map(|i| {
                    (
                        format!("OPAD_X1Y{}", opy + 1 + 2 * i),
                        format!("OPAD_X1Y{}", opy + 2 * i),
                    )
                })
                .collect(),
            pads_clk: (0..2)
                .map(|i| {
                    (
                        format!("IPAD_X2Y{}", ipy + 1 + 2 * i),
                        format!("IPAD_X2Y{}", ipy + 2 * i),
                    )
                })
                .collect(),
        });
    }

    egrid.finish();
    ExpandedDevice {
        kind: pgrid.kind,
        grids: grids.clone(),
        egrid,
        interposer: Some(interposer),
        disabled: disabled.clone(),
        int_holes,
        site_holes,
        bs_geom,
        frames,
        col_cfg,
        col_clk,
        col_lio,
        col_rio,
        col_lcio: None,
        col_rcio: None,
        col_lgt,
        col_rgt,
        col_mgt,
        row_dcmiob: None,
        row_iobdcm: None,
        io,
        gt,
        gtz,
        cfg_io,
        banklut,
    }
}
