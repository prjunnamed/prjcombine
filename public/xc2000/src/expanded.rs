use prjcombine_interconnect::grid::{ColId, DieId, ExpandedGrid, RowId};
use prjcombine_xilinx_bitstream::{BitTile, BitstreamGeom};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::grid::{Grid, GridKind};

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub egrid: ExpandedGrid<'a>,
    pub bs_geom: BitstreamGeom,
    pub col_frame: EntityVec<ColId, usize>,
    pub llh_frame: EntityPartVec<ColId, usize>,
    pub row_framebit: EntityVec<RowId, usize>,
    pub llv_framebit: EntityPartVec<RowId, usize>,
}

impl ExpandedDevice<'_> {
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
        let bit = self.llv_framebit[row];
        let height = if self.grid.kind == GridKind::Xc2000 {
            self.grid.btile_height_brk()
        } else if self.grid.kind.is_xc3000() || row == self.grid.row_mid() {
            self.grid.btile_height_clk()
        } else if row == self.grid.row_qb() || row == self.grid.row_qt() {
            self.grid.btile_height_brk()
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
        let frame = self.llh_frame[col];
        let width = if self.grid.kind == GridKind::Xc2000 {
            self.grid.btile_width_brk()
        } else if col == self.grid.col_mid() {
            self.grid.btile_width_clk()
        } else if col == self.grid.col_ql() || col == self.grid.col_qr() {
            self.grid.btile_width_brk()
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
