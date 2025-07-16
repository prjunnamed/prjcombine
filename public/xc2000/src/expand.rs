use prjcombine_interconnect::{
    db::IntDb,
    grid::{ColId, DieId, ExpandedGrid, RowId},
};
use prjcombine_xilinx_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::{
    chip::{Chip, ChipKind},
    expanded::{ExpandedDevice, REGION_GLOBAL},
};

impl Chip {
    fn get_bio_node(&self, col: ColId) -> &'static str {
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

    fn get_tio_node(&self, col: ColId) -> &'static str {
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

    fn get_lio_node(&self, row: RowId) -> &'static str {
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

    fn get_rio_node(&self, row: RowId) -> &'static str {
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
        let (_, mut die) = egrid.add_die(self.columns, self.rows);

        let mut row_framebit = EntityVec::new();
        let mut llv_framebit = EntityPartVec::new();
        let mut frame_len = 0;
        let mut frame_info = vec![];
        let mut col_frame: EntityVec<_, _> = die.cols().map(|_| 0).collect();
        let mut llh_frame = EntityPartVec::new();

        match self.kind {
            ChipKind::Xc2000 => {
                for col in die.cols() {
                    if col == self.col_w() {
                        for row in die.rows() {
                            if row == self.row_s() {
                                die.add_tile((col, row), "CLB.BL", &[(col, row), (col + 1, row)]);
                            } else if row == self.row_n() {
                                die.add_tile(
                                    (col, row),
                                    "CLB.TL",
                                    &[(col, row), (col, row - 1), (col + 1, row)],
                                );
                            } else if row == self.row_mid() - 1 {
                                die.add_tile((col, row), "CLB.ML", &[(col, row), (col, row - 1)]);
                            } else {
                                die.add_tile((col, row), "CLB.L", &[(col, row), (col, row - 1)]);
                            }
                        }
                    } else if col == self.col_e() {
                        for row in die.rows() {
                            if row == self.row_s() {
                                die.add_tile((col, row), "CLB.BR", &[(col, row)]);
                            } else if row == self.row_n() {
                                die.add_tile((col, row), "CLB.TR", &[(col, row), (col, row - 1)]);
                            } else if row == self.row_mid() - 1 {
                                die.add_tile((col, row), "CLB.MR", &[(col, row), (col, row - 1)]);
                            } else {
                                die.add_tile((col, row), "CLB.R", &[(col, row), (col, row - 1)]);
                            }
                        }
                    } else {
                        for row in die.rows() {
                            if row == self.row_s() {
                                let kind = if col == self.col_e() - 1 {
                                    "CLB.BR1"
                                } else {
                                    "CLB.B"
                                };
                                die.add_tile((col, row), kind, &[(col, row), (col + 1, row)]);
                            } else if row == self.row_n() {
                                let kind = if col == self.col_e() - 1 {
                                    "CLB.TR1"
                                } else {
                                    "CLB.T"
                                };
                                die.add_tile((col, row), kind, &[(col, row), (col + 1, row)]);
                            } else {
                                die.add_tile((col, row), "CLB", &[(col, row)]);
                            }
                        }
                    }
                }
                for row in die.rows() {
                    for &col in &self.cols_bidi {
                        die.add_tile((col, row), "BIDIH", &[]);
                    }
                }
                for col in die.cols() {
                    for &row in &self.rows_bidi {
                        die.add_tile((col, row), "BIDIV", &[]);
                    }
                }
                for col in die.cols() {
                    for row in die.rows() {
                        die[(col, row)].region_root[REGION_GLOBAL] =
                            (ColId::from_idx(0), RowId::from_idx(0));
                    }
                }
                die.fill_main_passes();

                for row in die.rows() {
                    if self.rows_bidi.contains(&row) {
                        llv_framebit.insert(row, frame_len);
                        frame_len += 1;
                    }
                    row_framebit.push(frame_len);
                    frame_len += self.btile_height_main(row);
                }

                for col in die.cols().rev() {
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

                for col in die.cols() {
                    for row in die.rows() {
                        let mut subkind =
                            (row.to_idx() + 2 * (self.columns - 1 - col.to_idx())) % 3;
                        if subkind == 1 && col == self.col_e() && row == self.row_n() - 1 {
                            // fuck me with the rustiest fork you can find
                            subkind = 3;
                        }
                        if col == self.col_w() {
                            if row == self.row_s() {
                                die.add_tile(
                                    (col, row),
                                    &format!("CLB.BL{s}.{subkind}"),
                                    &[(col, row), (col + 1, row), (col, row + 1)],
                                );
                            } else if row == self.row_n() {
                                die.add_tile(
                                    (col, row),
                                    &format!("CLB.TL{s}.{subkind}"),
                                    &[(col, row), (col + 1, row), (col, row - 1)],
                                );
                            } else {
                                die.add_tile(
                                    (col, row),
                                    &format!("CLB.L.{subkind}"),
                                    &[(col, row), (col + 1, row), (col, row - 1), (col, row + 1)],
                                );
                            }
                        } else if col == self.col_e() {
                            if row == self.row_s() {
                                die.add_tile(
                                    (col, row),
                                    &format!("CLB.BR{s}.{subkind}"),
                                    &[(col, row), (col, row + 1)],
                                );
                            } else if row == self.row_n() {
                                die.add_tile(
                                    (col, row),
                                    &format!("CLB.TR{s}.{subkind}"),
                                    &[(col, row), (col, row - 1)],
                                );
                            } else {
                                die.add_tile(
                                    (col, row),
                                    &format!("CLB.R.{subkind}"),
                                    &[(col, row), (col, row - 1), (col, row + 1)],
                                );
                            }
                        } else {
                            if row == self.row_s() {
                                die.add_tile(
                                    (col, row),
                                    &format!("CLB.B.{subkind}"),
                                    &[(col, row), (col + 1, row), (col, row + 1)],
                                );
                            } else if row == self.row_n() {
                                die.add_tile(
                                    (col, row),
                                    &format!("CLB.T{s}.{subkind}"),
                                    &[(col, row), (col + 1, row), (col, row - 1)],
                                );
                            } else {
                                die.add_tile(
                                    (col, row),
                                    &format!("CLB.{subkind}"),
                                    &[(col, row), (col + 1, row), (col, row - 1), (col, row + 1)],
                                );
                            }
                        }
                    }
                }
                {
                    let col = self.col_mid();
                    let row = self.row_s();
                    die.fill_conn_pair((col - 1, row), (col, row), "LLH.E", "LLH.W");
                    die.add_tile((col, row), "LLH.B", &[(col - 1, row), (col, row)]);
                    let row = self.row_n();
                    die.fill_conn_pair((col - 1, row), (col, row), "LLH.E", "LLH.W");
                    die.add_tile((col, row), "LLH.T", &[(col - 1, row), (col, row)]);
                }
                if self.is_small {
                    let row = self.row_mid();
                    let col = self.col_w();
                    die.fill_conn_pair((col, row - 1), (col, row), "LLV.S.N", "LLV.S.S");
                    die.add_tile((col, row), "LLV.LS", &[(col, row - 1), (col, row)]);
                    let col = self.col_e();
                    die.fill_conn_pair((col, row - 1), (col, row), "LLV.S.N", "LLV.S.S");
                    die.add_tile((col, row), "LLV.RS", &[(col, row - 1), (col, row)]);
                } else {
                    let row = self.row_mid();
                    for col in die.cols() {
                        let kind = if col == self.col_w() {
                            "LLV.L"
                        } else if col == self.col_e() {
                            "LLV.R"
                        } else {
                            "LLV"
                        };
                        die.fill_conn_pair((col, row - 1), (col, row), "LLV.N", "LLV.S");
                        die.add_tile((col, row), kind, &[(col, row - 1), (col, row)]);
                    }
                }
                for col in die.cols() {
                    for row in die.rows() {
                        die[(col, row)].region_root[REGION_GLOBAL] =
                            (ColId::from_idx(0), RowId::from_idx(0));
                    }
                }
                die.fill_main_passes();

                for row in die.rows() {
                    if row == self.row_mid() && !self.is_small {
                        llv_framebit.insert(row, frame_len);
                        frame_len += 1;
                    }
                    row_framebit.push(frame_len);
                    frame_len += self.btile_height_main(row);
                }

                for col in die.cols().rev() {
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
                let col_l = die.cols().next().unwrap();
                let col_r = die.cols().next_back().unwrap();
                let row_b = die.rows().next().unwrap();
                let row_t = die.rows().next_back().unwrap();

                for col in die.cols() {
                    if col == self.col_w() {
                        for row in die.rows() {
                            if row == self.row_s() {
                                die.add_tile((col, row), "CNR.BL", &[(col, row), (col + 1, row)]);
                            } else if row == self.row_n() {
                                die.add_tile(
                                    (col, row),
                                    "CNR.TL",
                                    &[
                                        (col, row),
                                        (col + 1, row),
                                        (col, row - 1),
                                        (col + 1, row - 1),
                                    ],
                                );
                            } else {
                                die.add_tile(
                                    (col, row),
                                    self.get_lio_node(row),
                                    &[(col, row), (col, row - 1), (col + 1, row), (col, row + 1)],
                                );
                            }
                        }
                    } else if col == self.col_e() {
                        for row in die.rows() {
                            if row == self.row_s() {
                                die.fill_tile((col, row), "CNR.BR");
                            } else if row == self.row_n() {
                                die.add_tile((col, row), "CNR.TR", &[(col, row), (col, row - 1)]);
                            } else {
                                die.add_tile(
                                    (col, row),
                                    self.get_rio_node(row),
                                    &[(col, row), (col, row - 1), (col, row + 1)],
                                );
                            }
                        }
                    } else {
                        for row in die.rows() {
                            if row == self.row_s() {
                                die.add_tile(
                                    (col, row),
                                    self.get_bio_node(col),
                                    &[(col, row), (col, row + 1), (col + 1, row), (col - 1, row)],
                                );
                            } else if row == self.row_n() {
                                die.add_tile(
                                    (col, row),
                                    self.get_tio_node(col),
                                    &[(col, row), (col + 1, row), (col - 1, row)],
                                );
                            } else {
                                let kind = if row == self.row_s() + 1 {
                                    if col == self.col_w() + 1 {
                                        "CLB.LB"
                                    } else if col == self.col_e() - 1 {
                                        "CLB.RB"
                                    } else {
                                        "CLB.B"
                                    }
                                } else if row == self.row_n() - 1 {
                                    if col == self.col_w() + 1 {
                                        "CLB.LT"
                                    } else if col == self.col_e() - 1 {
                                        "CLB.RT"
                                    } else {
                                        "CLB.T"
                                    }
                                } else {
                                    if col == self.col_w() + 1 {
                                        "CLB.L"
                                    } else if col == self.col_e() - 1 {
                                        "CLB.R"
                                    } else {
                                        "CLB"
                                    }
                                };
                                die.add_tile(
                                    (col, row),
                                    kind,
                                    &[(col, row), (col, row + 1), (col + 1, row)],
                                );
                            }
                        }
                    }
                }

                if self.kind.is_xl() {
                    for row in die.rows() {
                        for col in [self.col_ql(), self.col_qr()] {
                            if row == self.row_s() || row == self.row_n() {
                                die.fill_conn_pair(
                                    (col - 1, row),
                                    (col, row),
                                    "LLHQ.IO.E",
                                    "LLHQ.IO.W",
                                );
                            } else {
                                die.fill_conn_pair((col - 1, row), (col, row), "LLHQ.E", "LLHQ.W");
                            }
                            let kind = if row == self.row_s() {
                                "LLHQ.IO.B"
                            } else if row == self.row_n() {
                                "LLHQ.IO.T"
                            } else {
                                if row == self.row_s() + 1 {
                                    "LLHQ.CLB.B"
                                } else if row == self.row_n() - 1 {
                                    "LLHQ.CLB.T"
                                } else {
                                    "LLHQ.CLB"
                                }
                            };
                            die.add_tile((col, row), kind, &[(col - 1, row), (col, row)]);
                        }
                        let col = self.col_mid();
                        die.fill_conn_pair((col - 1, row), (col, row), "LLHC.E", "LLHC.W");
                        let kind = if row == self.row_s() {
                            "LLHC.IO.B"
                        } else if row == self.row_n() {
                            "LLHC.IO.T"
                        } else if row == self.row_s() + 1 {
                            "LLHC.CLB.B"
                        } else {
                            "LLHC.CLB"
                        };
                        die.add_tile((col, row), kind, &[(col - 1, row), (col, row)]);
                    }

                    for col in die.cols() {
                        for (bt, row) in [('B', self.row_qb()), ('T', self.row_qt())] {
                            die.fill_conn_pair((col, row - 1), (col, row), "LLVQ.N", "LLVQ.S");
                            let kind = if col == self.col_w() {
                                if bt == 'B' {
                                    "LLVQ.IO.L.B"
                                } else {
                                    "LLVQ.IO.L.T"
                                }
                            } else if col == self.col_e() {
                                if bt == 'B' {
                                    "LLVQ.IO.R.B"
                                } else {
                                    "LLVQ.IO.R.T"
                                }
                            } else {
                                "LLVQ.CLB"
                            };
                            die.add_tile((col, row), kind, &[(col, row - 1), (col, row)]);
                        }
                        let row = self.row_mid();
                        die.fill_conn_pair((col, row - 1), (col, row), "LLVC.N", "LLVC.S");
                        let kind = if col == self.col_w() {
                            "LLVC.IO.L"
                        } else if col == self.col_e() {
                            "LLVC.IO.R"
                        } else {
                            "LLVC.CLB"
                        };
                        die.add_tile((col, row), kind, &[(col, row - 1), (col, row)]);
                    }

                    if self.kind == ChipKind::Xc4000Xv {
                        for row in [self.row_qb(), self.row_qt()] {
                            for col in [self.col_ql(), self.col_qr()] {
                                die.add_tile((col, row), "CLKQ", &[(col - 1, row), (col, row)]);
                            }
                        }
                    } else {
                        die.add_tile((self.col_mid(), self.row_mid()), "CLKC", &[]);
                        die.add_tile(
                            (self.col_mid(), self.row_qb()),
                            "CLKQC",
                            &[(self.col_mid(), self.row_qb())],
                        );
                        die.add_tile(
                            (self.col_mid(), self.row_qt()),
                            "CLKQC",
                            &[(self.col_mid(), self.row_qt())],
                        );
                    }
                } else {
                    for row in die.rows() {
                        let col = self.col_mid();
                        die.fill_conn_pair((col - 1, row), (col, row), "LLHC.E", "LLHC.W");
                        let kind = if row == self.row_s() {
                            "LLH.IO.B"
                        } else if row == self.row_n() {
                            "LLH.IO.T"
                        } else if row == self.row_s() + 1 {
                            "LLH.CLB.B"
                        } else {
                            "LLH.CLB"
                        };
                        die.add_tile((col, row), kind, &[(col - 1, row), (col, row)]);
                    }

                    for col in die.cols() {
                        let row = self.row_mid();
                        die.fill_conn_pair((col, row - 1), (col, row), "LLVC.N", "LLVC.S");
                        let kind = if col == self.col_w() {
                            "LLV.IO.L"
                        } else if col == self.col_e() {
                            "LLV.IO.R"
                        } else {
                            "LLV.CLB"
                        };
                        die.add_tile((col, row), kind, &[(col, row - 1), (col, row)]);
                    }
                }

                for col in die.cols() {
                    if col != self.col_w() && col != self.col_e() {
                        die.fill_conn_pair(
                            (col, self.row_n() - 1),
                            (col, self.row_n()),
                            "TCLB.N",
                            "MAIN.S",
                        );
                    }
                }

                for row in die.rows() {
                    if row != self.row_s() && row != self.row_n() {
                        die.fill_conn_pair(
                            (self.col_w(), row),
                            (self.col_w() + 1, row),
                            "MAIN.E",
                            "LCLB.W",
                        );
                    }
                }

                die.fill_main_passes();
                die.fill_conn_term((col_l, row_b), "CNR.LL.W");
                die.fill_conn_term((col_r, row_b), "CNR.LR.S");
                die.fill_conn_term((col_l, row_t), "CNR.UL.N");
                die.fill_conn_term((col_r, row_t), "CNR.UR.E");

                for row in die.rows() {
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

                for col in die.cols().rev() {
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
                let col_l = die.cols().next().unwrap();
                let col_r = die.cols().next_back().unwrap();
                let row_b = die.rows().next().unwrap();
                let row_t = die.rows().next_back().unwrap();
                for col in die.cols() {
                    if col == col_l {
                        for row in die.rows() {
                            if row == row_b {
                                die.fill_tile((col, row), "CNR.BL");
                            } else if row == row_t {
                                die.fill_tile((col, row), "CNR.TL");
                            } else {
                                die.fill_tile((col, row), "IO.L");
                            }
                        }
                    } else if col == col_r {
                        for row in die.rows() {
                            if row == row_b {
                                die.fill_tile((col, row), "CNR.BR");
                            } else if row == row_t {
                                die.fill_tile((col, row), "CNR.TR");
                            } else {
                                die.fill_tile((col, row), "IO.R");
                            }
                        }
                    } else {
                        for row in die.rows() {
                            if row == row_b {
                                die.fill_tile((col, row), "IO.B");
                            } else if row == row_t {
                                die.fill_tile((col, row), "IO.T");
                            } else {
                                die.fill_tile((col, row), "CLB");
                            }
                        }
                    }
                }

                for col in die.cols() {
                    let kind = if col == col_l {
                        "CLKL"
                    } else if col == col_r {
                        "CLKR"
                    } else {
                        "CLKH"
                    };
                    let row_s = self.row_mid() - 1;
                    let row_n = self.row_mid();
                    die.fill_conn_pair((col, row_s), (col, row_n), "LLV.N", "LLV.S");
                    die.add_tile((col, row_n), kind, &[(col, row_s), (col, row_n)]);
                }

                for row in die.rows() {
                    let kind = if row == row_b {
                        "CLKB"
                    } else if row == row_t {
                        "CLKT"
                    } else {
                        "CLKV"
                    };
                    let col_l = self.col_mid() - 1;
                    let col_r = self.col_mid();
                    die.fill_conn_pair((col_l, row), (col_r, row), "LLH.E", "LLH.W");
                    die.add_tile((col_r, row), kind, &[(col_l, row), (col_r, row)]);
                }

                die.fill_main_passes();
                die.fill_conn_term((col_l, row_b), "CNR.LL");
                die.fill_conn_term((col_r, row_b), "CNR.LR");
                die.fill_conn_term((col_l, row_t), "CNR.UL");
                die.fill_conn_term((col_r, row_t), "CNR.UR");

                for row in die.rows() {
                    if row == self.row_mid() {
                        llv_framebit.insert(row, frame_len);
                        frame_len += 4;
                    }
                    row_framebit.push(frame_len);
                    let height = self.btile_height_main(row);
                    frame_len += height;
                }

                for col in die.cols().rev() {
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
