use itertools::Itertools;
use jzon::JsonValue;
use prjcombine_interconnect::grid::EdgeIoCoord;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CfgPin {
    Tck,
    Tdi,
    Tdo,
    Tms,
    CmpCsB,
    Done,
    ProgB,
    Suspend,
}

impl std::fmt::Display for CfgPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CfgPin::Done => write!(f, "DONE"),
            CfgPin::ProgB => write!(f, "PROG_B"),
            CfgPin::Tck => write!(f, "TCK"),
            CfgPin::Tms => write!(f, "TMS"),
            CfgPin::Tdi => write!(f, "TDI"),
            CfgPin::Tdo => write!(f, "TDO"),
            CfgPin::Suspend => write!(f, "SUSPEND"),
            CfgPin::CmpCsB => write!(f, "CMP_CS_B"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum GtPin {
    TxP(u8),
    TxN(u8),
    RxP(u8),
    RxN(u8),
    ClkP(u8),
    ClkN(u8),
    AVcc,
    AVccPll(u8),
    VtTx,
    VtRx,
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
            GtPin::VtRx => write!(f, "VTRX"),
            GtPin::VtTx => write!(f, "VTTX"),
            GtPin::ClkP(idx) => write!(f, "CLKP{idx}"),
            GtPin::ClkN(idx) => write!(f, "CLKN{idx}"),
            GtPin::AVcc => write!(f, "AVCC"),
            GtPin::AVccPll(idx) => write!(f, "AVCCPLL{idx}"),
            GtPin::RRef => write!(f, "RREF"),
            GtPin::AVttRCal => write!(f, "AVTTRCAL"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum BondPin {
    Io(EdgeIoCoord),
    Nc,
    Gnd,
    VccInt,
    VccAux,
    VccO(u32),
    VccBatt,
    Vfs,
    RFuse,
    Cfg(CfgPin),
    Gt(u32, GtPin),
}

impl std::fmt::Display for BondPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPin::Io(io) => write!(f, "{io}"),
            BondPin::Gt(bank, gtpin) => write!(f, "GT{bank}_{gtpin}"),
            BondPin::Nc => write!(f, "NC"),
            BondPin::Gnd => write!(f, "GND"),
            BondPin::VccInt => write!(f, "VCCINT"),
            BondPin::VccAux => write!(f, "VCCAUX"),
            BondPin::VccO(bank) => write!(f, "VCCO{bank}"),
            BondPin::VccBatt => write!(f, "VCCBATT"),
            BondPin::Cfg(cfg_pin) => write!(f, "{cfg_pin}"),
            BondPin::Vfs => write!(f, "VFS"),
            BondPin::RFuse => write!(f, "RFUSE"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
    // device bank -> pkg bank
    pub io_banks: BTreeMap<u32, u32>,
    pub vref: BTreeSet<EdgeIoCoord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandedBond<'a> {
    pub bond: &'a Bond,
    pub ios: BTreeMap<EdgeIoCoord, String>,
    pub gts: BTreeMap<(u32, GtPin), String>,
}

impl Bond {
    pub fn expand(&self) -> ExpandedBond {
        let mut ios = BTreeMap::new();
        let mut gts = BTreeMap::new();
        for (name, pad) in &self.pins {
            match *pad {
                BondPin::Io(io) => {
                    ios.insert(io, name.clone());
                }
                BondPin::Gt(bank, gtpin) => {
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
