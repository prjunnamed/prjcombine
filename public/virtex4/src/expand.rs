use crate::chip::{Chip, ChipKind, DisabledPart, Interposer};
use crate::expanded::ExpandedDevice;
use crate::gtz::GtzDb;
use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::grid::DieId;
use std::collections::BTreeSet;
use prjcombine_entity::EntityVec;

mod virtex4;
mod virtex5;
mod virtex6;
mod virtex7;

pub fn expand_grid<'a>(
    chips: &EntityVec<DieId, &'a Chip>,
    interposer: Option<&'a Interposer>,
    disabled: &BTreeSet<DisabledPart>,
    db: &'a IntDb,
    gdb: &'a GtzDb,
) -> ExpandedDevice<'a> {
    match chips.first().unwrap().kind {
        ChipKind::Virtex4 => virtex4::expand_grid(chips, disabled, db, gdb),
        ChipKind::Virtex5 => virtex5::expand_grid(chips, disabled, db, gdb),
        ChipKind::Virtex6 => virtex6::expand_grid(chips, disabled, db, gdb),
        ChipKind::Virtex7 => virtex7::expand_grid(chips, interposer.unwrap(), disabled, db, gdb),
    }
}
