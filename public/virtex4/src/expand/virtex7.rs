use bimap::BiHashMap;
use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::dir::{Dir, DirPartMap};
use prjcombine_interconnect::grid::{
    ColId, DieId, ExpandedDieRefMut, ExpandedGrid, Rect, RowId, TileIobId,
};
use prjcombine_xilinx_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo, FrameMaskMode,
};
use std::collections::BTreeSet;
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::bond::SharedCfgPad;
use crate::chip::{Chip, ColumnKind, DisabledPart, GtKind, Interposer, IoKind, Pcie2Kind};
use crate::expanded::{
    DieFrameGeom, ExpandedDevice, ExpandedGtz, IoCoord, REGION_HCLK, REGION_LEAF,
};
use crate::gtz::{GtzDb, GtzIntColId};

struct DieExpander<'a, 'b, 'c> {
    chip: &'b Chip,
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
        let row_cm = self.chip.row_reg_bot(self.chip.reg_cfg);
        let row_cb = row_cm - 50;
        let row_ct = row_cm + 50;
        if self.chip.regs == 1 {
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
        if self.chip.has_ps {
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
        for pcie2 in &self.chip.holes_pcie2 {
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
        for &(bc, br) in &self.chip.holes_pcie3 {
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
        for gtcol in &self.chip.cols_gt {
            let is_l = gtcol.col < self.col_clk;
            let is_m = if is_l {
                gtcol.col.to_idx() != 0
            } else {
                self.chip.columns.len() - gtcol.col.to_idx() > 7
            };
            for (reg, &kind) in &gtcol.regs {
                let br = self.chip.row_reg_bot(reg);
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
                    } else if !is_l && gtcol.col != self.chip.columns.last_id().unwrap() {
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
                    ColumnKind::Bram => {
                        self.die.add_tile((col, row), "INTF.BRAM", &[(col, row)]);
                    }
                    ColumnKind::Dsp
                    | ColumnKind::Cmt
                    | ColumnKind::Cfg
                    | ColumnKind::Clk
                    | ColumnKind::Io => {
                        self.die.add_tile((col, row), "INTF", &[(col, row)]);
                    }
                    ColumnKind::Gt => (),
                }
            }
        }
    }

    fn fill_cfg(&mut self) {
        let row_cm = self.chip.row_reg_bot(self.chip.reg_cfg);
        let row_cb: RowId = row_cm - 50;
        let row_ct: RowId = row_cm + 50;
        if self.chip.regs != 1 {
            for dx in 0..6 {
                let col = self.col_cfg - 6 + dx;
                if row_cb.to_idx() != 0 {
                    self.die.fill_conn_term((col, row_cb - 1), "TERM.N");
                }
                if row_ct.to_idx() != self.chip.regs * 50 {
                    self.die.fill_conn_term((col, row_ct), "TERM.S");
                }
            }
        }

        let crds: [_; 50] = core::array::from_fn(|dy| (self.col_cfg, row_cb + dy));
        self.die.add_tile((self.col_cfg, row_cb), "CFG", &crds);

        if self.chip.regs != 1 {
            let row_m = row_cm + 25;
            let crds: [_; 25] = core::array::from_fn(|dy| (self.col_cfg, row_m + dy));
            self.die.add_tile((self.col_cfg, row_m), "SYSMON", &crds);
        }
    }

    fn fill_ps(&mut self) {
        if self.chip.has_ps {
            let col_l = self.die.cols().next().unwrap();
            let row_t = self.die.rows().next_back().unwrap();
            let row_pb = row_t - 99;
            if self.chip.regs != 2 {
                for dx in 0..18 {
                    let col = col_l + dx;
                    self.die.fill_conn_term((col, row_pb - 1), "TERM.N");
                }
            }
            let col = col_l + 18;
            for dy in 0..100 {
                let row = row_pb + dy;
                self.die.fill_conn_term((col, row), "TERM.W");
                self.die.add_tile((col, row), "INTF", &[(col, row)]);
            }

            let crds: [_; 100] = core::array::from_fn(|dy| (col, row_pb + dy));
            self.die.add_tile((col, row_pb + 50), "PS", &crds);
        }
    }

    fn fill_pcie2(&mut self) {
        for pcie2 in &self.chip.holes_pcie2 {
            for dx in 1..3 {
                let col = pcie2.col + dx;
                if pcie2.row.to_idx() != 0 {
                    self.die.fill_conn_term((col, pcie2.row - 1), "TERM.N");
                }
                self.die.fill_conn_term((col, pcie2.row + 25), "TERM.S");
            }
            let col_l = pcie2.col;
            let col_r = pcie2.col + 3;
            for dy in 0..25 {
                let row = pcie2.row + dy;
                self.die
                    .add_tile((col_l, row), "INTF.DELAY", &[(col_l, row)]);
                self.die
                    .add_tile((col_r, row), "INTF.DELAY", &[(col_r, row)]);
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
            self.die.add_tile(crds[0], "PCIE", &crds);
        }
    }

    fn fill_pcie3(&mut self) {
        for &(bc, br) in &self.chip.holes_pcie3 {
            for dx in 1..5 {
                let col = bc + dx;
                self.die.fill_conn_term((col, br - 1), "TERM.N");
                self.die.fill_conn_term((col, br + 50), "TERM.S");
            }
            let col_l = bc;
            let col_r = bc + 5;
            for dy in 0..50 {
                let row = br + dy;
                self.die
                    .add_tile((col_l, row), "INTF.DELAY", &[(col_l, row)]);
                self.die
                    .add_tile((col_r, row), "INTF.DELAY", &[(col_r, row)]);
            }
            let mut crds = vec![];
            for dy in 0..50 {
                crds.push((bc, br + dy));
            }
            for dy in 0..50 {
                crds.push((bc + 5, br + dy));
            }
            self.die.add_tile(crds[0], "PCIE3", &crds);
        }
    }

    fn fill_gt(&mut self) {
        for gtcol in &self.chip.cols_gt {
            let is_l = gtcol.col < self.col_clk;
            for (reg, &kind) in &gtcol.regs {
                let br = self.chip.row_reg_bot(reg);
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
                                    self.die.fill_conn_term((col, br - 1), "TERM.N");
                                }
                                if br.to_idx() + 50 != self.chip.regs * 50 {
                                    self.die.fill_conn_term((col, br + 50), "TERM.S");
                                }
                            }
                            let col_l = gtcol.col;
                            let col_r = gtcol.col + 19;
                            for dy in 0..50 {
                                let row = br + dy;
                                self.die
                                    .add_tile((col_l, row), "INTF.DELAY", &[(col_l, row)]);
                                self.die.fill_conn_term((col_l, row), "TERM.E");
                                self.die.fill_conn_term((col_r, row), "TERM.W");
                            }
                        } else {
                            for dx in 1..19 {
                                let col = gtcol.col - 19 + dx;
                                if br.to_idx() != 0 {
                                    self.die.fill_conn_term((col, br - 1), "TERM.N");
                                }
                                if br.to_idx() + 50 != self.chip.regs * 50 {
                                    self.die.fill_conn_term((col, br + 50), "TERM.S");
                                }
                            }
                            let col_l = gtcol.col - 19;
                            let col_r = gtcol.col;
                            for dy in 0..50 {
                                let row = br + dy;
                                self.die
                                    .add_tile((col_r, row), "INTF.DELAY", &[(col_r, row)]);
                                self.die.fill_conn_term((col_l, row), "TERM.E");
                                self.die.fill_conn_term((col_r, row), "TERM.W");
                            }
                        }
                    } else if is_l {
                        for dy in 0..50 {
                            let row = br + dy;
                            self.die
                                .add_tile((gtcol.col, row), "INTF.DELAY", &[(gtcol.col, row)]);
                        }
                    } else {
                        if gtcol.col != self.chip.columns.last_id().unwrap() {
                            if reg.to_idx() != 0 && gtcol.regs[reg - 1].is_none() {
                                for dx in 1..7 {
                                    self.die.fill_conn_term((gtcol.col + dx, br - 1), "TERM.N");
                                }
                            }
                            if reg.to_idx() != self.chip.regs - 1 && gtcol.regs[reg + 1].is_none() {
                                for dx in 1..7 {
                                    self.die.fill_conn_term((gtcol.col + dx, br + 50), "TERM.S");
                                }
                            }
                            for dy in 0..50 {
                                self.die.fill_conn_term((gtcol.col, br + dy), "TERM.E");
                            }
                        }
                        for dy in 0..50 {
                            let row = br + dy;
                            self.die
                                .add_tile((gtcol.col, row), "INTF.DELAY", &[(gtcol.col, row)]);
                        }
                    }
                    let ksuf = if gtcol.is_middle { "_MID" } else { "" };
                    for dy in [0, 11, 28, 39] {
                        let row = br + dy;
                        let crds: [_; 11] = core::array::from_fn(|dy| (gtcol.col, row + dy));
                        self.die
                            .add_tile((gtcol.col, row), &format!("{sk}_CHANNEL{ksuf}"), &crds);
                    }
                    let row = br + 22;
                    let crds: [_; 6] = core::array::from_fn(|dy| (gtcol.col, row + dy));
                    self.die
                        .add_tile((gtcol.col, row + 3), &format!("{sk}_COMMON{ksuf}"), &crds);

                    self.gt.push((self.die.die, gtcol.col, row + 3));
                }
                if br.to_idx() != 0
                    && (kind.is_some() || gtcol.regs[reg - 1].is_some())
                    && !gtcol.is_middle
                {
                    self.die.add_tile((gtcol.col, br), "BRKH_GTX", &[]);
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
                if self.chip.has_no_tbuturn {
                    self.die.fill_conn_term((col, row_b), "TERM.S.HOLE");
                } else {
                    self.die.fill_conn_term((col, row_b), "TERM.S");
                }
            }
            if !self.is_int_hole(col, row_t) {
                if self.chip.has_no_tbuturn {
                    self.die.fill_conn_term((col, row_t), "TERM.N.HOLE");
                } else {
                    self.die.fill_conn_term((col, row_t), "TERM.N");
                }
            }
        }
        for row in self.die.rows() {
            if !self.is_int_hole(col_l, row) {
                self.die.fill_conn_term((col_l, row), "TERM.W");
            }
            if !self.is_int_hole(col_r, row) {
                self.die.fill_conn_term((col_r, row), "TERM.E");
            }
        }
        for reg in 1..self.chip.regs {
            let row_s = RowId::from_idx(reg * 50 - 1);
            let row_n = RowId::from_idx(reg * 50);
            for col in self.die.cols() {
                if !self.is_int_hole(col, row_s) && !self.is_int_hole(col, row_n) {
                    self.die
                        .fill_conn_pair((col, row_s), (col, row_n), "BRKH.N", "BRKH.S");
                }
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

    fn fill_bram_dsp(&mut self) {
        let col = self.chip.columns.first_id().unwrap();
        if self.chip.columns[col] == ColumnKind::Bram {
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
                if cd == ColumnKind::Bram && row.to_idx() % 50 == 25 {
                    let coords: Vec<_> = (0..15).map(|dy| (col, row + dy)).collect();
                    self.die.add_tile((col, row), "PMVBRAM", &coords);
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
                    self.die.add_tile((col, row), "PMVBRAM_NC", &[]);
                }
            }
        }
    }

    fn fill_io(&mut self) {
        for iocol in self.chip.cols_io.iter() {
            let col = iocol.col;
            for row in self.die.rows() {
                let reg = self.chip.row_to_reg(row);
                if let Some(kind) = iocol.regs[reg] {
                    if matches!(row.to_idx() % 50, 0 | 49) {
                        self.die.add_tile(
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
                        self.die.add_tile(
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
                        self.die.add_tile(
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
        for (col, &cd) in &self.chip.columns {
            if cd != ColumnKind::Cmt {
                continue;
            }
            for reg in self.chip.regs() {
                let row = self.chip.row_reg_hclk(reg);
                if self.is_site_hole(col, row) {
                    continue;
                }
                let crds: [_; 50] = core::array::from_fn(|dy| (col, row - 25 + dy));
                self.die.add_tile((col, row), "CMT", &crds);

                for row in [row - 24, row - 12, row, row + 12] {
                    let crds: [_; 12] = core::array::from_fn(|dy| (col, row + dy));
                    self.die.add_tile((col, row), "CMT_FIFO", &crds);
                }
            }
        }
    }

    fn fill_clk(&mut self) {
        let col = self.col_clk;
        for reg in self.chip.regs() {
            let row_h = self.chip.row_reg_hclk(reg);
            if self.chip.has_slr && reg.to_idx() == 0 {
                let crds: [_; 16] = core::array::from_fn(|dy| (col, row_h - 21 + dy));
                self.die.add_tile(crds[0], "CLK_BALI_REBUF", &crds);
            } else {
                let crds: [_; 2] = core::array::from_fn(|dy| (col, row_h - 13 + dy));
                self.die.add_tile(crds[0], "CLK_BUFG_REBUF", &crds);
            }

            self.die
                .add_tile((col, row_h), "CLK_HROW", &[(col, row_h - 1), (col, row_h)]);

            if self.chip.has_slr && reg.to_idx() == self.chip.regs - 1 {
                let crds: [_; 16] = core::array::from_fn(|dy| (col, row_h + 5 + dy));
                self.die.add_tile(crds[0], "CLK_BALI_REBUF", &crds);
            } else {
                let crds: [_; 2] = core::array::from_fn(|dy| (col, row_h + 11 + dy));
                self.die.add_tile(crds[0], "CLK_BUFG_REBUF", &crds);
            }
        }

        let row = self.chip.row_bufg() - 4;
        let crds: [_; 4] = core::array::from_fn(|dy| (col, row + dy));
        self.die.add_tile((col, row), "CLK_BUFG", &crds);
        if self.chip.reg_clk.to_idx() != self.chip.regs {
            let row = self.chip.row_bufg();
            let crds: [_; 4] = core::array::from_fn(|dy| (col, row + dy));
            self.die.add_tile((col, row), "CLK_BUFG", &crds);
        }

        let pmv_base = if self.chip.regs == 1 { 0 } else { 1 };
        for (kind, dy) in [
            ("CLK_PMV", pmv_base + 3),
            ("CLK_PMVIOB", 17),
            ("CLK_PMV2_SVT", 32),
            ("CLK_PMV2", 41),
            ("CLK_MTBF2", 45),
        ] {
            let row = self.chip.row_bufg() - 50 + dy;
            self.die.add_tile((col, row), kind, &[(col, row)]);
        }
    }

    fn fill_hclk(&mut self) {
        for col in self.die.cols() {
            let col_hrow = if col <= self.col_clk {
                self.col_clk
            } else {
                self.col_clk + 1
            };
            if col.to_idx() % 2 != 0 {
                continue;
            }
            for row in self.die.rows() {
                let row_hclk = self.chip.row_hclk(row);
                let crow = if row < row_hclk {
                    row_hclk - 1
                } else {
                    row_hclk
                };
                self.die[(col, row)].region_root[REGION_HCLK] = (col_hrow, row_hclk);
                self.die[(col + 1, row)].region_root[REGION_HCLK] = (col_hrow, row_hclk);
                self.die[(col, row)].region_root[REGION_LEAF] = (col, crow);
                self.die[(col + 1, row)].region_root[REGION_LEAF] = (col, crow);

                if row.to_idx() % 50 == 25 {
                    let hole_bot = self.is_int_hole(col, row - 1);
                    let hole_top = self.is_int_hole(col, row);
                    if hole_bot && hole_top {
                        continue;
                    }
                    self.die.add_tile((col, row), "HCLK", &[]);
                }

                if self.is_int_hole(col, row) {
                    continue;
                }
                self.die
                    .add_tile((col, row), "INT_LCLK", &[(col, row), (col + 1, row)]);
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
        }
        for &reg in &regs {
            for (col, &cd) in &self.chip.columns {
                self.frames.col_frame[reg].push(self.frame_info.len());
                if let Some(gtcol) = self.chip.get_col_gt(col) {
                    if gtcol.regs[reg].is_some()
                        && (gtcol.col == self.chip.columns.last_id().unwrap()
                            || gtcol.col == self.chip.columns.last_id().unwrap() - 6)
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
                                    region: if self.chip.regs == 1 {
                                        0
                                    } else {
                                        (reg - self.chip.reg_cfg) as i32
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
                    for gt in &self.chip.cols_gt {
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
                    if cd == ColumnKind::Cfg && matches!(minor, 28..30) && reg == self.chip.reg_cfg
                    {
                        mask_mode[1] = FrameMaskMode::DrpHclk(25, 13);
                    }
                    for hole in &self.chip.holes_pcie2 {
                        match hole.kind {
                            Pcie2Kind::Left => {
                                if self.chip.row_reg_bot(reg) == hole.row
                                    && col == hole.col + 3
                                    && matches!(minor, 28..30)
                                {
                                    mask_mode[0] = FrameMaskMode::PcieLeftDrpHclk(24, 13);
                                }
                            }
                            Pcie2Kind::Right => {
                                if self.chip.row_reg_bot(reg) == hole.row
                                    && col == hole.col
                                    && matches!(minor, 28..30)
                                {
                                    mask_mode[0] = FrameMaskMode::DrpHclk(24, 13);
                                }
                            }
                        }
                    }
                    for &(hcol, hrow) in &self.chip.holes_pcie3 {
                        if self.chip.row_reg_hclk(reg) == hrow + 50
                            && col == hcol + 4
                            && matches!(minor, 28..30)
                        {
                            mask_mode[0] = FrameMaskMode::DrpHclk(24, 13);
                        }
                        if self.chip.row_reg_hclk(reg) == hrow
                            && col == hcol + 4
                            && matches!(minor, 28..30)
                        {
                            mask_mode[1] = FrameMaskMode::DrpHclk(24, 13);
                        }
                    }
                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 0,
                            region: if self.chip.regs == 1 {
                                0
                            } else {
                                (reg - self.chip.reg_cfg) as i32
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
            for (col, &cd) in &self.chip.columns {
                if cd != ColumnKind::Bram {
                    continue;
                }
                if let Some(gtcol) = self.chip.get_col_gt(col) {
                    if gtcol.col != self.chip.columns.last_id().unwrap()
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
                            region: if self.chip.regs == 1 {
                                0
                            } else {
                                (reg - self.chip.reg_cfg) as i32
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

fn get_gtz_cols(chip: &Chip, num_l: usize, num_r: usize) -> EntityVec<GtzIntColId, ColId> {
    let mut res_l = vec![];
    let mut res_r = vec![];
    let col_clk = chip
        .columns
        .iter()
        .find(|&(_, &kind)| kind == ColumnKind::Clk)
        .unwrap()
        .0;
    let col_cfg = chip
        .columns
        .iter()
        .find(|&(_, &kind)| kind == ColumnKind::Cfg)
        .unwrap()
        .0;
    let mut col = col_clk;
    while res_l.len() < num_l {
        if matches!(chip.columns[col], ColumnKind::ClbLL | ColumnKind::ClbLM)
            && !(col >= col_cfg - 6 && col < col_cfg)
        {
            res_l.push(col);
        }
        col -= 1;
    }
    let mut col = col_clk;
    while res_r.len() < num_r {
        if matches!(chip.columns[col], ColumnKind::ClbLL | ColumnKind::ClbLM)
            && !(col >= col_cfg - 6 && col < col_cfg)
        {
            res_r.push(col);
        }
        col += 1;
    }
    res_l.into_iter().rev().chain(res_r).collect()
}

pub fn expand_grid<'a>(
    chips: &EntityVec<DieId, &'a Chip>,
    interposer: &'a Interposer,
    disabled: &BTreeSet<DisabledPart>,
    db: &'a IntDb,
    gdb: &'a GtzDb,
) -> ExpandedDevice<'a> {
    let mut egrid = ExpandedGrid::new(db);
    let pchip = &chips[interposer.primary];
    let mut bank = (15
        - chips
            .iter()
            .filter_map(|(die, chip)| {
                if die < interposer.primary {
                    Some(chip.regs)
                } else {
                    None
                }
            })
            .sum::<usize>()) as u32;
    let mut frames = EntityVec::new();
    let mut die_bs_geom = EntityVec::new();

    let col_cfg = pchip
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
    let col_clk = pchip
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
    let col_lio = pchip.columns.iter().find_map(|(col, &cd)| {
        if cd == ColumnKind::Io && col < col_cfg {
            Some(col)
        } else {
            None
        }
    });
    let col_rio = pchip.columns.iter().find_map(|(col, &cd)| {
        if cd == ColumnKind::Io && col > col_cfg {
            Some(col)
        } else {
            None
        }
    });
    let mut col_mgt = None;
    let mut col_lgt = None;
    let mut col_rgt = None;
    if pchip.cols_gt.len() == 2 && pchip.cols_gt[0].col.to_idx() != 0 {
        col_mgt = Some((pchip.cols_gt[0].col, pchip.cols_gt[1].col));
    } else {
        col_lgt = pchip.cols_gt.iter().find_map(|gtcol| {
            if gtcol.col < col_cfg {
                Some(gtcol.col)
            } else {
                None
            }
        });
        col_rgt = pchip.cols_gt.iter().find_map(|gtcol| {
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
    for &chip in chips.values() {
        let (_, die) = egrid.add_die(chip.columns.len(), chip.regs * 50);

        let mut de = DieExpander {
            chip,
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
        bank += chip.regs as u32;
    }

    let lvb6 = db.wires.get("LVB.6").unwrap().0;
    for (die, &chip) in chips {
        if chip.has_no_tbuturn {
            for col in chip.columns.ids() {
                for i in 0..6 {
                    let row = RowId::from_idx(i);
                    egrid.blackhole_wires.insert((die, (col, row), lvb6));
                }
                for i in 0..6 {
                    let row = RowId::from_idx(chip.regs * 50 - 6 + i);
                    egrid.blackhole_wires.insert((die, (col, row), lvb6));
                }
            }
        }
    }

    let mut xdie_wires = BiHashMap::new();
    for i in 1..chips.len() {
        let dieid_s = DieId::from_idx(i - 1);
        let dieid_n = DieId::from_idx(i);
        let die_s = egrid.die(dieid_s);
        let die_n = egrid.die(dieid_n);
        for col in die_s.cols() {
            for dy in 0..49 {
                let row_s = die_s.rows().next_back().unwrap() - 49 + dy;
                let row_n = die_n.rows().next().unwrap() + 1 + dy;
                if !die_s[(col, row_s)].tiles.is_empty() && !die_n[(col, row_n)].tiles.is_empty() {
                    xdie_wires.insert((dieid_n, (col, row_n), lvb6), (dieid_s, (col, row_s), lvb6));
                }
            }
        }
    }
    egrid.extra_conns = xdie_wires;

    let mut die_order = vec![];
    die_order.push(interposer.primary);
    for die in chips.ids() {
        if die != interposer.primary {
            die_order.push(die);
        }
    }

    let bs_geom = BitstreamGeom {
        kind: DeviceKind::Virtex7,
        die: die_bs_geom,
        die_order,
        has_gtz_bot: interposer.gtz_bot,
        has_gtz_top: interposer.gtz_top,
    };

    let mut cfg_io = BiHashMap::new();
    if pchip.has_ps {
        cfg_io.insert(
            SharedCfgPad::PudcB,
            IoCoord {
                die: interposer.primary,
                col: col_rio.unwrap(),
                row: pchip.row_reg_bot(pchip.reg_cfg) - 50 + 43,
                iob: TileIobId::from_idx(1),
            },
        );
    } else {
        cfg_io.extend(
            [
                (1, 0, SharedCfgPad::Data(16)),
                (1, 1, SharedCfgPad::Data(17)),
                (3, 0, SharedCfgPad::Data(18)),
                (3, 1, SharedCfgPad::Data(19)),
                (5, 0, SharedCfgPad::Data(20)),
                (5, 1, SharedCfgPad::Data(21)),
                (7, 0, SharedCfgPad::Data(22)),
                (9, 0, SharedCfgPad::Data(23)),
                (9, 1, SharedCfgPad::Data(24)),
                (11, 0, SharedCfgPad::Data(25)),
                (11, 1, SharedCfgPad::Data(26)),
                (13, 0, SharedCfgPad::Data(27)),
                (13, 1, SharedCfgPad::Data(28)),
                (15, 0, SharedCfgPad::Data(29)),
                (15, 1, SharedCfgPad::Data(30)),
                (17, 0, SharedCfgPad::Data(31)),
                (17, 1, SharedCfgPad::CsiB),
                (19, 0, SharedCfgPad::CsoB),
                (19, 1, SharedCfgPad::RdWrB),
                (29, 0, SharedCfgPad::Data(15)),
                (29, 1, SharedCfgPad::Data(14)),
                (31, 0, SharedCfgPad::Data(13)),
                (33, 0, SharedCfgPad::Data(12)),
                (33, 1, SharedCfgPad::Data(11)),
                (35, 0, SharedCfgPad::Data(10)),
                (35, 1, SharedCfgPad::Data(9)),
                (37, 0, SharedCfgPad::Data(8)),
                (37, 1, SharedCfgPad::FcsB),
                (39, 0, SharedCfgPad::Data(7)),
                (39, 1, SharedCfgPad::Data(6)),
                (41, 0, SharedCfgPad::Data(5)),
                (41, 1, SharedCfgPad::Data(4)),
                (43, 0, SharedCfgPad::EmCclk),
                (43, 1, SharedCfgPad::PudcB),
                (45, 0, SharedCfgPad::Data(3)),
                (45, 1, SharedCfgPad::Data(2)),
                (47, 0, SharedCfgPad::Data(1)),
                (47, 1, SharedCfgPad::Data(0)),
                (51, 0, SharedCfgPad::Rs(0)),
                (51, 1, SharedCfgPad::Rs(1)),
                (53, 0, SharedCfgPad::FweB),
                (53, 1, SharedCfgPad::FoeB),
                (55, 0, SharedCfgPad::Addr(16)),
                (55, 1, SharedCfgPad::Addr(17)),
                (57, 0, SharedCfgPad::Addr(18)),
                (59, 0, SharedCfgPad::Addr(19)),
                (59, 1, SharedCfgPad::Addr(20)),
                (61, 0, SharedCfgPad::Addr(21)),
                (61, 1, SharedCfgPad::Addr(22)),
                (63, 0, SharedCfgPad::Addr(23)),
                (63, 1, SharedCfgPad::Addr(24)),
                (65, 0, SharedCfgPad::Addr(25)),
                (65, 1, SharedCfgPad::Addr(26)),
                (67, 0, SharedCfgPad::Addr(27)),
                (67, 1, SharedCfgPad::Addr(28)),
                (69, 0, SharedCfgPad::AdvB),
            ]
            .into_iter()
            .map(|(dy, iob, pin)| {
                (
                    pin,
                    IoCoord {
                        die: interposer.primary,
                        col: col_lio.unwrap(),
                        row: pchip.row_reg_bot(pchip.reg_cfg) - 50 + dy,
                        iob: TileIobId::from_idx(iob),
                    },
                )
            }),
        );
    }

    let mut gtz = DirPartMap::new();
    if interposer.gtz_bot {
        let die = chips.first_id().unwrap();
        gtz.insert(
            Dir::S,
            ExpandedGtz {
                kind: gdb.gtz.get("GTZ_BOT").unwrap().0,
                bank: 400,
                die,
                cols: get_gtz_cols(chips[die], 46, 40),
                rows: (0..49).map(|i| RowId::from_idx(1 + i)).collect(),
            },
        );
    }
    if interposer.gtz_top {
        let die = chips.last_id().unwrap();
        let row_base = RowId::from_idx(chips[die].regs * 50 - 50);
        gtz.insert(
            Dir::N,
            ExpandedGtz {
                kind: gdb.gtz.get("GTZ_TOP").unwrap().0,
                bank: 300,
                die,
                cols: get_gtz_cols(chips[die], 40, 46),
                rows: (0..49).map(|i| row_base + i).collect(),
            },
        );
    }

    egrid.finish();
    ExpandedDevice {
        kind: pchip.kind,
        chips: chips.clone(),
        egrid,
        gdb,
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
