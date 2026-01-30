use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};
use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::dir::DirHV;
use prjcombine_interconnect::grid::{CellCoord, ColId, DieId, ExpandedGrid, Rect};
use prjcombine_xilinx_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};

use crate::chip::{Chip, ChipKind, ColumnIoKind, ColumnKind, DcmPairKind};
use crate::defs::{
    cslots, rslots,
    spartan3::{ccls as ccls_s3, tcls as tcls_s3},
    tslots,
    virtex2::{ccls as ccls_v2, tcls as tcls_v2},
};
use crate::expanded::ExpandedDevice;
use crate::iob::{get_iob_data_e, get_iob_data_n, get_iob_data_s, get_iob_data_w};

struct Expander<'a, 'b> {
    chip: &'b Chip,
    die: DieId,
    egrid: &'a mut ExpandedGrid<'b>,
    holes: Vec<Rect>,
    frame_info: Vec<FrameInfo>,
    clkv_frame: usize,
    spine_frame: usize,
    term_w_frame: usize,
    term_e_frame: usize,
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
        let tcid_int = if self.chip.kind.is_virtex2() {
            tcls_v2::INT_CNR
        } else if self.chip.kind == ChipKind::FpgaCore {
            tcls_s3::INT_CLB_FC
        } else {
            tcls_s3::INT_CLB
        };
        let (tcid_sw, tcid_se, tcid_nw, tcid_ne) = match self.chip.kind {
            ChipKind::Virtex2 => (
                tcls_v2::CNR_SW_V2,
                tcls_v2::CNR_SE_V2,
                tcls_v2::CNR_NW_V2,
                tcls_v2::CNR_NE_V2,
            ),
            ChipKind::Virtex2P | ChipKind::Virtex2PX => (
                tcls_v2::CNR_SW_V2P,
                tcls_v2::CNR_SE_V2P,
                tcls_v2::CNR_NW_V2P,
                tcls_v2::CNR_NE_V2P,
            ),
            ChipKind::Spartan3 => (
                tcls_s3::CNR_SW_S3,
                tcls_s3::CNR_SE_S3,
                tcls_s3::CNR_NW_S3,
                tcls_s3::CNR_NE_S3,
            ),
            ChipKind::FpgaCore => (
                tcls_s3::CNR_SW_FC,
                tcls_s3::CNR_SE_FC,
                tcls_s3::CNR_NW_FC,
                tcls_s3::CNR_NE_FC,
            ),
            ChipKind::Spartan3E => (
                tcls_s3::CNR_SW_S3E,
                tcls_s3::CNR_SE_S3E,
                tcls_s3::CNR_NW_S3E,
                tcls_s3::CNR_NE_S3E,
            ),
            ChipKind::Spartan3A | ChipKind::Spartan3ADsp => (
                tcls_s3::CNR_SW_S3A,
                tcls_s3::CNR_SE_S3A,
                tcls_s3::CNR_NW_S3A,
                tcls_s3::CNR_NE_S3A,
            ),
        };

        self.egrid
            .add_tile_single_id(self.chip.corner(DirHV::SW).cell, tcid_int);
        self.egrid
            .add_tile_single_id(self.chip.corner(DirHV::SE).cell, tcid_int);
        self.egrid
            .add_tile_single_id(self.chip.corner(DirHV::NW).cell, tcid_int);
        self.egrid
            .add_tile_single_id(self.chip.corner(DirHV::NE).cell, tcid_int);
        self.egrid
            .add_tile_single_id(self.chip.corner(DirHV::SW).cell, tcid_sw);
        self.egrid
            .add_tile_single_id(self.chip.corner(DirHV::SE).cell, tcid_se);
        self.egrid
            .add_tile_single_id(self.chip.corner(DirHV::NW).cell, tcid_nw);
        self.egrid
            .add_tile_single_id(self.chip.corner(DirHV::NE).cell, tcid_ne);

        if !self.chip.kind.is_virtex2() {
            self.egrid.add_tile_id(
                self.chip.corner(DirHV::NW).cell,
                if self.chip.kind == ChipKind::FpgaCore {
                    tcls_s3::RANDOR_INIT_FC
                } else {
                    tcls_s3::RANDOR_INIT
                },
                &[],
            );
        }
    }

    fn fill_term(&mut self) {
        for cell in self.egrid.row(self.die, self.chip.row_s()) {
            if self.chip.kind.is_virtex2() {
                self.egrid.add_tile_single_id(cell, tcls_v2::TERM_S);
                self.egrid.fill_conn_term_id(cell, ccls_v2::TERM_S);
            } else {
                self.egrid.fill_conn_term_id(cell, ccls_s3::TERM_S);
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_n()) {
            if self.chip.kind.is_virtex2() {
                self.egrid.add_tile_single_id(cell, tcls_v2::TERM_N);
                self.egrid.fill_conn_term_id(cell, ccls_v2::TERM_N);
            } else {
                self.egrid.fill_conn_term_id(cell, ccls_s3::TERM_N);
            }
        }
        for cell in self.egrid.column(self.die, self.chip.col_w()) {
            if self.chip.kind.is_virtex2() {
                self.egrid.add_tile_single_id(cell, tcls_v2::TERM_W);
                self.egrid.fill_conn_term_id(cell, ccls_v2::TERM_W);
            } else {
                self.egrid.fill_conn_term_id(cell, ccls_s3::TERM_W);
            }
        }
        for cell in self.egrid.column(self.die, self.chip.col_e()) {
            if self.chip.kind.is_virtex2() {
                self.egrid.add_tile_single_id(cell, tcls_v2::TERM_E);
                self.egrid.fill_conn_term_id(cell, ccls_v2::TERM_E);
            } else {
                self.egrid.fill_conn_term_id(cell, ccls_s3::TERM_E);
            }
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
                    if cd.io == ColumnIoKind::DoubleEClk(1) {
                        (tcls_v2::INT_IOI_CLK_N, tcls_v2::IOI_CLK_N)
                    } else {
                        (tcls_v2::INT_IOI, tcls_v2::IOI)
                    }
                }
                ChipKind::Spartan3 => (tcls_s3::INT_IOI_S3, tcls_s3::IOI_S3),
                ChipKind::FpgaCore => (tcls_s3::INT_IOI_FC, tcls_s3::IOI_FC),
                ChipKind::Spartan3E => (tcls_s3::INT_IOI_S3E, tcls_s3::IOI_S3E),
                ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                    (tcls_s3::INT_IOI_S3A_SN, tcls_s3::IOI_S3A_N)
                }
            };
            self.egrid.add_tile_single_id(cell, int_kind);
            self.egrid.add_tile_single_id(cell, ioi_kind);
            if cd.io != ColumnIoKind::None {
                let (data, tidx) = get_iob_data_n(self.chip.kind, cd.io);
                if tidx.to_idx() == 0 {
                    self.egrid.add_tile_e_id(cell, data.tcid, data.tiles);
                }
            }
            if !self.chip.kind.is_virtex2() {
                self.egrid.add_tile_id(
                    cell,
                    if self.chip.kind == ChipKind::FpgaCore {
                        tcls_s3::RANDOR_FC
                    } else {
                        tcls_s3::RANDOR
                    },
                    &[],
                );
            }
        }
    }

    fn fill_io_e(&mut self) {
        for cell in self.egrid.column(self.die, self.chip.col_e()) {
            if self.chip.is_row_io(cell.row) {
                continue;
            }
            let (int_kind, ioi_kind) = match self.chip.kind {
                ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => {
                    (tcls_v2::INT_IOI, tcls_v2::IOI)
                }
                ChipKind::Spartan3 => (tcls_s3::INT_IOI_S3, tcls_s3::IOI_S3),
                ChipKind::FpgaCore => (tcls_s3::INT_IOI_FC, tcls_s3::IOI_FC),
                ChipKind::Spartan3E => (tcls_s3::INT_IOI_S3E, tcls_s3::IOI_S3E),
                ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                    (tcls_s3::INT_IOI_S3A_WE, tcls_s3::IOI_S3A_WE)
                }
            };
            self.egrid.add_tile_single_id(cell, int_kind);
            self.egrid.add_tile_single_id(cell, ioi_kind);
            let (data, tidx) = get_iob_data_e(self.chip.kind, self.chip.rows[cell.row]);
            if tidx.to_idx() == 0 {
                self.egrid.add_tile_n_id(cell, data.tcid, data.tiles);
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
                    if cd.io == ColumnIoKind::DoubleEClk(1) {
                        (tcls_v2::INT_IOI_CLK_S, tcls_v2::IOI_CLK_S)
                    } else {
                        (tcls_v2::INT_IOI, tcls_v2::IOI)
                    }
                }
                ChipKind::Spartan3 => (tcls_s3::INT_IOI_S3, tcls_s3::IOI_S3),
                ChipKind::FpgaCore => (tcls_s3::INT_IOI_FC, tcls_s3::IOI_FC),
                ChipKind::Spartan3E => (tcls_s3::INT_IOI_S3E, tcls_s3::IOI_S3E),
                ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                    (tcls_s3::INT_IOI_S3A_SN, tcls_s3::IOI_S3A_S)
                }
            };
            self.egrid.add_tile_single_id(cell, int_kind);
            self.egrid.add_tile_single_id(cell, ioi_kind);
            if cd.io != ColumnIoKind::None {
                let (data, tidx) = get_iob_data_s(self.chip.kind, cd.io);
                if tidx.to_idx() == 0 {
                    self.egrid.add_tile_e_id(cell, data.tcid, data.tiles);
                }
            }
            if !self.chip.kind.is_virtex2() && self.chip.kind != ChipKind::FpgaCore {
                self.egrid.add_tile_id(cell, tcls_s3::RANDOR, &[]);
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
                    int_kind = tcls_v2::INT_IOI;
                    ioi_kind = tcls_v2::IOI;
                }
                ChipKind::Spartan3 => {
                    int_kind = tcls_s3::INT_IOI_S3;
                    ioi_kind = tcls_s3::IOI_S3;
                }
                ChipKind::FpgaCore => {
                    int_kind = tcls_s3::INT_IOI_FC;
                    ioi_kind = tcls_s3::IOI_FC;
                }
                ChipKind::Spartan3E => {
                    int_kind = tcls_s3::INT_IOI_S3E;
                    ioi_kind = tcls_s3::IOI_S3E;
                }
                ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                    int_kind = tcls_s3::INT_IOI_S3A_WE;
                    ioi_kind = tcls_s3::IOI_S3A_WE;
                }
            }
            self.egrid.add_tile_single_id(cell, int_kind);
            self.egrid.add_tile_single_id(cell, ioi_kind);
            let (data, tidx) = get_iob_data_w(self.chip.kind, self.chip.rows[cell.row]);
            if tidx.to_idx() == 0 {
                self.egrid.add_tile_n_id(cell, data.tcid, data.tiles);
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
                if self.chip.kind.is_virtex2() {
                    self.egrid.add_tile_single_id(cell, tcls_v2::INT_CLB);
                    self.egrid.add_tile_single_id(cell, tcls_v2::CLB);
                } else {
                    if self.chip.kind == ChipKind::FpgaCore {
                        self.egrid.add_tile_single_id(cell, tcls_s3::INT_CLB_FC);
                    } else {
                        self.egrid.add_tile_single_id(cell, tcls_s3::INT_CLB);
                    }
                    self.egrid.add_tile_single_id(cell, tcls_s3::CLB);
                }
            }
        }
    }

    fn fill_bram_dsp(&mut self) {
        let bram_kind = match self.chip.kind {
            ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => [tcls_v2::INT_BRAM; 4],
            ChipKind::Spartan3 => [tcls_s3::INT_BRAM_S3; 4],
            ChipKind::FpgaCore => return,
            ChipKind::Spartan3E => [tcls_s3::INT_BRAM_S3E; 4],
            ChipKind::Spartan3A => [
                tcls_s3::INT_BRAM_S3A_03,
                tcls_s3::INT_BRAM_S3A_12,
                tcls_s3::INT_BRAM_S3A_12,
                tcls_s3::INT_BRAM_S3A_03,
            ],
            ChipKind::Spartan3ADsp => [tcls_s3::INT_BRAM_S3ADSP; 4],
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
                    self.egrid.fill_conn_pair_id(
                        CellCoord::new(self.die, col + d, b - 1),
                        CellCoord::new(self.die, col + d, t + 1),
                        ccls_s3::TERM_BRAM_N,
                        ccls_s3::TERM_BRAM_S,
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
                self.egrid.add_tile_single_id(cell, bram_kind[idx]);
                if self.chip.kind == ChipKind::Spartan3ADsp {
                    self.egrid
                        .add_tile_single_id(cell.delta(3, 0), tcls_s3::INT_BRAM_S3ADSP);
                }
                if idx == 0 {
                    let kind = match self.chip.kind {
                        ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => {
                            tcls_v2::BRAM
                        }
                        ChipKind::Spartan3 => tcls_s3::BRAM_S3,
                        ChipKind::FpgaCore => unreachable!(),
                        ChipKind::Spartan3E => tcls_s3::BRAM_S3E,
                        ChipKind::Spartan3A => tcls_s3::BRAM_S3A,
                        ChipKind::Spartan3ADsp => tcls_s3::BRAM_S3ADSP,
                    };
                    self.egrid.add_tile_n_id(cell, kind, 4);
                    if self.chip.kind == ChipKind::Spartan3ADsp {
                        let cell = cell.delta(3, 0);
                        self.egrid.add_tile_n_id(cell, tcls_s3::DSP, 4);
                    }
                }
            }
        }
    }

    fn fill_dcm(&mut self) {
        if self.chip.kind.is_spartan3ea() {
            for pair in self.chip.get_dcm_pairs() {
                match pair.kind {
                    DcmPairKind::S => {
                        self.holes.push(pair.cell.delta(-4, 0).rect(8, 4));
                        let cell = pair.cell.delta(-1, 0);
                        self.egrid.add_tile_single_id(cell, tcls_s3::INT_DCM);
                        self.egrid.add_tile_single_id(cell, tcls_s3::DCM_S3E_SW);
                        let cell_root = cell.with_row(self.chip.row_s());
                        self.egrid[cell].region_root[rslots::DCM_CLKPAD] = cell_root;
                        self.egrid[cell].region_root[rslots::DCM_BUS] = cell_root;
                        let cell = pair.cell;
                        self.egrid.add_tile_single_id(cell, tcls_s3::INT_DCM);
                        self.egrid.add_tile_single_id(cell, tcls_s3::DCM_S3E_SE);
                        let cell_root = cell.with_row(self.chip.row_s());
                        self.egrid[cell].region_root[rslots::DCM_CLKPAD] = cell_root;
                        self.egrid[cell].region_root[rslots::DCM_BUS] = cell_root;
                    }
                    DcmPairKind::SingleS => {
                        self.holes.push(pair.cell.delta(-1, 0).rect(5, 4));
                        let cell = pair.cell.delta(-1, 0);
                        self.egrid
                            .add_tile_single_id(cell, tcls_s3::INT_DCM_S3E_DUMMY);
                        let cell = pair.cell;
                        self.egrid.add_tile_single_id(cell, tcls_s3::INT_DCM);
                        self.egrid.add_tile_single_id(cell, tcls_s3::DCM_S3E_SE);
                        let cell_root = cell.with_row(self.chip.row_s());
                        self.egrid[cell].region_root[rslots::DCM_CLKPAD] = cell_root;
                        self.egrid[cell].region_root[rslots::DCM_BUS] = cell_root;
                    }
                    DcmPairKind::N => {
                        self.holes.push(pair.cell.delta(-4, -3).rect(8, 4));
                        let cell = pair.cell.delta(-1, 0);
                        self.egrid.add_tile_single_id(cell, tcls_s3::INT_DCM);
                        self.egrid.add_tile_single_id(cell, tcls_s3::DCM_S3E_NW);
                        let cell_root = cell.with_row(self.chip.row_n());
                        self.egrid[cell].region_root[rslots::DCM_CLKPAD] = cell_root;
                        self.egrid[cell].region_root[rslots::DCM_BUS] = cell_root;
                        let cell = pair.cell;
                        self.egrid.add_tile_single_id(cell, tcls_s3::INT_DCM);
                        self.egrid.add_tile_single_id(cell, tcls_s3::DCM_S3E_NE);
                        let cell_root = cell.with_row(self.chip.row_n());
                        self.egrid[cell].region_root[rslots::DCM_CLKPAD] = cell_root;
                        self.egrid[cell].region_root[rslots::DCM_BUS] = cell_root;
                    }
                    DcmPairKind::SingleN => {
                        self.holes.push(pair.cell.delta(-1, -3).rect(5, 4));
                        let cell = pair.cell.delta(-1, 0);
                        self.egrid
                            .add_tile_single_id(cell, tcls_s3::INT_DCM_S3E_DUMMY);
                        let cell = pair.cell;
                        self.egrid.add_tile_single_id(cell, tcls_s3::INT_DCM);
                        self.egrid.add_tile_single_id(cell, tcls_s3::DCM_S3E_NE);
                        let cell_root = cell.with_row(self.chip.row_n());
                        self.egrid[cell].region_root[rslots::DCM_CLKPAD] = cell_root;
                        self.egrid[cell].region_root[rslots::DCM_BUS] = cell_root;
                    }
                    DcmPairKind::W => {
                        self.holes.push(pair.cell.delta(0, -4).rect(4, 8));
                        let cell = pair.cell.delta(0, -1);
                        self.egrid.add_tile_single_id(cell, tcls_s3::INT_DCM);
                        self.egrid.add_tile_single_id(cell, tcls_s3::DCM_S3E_WS);
                        let cell_root = cell.with_col(self.chip.col_w());
                        self.egrid[cell].region_root[rslots::DCM_CLKPAD] = cell_root;
                        self.egrid[cell].region_root[rslots::DCM_BUS] = cell_root;
                        let cell = pair.cell;
                        self.egrid.add_tile_single_id(cell, tcls_s3::INT_DCM);
                        self.egrid.add_tile_single_id(cell, tcls_s3::DCM_S3E_WN);
                        let cell_root = cell.with_col(self.chip.col_w());
                        self.egrid[cell].region_root[rslots::DCM_CLKPAD] = cell_root;
                        self.egrid[cell].region_root[rslots::DCM_BUS] = cell_root;
                    }
                    DcmPairKind::E => {
                        self.holes.push(pair.cell.delta(-3, -4).rect(4, 8));
                        let cell = pair.cell.delta(0, -1);
                        self.egrid.add_tile_single_id(cell, tcls_s3::INT_DCM);
                        self.egrid.add_tile_single_id(cell, tcls_s3::DCM_S3E_ES);
                        let cell_root = cell.with_col(self.chip.col_e());
                        self.egrid[cell].region_root[rslots::DCM_CLKPAD] = cell_root;
                        self.egrid[cell].region_root[rslots::DCM_BUS] = cell_root;
                        let cell = pair.cell;
                        self.egrid.add_tile_single_id(cell, tcls_s3::INT_DCM);
                        self.egrid.add_tile_single_id(cell, tcls_s3::DCM_S3E_EN);
                        let cell_root = cell.with_col(self.chip.col_e());
                        self.egrid[cell].region_root[rslots::DCM_CLKPAD] = cell_root;
                        self.egrid[cell].region_root[rslots::DCM_BUS] = cell_root;
                    }
                    DcmPairKind::Bram => {
                        let col_root = if pair.cell.col < self.chip.col_clk {
                            self.chip.col_w()
                        } else {
                            self.chip.col_e()
                        };
                        self.holes.push(pair.cell.delta(0, -4).rect(4, 8));
                        let cell = pair.cell.delta(0, -1);
                        self.egrid.add_tile_single_id(cell, tcls_s3::INT_DCM);
                        self.egrid.add_tile_single_id(cell, tcls_s3::DCM_S3E_WS);
                        let cell_root = cell.with_col(col_root);
                        self.egrid[cell].region_root[rslots::DCM_CLKPAD] = cell_root;
                        self.egrid[cell].region_root[rslots::DCM_BUS] = cell_root;
                        let cell = pair.cell;
                        self.egrid.add_tile_single_id(cell, tcls_s3::INT_DCM);
                        self.egrid.add_tile_single_id(cell, tcls_s3::DCM_S3E_WN);
                        let cell_root = cell.with_col(col_root);
                        self.egrid[cell].region_root[rslots::DCM_CLKPAD] = cell_root;
                        self.egrid[cell].region_root[rslots::DCM_BUS] = cell_root;
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
                        ChipKind::Virtex2 => (tcls_v2::INT_DCM_V2, Some(tcls_v2::DCM_V2)),
                        ChipKind::Virtex2P | ChipKind::Virtex2PX => {
                            (tcls_v2::INT_DCM_V2P, Some(tcls_v2::DCM_V2P))
                        }
                        ChipKind::Spartan3 => {
                            if col == self.chip.col_w() + 3 || col == self.chip.col_e() - 3 {
                                (tcls_s3::INT_DCM, Some(tcls_s3::DCM_S3))
                            } else {
                                (tcls_s3::INT_DCM_S3_DUMMY, None)
                            }
                        }
                        _ => unreachable!(),
                    };
                    for row in [self.chip.row_s(), self.chip.row_n()] {
                        let cell = CellCoord::new(self.die, col, row);
                        self.egrid.add_tile_single_id(cell, kind);
                        if let Some(dcm) = dcm {
                            self.egrid.add_tile_single_id(cell, dcm);
                        }
                    }
                }
                if self.chip.kind.is_virtex2() {
                    self.egrid.add_tile_single_id(
                        CellCoord::new(self.die, col, row_b),
                        tcls_v2::DCMCONN_S,
                    );
                    self.egrid.add_tile_single_id(
                        CellCoord::new(self.die, col, row_t),
                        tcls_v2::DCMCONN_N,
                    );
                } else {
                    if col == self.chip.col_w() + 3 || col == self.chip.col_e() - 3 {
                        self.egrid.add_tile_single_id(
                            CellCoord::new(self.die, col, row_b),
                            tcls_s3::DCMCONN_S,
                        );
                        self.egrid.add_tile_single_id(
                            CellCoord::new(self.die, col, row_t),
                            tcls_s3::DCMCONN_N,
                        );
                    }
                }
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
                self.egrid.add_tile_single_id(cell, tcls_v2::INT_PPC);
                self.egrid.add_tile_single_id(cell, tcls_v2::INTF_PPC);
            }
            // horiz passes
            for d in 1..15 {
                let cell_w = cell.delta(0, d);
                let cell_e = cell.delta(9, d);
                self.egrid
                    .add_tile_id(cell_w, tcls_v2::PPC_TERM_E, &[cell_w, cell_e]);
                self.egrid
                    .add_tile_id(cell_e, tcls_v2::PPC_TERM_W, &[cell_e, cell_w]);
                self.egrid
                    .fill_conn_pair_id(cell_w, cell_e, ccls_v2::PPC_E, ccls_v2::PPC_W);
            }
            // vert passes
            for d in 1..9 {
                let cell_s = cell.delta(d, 0);
                let cell_n = cell.delta(d, 15);
                self.egrid
                    .add_tile_id(cell_s, tcls_v2::PPC_TERM_N, &[cell_s, cell_n]);
                self.egrid
                    .add_tile_id(cell_n, tcls_v2::PPC_TERM_S, &[cell_n, cell_s]);
                self.egrid
                    .fill_conn_pair_id(cell_s, cell_n, ccls_v2::PPC_N, ccls_v2::PPC_S);
            }
            let tcid = if bc < self.chip.col_clk {
                tcls_v2::PPC_W
            } else {
                tcls_v2::PPC_E
            };
            self.egrid.add_tile_id(cell, tcid, &tcells);
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
            self.egrid
                .add_tile_single_id(cell_s, tcls_v2::INT_GT_CLKPAD);
            self.egrid
                .add_tile_single_id(cell_n, tcls_v2::INT_GT_CLKPAD);
            self.egrid
                .add_tile_single_id(cell_s, tcls_v2::INTF_GT_S_CLKPAD);
            self.egrid
                .add_tile_single_id(cell_n, tcls_v2::INTF_GT_N_CLKPAD);
            let n = match self.chip.kind {
                ChipKind::Virtex2P => 4,
                ChipKind::Virtex2PX => 8,
                _ => unreachable!(),
            };
            for d in 0..n {
                let cell = cell_s.delta(0, 1 + d);
                self.egrid.add_tile_single_id(cell, tcls_v2::INT_PPC);
                self.egrid.add_tile_single_id(
                    cell,
                    if d % 4 == 0 {
                        tcls_v2::INTF_GT_S0
                    } else {
                        tcls_v2::INTF_GT_S123
                    },
                );
            }
            for d in 0..n {
                let cell = cell_n.delta(0, -n + d);
                self.egrid.add_tile_single_id(cell, tcls_v2::INT_PPC);
                self.egrid.add_tile_single_id(
                    cell,
                    if d % 4 == 0 {
                        tcls_v2::INTF_GT_N0
                    } else {
                        tcls_v2::INTF_GT_N123
                    },
                );
            }
            if self.chip.kind == ChipKind::Virtex2P {
                self.egrid.add_tile_id(
                    cell_s,
                    tcls_v2::GIGABIT_S,
                    &[
                        cell_s,
                        cell_s.delta(0, 1),
                        cell_s.delta(0, 2),
                        cell_s.delta(0, 3),
                        cell_s.delta(0, 4),
                    ],
                );
                self.egrid.add_tile_id(
                    cell_n,
                    tcls_v2::GIGABIT_N,
                    &[
                        cell_n,
                        cell_n.delta(0, -4),
                        cell_n.delta(0, -3),
                        cell_n.delta(0, -2),
                        cell_n.delta(0, -1),
                    ],
                );
            } else {
                self.egrid.add_tile_id(
                    cell_s,
                    tcls_v2::GIGABIT10_S,
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
                self.egrid.add_tile_id(
                    cell_n,
                    tcls_v2::GIGABIT10_N,
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
            let mut term_s = ccls_s3::LLV_S;
            let mut term_n = ccls_s3::LLV_N;
            if self.chip.is_col_io(cell.col) && self.chip.kind != ChipKind::Spartan3A {
                term_s = ccls_s3::LLV_CLK_WE_S3E_S;
                term_n = ccls_s3::LLV_CLK_WE_S3E_N;
            }
            self.egrid.fill_conn_pair_id(cell_s, cell_n, term_n, term_s);
            self.egrid.add_tile_id(
                cell_n,
                if self.chip.kind.is_spartan3a() {
                    tcls_s3::LLV_S3A
                } else {
                    tcls_s3::LLV_S3E
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
            let mut term_w = ccls_s3::LLH_W;
            let mut term_e = ccls_s3::LLH_E;
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
                term_w = ccls_s3::LLH_DCM_S3ADSP_W;
                term_e = ccls_s3::LLH_DCM_S3ADSP_E;
            }
            self.egrid.fill_conn_pair_id(cell_w, cell_e, term_e, term_w);
            self.egrid.add_tile_id(
                cell_e,
                if self.chip.kind.is_spartan3a() && cell.row == self.chip.row_s() {
                    tcls_s3::LLH_S_S3A
                } else if self.chip.kind.is_spartan3a() && cell.row == self.chip.row_n() {
                    tcls_s3::LLH_N_S3A
                } else {
                    tcls_s3::LLH
                },
                &[cell_w, cell_e],
            );
        }
    }

    fn fill_misc_passes(&mut self) {
        if self.chip.kind == ChipKind::Spartan3E && !self.chip.has_ll {
            for col in [self.chip.col_w(), self.chip.col_e()] {
                self.egrid.fill_conn_pair_id(
                    CellCoord::new(self.die, col, self.chip.row_mid() - 1),
                    CellCoord::new(self.die, col, self.chip.row_mid()),
                    ccls_s3::CLK_WE_S3E_N,
                    ccls_s3::CLK_WE_S3E_S,
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
                    self.egrid.fill_conn_pair_id(
                        cell.delta(0, -1),
                        cell,
                        ccls_s3::BRKH_S3_N,
                        ccls_s3::BRKH_S3_S,
                    );
                }
            }
        }
        if self.chip.kind == ChipKind::Spartan3ADsp {
            for (col, cd) in &self.chip.columns {
                if cd.kind == ColumnKind::Dsp {
                    for row in [self.chip.row_s(), self.chip.row_n()] {
                        self.egrid.fill_conn_pair_id(
                            CellCoord::new(self.die, col, row),
                            CellCoord::new(self.die, col + 1, row),
                            ccls_s3::DSPHOLE_E,
                            ccls_s3::DSPHOLE_W,
                        );
                    }
                }
            }
            for col in [self.chip.col_w() + 3, self.chip.col_e() - 6] {
                for row in [self.chip.row_mid() - 1, self.chip.row_mid()] {
                    self.egrid.fill_conn_pair_id(
                        CellCoord::new(self.die, col, row),
                        CellCoord::new(self.die, col + 4, row),
                        ccls_s3::DSPHOLE_E,
                        ccls_s3::DSPHOLE_W,
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
                    self.egrid.fill_conn_pair_id(
                        CellCoord::new(self.die, col - 1, row),
                        CellCoord::new(self.die, col + 4, row),
                        ccls_s3::HDCM_E,
                        ccls_s3::HDCM_W,
                    );
                }
            }
        }
    }

    fn fill_main_passes(&mut self) {
        let pass_w;
        let pass_e;
        let pass_s;
        let pass_n;
        if self.chip.kind.is_virtex2() {
            pass_w = ccls_v2::PASS_W;
            pass_e = ccls_v2::PASS_E;
            pass_s = ccls_v2::PASS_S;
            pass_n = ccls_v2::PASS_N;
        } else if self.chip.kind == ChipKind::FpgaCore {
            pass_w = ccls_s3::PASS_W_FC;
            pass_e = ccls_s3::PASS_E;
            pass_s = ccls_s3::PASS_S_FC;
            pass_n = ccls_s3::PASS_N;
        } else {
            pass_w = ccls_s3::PASS_W;
            pass_e = ccls_s3::PASS_E;
            pass_s = ccls_s3::PASS_S;
            pass_n = ccls_s3::PASS_N;
        }
        let slot_w = cslots::W;
        let slot_e = cslots::E;
        let slot_s = cslots::S;
        let slot_n = cslots::N;
        let die = DieId::from_idx(0);
        // horizontal
        for row in self.egrid.rows(die) {
            let mut prev = None;
            for cell in self.egrid.row(die, row) {
                if !self.egrid[cell].tiles.contains_id(tslots::INT) {
                    continue;
                }
                if let Some(prev) = prev
                    && !self.egrid[cell].conns.contains_id(slot_w)
                {
                    self.egrid.fill_conn_pair_id(prev, cell, pass_e, pass_w);
                }
                if !self.egrid[cell].conns.contains_id(slot_e) {
                    prev = Some(cell);
                } else {
                    prev = None;
                }
            }
        }
        // vertical
        for col in self.egrid.cols(die) {
            let mut prev = None;
            for cell in self.egrid.column(die, col) {
                if !self.egrid[cell].tiles.contains_id(tslots::INT) {
                    continue;
                }
                if let Some(prev) = prev
                    && !self.egrid[cell].conns.contains_id(slot_s)
                {
                    self.egrid.fill_conn_pair_id(prev, cell, pass_n, pass_s);
                }
                if !self.egrid[cell].conns.contains_id(slot_n) {
                    prev = Some(cell);
                } else {
                    prev = None;
                }
            }
        }
    }

    fn fill_bram_passes(&mut self) {
        if matches!(self.chip.kind, ChipKind::Spartan3A | ChipKind::Spartan3ADsp) {
            let slot_n = cslots::N;
            let slot_s = cslots::S;
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

    fn fill_clk_sn_v2(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            let cell_root = cell.with_col(if cell.col < self.chip.col_clk {
                self.chip.col_clk - 1
            } else {
                self.chip.col_clk
            });
            self.egrid[cell].region_root[rslots::DCM_BUS] = cell_root;
            self.egrid[cell].region_root[rslots::DCM_CLKPAD] = cell_root;
        }
        {
            let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_s());
            self.egrid.add_tile_we_id(cell, tcls_v2::CLK_S, 1, 2);
        }
        {
            let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_n());
            self.egrid.add_tile_we_id(cell, tcls_v2::CLK_N, 1, 2);
        }

        {
            let cell = CellCoord::new(self.die, self.chip.col_w(), self.chip.row_pci.unwrap());
            self.egrid.add_tile_sn_id(cell, tcls_v2::PCI_W, 2, 4);
        }
        {
            let cell = CellCoord::new(self.die, self.chip.col_e(), self.chip.row_pci.unwrap());
            self.egrid.add_tile_sn_id(cell, tcls_v2::PCI_E, 2, 4);
        }
    }

    fn fill_clk_sn_s3(&mut self) {
        if !self.chip.kind.is_spartan3ea() {
            for cell in self.egrid.die_cells(self.die) {
                let cell_root_bus = cell.with_col(if cell.col < self.chip.col_clk {
                    self.chip.col_clk - 1
                } else {
                    self.chip.col_clk
                });
                self.egrid[cell].region_root[rslots::DCM_BUS] = cell_root_bus;
                self.egrid[cell].region_root[rslots::DCM_CLKPAD] = cell.with_col(self.chip.col_clk);
            }
        }
        let (tcid_s, tcid_n) = match self.chip.kind {
            ChipKind::Spartan3 => (tcls_s3::CLK_S_S3, tcls_s3::CLK_N_S3),
            ChipKind::FpgaCore => (tcls_s3::CLK_S_FC, tcls_s3::CLK_N_FC),
            ChipKind::Spartan3E => (tcls_s3::CLK_S_S3E, tcls_s3::CLK_N_S3E),
            ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                (tcls_s3::CLK_S_S3A, tcls_s3::CLK_N_S3A)
            }
            _ => unreachable!(),
        };
        let (w, n) = if self.chip.kind.is_spartan3ea() {
            (4, 8)
        } else {
            (1, 2)
        };
        {
            let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_s());
            self.egrid.add_tile_we_id(cell, tcid_s, w, n);
        }
        {
            let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_n());
            self.egrid.add_tile_we_id(cell, tcid_n, w, n);
        }
    }

    fn fill_clk_we_s3e(&mut self) {
        let (tcid_w, tcid_e) = if self.chip.kind == ChipKind::Spartan3E {
            (tcls_s3::CLK_W_S3E, tcls_s3::CLK_E_S3E)
        } else {
            (tcls_s3::CLK_W_S3A, tcls_s3::CLK_E_S3A)
        };
        {
            let cell = CellCoord::new(self.die, self.chip.col_w(), self.chip.row_mid());
            self.egrid.add_tile_sn_id(cell, tcid_w, 4, 8);
        }
        {
            let cell = CellCoord::new(self.die, self.chip.col_e(), self.chip.row_mid());
            self.egrid.add_tile_sn_id(cell, tcid_e, 4, 8);
        }
    }

    fn fill_hclk(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            self.egrid[cell].region_root[rslots::GLOBAL] =
                CellCoord::new(self.die, self.chip.col_clk, self.chip.row_mid());
        }
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
            for &(row_m, row_b, row_t) in &self.chip.rows_hclk {
                let row_q = if self.chip.kind.is_virtex2() {
                    row_m
                } else if row_m < self.chip.row_mid() {
                    self.chip.row_mid() - 1
                } else {
                    self.chip.row_mid()
                };
                for row in row_b.range(row_m) {
                    let row_root = if matches!(self.chip.columns[col].kind, ColumnKind::BramCont(_))
                        && self.chip.kind.is_spartan3a()
                    {
                        row_m
                    } else {
                        row_m - 1
                    };
                    let cell = CellCoord::new(self.die, col, row);
                    self.egrid[cell].region_root[rslots::LEAF] = cell.with_row(row_root);
                    self.egrid[cell].region_root[rslots::HCLK] =
                        CellCoord::new(self.die, col_q, row_q);
                }
                for row in row_m.range(row_t) {
                    let cell = CellCoord::new(self.die, col, row);
                    self.egrid[cell].region_root[rslots::LEAF] = cell.with_row(row_m);
                    self.egrid[cell].region_root[rslots::HCLK] =
                        CellCoord::new(self.die, col_q, row_q);
                }
                let kind = if matches!(self.chip.columns[col].kind, ColumnKind::BramCont(_))
                    && self.chip.kind.is_spartan3a()
                {
                    tcls_s3::HCLK_UNI
                } else if !self.chip.kind.is_virtex2() {
                    tcls_s3::HCLK
                } else {
                    tcls_v2::HCLK
                };
                let cell = CellCoord::new(self.die, col, row_m);
                self.egrid
                    .add_tile_id(cell, kind, &[cell.delta(0, -1), cell]);
            }
        }
    }

    fn fill_hrow(&mut self) {
        for &(row_m, _, _) in &self.chip.rows_hclk {
            if self.chip.kind.is_virtex2() {
                let kind = if row_m == self.chip.row_s() + 1 {
                    tcls_v2::HROW_S
                } else if row_m == self.chip.row_n() {
                    tcls_v2::HROW_N
                } else {
                    tcls_v2::HROW
                };
                self.egrid.add_tile_we_id(
                    CellCoord::new(self.die, self.chip.col_clk, row_m),
                    kind,
                    1,
                    2,
                );
            }
        }
    }

    fn fill_clkc(&mut self) {
        if !self.chip.kind.is_virtex2() && self.chip.cols_clkv.is_none() {
            self.egrid.add_tile_we_id(
                CellCoord::new(self.die, self.chip.col_clk, self.chip.row_mid()),
                tcls_s3::CLKC_50A,
                1,
                2,
            );
        }
    }

    fn fill_clkqc(&mut self) {
        if let Some((col_cl, col_cr)) = self.chip.cols_clkv {
            if matches!(self.chip.kind, ChipKind::Spartan3 | ChipKind::FpgaCore) {
                self.egrid.add_tile_sn_id(
                    CellCoord::new(self.die, col_cl, self.chip.row_mid()),
                    tcls_s3::CLKQC_S3,
                    1,
                    2,
                );
                self.egrid.add_tile_sn_id(
                    CellCoord::new(self.die, col_cr, self.chip.row_mid()),
                    tcls_s3::CLKQC_S3,
                    1,
                    2,
                );
            } else {
                self.egrid.add_tile_sn_id(
                    CellCoord::new(self.die, col_cl, self.chip.row_mid()),
                    tcls_s3::CLKQC_S3E,
                    1,
                    2,
                );
                self.egrid.add_tile_sn_id(
                    CellCoord::new(self.die, col_cr, self.chip.row_mid()),
                    tcls_s3::CLKQC_S3E,
                    1,
                    2,
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
        self.term_w_frame = self.frame_info.len();
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
        self.term_e_frame = self.frame_info.len();
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
            term_w_frame: 0,
            term_e_frame: 0,
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
        expander.fill_main_passes();
        expander.fill_bram_passes();
        if self.kind.is_virtex2() {
            expander.fill_clk_sn_v2();
        } else if matches!(self.kind, ChipKind::Spartan3 | ChipKind::FpgaCore) {
            expander.fill_clk_sn_s3();
        } else {
            expander.fill_clk_sn_s3();
            expander.fill_clk_we_s3e();
        }
        expander.fill_hclk();
        expander.fill_hrow();
        expander.fill_clkc();
        expander.fill_clkqc();
        expander.fill_frame_info();

        let clkv_frame = expander.clkv_frame;
        let spine_frame = expander.spine_frame;
        let term_w_frame = expander.term_w_frame;
        let term_e_frame = expander.term_e_frame;
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
            term_w_frame,
            term_e_frame,
            col_frame,
            bram_frame,
            holes,
        }
    }
}
