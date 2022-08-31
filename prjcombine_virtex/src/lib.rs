#![allow(clippy::collapsible_else_if)]

use prjcombine_entity::{EntityId, EntityIds, EntityPartVec};
use prjcombine_int::db::{BelId, BelInfo, BelNaming, IntDb};
use prjcombine_int::grid::{ColId, Coord, DieId, ExpandedGrid, ExpandedTileNode, RowId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GridKind {
    Virtex,
    VirtexE,
    VirtexEM,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    Data(u8), // ×8
    CsB,
    InitB,
    RdWrB,
    Dout,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub kind: GridKind,
    pub columns: usize,
    pub cols_bram: BTreeSet<ColId>,
    pub cols_clkv: Vec<(ColId, ColId, ColId)>,
    pub rows: usize,
    pub vref: BTreeSet<IoCoord>,
    pub cfg_io: BTreeMap<SharedCfgPin, IoCoord>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct IoCoord {
    pub col: ColId,
    pub row: RowId,
    pub bel: BelId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CfgPin {
    Tck,
    Tdi,
    Tdo,
    Tms,
    Cclk,
    Done,
    ProgB,
    M0,
    M1,
    M2,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BondPin {
    Clk(u32),
    Io(IoCoord),
    Nc,
    Gnd,
    VccInt,
    VccAux,
    VccO(u32),
    Cfg(CfgPin),
    Dxn,
    Dxp,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
    // device bank -> pkg bank
    pub io_banks: BTreeMap<u32, u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DisabledPart {
    // Virtex-E: primary DLLs are disabled
    PrimaryDlls,
    // Virtex-E: a BRAM column is disabled
    Bram(ColId),
}

#[derive(Copy, Clone, Debug)]
pub struct Io<'a> {
    pub bank: u32,
    pub coord: IoCoord,
    pub name: &'a str,
    pub is_vref: bool,
}

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub egrid: ExpandedGrid<'a>,
    pub bonded_ios: Vec<((ColId, RowId), BelId)>,
}

impl Grid {
    pub fn row_mid(&self) -> RowId {
        RowId::from_idx(self.rows / 2)
    }

    pub fn row_clk(&self) -> RowId {
        match self.rows % 8 {
            2 => RowId::from_idx(self.rows / 2),
            6 => RowId::from_idx(self.rows / 2 - 2),
            _ => unreachable!(),
        }
    }

    pub fn col_clk(&self) -> ColId {
        ColId::from_idx(self.columns / 2)
    }

    pub fn col_lio(&self) -> ColId {
        ColId::from_idx(0)
    }

    pub fn col_rio(&self) -> ColId {
        ColId::from_idx(self.columns - 1)
    }

    pub fn row_bio(&self) -> RowId {
        RowId::from_idx(0)
    }

    pub fn row_tio(&self) -> RowId {
        RowId::from_idx(self.rows - 1)
    }

    pub fn columns(&self) -> EntityIds<ColId> {
        EntityIds::new(self.columns)
    }

    pub fn rows(&self) -> EntityIds<RowId> {
        EntityIds::new(self.rows)
    }

    pub fn expand_grid<'a>(
        &'a self,
        disabled: &BTreeSet<DisabledPart>,
        db: &'a IntDb,
    ) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        let (_, mut grid) = egrid.add_die(self.columns, self.rows);
        let mut bramclut = EntityPartVec::new();

        let col_l = grid.cols().next().unwrap();
        let col_r = grid.cols().next_back().unwrap();
        let row_b = grid.rows().next().unwrap();
        let row_t = grid.rows().next_back().unwrap();
        let mut c = 0;
        let mut bramc = 0;
        for col in grid.cols() {
            if col == col_l {
                for row in grid.rows() {
                    if row == row_b {
                        let node = grid.fill_tile((col, row), "CNR.BL", "CNR.BL", "BL".to_string());
                        node.add_bel(0, "CAPTURE".to_string());
                    } else if row == row_t {
                        let node = grid.fill_tile((col, row), "CNR.TL", "CNR.TL", "TL".to_string());
                        node.add_bel(0, "STARTUP".to_string());
                        node.add_bel(1, "BSCAN".to_string());
                    } else {
                        let r = row_t.to_idx() - row.to_idx();
                        let node = grid.fill_tile((col, row), "IO.L", "IO.L", format!("LR{r}"));
                        node.add_bel(4, format!("TBUF_R{r}C{c}.1"));
                        node.add_bel(5, format!("TBUF_R{r}C{c}.0"));
                    }
                }
                c += 1;
            } else if col == col_r {
                for row in grid.rows() {
                    if row == row_b {
                        grid.fill_tile((col, row), "CNR.BR", "CNR.BR", "BR".to_string());
                    } else if row == row_t {
                        grid.fill_tile((col, row), "CNR.TR", "CNR.TR", "TR".to_string());
                    } else {
                        let r = row_t.to_idx() - row.to_idx();
                        let node = grid.fill_tile((col, row), "IO.R", "IO.R", format!("RR{r}"));
                        node.add_bel(4, format!("TBUF_R{r}C{c}.0"));
                        node.add_bel(5, format!("TBUF_R{r}C{c}.1"));
                    }
                }
                c += 1;
            } else if self.cols_bram.contains(&col) {
                bramclut.insert(col, bramc);
                bramc += 1;
                // skip for now
            } else {
                for row in grid.rows() {
                    if row == row_b {
                        grid.fill_tile((col, row), "IO.B", "IO.B", format!("BC{c}"));
                    } else if row == row_t {
                        grid.fill_tile((col, row), "IO.T", "IO.T", format!("TC{c}"));
                    } else {
                        let r = row_t.to_idx() - row.to_idx();
                        let node = grid.fill_tile((col, row), "CLB", "CLB", format!("R{r}C{c}"));
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
                c += 1;
            }
        }
        grid.fill_main_passes();

        // IO fill
        let mut bonded_ios = vec![];
        let mut ctr_pad = 1;
        let mut ctr_empty = 1;
        for col in grid.cols() {
            if self.cols_bram.contains(&col) {
                continue;
            }
            if col == col_l || col == col_r {
                continue;
            }
            let node = &mut grid[(col, row_t)].nodes[0];
            node.add_bel(3, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
            node.add_bel(2, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            node.add_bel(1, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            node.add_bel(0, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
            bonded_ios.push(((col, row_t), BelId::from_idx(2)));
            bonded_ios.push(((col, row_t), BelId::from_idx(1)));
        }
        for row in grid.rows().rev() {
            if row == row_b || row == row_t {
                continue;
            }
            let node = &mut grid[(col_r, row)].nodes[0];
            node.add_bel(0, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
            node.add_bel(1, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            node.add_bel(2, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            node.add_bel(3, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            bonded_ios.push(((col_r, row), BelId::from_idx(1)));
            bonded_ios.push(((col_r, row), BelId::from_idx(2)));
            bonded_ios.push(((col_r, row), BelId::from_idx(3)));
        }
        for col in grid.cols().rev() {
            if self.cols_bram.contains(&col) {
                continue;
            }
            if col == col_l || col == col_r {
                continue;
            }
            let node = &mut grid[(col, row_b)].nodes[0];
            node.add_bel(0, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
            node.add_bel(1, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            node.add_bel(2, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            node.add_bel(3, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
            bonded_ios.push(((col, row_b), BelId::from_idx(1)));
            bonded_ios.push(((col, row_b), BelId::from_idx(2)));
        }
        for row in grid.rows() {
            if row == row_b || row == row_t {
                continue;
            }
            let node = &mut grid[(col_l, row)].nodes[0];
            node.add_bel(3, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            node.add_bel(2, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            node.add_bel(1, format!("PAD{ctr_pad}"));
            ctr_pad += 1;
            node.add_bel(0, format!("EMPTY{ctr_empty}"));
            ctr_empty += 1;
            bonded_ios.push(((col_l, row), BelId::from_idx(3)));
            bonded_ios.push(((col_l, row), BelId::from_idx(2)));
            bonded_ios.push(((col_l, row), BelId::from_idx(1)));
        }

        let main_n = db.get_term("MAIN.N");
        let main_s = db.get_term("MAIN.S");
        let cols_bram: Vec<_> = self.cols_bram.iter().copied().collect();
        let bram_mid = cols_bram.len() / 2;
        let mut c = 0;
        for (i, col) in cols_bram.iter().copied().enumerate() {
            if disabled.contains(&DisabledPart::Bram(col)) {
                continue;
            }

            let rt_b;
            let rt_t;
            if self.kind == GridKind::Virtex {
                if col == col_l + 1 {
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
            if i == bram_mid - 2 || i == bram_mid + 1 || col == col_l + 1 || col == col_r - 1 {
                naming_b = "BRAM_BOT.BOT";
                naming_t = "BRAM_TOP.TOP";
            } else {
                naming_b = "BRAM_BOT.BOTP";
                naming_t = "BRAM_TOP.TOPP";
            }

            grid[(col, row_b)].add_xnode(
                db.get_node("BRAM_BOT"),
                &[&rt_b],
                db.get_node_naming(naming_b),
                &[(col, row_b), (col - 1, row_b)],
            );

            let mut prev_crd = (col, row_b);
            let mut prev_tile: Option<String> = None;
            for row in grid.rows() {
                if row == row_t || row.to_idx() % 4 != 1 {
                    continue;
                }
                let kind;
                let r = row_t.to_idx() - row.to_idx();
                let mut tile = format!("BRAMR{r}C{i}");
                if col == col_l + 1 {
                    kind = "LBRAM";
                    if self.kind == GridKind::Virtex {
                        tile = format!("LBRAMR{r}");
                    }
                } else if col == col_r - 1 {
                    kind = "RBRAM";
                    if self.kind == GridKind::Virtex {
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
                let node = grid[(col, row)].add_xnode(
                    db.get_node(kind),
                    &rts,
                    db.get_node_naming(kind),
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
                let r = (row_t.to_idx() - row.to_idx() - 4) / 4;
                node.add_bel(0, format!("RAMB4_R{r}C{c}"));
                grid.fill_term_pair_anon(prev_crd, (col, row), main_n, main_s);
                prev_crd = (col, row);
                prev_tile = Some(tile);
            }

            grid[(col, row_t)].add_xnode(
                db.get_node("BRAM_TOP"),
                &[&rt_t],
                db.get_node_naming(naming_t),
                &[(col, row_t), (col - 1, row_t)],
            );
            grid.fill_term_pair_anon(prev_crd, (col, row_t), main_n, main_s);
            c += 1;
        }

        // CLKB/CLKT and DLLs
        if self.kind == GridKind::Virtex {
            let col_c = self.col_clk();
            let col_pl = col_l + 1;
            let col_pr = col_r - 1;
            let node = grid[(col_c, row_b)].add_xnode(
                db.get_node("CLKB"),
                &["BM"],
                db.get_node_naming("CLKB"),
                &[(col_c, row_b), (col_pl, row_b), (col_pr, row_b)],
            );
            node.add_bel(0, "GCLKPAD0".to_string());
            node.add_bel(1, "GCLKPAD1".to_string());
            node.add_bel(2, "GCLKBUF0".to_string());
            node.add_bel(3, "GCLKBUF1".to_string());
            let node = grid[(col_c, row_t)].add_xnode(
                db.get_node("CLKT"),
                &["TM"],
                db.get_node_naming("CLKT"),
                &[(col_c, row_t), (col_pl, row_t), (col_pr, row_t)],
            );
            node.add_bel(0, "GCLKPAD2".to_string());
            node.add_bel(1, "GCLKPAD3".to_string());
            node.add_bel(2, "GCLKBUF2".to_string());
            node.add_bel(3, "GCLKBUF3".to_string());
            let node = grid[(col_pl, row_b)].add_xnode(
                db.get_node("DLL.BOT"),
                &["LBRAM_BOT", "BM"],
                db.get_node_naming("DLL.BL"),
                &[(col_pl, row_b), (col_pl - 1, row_b), (col_c, row_b)],
            );
            node.add_bel(0, "DLL1".to_string());
            let node = grid[(col_pl, row_t)].add_xnode(
                db.get_node("DLL.TOP"),
                &["LBRAM_TOP", "TM"],
                db.get_node_naming("DLL.TL"),
                &[(col_pl, row_t), (col_pl - 1, row_t), (col_c, row_t)],
            );
            node.add_bel(0, "DLL3".to_string());
            let node = grid[(col_pr, row_b)].add_xnode(
                db.get_node("DLL.BOT"),
                &["RBRAM_BOT", "BM"],
                db.get_node_naming("DLL.BR"),
                &[(col_pr, row_b), (col_pr - 1, row_b), (col_c, row_b)],
            );
            node.add_bel(0, "DLL0".to_string());
            let node = grid[(col_pr, row_t)].add_xnode(
                db.get_node("DLL.TOP"),
                &["RBRAM_TOP", "TM"],
                db.get_node_naming("DLL.TR"),
                &[(col_pr, row_t), (col_pr - 1, row_t), (col_c, row_t)],
            );
            node.add_bel(0, "DLL2".to_string());
        } else {
            let col_c = self.col_clk();
            let c_pl = bram_mid - 1;
            let c_pr = bram_mid;
            let c_sl = bram_mid - 2;
            let c_sr = bram_mid + 1;
            let col_pl = cols_bram[c_pl];
            let col_pr = cols_bram[c_pr];
            let col_sl = cols_bram[c_sl];
            let col_sr = cols_bram[c_sr];
            let is_s_gclk = c_sl == 0;
            let kind_b;
            let kind_t;
            let s;
            if disabled.contains(&DisabledPart::PrimaryDlls) {
                kind_b = "CLKB_2DLL";
                kind_t = "CLKT_2DLL";
                s = "";
            } else {
                kind_b = "CLKB_4DLL";
                kind_t = "CLKT_4DLL";
                s = "S";
            }
            let node = grid[(col_c, row_b)].add_xnode(
                db.get_node(kind_b),
                &["BM"],
                db.get_node_naming(kind_b),
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
            let node = grid[(col_c, row_t)].add_xnode(
                db.get_node(kind_t),
                &["TM"],
                db.get_node_naming(kind_t),
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
            let node = grid[(col_sl, row_b)].add_xnode(
                db.get_node("DLLS.BOT"),
                &[&format!("BRAM_BOTC{c_sl}"), "BM"],
                db.get_node_naming(if is_s_gclk { "DLLS.BL.GCLK" } else { "DLLS.BL" }),
                &[(col_sl, row_b), (col_sl - 1, row_b), (col_c, row_b)],
            );
            node.add_bel(0, format!("DLL1{s}"));
            let node = grid[(col_sl, row_t)].add_xnode(
                db.get_node("DLLS.TOP"),
                &[&format!("BRAM_TOPC{c_sl}"), "TM"],
                db.get_node_naming(if is_s_gclk { "DLLS.TL.GCLK" } else { "DLLS.TL" }),
                &[(col_sl, row_t), (col_sl - 1, row_t), (col_c, row_t)],
            );
            node.add_bel(0, format!("DLL3{s}"));
            let node = grid[(col_sr, row_b)].add_xnode(
                db.get_node("DLLS.BOT"),
                &[&format!("BRAM_BOTC{c_sr}"), "BM"],
                db.get_node_naming(if is_s_gclk { "DLLS.BR.GCLK" } else { "DLLS.BR" }),
                &[(col_sr, row_b), (col_sr - 1, row_b), (col_c, row_b)],
            );
            node.add_bel(0, format!("DLL0{s}"));
            let node = grid[(col_sr, row_t)].add_xnode(
                db.get_node("DLLS.TOP"),
                &[&format!("BRAM_TOPC{c_sr}"), "TM"],
                db.get_node_naming(if is_s_gclk { "DLLS.TR.GCLK" } else { "DLLS.TR" }),
                &[(col_sr, row_t), (col_sr - 1, row_t), (col_c, row_t)],
            );
            node.add_bel(0, format!("DLL2{s}"));
            if !disabled.contains(&DisabledPart::PrimaryDlls) {
                let node = grid[(col_pl, row_b)].add_xnode(
                    db.get_node("DLLP.BOT"),
                    &[&format!("BRAM_BOTC{c_pl}"), "BM"],
                    db.get_node_naming("DLLP.BL"),
                    &[
                        (col_pl, row_b),
                        (col_pl - 1, row_b),
                        (col_c, row_b),
                        (col_sl, row_b),
                    ],
                );
                node.add_bel(0, "DLL1P".to_string());
                let node = grid[(col_pl, row_t)].add_xnode(
                    db.get_node("DLLP.TOP"),
                    &[&format!("BRAM_TOPC{c_pl}"), "TM"],
                    db.get_node_naming("DLLP.TL"),
                    &[
                        (col_pl, row_t),
                        (col_pl - 1, row_t),
                        (col_c, row_t),
                        (col_sl, row_t),
                    ],
                );
                node.add_bel(0, "DLL3P".to_string());
                let node = grid[(col_pr, row_b)].add_xnode(
                    db.get_node("DLLP.BOT"),
                    &[&format!("BRAM_BOTC{c_pr}"), "BM"],
                    db.get_node_naming("DLLP.BR"),
                    &[
                        (col_pr, row_b),
                        (col_pr - 1, row_b),
                        (col_c, row_b),
                        (col_sr, row_b),
                    ],
                );
                node.add_bel(0, "DLL0P".to_string());
                let node = grid[(col_pr, row_t)].add_xnode(
                    db.get_node("DLLP.TOP"),
                    &[&format!("BRAM_TOPC{c_pr}"), "TM"],
                    db.get_node_naming("DLLP.TR"),
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

        // CLKL/CLKR
        let pci_l = (col_l, self.row_clk());
        let pci_r = (col_r, self.row_clk());
        let node = grid[pci_l].add_xnode(
            db.get_node("CLKL"),
            &["LM"],
            db.get_node_naming("CLKL"),
            &[pci_l],
        );
        node.add_bel(0, "LPCILOGIC".to_string());
        let node = grid[pci_r].add_xnode(
            db.get_node("CLKR"),
            &["RM"],
            db.get_node_naming("CLKR"),
            &[pci_r],
        );
        node.add_bel(0, "RPCILOGIC".to_string());

        let mut cc = 1;
        for &(col_m, col_l, col_r) in &self.cols_clkv {
            for row in grid.rows() {
                for c in col_l.to_idx()..col_m.to_idx() {
                    let col = ColId::from_idx(c);
                    grid[(col, row)].clkroot = (col_m - 1, row);
                }
                if col_m == self.col_lio() + 1 || col_m == self.col_rio() - 1 {
                    let lr = if col_m == self.col_lio() + 1 {
                        'L'
                    } else {
                        'R'
                    };
                    if row == self.row_bio() {
                        for c in col_m.to_idx()..col_r.to_idx() {
                            let col = ColId::from_idx(c);
                            grid[(col, row)].clkroot = (col_m, row);
                        }
                        let name = if self.kind == GridKind::Virtex {
                            format!("{lr}BRAM_BOT")
                        } else {
                            let c = bramclut[col_m];
                            format!("BRAM_BOTC{c}")
                        };
                        grid[(col_m, row)].add_xnode(
                            db.get_node("CLKV_BRAM_BOT"),
                            &[&name],
                            db.get_node_naming("CLKV_BRAM_BOT"),
                            &[(col_m, row), (col_m - 1, row), (col_m, row + 1)],
                        );
                    } else if row == self.row_tio() {
                        for c in col_m.to_idx()..col_r.to_idx() {
                            let col = ColId::from_idx(c);
                            grid[(col, row)].clkroot = (col_m, row);
                        }
                        let name = if self.kind == GridKind::Virtex {
                            format!("{lr}BRAM_TOP")
                        } else {
                            let c = bramclut[col_m];
                            format!("BRAM_TOPC{c}")
                        };
                        grid[(col_m, row)].add_xnode(
                            db.get_node("CLKV_BRAM_TOP"),
                            &[&name],
                            db.get_node_naming("CLKV_BRAM_TOP"),
                            &[(col_m, row), (col_m - 1, row), (col_m, row - 4)],
                        );
                    } else {
                        grid[(col_m, row)].clkroot = (col_m, self.row_clk());
                        for c in (col_m.to_idx() + 1)..col_r.to_idx() {
                            let col = ColId::from_idx(c);
                            grid[(col, row)].clkroot = (col_m + 1, row);
                        }
                        if row.to_idx() % 4 == 1 {
                            let r = row_t.to_idx() - row.to_idx();
                            let name = if self.kind == GridKind::Virtex {
                                format!("{lr}BRAMR{r}")
                            } else {
                                let c = bramclut[col_m];
                                format!("BRAMR{r}C{c}")
                            };
                            grid[(col_m, row)].add_xnode(
                                db.get_node("CLKV_BRAM"),
                                &[&name],
                                db.get_node_naming(if lr == 'L' {
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
                        grid[(col, row)].clkroot = (col_m, row);
                    }
                    let (name, naming) = if col_m == self.col_clk() {
                        if row == self.row_bio() {
                            ("BM".to_string(), "CLKV.CLKB")
                        } else if row == self.row_tio() {
                            ("TM".to_string(), "CLKV.CLKT")
                        } else {
                            let r = row_t.to_idx() - row.to_idx();
                            (format!("VMR{r}"), "CLKV.CLKV")
                        }
                    } else {
                        if row == self.row_bio() {
                            (format!("GCLKBC{cc}"), "CLKV.GCLKB")
                        } else if row == self.row_tio() {
                            (format!("GCLKTC{cc}"), "CLKV.GCLKT")
                        } else {
                            let r = row_t.to_idx() - row.to_idx();
                            (format!("GCLKVR{r}C{cc}"), "CLKV.GCLKV")
                        }
                    };
                    grid[(col_m - 1, row)].add_xnode(
                        db.get_node("CLKV"),
                        &[&name],
                        db.get_node_naming(naming),
                        &[(col_m - 1, row), (col_m, row)],
                    );
                }
            }
            if col_m == self.col_lio() + 1 || col_m == self.col_rio() - 1 {
                let name = if self.kind == GridKind::Virtex {
                    if col_m == self.col_lio() + 1 {
                        "LBRAMM".to_string()
                    } else {
                        "RBRAMM".to_string()
                    }
                } else {
                    let c = bramclut[col_m];
                    format!("BRAMMC{c}")
                };
                grid[(col_m, self.row_clk())].add_xnode(
                    db.get_node("BRAM_CLKH"),
                    &[&name],
                    db.get_node_naming("BRAM_CLKH"),
                    &[(col_m, self.row_clk())],
                );
            } else if col_m == self.col_clk() {
                grid[(col_m, self.row_clk())].add_xnode(
                    db.get_node("CLKC"),
                    &["M"],
                    db.get_node_naming("CLKC"),
                    &[(col_m, self.row_clk())],
                );
            } else {
                let name = format!("GCLKCC{cc}");
                grid[(col_m, self.row_clk())].add_xnode(
                    db.get_node("GCLKC"),
                    &[&name],
                    db.get_node_naming("GCLKC"),
                    &[(col_m, self.row_clk())],
                );
                cc += 1;
            }
        }

        ExpandedDevice {
            grid: self,
            egrid,
            bonded_ios,
        }
    }
}

impl<'a> ExpandedDevice<'a> {
    pub fn get_io_bel(
        &'a self,
        coord: Coord,
        bel: BelId,
    ) -> Option<(&'a ExpandedTileNode, &'a BelInfo, &'a BelNaming, &'a str)> {
        let die = self.egrid.die(DieId::from_idx(0));
        let node = die.tile(coord).nodes.first()?;
        let nk = &self.egrid.db.nodes[node.kind];
        let naming = &self.egrid.db.node_namings[node.naming];
        Some((node, &nk.bels[bel], &naming.bels[bel], &node.bels[bel]))
    }

    pub fn get_io(&'a self, coord: Coord, bel: BelId) -> Io<'a> {
        let (_, _, _, name) = self.get_io_bel(coord, bel).unwrap();
        let bank = if coord.1 == self.grid.row_tio() {
            if coord.0 < self.grid.col_clk() {
                0
            } else {
                1
            }
        } else if coord.0 == self.grid.col_rio() {
            if coord.1 < self.grid.row_mid() {
                3
            } else {
                2
            }
        } else if coord.1 == self.grid.row_bio() {
            if coord.0 < self.grid.col_clk() {
                5
            } else {
                4
            }
        } else if coord.0 == self.grid.col_lio() {
            if coord.1 < self.grid.row_mid() {
                6
            } else {
                7
            }
        } else {
            unreachable!()
        };
        let coord = IoCoord {
            col: coord.0,
            row: coord.1,
            bel,
        };
        Io {
            coord,
            bank,
            name,
            is_vref: self.grid.vref.contains(&coord),
        }
    }

    pub fn get_bonded_ios(&'a self) -> Vec<Io<'a>> {
        let mut res = vec![];
        for &(coord, bel) in &self.bonded_ios {
            res.push(self.get_io(coord, bel));
        }
        res
    }
}
