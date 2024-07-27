use prjcombine_int::db::{BelId, BelInfo, BelNaming};
use prjcombine_int::grid::{ColId, DieId, ExpandedGrid, ExpandedTileNode, RowId};
use prjcombine_virtex_bitstream::{BitTile, BitstreamGeom};
use unnamed_entity::{EntityId, EntityVec};

use crate::grid::{Grid, IoCoord, TileIobId};

#[derive(Copy, Clone, Debug)]
pub struct Io<'a> {
    pub coord: IoCoord,
    pub name: &'a str,
}

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub egrid: ExpandedGrid<'a>,
    pub bs_geom: BitstreamGeom,
    pub spine_frame: usize,
    pub spine_framebit: usize,
    pub col_frame: EntityVec<ColId, usize>,
    pub row_framebit: EntityVec<RowId, usize>,
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
        Io { coord, name }
    }

    pub fn get_bonded_ios(&'a self) -> Vec<Io<'a>> {
        let mut res = vec![];
        let die = self.egrid.die(DieId::from_idx(0));
        for col in die.cols() {
            if col == self.grid.col_lio() || col == self.grid.col_rio() {
                continue;
            }
            for iob in [3, 2, 1, 0] {
                res.push(self.get_io(IoCoord {
                    col,
                    row: self.grid.row_tio(),
                    iob: TileIobId::from_idx(iob),
                }));
            }
        }
        for row in die.rows().rev() {
            if row == self.grid.row_bio() || row == self.grid.row_tio() {
                continue;
            }
            for iob in [3, 2, 1, 0] {
                res.push(self.get_io(IoCoord {
                    col: self.grid.col_rio(),
                    row,
                    iob: TileIobId::from_idx(iob),
                }));
            }
        }
        for col in die.cols().rev() {
            if col == self.grid.col_lio() || col == self.grid.col_rio() {
                continue;
            }
            for iob in [0, 1, 2, 3] {
                res.push(self.get_io(IoCoord {
                    col,
                    row: self.grid.row_bio(),
                    iob: TileIobId::from_idx(iob),
                }));
            }
        }
        for row in die.rows() {
            if row == self.grid.row_bio() || row == self.grid.row_tio() {
                continue;
            }
            for iob in [0, 1, 2, 3] {
                res.push(self.get_io(IoCoord {
                    col: self.grid.col_lio(),
                    row,
                    iob: TileIobId::from_idx(iob),
                }));
            }
        }
        res
    }

    pub fn btile_main(&self, col: ColId, row: RowId) -> BitTile {
        let width = if col == self.grid.col_lio() {
            7
        } else if col == self.grid.col_rio() {
            8
        } else {
            12
        };
        let height = if row == self.grid.row_bio() || row == self.grid.row_tio() {
            28
        } else {
            34
        };

        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            width,
            self.row_framebit[row],
            height,
            false,
        )
    }

    pub fn btile_spine(&self, row: RowId) -> BitTile {
        let width = 1;
        let height = if row == self.grid.row_bio() || row == self.grid.row_tio() {
            28
        } else {
            34
        };

        BitTile::Main(
            DieId::from_idx(0),
            self.spine_frame,
            width,
            self.row_framebit[row],
            height,
            false,
        )
    }

    pub fn btile_hclk(&self, col: ColId) -> BitTile {
        let width = if col == self.grid.col_lio() {
            7
        } else if col == self.grid.col_rio() {
            8
        } else {
            12
        };
        let height = 4;

        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            width,
            self.spine_framebit,
            height,
            false,
        )
    }
}
