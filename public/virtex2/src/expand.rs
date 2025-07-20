use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::dir::DirHV;
use prjcombine_interconnect::grid::{CellCoord, ColId, DieId, ExpandedGrid, Rect};
use prjcombine_xilinx_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::chip::{Chip, ChipKind, ColumnIoKind, ColumnKind, DcmPairKind};
use crate::expanded::{ExpandedDevice, REGION_HCLK, REGION_LEAF};
use crate::iob::{get_iob_data_e, get_iob_data_n, get_iob_data_s, get_iob_data_w};
use crate::tslots;

struct Expander<'a, 'b> {
    chip: &'b Chip,
    die: DieId,
    egrid: &'a mut ExpandedGrid<'b>,
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
    fn is_hole(&self, cell: CellCoord) -> bool {
        for hole in &self.holes {
            if hole.contains(cell) {
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

        self.egrid
            .add_tile_single(self.chip.corner(DirHV::SW).cell, cnr_kind);
        self.egrid
            .add_tile_single(self.chip.corner(DirHV::SE).cell, cnr_kind);
        self.egrid
            .add_tile_single(self.chip.corner(DirHV::NW).cell, cnr_kind);
        self.egrid
            .add_tile_single(self.chip.corner(DirHV::NE).cell, cnr_kind);
        self.egrid
            .add_tile_single(self.chip.corner(DirHV::SW).cell, ll);
        self.egrid
            .add_tile_single(self.chip.corner(DirHV::SE).cell, lr);
        self.egrid
            .add_tile_single(self.chip.corner(DirHV::NW).cell, ul);
        self.egrid
            .add_tile_single(self.chip.corner(DirHV::NE).cell, ur);

        if !self.chip.kind.is_virtex2() {
            self.egrid
                .add_tile(self.chip.corner(DirHV::NW).cell, "RANDOR_INIT", &[]);
        }
    }

    fn fill_term(&mut self) {
        for cell in self.egrid.row(self.die, self.chip.row_s()) {
            if self.chip.kind.is_virtex2() {
                self.egrid.add_tile_single(cell, "TERM.S");
            }
            self.egrid.fill_conn_term(cell, "TERM.S");
        }
        for cell in self.egrid.row(self.die, self.chip.row_n()) {
            if self.chip.kind.is_virtex2() {
                self.egrid.add_tile_single(cell, "TERM.N");
            }
            self.egrid.fill_conn_term(cell, "TERM.N");
        }
        for cell in self.egrid.column(self.die, self.chip.col_w()) {
            if self.chip.kind.is_virtex2() {
                self.egrid.add_tile_single(cell, "TERM.W");
            }
            self.egrid.fill_conn_term(cell, "TERM.W");
        }
        for cell in self.egrid.column(self.die, self.chip.col_e()) {
            if self.chip.kind.is_virtex2() {
                self.egrid.add_tile_single(cell, "TERM.E");
            }
            self.egrid.fill_conn_term(cell, "TERM.E");
        }
    }

    fn fill_io_n(&mut self) {
        for cell in self.egrid.row(self.die, self.chip.row_n()) {
            let cd = &self.chip.columns[cell.col];
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
            self.egrid.add_tile_single(cell, int_kind);
            self.egrid.add_tile_single(cell, ioi_kind);
            if cd.io != ColumnIoKind::None {
                let (data, tidx) = get_iob_data_n(self.chip.kind, cd.io);
                if tidx.to_idx() == 0 {
                    self.egrid.add_tile_e(cell, data.node, data.tiles);
                }
            }
            if !self.chip.kind.is_virtex2() {
                self.egrid.add_tile(cell, "RANDOR", &[]);
            }
        }
    }

    fn fill_io_e(&mut self) {
        for cell in self.egrid.column(self.die, self.chip.col_e()) {
            if self.chip.is_row_io(cell.row) {
                continue;
            }
            let (int_kind, ioi_kind) = match self.chip.kind {
                ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => ("INT.IOI", "IOI"),
                ChipKind::Spartan3 => ("INT.IOI.S3", "IOI.S3"),
                ChipKind::FpgaCore => ("INT.IOI.FC", "IOI.FC"),
                ChipKind::Spartan3E => ("INT.IOI.S3E", "IOI.S3E"),
                ChipKind::Spartan3A | ChipKind::Spartan3ADsp => ("INT.IOI.S3A.LR", "IOI.S3A.LR"),
            };
            self.egrid.add_tile_single(cell, int_kind);
            self.egrid.add_tile_single(cell, ioi_kind);
            let (data, tidx) = get_iob_data_e(self.chip.kind, self.chip.rows[cell.row]);
            if tidx.to_idx() == 0 {
                self.egrid.add_tile_n(cell, data.node, data.tiles);
            }
        }
    }

    fn fill_io_s(&mut self) {
        for cell in self.egrid.row(self.die, self.chip.row_s()) {
            let cd = &self.chip.columns[cell.col];
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
            self.egrid.add_tile_single(cell, int_kind);
            self.egrid.add_tile_single(cell, ioi_kind);
            if cd.io != ColumnIoKind::None {
                let (data, tidx) = get_iob_data_s(self.chip.kind, cd.io);
                if tidx.to_idx() == 0 {
                    self.egrid.add_tile_e(cell, data.node, data.tiles);
                }
            }
            if !self.chip.kind.is_virtex2() && self.chip.kind != ChipKind::FpgaCore {
                self.egrid.add_tile(cell, "RANDOR", &[]);
            }
        }
    }

    fn fill_io_w(&mut self) {
        for cell in self.egrid.column(self.die, self.chip.col_w()) {
            if self.chip.is_row_io(cell.row) {
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
            self.egrid.add_tile_single(cell, int_kind);
            self.egrid.add_tile_single(cell, ioi_kind);
            let (data, tidx) = get_iob_data_w(self.chip.kind, self.chip.rows[cell.row]);
            if tidx.to_idx() == 0 {
                self.egrid.add_tile_n(cell, data.node, data.tiles);
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
            for cell in self.egrid.column(self.die, col) {
                if self.chip.is_row_io(cell.row) {
                    continue;
                }
                if self.is_hole(cell) {
                    continue;
                }
                self.egrid.add_tile_single(cell, "INT.CLB");
                self.egrid.add_tile_single(cell, "CLB");
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
                    die: self.die,
                    col_w: col,
                    col_e: col + 4,
                    row_s: b,
                    row_n: t + 1,
                });
                for d in 1..4 {
                    self.egrid.fill_conn_pair(
                        CellCoord::new(self.die, col + d, b - 1),
                        CellCoord::new(self.die, col + d, t + 1),
                        "TERM.BRAM.N",
                        "TERM.BRAM.S",
                    );
                }
            }
            for cell in self.egrid.column(self.die, col) {
                if self.chip.kind != ChipKind::Spartan3E && self.is_hole(cell) {
                    continue;
                }
                let Some(idx) = self.chip.bram_row(cell.row) else {
                    continue;
                };
                self.egrid.add_tile_single(cell, bram_kind[idx]);
                if self.chip.kind == ChipKind::Spartan3ADsp {
                    self.egrid
                        .add_tile_single(cell.delta(3, 0), "INT.BRAM.S3ADSP");
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
                    self.egrid.add_tile_n(cell, kind, 4);
                    if self.chip.kind == ChipKind::Spartan3ADsp {
                        let cell = cell.delta(3, 0);
                        self.egrid.add_tile_n(cell, "DSP", 4);
                        self.egrid.add_tile_n(cell, "INTF.DSP", 4);
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
                        self.holes.push(pair.cell.delta(-4, 0).rect(8, 4));
                        let cell = pair.cell.delta(-1, 0);
                        self.egrid.add_tile_single(cell, "INT.DCM");
                        self.egrid.add_tile_single(cell, "DCM.S3E.BL");
                        let cell = pair.cell;
                        self.egrid.add_tile_single(cell, "INT.DCM");
                        self.egrid.add_tile_single(cell, "DCM.S3E.BR");
                    }
                    DcmPairKind::BotSingle => {
                        self.holes.push(pair.cell.delta(-1, 0).rect(5, 4));
                        let cell = pair.cell.delta(-1, 0);
                        self.egrid.add_tile_single(cell, "INT.DCM.S3E.DUMMY");
                        let cell = pair.cell;
                        self.egrid.add_tile_single(cell, "INT.DCM");
                        self.egrid.add_tile_single(cell, "DCM.S3E.BR");
                    }
                    DcmPairKind::Top => {
                        self.holes.push(pair.cell.delta(-4, -3).rect(8, 4));
                        let cell = pair.cell.delta(-1, 0);
                        self.egrid.add_tile_single(cell, "INT.DCM");
                        self.egrid.add_tile_single(cell, "DCM.S3E.TL");
                        let cell = pair.cell;
                        self.egrid.add_tile_single(cell, "INT.DCM");
                        self.egrid.add_tile_single(cell, "DCM.S3E.TR");
                    }
                    DcmPairKind::TopSingle => {
                        self.holes.push(pair.cell.delta(-1, -3).rect(5, 4));
                        let cell = pair.cell.delta(-1, 0);
                        self.egrid.add_tile_single(cell, "INT.DCM.S3E.DUMMY");
                        let cell = pair.cell;
                        self.egrid.add_tile_single(cell, "INT.DCM");
                        self.egrid.add_tile_single(cell, "DCM.S3E.TR");
                    }
                    DcmPairKind::Left => {
                        self.holes.push(pair.cell.delta(0, -4).rect(4, 8));
                        let cell = pair.cell.delta(0, -1);
                        self.egrid.add_tile_single(cell, "INT.DCM");
                        self.egrid.add_tile_single(cell, "DCM.S3E.LB");
                        let cell = pair.cell;
                        self.egrid.add_tile_single(cell, "INT.DCM");
                        self.egrid.add_tile_single(cell, "DCM.S3E.LT");
                    }
                    DcmPairKind::Right => {
                        self.holes.push(pair.cell.delta(-3, -4).rect(4, 8));
                        let cell = pair.cell.delta(0, -1);
                        self.egrid.add_tile_single(cell, "INT.DCM");
                        self.egrid.add_tile_single(cell, "DCM.S3E.RB");
                        let cell = pair.cell;
                        self.egrid.add_tile_single(cell, "INT.DCM");
                        self.egrid.add_tile_single(cell, "DCM.S3E.RT");
                    }
                    DcmPairKind::Bram => {
                        self.holes.push(pair.cell.delta(0, -4).rect(4, 8));
                        let cell = pair.cell.delta(0, -1);
                        self.egrid.add_tile_single(cell, "INT.DCM");
                        self.egrid.add_tile_single(cell, "DCM.S3E.LB");
                        let cell = pair.cell;
                        self.egrid.add_tile_single(cell, "INT.DCM");
                        self.egrid.add_tile_single(cell, "DCM.S3E.LT");
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
                    for row in [self.chip.row_s(), self.chip.row_n()] {
                        let cell = CellCoord::new(self.die, col, row);
                        self.egrid.add_tile_single(cell, kind);
                        if !dcm.is_empty() {
                            self.egrid.add_tile_single(cell, dcm);
                        }
                    }
                }
                self.egrid
                    .add_tile_single(CellCoord::new(self.die, col, row_b), "DCMCONN.BOT");
                self.egrid
                    .add_tile_single(CellCoord::new(self.die, col, row_t), "DCMCONN.TOP");
            }
        }
    }

    fn fill_ppc(&mut self) {
        for &(bc, br) in &self.chip.holes_ppc {
            let cell = CellCoord::new(self.die, bc, br);
            self.holes.push(cell.rect(10, 16));

            let mut tcells = vec![];
            // left side
            tcells.extend(cell.cells_n_const::<16>());
            // right side
            tcells.extend(cell.delta(9, 0).cells_n_const::<16>());
            // bottom
            tcells.extend(cell.delta(1, 0).cells_e_const::<8>());
            // top
            tcells.extend(cell.delta(1, 15).cells_e_const::<8>());

            for &cell in &tcells {
                self.egrid.add_tile_single(cell, "INT.PPC");
                self.egrid.add_tile_single(cell, "INTF.PPC");
            }
            // horiz passes
            for d in 1..15 {
                let cell_w = cell.delta(0, d);
                let cell_e = cell.delta(9, d);
                self.egrid.add_tile(cell_w, "PPC.E", &[cell_w, cell_e]);
                self.egrid.add_tile(cell_e, "PPC.W", &[cell_e, cell_w]);
                self.egrid.fill_conn_pair(cell_w, cell_e, "PPC.E", "PPC.W");
            }
            // vert passes
            for d in 1..9 {
                let cell_s = cell.delta(d, 0);
                let cell_n = cell.delta(d, 15);
                self.egrid.add_tile(cell_s, "PPC.N", &[cell_s, cell_n]);
                self.egrid.add_tile(cell_n, "PPC.S", &[cell_n, cell_s]);
                self.egrid.fill_conn_pair(cell_s, cell_n, "PPC.N", "PPC.S");
            }
            let kind = if bc < self.chip.col_clk {
                "LBPPC"
            } else {
                "RBPPC"
            };
            self.egrid.add_tile(cell, kind, &tcells);
        }
    }

    fn fill_gt(&mut self) {
        for col in self.chip.cols_gt.keys().copied() {
            let cell_s = CellCoord::new(self.die, col, self.chip.row_s());
            let cell_n = CellCoord::new(self.die, col, self.chip.row_n());
            if self.chip.kind == ChipKind::Virtex2PX {
                self.holes.push(cell_s.rect(1, 9));
                self.holes.push(cell_n.delta(0, -8).rect(1, 9));
            } else {
                self.holes.push(cell_s.rect(1, 5));
                self.holes.push(cell_n.delta(0, -4).rect(1, 5));
            }
            self.egrid.add_tile_single(cell_s, "INT.GT.CLKPAD");
            self.egrid.add_tile_single(cell_n, "INT.GT.CLKPAD");
            self.egrid.add_tile_single(cell_s, "INTF.GT.BCLKPAD");
            self.egrid.add_tile_single(cell_n, "INTF.GT.TCLKPAD");
            let n = match self.chip.kind {
                ChipKind::Virtex2P => 4,
                ChipKind::Virtex2PX => 8,
                _ => unreachable!(),
            };
            for d in 0..n {
                let cell = cell_s.delta(0, 1 + d);
                self.egrid.add_tile_single(cell, "INT.PPC");
                self.egrid.add_tile_single(
                    cell,
                    if d % 4 == 0 {
                        "INTF.GT.B0"
                    } else {
                        "INTF.GT.B123"
                    },
                );
            }
            for d in 0..n {
                let cell = cell_n.delta(0, -n + d);
                self.egrid.add_tile_single(cell, "INT.PPC");
                self.egrid.add_tile_single(
                    cell,
                    if d % 4 == 0 {
                        "INTF.GT.T0"
                    } else {
                        "INTF.GT.T123"
                    },
                );
            }
            if self.chip.kind == ChipKind::Virtex2P {
                self.egrid.add_tile(
                    cell_s,
                    "GIGABIT.B",
                    &[
                        cell_s,
                        cell_s.delta(0, 1),
                        cell_s.delta(0, 2),
                        cell_s.delta(0, 3),
                        cell_s.delta(0, 4),
                    ],
                );
                self.egrid.add_tile(
                    cell_n,
                    "GIGABIT.T",
                    &[
                        cell_n,
                        cell_n.delta(0, -4),
                        cell_n.delta(0, -3),
                        cell_n.delta(0, -2),
                        cell_n.delta(0, -1),
                    ],
                );
            } else {
                self.egrid.add_tile(
                    cell_s,
                    "GIGABIT10.B",
                    &[
                        cell_s,
                        cell_s.delta(0, 1),
                        cell_s.delta(0, 2),
                        cell_s.delta(0, 3),
                        cell_s.delta(0, 4),
                        cell_s.delta(0, 5),
                        cell_s.delta(0, 6),
                        cell_s.delta(0, 7),
                        cell_s.delta(0, 8),
                    ],
                );
                self.egrid.add_tile(
                    cell_n,
                    "GIGABIT10.T",
                    &[
                        cell_n,
                        cell_n.delta(0, -8),
                        cell_n.delta(0, -7),
                        cell_n.delta(0, -6),
                        cell_n.delta(0, -5),
                        cell_n.delta(0, -4),
                        cell_n.delta(0, -3),
                        cell_n.delta(0, -2),
                        cell_n.delta(0, -1),
                    ],
                );
            }
        }
    }

    fn fill_llv(&mut self) {
        for cell in self.egrid.row(self.die, self.chip.row_mid()) {
            if matches!(self.chip.columns[cell.col].kind, ColumnKind::BramCont(_)) {
                continue;
            }
            let mut cell_s = cell.delta(0, -1);
            let mut cell_n = cell;
            while !self.egrid[cell_s].tiles.contains_id(tslots::INT) {
                cell_s.row -= 1;
            }
            while !self.egrid[cell_n].tiles.contains_id(tslots::INT) {
                cell_n.row += 1;
            }
            let mut term_s = "LLV.S";
            let mut term_n = "LLV.N";
            if self.chip.is_col_io(cell.col) && self.chip.kind != ChipKind::Spartan3A {
                term_s = "LLV.CLKLR.S3E.S";
                term_n = "LLV.CLKLR.S3E.N";
            }
            self.egrid.fill_conn_pair(cell_s, cell_n, term_n, term_s);
            self.egrid.add_tile(
                cell_n,
                if self.chip.kind.is_spartan3a() {
                    "LLV.S3A"
                } else {
                    "LLV.S3E"
                },
                &[cell_s, cell_n],
            );
        }
    }

    fn fill_llh(&mut self) {
        for cell in self.egrid.column(self.die, self.chip.col_clk) {
            let mut cell_w = cell.delta(-1, 0);
            let mut cell_e = cell;
            while !self.egrid[cell_w].tiles.contains_id(tslots::INT) {
                cell_w.col -= 1;
            }
            while !self.egrid[cell_e].tiles.contains_id(tslots::INT) {
                cell_e.col += 1;
            }
            let mut term_w = "LLH.W";
            let mut term_e = "LLH.E";
            if self.chip.kind == ChipKind::Spartan3ADsp
                && [
                    self.chip.row_s() + 2,
                    self.chip.row_s() + 3,
                    self.chip.row_s() + 4,
                    self.chip.row_n() - 4,
                    self.chip.row_n() - 3,
                    self.chip.row_n() - 2,
                ]
                .into_iter()
                .any(|x| x == cell.row)
            {
                term_w = "LLH.DCM.S3ADSP.W";
                term_e = "LLH.DCM.S3ADSP.E";
            }
            self.egrid.fill_conn_pair(cell_w, cell_e, term_e, term_w);
            self.egrid.add_tile(
                cell_e,
                if self.chip.kind.is_spartan3a() && cell.row == self.chip.row_s() {
                    "LLH.CLKB.S3A"
                } else if self.chip.kind.is_spartan3a() && cell.row == self.chip.row_n() {
                    "LLH.CLKT.S3A"
                } else {
                    "LLH"
                },
                &[cell_w, cell_e],
            );
        }
    }

    fn fill_misc_passes(&mut self) {
        if self.chip.kind == ChipKind::Spartan3E && !self.chip.has_ll {
            for col in [self.chip.col_w(), self.chip.col_e()] {
                self.egrid.fill_conn_pair(
                    CellCoord::new(self.die, col, self.chip.row_mid() - 1),
                    CellCoord::new(self.die, col, self.chip.row_mid()),
                    "CLKLR.S3E.N",
                    "CLKLR.S3E.S",
                );
            }
        }
        if self.chip.kind == ChipKind::Spartan3 {
            for &(_, _, row) in &self.chip.rows_hclk {
                if row == self.chip.row_mid() {
                    continue;
                }
                if row - 1 == self.chip.row_n() {
                    continue;
                }
                for cell in self.egrid.row(self.die, row) {
                    self.egrid
                        .fill_conn_pair(cell.delta(0, -1), cell, "BRKH.S3.N", "BRKH.S3.S");
                }
            }
        }
        if self.chip.kind == ChipKind::Spartan3ADsp {
            for (col, cd) in &self.chip.columns {
                if cd.kind == ColumnKind::Dsp {
                    for row in [self.chip.row_s(), self.chip.row_n()] {
                        self.egrid.fill_conn_pair(
                            CellCoord::new(self.die, col, row),
                            CellCoord::new(self.die, col + 1, row),
                            "DSPHOLE.E",
                            "DSPHOLE.W",
                        );
                    }
                }
            }
            for col in [self.chip.col_w() + 3, self.chip.col_e() - 6] {
                for row in [self.chip.row_mid() - 1, self.chip.row_mid()] {
                    self.egrid.fill_conn_pair(
                        CellCoord::new(self.die, col, row),
                        CellCoord::new(self.die, col + 4, row),
                        "DSPHOLE.E",
                        "DSPHOLE.W",
                    );
                }
                for row in [
                    self.chip.row_mid() - 4,
                    self.chip.row_mid() - 3,
                    self.chip.row_mid() - 2,
                    self.chip.row_mid() + 1,
                    self.chip.row_mid() + 2,
                    self.chip.row_mid() + 3,
                ] {
                    self.egrid.fill_conn_pair(
                        CellCoord::new(self.die, col - 1, row),
                        CellCoord::new(self.die, col + 4, row),
                        "HDCM.E",
                        "HDCM.W",
                    );
                }
            }
        }
    }

    fn fill_bram_passes(&mut self) {
        if matches!(self.chip.kind, ChipKind::Spartan3A | ChipKind::Spartan3ADsp) {
            let slot_n = self.egrid.db.get_conn_slot("N");
            let slot_s = self.egrid.db.get_conn_slot("S");
            for (col, cd) in &self.chip.columns {
                if matches!(cd.kind, ColumnKind::BramCont(_)) {
                    self.egrid[CellCoord::new(self.die, col, self.chip.row_s())]
                        .conns
                        .remove(slot_n);
                    self.egrid[CellCoord::new(self.die, col, self.chip.row_n())]
                        .conns
                        .remove(slot_s);
                }
            }
        }
    }

    fn fill_clkbt_v2(&mut self) {
        let (kind_b, kind_t) = match self.chip.kind {
            ChipKind::Virtex2 => ("CLKB.V2", "CLKT.V2"),
            ChipKind::Virtex2P => ("CLKB.V2P", "CLKT.V2P"),
            ChipKind::Virtex2PX => ("CLKB.V2PX", "CLKT.V2PX"),
            _ => unreachable!(),
        };
        {
            let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_s());
            self.egrid
                .add_tile(cell, kind_b, &[cell.delta(-1, 0), cell]);
        }
        {
            let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_n());
            self.egrid
                .add_tile(cell, kind_t, &[cell.delta(-1, 0), cell]);
        }

        {
            let cell = CellCoord::new(self.die, self.chip.col_w(), self.chip.row_pci.unwrap());
            self.egrid.add_tile_sn(cell, "REG_L", 2, 4);
        }
        {
            let cell = CellCoord::new(self.die, self.chip.col_e(), self.chip.row_pci.unwrap());
            self.egrid.add_tile_sn(cell, "REG_R", 2, 4);
        }
    }

    fn fill_clkbt_s3(&mut self) {
        let (clkb, clkt) = match self.chip.kind {
            ChipKind::Spartan3 => ("CLKB.S3", "CLKT.S3"),
            ChipKind::FpgaCore => ("CLKB.FC", "CLKT.FC"),
            ChipKind::Spartan3E => ("CLKB.S3E", "CLKT.S3E"),
            ChipKind::Spartan3A | ChipKind::Spartan3ADsp => ("CLKB.S3A", "CLKT.S3A"),
            _ => unreachable!(),
        };
        {
            let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_s());
            self.egrid.add_tile(cell, clkb, &[cell.delta(-1, 0)]);
        }
        {
            let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_n());
            self.egrid.add_tile(cell, clkt, &[cell.delta(-1, 0)]);
        }
    }

    fn fill_clklr_s3e(&mut self) {
        {
            let cell = CellCoord::new(self.die, self.chip.col_w(), self.chip.row_mid());
            self.egrid.add_tile(
                cell,
                if self.chip.kind == ChipKind::Spartan3E {
                    "CLKL.S3E"
                } else {
                    "CLKL.S3A"
                },
                &[cell.delta(0, -1), cell],
            );
        }
        {
            let cell = CellCoord::new(self.die, self.chip.col_e(), self.chip.row_mid());
            self.egrid.add_tile(
                cell,
                if self.chip.kind == ChipKind::Spartan3E {
                    "CLKR.S3E"
                } else {
                    "CLKR.S3A"
                },
                &[cell.delta(0, -1), cell],
            );
        }
    }

    fn fill_pci_ce(&mut self) {
        if self.chip.kind.is_spartan3ea() {
            for hv in DirHV::DIRS {
                self.egrid
                    .add_tile(self.chip.corner(hv).cell, "PCI_CE_CNR", &[]);
            }

            for &(row, _, _) in &self.chip.rows_hclk {
                let kind = if row > self.chip.row_mid() {
                    "PCI_CE_N"
                } else {
                    "PCI_CE_S"
                };
                for col in [self.chip.col_w(), self.chip.col_e()] {
                    let cell = CellCoord::new(self.die, col, row);
                    self.egrid.add_tile(cell, kind, &[]);
                }
            }
            if self.chip.kind == ChipKind::Spartan3A
                && let Some((col_w, col_e)) = self.chip.cols_clkv
            {
                for row in [self.chip.row_s(), self.chip.row_n()] {
                    let cell = CellCoord::new(self.die, col_w, row);
                    self.egrid.add_tile(cell, "PCI_CE_E", &[]);
                    let cell = CellCoord::new(self.die, col_e, row);
                    self.egrid.add_tile(cell, "PCI_CE_W", &[]);
                }
            }
        }
    }

    fn fill_gclkh(&mut self) {
        for col in self.egrid.cols(self.die) {
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
                for row in row_b.range(row_m) {
                    let cell = CellCoord::new(self.die, col, row);
                    self.egrid[cell].region_root[REGION_LEAF] =
                        CellCoord::new(DieId::from_idx(0), col, row_m - 1);
                    self.egrid[cell].region_root[REGION_HCLK] =
                        CellCoord::new(DieId::from_idx(0), col_q, row_q);
                }
                for row in row_m.range(row_t) {
                    let cell = CellCoord::new(self.die, col, row);
                    self.egrid[cell].region_root[REGION_LEAF] =
                        CellCoord::new(DieId::from_idx(0), col, row_m);
                    self.egrid[cell].region_root[REGION_HCLK] =
                        CellCoord::new(DieId::from_idx(0), col_q, row_q);
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
                let cell = CellCoord::new(self.die, col, row_m);
                self.egrid.add_tile(cell, kind, &[cell.delta(0, -1), cell]);
                if self.chip.columns[col].kind == ColumnKind::Dsp {
                    self.egrid.add_tile(cell, "GCLKH.DSP", &[]);
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
                self.egrid.add_tile(
                    CellCoord::new(self.die, self.chip.col_clk, row_m),
                    node_kind,
                    &[],
                );
            } else if let Some((col_cl, col_cr)) = self.chip.cols_clkv {
                for col in [col_cl, col_cr] {
                    self.egrid
                        .add_tile(CellCoord::new(self.die, col, row_m), "GCLKVC", &[]);
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
        self.egrid.add_tile(
            CellCoord::new(self.die, self.chip.col_clk, self.chip.row_mid()),
            kind,
            &[],
        );
    }

    fn fill_gclkvm(&mut self) {
        if let Some((col_cl, col_cr)) = self.chip.cols_clkv {
            if matches!(self.chip.kind, ChipKind::Spartan3 | ChipKind::FpgaCore) {
                self.egrid.add_tile(
                    CellCoord::new(self.die, col_cl, self.chip.row_mid()),
                    "GCLKVM.S3",
                    &[],
                );
                self.egrid.add_tile(
                    CellCoord::new(self.die, col_cr, self.chip.row_mid()),
                    "GCLKVM.S3",
                    &[],
                );
            } else {
                self.egrid.add_tile(
                    CellCoord::new(self.die, col_cl, self.chip.row_mid()),
                    "GCLKVM.S3E",
                    &[],
                );
                self.egrid.add_tile(
                    CellCoord::new(self.die, col_cr, self.chip.row_mid()),
                    "GCLKVM.S3E",
                    &[],
                );
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
    pub fn expand_grid<'a>(&'a self, db: &'a IntDb) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);

        let die = egrid.add_die(self.columns.len(), self.rows.len());

        let mut expander = Expander {
            chip: self,
            die,
            egrid: &mut egrid,
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
        expander.fill_io_n();
        expander.fill_io_e();
        expander.fill_io_s();
        expander.fill_io_w();
        expander.fill_term();
        if self.has_ll {
            expander.fill_llv();
            expander.fill_llh();
        }
        expander.fill_misc_passes();
        expander.egrid.fill_main_passes(expander.die);
        expander.fill_bram_passes();
        if self.kind.is_virtex2() {
            expander.fill_clkbt_v2();
        } else if matches!(self.kind, ChipKind::Spartan3 | ChipKind::FpgaCore) {
            expander.fill_clkbt_s3();
        } else {
            expander.fill_clkbt_s3();
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
            die_order: vec![expander.die],
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
