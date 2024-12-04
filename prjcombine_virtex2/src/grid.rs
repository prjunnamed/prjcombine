use prjcombine_int::db::Dir;
use prjcombine_int::grid::{ColId, Coord, RowId, SimpleIoCoord, TileIobId};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use unnamed_entity::{EntityId, EntityVec};

use crate::iob::{get_iob_data_b, get_iob_data_l, get_iob_data_r, get_iob_data_t, IobTileData};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Column {
    pub kind: ColumnKind,
    pub io: ColumnIoKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ColumnKind {
    Io,
    Clb,
    Bram,
    BramCont(u8),
    Dsp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum RowIoKind {
    None,
    Single,
    Double(u8),
    Triple(u8),
    Quad(u8),
    DoubleBot(u8),
    DoubleTop(u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
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

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "kind": match self.kind {
                GridKind::Virtex2 => "virtex2",
                GridKind::Virtex2P => "virtex2p",
                GridKind::Virtex2PX => "virtex2px",
                GridKind::Spartan3 => "spartan3",
                GridKind::Spartan3E => "spartan3e",
                GridKind::Spartan3A => "spartan3a",
                GridKind::Spartan3ADsp => "spartan3adsp",
                GridKind::FpgaCore => "fpgacore",
            },
            "columns": Vec::from_iter(self.columns.values().map(|column| {
                json!({
                    "kind": match column.kind {
                        ColumnKind::Io => "IO".to_string(),
                        ColumnKind::Clb => "CLB".to_string(),
                        ColumnKind::Bram => "BRAM".to_string(),
                        ColumnKind::BramCont(i) => format!("BRAM_CONT:{i}"),
                        ColumnKind::Dsp => "DSP".to_string(),
                    },
                    "io": match column.io {
                        ColumnIoKind::None => serde_json::Value::Null,
                        ColumnIoKind::Single => "SINGLE".into(),
                        ColumnIoKind::Double(i) => format!("DOUBLE:{i}").into(),
                        ColumnIoKind::Triple(i) => format!("TRIPLE:{i}").into(),
                        ColumnIoKind::Quad(i) => format!("QUAD:{i}").into(),
                        ColumnIoKind::SingleLeft => "SINGLE_LEFT".into(),
                        ColumnIoKind::SingleRight => "SINGLE_RIGHT".into(),
                        ColumnIoKind::SingleLeftAlt => "SINGLE_LEFT_ALT".into(),
                        ColumnIoKind::SingleRightAlt => "SINGLE_RIGHT_ALT".into(),
                        ColumnIoKind::DoubleLeft(i) => format!("DOUBLE_LEFT:{i}").into(),
                        ColumnIoKind::DoubleRight(i) => format!("DOUBLE_RIGHT:{i}").into(),
                        ColumnIoKind::DoubleRightClk(i) => format!("DOUBLE_RIGHT_CLK:{i}").into(),
                    },
                })
            })),
            "cols_clkv": self.cols_clkv,
            "cols_gt": Vec::from_iter(self.cols_gt.iter().map(|(col, (bank_b, bank_t))| json!({
                "column": col,
                "bank_b": bank_b,
                "bank_t": bank_t,
            }))),
            "rows": Vec::from_iter(self.rows.values().map(|io| match io {
                RowIoKind::None => serde_json::Value::Null,
                RowIoKind::Single => "SINGLE".into(),
                RowIoKind::Double(i) => format!("DOUBLE:{i}").into(),
                RowIoKind::Triple(i) => format!("TRIPLE:{i}").into(),
                RowIoKind::Quad(i) => format!("QUAD:{i}").into(),
                RowIoKind::DoubleBot(i) => format!("DOUBLE_BOT:{i}").into(),
                RowIoKind::DoubleTop(i) => format!("DOUBLE_TOP:{i}").into(),
            })),
            "rows_ram": self.rows_ram,
            "rows_hclk": self.rows_hclk,
            "row_pci": self.row_pci,
            "holes_ppc": self.holes_ppc,
            "dcms": match self.dcms {
                None => serde_json::Value::Null,
                Some(dcms) => match dcms {
                    Dcms::Two => 2,
                    Dcms::Four => 4,
                    Dcms::Eight => 8,
                }.into()
            },
            "has_ll": self.has_ll,
            "cfg_io": serde_json::Map::from_iter(self.cfg_io.iter().map(|(k, io)| {
                (match k {
                    SharedCfgPin::Data(i) => format!("D{i}"),
                    SharedCfgPin::Addr(i) => format!("A{i}"),
                    SharedCfgPin::CsiB => "CSI_B".to_string(),
                    SharedCfgPin::CsoB => "CSO_B".to_string(),
                    SharedCfgPin::RdWrB => "RDWR_B".to_string(),
                    SharedCfgPin::Dout => "DOUT".to_string(),
                    SharedCfgPin::InitB => "INIT_B".to_string(),
                    SharedCfgPin::Cclk => "CCLK".to_string(),
                    SharedCfgPin::M0 => "M0".to_string(),
                    SharedCfgPin::M1 => "M1".to_string(),
                    SharedCfgPin::M2 => "M2".to_string(),
                    SharedCfgPin::Ldc0 => "LDC0".to_string(),
                    SharedCfgPin::Ldc1 => "LDC1".to_string(),
                    SharedCfgPin::Ldc2 => "LDC2".to_string(),
                    SharedCfgPin::Hdc => "HDC".to_string(),
                    SharedCfgPin::HswapEn => "HSWAP_EN".to_string(),
                    SharedCfgPin::Awake => "AWAKE".to_string(),
                }, io.to_string().into())
            })),
            "dci_io": serde_json::Map::from_iter(self.dci_io.iter().map(|(k, (io_a, io_b))| {
                (k.to_string(), json!({
                    "vrp": io_a.to_string(),
                    "vrn": io_b.to_string(),
                }))
            })),
            "dci_io_alt": serde_json::Map::from_iter(self.dci_io_alt.iter().map(|(k, (io_a, io_b))| {
                (k.to_string(), json!({
                    "vrp": io_a.to_string(),
                    "vrn": io_b.to_string(),
                }))
            })),
        })
    }
}

impl std::fmt::Display for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tKIND: {k:?}", k = self.kind)?;
        writeln!(f, "\tCOLS:")?;
        for (col, cd) in &self.columns {
            if let Some((cl, cr)) = self.cols_clkv {
                if col == cl {
                    writeln!(f, "\t\t--- clock left spine")?;
                }
                if col == cr {
                    writeln!(f, "\t\t--- clock right spine")?;
                }
            }
            if col == self.col_clk {
                writeln!(f, "\t\t--- clock spine")?;
            }
            write!(f, "\t\tX{c}: ", c = col.to_idx())?;
            match cd.kind {
                ColumnKind::Io => write!(f, "IO")?,
                ColumnKind::Clb => write!(f, "CLB   ")?,
                ColumnKind::Bram => write!(f, "BRAM  ")?,
                ColumnKind::BramCont(i) => write!(f, "BRAM.{i}")?,
                ColumnKind::Dsp => write!(f, "DSP   ")?,
            }
            match cd.io {
                ColumnIoKind::None => (),
                ColumnIoKind::Single => write!(f, " IO: 1")?,
                ColumnIoKind::Double(i) => write!(f, " IO: 2.{i}")?,
                ColumnIoKind::Triple(i) => write!(f, " IO: 3.{i}")?,
                ColumnIoKind::Quad(i) => write!(f, " IO: 4.{i}")?,
                ColumnIoKind::SingleLeft => write!(f, " IO: 1L")?,
                ColumnIoKind::SingleRight => write!(f, " IO: 1R")?,
                ColumnIoKind::SingleLeftAlt => write!(f, " IO: 1LA")?,
                ColumnIoKind::SingleRightAlt => write!(f, " IO: 1RA")?,
                ColumnIoKind::DoubleLeft(i) => write!(f, " IO: 2L.{i}")?,
                ColumnIoKind::DoubleRight(i) => write!(f, " IO: 2R.{i}")?,
                ColumnIoKind::DoubleRightClk(i) => write!(f, " IO: 2R.CLK.{i}")?,
            }
            if let Some(&(bb, bt)) = self.cols_gt.get(&col) {
                write!(f, " GT: BOT {bb} TOP {bt}")?;
            }
            writeln!(f,)?;
        }
        let mut clkv_idx = 0;
        writeln!(f, "\tROWS:")?;
        for (row, rd) in &self.rows {
            if row == self.rows_hclk[clkv_idx].0 {
                writeln!(f, "\t\t--- clock row")?;
            }
            if row == self.rows_hclk[clkv_idx].2 {
                writeln!(f, "\t\t--- clock break")?;
                clkv_idx += 1;
            }
            if Some(row) == self.row_pci {
                writeln!(f, "\t\t--- PCI row")?;
            }
            if row == self.row_mid() {
                writeln!(f, "\t\t--- spine row")?;
            }
            write!(f, "\t\tY{r}: ", r = row.to_idx())?;
            match rd {
                RowIoKind::None => (),
                RowIoKind::Single => write!(f, " IO: 1")?,
                RowIoKind::Double(i) => write!(f, " IO: 2.{i}")?,
                RowIoKind::Triple(i) => write!(f, " IO: 3.{i}")?,
                RowIoKind::Quad(i) => write!(f, " IO: 4.{i}")?,
                RowIoKind::DoubleBot(i) => write!(f, " IO: 2B.{i}")?,
                RowIoKind::DoubleTop(i) => write!(f, " IO: 2T.{i}")?,
            }
            if let Some((rb, rt)) = self.rows_ram {
                if row == rb {
                    write!(f, " BRAM BOT TERM")?;
                }
                if row == rt {
                    write!(f, " BRAM TOP TERM")?;
                }
            }
            writeln!(f,)?;
        }
        for &(col, row) in &self.holes_ppc {
            writeln!(
                f,
                "\tPPC: X{xl}:X{xr} Y{yb}:Y{yt}",
                xl = col.to_idx(),
                xr = col.to_idx() + 10,
                yb = row.to_idx(),
                yt = row.to_idx() + 16
            )?;
        }
        if let Some(dcms) = self.dcms {
            writeln!(f, "\tDCMS: {dcms:?}")?;
        }
        if self.has_ll {
            writeln!(f, "\tHAS LL SPLITTERS")?;
        }
        writeln!(f, "\tHAS_SMALL_INT: {v:?}", v = self.has_small_int)?;
        writeln!(f, "\tCFG PINS:")?;
        for (k, v) in &self.cfg_io {
            writeln!(f, "\t\t{k:?}: {v}")?;
        }
        if !self.dci_io.is_empty() {
            writeln!(f, "\tDCI:")?;
            for k in 0..8 {
                writeln!(f, "\t\t{k}:")?;
                if let Some(&(vp, vn)) = self.dci_io.get(&k) {
                    writeln!(f, "\t\t\tVP: {vp}")?;
                    writeln!(f, "\t\t\tVN: {vn}")?;
                }
                if let Some(&(vp, vn)) = self.dci_io_alt.get(&k) {
                    writeln!(f, "\t\t\tALT VP: {vp}")?;
                    writeln!(f, "\t\t\tALT VN: {vn}")?;
                }
            }
        }
        Ok(())
    }
}
