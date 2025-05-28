use std::{collections::BTreeSet, error::Error, fs::File, path::Path};

use jzon::JsonValue;
use prjcombine_interconnect::{db::IntDb, grid::DieId};
use prjcombine_types::bsdata::BsData;
use serde::{Deserialize, Serialize};
use unnamed_entity::{EntityId, EntityMap, EntityVec, entity_id};

use crate::{
    bond::Bond,
    chip::{Chip, DisabledPart, Interposer},
    gtz::GtzDb,
};

entity_id! {
    pub id ChipId usize;
    pub id InterposerId usize;
    pub id BondId usize;
    pub id DevBondId usize;
    pub id DevSpeedId usize;
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeviceCombo {
    pub devbond: DevBondId,
    pub speed: DevSpeedId,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub name: String,
    pub chips: EntityVec<DieId, ChipId>,
    pub interposer: Option<InterposerId>,
    pub bonds: EntityMap<DevBondId, String, BondId>,
    pub speeds: EntityVec<DevSpeedId, String>,
    pub combos: Vec<DeviceCombo>,
    pub disabled: BTreeSet<DisabledPart>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Database {
    pub chips: EntityVec<ChipId, Chip>,
    pub interposers: EntityVec<InterposerId, Interposer>,
    pub bonds: EntityVec<BondId, Bond>,
    pub parts: Vec<Part>,
    pub int: IntDb,
    pub bsdata: BsData,
    pub gtz: GtzDb,
}

impl Database {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let f = File::open(path)?;
        let cf = zstd::stream::Decoder::new(f)?;
        Ok(bincode::deserialize_from(cf)?)
    }

    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let f = File::create(path)?;
        let mut cf = zstd::stream::Encoder::new(f, 9)?;
        bincode::serialize_into(&mut cf, self)?;
        cf.finish()?;
        Ok(())
    }
}

impl From<&DeviceCombo> for JsonValue {
    fn from(combo: &DeviceCombo) -> Self {
        jzon::object! {
            devbond: combo.devbond.to_idx(),
            speed: combo.speed.to_idx(),
        }
    }
}

impl From<&Part> for JsonValue {
    fn from(part: &Part) -> Self {
        jzon::object! {
            name: part.name.as_str(),
            chips: Vec::from_iter(part.chips.values().map(|x| x.to_idx())),
            interposer: part.interposer.map(|interp| interp.to_idx()),
            bonds: jzon::object::Object::from_iter(part.bonds.iter().map(|(_, name, bond)| (name.as_str(), bond.to_idx()))),
            speeds: Vec::from_iter(part.speeds.values().map(|x| x.as_str())),
            combos: Vec::from_iter(part.combos.iter()),
            disabled: Vec::from_iter(part.disabled.iter().map(|dis| dis.to_string())),
        }
    }
}

impl From<&Database> for JsonValue {
    fn from(db: &Database) -> Self {
        jzon::object! {
            chips: Vec::from_iter(db.chips.values()),
            interposers: Vec::from_iter(db.interposers.values()),
            bonds: Vec::from_iter(db.bonds.values()),
            parts: Vec::from_iter(db.parts.iter()),
            int: &db.int,
            bsdata: &db.bsdata,
            gtz: &db.gtz,
        }
    }
}
