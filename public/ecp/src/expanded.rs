use std::collections::BTreeMap;

use prjcombine_interconnect::grid::{CellCoord, ExpandedGrid, Rect};

use crate::{bels, chip::Chip};

pub struct ExpandedDevice<'a> {
    pub chip: &'a Chip,
    pub egrid: ExpandedGrid<'a>,
    pub bel_holes: Vec<Rect>,
    pub dqs: BTreeMap<CellCoord, CellCoord>,
}

impl ExpandedDevice<'_> {
    pub fn is_in_int_hole(&self, cell: CellCoord) -> bool {
        !self.egrid.has_bel(cell.bel(bels::INT))
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
