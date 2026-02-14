use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};
use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::dir::{Dir, DirH, DirHV, DirMap};
use prjcombine_interconnect::grid::builder::GridBuilder;
use prjcombine_interconnect::grid::{CellCoord, ColId, DieId, Rect};
use prjcombine_xilinx_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};
use std::collections::{BTreeSet, HashMap};

use crate::chip::{Chip, ColumnIoKind, ColumnKind, DcmKind, DisabledPart, PllKind, RegId};
use crate::defs::{self, rslots};
use crate::expanded::ExpandedDevice;

struct Expander<'a, 'b> {
    chip: &'b Chip,
    disabled: &'a BTreeSet<DisabledPart>,
    egrid: &'a mut GridBuilder<'b>,
    die: DieId,
    int_holes: Vec<Rect>,
    site_holes: Vec<Rect>,
    frame_info: Vec<FrameInfo>,
    bram_frame_info: Vec<FrameInfo>,
    iob_frame_len: usize,
    col_frame: EntityVec<RegId, EntityVec<ColId, usize>>,
    col_width: EntityVec<ColId, usize>,
    spine_frame: EntityVec<RegId, usize>,
    bram_frame: EntityVec<RegId, EntityPartVec<ColId, usize>>,
    iob_frame: HashMap<CellCoord, usize>,
    reg_frame: DirMap<usize>,
}

impl Expander<'_, '_> {
    fn is_site_hole(&self, cell: CellCoord) -> bool {
        for hole in &self.site_holes {
            if hole.contains(cell) {
                return true;
            }
        }
        false
    }

    fn is_int_hole(&self, cell: CellCoord) -> bool {
        for hole in &self.int_holes {
            if hole.contains(cell) {
                return true;
            }
        }
        false
    }

    fn fill_holes(&mut self) {
        if let Some(bcrd) = self.chip.bel_pcie() {
            self.int_holes.push(bcrd.cell.delta(1, 0).rect(3, 16));
            self.site_holes.push(bcrd.cell.rect(5, 16));
        }
        if let Some(bcrd) = self.chip.bel_gtp(DirHV::NW) {
            self.int_holes.push(bcrd.cell.delta(-6, -7).rect(11, 8));
            self.site_holes.push(bcrd.cell.delta(-6, -7).rect(11, 8));
            self.int_holes.push(bcrd.cell.delta(-4, -15).rect(7, 8));
            self.site_holes.push(bcrd.cell.delta(-5, -15).rect(9, 8));
        }
        if let Some(bcrd) = self.chip.bel_gtp(DirHV::NE) {
            self.int_holes.push(bcrd.cell.delta(-4, -7).rect(11, 8));
            self.site_holes.push(bcrd.cell.delta(-4, -7).rect(11, 8));
            self.int_holes.push(bcrd.cell.delta(-2, -15).rect(8, 8));
            self.site_holes.push(bcrd.cell.delta(-3, -15).rect(10, 8));
        }
        if let Some(bcrd) = self.chip.bel_gtp(DirHV::SW) {
            self.int_holes.push(bcrd.cell.delta(-6, 0).rect(11, 8));
            self.site_holes.push(bcrd.cell.delta(-6, 0).rect(11, 8));
            self.int_holes.push(bcrd.cell.delta(-4, 8).rect(7, 8));
            self.site_holes.push(bcrd.cell.delta(-5, 8).rect(9, 8));
        }
        if let Some(bcrd) = self.chip.bel_gtp(DirHV::SE) {
            self.int_holes.push(bcrd.cell.delta(-4, 0).rect(11, 8));
            self.site_holes.push(bcrd.cell.delta(-4, 0).rect(11, 8));
            self.int_holes.push(bcrd.cell.delta(-2, 8).rect(8, 8));
            self.site_holes.push(bcrd.cell.delta(-3, 8).rect(10, 8));
        }
    }

    fn fill_int(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            if self.is_int_hole(cell) {
                continue;
            }
            self.egrid.add_tile_single_id(cell, defs::tcls::INT);
            if self.is_site_hole(cell) {
                continue;
            }
            if matches!(
                self.chip.columns[cell.col].kind,
                ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::DspPlus
            ) {
                self.egrid.add_tile_single_id(cell, defs::tcls::INTF);
            }
        }
    }

    fn fill_io_w(&mut self) {
        for cell in self.egrid.column(self.die, self.chip.col_w()) {
            if self.chip.rows[cell.row].io_w {
                self.fill_ioi(cell);
                self.egrid.add_tile_single_id(cell, defs::tcls::IOB);
            } else {
                self.egrid.add_tile_single_id(cell, defs::tcls::INTF);
                if cell.row == self.chip.row_s() {
                    self.egrid.add_tile_single_id(cell, defs::tcls::CNR_SW);
                } else if cell.row == self.chip.row_n() {
                    self.egrid.add_tile_single_id(cell, defs::tcls::CNR_NW);
                }
            }

            self.egrid.fill_conn_term_id(cell, defs::ccls::TERM_W);

            if cell.row.to_idx() % 16 == 8 {
                self.egrid.add_tile_id(cell, defs::tcls::HCLK_IOI, &[]);
            }
        }
    }

    fn fill_io_e(&mut self) {
        for cell in self.egrid.column(self.die, self.chip.col_e()).rev() {
            if self.chip.rows[cell.row].io_e {
                self.fill_ioi(cell);
                self.egrid.add_tile_single_id(cell, defs::tcls::IOB);
            } else {
                self.egrid.add_tile_single_id(cell, defs::tcls::INTF);
                if cell.row == self.chip.row_s() {
                    self.egrid.add_tile_n_id(cell, defs::tcls::CNR_SE, 2);
                } else if cell.row == self.chip.row_n_inner() {
                    self.egrid.add_tile_n_id(cell, defs::tcls::CNR_NE, 2);
                }
            }

            self.egrid.fill_conn_term_id(cell, defs::ccls::TERM_E);

            if cell.row.to_idx() % 16 == 8 {
                self.egrid.add_tile_id(cell, defs::tcls::HCLK_IOI, &[]);
            }
        }
    }

    fn fill_io_n(&mut self) {
        for cell_o in self.egrid.row(self.die, self.chip.row_n()) {
            let cd = &self.chip.columns[cell_o.col];
            if cd.io_n == ColumnIoKind::None {
                continue;
            }
            self.site_holes.push(cell_o.delta(0, -1).rect(1, 2));
            for (cell, unused) in [
                (cell_o, cd.io_n == ColumnIoKind::Inner),
                (cell_o.delta(0, -1), cd.io_n == ColumnIoKind::Outer),
            ] {
                self.fill_ioi(cell);
                if !unused {
                    self.egrid.add_tile_single_id(cell, defs::tcls::IOB);
                }
            }
        }
    }

    fn fill_io_s(&mut self) {
        for cell_o in self.egrid.row(self.die, self.chip.row_s()) {
            let cd = &self.chip.columns[cell_o.col];
            if cd.io_s == ColumnIoKind::None {
                continue;
            }
            self.site_holes.push(cell_o.rect(1, 2));
            for (cell, unused) in [
                (cell_o, cd.io_s == ColumnIoKind::Inner),
                (cell_o.delta(0, 1), cd.io_s == ColumnIoKind::Outer),
            ] {
                self.fill_ioi(cell);
                if !unused {
                    self.egrid.add_tile_single_id(cell, defs::tcls::IOB);
                }
            }
        }
    }

    fn fill_mcb(&mut self) {
        if self.disabled.contains(&DisabledPart::Mcb) {
            return;
        }
        for (col, &cd) in &self.chip.columns {
            if cd.kind != ColumnKind::Io {
                continue;
            }
            for mcb in &self.chip.mcbs {
                let row = mcb.row_mcb;
                let cell = CellCoord::new(self.die, col, row);
                let mut tcells = cell.cells_n(12);
                for urow in mcb.row_mui {
                    tcells.extend(cell.with_row(urow).cells_n_const::<2>());
                }
                self.egrid.add_tile_id(cell, defs::tcls::MCB, &tcells);
                self.egrid[cell].region_root[rslots::PLLCLK] = cell.with_row(self.chip.row_clk());
            }
        }
    }

    fn fill_pcilogic(&mut self) {
        for edge in [DirH::W, DirH::E] {
            self.egrid
                .add_tile_single_id(self.chip.bel_pcilogicse(edge).cell, defs::tcls::PCILOGICSE);
        }
    }

    fn fill_spine(&mut self) {
        let cell_clkc = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_clk());
        {
            let cell = cell_clkc;
            self.site_holes.push(cell.rect(1, 1));
            self.egrid.add_tile_single_id(cell, defs::tcls::INTF);
            self.egrid.add_tile_id(
                cell,
                defs::tcls::CLKC,
                &[
                    cell.delta(0, -1),
                    cell,
                    cell.with_col(self.chip.col_w()),
                    cell.with_col(self.chip.col_e()),
                    cell.with_row(self.chip.row_s()),
                    cell.with_row(self.chip.row_n()),
                ],
            );
        }

        {
            let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_s());
            self.egrid.add_tile_id(
                cell,
                defs::tcls::CLK_S,
                &[cell.delta(0, 1), cell, cell.delta(1, 1), cell.delta(1, 0)],
            );
        }

        {
            let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_n());
            self.egrid.add_tile_id(
                cell,
                defs::tcls::CLK_N,
                &[cell, cell.delta(0, -1), cell.delta(1, 0), cell.delta(1, -1)],
            );
        }

        for cell in self.egrid.column(self.die, self.chip.col_clk) {
            if cell.row.to_idx() % 16 == 8 {
                self.egrid.add_tile_e_id(cell, defs::tcls::HCLK_ROW, 2);
            }
        }

        {
            let cell = CellCoord::new(self.die, self.chip.col_w(), self.chip.row_clk());
            self.egrid.add_tile_sn_id(cell, defs::tcls::CLK_W, 2, 6);
        }

        {
            let cell = CellCoord::new(self.die, self.chip.col_e(), self.chip.row_clk());
            self.egrid.add_tile_sn_id(cell, defs::tcls::CLK_E, 2, 6);
        }

        let mut prev = self.chip.bel_bufpll(Dir::S).cell;
        for reg in self.chip.regs() {
            let cell = prev.with_row(self.chip.row_reg_hclk(reg));
            if cell.row >= self.chip.row_clk() {
                break;
            }
            self.egrid
                .fill_conn_pair_id(prev, cell, defs::ccls::CMT_NEXT, defs::ccls::CMT_PREV);
            self.egrid
                .fill_conn_pair_id(prev, cell, defs::ccls::CMT_N, defs::ccls::CMT_S);
            prev = cell;
        }
        self.egrid.fill_conn_pair_id(
            prev,
            cell_clkc.delta(0, -1),
            defs::ccls::CMT_NEXT,
            defs::ccls::CMT_PREV,
        );
        self.egrid
            .fill_conn_pair_id(prev, cell_clkc, defs::ccls::CMT_N, defs::ccls::CMT_S);

        let mut prev = self.chip.bel_bufpll(Dir::N).cell;
        for reg in self.chip.regs().rev() {
            let cell = prev.with_row(self.chip.row_reg_hclk(reg));
            if cell.row <= self.chip.row_clk() {
                break;
            }
            self.egrid
                .fill_conn_pair_id(prev, cell, defs::ccls::CMT_NEXT, defs::ccls::CMT_PREV);
            self.egrid
                .fill_conn_pair_id(prev, cell, defs::ccls::CMT_S, defs::ccls::CMT_N);
            prev = cell;
        }
        self.egrid
            .fill_conn_pair_id(prev, cell_clkc, defs::ccls::CMT_NEXT, defs::ccls::CMT_PREV);
        self.egrid
            .fill_conn_pair_id(prev, cell_clkc, defs::ccls::CMT_S, defs::ccls::CMT_N);
    }

    fn fill_cmts(&mut self) {
        for (br, kind) in self.chip.get_dcms() {
            let cell = CellCoord::new(self.die, self.chip.col_clk, br);
            let buf_kind = match kind {
                DcmKind::Bot => defs::tcls::DCM_BUFPLL_BUF_S,
                DcmKind::BotMid => defs::tcls::DCM_BUFPLL_BUF_S_MID,
                DcmKind::Top => defs::tcls::DCM_BUFPLL_BUF_N,
                DcmKind::TopMid => defs::tcls::DCM_BUFPLL_BUF_N_MID,
            };
            self.site_holes.push(cell.delta(0, -1).rect(1, 2));
            for cell in [cell.delta(0, -1), cell] {
                self.egrid[cell.tile(defs::tslots::INT)].class = defs::tcls::INT_IOI;
                self.egrid
                    .add_tile_single_id(cell, defs::tcls::INTF_CMT_IOI);
            }
            self.egrid.add_tile_id(
                cell,
                defs::tcls::CMT_DCM,
                &[cell.delta(0, -1), cell, cell.delta(0, 16)],
            );
            self.egrid.add_tile_single_id(cell, buf_kind);
        }

        for (br, kind) in self.chip.get_plls() {
            let cell = CellCoord::new(self.die, self.chip.col_clk, br);
            let out = match kind {
                PllKind::BotOut0 => defs::tcls::PLL_BUFPLL_OUT0_S,
                PllKind::BotOut1 => defs::tcls::PLL_BUFPLL_OUT1_S,
                PllKind::BotNoOut => defs::tcls::PLL_BUFPLL_S,
                PllKind::TopOut0 => defs::tcls::PLL_BUFPLL_OUT0_N,
                PllKind::TopOut1 => defs::tcls::PLL_BUFPLL_OUT1_N,
                PllKind::TopNoOut => defs::tcls::PLL_BUFPLL_N,
            };
            self.site_holes.push(cell.delta(0, -1).rect(1, 2));
            self.egrid
                .add_tile_single_id(cell.delta(0, -1), defs::tcls::INTF_CMT);
            self.egrid[cell.tile(defs::tslots::INT)].class = defs::tcls::INT_IOI;
            self.egrid
                .add_tile_single_id(cell, defs::tcls::INTF_CMT_IOI);
            self.egrid.add_tile_id(
                cell,
                defs::tcls::CMT_PLL,
                &[cell.delta(0, -1), cell, cell.delta(0, -16)],
            );
            self.egrid.add_tile_single_id(cell, out);
        }
    }

    fn fill_gts_holes(&mut self) {
        if let Some(bcrd) = self.chip.bel_pcie() {
            for dy in 0..16 {
                self.fill_intf_rterm(bcrd.cell.delta(0, dy));
                self.fill_intf_lterm(bcrd.cell.delta(4, dy));
            }
        }
        if let Some(bcrd) = self.chip.bel_gtp(DirHV::NW) {
            let cell = bcrd.cell;
            for dy in 0..8 {
                self.egrid
                    .fill_conn_term_id(cell.delta(-7, -7 + dy), defs::ccls::TERM_E);
                self.egrid
                    .fill_conn_term_id(cell.delta(5, -7 + dy), defs::ccls::TERM_W);
                self.fill_intf_rterm(cell.delta(-5, -15 + dy));
                self.fill_intf_lterm(cell.delta(3, -15 + dy));
            }
        }
        if let Some(bcrd) = self.chip.bel_gtp(DirHV::NE) {
            let cell = bcrd.cell;
            for dy in 0..8 {
                self.egrid
                    .fill_conn_term_id(cell.delta(-5, -7 + dy), defs::ccls::TERM_E);
                self.egrid
                    .fill_conn_term_id(cell.delta(7, -7 + dy), defs::ccls::TERM_W);
                self.fill_intf_rterm(cell.delta(-3, -15 + dy));
                self.fill_intf_lterm(cell.delta(6, -15 + dy));
            }
        }
        if let Some(bcrd) = self.chip.bel_gtp(DirHV::SW) {
            let cell = bcrd.cell;
            for dy in 0..8 {
                self.egrid
                    .fill_conn_term_id(cell.delta(-7, dy), defs::ccls::TERM_E);
                self.egrid
                    .fill_conn_term_id(cell.delta(5, dy), defs::ccls::TERM_W);
                self.fill_intf_rterm(cell.delta(-5, 8 + dy));
                self.fill_intf_lterm(cell.delta(3, 8 + dy));
            }
        }
        if let Some(bcrd) = self.chip.bel_gtp(DirHV::SE) {
            let cell = bcrd.cell;
            for dy in 0..8 {
                self.egrid
                    .fill_conn_term_id(cell.delta(-5, dy), defs::ccls::TERM_E);
                self.egrid
                    .fill_conn_term_id(cell.delta(7, dy), defs::ccls::TERM_W);
                self.fill_intf_rterm(cell.delta(-3, 8 + dy));
                self.fill_intf_lterm(cell.delta(6, 8 + dy));
            }
        }
    }

    fn fill_gts(&mut self) {
        if self.disabled.contains(&DisabledPart::Gtp) {
            return;
        }
        if let Some(bcrd) = self.chip.bel_pcie() {
            let cell = bcrd.cell;
            let mut tcells = cell.cells_n(16);
            tcells.extend(cell.delta(4, 0).cells_n(16));
            self.egrid.add_tile_id(cell, defs::tcls::PCIE, &tcells);
            self.egrid
                .add_tile_id(tcells[8], defs::tcls::HCLK_CLEXL, &[]);
            self.egrid
                .add_tile_id(tcells[24], defs::tcls::HCLK_CLEXL, &[]);
        }
        if let Some(bcrd) = self.chip.bel_gtp(DirHV::NW) {
            let cell = bcrd.cell;
            let mut tcells = cell.delta(-5, -15).cells_n(8);
            tcells.extend(cell.delta(3, -15).cells_n(8));
            self.egrid.add_tile_id(cell, defs::tcls::GTP, &tcells);
            self.egrid
                .add_tile_id(tcells[7].delta(0, 1), defs::tcls::HCLK_CLEXL, &[]);
            self.egrid
                .add_tile_id(tcells[15].delta(0, 1), defs::tcls::HCLK_GTP, &[]);
            for cell in tcells {
                self.egrid[cell].region_root[rslots::PLLCLK] = self.chip.bel_bufpll(Dir::N).cell;
                self.egrid[cell].region_root[rslots::IOCLK] = self.chip.bel_bufpll(Dir::N).cell;
            }
        }
        if let Some(bcrd) = self.chip.bel_gtp(DirHV::NE) {
            let cell = bcrd.cell;
            let mut tcells = cell.delta(-3, -15).cells_n(8);
            tcells.extend(cell.delta(6, -15).cells_n(8));
            self.egrid.add_tile_id(cell, defs::tcls::GTP, &tcells);
            self.egrid
                .add_tile_id(tcells[7].delta(0, 1), defs::tcls::HCLK_CLEXL, &[]);
            self.egrid
                .add_tile_id(tcells[15].delta(0, 1), defs::tcls::HCLK_GTP, &[]);
            for cell in tcells {
                self.egrid[cell].region_root[rslots::PLLCLK] = self.chip.bel_bufpll(Dir::N).cell;
                self.egrid[cell].region_root[rslots::IOCLK] =
                    self.chip.bel_bufpll(Dir::N).cell.delta(1, 0);
            }
        }
        if let Some(bcrd) = self.chip.bel_gtp(DirHV::SW) {
            let cell = bcrd.cell;
            let mut tcells = cell.delta(-5, 8).cells_n(8);
            tcells.extend(cell.delta(3, 8).cells_n(8));
            self.egrid.add_tile_id(cell, defs::tcls::GTP, &tcells);
            self.egrid
                .add_tile_id(tcells[0], defs::tcls::HCLK_CLEXL, &[]);
            self.egrid.add_tile_id(tcells[8], defs::tcls::HCLK_GTP, &[]);
            for cell in tcells {
                self.egrid[cell].region_root[rslots::PLLCLK] = self.chip.bel_bufpll(Dir::S).cell;
                self.egrid[cell].region_root[rslots::IOCLK] = self.chip.bel_bufpll(Dir::S).cell;
            }
        }
        if let Some(bcrd) = self.chip.bel_gtp(DirHV::SE) {
            let cell = bcrd.cell;
            let mut tcells = cell.delta(-3, 8).cells_n(8);
            tcells.extend(cell.delta(6, 8).cells_n(8));
            self.egrid.add_tile_id(cell, defs::tcls::GTP, &tcells);
            self.egrid
                .add_tile_id(tcells[0], defs::tcls::HCLK_CLEXL, &[]);
            self.egrid.add_tile_id(tcells[8], defs::tcls::HCLK_GTP, &[]);
            for cell in tcells {
                self.egrid[cell].region_root[rslots::PLLCLK] = self.chip.bel_bufpll(Dir::S).cell;
                self.egrid[cell].region_root[rslots::IOCLK] =
                    self.chip.bel_bufpll(Dir::S).cell.delta(1, 0);
            }
        }
    }

    fn fill_term_sn(&mut self) {
        for cell in self.egrid.row(self.die, self.chip.row_s()) {
            let mut cell = cell;
            while self.is_int_hole(cell) {
                cell.row += 1;
            }
            if self.chip.columns[cell.col].kind != ColumnKind::Bram {
                self.egrid.fill_conn_term_id(cell, defs::ccls::TERM_S);
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_n()) {
            let mut cell = cell;
            while self.is_int_hole(cell) {
                cell.row -= 1;
            }
            self.egrid.fill_conn_term_id(cell, defs::ccls::TERM_N);
        }
    }

    fn fill_main_passes(&mut self) {
        let die = DieId::from_idx(0);
        // horizontal
        for row in self.egrid.rows(die) {
            let mut prev = None;
            for cell in self.egrid.row(die, row) {
                if !self.egrid[cell].tiles.contains_id(defs::tslots::INT) {
                    continue;
                }
                if let Some(prev) = prev
                    && !self.egrid[cell].conns.contains_id(defs::cslots::W)
                {
                    self.egrid.fill_conn_pair_id(
                        prev,
                        cell,
                        defs::ccls::PASS_E,
                        defs::ccls::PASS_W,
                    );
                }
                if !self.egrid[cell].conns.contains_id(defs::cslots::E) {
                    prev = Some(cell);
                } else {
                    prev = None;
                }
            }
        }
        // vertical
        for col in self.egrid.cols(die) {
            let mut prev = None;
            for cell in self.egrid.column(die, col) {
                if !self.egrid[cell].tiles.contains_id(defs::tslots::INT) {
                    continue;
                }
                if let Some(prev) = prev
                    && !self.egrid[cell].conns.contains_id(defs::cslots::S)
                {
                    self.egrid.fill_conn_pair_id(
                        prev,
                        cell,
                        defs::ccls::PASS_N,
                        defs::ccls::PASS_S,
                    );
                }
                if !self.egrid[cell].conns.contains_id(defs::cslots::N) {
                    prev = Some(cell);
                } else {
                    prev = None;
                }
            }
        }
    }

    fn fill_cle(&mut self) {
        for (col, &cd) in &self.chip.columns {
            if !matches!(
                cd.kind,
                ColumnKind::CleXL | ColumnKind::CleXM | ColumnKind::CleClk
            ) {
                continue;
            }
            if self.disabled.contains(&DisabledPart::ClbColumn(col)) {
                continue;
            }
            for cell in self.egrid.column(self.die, col) {
                if self.is_site_hole(cell) {
                    continue;
                }
                let (kind, hclk_kind) = if cd.kind == ColumnKind::CleXM {
                    (defs::tcls::CLEXM, defs::tcls::HCLK_CLEXM)
                } else {
                    (defs::tcls::CLEXL, defs::tcls::HCLK_CLEXL)
                };
                self.egrid.add_tile_single_id(cell, kind);
                if cell.row == self.chip.row_hclk(cell.row) {
                    self.egrid.add_tile_id(cell, hclk_kind, &[]);
                }
            }
        }
    }

    fn fill_bram(&mut self) {
        for (col, &cd) in &self.chip.columns {
            if cd.kind != ColumnKind::Bram {
                continue;
            }
            for cell in self.egrid.column(self.die, col) {
                if !cell.row.to_idx().is_multiple_of(4) {
                    continue;
                }
                let reg = self.chip.row_to_reg(cell.row);
                if self.disabled.contains(&DisabledPart::BramRegion(col, reg)) {
                    continue;
                }
                if self.is_site_hole(cell) {
                    continue;
                }
                self.egrid.add_tile_n_id(cell, defs::tcls::BRAM, 4);
            }
        }
    }

    fn fill_dsp(&mut self) {
        for (col, &cd) in &self.chip.columns {
            if !matches!(cd.kind, ColumnKind::Dsp | ColumnKind::DspPlus) {
                continue;
            }
            for cell in self.egrid.column(self.die, col) {
                if !cell.row.to_idx().is_multiple_of(4) {
                    continue;
                }
                let reg = self.chip.row_to_reg(cell.row);
                if self.disabled.contains(&DisabledPart::DspRegion(col, reg)) {
                    continue;
                }
                if self.is_site_hole(cell) {
                    continue;
                }
                self.egrid.add_tile_n_id(cell, defs::tcls::DSP, 4);
                if cell.row == self.chip.row_hclk(cell.row) {
                    self.egrid.add_tile_id(cell, defs::tcls::HCLK_CLEXL, &[]);
                }
            }
        }
    }

    fn fill_hclk(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            let crow = if cell.row.to_idx() % 16 < 8 {
                self.chip.row_hclk(cell.row) - 1
            } else {
                self.chip.row_hclk(cell.row)
            };
            let hcol = if cell.col <= self.chip.col_clk {
                self.chip.col_clk
            } else {
                self.chip.col_clk + 1
            };
            self.egrid[cell].region_root[rslots::HROW] =
                cell.with_cr(hcol, self.chip.row_hclk(cell.row));
            self.egrid[cell].region_root[rslots::LEAF] = cell.with_row(crow);
            self.egrid[cell].region_root[rslots::GLOBAL] =
                cell.with_cr(self.chip.col_clk, self.chip.row_clk());
            self.egrid[cell].region_root[rslots::DIVCLK_CMT] = cell.with_cr(
                self.chip.col_clk,
                if cell.row < self.chip.row_clk() {
                    self.chip.row_clk() - 1
                } else {
                    self.chip.row_clk()
                },
            );

            if cell.row.to_idx() % 16 == 8 {
                if self.is_int_hole(cell) && self.is_int_hole(cell.delta(0, -1)) {
                    continue;
                }
                self.egrid
                    .add_tile_id(cell, defs::tcls::HCLK, &[cell.delta(0, -1), cell]);
            }
        }
    }

    fn fill_ioi(&mut self, cell: CellCoord) {
        let tile = &mut self.egrid[cell];
        let tile = &mut tile.tiles[defs::tslots::INT];
        tile.class = defs::tcls::INT_IOI;
        self.egrid.add_tile_id(cell, defs::tcls::INTF_IOI, &[cell]);
        let kind = if cell.col == self.chip.col_w() || cell.col == self.chip.col_e() {
            self.egrid[cell].region_root[rslots::PLLCLK] = cell.with_row(self.chip.row_clk());
            self.egrid[cell].region_root[rslots::IOCLK] =
                cell.with_row(if self.chip.row_hclk(cell.row) <= self.chip.row_clk() {
                    self.chip.row_clk() - 1
                } else {
                    self.chip.row_clk()
                });
            defs::tcls::IOI_WE
        } else {
            let row = if cell.row < self.chip.row_clk() {
                self.chip.row_s()
            } else {
                self.chip.row_n()
            };
            self.egrid[cell].region_root[rslots::PLLCLK] = cell.with_cr(self.chip.col_clk, row);
            self.egrid[cell].region_root[rslots::IOCLK] = cell.with_cr(
                if cell.col <= self.chip.col_clk {
                    self.chip.col_clk
                } else {
                    self.chip.col_clk + 1
                },
                row,
            );
            defs::tcls::IOI_SN
        };
        self.egrid.add_tile_id(cell, kind, &[cell]);
    }

    fn fill_intf_rterm(&mut self, cell: CellCoord) {
        self.egrid.fill_conn_term_id(cell, defs::ccls::TERM_E);
        self.egrid.add_tile_single_id(cell, defs::tcls::INTF);
    }

    fn fill_intf_lterm(&mut self, cell: CellCoord) {
        self.egrid.fill_conn_term_id(cell, defs::ccls::TERM_W);
        self.egrid.add_tile_single_id(cell, defs::tcls::INTF);
    }

    fn fill_global(&mut self) {
        self.egrid.add_tile_id(
            CellCoord::new(self.die, self.chip.col_w(), self.chip.row_s()),
            defs::tcls::GLOBAL,
            &[],
        );
    }

    fn fill_frame_info(&mut self) {
        for (_, cd) in &self.chip.columns {
            let width = match cd.kind {
                ColumnKind::CleXL => 30,
                ColumnKind::CleXM => 31,
                ColumnKind::CleClk => 31,
                ColumnKind::Bram => 25,
                ColumnKind::Dsp => 24,
                ColumnKind::DspPlus => 31,
                ColumnKind::Io => 30,
            };
            self.col_width.push(width);
        }
        for reg in self.chip.regs() {
            self.col_frame.push(EntityVec::new());
            self.bram_frame.push(EntityPartVec::new());
            let mut major = 0;
            let mut bram_major = 0;
            self.spine_frame.push(self.frame_info.len());
            for minor in 0..4 {
                self.frame_info.push(FrameInfo {
                    addr: FrameAddr {
                        typ: 0,
                        region: reg.to_idx() as i32,
                        major,
                        minor: minor as u32,
                    },
                    mask_mode: [].into_iter().collect(),
                });
            }
            major += 1;
            for (col, cd) in &self.chip.columns {
                self.col_frame[reg].push(self.frame_info.len());
                for minor in 0..self.col_width[col] {
                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 0,
                            region: reg.to_idx() as i32,
                            major,
                            minor: minor as u32,
                        },
                        mask_mode: [].into_iter().collect(),
                    });
                }
                major += 1;
                if cd.kind == ColumnKind::Bram {
                    self.bram_frame[reg].insert(col, self.bram_frame_info.len());
                    // XXX uncertain
                    for minor in 0..4 {
                        self.bram_frame_info.push(FrameInfo {
                            addr: FrameAddr {
                                typ: 1,
                                region: reg.to_idx() as i32,
                                major: bram_major,
                                minor,
                            },
                            mask_mode: [].into_iter().collect(),
                        });
                    }
                    bram_major += 1;
                }
            }
        }
    }

    fn fill_iob_frame_info(&mut self) {
        for cell in self.egrid.column(self.die, self.chip.col_e()) {
            if cell.row == self.chip.row_clk() {
                self.reg_frame[Dir::E] = self.iob_frame_len;
                self.iob_frame_len += 384;
            }
            if self.chip.rows[cell.row].io_e {
                self.iob_frame.insert(cell, self.iob_frame_len);
                self.iob_frame_len += 128;
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_n()).rev() {
            if self.chip.columns[cell.col].kind == ColumnKind::CleClk {
                self.reg_frame[Dir::N] = self.iob_frame_len;
                self.iob_frame_len += 384;
            }
            if matches!(
                self.chip.columns[cell.col].io_n,
                ColumnIoKind::Inner | ColumnIoKind::Both
            ) {
                self.iob_frame.insert(cell.delta(0, -1), self.iob_frame_len);
                self.iob_frame_len += 128;
            }
            if matches!(
                self.chip.columns[cell.col].io_n,
                ColumnIoKind::Outer | ColumnIoKind::Both
            ) {
                self.iob_frame.insert(cell, self.iob_frame_len);
                self.iob_frame_len += 128;
            }
        }
        for cell in self.egrid.column(self.die, self.chip.col_w()).rev() {
            if self.chip.rows[cell.row].io_w {
                self.iob_frame.insert(cell, self.iob_frame_len);
                self.iob_frame_len += 128;
            }
            if cell.row == self.chip.row_clk() {
                self.reg_frame[Dir::W] = self.iob_frame_len;
                self.iob_frame_len += 384;
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_s()) {
            if matches!(
                self.chip.columns[cell.col].io_s,
                ColumnIoKind::Inner | ColumnIoKind::Both
            ) {
                self.iob_frame.insert(cell.delta(0, 1), self.iob_frame_len);
                self.iob_frame_len += 128;
            }
            if matches!(
                self.chip.columns[cell.col].io_s,
                ColumnIoKind::Outer | ColumnIoKind::Both
            ) {
                self.iob_frame.insert(cell, self.iob_frame_len);
                self.iob_frame_len += 128;
            }
            if self.chip.columns[cell.col].kind == ColumnKind::CleClk {
                self.reg_frame[Dir::S] = self.iob_frame_len;
                self.iob_frame_len += 384;
            }
        }
    }
}

impl Chip {
    pub fn expand_grid<'a>(
        &'a self,
        db: &'a IntDb,
        disabled: &BTreeSet<DisabledPart>,
    ) -> ExpandedDevice<'a> {
        let mut egrid = GridBuilder::new(db);
        let die = egrid.add_die(self.columns.len(), self.rows.len());
        let disabled = disabled.clone();

        let mut expander = Expander {
            chip: self,
            disabled: &disabled,
            egrid: &mut egrid,
            die,
            int_holes: vec![],
            site_holes: vec![],
            frame_info: vec![],
            bram_frame_info: vec![],
            iob_frame_len: 0,
            col_frame: EntityVec::new(),
            col_width: EntityVec::new(),
            spine_frame: EntityVec::new(),
            bram_frame: EntityVec::new(),
            iob_frame: HashMap::new(),
            reg_frame: DirMap::from_fn(|_| 0),
        };

        expander.fill_holes();
        expander.fill_int();
        expander.fill_io_n();
        expander.fill_io_e();
        expander.fill_io_s();
        expander.fill_io_w();
        expander.fill_mcb();
        expander.fill_pcilogic();
        expander.fill_spine();
        expander.fill_cmts();
        expander.fill_gts_holes();
        expander.fill_term_sn();
        expander.fill_main_passes();
        expander.fill_gts();
        expander.fill_bram();
        expander.fill_dsp();
        expander.fill_cle();
        expander.fill_hclk();
        expander.fill_global();
        expander.fill_frame_info();
        expander.fill_iob_frame_info();

        let die_bs_geom = DieBitstreamGeom {
            frame_len: 1040,
            frame_info: expander.frame_info,
            bram_frame_len: 1040 * 18,
            bram_frame_info: expander.bram_frame_info,
            iob_frame_len: expander.iob_frame_len,
        };
        let bs_geom = BitstreamGeom {
            kind: DeviceKind::Spartan6,
            die: [die_bs_geom].into_iter().collect(),
            die_order: vec![expander.die],
            has_gtz_bot: false,
            has_gtz_top: false,
        };
        let site_holes = expander.site_holes;
        let col_frame = expander.col_frame;
        let col_width = expander.col_width;
        let spine_frame = expander.spine_frame;
        let bram_frame = expander.bram_frame;
        let iob_frame = expander.iob_frame;
        let reg_frame = expander.reg_frame;

        let egrid = egrid.finish();
        ExpandedDevice {
            chip: self,
            disabled,
            egrid,
            site_holes,
            bs_geom,
            col_frame,
            col_width,
            spine_frame,
            bram_frame,
            iob_frame,
            reg_frame,
        }
    }
}
