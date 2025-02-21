use std::{collections::BTreeMap, error::Error, fs::File, path::Path};

use prjcombine_types::{FbId, FbMcId, tiledb::Tile};
use serde::{Deserialize, Serialize};
use serde_json::json;
use unnamed_entity::{EntityVec, entity_id};

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

    pub fn to_json(&self) -> serde_json::Value {
        fn fb_bit_to_json(crd: FbBitCoord) -> serde_json::Value {
            json!([crd.row, crd.bit, crd.column])
        }

        fn global_bit_to_json(crd: GlobalBitCoord) -> serde_json::Value {
            json!([crd.fb, crd.row, crd.bit, crd.column])
        }

        json! ({
            "chips": Vec::from_iter(self.chips.values().map(|chip| json! ({
                "kind": match chip.kind {
                    ChipKind::Xc9500 => "xc9500",
                    ChipKind::Xc9500Xl => "xc9500xl",
                    ChipKind::Xc9500Xv => "xc9500xv",
                },
                "idcode": chip.idcode,
                "fbs": chip.fbs,
                "ios": serde_json::Map::from_iter(
                    chip.io.iter().map(|(&(fb, mc), bank)| (format!("IOB_{fb}_{mc}"), json!(bank)))
                ),
                "banks": chip.banks,
                "tdo_bank": chip.tdo_bank,
                "io_special": chip.io_special,
                "imux_bits": chip.imux_bits.to_json(fb_bit_to_json),
                "uim_ibuf_bits": if let Some(ref bits) = chip.uim_ibuf_bits {
                    bits.to_json(global_bit_to_json)
                } else {
                    serde_json::Value::Null
                },
                "program_time": chip.program_time,
                "erase_time": chip.erase_time,
            }))),
            "bonds": Vec::from_iter(
                self.bonds.values().map(|bond| json!({
                    "io_special_override": &bond.io_special_override,
                    "pins": serde_json::Map::from_iter(
                        bond.pins.iter().map(|(k, v)| {
                            (k.clone(), match v {
                                Pad::Nc => "NC".to_string(),
                                Pad::Gnd => "GND".to_string(),
                                Pad::VccInt => "VCCINT".to_string(),
                                Pad::VccIo(bank) => format!("VCCIO{bank}"),
                                Pad::Iob(fb, mc) => format!("IOB_{fb}_{mc}"),
                                Pad::Tms => "TMS".to_string(),
                                Pad::Tck => "TCK".to_string(),
                                Pad::Tdi => "TDI".to_string(),
                                Pad::Tdo => "TDO".to_string(),
                            }.into())
                        })
                    ),
                }))
            ),
            "speeds": &self.speeds,
            "parts": &self.parts,
            "mc_bits": self.mc_bits.to_json(|bit| bit.into()),
            "fb_bits": self.fb_bits.to_json(fb_bit_to_json),
            "global_bits": self.global_bits.to_json(global_bit_to_json),
        })
    }
}
