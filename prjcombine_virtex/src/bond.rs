use prjcombine_int::grid::SimpleIoCoord;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
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
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum BondPin {
    Clk(u32),
    Io(SimpleIoCoord),
    Nc,
    Gnd,
    VccInt,
    VccAux,
    VccO(u32),
    Cfg(CfgPin),
    Dxn,
    Dxp,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
    // device bank -> pkg bank
    pub io_banks: BTreeMap<u32, u32>,
    pub vref: BTreeSet<SimpleIoCoord>,
    pub diffp: BTreeSet<SimpleIoCoord>,
    pub diffn: BTreeSet<SimpleIoCoord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandedBond<'a> {
    pub bond: &'a Bond,
    pub ios: BTreeMap<SimpleIoCoord, String>,
    pub clks: BTreeMap<u32, String>,
}

impl Bond {
    pub fn expand(&self) -> ExpandedBond {
        let mut ios = BTreeMap::new();
        let mut clks = BTreeMap::new();
        for (name, pad) in &self.pins {
            match *pad {
                BondPin::Io(io) => {
                    ios.insert(io, name.clone());
                }
                BondPin::Clk(bank) => {
                    clks.insert(bank, name.clone());
                }
                _ => (),
            }
        }
        ExpandedBond {
            bond: self,
            ios,
            clks,
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "pins": serde_json::Map::from_iter(
                self.pins.iter().map(|(pin, pad)| (pin.clone(), match pad {
                    BondPin::Io(io) => io.to_string(),
                    BondPin::Clk(bank) => format!("GCLK{bank}"),
                    BondPin::Gnd => "GND".to_string(),
                    BondPin::VccO(bank) => format!("VCCO{bank}"),
                    BondPin::Nc => "NC".to_string(),
                    BondPin::Cfg(cfg_pin) => match cfg_pin {
                        CfgPin::Cclk => "CCLK",
                        CfgPin::Done => "DONE",
                        CfgPin::ProgB => "PROG_B",
                        CfgPin::M0 => "M0",
                        CfgPin::M1 => "M1",
                        CfgPin::M2 => "M2",
                        CfgPin::Tck => "TCK",
                        CfgPin::Tms => "TMS",
                        CfgPin::Tdi => "TDI",
                        CfgPin::Tdo => "TDO",
                    }.to_string(),
                    BondPin::VccInt => "VCCINT".to_string(),
                    BondPin::VccAux => "VCCAUX".to_string(),
                    BondPin::Dxn => "DXN".to_string(),
                    BondPin::Dxp => "DXP".to_string(),
                }.into()))
            ),
            "io_banks": serde_json::Map::from_iter(self.io_banks.iter().map(|(k, v)| (
                k.to_string(), (*v).into()
            ))),
            "vref": Vec::from_iter(self.vref.iter().map(|io| io.to_string())),
            "diffp": Vec::from_iter(self.diffp.iter().map(|io| io.to_string())),
            "diffn": Vec::from_iter(self.diffn.iter().map(|io| io.to_string())),
        })
    }
}
