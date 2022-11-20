#![allow(clippy::comparison_chain)]

use prjcombine_entity::{entity_id, EntityId, EntityIds, EntityPartVec, EntityVec};
use prjcombine_int::grid::{ColId, ExpandedGrid, RowId};
use prjcombine_virtex_bitstream::BitstreamGeom;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod expand;
pub mod io;

entity_id! {
    pub id RegId u32, delta;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub columns: EntityVec<ColId, ColumnKind>,
    pub cols_vbrk: BTreeSet<ColId>,
    pub cols_mgt_buf: BTreeSet<ColId>,
    pub col_cfg: ColId,
    pub cols_qbuf: (ColId, ColId),
    pub cols_io: [Option<ColId>; 4],
    pub col_hard: Option<HardColumn>,
    pub regs: usize,
    pub reg_gth_start: RegId,
    pub reg_cfg: RegId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKind {
    ClbLL,
    ClbLM,
    Bram,
    Dsp,
    Io,
    Gt,
    Cmt,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HardColumn {
    pub col: ColId,
    pub rows_emac: Vec<RowId>,
    pub rows_pcie: Vec<RowId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DisabledPart {
    Emac(RowId),
    GtxRow(RegId),
    SysMon,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CfgPin {
    Tck,
    Tdi,
    Tdo,
    Tms,
    Cclk,
    Done,
    ProgB,
    InitB,
    RdWrB,
    CsiB,
    Din,
    Dout,
    M0,
    M1,
    M2,
    HswapEn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtPin {
    RxP(u8),
    RxN(u8),
    TxP(u8),
    TxN(u8),
    ClkP(u8),
    ClkN(u8),
    AVttRCal,
    RRef,
    RBias,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtxRegionPin {
    AVtt,
    AVcc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GthRegionPin {
    AVtt,
    AGnd,
    AVcc,
    AVccRx,
    AVccPll,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SysMonPin {
    VP,
    VN,
    AVss,
    AVdd,
    VRefP,
    VRefN,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BondPin {
    // bank, pin within bank
    Io(u32, u32),
    Nc,
    Gnd,
    Rsvd,
    VccInt,
    VccAux,
    VccO(u32),
    VccBatt,
    Cfg(CfgPin),
    Gt(u32, GtPin),
    GtxRegion(GtRegion, GtxRegionPin),
    GthRegion(GtRegion, GthRegionPin),
    Dxp,
    Dxn,
    Vfs,
    SysMon(SysMonPin),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    // ×32; high 16 bits are also low 16 bits of Addr
    // 0-2 double as FS
    Data(u8),
    Addr(u8), // ×26 total, but 0-15 are represented as Data(16-31)
    Rs(u8),   // ×2
    CsoB,
    FweB,
    FoeB, // doubles as MOSI
    FcsB,
}

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub disabled: &'a BTreeSet<DisabledPart>,
    pub egrid: ExpandedGrid<'a>,
    pub bs_geom: BitstreamGeom,
    pub col_frame: EntityVec<RegId, EntityVec<ColId, usize>>,
    pub bram_frame: EntityVec<RegId, EntityPartVec<ColId, usize>>,
}

impl Grid {
    pub fn row_to_reg(&self, row: RowId) -> RegId {
        RegId::from_idx(row.to_idx() / 40)
    }

    pub fn row_reg_bot(&self, reg: RegId) -> RowId {
        RowId::from_idx(reg.to_idx() * 40)
    }

    pub fn row_reg_hclk(&self, reg: RegId) -> RowId {
        RowId::from_idx(reg.to_idx() * 40 + 20)
    }

    pub fn row_hclk(&self, row: RowId) -> RowId {
        RowId::from_idx(row.to_idx() / 40 * 40 + 20)
    }

    pub fn regs(&self) -> EntityIds<RegId> {
        EntityIds::new(self.regs)
    }

    pub fn row_bufg(&self) -> RowId {
        self.row_reg_bot(self.reg_cfg)
    }

    pub fn has_gt(&self) -> bool {
        self.columns.values().any(|&x| x == ColumnKind::Gt)
    }

    pub fn has_left_gt(&self) -> bool {
        *self.columns.first().unwrap() == ColumnKind::Gt
    }
}
