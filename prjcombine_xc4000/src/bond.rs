use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Display};

use crate::grid::IoCoord;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CfgPin {
    Tdo,
    Cclk,
    Done,
    ProgB,
    PwrdwnB,
    M0,
    M1,
    M2,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BondPin {
    Io(IoCoord),
    Nc,
    Gnd,
    VccInt,
    VccO,
    Cfg(CfgPin),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandedBond<'a> {
    pub bond: &'a Bond,
    pub ios: BTreeMap<IoCoord, String>,
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
                BondPin::Nc => write!(f, "NC")?,
                BondPin::Gnd => write!(f, "GND")?,
                BondPin::VccInt => write!(f, "VCCINT")?,
                BondPin::VccO => write!(f, "VCCO")?,
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
