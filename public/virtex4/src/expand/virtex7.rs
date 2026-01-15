use bimap::BiHashMap;
use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};
use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::dir::{Dir, DirH, DirPartMap};
use prjcombine_interconnect::grid::{
    CellCoord, ColId, DieId, ExpandedGrid, Rect, RowId, TileIobId,
};
use prjcombine_xilinx_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo, FrameMaskMode,
};
use std::collections::BTreeSet;

use crate::bond::SharedCfgPad;
use crate::chip::{Chip, ColumnKind, DisabledPart, GtKind, Interposer, IoKind};
use crate::expanded::{DieFrameGeom, ExpandedDevice, ExpandedGtz, IoCoord};
use crate::gtz::{GtzDb, GtzIntColId};
use crate::{
    defs,
    defs::virtex7::{ccls, tcls, wires},
};

struct DieExpander<'a, 'b, 'c> {
    chip: &'b Chip,
    egrid: &'a mut ExpandedGrid<'b>,
    die: DieId,
    site_holes: &'a mut Vec<Rect>,
    int_holes: &'a mut Vec<Rect>,
    frame_info: Vec<FrameInfo>,
    frames: DieFrameGeom,
    col_cfg: ColId,
    col_clk: ColId,
    io: &'c mut Vec<IoCoord>,
    gt: &'c mut Vec<CellCoord>,
}

impl DieExpander<'_, '_, '_> {
    fn is_site_hole(&self, cell: CellCoord) -> bool {
        for hole in &*self.site_holes {
            if hole.contains(cell) {
                return true;
            }
        }
        false
    }

    fn is_int_hole(&self, cell: CellCoord) -> bool {
        for hole in &*self.int_holes {
            if hole.contains(cell) {
                return true;
            }
        }
        false
    }

    fn fill_holes(&mut self) {
        let cell = CellCoord::new(
            self.die,
            self.col_cfg - 6,
            self.chip.row_reg_bot(self.chip.reg_cfg - 1),
        );
        if self.chip.regs == 1 {
            self.int_holes.push(cell.rect(6, 50));
            self.site_holes.push(cell.rect(6, 50));
        } else {
            self.int_holes.push(cell.rect(6, 100));
            self.site_holes.push(cell.rect(6, 100));
        }
        if self.chip.has_ps {
            let col_l = self.egrid.cols(self.die).first().unwrap();
            let row_t = self.egrid.rows(self.die).last().unwrap();
            let cell = CellCoord::new(self.die, col_l, row_t - 99);
            self.int_holes.push(cell.rect(18, 100));
            self.site_holes.push(cell.rect(19, 100));
        }
        for pcie2 in &self.chip.holes_pcie2 {
            let cell = CellCoord::new(self.die, pcie2.col, pcie2.row);
            self.site_holes.push(cell.rect(4, 25));
            self.int_holes.push(cell.delta(1, 0).rect(2, 25));
        }
        for &(col, row) in &self.chip.holes_pcie3 {
            let cell = CellCoord::new(self.die, col, row);
            self.int_holes.push(cell.delta(1, 0).rect(4, 50));
            self.site_holes.push(cell.rect(6, 50));
        }
        for gtcol in &self.chip.cols_gt {
            let is_l = gtcol.col < self.col_clk;
            for (reg, &kind) in &gtcol.regs {
                let br = self.chip.row_reg_bot(reg);
                let cell = CellCoord::new(self.die, gtcol.col, br);
                if kind.is_some() {
                    if gtcol.is_middle {
                        if is_l {
                            self.int_holes.push(cell.delta(1, 0).rect(18, 50));
                            self.site_holes.push(cell.rect(19, 50));
                        } else {
                            self.int_holes.push(cell.delta(-18, 0).rect(18, 50));
                            self.site_holes.push(cell.delta(-18, 0).rect(19, 50));
                        }
                    } else if !is_l && gtcol.col != self.chip.columns.last_id().unwrap() {
                        self.site_holes.push(cell.rect(7, 50));
                        self.int_holes.push(cell.delta(1, 0).rect(6, 50));
                    }
                }
            }
        }
    }

    fn fill_int(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            if self.is_int_hole(cell) {
                continue;
            }
            self.egrid.add_tile_single_id(cell, tcls::INT);
            if self.is_site_hole(cell) {
                continue;
            }
            match self.chip.columns[cell.col] {
                ColumnKind::ClbLL => (),
                ColumnKind::ClbLM => (),
                ColumnKind::Bram => {
                    self.egrid.add_tile_single_id(cell, tcls::INTF_BRAM);
                }
                ColumnKind::Dsp
                | ColumnKind::Cmt
                | ColumnKind::Cfg
                | ColumnKind::Clk
                | ColumnKind::Io => {
                    self.egrid.add_tile_single_id(cell, tcls::INTF);
                }
                ColumnKind::Gt => (),
            }
        }
    }

    fn fill_cfg(&mut self) {
        let cell = CellCoord::new(
            self.die,
            self.col_cfg,
            self.chip.row_reg_bot(self.chip.reg_cfg - 1),
        );
        if self.chip.regs != 1 {
            for dx in 0..6 {
                if cell.row.to_idx() != 0 {
                    self.egrid
                        .fill_conn_term_id(cell.delta(-6 + dx, -1), ccls::TERM_N);
                }
                if cell.row.to_idx() != self.chip.regs * 50 - 100 {
                    self.egrid
                        .fill_conn_term_id(cell.delta(-6 + dx, 100), ccls::TERM_S);
                }
            }
        }

        self.egrid.add_tile_n_id(cell, tcls::CFG, 50);

        if self.chip.regs != 1 {
            self.egrid
                .add_tile_n_id(cell.delta(0, 75), tcls::SYSMON, 25);
        }
    }

    fn fill_ps(&mut self) {
        if self.chip.has_ps {
            let col_l = self.egrid.cols(self.die).first().unwrap();
            let row_t = self.egrid.rows(self.die).last().unwrap();
            let cell = CellCoord::new(self.die, col_l + 18, row_t - 99);
            if self.chip.regs != 2 {
                for dx in 0..18 {
                    self.egrid
                        .fill_conn_term_id(cell.delta(-18 + dx, -1), ccls::TERM_N);
                }
            }
            let tcells = cell.cells_n_const::<100>();
            for cell in tcells {
                self.egrid.fill_conn_term_id(cell, ccls::TERM_W);
                self.egrid.add_tile_single_id(cell, tcls::INTF);
            }

            self.egrid.add_tile_id(cell.delta(0, 50), tcls::PS, &tcells);
        }
    }

    fn fill_pcie2(&mut self) {
        for pcie2 in &self.chip.holes_pcie2 {
            let cell = CellCoord::new(self.die, pcie2.col, pcie2.row);
            for dx in 1..3 {
                if cell.row.to_idx() != 0 {
                    self.egrid
                        .fill_conn_term_id(cell.delta(dx, -1), ccls::TERM_N);
                }
                self.egrid
                    .fill_conn_term_id(cell.delta(dx, 25), ccls::TERM_S);
            }
            let mut tcells = vec![];
            match pcie2.side {
                DirH::W => {
                    tcells.extend(cell.delta(3, 0).cells_n_const::<25>());
                    tcells.extend(cell.cells_n_const::<25>());
                }
                DirH::E => {
                    tcells.extend(cell.cells_n_const::<25>());
                    tcells.extend(cell.delta(3, 0).cells_n_const::<25>());
                }
            }
            for &cell in &tcells {
                self.egrid.add_tile_single_id(cell, tcls::INTF_DELAY);
            }
            self.egrid.add_tile_id(tcells[0], tcls::PCIE, &tcells);
        }
    }

    fn fill_pcie3(&mut self) {
        for &(bc, br) in &self.chip.holes_pcie3 {
            let cell = CellCoord::new(self.die, bc, br);
            for dx in 1..5 {
                self.egrid
                    .fill_conn_term_id(cell.delta(dx, -1), ccls::TERM_N);
                self.egrid
                    .fill_conn_term_id(cell.delta(dx, 50), ccls::TERM_S);
            }
            let mut tcells = vec![];
            tcells.extend(cell.cells_n_const::<50>());
            tcells.extend(cell.delta(5, 0).cells_n_const::<50>());
            for &cell in &tcells {
                self.egrid.add_tile_single_id(cell, tcls::INTF_DELAY);
            }
            self.egrid.add_tile_id(tcells[0], tcls::PCIE3, &tcells);
        }
    }

    fn fill_gt(&mut self) {
        for gtcol in &self.chip.cols_gt {
            let is_l = gtcol.col < self.col_clk;
            for (reg, &kind) in &gtcol.regs {
                let br = self.chip.row_reg_bot(reg);
                let cell = CellCoord::new(self.die, gtcol.col, self.chip.row_reg_bot(reg));
                if let Some(kind) = kind {
                    if gtcol.is_middle {
                        assert_eq!(kind, GtKind::Gtp);
                        if is_l {
                            for dx in 1..19 {
                                if cell.row.to_idx() != 0 {
                                    self.egrid
                                        .fill_conn_term_id(cell.delta(dx, -1), ccls::TERM_N);
                                }
                                if cell.row.to_idx() + 50 != self.chip.regs * 50 {
                                    self.egrid
                                        .fill_conn_term_id(cell.delta(dx, 50), ccls::TERM_S);
                                }
                            }
                            for dy in 0..50 {
                                self.egrid
                                    .fill_conn_term_id(cell.delta(0, dy), ccls::TERM_E);
                                self.egrid
                                    .fill_conn_term_id(cell.delta(19, dy), ccls::TERM_W);
                            }
                        } else {
                            for dx in 1..19 {
                                if cell.row.to_idx() != 0 {
                                    self.egrid
                                        .fill_conn_term_id(cell.delta(-19 + dx, -1), ccls::TERM_N);
                                }
                                if cell.row.to_idx() + 50 != self.chip.regs * 50 {
                                    self.egrid
                                        .fill_conn_term_id(cell.delta(-19 + dx, 50), ccls::TERM_S);
                                }
                            }
                            for dy in 0..50 {
                                self.egrid
                                    .fill_conn_term_id(cell.delta(-19, dy), ccls::TERM_E);
                                self.egrid
                                    .fill_conn_term_id(cell.delta(0, dy), ccls::TERM_W);
                            }
                        }
                    } else if !is_l && gtcol.col != self.chip.columns.last_id().unwrap() {
                        if reg.to_idx() != 0 && gtcol.regs[reg - 1].is_none() {
                            for dx in 1..7 {
                                self.egrid
                                    .fill_conn_term_id(cell.delta(dx, -1), ccls::TERM_N);
                            }
                        }
                        if reg.to_idx() != self.chip.regs - 1 && gtcol.regs[reg + 1].is_none() {
                            for dx in 1..7 {
                                self.egrid
                                    .fill_conn_term_id(cell.delta(dx, 50), ccls::TERM_S);
                            }
                        }
                        for dy in 0..50 {
                            self.egrid
                                .fill_conn_term_id(cell.delta(0, dy), ccls::TERM_E);
                        }
                    }
                    for dy in 0..50 {
                        self.egrid
                            .add_tile_single_id(cell.delta(0, dy), tcls::INTF_DELAY);
                    }
                    for dy in [0, 11, 28, 39] {
                        self.egrid.add_tile_n_id(
                            cell.delta(0, dy),
                            match kind {
                                GtKind::Gtp => {
                                    if gtcol.is_middle {
                                        tcls::GTP_CHANNEL_MID
                                    } else {
                                        tcls::GTP_CHANNEL
                                    }
                                }
                                GtKind::Gtx => tcls::GTX_CHANNEL,
                                GtKind::Gth => tcls::GTH_CHANNEL,
                            },
                            11,
                        );
                    }
                    self.egrid.add_tile_sn_id(
                        cell.delta(0, 25),
                        match kind {
                            GtKind::Gtp => {
                                if gtcol.is_middle {
                                    tcls::GTP_COMMON_MID
                                } else {
                                    tcls::GTP_COMMON
                                }
                            }
                            GtKind::Gtx => tcls::GTX_COMMON,
                            GtKind::Gth => tcls::GTH_COMMON,
                        },
                        3,
                        6,
                    );

                    self.gt.push(cell.delta(0, 25));
                }
                if br.to_idx() != 0
                    && (kind.is_some() || gtcol.regs[reg - 1].is_some())
                    && !gtcol.is_middle
                {
                    self.egrid.add_tile_id(cell, tcls::BRKH_GTX, &[]);
                }
            }
        }
    }

    fn fill_terms(&mut self) {
        for cell in self
            .egrid
            .row(self.die, self.egrid.rows(self.die).first().unwrap())
        {
            if !self.is_int_hole(cell) {
                if self.chip.has_no_tbuturn {
                    self.egrid.fill_conn_term_id(cell, ccls::TERM_S_HOLE);
                } else {
                    self.egrid.fill_conn_term_id(cell, ccls::TERM_S);
                }
            }
        }
        for cell in self
            .egrid
            .row(self.die, self.egrid.rows(self.die).last().unwrap())
        {
            if !self.is_int_hole(cell) {
                if self.chip.has_no_tbuturn {
                    self.egrid.fill_conn_term_id(cell, ccls::TERM_N_HOLE);
                } else {
                    self.egrid.fill_conn_term_id(cell, ccls::TERM_N);
                }
            }
        }
        for cell in self
            .egrid
            .column(self.die, self.egrid.cols(self.die).first().unwrap())
        {
            if !self.is_int_hole(cell) {
                self.egrid.fill_conn_term_id(cell, ccls::TERM_W);
            }
        }
        for cell in self
            .egrid
            .column(self.die, self.egrid.cols(self.die).last().unwrap())
        {
            if !self.is_int_hole(cell) {
                self.egrid.fill_conn_term_id(cell, ccls::TERM_E);
            }
        }
        for reg in 1..self.chip.regs {
            for cell_n in self.egrid.row(self.die, RowId::from_idx(reg * 50)) {
                let cell_s = cell_n.delta(0, -1);
                if !self.is_int_hole(cell_s) && !self.is_int_hole(cell_n) {
                    self.egrid
                        .fill_conn_pair_id(cell_s, cell_n, ccls::BRKH_N, ccls::BRKH_S);
                }
            }
        }
    }

    fn fill_main_passes(&mut self) {
        // horizontal
        for row in self.egrid.rows(self.die) {
            let mut prev = None;
            for cell in self.egrid.row(self.die, row) {
                if !self.egrid[cell].tiles.contains_id(defs::tslots::INT) {
                    continue;
                }
                if let Some(prev) = prev
                    && !self.egrid[cell].conns.contains_id(defs::cslots::W)
                {
                    self.egrid
                        .fill_conn_pair_id(prev, cell, ccls::PASS_E, ccls::PASS_W);
                }
                if !self.egrid[cell].conns.contains_id(defs::cslots::E) {
                    prev = Some(cell);
                } else {
                    prev = None;
                }
            }
        }
        // vertical
        for col in self.egrid.cols(self.die) {
            let mut prev = None;
            for cell in self.egrid.column(self.die, col) {
                if !self.egrid[cell].tiles.contains_id(defs::tslots::INT) {
                    continue;
                }
                if let Some(prev) = prev
                    && !self.egrid[cell].conns.contains_id(defs::cslots::S)
                {
                    self.egrid
                        .fill_conn_pair_id(prev, cell, ccls::PASS_N, ccls::PASS_S);
                }
                if !self.egrid[cell].conns.contains_id(defs::cslots::N) {
                    prev = Some(cell);
                } else {
                    prev = None;
                }
            }
        }
    }

    fn fill_clb(&mut self) {
        for (col, &cd) in &self.chip.columns {
            let tcid = match cd {
                ColumnKind::ClbLL => tcls::CLBLL,
                ColumnKind::ClbLM => tcls::CLBLM,
                _ => continue,
            };
            for cell in self.egrid.column(self.die, col) {
                if self.is_site_hole(cell) {
                    continue;
                }
                self.egrid.add_tile_single_id(cell, tcid);
            }
        }
    }

    fn fill_bram_dsp(&mut self) {
        let col = self.chip.columns.first_id().unwrap();
        if self.chip.columns[col] == ColumnKind::Bram {
            let cell_s = CellCoord::new(self.die, col, self.chip.rows().first().unwrap());
            let cell_n = CellCoord::new(self.die, col, self.chip.rows().last().unwrap() - 4);
            self.site_holes
                .extend([cell_s.rect(1, 5), cell_n.rect(1, 5)]);
        }
        for (col, &cd) in &self.chip.columns {
            let tcid = match cd {
                ColumnKind::Bram => tcls::BRAM,
                ColumnKind::Dsp => tcls::DSP,
                _ => continue,
            };
            for cell in self.egrid.column(self.die, col) {
                if !cell.row.to_idx().is_multiple_of(5) {
                    continue;
                }
                if self.is_site_hole(cell) {
                    continue;
                }
                self.egrid.add_tile_n_id(cell, tcid, 5);
                if cd == ColumnKind::Bram && cell.row.to_idx() % 50 == 25 {
                    self.egrid.add_tile_n_id(cell, tcls::PMVBRAM, 15);
                }
            }
            if cd == ColumnKind::Bram {
                for cell in self.egrid.column(self.die, col) {
                    if cell.row.to_idx() % 50 != 25 {
                        continue;
                    }
                    if self.is_site_hole(cell.delta(0, -1)) {
                        continue;
                    }
                    if !self.is_site_hole(cell) {
                        continue;
                    }
                    self.egrid.add_tile_id(cell, tcls::PMVBRAM_NC, &[]);
                }
            }
        }
    }

    fn fill_io(&mut self) {
        for iocol in self.chip.cols_io.iter() {
            for cell in self.egrid.column(self.die, iocol.col) {
                let reg = self.chip.row_to_reg(cell.row);
                if let Some(kind) = iocol.regs[reg] {
                    if matches!(cell.row.to_idx() % 50, 0 | 49) {
                        self.egrid.add_tile_single_id(
                            cell,
                            if cell.row.to_idx().is_multiple_of(50) {
                                if kind == IoKind::Hpio {
                                    tcls::IO_HP_S
                                } else {
                                    tcls::IO_HR_S
                                }
                            } else {
                                if kind == IoKind::Hpio {
                                    tcls::IO_HP_N
                                } else {
                                    tcls::IO_HR_N
                                }
                            },
                        );
                        self.io.push(IoCoord {
                            cell,
                            iob: TileIobId::from_idx(0),
                        });
                    } else if cell.row.to_idx() % 2 == 1 {
                        self.egrid.add_tile_n_id(
                            cell,
                            if kind == IoKind::Hpio {
                                tcls::IO_HP_PAIR
                            } else {
                                tcls::IO_HR_PAIR
                            },
                            2,
                        );
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

                    if cell.row.to_idx() % 50 == 25 {
                        self.egrid.add_tile_sn_id(
                            cell,
                            match kind {
                                IoKind::Hpio => tcls::HCLK_IO_HP,
                                IoKind::Hrio => tcls::HCLK_IO_HR,
                            },
                            4,
                            8,
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
                let cell = CellCoord::new(self.die, col, row);
                if self.is_site_hole(cell) {
                    continue;
                }
                self.egrid.add_tile_sn_id(cell, tcls::CMT, 25, 50);

                for dy in [-24, -12, 0, 12] {
                    self.egrid
                        .add_tile_n_id(cell.delta(0, dy), tcls::CMT_FIFO, 12);
                }
            }
        }
    }

    fn fill_clk(&mut self) {
        for reg in self.chip.regs() {
            let cell = CellCoord::new(self.die, self.col_clk, self.chip.row_reg_hclk(reg));
            if self.chip.has_slr && reg.to_idx() == 0 {
                self.egrid
                    .add_tile_n_id(cell.delta(0, -21), tcls::CLK_BALI_REBUF, 16);
            } else {
                self.egrid
                    .add_tile_n_id(cell.delta(0, -13), tcls::CLK_BUFG_REBUF, 2);
            }

            self.egrid.add_tile_sn_id(cell, tcls::CLK_HROW, 1, 2);

            if self.chip.has_slr && reg.to_idx() == self.chip.regs - 1 {
                self.egrid
                    .add_tile_n_id(cell.delta(0, 5), tcls::CLK_BALI_REBUF, 16);
            } else {
                self.egrid
                    .add_tile_n_id(cell.delta(0, 11), tcls::CLK_BUFG_REBUF, 2);
            }
        }

        let cell_bufg = CellCoord::new(self.die, self.col_clk, self.chip.row_bufg());
        self.egrid
            .add_tile_n_id(cell_bufg.delta(0, -4), tcls::CLK_BUFG, 4);
        if self.chip.reg_clk.to_idx() != self.chip.regs {
            self.egrid.add_tile_n_id(cell_bufg, tcls::CLK_BUFG, 4);
        }

        let pmv_base = if self.chip.regs == 1 { 0 } else { 1 };
        for (tcid, dy) in [
            (tcls::CLK_PMV, pmv_base + 3),
            (tcls::CLK_PMVIOB, 17),
            (tcls::CLK_PMV2_SVT, 32),
            (tcls::CLK_PMV2, 41),
            (tcls::CLK_MTBF2, 45),
        ] {
            self.egrid
                .add_tile_single_id(cell_bufg.delta(0, -50 + dy), tcid);
        }
    }

    fn fill_hclk(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            let col_hrow = if cell.col <= self.col_clk {
                self.col_clk
            } else {
                self.col_clk + 1
            };
            if !cell.col.to_idx().is_multiple_of(2) {
                continue;
            }
            let row_hclk = self.chip.row_hclk(cell.row);
            let crow = if cell.row < row_hclk {
                row_hclk - 1
            } else {
                row_hclk
            };
            let cell_w = cell;
            let cell_e = cell.delta(1, 0);
            let cell_hclk = cell.with_cr(col_hrow, row_hclk);
            let cell_leaf = cell.with_row(crow);
            self.egrid[cell_w].region_root[defs::rslots::HCLK] = cell_hclk;
            self.egrid[cell_e].region_root[defs::rslots::HCLK] = cell_hclk;
            self.egrid[cell_w].region_root[defs::rslots::LEAF] = cell_leaf;
            self.egrid[cell_e].region_root[defs::rslots::LEAF] = cell_leaf;

            if cell.row.to_idx() % 50 == 25 {
                let hole_bot = self.is_int_hole(cell.delta(0, -1));
                let hole_top = self.is_int_hole(cell);
                if hole_bot && hole_top {
                    continue;
                }
                self.egrid.add_tile_id(cell_w, tcls::HCLK, &[]);
            }

            if self.is_int_hole(cell_w) {
                continue;
            }
            self.egrid
                .add_tile_id(cell_w, tcls::INT_LCLK, &[cell_w, cell_e]);
        }
    }

    fn fill_frame_info(&mut self) {
        let mut regs = Vec::from_iter(self.chip.regs());
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
                if let Some(gtcol) = self.chip.get_col_gt(col)
                    && gtcol.regs[reg].is_some()
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
                        match hole.side {
                            DirH::W => {
                                if self.chip.row_reg_bot(reg) == hole.row
                                    && col == hole.col + 3
                                    && matches!(minor, 28..30)
                                {
                                    mask_mode[0] = FrameMaskMode::PcieLeftDrpHclk(24, 13);
                                }
                            }
                            DirH::E => {
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
                if let Some(gtcol) = self.chip.get_col_gt(col)
                    && gtcol.col != self.chip.columns.last_id().unwrap()
                    && gtcol.regs[reg].is_some()
                {
                    break;
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

    let mut int_holes = vec![];
    let mut site_holes = vec![];
    let mut banklut = EntityVec::new();
    for &chip in chips.values() {
        let die = egrid.add_die(chip.columns.len(), chip.regs * 50);

        let mut de = DieExpander {
            chip,
            die,
            egrid: &mut egrid,
            site_holes: &mut site_holes,
            int_holes: &mut int_holes,
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
        de.fill_main_passes();
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
        banklut.push(bank);
        bank += chip.regs as u32;
    }

    let lvb6 = wires::LVB[6];
    for (die, &chip) in chips {
        if chip.has_no_tbuturn {
            for col in chip.columns.ids() {
                for i in 0..6 {
                    let row = RowId::from_idx(i);
                    egrid
                        .blackhole_wires
                        .insert(CellCoord::new(die, col, row).wire(lvb6));
                }
                for i in 0..6 {
                    let row = RowId::from_idx(chip.regs * 50 - 6 + i);
                    egrid
                        .blackhole_wires
                        .insert(CellCoord::new(die, col, row).wire(lvb6));
                }
            }
        }
    }

    let mut xdie_wires = BiHashMap::new();
    for i in 1..chips.len() {
        let die_s = DieId::from_idx(i - 1);
        let die_n = DieId::from_idx(i);
        for col in egrid.cols(die_s) {
            for dy in 0..49 {
                let row_s = egrid.rows(die_s).last().unwrap() - 49 + dy;
                let row_n = egrid.rows(die_n).first().unwrap() + 1 + dy;
                let cell_s = CellCoord::new(die_s, col, row_s);
                let cell_n = CellCoord::new(die_n, col, row_n);
                if egrid[cell_s].tiles.contains_id(defs::tslots::INT)
                    && egrid[cell_n].tiles.contains_id(defs::tslots::INT)
                {
                    xdie_wires.insert(cell_n.wire(lvb6), cell_s.wire(lvb6));
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
                cell: CellCoord {
                    die: interposer.primary,
                    col: col_rio.unwrap(),
                    row: pchip.row_reg_bot(pchip.reg_cfg) - 50 + 43,
                },
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
                        cell: CellCoord {
                            die: interposer.primary,
                            col: col_lio.unwrap(),
                            row: pchip.row_reg_bot(pchip.reg_cfg) - 50 + dy,
                        },
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
