use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};
use prjcombine_int::db::{BelId, IntDb};
use prjcombine_int::grid::{ColId, ExpandedDieRefMut, ExpandedGrid, RowId};
use std::collections::BTreeSet;

use crate::{DisabledPart, ExpandedDevice, Grid, GridKind};

struct Expander<'a, 'b> {
    grid: &'b Grid,
    db: &'a IntDb,
    disabled: &'b BTreeSet<DisabledPart>,
    die: ExpandedDieRefMut<'a, 'b>,
    clut: EntityPartVec<ColId, usize>,
    bramclut: EntityPartVec<ColId, usize>,
    rlut: EntityVec<RowId, usize>,
    cols_bram: Vec<ColId>,
    bonded_ios: Vec<((ColId, RowId), BelId)>,
}

impl<'a, 'b> Expander<'a, 'b> {
    fn fill_rlut(&mut self) {
        let n = self.grid.rows;
        for row in self.die.rows() {
            self.rlut.push(n - row.to_idx() - 1);
        }
    }

    fn fill_clut(&mut self) {
        let mut c = 0;
        let mut bramc = 0;
        for col in self.die.cols() {
            if self.grid.cols_bram.contains(&col) {
                self.bramclut.insert(col, bramc);
                bramc += 1;
            } else {
                self.clut.insert(col, c);
                c += 1;
            }
        }
    }

    fn fill_int(&mut self) {
        for col in self.die.cols() {
            if col == self.grid.col_lio() {
                let c = self.clut[col];
                for row in self.die.rows() {
                    if row == self.grid.row_bio() {
                        let node =
                            self.die
                                .fill_tile((col, row), "CNR.BL", "CNR.BL", "BL".to_string());
                        node.add_bel(0, "CAPTURE".to_string());
                    } else if row == self.grid.row_tio() {
                        let node =
                            self.die
                                .fill_tile((col, row), "CNR.TL", "CNR.TL", "TL".to_string());
                        node.add_bel(0, "STARTUP".to_string());
                        node.add_bel(1, "BSCAN".to_string());
                    } else {
                        let r = self.rlut[row];
                        let node = self
                            .die
                            .fill_tile((col, row), "IO.L", "IO.L", format!("LR{r}"));
                        node.add_bel(4, format!("TBUF_R{r}C{c}.1"));
                        node.add_bel(5, format!("TBUF_R{r}C{c}.0"));
                    }
                }
            } else if col == self.grid.col_rio() {
                let c = self.clut[col];
                for row in self.die.rows() {
                    if row == self.grid.row_bio() {
                        self.die
                            .fill_tile((col, row), "CNR.BR", "CNR.BR", "BR".to_string());
                    } else if row == self.grid.row_tio() {
                        self.die
                            .fill_tile((col, row), "CNR.TR", "CNR.TR", "TR".to_string());
                    } else {
                        let r = self.rlut[row];
                        let node = self
                            .die
                            .fill_tile((col, row), "IO.R", "IO.R", format!("RR{r}"));
                        node.add_bel(4, format!("TBUF_R{r}C{c}.0"));
                        node.add_bel(5, format!("TBUF_R{r}C{c}.1"));
                    }
                }
            } else if self.grid.cols_bram.contains(&col) {
                // skip for now
            } else {
                let c = self.clut[col];
                for row in self.die.rows() {
                    if row == self.grid.row_bio() {
                        self.die
                            .fill_tile((col, row), "IO.B", "IO.B", format!("BC{c}"));
                    } else if row == self.grid.row_tio() {
                        self.die
                            .fill_tile((col, row), "IO.T", "IO.T", format!("TC{c}"));
                    } else {
                        let r = self.rlut[row];
                        let node =
                            self.die
                                .fill_tile((col, row), "CLB", "CLB", format!("R{r}C{c}"));
                        node.add_bel(0, format!("CLB_R{r}C{c}.S0"));
                        node.add_bel(1, format!("CLB_R{r}C{c}.S1"));
                        if c % 2 == 1 {
                            node.add_bel(2, format!("TBUF_R{r}C{c}.0"));
                            node.add_bel(3, format!("TBUF_R{r}C{c}.1"));
                        } else {
                            node.add_bel(2, format!("TBUF_R{r}C{c}.1"));
                            node.add_bel(3, format!("TBUF_R{r}C{c}.0"));
                        }
                    }
                }
            }
        }
    }

    fn fill_io(&mut self) {
        let mut ctr_pad = 1;
        let mut ctr_empty = 1;
        for col in self.die.cols() {
            let row = self.grid.row_tio();
            if self.grid.cols_bram.contains(&col) {
                continue;
            }
            if col == self.grid.col_lio() || col == self.grid.col_rio() {
                continue;
            }
            let node = &mut self.die[(col, row)].nodes[0];
            node.add_bel(3, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
            node.add_bel(2, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            node.add_bel(1, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            node.add_bel(0, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
            self.bonded_ios.push(((col, row), BelId::from_idx(2)));
            self.bonded_ios.push(((col, row), BelId::from_idx(1)));
        }
        for row in self.die.rows().rev() {
            let col = self.grid.col_rio();
            if row == self.grid.row_bio() || row == self.grid.row_tio() {
                continue;
            }
            let node = &mut self.die[(col, row)].nodes[0];
            node.add_bel(0, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
            node.add_bel(1, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            node.add_bel(2, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            node.add_bel(3, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            self.bonded_ios.push(((col, row), BelId::from_idx(1)));
            self.bonded_ios.push(((col, row), BelId::from_idx(2)));
            self.bonded_ios.push(((col, row), BelId::from_idx(3)));
        }
        for col in self.die.cols().rev() {
            let row = self.grid.row_bio();
            if self.grid.cols_bram.contains(&col) {
                continue;
            }
            if col == self.grid.col_lio() || col == self.grid.col_rio() {
                continue;
            }
            let node = &mut self.die[(col, row)].nodes[0];
            node.add_bel(0, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
            node.add_bel(1, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            node.add_bel(2, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            node.add_bel(3, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
            self.bonded_ios.push(((col, row), BelId::from_idx(1)));
            self.bonded_ios.push(((col, row), BelId::from_idx(2)));
        }
        for row in self.die.rows() {
            let col = self.grid.col_lio();
            if row == self.grid.row_bio() || row == self.grid.row_tio() {
                continue;
            }
            let node = &mut self.die[(col, row)].nodes[0];
            node.add_bel(3, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            node.add_bel(2, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            node.add_bel(1, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            node.add_bel(0, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
            self.bonded_ios.push(((col, row), BelId::from_idx(3)));
            self.bonded_ios.push(((col, row), BelId::from_idx(2)));
            self.bonded_ios.push(((col, row), BelId::from_idx(1)));
        }
    }

    fn fill_bram(&mut self) {
        let mut bc = 0;
        let main_n = self.db.get_term("MAIN.N");
        let main_s = self.db.get_term("MAIN.S");
        let bram_mid = self.cols_bram.len() / 2;
        for (i, col) in self.cols_bram.iter().copied().enumerate() {
            if self.disabled.contains(&DisabledPart::Bram(col)) {
                continue;
            }

            let rt_b;
            let rt_t;
            if self.grid.kind == GridKind::Virtex {
                if col == self.grid.col_lio() + 1 {
                    rt_b = "LBRAM_BOT".to_string();
                    rt_t = "LBRAM_TOP".to_string();
                } else {
                    rt_b = "RBRAM_BOT".to_string();
                    rt_t = "RBRAM_TOP".to_string();
                }
            } else {
                rt_b = format!("BRAM_BOTC{i}");
                rt_t = format!("BRAM_TOPC{i}");
            }
            let naming_b;
            let naming_t;
            if i + 2 == bram_mid
                || i == bram_mid + 1
                || col == self.grid.col_lio() + 1
                || col == self.grid.col_rio() - 1
            {
                naming_b = "BRAM_BOT.BOT";
                naming_t = "BRAM_TOP.TOP";
            } else {
                naming_b = "BRAM_BOT.BOTP";
                naming_t = "BRAM_TOP.TOPP";
            }

            let row = self.grid.row_bio();
            self.die[(col, row)].add_xnode(
                self.db.get_node("BRAM_BOT"),
                &[&rt_b],
                self.db.get_node_naming(naming_b),
                &[(col, row), (col - 1, row)],
            );

            let mut prev_crd = (col, row);
            let mut prev_tile: Option<String> = None;
            for row in self.die.rows() {
                if row == self.grid.row_tio() || row.to_idx() % 4 != 1 {
                    continue;
                }
                let kind;
                let r = self.rlut[row];
                let mut tile = format!("BRAMR{r}C{i}");
                if col == self.grid.col_lio() + 1 {
                    kind = "LBRAM";
                    if self.grid.kind == GridKind::Virtex {
                        tile = format!("LBRAMR{r}");
                    }
                } else if col == self.grid.col_rio() - 1 {
                    kind = "RBRAM";
                    if self.grid.kind == GridKind::Virtex {
                        tile = format!("RBRAMR{r}");
                    }
                } else {
                    kind = "MBRAM";
                }
                let rts: Vec<&str> = if let Some(ref prev) = prev_tile {
                    vec![&tile, prev]
                } else {
                    vec![&tile]
                };
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node(kind),
                    &rts,
                    self.db.get_node_naming(kind),
                    &[
                        (col, row),
                        (col, row + 1),
                        (col, row + 2),
                        (col, row + 3),
                        (col - 1, row),
                        (col - 1, row + 1),
                        (col - 1, row + 2),
                        (col - 1, row + 3),
                    ],
                );
                let r = (self.grid.rows - 1 - row.to_idx() - 4) / 4;
                node.add_bel(0, format!("RAMB4_R{r}C{bc}"));
                self.die
                    .fill_term_pair_anon(prev_crd, (col, row), main_n, main_s);
                prev_crd = (col, row);
                prev_tile = Some(tile);
            }

            let row = self.grid.row_tio();
            self.die[(col, row)].add_xnode(
                self.db.get_node("BRAM_TOP"),
                &[&rt_t],
                self.db.get_node_naming(naming_t),
                &[(col, row), (col - 1, row)],
            );
            self.die
                .fill_term_pair_anon(prev_crd, (col, row), main_n, main_s);

            bc += 1;
        }
    }

    fn fill_clkbt(&mut self) {
        let row_b = self.grid.row_bio();
        let row_t = self.grid.row_tio();
        // CLKB/CLKT and DLLs
        if self.grid.kind == GridKind::Virtex {
            let col_c = self.grid.col_clk();
            let col_pl = self.grid.col_lio() + 1;
            let col_pr = self.grid.col_rio() - 1;
            let node = self.die[(col_c, row_b)].add_xnode(
                self.db.get_node("CLKB"),
                &["BM"],
                self.db.get_node_naming("CLKB"),
                &[(col_c, row_b), (col_pl, row_b), (col_pr, row_b)],
            );
            node.add_bel(0, "GCLKPAD0".to_string());
            node.add_bel(1, "GCLKPAD1".to_string());
            node.add_bel(2, "GCLKBUF0".to_string());
            node.add_bel(3, "GCLKBUF1".to_string());
            let node = self.die[(col_c, row_t)].add_xnode(
                self.db.get_node("CLKT"),
                &["TM"],
                self.db.get_node_naming("CLKT"),
                &[(col_c, row_t), (col_pl, row_t), (col_pr, row_t)],
            );
            node.add_bel(0, "GCLKPAD2".to_string());
            node.add_bel(1, "GCLKPAD3".to_string());
            node.add_bel(2, "GCLKBUF2".to_string());
            node.add_bel(3, "GCLKBUF3".to_string());
            let node = self.die[(col_pl, row_b)].add_xnode(
                self.db.get_node("DLL.BOT"),
                &["LBRAM_BOT", "BM"],
                self.db.get_node_naming("DLL.BL"),
                &[(col_pl, row_b), (col_pl - 1, row_b), (col_c, row_b)],
            );
            node.add_bel(0, "DLL1".to_string());
            let node = self.die[(col_pl, row_t)].add_xnode(
                self.db.get_node("DLL.TOP"),
                &["LBRAM_TOP", "TM"],
                self.db.get_node_naming("DLL.TL"),
                &[(col_pl, row_t), (col_pl - 1, row_t), (col_c, row_t)],
            );
            node.add_bel(0, "DLL3".to_string());
            let node = self.die[(col_pr, row_b)].add_xnode(
                self.db.get_node("DLL.BOT"),
                &["RBRAM_BOT", "BM"],
                self.db.get_node_naming("DLL.BR"),
                &[(col_pr, row_b), (col_pr - 1, row_b), (col_c, row_b)],
            );
            node.add_bel(0, "DLL0".to_string());
            let node = self.die[(col_pr, row_t)].add_xnode(
                self.db.get_node("DLL.TOP"),
                &["RBRAM_TOP", "TM"],
                self.db.get_node_naming("DLL.TR"),
                &[(col_pr, row_t), (col_pr - 1, row_t), (col_c, row_t)],
            );
            node.add_bel(0, "DLL2".to_string());
        } else {
            let col_c = self.grid.col_clk();
            let bram_mid = self.cols_bram.len() / 2;
            let c_pl = bram_mid - 1;
            let c_pr = bram_mid;
            let c_sl = bram_mid - 2;
            let c_sr = bram_mid + 1;
            let col_pl = self.cols_bram[c_pl];
            let col_pr = self.cols_bram[c_pr];
            let col_sl = self.cols_bram[c_sl];
            let col_sr = self.cols_bram[c_sr];
            let is_s_gclk = c_sl == 0;
            let kind_b;
            let kind_t;
            let s;
            if self.disabled.contains(&DisabledPart::PrimaryDlls) {
                kind_b = "CLKB_2DLL";
                kind_t = "CLKT_2DLL";
                s = "";
            } else {
                kind_b = "CLKB_4DLL";
                kind_t = "CLKT_4DLL";
                s = "S";
            }
            let node = self.die[(col_c, row_b)].add_xnode(
                self.db.get_node(kind_b),
                &["BM"],
                self.db.get_node_naming(kind_b),
                &[
                    (col_c, row_b),
                    (col_pl, row_b),
                    (col_pr, row_b),
                    (col_sl, row_b),
                    (col_sr, row_b),
                ],
            );
            node.add_bel(0, "GCLKPAD0".to_string());
            node.add_bel(1, "GCLKPAD1".to_string());
            node.add_bel(2, "GCLKBUF0".to_string());
            node.add_bel(3, "GCLKBUF1".to_string());
            let node = self.die[(col_c, row_t)].add_xnode(
                self.db.get_node(kind_t),
                &["TM"],
                self.db.get_node_naming(kind_t),
                &[
                    (col_c, row_t),
                    (col_pl, row_t),
                    (col_pr, row_t),
                    (col_sl, row_t),
                    (col_sr, row_t),
                ],
            );
            node.add_bel(0, "GCLKPAD2".to_string());
            node.add_bel(1, "GCLKPAD3".to_string());
            node.add_bel(2, "GCLKBUF2".to_string());
            node.add_bel(3, "GCLKBUF3".to_string());
            // DLLS
            let node = self.die[(col_sl, row_b)].add_xnode(
                self.db.get_node("DLLS.BOT"),
                &[&format!("BRAM_BOTC{c_sl}"), "BM"],
                self.db
                    .get_node_naming(if is_s_gclk { "DLLS.BL.GCLK" } else { "DLLS.BL" }),
                &[(col_sl, row_b), (col_sl - 1, row_b), (col_c, row_b)],
            );
            node.add_bel(0, format!("DLL1{s}"));
            let node = self.die[(col_sl, row_t)].add_xnode(
                self.db.get_node("DLLS.TOP"),
                &[&format!("BRAM_TOPC{c_sl}"), "TM"],
                self.db
                    .get_node_naming(if is_s_gclk { "DLLS.TL.GCLK" } else { "DLLS.TL" }),
                &[(col_sl, row_t), (col_sl - 1, row_t), (col_c, row_t)],
            );
            node.add_bel(0, format!("DLL3{s}"));
            let node = self.die[(col_sr, row_b)].add_xnode(
                self.db.get_node("DLLS.BOT"),
                &[&format!("BRAM_BOTC{c_sr}"), "BM"],
                self.db
                    .get_node_naming(if is_s_gclk { "DLLS.BR.GCLK" } else { "DLLS.BR" }),
                &[(col_sr, row_b), (col_sr - 1, row_b), (col_c, row_b)],
            );
            node.add_bel(0, format!("DLL0{s}"));
            let node = self.die[(col_sr, row_t)].add_xnode(
                self.db.get_node("DLLS.TOP"),
                &[&format!("BRAM_TOPC{c_sr}"), "TM"],
                self.db
                    .get_node_naming(if is_s_gclk { "DLLS.TR.GCLK" } else { "DLLS.TR" }),
                &[(col_sr, row_t), (col_sr - 1, row_t), (col_c, row_t)],
            );
            node.add_bel(0, format!("DLL2{s}"));
            if !self.disabled.contains(&DisabledPart::PrimaryDlls) {
                let node = self.die[(col_pl, row_b)].add_xnode(
                    self.db.get_node("DLLP.BOT"),
                    &[&format!("BRAM_BOTC{c_pl}"), "BM"],
                    self.db.get_node_naming("DLLP.BL"),
                    &[
                        (col_pl, row_b),
                        (col_pl - 1, row_b),
                        (col_c, row_b),
                        (col_sl, row_b),
                    ],
                );
                node.add_bel(0, "DLL1P".to_string());
                let node = self.die[(col_pl, row_t)].add_xnode(
                    self.db.get_node("DLLP.TOP"),
                    &[&format!("BRAM_TOPC{c_pl}"), "TM"],
                    self.db.get_node_naming("DLLP.TL"),
                    &[
                        (col_pl, row_t),
                        (col_pl - 1, row_t),
                        (col_c, row_t),
                        (col_sl, row_t),
                    ],
                );
                node.add_bel(0, "DLL3P".to_string());
                let node = self.die[(col_pr, row_b)].add_xnode(
                    self.db.get_node("DLLP.BOT"),
                    &[&format!("BRAM_BOTC{c_pr}"), "BM"],
                    self.db.get_node_naming("DLLP.BR"),
                    &[
                        (col_pr, row_b),
                        (col_pr - 1, row_b),
                        (col_c, row_b),
                        (col_sr, row_b),
                    ],
                );
                node.add_bel(0, "DLL0P".to_string());
                let node = self.die[(col_pr, row_t)].add_xnode(
                    self.db.get_node("DLLP.TOP"),
                    &[&format!("BRAM_TOPC{c_pr}"), "TM"],
                    self.db.get_node_naming("DLLP.TR"),
                    &[
                        (col_pr, row_t),
                        (col_pr - 1, row_t),
                        (col_c, row_t),
                        (col_sr, row_t),
                    ],
                );
                node.add_bel(0, "DLL2P".to_string());
            }
        }
    }

    fn fill_pcilogic(&mut self) {
        // CLKL/CLKR
        let pci_l = (self.grid.col_lio(), self.grid.row_clk());
        let pci_r = (self.grid.col_rio(), self.grid.row_clk());
        let node = self.die[pci_l].add_xnode(
            self.db.get_node("CLKL"),
            &["LM"],
            self.db.get_node_naming("CLKL"),
            &[pci_l],
        );
        node.add_bel(0, "LPCILOGIC".to_string());
        let node = self.die[pci_r].add_xnode(
            self.db.get_node("CLKR"),
            &["RM"],
            self.db.get_node_naming("CLKR"),
            &[pci_r],
        );
        node.add_bel(0, "RPCILOGIC".to_string());
    }

    fn fill_clk(&mut self) {
        let mut cc = 1;
        for &(col_m, col_l, col_r) in &self.grid.cols_clkv {
            for row in self.die.rows() {
                for c in col_l.to_idx()..col_m.to_idx() {
                    let col = ColId::from_idx(c);
                    self.die[(col, row)].clkroot = (col_m - 1, row);
                }
                if col_m == self.grid.col_lio() + 1 || col_m == self.grid.col_rio() - 1 {
                    let lr = if col_m == self.grid.col_lio() + 1 {
                        'L'
                    } else {
                        'R'
                    };
                    if row == self.grid.row_bio() {
                        for c in col_m.to_idx()..col_r.to_idx() {
                            let col = ColId::from_idx(c);
                            self.die[(col, row)].clkroot = (col_m, row);
                        }
                        let name = if self.grid.kind == GridKind::Virtex {
                            format!("{lr}BRAM_BOT")
                        } else {
                            let c = self.bramclut[col_m];
                            format!("BRAM_BOTC{c}")
                        };
                        self.die[(col_m, row)].add_xnode(
                            self.db.get_node("CLKV_BRAM_BOT"),
                            &[&name],
                            self.db.get_node_naming("CLKV_BRAM_BOT"),
                            &[(col_m, row), (col_m - 1, row), (col_m, row + 1)],
                        );
                    } else if row == self.grid.row_tio() {
                        for c in col_m.to_idx()..col_r.to_idx() {
                            let col = ColId::from_idx(c);
                            self.die[(col, row)].clkroot = (col_m, row);
                        }
                        let name = if self.grid.kind == GridKind::Virtex {
                            format!("{lr}BRAM_TOP")
                        } else {
                            let c = self.bramclut[col_m];
                            format!("BRAM_TOPC{c}")
                        };
                        self.die[(col_m, row)].add_xnode(
                            self.db.get_node("CLKV_BRAM_TOP"),
                            &[&name],
                            self.db.get_node_naming("CLKV_BRAM_TOP"),
                            &[(col_m, row), (col_m - 1, row), (col_m, row - 4)],
                        );
                    } else {
                        self.die[(col_m, row)].clkroot = (col_m, self.grid.row_clk());
                        for c in (col_m.to_idx() + 1)..col_r.to_idx() {
                            let col = ColId::from_idx(c);
                            self.die[(col, row)].clkroot = (col_m + 1, row);
                        }
                        if row.to_idx() % 4 == 1 {
                            let name = if self.grid.kind == GridKind::Virtex {
                                format!("{lr}BRAMR{r}", r = self.rlut[row])
                            } else {
                                let c = self.bramclut[col_m];
                                format!("BRAMR{r}C{c}", r = self.rlut[row])
                            };
                            self.die[(col_m, row)].add_xnode(
                                self.db.get_node("CLKV_BRAM"),
                                &[&name],
                                self.db.get_node_naming(if lr == 'L' {
                                    "CLKV_BRAM.L"
                                } else {
                                    "CLKV_BRAM.R"
                                }),
                                &[
                                    (col_m, row),
                                    (col_m - 1, row),
                                    (col_m - 1, row + 1),
                                    (col_m - 1, row + 2),
                                    (col_m - 1, row + 3),
                                    (col_m + 1, row),
                                    (col_m + 1, row + 1),
                                    (col_m + 1, row + 2),
                                    (col_m + 1, row + 3),
                                ],
                            );
                        }
                    }
                } else {
                    for c in col_m.to_idx()..col_r.to_idx() {
                        let col = ColId::from_idx(c);
                        self.die[(col, row)].clkroot = (col_m, row);
                    }
                    let (name, naming) = if col_m == self.grid.col_clk() {
                        if row == self.grid.row_bio() {
                            ("BM".to_string(), "CLKV.CLKB")
                        } else if row == self.grid.row_tio() {
                            ("TM".to_string(), "CLKV.CLKT")
                        } else {
                            (format!("VMR{r}", r = self.rlut[row]), "CLKV.CLKV")
                        }
                    } else {
                        if row == self.grid.row_bio() {
                            (format!("GCLKBC{cc}"), "CLKV.GCLKB")
                        } else if row == self.grid.row_tio() {
                            (format!("GCLKTC{cc}"), "CLKV.GCLKT")
                        } else {
                            (format!("GCLKVR{r}C{cc}", r = self.rlut[row]), "CLKV.GCLKV")
                        }
                    };
                    self.die[(col_m, row)].add_xnode(
                        self.db.get_node("CLKV"),
                        &[&name],
                        self.db.get_node_naming(naming),
                        &[(col_m - 1, row), (col_m, row)],
                    );
                }
            }
            if col_m == self.grid.col_lio() + 1 || col_m == self.grid.col_rio() - 1 {
                let name = if self.grid.kind == GridKind::Virtex {
                    if col_m == self.grid.col_lio() + 1 {
                        "LBRAMM".to_string()
                    } else {
                        "RBRAMM".to_string()
                    }
                } else {
                    let c = self.bramclut[col_m];
                    format!("BRAMMC{c}")
                };
                self.die[(col_m, self.grid.row_clk())].add_xnode(
                    self.db.get_node("BRAM_CLKH"),
                    &[&name],
                    self.db.get_node_naming("BRAM_CLKH"),
                    &[(col_m, self.grid.row_clk())],
                );
            } else if col_m == self.grid.col_clk() {
                self.die[(col_m, self.grid.row_clk())].add_xnode(
                    self.db.get_node("CLKC"),
                    &["M"],
                    self.db.get_node_naming("CLKC"),
                    &[],
                );
            } else {
                let name = format!("GCLKCC{cc}");
                self.die[(col_m, self.grid.row_clk())].add_xnode(
                    self.db.get_node("GCLKC"),
                    &[&name],
                    self.db.get_node_naming("GCLKC"),
                    &[],
                );
                cc += 1;
            }
        }
    }
}

impl Grid {
    pub fn expand_grid<'a>(
        &'a self,
        disabled: &BTreeSet<DisabledPart>,
        db: &'a IntDb,
    ) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        let (_, die) = egrid.add_die(self.columns, self.rows);

        let mut expander = Expander {
            grid: self,
            db,
            die,
            disabled,
            bonded_ios: vec![],
            clut: EntityPartVec::new(),
            bramclut: EntityPartVec::new(),
            rlut: EntityVec::new(),
            cols_bram: self.cols_bram.iter().copied().collect(),
        };
        expander.fill_clut();
        expander.fill_rlut();
        expander.fill_int();
        expander.die.fill_main_passes();
        expander.fill_io();
        expander.fill_bram();
        expander.fill_clkbt();
        expander.fill_pcilogic();
        expander.fill_clk();
        let bonded_ios = expander.bonded_ios;

        ExpandedDevice {
            grid: self,
            egrid,
            bonded_ios,
        }
    }
}