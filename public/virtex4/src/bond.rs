use bincode::{Decode, Encode};
use itertools::Itertools;
use jzon::JsonValue;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum CfgPad {
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

impl std::fmt::Display for CfgPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CfgPad::Cclk => write!(f, "CCLK"),
            CfgPad::Done => write!(f, "DONE"),
            CfgPad::M0 => write!(f, "M0"),
            CfgPad::M1 => write!(f, "M1"),
            CfgPad::M2 => write!(f, "M2"),
            CfgPad::ProgB => write!(f, "PROG_B"),
            CfgPad::InitB => write!(f, "INIT_B"),
            CfgPad::RdWrB => write!(f, "RDWR_B"),
            CfgPad::CsiB => write!(f, "CSI_B"),
            CfgPad::Tck => write!(f, "TCK"),
            CfgPad::Tms => write!(f, "TMS"),
            CfgPad::Tdi => write!(f, "TDI"),
            CfgPad::Tdo => write!(f, "TDO"),
            CfgPad::PwrdwnB => write!(f, "PWRDWN_B"),
            CfgPad::HswapEn => write!(f, "HSWAP_EN"),
            CfgPad::Din => write!(f, "DIN"),
            CfgPad::Dout => write!(f, "DOUT"),
            CfgPad::CfgBvs => write!(f, "CFGBVS"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum GtPad {
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

impl std::fmt::Display for GtPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GtPad::RxP(idx) => write!(f, "RXP{idx}"),
            GtPad::RxN(idx) => write!(f, "RXN{idx}"),
            GtPad::TxP(idx) => write!(f, "TXP{idx}"),
            GtPad::TxN(idx) => write!(f, "TXN{idx}"),
            GtPad::ClkP(idx) => write!(f, "CLKP{idx}"),
            GtPad::ClkN(idx) => write!(f, "CLKN{idx}"),
            GtPad::GndA => write!(f, "GNDA"),
            GtPad::AVccAuxTx => write!(f, "AVCCAUXTX"),
            GtPad::AVccAuxRx(idx) => write!(f, "AVCCAUXRX{idx}"),
            GtPad::AVccAuxMgt => write!(f, "AVCCAUXMGT"),
            GtPad::RTerm => write!(f, "RTERM"),
            GtPad::MgtVRef => write!(f, "MGTVREF"),
            GtPad::VtRx(idx) => write!(f, "VTRX{idx}"),
            GtPad::VtTx(idx) => write!(f, "VTTX{idx}"),
            GtPad::AVcc => write!(f, "AVCC"),
            GtPad::AVccPll => write!(f, "AVCCPLL"),
            GtPad::RRef => write!(f, "RREF"),
            GtPad::AVttRCal => write!(f, "AVTTRCAL"),
            GtPad::RBias => write!(f, "RBIAS"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum GtRegionPad {
    AVtt,
    AGnd,
    AVcc,
    AVccRx,
    AVccPll,
    AVttRxC,
    VccAux,
}

impl std::fmt::Display for GtRegionPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GtRegionPad::AVtt => write!(f, "AVTT"),
            GtRegionPad::AGnd => write!(f, "AGND"),
            GtRegionPad::AVcc => write!(f, "AVCC"),
            GtRegionPad::AVccRx => write!(f, "AVCCRX"),
            GtRegionPad::AVccPll => write!(f, "AVCCPLL"),
            GtRegionPad::AVttRxC => write!(f, "AVTTRXC"),
            GtRegionPad::VccAux => write!(f, "VCCAUX"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum SysMonPad {
    VP,
    VN,
    AVss,
    AVdd,
    VRefP,
    VRefN,
}

impl std::fmt::Display for SysMonPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SysMonPad::VP => write!(f, "VP"),
            SysMonPad::VN => write!(f, "VN"),
            SysMonPad::AVss => write!(f, "AVSS"),
            SysMonPad::AVdd => write!(f, "AVDD"),
            SysMonPad::VRefP => write!(f, "VREFP"),
            SysMonPad::VRefN => write!(f, "VREFN"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum GtzPad {
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

impl std::fmt::Display for GtzPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GtzPad::RxP(idx) => write!(f, "RXP{idx}"),
            GtzPad::RxN(idx) => write!(f, "RXN{idx}"),
            GtzPad::TxP(idx) => write!(f, "TXP{idx}"),
            GtzPad::TxN(idx) => write!(f, "TXN{idx}"),
            GtzPad::ClkP(idx) => write!(f, "CLKP{idx}"),
            GtzPad::ClkN(idx) => write!(f, "CLKN{idx}"),
            GtzPad::AGnd => write!(f, "AGND"),
            GtzPad::AVcc => write!(f, "AVCC"),
            GtzPad::VccH => write!(f, "VCCH"),
            GtzPad::VccL => write!(f, "VCCL"),
            GtzPad::ObsClkP => write!(f, "OBSCLKP"),
            GtzPad::ObsClkN => write!(f, "OBSCLKN"),
            GtzPad::ThermIn => write!(f, "THERM_IN"),
            GtzPad::ThermOut => write!(f, "THERM_OUT"),
            GtzPad::SenseAGnd => write!(f, "SENSE_AGND"),
            GtzPad::SenseGnd => write!(f, "SENSE_GND"),
            GtzPad::SenseGndL => write!(f, "SENSE_GNDL"),
            GtzPad::SenseAVcc => write!(f, "SENSE_AVCC"),
            GtzPad::SenseVcc => write!(f, "SENSE_VCC"),
            GtzPad::SenseVccL => write!(f, "SENSE_VCCL"),
            GtzPad::SenseVccH => write!(f, "SENSE_VCCH"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum PsPad {
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

impl std::fmt::Display for PsPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PsPad::Mio(i) => write!(f, "MIO{i}"),
            PsPad::Clk => write!(f, "CLK"),
            PsPad::PorB => write!(f, "POR_B"),
            PsPad::SrstB => write!(f, "SRST_B"),
            PsPad::DdrDq(i) => write!(f, "DDR_DQ{i}"),
            PsPad::DdrDm(i) => write!(f, "DDR_DM{i}"),
            PsPad::DdrDqsP(i) => write!(f, "DDR_DQS_P{i}"),
            PsPad::DdrDqsN(i) => write!(f, "DDR_DQS_N{i}"),
            PsPad::DdrA(i) => write!(f, "DDR_A{i}"),
            PsPad::DdrBa(i) => write!(f, "DDR_BA{i}"),
            PsPad::DdrVrP => write!(f, "DDR_VRP"),
            PsPad::DdrVrN => write!(f, "DDR_VRN"),
            PsPad::DdrCkP => write!(f, "DDR_CKP"),
            PsPad::DdrCkN => write!(f, "DDR_CKN"),
            PsPad::DdrCke => write!(f, "DDR_CKE"),
            PsPad::DdrOdt => write!(f, "DDR_ODT"),
            PsPad::DdrDrstB => write!(f, "DDR_DRST_B"),
            PsPad::DdrCsB => write!(f, "DDR_CS_B"),
            PsPad::DdrRasB => write!(f, "DDR_RAS_B"),
            PsPad::DdrCasB => write!(f, "DDR_CAS_B"),
            PsPad::DdrWeB => write!(f, "DDR_WE_B"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum BondPad {
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
    Cfg(CfgPad),
    Gt(u32, GtPad),
    Gtz(u32, GtzPad),
    GtRegion(GtRegion, GtRegionPad),
    Dxp,
    Dxn,
    Vfs,
    SysMon(u32, SysMonPad),
    VccPsInt,
    VccPsAux,
    VccPsPll,
    PsVref(u32, u32),
    PsIo(u32, PsPad),
}

impl std::fmt::Display for BondPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPad::Io(bank, idx) => write!(f, "IOB_{bank}_{idx}"),
            BondPad::Gt(bank, gtpin) => write!(f, "GT{bank}_{gtpin}"),
            BondPad::Gtz(bank, gtpin) => write!(f, "GTZ{bank}_{gtpin}"),
            BondPad::GtRegion(region, gtpin) => write!(f, "GTREG_{region}_{gtpin}"),
            BondPad::Nc => write!(f, "NC"),
            BondPad::Gnd => write!(f, "GND"),
            BondPad::VccInt => write!(f, "VCCINT"),
            BondPad::VccAux => write!(f, "VCCAUX"),
            BondPad::VccAuxIo(idx) => write!(f, "VCCAUX_IO{idx}"),
            BondPad::VccBram => write!(f, "VCCBRAM"),
            BondPad::VccO(bank) => write!(f, "VCCO{bank}"),
            BondPad::VccBatt => write!(f, "VCC_BATT"),
            BondPad::Cfg(pin) => write!(f, "{pin}"),
            BondPad::Dxn => write!(f, "DXN"),
            BondPad::Dxp => write!(f, "DXP"),
            BondPad::Rsvd => write!(f, "RSVD"),
            BondPad::RsvdGnd => write!(f, "RSVDGND"),
            BondPad::Vfs => write!(f, "VFS"),
            BondPad::SysMon(bank, pin) => write!(f, "SYSMON{bank}_{pin}"),
            BondPad::VccPsInt => write!(f, "VCC_PS_INT"),
            BondPad::VccPsAux => write!(f, "VCC_PS_AUX"),
            BondPad::VccPsPll => write!(f, "VCC_PS_PLL"),
            BondPad::PsVref(bank, idx) => write!(f, "PS{bank}.VREF{idx}"),
            BondPad::PsIo(bank, pin) => write!(f, "PS{bank}_{pin}"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPad>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum SharedCfgPad {
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

impl std::fmt::Display for SharedCfgPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SharedCfgPad::Data(idx) => write!(f, "D{idx}"),
            SharedCfgPad::Addr(idx) => write!(f, "A{idx}"),
            SharedCfgPad::Rs(idx) => write!(f, "RS{idx}"),
            SharedCfgPad::CsoB => write!(f, "CSO_B"),
            SharedCfgPad::FweB => write!(f, "FWE_B"),
            SharedCfgPad::FoeB => write!(f, "FOE_B"),
            SharedCfgPad::FcsB => write!(f, "FCS_B"),
            SharedCfgPad::CsiB => write!(f, "CSI_B"),
            SharedCfgPad::RdWrB => write!(f, "RD_WR_B"),
            SharedCfgPad::EmCclk => write!(f, "EM_CCLK"),
            SharedCfgPad::PudcB => write!(f, "PUDC_B"),
            SharedCfgPad::AdvB => write!(f, "ADV_B"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandedBond<'a> {
    pub bond: &'a Bond,
    pub ios: BTreeMap<(u32, u32), String>,
    pub gts: BTreeMap<(u32, GtPad), String>,
    pub gtzs: BTreeMap<(u32, GtzPad), String>,
    pub sysmons: BTreeMap<(u32, SysMonPad), String>,
}

impl Bond {
    pub fn expand(&self) -> ExpandedBond<'_> {
        let mut ios = BTreeMap::new();
        let mut gts = BTreeMap::new();
        let mut gtzs = BTreeMap::new();
        let mut sysmons = BTreeMap::new();
        for (name, pad) in &self.pins {
            match *pad {
                BondPad::Io(bank, idx) => {
                    ios.insert((bank, idx), name.clone());
                }
                BondPad::Gt(bank, gtpin) => {
                    gts.insert((bank, gtpin), name.clone());
                }
                BondPad::Gtz(bank, gtpin) => {
                    gtzs.insert((bank, gtpin), name.clone());
                }
                BondPad::SysMon(bank, smpin) => {
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
