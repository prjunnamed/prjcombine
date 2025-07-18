use std::collections::BTreeMap;

use prjcombine_interconnect::{
    db::IntDb,
    dir::{Dir, DirH, DirHV, DirV},
    grid::{CellCoord, ColId, DieId, ExpandedDieRefMut, ExpandedGrid, Rect, RowId},
};
use unnamed_entity::EntityId;

use crate::{
    chip::{Chip, ChipKind, IoKind, RowKind, SpecialLocKey},
    expanded::{ExpandedDevice, REGION_PCLK},
    tslots,
};

struct Expander<'a, 'b> {
    chip: &'b Chip,
    die: ExpandedDieRefMut<'a, 'b>,
    holes: Vec<Rect>,
    config: Option<CellCoord>,
    plls: BTreeMap<DirHV, CellCoord>,
    dqs: BTreeMap<CellCoord, CellCoord>,
}

impl Expander<'_, '_> {
    fn is_in_hole(&self, col: ColId, row: RowId) -> bool {
        for hole in &self.holes {
            if hole.contains(col, row) {
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
            for col in self.chip.columns.ids() {
                if col == self.chip.col_w() || col == self.chip.col_e() {
                    continue;
                }
                if self.is_in_hole(col, row) {
                    continue;
                }
                self.die.add_tile((col, row), "INT_PLC", &[(col, row)]);
                self.die.add_tile((col, row), tcls, &[(col, row)]);
            }
        }
    }

    fn fill_ebr_ecp(&mut self) {
        let mut idx = 0;
        for (row, rd) in &self.chip.rows {
            if rd.kind != RowKind::Ebr {
                continue;
            }
            for col in self.chip.columns.ids() {
                let cell = CellCoord::new(self.die.die, col, row);
                let is_int_pll = match self.chip.kind {
                    ChipKind::Ecp => {
                        (col >= self.chip.col_w() + 1 && col < self.chip.col_w() + 5)
                            || (col >= self.chip.col_e() - 4 && col < self.chip.col_e())
                    }
                    ChipKind::Xp if self.chip.col_clk.to_idx() % 2 == 0 => {
                        (col >= self.chip.col_w() + 2 && col < self.chip.col_w() + 8)
                            || (col >= self.chip.col_e() - 7 && col < self.chip.col_e() - 1)
                    }
                    ChipKind::Xp if self.chip.col_clk.to_idx() % 2 == 1 => {
                        (col >= self.chip.col_w() + 2 && col < self.chip.col_w() + 7)
                            || (col >= self.chip.col_e() - 6 && col < self.chip.col_e() - 1)
                    }
                    _ => unreachable!(),
                };
                if is_int_pll {
                    self.die.add_tile((col, row), "INT_PLL", &[(col, row)]);
                } else {
                    self.die.add_tile((col, row), "INT_EBR", &[(col, row)]);
                }
                let mut sn = [DirV::S, DirV::N][idx];
                if self.chip.kind == ChipKind::Xp && row >= self.chip.row_clk {
                    sn = DirV::N;
                }
                if col == self.chip.col_w() {
                    self.die.add_tile((col, row), "PLL_W", &[(col, row)]);
                    self.plls.insert(DirHV { h: DirH::W, v: sn }, cell);
                    continue;
                } else if col == self.chip.col_e() {
                    self.die.add_tile((col, row), "PLL_E", &[(col, row)]);
                    self.plls.insert(DirHV { h: DirH::E, v: sn }, cell);
                    continue;
                }
                if self.chip.kind == ChipKind::Xp
                    && (col == self.chip.col_w() + 1 || col == self.chip.col_e() - 1)
                {
                    continue;
                }
                if is_int_pll {
                    continue;
                }
                if self.chip.kind == ChipKind::Ecp
                    && idx == 0
                    && col >= self.chip.col_clk
                    && col < self.chip.col_clk + 4
                {
                    if col == self.chip.col_clk {
                        if row == self.chip.row_clk {
                            let crd: [_; 4] = core::array::from_fn(|i| (col + i, row));
                            self.die.add_tile((col, row), "CONFIG_S", &crd);
                        } else {
                            let crd: [_; 5] = core::array::from_fn(|i| {
                                if i < 4 {
                                    (col + i, row)
                                } else {
                                    (col, self.chip.row_clk)
                                }
                            });
                            self.die.add_tile((col, row), "CONFIG_L", &crd);
                        }
                        self.config = Some(cell);
                    }
                    continue;
                }
                if col.to_idx() % 2 == self.chip.col_clk.to_idx() % 2 {
                    let crd: [_; 2] = core::array::from_fn(|i| (col + i, row));
                    self.die.add_tile((col, row), "EBR", &crd);
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
            for col in self.chip.columns.ids() {
                self.die.add_tile((col, row), "INT_EBR", &[(col, row)]);
                if col == self.chip.col_w() || col == self.chip.col_e() {
                    continue;
                }
                if col.to_idx() % 8 == 1 {
                    let crd: [_; 8] = core::array::from_fn(|i| (col + i, row));
                    self.die.add_tile((col, row), "DSP", &crd);
                }
            }
        }
    }

    fn fill_io_ecp(&mut self) {
        for (row, rd) in &self.chip.rows {
            if !matches!(rd.kind, RowKind::Plc | RowKind::Fplc) {
                continue;
            }
            let col = self.chip.col_w();
            self.die.add_tile((col, row), "INT_IO_WE", &[(col, row)]);
            self.die.add_tile((col, row), "IO_W", &[(col, row)]);
            if rd.io_w == IoKind::DoubleDqs {
                self.die.add_tile((col, row), "DQS_W", &[(col, row)]);
                let row_base: RowId = match self.chip.kind {
                    ChipKind::Ecp => row - 3,
                    ChipKind::Xp => row - 2,
                    _ => unreachable!(),
                };
                for row_io in row_base.range(row_base + 8) {
                    self.dqs.insert(
                        CellCoord::new(DieId::from_idx(0), col, row_io),
                        CellCoord::new(DieId::from_idx(0), col, row),
                    );
                }
            }
            let col = self.chip.col_e();
            self.die.add_tile((col, row), "INT_IO_WE", &[(col, row)]);
            self.die.add_tile((col, row), "IO_E", &[(col, row)]);
            if rd.io_e == IoKind::DoubleDqs {
                self.die.add_tile((col, row), "DQS_E", &[(col, row)]);
                let row_base: RowId = match self.chip.kind {
                    ChipKind::Ecp => row - 3,
                    ChipKind::Xp => row - 2,
                    _ => unreachable!(),
                };
                for row_io in row_base.range(row_base + 8) {
                    self.dqs.insert(
                        CellCoord::new(DieId::from_idx(0), col, row_io),
                        CellCoord::new(DieId::from_idx(0), col, row),
                    );
                }
            }
        }
        for (col, cd) in &self.chip.columns {
            let row = self.chip.row_s();
            self.die.add_tile((col, row), "INT_IO_SN", &[(col, row)]);
            self.die.add_tile((col, row), "IO_S", &[(col, row)]);
            if cd.io_s == IoKind::DoubleDqs {
                self.die.add_tile((col, row), "DQS_S", &[(col, row)]);
                let col_base: ColId = match self.chip.kind {
                    ChipKind::Ecp => col - 4,
                    ChipKind::Xp => col - 5,
                    _ => unreachable!(),
                };
                for col_io in col_base.range(col_base + 8) {
                    self.dqs.insert(
                        CellCoord::new(DieId::from_idx(0), col_io, row),
                        CellCoord::new(DieId::from_idx(0), col, row),
                    );
                }
            }
            let row = self.chip.row_n();
            self.die.add_tile((col, row), "INT_IO_SN", &[(col, row)]);
            self.die.add_tile((col, row), "IO_N", &[(col, row)]);
            if cd.io_n == IoKind::DoubleDqs {
                self.die.add_tile((col, row), "DQS_N", &[(col, row)]);
                let col_base: ColId = match self.chip.kind {
                    ChipKind::Ecp => col - 4,
                    ChipKind::Xp => col - 5,
                    _ => unreachable!(),
                };
                for col_io in col_base.range(col_base + 8) {
                    self.dqs.insert(
                        CellCoord::new(DieId::from_idx(0), col_io, row),
                        CellCoord::new(DieId::from_idx(0), col, row),
                    );
                }
            }
        }
        let col = self.chip.bel_dqsdll(DirV::S).col;
        let row = self.chip.bel_dqsdll(DirV::S).row;
        self.die.add_tile((col, row), "DQSDLL_S", &[(col, row)]);
        let col = self.chip.bel_dqsdll(DirV::N).col;
        let row = self.chip.bel_dqsdll(DirV::N).row;
        self.die.add_tile((col, row), "DQSDLL_N", &[(col, row)]);
    }

    fn fill_clk_ecp(&mut self) {
        for col in self.chip.columns.ids() {
            for row in self.chip.rows.ids() {
                let col_pclk = if col < self.chip.col_clk {
                    self.chip.col_w()
                } else {
                    self.chip.col_e()
                };
                let row_pclk = if row < self.chip.row_clk {
                    self.chip.row_s()
                } else {
                    self.chip.row_n()
                };
                self.die[(col, row)].region_root[REGION_PCLK] = (col_pclk, row_pclk);
            }
        }

        let ebr_rows = Vec::from_iter(
            self.chip
                .rows
                .ids()
                .filter(|&row| self.chip.rows[row].kind == RowKind::Ebr),
        );
        let mut cells = vec![
            // actual clock root cells
            (self.chip.col_w(), self.chip.row_s()),
            (self.chip.col_e(), self.chip.row_s()),
            (self.chip.col_w(), self.chip.row_n()),
            (self.chip.col_e(), self.chip.row_n()),
        ];
        // DCS select inputs
        match self.chip.kind {
            ChipKind::Ecp if ebr_rows.len() == 1 => {
                cells.extend([
                    (self.chip.col_w(), ebr_rows[0]),
                    (self.chip.col_e(), ebr_rows[0]),
                ]);
            }
            ChipKind::Ecp if ebr_rows.len() == 2 => {
                cells.extend([
                    (self.chip.col_w() + 5, ebr_rows[0]),
                    (self.chip.col_w() + 6, ebr_rows[0]),
                    (self.chip.col_e() - 6, ebr_rows[0]),
                    (self.chip.col_e() - 5, ebr_rows[0]),
                    (self.chip.col_w() + 5, ebr_rows[1]),
                    (self.chip.col_w() + 6, ebr_rows[1]),
                    (self.chip.col_e() - 6, ebr_rows[1]),
                    (self.chip.col_e() - 5, ebr_rows[1]),
                ]);
            }
            ChipKind::Xp if ebr_rows.len() == 1 => {
                cells.extend([
                    (self.chip.col_w(), ebr_rows[0]),
                    (self.chip.col_w() + 1, ebr_rows[0]),
                    (self.chip.col_e() - 1, ebr_rows[0]),
                    (self.chip.col_e(), ebr_rows[0]),
                ]);
            }
            ChipKind::Xp if ebr_rows.len() == 2 => {
                cells.extend([
                    (self.chip.col_w(), ebr_rows[0]),
                    (self.chip.col_w() + 1, ebr_rows[0]),
                    (self.chip.col_e() - 1, ebr_rows[0]),
                    (self.chip.col_e(), ebr_rows[0]),
                    (self.chip.col_w(), ebr_rows[1]),
                    (self.chip.col_w() + 1, ebr_rows[1]),
                    (self.chip.col_e() - 1, ebr_rows[1]),
                    (self.chip.col_e(), ebr_rows[1]),
                ]);
            }
            _ => unreachable!(),
        }
        // fabric clock inputs
        for (&key, &cell) in &self.chip.special_loc {
            if !matches!(key, SpecialLocKey::PclkIn(..) | SpecialLocKey::SclkIn(..)) {
                continue;
            }
            cells.push((cell.col, cell.row));
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
        self.die
            .add_tile((self.chip.col_clk, self.chip.row_clk), kind, &cells);
    }

    fn fill_config_xp(&mut self) {
        let cell = self.chip.special_loc[&SpecialLocKey::Config];
        let col = cell.col;
        let row = cell.row;
        self.die.add_tile((col, row), "CONFIG", &[(col, row)]);
        self.config = Some(cell);
    }

    fn fill_io_machxo(&mut self) {
        let has_ebr = self.chip.special_loc.contains_key(&SpecialLocKey::Ebr(0));
        for (row, rd) in &self.chip.rows {
            if !matches!(rd.kind, RowKind::Plc | RowKind::Fplc) {
                continue;
            }

            let col = self.chip.col_w();
            let int_kind = if has_ebr {
                "INT_SIO_XW"
            } else if row >= self.chip.row_clk - 3 && row < self.chip.row_clk + 3 {
                "INT_SIO_W_CLK"
            } else {
                "INT_SIO_W"
            };
            self.die.add_tile((col, row), int_kind, &[(col, row)]);
            let kind = match (has_ebr, rd.io_w) {
                (true, IoKind::Double) => "SIO_XW2",
                (true, IoKind::Quad | IoKind::QuadReverse) => "SIO_XW4",
                (false, IoKind::Double) => "SIO_W2",
                (false, IoKind::Quad) => "SIO_W4",
                _ => unreachable!(),
            };
            self.die.add_tile((col, row), kind, &[(col, row)]);

            let col = self.chip.col_e();
            let int_kind = if self.chip.special_loc[&SpecialLocKey::Config].row == row {
                "INT_SIO_E_CFG"
            } else {
                "INT_SIO_E"
            };
            self.die.add_tile((col, row), int_kind, &[(col, row)]);
            let kind = match rd.io_e {
                IoKind::Double => "SIO_E2",
                IoKind::Quad => "SIO_E4",
                _ => unreachable!(),
            };
            self.die.add_tile((col, row), kind, &[(col, row)]);
        }
        for (col, cd) in &self.chip.columns {
            if col == self.chip.col_w() || col == self.chip.col_e() {
                continue;
            }
            let row = self.chip.row_s();
            let (int_kind, kind) = match cd.io_s {
                IoKind::Quad => ("INT_SIO_S4", "SIO_S4"),
                IoKind::Hex | IoKind::HexReverse => ("INT_SIO_S6", "SIO_S6"),
                _ => unreachable!(),
            };
            self.die.add_tile((col, row), int_kind, &[(col, row)]);
            self.die.add_tile((col, row), kind, &[(col, row)]);

            let row = self.chip.row_n();
            let (int_kind, kind) = match cd.io_n {
                IoKind::Quad => ("INT_SIO_N4", "SIO_N4"),
                IoKind::Hex | IoKind::HexReverse => ("INT_SIO_N6", "SIO_N6"),
                _ => unreachable!(),
            };
            self.die.add_tile((col, row), int_kind, &[(col, row)]);
            self.die.add_tile((col, row), kind, &[(col, row)]);
        }
    }

    fn fill_special_machxo(&mut self) {
        let has_ebr = self.chip.special_loc.contains_key(&SpecialLocKey::Ebr(0));
        for (&key, &cell) in &self.chip.special_loc {
            let col = cell.col;
            let row = cell.row;
            match key {
                SpecialLocKey::Pll(which) => {
                    let kind = match which.v {
                        DirV::S => "PLL_S",
                        DirV::N => "PLL_N",
                    };
                    self.die.add_tile((col, row), kind, &[(col, row)]);
                    self.plls.insert(which, cell);
                }
                SpecialLocKey::Ebr(_) => {
                    let crd: [_; 4] = core::array::from_fn(|i| (col, row + i));
                    self.die.add_tile((col, row), "EBR", &crd);
                }
                SpecialLocKey::Config => {
                    let crd: [_; 5] = core::array::from_fn(|i| (col, row + i));
                    self.die.add_tile((col, row), "CONFIG", &crd);
                    self.config = Some(cell);
                }
                SpecialLocKey::Osc => {
                    let kind = if has_ebr { "OSC_X" } else { "OSC" };
                    self.die.add_tile((col, row), kind, &[(col, row)]);
                }
                _ => unreachable!(),
            }
        }
        if has_ebr {
            let bel = self.chip.bel_cibtest_sel();
            let col = bel.col;
            let row = bel.row;
            self.die.add_tile((col, row), "CIBTEST_SEL", &[(col, row)]);
        }
    }

    fn fill_clk_machxo(&mut self) {
        for col in self.chip.columns.ids() {
            for row in self.chip.rows.ids() {
                self.die[(col, row)].region_root[REGION_PCLK] =
                    (self.chip.col_clk, self.chip.row_clk);
            }
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
        let crd: [_; 6] = core::array::from_fn(|i| (self.chip.col_w(), self.chip.row_clk - 3 + i));
        self.die
            .add_tile((self.chip.col_clk, self.chip.row_clk), kind, &crd);
    }

    fn fill_conns(&mut self) {
        for col in self.die.cols() {
            for row in self.die.rows() {
                if !self.die[(col, row)].tiles.contains_id(tslots::INT) {
                    continue;
                }
                if col != self.chip.col_w()
                    && self.die[(col - 1, row)].tiles.contains_id(tslots::INT)
                {
                    self.die
                        .fill_conn_pair((col - 1, row), (col, row), "PASS_E", "PASS_W");
                } else {
                    self.die.fill_conn_term((col, row), "TERM_W");
                }
                if col == self.chip.col_e()
                    || !self.die[(col + 1, row)].tiles.contains_id(tslots::INT)
                {
                    self.die.fill_conn_term((col, row), "TERM_E");
                }
                if row != self.chip.row_s()
                    && self.die[(col, row - 1)].tiles.contains_id(tslots::INT)
                {
                    self.die
                        .fill_conn_pair((col, row - 1), (col, row), "PASS_N", "PASS_S");
                } else {
                    self.die.fill_conn_term((col, row), "TERM_S");
                }
                if row == self.chip.row_n()
                    || !self.die[(col, row + 1)].tiles.contains_id(tslots::INT)
                {
                    self.die.fill_conn_term((col, row), "TERM_N");
                }
            }
        }
    }
}

impl Chip {
    pub fn expand_grid<'a>(&'a self, db: &'a IntDb) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        let (_, die) = egrid.add_die(self.columns.len(), self.rows.len());
        let mut expander = Expander {
            chip: self,
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
