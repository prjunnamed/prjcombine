use std::collections::BTreeSet, BTreeMap;
use serde::{Serialize, Deserialize};
use crate::DisabledPart;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub columns: Vec<ColumnKind>,
    pub cols_bli_bot: BTreeMap<usize, BliKind>,
    pub cols_bli_top: BTreeMap<usize, BliKind>,
    pub cols_vbrk: BTreeSet<u32>,
    pub cols_cpipe: BTreeSet<u32>,
    pub cols_hard: Vec<HardColumn>,
    pub cols_gt: Vec<GtColumn>,
    pub col_cfrm: u32,
    pub rows: u32,
    pub rows_gt_left: Vec<GtRowKind>,
    pub rows_gt_right: Vec<GtRowKind>,
    pub cpm: CpmKind,
    pub top: TopKind,
    pub bottom: BotKind,
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
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HardColumn {
    pub col: u32,
    pub rows: Vec<HardRowKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum GtRowKind {
    None,
    Gty,
    Gtyp,
    Gtm,
    Xram,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Bli {
    Unknown,
    // bank idx, nibble idx
    XpioNibble(u32, u32),
}

pub enum BotKind {
    Xpio(u32),
    Ssit,
}

pub enum TopKind {
    Xpio(u32),
    Ssit,
    Ai(u32, u32),
    AiMl(u32, u32),
    Hbm,
}
