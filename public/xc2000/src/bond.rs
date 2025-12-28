use std::collections::BTreeMap;

use bincode::{Decode, Encode};
use itertools::Itertools;
use prjcombine_interconnect::grid::EdgeIoCoord;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum CfgPad {
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

impl std::fmt::Display for CfgPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CfgPad::Cclk => write!(f, "CCLK"),
            CfgPad::Done => write!(f, "DONE"),
            CfgPad::ProgB => write!(f, "PROG_B"),
            CfgPad::PwrdwnB => write!(f, "PWRDWN_B"),
            CfgPad::M0 => write!(f, "M0"),
            CfgPad::M1 => write!(f, "M1"),
            CfgPad::Tdo => write!(f, "TDO"),
            CfgPad::M2 => write!(f, "M2"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum BondPad {
    Io(EdgeIoCoord),
    Gnd,
    Vcc,
    Nc,
    Cfg(CfgPad),
    // XC4000XV only
    VccInt,
}

impl std::fmt::Display for BondPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPad::Io(io) => write!(f, "{io}"),
            BondPad::Gnd => write!(f, "GND"),
            BondPad::Vcc => write!(f, "VCC"),
            BondPad::Nc => write!(f, "NC"),
            BondPad::Cfg(cfg_pin) => write!(f, "{cfg_pin}"),
            BondPad::VccInt => write!(f, "VCCINT"),
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
    pub fn expand(&self) -> ExpandedBond<'_> {
        let mut ios = BTreeMap::new();
        for (name, pad) in &self.pins {
            if let BondPad::Io(io) = *pad {
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

impl Bond {
    pub fn dump(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        for (pin, pad) in self.pins.iter().sorted_by_key(|(k, _)| pad_sort_key(k)) {
            writeln!(o, "\tpin {pin} = {pad};")?;
        }
        Ok(())
    }
}
