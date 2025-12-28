use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::{
    db::IntDb,
    dir::Dir,
    grid::{CellCoord, ExpandedGrid},
};

use crate::{
    chip::{Chip, SpecialTileKey},
    defs::{self, rslots as regions},
    expanded::ExpandedDevice,
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
                    egrid.add_tile_single_id(cell, self.kind.tile_class_ioi(Dir::S).unwrap());
                    egrid.add_tile_single_id(cell, self.kind.tile_class_iob(Dir::S).unwrap());
                    egrid[cell].region_root[regions::EDGE] = *self.special_tiles
                        [&SpecialTileKey::LatchIo(Dir::S)]
                        .cells
                        .first()
                        .unwrap();
                }
            } else if cell.row == self.row_n() {
                if cell.col == self.col_w() || cell.col == self.col_e() {
                    // empty corner
                } else {
                    egrid.add_tile_single_id(cell, self.kind.tile_class_ioi(Dir::N).unwrap());
                    egrid.add_tile_single_id(cell, self.kind.tile_class_iob(Dir::N).unwrap());
                    egrid[cell].region_root[regions::EDGE] = *self.special_tiles
                        [&SpecialTileKey::LatchIo(Dir::N)]
                        .cells
                        .first()
                        .unwrap();
                }
            } else {
                if self.kind.has_ioi_we() && cell.col == self.col_w() {
                    egrid.add_tile_single_id(cell, self.kind.tile_class_ioi(Dir::W).unwrap());
                    if self.kind.has_iob_we() {
                        egrid.add_tile_single_id(cell, self.kind.tile_class_iob(Dir::W).unwrap());
                    }
                    egrid[cell].region_root[regions::EDGE] = *self.special_tiles
                        [&SpecialTileKey::LatchIo(Dir::W)]
                        .cells
                        .first()
                        .unwrap();
                } else if self.kind.has_ioi_we() && cell.col == self.col_e() {
                    egrid.add_tile_single_id(cell, self.kind.tile_class_ioi(Dir::E).unwrap());
                    if self.kind.has_iob_we() {
                        egrid.add_tile_single_id(cell, self.kind.tile_class_iob(Dir::E).unwrap());
                    }
                    egrid[cell].region_root[regions::EDGE] = *self.special_tiles
                        [&SpecialTileKey::LatchIo(Dir::E)]
                        .cells
                        .first()
                        .unwrap();
                } else if self.cols_bram.contains(&cell.col) {
                    egrid.add_tile_single_id(cell, defs::tcls::INT_BRAM);
                    if (cell.row.to_idx() - 1).is_multiple_of(2) {
                        egrid.add_tile_n_id(cell, self.kind.tile_class_bram(), 2);
                    }
                } else {
                    egrid.add_tile_single_id(cell, self.kind.tile_class_plb());
                }
            }
        }

        for (&key, special) in &self.special_tiles {
            let anchor = self.special_tile(key).cell;
            egrid.add_tile_id(
                anchor,
                key.tile_class(self.kind),
                &Vec::from_iter(special.cells.values().copied()),
            );
        }

        for cell in egrid.die_cells(die) {
            if cell.col != self.col_w() {
                egrid.fill_conn_pair_id(
                    cell.delta(-1, 0),
                    cell,
                    defs::ccls::PASS_E,
                    defs::ccls::PASS_W,
                );
            }
            if cell.row != self.row_s() {
                egrid.fill_conn_pair_id(
                    cell.delta(0, -1),
                    cell,
                    defs::ccls::PASS_N,
                    defs::ccls::PASS_S,
                );
            }
        }

        for cell in egrid.die_cells(die) {
            egrid[cell].region_root[regions::GLOBAL] =
                CellCoord::new(die, self.col_w(), self.row_s());
            egrid[cell].region_root[regions::COLBUF] = cell.with_row(self.row_mid);
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
                            defs::tcls::COLBUF_IO_W
                        } else if self.kind.has_ioi_we() && cell.col == self.col_e() {
                            defs::tcls::COLBUF_IO_E
                        } else {
                            self.kind.tile_class_colbuf().unwrap()
                        };
                        egrid.add_tile_single_id(cell, tcls);
                    }
                }
            }
        }
        if self.rows_colbuf.is_empty() {
            for cell in egrid.row(die, self.row_mid) {
                egrid.add_tile_single_id(cell, defs::tcls::COLBUF_FIXED);
            }
        }

        let cnr_ws = CellCoord::new(die, self.col_w(), self.row_s());
        let cnr_wn = CellCoord::new(die, self.col_w(), self.row_n());
        let cnr_es = CellCoord::new(die, self.col_e(), self.row_s());
        let cnr_en = CellCoord::new(die, self.col_e(), self.row_n());
        if self.kind.has_ioi_we() {
            for i in 0..4 {
                for j in 0..4 {
                    let wh = egrid.resolve_wire(cnr_ws.wire(defs::QUAD_H[i][j])).unwrap();
                    let wv = egrid
                        .resolve_wire(cnr_ws.wire(defs::QUAD_V[3 - i][j]))
                        .unwrap();
                    egrid.extra_conns.insert(wh, wv);
                }
            }
            for i in 0..4 {
                for j in 0..4 {
                    let wh = egrid.resolve_wire(cnr_wn.wire(defs::QUAD_H[i][j])).unwrap();
                    let wv = egrid
                        .resolve_wire(cnr_wn.wire(defs::QUAD_V[4 - i][j]))
                        .unwrap();
                    egrid.extra_conns.insert(wh, wv);
                }
            }
            for i in 0..4 {
                for j in 0..4 {
                    let wh = egrid
                        .resolve_wire(cnr_es.wire(defs::QUAD_H[1 + i][j]))
                        .unwrap();
                    let wv = egrid
                        .resolve_wire(cnr_es.wire(defs::QUAD_V[3 - i][j]))
                        .unwrap();
                    egrid.extra_conns.insert(wh, wv);
                }
            }
            for i in 0..4 {
                for j in 0..4 {
                    let wh = egrid
                        .resolve_wire(cnr_en.wire(defs::QUAD_H[1 + i][j]))
                        .unwrap();
                    let wv = egrid
                        .resolve_wire(cnr_en.wire(defs::QUAD_V[4 - i][j]))
                        .unwrap();
                    egrid.extra_conns.insert(wh, wv);
                }
            }
        } else {
            for i in 0..16 {
                let seg = i / 4;
                let which = i % 4;
                let wh = egrid
                    .resolve_wire(cnr_ws.wire(defs::QUAD_H[seg][which]))
                    .unwrap();
                let seg = 3 - (32 + i) / 12;
                let mut which = (32 + i) % 12;
                which ^= seg & 1;
                let wv = egrid
                    .resolve_wire(cnr_ws.wire(defs::QUAD_V[seg][which]))
                    .unwrap();
                egrid.extra_conns.insert(wh, wv);
            }
            for i in 0..16 {
                let seg = i / 4;
                let which = i % 4;
                let wh = egrid
                    .resolve_wire(cnr_wn.wire(defs::QUAD_H[seg][which]))
                    .unwrap();
                let mut seg = 3 - i / 12;
                let mut which = i % 12;
                which ^= seg & 1;
                seg += 1;
                let wv = egrid
                    .resolve_wire(cnr_wn.wire(defs::QUAD_V[seg][which]))
                    .unwrap();
                egrid.extra_conns.insert(wh, wv);
            }
            for i in 0..16 {
                let seg = 1 + i / 4;
                let which = i % 4;
                let wh = egrid
                    .resolve_wire(cnr_es.wire(defs::QUAD_H[seg][which]))
                    .unwrap();
                let seg = 3 - (32 + i) / 12;
                let mut which = (32 + i) % 12;
                which ^= seg & 1;
                let wv = egrid
                    .resolve_wire(cnr_es.wire(defs::QUAD_V[seg][which]))
                    .unwrap();
                egrid.extra_conns.insert(wh, wv);
            }
            for i in 0..16 {
                let seg = 1 + i / 4;
                let which = i % 4;
                let wh = egrid
                    .resolve_wire(cnr_en.wire(defs::QUAD_H[seg][which]))
                    .unwrap();
                let mut seg = 3 - i / 12;
                let mut which = i % 12;
                which ^= seg & 1;
                seg += 1;
                let wv = egrid
                    .resolve_wire(cnr_en.wire(defs::QUAD_V[seg][which]))
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
