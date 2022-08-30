use crate::{eint, int, BelCoord, BelId, ColId, RowId, SlrId};
use prjcombine_entity::EntityId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub columns: usize,
    pub rows: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct Io<'a> {
    pub coord: BelCoord,
    pub name: &'a str,
}

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub egrid: eint::ExpandedGrid<'a>,
}

impl Grid {
    pub fn col_lio(&self) -> ColId {
        ColId::from_idx(0)
    }

    pub fn col_rio(&self) -> ColId {
        ColId::from_idx(self.columns - 1)
    }

    pub fn col_mid(&self) -> ColId {
        ColId::from_idx(self.columns / 2)
    }

    pub fn row_bio(&self) -> RowId {
        RowId::from_idx(0)
    }

    pub fn row_tio(&self) -> RowId {
        RowId::from_idx(self.rows - 1)
    }

    pub fn row_mid(&self) -> RowId {
        RowId::from_idx(self.rows / 2)
    }

    pub fn expand_grid<'a>(&'a self, db: &'a int::IntDb) -> ExpandedDevice<'a> {
        let mut egrid = eint::ExpandedGrid::new(db);
        egrid.tie_kind = Some("GND".to_string());
        egrid.tie_pin_gnd = Some("O".to_string());
        let (_, mut grid) = egrid.add_slr(self.columns, self.rows);

        let col_l = grid.cols().next().unwrap();
        let col_r = grid.cols().next_back().unwrap();
        let row_b = grid.rows().next().unwrap();
        let row_t = grid.rows().next_back().unwrap();
        for col in grid.cols() {
            let c = col.to_idx();
            if col == col_l {
                for row in grid.rows() {
                    let r = row_t.to_idx() - row.to_idx();
                    if row == row_b {
                        let node = grid.fill_tile((col, row), "CNR.BL", "CNR.BL", "BL".to_string());
                        node.add_bel(0, "BUFG_BL".to_string());
                        node.add_bel(2, "RDBK".to_string());
                        node.tie_name = Some(format!("GND_R{r}C{c}"));
                    } else if row == row_t {
                        let node = grid.fill_tile((col, row), "CNR.TL", "CNR.TL", "TL".to_string());
                        node.add_bel(0, "BUFG_TL".to_string());
                        node.add_bel(2, "BSCAN".to_string());
                        node.tie_name = Some(format!("GND_R{r}C{c}"));
                    } else {
                        let node = if row == row_t - 1 {
                            grid.fill_tile((col, row), "IO.L", "IO.L.CLK", "LCLK".to_string())
                        } else {
                            grid.fill_tile((col, row), "IO.L", "IO.L", format!("LR{r}"))
                        };
                        let p = (self.columns - 2) * 8
                            + (self.rows - 2) * 4
                            + (row.to_idx() - 1) * 4
                            + 1;
                        node.add_bel(0, format!("PAD{}", p));
                        node.add_bel(1, format!("PAD{}", p + 1));
                        node.add_bel(2, format!("PAD{}", p + 2));
                        node.add_bel(3, format!("PAD{}", p + 3));
                        node.add_bel(4, format!("TBUF_R{r}C{c}.0"));
                        node.add_bel(5, format!("TBUF_R{r}C{c}.1"));
                        node.add_bel(6, format!("TBUF_R{r}C{c}.2"));
                        node.add_bel(7, format!("TBUF_R{r}C{c}.3"));
                        node.tie_name = Some(format!("GND_R{r}C{c}"));
                    }
                }
            } else if col == col_r {
                for row in grid.rows() {
                    let r = row_t.to_idx() - row.to_idx();
                    if row == row_b {
                        let node = grid.fill_tile((col, row), "CNR.BR", "CNR.BR", "BR".to_string());
                        node.add_bel(0, "BUFG_BR".to_string());
                        node.add_bel(2, "STARTUP".to_string());
                        node.tie_name = Some(format!("GND_R{r}C{c}"));
                    } else if row == row_t {
                        let node = grid.fill_tile((col, row), "CNR.TR", "CNR.TR", "TR".to_string());
                        node.add_bel(0, "BUFG_TR".to_string());
                        node.add_bel(2, "OSC".to_string());
                        node.add_bel(3, "BYPOSC".to_string());
                        node.add_bel(4, "BSUPD".to_string());
                        node.tie_name = Some(format!("GND_R{r}C{c}"));
                    } else {
                        let node = if row == row_b + 1 {
                            grid.fill_tile((col, row), "IO.R", "IO.R.CLK", "RCLK".to_string())
                        } else {
                            grid.fill_tile((col, row), "IO.R", "IO.R", format!("RR{r}"))
                        };
                        let p =
                            (self.columns - 2) * 4 + (row_t.to_idx() - row.to_idx() - 1) * 4 + 1;
                        node.add_bel(0, format!("PAD{}", p + 3));
                        node.add_bel(1, format!("PAD{}", p + 2));
                        node.add_bel(2, format!("PAD{}", p + 1));
                        node.add_bel(3, format!("PAD{}", p));
                        node.add_bel(4, format!("TBUF_R{r}C{c}.0"));
                        node.add_bel(5, format!("TBUF_R{r}C{c}.1"));
                        node.add_bel(6, format!("TBUF_R{r}C{c}.2"));
                        node.add_bel(7, format!("TBUF_R{r}C{c}.3"));
                        node.tie_name = Some(format!("GND_R{r}C{c}"));
                    }
                }
            } else {
                for row in grid.rows() {
                    let r = row_t.to_idx() - row.to_idx();
                    if row == row_b {
                        let node = if col == col_l + 1 {
                            grid.fill_tile((col, row), "IO.B", "IO.B.CLK", "BCLK".to_string())
                        } else {
                            grid.fill_tile((col, row), "IO.B", "IO.B", format!("BC{c}"))
                        };
                        let p = (self.columns - 2) * 4
                            + (self.rows - 2) * 4
                            + (col_r.to_idx() - col.to_idx() - 1) * 4
                            + 1;
                        node.add_bel(0, format!("PAD{}", p));
                        node.add_bel(1, format!("PAD{}", p + 1));
                        node.add_bel(2, format!("PAD{}", p + 2));
                        node.add_bel(3, format!("PAD{}", p + 3));
                        node.add_bel(4, format!("TBUF_R{r}C{c}.0"));
                        node.add_bel(5, format!("TBUF_R{r}C{c}.1"));
                        node.add_bel(6, format!("TBUF_R{r}C{c}.2"));
                        node.add_bel(7, format!("TBUF_R{r}C{c}.3"));
                        node.tie_name = Some(format!("GND_R{r}C{c}"));
                    } else if row == row_t {
                        let node = if col == col_r - 2 {
                            grid.fill_tile((col, row), "IO.T", "IO.T.CLK", "TCLK".to_string())
                        } else {
                            grid.fill_tile((col, row), "IO.T", "IO.T", format!("TC{c}"))
                        };
                        let p = (col.to_idx() - 1) * 4 + 1;
                        node.add_bel(0, format!("PAD{}", p + 3));
                        node.add_bel(1, format!("PAD{}", p + 2));
                        node.add_bel(2, format!("PAD{}", p + 1));
                        node.add_bel(3, format!("PAD{}", p));
                        node.add_bel(4, format!("TBUF_R{r}C{c}.0"));
                        node.add_bel(5, format!("TBUF_R{r}C{c}.1"));
                        node.add_bel(6, format!("TBUF_R{r}C{c}.2"));
                        node.add_bel(7, format!("TBUF_R{r}C{c}.3"));
                        node.tie_name = Some(format!("GND_R{r}C{c}"));
                    } else {
                        let node = grid.fill_tile((col, row), "CLB", "CLB", format!("R{r}C{c}"));
                        node.add_bel(0, format!("CLB_R{r}C{c}.LC0"));
                        node.add_bel(1, format!("CLB_R{r}C{c}.LC1"));
                        node.add_bel(2, format!("CLB_R{r}C{c}.LC2"));
                        node.add_bel(3, format!("CLB_R{r}C{c}.LC3"));
                        node.add_bel(4, format!("TBUF_R{r}C{c}.0"));
                        node.add_bel(5, format!("TBUF_R{r}C{c}.1"));
                        node.add_bel(6, format!("TBUF_R{r}C{c}.2"));
                        node.add_bel(7, format!("TBUF_R{r}C{c}.3"));
                        node.add_bel(8, format!("VCC_GND_R{r}C{c}"));
                    }
                }
            }
        }

        let term_s = db.get_term("LLV.S");
        let term_n = db.get_term("LLV.N");
        for col in grid.cols() {
            let kind;
            let tile;
            if col == col_l {
                kind = "CLKL";
                tile = "LM".to_string();
            } else if col == col_r {
                kind = "CLKR";
                tile = "RM".to_string();
            } else {
                kind = "CLKH";
                let c = col.to_idx();
                tile = format!("HMC{c}");
            }
            let row_s = self.row_mid() - 1;
            let row_n = self.row_mid();
            grid.fill_term_pair_anon((col, row_s), (col, row_n), term_n, term_s);
            grid[(col, row_n)].add_xnode(
                db.get_node(kind),
                &[&tile],
                db.get_node_naming(kind),
                &[(col, row_n), (col, row_s)],
            );
        }

        let term_w = db.get_term("LLH.W");
        let term_e = db.get_term("LLH.E");
        for row in grid.rows() {
            let kind;
            let tile;
            if row == row_b {
                kind = "CLKB";
                tile = "BM".to_string();
            } else if row == row_t {
                kind = "CLKT";
                tile = "TM".to_string();
            } else {
                kind = "CLKV";
                let r = row_t.to_idx() - row.to_idx();
                tile = format!("VMR{r}");
            }
            let col_l = self.col_mid() - 1;
            let col_r = self.col_mid();
            grid.fill_term_pair_anon((col_l, row), (col_r, row), term_e, term_w);
            grid[(col_r, row)].add_xnode(
                db.get_node(kind),
                &[&tile],
                db.get_node_naming(kind),
                &[(col_r, row), (col_l, row)],
            );
        }

        grid.fill_main_passes();

        ExpandedDevice { grid: self, egrid }
    }
}

impl<'a> ExpandedDevice<'a> {
    pub fn get_io_bel(
        &'a self,
        coord: eint::Coord,
        bel: BelId,
    ) -> Option<(
        &'a eint::ExpandedTileNode,
        &'a int::BelInfo,
        &'a int::BelNaming,
        &'a str,
    )> {
        let slr = self.egrid.slr(SlrId::from_idx(0));
        let node = slr.tile(coord).nodes.first()?;
        let nk = &self.egrid.db.nodes[node.kind];
        let naming = &self.egrid.db.node_namings[node.naming];
        Some((node, &nk.bels[bel], &naming.bels[bel], &node.bels[bel]))
    }

    pub fn get_io(&'a self, coord: eint::Coord, bel: BelId) -> Io<'a> {
        let (_, _, _, name) = self.get_io_bel(coord, bel).unwrap();
        let coord = BelCoord {
            col: coord.0,
            row: coord.1,
            bel,
        };
        Io { coord, name }
    }

    pub fn get_bonded_ios(&'a self) -> Vec<Io<'a>> {
        let mut res = vec![];
        let slr = self.egrid.slr(SlrId::from_idx(0));
        for col in slr.cols() {
            if col == self.grid.col_lio() || col == self.grid.col_rio() {
                continue;
            }
            for bel in [3, 2, 1, 0] {
                res.push(self.get_io((col, self.grid.row_tio()), BelId::from_idx(bel)));
            }
        }
        for row in slr.rows().rev() {
            if row == self.grid.row_bio() || row == self.grid.row_tio() {
                continue;
            }
            for bel in [3, 2, 1, 0] {
                res.push(self.get_io((self.grid.col_rio(), row), BelId::from_idx(bel)));
            }
        }
        for col in slr.cols().rev() {
            if col == self.grid.col_lio() || col == self.grid.col_rio() {
                continue;
            }
            for bel in [0, 1, 2, 3] {
                res.push(self.get_io((col, self.grid.row_bio()), BelId::from_idx(bel)));
            }
        }
        for row in slr.rows() {
            if row == self.grid.row_bio() || row == self.grid.row_tio() {
                continue;
            }
            for bel in [0, 1, 2, 3] {
                res.push(self.get_io((self.grid.col_lio(), row), BelId::from_idx(bel)));
            }
        }
        res
    }
}
