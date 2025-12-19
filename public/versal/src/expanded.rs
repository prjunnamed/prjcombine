use prjcombine_entity::{EntityBitVec, EntityVec, entity_id};
use prjcombine_interconnect::grid::{ColId, DieId, ExpandedGrid, RowId};
use std::collections::{BTreeSet, HashMap};

use crate::chip::{Chip, DisabledPart, Interposer};

entity_id! {
    pub id UbumpId u8;
}

pub type UbumpLoc = (DieId, ColId, RowId, UbumpId);

#[derive(Debug)]
pub struct SllConns {
    pub conns: EntityVec<UbumpId, Option<UbumpLoc>>,
    pub cursed: EntityBitVec<UbumpId>,
}

#[derive(Debug)]
pub struct ExpandedDevice<'a> {
    pub chips: EntityVec<DieId, &'a Chip>,
    pub egrid: ExpandedGrid<'a>,
    pub interposer: &'a Interposer,
    pub disabled: BTreeSet<DisabledPart>,
    pub col_cfrm: EntityVec<DieId, ColId>,
    pub sll: HashMap<(DieId, ColId, RowId), SllConns>,
}

impl<'a> std::ops::Deref for ExpandedDevice<'a> {
    type Target = ExpandedGrid<'a>;

    fn deref(&self) -> &Self::Target {
        &self.egrid
    }
}
