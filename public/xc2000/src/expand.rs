use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};
use prjcombine_interconnect::{
    db::{IntDb, TileClassId},
    dir::{DirH, DirHV, DirV},
    grid::{CellCoord, ColId, DieId, RowId, builder::GridBuilder},
};
use prjcombine_xilinx_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};

use crate::{
    chip::{Chip, ChipKind},
    expanded::ExpandedDevice,
    xc2000, xc3000, xc4000, xc5200,
};

impl Chip {
    fn get_tcls_io_s(&self, col: ColId) -> TileClassId {
        assert!(self.kind.is_xc4000());
        if col == self.col_w() + 1 {
            xc4000::xc4000::tcls::IO_S1_W
        } else if col == self.col_e() - 1 {
            xc4000::xc4000::tcls::IO_S0_E
        } else if col.to_idx().is_multiple_of(2) {
            xc4000::xc4000::tcls::IO_S0
        } else {
            xc4000::xc4000::tcls::IO_S1
        }
    }

    fn get_tcls_io_n(&self, col: ColId) -> TileClassId {
        assert!(self.kind.is_xc4000());
        if col == self.col_w() + 1 {
            xc4000::xc4000::tcls::IO_N1_W
        } else if col == self.col_e() - 1 {
            xc4000::xc4000::tcls::IO_N0_E
        } else if col.to_idx().is_multiple_of(2) {
            xc4000::xc4000::tcls::IO_N0
        } else {
            xc4000::xc4000::tcls::IO_N1
        }
    }

    fn get_tcls_io_w(&self, row: RowId) -> TileClassId {
        assert!(self.kind.is_xc4000());
        if row == self.row_s() + 1 {
            xc4000::xc4000::tcls::IO_W1_S
        } else if row == self.row_n() - 1 {
            xc4000::xc4000::tcls::IO_W0_N
        } else if self.kind.is_xl() && row == self.bel_buff_io(DirHV::SW).row {
            if row.to_idx().is_multiple_of(2) {
                xc4000::xc4000::tcls::IO_W0_F1
            } else {
                xc4000::xc4000::tcls::IO_W1_F1
            }
        } else if self.kind.is_xl() && row == self.bel_buff_io(DirHV::NW).row {
            if row.to_idx().is_multiple_of(2) {
                xc4000::xc4000::tcls::IO_W0_F0
            } else {
                xc4000::xc4000::tcls::IO_W1_F0
            }
        } else if row.to_idx().is_multiple_of(2) {
            xc4000::xc4000::tcls::IO_W0
        } else {
            xc4000::xc4000::tcls::IO_W1
        }
    }

    fn get_tcls_io_e(&self, row: RowId) -> TileClassId {
        assert!(self.kind.is_xc4000());
        if row == self.row_s() + 1 {
            xc4000::xc4000::tcls::IO_E1_S
        } else if row == self.row_n() - 1 {
            xc4000::xc4000::tcls::IO_E0_N
        } else if self.kind.is_xl() && row == self.bel_buff_io(DirHV::SE).row {
            if row.to_idx().is_multiple_of(2) {
                xc4000::xc4000::tcls::IO_E0_F1
            } else {
                xc4000::xc4000::tcls::IO_E1_F1
            }
        } else if self.kind.is_xl() && row == self.bel_buff_io(DirHV::NE).row {
            if row.to_idx().is_multiple_of(2) {
                xc4000::xc4000::tcls::IO_E0_F0
            } else {
                xc4000::xc4000::tcls::IO_E1_F0
            }
        } else if row.to_idx().is_multiple_of(2) {
            xc4000::xc4000::tcls::IO_E0
        } else {
            xc4000::xc4000::tcls::IO_E1
        }
    }

    pub fn expand_grid<'a>(&'a self, db: &'a IntDb) -> ExpandedDevice<'a> {
        let mut egrid = GridBuilder::new(db);
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
                            egrid.add_tile_id(
                                cell,
                                xc4000::xc4000::tcls::CNR_SW,
                                &[cell, cell.delta(1, 0)],
                            );
                        } else if cell.row == self.row_n() {
                            egrid.add_tile_id(
                                cell,
                                xc4000::xc4000::tcls::CNR_NW,
                                &[cell, cell.delta(1, 0), cell.delta(0, -1), cell.delta(1, -1)],
                            );
                        } else {
                            egrid.add_tile_id(
                                cell,
                                self.get_tcls_io_w(cell.row),
                                &[cell, cell.delta(0, -1), cell.delta(1, 0), cell.delta(0, 1)],
                            );
                        }
                    } else if cell.col == self.col_e() {
                        if cell.row == self.row_s() {
                            egrid.add_tile_single_id(cell, xc4000::xc4000::tcls::CNR_SE);
                        } else if cell.row == self.row_n() {
                            egrid.add_tile_id(
                                cell,
                                xc4000::xc4000::tcls::CNR_NE,
                                &[cell, cell.delta(0, -1)],
                            );
                        } else {
                            egrid.add_tile_id(
                                cell,
                                self.get_tcls_io_e(cell.row),
                                &[cell, cell.delta(0, -1), cell.delta(0, 1)],
                            );
                        }
                    } else {
                        if cell.row == self.row_s() {
                            egrid.add_tile_id(
                                cell,
                                self.get_tcls_io_s(cell.col),
                                &[cell, cell.delta(0, 1), cell.delta(1, 0), cell.delta(-1, 0)],
                            );
                        } else if cell.row == self.row_n() {
                            egrid.add_tile_id(
                                cell,
                                self.get_tcls_io_n(cell.col),
                                &[cell, cell.delta(1, 0), cell.delta(-1, 0)],
                            );
                        } else {
                            let tcid = if cell.row == self.row_s() + 1 {
                                if cell.col == self.col_w() + 1 {
                                    xc4000::xc4000::tcls::CLB_SW
                                } else if cell.col == self.col_e() - 1 {
                                    xc4000::xc4000::tcls::CLB_SE
                                } else {
                                    xc4000::xc4000::tcls::CLB_S
                                }
                            } else if cell.row == self.row_n() - 1 {
                                if cell.col == self.col_w() + 1 {
                                    xc4000::xc4000::tcls::CLB_NW
                                } else if cell.col == self.col_e() - 1 {
                                    xc4000::xc4000::tcls::CLB_NE
                                } else {
                                    xc4000::xc4000::tcls::CLB_N
                                }
                            } else {
                                if cell.col == self.col_w() + 1 {
                                    xc4000::xc4000::tcls::CLB_W
                                } else if cell.col == self.col_e() - 1 {
                                    xc4000::xc4000::tcls::CLB_E
                                } else {
                                    xc4000::xc4000::tcls::CLB
                                }
                            };
                            egrid.add_tile_id(
                                cell,
                                tcid,
                                &[cell, cell.delta(0, 1), cell.delta(1, 0)],
                            );
                        }
                    }
                }

                if self.kind.is_xl() {
                    for col in [self.col_q(DirH::W), self.col_q(DirH::E)] {
                        for cell in egrid.column(die, col) {
                            let tcid = if cell.row == self.row_s() {
                                xc4000::xc4000::tcls::LLHQ_IO_S
                            } else if cell.row == self.row_n() {
                                xc4000::xc4000::tcls::LLHQ_IO_N
                            } else {
                                if cell.row == self.row_s() + 1 {
                                    xc4000::xc4000::tcls::LLHQ_CLB_S
                                } else if cell.row == self.row_n() - 1 {
                                    xc4000::xc4000::tcls::LLHQ_CLB_N
                                } else {
                                    xc4000::xc4000::tcls::LLHQ_CLB
                                }
                            };
                            egrid.add_tile_id(cell, tcid, &[cell.delta(-1, 0), cell]);
                        }
                    }
                    for cell in egrid.column(die, self.col_mid()) {
                        let tcid = if cell.row == self.row_s() {
                            xc4000::xc4000::tcls::LLHC_IO_S
                        } else if cell.row == self.row_n() {
                            xc4000::xc4000::tcls::LLHC_IO_N
                        } else if cell.row == self.row_s() + 1 {
                            xc4000::xc4000::tcls::LLHC_CLB_S
                        } else {
                            xc4000::xc4000::tcls::LLHC_CLB
                        };
                        egrid.add_tile_id(cell, tcid, &[cell.delta(-1, 0), cell]);
                    }

                    for (bt, row) in [('B', self.row_q(DirV::S)), ('T', self.row_q(DirV::N))] {
                        for cell in egrid.row(die, row) {
                            let tcid = if cell.col == self.col_w() {
                                if bt == 'B' {
                                    xc4000::xc4000::tcls::LLVQ_IO_SW
                                } else {
                                    xc4000::xc4000::tcls::LLVQ_IO_NW
                                }
                            } else if cell.col == self.col_e() {
                                if bt == 'B' {
                                    xc4000::xc4000::tcls::LLVQ_IO_SE
                                } else {
                                    xc4000::xc4000::tcls::LLVQ_IO_NE
                                }
                            } else {
                                xc4000::xc4000::tcls::LLVQ_CLB
                            };
                            egrid.add_tile_id(cell, tcid, &[cell.delta(0, -1), cell]);
                        }
                    }
                    for cell in egrid.row(die, self.row_mid()) {
                        let tcid = if cell.col == self.col_w() {
                            xc4000::xc4000::tcls::LLVC_IO_W
                        } else if cell.col == self.col_e() {
                            xc4000::xc4000::tcls::LLVC_IO_E
                        } else {
                            xc4000::xc4000::tcls::LLVC_CLB
                        };
                        egrid.add_tile_id(cell, tcid, &[cell.delta(0, -1), cell]);
                    }

                    if self.kind == ChipKind::Xc4000Xv {
                        for row in [self.row_q(DirV::S), self.row_q(DirV::N)] {
                            for col in [self.col_q(DirH::W), self.col_q(DirH::E)] {
                                let cell = CellCoord::new(die, col, row);
                                egrid.add_tile_id(
                                    cell,
                                    xc4000::xc4000::tcls::CLKQ,
                                    &[cell.delta(-1, 0), cell],
                                );
                            }
                        }
                    } else {
                        egrid.add_tile_single_id(
                            CellCoord::new(die, self.col_mid(), self.row_q(DirV::S)),
                            xc4000::xc4000::tcls::CLKQC,
                        );
                        egrid.add_tile_single_id(
                            CellCoord::new(die, self.col_mid(), self.row_q(DirV::N)),
                            xc4000::xc4000::tcls::CLKQC,
                        );
                    }

                    for cell in egrid.die_cells(die) {
                        egrid[cell].region_root[xc4000::rslots::GLOBAL] =
                            cell.with_cr(self.col_w(), self.row_s());
                        let root_h = cell.with_col(if cell.col < self.col_mid() {
                            self.col_w()
                        } else {
                            self.col_e()
                        });
                        let root_qh = cell.with_col(if cell.col < self.col_mid() {
                            if cell.col < self.col_q(DirH::W) {
                                self.col_w()
                            } else {
                                self.col_q(DirH::W)
                            }
                        } else {
                            if cell.col < self.col_q(DirH::E) {
                                self.col_q(DirH::E) - 1
                            } else {
                                self.col_e()
                            }
                        });
                        egrid[cell].region_root[xc4000::rslots::LONG_H] = root_qh;
                        if self.kind == ChipKind::Xc4000Xv {
                            egrid[cell].region_root[xc4000::rslots::BUFGLS_H] = root_qh;
                        } else {
                            egrid[cell].region_root[xc4000::rslots::BUFGLS_H] =
                                cell.with_col(self.col_mid());
                        }
                        egrid[cell].region_root[xc4000::rslots::LONG_H_TBUF] =
                            if cell.row == self.row_s() || cell.row == self.row_n() {
                                root_qh
                            } else {
                                root_h
                            };
                        egrid[cell].region_root[xc4000::rslots::DEC_H] = root_h;
                        let root_v = cell.with_row(if cell.row < self.row_mid() {
                            self.row_s()
                        } else {
                            self.row_n()
                        });
                        let root_qv = cell.with_row(if cell.row < self.row_mid() {
                            if cell.row < self.row_q(DirV::S) {
                                self.row_s()
                            } else {
                                self.row_q(DirV::S)
                            }
                        } else {
                            if cell.row < self.row_q(DirV::N) {
                                self.row_q(DirV::N) - 1
                            } else {
                                self.row_n()
                            }
                        });
                        egrid[cell].region_root[xc4000::rslots::LONG_V] = root_qv;
                        egrid[cell].region_root[xc4000::rslots::DEC_V] = root_v;
                        egrid[cell].region_root[xc4000::rslots::GCLK] =
                            cell.with_row(if cell.row < self.row_mid() {
                                self.row_q(DirV::S)
                            } else {
                                self.row_q(DirV::N)
                            });
                        egrid[cell].region_root[xc4000::rslots::BUFGE_V] =
                            cell.with_row(self.row_s());
                    }
                } else {
                    for cell in egrid.column(die, self.col_mid()) {
                        let tcid = if cell.row == self.row_s() {
                            xc4000::xc4000::tcls::LLH_IO_S
                        } else if cell.row == self.row_n() {
                            xc4000::xc4000::tcls::LLH_IO_N
                        } else if cell.row == self.row_s() + 1 {
                            xc4000::xc4000::tcls::LLH_CLB_S
                        } else {
                            xc4000::xc4000::tcls::LLH_CLB
                        };
                        egrid.add_tile_id(cell, tcid, &[cell.delta(-1, 0), cell]);
                    }

                    for cell in egrid.row(die, self.row_mid()) {
                        let tcid = if cell.col == self.col_w() {
                            xc4000::xc4000::tcls::LLV_IO_W
                        } else if cell.col == self.col_e() {
                            xc4000::xc4000::tcls::LLV_IO_E
                        } else {
                            xc4000::xc4000::tcls::LLV_CLB
                        };
                        egrid.add_tile_id(cell, tcid, &[cell.delta(0, -1), cell]);
                    }

                    for cell in egrid.die_cells(die) {
                        egrid[cell].region_root[xc4000::rslots::GLOBAL] =
                            cell.with_cr(self.col_w(), self.row_s());
                        let root_h = cell.with_col(if cell.col < self.col_mid() {
                            self.col_w()
                        } else {
                            self.col_e()
                        });
                        egrid[cell].region_root[xc4000::rslots::LONG_H] = root_h;
                        egrid[cell].region_root[xc4000::rslots::LONG_H_TBUF] = root_h;
                        egrid[cell].region_root[xc4000::rslots::DEC_H] = root_h;
                        let root_v = cell.with_row(if cell.row < self.row_mid() {
                            self.row_s()
                        } else {
                            self.row_n()
                        });
                        egrid[cell].region_root[xc4000::rslots::LONG_V] = root_v;
                        egrid[cell].region_root[xc4000::rslots::DEC_V] = root_v;
                        egrid[cell].region_root[xc4000::rslots::GCLK] =
                            cell.with_row(self.row_mid());
                    }
                }

                for cell in egrid.die_cells(die) {
                    if cell.col != self.col_w() {
                        if cell.col == self.col_w() + 1
                            && cell.row != self.row_s()
                            && cell.row != self.row_n()
                        {
                            egrid.fill_conn_pair_id(
                                cell.delta(-1, 0),
                                cell,
                                xc4000::xc4000::ccls::PASS_E,
                                xc4000::xc4000::ccls::PASS_CLB_W_W,
                            );
                        } else {
                            egrid.fill_conn_pair_id(
                                cell.delta(-1, 0),
                                cell,
                                xc4000::xc4000::ccls::PASS_E,
                                xc4000::xc4000::ccls::PASS_W,
                            );
                        }
                    }
                    if cell.row != self.row_s() {
                        if cell.row == self.row_n()
                            && cell.col != self.col_w()
                            && cell.col != self.col_e()
                        {
                            egrid.fill_conn_pair_id(
                                cell.delta(0, -1),
                                cell,
                                xc4000::xc4000::ccls::PASS_CLB_N_N,
                                xc4000::xc4000::ccls::PASS_S,
                            );
                        } else {
                            egrid.fill_conn_pair_id(
                                cell.delta(0, -1),
                                cell,
                                xc4000::xc4000::ccls::PASS_N,
                                xc4000::xc4000::ccls::PASS_S,
                            );
                        }
                    }
                }

                egrid.fill_conn_term_id(
                    CellCoord::new(die, self.col_w(), self.row_s()),
                    xc4000::xc4000::ccls::CNR_SW,
                );
                egrid.fill_conn_term_id(
                    CellCoord::new(die, self.col_e(), self.row_s()),
                    xc4000::xc4000::ccls::CNR_SE,
                );
                egrid.fill_conn_term_id(
                    CellCoord::new(die, self.col_w(), self.row_n()),
                    xc4000::xc4000::ccls::CNR_NW,
                );
                egrid.fill_conn_term_id(
                    CellCoord::new(die, self.col_e(), self.row_n()),
                    xc4000::xc4000::ccls::CNR_NE,
                );

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
                            egrid.add_tile_single_id(cell, xc5200::tcls::CNR_SW);
                        } else if cell.row == self.row_n() {
                            egrid.add_tile_single_id(cell, xc5200::tcls::CNR_NW);
                        } else {
                            egrid.add_tile_single_id(cell, xc5200::tcls::IO_W);
                        }
                    } else if cell.col == self.col_e() {
                        if cell.row == self.row_s() {
                            egrid.add_tile_single_id(cell, xc5200::tcls::CNR_SE);
                        } else if cell.row == self.row_n() {
                            egrid.add_tile_single_id(cell, xc5200::tcls::CNR_NE);
                        } else {
                            egrid.add_tile_single_id(cell, xc5200::tcls::IO_E);
                        }
                    } else {
                        if cell.row == self.row_s() {
                            egrid.add_tile_single_id(cell, xc5200::tcls::IO_S);
                        } else if cell.row == self.row_n() {
                            egrid.add_tile_single_id(cell, xc5200::tcls::IO_N);
                        } else {
                            egrid.add_tile_single_id(cell, xc5200::tcls::CLB);
                        }
                    }
                    if cell.col != self.col_w() {
                        egrid.fill_conn_pair_id(
                            cell.delta(-1, 0),
                            cell,
                            xc5200::ccls::PASS_E,
                            xc5200::ccls::PASS_W,
                        );
                    }
                    if cell.row != self.row_s() {
                        egrid.fill_conn_pair_id(
                            cell.delta(0, -1),
                            cell,
                            xc5200::ccls::PASS_N,
                            xc5200::ccls::PASS_S,
                        );
                    }
                }

                for cell in egrid.row(die, self.row_mid()) {
                    let tcid = if cell.col == self.col_w() {
                        xc5200::tcls::LLV_W
                    } else if cell.col == self.col_e() {
                        xc5200::tcls::LLV_E
                    } else {
                        xc5200::tcls::LLV
                    };
                    egrid.add_tile_id(cell, tcid, &[cell.delta(0, -1), cell]);
                }

                for cell in egrid.column(die, self.col_mid()) {
                    let kind = if cell.row == self.row_s() {
                        xc5200::tcls::LLH_S
                    } else if cell.row == self.row_n() {
                        xc5200::tcls::LLH_N
                    } else {
                        xc5200::tcls::LLH
                    };
                    egrid.add_tile_id(cell, kind, &[cell.delta(-1, 0), cell]);
                }

                for cell in egrid.die_cells(die) {
                    egrid[cell].region_root[xc5200::rslots::GCLK_H] = cell.with_col(self.col_w());
                    egrid[cell].region_root[xc5200::rslots::LONG_H] =
                        cell.with_col(if cell.col < self.col_mid() {
                            self.col_w()
                        } else {
                            self.col_e()
                        });
                    egrid[cell].region_root[xc5200::rslots::GCLK_V] = cell.with_row(self.row_s());
                    egrid[cell].region_root[xc5200::rslots::LONG_V] =
                        cell.with_row(if cell.row < self.row_mid() {
                            self.row_s()
                        } else {
                            self.row_n()
                        });
                }

                egrid.fill_conn_term_id(
                    CellCoord::new(die, self.col_w(), self.row_s()),
                    xc5200::ccls::CNR_SW,
                );
                egrid.fill_conn_term_id(
                    CellCoord::new(die, self.col_e(), self.row_s()),
                    xc5200::ccls::CNR_SE,
                );
                egrid.fill_conn_term_id(
                    CellCoord::new(die, self.col_w(), self.row_n()),
                    xc5200::ccls::CNR_NW,
                );
                egrid.fill_conn_term_id(
                    CellCoord::new(die, self.col_e(), self.row_n()),
                    xc5200::ccls::CNR_NE,
                );

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

        let egrid = egrid.finish();

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
