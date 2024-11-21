use crate::expanded::ExpandedDevice;
use crate::grid::{DisabledPart, Grid, GridKind, Interposer};
use prjcombine_int::db::IntDb;
use prjcombine_int::grid::DieId;
use std::collections::BTreeSet;
use unnamed_entity::EntityVec;

mod virtex4;
mod virtex5;
mod virtex6;
mod virtex7;

pub fn expand_grid<'a>(
    grids: &EntityVec<DieId, &'a Grid>,
    interposer: Option<&'a Interposer>,
    disabled: &BTreeSet<DisabledPart>,
    db: &'a IntDb,
) -> ExpandedDevice<'a> {
    match grids.first().unwrap().kind {
        GridKind::Virtex4 => virtex4::expand_grid(grids, disabled, db),
        GridKind::Virtex5 => virtex5::expand_grid(grids, disabled, db),
        GridKind::Virtex6 => virtex6::expand_grid(grids, disabled, db),
        GridKind::Virtex7 => virtex7::expand_grid(grids, interposer.unwrap(), disabled, db),
    }
}
