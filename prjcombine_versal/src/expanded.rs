use prjcombine_int::grid::{DieId, ExpandedGrid};
use std::collections::BTreeSet;
use unnamed_entity::EntityVec;

use crate::grid::{DisabledPart, Grid};

pub struct ExpandedDevice<'a> {
    pub grids: EntityVec<DieId, &'a Grid>,
    pub egrid: ExpandedGrid<'a>,
    pub disabled: BTreeSet<DisabledPart>,
}
