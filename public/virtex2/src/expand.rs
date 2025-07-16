use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::grid::{ColId, ExpandedDieRefMut, ExpandedGrid, Rect, RowId};
use prjcombine_xilinx_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::chip::{Chip, ChipKind, ColumnIoKind, ColumnKind, DcmPairKind, RowIoKind};
use crate::expanded::{ExpandedDevice, REGION_HCLK, REGION_LEAF};
use crate::iob::{get_iob_data_e, get_iob_data_n, get_iob_data_s, get_iob_data_w};
use crate::tslots;

struct Expander<'a, 'b> {
    chip: &'b Chip,
    die: ExpandedDieRefMut<'a, 'b>,
    holes: Vec<Rect>,
    frame_info: Vec<FrameInfo>,
    clkv_frame: usize,
    spine_frame: usize,
    lterm_frame: usize,
    rterm_frame: usize,
    col_frame: EntityVec<ColId, usize>,
    bram_frame: EntityPartVec<ColId, usize>,
}

impl Expander<'_, '_> {
    fn is_hole(&self, col: ColId, row: RowId) -> bool {
        for hole in &self.holes {
            if hole.contains(col, row) {
                return true;
            }
        }
        false
    }

    fn fill_cnr(&mut self) {
        let cnr_kind = if self.chip.kind.is_virtex2() {
            "INT.CNR"
        } else {
            "INT.CLB"
        };
        let (ll, lr, ul, ur) = match self.chip.kind {
            ChipKind::Virtex2 => ("LL.V2", "LR.V2", "UL.V2", "UR.V2"),
            ChipKind::Virtex2P | ChipKind::Virtex2PX => ("LL.V2P", "LR.V2P", "UL.V2P", "UR.V2P"),
            ChipKind::Spartan3 => ("LL.S3", "LR.S3", "UL.S3", "UR.S3"),
            ChipKind::FpgaCore => ("LL.FC", "LR.FC", "UL.FC", "UR.FC"),
            ChipKind::Spartan3E => ("LL.S3E", "LR.S3E", "UL.S3E", "UR.S3E"),
            ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                ("LL.S3A", "LR.S3A", "UL.S3A", "UR.S3A")
            }
        };
        let col_l = self.chip.col_w();
        let col_r = self.chip.col_e();
        let row_b = self.chip.row_s();
        let row_t = self.chip.row_n();

        self.die.fill_tile((col_l, row_b), cnr_kind);
        self.die.fill_tile((col_r, row_b), cnr_kind);
        self.die.fill_tile((col_l, row_t), cnr_kind);
        self.die.fill_tile((col_r, row_t), cnr_kind);
        self.die.add_tile((col_l, row_b), ll, &[(col_l, row_b)]);
        self.die.add_tile((col_r, row_b), lr, &[(col_r, row_b)]);
        self.die.add_tile((col_l, row_t), ul, &[(col_l, row_t)]);
        self.die.add_tile((col_r, row_t), ur, &[(col_r, row_t)]);

        if !self.chip.kind.is_virtex2() {
            self.die.add_tile((col_l, row_t), "RANDOR_INIT", &[]);
        }
    }

    fn fill_term(&mut self) {
        for col in self.chip.columns.ids() {
            self.chip
                .fill_term(&mut self.die, (col, self.chip.row_s()), "TERM.S");
            self.chip
                .fill_term(&mut self.die, (col, self.chip.row_n()), "TERM.N");
        }
        for row in self.chip.rows.ids() {
            self.chip
                .fill_term(&mut self.die, (self.chip.col_w(), row), "TERM.W");
            self.chip
                .fill_term(&mut self.die, (self.chip.col_e(), row), "TERM.E");
        }
    }

    fn fill_io_t(&mut self) {
        let row = self.chip.row_n();
        for (col, &cd) in &self.chip.columns {
            if self.chip.kind.is_spartan3ea() {
                if cd.kind == ColumnKind::Io {
                    continue;
                }
            } else {
                if cd.kind != ColumnKind::Clb {
                    continue;
                }
            }

            let (int_kind, ioi_kind) = match self.chip.kind {
                ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => {
                    if cd.io == ColumnIoKind::DoubleRightClk(1) {
                        ("INT.IOI.CLK_T", "IOI.CLK_T")
                    } else {
                        ("INT.IOI", "IOI")
                    }
                }
                ChipKind::Spartan3 => ("INT.IOI.S3", "IOI.S3"),
                ChipKind::FpgaCore => ("INT.IOI.FC", "IOI.FC"),
                ChipKind::Spartan3E => ("INT.IOI.S3E", "IOI.S3E"),
                ChipKind::Spartan3A | ChipKind::Spartan3ADsp => ("INT.IOI.S3A.TB", "IOI.S3A.T"),
            };
            self.die.fill_tile((col, row), int_kind);
            self.die.add_tile((col, row), ioi_kind, &[(col, row)]);
            if cd.io != ColumnIoKind::None {
                let (data, tidx) = get_iob_data_n(self.chip.kind, cd.io);
                if tidx.to_idx() == 0 {
                    let coords: Vec<_> = (0..data.tiles).map(|dx| (col + dx, row)).collect();
                    self.die.add_tile((col, row), data.node, &coords);
                }
            }
            if !self.chip.kind.is_virtex2() {
                self.die.add_tile((col, row), "RANDOR", &[]);
            }
        }
    }

    fn fill_io_r(&mut self) {
        for row in self.chip.rows.ids() {
            let col = self.chip.col_e();
            if row == self.chip.row_s() || row == self.chip.row_n() {
                continue;
            }
            let (int_kind, ioi_kind) = match self.chip.kind {
                ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => ("INT.IOI", "IOI"),
                ChipKind::Spartan3 => ("INT.IOI.S3", "IOI.S3"),
                ChipKind::FpgaCore => ("INT.IOI.FC", "IOI.FC"),
                ChipKind::Spartan3E => ("INT.IOI.S3E", "IOI.S3E"),
                ChipKind::Spartan3A | ChipKind::Spartan3ADsp => ("INT.IOI.S3A.LR", "IOI.S3A.LR"),
            };
            self.die.fill_tile((col, row), int_kind);
            self.die.add_tile((col, row), ioi_kind, &[(col, row)]);
            let (data, tidx) = get_iob_data_e(self.chip.kind, self.chip.rows[row]);
            if tidx.to_idx() == 0 {
                let coords: Vec<_> = (0..data.tiles).map(|dx| (col, row + dx)).collect();
                self.die.add_tile((col, row), data.node, &coords);
            }
        }
    }

    fn fill_io_b(&mut self) {
        for (col, &cd) in &self.chip.columns {
            let row = self.chip.row_s();
            if self.chip.kind.is_spartan3ea() {
                if cd.kind == ColumnKind::Io {
                    continue;
                }
            } else {
                if cd.kind != ColumnKind::Clb {
                    continue;
                }
            }

            let (int_kind, ioi_kind) = match self.chip.kind {
                ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => {
                    if cd.io == ColumnIoKind::DoubleRightClk(1) {
                        ("INT.IOI.CLK_B", "IOI.CLK_B")
                    } else {
                        ("INT.IOI", "IOI")
                    }
                }
                ChipKind::Spartan3 => ("INT.IOI.S3", "IOI.S3"),
                ChipKind::FpgaCore => ("INT.IOI.FC", "IOI.FC"),
                ChipKind::Spartan3E => ("INT.IOI.S3E", "IOI.S3E"),
                ChipKind::Spartan3A | ChipKind::Spartan3ADsp => ("INT.IOI.S3A.TB", "IOI.S3A.B"),
            };
            self.die.fill_tile((col, row), int_kind);
            self.die.add_tile((col, row), ioi_kind, &[(col, row)]);
            if cd.io != ColumnIoKind::None {
                let (data, tidx) = get_iob_data_s(self.chip.kind, cd.io);
                if tidx.to_idx() == 0 {
                    let coords: Vec<_> = (0..data.tiles).map(|dx| (col + dx, row)).collect();
                    self.die.add_tile((col, row), data.node, &coords);
                }
            }
            if !self.chip.kind.is_virtex2() && self.chip.kind != ChipKind::FpgaCore {
                self.die.add_tile((col, row), "RANDOR", &[(col, row)]);
            }
        }
    }

    fn fill_io_l(&mut self) {
        for row in self.chip.rows.ids() {
            let col = self.chip.col_w();
            if row == self.chip.row_s() || row == self.chip.row_n() {
                continue;
            }
            let int_kind;
            let ioi_kind;
            match self.chip.kind {
                ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => {
                    int_kind = "INT.IOI";
                    ioi_kind = "IOI";
                }
                ChipKind::Spartan3 => {
                    int_kind = "INT.IOI.S3";
                    ioi_kind = "IOI.S3";
                }
                ChipKind::FpgaCore => {
                    int_kind = "INT.IOI.FC";
                    ioi_kind = "IOI.FC";
                }
                ChipKind::Spartan3E => {
                    int_kind = "INT.IOI.S3E";
                    ioi_kind = "IOI.S3E";
                }
                ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                    int_kind = "INT.IOI.S3A.LR";
                    ioi_kind = "IOI.S3A.LR";
                }
            }
            self.die.fill_tile((col, row), int_kind);
            self.die.add_tile((col, row), ioi_kind, &[(col, row)]);
            let (data, tidx) = get_iob_data_w(self.chip.kind, self.chip.rows[row]);
            if tidx.to_idx() == 0 {
                let coords: Vec<_> = (0..data.tiles).map(|dx| (col, row + dx)).collect();
                self.die.add_tile((col, row), data.node, &coords);
            }
        }
    }

    fn fill_clb(&mut self) {
        for (col, &cd) in self.chip.columns.iter() {
            if self.chip.kind == ChipKind::Spartan3E {
                if cd.kind == ColumnKind::Io {
                    continue;
                }
            } else {
                if cd.kind != ColumnKind::Clb {
                    continue;
                }
            }
            for (row, &io) in self.chip.rows.iter() {
                if io == RowIoKind::None {
                    continue;
                }
                if self.is_hole(col, row) {
                    continue;
                }
                self.die.fill_tile((col, row), "INT.CLB");
                self.die.add_tile((col, row), "CLB", &[(col, row)]);
            }
        }
    }

    fn fill_bram_dsp(&mut self) {
        let bram_kind = match self.chip.kind {
            ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => ["INT.BRAM"; 4],
            ChipKind::Spartan3 => ["INT.BRAM.S3"; 4],
            ChipKind::FpgaCore => return,
            ChipKind::Spartan3E => ["INT.BRAM.S3E"; 4],
            ChipKind::Spartan3A => [
                "INT.BRAM.S3A.03",
                "INT.BRAM.S3A.12",
                "INT.BRAM.S3A.12",
                "INT.BRAM.S3A.03",
            ],
            ChipKind::Spartan3ADsp => ["INT.BRAM.S3ADSP"; 4],
        };
        for (col, &cd) in self.chip.columns.iter() {
            if cd.kind != ColumnKind::Bram {
                continue;
            }
            if let Some((b, t)) = self.chip.rows_ram {
                self.holes.push(Rect {
                    col_l: col,
                    col_r: col + 4,
                    row_b: b,
                    row_t: t + 1,
                });
                for d in 1..4 {
                    self.die.fill_conn_pair(
                        (col + d, b - 1),
                        (col + d, t + 1),
                        "TERM.BRAM.N",
                        "TERM.BRAM.S",
                    );
                }
            }
            for row in self.chip.rows.ids() {
                if self.chip.kind != ChipKind::Spartan3E && self.is_hole(col, row) {
                    continue;
                }
                let Some(idx) = self.chip.bram_row(row) else {
                    continue;
                };
                self.die.fill_tile((col, row), bram_kind[idx]);
                if self.chip.kind == ChipKind::Spartan3ADsp {
                    self.die.fill_tile((col + 3, row), "INT.BRAM.S3ADSP");
                }
                if idx == 0 {
                    let kind = match self.chip.kind {
                        ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => "BRAM",
                        ChipKind::Spartan3 => "BRAM.S3",
                        ChipKind::FpgaCore => unreachable!(),
                        ChipKind::Spartan3E => "BRAM.S3E",
                        ChipKind::Spartan3A => "BRAM.S3A",
                        ChipKind::Spartan3ADsp => "BRAM.S3ADSP",
                    };
                    self.die.add_tile(
                        (col, row),
                        kind,
                        &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                    );
                    if self.chip.kind == ChipKind::Spartan3ADsp {
                        self.die.add_tile(
                            (col + 3, row),
                            "DSP",
                            &[
                                (col + 3, row),
                                (col + 3, row + 1),
                                (col + 3, row + 2),
                                (col + 3, row + 3),
                            ],
                        );
                        self.die.add_tile(
                            (col + 3, row),
                            "INTF.DSP",
                            &[
                                (col + 3, row),
                                (col + 3, row + 1),
                                (col + 3, row + 2),
                                (col + 3, row + 3),
                            ],
                        );
                    }
                }
            }
        }
    }

    fn fill_dcm(&mut self) {
        if self.chip.kind.is_spartan3ea() {
            for pair in self.chip.get_dcm_pairs() {
                match pair.kind {
                    DcmPairKind::Bot => {
                        self.holes.push(Rect {
                            col_l: pair.col - 4,
                            col_r: pair.col + 4,
                            row_b: pair.row,
                            row_t: pair.row + 4,
                        });
                        let crd = (pair.col - 1, pair.row);
                        self.die.fill_tile(crd, "INT.DCM");
                        self.die.add_tile(crd, "DCM.S3E.BL", &[crd]);
                        let crd = (pair.col, pair.row);
                        self.die.fill_tile(crd, "INT.DCM");
                        self.die.add_tile(crd, "DCM.S3E.BR", &[crd]);
                    }
                    DcmPairKind::BotSingle => {
                        self.holes.push(Rect {
                            col_l: pair.col - 1,
                            col_r: pair.col + 4,
                            row_b: pair.row,
                            row_t: pair.row + 4,
                        });
                        let crd = (pair.col - 1, pair.row);
                        self.die.fill_tile(crd, "INT.DCM.S3E.DUMMY");
                        let crd = (pair.col, pair.row);
                        self.die.fill_tile(crd, "INT.DCM");
                        self.die.add_tile(crd, "DCM.S3E.BR", &[crd]);
                    }
                    DcmPairKind::Top => {
                        self.holes.push(Rect {
                            col_l: pair.col - 4,
                            col_r: pair.col + 4,
                            row_b: pair.row - 3,
                            row_t: pair.row + 1,
                        });
                        let crd = (pair.col - 1, pair.row);
                        self.die.fill_tile(crd, "INT.DCM");
                        self.die.add_tile(crd, "DCM.S3E.TL", &[crd]);
                        let crd = (pair.col, pair.row);
                        self.die.fill_tile(crd, "INT.DCM");
                        self.die.add_tile(crd, "DCM.S3E.TR", &[crd]);
                    }
                    DcmPairKind::TopSingle => {
                        self.holes.push(Rect {
                            col_l: pair.col - 1,
                            col_r: pair.col + 4,
                            row_b: pair.row - 3,
                            row_t: pair.row + 1,
                        });
                        let crd = (pair.col - 1, pair.row);
                        self.die.fill_tile(crd, "INT.DCM.S3E.DUMMY");
                        let crd = (pair.col, pair.row);
                        self.die.fill_tile(crd, "INT.DCM");
                        self.die.add_tile(crd, "DCM.S3E.TR", &[crd]);
                    }
                    DcmPairKind::Left => {
                        self.holes.push(Rect {
                            col_l: pair.col,
                            col_r: pair.col + 4,
                            row_b: pair.row - 4,
                            row_t: pair.row + 4,
                        });
                        let crd = (pair.col, pair.row - 1);
                        self.die.fill_tile(crd, "INT.DCM");
                        self.die.add_tile(crd, "DCM.S3E.LB", &[crd]);
                        let crd = (pair.col, pair.row);
                        self.die.fill_tile(crd, "INT.DCM");
                        self.die.add_tile(crd, "DCM.S3E.LT", &[crd]);
                    }
                    DcmPairKind::Right => {
                        self.holes.push(Rect {
                            col_l: pair.col - 3,
                            col_r: pair.col + 1,
                            row_b: pair.row - 4,
                            row_t: pair.row + 4,
                        });
                        let crd = (pair.col, pair.row - 1);
                        self.die.fill_tile(crd, "INT.DCM");
                        self.die.add_tile(crd, "DCM.S3E.RB", &[crd]);
                        let crd = (pair.col, pair.row);
                        self.die.fill_tile(crd, "INT.DCM");
                        self.die.add_tile(crd, "DCM.S3E.RT", &[crd]);
                    }
                    DcmPairKind::Bram => {
                        self.holes.push(Rect {
                            col_l: pair.col,
                            col_r: pair.col + 4,
                            row_b: pair.row - 4,
                            row_t: pair.row + 4,
                        });
                        let crd = (pair.col, pair.row - 1);
                        self.die.fill_tile(crd, "INT.DCM");
                        self.die.add_tile(crd, "DCM.S3E.LB", &[crd]);
                        let crd = (pair.col, pair.row);
                        self.die.fill_tile(crd, "INT.DCM");
                        self.die.add_tile(crd, "DCM.S3E.LT", &[crd]);
                    }
                }
            }
        } else {
            let row_b = self.chip.row_s();
            let row_t = self.chip.row_n();
            for (col, &cd) in self.chip.columns.iter() {
                if cd.kind != ColumnKind::Bram {
                    continue;
                }
                if !self.chip.cols_gt.contains_key(&col) {
                    let (kind, dcm) = match self.chip.kind {
                        ChipKind::Virtex2 => ("INT.DCM.V2", "DCM.V2"),
                        ChipKind::Virtex2P | ChipKind::Virtex2PX => ("INT.DCM.V2P", "DCM.V2P"),
                        ChipKind::Spartan3 => {
                            if col == self.chip.col_w() + 3 || col == self.chip.col_e() - 3 {
                                ("INT.DCM", "DCM.S3")
                            } else {
                                ("INT.DCM.S3.DUMMY", "")
                            }
                        }
                        _ => unreachable!(),
                    };
                    self.die.fill_tile((col, row_b), kind);
                    self.die.fill_tile((col, row_t), kind);
                    if !dcm.is_empty() {
                        self.die.add_tile((col, row_b), dcm, &[(col, row_b)]);
                        self.die.add_tile((col, row_t), dcm, &[(col, row_t)]);
                    }
                }
                self.die
                    .add_tile((col, row_b), "DCMCONN.BOT", &[(col, row_b)]);
                self.die
                    .add_tile((col, row_t), "DCMCONN.TOP", &[(col, row_t)]);
            }
        }
    }

    fn fill_ppc(&mut self) {
        for &(bc, br) in &self.chip.holes_ppc {
            self.holes.push(Rect {
                col_l: bc,
                col_r: bc + 10,
                row_b: br,
                row_t: br + 16,
            });

            let mut ints = vec![];
            // left side
            for d in 0..16 {
                ints.push((bc, br + d));
            }
            // right side
            for d in 0..16 {
                ints.push((bc + 9, br + d));
            }
            // bottom
            for d in 1..9 {
                ints.push((bc + d, br));
            }
            // top
            for d in 1..9 {
                ints.push((bc + d, br + 15));
            }

            for &crd in &ints {
                self.die.fill_tile(crd, "INT.PPC");
                self.die.add_tile(crd, "INTF.PPC", &[crd]);
            }
            // horiz passes
            for d in 1..15 {
                let col_l = bc;
                let col_r = bc + 9;
                let row = br + d;
                self.die
                    .add_tile((col_l, row), "PPC.E", &[(col_l, row), (col_r, row)]);
                self.die
                    .add_tile((col_r, row), "PPC.W", &[(col_r, row), (col_l, row)]);
                self.die
                    .fill_conn_pair((col_l, row), (col_r, row), "PPC.E", "PPC.W");
            }
            // vert passes
            for d in 1..9 {
                let col = bc + d;
                let row_b = br;
                let row_t = br + 15;
                self.die
                    .add_tile((col, row_b), "PPC.N", &[(col, row_b), (col, row_t)]);
                self.die
                    .add_tile((col, row_t), "PPC.S", &[(col, row_t), (col, row_b)]);
                self.die
                    .fill_conn_pair((col, row_b), (col, row_t), "PPC.N", "PPC.S");
            }
            let kind = if bc < self.chip.col_clk {
                "LBPPC"
            } else {
                "RBPPC"
            };
            self.die.add_tile((bc, br), kind, &ints);
        }
    }

    fn fill_gt(&mut self) {
        let row_b = self.chip.row_s();
        let row_t = self.chip.row_n();
        for col in self.chip.cols_gt.keys().copied() {
            if self.chip.kind == ChipKind::Virtex2PX {
                self.holes.push(Rect {
                    col_l: col,
                    col_r: col + 1,
                    row_b,
                    row_t: row_b + 9,
                });
                self.holes.push(Rect {
                    col_l: col,
                    col_r: col + 1,
                    row_b: row_t - 8,
                    row_t: row_t + 1,
                });
            } else {
                self.holes.push(Rect {
                    col_l: col,
                    col_r: col + 1,
                    row_b,
                    row_t: row_b + 5,
                });
                self.holes.push(Rect {
                    col_l: col,
                    col_r: col + 1,
                    row_b: row_t - 4,
                    row_t: row_t + 1,
                });
            }
            for row in [row_b, row_t] {
                self.die.fill_tile((col, row), "INT.GT.CLKPAD");
                self.die.add_tile(
                    (col, row),
                    if row == row_b {
                        "INTF.GT.BCLKPAD"
                    } else {
                        "INTF.GT.TCLKPAD"
                    },
                    &[(col, row)],
                );
            }
            let n = match self.chip.kind {
                ChipKind::Virtex2P => 4,
                ChipKind::Virtex2PX => 8,
                _ => unreachable!(),
            };
            for br in [row_b + 1, row_t - n] {
                for d in 0..n {
                    let row = br + d;
                    self.die.fill_tile((col, row), "INT.PPC");
                    self.die.add_tile(
                        (col, row),
                        if d % 4 == 0 {
                            if br == row_b + 1 {
                                "INTF.GT.B0"
                            } else {
                                "INTF.GT.T0"
                            }
                        } else {
                            if br == row_b + 1 {
                                "INTF.GT.B123"
                            } else {
                                "INTF.GT.T123"
                            }
                        },
                        &[(col, row)],
                    );
                }
            }
            if self.chip.kind == ChipKind::Virtex2P {
                self.die.add_tile(
                    (col, row_b),
                    "GIGABIT.B",
                    &[
                        (col, row_b),
                        (col, row_b + 1),
                        (col, row_b + 2),
                        (col, row_b + 3),
                        (col, row_b + 4),
                    ],
                );
                self.die.add_tile(
                    (col, row_t),
                    "GIGABIT.T",
                    &[
                        (col, row_t),
                        (col, row_t - 4),
                        (col, row_t - 3),
                        (col, row_t - 2),
                        (col, row_t - 1),
                    ],
                );
            } else {
                self.die.add_tile(
                    (col, row_b),
                    "GIGABIT10.B",
                    &[
                        (col, row_b),
                        (col, row_b + 1),
                        (col, row_b + 2),
                        (col, row_b + 3),
                        (col, row_b + 4),
                        (col, row_b + 5),
                        (col, row_b + 6),
                        (col, row_b + 7),
                        (col, row_b + 8),
                    ],
                );
                self.die.add_tile(
                    (col, row_t),
                    "GIGABIT10.T",
                    &[
                        (col, row_t),
                        (col, row_t - 8),
                        (col, row_t - 7),
                        (col, row_t - 6),
                        (col, row_t - 5),
                        (col, row_t - 4),
                        (col, row_t - 3),
                        (col, row_t - 2),
                        (col, row_t - 1),
                    ],
                );
            }
        }
    }

    fn fill_llv(&mut self) {
        for col in self.chip.columns.ids() {
            if matches!(self.chip.columns[col].kind, ColumnKind::BramCont(_)) {
                continue;
            }
            let mut row_s = self.chip.row_mid() - 1;
            let mut row_n = self.chip.row_mid();
            while !self.die[(col, row_s)].tiles.contains_id(tslots::INT) {
                row_s -= 1;
            }
            while !self.die[(col, row_n)].tiles.contains_id(tslots::INT) {
                row_n += 1;
            }
            let mut term_s = "LLV.S";
            let mut term_n = "LLV.N";
            if (col == self.chip.col_w() || col == self.chip.col_e())
                && self.chip.kind != ChipKind::Spartan3A
            {
                term_s = "LLV.CLKLR.S3E.S";
                term_n = "LLV.CLKLR.S3E.N";
            }
            self.die
                .fill_conn_pair((col, row_s), (col, row_n), term_n, term_s);
            self.die.add_tile(
                (col, row_n),
                if self.chip.kind.is_spartan3a() {
                    "LLV.S3A"
                } else {
                    "LLV.S3E"
                },
                &[(col, row_s), (col, row_n)],
            );
        }
    }

    fn fill_llh(&mut self) {
        let row_b = self.chip.row_s();
        let row_t = self.chip.row_n();
        for row in self.chip.rows.ids() {
            let mut col_l = self.chip.col_clk - 1;
            let mut col_r = self.chip.col_clk;
            while !self.die[(col_l, row)].tiles.contains_id(tslots::INT) {
                col_l -= 1;
            }
            while !self.die[(col_r, row)].tiles.contains_id(tslots::INT) {
                col_r += 1;
            }
            let mut term_w = "LLH.W";
            let mut term_e = "LLH.E";
            if self.chip.kind == ChipKind::Spartan3ADsp
                && [
                    row_b + 2,
                    row_b + 3,
                    row_b + 4,
                    row_t - 4,
                    row_t - 3,
                    row_t - 2,
                ]
                .into_iter()
                .any(|x| x == row)
            {
                term_w = "LLH.DCM.S3ADSP.W";
                term_e = "LLH.DCM.S3ADSP.E";
            }
            self.die
                .fill_conn_pair((col_l, row), (col_r, row), term_e, term_w);
            self.die.add_tile(
                (col_r, row),
                if self.chip.kind.is_spartan3a() && row == self.chip.row_s() {
                    "LLH.CLKB.S3A"
                } else if self.chip.kind.is_spartan3a() && row == self.chip.row_n() {
                    "LLH.CLKT.S3A"
                } else {
                    "LLH"
                },
                &[(col_l, row), (col_r, row)],
            );
        }
    }

    fn fill_misc_passes(&mut self) {
        if self.chip.kind == ChipKind::Spartan3E && !self.chip.has_ll {
            for col in [self.chip.col_w(), self.chip.col_e()] {
                self.die.fill_conn_pair(
                    (col, self.chip.row_mid() - 1),
                    (col, self.chip.row_mid()),
                    "CLKLR.S3E.N",
                    "CLKLR.S3E.S",
                );
            }
        }
        if self.chip.kind == ChipKind::Spartan3 {
            for &(_, _, row_n) in &self.chip.rows_hclk {
                let row_s = row_n - 1;
                if row_n == self.chip.row_mid() {
                    continue;
                }
                if row_s == self.chip.row_n() {
                    continue;
                }
                for col in self.die.cols() {
                    self.die
                        .fill_conn_pair((col, row_s), (col, row_n), "BRKH.S3.N", "BRKH.S3.S");
                }
            }
        }
        if self.chip.kind == ChipKind::Spartan3ADsp {
            for (col, cd) in &self.chip.columns {
                if cd.kind == ColumnKind::Dsp {
                    for row in [self.chip.row_s(), self.chip.row_n()] {
                        self.die.fill_conn_pair(
                            (col, row),
                            (col + 1, row),
                            "DSPHOLE.E",
                            "DSPHOLE.W",
                        );
                    }
                }
            }
            for col in [self.chip.col_w() + 3, self.chip.col_e() - 6] {
                for row in [self.chip.row_mid() - 1, self.chip.row_mid()] {
                    self.die
                        .fill_conn_pair((col, row), (col + 4, row), "DSPHOLE.E", "DSPHOLE.W");
                }
                for row in [
                    self.chip.row_mid() - 4,
                    self.chip.row_mid() - 3,
                    self.chip.row_mid() - 2,
                    self.chip.row_mid() + 1,
                    self.chip.row_mid() + 2,
                    self.chip.row_mid() + 3,
                ] {
                    self.die
                        .fill_conn_pair((col - 1, row), (col + 4, row), "HDCM.E", "HDCM.W");
                }
            }
        }
    }

    fn fill_bram_passes(&mut self) {
        if matches!(self.chip.kind, ChipKind::Spartan3A | ChipKind::Spartan3ADsp) {
            let slot_n = self.die.grid.db.get_conn_slot("N");
            let slot_s = self.die.grid.db.get_conn_slot("S");
            for (col, cd) in &self.chip.columns {
                if matches!(cd.kind, ColumnKind::BramCont(_)) {
                    self.die[(col, self.chip.row_s())].conns.remove(slot_n);
                    self.die[(col, self.chip.row_n())].conns.remove(slot_s);
                }
            }
        }
    }

    fn fill_clkbt_v2(&mut self) {
        let row_b = self.chip.row_s();
        let row_t = self.chip.row_n();
        let (kind_b, kind_t) = match self.chip.kind {
            ChipKind::Virtex2 => ("CLKB.V2", "CLKT.V2"),
            ChipKind::Virtex2P => ("CLKB.V2P", "CLKT.V2P"),
            ChipKind::Virtex2PX => ("CLKB.V2PX", "CLKT.V2PX"),
            _ => unreachable!(),
        };
        self.die.add_tile(
            (self.chip.col_clk, row_b),
            kind_b,
            &[(self.chip.col_clk - 1, row_b), (self.chip.col_clk, row_b)],
        );
        self.die.add_tile(
            (self.chip.col_clk, row_t),
            kind_t,
            &[(self.chip.col_clk - 1, row_t), (self.chip.col_clk, row_t)],
        );

        let col_l = self.chip.col_w();
        self.die.add_tile(
            (col_l, self.chip.row_pci.unwrap()),
            "REG_L",
            &[
                (col_l, self.chip.row_pci.unwrap() - 2),
                (col_l, self.chip.row_pci.unwrap() - 1),
                (col_l, self.chip.row_pci.unwrap()),
                (col_l, self.chip.row_pci.unwrap() + 1),
            ],
        );
        let col_r = self.chip.col_e();
        self.die.add_tile(
            (col_r, self.chip.row_pci.unwrap()),
            "REG_R",
            &[
                (col_r, self.chip.row_pci.unwrap() - 2),
                (col_r, self.chip.row_pci.unwrap() - 1),
                (col_r, self.chip.row_pci.unwrap()),
                (col_r, self.chip.row_pci.unwrap() + 1),
            ],
        );
    }

    fn fill_clkbt_s3(&mut self) {
        let (clkb, clkt) = match self.chip.kind {
            ChipKind::Spartan3 => ("CLKB.S3", "CLKT.S3"),
            ChipKind::FpgaCore => ("CLKB.FC", "CLKT.FC"),
            _ => unreachable!(),
        };
        let row_b = self.chip.row_s();
        let row_t = self.chip.row_n();
        self.die.add_tile(
            (self.chip.col_clk, row_b),
            clkb,
            &[(self.chip.col_clk - 1, row_b)],
        );
        self.die.add_tile(
            (self.chip.col_clk, row_t),
            clkt,
            &[(self.chip.col_clk - 1, row_t)],
        );
    }

    fn fill_clkbt_s3e(&mut self) {
        self.die.add_tile(
            (self.chip.col_clk, self.chip.row_s()),
            if self.chip.kind == ChipKind::Spartan3E {
                "CLKB.S3E"
            } else {
                "CLKB.S3A"
            },
            &[(self.chip.col_clk - 1, self.chip.row_s())],
        );
        self.die.add_tile(
            (self.chip.col_clk, self.chip.row_n()),
            if self.chip.kind == ChipKind::Spartan3E {
                "CLKT.S3E"
            } else {
                "CLKT.S3A"
            },
            &[(self.chip.col_clk - 1, self.chip.row_n())],
        );
    }

    fn fill_clklr_s3e(&mut self) {
        self.die.add_tile(
            (self.chip.col_w(), self.chip.row_mid()),
            if self.chip.kind == ChipKind::Spartan3E {
                "CLKL.S3E"
            } else {
                "CLKL.S3A"
            },
            &[
                (self.chip.col_w(), self.chip.row_mid() - 1),
                (self.chip.col_w(), self.chip.row_mid()),
            ],
        );
        self.die.add_tile(
            (self.chip.col_e(), self.chip.row_mid()),
            if self.chip.kind == ChipKind::Spartan3E {
                "CLKR.S3E"
            } else {
                "CLKR.S3A"
            },
            &[
                (self.chip.col_e(), self.chip.row_mid() - 1),
                (self.chip.col_e(), self.chip.row_mid()),
            ],
        );
    }

    fn fill_pci_ce(&mut self) {
        if self.chip.kind.is_spartan3ea() {
            for c in [
                (self.chip.col_w(), self.chip.row_s()),
                (self.chip.col_e(), self.chip.row_s()),
                (self.chip.col_w(), self.chip.row_n()),
                (self.chip.col_e(), self.chip.row_n()),
            ] {
                self.die.add_tile(c, "PCI_CE_CNR", &[]);
            }

            for &(row, _, _) in &self.chip.rows_hclk {
                let kind = if row > self.chip.row_mid() {
                    "PCI_CE_N"
                } else {
                    "PCI_CE_S"
                };
                for col in [self.chip.col_w(), self.chip.col_e()] {
                    self.die.add_tile((col, row), kind, &[]);
                }
            }
            if self.chip.kind == ChipKind::Spartan3A
                && let Some((col_l, col_r)) = self.chip.cols_clkv
            {
                for row in [self.chip.row_s(), self.chip.row_n()] {
                    self.die.add_tile((col_l, row), "PCI_CE_E", &[]);
                    self.die.add_tile((col_r, row), "PCI_CE_W", &[]);
                }
            }
        }
    }

    fn fill_gclkh(&mut self) {
        for col in self.die.cols() {
            let col_q = if col < self.chip.col_clk {
                if let Some((col_q, _)) = self.chip.cols_clkv {
                    col_q
                } else {
                    self.chip.col_clk - 1
                }
            } else {
                if let Some((_, col_q)) = self.chip.cols_clkv {
                    col_q
                } else {
                    self.chip.col_clk
                }
            };
            for (i, &(row_m, row_b, row_t)) in self.chip.rows_hclk.iter().enumerate() {
                let row_q = if self.chip.kind.is_virtex2() {
                    row_m
                } else if row_m < self.chip.row_mid() {
                    self.chip.row_mid() - 1
                } else {
                    self.chip.row_mid()
                };
                for r in row_b.to_idx()..row_m.to_idx() {
                    let row = RowId::from_idx(r);
                    self.die[(col, row)].region_root[REGION_LEAF] = (col, row_m - 1);
                    self.die[(col, row)].region_root[REGION_HCLK] = (col_q, row_q);
                }
                for r in row_m.to_idx()..row_t.to_idx() {
                    let row = RowId::from_idx(r);
                    self.die[(col, row)].region_root[REGION_LEAF] = (col, row_m);
                    self.die[(col, row)].region_root[REGION_HCLK] = (col_q, row_q);
                }
                let kind = if matches!(self.chip.columns[col].kind, ColumnKind::BramCont(_)) {
                    if row_m == self.chip.row_mid() {
                        "GCLKH.UNI"
                    } else if i == 0 {
                        if self.chip.kind == ChipKind::Spartan3E {
                            "GCLKH.S"
                        } else {
                            "GCLKH.UNI.S"
                        }
                    } else if i == self.chip.rows_hclk.len() - 1 {
                        if self.chip.kind == ChipKind::Spartan3E {
                            "GCLKH.N"
                        } else {
                            "GCLKH.UNI.N"
                        }
                    } else {
                        "GCLKH.0"
                    }
                } else {
                    "GCLKH"
                };
                self.die
                    .add_tile((col, row_m), kind, &[(col, row_m - 1), (col, row_m)]);
                if self.chip.columns[col].kind == ColumnKind::Dsp {
                    self.die.add_tile((col, row_m), "GCLKH.DSP", &[]);
                }
            }
        }
    }

    fn fill_gclkc(&mut self) {
        for &(row_m, _, _) in &self.chip.rows_hclk {
            if self.chip.kind.is_virtex2() {
                let node_kind = if row_m == self.chip.row_s() + 1 {
                    "GCLKC.B"
                } else if row_m == self.chip.row_n() {
                    "GCLKC.T"
                } else {
                    "GCLKC"
                };
                self.die
                    .add_tile((self.chip.col_clk, row_m), node_kind, &[]);
            } else if let Some((col_cl, col_cr)) = self.chip.cols_clkv {
                for col in [col_cl, col_cr] {
                    self.die.add_tile((col, row_m), "GCLKVC", &[]);
                }
            }
        }
    }

    fn fill_clkc(&mut self) {
        let kind = if !self.chip.kind.is_virtex2() && self.chip.cols_clkv.is_none() {
            "CLKC_50A"
        } else {
            "CLKC"
        };
        self.die
            .add_tile((self.chip.col_clk, self.chip.row_mid()), kind, &[]);
    }

    fn fill_gclkvm(&mut self) {
        if let Some((col_cl, col_cr)) = self.chip.cols_clkv {
            if matches!(self.chip.kind, ChipKind::Spartan3 | ChipKind::FpgaCore) {
                self.die
                    .add_tile((col_cl, self.chip.row_mid()), "GCLKVM.S3", &[]);
                self.die
                    .add_tile((col_cr, self.chip.row_mid()), "GCLKVM.S3", &[]);
            } else {
                self.die
                    .add_tile((col_cl, self.chip.row_mid()), "GCLKVM.S3E", &[]);
                self.die
                    .add_tile((col_cr, self.chip.row_mid()), "GCLKVM.S3E", &[]);
            }
        }
    }

    fn fill_frame_info(&mut self) {
        let mut major = 0;
        // spine
        self.clkv_frame = 0;
        let num_spine = if self.chip.kind.is_virtex2() {
            self.spine_frame = 0;
            4
        } else if self.chip.cols_clkv.is_none() {
            self.spine_frame = 0;
            2
        } else if self.chip.has_ll || self.chip.kind.is_spartan3a() {
            self.spine_frame = 2;
            4
        } else {
            self.spine_frame = 2;
            3
        };
        for minor in 0..num_spine {
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
        let num_term = if self.chip.kind.is_virtex2() { 4 } else { 2 };
        self.lterm_frame = self.frame_info.len();
        for minor in 0..num_term {
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
        let num_main = if self.chip.kind.is_virtex2() { 22 } else { 19 };
        for (_, cd) in &self.chip.columns {
            // For Bram and BramCont, to be fixed later.
            self.col_frame.push(self.frame_info.len());
            if matches!(cd.kind, ColumnKind::BramCont(_) | ColumnKind::Bram) {
                continue;
            }
            for minor in 0..num_main {
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
        self.rterm_frame = self.frame_info.len();
        for minor in 0..num_term {
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

        major = 0;
        let num_bram = if self.chip.kind.is_virtex2() { 64 } else { 76 };
        for (col, cd) in &self.chip.columns {
            if cd.kind != ColumnKind::Bram {
                continue;
            }
            self.bram_frame.insert(col, self.frame_info.len());
            for minor in 0..num_bram {
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

        major = 0;
        for (col, cd) in &self.chip.columns {
            if cd.kind != ColumnKind::Bram {
                continue;
            }
            self.col_frame[col] = self.frame_info.len();
            for minor in 0..num_main {
                self.frame_info.push(FrameInfo {
                    addr: FrameAddr {
                        typ: 2,
                        region: 0,
                        major,
                        minor,
                    },
                    mask_mode: [].into_iter().collect(),
                });
            }
            major += 1;
        }

        for (col, cd) in &self.chip.columns {
            if let ColumnKind::BramCont(i) = cd.kind {
                self.col_frame[col] = self.bram_frame[col - (i as usize)] + (i as usize - 1) * 19;
            }
        }
    }
}

impl Chip {
    fn fill_term(&self, die: &mut ExpandedDieRefMut, coord: (ColId, RowId), kind: &str) {
        if self.kind.is_virtex2() {
            die.add_tile(coord, kind, &[coord]);
        }
        die.fill_conn_term(coord, kind);
    }

    pub fn expand_grid<'a>(&'a self, db: &'a IntDb) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);

        let (_, die) = egrid.add_die(self.columns.len(), self.rows.len());

        let mut expander = Expander {
            chip: self,
            die,
            holes: vec![],
            frame_info: vec![],
            clkv_frame: 0,
            spine_frame: 0,
            lterm_frame: 0,
            rterm_frame: 0,
            col_frame: EntityVec::new(),
            bram_frame: EntityPartVec::new(),
        };

        expander.fill_gt();
        expander.fill_ppc();
        expander.fill_dcm();
        expander.fill_bram_dsp();
        expander.fill_clb();
        expander.fill_cnr();
        expander.fill_io_t();
        expander.fill_io_r();
        expander.fill_io_b();
        expander.fill_io_l();
        expander.fill_term();
        if self.has_ll {
            expander.fill_llv();
            expander.fill_llh();
        }
        expander.fill_misc_passes();
        expander.die.fill_main_passes();
        expander.fill_bram_passes();
        if self.kind.is_virtex2() {
            expander.fill_clkbt_v2();
        } else if matches!(self.kind, ChipKind::Spartan3 | ChipKind::FpgaCore) {
            expander.fill_clkbt_s3();
        } else {
            expander.fill_clkbt_s3e();
            expander.fill_clklr_s3e();
        }
        expander.fill_pci_ce();
        expander.fill_gclkh();
        expander.fill_gclkc();
        expander.fill_clkc();
        expander.fill_gclkvm();
        expander.fill_frame_info();

        let clkv_frame = expander.clkv_frame;
        let spine_frame = expander.spine_frame;
        let lterm_frame = expander.lterm_frame;
        let rterm_frame = expander.rterm_frame;
        let col_frame = expander.col_frame;
        let bram_frame = expander.bram_frame;
        let holes = expander.holes;

        let die_bs_geom = DieBitstreamGeom {
            frame_len: 32 + self.rows.len() * if self.kind.is_virtex2() { 80 } else { 64 },
            frame_info: expander.frame_info,
            bram_frame_len: 0,
            bram_frame_info: vec![],
            iob_frame_len: 0,
        };
        let bs_geom = BitstreamGeom {
            kind: if self.kind.is_spartan3a() {
                DeviceKind::Spartan3A
            } else {
                DeviceKind::Virtex2
            },
            die: [die_bs_geom].into_iter().collect(),
            die_order: vec![expander.die.die],
            has_gtz_bot: false,
            has_gtz_top: false,
        };

        egrid.finish();
        ExpandedDevice {
            chip: self,
            egrid,
            bs_geom,
            clkv_frame,
            spine_frame,
            lterm_frame,
            rterm_frame,
            col_frame,
            bram_frame,
            holes,
        }
    }
}
