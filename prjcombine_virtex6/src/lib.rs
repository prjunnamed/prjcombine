#![allow(clippy::comparison_chain)]

use prjcombine_entity::{EntityPartVec, EntityVec};
use prjcombine_int::grid::{ColId, ExpandedGrid};
use prjcombine_virtex_bitstream::BitstreamGeom;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub use prjcombine_virtex4::bond::{CfgPin, GtPin, SharedCfgPin, SysMonPin};
pub use prjcombine_virtex4::{
    CfgRowKind, ColumnKind, DisabledPart, Grid, GridKind, Gt, GtColumn, GtKind, HardColumn,
    IoColumn, IoCoord, Pcie2, RegId, SysMon, TileIobId,
};

mod expand;
pub mod io;

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

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub disabled: &'a BTreeSet<DisabledPart>,
    pub egrid: ExpandedGrid<'a>,
    pub bs_geom: BitstreamGeom,
    pub col_frame: EntityVec<RegId, EntityVec<ColId, usize>>,
    pub bram_frame: EntityVec<RegId, EntityPartVec<ColId, usize>>,
    pub col_lio: Option<ColId>,
    pub col_rio: Option<ColId>,
    pub col_lcio: Option<ColId>,
    pub col_rcio: Option<ColId>,
    pub col_cfg: ColId,
    pub col_lgt: Option<&'a GtColumn>,
    pub col_rgt: Option<&'a GtColumn>,
    pub gt: Vec<Gt>,
    pub sysmon: Vec<SysMon>,
}
