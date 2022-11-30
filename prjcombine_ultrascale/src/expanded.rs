use enum_map::EnumMap;
use prjcombine_entity::EntityVec;
use prjcombine_int::grid::{ColId, DieId, ExpandedGrid};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::grid::{ColSide, DeviceNaming, DisabledPart, Grid, GridKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ClkSrc {
    DspSplitter(ColId),
    Gt(ColId),
    Cmt(ColId),
    RouteSplitter(ColId),
}

pub struct ExpandedDevice<'a> {
    pub kind: GridKind,
    pub grids: EntityVec<DieId, &'a Grid>,
    pub grid_master: DieId,
    pub egrid: ExpandedGrid<'a>,
    pub disabled: BTreeSet<DisabledPart>,
    pub naming: &'a DeviceNaming,
    pub hdistr_src: EntityVec<ColId, EnumMap<ColSide, ClkSrc>>,
    pub hroute_src: EntityVec<ColId, EnumMap<ColSide, ClkSrc>>,
    pub has_pcie_cfg: bool,
    pub is_cut: bool,
}
