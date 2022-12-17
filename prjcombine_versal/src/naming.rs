use crate::grid::RegId;
use prjcombine_entity::EntityVec;
use prjcombine_int::grid::{ColId, DieId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceNaming {
    pub die: EntityVec<DieId, DieNaming>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DieNaming {
    pub hdio: BTreeMap<(ColId, RegId), HdioNaming>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HdioNaming {
    pub iob_xy: (u32, u32),
    pub dpll_xy: (u32, u32),
}
