use prjcombine_int::db::Dir;
use prjcombine_int::grid::{ColId, Coord, RowId, SimpleIoCoord, TileIobId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use unnamed_entity::{EntityId, EntityVec};

use crate::iob::{get_iob_data_b, get_iob_data_l, get_iob_data_r, get_iob_data_t, IobTileData};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GridKind {
    Virtex2,
    Virtex2P,
    Virtex2PX,
    Spartan3,
    FpgaCore,
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
    pub cfg_io: BTreeMap<SharedCfgPin, SimpleIoCoord>,
    pub dci_io: BTreeMap<u32, (SimpleIoCoord, SimpleIoCoord)>,
    pub dci_io_alt: BTreeMap<u32, (SimpleIoCoord, SimpleIoCoord)>,
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
    DoubleRightClk(u8),
}

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

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum DcmPairKind {
    Bot,
    BotSingle,
    Top,
    TopSingle,
    // S3E
    Left,
    Right,
    // S3A
    Bram,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct DcmPair {
    pub kind: DcmPairKind,
    pub col: ColId,
    pub row: RowId,
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

    pub fn bram_row(&self, row: RowId) -> Option<usize> {
        if let Some((b, t)) = self.rows_ram {
            if row <= b || row >= t {
                None
            } else {
                Some((row.to_idx() - (b.to_idx() + 1)) % 4)
            }
        } else {
            if row == self.row_bot() || row == self.row_top() {
                None
            } else {
                Some((row.to_idx() - 1) % 4)
            }
        }
    }

    pub fn get_dcm_pairs(&self) -> Vec<DcmPair> {
        let mut res = vec![];
        if let Some(dcms) = self.dcms {
            if dcms == Dcms::Two {
                if self.kind == GridKind::Spartan3E {
                    res.extend([
                        DcmPair {
                            kind: DcmPairKind::BotSingle,
                            col: self.col_clk,
                            row: self.row_bot() + 1,
                        },
                        DcmPair {
                            kind: DcmPairKind::TopSingle,
                            col: self.col_clk,
                            row: self.row_top() - 1,
                        },
                    ]);
                } else {
                    res.extend([DcmPair {
                        kind: DcmPairKind::Top,
                        col: self.col_clk,
                        row: self.row_top() - 1,
                    }]);
                }
            } else {
                res.extend([
                    DcmPair {
                        kind: DcmPairKind::Bot,
                        col: self.col_clk,
                        row: self.row_bot() + 1,
                    },
                    DcmPair {
                        kind: DcmPairKind::Top,
                        col: self.col_clk,
                        row: self.row_top() - 1,
                    },
                ]);
            }
            if dcms == Dcms::Eight {
                if self.kind == GridKind::Spartan3E {
                    res.extend([
                        DcmPair {
                            kind: DcmPairKind::Left,
                            col: self.col_left() + 9,
                            row: self.row_mid(),
                        },
                        DcmPair {
                            kind: DcmPairKind::Right,
                            col: self.col_right() - 9,
                            row: self.row_mid(),
                        },
                    ]);
                } else {
                    res.extend([
                        DcmPair {
                            kind: DcmPairKind::Bram,
                            col: self.col_left() + 3,
                            row: self.row_mid(),
                        },
                        DcmPair {
                            kind: DcmPairKind::Bram,
                            col: self.col_right() - 6,
                            row: self.row_mid(),
                        },
                    ]);
                }
            }
        }
        res
    }

    pub fn get_clk_io(&self, edge: Dir, idx: usize) -> Option<SimpleIoCoord> {
        if self.kind.is_virtex2() {
            match edge {
                Dir::S => {
                    if self.kind == GridKind::Virtex2PX && matches!(idx, 6 | 7) {
                        return None;
                    }
                    if idx < 4 {
                        Some(SimpleIoCoord {
                            col: self.col_clk,
                            row: self.row_bot(),
                            iob: TileIobId::from_idx(idx),
                        })
                    } else if idx < 8 {
                        Some(SimpleIoCoord {
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
                        Some(SimpleIoCoord {
                            col: self.col_clk,
                            row: self.row_top(),
                            iob: TileIobId::from_idx(idx),
                        })
                    } else if idx < 8 {
                        Some(SimpleIoCoord {
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
        } else if matches!(self.kind, GridKind::Spartan3 | GridKind::FpgaCore) {
            match edge {
                Dir::S => {
                    if idx < 2 {
                        Some(SimpleIoCoord {
                            col: self.col_clk,
                            row: self.row_bot(),
                            iob: TileIobId::from_idx(idx),
                        })
                    } else if idx < 4 {
                        Some(SimpleIoCoord {
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
                        Some(SimpleIoCoord {
                            col: self.col_clk,
                            row: self.row_top(),
                            iob: TileIobId::from_idx(idx),
                        })
                    } else if idx < 4 {
                        Some(SimpleIoCoord {
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
                (Dir::S, 0 | 1) => Some(SimpleIoCoord {
                    col: self.col_clk,
                    row: self.row_bot(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::S, 2 | 3) => Some(SimpleIoCoord {
                    col: self.col_clk + 1,
                    row: self.row_bot(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::S, 4 | 5) => Some(SimpleIoCoord {
                    col: self.col_clk - 3,
                    row: self.row_bot(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::S, 6 | 7) => Some(SimpleIoCoord {
                    col: self.col_clk - 1,
                    row: self.row_bot(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::N, 0 | 1) => Some(SimpleIoCoord {
                    col: self.col_clk + 2,
                    row: self.row_top(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::N, 2 | 3) => Some(SimpleIoCoord {
                    col: self.col_clk,
                    row: self.row_top(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::N, 4 | 5) => Some(SimpleIoCoord {
                    col: self.col_clk - 1,
                    row: self.row_top(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::N, 6 | 7) => Some(SimpleIoCoord {
                    col: self.col_clk - 2,
                    row: self.row_top(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::W, 0 | 1) => Some(SimpleIoCoord {
                    col: self.col_left(),
                    row: self.row_mid() + 3,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::W, 2 | 3) => Some(SimpleIoCoord {
                    col: self.col_left(),
                    row: self.row_mid() + 1,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::W, 4 | 5) => Some(SimpleIoCoord {
                    col: self.col_left(),
                    row: self.row_mid() - 1,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::W, 6 | 7) => Some(SimpleIoCoord {
                    col: self.col_left(),
                    row: self.row_mid() - 3,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::E, 0 | 1) => Some(SimpleIoCoord {
                    col: self.col_right(),
                    row: self.row_mid(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::E, 2 | 3) => Some(SimpleIoCoord {
                    col: self.col_right(),
                    row: self.row_mid() + 2,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::E, 4 | 5) => Some(SimpleIoCoord {
                    col: self.col_right(),
                    row: self.row_mid() - 4,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::E, 6 | 7) => Some(SimpleIoCoord {
                    col: self.col_right(),
                    row: self.row_mid() - 2,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                _ => None,
            }
        } else {
            match (edge, idx) {
                (Dir::S, 0 | 1) => Some(SimpleIoCoord {
                    col: self.col_clk,
                    row: self.row_bot(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::S, 2 | 3) => Some(SimpleIoCoord {
                    col: self.col_clk + 1,
                    row: self.row_bot(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::S, 4 | 5) => Some(SimpleIoCoord {
                    col: self.col_clk - 2,
                    row: self.row_bot(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::S, 6 | 7) => Some(SimpleIoCoord {
                    col: self.col_clk - 1,
                    row: self.row_bot(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::N, 0 | 1) => Some(SimpleIoCoord {
                    col: self.col_clk + 1,
                    row: self.row_top(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::N, 2 | 3) => Some(SimpleIoCoord {
                    col: self.col_clk,
                    row: self.row_top(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::N, 4 | 5) => Some(SimpleIoCoord {
                    col: self.col_clk - 1,
                    row: self.row_top(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::N, 6 | 7) => Some(SimpleIoCoord {
                    col: self.col_clk - 2,
                    row: self.row_top(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::W, 0 | 1) => Some(SimpleIoCoord {
                    col: self.col_left(),
                    row: self.row_mid() + 2,
                    iob: TileIobId::from_idx((idx % 2) ^ 1),
                }),
                (Dir::W, 2 | 3) => Some(SimpleIoCoord {
                    col: self.col_left(),
                    row: self.row_mid() + 1,
                    iob: TileIobId::from_idx((idx % 2) ^ 1),
                }),
                (Dir::W, 4 | 5) => Some(SimpleIoCoord {
                    col: self.col_left(),
                    row: self.row_mid() - 1,
                    iob: TileIobId::from_idx((idx % 2) ^ 1),
                }),
                (Dir::W, 6 | 7) => Some(SimpleIoCoord {
                    col: self.col_left(),
                    row: self.row_mid() - 2,
                    iob: TileIobId::from_idx((idx % 2) ^ 1),
                }),
                (Dir::E, 0 | 1) => Some(SimpleIoCoord {
                    col: self.col_right(),
                    row: self.row_mid(),
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::E, 2 | 3) => Some(SimpleIoCoord {
                    col: self.col_right(),
                    row: self.row_mid() + 1,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::E, 4 | 5) => Some(SimpleIoCoord {
                    col: self.col_right(),
                    row: self.row_mid() - 3,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                (Dir::E, 6 | 7) => Some(SimpleIoCoord {
                    col: self.col_right(),
                    row: self.row_mid() - 2,
                    iob: TileIobId::from_idx(idx % 2),
                }),
                _ => None,
            }
        }
    }

    pub fn get_pci_io(&self, edge: Dir) -> [SimpleIoCoord; 2] {
        match self.kind {
            GridKind::Spartan3E => match edge {
                Dir::W => [
                    SimpleIoCoord {
                        col: self.col_left(),
                        row: self.row_mid() + 1,
                        iob: TileIobId::from_idx(1),
                    },
                    SimpleIoCoord {
                        col: self.col_left(),
                        row: self.row_mid() - 1,
                        iob: TileIobId::from_idx(0),
                    },
                ],
                Dir::E => [
                    SimpleIoCoord {
                        col: self.col_right(),
                        row: self.row_mid(),
                        iob: TileIobId::from_idx(0),
                    },
                    SimpleIoCoord {
                        col: self.col_right(),
                        row: self.row_mid() - 2,
                        iob: TileIobId::from_idx(1),
                    },
                ],
                _ => unreachable!(),
            },
            GridKind::Spartan3A | GridKind::Spartan3ADsp => match edge {
                Dir::W => [
                    SimpleIoCoord {
                        col: self.col_left(),
                        row: self.row_mid() + 1,
                        iob: TileIobId::from_idx(0),
                    },
                    SimpleIoCoord {
                        col: self.col_left(),
                        row: self.row_mid() - 2,
                        iob: TileIobId::from_idx(1),
                    },
                ],
                Dir::E => [
                    SimpleIoCoord {
                        col: self.col_right(),
                        row: self.row_mid() + 1,
                        iob: TileIobId::from_idx(0),
                    },
                    SimpleIoCoord {
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

    pub fn get_iob_data(&self, coord: Coord) -> Option<(IobTileData, usize)> {
        if coord.0 == self.col_left() {
            let kind = self.rows[coord.1];
            if kind == RowIoKind::None {
                None
            } else {
                Some(get_iob_data_l(self.kind, kind))
            }
        } else if coord.0 == self.col_right() {
            let kind = self.rows[coord.1];
            if kind == RowIoKind::None {
                None
            } else {
                Some(get_iob_data_r(self.kind, kind))
            }
        } else if coord.1 == self.row_bot() {
            let kind = self.columns[coord.0].io;
            if kind == ColumnIoKind::None {
                None
            } else {
                Some(get_iob_data_b(self.kind, kind))
            }
        } else if coord.1 == self.row_top() {
            let kind = self.columns[coord.0].io;
            if kind == ColumnIoKind::None {
                None
            } else {
                Some(get_iob_data_t(self.kind, kind))
            }
        } else {
            unreachable!()
        }
    }
}
