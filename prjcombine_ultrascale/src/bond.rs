use prjcombine_int::grid::DieId;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;

use crate::grid::{HdioIobId, HpioIobId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CfgPin {
    Tck,
    Tdi,
    Tdo,
    Tms,
    ProgB,
    Done,
    M0,
    M1,
    M2,
    Cclk,
    HswapEn,
    InitB,
    RdWrB,
    Data(u8),
    CfgBvs,
    PorOverride,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum GtPin {
    RxP(u8),
    RxN(u8),
    TxP(u8),
    TxN(u8),
    ClkP(u8),
    ClkN(u8),
    AVcc,
    AVtt,
    RRef,
    AVttRCal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum GtRegion {
    All,
    L,
    R,
    LS,
    RS,
    LLC,
    RLC,
    LC,
    RC,
    LUC,
    RUC,
    LN,
    RN,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum GtRegionPin {
    AVtt,
    AVcc,
    VccAux,
    VccInt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SysMonPin {
    VP,
    VN,
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
    DdrCkP(u32),
    DdrCkN(u32),
    DdrCke(u32),
    DdrOdt(u32),
    DdrDrstB,
    DdrCsB(u32),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum HbmPin {
    Vcc,
    VccIo,
    VccAux,
    Rsvd,
    RsvdGnd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum RfDacPin {
    VOutP(u8),
    VOutN(u8),
    ClkP,
    ClkN,
    RExt,
    SysRefP,
    SysRefN,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum RfAdcPin {
    VInP(u8),
    VInN(u8),
    VInPairP(u8),
    VInPairN(u8),
    ClkP,
    ClkN,
    VCm(u8),
    RExt,
    PllTestOutP,
    PllTestOutN,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum BondPin {
    // bank, bel idx
    Hpio(u32, HpioIobId),
    Hdio(u32, HdioIobId),
    IoVref(u32),
    // bank, type
    Gt(u32, GtPin),
    GtRegion(GtRegion, GtRegionPin),
    // bank, type
    SysMon(DieId, SysMonPin),
    SysMonVRefP,
    SysMonVRefN,
    SysMonGnd,
    SysMonVcc,
    PsSysMonGnd,
    PsSysMonVcc,
    Cfg(CfgPin),
    Gnd,
    VccInt,
    VccAux,
    VccBram,
    VccAuxHpio,
    VccAuxHdio,
    VccAuxIo,
    VccIntIo,
    VccO(u32),
    VccBatt,
    Nc,
    Rsvd,
    RsvdGnd,
    Dxp,
    Dxn,
    VccPsAux,
    VccPsPll,
    VccPsIntLp,
    VccPsIntFp,
    VccPsIntFpDdr,
    VccPsBatt,
    VccPsDdrPll,
    VccIntVcu,
    // xqrku060 special
    GndSense,
    VccIntSense,
    IoPs(u32, PsPin),
    Hbm(u32, HbmPin),
    // RFSoC
    VccIntAms,
    VccSdfec,
    RfDacGnd,
    RfDacSubGnd,
    RfDacAVcc,
    RfDacAVccAux,
    RfDacAVtt,
    RfAdcGnd,
    RfAdcSubGnd,
    RfAdcAVcc,
    RfAdcAVccAux,
    RfDac(u32, RfDacPin),
    RfAdc(u32, RfAdcPin),
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    // ×32 total, but 0-3 are dedicated; high 16 bits are also low 16 bits of Addr
    Data(u8),
    Addr(u8), // ×29 total, but 0-15 are represented as Data(16-31)
    CsiB,     // doubles as ADV_B
    Dout,     // doubles as CSO_B
    EmCclk,
    PudcB,
    Rs(u8), // ×2
    FweB,   // doubles as FCS2_B
    FoeB,
    I2cSclk,
    I2cSda,   // on Ultrascale+, doubles as PERSTN1
    SmbAlert, // Ultrascale+ only
    PerstN0,
    PerstN1, // Ultrascale only (shared with I2C_SDA on Ultrascale+)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandedBond<'a> {
    pub bond: &'a Bond,
    pub hpios: BTreeMap<(u32, HpioIobId), String>,
    pub hdios: BTreeMap<(u32, HdioIobId), String>,
    pub gts: BTreeMap<(u32, GtPin), String>,
    pub sysmons: BTreeMap<(DieId, SysMonPin), String>,
}

impl Bond {
    pub fn expand(&self) -> ExpandedBond {
        let mut hpios = BTreeMap::new();
        let mut hdios = BTreeMap::new();
        let mut gts = BTreeMap::new();
        let mut sysmons = BTreeMap::new();
        for (name, pad) in &self.pins {
            match *pad {
                BondPin::Hpio(bank, idx) => {
                    hpios.insert((bank, idx), name.clone());
                }
                BondPin::Hdio(bank, idx) => {
                    hdios.insert((bank, idx), name.clone());
                }
                BondPin::Gt(bank, gtpin) => {
                    gts.insert((bank, gtpin), name.clone());
                }
                BondPin::SysMon(bank, smpin) => {
                    sysmons.insert((bank, smpin), name.clone());
                }
                _ => (),
            }
        }
        ExpandedBond {
            bond: self,
            hpios,
            hdios,
            gts,
            sysmons,
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "pins": serde_json::Map::from_iter(
                self.pins.iter().map(|(pin, pad)| (pin.clone(), match pad {
                    BondPin::Hpio(bank, io) => format!("HPIO:{bank}:{io}"),
                    BondPin::Hdio(bank, io) => format!("HDIO:{bank}:{io}"),
                    BondPin::Gt(bank, pad) => match pad {
                        GtPin::RxP(i) => format!("GT{bank}_RXP{i}"),
                        GtPin::RxN(i) => format!("GT{bank}_RXN{i}"),
                        GtPin::TxP(i) => format!("GT{bank}_TXP{i}"),
                        GtPin::TxN(i) => format!("GT{bank}_TXN{i}"),
                        GtPin::ClkP(i) => format!("GT{bank}_CLKP{i}"),
                        GtPin::ClkN(i) => format!("GT{bank}_CLKN{i}"),
                        GtPin::AVcc => format!("GT{bank}_AVCC"),
                        GtPin::AVtt => format!("GT{bank}_AVTT"),
                        GtPin::AVttRCal => format!("GT{bank}_AVTTRCAL"),
                        GtPin::RRef => format!("GT{bank}_RREF"),
                    },
                    BondPin::GtRegion(reg, pad) => format!("GT_{reg}_{pad}", reg = match reg {
                        GtRegion::All => "ALL".to_string(),
                        GtRegion::L => "L".to_string(),
                        GtRegion::R => "R".to_string(),
                        GtRegion::LS => "LS".to_string(),
                        GtRegion::RS => "RS".to_string(),
                        GtRegion::LN => "LN".to_string(),
                        GtRegion::RN => "RN".to_string(),
                        GtRegion::LC => "LC".to_string(),
                        GtRegion::RC => "RC".to_string(),
                        GtRegion::LLC => "LLC".to_string(),
                        GtRegion::RLC => "RLC".to_string(),
                        GtRegion::LUC => "LUC".to_string(),
                        GtRegion::RUC => "RUC".to_string(),
                    }, pad = match pad {
                        GtRegionPin::AVtt => "AVTT",
                        GtRegionPin::AVcc => "AVCC",
                        GtRegionPin::VccAux => "VCCAUX",
                        GtRegionPin::VccInt => "VCCINT",
                    }),
                    BondPin::SysMon(die, pad) => match pad {
                        SysMonPin::VP => format!("SYSMON{die}_VP"),
                        SysMonPin::VN => format!("SYSMON{die}_VN"),
                    },
                    BondPin::SysMonGnd => "SYSMON_GND".to_string(),
                    BondPin::SysMonVcc => "SYSMON_VCC".to_string(),
                    BondPin::SysMonVRefP => "SYSMON_VREFP".to_string(),
                    BondPin::SysMonVRefN => "SYSMON_VREFN".to_string(),
                    BondPin::IoPs(bank, pad) => match pad {
                        PsPin::Mio(i) => format!("PS{bank}_MIO{i}"),
                        PsPin::Mode(i) => format!("PS{bank}_MODE{i}"),
                        PsPin::Clk => format!("PS{bank}_CLK"),
                        PsPin::PorB => format!("PS{bank}_POR_B"),
                        PsPin::SrstB => format!("PS{bank}_SRST_B"),
                        PsPin::ErrorOut => format!("PS{bank}_ERROR_OUT"),
                        PsPin::ErrorStatus => format!("PS{bank}_ERROR_STATUS"),
                        PsPin::Done => format!("PS{bank}_DONE"),
                        PsPin::InitB => format!("PS{bank}_INIT_B"),
                        PsPin::ProgB => format!("PS{bank}_PROG_B"),
                        PsPin::JtagTck => format!("PS{bank}_JTAG_TCK"),
                        PsPin::JtagTms => format!("PS{bank}_JTAG_TMS"),
                        PsPin::JtagTdi => format!("PS{bank}_JTAG_TDI"),
                        PsPin::JtagTdo => format!("PS{bank}_JTAG_TDO"),
                        PsPin::PadI => format!("PS{bank}_PADI"),
                        PsPin::PadO => format!("PS{bank}_PADO"),
                        PsPin::DdrDq(i) => format!("PS{bank}_DDR_DQ{i}"),
                        PsPin::DdrDm(i) => format!("PS{bank}_DDR_DM{i}"),
                        PsPin::DdrDqsP(i) => format!("PS{bank}_DDR_DQS{i}P"),
                        PsPin::DdrDqsN(i) => format!("PS{bank}_DDR_DQS{i}N"),
                        PsPin::DdrA(i) => format!("PS{bank}_DDR_A{i}"),
                        PsPin::DdrBa(i) => format!("PS{bank}_DDR_BA{i}"),
                        PsPin::DdrBg(i) => format!("PS{bank}_DDR_BG{i}"),
                        PsPin::DdrCkP(i) => format!("PS{bank}_DDR_CKP{i}"),
                        PsPin::DdrCkN(i) => format!("PS{bank}_DDR_CKN{i}"),
                        PsPin::DdrCke(i) => format!("PS{bank}_DDR_CKE{i}"),
                        PsPin::DdrOdt(i) => format!("PS{bank}_DDR_ODT{i}"),
                        PsPin::DdrDrstB => format!("PS{bank}_DDR_DRST_B"),
                        PsPin::DdrCsB(i) => format!("PS{bank}_DDR_CS_B{i}"),
                        PsPin::DdrActN => format!("PS{bank}_DDR_ACT_N"),
                        PsPin::DdrAlertN => format!("PS{bank}_DDR_ALERT_N"),
                        PsPin::DdrParity => format!("PS{bank}_DDR_PARITY"),
                        PsPin::DdrZq => format!("PS{bank}_DDR_ZQ"),
                    },
                    BondPin::Hbm(bank, pad) => match pad {
                        HbmPin::Vcc => format!("HBM{bank}_VCC"),
                        HbmPin::VccIo => format!("HBM{bank}_VCCIO"),
                        HbmPin::VccAux => format!("HBM{bank}_VCCAUX"),
                        HbmPin::Rsvd => format!("HBM{bank}_RSVD"),
                        HbmPin::RsvdGnd => format!("HBM{bank}_RSVDGND"),
                    },
                    BondPin::RfAdc(bank, pad) => match pad {
                        RfAdcPin::VInP(i) => format!("RFADC{bank}_VINP{i}"),
                        RfAdcPin::VInN(i) => format!("RFADC{bank}_VINN{i}"),
                        RfAdcPin::VInPairP(i) => format!("RFADC{bank}_VINPAIRP{i}"),
                        RfAdcPin::VInPairN(i) => format!("RFADC{bank}_VINPAIRN{i}"),
                        RfAdcPin::ClkP => format!("RFADC{bank}_CLKP"),
                        RfAdcPin::ClkN => format!("RFADC{bank}_CLKN"),
                        RfAdcPin::VCm(i) => format!("RFADC{bank}_VCM{i}"),
                        RfAdcPin::RExt => format!("RFADC{bank}_REXT"),
                        RfAdcPin::PllTestOutP => format!("RFADC{bank}_PLLTESTOUTP"),
                        RfAdcPin::PllTestOutN => format!("RFADC{bank}_PLLTESTOUTN"),
                    },
                    BondPin::RfDac(bank, pad) => match pad {
                        RfDacPin::VOutP(i) => format!("RFDAC{bank}_VOUTP{i}"),
                        RfDacPin::VOutN(i) => format!("RFDAC{bank}_VOUTN{i}"),
                        RfDacPin::ClkP => format!("RFDAC{bank}_CLKP"),
                        RfDacPin::ClkN => format!("RFDAC{bank}_CLKN"),
                        RfDacPin::RExt => format!("RFDAC{bank}_REXT"),
                        RfDacPin::SysRefP => format!("RFDAC{bank}_SYSREFP"),
                        RfDacPin::SysRefN => format!("RFDAC{bank}_SYSREFN"),
                    },
                    BondPin::PsSysMonGnd => "PS_SYSMON_GND".to_string(),
                    BondPin::PsSysMonVcc => "PS_SYSMON_VCC".to_string(),
                    BondPin::Gnd => "GND".to_string(),
                    BondPin::VccO(bank) => format!("VCCO{bank}"),
                    BondPin::Nc => "NC".to_string(),
                    BondPin::Cfg(cfg_pin) => match cfg_pin {
                        CfgPin::Data(i) => format!("D{i}"),
                        CfgPin::Cclk => "CCLK".to_string(),
                        CfgPin::Done => "DONE".to_string(),
                        CfgPin::ProgB => "PROG_B".to_string(),
                        CfgPin::M0 => "M0".to_string(),
                        CfgPin::M1 => "M1".to_string(),
                        CfgPin::M2 => "M2".to_string(),
                        CfgPin::Tck => "TCK".to_string(),
                        CfgPin::Tms => "TMS".to_string(),
                        CfgPin::Tdi => "TDI".to_string(),
                        CfgPin::Tdo => "TDO".to_string(),
                        CfgPin::HswapEn => "HSWAP_EN".to_string(),
                        CfgPin::InitB => "INIT_B".to_string(),
                        CfgPin::RdWrB => "RDWR_B".to_string(),
                        CfgPin::CfgBvs => "CFGBVS".to_string(),
                        CfgPin::PorOverride => "POR_OVERRIDE".to_string(),
                    },
                    BondPin::IoVref(bank) => format!("IO_VREF{bank}"),
                    BondPin::VccInt => "VCCINT".to_string(),
                    BondPin::VccIntIo => "VCCINT_IO".to_string(),
                    BondPin::VccIntVcu => "VCCINT_VCU".to_string(),
                    BondPin::VccIntAms => "VCCINT_AMS".to_string(),
                    BondPin::VccAux => "VCCAUX".to_string(),
                    BondPin::VccAuxIo => "VCCAUX_IO".to_string(),
                    BondPin::VccAuxHpio => "VCCAUX_HPIO".to_string(),
                    BondPin::VccAuxHdio => "VCCAUX_HDIO".to_string(),
                    BondPin::VccBram => "VCCBRAM".to_string(),
                    BondPin::VccSdfec => "VCCSDFEC".to_string(),
                    BondPin::VccBatt => "VCCBATT".to_string(),
                    BondPin::VccPsIntFp => "VCCPSINT_FP".to_string(),
                    BondPin::VccPsIntFpDdr => "VCCPSINT_FP_DDR".to_string(),
                    BondPin::VccPsIntLp => "VCCPSINT_LP".to_string(),
                    BondPin::VccPsAux => "VCCPSAUX".to_string(),
                    BondPin::VccPsPll => "VCCPSPLL".to_string(),
                    BondPin::VccPsDdrPll => "VCCPSDDRPLL".to_string(),
                    BondPin::VccPsBatt => "VCCPSBATT".to_string(),
                    BondPin::Dxn => "DXN".to_string(),
                    BondPin::Dxp => "DXP".to_string(),
                    BondPin::Rsvd => "RSVD".to_string(),
                    BondPin::RsvdGnd => "RSVD_GND".to_string(),
                    BondPin::GndSense => "GND_SENSE".to_string(),
                    BondPin::VccIntSense => "VCCINT_SENSE".to_string(),
                    BondPin::RfAdcAVcc => "RFADC_AVCC".to_string(),
                    BondPin::RfAdcAVccAux => "RFADC_AVCCAUX".to_string(),
                    BondPin::RfAdcGnd => "RFADC_GND".to_string(),
                    BondPin::RfAdcSubGnd => "RFADC_SUBGND".to_string(),
                    BondPin::RfDacAVcc => "RFDAC_AVCC".to_string(),
                    BondPin::RfDacAVccAux => "RFDAC_AVCCAUX".to_string(),
                    BondPin::RfDacGnd => "RFDAC_GND".to_string(),
                    BondPin::RfDacSubGnd => "RFDAC_SUBGND".to_string(),
                    BondPin::RfDacAVtt => "RFDAC_AVTT".to_string(),
                }.into()))
            ),
        })
    }
}
