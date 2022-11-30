#![allow(clippy::collapsible_else_if)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use prjcombine_virtex4::bond::{CfgPin, GtPin, SharedCfgPin, SysMonPin, GtzPin, PsPin};
pub use prjcombine_virtex4::{
    CfgRowKind, ColumnKind, DieFrameGeom, DisabledPart, ExtraDie, Grid, GridKind, Gt, GtColumn,
    GtKind, HardColumn, Io, IoColumn, IoCoord, IoDiffKind, IoKind, IoVrKind, Pcie2, Pcie2Kind,
    RegId, SysMon, TileIobId, PsIo, GtzLoc, Gtz, ExpandedDevice,
};

mod expand;

pub use expand::expand_grid;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtRegionPin {
    AVtt,
    AVcc,
    VccAux,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BondPin {
    // bank, pin within bank
    Io(u32, u32),
    Nc,
    Gnd,
    VccInt,
    VccAux,
    VccBram,
    VccO(u32),
    VccBatt,
    VccAuxIo(u32),
    RsvdGnd,
    Cfg(CfgPin),
    Gt(u32, GtPin),
    Gtz(u32, GtzPin),
    GtRegion(u32, GtRegionPin),
    Dxp,
    Dxn,
    SysMon(u32, SysMonPin),
    VccPsInt,
    VccPsAux,
    VccPsPll,
    PsVref(u32, u32),
    PsIo(u32, PsPin),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
}
