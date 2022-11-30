use prjcombine_entity::{entity_id, EntityId, EntityIds, EntityVec};
use prjcombine_int::grid::{ColId, RowId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

entity_id! {
    pub id RegId u32, delta;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub kind: GridKind,
    pub columns: EntityVec<ColId, ColumnKind>,
    pub cols_vbrk: BTreeSet<ColId>,
    pub cols_mgt_buf: BTreeSet<ColId>,
    pub cols_qbuf: Option<(ColId, ColId)>,
    pub col_hard: Option<HardColumn>,
    pub cols_io: Vec<IoColumn>,
    pub cols_gt: Vec<GtColumn>,
    pub regs: usize,
    pub reg_cfg: RegId,
    pub reg_clk: RegId,
    pub rows_cfg: Vec<(RowId, CfgRowKind)>,
    pub holes_ppc: Vec<(ColId, RowId)>,
    pub holes_pcie2: Vec<Pcie2>,
    pub holes_pcie3: Vec<(ColId, RowId)>,
    pub has_bram_fx: bool,
    pub has_ps: bool,
    pub has_slr: bool,
    pub has_no_tbuturn: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GridKind {
    Virtex4,
    Virtex5,
    Virtex6,
    Virtex7,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKind {
    ClbLL,
    ClbLM,
    Bram,
    Dsp,
    Io,
    Cfg,
    Gt,
    Cmt,
    Clk,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CfgRowKind {
    Dcm,
    Ccm,
    Sysmon,
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IoColumn {
    pub col: ColId,
    pub regs: EntityVec<RegId, Option<IoKind>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GtColumn {
    pub col: ColId,
    pub regs: EntityVec<RegId, Option<GtKind>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HardColumn {
    pub col: ColId,
    pub rows_emac: Vec<RowId>,
    pub rows_pcie: Vec<RowId>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DisabledPart {
    Emac(RowId),
    GtxRow(RegId),
    SysMon,
    Gtp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ExtraDie {
    Gtz(GtzLoc),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GtzLoc {
    Top,
    Bottom,
}

impl Grid {
    #[inline]
    pub fn rows_per_reg(&self) -> usize {
        match self.kind {
            GridKind::Virtex4 => 16,
            GridKind::Virtex5 => 20,
            GridKind::Virtex6 => 40,
            GridKind::Virtex7 => 50,
        }
    }

    #[inline]
    pub fn row_to_reg(&self, row: RowId) -> RegId {
        RegId::from_idx(row.to_idx() / self.rows_per_reg())
    }

    #[inline]
    pub fn row_reg_bot(&self, reg: RegId) -> RowId {
        RowId::from_idx(reg.to_idx() * self.rows_per_reg())
    }

    #[inline]
    pub fn row_reg_hclk(&self, reg: RegId) -> RowId {
        self.row_reg_bot(reg) + self.rows_per_reg() / 2
    }

    #[inline]
    pub fn row_hclk(&self, row: RowId) -> RowId {
        self.row_reg_hclk(self.row_to_reg(row))
    }

    pub fn regs(&self) -> EntityIds<RegId> {
        EntityIds::new(self.regs)
    }

    pub fn row_bufg(&self) -> RowId {
        self.row_reg_bot(self.reg_clk)
    }

    pub fn get_col_io(&self, col: ColId) -> Option<&IoColumn> {
        self.cols_io.iter().find(|ioc| ioc.col == col)
    }

    pub fn get_col_gt(&self, col: ColId) -> Option<&GtColumn> {
        self.cols_gt.iter().find(|gtc| gtc.col == col)
    }

    pub fn col_ps(&self) -> ColId {
        assert!(self.has_ps);
        ColId::from_idx(18)
    }
}
