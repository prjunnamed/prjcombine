use itertools::Itertools;
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
                BondPin::Io(bank, idx) => write!(f, "IOB_{bank}_{idx}")?,
                BondPin::Gt(bank, gtpin) => {
                    write!(f, "GT{bank}.")?;
                    match gtpin {
                        GtPin::RxP(idx) => write!(f, "RXP{idx}")?,
                        GtPin::RxN(idx) => write!(f, "RXN{idx}")?,
                        GtPin::TxP(idx) => write!(f, "TXP{idx}")?,
                        GtPin::TxN(idx) => write!(f, "TXN{idx}")?,
                        GtPin::ClkP(idx) => write!(f, "CLKP{idx}")?,
                        GtPin::ClkN(idx) => write!(f, "CLKN{idx}")?,
                        GtPin::GndA => write!(f, "GNDA")?,
                        GtPin::AVccAuxTx => write!(f, "AVCCAUXTX")?,
                        GtPin::AVccAuxRx(idx) => write!(f, "AVCCAUXRX{idx}")?,
                        GtPin::AVccAuxMgt => write!(f, "AVCCAUXMGT")?,
                        GtPin::RTerm => write!(f, "RTERM")?,
                        GtPin::MgtVRef => write!(f, "MGTVREF")?,
                        GtPin::VtRx(idx) => write!(f, "VTRX{idx}")?,
                        GtPin::VtTx(idx) => write!(f, "VTTX{idx}")?,
                        GtPin::AVcc => write!(f, "AVCC")?,
                        GtPin::AVccPll => write!(f, "AVCCPLL")?,
                        GtPin::RRef => write!(f, "RREF")?,
                        GtPin::AVttRCal => write!(f, "AVTTRCAL")?,
                        GtPin::RBias => write!(f, "RBIAS")?,
                    }
                }
                BondPin::Gtz(bank, gtpin) => {
                    write!(f, "GTZ{bank}.")?;
                    match gtpin {
                        GtzPin::RxP(idx) => write!(f, "RXP{idx}")?,
                        GtzPin::RxN(idx) => write!(f, "RXN{idx}")?,
                        GtzPin::TxP(idx) => write!(f, "TXP{idx}")?,
                        GtzPin::TxN(idx) => write!(f, "TXN{idx}")?,
                        GtzPin::ClkP(idx) => write!(f, "CLKP{idx}")?,
                        GtzPin::ClkN(idx) => write!(f, "CLKN{idx}")?,
                        GtzPin::AGnd => write!(f, "AGND")?,
                        GtzPin::AVcc => write!(f, "AVCC")?,
                        GtzPin::VccH => write!(f, "VCCH")?,
                        GtzPin::VccL => write!(f, "VCCL")?,
                        GtzPin::ObsClkP => write!(f, "OBSCLKP")?,
                        GtzPin::ObsClkN => write!(f, "OBSCLKN")?,
                        GtzPin::ThermIn => write!(f, "THERM_IN")?,
                        GtzPin::ThermOut => write!(f, "THERM_OUT")?,
                        GtzPin::SenseAGnd => write!(f, "SENSE_AGND")?,
                        GtzPin::SenseGnd => write!(f, "SENSE_GND")?,
                        GtzPin::SenseGndL => write!(f, "SENSE_GNDL")?,
                        GtzPin::SenseAVcc => write!(f, "SENSE_AVCC")?,
                        GtzPin::SenseVcc => write!(f, "SENSE_VCC")?,
                        GtzPin::SenseVccL => write!(f, "SENSE_VCCL")?,
                        GtzPin::SenseVccH => write!(f, "SENSE_VCCH")?,
                    }
                }
                BondPin::GtRegion(region, gtpin) => {
                    write!(f, "GTREG")?;
                    match region {
                        GtRegion::All => (),
                        GtRegion::S => write!(f, "S")?,
                        GtRegion::N => write!(f, "N")?,
                        GtRegion::L => write!(f, "L")?,
                        GtRegion::R => write!(f, "R")?,
                        GtRegion::LS => write!(f, "LS")?,
                        GtRegion::RS => write!(f, "RS")?,
                        GtRegion::LN => write!(f, "LN")?,
                        GtRegion::RN => write!(f, "RN")?,
                        GtRegion::H => write!(f, "H")?,
                        GtRegion::LH => write!(f, "LH")?,
                        GtRegion::RH => write!(f, "RH")?,
                        GtRegion::Num(n) => write!(f, "{n}")?,
                    }
                    write!(f, ".")?;
                    match gtpin {
                        GtRegionPin::AVtt => write!(f, "AVTT")?,
                        GtRegionPin::AGnd => write!(f, "AGND")?,
                        GtRegionPin::AVcc => write!(f, "AVCC")?,
                        GtRegionPin::AVccRx => write!(f, "AVCCRX")?,
                        GtRegionPin::AVccPll => write!(f, "AVCCPLL")?,
                        GtRegionPin::AVttRxC => write!(f, "AVTTRXC")?,
                        GtRegionPin::VccAux => write!(f, "VCCAUX")?,
                    }
                }
                BondPin::Nc => write!(f, "NC")?,
                BondPin::Gnd => write!(f, "GND")?,
                BondPin::VccInt => write!(f, "VCCINT")?,
                BondPin::VccAux => write!(f, "VCCAUX")?,
                BondPin::VccAuxIo(idx) => write!(f, "VCCAUX_IO{idx}")?,
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
                BondPin::Cfg(CfgPin::CsiB) => write!(f, "CSI_B")?,
                BondPin::Cfg(CfgPin::Tck) => write!(f, "TCK")?,
                BondPin::Cfg(CfgPin::Tms) => write!(f, "TMS")?,
                BondPin::Cfg(CfgPin::Tdi) => write!(f, "TDI")?,
                BondPin::Cfg(CfgPin::Tdo) => write!(f, "TDO")?,
                BondPin::Cfg(CfgPin::PwrdwnB) => write!(f, "PWRDWN_B")?,
                BondPin::Cfg(CfgPin::HswapEn) => write!(f, "HSWAP_EN")?,
                BondPin::Cfg(CfgPin::Din) => write!(f, "DIN")?,
                BondPin::Cfg(CfgPin::Dout) => write!(f, "DOUT")?,
                BondPin::Cfg(CfgPin::CfgBvs) => write!(f, "CFGBVS")?,
                BondPin::Dxn => write!(f, "DXN")?,
                BondPin::Dxp => write!(f, "DXP")?,
                BondPin::Rsvd => write!(f, "RSVD")?,
                BondPin::RsvdGnd => write!(f, "RSVDGND")?,
                BondPin::Vfs => write!(f, "VFS")?,
                BondPin::SysMon(bank, pin) => {
                    write!(f, "SYSMON{bank}.")?;
                    match pin {
                        SysMonPin::VP => write!(f, "VP")?,
                        SysMonPin::VN => write!(f, "VN")?,
                        SysMonPin::AVss => write!(f, "AVSS")?,
                        SysMonPin::AVdd => write!(f, "AVDD")?,
                        SysMonPin::VRefP => write!(f, "VREFP")?,
                        SysMonPin::VRefN => write!(f, "VREFN")?,
                    }
                }
                BondPin::VccPsInt => write!(f, "VCC_PS_INT")?,
                BondPin::VccPsAux => write!(f, "VCC_PS_AUX")?,
                BondPin::VccPsPll => write!(f, "VCC_PS_PLL")?,
                BondPin::PsVref(bank, idx) => write!(f, "PS{bank}.VREF{idx}")?,
                BondPin::PsIo(bank, pin) => {
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
                        PsPin::DdrVrP => write!(f, "DDR_VRP")?,
                        PsPin::DdrVrN => write!(f, "DDR_VRN")?,
                        PsPin::DdrCkP => write!(f, "DDR_CKP")?,
                        PsPin::DdrCkN => write!(f, "DDR_CKN")?,
                        PsPin::DdrCke => write!(f, "DDR_CKE")?,
                        PsPin::DdrOdt => write!(f, "DDR_ODT")?,
                        PsPin::DdrDrstB => write!(f, "DDR_DRST_B")?,
                        PsPin::DdrCsB => write!(f, "DDR_CS_B")?,
                        PsPin::DdrRasB => write!(f, "DDR_RAS_B")?,
                        PsPin::DdrCasB => write!(f, "DDR_CAS_B")?,
                        PsPin::DdrWeB => write!(f, "DDR_WE_B")?,
                    }
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
