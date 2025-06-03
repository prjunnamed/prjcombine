use std::{collections::BTreeMap, fmt::Display};

use bincode::{Decode, Encode};
use itertools::Itertools;
use jzon::JsonValue;
use prjcombine_interconnect::grid::EdgeIoCoord;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
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

impl std::fmt::Display for CfgPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CfgPin::Cclk => write!(f, "CCLK"),
            CfgPin::Done => write!(f, "DONE"),
            CfgPin::ProgB => write!(f, "PROG_B"),
            CfgPin::PwrdwnB => write!(f, "PWRDWN_B"),
            CfgPin::M0 => write!(f, "M0"),
            CfgPin::M1 => write!(f, "M1"),
            CfgPin::Tdo => write!(f, "TDO"),
            CfgPin::M2 => write!(f, "M2"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum BondPin {
    Io(EdgeIoCoord),
    Gnd,
    Vcc,
    Nc,
    Cfg(CfgPin),
    // XC4000XV only
    VccInt,
}

impl std::fmt::Display for BondPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPin::Io(io) => write!(f, "{io}"),
            BondPin::Gnd => write!(f, "GND"),
            BondPin::Vcc => write!(f, "VCC"),
            BondPin::Nc => write!(f, "NC"),
            BondPin::Cfg(cfg_pin) => write!(f, "{cfg_pin}"),
            BondPin::VccInt => write!(f, "VCCINT"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandedBond<'a> {
    pub bond: &'a Bond,
    pub ios: BTreeMap<EdgeIoCoord, String>,
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

impl From<&Bond> for JsonValue {
    fn from(bond: &Bond) -> Self {
        jzon::object! {
            pins: jzon::object::Object::from_iter(
                bond.pins.iter().map(|(k, v)| (k, v.to_string()))
            ),
        }
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
            writeln!(f, "\t\t{pin:4}: {pad}")?;
        }
        Ok(())
    }
}
