use prjcombine_int::db::{BelId, BelInfo, BelNaming};
use prjcombine_int::grid::{ColId, DieId, ExpandedGrid, ExpandedTileNode, RowId};
use prjcombine_virtex_bitstream::{BitTile, BitstreamGeom};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::grid::{Grid, IoCoord};

#[derive(Copy, Clone, Debug)]
pub struct Io<'a> {
    pub bank: u32,
    pub coord: IoCoord,
    pub name: &'a str,
    pub is_vref: bool,
}

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub egrid: ExpandedGrid<'a>,
    pub bonded_ios: Vec<IoCoord>,
    pub bs_geom: BitstreamGeom,
    pub spine_frame: usize,
    pub col_frame: EntityVec<ColId, usize>,
    pub bram_frame: EntityPartVec<ColId, usize>,
}

impl<'a> ExpandedDevice<'a> {
    pub fn get_io_bel(
        &'a self,
        coord: IoCoord,
    ) -> Option<(&'a ExpandedTileNode, &'a BelInfo, &'a BelNaming, &'a str)> {
        let die = self.egrid.die(DieId::from_idx(0));
        let node = die.tile((coord.col, coord.row)).nodes.first()?;
        let nk = &self.egrid.db.nodes[node.kind];
        let naming = &self.egrid.db.node_namings[node.naming];
        let bel = BelId::from_idx(coord.iob.to_idx());
        Some((node, &nk.bels[bel], &naming.bels[bel], &node.bels[bel]))
    }

    pub fn get_io(&'a self, coord: IoCoord) -> Io<'a> {
        let (_, _, _, name) = self.get_io_bel(coord).unwrap();
        let bank = if coord.row == self.grid.row_tio() {
            if coord.col < self.grid.col_clk() {
                0
            } else {
                1
            }
        } else if coord.col == self.grid.col_rio() {
            if coord.row < self.grid.row_mid() {
                3
            } else {
                2
            }
        } else if coord.row == self.grid.row_bio() {
            if coord.col < self.grid.col_clk() {
                5
            } else {
                4
            }
        } else if coord.col == self.grid.col_lio() {
            if coord.row < self.grid.row_mid() {
                6
            } else {
                7
            }
        } else {
            unreachable!()
        };
        Io {
            coord,
            bank,
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
        let width = if col == self.grid.col_lio() || col == self.grid.col_rio() {
            54
        } else if self.grid.cols_bram.contains(&col) {
            27
        } else {
            48
        };
        let height = 18;

        let bit = height * row.to_idx();
        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            width,
            bit,
            height,
            false,
        )
    }

    pub fn btile_spine(&self, row: RowId) -> BitTile {
        let width = 8;
        let height = 18;

        let bit = height * row.to_idx();
        BitTile::Main(
            DieId::from_idx(0),
            self.spine_frame,
            width,
            bit,
            height,
            false,
        )
    }

    pub fn btile_bram(&self, col: ColId, row: RowId) -> BitTile {
        let width = 64;
        let height = 18;

        let bit = height * row.to_idx();
        BitTile::Main(
            DieId::from_idx(0),
            self.bram_frame[col],
            width,
            bit,
            height * 4,
            false,
        )
    }
}
