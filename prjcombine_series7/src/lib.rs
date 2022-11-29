#![allow(clippy::collapsible_else_if)]

use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};
use prjcombine_int::grid::{ColId, DieId, ExpandedGrid};
use prjcombine_virtex_bitstream::BitstreamGeom;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};

pub use prjcombine_virtex4::bond::{CfgPin, GtPin, SharedCfgPin, SysMonPin};
pub use prjcombine_virtex4::{
    CfgRowKind, ColumnKind, DisabledPart, Grid, GridKind, Gt, GtColumn, GtKind, HardColumn,
    IoColumn, IoCoord, IoKind, Pcie2, Pcie2Kind, RegId, SysMon, TileIobId,
};

mod expand;
pub mod io;

pub use expand::expand_grid;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum XadcIoLoc {
    Left,
    Right,
    LR,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtRegionPin {
    AVtt,
    AVcc,
    VccAux,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
    DdrCkP(u32),
    DdrCkN(u32),
    DdrCke(u32),
    DdrOdt(u32),
    DdrDrstB,
    DdrCsB(u32),
    DdrRasB,
    DdrCasB,
    DdrWeB,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BondPin {
    // bank, pin within bank
    Io(u32, u32),
    Nc,
    Gnd,
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
    GtRegion(u32, GtRegionPin),
    Dxp,
    Dxn,
    SysMon(u32, SysMonPin),
    VccPsInt,
    VccPsAux,
    VccPsPll,
    PsVref(u32, u32),
    PsIo(u32, PsPin),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ExtraDie {
    GtzTop,
    GtzBottom,
}

pub struct FrameGeom {
    pub col_frame: EntityVec<RegId, EntityVec<ColId, usize>>,
    pub bram_frame: EntityVec<RegId, EntityPartVec<ColId, usize>>,
}

pub struct ExpandedDevice<'a> {
    pub grids: EntityVec<DieId, &'a Grid>,
    pub grid_master: DieId,
    pub egrid: ExpandedGrid<'a>,
    pub disabled: BTreeSet<DisabledPart>,
    pub extras: Vec<ExtraDie>,
    pub bs_geom: BitstreamGeom,
    pub frames: EntityVec<DieId, FrameGeom>,
    pub col_lio: Option<ColId>,
    pub col_rio: Option<ColId>,
    pub col_lgt: Option<ColId>,
    pub col_rgt: Option<ColId>,
    pub col_mgt: Option<(ColId, ColId)>,
    pub col_cfg: ColId,
    pub col_clk: ColId,
    pub gt: Vec<Gt>,
    pub sysmon: Vec<SysMon>,
}

impl<'a> ExpandedDevice<'a> {
    pub fn adjust_vivado(&mut self) {
        let lvb6 = self
            .egrid
            .db
            .wires
            .iter()
            .find_map(|(k, v)| if v.name == "LVB.6" { Some(k) } else { None })
            .unwrap();
        let mut cursed_wires = HashSet::new();
        for i in 1..self.grids.len() {
            let dieid_s = DieId::from_idx(i - 1);
            let dieid_n = DieId::from_idx(i);
            let die_s = self.egrid.die(dieid_s);
            let die_n = self.egrid.die(dieid_n);
            for col in die_s.cols() {
                let row_s = die_s.rows().next_back().unwrap() - 49;
                let row_n = die_n.rows().next().unwrap() + 1;
                if !die_s[(col, row_s)].nodes.is_empty() && !die_n[(col, row_n)].nodes.is_empty() {
                    cursed_wires.insert((dieid_s, (col, row_s), lvb6));
                }
            }
        }
        self.egrid.blackhole_wires.extend(cursed_wires);
    }
}
