use std::collections::{BTreeMap, BTreeSet, HashMap};

use prjcombine_ecp::{
    bels,
    chip::{
        Chip, ChipKind, Column, IoGroupKind, PllLoc, PllPad, Row, RowKind, SpecialIoKey,
        SpecialLocKey,
    },
};
use prjcombine_interconnect::{
    dir::{Dir, DirH, DirHV, DirV},
    grid::{CellCoord, ColId, DieId, EdgeIoCoord, RowId, TileIobId},
};
use prjcombine_re_lattice_naming::{ChipNaming, WireName};
use prjcombine_re_lattice_rawdump::{Grid, NodeId};
use unnamed_entity::{EntityId, EntityVec};

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
                "PLC" => RowKind::Plc,
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
                self.chip.columns[col_start].pclk_leaf_break = true;
            }
            next = col_start + cols.len();
        }
        assert!(self.chip.columns[self.chip.col_clk].pclk_leaf_break);
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
        let clko0b = self.naming.strings.get("CLKO0B_DCC").unwrap();
        for &wn in self.nodes.values() {
            if wn.suffix == clko0b {
                let cell = self.chip.xlat_rc_wire(wn);
                self.chip.rows[cell.row].pclk_drive = true;
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
            } else if let Some(rc) = tile.name.strip_prefix("EBR_R") {
                // ??? machxo is weird
                let r = rc.strip_suffix("C0").unwrap();
                let row = self.chip.xlat_row(r.parse().unwrap());
                self.chip.rows[row].io_w = kind;
            } else if let Some(rc) = tile.name.strip_prefix("CIB_R") {
                let (r, c) = rc.split_once('C').unwrap();
                let row = self.chip.xlat_row(r.parse().unwrap());
                let col = self.chip.xlat_col(c.parse().unwrap());
                if col.to_idx() == 0 {
                    self.chip.rows[row].io_w = kind;
                } else {
                    self.chip.rows[row].io_e = kind;
                }
            } else {
                panic!("umm weird IO tile {}", tile.name);
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

    fn fill_special_io_ecp(&mut self) {
        for (wn, io) in self.gather_special_io() {
            let suffix = self.naming.strings[wn.suffix].as_str();
            if let Some(dir) = match suffix {
                "JLPIO" => Some(Dir::W),
                "JRPIO" => Some(Dir::E),
                "JBPIO" => Some(Dir::S),
                "JTPIO" => Some(Dir::N),
                _ => None,
            } {
                self.chip.special_io.insert(SpecialIoKey::Clock(dir, 0), io);
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
                panic!("WEIRD SPECIO: R{r}C{c}_{suffix} {io}", r = wn.r, c = wn.c,);
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
            if let Some((dir, i)) = match suffix {
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
                self.chip.special_io.insert(SpecialIoKey::Clock(dir, i), io);
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
            if let Some((dir, i)) = match suffix {
                "JLPIO0" => Some((Dir::W, 0)),
                "JRPIO0" => Some((Dir::E, 0)),
                "JTPIO0" => Some((Dir::N, 0)),
                "JLPIO1" => Some((Dir::W, 1)),
                "JRPIO1" => Some((Dir::E, 1)),
                "JTPIO1" => Some((Dir::N, 1)),
                _ => None,
            } {
                self.chip.special_io.insert(SpecialIoKey::Clock(dir, i), io);
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
            (SpecialIoKey::SpiSdi, 2, 0),
            (SpecialIoKey::SpiSdo, 3, 1),
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
        pclk_leaf_break: false,
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
    };
    builder.chip
}
