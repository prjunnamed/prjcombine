use std::{collections::BTreeSet, error::Error, fs::File, path::Path};

use bincode::{Decode, Encode};
use jzon::JsonValue;
use prjcombine_interconnect::{db::IntDb, grid::DieId};
use prjcombine_types::{
    bsdata::BsData,
    db::{BondId, ChipId, DevBondId, DevSpeedId, DeviceCombo, InterposerId},
};
use prjcombine_entity::{EntityId, EntityMap, EntityVec};

use crate::{
    bond::Bond,
    chip::{Chip, DisabledPart, Interposer},
    gtz::GtzDb,
};

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Device {
    pub name: String,
    pub chips: EntityVec<DieId, ChipId>,
    pub interposer: Option<InterposerId>,
    pub bonds: EntityMap<DevBondId, String, BondId>,
    pub speeds: EntityVec<DevSpeedId, String>,
    pub combos: Vec<DeviceCombo>,
    pub disabled: BTreeSet<DisabledPart>,
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Database {
    pub chips: EntityVec<ChipId, Chip>,
    pub interposers: EntityVec<InterposerId, Interposer>,
    pub bonds: EntityVec<BondId, Bond>,
    pub devices: Vec<Device>,
    pub int: IntDb,
    pub bsdata: BsData,
    pub gtz: GtzDb,
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
    fn from(part: &Device) -> Self {
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
            devices: Vec::from_iter(db.devices.iter()),
            int: &db.int,
            bsdata: &db.bsdata,
            gtz: &db.gtz,
        }
    }
}
