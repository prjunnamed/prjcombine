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
    pub col_hard: Option<HardColumn>,
    pub cols_io: [Option<ColId>; 3],
    pub regs: usize,
    pub reg_cfg: RegId,
    pub holes_ppc: Vec<(ColId, RowId)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKind {
    ClbLL,
    ClbLM,
    Bram,
    Dsp,
    Io,
    Gtp,
    Gtx,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HardColumn {
    pub col: ColId,
    pub rows_emac: Vec<RowId>,
    pub rows_pcie: Vec<RowId>,
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
pub enum SysMonPin {
    VP,
    VN,
    AVss,
    AVdd,
    VRefP,
    VRefN,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtPin {
    RxP(u8),
    RxN(u8),
    TxP(u8),
    TxN(u8),
    ClkP,
    ClkN,
    AVcc,
    AVccPll,
    VtRx,
    VtTx,
    RRef,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtRegion {
    All,
    L,
    R,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtRegionPin {
    AVttRxC,
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
    GtRegion(GtRegion, GtRegionPin),
    Dxp,
    Dxn,
    SysMon(SysMonPin),
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
        RegId::from_idx(row.to_idx() / 20)
    }

    pub fn row_reg_bot(&self, reg: RegId) -> RowId {
        RowId::from_idx(reg.to_idx() * 20)
    }

    pub fn row_reg_hclk(&self, reg: RegId) -> RowId {
        RowId::from_idx(reg.to_idx() * 20 + 10)
    }

    pub fn row_hclk(&self, row: RowId) -> RowId {
        RowId::from_idx(row.to_idx() / 20 * 20 + 10)
    }

    pub fn regs(&self) -> EntityIds<RegId> {
        EntityIds::new(self.regs)
    }

    pub fn row_botcen(&self) -> RowId {
        self.row_reg_hclk(self.reg_cfg - 1)
    }

    pub fn row_topcen(&self) -> RowId {
        self.row_reg_hclk(self.reg_cfg)
    }

    pub fn row_ioi_cmt(&self) -> RowId {
        if self.reg_cfg.to_idx() == 1 {
            RowId::from_idx(0)
        } else {
            self.row_reg_hclk(self.reg_cfg - 2)
        }
    }

    pub fn row_cmt_ioi(&self) -> RowId {
        if self.reg_cfg.to_idx() == self.regs - 1 {
            RowId::from_idx(self.regs * 20)
        } else {
            self.row_reg_hclk(self.reg_cfg + 1)
        }
    }

    pub fn row_bot_cmt(&self) -> RowId {
        if self.reg_cfg.to_idx() < 3 {
            RowId::from_idx(0)
        } else {
            self.row_reg_bot(self.reg_cfg - 3)
        }
    }

    pub fn row_top_cmt(&self) -> RowId {
        if (self.regs - self.reg_cfg.to_idx()) < 3 {
            RowId::from_idx(self.regs * 20)
        } else {
            self.row_reg_bot(self.reg_cfg + 3)
        }
    }

    pub fn has_left_gt(&self) -> bool {
        *self.columns.first().unwrap() == ColumnKind::Gtx
    }

    pub fn has_gt(&self) -> bool {
        matches!(
            *self.columns.last().unwrap(),
            ColumnKind::Gtx | ColumnKind::Gtp
        )
    }
}
