use std::{collections::BTreeMap, error::Error, fs::File, path::Path};

use jzon::JsonValue;
use prjcombine_types::{FbId, FbMcId, tiledb::Tile};
use serde::{Deserialize, Serialize};
use unnamed_entity::{EntityId, EntityVec, entity_id};

entity_id! {
    pub id ChipId u32;
    pub id BondId u32;
    pub id SpeedId u32;
    pub id BankId u8;
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum ChipKind {
    Xc9500,
    Xc9500Xl,
    Xc9500Xv,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Chip {
    pub kind: ChipKind,
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
pub enum BondPin {
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

impl std::fmt::Display for BondPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPin::Nc => write!(f, "NC"),
            BondPin::Gnd => write!(f, "GND"),
            BondPin::VccInt => write!(f, "VCCINT"),
            BondPin::VccIo(bank) => write!(f, "VCCIO{bank}"),
            BondPin::Iob(fb, mc) => write!(f, "IOB_{fb}_{mc}"),
            BondPin::Tms => write!(f, "TMS"),
            BondPin::Tck => write!(f, "TCK"),
            BondPin::Tdi => write!(f, "TDI"),
            BondPin::Tdo => write!(f, "TDO"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub io_special_override: BTreeMap<String, (FbId, FbMcId)>,
    pub pins: BTreeMap<String, BondPin>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Speed {
    pub timing: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub name: String,
    pub chip: ChipId,
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
    pub chips: EntityVec<ChipId, Chip>,
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

impl Chip {
    pub fn to_json(&self) -> JsonValue {
        fn fb_bit_to_json(crd: FbBitCoord) -> JsonValue {
            jzon::array![crd.row, crd.bit, crd.column]
        }

        fn global_bit_to_json(crd: GlobalBitCoord) -> JsonValue {
            jzon::array![crd.fb, crd.row, crd.bit, crd.column]
        }

        jzon::object! {
            kind: match self.kind {
                ChipKind::Xc9500 => "xc9500",
                ChipKind::Xc9500Xl => "xc9500xl",
                ChipKind::Xc9500Xv => "xc9500xv",
            },
            idcode: self.idcode,
            fbs: self.fbs,
            ios: jzon::object::Object::from_iter(
                self.io.iter().map(|(&(fb, mc), bank)| (format!("IOB_{fb}_{mc}"), bank.to_idx()))
            ),
            banks: self.banks,
            tdo_bank: self.tdo_bank.to_idx(),
            io_special: jzon::object::Object::from_iter(
                self.io_special.iter().map(|(key, (fb, mc))| {
                    (key, format!("IOB_{fb}_{mc}"))
                })
            ),
            imux_bits: self.imux_bits.to_jzon(fb_bit_to_json),
            uim_ibuf_bits: if let Some(ref bits) = self.uim_ibuf_bits {
                bits.to_jzon(global_bit_to_json)
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
                self.io_special_override.iter().map(|(key, (fb, mc))| {
                    (key, format!("IOB_{fb}_{mc}"))
                })
            ),
            pins: jzon::object::Object::from_iter(
                self.pins.iter().map(|(k, v)| (k, v.to_string()))
            ),
        }
    }
}

impl Speed {
    pub fn to_json(&self) -> JsonValue {
        jzon::object! {
            timing: jzon::object::Object::from_iter(
                self.timing.iter().map(|(k, v)| (k, *v))
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
        fn fb_bit_to_json(crd: FbBitCoord) -> JsonValue {
            jzon::array![crd.row, crd.bit, crd.column]
        }

        fn global_bit_to_json(crd: GlobalBitCoord) -> JsonValue {
            jzon::array![crd.fb, crd.row, crd.bit, crd.column]
        }

        jzon::object! {
            chips: Vec::from_iter(self.chips.values().map(Chip::to_json)),
            bonds: Vec::from_iter(self.bonds.values().map(Bond::to_json)),
            speeds: Vec::from_iter(self.speeds.values().map(Speed::to_json)),
            parts: Vec::from_iter(self.parts.iter().map(Part::to_json)),
            mc_bits: self.mc_bits.to_jzon(|bit| bit.into()),
            fb_bits: self.fb_bits.to_jzon(fb_bit_to_json),
            global_bits: self.global_bits.to_jzon(global_bit_to_json),
        }
    }
}
