use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs::File,
    path::Path,
};

use prjcombine_types::{FbId, FbMcId, Tile};
use serde::{Deserialize, Serialize};
use unnamed_entity::{entity_id, EntityVec};

entity_id! {
    pub id DeviceId u32;
    pub id BondId u32;
    pub id SpeedId u32;
    pub id GclkId u8;
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub idcode_part: u32,
    pub bs_cols: u32,
    pub imux_width: u32,
    pub fb_rows: u32,
    pub fb_cols: Vec<FbColumn>,
    pub io_mcs: BTreeSet<FbMcId>,
    pub io_special: BTreeMap<String, (FbId, FbMcId)>,
    pub global_bits: Tile<BitCoord>,
    pub jed_global_bits: Vec<(String, usize)>,
    pub imux_bits: Tile<BitCoord>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FbColumn {
    pub pt_col: u32,
    pub imux_col: u32,
    pub mc_col: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Pad {
    Nc,
    Gnd,
    Vcc,
    Gclk(GclkId),
    Iob(FbId, FbMcId),
    PortEn,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub idcode_part: u32,
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
pub struct BitCoord {
    pub row: u32,
    pub plane: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Database {
    pub devices: EntityVec<DeviceId, Device>,
    pub bonds: EntityVec<BondId, Bond>,
    pub speeds: EntityVec<SpeedId, Speed>,
    pub parts: Vec<Part>,
    pub mc_bits: Tile<BitCoord>,
    pub fb_bits: Tile<BitCoord>,
    pub jed_mc_bits_iob: Vec<(String, usize)>,
    pub jed_mc_bits_buried: Vec<(String, usize)>,
    pub jed_fb_bits: Vec<(String, usize)>,
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
