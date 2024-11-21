use prjcombine_int::{
    db::IntDb,
    grid::{ColId, DieId, ExpandedGrid, RowId},
};
use prjcombine_virtex_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};
use unnamed_entity::{EntityId, EntityVec};

use crate::{
    expanded::ExpandedDevice,
    grid::{Grid, GridKind},
};

impl Grid {
    fn get_bio_node(&self, col: ColId) -> &'static str {
        if col == self.col_lio() + 1 {
            "IO.BS.L"
        } else if col == self.col_rio() - 1 {
            "IO.B.R"
        } else if col.to_idx() % 2 == 0 {
            "IO.B"
        } else {
            "IO.BS"
        }
    }

    fn get_tio_node(&self, col: ColId) -> &'static str {
        if col == self.col_lio() + 1 {
            "IO.TS.L"
        } else if col == self.col_rio() - 1 {
            "IO.T.R"
        } else if col.to_idx() % 2 == 0 {
            "IO.T"
        } else {
            "IO.TS"
        }
    }

    fn get_lio_node(&self, row: RowId) -> &'static str {
        if row == self.row_bio() + 1 {
            "IO.LS.B"
        } else if row == self.row_tio() - 1 {
            "IO.L.T"
        } else if self.kind.is_xl() && row == self.row_qb() {
            if row.to_idx() % 2 == 0 {
                "IO.L.FB"
            } else {
                "IO.LS.FB"
            }
        } else if self.kind.is_xl() && row == self.row_qt() - 1 {
            if row.to_idx() % 2 == 0 {
                "IO.L.FT"
            } else {
                "IO.LS.FT"
            }
        } else if row.to_idx() % 2 == 0 {
            "IO.L"
        } else {
            "IO.LS"
        }
    }

    fn get_rio_node(&self, row: RowId) -> &'static str {
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
        if row == self.row_bio() + 1 {
            "IO.RS.B"
        } else if row == self.row_tio() - 1 {
            "IO.R.T"
        } else if self.kind.is_xl() && row == row_f {
            if row.to_idx() % 2 == 0 {
                "IO.R.FB"
            } else {
                "IO.RS.FB"
            }
        } else if self.kind.is_xl() && row == row_f1 {
            if row.to_idx() % 2 == 0 {
                "IO.R.FT"
            } else {
                "IO.RS.FT"
            }
        } else if row.to_idx() % 2 == 0 {
            "IO.R"
        } else {
            "IO.RS"
        }
    }

    pub fn expand_grid<'a>(&'a self, db: &'a IntDb) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        let (_, mut grid) = egrid.add_die(self.columns, self.rows);

        let col_l = grid.cols().next().unwrap();
        let col_r = grid.cols().next_back().unwrap();
        let row_b = grid.rows().next().unwrap();
        let row_t = grid.rows().next_back().unwrap();

        for col in grid.cols() {
            if col == self.col_lio() {
                for row in grid.rows() {
                    if row == self.row_bio() {
                        grid.add_xnode((col, row), "CNR.BL", &[(col, row), (col + 1, row)]);
                    } else if row == self.row_tio() {
                        grid.add_xnode(
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
                        grid.add_xnode(
                            (col, row),
                            self.get_lio_node(row),
                            &[(col, row), (col, row - 1), (col + 1, row), (col, row + 1)],
                        );
                    }
                }
            } else if col == self.col_rio() {
                for row in grid.rows() {
                    if row == self.row_bio() {
                        grid.fill_tile((col, row), "CNR.BR");
                    } else if row == self.row_tio() {
                        grid.add_xnode((col, row), "CNR.TR", &[(col, row), (col, row - 1)]);
                    } else {
                        grid.add_xnode(
                            (col, row),
                            self.get_rio_node(row),
                            &[(col, row), (col, row - 1), (col, row + 1)],
                        );
                    }
                }
            } else {
                for row in grid.rows() {
                    if row == self.row_bio() {
                        grid.add_xnode(
                            (col, row),
                            self.get_bio_node(col),
                            &[(col, row), (col, row + 1), (col + 1, row), (col - 1, row)],
                        );
                    } else if row == self.row_tio() {
                        grid.add_xnode(
                            (col, row),
                            self.get_tio_node(col),
                            &[(col, row), (col + 1, row), (col - 1, row)],
                        );
                    } else {
                        let kind = if row == self.row_bio() + 1 {
                            if col == self.col_lio() + 1 {
                                "CLB.LB"
                            } else if col == self.col_rio() - 1 {
                                "CLB.RB"
                            } else {
                                "CLB.B"
                            }
                        } else if row == self.row_tio() - 1 {
                            if col == self.col_lio() + 1 {
                                "CLB.LT"
                            } else if col == self.col_rio() - 1 {
                                "CLB.RT"
                            } else {
                                "CLB.T"
                            }
                        } else {
                            if col == self.col_lio() + 1 {
                                "CLB.L"
                            } else if col == self.col_rio() - 1 {
                                "CLB.R"
                            } else {
                                "CLB"
                            }
                        };
                        grid.add_xnode(
                            (col, row),
                            kind,
                            &[(col, row), (col, row + 1), (col + 1, row)],
                        );
                    }
                }
            }
        }

        if self.kind.is_xl() {
            for row in grid.rows() {
                for col in [self.col_ql(), self.col_qr()] {
                    if row == self.row_bio() || row == self.row_tio() {
                        grid.fill_term_pair((col - 1, row), (col, row), "LLHQ.IO.E", "LLHQ.IO.W");
                    } else {
                        grid.fill_term_pair((col - 1, row), (col, row), "LLHQ.E", "LLHQ.W");
                    }
                    let kind = if row == self.row_bio() {
                        "LLHQ.IO.B"
                    } else if row == self.row_tio() {
                        "LLHQ.IO.T"
                    } else {
                        if row == self.row_bio() + 1 {
                            "LLHQ.CLB.B"
                        } else if row == self.row_tio() - 1 {
                            "LLHQ.CLB.T"
                        } else {
                            "LLHQ.CLB"
                        }
                    };
                    grid.add_xnode((col, row), kind, &[(col - 1, row), (col, row)]);
                }
                let col = self.col_mid();
                grid.fill_term_pair((col - 1, row), (col, row), "LLHC.E", "LLHC.W");
                let kind = if row == self.row_bio() {
                    "LLHC.IO.B"
                } else if row == self.row_tio() {
                    "LLHC.IO.T"
                } else if row == self.row_bio() + 1 {
                    "LLHC.CLB.B"
                } else {
                    "LLHC.CLB"
                };
                grid.add_xnode((col, row), kind, &[(col - 1, row), (col, row)]);
            }

            for col in grid.cols() {
                for (bt, row) in [('B', self.row_qb()), ('T', self.row_qt())] {
                    grid.fill_term_pair((col, row - 1), (col, row), "LLVQ.N", "LLVQ.S");
                    let kind = if col == self.col_lio() {
                        if bt == 'B' {
                            "LLVQ.IO.L.B"
                        } else {
                            "LLVQ.IO.L.T"
                        }
                    } else if col == self.col_rio() {
                        if bt == 'B' {
                            "LLVQ.IO.R.B"
                        } else {
                            "LLVQ.IO.R.T"
                        }
                    } else {
                        "LLVQ.CLB"
                    };
                    grid.add_xnode((col, row), kind, &[(col, row - 1), (col, row)]);
                }
                let row = self.row_mid();
                grid.fill_term_pair((col, row - 1), (col, row), "LLVC.N", "LLVC.S");
                let kind = if col == self.col_lio() {
                    "LLVC.IO.L"
                } else if col == self.col_rio() {
                    "LLVC.IO.R"
                } else {
                    "LLVC.CLB"
                };
                grid.add_xnode((col, row), kind, &[(col, row - 1), (col, row)]);
            }

            if self.kind == GridKind::Xc4000Xv {
                for row in [self.row_qb(), self.row_qt()] {
                    for col in [self.col_ql(), self.col_qr()] {
                        grid.add_xnode((col, row), "CLKQ", &[(col - 1, row), (col, row)]);
                    }
                }
            } else {
                grid.add_xnode((self.col_mid(), self.row_mid()), "CLKC", &[]);
                grid.add_xnode(
                    (self.col_mid(), self.row_qb()),
                    "CLKQC",
                    &[(self.col_mid(), self.row_qb())],
                );
                grid.add_xnode(
                    (self.col_mid(), self.row_qt()),
                    "CLKQC",
                    &[(self.col_mid(), self.row_qt())],
                );
            }
        } else {
            for row in grid.rows() {
                let col = self.col_mid();
                grid.fill_term_pair((col - 1, row), (col, row), "LLHC.E", "LLHC.W");
                let kind = if row == self.row_bio() {
                    "LLH.IO.B"
                } else if row == self.row_tio() {
                    "LLH.IO.T"
                } else if row == self.row_bio() + 1 {
                    "LLH.CLB.B"
                } else {
                    "LLH.CLB"
                };
                grid.add_xnode((col, row), kind, &[(col - 1, row), (col, row)]);
            }

            for col in grid.cols() {
                let row = self.row_mid();
                grid.fill_term_pair((col, row - 1), (col, row), "LLVC.N", "LLVC.S");
                let kind = if col == self.col_lio() {
                    "LLV.IO.L"
                } else if col == self.col_rio() {
                    "LLV.IO.R"
                } else {
                    "LLV.CLB"
                };
                grid.add_xnode((col, row), kind, &[(col, row - 1), (col, row)]);
            }
        }

        for col in grid.cols() {
            if col != self.col_lio() && col != self.col_rio() {
                grid.fill_term_pair(
                    (col, self.row_tio() - 1),
                    (col, self.row_tio()),
                    "TCLB.N",
                    "MAIN.S",
                );
            }
        }

        for row in grid.rows() {
            if row != self.row_bio() && row != self.row_tio() {
                grid.fill_term_pair(
                    (self.col_lio(), row),
                    (self.col_lio() + 1, row),
                    "MAIN.E",
                    "LCLB.W",
                );
            }
        }

        grid.fill_main_passes();
        grid.fill_term((col_l, row_b), "CNR.LL.W");
        grid.fill_term((col_r, row_b), "CNR.LR.S");
        grid.fill_term((col_l, row_t), "CNR.UL.N");
        grid.fill_term((col_r, row_t), "CNR.UR.E");

        let mut spine_framebit = None;
        let mut qb_framebit = None;
        let mut qt_framebit = None;
        let mut row_framebit = EntityVec::new();
        let mut frame_len = 0;
        for row in grid.rows() {
            if self.kind.is_xl() && row == self.row_qb() {
                qb_framebit = Some(frame_len);
                frame_len += self.btile_height_brk();
            }
            if self.kind.is_xl() && row == self.row_qt() {
                qt_framebit = Some(frame_len);
                frame_len += self.btile_height_brk();
            }
            if row == self.row_mid() {
                if matches!(self.kind, GridKind::Xc4000Ex | GridKind::Xc4000Xla) {
                    // padding
                    frame_len += 2;
                }
                spine_framebit = Some(frame_len);
                frame_len += self.btile_height_clk();
            }
            row_framebit.push(frame_len);
            let height = self.btile_height_main(row);
            frame_len += height;
        }
        let spine_framebit = spine_framebit.unwrap();
        let quarter_framebit = qb_framebit.zip(qt_framebit);

        let mut frame_info = vec![];
        let mut spine_frame = None;
        let mut ql_frame = None;
        let mut qr_frame = None;
        let mut col_frame: EntityVec<_, _> = grid.cols().map(|_| 0).collect();
        for col in grid.cols().rev() {
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
                spine_frame = Some(frame_info.len());
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
            if self.kind.is_xl() && col == self.col_ql() {
                let minor = frame_info.len();
                ql_frame = Some(minor);
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
            if self.kind.is_xl() && col == self.col_qr() {
                let minor = frame_info.len();
                qr_frame = Some(minor);
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
        let quarter_frame = ql_frame.zip(qr_frame);

        let die_bs_geom = DieBitstreamGeom {
            frame_len,
            frame_info,
            bram_frame_len: 0,
            bram_frame_info: vec![],
            iob_frame_len: 0,
        };
        let bs_geom = BitstreamGeom {
            kind: if self.kind == GridKind::SpartanXl && self.columns == 30 {
                DeviceKind::S40Xl
            } else {
                DeviceKind::Xc4000
            },
            die: [die_bs_geom].into_iter().collect(),
            die_order: vec![DieId::from_idx(0)],
        };

        egrid.finish();

        ExpandedDevice {
            grid: self,
            egrid,
            bs_geom,
            spine_frame,
            quarter_frame,
            col_frame,
            spine_framebit,
            quarter_framebit,
            row_framebit,
        }
    }
}
