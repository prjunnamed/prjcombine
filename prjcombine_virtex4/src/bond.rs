use serde::{Deserialize, Serialize};
use serde_json::json;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum GtRegion {
    All,
    S,
    N,
    L,
    R,
    LS,
    RS,
    LN,
    RN,
    H,
    LH,
    RH,
    Num(u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum GtRegionPin {
    AVtt,
    AGnd,
    AVcc,
    AVccRx,
    AVccPll,
    AVttRxC,
    VccAux,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SysMonPin {
    VP,
    VN,
    AVss,
    AVdd,
    VRefP,
    VRefN,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum GtzPin {
    RxP(u8),
    RxN(u8),
    TxP(u8),
    TxN(u8),
    ClkP(u8),
    ClkN(u8),
    AGnd,
    AVcc,
    VccH,
    VccL,
    ObsClkP,
    ObsClkN,
    ThermIn,
    ThermOut,
    SenseAGnd,
    SenseGnd,
    SenseGndL,
    SenseAVcc,
    SenseVcc,
    SenseVccL,
    SenseVccH,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
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
    DdrCkP,
    DdrCkN,
    DdrCke,
    DdrOdt,
    DdrDrstB,
    DdrCsB,
    DdrRasB,
    DdrCasB,
    DdrWeB,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum BondPin {
    // bank, pin within bank
    Io(u32, u32),
    Nc,
    Gnd,
    Rsvd,
    VccInt,
    VccAux,
    VccBram,
    VccO(u32),
    VccBatt,
    VccAuxIo(u32),
    RsvdGnd,
    Cfg(CfgPin),
    Gt(u32, GtPin),
    Gtz(u32, GtzPin),
    GtRegion(GtRegion, GtRegionPin),
    Dxp,
    Dxn,
    Vfs,
    SysMon(u32, SysMonPin),
    VccPsInt,
    VccPsAux,
    VccPsPll,
    PsVref(u32, u32),
    PsIo(u32, PsPin),
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandedBond<'a> {
    pub bond: &'a Bond,
    pub ios: BTreeMap<(u32, u32), String>,
    pub gts: BTreeMap<(u32, GtPin), String>,
    pub gtzs: BTreeMap<(u32, GtzPin), String>,
    pub sysmons: BTreeMap<(u32, SysMonPin), String>,
}

impl Bond {
    pub fn expand(&self) -> ExpandedBond {
        let mut ios = BTreeMap::new();
        let mut gts = BTreeMap::new();
        let mut gtzs = BTreeMap::new();
        let mut sysmons = BTreeMap::new();
        for (name, pad) in &self.pins {
            match *pad {
                BondPin::Io(bank, idx) => {
                    ios.insert((bank, idx), name.clone());
                }
                BondPin::Gt(bank, gtpin) => {
                    gts.insert((bank, gtpin), name.clone());
                }
                BondPin::Gtz(bank, gtpin) => {
                    gtzs.insert((bank, gtpin), name.clone());
                }
                BondPin::SysMon(bank, smpin) => {
                    sysmons.insert((bank, smpin), name.clone());
                }
                _ => (),
            }
        }
        ExpandedBond {
            bond: self,
            ios,
            gts,
            gtzs,
            sysmons,
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "pins": serde_json::Map::from_iter(
                self.pins.iter().map(|(pin, pad)| (pin.clone(), match pad {
                    BondPin::Io(bank, io) => format!("IO:{bank}:{io}"),
                    BondPin::Gt(bank, pad) => match pad {
                        GtPin::RxP(i) => format!("GT{bank}_RXP{i}"),
                        GtPin::RxN(i) => format!("GT{bank}_RXN{i}"),
                        GtPin::TxP(i) => format!("GT{bank}_TXP{i}"),
                        GtPin::TxN(i) => format!("GT{bank}_TXN{i}"),
                        GtPin::ClkP(i) => format!("GT{bank}_CLKP{i}"),
                        GtPin::ClkN(i) => format!("GT{bank}_CLKN{i}"),
                        GtPin::GndA => format!("GT{bank}_GNDA"),
                        GtPin::AVcc => format!("GT{bank}_AVCC"),
                        GtPin::AVccPll => format!("GT{bank}_AVCCPLL"),
                        GtPin::AVccAuxRx(i) => format!("GT{bank}_AVCCAUXRX{i}"),
                        GtPin::AVccAuxTx => format!("GT{bank}_AVCCAUXTX"),
                        GtPin::AVccAuxMgt => format!("GT{bank}_AVCCAUXMGT"),
                        GtPin::AVttRCal => format!("GT{bank}_AVTTRCAL"),
                        GtPin::VtTx(i) => format!("GT{bank}_VTTX{i}"),
                        GtPin::VtRx(i) => format!("GT{bank}_VTRX{i}"),
                        GtPin::RTerm => format!("GT{bank}_RTERM"),
                        GtPin::RRef => format!("GT{bank}_RREF"),
                        GtPin::RBias => format!("GT{bank}_RBIAS"),
                        GtPin::MgtVRef => format!("GT{bank}_MGTVREF"),
                    },
                    BondPin::GtRegion(reg, pad) => format!("GT_{reg}_{pad}", reg = match reg {
                        GtRegion::All => "ALL".to_string(),
                        GtRegion::S => "S".to_string(),
                        GtRegion::N => "N".to_string(),
                        GtRegion::L => "L".to_string(),
                        GtRegion::R => "R".to_string(),
                        GtRegion::LS => "LS".to_string(),
                        GtRegion::RS => "RS".to_string(),
                        GtRegion::LN => "LN".to_string(),
                        GtRegion::RN => "RN".to_string(),
                        GtRegion::H => "H".to_string(),
                        GtRegion::LH => "LH".to_string(),
                        GtRegion::RH => "RH".to_string(),
                        GtRegion::Num(i) => format!("REG{i}"),
                    }, pad = match pad {
                        GtRegionPin::AVtt => "AVTT",
                        GtRegionPin::AGnd => "AGND",
                        GtRegionPin::AVcc => "AVCC",
                        GtRegionPin::AVccRx => "AVCCRX",
                        GtRegionPin::AVccPll => "AVCCPLL",
                        GtRegionPin::AVttRxC => "AVTTRXC",
                        GtRegionPin::VccAux => "VCCAUX",
                    }),
                    BondPin::Gtz(bank, pad) => match pad {
                        GtzPin::RxP(i) => format!("GTZ{bank}_RXP{i}"),
                        GtzPin::RxN(i) => format!("GTZ{bank}_RXN{i}"),
                        GtzPin::TxP(i) => format!("GTZ{bank}_TXP{i}"),
                        GtzPin::TxN(i) => format!("GTZ{bank}_TXN{i}"),
                        GtzPin::ClkP(i) => format!("GTZ{bank}_CLKP{i}"),
                        GtzPin::ClkN(i) => format!("GTZ{bank}_CLKN{i}"),
                        GtzPin::AGnd => format!("GTZ{bank}_AGND"),
                        GtzPin::AVcc => format!("GTZ{bank}_AVCC"),
                        GtzPin::VccH => format!("GTZ{bank}_VCCH"),
                        GtzPin::VccL => format!("GTZ{bank}_VCCL"),
                        GtzPin::ObsClkP => format!("GTZ{bank}_OBS_CLKP"),
                        GtzPin::ObsClkN => format!("GTZ{bank}_OBS_CLKN"),
                        GtzPin::ThermIn => format!("GTZ{bank}_THERM_IN"),
                        GtzPin::ThermOut => format!("GTZ{bank}_THERM_OUT"),
                        GtzPin::SenseAGnd => format!("GTZ{bank}_SENSE_AGND"),
                        GtzPin::SenseGnd => format!("GTZ{bank}_SENSE_GND"),
                        GtzPin::SenseGndL => format!("GTZ{bank}_SENSE_GNDL"),
                        GtzPin::SenseAVcc => format!("GTZ{bank}_SENSE_AVCC"),
                        GtzPin::SenseVcc => format!("GTZ{bank}_SENSE_VCC"),
                        GtzPin::SenseVccL => format!("GTZ{bank}_SENSE_VCCL"),
                        GtzPin::SenseVccH => format!("GTZ{bank}_SENSE_VCCH"),
                    },
                    BondPin::SysMon(bank, pad) => match pad {
                        SysMonPin::VP => format!("SYSMON{bank}_VP"),
                        SysMonPin::VN => format!("SYSMON{bank}_VN"),
                        SysMonPin::AVss => format!("SYSMON{bank}_AVSS"),
                        SysMonPin::AVdd => format!("SYSMON{bank}_AVDD"),
                        SysMonPin::VRefP => format!("SYSMON{bank}_VREFP"),
                        SysMonPin::VRefN => format!("SYSMON{bank}_VREFN"),
                    },
                    BondPin::PsIo(bank, pad) => match pad {
                        PsPin::Mio(i) => format!("PS{bank}_MIO{i}"),
                        PsPin::Clk => format!("PS{bank}_CLK"),
                        PsPin::PorB => format!("PS{bank}_POR_B"),
                        PsPin::SrstB => format!("PS{bank}_SRST_B"),
                        PsPin::DdrDq(i) => format!("PS{bank}_DDR_DQ{i}"),
                        PsPin::DdrDm(i) => format!("PS{bank}_DDR_DM{i}"),
                        PsPin::DdrDqsP(i) => format!("PS{bank}_DDR_DQS{i}P"),
                        PsPin::DdrDqsN(i) => format!("PS{bank}_DDR_DQS{i}N"),
                        PsPin::DdrA(i) => format!("PS{bank}_DDR_A{i}"),
                        PsPin::DdrBa(i) => format!("PS{bank}_DDR_BA{i}"),
                        PsPin::DdrVrP => format!("PS{bank}_DDR_VRP"),
                        PsPin::DdrVrN => format!("PS{bank}_DDR_VRN"),
                        PsPin::DdrCkP => format!("PS{bank}_DDR_CKP"),
                        PsPin::DdrCkN => format!("PS{bank}_DDR_CKN"),
                        PsPin::DdrCke => format!("PS{bank}_DDR_CKE"),
                        PsPin::DdrOdt => format!("PS{bank}_DDR_ODT"),
                        PsPin::DdrDrstB => format!("PS{bank}_DDR_DRST_B"),
                        PsPin::DdrCsB => format!("PS{bank}_DDR_CS_B"),
                        PsPin::DdrRasB => format!("PS{bank}_DDR_RAS_B"),
                        PsPin::DdrCasB => format!("PS{bank}_DDR_CAS_B"),
                        PsPin::DdrWeB => format!("PS{bank}_DDR_WE_B"),
                    },
                    BondPin::PsVref(bank, i) => format!("PS{bank}_VREF{i}"),
                    BondPin::Gnd => "GND".to_string(),
                    BondPin::VccO(bank) => format!("VCCO{bank}"),
                    BondPin::Nc => "NC".to_string(),
                    BondPin::Cfg(cfg_pin) => match cfg_pin {
                        CfgPin::Cclk => "CCLK",
                        CfgPin::Done => "DONE",
                        CfgPin::ProgB => "PROG_B",
                        CfgPin::PwrdwnB => "PWRDWN_B",
                        CfgPin::M0 => "M0",
                        CfgPin::M1 => "M1",
                        CfgPin::M2 => "M2",
                        CfgPin::Tck => "TCK",
                        CfgPin::Tms => "TMS",
                        CfgPin::Tdi => "TDI",
                        CfgPin::Tdo => "TDO",
                        CfgPin::HswapEn => "HSWAP_EN",
                        CfgPin::InitB => "INIT_B",
                        CfgPin::RdWrB => "RDWR_B",
                        CfgPin::CsiB => "CSI_B",
                        CfgPin::Din => "DIN",
                        CfgPin::Dout => "DOUT",
                        CfgPin::CfgBvs => "CFGBVS",
                    }.to_string(),
                    BondPin::VccInt => "VCCINT".to_string(),
                    BondPin::VccAux => "VCCAUX".to_string(),
                    BondPin::VccAuxIo(i) => format!("VCCAUX_IO{i}"),
                    BondPin::VccBram => "VCCBRAM".to_string(),
                    BondPin::VccBatt => "VCCBATT".to_string(),
                    BondPin::VccPsInt => "VCCPSINT".to_string(),
                    BondPin::VccPsAux => "VCCPSAUX".to_string(),
                    BondPin::VccPsPll => "VCCPSPLL".to_string(),
                    BondPin::Dxn => "DXN".to_string(),
                    BondPin::Dxp => "DXP".to_string(),
                    BondPin::Rsvd => "RSVD".to_string(),
                    BondPin::RsvdGnd => "RSVD_GND".to_string(),
                    BondPin::Vfs => "VFS".to_string(),
                }.into()))
            ),
        })
    }
}
