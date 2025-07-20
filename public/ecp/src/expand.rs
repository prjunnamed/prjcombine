#![allow(clippy::int_plus_one)]

use std::collections::BTreeMap;

use prjcombine_interconnect::{
    db::IntDb,
    dir::{Dir, DirH, DirHV, DirV},
    grid::{CellCoord, ColId, DieId, ExpandedGrid, Rect, RowId},
};
use unnamed_entity::EntityId;

use crate::{
    chip::{Chip, ChipKind, IoKind, RowKind, SpecialLocKey},
    expanded::{ExpandedDevice, REGION_PCLK},
    tslots,
};

struct Expander<'a, 'b> {
    chip: &'b Chip,
    die: DieId,
    egrid: &'a mut ExpandedGrid<'b>,
    holes: Vec<Rect>,
    config: Option<CellCoord>,
    plls: BTreeMap<DirHV, CellCoord>,
    dqs: BTreeMap<CellCoord, CellCoord>,
}

impl Expander<'_, '_> {
    fn is_in_hole(&self, cell: CellCoord) -> bool {
        for hole in &self.holes {
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
                if self.is_in_hole(cell) {
                    continue;
                }
                self.egrid.add_tile_single(cell, "INT_PLC");
                self.egrid.add_tile_single(cell, tcls);
            }
        }
    }

    fn fill_ebr_ecp(&mut self) {
        let mut idx = 0;
        for (row, rd) in &self.chip.rows {
            if rd.kind != RowKind::Ebr {
                continue;
            }
            for cell in self.egrid.row(self.die, row) {
                let is_int_pll = match self.chip.kind {
                    ChipKind::Ecp => {
                        (cell.col >= self.chip.col_w() + 1 && cell.col < self.chip.col_w() + 5)
                            || (cell.col >= self.chip.col_e() - 4 && cell.col < self.chip.col_e())
                    }
                    ChipKind::Xp if self.chip.col_clk.to_idx().is_multiple_of(2) => {
                        (cell.col >= self.chip.col_w() + 2 && cell.col < self.chip.col_w() + 8)
                            || (cell.col >= self.chip.col_e() - 7
                                && cell.col < self.chip.col_e() - 1)
                    }
                    ChipKind::Xp if !self.chip.col_clk.to_idx().is_multiple_of(2) => {
                        (cell.col >= self.chip.col_w() + 2 && cell.col < self.chip.col_w() + 7)
                            || (cell.col >= self.chip.col_e() - 6
                                && cell.col < self.chip.col_e() - 1)
                    }
                    _ => unreachable!(),
                };
                if is_int_pll {
                    self.egrid.add_tile_single(cell, "INT_PLL");
                } else {
                    self.egrid.add_tile_single(cell, "INT_EBR");
                }
                let mut sn = [DirV::S, DirV::N][idx];
                if self.chip.kind == ChipKind::Xp && row >= self.chip.row_clk {
                    sn = DirV::N;
                }
                if cell.col == self.chip.col_w() {
                    self.egrid.add_tile_single(cell, "PLL_W");
                    self.plls.insert(DirHV { h: DirH::W, v: sn }, cell);
                    continue;
                } else if cell.col == self.chip.col_e() {
                    self.egrid.add_tile_single(cell, "PLL_E");
                    self.plls.insert(DirHV { h: DirH::E, v: sn }, cell);
                    continue;
                }
                if self.chip.kind == ChipKind::Xp
                    && (cell.col == self.chip.col_w() + 1 || cell.col == self.chip.col_e() - 1)
                {
                    continue;
                }
                if is_int_pll {
                    continue;
                }
                if self.chip.kind == ChipKind::Ecp
                    && idx == 0
                    && cell.col >= self.chip.col_clk
                    && cell.col < self.chip.col_clk + 4
                {
                    if cell.col == self.chip.col_clk {
                        if row == self.chip.row_clk {
                            self.egrid.add_tile_e(cell, "CONFIG_S", 4);
                        } else {
                            let crd: [_; 5] = core::array::from_fn(|i| {
                                if i < 4 {
                                    cell.delta(i as i32, 0)
                                } else {
                                    cell.with_row(self.chip.row_clk)
                                }
                            });
                            self.egrid.add_tile(cell, "CONFIG_L", &crd);
                        }
                        self.config = Some(cell);
                    }
                    continue;
                }
                if cell.col.to_idx() % 2 == self.chip.col_clk.to_idx() % 2 {
                    self.egrid.add_tile_e(cell, "EBR", 2);
                }
            }
            idx += 1;
        }
    }

    fn fill_dsp_ecp(&mut self) {
        for (row, rd) in &self.chip.rows {
            if rd.kind != RowKind::Dsp {
                continue;
            }
            for cell in self.egrid.row(self.die, row) {
                self.egrid.add_tile_single(cell, "INT_EBR");
                if cell.col == self.chip.col_w() || cell.col == self.chip.col_e() {
                    continue;
                }
                if cell.col.to_idx() % 8 == 1 {
                    let tcells: [_; 8] = core::array::from_fn(|i| cell.delta(i as i32, 0));
                    self.egrid.add_tile(cell, "DSP", &tcells);
                }
            }
        }
    }

    fn fill_io_ecp(&mut self) {
        for cell in self.egrid.column(self.die, self.chip.col_w()) {
            let rd = &self.chip.rows[cell.row];
            if !matches!(rd.kind, RowKind::Plc | RowKind::Fplc) {
                continue;
            }
            self.egrid.add_tile_single(cell, "INT_IO_WE");
            self.egrid.add_tile_single(cell, "IO_W");
            if rd.io_w == IoKind::DoubleDqs {
                self.egrid.add_tile_single(cell, "DQS_W");
                let row_base: RowId = match self.chip.kind {
                    ChipKind::Ecp => cell.row - 3,
                    ChipKind::Xp => cell.row - 2,
                    _ => unreachable!(),
                };
                for row_io in row_base.range(row_base + 8) {
                    self.dqs.insert(cell.with_row(row_io), cell);
                }
            }
        }

        for cell in self.egrid.column(self.die, self.chip.col_e()) {
            let rd = &self.chip.rows[cell.row];
            if !matches!(rd.kind, RowKind::Plc | RowKind::Fplc) {
                continue;
            }
            self.egrid.add_tile_single(cell, "INT_IO_WE");
            self.egrid.add_tile_single(cell, "IO_E");
            if rd.io_e == IoKind::DoubleDqs {
                self.egrid.add_tile_single(cell, "DQS_E");
                let row_base: RowId = match self.chip.kind {
                    ChipKind::Ecp => cell.row - 3,
                    ChipKind::Xp => cell.row - 2,
                    _ => unreachable!(),
                };
                for row_io in row_base.range(row_base + 8) {
                    self.dqs.insert(cell.with_row(row_io), cell);
                }
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_s()) {
            let cd = &self.chip.columns[cell.col];
            self.egrid.add_tile_single(cell, "INT_IO_SN");
            if cell.col == self.chip.col_w() || cell.col == self.chip.col_e() {
                continue;
            }
            self.egrid.add_tile_single(cell, "IO_S");
            if cd.io_s == IoKind::DoubleDqs {
                self.egrid.add_tile_single(cell, "DQS_S");
                let col_base: ColId = match self.chip.kind {
                    ChipKind::Ecp => cell.col - 4,
                    ChipKind::Xp => cell.col - 5,
                    _ => unreachable!(),
                };
                for col_io in col_base.range(col_base + 8) {
                    self.dqs.insert(cell.with_col(col_io), cell);
                }
            }
        }
        for cell in self.egrid.row(self.die, self.chip.row_n()) {
            let cd = &self.chip.columns[cell.col];
            self.egrid.add_tile_single(cell, "INT_IO_SN");
            if cell.col == self.chip.col_w() || cell.col == self.chip.col_e() {
                continue;
            }
            self.egrid.add_tile_single(cell, "IO_N");
            if cd.io_n == IoKind::DoubleDqs {
                self.egrid.add_tile_single(cell, "DQS_N");
                let col_base: ColId = match self.chip.kind {
                    ChipKind::Ecp => cell.col - 4,
                    ChipKind::Xp => cell.col - 5,
                    _ => unreachable!(),
                };
                for col_io in col_base.range(col_base + 8) {
                    self.dqs.insert(cell.with_col(col_io), cell);
                }
            }
        }
        let cell = self.chip.bel_dqsdll(DirV::S).cell;
        self.egrid.add_tile_single(cell, "DQSDLL_S");
        let cell = self.chip.bel_dqsdll(DirV::N).cell;
        self.egrid.add_tile_single(cell, "DQSDLL_N");
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
            self.egrid[cell].region_root[REGION_PCLK] =
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

    fn fill_config_xp(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Config];
        self.egrid.add_tile_single(cell, "CONFIG");
        self.config = Some(cell);
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
                (true, IoKind::Double) => "SIO_XW2",
                (true, IoKind::Quad | IoKind::QuadReverse) => "SIO_XW4",
                (false, IoKind::Double) => "SIO_W2",
                (false, IoKind::Quad) => "SIO_W4",
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
                IoKind::Double => "SIO_E2",
                IoKind::Quad => "SIO_E4",
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
                IoKind::Quad => ("INT_SIO_S4", "SIO_S4"),
                IoKind::Hex | IoKind::HexReverse => ("INT_SIO_S6", "SIO_S6"),
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
                IoKind::Quad => ("INT_SIO_N4", "SIO_N4"),
                IoKind::Hex | IoKind::HexReverse => ("INT_SIO_N6", "SIO_N6"),
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
                    let kind = match which.v {
                        DirV::S => "PLL_S",
                        DirV::N => "PLL_N",
                    };
                    self.egrid.add_tile_single(cell, kind);
                    self.plls.insert(which, cell);
                }
                SpecialLocKey::Ebr(_) => {
                    self.egrid.add_tile_n(cell, "EBR", 4);
                }
                SpecialLocKey::Config => {
                    self.egrid.add_tile_n(cell, "CONFIG", 5);
                    self.config = Some(cell);
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
            self.egrid[cell].region_root[REGION_PCLK] =
                CellCoord::new(self.die, self.chip.col_clk, self.chip.row_clk);
        }
        let kind = if self
            .chip
            .special_loc
            .contains_key(&SpecialLocKey::Pll(DirHV::NW))
        {
            "CLK_ROOT_2PLL"
        } else if self
            .chip
            .special_loc
            .contains_key(&SpecialLocKey::Pll(DirHV::SW))
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
            holes: vec![],
            config: None,
            plls: BTreeMap::new(),
            dqs: BTreeMap::new(),
        };

        expander.fill_plc();
        match self.kind {
            ChipKind::Ecp => {
                expander.fill_ebr_ecp();
                expander.fill_dsp_ecp();
                expander.fill_io_ecp();
                expander.fill_clk_ecp();
            }
            ChipKind::Xp => {
                expander.fill_ebr_ecp();
                expander.fill_config_xp();
                expander.fill_io_ecp();
                expander.fill_clk_ecp();
            }
            ChipKind::MachXo => {
                expander.fill_io_machxo();
                expander.fill_special_machxo();
                expander.fill_clk_machxo();
            }
        }
        expander.fill_conns();

        let holes = expander.holes;
        let config = expander.config.unwrap();
        let plls = expander.plls;
        let dqs = expander.dqs;

        egrid.finish();
        ExpandedDevice {
            chip: self,
            egrid,
            holes,
            config,
            plls,
            dqs,
        }
    }
}
