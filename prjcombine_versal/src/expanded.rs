use prjcombine_int::grid::{ColId, DieId, ExpandedGrid};
use std::collections::BTreeSet;
use unnamed_entity::EntityVec;

use crate::grid::{DisabledPart, Grid, Interposer};

#[derive(Debug)]
pub struct ExpandedDevice<'a> {
    pub grids: EntityVec<DieId, &'a Grid>,
    pub egrid: ExpandedGrid<'a>,
    pub interposer: &'a Interposer,
    pub disabled: BTreeSet<DisabledPart>,
    pub col_cfrm: EntityVec<DieId, ColId>,
}
