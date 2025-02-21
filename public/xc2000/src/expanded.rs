use prjcombine_interconnect::grid::{ColId, DieId, ExpandedGrid, RowId};
use prjcombine_xilinx_bitstream::{BitTile, BitstreamGeom};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::chip::{Chip, ChipKind};

pub struct ExpandedDevice<'a> {
    pub chip: &'a Chip,
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
            self.chip.btile_width_main(col),
            self.row_framebit[row],
            self.chip.btile_height_main(row),
            false,
        )
    }

    pub fn btile_llv(&self, col: ColId, row: RowId) -> BitTile {
        let bit = self.llv_framebit[row];
        let height = if self.chip.kind == ChipKind::Xc2000 {
            self.chip.btile_height_brk()
        } else if self.chip.kind.is_xc3000() || row == self.chip.row_mid() {
            self.chip.btile_height_clk()
        } else if row == self.chip.row_qb() || row == self.chip.row_qt() {
            self.chip.btile_height_brk()
        } else {
            unreachable!()
        };
        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            self.chip.btile_width_main(col),
            bit,
            height,
            false,
        )
    }

    pub fn btile_llh(&self, col: ColId, row: RowId) -> BitTile {
        let frame = self.llh_frame[col];
        let width = if self.chip.kind == ChipKind::Xc2000 {
            self.chip.btile_width_brk()
        } else if col == self.chip.col_mid() {
            self.chip.btile_width_clk()
        } else if col == self.chip.col_ql() || col == self.chip.col_qr() {
            self.chip.btile_width_brk()
        } else {
            unreachable!()
        };
        BitTile::Main(
            DieId::from_idx(0),
            frame,
            width,
            self.row_framebit[row],
            self.chip.btile_height_main(row),
            false,
        )
    }
}
