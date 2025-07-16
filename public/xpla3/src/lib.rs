use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs::File,
    path::Path,
};

use bincode::{Decode, Encode};
use jzon::JsonValue;
use prjcombine_types::{
    bsdata::Tile,
    cpld::{BlockId, MacrocellCoord, MacrocellId},
    db::{BondId, ChipId, SpeedId},
    speed::Speed,
};
use unnamed_entity::{
    EntityId, EntityIds, EntityVec,
    id::{EntityIdU8, EntityTag},
};

pub struct GclkTag;
impl EntityTag for GclkTag {
    const PREFIX: &'static str = "GCLK";
}
pub type GclkId = EntityIdU8<GclkTag>;

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Chip {
    pub idcode_part: u32,
    pub bs_cols: usize,
    pub imux_width: usize,
    pub block_rows: usize,
    pub block_cols: Vec<FbColumn>,
    pub io_mcs: BTreeSet<MacrocellId>,
    pub io_special: BTreeMap<String, MacrocellCoord>,
    pub global_bits: Tile,
    pub jed_global_bits: Vec<(String, usize)>,
    pub imux_bits: Tile,
}

impl Chip {
    pub fn blocks(&self) -> EntityIds<BlockId> {
        EntityIds::new(self.block_rows * self.block_cols.len() * 2)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct FbColumn {
    pub pt_col: usize,
    pub imux_col: usize,
    pub mc_col: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Encode, Decode)]
pub enum BondPad {
    Nc,
    Gnd,
    Vcc,
    Gclk(GclkId),
    Iob(MacrocellCoord),
    PortEn,
}

impl std::fmt::Display for BondPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPad::Nc => write!(f, "NC"),
            BondPad::Gnd => write!(f, "GND"),
            BondPad::Vcc => write!(f, "VCC"),
            BondPad::Iob(mc) => write!(f, "IOB_{mc}"),
            BondPad::Gclk(gclk) => write!(f, "{gclk}"),
            BondPad::PortEn => write!(f, "PORT_EN"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Bond {
    pub idcode_part: u32,
    pub pins: BTreeMap<String, BondPad>,
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Device {
    pub name: String,
    pub chip: ChipId,
    pub packages: BTreeMap<String, BondId>,
    pub speeds: BTreeMap<String, SpeedId>,
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Database {
    pub chips: EntityVec<ChipId, Chip>,
    pub bonds: EntityVec<BondId, Bond>,
    pub speeds: EntityVec<SpeedId, Speed>,
    pub devices: Vec<Device>,
    pub mc_bits: Tile,
    pub block_bits: Tile,
    pub jed_mc_bits_iob: Vec<(String, usize)>,
    pub jed_mc_bits_buried: Vec<(String, usize)>,
    pub jed_block_bits: Vec<(String, usize)>,
}

impl Database {
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let f = File::create(path)?;
        let mut cf = zstd::stream::Encoder::new(f, 9)?;
        let config = bincode::config::standard();
        bincode::encode_into_std_write(self, &mut cf, config)?;
        cf.finish()?;
        Ok(())
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let f = File::open(path)?;
        let mut cf = zstd::stream::Decoder::new(f)?;
        let config = bincode::config::standard();
        Ok(bincode::decode_from_std_read(&mut cf, config)?)
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

impl From<&Chip> for JsonValue {
    fn from(chip: &Chip) -> JsonValue {
        jzon::object! {
            idcode_part: chip.idcode_part,
            bs_cols: chip.bs_cols,
            imux_width: chip.imux_width,
            block_rows: chip.block_rows,
            block_cols: Vec::from_iter(chip.block_cols.iter().map(|bcol| bcol.to_json())),
            io_mcs: Vec::from_iter(chip.io_mcs.iter().map(|mc| mc.to_idx())),
            io_special: jzon::object::Object::from_iter(
                chip.io_special.iter().map(|(key, mc)| {
                    (key, format!("IOB_{mc}"))
                })
            ),
            global_bits: &chip.global_bits,
            jed_global_bits: jed_bits_to_json(&chip.jed_global_bits),
            imux_bits: &chip.imux_bits,
        }
    }
}

impl From<&Bond> for JsonValue {
    fn from(bond: &Bond) -> JsonValue {
        jzon::object! {
            idcode_part: bond.idcode_part,
            pins: jzon::object::Object::from_iter(
                bond.pins.iter().map(|(k, v)| (k, v.to_string()))
            ),
        }
    }
}

impl From<&Device> for JsonValue {
    fn from(device: &Device) -> Self {
        jzon::object! {
            name: device.name.as_str(),
            chip: device.chip.to_idx(),
            packages: jzon::object::Object::from_iter(
                device.packages.iter().map(|(name, bond)| (name, bond.to_idx()))
            ),
            speeds: jzon::object::Object::from_iter(
                device.speeds.iter().map(|(name, speed)| (name, speed.to_idx()))
            ),
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
            mc_bits: &db.mc_bits,
            block_bits: &db.block_bits,
            jed_mc_bits_iob: jed_bits_to_json(&db.jed_mc_bits_iob),
            jed_mc_bits_buried: jed_bits_to_json(&db.jed_mc_bits_buried),
            jed_block_bits: jed_bits_to_json(&db.jed_block_bits),
        }
    }
}
