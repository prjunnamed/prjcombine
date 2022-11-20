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
    pub cols_io: [ColId; 3],
    pub regs: usize,
    pub has_bot_sysmon: bool,
    pub has_top_sysmon: bool,
    pub regs_cfg_io: usize,
    pub ccm: usize,
    pub reg_cfg: RegId,
    pub holes_ppc: Vec<(ColId, RowId)>,
    pub has_bram_fx: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKind {
    Clb,
    Bram,
    Dsp,
    Io,
    Gt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    Data(u8), // ×32
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
    PwrdwnB,
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
    ClkP,
    ClkN,
    GndA,
    VtRx(u8),
    VtTx(u8),
    AVccAuxRx(u8),
    AVccAuxTx,
    AVccAuxMgt,
    RTerm,
    MgtVRef,
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
    VccInt,
    VccAux,
    VccO(u32),
    VccBatt,
    Cfg(CfgPin),
    Gt(u32, GtPin),
    Dxp,
    Dxn,
    SysMon(u32, SysMonPin),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
}

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub egrid: ExpandedGrid<'a>,
    pub bs_geom: BitstreamGeom,
    pub col_frame: EntityVec<RegId, EntityVec<ColId, usize>>,
    pub bram_frame: EntityVec<RegId, EntityPartVec<ColId, usize>>,
    pub spine_frame: EntityVec<RegId, usize>,
}

impl Grid {
    pub fn row_to_reg(&self, row: RowId) -> RegId {
        RegId::from_idx(row.to_idx() / 16)
    }

    pub fn row_reg_bot(&self, reg: RegId) -> RowId {
        RowId::from_idx(reg.to_idx() * 16)
    }

    pub fn row_reg_hclk(&self, reg: RegId) -> RowId {
        RowId::from_idx(reg.to_idx() * 16 + 8)
    }

    pub fn row_hclk(&self, row: RowId) -> RowId {
        RowId::from_idx(row.to_idx() / 16 * 16 + 8)
    }

    pub fn regs(&self) -> EntityIds<RegId> {
        EntityIds::new(self.regs)
    }

    pub fn row_dcmiob(&self) -> RowId {
        self.row_reg_hclk(self.reg_cfg - self.regs_cfg_io - 1)
    }

    pub fn row_iobdcm(&self) -> RowId {
        self.row_reg_hclk(self.reg_cfg + self.regs_cfg_io)
    }

    pub fn row_cfg_below(&self) -> RowId {
        self.row_reg_hclk(self.reg_cfg - 1)
    }

    pub fn row_cfg_above(&self) -> RowId {
        self.row_reg_hclk(self.reg_cfg)
    }

    pub fn col_lgt(&self) -> ColId {
        self.columns.first_id().unwrap()
    }

    pub fn col_rgt(&self) -> ColId {
        self.columns.last_id().unwrap()
    }

    pub fn has_mgt(&self) -> bool {
        *self.columns.first().unwrap() == ColumnKind::Gt
    }
}
