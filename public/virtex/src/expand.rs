use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::grid::{
    CellCoord, ColId, ExpandedDieRefMut, ExpandedGrid, RowId, WireCoord,
};
use prjcombine_xilinx_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};
use std::collections::{BTreeSet, HashSet};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::chip::{Chip, ChipKind, DisabledPart};
use crate::expanded::{ExpandedDevice, REGION_GLOBAL, REGION_LEAF};

struct Expander<'a, 'b> {
    chip: &'b Chip,
    db: &'a IntDb,
    disabled: &'b BTreeSet<DisabledPart>,
    die: ExpandedDieRefMut<'a, 'b>,
    cols_bram: Vec<ColId>,
    spine_frame: usize,
    frame_info: Vec<FrameInfo>,
    col_frame: EntityVec<ColId, usize>,
    bram_frame: EntityPartVec<ColId, usize>,
    clkv_frame: EntityPartVec<ColId, usize>,
    blackhole_wires: HashSet<WireCoord>,
}

impl Expander<'_, '_> {
    fn fill_int(&mut self) {
        for col in self.die.cols() {
            if col == self.chip.col_w() {
                for row in self.die.rows() {
                    if row == self.chip.row_s() {
                        self.die.fill_tile((col, row), "CNR.BL");
                    } else if row == self.chip.row_n() {
                        self.die.fill_tile((col, row), "CNR.TL");
                    } else {
                        self.die.fill_tile((col, row), "IO.L");
                    }
                }
            } else if col == self.chip.col_e() {
                for row in self.die.rows() {
                    if row == self.chip.row_s() {
                        self.die.fill_tile((col, row), "CNR.BR");
                    } else if row == self.chip.row_n() {
                        self.die.fill_tile((col, row), "CNR.TR");
                    } else {
                        self.die.fill_tile((col, row), "IO.R");
                    }
                }
            } else if self.chip.cols_bram.contains(&col) {
                // skip for now
            } else {
                for row in self.die.rows() {
                    if row == self.chip.row_s() {
                        self.die.fill_tile((col, row), "IO.B");
                    } else if row == self.chip.row_n() {
                        self.die.fill_tile((col, row), "IO.T");
                    } else {
                        self.die.fill_tile((col, row), "CLB");
                    }
                }
            }
        }
    }

    fn fill_bram(&mut self) {
        for &col in &self.cols_bram {
            if self.disabled.contains(&DisabledPart::Bram(col)) {
                continue;
            }

            let row = self.chip.row_s();
            self.die
                .add_tile((col, row), "BRAM_BOT", &[(col, row), (col - 1, row)]);

            let mut prev_crd = (col, row);
            for row in self.die.rows() {
                if row == self.chip.row_n() || row.to_idx() % 4 != 1 {
                    continue;
                }
                let kind;
                if col == self.chip.col_w() + 1 {
                    kind = "LBRAM";
                } else if col == self.chip.col_e() - 1 {
                    kind = "RBRAM";
                } else {
                    kind = "MBRAM";
                }
                self.die.add_tile(
                    (col, row),
                    kind,
                    &[
                        (col, row),
                        (col, row + 1),
                        (col, row + 2),
                        (col, row + 3),
                        (col - 1, row),
                        (col - 1, row + 1),
                        (col - 1, row + 2),
                        (col - 1, row + 3),
                        (col + 1, row),
                        (col + 1, row + 1),
                        (col + 1, row + 2),
                        (col + 1, row + 3),
                    ],
                );
                self.die
                    .fill_conn_pair(prev_crd, (col, row), "MAIN.N", "MAIN.S");
                prev_crd = (col, row);
            }

            let row = self.chip.row_n();
            self.die
                .add_tile((col, row), "BRAM_TOP", &[(col, row), (col - 1, row)]);
            self.die
                .fill_conn_pair(prev_crd, (col, row), "MAIN.N", "MAIN.S");

            // special hack!
            for (wire, wname, _) in &self.db.wires {
                if wname.starts_with("BRAM.QUAD") {
                    self.blackhole_wires
                        .insert(CellCoord::new(self.die.die, col, self.chip.row_n()).wire(wire));
                }
            }
        }
    }

    fn fill_clkbt(&mut self) {
        let row_b = self.chip.row_s();
        let row_t = self.chip.row_n();
        // CLKB/CLKT and DLLs
        if self.chip.kind == ChipKind::Virtex {
            let col_c = self.chip.col_clk();
            let col_pl = self.chip.col_w() + 1;
            let col_pr = self.chip.col_e() - 1;
            self.die.add_tile(
                (col_c, row_b),
                "CLKB",
                &[(col_c, row_b), (col_pl, row_b), (col_pr, row_b)],
            );
            self.die.add_tile(
                (col_c, row_t),
                "CLKT",
                &[(col_c, row_t), (col_pl, row_t), (col_pr, row_t)],
            );
            self.die.add_tile(
                (col_pl, row_b),
                "DLL.BOT",
                &[(col_pl, row_b), (col_pl - 1, row_b), (col_c, row_b)],
            );
            self.die.add_tile(
                (col_pl, row_t),
                "DLL.TOP",
                &[(col_pl, row_t), (col_pl - 1, row_t), (col_c, row_t)],
            );
            self.die.add_tile(
                (col_pr, row_b),
                "DLL.BOT",
                &[(col_pr, row_b), (col_pr - 1, row_b), (col_c, row_b)],
            );
            self.die.add_tile(
                (col_pr, row_t),
                "DLL.TOP",
                &[(col_pr, row_t), (col_pr - 1, row_t), (col_c, row_t)],
            );
        } else {
            let col_c = self.chip.col_clk();
            let bram_mid = self.cols_bram.len() / 2;
            let c_pl = bram_mid - 1;
            let c_pr = bram_mid;
            let c_sl = bram_mid - 2;
            let c_sr = bram_mid + 1;
            let col_pl = self.cols_bram[c_pl];
            let col_pr = self.cols_bram[c_pr];
            let col_sl = self.cols_bram[c_sl];
            let col_sr = self.cols_bram[c_sr];
            let kind_b;
            let kind_t;
            if self.disabled.contains(&DisabledPart::PrimaryDlls) {
                kind_b = "CLKB_2DLL";
                kind_t = "CLKT_2DLL";
            } else {
                kind_b = "CLKB_4DLL";
                kind_t = "CLKT_4DLL";
            }
            self.die.add_tile(
                (col_c, row_b),
                kind_b,
                &[
                    (col_c, row_b),
                    (col_pl, row_b),
                    (col_pr, row_b),
                    (col_sl, row_b),
                    (col_sr, row_b),
                ],
            );
            self.die.add_tile(
                (col_c, row_t),
                kind_t,
                &[
                    (col_c, row_t),
                    (col_pl, row_t),
                    (col_pr, row_t),
                    (col_sl, row_t),
                    (col_sr, row_t),
                ],
            );
            // DLLS
            self.die.add_tile(
                (col_sl, row_b),
                "DLLS.BOT",
                &[(col_sl, row_b), (col_sl - 1, row_b), (col_c, row_b)],
            );
            self.die.add_tile(
                (col_sl, row_t),
                "DLLS.TOP",
                &[(col_sl, row_t), (col_sl - 1, row_t), (col_c, row_t)],
            );
            self.die.add_tile(
                (col_sr, row_b),
                "DLLS.BOT",
                &[(col_sr, row_b), (col_sr - 1, row_b), (col_c, row_b)],
            );
            self.die.add_tile(
                (col_sr, row_t),
                "DLLS.TOP",
                &[(col_sr, row_t), (col_sr - 1, row_t), (col_c, row_t)],
            );
            if !self.disabled.contains(&DisabledPart::PrimaryDlls) {
                self.die.add_tile(
                    (col_pl, row_b),
                    "DLLP.BOT",
                    &[
                        (col_pl, row_b),
                        (col_pl - 1, row_b),
                        (col_c, row_b),
                        (col_sl, row_b),
                    ],
                );
                self.die.add_tile(
                    (col_pl, row_t),
                    "DLLP.TOP",
                    &[
                        (col_pl, row_t),
                        (col_pl - 1, row_t),
                        (col_c, row_t),
                        (col_sl, row_t),
                    ],
                );
                self.die.add_tile(
                    (col_pr, row_b),
                    "DLLP.BOT",
                    &[
                        (col_pr, row_b),
                        (col_pr - 1, row_b),
                        (col_c, row_b),
                        (col_sr, row_b),
                    ],
                );
                self.die.add_tile(
                    (col_pr, row_t),
                    "DLLP.TOP",
                    &[
                        (col_pr, row_t),
                        (col_pr - 1, row_t),
                        (col_c, row_t),
                        (col_sr, row_t),
                    ],
                );
            }
        }
    }

    fn fill_pcilogic(&mut self) {
        // CLKL/CLKR
        let pci_l = (self.chip.col_w(), self.chip.row_clk());
        let pci_r = (self.chip.col_e(), self.chip.row_clk());
        self.die.add_tile(pci_l, "CLKL", &[pci_l]);
        self.die.add_tile(pci_r, "CLKR", &[pci_r]);
    }

    fn fill_clk(&mut self) {
        for col in self.die.cols() {
            for row in self.die.rows() {
                self.die[(col, row)].region_root[REGION_GLOBAL] =
                    (ColId::from_idx(0), RowId::from_idx(0));
            }
        }
        for &(col_m, col_l, col_r) in &self.chip.cols_clkv {
            for row in self.die.rows() {
                for c in col_l.to_idx()..col_m.to_idx() {
                    let col = ColId::from_idx(c);
                    self.die[(col, row)].region_root[REGION_LEAF] = (col_m - 1, row);
                }
                if col_m == self.chip.col_w() + 1 || col_m == self.chip.col_e() - 1 {
                    if row == self.chip.row_s() {
                        for c in col_m.to_idx()..col_r.to_idx() {
                            let col = ColId::from_idx(c);
                            self.die[(col, row)].region_root[REGION_LEAF] = (col_m, row);
                        }
                        self.die.add_tile(
                            (col_m, row),
                            "CLKV_BRAM_S",
                            &[(col_m, row), (col_m - 1, row), (col_m, row + 1)],
                        );
                    } else if row == self.chip.row_n() {
                        for c in col_m.to_idx()..col_r.to_idx() {
                            let col = ColId::from_idx(c);
                            self.die[(col, row)].region_root[REGION_LEAF] = (col_m, row);
                        }
                        self.die.add_tile(
                            (col_m, row),
                            "CLKV_BRAM_N",
                            &[(col_m, row), (col_m - 1, row), (col_m, row - 4)],
                        );
                    } else {
                        self.die[(col_m, row)].region_root[REGION_LEAF] =
                            (col_m, self.chip.row_clk());
                        for c in (col_m.to_idx() + 1)..col_r.to_idx() {
                            let col = ColId::from_idx(c);
                            self.die[(col, row)].region_root[REGION_LEAF] = (col_m + 1, row);
                        }
                    }
                } else {
                    for c in col_m.to_idx()..col_r.to_idx() {
                        let col = ColId::from_idx(c);
                        self.die[(col, row)].region_root[REGION_LEAF] = (col_m, row);
                    }
                    let kind = if row == self.chip.row_s() || row == self.chip.row_n() {
                        "CLKV.NULL"
                    } else if col_m == self.chip.col_clk() {
                        "CLKV.CLKV"
                    } else {
                        "CLKV.GCLKV"
                    };
                    self.die
                        .add_tile((col_m, row), kind, &[(col_m - 1, row), (col_m, row)]);
                }
            }
            if col_m == self.chip.col_w() + 1 || col_m == self.chip.col_e() - 1 {
                self.die.add_tile(
                    (col_m, self.chip.row_clk()),
                    "BRAM_CLKH",
                    &[(col_m, self.chip.row_clk())],
                );
            } else if col_m == self.chip.col_clk() {
                self.die.add_tile((col_m, self.chip.row_clk()), "CLKC", &[]);
            } else {
                self.die
                    .add_tile((col_m, self.chip.row_clk()), "GCLKC", &[]);
            }
        }
    }

    fn fill_frame_info(&mut self) {
        let mut major = 0;
        // spine
        self.spine_frame = 0;
        for minor in 0..8 {
            self.frame_info.push(FrameInfo {
                addr: FrameAddr {
                    typ: 0,
                    region: 0,
                    major,
                    minor,
                },
                mask_mode: [].into_iter().collect(),
            });
        }
        major += 1;
        self.clkv_frame.insert(self.chip.col_clk(), 7);

        for _ in self.chip.columns() {
            self.col_frame.push(0);
        }

        let split_bram = self.chip.kind != ChipKind::VirtexE;
        let mut clkv_frame = 0;

        for dx in 0..(self.chip.columns / 2) {
            for lr in ['R', 'L'] {
                let col = if lr == 'R' {
                    self.chip.col_clk() + dx
                } else {
                    self.chip.col_clk() - 1 - dx
                };
                if self.chip.cols_bram.contains(&col) && split_bram {
                    continue;
                }
                if col != self.chip.col_clk()
                    && !self.chip.cols_bram.contains(&col)
                    && self
                        .chip
                        .cols_clkv
                        .iter()
                        .any(|&(col_m, _, _)| col_m == col)
                {
                    self.clkv_frame.insert(col, clkv_frame);
                    clkv_frame += 1;
                }
                self.col_frame[col] = self.frame_info.len();
                let width = if col == self.chip.col_w() || col == self.chip.col_e() {
                    54
                } else if self.chip.cols_bram.contains(&col) {
                    27
                } else {
                    48
                };
                for minor in 0..width {
                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 0,
                            region: 0,
                            major,
                            minor,
                        },
                        mask_mode: [].into_iter().collect(),
                    });
                }
                major += 1;
            }
        }

        // bram main
        if split_bram {
            for dx in 0..(self.chip.columns / 2) {
                for lr in ['R', 'L'] {
                    let col = if lr == 'R' {
                        self.chip.col_clk() + dx
                    } else {
                        self.chip.col_clk() - 1 - dx
                    };
                    if !self.chip.cols_bram.contains(&col) {
                        continue;
                    }
                    self.col_frame[col] = self.frame_info.len();
                    for minor in 0..27 {
                        self.frame_info.push(FrameInfo {
                            addr: FrameAddr {
                                typ: 0,
                                region: 0,
                                major,
                                minor,
                            },
                            mask_mode: [].into_iter().collect(),
                        });
                    }
                    major += 1;
                }
            }
        }

        // bram data
        major = u32::from(self.chip.kind != ChipKind::Virtex);
        for dx in 0..(self.chip.columns / 2) {
            let lrorder = if self.chip.kind == ChipKind::Virtex {
                ['L', 'R']
            } else {
                ['R', 'L']
            };
            for lr in lrorder {
                let col = if lr == 'R' {
                    self.chip.col_clk() + dx
                } else {
                    self.chip.col_clk() - 1 - dx
                };
                if !self.chip.cols_bram.contains(&col) {
                    continue;
                }
                self.bram_frame.insert(col, self.frame_info.len());
                for minor in 0..64 {
                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 1,
                            region: 0,
                            major,
                            minor,
                        },
                        mask_mode: [].into_iter().collect(),
                    });
                }
                major += 1;
            }
        }
    }
}

impl Chip {
    pub fn expand_grid<'a>(
        &'a self,
        disabled: &BTreeSet<DisabledPart>,
        db: &'a IntDb,
    ) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        let (_, die) = egrid.add_die(self.columns, self.rows);

        let mut expander = Expander {
            chip: self,
            db,
            die,
            disabled,
            cols_bram: self.cols_bram.iter().copied().collect(),
            frame_info: vec![],
            spine_frame: 0,
            col_frame: EntityVec::new(),
            bram_frame: EntityPartVec::new(),
            clkv_frame: EntityPartVec::new(),
            blackhole_wires: HashSet::new(),
        };
        expander.fill_int();
        expander.die.fill_main_passes();
        expander.fill_bram();
        expander.fill_clkbt();
        expander.fill_pcilogic();
        expander.fill_clk();
        expander.fill_frame_info();

        let spine_frame = expander.spine_frame;
        let col_frame = expander.col_frame;
        let bram_frame = expander.bram_frame;
        let clkv_frame = expander.clkv_frame;

        let die_bs_geom = DieBitstreamGeom {
            frame_len: self.rows * 18,
            frame_info: expander.frame_info,
            bram_frame_len: 0,
            bram_frame_info: vec![],
            iob_frame_len: 0,
        };
        let bs_geom = BitstreamGeom {
            kind: DeviceKind::Virtex,
            die: [die_bs_geom].into_iter().collect(),
            die_order: vec![expander.die.die],
            has_gtz_bot: false,
            has_gtz_top: false,
        };

        egrid.blackhole_wires = expander.blackhole_wires;
        egrid.finish();
        ExpandedDevice {
            chip: self,
            egrid,
            bs_geom,
            spine_frame,
            col_frame,
            bram_frame,
            clkv_frame,
            disabled: disabled.clone(),
        }
    }
}
