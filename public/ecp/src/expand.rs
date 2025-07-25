use std::collections::BTreeMap;

use prjcombine_interconnect::{
    db::IntDb,
    dir::{Dir, DirH, DirHV, DirV},
    grid::{CellCoord, ColId, DieId, ExpandedGrid, Rect, RowId},
};
use unnamed_entity::{EntityId, EntityVec};

use crate::{
    bels,
    chip::{Chip, ChipKind, IoGroupKind, PllLoc, RowKind, SpecialLocKey},
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
                if cell.col == self.chip.col_w() || cell.col == self.chip.col_e() {
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

    fn fill_ebr_ecp(&mut self) {
        let ebr_width = match self.chip.kind {
            ChipKind::Ecp | ChipKind::Xp => 2,
            ChipKind::MachXo => unreachable!(),
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 | ChipKind::Ecp3 | ChipKind::Ecp3A => {
                3
            }
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
            ChipKind::MachXo | ChipKind::Xp => unreachable!(),
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 | ChipKind::Ecp3 | ChipKind::Ecp3A => {
                9
            }
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

    fn fill_int_io(&mut self, cell: CellCoord) {
        match self.chip.kind {
            ChipKind::Ecp | ChipKind::Xp => {
                if cell.row == self.chip.row_s() || cell.row == self.chip.row_n() {
                    self.egrid.add_tile_single(cell, "INT_IO_SN");
                } else {
                    self.egrid.add_tile_single(cell, "INT_IO_WE");
                }
            }
            ChipKind::MachXo | ChipKind::Ecp3 | ChipKind::Ecp3A => unreachable!(),
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => {
                if cell.row == self.chip.row_n() {
                    self.egrid.add_tile_single(cell, "INT_IO_N");
                } else if cell.row == self.chip.row_s() {
                    self.egrid.add_tile_single(cell, "INT_IO_S");
                } else {
                    self.egrid.add_tile_single(cell, "INT_IO_WE");
                }
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

    fn fill_clk_ecp3(&mut self) {
        let mut row_pclk = self.chip.row_s();
        let mut row_sclk = self.chip.row_s();
        let mut rows_sclk = EntityVec::new();
        let mut rows_pclk = EntityVec::new();
        for (row, rd) in &self.chip.rows {
            if rd.sclk_break {
                row_sclk = row;
            }
            if rd.pclk_break {
                row_pclk = row;
            }
            rows_sclk.push(row_sclk);
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
                    && (cell_src.col < self.chip.col_clk) == (cell.col < self.chip.col_clk)
                {
                    cell_src
                } else if let Some(cell_src) = self.egrid.cell_delta(cell, dx_alt, 0)
                    && (cell_src.col < self.chip.col_clk) == (cell.col < self.chip.col_clk)
                {
                    cell_src
                } else {
                    cell
                };
                let cell_src_pclk = cell_src.with_row(rows_pclk[cell.row]);
                self.egrid[cell].region_root[regions::PCLK[i]] = cell_src_pclk;
            }
        }

        for (row, rd) in &self.chip.rows {
            if !rd.pclk_drive {
                continue;
            }
            for cell in self.egrid.row(self.die, row) {
                let idx = self.chip.col_sclk_idx(cell.col);
                let kind = if cell.col == self.chip.col_w() || cell.col == self.chip.col_clk {
                    format!("PCLK{idx}_SOURCE_W")
                } else if cell.col == self.chip.col_e() || cell.col == self.chip.col_clk - 1 {
                    format!("PCLK{idx}_SOURCE_E")
                } else {
                    format!("PCLK{idx}_SOURCE")
                };
                self.egrid.add_tile(cell, &kind, &[cell, cell.delta(0, -1)]);
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
                        if cell.col == self.chip.col_clk {
                            "HSDCLK_SPLITTER"
                        } else {
                            "HSDCLK_ROOT"
                        },
                        &cells,
                    );
                }
            }
        }

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

    fn fill_config_xp(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Config];
        self.egrid.add_tile_single(cell, "CONFIG");
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
                expander.fill_clk_ecp3();
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
