use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::grid::IoCoord;

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
    Io(IoCoord),
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
}
