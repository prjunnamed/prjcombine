use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};
use prjcombine_interconnect::grid::{
    BelCoord, CellCoord, ColId, DieId, ExpandedGrid, Rect, RowId, TileCoord,
};
use prjcombine_types::bsdata::BitRectId;
use prjcombine_xilinx_bitstream::{BitRect, BitstreamGeom, Reg};

use crate::{
    chip::{Chip, ChipKind},
    defs::{self, bslots},
};

pub struct ExpandedDevice<'a> {
    pub chip: &'a Chip,
    pub egrid: ExpandedGrid<'a>,
    pub bs_geom: BitstreamGeom,
    pub holes: Vec<Rect>,
    pub clkv_frame: usize,
    pub spine_frame: usize,
    pub term_w_frame: usize,
    pub term_e_frame: usize,
    pub col_frame: EntityVec<ColId, usize>,
    pub bram_frame: EntityPartVec<ColId, usize>,
}

impl ExpandedDevice<'_> {
    pub fn is_in_hole(&self, cell: CellCoord) -> bool {
        for hole in &self.holes {
            if hole.contains(cell) {
                return true;
            }
        }
        false
    }

    pub fn btile_main(&self, cell: CellCoord) -> BitRect {
        let (width, height) = if self.chip.kind.is_virtex2() {
            (22, 80)
        } else {
            (19, 64)
        };
        let bit = 16 + height * cell.row.to_idx();
        BitRect::Main(
            DieId::from_idx(0),
            self.col_frame[cell.col],
            width,
            bit,
            height,
            false,
        )
    }

    pub fn btile_bram(&self, cell: CellCoord) -> BitRect {
        let (width, height, height_single) = if self.chip.kind.is_virtex2() {
            (64, 80 * 4, 80)
        } else {
            (19 * 4, 64 * 4, 64)
        };
        let bit = 16 + height_single * cell.row.to_idx();
        BitRect::Main(
            DieId::from_idx(0),
            self.bram_frame[cell.col],
            width,
            bit,
            height,
            false,
        )
    }

    pub fn btile_term_h(&self, cell: CellCoord) -> BitRect {
        let (width, height) = if self.chip.kind.is_virtex2() {
            (4, 80)
        } else {
            (2, 64)
        };
        let bit = 16 + height * cell.row.to_idx();
        let frame = if cell.col == self.chip.col_w() {
            self.term_w_frame
        } else if cell.col == self.chip.col_e() {
            self.term_e_frame
        } else {
            unreachable!()
        };
        BitRect::Main(DieId::from_idx(0), frame, width, bit, height, false)
    }

    pub fn btile_term_v(&self, cell: CellCoord) -> BitRect {
        let (width, height) = if self.chip.kind.is_virtex2() {
            (22, 80)
        } else {
            (19, 64)
        };
        let bit = if cell.row == self.chip.row_s() {
            if self.chip.kind.is_virtex2() {
                4
            } else if !self.chip.kind.is_spartan3a() {
                7
            } else {
                0
            }
        } else if cell.row == self.chip.row_n() {
            16 + height * self.chip.rows.len()
        } else {
            unreachable!()
        };
        BitRect::Main(
            DieId::from_idx(0),
            self.col_frame[cell.col],
            width,
            bit,
            if self.chip.kind.is_virtex2() {
                12
            } else if !self.chip.kind.is_spartan3a() {
                5
            } else {
                6
            },
            false,
        )
    }

    pub fn btile_spine(&self, row: RowId) -> BitRect {
        let (width, height) = if self.chip.kind.is_virtex2() {
            (4, 80)
        } else if self.chip.has_ll || self.chip.kind.is_spartan3a() {
            (2, 64)
        } else {
            (1, 64)
        };
        let bit = 16 + height * row.to_idx();
        BitRect::Main(
            DieId::from_idx(0),
            self.spine_frame,
            width,
            bit,
            height,
            false,
        )
    }

    pub fn btile_clkv(&self, cell: CellCoord) -> BitRect {
        assert!(!self.chip.kind.is_virtex2());
        let bit = 16 + 64 * cell.row.to_idx();
        BitRect::Main(
            DieId::from_idx(0),
            self.clkv_frame + if cell.col < self.chip.col_clk { 0 } else { 1 },
            1,
            bit,
            64,
            false,
        )
    }

    pub fn btile_spine_end(&self, row: RowId) -> BitRect {
        let (width, height) = if self.chip.kind.is_virtex2() {
            (4, 80)
        } else if self.chip.has_ll || self.chip.kind.is_spartan3a() {
            (2, 64)
        } else {
            (1, 64)
        };
        let bit = if row == self.chip.row_s() {
            0
        } else if row == self.chip.row_n() {
            16 + height * self.chip.rows.len()
        } else {
            unreachable!()
        };
        BitRect::Main(DieId::from_idx(0), self.spine_frame, width, bit, 16, false)
    }

    pub fn btile_llv_s(&self, col: ColId) -> BitRect {
        assert_eq!(self.chip.kind, ChipKind::Spartan3E);
        assert!(self.chip.has_ll);
        let bit = self.chip.rows_hclk.len() / 2;
        BitRect::Main(DieId::from_idx(0), self.col_frame[col], 19, bit, 1, false)
    }

    pub fn btile_llv_n(&self, col: ColId) -> BitRect {
        assert_eq!(self.chip.kind, ChipKind::Spartan3E);
        assert!(self.chip.has_ll);
        let bit = 16 + self.chip.rows.len() * 64 + 11 + self.chip.rows_hclk.len() / 2;
        BitRect::Main(DieId::from_idx(0), self.col_frame[col], 19, bit, 2, false)
    }

    pub fn btile_llv(&self, col: ColId) -> BitRect {
        assert!(self.chip.kind.is_spartan3a());
        assert!(self.chip.has_ll);
        let bit = 16 + self.chip.rows.len() * 64 + 8;
        BitRect::Main(DieId::from_idx(0), self.col_frame[col], 19, bit, 3, false)
    }

    pub fn btile_hclk(&self, cell: CellCoord) -> BitRect {
        let (width, height) = if self.chip.kind.is_virtex2() {
            (22, 80)
        } else {
            (19, 64)
        };
        let hclk_idx = self
            .chip
            .rows_hclk
            .iter()
            .position(|&(hrow, _, _)| hrow == cell.row)
            .unwrap();
        let bit = if cell.row <= self.chip.row_mid() {
            if self.chip.kind.is_spartan3a() {
                11 + hclk_idx
            } else {
                hclk_idx
            }
        } else {
            let hclk_idx = self.chip.rows_hclk.len() - hclk_idx - 1;
            if self.chip.kind.is_spartan3a() || self.chip.has_ll {
                16 + height * self.chip.rows.len() + 11 + hclk_idx
            } else {
                16 + height * self.chip.rows.len() + 12 + hclk_idx
            }
        };
        BitRect::Main(
            DieId::from_idx(0),
            self.col_frame[cell.col],
            width,
            bit,
            1,
            false,
        )
    }

    pub fn tile_bits(&self, tcrd: TileCoord) -> EntityVec<BitRectId, BitRect> {
        let col = tcrd.col;
        let row = tcrd.row;
        let tile = &self[tcrd];
        let tcls = &self.db.tile_classes[tile.class];
        if tcls.bitrects.is_empty() {
            EntityVec::new()
        } else if tcrd.slot == defs::tslots::GLOBAL {
            if self.chip.kind.is_spartan3a() {
                EntityVec::from_iter([
                    BitRect::Reg(tcrd.die, Reg::Cor1),
                    BitRect::Reg(tcrd.die, Reg::Cor2),
                    BitRect::Reg(tcrd.die, Reg::Ctl0),
                    BitRect::Reg(tcrd.die, Reg::CclkFrequency),
                    BitRect::Reg(tcrd.die, Reg::HcOpt),
                    BitRect::Reg(tcrd.die, Reg::Powerdown),
                    BitRect::Reg(tcrd.die, Reg::PuGwe),
                    BitRect::Reg(tcrd.die, Reg::PuGts),
                    BitRect::Reg(tcrd.die, Reg::Mode),
                    BitRect::Reg(tcrd.die, Reg::General1),
                    BitRect::Reg(tcrd.die, Reg::General2),
                    BitRect::Reg(tcrd.die, Reg::SeuOpt),
                ])
            } else {
                EntityVec::from_iter([
                    BitRect::Reg(tcrd.die, Reg::Cor0),
                    BitRect::Reg(tcrd.die, Reg::Ctl0),
                ])
            }
        } else if tcls.bels.contains_id(defs::bslots::BRAM) {
            EntityVec::from_iter([
                self.btile_main(tcrd.delta(0, 0)),
                self.btile_main(tcrd.delta(0, 1)),
                self.btile_main(tcrd.delta(0, 2)),
                self.btile_main(tcrd.delta(0, 3)),
                self.btile_bram(tcrd.cell),
            ])
        } else if tcls.bels.contains_id(defs::bslots::PCILOGICSE) {
            // CLK_W_*, CLK_E_*
            EntityVec::from_iter([
                self.btile_main(tcrd.delta(0, -1)),
                self.btile_main(tcrd.delta(0, 0)),
                self.btile_term_h(tcrd.delta(0, -2)),
                self.btile_term_h(tcrd.delta(0, -1)),
                self.btile_term_h(tcrd.delta(0, 0)),
                self.btile_term_h(tcrd.delta(0, 1)),
            ])
        } else if tcls.bels.contains_id(defs::bslots::BUFGMUX[0]) {
            // CLK_S_*, CLK_N_*
            EntityVec::from_iter([self.btile_spine(row), self.btile_spine_end(row)])
        } else if !self.chip.kind.is_virtex2() && tile.class == defs::spartan3::tcls::CLKC_50A {
            EntityVec::from_iter([self.btile_spine(row - 1)])
        } else if !self.chip.kind.is_virtex2()
            && matches!(
                tile.class,
                defs::spartan3::tcls::CLKQC_S3 | defs::spartan3::tcls::CLKQC_S3E
            )
        {
            EntityVec::from_iter([
                self.btile_clkv(tcrd.delta(0, -1)),
                self.btile_clkv(tcrd.delta(0, 0)),
            ])
        } else if tcrd.slot == defs::tslots::HROW {
            if row == self.chip.row_s() + 1 {
                EntityVec::from_iter([
                    self.btile_spine_end(row - 1),
                    self.btile_spine(row - 1),
                    self.btile_spine(row),
                    self.btile_spine(row + 1),
                ])
            } else if row == self.chip.row_n() {
                EntityVec::from_iter([
                    self.btile_spine(row - 2),
                    self.btile_spine(row - 1),
                    self.btile_spine(row),
                    self.btile_spine_end(row),
                ])
            } else {
                EntityVec::from_iter([
                    self.btile_spine(row - 2),
                    self.btile_spine(row - 1),
                    self.btile_spine(row),
                    self.btile_spine(row + 1),
                ])
            }
        } else if tcrd.slot == defs::tslots::HCLK {
            EntityVec::from_iter([self.btile_hclk(tcrd.cell)])
        } else if tcrd.slot == defs::tslots::IOB {
            if col == self.chip.col_w() || col == self.chip.col_e() {
                EntityVec::from_iter(
                    self.tile_cells(tcrd)
                        .map(|(_, cell)| self.btile_term_h(cell)),
                )
            } else {
                EntityVec::from_iter(
                    self.tile_cells(tcrd)
                        .map(|(_, cell)| self.btile_term_v(cell)),
                )
            }
        } else if self.chip.kind.is_virtex2()
            && matches!(
                tile.class,
                defs::virtex2::tcls::TERM_W | defs::virtex2::tcls::TERM_E
            )
        {
            EntityVec::from_iter([self.btile_term_h(tcrd.cell)])
        } else if self.chip.kind.is_virtex2()
            && matches!(
                tile.class,
                defs::virtex2::tcls::TERM_S
                    | defs::virtex2::tcls::TERM_N
                    | defs::virtex2::tcls::DCMCONN_S
                    | defs::virtex2::tcls::DCMCONN_N
            )
        {
            EntityVec::from_iter([self.btile_term_v(tcrd.cell)])
        } else if tcls.bels.contains_id(defs::bslots::DCM) {
            if self.chip.kind.is_virtex2() {
                EntityVec::from_iter([self.btile_main(tcrd.cell), self.btile_term_v(tcrd.cell)])
            } else if self.chip.kind == ChipKind::Spartan3 {
                EntityVec::from_iter([self.btile_main(tcrd.cell)])
            } else {
                match tile.class {
                    defs::spartan3::tcls::DCM_S3E_SW | defs::spartan3::tcls::DCM_S3E_EN => {
                        EntityVec::from_iter([
                            self.btile_main(tcrd.delta(0, 0)),
                            self.btile_main(tcrd.delta(0, 1)),
                            self.btile_main(tcrd.delta(0, 2)),
                            self.btile_main(tcrd.delta(0, 3)),
                            self.btile_main(tcrd.delta(-3, 0)),
                            self.btile_main(tcrd.delta(-3, 1)),
                            self.btile_main(tcrd.delta(-3, 2)),
                            self.btile_main(tcrd.delta(-3, 3)),
                        ])
                    }
                    defs::spartan3::tcls::DCM_S3E_SE | defs::spartan3::tcls::DCM_S3E_WN => {
                        EntityVec::from_iter([
                            self.btile_main(tcrd.delta(0, 0)),
                            self.btile_main(tcrd.delta(0, 1)),
                            self.btile_main(tcrd.delta(0, 2)),
                            self.btile_main(tcrd.delta(0, 3)),
                            self.btile_main(tcrd.delta(3, 0)),
                            self.btile_main(tcrd.delta(3, 1)),
                            self.btile_main(tcrd.delta(3, 2)),
                            self.btile_main(tcrd.delta(3, 3)),
                        ])
                    }
                    defs::spartan3::tcls::DCM_S3E_NW | defs::spartan3::tcls::DCM_S3E_ES => {
                        EntityVec::from_iter([
                            self.btile_main(tcrd.delta(0, 0)),
                            self.btile_main(tcrd.delta(0, -3)),
                            self.btile_main(tcrd.delta(0, -2)),
                            self.btile_main(tcrd.delta(0, -1)),
                            self.btile_main(tcrd.delta(-3, -3)),
                            self.btile_main(tcrd.delta(-3, -2)),
                            self.btile_main(tcrd.delta(-3, -1)),
                            self.btile_main(tcrd.delta(-3, 0)),
                        ])
                    }
                    defs::spartan3::tcls::DCM_S3E_NE | defs::spartan3::tcls::DCM_S3E_WS => {
                        EntityVec::from_iter([
                            self.btile_main(tcrd.delta(0, 0)),
                            self.btile_main(tcrd.delta(0, -3)),
                            self.btile_main(tcrd.delta(0, -2)),
                            self.btile_main(tcrd.delta(0, -1)),
                            self.btile_main(tcrd.delta(3, -3)),
                            self.btile_main(tcrd.delta(3, -2)),
                            self.btile_main(tcrd.delta(3, -1)),
                            self.btile_main(tcrd.delta(3, 0)),
                        ])
                    }
                    _ => unreachable!(),
                }
            }
        } else if !self.chip.kind.is_virtex2() && tile.class == defs::spartan3::tcls::RANDOR_FC {
            EntityVec::from_iter([self.btile_term_v(tcrd.cell)])
        } else if tcrd.slot == defs::tslots::BEL
            && (col == self.chip.col_w() || col == self.chip.col_e())
            && (row == self.chip.row_s() || row == self.chip.row_n())
        {
            // CNR
            if self.chip.kind.is_virtex2() {
                EntityVec::from_iter([self.btile_term_h(tcrd.cell), self.btile_term_v(tcrd.cell)])
            } else {
                EntityVec::from_iter([self.btile_term_h(tcrd.cell)])
            }
        } else if tcrd.slot == defs::tslots::RANDOR {
            EntityVec::from_iter([self.btile_main(tcrd.cell)])
        } else if self.chip.kind.is_virtex2() && tile.class == defs::virtex2::tcls::PPC_TERM_N {
            EntityVec::from_iter([self.btile_main(tcrd.delta(0, 1))])
        } else if self.chip.kind.is_virtex2() && tile.class == defs::virtex2::tcls::PPC_TERM_S {
            EntityVec::from_iter([self.btile_main(tcrd.delta(0, -1))])
        } else if tcls.bels.contains_id(defs::bslots::LLV) {
            if self.chip.kind == ChipKind::Spartan3E {
                EntityVec::from_iter([self.btile_llv_s(col), self.btile_llv_n(col)])
            } else {
                EntityVec::from_iter([self.btile_llv(col)])
            }
        } else if tcls.bels.contains_id(defs::bslots::LLH) {
            EntityVec::from_iter([self.btile_spine(row)])
        } else {
            EntityVec::from_iter(self.tile_cells(tcrd).map(|(_, cell)| self.btile_main(cell)))
        }
    }

    pub fn bel_carry_prev(&self, bcrd: BelCoord) -> Option<BelCoord> {
        if bslots::SLICE.contains(bcrd.slot) {
            todo!()
        } else if matches!(bcrd.slot, bslots::MULT | bslots::DSP) {
            let mut bcrd = bcrd;
            loop {
                if let Some(cell) = self.cell_delta(bcrd.cell, 0, -4) {
                    bcrd.cell = cell;
                } else {
                    return None;
                }
                if self.has_bel(bcrd) {
                    return Some(bcrd);
                }
            }
        } else {
            panic!("not a carry-chain bel: {}", bcrd.to_string(self.db))
        }
    }
}

impl<'a> std::ops::Deref for ExpandedDevice<'a> {
    type Target = ExpandedGrid<'a>;

    fn deref(&self) -> &Self::Target {
        &self.egrid
    }
}
