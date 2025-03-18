use std::collections::BTreeMap;

use itertools::Itertools;
use jzon::JsonValue;
use prjcombine_interconnect::{dir::DirV, grid::EdgeIoCoord};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CfgPin {
    CResetB,
    CDone,
    Tck,
    Tms,
    Tdo,
    Tdi,
    TrstB,
}

impl std::fmt::Display for CfgPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CfgPin::Tck => write!(f, "TCK"),
            CfgPin::Tms => write!(f, "TMS"),
            CfgPin::Tdi => write!(f, "TDI"),
            CfgPin::Tdo => write!(f, "TDO"),
            CfgPin::TrstB => write!(f, "TRST_B"),
            CfgPin::CDone => write!(f, "CDONE"),
            CfgPin::CResetB => write!(f, "CRESET_B"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum BondPin {
    Io(EdgeIoCoord),
    IoCDone(EdgeIoCoord),
    Nc,
    Gnd,
    VccInt,
    VccIo(u32),
    VccIoSpi,
    VppPump,
    VppDirect,
    Vref,
    GndPll(DirV),
    VccPll(DirV),
    GndLed,
    Cfg(CfgPin),
    PorTest,
    IoTriple([EdgeIoCoord; 3]),
}

impl std::fmt::Display for BondPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPin::Io(io) => write!(f, "{io}"),
            BondPin::IoCDone(io) => write!(f, "{io}_CDONE"),
            BondPin::Nc => write!(f, "NC"),
            BondPin::Gnd => write!(f, "GND"),
            BondPin::GndLed => write!(f, "GNDLED"),
            BondPin::VccInt => write!(f, "VCCINT"),
            BondPin::VccIo(bank) => write!(f, "VCCIO{bank}"),
            BondPin::VccIoSpi => write!(f, "VCCIO_SPI"),
            BondPin::VppPump => write!(f, "VPP_PUMP"),
            BondPin::VppDirect => write!(f, "VPP_DIRECT"),
            BondPin::GndPll(edge) => write!(f, "GNDPLL_{edge}"),
            BondPin::VccPll(edge) => write!(f, "VCCPLL_{edge}"),
            BondPin::Vref => write!(f, "VREF"),
            BondPin::PorTest => write!(f, "POR_TEST"),
            BondPin::Cfg(cfg_pin) => write!(f, "{cfg_pin}"),
            BondPin::IoTriple(ios) => write!(
                f,
                "{io0}_{io1}_{io2}",
                io0 = ios[0],
                io1 = ios[1],
                io2 = ios[2]
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
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
            match *pad {
                BondPin::Io(io) | BondPin::IoCDone(io) => {
                    ios.insert(io, name.clone());
                }
                BondPin::IoTriple(iot) => {
                    for io in iot {
                        ios.insert(io, name.clone());
                    }
                }
                _ => (),
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

impl std::fmt::Display for Bond {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tPINS:")?;
        for (pin, pad) in self.pins.iter().sorted_by_key(|(k, _)| pad_sort_key(k)) {
            writeln!(f, "\t\t{pin:4}: {pad}")?;
        }
        Ok(())
    }
}
