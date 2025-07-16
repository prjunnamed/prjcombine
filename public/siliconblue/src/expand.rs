use prjcombine_interconnect::{
    db::IntDb,
    dir::Dir,
    grid::{CellCoord, DieId, ExpandedGrid},
};
use unnamed_entity::{EntityId, EntityVec};

use crate::{
    chip::{Chip, ExtraNodeLoc},
    expanded::{ExpandedDevice, REGION_COLBUF, REGION_GLOBAL},
};

impl Chip {
    pub fn expand_grid<'a>(&'a self, db: &'a IntDb) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        let (_, mut die) = egrid.add_die(self.columns, self.rows);

        // fill tiles

        for col in die.cols() {
            for row in die.rows() {
                if row == self.row_s() {
                    if col == self.col_w() || col == self.col_e() {
                        // empty corner
                    } else {
                        die.fill_tile((col, row), self.kind.tile_class_ioi(Dir::S).unwrap());
                        die.add_tile(
                            (col, row),
                            self.kind.tile_class_iob(Dir::S).unwrap(),
                            &[(col, row)],
                        );
                    }
                } else if row == self.row_n() {
                    if col == self.col_w() || col == self.col_e() {
                        // empty corner
                    } else {
                        die.fill_tile((col, row), self.kind.tile_class_ioi(Dir::N).unwrap());
                        die.add_tile(
                            (col, row),
                            self.kind.tile_class_iob(Dir::N).unwrap(),
                            &[(col, row)],
                        );
                    }
                } else {
                    if self.kind.has_ioi_we() && col == self.col_w() {
                        die.fill_tile((col, row), self.kind.tile_class_ioi(Dir::W).unwrap());
                        if self.kind.has_iob_we() {
                            die.add_tile(
                                (col, row),
                                self.kind.tile_class_iob(Dir::W).unwrap(),
                                &[(col, row)],
                            );
                        }
                    } else if self.kind.has_ioi_we() && col == self.col_e() {
                        die.fill_tile((col, row), self.kind.tile_class_ioi(Dir::E).unwrap());
                        if self.kind.has_iob_we() {
                            die.add_tile(
                                (col, row),
                                self.kind.tile_class_iob(Dir::E).unwrap(),
                                &[(col, row)],
                            );
                        }
                    } else if self.cols_bram.contains(&col) {
                        die.fill_tile((col, row), "INT_BRAM");
                        if (row.to_idx() - 1).is_multiple_of(2) {
                            die.add_tile(
                                (col, row),
                                self.kind.tile_class_bram(),
                                &[(col, row), (col, row + 1)],
                            );
                        }
                    } else {
                        die.fill_tile((col, row), self.kind.tile_class_plb());
                    }
                }
            }
        }

        die.add_tile(
            (self.col_w(), self.row_s()),
            self.kind.tile_class_gb_root(),
            &[(self.col_w(), self.row_s())],
        );

        for (&loc, node) in &self.extra_nodes {
            if matches!(loc, ExtraNodeLoc::GbIo(..)) {
                continue;
            }
            let fcell = node.cells.first().unwrap();
            die.add_tile(
                (fcell.col, fcell.row),
                &loc.tile_class(self.kind),
                &Vec::from_iter(node.cells.values().map(|cell| (cell.col, cell.row))),
            );
        }

        for col in die.cols() {
            for row in die.rows() {
                if col != self.col_w() {
                    die.fill_conn_pair((col - 1, row), (col, row), "PASS_E", "PASS_W");
                }
                if row != self.row_s() {
                    die.fill_conn_pair((col, row - 1), (col, row), "PASS_N", "PASS_S");
                }
            }
        }

        for col in die.cols() {
            for row in die.rows() {
                die[(col, row)].region_root[REGION_GLOBAL] = (self.col_w(), self.row_s());
                die[(col, row)].region_root[REGION_COLBUF] = (self.col_w(), self.row_s());
            }
        }

        for &(row_m, row_b, row_t) in &self.rows_colbuf {
            for col in die.cols() {
                for row in row_b.range(row_t) {
                    let row_cb = if row < row_m {
                        if self.cols_bram.contains(&col) && !self.kind.has_ice40_bramv2() {
                            row_m - 2
                        } else {
                            row_m - 1
                        }
                    } else {
                        row_m
                    };
                    die[(col, row)].region_root[REGION_COLBUF] = (col, row_cb);
                    if row == row_cb {
                        let tcls = if self.kind.has_ioi_we() && col == self.col_w() {
                            "COLBUF_IO_W"
                        } else if self.kind.has_ioi_we() && col == self.col_e() {
                            "COLBUF_IO_E"
                        } else {
                            self.kind.tile_class_colbuf().unwrap()
                        };
                        die.add_tile((col, row), tcls, &[(col, row)]);
                    }
                }
            }
        }

        let cnr_ws = CellCoord::new(DieId::from_idx(0), self.col_w(), self.row_s());
        let cnr_wn = CellCoord::new(DieId::from_idx(0), self.col_w(), self.row_n());
        let cnr_es = CellCoord::new(DieId::from_idx(0), self.col_e(), self.row_s());
        let cnr_en = CellCoord::new(DieId::from_idx(0), self.col_e(), self.row_n());
        if self.kind.has_ioi_we() {
            for i in 0..4 {
                for j in 0..4 {
                    let wh = egrid
                        .resolve_wire(cnr_ws.wire(db.get_wire(&format!("QUAD.H{i}.{j}"))))
                        .unwrap();
                    let wv = egrid
                        .resolve_wire(
                            cnr_ws.wire(db.get_wire(&format!("QUAD.V{i}.{jj}", jj = 3 - j))),
                        )
                        .unwrap();
                    egrid.extra_conns.insert(wh, wv);
                }
            }
            for i in 0..4 {
                for j in 0..4 {
                    let wh = egrid
                        .resolve_wire(cnr_wn.wire(db.get_wire(&format!("QUAD.H{i}.{j}"))))
                        .unwrap();
                    let wv = egrid
                        .resolve_wire(
                            cnr_wn.wire(db.get_wire(&format!("QUAD.V{i}.{jj}", jj = 4 - j))),
                        )
                        .unwrap();
                    egrid.extra_conns.insert(wh, wv);
                }
            }
            for i in 0..4 {
                for j in 0..4 {
                    let wh = egrid
                        .resolve_wire(
                            cnr_es.wire(db.get_wire(&format!("QUAD.H{i}.{jj}", jj = 1 + j))),
                        )
                        .unwrap();
                    let wv = egrid
                        .resolve_wire(
                            cnr_es.wire(db.get_wire(&format!("QUAD.V{i}.{jj}", jj = 3 - j))),
                        )
                        .unwrap();
                    egrid.extra_conns.insert(wh, wv);
                }
            }
            for i in 0..4 {
                for j in 0..4 {
                    let wh = egrid
                        .resolve_wire(
                            cnr_en.wire(db.get_wire(&format!("QUAD.H{i}.{jj}", jj = 1 + j))),
                        )
                        .unwrap();
                    let wv = egrid
                        .resolve_wire(
                            cnr_en.wire(db.get_wire(&format!("QUAD.V{i}.{jj}", jj = 4 - j))),
                        )
                        .unwrap();
                    egrid.extra_conns.insert(wh, wv);
                }
            }
        } else {
            for i in 0..16 {
                let seg = i / 4;
                let which = i % 4;
                let wh = egrid
                    .resolve_wire(cnr_ws.wire(db.get_wire(&format!("QUAD.H{which}.{seg}"))))
                    .unwrap();
                let seg = 3 - (32 + i) / 12;
                let mut which = (32 + i) % 12;
                which ^= seg & 1;
                let wv = egrid
                    .resolve_wire(cnr_ws.wire(db.get_wire(&format!("QUAD.V{which}.{seg}"))))
                    .unwrap();
                egrid.extra_conns.insert(wh, wv);
            }
            for i in 0..16 {
                let seg = i / 4;
                let which = i % 4;
                let wh = egrid
                    .resolve_wire(cnr_wn.wire(db.get_wire(&format!("QUAD.H{which}.{seg}"))))
                    .unwrap();
                let mut seg = 3 - i / 12;
                let mut which = i % 12;
                which ^= seg & 1;
                seg += 1;
                let wv = egrid
                    .resolve_wire(cnr_wn.wire(db.get_wire(&format!("QUAD.V{which}.{seg}"))))
                    .unwrap();
                egrid.extra_conns.insert(wh, wv);
            }
            for i in 0..16 {
                let seg = 1 + i / 4;
                let which = i % 4;
                let wh = egrid
                    .resolve_wire(cnr_es.wire(db.get_wire(&format!("QUAD.H{which}.{seg}"))))
                    .unwrap();
                let seg = 3 - (32 + i) / 12;
                let mut which = (32 + i) % 12;
                which ^= seg & 1;
                let wv = egrid
                    .resolve_wire(cnr_es.wire(db.get_wire(&format!("QUAD.V{which}.{seg}"))))
                    .unwrap();
                egrid.extra_conns.insert(wh, wv);
            }
            for i in 0..16 {
                let seg = 1 + i / 4;
                let which = i % 4;
                let wh = egrid
                    .resolve_wire(cnr_en.wire(db.get_wire(&format!("QUAD.H{which}.{seg}"))))
                    .unwrap();
                let mut seg = 3 - i / 12;
                let mut which = i % 12;
                which ^= seg & 1;
                seg += 1;
                let wv = egrid
                    .resolve_wire(cnr_en.wire(db.get_wire(&format!("QUAD.V{which}.{seg}"))))
                    .unwrap();
                egrid.extra_conns.insert(wh, wv);
            }
        }

        // bitstream geometry

        let mut frame_width_l = 0;
        let mut frame_width_r = 0;
        let mut col_bit: EntityVec<_, _> = self.columns().map(|_| 0).collect();
        for col in self.columns() {
            if col >= self.col_mid() {
                break;
            }
            col_bit[col] = frame_width_l;
            frame_width_l += self.btile_width(col);
        }
        for col in self.columns().rev() {
            if col < self.col_mid() {
                break;
            }
            col_bit[col] = frame_width_r;
            frame_width_r += self.btile_width(col);
        }
        assert_eq!(frame_width_l, frame_width_r);
        let frame_width = frame_width_l + 2;

        egrid.finish();
        ExpandedDevice {
            chip: self,
            egrid,
            col_bit,
            frame_width,
        }
    }
}
