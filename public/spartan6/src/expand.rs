use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::dir::{Dir, DirH, DirHV, DirMap};
use prjcombine_interconnect::grid::{CellCoord, ColId, DieId, ExpandedGrid, Rect};
use prjcombine_xilinx_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};
use std::collections::{BTreeSet, HashMap};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::chip::{Chip, ColumnIoKind, ColumnKind, DcmKind, DisabledPart, PllKind, RegId};
use crate::expanded::ExpandedDevice;
use crate::{regions, tslots};

struct Expander<'a, 'b> {
    chip: &'b Chip,
    db: &'a IntDb,
    disabled: &'a BTreeSet<DisabledPart>,
    egrid: &'a mut ExpandedGrid<'b>,
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
            self.egrid.add_tile_single(cell, "INT");
            if self.is_site_hole(cell) {
                continue;
            }
            if matches!(
                self.chip.columns[cell.col].kind,
                ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::DspPlus
            ) {
                self.egrid.add_tile_single(cell, "INTF");
            }
        }
    }

    fn fill_lio(&mut self) {
        for cell in self.egrid.column(self.die, self.chip.col_w()) {
            if self.chip.rows[cell.row].lio {
                self.fill_ioi(cell);
                self.egrid.add_tile(cell, "IOB", &[]);
            } else {
                self.egrid.add_tile_single(cell, "INTF");
                if cell.row == self.chip.row_bio_outer() {
                    self.egrid.add_tile_single(cell, "LL");
                } else if cell.row == self.chip.row_tio_outer() {
                    self.egrid.add_tile_single(cell, "UL");
                }
            }

            if cell.row == self.chip.row_clk() - 2
                || cell.row == self.chip.row_clk() - 1
                || cell.row == self.chip.row_clk() + 2
                || cell.row == self.chip.row_clk() + 3
            {
                self.egrid.add_tile(cell, "CLKPIN_BUF", &[]);
            }
            self.egrid.fill_conn_term(cell, "TERM.W");

            if cell.row.to_idx() % 16 == 8 {
                self.egrid.add_tile(cell, "LRIOI_CLK", &[]);
                if cell.row == self.chip.rows_pci_ce_split.0
                    || cell.row == self.chip.rows_pci_ce_split.1
                {
                    self.egrid.add_tile(cell, "PCI_CE_SPLIT", &[]);
                } else {
                    self.egrid.add_tile(cell, "PCI_CE_TRUNK_BUF", &[]);
                    if cell.row != self.chip.row_clk() {
                        self.egrid.add_tile(cell, "PCI_CE_V_BUF", &[]);
                    }
                }
            }

            if cell.row == self.chip.row_bio_outer() || cell.row == self.chip.row_tio_outer() {
                self.egrid.add_tile(cell, "PCI_CE_H_BUF", &[]);
            }
        }
    }

    fn fill_rio(&mut self) {
        for cell in self.egrid.column(self.die, self.chip.col_e()).rev() {
            if self.chip.rows[cell.row].rio {
                self.fill_ioi(cell);
                self.egrid.add_tile(cell, "IOB", &[]);
            } else {
                self.egrid.add_tile(cell, "INTF", &[cell]);
                if cell.row == self.chip.row_bio_outer() {
                    self.egrid.add_tile_n(cell, "LR", 2);
                } else if cell.row == self.chip.row_tio_inner() {
                    self.egrid.add_tile_n(cell, "UR", 2);
                }
            }

            if cell.row == self.chip.row_clk() - 2
                || cell.row == self.chip.row_clk() - 1
                || cell.row == self.chip.row_clk() + 2
                || cell.row == self.chip.row_clk() + 3
            {
                self.egrid.add_tile(cell, "CLKPIN_BUF", &[]);
            }
            self.egrid.fill_conn_term(cell, "TERM.E");

            if cell.row.to_idx() % 16 == 8 {
                self.egrid.add_tile(cell, "LRIOI_CLK", &[]);
                if cell.row == self.chip.rows_pci_ce_split.0
                    || cell.row == self.chip.rows_pci_ce_split.1
                {
                    self.egrid.add_tile(cell, "PCI_CE_SPLIT", &[]);
                } else {
                    self.egrid.add_tile(cell, "PCI_CE_TRUNK_BUF", &[]);
                    if cell.row != self.chip.row_clk()
                        && !(self.chip.has_encrypt && cell.row.to_idx() == 8)
                    {
                        self.egrid.add_tile(cell, "PCI_CE_V_BUF", &[]);
                    }
                }
            }

            if cell.row == self.chip.row_bio_outer() || cell.row == self.chip.row_tio_outer() {
                self.egrid.add_tile(cell, "PCI_CE_H_BUF", &[]);
            }
        }
    }

    fn fill_tio(&mut self) {
        for cell_o in self.egrid.row(self.die, self.chip.row_tio_outer()) {
            let cd = &self.chip.columns[cell_o.col];
            if cd.tio == ColumnIoKind::None {
                continue;
            }
            self.site_holes.push(cell_o.delta(0, -1).rect(1, 2));
            for (cell, unused) in [
                (cell_o, cd.tio == ColumnIoKind::Inner),
                (cell_o.delta(0, -1), cd.tio == ColumnIoKind::Outer),
            ] {
                self.fill_ioi(cell);
                if !unused {
                    self.egrid.add_tile(cell, "IOB", &[]);
                }
            }
            let is_clk = cell_o.col == self.chip.col_clk || cell_o.col == self.chip.col_clk + 1;
            self.egrid.add_tile(cell_o, "BTIOI_CLK", &[]);
            if is_clk {
                self.egrid.add_tile(cell_o.delta(0, -1), "CLKPIN_BUF", &[]);
                self.egrid.add_tile(cell_o, "CLKPIN_BUF", &[]);
            }
        }
    }

    fn fill_bio(&mut self) {
        for cell_o in self.egrid.row(self.die, self.chip.row_bio_outer()) {
            let cd = &self.chip.columns[cell_o.col];
            if cd.bio == ColumnIoKind::None {
                continue;
            }
            self.site_holes.push(cell_o.rect(1, 2));
            for (cell, unused) in [
                (cell_o, cd.bio == ColumnIoKind::Inner),
                (cell_o.delta(0, 1), cd.bio == ColumnIoKind::Outer),
            ] {
                self.fill_ioi(cell);
                if !unused {
                    self.egrid.add_tile(cell, "IOB", &[]);
                }
            }
            let is_clk = cell_o.col == self.chip.col_clk || cell_o.col == self.chip.col_clk + 1;
            self.egrid.add_tile(cell_o, "BTIOI_CLK", &[]);
            if is_clk {
                self.egrid.add_tile(cell_o, "CLKPIN_BUF", &[]);
                self.egrid.add_tile(cell_o.delta(0, 1), "CLKPIN_BUF", &[]);
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
                self.egrid.add_tile(cell, "MCB", &tcells);
            }
        }
    }

    fn fill_pcilogic(&mut self) {
        for edge in [DirH::W, DirH::E] {
            self.egrid
                .add_tile_single(self.chip.bel_pcilogicse(edge).cell, "PCILOGICSE");
        }
    }

    fn fill_spine(&mut self) {
        {
            let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_clk());
            self.site_holes.push(cell.rect(1, 1));
            self.egrid.add_tile_single(cell, "INTF");
            self.egrid.add_tile_single(cell, "CLKC");
        }

        for row in [self.chip.rows_hclkbuf.0, self.chip.rows_hclkbuf.1] {
            let cell = CellCoord::new(self.die, self.chip.col_clk, row);
            self.egrid.add_tile(cell, "HCLK_V_MIDBUF", &[]);
        }

        for row in [self.chip.rows_midbuf.0, self.chip.rows_midbuf.1] {
            let cell = CellCoord::new(self.die, self.chip.col_clk, row);
            self.egrid.add_tile(cell, "CKPIN_V_MIDBUF", &[]);
        }

        {
            let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_bio_outer());
            self.egrid.add_tile(cell, "REG_B", &[cell.delta(1, 1)]);
        }

        {
            let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_tio_outer());
            self.egrid.add_tile(cell, "REG_T", &[cell.delta(1, 0)]);
        }

        for cell in self.egrid.column(self.die, self.chip.col_clk) {
            if cell.row.to_idx() % 16 == 8 {
                self.egrid.add_tile(cell, "HCLK_ROW", &[]);
            }
        }

        for col in [self.chip.cols_reg_buf.0, self.chip.cols_reg_buf.1] {
            let cell = CellCoord::new(self.die, col, self.chip.row_clk());
            self.egrid.add_tile(cell, "CKPIN_H_MIDBUF", &[]);
        }

        {
            let cell = CellCoord::new(self.die, self.chip.col_w(), self.chip.row_clk());
            self.egrid.add_tile_n(cell, "REG_L", 2);
        }

        {
            let cell = CellCoord::new(self.die, self.chip.col_e(), self.chip.row_clk());
            self.egrid.add_tile_n(cell, "REG_R", 2);
        }
    }

    fn fill_cmts(&mut self) {
        for (br, kind) in self.chip.get_dcms() {
            let cell = CellCoord::new(self.die, self.chip.col_clk, br);
            let buf_kind = match kind {
                DcmKind::Bot => "DCM_BUFPLL_BUF_S",
                DcmKind::BotMid => "DCM_BUFPLL_BUF_S_MID",
                DcmKind::Top => "DCM_BUFPLL_BUF_N",
                DcmKind::TopMid => "DCM_BUFPLL_BUF_N_MID",
            };
            self.site_holes.push(cell.delta(0, -1).rect(1, 2));
            for cell in [cell.delta(0, -1), cell] {
                self.egrid[cell.tile(tslots::INT)].class = self.db.get_tile_class("INT.IOI");
                self.egrid.add_tile_single(cell, "INTF.CMT.IOI");
            }
            self.egrid.add_tile_sn(cell, "CMT_DCM", 1, 2);
            self.egrid.add_tile(cell, buf_kind, &[]);
        }

        for (br, kind) in self.chip.get_plls() {
            let cell = CellCoord::new(self.die, self.chip.col_clk, br);
            let out = match kind {
                PllKind::BotOut0 => "PLL_BUFPLL_OUT0",
                PllKind::BotOut1 => "PLL_BUFPLL_OUT1",
                PllKind::BotNoOut => "PLL_BUFPLL_B",
                PllKind::TopOut0 => "PLL_BUFPLL_OUT0",
                PllKind::TopOut1 => "PLL_BUFPLL_OUT1",
                PllKind::TopNoOut => "PLL_BUFPLL_T",
            };
            self.site_holes.push(cell.delta(0, -1).rect(1, 2));
            self.egrid.add_tile_single(cell.delta(0, -1), "INTF.CMT");
            self.egrid[cell.tile(tslots::INT)].class = self.db.get_tile_class("INT.IOI");
            self.egrid.add_tile_single(cell, "INTF.CMT.IOI");
            self.egrid.add_tile_sn(cell, "CMT_PLL", 1, 2);
            self.egrid.add_tile(cell, out, &[]);
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
                self.egrid.fill_conn_term(cell.delta(-7, -7 + dy), "TERM.E");
                self.egrid.fill_conn_term(cell.delta(5, -7 + dy), "TERM.W");
                self.fill_intf_rterm(cell.delta(-5, -15 + dy));
                self.fill_intf_lterm(cell.delta(3, -15 + dy));
            }
        }
        if let Some(bcrd) = self.chip.bel_gtp(DirHV::NE) {
            let cell = bcrd.cell;
            for dy in 0..8 {
                self.egrid.fill_conn_term(cell.delta(-5, -7 + dy), "TERM.E");
                self.egrid.fill_conn_term(cell.delta(7, -7 + dy), "TERM.W");
                self.fill_intf_rterm(cell.delta(-3, -15 + dy));
                self.fill_intf_lterm(cell.delta(6, -15 + dy));
            }
        }
        if let Some(bcrd) = self.chip.bel_gtp(DirHV::SW) {
            let cell = bcrd.cell;
            for dy in 0..8 {
                self.egrid.fill_conn_term(cell.delta(-7, dy), "TERM.E");
                self.egrid.fill_conn_term(cell.delta(5, dy), "TERM.W");
                self.fill_intf_rterm(cell.delta(-5, 8 + dy));
                self.fill_intf_lterm(cell.delta(3, 8 + dy));
            }
        }
        if let Some(bcrd) = self.chip.bel_gtp(DirHV::SE) {
            let cell = bcrd.cell;
            for dy in 0..8 {
                self.egrid.fill_conn_term(cell.delta(-5, dy), "TERM.E");
                self.egrid.fill_conn_term(cell.delta(7, dy), "TERM.W");
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
            self.egrid.add_tile(cell, "PCIE", &tcells);
        }
        if let Some(bcrd) = self.chip.bel_gtp(DirHV::NW) {
            let cell = bcrd.cell;
            let mut tcells = cell.delta(-5, -15).cells_n(8);
            tcells.extend(cell.delta(3, -15).cells_n(8));
            self.egrid.add_tile(cell, "GTP", &tcells);
        }
        if let Some(bcrd) = self.chip.bel_gtp(DirHV::NE) {
            let cell = bcrd.cell;
            let mut tcells = cell.delta(-3, -15).cells_n(8);
            tcells.extend(cell.delta(6, -15).cells_n(8));
            self.egrid.add_tile(cell, "GTP", &tcells);
        }
        if let Some(bcrd) = self.chip.bel_gtp(DirHV::SW) {
            let cell = bcrd.cell;
            let mut tcells = cell.delta(-5, 8).cells_n(8);
            tcells.extend(cell.delta(3, 8).cells_n(8));
            self.egrid.add_tile(cell, "GTP", &tcells);
        }
        if let Some(bcrd) = self.chip.bel_gtp(DirHV::SE) {
            let cell = bcrd.cell;
            let mut tcells = cell.delta(-3, 8).cells_n(8);
            tcells.extend(cell.delta(6, 8).cells_n(8));
            self.egrid.add_tile(cell, "GTP", &tcells);
        }
    }

    fn fill_btterm(&mut self) {
        for cell in self.egrid.row(self.die, self.chip.row_bio_outer()) {
            let mut cell = cell;
            while self.is_int_hole(cell) {
                cell.row += 1;
            }
            if self.chip.columns[cell.col].kind != ColumnKind::Bram {
                self.egrid.fill_conn_term(cell, "TERM.S");
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_tio_outer()) {
            let mut cell = cell;
            while self.is_int_hole(cell) {
                cell.row -= 1;
            }
            self.egrid.fill_conn_term(cell, "TERM.N");
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
                let kind = if cd.kind == ColumnKind::CleXM {
                    "CLEXM"
                } else {
                    "CLEXL"
                };
                self.egrid.add_tile_single(cell, kind);
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
                self.egrid.add_tile_n(cell, "BRAM", 4);
            }

            let row = self.chip.row_bio_outer();
            self.egrid
                .add_tile(CellCoord::new(self.die, col, row), "PCI_CE_H_BUF", &[]);

            let row = self.chip.row_tio_outer();
            self.egrid
                .add_tile(CellCoord::new(self.die, col, row), "PCI_CE_H_BUF", &[]);
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
                self.egrid.add_tile_n(cell, "DSP", 4);
            }

            let row = self.chip.row_bio_outer();
            self.egrid
                .add_tile(CellCoord::new(self.die, col, row), "PCI_CE_H_BUF", &[]);

            let row = self.chip.row_tio_outer();
            self.egrid
                .add_tile(CellCoord::new(self.die, col, row), "PCI_CE_H_BUF", &[]);
        }
    }

    fn fill_hclk_fold(&mut self) {
        if let Some((col_w, col_e)) = self.chip.cols_clk_fold {
            for col in [col_w, col_e] {
                for cell in self.egrid.column(self.die, col) {
                    if cell.row.to_idx() % 16 != 8 {
                        continue;
                    }
                    self.egrid.add_tile(cell, "HCLK_H_MIDBUF", &[]);
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
            self.egrid[cell].region_root[regions::HCLK] = cell.with_cr(hcol, crow);
            self.egrid[cell].region_root[regions::LEAF] = cell.with_row(crow);

            if cell.row.to_idx() % 16 == 8 {
                if self.is_int_hole(cell) && self.is_int_hole(cell.delta(0, -1)) {
                    continue;
                }
                self.egrid
                    .add_tile(cell, "HCLK", &[cell.delta(0, -1), cell]);
            }
        }
    }

    fn fill_ioi(&mut self, cell: CellCoord) {
        let tile = &mut self.egrid[cell];
        let tile = &mut tile.tiles[tslots::INT];
        tile.class = self.db.get_tile_class("INT.IOI");
        self.egrid.add_tile(cell, "INTF.IOI", &[cell]);
        let kind = if cell.col == self.chip.col_w() || cell.col == self.chip.col_e() {
            "IOI.LR"
        } else {
            "IOI.BT"
        };
        self.egrid.add_tile(cell, kind, &[cell]);
    }

    fn fill_intf_rterm(&mut self, cell: CellCoord) {
        self.egrid.fill_conn_term(cell, "TERM.E");
        self.egrid.add_tile_single(cell, "INTF");
    }

    fn fill_intf_lterm(&mut self, cell: CellCoord) {
        self.egrid.fill_conn_term(cell, "TERM.W");
        self.egrid.add_tile_single(cell, "INTF");
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
            if self.chip.rows[cell.row].rio {
                self.iob_frame.insert(cell, self.iob_frame_len);
                self.iob_frame_len += 128;
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_tio_outer()).rev() {
            if self.chip.columns[cell.col].kind == ColumnKind::CleClk {
                self.reg_frame[Dir::N] = self.iob_frame_len;
                self.iob_frame_len += 384;
            }
            if matches!(
                self.chip.columns[cell.col].tio,
                ColumnIoKind::Inner | ColumnIoKind::Both
            ) {
                self.iob_frame.insert(cell.delta(0, -1), self.iob_frame_len);
                self.iob_frame_len += 128;
            }
            if matches!(
                self.chip.columns[cell.col].tio,
                ColumnIoKind::Outer | ColumnIoKind::Both
            ) {
                self.iob_frame.insert(cell, self.iob_frame_len);
                self.iob_frame_len += 128;
            }
        }
        for cell in self.egrid.column(self.die, self.chip.col_w()).rev() {
            if self.chip.rows[cell.row].lio {
                self.iob_frame.insert(cell, self.iob_frame_len);
                self.iob_frame_len += 128;
            }
            if cell.row == self.chip.row_clk() {
                self.reg_frame[Dir::W] = self.iob_frame_len;
                self.iob_frame_len += 384;
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_bio_outer()) {
            if matches!(
                self.chip.columns[cell.col].bio,
                ColumnIoKind::Inner | ColumnIoKind::Both
            ) {
                self.iob_frame.insert(cell.delta(0, 1), self.iob_frame_len);
                self.iob_frame_len += 128;
            }
            if matches!(
                self.chip.columns[cell.col].bio,
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
        let mut egrid = ExpandedGrid::new(db);
        let die = egrid.add_die(self.columns.len(), self.rows.len());
        let disabled = disabled.clone();

        let mut expander = Expander {
            chip: self,
            db,
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
        expander.fill_tio();
        expander.fill_rio();
        expander.fill_bio();
        expander.fill_lio();
        expander.fill_mcb();
        expander.fill_pcilogic();
        expander.fill_spine();
        expander.fill_cmts();
        expander.fill_gts_holes();
        expander.fill_btterm();
        expander.egrid.fill_main_passes(expander.die);
        expander.fill_gts();
        expander.fill_bram();
        expander.fill_dsp();
        expander.fill_cle();
        expander.fill_hclk_fold();
        expander.fill_hclk();
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

        egrid.finish();
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
