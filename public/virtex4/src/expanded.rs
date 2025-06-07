use crate::bond::{PsPad, SharedCfgPad};
use crate::chip::{Chip, ChipKind, DisabledPart, GtKind, Interposer, IoKind, RegId, XadcIoLoc};
use crate::gtz::{GtzBelId, GtzDb, GtzIntColId, GtzIntRowId};
use bimap::BiHashMap;
use prjcombine_interconnect::db::RegionSlotId;
use prjcombine_interconnect::dir::DirPartMap;
use prjcombine_interconnect::grid::{ColId, DieId, ExpandedGrid, NodeLoc, Rect, RowId, TileIobId};
use prjcombine_xilinx_bitstream::{BitTile, BitstreamGeom};
use std::collections::{BTreeSet, HashSet};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

pub const REGION_HCLK: RegionSlotId = RegionSlotId::from_idx_const(0);
pub const REGION_LEAF: RegionSlotId = RegionSlotId::from_idx_const(1);

#[derive(Clone, Debug)]
pub struct DieFrameGeom {
    pub col_frame: EntityVec<RegId, EntityVec<ColId, usize>>,
    pub col_width: EntityVec<RegId, EntityVec<ColId, usize>>,
    pub bram_frame: EntityVec<RegId, EntityPartVec<ColId, usize>>,
    pub spine_frame: EntityVec<RegId, usize>,
}

pub struct ExpandedDevice<'a> {
    pub kind: ChipKind,
    pub chips: EntityVec<DieId, &'a Chip>,
    pub egrid: ExpandedGrid<'a>,
    pub gdb: &'a GtzDb,
    pub disabled: BTreeSet<DisabledPart>,
    pub interposer: Option<&'a Interposer>,
    pub int_holes: EntityVec<DieId, Vec<Rect>>,
    pub site_holes: EntityVec<DieId, Vec<Rect>>,
    pub bs_geom: BitstreamGeom,
    pub frames: EntityVec<DieId, DieFrameGeom>,
    pub col_cfg: ColId,
    pub col_clk: ColId,
    pub col_lio: Option<ColId>,
    pub col_rio: Option<ColId>,
    pub col_lcio: Option<ColId>,
    pub col_rcio: Option<ColId>,
    pub col_lgt: Option<ColId>,
    pub col_rgt: Option<ColId>,
    pub col_mgt: Option<(ColId, ColId)>,
    pub row_dcmiob: Option<RowId>,
    pub row_iobdcm: Option<RowId>,
    pub io: Vec<IoCoord>,
    pub gt: Vec<(DieId, ColId, RowId)>,
    pub gtz: DirPartMap<ExpandedGtz>,
    pub cfg_io: BiHashMap<SharedCfgPad, IoCoord>,
    pub banklut: EntityVec<DieId, u32>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum IoDiffKind {
    None,
    P(IoCoord),
    N(IoCoord),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum IoVrKind {
    #[default]
    None,
    VrP,
    VrN,
}

#[derive(Debug, Clone)]
pub struct IoInfo {
    pub bank: u32,
    pub biob: u32,
    pub pkgid: u32,
    pub byte: Option<u32>,
    pub kind: IoKind,
    pub diff: IoDiffKind,
    pub is_vref: bool,
    pub is_lc: bool,
    pub is_gc: bool,
    pub is_srcc: bool,
    pub is_mrcc: bool,
    pub is_dqs: bool,
    pub vr: IoVrKind,
}

#[derive(Debug, Clone)]
pub struct GtInfo {
    pub bank: u32,
    pub kind: GtKind,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct IoCoord {
    pub die: DieId,
    pub col: ColId,
    pub row: RowId,
    pub iob: TileIobId,
}

#[derive(Clone, Debug)]
pub struct PsIo {
    pub bank: u32,
    pub name: String,
}

#[derive(Debug)]
pub struct ExpandedGtz {
    pub kind: GtzBelId,
    pub bank: u32,
    pub die: DieId,
    pub cols: EntityVec<GtzIntColId, ColId>,
    pub rows: EntityVec<GtzIntRowId, RowId>,
}

impl ExpandedDevice<'_> {
    pub fn adjust_vivado(&mut self) {
        if self.kind == ChipKind::Virtex7 {
            let lvb6 = self.egrid.db.wires.get("LVB.6").unwrap().0;
            let mut cursed_wires = HashSet::new();
            for i in 1..self.chips.len() {
                let dieid_s = DieId::from_idx(i - 1);
                let dieid_n = DieId::from_idx(i);
                let die_s = self.egrid.die(dieid_s);
                let die_n = self.egrid.die(dieid_n);
                for col in die_s.cols() {
                    let row_s = die_s.rows().next_back().unwrap() - 49;
                    let row_n = die_n.rows().next().unwrap() + 1;
                    if !die_s[(col, row_s)].tiles.is_empty()
                        && !die_n[(col, row_n)].tiles.is_empty()
                    {
                        cursed_wires.insert((dieid_s, (col, row_s), lvb6));
                    }
                }
            }
            self.egrid.blackhole_wires.extend(cursed_wires);
        }
    }

    pub fn in_int_hole(&self, die: DieId, col: ColId, row: RowId) -> bool {
        for hole in &self.int_holes[die] {
            if hole.contains(col, row) {
                return true;
            }
        }
        false
    }

    pub fn in_site_hole(&self, die: DieId, col: ColId, row: RowId) -> bool {
        for hole in &self.site_holes[die] {
            if hole.contains(col, row) {
                return true;
            }
        }
        false
    }

    pub fn get_io_info(&self, io: IoCoord) -> IoInfo {
        let chip = self.chips[io.die];
        let reg = chip.row_to_reg(io.row);
        match self.kind {
            ChipKind::Virtex4 => {
                let (bank, biob, pkgid, is_gc) = if io.col == self.col_cfg {
                    if io.row < chip.row_bufg() {
                        if io.row < self.row_dcmiob.unwrap() + 8 {
                            (
                                4,
                                (io.row.to_idx() % 8) * 2,
                                (8 - (io.row.to_idx() % 8)),
                                true,
                            )
                        } else if io.row >= chip.row_bufg() - 16 {
                            (
                                2,
                                (io.row.to_idx() % 8) * 2,
                                (8 - (io.row.to_idx() % 8)),
                                false,
                            )
                        } else if io.row < self.row_dcmiob.unwrap() + 24 {
                            (
                                2,
                                (io.row.to_idx() % 16) * 2 + 16,
                                (16 - ((io.row.to_idx() % 16) ^ 8)) + 8,
                                io.row.to_idx() % 16 < 8,
                            )
                        } else {
                            (
                                2,
                                (io.row.to_idx() % 16) * 2 + 48,
                                (16 - ((io.row.to_idx() % 16) ^ 8)) + 24,
                                false,
                            )
                        }
                    } else {
                        if io.row >= self.row_iobdcm.unwrap() - 8 {
                            (
                                3,
                                (io.row.to_idx() % 8) * 2,
                                (8 - (io.row.to_idx() % 8)),
                                true,
                            )
                        } else if io.row < chip.row_bufg() + 16 {
                            (
                                1,
                                (io.row.to_idx() % 8) * 2,
                                (8 - (io.row.to_idx() % 8)),
                                false,
                            )
                        } else if io.row >= self.row_iobdcm.unwrap() - 24 {
                            (
                                1,
                                (io.row.to_idx() % 16) * 2 + 16,
                                (16 - (io.row.to_idx() % 16)) + 8,
                                io.row.to_idx() % 16 >= 8,
                            )
                        } else {
                            (
                                1,
                                (io.row.to_idx() % 16) * 2 + 48,
                                (16 - (io.row.to_idx() % 16)) + 24,
                                false,
                            )
                        }
                    }
                } else if Some(io.col) == self.col_lio {
                    (
                        match chip.regs {
                            4 => [7, 5][reg.to_idx() / 2],
                            6 => [7, 9, 5][reg.to_idx() / 2],
                            8 => [7, 11, 9, 5][reg.to_idx() / 2],
                            10 => [7, 11, 13, 9, 5][reg.to_idx() / 2],
                            12 => [7, 11, 15, 13, 9, 5][reg.to_idx() / 2],
                            _ => unreachable!(),
                        },
                        io.row.to_idx() % 32 * 2,
                        32 - (io.row.to_idx() % 32),
                        false,
                    )
                } else {
                    (
                        match chip.regs {
                            4 => [8, 6][reg.to_idx() / 2],
                            6 => [8, 10, 6][reg.to_idx() / 2],
                            8 => [8, 12, 10, 6][reg.to_idx() / 2],
                            10 => [8, 12, 14, 10, 6][reg.to_idx() / 2],
                            12 => [8, 12, 16, 14, 10, 6][reg.to_idx() / 2],
                            _ => unreachable!(),
                        },
                        io.row.to_idx() % 32 * 2,
                        32 - (io.row.to_idx() % 32),
                        false,
                    )
                };
                let is_vr = match bank {
                    1 => biob == 28,
                    2 => biob == 34,
                    3 => biob == 12,
                    4 => biob == 2,
                    _ => biob == 18,
                };
                let biob = (biob + io.iob.to_idx()) as u32;
                let pkgid = pkgid as u32;
                let is_cc = matches!(io.row.to_idx() % 16, 7 | 8);
                IoInfo {
                    bank,
                    biob,
                    pkgid,
                    byte: None,
                    kind: IoKind::Hpio,
                    diff: if io.iob.to_idx() == 0 {
                        IoDiffKind::N(IoCoord {
                            iob: TileIobId::from_idx(1),
                            ..io
                        })
                    } else {
                        IoDiffKind::P(IoCoord {
                            iob: TileIobId::from_idx(0),
                            ..io
                        })
                    },
                    is_vref: io.row.to_idx() % 8 == 4 && io.iob.to_idx() == 0,
                    is_lc: is_cc || io.col == self.col_cfg,
                    is_gc,
                    is_srcc: is_cc,
                    is_mrcc: false,
                    is_dqs: false,
                    vr: if is_vr {
                        if io.iob.to_idx() == 0 {
                            IoVrKind::VrP
                        } else {
                            IoVrKind::VrN
                        }
                    } else {
                        IoVrKind::None
                    },
                }
            }
            ChipKind::Virtex5 => {
                let rr = reg - chip.reg_cfg;
                let bank = if io.col == self.col_cfg {
                    (if rr <= -4 {
                        6 + (-rr - 4) * 2
                    } else if rr >= 3 {
                        5 + (rr - 3) * 2
                    } else if rr < 0 {
                        2 + (-rr - 1) * 2
                    } else {
                        1 + rr * 2
                    }) as u32
                } else {
                    (if rr < 0 {
                        13 + (-rr - 1) * 4
                    } else {
                        11 + rr * 4
                    }) as u32
                        + if self.col_rio == Some(io.col) { 1 } else { 0 }
                };

                let is_short_bank = bank <= 4;
                let biob = if is_short_bank {
                    2 * (io.row.to_idx() % 10)
                } else {
                    2 * (io.row.to_idx() % 20)
                };
                let pkgid = if is_short_bank {
                    (18 - biob) / 2
                } else {
                    (38 - biob) / 2
                };
                let is_vr = match bank {
                    1 | 2 => false,
                    3 => biob == 14,
                    4 => biob == 4,
                    _ => biob == 14,
                };
                let biob = (biob + io.iob.to_idx()) as u32;
                let pkgid = pkgid as u32;
                IoInfo {
                    bank,
                    biob,
                    pkgid,
                    byte: None,
                    kind: IoKind::Hpio,
                    diff: if io.iob.to_idx() == 0 {
                        IoDiffKind::N(IoCoord {
                            iob: TileIobId::from_idx(1),
                            ..io
                        })
                    } else {
                        IoDiffKind::P(IoCoord {
                            iob: TileIobId::from_idx(0),
                            ..io
                        })
                    },
                    is_vref: io.row.to_idx() % 10 == 5 && io.iob.to_idx() == 0,
                    is_lc: false,
                    is_gc: matches!(bank, 3 | 4),
                    is_srcc: matches!(io.row.to_idx() % 20, 8..=11),
                    is_mrcc: false,
                    is_dqs: false,
                    vr: if is_vr {
                        if io.iob.to_idx() == 0 {
                            IoVrKind::VrP
                        } else {
                            IoVrKind::VrN
                        }
                    } else {
                        IoVrKind::None
                    },
                }
            }
            ChipKind::Virtex6 => {
                let bank = (if Some(io.col) == self.col_lio {
                    15
                } else if Some(io.col) == self.col_lcio {
                    25
                } else if Some(io.col) == self.col_rcio {
                    35
                } else if Some(io.col) == self.col_rio {
                    45
                } else {
                    unreachable!()
                } - chip.reg_cfg.to_idx()
                    + reg.to_idx()) as u32;
                let is_vr = match bank {
                    34 => io.row.to_idx() % 40 == 0,
                    24 => io.row.to_idx() % 40 == 4,
                    15 | 25 | 35 => io.row.to_idx() % 40 == 6,
                    _ => io.row.to_idx() % 40 == 14,
                };
                IoInfo {
                    bank,
                    biob: (io.row.to_idx() % 40 + io.iob.to_idx()) as u32,
                    pkgid: ((38 - io.row.to_idx() % 40) / 2) as u32,
                    byte: None,
                    kind: IoKind::Hpio,
                    diff: if io.iob.to_idx() == 0 {
                        IoDiffKind::N(IoCoord {
                            iob: TileIobId::from_idx(1),
                            ..io
                        })
                    } else {
                        IoDiffKind::P(IoCoord {
                            iob: TileIobId::from_idx(0),
                            ..io
                        })
                    },
                    is_vref: io.row.to_idx() % 20 == 10 && io.iob.to_idx() == 0,
                    is_lc: false,
                    is_gc: matches!(
                        (bank, io.row.to_idx() % 40),
                        (24 | 34, 36 | 38) | (25 | 35, 0 | 2)
                    ),
                    is_srcc: matches!(io.row.to_idx() % 40, 16 | 22),
                    is_mrcc: matches!(io.row.to_idx() % 40, 18 | 20),
                    is_dqs: false,
                    vr: if is_vr {
                        if io.iob.to_idx() == 0 {
                            IoVrKind::VrP
                        } else {
                            IoVrKind::VrN
                        }
                    } else {
                        IoVrKind::None
                    },
                }
            }
            ChipKind::Virtex7 => {
                let x = if Some(io.col) == self.col_lio { 0 } else { 20 };
                let y = self.banklut[io.die] + reg.to_idx() as u32 - chip.reg_cfg.to_idx() as u32;
                let bank = x + y;
                let iocol = chip
                    .cols_io
                    .iter()
                    .find(|iocol| iocol.col == io.col)
                    .unwrap();
                let kind = iocol.regs[reg].unwrap();
                IoInfo {
                    bank,
                    biob: (io.row.to_idx() % 50 + io.iob.to_idx()) as u32,
                    pkgid: (50 - io.row.to_idx() % 50) as u32 / 2,
                    byte: match io.row.to_idx() % 50 {
                        0 | 49 => None,
                        1..13 => Some(3),
                        13..25 => Some(2),
                        25..37 => Some(1),
                        37..49 => Some(0),
                        _ => unreachable!(),
                    },
                    kind,
                    diff: if matches!(io.row.to_idx() % 50, 0 | 49) {
                        IoDiffKind::None
                    } else if io.iob.to_idx() == 0 {
                        IoDiffKind::N(IoCoord {
                            iob: TileIobId::from_idx(1),
                            ..io
                        })
                    } else {
                        IoDiffKind::P(IoCoord {
                            iob: TileIobId::from_idx(0),
                            ..io
                        })
                    },
                    is_vref: matches!(io.row.to_idx() % 50, 11 | 37) && io.iob.to_idx() == 0,
                    is_lc: false,
                    is_gc: false,
                    is_srcc: matches!(io.row.to_idx() % 50, 21 | 27),
                    is_mrcc: matches!(io.row.to_idx() % 50, 23 | 25),
                    is_dqs: matches!(io.row.to_idx() % 50, 7 | 19 | 31 | 43),
                    vr: match io.row.to_idx() % 50 {
                        0 if kind == IoKind::Hpio => IoVrKind::VrP,
                        49 if kind == IoKind::Hpio => IoVrKind::VrN,
                        _ => IoVrKind::None,
                    },
                }
            }
        }
    }

    pub fn get_gt_info(&self, die: DieId, col: ColId, row: RowId) -> GtInfo {
        let chip = self.chips[die];
        let reg = chip.row_to_reg(row);
        let gtcol = chip.cols_gt.iter().find(|gtcol| gtcol.col == col).unwrap();
        let kind = gtcol.regs[reg].unwrap();
        let bank = match self.kind {
            ChipKind::Virtex4 => {
                let lr = if Some(col) == self.col_lgt { 'L' } else { 'R' };
                let banks: &[u32] = match (lr, chip.regs) {
                    ('L', 4) => &[105, 102],
                    ('L', 6) => &[105, 103, 102],
                    ('L', 8) => &[106, 105, 103, 102],
                    ('L', 10) => &[106, 105, 103, 102, 101],
                    ('L', 12) => &[106, 105, 104, 103, 102, 101],
                    ('R', 4) => &[110, 113],
                    ('R', 6) => &[110, 112, 113],
                    ('R', 8) => &[109, 110, 112, 113],
                    ('R', 10) => &[109, 110, 112, 113, 114],
                    ('R', 12) => &[109, 110, 111, 112, 113, 114],
                    _ => unreachable!(),
                };
                banks[row.to_idx() / 32]
            }
            ChipKind::Virtex5 => {
                (if reg < chip.reg_cfg {
                    113 + (chip.reg_cfg - reg - 1) * 4
                } else {
                    111 + (reg - chip.reg_cfg) * 4
                }) as u32
                    + if col.to_idx() != 0 { 1 } else { 0 }
            }
            ChipKind::Virtex6 => {
                (reg - chip.reg_cfg + if col.to_idx() == 0 { 105 } else { 115 }) as u32
            }
            ChipKind::Virtex7 => {
                if kind == GtKind::Gtp {
                    if chip.has_ps {
                        112
                    } else {
                        let x = if gtcol.is_middle && col > self.col_clk {
                            100
                        } else {
                            200
                        };
                        let y = if reg.to_idx() == 0 { 13 } else { 16 };
                        x + y
                    }
                } else {
                    let x = if col.to_idx() == 0 { 200 } else { 100 };
                    let y = self.banklut[die] + reg.to_idx() as u32 - chip.reg_cfg.to_idx() as u32;
                    x + y
                }
            }
        };
        GtInfo { bank, kind }
    }

    pub fn get_sysmon_vaux(
        &self,
        die: DieId,
        col: ColId,
        row: RowId,
        idx: usize,
    ) -> Option<(IoCoord, IoCoord)> {
        assert_eq!(col, self.col_cfg);
        match self.kind {
            ChipKind::Virtex4 => {
                let dy = match idx {
                    0 => return None,
                    1 => 0,
                    2 => 1,
                    3 => 2,
                    4 => 3,
                    5 => 5,
                    6 => 6,
                    7 => 7,
                    _ => unreachable!(),
                };
                Some((
                    IoCoord {
                        die,
                        col: self.col_lio.unwrap(),
                        row: row + dy,
                        iob: TileIobId::from_idx(1),
                    },
                    IoCoord {
                        die,
                        col: self.col_lio.unwrap(),
                        row: row + dy,
                        iob: TileIobId::from_idx(0),
                    },
                ))
            }
            ChipKind::Virtex5 => {
                let dy = [0, 1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 14, 18, 19][idx];
                Some((
                    IoCoord {
                        die,
                        col: self.col_lio.unwrap(),
                        row: row + dy,
                        iob: TileIobId::from_idx(1),
                    },
                    IoCoord {
                        die,
                        col: self.col_lio.unwrap(),
                        row: row + dy,
                        iob: TileIobId::from_idx(0),
                    },
                ))
            }
            ChipKind::Virtex6 => {
                let cl = self.col_lio.unwrap_or_else(|| self.col_lcio.unwrap());
                let cr = self.col_rcio.unwrap();
                let (col_io, dy) = [
                    (cr, 34),
                    (cr, 32),
                    (cr, 28),
                    (cr, 26),
                    (cr, 24),
                    (cr, 14),
                    (cr, 12),
                    (cr, 8),
                    (cl, 34),
                    (cl, 32),
                    (cl, 28),
                    (cl, 26),
                    (cl, 24),
                    (cl, 14),
                    (cl, 12),
                    (cl, 8),
                ][idx];
                Some((
                    IoCoord {
                        die,
                        col: col_io,
                        row: row + dy,
                        iob: TileIobId::from_idx(1),
                    },
                    IoCoord {
                        die,
                        col: col_io,
                        row: row + dy,
                        iob: TileIobId::from_idx(0),
                    },
                ))
            }
            ChipKind::Virtex7 => {
                let chip = self.chips[die];
                let vaux = match chip.get_xadc_io_loc() {
                    XadcIoLoc::Left => {
                        let col_lio = self.col_lio.unwrap();
                        [
                            Some((col_lio, 47)),
                            Some((col_lio, 43)),
                            Some((col_lio, 39)),
                            Some((col_lio, 33)),
                            Some((col_lio, 29)),
                            Some((col_lio, 25)),
                            None,
                            None,
                            Some((col_lio, 45)),
                            Some((col_lio, 41)),
                            Some((col_lio, 35)),
                            Some((col_lio, 31)),
                            Some((col_lio, 27)),
                            None,
                            None,
                            None,
                        ]
                    }
                    XadcIoLoc::Right => {
                        let col_rio = self.col_rio.unwrap();
                        [
                            Some((col_rio, 47)),
                            Some((col_rio, 43)),
                            Some((col_rio, 35)),
                            Some((col_rio, 31)),
                            Some((col_rio, 21)),
                            Some((col_rio, 15)),
                            Some((col_rio, 9)),
                            Some((col_rio, 5)),
                            Some((col_rio, 45)),
                            Some((col_rio, 39)),
                            Some((col_rio, 33)),
                            Some((col_rio, 29)),
                            Some((col_rio, 19)),
                            Some((col_rio, 13)),
                            Some((col_rio, 7)),
                            Some((col_rio, 1)),
                        ]
                    }
                    XadcIoLoc::Both => {
                        let col_lio = self.col_lio.unwrap();
                        let col_rio = self.col_rio.unwrap();
                        [
                            Some((col_lio, 47)),
                            Some((col_lio, 43)),
                            Some((col_lio, 35)),
                            Some((col_lio, 31)),
                            Some((col_rio, 47)),
                            Some((col_rio, 43)),
                            Some((col_rio, 35)),
                            Some((col_rio, 31)),
                            Some((col_lio, 45)),
                            Some((col_lio, 39)),
                            Some((col_lio, 33)),
                            Some((col_lio, 29)),
                            Some((col_rio, 45)),
                            Some((col_rio, 39)),
                            Some((col_rio, 33)),
                            Some((col_rio, 29)),
                        ]
                    }
                };
                vaux[idx].map(|(col_io, dy)| {
                    (
                        IoCoord {
                            die,
                            col: col_io,
                            row: row - 25 + dy,
                            iob: TileIobId::from_idx(1),
                        },
                        IoCoord {
                            die,
                            col: col_io,
                            row: row - 25 + dy,
                            iob: TileIobId::from_idx(0),
                        },
                    )
                })
            }
        }
    }

    pub fn get_ps_bank(&self, io: PsPad) -> u32 {
        match io {
            PsPad::Mio(idx) => {
                if idx < 16 {
                    500
                } else {
                    501
                }
            }
            PsPad::Clk => 500,
            PsPad::PorB => 500,
            PsPad::SrstB => 501,
            PsPad::DdrDq(_) => 502,
            PsPad::DdrDm(_) => 502,
            PsPad::DdrDqsP(_) => 502,
            PsPad::DdrDqsN(_) => 502,
            PsPad::DdrA(_) => 502,
            PsPad::DdrBa(_) => 502,
            PsPad::DdrVrP => 502,
            PsPad::DdrVrN => 502,
            PsPad::DdrCkP => 502,
            PsPad::DdrCkN => 502,
            PsPad::DdrCke => 502,
            PsPad::DdrOdt => 502,
            PsPad::DdrDrstB => 502,
            PsPad::DdrCsB => 502,
            PsPad::DdrRasB => 502,
            PsPad::DdrCasB => 502,
            PsPad::DdrWeB => 502,
        }
    }

    pub fn get_ps_pins(&self) -> Vec<PsPad> {
        let mut res = vec![];
        if self.chips.first().unwrap().has_ps {
            for i in 0..54 {
                res.push(PsPad::Mio(i));
            }
            res.extend([
                PsPad::Clk,
                PsPad::PorB,
                PsPad::SrstB,
                PsPad::DdrWeB,
                PsPad::DdrCasB,
                PsPad::DdrRasB,
                PsPad::DdrCsB,
                PsPad::DdrOdt,
                PsPad::DdrCke,
                PsPad::DdrCkN,
                PsPad::DdrCkP,
                PsPad::DdrDrstB,
                PsPad::DdrVrP,
                PsPad::DdrVrN,
            ]);
            for i in 0..15 {
                res.push(PsPad::DdrA(i));
            }
            for i in 0..3 {
                res.push(PsPad::DdrBa(i));
            }
            for i in 0..32 {
                res.push(PsPad::DdrDq(i));
            }
            for i in 0..4 {
                res.push(PsPad::DdrDm(i));
            }
            for i in 0..4 {
                res.push(PsPad::DdrDqsP(i));
                res.push(PsPad::DdrDqsN(i));
            }
        }
        res
    }

    pub fn btile_main(&self, die: DieId, col: ColId, row: RowId) -> BitTile {
        let reg = self.chips[die].row_to_reg(row);
        let rd = (row - self.chips[die].row_reg_bot(reg)) as usize;
        let (height, bit, flip) = if self.kind == ChipKind::Virtex4 {
            let flip = reg < self.chips[die].reg_cfg;
            let pos = if flip { 15 - rd } else { rd } * 80;
            (80, if pos < 640 { pos } else { pos + 32 }, flip)
        } else {
            (
                64,
                64 * rd
                    + if row >= self.chips[die].row_reg_hclk(reg) {
                        32
                    } else {
                        0
                    },
                false,
            )
        };
        BitTile::Main(
            die,
            self.frames[die].col_frame[reg][col],
            self.frames[die].col_width[reg][col],
            bit,
            height,
            flip,
        )
    }

    pub fn btile_hclk(&self, die: DieId, col: ColId, row: RowId) -> BitTile {
        let reg = self.chips[die].row_to_reg(row);
        let bit = if self.kind == ChipKind::Virtex4 {
            80 * self.chips[die].rows_per_reg() / 2
        } else {
            64 * self.chips[die].rows_per_reg() / 2
        };
        BitTile::Main(
            die,
            self.frames[die].col_frame[reg][col],
            self.frames[die].col_width[reg][col],
            bit,
            32,
            false,
        )
    }

    pub fn btile_spine(&self, die: DieId, row: RowId) -> BitTile {
        let reg = self.chips[die].row_to_reg(row);
        let rd = (row - self.chips[die].row_reg_bot(reg)) as usize;
        let (height, bit, flip) = if self.kind == ChipKind::Virtex4 {
            let flip = reg < self.chips[die].reg_cfg;
            let pos = if flip { 15 - rd } else { rd } * 80;
            (80, if pos < 640 { pos } else { pos + 32 }, flip)
        } else {
            (
                64,
                64 * rd
                    + if row >= self.chips[die].row_reg_hclk(reg) {
                        32
                    } else {
                        0
                    },
                false,
            )
        };
        BitTile::Main(
            die,
            self.frames[die].spine_frame[reg],
            match self.kind {
                ChipKind::Virtex4 => 3,
                ChipKind::Virtex5 => 4,
                _ => unreachable!(),
            },
            bit,
            height,
            flip,
        )
    }

    pub fn btile_spine_hclk(&self, die: DieId, row: RowId) -> BitTile {
        let reg = self.chips[die].row_to_reg(row);
        let bit = if self.kind == ChipKind::Virtex4 {
            80 * self.chips[die].rows_per_reg() / 2
        } else {
            64 * self.chips[die].rows_per_reg() / 2
        };
        BitTile::Main(
            die,
            self.frames[die].spine_frame[reg],
            match self.kind {
                ChipKind::Virtex4 => 3,
                ChipKind::Virtex5 => 4,
                _ => unreachable!(),
            },
            bit,
            32,
            false,
        )
    }

    pub fn btile_bram(&self, die: DieId, col: ColId, row: RowId) -> BitTile {
        let reg = self.chips[die].row_to_reg(row);
        let rd = (row - self.chips[die].row_reg_bot(reg)) as usize;
        let (width, bit, flip) = if self.kind == ChipKind::Virtex4 {
            let flip = reg < self.chips[die].reg_cfg;
            let pos = if flip { 12 - rd } else { rd } * 80;
            (64, if pos < 640 { pos } else { pos + 32 }, flip)
        } else {
            (
                128,
                64 * rd
                    + if row >= self.chips[die].row_reg_hclk(reg) {
                        32
                    } else {
                        0
                    },
                false,
            )
        };
        BitTile::Main(
            die,
            self.frames[die].bram_frame[reg][col],
            width,
            bit,
            320,
            flip,
        )
    }

    pub fn tile_bits(&self, nloc: NodeLoc) -> Vec<BitTile> {
        let (die, col, row, _) = nloc;
        let node = self.egrid.tile(nloc);
        let kind = self.egrid.db.tile_classes.key(node.class).as_str();
        if kind == "BRAM" {
            if self.kind == ChipKind::Virtex4 {
                vec![
                    self.btile_main(die, col, row),
                    self.btile_main(die, col, row + 1),
                    self.btile_main(die, col, row + 2),
                    self.btile_main(die, col, row + 3),
                    self.btile_bram(die, col, row),
                ]
            } else {
                vec![
                    self.btile_main(die, col, row),
                    self.btile_main(die, col, row + 1),
                    self.btile_main(die, col, row + 2),
                    self.btile_main(die, col, row + 3),
                    self.btile_main(die, col, row + 4),
                    self.btile_bram(die, col, row),
                ]
            }
        } else if self.kind == ChipKind::Virtex7 && kind == "HCLK" {
            assert_eq!(col.to_idx() % 2, 0);
            vec![
                self.btile_hclk(die, col, row),
                self.btile_hclk(die, col + 1, row),
            ]
        } else if kind.starts_with("HCLK") {
            vec![self.btile_hclk(die, col, row)]
        } else if self.kind == ChipKind::Virtex4 && kind.starts_with("CLK_DCM") {
            Vec::from_iter((0..8).map(|idx| self.btile_spine(die, row + idx)))
        } else if self.kind == ChipKind::Virtex4 && kind.starts_with("CLK_IOB") {
            Vec::from_iter((0..16).map(|idx| self.btile_spine(die, row + idx)))
        } else if kind == "CLK_TERM" {
            vec![self.btile_spine(die, row)]
        } else if self.kind == ChipKind::Virtex5
            && (kind.starts_with("CLK_CMT")
                || kind.starts_with("CLK_IOB")
                || kind.starts_with("CLK_MGT"))
        {
            Vec::from_iter((0..10).map(|idx| self.btile_spine(die, row + idx)))
        } else if self.kind == ChipKind::Virtex4 && kind == "CFG" {
            let mut res = vec![];
            for i in 0..16 {
                res.push(self.btile_main(die, col, row + i));
            }
            for i in 0..16 {
                res.push(self.btile_spine(die, row + i));
            }
            res
        } else if self.kind == ChipKind::Virtex5 && kind == "CFG" {
            let mut res = vec![];
            for i in 0..20 {
                res.push(self.btile_main(die, col, row + i));
            }
            for i in 0..20 {
                res.push(self.btile_spine(die, row + i));
            }
            res
        } else if kind == "CLK_HROW" {
            match self.kind {
                ChipKind::Virtex4 | ChipKind::Virtex5 => {
                    vec![
                        self.btile_spine(die, row - 1),
                        self.btile_spine(die, row),
                        self.btile_spine_hclk(die, row),
                    ]
                }
                ChipKind::Virtex6 => unreachable!(),
                ChipKind::Virtex7 => {
                    let mut res = vec![];
                    for i in 0..8 {
                        res.push(self.btile_main(die, col, row - 4 + i));
                    }
                    res.push(self.btile_hclk(die, col, row));
                    res
                }
            }
        } else if self.kind == ChipKind::Virtex6 && kind == "PCIE" {
            Vec::from_iter((0..20).map(|idx| self.btile_main(die, col + 3, row + idx)))
        } else if kind == "PCIE3" {
            Vec::from_iter((0..50).map(|idx| self.btile_main(die, col + 4, row + idx)))
        } else if matches!(self.kind, ChipKind::Virtex6 | ChipKind::Virtex7) && kind == "CMT" {
            let mut res = vec![];
            for i in 0..self.chips[die].rows_per_reg() {
                res.push(self.btile_main(
                    die,
                    col,
                    self.chips[die].row_hclk(row) - self.chips[die].rows_per_reg() / 2 + i,
                ));
            }
            res.push(self.btile_hclk(die, col, row));
            res
        } else if self.kind == ChipKind::Virtex5 && matches!(kind, "GTP" | "GTX") {
            let mut res = vec![];
            for i in 0..20 {
                res.push(self.btile_main(die, col, row + i));
            }
            res.push(self.btile_hclk(die, col, row + 10));
            res
        } else if kind == "GTP_COMMON_MID" {
            let col = if col.to_idx() % 2 == 0 {
                col - 1
            } else {
                col + 1
            };
            vec![
                self.btile_main(die, col, row - 3),
                self.btile_main(die, col, row - 2),
                self.btile_main(die, col, row - 1),
                self.btile_main(die, col, row),
                self.btile_main(die, col, row + 1),
                self.btile_main(die, col, row + 2),
                self.btile_hclk(die, col, row),
            ]
        } else if kind == "GTP_CHANNEL_MID" {
            let col = if col.to_idx() % 2 == 0 {
                col - 1
            } else {
                col + 1
            };
            Vec::from_iter((0..11).map(|i| self.btile_main(die, col, row + i)))
        } else if kind == "CMT_BUFG_BOT" {
            vec![
                self.btile_main(die, col, row - 2),
                self.btile_main(die, col, row - 1),
            ]
        } else if kind == "CMT_BUFG_TOP" {
            vec![
                self.btile_main(die, col, row),
                self.btile_main(die, col, row + 1),
            ]
        } else {
            Vec::from_iter(
                node.cells
                    .values()
                    .map(|&(col, row)| self.btile_main(die, col, row)),
            )
        }
    }

    pub fn tile_cfg(&self, die: DieId) -> NodeLoc {
        let chip = self.chips[die];
        match self.kind {
            ChipKind::Virtex4 => {
                self.egrid
                    .get_tile_by_class(die, (self.col_cfg, chip.row_bufg() - 8), |kind| {
                        kind == "CFG"
                    })
            }
            ChipKind::Virtex5 => {
                self.egrid
                    .get_tile_by_class(die, (self.col_cfg, chip.row_bufg() - 10), |kind| {
                        kind == "CFG"
                    })
            }
            ChipKind::Virtex6 => {
                self.egrid
                    .get_tile_by_class(die, (self.col_cfg, chip.row_bufg()), |kind| kind == "CFG")
            }
            ChipKind::Virtex7 => self.egrid.get_tile_by_class(
                die,
                (self.col_cfg, chip.row_reg_bot(chip.reg_cfg - 1)),
                |kind| kind == "CFG",
            ),
        }
    }
}
