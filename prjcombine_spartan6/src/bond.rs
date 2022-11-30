use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::grid::IoCoord;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CfgPin {
    Tck,
    Tdi,
    Tdo,
    Tms,
    CmpCsB,
    Done,
    ProgB,
    Suspend,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtPin {
    TxP(u8),
    TxN(u8),
    RxP(u8),
    RxN(u8),
    ClkP(u8),
    ClkN(u8),
    AVcc,
    AVccPll(u8),
    VtTx,
    VtRx,
    RRef,
    AVttRCal,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BondPin {
    Io(IoCoord),
    Nc,
    Gnd,
    VccInt,
    VccAux,
    VccO(u32),
    VccBatt,
    Vfs,
    RFuse,
    Cfg(CfgPin),
    Gt(u32, GtPin),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
    // device bank -> pkg bank
    pub io_banks: BTreeMap<u32, u32>,
}
