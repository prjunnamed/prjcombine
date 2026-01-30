use std::{cmp::Ordering, collections::HashSet};

use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};
use prjcombine_interconnect::grid::{CellCoord, ColId, DieId, EdgeIoCoord, RowId};
use prjcombine_re_xilinx_naming::{
    db::NamingDb,
    grid::{BelGrid, ExpandedGridNaming},
};
use prjcombine_virtex2::{
    chip::{Chip, ChipKind, ColumnIoKind, ColumnKind, DcmPairKind, RowIoKind},
    defs,
    defs::spartan3::ccls as ccls_s3,
    defs::spartan3::tcls as tcls_s3,
    defs::virtex2::ccls as ccls_v2,
    defs::virtex2::tcls as tcls_v2,
    expanded::ExpandedDevice,
    iob::{IobKind, get_iob_data_e, get_iob_data_n, get_iob_data_s, get_iob_data_w},
};

pub struct ExpandedNamedDevice<'a> {
    pub edev: &'a ExpandedDevice<'a>,
    pub ngrid: ExpandedGridNaming<'a>,
    pub chip: &'a Chip,
}

impl<'a> ExpandedNamedDevice<'a> {
    pub fn get_io_name(&'a self, io: EdgeIoCoord) -> &'a str {
        let bel = self.chip.get_io_loc(io);
        self.ngrid.get_bel_name(bel).unwrap()
    }
}

struct Namer<'a> {
    edev: &'a ExpandedDevice<'a>,
    chip: &'a Chip,
    die: DieId,
    ngrid: ExpandedGridNaming<'a>,
    xlut: EntityVec<ColId, usize>,
    sxlut: EntityPartVec<ColId, usize>,
    dcm_grid: BelGrid,
    bram_grid: BelGrid,
    gtxlut: EntityPartVec<ColId, usize>,
    vcc_xlut: EntityVec<ColId, usize>,
    vcc_ylut: EntityVec<RowId, usize>,
    clut: EntityVec<ColId, usize>,
    rlut: EntityVec<RowId, usize>,
    hclklut: EntityPartVec<RowId, usize>,
    bramclut: EntityVec<ColId, usize>,
    rows_brk: HashSet<RowId>,
    ctr_pad: usize,
    ctr_nopad: usize,
}

impl Namer<'_> {
    fn fill_xlut(&mut self) {
        let mut x = 0;
        let mut sx = 0;
        for (col, &cd) in &self.chip.columns {
            self.xlut.push(x);
            if cd.kind == ColumnKind::Dsp {
                x += 2;
            } else {
                x += 1;
            }
            if cd.kind == ColumnKind::Clb
                || (cd.kind != ColumnKind::Io && self.chip.kind == ChipKind::Spartan3E)
            {
                self.sxlut.insert(col, sx);
                sx += 2;
            }
        }
    }

    fn fill_gtxlut(&mut self) {
        for (i, col) in self.chip.cols_gt.keys().copied().enumerate() {
            self.gtxlut.insert(col, i);
        }
    }

    fn fill_clut(&mut self) {
        let mut c = 0;
        let mut bramc = 1;
        for &cd in self.chip.columns.values() {
            self.clut.push(c);
            self.bramclut.push(bramc);
            if cd.kind == ColumnKind::Bram {
                bramc += 1;
            } else {
                c += 1;
            }
        }
    }

    fn fill_rlut(&mut self) {
        let n = self.chip.rows.len();
        for row in self.edev.rows(self.die) {
            self.rlut.push(n - row.to_idx() - 1);
        }
    }

    fn fill_hclklut(&mut self) {
        for (i, &(row_m, _, _)) in self.chip.rows_hclk.iter().enumerate() {
            self.hclklut.insert(row_m, i);
        }
    }

    fn fill_rows_brk(&mut self) {
        for &(_, _, r) in &self.chip.rows_hclk {
            self.rows_brk.insert(r - 1);
        }
        self.rows_brk.remove(&self.chip.row_n());
        if self.chip.kind != ChipKind::Spartan3ADsp {
            self.rows_brk.remove(&(self.chip.row_mid() - 1));
        }
    }

    fn fill_vcc_lut(&mut self) {
        let mut xtmp = 0;
        if matches!(
            self.chip.kind,
            ChipKind::Spartan3E | ChipKind::Spartan3A | ChipKind::Spartan3ADsp
        ) {
            xtmp += 1;
        }
        for col in self.chip.columns.ids() {
            self.vcc_xlut.push(xtmp);
            if col == self.chip.col_clk - 1 {
                xtmp += 2;
            } else {
                xtmp += 1;
            }
        }
        xtmp = 0;
        if matches!(
            self.chip.kind,
            ChipKind::Spartan3E | ChipKind::Spartan3A | ChipKind::Spartan3ADsp
        ) {
            xtmp += 1;
        }
        for row in self.chip.rows.ids() {
            self.vcc_ylut.push(xtmp);
            if row == self.chip.row_mid() - 1
                && matches!(
                    self.chip.kind,
                    ChipKind::Spartan3E | ChipKind::Spartan3A | ChipKind::Spartan3ADsp
                )
            {
                xtmp += 2;
            } else {
                xtmp += 1;
            }
        }
    }

    fn get_int_name(&self, cell: CellCoord) -> (&'static str, String) {
        let x = self.xlut[cell.col];
        let y = cell.row.to_idx();

        for &(bc, br) in &self.chip.holes_ppc {
            if cell.col >= bc && cell.col < bc + 10 && cell.row >= br && cell.row < br + 16 {
                let naming = if cell.col == bc + 9 {
                    "INT_PPC_E"
                } else if cell.row == br {
                    "INT_PPC_S"
                } else if cell.row == br + 15 {
                    "INT_PPC_N"
                } else if cell.col == bc {
                    "INT_PPC_W"
                } else {
                    unreachable!();
                };
                let prefix = if cell.col == bc && cell.row == br + 1 {
                    "PTERMLL"
                } else if cell.col == bc && cell.row == br + 14 {
                    "PTERMUL"
                } else {
                    ""
                };
                let r = self.rlut[cell.row];
                let name = if self.chip.columns[cell.col].kind == ColumnKind::Clb {
                    let c = self.clut[cell.col];
                    format!("{prefix}R{r}C{c}")
                } else {
                    let c = self.bramclut[cell.col];
                    format!("PPCINTR{r}BRAMC{c}")
                };
                return (naming, name);
            }
        }
        for pair in self.chip.get_dcm_pairs() {
            match pair.kind {
                DcmPairKind::S => {
                    if cell == pair.cell.delta(-1, 0) {
                        return ("INT_DCM_S3E", format!("DCM_BL_CENTER_X{x}Y{y}"));
                    }
                    if cell == pair.cell {
                        return ("INT_DCM_S3E", format!("DCM_BR_CENTER_X{x}Y{y}"));
                    }
                }
                DcmPairKind::SingleS => {
                    if cell == pair.cell.delta(-1, 0) {
                        return ("INT_DCM_S3E_DUMMY", format!("DCMAUX_BL_CENTER_X{x}Y{y}"));
                    }
                    if cell == pair.cell {
                        return ("INT_DCM_S3E", format!("DCM_BR_CENTER_X{x}Y{y}"));
                    }
                }
                DcmPairKind::N => {
                    if cell == pair.cell.delta(-1, 0) {
                        return ("INT_DCM_S3E", format!("DCM_TL_CENTER_X{x}Y{y}"));
                    }
                    if cell == pair.cell {
                        return ("INT_DCM_S3E", format!("DCM_TR_CENTER_X{x}Y{y}"));
                    }
                }
                DcmPairKind::SingleN => {
                    if cell == pair.cell.delta(-1, 0) {
                        return ("INT_DCM_S3E_DUMMY", format!("DCMAUX_TL_CENTER_X{x}Y{y}"));
                    }
                    if cell == pair.cell {
                        return ("INT_DCM_S3E", format!("DCM_TR_CENTER_X{x}Y{y}"));
                    }
                }
                DcmPairKind::W => {
                    if cell == pair.cell.delta(0, -1) {
                        return ("INT_DCM_S3E_H", format!("DCM_H_BL_CENTER_X{x}Y{y}"));
                    }
                    if cell == pair.cell {
                        return ("INT_DCM_S3E_H", format!("DCM_H_TL_CENTER_X{x}Y{y}"));
                    }
                }
                DcmPairKind::E => {
                    if cell == pair.cell.delta(0, -1) {
                        return ("INT_DCM_S3E_H", format!("DCM_H_BR_CENTER_X{x}Y{y}"));
                    }
                    if cell == pair.cell {
                        return ("INT_DCM_S3E_H", format!("DCM_H_TR_CENTER_X{x}Y{y}"));
                    }
                }
                DcmPairKind::Bram => {
                    if cell == pair.cell.delta(0, -1) {
                        return ("INT_DCM_S3E_H", format!("DCM_BGAP_X{x}Y{y}"));
                    }
                    if cell == pair.cell {
                        return ("INT_DCM_S3E_H", format!("DCM_SPLY_X{x}Y{y}"));
                    }
                }
            }
        }

        if self.chip.is_col_io(cell.col) && self.chip.is_row_io(cell.row) {
            if self.chip.kind.is_spartan3ea() {
                let ul = if cell.row == self.chip.row_s() {
                    'L'
                } else {
                    'U'
                };
                let lr = if cell.col == self.chip.col_w() {
                    'L'
                } else {
                    'R'
                };
                ("INT_CNR", format!("{ul}{lr}_X{x}Y{y}"))
            } else {
                let bt = if cell.row == self.chip.row_s() {
                    'B'
                } else {
                    'T'
                };
                let lr = if cell.col == self.chip.col_w() {
                    'L'
                } else {
                    'R'
                };
                if self.chip.kind.is_virtex2p() {
                    ("INT_CNR", format!("{lr}IOI{bt}IOI"))
                } else if self.chip.kind == ChipKind::FpgaCore {
                    ("INT_CNR_FC", format!("{bt}{lr}"))
                } else {
                    ("INT_CNR", format!("{bt}{lr}"))
                }
            }
        } else if self.chip.is_row_io(cell.row)
            && !self.chip.kind.is_spartan3ea()
            && matches!(self.chip.columns[cell.col].kind, ColumnKind::Bram)
        {
            let bt = if cell.row == self.chip.row_s() {
                'B'
            } else {
                'T'
            };
            let c = self.bramclut[cell.col];
            let naming = match self.chip.kind {
                ChipKind::Virtex2 => "INT_DCM_V2",
                ChipKind::Virtex2P | ChipKind::Virtex2PX => {
                    if self.chip.cols_gt.contains_key(&cell.col) {
                        "INT_GT_CLKPAD"
                    } else {
                        "INT_DCM_V2P"
                    }
                }
                ChipKind::Spartan3 => {
                    if cell.col == self.chip.col_w() + 3 || cell.col == self.chip.col_e() - 3 {
                        "INT_DCM_S3"
                    } else {
                        "INT_DCM_S3_DUMMY"
                    }
                }
                _ => unreachable!(),
            };
            (naming, format!("{bt}IOIBRAMC{c}"))
        } else if self.chip.bram_row(cell.row).is_some()
            && matches!(self.chip.columns[cell.col].kind, ColumnKind::Bram)
        {
            // BRAM
            if !self.chip.kind.is_spartan3ea() {
                let c = self.bramclut[cell.col];
                let r = self.rlut[cell.row];

                let is_gt = self.chip.cols_gt.contains_key(&cell.col)
                    && self.chip.kind == ChipKind::Virtex2P
                    && (cell.row < self.chip.row_s() + 5 || cell.row >= self.chip.row_n() - 4);
                let is_gt10 = self.chip.cols_gt.contains_key(&cell.col)
                    && self.chip.kind == ChipKind::Virtex2PX
                    && (cell.row < self.chip.row_s() + 9 || cell.row >= self.chip.row_n() - 8);
                (
                    if is_gt || is_gt10 {
                        "INT_GT"
                    } else {
                        "INT_BRAM"
                    },
                    format!("BRAMR{r}C{c}"),
                )
            } else {
                let idx = self.chip.bram_row(cell.row).unwrap();
                let naming = if self.chip.kind == ChipKind::Spartan3ADsp {
                    if self.rows_brk.contains(&cell.row) {
                        "INT_BRAM_S3ADSP_BRK"
                    } else {
                        "INT_BRAM_S3ADSP"
                    }
                } else {
                    if self.rows_brk.contains(&cell.row) {
                        "INT_BRAM_BRK"
                    } else {
                        "INT_BRAM"
                    }
                };
                let mut md = "";
                if self.rows_brk.contains(&cell.row) {
                    md = "_BRK";
                }
                if self.chip.kind != ChipKind::Spartan3E {
                    if cell.row == self.chip.row_s() + 1 {
                        md = "_BOT";
                    }
                    if cell.row == self.chip.row_n() - 1 {
                        md = "_TOP";
                    }
                    if self.chip.cols_clkv.is_none() && cell.row == self.chip.row_n() - 5 {
                        md = "_TOP";
                    }
                }
                (naming, format!("BRAM{idx}_SMALL{md}_X{x}Y{y}"))
            }
        } else if self.chip.bram_row(cell.row).is_some()
            && matches!(self.chip.columns[cell.col].kind, ColumnKind::Dsp)
        {
            // DSP
            let idx = self.chip.bram_row(cell.row).unwrap();
            let naming = if self.rows_brk.contains(&cell.row) {
                "INT_MACC_BRK"
            } else {
                "INT_MACC"
            };
            let mut md = "";
            if self.rows_brk.contains(&cell.row) {
                md = "_BRK";
            }
            if self.chip.kind != ChipKind::Spartan3E {
                if cell.row == self.chip.row_s() + 1 {
                    md = "_BOT";
                }
                if cell.row == self.chip.row_n() - 1 {
                    md = "_TOP";
                }
                if self.chip.cols_clkv.is_none() && cell.row == self.chip.row_n() - 5 {
                    md = "_TOP";
                }
            }
            (naming, format!("MACC{idx}_SMALL{md}_X{x}Y{y}"))
        } else if self.chip.is_row_io(cell.row) {
            match self.chip.kind {
                ChipKind::Virtex2
                | ChipKind::Virtex2P
                | ChipKind::Virtex2PX
                | ChipKind::Spartan3
                | ChipKind::FpgaCore => {
                    let bt = if cell.row == self.chip.row_s() {
                        'B'
                    } else {
                        'T'
                    };
                    let c = self.clut[cell.col];
                    let naming = if self.chip.kind.is_virtex2() {
                        if self.chip.kind == ChipKind::Virtex2PX
                            && cell.col == self.chip.col_clk - 1
                        {
                            if cell.row == self.chip.row_s() {
                                "INT_IOI_CLK_S"
                            } else {
                                "INT_IOI_CLK_N"
                            }
                        } else {
                            "INT_IOI_SN"
                        }
                    } else if self.chip.kind == ChipKind::FpgaCore {
                        "INT_IOI_FC"
                    } else {
                        "INT_IOI"
                    };
                    (naming, format!("{bt}IOIC{c}"))
                }
                ChipKind::Spartan3E | ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                    let naming = if self.chip.kind.is_spartan3a() {
                        "INT_IOI_S3A_SN"
                    } else {
                        "INT_IOI"
                    };
                    let (data, tidx) = if cell.row == self.chip.row_s() {
                        get_iob_data_s(self.chip.kind, self.chip.columns[cell.col].io)
                    } else {
                        get_iob_data_n(self.chip.kind, self.chip.columns[cell.col].io)
                    };
                    let has_iobs = data
                        .iobs
                        .iter()
                        .any(|iob| iob.tile == tidx && iob.kind == IobKind::Iob);
                    let has_ibufs = data
                        .iobs
                        .iter()
                        .any(|iob| iob.tile == tidx && iob.kind == IobKind::Ibuf);
                    let kind = if !has_ibufs {
                        if cell.row == self.chip.row_s() {
                            "BIOIS"
                        } else {
                            "TIOIS"
                        }
                    } else if !has_iobs {
                        if cell.row == self.chip.row_s() {
                            "BIBUFS"
                        } else {
                            "TIBUFS"
                        }
                    } else {
                        if cell.row == self.chip.row_s() {
                            "BIOIB"
                        } else {
                            "TIOIB"
                        }
                    };
                    let name = format!("{kind}_X{x}Y{y}");
                    (naming, name)
                }
            }
        } else if self.chip.is_col_io(cell.col) {
            match self.chip.kind {
                ChipKind::Virtex2
                | ChipKind::Virtex2P
                | ChipKind::Virtex2PX
                | ChipKind::Spartan3
                | ChipKind::FpgaCore => {
                    let lr = if cell.col == self.chip.col_w() {
                        'L'
                    } else {
                        'R'
                    };
                    let r = self.rlut[cell.row];
                    let naming = if self.chip.kind.is_virtex2() {
                        "INT_IOI_WE"
                    } else if self.chip.kind == ChipKind::FpgaCore {
                        "INT_IOI_FC"
                    } else {
                        "INT_IOI"
                    };
                    (naming, format!("{lr}IOIR{r}"))
                }
                ChipKind::Spartan3E | ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                    let naming = if self.chip.kind.is_spartan3a() {
                        if self.rows_brk.contains(&cell.row) {
                            "INT_IOI_S3A_WE_BRK"
                        } else {
                            "INT_IOI_S3A_WE"
                        }
                    } else {
                        if self.rows_brk.contains(&cell.row) {
                            "INT_IOI_BRK"
                        } else {
                            "INT_IOI"
                        }
                    };
                    let (data, tidx) = if cell.col == self.chip.col_w() {
                        get_iob_data_w(self.chip.kind, self.chip.rows[cell.row])
                    } else {
                        get_iob_data_e(self.chip.kind, self.chip.rows[cell.row])
                    };
                    let has_ibufs = data
                        .iobs
                        .iter()
                        .any(|iob| iob.tile == tidx && iob.kind == IobKind::Ibuf);
                    let kind = if !has_ibufs {
                        if cell.col == self.chip.col_w() {
                            "LIOIS"
                        } else {
                            "RIOIS"
                        }
                    } else {
                        if cell.col == self.chip.col_w() {
                            "LIBUFS"
                        } else {
                            "RIBUFS"
                        }
                    };
                    let brk = if self.rows_brk.contains(&cell.row) {
                        "_BRK"
                    } else {
                        ""
                    };
                    let clk =
                        if cell.row == self.chip.row_mid() - 1 || cell.row == self.chip.row_mid() {
                            "_CLK"
                        } else {
                            ""
                        };
                    let pci = if cell.row >= self.chip.row_mid() - 4
                        && cell.row < self.chip.row_mid() + 4
                    {
                        "_PCI"
                    } else {
                        ""
                    };
                    let name = format!("{kind}{clk}{pci}{brk}_X{x}Y{y}");
                    (naming, name)
                }
            }
        } else {
            for &hole in &self.edev.holes {
                assert!(!hole.contains(cell));
            }
            if !self.chip.kind.is_spartan3ea() {
                let r = self.rlut[cell.row];
                let c = self.clut[cell.col];
                if self.chip.kind == ChipKind::FpgaCore {
                    ("INT_CLB_FC", format!("R{r}C{c}"))
                } else {
                    ("INT_CLB", format!("R{r}C{c}"))
                }
            } else {
                let naming = if self.rows_brk.contains(&cell.row) {
                    "INT_CLB_BRK"
                } else {
                    "INT_CLB"
                };
                (naming, format!("CLB_X{x}Y{y}"))
            }
        }
    }

    fn get_lterm_name(&self, row: RowId) -> (&'static str, String) {
        let x = self.xlut[self.chip.col_w()];
        let y = row.to_idx();
        if row == self.chip.row_s() {
            if !self.chip.kind.is_spartan3ea() {
                if !self.chip.kind.is_virtex2p() {
                    ("TERM_W", "LBTERM".into())
                } else {
                    ("TERM_W", "LTERMBIOI".into())
                }
            } else {
                ("TERM_W", format!("CNR_LBTERM_X{x}Y{y}"))
            }
        } else if row == self.chip.row_n() {
            if !self.chip.kind.is_spartan3ea() {
                if !self.chip.kind.is_virtex2p() {
                    ("TERM_W", "LTTERM".into())
                } else {
                    ("TERM_W", "LTERMTIOI".into())
                }
            } else {
                ("TERM_W", format!("CNR_LTTERM_X{x}Y{y}"))
            }
        } else {
            if !self.chip.kind.is_spartan3ea() {
                let r = self.rlut[row];
                let naming = if self.chip.kind.is_virtex2() {
                    if row < self.chip.row_pci.unwrap() {
                        "TERM_W_D"
                    } else {
                        "TERM_W_U"
                    }
                } else {
                    "TERM_W"
                };
                (naming, format!("LTERMR{r}"))
            } else {
                let mut kind = match self.chip.rows[row] {
                    RowIoKind::Single => "LTERM1",
                    RowIoKind::Double(0) => "LTERM2",
                    RowIoKind::Triple(0) => "LTERM3",
                    RowIoKind::Quad(0) => "LTERM4",
                    _ => "LTERM",
                };
                if self.chip.kind == ChipKind::Spartan3E {
                    if row == self.chip.row_mid() {
                        kind = "LTERM4CLK";
                    }
                    if row == self.chip.row_mid() - 4 {
                        kind = "LTERM4B";
                    }
                    if row == self.chip.row_mid() - 3 {
                        kind = "LTERMCLKA";
                    }
                    if row == self.chip.row_mid() - 1 {
                        kind = "LTERMCLK";
                    }
                    if row == self.chip.row_mid() + 1 {
                        kind = "LTERMCLKA";
                    }
                    if row == self.chip.row_mid() + 3 {
                        kind = "LTERMCLK";
                    }
                } else {
                    if row == self.chip.row_mid() {
                        kind = "LTERM4CLK";
                    }
                    if row == self.chip.row_mid() - 4 {
                        kind = "LTERM4B";
                    }
                    if row == self.chip.row_mid() - 2 {
                        kind = "LTERMCLKA";
                    }
                    if row == self.chip.row_mid() - 1 {
                        kind = "LTERMCLK";
                    }
                    if row == self.chip.row_mid() + 1 {
                        kind = "LTERMCLKA";
                    }
                    if row == self.chip.row_mid() + 2 {
                        kind = "LTERMCLK";
                    }
                }
                ("TERM_W", format!("{kind}_X{x}Y{y}"))
            }
        }
    }

    fn get_rterm_name(&self, row: RowId) -> (&'static str, String) {
        let x = self.xlut[self.chip.col_e()];
        let y = row.to_idx();
        if row == self.chip.row_s() {
            if !self.chip.kind.is_spartan3ea() {
                if !self.chip.kind.is_virtex2p() {
                    ("TERM_E", "RBTERM".into())
                } else {
                    ("TERM_E", "RTERMBIOI".into())
                }
            } else {
                ("TERM_E", format!("CNR_RBTERM_X{x}Y{y}"))
            }
        } else if row == self.chip.row_n() {
            if !self.chip.kind.is_spartan3ea() {
                if !self.chip.kind.is_virtex2p() {
                    ("TERM_E", "RTTERM".into())
                } else {
                    ("TERM_E", "RTERMTIOI".into())
                }
            } else {
                ("TERM_E", format!("CNR_RTTERM_X{x}Y{y}"))
            }
        } else {
            if !self.chip.kind.is_spartan3ea() {
                let r = self.rlut[row];
                let naming = if self.chip.kind.is_virtex2() {
                    if row < self.chip.row_pci.unwrap() {
                        "TERM_E_D"
                    } else {
                        "TERM_E_U"
                    }
                } else {
                    "TERM_E"
                };
                (naming, format!("RTERMR{r}"))
            } else {
                let mut kind = match self.chip.rows[row] {
                    RowIoKind::Single => "RTERM1",
                    RowIoKind::Double(0) => "RTERM2",
                    RowIoKind::Triple(0) => "RTERM3",
                    RowIoKind::Quad(0) => "RTERM4",
                    _ => "RTERM",
                };
                if self.chip.kind == ChipKind::Spartan3E {
                    if row == self.chip.row_mid() {
                        kind = "RTERM4CLK";
                    }
                    if row == self.chip.row_mid() - 4 {
                        kind = "RTERM4CLKB";
                    }
                    if row == self.chip.row_mid() - 2 {
                        kind = "RTERMCLKA";
                    }
                    if row == self.chip.row_mid() + 2 {
                        kind = "RTERMCLKA";
                    }
                } else {
                    if row == self.chip.row_mid() {
                        kind = "RTERM4CLK";
                    }
                    if row == self.chip.row_mid() - 4 {
                        kind = "RTERM4B";
                    }
                    if row == self.chip.row_mid() - 3 {
                        kind = "RTERMCLKB";
                    }
                    if row == self.chip.row_mid() - 2 {
                        kind = "RTERMCLKA";
                    }
                    if row == self.chip.row_mid() + 1 {
                        kind = "RTERMCLKA";
                    }
                }
                ("TERM_E", format!("{kind}_X{x}Y{y}"))
            }
        }
    }

    fn get_bterm_name(&self, col: ColId) -> (&'static str, String) {
        let x = self.xlut[col];
        let y = self.chip.row_s().to_idx();
        if col == self.chip.col_w() {
            if !self.chip.kind.is_spartan3ea() {
                if self.chip.kind.is_virtex2p() {
                    ("TERM_S_CNR", "LIOIBTERM".into())
                } else if self.chip.kind == ChipKind::FpgaCore {
                    ("TERM_S_CNR_FC", "BLTERM".into())
                } else {
                    ("TERM_S_CNR", "BLTERM".into())
                }
            } else {
                ("TERM_S_CNR", format!("CNR_BTERM_X{x}Y{y}"))
            }
        } else if col == self.chip.col_e() {
            if !self.chip.kind.is_spartan3ea() {
                if self.chip.kind.is_virtex2p() {
                    ("TERM_S_CNR", "RIOIBTERM".into())
                } else if self.chip.kind == ChipKind::FpgaCore {
                    ("TERM_S_CNR_FC", "BRTERM".into())
                } else {
                    ("TERM_S_CNR", "BRTERM".into())
                }
            } else {
                ("TERM_S_CNR", format!("CNR_BTERM_X{x}Y{y}"))
            }
        } else if !self.chip.kind.is_spartan3ea() && self.chip.columns[col].kind == ColumnKind::Bram
        {
            let c = self.bramclut[col];
            ("TERM_S", format!("BTERMBRAMC{c}"))
        } else {
            if !self.chip.kind.is_spartan3ea() {
                let c = self.clut[col];
                ("TERM_S", format!("BTERMC{c}"))
            } else {
                let cd = &self.chip.columns[col];
                let mut kind = match cd.io {
                    ColumnIoKind::Single => "BTERM1",
                    ColumnIoKind::Double(0) => "BTERM2",
                    ColumnIoKind::Triple(0) => "BTERM3",
                    ColumnIoKind::Quad(0) => "BTERM4",
                    _ => "BTERM",
                };
                if self.chip.kind == ChipKind::Spartan3E {
                    if cd.io == ColumnIoKind::Quad(0) && cd.kind == ColumnKind::BramCont(2) {
                        kind = "BTERM4_BRAM2";
                    }
                    if col == self.chip.col_clk - 3 {
                        kind = "BTERMCLKA";
                    }
                    if col == self.chip.col_clk - 1 {
                        kind = "BTERMCLKB";
                    }
                    if col == self.chip.col_clk {
                        kind = "BTERM4CLK";
                    }
                    if col == self.chip.col_clk + 1 {
                        kind = "BTERMCLK";
                    }
                } else {
                    if col == self.chip.col_clk - 2 {
                        kind = "BTERM2CLK";
                    }
                    if col == self.chip.col_clk - 1 {
                        kind = "BTERMCLKB";
                    }
                    if col == self.chip.col_clk {
                        kind = "BTERM2CLK";
                    }
                    if col == self.chip.col_clk + 1 {
                        kind = "BTERMCLK";
                    }
                    if self.chip.kind == ChipKind::Spartan3ADsp {
                        match cd.kind {
                            ColumnKind::BramCont(2) => {
                                kind = "BTERM1";
                            }
                            ColumnKind::Dsp => {
                                kind = "BTERM1_MACC";
                            }
                            _ => (),
                        }
                    }
                }
                ("TERM_S", format!("{kind}_X{x}Y{y}"))
            }
        }
    }

    fn get_tterm_name(&self, col: ColId) -> (&'static str, String) {
        let x = self.xlut[col];
        let y = self.chip.row_n().to_idx();
        if col == self.chip.col_w() {
            if !self.chip.kind.is_spartan3ea() {
                if self.chip.kind.is_virtex2p() {
                    ("TERM_N_CNR", "LIOITTERM".into())
                } else if self.chip.kind == ChipKind::FpgaCore {
                    ("TERM_N_CNR_FC", "TLTERM".into())
                } else {
                    ("TERM_N_CNR", "TLTERM".into())
                }
            } else {
                ("TERM_N_CNR", format!("CNR_TTERM_X{x}Y{y}"))
            }
        } else if col == self.chip.col_e() {
            if !self.chip.kind.is_spartan3ea() {
                if self.chip.kind.is_virtex2p() {
                    ("TERM_N_CNR", "RIOITTERM".into())
                } else if self.chip.kind == ChipKind::FpgaCore {
                    ("TERM_N_CNR_FC", "TRTERM".into())
                } else {
                    ("TERM_N_CNR", "TRTERM".into())
                }
            } else {
                ("TERM_N_CNR", format!("CNR_TTERM_X{x}Y{y}"))
            }
        } else if !self.chip.kind.is_spartan3ea() && self.chip.columns[col].kind == ColumnKind::Bram
        {
            let c = self.bramclut[col];
            ("TERM_N", format!("TTERMBRAMC{c}"))
        } else {
            if !self.chip.kind.is_spartan3ea() {
                let c = self.clut[col];
                ("TERM_N", format!("TTERMC{c}"))
            } else {
                let cd = &self.chip.columns[col];
                let mut kind = match cd.io {
                    ColumnIoKind::Single => "TTERM1",
                    ColumnIoKind::Double(0) => "TTERM2",
                    ColumnIoKind::Triple(0) => "TTERM3",
                    ColumnIoKind::Quad(0) => "TTERM4",
                    _ => "TTERM",
                };
                if self.chip.kind == ChipKind::Spartan3E {
                    if cd.io == ColumnIoKind::Quad(0) && cd.kind == ColumnKind::BramCont(2) {
                        kind = "TTERM4_BRAM2";
                    }
                    if col == self.chip.col_clk - 2 {
                        kind = "TTERMCLK";
                    }
                    if col == self.chip.col_clk - 1 {
                        kind = "TTERMCLKA";
                    }
                    if col == self.chip.col_clk {
                        kind = "TTERM4CLK";
                    }
                    if col == self.chip.col_clk + 2 {
                        kind = "TTERMCLKA";
                    }
                } else {
                    if col == self.chip.col_clk - 2 {
                        kind = "TTERM2CLK";
                    }
                    if col == self.chip.col_clk - 1 {
                        kind = "TTERMCLKA";
                    }
                    if col == self.chip.col_clk {
                        kind = "TTERM2CLK";
                    }
                    if col == self.chip.col_clk + 1 {
                        kind = "TTERMCLKA";
                    }
                    if self.chip.kind == ChipKind::Spartan3ADsp {
                        match cd.kind {
                            ColumnKind::BramCont(2) => {
                                kind = "TTERM1";
                            }
                            ColumnKind::Dsp => {
                                kind = "TTERM1_MACC";
                            }
                            _ => (),
                        }
                    }
                }
                ("TERM_N", format!("{kind}_X{x}Y{y}"))
            }
        }
    }

    fn get_bram_name(&self, cell: CellCoord) -> (&'static str, String) {
        let is_bot = matches!(self.chip.kind, ChipKind::Spartan3A | ChipKind::Spartan3ADsp)
            && cell.row == self.chip.row_s() + 1;
        let is_top = matches!(self.chip.kind, ChipKind::Spartan3A | ChipKind::Spartan3ADsp)
            && (cell.row == self.chip.row_n() - 4
                || cell.row == self.chip.row_n() - 8 && cell.col == self.chip.col_clk);
        let is_brk = self.rows_brk.contains(&(cell.row + 3));
        let naming = match self.chip.kind {
            ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => "BRAM",
            ChipKind::Spartan3 => "BRAM_S3",
            ChipKind::FpgaCore => unreachable!(),
            ChipKind::Spartan3E => "BRAM_S3E",
            ChipKind::Spartan3A => {
                if is_bot {
                    "BRAM_S3A_BOT"
                } else if is_top {
                    "BRAM_S3A_TOP"
                } else {
                    "BRAM_S3A"
                }
            }
            ChipKind::Spartan3ADsp => "BRAM_S3ADSP",
        };
        let name = if self.chip.kind.is_spartan3ea() {
            let x = self.xlut[cell.col] + 1;
            let y = cell.row.to_idx();
            let m = if self.chip.kind == ChipKind::Spartan3ADsp {
                "_3M"
            } else {
                ""
            };
            if is_bot {
                format!("BRAMSITE2{m}_BOT_X{x}Y{y}")
            } else if is_top {
                format!("BRAMSITE2{m}_TOP_X{x}Y{y}")
            } else if is_brk {
                format!("BRAMSITE2{m}_BRK_X{x}Y{y}")
            } else {
                format!("BRAMSITE2{m}_X{x}Y{y}")
            }
        } else {
            let c = self.bramclut[cell.col];
            let r = self.rlut[cell.row];
            format!("BMR{r}C{c}")
        };
        (naming, name)
    }

    fn get_dsp_name(&self, cell: CellCoord) -> (&'static str, String) {
        let is_bot = cell.row == self.chip.row_s() + 1;
        let is_top = cell.row == self.chip.row_n() - 4;
        let is_brk = self.rows_brk.contains(&(cell.row + 3));
        let naming = if is_top { "DSP_TOP" } else { "DSP" };
        let x = self.xlut[cell.col] + 1;
        let y = cell.row.to_idx();
        let name = if is_bot {
            format!("MACCSITE2_BOT_X{x}Y{y}")
        } else if is_top {
            format!("MACCSITE2_TOP_X{x}Y{y}")
        } else if is_brk {
            format!("MACCSITE2_BRK_X{x}Y{y}")
        } else {
            format!("MACCSITE2_X{x}Y{y}")
        };
        (naming, name)
    }

    fn get_hclk_name(&self, cell: CellCoord) -> (&'static str, String) {
        if !self.chip.kind.is_spartan3ea() {
            let mut r = self.chip.rows_hclk.len() - self.hclklut[cell.row];
            if self.chip.columns[cell.col].kind == ColumnKind::Bram {
                let c = self.bramclut[cell.col];
                ("HCLK", format!("GCLKHR{r}BRAMC{c}"))
            } else {
                // *sigh*.
                if self.chip.kind == ChipKind::Virtex2 && self.chip.columns.len() == 12 {
                    r -= 1;
                }
                let c = self.clut[cell.col];
                if self.chip.columns[cell.col].kind == ColumnKind::Io
                    && self.chip.kind.is_virtex2p()
                {
                    if cell.col == self.chip.col_w() {
                        ("HCLK", format!("LIOICLKR{r}"))
                    } else {
                        ("HCLK", format!("RIOICLKR{r}"))
                    }
                } else {
                    ("HCLK", format!("GCLKHR{r}C{c}"))
                }
            }
        } else {
            let x = self.xlut[cell.col];
            let y = cell.row.to_idx() - 1;
            let mut naming = "HCLK";
            let kind = match self.chip.columns[cell.col].kind {
                ColumnKind::Io => match cell.row.cmp(&self.chip.row_mid()) {
                    Ordering::Less => "GCLKH_PCI_CE_S",
                    Ordering::Equal => "GCLKH_PCI_CE_S_50A",
                    Ordering::Greater => "GCLKH_PCI_CE_N",
                },
                ColumnKind::BramCont(x) => {
                    if cell.row == self.chip.row_mid() {
                        naming = "HCLK_BRAM";
                        [
                            "BRAMSITE2_DN_GCLKH",
                            "BRAM2_GCLKH_FEEDTHRU",
                            "BRAM2_GCLKH_FEEDTHRUA",
                        ][x as usize - 1]
                    } else if self.hclklut[cell.row] == 0 {
                        if self.chip.kind == ChipKind::Spartan3E {
                            naming = "HCLK_BRAM_S";
                            [
                                "BRAMSITE2_DN_GCLKH",
                                "BRAM2_DN_GCLKH_FEEDTHRU",
                                "BRAM2_DN_GCLKH_FEEDTHRUA",
                            ][x as usize - 1]
                        } else {
                            naming = "HCLK_BRAM_UNI_S";
                            [
                                "BRAMSITE2_DN_GCLKH",
                                "BRAM2_GCLKH_FEEDTHRU",
                                "BRAM2_GCLKH_FEEDTHRUA",
                            ][x as usize - 1]
                        }
                    } else if self.hclklut[cell.row] == self.chip.rows_hclk.len() - 1 {
                        naming = "HCLK_BRAM_N";
                        if self.chip.kind == ChipKind::Spartan3E {
                            [
                                "BRAMSITE2_UP_GCLKH",
                                "BRAM2_UP_GCLKH_FEEDTHRU",
                                "BRAM2_UP_GCLKH_FEEDTHRUA",
                            ][x as usize - 1]
                        } else {
                            [
                                "BRAMSITE2_UP_GCLKH",
                                "BRAM2_GCLKH_FEEDTHRU",
                                "BRAM2_GCLKH_FEEDTHRUA",
                            ][x as usize - 1]
                        }
                    } else {
                        naming = "HCLK_0";
                        if self.chip.kind == ChipKind::Spartan3E {
                            [
                                "BRAMSITE2_MID_GCLKH",
                                "BRAM2_MID_GCLKH_FEEDTHRU",
                                "BRAM2_MID_GCLKH_FEEDTHRUA",
                            ][x as usize - 1]
                        } else {
                            [
                                if self.chip.kind != ChipKind::Spartan3ADsp {
                                    "BRAMSITE2_GCLKH"
                                } else if cell.row < self.chip.row_mid() {
                                    "BRAMSITE2_DN_GCLKH"
                                } else {
                                    "BRAMSITE2_UP_GCLKH"
                                },
                                "BRAM2_GCLKH_FEEDTHRU",
                                "BRAM2_GCLKH_FEEDTHRUA",
                            ][x as usize - 1]
                        }
                    }
                }
                ColumnKind::Dsp => {
                    naming = "HCLK_DSP";
                    "GCLKH"
                }
                _ => "GCLKH",
            };
            (naming, format!("{kind}_X{x}Y{y}"))
        }
    }

    fn get_ppc_h_name(&self, cell: CellCoord) -> (String, String) {
        let (_, name_l) = self.get_int_name(cell);
        let r = self.rlut[cell.row];
        let c = self.bramclut[cell.col + 8];
        let name_r = format!("BMR{r}C{c}");
        (name_l, name_r)
    }

    fn get_ppc_v_name(&self, cell: CellCoord) -> (String, String) {
        let rb = self.rlut[cell.row + 1];
        let rt = self.rlut[cell.row + 14];
        if self.chip.columns[cell.col].kind == ColumnKind::Clb {
            let c = self.clut[cell.col];
            (format!("PTERMR{rb}C{c}"), format!("PTERMR{rt}C{c}"))
        } else {
            let c = self.bramclut[cell.col];
            (
                format!("PTERMBR{rb}BRAMC{c}"),
                format!("PTERMTR{rt}BRAMC{c}"),
            )
        }
    }

    fn get_llv_name(&self, col: ColId) -> (&'static str, String) {
        let naming = if col == self.chip.col_w() {
            "LLV_CLKL"
        } else if col == self.chip.col_e() {
            "LLV_CLKR"
        } else {
            "LLV"
        };
        let x = self.xlut[col];
        let y = self.chip.row_mid().to_idx() - 1;
        let mut name = if col == self.chip.col_w() {
            format!("CLKL_IOIS_LL_X{x}Y{y}")
        } else if col == self.chip.col_e() {
            format!("CLKR_IOIS_LL_X{x}Y{y}")
        } else {
            format!("CLKH_LL_X{x}Y{y}")
        };
        if self.chip.kind == ChipKind::Spartan3E {
            if col == self.chip.col_w() + 9 {
                name = format!("CLKLH_DCM_LL_X{x}Y{y}");
            }
            if col == self.chip.col_e() - 9 {
                name = format!("CLKRH_DCM_LL_X{x}Y{y}");
            }
        } else {
            if col == self.chip.col_w() + 3 {
                name = format!("CLKLH_DCM_LL_X{x}Y{y}");
            }
            if col == self.chip.col_e() - 6 {
                name = format!("CLKRH_DCM_LL_X{x}Y{y}");
            }
            if [
                self.chip.col_w() + 1,
                self.chip.col_w() + 2,
                self.chip.col_e() - 2,
                self.chip.col_e() - 1,
            ]
            .into_iter()
            .any(|x| x == col)
            {
                name = format!("CLKH_DCM_LL_X{x}Y{y}");
            }
        }
        (naming, name)
    }

    fn get_llh_name(&self, row: RowId) -> String {
        let x = self.xlut[self.chip.col_clk - 1];
        let y = row.to_idx();
        if row == self.chip.row_s() {
            format!("CLKB_LL_X{x}Y{y}")
        } else if row == self.chip.row_n() {
            format!("CLKT_LL_X{x}Y{y}")
        } else if self.chip.kind != ChipKind::Spartan3E
            && [
                self.chip.row_s() + 2,
                self.chip.row_s() + 3,
                self.chip.row_s() + 4,
                self.chip.row_n() - 4,
                self.chip.row_n() - 3,
                self.chip.row_n() - 2,
            ]
            .into_iter()
            .any(|x| x == row)
        {
            format!("CLKV_DCM_LL_X{x}Y{y}")
        } else {
            format!("CLKV_LL_X{x}Y{y}")
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
            let mut clks = vec![];
            let mut pads = vec![];
            let mut ipads = vec![];
            if cd.io != ColumnIoKind::None {
                let (data, tidx) = get_iob_data_n(self.chip.kind, cd.io);
                for &iob in &data.iobs {
                    if iob.tile == tidx {
                        if iob.kind == IobKind::Clk {
                            clks.push(iob.iob.to_idx());
                        } else if iob.kind == IobKind::Ibuf && self.chip.kind != ChipKind::FpgaCore
                        {
                            ipads.push(iob.iob.to_idx());
                        } else {
                            pads.push(iob.iob.to_idx());
                        }
                    }
                }
            }
            let iobs: &[usize] = match self.chip.kind {
                ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => &[3, 2, 1, 0],
                ChipKind::Spartan3 => &[2, 1, 0],
                ChipKind::FpgaCore => &[3, 7, 2, 6, 1, 5, 0, 4],
                ChipKind::Spartan3E => &[2, 1, 0],
                ChipKind::Spartan3A | ChipKind::Spartan3ADsp => &[0, 1, 2],
            };
            let ntile = self
                .ngrid
                .tiles
                .get_mut(&CellCoord::new(self.die, col, row).tile(defs::tslots::BEL))
                .unwrap();
            for &i in iobs {
                let slot = if self.chip.kind == ChipKind::FpgaCore {
                    if i < 4 {
                        defs::bslots::IBUF[i]
                    } else {
                        defs::bslots::OBUF[i - 4]
                    }
                } else {
                    defs::bslots::IOI[i]
                };
                if clks.contains(&i) {
                    let name = match i {
                        0 => "CLKPPAD1",
                        1 => "CLKNPAD1",
                        _ => unreachable!(),
                    };
                    ntile.add_bel(slot, name.into());
                    self.ctr_pad += 1;
                } else if pads.contains(&i) {
                    ntile.add_bel(slot, format!("PAD{idx}", idx = self.ctr_pad));
                    self.ctr_pad += 1;
                } else if ipads.contains(&i) {
                    ntile.add_bel(slot, format!("IPAD{idx}", idx = self.ctr_pad));
                    self.ctr_pad += 1;
                } else {
                    ntile.add_bel(slot, format!("NOPAD{idx}", idx = self.ctr_nopad));
                    self.ctr_nopad += 1;
                }
            }
        }
    }

    fn fill_io_r(&mut self) {
        let col = self.chip.col_e();
        for row in self.chip.rows.ids().rev() {
            if row == self.chip.row_s() || row == self.chip.row_n() {
                continue;
            }
            let (data, tidx) = get_iob_data_e(self.chip.kind, self.chip.rows[row]);
            let mut pads = vec![];
            let mut ipads = vec![];
            for &iob in &data.iobs {
                if iob.tile == tidx {
                    if iob.kind == IobKind::Ibuf && self.chip.kind != ChipKind::FpgaCore {
                        ipads.push(iob.iob.to_idx());
                    } else {
                        pads.push(iob.iob.to_idx());
                    }
                }
            }
            let iobs: &[usize] = match self.chip.kind {
                ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => &[3, 2, 1, 0],
                ChipKind::Spartan3 => &[2, 1, 0],
                ChipKind::FpgaCore => &[3, 7, 2, 6, 1, 5, 0, 4],
                ChipKind::Spartan3E => &[2, 1, 0],
                ChipKind::Spartan3A | ChipKind::Spartan3ADsp => &[1, 0],
            };
            let ntile = self
                .ngrid
                .tiles
                .get_mut(&CellCoord::new(self.die, col, row).tile(defs::tslots::BEL))
                .unwrap();
            for &i in iobs {
                let slot = if self.chip.kind == ChipKind::FpgaCore {
                    if i < 4 {
                        defs::bslots::IBUF[i]
                    } else {
                        defs::bslots::OBUF[i - 4]
                    }
                } else {
                    defs::bslots::IOI[i]
                };
                if pads.contains(&i) {
                    ntile.add_bel(slot, format!("PAD{idx}", idx = self.ctr_pad));
                    self.ctr_pad += 1;
                } else if ipads.contains(&i) {
                    ntile.add_bel(slot, format!("IPAD{idx}", idx = self.ctr_pad));
                    self.ctr_pad += 1;
                } else {
                    ntile.add_bel(slot, format!("NOPAD{idx}", idx = self.ctr_nopad));
                    self.ctr_nopad += 1;
                }
            }
        }
    }

    fn fill_io_b(&mut self) {
        let row = self.chip.row_s();
        for (col, &cd) in self.chip.columns.iter().rev() {
            if self.chip.kind.is_spartan3ea() {
                if cd.kind == ColumnKind::Io {
                    continue;
                }
            } else {
                if cd.kind != ColumnKind::Clb {
                    continue;
                }
            }
            let mut clks = vec![];
            let mut pads = vec![];
            let mut ipads = vec![];
            if cd.io != ColumnIoKind::None {
                let (data, tidx) = get_iob_data_s(self.chip.kind, cd.io);
                for &iob in &data.iobs {
                    if iob.tile == tidx {
                        if iob.kind == IobKind::Clk {
                            clks.push(iob.iob.to_idx());
                        } else if iob.kind == IobKind::Ibuf && self.chip.kind != ChipKind::FpgaCore
                        {
                            ipads.push(iob.iob.to_idx());
                        } else {
                            pads.push(iob.iob.to_idx());
                        }
                    }
                }
            }
            let iobs: &[usize] = match self.chip.kind {
                ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => &[3, 2, 1, 0],
                ChipKind::Spartan3 => &[2, 1, 0],
                ChipKind::FpgaCore => &[3, 7, 2, 6, 1, 5, 0, 4],
                ChipKind::Spartan3E => &[2, 1, 0],
                ChipKind::Spartan3A | ChipKind::Spartan3ADsp => &[2, 1, 0],
            };
            let ntile = self
                .ngrid
                .tiles
                .get_mut(&CellCoord::new(self.die, col, row).tile(defs::tslots::BEL))
                .unwrap();
            for &i in iobs {
                let slot = if self.chip.kind == ChipKind::FpgaCore {
                    if i < 4 {
                        defs::bslots::IBUF[i]
                    } else {
                        defs::bslots::OBUF[i - 4]
                    }
                } else {
                    defs::bslots::IOI[i]
                };
                if clks.contains(&i) {
                    let name = match i {
                        2 => "CLKPPAD2",
                        3 => "CLKNPAD2",
                        _ => unreachable!(),
                    };
                    ntile.add_bel(slot, name.into());
                    self.ctr_pad += 1;
                } else if pads.contains(&i) {
                    let mut name = format!("PAD{idx}", idx = self.ctr_pad);
                    if self.chip.kind == ChipKind::Spartan3A && self.chip.cols_clkv.is_none() {
                        // 3s50a special
                        match self.ctr_pad {
                            94 => name = "PAD96".to_string(),
                            96 => name = "PAD97".to_string(),
                            97 => name = "PAD95".to_string(),
                            _ => (),
                        }
                    }
                    ntile.add_bel(slot, name);
                    self.ctr_pad += 1;
                } else if ipads.contains(&i) {
                    let mut name = format!("IPAD{idx}", idx = self.ctr_pad);
                    if self.chip.kind == ChipKind::Spartan3A
                        && self.chip.cols_clkv.is_none()
                        && self.ctr_pad == 95
                    {
                        name = "IPAD94".to_string();
                    }
                    ntile.add_bel(slot, name);
                    self.ctr_pad += 1;
                } else {
                    ntile.add_bel(slot, format!("NOPAD{idx}", idx = self.ctr_nopad));
                    self.ctr_nopad += 1;
                }
            }
        }
    }

    fn fill_io_l(&mut self) {
        let col = self.chip.col_w();
        for row in self.chip.rows.ids() {
            if row == self.chip.row_s() || row == self.chip.row_n() {
                continue;
            }
            let (data, tidx) = get_iob_data_w(self.chip.kind, self.chip.rows[row]);
            let mut pads = vec![];
            let mut ipads = vec![];
            for &iob in &data.iobs {
                if iob.tile == tidx {
                    if iob.kind == IobKind::Ibuf && self.chip.kind != ChipKind::FpgaCore {
                        ipads.push(iob.iob.to_idx());
                    } else {
                        pads.push(iob.iob.to_idx());
                    }
                }
            }
            let iobs: &[usize] = match self.chip.kind {
                ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => &[0, 1, 2, 3],
                ChipKind::Spartan3 => &[0, 1, 2],
                ChipKind::FpgaCore => &[0, 4, 1, 5, 2, 6, 3, 7],
                ChipKind::Spartan3E => &[2, 1, 0],
                ChipKind::Spartan3A | ChipKind::Spartan3ADsp => &[0, 1],
            };
            let ntile = self
                .ngrid
                .tiles
                .get_mut(&CellCoord::new(self.die, col, row).tile(defs::tslots::BEL))
                .unwrap();
            for &i in iobs {
                let slot = if self.chip.kind == ChipKind::FpgaCore {
                    if i < 4 {
                        defs::bslots::IBUF[i]
                    } else {
                        defs::bslots::OBUF[i - 4]
                    }
                } else {
                    defs::bslots::IOI[i]
                };
                if pads.contains(&i) {
                    ntile.add_bel(slot, format!("PAD{idx}", idx = self.ctr_pad));
                    self.ctr_pad += 1;
                } else if ipads.contains(&i) {
                    ntile.add_bel(slot, format!("IPAD{idx}", idx = self.ctr_pad));
                    self.ctr_pad += 1;
                } else {
                    ntile.add_bel(slot, format!("NOPAD{idx}", idx = self.ctr_nopad));
                    self.ctr_nopad += 1;
                }
            }
        }
    }
}

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let chip = edev.chip;
    let ngrid = ExpandedGridNaming::new(ndb, edev);
    let dcm_grid = ngrid.bel_grid(|_, _, tcls| tcls.bels.contains_id(defs::bslots::DCM));
    let bram_grid = ngrid.bel_grid(|_, _, tcls| tcls.bels.contains_id(defs::bslots::BRAM));
    let mut namer = Namer {
        edev,
        chip,
        die: DieId::from_idx(0),
        ngrid,
        xlut: EntityVec::new(),
        sxlut: EntityPartVec::new(),
        dcm_grid,
        bram_grid,
        gtxlut: EntityPartVec::new(),
        vcc_xlut: EntityVec::new(),
        vcc_ylut: EntityVec::new(),
        clut: EntityVec::new(),
        bramclut: EntityVec::new(),
        rlut: EntityVec::new(),
        hclklut: EntityPartVec::new(),
        rows_brk: HashSet::new(),
        ctr_pad: 1,
        ctr_nopad: if chip.kind.is_spartan3ea() { 0 } else { 1 },
    };

    namer.fill_xlut();
    namer.fill_gtxlut();
    namer.fill_clut();
    namer.fill_rlut();
    namer.fill_hclklut();
    namer.fill_rows_brk();
    namer.fill_vcc_lut();

    namer.ngrid.tie_kind = Some("VCC".to_string());
    namer.ngrid.tie_pin_pullup = Some("VCCOUT".to_string());

    for (tcrd, tile) in edev.tiles() {
        let CellCoord { col, row, .. } = tcrd.cell;
        let tcname = edev.db.tile_classes.key(tile.class);
        match (edev.chip.kind.is_virtex2(), tile.class) {
            (
                true,
                tcls_v2::INT_CLB
                | tcls_v2::INT_BRAM
                | tcls_v2::INT_IOI
                | tcls_v2::INT_IOI_CLK_S
                | tcls_v2::INT_IOI_CLK_N
                | tcls_v2::INT_DCM_V2
                | tcls_v2::INT_DCM_V2P
                | tcls_v2::INT_CNR
                | tcls_v2::INT_PPC
                | tcls_v2::INT_GT_CLKPAD,
            )
            | (
                false,
                tcls_s3::INT_CLB
                | tcls_s3::INT_CLB_FC
                | tcls_s3::INT_IOI_S3
                | tcls_s3::INT_IOI_FC
                | tcls_s3::INT_IOI_S3E
                | tcls_s3::INT_IOI_S3A_WE
                | tcls_s3::INT_IOI_S3A_SN
                | tcls_s3::INT_BRAM_S3
                | tcls_s3::INT_BRAM_S3E
                | tcls_s3::INT_BRAM_S3A_03
                | tcls_s3::INT_BRAM_S3A_12
                | tcls_s3::INT_BRAM_S3ADSP
                | tcls_s3::INT_DCM
                | tcls_s3::INT_DCM_S3_DUMMY
                | tcls_s3::INT_DCM_S3E_DUMMY,
            ) => {
                let (naming, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, naming, [name]);
                let x = col.to_idx();
                let y = row.to_idx();
                ntile.add_bel(defs::bslots::INT, format!("RLL_X{x}Y{y}"));
                if (edev.chip.kind.is_virtex2(), tile.class) != (false, tcls_s3::INT_DCM_S3E_DUMMY)
                {
                    let mut x = namer.vcc_xlut[col];
                    let mut y = namer.vcc_ylut[row];
                    if chip.kind == ChipKind::Virtex2 {
                        // Look, just..... don't ask me.
                        x = col.to_idx();
                        if col == chip.col_w() {
                            if row == chip.row_s() {
                                y = chip.rows.len() - 2;
                            } else if row == chip.row_n() {
                                y = chip.rows.len() - 1;
                            } else {
                                y -= 1;
                            }
                        } else if col == chip.col_e() {
                            if row == chip.row_s() {
                                y = 0;
                                x += 1;
                            } else if row == chip.row_n() {
                                y = 1;
                                x += 1;
                            } else {
                                y += 1;
                            }
                        } else if col < chip.col_clk {
                            if row == chip.row_s() {
                                y = 0;
                            } else if row == chip.row_n() {
                                y = 1;
                            } else {
                                y += 1;
                            }
                        } else {
                            if row == chip.row_s() {
                                y = 2;
                            } else if row == chip.row_n() {
                                y = 3;
                            } else {
                                y += 3;
                                if y >= chip.rows.len() {
                                    y -= chip.rows.len();
                                    x += 1;
                                }
                            }
                        }
                    }
                    ntile.tie_name = Some(format!("VCC_X{x}Y{y}"));
                }
            }
            (true, tcls_v2::INTF_PPC) => {
                let (naming, name) = namer.get_int_name(tcrd.cell);
                let naming = format!("INTF_{}", &naming[4..]);
                namer.ngrid.name_tile(tcrd, &naming, [name]);
            }
            (true, tcls_v2::INTF_GT_S_CLKPAD | tcls_v2::INTF_GT_N_CLKPAD) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                namer.ngrid.name_tile(tcrd, "INTF_GT_CLKPAD", [name]);
            }
            (
                true,
                tcls_v2::INTF_GT_S0
                | tcls_v2::INTF_GT_S123
                | tcls_v2::INTF_GT_N0
                | tcls_v2::INTF_GT_N123,
            ) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                namer.ngrid.name_tile(tcrd, "INTF_GT", [name]);
            }
            (true, tcls_v2::CLB) | (false, tcls_s3::CLB) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, "CLB", [name]);
                let sx = namer.sxlut[col];
                let sy = 2 * (row.to_idx() - 1);
                if chip.kind.is_virtex2() {
                    ntile.add_bel(defs::bslots::SLICE[0], format!("SLICE_X{sx}Y{sy}"));
                    ntile.add_bel(
                        defs::bslots::SLICE[1],
                        format!("SLICE_X{x}Y{y}", x = sx, y = sy + 1),
                    );
                    ntile.add_bel(
                        defs::bslots::SLICE[2],
                        format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy),
                    );
                    ntile.add_bel(
                        defs::bslots::SLICE[3],
                        format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy + 1),
                    );
                    if sx.is_multiple_of(4) {
                        ntile.add_bel(defs::bslots::TBUF[0], format!("TBUF_X{sx}Y{sy}"));
                        ntile.add_bel(
                            defs::bslots::TBUF[1],
                            format!("TBUF_X{x}Y{y}", x = sx, y = sy + 1),
                        );
                    } else {
                        ntile.add_bel(
                            defs::bslots::TBUF[0],
                            format!("TBUF_X{x}Y{y}", x = sx, y = sy + 1),
                        );
                        ntile.add_bel(defs::bslots::TBUF[1], format!("TBUF_X{sx}Y{sy}"));
                    }
                } else {
                    ntile.add_bel(defs::bslots::SLICE[0], format!("SLICE_X{sx}Y{sy}"));
                    ntile.add_bel(
                        defs::bslots::SLICE[1],
                        format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy),
                    );
                    ntile.add_bel(
                        defs::bslots::SLICE[2],
                        format!("SLICE_X{x}Y{y}", x = sx, y = sy + 1),
                    );
                    ntile.add_bel(
                        defs::bslots::SLICE[3],
                        format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy + 1),
                    );
                }
            }
            (false, tcls_s3::RANDOR | tcls_s3::RANDOR_FC) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let naming = if row == chip.row_s() {
                    "RANDOR_S"
                } else if row == chip.row_n() {
                    "RANDOR_N"
                } else {
                    unreachable!()
                };
                let ntile = namer.ngrid.name_tile(tcrd, naming, [name]);
                let x = if chip.kind == ChipKind::Spartan3 {
                    (namer.clut[col] - 1) * 2
                } else {
                    col.to_idx() - 1
                };
                let y = if chip.kind != ChipKind::FpgaCore && naming == "RANDOR_N" {
                    1
                } else {
                    0
                };
                ntile.add_bel(defs::bslots::RANDOR, format!("RANDOR_X{x}Y{y}"));
            }
            (false, tcls_s3::RANDOR_INIT | tcls_s3::RANDOR_INIT_FC) => {}
            (true, tcls_v2::BRAM)
            | (
                false,
                tcls_s3::BRAM_S3 | tcls_s3::BRAM_S3E | tcls_s3::BRAM_S3A | tcls_s3::BRAM_S3ADSP,
            ) => {
                let (naming, name) = namer.get_bram_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, naming, [name]);
                let x = namer.bram_grid.xlut[col];
                let y = namer.bram_grid.ylut[row];
                ntile.add_bel(defs::bslots::BRAM, format!("RAMB16_X{x}Y{y}"));
                if chip.kind != ChipKind::Spartan3ADsp {
                    ntile.add_bel(defs::bslots::MULT, format!("MULT18X18_X{x}Y{y}"));
                }
            }
            (false, tcls_s3::DSP) => {
                let (naming, name) = namer.get_dsp_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, naming, [name]);
                let x = namer.bram_grid.xlut[col - 3];
                let y = namer.bram_grid.ylut[row];
                ntile.add_bel(defs::bslots::DSP, format!("DSP48A_X{x}Y{y}"));
            }
            (true, tcls_v2::GIGABIT_S) => {
                let c = namer.bramclut[col];
                let r = namer.rlut[row + 1];
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, "GIGABIT_S", [format!("BMR{r}C{c}")]);
                let gx = namer.gtxlut[col];
                let (bank, _) = chip.cols_gt[&col];
                ntile.add_bel_multi(
                    defs::bslots::GT,
                    [
                        format!("GT_X{gx}Y0"),
                        format!("RXPPAD{bank}"),
                        format!("RXNPAD{bank}"),
                        format!("TXPPAD{bank}"),
                        format!("TXNPAD{bank}"),
                    ],
                );
            }
            (true, tcls_v2::GIGABIT10_S) => {
                let c = namer.bramclut[col];
                let r = namer.rlut[row + 1];
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, "GIGABIT10_S", [format!("BMR{r}C{c}")]);
                let gx = namer.gtxlut[col];
                let (bank, _) = chip.cols_gt[&col];
                ntile.add_bel_multi(
                    defs::bslots::GT10,
                    [
                        format!("GT10_X{gx}Y0"),
                        format!("RXPPAD{bank}"),
                        format!("RXNPAD{bank}"),
                        format!("TXPPAD{bank}"),
                        format!("TXNPAD{bank}"),
                    ],
                );
            }
            (true, tcls_v2::GIGABIT_N) => {
                let c = namer.bramclut[col];
                let r = namer.rlut[row - 4];
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, "GIGABIT_N", [format!("BMR{r}C{c}")]);
                let gx = namer.gtxlut[col];
                let (_, bank) = chip.cols_gt[&col];
                ntile.add_bel_multi(
                    defs::bslots::GT,
                    [
                        format!("GT_X{gx}Y1"),
                        format!("RXPPAD{bank}"),
                        format!("RXNPAD{bank}"),
                        format!("TXPPAD{bank}"),
                        format!("TXNPAD{bank}"),
                    ],
                );
            }
            (true, tcls_v2::GIGABIT10_N) => {
                let c = namer.bramclut[col];
                let r = namer.rlut[row - 8];
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, "GIGABIT10_N", [format!("BMR{r}C{c}")]);
                let gx = namer.gtxlut[col];
                let (_, bank) = chip.cols_gt[&col];
                ntile.add_bel_multi(
                    defs::bslots::GT10,
                    [
                        format!("GT10_X{gx}Y1"),
                        format!("RXPPAD{bank}"),
                        format!("RXNPAD{bank}"),
                        format!("TXPPAD{bank}"),
                        format!("TXNPAD{bank}"),
                    ],
                );
            }
            (true, tcls_v2::PPC_W | tcls_v2::PPC_E) => {
                let x = if tile.class == tcls_v2::PPC_W || chip.holes_ppc.len() == 1 {
                    0
                } else {
                    1
                };
                let ntile = namer.ngrid.name_tile(tcrd, tcname, [format!("PPC_X{x}Y0")]);
                ntile.add_bel(defs::bslots::PPC405, format!("PPC405_X{x}Y0"));
            }
            (true, tcls_v2::TERM_W) => {
                let (naming, name) = namer.get_lterm_name(row);
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            (true, tcls_v2::TERM_E) => {
                let (naming, name) = namer.get_rterm_name(row);
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            (true, tcls_v2::TERM_S) => {
                let (naming, name) = namer.get_bterm_name(col);
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            (true, tcls_v2::TERM_N) => {
                let (naming, name) = namer.get_tterm_name(col);
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            (true, tcls_v2::PPC_TERM_E) => {
                let (name_l, name_r) = namer.get_ppc_h_name(tcrd.cell);
                namer.ngrid.name_tile(tcrd, "PPC_TERM_E", [name_l, name_r]);
            }
            (true, tcls_v2::PPC_TERM_W) => {
                let (name_l, name_r) = namer.get_ppc_h_name(tcrd.cell.delta(-9, 0));
                namer.ngrid.name_tile(tcrd, "PPC_TERM_W", [name_r, name_l]);
            }
            (true, tcls_v2::PPC_TERM_N) => {
                let (name_b, name_t) = namer.get_ppc_v_name(tcrd.cell);
                namer.ngrid.name_tile(tcrd, "PPC_TERM_N", [name_b, name_t]);
            }
            (true, tcls_v2::PPC_TERM_S) => {
                let (name_b, name_t) = namer.get_ppc_v_name(tcrd.cell.delta(0, -15));
                namer.ngrid.name_tile(tcrd, "PPC_TERM_S", [name_t, name_b]);
            }
            (false, tcls_s3::LLV_S3E | tcls_s3::LLV_S3A) => {
                let (naming, name) = namer.get_llv_name(col);
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            (false, tcls_s3::LLH | tcls_s3::LLH_S_S3A | tcls_s3::LLH_N_S3A) => {
                let name = namer.get_llh_name(row);
                namer.ngrid.name_tile(tcrd, "LLH", [name]);
            }
            (true, tcls_v2::CLK_S) => {
                let (name, naming) = match chip.kind {
                    ChipKind::Virtex2 => ("CLKB", "CLK_S_V2"),
                    ChipKind::Virtex2P => ("ML_CLKB", "CLK_S_V2P"),
                    ChipKind::Virtex2PX => ("MK_CLKB", "CLK_S_V2PX"),
                    _ => unreachable!(),
                };
                let ntile = namer.ngrid.name_tile(tcrd, naming, [name.into()]);
                let vx = namer.vcc_xlut[chip.col_clk] - 1;
                let vy = chip.row_s().to_idx();
                ntile.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                ntile.add_bel(defs::bslots::BUFGMUX[0], "BUFGMUX0P".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[1], "BUFGMUX1S".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[2], "BUFGMUX2P".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[3], "BUFGMUX3S".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[4], "BUFGMUX4P".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[5], "BUFGMUX5S".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[6], "BUFGMUX6P".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[7], "BUFGMUX7S".to_string());
                ntile.add_bel(
                    defs::bslots::GLOBALSIG_BUFG[0],
                    format!("GSIG_X{x}Y0", x = chip.col_clk.to_idx()),
                );
                ntile.add_bel(
                    defs::bslots::GLOBALSIG_BUFG[1],
                    format!("GSIG_X{x}Y0", x = chip.col_clk.to_idx() + 1),
                );
            }
            (true, tcls_v2::CLK_N) => {
                let (name, naming) = match chip.kind {
                    ChipKind::Virtex2 => ("CLKT", "CLK_N_V2"),
                    ChipKind::Virtex2P => ("ML_CLKT", "CLK_N_V2P"),
                    ChipKind::Virtex2PX => ("MK_CLKT", "CLK_N_V2PX"),
                    _ => unreachable!(),
                };
                let ntile = namer.ngrid.name_tile(tcrd, naming, [name.into()]);
                let vx = namer.vcc_xlut[chip.col_clk] - 1;
                let vy = if chip.kind == ChipKind::Virtex2 {
                    1
                } else {
                    chip.rows.len() - 1
                };
                ntile.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                ntile.add_bel(defs::bslots::BUFGMUX[0], "BUFGMUX0S".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[1], "BUFGMUX1P".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[2], "BUFGMUX2S".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[3], "BUFGMUX3P".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[4], "BUFGMUX4S".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[5], "BUFGMUX5P".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[6], "BUFGMUX6S".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[7], "BUFGMUX7P".to_string());
                ntile.add_bel(
                    defs::bslots::GLOBALSIG_BUFG[0],
                    format!("GSIG_X{x}Y1", x = chip.col_clk.to_idx()),
                );
                ntile.add_bel(
                    defs::bslots::GLOBALSIG_BUFG[1],
                    format!("GSIG_X{x}Y1", x = chip.col_clk.to_idx() + 1),
                );
            }
            (false, tcls_s3::CLK_S_S3 | tcls_s3::CLK_S_FC) => {
                let bufg = if chip.kind == ChipKind::FpgaCore {
                    "BUFG"
                } else {
                    "BUFGMUX"
                };
                let ntile = namer.ngrid.name_tile(tcrd, tcname, ["CLKB".into()]);
                let vx = namer.vcc_xlut[chip.col_clk] - 1;
                let vy = 0;
                ntile.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                ntile.add_bel(defs::bslots::BUFGMUX[0], format!("{bufg}0"));
                ntile.add_bel(defs::bslots::BUFGMUX[1], format!("{bufg}1"));
                ntile.add_bel(defs::bslots::BUFGMUX[2], format!("{bufg}2"));
                ntile.add_bel(defs::bslots::BUFGMUX[3], format!("{bufg}3"));
                ntile.add_bel(
                    defs::bslots::GLOBALSIG_BUFG[0],
                    format!("GSIG_X{x}Y0", x = chip.col_clk.to_idx()),
                );
            }
            (false, tcls_s3::CLK_N_S3 | tcls_s3::CLK_N_FC) => {
                let bufg = if chip.kind == ChipKind::FpgaCore {
                    "BUFG"
                } else {
                    "BUFGMUX"
                };
                let ntile = namer.ngrid.name_tile(tcrd, tcname, ["CLKT".into()]);
                let vx = namer.vcc_xlut[chip.col_clk] - 1;
                let vy = namer.vcc_ylut[chip.row_n()];
                ntile.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                ntile.add_bel(defs::bslots::BUFGMUX[0], format!("{bufg}4"));
                ntile.add_bel(defs::bslots::BUFGMUX[1], format!("{bufg}5"));
                ntile.add_bel(defs::bslots::BUFGMUX[2], format!("{bufg}6"));
                ntile.add_bel(defs::bslots::BUFGMUX[3], format!("{bufg}7"));
                ntile.add_bel(
                    defs::bslots::GLOBALSIG_BUFG[0],
                    format!("GSIG_X{x}Y1", x = chip.col_clk.to_idx()),
                );
            }
            (false, tcls_s3::CLK_S_S3E | tcls_s3::CLK_S_S3A) => {
                let x = namer.xlut[chip.col_clk - 1];
                let y = row.to_idx();
                let yb = y + 1;
                let (name, name_buf) = if chip.has_ll {
                    (format!("CLKB_LL_X{x}Y{y}"), format!("CLKV_LL_X{x}Y{yb}"))
                } else {
                    (format!("CLKB_X{x}Y{y}"), format!("CLKV_X{x}Y{yb}"))
                };
                let ntile = namer.ngrid.name_tile(tcrd, tcname, [name, name_buf]);
                let vx = namer.vcc_xlut[chip.col_clk] - 1;
                let vy = 0;
                ntile.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                ntile.add_bel(defs::bslots::BUFGMUX[0], "BUFGMUX_X2Y1".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[1], "BUFGMUX_X2Y0".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[2], "BUFGMUX_X1Y1".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[3], "BUFGMUX_X1Y0".to_string());
                ntile.add_bel(
                    defs::bslots::GLOBALSIG_BUFG[0],
                    format!("GLOBALSIG_X{x}Y0", x = namer.xlut[chip.col_clk] + 1),
                );
            }
            (false, tcls_s3::CLK_N_S3E | tcls_s3::CLK_N_S3A) => {
                let x = namer.xlut[chip.col_clk - 1];
                let y = row.to_idx();
                let yb = y - 1;
                let (name, name_buf) = if chip.has_ll {
                    (format!("CLKT_LL_X{x}Y{y}"), format!("CLKV_LL_X{x}Y{yb}"))
                } else {
                    (format!("CLKT_X{x}Y{y}"), format!("CLKV_X{x}Y{yb}"))
                };
                let ntile = namer.ngrid.name_tile(tcrd, tcname, [name, name_buf]);
                let vx = namer.vcc_xlut[chip.col_clk] - 1;
                let vy = namer.vcc_ylut[chip.row_n()];
                ntile.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                ntile.add_bel(defs::bslots::BUFGMUX[0], "BUFGMUX_X2Y11".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[1], "BUFGMUX_X2Y10".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[2], "BUFGMUX_X1Y11".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[3], "BUFGMUX_X1Y10".to_string());
                ntile.add_bel(
                    defs::bslots::GLOBALSIG_BUFG[0],
                    format!(
                        "GLOBALSIG_X{x}Y{y}",
                        x = namer.xlut[chip.col_clk] + 1,
                        y = chip.rows_hclk.len() + 2
                    ),
                );
            }
            (false, tcls_s3::CLK_W_S3E | tcls_s3::CLK_W_S3A) => {
                let x = namer.xlut[col];
                let y = row.to_idx() - 1;
                let mut names = vec![format!("CLKL_X{x}Y{y}")];
                if chip.kind != ChipKind::Spartan3E {
                    names.push(if chip.has_ll {
                        format!("CLKL_IOIS_LL_X{x}Y{y}")
                    } else if chip.cols_clkv.is_none() {
                        format!("CLKL_IOIS_50A_X{x}Y{y}")
                    } else {
                        format!("CLKL_IOIS_X{x}Y{y}")
                    });
                }
                let ntile = namer.ngrid.name_tile(tcrd, tcname, names);
                let vy = namer.vcc_ylut[chip.row_mid()] - 1;
                let vx = 0;
                let gsy = chip.rows_hclk.len().div_ceil(2) + 1;
                ntile.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                ntile.add_bel(defs::bslots::BUFGMUX[0], "BUFGMUX_X0Y2".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[1], "BUFGMUX_X0Y3".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[2], "BUFGMUX_X0Y4".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[3], "BUFGMUX_X0Y5".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[4], "BUFGMUX_X0Y6".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[5], "BUFGMUX_X0Y7".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[6], "BUFGMUX_X0Y8".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[7], "BUFGMUX_X0Y9".to_string());
                ntile.add_bel(defs::bslots::PCILOGICSE, "PCILOGIC_X0Y0".to_string());
                ntile.add_bel(
                    defs::bslots::GLOBALSIG_BUFG[0],
                    format!("GLOBALSIG_X0Y{gsy}"),
                );
            }
            (false, tcls_s3::CLK_E_S3E | tcls_s3::CLK_E_S3A) => {
                let x = namer.xlut[col];
                let y = row.to_idx() - 1;
                let mut names = vec![format!("CLKR_X{x}Y{y}")];
                if chip.kind != ChipKind::Spartan3E {
                    names.push(if chip.has_ll {
                        format!("CLKR_IOIS_LL_X{x}Y{y}")
                    } else if chip.cols_clkv.is_none() {
                        format!("CLKR_IOIS_50A_X{x}Y{y}")
                    } else {
                        format!("CLKR_IOIS_X{x}Y{y}")
                    });
                }
                let ntile = namer.ngrid.name_tile(tcrd, tcname, names);
                let vy = namer.vcc_ylut[chip.row_mid()] - 1;
                let vx = namer.vcc_xlut[chip.col_e()] + 1;
                let gsy = chip.rows_hclk.len().div_ceil(2) + 1;
                ntile.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                ntile.add_bel(defs::bslots::BUFGMUX[0], "BUFGMUX_X3Y2".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[1], "BUFGMUX_X3Y3".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[2], "BUFGMUX_X3Y4".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[3], "BUFGMUX_X3Y5".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[4], "BUFGMUX_X3Y6".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[5], "BUFGMUX_X3Y7".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[6], "BUFGMUX_X3Y8".to_string());
                ntile.add_bel(defs::bslots::BUFGMUX[7], "BUFGMUX_X3Y9".to_string());
                ntile.add_bel(defs::bslots::PCILOGICSE, "PCILOGIC_X1Y0".to_string());
                ntile.add_bel(
                    defs::bslots::GLOBALSIG_BUFG[0],
                    format!("GLOBALSIG_X{x}Y{gsy}", x = namer.xlut[chip.col_e()] + 3),
                );
            }

            (true, tcls_v2::PCI_W) => {
                let rb = namer.rlut[row - 1];
                let rt = namer.rlut[row];
                let ntile = namer.ngrid.name_tile(
                    tcrd,
                    "REG_L",
                    [
                        if chip.kind == ChipKind::Virtex2 {
                            "HMLTERM"
                        } else {
                            "LTERMCLKH"
                        }
                        .into(),
                        format!("LTERMR{rb}"),
                        format!("LTERMR{rt}"),
                    ],
                );
                ntile.add_bel(defs::bslots::PCILOGIC, "PCILOGIC_X0Y0".into());
            }
            (true, tcls_v2::PCI_E) => {
                let rb = namer.rlut[row - 1];
                let rt = namer.rlut[row];
                let ntile = namer.ngrid.name_tile(
                    tcrd,
                    "REG_R",
                    [
                        if chip.kind == ChipKind::Virtex2 {
                            "HMRTERM"
                        } else {
                            "RTERMCLKH"
                        }
                        .into(),
                        format!("RTERMR{rb}"),
                        format!("RTERMR{rt}"),
                    ],
                );
                ntile.add_bel(defs::bslots::PCILOGIC, "PCILOGIC_X1Y0".into());
            }
            (true, tcls_v2::HCLK)
            | (
                false,
                tcls_s3::HCLK
                | tcls_s3::HCLK_UNI
            ) => {
                let (naming, name) = namer.get_hclk_name(tcrd.cell);
                let mut names = vec![name];
                if naming == "HCLK_DSP" {
                    names.push(format!(
                        "MACC2_GCLKH_FEEDTHRUA_X{x}Y{y}",
                        x = namer.xlut[col] + 1,
                        y = row.to_idx() - 1
                    ));
                }
                let ntile = namer.ngrid.name_tile(tcrd, naming, names);
                if !chip.kind.is_spartan3ea() {
                    let gsx = if col < chip.col_clk {
                        col.to_idx()
                    } else if !chip.kind.is_virtex2() {
                        col.to_idx() + 1
                    } else {
                        col.to_idx() + 2
                    };
                    let gsy = namer.hclklut[row];
                    ntile.add_bel(defs::bslots::GLOBALSIG_HCLK, format!("GSIG_X{gsx}Y{gsy}"));
                } else {
                    let gsx = if col < chip.col_clk {
                        namer.xlut[col] + 1
                    } else {
                        namer.xlut[col] + 2
                    };
                    let gsy = if row <= chip.row_mid() {
                        namer.hclklut[row] + 1
                    } else {
                        namer.hclklut[row] + 2
                    };
                    if naming == "HCLK_DSP" {
                        ntile.add_bel_multi(
                            defs::bslots::GLOBALSIG_HCLK,
                            [
                                format!("GLOBALSIG_X{gsx}Y{gsy}"),
                                format!("GLOBALSIG_X{gsx}Y{gsy}", gsx = gsx + 1),
                            ],
                        );
                    } else {
                        ntile.add_bel(
                            defs::bslots::GLOBALSIG_HCLK,
                            format!("GLOBALSIG_X{gsx}Y{gsy}"),
                        );
                    }
                }
            }
            (true, tcls_v2::HROW | tcls_v2::HROW_S | tcls_v2::HROW_N) => {
                let mut r = chip.rows_hclk.len() - namer.hclklut[row];
                // I hate ISE.
                if chip.columns.len() == 12 {
                    r -= 1;
                }
                let name = format!("GCLKCR{r}");
                namer.ngrid.name_tile(tcrd, "GCLKC", [name]);
            }
            (false, tcls_s3::CLKC_50A) => {
                let x = namer.xlut[col] - 1;
                let y = row.to_idx() - 1;
                let name = format!("CLKC_50A_X{x}Y{y}");
                namer.ngrid.name_tile(tcrd, "CLKC_50A", [name]);
            }
            (false, tcls_s3::CLKQC_S3) => {
                let lr = if col < chip.col_clk { 'L' } else { 'R' };
                let name = format!("{lr}GCLKVM");
                namer.ngrid.name_tile(tcrd, "GCLKVM_S3", [name]);
            }
            (false, tcls_s3::CLKQC_S3E) => {
                let x = namer.xlut[col] - 1;
                let y = row.to_idx() - 1;
                let naming = if col < chip.col_clk {
                    "GCLKVML"
                } else {
                    "GCLKVMR"
                };
                let name = format!("{naming}_X{x}Y{y}");
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }

            (true, tcls_v2::IOI | tcls_v2::IOI_CLK_S | tcls_v2::IOI_CLK_N) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let naming = if matches!(
                    chip.columns[col].io,
                    ColumnIoKind::SingleWAlt | ColumnIoKind::SingleEAlt
                ) {
                    "IOI_TBS"
                } else {
                    tcname
                };
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            (false, tcls_s3::IOI_S3) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let naming = if col == chip.col_w() {
                    "IOI_S3_W"
                } else if col == chip.col_e() {
                    "IOI_S3_E"
                } else if row == chip.row_s() {
                    "IOI_S3_S"
                } else if row == chip.row_n() {
                    "IOI_S3_N"
                } else {
                    unreachable!()
                };
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            (false, tcls_s3::IOI_FC) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let naming = if col == chip.col_w() {
                    "IOI_FC_W"
                } else if col == chip.col_e() {
                    "IOI_FC_E"
                } else if row == chip.row_s() {
                    "IOI_FC_S"
                } else if row == chip.row_n() {
                    "IOI_FC_N"
                } else {
                    unreachable!()
                };
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            (false, tcls_s3::IOI_S3E) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let naming = if col == chip.col_w() {
                    if row >= chip.row_mid() - 4 && row < chip.row_mid() + 4 {
                        if row == chip.row_mid() - 4 || row == chip.row_mid() {
                            "IOI_S3E_W_PCI"
                        } else {
                            "IOI_S3E_W_PCI_PCI"
                        }
                    } else {
                        "IOI_S3E_W"
                    }
                } else if col == chip.col_e() {
                    if row >= chip.row_mid() - 4 && row < chip.row_mid() + 4 {
                        if row == chip.row_mid() - 1 || row == chip.row_mid() + 3 {
                            "IOI_S3E_E_PCI"
                        } else {
                            "IOI_S3E_E_PCI_PCI"
                        }
                    } else {
                        "IOI_S3E_E"
                    }
                } else if row == chip.row_s() {
                    "IOI_S3E_S"
                } else if row == chip.row_n() {
                    "IOI_S3E_N"
                } else {
                    unreachable!()
                };
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            (false, tcls_s3::IOI_S3A_S) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let naming = if chip.kind == ChipKind::Spartan3ADsp {
                    "IOI_S3ADSP_S"
                } else {
                    "IOI_S3A_S"
                };
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            (false, tcls_s3::IOI_S3A_N) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let naming = if chip.kind == ChipKind::Spartan3ADsp {
                    "IOI_S3ADSP_N"
                } else {
                    "IOI_S3A_N"
                };
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            (false, tcls_s3::IOI_S3A_WE) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let naming = if col == chip.col_w() {
                    if row >= chip.row_mid() - 4
                        && row < chip.row_mid() + 4
                        && row != chip.row_mid() - 4
                        && row != chip.row_mid()
                    {
                        if chip.kind == ChipKind::Spartan3ADsp {
                            "IOI_S3ADSP_W_PCI"
                        } else {
                            "IOI_S3A_W_PCI"
                        }
                    } else {
                        if chip.kind == ChipKind::Spartan3ADsp {
                            "IOI_S3ADSP_W"
                        } else {
                            "IOI_S3A_W"
                        }
                    }
                } else if col == chip.col_e() {
                    if row >= chip.row_mid() - 4
                        && row < chip.row_mid() + 4
                        && row != chip.row_mid() - 1
                        && row != chip.row_mid() + 3
                    {
                        if chip.kind == ChipKind::Spartan3ADsp {
                            "IOI_S3ADSP_E_PCI"
                        } else {
                            "IOI_S3A_E_PCI"
                        }
                    } else {
                        if chip.kind == ChipKind::Spartan3ADsp {
                            "IOI_S3ADSP_E"
                        } else {
                            "IOI_S3A_E"
                        }
                    }
                } else {
                    unreachable!()
                };
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            (
                true,
                tcls_v2::IOB_V2_SW2
                | tcls_v2::IOB_V2_SE2
                | tcls_v2::IOB_V2_NW2
                | tcls_v2::IOB_V2_NE2
                | tcls_v2::IOB_V2_WS2
                | tcls_v2::IOB_V2_WN2
                | tcls_v2::IOB_V2_ES2
                | tcls_v2::IOB_V2_EN2
                | tcls_v2::IOB_V2P_SW2
                | tcls_v2::IOB_V2P_SE2
                | tcls_v2::IOB_V2P_SE2_CLK
                | tcls_v2::IOB_V2P_SW1
                | tcls_v2::IOB_V2P_SW1_ALT
                | tcls_v2::IOB_V2P_SE1
                | tcls_v2::IOB_V2P_SE1_ALT
                | tcls_v2::IOB_V2P_NW2
                | tcls_v2::IOB_V2P_NE2
                | tcls_v2::IOB_V2P_NE2_CLK
                | tcls_v2::IOB_V2P_NW1
                | tcls_v2::IOB_V2P_NW1_ALT
                | tcls_v2::IOB_V2P_NE1
                | tcls_v2::IOB_V2P_NE1_ALT
                | tcls_v2::IOB_V2P_WS2
                | tcls_v2::IOB_V2P_WN2
                | tcls_v2::IOB_V2P_ES2
                | tcls_v2::IOB_V2P_EN2,
            )
            | (
                false,
                tcls_s3::IOB_S3_W1
                | tcls_s3::IOB_S3_E1
                | tcls_s3::IOB_S3_S2
                | tcls_s3::IOB_S3_N2
                | tcls_s3::IOB_FC_W
                | tcls_s3::IOB_FC_E
                | tcls_s3::IOB_FC_S
                | tcls_s3::IOB_FC_N
                | tcls_s3::IOB_S3E_W1
                | tcls_s3::IOB_S3E_W2
                | tcls_s3::IOB_S3E_W3
                | tcls_s3::IOB_S3E_W4
                | tcls_s3::IOB_S3E_E1
                | tcls_s3::IOB_S3E_E2
                | tcls_s3::IOB_S3E_E3
                | tcls_s3::IOB_S3E_E4
                | tcls_s3::IOB_S3E_S1
                | tcls_s3::IOB_S3E_S2
                | tcls_s3::IOB_S3E_S3
                | tcls_s3::IOB_S3E_S4
                | tcls_s3::IOB_S3E_N1
                | tcls_s3::IOB_S3E_N2
                | tcls_s3::IOB_S3E_N3
                | tcls_s3::IOB_S3E_N4
                | tcls_s3::IOB_S3A_W4
                | tcls_s3::IOB_S3A_E4
                | tcls_s3::IOB_S3A_S2
                | tcls_s3::IOB_S3A_N2,
            ) => (),

            (true, tcls_v2::DCM_V2 | tcls_v2::DCM_V2P)
            | (
                false,
                tcls_s3::DCM_S3
                | tcls_s3::DCM_S3E_SW
                | tcls_s3::DCM_S3E_SE
                | tcls_s3::DCM_S3E_NW
                | tcls_s3::DCM_S3E_NE
                | tcls_s3::DCM_S3E_WS
                | tcls_s3::DCM_S3E_WN
                | tcls_s3::DCM_S3E_ES
                | tcls_s3::DCM_S3E_EN,
            ) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let naming = if !edev.chip.kind.is_virtex2() && tile.class != tcls_s3::DCM_S3 {
                    if row >= chip.row_mid() - 4 && row < chip.row_mid() + 4 {
                        "DCM_S3E_H"
                    } else if col < chip.col_clk {
                        "DCM_S3E_W"
                    } else {
                        "DCM_S3E_E"
                    }
                } else {
                    tcname
                };
                let ntile = namer.ngrid.name_tile(tcrd, naming, [name]);
                let x = namer.dcm_grid.xlut[col];
                let y = namer.dcm_grid.ylut[row];
                ntile.add_bel(defs::bslots::DCM, format!("DCM_X{x}Y{y}"));
            }
            (true, tcls_v2::DCMCONN_S) | (false, tcls_s3::DCMCONN_S) => {
                let (_, name) = namer.get_bterm_name(col);
                namer.ngrid.name_tile(tcrd, "DCMCONN_S", [name]);
            }
            (true, tcls_v2::DCMCONN_N) | (false, tcls_s3::DCMCONN_N) => {
                let (_, name) = namer.get_tterm_name(col);
                namer.ngrid.name_tile(tcrd, "DCMCONN_N", [name]);
            }

            (true, tcls_v2::CNR_SW_V2 | tcls_v2::CNR_SW_V2P) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, tcname, [name]);
                ntile.add_bel(defs::bslots::DCI[0], "DCI6".to_string());
                ntile.add_bel(defs::bslots::DCI[1], "DCI5".to_string());
            }
            (false, tcls_s3::CNR_SW_S3) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, tcname, [name]);
                ntile.add_bel(defs::bslots::DCI[0], "DCI6".to_string());
                ntile.add_bel(defs::bslots::DCI[1], "DCI5".to_string());
                ntile.add_bel(defs::bslots::DCIRESET[0], "DCIRESET6".to_string());
                ntile.add_bel(defs::bslots::DCIRESET[1], "DCIRESET5".to_string());
            }
            (false, tcls_s3::CNR_SW_FC | tcls_s3::CNR_SW_S3E | tcls_s3::CNR_SW_S3A) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                namer.ngrid.name_tile(tcrd, tcname, [name]);
            }
            (true, tcls_v2::CNR_SE_V2 | tcls_v2::CNR_SE_V2P) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, tcname, [name]);
                ntile.add_bel(defs::bslots::DCI[0], "DCI3".to_string());
                ntile.add_bel(defs::bslots::DCI[1], "DCI4".to_string());
                ntile.add_bel(defs::bslots::STARTUP, "STARTUP".to_string());
                ntile.add_bel(defs::bslots::CAPTURE, "CAPTURE".to_string());
                ntile.add_bel(defs::bslots::ICAP, "ICAP".to_string());
            }
            (false, tcls_s3::CNR_SE_S3) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, tcname, [name]);
                ntile.add_bel(defs::bslots::DCI[0], "DCI3".to_string());
                ntile.add_bel(defs::bslots::DCI[1], "DCI4".to_string());
                ntile.add_bel(defs::bslots::DCIRESET[0], "DCIRESET3".to_string());
                ntile.add_bel(defs::bslots::DCIRESET[1], "DCIRESET4".to_string());
                ntile.add_bel(defs::bslots::STARTUP, "STARTUP".to_string());
                ntile.add_bel(defs::bslots::CAPTURE, "CAPTURE".to_string());
                ntile.add_bel(defs::bslots::ICAP, "ICAP".to_string());
            }
            (false, tcls_s3::CNR_SE_FC | tcls_s3::CNR_SE_S3E) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, tcname, [name]);
                ntile.add_bel(defs::bslots::STARTUP, "STARTUP".to_string());
                ntile.add_bel(defs::bslots::CAPTURE, "CAPTURE".to_string());
                ntile.add_bel(defs::bslots::ICAP, "ICAP".to_string());
            }
            (false, tcls_s3::CNR_SE_S3A) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, tcname, [name]);
                ntile.add_bel(defs::bslots::STARTUP, "STARTUP".to_string());
                ntile.add_bel(defs::bslots::CAPTURE, "CAPTURE".to_string());
                ntile.add_bel(defs::bslots::ICAP, "ICAP".to_string());
                ntile.add_bel(defs::bslots::SPI_ACCESS, "SPI_ACCESS".to_string());
            }
            (true, tcls_v2::CNR_NW_V2 | tcls_v2::CNR_NW_V2P) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, tcname, [name]);
                ntile.add_bel(defs::bslots::DCI[0], "DCI7".to_string());
                ntile.add_bel(defs::bslots::DCI[1], "DCI0".to_string());
                ntile.add_bel(defs::bslots::PMV, "PMV".to_string());
            }
            (false, tcls_s3::CNR_NW_S3) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, tcname, [name]);
                ntile.add_bel(defs::bslots::DCI[0], "DCI7".to_string());
                ntile.add_bel(defs::bslots::DCI[1], "DCI0".to_string());
                ntile.add_bel(defs::bslots::DCIRESET[0], "DCIRESET7".to_string());
                ntile.add_bel(defs::bslots::DCIRESET[1], "DCIRESET0".to_string());
                ntile.add_bel(defs::bslots::PMV, "PMV".to_string());
            }
            (false, tcls_s3::CNR_NW_FC | tcls_s3::CNR_NW_S3E) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, tcname, [name]);
                ntile.add_bel(defs::bslots::PMV, "PMV".to_string());
            }
            (false, tcls_s3::CNR_NW_S3A) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, tcname, [name]);
                ntile.add_bel(defs::bslots::PMV, "PMV".to_string());
                ntile.add_bel(defs::bslots::DNA_PORT, "DNA_PORT".to_string());
            }
            (true, tcls_v2::CNR_NE_V2) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, tcname, [name]);
                ntile.add_bel(defs::bslots::DCI[0], "DCI2".to_string());
                ntile.add_bel(defs::bslots::DCI[1], "DCI1".to_string());
                ntile.add_bel(defs::bslots::BSCAN, "BSCAN".to_string());
            }
            (true, tcls_v2::CNR_NE_V2P) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, tcname, [name]);
                ntile.add_bel(defs::bslots::DCI[0], "DCI2".to_string());
                ntile.add_bel(defs::bslots::DCI[1], "DCI1".to_string());
                ntile.add_bel(defs::bslots::BSCAN, "BSCAN".to_string());
                ntile.add_bel(defs::bslots::JTAGPPC, "JTAGPPC".to_string());
            }
            (false, tcls_s3::CNR_NE_S3) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, tcname, [name]);
                ntile.add_bel(defs::bslots::DCI[0], "DCI2".to_string());
                ntile.add_bel(defs::bslots::DCI[1], "DCI1".to_string());
                ntile.add_bel(defs::bslots::DCIRESET[0], "DCIRESET2".to_string());
                ntile.add_bel(defs::bslots::DCIRESET[1], "DCIRESET1".to_string());
                ntile.add_bel(defs::bslots::BSCAN, "BSCAN".to_string());
            }
            (false, tcls_s3::CNR_NE_FC | tcls_s3::CNR_NE_S3E | tcls_s3::CNR_NE_S3A) => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, tcname, [name]);
                ntile.add_bel(defs::bslots::BSCAN, "BSCAN".to_string());
            }

            _ => panic!("ummm {tcname}?"),
        }
    }
    for (ccrd, conn) in edev.connectors() {
        let CellCoord { col, row, .. } = ccrd.cell;

        match (edev.chip.kind.is_virtex2(), conn.class) {
            (true, ccls_v2::TERM_W) | (false, ccls_s3::TERM_W) => {
                let (naming, name) = namer.get_lterm_name(row);
                namer.ngrid.name_conn_tile(ccrd, naming, name);
            }
            (true, ccls_v2::TERM_E) | (false, ccls_s3::TERM_E) => {
                let (naming, name) = namer.get_rterm_name(row);
                namer.ngrid.name_conn_tile(ccrd, naming, name);
            }
            (true, ccls_v2::TERM_S) | (false, ccls_s3::TERM_S) => {
                let (naming, name) = namer.get_bterm_name(col);
                namer.ngrid.name_conn_tile(ccrd, naming, name);
            }
            (true, ccls_v2::TERM_N) | (false, ccls_s3::TERM_N) => {
                let (naming, name) = namer.get_tterm_name(col);
                namer.ngrid.name_conn_tile(ccrd, naming, name);
            }
            (false, ccls_s3::TERM_BRAM_S) => {
                let x = namer.xlut[col];
                let y = row.to_idx() - 1;
                let name = format!("COB_TERM_T_X{x}Y{y}");
                namer.ngrid.name_conn_tile(ccrd, "TERM_BRAM_S", name);
            }
            (false, ccls_s3::TERM_BRAM_N) => {
                let x = namer.xlut[col];
                let y = row.to_idx() + 1;
                let name = format!("COB_TERM_B_X{x}Y{y}");
                namer.ngrid.name_conn_tile(ccrd, "TERM_BRAM_N", name);
            }
            (true, ccls_v2::PPC_W) => {
                let (name_l, name_r) = namer.get_ppc_h_name(ccrd.cell.delta(-9, 0));
                namer.ngrid.name_conn_pair(ccrd, "PPC_W", name_r, name_l);
            }
            (true, ccls_v2::PPC_E) => {
                let (name_l, name_r) = namer.get_ppc_h_name(ccrd.cell);
                namer.ngrid.name_conn_pair(ccrd, "PPC_E", name_l, name_r);
            }
            (true, ccls_v2::PPC_S) => {
                let (name_b, name_t) = namer.get_ppc_v_name(ccrd.cell.delta(0, -15));
                namer.ngrid.name_conn_pair(ccrd, "PPC_S", name_t, name_b);
            }
            (true, ccls_v2::PPC_N) => {
                let (name_b, name_t) = namer.get_ppc_v_name(ccrd.cell);
                namer.ngrid.name_conn_pair(ccrd, "PPC_N", name_b, name_t);
            }
            (true, ccls_v2::PASS_S) => {
                if chip.kind.is_virtex2()
                    && chip.columns[col].kind == ColumnKind::Bram
                    && chip.bram_row(row) == Some(0)
                    && row.to_idx() != 1
                    && !edev.is_in_hole(ccrd.cell)
                {
                    let (_, name) = namer.get_bram_name(ccrd.cell);
                    namer.ngrid.name_conn_tile(ccrd, "BRAM_S", name);
                }
            }
            _ => (),
        }
    }

    namer.fill_io_t();
    namer.fill_io_r();
    namer.fill_io_b();
    namer.fill_io_l();

    ExpandedNamedDevice {
        edev,
        ngrid: namer.ngrid,
        chip,
    }
}
