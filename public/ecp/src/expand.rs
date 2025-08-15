use std::collections::BTreeMap;

use prjcombine_interconnect::{
    db::IntDb,
    dir::{Dir, DirH, DirHV, DirV},
    grid::{CellCoord, ColId, DieId, ExpandedGrid, Rect, RowId},
};
use unnamed_entity::{EntityId, EntityVec};

use crate::{
    bels,
    chip::{Chip, ChipKind, IoGroupKind, MachXo2Kind, PllLoc, RowKind, SpecialLocKey},
    expanded::ExpandedDevice,
    regions, tslots,
};

struct Expander<'a, 'b> {
    chip: &'b Chip,
    die: DieId,
    egrid: &'a mut ExpandedGrid<'b>,
    bel_holes: Vec<Rect>,
    dqs: BTreeMap<CellCoord, CellCoord>,
}

impl Expander<'_, '_> {
    fn is_in_bel_hole(&self, cell: CellCoord) -> bool {
        for hole in &self.bel_holes {
            if hole.contains(cell) {
                return true;
            }
        }
        false
    }

    fn fill_plc(&mut self) {
        for (row, rd) in &self.chip.rows {
            let tcls = match rd.kind {
                RowKind::Plc => "PLC",
                RowKind::Fplc => "FPLC",
                _ => continue,
            };
            for cell in self.egrid.row(self.die, row) {
                if (cell.col == self.chip.col_w() || cell.col == self.chip.col_e())
                    && self.chip.kind != ChipKind::Crosslink
                {
                    continue;
                }
                if self.is_in_bel_hole(cell) {
                    continue;
                }
                self.egrid.add_tile_single(cell, "INT_PLC");
                self.egrid.add_tile_single(cell, tcls);
            }
        }
    }

    fn fill_config_scm(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Config];
        let mut cells = cell.cells_e(12);
        cells.push(cell.with_cr(self.chip.col_clk, self.chip.row_clk - 5));
        self.egrid.add_tile(cell, "CONFIG", &cells);
        for dx in 0..12 {
            self.egrid.add_tile_single(cell.delta(dx, 0), "INT_EBR");
        }
        self.bel_holes.push(cell.rect(12, 1));
    }

    fn fill_config_ecp(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Config];
        self.bel_holes.push(cell.rect(4, 1));
        if cell.row == self.chip.row_clk {
            self.egrid.add_tile_e(cell, "CONFIG_S", 4);
        } else {
            let tcells: [_; 5] = core::array::from_fn(|i| {
                if i < 4 {
                    cell.delta(i as i32, 0)
                } else {
                    cell.with_row(self.chip.row_clk)
                }
            });
            self.egrid.add_tile(cell, "CONFIG_L", &tcells);
        }
    }

    fn fill_config_ecp2(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Config];
        self.bel_holes.push(cell.rect(6, 1));
        self.egrid.add_tile(
            cell,
            "CONFIG",
            &[
                cell,
                cell.delta(1, 0),
                cell.delta(2, 0).with_row(self.chip.row_clk),
            ],
        );
    }

    fn fill_config_xp2(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Osc];
        self.egrid.add_tile_single(cell, "OSC");
        let cell = self.chip.special_loc[&SpecialLocKey::Config];
        self.egrid.add_tile_single(cell, "CONFIG");
    }

    fn fill_config_ecp3(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Config];
        self.bel_holes.push(cell.rect(12, 1));
        let mut tcells = cell.cells_e(13);
        tcells.push(cell.with_cr(self.chip.col_clk - 2, self.chip.row_clk));
        self.egrid.add_tile(cell, "CONFIG", &tcells);

        let cell = CellCoord::new(self.die, self.chip.col_w(), self.chip.row_s());
        self.egrid.add_tile_e(cell, "TEST_SW", 3);
        let cell = CellCoord::new(self.die, self.chip.col_e(), self.chip.row_s());
        self.egrid.add_tile_we(cell, "TEST_SE", 2, 3);
        let cell = CellCoord::new(self.die, self.chip.col_w(), self.chip.row_n());
        self.egrid.add_tile_e(cell, "TEST_NW", 2);
        let cell = CellCoord::new(self.die, self.chip.col_e(), self.chip.row_n());
        self.egrid.add_tile_we(cell, "TEST_NE", 1, 2);
    }

    fn fill_pll_scm(&mut self) {
        let cell = CellCoord::new(self.die, self.chip.col_w(), self.chip.row_s());
        self.egrid.add_tile(
            cell,
            "PLL_SW",
            &[
                cell,
                cell.delta(1, 0),
                cell.delta(2, 0),
                cell.delta(3, 0),
                cell.delta(0, 1),
                cell.delta(0, 2),
            ],
        );

        let cell = CellCoord::new(self.die, self.chip.col_e(), self.chip.row_s());
        self.egrid.add_tile(
            cell,
            "PLL_SE",
            &[
                cell,
                cell.delta(-1, 0),
                cell.delta(-2, 0),
                cell.delta(-3, 0),
                cell.delta(0, 1),
                cell.delta(0, 2),
            ],
        );

        let cell = CellCoord::new(self.die, self.chip.col_w(), self.chip.row_n() - 12);
        self.egrid
            .add_tile(cell, "PLL_NW", &[cell, cell.delta(1, 0), cell.delta(0, -1)]);

        let cell = CellCoord::new(self.die, self.chip.col_e(), self.chip.row_n() - 12);
        self.egrid.add_tile(
            cell,
            "PLL_NE",
            &[cell, cell.delta(-1, 0), cell.delta(0, -1)],
        );
    }

    fn fill_pll_ecp(&mut self) {
        for (&loc, &cell) in &self.chip.special_loc {
            let SpecialLocKey::Pll(loc) = loc else {
                continue;
            };
            match loc.quad.h {
                DirH::W => {
                    self.egrid.add_tile_single(cell, "PLL_W");
                    self.bel_holes.push(cell.rect(5, 1));
                    for i in 1..5 {
                        self.egrid.add_tile_single(cell.delta(i, 0), "INT_PLL");
                    }
                }
                DirH::E => {
                    self.egrid.add_tile_single(cell, "PLL_E");
                    self.bel_holes.push(cell.delta(-4, 0).rect(5, 1));
                    for i in 1..5 {
                        self.egrid.add_tile_single(cell.delta(-i, 0), "INT_PLL");
                    }
                }
            }
        }
    }

    fn fill_pll_xp(&mut self) {
        let hole_width: i32 = if self.chip.col_clk.to_idx().is_multiple_of(2) {
            8
        } else {
            7
        };
        for (&loc, &cell) in &self.chip.special_loc {
            let SpecialLocKey::Pll(loc) = loc else {
                continue;
            };
            match loc.quad.h {
                DirH::W => {
                    self.egrid.add_tile_single(cell, "PLL_W");
                    self.bel_holes.push(cell.rect(hole_width as usize, 1));
                    for i in 2..hole_width {
                        self.egrid.add_tile_single(cell.delta(i, 0), "INT_PLL");
                    }
                }
                DirH::E => {
                    self.egrid.add_tile_single(cell, "PLL_E");
                    self.bel_holes.push(
                        cell.delta(-(hole_width - 1), 0)
                            .rect(hole_width as usize, 1),
                    );
                    for i in 2..hole_width {
                        self.egrid.add_tile_single(cell.delta(-i, 0), "INT_PLL");
                    }
                }
            }
        }
    }

    fn fill_pll_ecp2(&mut self) {
        for (&loc, &cell) in &self.chip.special_loc {
            let SpecialLocKey::Pll(loc) = loc else {
                continue;
            };
            if loc.quad.v == DirV::S && loc.idx == 0 {
                match loc.quad.h {
                    DirH::W => {
                        self.bel_holes.push(cell.rect(7, 1));
                        self.egrid.add_tile_e(cell, "PLL_W", 4);
                    }
                    DirH::E => {
                        self.bel_holes.push(cell.delta(-6, 0).rect(7, 1));
                        self.egrid.add_tile(
                            cell,
                            "PLL_E",
                            &[
                                cell,
                                cell.delta(-1, 0),
                                cell.delta(-2, 0),
                                cell.delta(-3, 0),
                            ],
                        );
                    }
                }
            } else {
                self.bel_holes.push(cell.rect(1, 1));
                self.egrid.add_tile_n(
                    cell,
                    match loc.quad.h {
                        DirH::W => "SPLL_W",
                        DirH::E => "SPLL_E",
                    },
                    2,
                );
            }
        }
    }

    fn fill_pll_xp2(&mut self) {
        for (&loc, &cell) in &self.chip.special_loc {
            let SpecialLocKey::Pll(loc) = loc else {
                continue;
            };
            let kind = match loc.quad.v {
                DirV::S => "PLL_S",
                DirV::N => "PLL_N",
            };
            match loc.quad.h {
                DirH::W => {
                    self.egrid.add_tile(cell, kind, &[cell, cell.delta(1, 0)]);
                }
                DirH::E => {
                    self.egrid.add_tile(cell, kind, &[cell, cell.delta(-1, 0)]);
                }
            }
        }
    }

    fn fill_pll_ecp3(&mut self) {
        for (&loc, &cell) in &self.chip.special_loc {
            let SpecialLocKey::Pll(loc) = loc else {
                continue;
            };
            match loc.quad.h {
                DirH::W => {
                    if cell.row == self.chip.row_clk {
                        let mut tcells = Vec::from_iter((0..15).map(|i| cell.delta(i, 0)));
                        tcells.push(cell.delta(-1, -3));
                        tcells.push(cell.delta(-1, 1));
                        tcells.push(cell.delta(-1, 3));
                        self.egrid.add_tile(
                            cell,
                            if self.chip.kind == ChipKind::Ecp3A {
                                "PLL_DLL_A_W"
                            } else {
                                "PLL_DLL_W"
                            },
                            &tcells,
                        );
                        self.bel_holes.push(cell.rect(15, 1));
                    } else {
                        let mut tcells = Vec::from_iter((0..12).map(|i| cell.delta(i, 0)));
                        tcells.push(cell.delta(-1, -3));
                        self.egrid.add_tile(
                            cell,
                            if self.chip.kind == ChipKind::Ecp3A {
                                "PLL_A_W"
                            } else {
                                "PLL_W"
                            },
                            &tcells,
                        );
                        self.bel_holes.push(cell.rect(12, 1));
                    }
                    self.egrid.add_tile_e(cell, "IO_PLL_W", 2);
                }
                DirH::E => {
                    if cell.row == self.chip.row_clk {
                        let mut tcells = Vec::from_iter((0..15).map(|i| cell.delta(-i, 0)));
                        tcells.push(cell.delta(1, -3));
                        tcells.push(cell.delta(1, 1));
                        tcells.push(cell.delta(1, 3));
                        self.egrid.add_tile(
                            cell,
                            if self.chip.kind == ChipKind::Ecp3A {
                                "PLL_DLL_A_E"
                            } else {
                                "PLL_DLL_E"
                            },
                            &tcells,
                        );
                        self.bel_holes.push(cell.delta(-14, 0).rect(15, 1));
                    } else {
                        let mut tcells = Vec::from_iter((0..12).map(|i| cell.delta(-i, 0)));
                        tcells.push(cell.delta(1, -3));
                        self.egrid.add_tile(
                            cell,
                            if self.chip.kind == ChipKind::Ecp3A {
                                "PLL_A_E"
                            } else {
                                "PLL_E"
                            },
                            &tcells,
                        );
                        self.bel_holes.push(cell.delta(-11, 0).rect(12, 1));
                    }
                    self.egrid
                        .add_tile(cell, "IO_PLL_E", &[cell, cell.delta(-1, 0)]);
                }
            }
        }
        {
            let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_clk);
            self.bel_holes.push(cell.delta(-3, 0).rect(6, 1));
        }
    }

    fn fill_pll_machxo2(&mut self) {
        for (&loc, &cell) in &self.chip.special_loc {
            let SpecialLocKey::Pll(loc) = loc else {
                continue;
            };
            let kind = match loc.quad.h {
                DirH::W => "PLL_W",
                DirH::E => "PLL_E",
            };
            match loc.quad.h {
                DirH::W => {
                    self.egrid.add_tile(cell, kind, &[cell, cell.delta(1, 0)]);
                }
                DirH::E => {
                    self.egrid.add_tile(cell, kind, &[cell, cell.delta(-1, 0)]);
                }
            }
        }
    }

    fn fill_pll_ecp4(&mut self) {
        for hv in DirHV::DIRS {
            let cell = self.chip.special_loc[&SpecialLocKey::Pll(PllLoc::new(hv, 0))];
            let kind = format!("PLL_{hv}");
            match hv.v {
                DirV::S => {
                    self.egrid.add_tile(
                        cell,
                        &kind,
                        &[
                            cell.delta(
                                match hv.h {
                                    DirH::W => 1,
                                    DirH::E => -1,
                                },
                                0,
                            ),
                            cell,
                            cell.delta(0, 1),
                            cell.delta(0, 2),
                        ],
                    );
                }
                DirV::N => {
                    self.egrid.add_tile_sn(cell, &kind, 3, 4);
                }
            }
        }
    }

    fn fill_pll_ecp5(&mut self) {
        for hv in DirHV::DIRS {
            let Some(&cell) = self
                .chip
                .special_loc
                .get(&SpecialLocKey::Pll(PllLoc::new(hv, 0)))
            else {
                continue;
            };
            let kind = format!("PLL_{hv}");
            match hv.v {
                DirV::S => {
                    self.egrid.add_tile(
                        cell,
                        &kind,
                        &[
                            cell,
                            cell.delta(
                                match hv.h {
                                    DirH::W => 1,
                                    DirH::E => -1,
                                },
                                0,
                            ),
                        ],
                    );
                }
                DirV::N => {
                    self.egrid.add_tile(cell, &kind, &[cell, cell.delta(0, -1)]);
                }
            }
        }
    }

    fn fill_serdes_scm(&mut self) {
        let mut col_end_w = self.chip.col_w();
        let mut col_end_e = self.chip.col_e();
        for (col, cd) in &self.chip.columns {
            if col < self.chip.col_clk && cd.io_n == IoGroupKind::Serdes {
                col_end_w = col + 7;
            }
        }
        for (col, cd) in self.chip.columns.iter().rev() {
            if col >= self.chip.col_clk && cd.io_n == IoGroupKind::Serdes {
                col_end_e = col;
            }
        }
        let row = self.chip.row_n() - 12;
        self.bel_holes.push(
            CellCoord::new(self.die, self.chip.col_w(), row)
                .rect((col_end_w - self.chip.col_w()) as usize, 13),
        );
        self.bel_holes.push(
            CellCoord::new(self.die, col_end_e, row)
                .rect((self.chip.col_e() - col_end_e) as usize + 1, 13),
        );
        for cell in self.egrid.row(self.die, row) {
            if cell.col < col_end_w || cell.col >= col_end_e {
                self.egrid.add_tile_single(cell, "INT_EBR");
            }
            if self.chip.columns[cell.col].io_n == IoGroupKind::Serdes {
                let kind = if cell.col < self.chip.col_clk {
                    "SERDES_W"
                } else {
                    "SERDES_E"
                };
                self.egrid.add_tile_e(cell, kind, 7);
            }
        }
    }

    fn fill_serdes_ecp2(&mut self) {
        for cell in self.egrid.row(self.die, self.chip.row_s()) {
            if self.chip.columns[cell.col].io_s == IoGroupKind::Serdes {
                if cell.col < self.chip.col_clk {
                    self.bel_holes.push(cell.delta(-1, 0).rect(28, 8));
                    self.egrid.add_tile_single(cell.delta(-1, 7), "INT_IO_S");
                } else {
                    self.bel_holes.push(cell.rect(28, 8));
                    self.egrid.add_tile_single(cell.delta(27, 7), "INT_IO_S");
                }
                for dx in 0..27 {
                    self.egrid.add_tile_single(cell.delta(dx, 7), "INT_IO_S");
                }
                self.egrid.add_tile_e(cell.delta(0, 7), "SERDES_S", 27);
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_n()) {
            if self.chip.columns[cell.col].io_n == IoGroupKind::Serdes {
                if cell.col < self.chip.col_clk {
                    self.bel_holes.push(cell.delta(-1, -7).rect(28, 8));
                    self.egrid
                        .add_tile_single(cell.delta(-1, -7), "INT_SERDES_N");
                } else {
                    self.bel_holes.push(cell.delta(0, -7).rect(28, 8));
                    self.egrid
                        .add_tile_single(cell.delta(27, -7), "INT_SERDES_N");
                }
                for dx in 0..27 {
                    self.egrid
                        .add_tile_single(cell.delta(dx, -7), "INT_SERDES_N");
                }
                self.egrid.add_tile_e(cell.delta(0, -7), "SERDES_N", 27);
            }
        }
    }

    fn fill_serdes_ecp3(&mut self) {
        let mut first = true;
        let mut last = None;
        for cell in self.egrid.row(self.die, self.chip.row_s()) {
            if self.chip.columns[cell.col].io_s == IoGroupKind::Serdes {
                if first {
                    self.bel_holes.push(cell.delta(-3, 0).rect(3, 10));
                    for dx in 0..3 {
                        self.egrid
                            .add_tile_single(cell.delta(-3 + dx, 9), "INT_IO_S");
                    }
                    first = false;
                }
                self.bel_holes.push(cell.rect(36, 10));
                for dx in 0..36 {
                    self.egrid.add_tile_single(cell.delta(dx, 9), "INT_IO_S");
                }
                self.egrid.add_tile_e(cell.delta(0, 9), "SERDES", 36);
                last = Some(cell);
            }
        }
        let cell = last.unwrap().delta(36, 0);
        self.bel_holes.push(cell.rect(3, 10));
        for dx in 0..3 {
            self.egrid.add_tile_single(cell.delta(dx, 9), "INT_IO_S");
        }
    }

    fn fill_serdes_ecp4(&mut self) {
        for (key, kind, num_cells) in [
            (SpecialLocKey::SerdesSingle, "SERDES1", 62),
            (SpecialLocKey::SerdesDouble, "SERDES2", 120),
            (SpecialLocKey::SerdesTriple, "SERDES3", 177),
        ] {
            if let Some(&cell) = self.chip.special_loc.get(&key) {
                self.egrid.add_tile_e(cell, kind, num_cells);
            }
        }
    }

    fn fill_ebr_scm(&mut self) {
        for (row, rd) in &self.chip.rows {
            if rd.kind != RowKind::Ebr {
                continue;
            }
            for cell in self.egrid.row(self.die, row) {
                if self.is_in_bel_hole(cell) {
                    continue;
                }
                self.egrid.add_tile_single(cell, "INT_EBR");
                if self.is_in_bel_hole(cell.delta(1, 0)) {
                    continue;
                }
                if cell.col == self.chip.col_w() {
                    self.egrid.add_tile(
                        cell,
                        "MACO_W",
                        &[
                            cell,
                            cell.delta(1, 0),
                            cell.delta(2, 0),
                            cell.delta(3, 0),
                            // IO
                            cell.delta(0, -1),
                            cell.delta(0, 1),
                            // EBR
                            cell.delta(4, 0),
                            cell.delta(6, 0),
                            cell.delta(8, 0),
                            cell.delta(10, 0),
                            cell.delta(12, 0),
                            cell.delta(14, 0),
                            cell.delta(16, 0),
                            cell.delta(18, 0),
                            cell.delta(20, 0),
                            cell.delta(22, 0),
                        ],
                    );
                }
                if cell.col == self.chip.col_e() {
                    self.egrid.add_tile(
                        cell,
                        "MACO_E",
                        &[
                            cell,
                            cell.delta(-1, 0),
                            cell.delta(-2, 0),
                            cell.delta(-3, 0),
                            // IO
                            cell.delta(0, -1),
                            cell.delta(0, 1),
                            // EBR
                            cell.delta(-5, 0),
                            cell.delta(-7, 0),
                            cell.delta(-9, 0),
                            cell.delta(-11, 0),
                            cell.delta(-13, 0),
                            cell.delta(-15, 0),
                            cell.delta(-17, 0),
                            cell.delta(-19, 0),
                            cell.delta(-21, 0),
                            cell.delta(-23, 0),
                        ],
                    );
                }
                if cell.col < self.chip.col_w() + 4 || cell.col >= self.chip.col_e() - 3 {
                    continue;
                }
                if cell.col.to_idx() % 2 == self.chip.col_clk.to_idx() % 2 {
                    let kind = if cell.col < self.chip.col_clk {
                        "EBR_W"
                    } else {
                        "EBR_E"
                    };
                    self.egrid.add_tile_e(cell, kind, 2);
                    if cell.row != self.chip.row_n() - 12 {
                        if cell.col < self.chip.col_w() + 14 {
                            // nothing
                        } else if cell.col < self.chip.col_clk - 10 {
                            let cell_next = cell.delta(10, 0);
                            self.egrid.fill_conn_pair(
                                cell,
                                cell_next,
                                "PASS_EBR_W_E",
                                "PASS_EBR_W_W",
                            );
                        } else if cell.col < self.chip.col_clk {
                            let cell_next = cell.delta(10, 0);
                            self.egrid.fill_conn_pair(
                                cell,
                                cell_next,
                                "PASS_EBR_M_E",
                                "PASS_EBR_M_W",
                            );
                        } else if cell.col < self.chip.col_e() - 23 {
                            let cell_next = cell.delta(10, 0);
                            self.egrid.fill_conn_pair(
                                cell,
                                cell_next,
                                "PASS_EBR_E_E",
                                "PASS_EBR_E_W",
                            );
                        }
                    }
                }
            }
        }
    }

    fn fill_ebr_ecp(&mut self) {
        let ebr_width = match self.chip.kind {
            ChipKind::Ecp | ChipKind::Xp => 2,
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 | ChipKind::Ecp3 | ChipKind::Ecp3A => {
                3
            }
            _ => unreachable!(),
        };
        for (row, rd) in &self.chip.rows {
            if rd.kind != RowKind::Ebr {
                continue;
            }
            for cell in self.egrid.row(self.die, row) {
                if !self.egrid.has_bel(cell.bel(bels::INT)) {
                    let int_kind = if ebr_width == 3
                        && (cell.col == self.chip.col_w() || cell.col == self.chip.col_e())
                    {
                        if self.chip.kind.has_distributed_sclk_ecp3() && rd.sclk_break {
                            "INT_EBR_IO_SCLK"
                        } else {
                            "INT_EBR_IO"
                        }
                    } else {
                        if self.chip.kind.has_distributed_sclk_ecp3() && rd.sclk_break {
                            "INT_EBR_SCLK"
                        } else {
                            "INT_EBR"
                        }
                    };
                    self.egrid.add_tile_single(cell, int_kind);
                }
                if self.is_in_bel_hole(cell) {
                    continue;
                }
                if cell.col == self.chip.col_w() || cell.col == self.chip.col_e() {
                    if matches!(self.chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) && rd.sclk_break {
                        self.egrid.add_tile_single(cell, "ECLK_TAP");
                    }
                    continue;
                }
                if cell.col.to_idx() % ebr_width == self.chip.col_clk.to_idx() % ebr_width {
                    self.egrid.add_tile_e(cell, "EBR", ebr_width);
                }
            }
        }
    }

    fn fill_dsp_ecp(&mut self) {
        let dsp_width = match self.chip.kind {
            ChipKind::Ecp => 8,
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 | ChipKind::Ecp3 | ChipKind::Ecp3A => {
                9
            }
            _ => unreachable!(),
        };
        for (row, rd) in &self.chip.rows {
            if rd.kind != RowKind::Dsp {
                continue;
            }
            for cell in self.egrid.row(self.die, row) {
                if !self.egrid.has_bel(cell.bel(bels::INT)) {
                    let int_kind = if dsp_width == 9
                        && (cell.col == self.chip.col_w() || cell.col == self.chip.col_e())
                    {
                        if self.chip.kind.has_distributed_sclk_ecp3() && rd.sclk_break {
                            "INT_EBR_IO_SCLK"
                        } else {
                            "INT_EBR_IO"
                        }
                    } else {
                        if self.chip.kind.has_distributed_sclk_ecp3() && rd.sclk_break {
                            "INT_EBR_SCLK"
                        } else {
                            "INT_EBR"
                        }
                    };
                    self.egrid.add_tile_single(cell, int_kind);
                }
                if self.is_in_bel_hole(cell) {
                    continue;
                }
                if cell.col == self.chip.col_w() || cell.col == self.chip.col_e() {
                    continue;
                }
                if cell.col.to_idx() % dsp_width == 1 {
                    self.egrid.add_tile_e(cell, "DSP", dsp_width);
                }
            }
        }
    }

    fn fill_ebr_machxo2(&mut self) {
        for cell in self.egrid.row(self.die, self.chip.row_n()) {
            if self.chip.columns[cell.col].io_n == IoGroupKind::Ebr {
                self.egrid.add_tile_e(cell, "EBR_N", 3);
            }
        }
        for (row, rd) in &self.chip.rows {
            if rd.kind != RowKind::Ebr {
                continue;
            }
            for cell in self.egrid.row(self.die, row) {
                self.egrid.add_tile_single(cell, "INT_EBR");
                if self.is_in_bel_hole(cell) {
                    continue;
                }
                let is_ebr_start = if cell.col < self.chip.col_clk {
                    cell.col.to_idx() % 3 == (self.chip.col_clk.to_idx() - 1) % 3
                        && cell.col != self.chip.col_clk - 1
                } else {
                    cell.col.to_idx() % 3 == self.chip.columns.len() % 3
                };
                if is_ebr_start {
                    self.egrid.add_tile_e(cell, "EBR", 3);
                }
            }
        }
    }

    fn fill_ebr_ecp4(&mut self) {
        let (mut col_start_w, mut col_end_e) = match self.chip.kind {
            ChipKind::Ecp4 => (self.chip.col_w() + 5usize, self.chip.col_e() - 4usize),
            ChipKind::Ecp5 => (self.chip.col_w() + 3usize, self.chip.col_e() - 2usize),
            _ => unreachable!(),
        };
        let col_end_w = self.chip.col_clk - 1usize;
        let col_start_e = self.chip.col_clk + 1usize;
        while col_start_w.to_idx() % 9 != col_end_w.to_idx() % 9 {
            col_start_w += 1;
        }
        while col_start_e.to_idx() % 9 != col_end_e.to_idx() % 9 {
            col_end_e -= 1;
        }
        for (row, rd) in &self.chip.rows {
            let kind = match rd.kind {
                RowKind::Ebr => "EBR",
                RowKind::Dsp => "DSP",
                _ => continue,
            };
            for cell in self.egrid.row(self.die, row) {
                self.egrid.add_tile_single(cell, "INT_EBR");
                if (cell.col >= col_start_w
                    && cell.col < col_end_w
                    && cell.col.to_idx() % 9 == col_start_w.to_idx() % 9)
                    || (cell.col >= col_start_e
                        && cell.col < col_end_e
                        && cell.col.to_idx() % 9 == col_start_e.to_idx() % 9)
                {
                    self.egrid.add_tile_e(cell, kind, 9);
                }
            }
        }
    }

    fn fill_ebr_crosslink(&mut self) {
        let mut col_start_w = self.chip.col_w();
        let mut col_end_e = self.chip.col_e() + 1usize;
        let col_end_w = self.chip.col_clk;
        let col_start_e = self.chip.col_clk + 1usize;
        while col_start_w.to_idx() % 10 != col_end_w.to_idx() % 10 {
            col_start_w += 1;
        }
        while col_start_e.to_idx() % 10 != col_end_e.to_idx() % 10 {
            col_end_e -= 1;
        }

        for (row, rd) in &self.chip.rows {
            if rd.kind != RowKind::Ebr {
                continue;
            }
            for cell in self.egrid.row(self.die, row) {
                self.egrid.add_tile_single(cell, "INT_EBR");
                if (cell.col >= col_start_w
                    && cell.col < col_end_w
                    && cell.col.to_idx() % 10 == col_start_w.to_idx() % 10)
                    || (cell.col >= col_start_e
                        && cell.col < col_end_e
                        && cell.col.to_idx() % 10 == col_start_e.to_idx() % 10)
                {
                    self.egrid.add_tile_e(cell, "EBR", 9);
                }
            }
        }
    }

    fn fill_int_io(&mut self, cell: CellCoord) {
        match self.chip.kind {
            ChipKind::Ecp | ChipKind::Xp => {
                if cell.row == self.chip.row_s() || cell.row == self.chip.row_n() {
                    self.egrid.add_tile_single(cell, "INT_IO_SN");
                } else {
                    self.egrid.add_tile_single(cell, "INT_IO_WE");
                }
            }
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => {
                if cell.row == self.chip.row_n() {
                    self.egrid.add_tile_single(cell, "INT_IO_N");
                } else if cell.row == self.chip.row_s() {
                    self.egrid.add_tile_single(cell, "INT_IO_S");
                } else {
                    self.egrid.add_tile_single(cell, "INT_IO_WE");
                }
            }
            _ => unreachable!(),
        }
    }

    fn fill_io_scm(&mut self) {
        let mut prev = None;
        for cell in self.egrid.column(self.die, self.chip.col_w()).rev() {
            if self.is_in_bel_hole(cell) {
                continue;
            }
            let rd = &self.chip.rows[cell.row];
            if rd.kind == RowKind::Ebr {
                prev = None;
            }
            if !matches!(rd.kind, RowKind::Plc | RowKind::Fplc) {
                continue;
            }
            self.egrid.add_tile_single(cell, "INT_IO");
            self.egrid.add_tile_single(cell, "IO_INT_W");
            if let Some(prev) = prev {
                self.egrid
                    .fill_conn_pair(prev, cell, "PASS_IO_E", "PASS_IO_W");
            }
            prev = Some(cell);
            match rd.io_w {
                IoGroupKind::Quad => {
                    self.egrid.add_tile_n(cell, "IO_W4", 2);
                }
                IoGroupKind::Dozen => {
                    self.egrid.add_tile_n(cell, "IO_W12", 4);
                }
                IoGroupKind::None => (),
                _ => unreachable!(),
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_s()) {
            if self.is_in_bel_hole(cell) {
                continue;
            }
            self.egrid.add_tile_single(cell, "INT_IO");
            if cell.col >= self.chip.col_w() + 2 && cell.col < self.chip.col_e() - 1 {
                self.egrid.add_tile_single(cell, "IO_INT_S");
                if let Some(prev) = prev {
                    self.egrid
                        .fill_conn_pair(prev, cell, "PASS_IO_E", "PASS_IO_W");
                }
                prev = Some(cell);
            }
            match self.chip.columns[cell.col].io_s {
                IoGroupKind::Quad => {
                    self.egrid.add_tile_e(cell, "IO_S4", 2);
                }
                IoGroupKind::Dozen => {
                    self.egrid.add_tile_e(cell, "IO_S12", 4);
                }
                IoGroupKind::None => (),
                _ => unreachable!(),
            }
        }
        for cell in self.egrid.column(self.die, self.chip.col_e()) {
            if self.is_in_bel_hole(cell) {
                continue;
            }
            let rd = &self.chip.rows[cell.row];
            if rd.kind == RowKind::Ebr {
                prev = None;
            }
            if !matches!(rd.kind, RowKind::Plc | RowKind::Fplc) {
                continue;
            }
            self.egrid.add_tile_single(cell, "INT_IO");
            self.egrid.add_tile_single(cell, "IO_INT_E");
            if let Some(prev) = prev {
                self.egrid
                    .fill_conn_pair(prev, cell, "PASS_IO_E", "PASS_IO_W");
            }
            prev = Some(cell);
            match rd.io_e {
                IoGroupKind::Quad => {
                    self.egrid.add_tile_n(cell, "IO_E4", 2);
                }
                IoGroupKind::Dozen => {
                    self.egrid.add_tile_n(cell, "IO_E12", 4);
                }
                IoGroupKind::None => (),
                _ => unreachable!(),
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_n()) {
            if self.is_in_bel_hole(cell) {
                continue;
            }
            self.egrid.add_tile_single(cell, "INT_IO");
            match self.chip.columns[cell.col].io_n {
                IoGroupKind::Quad => {
                    self.egrid.add_tile_e(cell, "IO_N4", 2);
                }
                IoGroupKind::Octal => {
                    self.egrid.add_tile_e(cell, "IO_N8", 3);
                }
                IoGroupKind::Dozen => {
                    self.egrid.add_tile_e(cell, "IO_N12", 4);
                }
                IoGroupKind::None | IoGroupKind::Serdes => (),
                _ => unreachable!(),
            }
        }
    }

    fn fill_io_ecp(&mut self) {
        for cell in self.egrid.column(self.die, self.chip.col_w()) {
            if self.is_in_bel_hole(cell) {
                continue;
            }

            let rd = &self.chip.rows[cell.row];
            if !matches!(rd.kind, RowKind::Plc | RowKind::Fplc) {
                continue;
            }
            self.fill_int_io(cell);
            self.egrid.add_tile_single(cell, "IO_W");

            if rd.io_w == IoGroupKind::DoubleDqs {
                self.egrid.add_tile_single(cell, "DQS_W");
                let row_base: RowId = match self.chip.kind {
                    ChipKind::Ecp | ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => {
                        cell.row - 3
                    }
                    ChipKind::Xp => cell.row - 2,
                    _ => unreachable!(),
                };
                for row_io in row_base.range(row_base + 8) {
                    self.dqs.insert(cell.with_row(row_io), cell);
                }
            }
        }

        for cell in self.egrid.column(self.die, self.chip.col_e()) {
            if self.is_in_bel_hole(cell) {
                continue;
            }

            let rd = &self.chip.rows[cell.row];
            if !matches!(rd.kind, RowKind::Plc | RowKind::Fplc) {
                continue;
            }
            self.fill_int_io(cell);
            self.egrid.add_tile_single(cell, "IO_E");

            if rd.io_e == IoGroupKind::DoubleDqs {
                self.egrid.add_tile_single(cell, "DQS_E");
                let row_base: RowId = match self.chip.kind {
                    ChipKind::Ecp | ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => {
                        cell.row - 3
                    }
                    ChipKind::Xp => cell.row - 2,
                    _ => unreachable!(),
                };
                for row_io in row_base.range(row_base + 8) {
                    self.dqs.insert(cell.with_row(row_io), cell);
                }
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_s()) {
            if self.is_in_bel_hole(cell) {
                continue;
            }

            let cd = &self.chip.columns[cell.col];
            self.fill_int_io(cell);
            if cell.col == self.chip.col_w() || cell.col == self.chip.col_e() {
                continue;
            }
            self.egrid.add_tile_single(cell, "IO_S");

            if cd.io_s == IoGroupKind::DoubleDqs {
                self.egrid.add_tile_single(cell, "DQS_S");
                let (col_base, num): (ColId, usize) = match self.chip.kind {
                    ChipKind::Ecp => (cell.col - 4, 8),
                    ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => (cell.col - 4, 9),
                    ChipKind::Xp => (cell.col - 5, 8),
                    _ => unreachable!(),
                };
                for col_io in col_base.range(col_base + num) {
                    self.dqs.insert(cell.with_col(col_io), cell);
                }
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_n()) {
            if self.is_in_bel_hole(cell) {
                continue;
            }

            let cd = &self.chip.columns[cell.col];
            self.fill_int_io(cell);
            if cell.col == self.chip.col_w() || cell.col == self.chip.col_e() {
                continue;
            }
            self.egrid.add_tile_single(cell, "IO_N");

            if cd.io_n == IoGroupKind::DoubleDqs {
                self.egrid.add_tile_single(cell, "DQS_N");
                let (col_base, num): (ColId, usize) = match self.chip.kind {
                    ChipKind::Ecp => (cell.col - 4, 8),
                    ChipKind::Xp2 => (cell.col - 4, 9),
                    ChipKind::Xp => (cell.col - 5, 8),
                    _ => unreachable!(),
                };
                for col_io in col_base.range(col_base + num) {
                    self.dqs.insert(cell.with_col(col_io), cell);
                }
            }
        }
        if matches!(self.chip.kind, ChipKind::Ecp | ChipKind::Xp) {
            let cell = self.chip.bel_dqsdll_ecp(DirV::S).cell;
            self.egrid.add_tile_single(cell, "DQSDLL_S");
            let cell = self.chip.bel_dqsdll_ecp(DirV::N).cell;
            self.egrid.add_tile_single(cell, "DQSDLL_N");
        }
        if self.chip.kind == ChipKind::Xp2 {
            let cell = self.chip.bel_dqsdll_ecp2(DirH::W).cell;
            self.egrid.add_tile_single(cell, "CLK_W");
            let cell = self.chip.bel_dqsdll_ecp2(DirH::E).cell;
            self.egrid
                .add_tile(cell, "CLK_E", &[cell, cell.with_col(self.chip.col_clk + 2)]);
        }
    }

    fn fill_io_ecp3(&mut self) {
        let is_a = self.chip.kind == ChipKind::Ecp3A;
        for cell in self.egrid.row(self.die, self.chip.row_s()) {
            if !self.is_in_bel_hole(cell) {
                self.egrid.add_tile_single(cell, "INT_IO_S");
            }
            if self.chip.columns[cell.col].io_s == IoGroupKind::Quad {
                self.egrid.add_tile_e(cell, "XSIO_S", 3);
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_n()) {
            self.egrid.add_tile_single(cell, "INT_IO_N");
            match (
                self.chip.columns[cell.col].io_n,
                self.chip.columns[cell.col].bank_n,
            ) {
                (IoGroupKind::Quad, Some(8)) => {
                    self.egrid.add_tile_e(cell, "XSIO_N", 3);
                }
                (IoGroupKind::Quad, _) => {
                    if is_a {
                        self.egrid.add_tile_e(cell, "SIO_A_N", 3);
                    } else {
                        self.egrid.add_tile_e(cell, "SIO_N", 3);
                    }
                }
                (IoGroupKind::QuadDqs, _) => {
                    self.egrid.add_tile_e(cell, "SIO_DQS_N", 3);
                    self.egrid.add_tile_e(cell, "DQS_N", 6);
                    for col_io in cell.delta(-3, 0).col.range(cell.col + 6) {
                        self.dqs.insert(cell.with_col(col_io), cell);
                    }
                }
                _ => (),
            }
        }
        for cell in self.egrid.column(self.die, self.chip.col_w()) {
            if matches!(self.chip.rows[cell.row].kind, RowKind::Plc | RowKind::Fplc) {
                self.egrid.add_tile_single(cell, "INT_IO_WE");
            }
            match self.chip.rows[cell.row].io_w {
                IoGroupKind::Quad => {
                    self.egrid.add_tile_n(cell, "IO_W", 3);
                }
                IoGroupKind::QuadDqs => {
                    self.egrid.add_tile_n(cell, "IO_DQS_W", 3);
                    if is_a {
                        self.egrid.add_tile_n(cell, "DQS_A_W", 3);
                    } else {
                        self.egrid.add_tile_n(cell, "DQS_W", 3);
                    }
                    for row_io in cell.delta(0, -3).row.range(cell.row + 6) {
                        self.dqs.insert(cell.with_row(row_io), cell);
                    }
                    self.dqs.insert(cell.delta(1, -3), cell);
                }
                IoGroupKind::QuadDqsDummy => {
                    self.egrid.add_tile_n(cell, "IO_DQS_DUMMY_W", 3);
                    if is_a {
                        self.egrid.add_tile_n(cell, "DQS_A_W", 3);
                    } else {
                        self.egrid.add_tile_n(cell, "DQS_W", 3);
                    }
                    for row_io in cell.delta(0, -3).row.range(cell.row + 3) {
                        self.dqs.insert(cell.with_row(row_io), cell);
                    }
                }
                _ => (),
            }
        }
        for cell in self.egrid.column(self.die, self.chip.col_e()) {
            if matches!(self.chip.rows[cell.row].kind, RowKind::Plc | RowKind::Fplc) {
                self.egrid.add_tile_single(cell, "INT_IO_WE");
            }
            match (
                self.chip.rows[cell.row].io_e,
                self.chip.rows[cell.row].bank_e,
            ) {
                (IoGroupKind::Quad, Some(8)) => {
                    self.egrid.add_tile_n(cell, "XSIO_E", 3);
                }
                (IoGroupKind::Quad, _) => {
                    self.egrid.add_tile_n(cell, "IO_E", 3);
                }
                (IoGroupKind::QuadDqs, _) => {
                    self.egrid.add_tile_n(cell, "IO_DQS_E", 3);
                    if is_a {
                        self.egrid.add_tile_n(cell, "DQS_A_E", 3);
                    } else {
                        self.egrid.add_tile_n(cell, "DQS_E", 3);
                    }
                    for row_io in cell.delta(0, -3).row.range(cell.row + 6) {
                        self.dqs.insert(cell.with_row(row_io), cell);
                    }
                    self.dqs.insert(cell.delta(-1, -3), cell);
                }
                _ => (),
            }
        }
        for edge in [Dir::W, Dir::E, Dir::N] {
            let cell = self.chip.bel_eclksync(edge, 0).cell;
            self.egrid
                .add_tile_single(cell, &format!("ECLK_ROOT_{edge}"));
        }
    }

    fn fill_io_machxo2(&mut self) {
        for cell in self.egrid.row(self.die, self.chip.row_s()) {
            self.egrid.add_tile_single(cell, "INT_IO_S");
        }
        for cell in self.egrid.row(self.die, self.chip.row_n()) {
            self.egrid.add_tile_single(cell, "INT_IO_N");
        }
        for cell in self.egrid.column(self.die, self.chip.col_w()) {
            if matches!(self.chip.rows[cell.row].kind, RowKind::Plc | RowKind::Fplc) {
                self.egrid.add_tile_single(cell, "INT_IO_WE");
            }
        }
        for cell in self.egrid.column(self.die, self.chip.col_e()) {
            if matches!(self.chip.rows[cell.row].kind, RowKind::Plc | RowKind::Fplc) {
                self.egrid.add_tile_single(cell, "INT_IO_WE");
            }
        }

        let is_smol = self.chip.rows[self.chip.row_clk].kind != RowKind::Ebr;
        let has_slewrate = matches!(
            self.chip.kind,
            ChipKind::MachXo2(
                MachXo2Kind::MachXo3Lfp | MachXo2Kind::MachXo3D | MachXo2Kind::MachNx
            )
        );
        for (&key, &cell) in &self.chip.special_loc {
            let SpecialLocKey::Bc(bank) = key else {
                continue;
            };
            if has_slewrate && bank == 1 {
                self.egrid
                    .add_tile(cell, "BCSR_E", &[cell, cell.delta(0, -1)]);
            } else if has_slewrate {
                self.egrid.add_tile_single(
                    cell,
                    match bank {
                        0 => "BCSR_N",
                        2 => "BCSR_S",
                        3..=5 => "BCSR_W",
                        _ => unreachable!(),
                    },
                );
            } else if bank == 0 && !is_smol {
                self.egrid.add_tile_single(cell, "BC_N");
            } else {
                self.egrid.add_tile_single(cell, "BC");
            }
        }

        for cell in self.egrid.column(self.die, self.chip.col_w()) {
            let kind = match self.chip.rows[cell.row].io_w {
                IoGroupKind::Double => "IO_W2",
                IoGroupKind::Quad => "IO_W4",
                IoGroupKind::QuadI3c => "IO_W4_I3C",
                _ => continue,
            };
            self.egrid.add_tile_single(cell, kind);
        }
        for cell in self.egrid.column(self.die, self.chip.col_e()) {
            let kind = match self.chip.rows[cell.row].io_e {
                IoGroupKind::Double => "IO_E2",
                IoGroupKind::Quad => "IO_E4",
                _ => continue,
            };
            self.egrid.add_tile_single(cell, kind);
        }
        for cell in self.egrid.row(self.die, self.chip.row_s()) {
            let kind = match self.chip.columns[cell.col].io_s {
                IoGroupKind::Double if is_smol => "SIO_S2",
                IoGroupKind::Quad | IoGroupKind::QuadReverse if is_smol => "SIO_S4",
                IoGroupKind::Double if !is_smol => "IO_S2",
                IoGroupKind::Quad | IoGroupKind::QuadReverse if !is_smol => "IO_S4",
                _ => continue,
            };
            self.egrid.add_tile_single(cell, kind);
        }
        for cell in self.egrid.row(self.die, self.chip.row_n()) {
            let kind = match self.chip.columns[cell.col].io_n {
                IoGroupKind::Double if is_smol => "SIO_N2",
                IoGroupKind::Quad | IoGroupKind::QuadReverse if is_smol => "SIO_N4",
                IoGroupKind::Double if !is_smol => "IO_N2",
                IoGroupKind::Quad | IoGroupKind::QuadReverse if !is_smol => "IO_N4",
                _ => continue,
            };
            self.egrid.add_tile_single(cell, kind);
        }
    }

    fn fill_io_ecp4(&mut self) {
        for (&key, &cell) in &self.chip.special_loc {
            let SpecialLocKey::Bc(bank) = key else {
                continue;
            };
            let kind = match bank {
                0..4 => "BC_N",
                4..6 => "BC_E",
                6..8 => "BC_W",
                _ => unreachable!(),
            };
            self.egrid.add_tile_single(cell, kind);
        }
        {
            let cell = self
                .chip
                .special_loc
                .get(&SpecialLocKey::Bc(3))
                .copied()
                .unwrap_or(self.chip.special_loc[&SpecialLocKey::Bc(2)]);
            self.egrid.add_tile_single(cell, "PVTTEST");
        }
        for col in [
            self.chip.col_w() + 2,
            self.chip.col_w() + 3,
            self.chip.col_e() - 3,
            self.chip.col_e() - 2,
        ] {
            for (row, kind) in [
                (self.chip.row_s(), "DDRDLL_S"),
                (self.chip.row_n(), "DDRDLL_N"),
            ] {
                let cell = CellCoord::new(self.die, col, row);
                self.egrid.add_tile_single(cell, kind);
            }
        }
        for (col, row, kind) in [
            (self.chip.col_w() + 1, self.chip.row_n(), "DTR_N"),
            (self.chip.col_e() - 4, self.chip.row_s(), "DTR_S"),
        ] {
            let cell = CellCoord::new(self.die, col, row);
            self.egrid.add_tile_single(cell, kind);
        }
        for cell in self.egrid.row(self.die, self.chip.row_s()) {
            self.egrid.add_tile_single(cell, "INT_IO_S");
        }
        for cell in self.egrid.row(self.die, self.chip.row_n()) {
            self.egrid.add_tile_single(cell, "INT_IO_N");
            if self.chip.columns[cell.col].io_n != IoGroupKind::None {
                self.egrid.add_tile_e(cell, "IO_N", 4);
            }
            if self.chip.columns[cell.col].io_n == IoGroupKind::QuadDqs {
                self.egrid.add_tile_we(cell, "DQS_N", 1, 5);
                self.dqs.insert(cell, cell);
                self.dqs.insert(cell.delta(-4, 0), cell);
                self.dqs.insert(cell.delta(4, 0), cell);
            }
        }
        for cell in self.egrid.column(self.die, self.chip.col_w()) {
            let rd = &self.chip.rows[cell.row];
            if matches!(rd.kind, RowKind::Plc | RowKind::Fplc) {
                self.egrid.add_tile_single(cell, "INT_IO_WE");
            }
            if matches!(
                rd.io_w,
                IoGroupKind::Quad | IoGroupKind::QuadDqs | IoGroupKind::QuadEbrDqs
            ) {
                if rd.kind == RowKind::Dsp {
                    assert!(cell.row >= self.chip.row_clk);
                    self.egrid.add_tile(
                        cell,
                        "IO_W_DSP_N",
                        &[cell.delta(0, 1), cell, cell.delta(2, 0), cell.delta(1, 0)],
                    );
                    if rd.io_w == IoGroupKind::QuadDqs {
                        self.egrid.add_tile(
                            cell,
                            "DQS_W_DSP_N",
                            &[
                                cell.delta(0, 2),
                                cell.delta(0, 1),
                                cell,
                                cell.delta(2, 0),
                                cell.delta(1, 0),
                            ],
                        );
                        self.dqs.insert(cell, cell);
                        self.dqs.insert(cell.delta(0, -4), cell);
                        self.dqs.insert(cell.delta(0, 2), cell);
                    }
                } else if self.chip.rows[cell.row + 1].kind == RowKind::Dsp {
                    assert!(cell.row < self.chip.row_clk);
                    self.egrid.add_tile(
                        cell,
                        "IO_W_DSP_S",
                        &[cell.delta(1, 1), cell.delta(2, 1), cell.delta(0, 1), cell],
                    );
                } else {
                    self.egrid.add_tile(
                        cell,
                        "IO_W",
                        &[
                            cell.delta(0, 3),
                            cell.delta(0, 2),
                            cell.delta(0, 1),
                            cell.delta(0, 0),
                        ],
                    );
                    if rd.io_w == IoGroupKind::QuadDqs {
                        if self.chip.rows[cell.row + 3].kind == RowKind::Ebr {
                            self.egrid.add_tile(
                                cell,
                                "DQS_W_BELOW_EBR_S",
                                &[
                                    cell.delta(4, 3),
                                    cell.delta(0, 3),
                                    cell.delta(0, 2),
                                    cell.delta(0, 1),
                                    cell.delta(0, 0),
                                ],
                            );
                            self.dqs.insert(cell.delta(1, 3), cell);
                        } else if self.chip.rows[cell.row + 4].kind == RowKind::Dsp {
                            self.egrid.add_tile(
                                cell,
                                "DQS_W_BELOW_DSP_N",
                                &[
                                    cell.delta(1, 4),
                                    cell.delta(0, 3),
                                    cell.delta(0, 2),
                                    cell.delta(0, 1),
                                    cell.delta(0, 0),
                                ],
                            );
                            self.dqs.insert(cell.delta(0, 4), cell);
                        } else if self.chip.rows[cell.row + 4].kind == RowKind::Ebr {
                            self.egrid.add_tile(
                                cell,
                                "DQS_W_BELOW_EBR_N",
                                &[
                                    cell.delta(1, 4),
                                    cell.delta(0, 3),
                                    cell.delta(0, 2),
                                    cell.delta(0, 1),
                                    cell.delta(0, 0),
                                ],
                            );
                            self.dqs.insert(cell.delta(1, 4), cell);
                        } else {
                            self.egrid.add_tile(
                                cell,
                                "DQS_W",
                                &[
                                    cell.delta(0, 4),
                                    cell.delta(0, 3),
                                    cell.delta(0, 2),
                                    cell.delta(0, 1),
                                    cell.delta(0, 0),
                                ],
                            );
                            self.dqs.insert(cell.delta(0, 4), cell);
                        }
                        self.dqs.insert(cell, cell);
                        if self.chip.rows[cell.row].kind == RowKind::Ebr {
                            self.dqs.insert(cell.delta(1, 0), cell);
                        } else if self.chip.rows[cell.row - 1].kind == RowKind::Ebr {
                            self.dqs.insert(cell.delta(1, -1), cell);
                        } else if self.chip.rows[cell.row - 1].kind == RowKind::Dsp
                            || self.chip.rows[cell.row - 2].kind == RowKind::Dsp
                        {
                            self.dqs.insert(cell.delta(0, -2), cell);
                        } else {
                            self.dqs.insert(cell.delta(0, -4), cell);
                        }
                    }
                }
            }
            if rd.kind == RowKind::Ebr {
                if cell.row < self.chip.row_clk {
                    self.egrid.add_tile(
                        cell.delta(1, 0),
                        "IO_W_EBR_S",
                        &[
                            cell.delta(1, 0),
                            cell.delta(2, 0),
                            cell.delta(3, 0),
                            cell.delta(4, 0),
                        ],
                    );
                    if matches!(rd.io_w, IoGroupKind::EbrDqs | IoGroupKind::QuadEbrDqs) {
                        self.egrid.add_tile(
                            cell.delta(1, 0),
                            "DQS_W_EBR_S",
                            &[
                                cell.delta(0, 1),
                                cell.delta(1, 0),
                                cell.delta(2, 0),
                                cell.delta(3, 0),
                                cell.delta(4, 0),
                            ],
                        );
                        self.dqs.insert(cell.delta(1, 0), cell.delta(1, 0));
                        self.dqs.insert(cell.delta(0, -3), cell.delta(1, 0));
                        self.dqs.insert(cell.delta(0, 1), cell.delta(1, 0));
                    }
                } else {
                    self.egrid.add_tile(
                        cell.delta(1, 0),
                        "IO_W_EBR_N",
                        &[
                            cell.delta(4, 0),
                            cell.delta(3, 0),
                            cell.delta(2, 0),
                            cell.delta(1, 0),
                        ],
                    );
                    if matches!(rd.io_w, IoGroupKind::EbrDqs | IoGroupKind::QuadEbrDqs) {
                        self.egrid.add_tile(
                            cell.delta(1, 0),
                            "DQS_W_EBR_N",
                            &[
                                cell,
                                cell.delta(4, 0),
                                cell.delta(3, 0),
                                cell.delta(2, 0),
                                cell.delta(1, 0),
                            ],
                        );
                        self.dqs.insert(cell.delta(1, 0), cell.delta(1, 0));
                        self.dqs.insert(cell.delta(0, -4), cell.delta(1, 0));
                        self.dqs.insert(cell, cell.delta(1, 0));
                    }
                }
            }
        }
        for cell in self.egrid.column(self.die, self.chip.col_e()) {
            let rd = &self.chip.rows[cell.row];
            if matches!(rd.kind, RowKind::Plc | RowKind::Fplc) {
                self.egrid.add_tile_single(cell, "INT_IO_WE");
            }
            if matches!(
                rd.io_e,
                IoGroupKind::Quad | IoGroupKind::QuadDqs | IoGroupKind::QuadEbrDqs
            ) {
                if rd.kind == RowKind::Dsp {
                    assert!(cell.row >= self.chip.row_clk);
                    self.egrid.add_tile(
                        cell,
                        "IO_E_DSP_N",
                        &[cell.delta(0, 1), cell, cell.delta(-2, 0), cell.delta(-1, 0)],
                    );
                    if rd.io_e == IoGroupKind::QuadDqs {
                        self.egrid.add_tile(
                            cell,
                            "DQS_E_DSP_N",
                            &[
                                cell.delta(0, 2),
                                cell.delta(0, 1),
                                cell,
                                cell.delta(-2, 0),
                                cell.delta(-1, 0),
                            ],
                        );
                        self.dqs.insert(cell, cell);
                        self.dqs.insert(cell.delta(0, -4), cell);
                        self.dqs.insert(cell.delta(0, 2), cell);
                    }
                } else if self.chip.rows[cell.row + 1].kind == RowKind::Dsp {
                    assert!(cell.row < self.chip.row_clk);
                    self.egrid.add_tile(
                        cell,
                        "IO_E_DSP_S",
                        &[cell.delta(-1, 1), cell.delta(-2, 1), cell.delta(0, 1), cell],
                    );
                } else {
                    self.egrid.add_tile(
                        cell,
                        "IO_E",
                        &[
                            cell.delta(0, 3),
                            cell.delta(0, 2),
                            cell.delta(0, 1),
                            cell.delta(0, 0),
                        ],
                    );
                    if rd.io_e == IoGroupKind::QuadDqs {
                        if self.chip.rows[cell.row + 3].kind == RowKind::Ebr {
                            self.egrid.add_tile(
                                cell,
                                "DQS_E_BELOW_EBR_S",
                                &[
                                    cell.delta(-4, 3),
                                    cell.delta(0, 3),
                                    cell.delta(0, 2),
                                    cell.delta(0, 1),
                                    cell.delta(0, 0),
                                ],
                            );
                            self.dqs.insert(cell.delta(-1, 3), cell);
                        } else if self.chip.rows[cell.row + 4].kind == RowKind::Dsp {
                            self.egrid.add_tile(
                                cell,
                                "DQS_E_BELOW_DSP_N",
                                &[
                                    cell.delta(-1, 4),
                                    cell.delta(0, 3),
                                    cell.delta(0, 2),
                                    cell.delta(0, 1),
                                    cell.delta(0, 0),
                                ],
                            );
                            self.dqs.insert(cell.delta(0, 4), cell);
                        } else if self.chip.rows[cell.row + 4].kind == RowKind::Ebr {
                            self.egrid.add_tile(
                                cell,
                                "DQS_E_BELOW_EBR_N",
                                &[
                                    cell.delta(-1, 4),
                                    cell.delta(0, 3),
                                    cell.delta(0, 2),
                                    cell.delta(0, 1),
                                    cell.delta(0, 0),
                                ],
                            );
                            self.dqs.insert(cell.delta(-1, 4), cell);
                        } else {
                            self.egrid.add_tile(
                                cell,
                                "DQS_E",
                                &[
                                    cell.delta(0, 4),
                                    cell.delta(0, 3),
                                    cell.delta(0, 2),
                                    cell.delta(0, 1),
                                    cell.delta(0, 0),
                                ],
                            );
                            self.dqs.insert(cell.delta(0, 4), cell);
                        }
                        self.dqs.insert(cell, cell);
                        if self.chip.rows[cell.row].kind == RowKind::Ebr {
                            self.dqs.insert(cell.delta(-1, 0), cell);
                        } else if self.chip.rows[cell.row - 1].kind == RowKind::Ebr {
                            self.dqs.insert(cell.delta(-1, -1), cell);
                        } else if self.chip.rows[cell.row - 1].kind == RowKind::Dsp
                            || self.chip.rows[cell.row - 2].kind == RowKind::Dsp
                        {
                            self.dqs.insert(cell.delta(0, -2), cell);
                        } else {
                            self.dqs.insert(cell.delta(0, -4), cell);
                        }
                    }
                }
            }
            if rd.kind == RowKind::Ebr {
                if cell.row < self.chip.row_clk {
                    self.egrid.add_tile(
                        cell.delta(-1, 0),
                        "IO_E_EBR_S",
                        &[
                            cell.delta(-1, 0),
                            cell.delta(-2, 0),
                            cell.delta(-3, 0),
                            cell.delta(-4, 0),
                        ],
                    );
                    if matches!(rd.io_e, IoGroupKind::EbrDqs | IoGroupKind::QuadEbrDqs) {
                        self.egrid.add_tile(
                            cell.delta(-1, 0),
                            "DQS_E_EBR_S",
                            &[
                                cell.delta(0, 1),
                                cell.delta(-1, 0),
                                cell.delta(-2, 0),
                                cell.delta(-3, 0),
                                cell.delta(-4, 0),
                            ],
                        );
                        self.dqs.insert(cell.delta(-1, 0), cell.delta(-1, 0));
                        self.dqs.insert(cell.delta(0, -3), cell.delta(-1, 0));
                        self.dqs.insert(cell.delta(0, 1), cell.delta(-1, 0));
                    }
                } else {
                    self.egrid.add_tile(
                        cell.delta(-1, 0),
                        "IO_E_EBR_N",
                        &[
                            cell.delta(-4, 0),
                            cell.delta(-3, 0),
                            cell.delta(-2, 0),
                            cell.delta(-1, 0),
                        ],
                    );
                    if matches!(rd.io_e, IoGroupKind::EbrDqs | IoGroupKind::QuadEbrDqs) {
                        self.egrid.add_tile(
                            cell.delta(-1, 0),
                            "DQS_E_EBR_N",
                            &[
                                cell,
                                cell.delta(-4, 0),
                                cell.delta(-3, 0),
                                cell.delta(-2, 0),
                                cell.delta(-1, 0),
                            ],
                        );
                        self.dqs.insert(cell.delta(-1, 0), cell.delta(-1, 0));
                        self.dqs.insert(cell.delta(0, -4), cell.delta(-1, 0));
                        self.dqs.insert(cell, cell.delta(-1, 0));
                    }
                }
            }
        }
    }

    fn fill_io_ecp5(&mut self) {
        for (&key, &cell) in &self.chip.special_loc {
            let SpecialLocKey::Bc(bank) = key else {
                continue;
            };
            self.egrid.add_tile_single(cell, &format!("BC{bank}"));
        }
        for hv in DirHV::DIRS {
            let cell = self.chip.special_loc[&SpecialLocKey::DdrDll(hv)];
            self.egrid.add_tile_single(cell, "DDRDLL");
        }

        for cell in self.egrid.row(self.die, self.chip.row_s()) {
            self.egrid.add_tile_single(cell, "INT_IO_S");
            match self.chip.columns[cell.col].io_s {
                IoGroupKind::None => (),
                IoGroupKind::Single => {
                    self.egrid.add_tile_single(cell, "IO_S1");
                }
                IoGroupKind::Double => {
                    self.egrid.add_tile_e(cell, "IO_S2", 2);
                }
                IoGroupKind::Serdes => {
                    self.egrid.add_tile_e(cell, "SERDES", 12);
                }
                _ => unreachable!(),
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_n()) {
            self.egrid.add_tile_single(cell, "INT_IO_N");
            match self.chip.columns[cell.col].io_n {
                IoGroupKind::None => (),
                IoGroupKind::Double => {
                    self.egrid.add_tile_e(cell, "IO_N2", 2);
                }
                _ => unreachable!(),
            }
        }
        for cell in self.egrid.column(self.die, self.chip.col_w()) {
            let rd = &self.chip.rows[cell.row];
            if matches!(rd.kind, RowKind::Plc | RowKind::Fplc) {
                self.egrid.add_tile_single(cell, "INT_IO_WE");
            }
            match self.chip.rows[cell.row].io_w {
                IoGroupKind::None => (),
                IoGroupKind::Quad => {
                    self.egrid.add_tile_n(cell, "IO_W4", 3);
                }
                IoGroupKind::QuadDqs => {
                    self.egrid.add_tile_n(cell, "IO_W4", 3);
                    self.egrid.add_tile_n(cell, "DQS_W", 6);
                    for dy in [-3, 0, 3, 6] {
                        self.dqs.insert(cell.delta(0, dy), cell);
                    }
                }
                _ => unreachable!(),
            }
        }
        for cell in self.egrid.column(self.die, self.chip.col_e()) {
            let rd = &self.chip.rows[cell.row];
            if matches!(rd.kind, RowKind::Plc | RowKind::Fplc) {
                self.egrid.add_tile_single(cell, "INT_IO_WE");
            }
            match self.chip.rows[cell.row].io_e {
                IoGroupKind::None => (),
                IoGroupKind::Quad => {
                    self.egrid.add_tile_n(cell, "IO_E4", 3);
                }
                IoGroupKind::QuadDqs => {
                    self.egrid.add_tile_n(cell, "IO_E4", 3);
                    self.egrid.add_tile_n(cell, "DQS_E", 6);
                    for dy in [-3, 0, 3, 6] {
                        self.dqs.insert(cell.delta(0, dy), cell);
                    }
                }
                _ => unreachable!(),
            }
        }
    }

    fn fill_io_crosslink(&mut self) {
        for (&key, &cell) in &self.chip.special_loc {
            if matches!(key, SpecialLocKey::Bc(_)) {
                self.egrid.add_tile_single(cell, "BC");
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_s()) {
            self.egrid.add_tile_single(cell, "INT_IO_S");
            match self.chip.columns[cell.col].io_s {
                IoGroupKind::Quad => {
                    self.egrid.add_tile_e(cell, "IO_S4", 4);
                }
                IoGroupKind::Single => {
                    if cell.col.to_idx() % 2 == 0 {
                        self.egrid.add_tile_single(cell, "IO_S1A");
                    } else {
                        self.egrid.add_tile_single(cell, "IO_S1B");
                    }
                }
                _ => (),
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_n()) {
            self.egrid.add_tile_single(cell, "INT_IO_N");
            if self.chip.columns[cell.col].io_n == IoGroupKind::Mipi {
                self.egrid.add_tile_e(
                    cell,
                    if cell.col < self.chip.col_clk {
                        "MIPI_W"
                    } else {
                        "MIPI_E"
                    },
                    24,
                );
            }
        }
    }

    fn fill_clk_scm(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            let col_pclk = if cell.col < self.chip.col_clk {
                self.chip.col_w()
            } else {
                self.chip.col_e()
            };
            let row_pclk = if cell.row < self.chip.row_clk {
                self.chip.row_s()
            } else {
                self.chip.row_n()
            };
            self.egrid[cell].region_root[regions::PCLK0] =
                CellCoord::new(self.die, col_pclk, row_pclk);
        }

        for (bank, kind) in [
            (7, "CLK_W"),
            (2, "CLK_E"),
            (1, "CLK_N"),
            (5, "CLK_SW"),
            (4, "CLK_SE"),
        ] {
            let cell = self.chip.special_loc[&SpecialLocKey::Bc(bank)];
            let cells = if matches!(bank, 2 | 7) {
                [cell.delta(0, -1), cell]
            } else {
                [cell.delta(-1, 0), cell]
            };
            self.egrid.add_tile(cell, kind, &cells);
        }
        let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_s());
        self.egrid
            .add_tile(cell, "CLK_S", &[cell.delta(-1, 0), cell]);

        let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_clk);
        self.egrid.add_tile(
            cell,
            "CLK_ROOT",
            &[
                cell.delta(-1, -5),
                cell.delta(0, -5),
                cell.delta(-1, 4),
                cell.delta(0, 4),
            ],
        );
    }

    fn fill_clk_ecp(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            let col_pclk = if cell.col < self.chip.col_clk {
                self.chip.col_w()
            } else {
                self.chip.col_e()
            };
            let row_pclk = if cell.row < self.chip.row_clk {
                self.chip.row_s()
            } else {
                self.chip.row_n()
            };
            self.egrid[cell].region_root[regions::PCLK0] =
                CellCoord::new(self.die, col_pclk, row_pclk);
        }

        let ebr_rows = Vec::from_iter(
            self.chip
                .rows
                .ids()
                .filter(|&row| self.chip.rows[row].kind == RowKind::Ebr),
        );
        let mut cells = vec![
            // actual clock root cells
            CellCoord::new(self.die, self.chip.col_w(), self.chip.row_s()),
            CellCoord::new(self.die, self.chip.col_e(), self.chip.row_s()),
            CellCoord::new(self.die, self.chip.col_w(), self.chip.row_n()),
            CellCoord::new(self.die, self.chip.col_e(), self.chip.row_n()),
        ];
        // DCS select inputs
        match self.chip.kind {
            ChipKind::Ecp if ebr_rows.len() == 1 => {
                cells.extend([
                    CellCoord::new(self.die, self.chip.col_w(), ebr_rows[0]),
                    CellCoord::new(self.die, self.chip.col_e(), ebr_rows[0]),
                ]);
            }
            ChipKind::Ecp if ebr_rows.len() == 2 => {
                cells.extend([
                    CellCoord::new(self.die, self.chip.col_w() + 5, ebr_rows[0]),
                    CellCoord::new(self.die, self.chip.col_w() + 6, ebr_rows[0]),
                    CellCoord::new(self.die, self.chip.col_e() - 6, ebr_rows[0]),
                    CellCoord::new(self.die, self.chip.col_e() - 5, ebr_rows[0]),
                    CellCoord::new(self.die, self.chip.col_w() + 5, ebr_rows[1]),
                    CellCoord::new(self.die, self.chip.col_w() + 6, ebr_rows[1]),
                    CellCoord::new(self.die, self.chip.col_e() - 6, ebr_rows[1]),
                    CellCoord::new(self.die, self.chip.col_e() - 5, ebr_rows[1]),
                ]);
            }
            ChipKind::Xp if ebr_rows.len() == 1 => {
                cells.extend([
                    CellCoord::new(self.die, self.chip.col_w(), ebr_rows[0]),
                    CellCoord::new(self.die, self.chip.col_w() + 1, ebr_rows[0]),
                    CellCoord::new(self.die, self.chip.col_e() - 1, ebr_rows[0]),
                    CellCoord::new(self.die, self.chip.col_e(), ebr_rows[0]),
                ]);
            }
            ChipKind::Xp if ebr_rows.len() == 2 => {
                cells.extend([
                    CellCoord::new(self.die, self.chip.col_w(), ebr_rows[0]),
                    CellCoord::new(self.die, self.chip.col_w() + 1, ebr_rows[0]),
                    CellCoord::new(self.die, self.chip.col_e() - 1, ebr_rows[0]),
                    CellCoord::new(self.die, self.chip.col_e(), ebr_rows[0]),
                    CellCoord::new(self.die, self.chip.col_w(), ebr_rows[1]),
                    CellCoord::new(self.die, self.chip.col_w() + 1, ebr_rows[1]),
                    CellCoord::new(self.die, self.chip.col_e() - 1, ebr_rows[1]),
                    CellCoord::new(self.die, self.chip.col_e(), ebr_rows[1]),
                ]);
            }
            _ => unreachable!(),
        }
        // fabric clock inputs
        for (&key, &cell) in &self.chip.special_loc {
            if !matches!(key, SpecialLocKey::PclkIn(..) | SpecialLocKey::SclkIn(..)) {
                continue;
            }
            cells.push(cell);
        }
        let kind = match (self.chip.kind, ebr_rows.len()) {
            (ChipKind::Ecp, 1) => "CLK_ROOT_2PLL",
            (ChipKind::Ecp, 2) => "CLK_ROOT_4PLL",
            (ChipKind::Xp, 1) => {
                if self.chip.special_loc[&SpecialLocKey::SclkIn(Dir::E, 2)].col == self.chip.col_clk
                {
                    "CLK_ROOT_2PLL_A"
                } else {
                    "CLK_ROOT_2PLL_B"
                }
            }
            (ChipKind::Xp, 2) => "CLK_ROOT_4PLL",
            _ => unreachable!(),
        };
        self.egrid.add_tile(
            CellCoord::new(self.die, self.chip.col_clk, self.chip.row_clk),
            kind,
            &cells,
        );
    }

    fn fill_clk_ecp2(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            let col_pclk = if cell.col < self.chip.col_clk {
                self.chip.col_w()
            } else {
                self.chip.col_e()
            };
            let row_pclk = if cell.row < self.chip.row_clk {
                self.chip.row_s()
            } else {
                self.chip.row_n()
            };
            self.egrid[cell].region_root[regions::PCLK0] = cell.with_cr(col_pclk, row_pclk);
        }

        let mut rows_sclk = EntityVec::new();
        let mut row_src = self.chip.row_s();
        for (row, rd) in &self.chip.rows {
            if rd.sclk_break {
                row_src = row;
            }
            rows_sclk.push(row_src);
        }
        for i in 0..4 {
            for cell in self.egrid.die_cells(self.die) {
                if !self.egrid.has_bel(cell.bel(bels::INT)) {
                    continue;
                }
                let (dx, dx_alt) = match (cell.col.to_idx() + i) % 4 {
                    0 => (0, 0),
                    1 => (-1, 3),
                    2 => (-2, 2),
                    3 => (1, -3),
                    _ => unreachable!(),
                };
                let cell_src = if let Some(cell_src) = self.egrid.cell_delta(cell, dx, 0)
                    && self.egrid.has_bel(cell_src.bel(bels::INT))
                {
                    cell_src
                } else {
                    cell.delta(dx_alt, 0)
                };
                let cell_src = cell_src.with_row(rows_sclk[cell.row]);
                self.egrid[cell].region_root[regions::SCLK[i]] = cell_src;
                if cell == cell_src {
                    let mut cell = cell;
                    if !self.egrid.has_bel(cell.bel(bels::INT)) {
                        cell.row += 7;
                    }
                    self.egrid.add_tile_single(cell, &format!("SCLK{i}_SOURCE"));
                }
            }
        }

        for (row, rd) in &self.chip.rows {
            if !matches!(rd.kind, RowKind::Io | RowKind::Ebr | RowKind::Dsp) {
                continue;
            }
            let mut prev = None;
            for mut cell in self.egrid.row(self.die, row) {
                if !self.egrid.has_bel(cell.bel(bels::INT)) {
                    if cell.row == self.chip.row_s() {
                        cell.row += 7;
                    } else if cell.row == self.chip.row_n() {
                        cell.row -= 7;
                    } else {
                        unreachable!();
                    }
                }
                if self.chip.columns[cell.col].sdclk_break {
                    prev = None;
                }
                if let Some(prev) = prev {
                    self.egrid.fill_conn_pair(prev, cell, "PASS_SE", "PASS_SW");
                }
                prev = Some(cell);
            }

            for cell in self.egrid.row(self.die, row) {
                if self.chip.columns[cell.col].sdclk_break {
                    let mut cells = [None; 8];
                    for cell in cell.delta(-4, 0).cells_e_const::<4>() {
                        cells[self.chip.col_sclk_idx(cell.col)] = Some(cell);
                    }
                    for cell in cell.cells_e_const::<4>() {
                        cells[4 + self.chip.col_sclk_idx(cell.col)] = Some(cell);
                    }
                    let mut cells = cells.map(Option::unwrap);
                    for cell in &mut cells {
                        if !self.egrid.has_bel(cell.bel(bels::INT)) {
                            if cell.row == self.chip.row_s() {
                                cell.row += 7;
                            } else if cell.row == self.chip.row_n() {
                                cell.row -= 7;
                            } else {
                                unreachable!();
                            }
                        }
                    }
                    let mut cell = cell;
                    if !self.egrid.has_bel(cell.bel(bels::INT)) {
                        if cell.row == self.chip.row_s() {
                            cell.row += 7;
                        } else if cell.row == self.chip.row_n() {
                            cell.row -= 7;
                        } else {
                            unreachable!();
                        }
                    }
                    self.egrid.add_tile(
                        cell,
                        if cell.col == self.chip.col_clk {
                            "HSDCLK_ROOT"
                        } else {
                            "HSDCLK_SPLITTER"
                        },
                        &cells,
                    );
                }
            }
        }

        for col in self.chip.columns.ids() {
            let mut root = CellCoord::new(self.die, col, self.chip.row_s());
            for cell in self.egrid.column(self.die, col) {
                if self.chip.rows[cell.row].sclk_break {
                    root = cell;
                }
                self.egrid[cell].region_root[regions::VSDCLK] = root;
            }
        }

        for edge in Dir::DIRS {
            let bcrd = self.chip.bel_eclk_root(edge);
            match edge {
                Dir::H(_) => {
                    self.egrid
                        .add_tile_single(bcrd.cell, &format!("ECLK_ROOT_{edge}"));
                }
                Dir::V(_) => {
                    self.egrid.add_tile(
                        bcrd.cell,
                        &format!("ECLK_ROOT_{edge}"),
                        &[bcrd.cell, bcrd.cell.delta(-1, 0)],
                    );
                }
            }
        }

        for cell in self.egrid.row(self.die, self.chip.row_s()) {
            if self.chip.columns[cell.col].eclk_tap_s {
                self.egrid.add_tile_single(cell, "ECLK_TAP");
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_n()) {
            if self.chip.columns[cell.col].eclk_tap_n {
                self.egrid.add_tile_single(cell, "ECLK_TAP");
            }
        }
        for cell in self.egrid.column(self.die, self.chip.col_w()) {
            if matches!(self.chip.rows[cell.row].kind, RowKind::Ebr | RowKind::Dsp)
                && cell.row != self.chip.row_clk
                && !(self.chip.kind == ChipKind::Ecp2
                    && self.chip.rows[cell.row].kind == RowKind::Ebr
                    && cell.row > self.chip.row_clk)
            {
                self.egrid.add_tile_single(cell, "ECLK_TAP");
            }
        }
        for cell in self.egrid.column(self.die, self.chip.col_e()) {
            if matches!(self.chip.rows[cell.row].kind, RowKind::Ebr | RowKind::Dsp)
                && cell.row != self.chip.row_clk
                && !(self.chip.kind == ChipKind::Ecp2
                    && self.chip.rows[cell.row].kind == RowKind::Ebr
                    && cell.row > self.chip.row_clk)
            {
                self.egrid.add_tile_single(cell, "ECLK_TAP");
            }
        }

        let mut cells = vec![
            // actual clock root cells
            CellCoord::new(self.die, self.chip.col_w(), self.chip.row_s()),
            CellCoord::new(self.die, self.chip.col_e(), self.chip.row_s()),
            CellCoord::new(self.die, self.chip.col_w(), self.chip.row_n()),
            CellCoord::new(self.die, self.chip.col_e(), self.chip.row_n()),
        ];
        for cell in &mut cells {
            if !self.egrid.has_bel(cell.bel(bels::INT)) {
                if cell.row == self.chip.row_s() {
                    cell.row += 7;
                } else if cell.row == self.chip.row_n() {
                    cell.row -= 7;
                } else {
                    unreachable!();
                }
            }
        }
        // fabric clock inputs
        let mut num_pll = 0;
        for (&key, &cell) in &self.chip.special_loc {
            match key {
                SpecialLocKey::PclkIn(..) | SpecialLocKey::SclkIn(..) => {
                    cells.push(cell);
                }
                SpecialLocKey::Pll(..) => num_pll += 1,
                _ => (),
            }
        }
        let kind = match num_pll {
            2 => "CLK_ROOT_2PLL",
            4 => "CLK_ROOT_4PLL",
            6 => "CLK_ROOT_6PLL",
            8 => "CLK_ROOT_8PLL",
            _ => unreachable!(),
        };
        self.egrid.add_tile(
            CellCoord::new(self.die, self.chip.col_clk, self.chip.row_clk),
            kind,
            &cells,
        );
    }

    fn fill_sclk_ecp3(&mut self) {
        let mut row_sclk = self.chip.row_s();
        let mut rows_sclk = EntityVec::new();
        for (row, rd) in &self.chip.rows {
            if rd.sclk_break {
                row_sclk = row;
            }
            rows_sclk.push(row_sclk);
        }

        for i in 0..4 {
            for cell in self.egrid.die_cells(self.die) {
                let (dx, dx_alt) = match (i + 4 - self.chip.col_sclk_idx(cell.col)) % 4 {
                    0 => (0, 0),
                    1 => (1, 0),
                    2 => (-2, -1),
                    3 => (-1, 0),
                    _ => unreachable!(),
                };
                let cell_src = if let Some(cell_src) = self.egrid.cell_delta(cell, dx, 0) {
                    cell_src
                } else if let Some(cell_src) = self.egrid.cell_delta(cell, dx_alt, 0) {
                    cell_src
                } else {
                    cell
                };
                let cell_src_sclk = cell_src.with_row(rows_sclk[cell.row]);
                self.egrid[cell].region_root[regions::SCLK[i]] = cell_src_sclk;
            }
        }
        for (row, rd) in &self.chip.rows {
            if !rd.sclk_break && row != self.chip.row_s() {
                continue;
            }
            for cell in self.egrid.row(self.die, row) {
                let i = self.chip.col_sclk_idx(cell.col);
                let kind = if cell.col == self.chip.col_w() {
                    format!("SCLK{i}_SOURCE_W")
                } else if cell.col == self.chip.col_e() {
                    format!("SCLK{i}_SOURCE_E")
                } else {
                    format!("SCLK{i}_SOURCE")
                };
                self.egrid.add_tile_single(cell, &kind);
            }
        }

        for col in self.chip.columns.ids() {
            let mut root = CellCoord::new(self.die, col, self.chip.row_s());
            for cell in self.egrid.column(self.die, col) {
                if self.chip.rows[cell.row].sclk_break {
                    root = cell;
                }
                self.egrid[cell].region_root[regions::VSDCLK] = root;
            }
        }

        for (row, rd) in &self.chip.rows {
            if rd.kind != RowKind::Io && !rd.sclk_break {
                continue;
            }
            let mut prev = None;
            for mut cell in self.egrid.row(self.die, row) {
                if !self.egrid.has_bel(cell.bel(bels::INT)) {
                    if cell.row == self.chip.row_s() {
                        cell.row += 9;
                    } else {
                        unreachable!();
                    }
                }
                if self.chip.columns[cell.col].sdclk_break {
                    prev = None;
                }
                if let Some(prev) = prev {
                    self.egrid.fill_conn_pair(prev, cell, "PASS_SE", "PASS_SW");
                }
                prev = Some(cell);
            }

            for cell in self.egrid.row(self.die, row) {
                if self.chip.columns[cell.col].sdclk_break {
                    let mut cells = [None; 8];
                    for cell in cell.delta(-4, 0).cells_e_const::<4>() {
                        cells[self.chip.col_sclk_idx(cell.col)] = Some(cell);
                    }
                    for cell in cell.cells_e_const::<4>() {
                        cells[4 + self.chip.col_sclk_idx(cell.col)] = Some(cell);
                    }
                    let mut cells = cells.map(Option::unwrap);
                    for cell in &mut cells {
                        if !self.egrid.has_bel(cell.bel(bels::INT)) {
                            if cell.row == self.chip.row_s() {
                                cell.row += 9;
                            } else {
                                unreachable!();
                            }
                        }
                    }
                    let mut cell = cell;
                    if !self.egrid.has_bel(cell.bel(bels::INT)) {
                        if cell.row == self.chip.row_s() {
                            cell.row += 9;
                        } else {
                            unreachable!();
                        }
                    }
                    self.egrid.add_tile(
                        cell,
                        if cell.col == self.chip.col_clk
                            && !matches!(self.chip.kind, ChipKind::MachXo2(_))
                        {
                            "HSDCLK_SPLITTER"
                        } else {
                            "HSDCLK_ROOT"
                        },
                        &cells,
                    );
                }
            }
        }
    }

    fn fill_pclk_ecp3(&mut self) {
        let mut row_pclk = self.chip.row_s();
        let mut rows_pclk = EntityVec::new();
        for (row, rd) in &self.chip.rows {
            if rd.pclk_break {
                row_pclk = row;
            }
            rows_pclk.push(row_pclk);
        }

        for i in 0..4 {
            for cell in self.egrid.die_cells(self.die) {
                let (dx, dx_alt) = match (i + 4 - self.chip.col_sclk_idx(cell.col)) % 4 {
                    0 => (0, 0),
                    1 => (1, 0),
                    2 => (-2, -1),
                    3 => (-1, 0),
                    _ => unreachable!(),
                };
                let cell_src = if let Some(cell_src) = self.egrid.cell_delta(cell, dx, 0)
                    && ((cell_src.col < self.chip.col_clk) == (cell.col < self.chip.col_clk)
                        || matches!(self.chip.kind, ChipKind::MachXo2(_)))
                {
                    cell_src
                } else if let Some(cell_src) = self.egrid.cell_delta(cell, dx_alt, 0)
                    && ((cell_src.col < self.chip.col_clk) == (cell.col < self.chip.col_clk)
                        || matches!(self.chip.kind, ChipKind::MachXo2(_)))
                {
                    cell_src
                } else {
                    cell
                };
                let cell_src_pclk = cell_src.with_row(rows_pclk[cell.row]);
                self.egrid[cell].region_root[regions::PCLK[i]] = cell_src_pclk;
            }
        }

        let mut prev_driven = self.chip.rows[self.chip.row_s()].pclk_drive;
        for (row, rd) in &self.chip.rows {
            if !rd.pclk_drive {
                if rd.pclk_break {
                    prev_driven = false;
                }
                continue;
            }
            for cell in self.egrid.row(self.die, row) {
                let idx = self.chip.col_sclk_idx(cell.col);
                if prev_driven {
                    let kind = if cell.col == self.chip.col_w()
                        || (cell.col == self.chip.col_clk
                            && !matches!(self.chip.kind, ChipKind::MachXo2(_)))
                    {
                        if cell.row == self.chip.row_s() {
                            format!("PCLK{idx}_SOURCE_IO_N_W")
                        } else {
                            format!("PCLK{idx}_SOURCE_N_W")
                        }
                    } else if cell.col == self.chip.col_e()
                        || (cell.col == self.chip.col_clk - 1
                            && !matches!(self.chip.kind, ChipKind::MachXo2(_)))
                    {
                        if cell.row == self.chip.row_s() {
                            format!("PCLK{idx}_SOURCE_IO_N_E")
                        } else {
                            format!("PCLK{idx}_SOURCE_N_E")
                        }
                    } else {
                        if cell.row == self.chip.row_s() {
                            format!("PCLK{idx}_SOURCE_IO_N")
                        } else {
                            format!("PCLK{idx}_SOURCE_N")
                        }
                    };
                    self.egrid.add_tile_single(cell, &kind);
                } else {
                    let kind = if cell.col == self.chip.col_w()
                        || (cell.col == self.chip.col_clk
                            && !matches!(self.chip.kind, ChipKind::MachXo2(_)))
                    {
                        format!("PCLK{idx}_SOURCE_W")
                    } else if cell.col == self.chip.col_e()
                        || (cell.col == self.chip.col_clk - 1
                            && !matches!(self.chip.kind, ChipKind::MachXo2(_)))
                    {
                        format!("PCLK{idx}_SOURCE_E")
                    } else {
                        format!("PCLK{idx}_SOURCE")
                    };
                    self.egrid.add_tile(cell, &kind, &[cell, cell.delta(0, -1)]);
                }
            }
            prev_driven = true;
        }
    }

    fn fill_clk_ecp3(&mut self) {
        for cell in self.egrid.row(self.die, self.chip.row_n()) {
            if self.chip.columns[cell.col].eclk_tap_n {
                self.egrid.add_tile_single(cell, "ECLK_TAP");
            }
        }

        let mut cells = vec![];
        // fabric clock inputs
        for (&key, &cell) in &self.chip.special_loc {
            match key {
                SpecialLocKey::PclkIn(..)
                | SpecialLocKey::PclkInMid(0 | 2 | 4 | 6)
                | SpecialLocKey::SclkIn(..) => {
                    cells.push(cell);
                }
                _ => (),
            }
        }
        self.egrid.add_tile(
            CellCoord::new(self.die, self.chip.col_clk, self.chip.row_clk),
            "CLK_ROOT",
            &cells,
        );
    }

    fn fill_clk_machxo2(&mut self) {
        let ebr_rows = Vec::from_iter(
            self.chip
                .rows
                .ids()
                .filter(|&row| self.chip.rows[row].kind == RowKind::Ebr),
        );
        let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_clk);
        match ebr_rows.len() {
            0 => {
                self.egrid.add_tile(
                    cell,
                    "CLK_ROOT_0EBR",
                    &[
                        cell.with_col(self.chip.col_w()).delta(0, -2),
                        cell.with_col(self.chip.col_w()).delta(0, -1),
                        cell.with_col(self.chip.col_e()).delta(0, -2),
                        cell.with_col(self.chip.col_e()).delta(0, -1),
                        cell.with_row(self.chip.row_s()).delta(-1, 0),
                        cell.with_row(self.chip.row_s()),
                        cell.with_row(self.chip.row_n()).delta(-1, 0),
                        cell.with_row(self.chip.row_n()),
                    ],
                );
            }
            1 => {
                self.egrid.add_tile(
                    cell,
                    "CLK_ROOT_1EBR",
                    &[
                        cell.delta(-4, 0),
                        cell.delta(-2, 0),
                        cell.delta(-1, 0),
                        cell,
                        cell.with_col(self.chip.col_w()),
                        cell.with_col(self.chip.col_e()),
                        cell.with_row(self.chip.row_s()).delta(-1, 0),
                        cell.with_row(self.chip.row_n()).delta(-1, 0),
                    ],
                );
            }
            2 => {
                self.egrid.add_tile(
                    cell,
                    "CLK_ROOT_2EBR",
                    &[
                        cell.delta(-4, 0),
                        cell.delta(-2, 0),
                        cell.delta(-1, 0),
                        cell,
                        cell.with_col(self.chip.col_w()),
                        cell.with_col(self.chip.col_e()),
                        cell.with_row(self.chip.row_s()).delta(-1, 0),
                        cell.with_row(self.chip.row_n()).delta(-1, 0),
                        cell.with_row(ebr_rows[0]).delta(-2, 0),
                        cell.with_row(ebr_rows[0]).delta(-1, 0),
                        cell.with_row(ebr_rows[0]),
                        cell.with_row(ebr_rows[0]).with_col(self.chip.col_w()),
                        cell.with_row(ebr_rows[0]).with_col(self.chip.col_e()),
                    ],
                );
            }
            3 => {
                self.egrid.add_tile(
                    cell,
                    "CLK_ROOT_3EBR",
                    &[
                        cell.delta(-4, 0),
                        cell.delta(-1, 0),
                        cell.with_col(self.chip.col_w()),
                        cell.with_col(self.chip.col_e()),
                        cell.with_row(self.chip.row_s()).delta(-1, 0),
                        cell.with_row(self.chip.row_n()).delta(-1, 0),
                        cell.with_row(ebr_rows[0]).delta(-2, 0),
                        cell.with_row(ebr_rows[0]).delta(-1, 0),
                        cell.with_row(ebr_rows[0]),
                        cell.with_row(ebr_rows[2]).delta(-2, 0),
                        cell.with_row(ebr_rows[2]).delta(-1, 0),
                        cell.with_row(ebr_rows[2]),
                        cell.with_row(ebr_rows[0]).with_col(self.chip.col_w()),
                        cell.with_row(ebr_rows[0]).with_col(self.chip.col_e()),
                    ],
                );
            }
            _ => unreachable!(),
        }
        if !ebr_rows.is_empty() {
            self.egrid
                .add_tile_single(cell.with_row(self.chip.row_s()).delta(-1, 0), "CLK_S");
            self.egrid
                .add_tile_single(cell.with_row(self.chip.row_n()).delta(-1, 0), "CLK_N");
            self.egrid
                .add_tile_single(cell.with_col(self.chip.col_w()), "CLK_W");
            if self.chip.kind == ChipKind::MachXo2(MachXo2Kind::MachXo2) {
                self.egrid
                    .add_tile_sn(cell.with_col(self.chip.col_e()), "CLK_E_DQS", 2, 5);
            } else {
                self.egrid
                    .add_tile_single(cell.with_col(self.chip.col_e()), "CLK_E");
            }
        }
        for (edge, kind) in [(DirV::S, "DQSDLL_S"), (DirV::N, "DQSDLL_N")] {
            let Some(&cell) = self
                .chip
                .special_loc
                .get(&SpecialLocKey::DqsDll(Dir::V(edge)))
            else {
                continue;
            };
            self.egrid.add_tile_single(cell, kind);
        }
    }

    fn fill_clk_ecp4(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            let col_pclk = if cell.col < self.chip.col_clk {
                self.chip.col_w()
            } else {
                self.chip.col_e()
            };
            let row_pclk = if cell.row < self.chip.row_clk {
                self.chip.row_s()
            } else {
                self.chip.row_n()
            };
            self.egrid[cell].region_root[regions::PCLK0] =
                CellCoord::new(self.die, col_pclk, row_pclk);
        }

        let num_quads = if self
            .chip
            .special_loc
            .contains_key(&SpecialLocKey::SerdesSingle)
        {
            1
        } else if self
            .chip
            .special_loc
            .contains_key(&SpecialLocKey::SerdesDouble)
        {
            2
        } else if self
            .chip
            .special_loc
            .contains_key(&SpecialLocKey::SerdesTriple)
        {
            3
        } else {
            unreachable!()
        };

        let (kind_w, kind_e, kind_s, kind_n) = match num_quads {
            1 => ("CLK_W_S", "CLK_E_S", "CLK_S_S", "CLK_N_S"),
            2 => ("CLK_W_M", "CLK_E_M", "CLK_S_M", "CLK_N_M"),
            3 => ("CLK_W_L", "CLK_E_L", "CLK_S_L", "CLK_N_L"),
            _ => unreachable!(),
        };

        let has_bank0 = self.chip.special_loc.contains_key(&SpecialLocKey::Bc(0));
        let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_n());
        let mut tcells = if !has_bank0 {
            vec![
                cell.delta(-4, 0),
                cell.delta(-3, 0),
                cell.delta(-2, 0),
                cell.delta(-1, 0),
                cell,
                cell.delta(1, 0),
                cell.delta(2, 0),
                cell.delta(3, 0),
            ]
        } else {
            vec![
                cell.delta(-8, 0),
                cell.delta(-7, 0),
                cell.delta(-6, 0),
                cell.delta(-5, 0),
                cell.delta(-4, 0),
                cell.delta(-3, 0),
                cell.delta(-2, 0),
                cell.delta(-1, 0),
                cell,
                cell.delta(1, 0),
                cell.delta(2, 0),
                cell.delta(3, 0),
                cell.delta(4, 0),
                cell.delta(5, 0),
                cell.delta(6, 0),
                cell.delta(7, 0),
                self.chip.special_loc[&SpecialLocKey::Bc(1)].delta(1, 0),
                self.chip.special_loc[&SpecialLocKey::Bc(2)].delta(1, 0),
            ]
        };
        for i in 0..4 {
            if let Some(&cell) = self.chip.special_loc.get(&SpecialLocKey::PclkIn(Dir::N, i)) {
                tcells.push(cell);
            }
        }
        self.egrid.add_tile(cell, kind_n, &tcells);

        let cell = CellCoord::new(self.die, self.chip.col_w(), self.chip.row_clk);
        let mut tcells = vec![
            cell.delta(0, -4),
            cell.delta(0, -3),
            cell.delta(0, -2),
            cell.delta(0, -1),
            cell,
            cell.delta(0, 1),
            cell.delta(0, 2),
            cell.delta(0, 3),
        ];
        for i in 0..4 {
            if let Some(&cell) = self.chip.special_loc.get(&SpecialLocKey::PclkIn(Dir::W, i)) {
                tcells.push(cell);
            }
        }
        self.egrid.add_tile(cell, kind_w, &tcells);
        let cell = CellCoord::new(self.die, self.chip.col_e(), self.chip.row_clk);
        let mut tcells = vec![
            cell.delta(0, -4),
            cell.delta(0, -3),
            cell.delta(0, -2),
            cell.delta(0, -1),
            cell,
            cell.delta(0, 1),
            cell.delta(0, 2),
            cell.delta(0, 3),
        ];
        for i in 0..4 {
            if let Some(&cell) = self.chip.special_loc.get(&SpecialLocKey::PclkIn(Dir::E, i)) {
                tcells.push(cell);
            }
        }
        self.egrid.add_tile(cell, kind_e, &tcells);

        let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_s());
        let mut tcells = vec![cell.delta(-1, 0), cell];
        for i in 0..4 {
            if let Some(&cell) = self.chip.special_loc.get(&SpecialLocKey::PclkIn(Dir::S, i)) {
                tcells.push(cell);
            }
        }
        self.egrid.add_tile(cell, kind_s, &tcells);

        let row_s = self
            .chip
            .rows
            .ids()
            .rev()
            .filter(|&row| row < self.chip.row_clk)
            .find(|&row| self.chip.rows[row].kind == RowKind::Ebr)
            .unwrap();
        let row_n = self
            .chip
            .rows
            .ids()
            .filter(|&row| row >= self.chip.row_clk)
            .find(|&row| self.chip.rows[row].kind == RowKind::Ebr)
            .unwrap();
        let cell = self.chip.bel_clk_root().cell;
        self.egrid.add_tile(
            cell,
            "CLK_ROOT",
            &[
                cell.with_cr(self.chip.col_clk - 1, row_s),
                cell.with_cr(self.chip.col_clk, row_s),
                cell.with_cr(self.chip.col_clk - 1, row_n),
                cell.with_cr(self.chip.col_clk, row_n),
            ],
        );
    }

    fn fill_clk_ecp5(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            let col_pclk = if cell.col < self.chip.col_clk {
                self.chip.col_w()
            } else {
                self.chip.col_e()
            };
            let row_pclk = if cell.row < self.chip.row_clk {
                self.chip.row_s()
            } else {
                self.chip.row_n()
            };
            self.egrid[cell].region_root[regions::PCLK0] =
                CellCoord::new(self.die, col_pclk, row_pclk);
        }

        let cell = CellCoord::new(self.die, self.chip.col_w(), self.chip.row_clk);
        let mut tcells = vec![
            cell.delta(0, -2),
            cell.delta(0, -1),
            cell,
            cell.delta(0, 1),
            cell.delta(1, 0),
            cell.delta(2, 0),
        ];
        for i in 0..8 {
            if let Some(&cell) = self.chip.special_loc.get(&SpecialLocKey::PclkIn(Dir::W, i)) {
                tcells.push(cell);
            }
        }
        self.egrid.add_tile(
            cell,
            match tcells.len() {
                12 => "CLK_W_S",
                14 => "CLK_W_L",
                _ => unreachable!(),
            },
            &tcells,
        );

        let cell = CellCoord::new(self.die, self.chip.col_e(), self.chip.row_clk);
        let mut tcells = vec![
            cell.delta(0, -2),
            cell.delta(0, -1),
            cell,
            cell.delta(0, 1),
            cell.delta(-1, 0),
            cell.delta(-2, 0),
        ];
        for i in 0..8 {
            if let Some(&cell) = self.chip.special_loc.get(&SpecialLocKey::PclkIn(Dir::E, i)) {
                tcells.push(cell);
            }
        }
        self.egrid.add_tile(
            cell,
            match tcells.len() {
                12 => "CLK_E_S",
                14 => "CLK_E_L",
                _ => unreachable!(),
            },
            &tcells,
        );

        let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_n());
        let mut tcells = cell.delta(-2, 0).cells_e(4);
        for i in 0..8 {
            if let Some(&cell) = self.chip.special_loc.get(&SpecialLocKey::PclkIn(Dir::N, i)) {
                tcells.push(cell);
            }
        }
        self.egrid.add_tile(
            cell,
            match tcells.len() {
                10 => "CLK_N_S",
                12 => "CLK_N_L",
                _ => unreachable!(),
            },
            &tcells,
        );

        let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_s());
        let mut tcells = cell.delta(-1, 0).cells_e(2);
        for i in 0..8 {
            if let Some(&cell) = self.chip.special_loc.get(&SpecialLocKey::PclkIn(Dir::S, i)) {
                tcells.push(cell);
            }
        }
        self.egrid.add_tile(
            cell,
            match tcells.len() {
                8 => "CLK_S_S",
                10 => "CLK_S_L",
                _ => unreachable!(),
            },
            &tcells,
        );

        let cell = self.chip.bel_clk_root().cell;
        let cell_s = self.chip.special_loc[&SpecialLocKey::ClkQuarter(DirV::S)];
        let cell_n = self.chip.special_loc[&SpecialLocKey::ClkQuarter(DirV::N)];
        let row_s = self
            .chip
            .rows
            .ids()
            .rev()
            .filter(|&row| row < self.chip.row_clk)
            .find(|&row| matches!(self.chip.rows[row].kind, RowKind::Ebr | RowKind::Dsp))
            .unwrap();
        let row_n = self
            .chip
            .rows
            .ids()
            .filter(|&row| row > self.chip.row_clk)
            .find(|&row| matches!(self.chip.rows[row].kind, RowKind::Ebr | RowKind::Dsp))
            .unwrap();

        if row_s == cell_s.row {
            self.egrid.add_tile(
                cell,
                "CLK_ROOT_S",
                &[cell_s.delta(-1, 0), cell_s, cell_n.delta(-1, 0), cell_n],
            );
        } else {
            self.egrid.add_tile(
                cell,
                "CLK_ROOT_L",
                &[
                    cell_s.delta(-1, 0),
                    cell_s,
                    cell.with_row(row_s).delta(-1, 0),
                    cell.with_row(row_s),
                    cell.with_row(row_n).delta(-1, 0),
                    cell.with_row(row_n),
                    cell_n.delta(-1, 0),
                    cell_n,
                ],
            );
        }
    }

    fn fill_clk_crosslink(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            self.egrid[cell].region_root[regions::PCLK0] =
                cell.with_cr(self.chip.col_clk, self.chip.row_clk);
        }

        let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_s());
        let mut tcells = cell.delta(-2, 0).cells_e(4);
        for i in 0..2 {
            tcells.push(self.chip.special_loc[&SpecialLocKey::PclkIn(Dir::S, i)]);
        }
        self.egrid.add_tile(cell, "CLK_S", &tcells);

        let cell = CellCoord::new(self.die, self.chip.col_clk, self.chip.row_n());
        self.egrid.add_tile_we(cell, "CLK_N", 1, 2);

        let cell = self.chip.bel_clk_root().cell;
        self.egrid.add_tile_single(cell, "CLK_ROOT");
    }

    fn fill_config_xp(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Config];
        self.egrid.add_tile_single(cell, "CONFIG");
    }

    fn fill_config_machxo2(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Config];
        match self.chip.kind {
            ChipKind::MachXo2(
                MachXo2Kind::MachXo2 | MachXo2Kind::MachXo3L | MachXo2Kind::MachXo3Lfp,
            ) => {
                self.egrid.add_tile_e(cell, "CONFIG", 4);
            }
            ChipKind::MachXo2(MachXo2Kind::MachXo3D | MachXo2Kind::MachNx) => {
                self.egrid.add_tile_e(cell, "CONFIG_XO3D", 10);
            }
            _ => unreachable!(),
        }
    }

    fn fill_config_ecp4(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Config];
        let mut tcells = cell.cells_e(6);
        let row_dsp = self
            .chip
            .rows
            .ids()
            .rev()
            .filter(|&row| row < self.chip.row_clk)
            .find(|&row| self.chip.rows[row].kind == RowKind::Dsp)
            .unwrap();
        tcells.extend([
            cell.with_cr(self.chip.col_w(), self.chip.row_clk - 1),
            cell.with_cr(self.chip.col_w(), self.chip.row_clk),
            cell.with_cr(self.chip.col_clk - 1, self.chip.row_s()),
            cell.with_cr(self.chip.col_clk, self.chip.row_s()),
            cell.with_cr(self.chip.col_clk - 1, row_dsp),
        ]);
        self.egrid.add_tile(cell, "CONFIG", &tcells);
    }

    fn fill_config_ecp5(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Config];
        let mut tcells = cell.cells_e(3);
        tcells.extend([cell.with_cr(self.chip.col_clk, self.chip.row_clk)]);
        self.egrid.add_tile(cell, "CONFIG", &tcells);
        self.egrid.add_tile_single(cell.delta(18, 0), "DTR");
    }

    fn fill_config_crosslink(&mut self) {
        let cell = CellCoord::new(self.die, self.chip.col_w(), self.chip.row_n());
        self.egrid.add_tile_e(cell, "I2C_W", 2);
        let cell = CellCoord::new(self.die, self.chip.col_e() - 1, self.chip.row_n());
        self.egrid.add_tile_e(cell, "I2C_E", 2);

        let cell = self.chip.special_loc[&SpecialLocKey::Config];
        let mut tcells = cell.cells_e(2);
        tcells.extend([
            cell.delta(-2, 0),
            cell.with_cr(self.chip.col_clk, self.chip.row_clk),
        ]);
        self.egrid.add_tile(cell, "CONFIG", &tcells);

        let cell = self.chip.special_loc[&SpecialLocKey::Osc];
        self.egrid.add_tile_single(cell, "OSC");

        let cell = self.chip.special_loc[&SpecialLocKey::Pmu];
        self.egrid.add_tile_single(cell, "PMU");

        let cell = self.chip.special_loc[&SpecialLocKey::Pll(PllLoc::new(DirHV::SE, 0))];
        self.egrid.add_tile_e(cell, "PLL", 2);
    }

    fn fill_io_machxo(&mut self) {
        let has_ebr = self.chip.special_loc.contains_key(&SpecialLocKey::Ebr(0));
        for cell in self.egrid.column(self.die, self.chip.col_w()) {
            let rd = &self.chip.rows[cell.row];
            if !matches!(rd.kind, RowKind::Plc | RowKind::Fplc) {
                continue;
            }
            let int_kind = if has_ebr {
                "INT_SIO_XW"
            } else if cell.row >= self.chip.row_clk - 3 && cell.row < self.chip.row_clk + 3 {
                "INT_SIO_W_CLK"
            } else {
                "INT_SIO_W"
            };
            self.egrid.add_tile_single(cell, int_kind);
            let kind = match (has_ebr, rd.io_w) {
                (true, IoGroupKind::Double) => "SIO_XW2",
                (true, IoGroupKind::Quad | IoGroupKind::QuadReverse) => "SIO_XW4",
                (false, IoGroupKind::Double) => "SIO_W2",
                (false, IoGroupKind::Quad) => "SIO_W4",
                _ => unreachable!(),
            };
            self.egrid.add_tile_single(cell, kind);
        }
        for cell in self.egrid.column(self.die, self.chip.col_e()) {
            let rd = &self.chip.rows[cell.row];
            if !matches!(rd.kind, RowKind::Plc | RowKind::Fplc) {
                continue;
            }
            let int_kind = if cell == self.chip.special_loc[&SpecialLocKey::Config] {
                "INT_SIO_E_CFG"
            } else {
                "INT_SIO_E"
            };
            self.egrid.add_tile_single(cell, int_kind);
            let kind = match rd.io_e {
                IoGroupKind::Double => "SIO_E2",
                IoGroupKind::Quad => "SIO_E4",
                _ => unreachable!(),
            };
            self.egrid.add_tile_single(cell, kind);
        }
        for cell in self.egrid.row(self.die, self.chip.row_s()) {
            let cd = &self.chip.columns[cell.col];
            if cell.col == self.chip.col_w() || cell.col == self.chip.col_e() {
                continue;
            }
            let (int_kind, kind) = match cd.io_s {
                IoGroupKind::Quad => ("INT_SIO_S4", "SIO_S4"),
                IoGroupKind::Hex | IoGroupKind::HexReverse => ("INT_SIO_S6", "SIO_S6"),
                _ => unreachable!(),
            };
            self.egrid.add_tile_single(cell, int_kind);
            self.egrid.add_tile_single(cell, kind);
        }
        for cell in self.egrid.row(self.die, self.chip.row_n()) {
            let cd = &self.chip.columns[cell.col];
            if cell.col == self.chip.col_w() || cell.col == self.chip.col_e() {
                continue;
            }
            let (int_kind, kind) = match cd.io_n {
                IoGroupKind::Quad => ("INT_SIO_N4", "SIO_N4"),
                IoGroupKind::Hex | IoGroupKind::HexReverse => ("INT_SIO_N6", "SIO_N6"),
                _ => unreachable!(),
            };
            self.egrid.add_tile_single(cell, int_kind);
            self.egrid.add_tile_single(cell, kind);
        }
    }

    fn fill_special_machxo(&mut self) {
        let has_ebr = self.chip.special_loc.contains_key(&SpecialLocKey::Ebr(0));
        for (&key, &cell) in &self.chip.special_loc {
            match key {
                SpecialLocKey::Pll(which) => {
                    let kind = match which.quad.v {
                        DirV::S => "PLL_S",
                        DirV::N => "PLL_N",
                    };
                    self.egrid.add_tile_single(cell, kind);
                }
                SpecialLocKey::Ebr(_) => {
                    self.egrid.add_tile_n(cell, "EBR", 4);
                }
                SpecialLocKey::Config => {
                    self.egrid.add_tile_n(cell, "CONFIG", 5);
                }
                SpecialLocKey::Osc => {
                    let kind = if has_ebr { "OSC_X" } else { "OSC" };
                    self.egrid.add_tile_single(cell, kind);
                }
                _ => unreachable!(),
            }
        }
        if has_ebr {
            let bel = self.chip.bel_cibtest_sel();
            self.egrid.add_tile_single(bel.cell, "CIBTEST_SEL");
        }
    }

    fn fill_clk_machxo(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            self.egrid[cell].region_root[regions::PCLK0] =
                CellCoord::new(self.die, self.chip.col_clk, self.chip.row_clk);
        }
        let kind = if self
            .chip
            .special_loc
            .contains_key(&SpecialLocKey::Pll(PllLoc::new(DirHV::NW, 0)))
        {
            "CLK_ROOT_2PLL"
        } else if self
            .chip
            .special_loc
            .contains_key(&SpecialLocKey::Pll(PllLoc::new(DirHV::SW, 0)))
        {
            "CLK_ROOT_1PLL"
        } else {
            "CLK_ROOT_0PLL"
        };
        let tcells: [_; 6] = core::array::from_fn(|i| {
            CellCoord::new(self.die, self.chip.col_w(), self.chip.row_clk - 3 + i)
        });
        self.egrid.add_tile(
            CellCoord::new(self.die, self.chip.col_clk, self.chip.row_clk),
            kind,
            &tcells,
        );
    }

    fn fill_conns(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            if !self.egrid[cell].tiles.contains_id(tslots::INT) {
                continue;
            }
            if cell.col != self.chip.col_w()
                && self.egrid[cell.delta(-1, 0)].tiles.contains_id(tslots::INT)
            {
                self.egrid
                    .fill_conn_pair(cell.delta(-1, 0), cell, "PASS_E", "PASS_W");
            } else {
                self.egrid.fill_conn_term(cell, "TERM_W");
            }
            if cell.col == self.chip.col_e()
                || !self.egrid[cell.delta(1, 0)].tiles.contains_id(tslots::INT)
            {
                self.egrid.fill_conn_term(cell, "TERM_E");
            }
            if cell.row != self.chip.row_s()
                && self.egrid[cell.delta(0, -1)].tiles.contains_id(tslots::INT)
            {
                self.egrid
                    .fill_conn_pair(cell.delta(0, -1), cell, "PASS_N", "PASS_S");
            } else {
                self.egrid.fill_conn_term(cell, "TERM_S");
            }
            if cell.row == self.chip.row_n()
                || !self.egrid[cell.delta(0, 1)].tiles.contains_id(tslots::INT)
            {
                self.egrid.fill_conn_term(cell, "TERM_N");
            }
        }
    }
}

impl Chip {
    pub fn expand_grid<'a>(&'a self, db: &'a IntDb) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        let die = egrid.add_die(self.columns.len(), self.rows.len());
        let mut expander = Expander {
            chip: self,
            egrid: &mut egrid,
            die,
            bel_holes: vec![],
            dqs: BTreeMap::new(),
        };

        match self.kind {
            ChipKind::Scm => {
                expander.fill_config_scm();
                expander.fill_pll_scm();
                expander.fill_serdes_scm();
                expander.fill_plc();
                expander.fill_ebr_scm();
                expander.fill_io_scm();
                expander.fill_clk_scm();
            }
            ChipKind::Ecp => {
                expander.fill_config_ecp();
                expander.fill_pll_ecp();
                expander.fill_plc();
                expander.fill_ebr_ecp();
                expander.fill_dsp_ecp();
                expander.fill_io_ecp();
                expander.fill_clk_ecp();
            }
            ChipKind::Xp => {
                expander.fill_pll_xp();
                expander.fill_plc();
                expander.fill_ebr_ecp();
                expander.fill_config_xp();
                expander.fill_io_ecp();
                expander.fill_clk_ecp();
            }
            ChipKind::MachXo => {
                expander.fill_plc();
                expander.fill_io_machxo();
                expander.fill_special_machxo();
                expander.fill_clk_machxo();
            }
            ChipKind::Ecp2 | ChipKind::Ecp2M => {
                expander.fill_config_ecp2();
                expander.fill_pll_ecp2();
                expander.fill_serdes_ecp2();
                expander.fill_plc();
                expander.fill_ebr_ecp();
                expander.fill_dsp_ecp();
                expander.fill_io_ecp();
                expander.fill_clk_ecp2();
            }
            ChipKind::Xp2 => {
                expander.fill_config_xp2();
                expander.fill_pll_xp2();
                expander.fill_plc();
                expander.fill_ebr_ecp();
                expander.fill_dsp_ecp();
                expander.fill_io_ecp();
                expander.fill_clk_ecp2();
            }
            ChipKind::Ecp3 | ChipKind::Ecp3A => {
                expander.fill_config_ecp3();
                expander.fill_pll_ecp3();
                expander.fill_serdes_ecp3();
                expander.fill_plc();
                expander.fill_ebr_ecp();
                expander.fill_dsp_ecp();
                expander.fill_io_ecp3();
                expander.fill_sclk_ecp3();
                expander.fill_pclk_ecp3();
                expander.fill_clk_ecp3();
            }
            ChipKind::MachXo2(_) => {
                expander.fill_config_machxo2();
                expander.fill_pll_machxo2();
                expander.fill_plc();
                expander.fill_ebr_machxo2();
                expander.fill_io_machxo2();
                expander.fill_sclk_ecp3();
                expander.fill_pclk_ecp3();
                expander.fill_clk_machxo2();
            }
            ChipKind::Ecp4 => {
                expander.fill_config_ecp4();
                expander.fill_pll_ecp4();
                expander.fill_plc();
                expander.fill_ebr_ecp4();
                expander.fill_io_ecp4();
                expander.fill_serdes_ecp4();
                expander.fill_clk_ecp4();
            }
            ChipKind::Ecp5 => {
                expander.fill_config_ecp5();
                expander.fill_pll_ecp5();
                expander.fill_plc();
                expander.fill_ebr_ecp4();
                expander.fill_io_ecp5();
                expander.fill_clk_ecp5();
            }
            ChipKind::Crosslink => {
                expander.fill_config_crosslink();
                expander.fill_plc();
                expander.fill_ebr_crosslink();
                expander.fill_io_crosslink();
                expander.fill_clk_crosslink();
            }
        }
        expander.fill_conns();

        let bel_holes = expander.bel_holes;
        let dqs = expander.dqs;

        egrid.finish();
        ExpandedDevice {
            chip: self,
            egrid,
            bel_holes,
            dqs,
        }
    }
}
