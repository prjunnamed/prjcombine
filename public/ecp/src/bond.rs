use std::collections::BTreeMap;

use bincode::{Decode, Encode};
use itertools::Itertools;
use jzon::JsonValue;
use prjcombine_interconnect::{
    dir::{DirH, DirHV, DirV},
    grid::{ColId, EdgeIoCoord},
};

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
    SleepB,
    Toe,
    Hfp,
    // the following are normally shared, except sometimes on ECP2M.
    WriteN,
    CsN,
    Cs1N,
    D(u8),
    Dout,
    Di,
    Busy,
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
            CfgPad::Tck => write!(f, "TCK"),
            CfgPad::Tms => write!(f, "TMS"),
            CfgPad::Tdi => write!(f, "TDI"),
            CfgPad::Tdo => write!(f, "TDO"),
            CfgPad::SleepB => write!(f, "SLEEP_B"),
            CfgPad::Toe => write!(f, "TOE"),
            CfgPad::Hfp => write!(f, "HFP"),
            CfgPad::WriteN => write!(f, "WRITE_N"),
            CfgPad::CsN => write!(f, "CS_N"),
            CfgPad::Cs1N => write!(f, "CS1_N"),
            CfgPad::D(i) => write!(f, "D{i}"),
            CfgPad::Dout => write!(f, "DOUT"),
            CfgPad::Di => write!(f, "DI"),
            CfgPad::Busy => write!(f, "BUSY"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum PllSet {
    All,
    Side(DirH),
    Quad(DirHV),
}

impl std::fmt::Display for PllSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PllSet::All => write!(f, "ALL"),
            PllSet::Side(dir) => write!(f, "{dir}"),
            PllSet::Quad(quad) => write!(f, "{quad}"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum SerdesPad {
    InP(u8),
    InN(u8),
    OutP(u8),
    OutN(u8),
    ClkP,
    ClkN,
    VccP,
    VccA,
    VccAuxA,
    VccAux33,
    VccTx(u8),
    VccRx(u8),
    VccIB(u8),
    VccOB(u8),
    RxantOutP,
    RxantOutN,
    AuxTstPadOutP,
    AuxTstPadOutN,
    VccTxCommon,
}

impl std::fmt::Display for SerdesPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerdesPad::InP(ch) => write!(f, "CH{ch}_IN_P"),
            SerdesPad::InN(ch) => write!(f, "CH{ch}_IN_N"),
            SerdesPad::OutP(ch) => write!(f, "CH{ch}_OUT_P"),
            SerdesPad::OutN(ch) => write!(f, "CH{ch}_OUT_N"),
            SerdesPad::ClkP => write!(f, "CLK_P"),
            SerdesPad::ClkN => write!(f, "CLK_N"),
            SerdesPad::VccP => write!(f, "VCCP"),
            SerdesPad::VccAux33 => write!(f, "VCCAUX33"),
            SerdesPad::VccTx(ch) => write!(f, "CH{ch}_VCCTX"),
            SerdesPad::VccRx(ch) => write!(f, "CH{ch}_VCCTX"),
            SerdesPad::VccIB(ch) => write!(f, "CH{ch}_VCCIB"),
            SerdesPad::VccOB(ch) => write!(f, "CH{ch}_VCCOB"),
            SerdesPad::RxantOutP => write!(f, "RXANTOUTP"),
            SerdesPad::RxantOutN => write!(f, "RXANTOUTN"),
            SerdesPad::AuxTstPadOutP => write!(f, "AUXTSTPADOUTP"),
            SerdesPad::AuxTstPadOutN => write!(f, "AUXTSTPADOUTN"),
            SerdesPad::VccA => write!(f, "VCCA"),
            SerdesPad::VccAuxA => write!(f, "VCCAUXA"),
            SerdesPad::VccTxCommon => write!(f, "VCCTX_COMMON"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum AscPad {
    HvOut(u8),
    HviMonP,
    HiMonN,
    IMonP(u8),
    IMonN(u8),
    TMonP(u8),
    TMonN(u8),
    VMon(u8),
    VMonGs(u8),
    Trim(u8),
    Gpio(u8),
    Ldrv,
    Hdrv,
    RDat,
    WDat,
    WrClk,
    AscClk,
    Scl,
    Sda,
    I2cAddr,
    ResetB,
    VssA,
    VddA,
    VddD,
    Vdc,
}

impl std::fmt::Display for AscPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AscPad::HvOut(i) => write!(f, "HVOUT{i}"),
            AscPad::HviMonP => write!(f, "HVIMONP"),
            AscPad::HiMonN => write!(f, "HIMONN"),
            AscPad::IMonP(i) => write!(f, "IMON{i}P"),
            AscPad::IMonN(i) => write!(f, "IMON{i}N"),
            AscPad::TMonP(i) => write!(f, "TMON{i}P"),
            AscPad::TMonN(i) => write!(f, "TMON{i}N"),
            AscPad::VMon(i) => write!(f, "VMON{i}"),
            AscPad::VMonGs(i) => write!(f, "VMON{i}GS"),
            AscPad::Trim(i) => write!(f, "TRIM{i}"),
            AscPad::Gpio(i) => write!(f, "GPIO{i}"),
            AscPad::Ldrv => write!(f, "LDRV"),
            AscPad::Hdrv => write!(f, "HDRV"),
            AscPad::RDat => write!(f, "RDAT"),
            AscPad::WDat => write!(f, "WDATA"),
            AscPad::WrClk => write!(f, "WRCLK"),
            AscPad::AscClk => write!(f, "ASCCLK"),
            AscPad::Scl => write!(f, "SCL"),
            AscPad::Sda => write!(f, "SDA"),
            AscPad::I2cAddr => write!(f, "I2C_ADDR"),
            AscPad::ResetB => write!(f, "RESET_B"),
            AscPad::VssA => write!(f, "VSSA"),
            AscPad::VddA => write!(f, "VDDA"),
            AscPad::VddD => write!(f, "VDDD"),
            AscPad::Vdc => write!(f, "VDC"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum PfrPad {
    Io(EdgeIoCoord),
    JtagEn,
    AdcRefP0,
    AdcRefP1,
    AdcDp0,
    AdcDp1,
    VccInt,
    VccIo(u32),
    VccAux,
    VccAuxA,
    VccAuxH(u32),
    VccEclk,
    VccAdc18,
    VssAdc,
}

impl std::fmt::Display for PfrPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PfrPad::Io(io) => write!(f, "{io}"),
            PfrPad::JtagEn => write!(f, "JTAG_EN"),
            PfrPad::AdcRefP0 => write!(f, "ADC_REFP0"),
            PfrPad::AdcRefP1 => write!(f, "ADC_REFP1"),
            PfrPad::AdcDp0 => write!(f, "ADC_DP0"),
            PfrPad::AdcDp1 => write!(f, "ADC_DP1"),
            PfrPad::VccInt => write!(f, "VCCINT"),
            PfrPad::VccIo(bank) => write!(f, "VCCIO{bank}"),
            PfrPad::VccAux => write!(f, "VCCAUX"),
            PfrPad::VccAuxA => write!(f, "VCCAUXA"),
            PfrPad::VccAuxH(bank) => write!(f, "VCCAUXH{bank}"),
            PfrPad::VccEclk => write!(f, "VCCECLK"),
            PfrPad::VccAdc18 => write!(f, "VCCADC18"),
            PfrPad::VssAdc => write!(f, "VSSADC"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum BondPad {
    Io(EdgeIoCoord),
    Serdes(DirV, ColId, SerdesPad),
    SerdesCorner(SerdesPad),
    Cfg(CfgPad),
    Nc,
    Gnd,
    GndA,
    VccInt,
    VccAux,
    VccAuxA,
    VccJtag,
    VccIo(u32),
    Vtt(u32),
    VccPll(PllSet),
    GndPll(PllSet),
    PllCap(PllSet),
    VccA,
    TempVss,
    TempSense,
    XRes,
    Other,
    Asc(AscPad),
    IoAsc(EdgeIoCoord, AscPad),
    Pfr(PfrPad),
    IoPfr(EdgeIoCoord, PfrPad),
}

impl std::fmt::Display for BondPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPad::Io(io) => write!(f, "{io}"),
            BondPad::Serdes(edge, col, pad) => write!(f, "SERDES_{edge}{col:#}_{pad}"),
            BondPad::SerdesCorner(pad) => write!(f, "SERDES_CORNER_{pad}"),
            BondPad::Cfg(cfg_pin) => write!(f, "{cfg_pin}"),
            BondPad::Nc => write!(f, "NC"),
            BondPad::Gnd => write!(f, "GND"),
            BondPad::GndA => write!(f, "GNDA"),
            BondPad::VccInt => write!(f, "VCCINT"),
            BondPad::VccAux => write!(f, "VCCAUX"),
            BondPad::VccAuxA => write!(f, "VCCAUXA"),
            BondPad::VccJtag => write!(f, "VCC_JTAG"),
            BondPad::VccIo(bank) => write!(f, "VCCIO{bank}"),
            BondPad::Vtt(bank) => write!(f, "VTT{bank}"),
            BondPad::VccPll(set) => write!(f, "VCCPLL_{set}"),
            BondPad::GndPll(set) => write!(f, "GNDPLL_{set}"),
            BondPad::PllCap(set) => write!(f, "PLLCAP_{set}"),
            BondPad::VccA => write!(f, "VCCA"),
            BondPad::XRes => write!(f, "XRES"),
            BondPad::Other => write!(f, "OTHER"),
            BondPad::TempVss => write!(f, "TEMP_VSS"),
            BondPad::TempSense => write!(f, "TEMP_SENSE"),
            BondPad::Asc(pad) => write!(f, "ASC_{pad}"),
            BondPad::IoAsc(io, pad) => write!(f, "{io}_ASC_{pad}"),
            BondPad::Pfr(pad) => write!(f, "PFR_{pad}"),
            BondPad::IoPfr(io, pfr) => write!(f, "{io}_PFR_{pfr}"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum BondKind {
    Single,
    Asc,
    MachNx,
}

impl std::fmt::Display for BondKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondKind::Single => write!(f, "single"),
            BondKind::Asc => write!(f, "ASC"),
            BondKind::MachNx => write!(f, "Mach-NX"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Bond {
    pub kind: BondKind,
    pub pins: BTreeMap<String, BondPad>,
    // MachNX: XO5 IO -> XO3 IO
    pub pfr_io: BTreeMap<PfrPad, EdgeIoCoord>,
}

impl From<&Bond> for JsonValue {
    fn from(bond: &Bond) -> Self {
        jzon::object! {
            kind: bond.kind.to_string(),
            pins: jzon::object::Object::from_iter(
                bond.pins.iter().map(|(k, v)| (k, v.to_string()))
            ),
            pfr_io: jzon::object::Object::from_iter(
                bond.pfr_io.iter().map(|(k, v)| (k.to_string(), v.to_string()))
            ),
        }
    }
}

fn pad_sort_key(name: &str) -> (usize, &str, u32) {
    if let Some(pos) = name.rfind(|x: char| x.is_ascii_digit()) {
        (pos, &name[..pos], name[pos..].parse().unwrap())
    } else {
        (name.len(), name, 0)
    }
}

impl std::fmt::Display for Bond {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tKIND: {}", self.kind)?;
        writeln!(f, "\tPINS:")?;
        for (pin, pad) in self.pins.iter().sorted_by_key(|(k, _)| pad_sort_key(k)) {
            writeln!(f, "\t\t{pin:4}: {pad}")?;
        }
        if !self.pfr_io.is_empty() {
            writeln!(f, "\tPFR IO:")?;
            for (&pfr, &io) in &self.pfr_io {
                writeln!(f, "\t\t{pfr}: {io}")?;
            }
        }
        Ok(())
    }
}
