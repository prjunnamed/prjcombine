use bimap::BiHashMap;
use enum_map::EnumMap;
use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::grid::{ColId, DieId, ExpandedGrid, RowId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::grid::{
    ColSide, ColumnKindRight, DeviceNaming, DisabledPart, Grid, GridKind, HdioIobId, HpioIobId,
    IoRowKind, RegId,
};

use crate::bond::SharedCfgPin;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ClkSrc {
    DspSplitter(ColId),
    Gt(ColId),
    Cmt(ColId),
    RouteSplitter(ColId),
    RightHdio(ColId),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct HpioCoord {
    pub die: DieId,
    pub col: ColId,
    pub side: ColSide,
    pub reg: RegId,
    pub iob: HpioIobId,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct HdioCoord {
    pub die: DieId,
    pub col: ColId,
    pub reg: RegId,
    pub iob: HdioIobId,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum IoCoord {
    Hpio(HpioCoord),
    Hdio(HdioCoord),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum IoKind {
    Hpio,
    Hrio,
    Hdio,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum IoDiffKind {
    None,
    P(IoCoord),
    N(IoCoord),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Io {
    pub kind: IoKind,
    pub crd: IoCoord,
    pub name: String,
    pub bank: u32,
    pub diff: IoDiffKind,
    pub is_vrp: bool,
    pub is_qbc: bool,
    pub is_dbc: bool,
    pub is_gc: bool,
    pub sm_pair: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Gt {
    pub die: DieId,
    pub col: ColId,
    pub side: ColSide,
    pub reg: RegId,
    pub bank: u32,
    pub kind: IoRowKind,
    pub name_common: String,
    pub name_channel: Vec<String>,
}

pub struct ExpandedDevice<'a> {
    pub kind: GridKind,
    pub grids: EntityVec<DieId, &'a Grid>,
    pub grid_master: DieId,
    pub egrid: ExpandedGrid<'a>,
    pub disabled: BTreeSet<DisabledPart>,
    pub naming: &'a DeviceNaming,
    pub hdistr_src: EntityVec<ColId, EnumMap<ColSide, ClkSrc>>,
    pub hroute_src: EntityVec<ColId, EnumMap<ColSide, ClkSrc>>,
    pub has_pcie_cfg: bool,
    pub is_cut: bool,
    pub is_cut_d: bool,
    pub io: Vec<Io>,
    pub cfg_io: EntityVec<DieId, BiHashMap<SharedCfgPin, IoCoord>>,
    pub gt: Vec<Gt>,
}

impl ExpandedDevice<'_> {
    pub fn in_site_hole(&self, die: DieId, col: ColId, row: RowId, side: ColSide) -> bool {
        if let Some(ps) = self.grids[die].ps {
            if row.to_idx() < ps.height() {
                if col < ps.col {
                    return true;
                }
                if col == ps.col && side == ColSide::Left {
                    return true;
                }
            }
        }
        if self.grids[die].has_hbm
            && side == ColSide::Right
            && matches!(self.grids[die].columns[col].r, ColumnKindRight::Dsp(_))
            && row.to_idx() < 15
        {
            return true;
        }
        false
    }
}
