use std::collections::{BTreeMap, btree_map};

use prjcombine_ecp::{
    bels,
    chip::{IoGroupKind, PllLoc, SpecialIoKey, SpecialLocKey},
};
use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{LegacyBel, BelPin, TileWireCoord},
    dir::{Dir, DirH, DirHV, DirV},
    grid::{CellCoord, DieId},
};
use prjcombine_re_lattice_naming::WireName;

use crate::ChipContext;

impl ChipContext<'_> {
    pub(super) fn process_pclk_crosslink(&mut self) -> BTreeMap<(DirH, usize), WireName> {
        let mut hprx = BTreeMap::new();
        for i in 0..8 {
            let pclk = self.intdb.get_wire(&format!("PCLK{i}"));
            let mut driven = false;
            for (col, cd) in &self.chip.columns {
                if !cd.pclk_drive {
                    if cd.pclk_break {
                        driven = false;
                    }
                    continue;
                }
                let h = if col < self.chip.col_clk {
                    DirH::W
                } else {
                    DirH::E
                };
                let mut vptx = None;
                for col_tgt in [col - 1, col] {
                    if col_tgt == col - 1 && driven {
                        continue;
                    }
                    for row in self.chip.rows.ids() {
                        let cell = CellCoord::new(DieId::from_idx(0), col_tgt, row);
                        let pclk = self.naming.interconnect[&cell.wire(pclk)];
                        let cur_vptx = self.claim_single_in(pclk);
                        if vptx.is_none() {
                            self.claim_node(cur_vptx);
                            vptx = Some(cur_vptx);
                        } else {
                            assert_eq!(vptx, Some(cur_vptx));
                        }
                        if col == col_tgt {
                            self.add_bel_wire_no_claim(
                                cell.bel(bels::INT),
                                format!("PCLK{i}_IN"),
                                cur_vptx,
                            );
                        }
                    }
                }
                let cur_hprx = self.claim_single_in(vptx.unwrap());
                match hprx.entry((h, i)) {
                    btree_map::Entry::Vacant(e) => {
                        e.insert(cur_hprx);
                    }
                    btree_map::Entry::Occupied(e) => {
                        assert_eq!(*e.get(), cur_hprx);
                    }
                }
                driven = true;
            }
        }
        hprx
    }

    pub(super) fn process_clk_edge_crosslink(&mut self) {
        for edge in [DirV::S, DirV::N] {
            let cell_tile = CellCoord::new(
                DieId::from_idx(0),
                self.chip.col_clk,
                self.chip.row_edge(edge),
            );
            let cell = cell_tile.delta(-1, 0);
            let bt = match edge {
                DirV::S => 'B',
                DirV::N => 'T',
            };

            {
                let bcrd = cell_tile.bel(bels::CLKTEST);
                self.name_bel(bcrd, [format!("CLKTEST_{bt}MID")]);
                let mut bel = LegacyBel::default();
                for i in 0..3 {
                    let wire = self.rc_wire(cell, &format!("JTESTIN{i}_CLKTEST"));
                    self.add_bel_wire(bcrd, format!("TESTIN{i}"), wire);
                    bel.pins
                        .insert(format!("TESTIN{i}"), self.xlat_int_wire(bcrd, wire));
                }
                for i in 3..6 {
                    let wire = self.rc_wire(cell, &format!("TESTIN{i}_CLKTEST"));
                    self.add_bel_wire(bcrd, format!("TESTIN{i}"), wire);
                }
                self.insert_bel(bcrd, bel);
            }

            let bcrd = cell_tile.bel(bels::CLK_EDGE);
            self.name_bel_null(bcrd);

            let mut bel = LegacyBel::default();
            let int_names = match edge {
                DirV::S => ["JPCLKCIBB0", "JPCLKCIBB1"],
                DirV::N => ["JPCLKCIBT0", "JPCLKCIBT1"],
            };
            let mut int_ins = BTreeMap::new();
            for (i, name) in int_names.into_iter().enumerate() {
                let wire = self.rc_wire(cell, name);
                self.add_bel_wire(bcrd, format!("INT_IN_{i}"), wire);
                int_ins.insert(i, wire);
                bel.pins
                    .insert(format!("INT_IN_{i}"), self.xlat_int_wire(bcrd, wire));
            }
            self.insert_bel(bcrd, bel);

            let mut io_ins = BTreeMap::new();
            match edge {
                DirV::S => {
                    for i in 0..4 {
                        let name = ["20", "21", "10", "11"][i];
                        let pclk = self.rc_wire(cell, &format!("JPCLK{name}"));
                        self.add_bel_wire(bcrd, format!("IO_IN_S{i}"), pclk);
                        io_ins.insert(i, pclk);
                        let cell_dlldel = bcrd.cell.delta(-2 + (i as i32), 0);
                        let wire_dlldel = self.rc_io_wire(cell_dlldel, "JINCK");
                        self.claim_pip(pclk, wire_dlldel);
                    }
                }
                DirV::N => {
                    for i in 4..6 {
                        let name = ["00", "01"][i - 4];
                        let pclk = self.rc_wire(cell, &format!("JPCLK{name}"));
                        self.add_bel_wire(bcrd, format!("IO_IN_S{i}"), pclk);
                        io_ins.insert(i, pclk);
                        let io = self.chip.special_io[&SpecialIoKey::Clock(Dir::S, i as u8)];
                        let (cell_io, abcd) = self.xlat_io_loc_crosslink(io);
                        let paddi_pio = self.rc_io_wire(cell_io, &format!("JPADDI{abcd}_PIO"));
                        self.claim_pip(pclk, paddi_pio);
                    }
                }
            }

            let mut pll_ins = BTreeMap::new();
            if edge == DirV::S {
                let cell_pll =
                    self.chip.special_loc[&SpecialLocKey::Pll(PllLoc::new(DirHV::SE, 0))];
                for pin in ["CLKOP", "CLKOS", "CLKOS2", "CLKOS3"] {
                    let wire = self.rc_wire(cell, &format!("J{pin}"));
                    self.add_bel_wire(bcrd, "PLL_IN_{pin}", wire);
                    pll_ins.insert(pin, wire);
                    let wire_pll = self.rc_wire(cell_pll, &format!("J{pin}_PLL"));
                    self.claim_pip(wire, wire_pll);
                }
            }

            let mut clkdiv_ins = BTreeMap::new();
            let clkdiv_range = match edge {
                DirV::S => 0..2,
                DirV::N => 2..4,
            };
            for i in clkdiv_range {
                let wire = self.rc_wire(cell, &format!("JCLKDIVX{i}"));
                self.add_bel_wire(bcrd, format!("CLKDIV_IN_{i}_CDIVX"), wire);
                let cell_clkdiv = cell.with_row(self.chip.row_s());
                let wire_clkdiv = self.rc_io_wire(cell_clkdiv, &format!("JCDIVX_CLKDIV{i}"));
                self.claim_pip(wire, wire_clkdiv);
                clkdiv_ins.insert(i, wire);
            }

            let mut osc_ins = BTreeMap::new();
            if edge == DirV::S {
                let cell_osc = self.chip.special_loc[&SpecialLocKey::Osc];
                for (pin, name) in [("LFCLKOUT", "JOSCLOW"), ("HFCLKOUT", "JOSCHIGH")] {
                    let wire = self.rc_wire(cell, name);
                    self.add_bel_wire(bcrd, format!("OSC_IN_{pin}"), wire);
                    osc_ins.insert(pin, wire);
                    let wire_osc = self.rc_wire(cell_osc, &format!("J{pin}_OSC"));
                    self.claim_pip(wire, wire_osc);
                }
            }

            let mut mipi_ins = BTreeMap::new();
            if edge == DirV::N {
                for (col, cd) in &self.chip.columns {
                    if cd.io_n != IoGroupKind::Mipi {
                        continue;
                    }
                    let cell_mipi = cell.with_col(col);
                    let idx = if col < self.chip.col_clk { 0 } else { 1 };
                    for (pin, name) in [
                        ("CLKHSBYTE", format!("JCLKHSBYTE{idx}")),
                        ("HSBYTECLKS", format!("JMIPIDPHY{idx}TX")),
                        ("HSBYTECLKD", format!("JMIPIDPHY{idx}RX")),
                    ] {
                        let wire = self.rc_wire(cell, &name);
                        self.add_bel_wire(bcrd, format!("MIPI{idx}_IN_{pin}"), wire);
                        mipi_ins.insert((idx, pin), wire);
                        let wire_mipi = self.rc_io_wire(cell_mipi, &format!("J{pin}_MIPIDPHY"));
                        self.claim_pip(wire, wire_mipi);
                    }
                }
            }

            let num_dcc = match edge {
                DirV::S => 8,
                DirV::N => 6,
            };

            for i in 0..num_dcc {
                let bcrd_dcc = cell_tile.bel(bels::DCC[i]);
                self.name_bel(bcrd_dcc, [format!("DCC_{bt}{i}")]);
                let mut bel = LegacyBel::default();

                let ce = self.rc_wire(cell, &format!("JCE_{bt}DCC{i}"));
                self.add_bel_wire(bcrd_dcc, "CE", ce);
                bel.pins
                    .insert("CE".into(), self.xlat_int_wire(bcrd_dcc, ce));

                let clki = self.rc_wire(cell, &format!("CLKI_{bt}DCC{i}"));
                self.add_bel_wire(bcrd_dcc, "CLKI", clki);
                let clki_in = self.claim_single_in(clki);
                self.add_bel_wire(bcrd_dcc, "CLKI_IN", clki_in);

                let ios: &[_] = match (edge, i) {
                    (DirV::N, 0) => &[4],
                    (DirV::N, 1) => &[5],
                    (DirV::N, 2) => &[4],
                    (DirV::N, 3) => &[5],
                    (DirV::N, 4) => &[4],
                    (DirV::N, 5) => &[5],
                    (DirV::S, 0) => &[0],
                    (DirV::S, 1) => &[1],
                    (DirV::S, 2) => &[2],
                    (DirV::S, 3) => &[0, 3],
                    (DirV::S, 4) => &[2],
                    (DirV::S, 5) => &[1, 3],
                    (DirV::S, 6) => &[1, 2],
                    (DirV::S, 7) => &[0, 3],
                    _ => unreachable!(),
                };
                for &j in ios {
                    if let Some(&wire) = io_ins.get(&j) {
                        self.claim_pip(clki_in, wire);
                    }
                }

                let ints: &[_] = match (edge, i) {
                    (DirV::N, _) => &[0, 1],
                    (DirV::S, 0) => &[0, 1],
                    (DirV::S, 1) => &[0],
                    (DirV::S, 2) => &[0, 1],
                    (DirV::S, 3) => &[0],
                    (DirV::S, 4) => &[0, 1],
                    (DirV::S, 5) => &[1],
                    (DirV::S, 6) => &[0, 1],
                    (DirV::S, 7) => &[1],
                    _ => unreachable!(),
                };
                for &j in ints {
                    if let Some(&wire) = int_ins.get(&j) {
                        self.claim_pip(clki_in, wire);
                    }
                }

                let cdivs: &[_] = match (edge, i) {
                    (DirV::N, 0) => &[2],
                    (DirV::N, 1) => &[3],
                    (DirV::N, 2) => &[2],
                    (DirV::N, 3) => &[3],
                    (DirV::N, 4) => &[2],
                    (DirV::N, 5) => &[3],
                    (DirV::S, 0) => &[0],
                    (DirV::S, 1) => &[1],
                    (DirV::S, 2) => &[0],
                    (DirV::S, 3) => &[1],
                    (DirV::S, 4) => &[],
                    (DirV::S, 5) => &[0],
                    (DirV::S, 6) => &[1],
                    (DirV::S, 7) => &[],
                    _ => unreachable!(),
                };
                for &j in cdivs {
                    if let Some(&wire) = clkdiv_ins.get(&j) {
                        self.claim_pip(clki_in, wire);
                    }
                }

                match edge {
                    DirV::S => {
                        let ins: &[_] = match i {
                            0 => &["HFCLKOUT"],
                            1 => &["HFCLKOUT", "LFCLKOUT"],
                            2 => &["LFCLKOUT"],
                            3 => &["LFCLKOUT"],
                            4 => &["HFCLKOUT", "LFCLKOUT"],
                            5 => &["HFCLKOUT"],
                            6 => &[],
                            7 => &["HFCLKOUT", "LFCLKOUT"],
                            _ => unreachable!(),
                        };
                        for &pin in ins {
                            if let Some(&wire) = osc_ins.get(&pin) {
                                self.claim_pip(clki_in, wire);
                            }
                        }
                        let ins: &[_] = match i {
                            0 => &["CLKOP", "CLKOS3"],
                            1 => &["CLKOS", "CLKOS2"],
                            2 => &["CLKOP", "CLKOS2"],
                            3 => &["CLKOS", "CLKOS3"],
                            4 => &["CLKOP", "CLKOS2"],
                            5 => &["CLKOS", "CLKOS3"],
                            6 => &["CLKOP", "CLKOS2"],
                            7 => &["CLKOS", "CLKOS3"],
                            _ => unreachable!(),
                        };
                        for &pin in ins {
                            if let Some(&wire) = pll_ins.get(&pin) {
                                self.claim_pip(clki_in, wire);
                            }
                        }
                    }
                    DirV::N => {
                        let ins: &[_] = match i {
                            0 => &[(0, "CLKHSBYTE"), (0, "HSBYTECLKS"), (1, "HSBYTECLKD")],
                            1 => &[(1, "CLKHSBYTE"), (0, "HSBYTECLKD"), (1, "HSBYTECLKS")],
                            2 => &[(0, "CLKHSBYTE"), (0, "HSBYTECLKS"), (1, "HSBYTECLKS")],
                            3 => &[(1, "CLKHSBYTE"), (0, "HSBYTECLKD"), (1, "HSBYTECLKD")],
                            4 => &[(0, "CLKHSBYTE"), (0, "HSBYTECLKS"), (1, "HSBYTECLKS")],
                            5 => &[(1, "CLKHSBYTE"), (0, "HSBYTECLKD"), (1, "HSBYTECLKD")],
                            _ => unreachable!(),
                        };
                        for &key in ins {
                            if let Some(&wire) = mipi_ins.get(&key) {
                                self.claim_pip(clki_in, wire);
                            }
                        }
                    }
                }

                let clko = self.rc_wire(cell, &format!("CLKO_{bt}DCC{i}"));
                self.add_bel_wire(bcrd_dcc, "CLKO", clko);
                self.claim_pip(clko, clki);
                let clko_out = self.claim_single_out(clko);
                self.add_bel_wire(bcrd_dcc, "CLKO_OUT", clko_out);

                self.insert_bel(bcrd_dcc, bel);
            }
        }
    }

    pub(super) fn process_clk_root_crosslink(&mut self, hprx: BTreeMap<(DirH, usize), WireName>) {
        let cell_tile = self.chip.bel_clk_root().cell;
        let cell = cell_tile.delta(-1, 0);

        {
            let bcrd = cell_tile.bel(bels::CLKTEST);
            self.name_bel(bcrd, ["CLKTEST_CEN"]);
            let mut bel = LegacyBel::default();
            for i in 0..4 {
                let wire = self.rc_wire(cell, &format!("JTESTIN{i}_CLKTEST"));
                self.add_bel_wire(bcrd, format!("TESTIN{i}"), wire);
                bel.pins
                    .insert(format!("TESTIN{i}"), self.xlat_int_wire(bcrd, wire));
            }
            for i in 4..6 {
                let wire = self.rc_wire(cell, &format!("TESTIN{i}_CLKTEST"));
                self.add_bel_wire(bcrd, format!("TESTIN{i}"), wire);
            }
            self.insert_bel(bcrd, bel);
        }

        let dcs_in;
        {
            let bcrd = cell_tile.bel(bels::DCS0);
            self.name_bel(bcrd, ["DCS"]);
            let mut bel = LegacyBel::default();

            for pin in ["SEL0", "SEL1", "MODESEL"] {
                let wire = self.rc_wire(cell, &format!("J{pin}_DCS"));
                self.add_bel_wire(bcrd, pin, wire);
                bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
            }

            let dcsout = self.rc_wire(cell, "DCSOUT_DCS");
            self.add_bel_wire(bcrd, "DCSOUT", dcsout);

            for j in 0..2 {
                let clk = self.rc_wire(cell, &format!("CLK{j}_DCS"));
                self.add_bel_wire(bcrd, format!("CLK{j}"), clk);
                self.claim_pip(dcsout, clk);

                let clk_in = self.claim_single_in(clk);
                self.add_bel_wire(bcrd, format!("CLK{j}_IN"), clk_in);

                for (edge, num_dcc) in [(DirV::S, 8), (DirV::N, 6)] {
                    let cell_edge = cell_tile.with_row(self.chip.row_edge(edge));
                    for j in 0..num_dcc {
                        let bcrd_dcc = cell_edge.bel(bels::DCC[j]);
                        let wire_dcc = self.naming.bel_wire(bcrd_dcc, "CLKO_OUT");
                        self.claim_pip(clk_in, wire_dcc);
                    }
                }
            }

            let dcsout_out = self.claim_single_out(dcsout);
            self.add_bel_wire(bcrd, "DCSOUT_OUT", dcsout_out);
            dcs_in = dcsout_out;

            self.insert_bel(bcrd, bel);
        }

        let bcrd = self.chip.bel_clk_root();
        self.name_bel_null(bcrd);
        let mut bel = LegacyBel::default();

        for i in 0..8 {
            let wire = TileWireCoord::new_idx(0, self.intdb.get_wire(&format!("PCLK{i}")));
            bel.pins.insert(format!("PCLK{i}"), BelPin::new_out(wire));

            let mut pclk = None;
            for h in [DirH::W, DirH::E] {
                let pclk_hprx = hprx[&(h, i)];
                self.add_bel_wire(bcrd, format!("PCLK{i}_{h}_OUT"), pclk_hprx);

                let cur_pclk = self.claim_single_in(pclk_hprx);
                if pclk.is_none() {
                    pclk = Some(cur_pclk);
                } else {
                    assert_eq!(pclk, Some(cur_pclk));
                }
            }
            let pclk = pclk.unwrap();
            self.add_bel_wire(bcrd, format!("PCLK{i}"), pclk);

            self.claim_pip(pclk, dcs_in);

            for (edge, num_dcc) in [(DirV::S, 8), (DirV::N, 6)] {
                let cell_edge = cell_tile.with_row(self.chip.row_edge(edge));
                for j in 0..num_dcc {
                    let bcrd_dcc = cell_edge.bel(bels::DCC[j]);
                    let wire_dcc = self.naming.bel_wire(bcrd_dcc, "CLKO_OUT");
                    self.claim_pip(pclk, wire_dcc);
                }
            }
        }

        self.insert_bel(bcrd, bel);
    }
}
