use prjcombine_entity::EntityId;
use prjcombine_int::db::{BelId, BelInfo, BelNaming};
use prjcombine_int::grid::{ColId, Coord, DieId, ExpandedGrid, ExpandedTileNode, RowId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod expand;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub columns: usize,
    pub rows: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct Io<'a> {
    pub coord: IoCoord,
    pub name: &'a str,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct IoCoord {
    pub col: ColId,
    pub row: RowId,
    pub bel: BelId,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BondPin {
    Io(IoCoord),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
}

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub egrid: ExpandedGrid<'a>,
}

impl Grid {
    pub fn col_lio(&self) -> ColId {
        ColId::from_idx(0)
    }

    pub fn col_rio(&self) -> ColId {
        ColId::from_idx(self.columns - 1)
    }

    pub fn col_mid(&self) -> ColId {
        ColId::from_idx(self.columns / 2)
    }

    pub fn row_bio(&self) -> RowId {
        RowId::from_idx(0)
    }

    pub fn row_tio(&self) -> RowId {
        RowId::from_idx(self.rows - 1)
    }

    pub fn row_mid(&self) -> RowId {
        RowId::from_idx(self.rows / 2)
    }
}

impl<'a> ExpandedDevice<'a> {
    pub fn get_io_bel(
        &'a self,
        coord: Coord,
        bel: BelId,
    ) -> Option<(&'a ExpandedTileNode, &'a BelInfo, &'a BelNaming, &'a str)> {
        let die = self.egrid.die(DieId::from_idx(0));
        let node = die.tile(coord).nodes.first()?;
        let nk = &self.egrid.db.nodes[node.kind];
        let naming = &self.egrid.db.node_namings[node.naming];
        Some((node, &nk.bels[bel], &naming.bels[bel], &node.bels[bel]))
    }

    pub fn get_io(&'a self, coord: Coord, bel: BelId) -> Io<'a> {
        let (_, _, _, name) = self.get_io_bel(coord, bel).unwrap();
        let coord = IoCoord {
            col: coord.0,
            row: coord.1,
            bel,
        };
        Io { coord, name }
    }

    pub fn get_bonded_ios(&'a self) -> Vec<Io<'a>> {
        let mut res = vec![];
        let die = self.egrid.die(DieId::from_idx(0));
        for col in die.cols() {
            if col == self.grid.col_lio() || col == self.grid.col_rio() {
                continue;
            }
            for bel in [3, 2, 1, 0] {
                res.push(self.get_io((col, self.grid.row_tio()), BelId::from_idx(bel)));
            }
        }
        for row in die.rows().rev() {
            if row == self.grid.row_bio() || row == self.grid.row_tio() {
                continue;
            }
            for bel in [3, 2, 1, 0] {
                res.push(self.get_io((self.grid.col_rio(), row), BelId::from_idx(bel)));
            }
        }
        for col in die.cols().rev() {
            if col == self.grid.col_lio() || col == self.grid.col_rio() {
                continue;
            }
            for bel in [0, 1, 2, 3] {
                res.push(self.get_io((col, self.grid.row_bio()), BelId::from_idx(bel)));
            }
        }
        for row in die.rows() {
            if row == self.grid.row_bio() || row == self.grid.row_tio() {
                continue;
            }
            for bel in [0, 1, 2, 3] {
                res.push(self.get_io((self.grid.col_lio(), row), BelId::from_idx(bel)));
            }
        }
        res
    }
}
