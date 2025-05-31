use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs::File,
    path::Path,
};

use jzon::JsonValue;
use prjcombine_types::{FbId, FbMcId, speed::Speed, bsdata::Tile};
use serde::{Deserialize, Serialize};
use unnamed_entity::{EntityId, EntityIds, EntityVec, entity_id};

entity_id! {
    pub id ChipId u32;
    pub id BondId u32;
    pub id SpeedId u32;
    pub id GclkId u8;
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Chip {
    pub idcode_part: u32,
    pub bs_cols: usize,
    pub imux_width: usize,
    pub fb_rows: usize,
    pub fb_cols: Vec<FbColumn>,
    pub io_mcs: BTreeSet<FbMcId>,
    pub io_special: BTreeMap<String, (FbId, FbMcId)>,
    pub global_bits: Tile,
    pub jed_global_bits: Vec<(String, usize)>,
    pub imux_bits: Tile,
}

impl Chip {
    pub fn fbs(&self) -> EntityIds<FbId> {
        EntityIds::new(self.fb_rows * self.fb_cols.len() * 2)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FbColumn {
    pub pt_col: usize,
    pub imux_col: usize,
    pub mc_col: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum BondPin {
    Nc,
    Gnd,
    Vcc,
    Gclk(GclkId),
    Iob(FbId, FbMcId),
    PortEn,
}

impl std::fmt::Display for BondPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPin::Nc => write!(f, "NC"),
            BondPin::Gnd => write!(f, "GND"),
            BondPin::Vcc => write!(f, "VCC"),
            BondPin::Iob(fb, mc) => write!(f, "IOB_{fb}_{mc}"),
            BondPin::Gclk(gclk) => write!(f, "GCLK{gclk}"),
            BondPin::PortEn => write!(f, "PORT_EN"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub idcode_part: u32,
    pub pins: BTreeMap<String, BondPin>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub name: String,
    pub chip: ChipId,
    pub packages: BTreeMap<String, BondId>,
    pub speeds: BTreeMap<String, SpeedId>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Database {
    pub chips: EntityVec<ChipId, Chip>,
    pub bonds: EntityVec<BondId, Bond>,
    pub speeds: EntityVec<SpeedId, Speed>,
    pub parts: Vec<Part>,
    pub mc_bits: Tile,
    pub fb_bits: Tile,
    pub jed_mc_bits_iob: Vec<(String, usize)>,
    pub jed_mc_bits_buried: Vec<(String, usize)>,
    pub jed_fb_bits: Vec<(String, usize)>,
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

fn jed_bits_to_json(jed_bits: &[(String, usize)]) -> JsonValue {
    Vec::from_iter(
        jed_bits
            .iter()
            .map(|(name, index)| jzon::array![name.as_str(), *index]),
    )
    .into()
}

impl FbColumn {
    pub fn to_json(&self) -> JsonValue {
        jzon::object! {
            pt_col: self.pt_col,
            imux_col: self.imux_col,
            mc_col: self.mc_col,
        }
    }
}

impl Chip {
    pub fn to_json(&self) -> JsonValue {
        jzon::object! {
            idcode_part: self.idcode_part,
            bs_cols: self.bs_cols,
            imux_width: self.imux_width,
            fb_rows: self.fb_rows,
            fb_cols: Vec::from_iter(self.fb_cols.iter().map(|fbcol| fbcol.to_json())),
            io_mcs: Vec::from_iter(self.io_mcs.iter().map(|mc| mc.to_idx())),
            io_special: jzon::object::Object::from_iter(
                self.io_special.iter().map(|(key, (fb, mc))| {
                    (key, format!("IOB_{fb}_{mc}"))
                })
            ),
            global_bits: &self.global_bits,
            jed_global_bits: jed_bits_to_json(&self.jed_global_bits),
            imux_bits: &self.imux_bits,
        }
    }
}

impl Bond {
    pub fn to_json(&self) -> JsonValue {
        jzon::object! {
            idcode_part: self.idcode_part,
            pins: jzon::object::Object::from_iter(
                self.pins.iter().map(|(k, v)| (k, v.to_string()))
            ),
        }
    }
}

impl Part {
    pub fn to_json(&self) -> JsonValue {
        jzon::object! {
            name: self.name.as_str(),
            chip: self.chip.to_idx(),
            packages: jzon::object::Object::from_iter(
                self.packages.iter().map(|(name, bond)| (name, bond.to_idx()))
            ),
            speeds: jzon::object::Object::from_iter(
                self.speeds.iter().map(|(name, speed)| (name, speed.to_idx()))
            ),
        }
    }
}

impl Database {
    pub fn to_json(&self) -> JsonValue {
        jzon::object! {
            chips: Vec::from_iter(self.chips.values().map(Chip::to_json)),
            bonds: Vec::from_iter(self.bonds.values().map(Bond::to_json)),
            speeds: Vec::from_iter(self.speeds.values()),
            parts: Vec::from_iter(self.parts.iter().map(Part::to_json)),
            mc_bits: &self.mc_bits,
            fb_bits: &self.fb_bits,
            jed_mc_bits_iob: jed_bits_to_json(&self.jed_mc_bits_iob),
            jed_mc_bits_buried: jed_bits_to_json(&self.jed_mc_bits_buried),
            jed_fb_bits: jed_bits_to_json(&self.jed_fb_bits),
        }
    }
}
