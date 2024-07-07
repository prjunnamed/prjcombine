use prjcombine_int::db::{BelId, BelInfo, BelNaming};
use prjcombine_int::grid::{ColId, Coord, DieId, ExpandedGrid, ExpandedTileNode, Rect, RowId};
use prjcombine_virtex_bitstream::{BitTile, BitstreamGeom};
use serde::{Deserialize, Serialize};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::grid::{ColumnIoKind, Grid, GridKind, IoCoord, TileIobId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IoDiffKind {
    P(TileIobId),
    N(TileIobId),
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IoPadKind {
    None,
    Input,
    Io,
    Clk,
}

#[derive(Clone, Copy, Debug)]
pub struct Io<'a> {
    pub coord: IoCoord,
    pub bank: u32,
    pub diff: IoDiffKind,
    pub pad_kind: IoPadKind,
    pub name: &'a str,
    pub is_vref: bool,
}

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub egrid: ExpandedGrid<'a>,
    pub bonded_ios: Vec<IoCoord>,
    pub bs_geom: BitstreamGeom,
    pub holes: Vec<Rect>,
    pub clkv_frame: usize,
    pub spine_frame: usize,
    pub lterm_frame: usize,
    pub rterm_frame: usize,
    pub col_frame: EntityVec<ColId, usize>,
    pub bram_frame: EntityPartVec<ColId, usize>,
}

impl<'a> ExpandedDevice<'a> {
    pub fn is_in_hole(&self, col: ColId, row: RowId) -> bool {
        for hole in &self.holes {
            if hole.contains(col, row) {
                return true;
            }
        }
        false
    }
    pub fn get_io_node(&'a self, coord: Coord) -> Option<&'a ExpandedTileNode> {
        self.egrid.find_node(DieId::from_idx(0), coord, |x| {
            self.egrid.db.nodes.key(x.kind).starts_with("IOI")
        })
    }

    pub fn get_io_bel(
        &'a self,
        coord: IoCoord,
    ) -> Option<(&'a ExpandedTileNode, &'a BelInfo, &'a BelNaming, &'a str)> {
        let node = self.get_io_node((coord.col, coord.row))?;
        let nk = &self.egrid.db.nodes[node.kind];
        let naming = &self.egrid.db.node_namings[node.naming];
        let bel = BelId::from_idx(coord.iob.to_idx());
        Some((node, &nk.bels[bel], &naming.bels[bel], &node.bels[bel]))
    }

    pub fn get_io(&'a self, coord: IoCoord) -> Io<'a> {
        let (_, _, _, name) = self.get_io_bel(coord).unwrap();
        let bank = match self.grid.kind {
            GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX | GridKind::Spartan3 => {
                if coord.row == self.grid.row_top() {
                    if coord.col < self.grid.col_clk {
                        0
                    } else {
                        1
                    }
                } else if coord.col == self.grid.col_right() {
                    if coord.row < self.grid.row_mid() {
                        3
                    } else {
                        2
                    }
                } else if coord.row == self.grid.row_bot() {
                    if coord.col < self.grid.col_clk {
                        5
                    } else {
                        4
                    }
                } else if coord.col == self.grid.col_left() {
                    if coord.row < self.grid.row_mid() {
                        6
                    } else {
                        7
                    }
                } else {
                    unreachable!()
                }
            }
            GridKind::Spartan3E | GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                if coord.row == self.grid.row_top() {
                    0
                } else if coord.col == self.grid.col_right() {
                    1
                } else if coord.row == self.grid.row_bot() {
                    2
                } else if coord.col == self.grid.col_left() {
                    3
                } else {
                    unreachable!()
                }
            }
        };
        let diff = match self.grid.kind {
            GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => {
                if matches!(
                    self.grid.columns[coord.col].io,
                    ColumnIoKind::SingleLeftAlt | ColumnIoKind::SingleRightAlt
                ) {
                    match coord.iob.to_idx() {
                        0 => IoDiffKind::None,
                        1 => IoDiffKind::P(TileIobId::from_idx(2)),
                        2 => IoDiffKind::N(TileIobId::from_idx(1)),
                        3 => IoDiffKind::None,
                        _ => unreachable!(),
                    }
                } else {
                    match coord.iob.to_idx() {
                        0 => IoDiffKind::P(TileIobId::from_idx(1)),
                        1 => IoDiffKind::N(TileIobId::from_idx(0)),
                        2 => IoDiffKind::P(TileIobId::from_idx(3)),
                        3 => IoDiffKind::N(TileIobId::from_idx(2)),
                        _ => unreachable!(),
                    }
                }
            }
            GridKind::Spartan3 => {
                if coord.col == self.grid.col_left() {
                    match coord.iob.to_idx() {
                        0 => IoDiffKind::N(TileIobId::from_idx(1)),
                        1 => IoDiffKind::P(TileIobId::from_idx(0)),
                        2 => IoDiffKind::None,
                        _ => unreachable!(),
                    }
                } else {
                    match coord.iob.to_idx() {
                        0 => IoDiffKind::P(TileIobId::from_idx(1)),
                        1 => IoDiffKind::N(TileIobId::from_idx(0)),
                        2 => IoDiffKind::None,
                        _ => unreachable!(),
                    }
                }
            }
            GridKind::Spartan3E => match coord.iob.to_idx() {
                0 => IoDiffKind::P(TileIobId::from_idx(1)),
                1 => IoDiffKind::N(TileIobId::from_idx(0)),
                2 => IoDiffKind::None,
                _ => unreachable!(),
            },
            GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                if coord.row == self.grid.row_top() || coord.col == self.grid.col_left() {
                    match coord.iob.to_idx() {
                        0 => IoDiffKind::N(TileIobId::from_idx(1)),
                        1 => IoDiffKind::P(TileIobId::from_idx(0)),
                        2 => IoDiffKind::None,
                        _ => unreachable!(),
                    }
                } else {
                    match coord.iob.to_idx() {
                        0 => IoDiffKind::P(TileIobId::from_idx(1)),
                        1 => IoDiffKind::N(TileIobId::from_idx(0)),
                        2 => IoDiffKind::None,
                        _ => unreachable!(),
                    }
                }
            }
        };
        let pad_kind = if name.starts_with("PAD") {
            IoPadKind::Io
        } else if name.starts_with("IPAD") {
            IoPadKind::Input
        } else if name.starts_with("CLK") {
            IoPadKind::Clk
        } else {
            IoPadKind::None
        };
        Io {
            coord,
            bank,
            diff,
            pad_kind,
            name,
            is_vref: self.grid.vref.contains(&coord),
        }
    }

    pub fn get_bonded_ios(&'a self) -> Vec<Io<'a>> {
        let mut res = vec![];
        for &coord in &self.bonded_ios {
            res.push(self.get_io(coord));
        }
        res
    }

    pub fn btile_main(&self, col: ColId, row: RowId) -> BitTile {
        let (width, height) = if self.grid.kind.is_virtex2() {
            (22, 80)
        } else {
            (19, 64)
        };
        let bit = 16 + height * row.to_idx();
        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            width,
            bit,
            height,
            false,
        )
    }

    pub fn btile_bram(&self, col: ColId, row: RowId) -> BitTile {
        let (width, height, height_single) = if self.grid.kind.is_virtex2() {
            (64, 80 * 4, 80)
        } else {
            (19 * 4, 64 * 4, 64)
        };
        let bit = 16 + height_single * row.to_idx();
        BitTile::Main(
            DieId::from_idx(0),
            self.bram_frame[col],
            width,
            bit,
            height,
            false,
        )
    }

    pub fn btile_lrterm(&self, col: ColId, row: RowId) -> BitTile {
        let (width, height) = if self.grid.kind.is_virtex2() {
            (4, 80)
        } else {
            (2, 64)
        };
        let bit = 16 + height * row.to_idx();
        let frame = if col == self.grid.col_left() {
            self.lterm_frame
        } else if col == self.grid.col_right() {
            self.rterm_frame
        } else {
            unreachable!()
        };
        BitTile::Main(DieId::from_idx(0), frame, width, bit, height, false)
    }

    pub fn btile_btterm(&self, col: ColId, row: RowId) -> BitTile {
        let (width, height) = if self.grid.kind.is_virtex2() {
            (22, 80)
        } else {
            (19, 64)
        };
        let bit = if row == self.grid.row_bot() {
            if self.grid.kind.is_virtex2() {
                4
            } else if !self.grid.kind.is_spartan3a() {
                7
            } else {
                0
            }
        } else if row == self.grid.row_top() {
            16 + height * self.grid.rows.len()
        } else {
            unreachable!()
        };
        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            width,
            bit,
            if self.grid.kind.is_virtex2() {
                12
            } else if !self.grid.kind.is_spartan3a() {
                5
            } else {
                6
            },
            false,
        )
    }

    pub fn btile_spine(&self, row: RowId) -> BitTile {
        let (width, height) = if self.grid.kind.is_virtex2() {
            (4, 80)
        } else if self.grid.has_ll || self.grid.kind.is_spartan3a() {
            (2, 64)
        } else {
            (1, 64)
        };
        let bit = 16 + height * row.to_idx();
        BitTile::Main(
            DieId::from_idx(0),
            self.spine_frame,
            width,
            bit,
            height,
            false,
        )
    }

    pub fn btile_clkv(&self, col: ColId, row: RowId) -> BitTile {
        assert!(!self.grid.kind.is_virtex2());
        let bit = 16 + 64 * row.to_idx();
        BitTile::Main(
            DieId::from_idx(0),
            self.clkv_frame + if col < self.grid.col_clk { 0 } else { 1 },
            1,
            bit,
            64,
            false,
        )
    }

    pub fn btile_btspine(&self, row: RowId) -> BitTile {
        let (width, height) = if self.grid.kind.is_virtex2() {
            (4, 80)
        } else if self.grid.has_ll || self.grid.kind.is_spartan3a() {
            (2, 64)
        } else {
            (1, 64)
        };
        let bit = if row == self.grid.row_bot() {
            0
        } else if row == self.grid.row_top() {
            16 + height * self.grid.rows.len()
        } else {
            unreachable!()
        };
        BitTile::Main(DieId::from_idx(0), self.spine_frame, width, bit, 16, false)
    }

    pub fn btile_llv_b(&self, col: ColId) -> BitTile {
        assert_eq!(self.grid.kind, GridKind::Spartan3E);
        assert!(self.grid.has_ll);
        let bit = self.grid.rows_hclk.len() / 2;
        BitTile::Main(DieId::from_idx(0), self.col_frame[col], 19, bit, 1, false)
    }

    pub fn btile_llv_t(&self, col: ColId) -> BitTile {
        assert_eq!(self.grid.kind, GridKind::Spartan3E);
        assert!(self.grid.has_ll);
        let bit = 16 + self.grid.rows.len() * 64 + 11 + self.grid.rows_hclk.len() / 2;
        BitTile::Main(DieId::from_idx(0), self.col_frame[col], 19, bit, 2, false)
    }

    pub fn btile_llv(&self, col: ColId) -> BitTile {
        assert!(self.grid.kind.is_spartan3a());
        assert!(self.grid.has_ll);
        let bit = 16 + self.grid.rows.len() * 64 + 8;
        BitTile::Main(DieId::from_idx(0), self.col_frame[col], 19, bit, 3, false)
    }

    pub fn btile_hclk(&self, col: ColId, row: RowId) -> BitTile {
        let (width, height) = if self.grid.kind.is_virtex2() {
            (22, 80)
        } else {
            (19, 64)
        };
        let hclk_idx = self
            .grid
            .rows_hclk
            .iter()
            .position(|&(hrow, _, _)| hrow == row)
            .unwrap();
        let bit = if row <= self.grid.row_mid() {
            if self.grid.kind.is_spartan3a() {
                11 + hclk_idx
            } else {
                hclk_idx
            }
        } else {
            let hclk_idx = self.grid.rows_hclk.len() - hclk_idx - 1;
            if self.grid.kind.is_spartan3a() || self.grid.has_ll {
                16 + height * self.grid.rows.len() + 11 + hclk_idx
            } else {
                16 + height * self.grid.rows.len() + 12 + hclk_idx
            }
        };
        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            width,
            bit,
            1,
            false,
        )
    }
}
