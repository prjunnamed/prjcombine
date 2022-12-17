use enum_map::Enum;
use prjcombine_entity::{entity_id, EntityId, EntityIds, EntityVec};
use prjcombine_int::grid::{ColId, DieId, RowId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

entity_id! {
    pub id RegId u32, delta;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub columns: EntityVec<ColId, Column>,
    pub cols_vbrk: BTreeSet<ColId>,
    pub cols_cpipe: BTreeSet<ColId>,
    pub cols_hard: Vec<HardColumn>,
    pub col_cfrm: ColId,
    pub regs: usize,
    pub regs_gt_left: EntityVec<RegId, GtRowKind>,
    pub regs_gt_right: Option<EntityVec<RegId, GtRowKind>>,
    pub ps: PsKind,
    pub cpm: CpmKind,
    pub has_hnicx: bool,
    pub top: TopKind,
    pub bottom: BotKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub l: ColumnKind,
    pub r: ColumnKind,
    pub has_bli_bot_l: bool,
    pub has_bli_top_l: bool,
    pub has_bli_bot_r: bool,
    pub has_bli_top_r: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKind {
    Cle,
    CleLaguna,
    Bram,
    BramClkBuf,
    Uram,
    Dsp,
    Hard,
    Gt,
    Cfrm,
    VNoc,
    VNoc2,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, Enum)]
pub enum ColSide {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PsKind {
    Ps9,
    PsX,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CpmKind {
    None,
    Cpm4,
    Cpm5,
    Cpm5N,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, Enum)]
pub enum HardRowKind {
    None,
    Hdio,
    Pcie4,
    Pcie5,
    Mrmac,
    IlknB,
    IlknT,
    DcmacB,
    DcmacT,
    HscB,
    HscT,
    CpmExt,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HardColumn {
    pub col: ColId,
    pub regs: EntityVec<RegId, HardRowKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum GtRowKind {
    None,
    Gty,
    Gtyp,
    Gtm,
    Xram,
    Vdu,
    BfrB,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum BotKind {
    Xpio(usize),
    Ssit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum TopKind {
    Xpio(usize),
    Ssit,
    Me,
    Ai(usize, usize),
    AiMl(usize, usize, usize),
    Hbm,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum NocEndpoint {
    // tile idx, switch idx, port idx
    BotNps(usize, usize, usize),
    TopNps(usize, usize, usize),
    Ncrb(usize, usize, usize),
    // column, region, switch idx, port idx
    VNocNps(ColId, usize, usize, usize),
    VNocEnd(ColId, usize, usize),
    Pmc(usize),
    Me(usize, usize),
    // tile idx, port idx
    BotDmc(usize, usize),
    TopDmc(usize, usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DisabledPart {
    HardIp(DieId, ColId, RegId),
    HardIpSite(DieId, ColId, RegId),
    HdioDpll(DieId, ColId, RegId),
    Column(DieId, ColId),
    GtRight(DieId, RegId),
    Region(DieId, RegId),
}

impl Grid {
    pub fn row_to_reg(&self, row: RowId) -> RegId {
        RegId::from_idx(row.to_idx() / 48)
    }

    pub fn row_reg_bot(&self, reg: RegId) -> RowId {
        RowId::from_idx(reg.to_idx() * 48)
    }

    pub fn is_reg_top(&self, reg: RegId) -> bool {
        reg.to_idx() == self.regs - 1 || reg.to_idx() % 2 == 1
    }

    pub fn regs(&self) -> EntityIds<RegId> {
        EntityIds::new(self.regs)
    }

    pub fn get_col_hard(&self, col: ColId) -> Option<&HardColumn> {
        self.cols_hard.iter().find(|x| x.col == col)
    }
}
