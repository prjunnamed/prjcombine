use prjcombine_entity::EntityId;
use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ExpandedDieRefMut, ExpandedGrid, Rect, RowId};

use crate::{ColumnKind, ExpandedDevice, Grid};

struct Expander<'a, 'b> {
    grid: &'b Grid,
    db: &'a IntDb,
    die: ExpandedDieRefMut<'a, 'b>,
    site_holes: Vec<Rect>,
    int_holes: Vec<Rect>,
    dciylut: Vec<usize>,
}

impl<'a, 'b> Expander<'a, 'b> {
    fn fill_dciylut(&mut self) {
        let mut dciy = 0;
        for i in 0..self.grid.regs {
            self.dciylut.push(dciy);
            let row = RowId::from_idx(i * 16 + 8);
            if i % 2 == 0 || (row >= self.grid.row_dcmiob() && row <= self.grid.row_iobdcm()) {
                dciy += 1;
            }
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
                    ColumnKind::Bram => {
                        let yy = y % 4;
                        let dy = y - yy;
                        tile.add_xnode(
                            self.db.get_node("INTF"),
                            &[&format!("BRAM_X{x}Y{dy}")],
                            self.db.get_node_naming(&format!("INTF.BRAM.{yy}")),
                            &[(col, row)],
                        );
                    }
                    ColumnKind::Dsp => {
                        let yy = y % 4;
                        let dy = y - yy;
                        tile.add_xnode(
                            self.db.get_node("INTF"),
                            &[&format!("DSP_X{x}Y{dy}")],
                            self.db.get_node_naming(&format!("INTF.DSP.{yy}")),
                            &[(col, row)],
                        );
                    }
                    _ => (),
                }
            }
        }
    }

    fn fill_lrio(&mut self) {
        for (brx, biox, col) in [(0, 0, self.grid.cols_io[0]), (1, 2, self.grid.cols_io[2])] {
            for row in self.die.rows() {
                let x = col.to_idx();
                let y = row.to_idx();
                let naming = match y % 16 {
                    7 | 8 => "IOIS_LC",
                    _ => "IOIS_NC",
                };
                let l = if col.to_idx() == 0 { "_L" } else { "" };
                self.die[(col, row)].add_xnode(
                    self.db.get_node("INTF"),
                    &[&format!("{naming}{l}_X{x}Y{y}")],
                    self.db.get_node_naming("INTF.IOIS"),
                    &[(col, row)],
                );
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node("IOIS"),
                    &[&format!("{naming}{l}_X{x}Y{y}")],
                    self.db.get_node_naming(naming),
                    &[(col, row)],
                );
                node.add_bel(0, format!("ILOGIC_X{biox}Y{y}", y = 2 * y + 1));
                node.add_bel(1, format!("ILOGIC_X{biox}Y{y}", y = 2 * y));
                node.add_bel(2, format!("OLOGIC_X{biox}Y{y}", y = 2 * y + 1));
                node.add_bel(3, format!("OLOGIC_X{biox}Y{y}", y = 2 * y));
                node.add_bel(4, format!("IOB_X{biox}Y{y}", y = 2 * y + 1));
                node.add_bel(5, format!("IOB_X{biox}Y{y}", y = 2 * y));

                if row.to_idx() % 32 == 8 {
                    let name = format!("HCLK_IOIS_DCI_X{x}Y{y}", y = y - 1);
                    let name_io0 = format!("IOIS_NC{l}_X{x}Y{y}", y = y - 2);
                    let name_io1 = format!("IOIS_LC{l}_X{x}Y{y}", y = y - 1);
                    let name_io2 = format!("IOIS_LC{l}_X{x}Y{y}");
                    let node = self.die[(col, row)].add_xnode(
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
                    let node = self.die[(col, row)].add_xnode(
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

    fn fill_cio(&mut self) {
        let mut dcmy = 0;
        let mut ccmy = 0;
        for row in self.die.rows() {
            let col = self.grid.cols_io[1];
            let x = col.to_idx();
            let y = row.to_idx();
            if row >= self.grid.row_dcmiob() && row < self.grid.row_iobdcm() {
                if row >= self.grid.row_cfg_below() && row < self.grid.row_cfg_above() {
                    // CFG
                    let dy = row.to_idx() - (self.grid.reg_cfg * 16 - 8);
                    let y = y - dy;
                    let name = format!("CFG_CENTER_X{x}Y{y}", y = y + 7);
                    self.die[(col, row)].add_xnode(
                        self.db.get_node("INTF"),
                        &[&name],
                        self.db.get_node_naming(&format!("INTF.CFG.{dy}")),
                        &[(col, row)],
                    );
                    if dy == 0 {
                        let name_bufg_b = format!("CLK_BUFGCTRL_B_X{x}Y{y}");
                        let name_bufg_t = format!("CLK_BUFGCTRL_T_X{x}Y{y}", y = y + 8);
                        let name_hrow_b = format!("CLK_HROW_X{x}Y{y}", y = y - 1);
                        let name_hrow_t = format!("CLK_HROW_X{x}Y{y}", y = y + 15);
                        let name_hclk_b = format!("HCLK_CENTER_X{x}Y{y}", y = y - 1);
                        let name_hclk_t = format!("HCLK_CENTER_ABOVE_CFG_X{x}Y{y}", y = y + 15);
                        let crds: [_; 16] = core::array::from_fn(|i| (col, row + i));
                        let node = self.die[(col, row)].add_xnode(
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
                } else {
                    // IO
                    self.die[(col, row)].add_xnode(
                        self.db.get_node("INTF"),
                        &[&format!("IOIS_LC_X{x}Y{y}")],
                        self.db.get_node_naming("INTF.IOIS"),
                        &[(col, row)],
                    );
                    let node = self.die[(col, row)].add_xnode(
                        self.db.get_node("IOIS"),
                        &[&format!("IOIS_LC_X{x}Y{y}")],
                        self.db.get_node_naming("IOIS_LC"),
                        &[(col, row)],
                    );
                    node.add_bel(0, format!("ILOGIC_X1Y{y}", y = 2 * y + 1));
                    node.add_bel(1, format!("ILOGIC_X1Y{y}", y = 2 * y));
                    node.add_bel(2, format!("OLOGIC_X1Y{y}", y = 2 * y + 1));
                    node.add_bel(3, format!("OLOGIC_X1Y{y}", y = 2 * y));
                    node.add_bel(4, format!("IOB_X1Y{y}", y = 2 * y + 1));
                    node.add_bel(5, format!("IOB_X1Y{y}", y = 2 * y));
                }
            } else if (self.grid.has_bot_sysmon && row < RowId::from_idx(8))
                || (self.grid.has_top_sysmon && row >= RowId::from_idx(self.grid.regs * 16 - 8))
            {
                // SYS_MON
                let dy = row.to_idx() % 8;
                let y = y - dy;
                let name = format!("SYS_MON_X{x}Y{y}");
                self.die[(col, row)].add_xnode(
                    self.db.get_node("INTF"),
                    &[&name],
                    self.db.get_node_naming(&format!("INTF.SYSMON.{dy}")),
                    &[(col, row)],
                );
                if dy == 0 {
                    let crds: [_; 8] = core::array::from_fn(|i| (col, row + i));
                    let node = self.die[(col, row)].add_xnode(
                        self.db.get_node("SYSMON"),
                        &[&name],
                        self.db.get_node_naming("SYSMON"),
                        &crds,
                    );
                    let my = if row.to_idx() == 0 { 0 } else { 1 };
                    let ipx = if self.grid.columns.first().unwrap() == &ColumnKind::Gt {
                        1
                    } else {
                        0
                    };
                    let ipy = if row.to_idx() == 0 {
                        0
                    } else if self.grid.columns.first().unwrap() == &ColumnKind::Gt {
                        self.grid.regs * 3
                    } else {
                        2
                    };
                    node.add_bel(0, format!("MONITOR_X0Y{my}"));
                    node.add_bel(1, format!("IPAD_X{ipx}Y{ipy}"));
                    node.add_bel(2, format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 1));
                }
            } else {
                // DCM or CCM
                let dy = row.to_idx() % 4;
                let y = y - dy;
                let (kind, tk) = if (row < self.grid.row_dcmiob()
                    && row >= self.grid.row_dcmiob() - self.grid.ccm * 4)
                    || (row >= self.grid.row_iobdcm()
                        && row < self.grid.row_iobdcm() + self.grid.ccm * 4)
                {
                    ("CCM", "CCM")
                } else if row < self.grid.row_dcmiob() {
                    ("DCM", "DCM_BOT")
                } else {
                    ("DCM", "DCM")
                };
                if kind == "DCM" && dy == 0 {
                    self.die[(col, row)].nodes[0].naming = self.db.get_node_naming("INT.DCM0");
                }
                let name = format!("{tk}_X{x}Y{y}");
                self.die[(col, row)].add_xnode(
                    self.db.get_node("INTF"),
                    &[&name],
                    self.db.get_node_naming(&format!("INTF.{kind}.{dy}")),
                    &[(col, row)],
                );
                if row.to_idx() % 4 == 0 {
                    let crds: [_; 4] = core::array::from_fn(|i| (col, row + i));
                    let node = self.die[(col, row)].add_xnode(
                        self.db.get_node(kind),
                        &[&name],
                        self.db.get_node_naming(tk),
                        &crds,
                    );
                    if kind == "DCM" {
                        node.add_bel(0, format!("DCM_ADV_X0Y{dcmy}"));
                        dcmy += 1;
                    } else {
                        node.add_bel(0, format!("PMCD_X0Y{y}", y = ccmy * 2));
                        node.add_bel(1, format!("PMCD_X0Y{y}", y = ccmy * 2 + 1));
                        node.add_bel(2, format!("DPM_X0Y{ccmy}"));
                        ccmy += 1;
                    }
                }
                if row.to_idx() % 8 == 0 {
                    let bt = if row < self.grid.row_dcmiob() {
                        'B'
                    } else {
                        'T'
                    };
                    let name = format!("CLKV_DCM_{bt}_X{x}Y{y}");
                    self.die[(col, row)].add_xnode(
                        self.db.get_node("CLK_DCM"),
                        &[&name],
                        self.db.get_node_naming("CLK_DCM"),
                        &[],
                    );
                }
            }

            if row.to_idx() % 16 == 8 {
                let name_hrow = format!("CLK_HROW_X{x}Y{y}", y = y - 1);
                self.die[(col, row)].add_xnode(
                    self.db.get_node("CLK_HROW"),
                    &[&name_hrow],
                    self.db.get_node_naming("CLK_HROW"),
                    &[],
                );

                let reg = row.to_idx() / 16;
                if row < self.grid.row_dcmiob() || row > self.grid.row_iobdcm() {
                    let name = format!("HCLK_DCM_X{x}Y{y}", y = y - 1);
                    self.die[(col, row)].add_xnode(
                        self.db.get_node("HCLK_DCM"),
                        &[&name, &name_hrow],
                        self.db.get_node_naming("HCLK_DCM"),
                        &[],
                    );
                } else if row == self.grid.row_dcmiob() {
                    let name = format!("HCLK_DCMIOB_X{x}Y{y}", y = y - 1);
                    let name_io0 = format!("IOIS_LC_X{x}Y{y}");
                    let name_io1 = format!("IOIS_LC_X{x}Y{y}", y = y + 1);
                    let node = self.die[(col, row)].add_xnode(
                        self.db.get_node("HCLK_DCMIOB"),
                        &[&name, &name_io0, &name_io1, &name_hrow],
                        self.db.get_node_naming("HCLK_DCMIOB"),
                        &[(col, row), (col, row + 1)],
                    );
                    node.add_bel(0, format!("BUFIO_X1Y{y}", y = reg * 2 + 1));
                    node.add_bel(1, format!("BUFIO_X1Y{y}", y = reg * 2));
                    node.add_bel(2, format!("IDELAYCTRL_X1Y{reg}"));
                    node.add_bel(3, format!("DCI_X1Y{y}", y = self.dciylut[reg]));
                } else if row == self.grid.row_iobdcm() {
                    let name = format!("HCLK_IOBDCM_X{x}Y{y}", y = y - 1);
                    let name_io0 = format!("IOIS_LC_X{x}Y{y}", y = y - 2);
                    let name_io1 = format!("IOIS_LC_X{x}Y{y}", y = y - 1);
                    let node = self.die[(col, row)].add_xnode(
                        self.db.get_node("HCLK_IOBDCM"),
                        &[&name, &name_io0, &name_io1, &name_hrow],
                        self.db.get_node_naming("HCLK_IOBDCM"),
                        &[(col, row - 2), (col, row - 1)],
                    );
                    node.add_bel(0, format!("BUFIO_X1Y{y}", y = reg * 2 + 1));
                    node.add_bel(1, format!("BUFIO_X1Y{y}", y = reg * 2));
                    node.add_bel(2, format!("IDELAYCTRL_X1Y{reg}"));
                    node.add_bel(3, format!("DCI_X1Y{y}", y = self.dciylut[reg]));
                } else if row == self.grid.row_cfg_above() {
                    let name = format!("HCLK_CENTER_ABOVE_CFG_X{x}Y{y}", y = y - 1);
                    let name_io0 = format!("IOIS_LC_X{x}Y{y}");
                    let name_io1 = format!("IOIS_LC_X{x}Y{y}", y = y + 1);
                    let node = self.die[(col, row)].add_xnode(
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
                    let node = self.die[(col, row)].add_xnode(
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
            let col = self.grid.cols_io[1];
            let row = self.grid.row_dcmiob();
            let x = col.to_idx();
            let y = row.to_idx();
            let name = format!("CLK_IOB_B_X{x}Y{y}", y = y + 7);
            self.die[(col, row)].add_xnode(
                self.db.get_node("CLK_IOB"),
                &[&name],
                self.db.get_node_naming("CLK_IOB"),
                &[],
            );
        }
        {
            let col = self.grid.cols_io[1];
            let row = self.grid.row_iobdcm() - 16;
            let x = col.to_idx();
            let y = row.to_idx();
            let name = format!("CLK_IOB_T_X{x}Y{y}", y = y + 7);
            self.die[(col, row)].add_xnode(
                self.db.get_node("CLK_IOB"),
                &[&name],
                self.db.get_node_naming("CLK_IOB"),
                &[],
            );
        }
    }

    fn fill_ppc(&mut self) {
        for (py, &(bc, br)) in self.grid.holes_ppc.iter().enumerate() {
            self.die.nuke_rect(bc + 1, br + 1, 7, 22);
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
                let tile_l = &mut self.die[(col_l, row)];
                tile_l.nodes.truncate(1);
                tile_l.add_xnode(
                    self.db.get_node("INTF"),
                    &[tile],
                    self.db.get_node_naming(&format!("INTF.PPC.L{dy}")),
                    &[(col_l, row)],
                );
                let tile_r = &mut self.die[(col_r, row)];
                tile_r.nodes.truncate(1);
                tile_r.add_xnode(
                    self.db.get_node("INTF"),
                    &[tile],
                    self.db.get_node_naming(&format!("INTF.PPC.R{dy}")),
                    &[(col_r, row)],
                );
            }
            for dx in 0..7 {
                let col = bc + dx + 1;
                let tile_b = &mut self.die[(col, row_b)];
                tile_b.nodes.truncate(1);
                tile_b.add_xnode(
                    self.db.get_node("INTF"),
                    &[&tile_pb],
                    self.db.get_node_naming(&format!("INTF.PPC.B{dx}")),
                    &[(col, row_b)],
                );
                let tile_t = &mut self.die[(col, row_t)];
                tile_t.nodes.truncate(1);
                tile_t.add_xnode(
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
            let node = self.die[(bc, br)].add_xnode(
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
                self.die[(col_l, row)].add_xnode(
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
                self.die[(col_r, row)].add_xnode(
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
            if cd != ColumnKind::Io || col == col_l || col == col_r {
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
            if cd != ColumnKind::Clb {
                continue;
            }
            'a: for row in self.die.rows() {
                let tile = &mut self.die[(col, row)];
                for hole in &self.site_holes {
                    if hole.contains(col, row) {
                        continue 'a;
                    }
                }
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("CLB_X{x}Y{y}");
                let node = tile.add_xnode(
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
            'a: for row in self.die.rows() {
                if row.to_idx() % 4 != 0 {
                    continue;
                }
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
            let gtx = if col.to_idx() == 0 { 0 } else { 1 };
            let ipx = if col.to_idx() == 0 {
                0
            } else if self.grid.has_bot_sysmon {
                2
            } else {
                1
            };
            let mut ipy = 0;
            if self.grid.has_bot_sysmon {
                ipy = 2;
            }
            for row in self.die.rows() {
                let y = row.to_idx();
                if row.to_idx() % 32 == 16 {
                    let node = self.die[(col, row)].add_xnode(
                        self.db.get_node("MGTCLK"),
                        &[&format!("BRKH_MGT11CLK_{lr}_X{x}Y{y}", y = y - 1)],
                        self.db.get_node_naming(&format!("BRKH_MGT11CLK_{lr}")),
                        &[],
                    );
                    let gty = y / 32;
                    node.add_bel(0, format!("GT11CLK_X{gtx}Y{gty}"));
                    node.add_bel(1, format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 1));
                    node.add_bel(2, format!("IPAD_X{ipx}Y{ipy}"));
                    ipy += 2;
                }
                if row.to_idx() % 16 == 0 {
                    let ab = if row.to_idx() % 32 == 0 { 'B' } else { 'A' };
                    let crds: [_; 16] = core::array::from_fn(|i| (col, row + i));
                    let node = self.die[(col, row)].add_xnode(
                        self.db.get_node("MGT"),
                        &[&format!("MGT_{ab}{lr}_X{x}Y{y}", y = y + 8)],
                        self.db.get_node_naming(&format!("MGT_{ab}{lr}")),
                        &crds,
                    );
                    let gty = y / 16;
                    node.add_bel(0, format!("GT11_X{gtx}Y{gty}"));
                    node.add_bel(1, format!("IPAD_X{ipx}Y{ipy}"));
                    node.add_bel(2, format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 1));
                    node.add_bel(3, format!("OPAD_X{gtx}Y{opy}", opy = gty * 2));
                    node.add_bel(4, format!("OPAD_X{gtx}Y{opy}", opy = gty * 2 + 1));
                    ipy += 2;
                }
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
                    let node = self.die[(col, row)].add_xnode(
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
}

impl Grid {
    pub fn expand_grid<'a>(&'a self, db: &'a IntDb) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        egrid.tie_kind = Some("TIEOFF".to_string());
        egrid.tie_pin_pullup = Some("KEEP1".to_string());
        egrid.tie_pin_gnd = Some("HARD0".to_string());
        egrid.tie_pin_vcc = Some("HARD1".to_string());
        let (_, die) = egrid.add_die(self.columns.len(), self.regs * 16);
        let mut expander = Expander {
            grid: self,
            db,
            die,
            int_holes: vec![],
            site_holes: vec![],
            dciylut: vec![],
        };

        expander.fill_dciylut();
        expander.fill_int();
        expander.fill_lrio();
        expander.fill_cio();
        expander.fill_ppc();
        expander.fill_term();
        expander.die.fill_main_passes();
        expander.fill_clb();
        expander.fill_bram_dsp();
        expander.fill_gt();
        expander.fill_hclk();

        ExpandedDevice { grid: self, egrid }
    }
}