use bimap::BiHashMap;
use enum_map::EnumMap;
use prjcombine_int::grid::{ColId, DieId, ExpandedGrid, RowId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::grid::{
    ColSide, ColumnKindRight, DisabledPart, Grid, GridKind, HardRowKind, HdioIobId, HpioIobId, Interposer, IoRowKind, RegId
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
pub struct GtCoord {
    pub die: DieId,
    pub col: ColId,
    pub side: ColSide,
    pub reg: RegId,
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

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct IoInfo {
    pub kind: IoKind,
    pub bank: u32,
    pub diff: IoDiffKind,
    pub is_vrp: bool,
    pub is_qbc: bool,
    pub is_dbc: bool,
    pub is_gc: bool,
    pub sm_pair: Option<u32>,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct GtInfo {
    pub kind: IoRowKind,
    pub bank: u32,
}

pub struct ExpandedDevice<'a> {
    pub kind: GridKind,
    pub grids: EntityVec<DieId, &'a Grid>,
    pub interposer: &'a Interposer,
    pub egrid: ExpandedGrid<'a>,
    pub disabled: BTreeSet<DisabledPart>,
    pub hdistr_src: EntityVec<ColId, EnumMap<ColSide, ClkSrc>>,
    pub hroute_src: EntityVec<ColId, EnumMap<ColSide, ClkSrc>>,
    pub has_pcie_cfg: bool,
    pub is_cut: bool,
    pub is_cut_d: bool,
    pub io: Vec<IoCoord>,
    pub cfg_io: EntityVec<DieId, BiHashMap<SharedCfgPin, IoCoord>>,
    pub gt: Vec<GtCoord>,
    pub col_cfg_io: (ColId, ColSide),
    pub bankxlut: EntityPartVec<ColId, u32>,
    pub bankylut: EntityVec<DieId, u32>,
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

    pub fn get_io_info(&self, io: IoCoord) -> IoInfo {
        match io {
            IoCoord::Hpio(hpio) => {
                let grid = self.grids[hpio.die];
                let iocol = grid
                    .cols_io
                    .iter()
                    .find(|iocol| iocol.col == hpio.col)
                    .unwrap();
                let kind = match iocol.regs[hpio.reg] {
                    IoRowKind::Hpio => IoKind::Hpio,
                    IoRowKind::Hrio => IoKind::Hrio,
                    _ => unreachable!(),
                };
                let x = self.bankxlut[hpio.col];
                let y = self.bankylut[hpio.die] + hpio.reg.to_idx() as u32;
                let mut bank = x + y;
                let idx = hpio.iob.to_idx();
                if bank == 64 && kind == IoKind::Hrio {
                    if idx < 26 {
                        bank = 94;
                    } else {
                        bank = 84;
                    }
                }
                IoInfo {
                    kind,
                    bank,
                    diff: if idx % 13 == 12 {
                        IoDiffKind::None
                    } else if idx % 13 % 2 == 0 {
                        IoDiffKind::P(IoCoord::Hpio(HpioCoord {
                            iob: HpioIobId::from_idx(idx + 1),
                            ..hpio
                        }))
                    } else {
                        IoDiffKind::N(IoCoord::Hpio(HpioCoord {
                            iob: HpioIobId::from_idx(idx - 1),
                            ..hpio
                        }))
                    },
                    is_vrp: kind == IoKind::Hpio && idx == 12,
                    is_gc: matches!(idx, 21 | 22 | 23 | 24 | 26 | 27 | 28 | 29),
                    is_dbc: matches!(idx, 0 | 1 | 6 | 7 | 39 | 40 | 45 | 46),
                    is_qbc: matches!(idx, 13 | 14 | 19 | 20 | 26 | 27 | 32 | 33),
                    sm_pair: match idx {
                        4 | 5 => Some(15),
                        6 | 7 => Some(7),
                        8 | 9 => Some(14),
                        10 | 11 => Some(6),
                        13 | 14 => Some(13),
                        15 | 16 => Some(5),
                        17 | 18 => Some(12),
                        19 | 20 => Some(4),
                        30 | 31 => Some(11),
                        32 | 33 => Some(3),
                        34 | 35 => Some(10),
                        36 | 37 => Some(2),
                        39 | 40 => Some(9),
                        41 | 42 => Some(1),
                        43 | 44 => Some(8),
                        45 | 46 => Some(0),
                        _ => None,
                    },
                }
            }
            IoCoord::Hdio(hdio) => {
                let grid = self.grids[hdio.die];
                let hcol = grid
                    .cols_hard
                    .iter()
                    .find(|hcol| hcol.col == hdio.col + 1)
                    .unwrap();
                let kind = hcol.regs[hdio.reg];
                let x = self.bankxlut[hdio.col];
                let y = self.bankylut[hdio.die] + hdio.reg.to_idx() as u32;
                let bank = x + y;
                IoInfo {
                    kind: IoKind::Hdio,
                    bank,
                    diff: if hdio.iob.to_idx() % 2 == 0 {
                        IoDiffKind::P(IoCoord::Hdio(HdioCoord {
                            iob: HdioIobId::from_idx(hdio.iob.to_idx() ^ 1),
                            ..hdio
                        }))
                    } else {
                        IoDiffKind::N(IoCoord::Hdio(HdioCoord {
                            iob: HdioIobId::from_idx(hdio.iob.to_idx() ^ 1),
                            ..hdio
                        }))
                    },
                    is_vrp: false,
                    is_qbc: false,
                    is_dbc: false,
                    is_gc: (8..16).contains(&hdio.iob.to_idx()),
                    sm_pair: match (kind, hdio.iob.to_idx()) {
                        (HardRowKind::HdioAms, 0 | 1) => Some(11),
                        (HardRowKind::HdioAms, 2 | 3) => Some(10),
                        (HardRowKind::HdioAms, 4 | 5) => Some(9),
                        (HardRowKind::HdioAms, 6 | 7) => Some(8),
                        (HardRowKind::HdioAms, 8 | 9) => Some(7),
                        (HardRowKind::HdioAms, 10 | 11) => Some(6),
                        (HardRowKind::HdioAms, 12 | 13) => Some(5),
                        (HardRowKind::HdioAms, 14 | 15) => Some(4),
                        (HardRowKind::HdioAms, 16 | 17) => Some(3),
                        (HardRowKind::HdioAms, 18 | 19) => Some(2),
                        (HardRowKind::HdioAms, 20 | 21) => Some(1),
                        (HardRowKind::HdioAms, 22 | 23) => Some(0),
                        (HardRowKind::Hdio, 0 | 1) => Some(15),
                        (HardRowKind::Hdio, 2 | 3) => Some(14),
                        (HardRowKind::Hdio, 4 | 5) => Some(13),
                        (HardRowKind::Hdio, 6 | 7) => Some(12),
                        (HardRowKind::Hdio, 16 | 17) => Some(11),
                        (HardRowKind::Hdio, 18 | 19) => Some(10),
                        (HardRowKind::Hdio, 20 | 21) => Some(9),
                        (HardRowKind::Hdio, 22 | 23) => Some(8),
                        _ => None,
                    },
                }
            }
        }
    }

    pub fn get_gt_info(&self, gt: GtCoord) -> GtInfo {
        let grid = self.grids[gt.die];
        let iocol = grid
            .cols_io
            .iter()
            .find(|iocol| iocol.col == gt.col)
            .unwrap();
        let kind = iocol.regs[gt.reg];
        let x = if gt.col.to_idx() == 0 { 100 } else { 200 };
        let y = self.bankylut[gt.die] + gt.reg.to_idx() as u32;
        let bank = x + y;
        GtInfo { kind, bank }
    }
}
