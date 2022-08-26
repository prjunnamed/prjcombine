use crate::BelCoord;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum CfgPin {
    // dedicated
    Tck,
    Tdi,
    Tdo,
    Tms,
    PwrdwnB,
    ProgB,
    Done,
    // multi-function on S3E, S3A, S6; dedicated otherwise
    // M0 is also CMPMISO on s6
    M0,
    M1,
    M2,
    Cclk,
    HswapEn,
    // multi-function on v, v2, s3*; dedicated on v4+
    InitB,
    // shared with Busy
    Dout,
    RdWrB,
    // s3e: shared with Mosi
    CsiB,
    // 0-3 are dedicated on Ultrascale
    Data(u8),

    // dedicated v4+, was shared with Data(0) earlier
    Din,

    // s3a+ dedicated
    Suspend,
    // s6 dedicated
    CmpCsB,
    // s7 dedicated
    CfgBvs,
    // u dedicated
    PorOverride,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtPin {
    RxP,
    RxN,
    TxP,
    TxN,
    ClkP,
    ClkN,
    GndA,
    AVccAuxRx,
    AVccAuxTx,
    AVccAuxMgt,
    VtRx,
    VtTx,
    // v4
    RTerm,
    MgtVRef,
    // v5
    AVcc,
    AVccPll,
    RRef,
    // v6
    RBias,
    AVttRCal,

    GtzAGnd,
    GtzAVcc,
    GtzVccH,
    GtzVccL,
    GtzObsClkP,
    GtzObsClkN,
    GtzThermIn,
    GtzThermOut,
    GtzSenseAGnd,
    GtzSenseGnd,
    GtzSenseGndL,
    GtzSenseAVcc,
    GtzSenseVcc,
    GtzSenseVccL,
    GtzSenseVccH,

    // PS-GTR
    AVtt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtRegionPin {
    // v5; per side
    AVttRxC,
    // v6
    AVtt,
    AVcc,
    GthAVtt,
    GthAVcc,
    GthAVccRx,
    GthAVccPll,
    GthAGnd,
    // s7
    VccAux,
    // us+ GTM
    VccInt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SysMonPin {
    VP,
    VN,
    AVdd,
    AVss,
    VRefP,
    VRefN,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PsPin {
    Mio(u32),
    Clk,
    PorB,
    SrstB,
    DdrDq(u32),
    DdrDm(u32),
    DdrDqsP(u32),
    DdrDqsN(u32),
    DdrA(u32),
    DdrBa(u32),
    DdrVrP,
    DdrVrN,
    DdrCkP(u32),
    DdrCkN(u32),
    DdrCke(u32),
    DdrOdt(u32),
    DdrDrstB,
    DdrCsB(u32),
    DdrRasB,
    DdrCasB,
    DdrWeB,
    // Ps8+
    ErrorOut,
    ErrorStatus,
    Done,
    InitB,
    ProgB,
    JtagTck,
    JtagTdi,
    JtagTdo,
    JtagTms,
    Mode(u32),
    PadI,
    PadO,
    DdrActN,
    DdrAlertN,
    DdrBg(u32),
    DdrParity,
    DdrZq,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HbmPin {
    Vcc,
    VccIo,
    VccAux,
    Rsvd,
    RsvdGnd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DacPin {
    VOutP,
    VOutN,
    ClkP,
    ClkN,
    RExt,
    SysRefP,
    SysRefN,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AdcPin {
    VInP,
    VInN,
    VInPairP,
    VInPairN,
    ClkP,
    ClkN,
    VCm,
    RExt,
    PllTestOutP,
    PllTestOutN,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BondPin {
    IoByCoord(BelCoord),
    // bank, bel idx
    IoByBank(u32, u32),
    // bank, type, bel idx
    GtByBank(u32, GtPin, u32),
    GtByRegion(u32, GtRegionPin),
    // bank, type
    SysMonByBank(u32, SysMonPin),
    Cfg(CfgPin),
    Gnd,
    VccInt,
    VccAux,
    VccBram,
    VccAuxHpio,
    VccAuxHdio,
    VccAuxIo(u32),
    VccIntIo,
    VccO(u32),
    VccBatt,
    Nc,
    Rsvd,
    RsvdGnd,
    Dxp,
    Dxn,
    Vfs,
    RFuse,
    // PS7
    VccPsAux,
    VccPsInt,
    VccPsPll,
    // PS8
    VccPsIntLp,
    VccPsIntFp,
    VccPsIntFpDdr,
    VccPsBatt,
    VccPsDdrPll,
    VccIntVcu,
    // xqrku060 special
    GndSense,
    VccIntSense,
    // for PS7 and ultrascale
    IoVref(u32, u32),
    IoPs(u32, PsPin),
    Hbm(u32, HbmPin),
    // RFSoC
    VccIntAms,
    VccSdfec,
    DacGnd,
    DacSubGnd,
    DacAVcc,
    DacAVccAux,
    DacAVtt,
    AdcGnd,
    AdcSubGnd,
    AdcAVcc,
    AdcAVccAux,
    DacByBank(u32, DacPin, u32),
    AdcByBank(u32, AdcPin, u32),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
    // device bank -> pkg bank
    pub io_banks: BTreeMap<u32, u32>,
}
