use std::{collections::HashMap, error::Error, fs::File, path::Path};

use crate::device::{Device, Package};
use crate::types::{ImuxId, ImuxInput};
use serde::{Deserialize, Serialize};
use unnamed_entity::{EntityVec, entity_id};

pub type ImuxData = EntityVec<ImuxId, HashMap<ImuxInput, u32>>;

entity_id! {
    pub id DevId u32;
    pub id PkgId u32;
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device: Device,
    pub imux: ImuxData,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub device: DevId,
    pub package: PkgId,
    pub dev_name: String,
    pub pkg_name: String,
    pub speeds: Vec<String>,
    pub nds_version: String,
    pub vm6_family: String,
    pub vm6_dev: String,
    pub vm6_devpkg: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Database {
    pub devices: EntityVec<DevId, DeviceInfo>,
    pub packages: EntityVec<PkgId, Package>,
    pub parts: Vec<Part>,
}

impl Database {
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let f = File::create(path)?;
        let mut cf = zstd::stream::Encoder::new(f, 9)?;
        let config = bincode::config::legacy();
        bincode::serde::encode_into_std_write(self, &mut cf, config)?;
        cf.finish()?;
        Ok(())
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let f = File::open(path)?;
        let mut cf = zstd::stream::Decoder::new(f)?;
        let config = bincode::config::legacy();
        Ok(bincode::serde::decode_from_std_read(&mut cf, config)?)
    }
}
