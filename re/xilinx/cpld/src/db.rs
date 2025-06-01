use std::{collections::HashMap, error::Error, fs::File, path::Path};

use crate::device::{Device, Package};
use crate::types::{ImuxId, ImuxInput};
use bincode::{Decode, Encode};
use prjcombine_types::db::{BondId, ChipId};
use unnamed_entity::EntityVec;

pub type ImuxData = EntityVec<ImuxId, HashMap<ImuxInput, u32>>;

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct DeviceInfo {
    pub device: Device,
    pub imux: ImuxData,
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Part {
    pub device: ChipId,
    pub package: BondId,
    pub dev_name: String,
    pub pkg_name: String,
    pub speeds: Vec<String>,
    pub nds_version: String,
    pub vm6_family: String,
    pub vm6_dev: String,
    pub vm6_devpkg: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Database {
    pub devices: EntityVec<ChipId, DeviceInfo>,
    pub packages: EntityVec<BondId, Package>,
    pub parts: Vec<Part>,
}

impl Database {
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let f = File::create(path)?;
        let mut cf = zstd::stream::Encoder::new(f, 9)?;
        let config = bincode::config::legacy();
        bincode::encode_into_std_write(self, &mut cf, config)?;
        cf.finish()?;
        Ok(())
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let f = File::open(path)?;
        let mut cf = zstd::stream::Decoder::new(f)?;
        let config = bincode::config::legacy();
        Ok(bincode::decode_from_std_read(&mut cf, config)?)
    }
}
