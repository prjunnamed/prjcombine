use std::{collections::BTreeMap, error::Error, fs::File, path::Path};

use jzon::JsonValue;
use prjcombine_types::{FbId, FbMcId, IoId, IpadId, speed::Speed, tiledb::Tile};
use serde::{Deserialize, Serialize};
use unnamed_entity::{EntityId, EntityIds, EntityVec, entity_id};

entity_id! {
    pub id ChipId u32;
    pub id BondId u32;
    pub id SpeedId u32;
    pub id BankId u8;
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Chip {
    pub idcode_part: u32,
    pub ipads: usize,
    pub io: BTreeMap<IoId, Io>,
    pub banks: usize,
    pub has_vref: bool,
    pub bs_layout: BsLayout,
    pub bs_cols: usize,
    pub imux_width: usize,
    pub xfer_cols: Vec<usize>,
    pub mc_width: usize,
    pub fb_rows: usize,
    pub fb_cols: Vec<usize>,
    pub io_special: BTreeMap<String, (FbId, FbMcId)>,
    pub mc_bits: Tile,
    pub global_bits: Tile,
    pub jed_global_bits: Vec<(String, usize)>,
    pub imux_bits: Tile,
}

impl Chip {
    pub fn fbs(&self) -> EntityIds<FbId> {
        EntityIds::new(self.fb_rows * self.fb_cols.len() * 2)
    }
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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum BondPin {
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

impl std::fmt::Display for BondPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPin::Nc => write!(f, "NC"),
            BondPin::Gnd => write!(f, "GND"),
            BondPin::VccInt => write!(f, "VCCINT"),
            BondPin::VccIo(bank) => write!(f, "VCCIO{bank}"),
            BondPin::VccAux => write!(f, "VCCAUX"),
            BondPin::Iob(fb, mc) => write!(f, "IOB_{fb}_{mc}"),
            BondPin::Ipad(ipad) => write!(f, "IPAD{ipad}"),
            BondPin::Tck => write!(f, "TCK"),
            BondPin::Tms => write!(f, "TMS"),
            BondPin::Tdi => write!(f, "TDI"),
            BondPin::Tdo => write!(f, "TDO"),
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

fn jed_bits_to_json(jed_bits: &[(String, usize)]) -> JsonValue {
    Vec::from_iter(
        jed_bits
            .iter()
            .map(|(name, index)| jzon::array![name.as_str(), *index]),
    )
    .into()
}

impl Chip {
    pub fn to_json(&self) -> JsonValue {
        jzon::object! {
            idcode_part: self.idcode_part,
            ipads: self.ipads,
            banks: self.banks,
            has_vref: self.has_vref,
            bs_cols: self.bs_cols,
            xfer_cols: self.xfer_cols.clone(),
            imux_width: self.imux_width,
            mc_width: self.mc_width,
            bs_layout: match self.bs_layout {
                BsLayout::Narrow => "NARROW",
                BsLayout::Wide => "WIDE",
            },
            fb_rows: self.fb_rows,
            fb_cols: self.fb_cols.clone(),
            ios: jzon::object::Object::from_iter(
                self.io.iter().map(|(&crd, io_data)| (match crd {
                    IoId::Mc((fb, mc)) => format!("IOB_{fb}_{mc}"),
                    IoId::Ipad(ip) => format!("IPAD{ip}"),
                }, jzon::object! {
                    bank: io_data.bank.to_idx(),
                    pad_distance: io_data.pad_distance,
                }))
            ),
            io_special: jzon::object::Object::from_iter(
                self.io_special.iter().map(|(key, (fb, mc))| {
                    (key, format!("IOB_{fb}_{mc}"))
                })
            ),
            mc_bits: &self.mc_bits,
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
            jed_mc_bits_small: jed_bits_to_json(&self.jed_mc_bits_small),
            jed_mc_bits_large_iob: jed_bits_to_json(&self.jed_mc_bits_large_iob),
            jed_mc_bits_large_buried: jed_bits_to_json(&self.jed_mc_bits_large_buried),
        }
    }
}
