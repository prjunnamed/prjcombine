use std::collections::BTreeMap;

use bincode::{Decode, Encode};
use itertools::Itertools;
use jzon::JsonValue;
use prjcombine_interconnect::{dir::DirH, grid::EdgeIoCoord};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum CfgPad {
    Tck,
    Tdi,
    Tdo,
    Tms,
    Cclk,
    Done,
    ProgB,
    InitB,
    M0,
    M1,
    M2,
    SleepB,
}

impl std::fmt::Display for CfgPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CfgPad::Cclk => write!(f, "CCLK"),
            CfgPad::Done => write!(f, "DONE"),
            CfgPad::M0 => write!(f, "M0"),
            CfgPad::M1 => write!(f, "M1"),
            CfgPad::M2 => write!(f, "M2"),
            CfgPad::ProgB => write!(f, "PROG_B"),
            CfgPad::InitB => write!(f, "INIT_B"),
            CfgPad::Tck => write!(f, "TCK"),
            CfgPad::Tms => write!(f, "TMS"),
            CfgPad::Tdi => write!(f, "TDI"),
            CfgPad::Tdo => write!(f, "TDO"),
            CfgPad::SleepB => write!(f, "SLEEP_B"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum PllSet {
    All,
    Side(DirH),
}

impl std::fmt::Display for PllSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PllSet::All => write!(f, "ALL"),
            PllSet::Side(dir) => write!(f, "{dir}"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum BondPad {
    Io(EdgeIoCoord),
    Nc,
    Gnd,
    VccInt,
    VccAux,
    VccJtag,
    VccIo(u32),
    VccPll(PllSet),
    GndPll(PllSet),
    Cfg(CfgPad),
    XRes,
    Other,
}

impl std::fmt::Display for BondPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPad::Io(io) => write!(f, "{io}"),
            BondPad::Nc => write!(f, "NC"),
            BondPad::Gnd => write!(f, "GND"),
            BondPad::VccInt => write!(f, "VCCINT"),
            BondPad::VccAux => write!(f, "VCCAUX"),
            BondPad::VccJtag => write!(f, "VCC_JTAG"),
            BondPad::VccIo(bank) => write!(f, "VCCIO{bank}"),
            BondPad::VccPll(set) => write!(f, "VCCPLL_{set}"),
            BondPad::GndPll(set) => write!(f, "GNDPLL_{set}"),
            BondPad::Cfg(cfg_pin) => write!(f, "{cfg_pin}"),
            BondPad::XRes => write!(f, "XRES"),
            BondPad::Other => write!(f, "OTHER"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPad>,
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
