use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};
use prjcombine_interconnect::{
    db::IntDb,
    dir::{DirH, DirV},
    grid::{CellCoord, ColId, DieId, ExpandedGrid, RowId},
};
use prjcombine_xilinx_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};

use crate::{
    chip::{Chip, ChipKind},
    expanded::ExpandedDevice,
    xc2000, xc3000,
};

impl Chip {
    fn get_bio_tcls(&self, col: ColId) -> &'static str {
        assert!(self.kind.is_xc4000());
        if col == self.col_w() + 1 {
            "IO.BS.L"
        } else if col == self.col_e() - 1 {
            "IO.B.R"
        } else if col.to_idx().is_multiple_of(2) {
            "IO.B"
        } else {
            "IO.BS"
        }
    }

    fn get_tio_tcls(&self, col: ColId) -> &'static str {
        assert!(self.kind.is_xc4000());
        if col == self.col_w() + 1 {
            "IO.TS.L"
        } else if col == self.col_e() - 1 {
            "IO.T.R"
        } else if col.to_idx().is_multiple_of(2) {
            "IO.T"
        } else {
            "IO.TS"
        }
    }

    fn get_lio_tcls(&self, row: RowId) -> &'static str {
        assert!(self.kind.is_xc4000());
        if row == self.row_s() + 1 {
            "IO.LS.B"
        } else if row == self.row_n() - 1 {
            "IO.L.T"
        } else if self.kind.is_xl() && row == self.row_q(DirV::S) {
            if row.to_idx().is_multiple_of(2) {
                "IO.L.FB"
            } else {
                "IO.LS.FB"
            }
        } else if self.kind.is_xl() && row == self.row_q(DirV::N) - 1 {
            if row.to_idx().is_multiple_of(2) {
                "IO.L.FT"
            } else {
                "IO.LS.FT"
            }
        } else if row.to_idx().is_multiple_of(2) {
            "IO.L"
        } else {
            "IO.LS"
        }
    }

    fn get_rio_tcls(&self, row: RowId) -> &'static str {
        assert!(self.kind.is_xc4000());
        let row_f = if self.is_buff_large {
            self.row_q(DirV::S) + 1
        } else {
            self.row_q(DirV::S)
        };
        let row_f1 = if self.is_buff_large {
            self.row_q(DirV::N) - 2
        } else {
            self.row_q(DirV::N) - 1
        };
        if row == self.row_s() + 1 {
            "IO.RS.B"
        } else if row == self.row_n() - 1 {
            "IO.R.T"
        } else if self.kind.is_xl() && row == row_f {
            if row.to_idx().is_multiple_of(2) {
                "IO.R.FB"
            } else {
                "IO.RS.FB"
            }
        } else if self.kind.is_xl() && row == row_f1 {
            if row.to_idx().is_multiple_of(2) {
                "IO.R.FT"
            } else {
                "IO.RS.FT"
            }
        } else if row.to_idx().is_multiple_of(2) {
            "IO.R"
        } else {
            "IO.RS"
        }
    }

    pub fn expand_grid<'a>(&'a self, db: &'a IntDb) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        let die = egrid.add_die(self.columns, self.rows);

        let mut row_framebit = EntityVec::new();
        let mut llv_framebit = EntityPartVec::new();
        let mut frame_len = 0;
        let mut frame_info = vec![];
        let mut col_frame: EntityVec<_, _> = egrid.cols(die).map(|_| 0).collect();
        let mut llh_frame = EntityPartVec::new();

        match self.kind {
            ChipKind::Xc2000 => {
                for cell in egrid.die_cells(die) {
                    if cell.col == self.col_w() {
                        if cell.row == self.row_s() {
                            egrid.add_tile_id(
                                cell,
                                xc2000::tcls::CLB_SW,
                                &[cell, cell.delta(1, 0)],
                            );
                        } else if cell.row == self.row_n() {
                            egrid.add_tile_id(
                                cell,
                                xc2000::tcls::CLB_NW,
                                &[cell, cell.delta(0, -1), cell.delta(1, 0)],
                            );
                        } else if cell.row == self.row_mid() - 1 {
                            egrid.add_tile_id(
                                cell,
                                xc2000::tcls::CLB_MW,
                                &[cell, cell.delta(0, -1)],
                            );
                        } else {
                            egrid.add_tile_id(
                                cell,
                                xc2000::tcls::CLB_W,
                                &[cell, cell.delta(0, -1)],
                            );
                        }
                    } else if cell.col == self.col_e() {
                        if cell.row == self.row_s() {
                            egrid.add_tile_id(cell, xc2000::tcls::CLB_SE, &[cell]);
                        } else if cell.row == self.row_n() {
                            egrid.add_tile_id(
                                cell,
                                xc2000::tcls::CLB_NE,
                                &[cell, cell.delta(0, -1)],
                            );
                        } else if cell.row == self.row_mid() - 1 {
                            egrid.add_tile_id(
                                cell,
                                xc2000::tcls::CLB_ME,
                                &[cell, cell.delta(0, -1)],
                            );
                        } else {
                            egrid.add_tile_id(
                                cell,
                                xc2000::tcls::CLB_E,
                                &[cell, cell.delta(0, -1)],
                            );
                        }
                    } else {
                        if cell.row == self.row_s() {
                            let tcid = if cell.col == self.col_e() - 1 {
                                xc2000::tcls::CLB_SE1
                            } else {
                                xc2000::tcls::CLB_S
                            };
                            egrid.add_tile_id(cell, tcid, &[cell, cell.delta(1, 0)]);
                        } else if cell.row == self.row_n() {
                            let tcid = if cell.col == self.col_e() - 1 {
                                xc2000::tcls::CLB_NE1
                            } else {
                                xc2000::tcls::CLB_N
                            };
                            egrid.add_tile_id(cell, tcid, &[cell, cell.delta(1, 0)]);
                        } else {
                            egrid.add_tile_id(cell, xc2000::tcls::CLB, &[cell]);
                        }
                    }
                    if cell.col != self.col_w() {
                        egrid.fill_conn_pair_id(
                            cell.delta(-1, 0),
                            cell,
                            xc2000::ccls::PASS_E,
                            xc2000::ccls::PASS_W,
                        );
                    }
                    if cell.row != self.row_s() {
                        egrid.fill_conn_pair_id(
                            cell.delta(0, -1),
                            cell,
                            xc2000::ccls::PASS_N,
                            xc2000::ccls::PASS_S,
                        );
                    }
                }
                for &col in &self.cols_bidi {
                    for cell in egrid.column(die, col) {
                        if cell.row == self.row_s() {
                            egrid.add_tile_single_id(cell, xc2000::tcls::BIDIH_S);
                        } else if cell.row == self.row_n() {
                            egrid.add_tile_single_id(cell, xc2000::tcls::BIDIH_N);
                        } else {
                            egrid.add_tile_single_id(cell, xc2000::tcls::BIDIH);
                        }
                    }
                }
                for &row in &self.rows_bidi {
                    for cell in egrid.row(die, row) {
                        if cell.col == self.col_w() {
                            egrid.add_tile_id(cell, xc2000::tcls::BIDIV_W, &[cell.delta(0, -1)]);
                        } else if cell.col == self.col_e() {
                            egrid.add_tile_id(cell, xc2000::tcls::BIDIV_E, &[cell.delta(0, -1)]);
                        } else {
                            egrid.add_tile_id(cell, xc2000::tcls::BIDIV, &[cell.delta(0, -1)]);
                        }
                    }
                }
                for cell in egrid.die_cells(die) {
                    egrid[cell].region_root[xc2000::rslots::GLOBAL] =
                        cell.with_cr(self.col_w(), self.row_s());
                    egrid[cell].region_root[xc2000::rslots::LONG_H] = cell.with_col(self.col_w());
                    egrid[cell].region_root[xc2000::rslots::LONG_V] = cell.with_row(self.row_s());
                }

                for row in egrid.rows(die) {
                    if self.rows_bidi.contains(&row) {
                        llv_framebit.insert(row, frame_len);
                        frame_len += 1;
                    }
                    row_framebit.push(frame_len);
                    frame_len += self.btile_height_main(row);
                }

                for col in egrid.cols(die).rev() {
                    let width = self.btile_width_main(col);
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
                    if self.cols_bidi.contains(&col) {
                        let width = self.btile_width_brk();
                        llh_frame.insert(col, frame_info.len());
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
                    }
                }
            }
            ChipKind::Xc3000 | ChipKind::Xc3000A => {
                for cell in egrid.die_cells(die) {
                    let mut subkind =
                        (cell.row.to_idx() + 2 * (self.columns - 1 - cell.col.to_idx())) % 3;
                    if cell.col == self.col_w() {
                        if cell.row == self.row_s() {
                            egrid.add_tile_id(
                                cell,
                                if self.is_small {
                                    assert_eq!(subkind, 2);
                                    xc3000::tcls::CLB_SW2_S
                                } else {
                                    [
                                        xc3000::tcls::CLB_SW0_L,
                                        xc3000::tcls::CLB_SW1_L,
                                        xc3000::tcls::CLB_SW2_L,
                                    ][subkind]
                                },
                                &[cell, cell.delta(1, 0), cell.delta(0, 1)],
                            );
                        } else if cell.row == self.row_n() {
                            egrid.add_tile_id(
                                cell,
                                if self.is_small {
                                    assert_eq!(subkind, 0);
                                    xc3000::tcls::CLB_NW0_S
                                } else {
                                    [
                                        xc3000::tcls::CLB_NW0_L,
                                        xc3000::tcls::CLB_NW1_L,
                                        xc3000::tcls::CLB_NW2_L,
                                    ][subkind]
                                },
                                &[cell, cell.delta(1, 0), cell.delta(0, -1)],
                            );
                        } else {
                            egrid.add_tile_id(
                                cell,
                                [
                                    xc3000::tcls::CLB_W0,
                                    xc3000::tcls::CLB_W1,
                                    xc3000::tcls::CLB_W2,
                                ][subkind],
                                &[cell, cell.delta(1, 0), cell.delta(0, -1), cell.delta(0, 1)],
                            );
                        }
                    } else if cell.col == self.col_e() {
                        if cell.row == self.row_s() {
                            assert_eq!(subkind, 0);
                            egrid.add_tile_id(
                                cell,
                                if self.is_small {
                                    xc3000::tcls::CLB_SE0_S
                                } else {
                                    xc3000::tcls::CLB_SE0_L
                                },
                                &[cell, cell.delta(0, 1)],
                            );
                        } else if cell.row == self.row_n() {
                            egrid.add_tile_id(
                                cell,
                                if self.is_small {
                                    assert_eq!(subkind, 1);
                                    xc3000::tcls::CLB_NE1_S
                                } else {
                                    [
                                        xc3000::tcls::CLB_NE0_L,
                                        xc3000::tcls::CLB_NE1_L,
                                        xc3000::tcls::CLB_NE2_L,
                                    ][subkind]
                                },
                                &[cell, cell.delta(0, -1)],
                            );
                        } else {
                            if subkind == 1 && cell.row == self.row_n() - 1 {
                                // fuck me with the rustiest fork you can find
                                subkind = 3;
                            }
                            egrid.add_tile_id(
                                cell,
                                [
                                    xc3000::tcls::CLB_E0,
                                    xc3000::tcls::CLB_E1,
                                    xc3000::tcls::CLB_E2,
                                    xc3000::tcls::CLB_E3,
                                ][subkind],
                                &[cell, cell.delta(0, -1), cell.delta(0, 1)],
                            );
                        }
                    } else {
                        if cell.row == self.row_s() {
                            egrid.add_tile_id(
                                cell,
                                [
                                    xc3000::tcls::CLB_S0,
                                    xc3000::tcls::CLB_S1,
                                    xc3000::tcls::CLB_S2,
                                ][subkind],
                                &[cell, cell.delta(1, 0), cell.delta(0, 1)],
                            );
                        } else if cell.row == self.row_n() {
                            egrid.add_tile_id(
                                cell,
                                if self.is_small {
                                    [
                                        xc3000::tcls::CLB_N0_S,
                                        xc3000::tcls::CLB_N1_S,
                                        xc3000::tcls::CLB_N2_S,
                                    ][subkind]
                                } else {
                                    [
                                        xc3000::tcls::CLB_N0_L,
                                        xc3000::tcls::CLB_N1_L,
                                        xc3000::tcls::CLB_N2_L,
                                    ][subkind]
                                },
                                &[cell, cell.delta(1, 0), cell.delta(0, -1)],
                            );
                        } else {
                            egrid.add_tile_id(
                                cell,
                                [xc3000::tcls::CLB0, xc3000::tcls::CLB1, xc3000::tcls::CLB2]
                                    [subkind],
                                &[cell, cell.delta(1, 0), cell.delta(0, -1), cell.delta(0, 1)],
                            );
                        }
                    }
                    if cell.col != self.col_w() {
                        egrid.fill_conn_pair_id(
                            cell.delta(-1, 0),
                            cell,
                            xc3000::ccls::PASS_E,
                            xc3000::ccls::PASS_W,
                        );
                    }
                    if cell.row != self.row_s() {
                        egrid.fill_conn_pair_id(
                            cell.delta(0, -1),
                            cell,
                            xc3000::ccls::PASS_N,
                            xc3000::ccls::PASS_S,
                        );
                    }
                }
                {
                    let cell = CellCoord::new(die, self.col_mid(), self.row_s());
                    egrid.add_tile_id(cell, xc3000::tcls::LLH_S, &[cell.delta(-1, 0), cell]);
                    let cell = CellCoord::new(die, self.col_mid(), self.row_n());
                    egrid.add_tile_id(cell, xc3000::tcls::LLH_N, &[cell.delta(-1, 0), cell]);
                }
                if self.is_small {
                    let cell = CellCoord::new(die, self.col_w(), self.row_mid());
                    egrid.add_tile_id(cell, xc3000::tcls::LLVS_W, &[cell.delta(0, -1), cell]);
                    let cell = CellCoord::new(die, self.col_e(), self.row_mid());
                    egrid.add_tile_id(cell, xc3000::tcls::LLVS_E, &[cell.delta(0, -1), cell]);
                } else {
                    for cell in egrid.row(die, self.row_mid()) {
                        let tcid = if cell.col == self.col_w() {
                            xc3000::tcls::LLV_W
                        } else if cell.col == self.col_e() {
                            xc3000::tcls::LLV_E
                        } else {
                            xc3000::tcls::LLV
                        };
                        egrid.add_tile_id(cell, tcid, &[cell.delta(0, -1), cell]);
                    }
                }
                let cell = CellCoord::new(die, self.col_e(), self.row_mid());
                egrid.add_tile_id(cell, xc3000::tcls::MISC_E, &[]);
                for cell in egrid.die_cells(die) {
                    egrid[cell].region_root[xc3000::rslots::GLOBAL] =
                        cell.with_cr(self.col_w(), self.row_s());
                    egrid[cell].region_root[xc3000::rslots::LONG_H] = cell.with_col(self.col_w());
                    egrid[cell].region_root[xc3000::rslots::LONG_H_IO0] =
                        cell.with_col(if cell.col < self.col_mid() {
                            self.col_w()
                        } else {
                            self.col_e()
                        });
                    egrid[cell].region_root[xc3000::rslots::LONG_V] =
                        cell.with_row(if cell.row < self.row_mid() || self.is_small {
                            self.row_s()
                        } else {
                            self.row_n()
                        });
                    egrid[cell].region_root[xc3000::rslots::LONG_V_IO0] =
                        cell.with_row(if cell.row < self.row_mid() {
                            self.row_s()
                        } else {
                            self.row_n()
                        });
                    egrid[cell].region_root[xc3000::rslots::LONG_V_IO1] =
                        cell.with_row(self.row_s());
                }

                for row in egrid.rows(die) {
                    if row == self.row_mid() && !self.is_small {
                        llv_framebit.insert(row, frame_len);
                        frame_len += 1;
                    }
                    row_framebit.push(frame_len);
                    frame_len += self.btile_height_main(row);
                }

                for col in egrid.cols(die).rev() {
                    let width = self.btile_width_main(col);
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
                }
            }
            ChipKind::Xc4000
            | ChipKind::Xc4000A
            | ChipKind::Xc4000H
            | ChipKind::Xc4000E
            | ChipKind::Xc4000Ex
            | ChipKind::Xc4000Xla
            | ChipKind::Xc4000Xv
            | ChipKind::SpartanXl => {
                for cell in egrid.die_cells(die) {
                    if cell.col == self.col_w() {
                        if cell.row == self.row_s() {
                            egrid.add_tile(cell, "CNR.BL", &[cell, cell.delta(1, 0)]);
                        } else if cell.row == self.row_n() {
                            egrid.add_tile(
                                cell,
                                "CNR.TL",
                                &[cell, cell.delta(1, 0), cell.delta(0, -1), cell.delta(1, -1)],
                            );
                        } else {
                            egrid.add_tile(
                                cell,
                                self.get_lio_tcls(cell.row),
                                &[cell, cell.delta(0, -1), cell.delta(1, 0), cell.delta(0, 1)],
                            );
                        }
                    } else if cell.col == self.col_e() {
                        if cell.row == self.row_s() {
                            egrid.add_tile_single(cell, "CNR.BR");
                        } else if cell.row == self.row_n() {
                            egrid.add_tile(cell, "CNR.TR", &[cell, cell.delta(0, -1)]);
                        } else {
                            egrid.add_tile(
                                cell,
                                self.get_rio_tcls(cell.row),
                                &[cell, cell.delta(0, -1), cell.delta(0, 1)],
                            );
                        }
                    } else {
                        if cell.row == self.row_s() {
                            egrid.add_tile(
                                cell,
                                self.get_bio_tcls(cell.col),
                                &[cell, cell.delta(0, 1), cell.delta(1, 0), cell.delta(-1, 0)],
                            );
                        } else if cell.row == self.row_n() {
                            egrid.add_tile(
                                cell,
                                self.get_tio_tcls(cell.col),
                                &[cell, cell.delta(1, 0), cell.delta(-1, 0)],
                            );
                        } else {
                            let kind = if cell.row == self.row_s() + 1 {
                                if cell.col == self.col_w() + 1 {
                                    "CLB.LB"
                                } else if cell.col == self.col_e() - 1 {
                                    "CLB.RB"
                                } else {
                                    "CLB.B"
                                }
                            } else if cell.row == self.row_n() - 1 {
                                if cell.col == self.col_w() + 1 {
                                    "CLB.LT"
                                } else if cell.col == self.col_e() - 1 {
                                    "CLB.RT"
                                } else {
                                    "CLB.T"
                                }
                            } else {
                                if cell.col == self.col_w() + 1 {
                                    "CLB.L"
                                } else if cell.col == self.col_e() - 1 {
                                    "CLB.R"
                                } else {
                                    "CLB"
                                }
                            };
                            egrid.add_tile(cell, kind, &[cell, cell.delta(0, 1), cell.delta(1, 0)]);
                        }
                    }
                }

                if self.kind.is_xl() {
                    for col in [self.col_q(DirH::W), self.col_q(DirH::E)] {
                        for cell in egrid.column(die, col) {
                            if cell.row == self.row_s() || cell.row == self.row_n() {
                                egrid.fill_conn_pair(
                                    cell.delta(-1, 0),
                                    cell,
                                    "LLHQ.IO.E",
                                    "LLHQ.IO.W",
                                );
                            } else {
                                egrid.fill_conn_pair(cell.delta(-1, 0), cell, "LLHQ.E", "LLHQ.W");
                            }
                            let kind = if cell.row == self.row_s() {
                                "LLHQ.IO.B"
                            } else if cell.row == self.row_n() {
                                "LLHQ.IO.T"
                            } else {
                                if cell.row == self.row_s() + 1 {
                                    "LLHQ.CLB.B"
                                } else if cell.row == self.row_n() - 1 {
                                    "LLHQ.CLB.T"
                                } else {
                                    "LLHQ.CLB"
                                }
                            };
                            egrid.add_tile(cell, kind, &[cell.delta(-1, 0), cell]);
                        }
                    }
                    for cell in egrid.column(die, self.col_mid()) {
                        egrid.fill_conn_pair(cell.delta(-1, 0), cell, "LLHC.E", "LLHC.W");
                        let kind = if cell.row == self.row_s() {
                            "LLHC.IO.B"
                        } else if cell.row == self.row_n() {
                            "LLHC.IO.T"
                        } else if cell.row == self.row_s() + 1 {
                            "LLHC.CLB.B"
                        } else {
                            "LLHC.CLB"
                        };
                        egrid.add_tile(cell, kind, &[cell.delta(-1, 0), cell]);
                    }

                    for (bt, row) in [('B', self.row_q(DirV::S)), ('T', self.row_q(DirV::N))] {
                        for cell in egrid.row(die, row) {
                            egrid.fill_conn_pair(cell.delta(0, -1), cell, "LLVQ.N", "LLVQ.S");
                            let kind = if cell.col == self.col_w() {
                                if bt == 'B' {
                                    "LLVQ.IO.L.B"
                                } else {
                                    "LLVQ.IO.L.T"
                                }
                            } else if cell.col == self.col_e() {
                                if bt == 'B' {
                                    "LLVQ.IO.R.B"
                                } else {
                                    "LLVQ.IO.R.T"
                                }
                            } else {
                                "LLVQ.CLB"
                            };
                            egrid.add_tile(cell, kind, &[cell.delta(0, -1), cell]);
                        }
                    }
                    for cell in egrid.row(die, self.row_mid()) {
                        egrid.fill_conn_pair(cell.delta(0, -1), cell, "LLVC.N", "LLVC.S");
                        let kind = if cell.col == self.col_w() {
                            "LLVC.IO.L"
                        } else if cell.col == self.col_e() {
                            "LLVC.IO.R"
                        } else {
                            "LLVC.CLB"
                        };
                        egrid.add_tile(cell, kind, &[cell.delta(0, -1), cell]);
                    }

                    if self.kind == ChipKind::Xc4000Xv {
                        for row in [self.row_q(DirV::S), self.row_q(DirV::N)] {
                            for col in [self.col_q(DirH::W), self.col_q(DirH::E)] {
                                let cell = CellCoord::new(die, col, row);
                                egrid.add_tile(cell, "CLKQ", &[cell.delta(-1, 0), cell]);
                            }
                        }
                    } else {
                        egrid.add_tile(
                            CellCoord::new(die, self.col_mid(), self.row_mid()),
                            "CLKC",
                            &[],
                        );
                        egrid.add_tile_single(
                            CellCoord::new(die, self.col_mid(), self.row_q(DirV::S)),
                            "CLKQC",
                        );
                        egrid.add_tile_single(
                            CellCoord::new(die, self.col_mid(), self.row_q(DirV::N)),
                            "CLKQC",
                        );
                    }
                } else {
                    for cell in egrid.column(die, self.col_mid()) {
                        egrid.fill_conn_pair(cell.delta(-1, 0), cell, "LLHC.E", "LLHC.W");
                        let kind = if cell.row == self.row_s() {
                            "LLH.IO.B"
                        } else if cell.row == self.row_n() {
                            "LLH.IO.T"
                        } else if cell.row == self.row_s() + 1 {
                            "LLH.CLB.B"
                        } else {
                            "LLH.CLB"
                        };
                        egrid.add_tile(cell, kind, &[cell.delta(-1, 0), cell]);
                    }

                    for cell in egrid.row(die, self.row_mid()) {
                        egrid.fill_conn_pair(cell.delta(0, -1), cell, "LLVC.N", "LLVC.S");
                        let kind = if cell.col == self.col_w() {
                            "LLV.IO.L"
                        } else if cell.col == self.col_e() {
                            "LLV.IO.R"
                        } else {
                            "LLV.CLB"
                        };
                        egrid.add_tile(cell, kind, &[cell.delta(0, -1), cell]);
                    }
                }

                for cell in egrid.row(die, self.row_n()) {
                    if cell.col != self.col_w() && cell.col != self.col_e() {
                        egrid.fill_conn_pair(cell.delta(0, -1), cell, "TCLB.N", "MAIN.S");
                    }
                }

                for cell in egrid.column(die, self.col_w()) {
                    if cell.row != self.row_s() && cell.row != self.row_n() {
                        egrid.fill_conn_pair(cell, cell.delta(1, 0), "MAIN.E", "LCLB.W");
                    }
                }

                egrid.fill_main_passes(die);
                egrid.fill_conn_term(CellCoord::new(die, self.col_w(), self.row_s()), "CNR.LL.W");
                egrid.fill_conn_term(CellCoord::new(die, self.col_e(), self.row_s()), "CNR.LR.S");
                egrid.fill_conn_term(CellCoord::new(die, self.col_w(), self.row_n()), "CNR.UL.N");
                egrid.fill_conn_term(CellCoord::new(die, self.col_e(), self.row_n()), "CNR.UR.E");

                for row in egrid.rows(die) {
                    if self.kind.is_xl()
                        && (row == self.row_q(DirV::S) || row == self.row_q(DirV::N))
                    {
                        llv_framebit.insert(row, frame_len);
                        frame_len += self.btile_height_brk();
                    }
                    if row == self.row_mid() {
                        if matches!(self.kind, ChipKind::Xc4000Ex | ChipKind::Xc4000Xla) {
                            // padding
                            frame_len += 2;
                        }
                        llv_framebit.insert(row, frame_len);
                        frame_len += self.btile_height_clk();
                    }
                    row_framebit.push(frame_len);
                    let height = self.btile_height_main(row);
                    frame_len += height;
                }

                for col in egrid.cols(die).rev() {
                    let width = self.btile_width_main(col);
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
                        let width = self.btile_width_clk();
                        llh_frame.insert(col, frame_info.len());
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
                    }
                    if self.kind.is_xl()
                        && (col == self.col_q(DirH::W) || col == self.col_q(DirH::E))
                    {
                        let minor = frame_info.len();
                        llh_frame.insert(col, frame_info.len());
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
            }
            ChipKind::Xc5200 => {
                for cell in egrid.die_cells(die) {
                    if cell.col == self.col_w() {
                        if cell.row == self.row_s() {
                            egrid.add_tile_single(cell, "CNR.BL");
                        } else if cell.row == self.row_n() {
                            egrid.add_tile_single(cell, "CNR.TL");
                        } else {
                            egrid.add_tile_single(cell, "IO.L");
                        }
                    } else if cell.col == self.col_e() {
                        if cell.row == self.row_s() {
                            egrid.add_tile_single(cell, "CNR.BR");
                        } else if cell.row == self.row_n() {
                            egrid.add_tile_single(cell, "CNR.TR");
                        } else {
                            egrid.add_tile_single(cell, "IO.R");
                        }
                    } else {
                        if cell.row == self.row_s() {
                            egrid.add_tile_single(cell, "IO.B");
                        } else if cell.row == self.row_n() {
                            egrid.add_tile_single(cell, "IO.T");
                        } else {
                            egrid.add_tile_single(cell, "CLB");
                        }
                    }
                }

                for cell in egrid.row(die, self.row_mid()) {
                    let kind = if cell.col == self.col_w() {
                        "CLKL"
                    } else if cell.col == self.col_e() {
                        "CLKR"
                    } else {
                        "CLKH"
                    };
                    egrid.fill_conn_pair(cell.delta(0, -1), cell, "LLV.N", "LLV.S");
                    egrid.add_tile(cell, kind, &[cell.delta(0, -1), cell]);
                }

                for cell in egrid.column(die, self.col_mid()) {
                    let kind = if cell.row == self.row_s() {
                        "CLKB"
                    } else if cell.row == self.row_n() {
                        "CLKT"
                    } else {
                        "CLKV"
                    };
                    egrid.fill_conn_pair(cell.delta(-1, 0), cell, "LLH.E", "LLH.W");
                    egrid.add_tile(cell, kind, &[cell.delta(-1, 0), cell]);
                }

                egrid.fill_main_passes(die);
                egrid.fill_conn_term(CellCoord::new(die, self.col_w(), self.row_s()), "CNR.LL");
                egrid.fill_conn_term(CellCoord::new(die, self.col_e(), self.row_s()), "CNR.LR");
                egrid.fill_conn_term(CellCoord::new(die, self.col_w(), self.row_n()), "CNR.UL");
                egrid.fill_conn_term(CellCoord::new(die, self.col_e(), self.row_n()), "CNR.UR");

                for row in egrid.rows(die) {
                    if row == self.row_mid() {
                        llv_framebit.insert(row, frame_len);
                        frame_len += 4;
                    }
                    row_framebit.push(frame_len);
                    let height = self.btile_height_main(row);
                    frame_len += height;
                }

                for col in egrid.cols(die).rev() {
                    let width = self.btile_width_main(col);
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
                        llh_frame.insert(col, minor);
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
            }
        }

        egrid.finish();

        let die_bs_geom = DieBitstreamGeom {
            frame_len,
            frame_info,
            bram_frame_len: 0,
            bram_frame_info: vec![],
            iob_frame_len: 0,
        };
        let bs_geom = BitstreamGeom {
            kind: match self.kind {
                ChipKind::Xc2000 | ChipKind::Xc3000 | ChipKind::Xc3000A => DeviceKind::Xc2000,
                ChipKind::Xc4000
                | ChipKind::Xc4000A
                | ChipKind::Xc4000H
                | ChipKind::Xc4000E
                | ChipKind::Xc4000Ex
                | ChipKind::Xc4000Xla
                | ChipKind::Xc4000Xv => DeviceKind::Xc4000,
                ChipKind::SpartanXl => {
                    if self.columns == 30 {
                        DeviceKind::S40Xl
                    } else {
                        DeviceKind::Xc4000
                    }
                }
                ChipKind::Xc5200 => DeviceKind::Xc5200,
            },
            die: [die_bs_geom].into_iter().collect(),
            die_order: vec![DieId::from_idx(0)],
            has_gtz_bot: false,
            has_gtz_top: false,
        };

        ExpandedDevice {
            chip: self,
            egrid,
            bs_geom,
            col_frame,
            llh_frame,
            row_framebit,
            llv_framebit,
        }
    }
}
