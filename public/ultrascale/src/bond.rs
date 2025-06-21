use bincode::{Decode, Encode};
use itertools::Itertools;
use jzon::JsonValue;
use prjcombine_interconnect::grid::{DieId, TileIobId};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum CfgPad {
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
            CfgPad::Tck => write!(f, "TCK"),
            CfgPad::Tms => write!(f, "TMS"),
            CfgPad::Tdi => write!(f, "TDI"),
            CfgPad::Tdo => write!(f, "TDO"),
            CfgPad::HswapEn => write!(f, "HSWAP_EN"),
            CfgPad::Data(idx) => write!(f, "DATA{idx}"),
            CfgPad::CfgBvs => write!(f, "CFGBVS"),
            CfgPad::PorOverride => write!(f, "POR_OVERRIDE"),
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
    AVcc,
    AVtt,
    RRef,
    AVttRCal,
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
            GtPad::AVcc => write!(f, "AVCC"),
            GtPad::RRef => write!(f, "RREF"),
            GtPad::AVttRCal => write!(f, "AVTTRCAL"),
            GtPad::AVtt => write!(f, "AVTT"),
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
pub enum GtRegionPad {
    AVtt,
    AVcc,
    VccAux,
    VccInt,
}

impl std::fmt::Display for GtRegionPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GtRegionPad::AVtt => write!(f, "AVTT"),
            GtRegionPad::AVcc => write!(f, "AVCC"),
            GtRegionPad::VccAux => write!(f, "VCCAUX"),
            GtRegionPad::VccInt => write!(f, "VCCINT"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum SysMonPad {
    VP,
    VN,
}

impl std::fmt::Display for SysMonPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SysMonPad::VP => write!(f, "VP"),
            SysMonPad::VN => write!(f, "VN"),
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
            PsPad::DdrCkP(idx) => write!(f, "DDR_CKP{idx}"),
            PsPad::DdrCkN(idx) => write!(f, "DDR_CKN{idx}"),
            PsPad::DdrCke(idx) => write!(f, "DDR_CKE{idx}"),
            PsPad::DdrOdt(idx) => write!(f, "DDR_ODT{idx}"),
            PsPad::DdrCsB(idx) => write!(f, "DDR_CS_B{idx}"),
            PsPad::DdrDrstB => write!(f, "DDR_DRST_B"),
            PsPad::DdrActN => write!(f, "DDR_ACT_N"),
            PsPad::DdrAlertN => write!(f, "DDR_ALERT_N"),
            PsPad::DdrBg(idx) => write!(f, "DDR_BG{idx}"),
            PsPad::DdrParity => write!(f, "DDR_PARITY"),
            PsPad::DdrZq => write!(f, "DDR_ZQ"),
            PsPad::ErrorOut => write!(f, "ERROR_OUT"),
            PsPad::ErrorStatus => write!(f, "ERROR_STATUS"),
            PsPad::Done => write!(f, "DONE"),
            PsPad::InitB => write!(f, "INIT_B"),
            PsPad::ProgB => write!(f, "PROG_B"),
            PsPad::JtagTck => write!(f, "JTAG_TCK"),
            PsPad::JtagTdi => write!(f, "JTAG_TDI"),
            PsPad::JtagTdo => write!(f, "JTAG_TDO"),
            PsPad::JtagTms => write!(f, "JTAG_TMS"),
            PsPad::Mode(i) => write!(f, "MODE{i}"),
            PsPad::PadI => write!(f, "PAD_I"),
            PsPad::PadO => write!(f, "PAD_O"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum HbmPad {
    Vcc,
    VccIo,
    VccAux,
    Rsvd,
    RsvdGnd,
}

impl std::fmt::Display for HbmPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HbmPad::Vcc => write!(f, "VCC"),
            HbmPad::VccIo => write!(f, "VCCIO"),
            HbmPad::VccAux => write!(f, "VCCAUX"),
            HbmPad::Rsvd => write!(f, "RSVD"),
            HbmPad::RsvdGnd => write!(f, "RSVD_GND"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum RfDacPad {
    VOutP(u8),
    VOutN(u8),
    ClkP,
    ClkN,
    RExt,
    SysRefP,
    SysRefN,
}

impl std::fmt::Display for RfDacPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RfDacPad::VOutP(idx) => write!(f, "VOUT{idx}P"),
            RfDacPad::VOutN(idx) => write!(f, "VOUT{idx}N"),
            RfDacPad::ClkP => write!(f, "CLKP"),
            RfDacPad::ClkN => write!(f, "CLKN"),
            RfDacPad::RExt => write!(f, "REXT"),
            RfDacPad::SysRefP => write!(f, "SYSREFP"),
            RfDacPad::SysRefN => write!(f, "SYSREFN"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum RfAdcPad {
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

impl std::fmt::Display for RfAdcPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RfAdcPad::VInP(idx) => write!(f, "VIN{idx}_P"),
            RfAdcPad::VInN(idx) => write!(f, "VIN{idx}_N"),
            RfAdcPad::VInPairP(idx) => write!(f, "VIN_PAIR{idx}_P"),
            RfAdcPad::VInPairN(idx) => write!(f, "VIN_PAIR{idx}_N"),
            RfAdcPad::ClkP => write!(f, "CLKP"),
            RfAdcPad::ClkN => write!(f, "CLKN"),
            RfAdcPad::VCm(idx) => write!(f, "VCM{idx}"),
            RfAdcPad::RExt => write!(f, "REXT"),
            RfAdcPad::PllTestOutP => write!(f, "PLL_TEST_OUT_P"),
            RfAdcPad::PllTestOutN => write!(f, "PLL_TEST_OUT_N"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum BondPad {
    // bank, bel idx
    Hpio(u32, TileIobId),
    Hdio(u32, TileIobId),
    HdioLc(u32, TileIobId),
    Xp5io(u32, TileIobId),
    IoVref(u32),
    Xp5ioVr(u32),
    // bank, type
    Gt(u32, GtPad),
    GtRegion(GtRegion, GtRegionPad),
    // bank, type
    SysMon(DieId, SysMonPad),
    SysMonVRefP,
    SysMonVRefN,
    SysMonGnd,
    SysMonVcc,
    PsSysMonGnd,
    PsSysMonVcc,
    Cfg(CfgPad),
    Gnd,
    VccInt,
    VccAux,
    VccBram,
    VccAuxHpio,
    VccAuxHdio,
    VccAuxXp5io,
    VccAuxIo,
    VccIntIo,
    VccIntHpio,
    VccIntXp5io,
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
    IoPs(u32, PsPad),
    Hbm(u32, HbmPad),
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
    RfDac(u32, RfDacPad),
    RfAdc(u32, RfAdcPad),
}

impl std::fmt::Display for BondPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPad::Hpio(bank, idx) => write!(f, "HPIOB_{bank}_{idx:#}"),
            BondPad::Hdio(bank, idx) => write!(f, "HDIOB_{bank}_{idx:#}"),
            BondPad::HdioLc(bank, idx) => write!(f, "HDIOBLC_{bank}_{idx:#}"),
            BondPad::Xp5io(bank, idx) => write!(f, "XP5IOB_{bank}_{idx:#}"),
            BondPad::IoVref(bank) => write!(f, "IO_{bank}_VREF"),
            BondPad::Xp5ioVr(bank) => write!(f, "XP5IO_{bank}_VR"),
            BondPad::Gt(bank, gtpin) => write!(f, "GT{bank}_{gtpin}"),
            BondPad::GtRegion(reg, gtpin) => write!(f, "GT_{reg}_{gtpin}"),
            BondPad::Nc => write!(f, "NC"),
            BondPad::Gnd => write!(f, "GND"),
            BondPad::VccInt => write!(f, "VCCINT"),
            BondPad::VccAux => write!(f, "VCCAUX"),
            BondPad::VccBram => write!(f, "VCCBRAM"),
            BondPad::VccO(bank) => write!(f, "VCCO{bank}"),
            BondPad::VccBatt => write!(f, "VCC_BATT"),
            BondPad::Cfg(cfg_pin) => write!(f, "{cfg_pin}"),
            BondPad::Dxn => write!(f, "DXN"),
            BondPad::Dxp => write!(f, "DXP"),
            BondPad::Rsvd => write!(f, "RSVD"),
            BondPad::RsvdGnd => write!(f, "RSVDGND"),
            BondPad::SysMon(bank, pin) => write!(f, "SYSMON_{bank}_{pin}"),
            BondPad::VccPsAux => write!(f, "VCC_PS_AUX"),
            BondPad::VccPsPll => write!(f, "VCC_PS_PLL"),
            BondPad::IoPs(bank, pin) => write!(f, "PS{bank}_{pin}"),
            BondPad::SysMonVRefP => write!(f, "SYSMON_VREFP"),
            BondPad::SysMonVRefN => write!(f, "SYSMON_VREFN"),
            BondPad::SysMonGnd => write!(f, "SYSMON_GND"),
            BondPad::SysMonVcc => write!(f, "SYSMON_VCC"),
            BondPad::PsSysMonGnd => write!(f, "PS_SYSMON_GND"),
            BondPad::PsSysMonVcc => write!(f, "PS_SYSMON_VCC"),
            BondPad::VccAuxHpio => write!(f, "VCCAUX_HPIO"),
            BondPad::VccAuxHdio => write!(f, "VCCAUX_HDIO"),
            BondPad::VccAuxXp5io => write!(f, "VCCAUX_XP5IO"),
            BondPad::VccAuxIo => write!(f, "VCCAUX_IO"),
            BondPad::VccIntIo => write!(f, "VCCINT_IO"),
            BondPad::VccIntHpio => write!(f, "VCCINT_HPIO"),
            BondPad::VccIntXp5io => write!(f, "VCCINT_XP5IO"),
            BondPad::VccPsIntLp => write!(f, "VCC_PS_INT_LP"),
            BondPad::VccPsIntFp => write!(f, "VCC_PS_INT_FP"),
            BondPad::VccPsIntFpDdr => write!(f, "VCC_PS_INT_FP_DDR"),
            BondPad::VccPsBatt => write!(f, "VCC_PS_BATT"),
            BondPad::VccPsDdrPll => write!(f, "VCC_PS_DDR_PLL"),
            BondPad::VccIntVcu => write!(f, "VCCINT_VCU"),
            BondPad::GndSense => write!(f, "GND_SENSE"),
            BondPad::VccIntSense => write!(f, "VCCINT_SENSE"),
            BondPad::VccIntAms => write!(f, "VCCINT_AMS"),
            BondPad::VccSdfec => write!(f, "VCC_SDFEC"),
            BondPad::RfDacGnd => write!(f, "RFDAC_GND"),
            BondPad::RfDacSubGnd => write!(f, "RFDAC_AGND"),
            BondPad::RfDacAVcc => write!(f, "RFDAC_AVCC"),
            BondPad::RfDacAVccAux => write!(f, "RFDAC_AVCCAUX"),
            BondPad::RfDacAVtt => write!(f, "RFDAC_AVTT"),
            BondPad::RfAdcGnd => write!(f, "RFADC_GND"),
            BondPad::RfAdcSubGnd => write!(f, "RFADC_SUBGND"),
            BondPad::RfAdcAVcc => write!(f, "RFADC_AVCC"),
            BondPad::RfAdcAVccAux => write!(f, "RFADC_AVCCAUX"),
            BondPad::Hbm(bank, pin) => write!(f, "HBM{bank}_{pin}"),
            BondPad::RfDac(bank, pin) => write!(f, "RFDAC{bank}_{pin}"),
            BondPad::RfAdc(bank, pin) => write!(f, "RFADC{bank}_{pin}"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPad>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum SharedCfgPad {
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

impl std::fmt::Display for SharedCfgPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SharedCfgPad::Data(idx) => write!(f, "D{idx}"),
            SharedCfgPad::Addr(idx) => write!(f, "A{idx}"),
            SharedCfgPad::Rs(idx) => write!(f, "RS{idx}"),
            SharedCfgPad::Dout => write!(f, "DOUT"),
            SharedCfgPad::FweB => write!(f, "FWE_B"),
            SharedCfgPad::FoeB => write!(f, "FOE_B"),
            SharedCfgPad::CsiB => write!(f, "CSI_B"),
            SharedCfgPad::EmCclk => write!(f, "EM_CCLK"),
            SharedCfgPad::PudcB => write!(f, "PUDC_B"),
            SharedCfgPad::I2cSclk => write!(f, "I2C_SCLK"),
            SharedCfgPad::I2cSda => write!(f, "I2C_SDA"),
            SharedCfgPad::SmbAlert => write!(f, "SMB_ALERT"),
            SharedCfgPad::PerstN0 => write!(f, "PERST_N0"),
            SharedCfgPad::PerstN1 => write!(f, "PERST_N1"),
            SharedCfgPad::Busy => write!(f, "BUSY"),
            SharedCfgPad::Fcs1B => write!(f, "FCSI_B"),
            SharedCfgPad::OspiDs => write!(f, "OSPI_DS"),
            SharedCfgPad::OspiRstB => write!(f, "OSPI_RST_B"),
            SharedCfgPad::OspiEccFail => write!(f, "OSPI_ECC_FAIL"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandedBond<'a> {
    pub bond: &'a Bond,
    pub hpios: BTreeMap<(u32, TileIobId), String>,
    pub hdios: BTreeMap<(u32, TileIobId), String>,
    pub hdiolcs: BTreeMap<(u32, TileIobId), String>,
    pub gts: BTreeMap<(u32, GtPad), String>,
    pub sysmons: BTreeMap<(DieId, SysMonPad), String>,
}

impl Bond {
    pub fn expand(&self) -> ExpandedBond<'_> {
        let mut hpios = BTreeMap::new();
        let mut hdios = BTreeMap::new();
        let mut hdiolcs = BTreeMap::new();
        let mut gts = BTreeMap::new();
        let mut sysmons = BTreeMap::new();
        for (name, pad) in &self.pins {
            match *pad {
                BondPad::Hpio(bank, idx) => {
                    hpios.insert((bank, idx), name.clone());
                }
                BondPad::Hdio(bank, idx) => {
                    hdios.insert((bank, idx), name.clone());
                }
                BondPad::HdioLc(bank, idx) => {
                    hdiolcs.insert((bank, idx), name.clone());
                }
                BondPad::Gt(bank, gtpin) => {
                    gts.insert((bank, gtpin), name.clone());
                }
                BondPad::SysMon(bank, smpin) => {
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
