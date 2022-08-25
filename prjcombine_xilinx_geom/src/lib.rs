#![allow(clippy::collapsible_else_if)]

use prjcombine_entity::{entity_id, EntityId, EntityVec};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub mod series7;
pub mod spartan6;
pub mod ultrascale;
pub mod versal;
pub mod virtex;
pub mod virtex2;
pub mod virtex4;
pub mod virtex5;
pub mod virtex6;
pub mod xc4k;
pub mod xc5200;

pub mod eint;
pub mod int;

entity_id! {
    pub id GridId usize;
    pub id BondId usize;
    pub id DevBondId usize;
    pub id DevSpeedId usize;

    pub id ColId u16;
    pub id RowId u16;
    pub id SlrId u16;
    pub id BelId u16;
}

impl core::ops::Add<usize> for ColId {
    type Output = ColId;
    fn add(self, x: usize) -> ColId {
        ColId::from_idx(self.to_idx() + x)
    }
}

impl core::ops::AddAssign<usize> for ColId {
    fn add_assign(&mut self, x: usize) {
        *self = *self + x;
    }
}

impl core::ops::Sub<usize> for ColId {
    type Output = ColId;
    fn sub(self, x: usize) -> ColId {
        ColId::from_idx(self.to_idx() - x)
    }
}

impl core::ops::SubAssign<usize> for ColId {
    fn sub_assign(&mut self, x: usize) {
        *self = *self - x;
    }
}

impl core::ops::Add<usize> for RowId {
    type Output = RowId;
    fn add(self, x: usize) -> RowId {
        RowId::from_idx(self.to_idx() + x)
    }
}

impl core::ops::AddAssign<usize> for RowId {
    fn add_assign(&mut self, x: usize) {
        *self = *self + x;
    }
}

impl core::ops::Sub<usize> for RowId {
    type Output = RowId;
    fn sub(self, x: usize) -> RowId {
        RowId::from_idx(self.to_idx() - x)
    }
}

impl core::ops::SubAssign<usize> for RowId {
    fn sub_assign(&mut self, x: usize) {
        *self = *self - x;
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Grid {
    Xc4k(xc4k::Grid),
    Xc5200(xc5200::Grid),
    Virtex(virtex::Grid),
    Virtex2(virtex2::Grid),
    Spartan6(spartan6::Grid),
    Virtex4(virtex4::Grid),
    Virtex5(virtex5::Grid),
    Virtex6(virtex6::Grid),
    Series7(series7::Grid),
    Ultrascale(ultrascale::Grid),
    Versal(versal::Grid),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceBond {
    pub name: String,
    pub bond: BondId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DisabledPart {
    // Virtex-E: primary DLLs are disabled
    VirtexPrimaryDlls,
    // Virtex-E: a BRAM column is disabled
    VirtexBram(ColId),
    // Virtex 6: disable primitive in given row
    Virtex6Emac(RowId),
    Virtex6GtxRow(u32),
    Virtex6SysMon,
    Spartan6Gtp,
    Spartan6Mcb,
    Spartan6ClbColumn(ColId),
    Spartan6BramRegion(ColId, u32),
    Spartan6DspRegion(ColId, u32),
    Region(SlrId, u32),
    Ps,
    VersalHardIp(SlrId, ColId, usize),
    VersalColumn(SlrId, ColId),
    VersalGtRight(SlrId, usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PartNamingKey {
    VersalHdio(SlrId, ColId, usize),
    VersalGtLeft(SlrId, usize),
    VersalGtRight(SlrId, usize),
    VersalVNoc(SlrId, ColId, usize),
    VersalDdrMcBot(usize),
    VersalDdrMcTop(usize),
    VersalXpioBot(usize),
    VersalXpioTop(usize),
    VersalHbmTop(usize),
    // XXX NPS bot
    // XXX NPS top
    // XXX NCRB top
    // XXX ME top
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceCombo {
    pub name: String,
    pub devbond_idx: DevBondId,
    pub speed_idx: DevSpeedId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ExtraDie {
    GtzTop,
    GtzBottom,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Device {
    pub name: String,
    pub grids: EntityVec<SlrId, GridId>,
    pub grid_master: SlrId,
    pub extras: Vec<ExtraDie>,
    pub bonds: EntityVec<DevBondId, DeviceBond>,
    pub speeds: EntityVec<DevSpeedId, String>,
    // valid (bond, speed) pairs
    pub combos: Vec<DeviceCombo>,
    pub disabled: BTreeSet<DisabledPart>,
}

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
    // multi-function
    // v, v2, s3*: Data(0) shared with Din
    // v5: 0-2 are also fs
    Data(u8),

    // the following are s3e+; multi-function
    // used for SPI CS on s3e
    CsoB,
    // s6/v5+
    FcsB,
    // s6/v5+, is also Mosi
    FoeB,
    // s6/v5+
    FweB,
    Ldc(u8),
    Hdc,
    // exists on S3E+; on S3E, VS0:2 are A17:19; s3e has 24 pins, s3a has 26
    // on v5+, 0-15 are also data 16-31
    Addr(u8),
    // v5+
    Rs(u8),

    // dedicated v4+, was shared with Data(0) earlier
    Din,

    // the following are s3a+
    // dedicated
    Suspend,
    // multi-function
    Awake,

    // s6 dedicated
    CmpCsB,
    // s6 multi-function
    CmpClk,
    CmpMosi,
    Scp(u8),
    UserCclk,

    // s7 dedicated
    CfgBvs,
    // s7 multi-function
    AdvB,

    // u dedicated
    PorOverride,
    // u multi-function
    I2cSclk,
    I2cSda,
    PerstN0,
    PerstN1,
    // u+ multi-function
    SmbAlert,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct BelCoord {
    pub col: ColId,
    pub row: RowId,
    pub bel: BelId,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeomDb {
    pub grids: EntityVec<GridId, Grid>,
    pub bonds: EntityVec<BondId, Bond>,
    pub devices: Vec<Device>,
    pub ints: BTreeMap<String, int::IntDb>,
}
