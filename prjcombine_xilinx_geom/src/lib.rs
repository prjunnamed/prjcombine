#![allow(clippy::collapsible_else_if)]

use prjcombine_entity::{entity_id, EntityVec};
use prjcombine_int::db::IntDb;
use prjcombine_int::grid::DieId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

entity_id! {
    pub id GridId usize;
    pub id BondId usize;
    pub id DevBondId usize;
    pub id DevSpeedId usize;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Grid {
    Xc4k(prjcombine_xc4k::Grid),
    Xc5200(prjcombine_xc5200::Grid),
    Virtex(prjcombine_virtex::Grid),
    Virtex2(prjcombine_virtex2::Grid),
    Spartan6(prjcombine_spartan6::Grid),
    Virtex4(prjcombine_virtex4::Grid),
    Virtex5(prjcombine_virtex5::Grid),
    Virtex6(prjcombine_virtex6::Grid),
    Series7(prjcombine_series7::Grid),
    Ultrascale(prjcombine_ultrascale::Grid),
    Versal(prjcombine_versal::Grid),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceBond {
    pub name: String,
    pub bond: BondId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DisabledPart {
    Virtex(prjcombine_virtex::DisabledPart),
    Spartan6(prjcombine_spartan6::DisabledPart),
    Virtex6(prjcombine_virtex6::DisabledPart),
    Series7(prjcombine_series7::DisabledPart),
    Ultrascale(prjcombine_ultrascale::DisabledPart),
    Versal(prjcombine_versal::DisabledPart),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceCombo {
    pub name: String,
    pub devbond_idx: DevBondId,
    pub speed_idx: DevSpeedId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ExtraDie {
    Series7(prjcombine_series7::ExtraDie),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Device {
    pub name: String,
    pub grids: EntityVec<DieId, GridId>,
    pub grid_master: DieId,
    pub extras: Vec<ExtraDie>,
    pub bonds: EntityVec<DevBondId, DeviceBond>,
    pub speeds: EntityVec<DevSpeedId, String>,
    // valid (bond, speed) pairs
    pub combos: Vec<DeviceCombo>,
    pub disabled: BTreeSet<DisabledPart>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Bond {
    Xc4k(prjcombine_xc4k::Bond),
    Xc5200(prjcombine_xc5200::Bond),
    Virtex(prjcombine_virtex::Bond),
    Virtex2(prjcombine_virtex2::Bond),
    Spartan6(prjcombine_spartan6::Bond),
    Virtex4(prjcombine_virtex4::Bond),
    Virtex5(prjcombine_virtex5::Bond),
    Virtex6(prjcombine_virtex6::Bond),
    Series7(prjcombine_series7::Bond),
    Ultrascale(prjcombine_ultrascale::Bond),
    Versal(prjcombine_versal::Bond),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeomDb {
    pub grids: EntityVec<GridId, Grid>,
    pub bonds: EntityVec<BondId, Bond>,
    pub devices: Vec<Device>,
    pub ints: BTreeMap<String, IntDb>,
}
