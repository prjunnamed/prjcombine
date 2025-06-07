use prjcombine_interconnect::{
    db::RegionSlotId,
    grid::{ColId, DieId, ExpandedGrid, NodeLoc, RowId},
};
use prjcombine_xilinx_bitstream::{BitTile, BitstreamGeom};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::chip::{Chip, ChipKind};

pub const REGION_GLOBAL: RegionSlotId = RegionSlotId::from_idx_const(0);

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
    pub fn tile_bits(&self, nloc: NodeLoc) -> Vec<BitTile> {
        let (_, col, row, _) = nloc;
        let node = self.egrid.tile(nloc);
        let kind = self.egrid.db.tile_classes.key(node.class);
        match self.chip.kind {
            ChipKind::Xc2000 => {
                if kind.starts_with("BIDI") {
                    todo!()
                } else {
                    let mut res = vec![self.btile_main(col, row)];
                    if col != self.chip.col_e()
                        && (row == self.chip.row_s() || row == self.chip.row_n())
                    {
                        res.push(self.btile_main(col + 1, row));
                    }
                    res
                }
            }
            ChipKind::Xc3000 | ChipKind::Xc3000A => {
                if kind.starts_with("LLH") || (kind.starts_with("LLV") && kind.ends_with('S')) {
                    vec![self.btile_main(col, row)]
                } else if kind.starts_with("LLV") {
                    vec![self.btile_llv(col, row), self.btile_main(col, row)]
                } else {
                    let mut res = vec![self.btile_main(col, row)];
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
                if kind.starts_with("LLH") {
                    if row == self.chip.row_s() {
                        vec![self.btile_llh(col, row), self.btile_main(col - 1, row)]
                    } else if row == self.chip.row_n() {
                        vec![
                            self.btile_llh(col, row),
                            self.btile_llh(col, row - 1),
                            self.btile_main(col - 1, row),
                        ]
                    } else if row == self.chip.row_s() + 1 {
                        vec![
                            self.btile_llh(col, row),
                            self.btile_llh(col, row - 1),
                            self.btile_main(col - 1, row - 1),
                        ]
                    } else {
                        vec![self.btile_llh(col, row), self.btile_llh(col, row - 1)]
                    }
                } else if kind.starts_with("LLV") {
                    if col == self.chip.col_w() {
                        vec![self.btile_llv(col, row), self.btile_llv(col + 1, row)]
                    } else {
                        vec![self.btile_llv(col, row)]
                    }
                } else {
                    if col == self.chip.col_w() {
                        if row == self.chip.row_s() {
                            // LL
                            vec![self.btile_main(col, row)]
                        } else if row == self.chip.row_n() {
                            // UL
                            vec![self.btile_main(col, row)]
                        } else {
                            // LEFT
                            vec![self.btile_main(col, row), self.btile_main(col, row - 1)]
                        }
                    } else if col == self.chip.col_e() {
                        if row == self.chip.row_s() {
                            // LR
                            vec![self.btile_main(col, row)]
                        } else if row == self.chip.row_n() {
                            // UR
                            vec![
                                self.btile_main(col, row),
                                self.btile_main(col, row - 1),
                                self.btile_main(col - 1, row),
                            ]
                        } else {
                            // RT
                            vec![
                                self.btile_main(col, row),
                                self.btile_main(col, row - 1),
                                self.btile_main(col - 1, row),
                            ]
                        }
                    } else {
                        if row == self.chip.row_s() {
                            // BOT
                            vec![self.btile_main(col, row), self.btile_main(col + 1, row)]
                        } else if row == self.chip.row_n() {
                            // TOP
                            vec![
                                self.btile_main(col, row),
                                self.btile_main(col, row - 1),
                                self.btile_main(col + 1, row),
                                self.btile_main(col - 1, row),
                            ]
                        } else {
                            // CLB
                            vec![
                                self.btile_main(col, row),
                                self.btile_main(col, row - 1),
                                self.btile_main(col - 1, row),
                                self.btile_main(col, row + 1),
                                self.btile_main(col + 1, row),
                            ]
                        }
                    }
                }
            }
            ChipKind::Xc5200 => {
                if matches!(&kind[..], "CLKL" | "CLKR" | "CLKH") {
                    vec![self.btile_llv(col, row)]
                } else if matches!(&kind[..], "CLKB" | "CLKT" | "CLKV") {
                    vec![self.btile_llh(col, row)]
                } else {
                    vec![self.btile_main(col, row)]
                }
            }
        }
    }

    pub fn btile_main(&self, col: ColId, row: RowId) -> BitTile {
        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            self.chip.btile_width_main(col),
            self.row_framebit[row],
            self.chip.btile_height_main(row),
            false,
        )
    }

    pub fn btile_llv(&self, col: ColId, row: RowId) -> BitTile {
        let bit = self.llv_framebit[row];
        let height = if self.chip.kind == ChipKind::Xc2000 {
            self.chip.btile_height_brk()
        } else if self.chip.kind.is_xc3000() || row == self.chip.row_mid() {
            self.chip.btile_height_clk()
        } else if row == self.chip.row_qb() || row == self.chip.row_qt() {
            self.chip.btile_height_brk()
        } else {
            unreachable!()
        };
        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[col],
            self.chip.btile_width_main(col),
            bit,
            height,
            false,
        )
    }

    pub fn btile_llh(&self, col: ColId, row: RowId) -> BitTile {
        let frame = self.llh_frame[col];
        let width = if self.chip.kind == ChipKind::Xc2000 {
            self.chip.btile_width_brk()
        } else if col == self.chip.col_mid() {
            self.chip.btile_width_clk()
        } else if col == self.chip.col_ql() || col == self.chip.col_qr() {
            self.chip.btile_width_brk()
        } else {
            unreachable!()
        };
        BitTile::Main(
            DieId::from_idx(0),
            frame,
            width,
            self.row_framebit[row],
            self.chip.btile_height_main(row),
            false,
        )
    }
}
