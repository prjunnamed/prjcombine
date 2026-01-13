use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};
use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::dir::DirHV;
use prjcombine_interconnect::grid::{CellCoord, ColId, DieId, ExpandedGrid, Rect};
use prjcombine_xilinx_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};

use crate::chip::{Chip, ChipKind, ColumnIoKind, ColumnKind, DcmPairKind};
use crate::defs;
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
        let cnr_kind = if self.chip.kind.is_virtex2() {
            defs::virtex2::tcls::INT_CNR
        } else if self.chip.kind == ChipKind::FpgaCore {
            defs::spartan3::tcls::INT_CLB_FC
        } else {
            defs::spartan3::tcls::INT_CLB
        };
        let (ll, lr, ul, ur) = match self.chip.kind {
            ChipKind::Virtex2 => (
                defs::virtex2::tcls::CNR_SW_V2,
                defs::virtex2::tcls::CNR_SE_V2,
                defs::virtex2::tcls::CNR_NW_V2,
                defs::virtex2::tcls::CNR_NE_V2,
            ),
            ChipKind::Virtex2P | ChipKind::Virtex2PX => (
                defs::virtex2::tcls::CNR_SW_V2P,
                defs::virtex2::tcls::CNR_SE_V2P,
                defs::virtex2::tcls::CNR_NW_V2P,
                defs::virtex2::tcls::CNR_NE_V2P,
            ),
            ChipKind::Spartan3 => (
                defs::spartan3::tcls::CNR_SW_S3,
                defs::spartan3::tcls::CNR_SE_S3,
                defs::spartan3::tcls::CNR_NW_S3,
                defs::spartan3::tcls::CNR_NE_S3,
            ),
            ChipKind::FpgaCore => (
                defs::spartan3::tcls::CNR_SW_FC,
                defs::spartan3::tcls::CNR_SE_FC,
                defs::spartan3::tcls::CNR_NW_FC,
                defs::spartan3::tcls::CNR_NE_FC,
            ),
            ChipKind::Spartan3E => (
                defs::spartan3::tcls::CNR_SW_S3E,
                defs::spartan3::tcls::CNR_SE_S3E,
                defs::spartan3::tcls::CNR_NW_S3E,
                defs::spartan3::tcls::CNR_NE_S3E,
            ),
            ChipKind::Spartan3A | ChipKind::Spartan3ADsp => (
                defs::spartan3::tcls::CNR_SW_S3A,
                defs::spartan3::tcls::CNR_SE_S3A,
                defs::spartan3::tcls::CNR_NW_S3A,
                defs::spartan3::tcls::CNR_NE_S3A,
            ),
        };

        self.egrid
            .add_tile_single_id(self.chip.corner(DirHV::SW).cell, cnr_kind);
        self.egrid
            .add_tile_single_id(self.chip.corner(DirHV::SE).cell, cnr_kind);
        self.egrid
            .add_tile_single_id(self.chip.corner(DirHV::NW).cell, cnr_kind);
        self.egrid
            .add_tile_single_id(self.chip.corner(DirHV::NE).cell, cnr_kind);
        self.egrid
            .add_tile_single_id(self.chip.corner(DirHV::SW).cell, ll);
        self.egrid
            .add_tile_single_id(self.chip.corner(DirHV::SE).cell, lr);
        self.egrid
            .add_tile_single_id(self.chip.corner(DirHV::NW).cell, ul);
        self.egrid
            .add_tile_single_id(self.chip.corner(DirHV::NE).cell, ur);

        if !self.chip.kind.is_virtex2() {
            self.egrid.add_tile_id(
                self.chip.corner(DirHV::NW).cell,
                if self.chip.kind == ChipKind::FpgaCore {
                    defs::spartan3::tcls::RANDOR_INIT_FC
                } else {
                    defs::spartan3::tcls::RANDOR_INIT
                },
                &[],
            );
        }
    }

    fn fill_term(&mut self) {
        for cell in self.egrid.row(self.die, self.chip.row_s()) {
            if self.chip.kind.is_virtex2() {
                self.egrid
                    .add_tile_single_id(cell, defs::virtex2::tcls::TERM_S);
                self.egrid
                    .fill_conn_term_id(cell, defs::virtex2::ccls::TERM_S);
            } else {
                self.egrid
                    .fill_conn_term_id(cell, defs::spartan3::ccls::TERM_S);
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_n()) {
            if self.chip.kind.is_virtex2() {
                self.egrid
                    .add_tile_single_id(cell, defs::virtex2::tcls::TERM_N);
                self.egrid
                    .fill_conn_term_id(cell, defs::virtex2::ccls::TERM_N);
            } else {
                self.egrid
                    .fill_conn_term_id(cell, defs::spartan3::ccls::TERM_N);
            }
        }
        for cell in self.egrid.column(self.die, self.chip.col_w()) {
            if self.chip.kind.is_virtex2() {
                self.egrid
                    .add_tile_single_id(cell, defs::virtex2::tcls::TERM_W);
                self.egrid
                    .fill_conn_term_id(cell, defs::virtex2::ccls::TERM_W);
            } else {
                self.egrid
                    .fill_conn_term_id(cell, defs::spartan3::ccls::TERM_W);
            }
        }
        for cell in self.egrid.column(self.die, self.chip.col_e()) {
            if self.chip.kind.is_virtex2() {
                self.egrid
                    .add_tile_single_id(cell, defs::virtex2::tcls::TERM_E);
                self.egrid
                    .fill_conn_term_id(cell, defs::virtex2::ccls::TERM_E);
            } else {
                self.egrid
                    .fill_conn_term_id(cell, defs::spartan3::ccls::TERM_E);
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
                        (
                            defs::virtex2::tcls::INT_IOI_CLK_N,
                            defs::virtex2::tcls::IOI_CLK_N,
                        )
                    } else {
                        (defs::virtex2::tcls::INT_IOI, defs::virtex2::tcls::IOI)
                    }
                }
                ChipKind::Spartan3 => (
                    defs::spartan3::tcls::INT_IOI_S3,
                    defs::spartan3::tcls::IOI_S3,
                ),
                ChipKind::FpgaCore => (
                    defs::spartan3::tcls::INT_IOI_FC,
                    defs::spartan3::tcls::IOI_FC,
                ),
                ChipKind::Spartan3E => (
                    defs::spartan3::tcls::INT_IOI_S3E,
                    defs::spartan3::tcls::IOI_S3E,
                ),
                ChipKind::Spartan3A | ChipKind::Spartan3ADsp => (
                    defs::spartan3::tcls::INT_IOI_S3A_SN,
                    defs::spartan3::tcls::IOI_S3A_N,
                ),
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
                        defs::spartan3::tcls::RANDOR_FC
                    } else {
                        defs::spartan3::tcls::RANDOR
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
                    (defs::virtex2::tcls::INT_IOI, defs::virtex2::tcls::IOI)
                }
                ChipKind::Spartan3 => (
                    defs::spartan3::tcls::INT_IOI_S3,
                    defs::spartan3::tcls::IOI_S3,
                ),
                ChipKind::FpgaCore => (
                    defs::spartan3::tcls::INT_IOI_FC,
                    defs::spartan3::tcls::IOI_FC,
                ),
                ChipKind::Spartan3E => (
                    defs::spartan3::tcls::INT_IOI_S3E,
                    defs::spartan3::tcls::IOI_S3E,
                ),
                ChipKind::Spartan3A | ChipKind::Spartan3ADsp => (
                    defs::spartan3::tcls::INT_IOI_S3A_WE,
                    defs::spartan3::tcls::IOI_S3A_WE,
                ),
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
                        (
                            defs::virtex2::tcls::INT_IOI_CLK_S,
                            defs::virtex2::tcls::IOI_CLK_S,
                        )
                    } else {
                        (defs::virtex2::tcls::INT_IOI, defs::virtex2::tcls::IOI)
                    }
                }
                ChipKind::Spartan3 => (
                    defs::spartan3::tcls::INT_IOI_S3,
                    defs::spartan3::tcls::IOI_S3,
                ),
                ChipKind::FpgaCore => (
                    defs::spartan3::tcls::INT_IOI_FC,
                    defs::spartan3::tcls::IOI_FC,
                ),
                ChipKind::Spartan3E => (
                    defs::spartan3::tcls::INT_IOI_S3E,
                    defs::spartan3::tcls::IOI_S3E,
                ),
                ChipKind::Spartan3A | ChipKind::Spartan3ADsp => (
                    defs::spartan3::tcls::INT_IOI_S3A_SN,
                    defs::spartan3::tcls::IOI_S3A_S,
                ),
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
                self.egrid
                    .add_tile_id(cell, defs::spartan3::tcls::RANDOR, &[]);
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
                    int_kind = defs::virtex2::tcls::INT_IOI;
                    ioi_kind = defs::virtex2::tcls::IOI;
                }
                ChipKind::Spartan3 => {
                    int_kind = defs::spartan3::tcls::INT_IOI_S3;
                    ioi_kind = defs::spartan3::tcls::IOI_S3;
                }
                ChipKind::FpgaCore => {
                    int_kind = defs::spartan3::tcls::INT_IOI_FC;
                    ioi_kind = defs::spartan3::tcls::IOI_FC;
                }
                ChipKind::Spartan3E => {
                    int_kind = defs::spartan3::tcls::INT_IOI_S3E;
                    ioi_kind = defs::spartan3::tcls::IOI_S3E;
                }
                ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                    int_kind = defs::spartan3::tcls::INT_IOI_S3A_WE;
                    ioi_kind = defs::spartan3::tcls::IOI_S3A_WE;
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
                    self.egrid
                        .add_tile_single_id(cell, defs::virtex2::tcls::INT_CLB);
                    self.egrid
                        .add_tile_single_id(cell, defs::virtex2::tcls::CLB);
                } else {
                    if self.chip.kind == ChipKind::FpgaCore {
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::INT_CLB_FC);
                    } else {
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::INT_CLB);
                    }
                    self.egrid
                        .add_tile_single_id(cell, defs::spartan3::tcls::CLB);
                }
            }
        }
    }

    fn fill_bram_dsp(&mut self) {
        let bram_kind = match self.chip.kind {
            ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => {
                [defs::virtex2::tcls::INT_BRAM; 4]
            }
            ChipKind::Spartan3 => [defs::spartan3::tcls::INT_BRAM_S3; 4],
            ChipKind::FpgaCore => return,
            ChipKind::Spartan3E => [defs::spartan3::tcls::INT_BRAM_S3E; 4],
            ChipKind::Spartan3A => [
                defs::spartan3::tcls::INT_BRAM_S3A_03,
                defs::spartan3::tcls::INT_BRAM_S3A_12,
                defs::spartan3::tcls::INT_BRAM_S3A_12,
                defs::spartan3::tcls::INT_BRAM_S3A_03,
            ],
            ChipKind::Spartan3ADsp => [defs::spartan3::tcls::INT_BRAM_S3ADSP; 4],
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
                        defs::spartan3::ccls::TERM_BRAM_N,
                        defs::spartan3::ccls::TERM_BRAM_S,
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
                    self.egrid.add_tile_single_id(
                        cell.delta(3, 0),
                        defs::spartan3::tcls::INT_BRAM_S3ADSP,
                    );
                }
                if idx == 0 {
                    let kind = match self.chip.kind {
                        ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => {
                            defs::virtex2::tcls::BRAM
                        }
                        ChipKind::Spartan3 => defs::spartan3::tcls::BRAM_S3,
                        ChipKind::FpgaCore => unreachable!(),
                        ChipKind::Spartan3E => defs::spartan3::tcls::BRAM_S3E,
                        ChipKind::Spartan3A => defs::spartan3::tcls::BRAM_S3A,
                        ChipKind::Spartan3ADsp => defs::spartan3::tcls::BRAM_S3ADSP,
                    };
                    self.egrid.add_tile_n_id(cell, kind, 4);
                    if self.chip.kind == ChipKind::Spartan3ADsp {
                        let cell = cell.delta(3, 0);
                        self.egrid.add_tile_n_id(cell, defs::spartan3::tcls::DSP, 4);
                        self.egrid
                            .add_tile_n_id(cell, defs::spartan3::tcls::INTF_DSP, 4);
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
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::INT_DCM);
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::DCM_S3E_SW);
                        let cell = pair.cell;
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::INT_DCM);
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::DCM_S3E_SE);
                    }
                    DcmPairKind::SingleS => {
                        self.holes.push(pair.cell.delta(-1, 0).rect(5, 4));
                        let cell = pair.cell.delta(-1, 0);
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::INT_DCM_S3E_DUMMY);
                        let cell = pair.cell;
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::INT_DCM);
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::DCM_S3E_SE);
                    }
                    DcmPairKind::N => {
                        self.holes.push(pair.cell.delta(-4, -3).rect(8, 4));
                        let cell = pair.cell.delta(-1, 0);
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::INT_DCM);
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::DCM_S3E_NW);
                        let cell = pair.cell;
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::INT_DCM);
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::DCM_S3E_NE);
                    }
                    DcmPairKind::SingleN => {
                        self.holes.push(pair.cell.delta(-1, -3).rect(5, 4));
                        let cell = pair.cell.delta(-1, 0);
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::INT_DCM_S3E_DUMMY);
                        let cell = pair.cell;
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::INT_DCM);
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::DCM_S3E_NE);
                    }
                    DcmPairKind::W => {
                        self.holes.push(pair.cell.delta(0, -4).rect(4, 8));
                        let cell = pair.cell.delta(0, -1);
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::INT_DCM);
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::DCM_S3E_WS);
                        let cell = pair.cell;
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::INT_DCM);
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::DCM_S3E_WN);
                    }
                    DcmPairKind::E => {
                        self.holes.push(pair.cell.delta(-3, -4).rect(4, 8));
                        let cell = pair.cell.delta(0, -1);
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::INT_DCM);
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::DCM_S3E_ES);
                        let cell = pair.cell;
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::INT_DCM);
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::DCM_S3E_EN);
                    }
                    DcmPairKind::Bram => {
                        self.holes.push(pair.cell.delta(0, -4).rect(4, 8));
                        let cell = pair.cell.delta(0, -1);
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::INT_DCM);
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::DCM_S3E_WS);
                        let cell = pair.cell;
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::INT_DCM);
                        self.egrid
                            .add_tile_single_id(cell, defs::spartan3::tcls::DCM_S3E_WN);
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
                        ChipKind::Virtex2 => (
                            defs::virtex2::tcls::INT_DCM_V2,
                            Some(defs::virtex2::tcls::DCM_V2),
                        ),
                        ChipKind::Virtex2P | ChipKind::Virtex2PX => (
                            defs::virtex2::tcls::INT_DCM_V2P,
                            Some(defs::virtex2::tcls::DCM_V2P),
                        ),
                        ChipKind::Spartan3 => {
                            if col == self.chip.col_w() + 3 || col == self.chip.col_e() - 3 {
                                (
                                    defs::spartan3::tcls::INT_DCM,
                                    Some(defs::spartan3::tcls::DCM_S3),
                                )
                            } else {
                                (defs::spartan3::tcls::INT_DCM_S3_DUMMY, None)
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
                        defs::virtex2::tcls::DCMCONN_S,
                    );
                    self.egrid.add_tile_single_id(
                        CellCoord::new(self.die, col, row_t),
                        defs::virtex2::tcls::DCMCONN_N,
                    );
                } else {
                    self.egrid.add_tile_single_id(
                        CellCoord::new(self.die, col, row_b),
                        defs::spartan3::tcls::DCMCONN_S,
                    );
                    self.egrid.add_tile_single_id(
                        CellCoord::new(self.die, col, row_t),
                        defs::spartan3::tcls::DCMCONN_N,
                    );
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
                self.egrid
                    .add_tile_single_id(cell, defs::virtex2::tcls::INT_PPC);
                self.egrid
                    .add_tile_single_id(cell, defs::virtex2::tcls::INTF_PPC);
            }
            // horiz passes
            for d in 1..15 {
                let cell_w = cell.delta(0, d);
                let cell_e = cell.delta(9, d);
                self.egrid
                    .add_tile_id(cell_w, defs::virtex2::tcls::PPC_TERM_E, &[cell_w, cell_e]);
                self.egrid
                    .add_tile_id(cell_e, defs::virtex2::tcls::PPC_TERM_W, &[cell_e, cell_w]);
                self.egrid.fill_conn_pair_id(
                    cell_w,
                    cell_e,
                    defs::virtex2::ccls::PPC_E,
                    defs::virtex2::ccls::PPC_W,
                );
            }
            // vert passes
            for d in 1..9 {
                let cell_s = cell.delta(d, 0);
                let cell_n = cell.delta(d, 15);
                self.egrid
                    .add_tile_id(cell_s, defs::virtex2::tcls::PPC_TERM_N, &[cell_s, cell_n]);
                self.egrid
                    .add_tile_id(cell_n, defs::virtex2::tcls::PPC_TERM_S, &[cell_n, cell_s]);
                self.egrid.fill_conn_pair_id(
                    cell_s,
                    cell_n,
                    defs::virtex2::ccls::PPC_N,
                    defs::virtex2::ccls::PPC_S,
                );
            }
            let tcid = if bc < self.chip.col_clk {
                defs::virtex2::tcls::PPC_W
            } else {
                defs::virtex2::tcls::PPC_E
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
                .add_tile_single_id(cell_s, defs::virtex2::tcls::INT_GT_CLKPAD);
            self.egrid
                .add_tile_single_id(cell_n, defs::virtex2::tcls::INT_GT_CLKPAD);
            self.egrid
                .add_tile_single_id(cell_s, defs::virtex2::tcls::INTF_GT_S_CLKPAD);
            self.egrid
                .add_tile_single_id(cell_n, defs::virtex2::tcls::INTF_GT_N_CLKPAD);
            let n = match self.chip.kind {
                ChipKind::Virtex2P => 4,
                ChipKind::Virtex2PX => 8,
                _ => unreachable!(),
            };
            for d in 0..n {
                let cell = cell_s.delta(0, 1 + d);
                self.egrid
                    .add_tile_single_id(cell, defs::virtex2::tcls::INT_PPC);
                self.egrid.add_tile_single_id(
                    cell,
                    if d % 4 == 0 {
                        defs::virtex2::tcls::INTF_GT_S0
                    } else {
                        defs::virtex2::tcls::INTF_GT_S123
                    },
                );
            }
            for d in 0..n {
                let cell = cell_n.delta(0, -n + d);
                self.egrid
                    .add_tile_single_id(cell, defs::virtex2::tcls::INT_PPC);
                self.egrid.add_tile_single_id(
                    cell,
                    if d % 4 == 0 {
                        defs::virtex2::tcls::INTF_GT_N0
                    } else {
                        defs::virtex2::tcls::INTF_GT_N123
                    },
                );
            }
            if self.chip.kind == ChipKind::Virtex2P {
                self.egrid.add_tile_id(
                    cell_s,
                    defs::virtex2::tcls::GIGABIT_S,
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
                    defs::virtex2::tcls::GIGABIT_N,
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
                    defs::virtex2::tcls::GIGABIT10_S,
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
                    defs::virtex2::tcls::GIGABIT10_N,
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
            while !self.egrid[cell_s].tiles.contains_id(defs::tslots::INT) {
                cell_s.row -= 1;
            }
            while !self.egrid[cell_n].tiles.contains_id(defs::tslots::INT) {
                cell_n.row += 1;
            }
            let mut term_s = defs::spartan3::ccls::LLV_S;
            let mut term_n = defs::spartan3::ccls::LLV_N;
            if self.chip.is_col_io(cell.col) && self.chip.kind != ChipKind::Spartan3A {
                term_s = defs::spartan3::ccls::LLV_CLK_WE_S3E_S;
                term_n = defs::spartan3::ccls::LLV_CLK_WE_S3E_N;
            }
            self.egrid.fill_conn_pair_id(cell_s, cell_n, term_n, term_s);
            self.egrid.add_tile_id(
                cell_n,
                if self.chip.kind.is_spartan3a() {
                    defs::spartan3::tcls::LLV_S3A
                } else {
                    defs::spartan3::tcls::LLV_S3E
                },
                &[cell_s, cell_n],
            );
        }
    }

    fn fill_llh(&mut self) {
        for cell in self.egrid.column(self.die, self.chip.col_clk) {
            let mut cell_w = cell.delta(-1, 0);
            let mut cell_e = cell;
            while !self.egrid[cell_w].tiles.contains_id(defs::tslots::INT) {
                cell_w.col -= 1;
            }
            while !self.egrid[cell_e].tiles.contains_id(defs::tslots::INT) {
                cell_e.col += 1;
            }
            let mut term_w = defs::spartan3::ccls::LLH_W;
            let mut term_e = defs::spartan3::ccls::LLH_E;
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
                term_w = defs::spartan3::ccls::LLH_DCM_S3ADSP_W;
                term_e = defs::spartan3::ccls::LLH_DCM_S3ADSP_E;
            }
            self.egrid.fill_conn_pair_id(cell_w, cell_e, term_e, term_w);
            self.egrid.add_tile_id(
                cell_e,
                if self.chip.kind.is_spartan3a() && cell.row == self.chip.row_s() {
                    defs::spartan3::tcls::LLH_S_S3A
                } else if self.chip.kind.is_spartan3a() && cell.row == self.chip.row_n() {
                    defs::spartan3::tcls::LLH_N_S3A
                } else {
                    defs::spartan3::tcls::LLH
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
                    defs::spartan3::ccls::CLK_WE_S3E_N,
                    defs::spartan3::ccls::CLK_WE_S3E_S,
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
                        defs::spartan3::ccls::BRKH_S3_N,
                        defs::spartan3::ccls::BRKH_S3_S,
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
                            defs::spartan3::ccls::DSPHOLE_E,
                            defs::spartan3::ccls::DSPHOLE_W,
                        );
                    }
                }
            }
            for col in [self.chip.col_w() + 3, self.chip.col_e() - 6] {
                for row in [self.chip.row_mid() - 1, self.chip.row_mid()] {
                    self.egrid.fill_conn_pair_id(
                        CellCoord::new(self.die, col, row),
                        CellCoord::new(self.die, col + 4, row),
                        defs::spartan3::ccls::DSPHOLE_E,
                        defs::spartan3::ccls::DSPHOLE_W,
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
                        defs::spartan3::ccls::HDCM_E,
                        defs::spartan3::ccls::HDCM_W,
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
            pass_w = defs::virtex2::ccls::PASS_W;
            pass_e = defs::virtex2::ccls::PASS_E;
            pass_s = defs::virtex2::ccls::PASS_S;
            pass_n = defs::virtex2::ccls::PASS_N;
        } else if self.chip.kind == ChipKind::FpgaCore {
            pass_w = defs::spartan3::ccls::PASS_W_FC;
            pass_e = defs::spartan3::ccls::PASS_E;
            pass_s = defs::spartan3::ccls::PASS_S_FC;
            pass_n = defs::spartan3::ccls::PASS_N;
        } else {
            pass_w = defs::spartan3::ccls::PASS_W;
            pass_e = defs::spartan3::ccls::PASS_E;
            pass_s = defs::spartan3::ccls::PASS_S;
            pass_n = defs::spartan3::ccls::PASS_N;
        }
        let slot_w = defs::cslots::W;
        let slot_e = defs::cslots::E;
        let slot_s = defs::cslots::S;
        let slot_n = defs::cslots::N;
        let die = DieId::from_idx(0);
        // horizontal
        for row in self.egrid.rows(die) {
            let mut prev = None;
            for cell in self.egrid.row(die, row) {
                if !self.egrid[cell].tiles.contains_id(defs::tslots::INT) {
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
                if !self.egrid[cell].tiles.contains_id(defs::tslots::INT) {
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
            let slot_n = defs::cslots::N;
            let slot_s = defs::cslots::S;
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
            ChipKind::Virtex2 => (defs::virtex2::tcls::CLK_S_V2, defs::virtex2::tcls::CLK_N_V2),
            ChipKind::Virtex2P => (
                defs::virtex2::tcls::CLK_S_V2P,
                defs::virtex2::tcls::CLK_N_V2P,
            ),
            ChipKind::Virtex2PX => (
                defs::virtex2::tcls::CLK_S_V2PX,
                defs::virtex2::tcls::CLK_N_V2PX,
            ),
            _ => unreachable!(),
        };
        {
            let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_s());
            self.egrid
                .add_tile_id(cell, kind_b, &[cell.delta(-1, 0), cell]);
        }
        {
            let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_n());
            self.egrid
                .add_tile_id(cell, kind_t, &[cell.delta(-1, 0), cell]);
        }

        {
            let cell = CellCoord::new(self.die, self.chip.col_w(), self.chip.row_pci.unwrap());
            self.egrid
                .add_tile_sn_id(cell, defs::virtex2::tcls::PCI_W, 2, 4);
        }
        {
            let cell = CellCoord::new(self.die, self.chip.col_e(), self.chip.row_pci.unwrap());
            self.egrid
                .add_tile_sn_id(cell, defs::virtex2::tcls::PCI_E, 2, 4);
        }
    }

    fn fill_clkbt_s3(&mut self) {
        let (clkb, clkt) = match self.chip.kind {
            ChipKind::Spartan3 => (
                defs::spartan3::tcls::CLK_S_S3,
                defs::spartan3::tcls::CLK_N_S3,
            ),
            ChipKind::FpgaCore => (
                defs::spartan3::tcls::CLK_S_FC,
                defs::spartan3::tcls::CLK_N_FC,
            ),
            ChipKind::Spartan3E => (
                defs::spartan3::tcls::CLK_S_S3E,
                defs::spartan3::tcls::CLK_N_S3E,
            ),
            ChipKind::Spartan3A | ChipKind::Spartan3ADsp => (
                defs::spartan3::tcls::CLK_S_S3A,
                defs::spartan3::tcls::CLK_N_S3A,
            ),
            _ => unreachable!(),
        };
        {
            let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_s());
            self.egrid.add_tile_id(cell, clkb, &[cell.delta(-1, 0)]);
        }
        {
            let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_n());
            self.egrid.add_tile_id(cell, clkt, &[cell.delta(-1, 0)]);
        }
    }

    fn fill_clklr_s3e(&mut self) {
        {
            let cell = CellCoord::new(self.die, self.chip.col_w(), self.chip.row_mid());
            self.egrid.add_tile_id(
                cell,
                if self.chip.kind == ChipKind::Spartan3E {
                    defs::spartan3::tcls::CLK_W_S3E
                } else {
                    defs::spartan3::tcls::CLK_W_S3A
                },
                &[cell.delta(0, -1), cell],
            );
        }
        {
            let cell = CellCoord::new(self.die, self.chip.col_e(), self.chip.row_mid());
            self.egrid.add_tile_id(
                cell,
                if self.chip.kind == ChipKind::Spartan3E {
                    defs::spartan3::tcls::CLK_E_S3E
                } else {
                    defs::spartan3::tcls::CLK_E_S3A
                },
                &[cell.delta(0, -1), cell],
            );
        }
    }

    fn fill_pci_ce(&mut self) {
        if self.chip.kind.is_spartan3ea() {
            for hv in DirHV::DIRS {
                self.egrid.add_tile_id(
                    self.chip.corner(hv).cell,
                    defs::spartan3::tcls::PCI_CE_CNR,
                    &[],
                );
            }

            for &(row, _, _) in &self.chip.rows_hclk {
                let kind = if row > self.chip.row_mid() {
                    defs::spartan3::tcls::PCI_CE_N
                } else {
                    defs::spartan3::tcls::PCI_CE_S
                };
                for col in [self.chip.col_w(), self.chip.col_e()] {
                    let cell = CellCoord::new(self.die, col, row);
                    self.egrid.add_tile_id(cell, kind, &[]);
                }
            }
            if self.chip.kind == ChipKind::Spartan3A
                && let Some((col_w, col_e)) = self.chip.cols_clkv
            {
                for row in [self.chip.row_s(), self.chip.row_n()] {
                    let cell = CellCoord::new(self.die, col_w, row);
                    self.egrid
                        .add_tile_id(cell, defs::spartan3::tcls::PCI_CE_E, &[]);
                    let cell = CellCoord::new(self.die, col_e, row);
                    self.egrid
                        .add_tile_id(cell, defs::spartan3::tcls::PCI_CE_W, &[]);
                }
            }
        }
    }

    fn fill_hclk(&mut self) {
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
                    self.egrid[cell].region_root[defs::rslots::LEAF] =
                        CellCoord::new(DieId::from_idx(0), col, row_m - 1);
                    self.egrid[cell].region_root[defs::rslots::HCLK] =
                        CellCoord::new(DieId::from_idx(0), col_q, row_q);
                }
                for row in row_m.range(row_t) {
                    let cell = CellCoord::new(self.die, col, row);
                    self.egrid[cell].region_root[defs::rslots::LEAF] =
                        CellCoord::new(DieId::from_idx(0), col, row_m);
                    self.egrid[cell].region_root[defs::rslots::HCLK] =
                        CellCoord::new(DieId::from_idx(0), col_q, row_q);
                }
                let kind = if matches!(self.chip.columns[col].kind, ColumnKind::BramCont(_)) {
                    if row_m == self.chip.row_mid() {
                        defs::spartan3::tcls::HCLK_UNI
                    } else if i == 0 {
                        if self.chip.kind == ChipKind::Spartan3E {
                            defs::spartan3::tcls::HCLK_S
                        } else {
                            defs::spartan3::tcls::HCLK_UNI_S
                        }
                    } else if i == self.chip.rows_hclk.len() - 1 {
                        if self.chip.kind == ChipKind::Spartan3E {
                            defs::spartan3::tcls::HCLK_N
                        } else {
                            defs::spartan3::tcls::HCLK_UNI_N
                        }
                    } else {
                        defs::spartan3::tcls::HCLK_0
                    }
                } else if !self.chip.kind.is_virtex2() {
                    defs::spartan3::tcls::HCLK
                } else {
                    defs::virtex2::tcls::HCLK
                };
                let cell = CellCoord::new(self.die, col, row_m);
                self.egrid
                    .add_tile_id(cell, kind, &[cell.delta(0, -1), cell]);
                if self.chip.columns[col].kind == ColumnKind::Dsp {
                    self.egrid
                        .add_tile_id(cell, defs::spartan3::tcls::HCLK_DSP, &[]);
                }
            }
        }
    }

    fn fill_hrow(&mut self) {
        for &(row_m, _, _) in &self.chip.rows_hclk {
            if self.chip.kind.is_virtex2() {
                let kind = if row_m == self.chip.row_s() + 1 {
                    defs::virtex2::tcls::HROW_S
                } else if row_m == self.chip.row_n() {
                    defs::virtex2::tcls::HROW_N
                } else {
                    defs::virtex2::tcls::HROW
                };
                self.egrid.add_tile_id(
                    CellCoord::new(self.die, self.chip.col_clk, row_m),
                    kind,
                    &[],
                );
            } else if let Some((col_cl, col_cr)) = self.chip.cols_clkv {
                for col in [col_cl, col_cr] {
                    self.egrid.add_tile_id(
                        CellCoord::new(self.die, col, row_m),
                        defs::spartan3::tcls::HROW,
                        &[],
                    );
                }
            }
        }
    }

    fn fill_clkc(&mut self) {
        let kind = if self.chip.kind.is_virtex2() {
            defs::virtex2::tcls::CLKC
        } else if self.chip.cols_clkv.is_none() {
            defs::spartan3::tcls::CLKC_50A
        } else {
            defs::spartan3::tcls::CLKC
        };
        self.egrid.add_tile_id(
            CellCoord::new(self.die, self.chip.col_clk, self.chip.row_mid()),
            kind,
            &[],
        );
    }

    fn fill_clkqc(&mut self) {
        if let Some((col_cl, col_cr)) = self.chip.cols_clkv {
            if matches!(self.chip.kind, ChipKind::Spartan3 | ChipKind::FpgaCore) {
                self.egrid.add_tile_id(
                    CellCoord::new(self.die, col_cl, self.chip.row_mid()),
                    defs::spartan3::tcls::CLKQC_S3,
                    &[],
                );
                self.egrid.add_tile_id(
                    CellCoord::new(self.die, col_cr, self.chip.row_mid()),
                    defs::spartan3::tcls::CLKQC_S3,
                    &[],
                );
            } else {
                self.egrid.add_tile_id(
                    CellCoord::new(self.die, col_cl, self.chip.row_mid()),
                    defs::spartan3::tcls::CLKQC_S3E,
                    &[],
                );
                self.egrid.add_tile_id(
                    CellCoord::new(self.die, col_cr, self.chip.row_mid()),
                    defs::spartan3::tcls::CLKQC_S3E,
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
            expander.fill_clkbt_v2();
        } else if matches!(self.kind, ChipKind::Spartan3 | ChipKind::FpgaCore) {
            expander.fill_clkbt_s3();
        } else {
            expander.fill_clkbt_s3();
            expander.fill_clklr_s3e();
        }
        expander.fill_pci_ce();
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
