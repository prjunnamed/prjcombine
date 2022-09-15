use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ColId, ExpandedDieRefMut, ExpandedGrid, Rect};

use crate::{ColumnKind, ExpandedDevice, Grid};

struct Expander<'a, 'b> {
    grid: &'b Grid,
    db: &'a IntDb,
    die: ExpandedDieRefMut<'a, 'b>,
    site_holes: Vec<Rect>,
    int_holes: Vec<Rect>,
    rxlut: EntityVec<ColId, usize>,
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
                    } else if col == self.grid.cols_io[1].unwrap() {
                        7
                    } else {
                        6
                    }
                }
                ColumnKind::Gtp | ColumnKind::Gtx => 4,
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
                    ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::Io => {
                        tile.add_xnode(
                            self.db.get_node("INTF"),
                            &[&format!("INT_INTERFACE_X{x}Y{y}")],
                            self.db.get_node_naming("INTF"),
                            &[(col, row)],
                        );
                    }
                    ColumnKind::Gtp | ColumnKind::Gtx if col.to_idx() != 0 => {
                        tile.add_xnode(
                            self.db.get_node("INTF.DELAY"),
                            &[&format!("GTP_INT_INTERFACE_X{x}Y{y}")],
                            self.db.get_node_naming("INTF.GTP"),
                            &[(col, row)],
                        );
                    }
                    ColumnKind::Gtp | ColumnKind::Gtx => {
                        tile.add_xnode(
                            self.db.get_node("INTF.DELAY"),
                            &[&format!("GTX_LEFT_INT_INTERFACE_X{x}Y{y}")],
                            self.db.get_node_naming("INTF.GTX_LEFT"),
                            &[(col, row)],
                        );
                    }
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
            if self.grid.columns[col_l] == ColumnKind::Gtx {
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
            if matches!(self.grid.columns[col_r], ColumnKind::Gtp | ColumnKind::Gtx) {
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
            if cd != ColumnKind::Io || col == col_l || col == col_r {
                continue;
            }
            for row in self.die.rows() {
                let x = col.to_idx();
                let y = row.to_idx();
                let tile_l = format!("INT_BUFS_L_X{x}Y{y}");
                let mon = if self.grid.columns[col_l] == ColumnKind::Gtx {
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
                        let l = if col < self.grid.cols_io[1].unwrap() {
                            "_LEFT"
                        } else {
                            ""
                        };
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
        for (iox, col) in self.grid.cols_io.iter().enumerate() {
            let mgt = if self.grid.has_left_gt() { "_MGT" } else { "" };
            let col = if let &Some(c) = col { c } else { continue };
            let x = col.to_idx();
            let mut cmty = 0;
            for row in self.die.rows() {
                let y = row.to_idx();
                let is_cfg = col == self.grid.cols_io[1].unwrap();
                if is_cfg && row >= self.grid.row_botcen() && row < self.grid.row_topcen() {
                    if row.to_idx() % 20 == 10 {
                        let rx = self.rxlut[col] + 3;
                        let ry = self.grid.reg_cfg * 22;
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
                        let ipx = if self.grid.has_left_gt() { 1 } else { 0 };
                        let ipy = if self.grid.has_gt() {
                            self.grid.reg_cfg * 6
                        } else {
                            0
                        };
                        node.add_bel(48, format!("IPAD_X{ipx}Y{ipy}"));
                        node.add_bel(49, format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 1));
                    }
                } else if is_cfg
                    && ((row >= self.grid.row_bot_cmt() && row < self.grid.row_ioi_cmt())
                        || (row >= self.grid.row_cmt_ioi() && row < self.grid.row_top_cmt()))
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
                        let naming = if row < self.grid.row_botcen() {
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
                            } else if row >= self.grid.row_topcen()
                                && row < self.grid.row_topcen() + 10
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
                    node.add_bel(0, format!("IOB_X{iox}Y{y}", y = y * 2 + 1));
                    node.add_bel(1, format!("IOB_X{iox}Y{y}", y = y * 2));
                }

                if row.to_idx() % 20 == 10 {
                    let ry = y / 20;
                    if is_cfg {
                        let kind = if self.grid.has_left_gt() {
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

                        if row == self.grid.row_botcen() {
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
                        } else if row == self.grid.row_topcen() {
                            let name = format!("HCLK_IOI_TOPCEN{mgt}_X{x}Y{y}", y = y - 1);
                            let name_i2 = format!("IOI_X{x}Y{y}", y = y);
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
                        } else if row == self.grid.row_ioi_cmt() {
                            let name = format!("HCLK_IOI_CMT{mgt}_X{x}Y{y}", y = y - 1);
                            let name_i2 = format!("IOI_X{x}Y{y}", y = y);
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
                        } else if row == self.grid.row_cmt_ioi() {
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
                        } else if (row >= self.grid.row_bot_cmt() && row < self.grid.row_ioi_cmt())
                            || (row >= self.grid.row_cmt_ioi() && row < self.grid.row_top_cmt())
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
                            let name_i2 = format!("IOI_X{x}Y{y}", y = y);
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

                            if row < self.grid.row_botcen() {
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
                        let name_i2 = format!("IOI_X{x}Y{y}", y = y);
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
        }
    }

    fn fill_gt(&mut self) {
        let mut gtx = 0;
        for (col, &cd) in &self.grid.columns {
            let (kind, naming) = match cd {
                ColumnKind::Gtp => ("GTP", "GT3"),
                ColumnKind::Gtx => ("GTX", if col.to_idx() == 0 { "GTX_LEFT" } else { "GTX" }),
                _ => continue,
            };
            let ipx = if col.to_idx() == 0 { 0 } else { gtx + 1 };
            for row in self.die.rows() {
                if row.to_idx() % 20 != 0 {
                    continue;
                }
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
                let gty = row.to_idx() / 20;
                let ipy = if gty < self.grid.reg_cfg {
                    gty * 6
                } else {
                    gty * 6 + 6
                };
                node.add_bel(0, format!("{kind}_DUAL_X{gtx}Y{gty}"));
                node.add_bel(1, format!("BUFDS_X{gtx}Y{gty}"));
                node.add_bel(2, format!("CRC64_X{gtx}Y{y}", y = gty * 2));
                node.add_bel(3, format!("CRC64_X{gtx}Y{y}", y = gty * 2 + 1));
                node.add_bel(4, format!("CRC32_X{gtx}Y{y}", y = gty * 4));
                node.add_bel(5, format!("CRC32_X{gtx}Y{y}", y = gty * 4 + 1));
                node.add_bel(6, format!("CRC32_X{gtx}Y{y}", y = gty * 4 + 2));
                node.add_bel(7, format!("CRC32_X{gtx}Y{y}", y = gty * 4 + 3));
                node.add_bel(8, format!("IPAD_X{ipx}Y{y}", y = ipy + 1));
                node.add_bel(9, format!("IPAD_X{ipx}Y{y}", y = ipy));
                node.add_bel(10, format!("IPAD_X{ipx}Y{y}", y = ipy + 3));
                node.add_bel(11, format!("IPAD_X{ipx}Y{y}", y = ipy + 2));
                node.add_bel(12, format!("IPAD_X{ipx}Y{y}", y = ipy + 5));
                node.add_bel(13, format!("IPAD_X{ipx}Y{y}", y = ipy + 4));
                node.add_bel(14, format!("OPAD_X{gtx}Y{y}", y = gty * 4 + 1));
                node.add_bel(15, format!("OPAD_X{gtx}Y{y}", y = gty * 4));
                node.add_bel(16, format!("OPAD_X{gtx}Y{y}", y = gty * 4 + 3));
                node.add_bel(17, format!("OPAD_X{gtx}Y{y}", y = gty * 4 + 2));
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
                    let kind = match self.grid.columns[col] {
                        ColumnKind::Gtp => "HCLK_GT3",
                        ColumnKind::Gtx => {
                            if x == 0 {
                                "HCLK_GTX_LEFT"
                            } else {
                                "HCLK_GTX"
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
}

impl Grid {
    pub fn expand_grid<'a>(&'a self, db: &'a IntDb) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        egrid.tie_kind = Some("TIEOFF".to_string());
        egrid.tie_pin_pullup = Some("KEEP1".to_string());
        egrid.tie_pin_gnd = Some("HARD0".to_string());
        egrid.tie_pin_vcc = Some("HARD1".to_string());
        let (_, die) = egrid.add_die(self.columns.len(), self.regs * 20);

        let mut expander = Expander {
            grid: self,
            db,
            die,
            int_holes: vec![],
            site_holes: vec![],
            rxlut: EntityVec::new(),
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

        ExpandedDevice { grid: self, egrid }
    }
}
