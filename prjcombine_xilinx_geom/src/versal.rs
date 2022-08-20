use std::collections::BTreeSet;
use serde::{Serialize, Deserialize};
use crate::ColId;
use prjcombine_entity::EntityVec;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub columns: EntityVec<ColId, Column>,
    pub cols_vbrk: BTreeSet<ColId>,
    pub cols_cpipe: BTreeSet<ColId>,
    pub cols_hard: [Option<HardColumn>; 3],
    pub col_cfrm: ColId,
    pub regs: usize,
    pub regs_gt_left: Vec<GtRowKind>,
    pub regs_gt_right: Option<Vec<GtRowKind>>,
    pub cpm: CpmKind,
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
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CpmKind {
    None,
    Cpm4,
    Cpm5,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
    pub regs: Vec<HardRowKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum GtRowKind {
    None,
    Gty,
    Gtyp,
    Gtm,
    Xram,
    Vdu,
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
