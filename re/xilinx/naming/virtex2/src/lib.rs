use std::{cmp::Ordering, collections::HashSet};

use prjcombine_interconnect::grid::{CellCoord, ColId, DieId, EdgeIoCoord, RowId};
use prjcombine_re_xilinx_naming::{
    db::NamingDb,
    grid::{BelGrid, ExpandedGridNaming},
};
use prjcombine_virtex2::{
    bels,
    chip::{Chip, ChipKind, ColumnIoKind, ColumnKind, DcmPairKind, RowIoKind},
    expanded::ExpandedDevice,
    iob::{IobKind, get_iob_data_e, get_iob_data_n, get_iob_data_s, get_iob_data_w},
    tslots,
};
use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};

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
                    "INT.PPC.R"
                } else if cell.row == br {
                    "INT.PPC.B"
                } else if cell.row == br + 15 {
                    "INT.PPC.T"
                } else if cell.col == bc {
                    "INT.PPC.L"
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
                DcmPairKind::Bot => {
                    if cell == pair.cell.delta(-1, 0) {
                        return ("INT.DCM.S3E", format!("DCM_BL_CENTER_X{x}Y{y}"));
                    }
                    if cell == pair.cell {
                        return ("INT.DCM.S3E", format!("DCM_BR_CENTER_X{x}Y{y}"));
                    }
                }
                DcmPairKind::BotSingle => {
                    if cell == pair.cell.delta(-1, 0) {
                        return ("INT.DCM.S3E.DUMMY", format!("DCMAUX_BL_CENTER_X{x}Y{y}"));
                    }
                    if cell == pair.cell {
                        return ("INT.DCM.S3E", format!("DCM_BR_CENTER_X{x}Y{y}"));
                    }
                }
                DcmPairKind::Top => {
                    if cell == pair.cell.delta(-1, 0) {
                        return ("INT.DCM.S3E", format!("DCM_TL_CENTER_X{x}Y{y}"));
                    }
                    if cell == pair.cell {
                        return ("INT.DCM.S3E", format!("DCM_TR_CENTER_X{x}Y{y}"));
                    }
                }
                DcmPairKind::TopSingle => {
                    if cell == pair.cell.delta(-1, 0) {
                        return ("INT.DCM.S3E.DUMMY", format!("DCMAUX_TL_CENTER_X{x}Y{y}"));
                    }
                    if cell == pair.cell {
                        return ("INT.DCM.S3E", format!("DCM_TR_CENTER_X{x}Y{y}"));
                    }
                }
                DcmPairKind::Left => {
                    if cell == pair.cell.delta(0, -1) {
                        return ("INT.DCM.S3E.H", format!("DCM_H_BL_CENTER_X{x}Y{y}"));
                    }
                    if cell == pair.cell {
                        return ("INT.DCM.S3E.H", format!("DCM_H_TL_CENTER_X{x}Y{y}"));
                    }
                }
                DcmPairKind::Right => {
                    if cell == pair.cell.delta(0, -1) {
                        return ("INT.DCM.S3E.H", format!("DCM_H_BR_CENTER_X{x}Y{y}"));
                    }
                    if cell == pair.cell {
                        return ("INT.DCM.S3E.H", format!("DCM_H_TR_CENTER_X{x}Y{y}"));
                    }
                }
                DcmPairKind::Bram => {
                    if cell == pair.cell.delta(0, -1) {
                        return ("INT.DCM.S3E.H", format!("DCM_BGAP_X{x}Y{y}"));
                    }
                    if cell == pair.cell {
                        return ("INT.DCM.S3E.H", format!("DCM_SPLY_X{x}Y{y}"));
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
                ("INT.CNR", format!("{ul}{lr}_X{x}Y{y}"))
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
                    ("INT.CNR", format!("{lr}IOI{bt}IOI"))
                } else {
                    ("INT.CNR", format!("{bt}{lr}"))
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
                ChipKind::Virtex2 => "INT.BRAM_IOIS",
                ChipKind::Virtex2P | ChipKind::Virtex2PX => {
                    if self.chip.cols_gt.contains_key(&cell.col) {
                        "INT.GT.CLKPAD"
                    } else {
                        "INT.ML_BRAM_IOIS"
                    }
                }
                ChipKind::Spartan3 => {
                    if cell.col == self.chip.col_w() + 3 || cell.col == self.chip.col_e() - 3 {
                        "INT.DCM.S3"
                    } else {
                        "INT.DCM.S3.DUMMY"
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
                        "INT.GT"
                    } else {
                        "INT.BRAM"
                    },
                    format!("BRAMR{r}C{c}"),
                )
            } else {
                let idx = self.chip.bram_row(cell.row).unwrap();
                let naming = if self.chip.kind == ChipKind::Spartan3ADsp {
                    if self.rows_brk.contains(&cell.row) {
                        "INT.BRAM.S3ADSP.BRK"
                    } else {
                        "INT.BRAM.S3ADSP"
                    }
                } else {
                    if self.rows_brk.contains(&cell.row) {
                        "INT.BRAM.BRK"
                    } else {
                        "INT.BRAM"
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
                "INT.MACC.BRK"
            } else {
                "INT.MACC"
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
                                "INT.IOI.CLK_B"
                            } else {
                                "INT.IOI.CLK_T"
                            }
                        } else {
                            "INT.IOI.TB"
                        }
                    } else if self.chip.kind == ChipKind::FpgaCore {
                        "INT.IOI.FC"
                    } else {
                        "INT.IOI"
                    };
                    (naming, format!("{bt}IOIC{c}"))
                }
                ChipKind::Spartan3E | ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                    let naming = if self.chip.kind.is_spartan3a() {
                        "INT.IOI.S3A.TB"
                    } else {
                        "INT.IOI"
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
                        "INT.IOI.LR"
                    } else if self.chip.kind == ChipKind::FpgaCore {
                        "INT.IOI.FC"
                    } else {
                        "INT.IOI"
                    };
                    (naming, format!("{lr}IOIR{r}"))
                }
                ChipKind::Spartan3E | ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                    let naming = if self.chip.kind.is_spartan3a() {
                        if self.rows_brk.contains(&cell.row) {
                            "INT.IOI.S3A.LR.BRK"
                        } else {
                            "INT.IOI.S3A.LR"
                        }
                    } else {
                        if self.rows_brk.contains(&cell.row) {
                            "INT.IOI.BRK"
                        } else {
                            "INT.IOI"
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
                ("INT.CLB", format!("R{r}C{c}"))
            } else {
                let naming = if self.rows_brk.contains(&cell.row) {
                    "INT.CLB.BRK"
                } else {
                    "INT.CLB"
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
                    ("TERM.W", "LBTERM".into())
                } else {
                    ("TERM.W", "LTERMBIOI".into())
                }
            } else {
                ("TERM.W", format!("CNR_LBTERM_X{x}Y{y}"))
            }
        } else if row == self.chip.row_n() {
            if !self.chip.kind.is_spartan3ea() {
                if !self.chip.kind.is_virtex2p() {
                    ("TERM.W", "LTTERM".into())
                } else {
                    ("TERM.W", "LTERMTIOI".into())
                }
            } else {
                ("TERM.W", format!("CNR_LTTERM_X{x}Y{y}"))
            }
        } else {
            if !self.chip.kind.is_spartan3ea() {
                let r = self.rlut[row];
                let naming = if self.chip.kind.is_virtex2() {
                    if row < self.chip.row_pci.unwrap() {
                        "TERM.W.D"
                    } else {
                        "TERM.W.U"
                    }
                } else {
                    "TERM.W"
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
                ("TERM.W", format!("{kind}_X{x}Y{y}"))
            }
        }
    }

    fn get_rterm_name(&self, row: RowId) -> (&'static str, String) {
        let x = self.xlut[self.chip.col_e()];
        let y = row.to_idx();
        if row == self.chip.row_s() {
            if !self.chip.kind.is_spartan3ea() {
                if !self.chip.kind.is_virtex2p() {
                    ("TERM.E", "RBTERM".into())
                } else {
                    ("TERM.E", "RTERMBIOI".into())
                }
            } else {
                ("TERM.E", format!("CNR_RBTERM_X{x}Y{y}"))
            }
        } else if row == self.chip.row_n() {
            if !self.chip.kind.is_spartan3ea() {
                if !self.chip.kind.is_virtex2p() {
                    ("TERM.E", "RTTERM".into())
                } else {
                    ("TERM.E", "RTERMTIOI".into())
                }
            } else {
                ("TERM.E", format!("CNR_RTTERM_X{x}Y{y}"))
            }
        } else {
            if !self.chip.kind.is_spartan3ea() {
                let r = self.rlut[row];
                let naming = if self.chip.kind.is_virtex2() {
                    if row < self.chip.row_pci.unwrap() {
                        "TERM.E.D"
                    } else {
                        "TERM.E.U"
                    }
                } else {
                    "TERM.E"
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
                ("TERM.E", format!("{kind}_X{x}Y{y}"))
            }
        }
    }

    fn get_bterm_name(&self, col: ColId) -> (&'static str, String) {
        let x = self.xlut[col];
        let y = self.chip.row_s().to_idx();
        if col == self.chip.col_w() {
            if !self.chip.kind.is_spartan3ea() {
                if !self.chip.kind.is_virtex2p() {
                    ("TERM.S.CNR", "BLTERM".into())
                } else {
                    ("TERM.S.CNR", "LIOIBTERM".into())
                }
            } else {
                ("TERM.S.CNR", format!("CNR_BTERM_X{x}Y{y}"))
            }
        } else if col == self.chip.col_e() {
            if !self.chip.kind.is_spartan3ea() {
                if !self.chip.kind.is_virtex2p() {
                    ("TERM.S.CNR", "BRTERM".into())
                } else {
                    ("TERM.S.CNR", "RIOIBTERM".into())
                }
            } else {
                ("TERM.S.CNR", format!("CNR_BTERM_X{x}Y{y}"))
            }
        } else if !self.chip.kind.is_spartan3ea() && self.chip.columns[col].kind == ColumnKind::Bram
        {
            let c = self.bramclut[col];
            ("TERM.S", format!("BTERMBRAMC{c}"))
        } else {
            if !self.chip.kind.is_spartan3ea() {
                let c = self.clut[col];
                ("TERM.S", format!("BTERMC{c}"))
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
                ("TERM.S", format!("{kind}_X{x}Y{y}"))
            }
        }
    }

    fn get_tterm_name(&self, col: ColId) -> (&'static str, String) {
        let x = self.xlut[col];
        let y = self.chip.row_n().to_idx();
        if col == self.chip.col_w() {
            if !self.chip.kind.is_spartan3ea() {
                if !self.chip.kind.is_virtex2p() {
                    ("TERM.N.CNR", "TLTERM".into())
                } else {
                    ("TERM.N.CNR", "LIOITTERM".into())
                }
            } else {
                ("TERM.N.CNR", format!("CNR_TTERM_X{x}Y{y}"))
            }
        } else if col == self.chip.col_e() {
            if !self.chip.kind.is_spartan3ea() {
                if !self.chip.kind.is_virtex2p() {
                    ("TERM.N.CNR", "TRTERM".into())
                } else {
                    ("TERM.N.CNR", "RIOITTERM".into())
                }
            } else {
                ("TERM.N.CNR", format!("CNR_TTERM_X{x}Y{y}"))
            }
        } else if !self.chip.kind.is_spartan3ea() && self.chip.columns[col].kind == ColumnKind::Bram
        {
            let c = self.bramclut[col];
            ("TERM.N", format!("TTERMBRAMC{c}"))
        } else {
            if !self.chip.kind.is_spartan3ea() {
                let c = self.clut[col];
                ("TERM.N", format!("TTERMC{c}"))
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
                ("TERM.N", format!("{kind}_X{x}Y{y}"))
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
            ChipKind::Spartan3 => "BRAM.S3",
            ChipKind::FpgaCore => unreachable!(),
            ChipKind::Spartan3E => "BRAM.S3E",
            ChipKind::Spartan3A => {
                if is_bot {
                    "BRAM.S3A.BOT"
                } else if is_top {
                    "BRAM.S3A.TOP"
                } else {
                    "BRAM.S3A"
                }
            }
            ChipKind::Spartan3ADsp => "BRAM.S3ADSP",
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
        let naming = if is_top { "DSP.TOP" } else { "DSP" };
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
                ("GCLKH", format!("GCLKHR{r}BRAMC{c}"))
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
                        ("GCLKH", format!("LIOICLKR{r}"))
                    } else {
                        ("GCLKH", format!("RIOICLKR{r}"))
                    }
                } else {
                    ("GCLKH", format!("GCLKHR{r}C{c}"))
                }
            }
        } else {
            let x = self.xlut[cell.col];
            let y = cell.row.to_idx() - 1;
            let mut naming = "GCLKH";
            let kind = match self.chip.columns[cell.col].kind {
                ColumnKind::Io => match cell.row.cmp(&self.chip.row_mid()) {
                    Ordering::Less => "GCLKH_PCI_CE_S",
                    Ordering::Equal => "GCLKH_PCI_CE_S_50A",
                    Ordering::Greater => "GCLKH_PCI_CE_N",
                },
                ColumnKind::BramCont(x) => {
                    if cell.row == self.chip.row_mid() {
                        naming = "GCLKH.BRAM";
                        [
                            "BRAMSITE2_DN_GCLKH",
                            "BRAM2_GCLKH_FEEDTHRU",
                            "BRAM2_GCLKH_FEEDTHRUA",
                        ][x as usize - 1]
                    } else if self.hclklut[cell.row] == 0 {
                        naming = "GCLKH.BRAM.S";
                        if self.chip.kind == ChipKind::Spartan3E {
                            [
                                "BRAMSITE2_DN_GCLKH",
                                "BRAM2_DN_GCLKH_FEEDTHRU",
                                "BRAM2_DN_GCLKH_FEEDTHRUA",
                            ][x as usize - 1]
                        } else {
                            [
                                "BRAMSITE2_DN_GCLKH",
                                "BRAM2_GCLKH_FEEDTHRU",
                                "BRAM2_GCLKH_FEEDTHRUA",
                            ][x as usize - 1]
                        }
                    } else if self.hclklut[cell.row] == self.chip.rows_hclk.len() - 1 {
                        naming = "GCLKH.BRAM.N";
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
                        naming = "GCLKH.0";
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
            "LLV.CLKL"
        } else if col == self.chip.col_e() {
            "LLV.CLKR"
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
                .get_mut(&CellCoord::new(self.die, col, row).tile(tslots::BEL))
                .unwrap();
            for &i in iobs {
                let slot = if self.chip.kind == ChipKind::FpgaCore {
                    if i < 4 {
                        bels::IBUF[i]
                    } else {
                        bels::OBUF[i - 4]
                    }
                } else {
                    bels::IO[i]
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
                .get_mut(&CellCoord::new(self.die, col, row).tile(tslots::BEL))
                .unwrap();
            for &i in iobs {
                let slot = if self.chip.kind == ChipKind::FpgaCore {
                    if i < 4 {
                        bels::IBUF[i]
                    } else {
                        bels::OBUF[i - 4]
                    }
                } else {
                    bels::IO[i]
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
                .get_mut(&CellCoord::new(self.die, col, row).tile(tslots::BEL))
                .unwrap();
            for &i in iobs {
                let slot = if self.chip.kind == ChipKind::FpgaCore {
                    if i < 4 {
                        bels::IBUF[i]
                    } else {
                        bels::OBUF[i - 4]
                    }
                } else {
                    bels::IO[i]
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
                .get_mut(&CellCoord::new(self.die, col, row).tile(tslots::BEL))
                .unwrap();
            for &i in iobs {
                let slot = if self.chip.kind == ChipKind::FpgaCore {
                    if i < 4 {
                        bels::IBUF[i]
                    } else {
                        bels::OBUF[i - 4]
                    }
                } else {
                    bels::IO[i]
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
    let dcm_grid = ngrid.bel_grid(|_, name, _| name.starts_with("DCM."));
    let bram_grid = ngrid.bel_grid(|_, name, _| name.starts_with("BRAM"));
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
        let kind = edev.db.tile_classes.key(tile.class);
        match &kind[..] {
            _ if kind.starts_with("INT.") => {
                let (naming, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, naming, [name]);
                let x = col.to_idx();
                let y = row.to_idx();
                ntile.add_bel(bels::RLL, format!("RLL_X{x}Y{y}"));
                if kind != "INT.DCM.S3E.DUMMY" {
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
            "INTF.PPC" => {
                let (naming, name) = namer.get_int_name(tcrd.cell);
                let naming = format!("INTF.{}", &naming[4..]);
                namer.ngrid.name_tile(tcrd, &naming, [name]);
            }
            "INTF.GT.BCLKPAD" | "INTF.GT.TCLKPAD" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                namer.ngrid.name_tile(tcrd, "INTF.GT.CLKPAD", [name]);
            }
            "INTF.GT.B0" | "INTF.GT.B123" | "INTF.GT.T0" | "INTF.GT.T123" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                namer.ngrid.name_tile(tcrd, "INTF.GT", [name]);
            }
            "CLB" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, "CLB", [name]);
                let sx = namer.sxlut[col];
                let sy = 2 * (row.to_idx() - 1);
                if chip.kind.is_virtex2() {
                    ntile.add_bel(bels::SLICE0, format!("SLICE_X{sx}Y{sy}"));
                    ntile.add_bel(bels::SLICE1, format!("SLICE_X{x}Y{y}", x = sx, y = sy + 1));
                    ntile.add_bel(bels::SLICE2, format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy));
                    ntile.add_bel(
                        bels::SLICE3,
                        format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy + 1),
                    );
                    if sx.is_multiple_of(4) {
                        ntile.add_bel(bels::TBUF0, format!("TBUF_X{sx}Y{sy}"));
                        ntile.add_bel(bels::TBUF1, format!("TBUF_X{x}Y{y}", x = sx, y = sy + 1));
                    } else {
                        ntile.add_bel(bels::TBUF0, format!("TBUF_X{x}Y{y}", x = sx, y = sy + 1));
                        ntile.add_bel(bels::TBUF1, format!("TBUF_X{sx}Y{sy}"));
                    }
                } else {
                    ntile.add_bel(bels::SLICE0, format!("SLICE_X{sx}Y{sy}"));
                    ntile.add_bel(bels::SLICE1, format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy));
                    ntile.add_bel(bels::SLICE2, format!("SLICE_X{x}Y{y}", x = sx, y = sy + 1));
                    ntile.add_bel(
                        bels::SLICE3,
                        format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy + 1),
                    );
                }
            }
            "RANDOR" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let naming = if row == chip.row_s() {
                    "RANDOR.B"
                } else if row == chip.row_n() {
                    "RANDOR.T"
                } else {
                    unreachable!()
                };
                let ntile = namer.ngrid.name_tile(tcrd, naming, [name]);
                let x = if chip.kind == ChipKind::Spartan3 {
                    (namer.clut[col] - 1) * 2
                } else {
                    col.to_idx() - 1
                };
                let y = if chip.kind != ChipKind::FpgaCore && naming == "RANDOR.T" {
                    1
                } else {
                    0
                };
                ntile.add_bel(bels::RANDOR, format!("RANDOR_X{x}Y{y}"));
            }
            "RANDOR_INIT" => {}
            "BRAM" | "BRAM.S3" | "BRAM.S3E" | "BRAM.S3A" | "BRAM.S3ADSP" => {
                let (naming, name) = namer.get_bram_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, naming, [name]);
                let x = namer.bram_grid.xlut[col];
                let y = namer.bram_grid.ylut[row];
                ntile.add_bel(bels::BRAM, format!("RAMB16_X{x}Y{y}"));
                if chip.kind != ChipKind::Spartan3ADsp {
                    ntile.add_bel(bels::MULT, format!("MULT18X18_X{x}Y{y}"));
                }
            }
            "DSP" => {
                let (naming, name) = namer.get_dsp_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, naming, [name]);
                let x = namer.bram_grid.xlut[col - 3];
                let y = namer.bram_grid.ylut[row];
                ntile.add_bel(bels::DSP, format!("DSP48A_X{x}Y{y}"));
            }
            "INTF.DSP" => {
                let (_, name) = namer.get_dsp_name(tcrd.cell);
                namer.ngrid.name_tile(tcrd, "INTF.DSP", [name]);
            }
            "GIGABIT.B" => {
                let c = namer.bramclut[col];
                let r = namer.rlut[row + 1];
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, "GIGABIT.B", [format!("BMR{r}C{c}")]);
                let gx = namer.gtxlut[col];
                let (bank, _) = chip.cols_gt[&col];
                ntile.add_bel(bels::GT, format!("GT_X{gx}Y0"));
                ntile.add_bel(bels::IPAD_RXP, format!("RXPPAD{bank}"));
                ntile.add_bel(bels::IPAD_RXN, format!("RXNPAD{bank}"));
                ntile.add_bel(bels::OPAD_TXP, format!("TXPPAD{bank}"));
                ntile.add_bel(bels::OPAD_TXN, format!("TXNPAD{bank}"));
            }
            "GIGABIT10.B" => {
                let c = namer.bramclut[col];
                let r = namer.rlut[row + 1];
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, "GIGABIT10.B", [format!("BMR{r}C{c}")]);
                let gx = namer.gtxlut[col];
                let (bank, _) = chip.cols_gt[&col];
                ntile.add_bel(bels::GT10, format!("GT10_X{gx}Y0"));
                ntile.add_bel(bels::IPAD_RXP, format!("RXPPAD{bank}"));
                ntile.add_bel(bels::IPAD_RXN, format!("RXNPAD{bank}"));
                ntile.add_bel(bels::OPAD_TXP, format!("TXPPAD{bank}"));
                ntile.add_bel(bels::OPAD_TXN, format!("TXNPAD{bank}"));
            }
            "GIGABIT.T" => {
                let c = namer.bramclut[col];
                let r = namer.rlut[row - 4];
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, "GIGABIT.T", [format!("BMR{r}C{c}")]);
                let gx = namer.gtxlut[col];
                let (_, bank) = chip.cols_gt[&col];
                ntile.add_bel(bels::GT, format!("GT_X{gx}Y1"));
                ntile.add_bel(bels::IPAD_RXP, format!("RXPPAD{bank}"));
                ntile.add_bel(bels::IPAD_RXN, format!("RXNPAD{bank}"));
                ntile.add_bel(bels::OPAD_TXP, format!("TXPPAD{bank}"));
                ntile.add_bel(bels::OPAD_TXN, format!("TXNPAD{bank}"));
            }
            "GIGABIT10.T" => {
                let c = namer.bramclut[col];
                let r = namer.rlut[row - 8];
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, "GIGABIT10.T", [format!("BMR{r}C{c}")]);
                let gx = namer.gtxlut[col];
                let (_, bank) = chip.cols_gt[&col];
                ntile.add_bel(bels::GT10, format!("GT10_X{gx}Y1"));
                ntile.add_bel(bels::IPAD_RXP, format!("RXPPAD{bank}"));
                ntile.add_bel(bels::IPAD_RXN, format!("RXNPAD{bank}"));
                ntile.add_bel(bels::OPAD_TXP, format!("TXPPAD{bank}"));
                ntile.add_bel(bels::OPAD_TXN, format!("TXNPAD{bank}"));
            }
            "LBPPC" | "RBPPC" => {
                let x = if kind == "LBPPC" || chip.holes_ppc.len() == 1 {
                    0
                } else {
                    1
                };
                let ntile = namer.ngrid.name_tile(tcrd, kind, [format!("PPC_X{x}Y0")]);
                ntile.add_bel(bels::PPC405, format!("PPC405_X{x}Y0"));
            }
            "TERM.W" => {
                let (naming, name) = namer.get_lterm_name(row);
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            "TERM.E" => {
                let (naming, name) = namer.get_rterm_name(row);
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            "TERM.S" => {
                let (naming, name) = namer.get_bterm_name(col);
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            "TERM.N" => {
                let (naming, name) = namer.get_tterm_name(col);
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            "PPC.E" => {
                let (name_l, name_r) = namer.get_ppc_h_name(tcrd.cell);
                namer.ngrid.name_tile(tcrd, "PPC.E", [name_l, name_r]);
            }
            "PPC.W" => {
                let (name_l, name_r) = namer.get_ppc_h_name(tcrd.cell.delta(-9, 0));
                namer.ngrid.name_tile(tcrd, "PPC.W", [name_r, name_l]);
            }
            "PPC.N" => {
                let (name_b, name_t) = namer.get_ppc_v_name(tcrd.cell);
                namer.ngrid.name_tile(tcrd, "PPC.N", [name_b, name_t]);
            }
            "PPC.S" => {
                let (name_b, name_t) = namer.get_ppc_v_name(tcrd.cell.delta(0, -15));
                namer.ngrid.name_tile(tcrd, "PPC.S", [name_t, name_b]);
            }
            "LLV.S3E" | "LLV.S3A" => {
                let (naming, name) = namer.get_llv_name(col);
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            "LLH" | "LLH.CLKB.S3A" | "LLH.CLKT.S3A" => {
                let name = namer.get_llh_name(row);
                namer.ngrid.name_tile(tcrd, "LLH", [name]);
            }
            "CLKB.V2" | "CLKB.V2P" | "CLKB.V2PX" => {
                let name = match chip.kind {
                    ChipKind::Virtex2 => "CLKB",
                    ChipKind::Virtex2P => "ML_CLKB",
                    ChipKind::Virtex2PX => "MK_CLKB",
                    _ => unreachable!(),
                };
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name.into()]);
                let vx = namer.vcc_xlut[chip.col_clk] - 1;
                let vy = chip.row_s().to_idx();
                ntile.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                ntile.add_bel(bels::BUFGMUX0, "BUFGMUX0P".to_string());
                ntile.add_bel(bels::BUFGMUX1, "BUFGMUX1S".to_string());
                ntile.add_bel(bels::BUFGMUX2, "BUFGMUX2P".to_string());
                ntile.add_bel(bels::BUFGMUX3, "BUFGMUX3S".to_string());
                ntile.add_bel(bels::BUFGMUX4, "BUFGMUX4P".to_string());
                ntile.add_bel(bels::BUFGMUX5, "BUFGMUX5S".to_string());
                ntile.add_bel(bels::BUFGMUX6, "BUFGMUX6P".to_string());
                ntile.add_bel(bels::BUFGMUX7, "BUFGMUX7S".to_string());
                ntile.add_bel(
                    bels::GLOBALSIG_S0,
                    format!("GSIG_X{x}Y0", x = chip.col_clk.to_idx()),
                );
                ntile.add_bel(
                    bels::GLOBALSIG_S1,
                    format!("GSIG_X{x}Y0", x = chip.col_clk.to_idx() + 1),
                );
            }
            "CLKT.V2" | "CLKT.V2P" | "CLKT.V2PX" => {
                let name = match chip.kind {
                    ChipKind::Virtex2 => "CLKT",
                    ChipKind::Virtex2P => "ML_CLKT",
                    ChipKind::Virtex2PX => "MK_CLKT",
                    _ => unreachable!(),
                };
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name.into()]);
                let vx = namer.vcc_xlut[chip.col_clk] - 1;
                let vy = if chip.kind == ChipKind::Virtex2 {
                    1
                } else {
                    chip.rows.len() - 1
                };
                ntile.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                ntile.add_bel(bels::BUFGMUX0, "BUFGMUX0S".to_string());
                ntile.add_bel(bels::BUFGMUX1, "BUFGMUX1P".to_string());
                ntile.add_bel(bels::BUFGMUX2, "BUFGMUX2S".to_string());
                ntile.add_bel(bels::BUFGMUX3, "BUFGMUX3P".to_string());
                ntile.add_bel(bels::BUFGMUX4, "BUFGMUX4S".to_string());
                ntile.add_bel(bels::BUFGMUX5, "BUFGMUX5P".to_string());
                ntile.add_bel(bels::BUFGMUX6, "BUFGMUX6S".to_string());
                ntile.add_bel(bels::BUFGMUX7, "BUFGMUX7P".to_string());
                ntile.add_bel(
                    bels::GLOBALSIG_N0,
                    format!("GSIG_X{x}Y1", x = chip.col_clk.to_idx()),
                );
                ntile.add_bel(
                    bels::GLOBALSIG_N1,
                    format!("GSIG_X{x}Y1", x = chip.col_clk.to_idx() + 1),
                );
            }
            "CLKB.S3" | "CLKB.FC" => {
                let bufg = if chip.kind == ChipKind::FpgaCore {
                    "BUFG"
                } else {
                    "BUFGMUX"
                };
                let ntile = namer.ngrid.name_tile(tcrd, kind, ["CLKB".into()]);
                let vx = namer.vcc_xlut[chip.col_clk] - 1;
                let vy = 0;
                ntile.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                ntile.add_bel(bels::BUFGMUX0, format!("{bufg}0"));
                ntile.add_bel(bels::BUFGMUX1, format!("{bufg}1"));
                ntile.add_bel(bels::BUFGMUX2, format!("{bufg}2"));
                ntile.add_bel(bels::BUFGMUX3, format!("{bufg}3"));
                ntile.add_bel(
                    bels::GLOBALSIG_S,
                    format!("GSIG_X{x}Y0", x = chip.col_clk.to_idx()),
                );
            }
            "CLKT.S3" | "CLKT.FC" => {
                let bufg = if chip.kind == ChipKind::FpgaCore {
                    "BUFG"
                } else {
                    "BUFGMUX"
                };
                let ntile = namer.ngrid.name_tile(tcrd, kind, ["CLKT".into()]);
                let vx = namer.vcc_xlut[chip.col_clk] - 1;
                let vy = namer.vcc_ylut[chip.row_n()];
                ntile.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                ntile.add_bel(bels::BUFGMUX0, format!("{bufg}4"));
                ntile.add_bel(bels::BUFGMUX1, format!("{bufg}5"));
                ntile.add_bel(bels::BUFGMUX2, format!("{bufg}6"));
                ntile.add_bel(bels::BUFGMUX3, format!("{bufg}7"));
                ntile.add_bel(
                    bels::GLOBALSIG_N,
                    format!("GSIG_X{x}Y1", x = chip.col_clk.to_idx()),
                );
            }
            "CLKB.S3E" | "CLKB.S3A" => {
                let x = namer.xlut[chip.col_clk - 1];
                let y = row.to_idx();
                let yb = y + 1;
                let (name, name_buf) = if chip.has_ll {
                    (format!("CLKB_LL_X{x}Y{y}"), format!("CLKV_LL_X{x}Y{yb}"))
                } else {
                    (format!("CLKB_X{x}Y{y}"), format!("CLKV_X{x}Y{yb}"))
                };
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name, name_buf]);
                let vx = namer.vcc_xlut[chip.col_clk] - 1;
                let vy = 0;
                ntile.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                ntile.add_bel(bels::BUFGMUX0, "BUFGMUX_X2Y1".to_string());
                ntile.add_bel(bels::BUFGMUX1, "BUFGMUX_X2Y0".to_string());
                ntile.add_bel(bels::BUFGMUX2, "BUFGMUX_X1Y1".to_string());
                ntile.add_bel(bels::BUFGMUX3, "BUFGMUX_X1Y0".to_string());
                ntile.add_bel(
                    bels::GLOBALSIG_S,
                    format!("GLOBALSIG_X{x}Y0", x = namer.xlut[chip.col_clk] + 1),
                );
            }
            "CLKT.S3E" | "CLKT.S3A" => {
                let x = namer.xlut[chip.col_clk - 1];
                let y = row.to_idx();
                let yb = y - 1;
                let (name, name_buf) = if chip.has_ll {
                    (format!("CLKT_LL_X{x}Y{y}"), format!("CLKV_LL_X{x}Y{yb}"))
                } else {
                    (format!("CLKT_X{x}Y{y}"), format!("CLKV_X{x}Y{yb}"))
                };
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name, name_buf]);
                let vx = namer.vcc_xlut[chip.col_clk] - 1;
                let vy = namer.vcc_ylut[chip.row_n()];
                ntile.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                ntile.add_bel(bels::BUFGMUX0, "BUFGMUX_X2Y11".to_string());
                ntile.add_bel(bels::BUFGMUX1, "BUFGMUX_X2Y10".to_string());
                ntile.add_bel(bels::BUFGMUX2, "BUFGMUX_X1Y11".to_string());
                ntile.add_bel(bels::BUFGMUX3, "BUFGMUX_X1Y10".to_string());
                ntile.add_bel(
                    bels::GLOBALSIG_N,
                    format!(
                        "GLOBALSIG_X{x}Y{y}",
                        x = namer.xlut[chip.col_clk] + 1,
                        y = chip.rows_hclk.len() + 2
                    ),
                );
            }
            "CLKL.S3E" | "CLKL.S3A" => {
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
                let ntile = namer.ngrid.name_tile(tcrd, kind, names);
                let vy = namer.vcc_ylut[chip.row_mid()] - 1;
                let vx = 0;
                let gsy = chip.rows_hclk.len().div_ceil(2) + 1;
                ntile.add_bel(bels::BUFGMUX0, "BUFGMUX_X0Y2".to_string());
                ntile.add_bel(bels::BUFGMUX1, "BUFGMUX_X0Y3".to_string());
                ntile.add_bel(bels::BUFGMUX2, "BUFGMUX_X0Y4".to_string());
                ntile.add_bel(bels::BUFGMUX3, "BUFGMUX_X0Y5".to_string());
                ntile.add_bel(bels::BUFGMUX4, "BUFGMUX_X0Y6".to_string());
                ntile.add_bel(bels::BUFGMUX5, "BUFGMUX_X0Y7".to_string());
                ntile.add_bel(bels::BUFGMUX6, "BUFGMUX_X0Y8".to_string());
                ntile.add_bel(bels::BUFGMUX7, "BUFGMUX_X0Y9".to_string());
                ntile.add_bel(bels::PCILOGICSE, "PCILOGIC_X0Y0".to_string());
                ntile.add_bel(bels::VCC, format!("VCC_X{vx}Y{vy}"));
                ntile.add_bel(bels::GLOBALSIG_WE, format!("GLOBALSIG_X0Y{gsy}"));
            }
            "CLKR.S3E" | "CLKR.S3A" => {
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
                let ntile = namer.ngrid.name_tile(tcrd, kind, names);
                let vy = namer.vcc_ylut[chip.row_mid()] - 1;
                let vx = namer.vcc_xlut[chip.col_e()] + 1;
                let gsy = chip.rows_hclk.len().div_ceil(2) + 1;
                ntile.add_bel(bels::BUFGMUX0, "BUFGMUX_X3Y2".to_string());
                ntile.add_bel(bels::BUFGMUX1, "BUFGMUX_X3Y3".to_string());
                ntile.add_bel(bels::BUFGMUX2, "BUFGMUX_X3Y4".to_string());
                ntile.add_bel(bels::BUFGMUX3, "BUFGMUX_X3Y5".to_string());
                ntile.add_bel(bels::BUFGMUX4, "BUFGMUX_X3Y6".to_string());
                ntile.add_bel(bels::BUFGMUX5, "BUFGMUX_X3Y7".to_string());
                ntile.add_bel(bels::BUFGMUX6, "BUFGMUX_X3Y8".to_string());
                ntile.add_bel(bels::BUFGMUX7, "BUFGMUX_X3Y9".to_string());
                ntile.add_bel(bels::PCILOGICSE, "PCILOGIC_X1Y0".to_string());
                ntile.add_bel(bels::VCC, format!("VCC_X{vx}Y{vy}"));
                ntile.add_bel(
                    bels::GLOBALSIG_WE,
                    format!("GLOBALSIG_X{x}Y{gsy}", x = namer.xlut[chip.col_e()] + 3),
                );
            }

            "REG_L" => {
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
                ntile.add_bel(bels::PCILOGIC, "PCILOGIC_X0Y0".into());
            }
            "REG_R" => {
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
                ntile.add_bel(bels::PCILOGIC, "PCILOGIC_X1Y0".into());
            }
            "GCLKH" | "GCLKH.UNI" | "GCLKH.S" | "GCLKH.UNI.S" | "GCLKH.N" | "GCLKH.UNI.N"
            | "GCLKH.0" => {
                let (naming, name) = namer.get_hclk_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, naming, [name]);
                if !chip.kind.is_spartan3ea() {
                    let gsx = if col < chip.col_clk {
                        col.to_idx()
                    } else if !chip.kind.is_virtex2() {
                        col.to_idx() + 1
                    } else {
                        col.to_idx() + 2
                    };
                    let gsy = namer.hclklut[row];
                    ntile.add_bel(bels::GLOBALSIG, format!("GSIG_X{gsx}Y{gsy}"));
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
                    ntile.add_bel(bels::GLOBALSIG, format!("GLOBALSIG_X{gsx}Y{gsy}"));
                }
            }
            "GCLKH.DSP" => {
                let name = format!(
                    "MACC2_GCLKH_FEEDTHRUA_X{x}Y{y}",
                    x = namer.xlut[col] + 1,
                    y = row.to_idx() - 1
                );
                let ntile = namer.ngrid.name_tile(tcrd, "GCLKH.DSP", [name]);
                let gsx = if col < chip.col_clk {
                    namer.xlut[col] + 1
                } else {
                    namer.xlut[col] + 2
                } + 1;
                let gsy = if row <= chip.row_mid() {
                    namer.hclklut[row] + 1
                } else {
                    namer.hclklut[row] + 2
                };
                ntile.add_bel(bels::GLOBALSIG_DSP, format!("GLOBALSIG_X{gsx}Y{gsy}"));
            }
            "PCI_CE_CNR" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                namer.ngrid.name_tile(tcrd, "PCI_CE_CNR", [name]);
            }
            "PCI_CE_N" | "PCI_CE_S" => {
                let (_, name) = namer.get_hclk_name(tcrd.cell);
                namer.ngrid.name_tile(tcrd, kind, [name]);
            }
            "PCI_CE_E" => {
                let x = namer.xlut[col] - 1;
                let y = row.to_idx();
                let name = format!("GCLKV_IOISL_X{x}Y{y}");
                namer.ngrid.name_tile(tcrd, "PCI_CE_E", [name]);
            }
            "PCI_CE_W" => {
                let x = namer.xlut[col] - 1;
                let y = row.to_idx();
                let name = format!("GCLKV_IOISR_X{x}Y{y}");
                namer.ngrid.name_tile(tcrd, "PCI_CE_W", [name]);
            }
            "GCLKC" | "GCLKC.B" | "GCLKC.T" => {
                let mut r = chip.rows_hclk.len() - namer.hclklut[row];
                // I hate ISE.
                if chip.columns.len() == 12 {
                    r -= 1;
                }
                let name = format!("GCLKCR{r}");
                namer.ngrid.name_tile(tcrd, "GCLKC", [name]);
            }
            "GCLKVC" => {
                let name = if !chip.kind.is_spartan3ea() {
                    let r = chip.rows_hclk.len() - namer.hclklut[row];
                    let lr = if col < chip.col_clk { 'L' } else { 'R' };
                    format!("{lr}CLKVCR{r}")
                } else {
                    let x = namer.xlut[col] - 1;
                    let y = row.to_idx() - 1;
                    format!("GCLKVC_X{x}Y{y}")
                };
                namer.ngrid.name_tile(tcrd, "GCLKVC", [name]);
            }
            "CLKC" => {
                let name = if chip.kind.is_spartan3ea() {
                    let x = namer.xlut[col] - 1;
                    let y = row.to_idx() - 1;
                    if chip.kind == ChipKind::Spartan3E && chip.has_ll {
                        format!("CLKC_LL_X{x}Y{y}")
                    } else {
                        format!("CLKC_X{x}Y{y}")
                    }
                } else {
                    "M".to_string()
                };
                namer.ngrid.name_tile(tcrd, "CLKC", [name]);
            }
            "CLKC_50A" => {
                let x = namer.xlut[col] - 1;
                let y = row.to_idx() - 1;
                let name = format!("CLKC_50A_X{x}Y{y}");
                namer.ngrid.name_tile(tcrd, "CLKC_50A", [name]);
            }
            "GCLKVM.S3" => {
                let lr = if col < chip.col_clk { 'L' } else { 'R' };
                let name = format!("{lr}GCLKVM");
                namer.ngrid.name_tile(tcrd, "GCLKVM.S3", [name]);
            }
            "GCLKVM.S3E" => {
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

            "IOI" | "IOI.CLK_B" | "IOI.CLK_T" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let naming = if matches!(
                    chip.columns[col].io,
                    ColumnIoKind::SingleLeftAlt | ColumnIoKind::SingleRightAlt
                ) {
                    "IOI.TBS"
                } else {
                    kind
                };
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            "IOI.S3" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let naming = if col == chip.col_w() {
                    "IOI.S3.L"
                } else if col == chip.col_e() {
                    "IOI.S3.R"
                } else if row == chip.row_s() {
                    "IOI.S3.B"
                } else if row == chip.row_n() {
                    "IOI.S3.T"
                } else {
                    unreachable!()
                };
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            "IOI.FC" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let naming = if col == chip.col_w() {
                    "IOI.FC.L"
                } else if col == chip.col_e() {
                    "IOI.FC.R"
                } else if row == chip.row_s() {
                    "IOI.FC.B"
                } else if row == chip.row_n() {
                    "IOI.FC.T"
                } else {
                    unreachable!()
                };
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            "IOI.S3E" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let naming = if col == chip.col_w() {
                    if row >= chip.row_mid() - 4 && row < chip.row_mid() + 4 {
                        if row == chip.row_mid() - 4 || row == chip.row_mid() {
                            "IOI.S3E.L.PCI"
                        } else {
                            "IOI.S3E.L.PCI.PCI"
                        }
                    } else {
                        "IOI.S3E.L"
                    }
                } else if col == chip.col_e() {
                    if row >= chip.row_mid() - 4 && row < chip.row_mid() + 4 {
                        if row == chip.row_mid() - 1 || row == chip.row_mid() + 3 {
                            "IOI.S3E.R.PCI"
                        } else {
                            "IOI.S3E.R.PCI.PCI"
                        }
                    } else {
                        "IOI.S3E.R"
                    }
                } else if row == chip.row_s() {
                    "IOI.S3E.B"
                } else if row == chip.row_n() {
                    "IOI.S3E.T"
                } else {
                    unreachable!()
                };
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            "IOI.S3A.B" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let naming = if chip.kind == ChipKind::Spartan3ADsp {
                    "IOI.S3ADSP.B"
                } else {
                    "IOI.S3A.B"
                };
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            "IOI.S3A.T" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let naming = if chip.kind == ChipKind::Spartan3ADsp {
                    "IOI.S3ADSP.T"
                } else {
                    "IOI.S3A.T"
                };
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            "IOI.S3A.LR" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let naming = if col == chip.col_w() {
                    if row >= chip.row_mid() - 4
                        && row < chip.row_mid() + 4
                        && row != chip.row_mid() - 4
                        && row != chip.row_mid()
                    {
                        if chip.kind == ChipKind::Spartan3ADsp {
                            "IOI.S3ADSP.L.PCI"
                        } else {
                            "IOI.S3A.L.PCI"
                        }
                    } else {
                        if chip.kind == ChipKind::Spartan3ADsp {
                            "IOI.S3ADSP.L"
                        } else {
                            "IOI.S3A.L"
                        }
                    }
                } else if col == chip.col_e() {
                    if row >= chip.row_mid() - 4
                        && row < chip.row_mid() + 4
                        && row != chip.row_mid() - 1
                        && row != chip.row_mid() + 3
                    {
                        if chip.kind == ChipKind::Spartan3ADsp {
                            "IOI.S3ADSP.R.PCI"
                        } else {
                            "IOI.S3A.R.PCI"
                        }
                    } else {
                        if chip.kind == ChipKind::Spartan3ADsp {
                            "IOI.S3ADSP.R"
                        } else {
                            "IOI.S3A.R"
                        }
                    }
                } else {
                    unreachable!()
                };
                namer.ngrid.name_tile(tcrd, naming, [name]);
            }
            _ if kind.starts_with("IOBS.") => (),

            _ if kind.starts_with("DCM.") => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let naming = if kind.starts_with("DCM.S3E") {
                    if row >= chip.row_mid() - 4 && row < chip.row_mid() + 4 {
                        "DCM.S3E.H"
                    } else if col < chip.col_clk {
                        "DCM.S3E.L"
                    } else {
                        "DCM.S3E.R"
                    }
                } else {
                    kind
                };
                let ntile = namer.ngrid.name_tile(tcrd, naming, [name]);
                let x = namer.dcm_grid.xlut[col];
                let y = namer.dcm_grid.ylut[row];
                ntile.add_bel(bels::DCM, format!("DCM_X{x}Y{y}"));
            }
            "DCMCONN.BOT" => {
                let (_, name) = namer.get_bterm_name(col);
                namer.ngrid.name_tile(tcrd, "DCMCONN.BOT", [name]);
            }
            "DCMCONN.TOP" => {
                let (_, name) = namer.get_tterm_name(col);
                namer.ngrid.name_tile(tcrd, "DCMCONN.TOP", [name]);
            }

            "LL.V2" | "LL.V2P" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name]);
                ntile.add_bel(bels::DCI0, "DCI6".to_string());
                ntile.add_bel(bels::DCI1, "DCI5".to_string());
            }
            "LL.S3" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name]);
                ntile.add_bel(bels::DCI0, "DCI6".to_string());
                ntile.add_bel(bels::DCI1, "DCI5".to_string());
                ntile.add_bel(bels::DCIRESET0, "DCIRESET6".to_string());
                ntile.add_bel(bels::DCIRESET1, "DCIRESET5".to_string());
            }
            "LL.FC" | "LL.S3E" | "LL.S3A" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                namer.ngrid.name_tile(tcrd, kind, [name]);
            }
            "LR.V2" | "LR.V2P" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name]);
                ntile.add_bel(bels::DCI0, "DCI3".to_string());
                ntile.add_bel(bels::DCI1, "DCI4".to_string());
                ntile.add_bel(bels::STARTUP, "STARTUP".to_string());
                ntile.add_bel(bels::CAPTURE, "CAPTURE".to_string());
                ntile.add_bel(bels::ICAP, "ICAP".to_string());
            }
            "LR.S3" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name]);
                ntile.add_bel(bels::DCI0, "DCI3".to_string());
                ntile.add_bel(bels::DCI1, "DCI4".to_string());
                ntile.add_bel(bels::DCIRESET0, "DCIRESET3".to_string());
                ntile.add_bel(bels::DCIRESET1, "DCIRESET4".to_string());
                ntile.add_bel(bels::STARTUP, "STARTUP".to_string());
                ntile.add_bel(bels::CAPTURE, "CAPTURE".to_string());
                ntile.add_bel(bels::ICAP, "ICAP".to_string());
            }
            "LR.FC" | "LR.S3E" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name]);
                ntile.add_bel(bels::STARTUP, "STARTUP".to_string());
                ntile.add_bel(bels::CAPTURE, "CAPTURE".to_string());
                ntile.add_bel(bels::ICAP, "ICAP".to_string());
            }
            "LR.S3A" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name]);
                ntile.add_bel(bels::STARTUP, "STARTUP".to_string());
                ntile.add_bel(bels::CAPTURE, "CAPTURE".to_string());
                ntile.add_bel(bels::ICAP, "ICAP".to_string());
                ntile.add_bel(bels::SPI_ACCESS, "SPI_ACCESS".to_string());
            }
            "UL.V2" | "UL.V2P" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name]);
                ntile.add_bel(bels::DCI0, "DCI7".to_string());
                ntile.add_bel(bels::DCI1, "DCI0".to_string());
                ntile.add_bel(bels::PMV, "PMV".to_string());
            }
            "UL.S3" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name]);
                ntile.add_bel(bels::DCI0, "DCI7".to_string());
                ntile.add_bel(bels::DCI1, "DCI0".to_string());
                ntile.add_bel(bels::DCIRESET0, "DCIRESET7".to_string());
                ntile.add_bel(bels::DCIRESET1, "DCIRESET0".to_string());
                ntile.add_bel(bels::PMV, "PMV".to_string());
            }
            "UL.FC" | "UL.S3E" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name]);
                ntile.add_bel(bels::PMV, "PMV".to_string());
            }
            "UL.S3A" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name]);
                ntile.add_bel(bels::PMV, "PMV".to_string());
                ntile.add_bel(bels::DNA_PORT, "DNA_PORT".to_string());
            }
            "UR.V2" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name]);
                ntile.add_bel(bels::DCI0, "DCI2".to_string());
                ntile.add_bel(bels::DCI1, "DCI1".to_string());
                ntile.add_bel(bels::BSCAN, "BSCAN".to_string());
            }
            "UR.V2P" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name]);
                ntile.add_bel(bels::DCI0, "DCI2".to_string());
                ntile.add_bel(bels::DCI1, "DCI1".to_string());
                ntile.add_bel(bels::BSCAN, "BSCAN".to_string());
                ntile.add_bel(bels::JTAGPPC, "JTAGPPC".to_string());
            }
            "UR.S3" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name]);
                ntile.add_bel(bels::DCI0, "DCI2".to_string());
                ntile.add_bel(bels::DCI1, "DCI1".to_string());
                ntile.add_bel(bels::DCIRESET0, "DCIRESET2".to_string());
                ntile.add_bel(bels::DCIRESET1, "DCIRESET1".to_string());
                ntile.add_bel(bels::BSCAN, "BSCAN".to_string());
            }
            "UR.FC" | "UR.S3E" | "UR.S3A" => {
                let (_, name) = namer.get_int_name(tcrd.cell);
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name]);
                ntile.add_bel(bels::BSCAN, "BSCAN".to_string());
            }

            _ => panic!("ummm {kind}?"),
        }
    }
    for (ccrd, conn) in edev.connectors() {
        let CellCoord { col, row, .. } = ccrd.cell;
        let kind = edev.db.conn_classes.key(conn.class);

        match &kind[..] {
            "TERM.W" => {
                let (naming, name) = namer.get_lterm_name(row);
                namer.ngrid.name_conn_tile(ccrd, naming, name);
            }
            "TERM.E" => {
                let (naming, name) = namer.get_rterm_name(row);
                namer.ngrid.name_conn_tile(ccrd, naming, name);
            }
            "TERM.S" => {
                let (naming, name) = namer.get_bterm_name(col);
                namer.ngrid.name_conn_tile(ccrd, naming, name);
            }
            "TERM.N" => {
                let (naming, name) = namer.get_tterm_name(col);
                namer.ngrid.name_conn_tile(ccrd, naming, name);
            }
            "TERM.BRAM.S" => {
                let x = namer.xlut[col];
                let y = row.to_idx() - 1;
                let name = format!("COB_TERM_T_X{x}Y{y}");
                namer.ngrid.name_conn_tile(ccrd, "TERM.BRAM.S", name);
            }
            "TERM.BRAM.N" => {
                let x = namer.xlut[col];
                let y = row.to_idx() + 1;
                let name = format!("COB_TERM_B_X{x}Y{y}");
                namer.ngrid.name_conn_tile(ccrd, "TERM.BRAM.N", name);
            }
            "PPC.W" => {
                let (name_l, name_r) = namer.get_ppc_h_name(ccrd.cell.delta(-9, 0));
                namer.ngrid.name_conn_pair(ccrd, "PPC.W", name_r, name_l);
            }
            "PPC.E" => {
                let (name_l, name_r) = namer.get_ppc_h_name(ccrd.cell);
                namer.ngrid.name_conn_pair(ccrd, "PPC.E", name_l, name_r);
            }
            "PPC.S" => {
                let (name_b, name_t) = namer.get_ppc_v_name(ccrd.cell.delta(0, -15));
                namer.ngrid.name_conn_pair(ccrd, "PPC.S", name_t, name_b);
            }
            "PPC.N" => {
                let (name_b, name_t) = namer.get_ppc_v_name(ccrd.cell);
                namer.ngrid.name_conn_pair(ccrd, "PPC.N", name_b, name_t);
            }
            "MAIN.S" => {
                if chip.kind.is_virtex2()
                    && chip.columns[col].kind == ColumnKind::Bram
                    && chip.bram_row(row) == Some(0)
                    && row.to_idx() != 1
                    && !edev.is_in_hole(ccrd.cell)
                {
                    let (_, name) = namer.get_bram_name(ccrd.cell);
                    namer.ngrid.name_conn_tile(ccrd, "BRAM.S", name);
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
