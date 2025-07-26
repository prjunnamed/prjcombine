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
                let bpin = self.xlat_int_wire(bcrd, wire);

                let wire = self.rc_wire(cell_mult36, &format!("J{pin}_MULT36"));
                self.add_bel_wire(bcrd, format!("{pin}_MULT36"), wire);
                assert_eq!(bpin, self.xlat_int_wire(bcrd, wire));

                for (i, cell) in cell_mult9.into_iter().enumerate() {
                    let wire = self.rc_wire(cell, &format!("J{pin}_MULT9"));
                    self.add_bel_wire(bcrd, format!("{pin}_MULT9_{i}"), wire);
                    assert_eq!(bpin, self.xlat_int_wire(bcrd, wire));
                }

                for (i, cell) in cell_mult18.into_iter().enumerate() {
                    let wire = self.rc_wire(cell, &format!("J{pin}_MULT18"));
                    self.add_bel_wire(bcrd, format!("{pin}_MULT18_{i}"), wire);
                    assert_eq!(bpin, self.xlat_int_wire(bcrd, wire));
                }

                bel.pins.insert(pin.into(), bpin);
            }

            for ab in ['A', 'B'] {
                for i in 0..4 {
                    for j in 0..18 {
                        let wire = self.rc_wire(cell_mult36, &format!("JMU{ab}{i}{j}_MULT36"));
                        self.add_bel_wire(bcrd, format!("MU{ab}{i}{j}_MULT36"), wire);
                        let bpin = self.xlat_int_wire(bcrd, wire);

                        if i < 2 || j < 9 {
                            let wire = self.rc_wire(cell_mac52, &format!("JMU{ab}{i}{j}_MAC52"));
                            self.add_bel_wire(bcrd, format!("MU{ab}{i}{j}_MAC52"), wire);
                            assert_eq!(bpin, self.xlat_int_wire(bcrd, wire));
                        }

                        if j < 9 {
                            let wire = self.rc_wire(cell_mult9[i], &format!("JMU{ab}0{j}_MULT9"));
                            self.add_bel_wire(bcrd, format!("MU{ab}{i}{j}_MULT9"), wire);
                            assert_eq!(bpin, self.xlat_int_wire(bcrd, wire));
                        }

                        if i == 0 || i == 2 || j < 9 {
                            let wire = self.rc_wire(
                                cell_mult18[i / 2],
                                &format!("JMU{ab}{ii}{j}_MULT18", ii = i % 2),
                            );
                            self.add_bel_wire(bcrd, format!("MU{ab}{i}{j}_MULT18"), wire);
                            assert_eq!(bpin, self.xlat_int_wire(bcrd, wire));
                        }

                        bel.pins.insert(format!("MU{ab}{i}{j}"), bpin);
                    }
                }
            }

            for i in 0..4 {
                for j in 0..36 {
                    let wire = self.rc_wire(cell_mult36, &format!("JMUP{i}{j}_MULT36"));
                    self.add_bel_wire(bcrd, format!("MUP{i}{j}_MULT36"), wire);
                    let bpin = self.xlat_int_wire(bcrd, wire);

                    if i < 2 {
                        let wire = self.rc_wire(cell_mac52, &format!("JMUP{i}{j}_MAC52"));
                        self.add_bel_wire(bcrd, format!("MUP{i}{j}_MAC52"), wire);
                        assert_eq!(bpin, self.xlat_int_wire(bcrd, wire));
                    }

                    if j < 18 {
                        let wire = self.rc_wire(cell_mult9[i], &format!("JMUP0{j}_MULT9"));
                        self.add_bel_wire(bcrd, format!("MUP{i}{j}_MULT9"), wire);
                        assert_eq!(bpin, self.xlat_int_wire(bcrd, wire));
                    }

                    if i == 0 || i == 2 {
                        let wire = self.rc_wire(cell_mult18[i / 2], &format!("JMUP0{j}_MULT18"));
                        self.add_bel_wire(bcrd, format!("MUP{i}{j}_MULT18"), wire);
                        assert_eq!(bpin, self.xlat_int_wire(bcrd, wire));
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
                            let bpin = self.xlat_int_wire(bcrd, sro);
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
                            let bpin = self.xlat_int_wire(bcrd, sro);
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
                    let bpin = self.xlat_int_wire(bcrd, wire);

                    let wire = self.rc_wire(cell_mac52, &format!("J{pin}{i}_MAC52"));
                    self.add_bel_wire(bcrd, format!("{pin}{i}_MAC52"), wire);
                    assert_eq!(bpin, self.xlat_int_wire(bcrd, wire));

                    let wire = self.rc_wire(cell_mult9[i], &format!("J{pin}0_MULT9"));
                    self.add_bel_wire(bcrd, format!("{pin}{i}_MULT9"), wire);
                    assert_eq!(bpin, self.xlat_int_wire(bcrd, wire));

                    let wire = self.rc_wire(
                        cell_mult18[i / 2],
                        &format!("J{pin}{ii}_MULT18", ii = i % 2),
                    );
                    self.add_bel_wire(bcrd, format!("{pin}{i}_MULT18"), wire);
                    assert_eq!(bpin, self.xlat_int_wire(bcrd, wire));

                    bel.pins.insert(format!("{pin}{i}"), bpin);
                }
            }

            let wire = self.rc_wire(cell_mac52, "JACCUMSLOAD1_MAC52");
            self.add_bel_wire(bcrd, "ACCUMSLOAD1_MAC52", wire);
            let bpin = self.xlat_int_wire(bcrd, wire);
            bel.pins.insert("ACCUMSLOAD1".into(), bpin);

            let wire = self.rc_wire(cell_mult36, "JACCUMSLOAD3_MULT36");
            self.add_bel_wire(bcrd, "ACCUMSLOAD3_MULT36", wire);
            let bpin = self.xlat_int_wire(bcrd, wire);
            bel.pins.insert("ACCUMSLOAD3".into(), bpin);

            for i in [1, 3] {
                let wire = self.rc_wire(cell_mult36, &format!("JADDNSUB{i}_MULT36"));
                self.add_bel_wire(bcrd, format!("ADDNSUB{i}_MULT36"), wire);
                let bpin = self.xlat_int_wire(bcrd, wire);

                let wire = self.rc_wire(cell_mac52, &format!("JADDNSUB{i}_MAC52"));
                self.add_bel_wire(bcrd, format!("ADDNSUB{i}_MAC52"), wire);
                assert_eq!(bpin, self.xlat_int_wire(bcrd, wire));

                let wire = self.rc_wire(cell_mult18[i / 2], "JADDNSUB1_MULT18");
                self.add_bel_wire(bcrd, format!("ADDNSUB{i}_MULT18"), wire);
                assert_eq!(bpin, self.xlat_int_wire(bcrd, wire));

                bel.pins.insert(format!("ADDNSUB{i}"), bpin);
            }

            if is_ecp2 {
                for i in 0..16 {
                    let wire = self.rc_wire(cell_mult36, &format!("JLD{i}_MULT36"));
                    self.add_bel_wire(bcrd, format!("LD{i}_MULT36"), wire);
                    let bpin = self.xlat_int_wire(bcrd, wire);
                    bel.pins.insert(format!("LD{i}_MULT36"), bpin);

                    let wire = self.rc_wire(cell_mac52, &format!("JLD{i}_MAC52"));
                    self.add_bel_wire(bcrd, format!("LD{i}_MAC52"), wire);
                    let bpin = self.xlat_int_wire(bcrd, wire);
                    bel.pins.insert(format!("LD{i}_MAC52"), bpin);
                }
            }

            self.insert_bel(bcrd, bel);
        }
    }

    pub fn process_dsp_ecp3(&mut self) {
        let tcid = self.intdb.get_tile_class("DSP");
        for &tcrd in &self.edev.egrid.tile_index[tcid] {
            for idx in 0..2 {
                let bcrd = tcrd.bel(bels::DSP[idx]);
                let (r, c) = self.rc(tcrd.cell);
                let c = c + (idx as u8) * 4;
                self.name_bel(
                    bcrd,
                    [
                        format!("MULT9_R{r}C{c}"),
                        format!("MULT9_R{r}C{c}", c = c + 1),
                        format!("MULT18_R{r}C{c}"),
                        format!("MULT18_R{r}C{c}", c = c + 1),
                        format!("ALU24_R{r}C{c}", c = c + 2),
                        format!("ALU54_R{r}C{c}", c = c + 3),
                    ],
                );
                let cell_base = tcrd.cell.delta((idx as i32) * 4, 0);
                let cell_mult = [cell_base, cell_base.delta(1, 0)];
                let cell_alu24 = cell_base.delta(2, 0);
                let cell_alu54 = cell_base.delta(3, 0);
                let mut bel = Bel::default();

                for pin in [
                    "CLK0", "CLK1", "CLK2", "CLK3", "CE0", "CE1", "CE2", "CE3", "RST0", "RST1",
                    "RST2", "RST3",
                ] {
                    let wire = self.rc_wire(cell_alu54, &format!("J{pin}_ALU54"));
                    self.add_bel_wire(bcrd, format!("{pin}_ALU54"), wire);
                    let bpin = self.xlat_int_wire(bcrd, wire);

                    let wire = self.rc_wire(cell_alu24, &format!("J{pin}_ALU24"));
                    self.add_bel_wire(bcrd, format!("{pin}_ALU24"), wire);
                    assert_eq!(bpin, self.xlat_int_wire(bcrd, wire));

                    for (i, cell) in cell_mult.into_iter().enumerate() {
                        let wire = self.rc_wire(cell, &format!("J{pin}_MULT9"));
                        self.add_bel_wire(bcrd, format!("{pin}_MULT9_{i}"), wire);
                        assert_eq!(bpin, self.xlat_int_wire(bcrd, wire));

                        let wire = self.rc_wire(cell, &format!("J{pin}_MULT18"));
                        self.add_bel_wire(bcrd, format!("{pin}_MULT18_{i}"), wire);
                        assert_eq!(bpin, self.xlat_int_wire(bcrd, wire));
                    }

                    bel.pins.insert(pin.into(), bpin);
                }

                for i in 0..11 {
                    let wire = self.rc_wire(cell_alu54, &format!("JOP{i}_ALU54"));
                    self.add_bel_wire(bcrd, format!("OP{i}_ALU54"), wire);
                    let bpin = self.xlat_int_wire(bcrd, wire);

                    if matches!(i, 5 | 7) {
                        let wire = self.rc_wire(cell_alu24, &format!("JOP{i}_ALU24"));
                        self.add_bel_wire(bcrd, format!("OP{i}_ALU24"), wire);
                        assert_eq!(bpin, self.xlat_int_wire(bcrd, wire));
                    }

                    bel.pins.insert(format!("OP{i}"), bpin);
                }

                for pin in ["SIGNED", "SOURCE"] {
                    for i in 0..2 {
                        for ab in ['A', 'B'] {
                            let wire_mult18 =
                                self.rc_wire(cell_mult[i], &format!("J{pin}{ab}_MULT18"));
                            self.add_bel_wire(bcrd, format!("{pin}{ab}_MULT18_{i}"), wire_mult18);
                            let bpin = self.xlat_int_wire(bcrd, wire_mult18);

                            let wire_mult9 =
                                self.rc_wire(cell_mult[i], &format!("J{pin}{ab}_MULT9"));
                            self.add_bel_wire(bcrd, format!("{pin}{ab}_MULT9_{i}"), wire_mult9);
                            assert_eq!(bpin, self.xlat_int_wire(bcrd, wire_mult9));

                            bel.pins.insert(format!("{pin}{ab}_{i}"), bpin);
                        }
                    }
                }

                for i in 0..2 {
                    let ab = ['A', 'B'][i];

                    let wire_alu54 = self.rc_wire(cell_alu54, &format!("JSIGNEDI{ab}_ALU54"));
                    self.add_bel_wire(bcrd, format!("SIGNEDI{ab}_ALU54"), wire_alu54);
                    let wire_alu24 = self.rc_wire(cell_alu24, &format!("JSIGNEDI{ab}_ALU24"));
                    self.add_bel_wire(bcrd, format!("SIGNEDI{ab}_ALU24"), wire_alu24);

                    let wire_mult9 = self.rc_wire(cell_mult[i], "JSIGNEDP_MULT9");
                    self.add_bel_wire(bcrd, format!("SIGNEDP_MULT9_{i}"), wire_mult9);
                    let wire_mult18 = self.rc_wire(cell_mult[i], "JSIGNEDP_MULT18");
                    self.add_bel_wire(bcrd, format!("SIGNEDP_MULT18_{i}"), wire_mult18);

                    self.claim_pip(wire_alu54, wire_mult9);
                    self.claim_pip(wire_alu54, wire_mult18);
                    self.claim_pip(wire_alu24, wire_mult9);
                }

                for mult_idx in 0..2 {
                    let mult_ab = ['A', 'B'][mult_idx];
                    for inp_idx in 0..2 {
                        let ab = ['A', 'B'][inp_idx];
                        for i in 0..18 {
                            let alu_pin = i + inp_idx * 18;
                            let wire_alu54 =
                                self.rc_wire(cell_alu54, &format!("J{mult_ab}{alu_pin}_ALU54"));
                            self.add_bel_wire(
                                bcrd,
                                format!("{mult_ab}{alu_pin}_ALU54"),
                                wire_alu54,
                            );

                            let wire_mult18 =
                                self.rc_wire(cell_mult[mult_idx], &format!("J{ab}{i}_MULT18"));
                            self.add_bel_wire(
                                bcrd,
                                format!("{ab}{i}_MULT18_{mult_idx}"),
                                wire_mult18,
                            );
                            let bpin = self.xlat_int_wire(bcrd, wire_mult18);

                            let wire_mult18_ro =
                                self.rc_wire(cell_mult[mult_idx], &format!("JRO{ab}{i}_MULT18"));
                            self.add_bel_wire(
                                bcrd,
                                format!("RO{ab}{i}_MULT18_{mult_idx}"),
                                wire_mult18_ro,
                            );
                            self.claim_pip(wire_mult18_ro, wire_mult18);
                            self.claim_pip(wire_alu54, wire_mult18_ro);

                            if i < 9 {
                                let wire_mult9 =
                                    self.rc_wire(cell_mult[mult_idx], &format!("J{ab}{i}_MULT9"));
                                self.add_bel_wire(
                                    bcrd,
                                    format!("{ab}{i}_MULT9_{mult_idx}"),
                                    wire_mult9,
                                );
                                assert_eq!(bpin, self.xlat_int_wire(bcrd, wire_mult9));

                                let wire_mult9_ro =
                                    self.rc_wire(cell_mult[mult_idx], &format!("JRO{ab}{i}_MULT9"));
                                self.add_bel_wire(
                                    bcrd,
                                    format!("RO{ab}{i}_MULT9_{mult_idx}"),
                                    wire_mult9_ro,
                                );
                                self.claim_pip(wire_mult9_ro, wire_mult9);
                                self.claim_pip(wire_alu54, wire_mult9_ro);
                            }

                            bel.pins.insert(format!("{ab}{i}_{mult_idx}"), bpin);
                        }
                    }
                }

                for i in 0..54 {
                    let wire = self.rc_wire(cell_alu54, &format!("JC{i}_ALU54"));
                    self.add_bel_wire(bcrd, format!("C{i}_ALU54"), wire);
                    let bpin = self.xlat_int_wire(bcrd, wire);

                    bel.pins.insert(format!("C{i}"), bpin);
                }

                for i in 0..72 {
                    let mult_idx = i / 36;
                    let mult_ab = ['A', 'B'][mult_idx];
                    let mult_pin = i % 36;

                    let wire_mult18 =
                        self.rc_wire(cell_mult[mult_idx], &format!("JP{mult_pin}_MULT18"));
                    self.add_bel_wire(bcrd, format!("P{mult_pin}_MULT18_{mult_idx}"), wire_mult18);
                    let bpin = self.xlat_int_wire(bcrd, wire_mult18);

                    let wire_alu54 =
                        self.rc_wire(cell_alu54, &format!("JM{mult_ab}{mult_pin}_ALU54"));
                    self.add_bel_wire(bcrd, format!("M{mult_ab}{mult_pin}_ALU54"), wire_alu54);
                    self.claim_pip(wire_alu54, wire_mult18);

                    if mult_pin < 18 {
                        let wire_mult9 =
                            self.rc_wire(cell_mult[mult_idx], &format!("JP{mult_pin}_MULT9"));
                        self.add_bel_wire(
                            bcrd,
                            format!("P{mult_pin}_MULT9_{mult_idx}"),
                            wire_mult9,
                        );
                        assert_eq!(bpin, self.xlat_int_wire(bcrd, wire_mult9));

                        self.claim_pip(wire_alu54, wire_mult9);

                        let wire_alu24 =
                            self.rc_wire(cell_alu24, &format!("JM{mult_ab}{mult_pin}_ALU24"));
                        self.add_bel_wire(bcrd, format!("M{mult_ab}{mult_pin}_ALU24"), wire_alu24);
                        self.claim_pip(wire_alu24, wire_mult9);
                    }

                    if i < 54 {
                        let wire_r_alu54 = self.rc_wire(cell_alu54, &format!("JR{i}_ALU54"));
                        self.add_bel_wire(bcrd, format!("R{i}_ALU54"), wire_r_alu54);
                        assert_eq!(bpin, self.xlat_int_wire(bcrd, wire_r_alu54));
                    }
                    if i < 24 {
                        let wire_r_alu24 = self.rc_wire(cell_alu24, &format!("JR{i}_ALU24"));
                        self.add_bel_wire(bcrd, format!("R{i}_ALU24"), wire_r_alu24);
                        assert_eq!(bpin, self.xlat_int_wire(bcrd, wire_r_alu24));
                    }

                    bel.pins.insert(format!("R{i}"), bpin);
                }

                for pin in [
                    "OVERUNDER",
                    "UNDER",
                    "OVER",
                    "EQZ",
                    "EQZM",
                    "EQOM",
                    "EQPAT",
                    "EQPATB",
                ] {
                    let wire = self.rc_wire(cell_alu54, &format!("J{pin}_ALU54"));
                    self.add_bel_wire(bcrd, format!("{pin}_ALU54"), wire);
                    let bpin = self.xlat_int_wire(bcrd, wire);

                    bel.pins.insert(pin.into(), bpin);
                }

                for mult_idx in 0..2 {
                    let cell_src = if mult_idx == 1 {
                        Some(cell_mult[0])
                    } else if idx == 1 {
                        Some(bcrd.cell.delta(1, 0))
                    } else if bcrd.cell.col != self.chip.col_w() + 1 {
                        Some(bcrd.cell.delta(-4, 0))
                    } else {
                        None
                    };
                    for ab in ['A', 'B'] {
                        for i in 0..18 {
                            let wire_sri_mult18 =
                                self.rc_wire(cell_mult[mult_idx], &format!("JSRI{ab}{i}_MULT18"));
                            self.add_bel_wire(
                                bcrd,
                                format!("SRI{ab}{i}_MULT18_{mult_idx}"),
                                wire_sri_mult18,
                            );
                            let wire_sro_mult18 =
                                self.rc_wire(cell_mult[mult_idx], &format!("JSRO{ab}{i}_MULT18"));
                            self.add_bel_wire(
                                bcrd,
                                format!("SRO{ab}{i}_MULT18_{mult_idx}"),
                                wire_sro_mult18,
                            );
                            self.claim_pip(wire_sro_mult18, wire_sri_mult18);

                            if let Some(cell_src) = cell_src {
                                let wire_src =
                                    self.rc_wire(cell_src, &format!("JSRO{ab}{i}_MULT18"));
                                self.claim_pip(wire_sri_mult18, wire_src);
                            }
                            if i < 9 {
                                let wire_sri_mult9 = self
                                    .rc_wire(cell_mult[mult_idx], &format!("JSRI{ab}{i}_MULT9"));
                                self.add_bel_wire(
                                    bcrd,
                                    format!("SRI{ab}{i}_MULT9_{mult_idx}"),
                                    wire_sri_mult9,
                                );
                                let wire_sro_mult9 = self
                                    .rc_wire(cell_mult[mult_idx], &format!("JSRO{ab}{i}_MULT9"));
                                self.add_bel_wire(
                                    bcrd,
                                    format!("SRO{ab}{i}_MULT9_{mult_idx}"),
                                    wire_sro_mult9,
                                );
                                self.claim_pip(wire_sro_mult9, wire_sri_mult9);

                                if let Some(cell_src) = cell_src {
                                    let wire_src_mult9 =
                                        self.rc_wire(cell_src, &format!("JSRO{ab}{i}_MULT9"));
                                    let wire_src_mult18 =
                                        self.rc_wire(cell_src, &format!("JSRO{ab}{i}_MULT18"));
                                    self.claim_pip(wire_sri_mult9, wire_src_mult9);
                                    self.claim_pip(wire_sri_mult18, wire_src_mult9);
                                    self.claim_pip(wire_sri_mult9, wire_src_mult18);
                                }
                            }

                            if mult_idx == 1 && ab == 'A' {
                                let bpin = self.xlat_int_wire(bcrd, wire_sro_mult18);
                                assert_eq!(bel.pins[&format!("R{ii}", ii = 54 + i)], bpin);
                            }
                        }
                    }
                }

                for i in 0..54 {
                    let wire = self.rc_wire(cell_alu54, &format!("JCIN{i}_ALU54"));
                    self.add_bel_wire(bcrd, format!("CIN{i}_ALU54"), wire);
                    if cell_base.col != self.chip.col_w() + 1 {
                        let cell_src = cell_alu54.delta([-5, -4][idx], 0);
                        let wire_src = self.rc_wire(cell_src, &format!("JR{i}_ALU54"));
                        self.claim_pip(wire, wire_src);
                    }
                }
                for i in 0..24 {
                    let wire = self.rc_wire(cell_alu24, &format!("JCIN{i}_ALU24"));
                    self.add_bel_wire(bcrd, format!("CIN{i}_ALU24"), wire);
                    if cell_base.col != self.chip.col_w() + 1 {
                        let cell_src = cell_alu24.delta([-5, -4][idx], 0);
                        let wire_src = self.rc_wire(cell_src, &format!("JR{i}_ALU24"));
                        self.claim_pip(wire, wire_src);
                    }
                }
                let wire = self.rc_wire(cell_alu54, "JSIGNEDR_ALU54");
                self.add_bel_wire(bcrd, "SIGNEDR_ALU54", wire);
                let wire = self.rc_wire(cell_alu54, "JSIGNEDCIN_ALU54");
                self.add_bel_wire(bcrd, "SIGNEDCIN_ALU54", wire);
                if cell_base.col != self.chip.col_w() + 1 {
                    let cell_src = cell_alu54.delta([-5, -4][idx], 0);
                    let wire_src = self.rc_wire(cell_src, "JSIGNEDR_ALU54");
                    self.claim_pip(wire, wire_src);
                }

                self.insert_bel(bcrd, bel);
            }
        }
    }

    pub fn process_dsp(&mut self) {
        match self.chip.kind {
            ChipKind::Ecp | ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => {
                self.process_dsp_ecp()
            }
            ChipKind::Xp | ChipKind::MachXo => (),
            ChipKind::Ecp3 | ChipKind::Ecp3A => self.process_dsp_ecp3(),
        }
    }
}
