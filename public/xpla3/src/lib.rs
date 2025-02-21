use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs::File,
    path::Path,
};

use prjcombine_types::{FbId, FbMcId, tiledb::Tile};
use serde::{Deserialize, Serialize};
use serde_json::json;
use unnamed_entity::{EntityVec, entity_id};

entity_id! {
    pub id ChipId u32;
    pub id BondId u32;
    pub id SpeedId u32;
    pub id GclkId u8;
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Chip {
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
    pub chip: ChipId,
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
    pub chips: EntityVec<ChipId, Chip>,
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

    pub fn to_json(&self) -> serde_json::Value {
        fn bit_to_json(crd: BitCoord) -> serde_json::Value {
            json!([crd.row, crd.plane, crd.column])
        }

        json! ({
            "chips": Vec::from_iter(self.chips.values().map(|chip| json! ({
                "idcode_part": chip.idcode_part,
                "bs_cols": chip.bs_cols,
                "imux_width": chip.imux_width,
                "fb_rows": chip.fb_rows,
                "fb_cols": chip.fb_cols,
                "io_mcs": chip.io_mcs,
                "io_special": chip.io_special,
                "global_bits": chip.global_bits.to_json(bit_to_json),
                "jed_global_bits": chip.jed_global_bits,
                "imux_bits": chip.imux_bits.to_json(bit_to_json),
            }))),
            "bonds": Vec::from_iter(
                self.bonds.values().map(|bond| json!({
                    "idcode_part": bond.idcode_part,
                    "pins": serde_json::Map::from_iter(
                        bond.pins.iter().map(|(k, v)| {
                            (k.clone(), match v {
                                Pad::Nc => "NC".to_string(),
                                Pad::Gnd => "GND".to_string(),
                                Pad::Vcc => "VCC".to_string(),
                                Pad::Iob(fb, mc) => format!("IOB_{fb}_{mc}"),
                                Pad::Gclk(gclk) => format!("GCLK{gclk}"),
                                Pad::PortEn => "PORT_EN".to_string(),
                            }.into())
                        })
                    ),
                }))
            ),
            "speeds": &self.speeds,
            "parts": &self.parts,
            "mc_bits": self.mc_bits.to_json(bit_to_json),
            "fb_bits": self.fb_bits.to_json(bit_to_json),
            "jed_mc_bits_iob": &self.jed_mc_bits_iob,
            "jed_mc_bits_buried": &self.jed_mc_bits_buried,
            "jed_fb_bits": &self.jed_fb_bits,
        })
    }
}
