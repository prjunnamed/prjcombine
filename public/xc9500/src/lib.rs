use std::{collections::BTreeMap, error::Error, fs::File, path::Path};

use bincode::{Decode, Encode};
use jzon::JsonValue;
use prjcombine_types::{
    bsdata::Tile,
    cpld::MacrocellCoord,
    db::{BondId, ChipId, SpeedId},
    speed::Speed,
};
use unnamed_entity::{
    EntityId, EntityVec,
    id::{EntityIdU8, EntityTag},
};

pub struct BankTag;
impl EntityTag for BankTag {
    const PREFIX: &'static str = "BANK";
}
pub type BankId = EntityIdU8<BankTag>;

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub enum ChipKind {
    Xc9500,
    Xc9500Xl,
    Xc9500Xv,
}

impl std::fmt::Display for ChipKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChipKind::Xc9500 => write!(f, "xc9500"),
            ChipKind::Xc9500Xl => write!(f, "xc9500xl"),
            ChipKind::Xc9500Xv => write!(f, "xc9500xv"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Chip {
    pub kind: ChipKind,
    pub idcode: u32,
    pub blocks: usize,
    pub io: BTreeMap<MacrocellCoord, BankId>,
    pub banks: usize,
    pub tdo_bank: BankId,
    pub io_special: BTreeMap<String, MacrocellCoord>,
    pub imux_bits: Tile,
    pub uim_ibuf_bits: Option<Tile>,
    pub program_time: u32,
    pub erase_time: u32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Encode, Decode)]
pub enum BondPad {
    Nc,
    Gnd,
    VccInt,
    VccIo(BankId),
    Iob(MacrocellCoord),
    Tms,
    Tck,
    Tdi,
    Tdo,
}

impl std::fmt::Display for BondPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPad::Nc => write!(f, "NC"),
            BondPad::Gnd => write!(f, "GND"),
            BondPad::VccInt => write!(f, "VCCINT"),
            BondPad::VccIo(bank) => write!(f, "VCCIO{bank:#}"),
            BondPad::Iob(mc) => write!(f, "IOB_{mc}"),
            BondPad::Tms => write!(f, "TMS"),
            BondPad::Tck => write!(f, "TCK"),
            BondPad::Tdi => write!(f, "TDI"),
            BondPad::Tdo => write!(f, "TDO"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Bond {
    pub io_special_override: BTreeMap<String, MacrocellCoord>,
    pub pins: BTreeMap<String, BondPad>,
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Part {
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
    pub parts: Vec<Part>,
    pub mc_bits: Tile,
    pub block_bits: Tile,
    pub global_bits: Tile,
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

impl Chip {
    pub fn to_json(&self) -> JsonValue {
        jzon::object! {
            kind: match self.kind {
                ChipKind::Xc9500 => "xc9500",
                ChipKind::Xc9500Xl => "xc9500xl",
                ChipKind::Xc9500Xv => "xc9500xv",
            },
            idcode: self.idcode,
            blocks: self.blocks,
            ios: jzon::object::Object::from_iter(
                self.io.iter().map(|(&mc, bank)| (format!("IOB_{mc}"), bank.to_idx()))
            ),
            banks: self.banks,
            tdo_bank: self.tdo_bank.to_idx(),
            io_special: jzon::object::Object::from_iter(
                self.io_special.iter().map(|(key, mc)| {
                    (key, format!("IOB_{mc}"))
                })
            ),
            imux_bits: &self.imux_bits,
            uim_ibuf_bits: if let Some(ref bits) = self.uim_ibuf_bits {
                bits.into()
            } else {
                JsonValue::Null
            },
            program_time: self.program_time,
            erase_time: self.erase_time,
        }
    }
}

impl Bond {
    pub fn to_json(&self) -> JsonValue {
        jzon::object! {
            io_special_override: jzon::object::Object::from_iter(
                self.io_special_override.iter().map(|(key, mc)| {
                    (key, format!("IOB_{mc}"))
                })
            ),
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
            block_bits: &self.block_bits,
            global_bits: &self.global_bits,
        }
    }
}
