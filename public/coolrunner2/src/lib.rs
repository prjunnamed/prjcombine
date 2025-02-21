use std::{collections::BTreeMap, error::Error, fs::File, path::Path};

use prjcombine_types::{FbId, FbMcId, IoId, IpadId, tiledb::Tile};
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
pub struct Chip {
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

    pub fn to_json(&self) -> serde_json::Value {
        fn bit_to_json(crd: BitCoord) -> serde_json::Value {
            json!([crd.row, crd.column])
        }

        json! ({
            "chips": Vec::from_iter(self.chips.values().map(|chip| json! ({
                "idcode_part": chip.idcode_part,
                "ipads": chip.ipads,
                "banks": chip.banks,
                "has_vref": chip.has_vref,
                "bs_cols": chip.bs_cols,
                "xfer_cols": chip.xfer_cols,
                "imux_width": chip.imux_width,
                "mc_width": chip.mc_width,
                "bs_layout": match chip.bs_layout {
                    BsLayout::Narrow => "NARROW",
                    BsLayout::Wide => "WIDE",
                },
                "fb_rows": chip.fb_rows,
                "fb_cols": chip.fb_cols,
                "ios": serde_json::Map::from_iter(
                    chip.io.iter().map(|(&io, bank)| (match io {
                        IoId::Mc((fb, mc)) => format!("IOB_{fb}_{mc}"),
                        IoId::Ipad(ip) => format!("IPAD{ip}"),
                    }, json!(bank)))
                ),
                "io_special": chip.io_special,
                "mc_bits": chip.mc_bits.to_json(bit_to_json),
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
                                Pad::VccInt => "VCCINT".to_string(),
                                Pad::VccIo(bank) => format!("VCCIO{bank}"),
                                Pad::VccAux => "VCCAUX".to_string(),
                                Pad::Iob(fb, mc) => format!("IOB_{fb}_{mc}"),
                                Pad::Ipad(ipad) => format!("IPAD{ipad}"),
                                Pad::Tck => "TCK".to_string(),
                                Pad::Tms => "TMS".to_string(),
                                Pad::Tdi => "TDI".to_string(),
                                Pad::Tdo => "TDO".to_string(),
                            }.into())
                        })
                    ),
                }))
            ),
            "speeds": &self.speeds,
            "parts": &self.parts,
            "jed_mc_bits_small": &self.jed_mc_bits_small,
            "jed_mc_bits_large_iob": &self.jed_mc_bits_large_iob,
            "jed_mc_bits_large_buried": &self.jed_mc_bits_large_buried,
        })
    }
}
