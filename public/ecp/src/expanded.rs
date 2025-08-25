use std::collections::BTreeMap;

use prjcombine_interconnect::grid::{CellCoord, ColId, ExpandedGrid, Rect, RowId};
use unnamed_entity::{EntityPartVec, EntityVec};

use crate::{bels, chip::Chip};

pub struct ExpandedDevice<'a> {
    pub chip: &'a Chip,
    pub egrid: ExpandedGrid<'a>,
    pub bel_holes: Vec<Rect>,
    pub dqs: BTreeMap<CellCoord, CellCoord>,
    pub frame_len: usize,
    pub frames_num: usize,
    pub clk_frame: usize,
    pub col_frame: EntityVec<ColId, usize>,
    pub col_term_frame: EntityPartVec<ColId, usize>,
    pub row_bit: EntityVec<RowId, usize>,
    pub row_ebr_bit: EntityPartVec<RowId, usize>,
}

impl ExpandedDevice<'_> {
    pub fn is_in_int_hole(&self, cell: CellCoord) -> bool {
        !self.has_bel(cell.bel(bels::INT))
    }

    pub fn is_in_bel_hole(&self, cell: CellCoord) -> bool {
        for hole in &self.bel_holes {
            if hole.contains(cell) {
                return true;
            }
        }
        false
    }
}

impl<'a> std::ops::Deref for ExpandedDevice<'a> {
    type Target = ExpandedGrid<'a>;

    fn deref(&self) -> &Self::Target {
        &self.egrid
    }
}
