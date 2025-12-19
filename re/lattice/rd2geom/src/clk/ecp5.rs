use std::collections::{BTreeMap, btree_map};

use prjcombine_ecp::{
    bels,
    chip::{IoGroupKind, PllLoc, SpecialLocKey},
};
use prjcombine_interconnect::{
    db::{Bel, BelPin, TileWireCoord},
    dir::{Dir, DirH, DirHV, DirMap, DirV},
    grid::{CellCoord, DieId},
};
use prjcombine_re_lattice_naming::WireName;
use prjcombine_entity::EntityId;

use crate::ChipContext;

impl ChipContext<'_> {
    pub(super) fn process_pclk_ecp5(&mut self) -> BTreeMap<(DirHV, usize), WireName> {
        let mut hprx = BTreeMap::new();
        for i in 0..16 {
            let pclk = self.intdb.get_wire(&format!("PCLK{i}"));
            for (col, cd) in &self.chip.columns {
                if !cd.pclk_drive {
                    continue;
                }
                let h = if col < self.chip.col_clk {
                    DirH::W
                } else {
                    DirH::E
                };
                for (v, row_s, row_n) in [
                    (DirV::S, self.chip.row_s(), self.chip.row_clk),
                    (DirV::N, self.chip.row_clk, self.chip.row_n() + 1),
                ] {
                    let mut vptx = None;
                    for col_tgt in [col - 1, col] {
                        for row in row_s.range(row_n) {
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
                    match hprx.entry((DirHV { h, v }, i)) {
                        btree_map::Entry::Vacant(e) => {
                            e.insert(cur_hprx);
                        }
                        btree_map::Entry::Occupied(e) => {
                            assert_eq!(*e.get(), cur_hprx);
                        }
                    }
                }
            }
        }
        hprx
    }

    pub(super) fn process_clk_edge_ecp5(&mut self) {
        for (edge, col, row) in [
            (Dir::W, self.chip.col_w(), self.chip.row_clk),
            (Dir::E, self.chip.col_e(), self.chip.row_clk),
            (Dir::S, self.chip.col_clk, self.chip.row_s()),
            (Dir::N, self.chip.col_clk, self.chip.row_n()),
        ] {
            let cell_tile = CellCoord::new(DieId::from_idx(0), col, row);
            let cell = match edge {
                Dir::H(_) => cell_tile,
                Dir::V(_) => cell_tile.delta(-1, 0),
            };
            let lrbt = match edge {
                Dir::W => 'L',
                Dir::E => 'R',
                Dir::S => 'B',
                Dir::N => 'T',
            };

            {
                let num_pins = if edge == Dir::S { 4 } else { 3 };
                let bcrd = cell_tile.bel(bels::CLKTEST);
                self.name_bel(bcrd, [format!("CLKTEST_{lrbt}MID")]);
                let mut bel = Bel::default();
                for i in 0..24 {
                    let wire = self.rc_wire(cell, &format!("JTESTIN{i}_CLKTEST"));
                    self.add_bel_wire(bcrd, format!("TESTIN{i}"), wire);
                    if i < num_pins {
                        bel.pins
                            .insert(format!("TESTIN{i}"), self.xlat_int_wire(bcrd, wire));
                    }
                }
                self.insert_bel(bcrd, bel);
            }

            let bcrd = cell_tile.bel(bels::CLK_EDGE);
            self.name_bel_null(bcrd);

            let mut pll_ins = BTreeMap::new();
            let plls = match edge {
                Dir::W => [DirHV::SW, DirHV::NW].as_slice(),
                Dir::E => [DirHV::SE, DirHV::NE].as_slice(),
                Dir::N => [DirHV::NW, DirHV::NE].as_slice(),
                Dir::S => [].as_slice(),
            };
            for &hv in plls {
                let Some(&cell_pll) = self
                    .chip
                    .special_loc
                    .get(&SpecialLocKey::Pll(PllLoc::new(hv, 0)))
                else {
                    continue;
                };
                let corner = match hv {
                    DirHV::SW => "LL",
                    DirHV::SE => "LR",
                    DirHV::NW => "UL",
                    DirHV::NE => "UR",
                };
                for pin in ["CLKOP", "CLKOS", "CLKOS2", "CLKOS3"] {
                    let wire = self.rc_wire(cell, &format!("J{corner}CPLL0{pin}"));
                    self.add_bel_wire(bcrd, format!("PLL_IN_{hv}_{pin}"), wire);
                    let wire_pll = self.rc_wire(cell_pll, &format!("J{pin}_PLL"));
                    self.claim_pip(wire, wire_pll);
                    pll_ins.insert((hv, pin), wire);
                }
            }

            let mut clkdiv_ins = BTreeMap::new();
            let mut pcs_ins = BTreeMap::new();
            if edge == Dir::S {
                if !self.skip_serdes {
                    for i in 0..2 {
                        for pin in ["CDIVX", "CDIV1"] {
                            let wire = self.rc_wire(cell, &format!("PCS{pin}{i}"));
                            self.add_bel_wire(bcrd, format!("CLKDIV_IN_{i}_{pin}"), wire);
                            let wire_clkdiv = self.rc_wire(cell, &format!("{pin}_PCSCLKDIV{i}"));
                            self.claim_pip(wire, wire_clkdiv);
                            clkdiv_ins.insert((i, pin), wire);
                        }
                    }
                    for (col, cd) in &self.chip.columns {
                        if cd.io_s != IoGroupKind::Serdes {
                            continue;
                        }
                        let dual = (cd.bank_s.unwrap() - 50) as usize;
                        let abcd = ['A', 'B'][dual];
                        for ch in 0..2 {
                            for pin in ["RXCLK", "TXCLK"] {
                                let wire = self.rc_wire(cell, &format!("JPCS{abcd}{pin}{ch}"));
                                self.add_bel_wire_no_claim(
                                    bcrd,
                                    format!("PCS_IN_D{dual}CH{ch}_{pin}"),
                                    wire,
                                );
                                pcs_ins.insert((dual, ch, pin), wire);
                            }
                        }
                        let wire = self.rc_wire(cell, &format!("JSERDESREFCLK{dual}"));
                        self.add_bel_wire(bcrd, format!("PCS_IN_D{dual}_EXTREF"), wire);
                        pcs_ins.insert((dual, 0, "EXTREF"), wire);
                        let cell_pcs = cell.with_col(col);
                        let wire_pcs = self.rc_io_wire(cell_pcs, "JREFCLKO_EXTREF");
                        self.claim_pip(wire, wire_pcs);
                    }
                }
            } else if edge != Dir::N {
                for i in 0..2 {
                    let wire = self.rc_wire(cell, &format!("J{lrbt}CDIVX{i}"));
                    self.add_bel_wire(bcrd, format!("CLKDIV_IN_{i}_CDIVX"), wire);
                    let wire_clkdiv = self.rc_io_wire(cell, &format!("JCDIVX_CLKDIV{i}"));
                    self.claim_pip(wire, wire_clkdiv);
                    clkdiv_ins.insert((i, "CDIVX"), wire);
                }
            }

            let mut io_ins = BTreeMap::new();
            for i in 0..4 {
                let name = match edge {
                    Dir::N => ["00", "01", "10", "11"][i],
                    Dir::W => ["60", "61", "70", "71"][i],
                    Dir::E => ["30", "31", "20", "21"][i],
                    Dir::S => continue,
                };
                let pclk = self.rc_wire(cell, &format!("JPCLKT{name}"));
                self.add_bel_wire(bcrd, format!("IO_IN_{edge}{i}"), pclk);
                io_ins.insert(i, pclk);
                let cell_dlldel = match edge {
                    Dir::N => bcrd.cell.delta(-2 + (i as i32), 0),
                    Dir::W => bcrd.cell.delta(0, -2 + ((i ^ 1) as i32)),
                    Dir::E => bcrd.cell.delta(0, -2 + ((i ^ 1) as i32)),
                    Dir::S => unreachable!(),
                };
                let wire_dlldel = self.rc_io_wire(cell_dlldel, "JINCK");
                self.claim_pip(pclk, wire_dlldel);
            }

            let mut bel = Bel::default();
            let int_names = match edge {
                Dir::W => [
                    "JLLQPCLKCIB0",
                    "JLLQPCLKCIB1",
                    "JULQPCLKCIB0",
                    "JULQPCLKCIB1",
                    "JLLMPCLKCIB0",
                    "JLLMPCLKCIB2",
                    "JULMPCLKCIB0",
                    "JULMPCLKCIB2",
                ],
                Dir::E => [
                    "JLRQPCLKCIB0",
                    "JLRQPCLKCIB1",
                    "JURQPCLKCIB0",
                    "JURQPCLKCIB1",
                    "JLRMPCLKCIB0",
                    "JLRMPCLKCIB2",
                    "JURMPCLKCIB0",
                    "JURMPCLKCIB2",
                ],
                Dir::S => [
                    "JBLQPCLKCIB0",
                    "JBLQPCLKCIB1",
                    "JBRQPCLKCIB0",
                    "JBRQPCLKCIB1",
                    "JLLMPCLKCIB1",
                    "JLLMPCLKCIB3",
                    "JLRMPCLKCIB1",
                    "JLRMPCLKCIB3",
                ],
                Dir::N => [
                    "JTLQPCLKCIB0",
                    "JTLQPCLKCIB1",
                    "JTRQPCLKCIB0",
                    "JTRQPCLKCIB1",
                    "JULMPCLKCIB1",
                    "JULMPCLKCIB3",
                    "JURMPCLKCIB1",
                    "JURMPCLKCIB3",
                ],
            };
            let mut int_ins = BTreeMap::new();
            for (i, name) in int_names.into_iter().enumerate() {
                if !self
                    .chip
                    .special_loc
                    .contains_key(&SpecialLocKey::PclkIn(edge, i as u8))
                {
                    continue;
                }
                let wire = self.rc_wire(cell, name);
                self.add_bel_wire(bcrd, format!("INT_IN_{i}"), wire);
                int_ins.insert(i, wire);
                bel.pins
                    .insert(format!("INT_IN_{i}"), self.xlat_int_wire(bcrd, wire));
            }
            self.insert_bel(bcrd, bel);

            let mut special_ins = BTreeMap::new();
            let cell_config = self.chip.special_loc[&SpecialLocKey::Config];
            match edge {
                Dir::W => {
                    let wire = self.rc_wire(cell, "JOSC");
                    self.add_bel_wire(bcrd, "OSC_IN", wire);
                    special_ins.insert("OSC", wire);

                    let wire_osc = self.rc_wire(cell_config, "JOSC_OSC");
                    self.claim_pip(wire, wire_osc);
                }
                Dir::S => {
                    for (pin, wn, wn_config) in [
                        ("JTCK", "JJTCK", "JJTCK_JTAG"),
                        ("SEDCLKOUT", "JSEDCLKOUT", "JSEDCLKOUT_SED"),
                    ] {
                        let wire = self.rc_wire(cell, wn);
                        self.add_bel_wire(bcrd, format!("{pin}_IN"), wire);
                        special_ins.insert(pin, wire);

                        let wire_config = self.rc_wire(cell_config, wn_config);
                        self.claim_pip(wire, wire_config);
                    }
                }
                _ => (),
            }

            let num_dcc = match edge {
                Dir::H(_) => 14,
                Dir::S => 16,
                Dir::N => 12,
            };

            for i in 0..num_dcc {
                let bcrd_dcc = cell_tile.bel(bels::DCC[i]);
                self.name_bel(bcrd_dcc, [format!("DCC_{lrbt}{i}")]);
                let mut bel = Bel::default();

                let ce = self.rc_wire(cell, &format!("JCE_{lrbt}DCC{i}"));
                self.add_bel_wire(bcrd_dcc, "CE", ce);
                bel.pins
                    .insert("CE".into(), self.xlat_int_wire(bcrd_dcc, ce));

                let clki = self.rc_wire(cell, &format!("CLKI_{lrbt}DCC{i}"));
                self.add_bel_wire(bcrd_dcc, "CLKI", clki);
                let clki_in = self.claim_single_in(clki);
                self.add_bel_wire(bcrd_dcc, "CLKI_IN", clki_in);

                let ios: &[_] = match (edge, i) {
                    (Dir::W, 0) => &[0],
                    (Dir::W, 1) => &[1],
                    (Dir::W, 2) => &[2],
                    (Dir::W, 3) => &[3],
                    (Dir::W, 4) => &[3],
                    (Dir::W, 5) => &[2],
                    (Dir::W, 6) => &[1],
                    (Dir::W, 7) => &[],
                    (Dir::W, 8) => &[3],
                    (Dir::W, 9) => &[],
                    (Dir::W, 10) => &[0],
                    (Dir::W, 11) => &[2],
                    (Dir::W, 12) => &[1],
                    (Dir::W, 13) => &[0],
                    (Dir::E, 0) => &[2],
                    (Dir::E, 1) => &[3],
                    (Dir::E, 2) => &[0],
                    (Dir::E, 3) => &[1],
                    (Dir::E, 4) => &[1],
                    (Dir::E, 5) => &[0],
                    (Dir::E, 6) => &[3],
                    (Dir::E, 7) => &[],
                    (Dir::E, 8) => &[1],
                    (Dir::E, 9) => &[],
                    (Dir::E, 10) => &[2],
                    (Dir::E, 11) => &[0],
                    (Dir::E, 12) => &[3],
                    (Dir::E, 13) => &[2],
                    (Dir::N, 0) => &[1],
                    (Dir::N, 1) => &[2],
                    (Dir::N, 2) => &[0],
                    (Dir::N, 3) => &[3],
                    (Dir::N, 4) => &[1],
                    (Dir::N, 5) => &[0],
                    (Dir::N, 6) => &[3],
                    (Dir::N, 7) => &[1],
                    (Dir::N, 8) => &[3],
                    (Dir::N, 9) => &[2],
                    (Dir::N, 10) => &[0],
                    (Dir::N, 11) => &[2],
                    (Dir::S, _) => &[],
                    _ => unreachable!(),
                };
                for &j in ios {
                    if let Some(&wire) = io_ins.get(&j) {
                        self.claim_pip(clki_in, wire);
                    }
                }

                let ints: &[_] = match (edge, i) {
                    (Dir::H(_), 0) => &[0, 4],
                    (Dir::H(_), 1) => &[1, 5],
                    (Dir::H(_), 2) => &[2],
                    (Dir::H(_), 3) => &[3, 7],
                    (Dir::H(_), 4) => &[3],
                    (Dir::H(_), 5) => &[2, 6],
                    (Dir::H(_), 6) => &[1, 7],
                    (Dir::H(_), 7) => &[0, 4],
                    (Dir::H(_), 8) => &[3, 4],
                    (Dir::H(_), 9) => &[2, 5],
                    (Dir::H(_), 10) => &[1, 6],
                    (Dir::H(_), 11) => &[7],
                    (Dir::H(_), 12) => &[0, 6],
                    (Dir::H(_), 13) => &[5],
                    (Dir::N, 0) => &[3, 6],
                    (Dir::N, 1) => &[0, 7],
                    (Dir::N, 2) => &[0, 5],
                    (Dir::N, 3) => &[1, 4],
                    (Dir::N, 4) => &[3, 6],
                    (Dir::N, 5) => &[2, 5],
                    (Dir::N, 6) => &[1, 4],
                    (Dir::N, 7) => &[2, 6],
                    (Dir::N, 8) => &[0, 4],
                    (Dir::N, 9) => &[3, 7],
                    (Dir::N, 10) => &[1, 5],
                    (Dir::N, 11) => &[2, 7],
                    (Dir::S, 0) => &[0, 4],
                    (Dir::S, 1) => &[1, 5],
                    (Dir::S, 2) => &[2, 6],
                    (Dir::S, 3) => &[3, 7],
                    (Dir::S, 4) => &[3, 4],
                    (Dir::S, 5) => &[2, 5],
                    (Dir::S, 6) => &[1, 6],
                    (Dir::S, 7) => &[0, 7],
                    (Dir::S, 8) => &[3, 6],
                    (Dir::S, 9) => &[2, 5],
                    (Dir::S, 10) => &[1, 7],
                    (Dir::S, 11) => &[0, 4],
                    (Dir::S, 12) => &[0, 5],
                    (Dir::S, 13) => &[1, 6],
                    (Dir::S, 14) => &[2, 4],
                    (Dir::S, 15) => &[3, 7],
                    _ => unreachable!(),
                };
                for &j in ints {
                    if let Some(&wire) = int_ins.get(&j) {
                        self.claim_pip(clki_in, wire);
                    }
                }

                let cdivs: &[_] = match (edge, i) {
                    (Dir::H(_), 0) => &[],
                    (Dir::H(_), 1) => &[(1, "CDIVX")],
                    (Dir::H(_), 2) => &[(1, "CDIVX")],
                    (Dir::H(_), 3) => &[],
                    (Dir::H(_), 4) => &[(0, "CDIVX")],
                    (Dir::H(_), 5) => &[],
                    (Dir::H(_), 6) => &[],
                    (Dir::H(_), 7) => &[(1, "CDIVX")],
                    (Dir::H(_), 8) => &[(0, "CDIVX")],
                    (Dir::H(_), 9) => &[],
                    (Dir::H(_), 10) => &[],
                    (Dir::H(_), 11) => &[],
                    (Dir::H(_), 12) => &[],
                    (Dir::H(_), 13) => &[(0, "CDIVX")],
                    (Dir::N, _) => &[],
                    (Dir::S, 0) => &[(0, "CDIV1"), (1, "CDIVX")],
                    (Dir::S, 1) => &[(0, "CDIVX"), (1, "CDIV1")],
                    (Dir::S, 2) => &[(0, "CDIVX"), (1, "CDIV1")],
                    (Dir::S, 3) => &[(0, "CDIV1"), (1, "CDIVX")],
                    (Dir::S, 4) => &[(1, "CDIVX")],
                    (Dir::S, 5) => &[(1, "CDIV1")],
                    (Dir::S, 6) => &[(0, "CDIVX")],
                    (Dir::S, 7) => &[(0, "CDIV1")],
                    (Dir::S, 8) => &[(0, "CDIV1"), (1, "CDIVX")],
                    (Dir::S, 9) => &[(0, "CDIVX"), (1, "CDIV1")],
                    (Dir::S, 10) => &[],
                    (Dir::S, 11) => &[],
                    (Dir::S, 12) => &[(0, "CDIV1"), (1, "CDIVX")],
                    (Dir::S, 13) => &[(0, "CDIVX"), (1, "CDIV1")],
                    (Dir::S, 14) => &[(0, "CDIVX"), (1, "CDIV1")],
                    (Dir::S, 15) => &[(0, "CDIV1"), (1, "CDIVX")],
                    _ => unreachable!(),
                };
                for &key in cdivs {
                    if let Some(&wire) = clkdiv_ins.get(&key) {
                        self.claim_pip(clki_in, wire);
                    }
                }

                match edge {
                    Dir::H(h) => {
                        let plls: &[_] = [
                            &[(DirV::S, "CLKOS2"), (DirV::N, "CLKOP")][..],
                            &[(DirV::N, "CLKOS")],
                            &[(DirV::N, "CLKOS2")],
                            &[(DirV::S, "CLKOP"), (DirV::N, "CLKOS3")],
                            &[(DirV::S, "CLKOS3"), (DirV::N, "CLKOS2")],
                            &[(DirV::S, "CLKOS2")],
                            &[(DirV::S, "CLKOS"), (DirV::N, "CLKOS3")],
                            &[(DirV::S, "CLKOS3"), (DirV::N, "CLKOP")],
                            &[(DirV::S, "CLKOP"), (DirV::N, "CLKOS")],
                            &[(DirV::S, "CLKOS"), (DirV::N, "CLKOP")],
                            &[(DirV::S, "CLKOS2")],
                            &[(DirV::S, "CLKOS3"), (DirV::N, "CLKOS2")],
                            &[(DirV::S, "CLKOP"), (DirV::N, "CLKOS")],
                            &[(DirV::S, "CLKOS"), (DirV::N, "CLKOS3")],
                        ][i];
                        for &(v, pin) in plls {
                            if let Some(&wire) = pll_ins.get(&(DirHV { h, v }, pin)) {
                                self.claim_pip(clki_in, wire);
                            }
                        }
                    }
                    Dir::N => {
                        let plls: &[_] = [
                            &[(DirH::W, "CLKOP"), (DirH::E, "CLKOP")][..],
                            &[(DirH::W, "CLKOS2"), (DirH::E, "CLKOP")],
                            &[(DirH::W, "CLKOP"), (DirH::E, "CLKOS2")],
                            &[(DirH::W, "CLKOS"), (DirH::E, "CLKOS3")],
                            &[(DirH::W, "CLKOS3"), (DirH::E, "CLKOS3")],
                            &[(DirH::W, "CLKOS"), (DirH::E, "CLKOS")],
                            &[(DirH::W, "CLKOS2"), (DirH::E, "CLKOS")],
                            &[(DirH::W, "CLKOS"), (DirH::E, "CLKOS2")],
                            &[(DirH::W, "CLKOS2"), (DirH::E, "CLKOS2")],
                            &[(DirH::W, "CLKOS3"), (DirH::E, "CLKOP")],
                            &[(DirH::W, "CLKOS3"), (DirH::E, "CLKOS3")],
                            &[(DirH::W, "CLKOP"), (DirH::E, "CLKOS")],
                        ][i];
                        for &(h, pin) in plls {
                            if let Some(&wire) = pll_ins.get(&(DirHV { h, v: DirV::N }, pin)) {
                                self.claim_pip(clki_in, wire);
                            }
                        }
                    }
                    Dir::S => (),
                };

                let specials: &[_] = match (edge, i) {
                    (Dir::W, 2 | 5 | 7 | 10) => &["OSC"],
                    (Dir::S, 0 | 7 | 11 | 12) => &["SEDCLKOUT"],
                    (Dir::S, 1 | 6 | 10 | 13) => &["JTCK"],
                    _ => &[],
                };
                for &key in specials {
                    if let Some(&wire) = special_ins.get(&key) {
                        self.claim_pip(clki_in, wire);
                    }
                }

                let pcs: &[_] = match (edge, i) {
                    (Dir::S, 0) => &[(0, 0, "TXCLK"), (1, 1, "RXCLK")],
                    (Dir::S, 1) => &[(0, 1, "TXCLK"), (1, 0, "RXCLK")],
                    (Dir::S, 2) => &[(0, 1, "RXCLK"), (1, 0, "TXCLK"), (1, 0, "EXTREF")],
                    (Dir::S, 3) => &[(0, 0, "RXCLK"), (0, 0, "EXTREF"), (1, 1, "TXCLK")],
                    (Dir::S, 4) => &[(0, 0, "RXCLK"), (0, 0, "EXTREF"), (1, 1, "TXCLK")],
                    (Dir::S, 5) => &[(0, 1, "RXCLK"), (1, 0, "TXCLK"), (1, 0, "EXTREF")],
                    (Dir::S, 6) => &[(0, 1, "TXCLK"), (1, 0, "RXCLK"), (1, 0, "EXTREF")],
                    (Dir::S, 7) => &[(0, 0, "TXCLK"), (0, 0, "EXTREF"), (1, 1, "RXCLK")],
                    (Dir::S, 8) => &[(0, 0, "RXCLK"), (0, 0, "EXTREF"), (1, 1, "TXCLK")],
                    (Dir::S, 9) => &[(0, 1, "RXCLK"), (1, 0, "TXCLK"), (1, 0, "EXTREF")],
                    (Dir::S, 10) => &[(0, 1, "TXCLK"), (0, 0, "EXTREF"), (1, 0, "RXCLK")],
                    (Dir::S, 11) => &[(0, 0, "TXCLK"), (1, 1, "RXCLK"), (1, 0, "EXTREF")],
                    (Dir::S, 12) => &[(0, 0, "TXCLK"), (1, 1, "RXCLK"), (1, 0, "EXTREF")],
                    (Dir::S, 13) => &[(0, 1, "TXCLK"), (0, 0, "EXTREF"), (1, 0, "RXCLK")],
                    (Dir::S, 14) => &[(0, 1, "RXCLK"), (1, 0, "TXCLK")],
                    (Dir::S, 15) => &[(0, 0, "RXCLK"), (1, 1, "TXCLK")],
                    _ => &[],
                };
                for &key in pcs {
                    if let Some(&wire) = pcs_ins.get(&key) {
                        self.claim_pip(clki_in, wire);
                    }
                }

                let clko = self.rc_wire(cell, &format!("CLKO_{lrbt}DCC{i}"));
                self.add_bel_wire(bcrd_dcc, "CLKO", clko);
                self.claim_pip(clko, clki);
                let clko_out = self.claim_single_out(clko);
                self.add_bel_wire(bcrd_dcc, "CLKO_OUT", clko_out);

                self.insert_bel(bcrd_dcc, bel);
            }
        }
    }

    pub(super) fn process_clk_root_ecp5(&mut self, hprx: BTreeMap<(DirHV, usize), WireName>) {
        let cell_tile = self.chip.bel_clk_root().cell;
        let cell = cell_tile.delta(-1, 0);

        let cell_edge = DirMap::from_fn(|edge| match edge {
            Dir::H(edge) => cell_tile.with_col(self.chip.col_edge(edge)),
            Dir::V(edge) => cell_tile.with_row(self.chip.row_edge(edge)),
        });

        {
            let bcrd = cell_tile.bel(bels::CLKTEST);
            self.name_bel(bcrd, ["CLKTEST_CEN"]);
            let mut bel = Bel::default();
            for i in 0..24 {
                let wire = self.rc_wire(cell, &format!("JTESTIN{i}_CLKTEST"));
                self.add_bel_wire(bcrd, format!("TESTIN{i}"), wire);
                bel.pins
                    .insert(format!("TESTIN{i}"), self.xlat_int_wire(bcrd, wire));
            }
            self.insert_bel(bcrd, bel);
        }

        let mut dcc_ins = BTreeMap::new();
        for (hv, quad, slot) in [
            (DirHV::SW, "BL", bels::DCC_SW0),
            (DirHV::SE, "BR", bels::DCC_SE0),
            (DirHV::NW, "TL", bels::DCC_NW0),
            (DirHV::NE, "TR", bels::DCC_NE0),
        ] {
            let bcrd = cell_tile.bel(slot);
            self.name_bel(bcrd, [format!("DCC_{quad}")]);
            let mut bel = Bel::default();

            let ce = self.rc_wire(cell, &format!("JCE_DCC{quad}"));
            self.add_bel_wire(bcrd, "CE", ce);
            bel.pins.insert("CE".into(), self.xlat_int_wire(bcrd, ce));

            let clki = self.rc_wire(cell, &format!("JCLKI_DCC{quad}"));
            self.add_bel_wire(bcrd, "CLKI", clki);
            bel.pins
                .insert("CLKI".into(), self.xlat_int_wire(bcrd, clki));

            let clko = self.rc_wire(cell, &format!("CLKO_DCC{quad}"));
            self.add_bel_wire(bcrd, "CLKO", clko);
            self.claim_pip(clko, clki);

            let clko_out = self.claim_single_out(clko);
            self.add_bel_wire(bcrd, "CLKO_OUT", clko_out);
            dcc_ins.insert(hv, clko_out);

            self.insert_bel(bcrd, bel);
        }

        let mut dcs_ins = vec![];
        for i in 0..2 {
            let bcrd = cell_tile.bel(bels::DCS[i]);
            self.name_bel(bcrd, [format!("DCS{i}")]);
            let mut bel = Bel::default();

            for pin in ["SEL0", "SEL1", "MODESEL"] {
                let wire = self.rc_wire(cell, &format!("J{pin}_DCS{i}"));
                self.add_bel_wire(bcrd, pin, wire);
                bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
            }

            let dcsout = self.rc_wire(cell, &format!("DCSOUT_DCS{i}"));
            self.add_bel_wire(bcrd, "DCSOUT", dcsout);

            for j in 0..2 {
                let clk = self.rc_wire(cell, &format!("CLK{j}_DCS{i}"));
                self.add_bel_wire(bcrd, format!("CLK{j}"), clk);
                self.claim_pip(dcsout, clk);

                let clk_in = self.claim_single_in(clk);
                self.add_bel_wire(bcrd, format!("CLK{j}_IN"), clk_in);

                for (edge, num_dcc) in [(Dir::W, 14), (Dir::E, 14), (Dir::S, 16), (Dir::N, 12)] {
                    for j in 0..num_dcc {
                        let bcrd_dcc = cell_edge[edge].bel(bels::DCC[j]);
                        let wire_dcc = self.naming.bel_wire(bcrd_dcc, "CLKO_OUT");
                        self.claim_pip(clk_in, wire_dcc);
                    }
                }

                for hv in DirHV::DIRS {
                    self.claim_pip(clk_in, dcc_ins[&hv]);
                }
            }

            let dcsout_out = self.claim_single_out(dcsout);
            self.add_bel_wire(bcrd, "DCSOUT_OUT", dcsout_out);
            dcs_ins.push(dcsout_out);

            self.insert_bel(bcrd, bel);
        }

        let bcrd = self.chip.bel_clk_root();
        self.name_bel_null(bcrd);
        let mut bel = Bel::default();

        for (tcid, hv) in [DirHV::SW, DirHV::SE, DirHV::NW, DirHV::NE]
            .into_iter()
            .enumerate()
        {
            for i in 0..16 {
                let wire = self.intdb.get_wire(&format!("PCLK{i}"));
                let wire = TileWireCoord::new_idx(tcid, wire);
                bel.pins
                    .insert(format!("PCLK{i}_{hv}"), BelPin::new_out(wire));

                let pclk_hprx = hprx[&(hv, i)];
                self.add_bel_wire(bcrd, format!("PCLK{i}_{hv}_OUT"), pclk_hprx);

                let pclk = self.claim_single_in(pclk_hprx);
                self.add_bel_wire(bcrd, format!("PCLK{i}_{hv}"), pclk);

                for (edge, num_dcc) in [(Dir::W, 14), (Dir::E, 14), (Dir::S, 16), (Dir::N, 12)] {
                    for j in 0..num_dcc {
                        let bcrd_dcc = cell_edge[edge].bel(bels::DCC[j]);
                        let wire_dcc = self.naming.bel_wire(bcrd_dcc, "CLKO_OUT");
                        self.claim_pip(pclk, wire_dcc);
                    }
                }

                for &wire in &dcs_ins {
                    self.claim_pip(pclk, wire);
                }

                for hv in DirHV::DIRS {
                    self.claim_pip(pclk, dcc_ins[&hv]);
                }
            }
        }
        for i in 0..2 {
            let brgeclk = self.rc_wire(cell, &format!("JBRGECLK{i}"));
            self.add_bel_wire(bcrd, format!("BRGECLK{i}"), brgeclk);
            let brgeclk_in = self.claim_single_in(brgeclk);
            self.add_bel_wire(bcrd, format!("BRGECLK{i}_IN"), brgeclk_in);
            let cell_edge = cell_edge[if i == 0 { Dir::E } else { Dir::W }];
            let wire_edge = self.rc_io_wire(cell_edge, &format!("JECLKO_BRGECLKSYNC{i}"));
            self.claim_pip(brgeclk_in, wire_edge);
        }
        self.insert_bel(bcrd, bel);
    }
}
