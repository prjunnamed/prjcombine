use std::collections::{BTreeSet, BTreeMap};
use serde::{Serialize, Deserialize};
use crate::{CfgPin, BelCoord, eint, int, ColId, RowId};
use ndarray::Array2;
use prjcombine_entity::{EntityVec, EntityId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GridKind {
    Virtex2,
    Virtex2P,
    Virtex2PX,
    Spartan3,
    Spartan3E,
    Spartan3A,
    Spartan3ADsp,
}

impl GridKind {
    pub fn is_virtex2(self) -> bool {
        matches!(self, Self::Virtex2 | Self::Virtex2P | Self::Virtex2PX)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub kind: GridKind,
    pub columns: EntityVec<ColId, Column>,
    pub col_clk: ColId,
    // For Spartan 3* other than 3s50a
    pub cols_clkv: Option<(ColId, ColId)>,
    // column -> (bottom bank, top bank)
    pub cols_gt: BTreeMap<ColId, (u32, u32)>,
    pub rows: EntityVec<RowId, RowIoKind>,
    // For Spartan 3E: range of rows containing RAMs
    pub rows_ram: Option<(RowId, RowId)>,
    // (hclk row, end row)
    pub rows_hclk: Vec<(RowId, RowId)>,
    // For Virtex 2
    pub row_pci: Option<RowId>,
    pub holes_ppc: Vec<(ColId, RowId)>,
    // For Spartan 3E, 3A*
    pub dcms: Option<Dcms>,
    pub has_ll: bool,
    pub has_small_int: bool,
    pub vref: BTreeSet<BelCoord>,
    pub cfg_io: BTreeMap<CfgPin, BelCoord>,
    pub dci_io: BTreeMap<u32, (BelCoord, BelCoord)>,
    pub dci_io_alt: BTreeMap<u32, (BelCoord, BelCoord)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub kind: ColumnKind,
    pub io: ColumnIoKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKind {
    Io,
    Clb,
    Bram,
    BramCont(u8),
    Dsp,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnIoKind {
    None,
    Single,
    Double(u8),
    Triple(u8),
    Quad(u8),
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RowIoKind {
    None,
    Single,
    Double(u8),
    Triple(u8),
    Quad(u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Dcms {
    Two,
    Four,
    Eight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IoKind {
    Io,
    Input,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Io {
    pub bank: u32,
    pub coord: BelCoord,
    pub name: String,
    pub kind: IoKind,
}

impl Grid {
    pub fn col_left(&self) -> ColId {
        self.columns.first_id().unwrap()
    }

    pub fn col_right(&self) -> ColId {
        self.columns.last_id().unwrap()
    }

    pub fn row_bot(&self) -> RowId {
        self.rows.first_id().unwrap()
    }

    pub fn row_top(&self) -> RowId {
        self.rows.last_id().unwrap()
    }

    pub fn row_mid(&self) -> RowId {
        RowId::from_idx(self.rows.len() / 2)
    }

    pub fn is_virtex2(&self) -> bool {
        matches!(self.kind, GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX)
    }

    pub fn get_io(&self) -> Vec<Io> {
        let mut res = Vec::new();
        let mut ctr = 1;
        match self.kind {
            GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
                // top
                for (col, &cd) in self.columns.iter() {
                    let row = self.row_top();
                    let is_l = col < self.col_clk - 2 || (col >= self.col_clk && col < self.col_clk + 2);
                    let bels: &[u32] = match cd.io {
                        ColumnIoKind::None => &[],
                        ColumnIoKind::Single => if is_l {&[2, 1, 0]} else {&[3, 2, 1]},
                        ColumnIoKind::Double(0) => if is_l {&[3, 2, 1, 0]} else {&[3, 2]},
                        ColumnIoKind::Double(1) => if is_l {&[1, 0]} else {&[3, 2, 1, 0]},
                        _ => unreachable!(),
                    };
                    for &bel in bels {
                        let mut name = format!("PAD{ctr}");
                        if self.kind == GridKind::Virtex2PX {
                            if col == self.col_clk - 1 {
                                match bel {
                                    0 => name = format!("CLKPPAD1"),
                                    1 => name = format!("CLKNPAD1"),
                                    _ => (),
                                }
                            }
                        }
                        res.push(Io {
                            coord: BelCoord {
                                col,
                                row,
                                bel,
                            },
                            bank: if col < self.col_clk { 0 } else { 1 },
                            name,
                            kind: IoKind::Io,
                        });
                        ctr += 1;
                    }
                }
                // right
                for (row, &kind) in self.rows.iter().rev() {
                    let col = self.col_right();
                    let is_b = row < self.row_mid();
                    let bels: &[u32] = match kind {
                        RowIoKind::None => &[],
                        RowIoKind::Double(0) => if is_b {&[3, 2, 1, 0]} else {&[1, 0]},
                        RowIoKind::Double(1) => if is_b {&[3, 2]} else {&[3, 2, 1, 0]},
                        _ => unreachable!(),
                    };
                    for &bel in bels {
                        res.push(Io {
                            coord: BelCoord {
                                col,
                                row,
                                bel,
                            },
                            bank: if row < self.row_mid() { 3 } else { 2 },
                            name: format!("PAD{ctr}"),
                            kind: IoKind::Io,
                        });
                        ctr += 1;
                    }
                }
                // bottom
                for (col, &cd) in self.columns.iter().rev() {
                    let row = self.row_bot();
                    let is_l = col < self.col_clk - 2 || (col >= self.col_clk && col < self.col_clk + 2);
                    let bels: &[u32] = match cd.io {
                        ColumnIoKind::None => &[],
                        ColumnIoKind::Single => if is_l {&[3, 2, 1]} else {&[2, 1, 0]},
                        ColumnIoKind::Double(0) => if is_l {&[3, 2, 1, 0]} else {&[1, 0]},
                        ColumnIoKind::Double(1) => if is_l {&[3, 2]} else {&[3, 2, 1, 0]},
                        _ => unreachable!(),
                    };
                    for &bel in bels {
                        let mut name = format!("PAD{ctr}");
                        if self.kind == GridKind::Virtex2PX {
                            if col == self.col_clk - 1 {
                                match bel {
                                    2 => name = format!("CLKPPAD2"),
                                    3 => name = format!("CLKNPAD2"),
                                    _ => (),
                                }
                            }
                        }
                        res.push(Io {
                            coord: BelCoord {
                                col,
                                row,
                                bel,
                            },
                            bank: if col < self.col_clk { 5 } else { 4 },
                            name,
                            kind: IoKind::Io,
                        });
                        ctr += 1;
                    }
                }
                // left
                for (row, &kind) in self.rows.iter() {
                    let col = self.col_left();
                    let is_b = row < self.row_mid();
                    let bels: &[u32] = match kind {
                        RowIoKind::None => &[],
                        RowIoKind::Double(0) => if is_b {&[0, 1, 2, 3]} else {&[0, 1]},
                        RowIoKind::Double(1) => if is_b {&[2, 3]} else {&[0, 1, 2, 3]},
                        _ => unreachable!(),
                    };
                    for &bel in bels {
                        res.push(Io {
                            coord: BelCoord {
                                col,
                                row,
                                bel,
                            },
                            bank: if row < self.row_mid() { 6 } else { 7 },
                            name: format!("PAD{ctr}"),
                            kind: IoKind::Io,
                        });
                        ctr += 1;
                    }
                }
            }
            GridKind::Spartan3 => {
                // top
                for (col, &cd) in self.columns.iter() {
                    let row = self.row_top();
                    let bels: &[u32] = match cd.io {
                        ColumnIoKind::None => &[],
                        ColumnIoKind::Double(0) => &[2, 1, 0],
                        ColumnIoKind::Double(1) => &[1, 0],
                        _ => unreachable!(),
                    };
                    for &bel in bels {
                        res.push(Io {
                            coord: BelCoord {
                                col,
                                row,
                                bel,
                            },
                            bank: if col < self.col_clk { 0 } else { 1 },
                            name: format!("PAD{ctr}"),
                            kind: IoKind::Io,
                        });
                        ctr += 1;
                    }
                }
                // right
                for (row, &kind) in self.rows.iter().rev() {
                    let col = self.col_right();
                    let bels: &[u32] = match kind {
                        RowIoKind::None => &[],
                        RowIoKind::Single => &[1, 0],
                        _ => unreachable!(),
                    };
                    for &bel in bels {
                        res.push(Io {
                            coord: BelCoord {
                                col,
                                row,
                                bel,
                            },
                            bank: if row < self.row_mid() { 3 } else { 2 },
                            name: format!("PAD{ctr}"),
                            kind: IoKind::Io,
                        });
                        ctr += 1;
                    }
                }
                // bottom
                for (col, &cd) in self.columns.iter().rev() {
                    let row = self.row_bot();
                    let bels: &[u32] = match cd.io {
                        ColumnIoKind::None => &[],
                        ColumnIoKind::Double(0) => &[2, 1, 0],
                        ColumnIoKind::Double(1) => &[1, 0],
                        _ => unreachable!(),
                    };
                    for &bel in bels {
                        res.push(Io {
                            coord: BelCoord {
                                col,
                                row,
                                bel,
                            },
                            bank: if col < self.col_clk { 5 } else { 4 },
                            name: format!("PAD{ctr}"),
                            kind: IoKind::Io,
                        });
                        ctr += 1;
                    }
                }
                // left
                for (row, &kind) in self.rows.iter() {
                    let col = self.col_left();
                    let bels: &[u32] = match kind {
                        RowIoKind::None => &[],
                        RowIoKind::Single => &[0, 1],
                        _ => unreachable!(),
                    };
                    for &bel in bels {
                        res.push(Io {
                            coord: BelCoord {
                                col,
                                row,
                                bel,
                            },
                            bank: if row < self.row_mid() { 6 } else { 7 },
                            name: format!("PAD{ctr}"),
                            kind: IoKind::Io,
                        });
                        ctr += 1;
                    }
                }
            }
            GridKind::Spartan3E => {
                const I: IoKind = IoKind::Input;
                const IO: IoKind = IoKind::Io;
                // top
                for (col, &cd) in self.columns.iter() {
                    let row = self.row_top();
                    let bels: &[(u32, IoKind)] = match cd.io {
                        ColumnIoKind::None => &[],
                        ColumnIoKind::Single => &[(2, IO)],
                        ColumnIoKind::Double(0) => &[(1, IO), (0, IO)],
                        ColumnIoKind::Double(1) => &[(2, I)],
                        ColumnIoKind::Triple(0) => &[(1, IO), (0, IO)],
                        ColumnIoKind::Triple(1) => &[(2, I)],
                        ColumnIoKind::Triple(2) => &[(1, IO), (0, IO)],
                        ColumnIoKind::Quad(0) => &[(1, IO), (0, IO)],
                        ColumnIoKind::Quad(1) => &[(2, IO)],
                        ColumnIoKind::Quad(2) => &[(1, IO), (0, IO)],
                        ColumnIoKind::Quad(3) => &[(1, I), (0, I)],
                        _ => unreachable!(),
                    };
                    for &(bel, kind) in bels {
                        res.push(Io {
                            coord: BelCoord {
                                col,
                                row,
                                bel,
                            },
                            bank: 0,
                            name: if kind == IoKind::Io {format!("PAD{ctr}")} else {format!("IPAD{ctr}")},
                            kind,
                        });
                        ctr += 1;
                    }
                }
                // right
                for (row, &kind) in self.rows.iter().rev() {
                    let col = self.col_right();
                    let bels: &[(u32, IoKind)] = match kind {
                        RowIoKind::None => &[],
                        RowIoKind::Single => &[(2, IO)],
                        RowIoKind::Double(0) => &[(1, IO), (0, IO)],
                        RowIoKind::Double(1) => &[],
                        RowIoKind::Triple(0) => &[(1, IO), (0, IO)],
                        RowIoKind::Triple(1) => &[(2, IO)],
                        RowIoKind::Triple(2) => &[(2, I)],
                        RowIoKind::Quad(0) => &[(1, IO), (0, IO)],
                        RowIoKind::Quad(1) => &[],
                        RowIoKind::Quad(2) => &[(1, IO), (0, IO)],
                        RowIoKind::Quad(3) => &[(2, I)],
                        _ => unreachable!(),
                    };
                    for &(bel, kind) in bels {
                        res.push(Io {
                            coord: BelCoord {
                                col,
                                row,
                                bel,
                            },
                            bank: 1,
                            name: if kind == IoKind::Io {format!("PAD{ctr}")} else {format!("IPAD{ctr}")},
                            kind,
                        });
                        ctr += 1;
                    }
                }
                // bottom
                for (col, &cd) in self.columns.iter().rev() {
                    let row = self.row_bot();
                    let bels: &[(u32, IoKind)] = match cd.io {
                        ColumnIoKind::None => &[],
                        ColumnIoKind::Single => &[(2, IO)],
                        ColumnIoKind::Double(0) => &[(2, I)],
                        ColumnIoKind::Double(1) => &[(1, IO), (0, IO)],
                        ColumnIoKind::Triple(0) => &[(1, IO), (0, IO)],
                        ColumnIoKind::Triple(1) => &[(2, I)],
                        ColumnIoKind::Triple(2) => &[(1, IO), (0, IO)],
                        ColumnIoKind::Quad(0) => &[(1, I), (0, I)],
                        ColumnIoKind::Quad(1) => &[(1, IO), (0, IO)],
                        ColumnIoKind::Quad(2) => &[(2, IO)],
                        ColumnIoKind::Quad(3) => &[(1, IO), (0, IO)],
                        _ => unreachable!(),
                    };
                    for &(bel, kind) in bels {
                        res.push(Io {
                            coord: BelCoord {
                                col,
                                row,
                                bel,
                            },
                            bank: 2,
                            name: if kind == IoKind::Io {format!("PAD{ctr}")} else {format!("IPAD{ctr}")},
                            kind,
                        });
                        ctr += 1;
                    }
                }
                // left
                for (row, &kind) in self.rows.iter() {
                    let col = self.col_left();
                    let bels: &[(u32, IoKind)] = match kind {
                        RowIoKind::None => &[],
                        RowIoKind::Single => &[(2, IO)],
                        RowIoKind::Double(0) => &[],
                        RowIoKind::Double(1) => &[(1, IO), (0, IO)],
                        RowIoKind::Triple(0) => &[(2, I)],
                        RowIoKind::Triple(1) => &[(2, IO)],
                        RowIoKind::Triple(2) => &[(1, IO), (0, IO)],
                        RowIoKind::Quad(0) => &[(2, I)],
                        RowIoKind::Quad(1) => &[(1, IO), (0, IO)],
                        RowIoKind::Quad(2) => &[],
                        RowIoKind::Quad(3) => &[(1, IO), (0, IO)],
                        _ => unreachable!(),
                    };
                    for &(bel, kind) in bels {
                        res.push(Io {
                            coord: BelCoord {
                                col,
                                row,
                                bel,
                            },
                            bank: 3,
                            name: if kind == IoKind::Io {format!("PAD{ctr}")} else {format!("IPAD{ctr}")},
                            kind,
                        });
                        ctr += 1;
                    }
                }
            }
            GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                const I: IoKind = IoKind::Input;
                const IO: IoKind = IoKind::Io;
                // top
                for (col, &cd) in self.columns.iter() {
                    let row = self.row_top();
                    let bels: &[(u32, IoKind)] = match cd.io {
                        ColumnIoKind::None => &[],
                        ColumnIoKind::Double(0) => &[(0, IO), (1, IO), (2, I)],
                        ColumnIoKind::Double(1) => &[(0, IO), (1, IO)],
                        _ => unreachable!(),
                    };
                    for &(bel, kind) in bels {
                        res.push(Io {
                            coord: BelCoord {
                                col,
                                row,
                                bel,
                            },
                            bank: 0,
                            name: if kind == IoKind::Io {format!("PAD{ctr}")} else {format!("IPAD{ctr}")},
                            kind,
                        });
                        ctr += 1;
                    }
                }
                // right
                for (row, &kind) in self.rows.iter().rev() {
                    let col = self.col_right();
                    let bels: &[(u32, IoKind)] = match kind {
                        RowIoKind::None => &[],
                        RowIoKind::Quad(0) => &[(1, IO), (0, IO)],
                        RowIoKind::Quad(1) => &[(1, IO), (0, IO)],
                        RowIoKind::Quad(2) => &[(1, IO), (0, IO)],
                        RowIoKind::Quad(3) => &[(1, I), (0, I)],
                        _ => unreachable!(),
                    };
                    for &(bel, kind) in bels {
                        res.push(Io {
                            coord: BelCoord {
                                col,
                                row,
                                bel,
                            },
                            bank: 1,
                            name: if kind == IoKind::Io {format!("PAD{ctr}")} else {format!("IPAD{ctr}")},
                            kind,
                        });
                        ctr += 1;
                    }
                }
                // bottom
                for (col, &cd) in self.columns.iter().rev() {
                    let row = self.row_bot();
                    let bels: &[(u32, IoKind)] = match cd.io {
                        ColumnIoKind::None => &[],
                        ColumnIoKind::Double(0) => &[(2, I), (1, IO), (0, IO)],
                        ColumnIoKind::Double(1) => &[(1, IO), (0, IO)],
                        _ => unreachable!(),
                    };
                    for &(bel, kind) in bels {
                        let mut name = if kind == IoKind::Io {format!("PAD{ctr}")} else {format!("IPAD{ctr}")};
                        // 3s50a special
                        if self.cols_clkv.is_none() {
                            match ctr {
                                94 => name = format!("PAD96"),
                                95 => name = format!("IPAD94"),
                                96 => name = format!("PAD97"),
                                97 => name = format!("PAD95"),
                                _ => (),
                            }
                        }
                        res.push(Io {
                            coord: BelCoord {
                                col,
                                row,
                                bel,
                            },
                            bank: 2,
                            name,
                            kind,
                        });
                        ctr += 1;
                    }
                }
                // left
                for (row, &kind) in self.rows.iter() {
                    let col = self.col_left();
                    let bels: &[(u32, IoKind)] = match kind {
                        RowIoKind::None => &[],
                        RowIoKind::Quad(0) => &[(0, I), (1, I)],
                        RowIoKind::Quad(1) => &[(0, IO), (1, IO)],
                        RowIoKind::Quad(2) => &[(0, IO), (1, IO)],
                        RowIoKind::Quad(3) => &[(0, IO), (1, IO)],
                        _ => unreachable!(),
                    };
                    for &(bel, kind) in bels {
                        res.push(Io {
                            coord: BelCoord {
                                col,
                                row,
                                bel,
                            },
                            bank: 3,
                            name: if kind == IoKind::Io {format!("PAD{ctr}")} else {format!("IPAD{ctr}")},
                            kind,
                        });
                        ctr += 1;
                    }
                }
            }
        }
        res
    }

    fn fill_term(&self, slr: &mut eint::ExpandedSlrRefMut, coord: eint::Coord, kind: &str, naming: &str, name: String) {
        if self.kind.is_virtex2() {
            let kind = slr.grid.db.get_node(kind);
            let naming = slr.grid.db.get_node_naming(naming);
            slr[coord].add_xnode(kind, &[&name], naming, &[coord]);
        }
        slr.fill_term_tile(coord, kind, naming, name);
    }

    pub fn expand_grid<'a>(&self, db: &'a int::IntDb) -> eint::ExpandedGrid<'a> {
        let mut egrid = eint::ExpandedGrid::new(db);
        egrid.tie_kind = Some("VCC".to_string());
        egrid.tie_pin_pullup = Some("VCCOUT".to_string());

        let slrid = egrid.tiles.push(Array2::default([self.rows.len(), self.columns.len()]));
        let mut grid = egrid.slr_mut(slrid);
        let def_rt = int::NodeRawTileId::from_idx(0);

        let use_xy = matches!(self.kind, GridKind::Spartan3E | GridKind::Spartan3A | GridKind::Spartan3ADsp);
        let mut rows_brk: BTreeSet<_> = self.rows_hclk.iter().map(|&(_, r)| r - 1).collect();
        rows_brk.remove(&self.row_top());
        if self.kind != GridKind::Spartan3ADsp {
            rows_brk.remove(&(self.row_mid() - 1));
        }
        let mut xtmp = 0;
        let xlut = self.columns.map_values(|cd| {
            let res = xtmp;
            if cd.kind == ColumnKind::Dsp {
                xtmp += 2;
            } else {
                xtmp += 1;
            }
            res
        });
        xtmp = 1;
        let clut = self.columns.map_values(|cd| {
            let res = xtmp;
            if cd.kind == ColumnKind::Clb {
                xtmp += 1;
            }
            res
        });
        xtmp = 1;
        let bramclut = self.columns.map_values(|cd| {
            let res = xtmp;
            if cd.kind == ColumnKind::Bram {
                xtmp += 1;
            }
            res
        });

        let cnr_kind = if self.is_virtex2() {"CNR"} else {"CLB"};
        let col_l = self.col_left();
        let col_r = self.col_right();
        let row_b = self.row_bot();
        let row_t = self.row_top();
        let xl = xlut[col_l];
        let xr = xlut[col_r];
        let yb = row_b.to_idx();
        let yt = row_t.to_idx();
        if use_xy {
            grid.fill_tile((col_l, row_b), cnr_kind, "CNR", format!("LL_X{xl}Y{yb}"));
            grid.fill_tile((col_r, row_b), cnr_kind, "CNR", format!("LR_X{xr}Y{yb}"));
            grid.fill_tile((col_l, row_t), cnr_kind, "CNR", format!("UL_X{xl}Y{yt}"));
            grid.fill_tile((col_r, row_t), cnr_kind, "CNR", format!("UR_X{xr}Y{yt}"));
            self.fill_term(&mut grid, (col_l, row_b), "TERM.W", "TERM.W", format!("CNR_LBTERM_X{xl}Y{yb}"));
            self.fill_term(&mut grid, (col_l, row_t), "TERM.W", "TERM.W", format!("CNR_LTTERM_X{xl}Y{yt}"));
            self.fill_term(&mut grid, (col_r, row_b), "TERM.E", "TERM.E", format!("CNR_RBTERM_X{xr}Y{yb}"));
            self.fill_term(&mut grid, (col_r, row_t), "TERM.E", "TERM.E", format!("CNR_RTTERM_X{xr}Y{yt}"));
            self.fill_term(&mut grid, (col_l, row_b), "TERM.S", "TERM.S.CNR", format!("CNR_BTERM_X{xl}Y{yb}"));
            self.fill_term(&mut grid, (col_l, row_t), "TERM.N", "TERM.N.CNR", format!("CNR_TTERM_X{xl}Y{yt}"));
            self.fill_term(&mut grid, (col_r, row_b), "TERM.S", "TERM.S.CNR", format!("CNR_BTERM_X{xr}Y{yb}"));
            self.fill_term(&mut grid, (col_r, row_t), "TERM.N", "TERM.N.CNR", format!("CNR_TTERM_X{xr}Y{yt}"));
        } else if matches!(self.kind, GridKind::Virtex2P | GridKind::Virtex2PX) {
            grid.fill_tile((col_l, row_b), cnr_kind, "CNR", format!("LIOIBIOI"));
            grid.fill_tile((col_r, row_b), cnr_kind, "CNR", format!("RIOIBIOI"));
            grid.fill_tile((col_l, row_t), cnr_kind, "CNR", format!("LIOITIOI"));
            grid.fill_tile((col_r, row_t), cnr_kind, "CNR", format!("RIOITIOI"));
            self.fill_term(&mut grid, (col_l, row_b), "TERM.W", "TERM.W", format!("LTERMBIOI"));
            self.fill_term(&mut grid, (col_l, row_t), "TERM.W", "TERM.W", format!("LTERMTIOI"));
            self.fill_term(&mut grid, (col_r, row_b), "TERM.E", "TERM.E", format!("RTERMBIOI"));
            self.fill_term(&mut grid, (col_r, row_t), "TERM.E", "TERM.E", format!("RTERMTIOI"));
            self.fill_term(&mut grid, (col_l, row_b), "TERM.S", "TERM.S.CNR", format!("LIOIBTERM"));
            self.fill_term(&mut grid, (col_l, row_t), "TERM.N", "TERM.N.CNR", format!("LIOITTERM"));
            self.fill_term(&mut grid, (col_r, row_b), "TERM.S", "TERM.S.CNR", format!("RIOIBTERM"));
            self.fill_term(&mut grid, (col_r, row_t), "TERM.N", "TERM.N.CNR", format!("RIOITTERM"));
        } else {
            grid.fill_tile((col_l, row_b), cnr_kind, "CNR", format!("BL"));
            grid.fill_tile((col_r, row_b), cnr_kind, "CNR", format!("BR"));
            grid.fill_tile((col_l, row_t), cnr_kind, "CNR", format!("TL"));
            grid.fill_tile((col_r, row_t), cnr_kind, "CNR", format!("TR"));
            self.fill_term(&mut grid, (col_l, row_b), "TERM.W", "TERM.W", format!("LBTERM"));
            self.fill_term(&mut grid, (col_l, row_t), "TERM.W", "TERM.W", format!("LTTERM"));
            self.fill_term(&mut grid, (col_r, row_b), "TERM.E", "TERM.E", format!("RBTERM"));
            self.fill_term(&mut grid, (col_r, row_t), "TERM.E", "TERM.E", format!("RTTERM"));
            self.fill_term(&mut grid, (col_l, row_b), "TERM.S", "TERM.S.CNR", format!("BLTERM"));
            self.fill_term(&mut grid, (col_l, row_t), "TERM.N", "TERM.N.CNR", format!("TLTERM"));
            self.fill_term(&mut grid, (col_r, row_b), "TERM.S", "TERM.S.CNR", format!("BRTERM"));
            self.fill_term(&mut grid, (col_r, row_t), "TERM.N", "TERM.N.CNR", format!("TRTERM"));
        }

        let io_kind = match self.kind {
            GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => "IOI",
            GridKind::Spartan3 => "IOI.S3",
            GridKind::Spartan3E => "IOI.S3E",
            _ => "IOI.S3A.LR",
        };
        for (row, kind) in self.rows.iter() {
            if matches!(kind, RowIoKind::None) {
                continue;
            }
            let naming = match self.kind {
                GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => "IOI.LR",
                GridKind::Spartan3 => "IOI",
                GridKind::Spartan3E => if rows_brk.contains(&row) {"IOI.BRK"} else {"IOI"},
                GridKind::Spartan3A | GridKind::Spartan3ADsp => if rows_brk.contains(&row) {"IOI.S3A.LR.BRK"} else {"IOI.S3A.LR"},
            };
            if use_xy {
                let y = row.to_idx();
                let (lk, rk) = match kind {
                    RowIoKind::Quad(0) | RowIoKind::Triple(0) => ("LIBUFS", "RIOIS"),
                    RowIoKind::Quad(3) | RowIoKind::Triple(2) => ("LIOIS", "RIBUFS"),
                    _ => ("LIOIS", "RIOIS"),
                };
                let clk = if row == self.row_mid() - 1 || row == self.row_mid() {"_CLK"} else {""};
                let pci = if row >= self.row_mid() - 4 && row < self.row_mid() + 4 {"_PCI"} else {""};
                let brk = if rows_brk.contains(&row) {"_BRK"} else {""};
                grid.fill_tile((col_l, row), io_kind, naming, format!("{lk}{clk}{pci}{brk}_X{xl}Y{y}"));
                grid.fill_tile((col_r, row), io_kind, naming, format!("{rk}{clk}{pci}{brk}_X{xr}Y{y}"));
                let (mut ltk, mut rtk) = match kind {
                    RowIoKind::Single => ("LTERM1", "RTERM1"),
                    RowIoKind::Double(0) => ("LTERM2", "RTERM2"),
                    RowIoKind::Triple(0) => ("LTERM3", "RTERM3"),
                    RowIoKind::Quad(0) => ("LTERM4", "RTERM4"),
                    _ => ("LTERM", "RTERM"),
                };
                if row == self.row_mid() {
                    ltk = "LTERM4CLK";
                    rtk = "RTERM4CLK";
                }
                if self.kind == GridKind::Spartan3E {
                    if row == self.row_mid() - 4 {
                        ltk = "LTERM4B";
                        rtk = "RTERM4CLKB";
                    }
                    if row == self.row_mid() - 3 {
                        ltk = "LTERMCLKA";
                    }
                    if row == self.row_mid() - 2 {
                        rtk = "RTERMCLKA";
                    }
                    if row == self.row_mid() - 1 {
                        ltk = "LTERMCLK";
                    }
                    if row == self.row_mid() + 1 {
                        ltk = "LTERMCLKA";
                    }
                    if row == self.row_mid() + 2 {
                        rtk = "RTERMCLKA";
                    }
                    if row == self.row_mid() + 3 {
                        ltk = "LTERMCLK";
                    }
                } else {
                    if row == self.row_mid() - 4 {
                        ltk = "LTERM4B";
                        rtk = "RTERM4B";
                    }
                    if row == self.row_mid() - 3 {
                        rtk = "RTERMCLKB";
                    }
                    if row == self.row_mid() - 2 {
                        ltk = "LTERMCLKA";
                        rtk = "RTERMCLKA";
                    }
                    if row == self.row_mid() - 1 {
                        ltk = "LTERMCLK";
                    }
                    if row == self.row_mid() + 1 {
                        ltk = "LTERMCLKA";
                        rtk = "RTERMCLKA";
                    }
                    if row == self.row_mid() + 2 {
                        ltk = "LTERMCLK";
                    }
                }
                self.fill_term(&mut grid, (col_l, row), "TERM.W", "TERM.W", format!("{ltk}_X{xl}Y{y}"));
                self.fill_term(&mut grid, (col_r, row), "TERM.E", "TERM.E", format!("{rtk}_X{xr}Y{y}"));
            } else {
                let r = yt - row.to_idx();
                grid.fill_tile((col_l, row), io_kind, naming, format!("LIOIR{r}"));
                grid.fill_tile((col_r, row), io_kind, naming, format!("RIOIR{r}"));
                let t_e;
                let t_w;
                if self.kind == GridKind::Spartan3 {
                    t_e = "TERM.E";
                    t_w = "TERM.W";
                } else if row < self.row_pci.unwrap() {
                    t_e = "TERM.E.D";
                    t_w = "TERM.W.D";
                } else {
                    t_e = "TERM.E.U";
                    t_w = "TERM.W.U";
                }
                self.fill_term(&mut grid, (col_l, row), "TERM.W", t_w, format!("LTERMR{r}"));
                self.fill_term(&mut grid, (col_r, row), "TERM.E", t_e, format!("RTERMR{r}"));
            }
        }

        let io_naming = match self.kind {
            GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => "IOI.TB",
            GridKind::Spartan3 | GridKind::Spartan3E => "IOI",
            GridKind::Spartan3A | GridKind::Spartan3ADsp => "IOI.S3A.TB",
        };
        let io_kind = match self.kind {
            GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => "IOI",
            GridKind::Spartan3 => "IOI.S3",
            GridKind::Spartan3E => "IOI.S3E",
            _ => "IOI.S3A.TB",
        };
        for (col, cd) in self.columns.iter() {
            if use_xy {
                if cd.kind == ColumnKind::Io {
                    continue;
                }
                let (bk, tk) = match (self.kind, cd.io) {
                    (GridKind::Spartan3E, ColumnIoKind::Double(0) | ColumnIoKind::Quad(0)) => ("BIBUFS", "TIOIS"),
                    (GridKind::Spartan3E, ColumnIoKind::Double(1) | ColumnIoKind::Quad(3)) => ("BIOIS", "TIBUFS"),
                    (GridKind::Spartan3E, ColumnIoKind::Triple(1)) => ("BIBUFS", "TIBUFS"),
                    (GridKind::Spartan3A | GridKind::Spartan3ADsp, ColumnIoKind::Double(0)) => ("BIOIB", "TIOIB"),
                    _ => ("BIOIS", "TIOIS"),
                };
                let x = xlut[col];
                grid.fill_tile((col, row_b), io_kind, io_naming, format!("{bk}_X{x}Y{yb}"));
                grid.fill_tile((col, row_t), io_kind, io_naming, format!("{tk}_X{x}Y{yt}"));
                let (mut btk, mut ttk) = match cd.io {
                    ColumnIoKind::Single => ("BTERM1", "TTERM1"),
                    ColumnIoKind::Double(0) => ("BTERM2", "TTERM2"),
                    ColumnIoKind::Triple(0) => ("BTERM3", "TTERM3"),
                    ColumnIoKind::Quad(0) => ("BTERM4", "TTERM4"),
                    _ => ("BTERM", "TTERM"),
                };
                if self.kind == GridKind::Spartan3E {
                    if cd.io == ColumnIoKind::Quad(0) && cd.kind == ColumnKind::BramCont(2) {
                        btk = "BTERM4_BRAM2";
                        ttk = "TTERM4_BRAM2";
                    }
                    if col == self.col_clk - 3 {
                        btk = "BTERMCLKA";
                    }
                    if col == self.col_clk - 2 {
                        ttk = "TTERMCLK";
                    }
                    if col == self.col_clk - 1 {
                        btk = "BTERMCLKB";
                        ttk = "TTERMCLKA";
                    }
                    if col == self.col_clk {
                        btk = "BTERM4CLK";
                        ttk = "TTERM4CLK";
                    }
                    if col == self.col_clk + 1 {
                        btk = "BTERMCLK";
                    }
                    if col == self.col_clk + 2 {
                        ttk = "TTERMCLKA";
                    }
                } else {
                    if col == self.col_clk - 2 {
                        btk = "BTERM2CLK";
                        ttk = "TTERM2CLK";
                    }
                    if col == self.col_clk - 1 {
                        btk = "BTERMCLKB";
                        ttk = "TTERMCLKA";
                    }
                    if col == self.col_clk {
                        btk = "BTERM2CLK";
                        ttk = "TTERM2CLK";
                    }
                    if col == self.col_clk + 1 {
                        btk = "BTERMCLK";
                        ttk = "TTERMCLKA";
                    }
                }
                if self.kind == GridKind::Spartan3ADsp {
                    match cd.kind {
                        ColumnKind::BramCont(2) => {
                            btk = "BTERM1";
                            ttk = "TTERM1";
                        }
                        ColumnKind::Dsp => {
                            btk = "BTERM1_MACC";
                            ttk = "TTERM1_MACC";
                        }
                        _ => (),
                    }
                }
                self.fill_term(&mut grid, (col, row_b), "TERM.S", "TERM.S", format!("{btk}_X{x}Y{yb}"));
                self.fill_term(&mut grid, (col, row_t), "TERM.N", "TERM.N", format!("{ttk}_X{x}Y{yt}"));
            } else {
                if cd.kind != ColumnKind::Clb {
                    continue;
                }
                let c = clut[col];
                if self.kind == GridKind::Virtex2PX && col == self.col_clk - 1 {
                    grid.fill_tile((col, row_b), "IOI.CLK_B", "IOI.CLK_B", format!("BIOIC{c}"));
                    grid.fill_tile((col, row_t), "IOI.CLK_T", "IOI.CLK_T", format!("TIOIC{c}"));
                } else {
                    grid.fill_tile((col, row_b), io_kind, io_naming, format!("BIOIC{c}"));
                    grid.fill_tile((col, row_t), io_kind, io_naming, format!("TIOIC{c}"));
                }
                self.fill_term(&mut grid, (col, row_b), "TERM.S", "TERM.S", format!("BTERMC{c}"));
                self.fill_term(&mut grid, (col, row_t), "TERM.N", "TERM.N", format!("TTERMC{c}"));
            }
        }

        for (col, &cd) in self.columns.iter() {
            if self.kind == GridKind::Spartan3E {
                if cd.kind == ColumnKind::Io {
                    continue;
                }
            } else {
                if cd.kind != ColumnKind::Clb {
                    continue;
                }
            }
            for (row, &io) in self.rows.iter() {
                if io == RowIoKind::None {
                    continue;
                }
                if use_xy {
                    let x = xlut[col];
                    let y = row.to_idx();
                    let naming = if rows_brk.contains(&row) {"CLB.BRK"} else {"CLB"};
                    grid.fill_tile((col, row), "CLB", naming, format!("CLB_X{x}Y{y}"));
                } else {
                    let c = clut[col];
                    let r = yt - row.to_idx();
                    grid.fill_tile((col, row), "CLB", "CLB", format!("R{r}C{c}"));
                }
            }
        }

        let bram_kind = match self.kind {
            GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => "BRAM",
            GridKind::Spartan3 => "BRAM.S3",
            GridKind::Spartan3E => "BRAM.S3E",
            GridKind::Spartan3A => "BRAM.S3A",
            GridKind::Spartan3ADsp => "BRAM.S3ADSP",
        };
        for (col, &cd) in self.columns.iter() {
            if cd.kind != ColumnKind::Bram {
                continue;
            }
            if let Some((b, t)) = self.rows_ram {
                grid.nuke_rect(col, b, 4, t.to_idx() - b.to_idx() + 1);
                for d in 1..4 {
                    let x = xlut[col + d];
                    let yb = b.to_idx();
                    let yt = t.to_idx();
                    grid.fill_term_pair_bounce(
                        (col + d, b - 1),
                        (col + d, t + 1),
                        db.get_term("TERM.BRAM.N"),
                        db.get_term("TERM.BRAM.S"),
                        format!("COB_TERM_B_X{x}Y{yb}"),
                        format!("COB_TERM_T_X{x}Y{yt}"),
                        db.get_term_naming("TERM.BRAM.N"),
                        db.get_term_naming("TERM.BRAM.S"),
                    );
                }
            }
            let mut i = 0;
            for (row, &io) in self.rows.iter() {
                if io == RowIoKind::None {
                    continue;
                }
                if let Some((b, t)) = self.rows_ram {
                    if row <= b || row >= t {
                        continue;
                    }
                }
                let naming = match self.kind {
                    GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX | GridKind::Spartan3 => "BRAM",
                    GridKind::Spartan3E | GridKind::Spartan3A => if rows_brk.contains(&row) {"BRAM.BRK"} else {"BRAM"},
                    GridKind::Spartan3ADsp => if rows_brk.contains(&row) {"BRAM.S3ADSP.BRK"} else {"BRAM.S3ADSP"},
                };
                if use_xy {
                    let x = xlut[col];
                    let y = row.to_idx();
                    let mut md = "";
                    if rows_brk.contains(&row) {
                        md = "_BRK";
                    }
                    if self.kind != GridKind::Spartan3E {
                        if row == row_b + 1 {
                            md = "_BOT";
                        }
                        if row == row_t - 1 {
                            md = "_TOP";
                        }
                        if self.cols_clkv.is_none() && row == row_t - 5 {
                            md = "_TOP";
                        }
                    }
                    grid.fill_tile((col, row), bram_kind, naming, format!("BRAM{i}_SMALL{md}_X{x}Y{y}"));
                    if self.kind == GridKind::Spartan3ADsp {
                        let naming_macc = if rows_brk.contains(&row) {"MACC.BRK"} else {"MACC"};
                        let x = xlut[col + 3];
                        grid.fill_tile((col + 3, row), "BRAM.S3ADSP", naming_macc, format!("MACC{i}_SMALL{md}_X{x}Y{y}"));
                    }
                    i += 1;
                    i %= 4;
                } else {
                    let c = bramclut[col];
                    let r = yt - row.to_idx();
                    grid.fill_tile((col, row), bram_kind, naming, format!("BRAMR{r}C{c}"));
                }
            }
        }

        if let Some(dcms) = self.dcms {
            if dcms != Dcms::Two {
                grid.nuke_rect(self.col_clk - 4, row_b + 1, 4, 4);
                let x = xlut[self.col_clk - 1];
                let y = row_b.to_idx() + 1;
                grid.fill_tile_special((self.col_clk - 1, row_b + 1), "DCM", "DCM.S3E", format!("DCM_BL_CENTER_X{x}Y{y}"));
            }
            if !(self.kind != GridKind::Spartan3E && dcms == Dcms::Two) {
                grid.nuke_rect(self.col_clk, row_b + 1, 4, 4);
                let x = xlut[self.col_clk];
                let y = row_b.to_idx() + 1;
                grid.fill_tile_special((self.col_clk, row_b + 1), "DCM", "DCM.S3E", format!("DCM_BR_CENTER_X{x}Y{y}"));
            }
            if !(self.kind == GridKind::Spartan3E && dcms == Dcms::Two) {
                grid.nuke_rect(self.col_clk - 4, row_t - 4, 4, 4);
                let x = xlut[self.col_clk - 1];
                let y = row_t.to_idx() - 1;
                grid.fill_tile_special((self.col_clk - 1, row_t - 1), "DCM", "DCM.S3E", format!("DCM_TL_CENTER_X{x}Y{y}"));
            }
            {
                grid.nuke_rect(self.col_clk, row_t - 4, 4, 4);
                let x = xlut[self.col_clk];
                let y = row_t.to_idx() - 1;
                grid.fill_tile_special((self.col_clk, row_t - 1), "DCM", "DCM.S3E", format!("DCM_TR_CENTER_X{x}Y{y}"));
            }
            if self.kind == GridKind::Spartan3E && dcms == Dcms::Two {
                grid.nuke_rect(self.col_clk - 1, row_b + 1, 1, 4);
                grid.nuke_rect(self.col_clk - 1, row_t - 4, 1, 4);
                let x = xlut[self.col_clk - 1];
                let y = row_b.to_idx() + 1;
                grid.fill_tile_special((self.col_clk - 1, row_b + 1), "DCM.S3E.DUMMY", "DCM.S3E.DUMMY", format!("DCMAUX_BL_CENTER_X{x}Y{y}"));
                let y = row_t.to_idx() - 1;
                grid.fill_tile_special((self.col_clk - 1, row_t - 1), "DCM.S3E.DUMMY", "DCM.S3E.DUMMY", format!("DCMAUX_TL_CENTER_X{x}Y{y}"));
            }
            if dcms == Dcms::Eight {
                if self.kind == GridKind::Spartan3E {
                    grid.nuke_rect(col_l + 9, self.row_mid() - 4, 4, 4);
                    grid.nuke_rect(col_l + 9, self.row_mid(), 4, 4);
                    grid.nuke_rect(col_r - 12, self.row_mid() - 4, 4, 4);
                    grid.nuke_rect(col_r - 12, self.row_mid(), 4, 4);
                    let col = col_l + 9;
                    let x = xlut[col];
                    let row = self.row_mid();
                    let y = row.to_idx();
                    grid.fill_tile_special((col, row), "DCM", "DCM.S3E.H", format!("DCM_H_TL_CENTER_X{x}Y{y}"));
                    let row = self.row_mid() - 1;
                    let y = row.to_idx();
                    grid.fill_tile_special((col, row), "DCM", "DCM.S3E.H", format!("DCM_H_BL_CENTER_X{x}Y{y}"));
                    let col = col_r - 9;
                    let x = xlut[col];
                    let row = self.row_mid();
                    let y = row.to_idx();
                    grid.fill_tile_special((col, row), "DCM", "DCM.S3E.H", format!("DCM_H_TR_CENTER_X{x}Y{y}"));
                    let row = self.row_mid() - 1;
                    let y = row.to_idx();
                    grid.fill_tile_special((col, row), "DCM", "DCM.S3E.H", format!("DCM_H_BR_CENTER_X{x}Y{y}"));
                } else {
                    for col in [col_l + 3, col_r - 6] {
                        grid.nuke_rect(col, self.row_mid() - 4, 4, 4);
                        grid.nuke_rect(col, self.row_mid(), 4, 4);
                        let x = xlut[col];
                        let row = self.row_mid();
                        let y = row.to_idx();
                        grid.fill_tile_special((col, row), "DCM", "DCM.S3E.H", format!("DCM_SPLY_X{x}Y{y}"));
                        let row = self.row_mid() - 1;
                        let y = row.to_idx();
                        grid.fill_tile_special((col, row), "DCM", "DCM.S3E.H", format!("DCM_BGAP_X{x}Y{y}"));
                    }
                }
            }
        } else {
            for (col, &cd) in self.columns.iter() {
                if cd.kind != ColumnKind::Bram {
                    continue;
                }
                let (kind, naming) = match self.kind {
                    GridKind::Virtex2 => ("DCM.V2", "BRAM_IOIS"),
                    GridKind::Virtex2P | GridKind::Virtex2PX => ("DCM.V2P", "ML_BRAM_IOIS"),
                    GridKind::Spartan3 => if col == col_l + 3 || col == col_r - 3 {
                        ("DCM", "DCM.S3")
                    } else {
                        ("DCM.S3.DUMMY", "DCM.S3.DUMMY")
                    }
                    _ => unreachable!(),
                };
                let c = bramclut[col];
                grid.fill_tile((col, row_b), kind, naming, format!("BIOIBRAMC{c}"));
                grid.fill_tile((col, row_t), kind, naming, format!("TIOIBRAMC{c}"));
                self.fill_term(&mut grid, (col, row_b), "TERM.S", "TERM.S", format!("BTERMBRAMC{c}"));
                self.fill_term(&mut grid, (col, row_t), "TERM.N", "TERM.N", format!("TTERMBRAMC{c}"));
            }
        }

        for &(bc, br) in &self.holes_ppc {
            grid.nuke_rect(bc, br, 10, 16);
            // left side
            for d in 1..15 {
                let col = bc;
                let row = br + d;
                let r = yt - row.to_idx();
                let c = clut[col];
                let pref = match d {
                    1 => "PTERMLL",
                    14 => "PTERMUL",
                    _ => "",
                };
                grid.fill_tile_special((col, row), "PPC", "PPC.L", format!("{pref}R{r}C{c}"));
            }
            // right side
            for d in 0..16 {
                let col = bc + 9;
                let row = br + d;
                let r = yt - row.to_idx();
                let c = clut[col];
                grid.fill_tile_special((col, row), "PPC", "PPC.R", format!("R{r}C{c}"));
            }
            // bottom
            for d in 0..9 {
                let col = bc + d;
                let row = br;
                let r = yt - row.to_idx();
                if self.columns[col].kind == ColumnKind::Clb {
                    let c = clut[col];
                    grid.fill_tile_special((col, row), "PPC", "PPC.B", format!("R{r}C{c}"));
                } else {
                    let c = bramclut[col];
                    grid.fill_tile_special((col, row), "PPC", "PPC.B", format!("PPCINTR{r}BRAMC{c}"));
                }
            }
            // top
            for d in 0..9 {
                let col = bc + d;
                let row = br + 15;
                let r = yt - row.to_idx();
                if self.columns[col].kind == ColumnKind::Clb {
                    let c = clut[col];
                    grid.fill_tile_special((col, row), "PPC", "PPC.T", format!("R{r}C{c}"));
                } else {
                    let c = bramclut[col];
                    grid.fill_tile_special((col, row), "PPC", "PPC.T", format!("PPCINTR{r}BRAMC{c}"));
                }
            }
            // horiz passes
            for d in 1..15 {
                let col_l = bc;
                let col_r = bc + 9;
                let row = br + d;
                let tile_l = grid[(col_l, row)].nodes[0].names[def_rt].clone();
                let c = bramclut[col_r - 1];
                let r = yt - row.to_idx();
                let tile_r = format!("BMR{r}C{c}");
                grid[(col_l, row)].add_xnode(
                    db.get_node("PPC.E"),
                    &[&tile_l, &tile_r],
                    db.get_node_naming("PPC.E"),
                    &[(col_l, row), (col_r, row)]
                );
                grid[(col_r, row)].add_xnode(
                    db.get_node("PPC.W"),
                    &[&tile_r, &tile_l],
                    db.get_node_naming("PPC.W"),
                    &[(col_r, row), (col_l, row)]
                );
                grid.fill_term_pair_dbuf((col_l, row), (col_r, row), db.get_term("PPC.E"), db.get_term("PPC.W"), tile_l, tile_r, db.get_term_naming("PPC.E"), db.get_term_naming("PPC.W"));
            }
            // vert passes
            for d in 1..9 {
                let col = bc + d;
                let row_b = br;
                let row_t = br + 15;
                let rb = yt - row_b.to_idx() - 1;
                let rt = yt - row_t.to_idx() + 1;
                let tile_b;
                let tile_t;
                if self.columns[col].kind == ColumnKind::Clb {
                    let c = clut[col];
                    tile_b = format!("PTERMR{rb}C{c}");
                    tile_t = format!("PTERMR{rt}C{c}");
                } else {
                    let c = bramclut[col];
                    tile_b = format!("PTERMBR{rb}BRAMC{c}");
                    tile_t = format!("PTERMTR{rt}BRAMC{c}");
                }
                grid[(col, row_b)].add_xnode(
                    db.get_node("PPC.N"),
                    &[&tile_b, &tile_t],
                    db.get_node_naming("PPC.N"),
                    &[(col, row_b), (col, row_t)]
                );
                grid[(col, row_t)].add_xnode(
                    db.get_node("PPC.S"),
                    &[&tile_t, &tile_b],
                    db.get_node_naming("PPC.S"),
                    &[(col, row_t), (col, row_b)]
                );
                grid.fill_term_pair_dbuf((col, row_b), (col, row_t), db.get_term("PPC.N"), db.get_term("PPC.S"), tile_b, tile_t, db.get_term_naming("PPC.N"), db.get_term_naming("PPC.S"));
            }
            for dr in 0..16 {
                let row = br + dr;
                for dc in 0..10 {
                    let col = bc + dc;
                    let tile = &mut grid[(col, row)];
                    if tile.nodes.is_empty() {
                        continue;
                    }
                    let name = tile.nodes[0].names[def_rt].clone();
                    let nname = db.node_namings.key(tile.nodes[0].naming);
                    tile.add_intf(
                        db.get_intf("PPC"),
                        name,
                        db.get_intf_naming(nname),
                    );
                }
            }
        }

        for &col in self.cols_gt.keys() {
            let kind_gt = db.get_node("PPC");
            let kind_gt0 = db.get_node("GT.CLKPAD");
            let naming_gt = db.get_node_naming("GT");
            let naming_gt0 = db.get_node_naming("GT.CLKPAD");
            for row in [row_b, row_t] {
                let tile = &mut grid[(col, row)];
                let node = &mut tile.nodes[0];
                node.special = true;
                node.kind = kind_gt0;
                node.naming = naming_gt0;
                let name = node.names[def_rt].clone();
                tile.add_intf(
                    db.get_intf("GT.CLKPAD"),
                    name,
                    db.get_intf_naming("GT.CLKPAD"),
                );
            }
            let n = match self.kind {
                GridKind::Virtex2P => 4,
                GridKind::Virtex2PX => 8,
                _ => unreachable!(),
            };
            for br in [row_b + 1, row_t - n] {
                for d in 0..n {
                    let row = br + d;
                    let tile = &mut grid[(col, row)];
                    let node = &mut tile.nodes[0];
                    node.special = true;
                    node.kind = kind_gt;
                    node.naming = naming_gt;
                    let name = node.names[def_rt].clone();
                    tile.add_intf(
                        db.get_intf(if d % 4 == 0 {"GT.0"} else {"GT.123"}),
                        name,
                        db.get_intf_naming("GT"),
                    );
                }
            }
        }

        if self.has_ll {
            for col in self.columns.ids() {
                if matches!(self.columns[col].kind, ColumnKind::BramCont(_)) {
                    continue;
                }
                let mut row_s = self.row_mid() - 1;
                let mut row_n = self.row_mid();
                while grid[(col, row_s)].nodes.is_empty() {
                    row_s -= 1;
                }
                while grid[(col, row_n)].nodes.is_empty() {
                    row_n += 1;
                }
                let mut term_s = db.get_term("LLV.S");
                let mut term_n = db.get_term("LLV.N");
                let mut naming = db.get_node_naming("LLV");
                let mut tile;
                let x = xlut[col];
                let y = self.row_mid().to_idx() - 1;
                if col == col_l || col == col_r {
                    if col == col_l {
                        naming = db.get_node_naming("LLV.CLKL");
                        tile = format!("CLKL_IOIS_LL_X{x}Y{y}");
                    } else {
                        naming = db.get_node_naming("LLV.CLKR");
                        tile = format!("CLKR_IOIS_LL_X{x}Y{y}");
                    }
                    if self.kind != GridKind::Spartan3A {
                        term_s = db.get_term("LLV.CLKLR.S3E.S");
                        term_n = db.get_term("LLV.CLKLR.S3E.N");
                    }
                } else {
                    tile = format!("CLKH_LL_X{x}Y{y}");
                }
                if self.kind == GridKind::Spartan3E {
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
                    if [col_l + 1, col_l + 2, col_r - 2, col_r - 1].into_iter().any(|x| x == col) {
                        tile = format!("CLKH_DCM_LL_X{x}Y{y}");
                    }
                }
                grid.fill_term_pair_anon((col, row_s), (col, row_n), term_n, term_s);
                grid[(col, row_n)].add_xnode(
                    db.get_node("LLV"),
                    &[&tile],
                    naming,
                    &[(col, row_n), (col, row_s)],
                );
            }
            for row in self.rows.ids() {
                let mut col_l = self.col_clk - 1;
                let mut col_r = self.col_clk;
                while grid[(col_l, row)].nodes.is_empty() {
                    col_l -= 1;
                }
                while grid[(col_r, row)].nodes.is_empty() {
                    col_r += 1;
                }
                let x = xlut[self.col_clk - 1];
                let y = row.to_idx();
                let mut term_w = db.get_term("LLH.W");
                let mut term_e = db.get_term("LLH.E");
                let tile = if row == row_b {
                    format!("CLKB_LL_X{x}Y{y}")
                } else if row == row_t {
                    format!("CLKT_LL_X{x}Y{y}")
                } else if self.kind != GridKind::Spartan3E && [row_b + 2, row_b + 3, row_b + 4, row_t - 4, row_t - 3, row_t - 2].into_iter().any(|x| x == row) {
                    if self.kind == GridKind::Spartan3ADsp {
                        term_w = db.get_term("LLH.DCM.S3ADSP.W");
                        term_e = db.get_term("LLH.DCM.S3ADSP.E");
                    }
                    format!("CLKV_DCM_LL_X{x}Y{y}")
                } else {
                    format!("CLKV_LL_X{x}Y{y}")
                };
                grid.fill_term_pair_anon((col_l, row), (col_r, row), term_e, term_w);
                grid[(col_r, row)].add_xnode(
                    db.get_node("LLH"),
                    &[&tile],
                    db.get_node_naming("LLH"),
                    &[(col_r, row), (col_l, row)],
                );
            }
        }
        if self.kind == GridKind::Spartan3E && !self.has_ll {
            let term_s = db.get_term("CLKLR.S3E.S");
            let term_n = db.get_term("CLKLR.S3E.N");
            for col in [col_l, col_r] {
                grid.fill_term_pair_anon((col, self.row_mid() - 1), (col, self.row_mid()), term_n, term_s);
            }
        }
        if self.kind == GridKind::Spartan3 && !rows_brk.is_empty() {
            let term_s = db.get_term("BRKH.S3.S");
            let term_n = db.get_term("BRKH.S3.N");
            for &row_s in &rows_brk {
                let row_n = row_s + 1;
                for col in grid.cols() {
                    grid.fill_term_pair_anon((col, row_s), (col, row_n), term_n, term_s);
                }
            }
        }
        if self.kind == GridKind::Spartan3ADsp {
            let dsphole_e = db.get_term("DSPHOLE.E");
            let dsphole_w = db.get_term("DSPHOLE.W");
            let hdcm_e = db.get_term("HDCM.E");
            let hdcm_w = db.get_term("HDCM.W");
            for (col, cd) in &self.columns {
                if cd.kind == ColumnKind::Dsp {
                    for row in [row_b, row_t] {
                        grid.fill_term_pair_anon((col, row), (col + 1, row), dsphole_e, dsphole_w);
                    }
                }
            }
            for col in [col_l + 3, col_r - 6] {
                for row in [
                    self.row_mid() - 1,
                    self.row_mid(),
                ] {
                    grid.fill_term_pair_anon((col, row), (col + 4, row), dsphole_e, dsphole_w);
                }
                for row in [
                    self.row_mid() - 4,
                    self.row_mid() - 3,
                    self.row_mid() - 2,
                    self.row_mid() + 1,
                    self.row_mid() + 2,
                    self.row_mid() + 3 
                ] {
                    grid.fill_term_pair_anon((col - 1, row), (col + 4, row), hdcm_e, hdcm_w);
                }
            }
        }
        grid.fill_main_passes();

        if self.is_virtex2() {
            for (col, cd) in &self.columns {
                if !matches!(cd.kind, ColumnKind::Bram) {
                    continue;
                }
                for row in self.rows.ids() {
                    if row.to_idx() % 4 != 1 {
                        continue;
                    }
                    if row.to_idx() == 1 {
                        continue;
                    }
                    let et = &mut grid[(col, row)];
                    if et.nodes.is_empty() {
                        continue;
                    }
                    if et.nodes[0].special {
                        continue;
                    }
                    if let Some(ref mut p) = et.terms[int::Dir::S] {
                        p.naming = Some(db.get_term_naming("BRAM.N"));
                    } else {
                        unreachable!();
                    }
                    if let Some(ref mut p) = grid[(col, row - 1)].terms[int::Dir::N] {
                        p.naming = Some(db.get_term_naming("BRAM.S"));
                    } else {
                        unreachable!();
                    }
                }
            }
        }

        if matches!(self.kind, GridKind::Spartan3A | GridKind::Spartan3ADsp) {
            for (col, cd) in &self.columns {
                if matches!(cd.kind, ColumnKind::BramCont(_)) {
                    grid[(col, row_b)].terms[int::Dir::N] = None;
                    grid[(col, row_t)].terms[int::Dir::S] = None;
                }
            }
        }

        xtmp = 0;
        if matches!(self.kind, GridKind::Spartan3E | GridKind::Spartan3A | GridKind::Spartan3ADsp) {
            xtmp += 1;
        }
        let mut vcc_xlut = EntityVec::new();
        for col in self.columns.ids() {
            vcc_xlut.push(xtmp);
            if col == self.col_clk - 1 {
                xtmp += 2;
            } else {
                xtmp += 1;
            }
        }
        xtmp = 0;
        if matches!(self.kind, GridKind::Spartan3E | GridKind::Spartan3A | GridKind::Spartan3ADsp) {
            xtmp += 1;
        }
        let mut vcc_ylut = EntityVec::new();
        for row in self.rows.ids() {
            vcc_ylut.push(xtmp);
            if row == self.row_mid() - 1 && matches!(self.kind, GridKind::Spartan3E | GridKind::Spartan3A | GridKind::Spartan3ADsp) {
                xtmp += 2;
            } else {
                xtmp += 1;
            }
        }
        for col in self.columns.ids() {
            for row in self.rows.ids() {
                let tile = &mut grid[(col, row)];
                if tile.nodes.is_empty() {
                    continue;
                }
                if db.nodes.key(tile.nodes[0].kind) == "DCM.S3E.DUMMY" {
                    continue;
                }
                let mut x = vcc_xlut[col];
                let mut y = vcc_ylut[row];
                if self.kind == GridKind::Virtex2 {
                    // Look, just..... don't ask me.
                    x = col.to_idx();
                    if col == col_l {
                        if row == row_b {
                            y = self.rows.len() - 2;
                        } else if row == row_t {
                            y = self.rows.len() - 1;
                        } else {
                            y -= 1;
                        }
                    } else if col == col_r {
                        if row == row_b {
                            y = 0;
                            x += 1;
                        } else if row == row_t {
                            y = 1;
                            x += 1;
                        } else {
                            y += 1;
                        }
                    } else if col < self.col_clk {
                        if row == row_b {
                            y = 0;
                        } else if row == row_t {
                            y = 1;
                        } else {
                            y += 1;
                        }
                    } else {
                        if row == row_b {
                            y = 2;
                        } else if row == row_t {
                            y = 3;
                        } else {
                            y += 3;
                            if y >= self.rows.len() {
                                y -= self.rows.len();
                                x += 1;
                            }
                        }
                    }
                }
                tile.nodes[0].tie_name = Some(format!("VCC_X{x}Y{y}"));
            }
        }

        if self.kind.is_virtex2() {
            let (kind_b, kind_t) = match self.kind {
                GridKind::Virtex2 => ("CLKB", "CLKT"),
                GridKind::Virtex2P => ("ML_CLKB", "ML_CLKT"),
                GridKind::Virtex2PX => ("MK_CLKB", "MK_CLKT"),
                _ => unreachable!(),
            };
            let vx = vcc_xlut[self.col_clk] - 1;
            let vyb = row_b.to_idx();
            let node = grid[(self.col_clk - 1, row_b)].add_xnode(
                db.get_node("CLKB"),
                &[kind_b],
                db.get_node_naming("CLKB"),
                &[(self.col_clk - 1, row_b), (self.col_clk, row_b)],
            );
            node.tie_name = Some(format!("VCC_X{vx}Y{vyb}"));
            let vyt = if self.kind == GridKind::Virtex2 {1} else {row_t.to_idx()};
            let node = grid[(self.col_clk - 1, row_t)].add_xnode(
                db.get_node("CLKT"),
                &[kind_t],
                db.get_node_naming("CLKT"),
                &[(self.col_clk - 1, row_t), (self.col_clk, row_t)],
            );
            node.tie_name = Some(format!("VCC_X{vx}Y{vyt}"));
        } else {
            let kind_b;
            let kind_t;
            let tile_b;
            let tile_t;
            let vyb = 0;
            let vyt = vcc_ylut[row_t];
            if self.kind == GridKind::Spartan3 {
                kind_b = "CLKB.S3";
                kind_t = "CLKT.S3";
                tile_b = format!("CLKB");
                tile_t = format!("CLKT");
            } else {
                kind_b = "CLKB.S3E";
                kind_t = "CLKT.S3E";
                let x = xlut[self.col_clk - 1];
                let yb = row_b.to_idx();
                let yt = row_t.to_idx();
                if self.has_ll {
                    tile_b = format!("CLKB_LL_X{x}Y{yb}");
                    tile_t = format!("CLKT_LL_X{x}Y{yt}");
                } else {
                    tile_b = format!("CLKB_X{x}Y{yb}");
                    tile_t = format!("CLKT_X{x}Y{yt}");
                }
            }
            let vx = vcc_xlut[self.col_clk] - 1;
            let node = grid[(self.col_clk - 1, row_b)].add_xnode(
                db.get_node(kind_b),
                &[&tile_b],
                db.get_node_naming("CLKB"),
                &[(self.col_clk - 1, row_b)]
            );
            node.tie_name = Some(format!("VCC_X{vx}Y{vyb}"));
            let node = grid[(self.col_clk - 1, row_t)].add_xnode(
                db.get_node(kind_t),
                &[&tile_t],
                db.get_node_naming("CLKT"),
                &[(self.col_clk - 1, row_t)]
            );
            node.tie_name = Some(format!("VCC_X{vx}Y{vyt}"));
        }

        egrid
    }
}
