use std::{collections::BTreeMap, error::Error, fs::File, path::Path};

use prjcombine_types::{tiledb::Tile, FbId, FbMcId};
use serde::{Deserialize, Serialize};
use unnamed_entity::{entity_id, EntityVec};

entity_id! {
    pub id DeviceId u32;
    pub id BondId u32;
    pub id SpeedId u32;
    pub id BankId u8;
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum DeviceKind {
    Xc9500,
    Xc9500Xl,
    Xc9500Xv,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub kind: DeviceKind,
    pub idcode: u32,
    pub fbs: usize,
    pub io: BTreeMap<(FbId, FbMcId), BankId>,
    pub banks: usize,
    pub tdo_bank: BankId,
    pub io_special: BTreeMap<String, (FbId, FbMcId)>,
    pub imux_bits: Tile<FbBitCoord>,
    pub uim_ibuf_bits: Option<Tile<GlobalBitCoord>>,
    pub program_time: u32,
    pub erase_time: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Pad {
    Nc,
    Gnd,
    VccInt,
    VccIo(BankId),
    Iob(FbId, FbMcId),
    Tms,
    Tck,
    Tdi,
    Tdo,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub io_special_override: BTreeMap<String, (FbId, FbMcId)>,
    pub pins: BTreeMap<String, Pad>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Speed {
    pub timing: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub name: String,
    pub device: DeviceId,
    pub packages: BTreeMap<String, BondId>,
    pub speeds: BTreeMap<String, SpeedId>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct FbBitCoord {
    pub row: u32,
    pub bit: u32,
    pub column: u32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct GlobalBitCoord {
    pub fb: u32,
    pub row: u32,
    pub bit: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Database {
    pub devices: EntityVec<DeviceId, Device>,
    pub bonds: EntityVec<BondId, Bond>,
    pub speeds: EntityVec<SpeedId, Speed>,
    pub parts: Vec<Part>,
    pub mc_bits: Tile<u32>,
    pub fb_bits: Tile<FbBitCoord>,
    pub global_bits: Tile<GlobalBitCoord>,
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
