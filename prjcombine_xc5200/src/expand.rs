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
        let (_, mut grid) = egrid.add_die(self.columns, self.rows);

        let col_l = grid.cols().next().unwrap();
        let col_r = grid.cols().next_back().unwrap();
        let row_b = grid.rows().next().unwrap();
        let row_t = grid.rows().next_back().unwrap();
        for col in grid.cols() {
            if col == col_l {
                for row in grid.rows() {
                    if row == row_b {
                        grid.fill_tile((col, row), "CNR.BL");
                    } else if row == row_t {
                        grid.fill_tile((col, row), "CNR.TL");
                    } else {
                        grid.fill_tile((col, row), "IO.L");
                    }
                }
            } else if col == col_r {
                for row in grid.rows() {
                    if row == row_b {
                        grid.fill_tile((col, row), "CNR.BR");
                    } else if row == row_t {
                        grid.fill_tile((col, row), "CNR.TR");
                    } else {
                        grid.fill_tile((col, row), "IO.R");
                    }
                }
            } else {
                for row in grid.rows() {
                    if row == row_b {
                        grid.fill_tile((col, row), "IO.B");
                    } else if row == row_t {
                        grid.fill_tile((col, row), "IO.T");
                    } else {
                        grid.fill_tile((col, row), "CLB");
                    }
                }
            }
        }

        for col in grid.cols() {
            let kind = if col == col_l {
                "CLKL"
            } else if col == col_r {
                "CLKR"
            } else {
                "CLKH"
            };
            let row_s = self.row_mid() - 1;
            let row_n = self.row_mid();
            grid.fill_term_pair((col, row_s), (col, row_n), "LLV.N", "LLV.S");
            grid.add_xnode((col, row_n), kind, &[(col, row_s), (col, row_n)]);
        }

        for row in grid.rows() {
            let kind = if row == row_b {
                "CLKB"
            } else if row == row_t {
                "CLKT"
            } else {
                "CLKV"
            };
            let col_l = self.col_mid() - 1;
            let col_r = self.col_mid();
            grid.fill_term_pair((col_l, row), (col_r, row), "LLH.E", "LLH.W");
            grid.add_xnode((col_r, row), kind, &[(col_l, row), (col_r, row)]);
        }

        grid.fill_main_passes();
        grid.fill_term((col_l, row_b), "CNR.LL");
        grid.fill_term((col_r, row_b), "CNR.LR");
        grid.fill_term((col_l, row_t), "CNR.UL");
        grid.fill_term((col_r, row_t), "CNR.UR");

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
                    mask_mode: [].into_iter().collect(),
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
                    mask_mode: [].into_iter().collect(),
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
