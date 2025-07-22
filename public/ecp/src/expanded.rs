use std::collections::BTreeMap;

use prjcombine_interconnect::{
    db::RegionSlotId,
    grid::{CellCoord, ExpandedGrid, Rect},
};

use crate::{bels, chip::Chip};

pub const REGION_PCLK: RegionSlotId = RegionSlotId::from_idx_const(0);
pub const REGION_SCLK0: RegionSlotId = RegionSlotId::from_idx_const(1);
pub const REGION_SCLK1: RegionSlotId = RegionSlotId::from_idx_const(2);
pub const REGION_SCLK2: RegionSlotId = RegionSlotId::from_idx_const(3);
pub const REGION_SCLK3: RegionSlotId = RegionSlotId::from_idx_const(4);
pub const REGION_SCLK: [RegionSlotId; 4] = [REGION_SCLK0, REGION_SCLK1, REGION_SCLK2, REGION_SCLK3];
pub const REGION_HSDCLK: RegionSlotId = RegionSlotId::from_idx_const(5);
pub const REGION_VSDCLK: RegionSlotId = RegionSlotId::from_idx_const(6);

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
