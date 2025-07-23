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
    VccAux33,
    VccTx(u8),
    VccRx(u8),
    VccIB(u8),
    VccOB(u8),
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
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum BondPad {
    Io(EdgeIoCoord),
    Serdes(DirV, ColId, SerdesPad),
    Nc,
    Gnd,
    VccInt,
    VccAux,
    VccJtag,
    VccIo(u32),
    VccPll(PllSet),
    GndPll(PllSet),
    PllCap(PllSet),
    Cfg(CfgPad),
    XRes,
    Other,
}

impl std::fmt::Display for BondPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPad::Io(io) => write!(f, "{io}"),
            BondPad::Serdes(edge, col, pad) => write!(f, "SERDES_{edge}{col:#}_{pad}"),
            BondPad::Nc => write!(f, "NC"),
            BondPad::Gnd => write!(f, "GND"),
            BondPad::VccInt => write!(f, "VCCINT"),
            BondPad::VccAux => write!(f, "VCCAUX"),
            BondPad::VccJtag => write!(f, "VCC_JTAG"),
            BondPad::VccIo(bank) => write!(f, "VCCIO{bank}"),
            BondPad::VccPll(set) => write!(f, "VCCPLL_{set}"),
            BondPad::GndPll(set) => write!(f, "GNDPLL_{set}"),
            BondPad::PllCap(set) => write!(f, "PLLCAP_{set}"),
            BondPad::Cfg(cfg_pin) => write!(f, "{cfg_pin}"),
            BondPad::XRes => write!(f, "XRES"),
            BondPad::Other => write!(f, "OTHER"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPad>,
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
