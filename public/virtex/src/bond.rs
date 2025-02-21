use itertools::Itertools;
use jzon::JsonValue;
use prjcombine_interconnect::grid::EdgeIoCoord;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

impl std::fmt::Display for CfgPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CfgPin::Cclk => write!(f, "CCLK"),
            CfgPin::Done => write!(f, "DONE"),
            CfgPin::ProgB => write!(f, "PROG_B"),
            CfgPin::M0 => write!(f, "M0"),
            CfgPin::M1 => write!(f, "M1"),
            CfgPin::M2 => write!(f, "M2"),
            CfgPin::Tck => write!(f, "TCK"),
            CfgPin::Tms => write!(f, "TMS"),
            CfgPin::Tdi => write!(f, "TDI"),
            CfgPin::Tdo => write!(f, "TDO"),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BondPin {
    Clk(u32),
    Io(EdgeIoCoord),
    Nc,
    Gnd,
    VccInt,
    VccAux,
    VccO(u32),
    Cfg(CfgPin),
    Dxn,
    Dxp,
}

impl std::fmt::Display for BondPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPin::Io(io) => write!(f, "{io}"),
            BondPin::Clk(idx) => write!(f, "CLK{idx}"),
            BondPin::Nc => write!(f, "NC"),
            BondPin::Gnd => write!(f, "GND"),
            BondPin::VccInt => write!(f, "VCCINT"),
            BondPin::VccAux => write!(f, "VCCAUX"),
            BondPin::VccO(bank) => write!(f, "VCCO{bank}"),
            BondPin::Cfg(cfg_pin) => write!(f, "{cfg_pin}"),
            BondPin::Dxn => write!(f, "DXN"),
            BondPin::Dxp => write!(f, "DXP"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
    // device bank -> pkg bank
    pub io_banks: BTreeMap<u32, u32>,
    pub vref: BTreeSet<EdgeIoCoord>,
    pub diffp: BTreeSet<EdgeIoCoord>,
    pub diffn: BTreeSet<EdgeIoCoord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandedBond<'a> {
    pub bond: &'a Bond,
    pub ios: BTreeMap<EdgeIoCoord, String>,
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
}

impl From<&Bond> for JsonValue {
    fn from(bond: &Bond) -> Self {
        jzon::object! {
            pins: jzon::object::Object::from_iter(
                bond.pins.iter().map(|(k, v)| (k, v.to_string()))
            ),
            io_banks: jzon::object::Object::from_iter(bond.io_banks.iter().map(|(k, v)| (
                k.to_string(), *v
            ))),
            vref: Vec::from_iter(bond.vref.iter().map(|io| io.to_string())),
            diffp: Vec::from_iter(bond.diffp.iter().map(|io| io.to_string())),
            diffn: Vec::from_iter(bond.diffn.iter().map(|io| io.to_string())),
        }
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
            writeln!(f, "\t\t{pin:4}: {pad}")?;
        }
        writeln!(f, "\tVREF:")?;
        for v in &self.vref {
            writeln!(f, "\t\t{v}")?;
        }
        writeln!(f, "\tDIFFP:")?;
        for v in &self.diffp {
            writeln!(f, "\t\t{v}")?;
        }
        writeln!(f, "\tDIFFN:")?;
        for v in &self.diffn {
            writeln!(f, "\t\t{v}")?;
        }
        Ok(())
    }
}
