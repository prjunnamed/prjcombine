use prjcombine_interconnect::{
    db::IntDb,
    grid::{DieId, ExpandedGrid},
};
use unnamed_entity::{EntityId, EntityVec};

use crate::{
    chip::Chip,
    expanded::{ExpandedDevice, REGION_GLOBAL},
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
                        die.fill_tile((col, row), "CNR");
                    } else {
                        die.fill_tile((col, row), "IO.S");
                    }
                } else if row == self.row_n() {
                    if col == self.col_w() || col == self.col_e() {
                        die.fill_tile((col, row), "CNR");
                    } else {
                        die.fill_tile((col, row), "IO.N");
                    }
                } else {
                    if self.kind.has_io_we() && col == self.col_w() {
                        die.fill_tile((col, row), "IO.W");
                    } else if self.kind.has_io_we() && col == self.col_e() {
                        die.fill_tile((col, row), "IO.E");
                    } else if self.cols_bram.contains(&col) {
                        die.fill_tile((col, row), "INT.BRAM");
                        if (row.to_idx() - 1) % 2 == 0 {
                            die.add_tile((col, row), "BRAM", &[(col, row), (col, row + 1)]);
                        }
                    } else {
                        die.fill_tile((col, row), "PLB");
                    }
                }
            }
        }

        die.add_tile(
            (self.col_w(), self.row_s()),
            "GB_OUT",
            &[(self.col_w(), self.row_s())],
        );

        for (&loc, node) in &self.extra_nodes {
            die.add_tile(
                *node.tiles.first().unwrap(),
                &loc.node_kind(),
                &Vec::from_iter(node.tiles.values().copied()),
            );
        }

        die.fill_main_passes();

        for col in die.cols() {
            for row in die.rows() {
                die[(col, row)].region_root[REGION_GLOBAL] = (self.col_w(), self.row_s());
            }
        }

        if self.kind.has_io_we() {
            for i in 0..4 {
                for j in 0..4 {
                    let wh = egrid
                        .resolve_wire((
                            DieId::from_idx(0),
                            (self.col_w(), self.row_s()),
                            db.get_wire(&format!("QUAD.H{i}.{j}")),
                        ))
                        .unwrap();
                    let wv = egrid
                        .resolve_wire((
                            DieId::from_idx(0),
                            (self.col_w(), self.row_s()),
                            db.get_wire(&format!("QUAD.V{i}.{jj}", jj = 3 - j)),
                        ))
                        .unwrap();
                    egrid.extra_conns.insert(wh, wv);
                }
            }
            for i in 0..4 {
                for j in 0..4 {
                    let wh = egrid
                        .resolve_wire((
                            DieId::from_idx(0),
                            (self.col_w(), self.row_n()),
                            db.get_wire(&format!("QUAD.H{i}.{j}")),
                        ))
                        .unwrap();
                    let wv = egrid
                        .resolve_wire((
                            DieId::from_idx(0),
                            (self.col_w(), self.row_n()),
                            db.get_wire(&format!("QUAD.V{i}.{jj}", jj = 4 - j)),
                        ))
                        .unwrap();
                    egrid.extra_conns.insert(wh, wv);
                }
            }
            for i in 0..4 {
                for j in 0..4 {
                    let wh = egrid
                        .resolve_wire((
                            DieId::from_idx(0),
                            (self.col_e(), self.row_s()),
                            db.get_wire(&format!("QUAD.H{i}.{jj}", jj = 1 + j)),
                        ))
                        .unwrap();
                    let wv = egrid
                        .resolve_wire((
                            DieId::from_idx(0),
                            (self.col_e(), self.row_s()),
                            db.get_wire(&format!("QUAD.V{i}.{jj}", jj = 3 - j)),
                        ))
                        .unwrap();
                    egrid.extra_conns.insert(wh, wv);
                }
            }
            for i in 0..4 {
                for j in 0..4 {
                    let wh = egrid
                        .resolve_wire((
                            DieId::from_idx(0),
                            (self.col_e(), self.row_n()),
                            db.get_wire(&format!("QUAD.H{i}.{jj}", jj = 1 + j)),
                        ))
                        .unwrap();
                    let wv = egrid
                        .resolve_wire((
                            DieId::from_idx(0),
                            (self.col_e(), self.row_n()),
                            db.get_wire(&format!("QUAD.V{i}.{jj}", jj = 4 - j)),
                        ))
                        .unwrap();
                    egrid.extra_conns.insert(wh, wv);
                }
            }
        } else {
            for i in 0..16 {
                let seg = i / 4;
                let which = i % 4;
                let wh = egrid
                    .resolve_wire((
                        DieId::from_idx(0),
                        (self.col_w(), self.row_s()),
                        db.get_wire(&format!("QUAD.H{which}.{seg}")),
                    ))
                    .unwrap();
                let seg = 3 - (32 + i) / 12;
                let mut which = (32 + i) % 12;
                which ^= seg & 1;
                let wv = egrid
                    .resolve_wire((
                        DieId::from_idx(0),
                        (self.col_w(), self.row_s()),
                        db.get_wire(&format!("QUAD.V{which}.{seg}")),
                    ))
                    .unwrap();
                egrid.extra_conns.insert(wh, wv);
            }
            for i in 0..16 {
                let seg = i / 4;
                let which = i % 4;
                let wh = egrid
                    .resolve_wire((
                        DieId::from_idx(0),
                        (self.col_w(), self.row_n()),
                        db.get_wire(&format!("QUAD.H{which}.{seg}")),
                    ))
                    .unwrap();
                let mut seg = 3 - i / 12;
                let mut which = i % 12;
                which ^= seg & 1;
                seg += 1;
                let wv = egrid
                    .resolve_wire((
                        DieId::from_idx(0),
                        (self.col_w(), self.row_n()),
                        db.get_wire(&format!("QUAD.V{which}.{seg}")),
                    ))
                    .unwrap();
                egrid.extra_conns.insert(wh, wv);
            }
            for i in 0..16 {
                let seg = 1 + i / 4;
                let which = i % 4;
                let wh = egrid
                    .resolve_wire((
                        DieId::from_idx(0),
                        (self.col_e(), self.row_s()),
                        db.get_wire(&format!("QUAD.H{which}.{seg}")),
                    ))
                    .unwrap();
                let seg = 3 - (32 + i) / 12;
                let mut which = (32 + i) % 12;
                which ^= seg & 1;
                let wv = egrid
                    .resolve_wire((
                        DieId::from_idx(0),
                        (self.col_e(), self.row_s()),
                        db.get_wire(&format!("QUAD.V{which}.{seg}")),
                    ))
                    .unwrap();
                egrid.extra_conns.insert(wh, wv);
            }
            for i in 0..16 {
                let seg = 1 + i / 4;
                let which = i % 4;
                let wh = egrid
                    .resolve_wire((
                        DieId::from_idx(0),
                        (self.col_e(), self.row_n()),
                        db.get_wire(&format!("QUAD.H{which}.{seg}")),
                    ))
                    .unwrap();
                let mut seg = 3 - i / 12;
                let mut which = i % 12;
                which ^= seg & 1;
                seg += 1;
                let wv = egrid
                    .resolve_wire((
                        DieId::from_idx(0),
                        (self.col_e(), self.row_n()),
                        db.get_wire(&format!("QUAD.V{which}.{seg}")),
                    ))
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
