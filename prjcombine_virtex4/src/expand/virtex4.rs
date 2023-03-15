#![allow(clippy::collapsible_else_if)]
use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};
use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ColId, DieId, ExpandedDieRefMut, ExpandedGrid, Rect, RowId};
use prjcombine_virtex_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};

use crate::bond::SharedCfgPin;
use crate::expanded::{
    DieFrameGeom, ExpandedDevice, Gt, Io, IoCoord, IoDiffKind, IoVrKind, SysMon, TileIobId,
};
use crate::grid::{CfgRowKind, ColumnKind, DisabledPart, ExtraDie, Grid, GtKind, IoKind};
use bimap::BiHashMap;
use std::collections::BTreeSet;

struct Expander<'a, 'b> {
    grid: &'b Grid,
    db: &'a IntDb,
    die: ExpandedDieRefMut<'a, 'b>,
    site_holes: Vec<Rect>,
    int_holes: Vec<Rect>,
    dciylut: Vec<usize>,
    frame_info: Vec<FrameInfo>,
    frames: DieFrameGeom,
    col_cfg: ColId,
    col_lio: Option<ColId>,
    col_rio: Option<ColId>,
    row_dcmiob: Option<RowId>,
    row_iobdcm: Option<RowId>,
    io: Vec<Io>,
    gt: Vec<Gt>,
    sysmon: Vec<SysMon>,
}

impl<'a, 'b> Expander<'a, 'b> {
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

    fn fill_dciylut(&mut self) {
        let mut dciy = 0;
        for i in 0..self.grid.regs {
            self.dciylut.push(dciy);
            let row = RowId::from_idx(i * 16 + 8);
            if i % 2 == 0 || (row >= self.row_dcmiob.unwrap() && row <= self.row_iobdcm.unwrap()) {
                dciy += 1;
            }
        }
    }

    fn fill_holes(&mut self) {
        for &(bc, br) in &self.grid.holes_ppc {
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
                let x = col.to_idx();
                let y = row.to_idx();
                let node = self.die.add_xnode(
                    (col, row),
                    self.db.get_node("INT"),
                    &[&format!("INT_X{x}Y{y}")],
                    self.db.get_node_naming("INT"),
                    &[(col, row)],
                );
                node.tie_name = Some(format!("TIEOFF_X{x}Y{y}"));
            }
        }
    }

    fn fill_lrio(&mut self) {
        for (brx, biox, col) in [(0, 0, self.col_lio.unwrap()), (1, 2, self.col_rio.unwrap())] {
            for row in self.die.rows() {
                let x = col.to_idx();
                let y = row.to_idx();
                let naming = match y % 16 {
                    7 | 8 => "IOIS_LC",
                    _ => "IOIS_NC",
                };
                let l = if col.to_idx() == 0 { "_L" } else { "" };
                self.die.add_xnode(
                    (col, row),
                    self.db.get_node("INTF"),
                    &[&format!("{naming}{l}_X{x}Y{y}")],
                    self.db.get_node_naming("INTF.IOIS"),
                    &[(col, row)],
                );
                let node = self.die.add_xnode(
                    (col, row),
                    self.db.get_node("IOIS"),
                    &[&format!("{naming}{l}_X{x}Y{y}")],
                    self.db.get_node_naming(naming),
                    &[(col, row)],
                );
                node.add_bel(0, format!("ILOGIC_X{biox}Y{y}", y = 2 * y + 1));
                node.add_bel(1, format!("ILOGIC_X{biox}Y{y}", y = 2 * y));
                node.add_bel(2, format!("OLOGIC_X{biox}Y{y}", y = 2 * y + 1));
                node.add_bel(3, format!("OLOGIC_X{biox}Y{y}", y = 2 * y));
                let iob_name_p = format!("IOB_X{biox}Y{y}", y = 2 * y + 1);
                let iob_name_n = format!("IOB_X{biox}Y{y}", y = 2 * y);
                node.add_bel(4, iob_name_p.clone());
                node.add_bel(5, iob_name_n.clone());
                let lr = if col == self.col_lio.unwrap() {
                    'L'
                } else {
                    'R'
                };
                let banks: &[u32] = match (lr, self.grid.regs) {
                    ('L', 4) => &[7, 5],
                    ('L', 6) => &[7, 9, 5],
                    ('L', 8) => &[7, 11, 9, 5],
                    ('L', 10) => &[7, 11, 13, 9, 5],
                    ('L', 12) => &[7, 11, 15, 13, 9, 5],
                    ('R', 4) => &[8, 6],
                    ('R', 6) => &[8, 10, 6],
                    ('R', 8) => &[8, 12, 10, 6],
                    ('R', 10) => &[8, 12, 14, 10, 6],
                    ('R', 12) => &[8, 12, 16, 14, 10, 6],
                    _ => unreachable!(),
                };
                let bank = banks[row.to_idx() / 32];
                let biob = (row.to_idx() % 32 * 2) as u32;
                let pkgid = (64 - biob) / 2;
                let crd_p = IoCoord {
                    die: self.die.die,
                    col,
                    row,
                    iob: TileIobId::from_idx(0),
                };
                let crd_n = IoCoord {
                    die: self.die.die,
                    col,
                    row,
                    iob: TileIobId::from_idx(1),
                };
                let is_cc = matches!(row.to_idx() % 16, 7 | 8);
                let is_vref = row.to_idx() % 8 == 4;
                let is_vr = biob == 18;
                self.io.extend([
                    Io {
                        crd: crd_p,
                        name: iob_name_p,
                        bank,
                        biob: biob + 1,
                        pkgid,
                        byte: None,
                        kind: IoKind::Hpio,
                        diff: IoDiffKind::P(crd_n),
                        is_lc: is_cc,
                        is_gc: false,
                        is_srcc: is_cc,
                        is_mrcc: false,
                        is_dqs: false,
                        is_vref: false,
                        vr: if is_vr { IoVrKind::VrN } else { IoVrKind::None },
                    },
                    Io {
                        crd: crd_n,
                        name: iob_name_n,
                        bank,
                        biob,
                        pkgid,
                        byte: None,
                        kind: IoKind::Hpio,
                        diff: IoDiffKind::N(crd_p),
                        is_lc: is_cc,
                        is_gc: false,
                        is_srcc: is_cc,
                        is_mrcc: false,
                        is_dqs: false,
                        is_vref,
                        vr: if is_vr { IoVrKind::VrP } else { IoVrKind::None },
                    },
                ]);

                if row.to_idx() % 32 == 8 {
                    let name = format!("HCLK_IOIS_DCI_X{x}Y{y}", y = y - 1);
                    let name_io0 = format!("IOIS_NC{l}_X{x}Y{y}", y = y - 2);
                    let name_io1 = format!("IOIS_LC{l}_X{x}Y{y}", y = y - 1);
                    let name_io2 = format!("IOIS_LC{l}_X{x}Y{y}");
                    let node = self.die.add_xnode(
                        (col, row),
                        self.db.get_node("HCLK_IOIS_DCI"),
                        &[&name, &name_io0, &name_io1, &name_io2],
                        self.db.get_node_naming("HCLK_IOIS_DCI"),
                        &[(col, row - 2), (col, row - 1), (col, row)],
                    );
                    let reg = row.to_idx() / 16;
                    node.add_bel(0, format!("BUFR_X{brx}Y{y}", y = reg * 2 + 1));
                    node.add_bel(1, format!("BUFR_X{brx}Y{y}", y = reg * 2));
                    node.add_bel(2, format!("BUFIO_X{biox}Y{y}", y = reg * 2 + 1));
                    node.add_bel(3, format!("BUFIO_X{biox}Y{y}", y = reg * 2));
                    node.add_bel(4, format!("IDELAYCTRL_X{biox}Y{reg}"));
                    node.add_bel(5, format!("DCI_X{biox}Y{y}", y = self.dciylut[reg]));
                } else if row.to_idx() % 32 == 24 {
                    let name = format!("HCLK_IOIS_LVDS_X{x}Y{y}", y = y - 1);
                    let name_io0 = format!("IOIS_NC{l}_X{x}Y{y}", y = y - 2);
                    let name_io1 = format!("IOIS_LC{l}_X{x}Y{y}", y = y - 1);
                    let name_io2 = format!("IOIS_LC{l}_X{x}Y{y}");
                    let node = self.die.add_xnode(
                        (col, row),
                        self.db.get_node("HCLK_IOIS_LVDS"),
                        &[&name, &name_io0, &name_io1, &name_io2],
                        self.db.get_node_naming("HCLK_IOIS_LVDS"),
                        &[(col, row - 2), (col, row - 1), (col, row)],
                    );
                    let reg = row.to_idx() / 16;
                    node.add_bel(0, format!("BUFR_X{brx}Y{y}", y = reg * 2 + 1));
                    node.add_bel(1, format!("BUFR_X{brx}Y{y}", y = reg * 2));
                    node.add_bel(2, format!("BUFIO_X{biox}Y{y}", y = reg * 2 + 1));
                    node.add_bel(3, format!("BUFIO_X{biox}Y{y}", y = reg * 2));
                    node.add_bel(4, format!("IDELAYCTRL_X{biox}Y{reg}"));
                }
            }
        }
    }

    fn fill_cfg(&mut self) {
        let col = self.col_cfg;
        let x = col.to_idx();
        let row_cfg = self.grid.row_reg_bot(self.grid.reg_cfg) - 8;
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
            let y = row.to_idx();
            let name = format!("CFG_CENTER_X{x}Y{y}", y = y + 7);
            for (i, crd) in crds.into_iter().enumerate() {
                self.die.add_xnode(
                    crd,
                    self.db.get_node("INTF"),
                    &[&name],
                    self.db.get_node_naming(&format!("INTF.CFG.{i}")),
                    &[crd],
                );
            }
            let name_bufg_b = format!("CLK_BUFGCTRL_B_X{x}Y{y}");
            let name_bufg_t = format!("CLK_BUFGCTRL_T_X{x}Y{y}", y = y + 8);
            let name_hrow_b = format!("CLK_HROW_X{x}Y{y}", y = y - 1);
            let name_hrow_t = format!("CLK_HROW_X{x}Y{y}", y = y + 15);
            let name_hclk_b = format!("HCLK_CENTER_X{x}Y{y}", y = y - 1);
            let name_hclk_t = format!("HCLK_CENTER_ABOVE_CFG_X{x}Y{y}", y = y + 15);
            let node = self.die.add_xnode(
                crds[0],
                self.db.get_node("CFG"),
                &[
                    &name,
                    &name_bufg_b,
                    &name_bufg_t,
                    &name_hrow_b,
                    &name_hrow_t,
                    &name_hclk_b,
                    &name_hclk_t,
                ],
                self.db.get_node_naming("CFG"),
                &crds,
            );
            for i in 0..32 {
                node.add_bel(i, format!("BUFGCTRL_X0Y{i}"));
            }
            for i in 0..4 {
                node.add_bel(32 + i, format!("BSCAN_X0Y{i}"));
            }
            for i in 0..2 {
                node.add_bel(36 + i, format!("ICAP_X0Y{i}"));
            }
            node.add_bel(38, "PMV".to_string());
            node.add_bel(39, "STARTUP".to_string());
            node.add_bel(40, "JTAGPPC".to_string());
            node.add_bel(41, "FRAME_ECC".to_string());
            node.add_bel(42, "DCIRESET".to_string());
            node.add_bel(43, "CAPTURE".to_string());
            node.add_bel(44, "USR_ACCESS_SITE".to_string());
        }
        let mut dcmy = 0;
        let mut ccmy = 0;
        let mut smy = 0;
        let mut row_dcmiob = RowId::from_idx(0);
        let mut row_iobdcm = RowId::from_idx(self.die.rows().len());
        for &(row, kind) in &self.grid.rows_cfg {
            let y = row.to_idx();
            match kind {
                CfgRowKind::Sysmon => {
                    self.site_holes.push(Rect {
                        col_l: col,
                        col_r: col + 1,
                        row_b: row,
                        row_t: row + 8,
                    });
                    let name = format!("SYS_MON_X{x}Y{y}");
                    let crds: [_; 8] = core::array::from_fn(|i| (col, row + i));
                    for (i, crd) in crds.into_iter().enumerate() {
                        self.die.add_xnode(
                            crd,
                            self.db.get_node("INTF"),
                            &[&name],
                            self.db.get_node_naming(&format!("INTF.SYSMON.{i}")),
                            &[crd],
                        );
                    }
                    let ipx = usize::from(self.grid.columns.first().unwrap() == &ColumnKind::Gt);
                    let ipy = if row.to_idx() == 0 {
                        0
                    } else if self.grid.columns.first().unwrap() == &ColumnKind::Gt {
                        self.grid.regs * 3
                    } else {
                        2
                    };
                    let sysmon = SysMon {
                        die: self.die.die,
                        col,
                        row,
                        bank: smy,
                        pad_vp: format!("IPAD_X{ipx}Y{ipy}"),
                        pad_vn: format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 1),
                        vaux: [
                            None,
                            Some(0),
                            Some(1),
                            Some(2),
                            Some(3),
                            Some(5),
                            Some(6),
                            Some(7),
                        ]
                        .into_iter()
                        .map(|x| {
                            x.map(|dy| {
                                (
                                    IoCoord {
                                        die: self.die.die,
                                        col: self.col_lio.unwrap(),
                                        row: row + dy,
                                        iob: TileIobId::from_idx(0),
                                    },
                                    IoCoord {
                                        die: self.die.die,
                                        col: self.col_lio.unwrap(),
                                        row: row + dy,
                                        iob: TileIobId::from_idx(1),
                                    },
                                )
                            })
                        })
                        .collect(),
                    };
                    let node = self.die.add_xnode(
                        crds[0],
                        self.db.get_node("SYSMON"),
                        &[&name],
                        self.db.get_node_naming("SYSMON"),
                        &crds,
                    );
                    node.add_bel(0, format!("MONITOR_X0Y{smy}"));
                    node.add_bel(1, sysmon.pad_vp.clone());
                    node.add_bel(2, sysmon.pad_vn.clone());
                    self.sysmon.push(sysmon);
                    smy += 1;
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
                    let (sk, tk) = if kind == CfgRowKind::Ccm {
                        ("CCM", "CCM")
                    } else if row < self.grid.row_reg_bot(self.grid.reg_cfg) {
                        ("DCM", "DCM_BOT")
                    } else {
                        ("DCM", "DCM")
                    };
                    let name = format!("{tk}_X{x}Y{y}");
                    for (i, crd) in crds.into_iter().enumerate() {
                        self.die.add_xnode(
                            crd,
                            self.db.get_node("INTF"),
                            &[&name],
                            self.db.get_node_naming(&format!("INTF.{sk}.{i}")),
                            &[crd],
                        );
                    }
                    let node = self.die.add_xnode(
                        (col, row),
                        self.db.get_node(sk),
                        &[&name],
                        self.db.get_node_naming(tk),
                        &crds,
                    );
                    if kind == CfgRowKind::Dcm {
                        node.add_bel(0, format!("DCM_ADV_X0Y{dcmy}"));
                        self.die[(col, row)].nodes.first_mut().unwrap().naming =
                            self.db.get_node_naming("INT.DCM0");
                        dcmy += 1;
                    } else {
                        node.add_bel(0, format!("PMCD_X0Y{y}", y = ccmy * 2));
                        node.add_bel(1, format!("PMCD_X0Y{y}", y = ccmy * 2 + 1));
                        node.add_bel(2, format!("DPM_X0Y{ccmy}"));
                        ccmy += 1;
                    }
                    if row.to_idx() % 8 == 0 {
                        let bt = if row < row_cfg { 'B' } else { 'T' };
                        let name = format!("CLKV_DCM_{bt}_X{x}Y{y}");
                        self.die.add_xnode(
                            (col, row),
                            self.db.get_node("CLK_DCM"),
                            &[&name],
                            self.db.get_node_naming("CLK_DCM"),
                            &[],
                        );
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
            let x = col.to_idx();
            let y = row.to_idx();
            if !self.is_site_hole(col, row) {
                self.die.add_xnode(
                    (col, row),
                    self.db.get_node("INTF"),
                    &[&format!("IOIS_LC_X{x}Y{y}")],
                    self.db.get_node_naming("INTF.IOIS"),
                    &[(col, row)],
                );
                let node = self.die.add_xnode(
                    (col, row),
                    self.db.get_node("IOIS"),
                    &[&format!("IOIS_LC_X{x}Y{y}")],
                    self.db.get_node_naming("IOIS_LC"),
                    &[(col, row)],
                );
                node.add_bel(0, format!("ILOGIC_X1Y{y}", y = 2 * y + 1));
                node.add_bel(1, format!("ILOGIC_X1Y{y}", y = 2 * y));
                node.add_bel(2, format!("OLOGIC_X1Y{y}", y = 2 * y + 1));
                node.add_bel(3, format!("OLOGIC_X1Y{y}", y = 2 * y));
                let iob_name_p = format!("IOB_X1Y{y}", y = 2 * y + 1);
                let iob_name_n = format!("IOB_X1Y{y}", y = 2 * y);
                node.add_bel(4, iob_name_p.clone());
                node.add_bel(5, iob_name_n.clone());
                let bank;
                let biob;
                let pkgid;
                let is_gc;
                if row < self.grid.row_bufg() {
                    if row < self.row_dcmiob.unwrap() + 8 {
                        bank = 4;
                        biob = (row.to_idx() % 8) as u32 * 2;
                        pkgid = (8 - (row.to_idx() % 8)) as u32;
                        is_gc = true;
                    } else if row >= self.grid.row_bufg() - 16 {
                        bank = 2;
                        biob = (row.to_idx() % 8) as u32 * 2;
                        pkgid = (8 - (row.to_idx() % 8)) as u32;
                        is_gc = false;
                    } else if row < self.row_dcmiob.unwrap() + 24 {
                        bank = 2;
                        biob = (row.to_idx() % 16) as u32 * 2 + 16;
                        pkgid = (16 - ((row.to_idx() % 16) ^ 8)) as u32 + 8;
                        is_gc = biob < 32;
                    } else {
                        bank = 2;
                        biob = (row.to_idx() % 16) as u32 * 2 + 48;
                        pkgid = (16 - ((row.to_idx() % 16) ^ 8)) as u32 + 24;
                        is_gc = false;
                    }
                } else {
                    if row >= self.row_iobdcm.unwrap() - 8 {
                        bank = 3;
                        biob = (row.to_idx() % 8) as u32 * 2;
                        pkgid = (8 - (row.to_idx() % 8)) as u32;
                        is_gc = true;
                    } else if row < self.grid.row_bufg() + 16 {
                        bank = 1;
                        biob = (row.to_idx() % 8) as u32 * 2;
                        pkgid = (8 - (row.to_idx() % 8)) as u32;
                        is_gc = false;
                    } else if row >= self.row_iobdcm.unwrap() - 24 {
                        bank = 1;
                        biob = (row.to_idx() % 16) as u32 * 2 + 16;
                        pkgid = (16 - (row.to_idx() % 16)) as u32 + 8;
                        is_gc = biob >= 32;
                    } else {
                        bank = 1;
                        biob = (row.to_idx() % 16) as u32 * 2 + 48;
                        pkgid = (16 - (row.to_idx() % 16)) as u32 + 24;
                        is_gc = false;
                    }
                }
                let crd_p = IoCoord {
                    die: self.die.die,
                    col,
                    row,
                    iob: TileIobId::from_idx(0),
                };
                let crd_n = IoCoord {
                    die: self.die.die,
                    col,
                    row,
                    iob: TileIobId::from_idx(1),
                };
                let is_cc = matches!(row.to_idx() % 16, 7 | 8);
                let is_vref = row.to_idx() % 8 == 4;
                let is_vr = match bank {
                    1 => biob == 28,
                    2 => biob == 34,
                    3 => biob == 12,
                    4 => biob == 2,
                    _ => unreachable!(),
                };
                self.io.extend([
                    Io {
                        crd: crd_p,
                        name: iob_name_p,
                        bank,
                        biob: biob + 1,
                        pkgid,
                        byte: None,
                        kind: IoKind::Hpio,
                        diff: IoDiffKind::P(crd_n),
                        is_lc: true,
                        is_gc,
                        is_srcc: is_cc,
                        is_mrcc: false,
                        is_dqs: false,
                        is_vref: false,
                        vr: if is_vr { IoVrKind::VrN } else { IoVrKind::None },
                    },
                    Io {
                        crd: crd_n,
                        name: iob_name_n,
                        bank,
                        biob,
                        pkgid,
                        byte: None,
                        kind: IoKind::Hpio,
                        diff: IoDiffKind::N(crd_p),
                        is_lc: true,
                        is_gc,
                        is_srcc: is_cc,
                        is_mrcc: false,
                        is_dqs: false,
                        is_vref,
                        vr: if is_vr { IoVrKind::VrP } else { IoVrKind::None },
                    },
                ]);
            }

            if row.to_idx() % 16 == 8 {
                let name_hrow = format!("CLK_HROW_X{x}Y{y}", y = y - 1);
                self.die.add_xnode(
                    (col, row),
                    self.db.get_node("CLK_HROW"),
                    &[&name_hrow],
                    self.db.get_node_naming("CLK_HROW"),
                    &[],
                );

                let reg = row.to_idx() / 16;
                if row < self.row_dcmiob.unwrap() || row > self.row_iobdcm.unwrap() {
                    let name = format!("HCLK_DCM_X{x}Y{y}", y = y - 1);
                    self.die.add_xnode(
                        (col, row),
                        self.db.get_node("HCLK_DCM"),
                        &[&name, &name_hrow],
                        self.db.get_node_naming("HCLK_DCM"),
                        &[],
                    );
                } else if row == self.row_dcmiob.unwrap() {
                    let name = format!("HCLK_DCMIOB_X{x}Y{y}", y = y - 1);
                    let name_io0 = format!("IOIS_LC_X{x}Y{y}");
                    let name_io1 = format!("IOIS_LC_X{x}Y{y}", y = y + 1);
                    let node = self.die.add_xnode(
                        (col, row),
                        self.db.get_node("HCLK_DCMIOB"),
                        &[&name, &name_io0, &name_io1, &name_hrow],
                        self.db.get_node_naming("HCLK_DCMIOB"),
                        &[(col, row), (col, row + 1)],
                    );
                    node.add_bel(0, format!("BUFIO_X1Y{y}", y = reg * 2 + 1));
                    node.add_bel(1, format!("BUFIO_X1Y{y}", y = reg * 2));
                    node.add_bel(2, format!("IDELAYCTRL_X1Y{reg}"));
                    node.add_bel(3, format!("DCI_X1Y{y}", y = self.dciylut[reg]));
                } else if row == self.row_iobdcm.unwrap() {
                    let name = format!("HCLK_IOBDCM_X{x}Y{y}", y = y - 1);
                    let name_io0 = format!("IOIS_LC_X{x}Y{y}", y = y - 2);
                    let name_io1 = format!("IOIS_LC_X{x}Y{y}", y = y - 1);
                    let node = self.die.add_xnode(
                        (col, row),
                        self.db.get_node("HCLK_IOBDCM"),
                        &[&name, &name_io0, &name_io1, &name_hrow],
                        self.db.get_node_naming("HCLK_IOBDCM"),
                        &[(col, row - 2), (col, row - 1)],
                    );
                    node.add_bel(0, format!("BUFIO_X1Y{y}", y = reg * 2 + 1));
                    node.add_bel(1, format!("BUFIO_X1Y{y}", y = reg * 2));
                    node.add_bel(2, format!("IDELAYCTRL_X1Y{reg}"));
                    node.add_bel(3, format!("DCI_X1Y{y}", y = self.dciylut[reg]));
                } else if row == self.grid.row_bufg() + 8 {
                    let name = format!("HCLK_CENTER_ABOVE_CFG_X{x}Y{y}", y = y - 1);
                    let name_io0 = format!("IOIS_LC_X{x}Y{y}");
                    let name_io1 = format!("IOIS_LC_X{x}Y{y}", y = y + 1);
                    let node = self.die.add_xnode(
                        (col, row),
                        self.db.get_node("HCLK_CENTER_ABOVE_CFG"),
                        &[&name, &name_io0, &name_io1],
                        self.db.get_node_naming("HCLK_CENTER_ABOVE_CFG"),
                        &[(col, row), (col, row + 1)],
                    );
                    node.add_bel(0, format!("BUFIO_X1Y{y}", y = reg * 2 + 1));
                    node.add_bel(1, format!("BUFIO_X1Y{y}", y = reg * 2));
                    node.add_bel(2, format!("IDELAYCTRL_X1Y{reg}"));
                    node.add_bel(3, format!("DCI_X1Y{y}", y = self.dciylut[reg]));
                } else {
                    let name = format!("HCLK_CENTER_X{x}Y{y}", y = y - 1);
                    let name_io0 = format!("IOIS_LC_X{x}Y{y}", y = y - 2);
                    let name_io1 = format!("IOIS_LC_X{x}Y{y}", y = y - 1);
                    let node = self.die.add_xnode(
                        (col, row),
                        self.db.get_node("HCLK_CENTER"),
                        &[&name, &name_io0, &name_io1],
                        self.db.get_node_naming("HCLK_CENTER"),
                        &[(col, row - 2), (col, row - 1)],
                    );
                    node.add_bel(0, format!("BUFIO_X1Y{y}", y = reg * 2 + 1));
                    node.add_bel(1, format!("BUFIO_X1Y{y}", y = reg * 2));
                    node.add_bel(2, format!("IDELAYCTRL_X1Y{reg}"));
                    node.add_bel(3, format!("DCI_X1Y{y}", y = self.dciylut[reg]));
                }
            }
        }

        {
            let row = self.row_dcmiob.unwrap();
            let x = col.to_idx();
            let y = row.to_idx();
            let name = format!("CLK_IOB_B_X{x}Y{y}", y = y + 7);
            self.die.add_xnode(
                (col, row),
                self.db.get_node("CLK_IOB"),
                &[&name],
                self.db.get_node_naming("CLK_IOB"),
                &[],
            );
        }
        {
            let row = self.row_iobdcm.unwrap() - 16;
            let x = col.to_idx();
            let y = row.to_idx();
            let name = format!("CLK_IOB_T_X{x}Y{y}", y = y + 7);
            self.die.add_xnode(
                (col, row),
                self.db.get_node("CLK_IOB"),
                &[&name],
                self.db.get_node_naming("CLK_IOB"),
                &[],
            );
        }
    }

    fn fill_ppc(&mut self) {
        for (py, &(bc, br)) in self.grid.holes_ppc.iter().enumerate() {
            let x = bc.to_idx();
            let yb = br.to_idx() + 3;
            let yt = br.to_idx() + 19;
            let tile_pb = format!("PB_X{x}Y{yb}");
            let tile_pt = format!("PT_X{x}Y{yt}");
            let col_l = bc;
            let col_r = bc + 8;
            for dy in 0..22 {
                let row = br + 1 + dy;
                let tile = if dy < 11 { &tile_pb } else { &tile_pt };
                self.die.fill_term_pair_buf(
                    (col_l, row),
                    (col_r, row),
                    self.db.get_term("PPC.E"),
                    self.db.get_term("PPC.W"),
                    tile.clone(),
                    self.db.get_term_naming(&format!("TERM.PPC.E{dy}")),
                    self.db.get_term_naming(&format!("TERM.PPC.W{dy}")),
                );
            }
            let row_b = br;
            let row_t = br + 23;
            for dx in 0..7 {
                let col = bc + 1 + dx;
                self.die.fill_term_pair_dbuf(
                    (col, row_b),
                    (col, row_t),
                    self.db.get_term(if dx < 5 { "PPCA.N" } else { "PPCB.N" }),
                    self.db.get_term(if dx < 5 { "PPCA.S" } else { "PPCB.S" }),
                    tile_pb.clone(),
                    tile_pt.clone(),
                    self.db.get_term_naming(&format!("TERM.PPC.N{dx}")),
                    self.db.get_term_naming(&format!("TERM.PPC.S{dx}")),
                );
            }
            for dy in 0..24 {
                let row = br + dy;
                let tile = if dy < 12 { &tile_pb } else { &tile_pt };
                self.die.add_xnode(
                    (col_l, row),
                    self.db.get_node("INTF"),
                    &[tile],
                    self.db.get_node_naming(&format!("INTF.PPC.L{dy}")),
                    &[(col_l, row)],
                );
                self.die.add_xnode(
                    (col_r, row),
                    self.db.get_node("INTF"),
                    &[tile],
                    self.db.get_node_naming(&format!("INTF.PPC.R{dy}")),
                    &[(col_r, row)],
                );
            }
            for dx in 0..7 {
                let col = bc + dx + 1;
                self.die.add_xnode(
                    (col, row_b),
                    self.db.get_node("INTF"),
                    &[&tile_pb],
                    self.db.get_node_naming(&format!("INTF.PPC.B{dx}")),
                    &[(col, row_b)],
                );
                self.die.add_xnode(
                    (col, row_t),
                    self.db.get_node("INTF"),
                    &[&tile_pt],
                    self.db.get_node_naming(&format!("INTF.PPC.T{dx}")),
                    &[(col, row_t)],
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
            let node = self.die.add_xnode(
                (bc, br),
                self.db.get_node("PPC"),
                &[&tile_pb, &tile_pt],
                self.db.get_node_naming("PPC"),
                &crds,
            );
            node.add_bel(0, format!("PPC405_ADV_X0Y{py}"));
            node.add_bel(1, format!("EMAC_X0Y{py}"));
        }
    }

    fn fill_term(&mut self) {
        let row_b = self.die.rows().next().unwrap();
        let row_t = self.die.rows().next_back().unwrap();
        let yb = row_b.to_idx();
        let yt = row_t.to_idx();
        for col in self.die.cols() {
            let x = col.to_idx();
            self.die.fill_term_tile(
                (col, row_b),
                "TERM.S",
                "TERM.S",
                format!("B_TERM_INT_X{x}Y{yb}"),
            );
            self.die.fill_term_tile(
                (col, row_t),
                "TERM.N",
                "TERM.N",
                format!("T_TERM_INT_X{x}Y{yt}"),
            );
        }
        let col_l = self.die.cols().next().unwrap();
        let col_r = self.die.cols().next_back().unwrap();
        let xl = col_l.to_idx();
        let xr = col_r.to_idx();
        for row in self.die.rows() {
            let y = row.to_idx();
            if self.grid.columns[col_l] == ColumnKind::Gt {
                let dy = y % 16;
                let yy = y - dy + 8;
                let ab = if y % 32 >= 16 { "A" } else { "B" };
                let tile = format!("MGT_{ab}L_X{xl}Y{yy}");
                self.die.fill_term_tile(
                    (col_l, row),
                    "TERM.W",
                    &format!("TERM.W.MGT{dy}"),
                    tile.clone(),
                );
                self.die.add_xnode(
                    (col_l, row),
                    self.db.get_node("INTF"),
                    &[&tile],
                    self.db.get_node_naming(&format!("INTF.MGT.{dy}")),
                    &[(col_l, row)],
                );
            } else {
                self.die.fill_term_tile(
                    (col_l, row),
                    "TERM.W",
                    "TERM.W",
                    format!("L_TERM_INT_X{xl}Y{y}"),
                );
            }
            if self.grid.columns[col_r] == ColumnKind::Gt {
                let dy = y % 16;
                let yy = y - dy + 8;
                let ab = if y % 32 >= 16 { "A" } else { "B" };
                let tile = format!("MGT_{ab}R_X{xr}Y{yy}");
                self.die.fill_term_tile(
                    (col_r, row),
                    "TERM.E",
                    &format!("TERM.E.MGT{dy}"),
                    tile.clone(),
                );
                self.die.add_xnode(
                    (col_r, row),
                    self.db.get_node("INTF"),
                    &[&tile],
                    self.db.get_node_naming(&format!("INTF.MGT.{dy}")),
                    &[(col_r, row)],
                );
            } else {
                self.die.fill_term_tile(
                    (col_r, row),
                    "TERM.E",
                    "TERM.E",
                    format!("R_TERM_INT_X{xr}Y{y}"),
                );
            }
        }

        let term_s = self.db.get_term("BRKH.S");
        let term_n = self.db.get_term("BRKH.N");
        for col in self.die.cols() {
            'a: for row in self.die.rows() {
                if row.to_idx() % 8 != 0 || row.to_idx() == 0 {
                    continue;
                }
                for hole in &self.int_holes {
                    if hole.contains(col, row) {
                        continue 'a;
                    }
                }
                self.die
                    .fill_term_pair_anon((col, row - 1), (col, row), term_n, term_s);
            }
        }

        let term_w = self.db.get_term("CLB_BUFFER.W");
        let term_e = self.db.get_term("CLB_BUFFER.E");
        let naming_w = self.db.get_term_naming("PASS.CLB_BUFFER.W");
        let naming_e = self.db.get_term_naming("PASS.CLB_BUFFER.E");
        for (col, &cd) in &self.grid.columns {
            if !matches!(cd, ColumnKind::Io | ColumnKind::Cfg) || col == col_l || col == col_r {
                continue;
            }
            for row in self.die.rows() {
                let x = col.to_idx();
                let y = row.to_idx();
                let tile = format!("CLB_BUFFER_X{x}Y{y}");
                self.die.fill_term_pair_buf(
                    (col, row),
                    (col + 1, row),
                    term_e,
                    term_w,
                    tile,
                    naming_w,
                    naming_e,
                );
            }
        }
    }

    fn fill_clb(&mut self) {
        let mut sx = 0;
        for (col, &cd) in &self.grid.columns {
            if cd != ColumnKind::ClbLM {
                continue;
            }
            for row in self.die.rows() {
                if self.is_site_hole(col, row) {
                    continue;
                }
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("CLB_X{x}Y{y}");
                let node = self.die.add_xnode(
                    (col, row),
                    self.db.get_node("CLB"),
                    &[&name],
                    self.db.get_node_naming("CLB"),
                    &[(col, row)],
                );
                node.add_bel(0, format!("SLICE_X{sx}Y{sy}", sy = 2 * y));
                node.add_bel(1, format!("SLICE_X{sx}Y{sy}", sx = sx + 1, sy = 2 * y));
                node.add_bel(2, format!("SLICE_X{sx}Y{sy}", sy = 2 * y + 1));
                node.add_bel(3, format!("SLICE_X{sx}Y{sy}", sx = sx + 1, sy = 2 * y + 1));
            }
            sx += 2;
        }
    }

    fn fill_bram_dsp(&mut self) {
        let mut bx = 0;
        let mut dx = 0;
        for (col, &cd) in &self.grid.columns {
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
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("{kind}_X{x}Y{y}");
                for dy in 0..4 {
                    self.die.add_xnode(
                        (col, row + dy),
                        self.db.get_node("INTF"),
                        &[&name],
                        self.db.get_node_naming(&format!("INTF.{kind}.{dy}")),
                        &[(col, row + dy)],
                    );
                }
                let node = self.die.add_xnode(
                    (col, row),
                    self.db.get_node(kind),
                    &[&name],
                    self.db.get_node_naming(kind),
                    &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                );
                if cd == ColumnKind::Bram {
                    node.add_bel(0, format!("RAMB16_X{bx}Y{sy}", sy = y / 4));
                    node.add_bel(1, format!("FIFO16_X{bx}Y{sy}", sy = y / 4));
                } else {
                    node.add_bel(0, format!("DSP48_X{dx}Y{sy}", sy = y / 4 * 2));
                    node.add_bel(1, format!("DSP48_X{dx}Y{sy}", sy = y / 4 * 2 + 1));
                }
            }
            if cd == ColumnKind::Bram {
                bx += 1;
            } else {
                dx += 1;
            }
        }
    }

    fn fill_gt(&mut self) {
        for (col, &cd) in &self.grid.columns {
            if cd != ColumnKind::Gt {
                continue;
            }
            let x = col.to_idx();
            let lr = if col.to_idx() == 0 { 'L' } else { 'R' };
            let gtx = usize::from(col.to_idx() != 0);
            let banks: &[u32] = match (lr, self.grid.regs) {
                ('L', 4) => &[105, 102],
                ('L', 6) => &[105, 103, 102],
                ('L', 8) => &[106, 105, 103, 102],
                ('L', 10) => &[106, 105, 103, 102, 101],
                ('L', 12) => &[106, 105, 104, 103, 102, 101],
                ('R', 4) => &[110, 113],
                ('R', 6) => &[110, 112, 113],
                ('R', 8) => &[109, 110, 112, 113],
                ('R', 10) => &[109, 110, 112, 113, 114],
                ('R', 12) => &[109, 110, 111, 112, 113, 114],
                _ => unreachable!(),
            };
            let has_bot_sysmon = self
                .grid
                .rows_cfg
                .contains(&(RowId::from_idx(0), CfgRowKind::Sysmon));
            let ipx = if col.to_idx() == 0 {
                0
            } else if has_bot_sysmon {
                2
            } else {
                1
            };
            let mut ipy = 0;
            if has_bot_sysmon {
                ipy = 2;
            }
            let mut gty = 0;
            for reg in self.grid.regs() {
                if reg.to_idx() % 2 != 0 {
                    continue;
                }
                let row = self.grid.row_reg_bot(reg);
                let gt = Gt {
                    die: self.die.die,
                    col,
                    row,
                    bank: banks[gty],
                    kind: GtKind::Gtp,
                    pads_clk: vec![(
                        format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 3),
                        format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 2),
                    )],
                    pads_rx: vec![
                        (
                            format!("IPAD_X{ipx}Y{ipy}"),
                            format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 1),
                        ),
                        (
                            format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 4),
                            format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 5),
                        ),
                    ],
                    pads_tx: vec![
                        (
                            format!("OPAD_X{gtx}Y{opy}", opy = gty * 4),
                            format!("OPAD_X{gtx}Y{opy}", opy = gty * 4 + 1),
                        ),
                        (
                            format!("OPAD_X{gtx}Y{opy}", opy = gty * 4 + 2),
                            format!("OPAD_X{gtx}Y{opy}", opy = gty * 4 + 3),
                        ),
                    ],
                };
                {
                    let row = row + 16;
                    let y = row.to_idx();
                    let node = self.die.add_xnode(
                        (col, row),
                        self.db.get_node("MGTCLK"),
                        &[&format!("BRKH_MGT11CLK_{lr}_X{x}Y{y}", y = y - 1)],
                        self.db.get_node_naming(&format!("BRKH_MGT11CLK_{lr}")),
                        &[],
                    );
                    node.add_bel(0, format!("GT11CLK_X{gtx}Y{gty}"));
                    node.add_bel(1, gt.pads_clk[0].0.clone());
                    node.add_bel(2, gt.pads_clk[0].1.clone());
                }
                for i in 0..2 {
                    let row = row + 16 * i;
                    let y = row.to_idx();
                    let ab = if i == 0 { 'B' } else { 'A' };
                    let crds: [_; 16] = core::array::from_fn(|i| (col, row + i));
                    let node = self.die.add_xnode(
                        (col, row),
                        self.db.get_node("MGT"),
                        &[&format!("MGT_{ab}{lr}_X{x}Y{y}", y = y + 8)],
                        self.db.get_node_naming(&format!("MGT_{ab}{lr}")),
                        &crds,
                    );
                    node.add_bel(0, format!("GT11_X{gtx}Y{gty}", gty = gty * 2 + i));
                    node.add_bel(1, gt.pads_rx[i].0.clone());
                    node.add_bel(2, gt.pads_rx[i].1.clone());
                    node.add_bel(3, gt.pads_tx[i].0.clone());
                    node.add_bel(4, gt.pads_tx[i].1.clone());
                }
                self.gt.push(gt);
                ipy += 6;
                gty += 1;
            }
        }
    }

    fn fill_hclk(&mut self) {
        for col in self.die.cols() {
            'a: for row in self.die.rows() {
                let crow = self.grid.row_hclk(row);
                self.die[(col, row)].clkroot = (col, crow);
                if row.to_idx() % 16 == 8 {
                    for hole in &self.int_holes {
                        if hole.contains(col, row) {
                            continue 'a;
                        }
                    }
                    let x = col.to_idx();
                    let y = row.to_idx();
                    let name = format!("HCLK_X{x}Y{y}", y = y - 1);
                    let node = self.die.add_xnode(
                        (col, row),
                        self.db.get_node("HCLK"),
                        &[&name],
                        self.db.get_node_naming("HCLK"),
                        &[(col, row)],
                    );
                    node.add_bel(0, format!("GLOBALSIG_X{x}Y{y}", y = y / 16));
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
            self.frames.spine_frame.push(0);
        }
        for &reg in &regs {
            let mut major = 0;
            for &cd in self.grid.columns.values() {
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
                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 0,
                            region: (reg - self.grid.reg_cfg) as i32,
                            major,
                            minor,
                        },
                    });
                }
                major += 1;
                if cd == ColumnKind::Cfg {
                    self.frames.spine_frame[reg] = self.frame_info.len();
                    for minor in 0..3 {
                        self.frame_info.push(FrameInfo {
                            addr: FrameAddr {
                                typ: 0,
                                region: (reg - self.grid.reg_cfg) as i32,
                                major,
                                minor,
                            },
                        });
                    }
                    major += 1;
                }
            }
        }
        for &reg in &regs {
            let mut major = 0;
            for (col, &cd) in &self.grid.columns {
                if cd != ColumnKind::Bram {
                    continue;
                }
                self.frames.col_frame[reg][col] = self.frame_info.len();
                for minor in 0..20 {
                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 1,
                            region: (reg - self.grid.reg_cfg) as i32,
                            major,
                            minor,
                        },
                    });
                }
                major += 1;
            }
        }
        for &reg in &regs {
            let mut major = 0;
            for (col, &cd) in &self.grid.columns {
                if cd != ColumnKind::Bram {
                    continue;
                }
                self.frames.bram_frame[reg].insert(col, self.frame_info.len());
                for minor in 0..64 {
                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 2,
                            region: (reg - self.grid.reg_cfg) as i32,
                            major,
                            minor,
                        },
                    });
                }
                major += 1;
            }
        }
    }
}

pub fn expand_grid<'a>(
    grids: &EntityVec<DieId, &'a Grid>,
    grid_master: DieId,
    extras: &[ExtraDie],
    disabled: &BTreeSet<DisabledPart>,
    db: &'a IntDb,
) -> ExpandedDevice<'a> {
    let mut egrid = ExpandedGrid::new(db);
    let grid = grids[grid_master];
    assert_eq!(grids.len(), 1);
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
    let cols_io: Vec<_> = grid
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
    egrid.tie_kind = Some("TIEOFF".to_string());
    egrid.tie_pin_pullup = Some("KEEP1".to_string());
    egrid.tie_pin_gnd = Some("HARD0".to_string());
    egrid.tie_pin_vcc = Some("HARD1".to_string());
    let (_, die) = egrid.add_die(grid.columns.len(), grid.regs * 16);
    let mut expander = Expander {
        grid,
        db,
        die,
        int_holes: vec![],
        site_holes: vec![],
        dciylut: vec![],
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
        sysmon: vec![],
    };

    expander.fill_holes();
    expander.fill_int();
    expander.fill_cfg();
    expander.fill_dciylut();
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

    let site_holes = expander.site_holes;
    let frames = expander.frames;
    let io = expander.io;
    let gt = expander.gt;
    let sysmon = expander.sysmon;
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
    };

    let mut cfg_io = BiHashMap::new();
    for i in 0..16 {
        cfg_io.insert(
            SharedCfgPin::Data(i as u8),
            IoCoord {
                die: grid_master,
                col: col_cfg,
                row: grid.row_reg_bot(grid.reg_cfg) - 16 + i / 2,
                iob: TileIobId::from_idx(!i & 1),
            },
        );
    }
    for i in 0..16 {
        cfg_io.insert(
            SharedCfgPin::Data(i as u8 + 16),
            IoCoord {
                die: grid_master,
                col: col_cfg,
                row: grid.row_reg_bot(grid.reg_cfg) + 8 + i / 2,
                iob: TileIobId::from_idx(!i & 1),
            },
        );
    }

    egrid.finish();
    ExpandedDevice {
        kind: grid.kind,
        grids: grids.clone(),
        grid_master,
        extras: extras.to_vec(),
        disabled: disabled.clone(),
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
        gtz: vec![],
        sysmon,
        cfg_io,
        ps_io: Default::default(),
    }
}
