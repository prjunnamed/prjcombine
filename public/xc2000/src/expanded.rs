use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};
use prjcombine_interconnect::{
    dir::{DirH, DirV},
    grid::{ColId, DieId, ExpandedGrid, RowId, TileCoord},
};
use prjcombine_types::bsdata::BitRectId;
use prjcombine_xilinx_bitstream::{BitRect, BitstreamGeom};

use crate::{
    chip::{Chip, ChipKind},
    xc2000, xc3000, xc4000, xc5200,
};

pub struct ExpandedDevice<'a> {
    pub chip: &'a Chip,
    pub egrid: ExpandedGrid<'a>,
    pub bs_geom: BitstreamGeom,
    pub col_frame: EntityVec<ColId, usize>,
    pub llh_frame: EntityPartVec<ColId, usize>,
    pub row_framebit: EntityVec<RowId, usize>,
    pub llv_framebit: EntityPartVec<RowId, usize>,
}

impl ExpandedDevice<'_> {
    pub fn tile_bits(&self, tcrd: TileCoord) -> EntityVec<BitRectId, BitRect> {
        let col = tcrd.col;
        let row = tcrd.row;
        let tile = &self[tcrd];
        match self.chip.kind {
            ChipKind::Xc2000 => {
                if tile.class == xc2000::tcls::BIDIV_E {
                    EntityVec::from_iter([self.btile_llv(col, row), self.btile_main(col, row)])
                } else if tcrd.slot == xc2000::tslots::BIDIV {
                    EntityVec::from_iter([self.btile_llv(col, row)])
                } else if tcrd.slot == xc2000::tslots::BIDIH {
                    EntityVec::from_iter([self.btile_llh(col, row)])
                } else {
                    let mut res = EntityVec::from_iter([self.btile_main(col, row)]);
                    if col != self.chip.col_e()
                        && (row == self.chip.row_s() || row == self.chip.row_n())
                    {
                        res.push(self.btile_main(col + 1, row));
                    }
                    res
                }
            }
            ChipKind::Xc3000 | ChipKind::Xc3000A => {
                if tcrd.slot == xc3000::tslots::LLV && !self.chip.is_small {
                    EntityVec::from_iter([self.btile_llv(col, row)])
                } else if tcrd.slot != xc3000::tslots::MAIN {
                    EntityVec::from_iter([self.btile_main(col, row)])
                } else {
                    let mut res = EntityVec::from_iter([self.btile_main(col, row)]);
                    if row != self.chip.row_n() {
                        res.push(self.btile_main(col, row + 1));
                    }
                    res
                }
            }
            ChipKind::Xc4000
            | ChipKind::Xc4000A
            | ChipKind::Xc4000H
            | ChipKind::Xc4000E
            | ChipKind::Xc4000Ex
            | ChipKind::Xc4000Xla
            | ChipKind::Xc4000Xv
            | ChipKind::SpartanXl => {
                if tcrd.slot == xc4000::tslots::LLH {
                    if row == self.chip.row_s() {
                        EntityVec::from_iter([
                            self.btile_llh(col, row),
                            self.btile_main(col - 1, row),
                        ])
                    } else if row == self.chip.row_n() {
                        EntityVec::from_iter([
                            self.btile_llh(col, row),
                            self.btile_llh(col, row - 1),
                            self.btile_main(col - 1, row),
                        ])
                    } else if row == self.chip.row_s() + 1 {
                        EntityVec::from_iter([
                            self.btile_llh(col, row),
                            self.btile_llh(col, row - 1),
                            self.btile_main(col - 1, row - 1),
                        ])
                    } else {
                        EntityVec::from_iter([
                            self.btile_llh(col, row),
                            self.btile_llh(col, row - 1),
                        ])
                    }
                } else if tcrd.slot == xc4000::tslots::LLV {
                    if col == self.chip.col_w() {
                        EntityVec::from_iter([
                            self.btile_llv(col, row),
                            self.btile_llv(col + 1, row),
                        ])
                    } else {
                        EntityVec::from_iter([self.btile_llv(col, row)])
                    }
                } else {
                    if col == self.chip.col_w() {
                        if row == self.chip.row_s() {
                            // LL
                            EntityVec::from_iter([self.btile_main(col, row)])
                        } else if row == self.chip.row_n() {
                            // UL
                            EntityVec::from_iter([self.btile_main(col, row)])
                        } else {
                            // LEFT
                            EntityVec::from_iter([
                                self.btile_main(col, row),
                                self.btile_main(col, row - 1),
                            ])
                        }
                    } else if col == self.chip.col_e() {
                        if row == self.chip.row_s() {
                            // LR
                            EntityVec::from_iter([self.btile_main(col, row)])
                        } else if row == self.chip.row_n() {
                            // UR
                            EntityVec::from_iter([
                                self.btile_main(col, row),
                                self.btile_main(col, row - 1),
                                self.btile_main(col - 1, row),
                            ])
                        } else {
                            // RT
                            EntityVec::from_iter([
                                self.btile_main(col, row),
                                self.btile_main(col, row - 1),
                                self.btile_main(col - 1, row),
                            ])
                        }
                    } else {
                        if row == self.chip.row_s() {
                            // BOT
                            EntityVec::from_iter([
                                self.btile_main(col, row),
                                self.btile_main(col + 1, row),
                            ])
                        } else if row == self.chip.row_n() {
                            // TOP
                            EntityVec::from_iter([
                                self.btile_main(col, row),
                                self.btile_main(col, row - 1),
                                self.btile_main(col + 1, row),
                                self.btile_main(col - 1, row),
                            ])
                        } else {
                            // CLB
                            EntityVec::from_iter([
                                self.btile_main(col, row),
                                self.btile_main(col, row - 1),
                                self.btile_main(col - 1, row),
                                self.btile_main(col, row + 1),
                                self.btile_main(col + 1, row),
                            ])
                        }
                    }
                }
            }
            ChipKind::Xc5200 => {
                if tcrd.slot == xc5200::tslots::LLV {
                    if tile.class == xc5200::tcls::LLV {
                        EntityVec::from_iter([self.btile_llv(col, row)])
                    } else {
                        EntityVec::from_iter([self.btile_llv(col, row), self.btile_main(col, row)])
                    }
                } else if tcrd.slot == xc5200::tslots::LLH {
                    EntityVec::from_iter([self.btile_llh(col, row)])
                } else {
                    EntityVec::from_iter([self.btile_main(col, row)])
                }
            }
        }
    }

    pub fn btile_main(&self, col: ColId, row: RowId) -> BitRect {
        BitRect::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            self.chip.btile_width_main(col),
            self.row_framebit[row],
            self.chip.btile_height_main(row),
            false,
        )
    }

    pub fn btile_llv(&self, col: ColId, row: RowId) -> BitRect {
        let bit = self.llv_framebit[row];
        let height = if self.chip.kind == ChipKind::Xc2000 {
            self.chip.btile_height_brk()
        } else if self.chip.kind.is_xc3000() || row == self.chip.row_mid() {
            self.chip.btile_height_clk()
        } else if row == self.chip.row_q(DirV::S) || row == self.chip.row_q(DirV::N) {
            self.chip.btile_height_brk()
        } else {
            unreachable!()
        };
        BitRect::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            self.chip.btile_width_main(col),
            bit,
            height,
            false,
        )
    }

    pub fn btile_llh(&self, col: ColId, row: RowId) -> BitRect {
        let frame = self.llh_frame[col];
        let width = if self.chip.kind == ChipKind::Xc2000 {
            self.chip.btile_width_brk()
        } else if col == self.chip.col_mid() {
            self.chip.btile_width_clk()
        } else if col == self.chip.col_q(DirH::W) || col == self.chip.col_q(DirH::E) {
            self.chip.btile_width_brk()
        } else {
            unreachable!()
        };
        BitRect::Main(
            DieId::from_idx(0),
            frame,
            width,
            self.row_framebit[row],
            self.chip.btile_height_main(row),
            false,
        )
    }
}

impl<'a> std::ops::Deref for ExpandedDevice<'a> {
    type Target = ExpandedGrid<'a>;

    fn deref(&self) -> &Self::Target {
        &self.egrid
    }
}
