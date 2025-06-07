use std::collections::BTreeMap;

use bincode::{Decode, Encode};
use itertools::Itertools;
use jzon::JsonValue;
use prjcombine_interconnect::{dir::DirV, grid::EdgeIoCoord};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum CfgPad {
    CResetB,
    CDone,
    Tck,
    Tms,
    Tdo,
    Tdi,
    TrstB,
}

impl std::fmt::Display for CfgPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CfgPad::Tck => write!(f, "TCK"),
            CfgPad::Tms => write!(f, "TMS"),
            CfgPad::Tdi => write!(f, "TDI"),
            CfgPad::Tdo => write!(f, "TDO"),
            CfgPad::TrstB => write!(f, "TRST_B"),
            CfgPad::CDone => write!(f, "CDONE"),
            CfgPad::CResetB => write!(f, "CRESET_B"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum BondPad {
    Io(EdgeIoCoord),
    IoCDone(EdgeIoCoord),
    IoTriple([EdgeIoCoord; 3]),
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
    Cfg(CfgPad),
    PorTest,
}

impl std::fmt::Display for BondPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPad::Io(io) => write!(f, "{io}"),
            BondPad::IoCDone(io) => write!(f, "{io}_CDONE"),
            BondPad::Nc => write!(f, "NC"),
            BondPad::Gnd => write!(f, "GND"),
            BondPad::GndLed => write!(f, "GNDLED"),
            BondPad::VccInt => write!(f, "VCCINT"),
            BondPad::VccIo(bank) => write!(f, "VCCIO{bank}"),
            BondPad::VccIoSpi => write!(f, "VCCIO_SPI"),
            BondPad::VppPump => write!(f, "VPP_PUMP"),
            BondPad::VppDirect => write!(f, "VPP_DIRECT"),
            BondPad::GndPll(edge) => write!(f, "GNDPLL_{edge}"),
            BondPad::VccPll(edge) => write!(f, "VCCPLL_{edge}"),
            BondPad::Vref => write!(f, "VREF"),
            BondPad::PorTest => write!(f, "POR_TEST"),
            BondPad::Cfg(cfg_pin) => write!(f, "{cfg_pin}"),
            BondPad::IoTriple(ios) => write!(
                f,
                "{io0}_{io1}_{io2}",
                io0 = ios[0],
                io1 = ios[1],
                io2 = ios[2]
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPad>,
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
                BondPad::Io(io) | BondPad::IoCDone(io) => {
                    ios.insert(io, name.clone());
                }
                BondPad::IoTriple(iot) => {
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
