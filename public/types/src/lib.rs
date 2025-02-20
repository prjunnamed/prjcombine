use core::fmt::Debug;

use serde::{Deserialize, Serialize};
use unnamed_entity::entity_id;

pub mod bscan;
pub mod tiledb;

entity_id! {
    pub id FbId u8;
    pub id FbMcId u8;
    pub id IpadId u8;
}

pub type McId = (FbId, FbMcId);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum IoId {
    Ipad(IpadId),
    Mc(McId),
}
