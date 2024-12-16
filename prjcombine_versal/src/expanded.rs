use prjcombine_int::grid::{ColId, DieId, ExpandedGrid, RowId};
use std::collections::{BTreeSet, HashMap};
use unnamed_entity::{entity_id, EntityBitVec, EntityVec};

entity_id! {
    pub id UbumpId u8;
}

pub type UbumpLoc = (DieId, ColId, RowId, UbumpId);

use crate::grid::{DisabledPart, Grid, Interposer};

#[derive(Debug)]
pub struct SllConns {
    pub conns: EntityVec<UbumpId, Option<UbumpLoc>>,
    pub cursed: EntityBitVec<UbumpId>,
}

#[derive(Debug)]
pub struct ExpandedDevice<'a> {
    pub grids: EntityVec<DieId, &'a Grid>,
    pub egrid: ExpandedGrid<'a>,
    pub interposer: &'a Interposer,
    pub disabled: BTreeSet<DisabledPart>,
    pub col_cfrm: EntityVec<DieId, ColId>,
    pub sll: HashMap<(DieId, ColId, RowId), SllConns>,
}
