use crate::expanded::ExpandedDevice;
use crate::grid::{DisabledPart, ExtraDie, Grid, GridKind};
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
    grid_master: DieId,
    extras: &[ExtraDie],
    disabled: &BTreeSet<DisabledPart>,
    db: &'a IntDb,
) -> ExpandedDevice<'a> {
    match grids[grid_master].kind {
        GridKind::Virtex4 => virtex4::expand_grid(grids, grid_master, extras, disabled, db),
        GridKind::Virtex5 => virtex5::expand_grid(grids, grid_master, extras, disabled, db),
        GridKind::Virtex6 => virtex6::expand_grid(grids, grid_master, extras, disabled, db),
        GridKind::Virtex7 => virtex7::expand_grid(grids, grid_master, extras, disabled, db),
    }
}
