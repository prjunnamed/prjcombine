use std::{collections::HashMap, error::Error, fs::File, path::Path};

use prjcombine_xilinx_cpld::device::{Device, Package};
use prjcombine_xilinx_cpld::types::{ImuxId, ImuxInput};
use serde::{Deserialize, Serialize};
use unnamed_entity::{entity_id, EntityVec};

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
        bincode::serialize_into(&mut cf, self)?;
        cf.finish()?;
        Ok(())
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let f = File::open(path)?;
        let cf = zstd::stream::Decoder::new(f)?;
        Ok(bincode::deserialize_from(cf)?)
    }
}
