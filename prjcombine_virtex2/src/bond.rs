use itertools::Itertools;
use prjcombine_int::grid::EdgeIoCoord;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum GtPin {
    RxP,
    RxN,
    TxP,
    TxN,
    GndA,
    VtRx,
    VtTx,
    AVccAuxRx,
    AVccAuxTx,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CfgPin {
    Tck,
    Tdi,
    Tdo,
    Tms,
    Cclk,
    Done,
    ProgB,
    M0,
    M1,
    M2,
    HswapEn,
    PwrdwnB,
    Suspend,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum BondPin {
    Io(EdgeIoCoord),
    Gt(u32, GtPin),
    Nc,
    Rsvd,
    Gnd,
    VccInt,
    VccAux,
    VccO(u32),
    VccBatt,
    Cfg(CfgPin),
    Dxn,
    Dxp,
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
                        GtPin::RxP => format!("GT{bank}_RXP"),
                        GtPin::RxN => format!("GT{bank}_RXN"),
                        GtPin::TxP => format!("GT{bank}_TXP"),
                        GtPin::TxN => format!("GT{bank}_TXN"),
                        GtPin::GndA => format!("GT{bank}_GNDA"),
                        GtPin::VtRx => format!("GT{bank}_VTRX"),
                        GtPin::VtTx => format!("GT{bank}_VTTX"),
                        GtPin::AVccAuxRx => format!("GT{bank}_AVCCAUXRX"),
                        GtPin::AVccAuxTx => format!("GT{bank}_AVCCAUXTX"),
                    },
                    BondPin::Gnd => "GND".to_string(),
                    BondPin::VccO(bank) => format!("VCCO{bank}"),
                    BondPin::Nc => "NC".to_string(),
                    BondPin::Cfg(cfg_pin) => match cfg_pin {
                        CfgPin::Cclk => "CCLK",
                        CfgPin::Done => "DONE",
                        CfgPin::ProgB => "PROG_B",
                        CfgPin::PwrdwnB => "PWRDWN_B",
                        CfgPin::M0 => "M0",
                        CfgPin::M1 => "M1",
                        CfgPin::M2 => "M2",
                        CfgPin::Tck => "TCK",
                        CfgPin::Tms => "TMS",
                        CfgPin::Tdi => "TDI",
                        CfgPin::Tdo => "TDO",
                        CfgPin::HswapEn => "HSWAP_EN",
                        CfgPin::Suspend => "SUSPEND",
                    }.to_string(),
                    BondPin::VccInt => "VCCINT".to_string(),
                    BondPin::VccAux => "VCCAUX".to_string(),
                    BondPin::VccBatt => "VCCBATT".to_string(),
                    BondPin::Dxn => "DXN".to_string(),
                    BondPin::Dxp => "DXP".to_string(),
                    BondPin::Rsvd => "RSVD".to_string(),
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
                        GtPin::RxP => write!(f, "RXP")?,
                        GtPin::RxN => write!(f, "RXN")?,
                        GtPin::TxP => write!(f, "TXP")?,
                        GtPin::TxN => write!(f, "TXN")?,
                        GtPin::GndA => write!(f, "GNDA")?,
                        GtPin::VtRx => write!(f, "VTRX")?,
                        GtPin::VtTx => write!(f, "VTTX")?,
                        GtPin::AVccAuxRx => write!(f, "AVCCAUXRX")?,
                        GtPin::AVccAuxTx => write!(f, "AVCCAUXTX")?,
                    }
                }
                BondPin::Nc => write!(f, "NC")?,
                BondPin::Gnd => write!(f, "GND")?,
                BondPin::VccInt => write!(f, "VCCINT")?,
                BondPin::VccAux => write!(f, "VCCAUX")?,
                BondPin::VccO(bank) => write!(f, "VCCO{bank}")?,
                BondPin::VccBatt => write!(f, "VCC_BATT")?,
                BondPin::Cfg(CfgPin::Cclk) => write!(f, "CCLK")?,
                BondPin::Cfg(CfgPin::Done) => write!(f, "DONE")?,
                BondPin::Cfg(CfgPin::M0) => write!(f, "M0")?,
                BondPin::Cfg(CfgPin::M1) => write!(f, "M1")?,
                BondPin::Cfg(CfgPin::M2) => write!(f, "M2")?,
                BondPin::Cfg(CfgPin::ProgB) => write!(f, "PROG_B")?,
                BondPin::Cfg(CfgPin::Tck) => write!(f, "TCK")?,
                BondPin::Cfg(CfgPin::Tms) => write!(f, "TMS")?,
                BondPin::Cfg(CfgPin::Tdi) => write!(f, "TDI")?,
                BondPin::Cfg(CfgPin::Tdo) => write!(f, "TDO")?,
                BondPin::Cfg(CfgPin::PwrdwnB) => write!(f, "PWRDWN_B")?,
                BondPin::Cfg(CfgPin::HswapEn) => write!(f, "HSWAP_EN")?,
                BondPin::Cfg(CfgPin::Suspend) => write!(f, "SUSPEND")?,
                BondPin::Dxn => write!(f, "DXN")?,
                BondPin::Dxp => write!(f, "DXP")?,
                BondPin::Rsvd => write!(f, "RSVD")?,
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
