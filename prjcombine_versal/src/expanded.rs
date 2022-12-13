use prjcombine_entity::EntityVec;
use prjcombine_int::grid::{DieId, ExpandedGrid};
use std::collections::BTreeSet;

use crate::grid::{DisabledPart, Grid};

pub struct ExpandedDevice<'a> {
    pub grids: EntityVec<DieId, &'a Grid>,
    pub egrid: ExpandedGrid<'a>,
    pub disabled: BTreeSet<DisabledPart>,
}
