use std::{collections::BTreeMap, error::Error, fs::File, path::Path};

use prjcombine_types::{tiledb::Tile, FbId, FbMcId, IoId, IpadId};
use serde::{Deserialize, Serialize};
use unnamed_entity::{entity_id, EntityVec};

entity_id! {
    pub id DeviceId u32;
    pub id BondId u32;
    pub id SpeedId u32;
    pub id BankId u8;
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub idcode_part: u32,
    pub ipads: usize,
    pub io: BTreeMap<IoId, Io>,
    pub banks: usize,
    pub has_vref: bool,
    pub bs_layout: BsLayout,
    pub bs_cols: u32,
    pub imux_width: u32,
    pub xfer_cols: Vec<u32>,
    pub mc_width: u32,
    pub fb_rows: u32,
    pub fb_cols: Vec<u32>,
    pub io_special: BTreeMap<String, (FbId, FbMcId)>,
    pub mc_bits: Tile<BitCoord>,
    pub global_bits: Tile<BitCoord>,
    pub jed_global_bits: Vec<(String, usize)>,
    pub imux_bits: Tile<BitCoord>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum BsLayout {
    Narrow,
    Wide,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Io {
    pub bank: BankId,
    pub pad_distance: u32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct BitCoord {
    pub row: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Pad {
    Nc,
    Gnd,
    VccInt,
    VccIo(BankId),
    VccAux,
    Iob(FbId, FbMcId),
    Ipad(IpadId),
    Tms,
    Tck,
    Tdi,
    Tdo,
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

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Database {
    pub devices: EntityVec<DeviceId, Device>,
    pub bonds: EntityVec<BondId, Bond>,
    pub speeds: EntityVec<SpeedId, Speed>,
    pub parts: Vec<Part>,
    pub jed_mc_bits_small: Vec<(String, usize)>,
    pub jed_mc_bits_large_iob: Vec<(String, usize)>,
    pub jed_mc_bits_large_buried: Vec<(String, usize)>,
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
