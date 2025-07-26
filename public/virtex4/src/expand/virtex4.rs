use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::grid::{
    CellCoord, ColId, DieId, ExpandedGrid, Rect, RowId, TileIobId,
};
use prjcombine_xilinx_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo, FrameMaskMode,
};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::bond::SharedCfgPad;
use crate::chip::{CfgRowKind, Chip, ColumnKind, DisabledPart};
use crate::expanded::{DieFrameGeom, ExpandedDevice, IoCoord};
use crate::gtz::GtzDb;
use crate::regions;
use bimap::BiHashMap;
use std::collections::BTreeSet;

struct Expander<'a, 'b> {
    chip: &'b Chip,
    egrid: &'a mut ExpandedGrid<'b>,
    die: DieId,
    site_holes: &'a mut Vec<Rect>,
    int_holes: &'a mut Vec<Rect>,
    frame_info: Vec<FrameInfo>,
    frames: DieFrameGeom,
    col_cfg: ColId,
    col_lio: Option<ColId>,
    col_rio: Option<ColId>,
    row_dcmiob: Option<RowId>,
    row_iobdcm: Option<RowId>,
    io: Vec<IoCoord>,
    gt: Vec<CellCoord>,
}

impl Expander<'_, '_> {
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
        for &(bc, br) in &self.chip.holes_ppc {
            let cell = CellCoord::new(self.die, bc, br);
            self.int_holes.push(cell.delta(1, 1).rect(7, 22));
            self.site_holes.push(cell.rect(9, 24));
        }
    }

    fn fill_int(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            if self.is_int_hole(cell) {
                continue;
            }
            self.egrid.add_tile_single(cell, "INT");
        }
    }

    fn fill_lrio(&mut self) {
        for col in [self.col_lio.unwrap(), self.col_rio.unwrap()] {
            for cell in self.egrid.column(self.die, col) {
                self.egrid.add_tile_single(cell, "INTF");
                self.egrid.add_tile_single(cell, "IO");
                let crd_n = IoCoord {
                    cell,
                    iob: TileIobId::from_idx(0),
                };
                let crd_p = IoCoord {
                    cell,
                    iob: TileIobId::from_idx(1),
                };
                self.io.extend([crd_n, crd_p]);

                if cell.row.to_idx() % 32 == 8 {
                    self.egrid.add_tile_sn(cell, "HCLK_IOIS_DCI", 2, 3);
                } else if cell.row.to_idx() % 32 == 24 {
                    self.egrid.add_tile_sn(cell, "HCLK_IOIS_LVDS", 2, 3);
                }
            }
        }
    }

    fn fill_cfg(&mut self) {
        let col = self.col_cfg;
        let row_cfg = self.chip.row_reg_bot(self.chip.reg_cfg) - 8;
        // CFG_CENTER
        {
            let cell = CellCoord::new(self.die, col, row_cfg);
            self.site_holes.push(cell.rect(1, 16));
            let tcells = cell.cells_n_const::<16>();
            for cell in tcells {
                self.egrid.add_tile_single(cell, "INTF");
            }
            self.egrid.add_tile(cell, "CFG", &tcells);
        }
        let mut row_dcmiob = RowId::from_idx(0);
        let mut row_iobdcm = RowId::from_idx(self.chip.rows().len());
        for &(row, kind) in &self.chip.rows_cfg {
            let cell = CellCoord::new(self.die, col, row);
            match kind {
                CfgRowKind::Sysmon => {
                    self.site_holes.push(cell.rect(1, 8));
                    let tcells = cell.cells_n_const::<8>();
                    for cell in tcells {
                        self.egrid.add_tile_single(cell, "INTF");
                    }
                    self.egrid.add_tile(cell, "SYSMON", &tcells);
                    if row < row_cfg {
                        row_dcmiob = row_dcmiob.max(row + 8);
                    } else {
                        row_iobdcm = row_iobdcm.min(row);
                    }
                }
                CfgRowKind::Dcm | CfgRowKind::Ccm => {
                    self.site_holes.push(cell.rect(1, 4));
                    let tcells = cell.cells_n_const::<4>();
                    for cell in tcells {
                        self.egrid.add_tile_single(cell, "INTF");
                    }
                    self.egrid.add_tile(
                        cell,
                        if kind == CfgRowKind::Ccm {
                            "CCM"
                        } else {
                            "DCM"
                        },
                        &tcells,
                    );
                    if row.to_idx().is_multiple_of(8) {
                        let bt = if row < row_cfg { 'B' } else { 'T' };
                        self.egrid.add_tile(cell, &format!("CLK_DCM_{bt}"), &[]);
                    }
                    if row < row_cfg {
                        row_dcmiob = row_dcmiob.max(row + 4);
                    } else {
                        row_iobdcm = row_iobdcm.min(row);
                    }
                }
            }
        }
        self.row_dcmiob = Some(row_dcmiob);
        self.row_iobdcm = Some(row_iobdcm);
    }

    fn fill_cio(&mut self) {
        let col = self.col_cfg;
        for cell in self.egrid.column(self.die, self.col_cfg) {
            if !self.is_site_hole(cell) {
                self.egrid.add_tile_single(cell, "INTF");
                self.egrid.add_tile_single(cell, "IO");
                let crd_n = IoCoord {
                    cell,
                    iob: TileIobId::from_idx(0),
                };
                let crd_p = IoCoord {
                    cell,
                    iob: TileIobId::from_idx(1),
                };
                self.io.extend([crd_n, crd_p]);
            }

            if cell.row.to_idx() % 16 == 8 {
                self.egrid.add_tile(cell, "CLK_HROW", &[]);

                if cell.row < self.row_dcmiob.unwrap() || cell.row > self.row_iobdcm.unwrap() {
                    self.egrid.add_tile(cell, "HCLK_DCM", &[]);
                } else if cell.row == self.row_dcmiob.unwrap() {
                    self.egrid.add_tile_n(cell, "HCLK_DCMIOB", 2);
                } else if cell.row == self.row_iobdcm.unwrap() {
                    self.egrid.add_tile_sn(cell, "HCLK_IOBDCM", 2, 2);
                } else if cell.row == self.chip.row_bufg() + 8 {
                    self.egrid.add_tile_n(cell, "HCLK_CENTER_ABOVE_CFG", 2);
                } else {
                    self.egrid.add_tile_sn(cell, "HCLK_CENTER", 2, 2);
                }
            }
        }

        {
            let row = self.row_dcmiob.unwrap();
            let cell = CellCoord::new(self.die, col, row);
            self.egrid.add_tile(cell, "CLK_IOB_B", &[]);
        }
        {
            let row: RowId = self.row_iobdcm.unwrap() - 16;
            let cell = CellCoord::new(self.die, col, row);
            self.egrid.add_tile(cell, "CLK_IOB_T", &[]);
        }
        {
            let row = self.chip.rows().next().unwrap();
            let cell = CellCoord::new(self.die, col, row);
            self.egrid.add_tile(cell, "CLK_TERM", &[]);
        }
        {
            let row = self.chip.rows().next_back().unwrap();
            let cell = CellCoord::new(self.die, col, row);
            self.egrid.add_tile(cell, "CLK_TERM", &[]);
        }
    }

    fn fill_ppc(&mut self) {
        for &(bc, br) in &self.chip.holes_ppc {
            let cell = CellCoord::new(self.die, bc, br);
            for dy in 1..23 {
                self.egrid
                    .fill_conn_pair(cell.delta(0, dy), cell.delta(8, dy), "PPC.E", "PPC.W");
            }
            for dx in 1..8 {
                self.egrid.fill_conn_pair(
                    cell.delta(dx, 0),
                    cell.delta(dx, 23),
                    if dx < 6 { "PPCA.N" } else { "PPCB.N" },
                    if dx < 6 { "PPCA.S" } else { "PPCB.S" },
                );
            }
            let mut tcells = vec![];
            tcells.extend(cell.cells_n_const::<24>());
            tcells.extend(cell.delta(8, 0).cells_n_const::<24>());
            tcells.extend(cell.delta(1, 0).cells_e_const::<7>());
            tcells.extend(cell.delta(1, 23).cells_e_const::<7>());
            for &cell in &tcells {
                self.egrid.add_tile_single(cell, "INTF");
            }
            self.egrid.add_tile(cell, "PPC", &tcells);
        }
    }

    fn fill_term(&mut self) {
        let row_b = self.chip.rows().next().unwrap();
        for cell in self.egrid.row(self.die, row_b) {
            self.egrid.fill_conn_term(cell, "TERM.S");
        }
        let row_t = self.chip.rows().next_back().unwrap();
        for cell in self.egrid.row(self.die, row_t) {
            self.egrid.fill_conn_term(cell, "TERM.N");
        }
        let col_l = self.chip.columns.ids().next().unwrap();
        for cell in self.egrid.column(self.die, col_l) {
            self.egrid.fill_conn_term(cell, "TERM.W");
        }
        let col_r = self.chip.columns.ids().next_back().unwrap();
        for cell in self.egrid.column(self.die, col_r) {
            self.egrid.fill_conn_term(cell, "TERM.E");
        }

        let term_s = "BRKH.S";
        let term_n = "BRKH.N";
        for row in self.chip.rows() {
            if !row.to_idx().is_multiple_of(8) || row.to_idx() == 0 {
                continue;
            }
            for cell in self.egrid.row(self.die, row) {
                if self.is_int_hole(cell) {
                    continue;
                }
                self.egrid
                    .fill_conn_pair(cell.delta(0, -1), cell, term_n, term_s);
            }
        }

        let term_w = "CLB_BUFFER.W";
        let term_e = "CLB_BUFFER.E";
        for (col, &cd) in &self.chip.columns {
            if !matches!(cd, ColumnKind::Io | ColumnKind::Cfg) || col == col_l || col == col_r {
                continue;
            }
            for cell in self.egrid.column(self.die, col) {
                self.egrid
                    .fill_conn_pair(cell, cell.delta(1, 0), term_e, term_w);
            }
        }
    }

    fn fill_clb(&mut self) {
        for (col, &cd) in &self.chip.columns {
            if cd != ColumnKind::ClbLM {
                continue;
            }
            for cell in self.egrid.column(self.die, col) {
                if self.is_site_hole(cell) {
                    continue;
                }
                self.egrid.add_tile_single(cell, "CLB");
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
            for cell in self.egrid.column(self.die, col) {
                if !cell.row.to_idx().is_multiple_of(4) {
                    continue;
                }
                if self.is_site_hole(cell) {
                    continue;
                }
                let tcells = cell.cells_n_const::<4>();
                for cell in tcells {
                    self.egrid.add_tile_single(cell, "INTF");
                }
                self.egrid.add_tile(cell, kind, &tcells);
            }
        }
    }

    fn fill_gt(&mut self) {
        for (col, &cd) in &self.chip.columns {
            if cd != ColumnKind::Gt {
                continue;
            }
            for reg in self.chip.regs() {
                if !reg.to_idx().is_multiple_of(2) {
                    continue;
                }
                let row = self.chip.row_reg_bot(reg);
                let cell = CellCoord::new(self.die, col, row);
                let tcells = cell.cells_n_const::<32>();
                for cell in tcells {
                    self.egrid.add_tile_single(cell, "INTF");
                }
                self.egrid.add_tile(cell, "MGT", &tcells);
                self.egrid.add_tile(cell.delta(0, 8), "HCLK_MGT", &[]);
                self.egrid.add_tile(cell.delta(0, 24), "HCLK_MGT", &[]);
                self.gt.push(cell);
            }
        }
    }

    fn fill_hclk(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            let col_hrow = if cell.col <= self.col_cfg {
                self.col_cfg
            } else {
                self.col_cfg + 1
            };
            let crow = self.chip.row_hclk(cell.row);
            self.egrid[cell].region_root[regions::HCLK] = cell.with_cr(col_hrow, crow);
            self.egrid[cell].region_root[regions::LEAF] = cell.with_row(crow);
            if cell.row.to_idx() % 16 == 8 {
                if self.is_int_hole(cell) {
                    continue;
                }
                self.egrid.add_tile_single(cell, "HCLK");
                if cell.col == self.chip.columns.first_id().unwrap()
                    || cell.col == self.chip.columns.last_id().unwrap()
                {
                    self.egrid.add_tile(cell, "HCLK_TERM", &[]);
                }
                if self.chip.cols_vbrk.contains(&cell.col) {
                    let rcell = if cell.col < self.col_cfg {
                        cell
                    } else {
                        cell.delta(-1, 0)
                    };
                    self.egrid.add_tile(rcell, "HCLK_MGT_REPEATER", &[]);
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
            for &cd in self.chip.columns.values() {
                // Fixed later for Bram
                self.frames.col_frame[reg].push(self.frame_info.len());
                let width = match cd {
                    ColumnKind::ClbLM => 22,
                    ColumnKind::Bram => 20,
                    ColumnKind::Dsp => 21,
                    ColumnKind::Io | ColumnKind::Cfg => 30,
                    ColumnKind::Gt => 20,
                    _ => unreachable!(),
                };
                self.frames.col_width[reg].push(width as usize);
                if cd == ColumnKind::Bram {
                    continue;
                }
                for minor in 0..width {
                    let mut mask_mode = [FrameMaskMode::None; 4];
                    if cd == ColumnKind::Gt && minor == 19 {
                        mask_mode = [FrameMaskMode::DrpV4; 4];
                    }
                    if cd == ColumnKind::Cfg {
                        for &(row, kind) in &self.chip.rows_cfg {
                            if self.chip.row_to_reg(row) == reg {
                                let idx = row.to_idx() / 4 % 4;
                                match kind {
                                    CfgRowKind::Dcm => {
                                        if matches!(minor, 19 | 20) {
                                            mask_mode[idx] = FrameMaskMode::DrpV4;
                                        }
                                    }
                                    CfgRowKind::Ccm => (),
                                    CfgRowKind::Sysmon => {
                                        if matches!(minor, 19 | 20 | 21 | 24 | 25 | 26 | 27 | 28) {
                                            mask_mode[idx] = FrameMaskMode::All;
                                            mask_mode[idx + 1] = FrameMaskMode::All;
                                        }
                                    }
                                }
                            }
                        }
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
                if cd == ColumnKind::Cfg {
                    self.frames.spine_frame[reg] = self.frame_info.len();
                    for minor in 0..3 {
                        self.frame_info.push(FrameInfo {
                            addr: FrameAddr {
                                typ: 0,
                                region: (reg - self.chip.reg_cfg) as i32,
                                major,
                                minor,
                            },
                            mask_mode: [FrameMaskMode::None; 4].into_iter().collect(),
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
                self.frames.col_frame[reg][col] = self.frame_info.len();
                for minor in 0..20 {
                    let mask_mode = if minor == 19 {
                        FrameMaskMode::BramV4
                    } else {
                        FrameMaskMode::None
                    };
                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 1,
                            region: (reg - self.chip.reg_cfg) as i32,
                            major,
                            minor,
                        },
                        mask_mode: [mask_mode; 4].into_iter().collect(),
                    });
                }
                major += 1;
            }
        }
        for &reg in &regs {
            let mut major = 0;
            for (col, &cd) in &self.chip.columns {
                if cd != ColumnKind::Bram {
                    continue;
                }
                self.frames.bram_frame[reg].insert(col, self.frame_info.len());
                for minor in 0..64 {
                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 2,
                            region: (reg - self.chip.reg_cfg) as i32,
                            major,
                            minor,
                        },
                        mask_mode: [FrameMaskMode::All; 4].into_iter().collect(),
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
    assert_eq!(cols_io.len(), 2);
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
    let die = egrid.add_die(chip.columns.len(), chip.regs * 16);
    let mut int_holes = vec![];
    let mut site_holes = vec![];
    let mut expander = Expander {
        chip,
        egrid: &mut egrid,
        die,
        int_holes: &mut int_holes,
        site_holes: &mut site_holes,
        frame_info: vec![],
        frames: DieFrameGeom {
            col_frame: EntityVec::new(),
            col_width: EntityVec::new(),
            bram_frame: EntityVec::new(),
            spine_frame: EntityVec::new(),
        },
        col_lio: Some(cols_io[0]),
        col_cfg,
        col_rio: Some(cols_io[1]),
        row_dcmiob: None,
        row_iobdcm: None,
        io: vec![],
        gt: vec![],
    };

    expander.fill_holes();
    expander.fill_int();
    expander.fill_cfg();
    expander.fill_lrio();
    expander.fill_cio();
    expander.fill_ppc();
    expander.fill_term();
    expander.egrid.fill_main_passes(die);
    expander.fill_clb();
    expander.fill_bram_dsp();
    expander.fill_gt();
    expander.fill_hclk();
    expander.fill_frame_info();

    let frames = expander.frames;
    let io = expander.io;
    let gt = expander.gt;
    let row_dcmiob = expander.row_dcmiob;
    let row_iobdcm = expander.row_iobdcm;
    let die_bs_geom = DieBitstreamGeom {
        frame_len: 80 * 16 + 32,
        frame_info: expander.frame_info,
        bram_frame_len: 0,
        bram_frame_info: vec![],
        iob_frame_len: 0,
    };
    let bs_geom = BitstreamGeom {
        kind: DeviceKind::Virtex4,
        die: [die_bs_geom].into_iter().collect(),
        die_order: vec![die],
        has_gtz_bot: false,
        has_gtz_top: false,
    };

    let mut cfg_io = BiHashMap::new();
    for i in 0..16 {
        cfg_io.insert(
            SharedCfgPad::Data(i as u8),
            IoCoord {
                cell: CellCoord::new(
                    DieId::from_idx(0),
                    col_cfg,
                    chip.row_reg_bot(chip.reg_cfg) - 16 + i / 2,
                ),
                iob: TileIobId::from_idx(i & 1),
            },
        );
    }
    for i in 0..16 {
        cfg_io.insert(
            SharedCfgPad::Data(i as u8 + 16),
            IoCoord {
                cell: CellCoord::new(
                    DieId::from_idx(0),
                    col_cfg,
                    chip.row_reg_bot(chip.reg_cfg) + 8 + i / 2,
                ),
                iob: TileIobId::from_idx(i & 1),
            },
        );
    }

    egrid.finish();
    ExpandedDevice {
        kind: chip.kind,
        chips: chips.clone(),
        interposer: None,
        disabled: disabled.clone(),
        int_holes,
        site_holes,
        egrid,
        bs_geom,
        frames: [frames].into_iter().collect(),
        col_cfg,
        col_clk: col_cfg,
        col_lio: Some(cols_io[0]),
        col_rio: Some(cols_io[1]),
        col_lcio: None,
        col_rcio: None,
        col_lgt,
        col_rgt,
        col_mgt: None,
        row_dcmiob,
        row_iobdcm,
        io,
        gt,
        gtz: Default::default(),
        cfg_io,
        banklut: EntityVec::new(),
        gdb,
    }
}
