use prjcombine_interconnect::grid::{
    CellCoord, ColId, DieId, ExpandedGrid, Rect, RowId, TileCoord,
};
use prjcombine_xilinx_bitstream::{BitTile, BitstreamGeom};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::chip::{Chip, ChipKind};

pub struct ExpandedDevice<'a> {
    pub chip: &'a Chip,
    pub egrid: ExpandedGrid<'a>,
    pub bs_geom: BitstreamGeom,
    pub holes: Vec<Rect>,
    pub clkv_frame: usize,
    pub spine_frame: usize,
    pub lterm_frame: usize,
    pub rterm_frame: usize,
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

    pub fn btile_main(&self, cell: CellCoord) -> BitTile {
        let (width, height) = if self.chip.kind.is_virtex2() {
            (22, 80)
        } else {
            (19, 64)
        };
        let bit = 16 + height * cell.row.to_idx();
        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[cell.col],
            width,
            bit,
            height,
            false,
        )
    }

    pub fn btile_bram(&self, cell: CellCoord) -> BitTile {
        let (width, height, height_single) = if self.chip.kind.is_virtex2() {
            (64, 80 * 4, 80)
        } else {
            (19 * 4, 64 * 4, 64)
        };
        let bit = 16 + height_single * cell.row.to_idx();
        BitTile::Main(
            DieId::from_idx(0),
            self.bram_frame[cell.col],
            width,
            bit,
            height,
            false,
        )
    }

    pub fn btile_lrterm(&self, cell: CellCoord) -> BitTile {
        let (width, height) = if self.chip.kind.is_virtex2() {
            (4, 80)
        } else {
            (2, 64)
        };
        let bit = 16 + height * cell.row.to_idx();
        let frame = if cell.col == self.chip.col_w() {
            self.lterm_frame
        } else if cell.col == self.chip.col_e() {
            self.rterm_frame
        } else {
            unreachable!()
        };
        BitTile::Main(DieId::from_idx(0), frame, width, bit, height, false)
    }

    pub fn btile_btterm(&self, cell: CellCoord) -> BitTile {
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
        BitTile::Main(
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

    pub fn btile_spine(&self, row: RowId) -> BitTile {
        let (width, height) = if self.chip.kind.is_virtex2() {
            (4, 80)
        } else if self.chip.has_ll || self.chip.kind.is_spartan3a() {
            (2, 64)
        } else {
            (1, 64)
        };
        let bit = 16 + height * row.to_idx();
        BitTile::Main(
            DieId::from_idx(0),
            self.spine_frame,
            width,
            bit,
            height,
            false,
        )
    }

    pub fn btile_clkv(&self, cell: CellCoord) -> BitTile {
        assert!(!self.chip.kind.is_virtex2());
        let bit = 16 + 64 * cell.row.to_idx();
        BitTile::Main(
            DieId::from_idx(0),
            self.clkv_frame + if cell.col < self.chip.col_clk { 0 } else { 1 },
            1,
            bit,
            64,
            false,
        )
    }

    pub fn btile_btspine(&self, row: RowId) -> BitTile {
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
        BitTile::Main(DieId::from_idx(0), self.spine_frame, width, bit, 16, false)
    }

    pub fn btile_llv_b(&self, col: ColId) -> BitTile {
        assert_eq!(self.chip.kind, ChipKind::Spartan3E);
        assert!(self.chip.has_ll);
        let bit = self.chip.rows_hclk.len() / 2;
        BitTile::Main(DieId::from_idx(0), self.col_frame[col], 19, bit, 1, false)
    }

    pub fn btile_llv_t(&self, col: ColId) -> BitTile {
        assert_eq!(self.chip.kind, ChipKind::Spartan3E);
        assert!(self.chip.has_ll);
        let bit = 16 + self.chip.rows.len() * 64 + 11 + self.chip.rows_hclk.len() / 2;
        BitTile::Main(DieId::from_idx(0), self.col_frame[col], 19, bit, 2, false)
    }

    pub fn btile_llv(&self, col: ColId) -> BitTile {
        assert!(self.chip.kind.is_spartan3a());
        assert!(self.chip.has_ll);
        let bit = 16 + self.chip.rows.len() * 64 + 8;
        BitTile::Main(DieId::from_idx(0), self.col_frame[col], 19, bit, 3, false)
    }

    pub fn btile_hclk(&self, cell: CellCoord) -> BitTile {
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
        BitTile::Main(
            DieId::from_idx(0),
            self.col_frame[cell.col],
            width,
            bit,
            1,
            false,
        )
    }

    pub fn tile_bits(&self, tcrd: TileCoord) -> Vec<BitTile> {
        let col = tcrd.col;
        let row = tcrd.row;
        let tile = &self[tcrd];
        let kind = self.db.tile_classes.key(tile.class).as_str();
        if kind.starts_with("BRAM") {
            vec![
                self.btile_main(tcrd.delta(0, 0)),
                self.btile_main(tcrd.delta(0, 1)),
                self.btile_main(tcrd.delta(0, 2)),
                self.btile_main(tcrd.delta(0, 3)),
                self.btile_bram(tcrd.cell),
            ]
        } else if kind.starts_with("CLKB") || kind.starts_with("CLKT") {
            vec![self.btile_spine(row), self.btile_btspine(row)]
        } else if kind.starts_with("CLKL") || kind.starts_with("CLKR") {
            vec![
                self.btile_main(tcrd.delta(0, -1)),
                self.btile_main(tcrd.delta(0, 0)),
                self.btile_lrterm(tcrd.delta(0, -2)),
                self.btile_lrterm(tcrd.delta(0, -1)),
                self.btile_lrterm(tcrd.delta(0, 0)),
                self.btile_lrterm(tcrd.delta(0, 1)),
            ]
        } else if kind == "CLKC_50A" {
            vec![self.btile_spine(row - 1)]
        } else if kind.starts_with("GCLKVM") {
            vec![
                self.btile_clkv(tcrd.delta(0, -1)),
                self.btile_clkv(tcrd.delta(0, 0)),
            ]
        } else if kind.starts_with("GCLKC") {
            if row == self.chip.row_s() + 1 {
                vec![
                    self.btile_btspine(row - 1),
                    self.btile_spine(row - 1),
                    self.btile_spine(row),
                    self.btile_spine(row + 1),
                ]
            } else if row == self.chip.row_n() {
                vec![
                    self.btile_spine(row - 2),
                    self.btile_spine(row - 1),
                    self.btile_spine(row),
                    self.btile_btspine(row),
                ]
            } else {
                vec![
                    self.btile_spine(row - 2),
                    self.btile_spine(row - 1),
                    self.btile_spine(row),
                    self.btile_spine(row + 1),
                ]
            }
        } else if kind.starts_with("GCLKH") {
            vec![self.btile_hclk(tcrd.cell)]
        } else if kind.starts_with("IOBS") {
            if col == self.chip.col_w() || col == self.chip.col_e() {
                Vec::from_iter(
                    self.tile_cells(tcrd)
                        .map(|(_, cell)| self.btile_lrterm(cell)),
                )
            } else {
                Vec::from_iter(
                    self.tile_cells(tcrd)
                        .map(|(_, cell)| self.btile_btterm(cell)),
                )
            }
        } else if matches!(kind, "TERM.W" | "TERM.E") {
            vec![self.btile_lrterm(tcrd.cell)]
        } else if kind.starts_with("DCMCONN") || matches!(kind, "TERM.S" | "TERM.N") {
            vec![self.btile_btterm(tcrd.cell)]
        } else if kind.starts_with("DCM") {
            if self.chip.kind.is_virtex2() {
                vec![self.btile_main(tcrd.cell), self.btile_btterm(tcrd.cell)]
            } else if self.chip.kind == ChipKind::Spartan3 {
                vec![self.btile_main(tcrd.cell)]
            } else {
                match kind {
                    "DCM.S3E.BL" | "DCM.S3E.RT" => vec![
                        self.btile_main(tcrd.delta(0, 0)),
                        self.btile_main(tcrd.delta(0, 1)),
                        self.btile_main(tcrd.delta(0, 2)),
                        self.btile_main(tcrd.delta(0, 3)),
                        self.btile_main(tcrd.delta(-3, 0)),
                        self.btile_main(tcrd.delta(-3, 1)),
                        self.btile_main(tcrd.delta(-3, 2)),
                        self.btile_main(tcrd.delta(-3, 3)),
                    ],
                    "DCM.S3E.BR" | "DCM.S3E.LT" => vec![
                        self.btile_main(tcrd.delta(0, 0)),
                        self.btile_main(tcrd.delta(0, 1)),
                        self.btile_main(tcrd.delta(0, 2)),
                        self.btile_main(tcrd.delta(0, 3)),
                        self.btile_main(tcrd.delta(3, 0)),
                        self.btile_main(tcrd.delta(3, 1)),
                        self.btile_main(tcrd.delta(3, 2)),
                        self.btile_main(tcrd.delta(3, 3)),
                    ],
                    "DCM.S3E.TL" | "DCM.S3E.RB" => vec![
                        self.btile_main(tcrd.delta(0, 0)),
                        self.btile_main(tcrd.delta(0, -3)),
                        self.btile_main(tcrd.delta(0, -2)),
                        self.btile_main(tcrd.delta(0, -1)),
                        self.btile_main(tcrd.delta(-3, -3)),
                        self.btile_main(tcrd.delta(-3, -2)),
                        self.btile_main(tcrd.delta(-3, -1)),
                        self.btile_main(tcrd.delta(-3, 0)),
                    ],
                    "DCM.S3E.TR" | "DCM.S3E.LB" => vec![
                        self.btile_main(tcrd.delta(0, 0)),
                        self.btile_main(tcrd.delta(0, -3)),
                        self.btile_main(tcrd.delta(0, -2)),
                        self.btile_main(tcrd.delta(0, -1)),
                        self.btile_main(tcrd.delta(3, -3)),
                        self.btile_main(tcrd.delta(3, -2)),
                        self.btile_main(tcrd.delta(3, -1)),
                        self.btile_main(tcrd.delta(3, 0)),
                    ],
                    _ => unreachable!(),
                }
            }
        } else if kind.starts_with("LL.")
            || kind.starts_with("LR.")
            || kind.starts_with("UL.") | kind.starts_with("UR.")
        {
            if self.chip.kind.is_virtex2() {
                vec![self.btile_lrterm(tcrd.cell), self.btile_btterm(tcrd.cell)]
            } else {
                vec![self.btile_lrterm(tcrd.cell)]
            }
        } else if matches!(kind, "RANDOR" | "RANDOR_INIT") {
            vec![self.btile_main(tcrd.cell)]
        } else if kind == "PPC.N" {
            vec![self.btile_main(tcrd.delta(0, 1))]
        } else if kind == "PPC.S" {
            vec![self.btile_main(tcrd.delta(0, -1))]
        } else if kind.starts_with("LLV") {
            if self.chip.kind == ChipKind::Spartan3E {
                vec![self.btile_llv_b(col), self.btile_llv_t(col)]
            } else {
                vec![self.btile_llv(col)]
            }
        } else if kind.starts_with("LLH") {
            vec![self.btile_spine(row)]
        } else {
            Vec::from_iter(self.tile_cells(tcrd).map(|(_, cell)| self.btile_main(cell)))
        }
    }
}

impl<'a> std::ops::Deref for ExpandedDevice<'a> {
    type Target = ExpandedGrid<'a>;

    fn deref(&self) -> &Self::Target {
        &self.egrid
    }
}
