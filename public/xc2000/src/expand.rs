use prjcombine_interconnect::{
    db::IntDb,
    grid::{CellCoord, ColId, DieId, ExpandedGrid, RowId},
};
use prjcombine_xilinx_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::{
    chip::{Chip, ChipKind},
    expanded::ExpandedDevice,
    regions,
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
        } else if self.kind.is_xl() && row == self.row_qb() {
            if row.to_idx().is_multiple_of(2) {
                "IO.L.FB"
            } else {
                "IO.LS.FB"
            }
        } else if self.kind.is_xl() && row == self.row_qt() - 1 {
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
            self.row_qb() + 1
        } else {
            self.row_qb()
        };
        let row_f1 = if self.is_buff_large {
            self.row_qt() - 2
        } else {
            self.row_qt() - 1
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
                            egrid.add_tile(cell, "CLB.BL", &[cell, cell.delta(1, 0)]);
                        } else if cell.row == self.row_n() {
                            egrid.add_tile(
                                cell,
                                "CLB.TL",
                                &[cell, cell.delta(0, -1), cell.delta(1, 0)],
                            );
                        } else if cell.row == self.row_mid() - 1 {
                            egrid.add_tile(cell, "CLB.ML", &[cell, cell.delta(0, -1)]);
                        } else {
                            egrid.add_tile(cell, "CLB.L", &[cell, cell.delta(0, -1)]);
                        }
                    } else if cell.col == self.col_e() {
                        if cell.row == self.row_s() {
                            egrid.add_tile(cell, "CLB.BR", &[cell]);
                        } else if cell.row == self.row_n() {
                            egrid.add_tile(cell, "CLB.TR", &[cell, cell.delta(0, -1)]);
                        } else if cell.row == self.row_mid() - 1 {
                            egrid.add_tile(cell, "CLB.MR", &[cell, cell.delta(0, -1)]);
                        } else {
                            egrid.add_tile(cell, "CLB.R", &[cell, cell.delta(0, -1)]);
                        }
                    } else {
                        if cell.row == self.row_s() {
                            let kind = if cell.col == self.col_e() - 1 {
                                "CLB.BR1"
                            } else {
                                "CLB.B"
                            };
                            egrid.add_tile(cell, kind, &[cell, cell.delta(1, 0)]);
                        } else if cell.row == self.row_n() {
                            let kind = if cell.col == self.col_e() - 1 {
                                "CLB.TR1"
                            } else {
                                "CLB.T"
                            };
                            egrid.add_tile(cell, kind, &[cell, cell.delta(1, 0)]);
                        } else {
                            egrid.add_tile(cell, "CLB", &[cell]);
                        }
                    }
                }
                for &col in &self.cols_bidi {
                    for cell in egrid.column(die, col) {
                        egrid.add_tile(cell, "BIDIH", &[]);
                    }
                }
                for &row in &self.rows_bidi {
                    for cell in egrid.row(die, row) {
                        egrid.add_tile(cell, "BIDIV", &[]);
                    }
                }
                for cell in egrid.die_cells(die) {
                    egrid[cell].region_root[regions::GLOBAL] =
                        CellCoord::new(DieId::from_idx(0), ColId::from_idx(0), RowId::from_idx(0));
                }
                egrid.fill_main_passes(die);

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
                let s = if self.is_small { "S" } else { "" };

                for cell in egrid.die_cells(die) {
                    let mut subkind =
                        (cell.row.to_idx() + 2 * (self.columns - 1 - cell.col.to_idx())) % 3;
                    if subkind == 1 && cell.col == self.col_e() && cell.row == self.row_n() - 1 {
                        // fuck me with the rustiest fork you can find
                        subkind = 3;
                    }
                    if cell.col == self.col_w() {
                        if cell.row == self.row_s() {
                            egrid.add_tile(
                                cell,
                                &format!("CLB.BL{s}.{subkind}"),
                                &[cell, cell.delta(1, 0), cell.delta(0, 1)],
                            );
                        } else if cell.row == self.row_n() {
                            egrid.add_tile(
                                cell,
                                &format!("CLB.TL{s}.{subkind}"),
                                &[cell, cell.delta(1, 0), cell.delta(0, -1)],
                            );
                        } else {
                            egrid.add_tile(
                                cell,
                                &format!("CLB.L.{subkind}"),
                                &[cell, cell.delta(1, 0), cell.delta(0, -1), cell.delta(0, 1)],
                            );
                        }
                    } else if cell.col == self.col_e() {
                        if cell.row == self.row_s() {
                            egrid.add_tile(
                                cell,
                                &format!("CLB.BR{s}.{subkind}"),
                                &[cell, cell.delta(0, 1)],
                            );
                        } else if cell.row == self.row_n() {
                            egrid.add_tile(
                                cell,
                                &format!("CLB.TR{s}.{subkind}"),
                                &[cell, cell.delta(0, -1)],
                            );
                        } else {
                            egrid.add_tile(
                                cell,
                                &format!("CLB.R.{subkind}"),
                                &[cell, cell.delta(0, -1), cell.delta(0, 1)],
                            );
                        }
                    } else {
                        if cell.row == self.row_s() {
                            egrid.add_tile(
                                cell,
                                &format!("CLB.B.{subkind}"),
                                &[cell, cell.delta(1, 0), cell.delta(0, 1)],
                            );
                        } else if cell.row == self.row_n() {
                            egrid.add_tile(
                                cell,
                                &format!("CLB.T{s}.{subkind}"),
                                &[cell, cell.delta(1, 0), cell.delta(0, -1)],
                            );
                        } else {
                            egrid.add_tile(
                                cell,
                                &format!("CLB.{subkind}"),
                                &[cell, cell.delta(1, 0), cell.delta(0, -1), cell.delta(0, 1)],
                            );
                        }
                    }
                }
                {
                    let cell = CellCoord::new(die, self.col_mid(), self.row_s());
                    egrid.fill_conn_pair(cell.delta(-1, 0), cell, "LLH.E", "LLH.W");
                    egrid.add_tile(cell, "LLH.B", &[cell.delta(-1, 0), cell]);
                    let cell = CellCoord::new(die, self.col_mid(), self.row_n());
                    egrid.fill_conn_pair(cell.delta(-1, 0), cell, "LLH.E", "LLH.W");
                    egrid.add_tile(cell, "LLH.T", &[cell.delta(-1, 0), cell]);
                }
                if self.is_small {
                    let cell = CellCoord::new(die, self.col_w(), self.row_mid());
                    egrid.fill_conn_pair(cell.delta(0, -1), cell, "LLV.S.N", "LLV.S.S");
                    egrid.add_tile(cell, "LLV.LS", &[cell.delta(0, -1), cell]);
                    let cell = CellCoord::new(die, self.col_e(), self.row_mid());
                    egrid.fill_conn_pair(cell.delta(0, -1), cell, "LLV.S.N", "LLV.S.S");
                    egrid.add_tile(cell, "LLV.RS", &[cell.delta(0, -1), cell]);
                } else {
                    for cell in egrid.row(die, self.row_mid()) {
                        let kind = if cell.col == self.col_w() {
                            "LLV.L"
                        } else if cell.col == self.col_e() {
                            "LLV.R"
                        } else {
                            "LLV"
                        };
                        egrid.fill_conn_pair(cell.delta(0, -1), cell, "LLV.N", "LLV.S");
                        egrid.add_tile(cell, kind, &[cell.delta(0, -1), cell]);
                    }
                }
                for cell in egrid.die_cells(die) {
                    egrid[cell].region_root[regions::GLOBAL] =
                        CellCoord::new(DieId::from_idx(0), ColId::from_idx(0), RowId::from_idx(0));
                }
                egrid.fill_main_passes(die);

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
                    for col in [self.col_ql(), self.col_qr()] {
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

                    for (bt, row) in [('B', self.row_qb()), ('T', self.row_qt())] {
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
                        for row in [self.row_qb(), self.row_qt()] {
                            for col in [self.col_ql(), self.col_qr()] {
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
                            CellCoord::new(die, self.col_mid(), self.row_qb()),
                            "CLKQC",
                        );
                        egrid.add_tile_single(
                            CellCoord::new(die, self.col_mid(), self.row_qt()),
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
                    if self.kind.is_xl() && (row == self.row_qb() || row == self.row_qt()) {
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
                    if self.kind.is_xl() && (col == self.col_ql() || col == self.col_qr()) {
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
