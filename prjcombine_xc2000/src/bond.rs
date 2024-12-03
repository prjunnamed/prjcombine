use std::{collections::BTreeMap, fmt::Display};

use itertools::Itertools;
use prjcombine_int::grid::SimpleIoCoord;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum CfgPin {
    Cclk,
    Done,
    ProgB,
    PwrdwnB,
    M0,
    M1,
    // XC4000 only
    Tdo,
    M2,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum BondPin {
    Io(SimpleIoCoord),
    Gnd,
    Vcc,
    Nc,
    Cfg(CfgPin),
    // XC4000XV only
    VccInt,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandedBond<'a> {
    pub bond: &'a Bond,
    pub ios: BTreeMap<SimpleIoCoord, String>,
}

impl Bond {
    pub fn expand(&self) -> ExpandedBond {
        let mut ios = BTreeMap::new();
        for (name, pad) in &self.pins {
            if let BondPin::Io(io) = *pad {
                ios.insert(io, name.clone());
            }
        }
        ExpandedBond { bond: self, ios }
    }

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "pins": serde_json::Map::from_iter(
                self.pins.iter().map(|(pin, pad)| (pin.clone(), match pad {
                    BondPin::Io(io) => io.to_string(),
                    BondPin::Gnd => "GND".to_string(),
                    BondPin::Vcc => "VCC".to_string(),
                    BondPin::Nc => "NC".to_string(),
                    BondPin::Cfg(cfg_pin) => match cfg_pin {
                        CfgPin::Cclk => "CCLK",
                        CfgPin::Done => "DONE",
                        CfgPin::ProgB => "PROG_B",
                        CfgPin::PwrdwnB => "PWRDWN_B",
                        CfgPin::M0 => "M0",
                        CfgPin::M1 => "M1",
                        CfgPin::Tdo => "TDO",
                        CfgPin::M2 => "M2",
                    }.to_string(),
                    BondPin::VccInt => "VCCINT".to_string(),
                }.into()))
            ),
        })
    }
}

fn pad_sort_key(name: &str) -> (usize, &str, u32) {
    let pos = name.find(|x: char| x.is_ascii_digit()).unwrap();
    (pos, &name[..pos], name[pos..].parse().unwrap())
}

impl Display for Bond {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tPINS:")?;
        for (pin, pad) in self.pins.iter().sorted_by_key(|(k, _)| pad_sort_key(k)) {
            write!(f, "\t\t{pin:4}: ")?;
            match pad {
                BondPin::Io(io) => {
                    write!(f, "IOB_X{x}Y{y}B{b}", x = io.col, y = io.row, b = io.iob)?
                }
                BondPin::Gnd => write!(f, "GND")?,
                BondPin::Vcc => write!(f, "VCC")?,
                BondPin::VccInt => write!(f, "VCCINT")?,
                BondPin::Nc => write!(f, "NC")?,
                BondPin::Cfg(CfgPin::Cclk) => write!(f, "CCLK")?,
                BondPin::Cfg(CfgPin::Done) => write!(f, "DONE")?,
                BondPin::Cfg(CfgPin::ProgB) => write!(f, "PROG_B")?,
                BondPin::Cfg(CfgPin::M0) => write!(f, "M0")?,
                BondPin::Cfg(CfgPin::M1) => write!(f, "M1")?,
                BondPin::Cfg(CfgPin::M2) => write!(f, "M2")?,
                BondPin::Cfg(CfgPin::Tdo) => write!(f, "TDO")?,
                BondPin::Cfg(CfgPin::PwrdwnB) => write!(f, "PWRDWN_B")?,
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
