use std::collections::{BTreeSet, BTreeMap};
use serde::{Serialize, Deserialize};
use crate::{CfgPin, BelCoord, ColId, RowId};
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub kind: GridKind,
    pub columns: EntityVec<ColId, ColumnKind>,
    pub cols_io: EntityVec<ColId, ColumnIoKind>,
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
    pub fn row_mid(&self) -> RowId {
        RowId::from_idx(self.rows.len() / 2)
    }

    pub fn get_io(&self) -> Vec<Io> {
        let mut res = Vec::new();
        let mut ctr = 1;
        match self.kind {
            GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
                // top
                for col in self.columns.ids() {
                    let row = self.rows.last_id().unwrap();
                    let is_l = col < self.col_clk - 2 || (col >= self.col_clk && col < self.col_clk + 2);
                    let bels: &[u32] = match self.cols_io[col] {
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
                    let col = self.columns.last_id().unwrap();
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
                for col in self.columns.ids().rev() {
                    let row = self.rows.first_id().unwrap();
                    let is_l = col < self.col_clk - 2 || (col >= self.col_clk && col < self.col_clk + 2);
                    let bels: &[u32] = match self.cols_io[col] {
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
                    let col = self.columns.first_id().unwrap();
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
                for col in self.columns.ids() {
                    let row = self.rows.last_id().unwrap();
                    let bels: &[u32] = match self.cols_io[col] {
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
                    let col = self.columns.last_id().unwrap();
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
                for col in self.columns.ids().rev() {
                    let row = self.rows.first_id().unwrap();
                    let bels: &[u32] = match self.cols_io[col] {
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
                    let col = self.columns.first_id().unwrap();
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
                for col in self.columns.ids() {
                    let row = self.rows.last_id().unwrap();
                    let bels: &[(u32, IoKind)] = match self.cols_io[col] {
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
                    let col = self.columns.last_id().unwrap();
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
                for col in self.columns.ids().rev() {
                    let row = self.rows.first_id().unwrap();
                    let bels: &[(u32, IoKind)] = match self.cols_io[col] {
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
                    let col = self.columns.first_id().unwrap();
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
                for col in self.columns.ids() {
                    let row = self.rows.last_id().unwrap();
                    let bels: &[(u32, IoKind)] = match self.cols_io[col] {
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
                    let col = self.columns.last_id().unwrap();
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
                for col in self.columns.ids().rev() {
                    let row = self.rows.first_id().unwrap();
                    let bels: &[(u32, IoKind)] = match self.cols_io[col] {
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
                    let col = self.columns.first_id().unwrap();
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
}
