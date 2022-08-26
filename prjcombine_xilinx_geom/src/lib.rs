#![allow(clippy::collapsible_else_if)]

use prjcombine_entity::{entity_id, EntityId, EntityVec};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub mod series7;
pub mod spartan6;
pub mod ultrascale;
pub mod versal;
pub mod virtex;
pub mod virtex2;
pub mod virtex4;
pub mod virtex5;
pub mod virtex6;
pub mod xc4k;
pub mod xc5200;

pub mod eint;
pub mod int;
pub mod pkg;

entity_id! {
    pub id GridId usize;
    pub id BondId usize;
    pub id DevBondId usize;
    pub id DevSpeedId usize;

    pub id ColId u16;
    pub id RowId u16;
    pub id SlrId u16;
    pub id BelId u16;
}

impl core::ops::Add<usize> for ColId {
    type Output = ColId;
    fn add(self, x: usize) -> ColId {
        ColId::from_idx(self.to_idx() + x)
    }
}

impl core::ops::AddAssign<usize> for ColId {
    fn add_assign(&mut self, x: usize) {
        *self = *self + x;
    }
}

impl core::ops::Sub<usize> for ColId {
    type Output = ColId;
    fn sub(self, x: usize) -> ColId {
        ColId::from_idx(self.to_idx() - x)
    }
}

impl core::ops::SubAssign<usize> for ColId {
    fn sub_assign(&mut self, x: usize) {
        *self = *self - x;
    }
}

impl core::ops::Add<usize> for RowId {
    type Output = RowId;
    fn add(self, x: usize) -> RowId {
        RowId::from_idx(self.to_idx() + x)
    }
}

impl core::ops::AddAssign<usize> for RowId {
    fn add_assign(&mut self, x: usize) {
        *self = *self + x;
    }
}

impl core::ops::Sub<usize> for RowId {
    type Output = RowId;
    fn sub(self, x: usize) -> RowId {
        RowId::from_idx(self.to_idx() - x)
    }
}

impl core::ops::SubAssign<usize> for RowId {
    fn sub_assign(&mut self, x: usize) {
        *self = *self - x;
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Grid {
    Xc4k(xc4k::Grid),
    Xc5200(xc5200::Grid),
    Virtex(virtex::Grid),
    Virtex2(virtex2::Grid),
    Spartan6(spartan6::Grid),
    Virtex4(virtex4::Grid),
    Virtex5(virtex5::Grid),
    Virtex6(virtex6::Grid),
    Series7(series7::Grid),
    Ultrascale(ultrascale::Grid),
    Versal(versal::Grid),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceBond {
    pub name: String,
    pub bond: BondId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DisabledPart {
    // Virtex-E: primary DLLs are disabled
    VirtexPrimaryDlls,
    // Virtex-E: a BRAM column is disabled
    VirtexBram(ColId),
    // Virtex 6: disable primitive in given row
    Virtex6Emac(RowId),
    Virtex6GtxRow(u32),
    Virtex6SysMon,
    Spartan6Gtp,
    Spartan6Mcb,
    Spartan6ClbColumn(ColId),
    Spartan6BramRegion(ColId, u32),
    Spartan6DspRegion(ColId, u32),
    Region(SlrId, u32),
    Ps,
    VersalHardIp(SlrId, ColId, usize),
    VersalColumn(SlrId, ColId),
    VersalGtRight(SlrId, usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PartNamingKey {
    VersalHdio(SlrId, ColId, usize),
    VersalGtLeft(SlrId, usize),
    VersalGtRight(SlrId, usize),
    VersalVNoc(SlrId, ColId, usize),
    VersalDdrMcBot(usize),
    VersalDdrMcTop(usize),
    VersalXpioBot(usize),
    VersalXpioTop(usize),
    VersalHbmTop(usize),
    // XXX NPS bot
    // XXX NPS top
    // XXX NCRB top
    // XXX ME top
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceCombo {
    pub name: String,
    pub devbond_idx: DevBondId,
    pub speed_idx: DevSpeedId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ExtraDie {
    GtzTop,
    GtzBottom,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Device {
    pub name: String,
    pub grids: EntityVec<SlrId, GridId>,
    pub grid_master: SlrId,
    pub extras: Vec<ExtraDie>,
    pub bonds: EntityVec<DevBondId, DeviceBond>,
    pub speeds: EntityVec<DevSpeedId, String>,
    // valid (bond, speed) pairs
    pub combos: Vec<DeviceCombo>,
    pub disabled: BTreeSet<DisabledPart>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct BelCoord {
    pub col: ColId,
    pub row: RowId,
    pub bel: BelId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeomDb {
    pub grids: EntityVec<GridId, Grid>,
    pub bonds: EntityVec<BondId, pkg::Bond>,
    pub devices: Vec<Device>,
    pub ints: BTreeMap<String, int::IntDb>,
}
