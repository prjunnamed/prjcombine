use assert_matches::assert_matches;
use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};
use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ColId, DieId, ExpandedDieRefMut, ExpandedGrid, Rect, RowId};
use prjcombine_virtex_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};
use std::collections::BTreeSet;

use prjcombine_virtex4::bond::SharedCfgPin;
use prjcombine_virtex4::grid::{ColumnKind, DisabledPart, ExtraDie, Grid, GtKind, IoKind};

use prjcombine_virtex4::expanded::{
    DieFrameGeom, ExpandedDevice, Gt, Io, IoCoord, IoDiffKind, IoVrKind, SysMon, TileIobId,
};

struct Expander<'a, 'b> {
    grid: &'b Grid,
    db: &'a IntDb,
    die: ExpandedDieRefMut<'a, 'b>,
    site_holes: Vec<Rect>,
    int_holes: Vec<Rect>,
    rxlut: EntityVec<ColId, usize>,
    frame_info: Vec<FrameInfo>,
    frames: DieFrameGeom,
    col_cfg: ColId,
    col_lio: Option<ColId>,
    col_rio: Option<ColId>,
    col_lgt: Option<ColId>,
    io: Vec<Io>,
    gt: Vec<Gt>,
    sysmon: Vec<SysMon>,
}

impl<'a, 'b> Expander<'a, 'b> {
    fn fill_rxlut(&mut self) {
        let mut rx = 0;
        for (col, &kind) in &self.grid.columns {
            if self.grid.cols_vbrk.contains(&col) {
                rx += 1;
            }
            self.rxlut.push(rx);
            rx += match kind {
                ColumnKind::ClbLL | ColumnKind::ClbLM => 2,
                ColumnKind::Bram | ColumnKind::Dsp => 3,
                ColumnKind::Io => {
                    if col.to_idx() == 0 {
                        5
                    } else {
                        6
                    }
                }
                ColumnKind::Cfg => 7,
                ColumnKind::Gt => 4,
                _ => unreachable!(),
            };
        }
    }

    fn fill_int(&mut self) {
        for (col, &kind) in &self.grid.columns {
            for row in self.die.rows() {
                let x = col.to_idx();
                let y = row.to_idx();
                self.die
                    .fill_tile((col, row), "INT", "INT", format!("INT_X{x}Y{y}"));
                let tile = &mut self.die[(col, row)];
                tile.nodes[0].tie_name = Some(format!("TIEOFF_X{x}Y{y}"));
                match kind {
                    ColumnKind::ClbLL => (),
                    ColumnKind::ClbLM => (),
                    ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::Io | ColumnKind::Cfg => {
                        tile.add_xnode(
                            self.db.get_node("INTF"),
                            &[&format!("INT_INTERFACE_X{x}Y{y}")],
                            self.db.get_node_naming("INTF"),
                            &[(col, row)],
                        );
                    }
                    ColumnKind::Gt if col.to_idx() != 0 => {
                        tile.add_xnode(
                            self.db.get_node("INTF.DELAY"),
                            &[&format!("GTP_INT_INTERFACE_X{x}Y{y}")],
                            self.db.get_node_naming("INTF.GTP"),
                            &[(col, row)],
                        );
                    }
                    ColumnKind::Gt => {
                        tile.add_xnode(
                            self.db.get_node("INTF.DELAY"),
                            &[&format!("GTX_LEFT_INT_INTERFACE_X{x}Y{y}")],
                            self.db.get_node_naming("INTF.GTX_LEFT"),
                            &[(col, row)],
                        );
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    fn fill_ppc(&mut self) {
        for (py, &(bc, br)) in self.grid.holes_ppc.iter().enumerate() {
            self.die.nuke_rect(bc + 1, br, 12, 40);
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
            let col_l = bc;
            let col_r = bc + 13;
            let xl = col_l.to_idx();
            let xr = col_r.to_idx();
            for dy in 0..40 {
                let row = br + dy;
                let y = row.to_idx();
                // sigh.
                let rxr = 53;
                let ry = y / 10 * 11 + y % 10 + 1;
                let tile_l = format!("L_TERM_PPC_X{xl}Y{y}");
                let tile_r = format!("R_TERM_PPC_X{rxr}Y{ry}");
                self.die.fill_term_pair_dbuf(
                    (col_l, row),
                    (col_r, row),
                    self.db.get_term("PPC.E"),
                    self.db.get_term("PPC.W"),
                    tile_l,
                    tile_r,
                    self.db.get_term_naming("PPC.E"),
                    self.db.get_term_naming("PPC.W"),
                );
                let tile = &mut self.die[(col_l, row)];
                tile.nodes.truncate(1);
                tile.add_xnode(
                    self.db.get_node("INTF.DELAY"),
                    &[&format!("PPC_L_INT_INTERFACE_X{xl}Y{y}")],
                    self.db.get_node_naming("INTF.PPC_L"),
                    &[(col_l, row)],
                );
                let tile = &mut self.die[(col_r, row)];
                tile.nodes.truncate(1);
                tile.add_xnode(
                    self.db.get_node("INTF.DELAY"),
                    &[&format!("PPC_R_INT_INTERFACE_X{xr}Y{y}")],
                    self.db.get_node_naming("INTF.PPC_R"),
                    &[(col_r, row)],
                );
            }
            let row_b = br - 1;
            let row_t = br + 40;
            let yb = row_b.to_idx();
            let yt = row_t.to_idx();
            for dx in 1..13 {
                let col = bc + dx;
                let x = col.to_idx();
                self.die.fill_term_tile(
                    (col, row_b),
                    "TERM.N.PPC",
                    "TERM.N.PPC",
                    format!("PPC_B_TERM_X{x}Y{yb}"),
                );
                self.die.fill_term_tile(
                    (col, row_t),
                    "TERM.S.PPC",
                    "TERM.S.PPC",
                    format!("PPC_T_TERM_X{x}Y{yt}"),
                );
            }
            let mut crds = vec![];
            for dy in 0..40 {
                crds.push((col_l, br + dy));
            }
            for dy in 0..40 {
                crds.push((col_r, br + dy));
            }
            let yb = br.to_idx() / 10 * 11 + 11;
            let yt = br.to_idx() / 10 * 11 + 33;
            let tile_pb = format!("PPC_B_X36Y{yb}");
            let tile_pt = format!("PPC_T_X36Y{yt}");
            let node = self.die[(bc, br)].add_xnode(
                self.db.get_node("PPC"),
                &[&tile_pb, &tile_pt],
                self.db.get_node_naming("PPC"),
                &crds,
            );
            node.add_bel(0, format!("PPC440_X0Y{py}"));
        }
    }

    fn fill_terms(&mut self) {
        let row_b = self.die.rows().next().unwrap();
        let row_t = self.die.rows().next_back().unwrap();
        let term_n = self.db.get_term("MAIN.NHOLE.N");
        let term_s = self.db.get_term("MAIN.NHOLE.S");
        for col in self.die.cols() {
            self.die.fill_term_anon((col, row_b), "TERM.S.HOLE");
            self.die.fill_term_anon((col, row_t), "TERM.N.HOLE");
            self.die
                .fill_term_pair_anon((col, row_t - 1), (col, row_t), term_n, term_s);
        }
        let col_l = self.die.cols().next().unwrap();
        let col_r = self.die.cols().next_back().unwrap();
        let xl = col_l.to_idx();
        let xr = col_r.to_idx();
        for row in self.die.rows() {
            let y = row.to_idx();
            if self.grid.columns[col_l] == ColumnKind::Gt {
                self.die.fill_term_tile(
                    (col_l, row),
                    "TERM.W",
                    "TERM.W",
                    format!("GTX_L_TERM_INT_X{xl}Y{y}"),
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
                self.die.fill_term_tile(
                    (col_r, row),
                    "TERM.E",
                    "TERM.E",
                    format!("R_TERM_INT_X{xr}Y{y}"),
                );
            } else {
                self.die.fill_term_anon((col_r, row), "TERM.E.HOLE");
            }
        }
    }

    fn fill_int_bufs(&mut self) {
        let col_l = self.die.cols().next().unwrap();
        let col_r = self.die.cols().next_back().unwrap();
        let term_w = self.db.get_term("INT_BUFS.W");
        let term_e = self.db.get_term("INT_BUFS.E");
        let naming_w = self.db.get_term_naming("INT_BUFS.W");
        let naming_e = self.db.get_term_naming("INT_BUFS.E");
        for (col, &cd) in &self.grid.columns {
            if !matches!(cd, ColumnKind::Io | ColumnKind::Cfg) || col == col_l || col == col_r {
                continue;
            }
            for row in self.die.rows() {
                let x = col.to_idx();
                let y = row.to_idx();
                let tile_l = format!("INT_BUFS_L_X{x}Y{y}");
                let mon = if self.grid.columns[col_l] == ColumnKind::Gt {
                    "_MON"
                } else {
                    ""
                };
                let tile_r = format!("INT_BUFS_R{mon}_X{xx}Y{y}", xx = x + 1);
                self.die.fill_term_pair_dbuf(
                    (col, row),
                    (col + 1, row),
                    term_e,
                    term_w,
                    tile_l,
                    tile_r,
                    naming_e,
                    naming_w,
                );
            }
        }
    }

    fn fill_clb(&mut self) {
        let mut sx = 0;
        for (col, &cd) in &self.grid.columns {
            let kind = match cd {
                ColumnKind::ClbLL => "CLBLL",
                ColumnKind::ClbLM => "CLBLM",
                _ => continue,
            };
            'a: for row in self.die.rows() {
                for hole in &self.site_holes {
                    if hole.contains(col, row) {
                        continue 'a;
                    }
                }
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("{kind}_X{x}Y{y}");
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node(kind),
                    &[&name],
                    self.db.get_node_naming(kind),
                    &[(col, row)],
                );
                node.add_bel(0, format!("SLICE_X{sx}Y{y}"));
                node.add_bel(1, format!("SLICE_X{sx}Y{y}", sx = sx + 1));
            }
            sx += 2;
        }
    }

    fn fill_hard(&mut self) {
        if let Some(ref hard) = self.grid.col_hard {
            let col = hard.col;
            let x = col.to_idx();
            for (i, &row) in hard.rows_emac.iter().enumerate() {
                for dy in 0..10 {
                    let row = row + dy;
                    let y = row.to_idx();
                    let tile = &mut self.die[(col, row)];
                    tile.nodes.truncate(1);
                    tile.add_xnode(
                        self.db.get_node("INTF.DELAY"),
                        &[&format!("EMAC_INT_INTERFACE_X{x}Y{y}")],
                        self.db.get_node_naming("INTF.EMAC"),
                        &[(col, row)],
                    );
                }
                self.site_holes.push(Rect {
                    col_l: col,
                    col_r: col + 1,
                    row_b: row,
                    row_t: row + 10,
                });
                let y = row.to_idx();
                let crds: Vec<_> = (0..10).map(|dy| (col, row + dy)).collect();
                let name = format!("EMAC_X{x}Y{y}");
                let node = self.die[crds[0]].add_xnode(
                    self.db.get_node("EMAC"),
                    &[&name],
                    self.db.get_node_naming("EMAC"),
                    &crds,
                );
                node.add_bel(0, format!("TEMAC_X0Y{i}"));
            }
            for (i, &row) in hard.rows_pcie.iter().enumerate() {
                for dy in 0..40 {
                    let row = row + dy;
                    let y = row.to_idx();
                    let tile = &mut self.die[(col, row)];
                    tile.nodes.truncate(1);
                    tile.add_xnode(
                        self.db.get_node("INTF.DELAY"),
                        &[&format!("PCIE_INT_INTERFACE_X{x}Y{y}")],
                        self.db.get_node_naming("INTF.PCIE"),
                        &[(col, row)],
                    );
                }
                self.site_holes.push(Rect {
                    col_l: col,
                    col_r: col + 1,
                    row_b: row,
                    row_t: row + 40,
                });
                let y = row.to_idx();
                let crds: Vec<_> = (0..40).map(|dy| (col, row + dy)).collect();
                let name_b = format!("PCIE_B_X{x}Y{y}", y = y + 10);
                let name_t = format!("PCIE_T_X{x}Y{y}", y = y + 30);
                let node = self.die[crds[0]].add_xnode(
                    self.db.get_node("PCIE"),
                    &[&name_b, &name_t],
                    self.db.get_node_naming("PCIE"),
                    &crds,
                );
                node.add_bel(0, format!("PCIE_X0Y{i}"));
            }
        }
    }

    fn fill_bram_dsp(&mut self) {
        let mut px = 0;
        let mut bx = 0;
        let mut dx = 0;
        for (col, &cd) in &self.grid.columns {
            let kind = match cd {
                ColumnKind::Bram => "BRAM",
                ColumnKind::Dsp => "DSP",
                _ => continue,
            };
            let mut tk = kind;
            if let Some(ref hard) = self.grid.col_hard {
                if hard.col == col {
                    tk = "PCIE_BRAM";
                }
            }
            'a: for row in self.die.rows() {
                if row.to_idx() % 5 != 0 {
                    continue;
                }
                for hole in &self.site_holes {
                    if hole.contains(col, row) {
                        continue 'a;
                    }
                }
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("{tk}_X{x}Y{y}");
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node(kind),
                    &[&name],
                    self.db.get_node_naming(kind),
                    &[
                        (col, row),
                        (col, row + 1),
                        (col, row + 2),
                        (col, row + 3),
                        (col, row + 4),
                    ],
                );
                if cd == ColumnKind::Bram {
                    node.add_bel(0, format!("RAMB36_X{bx}Y{sy}", sy = y / 5));
                } else {
                    node.add_bel(0, format!("DSP48_X{dx}Y{sy}", sy = y / 5 * 2));
                    node.add_bel(1, format!("DSP48_X{dx}Y{sy}", sy = y / 5 * 2 + 1));
                }
                if kind == "BRAM" && row.to_idx() % 20 == 10 {
                    if self.grid.cols_mgt_buf.contains(&col) {
                        let l = if col < self.col_cfg { "_LEFT" } else { "" };
                        let name_h = format!("HCLK_BRAM_MGT{l}_X{x}Y{y}", y = y - 1);
                        self.die[(col, row)].add_xnode(
                            self.db.get_node("HCLK_BRAM_MGT"),
                            &[&name_h],
                            self.db.get_node_naming("HCLK_BRAM_MGT"),
                            &[],
                        );
                    } else {
                        let name_h = format!("HCLK_{tk}_X{x}Y{y}", y = y - 1);
                        let node = self.die[(col, row)].add_xnode(
                            self.db.get_node("PMVBRAM"),
                            &[&name_h, &name],
                            self.db.get_node_naming("PMVBRAM"),
                            &[
                                (col, row),
                                (col, row + 1),
                                (col, row + 2),
                                (col, row + 3),
                                (col, row + 4),
                            ],
                        );
                        node.add_bel(0, format!("PMVBRAM_X{px}Y{sy}", sy = y / 20));
                    }
                }
            }
            if cd == ColumnKind::Bram {
                bx += 1;
                if !self.grid.cols_mgt_buf.contains(&col) {
                    px += 1;
                }
            } else {
                dx += 1;
            }
        }
    }

    fn fill_io(&mut self) {
        let row_ioi_cmt = if self.grid.reg_cfg.to_idx() == 1 {
            RowId::from_idx(0)
        } else {
            self.grid.row_bufg() - 30
        };
        let row_cmt_ioi = if self.grid.reg_cfg.to_idx() == self.grid.regs - 1 {
            RowId::from_idx(self.grid.regs * 20)
        } else {
            self.grid.row_bufg() + 30
        };
        let row_bot_cmt = if self.grid.reg_cfg.to_idx() < 3 {
            RowId::from_idx(0)
        } else {
            self.grid.row_bufg() - 60
        };
        let row_top_cmt = if (self.grid.regs - self.grid.reg_cfg.to_idx()) < 3 {
            RowId::from_idx(self.grid.regs * 20)
        } else {
            self.grid.row_bufg() + 60
        };
        let mut iox = 0;
        for (col, &cd) in &self.grid.columns {
            if !matches!(cd, ColumnKind::Io | ColumnKind::Cfg) {
                continue;
            }
            let mgt = if self.col_lgt.is_some() { "_MGT" } else { "" };
            let x = col.to_idx();
            let mut cmty = 0;
            for row in self.die.rows() {
                let y = row.to_idx();
                let is_cfg = col == self.col_cfg;
                if is_cfg && row >= self.grid.row_bufg() - 10 && row < self.grid.row_bufg() + 10 {
                    if row.to_idx() % 20 == 10 {
                        let ipx = usize::from(self.col_lgt.is_some());
                        let ipy = if !self.grid.cols_gt.is_empty() {
                            self.grid.reg_cfg.to_idx() * 6
                        } else {
                            0
                        };
                        let sysmon = SysMon {
                            die: self.die.die,
                            col,
                            row,
                            bank: 0,
                            pad_vp: format!("IPAD_X{ipx}Y{ipy}"),
                            pad_vn: format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 1),
                            vaux: [0, 1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 14, 18, 19]
                                .into_iter()
                                .map(|dy| {
                                    Some((
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
                                    ))
                                })
                                .collect(),
                        };
                        let rx = self.rxlut[col] + 3;
                        let ry = self.grid.reg_cfg.to_idx() * 22;
                        let name = format!("CFG_CENTER_X{rx}Y{ry}");
                        let name_bufg = format!("CLK_BUFGMUX_X{rx}Y{ry}");
                        let crds: [_; 20] = core::array::from_fn(|i| (col, row + i));
                        let node = self.die[(col, row)].add_xnode(
                            self.db.get_node("CFG"),
                            &[&name, &name_bufg],
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
                        node.add_bel(45, "KEY_CLEAR".to_string());
                        node.add_bel(46, "EFUSE_USR".to_string());
                        node.add_bel(47, "SYSMON_X0Y0".to_string());
                        node.add_bel(48, sysmon.pad_vp.clone());
                        node.add_bel(49, sysmon.pad_vn.clone());
                        self.sysmon.push(sysmon);
                    }
                } else if is_cfg
                    && ((row >= row_bot_cmt && row < row_ioi_cmt)
                        || (row >= row_cmt_ioi && row < row_top_cmt))
                {
                    if row.to_idx() % 10 == 0 {
                        let naming = if row.to_idx() % 20 == 0 {
                            "CMT_BOT"
                        } else {
                            "CMT_TOP"
                        };
                        let name = format!("CMT_X{x}Y{y}");
                        let crds: [_; 10] = core::array::from_fn(|i| (col, row + i));
                        let node = self.die[(col, row)].add_xnode(
                            self.db.get_node("CMT"),
                            &[&name],
                            self.db.get_node_naming(naming),
                            &crds,
                        );
                        node.add_bel(0, format!("DCM_ADV_X0Y{y}", y = cmty * 2));
                        node.add_bel(1, format!("DCM_ADV_X0Y{y}", y = cmty * 2 + 1));
                        node.add_bel(2, format!("PLL_ADV_X0Y{cmty}"));
                        cmty += 1;

                        let rx = self.rxlut[col] + 4;
                        let ry = y / 10 * 11 + 1;
                        let naming = if row < self.grid.row_bufg() {
                            "CLK_CMT_BOT"
                        } else {
                            "CLK_CMT_TOP"
                        };
                        let name = format!("{naming}{mgt}_X{rx}Y{ry}");
                        self.die[(col, row)].add_xnode(
                            self.db.get_node("CLK_CMT"),
                            &[&name],
                            self.db.get_node_naming(naming),
                            &[],
                        );
                    }
                } else {
                    let node = self.die[(col, row)].add_xnode(
                        self.db.get_node("IOI"),
                        &[&format!("IOI_X{x}Y{y}")],
                        self.db.get_node_naming("IOI"),
                        &[(col, row)],
                    );
                    node.add_bel(0, format!("ILOGIC_X{iox}Y{y}", y = y * 2 + 1));
                    node.add_bel(1, format!("ILOGIC_X{iox}Y{y}", y = y * 2));
                    node.add_bel(2, format!("OLOGIC_X{iox}Y{y}", y = y * 2 + 1));
                    node.add_bel(3, format!("OLOGIC_X{iox}Y{y}", y = y * 2));
                    node.add_bel(4, format!("IODELAY_X{iox}Y{y}", y = y * 2 + 1));
                    node.add_bel(5, format!("IODELAY_X{iox}Y{y}", y = y * 2));
                    let naming = match iox {
                        0 => {
                            if col.to_idx() == 0 {
                                "LIOB"
                            } else if row >= self.grid.row_bufg() + 10
                                && row < self.grid.row_bufg() + 20
                            {
                                "RIOB"
                            } else {
                                "LIOB_MON"
                            }
                        }
                        1 => "CIOB",
                        2 => "RIOB",
                        _ => unreachable!(),
                    };
                    let node = self.die[(col, row)].add_xnode(
                        self.db.get_node("IOB"),
                        &[&format!("{naming}_X{x}Y{y}")],
                        self.db.get_node_naming(naming),
                        &[],
                    );
                    let iob_name_p = format!("IOB_X{iox}Y{y}", y = y * 2 + 1);
                    let iob_name_n = format!("IOB_X{iox}Y{y}", y = y * 2);
                    node.add_bel(0, iob_name_p.clone());
                    node.add_bel(1, iob_name_n.clone());
                    let reg = self.grid.row_to_reg(row);
                    let rr = reg - self.grid.reg_cfg;
                    let bank = if col == self.col_cfg {
                        (if rr <= -4 {
                            6 + (-rr - 4) * 2
                        } else if rr >= 3 {
                            5 + (rr - 3) * 2
                        } else if rr < 0 {
                            2 + (-rr - 1) * 2
                        } else {
                            1 + rr * 2
                        }) as u32
                    } else {
                        (if rr < 0 {
                            13 + (-rr - 1) * 4
                        } else {
                            11 + rr * 4
                        }) as u32
                            + u32::from(self.col_rio == Some(col))
                    };
                    let is_short_bank = bank <= 4;
                    let biob = if is_short_bank {
                        2 * (row.to_idx() % 10) as u32
                    } else {
                        2 * (row.to_idx() % 20) as u32
                    };
                    let pkgid = if is_short_bank {
                        (18 - biob) / 2
                    } else {
                        (38 - biob) / 2
                    };
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
                    let is_gc = matches!(bank, 3 | 4);
                    let is_srcc = matches!(row.to_idx() % 20, 8..=11);
                    let is_vref = matches!(biob, 10 | 30);
                    let is_vr = match bank {
                        1 | 2 => false,
                        3 => biob == 14,
                        4 => biob == 4,
                        _ => biob == 14,
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
                            is_lc: false,
                            is_gc,
                            is_srcc,
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
                            is_lc: false,
                            is_gc,
                            is_srcc,
                            is_mrcc: false,
                            is_dqs: false,
                            is_vref,
                            vr: if is_vr { IoVrKind::VrP } else { IoVrKind::None },
                        },
                    ]);
                }

                if row.to_idx() % 20 == 10 {
                    let ry = y / 20;
                    if is_cfg {
                        let kind = if self.col_lgt.is_some() {
                            "CLK_HROW_MGT"
                        } else {
                            "CLK_HROW"
                        };
                        let name_hrow = format!("{kind}_X{x}Y{y}", y = y - 1);
                        self.die[(col, row)].add_xnode(
                            self.db.get_node("CLK_HROW"),
                            &[&name_hrow],
                            self.db.get_node_naming("CLK_HROW"),
                            &[],
                        );

                        if row == self.grid.row_bufg() - 10 {
                            let name = format!("HCLK_IOI_BOTCEN{mgt}_X{x}Y{y}", y = y - 1);
                            let name_i0 = format!("IOI_X{x}Y{y}", y = y - 2);
                            let name_i1 = format!("IOI_X{x}Y{y}", y = y - 1);
                            let node = self.die[(col, row)].add_xnode(
                                self.db.get_node("HCLK_IOI_BOTCEN"),
                                &[&name, &name_i0, &name_i1],
                                self.db.get_node_naming("HCLK_IOI_BOTCEN"),
                                &[(col, row - 2), (col, row - 1)],
                            );
                            node.add_bel(0, format!("BUFIO_X{iox}Y{y}", y = ry * 4 + 1));
                            node.add_bel(1, format!("BUFIO_X{iox}Y{y}", y = ry * 4));
                            node.add_bel(2, format!("IDELAYCTRL_X{iox}Y{ry}"));
                            node.add_bel(3, format!("DCI_X{iox}Y{ry}"));
                        } else if row == self.grid.row_bufg() + 10 {
                            let name = format!("HCLK_IOI_TOPCEN{mgt}_X{x}Y{y}", y = y - 1);
                            let name_i2 = format!("IOI_X{x}Y{y}");
                            let name_i3 = format!("IOI_X{x}Y{y}", y = y + 1);
                            let node = self.die[(col, row)].add_xnode(
                                self.db.get_node("HCLK_IOI_TOPCEN"),
                                &[&name, &name_i2, &name_i3],
                                self.db.get_node_naming("HCLK_IOI_TOPCEN"),
                                &[(col, row), (col, row + 1)],
                            );
                            node.add_bel(0, format!("BUFIO_X{iox}Y{y}", y = ry * 4 + 2));
                            node.add_bel(1, format!("BUFIO_X{iox}Y{y}", y = ry * 4 + 3));
                            node.add_bel(2, format!("IDELAYCTRL_X{iox}Y{ry}"));
                            node.add_bel(3, format!("DCI_X{iox}Y{ry}"));
                        } else if row == row_ioi_cmt {
                            let name = format!("HCLK_IOI_CMT{mgt}_X{x}Y{y}", y = y - 1);
                            let name_i2 = format!("IOI_X{x}Y{y}");
                            let name_i3 = format!("IOI_X{x}Y{y}", y = y + 1);
                            let node = self.die[(col, row)].add_xnode(
                                self.db.get_node("HCLK_IOI_CMT"),
                                &[&name, &name_i2, &name_i3],
                                self.db.get_node_naming("HCLK_IOI_CMT"),
                                &[(col, row), (col, row + 1)],
                            );
                            node.add_bel(0, format!("BUFIO_X{iox}Y{y}", y = ry * 4 + 2));
                            node.add_bel(1, format!("BUFIO_X{iox}Y{y}", y = ry * 4 + 3));
                            node.add_bel(2, format!("IDELAYCTRL_X{iox}Y{ry}"));
                            node.add_bel(3, format!("DCI_X{iox}Y{ry}"));

                            let name = format!("HCLK_IOB_CMT_BOT{mgt}_X{x}Y{y}", y = y - 1);
                            self.die[(col, row)].add_xnode(
                                self.db.get_node("HCLK_CMT"),
                                &[&name, &name_hrow],
                                self.db.get_node_naming("HCLK_CMT"),
                                &[],
                            );

                            let name = format!("CLK_IOB_B_X{x}Y{y}");
                            self.die[(col, row)].add_xnode(
                                self.db.get_node("CLK_IOB"),
                                &[&name],
                                self.db.get_node_naming("CLK_IOB_B"),
                                &[],
                            );
                        } else if row == row_cmt_ioi {
                            let name = format!("HCLK_CMT_IOI_X{x}Y{y}", y = y - 1);
                            let name_i0 = format!("IOI_X{x}Y{y}", y = y - 2);
                            let name_i1 = format!("IOI_X{x}Y{y}", y = y - 1);
                            let node = self.die[(col, row)].add_xnode(
                                self.db.get_node("HCLK_CMT_IOI"),
                                &[&name, &name_i0, &name_i1],
                                self.db.get_node_naming("HCLK_CMT_IOI"),
                                &[(col, row - 2), (col, row - 1)],
                            );
                            node.add_bel(0, format!("BUFIO_X{iox}Y{y}", y = ry * 4 + 1));
                            node.add_bel(1, format!("BUFIO_X{iox}Y{y}", y = ry * 4));
                            node.add_bel(2, format!("IDELAYCTRL_X{iox}Y{ry}"));
                            node.add_bel(3, format!("DCI_X{iox}Y{ry}"));

                            let name = format!("HCLK_IOB_CMT_TOP{mgt}_X{x}Y{y}", y = y - 1);
                            self.die[(col, row)].add_xnode(
                                self.db.get_node("HCLK_CMT"),
                                &[&name, &name_hrow],
                                self.db.get_node_naming("HCLK_CMT"),
                                &[],
                            );

                            let name = format!("CLK_IOB_T_X{x}Y{y}", y = y - 10);
                            self.die[(col, row - 10)].add_xnode(
                                self.db.get_node("CLK_IOB"),
                                &[&name],
                                self.db.get_node_naming("CLK_IOB_T"),
                                &[],
                            );
                        } else if (row >= row_bot_cmt && row < row_ioi_cmt)
                            || (row >= row_cmt_ioi && row < row_top_cmt)
                        {
                            let name = format!("HCLK_IOB_CMT_MID{mgt}_X{x}Y{y}", y = y - 1);
                            self.die[(col, row)].add_xnode(
                                self.db.get_node("HCLK_CMT"),
                                &[&name, &name_hrow],
                                self.db.get_node_naming("HCLK_CMT"),
                                &[],
                            );
                        } else {
                            let name = format!("HCLK_IOI_CENTER_X{x}Y{y}", y = y - 1);
                            let name_i0 = format!("IOI_X{x}Y{y}", y = y - 2);
                            let name_i1 = format!("IOI_X{x}Y{y}", y = y - 1);
                            let name_i2 = format!("IOI_X{x}Y{y}");
                            let node = self.die[(col, row)].add_xnode(
                                self.db.get_node("HCLK_IOI_CENTER"),
                                &[&name, &name_i0, &name_i1, &name_i2],
                                self.db.get_node_naming("HCLK_IOI_CENTER"),
                                &[(col, row - 2), (col, row - 1), (col, row)],
                            );
                            node.add_bel(0, format!("BUFIO_X{iox}Y{y}", y = ry * 4 + 2));
                            node.add_bel(1, format!("BUFIO_X{iox}Y{y}", y = ry * 4 + 3));
                            node.add_bel(2, format!("BUFIO_X{iox}Y{y}", y = ry * 4 + 1));
                            node.add_bel(3, format!("BUFIO_X{iox}Y{y}", y = ry * 4));
                            node.add_bel(4, format!("IDELAYCTRL_X{iox}Y{ry}"));
                            node.add_bel(5, format!("DCI_X{iox}Y{ry}"));

                            if row < self.grid.row_bufg() {
                                let name = format!("CLK_MGT_BOT{mgt}_X{x}Y{y}");
                                self.die[(col, row)].add_xnode(
                                    self.db.get_node("CLK_MGT"),
                                    &[&name],
                                    self.db.get_node_naming("CLK_MGT_BOT"),
                                    &[],
                                );
                            } else {
                                let name = format!("CLK_MGT_TOP{mgt}_X{x}Y{y}", y = y - 10);
                                self.die[(col, row - 10)].add_xnode(
                                    self.db.get_node("CLK_MGT"),
                                    &[&name],
                                    self.db.get_node_naming("CLK_MGT_TOP"),
                                    &[],
                                );
                            }
                        }
                    } else {
                        let name = format!("HCLK_IOI_X{x}Y{y}", y = y - 1);
                        let name_i0 = format!("IOI_X{x}Y{y}", y = y - 2);
                        let name_i1 = format!("IOI_X{x}Y{y}", y = y - 1);
                        let name_i2 = format!("IOI_X{x}Y{y}");
                        let name_i3 = format!("IOI_X{x}Y{y}", y = y + 1);
                        let node = self.die[(col, row)].add_xnode(
                            self.db.get_node("HCLK_IOI"),
                            &[&name, &name_i0, &name_i1, &name_i2, &name_i3],
                            self.db.get_node_naming("HCLK_IOI"),
                            &[(col, row - 2), (col, row - 1), (col, row), (col, row + 1)],
                        );
                        node.add_bel(0, format!("BUFIO_X{iox}Y{y}", y = ry * 4 + 2));
                        node.add_bel(1, format!("BUFIO_X{iox}Y{y}", y = ry * 4 + 3));
                        node.add_bel(2, format!("BUFIO_X{iox}Y{y}", y = ry * 4 + 1));
                        node.add_bel(3, format!("BUFIO_X{iox}Y{y}", y = ry * 4));
                        node.add_bel(4, format!("BUFR_X{x}Y{y}", x = iox / 2, y = ry * 2));
                        node.add_bel(5, format!("BUFR_X{x}Y{y}", x = iox / 2, y = ry * 2 + 1));
                        node.add_bel(6, format!("IDELAYCTRL_X{iox}Y{ry}"));
                        node.add_bel(7, format!("DCI_X{iox}Y{ry}"));
                    }
                }
            }
            iox += 1;
        }
    }

    fn fill_gt(&mut self) {
        let mut gtx = 0;
        for gtc in &self.grid.cols_gt {
            let col = gtc.col;
            let ipx = if col.to_idx() == 0 { 0 } else { gtx + 1 };
            for row in self.die.rows() {
                if row.to_idx() % 20 != 0 {
                    continue;
                }
                let reg = self.grid.row_to_reg(row);
                let bank = if reg < self.grid.reg_cfg {
                    113 + (self.grid.reg_cfg - reg - 1) * 4
                } else {
                    111 + (reg - self.grid.reg_cfg) * 4
                } as u32
                    + if col.to_idx() != 0 { 1 } else { 0 };
                let gty = reg.to_idx();
                let ipy = if gty < self.grid.reg_cfg.to_idx() {
                    gty * 6
                } else {
                    gty * 6 + 6
                };
                let gt = Gt {
                    die: self.die.die,
                    col,
                    row,
                    bank,
                    kind: gtc.regs[reg].unwrap(),
                    pads_clk: vec![(
                        format!("IPAD_X{ipx}Y{y}", y = ipy + 5),
                        format!("IPAD_X{ipx}Y{y}", y = ipy + 4),
                    )],
                    pads_rx: vec![
                        (
                            format!("IPAD_X{ipx}Y{y}", y = ipy + 1),
                            format!("IPAD_X{ipx}Y{ipy}"),
                        ),
                        (
                            format!("IPAD_X{ipx}Y{y}", y = ipy + 3),
                            format!("IPAD_X{ipx}Y{y}", y = ipy + 2),
                        ),
                    ],
                    pads_tx: vec![
                        (
                            format!("OPAD_X{gtx}Y{y}", y = gty * 4 + 1),
                            format!("OPAD_X{gtx}Y{y}", y = gty * 4),
                        ),
                        (
                            format!("OPAD_X{gtx}Y{y}", y = gty * 4 + 3),
                            format!("OPAD_X{gtx}Y{y}", y = gty * 4 + 2),
                        ),
                    ],
                };
                let (kind, naming) = match gt.kind {
                    GtKind::Gtp => ("GTP", "GT3"),
                    GtKind::Gtx => ("GTX", if col.to_idx() == 0 { "GTX_LEFT" } else { "GTX" }),
                    _ => continue,
                };
                let x = col.to_idx();
                let y = row.to_idx();
                let crds: [_; 20] = core::array::from_fn(|i| (col, row + i));
                let name = format!("{naming}_X{x}Y{y}", y = y + 9);
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node(kind),
                    &[&name],
                    self.db.get_node_naming(naming),
                    &crds,
                );
                node.add_bel(0, format!("{kind}_DUAL_X{gtx}Y{gty}"));
                node.add_bel(1, format!("BUFDS_X{gtx}Y{gty}"));
                node.add_bel(2, format!("CRC64_X{gtx}Y{y}", y = gty * 2));
                node.add_bel(3, format!("CRC64_X{gtx}Y{y}", y = gty * 2 + 1));
                node.add_bel(4, format!("CRC32_X{gtx}Y{y}", y = gty * 4));
                node.add_bel(5, format!("CRC32_X{gtx}Y{y}", y = gty * 4 + 1));
                node.add_bel(6, format!("CRC32_X{gtx}Y{y}", y = gty * 4 + 2));
                node.add_bel(7, format!("CRC32_X{gtx}Y{y}", y = gty * 4 + 3));
                node.add_bel(8, gt.pads_rx[0].0.clone());
                node.add_bel(9, gt.pads_rx[0].1.clone());
                node.add_bel(10, gt.pads_rx[1].0.clone());
                node.add_bel(11, gt.pads_rx[1].1.clone());
                node.add_bel(12, gt.pads_clk[0].0.clone());
                node.add_bel(13, gt.pads_clk[0].1.clone());
                node.add_bel(14, gt.pads_tx[0].0.clone());
                node.add_bel(15, gt.pads_tx[0].1.clone());
                node.add_bel(16, gt.pads_tx[1].0.clone());
                node.add_bel(17, gt.pads_tx[1].1.clone());
                self.gt.push(gt);
            }
            gtx += 1;
        }
    }

    fn fill_hclk(&mut self) {
        for col in self.die.cols() {
            'a: for row in self.die.rows() {
                let crow = self.grid.row_hclk(row);
                self.die[(col, row)].clkroot = (col, crow);

                if row.to_idx() % 20 == 10 {
                    for hole in &self.int_holes {
                        if hole.contains(col, row) {
                            continue 'a;
                        }
                    }
                    let x = col.to_idx();
                    let y = row.to_idx() - 1;
                    let reg = self.grid.row_to_reg(row);
                    let kind = match self.grid.columns[col] {
                        ColumnKind::Gt => {
                            let gtc = self.grid.cols_gt.iter().find(|gtc| gtc.col == col).unwrap();
                            match gtc.regs[reg].unwrap() {
                                GtKind::Gtp => "HCLK_GT3",
                                GtKind::Gtx => {
                                    if x == 0 {
                                        "HCLK_GTX_LEFT"
                                    } else {
                                        "HCLK_GTX"
                                    }
                                }
                                _ => unreachable!(),
                            }
                        }
                        _ => "HCLK",
                    };
                    let name = format!("{kind}_X{x}Y{y}");
                    let node = self.die[(col, row)].add_xnode(
                        self.db.get_node("HCLK"),
                        &[&name],
                        self.db.get_node_naming("HCLK"),
                        &[(col, row)],
                    );
                    node.add_bel(0, format!("GLOBALSIG_X{x}Y{y}", y = y / 20));
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
            self.frames.bram_frame.push(EntityPartVec::new());
            self.frames.spine_frame.push(0);
        }
        for &reg in &regs {
            let mut major = 0;
            for (col, cd) in &self.grid.columns {
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
                if col == self.col_cfg {
                    self.frames.spine_frame[reg] = self.frame_info.len();
                    for minor in 0..4 {
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
                self.frames.bram_frame[reg].insert(col, self.frame_info.len());
                for minor in 0..128 {
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
    assert_matches!(cols_io.len(), 1 | 2);
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
    let (_, die) = egrid.add_die(grid.columns.len(), grid.regs * 20);

    let mut expander = Expander {
        grid,
        db,
        die,
        int_holes: vec![],
        site_holes: vec![],
        rxlut: EntityVec::new(),
        frame_info: vec![],
        frames: DieFrameGeom {
            col_frame: EntityVec::new(),
            bram_frame: EntityVec::new(),
            spine_frame: EntityVec::new(),
        },
        col_cfg,
        col_lio: Some(cols_io[0]),
        col_rio: cols_io.get(1).copied(),
        col_lgt,
        io: vec![],
        gt: vec![],
        sysmon: vec![],
    };

    expander.fill_rxlut();
    expander.fill_int();
    expander.fill_ppc();
    expander.fill_terms();
    expander.fill_int_bufs();
    expander.die.fill_main_passes();
    expander.fill_clb();
    expander.fill_hard();
    expander.fill_bram_dsp();
    expander.fill_io();
    expander.fill_gt();
    expander.fill_hclk();
    expander.fill_frame_info();

    let frames = expander.frames;
    let io = expander.io;
    let gt = expander.gt;
    let sysmon = expander.sysmon;
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
    };

    let cfg_io = [
        (6, 1, SharedCfgPin::Data(8)),
        (6, 0, SharedCfgPin::Data(9)),
        (7, 1, SharedCfgPin::Data(10)),
        (7, 0, SharedCfgPin::Data(11)),
        (8, 1, SharedCfgPin::Data(12)),
        (8, 0, SharedCfgPin::Data(13)),
        (9, 1, SharedCfgPin::Data(14)),
        (9, 0, SharedCfgPin::Data(15)),
        (10, 1, SharedCfgPin::Data(0)),
        (10, 0, SharedCfgPin::Data(1)),
        (11, 1, SharedCfgPin::Data(2)),
        (11, 0, SharedCfgPin::Data(3)),
        (12, 1, SharedCfgPin::Data(4)),
        (12, 0, SharedCfgPin::Data(5)),
        (13, 1, SharedCfgPin::Data(6)),
        (13, 0, SharedCfgPin::Data(7)),
        (14, 1, SharedCfgPin::CsoB),
        (14, 0, SharedCfgPin::FweB),
        (15, 1, SharedCfgPin::FoeB),
        (15, 0, SharedCfgPin::FcsB),
        (16, 1, SharedCfgPin::Addr(20)),
        (16, 0, SharedCfgPin::Addr(21)),
        (17, 1, SharedCfgPin::Addr(22)),
        (17, 0, SharedCfgPin::Addr(23)),
        (18, 1, SharedCfgPin::Addr(24)),
        (18, 0, SharedCfgPin::Addr(25)),
        (19, 1, SharedCfgPin::Rs(0)),
        (19, 0, SharedCfgPin::Rs(1)),
        (40, 1, SharedCfgPin::Data(16)),
        (40, 0, SharedCfgPin::Data(17)),
        (41, 1, SharedCfgPin::Data(18)),
        (41, 0, SharedCfgPin::Data(19)),
        (42, 1, SharedCfgPin::Data(20)),
        (42, 0, SharedCfgPin::Data(21)),
        (43, 1, SharedCfgPin::Data(22)),
        (43, 0, SharedCfgPin::Data(23)),
        (44, 1, SharedCfgPin::Data(24)),
        (44, 0, SharedCfgPin::Data(25)),
        (45, 1, SharedCfgPin::Data(26)),
        (45, 0, SharedCfgPin::Data(27)),
        (46, 1, SharedCfgPin::Data(28)),
        (46, 0, SharedCfgPin::Data(29)),
        (47, 1, SharedCfgPin::Data(30)),
        (47, 0, SharedCfgPin::Data(31)),
        (48, 1, SharedCfgPin::Addr(16)),
        (48, 0, SharedCfgPin::Addr(17)),
        (49, 1, SharedCfgPin::Addr(18)),
        (49, 0, SharedCfgPin::Addr(19)),
    ]
    .into_iter()
    .map(|(dy, iob, pin)| {
        (
            pin,
            IoCoord {
                die: grid_master,
                col: col_cfg,
                row: grid.row_reg_bot(grid.reg_cfg) - 30 + dy,
                iob: TileIobId::from_idx(iob),
            },
        )
    })
    .collect();

    ExpandedDevice {
        kind: grid.kind,
        grids: grids.clone(),
        grid_master,
        extras: extras.to_vec(),
        disabled: disabled.clone(),
        egrid,
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
        gtz: vec![],
        sysmon,
        cfg_io,
        ps_io: Default::default(),
    }
}
