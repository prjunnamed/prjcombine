use prjcombine_int::db::{Dir, IntDb, NodeRawTileId};
use prjcombine_int::grid::{ColId, Coord, ExpandedDieRefMut, ExpandedGrid, Rect, RowId};
use prjcombine_virtex_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};
use std::cmp::Ordering;
use std::collections::HashSet;
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::expanded::ExpandedDevice;
use crate::grid::{
    ColumnIoKind, ColumnKind, DcmPairKind, Dcms, Grid, GridKind, IoCoord, RowIoKind, TileIobId,
};

struct Expander<'a, 'b> {
    grid: &'b Grid,
    db: &'a IntDb,
    die: ExpandedDieRefMut<'a, 'b>,
    holes: Vec<Rect>,
    bonded_ios: Vec<IoCoord>,
    xlut: EntityVec<ColId, usize>,
    vcc_xlut: EntityVec<ColId, usize>,
    vcc_ylut: EntityVec<RowId, usize>,
    clut: EntityVec<ColId, usize>,
    rlut: EntityVec<RowId, usize>,
    bramclut: EntityVec<ColId, usize>,
    rows_brk: HashSet<RowId>,
    ctr_pad: usize,
    ctr_nopad: usize,
    frame_info: Vec<FrameInfo>,
    clkv_frame: usize,
    spine_frame: usize,
    lterm_frame: usize,
    rterm_frame: usize,
    col_frame: EntityVec<ColId, usize>,
    bram_frame: EntityPartVec<ColId, usize>,
}

impl<'a, 'b> Expander<'a, 'b> {
    fn is_hole(&self, col: ColId, row: RowId) -> bool {
        for hole in &self.holes {
            if hole.contains(col, row) {
                return true;
            }
        }
        false
    }

    fn fill_xlut(&mut self) {
        let mut x = 0;
        for &cd in self.grid.columns.values() {
            self.xlut.push(x);
            if cd.kind == ColumnKind::Dsp {
                x += 2;
            } else {
                x += 1;
            }
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

    fn fill_rows_brk(&mut self) {
        for &(_, _, r) in &self.grid.rows_hclk {
            self.rows_brk.insert(r - 1);
        }
        self.rows_brk.remove(&self.grid.row_top());
        if self.grid.kind != GridKind::Spartan3ADsp {
            self.rows_brk.remove(&(self.grid.row_mid() - 1));
        }
    }

    fn fill_cnr_int(&mut self) {
        let cnr_kind = if self.grid.kind.is_virtex2() {
            "INT.CNR"
        } else {
            "INT.CLB"
        };
        let col_l = self.grid.col_left();
        let col_r = self.grid.col_right();
        let row_b = self.grid.row_bot();
        let row_t = self.grid.row_top();
        let xl = self.xlut[col_l];
        let xr = self.xlut[col_r];
        let yb = row_b.to_idx();
        let yt = row_t.to_idx();
        if self.grid.kind.is_spartan3ea() {
            self.die.fill_tile(
                (col_l, row_b),
                cnr_kind,
                "INT.CNR",
                format!("LL_X{xl}Y{yb}"),
            );
            self.die.fill_tile(
                (col_r, row_b),
                cnr_kind,
                "INT.CNR",
                format!("LR_X{xr}Y{yb}"),
            );
            self.die.fill_tile(
                (col_l, row_t),
                cnr_kind,
                "INT.CNR",
                format!("UL_X{xl}Y{yt}"),
            );
            self.die.fill_tile(
                (col_r, row_t),
                cnr_kind,
                "INT.CNR",
                format!("UR_X{xr}Y{yt}"),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_l, row_b),
                "TERM.W",
                "TERM.W",
                format!("CNR_LBTERM_X{xl}Y{yb}"),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_l, row_t),
                "TERM.W",
                "TERM.W",
                format!("CNR_LTTERM_X{xl}Y{yt}"),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_r, row_b),
                "TERM.E",
                "TERM.E",
                format!("CNR_RBTERM_X{xr}Y{yb}"),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_r, row_t),
                "TERM.E",
                "TERM.E",
                format!("CNR_RTTERM_X{xr}Y{yt}"),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_l, row_b),
                "TERM.S",
                "TERM.S.CNR",
                format!("CNR_BTERM_X{xl}Y{yb}"),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_l, row_t),
                "TERM.N",
                "TERM.N.CNR",
                format!("CNR_TTERM_X{xl}Y{yt}"),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_r, row_b),
                "TERM.S",
                "TERM.S.CNR",
                format!("CNR_BTERM_X{xr}Y{yb}"),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_r, row_t),
                "TERM.N",
                "TERM.N.CNR",
                format!("CNR_TTERM_X{xr}Y{yt}"),
            );
        } else if self.grid.kind.is_virtex2p() {
            self.die
                .fill_tile((col_l, row_b), cnr_kind, "INT.CNR", "LIOIBIOI".to_string());
            self.die
                .fill_tile((col_r, row_b), cnr_kind, "INT.CNR", "RIOIBIOI".to_string());
            self.die
                .fill_tile((col_l, row_t), cnr_kind, "INT.CNR", "LIOITIOI".to_string());
            self.die
                .fill_tile((col_r, row_t), cnr_kind, "INT.CNR", "RIOITIOI".to_string());
            self.grid.fill_term(
                &mut self.die,
                (col_l, row_b),
                "TERM.W",
                "TERM.W",
                "LTERMBIOI".to_string(),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_l, row_t),
                "TERM.W",
                "TERM.W",
                "LTERMTIOI".to_string(),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_r, row_b),
                "TERM.E",
                "TERM.E",
                "RTERMBIOI".to_string(),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_r, row_t),
                "TERM.E",
                "TERM.E",
                "RTERMTIOI".to_string(),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_l, row_b),
                "TERM.S",
                "TERM.S.CNR",
                "LIOIBTERM".to_string(),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_l, row_t),
                "TERM.N",
                "TERM.N.CNR",
                "LIOITTERM".to_string(),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_r, row_b),
                "TERM.S",
                "TERM.S.CNR",
                "RIOIBTERM".to_string(),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_r, row_t),
                "TERM.N",
                "TERM.N.CNR",
                "RIOITTERM".to_string(),
            );
        } else {
            self.die
                .fill_tile((col_l, row_b), cnr_kind, "INT.CNR", "BL".to_string());
            self.die
                .fill_tile((col_r, row_b), cnr_kind, "INT.CNR", "BR".to_string());
            self.die
                .fill_tile((col_l, row_t), cnr_kind, "INT.CNR", "TL".to_string());
            self.die
                .fill_tile((col_r, row_t), cnr_kind, "INT.CNR", "TR".to_string());
            self.grid.fill_term(
                &mut self.die,
                (col_l, row_b),
                "TERM.W",
                "TERM.W",
                "LBTERM".to_string(),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_l, row_t),
                "TERM.W",
                "TERM.W",
                "LTTERM".to_string(),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_r, row_b),
                "TERM.E",
                "TERM.E",
                "RBTERM".to_string(),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_r, row_t),
                "TERM.E",
                "TERM.E",
                "RTTERM".to_string(),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_l, row_b),
                "TERM.S",
                "TERM.S.CNR",
                "BLTERM".to_string(),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_l, row_t),
                "TERM.N",
                "TERM.N.CNR",
                "TLTERM".to_string(),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_r, row_b),
                "TERM.S",
                "TERM.S.CNR",
                "BRTERM".to_string(),
            );
            self.grid.fill_term(
                &mut self.die,
                (col_r, row_t),
                "TERM.N",
                "TERM.N.CNR",
                "TRTERM".to_string(),
            );
        }
    }

    fn fill_cnr_ll(&mut self) {
        let col = self.grid.col_left();
        let row = self.grid.row_bot();
        let kind = match self.grid.kind {
            GridKind::Virtex2 => "LL.V2",
            GridKind::Virtex2P | GridKind::Virtex2PX => "LL.V2P",
            GridKind::Spartan3 => "LL.S3",
            GridKind::FpgaCore => "LL.FC",
            GridKind::Spartan3E => "LL.S3E",
            GridKind::Spartan3A | GridKind::Spartan3ADsp => "LL.S3A",
        };
        let tile = &mut self.die[(col, row)];
        let name = tile.nodes.first().unwrap().names[NodeRawTileId::from_idx(0)].clone();
        let node = self.die.add_xnode(
            (col, row),
            self.db.get_node(kind),
            &[&name],
            self.db.get_node_naming(kind),
            &[(col, row)],
        );
        if self.grid.kind.is_virtex2() {
            node.add_bel(0, "DCI6".to_string());
            node.add_bel(1, "DCI5".to_string());
        } else if self.grid.kind == GridKind::Spartan3 {
            node.add_bel(0, "DCI6".to_string());
            node.add_bel(1, "DCI5".to_string());
            node.add_bel(2, "DCIRESET6".to_string());
            node.add_bel(3, "DCIRESET5".to_string());
        }
    }

    fn fill_cnr_lr(&mut self) {
        let col = self.grid.col_right();
        let row = self.grid.row_bot();
        let kind = match self.grid.kind {
            GridKind::Virtex2 => "LR.V2",
            GridKind::Virtex2P | GridKind::Virtex2PX => "LR.V2P",
            GridKind::Spartan3 => "LR.S3",
            GridKind::FpgaCore => "LR.FC",
            GridKind::Spartan3E => "LR.S3E",
            GridKind::Spartan3A | GridKind::Spartan3ADsp => "LR.S3A",
        };
        let tile = &mut self.die[(col, row)];
        let name = tile.nodes.first().unwrap().names[NodeRawTileId::from_idx(0)].clone();
        let node = self.die.add_xnode(
            (col, row),
            self.db.get_node(kind),
            &[&name],
            self.db.get_node_naming(kind),
            &[(col, row)],
        );
        if self.grid.kind.is_virtex2() {
            node.add_bel(0, "DCI3".to_string());
            node.add_bel(1, "DCI4".to_string());
            node.add_bel(2, "STARTUP".to_string());
            node.add_bel(3, "CAPTURE".to_string());
            node.add_bel(4, "ICAP".to_string());
        } else if self.grid.kind == GridKind::Spartan3 {
            node.add_bel(0, "DCI3".to_string());
            node.add_bel(1, "DCI4".to_string());
            node.add_bel(2, "DCIRESET3".to_string());
            node.add_bel(3, "DCIRESET4".to_string());
            node.add_bel(4, "STARTUP".to_string());
            node.add_bel(5, "CAPTURE".to_string());
            node.add_bel(6, "ICAP".to_string());
        } else {
            node.add_bel(0, "STARTUP".to_string());
            node.add_bel(1, "CAPTURE".to_string());
            node.add_bel(2, "ICAP".to_string());
            if self.grid.kind.is_spartan3a() {
                node.add_bel(3, "SPI_ACCESS".to_string());
            }
        }
    }

    fn fill_cnr_ul(&mut self) {
        let col = self.grid.col_left();
        let row = self.grid.row_top();
        let kind = match self.grid.kind {
            GridKind::Virtex2 => "UL.V2",
            GridKind::Virtex2P | GridKind::Virtex2PX => "UL.V2P",
            GridKind::Spartan3 => "UL.S3",
            GridKind::FpgaCore => "UL.FC",
            GridKind::Spartan3E => "UL.S3E",
            GridKind::Spartan3A | GridKind::Spartan3ADsp => "UL.S3A",
        };
        let tile = &mut self.die[(col, row)];
        let name = tile.nodes.first().unwrap().names[NodeRawTileId::from_idx(0)].clone();
        let node = self.die.add_xnode(
            (col, row),
            self.db.get_node(kind),
            &[&name],
            self.db.get_node_naming(kind),
            &[(col, row)],
        );
        if self.grid.kind.is_virtex2() {
            node.add_bel(0, "DCI7".to_string());
            node.add_bel(1, "DCI0".to_string());
            node.add_bel(2, "PMV".to_string());
        } else if self.grid.kind == GridKind::Spartan3 {
            node.add_bel(0, "DCI7".to_string());
            node.add_bel(1, "DCI0".to_string());
            node.add_bel(2, "DCIRESET7".to_string());
            node.add_bel(3, "DCIRESET0".to_string());
            node.add_bel(4, "PMV".to_string());
        } else {
            node.add_bel(0, "PMV".to_string());
            if self.grid.kind.is_spartan3a() {
                node.add_bel(1, "DNA_PORT".to_string());
            }
        }
    }

    fn fill_cnr_ur(&mut self) {
        let col = self.grid.col_right();
        let row = self.grid.row_top();
        let kind = match self.grid.kind {
            GridKind::Virtex2 => "UR.V2",
            GridKind::Virtex2P | GridKind::Virtex2PX => "UR.V2P",
            GridKind::Spartan3 => "UR.S3",
            GridKind::FpgaCore => "UR.FC",
            GridKind::Spartan3E => "UR.S3E",
            GridKind::Spartan3A | GridKind::Spartan3ADsp => "UR.S3A",
        };
        let tile = &mut self.die[(col, row)];
        let name = tile.nodes.first().unwrap().names[NodeRawTileId::from_idx(0)].clone();
        let node = self.die.add_xnode(
            (col, row),
            self.db.get_node(kind),
            &[&name],
            self.db.get_node_naming(kind),
            &[(col, row)],
        );
        if self.grid.kind.is_virtex2() {
            node.add_bel(0, "DCI2".to_string());
            node.add_bel(1, "DCI1".to_string());
            node.add_bel(2, "BSCAN".to_string());
            if self.grid.kind.is_virtex2p() {
                node.add_bel(3, "JTAGPPC".to_string());
            }
        } else if self.grid.kind == GridKind::Spartan3 {
            node.add_bel(0, "DCI2".to_string());
            node.add_bel(1, "DCI1".to_string());
            node.add_bel(2, "DCIRESET2".to_string());
            node.add_bel(3, "DCIRESET1".to_string());
            node.add_bel(4, "BSCAN".to_string());
        } else {
            node.add_bel(0, "BSCAN".to_string());
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
            let pads: &[usize];
            let ipads: &[usize];
            let mut int_kind;
            let mut int_naming;
            let mut ioi_kind;
            let mut ioi_naming;
            let mut iobs_kind;
            let iobs: &[usize];
            let mut term = "";
            let mut kind = "";
            match self.grid.kind {
                GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
                    (pads, iobs_kind) = match cd.io {
                        ColumnIoKind::None => (&[][..], None),
                        ColumnIoKind::SingleLeft => (&[2, 1, 0][..], Some(("IOBS.V2P.T.L1", 1))),
                        ColumnIoKind::SingleLeftAlt => {
                            (&[2, 1, 0][..], Some(("IOBS.V2P.T.L1.ALT", 1)))
                        }
                        ColumnIoKind::SingleRight => (&[3, 2, 1][..], Some(("IOBS.V2P.T.R1", 1))),
                        ColumnIoKind::SingleRightAlt => {
                            (&[3, 2, 1][..], Some(("IOBS.V2P.T.R1.ALT", 1)))
                        }
                        ColumnIoKind::DoubleLeft(0) => (
                            &[3, 2, 1, 0][..],
                            Some((
                                if self.grid.kind.is_virtex2p() {
                                    "IOBS.V2P.T.L2"
                                } else {
                                    "IOBS.V2.T.L2"
                                },
                                2,
                            )),
                        ),
                        ColumnIoKind::DoubleLeft(1) => (&[1, 0][..], None),
                        ColumnIoKind::DoubleRight(0) => (
                            &[3, 2][..],
                            Some((
                                if self.grid.kind.is_virtex2p() {
                                    "IOBS.V2P.T.R2"
                                } else {
                                    "IOBS.V2.T.R2"
                                },
                                2,
                            )),
                        ),
                        ColumnIoKind::DoubleRight(1) => (&[3, 2, 1, 0][..], None),
                        _ => unreachable!(),
                    };
                    ipads = &[];
                    int_kind = "INT.IOI";
                    int_naming = "INT.IOI.TB";
                    ioi_kind = "IOI";
                    ioi_naming = "IOI";
                    if matches!(
                        cd.io,
                        ColumnIoKind::SingleLeftAlt | ColumnIoKind::SingleRightAlt
                    ) {
                        ioi_naming = "IOI.TBS";
                    }
                    if self.grid.kind == GridKind::Virtex2PX && col == self.grid.col_clk - 1 {
                        ioi_kind = "IOI.CLK_T";
                        ioi_naming = "IOI.CLK_T";
                        int_kind = "INT.IOI.CLK_T";
                        int_naming = "INT.IOI.CLK_T";
                    }
                    if self.grid.kind == GridKind::Virtex2PX && col == self.grid.col_clk - 2 {
                        iobs_kind = Some(("IOBS.V2P.T.R2.CLK", 2));
                    }
                    iobs = &[3, 2, 1, 0];
                }
                GridKind::Spartan3 => {
                    (pads, iobs_kind) = match cd.io {
                        ColumnIoKind::Double(0) => (&[2, 1, 0][..], Some(("IOBS.S3.T2", 2))),
                        ColumnIoKind::Double(1) => (&[1, 0][..], None),
                        _ => unreachable!(),
                    };
                    ipads = &[];
                    int_kind = "INT.IOI.S3";
                    int_naming = "INT.IOI";
                    ioi_kind = "IOI.S3";
                    ioi_naming = "IOI.S3.T";
                    iobs = &[2, 1, 0];
                }
                GridKind::FpgaCore => {
                    pads = &[3, 7, 2, 6, 1, 5, 0, 4];
                    iobs_kind = Some(("IOBS.FC.T", 1));
                    ipads = &[];
                    int_kind = "INT.IOI.FC";
                    int_naming = "INT.IOI.FC";
                    ioi_kind = "IOI.FC";
                    ioi_naming = "IOI.FC.T";
                    iobs = &[3, 7, 2, 6, 1, 5, 0, 4];
                }
                GridKind::Spartan3E => {
                    (pads, ipads, term, iobs_kind) = match cd.io {
                        ColumnIoKind::Single => {
                            (&[2][..], &[][..], "TTERM1", Some(("IOBS.S3E.T1", 1)))
                        }
                        ColumnIoKind::Double(0) => {
                            (&[1, 0][..], &[][..], "TTERM2", Some(("IOBS.S3E.T2", 2)))
                        }
                        ColumnIoKind::Double(1) => (&[][..], &[2][..], "TTERM", None),
                        ColumnIoKind::Triple(0) => {
                            (&[1, 0][..], &[][..], "TTERM3", Some(("IOBS.S3E.T3", 3)))
                        }
                        ColumnIoKind::Triple(1) => (&[][..], &[2][..], "TTERM", None),
                        ColumnIoKind::Triple(2) => (&[1, 0][..], &[][..], "TTERM", None),
                        ColumnIoKind::Quad(0) => {
                            (&[1, 0][..], &[][..], "TTERM4", Some(("IOBS.S3E.T4", 4)))
                        }
                        ColumnIoKind::Quad(1) => (&[2][..], &[][..], "TTERM", None),
                        ColumnIoKind::Quad(2) => (&[1, 0][..], &[][..], "TTERM", None),
                        ColumnIoKind::Quad(3) => (&[][..], &[1, 0][..], "TTERM", None),
                        _ => unreachable!(),
                    };
                    if cd.io == ColumnIoKind::Quad(0) && cd.kind == ColumnKind::BramCont(2) {
                        term = "TTERM4_BRAM2";
                    }
                    if col == self.grid.col_clk - 2 {
                        term = "TTERMCLK";
                    }
                    if col == self.grid.col_clk - 1 {
                        term = "TTERMCLKA";
                    }
                    if col == self.grid.col_clk {
                        term = "TTERM4CLK";
                    }
                    if col == self.grid.col_clk + 2 {
                        term = "TTERMCLKA";
                    }
                    int_kind = "INT.IOI.S3E";
                    int_naming = "INT.IOI";
                    ioi_kind = "IOI.S3E";
                    ioi_naming = "IOI.S3E.T";
                    iobs = &[2, 1, 0];
                    if ipads.is_empty() {
                        kind = "TIOIS";
                    } else {
                        kind = "TIBUFS";
                    }
                }
                GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                    (pads, ipads, term, iobs_kind) = match cd.io {
                        ColumnIoKind::Double(0) => {
                            (&[0, 1][..], &[2][..], "TTERM2", Some(("IOBS.S3A.T2", 2)))
                        }
                        ColumnIoKind::Double(1) => (&[0, 1][..], &[][..], "TTERM", None),
                        _ => unreachable!(),
                    };
                    int_kind = "INT.IOI.S3A.TB";
                    int_naming = "INT.IOI.S3A.TB";
                    ioi_kind = "IOI.S3A.T";
                    if self.grid.kind == GridKind::Spartan3ADsp {
                        ioi_naming = "IOI.S3ADSP.T";
                    } else {
                        ioi_naming = "IOI.S3A.T";
                    }
                    iobs = &[0, 1, 2];
                    if ipads.is_empty() {
                        kind = "TIOIS";
                    } else {
                        kind = "TIOIB";
                    }
                    if col == self.grid.col_clk - 2 {
                        term = "TTERM2CLK";
                    }
                    if col == self.grid.col_clk - 1 {
                        term = "TTERMCLKA";
                    }
                    if col == self.grid.col_clk {
                        term = "TTERM2CLK";
                    }
                    if col == self.grid.col_clk + 1 {
                        term = "TTERMCLKA";
                    }
                    if self.grid.kind == GridKind::Spartan3ADsp {
                        match cd.kind {
                            ColumnKind::BramCont(2) => {
                                term = "TTERM1";
                            }
                            ColumnKind::Dsp => {
                                term = "TTERM1_MACC";
                            }
                            _ => (),
                        }
                    }
                }
            }
            let name;
            let term_name;
            if self.grid.kind.is_spartan3ea() {
                let x = self.xlut[col];
                let y = row.to_idx();
                name = format!("{kind}_X{x}Y{y}");
                term_name = format!("{term}_X{x}Y{y}");
            } else {
                let c = self.clut[col];
                name = format!("TIOIC{c}");
                term_name = format!("TTERMC{c}");
            }
            self.die
                .fill_tile((col, row), int_kind, int_naming, name.clone());
            self.grid
                .fill_term(&mut self.die, (col, row), "TERM.N", "TERM.N", term_name);
            let node = self.die.add_xnode(
                (col, row),
                self.db.get_node(ioi_kind),
                &[&name],
                self.db.get_node_naming(ioi_naming),
                &[(col, row)],
            );
            for &i in iobs {
                if pads.contains(&i) {
                    self.bonded_ios.push(IoCoord {
                        col,
                        row,
                        iob: TileIobId::from_idx(i),
                    });
                    match (ioi_kind, i) {
                        ("IOI.CLK_T", 0) => node.add_bel(i, "CLKPPAD1".to_string()),
                        ("IOI.CLK_T", 1) => node.add_bel(i, "CLKNPAD1".to_string()),
                        _ => node.add_bel(i, format!("PAD{idx}", idx = self.ctr_pad)),
                    }
                    self.ctr_pad += 1;
                } else if ipads.contains(&i) {
                    self.bonded_ios.push(IoCoord {
                        col,
                        row,
                        iob: TileIobId::from_idx(i),
                    });
                    node.add_bel(i, format!("IPAD{idx}", idx = self.ctr_pad));
                    self.ctr_pad += 1;
                } else {
                    node.add_bel(i, format!("NOPAD{idx}", idx = self.ctr_nopad));
                    self.ctr_nopad += 1;
                }
            }
            if let Some((kind, num)) = iobs_kind {
                let coords: Vec<_> = (0..num).map(|dx| (col + dx, row)).collect();
                self.die.add_xnode(
                    (col, row),
                    self.db.get_node(kind),
                    &[&name],
                    self.db.get_node_naming(kind),
                    &coords,
                );
            }
            if !self.grid.kind.is_virtex2() {
                let node = self.die.add_xnode(
                    (col, row),
                    self.db.get_node("RANDOR"),
                    &[&name],
                    self.db.get_node_naming("RANDOR.T"),
                    &[],
                );
                let x = if self.grid.kind == GridKind::Spartan3 {
                    (self.clut[col] - 1) * 2
                } else {
                    col.to_idx() - 1
                };
                if self.grid.kind == GridKind::FpgaCore {
                    node.add_bel(0, format!("RANDOR_X{x}Y0"));
                } else {
                    node.add_bel(0, format!("RANDOR_X{x}Y1"));
                }
            }
        }
    }

    fn fill_io_r(&mut self) {
        for (row, &rd) in self.grid.rows.iter().rev() {
            let col = self.grid.col_right();
            if row == self.grid.row_bot() || row == self.grid.row_top() {
                continue;
            }
            let pads: &[usize];
            let ipads: &[usize];
            let int_kind;
            let int_naming;
            let ioi_kind;
            let ioi_naming;
            let iobs_kind;
            let iobs: &[usize];
            let mut term = "";
            let mut term_kind = "TERM.E";
            match self.grid.kind {
                GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
                    (pads, iobs_kind) = match rd {
                        RowIoKind::DoubleBot(0) => (
                            &[3, 2, 1, 0][..],
                            Some((
                                if self.grid.kind.is_virtex2p() {
                                    "IOBS.V2P.R.B2"
                                } else {
                                    "IOBS.V2.R.B2"
                                },
                                2,
                            )),
                        ),
                        RowIoKind::DoubleBot(1) => (&[3, 2][..], None),
                        RowIoKind::DoubleTop(0) => (
                            &[1, 0][..],
                            Some((
                                if self.grid.kind.is_virtex2p() {
                                    "IOBS.V2P.R.T2"
                                } else {
                                    "IOBS.V2.R.T2"
                                },
                                2,
                            )),
                        ),
                        RowIoKind::DoubleTop(1) => (&[3, 2, 1, 0][..], None),
                        _ => unreachable!(),
                    };
                    ipads = &[];
                    int_kind = "INT.IOI";
                    int_naming = "INT.IOI.LR";
                    ioi_kind = "IOI";
                    ioi_naming = "IOI";
                    iobs = &[3, 2, 1, 0];
                    term_kind = if row < self.grid.row_pci.unwrap() {
                        "TERM.E.D"
                    } else {
                        "TERM.E.U"
                    };
                }
                GridKind::Spartan3 => {
                    pads = &[1, 0];
                    ipads = &[];
                    iobs_kind = Some(("IOBS.S3.R1", 1));
                    int_kind = "INT.IOI.S3";
                    int_naming = "INT.IOI";
                    ioi_kind = "IOI.S3";
                    ioi_naming = "IOI.S3.R";
                    iobs = &[2, 1, 0];
                }
                GridKind::FpgaCore => {
                    pads = &[3, 7, 2, 6, 1, 5, 0, 4];
                    iobs_kind = Some(("IOBS.FC.R", 1));
                    ipads = &[];
                    int_kind = "INT.IOI.FC";
                    int_naming = "INT.IOI.FC";
                    ioi_kind = "IOI.FC";
                    ioi_naming = "IOI.FC.R";
                    iobs = &[3, 7, 2, 6, 1, 5, 0, 4];
                }
                GridKind::Spartan3E => {
                    (pads, ipads, term, iobs_kind) = match rd {
                        RowIoKind::Single => {
                            (&[2][..], &[][..], "RTERM1", Some(("IOBS.S3E.R1", 1)))
                        }
                        RowIoKind::Double(0) => {
                            (&[1, 0][..], &[][..], "RTERM2", Some(("IOBS.S3E.R2", 2)))
                        }
                        RowIoKind::Double(1) => (&[][..], &[][..], "RTERM", None),
                        RowIoKind::Triple(0) => {
                            (&[1, 0][..], &[][..], "RTERM3", Some(("IOBS.S3E.R3", 3)))
                        }
                        RowIoKind::Triple(1) => (&[2][..], &[][..], "RTERM", None),
                        RowIoKind::Triple(2) => (&[][..], &[2][..], "RTERM", None),
                        RowIoKind::Quad(0) => {
                            (&[1, 0][..], &[][..], "RTERM4", Some(("IOBS.S3E.R4", 4)))
                        }
                        RowIoKind::Quad(1) => (&[][..], &[][..], "RTERM", None),
                        RowIoKind::Quad(2) => (&[1, 0][..], &[][..], "RTERM", None),
                        RowIoKind::Quad(3) => (&[][..], &[2][..], "RTERM", None),
                        _ => unreachable!(),
                    };
                    if row == self.grid.row_mid() {
                        term = "RTERM4CLK";
                    }
                    if row == self.grid.row_mid() - 4 {
                        term = "RTERM4CLKB";
                    }
                    if row == self.grid.row_mid() - 2 {
                        term = "RTERMCLKA";
                    }
                    if row == self.grid.row_mid() + 2 {
                        term = "RTERMCLKA";
                    }
                    int_kind = "INT.IOI.S3E";
                    if self.rows_brk.contains(&row) {
                        int_naming = "INT.IOI.BRK";
                    } else {
                        int_naming = "INT.IOI";
                    }
                    ioi_kind = "IOI.S3E";
                    if row >= self.grid.row_mid() - 4 && row < self.grid.row_mid() + 4 {
                        if ipads.is_empty() {
                            ioi_naming = "IOI.S3E.R.PCI.PCI";
                        } else {
                            ioi_naming = "IOI.S3E.R.PCI";
                        }
                    } else {
                        ioi_naming = "IOI.S3E.R";
                    }
                    iobs = &[2, 1, 0];
                }
                GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                    (pads, ipads, term, iobs_kind) = match rd {
                        RowIoKind::Quad(0) => {
                            (&[1, 0][..], &[][..], "RTERM4", Some(("IOBS.S3A.R4", 4)))
                        }
                        RowIoKind::Quad(1) => (&[1, 0][..], &[][..], "RTERM", None),
                        RowIoKind::Quad(2) => (&[1, 0][..], &[][..], "RTERM", None),
                        RowIoKind::Quad(3) => (&[][..], &[1, 0][..], "RTERM", None),
                        _ => unreachable!(),
                    };
                    if row == self.grid.row_mid() {
                        term = "RTERM4CLK";
                    }
                    if row == self.grid.row_mid() - 4 {
                        term = "RTERM4B";
                    }
                    if row == self.grid.row_mid() - 3 {
                        term = "RTERMCLKB";
                    }
                    if row == self.grid.row_mid() - 2 {
                        term = "RTERMCLKA";
                    }
                    if row == self.grid.row_mid() + 1 {
                        term = "RTERMCLKA";
                    }
                    int_kind = "INT.IOI.S3A.LR";
                    if self.rows_brk.contains(&row) {
                        int_naming = "INT.IOI.S3A.LR.BRK";
                    } else {
                        int_naming = "INT.IOI.S3A.LR";
                    }
                    ioi_kind = "IOI.S3A.LR";
                    if self.grid.kind == GridKind::Spartan3ADsp {
                        if row >= self.grid.row_mid() - 4
                            && row < self.grid.row_mid() + 4
                            && ipads.is_empty()
                        {
                            ioi_naming = "IOI.S3ADSP.R.PCI";
                        } else {
                            ioi_naming = "IOI.S3ADSP.R";
                        }
                    } else {
                        if row >= self.grid.row_mid() - 4
                            && row < self.grid.row_mid() + 4
                            && ipads.is_empty()
                        {
                            ioi_naming = "IOI.S3A.R.PCI";
                        } else {
                            ioi_naming = "IOI.S3A.R";
                        }
                    }
                    iobs = &[1, 0];
                }
            }
            let name;
            let term_name;
            if self.grid.kind.is_spartan3ea() {
                let x = self.xlut[col];
                let y = row.to_idx();
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
                let kind = if ipads.is_empty() { "RIOIS" } else { "RIBUFS" };
                name = format!("{kind}{clk}{pci}{brk}_X{x}Y{y}");
                term_name = format!("{term}_X{x}Y{y}");
            } else {
                let r = self.rlut[row];
                name = format!("RIOIR{r}");
                term_name = format!("RTERMR{r}");
            }
            self.die
                .fill_tile((col, row), int_kind, int_naming, name.clone());
            self.grid
                .fill_term(&mut self.die, (col, row), "TERM.E", term_kind, term_name);
            let node = self.die.add_xnode(
                (col, row),
                self.db.get_node(ioi_kind),
                &[&name],
                self.db.get_node_naming(ioi_naming),
                &[(col, row)],
            );
            for &i in iobs {
                if pads.contains(&i) {
                    self.bonded_ios.push(IoCoord {
                        col,
                        row,
                        iob: TileIobId::from_idx(i),
                    });
                    node.add_bel(i, format!("PAD{idx}", idx = self.ctr_pad));
                    self.ctr_pad += 1;
                } else if ipads.contains(&i) {
                    self.bonded_ios.push(IoCoord {
                        col,
                        row,
                        iob: TileIobId::from_idx(i),
                    });
                    node.add_bel(i, format!("IPAD{idx}", idx = self.ctr_pad));
                    self.ctr_pad += 1;
                } else {
                    node.add_bel(i, format!("NOPAD{idx}", idx = self.ctr_nopad));
                    self.ctr_nopad += 1;
                }
            }
            if let Some((kind, num)) = iobs_kind {
                let coords: Vec<_> = (0..num).map(|dx| (col, row + dx)).collect();
                self.die.add_xnode(
                    (col, row),
                    self.db.get_node(kind),
                    &[&name],
                    self.db.get_node_naming(kind),
                    &coords,
                );
            }
        }
    }

    fn fill_io_b(&mut self) {
        for (col, &cd) in self.grid.columns.iter().rev() {
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
            let pads: &[usize];
            let ipads: &[usize];
            let mut int_kind;
            let mut int_naming;
            let mut ioi_kind;
            let mut ioi_naming;
            let mut iobs_kind;
            let iobs: &[usize];
            let mut term = "";
            let mut kind = "";
            match self.grid.kind {
                GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
                    (pads, iobs_kind) = match cd.io {
                        ColumnIoKind::None => (&[][..], None),
                        ColumnIoKind::SingleLeft => (&[3, 2, 1][..], Some(("IOBS.V2P.B.L1", 1))),
                        ColumnIoKind::SingleLeftAlt => {
                            (&[3, 2, 1][..], Some(("IOBS.V2P.B.L1.ALT", 1)))
                        }
                        ColumnIoKind::SingleRight => (&[2, 1, 0][..], Some(("IOBS.V2P.B.R1", 1))),
                        ColumnIoKind::SingleRightAlt => {
                            (&[2, 1, 0][..], Some(("IOBS.V2P.B.R1.ALT", 1)))
                        }
                        ColumnIoKind::DoubleLeft(0) => (
                            &[3, 2, 1, 0][..],
                            Some((
                                if self.grid.kind.is_virtex2p() {
                                    "IOBS.V2P.B.L2"
                                } else {
                                    "IOBS.V2.B.L2"
                                },
                                2,
                            )),
                        ),
                        ColumnIoKind::DoubleRight(0) => (
                            &[1, 0][..],
                            Some((
                                if self.grid.kind.is_virtex2p() {
                                    "IOBS.V2P.B.R2"
                                } else {
                                    "IOBS.V2.B.R2"
                                },
                                2,
                            )),
                        ),
                        ColumnIoKind::DoubleLeft(1) => (&[3, 2][..], None),
                        ColumnIoKind::DoubleRight(1) => (&[3, 2, 1, 0][..], None),
                        _ => unreachable!(),
                    };
                    ipads = &[];
                    int_kind = "INT.IOI";
                    int_naming = "INT.IOI.TB";
                    ioi_kind = "IOI";
                    ioi_naming = "IOI";
                    if matches!(
                        cd.io,
                        ColumnIoKind::SingleLeftAlt | ColumnIoKind::SingleRightAlt
                    ) {
                        ioi_naming = "IOI.TBS";
                    }
                    if self.grid.kind == GridKind::Virtex2PX && col == self.grid.col_clk - 1 {
                        ioi_kind = "IOI.CLK_B";
                        ioi_naming = "IOI.CLK_B";
                        int_kind = "INT.IOI.CLK_B";
                        int_naming = "INT.IOI.CLK_B";
                    }
                    if self.grid.kind == GridKind::Virtex2PX && col == self.grid.col_clk - 2 {
                        iobs_kind = Some(("IOBS.V2P.B.R2.CLK", 2));
                    }
                    iobs = &[3, 2, 1, 0];
                }
                GridKind::Spartan3 => {
                    (pads, iobs_kind) = match cd.io {
                        ColumnIoKind::Double(0) => (&[1, 0][..], Some(("IOBS.S3.B2", 2))),
                        ColumnIoKind::Double(1) => (&[2, 1, 0][..], None),
                        _ => unreachable!(),
                    };
                    ipads = &[];
                    int_kind = "INT.IOI.S3";
                    int_naming = "INT.IOI";
                    ioi_kind = "IOI.S3";
                    ioi_naming = "IOI.S3.B";
                    iobs = &[2, 1, 0];
                }
                GridKind::FpgaCore => {
                    pads = &[3, 7, 2, 6, 1, 5, 0, 4];
                    iobs_kind = Some(("IOBS.FC.B", 1));
                    ipads = &[];
                    int_kind = "INT.IOI.FC";
                    int_naming = "INT.IOI.FC";
                    ioi_kind = "IOI.FC";
                    ioi_naming = "IOI.FC.B";
                    iobs = &[3, 7, 2, 6, 1, 5, 0, 4];
                }
                GridKind::Spartan3E => {
                    (pads, ipads, term, iobs_kind) = match cd.io {
                        ColumnIoKind::Single => {
                            (&[2][..], &[][..], "BTERM1", Some(("IOBS.S3E.B1", 1)))
                        }
                        ColumnIoKind::Double(0) => {
                            (&[][..], &[2][..], "BTERM2", Some(("IOBS.S3E.B2", 2)))
                        }
                        ColumnIoKind::Double(1) => (&[1, 0][..], &[][..], "BTERM", None),
                        ColumnIoKind::Triple(0) => {
                            (&[1, 0][..], &[][..], "BTERM3", Some(("IOBS.S3E.B3", 3)))
                        }
                        ColumnIoKind::Triple(1) => (&[][..], &[2][..], "BTERM", None),
                        ColumnIoKind::Triple(2) => (&[1, 0][..], &[][..], "BTERM", None),
                        ColumnIoKind::Quad(0) => {
                            (&[][..], &[1, 0][..], "BTERM4", Some(("IOBS.S3E.B4", 4)))
                        }
                        ColumnIoKind::Quad(1) => (&[1, 0][..], &[][..], "BTERM", None),
                        ColumnIoKind::Quad(2) => (&[2][..], &[][..], "BTERM", None),
                        ColumnIoKind::Quad(3) => (&[1, 0][..], &[][..], "BTERM", None),
                        _ => unreachable!(),
                    };
                    if cd.io == ColumnIoKind::Quad(0) && cd.kind == ColumnKind::BramCont(2) {
                        term = "BTERM4_BRAM2";
                    }
                    if col == self.grid.col_clk - 3 {
                        term = "BTERMCLKA";
                    }
                    if col == self.grid.col_clk - 1 {
                        term = "BTERMCLKB";
                    }
                    if col == self.grid.col_clk {
                        term = "BTERM4CLK";
                    }
                    if col == self.grid.col_clk + 1 {
                        term = "BTERMCLK";
                    }
                    int_kind = "INT.IOI.S3E";
                    int_naming = "INT.IOI";
                    ioi_kind = "IOI.S3E";
                    ioi_naming = "IOI.S3E.B";
                    iobs = &[2, 1, 0];
                    if ipads.is_empty() {
                        kind = "BIOIS";
                    } else {
                        kind = "BIBUFS";
                    }
                }
                GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                    (pads, ipads, term, iobs_kind) = match cd.io {
                        ColumnIoKind::Double(0) => {
                            (&[1, 0][..], &[2][..], "BTERM2", Some(("IOBS.S3A.B2", 2)))
                        }
                        ColumnIoKind::Double(1) => (&[1, 0][..], &[][..], "BTERM", None),
                        _ => unreachable!(),
                    };
                    int_kind = "INT.IOI.S3A.TB";
                    int_naming = "INT.IOI.S3A.TB";
                    ioi_kind = "IOI.S3A.B";
                    if self.grid.kind == GridKind::Spartan3ADsp {
                        ioi_naming = "IOI.S3ADSP.B";
                    } else {
                        ioi_naming = "IOI.S3A.B";
                    }
                    iobs = &[2, 1, 0];
                    if ipads.is_empty() {
                        kind = "BIOIS";
                    } else {
                        kind = "BIOIB";
                    }
                    if col == self.grid.col_clk - 2 {
                        term = "BTERM2CLK";
                    }
                    if col == self.grid.col_clk - 1 {
                        term = "BTERMCLKB";
                    }
                    if col == self.grid.col_clk {
                        term = "BTERM2CLK";
                    }
                    if col == self.grid.col_clk + 1 {
                        term = "BTERMCLK";
                    }
                    if self.grid.kind == GridKind::Spartan3ADsp {
                        match cd.kind {
                            ColumnKind::BramCont(2) => {
                                term = "BTERM1";
                            }
                            ColumnKind::Dsp => {
                                term = "BTERM1_MACC";
                            }
                            _ => (),
                        }
                    }
                }
            }
            let name;
            let term_name;
            if self.grid.kind.is_spartan3ea() {
                let x = self.xlut[col];
                let y = row.to_idx();
                name = format!("{kind}_X{x}Y{y}");
                term_name = format!("{term}_X{x}Y{y}");
            } else {
                let c = self.clut[col];
                name = format!("BIOIC{c}");
                term_name = format!("BTERMC{c}");
            }
            self.die
                .fill_tile((col, row), int_kind, int_naming, name.clone());
            self.grid
                .fill_term(&mut self.die, (col, row), "TERM.S", "TERM.S", term_name);
            let node = self.die.add_xnode(
                (col, row),
                self.db.get_node(ioi_kind),
                &[&name],
                self.db.get_node_naming(ioi_naming),
                &[(col, row)],
            );
            for &i in iobs {
                if pads.contains(&i) {
                    self.bonded_ios.push(IoCoord {
                        col,
                        row,
                        iob: TileIobId::from_idx(i),
                    });
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
                    match (ioi_kind, i) {
                        ("IOI.CLK_B", 2) => name = "CLKPPAD2".to_string(),
                        ("IOI.CLK_B", 3) => name = "CLKNPAD2".to_string(),
                        _ => (),
                    }
                    node.add_bel(i, name);
                    self.ctr_pad += 1;
                } else if ipads.contains(&i) {
                    self.bonded_ios.push(IoCoord {
                        col,
                        row,
                        iob: TileIobId::from_idx(i),
                    });
                    let mut name = format!("IPAD{idx}", idx = self.ctr_pad);
                    if self.grid.kind == GridKind::Spartan3A
                        && self.grid.cols_clkv.is_none()
                        && self.ctr_pad == 95
                    {
                        name = "IPAD94".to_string();
                    }
                    node.add_bel(i, name);
                    self.ctr_pad += 1;
                } else {
                    node.add_bel(i, format!("NOPAD{idx}", idx = self.ctr_nopad));
                    self.ctr_nopad += 1;
                }
            }
            if let Some((kind, num)) = iobs_kind {
                let coords: Vec<_> = (0..num).map(|dx| (col + dx, row)).collect();
                self.die.add_xnode(
                    (col, row),
                    self.db.get_node(kind),
                    &[&name],
                    self.db.get_node_naming(kind),
                    &coords,
                );
            }
            if !self.grid.kind.is_virtex2() && self.grid.kind != GridKind::FpgaCore {
                let node = self.die.add_xnode(
                    (col, row),
                    self.db.get_node("RANDOR"),
                    &[&name],
                    self.db.get_node_naming("RANDOR.B"),
                    &[(col, row)],
                );
                let x = if self.grid.kind == GridKind::Spartan3 {
                    (self.clut[col] - 1) * 2
                } else {
                    col.to_idx() - 1
                };
                node.add_bel(0, format!("RANDOR_X{x}Y0"));
            }
        }
    }

    fn fill_io_l(&mut self) {
        for (row, &rd) in self.grid.rows.iter() {
            let col = self.grid.col_left();
            if row == self.grid.row_bot() || row == self.grid.row_top() {
                continue;
            }
            let pads: &[usize];
            let ipads: &[usize];
            let int_kind;
            let int_naming;
            let ioi_kind;
            let ioi_naming;
            let iobs_kind;
            let iobs: &[usize];
            let mut term = "";
            let mut term_kind = "TERM.W";
            match self.grid.kind {
                GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
                    (pads, iobs_kind) = match rd {
                        RowIoKind::DoubleBot(0) => (
                            &[0, 1, 2, 3][..],
                            Some((
                                if self.grid.kind.is_virtex2p() {
                                    "IOBS.V2P.L.B2"
                                } else {
                                    "IOBS.V2.L.B2"
                                },
                                2,
                            )),
                        ),
                        RowIoKind::DoubleBot(1) => (&[2, 3][..], None),
                        RowIoKind::DoubleTop(0) => (
                            &[0, 1][..],
                            Some((
                                if self.grid.kind.is_virtex2p() {
                                    "IOBS.V2P.L.T2"
                                } else {
                                    "IOBS.V2.L.T2"
                                },
                                2,
                            )),
                        ),
                        RowIoKind::DoubleTop(1) => (&[0, 1, 2, 3][..], None),
                        _ => unreachable!(),
                    };
                    ipads = &[];
                    int_kind = "INT.IOI";
                    int_naming = "INT.IOI.LR";
                    ioi_kind = "IOI";
                    ioi_naming = "IOI";
                    iobs = &[0, 1, 2, 3];
                    term_kind = if row < self.grid.row_pci.unwrap() {
                        "TERM.W.D"
                    } else {
                        "TERM.W.U"
                    };
                }
                GridKind::Spartan3 => {
                    pads = &[0, 1];
                    ipads = &[];
                    iobs_kind = Some(("IOBS.S3.L1", 1));
                    int_kind = "INT.IOI.S3";
                    int_naming = "INT.IOI";
                    ioi_kind = "IOI.S3";
                    ioi_naming = "IOI.S3.L";
                    iobs = &[0, 1, 2];
                }
                GridKind::FpgaCore => {
                    pads = &[0, 4, 1, 5, 2, 6, 3, 7];
                    iobs_kind = Some(("IOBS.FC.L", 1));
                    ipads = &[];
                    int_kind = "INT.IOI.FC";
                    int_naming = "INT.IOI.FC";
                    ioi_kind = "IOI.FC";
                    ioi_naming = "IOI.FC.L";
                    iobs = &[0, 4, 1, 5, 2, 6, 3, 7];
                }
                GridKind::Spartan3E => {
                    (pads, ipads, term, iobs_kind) = match rd {
                        RowIoKind::Single => {
                            (&[2][..], &[][..], "LTERM1", Some(("IOBS.S3E.L1", 1)))
                        }
                        RowIoKind::Double(0) => {
                            (&[][..], &[][..], "LTERM2", Some(("IOBS.S3E.L2", 2)))
                        }
                        RowIoKind::Double(1) => (&[1, 0][..], &[][..], "LTERM", None),
                        RowIoKind::Triple(0) => {
                            (&[][..], &[2][..], "LTERM3", Some(("IOBS.S3E.L3", 3)))
                        }
                        RowIoKind::Triple(1) => (&[2][..], &[][..], "LTERM", None),
                        RowIoKind::Triple(2) => (&[1, 0][..], &[][..], "LTERM", None),
                        RowIoKind::Quad(0) => {
                            (&[][..], &[2][..], "LTERM4", Some(("IOBS.S3E.L4", 4)))
                        }
                        RowIoKind::Quad(1) => (&[1, 0][..], &[][..], "LTERM", None),
                        RowIoKind::Quad(2) => (&[][..], &[][..], "LTERM", None),
                        RowIoKind::Quad(3) => (&[1, 0][..], &[][..], "LTERM", None),
                        _ => unreachable!(),
                    };
                    if row == self.grid.row_mid() {
                        term = "LTERM4CLK";
                    }
                    if row == self.grid.row_mid() - 4 {
                        term = "LTERM4B";
                    }
                    if row == self.grid.row_mid() - 3 {
                        term = "LTERMCLKA";
                    }
                    if row == self.grid.row_mid() - 1 {
                        term = "LTERMCLK";
                    }
                    if row == self.grid.row_mid() + 1 {
                        term = "LTERMCLKA";
                    }
                    if row == self.grid.row_mid() + 3 {
                        term = "LTERMCLK";
                    }
                    int_kind = "INT.IOI.S3E";
                    if self.rows_brk.contains(&row) {
                        int_naming = "INT.IOI.BRK";
                    } else {
                        int_naming = "INT.IOI";
                    }
                    ioi_kind = "IOI.S3E";
                    if row >= self.grid.row_mid() - 4 && row < self.grid.row_mid() + 4 {
                        if ipads.is_empty() {
                            ioi_naming = "IOI.S3E.L.PCI.PCI";
                        } else {
                            ioi_naming = "IOI.S3E.L.PCI";
                        }
                    } else {
                        ioi_naming = "IOI.S3E.L";
                    }
                    iobs = &[2, 1, 0];
                }
                GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                    (pads, ipads, term, iobs_kind) = match rd {
                        RowIoKind::Quad(0) => {
                            (&[][..], &[0, 1][..], "LTERM4", Some(("IOBS.S3A.L4", 4)))
                        }
                        RowIoKind::Quad(1) => (&[0, 1][..], &[][..], "LTERM", None),
                        RowIoKind::Quad(2) => (&[0, 1][..], &[][..], "LTERM", None),
                        RowIoKind::Quad(3) => (&[0, 1][..], &[][..], "LTERM", None),
                        _ => unreachable!(),
                    };
                    if row == self.grid.row_mid() {
                        term = "LTERM4CLK";
                    }
                    if row == self.grid.row_mid() - 4 {
                        term = "LTERM4B";
                    }
                    if row == self.grid.row_mid() - 2 {
                        term = "LTERMCLKA";
                    }
                    if row == self.grid.row_mid() - 1 {
                        term = "LTERMCLK";
                    }
                    if row == self.grid.row_mid() + 1 {
                        term = "LTERMCLKA";
                    }
                    if row == self.grid.row_mid() + 2 {
                        term = "LTERMCLK";
                    }
                    int_kind = "INT.IOI.S3A.LR";
                    if self.rows_brk.contains(&row) {
                        int_naming = "INT.IOI.S3A.LR.BRK";
                    } else {
                        int_naming = "INT.IOI.S3A.LR";
                    }
                    ioi_kind = "IOI.S3A.LR";
                    if self.grid.kind == GridKind::Spartan3ADsp {
                        if row >= self.grid.row_mid() - 4
                            && row < self.grid.row_mid() + 4
                            && ipads.is_empty()
                        {
                            ioi_naming = "IOI.S3ADSP.L.PCI";
                        } else {
                            ioi_naming = "IOI.S3ADSP.L";
                        }
                    } else {
                        if row >= self.grid.row_mid() - 4
                            && row < self.grid.row_mid() + 4
                            && ipads.is_empty()
                        {
                            ioi_naming = "IOI.S3A.L.PCI";
                        } else {
                            ioi_naming = "IOI.S3A.L";
                        }
                    }
                    iobs = &[0, 1];
                }
            }
            let name;
            let term_name;
            if self.grid.kind.is_spartan3ea() {
                let x = self.xlut[col];
                let y = row.to_idx();
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
                let kind = if ipads.is_empty() { "LIOIS" } else { "LIBUFS" };
                name = format!("{kind}{clk}{pci}{brk}_X{x}Y{y}");
                term_name = format!("{term}_X{x}Y{y}");
            } else {
                let r = self.rlut[row];
                name = format!("LIOIR{r}");
                term_name = format!("LTERMR{r}");
            }
            self.die
                .fill_tile((col, row), int_kind, int_naming, name.clone());
            self.grid
                .fill_term(&mut self.die, (col, row), "TERM.W", term_kind, term_name);
            let node = self.die.add_xnode(
                (col, row),
                self.db.get_node(ioi_kind),
                &[&name],
                self.db.get_node_naming(ioi_naming),
                &[(col, row)],
            );
            for &i in iobs {
                if pads.contains(&i) {
                    self.bonded_ios.push(IoCoord {
                        col,
                        row,
                        iob: TileIobId::from_idx(i),
                    });
                    node.add_bel(i, format!("PAD{idx}", idx = self.ctr_pad));
                    self.ctr_pad += 1;
                } else if ipads.contains(&i) {
                    self.bonded_ios.push(IoCoord {
                        col,
                        row,
                        iob: TileIobId::from_idx(i),
                    });
                    node.add_bel(i, format!("IPAD{idx}", idx = self.ctr_pad));
                    self.ctr_pad += 1;
                } else {
                    node.add_bel(i, format!("NOPAD{idx}", idx = self.ctr_nopad));
                    self.ctr_nopad += 1;
                }
            }
            if let Some((kind, num)) = iobs_kind {
                let coords: Vec<_> = (0..num).map(|dx| (col, row + dx)).collect();
                self.die.add_xnode(
                    (col, row),
                    self.db.get_node(kind),
                    &[&name],
                    self.db.get_node_naming(kind),
                    &coords,
                );
            }
        }
    }

    fn fill_clb(&mut self) {
        let mut cx = 0;
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
                let tile = if self.grid.kind.is_spartan3ea() {
                    let x = self.xlut[col];
                    let y = row.to_idx();
                    format!("CLB_X{x}Y{y}")
                } else {
                    let c = self.clut[col];
                    let r = self.rlut[row];
                    format!("R{r}C{c}")
                };
                let naming = if self.grid.kind.is_spartan3ea() && self.rows_brk.contains(&row) {
                    "INT.CLB.BRK"
                } else {
                    "INT.CLB"
                };
                self.die
                    .fill_tile((col, row), "INT.CLB", naming, tile.clone());
                let node = self.die.add_xnode(
                    (col, row),
                    self.db.get_node("CLB"),
                    &[&tile],
                    self.db.get_node_naming("CLB"),
                    &[(col, row)],
                );
                let sx = 2 * cx;
                let sy = 2 * (row.to_idx() - 1);
                if self.grid.kind.is_virtex2() {
                    node.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                    node.add_bel(1, format!("SLICE_X{x}Y{y}", x = sx, y = sy + 1));
                    node.add_bel(2, format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy));
                    node.add_bel(3, format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy + 1));
                    if cx % 2 == 0 {
                        node.add_bel(4, format!("TBUF_X{sx}Y{sy}"));
                        node.add_bel(5, format!("TBUF_X{x}Y{y}", x = sx, y = sy + 1));
                    } else {
                        node.add_bel(4, format!("TBUF_X{x}Y{y}", x = sx, y = sy + 1));
                        node.add_bel(5, format!("TBUF_X{sx}Y{sy}"));
                    }
                } else {
                    node.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                    node.add_bel(1, format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy));
                    node.add_bel(2, format!("SLICE_X{x}Y{y}", x = sx, y = sy + 1));
                    node.add_bel(3, format!("SLICE_X{x}Y{y}", x = sx + 1, y = sy + 1));
                }
            }
            cx += 1;
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
        let mut sx = 0;
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
                    let x = self.xlut[col + d];
                    let yb = b.to_idx();
                    let yt = t.to_idx();
                    self.die.fill_term_pair_bounce(
                        (col + d, b - 1),
                        (col + d, t + 1),
                        self.db.get_term("TERM.BRAM.N"),
                        self.db.get_term("TERM.BRAM.S"),
                        format!("COB_TERM_B_X{x}Y{yb}"),
                        format!("COB_TERM_T_X{x}Y{yt}"),
                        self.db.get_term_naming("TERM.BRAM.N"),
                        self.db.get_term_naming("TERM.BRAM.S"),
                    );
                }
            }
            let mut i = 0;
            for (row, &io) in self.grid.rows.iter() {
                if io == RowIoKind::None {
                    continue;
                }
                if let Some((b, t)) = self.grid.rows_ram {
                    if row <= b || row >= t {
                        continue;
                    }
                } else if self.is_hole(col, row) {
                    continue;
                }
                let naming = match self.grid.kind {
                    GridKind::Virtex2
                    | GridKind::Virtex2P
                    | GridKind::Virtex2PX
                    | GridKind::Spartan3 => "INT.BRAM",
                    GridKind::FpgaCore => unreachable!(),
                    GridKind::Spartan3E | GridKind::Spartan3A => {
                        if self.rows_brk.contains(&row) {
                            "INT.BRAM.BRK"
                        } else {
                            "INT.BRAM"
                        }
                    }
                    GridKind::Spartan3ADsp => {
                        if self.rows_brk.contains(&row) {
                            "INT.BRAM.S3ADSP.BRK"
                        } else {
                            "INT.BRAM.S3ADSP"
                        }
                    }
                };
                if self.grid.kind.is_spartan3ea() {
                    let x = self.xlut[col];
                    let y = row.to_idx();
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
                    self.die.fill_tile(
                        (col, row),
                        bram_kind[i],
                        naming,
                        format!("BRAM{i}_SMALL{md}_X{x}Y{y}"),
                    );
                    if self.grid.kind == GridKind::Spartan3ADsp {
                        let naming_macc = if self.rows_brk.contains(&row) {
                            "INT.MACC.BRK"
                        } else {
                            "INT.MACC"
                        };
                        let x = self.xlut[col + 3];
                        self.die.fill_tile(
                            (col + 3, row),
                            "INT.BRAM.S3ADSP",
                            naming_macc,
                            format!("MACC{i}_SMALL{md}_X{x}Y{y}"),
                        );
                    }
                } else {
                    let c = self.bramclut[col];
                    let r = self.rlut[row];
                    self.die
                        .fill_tile((col, row), bram_kind[i], naming, format!("BRAMR{r}C{c}"));
                }
                if i == 0 {
                    let is_bot =
                        matches!(self.grid.kind, GridKind::Spartan3A | GridKind::Spartan3ADsp)
                            && row == self.grid.row_bot() + 1;
                    let is_top =
                        matches!(self.grid.kind, GridKind::Spartan3A | GridKind::Spartan3ADsp)
                            && (row == self.grid.row_top() - 4
                                || row == self.grid.row_top() - 8 && col == self.grid.col_clk);
                    let is_brk = self.rows_brk.contains(&(row + 3));
                    let kind = match self.grid.kind {
                        GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => "BRAM",
                        GridKind::Spartan3 => "BRAM.S3",
                        GridKind::FpgaCore => unreachable!(),
                        GridKind::Spartan3E => "BRAM.S3E",
                        GridKind::Spartan3A => "BRAM.S3A",
                        GridKind::Spartan3ADsp => "BRAM.S3ADSP",
                    };
                    let naming = if self.grid.kind == GridKind::Spartan3A {
                        if is_bot {
                            "BRAM.S3A.BOT"
                        } else if is_top {
                            "BRAM.S3A.TOP"
                        } else {
                            "BRAM.S3A"
                        }
                    } else {
                        kind
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
                    let node = self.die.add_xnode(
                        (col, row),
                        self.db.get_node(kind),
                        &[&name],
                        self.db.get_node_naming(naming),
                        &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                    );
                    let mut sy = (row.to_idx() - 1) / 4;
                    if let Some((b, _)) = self.grid.rows_ram {
                        sy = (row.to_idx() - b.to_idx() - 1) / 4;
                    }
                    if self.grid.kind == GridKind::Spartan3A
                        && self.grid.dcms == Some(Dcms::Eight)
                        && row >= self.grid.row_mid()
                    {
                        sy -= 2;
                    }
                    node.add_bel(0, format!("RAMB16_X{sx}Y{sy}"));
                    if self.grid.kind != GridKind::Spartan3ADsp {
                        node.add_bel(1, format!("MULT18X18_X{sx}Y{sy}"));
                    } else {
                        let naming = if is_top { "DSP.TOP" } else { "DSP" };
                        let x = self.xlut[col] + 4;
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
                        let node = self.die.add_xnode(
                            (col + 3, row),
                            self.db.get_node("DSP"),
                            &[&name],
                            self.db.get_node_naming(naming),
                            &[
                                (col + 3, row),
                                (col + 3, row + 1),
                                (col + 3, row + 2),
                                (col + 3, row + 3),
                            ],
                        );
                        node.add_bel(0, format!("DSP48A_X{sx}Y{sy}"));
                        self.die.add_xnode(
                            (col + 3, row),
                            self.db.get_node("INTF.DSP"),
                            &[&name],
                            self.db.get_node_naming("INTF.DSP"),
                            &[
                                (col + 3, row),
                                (col + 3, row + 1),
                                (col + 3, row + 2),
                                (col + 3, row + 3),
                            ],
                        );
                    }
                }
                i += 1;
                i %= 4;
            }
            sx += 1;
        }
    }

    fn fill_dcm(&mut self) {
        if self.grid.kind.is_spartan3ea() {
            let mut dcm_tiles = vec![];
            for pair in self.grid.get_dcm_pairs() {
                match pair.kind {
                    DcmPairKind::Bot => {
                        self.holes.push(Rect {
                            col_l: pair.col - 4,
                            col_r: pair.col + 4,
                            row_b: pair.row,
                            row_t: pair.row + 4,
                        });
                        dcm_tiles.push((pair.col - 1, pair.row, "DCM_BL_CENTER", false, false));
                        dcm_tiles.push((pair.col, pair.row, "DCM_BR_CENTER", false, false));
                    }
                    DcmPairKind::BotSingle => {
                        self.holes.push(Rect {
                            col_l: pair.col - 1,
                            col_r: pair.col + 4,
                            row_b: pair.row,
                            row_t: pair.row + 4,
                        });
                        dcm_tiles.push((pair.col - 1, pair.row, "DCMAUX_BL_CENTER", false, true));
                        dcm_tiles.push((pair.col, pair.row, "DCM_BR_CENTER", false, false));
                    }
                    DcmPairKind::Top => {
                        self.holes.push(Rect {
                            col_l: pair.col - 4,
                            col_r: pair.col + 4,
                            row_b: pair.row - 3,
                            row_t: pair.row + 1,
                        });
                        dcm_tiles.push((pair.col - 1, pair.row, "DCM_TL_CENTER", false, false));
                        dcm_tiles.push((pair.col, pair.row, "DCM_TR_CENTER", false, false));
                    }
                    DcmPairKind::TopSingle => {
                        self.holes.push(Rect {
                            col_l: pair.col - 1,
                            col_r: pair.col + 4,
                            row_b: pair.row - 3,
                            row_t: pair.row + 1,
                        });
                        dcm_tiles.push((pair.col - 1, pair.row, "DCMAUX_TL_CENTER", false, true));
                        dcm_tiles.push((pair.col, pair.row, "DCM_TR_CENTER", false, false));
                    }
                    DcmPairKind::Left => {
                        self.holes.push(Rect {
                            col_l: pair.col,
                            col_r: pair.col + 4,
                            row_b: pair.row - 4,
                            row_t: pair.row + 4,
                        });
                        dcm_tiles.push((pair.col, pair.row, "DCM_H_TL_CENTER", true, false));
                        dcm_tiles.push((pair.col, pair.row - 1, "DCM_H_BL_CENTER", true, false));
                    }
                    DcmPairKind::Right => {
                        self.holes.push(Rect {
                            col_l: pair.col - 3,
                            col_r: pair.col + 1,
                            row_b: pair.row - 4,
                            row_t: pair.row + 4,
                        });
                        dcm_tiles.push((pair.col, pair.row, "DCM_H_TR_CENTER", true, false));
                        dcm_tiles.push((pair.col, pair.row - 1, "DCM_H_BR_CENTER", true, false));
                    }
                    DcmPairKind::Bram => {
                        self.holes.push(Rect {
                            col_l: pair.col,
                            col_r: pair.col + 4,
                            row_b: pair.row - 4,
                            row_t: pair.row + 4,
                        });
                        dcm_tiles.push((pair.col, pair.row, "DCM_SPLY", true, false));
                        dcm_tiles.push((pair.col, pair.row - 1, "DCM_BGAP", true, false));
                    }
                }
            }
            let mut dcm_cols = vec![];
            let mut dcm_rows = vec![];
            for &(col, row, _, _, is_aux) in &dcm_tiles {
                if !is_aux {
                    dcm_cols.push(col);
                    dcm_rows.push(row);
                }
            }
            dcm_cols.sort_unstable();
            dcm_cols.dedup();
            dcm_rows.sort_unstable();
            dcm_rows.dedup();
            for (col, row, tk, is_h, is_aux) in dcm_tiles {
                let x = self.xlut[col];
                let y = row.to_idx();
                let name = format!("{tk}_X{x}Y{y}");
                self.die.fill_tile(
                    (col, row),
                    if is_aux {
                        "INT.DCM.S3E.DUMMY"
                    } else {
                        "INT.DCM"
                    },
                    if is_aux {
                        "INT.DCM.S3E.DUMMY"
                    } else if is_h {
                        "INT.DCM.S3E.H"
                    } else {
                        "INT.DCM.S3E"
                    },
                    name.clone(),
                );
                if is_aux {
                    continue;
                }
                let dx = dcm_cols.binary_search(&col).unwrap();
                let dy = dcm_rows.binary_search(&row).unwrap();
                let node = self.die.add_xnode(
                    (col, row),
                    self.db.get_node(if is_h {
                        if col < self.grid.col_clk || self.grid.kind.is_spartan3a() {
                            if row < self.grid.row_mid() {
                                "DCM.S3E.LB"
                            } else {
                                "DCM.S3E.LT"
                            }
                        } else {
                            if row < self.grid.row_mid() {
                                "DCM.S3E.RB"
                            } else {
                                "DCM.S3E.RT"
                            }
                        }
                    } else {
                        if row < self.grid.row_mid() {
                            if col < self.grid.col_clk {
                                "DCM.S3E.BL"
                            } else {
                                "DCM.S3E.BR"
                            }
                        } else {
                            if col < self.grid.col_clk {
                                "DCM.S3E.TL"
                            } else {
                                "DCM.S3E.TR"
                            }
                        }
                    }),
                    &[&name],
                    self.db.get_node_naming(if is_h {
                        "DCM.S3E.H"
                    } else if col < self.grid.col_clk {
                        "DCM.S3E.L"
                    } else {
                        "DCM.S3E.R"
                    }),
                    &[(col, row)],
                );
                node.add_bel(0, format!("DCM_X{dx}Y{dy}"));
            }
        } else {
            let row_b = self.grid.row_bot();
            let row_t = self.grid.row_top();
            let mut dx = 0;
            for (col, &cd) in self.grid.columns.iter() {
                if cd.kind != ColumnKind::Bram {
                    continue;
                }
                if self.grid.cols_gt.contains_key(&col) {
                    continue;
                }
                let (kind, naming, dcm) = match self.grid.kind {
                    GridKind::Virtex2 => ("INT.DCM.V2", "INT.BRAM_IOIS", "DCM.V2"),
                    GridKind::Virtex2P | GridKind::Virtex2PX => {
                        ("INT.DCM.V2P", "INT.ML_BRAM_IOIS", "DCM.V2P")
                    }
                    GridKind::Spartan3 => {
                        if col == self.grid.col_left() + 3 || col == self.grid.col_right() - 3 {
                            ("INT.DCM", "INT.DCM.S3", "DCM.S3")
                        } else {
                            ("INT.DCM.S3.DUMMY", "INT.DCM.S3.DUMMY", "")
                        }
                    }
                    _ => unreachable!(),
                };
                let c = self.bramclut[col];
                let name_b = format!("BIOIBRAMC{c}");
                let name_t = format!("TIOIBRAMC{c}");
                self.die
                    .fill_tile((col, row_b), kind, naming, name_b.clone());
                self.die
                    .fill_tile((col, row_t), kind, naming, name_t.clone());
                if dcm.is_empty() {
                    continue;
                }
                let node = self.die.add_xnode(
                    (col, row_b),
                    self.db.get_node(dcm),
                    &[&name_b],
                    self.db.get_node_naming(dcm),
                    &[(col, row_b)],
                );
                node.add_bel(0, format!("DCM_X{dx}Y0"));
                let node = self.die.add_xnode(
                    (col, row_t),
                    self.db.get_node(dcm),
                    &[&name_t],
                    self.db.get_node_naming(dcm),
                    &[(col, row_t)],
                );
                node.add_bel(0, format!("DCM_X{dx}Y1"));
                // terms / dcmconn added later
                dx += 1;
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
                let col = bc;
                let row = br + d;
                let r = self.rlut[row];
                let c = self.clut[col];
                let pref = match d {
                    1 => "PTERMLL",
                    14 => "PTERMUL",
                    _ => "",
                };
                let kind = match d {
                    0 => "INT.PPC.B",
                    15 => "INT.PPC.T",
                    _ => "INT.PPC.L",
                };
                self.die
                    .fill_tile((col, row), "INT.PPC", kind, format!("{pref}R{r}C{c}"));
                ints.push((col, row));
            }
            // right side
            for d in 0..16 {
                let col = bc + 9;
                let row = br + d;
                let r = self.rlut[row];
                let c = self.clut[col];
                self.die
                    .fill_tile((col, row), "INT.PPC", "INT.PPC.R", format!("R{r}C{c}"));
                ints.push((col, row));
            }
            // bottom
            for d in 1..9 {
                let col = bc + d;
                let row = br;
                let r = self.rlut[row];
                if self.grid.columns[col].kind == ColumnKind::Clb {
                    let c = self.clut[col];
                    self.die
                        .fill_tile((col, row), "INT.PPC", "INT.PPC.B", format!("R{r}C{c}"));
                } else {
                    let c = self.bramclut[col];
                    self.die.fill_tile(
                        (col, row),
                        "INT.PPC",
                        "INT.PPC.B",
                        format!("PPCINTR{r}BRAMC{c}"),
                    );
                }
                ints.push((col, row));
            }
            // top
            for d in 1..9 {
                let col = bc + d;
                let row = br + 15;
                let r = self.rlut[row];
                if self.grid.columns[col].kind == ColumnKind::Clb {
                    let c = self.clut[col];
                    self.die
                        .fill_tile((col, row), "INT.PPC", "INT.PPC.T", format!("R{r}C{c}"));
                } else {
                    let c = self.bramclut[col];
                    self.die.fill_tile(
                        (col, row),
                        "INT.PPC",
                        "INT.PPC.T",
                        format!("PPCINTR{r}BRAMC{c}"),
                    );
                }
                ints.push((col, row));
            }
            // horiz passes
            for d in 1..15 {
                let col_l = bc;
                let col_r = bc + 9;
                let row = br + d;
                let tile_l = self.die[(col_l, row)].nodes.first().unwrap().names
                    [NodeRawTileId::from_idx(0)]
                .clone();
                let c = self.bramclut[col_r - 1];
                let r = self.rlut[row];
                let tile_r = format!("BMR{r}C{c}");
                self.die.add_xnode(
                    (col_l, row),
                    self.db.get_node("PPC.E"),
                    &[&tile_l, &tile_r],
                    self.db.get_node_naming("PPC.E"),
                    &[(col_l, row), (col_r, row)],
                );
                self.die.add_xnode(
                    (col_r, row),
                    self.db.get_node("PPC.W"),
                    &[&tile_r, &tile_l],
                    self.db.get_node_naming("PPC.W"),
                    &[(col_r, row), (col_l, row)],
                );
                self.die.fill_term_pair_dbuf(
                    (col_l, row),
                    (col_r, row),
                    self.db.get_term("PPC.E"),
                    self.db.get_term("PPC.W"),
                    tile_l,
                    tile_r,
                    self.db.get_term_naming("PPC.E"),
                    self.db.get_term_naming("PPC.W"),
                );
            }
            // vert passes
            for d in 1..9 {
                let col = bc + d;
                let row_b = br;
                let row_t = br + 15;
                let rb = self.rlut[row_b + 1];
                let rt = self.rlut[row_t - 1];
                let tile_b;
                let tile_t;
                if self.grid.columns[col].kind == ColumnKind::Clb {
                    let c = self.clut[col];
                    tile_b = format!("PTERMR{rb}C{c}");
                    tile_t = format!("PTERMR{rt}C{c}");
                } else {
                    let c = self.bramclut[col];
                    tile_b = format!("PTERMBR{rb}BRAMC{c}");
                    tile_t = format!("PTERMTR{rt}BRAMC{c}");
                }
                self.die.add_xnode(
                    (col, row_b),
                    self.db.get_node("PPC.N"),
                    &[&tile_b, &tile_t],
                    self.db.get_node_naming("PPC.N"),
                    &[(col, row_b), (col, row_t)],
                );
                self.die.add_xnode(
                    (col, row_t),
                    self.db.get_node("PPC.S"),
                    &[&tile_t, &tile_b],
                    self.db.get_node_naming("PPC.S"),
                    &[(col, row_t), (col, row_b)],
                );
                self.die.fill_term_pair_dbuf(
                    (col, row_b),
                    (col, row_t),
                    self.db.get_term("PPC.N"),
                    self.db.get_term("PPC.S"),
                    tile_b,
                    tile_t,
                    self.db.get_term_naming("PPC.N"),
                    self.db.get_term_naming("PPC.S"),
                );
            }
            for dr in 0..16 {
                let row = br + dr;
                for dc in 0..10 {
                    let col = bc + dc;
                    let tile = &mut self.die[(col, row)];
                    if tile.nodes.is_empty() {
                        continue;
                    }
                    let node = tile.nodes.first().unwrap();
                    let name = node.names[NodeRawTileId::from_idx(0)].clone();
                    let nname = self.db.node_namings.key(node.naming);
                    self.die.add_xnode(
                        (col, row),
                        self.db.get_node("INTF.PPC"),
                        &[&name],
                        self.db.get_node_naming(&format!("INTF.{}", &nname[4..])),
                        &[(col, row)],
                    );
                }
            }
            let (kind, name, site) = if bc < self.grid.col_clk {
                ("LBPPC", "PPC_X0Y0", "PPC405_X0Y0")
            } else if self.grid.holes_ppc.len() == 1 {
                ("RBPPC", "PPC_X0Y0", "PPC405_X0Y0")
            } else {
                ("RBPPC", "PPC_X1Y0", "PPC405_X1Y0")
            };
            let node = self.die.add_xnode(
                (bc, br),
                self.db.get_node(kind),
                &[name],
                self.db.get_node_naming(kind),
                &ints,
            );
            node.add_bel(0, site.to_string());
        }
    }

    fn fill_gt(&mut self) {
        let row_b = self.grid.row_bot();
        let row_t = self.grid.row_top();
        for (gx, (&col, &(bbank, tbank))) in self.grid.cols_gt.iter().enumerate() {
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
            let c = self.bramclut[col];
            for row in [row_b, row_t] {
                let bt = if row == row_b { 'B' } else { 'T' };
                let name = format!("{bt}IOIBRAMC{c}");
                self.die
                    .fill_tile((col, row), "INT.GT.CLKPAD", "INT.GT.CLKPAD", name.clone());
                self.die.add_xnode(
                    (col, row),
                    self.db.get_node(if row == row_b {
                        "INTF.GT.BCLKPAD"
                    } else {
                        "INTF.GT.TCLKPAD"
                    }),
                    &[&name],
                    self.db.get_node_naming("INTF.GT.CLKPAD"),
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
                    let r = self.rlut[row];
                    let name = format!("BRAMR{r}C{c}");
                    self.die
                        .fill_tile((col, row), "INT.PPC", "INT.GT", name.clone());
                    self.die.add_xnode(
                        (col, row),
                        self.db.get_node(if d % 4 == 0 {
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
                        }),
                        &[&name],
                        self.db.get_node_naming("INTF.GT"),
                        &[(col, row)],
                    );
                }
            }
            let r = self.rlut[row_b + 1];
            let node_b;
            let node_t;
            if self.grid.kind == GridKind::Virtex2P {
                node_b = self.die.add_xnode(
                    (col, row_b),
                    self.db.get_node("GIGABIT.B"),
                    &[&format!("BMR{r}C{c}")],
                    self.db.get_node_naming("GIGABIT.B"),
                    &[
                        (col, row_b),
                        (col, row_b + 1),
                        (col, row_b + 2),
                        (col, row_b + 3),
                        (col, row_b + 4),
                    ],
                );
                node_b.add_bel(0, format!("GT_X{gx}Y0"));
            } else {
                node_b = self.die.add_xnode(
                    (col, row_b),
                    self.db.get_node("GIGABIT10.B"),
                    &[&format!("BMR{r}C{c}")],
                    self.db.get_node_naming("GIGABIT10.B"),
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
                node_b.add_bel(0, format!("GT10_X{gx}Y0"));
            }
            node_b.add_bel(1, format!("RXPPAD{bbank}"));
            node_b.add_bel(2, format!("RXNPAD{bbank}"));
            node_b.add_bel(3, format!("TXPPAD{bbank}"));
            node_b.add_bel(4, format!("TXNPAD{bbank}"));
            if self.grid.kind == GridKind::Virtex2P {
                node_t = self.die.add_xnode(
                    (col, row_t),
                    self.db.get_node("GIGABIT.T"),
                    &[&format!("BMR4C{c}")],
                    self.db.get_node_naming("GIGABIT.T"),
                    &[
                        (col, row_t),
                        (col, row_t - 4),
                        (col, row_t - 3),
                        (col, row_t - 2),
                        (col, row_t - 1),
                    ],
                );
                node_t.add_bel(0, format!("GT_X{gx}Y1"));
            } else {
                node_t = self.die.add_xnode(
                    (col, row_t),
                    self.db.get_node("GIGABIT10.T"),
                    &[&format!("BMR8C{c}")],
                    self.db.get_node_naming("GIGABIT10.T"),
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
                node_t.add_bel(0, format!("GT10_X{gx}Y1"));
            }
            node_t.add_bel(1, format!("RXPPAD{tbank}"));
            node_t.add_bel(2, format!("RXNPAD{tbank}"));
            node_t.add_bel(3, format!("TXPPAD{tbank}"));
            node_t.add_bel(4, format!("TXNPAD{tbank}"));
        }
    }

    fn fill_llv(&mut self) {
        let col_l = self.grid.col_left();
        let col_r = self.grid.col_right();
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
            let mut term_s = self.db.get_term("LLV.S");
            let mut term_n = self.db.get_term("LLV.N");
            let mut naming = self.db.get_node_naming("LLV");
            let mut tile;
            let x = self.xlut[col];
            let y = self.grid.row_mid().to_idx() - 1;
            if col == col_l || col == col_r {
                if col == col_l {
                    naming = self.db.get_node_naming("LLV.CLKL");
                    tile = format!("CLKL_IOIS_LL_X{x}Y{y}");
                } else {
                    naming = self.db.get_node_naming("LLV.CLKR");
                    tile = format!("CLKR_IOIS_LL_X{x}Y{y}");
                }
                if self.grid.kind != GridKind::Spartan3A {
                    term_s = self.db.get_term("LLV.CLKLR.S3E.S");
                    term_n = self.db.get_term("LLV.CLKLR.S3E.N");
                }
            } else {
                tile = format!("CLKH_LL_X{x}Y{y}");
            }
            if self.grid.kind == GridKind::Spartan3E {
                if col == col_l + 9 {
                    tile = format!("CLKLH_DCM_LL_X{x}Y{y}");
                }
                if col == col_r - 9 {
                    tile = format!("CLKRH_DCM_LL_X{x}Y{y}");
                }
            } else {
                if col == col_l + 3 {
                    tile = format!("CLKLH_DCM_LL_X{x}Y{y}");
                }
                if col == col_r - 6 {
                    tile = format!("CLKRH_DCM_LL_X{x}Y{y}");
                }
                if [col_l + 1, col_l + 2, col_r - 2, col_r - 1]
                    .into_iter()
                    .any(|x| x == col)
                {
                    tile = format!("CLKH_DCM_LL_X{x}Y{y}");
                }
            }
            self.die
                .fill_term_pair_anon((col, row_s), (col, row_n), term_n, term_s);
            let node_kind = if self.grid.kind.is_spartan3a() {
                "LLV.S3A"
            } else {
                "LLV.S3E"
            };
            self.die.add_xnode(
                (col, row_n),
                self.db.get_node(node_kind),
                &[&tile],
                naming,
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
            let x = self.xlut[self.grid.col_clk - 1];
            let y = row.to_idx();
            let mut term_w = self.db.get_term("LLH.W");
            let mut term_e = self.db.get_term("LLH.E");
            let tile = if row == row_b {
                format!("CLKB_LL_X{x}Y{y}")
            } else if row == row_t {
                format!("CLKT_LL_X{x}Y{y}")
            } else if self.grid.kind != GridKind::Spartan3E
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
                if self.grid.kind == GridKind::Spartan3ADsp {
                    term_w = self.db.get_term("LLH.DCM.S3ADSP.W");
                    term_e = self.db.get_term("LLH.DCM.S3ADSP.E");
                }
                format!("CLKV_DCM_LL_X{x}Y{y}")
            } else {
                format!("CLKV_LL_X{x}Y{y}")
            };
            self.die
                .fill_term_pair_anon((col_l, row), (col_r, row), term_e, term_w);
            let node_kind = if self.grid.kind.is_spartan3a() && row == self.grid.row_bot() {
                "LLH.CLKB.S3A"
            } else if self.grid.kind.is_spartan3a() && row == self.grid.row_top() {
                "LLH.CLKT.S3A"
            } else {
                "LLH"
            };

            self.die.add_xnode(
                (col_r, row),
                self.db.get_node(node_kind),
                &[&tile],
                self.db.get_node_naming("LLH"),
                &[(col_l, row), (col_r, row)],
            );
        }
    }

    fn fill_misc_passes(&mut self) {
        if self.grid.kind == GridKind::Spartan3E && !self.grid.has_ll {
            let term_s = self.db.get_term("CLKLR.S3E.S");
            let term_n = self.db.get_term("CLKLR.S3E.N");
            for col in [self.grid.col_left(), self.grid.col_right()] {
                self.die.fill_term_pair_anon(
                    (col, self.grid.row_mid() - 1),
                    (col, self.grid.row_mid()),
                    term_n,
                    term_s,
                );
            }
        }
        if self.grid.kind == GridKind::Spartan3 && !self.rows_brk.is_empty() {
            let term_s = self.db.get_term("BRKH.S3.S");
            let term_n = self.db.get_term("BRKH.S3.N");
            for &row_s in &self.rows_brk {
                let row_n = row_s + 1;
                for col in self.die.cols() {
                    self.die
                        .fill_term_pair_anon((col, row_s), (col, row_n), term_n, term_s);
                }
            }
        }
        if self.grid.kind == GridKind::Spartan3ADsp {
            let dsphole_e = self.db.get_term("DSPHOLE.E");
            let dsphole_w = self.db.get_term("DSPHOLE.W");
            let hdcm_e = self.db.get_term("HDCM.E");
            let hdcm_w = self.db.get_term("HDCM.W");
            for (col, cd) in &self.grid.columns {
                if cd.kind == ColumnKind::Dsp {
                    for row in [self.grid.row_bot(), self.grid.row_top()] {
                        self.die.fill_term_pair_anon(
                            (col, row),
                            (col + 1, row),
                            dsphole_e,
                            dsphole_w,
                        );
                    }
                }
            }
            for col in [self.grid.col_left() + 3, self.grid.col_right() - 6] {
                for row in [self.grid.row_mid() - 1, self.grid.row_mid()] {
                    self.die
                        .fill_term_pair_anon((col, row), (col + 4, row), dsphole_e, dsphole_w);
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
                        .fill_term_pair_anon((col - 1, row), (col + 4, row), hdcm_e, hdcm_w);
                }
            }
        }
    }

    fn fill_bram_passes(&mut self) {
        if self.grid.kind.is_virtex2() {
            for (col, cd) in &self.grid.columns {
                if !matches!(cd.kind, ColumnKind::Bram) {
                    continue;
                }
                for row in self.grid.rows.ids() {
                    if row.to_idx() % 4 != 1 {
                        continue;
                    }
                    if row.to_idx() == 1 {
                        continue;
                    }
                    if row == self.grid.row_top() {
                        continue;
                    }
                    if self.is_hole(col, row) {
                        continue;
                    }
                    let p = self.die[(col, row)].terms[Dir::S].as_mut().unwrap();
                    p.naming = Some(self.db.get_term_naming("BRAM.S"));
                    let c = self.bramclut[col];
                    let r = self.rlut[row];
                    p.tile = Some(format!("BMR{r}C{c}"));
                }
            }
        }

        if matches!(self.grid.kind, GridKind::Spartan3A | GridKind::Spartan3ADsp) {
            for (col, cd) in &self.grid.columns {
                if matches!(cd.kind, ColumnKind::BramCont(_)) {
                    self.die[(col, self.grid.row_bot())].terms[Dir::N] = None;
                    self.die[(col, self.grid.row_top())].terms[Dir::S] = None;
                }
            }
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

    fn fill_int_sites(&mut self) {
        for col in self.grid.columns.ids() {
            for row in self.grid.rows.ids() {
                let tile = &mut self.die[(col, row)];
                if tile.nodes.is_empty() {
                    continue;
                }
                let x = col.to_idx();
                let y = row.to_idx();
                let node = tile.nodes.first_mut().unwrap();
                node.add_bel(0, format!("RLL_X{x}Y{y}"));
                if self.db.nodes.key(node.kind) == "INT.DCM.S3E.DUMMY" {
                    continue;
                }
                let mut x = self.vcc_xlut[col];
                let mut y = self.vcc_ylut[row];
                if self.grid.kind == GridKind::Virtex2 {
                    // Look, just..... don't ask me.
                    x = col.to_idx();
                    if col == self.grid.col_left() {
                        if row == self.grid.row_bot() {
                            y = self.grid.rows.len() - 2;
                        } else if row == self.grid.row_top() {
                            y = self.grid.rows.len() - 1;
                        } else {
                            y -= 1;
                        }
                    } else if col == self.grid.col_right() {
                        if row == self.grid.row_bot() {
                            y = 0;
                            x += 1;
                        } else if row == self.grid.row_top() {
                            y = 1;
                            x += 1;
                        } else {
                            y += 1;
                        }
                    } else if col < self.grid.col_clk {
                        if row == self.grid.row_bot() {
                            y = 0;
                        } else if row == self.grid.row_top() {
                            y = 1;
                        } else {
                            y += 1;
                        }
                    } else {
                        if row == self.grid.row_bot() {
                            y = 2;
                        } else if row == self.grid.row_top() {
                            y = 3;
                        } else {
                            y += 3;
                            if y >= self.grid.rows.len() {
                                y -= self.grid.rows.len();
                                x += 1;
                            }
                        }
                    }
                }
                node.tie_name = Some(format!("VCC_X{x}Y{y}"));
            }
        }
    }

    fn fill_clkbt_v2(&mut self) {
        let row_b = self.grid.row_bot();
        let row_t = self.grid.row_top();
        let (kind_b, kind_t, tk_b, tk_t) = match self.grid.kind {
            GridKind::Virtex2 => ("CLKB.V2", "CLKT.V2", "CLKB", "CLKT"),
            GridKind::Virtex2P => ("CLKB.V2P", "CLKT.V2P", "ML_CLKB", "ML_CLKT"),
            GridKind::Virtex2PX => ("CLKB.V2PX", "CLKT.V2PX", "MK_CLKB", "MK_CLKT"),
            _ => unreachable!(),
        };
        let vx = self.vcc_xlut[self.grid.col_clk] - 1;
        let vyb = self.grid.row_bot().to_idx();
        let node = self.die.add_xnode(
            (self.grid.col_clk, row_b),
            self.db.get_node(kind_b),
            &[tk_b],
            self.db.get_node_naming(kind_b),
            &[(self.grid.col_clk - 1, row_b), (self.grid.col_clk, row_b)],
        );
        node.tie_name = Some(format!("VCC_X{vx}Y{vyb}"));
        node.add_bel(0, "BUFGMUX0P".to_string());
        node.add_bel(1, "BUFGMUX1S".to_string());
        node.add_bel(2, "BUFGMUX2P".to_string());
        node.add_bel(3, "BUFGMUX3S".to_string());
        node.add_bel(4, "BUFGMUX4P".to_string());
        node.add_bel(5, "BUFGMUX5S".to_string());
        node.add_bel(6, "BUFGMUX6P".to_string());
        node.add_bel(7, "BUFGMUX7S".to_string());
        node.add_bel(8, format!("GSIG_X{x}Y0", x = self.grid.col_clk.to_idx()));
        node.add_bel(
            9,
            format!("GSIG_X{x}Y0", x = self.grid.col_clk.to_idx() + 1),
        );
        let vyt = if self.grid.kind == GridKind::Virtex2 {
            1
        } else {
            self.grid.rows.len() - 1
        };
        let node = self.die.add_xnode(
            (self.grid.col_clk, row_t),
            self.db.get_node(kind_t),
            &[tk_t],
            self.db.get_node_naming(kind_t),
            &[(self.grid.col_clk - 1, row_t), (self.grid.col_clk, row_t)],
        );
        node.tie_name = Some(format!("VCC_X{vx}Y{vyt}"));
        node.add_bel(0, "BUFGMUX0S".to_string());
        node.add_bel(1, "BUFGMUX1P".to_string());
        node.add_bel(2, "BUFGMUX2S".to_string());
        node.add_bel(3, "BUFGMUX3P".to_string());
        node.add_bel(4, "BUFGMUX4S".to_string());
        node.add_bel(5, "BUFGMUX5P".to_string());
        node.add_bel(6, "BUFGMUX6S".to_string());
        node.add_bel(7, "BUFGMUX7P".to_string());
        node.add_bel(8, format!("GSIG_X{x}Y1", x = self.grid.col_clk.to_idx()));
        node.add_bel(
            9,
            format!("GSIG_X{x}Y1", x = self.grid.col_clk.to_idx() + 1),
        );

        let rt = self.rlut[self.grid.row_pci.unwrap()];
        let rb = self.rlut[self.grid.row_pci.unwrap() - 1];
        let col_l = self.grid.col_left();
        let node = self.die.add_xnode(
            (col_l, self.grid.row_pci.unwrap()),
            self.db.get_node("REG_L"),
            &[
                if self.grid.kind == GridKind::Virtex2 {
                    "HMLTERM"
                } else {
                    "LTERMCLKH"
                },
                &format!("LTERMR{rb}"),
                &format!("LTERMR{rt}"),
            ],
            self.db.get_node_naming("REG_L"),
            &[
                (col_l, self.grid.row_pci.unwrap() - 2),
                (col_l, self.grid.row_pci.unwrap() - 1),
                (col_l, self.grid.row_pci.unwrap()),
                (col_l, self.grid.row_pci.unwrap() + 1),
            ],
        );
        node.add_bel(0, "PCILOGIC_X0Y0".to_string());
        let col_r = self.grid.col_right();
        let node = self.die.add_xnode(
            (col_r, self.grid.row_pci.unwrap()),
            self.db.get_node("REG_R"),
            &[
                if self.grid.kind == GridKind::Virtex2 {
                    "HMRTERM"
                } else {
                    "RTERMCLKH"
                },
                &format!("RTERMR{rb}"),
                &format!("RTERMR{rt}"),
            ],
            self.db.get_node_naming("REG_R"),
            &[
                (col_r, self.grid.row_pci.unwrap() - 2),
                (col_r, self.grid.row_pci.unwrap() - 1),
                (col_r, self.grid.row_pci.unwrap()),
                (col_r, self.grid.row_pci.unwrap() + 1),
            ],
        );
        node.add_bel(0, "PCILOGIC_X1Y0".to_string());
    }

    fn fill_clkbt_s3(&mut self) {
        let (clkb, clkt, bufg) = match self.grid.kind {
            GridKind::Spartan3 => ("CLKB.S3", "CLKT.S3", "BUFGMUX"),
            GridKind::FpgaCore => ("CLKB.FC", "CLKT.FC", "BUFG"),
            _ => unreachable!(),
        };
        let row_b = self.grid.row_bot();
        let row_t = self.grid.row_top();
        let vyb = 0;
        let vyt = self.vcc_ylut[row_t];
        let vx = self.vcc_xlut[self.grid.col_clk] - 1;
        let node = self.die.add_xnode(
            (self.grid.col_clk, row_b),
            self.db.get_node(clkb),
            &["CLKB"],
            self.db.get_node_naming(clkb),
            &[(self.grid.col_clk - 1, row_b)],
        );
        node.tie_name = Some(format!("VCC_X{vx}Y{vyb}"));
        node.add_bel(0, format!("{bufg}0"));
        node.add_bel(1, format!("{bufg}1"));
        node.add_bel(2, format!("{bufg}2"));
        node.add_bel(3, format!("{bufg}3"));
        node.add_bel(4, format!("GSIG_X{x}Y0", x = self.grid.col_clk.to_idx()));
        let node = self.die.add_xnode(
            (self.grid.col_clk, row_t),
            self.db.get_node(clkt),
            &["CLKT"],
            self.db.get_node_naming(clkt),
            &[(self.grid.col_clk - 1, row_t)],
        );
        node.tie_name = Some(format!("VCC_X{vx}Y{vyt}"));
        node.add_bel(0, format!("{bufg}4"));
        node.add_bel(1, format!("{bufg}5"));
        node.add_bel(2, format!("{bufg}6"));
        node.add_bel(3, format!("{bufg}7"));
        node.add_bel(4, format!("GSIG_X{x}Y1", x = self.grid.col_clk.to_idx()));
    }

    fn fill_clkbt_s3e(&mut self) {
        let row_b = self.grid.row_bot();
        let row_t = self.grid.row_top();
        let tile_b;
        let tile_t;
        let buf_b;
        let buf_t;
        let vyb = 0;
        let vyt = self.vcc_ylut[row_t];
        let x = self.xlut[self.grid.col_clk - 1];
        let yb = row_b.to_idx();
        let ybb = yb + 1;
        let yt = row_t.to_idx();
        let ybt = yt - 1;
        if self.grid.has_ll {
            tile_b = format!("CLKB_LL_X{x}Y{yb}");
            tile_t = format!("CLKT_LL_X{x}Y{yt}");
            buf_b = format!("CLKV_LL_X{x}Y{ybb}");
            buf_t = format!("CLKV_LL_X{x}Y{ybt}");
        } else {
            tile_b = format!("CLKB_X{x}Y{yb}");
            tile_t = format!("CLKT_X{x}Y{yt}");
            buf_b = format!("CLKV_X{x}Y{ybb}");
            buf_t = format!("CLKV_X{x}Y{ybt}");
        }
        let vx = self.vcc_xlut[self.grid.col_clk] - 1;
        let kind_b = if self.grid.kind == GridKind::Spartan3E {
            "CLKB.S3E"
        } else {
            "CLKB.S3A"
        };
        let node = self.die.add_xnode(
            (self.grid.col_clk, row_b),
            self.db.get_node(kind_b),
            &[&tile_b, &buf_b],
            self.db.get_node_naming(kind_b),
            &[(self.grid.col_clk - 1, row_b)],
        );
        node.tie_name = Some(format!("VCC_X{vx}Y{vyb}"));
        node.add_bel(0, "BUFGMUX_X2Y1".to_string());
        node.add_bel(1, "BUFGMUX_X2Y0".to_string());
        node.add_bel(2, "BUFGMUX_X1Y1".to_string());
        node.add_bel(3, "BUFGMUX_X1Y0".to_string());
        node.add_bel(
            4,
            format!("GLOBALSIG_X{x}Y0", x = self.xlut[self.grid.col_clk] + 1),
        );
        let kind_t = if self.grid.kind == GridKind::Spartan3E {
            "CLKT.S3E"
        } else {
            "CLKT.S3A"
        };
        let node = self.die.add_xnode(
            (self.grid.col_clk, row_t),
            self.db.get_node(kind_t),
            &[&tile_t, &buf_t],
            self.db.get_node_naming(kind_t),
            &[(self.grid.col_clk - 1, row_t)],
        );
        node.tie_name = Some(format!("VCC_X{vx}Y{vyt}"));
        node.add_bel(0, "BUFGMUX_X2Y11".to_string());
        node.add_bel(1, "BUFGMUX_X2Y10".to_string());
        node.add_bel(2, "BUFGMUX_X1Y11".to_string());
        node.add_bel(3, "BUFGMUX_X1Y10".to_string());
        node.add_bel(
            4,
            format!(
                "GLOBALSIG_X{x}Y{y}",
                x = self.xlut[self.grid.col_clk] + 1,
                y = self.grid.rows_hclk.len() + 2
            ),
        );
    }

    fn fill_clklr_s3e(&mut self) {
        let col_l = self.grid.col_left();
        let col_r = self.grid.col_right();
        let vy = self.vcc_ylut[self.grid.row_mid()] - 1;
        let vxl = 0;
        let vxr = self.vcc_xlut[col_r] + 1;
        let xl = self.xlut[col_l];
        let xr = self.xlut[col_r];
        let y = self.grid.row_mid().to_idx() - 1;
        let tile_l = format!("CLKL_X{xl}Y{y}");
        let tile_r = format!("CLKR_X{xr}Y{y}");
        let tile_l_ioi;
        let tile_r_ioi;
        if self.grid.has_ll {
            tile_l_ioi = format!("CLKL_IOIS_LL_X{xl}Y{y}");
            tile_r_ioi = format!("CLKR_IOIS_LL_X{xr}Y{y}");
        } else if self.grid.cols_clkv.is_none() {
            tile_l_ioi = format!("CLKL_IOIS_50A_X{xl}Y{y}");
            tile_r_ioi = format!("CLKR_IOIS_50A_X{xr}Y{y}");
        } else {
            tile_l_ioi = format!("CLKL_IOIS_X{xl}Y{y}");
            tile_r_ioi = format!("CLKR_IOIS_X{xr}Y{y}");
        }
        let tiles_l: Vec<&str>;
        let tiles_r: Vec<&str>;
        let kind_l;
        let kind_r;
        if self.grid.kind == GridKind::Spartan3E {
            tiles_l = vec![&tile_l];
            tiles_r = vec![&tile_r];
            kind_l = "CLKL.S3E";
            kind_r = "CLKR.S3E";
        } else {
            tiles_l = vec![&tile_l, &tile_l_ioi];
            tiles_r = vec![&tile_r, &tile_r_ioi];
            kind_l = "CLKL.S3A";
            kind_r = "CLKR.S3A";
        }
        let gsy = (self.grid.rows_hclk.len() + 1) / 2 + 1;
        let node = self.die.add_xnode(
            (col_l, self.grid.row_mid()),
            self.db.get_node(kind_l),
            &tiles_l,
            self.db.get_node_naming(kind_l),
            &[
                (col_l, self.grid.row_mid() - 1),
                (col_l, self.grid.row_mid()),
            ],
        );
        node.add_bel(0, "BUFGMUX_X0Y2".to_string());
        node.add_bel(1, "BUFGMUX_X0Y3".to_string());
        node.add_bel(2, "BUFGMUX_X0Y4".to_string());
        node.add_bel(3, "BUFGMUX_X0Y5".to_string());
        node.add_bel(4, "BUFGMUX_X0Y6".to_string());
        node.add_bel(5, "BUFGMUX_X0Y7".to_string());
        node.add_bel(6, "BUFGMUX_X0Y8".to_string());
        node.add_bel(7, "BUFGMUX_X0Y9".to_string());
        node.add_bel(8, "PCILOGIC_X0Y0".to_string());
        node.add_bel(9, format!("VCC_X{vxl}Y{vy}"));
        node.add_bel(10, format!("GLOBALSIG_X0Y{gsy}"));
        let node = self.die.add_xnode(
            (col_r, self.grid.row_mid()),
            self.db.get_node(kind_r),
            &tiles_r,
            self.db.get_node_naming(kind_r),
            &[
                (col_r, self.grid.row_mid() - 1),
                (col_r, self.grid.row_mid()),
            ],
        );
        node.add_bel(0, "BUFGMUX_X3Y2".to_string());
        node.add_bel(1, "BUFGMUX_X3Y3".to_string());
        node.add_bel(2, "BUFGMUX_X3Y4".to_string());
        node.add_bel(3, "BUFGMUX_X3Y5".to_string());
        node.add_bel(4, "BUFGMUX_X3Y6".to_string());
        node.add_bel(5, "BUFGMUX_X3Y7".to_string());
        node.add_bel(6, "BUFGMUX_X3Y8".to_string());
        node.add_bel(7, "BUFGMUX_X3Y9".to_string());
        node.add_bel(8, "PCILOGIC_X1Y0".to_string());
        node.add_bel(9, format!("VCC_X{vxr}Y{vy}"));
        node.add_bel(
            10,
            format!("GLOBALSIG_X{x}Y{gsy}", x = self.xlut[col_r] + 3),
        );
    }

    fn fill_btterm_dcm(&mut self) {
        if self.grid.kind.is_virtex2() || self.grid.kind == GridKind::Spartan3 {
            let row_b = self.grid.row_bot();
            let row_t = self.grid.row_top();
            let mut c = 1;
            for (col, &cd) in self.grid.columns.iter() {
                if cd.kind != ColumnKind::Bram {
                    continue;
                }
                let name_b = format!("BTERMBRAMC{c}");
                let name_t = format!("TTERMBRAMC{c}");
                self.grid.fill_term(
                    &mut self.die,
                    (col, row_b),
                    "TERM.S",
                    "TERM.S",
                    name_b.clone(),
                );
                self.grid.fill_term(
                    &mut self.die,
                    (col, row_t),
                    "TERM.N",
                    "TERM.N",
                    name_t.clone(),
                );
                if self.grid.kind == GridKind::Spartan3
                    && !(col == self.grid.col_left() + 3 || col == self.grid.col_right() - 3)
                {
                    c += 1;
                    continue;
                }
                self.die.add_xnode(
                    (col, row_b),
                    self.db.get_node("DCMCONN.BOT"),
                    &[&name_b],
                    self.db.get_node_naming("DCMCONN.BOT"),
                    &[(col, row_b)],
                );
                self.die.add_xnode(
                    (col, row_t),
                    self.db.get_node("DCMCONN.TOP"),
                    &[&name_t],
                    self.db.get_node_naming("DCMCONN.TOP"),
                    &[(col, row_t)],
                );
                c += 1;
            }
        }
    }

    fn fill_pci_ce(&mut self) {
        if self.grid.kind.is_spartan3ea() {
            for c in [
                (self.grid.col_left(), self.grid.row_bot()),
                (self.grid.col_right(), self.grid.row_bot()),
                (self.grid.col_left(), self.grid.row_top()),
                (self.grid.col_right(), self.grid.row_top()),
            ] {
                let tile = &mut self.die[c];
                let name = tile.nodes.first().unwrap().names[NodeRawTileId::from_idx(0)].clone();
                self.die.add_xnode(
                    c,
                    self.db.get_node("PCI_CE_CNR"),
                    &[&name],
                    self.db.get_node_naming("PCI_CE_CNR"),
                    &[],
                );
            }

            for &(row, _, _) in &self.grid.rows_hclk {
                let kind = if row > self.grid.row_mid() {
                    "PCI_CE_N"
                } else {
                    "PCI_CE_S"
                };
                for col in [self.grid.col_left(), self.grid.col_right()] {
                    let x = self.xlut[col];
                    let y = row.to_idx() - 1;
                    let name = if row == self.grid.row_mid() {
                        format!("GCLKH_{kind}_50A_X{x}Y{y}")
                    } else {
                        format!("GCLKH_{kind}_X{x}Y{y}")
                    };
                    self.die.add_xnode(
                        (col, row),
                        self.db.get_node(kind),
                        &[&name],
                        self.db.get_node_naming(kind),
                        &[],
                    );
                }
            }
            if self.grid.kind == GridKind::Spartan3A {
                if let Some((col_l, col_r)) = self.grid.cols_clkv {
                    for row in [self.grid.row_bot(), self.grid.row_top()] {
                        let x = self.xlut[col_l] - 1;
                        let y = row.to_idx();
                        let name = format!("GCLKV_IOISL_X{x}Y{y}");
                        self.die.add_xnode(
                            (col_l, row),
                            self.db.get_node("PCI_CE_E"),
                            &[&name],
                            self.db.get_node_naming("PCI_CE_E"),
                            &[],
                        );
                        let x = self.xlut[col_r] - 1;
                        let name = format!("GCLKV_IOISR_X{x}Y{y}");
                        self.die.add_xnode(
                            (col_r, row),
                            self.db.get_node("PCI_CE_W"),
                            &[&name],
                            self.db.get_node_naming("PCI_CE_W"),
                            &[],
                        );
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
                let mut kind = "GCLKH";
                let mut naming = "GCLKH";
                let name = if self.grid.kind.is_virtex2()
                    || matches!(self.grid.kind, GridKind::Spartan3 | GridKind::FpgaCore)
                {
                    let mut r = self.grid.rows_hclk.len() - i;
                    if self.grid.columns[col].kind == ColumnKind::Bram {
                        let c = self.bramclut[col];
                        format!("GCLKHR{r}BRAMC{c}")
                    } else {
                        // *sigh*.
                        if self.grid.kind == GridKind::Virtex2 && self.die.cols().len() == 12 {
                            r -= 1;
                        }
                        let c = self.clut[col];
                        if self.grid.columns[col].kind == ColumnKind::Io
                            && self.grid.kind.is_virtex2p()
                        {
                            if col == self.grid.col_left() {
                                format!("LIOICLKR{r}")
                            } else {
                                format!("RIOICLKR{r}")
                            }
                        } else {
                            format!("GCLKHR{r}C{c}")
                        }
                    }
                } else {
                    let tk = match self.grid.columns[col].kind {
                        ColumnKind::Io => match row_m.cmp(&self.grid.row_mid()) {
                            Ordering::Less => "GCLKH_PCI_CE_S",
                            Ordering::Equal => "GCLKH_PCI_CE_S_50A",
                            Ordering::Greater => "GCLKH_PCI_CE_N",
                        },
                        ColumnKind::BramCont(x) => {
                            if row_m == self.grid.row_mid() {
                                kind = "GCLKH.UNI";
                                naming = "GCLKH.BRAM";
                                [
                                    "BRAMSITE2_DN_GCLKH",
                                    "BRAM2_GCLKH_FEEDTHRU",
                                    "BRAM2_GCLKH_FEEDTHRUA",
                                ][x as usize - 1]
                            } else if i == 0 {
                                naming = "GCLKH.BRAM.S";
                                if self.grid.kind == GridKind::Spartan3E {
                                    kind = "GCLKH.S";
                                    [
                                        "BRAMSITE2_DN_GCLKH",
                                        "BRAM2_DN_GCLKH_FEEDTHRU",
                                        "BRAM2_DN_GCLKH_FEEDTHRUA",
                                    ][x as usize - 1]
                                } else {
                                    kind = "GCLKH.UNI.S";
                                    [
                                        "BRAMSITE2_DN_GCLKH",
                                        "BRAM2_GCLKH_FEEDTHRU",
                                        "BRAM2_GCLKH_FEEDTHRUA",
                                    ][x as usize - 1]
                                }
                            } else if i == self.grid.rows_hclk.len() - 1 {
                                naming = "GCLKH.BRAM.N";
                                if self.grid.kind == GridKind::Spartan3E {
                                    kind = "GCLKH.N";
                                    [
                                        "BRAMSITE2_UP_GCLKH",
                                        "BRAM2_UP_GCLKH_FEEDTHRU",
                                        "BRAM2_UP_GCLKH_FEEDTHRUA",
                                    ][x as usize - 1]
                                } else {
                                    kind = "GCLKH.UNI.N";
                                    [
                                        "BRAMSITE2_UP_GCLKH",
                                        "BRAM2_GCLKH_FEEDTHRU",
                                        "BRAM2_GCLKH_FEEDTHRUA",
                                    ][x as usize - 1]
                                }
                            } else {
                                kind = "GCLKH.0";
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
                                        } else if row_m < self.grid.row_mid() {
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
                    let x = self.xlut[col];
                    let y = row_m.to_idx() - 1;
                    format!("{tk}_X{x}Y{y}")
                };
                let node = self.die.add_xnode(
                    (col, row_m),
                    self.db.get_node(kind),
                    &[&name],
                    self.db.get_node_naming(naming),
                    &[(col, row_m - 1), (col, row_m)],
                );
                if self.grid.kind.is_virtex2()
                    || matches!(self.grid.kind, GridKind::Spartan3 | GridKind::FpgaCore)
                {
                    let gsx = if col < self.grid.col_clk {
                        col.to_idx()
                    } else if !self.grid.kind.is_spartan3ea() {
                        col.to_idx() + 1
                    } else {
                        col.to_idx() + 2
                    };
                    let gsy = i;
                    node.add_bel(0, format!("GSIG_X{gsx}Y{gsy}"));
                } else {
                    let gsx = if col < self.grid.col_clk {
                        self.xlut[col] + 1
                    } else {
                        self.xlut[col] + 2
                    };
                    let gsy = if row_m <= self.grid.row_mid() {
                        i + 1
                    } else {
                        i + 2
                    };
                    node.add_bel(0, format!("GLOBALSIG_X{gsx}Y{gsy}"));
                    if self.grid.columns[col].kind == ColumnKind::Dsp {
                        let name = format!(
                            "MACC2_GCLKH_FEEDTHRUA_X{x}Y{y}",
                            x = self.xlut[col] + 1,
                            y = row_m.to_idx() - 1
                        );
                        let node = self.die.add_xnode(
                            (col, row_m),
                            self.db.get_node("GCLKH.DSP"),
                            &[&name],
                            self.db.get_node_naming("GCLKH.DSP"),
                            &[],
                        );
                        let gsxd = gsx + 1;
                        node.add_bel(0, format!("GLOBALSIG_X{gsxd}Y{gsy}"));
                    }
                }
            }
        }
    }

    fn fill_gclkc(&mut self) {
        for (i, &(row_m, _, _)) in self.grid.rows_hclk.iter().enumerate() {
            if self.grid.kind.is_virtex2() {
                let mut r = self.grid.rows_hclk.len() - i;
                if self.die.cols().len() == 12 {
                    r -= 1;
                }
                let name = format!("GCLKCR{r}");
                let node_kind = if row_m == self.grid.row_bot() + 1 {
                    "GCLKC.B"
                } else if row_m == self.grid.row_top() {
                    "GCLKC.T"
                } else {
                    "GCLKC"
                };
                self.die.add_xnode(
                    (self.grid.col_clk, row_m),
                    self.db.get_node(node_kind),
                    &[&name],
                    self.db.get_node_naming("GCLKC"),
                    &[],
                );
            } else if let Some((col_cl, col_cr)) = self.grid.cols_clkv {
                let r = self.grid.rows_hclk.len() - i;
                for (lr, col) in [('L', col_cl), ('R', col_cr)] {
                    let name = if matches!(self.grid.kind, GridKind::Spartan3 | GridKind::FpgaCore)
                    {
                        format!("{lr}CLKVCR{r}")
                    } else {
                        let x = self.xlut[col] - 1;
                        let y = row_m.to_idx() - 1;
                        format!("GCLKVC_X{x}Y{y}")
                    };
                    self.die.add_xnode(
                        (col, row_m),
                        self.db.get_node("GCLKVC"),
                        &[&name],
                        self.db.get_node_naming("GCLKVC"),
                        &[],
                    );
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
        let name = if self.grid.kind.is_spartan3ea() {
            let x = self.xlut[self.grid.col_clk] - 1;
            let y = self.grid.row_mid().to_idx() - 1;
            if self.grid.kind == GridKind::Spartan3E && self.grid.has_ll {
                format!("{kind}_LL_X{x}Y{y}")
            } else {
                format!("{kind}_X{x}Y{y}")
            }
        } else {
            "M".to_string()
        };
        self.die.add_xnode(
            (self.grid.col_clk, self.grid.row_mid()),
            self.db.get_node(kind),
            &[&name],
            self.db.get_node_naming(kind),
            &[],
        );
    }

    fn fill_gclkvm(&mut self) {
        if let Some((col_cl, col_cr)) = self.grid.cols_clkv {
            if matches!(self.grid.kind, GridKind::Spartan3 | GridKind::FpgaCore) {
                self.die.add_xnode(
                    (col_cl, self.grid.row_mid()),
                    self.db.get_node("GCLKVM.S3"),
                    &["LGCLKVM"],
                    self.db.get_node_naming("GCLKVM.S3"),
                    &[],
                );
                self.die.add_xnode(
                    (col_cr, self.grid.row_mid()),
                    self.db.get_node("GCLKVM.S3"),
                    &["RGCLKVM"],
                    self.db.get_node_naming("GCLKVM.S3"),
                    &[],
                );
            } else {
                let xl = self.xlut[col_cl] - 1;
                let xr = self.xlut[col_cr] - 1;
                let y = self.grid.row_mid().to_idx() - 1;
                let name_l = format!("GCLKVML_X{xl}Y{y}");
                let name_r = format!("GCLKVMR_X{xr}Y{y}");
                self.die.add_xnode(
                    (col_cl, self.grid.row_mid()),
                    self.db.get_node("GCLKVM.S3E"),
                    &[&name_l],
                    self.db.get_node_naming("GCLKVML"),
                    &[],
                );
                self.die.add_xnode(
                    (col_cr, self.grid.row_mid()),
                    self.db.get_node("GCLKVM.S3E"),
                    &[&name_r],
                    self.db.get_node_naming("GCLKVMR"),
                    &[],
                );
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
    fn fill_term(
        &self,
        die: &mut ExpandedDieRefMut,
        coord: Coord,
        kind: &str,
        naming: &str,
        name: String,
    ) {
        if self.kind.is_virtex2() {
            let kind = die.grid.db.get_node(kind);
            let naming = die.grid.db.get_node_naming(naming);
            die.add_xnode(coord, kind, &[&name], naming, &[coord]);
        }
        die.fill_term_tile(coord, kind, naming, name);
    }

    pub fn expand_grid<'a>(&'a self, db: &'a IntDb) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        egrid.tie_kind = Some("VCC".to_string());
        egrid.tie_pin_pullup = Some("VCCOUT".to_string());

        let (_, die) = egrid.add_die(self.columns.len(), self.rows.len());

        let mut expander = Expander {
            grid: self,
            db,
            die,
            holes: vec![],
            bonded_ios: vec![],
            xlut: EntityVec::new(),
            vcc_xlut: EntityVec::new(),
            vcc_ylut: EntityVec::new(),
            clut: EntityVec::new(),
            bramclut: EntityVec::new(),
            rlut: EntityVec::new(),
            rows_brk: HashSet::new(),
            ctr_pad: 1,
            ctr_nopad: if self.kind.is_spartan3ea() { 0 } else { 1 },
            frame_info: vec![],
            clkv_frame: 0,
            spine_frame: 0,
            lterm_frame: 0,
            rterm_frame: 0,
            col_frame: EntityVec::new(),
            bram_frame: EntityPartVec::new(),
        };

        expander.fill_xlut();
        expander.fill_clut();
        expander.fill_rlut();
        expander.fill_rows_brk();

        expander.fill_dcm();
        expander.fill_ppc();
        expander.fill_gt();
        expander.fill_bram_dsp();
        expander.fill_clb();
        expander.fill_cnr_int();
        expander.fill_cnr_ll();
        expander.fill_cnr_lr();
        expander.fill_cnr_ul();
        expander.fill_cnr_ur();
        expander.fill_io_t();
        expander.fill_io_r();
        expander.fill_io_b();
        expander.fill_io_l();
        if self.has_ll {
            expander.fill_llv();
            expander.fill_llh();
        }
        expander.fill_misc_passes();
        expander.die.fill_main_passes();
        expander.fill_bram_passes();
        expander.fill_vcc_lut();
        expander.fill_int_sites();
        if self.kind.is_virtex2() {
            expander.fill_clkbt_v2();
        } else if matches!(self.kind, GridKind::Spartan3 | GridKind::FpgaCore) {
            expander.fill_clkbt_s3();
        } else {
            expander.fill_clkbt_s3e();
            expander.fill_clklr_s3e();
        }
        expander.fill_btterm_dcm();
        expander.fill_pci_ce();
        expander.fill_gclkh();
        expander.fill_gclkc();
        expander.fill_clkc();
        expander.fill_gclkvm();
        expander.fill_frame_info();

        let bonded_ios = expander.bonded_ios;
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
        };

        egrid.finish();
        ExpandedDevice {
            grid: self,
            egrid,
            bonded_ios,
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
