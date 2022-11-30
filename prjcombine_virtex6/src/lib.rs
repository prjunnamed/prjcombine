#![allow(clippy::comparison_chain)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use prjcombine_virtex4::bond::{CfgPin, GtPin, SharedCfgPin, SysMonPin};
pub use prjcombine_virtex4::{
    CfgRowKind, ColumnKind, DieFrameGeom, DisabledPart, ExpandedDevice, ExtraDie, Grid, GridKind,
    Gt, GtColumn, GtKind, HardColumn, Io, IoColumn, IoCoord, IoDiffKind, IoKind, IoVrKind, Pcie2,
    RegId, SysMon, TileIobId,
};

mod expand;

pub use expand::expand_grid;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtRegion {
    All,
    S,
    N,
    L,
    R,
    LS,
    RS,
    LN,
    RN,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtxRegionPin {
    AVtt,
    AVcc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GthRegionPin {
    AVtt,
    AGnd,
    AVcc,
    AVccRx,
    AVccPll,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BondPin {
    // bank, pin within bank
    Io(u32, u32),
    Nc,
    Gnd,
    Rsvd,
    VccInt,
    VccAux,
    VccO(u32),
    VccBatt,
    Cfg(CfgPin),
    Gt(u32, GtPin),
    GtxRegion(GtRegion, GtxRegionPin),
    GthRegion(GtRegion, GthRegionPin),
    Dxp,
    Dxn,
    Vfs,
    SysMon(u32, SysMonPin),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
}
