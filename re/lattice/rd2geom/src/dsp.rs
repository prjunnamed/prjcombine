use prjcombine_ecp::{bels, chip::ChipKind};
use prjcombine_interconnect::db::Bel;

use crate::ChipContext;

impl ChipContext<'_> {
    pub fn process_dsp_ecp(&mut self) {
        let tcid = self.intdb.get_tile_class("DSP");
        let is_ecp2 = self.chip.kind != ChipKind::Ecp;
        for &tcrd in &self.edev.egrid.tile_index[tcid] {
            let bcrd = tcrd.bel(bels::DSP0);
            let (r, c) = self.rc(tcrd.cell);
            self.name_bel(
                bcrd,
                [
                    format!("MULT9_R{r}C{c}"),
                    format!("MULT9_R{r}C{c}", c = c + 2),
                    format!("MULT9_R{r}C{c}", c = c + 4),
                    format!("MULT9_R{r}C{c}", c = c + 6),
                    format!("MULT18_R{r}C{c}", c = c + 1),
                    format!("MULT18_R{r}C{c}", c = c + 5),
                    format!("MULT36_R{r}C{c}", c = c + 7),
                    format!("MAC52_R{r}C{c}", c = c + 3),
                ],
            );
            let cell_mult9 = [
                tcrd.cell,
                tcrd.cell.delta(2, 0),
                tcrd.cell.delta(4, 0),
                tcrd.cell.delta(6, 0),
            ];
            let cell_mult18 = [tcrd.cell.delta(1, 0), tcrd.cell.delta(5, 0)];
            let cell_mac52 = tcrd.cell.delta(3, 0);
            let cell_mult36 = tcrd.cell.delta(7, 0);
            let mut bel = Bel::default();

            for pin in [
                "CLK0", "CLK1", "CLK2", "CLK3", "CE0", "CE1", "CE2", "CE3", "RST0", "RST1", "RST2",
                "RST3",
            ] {
                let wire = self.rc_wire(cell_mac52, &format!("J{pin}_MAC52"));
                self.add_bel_wire(bcrd, format!("{pin}_MAC52"), wire);
                let bpin = self.xlat_int_wire(tcrd, wire).unwrap();

                let wire = self.rc_wire(cell_mult36, &format!("J{pin}_MULT36"));
                self.add_bel_wire(bcrd, format!("{pin}_MULT36"), wire);
                assert_eq!(bpin, self.xlat_int_wire(tcrd, wire).unwrap());

                for (i, cell) in cell_mult9.into_iter().enumerate() {
                    let wire = self.rc_wire(cell, &format!("J{pin}_MULT9"));
                    self.add_bel_wire(bcrd, format!("{pin}_MULT9_{i}"), wire);
                    assert_eq!(bpin, self.xlat_int_wire(tcrd, wire).unwrap());
                }

                for (i, cell) in cell_mult18.into_iter().enumerate() {
                    let wire = self.rc_wire(cell, &format!("J{pin}_MULT18"));
                    self.add_bel_wire(bcrd, format!("{pin}_MULT18_{i}"), wire);
                    assert_eq!(bpin, self.xlat_int_wire(tcrd, wire).unwrap());
                }

                bel.pins.insert(pin.into(), bpin);
            }

            for ab in ['A', 'B'] {
                for i in 0..4 {
                    for j in 0..18 {
                        let wire = self.rc_wire(cell_mult36, &format!("JMU{ab}{i}{j}_MULT36"));
                        self.add_bel_wire(bcrd, format!("MU{ab}{i}{j}_MULT36"), wire);
                        let bpin = self.xlat_int_wire(tcrd, wire).unwrap();

                        if i < 2 || j < 9 {
                            let wire = self.rc_wire(cell_mac52, &format!("JMU{ab}{i}{j}_MAC52"));
                            self.add_bel_wire(bcrd, format!("MU{ab}{i}{j}_MAC52"), wire);
                            assert_eq!(bpin, self.xlat_int_wire(tcrd, wire).unwrap());
                        }

                        if j < 9 {
                            let wire = self.rc_wire(cell_mult9[i], &format!("JMU{ab}0{j}_MULT9"));
                            self.add_bel_wire(bcrd, format!("MU{ab}{i}{j}_MULT9"), wire);
                            assert_eq!(bpin, self.xlat_int_wire(tcrd, wire).unwrap());
                        }

                        if i == 0 || i == 2 || j < 9 {
                            let wire = self.rc_wire(
                                cell_mult18[i / 2],
                                &format!("JMU{ab}{ii}{j}_MULT18", ii = i % 2),
                            );
                            self.add_bel_wire(bcrd, format!("MU{ab}{i}{j}_MULT18"), wire);
                            assert_eq!(bpin, self.xlat_int_wire(tcrd, wire).unwrap());
                        }

                        bel.pins.insert(format!("MU{ab}{i}{j}"), bpin);
                    }
                }
            }

            for i in 0..4 {
                for j in 0..36 {
                    let wire = self.rc_wire(cell_mult36, &format!("JMUP{i}{j}_MULT36"));
                    self.add_bel_wire(bcrd, format!("MUP{i}{j}_MULT36"), wire);
                    let bpin = self.xlat_int_wire(tcrd, wire).unwrap();

                    if i < 2 {
                        let wire = self.rc_wire(cell_mac52, &format!("JMUP{i}{j}_MAC52"));
                        self.add_bel_wire(bcrd, format!("MUP{i}{j}_MAC52"), wire);
                        assert_eq!(bpin, self.xlat_int_wire(tcrd, wire).unwrap());
                    }

                    if j < 18 {
                        let wire = self.rc_wire(cell_mult9[i], &format!("JMUP0{j}_MULT9"));
                        self.add_bel_wire(bcrd, format!("MUP{i}{j}_MULT9"), wire);
                        assert_eq!(bpin, self.xlat_int_wire(tcrd, wire).unwrap());
                    }

                    if i == 0 || i == 2 {
                        let wire = self.rc_wire(cell_mult18[i / 2], &format!("JMUP0{j}_MULT18"));
                        self.add_bel_wire(bcrd, format!("MUP{i}{j}_MULT18"), wire);
                        assert_eq!(bpin, self.xlat_int_wire(tcrd, wire).unwrap());
                    }

                    bel.pins.insert(format!("MUP{i}{j}"), bpin);
                }
            }

            for (cell, kind, name, pkind) in [
                (cell_mult18[0], "MULT18", "MULT18_0", "MULT36"),
                (cell_mac52, "MAC52", "MAC52", "MULT18"),
                (cell_mult18[1], "MULT18", "MULT18_1", "MAC52"),
                (cell_mult36, "MULT36", "MULT36", "MULT18"),
            ] {
                for ab in ['A', 'B'] {
                    for j in 0..18 {
                        let sri = self.rc_wire(cell, &format!("JSRI{ab}{j}_{kind}"));
                        let sro = self.rc_wire(cell, &format!("JSRO{ab}{j}_{kind}"));
                        self.add_bel_wire(bcrd, format!("JSRI{ab}{j}_{name}"), sri);
                        self.add_bel_wire(bcrd, format!("JSRO{ab}{j}_{name}"), sro);
                        self.claim_pip(sro, sri);
                        if cell.col != self.chip.col_w() + 2 {
                            let cell_src = if is_ecp2 && cell == cell_mult18[0] {
                                cell.delta(-3, 0)
                            } else {
                                cell.delta(-2, 0)
                            };
                            let psro = self.rc_wire(cell_src, &format!("JSRO{ab}{j}_{pkind}"));
                            self.claim_pip(sri, psro);
                        }
                        if kind == "MULT36" {
                            let bpin = self.xlat_int_wire(tcrd, sro).unwrap();
                            bel.pins.insert(format!("SRO{ab}{j}"), bpin);
                        }
                    }
                }
            }

            for i in 0..4 {
                let cell = cell_mult9[i];
                for ab in ['A', 'B'] {
                    for j in 0..9 {
                        let sri = self.rc_wire(cell, &format!("JSRI{ab}{j}_MULT9"));
                        let sro = self.rc_wire(cell, &format!("JSRO{ab}{j}_MULT9"));
                        self.add_bel_wire(bcrd, format!("JSRI{ab}{j}_MULT9_{i}"), sri);
                        self.add_bel_wire(bcrd, format!("JSRO{ab}{j}_MULT9_{i}"), sro);
                        self.claim_pip(sro, sri);
                        if cell.col != self.chip.col_w() + 1 {
                            let cell_src = if is_ecp2 && i == 0 {
                                cell.delta(-3, 0)
                            } else {
                                cell.delta(-2, 0)
                            };
                            let psro = self.rc_wire(cell_src, &format!("JSRO{ab}{j}_MULT9"));
                            self.claim_pip(sri, psro);
                        }
                        if i == 3 {
                            let bpin = self.xlat_int_wire(tcrd, sro).unwrap();
                            assert_eq!(bel.pins[&format!("SRO{ab}{j}")], bpin);
                        }
                    }
                }
            }

            let pins = if is_ecp2 {
                ["SIGNEDA", "SIGNEDB", "SOURCEA", "SOURCEB"].as_slice()
            } else {
                ["SIGNEDAB"].as_slice()
            };
            for pin in pins {
                for i in 0..4 {
                    let wire = self.rc_wire(cell_mult36, &format!("J{pin}{i}_MULT36"));
                    self.add_bel_wire(bcrd, format!("{pin}{i}_MULT36"), wire);
                    let bpin = self.xlat_int_wire(tcrd, wire).unwrap();

                    let wire = self.rc_wire(cell_mac52, &format!("J{pin}{i}_MAC52"));
                    self.add_bel_wire(bcrd, format!("{pin}{i}_MAC52"), wire);
                    assert_eq!(bpin, self.xlat_int_wire(tcrd, wire).unwrap());

                    let wire = self.rc_wire(cell_mult9[i], &format!("J{pin}0_MULT9"));
                    self.add_bel_wire(bcrd, format!("{pin}{i}_MULT9"), wire);
                    assert_eq!(bpin, self.xlat_int_wire(tcrd, wire).unwrap());

                    let wire = self.rc_wire(
                        cell_mult18[i / 2],
                        &format!("J{pin}{ii}_MULT18", ii = i % 2),
                    );
                    self.add_bel_wire(bcrd, format!("{pin}{i}_MULT18"), wire);
                    assert_eq!(bpin, self.xlat_int_wire(tcrd, wire).unwrap());

                    bel.pins.insert(format!("{pin}{i}"), bpin);
                }
            }

            let wire = self.rc_wire(cell_mac52, "JACCUMSLOAD1_MAC52");
            self.add_bel_wire(bcrd, "ACCUMSLOAD1_MAC52", wire);
            let bpin = self.xlat_int_wire(tcrd, wire).unwrap();
            bel.pins.insert("ACCUMSLOAD1".into(), bpin);

            let wire = self.rc_wire(cell_mult36, "JACCUMSLOAD3_MULT36");
            self.add_bel_wire(bcrd, "ACCUMSLOAD3_MULT36", wire);
            let bpin = self.xlat_int_wire(tcrd, wire).unwrap();
            bel.pins.insert("ACCUMSLOAD3".into(), bpin);

            for i in [1, 3] {
                let wire = self.rc_wire(cell_mult36, &format!("JADDNSUB{i}_MULT36"));
                self.add_bel_wire(bcrd, format!("ADDNSUB{i}_MULT36"), wire);
                let bpin = self.xlat_int_wire(tcrd, wire).unwrap();

                let wire = self.rc_wire(cell_mac52, &format!("JADDNSUB{i}_MAC52"));
                self.add_bel_wire(bcrd, format!("ADDNSUB{i}_MAC52"), wire);
                assert_eq!(bpin, self.xlat_int_wire(tcrd, wire).unwrap());

                let wire = self.rc_wire(cell_mult18[i / 2], "JADDNSUB1_MULT18");
                self.add_bel_wire(bcrd, format!("ADDNSUB{i}_MULT18"), wire);
                assert_eq!(bpin, self.xlat_int_wire(tcrd, wire).unwrap());

                bel.pins.insert(format!("ADDNSUB{i}"), bpin);
            }

            if is_ecp2 {
                for i in 0..16 {
                    let wire = self.rc_wire(cell_mult36, &format!("JLD{i}_MULT36"));
                    self.add_bel_wire(bcrd, format!("LD{i}_MULT36"), wire);
                    let bpin = self.xlat_int_wire(tcrd, wire).unwrap();
                    bel.pins.insert(format!("LD{i}_MULT36"), bpin);

                    let wire = self.rc_wire(cell_mac52, &format!("JLD{i}_MAC52"));
                    self.add_bel_wire(bcrd, format!("LD{i}_MAC52"), wire);
                    let bpin = self.xlat_int_wire(tcrd, wire).unwrap();
                    bel.pins.insert(format!("LD{i}_MAC52"), bpin);
                }
            }

            self.insert_bel(bcrd, bel);
        }
    }

    pub fn process_dsp(&mut self) {
        match self.chip.kind {
            ChipKind::Ecp | ChipKind::Ecp2 | ChipKind::Ecp2M => self.process_dsp_ecp(),
            ChipKind::Xp | ChipKind::MachXo => (),
        }
    }
}
