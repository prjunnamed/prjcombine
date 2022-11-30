use prjcombine_entity::{entity_id, EntityId, EntityVec};
use prjcombine_int::db::Dir;
use prjcombine_int::grid::{ColId, RowId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

entity_id! {
    pub id TileIobId u8;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct IoCoord {
    pub col: ColId,
    pub row: RowId,
    pub iob: TileIobId,
}

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
    pub fn is_virtex2p(self) -> bool {
        matches!(self, Self::Virtex2P | Self::Virtex2PX)
    }
    pub fn is_spartan3ea(self) -> bool {
        matches!(self, Self::Spartan3E | Self::Spartan3A | Self::Spartan3ADsp)
    }
    pub fn is_spartan3a(self) -> bool {
        matches!(self, Self::Spartan3A | Self::Spartan3ADsp)
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
    // (hclk row, start row, end row)
    pub rows_hclk: Vec<(RowId, RowId, RowId)>,
    // For Virtex 2
    pub row_pci: Option<RowId>,
    pub holes_ppc: Vec<(ColId, RowId)>,
    // For Spartan 3E, 3A*
    pub dcms: Option<Dcms>,
    pub has_ll: bool,
    pub has_small_int: bool,
    pub vref: BTreeSet<IoCoord>,
    pub cfg_io: BTreeMap<SharedCfgPin, IoCoord>,
    pub dci_io: BTreeMap<u32, (IoCoord, IoCoord)>,
    pub dci_io_alt: BTreeMap<u32, (IoCoord, IoCoord)>,
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
    SingleLeft,
    SingleRight,
    SingleLeftAlt,
    SingleRightAlt,
    DoubleLeft(u8),
    DoubleRight(u8),
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RowIoKind {
    None,
    Single,
    Double(u8),
    Triple(u8),
    Quad(u8),
    DoubleBot(u8),
    DoubleTop(u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Dcms {
    Two,
    Four,
    Eight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    Data(u8), // ×8
    CsiB,     // Called CS_B on Virtex 2 and Spartan 3.
    InitB,
    RdWrB,
    Dout,
    // Shared on Spartan 3E, Spartan 3A only; dedicated on Virtex 2, Spartan 3.
    M0,
    M1,
    M2,
    Cclk,
    HswapEn,
    // Spartan 3E, Spartan 3A only.
    CsoB,
    Ldc0,
    Ldc1,
    Ldc2,
    Hdc,
    Addr(u8), // ×20 on 3s100e, ×24 on other Spartan 3E, ×26 on Spartan 3A
    // Spartan 3A only.
    Awake,
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

    pub fn get_clk_io(&self, edge: Dir, idx: usize) -> Option<IoCoord> {
        if self.kind.is_virtex2() {
            match edge {
                Dir::S => {
                    if self.kind == GridKind::Virtex2PX && matches!(idx, 6 | 7) {
                        return None;
                    }
                    if idx < 4 {
                        Some(IoCoord {
                            col: self.col_clk,
                            row: self.row_bot(),
                            iob: TileIobId::from_idx(idx),
                        })
                    } else if idx < 8 {
                        Some(IoCoord {
                            col: self.col_clk - 1,
                            row: self.row_bot(),
                            iob: TileIobId::from_idx(idx - 4),
                        })
                    } else {
                        None
                    }
                }
                Dir::N => {
                    if self.kind == GridKind::Virtex2PX && matches!(idx, 4 | 5) {
                        return None;
                    }
                    if idx < 4 {
                        Some(IoCoord {
                            col: self.col_clk,
                            row: self.row_top(),
                            iob: TileIobId::from_idx(idx),
                        })
                    } else if idx < 8 {
                        Some(IoCoord {
                            col: self.col_clk - 1,
                            row: self.row_top(),
                            iob: TileIobId::from_idx(idx - 4),
                        })
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else if self.kind == GridKind::Spartan3 {
            match edge {
                Dir::S => {
                    if idx < 2 {
                        Some(IoCoord {
                            col: self.col_clk,
                            row: self.row_bot(),
                            iob: TileIobId::from_idx(idx),
                        })
                    } else if idx < 4 {
                        Some(IoCoord {
                            col: self.col_clk - 1,
                            row: self.row_bot(),
                            iob: TileIobId::from_idx(idx - 2),
                        })
                    } else {
                        None
                    }
                }
                Dir::N => {
                    if idx < 2 {
                        Some(IoCoord {
                            col: self.col_clk,
                            row: self.row_top(),
                            iob: TileIobId::from_idx(idx),
                        })
                    } else if idx < 4 {
                        Some(IoCoord {
                            col: self.col_clk - 1,
                            row: self.row_top(),
                            iob: TileIobId::from_idx(idx - 2),
                        })
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else if self.kind == GridKind::Spartan3E {
            match (edge, idx) {
                (Dir::S, 0 | 1) => Some(IoCoord {
                    col: self.col_clk,
                    row: self.row_bot(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::S, 2 | 3) => Some(IoCoord {
                    col: self.col_clk + 1,
                    row: self.row_bot(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::S, 4 | 5) => Some(IoCoord {
                    col: self.col_clk - 3,
                    row: self.row_bot(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::S, 6 | 7) => Some(IoCoord {
                    col: self.col_clk - 1,
                    row: self.row_bot(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::N, 0 | 1) => Some(IoCoord {
                    col: self.col_clk + 2,
                    row: self.row_top(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::N, 2 | 3) => Some(IoCoord {
                    col: self.col_clk,
                    row: self.row_top(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::N, 4 | 5) => Some(IoCoord {
                    col: self.col_clk - 1,
                    row: self.row_top(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::N, 6 | 7) => Some(IoCoord {
                    col: self.col_clk - 2,
                    row: self.row_top(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::W, 0 | 1) => Some(IoCoord {
                    col: self.col_left(),
                    row: self.row_mid() + 3,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::W, 2 | 3) => Some(IoCoord {
                    col: self.col_left(),
                    row: self.row_mid() + 1,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::W, 4 | 5) => Some(IoCoord {
                    col: self.col_left(),
                    row: self.row_mid() - 1,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::W, 6 | 7) => Some(IoCoord {
                    col: self.col_left(),
                    row: self.row_mid() - 3,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::E, 0 | 1) => Some(IoCoord {
                    col: self.col_right(),
                    row: self.row_mid(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::E, 2 | 3) => Some(IoCoord {
                    col: self.col_right(),
                    row: self.row_mid() + 2,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::E, 4 | 5) => Some(IoCoord {
                    col: self.col_right(),
                    row: self.row_mid() - 4,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::E, 6 | 7) => Some(IoCoord {
                    col: self.col_right(),
                    row: self.row_mid() - 2,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                _ => None,
            }
        } else {
            match (edge, idx) {
                (Dir::S, 0 | 1) => Some(IoCoord {
                    col: self.col_clk,
                    row: self.row_bot(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::S, 2 | 3) => Some(IoCoord {
                    col: self.col_clk + 1,
                    row: self.row_bot(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::S, 4 | 5) => Some(IoCoord {
                    col: self.col_clk - 2,
                    row: self.row_bot(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::S, 6 | 7) => Some(IoCoord {
                    col: self.col_clk - 1,
                    row: self.row_bot(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::N, 0 | 1) => Some(IoCoord {
                    col: self.col_clk + 1,
                    row: self.row_top(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::N, 2 | 3) => Some(IoCoord {
                    col: self.col_clk,
                    row: self.row_top(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::N, 4 | 5) => Some(IoCoord {
                    col: self.col_clk - 1,
                    row: self.row_top(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::N, 6 | 7) => Some(IoCoord {
                    col: self.col_clk - 2,
                    row: self.row_top(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::W, 0 | 1) => Some(IoCoord {
                    col: self.col_left(),
                    row: self.row_mid() + 2,
                    iob: TileIobId::from_idx((idx % 2) ^ 1),
                }),
                (Dir::W, 2 | 3) => Some(IoCoord {
                    col: self.col_left(),
                    row: self.row_mid() + 1,
                    iob: TileIobId::from_idx((idx % 2) ^ 1),
                }),
                (Dir::W, 4 | 5) => Some(IoCoord {
                    col: self.col_left(),
                    row: self.row_mid() - 1,
                    iob: TileIobId::from_idx((idx % 2) ^ 1),
                }),
                (Dir::W, 6 | 7) => Some(IoCoord {
                    col: self.col_left(),
                    row: self.row_mid() - 2,
                    iob: TileIobId::from_idx((idx % 2) ^ 1),
                }),
                (Dir::E, 0 | 1) => Some(IoCoord {
                    col: self.col_right(),
                    row: self.row_mid(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::E, 2 | 3) => Some(IoCoord {
                    col: self.col_right(),
                    row: self.row_mid() + 1,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::E, 4 | 5) => Some(IoCoord {
                    col: self.col_right(),
                    row: self.row_mid() - 3,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::E, 6 | 7) => Some(IoCoord {
                    col: self.col_right(),
                    row: self.row_mid() - 2,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                _ => None,
            }
        }
    }

    pub fn get_pci_io(&self, edge: Dir) -> [IoCoord; 2] {
        match self.kind {
            GridKind::Spartan3E => match edge {
                Dir::W => [
                    IoCoord {
                        col: self.col_left(),
                        row: self.row_mid() + 1,
                        iob: TileIobId::from_idx(1),
                    },
                    IoCoord {
                        col: self.col_left(),
                        row: self.row_mid() - 1,
                        iob: TileIobId::from_idx(0),
                    },
                ],
                Dir::E => [
                    IoCoord {
                        col: self.col_right(),
                        row: self.row_mid(),
                        iob: TileIobId::from_idx(0),
                    },
                    IoCoord {
                        col: self.col_right(),
                        row: self.row_mid() - 2,
                        iob: TileIobId::from_idx(1),
                    },
                ],
                _ => unreachable!(),
            },
            GridKind::Spartan3A | GridKind::Spartan3ADsp => match edge {
                Dir::W => [
                    IoCoord {
                        col: self.col_left(),
                        row: self.row_mid() + 1,
                        iob: TileIobId::from_idx(0),
                    },
                    IoCoord {
                        col: self.col_left(),
                        row: self.row_mid() - 2,
                        iob: TileIobId::from_idx(1),
                    },
                ],
                Dir::E => [
                    IoCoord {
                        col: self.col_right(),
                        row: self.row_mid() + 1,
                        iob: TileIobId::from_idx(0),
                    },
                    IoCoord {
                        col: self.col_right(),
                        row: self.row_mid() - 2,
                        iob: TileIobId::from_idx(1),
                    },
                ],
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
}
