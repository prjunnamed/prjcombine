use prjcombine_int::grid::SimpleIoCoord;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
}
