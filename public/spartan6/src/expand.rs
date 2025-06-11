use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::dir::{Dir, DirMap};
use prjcombine_interconnect::grid::{ColId, Coord, ExpandedDieRefMut, ExpandedGrid, Rect, RowId};
use prjcombine_xilinx_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};
use std::collections::{BTreeSet, HashMap};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::chip::{Chip, ColumnIoKind, ColumnKind, DcmKind, DisabledPart, Gts, PllKind, RegId};
use crate::expanded::{ExpandedDevice, REGION_HCLK, REGION_LEAF};
use crate::tslots;

struct Expander<'a, 'b> {
    chip: &'b Chip,
    db: &'a IntDb,
    disabled: &'b BTreeSet<DisabledPart>,
    die: ExpandedDieRefMut<'a, 'b>,
    int_holes: Vec<Rect>,
    site_holes: Vec<Rect>,
    frame_info: Vec<FrameInfo>,
    bram_frame_info: Vec<FrameInfo>,
    iob_frame_len: usize,
    col_frame: EntityVec<RegId, EntityVec<ColId, usize>>,
    col_width: EntityVec<ColId, usize>,
    spine_frame: EntityVec<RegId, usize>,
    bram_frame: EntityVec<RegId, EntityPartVec<ColId, usize>>,
    iob_frame: HashMap<(ColId, RowId), usize>,
    reg_frame: DirMap<usize>,
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
        if let Gts::Single(bc) | Gts::Double(bc, _) | Gts::Quad(bc, _) = self.chip.gts {
            let row_gt_mid = self.chip.row_top() - 8;
            let row_gt_bot = row_gt_mid - 8;
            let row_pcie_bot = row_gt_bot - 16;
            self.int_holes.push(Rect {
                col_l: bc - 6,
                col_r: bc + 5,
                row_b: row_gt_mid,
                row_t: row_gt_mid + 8,
            });
            self.site_holes.push(Rect {
                col_l: bc - 6,
                col_r: bc + 5,
                row_b: row_gt_mid,
                row_t: row_gt_mid + 8,
            });
            self.int_holes.push(Rect {
                col_l: bc - 4,
                col_r: bc + 3,
                row_b: row_gt_bot,
                row_t: row_gt_mid,
            });
            self.site_holes.push(Rect {
                col_l: bc - 5,
                col_r: bc + 4,
                row_b: row_gt_bot,
                row_t: row_gt_mid,
            });

            // PCIE
            self.int_holes.push(Rect {
                col_l: bc - 1,
                col_r: bc + 2,
                row_b: row_pcie_bot,
                row_t: row_gt_bot,
            });
            self.site_holes.push(Rect {
                col_l: bc - 2,
                col_r: bc + 3,
                row_b: row_pcie_bot,
                row_t: row_gt_bot,
            });
        }
        if let Gts::Double(_, bc) | Gts::Quad(_, bc) = self.chip.gts {
            let row_gt_mid = self.chip.row_top() - 8;
            let row_gt_bot = row_gt_mid - 8;
            self.int_holes.push(Rect {
                col_l: bc - 4,
                col_r: bc + 7,
                row_b: row_gt_mid,
                row_t: row_gt_mid + 8,
            });
            self.site_holes.push(Rect {
                col_l: bc - 4,
                col_r: bc + 7,
                row_b: row_gt_mid,
                row_t: row_gt_mid + 8,
            });
            self.int_holes.push(Rect {
                col_l: bc - 2,
                col_r: bc + 6,
                row_b: row_gt_bot,
                row_t: row_gt_mid,
            });
            self.site_holes.push(Rect {
                col_l: bc - 3,
                col_r: bc + 7,
                row_b: row_gt_bot,
                row_t: row_gt_mid,
            });
        }
        if let Gts::Quad(bcl, bcr) = self.chip.gts {
            let row_gt_bot = RowId::from_idx(0);
            let row_gt_mid = RowId::from_idx(8);
            self.int_holes.push(Rect {
                col_l: bcl - 6,
                col_r: bcl + 5,
                row_b: row_gt_bot,
                row_t: row_gt_mid,
            });
            self.site_holes.push(Rect {
                col_l: bcl - 6,
                col_r: bcl + 5,
                row_b: row_gt_bot,
                row_t: row_gt_mid,
            });
            self.int_holes.push(Rect {
                col_l: bcl - 4,
                col_r: bcl + 3,
                row_b: row_gt_mid,
                row_t: row_gt_mid + 8,
            });
            self.site_holes.push(Rect {
                col_l: bcl - 5,
                col_r: bcl + 4,
                row_b: row_gt_mid,
                row_t: row_gt_mid + 8,
            });

            // right
            self.int_holes.push(Rect {
                col_l: bcr - 4,
                col_r: bcr + 7,
                row_b: row_gt_bot,
                row_t: row_gt_mid,
            });
            self.site_holes.push(Rect {
                col_l: bcr - 4,
                col_r: bcr + 7,
                row_b: row_gt_bot,
                row_t: row_gt_mid,
            });
            self.int_holes.push(Rect {
                col_l: bcr - 2,
                col_r: bcr + 6,
                row_b: row_gt_mid,
                row_t: row_gt_mid + 8,
            });
            self.site_holes.push(Rect {
                col_l: bcr - 3,
                col_r: bcr + 7,
                row_b: row_gt_mid,
                row_t: row_gt_mid + 8,
            });
        }
    }

    fn fill_int(&mut self) {
        for (col, &cd) in &self.chip.columns {
            for row in self.die.rows() {
                if self.is_int_hole(col, row) {
                    continue;
                }
                self.die.add_tile((col, row), "INT", &[(col, row)]);
                if self.is_site_hole(col, row) {
                    continue;
                }
                if matches!(
                    cd.kind,
                    ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::DspPlus
                ) {
                    self.die.add_tile((col, row), "INTF", &[(col, row)]);
                }
            }
        }
    }

    fn fill_lio(&mut self) {
        let col = self.chip.col_lio();
        for (row, &rd) in &self.chip.rows {
            if rd.lio {
                self.fill_ioi((col, row));
                self.die.add_tile((col, row), "IOB", &[]);
            } else {
                self.die.add_tile((col, row), "INTF", &[(col, row)]);
                if row == self.chip.row_bio_outer() {
                    self.die.add_tile((col, row), "LL", &[(col, row)]);
                } else if row == self.chip.row_tio_outer() {
                    self.die.add_tile((col, row), "UL", &[(col, row)]);
                }
            }

            if row == self.chip.row_clk() - 2
                || row == self.chip.row_clk() - 1
                || row == self.chip.row_clk() + 2
                || row == self.chip.row_clk() + 3
            {
                self.die.add_tile((col, row), "CLKPIN_BUF", &[]);
            }
            self.die.fill_conn_term((col, row), "TERM.W");

            if row.to_idx() % 16 == 8 {
                self.die.add_tile((col, row), "LRIOI_CLK", &[]);
                if row == self.chip.rows_pci_ce_split.0 || row == self.chip.rows_pci_ce_split.1 {
                    self.die.add_tile((col, row), "PCI_CE_SPLIT", &[]);
                } else {
                    self.die.add_tile((col, row), "PCI_CE_TRUNK_BUF", &[]);
                    if row != self.chip.row_clk() {
                        self.die.add_tile((col, row), "PCI_CE_V_BUF", &[]);
                    }
                }
            }

            if row == self.chip.row_bio_outer() || row == self.chip.row_tio_outer() {
                self.die.add_tile((col, row), "PCI_CE_H_BUF", &[]);
            }
        }
    }

    fn fill_rio(&mut self) {
        let col = self.chip.col_rio();
        for (row, &rd) in self.chip.rows.iter().rev() {
            if rd.rio {
                self.fill_ioi((col, row));
                self.die.add_tile((col, row), "IOB", &[]);
            } else {
                self.die.add_tile((col, row), "INTF", &[(col, row)]);
                if row == self.chip.row_bio_outer() {
                    self.die
                        .add_tile((col, row), "LR", &[(col, row), (col, row + 1)]);
                } else if row == self.chip.row_tio_inner() {
                    self.die
                        .add_tile((col, row), "UR", &[(col, row), (col, row + 1)]);
                }
            }

            if row == self.chip.row_clk() - 2
                || row == self.chip.row_clk() - 1
                || row == self.chip.row_clk() + 2
                || row == self.chip.row_clk() + 3
            {
                self.die.add_tile((col, row), "CLKPIN_BUF", &[]);
            }
            self.die.fill_conn_term((col, row), "TERM.E");

            if row.to_idx() % 16 == 8 {
                self.die.add_tile((col, row), "LRIOI_CLK", &[]);
                if row == self.chip.rows_pci_ce_split.0 || row == self.chip.rows_pci_ce_split.1 {
                    self.die.add_tile((col, row), "PCI_CE_SPLIT", &[]);
                } else {
                    self.die.add_tile((col, row), "PCI_CE_TRUNK_BUF", &[]);
                    if row != self.chip.row_clk() && !(self.chip.has_encrypt && row.to_idx() == 8) {
                        self.die.add_tile((col, row), "PCI_CE_V_BUF", &[]);
                    }
                }
            }

            if row == self.chip.row_bio_outer() || row == self.chip.row_tio_outer() {
                self.die.add_tile((col, row), "PCI_CE_H_BUF", &[]);
            }
        }
    }

    fn fill_tio(&mut self) {
        for (col, &cd) in &self.chip.columns {
            if cd.tio == ColumnIoKind::None {
                continue;
            }
            self.site_holes.push(Rect {
                col_l: col,
                col_r: col + 1,
                row_b: self.chip.row_tio_inner(),
                row_t: self.chip.row_tio_inner() + 2,
            });
            for (row, unused) in [
                (self.chip.row_tio_outer(), cd.tio == ColumnIoKind::Inner),
                (self.chip.row_tio_inner(), cd.tio == ColumnIoKind::Outer),
            ] {
                self.fill_ioi((col, row));
                if !unused {
                    self.die.add_tile((col, row), "IOB", &[]);
                }
            }
            let row = self.chip.row_tio_outer();
            let is_clk = col == self.chip.col_clk || col == self.chip.col_clk + 1;
            self.die.add_tile((col, row), "BTIOI_CLK", &[]);
            if is_clk {
                self.die.add_tile((col, row - 1), "CLKPIN_BUF", &[]);
                self.die.add_tile((col, row), "CLKPIN_BUF", &[]);
            }
        }
    }

    fn fill_bio(&mut self) {
        for (col, &cd) in self.chip.columns.iter().rev() {
            if cd.bio == ColumnIoKind::None {
                continue;
            }
            self.site_holes.push(Rect {
                col_l: col,
                col_r: col + 1,
                row_b: self.chip.row_bio_outer(),
                row_t: self.chip.row_bio_outer() + 2,
            });
            for (row, unused) in [
                (self.chip.row_bio_outer(), cd.bio == ColumnIoKind::Inner),
                (self.chip.row_bio_inner(), cd.bio == ColumnIoKind::Outer),
            ] {
                self.fill_ioi((col, row));
                if !unused {
                    self.die.add_tile((col, row), "IOB", &[]);
                }
            }
            let row = self.chip.row_bio_outer();
            let is_clk = col == self.chip.col_clk || col == self.chip.col_clk + 1;
            self.die.add_tile((col, row), "BTIOI_CLK", &[]);
            if is_clk {
                self.die.add_tile((col, row), "CLKPIN_BUF", &[]);
                self.die.add_tile((col, row + 1), "CLKPIN_BUF", &[]);
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
                let mut crds = vec![];
                for dy in 0..12 {
                    crds.push((col, row + dy));
                }
                for urow in mcb.row_mui {
                    for dy in 0..2 {
                        crds.push((col, urow + dy));
                    }
                }
                self.die.add_tile((col, row), "MCB", &crds);
            }
        }
    }

    fn fill_pcilogic(&mut self) {
        let row = self.chip.row_clk();

        let col = self.chip.col_lio();
        self.die.add_tile((col, row), "PCILOGICSE", &[(col, row)]);

        let col = self.chip.col_rio();
        self.die.add_tile((col, row), "PCILOGICSE", &[(col, row)]);
    }

    fn fill_spine(&mut self) {
        let col = self.chip.col_clk;

        let row = self.chip.row_clk();
        self.site_holes.push(Rect {
            col_l: col,
            col_r: col + 1,
            row_b: row,
            row_t: row + 1,
        });
        self.die.add_tile((col, row), "INTF", &[(col, row)]);
        self.die.add_tile((col, row), "CLKC", &[(col, row)]);

        for row in [self.chip.rows_hclkbuf.0, self.chip.rows_hclkbuf.1] {
            self.die.add_tile((col, row), "HCLK_V_MIDBUF", &[]);
        }

        for row in [self.chip.rows_midbuf.0, self.chip.rows_midbuf.1] {
            self.die.add_tile((col, row), "CKPIN_V_MIDBUF", &[]);
        }

        {
            let row = self.chip.row_bio_outer();
            self.die
                .add_tile((col, row), "REG_B", &[(col + 1, row + 1)]);
        }

        {
            let row = self.chip.row_tio_outer();
            self.die.add_tile((col, row), "REG_T", &[(col + 1, row)]);
        }

        for row in self.die.rows() {
            if row.to_idx() % 16 == 8 {
                self.die.add_tile((col, row), "HCLK_ROW", &[]);
            }
        }

        for col in [self.chip.cols_reg_buf.0, self.chip.cols_reg_buf.1] {
            let row = self.chip.row_clk();
            self.die.add_tile((col, row), "CKPIN_H_MIDBUF", &[]);
        }

        {
            let col = self.chip.col_lio();
            self.die
                .add_tile((col, row), "REG_L", &[(col, row), (col, row + 1)]);
        }

        {
            let col = self.chip.col_rio();
            self.die
                .add_tile((col, row), "REG_R", &[(col, row), (col, row + 1)]);
        }
    }

    fn fill_cmts(&mut self) {
        let col = self.chip.col_clk;

        for (br, kind) in self.chip.get_dcms() {
            let buf_kind = match kind {
                DcmKind::Bot => "DCM_BUFPLL_BUF_S",
                DcmKind::BotMid => "DCM_BUFPLL_BUF_S_MID",
                DcmKind::Top => "DCM_BUFPLL_BUF_N",
                DcmKind::TopMid => "DCM_BUFPLL_BUF_N_MID",
            };
            self.site_holes.push(Rect {
                col_l: col,
                col_r: col + 1,
                row_b: br - 1,
                row_t: br + 1,
            });
            for row in [br - 1, br] {
                let tile = &mut self.die[(col, row)];
                let node = &mut tile.tiles[tslots::INT];
                node.class = self.db.get_tile_class("INT.IOI");
                self.die.add_tile((col, row), "INTF.CMT.IOI", &[(col, row)]);
            }
            self.die
                .add_tile((col, br), "CMT_DCM", &[(col, br - 1), (col, br)]);
            self.die.add_tile((col, br), buf_kind, &[]);
        }

        for (br, kind) in self.chip.get_plls() {
            let out = match kind {
                PllKind::BotOut0 => "PLL_BUFPLL_OUT0",
                PllKind::BotOut1 => "PLL_BUFPLL_OUT1",
                PllKind::BotNoOut => "PLL_BUFPLL_B",
                PllKind::TopOut0 => "PLL_BUFPLL_OUT0",
                PllKind::TopOut1 => "PLL_BUFPLL_OUT1",
                PllKind::TopNoOut => "PLL_BUFPLL_T",
            };
            self.site_holes.push(Rect {
                col_l: col,
                col_r: col + 1,
                row_b: br - 1,
                row_t: br + 1,
            });
            let row: RowId = br - 1;
            self.die.add_tile((col, row), "INTF.CMT", &[(col, row)]);
            let row = br;
            let tile = &mut self.die[(col, row)];
            let node = &mut tile.tiles[tslots::INT];
            node.class = self.db.get_tile_class("INT.IOI");
            self.die.add_tile((col, row), "INTF.CMT.IOI", &[(col, row)]);

            self.die
                .add_tile((col, br), "CMT_PLL", &[(col, br - 1), (col, br)]);
            self.die.add_tile((col, br), out, &[]);
        }
    }

    fn fill_gts_holes(&mut self) {
        if let Gts::Single(bc) | Gts::Double(bc, _) | Gts::Quad(bc, _) = self.chip.gts {
            let row_gt_mid = self.chip.row_top() - 8;
            let row_gt_bot = row_gt_mid - 8;
            let row_pcie_bot = row_gt_bot - 16;
            let col_l = bc - 7;
            let col_r = bc + 5;
            for dy in 0..8 {
                let row = row_gt_mid + dy;
                self.die.fill_conn_term((col_l, row), "TERM.E");
                self.die.fill_conn_term((col_r, row), "TERM.W");
            }
            let col_l = bc - 5;
            let col_r = bc + 3;
            for dy in 0..8 {
                let row = row_gt_bot + dy;
                self.fill_intf_rterm((col_l, row));
                self.fill_intf_lterm((col_r, row));
            }

            let col_l = bc - 2;
            let col_r = bc + 2;
            for dy in 0..16 {
                let row = row_pcie_bot + dy;
                self.fill_intf_rterm((col_l, row));
                self.fill_intf_lterm((col_r, row));
            }
        }
        if let Gts::Double(_, bc) | Gts::Quad(_, bc) = self.chip.gts {
            let row_gt_mid = self.chip.row_top() - 8;
            let row_gt_bot = row_gt_mid - 8;
            let col_l = bc - 5;
            let col_r = bc + 7;
            for dy in 0..8 {
                let row = row_gt_mid + dy;
                self.die.fill_conn_term((col_l, row), "TERM.E");
                self.die.fill_conn_term((col_r, row), "TERM.W");
            }
            let col_l = bc - 3;
            let col_r = bc + 6;
            for dy in 0..8 {
                let row = row_gt_bot + dy;
                self.fill_intf_rterm((col_l, row));
                self.fill_intf_lterm((col_r, row));
            }
        }
        if let Gts::Quad(bcl, bcr) = self.chip.gts {
            let row_gt_bot = RowId::from_idx(0);
            let row_gt_mid = RowId::from_idx(8);
            let col_l = bcl - 7;
            let col_r = bcl + 5;
            for dy in 0..8 {
                let row = row_gt_bot + dy;
                self.die.fill_conn_term((col_l, row), "TERM.E");
                self.die.fill_conn_term((col_r, row), "TERM.W");
            }
            let col_l = bcl - 5;
            let col_r = bcl + 3;
            for dy in 0..8 {
                let row = row_gt_mid + dy;
                self.fill_intf_rterm((col_l, row));
                self.fill_intf_lterm((col_r, row));
            }

            let col_l = bcr - 5;
            let col_r = bcr + 7;
            for dy in 0..8 {
                let row = row_gt_bot + dy;
                self.die.fill_conn_term((col_l, row), "TERM.E");
                self.die.fill_conn_term((col_r, row), "TERM.W");
            }
            let col_l = bcr - 3;
            let col_r = bcr + 6;
            for dy in 0..8 {
                let row = row_gt_mid + dy;
                self.fill_intf_rterm((col_l, row));
                self.fill_intf_lterm((col_r, row));
            }
        }
    }

    fn fill_gts(&mut self) {
        if self.disabled.contains(&DisabledPart::Gtp) {
            return;
        }
        match self.chip.gts {
            Gts::Single(bc) | Gts::Double(bc, _) | Gts::Quad(bc, _) => {
                let row_gt_mid: RowId = self.chip.row_top() - 8;
                let row_gt_bot: RowId = row_gt_mid - 8;
                let row_pcie_bot: RowId = row_gt_bot - 16;

                let col_l = bc - 5;
                let col_r = bc + 3;
                let mut crd = vec![];
                for dy in 0..8 {
                    crd.push((col_l, row_gt_bot + dy));
                }
                for dy in 0..8 {
                    crd.push((col_r, row_gt_bot + dy));
                }
                self.die
                    .add_tile((bc, self.chip.row_tio_outer()), "GTP", &crd);

                let col_l = bc - 2;
                let col_r = bc + 2;
                let mut crd = vec![];
                for dy in 0..16 {
                    crd.push((col_l, row_pcie_bot + dy));
                }
                for dy in 0..16 {
                    crd.push((col_r, row_pcie_bot + dy));
                }
                self.die.add_tile(crd[0], "PCIE", &crd);
            }
            _ => (),
        }
        match self.chip.gts {
            Gts::Double(_, bc) | Gts::Quad(_, bc) => {
                let row_gt_mid: RowId = self.chip.row_top() - 8;
                let row_gt_bot: RowId = row_gt_mid - 8;

                let col_l = bc - 3;
                let col_r = bc + 6;
                let mut crd = vec![];
                for dy in 0..8 {
                    crd.push((col_l, row_gt_bot + dy));
                }
                for dy in 0..8 {
                    crd.push((col_r, row_gt_bot + dy));
                }
                self.die
                    .add_tile((bc, self.chip.row_tio_outer()), "GTP", &crd);
            }
            _ => (),
        }
        if let Gts::Quad(bcl, bcr) = self.chip.gts {
            let row_gt_mid = RowId::from_idx(8);

            let col_l = bcl - 5;
            let col_r = bcl + 3;
            let mut crd = vec![];
            for dy in 0..8 {
                crd.push((col_l, row_gt_mid + dy));
            }
            for dy in 0..8 {
                crd.push((col_r, row_gt_mid + dy));
            }
            self.die
                .add_tile((bcl, self.chip.row_bio_outer()), "GTP", &crd);

            let col_l = bcr - 3;
            let col_r = bcr + 6;
            let mut crd = vec![];
            for dy in 0..8 {
                crd.push((col_l, row_gt_mid + dy));
            }
            for dy in 0..8 {
                crd.push((col_r, row_gt_mid + dy));
            }
            self.die
                .add_tile((bcr, self.chip.row_bio_outer()), "GTP", &crd);
        }
    }

    fn fill_btterm(&mut self) {
        for (col, &cd) in &self.chip.columns {
            let mut row_b = self.chip.row_bio_outer();
            while self.is_int_hole(col, row_b) {
                row_b += 1;
            }
            if cd.kind != ColumnKind::Bram {
                self.die.fill_conn_term((col, row_b), "TERM.S");
            }

            let mut row_t = self.chip.row_tio_outer();
            while self.is_int_hole(col, row_t) {
                row_t -= 1;
            }
            self.die.fill_conn_term((col, row_t), "TERM.N");
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
            for row in self.die.rows() {
                if self.is_site_hole(col, row) {
                    continue;
                }
                let kind = if cd.kind == ColumnKind::CleXM {
                    "CLEXM"
                } else {
                    "CLEXL"
                };
                self.die.add_tile((col, row), kind, &[(col, row)]);
            }
        }
    }

    fn fill_bram(&mut self) {
        for (col, &cd) in &self.chip.columns {
            if cd.kind != ColumnKind::Bram {
                continue;
            }
            for row in self.die.rows() {
                if row.to_idx() % 4 != 0 {
                    continue;
                }
                let reg = self.chip.row_to_reg(row);
                if self.disabled.contains(&DisabledPart::BramRegion(col, reg)) {
                    continue;
                }
                if self.is_site_hole(col, row) {
                    continue;
                }
                self.die.add_tile(
                    (col, row),
                    "BRAM",
                    &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                );
            }

            let row = self.chip.row_bio_outer();
            self.die.add_tile((col, row), "PCI_CE_H_BUF", &[]);

            let row = self.chip.row_tio_outer();
            self.die.add_tile((col, row), "PCI_CE_H_BUF", &[]);
        }
    }

    fn fill_dsp(&mut self) {
        for (col, &cd) in &self.chip.columns {
            if !matches!(cd.kind, ColumnKind::Dsp | ColumnKind::DspPlus) {
                continue;
            }
            for row in self.die.rows() {
                if row.to_idx() % 4 != 0 {
                    continue;
                }
                let reg = self.chip.row_to_reg(row);
                if self.disabled.contains(&DisabledPart::DspRegion(col, reg)) {
                    continue;
                }
                if cd.kind == ColumnKind::DspPlus {
                    if row.to_idx() >= self.chip.rows.len() - 16 {
                        continue;
                    }
                    if matches!(self.chip.gts, Gts::Quad(_, _)) && row.to_idx() < 16 {
                        continue;
                    }
                }
                if self.is_site_hole(col, row) {
                    continue;
                }
                self.die.add_tile(
                    (col, row),
                    "DSP",
                    &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                );
            }

            let row = self.chip.row_bio_outer();
            self.die.add_tile((col, row), "PCI_CE_H_BUF", &[]);

            let row = self.chip.row_tio_outer();
            self.die.add_tile((col, row), "PCI_CE_H_BUF", &[]);
        }
    }

    fn fill_hclk_fold(&mut self) {
        if let Some((col_l, col_r)) = self.chip.cols_clk_fold {
            for col in [col_l, col_r] {
                for row in self.die.rows() {
                    if row.to_idx() % 16 != 8 {
                        continue;
                    }
                    self.die.add_tile((col, row), "HCLK_H_MIDBUF", &[]);
                }
            }
        }
    }

    fn fill_hclk(&mut self) {
        for col in self.die.cols() {
            for row in self.die.rows() {
                let crow = if row.to_idx() % 16 < 8 {
                    self.chip.row_hclk(row) - 1
                } else {
                    self.chip.row_hclk(row)
                };
                let hcol = if col <= self.chip.col_clk {
                    self.chip.col_clk
                } else {
                    self.chip.col_clk + 1
                };
                self.die[(col, row)].region_root[REGION_HCLK] = (hcol, crow);
                self.die[(col, row)].region_root[REGION_LEAF] = (col, crow);

                if row.to_idx() % 16 == 8 {
                    if self.is_int_hole(col, row) && self.is_int_hole(col, row - 1) {
                        continue;
                    }
                    self.die
                        .add_tile((col, row), "HCLK", &[(col, row - 1), (col, row)]);
                }
            }
        }
    }

    fn fill_ioi(&mut self, crd: Coord) {
        let tile = &mut self.die[crd];
        let node = &mut tile.tiles[tslots::INT];
        node.class = self.db.get_tile_class("INT.IOI");
        self.die.add_tile(crd, "INTF.IOI", &[crd]);
        let kind = if crd.0 == self.chip.col_lio() || crd.0 == self.chip.col_rio() {
            "IOI.LR"
        } else {
            "IOI.BT"
        };
        self.die.add_tile(crd, kind, &[crd]);
    }

    fn fill_intf_rterm(&mut self, crd: Coord) {
        self.die.fill_conn_term(crd, "TERM.E");
        self.die.add_tile(crd, "INTF", &[crd]);
    }

    fn fill_intf_lterm(&mut self, crd: Coord) {
        self.die.fill_conn_term(crd, "TERM.W");
        self.die.add_tile(crd, "INTF", &[crd]);
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
        for row in self.die.rows() {
            if row == self.chip.row_clk() {
                self.reg_frame[Dir::E] = self.iob_frame_len;
                self.iob_frame_len += 384;
            }
            if self.chip.rows[row].rio {
                self.iob_frame
                    .insert((self.chip.col_rio(), row), self.iob_frame_len);
                self.iob_frame_len += 128;
            }
        }
        for col in self.die.cols().rev() {
            if self.chip.columns[col].kind == ColumnKind::CleClk {
                self.reg_frame[Dir::N] = self.iob_frame_len;
                self.iob_frame_len += 384;
            }
            if matches!(
                self.chip.columns[col].tio,
                ColumnIoKind::Inner | ColumnIoKind::Both
            ) {
                self.iob_frame
                    .insert((col, self.chip.row_tio_inner()), self.iob_frame_len);
                self.iob_frame_len += 128;
            }
            if matches!(
                self.chip.columns[col].tio,
                ColumnIoKind::Outer | ColumnIoKind::Both
            ) {
                self.iob_frame
                    .insert((col, self.chip.row_tio_outer()), self.iob_frame_len);
                self.iob_frame_len += 128;
            }
        }
        for row in self.die.rows().rev() {
            if self.chip.rows[row].lio {
                self.iob_frame
                    .insert((self.chip.col_lio(), row), self.iob_frame_len);
                self.iob_frame_len += 128;
            }
            if row == self.chip.row_clk() {
                self.reg_frame[Dir::W] = self.iob_frame_len;
                self.iob_frame_len += 384;
            }
        }
        for col in self.die.cols() {
            if matches!(
                self.chip.columns[col].bio,
                ColumnIoKind::Inner | ColumnIoKind::Both
            ) {
                self.iob_frame
                    .insert((col, self.chip.row_bio_inner()), self.iob_frame_len);
                self.iob_frame_len += 128;
            }
            if matches!(
                self.chip.columns[col].bio,
                ColumnIoKind::Outer | ColumnIoKind::Both
            ) {
                self.iob_frame
                    .insert((col, self.chip.row_bio_outer()), self.iob_frame_len);
                self.iob_frame_len += 128;
            }
            if self.chip.columns[col].kind == ColumnKind::CleClk {
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
        let (_, die) = egrid.add_die(self.columns.len(), self.rows.len());
        let disabled = disabled.clone();

        let mut expander = Expander {
            chip: self,
            db,
            disabled: &disabled,
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
        expander.die.fill_main_passes();
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
            die_order: vec![expander.die.die],
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
