use std::collections::BTreeMap;

use prjcombine_interconnect::{db::RegionSlotId, dir::DirHV, grid::{CellCoord, ExpandedGrid, Rect}};

use crate::chip::Chip;

pub const REGION_PCLK: RegionSlotId = RegionSlotId::from_idx_const(0);

pub struct ExpandedDevice<'a> {
    pub chip: &'a Chip,
    pub egrid: ExpandedGrid<'a>,
    pub holes: Vec<Rect>,
    pub config: CellCoord,
    pub plls: BTreeMap<DirHV, CellCoord>,
    pub dqs: BTreeMap<CellCoord, CellCoord>,
}

impl ExpandedDevice<'_> {
    pub fn is_in_hole(&self, cell: CellCoord) -> bool {
        for hole in &self.holes {
            if hole.contains(cell.col, cell.row) {
                return true;
            }
        }
        false
    }
}