use std::{collections::BTreeMap, error::Error, fs::File, path::Path};

use jzon::JsonValue;
use prjcombine_interconnect::db::IntDb;
use prjcombine_types::tiledb::TileDb;
use serde::{Deserialize, Serialize};
use unnamed_entity::{EntityId, EntityVec, entity_id};

use crate::{bond::Bond, chip::Chip};

entity_id! {
    pub id ChipId usize;
    pub id BondId usize;
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub name: String,
    pub chip: ChipId,
    pub bonds: BTreeMap<String, BondId>,
    pub speeds: Vec<String>,
    pub temps: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Database {
    pub chips: EntityVec<ChipId, Chip>,
    pub bonds: EntityVec<BondId, Bond>,
    pub parts: Vec<Part>,
    pub int: IntDb,
    pub tiles: TileDb,
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

impl From<&Part> for JsonValue {
    fn from(part: &Part) -> Self {
        jzon::object! {
            name: part.name.as_str(),
            chip: part.chip.to_idx(),
            bonds: jzon::object::Object::from_iter(part.bonds.iter().map(|(name, bond)| (name.as_str(), bond.to_idx()))),
            speeds: Vec::from_iter(part.speeds.iter().map(|x| x.as_str())),
            temps: Vec::from_iter(part.temps.iter().map(|x| x.as_str())),
        }
    }
}

impl From<&Database> for JsonValue {
    fn from(db: &Database) -> Self {
        jzon::object! {
            chips: Vec::from_iter(db.chips.values()),
            bonds: Vec::from_iter(db.bonds.values()),
            parts: Vec::from_iter(db.parts.iter()),
            int: &db.int,
            tiles: &db.tiles,
        }
    }
}
