use prjcombine_int::db::BelId;
use prjcombine_int::grid::{ColId, ExpandedGrid, RowId};
use prjcombine_virtex_bitstream::BitstreamGeom;
use std::collections::BTreeSet;

use crate::grid::{DisabledPart, Grid};

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub disabled: &'a BTreeSet<DisabledPart>,
    pub egrid: ExpandedGrid<'a>,
    pub bonded_ios: Vec<((ColId, RowId), BelId)>,
    pub bs_geom: BitstreamGeom,
}
