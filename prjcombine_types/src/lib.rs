use std::collections::BTreeMap;

use bitvec::vec::BitVec;
use serde::{Deserialize, Serialize};
use unnamed_entity::entity_id;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Tile<T> {
    pub items: BTreeMap<String, TileItem<T>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TileItem<T> {
    pub bits: Vec<T>,
    pub kind: TileItemKind,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum TileItemKind {
    Enum { values: BTreeMap<String, BitVec> },
    BitVec { invert: bool },
}

entity_id! {
    pub id FbId u8;
    pub id FbMcId u8;
}

pub type McId = (FbId, FbMcId);
