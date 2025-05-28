use bimap::BiHashMap;
use prjcombine_interconnect::db::RegionSlotId;
use prjcombine_interconnect::grid::{ColId, DieId, ExpandedGrid, RowId, TileIobId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::chip::{Chip, ChipKind, DisabledPart, HardRowKind, Interposer, IoRowKind, RegId};

use crate::bond::SharedCfgPin;

pub const REGION_LEAF: RegionSlotId = RegionSlotId::from_idx_const(0);

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
    pub reg: RegId,
    pub iob: TileIobId,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct HdioCoord {
    pub die: DieId,
    pub col: ColId,
    pub reg: RegId,
    pub iob: TileIobId,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum IoCoord {
    Hpio(HpioCoord),
    Hdio(HdioCoord),
    HdioLc(HdioCoord),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct GtCoord {
    pub die: DieId,
    pub col: ColId,
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
    pub kind: ChipKind,
    pub chips: EntityVec<DieId, &'a Chip>,
    pub interposer: &'a Interposer,
    pub egrid: ExpandedGrid<'a>,
    pub disabled: BTreeSet<DisabledPart>,
    pub hdistr_src: EntityVec<ColId, ClkSrc>,
    pub hroute_src: EntityVec<ColId, ClkSrc>,
    pub has_pcie_cfg: bool,
    pub is_cut: bool,
    pub is_cut_d: bool,
    pub io: Vec<IoCoord>,
    pub cfg_io: EntityVec<DieId, BiHashMap<SharedCfgPin, IoCoord>>,
    pub gt: Vec<GtCoord>,
    pub col_cfg_io: ColId,
    pub bankxlut: EntityPartVec<ColId, u32>,
    pub bankylut: EntityVec<DieId, EntityPartVec<RegId, u32>>,
}

impl ExpandedDevice<'_> {
    pub fn in_site_hole(&self, die: DieId, col: ColId, row: RowId) -> bool {
        self.chips[die].in_site_hole(col, row)
    }

    pub fn get_io_info(&self, io: IoCoord) -> IoInfo {
        match io {
            IoCoord::Hpio(hpio) => {
                let chip = self.chips[hpio.die];
                let iocol = chip
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
                let y = self.bankylut[hpio.die][hpio.reg];
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
                            iob: TileIobId::from_idx(idx + 1),
                            ..hpio
                        }))
                    } else {
                        IoDiffKind::N(IoCoord::Hpio(HpioCoord {
                            iob: TileIobId::from_idx(idx - 1),
                            ..hpio
                        }))
                    },
                    is_vrp: kind == IoKind::Hpio && idx == 12,
                    is_gc: matches!(idx, 21 | 22 | 23 | 24 | 26 | 27 | 28 | 29),
                    is_dbc: matches!(idx, 0 | 1 | 6 | 7 | 39 | 40 | 45 | 46),
                    is_qbc: matches!(idx, 13 | 14 | 19 | 20 | 26 | 27 | 32 | 33),
                    sm_pair: if chip.has_csec {
                        None
                    } else {
                        match idx {
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
                        }
                    },
                }
            }
            IoCoord::Hdio(hdio) => {
                let chip = self.chips[hdio.die];
                let hcol = chip
                    .cols_hard
                    .iter()
                    .find(|hcol| hcol.col == hdio.col)
                    .unwrap();
                let kind = hcol.regs[hdio.reg];
                let x = self.bankxlut[hdio.col];
                let y = self.bankylut[hdio.die][hdio.reg];
                let bank = x + y;
                IoInfo {
                    kind: IoKind::Hdio,
                    bank,
                    diff: if hdio.iob.to_idx() % 2 == 0 {
                        IoDiffKind::P(IoCoord::Hdio(HdioCoord {
                            iob: TileIobId::from_idx(hdio.iob.to_idx() ^ 1),
                            ..hdio
                        }))
                    } else {
                        IoDiffKind::N(IoCoord::Hdio(HdioCoord {
                            iob: TileIobId::from_idx(hdio.iob.to_idx() ^ 1),
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
            IoCoord::HdioLc(hdio) => {
                let chip = self.chips[hdio.die];
                let x = self.bankxlut[hdio.col];
                let y =
                    self.bankylut[hdio.die][hdio.reg] + if hdio.iob.to_idx() >= 42 { 1 } else { 0 };
                let bank = x + y;
                let is_ams = hdio.reg == chip.reg_cfg() + 1;
                IoInfo {
                    kind: IoKind::Hdio,
                    bank,
                    diff: if hdio.iob.to_idx() % 2 == 0 {
                        IoDiffKind::P(IoCoord::Hdio(HdioCoord {
                            iob: TileIobId::from_idx(hdio.iob.to_idx() ^ 1),
                            ..hdio
                        }))
                    } else {
                        IoDiffKind::N(IoCoord::Hdio(HdioCoord {
                            iob: TileIobId::from_idx(hdio.iob.to_idx() ^ 1),
                            ..hdio
                        }))
                    },
                    is_vrp: false,
                    is_qbc: false,
                    is_dbc: false,
                    is_gc: matches!(&hdio.iob.to_idx(), 10..14 | 42..46),
                    sm_pair: if !is_ams {
                        None
                    } else {
                        match hdio.iob.to_idx() {
                            14 | 15 => Some(15),
                            16 | 17 => Some(13),
                            18 | 19 => Some(12),
                            24 | 25 => Some(9),
                            30 | 31 => Some(14),
                            36 | 37 => Some(11),
                            38 | 39 => Some(10),
                            40 | 41 => Some(8),
                            58 | 59 => Some(5),
                            64 | 65 => Some(3),
                            66 | 67 => Some(1),
                            68 | 69 => Some(0),
                            70 | 71 => Some(7),
                            72 | 73 => Some(6),
                            74 | 75 => Some(4),
                            80 | 81 => Some(2),
                            _ => None,
                        }
                    },
                }
            }
        }
    }

    pub fn get_gt_info(&self, gt: GtCoord) -> GtInfo {
        let chip = self.chips[gt.die];
        let iocol = chip
            .cols_io
            .iter()
            .find(|iocol| iocol.col == gt.col)
            .unwrap();
        let kind = iocol.regs[gt.reg];
        let x = if gt.col.to_idx() == 0 { 100 } else { 200 };
        let y = self.bankylut[gt.die][gt.reg];
        let bank = x + y;
        GtInfo { kind, bank }
    }

    pub fn is_hdiolc(&self, crd: HdioCoord) -> bool {
        let chip = self.chips[crd.die];
        if chip.cols_io.iter().any(|iocol| iocol.col == crd.col) {
            true
        } else {
            let hcol = chip
                .cols_hard
                .iter()
                .find(|hcol| hcol.col == crd.col)
                .unwrap();
            hcol.regs[crd.reg] == HardRowKind::HdioLc
        }
    }
}
