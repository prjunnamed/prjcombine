#![allow(clippy::collapsible_else_if)]

use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::grid::{ColId, DieId, ExpandedGrid, RowId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};

mod expand;
pub mod io;

pub use expand::expand_grid;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub kind: GridKind,
    pub xadc_kind: XadcKind,
    pub columns: EntityVec<ColId, ColumnKind>,
    pub cols_vbrk: BTreeSet<ColId>,
    pub col_cfg: ColId,
    pub col_clk: ColId,
    pub cols_io: [Option<IoColumn>; 2],
    pub cols_gt: [Option<GtColumn>; 2],
    pub cols_gtp_mid: Option<(GtColumn, GtColumn)>,
    pub regs: usize,
    pub reg_cfg: usize,
    pub reg_clk: usize,
    pub pcie2: Vec<Pcie2>,
    pub pcie3: Vec<(ColId, RowId)>,
    pub has_ps: bool,
    pub has_slr: bool,
    pub has_no_tbuturn: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GridKind {
    Artix,
    Kintex,
    Virtex,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum XadcKind {
    Left,
    Right,
    Both,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKind {
    ClbLL,
    ClbLM,
    Bram,
    Dsp,
    Io,
    Cmt,
    Gt,
    Cfg,
    Clk,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IoColumn {
    pub col: ColId,
    pub regs: Vec<Option<IoKind>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GtColumn {
    pub col: ColId,
    pub regs: Vec<Option<GtKind>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtKind {
    Gtp,
    Gtx,
    Gth,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IoKind {
    Hpio,
    Hrio,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Pcie2Kind {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Pcie2 {
    pub kind: Pcie2Kind,
    pub col: ColId,
    pub row: RowId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    // ×32; high 16 bits are also low 16 bits of Addr
    // 0 doubles as MOSI
    // 1 doubles as DIN
    Data(u8),
    Addr(u8), // ×29 total, but 0-15 are represented as Data(16-31)
    CsiB,
    Dout, // doubles as CSO_B
    RdWrB,
    EmCclk,
    PudcB,
    Rs(u8), // ×2
    AdvB,
    FweB,
    FoeB,
    FcsB,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
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
    InitB,
    CfgBvs,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtPin {
    RxP(u8),
    RxN(u8),
    TxP(u8),
    TxN(u8),
    ClkP(u8),
    ClkN(u8),
    RRef,
    AVttRCal,
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
pub enum SysMonPin {
    VP,
    VN,
    AVdd,
    AVss,
    VRefP,
    VRefN,
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
    SysMon(DieId, SysMonPin),
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
pub enum DisabledPart {
    Gtp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ExtraDie {
    GtzTop,
    GtzBottom,
}

pub struct ExpandedDevice<'a> {
    pub grids: EntityVec<DieId, &'a Grid>,
    pub grid_master: DieId,
    pub egrid: ExpandedGrid<'a>,
    pub disabled: BTreeSet<DisabledPart>,
    pub extras: Vec<ExtraDie>,
}

impl Grid {
    pub fn row_hclk(&self, row: RowId) -> RowId {
        RowId::from_idx(row.to_idx() / 50 * 50 + 25)
    }
    pub fn row_bufg(&self) -> RowId {
        RowId::from_idx(self.reg_clk * 50)
    }

    pub fn col_ps(&self) -> ColId {
        assert!(self.has_ps);
        ColId::from_idx(18)
    }
}

impl<'a> ExpandedDevice<'a> {
    pub fn adjust_ise(&mut self) {
        for (die, &grid) in &self.grids {
            if grid.has_no_tbuturn {
                let (w, _) = self
                    .egrid
                    .db
                    .wires
                    .iter()
                    .find(|(_, w)| w.name == "LVB.6")
                    .unwrap();
                for col in grid.columns.ids() {
                    for i in 0..6 {
                        let row = RowId::from_idx(i);
                        self.egrid.blackhole_wires.insert((die, (col, row), w));
                    }
                    for i in 0..6 {
                        let row = RowId::from_idx(grid.regs * 50 - 6 + i);
                        self.egrid.blackhole_wires.insert((die, (col, row), w));
                    }
                }
            }
        }
    }

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
        self.egrid.cursed_wires = cursed_wires;
    }
}
