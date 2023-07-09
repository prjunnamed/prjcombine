use crate::grid::RegId;
use prjcombine_entity::EntityVec;
use prjcombine_int::grid::{ColId, DieId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceNaming {
    pub die: EntityVec<DieId, DieNaming>,
    pub is_dsp_v2: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DieNaming {
    pub hdio: BTreeMap<(ColId, RegId), HdioNaming>,
    pub sysmon_sat_vnoc: BTreeMap<(ColId, RegId), (u32, u32)>,
    pub vnoc2: BTreeMap<(ColId, RegId), VNoc2Naming>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HdioNaming {
    pub iob_xy: (u32, u32),
    pub dpll_xy: (u32, u32),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VNoc2Naming {
    pub nsu_xy: (u32, u32),
    pub nmu_xy: (u32, u32),
    pub nps_xy: (u32, u32),
    pub scan_xy: (u32, u32),
}
