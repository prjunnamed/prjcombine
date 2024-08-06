use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{DieId, ExpandedGrid};
use prjcombine_virtex_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};
use unnamed_entity::{EntityId, EntityVec};

use crate::expanded::ExpandedDevice;
use crate::grid::Grid;

impl Grid {
    pub fn expand_grid<'a>(&'a self, db: &'a IntDb) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        egrid.tie_kind = Some("GND".to_string());
        egrid.tie_pin_gnd = Some("O".to_string());
        let (_, mut grid) = egrid.add_die(self.columns, self.rows);

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
                        node.add_bel(0, format!("PAD{p}"));
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
                        node.add_bel(3, format!("PAD{p}"));
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
                        node.add_bel(0, format!("PAD{p}"));
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
                        node.add_bel(3, format!("PAD{p}"));
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
            grid.add_xnode(
                (col, row_n),
                db.get_node(kind),
                &[&tile],
                db.get_node_naming(kind),
                &[(col, row_s), (col, row_n)],
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
            grid.add_xnode(
                (col_r, row),
                db.get_node(kind),
                &[&tile],
                db.get_node_naming(kind),
                &[(col_l, row), (col_r, row)],
            );
        }

        grid.fill_main_passes();
        grid.fill_term_anon((col_l, row_b), "CNR.LL");
        grid.fill_term_anon((col_r, row_b), "CNR.LR");
        grid.fill_term_anon((col_l, row_t), "CNR.UL");
        grid.fill_term_anon((col_r, row_t), "CNR.UR");

        let mut spine_framebit = None;
        let mut row_framebit = EntityVec::new();
        let mut frame_len = 0;
        for row in grid.rows() {
            if row == self.row_mid() {
                spine_framebit = Some(frame_len);
                frame_len += 4;
            }
            row_framebit.push(frame_len);
            let height = if row == self.row_bio() || row == self.row_tio() {
                28
            } else {
                34
            };
            frame_len += height;
        }
        let spine_framebit = spine_framebit.unwrap();

        let mut frame_info = vec![];
        let mut spine_frame = None;
        let mut col_frame: EntityVec<_, _> = grid.cols().map(|_| 0).collect();
        for col in grid.cols().rev() {
            let width = if col == self.col_lio() {
                7
            } else if col == self.col_rio() {
                8
            } else {
                12
            };
            col_frame[col] = frame_info.len();
            for _ in 0..width {
                let minor = frame_info.len();
                frame_info.push(FrameInfo {
                    addr: FrameAddr {
                        typ: 0,
                        region: 0,
                        major: 0,
                        minor: minor as u32,
                    },
                });
            }
            if col == self.col_mid() {
                let minor = frame_info.len();
                spine_frame = Some(minor);
                frame_info.push(FrameInfo {
                    addr: FrameAddr {
                        typ: 0,
                        region: 0,
                        major: 0,
                        minor: minor as u32,
                    },
                });
            }
        }
        let spine_frame = spine_frame.unwrap();

        let die_bs_geom = DieBitstreamGeom {
            frame_len,
            frame_info,
            bram_frame_len: 0,
            bram_frame_info: vec![],
            iob_frame_len: 0,
        };
        let bs_geom = BitstreamGeom {
            kind: DeviceKind::Xc5200,
            die: [die_bs_geom].into_iter().collect(),
            die_order: vec![DieId::from_idx(0)],
        };

        egrid.finish();

        ExpandedDevice {
            grid: self,
            egrid,
            bs_geom,
            spine_frame,
            spine_framebit,
            col_frame,
            row_framebit,
        }
    }
}
