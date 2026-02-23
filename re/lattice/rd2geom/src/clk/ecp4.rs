use std::collections::{BTreeMap, btree_map};

use prjcombine_ecp::{
    bels,
    chip::{Chip, IoGroupKind, RowKind, SpecialLocKey},
};
use prjcombine_interconnect::{
    db::{BelPin, LegacyBel, TileWireCoord},
    dir::{Dir, DirH, DirHV, DirMap, DirV},
    grid::DieIdExt,
};
use prjcombine_re_lattice_naming::WireName;

use crate::ChipContext;

impl ChipContext<'_> {
    pub(super) fn process_pclk_ecp4(&mut self) -> BTreeMap<(DirHV, usize), WireName> {
        let mut hprx = BTreeMap::new();
        let mut roots = BTreeMap::new();
        for i in 0..20 {
            let pclk = self.intdb.get_wire(&format!("PCLK{i}"));
            for (row_src, rd) in &self.chip.rows {
                if rd.kind != RowKind::Ebr {
                    continue;
                }
                let v;
                let row_mid;
                if row_src < self.chip.row_clk {
                    v = DirV::S;
                    row_mid = row_src + 1;
                } else {
                    v = DirV::N;
                    row_mid = row_src;
                }
                assert!(self.chip.rows[row_mid].pclk_break);
                let mut row_s = row_mid - 1;
                let mut row_n = row_mid + 1;
                while row_s != self.chip.row_s() && !self.chip.rows[row_s].pclk_break {
                    row_s -= 1;
                }
                while row_n != (self.chip.row_n() + 1) && !self.chip.rows[row_n].pclk_break {
                    row_n += 1;
                }
                for cell_src in self.edev.row(Chip::DIE, row_src) {
                    for (sn, range) in [('S', row_s.range(row_mid)), ('N', row_mid.range(row_n))] {
                        let mut vptx = None;
                        for row in range {
                            let cell = cell_src.with_row(row);
                            let pclk = self.naming.interconnect[&cell.wire(pclk)];
                            let cur_vptx = self.claim_single_in(pclk);
                            if vptx.is_none() {
                                vptx = Some(cur_vptx);
                            } else {
                                assert_eq!(vptx, Some(cur_vptx));
                            }
                        }
                        let vptx = vptx.unwrap();
                        self.add_bel_wire(cell_src.bel(bels::INT), format!("PCLK{i}_{sn}"), vptx);
                        let cur_hprx = self.claim_single_in(vptx);
                        let h = if cell_src.col < self.chip.col_clk {
                            DirH::W
                        } else {
                            DirH::E
                        };
                        self.add_bel_wire_no_claim(
                            cell_src.bel(bels::INT),
                            format!("PCLK{i}_IN"),
                            vptx,
                        );
                        match hprx.entry((row_src, i, h)) {
                            btree_map::Entry::Vacant(e) => {
                                self.claim_node(cur_hprx);
                                e.insert(cur_hprx);
                                let hv = DirHV { h, v };
                                let root = self.claim_single_in(cur_hprx);
                                match roots.entry((hv, i)) {
                                    btree_map::Entry::Vacant(e) => {
                                        e.insert(root);
                                    }
                                    btree_map::Entry::Occupied(e) => {
                                        assert_eq!(*e.get(), root);
                                    }
                                }
                            }
                            btree_map::Entry::Occupied(e) => {
                                assert_eq!(*e.get(), cur_hprx);
                            }
                        }
                    }
                }
            }
        }
        roots
    }

    pub(super) fn process_clk_edge_ecp4(&mut self) {
        for (edge, col, row) in [
            (Dir::W, self.chip.col_w(), self.chip.row_clk),
            (Dir::E, self.chip.col_e(), self.chip.row_clk),
            (Dir::S, self.chip.col_clk, self.chip.row_s()),
            (Dir::N, self.chip.col_clk, self.chip.row_n()),
        ] {
            let cell_tile = Chip::DIE.cell(col, row);
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
                let bcrd = cell_tile.bel(bels::CLKTEST);
                self.name_bel(bcrd, [format!("CLKTEST_{lrbt}MID")]);
                let mut bel = LegacyBel::default();
                for i in 0..24 {
                    let wire = self.rc_wire(cell, &format!("JTESTIN{i}_CLKTEST"));
                    self.add_bel_wire(bcrd, format!("TESTIN{i}"), wire);
                    if i < 4 {
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
                let corner = match hv {
                    DirHV::SW => "LL",
                    DirHV::SE => "LR",
                    DirHV::NW => "UL",
                    DirHV::NE => "UR",
                };
                for i in 0..2 {
                    for pin in ["CLKOP", "CLKOS", "CLKOS2", "CLKOS3"] {
                        let wire = self.rc_wire(cell, &format!("J{corner}CPLL{i}{pin}"));
                        self.add_bel_wire(bcrd, format!("PLL_IN_{hv}{i}_{pin}"), wire);
                        let cell_pll = cell.with_cr(
                            self.chip.col_edge(hv.h),
                            match hv.v {
                                DirV::S => self.chip.row_s() + (2 - i),
                                DirV::N => self.chip.row_n() - (1 + i),
                            },
                        );
                        let wire_pll = self.rc_wire(cell_pll, &format!("J{pin}_PLL"));
                        self.claim_pip(wire, wire_pll);
                        pll_ins.insert((hv, i, pin), wire);
                    }
                }
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

            let mut clkdiv_ins = BTreeMap::new();
            let mut pcs_ins = BTreeMap::new();
            if edge == Dir::S {
                for i in 0..4 {
                    for pin in ["CDIVX", "CDIV1"] {
                        let wire = self.rc_wire(cell, &format!("PCS{pin}{i}"));
                        self.add_bel_wire(bcrd, format!("CLKDIV_IN_{i}_{pin}"), wire);
                        let wire_clkdiv = self.rc_wire(cell, &format!("{pin}_PCSCLKDIV{i}"));
                        self.claim_pip(wire, wire_clkdiv);
                        clkdiv_ins.insert((i, pin), wire);
                    }
                }
                for quad in 0..num_quads {
                    let abcd = ['A', 'B', 'C', 'D'][quad];
                    for ch in 0..4 {
                        for pin in ["RXCLK", "TXCLK"] {
                            let wire = self.rc_wire(cell, &format!("JPCS{abcd}{pin}{ch}"));
                            self.add_bel_wire_no_claim(
                                bcrd,
                                format!("PCS_IN_Q{quad}CH{ch}_{pin}"),
                                wire,
                            );
                            pcs_ins.insert((quad, ch, pin), wire);
                        }
                    }
                }
            } else {
                for i in 0..4 {
                    let wire = self.rc_wire(cell, &format!("J{lrbt}CDIVX{i}"));
                    self.add_bel_wire(bcrd, format!("CLKDIV_IN_{i}_CDIVX"), wire);
                    let wire_clkdiv = self.rc_io_wire(cell, &format!("JCDIVX_CLKDIV{i}"));
                    self.claim_pip(wire, wire_clkdiv);
                    clkdiv_ins.insert((i, "CDIVX"), wire);
                }
                if edge == Dir::E && 2 < num_quads {
                    let cell_pcs = Chip::DIE.cell(self.chip.col_w(), self.chip.row_s());
                    for ch in 0..4 {
                        for (pin, pin_asb) in [("TXCLK", "FOPCLKA"), ("RXCLK", "FOPCLKB")] {
                            let wire = self.rc_wire(cell, &format!("JPCS{pin}{ch}"));
                            self.add_bel_wire(bcrd, format!("PCS_IN_Q2CH{ch}_{pin}"), wire);
                            pcs_ins.insert((2, ch, pin), wire);
                            let wire_pcs =
                                self.rc_io_sn_wire(cell_pcs, &format!("JQ2CH{ch}_{pin_asb}_ASB"));
                            self.claim_pip(wire, wire_pcs);
                        }
                    }
                }
            }

            let mut io_ins = BTreeMap::new();
            for i in 0..8 {
                if !self.edev.has_bel(bcrd.bel(bels::DLLDEL[i])) {
                    continue;
                }
                let name = match edge {
                    Dir::N => ["00", "01", "10", "11", "20", "21", "30", "31"][i],
                    Dir::W => ["60", "61", "70", "71"][i],
                    Dir::E => ["50", "51", "40", "41"][i],
                    Dir::S => unreachable!(),
                };
                let pclk = self.rc_wire(cell, &format!("JPCLKT{name}"));
                self.add_bel_wire(bcrd, format!("IO_IN_{edge}{i}"), pclk);
                io_ins.insert(i, pclk);
                let cell_dlldel = match edge {
                    Dir::N => bcrd.cell.delta(-4 + (i as i32), 0),
                    Dir::W => bcrd.cell.delta(0, -2 + (i as i32)),
                    Dir::E => bcrd.cell.delta(0, -2 + ((i ^ 1) as i32)),
                    Dir::S => unreachable!(),
                };
                let wire_dlldel = self.rc_io_wire(cell_dlldel, "JINCK");
                self.claim_pip(pclk, wire_dlldel);
            }

            let mut bel = LegacyBel::default();
            let int_names = match edge {
                Dir::W => [
                    ("JLLQPCLKCIB0", 1),
                    ("JLLQPCLKCIB1", 1),
                    ("JULQPCLKCIB0", 1),
                    ("JULQPCLKCIB1", 1),
                    ("JLLMPCLKCIB0", 2),
                    ("JULMPCLKCIB0", 2),
                    ("JLLMPCLKCIB2", 3),
                    ("JULMPCLKCIB2", 3),
                ],
                Dir::E => [
                    ("JLRQPCLKCIB0", 1),
                    ("JLRQPCLKCIB1", 1),
                    ("JURQPCLKCIB0", 1),
                    ("JURQPCLKCIB1", 1),
                    ("JLRMPCLKCIB0", 2),
                    ("JURMPCLKCIB0", 2),
                    ("JLRMPCLKCIB2", 3),
                    ("JURMPCLKCIB2", 3),
                ],
                Dir::S => [
                    ("JBLQPCLKCIB0", 1),
                    ("JBLQPCLKCIB1", 1),
                    ("JBRQPCLKCIB0", 1),
                    ("JBRQPCLKCIB1", 1),
                    ("JLLMPCLKCIB1", 2),
                    ("JLRMPCLKCIB1", 2),
                    ("JLLMPCLKCIB3", 3),
                    ("JLRMPCLKCIB3", 3),
                ],
                Dir::N => [
                    ("JTLQPCLKCIB0", 1),
                    ("JTLQPCLKCIB1", 1),
                    ("JTRQPCLKCIB0", 1),
                    ("JTRQPCLKCIB1", 1),
                    ("JULMPCLKCIB1", 2),
                    ("JURMPCLKCIB1", 2),
                    ("JULMPCLKCIB3", 3),
                    ("JURMPCLKCIB3", 3),
                ],
            };
            let mut int_ins = BTreeMap::new();
            for (i, (name, min_quads)) in int_names.into_iter().enumerate() {
                if num_quads < min_quads {
                    continue;
                }
                let wire = self.rc_wire(cell, name);
                self.add_bel_wire(bcrd, format!("INT_IN_{i}"), wire);
                int_ins.insert(i, wire);
                bel.pins
                    .insert(format!("INT_IN_{i}"), self.xlat_int_wire(bcrd, wire));
            }
            self.insert_bel(bcrd, bel);

            let mut quads = vec![];
            match edge {
                Dir::H(edge) => {
                    let mut quads_s = vec![];
                    let mut quads_n = vec![];
                    for (row, rd) in &self.chip.rows {
                        let io = match edge {
                            DirH::W => rd.io_w,
                            DirH::E => rd.io_e,
                        };
                        let quad = match io {
                            IoGroupKind::QuadDqs => {
                                Some(cell.with_cr(self.chip.col_edge(edge), row))
                            }
                            IoGroupKind::QuadEbrDqs | IoGroupKind::EbrDqs => Some(cell.with_cr(
                                match edge {
                                    DirH::W => self.chip.col_w() + 1,
                                    DirH::E => self.chip.col_e() - 1,
                                },
                                row,
                            )),
                            _ => None,
                        };
                        if let Some(quad) = quad {
                            if row < self.chip.row_clk {
                                quads_s.push(quad);
                            } else {
                                quads_n.push(quad);
                            }
                        }
                    }
                    quads_n.reverse();
                    for (i, quad) in quads_s.into_iter().enumerate() {
                        quads.push((Dir::S, i, format!("JL{lrbt}CDR{i}"), quad));
                    }
                    for (i, quad) in quads_n.into_iter().enumerate() {
                        quads.push((Dir::N, i, format!("JU{lrbt}CDR{i}"), quad));
                    }
                }
                Dir::S => (),
                Dir::N => {
                    let mut quads_w = vec![];
                    let mut quads_e = vec![];
                    for (col, cd) in &self.chip.columns {
                        let quad = cell.with_cr(col, self.chip.row_n());
                        if cd.io_n == IoGroupKind::QuadDqs {
                            if col < self.chip.col_clk {
                                quads_w.push(quad);
                            } else {
                                quads_e.push(quad);
                            }
                        }
                    }
                    quads_e.reverse();
                    for (i, quad) in quads_w.into_iter().enumerate() {
                        quads.push((Dir::W, i, format!("JTLCDR{i}"), quad));
                    }
                    for (i, quad) in quads_e.into_iter().enumerate() {
                        quads.push((Dir::E, i, format!("JTRCDR{i}"), quad));
                    }
                }
            }
            let mut dqs_ins = BTreeMap::new();
            for (dir, idx, wn, quad) in quads {
                let wire = self.rc_wire(cell, &wn);
                self.add_bel_wire(bcrd, format!("DQS_IN_{dir}{idx}"), wire);
                dqs_ins.insert((dir, idx), wire);
                let bel_dqs = quad.bel(bels::DQS0);
                let wire_dqs = self.naming.bel_wire(bel_dqs, "DIVCLK");
                self.claim_pip(wire, wire_dqs);
            }

            let num_dcc = match edge {
                Dir::H(_) => 14,
                Dir::V(_) => 16,
            };
            for i in 0..num_dcc {
                let bcrd_dcc = cell_tile.bel(bels::DCC[i]);
                self.name_bel(bcrd_dcc, [format!("DCC_{lrbt}{i}")]);
                let mut bel = LegacyBel::default();

                let ce = self.rc_wire(cell, &format!("JCE_{lrbt}DCC{i}"));
                self.add_bel_wire(bcrd_dcc, "CE", ce);
                bel.pins
                    .insert("CE".into(), self.xlat_int_wire(bcrd_dcc, ce));

                let clki = self.rc_wire(cell, &format!("CLKI_{lrbt}DCC{i}"));
                self.add_bel_wire(bcrd_dcc, "CLKI", clki);
                let clki_in = self.claim_single_in(clki);
                self.add_bel_wire(bcrd_dcc, "CLKI_IN", clki_in);

                let dqs: &[_] = match (edge, i) {
                    (Dir::H(_), 0) => &[(Dir::S, 0)],
                    (Dir::H(_), 1) => &[(Dir::S, 1)],
                    (Dir::H(_), 2) => &[(Dir::S, 2), (Dir::N, 4)],
                    (Dir::H(_), 3) => &[(Dir::S, 3), (Dir::N, 3)],
                    (Dir::H(_), 4) => &[(Dir::S, 4), (Dir::N, 2)],
                    (Dir::H(_), 5) => &[(Dir::S, 5), (Dir::N, 0)],
                    (Dir::H(_), 6) => &[(Dir::N, 1)],
                    (Dir::H(_), 7) => &[(Dir::N, 0), (Dir::N, 3)],
                    (Dir::H(_), 8) => &[(Dir::N, 1), (Dir::N, 2)],
                    (Dir::H(_), 9) => &[(Dir::N, 4)],
                    (Dir::H(_), 10) => &[(Dir::S, 3)],
                    (Dir::H(_), 11) => &[(Dir::S, 2), (Dir::S, 4)],
                    (Dir::H(_), 12) => &[(Dir::S, 1), (Dir::S, 5)],
                    (Dir::H(_), 13) => &[(Dir::S, 0)],
                    (Dir::N, 0) => &[(Dir::W, 0)],
                    (Dir::N, 1) => &[(Dir::W, 1), (Dir::E, 6)],
                    (Dir::N, 2) => &[(Dir::W, 2), (Dir::E, 5)],
                    (Dir::N, 3) => &[(Dir::W, 3), (Dir::E, 4)],
                    (Dir::N, 4) => &[(Dir::W, 4), (Dir::E, 3)],
                    (Dir::N, 5) => &[(Dir::W, 5), (Dir::E, 2)],
                    (Dir::N, 6) => &[(Dir::W, 6), (Dir::E, 1)],
                    (Dir::N, 7) => &[(Dir::E, 0)],
                    (Dir::N, 8) => &[(Dir::E, 2)],
                    (Dir::N, 9) => &[(Dir::E, 0), (Dir::E, 1)],
                    (Dir::N, 10) => &[(Dir::E, 3), (Dir::E, 5)],
                    (Dir::N, 11) => &[(Dir::E, 4), (Dir::E, 6)],
                    (Dir::N, 12) => &[(Dir::W, 3), (Dir::W, 5)],
                    (Dir::N, 13) => &[(Dir::W, 2), (Dir::W, 4)],
                    (Dir::N, 14) => &[(Dir::W, 1)],
                    (Dir::N, 15) => &[(Dir::W, 0), (Dir::W, 6)],
                    (Dir::S, _) => &[],
                    _ => unreachable!(),
                };
                for &(dir, j) in dqs {
                    if let Some(&wire) = dqs_ins.get(&(dir, j)) {
                        self.claim_pip(clki_in, wire);
                    }
                }
                let ios: &[_] = match (edge, i) {
                    (Dir::W, 0) => &[0],
                    (Dir::W, 1) => &[1],
                    (Dir::W, 2) => &[2],
                    (Dir::W, 3) => &[3],
                    (Dir::W, 4) => &[3],
                    (Dir::W, 5) => &[2],
                    (Dir::W, 6) => &[],
                    (Dir::W, 7) => &[],
                    (Dir::W, 8) => &[1],
                    (Dir::W, 9) => &[3],
                    (Dir::W, 10) => &[0],
                    (Dir::W, 11) => &[2],
                    (Dir::W, 12) => &[0],
                    (Dir::W, 13) => &[1],
                    (Dir::E, 0) => &[2],
                    (Dir::E, 1) => &[3],
                    (Dir::E, 2) => &[0],
                    (Dir::E, 3) => &[1],
                    (Dir::E, 4) => &[1],
                    (Dir::E, 5) => &[0],
                    (Dir::E, 6) => &[],
                    (Dir::E, 7) => &[],
                    (Dir::E, 8) => &[3],
                    (Dir::E, 9) => &[1],
                    (Dir::E, 10) => &[2],
                    (Dir::E, 11) => &[0],
                    (Dir::E, 12) => &[2],
                    (Dir::E, 13) => &[3],
                    (Dir::N, 0) => &[2, 7],
                    (Dir::N, 1) => &[3],
                    (Dir::N, 2) => &[0],
                    (Dir::N, 3) => &[1, 5],
                    (Dir::N, 4) => &[0, 1],
                    (Dir::N, 5) => &[1, 4],
                    (Dir::N, 6) => &[6],
                    (Dir::N, 7) => &[3],
                    (Dir::N, 8) => &[4, 7],
                    (Dir::N, 9) => &[5, 6],
                    (Dir::N, 10) => &[2],
                    (Dir::N, 11) => &[0],
                    (Dir::N, 12) => &[2],
                    (Dir::N, 13) => &[3],
                    (Dir::N, 14) => &[4, 6],
                    (Dir::N, 15) => &[5, 7],
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
                    (Dir::H(_), 1) => &[1],
                    (Dir::H(_), 2) => &[2],
                    (Dir::H(_), 3) => &[3, 7],
                    (Dir::H(_), 4) => &[3, 6],
                    (Dir::H(_), 5) => &[2, 5],
                    (Dir::H(_), 6) => &[1, 7],
                    (Dir::H(_), 7) => &[0, 4],
                    (Dir::H(_), 8) => &[3, 4],
                    (Dir::H(_), 9) => &[2, 6],
                    (Dir::H(_), 10) => &[1, 5],
                    (Dir::H(_), 11) => &[0, 7],
                    (Dir::H(_), 12) => &[5],
                    (Dir::H(_), 13) => &[6],
                    (Dir::N, 0) => &[4],
                    (Dir::N, 1) => &[1],
                    (Dir::N, 2) => &[2, 5],
                    (Dir::N, 3) => &[7],
                    (Dir::N, 4) => &[3, 7],
                    (Dir::N, 5) => &[2, 5],
                    (Dir::N, 6) => &[1, 6],
                    (Dir::N, 7) => &[0, 3],
                    (Dir::N, 8) => &[4, 7],
                    (Dir::N, 9) => &[4],
                    (Dir::N, 10) => &[1, 6],
                    (Dir::N, 11) => &[0],
                    (Dir::N, 12) => &[0],
                    (Dir::N, 13) => &[5],
                    (Dir::N, 14) => &[2, 6],
                    (Dir::N, 15) => &[3],
                    (Dir::S, 0) => &[0, 4],
                    (Dir::S, 1) => &[1, 6],
                    (Dir::S, 2) => &[2, 5],
                    (Dir::S, 3) => &[3, 7],
                    (Dir::S, 4) => &[3, 4],
                    (Dir::S, 5) => &[2, 6],
                    (Dir::S, 6) => &[1, 5],
                    (Dir::S, 7) => &[0, 7],
                    (Dir::S, 8) => &[3, 5],
                    (Dir::S, 9) => &[2, 6],
                    (Dir::S, 10) => &[1, 7],
                    (Dir::S, 11) => &[0, 4],
                    (Dir::S, 12) => &[0, 6],
                    (Dir::S, 13) => &[1, 5],
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
                    (Dir::H(_), 0) => &[(0, "CDIVX")],
                    (Dir::H(_), 1) => &[(1, "CDIVX")],
                    (Dir::H(_), 2) => &[(2, "CDIVX")],
                    (Dir::H(_), 3) => &[(3, "CDIVX")],
                    (Dir::H(_), 4) => &[(3, "CDIVX")],
                    (Dir::H(_), 5) => &[(2, "CDIVX")],
                    (Dir::H(_), 6) => &[(1, "CDIVX")],
                    (Dir::H(_), 7) => &[(1, "CDIVX")],
                    (Dir::H(_), 8) => &[(0, "CDIVX")],
                    (Dir::H(_), 9) => &[(0, "CDIVX")],
                    (Dir::H(_), 10) => &[(2, "CDIVX")],
                    (Dir::H(_), 11) => &[(3, "CDIVX")],
                    (Dir::H(_), 12) => &[],
                    (Dir::H(_), 13) => &[],
                    (Dir::N, 0) => &[(2, "CDIVX")],
                    (Dir::N, 1) => &[(0, "CDIVX")],
                    (Dir::N, 2) => &[(2, "CDIVX")],
                    (Dir::N, 3) => &[(3, "CDIVX")],
                    (Dir::N, 4) => &[(3, "CDIVX")],
                    (Dir::N, 5) => &[(2, "CDIVX")],
                    (Dir::N, 6) => &[(1, "CDIVX")],
                    (Dir::N, 7) => &[(1, "CDIVX")],
                    (Dir::N, 8) => &[(0, "CDIVX")],
                    (Dir::N, 9) => &[(2, "CDIVX")],
                    (Dir::N, 10) => &[(0, "CDIVX")],
                    (Dir::N, 11) => &[(3, "CDIVX")],
                    (Dir::N, 12) => &[(3, "CDIVX")],
                    (Dir::N, 13) => &[(1, "CDIVX")],
                    (Dir::N, 14) => &[(1, "CDIVX")],
                    (Dir::N, 15) => &[(0, "CDIVX")],
                    (Dir::S, 0) => &[(0, "CDIV1"), (1, "CDIVX"), (3, "CDIVX")],
                    (Dir::S, 1) => &[(0, "CDIVX"), (1, "CDIV1"), (3, "CDIV1")],
                    (Dir::S, 2) => &[(0, "CDIVX"), (1, "CDIV1"), (2, "CDIVX")],
                    (Dir::S, 3) => &[(0, "CDIV1"), (1, "CDIVX"), (2, "CDIV1")],
                    (Dir::S, 4) => &[(1, "CDIVX"), (2, "CDIV1"), (3, "CDIVX")],
                    (Dir::S, 5) => &[(1, "CDIV1"), (2, "CDIVX"), (3, "CDIV1")],
                    (Dir::S, 6) => &[(0, "CDIVX"), (2, "CDIVX"), (3, "CDIV1")],
                    (Dir::S, 7) => &[(0, "CDIV1"), (2, "CDIV1"), (3, "CDIVX")],
                    (Dir::S, 8) => &[(0, "CDIV1"), (1, "CDIVX"), (2, "CDIV1")],
                    (Dir::S, 9) => &[(0, "CDIVX"), (1, "CDIV1"), (2, "CDIVX")],
                    (Dir::S, 10) => &[(2, "CDIV1"), (3, "CDIV1"), (3, "CDIVX")],
                    (Dir::S, 11) => &[(2, "CDIVX"), (3, "CDIVX"), (3, "CDIV1")],
                    (Dir::S, 12) => &[(0, "CDIV1"), (1, "CDIVX"), (2, "CDIVX")],
                    (Dir::S, 13) => &[(0, "CDIVX"), (1, "CDIV1"), (2, "CDIV1")],
                    (Dir::S, 14) => &[(0, "CDIVX"), (1, "CDIV1"), (3, "CDIVX")],
                    (Dir::S, 15) => &[(0, "CDIV1"), (1, "CDIVX"), (3, "CDIV1")],
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
                            &[
                                (DirV::S, 0, "CLKOP"),
                                (DirV::N, 1, "CLKOP"),
                                (DirV::N, 1, "CLKOS3"),
                            ][..],
                            &[
                                (DirV::S, 1, "CLKOP"),
                                (DirV::S, 0, "CLKOS"),
                                (DirV::N, 1, "CLKOS"),
                                (DirV::N, 1, "CLKOS2"),
                            ],
                            &[
                                (DirV::S, 1, "CLKOS"),
                                (DirV::N, 1, "CLKOS"),
                                (DirV::S, 0, "CLKOS2"),
                            ],
                            &[(DirV::N, 1, "CLKOP"), (DirV::S, 0, "CLKOS3")],
                            &[(DirV::S, 1, "CLKOP"), (DirV::N, 0, "CLKOS3")],
                            &[(DirV::S, 1, "CLKOS"), (DirV::N, 0, "CLKOS2")],
                            &[
                                (DirV::S, 0, "CLKOS"),
                                (DirV::N, 0, "CLKOS"),
                                (DirV::S, 1, "CLKOS2"),
                            ],
                            &[
                                (DirV::S, 0, "CLKOP"),
                                (DirV::N, 0, "CLKOP"),
                                (DirV::S, 1, "CLKOS3"),
                            ],
                            &[(DirV::N, 0, "CLKOP"), (DirV::N, 1, "CLKOS2")],
                            &[(DirV::N, 1, "CLKOS"), (DirV::N, 0, "CLKOS")],
                            &[
                                (DirV::N, 1, "CLKOP"),
                                (DirV::N, 0, "CLKOP"),
                                (DirV::N, 0, "CLKOS2"),
                            ],
                            &[
                                (DirV::S, 1, "CLKOP"),
                                (DirV::N, 0, "CLKOS"),
                                (DirV::S, 0, "CLKOS2"),
                            ],
                            &[
                                (DirV::S, 1, "CLKOS"),
                                (DirV::S, 0, "CLKOS"),
                                (DirV::S, 1, "CLKOS3"),
                                (DirV::N, 1, "CLKOS3"),
                            ],
                            &[
                                (DirV::S, 0, "CLKOP"),
                                (DirV::S, 1, "CLKOS2"),
                                (DirV::S, 0, "CLKOS3"),
                                (DirV::N, 0, "CLKOS3"),
                            ],
                        ][i];
                        for &(v, j, pin) in plls {
                            if let Some(&wire) = pll_ins.get(&(DirHV { h, v }, j, pin)) {
                                self.claim_pip(clki_in, wire);
                            }
                        }
                    }
                    Dir::N => {
                        let plls: &[_] = [
                            &[
                                (DirH::W, 0, "CLKOP"),
                                (DirH::E, 1, "CLKOP"),
                                (DirH::E, 1, "CLKOS3"),
                            ][..],
                            &[
                                (DirH::W, 1, "CLKOP"),
                                (DirH::W, 0, "CLKOS"),
                                (DirH::E, 1, "CLKOS"),
                                (DirH::E, 1, "CLKOS2"),
                            ],
                            &[
                                (DirH::W, 1, "CLKOS"),
                                (DirH::E, 1, "CLKOS"),
                                (DirH::W, 0, "CLKOS2"),
                            ],
                            &[
                                (DirH::E, 1, "CLKOP"),
                                (DirH::W, 1, "CLKOS3"),
                                (DirH::W, 0, "CLKOS3"),
                            ],
                            &[(DirH::W, 1, "CLKOP"), (DirH::E, 0, "CLKOS3")],
                            &[(DirH::W, 1, "CLKOS2"), (DirH::E, 0, "CLKOS2")],
                            &[
                                (DirH::W, 1, "CLKOS"),
                                (DirH::W, 0, "CLKOS"),
                                (DirH::E, 0, "CLKOS"),
                            ],
                            &[
                                (DirH::W, 0, "CLKOP"),
                                (DirH::E, 0, "CLKOP"),
                                (DirH::W, 1, "CLKOS3"),
                            ],
                            &[(DirH::E, 1, "CLKOS"), (DirH::E, 0, "CLKOP")],
                            &[
                                (DirH::E, 0, "CLKOS"),
                                (DirH::W, 0, "CLKOS3"),
                                (DirH::E, 1, "CLKOS2"),
                            ],
                            &[
                                (DirH::E, 1, "CLKOP"),
                                (DirH::E, 0, "CLKOS"),
                                (DirH::E, 0, "CLKOS2"),
                            ],
                            &[
                                (DirH::W, 1, "CLKOS"),
                                (DirH::E, 0, "CLKOP"),
                                (DirH::W, 0, "CLKOS2"),
                                (DirH::E, 0, "CLKOS3"),
                            ],
                            &[
                                (DirH::W, 0, "CLKOS"),
                                (DirH::W, 1, "CLKOS3"),
                                (DirH::W, 1, "CLKOS2"),
                                (DirH::E, 1, "CLKOS3"),
                            ],
                            &[
                                (DirH::W, 1, "CLKOP"),
                                (DirH::W, 0, "CLKOP"),
                                (DirH::W, 0, "CLKOS3"),
                                (DirH::E, 0, "CLKOS3"),
                            ],
                            &[(DirH::W, 1, "CLKOS2"), (DirH::W, 0, "CLKOS2")],
                            &[
                                (DirH::E, 1, "CLKOS3"),
                                (DirH::E, 1, "CLKOS2"),
                                (DirH::E, 0, "CLKOS2"),
                            ],
                        ][i];
                        for &(h, j, pin) in plls {
                            if let Some(&wire) = pll_ins.get(&(DirHV { h, v: DirV::N }, j, pin)) {
                                self.claim_pip(clki_in, wire);
                            }
                        }
                    }
                    Dir::S => (),
                };

                let pcs: &[_] = match (edge, i) {
                    (Dir::S, 0) => &[
                        (0, 0, "TXCLK"),
                        (0, 3, "RXCLK"),
                        (1, 0, "TXCLK"),
                        (1, 3, "RXCLK"),
                    ],
                    (Dir::S, 1) => &[
                        (0, 1, "TXCLK"),
                        (0, 2, "RXCLK"),
                        (1, 1, "TXCLK"),
                        (1, 2, "RXCLK"),
                    ],
                    (Dir::S, 2) => &[
                        (0, 2, "TXCLK"),
                        (0, 1, "RXCLK"),
                        (1, 2, "TXCLK"),
                        (1, 1, "RXCLK"),
                    ],
                    (Dir::S, 3) => &[
                        (0, 3, "TXCLK"),
                        (0, 0, "RXCLK"),
                        (1, 3, "TXCLK"),
                        (1, 0, "RXCLK"),
                    ],
                    (Dir::S, 4) => &[
                        (0, 3, "TXCLK"),
                        (0, 0, "RXCLK"),
                        (1, 3, "TXCLK"),
                        (1, 0, "RXCLK"),
                    ],
                    (Dir::S, 5) => &[
                        (0, 2, "TXCLK"),
                        (0, 1, "RXCLK"),
                        (1, 2, "TXCLK"),
                        (1, 1, "RXCLK"),
                    ],
                    (Dir::S, 6) => &[
                        (0, 1, "TXCLK"),
                        (0, 2, "RXCLK"),
                        (1, 1, "TXCLK"),
                        (1, 2, "RXCLK"),
                    ],
                    (Dir::S, 7) => &[
                        (0, 0, "TXCLK"),
                        (0, 3, "RXCLK"),
                        (1, 0, "TXCLK"),
                        (1, 3, "RXCLK"),
                    ],
                    (Dir::S, 8) => &[
                        (0, 3, "TXCLK"),
                        (0, 0, "RXCLK"),
                        (1, 3, "TXCLK"),
                        (1, 0, "RXCLK"),
                    ],
                    (Dir::S, 9) => &[
                        (0, 2, "TXCLK"),
                        (0, 1, "RXCLK"),
                        (1, 2, "TXCLK"),
                        (1, 1, "RXCLK"),
                    ],
                    (Dir::S, 10) => &[
                        (0, 1, "TXCLK"),
                        (0, 2, "RXCLK"),
                        (1, 1, "TXCLK"),
                        (1, 2, "RXCLK"),
                    ],
                    (Dir::S, 11) => &[
                        (0, 0, "TXCLK"),
                        (0, 3, "RXCLK"),
                        (1, 0, "TXCLK"),
                        (1, 3, "RXCLK"),
                    ],
                    (Dir::S, 12) => &[
                        (0, 0, "TXCLK"),
                        (0, 3, "RXCLK"),
                        (1, 0, "TXCLK"),
                        (1, 3, "RXCLK"),
                    ],
                    (Dir::S, 13) => &[
                        (0, 1, "TXCLK"),
                        (0, 2, "RXCLK"),
                        (1, 1, "TXCLK"),
                        (1, 2, "RXCLK"),
                    ],
                    (Dir::S, 14) => &[
                        (0, 2, "TXCLK"),
                        (0, 1, "RXCLK"),
                        (1, 2, "TXCLK"),
                        (1, 1, "RXCLK"),
                    ],
                    (Dir::S, 15) => &[
                        (0, 3, "TXCLK"),
                        (0, 0, "RXCLK"),
                        (1, 3, "TXCLK"),
                        (1, 0, "RXCLK"),
                    ],
                    (Dir::E, 0) => &[(2, 3, "RXCLK")],
                    (Dir::E, 1) => &[(2, 3, "TXCLK")],
                    (Dir::E, 2) => &[(2, 2, "TXCLK"), (2, 1, "RXCLK")],
                    (Dir::E, 3) => &[(2, 0, "RXCLK"), (2, 2, "RXCLK")],
                    (Dir::E, 4) => &[(2, 2, "TXCLK"), (2, 1, "RXCLK")],
                    (Dir::E, 5) => &[(2, 3, "TXCLK"), (2, 3, "RXCLK")],
                    (Dir::E, 6) => &[(2, 0, "TXCLK"), (2, 2, "RXCLK")],
                    (Dir::E, 7) => &[(2, 1, "TXCLK"), (2, 0, "RXCLK")],
                    (Dir::E, 8) => &[(2, 3, "TXCLK"), (2, 0, "RXCLK")],
                    (Dir::E, 9) => &[(2, 2, "TXCLK"), (2, 1, "RXCLK")],
                    (Dir::E, 10) => &[(2, 1, "TXCLK")],
                    (Dir::E, 11) => &[(2, 0, "TXCLK")],
                    (Dir::E, 12) => &[(2, 0, "TXCLK"), (2, 3, "RXCLK")],
                    (Dir::E, 13) => &[(2, 1, "TXCLK"), (2, 2, "RXCLK")],
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

    pub(super) fn process_clk_root_ecp4(&mut self, pclk_roots: BTreeMap<(DirHV, usize), WireName>) {
        let cell_tile = self.chip.bel_clk_root().cell;
        let cell = cell_tile.delta(-1, 0);

        let cell_edge = DirMap::from_fn(|edge| match edge {
            Dir::H(edge) => cell_tile.with_col(self.chip.col_edge(edge)),
            Dir::V(edge) => cell_tile.with_row(self.chip.row_edge(edge)),
        });

        {
            let bcrd = cell_tile.bel(bels::CLKTEST);
            self.name_bel(bcrd, ["CLKTEST_CEN"]);
            let mut bel = LegacyBel::default();
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
            let mut bel = LegacyBel::default();

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
            let mut bel = LegacyBel::default();

            for pin in ["SEL0", "SEL1", "SEL2", "SEL3", "MODESEL"] {
                let wire = self.rc_wire(cell, &format!("J{pin}_DCS{i}"));
                self.add_bel_wire(bcrd, pin, wire);
                bel.pins.insert(pin.into(), self.xlat_int_wire(bcrd, wire));
            }

            let dcsout = self.rc_wire(cell, &format!("DCSOUT_DCS{i}"));
            self.add_bel_wire(bcrd, "DCSOUT", dcsout);

            for j in 0..4 {
                let clk = self.rc_wire(cell, &format!("CLK{j}_DCS{i}"));
                self.add_bel_wire(bcrd, format!("CLK{j}"), clk);
                self.claim_pip(dcsout, clk);

                let clk_in = self.claim_single_in(clk);
                self.add_bel_wire(bcrd, format!("CLK{j}_IN"), clk_in);

                for (edge, num_dcc) in [(Dir::W, 14), (Dir::E, 14), (Dir::S, 16), (Dir::N, 16)] {
                    for j in 0..num_dcc {
                        let bcrd_dcc = cell_edge[edge].bel(bels::DCC[j]);
                        let wire_dcc = self.naming.bel_wire(bcrd_dcc, "CLKO_OUT");
                        self.claim_pip(clk_in, wire_dcc);
                    }
                }

                let dccs = match j {
                    0 => [DirHV::SW, DirHV::SE],
                    1 => [DirHV::NW, DirHV::SE],
                    2 => [DirHV::NW, DirHV::NE],
                    3 => [DirHV::SW, DirHV::NE],
                    _ => unreachable!(),
                };
                for hv in dccs {
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
        let mut bel = LegacyBel::default();
        for (tcid, hv) in [DirHV::SW, DirHV::SE, DirHV::NW, DirHV::NE]
            .into_iter()
            .enumerate()
        {
            for i in 0..20 {
                let wire = self.intdb.get_wire(&format!("PCLK{i}"));
                let wire = TileWireCoord::new_idx(tcid, wire);
                bel.pins
                    .insert(format!("PCLK{i}_{hv}"), BelPin::new_out(wire));

                let pclk = pclk_roots[&(hv, i)];
                self.add_bel_wire(bcrd, format!("PCLK{i}_{hv}"), pclk);

                for (edge, num_dcc) in [(Dir::W, 14), (Dir::E, 14), (Dir::S, 16), (Dir::N, 16)] {
                    for j in 0..num_dcc {
                        let bcrd_dcc = cell_edge[edge].bel(bels::DCC[j]);
                        let wire_dcc = self.naming.bel_wire(bcrd_dcc, "CLKO_OUT");
                        self.claim_pip(pclk, wire_dcc);
                    }
                }

                for &wire in &dcs_ins {
                    self.claim_pip(pclk, wire);
                }

                let dccs = match i {
                    0..5 => [DirHV::SW, DirHV::NE],
                    5..10 => [DirHV::NW, DirHV::NE],
                    10..15 => [DirHV::NW, DirHV::SE],
                    15..20 => [DirHV::SW, DirHV::SE],
                    _ => unreachable!(),
                };
                for hv in dccs {
                    self.claim_pip(pclk, dcc_ins[&hv]);
                }
            }
        }
        for i in 0..4 {
            let brgeclk = self.rc_wire(cell, &format!("JBRGECLK{i}"));
            self.add_bel_wire(bcrd, format!("BRGECLK{i}"), brgeclk);
            let brgeclk_in = self.claim_single_in(brgeclk);
            self.add_bel_wire(bcrd, format!("BRGECLK{i}_IN"), brgeclk_in);
            let cell_edge = cell_edge[if i < 2 { Dir::E } else { Dir::W }];
            let wire_edge = self.rc_io_wire(cell_edge, &format!("JECLKO_BRGECLKSYNC{i}"));
            self.claim_pip(brgeclk_in, wire_edge);
        }
        self.insert_bel(bcrd, bel);
    }
}
