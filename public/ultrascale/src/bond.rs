use bincode::{Decode, Encode};
use itertools::Itertools;
use jzon::JsonValue;
use prjcombine_interconnect::grid::{DieId, TileIobId};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
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
            CfgPin::Tck => write!(f, "TCK"),
            CfgPin::Tms => write!(f, "TMS"),
            CfgPin::Tdi => write!(f, "TDI"),
            CfgPin::Tdo => write!(f, "TDO"),
            CfgPin::HswapEn => write!(f, "HSWAP_EN"),
            CfgPin::Data(idx) => write!(f, "DATA{idx}"),
            CfgPin::CfgBvs => write!(f, "CFGBVS"),
            CfgPin::PorOverride => write!(f, "POR_OVERRIDE"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
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

impl std::fmt::Display for GtPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GtPin::RxP(idx) => write!(f, "RXP{idx}"),
            GtPin::RxN(idx) => write!(f, "RXN{idx}"),
            GtPin::TxP(idx) => write!(f, "TXP{idx}"),
            GtPin::TxN(idx) => write!(f, "TXN{idx}"),
            GtPin::ClkP(idx) => write!(f, "CLKP{idx}"),
            GtPin::ClkN(idx) => write!(f, "CLKN{idx}"),
            GtPin::AVcc => write!(f, "AVCC"),
            GtPin::RRef => write!(f, "RREF"),
            GtPin::AVttRCal => write!(f, "AVTTRCAL"),
            GtPin::AVtt => write!(f, "AVTT"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
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

impl std::fmt::Display for GtRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GtRegion::All => write!(f, "ALL"),
            GtRegion::L => write!(f, "L"),
            GtRegion::R => write!(f, "R"),
            GtRegion::LS => write!(f, "LS"),
            GtRegion::RS => write!(f, "RS"),
            GtRegion::LN => write!(f, "LN"),
            GtRegion::RN => write!(f, "RN"),
            GtRegion::LC => write!(f, "LC"),
            GtRegion::RC => write!(f, "RC"),
            GtRegion::LLC => write!(f, "LLC"),
            GtRegion::RLC => write!(f, "RLC"),
            GtRegion::LUC => write!(f, "LUC"),
            GtRegion::RUC => write!(f, "RUC"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum GtRegionPin {
    AVtt,
    AVcc,
    VccAux,
    VccInt,
}

impl std::fmt::Display for GtRegionPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GtRegionPin::AVtt => write!(f, "AVTT"),
            GtRegionPin::AVcc => write!(f, "AVCC"),
            GtRegionPin::VccAux => write!(f, "VCCAUX"),
            GtRegionPin::VccInt => write!(f, "VCCINT"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum SysMonPin {
    VP,
    VN,
}

impl std::fmt::Display for SysMonPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SysMonPin::VP => write!(f, "VP"),
            SysMonPin::VN => write!(f, "VN"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
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
            PsPin::DdrCkP(idx) => write!(f, "DDR_CKP{idx}"),
            PsPin::DdrCkN(idx) => write!(f, "DDR_CKN{idx}"),
            PsPin::DdrCke(idx) => write!(f, "DDR_CKE{idx}"),
            PsPin::DdrOdt(idx) => write!(f, "DDR_ODT{idx}"),
            PsPin::DdrCsB(idx) => write!(f, "DDR_CS_B{idx}"),
            PsPin::DdrDrstB => write!(f, "DDR_DRST_B"),
            PsPin::DdrActN => write!(f, "DDR_ACT_N"),
            PsPin::DdrAlertN => write!(f, "DDR_ALERT_N"),
            PsPin::DdrBg(idx) => write!(f, "DDR_BG{idx}"),
            PsPin::DdrParity => write!(f, "DDR_PARITY"),
            PsPin::DdrZq => write!(f, "DDR_ZQ"),
            PsPin::ErrorOut => write!(f, "ERROR_OUT"),
            PsPin::ErrorStatus => write!(f, "ERROR_STATUS"),
            PsPin::Done => write!(f, "DONE"),
            PsPin::InitB => write!(f, "INIT_B"),
            PsPin::ProgB => write!(f, "PROG_B"),
            PsPin::JtagTck => write!(f, "JTAG_TCK"),
            PsPin::JtagTdi => write!(f, "JTAG_TDI"),
            PsPin::JtagTdo => write!(f, "JTAG_TDO"),
            PsPin::JtagTms => write!(f, "JTAG_TMS"),
            PsPin::Mode(i) => write!(f, "MODE{i}"),
            PsPin::PadI => write!(f, "PAD_I"),
            PsPin::PadO => write!(f, "PAD_O"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum HbmPin {
    Vcc,
    VccIo,
    VccAux,
    Rsvd,
    RsvdGnd,
}

impl std::fmt::Display for HbmPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HbmPin::Vcc => write!(f, "VCC"),
            HbmPin::VccIo => write!(f, "VCCIO"),
            HbmPin::VccAux => write!(f, "VCCAUX"),
            HbmPin::Rsvd => write!(f, "RSVD"),
            HbmPin::RsvdGnd => write!(f, "RSVD_GND"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum RfDacPin {
    VOutP(u8),
    VOutN(u8),
    ClkP,
    ClkN,
    RExt,
    SysRefP,
    SysRefN,
}

impl std::fmt::Display for RfDacPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RfDacPin::VOutP(idx) => write!(f, "VOUT{idx}P"),
            RfDacPin::VOutN(idx) => write!(f, "VOUT{idx}N"),
            RfDacPin::ClkP => write!(f, "CLKP"),
            RfDacPin::ClkN => write!(f, "CLKN"),
            RfDacPin::RExt => write!(f, "REXT"),
            RfDacPin::SysRefP => write!(f, "SYSREFP"),
            RfDacPin::SysRefN => write!(f, "SYSREFN"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
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

impl std::fmt::Display for RfAdcPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RfAdcPin::VInP(idx) => write!(f, "VIN{idx}_P"),
            RfAdcPin::VInN(idx) => write!(f, "VIN{idx}_N"),
            RfAdcPin::VInPairP(idx) => write!(f, "VIN_PAIR{idx}_P"),
            RfAdcPin::VInPairN(idx) => write!(f, "VIN_PAIR{idx}_N"),
            RfAdcPin::ClkP => write!(f, "CLKP"),
            RfAdcPin::ClkN => write!(f, "CLKN"),
            RfAdcPin::VCm(idx) => write!(f, "VCM{idx}"),
            RfAdcPin::RExt => write!(f, "REXT"),
            RfAdcPin::PllTestOutP => write!(f, "PLL_TEST_OUT_P"),
            RfAdcPin::PllTestOutN => write!(f, "PLL_TEST_OUT_N"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
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

impl std::fmt::Display for BondPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPin::Hpio(bank, idx) => write!(f, "HPIOB_{bank}_{idx:#}"),
            BondPin::Hdio(bank, idx) => write!(f, "HDIOB_{bank}_{idx:#}"),
            BondPin::HdioLc(bank, idx) => write!(f, "HDIOBLC_{bank}_{idx:#}"),
            BondPin::IoVref(bank) => write!(f, "IO_{bank}_VREF"),
            BondPin::Gt(bank, gtpin) => write!(f, "GT{bank}_{gtpin}"),
            BondPin::GtRegion(reg, gtpin) => write!(f, "GT_{reg}_{gtpin}"),
            BondPin::Nc => write!(f, "NC"),
            BondPin::Gnd => write!(f, "GND"),
            BondPin::VccInt => write!(f, "VCCINT"),
            BondPin::VccAux => write!(f, "VCCAUX"),
            BondPin::VccBram => write!(f, "VCCBRAM"),
            BondPin::VccO(bank) => write!(f, "VCCO{bank}"),
            BondPin::VccBatt => write!(f, "VCC_BATT"),
            BondPin::Cfg(cfg_pin) => write!(f, "{cfg_pin}"),
            BondPin::Dxn => write!(f, "DXN"),
            BondPin::Dxp => write!(f, "DXP"),
            BondPin::Rsvd => write!(f, "RSVD"),
            BondPin::RsvdGnd => write!(f, "RSVDGND"),
            BondPin::SysMon(bank, pin) => write!(f, "SYSMON_{bank}_{pin}"),
            BondPin::VccPsAux => write!(f, "VCC_PS_AUX"),
            BondPin::VccPsPll => write!(f, "VCC_PS_PLL"),
            BondPin::IoPs(bank, pin) => write!(f, "PS{bank}_{pin}"),
            BondPin::SysMonVRefP => write!(f, "SYSMON_VREFP"),
            BondPin::SysMonVRefN => write!(f, "SYSMON_VREFN"),
            BondPin::SysMonGnd => write!(f, "SYSMON_GND"),
            BondPin::SysMonVcc => write!(f, "SYSMON_VCC"),
            BondPin::PsSysMonGnd => write!(f, "PS_SYSMON_GND"),
            BondPin::PsSysMonVcc => write!(f, "PS_SYSMON_VCC"),
            BondPin::VccAuxHpio => write!(f, "VCCAUX_HPIO"),
            BondPin::VccAuxHdio => write!(f, "VCCAUX_HDIO"),
            BondPin::VccAuxIo => write!(f, "VCCAUX_IO"),
            BondPin::VccIntIo => write!(f, "VCCINT_IO"),
            BondPin::VccPsIntLp => write!(f, "VCC_PS_INT_LP"),
            BondPin::VccPsIntFp => write!(f, "VCC_PS_INT_FP"),
            BondPin::VccPsIntFpDdr => write!(f, "VCC_PS_INT_FP_DDR"),
            BondPin::VccPsBatt => write!(f, "VCC_PS_BATT"),
            BondPin::VccPsDdrPll => write!(f, "VCC_PS_DDR_PLL"),
            BondPin::VccIntVcu => write!(f, "VCCINT_VCU"),
            BondPin::GndSense => write!(f, "GND_SENSE"),
            BondPin::VccIntSense => write!(f, "VCCINT_SENSE"),
            BondPin::VccIntAms => write!(f, "VCCINT_AMS"),
            BondPin::VccSdfec => write!(f, "VCC_SDFEC"),
            BondPin::RfDacGnd => write!(f, "RFDAC_GND"),
            BondPin::RfDacSubGnd => write!(f, "RFDAC_AGND"),
            BondPin::RfDacAVcc => write!(f, "RFDAC_AVCC"),
            BondPin::RfDacAVccAux => write!(f, "RFDAC_AVCCAUX"),
            BondPin::RfDacAVtt => write!(f, "RFDAC_AVTT"),
            BondPin::RfAdcGnd => write!(f, "RFADC_GND"),
            BondPin::RfAdcSubGnd => write!(f, "RFADC_SUBGND"),
            BondPin::RfAdcAVcc => write!(f, "RFADC_AVCC"),
            BondPin::RfAdcAVccAux => write!(f, "RFADC_AVCCAUX"),
            BondPin::Hbm(bank, pin) => write!(f, "HBM{bank}_{pin}"),
            BondPin::RfDac(bank, pin) => write!(f, "RFDAC{bank}_{pin}"),
            BondPin::RfAdc(bank, pin) => write!(f, "RFADC{bank}_{pin}"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
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

impl std::fmt::Display for SharedCfgPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SharedCfgPin::Data(idx) => write!(f, "D{idx}"),
            SharedCfgPin::Addr(idx) => write!(f, "A{idx}"),
            SharedCfgPin::Rs(idx) => write!(f, "RS{idx}"),
            SharedCfgPin::Dout => write!(f, "DOUT"),
            SharedCfgPin::FweB => write!(f, "FWE_B"),
            SharedCfgPin::FoeB => write!(f, "FOE_B"),
            SharedCfgPin::CsiB => write!(f, "CSI_B"),
            SharedCfgPin::EmCclk => write!(f, "EM_CCLK"),
            SharedCfgPin::PudcB => write!(f, "PUDC_B"),
            SharedCfgPin::I2cSclk => write!(f, "I2C_SCLK"),
            SharedCfgPin::I2cSda => write!(f, "I2C_SDA"),
            SharedCfgPin::SmbAlert => write!(f, "SMB_ALERT"),
            SharedCfgPin::PerstN0 => write!(f, "PERST_N0"),
            SharedCfgPin::PerstN1 => write!(f, "PERST_N1"),
            SharedCfgPin::Busy => write!(f, "BUSY"),
            SharedCfgPin::Fcs1B => write!(f, "FCSI_B"),
            SharedCfgPin::OspiDs => write!(f, "OSPI_DS"),
            SharedCfgPin::OspiRstB => write!(f, "OSPI_RST_B"),
            SharedCfgPin::OspiEccFail => write!(f, "OSPI_ECC_FAIL"),
        }
    }
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
