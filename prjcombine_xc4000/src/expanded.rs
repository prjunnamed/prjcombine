use prjcombine_int::{
    db::{BelId, BelInfo, BelNaming},
    grid::{ColId, DieId, ExpandedGrid, ExpandedTileNode, RowId},
};
use prjcombine_virtex_bitstream::{BitTile, BitstreamGeom};
use unnamed_entity::{EntityId, EntityVec};

use crate::grid::{Grid, IoCoord};

#[derive(Debug)]
pub struct Io {
    pub name: String,
    pub crd: IoCoord,
}

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub egrid: ExpandedGrid<'a>,
    pub io: Vec<Io>,
    pub bs_geom: BitstreamGeom,
    pub spine_frame: usize,
    pub quarter_frame: Option<(usize, usize)>,
    pub col_frame: EntityVec<ColId, usize>,
    pub spine_framebit: usize,
    pub quarter_framebit: Option<(usize, usize)>,
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

    pub fn btile_main(&self, col: ColId, row: RowId) -> BitTile {
        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            self.grid.btile_width_main(col),
            self.row_framebit[row],
            self.grid.btile_height_main(row),
            false,
        )
    }

    pub fn btile_llv(&self, col: ColId, row: RowId) -> BitTile {
        let (bit, height) = if row == self.grid.row_mid() {
            (self.spine_framebit, self.grid.btile_height_clk())
        } else if row == self.grid.row_qb() {
            (
                self.quarter_framebit.unwrap().0,
                self.grid.btile_height_brk(),
            )
        } else if row == self.grid.row_qt() {
            (
                self.quarter_framebit.unwrap().1,
                self.grid.btile_height_brk(),
            )
        } else {
            unreachable!()
        };
        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            self.grid.btile_width_main(col),
            bit,
            height,
            false,
        )
    }

    pub fn btile_llh(&self, col: ColId, row: RowId) -> BitTile {
        let (frame, width) = if col == self.grid.col_mid() {
            (self.spine_frame, self.grid.btile_width_clk())
        } else if col == self.grid.col_ql() {
            (self.quarter_frame.unwrap().0, self.grid.btile_width_brk())
        } else if col == self.grid.col_qr() {
            (self.quarter_frame.unwrap().1, self.grid.btile_width_brk())
        } else {
            unreachable!()
        };
        BitTile::Main(
            DieId::from_idx(0),
            frame,
            width,
            self.row_framebit[row],
            self.grid.btile_height_main(row),
            false,
        )
    }
}
