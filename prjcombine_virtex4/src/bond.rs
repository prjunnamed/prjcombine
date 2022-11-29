use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CfgPin {
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
    HswapEn,
    // [V7] these 4 are shared instead
    RdWrB,
    CsiB,
    Din,
    Dout,
    // V4 only
    PwrdwnB,
    // V7 only
    CfgBvs,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtPin {
    RxP(u8),
    RxN(u8),
    TxP(u8),
    TxN(u8),
    ClkP(u8),
    ClkN(u8),
    // V4
    GndA,
    AVccAuxRx(u8),
    AVccAuxTx,
    AVccAuxMgt,
    RTerm,
    MgtVRef,
    // V4, V5
    VtRx(u8),
    VtTx(u8),
    // V5
    AVcc,
    AVccPll,
    // V5, V6, V7
    RRef,
    // V6, V7
    AVttRCal,
    // V6
    RBias,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SysMonPin {
    VP,
    VN,
    AVss,
    AVdd,
    VRefP,
    VRefN,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BondPin {
    // bank, pin within bank
    Io(u32, u32),
    Nc,
    Gnd,
    VccInt,
    VccAux,
    VccO(u32),
    VccBatt,
    Cfg(CfgPin),
    Gt(u32, GtPin),
    Dxp,
    Dxn,
    SysMon(u32, SysMonPin),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    // ×32
    // [V5+] high 16 bits are also low 16 bits of Addr
    // [V5, V6] 0-2 double as FS
    // [V7] 0 doubles as MOSI, 1 doubles as DIN
    Data(u8),
    // the following are V5+
    Addr(u8), // ×26 [V5, V6] or ×29 [V7] total, but 0-15 are represented as Data(16-31)
    Rs(u8),   // ×2
    CsoB,     // [V7] doubles as DOUT
    FweB,
    FoeB, // [V5, V6] doubles as MOSI
    FcsB,
    // the following are V7-only
    CsiB,
    RdWrB,
    EmCclk,
    PudcB,
    AdvB,
}
