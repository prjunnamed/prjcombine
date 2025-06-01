use std::{collections::BTreeMap, error::Error, fs::File, path::Path};

use jzon::JsonValue;
use prjcombine_interconnect::db::IntDb;
use prjcombine_types::{bsdata::BsData, speed::Speed};
use serde::{Deserialize, Serialize};
use unnamed_entity::{EntityId, EntityVec, entity_id};

use crate::{bond::Bond, chip::Chip};

entity_id! {
    pub id ChipId usize;
    pub id BondId usize;
    pub id SpeedId usize;
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub name: String,
    pub chip: ChipId,
    pub bonds: BTreeMap<String, BondId>,
    pub speeds: BTreeMap<String, SpeedId>,
    pub temps: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Database {
    pub chips: EntityVec<ChipId, Chip>,
    pub bonds: EntityVec<BondId, Bond>,
    pub speeds: EntityVec<SpeedId, Speed>,
    pub parts: Vec<Part>,
    pub int: IntDb,
    pub bsdata: BsData,
}

impl Database {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let f = File::open(path)?;
        let mut cf = zstd::stream::Decoder::new(f)?;
        let config = bincode::config::legacy();
        Ok(bincode::serde::decode_from_std_read(&mut cf, config)?)
    }

    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let f = File::create(path)?;
        let mut cf = zstd::stream::Encoder::new(f, 9)?;
        let config = bincode::config::legacy();
        bincode::serde::encode_into_std_write(self, &mut cf, config)?;
        cf.finish()?;
        Ok(())
    }
}

impl From<&Part> for JsonValue {
    fn from(part: &Part) -> Self {
        jzon::object! {
            name: part.name.as_str(),
            chip: part.chip.to_idx(),
            bonds: jzon::object::Object::from_iter(part.bonds.iter().map(|(name, bond)| (name.as_str(), bond.to_idx()))),
            speeds: jzon::object::Object::from_iter(part.speeds.iter().map(|(name, speed)| (name.as_str(), speed.to_idx()))),
            temps: Vec::from_iter(part.temps.iter().map(|x| x.as_str())),
        }
    }
}

impl From<&Database> for JsonValue {
    fn from(db: &Database) -> Self {
        jzon::object! {
            chips: Vec::from_iter(db.chips.values()),
            bonds: Vec::from_iter(db.bonds.values()),
            speeds: Vec::from_iter(db.speeds.values()),
            parts: Vec::from_iter(db.parts.iter()),
            int: &db.int,
            bsdata: &db.bsdata,
        }
    }
}
