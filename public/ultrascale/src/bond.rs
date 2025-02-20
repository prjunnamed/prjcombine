use itertools::Itertools;
use prjcombine_interconnect::grid::{DieId, TileIobId};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;

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
    Hpio(u32, TileIobId),
    Hdio(u32, TileIobId),
    HdioLc(u32, TileIobId),
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
    // CSEC new stuff
    Busy,
    Fcs1B,
    OspiDs,
    OspiRstB,
    OspiEccFail,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandedBond<'a> {
    pub bond: &'a Bond,
    pub hpios: BTreeMap<(u32, TileIobId), String>,
    pub hdios: BTreeMap<(u32, TileIobId), String>,
    pub hdiolcs: BTreeMap<(u32, TileIobId), String>,
    pub gts: BTreeMap<(u32, GtPin), String>,
    pub sysmons: BTreeMap<(DieId, SysMonPin), String>,
}

impl Bond {
    pub fn expand(&self) -> ExpandedBond {
        let mut hpios = BTreeMap::new();
        let mut hdios = BTreeMap::new();
        let mut hdiolcs = BTreeMap::new();
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
                BondPin::HdioLc(bank, idx) => {
                    hdiolcs.insert((bank, idx), name.clone());
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
            hdiolcs,
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
                    BondPin::HdioLc(bank, io) => format!("HDIOLC:{bank}:{io}"),
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

fn pad_sort_key(name: &str) -> (usize, &str, u32) {
    let pos = name.find(|x: char| x.is_ascii_digit()).unwrap();
    (pos, &name[..pos], name[pos..].parse().unwrap())
}

impl std::fmt::Display for Bond {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tPINS:")?;
        for (pin, pad) in self.pins.iter().sorted_by_key(|(k, _)| pad_sort_key(k)) {
            write!(f, "\t\t{pin:4}: ")?;
            match pad {
                BondPin::Hpio(bank, idx) => write!(f, "HPIOB_{bank}_{idx}")?,
                BondPin::Hdio(bank, idx) => write!(f, "HDIOB_{bank}_{idx}")?,
                BondPin::HdioLc(bank, idx) => write!(f, "HDIOBLC_{bank}_{idx}")?,
                BondPin::IoVref(bank) => write!(f, "IO_{bank}_VREF")?,
                BondPin::Gt(bank, gtpin) => {
                    write!(f, "GT{bank}.")?;
                    match gtpin {
                        GtPin::RxP(idx) => write!(f, "RXP{idx}")?,
                        GtPin::RxN(idx) => write!(f, "RXN{idx}")?,
                        GtPin::TxP(idx) => write!(f, "TXP{idx}")?,
                        GtPin::TxN(idx) => write!(f, "TXN{idx}")?,
                        GtPin::ClkP(idx) => write!(f, "CLKP{idx}")?,
                        GtPin::ClkN(idx) => write!(f, "CLKN{idx}")?,
                        GtPin::AVcc => write!(f, "AVCC")?,
                        GtPin::RRef => write!(f, "RREF")?,
                        GtPin::AVttRCal => write!(f, "AVTTRCAL")?,
                        GtPin::AVtt => write!(f, "AVTT")?,
                    }
                }
                BondPin::GtRegion(region, gtpin) => {
                    write!(f, "GTREG")?;
                    match region {
                        GtRegion::All => (),
                        GtRegion::L => write!(f, "L")?,
                        GtRegion::R => write!(f, "R")?,
                        GtRegion::LS => write!(f, "LS")?,
                        GtRegion::RS => write!(f, "RS")?,
                        GtRegion::LN => write!(f, "LN")?,
                        GtRegion::RN => write!(f, "RN")?,
                        GtRegion::LLC => write!(f, "LLC")?,
                        GtRegion::RLC => write!(f, "RLC")?,
                        GtRegion::LC => write!(f, "LC")?,
                        GtRegion::RC => write!(f, "RC")?,
                        GtRegion::LUC => write!(f, "LUC")?,
                        GtRegion::RUC => write!(f, "RUC")?,
                    }
                    write!(f, ".")?;
                    match gtpin {
                        GtRegionPin::AVtt => write!(f, "AVTT")?,
                        GtRegionPin::AVcc => write!(f, "AVCC")?,
                        GtRegionPin::VccAux => write!(f, "VCCAUX")?,
                        GtRegionPin::VccInt => write!(f, "VCCINT")?,
                    }
                }
                BondPin::Nc => write!(f, "NC")?,
                BondPin::Gnd => write!(f, "GND")?,
                BondPin::VccInt => write!(f, "VCCINT")?,
                BondPin::VccAux => write!(f, "VCCAUX")?,
                BondPin::VccBram => write!(f, "VCCBRAM")?,
                BondPin::VccO(bank) => write!(f, "VCCO{bank}")?,
                BondPin::VccBatt => write!(f, "VCC_BATT")?,
                BondPin::Cfg(CfgPin::Cclk) => write!(f, "CCLK")?,
                BondPin::Cfg(CfgPin::Done) => write!(f, "DONE")?,
                BondPin::Cfg(CfgPin::M0) => write!(f, "M0")?,
                BondPin::Cfg(CfgPin::M1) => write!(f, "M1")?,
                BondPin::Cfg(CfgPin::M2) => write!(f, "M2")?,
                BondPin::Cfg(CfgPin::ProgB) => write!(f, "PROG_B")?,
                BondPin::Cfg(CfgPin::InitB) => write!(f, "INIT_B")?,
                BondPin::Cfg(CfgPin::RdWrB) => write!(f, "RDWR_B")?,
                BondPin::Cfg(CfgPin::Tck) => write!(f, "TCK")?,
                BondPin::Cfg(CfgPin::Tms) => write!(f, "TMS")?,
                BondPin::Cfg(CfgPin::Tdi) => write!(f, "TDI")?,
                BondPin::Cfg(CfgPin::Tdo) => write!(f, "TDO")?,
                BondPin::Cfg(CfgPin::HswapEn) => write!(f, "HSWAP_EN")?,
                BondPin::Cfg(CfgPin::Data(idx)) => write!(f, "DATA{idx}")?,
                BondPin::Cfg(CfgPin::CfgBvs) => write!(f, "CFGBVS")?,
                BondPin::Cfg(CfgPin::PorOverride) => write!(f, "POR_OVERRIDE")?,
                BondPin::Dxn => write!(f, "DXN")?,
                BondPin::Dxp => write!(f, "DXP")?,
                BondPin::Rsvd => write!(f, "RSVD")?,
                BondPin::RsvdGnd => write!(f, "RSVDGND")?,
                BondPin::SysMon(bank, pin) => {
                    write!(f, "SYSMON{bank}.")?;
                    match pin {
                        SysMonPin::VP => write!(f, "VP")?,
                        SysMonPin::VN => write!(f, "VN")?,
                    }
                }
                BondPin::VccPsAux => write!(f, "VCC_PS_AUX")?,
                BondPin::VccPsPll => write!(f, "VCC_PS_PLL")?,
                BondPin::IoPs(bank, pin) => {
                    write!(f, "PS{bank}.")?;
                    match pin {
                        PsPin::Mio(i) => write!(f, "MIO{i}")?,
                        PsPin::Clk => write!(f, "CLK")?,
                        PsPin::PorB => write!(f, "POR_B")?,
                        PsPin::SrstB => write!(f, "SRST_B")?,
                        PsPin::DdrDq(i) => write!(f, "DDR_DQ{i}")?,
                        PsPin::DdrDm(i) => write!(f, "DDR_DM{i}")?,
                        PsPin::DdrDqsP(i) => write!(f, "DDR_DQS_P{i}")?,
                        PsPin::DdrDqsN(i) => write!(f, "DDR_DQS_N{i}")?,
                        PsPin::DdrA(i) => write!(f, "DDR_A{i}")?,
                        PsPin::DdrBa(i) => write!(f, "DDR_BA{i}")?,
                        PsPin::DdrCkP(idx) => write!(f, "DDR_CKP{idx}")?,
                        PsPin::DdrCkN(idx) => write!(f, "DDR_CKN{idx}")?,
                        PsPin::DdrCke(idx) => write!(f, "DDR_CKE{idx}")?,
                        PsPin::DdrOdt(idx) => write!(f, "DDR_ODT{idx}")?,
                        PsPin::DdrCsB(idx) => write!(f, "DDR_CS_B{idx}")?,
                        PsPin::DdrDrstB => write!(f, "DDR_DRST_B")?,
                        PsPin::DdrActN => write!(f, "DDR_ACT_N")?,
                        PsPin::DdrAlertN => write!(f, "DDR_ALERT_N")?,
                        PsPin::DdrBg(idx) => write!(f, "DDR_BG{idx}")?,
                        PsPin::DdrParity => write!(f, "DDR_PARITY")?,
                        PsPin::DdrZq => write!(f, "DDR_ZQ")?,
                        PsPin::ErrorOut => write!(f, "ERROR_OUT")?,
                        PsPin::ErrorStatus => write!(f, "ERROR_STATUS")?,
                        PsPin::Done => write!(f, "DONE")?,
                        PsPin::InitB => write!(f, "INIT_B")?,
                        PsPin::ProgB => write!(f, "PROG_B")?,
                        PsPin::JtagTck => write!(f, "JTAG_TCK")?,
                        PsPin::JtagTdi => write!(f, "JTAG_TDI")?,
                        PsPin::JtagTdo => write!(f, "JTAG_TDO")?,
                        PsPin::JtagTms => write!(f, "JTAG_TMS")?,
                        PsPin::Mode(i) => write!(f, "MODE{i}")?,
                        PsPin::PadI => write!(f, "PAD_I")?,
                        PsPin::PadO => write!(f, "PAD_O")?,
                    }
                }
                BondPin::SysMonVRefP => write!(f, "SYSMON_VREFP")?,
                BondPin::SysMonVRefN => write!(f, "SYSMON_VREFN")?,
                BondPin::SysMonGnd => write!(f, "SYSMON_GND")?,
                BondPin::SysMonVcc => write!(f, "SYSMON_VCC")?,
                BondPin::PsSysMonGnd => write!(f, "PS_SYSMON_GND")?,
                BondPin::PsSysMonVcc => write!(f, "PS_SYSMON_VCC")?,
                BondPin::VccAuxHpio => write!(f, "VCCAUX_HPIO")?,
                BondPin::VccAuxHdio => write!(f, "VCCAUX_HDIO")?,
                BondPin::VccAuxIo => write!(f, "VCCAUX_IO")?,
                BondPin::VccIntIo => write!(f, "VCCINT_IO")?,
                BondPin::VccPsIntLp => write!(f, "VCC_PS_INT_LP")?,
                BondPin::VccPsIntFp => write!(f, "VCC_PS_INT_FP")?,
                BondPin::VccPsIntFpDdr => write!(f, "VCC_PS_INT_FP_DDR")?,
                BondPin::VccPsBatt => write!(f, "VCC_PS_BATT")?,
                BondPin::VccPsDdrPll => write!(f, "VCC_PS_DDR_PLL")?,
                BondPin::VccIntVcu => write!(f, "VCCINT_VCU")?,
                BondPin::GndSense => write!(f, "GND_SENSE")?,
                BondPin::VccIntSense => write!(f, "VCCINT_SENSE")?,
                BondPin::VccIntAms => write!(f, "VCCINT_AMS")?,
                BondPin::VccSdfec => write!(f, "VCC_SDFEC")?,
                BondPin::RfDacGnd => write!(f, "RFDAC_GND")?,
                BondPin::RfDacSubGnd => write!(f, "RFDAC_AGND")?,
                BondPin::RfDacAVcc => write!(f, "RFDAC_AVCC")?,
                BondPin::RfDacAVccAux => write!(f, "RFDAC_AVCCAUX")?,
                BondPin::RfDacAVtt => write!(f, "RFDAC_AVTT")?,
                BondPin::RfAdcGnd => write!(f, "RFADC_GND")?,
                BondPin::RfAdcSubGnd => write!(f, "RFADC_SUBGND")?,
                BondPin::RfAdcAVcc => write!(f, "RFADC_AVCC")?,
                BondPin::RfAdcAVccAux => write!(f, "RFADC_AVCCAUX")?,
                BondPin::Hbm(bank, pin) => {
                    write!(f, "HBM{bank}.")?;
                    match pin {
                        HbmPin::Vcc => write!(f, "VCC")?,
                        HbmPin::VccIo => write!(f, "VCCIO")?,
                        HbmPin::VccAux => write!(f, "VCCAUX")?,
                        HbmPin::Rsvd => write!(f, "RSVD")?,
                        HbmPin::RsvdGnd => write!(f, "RSVD_GND")?,
                    }
                }
                BondPin::RfDac(bank, pin) => {
                    write!(f, "RFDAC{bank}.")?;
                    match pin {
                        RfDacPin::VOutP(idx) => write!(f, "VOUT{idx}P")?,
                        RfDacPin::VOutN(idx) => write!(f, "VOUT{idx}N")?,
                        RfDacPin::ClkP => write!(f, "CLKP")?,
                        RfDacPin::ClkN => write!(f, "CLKN")?,
                        RfDacPin::RExt => write!(f, "REXT")?,
                        RfDacPin::SysRefP => write!(f, "SYSREFP")?,
                        RfDacPin::SysRefN => write!(f, "SYSREFN")?,
                    }
                }
                BondPin::RfAdc(bank, pin) => {
                    write!(f, "RFADC{bank}.")?;
                    match pin {
                        RfAdcPin::VInP(idx) => write!(f, "VIN{idx}_P")?,
                        RfAdcPin::VInN(idx) => write!(f, "VIN{idx}_N")?,
                        RfAdcPin::VInPairP(idx) => write!(f, "VIN_PAIR{idx}_P")?,
                        RfAdcPin::VInPairN(idx) => write!(f, "VIN_PAIR{idx}_N")?,
                        RfAdcPin::ClkP => write!(f, "CLKP")?,
                        RfAdcPin::ClkN => write!(f, "CLKN")?,
                        RfAdcPin::VCm(idx) => write!(f, "VCM{idx}")?,
                        RfAdcPin::RExt => write!(f, "REXT")?,
                        RfAdcPin::PllTestOutP => write!(f, "PLL_TEST_OUT_P")?,
                        RfAdcPin::PllTestOutN => write!(f, "PLL_TEST_OUT_N")?,
                    }
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
