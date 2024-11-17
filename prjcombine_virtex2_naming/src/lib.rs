use std::{cmp::Ordering, collections::HashSet};

use prjcombine_int::{
    db::BelId,
    grid::{ColId, DieId, ExpandedDieRef, LayerId, RowId},
};
use prjcombine_virtex2::{
    expanded::ExpandedDevice,
    grid::{ColumnIoKind, ColumnKind, DcmPairKind, Grid, GridKind, IoCoord, RowIoKind},
    iob::{get_iob_data_b, get_iob_data_l, get_iob_data_r, get_iob_data_t, IobKind},
};
use prjcombine_xilinx_naming::{
    db::NamingDb,
    grid::{BelGrid, ExpandedGridNaming},
};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

pub struct ExpandedNamedDevice<'a> {
    pub edev: &'a ExpandedDevice<'a>,
    pub ngrid: ExpandedGridNaming<'a>,
    pub grid: &'a Grid,
}

impl<'a> ExpandedNamedDevice<'a> {
    pub fn get_io_name(&'a self, coord: IoCoord) -> &'a str {
        let die = DieId::from_idx(0);
        let nnode = &self.ngrid.nodes[&(die, coord.col, coord.row, LayerId::from_idx(1))];
        let bel = BelId::from_idx(coord.iob.to_idx());
        &nnode.bels[bel]
    }
}

struct Namer<'a> {
    edev: &'a ExpandedDevice<'a>,
    grid: &'a Grid,
    die: ExpandedDieRef<'a, 'a>,
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
        for (col, &cd) in &self.grid.columns {
            self.xlut.push(x);
            if cd.kind == ColumnKind::Dsp {
                x += 2;
            } else {
                x += 1;
            }
            if cd.kind == ColumnKind::Clb
                || (cd.kind != ColumnKind::Io && self.grid.kind == GridKind::Spartan3E)
            {
                self.sxlut.insert(col, sx);
                sx += 2;
            }
        }
    }

    fn fill_gtxlut(&mut self) {
        for (i, col) in self.grid.cols_gt.keys().copied().enumerate() {
            self.gtxlut.insert(col, i);
        }
    }

    fn fill_clut(&mut self) {
        let mut c = 0;
        let mut bramc = 1;
        for &cd in self.grid.columns.values() {
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
        let n = self.grid.rows.len();
        for row in self.die.rows() {
            self.rlut.push(n - row.to_idx() - 1);
        }
    }

    fn fill_hclklut(&mut self) {
        for (i, &(row_m, _, _)) in self.grid.rows_hclk.iter().enumerate() {
            self.hclklut.insert(row_m, i);
        }
    }

    fn fill_rows_brk(&mut self) {
        for &(_, _, r) in &self.grid.rows_hclk {
            self.rows_brk.insert(r - 1);
        }
        self.rows_brk.remove(&self.grid.row_top());
        if self.grid.kind != GridKind::Spartan3ADsp {
            self.rows_brk.remove(&(self.grid.row_mid() - 1));
        }
    }

    fn fill_vcc_lut(&mut self) {
        let mut xtmp = 0;
        if matches!(
            self.grid.kind,
            GridKind::Spartan3E | GridKind::Spartan3A | GridKind::Spartan3ADsp
        ) {
            xtmp += 1;
        }
        for col in self.grid.columns.ids() {
            self.vcc_xlut.push(xtmp);
            if col == self.grid.col_clk - 1 {
                xtmp += 2;
            } else {
                xtmp += 1;
            }
        }
        xtmp = 0;
        if matches!(
            self.grid.kind,
            GridKind::Spartan3E | GridKind::Spartan3A | GridKind::Spartan3ADsp
        ) {
            xtmp += 1;
        }
        for row in self.grid.rows.ids() {
            self.vcc_ylut.push(xtmp);
            if row == self.grid.row_mid() - 1
                && matches!(
                    self.grid.kind,
                    GridKind::Spartan3E | GridKind::Spartan3A | GridKind::Spartan3ADsp
                )
            {
                xtmp += 2;
            } else {
                xtmp += 1;
            }
        }
    }

    fn get_int_name(&self, col: ColId, row: RowId) -> (&'static str, String) {
        let x = self.xlut[col];
        let y = row.to_idx();

        for &(bc, br) in &self.grid.holes_ppc {
            if col >= bc && col < bc + 10 && row >= br && row < br + 16 {
                let naming = if col == bc + 9 {
                    "INT.PPC.R"
                } else if row == br {
                    "INT.PPC.B"
                } else if row == br + 15 {
                    "INT.PPC.T"
                } else if col == bc {
                    "INT.PPC.L"
                } else {
                    unreachable!();
                };
                let prefix = if col == bc && row == br + 1 {
                    "PTERMLL"
                } else if col == bc && row == br + 14 {
                    "PTERMUL"
                } else {
                    ""
                };
                let r = self.rlut[row];
                let name = if self.grid.columns[col].kind == ColumnKind::Clb {
                    let c = self.clut[col];
                    format!("{prefix}R{r}C{c}")
                } else {
                    let c = self.bramclut[col];
                    format!("PPCINTR{r}BRAMC{c}")
                };
                return (naming, name);
            }
        }
        for pair in self.grid.get_dcm_pairs() {
            match pair.kind {
                DcmPairKind::Bot => {
                    if col == pair.col - 1 && row == pair.row {
                        return ("INT.DCM.S3E", format!("DCM_BL_CENTER_X{x}Y{y}"));
                    }
                    if col == pair.col && row == pair.row {
                        return ("INT.DCM.S3E", format!("DCM_BR_CENTER_X{x}Y{y}"));
                    }
                }
                DcmPairKind::BotSingle => {
                    if col == pair.col - 1 && row == pair.row {
                        return ("INT.DCM.S3E.DUMMY", format!("DCMAUX_BL_CENTER_X{x}Y{y}"));
                    }
                    if col == pair.col && row == pair.row {
                        return ("INT.DCM.S3E", format!("DCM_BR_CENTER_X{x}Y{y}"));
                    }
                }
                DcmPairKind::Top => {
                    if col == pair.col - 1 && row == pair.row {
                        return ("INT.DCM.S3E", format!("DCM_TL_CENTER_X{x}Y{y}"));
                    }
                    if col == pair.col && row == pair.row {
                        return ("INT.DCM.S3E", format!("DCM_TR_CENTER_X{x}Y{y}"));
                    }
                }
                DcmPairKind::TopSingle => {
                    if col == pair.col - 1 && row == pair.row {
                        return ("INT.DCM.S3E.DUMMY", format!("DCMAUX_TL_CENTER_X{x}Y{y}"));
                    }
                    if col == pair.col && row == pair.row {
                        return ("INT.DCM.S3E", format!("DCM_TR_CENTER_X{x}Y{y}"));
                    }
                }
                DcmPairKind::Left => {
                    if col == pair.col && row == pair.row - 1 {
                        return ("INT.DCM.S3E.H", format!("DCM_H_BL_CENTER_X{x}Y{y}"));
                    }
                    if col == pair.col && row == pair.row {
                        return ("INT.DCM.S3E.H", format!("DCM_H_TL_CENTER_X{x}Y{y}"));
                    }
                }
                DcmPairKind::Right => {
                    if col == pair.col && row == pair.row - 1 {
                        return ("INT.DCM.S3E.H", format!("DCM_H_BR_CENTER_X{x}Y{y}"));
                    }
                    if col == pair.col && row == pair.row {
                        return ("INT.DCM.S3E.H", format!("DCM_H_TR_CENTER_X{x}Y{y}"));
                    }
                }
                DcmPairKind::Bram => {
                    if col == pair.col && row == pair.row - 1 {
                        return ("INT.DCM.S3E.H", format!("DCM_BGAP_X{x}Y{y}"));
                    }
                    if col == pair.col && row == pair.row {
                        return ("INT.DCM.S3E.H", format!("DCM_SPLY_X{x}Y{y}"));
                    }
                }
            }
        }

        if (col == self.grid.col_left() || col == self.grid.col_right())
            && (row == self.grid.row_bot() || row == self.grid.row_top())
        {
            if self.grid.kind.is_spartan3ea() {
                let ul = if row == self.grid.row_bot() { 'L' } else { 'U' };
                let lr = if col == self.grid.col_left() {
                    'L'
                } else {
                    'R'
                };
                ("INT.CNR", format!("{ul}{lr}_X{x}Y{y}"))
            } else {
                let bt = if row == self.grid.row_bot() { 'B' } else { 'T' };
                let lr = if col == self.grid.col_left() {
                    'L'
                } else {
                    'R'
                };
                if self.grid.kind.is_virtex2p() {
                    ("INT.CNR", format!("{lr}IOI{bt}IOI"))
                } else {
                    ("INT.CNR", format!("{bt}{lr}"))
                }
            }
        } else if (row == self.grid.row_bot() || row == self.grid.row_top())
            && !self.grid.kind.is_spartan3ea()
            && matches!(self.grid.columns[col].kind, ColumnKind::Bram)
        {
            let bt = if row == self.grid.row_bot() { 'B' } else { 'T' };
            let c = self.bramclut[col];
            let naming = match self.grid.kind {
                GridKind::Virtex2 => "INT.BRAM_IOIS",
                GridKind::Virtex2P | GridKind::Virtex2PX => {
                    if self.grid.cols_gt.contains_key(&col) {
                        "INT.GT.CLKPAD"
                    } else {
                        "INT.ML_BRAM_IOIS"
                    }
                }
                GridKind::Spartan3 => {
                    if col == self.grid.col_left() + 3 || col == self.grid.col_right() - 3 {
                        "INT.DCM.S3"
                    } else {
                        "INT.DCM.S3.DUMMY"
                    }
                }
                _ => unreachable!(),
            };
            (naming, format!("{bt}IOIBRAMC{c}"))
        } else if self.grid.bram_row(row).is_some()
            && matches!(self.grid.columns[col].kind, ColumnKind::Bram)
        {
            // BRAM
            if !self.grid.kind.is_spartan3ea() {
                let c = self.bramclut[col];
                let r = self.rlut[row];

                let is_gt = self.grid.cols_gt.contains_key(&col)
                    && self.grid.kind == GridKind::Virtex2P
                    && (row < self.grid.row_bot() + 5 || row >= self.grid.row_top() - 4);
                let is_gt10 = self.grid.cols_gt.contains_key(&col)
                    && self.grid.kind == GridKind::Virtex2PX
                    && (row < self.grid.row_bot() + 9 || row >= self.grid.row_top() - 8);
                (
                    if is_gt || is_gt10 {
                        "INT.GT"
                    } else {
                        "INT.BRAM"
                    },
                    format!("BRAMR{r}C{c}"),
                )
            } else {
                let idx = self.grid.bram_row(row).unwrap();
                let naming = if self.grid.kind == GridKind::Spartan3ADsp {
                    if self.rows_brk.contains(&row) {
                        "INT.BRAM.S3ADSP.BRK"
                    } else {
                        "INT.BRAM.S3ADSP"
                    }
                } else {
                    if self.rows_brk.contains(&row) {
                        "INT.BRAM.BRK"
                    } else {
                        "INT.BRAM"
                    }
                };
                let mut md = "";
                if self.rows_brk.contains(&row) {
                    md = "_BRK";
                }
                if self.grid.kind != GridKind::Spartan3E {
                    if row == self.grid.row_bot() + 1 {
                        md = "_BOT";
                    }
                    if row == self.grid.row_top() - 1 {
                        md = "_TOP";
                    }
                    if self.grid.cols_clkv.is_none() && row == self.grid.row_top() - 5 {
                        md = "_TOP";
                    }
                }
                (naming, format!("BRAM{idx}_SMALL{md}_X{x}Y{y}"))
            }
        } else if self.grid.bram_row(row).is_some()
            && matches!(self.grid.columns[col].kind, ColumnKind::Dsp)
        {
            // DSP
            let idx = self.grid.bram_row(row).unwrap();
            let naming = if self.rows_brk.contains(&row) {
                "INT.MACC.BRK"
            } else {
                "INT.MACC"
            };
            let mut md = "";
            if self.rows_brk.contains(&row) {
                md = "_BRK";
            }
            if self.grid.kind != GridKind::Spartan3E {
                if row == self.grid.row_bot() + 1 {
                    md = "_BOT";
                }
                if row == self.grid.row_top() - 1 {
                    md = "_TOP";
                }
                if self.grid.cols_clkv.is_none() && row == self.grid.row_top() - 5 {
                    md = "_TOP";
                }
            }
            (naming, format!("MACC{idx}_SMALL{md}_X{x}Y{y}"))
        } else if row == self.grid.row_bot() || row == self.grid.row_top() {
            match self.grid.kind {
                GridKind::Virtex2
                | GridKind::Virtex2P
                | GridKind::Virtex2PX
                | GridKind::Spartan3
                | GridKind::FpgaCore => {
                    let bt = if row == self.grid.row_bot() { 'B' } else { 'T' };
                    let c = self.clut[col];
                    let naming = if self.grid.kind.is_virtex2() {
                        if self.grid.kind == GridKind::Virtex2PX && col == self.grid.col_clk - 1 {
                            if row == self.grid.row_bot() {
                                "INT.IOI.CLK_B"
                            } else {
                                "INT.IOI.CLK_T"
                            }
                        } else {
                            "INT.IOI.TB"
                        }
                    } else if self.grid.kind == GridKind::FpgaCore {
                        "INT.IOI.FC"
                    } else {
                        "INT.IOI"
                    };
                    (naming, format!("{bt}IOIC{c}"))
                }
                GridKind::Spartan3E | GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                    let naming = if self.grid.kind.is_spartan3a() {
                        "INT.IOI.S3A.TB"
                    } else {
                        "INT.IOI"
                    };
                    let (data, tidx) = if row == self.grid.row_bot() {
                        get_iob_data_b(self.grid.kind, self.grid.columns[col].io)
                    } else {
                        get_iob_data_t(self.grid.kind, self.grid.columns[col].io)
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
                        if row == self.grid.row_bot() {
                            "BIOIS"
                        } else {
                            "TIOIS"
                        }
                    } else if !has_iobs {
                        if row == self.grid.row_bot() {
                            "BIBUFS"
                        } else {
                            "TIBUFS"
                        }
                    } else {
                        if row == self.grid.row_bot() {
                            "BIOIB"
                        } else {
                            "TIOIB"
                        }
                    };
                    let name = format!("{kind}_X{x}Y{y}");
                    (naming, name)
                }
            }
        } else if col == self.grid.col_left() || col == self.grid.col_right() {
            match self.grid.kind {
                GridKind::Virtex2
                | GridKind::Virtex2P
                | GridKind::Virtex2PX
                | GridKind::Spartan3
                | GridKind::FpgaCore => {
                    let lr = if col == self.grid.col_left() {
                        'L'
                    } else {
                        'R'
                    };
                    let r = self.rlut[row];
                    let naming = if self.grid.kind.is_virtex2() {
                        "INT.IOI.LR"
                    } else if self.grid.kind == GridKind::FpgaCore {
                        "INT.IOI.FC"
                    } else {
                        "INT.IOI"
                    };
                    (naming, format!("{lr}IOIR{r}"))
                }
                GridKind::Spartan3E | GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                    let naming = if self.grid.kind.is_spartan3a() {
                        if self.rows_brk.contains(&row) {
                            "INT.IOI.S3A.LR.BRK"
                        } else {
                            "INT.IOI.S3A.LR"
                        }
                    } else {
                        if self.rows_brk.contains(&row) {
                            "INT.IOI.BRK"
                        } else {
                            "INT.IOI"
                        }
                    };
                    let (data, tidx) = if col == self.grid.col_left() {
                        get_iob_data_l(self.grid.kind, self.grid.rows[row])
                    } else {
                        get_iob_data_r(self.grid.kind, self.grid.rows[row])
                    };
                    let has_ibufs = data
                        .iobs
                        .iter()
                        .any(|iob| iob.tile == tidx && iob.kind == IobKind::Ibuf);
                    let kind = if !has_ibufs {
                        if col == self.grid.col_left() {
                            "LIOIS"
                        } else {
                            "RIOIS"
                        }
                    } else {
                        if col == self.grid.col_left() {
                            "LIBUFS"
                        } else {
                            "RIBUFS"
                        }
                    };
                    let brk = if self.rows_brk.contains(&row) {
                        "_BRK"
                    } else {
                        ""
                    };
                    let clk = if row == self.grid.row_mid() - 1 || row == self.grid.row_mid() {
                        "_CLK"
                    } else {
                        ""
                    };
                    let pci = if row >= self.grid.row_mid() - 4 && row < self.grid.row_mid() + 4 {
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
                assert!(!hole.contains(col, row));
            }
            if !self.grid.kind.is_spartan3ea() {
                let r = self.rlut[row];
                let c = self.clut[col];
                ("INT.CLB", format!("R{r}C{c}"))
            } else {
                let naming = if self.rows_brk.contains(&row) {
                    "INT.CLB.BRK"
                } else {
                    "INT.CLB"
                };
                (naming, format!("CLB_X{x}Y{y}"))
            }
        }
    }

    fn get_lterm_name(&self, row: RowId) -> (&'static str, String) {
        let x = self.xlut[self.grid.col_left()];
        let y = row.to_idx();
        if row == self.grid.row_bot() {
            if !self.grid.kind.is_spartan3ea() {
                if !self.grid.kind.is_virtex2p() {
                    ("TERM.W", "LBTERM".into())
                } else {
                    ("TERM.W", "LTERMBIOI".into())
                }
            } else {
                ("TERM.W", format!("CNR_LBTERM_X{x}Y{y}"))
            }
        } else if row == self.grid.row_top() {
            if !self.grid.kind.is_spartan3ea() {
                if !self.grid.kind.is_virtex2p() {
                    ("TERM.W", "LTTERM".into())
                } else {
                    ("TERM.W", "LTERMTIOI".into())
                }
            } else {
                ("TERM.W", format!("CNR_LTTERM_X{x}Y{y}"))
            }
        } else {
            if !self.grid.kind.is_spartan3ea() {
                let r = self.rlut[row];
                let naming = if self.grid.kind.is_virtex2() {
                    if row < self.grid.row_pci.unwrap() {
                        "TERM.W.D"
                    } else {
                        "TERM.W.U"
                    }
                } else {
                    "TERM.W"
                };
                (naming, format!("LTERMR{r}"))
            } else {
                let mut kind = match self.grid.rows[row] {
                    RowIoKind::Single => "LTERM1",
                    RowIoKind::Double(0) => "LTERM2",
                    RowIoKind::Triple(0) => "LTERM3",
                    RowIoKind::Quad(0) => "LTERM4",
                    _ => "LTERM",
                };
                if self.grid.kind == GridKind::Spartan3E {
                    if row == self.grid.row_mid() {
                        kind = "LTERM4CLK";
                    }
                    if row == self.grid.row_mid() - 4 {
                        kind = "LTERM4B";
                    }
                    if row == self.grid.row_mid() - 3 {
                        kind = "LTERMCLKA";
                    }
                    if row == self.grid.row_mid() - 1 {
                        kind = "LTERMCLK";
                    }
                    if row == self.grid.row_mid() + 1 {
                        kind = "LTERMCLKA";
                    }
                    if row == self.grid.row_mid() + 3 {
                        kind = "LTERMCLK";
                    }
                } else {
                    if row == self.grid.row_mid() {
                        kind = "LTERM4CLK";
                    }
                    if row == self.grid.row_mid() - 4 {
                        kind = "LTERM4B";
                    }
                    if row == self.grid.row_mid() - 2 {
                        kind = "LTERMCLKA";
                    }
                    if row == self.grid.row_mid() - 1 {
                        kind = "LTERMCLK";
                    }
                    if row == self.grid.row_mid() + 1 {
                        kind = "LTERMCLKA";
                    }
                    if row == self.grid.row_mid() + 2 {
                        kind = "LTERMCLK";
                    }
                }
                ("TERM.W", format!("{kind}_X{x}Y{y}"))
            }
        }
    }

    fn get_rterm_name(&self, row: RowId) -> (&'static str, String) {
        let x = self.xlut[self.grid.col_right()];
        let y = row.to_idx();
        if row == self.grid.row_bot() {
            if !self.grid.kind.is_spartan3ea() {
                if !self.grid.kind.is_virtex2p() {
                    ("TERM.E", "RBTERM".into())
                } else {
                    ("TERM.E", "RTERMBIOI".into())
                }
            } else {
                ("TERM.E", format!("CNR_RBTERM_X{x}Y{y}"))
            }
        } else if row == self.grid.row_top() {
            if !self.grid.kind.is_spartan3ea() {
                if !self.grid.kind.is_virtex2p() {
                    ("TERM.E", "RTTERM".into())
                } else {
                    ("TERM.E", "RTERMTIOI".into())
                }
            } else {
                ("TERM.E", format!("CNR_RTTERM_X{x}Y{y}"))
            }
        } else {
            if !self.grid.kind.is_spartan3ea() {
                let r = self.rlut[row];
                let naming = if self.grid.kind.is_virtex2() {
                    if row < self.grid.row_pci.unwrap() {
                        "TERM.E.D"
                    } else {
                        "TERM.E.U"
                    }
                } else {
                    "TERM.E"
                };
                (naming, format!("RTERMR{r}"))
            } else {
                let mut kind = match self.grid.rows[row] {
                    RowIoKind::Single => "RTERM1",
                    RowIoKind::Double(0) => "RTERM2",
                    RowIoKind::Triple(0) => "RTERM3",
                    RowIoKind::Quad(0) => "RTERM4",
                    _ => "RTERM",
                };
                if self.grid.kind == GridKind::Spartan3E {
                    if row == self.grid.row_mid() {
                        kind = "RTERM4CLK";
                    }
                    if row == self.grid.row_mid() - 4 {
                        kind = "RTERM4CLKB";
                    }
                    if row == self.grid.row_mid() - 2 {
                        kind = "RTERMCLKA";
                    }
                    if row == self.grid.row_mid() + 2 {
                        kind = "RTERMCLKA";
                    }
                } else {
                    if row == self.grid.row_mid() {
                        kind = "RTERM4CLK";
                    }
                    if row == self.grid.row_mid() - 4 {
                        kind = "RTERM4B";
                    }
                    if row == self.grid.row_mid() - 3 {
                        kind = "RTERMCLKB";
                    }
                    if row == self.grid.row_mid() - 2 {
                        kind = "RTERMCLKA";
                    }
                    if row == self.grid.row_mid() + 1 {
                        kind = "RTERMCLKA";
                    }
                }
                ("TERM.E", format!("{kind}_X{x}Y{y}"))
            }
        }
    }

    fn get_bterm_name(&self, col: ColId) -> (&'static str, String) {
        let x = self.xlut[col];
        let y = self.grid.row_bot().to_idx();
        if col == self.grid.col_left() {
            if !self.grid.kind.is_spartan3ea() {
                if !self.grid.kind.is_virtex2p() {
                    ("TERM.S.CNR", "BLTERM".into())
                } else {
                    ("TERM.S.CNR", "LIOIBTERM".into())
                }
            } else {
                ("TERM.S.CNR", format!("CNR_BTERM_X{x}Y{y}"))
            }
        } else if col == self.grid.col_right() {
            if !self.grid.kind.is_spartan3ea() {
                if !self.grid.kind.is_virtex2p() {
                    ("TERM.S.CNR", "BRTERM".into())
                } else {
                    ("TERM.S.CNR", "RIOIBTERM".into())
                }
            } else {
                ("TERM.S.CNR", format!("CNR_BTERM_X{x}Y{y}"))
            }
        } else if !self.grid.kind.is_spartan3ea() && self.grid.columns[col].kind == ColumnKind::Bram
        {
            let c = self.bramclut[col];
            ("TERM.S", format!("BTERMBRAMC{c}"))
        } else {
            if !self.grid.kind.is_spartan3ea() {
                let c = self.clut[col];
                ("TERM.S", format!("BTERMC{c}"))
            } else {
                let cd = &self.grid.columns[col];
                let mut kind = match cd.io {
                    ColumnIoKind::Single => "BTERM1",
                    ColumnIoKind::Double(0) => "BTERM2",
                    ColumnIoKind::Triple(0) => "BTERM3",
                    ColumnIoKind::Quad(0) => "BTERM4",
                    _ => "BTERM",
                };
                if self.grid.kind == GridKind::Spartan3E {
                    if cd.io == ColumnIoKind::Quad(0) && cd.kind == ColumnKind::BramCont(2) {
                        kind = "BTERM4_BRAM2";
                    }
                    if col == self.grid.col_clk - 3 {
                        kind = "BTERMCLKA";
                    }
                    if col == self.grid.col_clk - 1 {
                        kind = "BTERMCLKB";
                    }
                    if col == self.grid.col_clk {
                        kind = "BTERM4CLK";
                    }
                    if col == self.grid.col_clk + 1 {
                        kind = "BTERMCLK";
                    }
                } else {
                    if col == self.grid.col_clk - 2 {
                        kind = "BTERM2CLK";
                    }
                    if col == self.grid.col_clk - 1 {
                        kind = "BTERMCLKB";
                    }
                    if col == self.grid.col_clk {
                        kind = "BTERM2CLK";
                    }
                    if col == self.grid.col_clk + 1 {
                        kind = "BTERMCLK";
                    }
                    if self.grid.kind == GridKind::Spartan3ADsp {
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
        let y = self.grid.row_top().to_idx();
        if col == self.grid.col_left() {
            if !self.grid.kind.is_spartan3ea() {
                if !self.grid.kind.is_virtex2p() {
                    ("TERM.N.CNR", "TLTERM".into())
                } else {
                    ("TERM.N.CNR", "LIOITTERM".into())
                }
            } else {
                ("TERM.N.CNR", format!("CNR_TTERM_X{x}Y{y}"))
            }
        } else if col == self.grid.col_right() {
            if !self.grid.kind.is_spartan3ea() {
                if !self.grid.kind.is_virtex2p() {
                    ("TERM.N.CNR", "TRTERM".into())
                } else {
                    ("TERM.N.CNR", "RIOITTERM".into())
                }
            } else {
                ("TERM.N.CNR", format!("CNR_TTERM_X{x}Y{y}"))
            }
        } else if !self.grid.kind.is_spartan3ea() && self.grid.columns[col].kind == ColumnKind::Bram
        {
            let c = self.bramclut[col];
            ("TERM.N", format!("TTERMBRAMC{c}"))
        } else {
            if !self.grid.kind.is_spartan3ea() {
                let c = self.clut[col];
                ("TERM.N", format!("TTERMC{c}"))
            } else {
                let cd = &self.grid.columns[col];
                let mut kind = match cd.io {
                    ColumnIoKind::Single => "TTERM1",
                    ColumnIoKind::Double(0) => "TTERM2",
                    ColumnIoKind::Triple(0) => "TTERM3",
                    ColumnIoKind::Quad(0) => "TTERM4",
                    _ => "TTERM",
                };
                if self.grid.kind == GridKind::Spartan3E {
                    if cd.io == ColumnIoKind::Quad(0) && cd.kind == ColumnKind::BramCont(2) {
                        kind = "TTERM4_BRAM2";
                    }
                    if col == self.grid.col_clk - 2 {
                        kind = "TTERMCLK";
                    }
                    if col == self.grid.col_clk - 1 {
                        kind = "TTERMCLKA";
                    }
                    if col == self.grid.col_clk {
                        kind = "TTERM4CLK";
                    }
                    if col == self.grid.col_clk + 2 {
                        kind = "TTERMCLKA";
                    }
                } else {
                    if col == self.grid.col_clk - 2 {
                        kind = "TTERM2CLK";
                    }
                    if col == self.grid.col_clk - 1 {
                        kind = "TTERMCLKA";
                    }
                    if col == self.grid.col_clk {
                        kind = "TTERM2CLK";
                    }
                    if col == self.grid.col_clk + 1 {
                        kind = "TTERMCLKA";
                    }
                    if self.grid.kind == GridKind::Spartan3ADsp {
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

    fn get_bram_name(&self, col: ColId, row: RowId) -> (&'static str, String) {
        let is_bot = matches!(self.grid.kind, GridKind::Spartan3A | GridKind::Spartan3ADsp)
            && row == self.grid.row_bot() + 1;
        let is_top = matches!(self.grid.kind, GridKind::Spartan3A | GridKind::Spartan3ADsp)
            && (row == self.grid.row_top() - 4
                || row == self.grid.row_top() - 8 && col == self.grid.col_clk);
        let is_brk = self.rows_brk.contains(&(row + 3));
        let naming = match self.grid.kind {
            GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => "BRAM",
            GridKind::Spartan3 => "BRAM.S3",
            GridKind::FpgaCore => unreachable!(),
            GridKind::Spartan3E => "BRAM.S3E",
            GridKind::Spartan3A => {
                if is_bot {
                    "BRAM.S3A.BOT"
                } else if is_top {
                    "BRAM.S3A.TOP"
                } else {
                    "BRAM.S3A"
                }
            }
            GridKind::Spartan3ADsp => "BRAM.S3ADSP",
        };
        let name = if self.grid.kind.is_spartan3ea() {
            let x = self.xlut[col] + 1;
            let y = row.to_idx();
            let m = if self.grid.kind == GridKind::Spartan3ADsp {
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
            let c = self.bramclut[col];
            let r = self.rlut[row];
            format!("BMR{r}C{c}")
        };
        (naming, name)
    }

    fn get_dsp_name(&self, col: ColId, row: RowId) -> (&'static str, String) {
        let is_bot = row == self.grid.row_bot() + 1;
        let is_top = row == self.grid.row_top() - 4;
        let is_brk = self.rows_brk.contains(&(row + 3));
        let naming = if is_top { "DSP.TOP" } else { "DSP" };
        let x = self.xlut[col] + 1;
        let y = row.to_idx();
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

    fn get_hclk_name(&self, col: ColId, row: RowId) -> (&'static str, String) {
        if !self.grid.kind.is_spartan3ea() {
            let mut r = self.grid.rows_hclk.len() - self.hclklut[row];
            if self.grid.columns[col].kind == ColumnKind::Bram {
                let c = self.bramclut[col];
                ("GCLKH", format!("GCLKHR{r}BRAMC{c}"))
            } else {
                // *sigh*.
                if self.grid.kind == GridKind::Virtex2 && self.grid.columns.len() == 12 {
                    r -= 1;
                }
                let c = self.clut[col];
                if self.grid.columns[col].kind == ColumnKind::Io && self.grid.kind.is_virtex2p() {
                    if col == self.grid.col_left() {
                        ("GCLKH", format!("LIOICLKR{r}"))
                    } else {
                        ("GCLKH", format!("RIOICLKR{r}"))
                    }
                } else {
                    ("GCLKH", format!("GCLKHR{r}C{c}"))
                }
            }
        } else {
            let x = self.xlut[col];
            let y = row.to_idx() - 1;
            let mut naming = "GCLKH";
            let kind = match self.grid.columns[col].kind {
                ColumnKind::Io => match row.cmp(&self.grid.row_mid()) {
                    Ordering::Less => "GCLKH_PCI_CE_S",
                    Ordering::Equal => "GCLKH_PCI_CE_S_50A",
                    Ordering::Greater => "GCLKH_PCI_CE_N",
                },
                ColumnKind::BramCont(x) => {
                    if row == self.grid.row_mid() {
                        naming = "GCLKH.BRAM";
                        [
                            "BRAMSITE2_DN_GCLKH",
                            "BRAM2_GCLKH_FEEDTHRU",
                            "BRAM2_GCLKH_FEEDTHRUA",
                        ][x as usize - 1]
                    } else if self.hclklut[row] == 0 {
                        naming = "GCLKH.BRAM.S";
                        if self.grid.kind == GridKind::Spartan3E {
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
                    } else if self.hclklut[row] == self.grid.rows_hclk.len() - 1 {
                        naming = "GCLKH.BRAM.N";
                        if self.grid.kind == GridKind::Spartan3E {
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
                        if self.grid.kind == GridKind::Spartan3E {
                            [
                                "BRAMSITE2_MID_GCLKH",
                                "BRAM2_MID_GCLKH_FEEDTHRU",
                                "BRAM2_MID_GCLKH_FEEDTHRUA",
                            ][x as usize - 1]
                        } else {
                            [
                                if self.grid.kind != GridKind::Spartan3ADsp {
                                    "BRAMSITE2_GCLKH"
                                } else if row < self.grid.row_mid() {
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

    fn get_ppc_h_name(&self, col: ColId, row: RowId) -> (String, String) {
        let (_, name_l) = self.get_int_name(col, row);
        let r = self.rlut[row];
        let c = self.bramclut[col + 8];
        let name_r = format!("BMR{r}C{c}");
        (name_l, name_r)
    }

    fn get_ppc_v_name(&self, col: ColId, row: RowId) -> (String, String) {
        let rb = self.rlut[row + 1];
        let rt = self.rlut[row + 14];
        if self.grid.columns[col].kind == ColumnKind::Clb {
            let c = self.clut[col];
            (format!("PTERMR{rb}C{c}"), format!("PTERMR{rt}C{c}"))
        } else {
            let c = self.bramclut[col];
            (
                format!("PTERMBR{rb}BRAMC{c}"),
                format!("PTERMTR{rt}BRAMC{c}"),
            )
        }
    }

    fn get_llv_name(&self, col: ColId) -> (&'static str, String) {
        let naming = if col == self.grid.col_left() {
            "LLV.CLKL"
        } else if col == self.grid.col_right() {
            "LLV.CLKR"
        } else {
            "LLV"
        };
        let x = self.xlut[col];
        let y = self.grid.row_mid().to_idx() - 1;
        let mut name = if col == self.grid.col_left() {
            format!("CLKL_IOIS_LL_X{x}Y{y}")
        } else if col == self.grid.col_right() {
            format!("CLKR_IOIS_LL_X{x}Y{y}")
        } else {
            format!("CLKH_LL_X{x}Y{y}")
        };
        if self.grid.kind == GridKind::Spartan3E {
            if col == self.grid.col_left() + 9 {
                name = format!("CLKLH_DCM_LL_X{x}Y{y}");
            }
            if col == self.grid.col_right() - 9 {
                name = format!("CLKRH_DCM_LL_X{x}Y{y}");
            }
        } else {
            if col == self.grid.col_left() + 3 {
                name = format!("CLKLH_DCM_LL_X{x}Y{y}");
            }
            if col == self.grid.col_right() - 6 {
                name = format!("CLKRH_DCM_LL_X{x}Y{y}");
            }
            if [
                self.grid.col_left() + 1,
                self.grid.col_left() + 2,
                self.grid.col_right() - 2,
                self.grid.col_right() - 1,
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
        let x = self.xlut[self.grid.col_clk - 1];
        let y = row.to_idx();
        if row == self.grid.row_bot() {
            format!("CLKB_LL_X{x}Y{y}")
        } else if row == self.grid.row_top() {
            format!("CLKT_LL_X{x}Y{y}")
        } else if self.grid.kind != GridKind::Spartan3E
            && [
                self.grid.row_bot() + 2,
                self.grid.row_bot() + 3,
                self.grid.row_bot() + 4,
                self.grid.row_top() - 4,
                self.grid.row_top() - 3,
                self.grid.row_top() - 2,
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
            let mut clks = vec![];
            let mut pads = vec![];
            let mut ipads = vec![];
            if cd.io != ColumnIoKind::None {
                let (data, tidx) = get_iob_data_t(self.grid.kind, cd.io);
                for &iob in &data.iobs {
                    if iob.tile == tidx {
                        if iob.kind == IobKind::Clk {
                            clks.push(iob.bel.to_idx());
                        } else if iob.kind == IobKind::Ibuf && self.grid.kind != GridKind::FpgaCore
                        {
                            ipads.push(iob.bel.to_idx());
                        } else {
                            pads.push(iob.bel.to_idx());
                        }
                    }
                }
            }
            let iobs: &[usize] = match self.grid.kind {
                GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => &[3, 2, 1, 0],
                GridKind::Spartan3 => &[2, 1, 0],
                GridKind::FpgaCore => &[3, 7, 2, 6, 1, 5, 0, 4],
                GridKind::Spartan3E => &[2, 1, 0],
                GridKind::Spartan3A | GridKind::Spartan3ADsp => &[0, 1, 2],
            };
            let nnode = self
                .ngrid
                .nodes
                .get_mut(&(self.die.die, col, row, LayerId::from_idx(1)))
                .unwrap();
            for &i in iobs {
                if clks.contains(&i) {
                    let name = match i {
                        0 => "CLKPPAD1",
                        1 => "CLKNPAD1",
                        _ => unreachable!(),
                    };
                    nnode.add_bel(i, name.into());
                    self.ctr_pad += 1;
                } else if pads.contains(&i) {
                    nnode.add_bel(i, format!("PAD{idx}", idx = self.ctr_pad));
                    self.ctr_pad += 1;
                } else if ipads.contains(&i) {
                    nnode.add_bel(i, format!("IPAD{idx}", idx = self.ctr_pad));
                    self.ctr_pad += 1;
                } else {
                    nnode.add_bel(i, format!("NOPAD{idx}", idx = self.ctr_nopad));
                    self.ctr_nopad += 1;
                }
            }
        }
    }

    fn fill_io_r(&mut self) {
        let col = self.grid.col_right();
        for row in self.grid.rows.ids().rev() {
            if row == self.grid.row_bot() || row == self.grid.row_top() {
                continue;
            }
            let (data, tidx) = get_iob_data_r(self.grid.kind, self.grid.rows[row]);
            let mut pads = vec![];
            let mut ipads = vec![];
            for &iob in &data.iobs {
                if iob.tile == tidx {
                    if iob.kind == IobKind::Ibuf && self.grid.kind != GridKind::FpgaCore {
                        ipads.push(iob.bel.to_idx());
                    } else {
                        pads.push(iob.bel.to_idx());
                    }
                }
            }
            let iobs: &[usize] = match self.grid.kind {
                GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => &[3, 2, 1, 0],
                GridKind::Spartan3 => &[2, 1, 0],
                GridKind::FpgaCore => &[3, 7, 2, 6, 1, 5, 0, 4],
                GridKind::Spartan3E => &[2, 1, 0],
                GridKind::Spartan3A | GridKind::Spartan3ADsp => &[1, 0],
            };
            let nnode = self
                .ngrid
                .nodes
                .get_mut(&(self.die.die, col, row, LayerId::from_idx(1)))
                .unwrap();
            for &i in iobs {
                if pads.contains(&i) {
                    nnode.add_bel(i, format!("PAD{idx}", idx = self.ctr_pad));
                    self.ctr_pad += 1;
                } else if ipads.contains(&i) {
                    nnode.add_bel(i, format!("IPAD{idx}", idx = self.ctr_pad));
                    self.ctr_pad += 1;
                } else {
                    nnode.add_bel(i, format!("NOPAD{idx}", idx = self.ctr_nopad));
                    self.ctr_nopad += 1;
                }
            }
        }
    }

    fn fill_io_b(&mut self) {
        let row = self.grid.row_bot();
        for (col, &cd) in self.grid.columns.iter().rev() {
            if self.grid.kind.is_spartan3ea() {
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
                let (data, tidx) = get_iob_data_b(self.grid.kind, cd.io);
                for &iob in &data.iobs {
                    if iob.tile == tidx {
                        if iob.kind == IobKind::Clk {
                            clks.push(iob.bel.to_idx());
                        } else if iob.kind == IobKind::Ibuf && self.grid.kind != GridKind::FpgaCore
                        {
                            ipads.push(iob.bel.to_idx());
                        } else {
                            pads.push(iob.bel.to_idx());
                        }
                    }
                }
            }
            let iobs: &[usize] = match self.grid.kind {
                GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => &[3, 2, 1, 0],
                GridKind::Spartan3 => &[2, 1, 0],
                GridKind::FpgaCore => &[3, 7, 2, 6, 1, 5, 0, 4],
                GridKind::Spartan3E => &[2, 1, 0],
                GridKind::Spartan3A | GridKind::Spartan3ADsp => &[2, 1, 0],
            };
            let nnode = self
                .ngrid
                .nodes
                .get_mut(&(self.die.die, col, row, LayerId::from_idx(1)))
                .unwrap();
            for &i in iobs {
                if clks.contains(&i) {
                    let name = match i {
                        2 => "CLKPPAD2",
                        3 => "CLKNPAD2",
                        _ => unreachable!(),
                    };
                    nnode.add_bel(i, name.into());
                    self.ctr_pad += 1;
                } else if pads.contains(&i) {
                    let mut name = format!("PAD{idx}", idx = self.ctr_pad);
                    if self.grid.kind == GridKind::Spartan3A && self.grid.cols_clkv.is_none() {
                        // 3s50a special
                        match self.ctr_pad {
                            94 => name = "PAD96".to_string(),
                            96 => name = "PAD97".to_string(),
                            97 => name = "PAD95".to_string(),
                            _ => (),
                        }
                    }
                    nnode.add_bel(i, name);
                    self.ctr_pad += 1;
                } else if ipads.contains(&i) {
                    let mut name = format!("IPAD{idx}", idx = self.ctr_pad);
                    if self.grid.kind == GridKind::Spartan3A
                        && self.grid.cols_clkv.is_none()
                        && self.ctr_pad == 95
                    {
                        name = "IPAD94".to_string();
                    }
                    nnode.add_bel(i, name);
                    self.ctr_pad += 1;
                } else {
                    nnode.add_bel(i, format!("NOPAD{idx}", idx = self.ctr_nopad));
                    self.ctr_nopad += 1;
                }
            }
        }
    }

    fn fill_io_l(&mut self) {
        let col = self.grid.col_left();
        for row in self.grid.rows.ids() {
            if row == self.grid.row_bot() || row == self.grid.row_top() {
                continue;
            }
            let (data, tidx) = get_iob_data_l(self.grid.kind, self.grid.rows[row]);
            let mut pads = vec![];
            let mut ipads = vec![];
            for &iob in &data.iobs {
                if iob.tile == tidx {
                    if iob.kind == IobKind::Ibuf && self.grid.kind != GridKind::FpgaCore {
                        ipads.push(iob.bel.to_idx());
                    } else {
                        pads.push(iob.bel.to_idx());
                    }
                }
            }
            let iobs: &[usize] = match self.grid.kind {
                GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => &[0, 1, 2, 3],
                GridKind::Spartan3 => &[0, 1, 2],
                GridKind::FpgaCore => &[0, 4, 1, 5, 2, 6, 3, 7],
                GridKind::Spartan3E => &[2, 1, 0],
                GridKind::Spartan3A | GridKind::Spartan3ADsp => &[0, 1],
            };
            let nnode = self
                .ngrid
                .nodes
                .get_mut(&(self.die.die, col, row, LayerId::from_idx(1)))
                .unwrap();
            for &i in iobs {
                if pads.contains(&i) {
                    nnode.add_bel(i, format!("PAD{idx}", idx = self.ctr_pad));
                    self.ctr_pad += 1;
                } else if ipads.contains(&i) {
                    nnode.add_bel(i, format!("IPAD{idx}", idx = self.ctr_pad));
                    self.ctr_pad += 1;
                } else {
                    nnode.add_bel(i, format!("NOPAD{idx}", idx = self.ctr_nopad));
                    self.ctr_nopad += 1;
                }
            }
        }
    }
}

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let egrid = &edev.egrid;
    let grid = edev.grid;
    let ngrid = ExpandedGridNaming::new(ndb, egrid);
    let dcm_grid = ngrid.bel_grid(|_, name, _| name.starts_with("DCM."));
    let bram_grid = ngrid.bel_grid(|_, name, _| name.starts_with("BRAM"));
    let mut namer = Namer {
        edev,
        grid,
        die: egrid.die(DieId::from_idx(0)),
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
        ctr_nopad: if grid.kind.is_spartan3ea() { 0 } else { 1 },
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

    for die in egrid.dies() {
        for col in die.cols() {
            for row in die.rows() {
                for (layer, node) in &die[(col, row)].nodes {
                    let nloc = (die.die, col, row, layer);
                    let kind = egrid.db.nodes.key(node.kind);
                    match &kind[..] {
                        _ if kind.starts_with("INT.") => {
                            let (naming, name) = namer.get_int_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, naming, [name]);
                            let x = col.to_idx();
                            let y = row.to_idx();
                            nnode.add_bel(0, format!("RLL_X{x}Y{y}"));
                            if kind != "INT.DCM.S3E.DUMMY" {
                                let mut x = namer.vcc_xlut[col];
                                let mut y = namer.vcc_ylut[row];
                                if grid.kind == GridKind::Virtex2 {
                                    // Look, just..... don't ask me.
                                    x = col.to_idx();
                                    if col == grid.col_left() {
                                        if row == grid.row_bot() {
                                            y = grid.rows.len() - 2;
                                        } else if row == grid.row_top() {
                                            y = grid.rows.len() - 1;
                                        } else {
                                            y -= 1;
                                        }
                                    } else if col == grid.col_right() {
                                        if row == grid.row_bot() {
                                            y = 0;
                                            x += 1;
                                        } else if row == grid.row_top() {
                                            y = 1;
                                            x += 1;
                                        } else {
                                            y += 1;
                                        }
                                    } else if col < grid.col_clk {
                                        if row == grid.row_bot() {
                                            y = 0;
                                        } else if row == grid.row_top() {
                                            y = 1;
                                        } else {
                                            y += 1;
                                        }
                                    } else {
                                        if row == grid.row_bot() {
                                            y = 2;
                                        } else if row == grid.row_top() {
                                            y = 3;
                                        } else {
                                            y += 3;
                                            if y >= grid.rows.len() {
                                                y -= grid.rows.len();
                                                x += 1;
                                            }
                                        }
                                    }
                                }
                                nnode.tie_name = Some(format!("VCC_X{x}Y{y}"));
                            }
                        }
                        "INTF.PPC" => {
                            let (naming, name) = namer.get_int_name(col, row);
                            let naming = format!("INTF.{}", &naming[4..]);
                            namer.ngrid.name_node(nloc, &naming, [name]);
                        }
                        "INTF.GT.BCLKPAD" | "INTF.GT.TCLKPAD" => {
                            let (_, name) = namer.get_int_name(col, row);
                            namer.ngrid.name_node(nloc, "INTF.GT.CLKPAD", [name]);
                        }
                        "INTF.GT.B0" | "INTF.GT.B123" | "INTF.GT.T0" | "INTF.GT.T123" => {
                            let (_, name) = namer.get_int_name(col, row);
                            namer.ngrid.name_node(nloc, "INTF.GT", [name]);
                        }
                        "CLB" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, "CLB", [name]);
                            let sx = namer.sxlut[col];
                            let sy = 2 * (row.to_idx() - 1);
                            if grid.kind.is_virtex2() {
                                nnode.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                                nnode.add_bel(1, format!("SLICE_X{x}Y{y}", x = sx, y = sy + 1));
                                nnode.add_bel(2, format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy));
                                nnode.add_bel(3, format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy + 1));
                                if sx % 4 == 0 {
                                    nnode.add_bel(4, format!("TBUF_X{sx}Y{sy}"));
                                    nnode.add_bel(5, format!("TBUF_X{x}Y{y}", x = sx, y = sy + 1));
                                } else {
                                    nnode.add_bel(4, format!("TBUF_X{x}Y{y}", x = sx, y = sy + 1));
                                    nnode.add_bel(5, format!("TBUF_X{sx}Y{sy}"));
                                }
                            } else {
                                nnode.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                                nnode.add_bel(1, format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy));
                                nnode.add_bel(2, format!("SLICE_X{x}Y{y}", x = sx, y = sy + 1));
                                nnode.add_bel(3, format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy + 1));
                            }
                        }
                        "RANDOR" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let naming = if row == grid.row_bot() {
                                "RANDOR.B"
                            } else if row == grid.row_top() {
                                "RANDOR.T"
                            } else {
                                unreachable!()
                            };
                            let nnode = namer.ngrid.name_node(nloc, naming, [name]);
                            let x = if grid.kind == GridKind::Spartan3 {
                                (namer.clut[col] - 1) * 2
                            } else {
                                col.to_idx() - 1
                            };
                            let y = if grid.kind != GridKind::FpgaCore && naming == "RANDOR.T" {
                                1
                            } else {
                                0
                            };
                            nnode.add_bel(0, format!("RANDOR_X{x}Y{y}"));
                        }
                        "BRAM" | "BRAM.S3" | "BRAM.S3E" | "BRAM.S3A" | "BRAM.S3ADSP" => {
                            let (naming, name) = namer.get_bram_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, naming, [name]);
                            let x = namer.bram_grid.xlut[col];
                            let y = namer.bram_grid.ylut[row];
                            nnode.add_bel(0, format!("RAMB16_X{x}Y{y}"));
                            if grid.kind != GridKind::Spartan3ADsp {
                                nnode.add_bel(1, format!("MULT18X18_X{x}Y{y}"));
                            }
                        }
                        "DSP" => {
                            let (naming, name) = namer.get_dsp_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, naming, [name]);
                            let x = namer.bram_grid.xlut[col - 3];
                            let y = namer.bram_grid.ylut[row];
                            nnode.add_bel(0, format!("DSP48A_X{x}Y{y}"));
                        }
                        "INTF.DSP" => {
                            let (_, name) = namer.get_dsp_name(col, row);
                            namer.ngrid.name_node(nloc, "INTF.DSP", [name]);
                        }
                        "GIGABIT.B" => {
                            let c = namer.bramclut[col];
                            let r = namer.rlut[row + 1];
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, "GIGABIT.B", [format!("BMR{r}C{c}")]);
                            let gx = namer.gtxlut[col];
                            let (bank, _) = grid.cols_gt[&col];
                            nnode.add_bel(0, format!("GT_X{gx}Y0"));
                            nnode.add_bel(1, format!("RXPPAD{bank}"));
                            nnode.add_bel(2, format!("RXNPAD{bank}"));
                            nnode.add_bel(3, format!("TXPPAD{bank}"));
                            nnode.add_bel(4, format!("TXNPAD{bank}"));
                        }
                        "GIGABIT10.B" => {
                            let c = namer.bramclut[col];
                            let r = namer.rlut[row + 1];
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, "GIGABIT10.B", [format!("BMR{r}C{c}")]);
                            let gx = namer.gtxlut[col];
                            let (bank, _) = grid.cols_gt[&col];
                            nnode.add_bel(0, format!("GT10_X{gx}Y0"));
                            nnode.add_bel(1, format!("RXPPAD{bank}"));
                            nnode.add_bel(2, format!("RXNPAD{bank}"));
                            nnode.add_bel(3, format!("TXPPAD{bank}"));
                            nnode.add_bel(4, format!("TXNPAD{bank}"));
                        }
                        "GIGABIT.T" => {
                            let c = namer.bramclut[col];
                            let r = namer.rlut[row - 4];
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, "GIGABIT.T", [format!("BMR{r}C{c}")]);
                            let gx = namer.gtxlut[col];
                            let (_, bank) = grid.cols_gt[&col];
                            nnode.add_bel(0, format!("GT_X{gx}Y1"));
                            nnode.add_bel(1, format!("RXPPAD{bank}"));
                            nnode.add_bel(2, format!("RXNPAD{bank}"));
                            nnode.add_bel(3, format!("TXPPAD{bank}"));
                            nnode.add_bel(4, format!("TXNPAD{bank}"));
                        }
                        "GIGABIT10.T" => {
                            let c = namer.bramclut[col];
                            let r = namer.rlut[row - 8];
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, "GIGABIT10.T", [format!("BMR{r}C{c}")]);
                            let gx = namer.gtxlut[col];
                            let (_, bank) = grid.cols_gt[&col];
                            nnode.add_bel(0, format!("GT10_X{gx}Y1"));
                            nnode.add_bel(1, format!("RXPPAD{bank}"));
                            nnode.add_bel(2, format!("RXNPAD{bank}"));
                            nnode.add_bel(3, format!("TXPPAD{bank}"));
                            nnode.add_bel(4, format!("TXNPAD{bank}"));
                        }
                        "LBPPC" | "RBPPC" => {
                            let x = if kind == "LBPPC" || grid.holes_ppc.len() == 1 {
                                0
                            } else {
                                1
                            };
                            let nnode = namer.ngrid.name_node(nloc, kind, [format!("PPC_X{x}Y0")]);
                            nnode.add_bel(0, format!("PPC405_X{x}Y0"));
                        }
                        "TERM.W" => {
                            let (naming, name) = namer.get_lterm_name(row);
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "TERM.E" => {
                            let (naming, name) = namer.get_rterm_name(row);
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "TERM.S" => {
                            let (naming, name) = namer.get_bterm_name(col);
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "TERM.N" => {
                            let (naming, name) = namer.get_tterm_name(col);
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "PPC.E" => {
                            let (name_l, name_r) = namer.get_ppc_h_name(col, row);
                            namer.ngrid.name_node(nloc, "PPC.E", [name_l, name_r]);
                        }
                        "PPC.W" => {
                            let (name_l, name_r) = namer.get_ppc_h_name(col - 9, row);
                            namer.ngrid.name_node(nloc, "PPC.W", [name_r, name_l]);
                        }
                        "PPC.N" => {
                            let (name_b, name_t) = namer.get_ppc_v_name(col, row);
                            namer.ngrid.name_node(nloc, "PPC.N", [name_b, name_t]);
                        }
                        "PPC.S" => {
                            let (name_b, name_t) = namer.get_ppc_v_name(col, row - 15);
                            namer.ngrid.name_node(nloc, "PPC.S", [name_t, name_b]);
                        }
                        "LLV.S3E" | "LLV.S3A" => {
                            let (naming, name) = namer.get_llv_name(col);
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "LLH" | "LLH.CLKB.S3A" | "LLH.CLKT.S3A" => {
                            let name = namer.get_llh_name(row);
                            namer.ngrid.name_node(nloc, "LLH", [name]);
                        }
                        "CLKB.V2" | "CLKB.V2P" | "CLKB.V2PX" => {
                            let name = match grid.kind {
                                GridKind::Virtex2 => "CLKB",
                                GridKind::Virtex2P => "ML_CLKB",
                                GridKind::Virtex2PX => "MK_CLKB",
                                _ => unreachable!(),
                            };
                            let nnode = namer.ngrid.name_node(nloc, kind, [name.into()]);
                            let vx = namer.vcc_xlut[grid.col_clk] - 1;
                            let vy = grid.row_bot().to_idx();
                            nnode.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                            nnode.add_bel(0, "BUFGMUX0P".to_string());
                            nnode.add_bel(1, "BUFGMUX1S".to_string());
                            nnode.add_bel(2, "BUFGMUX2P".to_string());
                            nnode.add_bel(3, "BUFGMUX3S".to_string());
                            nnode.add_bel(4, "BUFGMUX4P".to_string());
                            nnode.add_bel(5, "BUFGMUX5S".to_string());
                            nnode.add_bel(6, "BUFGMUX6P".to_string());
                            nnode.add_bel(7, "BUFGMUX7S".to_string());
                            nnode.add_bel(8, format!("GSIG_X{x}Y0", x = grid.col_clk.to_idx()));
                            nnode.add_bel(9, format!("GSIG_X{x}Y0", x = grid.col_clk.to_idx() + 1));
                        }
                        "CLKT.V2" | "CLKT.V2P" | "CLKT.V2PX" => {
                            let name = match grid.kind {
                                GridKind::Virtex2 => "CLKT",
                                GridKind::Virtex2P => "ML_CLKT",
                                GridKind::Virtex2PX => "MK_CLKT",
                                _ => unreachable!(),
                            };
                            let nnode = namer.ngrid.name_node(nloc, kind, [name.into()]);
                            let vx = namer.vcc_xlut[grid.col_clk] - 1;
                            let vy = if grid.kind == GridKind::Virtex2 {
                                1
                            } else {
                                grid.rows.len() - 1
                            };
                            nnode.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                            nnode.add_bel(0, "BUFGMUX0S".to_string());
                            nnode.add_bel(1, "BUFGMUX1P".to_string());
                            nnode.add_bel(2, "BUFGMUX2S".to_string());
                            nnode.add_bel(3, "BUFGMUX3P".to_string());
                            nnode.add_bel(4, "BUFGMUX4S".to_string());
                            nnode.add_bel(5, "BUFGMUX5P".to_string());
                            nnode.add_bel(6, "BUFGMUX6S".to_string());
                            nnode.add_bel(7, "BUFGMUX7P".to_string());
                            nnode.add_bel(8, format!("GSIG_X{x}Y1", x = grid.col_clk.to_idx()));
                            nnode.add_bel(9, format!("GSIG_X{x}Y1", x = grid.col_clk.to_idx() + 1));
                        }
                        "CLKB.S3" | "CLKB.FC" => {
                            let bufg = if grid.kind == GridKind::FpgaCore {
                                "BUFG"
                            } else {
                                "BUFGMUX"
                            };
                            let nnode = namer.ngrid.name_node(nloc, kind, ["CLKB".into()]);
                            let vx = namer.vcc_xlut[grid.col_clk] - 1;
                            let vy = 0;
                            nnode.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                            nnode.add_bel(0, format!("{bufg}0"));
                            nnode.add_bel(1, format!("{bufg}1"));
                            nnode.add_bel(2, format!("{bufg}2"));
                            nnode.add_bel(3, format!("{bufg}3"));
                            nnode.add_bel(4, format!("GSIG_X{x}Y0", x = grid.col_clk.to_idx()));
                        }
                        "CLKT.S3" | "CLKT.FC" => {
                            let bufg = if grid.kind == GridKind::FpgaCore {
                                "BUFG"
                            } else {
                                "BUFGMUX"
                            };
                            let nnode = namer.ngrid.name_node(nloc, kind, ["CLKT".into()]);
                            let vx = namer.vcc_xlut[grid.col_clk] - 1;
                            let vy = namer.vcc_ylut[grid.row_top()];
                            nnode.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                            nnode.add_bel(0, format!("{bufg}4"));
                            nnode.add_bel(1, format!("{bufg}5"));
                            nnode.add_bel(2, format!("{bufg}6"));
                            nnode.add_bel(3, format!("{bufg}7"));
                            nnode.add_bel(4, format!("GSIG_X{x}Y1", x = grid.col_clk.to_idx()));
                        }
                        "CLKB.S3E" | "CLKB.S3A" => {
                            let x = namer.xlut[grid.col_clk - 1];
                            let y = row.to_idx();
                            let yb = y + 1;
                            let (name, name_buf) = if grid.has_ll {
                                (format!("CLKB_LL_X{x}Y{y}"), format!("CLKV_LL_X{x}Y{yb}"))
                            } else {
                                (format!("CLKB_X{x}Y{y}"), format!("CLKV_X{x}Y{yb}"))
                            };
                            let nnode = namer.ngrid.name_node(nloc, kind, [name, name_buf]);
                            let vx = namer.vcc_xlut[grid.col_clk] - 1;
                            let vy = 0;
                            nnode.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                            nnode.add_bel(0, "BUFGMUX_X2Y1".to_string());
                            nnode.add_bel(1, "BUFGMUX_X2Y0".to_string());
                            nnode.add_bel(2, "BUFGMUX_X1Y1".to_string());
                            nnode.add_bel(3, "BUFGMUX_X1Y0".to_string());
                            nnode.add_bel(
                                4,
                                format!("GLOBALSIG_X{x}Y0", x = namer.xlut[grid.col_clk] + 1),
                            );
                        }
                        "CLKT.S3E" | "CLKT.S3A" => {
                            let x = namer.xlut[grid.col_clk - 1];
                            let y = row.to_idx();
                            let yb = y - 1;
                            let (name, name_buf) = if grid.has_ll {
                                (format!("CLKT_LL_X{x}Y{y}"), format!("CLKV_LL_X{x}Y{yb}"))
                            } else {
                                (format!("CLKT_X{x}Y{y}"), format!("CLKV_X{x}Y{yb}"))
                            };
                            let nnode = namer.ngrid.name_node(nloc, kind, [name, name_buf]);
                            let vx = namer.vcc_xlut[grid.col_clk] - 1;
                            let vy = namer.vcc_ylut[grid.row_top()];
                            nnode.tie_name = Some(format!("VCC_X{vx}Y{vy}"));
                            nnode.add_bel(0, "BUFGMUX_X2Y11".to_string());
                            nnode.add_bel(1, "BUFGMUX_X2Y10".to_string());
                            nnode.add_bel(2, "BUFGMUX_X1Y11".to_string());
                            nnode.add_bel(3, "BUFGMUX_X1Y10".to_string());
                            nnode.add_bel(
                                4,
                                format!(
                                    "GLOBALSIG_X{x}Y{y}",
                                    x = namer.xlut[grid.col_clk] + 1,
                                    y = grid.rows_hclk.len() + 2
                                ),
                            );
                        }
                        "CLKL.S3E" | "CLKL.S3A" => {
                            let x = namer.xlut[col];
                            let y = row.to_idx() - 1;
                            let mut names = vec![format!("CLKL_X{x}Y{y}")];
                            if grid.kind != GridKind::Spartan3E {
                                names.push(if grid.has_ll {
                                    format!("CLKL_IOIS_LL_X{x}Y{y}")
                                } else if grid.cols_clkv.is_none() {
                                    format!("CLKL_IOIS_50A_X{x}Y{y}")
                                } else {
                                    format!("CLKL_IOIS_X{x}Y{y}")
                                });
                            }
                            let nnode = namer.ngrid.name_node(nloc, kind, names);
                            let vy = namer.vcc_ylut[grid.row_mid()] - 1;
                            let vx = 0;
                            let gsy = (grid.rows_hclk.len() + 1) / 2 + 1;
                            nnode.add_bel(0, "BUFGMUX_X0Y2".to_string());
                            nnode.add_bel(1, "BUFGMUX_X0Y3".to_string());
                            nnode.add_bel(2, "BUFGMUX_X0Y4".to_string());
                            nnode.add_bel(3, "BUFGMUX_X0Y5".to_string());
                            nnode.add_bel(4, "BUFGMUX_X0Y6".to_string());
                            nnode.add_bel(5, "BUFGMUX_X0Y7".to_string());
                            nnode.add_bel(6, "BUFGMUX_X0Y8".to_string());
                            nnode.add_bel(7, "BUFGMUX_X0Y9".to_string());
                            nnode.add_bel(8, "PCILOGIC_X0Y0".to_string());
                            nnode.add_bel(9, format!("VCC_X{vx}Y{vy}"));
                            nnode.add_bel(10, format!("GLOBALSIG_X0Y{gsy}"));
                        }
                        "CLKR.S3E" | "CLKR.S3A" => {
                            let x = namer.xlut[col];
                            let y = row.to_idx() - 1;
                            let mut names = vec![format!("CLKR_X{x}Y{y}")];
                            if grid.kind != GridKind::Spartan3E {
                                names.push(if grid.has_ll {
                                    format!("CLKR_IOIS_LL_X{x}Y{y}")
                                } else if grid.cols_clkv.is_none() {
                                    format!("CLKR_IOIS_50A_X{x}Y{y}")
                                } else {
                                    format!("CLKR_IOIS_X{x}Y{y}")
                                });
                            }
                            let nnode = namer.ngrid.name_node(nloc, kind, names);
                            let vy = namer.vcc_ylut[grid.row_mid()] - 1;
                            let vx = namer.vcc_xlut[grid.col_right()] + 1;
                            let gsy = (grid.rows_hclk.len() + 1) / 2 + 1;
                            nnode.add_bel(0, "BUFGMUX_X3Y2".to_string());
                            nnode.add_bel(1, "BUFGMUX_X3Y3".to_string());
                            nnode.add_bel(2, "BUFGMUX_X3Y4".to_string());
                            nnode.add_bel(3, "BUFGMUX_X3Y5".to_string());
                            nnode.add_bel(4, "BUFGMUX_X3Y6".to_string());
                            nnode.add_bel(5, "BUFGMUX_X3Y7".to_string());
                            nnode.add_bel(6, "BUFGMUX_X3Y8".to_string());
                            nnode.add_bel(7, "BUFGMUX_X3Y9".to_string());
                            nnode.add_bel(8, "PCILOGIC_X1Y0".to_string());
                            nnode.add_bel(9, format!("VCC_X{vx}Y{vy}"));
                            nnode.add_bel(
                                10,
                                format!(
                                    "GLOBALSIG_X{x}Y{gsy}",
                                    x = namer.xlut[grid.col_right()] + 3
                                ),
                            );
                        }

                        "REG_L" => {
                            let rb = namer.rlut[row - 1];
                            let rt = namer.rlut[row];
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                "REG_L",
                                [
                                    if grid.kind == GridKind::Virtex2 {
                                        "HMLTERM"
                                    } else {
                                        "LTERMCLKH"
                                    }
                                    .into(),
                                    format!("LTERMR{rb}"),
                                    format!("LTERMR{rt}"),
                                ],
                            );
                            nnode.add_bel(0, "PCILOGIC_X0Y0".into());
                        }
                        "REG_R" => {
                            let rb = namer.rlut[row - 1];
                            let rt = namer.rlut[row];
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                "REG_R",
                                [
                                    if grid.kind == GridKind::Virtex2 {
                                        "HMRTERM"
                                    } else {
                                        "RTERMCLKH"
                                    }
                                    .into(),
                                    format!("RTERMR{rb}"),
                                    format!("RTERMR{rt}"),
                                ],
                            );
                            nnode.add_bel(0, "PCILOGIC_X1Y0".into());
                        }
                        "GCLKH" | "GCLKH.UNI" | "GCLKH.S" | "GCLKH.UNI.S" | "GCLKH.N"
                        | "GCLKH.UNI.N" | "GCLKH.0" => {
                            let (naming, name) = namer.get_hclk_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, naming, [name]);
                            if !grid.kind.is_spartan3ea() {
                                let gsx = if col < grid.col_clk {
                                    col.to_idx()
                                } else if !grid.kind.is_virtex2() {
                                    col.to_idx() + 1
                                } else {
                                    col.to_idx() + 2
                                };
                                let gsy = namer.hclklut[row];
                                nnode.add_bel(0, format!("GSIG_X{gsx}Y{gsy}"));
                            } else {
                                let gsx = if col < grid.col_clk {
                                    namer.xlut[col] + 1
                                } else {
                                    namer.xlut[col] + 2
                                };
                                let gsy = if row <= grid.row_mid() {
                                    namer.hclklut[row] + 1
                                } else {
                                    namer.hclklut[row] + 2
                                };
                                nnode.add_bel(0, format!("GLOBALSIG_X{gsx}Y{gsy}"));
                            }
                        }
                        "GCLKH.DSP" => {
                            let name = format!(
                                "MACC2_GCLKH_FEEDTHRUA_X{x}Y{y}",
                                x = namer.xlut[col] + 1,
                                y = row.to_idx() - 1
                            );
                            let nnode = namer.ngrid.name_node(nloc, "GCLKH.DSP", [name]);
                            let gsx = if col < grid.col_clk {
                                namer.xlut[col] + 1
                            } else {
                                namer.xlut[col] + 2
                            } + 1;
                            let gsy = if row <= grid.row_mid() {
                                namer.hclklut[row] + 1
                            } else {
                                namer.hclklut[row] + 2
                            };
                            nnode.add_bel(0, format!("GLOBALSIG_X{gsx}Y{gsy}"));
                        }
                        "PCI_CE_CNR" => {
                            let (_, name) = namer.get_int_name(col, row);
                            namer.ngrid.name_node(nloc, "PCI_CE_CNR", [name]);
                        }
                        "PCI_CE_N" | "PCI_CE_S" => {
                            let (_, name) = namer.get_hclk_name(col, row);
                            namer.ngrid.name_node(nloc, kind, [name]);
                        }
                        "PCI_CE_E" => {
                            let x = namer.xlut[col] - 1;
                            let y = row.to_idx();
                            let name = format!("GCLKV_IOISL_X{x}Y{y}");
                            namer.ngrid.name_node(nloc, "PCI_CE_E", [name]);
                        }
                        "PCI_CE_W" => {
                            let x = namer.xlut[col] - 1;
                            let y = row.to_idx();
                            let name = format!("GCLKV_IOISR_X{x}Y{y}");
                            namer.ngrid.name_node(nloc, "PCI_CE_W", [name]);
                        }
                        "GCLKC" | "GCLKC.B" | "GCLKC.T" => {
                            let mut r = grid.rows_hclk.len() - namer.hclklut[row];
                            // I hate ISE.
                            if grid.columns.len() == 12 {
                                r -= 1;
                            }
                            let name = format!("GCLKCR{r}");
                            namer.ngrid.name_node(nloc, "GCLKC", [name]);
                        }
                        "GCLKVC" => {
                            let name = if !grid.kind.is_spartan3ea() {
                                let r = grid.rows_hclk.len() - namer.hclklut[row];
                                let lr = if col < grid.col_clk { 'L' } else { 'R' };
                                format!("{lr}CLKVCR{r}")
                            } else {
                                let x = namer.xlut[col] - 1;
                                let y = row.to_idx() - 1;
                                format!("GCLKVC_X{x}Y{y}")
                            };
                            namer.ngrid.name_node(nloc, "GCLKVC", [name]);
                        }
                        "CLKC" => {
                            let name = if grid.kind.is_spartan3ea() {
                                let x = namer.xlut[col] - 1;
                                let y = row.to_idx() - 1;
                                if grid.kind == GridKind::Spartan3E && grid.has_ll {
                                    format!("CLKC_LL_X{x}Y{y}")
                                } else {
                                    format!("CLKC_X{x}Y{y}")
                                }
                            } else {
                                "M".to_string()
                            };
                            namer.ngrid.name_node(nloc, "CLKC", [name]);
                        }
                        "CLKC_50A" => {
                            let x = namer.xlut[col] - 1;
                            let y = row.to_idx() - 1;
                            let name = format!("CLKC_50A_X{x}Y{y}");
                            namer.ngrid.name_node(nloc, "CLKC_50A", [name]);
                        }
                        "GCLKVM.S3" => {
                            let lr = if col < grid.col_clk { 'L' } else { 'R' };
                            let name = format!("{lr}GCLKVM");
                            namer.ngrid.name_node(nloc, "GCLKVM.S3", [name]);
                        }
                        "GCLKVM.S3E" => {
                            let x = namer.xlut[col] - 1;
                            let y = row.to_idx() - 1;
                            let naming = if col < grid.col_clk {
                                "GCLKVML"
                            } else {
                                "GCLKVMR"
                            };
                            let name = format!("{naming}_X{x}Y{y}");
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }

                        "IOI" | "IOI.CLK_B" | "IOI.CLK_T" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let naming = if matches!(
                                grid.columns[col].io,
                                ColumnIoKind::SingleLeftAlt | ColumnIoKind::SingleRightAlt
                            ) {
                                "IOI.TBS"
                            } else {
                                kind
                            };
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "IOI.S3" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let naming = if col == grid.col_left() {
                                "IOI.S3.L"
                            } else if col == grid.col_right() {
                                "IOI.S3.R"
                            } else if row == grid.row_bot() {
                                "IOI.S3.B"
                            } else if row == grid.row_top() {
                                "IOI.S3.T"
                            } else {
                                unreachable!()
                            };
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "IOI.FC" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let naming = if col == grid.col_left() {
                                "IOI.FC.L"
                            } else if col == grid.col_right() {
                                "IOI.FC.R"
                            } else if row == grid.row_bot() {
                                "IOI.FC.B"
                            } else if row == grid.row_top() {
                                "IOI.FC.T"
                            } else {
                                unreachable!()
                            };
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "IOI.S3E" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let naming = if col == grid.col_left() {
                                if row >= grid.row_mid() - 4 && row < grid.row_mid() + 4 {
                                    if row == grid.row_mid() - 4 || row == grid.row_mid() {
                                        "IOI.S3E.L.PCI"
                                    } else {
                                        "IOI.S3E.L.PCI.PCI"
                                    }
                                } else {
                                    "IOI.S3E.L"
                                }
                            } else if col == grid.col_right() {
                                if row >= grid.row_mid() - 4 && row < grid.row_mid() + 4 {
                                    if row == grid.row_mid() - 1 || row == grid.row_mid() + 3 {
                                        "IOI.S3E.R.PCI"
                                    } else {
                                        "IOI.S3E.R.PCI.PCI"
                                    }
                                } else {
                                    "IOI.S3E.R"
                                }
                            } else if row == grid.row_bot() {
                                "IOI.S3E.B"
                            } else if row == grid.row_top() {
                                "IOI.S3E.T"
                            } else {
                                unreachable!()
                            };
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "IOI.S3A.B" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let naming = if grid.kind == GridKind::Spartan3ADsp {
                                "IOI.S3ADSP.B"
                            } else {
                                "IOI.S3A.B"
                            };
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "IOI.S3A.T" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let naming = if grid.kind == GridKind::Spartan3ADsp {
                                "IOI.S3ADSP.T"
                            } else {
                                "IOI.S3A.T"
                            };
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        "IOI.S3A.LR" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let naming = if col == grid.col_left() {
                                if row >= grid.row_mid() - 4
                                    && row < grid.row_mid() + 4
                                    && row != grid.row_mid() - 4
                                    && row != grid.row_mid()
                                {
                                    if grid.kind == GridKind::Spartan3ADsp {
                                        "IOI.S3ADSP.L.PCI"
                                    } else {
                                        "IOI.S3A.L.PCI"
                                    }
                                } else {
                                    if grid.kind == GridKind::Spartan3ADsp {
                                        "IOI.S3ADSP.L"
                                    } else {
                                        "IOI.S3A.L"
                                    }
                                }
                            } else if col == grid.col_right() {
                                if row >= grid.row_mid() - 4
                                    && row < grid.row_mid() + 4
                                    && row != grid.row_mid() - 1
                                    && row != grid.row_mid() + 3
                                {
                                    if grid.kind == GridKind::Spartan3ADsp {
                                        "IOI.S3ADSP.R.PCI"
                                    } else {
                                        "IOI.S3A.R.PCI"
                                    }
                                } else {
                                    if grid.kind == GridKind::Spartan3ADsp {
                                        "IOI.S3ADSP.R"
                                    } else {
                                        "IOI.S3A.R"
                                    }
                                }
                            } else {
                                unreachable!()
                            };
                            namer.ngrid.name_node(nloc, naming, [name]);
                        }
                        _ if kind.starts_with("IOBS.") => (),

                        _ if kind.starts_with("DCM.") => {
                            let (_, name) = namer.get_int_name(col, row);
                            let naming = if kind.starts_with("DCM.S3E") {
                                if row >= grid.row_mid() - 4 && row < grid.row_mid() + 4 {
                                    "DCM.S3E.H"
                                } else if col < grid.col_clk {
                                    "DCM.S3E.L"
                                } else {
                                    "DCM.S3E.R"
                                }
                            } else {
                                kind
                            };
                            let nnode = namer.ngrid.name_node(nloc, naming, [name]);
                            let x = namer.dcm_grid.xlut[col];
                            let y = namer.dcm_grid.ylut[row];
                            nnode.add_bel(0, format!("DCM_X{x}Y{y}"));
                        }
                        "DCMCONN.BOT" => {
                            let (_, name) = namer.get_bterm_name(col);
                            namer.ngrid.name_node(nloc, "DCMCONN.BOT", [name]);
                        }
                        "DCMCONN.TOP" => {
                            let (_, name) = namer.get_tterm_name(col);
                            namer.ngrid.name_node(nloc, "DCMCONN.TOP", [name]);
                        }

                        "LL.V2" | "LL.V2P" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, kind, [name]);
                            nnode.add_bel(0, "DCI6".to_string());
                            nnode.add_bel(1, "DCI5".to_string());
                        }
                        "LL.S3" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, kind, [name]);
                            nnode.add_bel(0, "DCI6".to_string());
                            nnode.add_bel(1, "DCI5".to_string());
                            nnode.add_bel(2, "DCIRESET6".to_string());
                            nnode.add_bel(3, "DCIRESET5".to_string());
                        }
                        "LL.FC" | "LL.S3E" | "LL.S3A" => {
                            let (_, name) = namer.get_int_name(col, row);
                            namer.ngrid.name_node(nloc, kind, [name]);
                        }
                        "LR.V2" | "LR.V2P" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, kind, [name]);
                            nnode.add_bel(0, "DCI3".to_string());
                            nnode.add_bel(1, "DCI4".to_string());
                            nnode.add_bel(2, "STARTUP".to_string());
                            nnode.add_bel(3, "CAPTURE".to_string());
                            nnode.add_bel(4, "ICAP".to_string());
                        }
                        "LR.S3" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, kind, [name]);
                            nnode.add_bel(0, "DCI3".to_string());
                            nnode.add_bel(1, "DCI4".to_string());
                            nnode.add_bel(2, "DCIRESET3".to_string());
                            nnode.add_bel(3, "DCIRESET4".to_string());
                            nnode.add_bel(4, "STARTUP".to_string());
                            nnode.add_bel(5, "CAPTURE".to_string());
                            nnode.add_bel(6, "ICAP".to_string());
                        }
                        "LR.FC" | "LR.S3E" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, kind, [name]);
                            nnode.add_bel(0, "STARTUP".to_string());
                            nnode.add_bel(1, "CAPTURE".to_string());
                            nnode.add_bel(2, "ICAP".to_string());
                        }
                        "LR.S3A" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, kind, [name]);
                            nnode.add_bel(0, "STARTUP".to_string());
                            nnode.add_bel(1, "CAPTURE".to_string());
                            nnode.add_bel(2, "ICAP".to_string());
                            nnode.add_bel(3, "SPI_ACCESS".to_string());
                        }
                        "UL.V2" | "UL.V2P" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, kind, [name]);
                            nnode.add_bel(0, "DCI7".to_string());
                            nnode.add_bel(1, "DCI0".to_string());
                            nnode.add_bel(2, "PMV".to_string());
                        }
                        "UL.S3" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, kind, [name]);
                            nnode.add_bel(0, "DCI7".to_string());
                            nnode.add_bel(1, "DCI0".to_string());
                            nnode.add_bel(2, "DCIRESET7".to_string());
                            nnode.add_bel(3, "DCIRESET0".to_string());
                            nnode.add_bel(4, "PMV".to_string());
                        }
                        "UL.FC" | "UL.S3E" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, kind, [name]);
                            nnode.add_bel(0, "PMV".to_string());
                        }
                        "UL.S3A" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, kind, [name]);
                            nnode.add_bel(0, "PMV".to_string());
                            nnode.add_bel(1, "DNA_PORT".to_string());
                        }
                        "UR.V2" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, kind, [name]);
                            nnode.add_bel(0, "DCI2".to_string());
                            nnode.add_bel(1, "DCI1".to_string());
                            nnode.add_bel(2, "BSCAN".to_string());
                        }
                        "UR.V2P" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, kind, [name]);
                            nnode.add_bel(0, "DCI2".to_string());
                            nnode.add_bel(1, "DCI1".to_string());
                            nnode.add_bel(2, "BSCAN".to_string());
                            nnode.add_bel(3, "JTAGPPC".to_string());
                        }
                        "UR.S3" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, kind, [name]);
                            nnode.add_bel(0, "DCI2".to_string());
                            nnode.add_bel(1, "DCI1".to_string());
                            nnode.add_bel(2, "DCIRESET2".to_string());
                            nnode.add_bel(3, "DCIRESET1".to_string());
                            nnode.add_bel(4, "BSCAN".to_string());
                        }
                        "UR.FC" | "UR.S3E" | "UR.S3A" => {
                            let (_, name) = namer.get_int_name(col, row);
                            let nnode = namer.ngrid.name_node(nloc, kind, [name]);
                            nnode.add_bel(0, "BSCAN".to_string());
                        }

                        _ => unreachable!(),
                    }
                }
                for (dir, term) in &die[(col, row)].terms {
                    let Some(term) = term else { continue };
                    let tloc = (die.die, col, row, dir);
                    let kind = egrid.db.terms.key(term.kind);

                    match &kind[..] {
                        "TERM.W" => {
                            let (naming, name) = namer.get_lterm_name(row);
                            namer.ngrid.name_term_tile(tloc, naming, name);
                        }
                        "TERM.E" => {
                            let (naming, name) = namer.get_rterm_name(row);
                            namer.ngrid.name_term_tile(tloc, naming, name);
                        }
                        "TERM.S" => {
                            let (naming, name) = namer.get_bterm_name(col);
                            namer.ngrid.name_term_tile(tloc, naming, name);
                        }
                        "TERM.N" => {
                            let (naming, name) = namer.get_tterm_name(col);
                            namer.ngrid.name_term_tile(tloc, naming, name);
                        }
                        "TERM.BRAM.S" => {
                            let x = namer.xlut[col];
                            let y = row.to_idx() - 1;
                            let name = format!("COB_TERM_T_X{x}Y{y}");
                            namer.ngrid.name_term_tile(tloc, "TERM.BRAM.S", name);
                        }
                        "TERM.BRAM.N" => {
                            let x = namer.xlut[col];
                            let y = row.to_idx() + 1;
                            let name = format!("COB_TERM_B_X{x}Y{y}");
                            namer.ngrid.name_term_tile(tloc, "TERM.BRAM.N", name);
                        }
                        "PPC.W" => {
                            let (name_l, name_r) = namer.get_ppc_h_name(col - 9, row);
                            namer.ngrid.name_term_pair(tloc, "PPC.W", name_r, name_l);
                        }
                        "PPC.E" => {
                            let (name_l, name_r) = namer.get_ppc_h_name(col, row);
                            namer.ngrid.name_term_pair(tloc, "PPC.E", name_l, name_r);
                        }
                        "PPC.S" => {
                            let (name_b, name_t) = namer.get_ppc_v_name(col, row - 15);
                            namer.ngrid.name_term_pair(tloc, "PPC.S", name_t, name_b);
                        }
                        "PPC.N" => {
                            let (name_b, name_t) = namer.get_ppc_v_name(col, row);
                            namer.ngrid.name_term_pair(tloc, "PPC.N", name_b, name_t);
                        }
                        "MAIN.S" => {
                            if grid.kind.is_virtex2()
                                && grid.columns[col].kind == ColumnKind::Bram
                                && grid.bram_row(row) == Some(0)
                                && row.to_idx() != 1
                                && !edev.is_in_hole(col, row)
                            {
                                let (_, name) = namer.get_bram_name(col, row);
                                namer.ngrid.name_term_tile(tloc, "BRAM.S", name);
                            }
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    namer.fill_io_t();
    namer.fill_io_r();
    namer.fill_io_b();
    namer.fill_io_l();

    ExpandedNamedDevice {
        edev,
        ngrid: namer.ngrid,
        grid,
    }
}
