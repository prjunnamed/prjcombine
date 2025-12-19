use std::{collections::BTreeMap, error::Error, fs::File, path::Path};

use bincode::{Decode, Encode};
use jzon::JsonValue;
use prjcombine_interconnect::db::IntDb;
use prjcombine_types::{
    bsdata::BsData,
    db::{BondId, ChipId, SpeedId},
    speed::Speed,
};
use prjcombine_entity::{EntityId, EntityVec};

use crate::{bond::Bond, chip::Chip};

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Device {
    pub name: String,
    pub chip: ChipId,
    pub bonds: BTreeMap<String, BondId>,
    pub speeds: BTreeMap<String, SpeedId>,
    pub temps: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Database {
    pub chips: EntityVec<ChipId, Chip>,
    pub bonds: EntityVec<BondId, Bond>,
    pub speeds: EntityVec<SpeedId, Speed>,
    pub devices: Vec<Device>,
    pub int: IntDb,
    pub bsdata: BsData,
}

impl Database {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let f = File::open(path)?;
        let mut cf = zstd::stream::Decoder::new(f)?;
        let config = bincode::config::standard();
        Ok(bincode::decode_from_std_read(&mut cf, config)?)
    }

    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let f = File::create(path)?;
        let mut cf = zstd::stream::Encoder::new(f, 9)?;
        let config = bincode::config::standard();
        bincode::encode_into_std_write(self, &mut cf, config)?;
        cf.finish()?;
        Ok(())
    }
}

impl From<&Device> for JsonValue {
    fn from(device: &Device) -> Self {
        jzon::object! {
            name: device.name.as_str(),
            chip: device.chip.to_idx(),
            bonds: jzon::object::Object::from_iter(device.bonds.iter().map(|(name, bond)| (name.as_str(), bond.to_idx()))),
            speeds: jzon::object::Object::from_iter(device.speeds.iter().map(|(name, speed)| (name.as_str(), speed.to_idx()))),
            temps: Vec::from_iter(device.temps.iter().map(|x| x.as_str())),
        }
    }
}

impl From<&Database> for JsonValue {
    fn from(db: &Database) -> Self {
        jzon::object! {
            chips: Vec::from_iter(db.chips.values()),
            bonds: Vec::from_iter(db.bonds.values()),
            speeds: Vec::from_iter(db.speeds.values()),
            devices: Vec::from_iter(db.devices.iter()),
            int: &db.int,
            bsdata: &db.bsdata,
        }
    }
}
