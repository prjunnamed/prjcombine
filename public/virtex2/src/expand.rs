use prjcombine_interconnect::db::{Dir, IntDb};
use prjcombine_interconnect::grid::{ColId, Coord, ExpandedDieRefMut, ExpandedGrid, Rect, RowId};
use prjcombine_xilinx_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::expanded::ExpandedDevice;
use crate::grid::{ColumnIoKind, ColumnKind, DcmPairKind, Grid, GridKind, RowIoKind};
use crate::iob::{get_iob_data_b, get_iob_data_l, get_iob_data_r, get_iob_data_t};

struct Expander<'a, 'b> {
    grid: &'b Grid,
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
        let cnr_kind = if self.grid.kind.is_virtex2() {
            "INT.CNR"
        } else {
            "INT.CLB"
        };
        let (ll, lr, ul, ur) = match self.grid.kind {
            GridKind::Virtex2 => ("LL.V2", "LR.V2", "UL.V2", "UR.V2"),
            GridKind::Virtex2P | GridKind::Virtex2PX => ("LL.V2P", "LR.V2P", "UL.V2P", "UR.V2P"),
            GridKind::Spartan3 => ("LL.S3", "LR.S3", "UL.S3", "UR.S3"),
            GridKind::FpgaCore => ("LL.FC", "LR.FC", "UL.FC", "UR.FC"),
            GridKind::Spartan3E => ("LL.S3E", "LR.S3E", "UL.S3E", "UR.S3E"),
            GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                ("LL.S3A", "LR.S3A", "UL.S3A", "UR.S3A")
            }
        };
        let col_l = self.grid.col_left();
        let col_r = self.grid.col_right();
        let row_b = self.grid.row_bot();
        let row_t = self.grid.row_top();

        self.die.fill_tile((col_l, row_b), cnr_kind);
        self.die.fill_tile((col_r, row_b), cnr_kind);
        self.die.fill_tile((col_l, row_t), cnr_kind);
        self.die.fill_tile((col_r, row_t), cnr_kind);
        self.die.add_xnode((col_l, row_b), ll, &[(col_l, row_b)]);
        self.die.add_xnode((col_r, row_b), lr, &[(col_r, row_b)]);
        self.die.add_xnode((col_l, row_t), ul, &[(col_l, row_t)]);
        self.die.add_xnode((col_r, row_t), ur, &[(col_r, row_t)]);
    }

    fn fill_term(&mut self) {
        for col in self.grid.columns.ids() {
            self.grid
                .fill_term(&mut self.die, (col, self.grid.row_bot()), "TERM.S");
            self.grid
                .fill_term(&mut self.die, (col, self.grid.row_top()), "TERM.N");
        }
        for row in self.grid.rows.ids() {
            self.grid
                .fill_term(&mut self.die, (self.grid.col_left(), row), "TERM.W");
            self.grid
                .fill_term(&mut self.die, (self.grid.col_right(), row), "TERM.E");
        }
    }

    fn fill_io_t(&mut self) {
        let row = self.grid.row_top();
        for (col, &cd) in &self.grid.columns {
            if self.grid.kind.is_spartan3ea() {
                if cd.kind == ColumnKind::Io {
                    continue;
                }
            } else {
                if cd.kind != ColumnKind::Clb {
                    continue;
                }
            }

            let (int_kind, ioi_kind) = match self.grid.kind {
                GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
                    if cd.io == ColumnIoKind::DoubleRightClk(1) {
                        ("INT.IOI.CLK_T", "IOI.CLK_T")
                    } else {
                        ("INT.IOI", "IOI")
                    }
                }
                GridKind::Spartan3 => ("INT.IOI.S3", "IOI.S3"),
                GridKind::FpgaCore => ("INT.IOI.FC", "IOI.FC"),
                GridKind::Spartan3E => ("INT.IOI.S3E", "IOI.S3E"),
                GridKind::Spartan3A | GridKind::Spartan3ADsp => ("INT.IOI.S3A.TB", "IOI.S3A.T"),
            };
            self.die.fill_tile((col, row), int_kind);
            self.die.add_xnode((col, row), ioi_kind, &[(col, row)]);
            if cd.io != ColumnIoKind::None {
                let (data, tidx) = get_iob_data_t(self.grid.kind, cd.io);
                if tidx == 0 {
                    let coords: Vec<_> = (0..data.tiles).map(|dx| (col + dx, row)).collect();
                    self.die.add_xnode((col, row), data.node, &coords);
                }
            }
            if !self.grid.kind.is_virtex2() {
                self.die.add_xnode((col, row), "RANDOR", &[]);
            }
        }
    }

    fn fill_io_r(&mut self) {
        for row in self.grid.rows.ids() {
            let col = self.grid.col_right();
            if row == self.grid.row_bot() || row == self.grid.row_top() {
                continue;
            }
            let (int_kind, ioi_kind) = match self.grid.kind {
                GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => ("INT.IOI", "IOI"),
                GridKind::Spartan3 => ("INT.IOI.S3", "IOI.S3"),
                GridKind::FpgaCore => ("INT.IOI.FC", "IOI.FC"),
                GridKind::Spartan3E => ("INT.IOI.S3E", "IOI.S3E"),
                GridKind::Spartan3A | GridKind::Spartan3ADsp => ("INT.IOI.S3A.LR", "IOI.S3A.LR"),
            };
            self.die.fill_tile((col, row), int_kind);
            self.die.add_xnode((col, row), ioi_kind, &[(col, row)]);
            let (data, tidx) = get_iob_data_r(self.grid.kind, self.grid.rows[row]);
            if tidx == 0 {
                let coords: Vec<_> = (0..data.tiles).map(|dx| (col, row + dx)).collect();
                self.die.add_xnode((col, row), data.node, &coords);
            }
        }
    }

    fn fill_io_b(&mut self) {
        for (col, &cd) in &self.grid.columns {
            let row = self.grid.row_bot();
            if self.grid.kind.is_spartan3ea() {
                if cd.kind == ColumnKind::Io {
                    continue;
                }
            } else {
                if cd.kind != ColumnKind::Clb {
                    continue;
                }
            }

            let (int_kind, ioi_kind) = match self.grid.kind {
                GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
                    if cd.io == ColumnIoKind::DoubleRightClk(1) {
                        ("INT.IOI.CLK_B", "IOI.CLK_B")
                    } else {
                        ("INT.IOI", "IOI")
                    }
                }
                GridKind::Spartan3 => ("INT.IOI.S3", "IOI.S3"),
                GridKind::FpgaCore => ("INT.IOI.FC", "IOI.FC"),
                GridKind::Spartan3E => ("INT.IOI.S3E", "IOI.S3E"),
                GridKind::Spartan3A | GridKind::Spartan3ADsp => ("INT.IOI.S3A.TB", "IOI.S3A.B"),
            };
            self.die.fill_tile((col, row), int_kind);
            self.die.add_xnode((col, row), ioi_kind, &[(col, row)]);
            if cd.io != ColumnIoKind::None {
                let (data, tidx) = get_iob_data_b(self.grid.kind, cd.io);
                if tidx == 0 {
                    let coords: Vec<_> = (0..data.tiles).map(|dx| (col + dx, row)).collect();
                    self.die.add_xnode((col, row), data.node, &coords);
                }
            }
            if !self.grid.kind.is_virtex2() && self.grid.kind != GridKind::FpgaCore {
                self.die.add_xnode((col, row), "RANDOR", &[(col, row)]);
            }
        }
    }

    fn fill_io_l(&mut self) {
        for row in self.grid.rows.ids() {
            let col = self.grid.col_left();
            if row == self.grid.row_bot() || row == self.grid.row_top() {
                continue;
            }
            let int_kind;
            let ioi_kind;
            match self.grid.kind {
                GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
                    int_kind = "INT.IOI";
                    ioi_kind = "IOI";
                }
                GridKind::Spartan3 => {
                    int_kind = "INT.IOI.S3";
                    ioi_kind = "IOI.S3";
                }
                GridKind::FpgaCore => {
                    int_kind = "INT.IOI.FC";
                    ioi_kind = "IOI.FC";
                }
                GridKind::Spartan3E => {
                    int_kind = "INT.IOI.S3E";
                    ioi_kind = "IOI.S3E";
                }
                GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                    int_kind = "INT.IOI.S3A.LR";
                    ioi_kind = "IOI.S3A.LR";
                }
            }
            self.die.fill_tile((col, row), int_kind);
            self.die.add_xnode((col, row), ioi_kind, &[(col, row)]);
            let (data, tidx) = get_iob_data_l(self.grid.kind, self.grid.rows[row]);
            if tidx == 0 {
                let coords: Vec<_> = (0..data.tiles).map(|dx| (col, row + dx)).collect();
                self.die.add_xnode((col, row), data.node, &coords);
            }
        }
    }

    fn fill_clb(&mut self) {
        for (col, &cd) in self.grid.columns.iter() {
            if self.grid.kind == GridKind::Spartan3E {
                if cd.kind == ColumnKind::Io {
                    continue;
                }
            } else {
                if cd.kind != ColumnKind::Clb {
                    continue;
                }
            }
            for (row, &io) in self.grid.rows.iter() {
                if io == RowIoKind::None {
                    continue;
                }
                if self.is_hole(col, row) {
                    continue;
                }
                self.die.fill_tile((col, row), "INT.CLB");
                self.die.add_xnode((col, row), "CLB", &[(col, row)]);
            }
        }
    }

    fn fill_bram_dsp(&mut self) {
        let bram_kind = match self.grid.kind {
            GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => ["INT.BRAM"; 4],
            GridKind::Spartan3 => ["INT.BRAM.S3"; 4],
            GridKind::FpgaCore => return,
            GridKind::Spartan3E => ["INT.BRAM.S3E"; 4],
            GridKind::Spartan3A => [
                "INT.BRAM.S3A.03",
                "INT.BRAM.S3A.12",
                "INT.BRAM.S3A.12",
                "INT.BRAM.S3A.03",
            ],
            GridKind::Spartan3ADsp => ["INT.BRAM.S3ADSP"; 4],
        };
        for (col, &cd) in self.grid.columns.iter() {
            if cd.kind != ColumnKind::Bram {
                continue;
            }
            if let Some((b, t)) = self.grid.rows_ram {
                self.holes.push(Rect {
                    col_l: col,
                    col_r: col + 4,
                    row_b: b,
                    row_t: t + 1,
                });
                for d in 1..4 {
                    self.die.fill_term_pair(
                        (col + d, b - 1),
                        (col + d, t + 1),
                        "TERM.BRAM.N",
                        "TERM.BRAM.S",
                    );
                }
            }
            for row in self.grid.rows.ids() {
                if self.grid.kind != GridKind::Spartan3E && self.is_hole(col, row) {
                    continue;
                }
                let Some(idx) = self.grid.bram_row(row) else {
                    continue;
                };
                self.die.fill_tile((col, row), bram_kind[idx]);
                if self.grid.kind == GridKind::Spartan3ADsp {
                    self.die.fill_tile((col + 3, row), "INT.BRAM.S3ADSP");
                }
                if idx == 0 {
                    let kind = match self.grid.kind {
                        GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => "BRAM",
                        GridKind::Spartan3 => "BRAM.S3",
                        GridKind::FpgaCore => unreachable!(),
                        GridKind::Spartan3E => "BRAM.S3E",
                        GridKind::Spartan3A => "BRAM.S3A",
                        GridKind::Spartan3ADsp => "BRAM.S3ADSP",
                    };
                    self.die.add_xnode(
                        (col, row),
                        kind,
                        &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                    );
                    if self.grid.kind == GridKind::Spartan3ADsp {
                        self.die.add_xnode(
                            (col + 3, row),
                            "DSP",
                            &[
                                (col + 3, row),
                                (col + 3, row + 1),
                                (col + 3, row + 2),
                                (col + 3, row + 3),
                            ],
                        );
                        self.die.add_xnode(
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
        if self.grid.kind.is_spartan3ea() {
            for pair in self.grid.get_dcm_pairs() {
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
                        self.die.add_xnode(crd, "DCM.S3E.BL", &[crd]);
                        let crd = (pair.col, pair.row);
                        self.die.fill_tile(crd, "INT.DCM");
                        self.die.add_xnode(crd, "DCM.S3E.BR", &[crd]);
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
                        self.die.add_xnode(crd, "DCM.S3E.BR", &[crd]);
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
                        self.die.add_xnode(crd, "DCM.S3E.TL", &[crd]);
                        let crd = (pair.col, pair.row);
                        self.die.fill_tile(crd, "INT.DCM");
                        self.die.add_xnode(crd, "DCM.S3E.TR", &[crd]);
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
                        self.die.add_xnode(crd, "DCM.S3E.TR", &[crd]);
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
                        self.die.add_xnode(crd, "DCM.S3E.LB", &[crd]);
                        let crd = (pair.col, pair.row);
                        self.die.fill_tile(crd, "INT.DCM");
                        self.die.add_xnode(crd, "DCM.S3E.LT", &[crd]);
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
                        self.die.add_xnode(crd, "DCM.S3E.RB", &[crd]);
                        let crd = (pair.col, pair.row);
                        self.die.fill_tile(crd, "INT.DCM");
                        self.die.add_xnode(crd, "DCM.S3E.RT", &[crd]);
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
                        self.die.add_xnode(crd, "DCM.S3E.LB", &[crd]);
                        let crd = (pair.col, pair.row);
                        self.die.fill_tile(crd, "INT.DCM");
                        self.die.add_xnode(crd, "DCM.S3E.LT", &[crd]);
                    }
                }
            }
        } else {
            let row_b = self.grid.row_bot();
            let row_t = self.grid.row_top();
            for (col, &cd) in self.grid.columns.iter() {
                if cd.kind != ColumnKind::Bram {
                    continue;
                }
                if !self.grid.cols_gt.contains_key(&col) {
                    let (kind, dcm) = match self.grid.kind {
                        GridKind::Virtex2 => ("INT.DCM.V2", "DCM.V2"),
                        GridKind::Virtex2P | GridKind::Virtex2PX => ("INT.DCM.V2P", "DCM.V2P"),
                        GridKind::Spartan3 => {
                            if col == self.grid.col_left() + 3 || col == self.grid.col_right() - 3 {
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
                        self.die.add_xnode((col, row_b), dcm, &[(col, row_b)]);
                        self.die.add_xnode((col, row_t), dcm, &[(col, row_t)]);
                    }
                }
                self.die
                    .add_xnode((col, row_b), "DCMCONN.BOT", &[(col, row_b)]);
                self.die
                    .add_xnode((col, row_t), "DCMCONN.TOP", &[(col, row_t)]);
            }
        }
    }

    fn fill_ppc(&mut self) {
        for &(bc, br) in &self.grid.holes_ppc {
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
                self.die.add_xnode(crd, "INTF.PPC", &[crd]);
            }
            // horiz passes
            for d in 1..15 {
                let col_l = bc;
                let col_r = bc + 9;
                let row = br + d;
                self.die
                    .add_xnode((col_l, row), "PPC.E", &[(col_l, row), (col_r, row)]);
                self.die
                    .add_xnode((col_r, row), "PPC.W", &[(col_r, row), (col_l, row)]);
                self.die
                    .fill_term_pair((col_l, row), (col_r, row), "PPC.E", "PPC.W");
            }
            // vert passes
            for d in 1..9 {
                let col = bc + d;
                let row_b = br;
                let row_t = br + 15;
                self.die
                    .add_xnode((col, row_b), "PPC.N", &[(col, row_b), (col, row_t)]);
                self.die
                    .add_xnode((col, row_t), "PPC.S", &[(col, row_t), (col, row_b)]);
                self.die
                    .fill_term_pair((col, row_b), (col, row_t), "PPC.N", "PPC.S");
            }
            let kind = if bc < self.grid.col_clk {
                "LBPPC"
            } else {
                "RBPPC"
            };
            self.die.add_xnode((bc, br), kind, &ints);
        }
    }

    fn fill_gt(&mut self) {
        let row_b = self.grid.row_bot();
        let row_t = self.grid.row_top();
        for col in self.grid.cols_gt.keys().copied() {
            if self.grid.kind == GridKind::Virtex2PX {
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
                self.die.add_xnode(
                    (col, row),
                    if row == row_b {
                        "INTF.GT.BCLKPAD"
                    } else {
                        "INTF.GT.TCLKPAD"
                    },
                    &[(col, row)],
                );
            }
            let n = match self.grid.kind {
                GridKind::Virtex2P => 4,
                GridKind::Virtex2PX => 8,
                _ => unreachable!(),
            };
            for br in [row_b + 1, row_t - n] {
                for d in 0..n {
                    let row = br + d;
                    self.die.fill_tile((col, row), "INT.PPC");
                    self.die.add_xnode(
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
            if self.grid.kind == GridKind::Virtex2P {
                self.die.add_xnode(
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
                self.die.add_xnode(
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
                self.die.add_xnode(
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
                self.die.add_xnode(
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
        for col in self.grid.columns.ids() {
            if matches!(self.grid.columns[col].kind, ColumnKind::BramCont(_)) {
                continue;
            }
            let mut row_s = self.grid.row_mid() - 1;
            let mut row_n = self.grid.row_mid();
            while self.die[(col, row_s)].nodes.is_empty() {
                row_s -= 1;
            }
            while self.die[(col, row_n)].nodes.is_empty() {
                row_n += 1;
            }
            let mut term_s = "LLV.S";
            let mut term_n = "LLV.N";
            if (col == self.grid.col_left() || col == self.grid.col_right())
                && self.grid.kind != GridKind::Spartan3A
            {
                term_s = "LLV.CLKLR.S3E.S";
                term_n = "LLV.CLKLR.S3E.N";
            }
            self.die
                .fill_term_pair((col, row_s), (col, row_n), term_n, term_s);
            self.die.add_xnode(
                (col, row_n),
                if self.grid.kind.is_spartan3a() {
                    "LLV.S3A"
                } else {
                    "LLV.S3E"
                },
                &[(col, row_s), (col, row_n)],
            );
        }
    }

    fn fill_llh(&mut self) {
        let row_b = self.grid.row_bot();
        let row_t = self.grid.row_top();
        for row in self.grid.rows.ids() {
            let mut col_l = self.grid.col_clk - 1;
            let mut col_r = self.grid.col_clk;
            while self.die[(col_l, row)].nodes.is_empty() {
                col_l -= 1;
            }
            while self.die[(col_r, row)].nodes.is_empty() {
                col_r += 1;
            }
            let mut term_w = "LLH.W";
            let mut term_e = "LLH.E";
            if self.grid.kind == GridKind::Spartan3ADsp
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
                .fill_term_pair((col_l, row), (col_r, row), term_e, term_w);
            self.die.add_xnode(
                (col_r, row),
                if self.grid.kind.is_spartan3a() && row == self.grid.row_bot() {
                    "LLH.CLKB.S3A"
                } else if self.grid.kind.is_spartan3a() && row == self.grid.row_top() {
                    "LLH.CLKT.S3A"
                } else {
                    "LLH"
                },
                &[(col_l, row), (col_r, row)],
            );
        }
    }

    fn fill_misc_passes(&mut self) {
        if self.grid.kind == GridKind::Spartan3E && !self.grid.has_ll {
            for col in [self.grid.col_left(), self.grid.col_right()] {
                self.die.fill_term_pair(
                    (col, self.grid.row_mid() - 1),
                    (col, self.grid.row_mid()),
                    "CLKLR.S3E.N",
                    "CLKLR.S3E.S",
                );
            }
        }
        if self.grid.kind == GridKind::Spartan3 {
            for &(_, _, row_n) in &self.grid.rows_hclk {
                let row_s = row_n - 1;
                if row_n == self.grid.row_mid() {
                    continue;
                }
                if row_s == self.grid.row_top() {
                    continue;
                }
                for col in self.die.cols() {
                    self.die
                        .fill_term_pair((col, row_s), (col, row_n), "BRKH.S3.N", "BRKH.S3.S");
                }
            }
        }
        if self.grid.kind == GridKind::Spartan3ADsp {
            for (col, cd) in &self.grid.columns {
                if cd.kind == ColumnKind::Dsp {
                    for row in [self.grid.row_bot(), self.grid.row_top()] {
                        self.die.fill_term_pair(
                            (col, row),
                            (col + 1, row),
                            "DSPHOLE.E",
                            "DSPHOLE.W",
                        );
                    }
                }
            }
            for col in [self.grid.col_left() + 3, self.grid.col_right() - 6] {
                for row in [self.grid.row_mid() - 1, self.grid.row_mid()] {
                    self.die
                        .fill_term_pair((col, row), (col + 4, row), "DSPHOLE.E", "DSPHOLE.W");
                }
                for row in [
                    self.grid.row_mid() - 4,
                    self.grid.row_mid() - 3,
                    self.grid.row_mid() - 2,
                    self.grid.row_mid() + 1,
                    self.grid.row_mid() + 2,
                    self.grid.row_mid() + 3,
                ] {
                    self.die
                        .fill_term_pair((col - 1, row), (col + 4, row), "HDCM.E", "HDCM.W");
                }
            }
        }
    }

    fn fill_bram_passes(&mut self) {
        if matches!(self.grid.kind, GridKind::Spartan3A | GridKind::Spartan3ADsp) {
            for (col, cd) in &self.grid.columns {
                if matches!(cd.kind, ColumnKind::BramCont(_)) {
                    self.die[(col, self.grid.row_bot())].terms[Dir::N] = None;
                    self.die[(col, self.grid.row_top())].terms[Dir::S] = None;
                }
            }
        }
    }

    fn fill_clkbt_v2(&mut self) {
        let row_b = self.grid.row_bot();
        let row_t = self.grid.row_top();
        let (kind_b, kind_t) = match self.grid.kind {
            GridKind::Virtex2 => ("CLKB.V2", "CLKT.V2"),
            GridKind::Virtex2P => ("CLKB.V2P", "CLKT.V2P"),
            GridKind::Virtex2PX => ("CLKB.V2PX", "CLKT.V2PX"),
            _ => unreachable!(),
        };
        self.die.add_xnode(
            (self.grid.col_clk, row_b),
            kind_b,
            &[(self.grid.col_clk - 1, row_b), (self.grid.col_clk, row_b)],
        );
        self.die.add_xnode(
            (self.grid.col_clk, row_t),
            kind_t,
            &[(self.grid.col_clk - 1, row_t), (self.grid.col_clk, row_t)],
        );

        let col_l = self.grid.col_left();
        self.die.add_xnode(
            (col_l, self.grid.row_pci.unwrap()),
            "REG_L",
            &[
                (col_l, self.grid.row_pci.unwrap() - 2),
                (col_l, self.grid.row_pci.unwrap() - 1),
                (col_l, self.grid.row_pci.unwrap()),
                (col_l, self.grid.row_pci.unwrap() + 1),
            ],
        );
        let col_r = self.grid.col_right();
        self.die.add_xnode(
            (col_r, self.grid.row_pci.unwrap()),
            "REG_R",
            &[
                (col_r, self.grid.row_pci.unwrap() - 2),
                (col_r, self.grid.row_pci.unwrap() - 1),
                (col_r, self.grid.row_pci.unwrap()),
                (col_r, self.grid.row_pci.unwrap() + 1),
            ],
        );
    }

    fn fill_clkbt_s3(&mut self) {
        let (clkb, clkt) = match self.grid.kind {
            GridKind::Spartan3 => ("CLKB.S3", "CLKT.S3"),
            GridKind::FpgaCore => ("CLKB.FC", "CLKT.FC"),
            _ => unreachable!(),
        };
        let row_b = self.grid.row_bot();
        let row_t = self.grid.row_top();
        self.die.add_xnode(
            (self.grid.col_clk, row_b),
            clkb,
            &[(self.grid.col_clk - 1, row_b)],
        );
        self.die.add_xnode(
            (self.grid.col_clk, row_t),
            clkt,
            &[(self.grid.col_clk - 1, row_t)],
        );
    }

    fn fill_clkbt_s3e(&mut self) {
        self.die.add_xnode(
            (self.grid.col_clk, self.grid.row_bot()),
            if self.grid.kind == GridKind::Spartan3E {
                "CLKB.S3E"
            } else {
                "CLKB.S3A"
            },
            &[(self.grid.col_clk - 1, self.grid.row_bot())],
        );
        self.die.add_xnode(
            (self.grid.col_clk, self.grid.row_top()),
            if self.grid.kind == GridKind::Spartan3E {
                "CLKT.S3E"
            } else {
                "CLKT.S3A"
            },
            &[(self.grid.col_clk - 1, self.grid.row_top())],
        );
    }

    fn fill_clklr_s3e(&mut self) {
        self.die.add_xnode(
            (self.grid.col_left(), self.grid.row_mid()),
            if self.grid.kind == GridKind::Spartan3E {
                "CLKL.S3E"
            } else {
                "CLKL.S3A"
            },
            &[
                (self.grid.col_left(), self.grid.row_mid() - 1),
                (self.grid.col_left(), self.grid.row_mid()),
            ],
        );
        self.die.add_xnode(
            (self.grid.col_right(), self.grid.row_mid()),
            if self.grid.kind == GridKind::Spartan3E {
                "CLKR.S3E"
            } else {
                "CLKR.S3A"
            },
            &[
                (self.grid.col_right(), self.grid.row_mid() - 1),
                (self.grid.col_right(), self.grid.row_mid()),
            ],
        );
    }

    fn fill_pci_ce(&mut self) {
        if self.grid.kind.is_spartan3ea() {
            for c in [
                (self.grid.col_left(), self.grid.row_bot()),
                (self.grid.col_right(), self.grid.row_bot()),
                (self.grid.col_left(), self.grid.row_top()),
                (self.grid.col_right(), self.grid.row_top()),
            ] {
                self.die.add_xnode(c, "PCI_CE_CNR", &[]);
            }

            for &(row, _, _) in &self.grid.rows_hclk {
                let kind = if row > self.grid.row_mid() {
                    "PCI_CE_N"
                } else {
                    "PCI_CE_S"
                };
                for col in [self.grid.col_left(), self.grid.col_right()] {
                    self.die.add_xnode((col, row), kind, &[]);
                }
            }
            if self.grid.kind == GridKind::Spartan3A {
                if let Some((col_l, col_r)) = self.grid.cols_clkv {
                    for row in [self.grid.row_bot(), self.grid.row_top()] {
                        self.die.add_xnode((col_l, row), "PCI_CE_E", &[]);
                        self.die.add_xnode((col_r, row), "PCI_CE_W", &[]);
                    }
                }
            }
        }
    }

    fn fill_gclkh(&mut self) {
        for col in self.die.cols() {
            for (i, &(row_m, row_b, row_t)) in self.grid.rows_hclk.iter().enumerate() {
                for r in row_b.to_idx()..row_m.to_idx() {
                    let row = RowId::from_idx(r);
                    self.die[(col, row)].clkroot = (col, row_m - 1);
                }
                for r in row_m.to_idx()..row_t.to_idx() {
                    let row = RowId::from_idx(r);
                    self.die[(col, row)].clkroot = (col, row_m);
                }
                let kind = if matches!(self.grid.columns[col].kind, ColumnKind::BramCont(_)) {
                    if row_m == self.grid.row_mid() {
                        "GCLKH.UNI"
                    } else if i == 0 {
                        if self.grid.kind == GridKind::Spartan3E {
                            "GCLKH.S"
                        } else {
                            "GCLKH.UNI.S"
                        }
                    } else if i == self.grid.rows_hclk.len() - 1 {
                        if self.grid.kind == GridKind::Spartan3E {
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
                    .add_xnode((col, row_m), kind, &[(col, row_m - 1), (col, row_m)]);
                if self.grid.columns[col].kind == ColumnKind::Dsp {
                    self.die.add_xnode((col, row_m), "GCLKH.DSP", &[]);
                }
            }
        }
    }

    fn fill_gclkc(&mut self) {
        for &(row_m, _, _) in &self.grid.rows_hclk {
            if self.grid.kind.is_virtex2() {
                let node_kind = if row_m == self.grid.row_bot() + 1 {
                    "GCLKC.B"
                } else if row_m == self.grid.row_top() {
                    "GCLKC.T"
                } else {
                    "GCLKC"
                };
                self.die
                    .add_xnode((self.grid.col_clk, row_m), node_kind, &[]);
            } else if let Some((col_cl, col_cr)) = self.grid.cols_clkv {
                for col in [col_cl, col_cr] {
                    self.die.add_xnode((col, row_m), "GCLKVC", &[]);
                }
            }
        }
    }

    fn fill_clkc(&mut self) {
        let kind = if !self.grid.kind.is_virtex2() && self.grid.cols_clkv.is_none() {
            "CLKC_50A"
        } else {
            "CLKC"
        };
        self.die
            .add_xnode((self.grid.col_clk, self.grid.row_mid()), kind, &[]);
    }

    fn fill_gclkvm(&mut self) {
        if let Some((col_cl, col_cr)) = self.grid.cols_clkv {
            if matches!(self.grid.kind, GridKind::Spartan3 | GridKind::FpgaCore) {
                self.die
                    .add_xnode((col_cl, self.grid.row_mid()), "GCLKVM.S3", &[]);
                self.die
                    .add_xnode((col_cr, self.grid.row_mid()), "GCLKVM.S3", &[]);
            } else {
                self.die
                    .add_xnode((col_cl, self.grid.row_mid()), "GCLKVM.S3E", &[]);
                self.die
                    .add_xnode((col_cr, self.grid.row_mid()), "GCLKVM.S3E", &[]);
            }
        }
    }

    fn fill_frame_info(&mut self) {
        let mut major = 0;
        // spine
        self.clkv_frame = 0;
        let num_spine = if self.grid.kind.is_virtex2() {
            self.spine_frame = 0;
            4
        } else if self.grid.cols_clkv.is_none() {
            self.spine_frame = 0;
            2
        } else if self.grid.has_ll || self.grid.kind.is_spartan3a() {
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
        let num_term = if self.grid.kind.is_virtex2() { 4 } else { 2 };
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
        let num_main = if self.grid.kind.is_virtex2() { 22 } else { 19 };
        for (_, cd) in &self.grid.columns {
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
        let num_bram = if self.grid.kind.is_virtex2() { 64 } else { 76 };
        for (col, cd) in &self.grid.columns {
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
        for (col, cd) in &self.grid.columns {
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

        for (col, cd) in &self.grid.columns {
            if let ColumnKind::BramCont(i) = cd.kind {
                self.col_frame[col] = self.bram_frame[col - (i as usize)] + (i as usize - 1) * 19;
            }
        }
    }
}

impl Grid {
    fn fill_term(&self, die: &mut ExpandedDieRefMut, coord: Coord, kind: &str) {
        if self.kind.is_virtex2() {
            die.add_xnode(coord, kind, &[coord]);
        }
        die.fill_term(coord, kind);
    }

    pub fn expand_grid<'a>(&'a self, db: &'a IntDb) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);

        let (_, die) = egrid.add_die(self.columns.len(), self.rows.len());

        let mut expander = Expander {
            grid: self,
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
        } else if matches!(self.kind, GridKind::Spartan3 | GridKind::FpgaCore) {
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
            grid: self,
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
