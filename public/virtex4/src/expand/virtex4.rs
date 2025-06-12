use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::grid::{
    CellCoord, ColId, DieId, ExpandedDieRefMut, ExpandedGrid, Rect, RowId, TileIobId,
};
use prjcombine_xilinx_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo, FrameMaskMode,
};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::bond::SharedCfgPad;
use crate::chip::{CfgRowKind, Chip, ColumnKind, DisabledPart};
use crate::expanded::{DieFrameGeom, ExpandedDevice, IoCoord, REGION_HCLK, REGION_LEAF};
use crate::gtz::GtzDb;
use bimap::BiHashMap;
use std::collections::BTreeSet;

struct Expander<'a, 'b> {
    chip: &'b Chip,
    die: ExpandedDieRefMut<'a, 'b>,
    site_holes: Vec<Rect>,
    int_holes: Vec<Rect>,
    frame_info: Vec<FrameInfo>,
    frames: DieFrameGeom,
    col_cfg: ColId,
    col_lio: Option<ColId>,
    col_rio: Option<ColId>,
    row_dcmiob: Option<RowId>,
    row_iobdcm: Option<RowId>,
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
                col_r: bc + 8,
                row_b: br + 1,
                row_t: br + 23,
            });
            self.site_holes.push(Rect {
                col_l: bc,
                col_r: bc + 9,
                row_b: br,
                row_t: br + 24,
            });
        }
    }

    fn fill_int(&mut self) {
        for col in self.die.cols() {
            for row in self.die.rows() {
                if self.is_int_hole(col, row) {
                    continue;
                }
                self.die.add_tile((col, row), "INT", &[(col, row)]);
            }
        }
    }

    fn fill_lrio(&mut self) {
        for col in [self.col_lio.unwrap(), self.col_rio.unwrap()] {
            for row in self.die.rows() {
                let cell = CellCoord::new(self.die.die, col, row);
                self.die.add_tile((col, row), "INTF", &[(col, row)]);
                self.die.add_tile((col, row), "IO", &[(col, row)]);
                let crd_n = IoCoord {
                    cell,
                    iob: TileIobId::from_idx(0),
                };
                let crd_p = IoCoord {
                    cell,
                    iob: TileIobId::from_idx(1),
                };
                self.io.extend([crd_n, crd_p]);

                if row.to_idx() % 32 == 8 {
                    self.die.add_tile(
                        (col, row),
                        "HCLK_IOIS_DCI",
                        &[(col, row - 2), (col, row - 1), (col, row)],
                    );
                } else if row.to_idx() % 32 == 24 {
                    self.die.add_tile(
                        (col, row),
                        "HCLK_IOIS_LVDS",
                        &[(col, row - 2), (col, row - 1), (col, row)],
                    );
                }
            }
        }
    }

    fn fill_cfg(&mut self) {
        let col = self.col_cfg;
        let row_cfg = self.chip.row_reg_bot(self.chip.reg_cfg) - 8;
        // CFG_CENTER
        {
            let row = row_cfg;
            self.site_holes.push(Rect {
                col_l: col,
                col_r: col + 1,
                row_b: row,
                row_t: row + 16,
            });
            let crds: [_; 16] = core::array::from_fn(|i| (col, row + i));
            for crd in crds {
                self.die.add_tile(crd, "INTF", &[crd]);
            }
            self.die.add_tile(crds[0], "CFG", &crds);
        }
        let mut row_dcmiob = RowId::from_idx(0);
        let mut row_iobdcm = RowId::from_idx(self.die.rows().len());
        for &(row, kind) in &self.chip.rows_cfg {
            match kind {
                CfgRowKind::Sysmon => {
                    self.site_holes.push(Rect {
                        col_l: col,
                        col_r: col + 1,
                        row_b: row,
                        row_t: row + 8,
                    });
                    let crds: [_; 8] = core::array::from_fn(|i| (col, row + i));
                    for crd in crds {
                        self.die.add_tile(crd, "INTF", &[crd]);
                    }
                    self.die.add_tile(crds[0], "SYSMON", &crds);
                    if row < row_cfg {
                        row_dcmiob = row_dcmiob.max(row + 8);
                    } else {
                        row_iobdcm = row_iobdcm.min(row);
                    }
                }
                CfgRowKind::Dcm | CfgRowKind::Ccm => {
                    self.site_holes.push(Rect {
                        col_l: col,
                        col_r: col + 1,
                        row_b: row,
                        row_t: row + 4,
                    });
                    let crds: [_; 4] = core::array::from_fn(|i| (col, row + i));
                    for crd in crds {
                        self.die.add_tile(crd, "INTF", &[crd]);
                    }
                    self.die.add_tile(
                        (col, row),
                        if kind == CfgRowKind::Ccm {
                            "CCM"
                        } else {
                            "DCM"
                        },
                        &crds,
                    );
                    if row.to_idx() % 8 == 0 {
                        let bt = if row < row_cfg { 'B' } else { 'T' };
                        self.die.add_tile((col, row), &format!("CLK_DCM_{bt}"), &[]);
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
        for row in self.die.rows() {
            let cell = CellCoord::new(self.die.die, col, row);
            if !self.is_site_hole(col, row) {
                self.die.add_tile((col, row), "INTF", &[(col, row)]);
                self.die.add_tile((col, row), "IO", &[(col, row)]);
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

            if row.to_idx() % 16 == 8 {
                self.die.add_tile((col, row), "CLK_HROW", &[]);

                if row < self.row_dcmiob.unwrap() || row > self.row_iobdcm.unwrap() {
                    self.die.add_tile((col, row), "HCLK_DCM", &[]);
                } else if row == self.row_dcmiob.unwrap() {
                    self.die
                        .add_tile((col, row), "HCLK_DCMIOB", &[(col, row), (col, row + 1)]);
                } else if row == self.row_iobdcm.unwrap() {
                    self.die
                        .add_tile((col, row), "HCLK_IOBDCM", &[(col, row - 2), (col, row - 1)]);
                } else if row == self.chip.row_bufg() + 8 {
                    self.die.add_tile(
                        (col, row),
                        "HCLK_CENTER_ABOVE_CFG",
                        &[(col, row), (col, row + 1)],
                    );
                } else {
                    self.die
                        .add_tile((col, row), "HCLK_CENTER", &[(col, row - 2), (col, row - 1)]);
                }
            }
        }

        {
            let row = self.row_dcmiob.unwrap();
            self.die.add_tile((col, row), "CLK_IOB_B", &[]);
        }
        {
            let row: RowId = self.row_iobdcm.unwrap() - 16;
            self.die.add_tile((col, row), "CLK_IOB_T", &[]);
        }
        {
            let row = self.die.rows().next().unwrap();
            self.die.add_tile((col, row), "CLK_TERM", &[]);
        }
        {
            let row = self.die.rows().next_back().unwrap();
            self.die.add_tile((col, row), "CLK_TERM", &[]);
        }
    }

    fn fill_ppc(&mut self) {
        for &(bc, br) in &self.chip.holes_ppc {
            let col_l = bc;
            let col_r = bc + 8;
            for dy in 0..22 {
                let row = br + 1 + dy;
                self.die
                    .fill_conn_pair((col_l, row), (col_r, row), "PPC.E", "PPC.W");
            }
            let row_b = br;
            let row_t = br + 23;
            for dx in 0..7 {
                let col = bc + 1 + dx;
                self.die.fill_conn_pair(
                    (col, row_b),
                    (col, row_t),
                    if dx < 5 { "PPCA.N" } else { "PPCB.N" },
                    if dx < 5 { "PPCA.S" } else { "PPCB.S" },
                );
            }
            let mut crds = vec![];
            for dy in 0..24 {
                crds.push((col_l, br + dy));
            }
            for dy in 0..24 {
                crds.push((col_r, br + dy));
            }
            for dx in 1..8 {
                crds.push((bc + dx, row_b));
            }
            for dx in 1..8 {
                crds.push((bc + dx, row_t));
            }
            for &(col, row) in &crds {
                self.die.add_tile((col, row), "INTF", &[(col, row)]);
            }
            self.die.add_tile((bc, br), "PPC", &crds);
        }
    }

    fn fill_term(&mut self) {
        let row_b = self.die.rows().next().unwrap();
        let row_t = self.die.rows().next_back().unwrap();
        for col in self.die.cols() {
            self.die.fill_conn_term((col, row_b), "TERM.S");
            self.die.fill_conn_term((col, row_t), "TERM.N");
        }
        let col_l = self.die.cols().next().unwrap();
        let col_r = self.die.cols().next_back().unwrap();
        for row in self.die.rows() {
            self.die.fill_conn_term((col_l, row), "TERM.W");
            self.die.fill_conn_term((col_r, row), "TERM.E");
        }

        let term_s = "BRKH.S";
        let term_n = "BRKH.N";
        for col in self.die.cols() {
            for row in self.die.rows() {
                if row.to_idx() % 8 != 0 || row.to_idx() == 0 {
                    continue;
                }
                if self.is_int_hole(col, row) {
                    continue;
                }
                self.die
                    .fill_conn_pair((col, row - 1), (col, row), term_n, term_s);
            }
        }

        let term_w = "CLB_BUFFER.W";
        let term_e = "CLB_BUFFER.E";
        for (col, &cd) in &self.chip.columns {
            if !matches!(cd, ColumnKind::Io | ColumnKind::Cfg) || col == col_l || col == col_r {
                continue;
            }
            for row in self.die.rows() {
                self.die
                    .fill_conn_pair((col, row), (col + 1, row), term_e, term_w);
            }
        }
    }

    fn fill_clb(&mut self) {
        for (col, &cd) in &self.chip.columns {
            if cd != ColumnKind::ClbLM {
                continue;
            }
            for row in self.die.rows() {
                if self.is_site_hole(col, row) {
                    continue;
                }
                self.die.add_tile((col, row), "CLB", &[(col, row)]);
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
                if row.to_idx() % 4 != 0 {
                    continue;
                }
                if self.is_site_hole(col, row) {
                    continue;
                }
                for dy in 0..4 {
                    self.die
                        .add_tile((col, row + dy), "INTF", &[(col, row + dy)]);
                }
                self.die.add_tile(
                    (col, row),
                    kind,
                    &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                );
            }
        }
    }

    fn fill_gt(&mut self) {
        for (col, &cd) in &self.chip.columns {
            if cd != ColumnKind::Gt {
                continue;
            }
            for reg in self.chip.regs() {
                if reg.to_idx() % 2 != 0 {
                    continue;
                }
                let row = self.chip.row_reg_bot(reg);
                let crds: [_; 32] = core::array::from_fn(|i| (col, row + i));
                for (col, row) in crds {
                    self.die.add_tile((col, row), "INTF", &[(col, row)]);
                }
                self.die.add_tile((col, row), "MGT", &crds);
                self.die.add_tile((col, row + 8), "HCLK_MGT", &[]);
                self.die.add_tile((col, row + 24), "HCLK_MGT", &[]);
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
                if row.to_idx() % 16 == 8 {
                    if self.is_int_hole(col, row) {
                        continue;
                    }
                    self.die.add_tile((col, row), "HCLK", &[(col, row)]);
                    if col == self.chip.columns.first_id().unwrap()
                        || col == self.chip.columns.last_id().unwrap()
                    {
                        self.die.add_tile((col, row), "HCLK_TERM", &[]);
                    }
                    if self.chip.cols_vbrk.contains(&col) {
                        let rcol = if col < self.col_cfg { col } else { col - 1 };
                        self.die.add_tile((rcol, row), "HCLK_MGT_REPEATER", &[]);
                    }
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
    let (_, die) = egrid.add_die(chip.columns.len(), chip.regs * 16);
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
    expander.die.fill_main_passes();
    expander.fill_clb();
    expander.fill_bram_dsp();
    expander.fill_gt();
    expander.fill_hclk();
    expander.fill_frame_info();

    let int_holes = expander.int_holes;
    let site_holes = expander.site_holes;
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
        die_order: vec![expander.die.die],
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
        int_holes: [int_holes].into_iter().collect(),
        site_holes: [site_holes].into_iter().collect(),
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
