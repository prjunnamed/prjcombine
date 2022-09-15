use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::grid::{ColId, ExpandedGrid, RowId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod expand;
pub mod io;

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
    pub reg_cfg: usize,
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
}

impl Grid {
    pub fn row_hclk(&self, row: RowId) -> RowId {
        RowId::from_idx(row.to_idx() / 16 * 16 + 8)
    }

    pub fn row_dcmiob(&self) -> RowId {
        RowId::from_idx(self.reg_cfg * 16 - self.regs_cfg_io * 16 - 8)
    }

    pub fn row_iobdcm(&self) -> RowId {
        RowId::from_idx(self.reg_cfg * 16 + self.regs_cfg_io * 16 + 8)
    }

    pub fn row_cfg_below(&self) -> RowId {
        RowId::from_idx(self.reg_cfg * 16 - 8)
    }

    pub fn row_cfg_above(&self) -> RowId {
        RowId::from_idx(self.reg_cfg * 16 + 8)
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
