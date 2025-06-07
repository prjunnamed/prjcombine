use bincode::{Decode, Encode};
use itertools::Itertools;
use jzon::JsonValue;
use prjcombine_interconnect::grid::EdgeIoCoord;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum GtPad {
    RxP,
    RxN,
    TxP,
    TxN,
    GndA,
    VtRx,
    VtTx,
    AVccAuxRx,
    AVccAuxTx,
}

impl std::fmt::Display for GtPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GtPad::RxP => write!(f, "RXP"),
            GtPad::RxN => write!(f, "RXN"),
            GtPad::TxP => write!(f, "TXP"),
            GtPad::TxN => write!(f, "TXN"),
            GtPad::GndA => write!(f, "GNDA"),
            GtPad::VtRx => write!(f, "VTRX"),
            GtPad::VtTx => write!(f, "VTTX"),
            GtPad::AVccAuxRx => write!(f, "AVCCAUXRX"),
            GtPad::AVccAuxTx => write!(f, "AVCCAUXTX"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum CfgPad {
    Tck,
    Tdi,
    Tdo,
    Tms,
    Cclk,
    Done,
    ProgB,
    M0,
    M1,
    M2,
    HswapEn,
    PwrdwnB,
    Suspend,
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
            CfgPad::Tck => write!(f, "TCK"),
            CfgPad::Tms => write!(f, "TMS"),
            CfgPad::Tdi => write!(f, "TDI"),
            CfgPad::Tdo => write!(f, "TDO"),
            CfgPad::PwrdwnB => write!(f, "PWRDWN_B"),
            CfgPad::HswapEn => write!(f, "HSWAP_EN"),
            CfgPad::Suspend => write!(f, "SUSPEND"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum BondPad {
    Io(EdgeIoCoord),
    Gt(u32, GtPad),
    Nc,
    Rsvd,
    Gnd,
    VccInt,
    VccAux,
    VccO(u32),
    VccBatt,
    Cfg(CfgPad),
    Dxn,
    Dxp,
}

impl std::fmt::Display for BondPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPad::Io(io) => write!(f, "{io}"),
            BondPad::Gt(bank, gtpin) => write!(f, "GT{bank}_{gtpin}"),
            BondPad::Nc => write!(f, "NC"),
            BondPad::Gnd => write!(f, "GND"),
            BondPad::VccInt => write!(f, "VCCINT"),
            BondPad::VccAux => write!(f, "VCCAUX"),
            BondPad::VccO(bank) => write!(f, "VCCO{bank}"),
            BondPad::VccBatt => write!(f, "VCCBATT"),
            BondPad::Cfg(cfg_pin) => write!(f, "{cfg_pin}"),
            BondPad::Dxn => write!(f, "DXN"),
            BondPad::Dxp => write!(f, "DXP"),
            BondPad::Rsvd => write!(f, "RSVD"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPad>,
    // device bank -> pkg bank
    pub io_banks: BTreeMap<u32, u32>,
    pub vref: BTreeSet<EdgeIoCoord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandedBond<'a> {
    pub bond: &'a Bond,
    pub ios: BTreeMap<EdgeIoCoord, String>,
    pub gts: BTreeMap<(u32, GtPad), String>,
}

impl Bond {
    pub fn expand(&self) -> ExpandedBond {
        let mut ios = BTreeMap::new();
        let mut gts = BTreeMap::new();
        for (name, pad) in &self.pins {
            match *pad {
                BondPad::Io(io) => {
                    ios.insert(io, name.clone());
                }
                BondPad::Gt(bank, gtpin) => {
                    gts.insert((bank, gtpin), name.clone());
                }
                _ => (),
            }
        }
        ExpandedBond {
            bond: self,
            ios,
            gts,
        }
    }
}

impl From<&Bond> for JsonValue {
    fn from(bond: &Bond) -> Self {
        jzon::object! {
            pins: jzon::object::Object::from_iter(
                bond.pins.iter().map(|(k, v)| (k, v.to_string()))
            ),
            io_banks: jzon::object::Object::from_iter(bond.io_banks.iter().map(|(k, v)| (
                k.to_string(), *v
            ))),
            vref: Vec::from_iter(bond.vref.iter().map(|io| io.to_string())),
        }
    }
}

fn pad_sort_key(name: &str) -> (usize, &str, u32) {
    let pos = name.find(|x: char| x.is_ascii_digit()).unwrap();
    (pos, &name[..pos], name[pos..].parse().unwrap())
}

impl std::fmt::Display for Bond {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tBANKS:")?;
        for (k, v) in &self.io_banks {
            writeln!(f, "\t\t{k}: {v}")?;
        }
        writeln!(f, "\tPINS:")?;
        for (pin, pad) in self.pins.iter().sorted_by_key(|(k, _)| pad_sort_key(k)) {
            writeln!(f, "\t\t{pin:4}: {pad}")?;
        }
        writeln!(f, "\tVREF:")?;
        for v in &self.vref {
            writeln!(f, "\t\t{v}")?;
        }
        Ok(())
    }
}
