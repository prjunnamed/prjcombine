use itertools::Itertools;
use jzon::JsonValue;
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

impl std::fmt::Display for CfgPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CfgPin::Cclk => write!(f, "CCLK"),
            CfgPin::Done => write!(f, "DONE"),
            CfgPin::M0 => write!(f, "M0"),
            CfgPin::M1 => write!(f, "M1"),
            CfgPin::M2 => write!(f, "M2"),
            CfgPin::ProgB => write!(f, "PROG_B"),
            CfgPin::InitB => write!(f, "INIT_B"),
            CfgPin::RdWrB => write!(f, "RDWR_B"),
            CfgPin::CsiB => write!(f, "CSI_B"),
            CfgPin::Tck => write!(f, "TCK"),
            CfgPin::Tms => write!(f, "TMS"),
            CfgPin::Tdi => write!(f, "TDI"),
            CfgPin::Tdo => write!(f, "TDO"),
            CfgPin::PwrdwnB => write!(f, "PWRDWN_B"),
            CfgPin::HswapEn => write!(f, "HSWAP_EN"),
            CfgPin::Din => write!(f, "DIN"),
            CfgPin::Dout => write!(f, "DOUT"),
            CfgPin::CfgBvs => write!(f, "CFGBVS"),
        }
    }
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

impl std::fmt::Display for GtPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GtPin::RxP(idx) => write!(f, "RXP{idx}"),
            GtPin::RxN(idx) => write!(f, "RXN{idx}"),
            GtPin::TxP(idx) => write!(f, "TXP{idx}"),
            GtPin::TxN(idx) => write!(f, "TXN{idx}"),
            GtPin::ClkP(idx) => write!(f, "CLKP{idx}"),
            GtPin::ClkN(idx) => write!(f, "CLKN{idx}"),
            GtPin::GndA => write!(f, "GNDA"),
            GtPin::AVccAuxTx => write!(f, "AVCCAUXTX"),
            GtPin::AVccAuxRx(idx) => write!(f, "AVCCAUXRX{idx}"),
            GtPin::AVccAuxMgt => write!(f, "AVCCAUXMGT"),
            GtPin::RTerm => write!(f, "RTERM"),
            GtPin::MgtVRef => write!(f, "MGTVREF"),
            GtPin::VtRx(idx) => write!(f, "VTRX{idx}"),
            GtPin::VtTx(idx) => write!(f, "VTTX{idx}"),
            GtPin::AVcc => write!(f, "AVCC"),
            GtPin::AVccPll => write!(f, "AVCCPLL"),
            GtPin::RRef => write!(f, "RREF"),
            GtPin::AVttRCal => write!(f, "AVTTRCAL"),
            GtPin::RBias => write!(f, "RBIAS"),
        }
    }
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

impl std::fmt::Display for GtRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GtRegion::All => write!(f, "ALL"),
            GtRegion::S => write!(f, "S"),
            GtRegion::N => write!(f, "N"),
            GtRegion::L => write!(f, "L"),
            GtRegion::R => write!(f, "R"),
            GtRegion::LS => write!(f, "LS"),
            GtRegion::RS => write!(f, "RS"),
            GtRegion::LN => write!(f, "LN"),
            GtRegion::RN => write!(f, "RN"),
            GtRegion::H => write!(f, "H"),
            GtRegion::LH => write!(f, "LH"),
            GtRegion::RH => write!(f, "RH"),
            GtRegion::Num(n) => write!(f, "{n}"),
        }
    }
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

impl std::fmt::Display for GtRegionPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GtRegionPin::AVtt => write!(f, "AVTT"),
            GtRegionPin::AGnd => write!(f, "AGND"),
            GtRegionPin::AVcc => write!(f, "AVCC"),
            GtRegionPin::AVccRx => write!(f, "AVCCRX"),
            GtRegionPin::AVccPll => write!(f, "AVCCPLL"),
            GtRegionPin::AVttRxC => write!(f, "AVTTRXC"),
            GtRegionPin::VccAux => write!(f, "VCCAUX"),
        }
    }
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

impl std::fmt::Display for SysMonPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SysMonPin::VP => write!(f, "VP"),
            SysMonPin::VN => write!(f, "VN"),
            SysMonPin::AVss => write!(f, "AVSS"),
            SysMonPin::AVdd => write!(f, "AVDD"),
            SysMonPin::VRefP => write!(f, "VREFP"),
            SysMonPin::VRefN => write!(f, "VREFN"),
        }
    }
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

impl std::fmt::Display for GtzPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GtzPin::RxP(idx) => write!(f, "RXP{idx}"),
            GtzPin::RxN(idx) => write!(f, "RXN{idx}"),
            GtzPin::TxP(idx) => write!(f, "TXP{idx}"),
            GtzPin::TxN(idx) => write!(f, "TXN{idx}"),
            GtzPin::ClkP(idx) => write!(f, "CLKP{idx}"),
            GtzPin::ClkN(idx) => write!(f, "CLKN{idx}"),
            GtzPin::AGnd => write!(f, "AGND"),
            GtzPin::AVcc => write!(f, "AVCC"),
            GtzPin::VccH => write!(f, "VCCH"),
            GtzPin::VccL => write!(f, "VCCL"),
            GtzPin::ObsClkP => write!(f, "OBSCLKP"),
            GtzPin::ObsClkN => write!(f, "OBSCLKN"),
            GtzPin::ThermIn => write!(f, "THERM_IN"),
            GtzPin::ThermOut => write!(f, "THERM_OUT"),
            GtzPin::SenseAGnd => write!(f, "SENSE_AGND"),
            GtzPin::SenseGnd => write!(f, "SENSE_GND"),
            GtzPin::SenseGndL => write!(f, "SENSE_GNDL"),
            GtzPin::SenseAVcc => write!(f, "SENSE_AVCC"),
            GtzPin::SenseVcc => write!(f, "SENSE_VCC"),
            GtzPin::SenseVccL => write!(f, "SENSE_VCCL"),
            GtzPin::SenseVccH => write!(f, "SENSE_VCCH"),
        }
    }
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

impl std::fmt::Display for PsPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PsPin::Mio(i) => write!(f, "MIO{i}"),
            PsPin::Clk => write!(f, "CLK"),
            PsPin::PorB => write!(f, "POR_B"),
            PsPin::SrstB => write!(f, "SRST_B"),
            PsPin::DdrDq(i) => write!(f, "DDR_DQ{i}"),
            PsPin::DdrDm(i) => write!(f, "DDR_DM{i}"),
            PsPin::DdrDqsP(i) => write!(f, "DDR_DQS_P{i}"),
            PsPin::DdrDqsN(i) => write!(f, "DDR_DQS_N{i}"),
            PsPin::DdrA(i) => write!(f, "DDR_A{i}"),
            PsPin::DdrBa(i) => write!(f, "DDR_BA{i}"),
            PsPin::DdrVrP => write!(f, "DDR_VRP"),
            PsPin::DdrVrN => write!(f, "DDR_VRN"),
            PsPin::DdrCkP => write!(f, "DDR_CKP"),
            PsPin::DdrCkN => write!(f, "DDR_CKN"),
            PsPin::DdrCke => write!(f, "DDR_CKE"),
            PsPin::DdrOdt => write!(f, "DDR_ODT"),
            PsPin::DdrDrstB => write!(f, "DDR_DRST_B"),
            PsPin::DdrCsB => write!(f, "DDR_CS_B"),
            PsPin::DdrRasB => write!(f, "DDR_RAS_B"),
            PsPin::DdrCasB => write!(f, "DDR_CAS_B"),
            PsPin::DdrWeB => write!(f, "DDR_WE_B"),
        }
    }
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

impl std::fmt::Display for BondPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPin::Io(bank, idx) => write!(f, "IOB_{bank}_{idx}"),
            BondPin::Gt(bank, gtpin) => write!(f, "GT{bank}_{gtpin}"),
            BondPin::Gtz(bank, gtpin) => write!(f, "GTZ{bank}_{gtpin}"),
            BondPin::GtRegion(region, gtpin) => write!(f, "GTREG_{region}_{gtpin}"),
            BondPin::Nc => write!(f, "NC"),
            BondPin::Gnd => write!(f, "GND"),
            BondPin::VccInt => write!(f, "VCCINT"),
            BondPin::VccAux => write!(f, "VCCAUX"),
            BondPin::VccAuxIo(idx) => write!(f, "VCCAUX_IO{idx}"),
            BondPin::VccBram => write!(f, "VCCBRAM"),
            BondPin::VccO(bank) => write!(f, "VCCO{bank}"),
            BondPin::VccBatt => write!(f, "VCC_BATT"),
            BondPin::Cfg(pin) => write!(f, "{pin}"),
            BondPin::Dxn => write!(f, "DXN"),
            BondPin::Dxp => write!(f, "DXP"),
            BondPin::Rsvd => write!(f, "RSVD"),
            BondPin::RsvdGnd => write!(f, "RSVDGND"),
            BondPin::Vfs => write!(f, "VFS"),
            BondPin::SysMon(bank, pin) => write!(f, "SYSMON{bank}_{pin}"),
            BondPin::VccPsInt => write!(f, "VCC_PS_INT"),
            BondPin::VccPsAux => write!(f, "VCC_PS_AUX"),
            BondPin::VccPsPll => write!(f, "VCC_PS_PLL"),
            BondPin::PsVref(bank, idx) => write!(f, "PS{bank}.VREF{idx}"),
            BondPin::PsIo(bank, pin) => write!(f, "PS{bank}_{pin}"),
        }
    }
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

impl std::fmt::Display for SharedCfgPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SharedCfgPin::Data(idx) => write!(f, "D{idx}"),
            SharedCfgPin::Addr(idx) => write!(f, "A{idx}"),
            SharedCfgPin::Rs(idx) => write!(f, "RS{idx}"),
            SharedCfgPin::CsoB => write!(f, "CSO_B"),
            SharedCfgPin::FweB => write!(f, "FWE_B"),
            SharedCfgPin::FoeB => write!(f, "FOE_B"),
            SharedCfgPin::FcsB => write!(f, "FCS_B"),
            SharedCfgPin::CsiB => write!(f, "CSI_B"),
            SharedCfgPin::RdWrB => write!(f, "RD_WR_B"),
            SharedCfgPin::EmCclk => write!(f, "EM_CCLK"),
            SharedCfgPin::PudcB => write!(f, "PUDC_B"),
            SharedCfgPin::AdvB => write!(f, "ADV_B"),
        }
    }
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
}

impl From<&Bond> for JsonValue {
    fn from(bond: &Bond) -> Self {
        jzon::object! {
            pins: jzon::object::Object::from_iter(
                bond.pins.iter().map(|(k, v)| (k, v.to_string()))
            ),
        }
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
            writeln!(f, "\t\t{pin:4}: {pad}")?;
        }
        Ok(())
    }
}
