use std::collections::{BTreeMap, BTreeSet, HashMap, btree_map};

use prjcombine_ecp::{
    bels,
    chip::{
        Chip, ChipKind, Column, IoGroupKind, MachXo2Kind, PllLoc, PllPad, Row, RowKind,
        SpecialIoKey, SpecialLocKey,
    },
};
use prjcombine_interconnect::{
    db::PinDir,
    dir::{Dir, DirH, DirHV, DirV},
    grid::{CellCoord, ColId, DieId, EdgeIoCoord, RowId, TileIobId},
};
use prjcombine_re_lattice_naming::{ChipNaming, WireName};
use prjcombine_re_lattice_rawdump::{Grid, NodeId};
use prjcombine_entity::{EntityId, EntityVec};

pub trait ChipExt {
    fn xlat_row(&self, r: u8) -> RowId;
    fn xlat_col(&self, c: u8) -> ColId;
    fn xlat_rc(&self, r: u8, c: u8) -> CellCoord;
    fn xlat_rc_wire(&self, wire: WireName) -> CellCoord;
}

impl ChipExt for Chip {
    fn xlat_row(&self, r: u8) -> RowId {
        RowId::from_idx(self.rows.len() - usize::from(r))
    }

    fn xlat_col(&self, c: u8) -> ColId {
        ColId::from_idx((c - 1).into())
    }

    fn xlat_rc(&self, r: u8, c: u8) -> CellCoord {
        let c = if c == 0 {
            1
        } else if c as usize == self.columns.len() + 1 {
            c - 1
        } else {
            c
        };
        let r = if r == 0 {
            1
        } else if r as usize == self.rows.len() + 1 {
            r - 1
        } else {
            r
        };
        let die = DieId::from_idx(0);
        let col = self.xlat_col(c);
        let row = self.xlat_row(r);
        CellCoord::new(die, col, row)
    }

    fn xlat_rc_wire(&self, wire: WireName) -> CellCoord {
        self.xlat_rc(wire.r, wire.c)
    }
}

struct ChipBuilder<'a> {
    name: &'a str,
    chip: Chip,
    grid: &'a Grid,
    naming: &'a ChipNaming,
    nodes: &'a EntityVec<NodeId, WireName>,
}

impl ChipBuilder<'_> {
    fn fill_ebr_dsp_rows(&mut self) {
        for wn in self.nodes.values() {
            if self.naming.strings[wn.suffix].ends_with("EBR") && self.chip.kind != ChipKind::MachXo
            {
                let row = self.chip.xlat_row(wn.r);
                if row != self.chip.row_n() && row != self.chip.row_s() {
                    self.chip.rows[row].kind = RowKind::Ebr;
                }
            }
            if self.naming.strings[wn.suffix].ends_with("MULT9") {
                let row = self.chip.xlat_row(wn.r);
                self.chip.rows[row].kind = RowKind::Dsp;
            }
        }
    }

    fn fill_plc_rows(&mut self) {
        for tile in &self.grid.tiles {
            let kind = match tile.kind.as_str() {
                "PLC" | "PLC2" | "PLC_CR" => RowKind::Plc,
                "FPLC" => RowKind::Fplc,
                _ => continue,
            };
            let (r, _c) = tile
                .name
                .strip_prefix('R')
                .unwrap()
                .split_once('C')
                .unwrap();
            let row = self.chip.xlat_row(r.parse().unwrap());
            self.chip.rows[row].kind = kind;
        }
    }

    fn fill_kind_ecp3(&mut self) {
        for &wn in self.nodes.values() {
            if self.naming.strings[wn.suffix] == "JECLKCNT_DQSTEST" {
                self.chip.kind = ChipKind::Ecp3A;
            }
        }
    }

    fn fill_kind_machxo2(&mut self) {
        let mut has_riologic = false;
        let mut has_slewrate = false;
        let mut has_icc = false;
        let mut has_esb = false;
        for &wn in self.nodes.values() {
            if self.naming.strings[wn.suffix] == "JCLKA_RIOLOGIC" {
                has_riologic = true;
            }
            if self.naming.strings[wn.suffix] == "JSLEWRATEA_PIO" {
                has_slewrate = true;
            }
            if self.naming.strings[wn.suffix] == "JASFCLKI_ESB" {
                has_esb = true;
            }
            if self.naming.strings[wn.suffix] == "JRXCLK_ICC" {
                has_icc = true;
            }
        }
        if has_icc {
            self.chip.kind = ChipKind::MachXo2(MachXo2Kind::MachNx);
        } else if has_esb {
            self.chip.kind = ChipKind::MachXo2(MachXo2Kind::MachXo3D);
        } else if has_slewrate {
            self.chip.kind = ChipKind::MachXo2(MachXo2Kind::MachXo3Lfp);
        } else if !has_riologic && self.chip.rows.values().any(|rd| rd.kind == RowKind::Ebr) {
            self.chip.kind = ChipKind::MachXo2(MachXo2Kind::MachXo3L);
        }
    }

    fn fill_clk_scm(&mut self) {
        for &wn in self.nodes.values() {
            if self.naming.strings[wn.suffix] == "JCIBTESTB0_TESTCK" {
                let cell = self.chip.xlat_rc_wire(wn).delta(1, 0);
                self.chip.col_clk = cell.col;
                self.chip.row_clk = cell.row;
                return;
            }
        }
        panic!("ummm where clocks");
    }

    fn fill_clk_ecp(&mut self) {
        for &wn in self.nodes.values() {
            if self.naming.strings[wn.suffix] == "LLDCSOUT0_DCS" {
                let cell = self.chip.xlat_rc_wire(wn).delta(1, 0);
                self.chip.col_clk = cell.col;
                self.chip.row_clk = cell.row;
                return;
            }
        }
        panic!("ummm where clocks");
    }

    fn fill_clk_machxo2(&mut self) {
        for &wn in self.nodes.values() {
            if self.naming.strings[wn.suffix] == "DCMOUT6_DCM" {
                let cell = self.chip.xlat_rc_wire(wn).delta(1, 0);
                self.chip.col_clk = cell.col;
                self.chip.row_clk = cell.row;
                return;
            }
        }
        panic!("ummm where clocks");
    }

    fn fill_clk_ecp4(&mut self) {
        for &wn in self.nodes.values() {
            if self.naming.strings[wn.suffix] == "DCS0" {
                let cell = self.chip.xlat_rc_wire(wn).delta(1, 0);
                self.chip.col_clk = cell.col;
                self.chip.row_clk = cell.row;
                return;
            }
        }
        panic!("ummm where clocks");
    }

    fn fill_clk_crosslink(&mut self) {
        for &wn in self.nodes.values() {
            if self.naming.strings[wn.suffix] == "DCSOUT" {
                let cell = self.chip.xlat_rc_wire(wn).delta(1, 0);
                self.chip.col_clk = cell.col;
                self.chip.row_clk = cell.row;
                return;
            }
        }
        panic!("ummm where clocks");
    }

    fn fill_pclk_scm(&mut self) {
        let hpbx0001 = self.naming.strings.get("HPBX0001").unwrap();
        let vpsx0000 = self.naming.strings.get("VPSX0000").unwrap();
        let mut clks: BTreeMap<u8, Vec<ColId>> = BTreeMap::new();
        for &(nf, nt) in self.grid.pips.keys() {
            let wfn = self.nodes[nf];
            let wtn = self.nodes[nt];
            if wfn.suffix == vpsx0000 {
                let cell = self.chip.xlat_rc_wire(wfn);
                self.chip.columns[cell.col + 1].pclk_drive = true;
            }
            if wtn.suffix == hpbx0001 {
                let cell = self.chip.xlat_rc_wire(wtn);
                if cell.row == self.chip.row_clk {
                    clks.entry(wfn.c).or_default().push(cell.col);
                }
            }
        }
        let mut next = ColId::from_idx(0);
        for (_, mut cols) in clks {
            cols.sort();
            let col_start = cols[0];
            assert_eq!(col_start, next);
            for (i, &col) in cols.iter().enumerate() {
                assert_eq!(col, col_start + i);
            }
            if col_start.to_idx() != 0 {
                self.chip.columns[col_start].pclk_break = true;
            }
            next = col_start + cols.len();
        }
        assert!(self.chip.columns[self.chip.col_clk].pclk_break);
    }

    fn fill_pclk_ecp2(&mut self) {
        let clk0 = self.naming.strings.get("CLK0").unwrap();
        let jclk0 = self.naming.strings.get("JCLK0").unwrap();
        let hpbx0000 = self.naming.strings.get("HPBX0000").unwrap();
        let mut clks: BTreeMap<u8, Vec<ColId>> = BTreeMap::new();
        for &(nf, nt) in self.grid.pips.keys() {
            let wfn = self.nodes[nf];
            let wtn = self.nodes[nt];
            if wfn.suffix == hpbx0000 && (wtn.suffix == jclk0 || wtn.suffix == clk0) {
                let cell = self.chip.xlat_rc_wire(wtn);
                if cell.row == self.chip.row_clk {
                    clks.entry(wfn.c).or_default().push(cell.col);
                }
            }
        }
        let mut next = ColId::from_idx(0);
        for (_, mut cols) in clks {
            cols.sort();
            let col_start = cols[0];
            assert_eq!(col_start, next);
            for (i, &col) in cols.iter().enumerate() {
                assert_eq!(col, col_start + i);
            }
            if col_start.to_idx() != 0 {
                self.chip.columns[col_start].pclk_break = true;
            }
            next = col_start + cols.len();
        }
        assert!(self.chip.columns[self.chip.col_clk].pclk_break);
    }

    fn fill_pclk_ecp3(&mut self) {
        let idx = (self.chip.col_sclk_idx(self.chip.col_w()) + 2) % 4;
        let hpbx = self.naming.strings.get(&format!("HPBX0{idx}00")).unwrap();
        let mut clks: BTreeMap<WireName, Vec<RowId>> = BTreeMap::new();
        for &(nf, nt) in self.grid.pips.keys() {
            let wfn = self.nodes[nf];
            let wtn = self.nodes[nt];
            if wtn.suffix == hpbx && wtn.c == 1 {
                let cell = self.chip.xlat_rc_wire(wtn);
                clks.entry(wfn).or_default().push(cell.row);
            }
        }
        let mut clks = Vec::from_iter(clks);
        clks.sort_by_key(|(_, rows)| rows[0]);
        let mut next = RowId::from_idx(0);
        for (_, mut rows) in clks {
            rows.sort();
            let row_start = rows[0];
            assert_eq!(row_start, next);
            for (i, &col) in rows.iter().enumerate() {
                assert_eq!(col, row_start + i);
            }
            if row_start.to_idx() != 0 {
                self.chip.rows[row_start].pclk_break = true;
            }
            next = row_start + rows.len();
        }
        let clko0b = self.naming.strings.get("CLKO0T_DCC").unwrap();
        for &wn in self.nodes.values() {
            if wn.suffix == clko0b {
                let mut cell = self.chip.xlat_rc_wire(wn);
                if matches!(self.chip.kind, ChipKind::MachXo2(_))
                    && self.chip.rows[cell.row].kind == RowKind::Plc
                {
                    cell.row = self.chip.row_s();
                }
                self.chip.rows[cell.row].pclk_drive = true;
            }
        }
    }

    fn fill_pclk_ecp4(&mut self) {
        let hpbx = self.naming.strings.get("HPBX0000").unwrap();
        let mut clks: BTreeMap<WireName, Vec<RowId>> = BTreeMap::new();
        for &(nf, nt) in self.grid.pips.keys() {
            let wfn = self.nodes[nf];
            let wtn = self.nodes[nt];
            if wtn.suffix == hpbx && wtn.c == 1 {
                let cell = self.chip.xlat_rc_wire(wtn);
                clks.entry(wfn).or_default().push(cell.row);
            }
        }
        let mut clks = Vec::from_iter(clks);
        clks.sort_by_key(|(_, rows)| rows[0]);
        let mut next = RowId::from_idx(0);
        for (_, mut rows) in clks {
            rows.sort();
            let row_start = rows[0];
            assert_eq!(row_start, next);
            for (i, &col) in rows.iter().enumerate() {
                assert_eq!(col, row_start + i);
            }
            if row_start.to_idx() != 0 {
                self.chip.rows[row_start].pclk_break = true;
            }
            next = row_start + rows.len();
        }
    }

    fn fill_pclk_ecp5(&mut self) {
        let clk0 = self.naming.strings.get("CLK0").unwrap();
        let jclk0 = self.naming.strings.get("JCLK0").unwrap();
        let hpbx0200 = self.naming.strings.get("HPBX0200").unwrap();
        let mut clks: BTreeMap<u8, Vec<ColId>> = BTreeMap::new();
        for &(nf, nt) in self.grid.pips.keys() {
            let wfn = self.nodes[nf];
            let wtn = self.nodes[nt];
            if wfn.suffix == hpbx0200 && (wtn.suffix == jclk0 || wtn.suffix == clk0) {
                let cell = self.chip.xlat_rc_wire(wtn);
                if cell.row == self.chip.row_clk {
                    clks.entry(wfn.c).or_default().push(cell.col);
                }
            }
        }
        let mut next = ColId::from_idx(0);
        let mut drive = false;
        for (_, mut cols) in clks {
            cols.sort();
            let col_start = cols[0];
            assert_eq!(col_start, next);
            for (i, &col) in cols.iter().enumerate() {
                assert_eq!(col, col_start + i);
            }
            if col_start.to_idx() != 0 {
                self.chip.columns[col_start].pclk_break = true;
            }
            self.chip.columns[col_start].pclk_drive = drive;
            drive = !drive;
            next = col_start + cols.len();
        }
        assert!(self.chip.columns[self.chip.col_clk].pclk_break);

        if self.chip.kind == ChipKind::Crosslink {
            let col = self
                .chip
                .columns
                .ids()
                .rev()
                .find(|&col| self.chip.columns[col].pclk_break)
                .unwrap();
            self.chip.columns[col].pclk_drive = true;
        } else {
            let hprx0000 = self.naming.strings.get("HPRX0000").unwrap();
            for &wn in self.nodes.values() {
                if wn.suffix != hprx0000 {
                    continue;
                }
                let cell = self.chip.xlat_rc_wire(wn).with_col(self.chip.col_clk);
                let v = if cell.row < self.chip.row_clk {
                    DirV::S
                } else {
                    DirV::N
                };
                self.chip
                    .special_loc
                    .insert(SpecialLocKey::ClkQuarter(v), cell);
            }
        }
    }

    fn fill_sclk_ecp2(&mut self) {
        let hsbx0000 = self.naming.strings.get("HSBX0000").unwrap();
        let mut clks: BTreeMap<RowId, Vec<RowId>> = BTreeMap::new();
        for &(nf, nt) in self.grid.pips.keys() {
            let wfn = self.nodes[nf];
            let wtn = self.nodes[nt];
            if wtn.suffix == hsbx0000 {
                let cell_t = self.chip.xlat_rc_wire(wtn);
                let cell_f = self.chip.xlat_rc_wire(wfn);
                if cell_t.col == self.chip.col_clk {
                    clks.entry(cell_f.row).or_default().push(cell_t.row);
                }
            }
        }
        let mut next = RowId::from_idx(0);
        for (_, mut rows) in clks {
            rows.sort();
            let row_start = rows[0];
            assert_eq!(row_start, next);
            for (i, &row) in rows.iter().enumerate() {
                assert_eq!(row, row_start + i);
            }
            if row_start.to_idx() != 0 {
                self.chip.rows[row_start].sclk_break = true;
            }
            next = row_start + rows.len();
        }

        let hssx_l2r = self.naming.strings.get("HSSX0000_L2R").unwrap();
        for &wn in self.nodes.values() {
            if wn.suffix == hssx_l2r {
                let cell = self.chip.xlat_rc_wire(wn);
                self.chip.columns[cell.col + 1].sdclk_break = true;
            }
        }
    }

    fn fill_sclk_ecp3(&mut self) {
        let hsbx0000 = self.naming.strings.get("HSBX0000").unwrap();
        let mut clks: BTreeMap<RowId, Vec<RowId>> = BTreeMap::new();
        for &(nf, nt) in self.grid.pips.keys() {
            let wfn = self.nodes[nf];
            let wtn = self.nodes[nt];
            if wtn.suffix == hsbx0000 {
                let cell_t = self.chip.xlat_rc_wire(wtn);
                let cell_f = self.chip.xlat_rc_wire(wfn);
                if cell_t.col == self.chip.col_w() {
                    clks.entry(cell_f.row).or_default().push(cell_t.row);
                }
            }
        }
        let mut next = RowId::from_idx(0);
        for (_, mut rows) in clks {
            rows.sort();
            let row_start = rows[0];
            assert_eq!(row_start, next);
            for (i, &row) in rows.iter().enumerate() {
                assert_eq!(row, row_start + i);
            }
            if row_start.to_idx() != 0 {
                self.chip.rows[row_start].sclk_break = true;
            }
            next = row_start + rows.len();
        }

        let hssx_l2r = self.naming.strings.get("HSSX0000_L2R").unwrap();
        for &wn in self.nodes.values() {
            if wn.suffix == hssx_l2r {
                let cell = self.chip.xlat_rc_wire(wn);
                self.chip.columns[cell.col + 1].sdclk_break = true;
            }
        }
    }

    fn fill_eclk_tap_ecp2(&mut self) {
        let jf6 = self.naming.strings.get("JF6").unwrap();
        for &(nf, nt) in self.grid.pips.keys() {
            let wfn = self.nodes[nf];
            let wtn = self.nodes[nt];
            if wtn.suffix == jf6 && self.naming.strings[wfn.suffix].contains("FRC") {
                let cell = self.chip.xlat_rc_wire(wtn);
                if cell.row == self.chip.row_s() {
                    self.chip.columns[cell.col].eclk_tap_s = true;
                } else if cell.row == self.chip.row_n() {
                    self.chip.columns[cell.col].eclk_tap_n = true;
                }
            }
        }
    }

    fn fill_eclk_tap_ecp3(&mut self) {
        let jf6 = self.naming.strings.get("JF6").unwrap();
        for &(nf, nt) in self.grid.pips.keys() {
            let wfn = self.nodes[nf];
            let wtn = self.nodes[nt];
            if wtn.suffix == jf6 && self.naming.strings[wfn.suffix].contains("TECLK") {
                let cell = self.chip.xlat_rc_wire(wtn);
                if cell.row == self.chip.row_s() {
                    self.chip.columns[cell.col].eclk_tap_s = true;
                } else if cell.row == self.chip.row_n() {
                    self.chip.columns[cell.col].eclk_tap_n = true;
                }
            }
        }
    }

    fn fill_clk_machxo(&mut self) {
        for ((y, x), tile) in self.grid.tiles.indexed_iter() {
            if tile.kind.starts_with("CLK3") {
                self.chip.col_clk = ColId::from_idx(x);
                self.chip.row_clk = RowId::from_idx(self.chip.rows.len() - 1 - y);
                return;
            }
        }
        unreachable!()
    }

    fn fill_machxo_special_loc(&mut self) {
        let mut ebr_locs = BTreeSet::new();
        for &wn in self.nodes.values() {
            if self.naming.strings[wn.suffix].ends_with("_EBR") {
                let cell = self.chip.xlat_rc_wire(wn).delta(0, -3);
                ebr_locs.insert(cell);
            }
            if self.naming.strings[wn.suffix].ends_with("_PLL") {
                let cell = self.chip.xlat_rc_wire(wn);
                let loc = if cell.row < self.chip.row_clk {
                    SpecialLocKey::Pll(PllLoc::new(DirHV::SW, 0))
                } else {
                    SpecialLocKey::Pll(PllLoc::new(DirHV::NW, 0))
                };
                self.chip.special_loc.insert(loc, cell);
            }
            if self.naming.strings[wn.suffix].ends_with("_OSC") {
                let cell = self.chip.xlat_rc_wire(wn);
                self.chip.special_loc.insert(SpecialLocKey::Osc, cell);
            }
            if self.naming.strings[wn.suffix].ends_with("_JTAG") {
                let cell = self.chip.xlat_rc_wire(wn);
                self.chip.special_loc.insert(SpecialLocKey::Config, cell);
            }
        }
        for (i, cell) in ebr_locs.into_iter().enumerate() {
            self.chip
                .special_loc
                .insert(SpecialLocKey::Ebr(i.try_into().unwrap()), cell);
        }
    }

    fn fill_special_loc_crosslink(&mut self) {
        for &wn in self.nodes.values() {
            if self.naming.strings[wn.suffix].ends_with("_PLL") {
                let cell = self.chip.xlat_rc_wire(wn);
                let loc = SpecialLocKey::Pll(PllLoc::new(DirHV::SE, 0));
                self.chip.special_loc.insert(loc, cell);
            }
            if self.naming.strings[wn.suffix].ends_with("_OSC") {
                let cell = self.chip.xlat_rc_wire(wn);
                self.chip.special_loc.insert(SpecialLocKey::Osc, cell);
            }
            if self.naming.strings[wn.suffix].ends_with("_GSR") {
                let cell = self.chip.xlat_rc_wire(wn);
                self.chip.special_loc.insert(SpecialLocKey::Config, cell);
            }
            if self.naming.strings[wn.suffix].ends_with("_PMU") {
                let cell = self.chip.xlat_rc_wire(wn);
                self.chip.special_loc.insert(SpecialLocKey::Pmu, cell);
            }
        }
    }

    fn fill_config_loc_scm(&mut self) {
        let cell = CellCoord::new(
            DieId::from_idx(0),
            self.chip.col_clk - 12,
            self.chip.row_n() - 12,
        );
        self.chip.special_loc.insert(SpecialLocKey::Config, cell);
    }

    fn fill_config_loc_ecp(&mut self) {
        for &wn in self.nodes.values() {
            if self.naming.strings[wn.suffix].ends_with("_START") {
                let cell = self.chip.xlat_rc_wire(wn);
                self.chip.special_loc.insert(SpecialLocKey::Config, cell);
            }
        }
    }

    fn fill_config_loc_xp2(&mut self) {
        self.chip.special_loc.insert(
            SpecialLocKey::Osc,
            CellCoord::new(DieId::from_idx(0), self.chip.col_e(), self.chip.row_n() - 1),
        );
        let row = self
            .chip
            .rows
            .ids()
            .find(|&row| self.chip.rows[row].kind == RowKind::Dsp)
            .unwrap();
        self.chip.special_loc.insert(
            SpecialLocKey::Config,
            CellCoord::new(DieId::from_idx(0), self.chip.col_e(), row),
        );
    }

    fn fill_config_loc_ecp3(&mut self) {
        for &wn in self.nodes.values() {
            if self.naming.strings[wn.suffix].ends_with("_START") {
                let cell = self.chip.xlat_rc_wire(wn).delta(-3, 0);
                self.chip.special_loc.insert(SpecialLocKey::Config, cell);
            }
        }
    }

    fn fill_config_bits_loc_xp(&mut self) {
        for tile in &self.grid.tiles {
            match tile.kind.as_str() {
                "PIC_R_3K_CONFIG" => {
                    let row = self
                        .chip
                        .xlat_row(tile.name.strip_prefix("PR").unwrap().parse().unwrap());
                    let cell = CellCoord::new(DieId::from_idx(0), self.chip.col_e(), row);
                    self.chip
                        .special_loc
                        .insert(SpecialLocKey::ConfigBits, cell);
                }
                "PIC_L_6K_CONFIG" => {
                    let row = self
                        .chip
                        .xlat_row(tile.name.strip_prefix("PL").unwrap().parse().unwrap());
                    let cell = CellCoord::new(DieId::from_idx(0), self.chip.col_w(), row);
                    self.chip
                        .special_loc
                        .insert(SpecialLocKey::ConfigBits, cell);
                }
                _ => (),
            }
        }
    }

    fn fill_frames_xp(&mut self) {
        match self.chip.rows.len() {
            19 => {
                self.chip.extra_frames_w = 3;
                self.chip.extra_frames_e = 3;
            }
            27 => {
                self.chip.extra_frames_w = 3;
                self.chip.extra_frames_e = 11;
            }
            36 => {
                self.chip.extra_frames_w = 3;
                self.chip.extra_frames_e = 3;
                self.chip.double_frames = true;
            }
            44 => {
                self.chip.extra_frames_w = 5;
                self.chip.extra_frames_e = 5;
                self.chip.double_frames = true;
            }
            48 => {
                self.chip.extra_frames_w = 1;
                self.chip.extra_frames_e = 1;
                self.chip.double_frames = true;
            }
            _ => unreachable!(),
        }
    }

    fn fill_pll_scm(&mut self) {
        for hv in DirHV::DIRS {
            let col = self.chip.col_edge(hv.h);
            let row = match hv.v {
                DirV::S => self.chip.row_s(),
                DirV::N => self.chip.row_n() - 12,
            };
            let cell = CellCoord::new(DieId::from_idx(0), col, row);
            self.chip
                .special_loc
                .insert(SpecialLocKey::Pll(PllLoc::new(hv, 0)), cell);
        }
    }

    fn fill_pll_ecp(&mut self) {
        for edge in [DirH::W, DirH::E] {
            for (row, rd) in &self.chip.rows {
                if rd.kind != RowKind::Ebr {
                    continue;
                }
                let sn = match (row.cmp(&self.chip.row_clk), self.chip.kind) {
                    (std::cmp::Ordering::Less, _) => DirV::S,
                    (std::cmp::Ordering::Equal, ChipKind::Xp) => DirV::N,
                    (std::cmp::Ordering::Equal, ChipKind::Ecp) => DirV::S,
                    (std::cmp::Ordering::Greater, _) => DirV::N,
                    _ => unreachable!(),
                };
                self.chip.special_loc.insert(
                    SpecialLocKey::Pll(PllLoc::new_hv(edge, sn, 0)),
                    CellCoord::new(DieId::from_idx(0), self.chip.col_edge(edge), row),
                );
            }
        }
    }

    fn fill_ebr_machxo2(&mut self) {
        for &wn in self.nodes.values() {
            if self.naming.strings[wn.suffix].ends_with("_EBR") {
                let cell = self.chip.xlat_rc_wire(wn);
                if cell.row == self.chip.row_n() {
                    self.chip.columns[cell.col].io_n = IoGroupKind::Ebr;
                }
            }
        }
    }

    fn fill_io(&mut self, tiles: &[(&str, IoGroupKind)]) {
        let tiles = BTreeMap::from_iter(tiles.iter().copied());
        for tile in &self.grid.tiles {
            let Some(&kind) = tiles.get(&tile.kind.as_str()) else {
                continue;
            };
            if let Some(r) = tile.name.strip_prefix("PL") {
                let row = self.chip.xlat_row(r.parse().unwrap());
                self.chip.rows[row].io_w = kind;
            } else if let Some(r) = tile.name.strip_prefix("PR") {
                let row = self.chip.xlat_row(r.parse().unwrap());
                if tile.kind == "PIC_L_NOPIO" {
                    // ...... fucking vendors
                    self.chip.rows[row].io_w = kind;
                } else {
                    self.chip.rows[row].io_e = kind;
                }
            } else if let Some(c) = tile.name.strip_prefix("PB") {
                let col = self.chip.xlat_col(c.parse().unwrap());
                self.chip.columns[col].io_s = kind;
            } else if let Some(c) = tile.name.strip_prefix("PT") {
                let col = self.chip.xlat_col(c.parse().unwrap());
                if tile.kind == "PIC_B5KVIQ" {
                    // likewise.
                    self.chip.columns[col].io_s = kind;
                } else {
                    self.chip.columns[col].io_n = kind;
                }
            } else if let Some(rc) = tile
                .name
                .strip_prefix("EBR_R")
                .or_else(|| tile.name.strip_prefix("CIB_R"))
                .or_else(|| tile.name.strip_prefix("MIB_R"))
                .or_else(|| tile.name.strip_prefix("DUMMYEND_R"))
            {
                let (r, c) = rc.split_once('C').unwrap();
                let cell = self.chip.xlat_rc(r.parse().unwrap(), c.parse().unwrap());
                if cell.col == self.chip.col_w() && self.chip.kind != ChipKind::Crosslink {
                    self.chip.rows[cell.row].io_w = kind;
                } else if cell.col == self.chip.col_e() && self.chip.kind != ChipKind::Crosslink {
                    self.chip.rows[cell.row].io_e = kind;
                } else if cell.row == self.chip.row_s() {
                    self.chip.columns[cell.col].io_s = kind;
                } else if cell.row == self.chip.row_n()
                    || (cell.row == self.chip.row_n() - 12
                        && kind == IoGroupKind::Serdes
                        && self.chip.kind == ChipKind::Scm)
                {
                    self.chip.columns[cell.col].io_n = kind;
                } else {
                    panic!("umm weird IO tile {}", tile.name);
                }
            } else {
                panic!("umm weird IO tile {}", tile.name);
            }
        }
    }

    fn fill_io_scm(&mut self) {
        self.fill_io(&[
            ("PICL1B", IoGroupKind::Quad),
            ("PICL3", IoGroupKind::Dozen),
            ("PICR1B", IoGroupKind::Quad),
            ("PICR3", IoGroupKind::Dozen),
            ("PICT1B", IoGroupKind::Quad),
            ("PICT2B", IoGroupKind::Octal),
            ("PICT3", IoGroupKind::Dozen),
            ("PIC4B1B", IoGroupKind::Quad),
            ("PIC4B3", IoGroupKind::Dozen),
            ("PIC5B1B", IoGroupKind::Quad),
            ("PIC5B3", IoGroupKind::Dozen),
            ("LPCS6", IoGroupKind::Serdes),
            ("RPCS0", IoGroupKind::Serdes),
        ]);
        let mut add_cols_n = BTreeMap::new();
        let mut add_cols_s = BTreeMap::new();
        for (col, cd) in &mut self.chip.columns {
            let dx = match cd.io_n {
                IoGroupKind::Quad => 1,
                IoGroupKind::Octal => 2,
                IoGroupKind::Dozen => 3,
                _ => continue,
            };
            add_cols_n.insert(col - dx, cd.io_n);
            cd.io_n = IoGroupKind::None;
        }
        for (col, cd) in &mut self.chip.columns {
            let dx = match cd.io_s {
                IoGroupKind::Quad => 1,
                IoGroupKind::Octal => 2,
                IoGroupKind::Dozen => 3,
                _ => continue,
            };
            add_cols_s.insert(col - dx, cd.io_s);
            cd.io_s = IoGroupKind::None;
        }
        for (col, kind) in add_cols_n {
            self.chip.columns[col].io_n = kind;
        }
        for (col, kind) in add_cols_s {
            self.chip.columns[col].io_s = kind;
        }
        let mut add_rows_w = BTreeMap::new();
        let mut add_rows_e = BTreeMap::new();
        for (row, rd) in &mut self.chip.rows {
            let dy = match rd.io_w {
                IoGroupKind::Quad => 1,
                IoGroupKind::Octal => 2,
                IoGroupKind::Dozen => 3,
                _ => continue,
            };
            add_rows_w.insert(row - dy, rd.io_w);
            rd.io_w = IoGroupKind::None;
        }
        for (row, rd) in &mut self.chip.rows {
            let dy = match rd.io_e {
                IoGroupKind::Quad => 1,
                IoGroupKind::Octal => 2,
                IoGroupKind::Dozen => 3,
                _ => continue,
            };
            add_rows_e.insert(row - dy, rd.io_e);
            rd.io_e = IoGroupKind::None;
        }
        for (row, kind) in add_rows_w {
            self.chip.rows[row].io_w = kind;
        }
        for (row, kind) in add_rows_e {
            self.chip.rows[row].io_e = kind;
        }

        self.fill_io_banks_8();
        let mut serdes_bank = 9;
        for (col, cd) in &mut self.chip.columns {
            if cd.io_n == IoGroupKind::Serdes {
                if col < self.chip.col_clk {
                    cd.bank_n = Some(serdes_bank);
                    serdes_bank += 2;
                }
            } else if cd.io_n != IoGroupKind::None {
                cd.bank_n = Some(1);
            }
        }
        serdes_bank = 10;
        for (col, cd) in self.chip.columns.iter_mut().rev() {
            if cd.io_n == IoGroupKind::Serdes && col >= self.chip.col_clk {
                cd.bank_n = Some(serdes_bank);
                serdes_bank += 2;
            }
        }
    }

    fn fill_io_ecp(&mut self) {
        self.fill_io(&[
            ("PIC_L", IoGroupKind::Double),
            ("PIC_LDQS", IoGroupKind::DoubleDqs),
            ("PIC_R", IoGroupKind::Double),
            ("PIC_RDQS", IoGroupKind::DoubleDqs),
            ("PIC_RA", IoGroupKind::Double),
            ("PIC_RB", IoGroupKind::Double),
            ("PIC_T", IoGroupKind::Double),
            ("PIC_TDQS", IoGroupKind::DoubleDqs),
            ("PIC_B", IoGroupKind::Double),
            ("PIC_BDQS", IoGroupKind::DoubleDqs),
            ("PIC_BAB1", IoGroupKind::Double),
            ("PIC_BAB2", IoGroupKind::Double),
            ("PIC_BB1", IoGroupKind::Double),
            ("PIC_BB2", IoGroupKind::Double),
            ("PIC_BB3", IoGroupKind::Double),
            ("PIC_BDQSB", IoGroupKind::DoubleDqs),
        ]);
    }

    fn fill_io_xp(&mut self) {
        self.fill_io(&[
            ("PIC_L", IoGroupKind::Double),
            ("PIC_L_6K_CONFIG", IoGroupKind::DoubleA),
            ("PIC_L_A", IoGroupKind::DoubleA),
            ("PIC_L_A_20K", IoGroupKind::DoubleA),
            ("PIC_L_B", IoGroupKind::DoubleB),
            ("PIC_L_B_20K", IoGroupKind::DoubleB),
            ("PIC_LDQS", IoGroupKind::DoubleDqs),
            ("PIC_R", IoGroupKind::Double),
            ("PIC_R_3K_CONFIG", IoGroupKind::Double),
            ("PIC_R_A", IoGroupKind::DoubleA),
            ("PIC_R_A_20K", IoGroupKind::DoubleA),
            ("PIC_R_B", IoGroupKind::DoubleB),
            ("PIC_R_B_20K", IoGroupKind::DoubleB),
            ("PIC_RDQS", IoGroupKind::DoubleDqs),
            ("PIC_B_NO_IO", IoGroupKind::None),
            ("PIC_BL", IoGroupKind::Double),
            ("PIC_BL_A", IoGroupKind::DoubleA),
            ("PIC_BL_B", IoGroupKind::DoubleB),
            ("PIC_BLDQS", IoGroupKind::DoubleDqs),
            ("PIC_BR", IoGroupKind::Double),
            ("PIC_BR_A", IoGroupKind::DoubleA),
            ("PIC_BR_B", IoGroupKind::DoubleB),
            ("PIC_BRDQS", IoGroupKind::DoubleDqs),
            ("PIC_T_NO_IO", IoGroupKind::None),
            ("PIC_TL", IoGroupKind::Double),
            ("PIC_TL_A", IoGroupKind::DoubleA),
            ("PIC_TL_A_CFG", IoGroupKind::Double),
            ("PIC_TL_AB_CFG", IoGroupKind::Double),
            ("PIC_TL_A_ONLY_CFG", IoGroupKind::DoubleA),
            ("PIC_TL_B", IoGroupKind::DoubleB),
            ("PIC_TLDQS", IoGroupKind::DoubleDqs),
            ("PIC_TR", IoGroupKind::Double),
            ("PIC_TR_A", IoGroupKind::DoubleA),
            ("PIC_TR_A_CFG", IoGroupKind::Double),
            ("PIC_TR_AB_CFG", IoGroupKind::Double),
            ("PIC_TR_A_ONLY_CFG", IoGroupKind::DoubleA),
            ("PIC_TR_B", IoGroupKind::DoubleB),
            ("PIC_TR_B_CFG", IoGroupKind::Double),
            ("PIC_TRDQS", IoGroupKind::DoubleDqs),
        ]);
        if self.chip.rows.len() == 48 {
            let col_w1 = self.chip.col_w() + 1;
            let col_e1 = self.chip.col_e() - 1;
            self.chip.columns[col_w1].io_s = IoGroupKind::None;
            self.chip.columns[col_w1].io_n = IoGroupKind::None;
            self.chip.columns[col_e1].io_s = IoGroupKind::None;
            self.chip.columns[col_e1].io_n = IoGroupKind::None;
        }
    }

    fn fill_io_machxo(&mut self) {
        self.fill_io(&[
            ("PIC_L", IoGroupKind::Quad),
            ("PIC2_L", IoGroupKind::Double),
            ("PIC4_L", IoGroupKind::Quad),
            ("PIC_L_GSR", IoGroupKind::Quad),
            ("PIC_L_OSC", IoGroupKind::Quad),
            ("PIC_L_ISP", IoGroupKind::Quad),
            ("PIC2_L_GSR", IoGroupKind::Double),
            ("PIC2_L_OSC", IoGroupKind::Double),
            ("PIC2_L_ISP", IoGroupKind::Double),
            ("PIC2_L_EBR1K_0", IoGroupKind::Double),
            ("PIC4_L_EBR1K_1", IoGroupKind::Quad),
            ("PIC4_L_EBR1K_2", IoGroupKind::Quad),
            ("PIC4_L_EBR1K_3", IoGroupKind::Quad),
            ("PIC4_L_EBR1K_4", IoGroupKind::Quad),
            ("PIC4_L_EBR1K_5", IoGroupKind::Quad),
            ("PIC4_L_EBR1K_6", IoGroupKind::Quad),
            ("PIC2_L_EBR2K_1", IoGroupKind::Double),
            ("PIC2_L_EBR2K_2", IoGroupKind::Double),
            ("PIC2_L_EBR2K_3", IoGroupKind::Double),
            ("PIC4_L_EBR2K_4", IoGroupKind::Quad),
            ("PIC4_L_EBR2K_5", IoGroupKind::Quad),
            ("PIC4_L_EBR2K_6", IoGroupKind::Quad),
            ("PIC4_L_EBR2K_7", IoGroupKind::QuadReverse),
            ("PIC4_L_EBR2K_8", IoGroupKind::Quad),
            ("PIC4_L_EBR2K_9", IoGroupKind::Quad),
            ("PIC4_L_EBR2K_10", IoGroupKind::Quad),
            ("PIC4_L_EBR2K_11", IoGroupKind::Quad),
            ("PIC4_L_EBR2K_12", IoGroupKind::Quad),
            ("PIC4_L_EBR2K_13", IoGroupKind::Quad),
            ("PIC4_L_EBR2K_14", IoGroupKind::Quad),
            ("PIC4_L_EBR2K_15", IoGroupKind::Quad),
            ("PIC4_L_EBR2K_16", IoGroupKind::Quad),
            ("PIC4_L_EBR2K_17", IoGroupKind::Quad),
            ("PIC4_L_EBR2K_18", IoGroupKind::Quad),
            ("PIC2_L_EBR2K_19", IoGroupKind::Double),
            ("PIC2_L_PLL1K", IoGroupKind::Double),
            ("PIC_R", IoGroupKind::Quad),
            ("PIC2_R", IoGroupKind::Double),
            ("PIC2_R_LVDS", IoGroupKind::Double),
            ("PIC4_R", IoGroupKind::Quad),
            ("PIC4_B", IoGroupKind::Quad),
            ("PIC6_B", IoGroupKind::Hex),
            ("PIC4_T", IoGroupKind::Quad),
            ("PIC6_T", IoGroupKind::Hex),
        ]);
        if self.chip.rows.len() == 21 {
            self.chip.columns[ColId::from_idx(3)].io_n = IoGroupKind::HexReverse;
            self.chip.columns[ColId::from_idx(5)].io_n = IoGroupKind::HexReverse;
            self.chip.columns[ColId::from_idx(9)].io_s = IoGroupKind::HexReverse;
        }
    }

    fn fill_io_ecp2(&mut self) {
        self.fill_io(&[
            ("PIC_L", IoGroupKind::Double),
            ("PIC_LLPCLK", IoGroupKind::Double),
            ("PIC_LUPCLK", IoGroupKind::Double),
            ("PIC_LDQS", IoGroupKind::DoubleDqs),
            ("PIC_LDQSM2", IoGroupKind::Double),
            ("PIC_LDQSM3", IoGroupKind::Double),
            ("PIC_R", IoGroupKind::Double),
            ("PIC_RLPCLK", IoGroupKind::Double),
            ("PIC_RUPCLK", IoGroupKind::Double),
            ("PIC_RDQS", IoGroupKind::DoubleDqs),
            ("PIC_RDQSM2", IoGroupKind::Double),
            ("PIC_RDQSM3", IoGroupKind::Double),
            ("PIC_RCPU", IoGroupKind::Double),
            ("PIC_B", IoGroupKind::Double),
            ("PIC_BSPL", IoGroupKind::Double),
            ("PIC_BSPR", IoGroupKind::Double),
            ("PIC_BDQS", IoGroupKind::DoubleDqs),
            ("PIC_BLPCLK", IoGroupKind::Double),
            ("PIC_BRPCLK", IoGroupKind::Double),
            ("PIC_T", IoGroupKind::Double),
            ("PIC_TSPL", IoGroupKind::Double),
            ("PIC_TSPR", IoGroupKind::Double),
            ("PIC_TLPCLK", IoGroupKind::Double),
            ("PIC_TRPCLK", IoGroupKind::Double),
        ]);
    }

    fn fill_io_xp2(&mut self) {
        self.fill_io(&[
            ("PIC_L", IoGroupKind::Double),
            ("PIC_L_NOPIO", IoGroupKind::DoubleDummy),
            ("PIC_LDQS", IoGroupKind::DoubleDqs),
            ("PIC_LDQSM2", IoGroupKind::Double),
            ("PIC_LDQSM3", IoGroupKind::Double),
            ("PIC_R", IoGroupKind::Double),
            ("PIC_R_NOPIO", IoGroupKind::DoubleDummy),
            ("PIC_RDQS", IoGroupKind::DoubleDqs),
            ("PIC_RDQSM2", IoGroupKind::Double),
            ("PIC_RDQSM3", IoGroupKind::Double),
            ("PIC_B", IoGroupKind::Double),
            ("PIC_BSPL", IoGroupKind::Double),
            ("PIC_BSPR", IoGroupKind::Double),
            ("PIC_B5KVIQ", IoGroupKind::Double),
            ("PIC_BDQS", IoGroupKind::DoubleDqs),
            ("PIC_BLPCLK", IoGroupKind::Double),
            ("PIC_BRPCLK", IoGroupKind::Double),
            ("PIC_B_NOPIO", IoGroupKind::DoubleDummy),
            ("PIC_T", IoGroupKind::Double),
            ("PIC_TSPL", IoGroupKind::Double),
            ("PIC_TSPR", IoGroupKind::Double),
            ("PIC_T5KVIQ", IoGroupKind::Double),
            ("PIC_TDQS", IoGroupKind::DoubleDqs),
            ("PIC_TLPCLK", IoGroupKind::Double),
            ("PIC_TRPCLK", IoGroupKind::Double),
            ("PIC_T_NOPIO", IoGroupKind::DoubleDummy),
        ]);
    }

    fn fill_io_ecp3(&mut self) {
        self.fill_io(&[
            // west
            ("PIC_L2", IoGroupKind::Quad),
            ("PIC_L2A", IoGroupKind::Quad),
            ("PIC_L2B", IoGroupKind::Quad),
            ("PIC_L2E", IoGroupKind::Quad),
            ("PIC_LDQS2A", IoGroupKind::QuadDqs),
            ("PIC_LDQS2AS", IoGroupKind::QuadDqsDummy),
            ("PIC_LDQS2B", IoGroupKind::QuadDqs),
            ("PIC_LDQS2C", IoGroupKind::QuadDqs),
            ("PIC_LDQS2D", IoGroupKind::QuadDqsDummy),
            ("PIC_LDQS2E", IoGroupKind::QuadDqsDummy),
            ("PIC_LDQS2F", IoGroupKind::QuadDqs),
            ("PICATEMB_L2EVREF", IoGroupKind::Quad),
            ("PICATEMB_L2EPT", IoGroupKind::Quad),
            ("PICATVREFL_L2EPT", IoGroupKind::Quad),
            ("LLC2", IoGroupKind::Quad),
            ("PICATPLL_L2E", IoGroupKind::Quad),
            ("PICATEMB_L2APT", IoGroupKind::Quad),
            ("PICATEMB_L2A", IoGroupKind::Quad),
            ("PICATVREFU_L2APT", IoGroupKind::Quad),
            ("PICATVREFL_L2APT", IoGroupKind::Quad),
            ("LLC0", IoGroupKind::Quad),
            ("PICATPLL_L2A", IoGroupKind::Quad),
            ("PICATPLL_L2APT", IoGroupKind::Quad),
            ("PICATEMB_L2BPT", IoGroupKind::Quad),
            ("PICATEMB_L2B", IoGroupKind::Quad),
            ("PICATVREFU_L2BPT", IoGroupKind::Quad),
            ("PICATVREFL_L2BPT", IoGroupKind::Quad),
            ("PICATDSP_L2B", IoGroupKind::Quad),
            ("LLC1", IoGroupKind::Quad),
            ("PICATPLL_L2B", IoGroupKind::Quad),
            ("PICATPLL_L2BPT", IoGroupKind::Quad),
            // east
            ("PIC_R2", IoGroupKind::Quad),
            ("PIC_RCPU2", IoGroupKind::Quad),
            ("PIC_RCPU2C", IoGroupKind::Quad),
            ("PICATEMB_RCPU2VREF", IoGroupKind::Quad),
            ("PICATEMB_RCPU2PT", IoGroupKind::Quad),
            ("PICATEMB_RCPU2", IoGroupKind::Quad),
            ("PIC_RDQS2", IoGroupKind::QuadDqs),
            ("PIC_R3DQS2", IoGroupKind::QuadDqs),
            ("PIC_RDQS2C", IoGroupKind::QuadDqs),
            ("PICATVREFL_R2PT", IoGroupKind::Quad),
            ("PICATVREFU_R2PT", IoGroupKind::Quad),
            ("PICATDSP_R2", IoGroupKind::Quad),
            ("PICATEMB_R2", IoGroupKind::Quad),
            ("LRC", IoGroupKind::Quad),
            ("PICATPLL_R2", IoGroupKind::Quad),
            ("PICATPLL_R2PT", IoGroupKind::Quad),
            // south
            ("PIC_B0", IoGroupKind::Quad),
            // north
            ("PIC_T0", IoGroupKind::Quad),
            ("PIC_TSPR0", IoGroupKind::Quad),
            ("PIC_TVIQSPR0", IoGroupKind::Quad),
            ("PIC_TCPU0", IoGroupKind::Quad),
            ("PIC_TDQS0", IoGroupKind::QuadDqs),
        ]);
    }

    fn fill_io_machxo2(&mut self) {
        self.fill_io(&[
            ("PIC_L0", IoGroupKind::Quad),
            ("PIC_L0_I3C", IoGroupKind::QuadI3c),
            ("PIC_LS0", IoGroupKind::Double),
            ("PIC_L0_VREF3", IoGroupKind::Quad),
            ("PIC_L0_VREF4", IoGroupKind::Quad),
            ("PIC_L0_VREF5", IoGroupKind::Quad),
            ("PIC_L1", IoGroupKind::Quad),
            ("PIC_L1_I3C", IoGroupKind::QuadI3c),
            ("PIC_L1_VREF3", IoGroupKind::Quad),
            ("PIC_L1_VREF4", IoGroupKind::Quad),
            ("PIC_L1_VREF5", IoGroupKind::Quad),
            ("PIC_L2", IoGroupKind::Quad),
            ("PIC_L2_VREF4", IoGroupKind::Quad),
            ("PIC_L2_VREF5", IoGroupKind::Quad),
            ("PIC_L3", IoGroupKind::Quad),
            ("PIC_L3_VREF4", IoGroupKind::Quad),
            ("PIC_L3_VREF5", IoGroupKind::Quad),
            ("LLC0PIC", IoGroupKind::Double),
            ("LLC0PIC_VREF3", IoGroupKind::Quad),
            ("LLC0PIC_I3C_VREF3", IoGroupKind::QuadI3c),
            ("LLC1PIC", IoGroupKind::Quad),
            ("LLC3PIC_VREF3", IoGroupKind::Quad),
            ("ULC3PIC", IoGroupKind::Quad),
            ("PIC_R0", IoGroupKind::Quad),
            ("PIC_R0_256", IoGroupKind::Quad),
            ("PIC_RS0", IoGroupKind::Double),
            ("PIC_RS0_256", IoGroupKind::Double),
            ("PIC_R1", IoGroupKind::Quad),
            ("PIC_R1_640", IoGroupKind::Quad),
            ("LRC0PIC", IoGroupKind::Quad),
            ("LRC1PIC1", IoGroupKind::Quad),
            ("LRC1PIC2", IoGroupKind::Quad),
            ("URC1PIC", IoGroupKind::Quad),
            ("PIC_B0", IoGroupKind::Quad),
            ("PIC_B0_256", IoGroupKind::Quad),
            ("PIC_BS0_256", IoGroupKind::Double),
            ("PIC_T0", IoGroupKind::Quad),
            ("PIC_T0_256", IoGroupKind::Quad),
            ("PIC_TS0", IoGroupKind::Double),
        ]);
        if self.chip.rows.len() == 11 {
            self.chip.columns[ColId::from_idx(8)].io_s = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(10)].io_s = IoGroupKind::QuadReverse;
        }
        if self.chip.rows.len() == 14 {
            self.chip.columns[ColId::from_idx(17)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(19)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(23)].io_n = IoGroupKind::QuadReverse;
        }
        if self.chip.rows.len() == 21 {
            self.chip.columns[ColId::from_idx(5)].io_s = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(11)].io_s = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(17)].io_s = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(25)].io_s = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(11)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(18)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(19)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(21)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(22)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(25)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(27)].io_n = IoGroupKind::QuadReverse;
        }
        if self.chip.rows.len() == 26 {
            self.chip.columns[ColId::from_idx(6)].io_s = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(9)].io_s = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(14)].io_s = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(20)].io_s = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(27)].io_s = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(33)].io_s = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(11)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(15)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(20)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(21)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(25)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(26)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(31)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(33)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(35)].io_n = IoGroupKind::QuadReverse;
        }
        if self.chip.rows.len() == 30 {
            self.chip.columns[ColId::from_idx(7)].io_s = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(12)].io_s = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(20)].io_s = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(26)].io_s = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(31)].io_s = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(40)].io_s = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(11)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(20)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(23)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(26)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(28)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(30)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(38)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(41)].io_n = IoGroupKind::QuadReverse;
            self.chip.columns[ColId::from_idx(43)].io_n = IoGroupKind::QuadReverse;
        }
    }

    fn fill_io_ecp4(&mut self) {
        self.fill_io(&[
            ("MIB_L_PIC0A", IoGroupKind::Quad),
            ("MIB_L_PIC0B", IoGroupKind::Quad),
            ("MIB_L_PIC0B_BREF", IoGroupKind::Quad),
            ("MIB_L_PIC0B_HIQ_U", IoGroupKind::Quad),
            ("MIB_L_PIC0A_DQS1", IoGroupKind::Quad),
            ("MIB_L_PIC0B_DQS0", IoGroupKind::Quad),
            ("MIB_L_PIC0A_DQS2", IoGroupKind::Quad),
            ("MIB_L_PIC0B_DQS3", IoGroupKind::Quad),
            ("MIB_L_PIC0A_DQS3", IoGroupKind::Quad),
            ("MIB_LS_PIC0B_E", IoGroupKind::Quad),
            ("MIB_LS_PIC0B_E_DQS2", IoGroupKind::Quad),
            ("MIB_LS_PIC0B_D", IoGroupKind::Quad),
            ("MIB_LS_PIC0B_D_DQS0", IoGroupKind::Quad),
            ("MIB_LS_PIC0A_D", IoGroupKind::Quad),
            ("MIB_LS_PIC0A_E", IoGroupKind::Quad),
            ("MIB_LS_PIC0A_E_DQS1", IoGroupKind::Quad),
            ("MIB_LS_PIC0A_E_DQS2", IoGroupKind::Quad),
            ("MIB_R_PIC0A", IoGroupKind::Quad),
            ("MIB_R_PIC0B", IoGroupKind::Quad),
            ("MIB_R_PIC0B_BREF", IoGroupKind::Quad),
            ("MIB_R_PIC0B_HIQ_U", IoGroupKind::Quad),
            ("MIB_R_PIC0A_DQS1", IoGroupKind::Quad),
            ("MIB_R_PIC0B_DQS0", IoGroupKind::Quad),
            ("MIB_R_PIC0A_DQS2", IoGroupKind::Quad),
            ("MIB_R_PIC0B_DQS3", IoGroupKind::Quad),
            ("MIB_R_PIC0A_DQS3", IoGroupKind::Quad),
            ("MIB_RS_PIC0B_E", IoGroupKind::Quad),
            ("MIB_RS_PIC0B_E_DQS2", IoGroupKind::Quad),
            ("MIB_RS_PIC0B_D", IoGroupKind::Quad),
            ("MIB_RS_PIC0B_D_DQS0", IoGroupKind::Quad),
            ("MIB_RS_PIC0A_D", IoGroupKind::Quad),
            ("MIB_RS_PIC0A_E", IoGroupKind::Quad),
            ("MIB_RS_PIC0A_E_DQS1", IoGroupKind::Quad),
            ("MIB_RS_PIC0A_E_DQS2", IoGroupKind::Quad),
            ("MIB_T_PIC0A", IoGroupKind::Quad),
            ("MIB_T_PIC0B", IoGroupKind::Quad),
            ("MIB_T_PIC0B_DLLDEL_B1", IoGroupKind::Quad),
            ("MIB_T_PIC0A_DLLDEL_B0", IoGroupKind::Quad),
            ("MIB_T_PIC0B_DLLDEL_B1_BIG", IoGroupKind::Quad),
            ("MIB_T_PIC0A_DLLDEL_B1_BIG", IoGroupKind::Quad),
            ("MIB_T_PIC0A_DQS1", IoGroupKind::Quad),
            ("MIB_T_PIC0B_DQS0", IoGroupKind::Quad),
        ]);

        let mut skip_w = 0;
        let mut skip_e = 0;
        for (row, rd) in &mut self.chip.rows {
            if rd.io_w != IoGroupKind::None {
                if skip_w != 0 {
                    rd.io_w = IoGroupKind::None;
                    skip_w -= 1;
                } else {
                    skip_w = 3;
                }
            }
            if rd.io_e != IoGroupKind::None {
                if skip_e != 0 {
                    rd.io_e = IoGroupKind::None;
                    skip_e -= 1;
                } else {
                    skip_e = 3;
                }
            }
            if rd.kind == RowKind::Dsp {
                assert!(skip_w >= 2);
                assert!(skip_e >= 2);
                skip_w -= 2;
                skip_e -= 2;
            }
            if row < self.chip.row_clk {
                if rd.io_w != IoGroupKind::None || rd.kind == RowKind::Ebr {
                    rd.bank_w = Some(6);
                }
                if rd.io_e != IoGroupKind::None || rd.kind == RowKind::Ebr {
                    rd.bank_e = Some(5);
                }
            } else {
                if rd.io_w != IoGroupKind::None || rd.kind == RowKind::Ebr {
                    rd.bank_w = Some(7);
                }
                if rd.io_e != IoGroupKind::None || rd.kind == RowKind::Ebr {
                    rd.bank_e = Some(4);
                }
            }
        }
        assert_eq!(skip_w, 0);
        assert_eq!(skip_e, 0);

        let mut bank = 2;
        for col in (self.chip.col_clk + 1usize).range(self.chip.col_e()) {
            if self.chip.columns[col].io_n == IoGroupKind::None {
                bank = 3;
            } else {
                self.chip.columns[col].bank_n = Some(bank);
            }
        }
        let mut bank = 1;
        for col in self.chip.col_w().range(self.chip.col_clk - 1usize).rev() {
            if self.chip.columns[col].io_n == IoGroupKind::None {
                bank = 0;
            } else {
                self.chip.columns[col].bank_n = Some(bank);
            }
        }

        let mut skip = 0;
        for (_col, cd) in &mut self.chip.columns {
            if cd.io_n != IoGroupKind::None {
                if skip != 0 {
                    cd.io_n = IoGroupKind::None;
                    cd.bank_n = None;
                    skip -= 1;
                } else {
                    skip = 3;
                }
            }
        }
        assert_eq!(skip, 0);
    }

    fn fill_dqs_ecp4(&mut self) {
        let dqsi = self.naming.strings.get("JDQSI_DQS").unwrap();
        let paddi = self.naming.strings.get("JPADDI_PIO").unwrap();
        let paddiea = self.naming.strings.get("JPADDIEA_PIO").unwrap();
        for &(wf, wt) in self.grid.pips.keys() {
            let wfn = self.nodes[wf];
            let wtn = self.nodes[wt];
            if wtn.suffix == dqsi {
                let cell = self.chip.xlat_rc_wire(wtn);
                let is_ea = if wfn.suffix == paddi {
                    false
                } else if wfn.suffix == paddiea {
                    true
                } else {
                    unreachable!()
                };
                if cell.row == self.chip.row_n() {
                    assert!(!is_ea);
                    assert_eq!(self.chip.columns[cell.col].io_n, IoGroupKind::Quad);
                    self.chip.columns[cell.col].io_n = IoGroupKind::QuadDqs;
                } else {
                    let edge = if cell.col == self.chip.col_w() {
                        DirH::W
                    } else if cell.col == self.chip.col_e() {
                        DirH::E
                    } else {
                        unreachable!()
                    };
                    if is_ea && self.chip.rows[cell.row].kind == RowKind::Ebr {
                        let io_kind = match edge {
                            DirH::W => &mut self.chip.rows[cell.row].io_w,
                            DirH::E => &mut self.chip.rows[cell.row].io_e,
                        };
                        *io_kind = match *io_kind {
                            IoGroupKind::None => IoGroupKind::EbrDqs,
                            IoGroupKind::Quad => IoGroupKind::QuadEbrDqs,
                            _ => unreachable!(),
                        };
                    } else {
                        let mut row = cell.row;
                        loop {
                            let io_kind = match edge {
                                DirH::W => &mut self.chip.rows[row].io_w,
                                DirH::E => &mut self.chip.rows[row].io_e,
                            };
                            if *io_kind == IoGroupKind::Quad {
                                *io_kind = IoGroupKind::QuadDqs;
                                break;
                            }
                            row -= 1;
                        }
                    }
                }
            }
        }
    }

    fn fill_io_ecp5(&mut self) {
        self.fill_io(&[
            ("PICL2", IoGroupKind::Quad),
            ("PICL2_DQS1", IoGroupKind::QuadDqs),
            ("MIB_CIB_LR", IoGroupKind::Quad),
            ("MIB_CIB_LRC", IoGroupKind::Quad),
            ("PICR2", IoGroupKind::Quad),
            ("PICR2_DQS1", IoGroupKind::QuadDqs),
            ("MIB_CIB_LR_A", IoGroupKind::Quad),
            ("MIB_CIB_LRC_A", IoGroupKind::Quad),
            ("PICB0", IoGroupKind::Double),
            ("EFB0_PICB0", IoGroupKind::Double),
            ("EFB2_PICB0", IoGroupKind::Double),
            ("SPICB0", IoGroupKind::Single),
            ("PICT0", IoGroupKind::Double),
        ]);
        let dqs_w = BTreeSet::from_iter(
            self.chip
                .rows
                .ids()
                .filter(|&row| self.chip.rows[row].io_w == IoGroupKind::QuadDqs)
                .map(|row| row - 3),
        );
        let dqs_e = BTreeSet::from_iter(
            self.chip
                .rows
                .ids()
                .filter(|&row| self.chip.rows[row].io_e == IoGroupKind::QuadDqs)
                .map(|row| row - 3),
        );
        for (row, rd) in &mut self.chip.rows {
            if rd.io_w != IoGroupKind::None {
                rd.bank_w = Some(if row < self.chip.row_clk { 6 } else { 7 });
                rd.io_w = if dqs_w.contains(&row) {
                    IoGroupKind::QuadDqs
                } else {
                    IoGroupKind::Quad
                };
            }
            if rd.io_e != IoGroupKind::None {
                rd.bank_e = Some(if row < self.chip.row_clk { 3 } else { 2 });
                rd.io_e = if dqs_e.contains(&row) {
                    IoGroupKind::QuadDqs
                } else {
                    IoGroupKind::Quad
                };
            }
        }
        for (col, cd) in &mut self.chip.columns {
            if cd.io_s != IoGroupKind::None {
                cd.bank_s = Some(if col < self.chip.col_clk { 8 } else { 4 });
            }
            if cd.io_n != IoGroupKind::None {
                cd.bank_n = Some(if col < self.chip.col_clk { 0 } else { 1 });
            }
        }
    }

    fn fill_io_crosslink(&mut self) {
        self.fill_io(&[("LVDS_0", IoGroupKind::Quad), ("GPIO", IoGroupKind::Single)]);
        for (col, cd) in &mut self.chip.columns {
            if cd.io_s != IoGroupKind::None {
                let bank = if col < self.chip.col_clk {
                    2
                } else if cd.io_s == IoGroupKind::Quad {
                    1
                } else {
                    0
                };
                cd.bank_s = Some(bank);
            }
        }
    }

    fn fill_bc_ecp5(&mut self) {
        for &(wf, wt) in self.grid.pips.keys() {
            let wfn = self.nodes[wf];
            let wtn = self.nodes[wt];
            if let Some(bank) =
                self.naming.strings[wtn.suffix].strip_prefix("JPVT_SRC_IN0_BREFTEST")
            {
                let bank = bank.parse().unwrap();
                let cell = self.chip.xlat_rc_wire(wfn);
                self.chip.special_loc.insert(SpecialLocKey::Bc(bank), cell);
            }
        }
    }

    fn fill_bc_crosslink(&mut self) {
        for &(wf, wt) in self.grid.pips.keys() {
            let wfn = self.nodes[wf];
            let wtn = self.nodes[wt];
            if self.naming.strings[wtn.suffix] == "JINRDENI_BCINRD" {
                let cell = self.chip.xlat_rc_wire(wfn);
                let bank = if cell.col < self.chip.col_clk { 2 } else { 1 };
                self.chip.special_loc.insert(SpecialLocKey::Bc(bank), cell);
            }
        }
    }

    fn fill_ddrdll_ecp5(&mut self) {
        let ebr_dsp_rows = Vec::from_iter(
            self.chip
                .rows
                .iter()
                .filter(|&(_, rd)| matches!(rd.kind, RowKind::Ebr | RowKind::Dsp))
                .map(|(row, _)| row),
        );
        for hv in DirHV::DIRS {
            let row = match hv.v {
                DirV::S => ebr_dsp_rows[0],
                DirV::N => *ebr_dsp_rows.last().unwrap(),
            };
            let col = match hv.h {
                DirH::W => self.chip.col_w() + 1,
                DirH::E => self.chip.col_e() - 1,
            };
            let cell = CellCoord::new(DieId::from_idx(0), col, row);
            self.chip
                .special_loc
                .insert(SpecialLocKey::DdrDll(hv), cell);
        }
    }

    fn fill_io_banks_8(&mut self) {
        for (row, rd) in &mut self.chip.rows {
            if row < self.chip.row_clk {
                if rd.io_w != IoGroupKind::None {
                    rd.bank_w = Some(6);
                }
                if rd.io_e != IoGroupKind::None {
                    rd.bank_e = Some(3);
                }
            } else {
                if rd.io_w != IoGroupKind::None {
                    rd.bank_w = Some(7);
                }
                if rd.io_e != IoGroupKind::None {
                    rd.bank_e = Some(2);
                }
            }
        }
        for (col, cd) in &mut self.chip.columns {
            if col < self.chip.col_clk {
                if cd.io_s != IoGroupKind::None {
                    cd.bank_s = Some(5);
                }
                if cd.io_n != IoGroupKind::None {
                    cd.bank_n = Some(0);
                }
            } else {
                if cd.io_s != IoGroupKind::None {
                    cd.bank_s = Some(4);
                }
                if cd.io_n != IoGroupKind::None {
                    cd.bank_n = Some(1);
                }
            }
        }
    }

    fn fill_io_banks_machxo(&mut self) {
        let num_rows = self.chip.rows.len();
        for (row, rd) in &mut self.chip.rows {
            if rd.io_w == IoGroupKind::None {
                continue;
            }
            let (bank_w, bank_e) = match num_rows {
                10 => (1, 0),
                12 => (3, 1),
                17 => {
                    if row < self.chip.row_clk {
                        (6, 3)
                    } else {
                        (7, 2)
                    }
                }
                21 => {
                    if row < self.chip.row_clk + 1 {
                        (6, 3)
                    } else {
                        (7, 2)
                    }
                }
                _ => unreachable!(),
            };
            rd.bank_w = Some(bank_w);
            rd.bank_e = Some(bank_e);
        }
        for (col, cd) in &mut self.chip.columns {
            if cd.io_s == IoGroupKind::None {
                continue;
            }
            let (bank_s, bank_n) = match num_rows {
                10 => (1, 0),
                12 => (2, 0),
                17 | 21 => {
                    if col < self.chip.col_clk {
                        (5, 0)
                    } else {
                        (4, 1)
                    }
                }
                _ => unreachable!(),
            };
            cd.bank_s = Some(bank_s);
            cd.bank_n = Some(bank_n);
        }
    }

    fn fill_io_banks_ecp2(&mut self) {
        self.fill_io_banks_8();
        for (row, rd) in &mut self.chip.rows {
            if row.to_idx() <= 8 && rd.bank_e.is_some() {
                rd.bank_e = Some(8);
            }
        }
    }

    fn fill_io_banks_ecp3(&mut self) {
        let row_cfg = self.chip.special_loc[&SpecialLocKey::Config].row;
        for (row, rd) in &mut self.chip.rows {
            if row < self.chip.row_clk {
                if rd.io_w != IoGroupKind::None {
                    rd.bank_w = Some(6);
                }
                if rd.io_e != IoGroupKind::None {
                    rd.bank_e = Some(3);
                }
            } else {
                if rd.io_w != IoGroupKind::None {
                    rd.bank_w = Some(7);
                }
                if rd.io_e != IoGroupKind::None {
                    if row < row_cfg {
                        rd.bank_e = Some(2);
                    } else {
                        rd.bank_e = Some(8);
                    }
                }
            }
        }
        let col_e = self.chip.col_e();
        for (col, cd) in &mut self.chip.columns {
            if col < self.chip.col_clk {
                if cd.io_s != IoGroupKind::None {
                    cd.bank_s = Some(6);
                }
                if cd.io_n != IoGroupKind::None {
                    cd.bank_n = Some(0);
                }
            } else {
                if cd.io_s != IoGroupKind::None {
                    cd.bank_s = Some(3);
                }
                if cd.io_n != IoGroupKind::None {
                    if col < col_e - 6 {
                        cd.bank_n = Some(1);
                    } else {
                        cd.bank_n = Some(8);
                    }
                }
            }
        }
    }

    fn fill_io_banks_machxo2(&mut self) {
        let (r4, r5) = match self.chip.rows.len() {
            6 => (6, 6),
            7 => (7, 7),
            11 => (11, 11),
            14 => (4, 9),
            21 => (7, 13),
            26 => (9, 17),
            30 => (11, 18),
            _ => unreachable!(),
        };
        let row4 = RowId::from_idx(r4);
        let row5 = RowId::from_idx(r5);
        for (row, rd) in &mut self.chip.rows {
            if rd.io_w != IoGroupKind::None {
                if row < row4 {
                    rd.bank_w = Some(3);
                } else if row < row5 {
                    rd.bank_w = Some(4);
                } else {
                    rd.bank_w = Some(5);
                }
            }
            if rd.io_e != IoGroupKind::None {
                rd.bank_e = Some(1);
            }
        }
        for (_, cd) in &mut self.chip.columns {
            if cd.io_s != IoGroupKind::None {
                cd.bank_s = Some(2);
            }
            if cd.io_n != IoGroupKind::None {
                cd.bank_n = Some(0);
            }
        }
    }

    fn fill_bc_machxo2(&mut self) {
        let mut xlat = BTreeMap::new();
        let suffix = self.naming.strings.get("JPGENI_BCPG").unwrap();
        for &(wf, wt) in self.grid.pips.keys() {
            let wfn = self.nodes[wf];
            let wtn = self.nodes[wt];
            if wtn.suffix == suffix {
                let cell_to = self.chip.xlat_rc_wire(wtn);
                let cell_from = self.chip.xlat_rc_wire(wfn);
                xlat.insert(cell_to, cell_from);
            }
        }
        let has_bank4 = self.chip.rows.values().any(|rd| rd.bank_w == Some(4));
        let bcs = if has_bank4 {
            vec![
                (0, self.chip.col_clk - 1, self.chip.row_n()),
                (1, self.chip.col_e(), self.chip.row_clk),
                (2, self.chip.col_clk - 1, self.chip.row_s()),
                (3, self.chip.col_w(), self.chip.row_s()),
                (4, self.chip.col_w(), self.chip.row_clk),
                (5, self.chip.col_w(), self.chip.row_n()),
            ]
        } else {
            vec![
                (0, self.chip.col_clk - 1, self.chip.row_n()),
                (1, self.chip.col_e(), self.chip.row_clk),
                (2, self.chip.col_clk - 1, self.chip.row_s()),
                (3, self.chip.col_w(), self.chip.row_clk),
            ]
        };
        for (bank, col, row) in bcs {
            self.chip.special_loc.insert(
                SpecialLocKey::Bc(bank),
                xlat[&CellCoord::new(DieId::from_idx(0), col, row)],
            );
        }
    }

    fn fill_bc_ecp4(&mut self) {
        let has_bank0 = self.chip.columns.values().any(|cd| cd.bank_n == Some(0));
        for &(wf, wt) in self.grid.pips.keys() {
            let wfn = self.nodes[wf];
            let wtn = self.nodes[wt];
            let wtns = self.naming.strings[wtn.suffix].as_str();
            if wtns.starts_with("JPGENI_BCPG") {
                let cell_to = self.chip.xlat_rc_wire(wtn);
                let cell_from = self.chip.xlat_rc_wire(wfn);
                let bank = match wtns {
                    "JPGENI_BCPG_L" => {
                        if cell_to.row == self.chip.row_s() {
                            6
                        } else {
                            7
                        }
                    }
                    "JPGENI_BCPG_R" => {
                        if cell_to.row == self.chip.row_s() {
                            5
                        } else {
                            4
                        }
                    }
                    "JPGENI_BCPG_T" => {
                        if cell_to.col == self.chip.col_w() {
                            if has_bank0 { 0 } else { 1 }
                        } else {
                            if has_bank0 { 3 } else { 2 }
                        }
                    }
                    "JPGENI_BCPG_TL" => 1,
                    "JPGENI_BCPG_TR" => 2,
                    _ => unreachable!(),
                };
                self.chip
                    .special_loc
                    .insert(SpecialLocKey::Bc(bank), cell_from);
            }
        }
    }

    fn fill_dqsdll_machxo2(&mut self) {
        let Some(suffix) = self.naming.strings.get("JLOCK_DQSDLL") else {
            return;
        };

        for &wn in self.nodes.values() {
            if wn.suffix == suffix {
                let cell = self.chip.xlat_rc_wire(wn);
                let key = if cell.row == self.chip.row_s() {
                    SpecialLocKey::DqsDll(Dir::S)
                } else {
                    SpecialLocKey::DqsDll(Dir::N)
                };
                self.chip.special_loc.insert(key, cell);
            }
        }
    }

    fn gather_special_io_scm(&mut self) -> BTreeMap<WireName, (EdgeIoCoord, PinDir)> {
        let jinddcka = self.naming.strings.get("JINDDCKA").unwrap();
        let jinddckb = self.naming.strings.get("JINDDCKB").unwrap();
        let jinddckc = self.naming.strings.get("JINDDCKC").unwrap();
        let jinddckd = self.naming.strings.get("JINDDCKD").unwrap();
        let jpaddoa = self.naming.strings.get("JPADDOA_PIO").unwrap();
        let jpaddob = self.naming.strings.get("JPADDOB_PIO").unwrap();
        let jpaddoc = self.naming.strings.get("JPADDOC_PIO").unwrap();
        let jpaddod = self.naming.strings.get("JPADDOD_PIO").unwrap();
        let mut pad_nodes = HashMap::new();
        for (node, &wn) in self.nodes {
            let mut cell = self.chip.xlat_rc_wire(wn);
            let (idx, dir) = if wn.suffix == jinddcka {
                (0, PinDir::Input)
            } else if wn.suffix == jinddckb {
                (1, PinDir::Input)
            } else if wn.suffix == jinddckc {
                (2, PinDir::Input)
            } else if wn.suffix == jinddckd {
                (3, PinDir::Input)
            } else if wn.suffix == jpaddoa {
                (0, PinDir::Output)
            } else if wn.suffix == jpaddob {
                (1, PinDir::Output)
            } else if wn.suffix == jpaddoc {
                (2, PinDir::Output)
            } else if wn.suffix == jpaddod {
                (3, PinDir::Output)
            } else {
                continue;
            };
            let bel = if cell.col == self.chip.col_w() || cell.col == self.chip.col_e() {
                let mut ridx = 0;
                let edge = if cell.col == self.chip.col_w() {
                    DirH::W
                } else {
                    DirH::E
                };
                loop {
                    let kind = match edge {
                        DirH::W => self.chip.rows[cell.row].io_w,
                        DirH::E => self.chip.rows[cell.row].io_e,
                    };
                    match kind {
                        IoGroupKind::None => {
                            ridx += 1;
                            cell.row -= 1;
                        }
                        IoGroupKind::Quad => break bels::IO[idx],
                        IoGroupKind::Dozen => break bels::IO[(2 - ridx) * 4 + idx],
                        _ => unreachable!(),
                    }
                }
            } else {
                let mut idx = idx;
                let edge = if cell.row == self.chip.row_s() {
                    DirV::S
                } else {
                    DirV::N
                };
                loop {
                    let kind = match edge {
                        DirV::S => self.chip.columns[cell.col].io_s,
                        DirV::N => self.chip.columns[cell.col].io_n,
                    };
                    if kind == IoGroupKind::None {
                        idx += 4;
                        cell.col -= 1;
                    } else {
                        break;
                    }
                }
                bels::IO[idx]
            };
            let io = self.chip.get_io_crd(cell.bel(bel));
            pad_nodes.insert(node, (io, dir));
        }
        let mut result = BTreeMap::new();
        for &(wf, wt) in self.grid.pips.keys() {
            if let Some(&io) = pad_nodes.get(&wf) {
                let wn = self.nodes[wt];
                let suffix = self.naming.strings[wn.suffix].as_str();
                if matches!(suffix, |"JQ0"| "JQ1"
                    | "JQ2"
                    | "JQ3"
                    | "JF1"
                    | "JF4"
                    | "JF6"
                    | "JFMPIC_4"
                    | "JFMPIC_6"
                    | "JFMPIC_13")
                {
                    continue;
                }
                result.insert(wn, io);
            } else if let Some(&io) = pad_nodes.get(&wt) {
                let wn = self.nodes[wf];
                let suffix = self.naming.strings[wn.suffix].as_str();
                if matches!(
                    suffix,
                    "JA0"
                        | "JA1"
                        | "JA2"
                        | "JA3"
                        | "JB5"
                        | "JD0"
                        | "JD3"
                        | "JTOPIC_0"
                        | "JTOPIC_1"
                        | "JTOPIC_5"
                        | "JPADDIA"
                        | "JPADDIB"
                        | "JPADDIC"
                        | "JPADDID"
                        | "JINDDA"
                        | "JINDDB"
                        | "JINDDC"
                        | "JINDDD"
                ) {
                    continue;
                }
                result.insert(wn, io);
            }
        }
        result
    }

    fn gather_special_io(&mut self) -> BTreeMap<WireName, EdgeIoCoord> {
        let jpaddia_pio = self.naming.strings.get("JPADDIA_PIO");
        let jpaddib_pio = self.naming.strings.get("JPADDIB_PIO");
        let jpaddic_pio = self.naming.strings.get("JPADDIC_PIO");
        let jpaddid_pio = self.naming.strings.get("JPADDID_PIO");
        let jpaddie_pio = self.naming.strings.get("JPADDIE_PIO");
        let jpaddif_pio = self.naming.strings.get("JPADDIF_PIO");
        let jpaddiea_pio = self.naming.strings.get("JPADDIEA_PIO");
        let jpaddieb_pio = self.naming.strings.get("JPADDIEB_PIO");
        let jpaddiec_pio = self.naming.strings.get("JPADDIEC_PIO");
        let jpaddied_pio = self.naming.strings.get("JPADDIED_PIO");
        let mut pad_nodes = HashMap::new();
        for (node, &wn) in self.nodes {
            let mut cell = self.chip.xlat_rc_wire(wn);
            let (bel, e) = if Some(wn.suffix) == jpaddia_pio {
                (bels::IO0, false)
            } else if Some(wn.suffix) == jpaddib_pio {
                (bels::IO1, false)
            } else if Some(wn.suffix) == jpaddic_pio {
                (bels::IO2, false)
            } else if Some(wn.suffix) == jpaddid_pio {
                (bels::IO3, false)
            } else if Some(wn.suffix) == jpaddie_pio {
                (bels::IO4, false)
            } else if Some(wn.suffix) == jpaddif_pio {
                (bels::IO5, false)
            } else if Some(wn.suffix) == jpaddiea_pio {
                (bels::IO0, true)
            } else if Some(wn.suffix) == jpaddieb_pio {
                (bels::IO1, true)
            } else if Some(wn.suffix) == jpaddiec_pio {
                (bels::IO2, true)
            } else if Some(wn.suffix) == jpaddied_pio {
                (bels::IO3, true)
            } else {
                continue;
            };
            if matches!(self.chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) {
                if e {
                    if cell.col == self.chip.col_w() {
                        cell.col += 1;
                    } else if cell.col == self.chip.col_e() {
                        cell.col -= 1;
                    } else {
                        unreachable!();
                    }
                }
                let mut io = self.chip.get_io_crd(cell.bel(bel));
                if !e {
                    match io {
                        EdgeIoCoord::W(ref mut row, ref mut iob) => {
                            if self.chip.rows[*row].io_w == IoGroupKind::None {
                                *row -= 2;
                            } else {
                                *iob = TileIobId::from_idx(iob.to_idx() + 2);
                            }
                        }
                        EdgeIoCoord::E(ref mut row, ref mut iob) => {
                            if self.chip.rows[*row].io_e == IoGroupKind::None {
                                *row -= 2;
                            } else {
                                *iob = TileIobId::from_idx(iob.to_idx() + 2);
                            }
                        }
                        EdgeIoCoord::S(ref mut col, ref mut iob) => {
                            if self.chip.columns[*col].io_s == IoGroupKind::None {
                                *iob = TileIobId::from_idx(iob.to_idx() + 2);
                                *col -= 2;
                            }
                        }
                        EdgeIoCoord::N(ref mut col, ref mut iob) => {
                            if self.chip.columns[*col].io_n == IoGroupKind::None {
                                *iob = TileIobId::from_idx(iob.to_idx() + 2);
                                *col -= 2;
                            }
                        }
                    }
                }
                pad_nodes.insert(node, io);
            } else {
                assert!(!e);
                let io = self.chip.get_io_crd(cell.bel(bel));
                pad_nodes.insert(node, io);
            }
        }
        let mut result = BTreeMap::new();
        for &(wf, wt) in self.grid.pips.keys() {
            let Some(&io) = pad_nodes.get(&wf) else {
                continue;
            };
            let wn = self.nodes[wt];
            let suffix = self.naming.strings[wn.suffix].as_str();
            if matches!(
                suffix,
                "JPADDIA"
                    | "JPADDIB"
                    | "DIA_IOLOGIC"
                    | "DIB_IOLOGIC"
                    | "DIA_SIOLOGIC"
                    | "DIB_SIOLOGIC"
                    | "DIA_XSIOLOGIC"
                    | "DIB_XSIOLOGIC"
                    | "DIEA_XSIOLOGIC"
                    | "DIEB_XSIOLOGIC"
                    | "DIEC_XSIOLOGIC"
                    | "DIED_XSIOLOGIC"
                    | "DIA_DQSIOL"
                    | "DIA_SDQSIOL"
                    | "JDQSI_DQS"
                    | "JDQSI_SDQS"
                    | "JF0"
                    | "JF1"
                    | "JF2"
                    | "JF3"
                    | "JF4"
                    | "JF5"
                    | "JF6"
                    | "JF7"
                    | "JQ0"
                    | "JQ1"
                    | "JOFX6"
                    | "JOFX7"
            ) {
                continue;
            }
            result.insert(wn, io);
        }
        result
    }

    fn gather_special_io_machxo2(&mut self) -> BTreeMap<WireName, (EdgeIoCoord, PinDir)> {
        let jpaddia_pio = self.naming.strings.get("JPADDIA_PIO").unwrap();
        let jpaddib_pio = self.naming.strings.get("JPADDIB_PIO").unwrap();
        let jpaddic_pio = self.naming.strings.get("JPADDIC_PIO").unwrap();
        let jpaddid_pio = self.naming.strings.get("JPADDID_PIO").unwrap();
        let jdia = self.naming.strings.get("JDIA").unwrap();
        let jdib = self.naming.strings.get("JDIB").unwrap();
        let jdic = self.naming.strings.get("JDIC").unwrap();
        let jdid = self.naming.strings.get("JDID").unwrap();
        let jpaddoa = self.naming.strings.get("JPADDOA").unwrap();
        let jpaddob = self.naming.strings.get("JPADDOB").unwrap();
        let jpaddoc = self.naming.strings.get("JPADDOC").unwrap();
        let jpaddod = self.naming.strings.get("JPADDOD").unwrap();
        let mut pad_nodes = HashMap::new();
        for (node, &wn) in self.nodes {
            let mut cell = self.chip.xlat_rc_wire(wn);
            let (bel, e, dir) = if wn.suffix == jpaddia_pio || wn.suffix == jdia {
                (bels::IO0, false, PinDir::Input)
            } else if wn.suffix == jpaddib_pio || wn.suffix == jdib {
                (bels::IO1, false, PinDir::Input)
            } else if wn.suffix == jpaddic_pio || wn.suffix == jdic {
                (bels::IO2, false, PinDir::Input)
            } else if wn.suffix == jpaddid_pio || wn.suffix == jdid {
                (bels::IO3, false, PinDir::Input)
            } else if wn.suffix == jpaddoa {
                (bels::IO0, false, PinDir::Output)
            } else if wn.suffix == jpaddob {
                (bels::IO1, false, PinDir::Output)
            } else if wn.suffix == jpaddoc {
                (bels::IO2, false, PinDir::Output)
            } else if wn.suffix == jpaddod {
                (bels::IO3, false, PinDir::Output)
            } else {
                continue;
            };
            if matches!(self.chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) {
                if e {
                    if cell.col == self.chip.col_w() {
                        cell.col += 1;
                    } else if cell.col == self.chip.col_e() {
                        cell.col -= 1;
                    } else {
                        unreachable!();
                    }
                }
                let mut io = self.chip.get_io_crd(cell.bel(bel));
                if !e {
                    match io {
                        EdgeIoCoord::W(ref mut row, ref mut iob) => {
                            if self.chip.rows[*row].io_w == IoGroupKind::None {
                                *row -= 2;
                            } else {
                                *iob = TileIobId::from_idx(iob.to_idx() + 2);
                            }
                        }
                        EdgeIoCoord::E(ref mut row, ref mut iob) => {
                            if self.chip.rows[*row].io_e == IoGroupKind::None {
                                *row -= 2;
                            } else {
                                *iob = TileIobId::from_idx(iob.to_idx() + 2);
                            }
                        }
                        EdgeIoCoord::S(ref mut col, ref mut iob) => {
                            if self.chip.columns[*col].io_s == IoGroupKind::None {
                                *iob = TileIobId::from_idx(iob.to_idx() + 2);
                                *col -= 2;
                            }
                        }
                        EdgeIoCoord::N(ref mut col, ref mut iob) => {
                            if self.chip.columns[*col].io_n == IoGroupKind::None {
                                *iob = TileIobId::from_idx(iob.to_idx() + 2);
                                *col -= 2;
                            }
                        }
                    }
                }
                pad_nodes.insert(node, (io, dir));
            } else {
                assert!(!e);
                let io = self.chip.get_io_crd(cell.bel(bel));
                pad_nodes.insert(node, (io, dir));
            }
        }
        let mut result = BTreeMap::new();
        for &(wf, wt) in self.grid.pips.keys() {
            if let Some(&io) = pad_nodes.get(&wf) {
                let wn = self.nodes[wt];
                let suffix = self.naming.strings[wn.suffix].as_str();
                if matches!(
                    suffix,
                    "PADDIA_IOLOGIC"
                        | "PADDIB_IOLOGIC"
                        | "PADDIC_IOLOGIC"
                        | "PADDID_IOLOGIC"
                        | "PADDIA_RIOLOGIC"
                        | "PADDIB_RIOLOGIC"
                        | "PADDIC_RIOLOGIC"
                        | "PADDID_RIOLOGIC"
                        | "PADDIA_BIOLOGIC"
                        | "PADDIC_BSIOLOGIC"
                        | "PADDIA_TIOLOGIC"
                        | "PADDIC_TSIOLOGIC"
                        | "DIA_IOLOGIC"
                        | "DIB_IOLOGIC"
                        | "DIC_IOLOGIC"
                        | "DID_IOLOGIC"
                        | "DIA_RIOLOGIC"
                        | "DIB_RIOLOGIC"
                        | "DIC_RIOLOGIC"
                        | "DID_RIOLOGIC"
                        | "DIA_BIOLOGIC"
                        | "DIC_BSIOLOGIC"
                        | "DIA_TIOLOGIC"
                        | "DIC_TSIOLOGIC"
                        | "JDIA"
                        | "JDIB"
                        | "JDIC"
                        | "JDID"
                        | "JQ0"
                        | "JQ1"
                        | "JQ2"
                        | "JQ3"
                        | "PADDOA_PIO"
                        | "PADDOB_PIO"
                        | "PADDOC_PIO"
                        | "PADDOD_PIO"
                        | "JPADDOA_PIO"
                        | "JPADDOB_PIO"
                        | "JPADDOC_PIO"
                        | "JPADDOD_PIO"
                ) {
                    continue;
                }
                result.insert(wn, io);
            } else if let Some(&io) = pad_nodes.get(&wt) {
                let wn = self.nodes[wf];
                let suffix = self.naming.strings[wn.suffix].as_str();
                if matches!(
                    suffix,
                    "JA0"
                        | "JA1"
                        | "JA2"
                        | "JA3"
                        | "INDDA_IOLOGIC"
                        | "INDDB_IOLOGIC"
                        | "INDDC_IOLOGIC"
                        | "INDDD_IOLOGIC"
                        | "INDDA_RIOLOGIC"
                        | "INDDB_RIOLOGIC"
                        | "INDDC_RIOLOGIC"
                        | "INDDD_RIOLOGIC"
                        | "INDDA_BIOLOGIC"
                        | "INDDC_BSIOLOGIC"
                        | "INDDA_TIOLOGIC"
                        | "INDDC_TSIOLOGIC"
                ) {
                    continue;
                }
                result.insert(wn, io);
            }
        }
        result
    }

    fn fill_special_io_scm(&mut self) {
        for (wn, (io, _dir)) in self.gather_special_io_scm() {
            let cell = self.chip.xlat_rc_wire(wn);
            let suffix = self.naming.strings[wn.suffix].as_str();
            let key = if let Some(idx) = suffix.strip_prefix("JPIO_")
                && let Ok(idx) = idx.parse()
            {
                let mut idx: u8 = idx;
                let edge = if cell.col == self.chip.col_w() {
                    Dir::W
                } else if cell.col == self.chip.col_e() {
                    Dir::E
                } else if cell.row == self.chip.row_s() {
                    if cell.col >= self.chip.col_clk {
                        idx += 8;
                    }
                    Dir::S
                } else if cell.row == self.chip.row_n() {
                    Dir::N
                } else {
                    unreachable!()
                };
                SpecialIoKey::Clock(edge, idx)
            } else if let Some(which) = suffix.strip_prefix("JPCK")
                && which.len() == 2
                && let Ok(bank) = which[..1].parse()
                && let Ok(idx) = which[1..].parse()
            {
                let bank: u32 = bank;
                let idx: u8 = idx;
                let (edge, idx) = match bank {
                    1 => (Dir::N, idx),
                    2 => (Dir::E, idx),
                    3 => (Dir::E, idx + 4),
                    4 => (Dir::S, idx + 8),
                    5 => (Dir::S, idx),
                    6 => (Dir::W, idx + 4),
                    7 => (Dir::W, idx),
                    _ => unreachable!(),
                };
                SpecialIoKey::Clock(edge, idx)
            } else if suffix.starts_with("JPIO_IN_") || suffix.starts_with("JPIO_FB_") {
                let pad = match suffix {
                    "JPIO_IN_A" => PllPad::PllIn0,
                    "JPIO_IN_B" => PllPad::PllIn1,
                    "JPIO_IN_C" => PllPad::DllIn0,
                    "JPIO_IN_D" => PllPad::DllIn1,
                    "JPIO_IN_E" => PllPad::DllIn2,
                    "JPIO_IN_F" => PllPad::DllIn3,
                    "JPIO_FB_A" => PllPad::PllIn1,
                    "JPIO_FB_B" => PllPad::PllIn0,
                    "JPIO_FB_C" => PllPad::DllIn1,
                    "JPIO_FB_D" => PllPad::DllIn0,
                    "JPIO_FB_E" => PllPad::DllIn3,
                    "JPIO_FB_F" => PllPad::DllIn2,
                    _ => {
                        println!("WEIRD SPECIO: R{r}C{c}_{suffix} {io}", r = wn.r, c = wn.c);
                        continue;
                    }
                };
                let h = if cell.col == self.chip.col_w() {
                    DirH::W
                } else if cell.col == self.chip.col_e() {
                    DirH::E
                } else {
                    unreachable!()
                };
                let v = if cell.row == self.chip.row_s() {
                    DirV::S
                } else if cell.row == self.chip.row_n() {
                    DirV::N
                } else {
                    unreachable!()
                };
                SpecialIoKey::Pll(pad, PllLoc::new_hv(h, v, 0))
            } else if let Some(suffix) = suffix.strip_prefix("JMPIWRDATA")
                && let Some(idx) = suffix.strip_suffix("_SYSBUS")
            {
                let idx = idx.parse().unwrap();
                SpecialIoKey::D(idx)
            } else if let Some(suffix) = suffix.strip_prefix("JMPIWRPARITY")
                && let Some(idx) = suffix.strip_suffix("_SYSBUS")
            {
                let idx = idx.parse().unwrap();
                SpecialIoKey::DP(idx)
            } else if let Some(suffix) = suffix.strip_prefix("JMPIRDDATA")
                && let Some(idx) = suffix.strip_suffix("_SYSBUS")
            {
                let idx = idx.parse().unwrap();
                SpecialIoKey::D(idx)
            } else if let Some(suffix) = suffix.strip_prefix("JMPIRDPARITY")
                && let Some(idx) = suffix.strip_suffix("_SYSBUS")
            {
                let idx = idx.parse().unwrap();
                SpecialIoKey::DP(idx)
            } else if let Some(suffix) = suffix.strip_prefix("JMPIADDR")
                && let Some(idx) = suffix.strip_suffix("_SYSBUS")
            {
                let idx: u8 = idx.parse().unwrap();
                SpecialIoKey::A(idx - 14)
            } else {
                match suffix {
                    "JCS0N_SYSBUS" => SpecialIoKey::CsN,
                    "JCS1_SYSBUS" => SpecialIoKey::Cs1,
                    "JMPITSIZ0_SYSBUS" => SpecialIoKey::A(18),
                    "JMPITSIZ1_SYSBUS" => SpecialIoKey::A(19),
                    "JMPIBDIP_SYSBUS" => SpecialIoKey::A(20),
                    "JMPIBURST_SYSBUS" => SpecialIoKey::A(21),
                    "JMPICLK_SYSBUS" => SpecialIoKey::MpiClk,
                    "JMPIRWN_SYSBUS" => SpecialIoKey::WriteN,
                    "JMPISTRBN_SYSBUS" => SpecialIoKey::ReadN,
                    "JMPITA_SYSBUS" => SpecialIoKey::MpiAckN,
                    "JMPITEA_SYSBUS" => SpecialIoKey::MpiTeaN,
                    "JMPIRETRY_SYSBUS" => SpecialIoKey::MpiRetryN,
                    "JEXTDONEI_SYSBUS" => SpecialIoKey::ExtDoneI,
                    "JEXTDONEO_SYSBUS" => SpecialIoKey::ExtDoneO,
                    "JEXTCLKP1I_SYSBUS" => SpecialIoKey::ExtClkI(1),
                    "JEXTCLKP2I_SYSBUS" => SpecialIoKey::ExtClkI(2),
                    "JEXTCLKP1O_SYSBUS" => SpecialIoKey::ExtClkO(1),
                    "JEXTCLKP2O_SYSBUS" => SpecialIoKey::ExtClkO(2),
                    _ => {
                        println!("WEIRD SPECIO: R{r}C{c}_{suffix} {io}", r = wn.r, c = wn.c);
                        continue;
                    }
                }
            };
            match self.chip.special_io.entry(key) {
                btree_map::Entry::Vacant(e) => {
                    e.insert(io);
                }
                btree_map::Entry::Occupied(e) => {
                    assert_eq!(*e.get(), io);
                }
            }
        }
        let col_clk_w = self
            .chip
            .columns
            .ids()
            .rev()
            .filter(|&col| col < self.chip.col_clk)
            .find(|&col| self.chip.columns[col].pclk_drive)
            .unwrap();
        let col_clk_e = self
            .chip
            .columns
            .ids()
            .filter(|&col| col >= self.chip.col_clk)
            .find(|&col| self.chip.columns[col].pclk_drive)
            .unwrap();
        self.chip.special_loc.insert(
            SpecialLocKey::Bc(5),
            CellCoord::new(DieId::from_idx(0), col_clk_w, self.chip.row_s()),
        );
        self.chip.special_loc.insert(
            SpecialLocKey::Bc(4),
            CellCoord::new(DieId::from_idx(0), col_clk_e, self.chip.row_s()),
        );
        self.chip.special_loc.insert(
            SpecialLocKey::Bc(7),
            CellCoord::new(DieId::from_idx(0), self.chip.col_w(), self.chip.row_clk),
        );
        self.chip.special_loc.insert(
            SpecialLocKey::Bc(2),
            CellCoord::new(DieId::from_idx(0), self.chip.col_e(), self.chip.row_clk),
        );
        self.chip.special_loc.insert(
            SpecialLocKey::Bc(1),
            CellCoord::new(DieId::from_idx(0), self.chip.col_clk, self.chip.row_n()),
        );
    }

    fn fill_special_io_ecp(&mut self) {
        for (wn, io) in self.gather_special_io() {
            let suffix = self.naming.strings[wn.suffix].as_str();
            if let Some(edge) = match suffix {
                "JLPIO" => Some(Dir::W),
                "JRPIO" => Some(Dir::E),
                "JBPIO" => Some(Dir::S),
                "JTPIO" => Some(Dir::N),
                _ => None,
            } {
                self.chip
                    .special_io
                    .insert(SpecialIoKey::Clock(edge, 0), io);
            } else if let Some(pad) = match suffix {
                "JCLKI3" => Some(PllPad::PllIn0),
                "JCLKFB3" => Some(PllPad::PllFb),
                _ => None,
            } {
                let cell = self.chip.xlat_rc_wire(wn);
                let h = if cell.col < self.chip.col_clk {
                    DirH::W
                } else {
                    DirH::E
                };
                let v = if cell.row < self.chip.row_clk
                    || (cell.row == self.chip.row_clk && self.chip.kind == ChipKind::Ecp)
                {
                    DirV::S
                } else {
                    DirV::N
                };
                let hv = DirHV { h, v };
                let key = SpecialIoKey::Pll(pad, PllLoc::new(hv, 0));
                self.chip.special_io.insert(key, io);
            } else {
                panic!("WEIRD SPECIO: R{r}C{c}_{suffix} {io}", r = wn.r, c = wn.c);
            }
        }
    }

    fn fill_pll_ecp2(&mut self) {
        let ebr_rows = Vec::from_iter(
            self.chip
                .rows
                .iter()
                .filter(|&(_, rd)| rd.kind == RowKind::Ebr)
                .map(|(row, _)| row),
        );
        let dsp_rows = Vec::from_iter(
            self.chip
                .rows
                .iter()
                .filter(|&(row, rd)| rd.kind == RowKind::Dsp && row != self.chip.row_clk)
                .map(|(row, _)| row),
        );
        for edge in [DirH::W, DirH::E] {
            self.chip.special_loc.insert(
                SpecialLocKey::Pll(PllLoc::new_hv(edge, DirV::S, 0)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_edge(edge), ebr_rows[0]),
            );
            if ebr_rows.len() == 2 {
                self.chip.special_loc.insert(
                    SpecialLocKey::Pll(PllLoc::new_hv(edge, DirV::N, 0)),
                    CellCoord::new(DieId::from_idx(0), self.chip.col_edge(edge), ebr_rows[1]),
                );
            }
            if !dsp_rows.is_empty() {
                self.chip.special_loc.insert(
                    SpecialLocKey::Pll(PllLoc::new_hv(edge, DirV::N, 1)),
                    CellCoord::new(DieId::from_idx(0), self.chip.col_edge(edge), dsp_rows[0]),
                );
            }
        }
    }

    fn fill_pll_ecp2m(&mut self) {
        let bot_rows = Vec::from_iter(
            self.chip
                .rows
                .iter()
                .filter(|&(row, rd)| rd.kind == RowKind::Ebr && row < self.chip.row_clk)
                .map(|(row, _)| row),
        );
        let top_rows = Vec::from_iter(
            self.chip
                .rows
                .iter()
                .filter(|&(row, rd)| rd.kind == RowKind::Ebr && row >= self.chip.row_clk)
                .map(|(row, _)| row),
        );
        for edge in [DirH::W, DirH::E] {
            self.chip.special_loc.insert(
                SpecialLocKey::Pll(PllLoc::new_hv(edge, DirV::S, 0)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_edge(edge), bot_rows[0]),
            );
            if bot_rows.len() > 1 {
                self.chip.special_loc.insert(
                    SpecialLocKey::Pll(PllLoc::new_hv(edge, DirV::S, 1)),
                    CellCoord::new(
                        DieId::from_idx(0),
                        self.chip.col_edge(edge),
                        *bot_rows.last().unwrap(),
                    ),
                );
            }
            if top_rows.len() > 1 {
                self.chip.special_loc.insert(
                    SpecialLocKey::Pll(PllLoc::new_hv(edge, DirV::N, 1)),
                    CellCoord::new(DieId::from_idx(0), self.chip.col_edge(edge), top_rows[0]),
                );
            }
            if !top_rows.is_empty() {
                self.chip.special_loc.insert(
                    SpecialLocKey::Pll(PllLoc::new_hv(edge, DirV::N, 0)),
                    CellCoord::new(
                        DieId::from_idx(0),
                        self.chip.col_edge(edge),
                        *top_rows.last().unwrap(),
                    ),
                );
            }
        }
    }

    fn fill_pll_xp2(&mut self) {
        for &wn in self.nodes.values() {
            if self.naming.strings[wn.suffix].ends_with("_PLL") {
                let cell = self.chip.xlat_rc_wire(wn);
                let v = if cell.row < self.chip.row_clk {
                    DirV::S
                } else {
                    DirV::N
                };
                let h = if cell.col < self.chip.col_clk {
                    DirH::W
                } else {
                    DirH::E
                };
                let loc = SpecialLocKey::Pll(PllLoc::new_hv(h, v, 0));
                self.chip.special_loc.insert(loc, cell);
            }
        }
    }

    fn fill_pll_ecp3(&mut self) {
        let mut plls: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
        for &wn in self.nodes.values() {
            if self.naming.strings[wn.suffix].ends_with("_PLL") {
                let cell = self.chip.xlat_rc_wire(wn);
                let v = if cell.row < self.chip.row_clk {
                    DirV::S
                } else {
                    DirV::N
                };
                let h = if cell.col < self.chip.col_clk {
                    DirH::W
                } else {
                    DirH::E
                };
                let cell = cell.delta(
                    match h {
                        DirH::W => -3,
                        DirH::E => 3,
                    },
                    0,
                );
                plls.entry(DirHV { h, v }).or_default().insert(cell);
            }
        }
        for (quad, cells) in plls {
            let mut cells = Vec::from_iter(cells);
            match quad.v {
                DirV::S => {
                    cells.sort_by_key(|cell| std::cmp::Reverse(cell.row));
                }
                DirV::N => {
                    cells.sort();
                }
            }
            for (i, cell) in cells.into_iter().enumerate() {
                let loc = SpecialLocKey::Pll(PllLoc::new(quad, i as u8));
                self.chip.special_loc.insert(loc, cell);
            }
        }
    }

    fn fill_serdes_ecp2m(&mut self) {
        let name = self.naming.strings.get("JFF_TX_D_0_0_PCS").unwrap();
        for &wn in self.nodes.values() {
            if wn.suffix == name {
                let mut cell = self.chip.xlat_rc_wire(wn);
                if cell.col < self.chip.col_clk {
                    cell.col -= 12;
                } else {
                    cell.col -= 13;
                }
                if cell.row < self.chip.row_clk {
                    self.chip.columns[cell.col].io_s = IoGroupKind::Serdes;
                    self.chip.columns[cell.col].bank_s = if cell.col < self.chip.col_clk {
                        Some(14)
                    } else {
                        Some(13)
                    };
                } else {
                    self.chip.columns[cell.col].io_n = IoGroupKind::Serdes;
                    self.chip.columns[cell.col].bank_n = if cell.col < self.chip.col_clk {
                        Some(11)
                    } else {
                        Some(12)
                    };
                }
            }
        }
    }

    fn fill_serdes_ecp3(&mut self) {
        let name = self.naming.strings.get("JFF_TX_D_0_0_PCS").unwrap();
        for &wn in self.nodes.values() {
            if wn.suffix == name {
                let cell = self.chip.xlat_rc_wire(wn);
                self.chip.columns[cell.col].io_s = IoGroupKind::Serdes;
                self.chip.columns[cell.col].bank_s = if cell.col < self.chip.col_clk {
                    Some(14)
                } else {
                    Some(13)
                };
            }
        }
        let mut bank = 50;
        for (col, cd) in &mut self.chip.columns {
            if col < self.chip.col_clk - 18 {
                continue;
            }
            if cd.io_s == IoGroupKind::Serdes {
                cd.bank_s = Some(bank);
                bank += 2;
            }
        }
        let mut bank = 51;
        for (col, cd) in self.chip.columns.iter_mut().rev() {
            if col >= self.chip.col_clk - 18 {
                continue;
            }
            if cd.io_s == IoGroupKind::Serdes {
                cd.bank_s = Some(bank);
                bank += 2;
            }
        }
    }

    fn fill_serdes_ecp4(&mut self) {
        let (loc, col) = match self.chip.rows.len() {
            78 => (SpecialLocKey::SerdesSingle, 16),
            128 => (SpecialLocKey::SerdesDouble, 12),
            130 => (SpecialLocKey::SerdesTriple, 13),
            _ => unreachable!(),
        };
        self.chip.special_loc.insert(
            loc,
            CellCoord::new(DieId::from_idx(0), ColId::from_idx(col), self.chip.row_s()),
        );
    }

    fn fill_serdes_ecp5(&mut self) {
        let cols = match self.chip.rows.len() {
            49 => [41].as_slice(),
            70 => [41, 68].as_slice(),
            94 => [45, 70].as_slice(),
            _ => unreachable!(),
        };
        for (i, &col) in cols.iter().enumerate() {
            let col = ColId::from_idx(col);
            self.chip.columns[col].io_s = IoGroupKind::Serdes;
            self.chip.columns[col].bank_s = Some(50 + (i as u32));
        }
    }

    fn fill_mipi_crosslink(&mut self) {
        let name = self.naming.strings.get("DP0_MIPIDPHY").unwrap();
        for &wn in self.nodes.values() {
            if wn.suffix == name {
                let cell = self.chip.xlat_rc_wire(wn);
                self.chip.columns[cell.col].io_n = IoGroupKind::Mipi;
                self.chip.columns[cell.col].bank_n = if cell.col < self.chip.col_clk {
                    Some(60)
                } else {
                    Some(61)
                };
            }
        }
    }

    fn fill_special_io_ecp2(&mut self) {
        let pll_xlat =
            BTreeMap::from_iter(self.chip.special_loc.iter().filter_map(|(&key, &cell)| {
                if let SpecialLocKey::Pll(loc) = key {
                    Some((cell, loc))
                } else {
                    None
                }
            }));
        for (wn, io) in self.gather_special_io() {
            let suffix = self.naming.strings[wn.suffix].as_str();
            if let Some((edge, i)) = match suffix {
                "JLPIO0" => Some((Dir::W, 0)),
                "JRPIO0" => Some((Dir::E, 0)),
                "JBPIO0" => Some((Dir::S, 0)),
                "JTPIO0" => Some((Dir::N, 0)),
                "JLPIO1" => Some((Dir::W, 1)),
                "JRPIO1" => Some((Dir::E, 1)),
                "JBPIO1" => Some((Dir::S, 1)),
                "JTPIO1" => Some((Dir::N, 1)),
                _ => None,
            } {
                self.chip
                    .special_io
                    .insert(SpecialIoKey::Clock(edge, i), io);
            } else if matches!(
                suffix,
                "JPIO0"
                    | "JPIO1"
                    | "JPLLPIO0"
                    | "JPLLPIO1"
                    | "JCLK_SSPIPIN"
                    | "JCS_SSPIPIN"
                    | "JSI_SSPIPIN"
            ) {
                // discard — redundant
            } else if let Some(pad) = match suffix {
                "JPLLCLKI0" => Some(PllPad::PllIn1),
                "JPLLCLKI3" => Some(PllPad::PllIn0),
                "JPLLCLKFB1" => Some(PllPad::PllFb),
                "JDLLCLKI0" => Some(PllPad::DllIn0),
                "JDLLCLKI3" => Some(PllPad::DllIn1),
                "JDLLCLKFB1" => Some(PllPad::DllFb),
                "JSPLLCLKI0" => Some(PllPad::PllIn1),
                "JSPLLCLKI3" => Some(PllPad::PllIn0),
                "JSPLLCLKFB1" => Some(PllPad::PllFb),
                _ => None,
            } {
                let mut cell = self.chip.xlat_rc_wire(wn);
                if cell.col == self.chip.col_w() + 2 {
                    cell.col = self.chip.col_w();
                } else if cell.col == self.chip.col_e() - 2 {
                    cell.col = self.chip.col_e();
                }
                let pll = pll_xlat[&cell];
                let key = SpecialIoKey::Pll(pad, pll);
                self.chip.special_io.insert(key, io);
            } else {
                println!(
                    "{name}: WEIRD SPECIO: R{r}C{c}_{suffix} {io}",
                    name = self.name,
                    r = wn.r,
                    c = wn.c,
                );
            }
        }
    }

    fn fill_special_io_ecp3(&mut self) {
        let pll_xlat =
            BTreeMap::from_iter(self.chip.special_loc.iter().filter_map(|(&key, &cell)| {
                if let SpecialLocKey::Pll(loc) = key {
                    Some((cell, loc))
                } else {
                    None
                }
            }));
        for (wn, io) in self.gather_special_io() {
            let suffix = self.naming.strings[wn.suffix].as_str();
            if let Some((edge, i)) = match suffix {
                "JLPIO0" => Some((Dir::W, 0)),
                "JRPIO0" => Some((Dir::E, 0)),
                "JTPIO0" => Some((Dir::N, 0)),
                "JLPIO1" => Some((Dir::W, 1)),
                "JRPIO1" => Some((Dir::E, 1)),
                "JTPIO1" => Some((Dir::N, 1)),
                _ => None,
            } {
                self.chip
                    .special_io
                    .insert(SpecialIoKey::Clock(edge, i), io);
            } else if matches!(
                suffix,
                "JPIO1"
                    | "JPIO2"
                    | "JPLLPIO1"
                    | "JPLLPIO2"
                    | "JPLLCLKI4"
                    | "JPLLCLKI0"
                    | "JDLLCLKI3"
                    | "JDLLCLKI4"
            ) {
                // discard — redundant
            } else if let Some(pad) = match suffix {
                "JPLLCLKI3" => Some(PllPad::PllIn0),
                "JPLLCLKFB1" => Some(PllPad::PllFb),
                "JDLLCLKI0" => Some(PllPad::DllIn0),
                "JDLLCLKFB1" => Some(PllPad::DllFb),
                _ => None,
            } {
                let mut cell = self.chip.xlat_rc_wire(wn);
                if cell.col < self.chip.col_clk {
                    cell.col = self.chip.col_w() + 1;
                } else {
                    cell.col = self.chip.col_e() - 1;
                }
                let pll = pll_xlat[&cell];
                let key = SpecialIoKey::Pll(pad, pll);
                self.chip.special_io.insert(key, io);
            } else {
                println!(
                    "{name}: WEIRD SPECIO: R{r}C{c}_{suffix} {io}",
                    name = self.name,
                    r = wn.r,
                    c = wn.c,
                );
            }
        }
    }

    fn fill_special_io_machxo2(&mut self) {
        fn get_edge(chip: &Chip, wn: WireName) -> Dir {
            let cell = chip.xlat_rc_wire(wn);
            if cell.col == chip.col_w() {
                Dir::W
            } else if cell.col == chip.col_e() {
                Dir::E
            } else if cell.row == chip.row_s() {
                Dir::S
            } else if cell.row == chip.row_n() {
                Dir::N
            } else {
                unreachable!()
            }
        }
        fn get_pll(chip: &Chip, wn: WireName) -> PllLoc {
            let cell = chip.xlat_rc_wire(wn);
            if cell.col < chip.col_clk {
                PllLoc::new(DirHV::NW, 0)
            } else {
                PllLoc::new(DirHV::NE, 0)
            }
        }
        for (wn, (io, dir)) in self.gather_special_io_machxo2() {
            let suffix = self.naming.strings[wn.suffix].as_str();
            let key = match suffix {
                "JTCK_JTAG" => SpecialIoKey::Tck,
                "JTMS_JTAG" => SpecialIoKey::Tms,
                "JTDI_JTAG" => SpecialIoKey::Tdi,
                "JTDO_JTAG" => SpecialIoKey::Tdo,
                "JUFMSN_EFB" => SpecialIoKey::SpiPCsB,
                "JSPIMCSN0_EFB" => SpecialIoKey::SpiCCsB,
                "JSPIMOSII_EFB" => SpecialIoKey::D(0),
                "JSPIMOSIO_EFB" => SpecialIoKey::D(0),
                "JSPIMISOI_EFB" => SpecialIoKey::D(1),
                "JSPIMISOO_EFB" => SpecialIoKey::D(1),
                "JSPISCKI_EFB" => SpecialIoKey::Cclk,
                "JSPISCKO_EFB" => SpecialIoKey::Cclk,
                "JI2C1SCLI_EFB" => SpecialIoKey::D(2),
                "JI2C1SCLO_EFB" => SpecialIoKey::D(2),
                "JI2C1SDAI_EFB" => SpecialIoKey::D(3),
                "JI2C1SDAO_EFB" => SpecialIoKey::D(3),
                "JPCLKT00" => SpecialIoKey::Clock(Dir::N, 0),
                "JPCLKT01" => SpecialIoKey::Clock(Dir::N, 1),
                "JPCLKT10" => SpecialIoKey::Clock(Dir::E, 0),
                "JPCLKT20" => SpecialIoKey::Clock(Dir::S, 0),
                "JPCLKT21" => SpecialIoKey::Clock(Dir::S, 1),
                "JPCLKT30" => SpecialIoKey::Clock(Dir::W, 0),
                "JPCLKT31" => SpecialIoKey::Clock(Dir::W, 1),
                "JPCLKT32" => SpecialIoKey::Clock(Dir::W, 2),
                "JPADDI0" | "JCLKI0_DLLDEL" => SpecialIoKey::Clock(get_edge(&self.chip, wn), 0),
                "JPADDI1" | "JCLKI1_DLLDEL" => SpecialIoKey::Clock(get_edge(&self.chip, wn), 1),
                "JPADDI2" | "JCLKI2_DLLDEL" => SpecialIoKey::Clock(get_edge(&self.chip, wn), 2),
                "JREFCLK3" => SpecialIoKey::Pll(PllPad::PllIn0, get_pll(&self.chip, wn)),
                "JCLKFB0" => SpecialIoKey::Pll(PllPad::PllFb, get_pll(&self.chip, wn)),
                "JDQSI0_DQS" => SpecialIoKey::DqsE(0),
                "JDQSI1_DQS" => SpecialIoKey::DqsE(1),
                // redundant
                "JREFCLK4" | "JREFCLK5" | "JREFCLK6" | "JREFCLK7" => continue,
                _ => {
                    println!(
                        "{name}: WEIRD SPECIO: R{r}C{c}_{suffix} {io} {dir:?}",
                        name = self.name,
                        r = wn.r,
                        c = wn.c,
                    );
                    continue;
                }
            };
            match self.chip.special_io.entry(key) {
                btree_map::Entry::Vacant(e) => {
                    e.insert(io);
                }
                btree_map::Entry::Occupied(e) => {
                    assert_eq!(*e.get(), io);
                }
            }
        }
    }

    fn fill_special_io_ecp4(&mut self) {
        let has_bank0 = self.chip.special_loc.contains_key(&SpecialLocKey::Bc(0));
        for (idx, dx, iob, cond) in [
            (0, -9, 2, has_bank0),
            (1, -9, 0, has_bank0),
            (2, -5, 2, true),
            (3, -5, 0, true),
            (4, 1, 2, true),
            (5, 1, 0, true),
            (6, 5, 2, has_bank0),
            (7, 5, 0, has_bank0),
        ] {
            if cond {
                let col = self.chip.col_clk + dx;
                let iob = TileIobId::from_idx(iob);
                let io = EdgeIoCoord::N(col, iob);
                self.chip
                    .special_io
                    .insert(SpecialIoKey::Clock(Dir::N, idx), io);
            }
        }
        for (idx, dy, iob) in [(0, -5, 2), (1, -5, 0), (2, 1, 2), (3, 1, 0)] {
            let row = self.chip.row_clk + dy;
            let iob = TileIobId::from_idx(iob);
            let io = EdgeIoCoord::W(row, iob);
            self.chip
                .special_io
                .insert(SpecialIoKey::Clock(Dir::W, idx), io);
            let io = EdgeIoCoord::E(row, iob);
            self.chip
                .special_io
                .insert(SpecialIoKey::Clock(Dir::E, idx), io);
        }

        let mut quads_w = vec![];
        let mut quads_e = vec![];
        for (row, rd) in &self.chip.rows {
            if rd.kind == RowKind::Ebr {
                quads_w.push((row, 4));
                quads_e.push((row, 4));
            }
            if matches!(
                rd.io_w,
                IoGroupKind::Quad | IoGroupKind::QuadDqs | IoGroupKind::QuadEbrDqs
            ) {
                quads_w.push((row, 0));
            }
            if matches!(
                rd.io_e,
                IoGroupKind::Quad | IoGroupKind::QuadDqs | IoGroupKind::QuadEbrDqs
            ) {
                quads_e.push((row, 0));
            }
        }
        let mut quads_n = vec![];
        for (col, cd) in &self.chip.columns {
            if matches!(cd.io_n, IoGroupKind::Quad | IoGroupKind::QuadDqs) {
                quads_n.push(col);
            }
        }

        let quads_w_s = Vec::from_iter(
            quads_w
                .iter()
                .copied()
                .filter(|&(row, _)| row < self.chip.row_clk),
        );

        let quad_q = match self.chip.rows.len() {
            78 => quads_w[6],
            128 | 130 => quads_w[10],
            _ => unreachable!(),
        };

        for ((row, iob), keys) in [
            (
                quads_w[0],
                [
                    Some(SpecialIoKey::Di),
                    Some(SpecialIoKey::Dout),
                    None,
                    Some(SpecialIoKey::WriteN),
                ],
            ),
            (
                quads_w[1],
                [
                    Some(SpecialIoKey::Pll(PllPad::PllFb, PllLoc::new(DirHV::SW, 1))),
                    None,
                    Some(SpecialIoKey::CsN),
                    Some(SpecialIoKey::Cs1N),
                ],
            ),
            (
                quads_w[2],
                [
                    Some(SpecialIoKey::D(1)),
                    Some(SpecialIoKey::D(0)),
                    Some(SpecialIoKey::Pll(PllPad::PllIn0, PllLoc::new(DirHV::SW, 1))),
                    None,
                ],
            ),
            (
                quads_w[3],
                [
                    Some(SpecialIoKey::D(4)),
                    Some(SpecialIoKey::D(3)),
                    None,
                    Some(SpecialIoKey::D(2)),
                ],
            ),
            (
                quads_w[4],
                [
                    Some(SpecialIoKey::Pll(PllPad::PllFb, PllLoc::new(DirHV::SW, 0))),
                    None,
                    Some(SpecialIoKey::D(6)),
                    Some(SpecialIoKey::D(5)),
                ],
            ),
            (
                quads_w[5],
                [
                    Some(SpecialIoKey::D(8)),
                    Some(SpecialIoKey::D(7)),
                    Some(SpecialIoKey::Pll(PllPad::PllIn0, PllLoc::new(DirHV::SW, 0))),
                    None,
                ],
            ),
            (
                quad_q,
                [
                    None,
                    None,
                    Some(SpecialIoKey::D(10)),
                    Some(SpecialIoKey::D(9)),
                ],
            ),
            (
                quads_w_s[quads_w_s.len() - 3],
                [
                    Some(SpecialIoKey::D(13)),
                    Some(SpecialIoKey::D(12)),
                    None,
                    Some(SpecialIoKey::D(11)),
                ],
            ),
            (
                quads_w_s[quads_w_s.len() - 2],
                [
                    None,
                    None,
                    Some(SpecialIoKey::D(15)),
                    Some(SpecialIoKey::D(14)),
                ],
            ),
            (
                quads_w[quads_w.len() - 1],
                [
                    Some(SpecialIoKey::Pll(PllPad::PllFb, PllLoc::new(DirHV::NW, 0))),
                    None,
                    Some(SpecialIoKey::Pll(PllPad::PllIn0, PllLoc::new(DirHV::NW, 0))),
                    None,
                ],
            ),
        ] {
            for (i, key) in keys.into_iter().enumerate() {
                if let Some(key) = key {
                    self.chip
                        .special_io
                        .insert(key, EdgeIoCoord::W(row, TileIobId::from_idx(iob + i)));
                }
            }
        }

        for ((row, iob), keys) in [
            (
                quads_e[0],
                [
                    Some(SpecialIoKey::Pll(PllPad::PllFb, PllLoc::new(DirHV::SE, 1))),
                    None,
                    Some(SpecialIoKey::Pll(PllPad::PllIn0, PllLoc::new(DirHV::SE, 1))),
                    None,
                ],
            ),
            (
                quads_e[2],
                [
                    Some(SpecialIoKey::Pll(PllPad::PllFb, PllLoc::new(DirHV::SE, 0))),
                    None,
                    Some(SpecialIoKey::Pll(PllPad::PllIn0, PllLoc::new(DirHV::SE, 0))),
                    None,
                ],
            ),
            (
                quads_e[quads_e.len() - 1],
                [
                    Some(SpecialIoKey::Pll(PllPad::PllFb, PllLoc::new(DirHV::NE, 0))),
                    None,
                    Some(SpecialIoKey::Pll(PllPad::PllIn0, PllLoc::new(DirHV::NE, 0))),
                    None,
                ],
            ),
        ] {
            for (i, key) in keys.into_iter().enumerate() {
                if let Some(key) = key {
                    self.chip
                        .special_io
                        .insert(key, EdgeIoCoord::E(row, TileIobId::from_idx(iob + i)));
                }
            }
        }
        for (col, hv) in [
            (quads_n[0], DirHV::NW),
            (quads_n[quads_n.len() - 1], DirHV::NE),
        ] {
            for (iob, key) in [
                (0, SpecialIoKey::Pll(PllPad::PllFb, PllLoc::new(hv, 1))),
                (2, SpecialIoKey::Pll(PllPad::PllIn0, PllLoc::new(hv, 1))),
            ] {
                self.chip
                    .special_io
                    .insert(key, EdgeIoCoord::N(col, TileIobId::from_idx(iob)));
            }
        }

        for hv in DirHV::DIRS {
            self.chip.special_loc.insert(
                SpecialLocKey::Pll(PllLoc::new(hv, 0)),
                CellCoord::new(
                    DieId::from_idx(0),
                    self.chip.col_edge(hv.h),
                    self.chip.row_edge(hv.v),
                ),
            );
        }
    }

    fn fill_special_io_ecp5(&mut self) {
        for (idx, dx) in [(0, -3), (1, -5), (2, 3), (3, 1)] {
            let col = self.chip.col_clk + dx;
            let iob = TileIobId::from_idx(0);
            let io = EdgeIoCoord::N(col, iob);
            self.chip
                .special_io
                .insert(SpecialIoKey::Clock(Dir::N, idx), io);
        }
        for (idx, dy, iob) in [(0, -3, 2), (1, -3, 0), (2, 0, 2), (3, 0, 0)] {
            let row = self.chip.row_clk + dy;
            let iob = TileIobId::from_idx(iob);
            let io = EdgeIoCoord::W(row, iob);
            self.chip
                .special_io
                .insert(SpecialIoKey::Clock(Dir::W, idx), io);
            let io = EdgeIoCoord::E(row, iob);
            self.chip
                .special_io
                .insert(SpecialIoKey::Clock(Dir::E, idx), io);
        }

        for (key, dx, iob) in [
            (SpecialIoKey::D(7), 3, 0),
            (SpecialIoKey::D(6), 3, 1),
            (SpecialIoKey::D(5), 5, 0),
            (SpecialIoKey::D(4), 5, 1),
            (SpecialIoKey::D(3), 8, 0),
            (SpecialIoKey::D(2), 8, 1),
            (SpecialIoKey::D(1), 10, 0),
            (SpecialIoKey::D(0), 10, 1),
            (SpecialIoKey::CsN, 12, 0),
            (SpecialIoKey::Cs1N, 12, 1),
            (SpecialIoKey::Di, 14, 0),
            (SpecialIoKey::Dout, 14, 1),
            (SpecialIoKey::WriteN, 17, 0),
        ] {
            let col = self.chip.col_w() + dx;
            let iob = TileIobId::from_idx(iob);
            let io = EdgeIoCoord::S(col, iob);
            self.chip.special_io.insert(key, io);
        }

        for (key, dy, iob) in [
            (SpecialIoKey::D(8), 3, 3),
            (SpecialIoKey::D(9), 3, 2),
            (SpecialIoKey::D(10), 6, 3),
            (SpecialIoKey::D(11), 6, 2),
            (SpecialIoKey::D(12), 6, 1),
            (SpecialIoKey::D(13), 6, 0),
            (SpecialIoKey::D(14), 9, 3),
            (SpecialIoKey::D(15), 9, 2),
        ] {
            let row = self.chip.row_s() + dy;
            let iob = TileIobId::from_idx(iob);
            let io = EdgeIoCoord::W(row, iob);
            self.chip.special_io.insert(key, io);
        }

        for h in [DirH::W, DirH::E] {
            let iob = TileIobId::from_idx(2);
            self.chip.special_io.insert(
                SpecialIoKey::Pll(PllPad::PllIn0, PllLoc::new_hv(h, DirV::S, 0)),
                match h {
                    DirH::W => EdgeIoCoord::W(self.chip.row_s(), iob),
                    DirH::E => EdgeIoCoord::E(self.chip.row_s(), iob),
                },
            );

            let loc = PllLoc::new_hv(h, DirV::N, 0);
            if self.chip.special_loc.contains_key(&SpecialLocKey::Pll(loc)) {
                let iob = TileIobId::from_idx(0);
                self.chip.special_io.insert(
                    SpecialIoKey::Pll(PllPad::PllIn0, loc),
                    match h {
                        DirH::W => EdgeIoCoord::W(self.chip.row_n() - 12, iob),
                        DirH::E => EdgeIoCoord::E(self.chip.row_n() - 12, iob),
                    },
                );
                self.chip.special_io.insert(
                    SpecialIoKey::Pll(PllPad::PllIn1, loc),
                    match h {
                        DirH::W => EdgeIoCoord::N(self.chip.col_w() + 3, iob),
                        DirH::E => EdgeIoCoord::N(self.chip.col_e() - 4, iob),
                    },
                );
            }
        }
    }

    fn fill_special_io_crosslink(&mut self) {
        for (col, iob, key) in [
            (1, 2, SpecialIoKey::MipiClk(DirH::W)),
            (
                11,
                0,
                SpecialIoKey::Pll(PllPad::PllIn0, PllLoc::new(DirHV::SE, 0)),
            ),
            (15, 0, SpecialIoKey::Clock(Dir::S, 0)),
            (15, 2, SpecialIoKey::Clock(Dir::S, 1)),
            (28, 0, SpecialIoKey::Clock(Dir::S, 2)),
            (28, 2, SpecialIoKey::Clock(Dir::S, 3)),
            (33, 2, SpecialIoKey::MipiClk(DirH::E)),
            (46, 0, SpecialIoKey::D(2)),
            (47, 0, SpecialIoKey::D(3)),
            (46, 0, SpecialIoKey::Clock(Dir::S, 4)),
            (47, 0, SpecialIoKey::Clock(Dir::S, 5)),
            (48, 0, SpecialIoKey::PmuWakeupN),
            (49, 0, SpecialIoKey::D(0)),
            (50, 0, SpecialIoKey::D(1)),
            (51, 0, SpecialIoKey::CsN),
            (52, 0, SpecialIoKey::Cclk),
        ] {
            let io = EdgeIoCoord::S(ColId::from_idx(col), TileIobId::from_idx(iob));
            self.chip.special_io.insert(key, io);
        }
        for (col, key) in [
            (5, SpecialLocKey::PclkIn(Dir::S, 0)),
            (33, SpecialLocKey::PclkIn(Dir::S, 1)),
        ] {
            self.chip.special_loc.insert(
                key,
                CellCoord::new(DieId::from_idx(0), ColId::from_idx(col), self.chip.row_s()),
            );
        }
    }

    fn fill_fabric_clock_ecp(&mut self) {
        let mut xlat = HashMap::new();
        for (name, key) in [
            ("JCIBLLQ", SpecialLocKey::PclkIn(Dir::W, 0)),
            ("JCIBURQ", SpecialLocKey::PclkIn(Dir::E, 0)),
            ("JCIBLRQ", SpecialLocKey::PclkIn(Dir::S, 0)),
            ("JCIBULQ", SpecialLocKey::PclkIn(Dir::N, 0)),
            ("JCIBL0", SpecialLocKey::SclkIn(Dir::W, 0)),
            ("JCIBL1", SpecialLocKey::SclkIn(Dir::W, 1)),
            ("JCIBL2", SpecialLocKey::SclkIn(Dir::W, 2)),
            ("JCIBL3", SpecialLocKey::SclkIn(Dir::W, 3)),
            ("JCIBR0", SpecialLocKey::SclkIn(Dir::E, 0)),
            ("JCIBR1", SpecialLocKey::SclkIn(Dir::E, 1)),
            ("JCIBR2", SpecialLocKey::SclkIn(Dir::E, 2)),
            ("JCIBR3", SpecialLocKey::SclkIn(Dir::E, 3)),
            ("JCIBB0", SpecialLocKey::SclkIn(Dir::S, 0)),
            ("JCIBB1", SpecialLocKey::SclkIn(Dir::S, 1)),
            ("JCIBB2", SpecialLocKey::SclkIn(Dir::S, 2)),
            ("JCIBB3", SpecialLocKey::SclkIn(Dir::S, 3)),
            ("JCIBT0", SpecialLocKey::SclkIn(Dir::N, 0)),
            ("JCIBT1", SpecialLocKey::SclkIn(Dir::N, 1)),
            ("JCIBT2", SpecialLocKey::SclkIn(Dir::N, 2)),
            ("JCIBT3", SpecialLocKey::SclkIn(Dir::N, 3)),
        ] {
            if let Some(s) = self.naming.strings.get(name) {
                xlat.insert(s, key);
            }
        }
        for &(wf, wt) in self.grid.pips.keys() {
            let wnt = self.nodes[wt];
            let Some(&key) = xlat.get(&wnt.suffix) else {
                continue;
            };
            let wnf = self.nodes[wf];
            let cell = self.chip.xlat_rc_wire(wnf);
            self.chip.special_loc.insert(key, cell);
        }
    }

    fn fill_fabric_clock_ecp2(&mut self) {
        let mut xlat = HashMap::new();
        for (name, key) in [
            ("JCIBLLQ0", SpecialLocKey::PclkIn(Dir::W, 0)),
            ("JCIBLLQ1", SpecialLocKey::PclkIn(Dir::S, 0)),
            ("JCIBURQ0", SpecialLocKey::PclkIn(Dir::E, 2)),
            ("JCIBURQ1", SpecialLocKey::PclkIn(Dir::N, 1)),
            ("JCIBURQ2", SpecialLocKey::PclkIn(Dir::E, 3)),
            ("JCIBLRQ0", SpecialLocKey::PclkIn(Dir::E, 1)),
            ("JCIBLRQ1", SpecialLocKey::PclkIn(Dir::S, 1)),
            ("JCIBLRQ2", SpecialLocKey::PclkIn(Dir::E, 0)),
            ("JCIBULQ0", SpecialLocKey::PclkIn(Dir::W, 1)),
            ("JCIBULQ1", SpecialLocKey::PclkIn(Dir::N, 0)),
            ("JCIBL0", SpecialLocKey::SclkIn(Dir::W, 0)),
            ("JCIBL1", SpecialLocKey::SclkIn(Dir::W, 1)),
            ("JCIBL2", SpecialLocKey::SclkIn(Dir::W, 2)),
            ("JCIBL3", SpecialLocKey::SclkIn(Dir::W, 3)),
            ("JCIBR0", SpecialLocKey::SclkIn(Dir::E, 0)),
            ("JCIBR1", SpecialLocKey::SclkIn(Dir::E, 1)),
            ("JCIBR2", SpecialLocKey::SclkIn(Dir::E, 2)),
            ("JCIBR3", SpecialLocKey::SclkIn(Dir::E, 3)),
            ("JCIBB0", SpecialLocKey::SclkIn(Dir::S, 0)),
            ("JCIBB1", SpecialLocKey::SclkIn(Dir::S, 1)),
            ("JCIBB2", SpecialLocKey::SclkIn(Dir::S, 2)),
            ("JCIBB3", SpecialLocKey::SclkIn(Dir::S, 3)),
            ("JCIBT0", SpecialLocKey::SclkIn(Dir::N, 0)),
            ("JCIBT1", SpecialLocKey::SclkIn(Dir::N, 1)),
            ("JCIBT2", SpecialLocKey::SclkIn(Dir::N, 2)),
            ("JCIBT3", SpecialLocKey::SclkIn(Dir::N, 3)),
        ] {
            if let Some(s) = self.naming.strings.get(name) {
                xlat.insert(s, key);
            }
        }
        for &(wf, wt) in self.grid.pips.keys() {
            let wnt = self.nodes[wt];
            let Some(&key) = xlat.get(&wnt.suffix) else {
                continue;
            };
            let wnf = self.nodes[wf];
            let cell = self.chip.xlat_rc_wire(wnf);
            self.chip.special_loc.insert(key, cell);
        }
    }

    fn fill_fabric_clock_ecp3(&mut self) {
        let mut xlat = HashMap::new();
        for (name, key) in [
            ("JPCLKCIBLLQ0", SpecialLocKey::PclkIn(Dir::W, 0)),
            ("JPCLKCIBLLQ1", SpecialLocKey::PclkIn(Dir::S, 0)),
            ("JPCLKCIBURQ0", SpecialLocKey::PclkIn(Dir::E, 2)),
            ("JPCLKCIBURQ1", SpecialLocKey::PclkIn(Dir::N, 1)),
            ("JPCLKCIBURQ2", SpecialLocKey::PclkIn(Dir::E, 3)),
            ("JPCLKCIBLRQ0", SpecialLocKey::PclkIn(Dir::E, 1)),
            ("JPCLKCIBLRQ1", SpecialLocKey::PclkIn(Dir::S, 1)),
            ("JPCLKCIBLRQ2", SpecialLocKey::PclkIn(Dir::E, 0)),
            ("JPCLKCIBULQ0", SpecialLocKey::PclkIn(Dir::W, 1)),
            ("JPCLKCIBULQ1", SpecialLocKey::PclkIn(Dir::N, 0)),
            ("JPCLKCIBMID0", SpecialLocKey::PclkInMid(0)),
            ("JPCLKCIBMID1", SpecialLocKey::PclkInMid(1)),
            ("JPCLKCIBMID2", SpecialLocKey::PclkInMid(2)),
            ("JPCLKCIBMID3", SpecialLocKey::PclkInMid(3)),
            ("JPCLKCIBMID4", SpecialLocKey::PclkInMid(4)),
            ("JPCLKCIBMID5", SpecialLocKey::PclkInMid(5)),
            ("JPCLKCIBMID6", SpecialLocKey::PclkInMid(6)),
            ("JPCLKCIBMID7", SpecialLocKey::PclkInMid(7)),
            ("JSCLKCIBL0", SpecialLocKey::SclkIn(Dir::W, 0)),
            ("JSCLKCIBL1", SpecialLocKey::SclkIn(Dir::W, 1)),
            ("JSCLKCIBL2", SpecialLocKey::SclkIn(Dir::W, 2)),
            ("JSCLKCIBL3", SpecialLocKey::SclkIn(Dir::W, 3)),
            ("JSCLKCIBR0", SpecialLocKey::SclkIn(Dir::E, 0)),
            ("JSCLKCIBR1", SpecialLocKey::SclkIn(Dir::E, 1)),
            ("JSCLKCIBR2", SpecialLocKey::SclkIn(Dir::E, 2)),
            ("JSCLKCIBR3", SpecialLocKey::SclkIn(Dir::E, 3)),
            ("JSCLKCIBB0", SpecialLocKey::SclkIn(Dir::S, 0)),
            ("JSCLKCIBB1", SpecialLocKey::SclkIn(Dir::S, 1)),
            ("JSCLKCIBB2", SpecialLocKey::SclkIn(Dir::S, 2)),
            ("JSCLKCIBB3", SpecialLocKey::SclkIn(Dir::S, 3)),
            ("JSCLKCIBT0", SpecialLocKey::SclkIn(Dir::N, 0)),
            ("JSCLKCIBT1", SpecialLocKey::SclkIn(Dir::N, 1)),
            ("JSCLKCIBT2", SpecialLocKey::SclkIn(Dir::N, 2)),
            ("JSCLKCIBT3", SpecialLocKey::SclkIn(Dir::N, 3)),
        ] {
            if let Some(s) = self.naming.strings.get(name) {
                xlat.insert(s, key);
            }
        }
        for &(wf, wt) in self.grid.pips.keys() {
            let wnt = self.nodes[wt];
            let Some(&key) = xlat.get(&wnt.suffix) else {
                continue;
            };
            let wnf = self.nodes[wf];
            let cell = self.chip.xlat_rc_wire(wnf);
            self.chip.special_loc.insert(key, cell);
        }
    }

    fn fill_fabric_clock_ecp4(&mut self) {
        let mut xlat = HashMap::new();
        for (name, key) in [
            ("JLLMPCLKCIB0", SpecialLocKey::PclkIn(Dir::W, 0)),
            ("JLLMPCLKCIB2", SpecialLocKey::PclkIn(Dir::W, 1)),
            ("JULMPCLKCIB0", SpecialLocKey::PclkIn(Dir::W, 2)),
            ("JULMPCLKCIB2", SpecialLocKey::PclkIn(Dir::W, 3)),
            ("JLRMPCLKCIB0", SpecialLocKey::PclkIn(Dir::E, 0)),
            ("JLRMPCLKCIB2", SpecialLocKey::PclkIn(Dir::E, 1)),
            ("JURMPCLKCIB0", SpecialLocKey::PclkIn(Dir::E, 2)),
            ("JURMPCLKCIB2", SpecialLocKey::PclkIn(Dir::E, 3)),
            ("JLLMPCLKCIB1", SpecialLocKey::PclkIn(Dir::S, 0)),
            ("JLLMPCLKCIB3", SpecialLocKey::PclkIn(Dir::S, 1)),
            ("JLRMPCLKCIB1", SpecialLocKey::PclkIn(Dir::S, 2)),
            ("JLRMPCLKCIB3", SpecialLocKey::PclkIn(Dir::S, 3)),
            ("JULMPCLKCIB1", SpecialLocKey::PclkIn(Dir::N, 0)),
            ("JULMPCLKCIB3", SpecialLocKey::PclkIn(Dir::N, 1)),
            ("JURMPCLKCIB1", SpecialLocKey::PclkIn(Dir::N, 2)),
            ("JURMPCLKCIB3", SpecialLocKey::PclkIn(Dir::N, 3)),
        ] {
            if let Some(s) = self.naming.strings.get(name) {
                xlat.insert(s, key);
            }
        }
        for &(wf, wt) in self.grid.pips.keys() {
            let wnt = self.nodes[wt];
            let Some(&key) = xlat.get(&wnt.suffix) else {
                continue;
            };
            let wnf = self.nodes[wf];
            let cell = self.chip.xlat_rc_wire(wnf);
            self.chip.special_loc.insert(key, cell);
        }
    }

    fn fill_fabric_clock_ecp5(&mut self) {
        let mut xlat = HashMap::new();
        for (name, key) in [
            ("JLLQPCLKCIB0", SpecialLocKey::PclkIn(Dir::W, 0)),
            ("JLLQPCLKCIB1", SpecialLocKey::PclkIn(Dir::W, 1)),
            ("JULQPCLKCIB0", SpecialLocKey::PclkIn(Dir::W, 2)),
            ("JULQPCLKCIB1", SpecialLocKey::PclkIn(Dir::W, 3)),
            ("JLLMPCLKCIB0", SpecialLocKey::PclkIn(Dir::W, 4)),
            ("JLLMPCLKCIB2", SpecialLocKey::PclkIn(Dir::W, 5)),
            ("JULMPCLKCIB0", SpecialLocKey::PclkIn(Dir::W, 6)),
            ("JULMPCLKCIB2", SpecialLocKey::PclkIn(Dir::W, 7)),
            ("JLRQPCLKCIB0", SpecialLocKey::PclkIn(Dir::E, 0)),
            ("JLRQPCLKCIB1", SpecialLocKey::PclkIn(Dir::E, 1)),
            ("JURQPCLKCIB0", SpecialLocKey::PclkIn(Dir::E, 2)),
            ("JURQPCLKCIB1", SpecialLocKey::PclkIn(Dir::E, 3)),
            ("JLRMPCLKCIB0", SpecialLocKey::PclkIn(Dir::E, 4)),
            ("JLRMPCLKCIB2", SpecialLocKey::PclkIn(Dir::E, 5)),
            ("JURMPCLKCIB0", SpecialLocKey::PclkIn(Dir::E, 6)),
            ("JURMPCLKCIB2", SpecialLocKey::PclkIn(Dir::E, 7)),
            ("JBLQPCLKCIB0", SpecialLocKey::PclkIn(Dir::S, 0)),
            ("JBLQPCLKCIB1", SpecialLocKey::PclkIn(Dir::S, 1)),
            ("JBRQPCLKCIB0", SpecialLocKey::PclkIn(Dir::S, 2)),
            ("JBRQPCLKCIB1", SpecialLocKey::PclkIn(Dir::S, 3)),
            ("JLLMPCLKCIB1", SpecialLocKey::PclkIn(Dir::S, 4)),
            ("JLLMPCLKCIB3", SpecialLocKey::PclkIn(Dir::S, 5)),
            ("JLRMPCLKCIB1", SpecialLocKey::PclkIn(Dir::S, 6)),
            ("JLRMPCLKCIB3", SpecialLocKey::PclkIn(Dir::S, 7)),
            ("JTLQPCLKCIB0", SpecialLocKey::PclkIn(Dir::N, 0)),
            ("JTLQPCLKCIB1", SpecialLocKey::PclkIn(Dir::N, 1)),
            ("JTRQPCLKCIB0", SpecialLocKey::PclkIn(Dir::N, 2)),
            ("JTRQPCLKCIB1", SpecialLocKey::PclkIn(Dir::N, 3)),
            ("JULMPCLKCIB1", SpecialLocKey::PclkIn(Dir::N, 4)),
            ("JULMPCLKCIB3", SpecialLocKey::PclkIn(Dir::N, 5)),
            ("JURMPCLKCIB1", SpecialLocKey::PclkIn(Dir::N, 6)),
            ("JURMPCLKCIB3", SpecialLocKey::PclkIn(Dir::N, 7)),
        ] {
            if let Some(s) = self.naming.strings.get(name) {
                xlat.insert(s, key);
            }
        }
        for &(wf, wt) in self.grid.pips.keys() {
            let wnt = self.nodes[wt];
            let Some(&key) = xlat.get(&wnt.suffix) else {
                continue;
            };
            let wnf = self.nodes[wf];
            let cell = self.chip.xlat_rc_wire(wnf);
            self.chip.special_loc.insert(key, cell);
        }
    }

    fn fill_special_io_machxo(&mut self) {
        for (wn, io) in self.gather_special_io() {
            let suffix = self.naming.strings[wn.suffix].as_str();
            if let Some(key) = match suffix {
                "JGCLK0" => Some(SpecialIoKey::Clock(Dir::N, 0)),
                "JGCLK1" => Some(SpecialIoKey::Clock(Dir::N, 1)),
                "JGCLK2" => Some(SpecialIoKey::Clock(Dir::S, 0)),
                "JGCLK3" => Some(SpecialIoKey::Clock(Dir::S, 1)),
                "JTSALLI_TSALL" => Some(SpecialIoKey::TsAll),
                "JGSRPADN_GSR" => Some(SpecialIoKey::Gsr),
                _ => None,
            } {
                self.chip.special_io.insert(key, io);
            } else if let Some(pad) = match suffix {
                "JCLKI3" => Some(PllPad::PllIn0),
                "JCLKFB3" => Some(PllPad::PllFb),
                _ => None,
            } {
                let cell = self.chip.xlat_rc_wire(wn);
                let h = if cell.col < self.chip.col_clk {
                    DirH::W
                } else {
                    DirH::E
                };
                let v = if cell.row < self.chip.row_clk {
                    DirV::S
                } else {
                    DirV::N
                };
                let hv = DirHV { h, v };
                let key = SpecialIoKey::Pll(pad, PllLoc::new(hv, 0));
                self.chip.special_io.insert(key, io);
            } else {
                panic!("WEIRD SPECIO: R{r}C{c}_{suffix} {io}", r = wn.r, c = wn.c,);
            }
        }
        let (col, iob) = match self.chip.rows.len() {
            10 => (4, 1),
            12 => (8, 4),
            17 => (9, 4),
            21 => (14, 2),
            _ => unreachable!(),
        };
        let col = ColId::from_idx(col);
        let iob = TileIobId::from_idx(iob);
        let io = EdgeIoCoord::S(col, iob);
        self.chip.special_io.insert(SpecialIoKey::SleepN, io);
    }

    fn fill_direct_io_machxo(&mut self) {
        let xlat = BTreeMap::from_iter(
            ['A', 'B', 'C', 'D', 'E', 'F']
                .into_iter()
                .enumerate()
                .map(|(i, l)| (self.naming.strings.get(&format!("JDD2{l}")).unwrap(), i)),
        );
        for &(wf, wt) in self.grid.pips.keys() {
            let wtn = self.nodes[wt];
            if let Some(&iob) = xlat.get(&wtn.suffix) {
                let io_cell = self.chip.xlat_rc_wire(wtn);
                let io = io_cell.bel(bels::IO[iob]);
                let io = self.chip.get_io_crd(io);
                let wfn = self.nodes[wf];
                let stage_cell = self.chip.xlat_rc_wire(wfn);
                let plc_cell = match io.edge() {
                    Dir::W => stage_cell.delta(1, 0),
                    Dir::E => stage_cell.delta(-1, 0),
                    Dir::S => stage_cell.delta(0, 1),
                    Dir::N => stage_cell.delta(0, -1),
                };
                let lut = self.naming.strings[wfn.suffix]
                    .strip_prefix("JDD")
                    .unwrap()
                    .parse()
                    .unwrap();
                self.chip.io_direct_plc.insert(io, (plc_cell, lut));
            }
        }
    }

    fn fill_config_io_ecp(&mut self) {
        for (key, dx, iob) in [
            (SpecialIoKey::WriteN, 0, 0),
            (SpecialIoKey::Cs1N, 0, 1),
            (SpecialIoKey::CsN, 1, 1),
            (SpecialIoKey::D(0), 2, 1),
            (SpecialIoKey::D(2), 3, 0),
            (SpecialIoKey::D(1), 3, 1),
            (SpecialIoKey::D(3), 4, 1),
            (SpecialIoKey::D(4), 5, 1),
            (SpecialIoKey::D(5), 6, 1),
            (SpecialIoKey::D(6), 7, 1),
        ] {
            let io = EdgeIoCoord::S(self.chip.col_clk + dx, TileIobId::from_idx(iob));
            self.chip.special_io.insert(key, io);
        }
        let row_cfg = self
            .chip
            .rows
            .iter()
            .find(|(_, rd)| rd.kind == RowKind::Ebr)
            .unwrap()
            .0;
        for (key, dy, iob) in [
            (SpecialIoKey::Di, -2, 1),
            (SpecialIoKey::Dout, -2, 0),
            (SpecialIoKey::Busy, -1, 1),
            (SpecialIoKey::D(7), -1, 0),
        ] {
            let io = EdgeIoCoord::E(row_cfg + dy, TileIobId::from_idx(iob));
            self.chip.special_io.insert(key, io);
        }
    }

    fn fill_config_io_xp(&mut self) {
        for (key, dx, iob) in [
            (SpecialIoKey::CsN, -9, 0),
            (SpecialIoKey::Di, -8, 0),
            (SpecialIoKey::WriteN, -6, 0),
            (SpecialIoKey::Dout, -5, 0),
            (SpecialIoKey::Cs1N, -1, 0),
            (SpecialIoKey::Busy, -1, 1),
            (SpecialIoKey::D(7), 0, 1),
            (SpecialIoKey::D(6), 1, 1),
            (SpecialIoKey::D(5), 2, 0),
            (SpecialIoKey::D(4), 3, 0),
            (SpecialIoKey::D(3), 6, 1),
            (SpecialIoKey::D(2), 7, 0),
            (SpecialIoKey::D(1), 8, 1),
            (SpecialIoKey::D(0), 9, 0),
        ] {
            let io = EdgeIoCoord::N(self.chip.col_clk + dx, TileIobId::from_idx(iob));
            self.chip.special_io.insert(key, io);
        }
    }

    fn fill_config_io_ecp2(&mut self) {
        if self.chip.rows[self.chip.row_s() + 2].io_e == IoGroupKind::None {
            return;
        }
        for (key, dy, iob) in [
            (SpecialIoKey::WriteN, 2, 1),
            (SpecialIoKey::Cs1N, 2, 0),
            (SpecialIoKey::CsN, 3, 1),
            (SpecialIoKey::D(0), 3, 0),
            (SpecialIoKey::D(1), 4, 1),
            (SpecialIoKey::D(2), 4, 0),
            (SpecialIoKey::D(3), 5, 1),
            (SpecialIoKey::D(4), 5, 0),
            (SpecialIoKey::D(5), 6, 1),
            (SpecialIoKey::D(6), 6, 0),
            (SpecialIoKey::D(7), 7, 1),
            (SpecialIoKey::Di, 7, 0),
            (SpecialIoKey::Dout, 8, 1),
            (SpecialIoKey::Busy, 8, 0),
        ] {
            let io = EdgeIoCoord::E(self.chip.row_s() + dy, TileIobId::from_idx(iob));
            self.chip.special_io.insert(key, io);
        }
    }

    fn fill_config_io_xp2(&mut self) {
        for (key, dy, iob) in [
            (SpecialIoKey::InitB, 2, 1),
            (SpecialIoKey::D(0), 2, 0),
            (SpecialIoKey::D(1), 3, 1),
            (SpecialIoKey::Cclk, 3, 0),
            (SpecialIoKey::SpiCCsB, 5, 1),
            (SpecialIoKey::SpiPCsB, 5, 0),
            (SpecialIoKey::M1, 6, 0),
            (SpecialIoKey::Done, 7, 1),
            (SpecialIoKey::ProgB, 7, 0),
        ] {
            let io = EdgeIoCoord::W(self.chip.row_clk + dy, TileIobId::from_idx(iob));
            self.chip.special_io.insert(key, io);
        }
    }

    fn fill_config_io_ecp3(&mut self) {
        let row_cfg = self.chip.special_loc[&SpecialLocKey::Config].row;
        for (key, dy, iob) in [
            (SpecialIoKey::Busy, 0, 3),
            (SpecialIoKey::D(7), 0, 2),
            (SpecialIoKey::D(6), 0, 1),
            (SpecialIoKey::D(5), 0, 0),
            (SpecialIoKey::D(4), 3, 3),
            (SpecialIoKey::D(3), 3, 2),
            (SpecialIoKey::D(2), 3, 1),
            (SpecialIoKey::D(1), 3, 0),
            (SpecialIoKey::D(0), 6, 3),
            (SpecialIoKey::WriteN, 6, 2),
            (SpecialIoKey::MClk, 6, 1),
            (SpecialIoKey::Dout, 6, 0),
            (SpecialIoKey::CsN, 9, 3),
            (SpecialIoKey::Cs1N, 9, 2),
            (SpecialIoKey::Di, 9, 1),
        ] {
            let io = EdgeIoCoord::E(row_cfg + dy, TileIobId::from_idx(iob));
            self.chip.special_io.insert(key, io);
        }
    }
}

fn init_chip(kind: ChipKind, naming: &ChipNaming, nodes: &EntityVec<NodeId, WireName>) -> Chip {
    let ja0 = naming.strings.get("JA0").unwrap();
    let mut max_r = 0;
    let mut max_c = 0;
    for wn in nodes.values() {
        if wn.suffix == ja0 {
            max_r = max_r.max(wn.r);
            max_c = max_c.max(wn.c);
        }
    }
    let columns = EntityVec::from_iter((0..max_c).map(|_| Column {
        io_s: IoGroupKind::None,
        io_n: IoGroupKind::None,
        bank_s: None,
        bank_n: None,
        eclk_tap_s: false,
        eclk_tap_n: false,
        pclk_break: false,
        pclk_drive: false,
        sdclk_break: false,
    }));
    let rows = EntityVec::from_iter((0..max_r).map(|_| Row {
        kind: RowKind::Io,
        io_w: IoGroupKind::None,
        io_e: IoGroupKind::None,
        bank_w: None,
        bank_e: None,
        sclk_break: false,
        pclk_break: false,
        pclk_drive: false,
    }));
    Chip {
        kind,
        columns,
        rows,
        col_clk: ColId::from_idx(0),
        row_clk: RowId::from_idx(0),
        special_loc: BTreeMap::new(),
        special_io: BTreeMap::new(),
        io_direct_plc: BTreeMap::new(),
        extra_frames_w: 0,
        extra_frames_e: 0,
        double_frames: false,
    }
}

pub fn make_chip(
    name: &str,
    grid: &Grid,
    kind: ChipKind,
    naming: &ChipNaming,
    nodes: &EntityVec<NodeId, WireName>,
) -> Chip {
    let chip = init_chip(kind, naming, nodes);
    let mut builder = ChipBuilder {
        name,
        chip,
        grid,
        naming,
        nodes,
    };
    builder.fill_ebr_dsp_rows();
    builder.fill_plc_rows();
    match builder.chip.kind {
        ChipKind::Scm => {
            builder.fill_clk_scm();
            builder.fill_pclk_scm();
            builder.fill_config_loc_scm();
            builder.fill_pll_scm();
            builder.fill_io_scm();
            builder.fill_special_io_scm();
        }
        ChipKind::Ecp => {
            builder.fill_clk_ecp();
            builder.fill_config_loc_ecp();
            builder.fill_pll_ecp();
            builder.fill_io_ecp();
            builder.fill_io_banks_8();
            builder.fill_special_io_ecp();
            builder.fill_fabric_clock_ecp();
            builder.fill_config_io_ecp();
        }
        ChipKind::Xp => {
            builder.fill_clk_ecp();
            builder.fill_config_loc_ecp();
            builder.fill_config_bits_loc_xp();
            builder.fill_frames_xp();
            builder.fill_pll_ecp();
            builder.fill_io_xp();
            builder.fill_io_banks_8();
            builder.fill_special_io_ecp();
            builder.fill_fabric_clock_ecp();
            builder.fill_config_io_xp();
        }
        ChipKind::MachXo => {
            builder.fill_clk_machxo();
            builder.fill_machxo_special_loc();
            builder.fill_io_machxo();
            builder.fill_io_banks_machxo();
            builder.fill_special_io_machxo();
            builder.fill_direct_io_machxo();
        }
        ChipKind::Ecp2 | ChipKind::Ecp2M => {
            builder.fill_clk_ecp();
            builder.fill_pclk_ecp2();
            builder.fill_sclk_ecp2();
            builder.fill_eclk_tap_ecp2();
            builder.fill_config_loc_ecp();
            builder.fill_io_ecp2();
            builder.fill_io_banks_ecp2();
            if builder.chip.kind == ChipKind::Ecp2 {
                builder.fill_pll_ecp2();
            } else {
                builder.fill_pll_ecp2m();
                builder.fill_serdes_ecp2m();
            }
            builder.fill_special_io_ecp2();
            builder.fill_fabric_clock_ecp2();
            builder.fill_config_io_ecp2();
        }
        ChipKind::Xp2 => {
            builder.fill_clk_ecp();
            builder.fill_pclk_ecp2();
            builder.fill_sclk_ecp2();
            builder.fill_eclk_tap_ecp2();
            builder.fill_config_loc_xp2();
            builder.fill_io_xp2();
            builder.fill_io_banks_8();
            builder.fill_pll_xp2();
            builder.fill_special_io_ecp2();
            builder.fill_fabric_clock_ecp2();
            builder.fill_config_io_xp2();
        }
        ChipKind::Ecp3 | ChipKind::Ecp3A => {
            builder.fill_kind_ecp3();
            builder.fill_clk_ecp();
            builder.fill_pclk_ecp3();
            builder.fill_sclk_ecp3();
            builder.fill_eclk_tap_ecp3();
            builder.fill_config_loc_ecp3();
            builder.fill_io_ecp3();
            builder.fill_io_banks_ecp3();
            builder.fill_serdes_ecp3();
            builder.fill_pll_ecp3();
            builder.fill_special_io_ecp3();
            builder.fill_fabric_clock_ecp3();
            builder.fill_config_io_ecp3();
        }
        ChipKind::MachXo2(_) => {
            builder.fill_kind_machxo2();
            builder.fill_clk_machxo2();
            builder.fill_pclk_ecp3();
            builder.fill_sclk_ecp3();
            builder.fill_ebr_machxo2();
            builder.fill_config_loc_ecp();
            builder.fill_io_machxo2();
            builder.fill_io_banks_machxo2();
            builder.fill_bc_machxo2();
            builder.fill_dqsdll_machxo2();
            builder.fill_pll_xp2();
            builder.fill_special_io_machxo2();
        }
        ChipKind::Ecp4 => {
            builder.fill_clk_ecp4();
            builder.fill_pclk_ecp4();
            builder.fill_config_loc_ecp();
            builder.fill_io_ecp4();
            builder.fill_dqs_ecp4();
            builder.fill_bc_ecp4();
            builder.fill_serdes_ecp4();
            builder.fill_special_io_ecp4();
            builder.fill_fabric_clock_ecp4();
        }
        ChipKind::Ecp5 => {
            builder.fill_clk_ecp4();
            builder.fill_pclk_ecp5();
            builder.fill_config_loc_ecp();
            builder.fill_io_ecp5();
            builder.fill_bc_ecp5();
            builder.fill_ddrdll_ecp5();
            builder.fill_serdes_ecp5();
            builder.fill_pll_xp2();
            builder.fill_special_io_ecp5();
            builder.fill_fabric_clock_ecp5();
        }
        ChipKind::Crosslink => {
            builder.fill_clk_crosslink();
            builder.fill_pclk_ecp5();
            builder.fill_special_loc_crosslink();
            builder.fill_io_crosslink();
            builder.fill_bc_crosslink();
            builder.fill_mipi_crosslink();
            builder.fill_special_io_crosslink();
        }
    };
    builder.chip
}
