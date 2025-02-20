use itertools::Itertools;
use prjcombine_interconnect::grid::EdgeIoCoord;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CfgPin {
    Tck,
    Tdi,
    Tdo,
    Tms,
    CmpCsB,
    Done,
    ProgB,
    Suspend,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum GtPin {
    TxP(u8),
    TxN(u8),
    RxP(u8),
    RxN(u8),
    ClkP(u8),
    ClkN(u8),
    AVcc,
    AVccPll(u8),
    VtTx,
    VtRx,
    RRef,
    AVttRCal,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum BondPin {
    Io(EdgeIoCoord),
    Nc,
    Gnd,
    VccInt,
    VccAux,
    VccO(u32),
    VccBatt,
    Vfs,
    RFuse,
    Cfg(CfgPin),
    Gt(u32, GtPin),
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
    // device bank -> pkg bank
    pub io_banks: BTreeMap<u32, u32>,
    pub vref: BTreeSet<EdgeIoCoord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandedBond<'a> {
    pub bond: &'a Bond,
    pub ios: BTreeMap<EdgeIoCoord, String>,
    pub gts: BTreeMap<(u32, GtPin), String>,
}

impl Bond {
    pub fn expand(&self) -> ExpandedBond {
        let mut ios = BTreeMap::new();
        let mut gts = BTreeMap::new();
        for (name, pad) in &self.pins {
            match *pad {
                BondPin::Io(io) => {
                    ios.insert(io, name.clone());
                }
                BondPin::Gt(bank, gtpin) => {
                    gts.insert((bank, gtpin), name.clone());
                }
                _ => (),
            }
        }
        ExpandedBond {
            bond: self,
            ios,
            gts,
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "pins": serde_json::Map::from_iter(
                self.pins.iter().map(|(pin, pad)| (pin.clone(), match pad {
                    BondPin::Io(io) => io.to_string(),
                    BondPin::Gt(bank, pad) => match pad {
                        GtPin::RxP(i) => format!("GT{bank}_RXP{i}"),
                        GtPin::RxN(i) => format!("GT{bank}_RXN{i}"),
                        GtPin::TxP(i) => format!("GT{bank}_TXP{i}"),
                        GtPin::TxN(i) => format!("GT{bank}_TXN{i}"),
                        GtPin::ClkP(i) => format!("GT{bank}_CLKP{i}"),
                        GtPin::ClkN(i) => format!("GT{bank}_CLKN{i}"),
                        GtPin::VtRx => format!("GT{bank}_VTRX"),
                        GtPin::VtTx => format!("GT{bank}_VTTX"),
                        GtPin::AVcc => format!("GT{bank}_AVCC"),
                        GtPin::AVccPll(i) => format!("GT{bank}_AVCCPLL{i}"),
                        GtPin::RRef => format!("GT{bank}_RREF"),
                        GtPin::AVttRCal => format!("GT{bank}_AVTTRCAL"),
                    },
                    BondPin::Gnd => "GND".to_string(),
                    BondPin::VccO(bank) => format!("VCCO{bank}"),
                    BondPin::Nc => "NC".to_string(),
                    BondPin::Cfg(cfg_pin) => match cfg_pin {
                        CfgPin::Done => "DONE",
                        CfgPin::ProgB => "PROG_B",
                        CfgPin::Tck => "TCK",
                        CfgPin::Tms => "TMS",
                        CfgPin::Tdi => "TDI",
                        CfgPin::Tdo => "TDO",
                        CfgPin::Suspend => "SUSPEND",
                        CfgPin::CmpCsB => "CMP_CS_B",
                    }.to_string(),
                    BondPin::VccInt => "VCCINT".to_string(),
                    BondPin::VccAux => "VCCAUX".to_string(),
                    BondPin::VccBatt => "VCCBATT".to_string(),
                    BondPin::Vfs => "VFS".to_string(),
                    BondPin::RFuse => "RFUSE".to_string(),
                }.into()))
            ),
            "io_banks": serde_json::Map::from_iter(self.io_banks.iter().map(|(k, v)| (
                k.to_string(), (*v).into()
            ))),
            "vref": Vec::from_iter(self.vref.iter().map(|io| io.to_string())),
        })
    }
}

fn pad_sort_key(name: &str) -> (usize, &str, u32) {
    let pos = name.find(|x: char| x.is_ascii_digit()).unwrap();
    (pos, &name[..pos], name[pos..].parse().unwrap())
}

impl std::fmt::Display for Bond {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tBANKS:")?;
        for (k, v) in &self.io_banks {
            writeln!(f, "\t\t{k}: {v}")?;
        }
        writeln!(f, "\tPINS:")?;
        for (pin, pad) in self.pins.iter().sorted_by_key(|(k, _)| pad_sort_key(k)) {
            write!(f, "\t\t{pin:4}: ")?;
            match pad {
                BondPin::Io(io) => write!(f, "{io}")?,
                BondPin::Gt(bank, gtpin) => {
                    write!(f, "GT{bank}.")?;
                    match gtpin {
                        GtPin::RxP(idx) => write!(f, "RXP{idx}")?,
                        GtPin::RxN(idx) => write!(f, "RXN{idx}")?,
                        GtPin::TxP(idx) => write!(f, "TXP{idx}")?,
                        GtPin::TxN(idx) => write!(f, "TXN{idx}")?,
                        GtPin::VtRx => write!(f, "VTRX")?,
                        GtPin::VtTx => write!(f, "VTTX")?,
                        GtPin::ClkP(idx) => write!(f, "CLKP{idx}")?,
                        GtPin::ClkN(idx) => write!(f, "CLKN{idx}")?,
                        GtPin::AVcc => write!(f, "AVCC")?,
                        GtPin::AVccPll(idx) => write!(f, "AVCCPLL{idx}")?,
                        GtPin::RRef => write!(f, "RREF")?,
                        GtPin::AVttRCal => write!(f, "AVTTRCAL")?,
                    }
                }
                BondPin::Nc => write!(f, "NC")?,
                BondPin::Gnd => write!(f, "GND")?,
                BondPin::VccInt => write!(f, "VCCINT")?,
                BondPin::VccAux => write!(f, "VCCAUX")?,
                BondPin::VccO(bank) => write!(f, "VCCO{bank}")?,
                BondPin::VccBatt => write!(f, "VCC_BATT")?,
                BondPin::Cfg(CfgPin::Done) => write!(f, "DONE")?,
                BondPin::Cfg(CfgPin::ProgB) => write!(f, "PROG_B")?,
                BondPin::Cfg(CfgPin::Tck) => write!(f, "TCK")?,
                BondPin::Cfg(CfgPin::Tms) => write!(f, "TMS")?,
                BondPin::Cfg(CfgPin::Tdi) => write!(f, "TDI")?,
                BondPin::Cfg(CfgPin::Tdo) => write!(f, "TDO")?,
                BondPin::Cfg(CfgPin::Suspend) => write!(f, "SUSPEND")?,
                BondPin::Cfg(CfgPin::CmpCsB) => write!(f, "CMPCS_B")?,
                BondPin::Vfs => write!(f, "VFS")?,
                BondPin::RFuse => write!(f, "RFUSE")?,
            }
            writeln!(f)?;
        }
        writeln!(f, "\tVREF:")?;
        for v in &self.vref {
            writeln!(f, "\t\t{v}")?;
        }
        Ok(())
    }
}
