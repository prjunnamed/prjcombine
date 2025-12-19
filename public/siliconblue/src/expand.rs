use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::{
    db::IntDb,
    dir::Dir,
    grid::{CellCoord, ExpandedGrid},
};

use crate::{
    chip::{Chip, SpecialTileKey},
    expanded::ExpandedDevice,
    regions,
};

impl Chip {
    pub fn expand_grid<'a>(&'a self, db: &'a IntDb) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        let die = egrid.add_die(self.columns, self.rows);

        // fill tiles

        for cell in egrid.die_cells(die) {
            if cell.row == self.row_s() {
                if cell.col == self.col_w() || cell.col == self.col_e() {
                    // empty corner
                } else {
                    egrid.add_tile_single(cell, self.kind.tile_class_ioi(Dir::S).unwrap());
                    egrid.add_tile_single(cell, self.kind.tile_class_iob(Dir::S).unwrap());
                }
            } else if cell.row == self.row_n() {
                if cell.col == self.col_w() || cell.col == self.col_e() {
                    // empty corner
                } else {
                    egrid.add_tile_single(cell, self.kind.tile_class_ioi(Dir::N).unwrap());
                    egrid.add_tile_single(cell, self.kind.tile_class_iob(Dir::N).unwrap());
                }
            } else {
                if self.kind.has_ioi_we() && cell.col == self.col_w() {
                    egrid.add_tile_single(cell, self.kind.tile_class_ioi(Dir::W).unwrap());
                    if self.kind.has_iob_we() {
                        egrid.add_tile_single(cell, self.kind.tile_class_iob(Dir::W).unwrap());
                    }
                } else if self.kind.has_ioi_we() && cell.col == self.col_e() {
                    egrid.add_tile_single(cell, self.kind.tile_class_ioi(Dir::E).unwrap());
                    if self.kind.has_iob_we() {
                        egrid.add_tile_single(cell, self.kind.tile_class_iob(Dir::E).unwrap());
                    }
                } else if self.cols_bram.contains(&cell.col) {
                    egrid.add_tile_single(cell, "INT_BRAM");
                    if (cell.row.to_idx() - 1).is_multiple_of(2) {
                        egrid.add_tile_n(cell, self.kind.tile_class_bram(), 2);
                    }
                } else {
                    egrid.add_tile_single(cell, self.kind.tile_class_plb());
                }
            }
        }

        egrid.add_tile_single(
            CellCoord::new(die, self.col_w(), self.row_s()),
            self.kind.tile_class_gb_root(),
        );

        for (&key, special) in &self.special_tiles {
            if matches!(key, SpecialTileKey::GbIo(..)) {
                continue;
            }
            let fcell = *special.cells.first().unwrap();
            egrid.add_tile(
                fcell,
                &key.tile_class(self.kind),
                &Vec::from_iter(special.cells.values().copied()),
            );
        }

        for cell in egrid.die_cells(die) {
            if cell.col != self.col_w() {
                egrid.fill_conn_pair(cell.delta(-1, 0), cell, "PASS_E", "PASS_W");
            }
            if cell.row != self.row_s() {
                egrid.fill_conn_pair(cell.delta(0, -1), cell, "PASS_N", "PASS_S");
            }
        }

        for cell in egrid.die_cells(die) {
            egrid[cell].region_root[regions::GLOBAL] =
                CellCoord::new(die, self.col_w(), self.row_s());
            egrid[cell].region_root[regions::COLBUF] =
                CellCoord::new(die, self.col_w(), self.row_s());
        }

        for &(row_m, row_b, row_t) in &self.rows_colbuf {
            for row in row_b.range(row_t) {
                for cell in egrid.row(die, row) {
                    let row_cb = if row < row_m {
                        if self.cols_bram.contains(&cell.col) && !self.kind.has_ice40_bramv2() {
                            row_m - 2
                        } else {
                            row_m - 1
                        }
                    } else {
                        row_m
                    };
                    egrid[cell].region_root[regions::COLBUF] = cell.with_row(row_cb);
                    if row == row_cb {
                        let tcls = if self.kind.has_ioi_we() && cell.col == self.col_w() {
                            "COLBUF_IO_W"
                        } else if self.kind.has_ioi_we() && cell.col == self.col_e() {
                            "COLBUF_IO_E"
                        } else {
                            self.kind.tile_class_colbuf().unwrap()
                        };
                        egrid.add_tile_single(cell, tcls);
                    }
                }
            }
        }

        let cnr_ws = CellCoord::new(die, self.col_w(), self.row_s());
        let cnr_wn = CellCoord::new(die, self.col_w(), self.row_n());
        let cnr_es = CellCoord::new(die, self.col_e(), self.row_s());
        let cnr_en = CellCoord::new(die, self.col_e(), self.row_n());
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
