use prjcombine_entity::entity_id;
use prjcombine_int::grid::{ColId, RowId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

entity_id! {
    pub id TileIobId u8;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GridKind {
    // plain, D (no memory)
    Xc4000,
    Xc4000A,
    Xc4000H,
    // E, L, Spartan
    Xc4000E,
    // EX, XL
    Xc4000Ex,
    Xc4000Xla,
    Xc4000Xv,
    SpartanXl,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub kind: GridKind,
    pub columns: u32,
    pub rows: u32,
    pub cfg_io: BTreeMap<SharedCfgPin, IoCoord>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    Tck,
    Tdi,
    Tms,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct IoCoord {
    pub col: ColId,
    pub row: RowId,
    pub iob: TileIobId,
}
