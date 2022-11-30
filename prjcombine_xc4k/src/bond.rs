use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
