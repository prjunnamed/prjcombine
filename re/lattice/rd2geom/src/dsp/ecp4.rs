use prjcombine_ecp::bels;
use prjcombine_interconnect::db::LegacyBel;

use crate::ChipContext;

impl ChipContext<'_> {
    pub(super) fn process_dsp_ecp4(&mut self) {
        let tcid = self.intdb.get_tile_class("DSP");
        for &tcrd in &self.edev.tile_index[tcid] {
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
                        format!("PRADD9_R{r}C{c}"),
                        format!("PRADD9_R{r}C{c}", c = c + 1),
                        format!("PRADD18_R{r}C{c}"),
                        format!("PRADD18_R{r}C{c}", c = c + 1),
                        format!("ALU24_R{r}C{c}", c = c + 2),
                        format!("ALU54_R{r}C{c}", c = c + 3),
                    ],
                );
                let cell_base = tcrd.cell.delta((idx as i32) * 4, 0);
                let cell_mult = [cell_base, cell_base.delta(1, 0)];
                let cell_alu24 = cell_base.delta(2, 0);
                let cell_alu54 = cell_base.delta(3, 0);
                let mut bel = LegacyBel::default();

                let cell_prev = if idx == 1 {
                    Some(bcrd.cell)
                } else if bcrd.col == self.chip.col_clk + 1 {
                    Some(bcrd.delta(-11, 0).delta(4, 0))
                } else if let Some(cell_prev) = self.edev.cell_delta(bcrd.cell, -9, 0)
                    && self.edev.has_bel(cell_prev.bel(bels::DSP0))
                {
                    Some(cell_prev.delta(4, 0))
                } else {
                    None
                };
                let cell_prev_mult = [cell_prev, cell_prev.map(|cell| cell.delta(1, 0))];
                let cell_prev_alu24 = cell_prev.map(|cell| cell.delta(2, 0));
                let cell_prev_alu54 = cell_prev.map(|cell| cell.delta(3, 0));

                let cell_next = if idx == 0 {
                    Some(bcrd.cell.delta(4, 0))
                } else if bcrd.col == self.chip.col_clk - 10 {
                    Some(bcrd.delta(11, 0))
                } else if let Some(cell_next) = self.edev.cell_delta(bcrd.cell, 9, 0)
                    && self.edev.has_bel(cell_next.bel(bels::DSP0))
                {
                    Some(cell_next)
                } else {
                    None
                };
                let cell_next_mult = [cell_next, cell_next.map(|cell| cell.delta(1, 0))];
                let cell_next_alu24 = cell_next.map(|cell| cell.delta(2, 0));
                let cell_next_alu54 = cell_next.map(|cell| cell.delta(3, 0));

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

                        let wire = self.rc_wire(cell, &format!("J{pin}_PRADD9"));
                        self.add_bel_wire(bcrd, format!("{pin}_PRADD9_{i}"), wire);
                        assert_eq!(bpin, self.xlat_int_wire(bcrd, wire));

                        let wire = self.rc_wire(cell, &format!("J{pin}_PRADD18"));
                        self.add_bel_wire(bcrd, format!("{pin}_PRADD18_{i}"), wire);
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

                for i in 0..2 {
                    let wire_pradd9 = self.rc_wire(cell_mult[i], "JOPPRE_PRADD9");
                    self.add_bel_wire(bcrd, format!("OPPRE_PRADD9_{i}"), wire_pradd9);
                    let bpin = self.xlat_int_wire(bcrd, wire_pradd9);

                    let wire_pradd18 = self.rc_wire(cell_mult[i], "JOPPRE_PRADD18");
                    self.add_bel_wire(bcrd, format!("OPPRE_PRADD18_{i}"), wire_pradd18);
                    assert_eq!(bpin, self.xlat_int_wire(bcrd, wire_pradd18));

                    bel.pins.insert(format!("OPPRE_{i}"), bpin);
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

                            if ab == 'A' && pin == "SOURCE" {
                                let wire_pradd9 =
                                    self.rc_wire(cell_mult[i], &format!("J{pin}{ab}_PRADD9"));
                                self.add_bel_wire(
                                    bcrd,
                                    format!("{pin}{ab}_PRADD9_{i}"),
                                    wire_pradd9,
                                );
                                assert_eq!(bpin, self.xlat_int_wire(bcrd, wire_pradd9));

                                let wire_pradd18 =
                                    self.rc_wire(cell_mult[i], &format!("J{pin}{ab}_PRADD18"));
                                self.add_bel_wire(
                                    bcrd,
                                    format!("{pin}{ab}_PRADD18_{i}"),
                                    wire_pradd18,
                                );
                                assert_eq!(bpin, self.xlat_int_wire(bcrd, wire_pradd18));
                            }
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

                // input A
                for mult_idx in 0..2 {
                    let mult_ab = ['A', 'B'][mult_idx];
                    for i in 0..18 {
                        let wire_mui = self.rc_wire(cell_mult[mult_idx], &format!("JMUIA{i}"));
                        self.add_bel_wire(bcrd, format!("MUIA{i}_{mult_idx}"), wire_mui);
                        let bpin = self.xlat_int_wire(bcrd, wire_mui);

                        let wire_pradd18 =
                            self.rc_wire(cell_mult[mult_idx], &format!("JPA{i}_PRADD18"));
                        self.add_bel_wire(bcrd, format!("PA{i}_PRADD18_{mult_idx}"), wire_pradd18);
                        assert_eq!(self.xlat_int_wire(bcrd, wire_pradd18), bpin);

                        if i < 9 {
                            let wire_pradd9 =
                                self.rc_wire(cell_mult[mult_idx], &format!("JPA{i}_PRADD9"));
                            self.add_bel_wire(
                                bcrd,
                                format!("PA{i}_PRADD9_{mult_idx}"),
                                wire_pradd9,
                            );
                            assert_eq!(self.xlat_int_wire(bcrd, wire_pradd9), bpin);
                        }

                        bel.pins.insert(format!("A{i}_{mult_idx}"), bpin);

                        let wire_po = self.rc_wire(cell_mult[mult_idx], &format!("JPO{i}"));
                        self.add_bel_wire(bcrd, format!("PO{i}_{mult_idx}"), wire_po);

                        let wire_po_pradd18 =
                            self.rc_wire(cell_mult[mult_idx], &format!("JPO{i}_PRADD18"));
                        self.add_bel_wire(
                            bcrd,
                            format!("PO{i}_PRADD18_{mult_idx}"),
                            wire_po_pradd18,
                        );
                        self.claim_pip(wire_po, wire_po_pradd18);

                        if i < 9 {
                            let wire_po_pradd9 =
                                self.rc_wire(cell_mult[mult_idx], &format!("JPO{i}_PRADD9"));
                            self.add_bel_wire(
                                bcrd,
                                format!("PO{i}_PRADD9_{mult_idx}"),
                                wire_po_pradd9,
                            );
                            self.claim_pip(wire_po, wire_po_pradd9);
                        }

                        let wire_multa = self.rc_wire(cell_mult[mult_idx], &format!("JMULTA{i}"));
                        self.add_bel_wire(bcrd, format!("MULTA{i}_{mult_idx}"), wire_multa);

                        self.claim_pip(wire_multa, wire_mui);
                        self.claim_pip(wire_multa, wire_po);

                        let wire_alu = self.rc_wire(cell_alu54, &format!("J{mult_ab}{i}_ALU54"));
                        self.add_bel_wire(bcrd, format!("{mult_ab}{i}_ALU54"), wire_alu);

                        let wire_mult18 =
                            self.rc_wire(cell_mult[mult_idx], &format!("JA{i}_MULT18"));
                        self.add_bel_wire(bcrd, format!("A{i}_MULT18_{mult_idx}"), wire_mult18);
                        self.claim_pip(wire_mult18, wire_multa);

                        let wire_ro_mult18 =
                            self.rc_wire(cell_mult[mult_idx], &format!("JROA{i}_MULT18"));
                        self.add_bel_wire(
                            bcrd,
                            format!("ROA{i}_MULT18_{mult_idx}"),
                            wire_ro_mult18,
                        );
                        self.claim_pip(wire_ro_mult18, wire_mult18);
                        self.claim_pip(wire_alu, wire_ro_mult18);

                        if i < 9 {
                            let wire_mult9 =
                                self.rc_wire(cell_mult[mult_idx], &format!("JA{i}_MULT9"));
                            self.add_bel_wire(bcrd, format!("A{i}_MULT9_{mult_idx}"), wire_mult9);
                            self.claim_pip(wire_mult9, wire_multa);

                            let wire_ro_mult9 =
                                self.rc_wire(cell_mult[mult_idx], &format!("JROA{i}_MULT9"));
                            self.add_bel_wire(
                                bcrd,
                                format!("ROA{i}_MULT9_{mult_idx}"),
                                wire_ro_mult9,
                            );
                            self.claim_pip(wire_ro_mult9, wire_mult9);
                            self.claim_pip(wire_alu, wire_ro_mult9);
                        }
                    }
                }

                // input B
                for mult_idx in 0..2 {
                    let mult_ab = ['A', 'B'][mult_idx];
                    for i in 0..18 {
                        let alu_pin = i + 18;
                        let wire_alu =
                            self.rc_wire(cell_alu54, &format!("J{mult_ab}{alu_pin}_ALU54"));
                        self.add_bel_wire(bcrd, format!("{mult_ab}{alu_pin}_ALU54"), wire_alu);

                        let wire_pradd18 =
                            self.rc_wire(cell_mult[mult_idx], &format!("JPB{i}_PRADD18"));
                        self.add_bel_wire(bcrd, format!("PB{i}_PRADD18_{mult_idx}"), wire_pradd18);
                        let bpin = self.xlat_int_wire(bcrd, wire_pradd18);

                        let wire_mult18 =
                            self.rc_wire(cell_mult[mult_idx], &format!("JB{i}_MULT18"));
                        self.add_bel_wire(bcrd, format!("B{i}_MULT18_{mult_idx}"), wire_mult18);
                        assert_eq!(self.xlat_int_wire(bcrd, wire_mult18), bpin);

                        let wire_ro_mult18 =
                            self.rc_wire(cell_mult[mult_idx], &format!("JROB{i}_MULT18"));
                        self.add_bel_wire(
                            bcrd,
                            format!("ROB{i}_MULT18_{mult_idx}"),
                            wire_ro_mult18,
                        );
                        self.claim_pip(wire_ro_mult18, wire_mult18);
                        self.claim_pip(wire_alu, wire_ro_mult18);

                        if i < 9 {
                            let wire_pradd9 =
                                self.rc_wire(cell_mult[mult_idx], &format!("JPB{i}_PRADD9"));
                            self.add_bel_wire(
                                bcrd,
                                format!("PB{i}_PRADD9_{mult_idx}"),
                                wire_pradd9,
                            );
                            assert_eq!(self.xlat_int_wire(bcrd, wire_pradd9), bpin);

                            let wire_mult9 =
                                self.rc_wire(cell_mult[mult_idx], &format!("JB{i}_MULT9"));
                            self.add_bel_wire(bcrd, format!("B{i}_MULT9_{mult_idx}"), wire_mult9);
                            assert_eq!(self.xlat_int_wire(bcrd, wire_mult9), bpin);

                            let wire_ro_mult9 =
                                self.rc_wire(cell_mult[mult_idx], &format!("JROB{i}_MULT9"));
                            self.add_bel_wire(
                                bcrd,
                                format!("ROB{i}_MULT9_{mult_idx}"),
                                wire_ro_mult9,
                            );
                            self.claim_pip(wire_ro_mult9, wire_mult9);
                            self.claim_pip(wire_alu, wire_ro_mult9);
                        }

                        bel.pins.insert(format!("B{i}_{mult_idx}"), bpin);
                    }
                }

                // input C
                for i in 0..54 {
                    if i < 27 {
                        let wire_roc = self.rc_wire(cell_alu24, &format!("JROC{ii}", ii = i + 27));

                        let wire_mui = self.rc_wire(cell_alu24, &format!("JMUIC{i}"));
                        self.add_bel_wire(bcrd, format!("MUIC{i}"), wire_mui);
                        let bpin = self.xlat_int_wire(bcrd, wire_mui);
                        bel.pins.insert(format!("C{i}"), bpin);

                        let wire_dsp = self.rc_wire(cell_alu24, &format!("JDSPC{i}"));
                        self.add_bel_wire(bcrd, format!("DSPC{i}"), wire_dsp);
                        self.claim_pip(wire_dsp, wire_mui);
                        self.claim_pip(wire_dsp, wire_roc);

                        let wire_alu = self.rc_wire(cell_alu54, &format!("JC{i}_ALU54"));
                        self.add_bel_wire(bcrd, format!("C{i}_ALU54"), wire_alu);
                        self.claim_pip(wire_alu, wire_dsp);

                        if i < 18 {
                            let wire_mult18 = self.rc_wire(cell_mult[0], &format!("JC{i}_MULT18"));
                            self.add_bel_wire(bcrd, format!("C{i}_MULT18_0"), wire_mult18);
                            self.claim_pip(wire_mult18, wire_dsp);

                            let wire_ro_mult18 =
                                self.rc_wire(cell_mult[0], &format!("ROC{i}_MULT18"));
                            self.add_bel_wire(bcrd, format!("ROC{i}_MULT18_0"), wire_ro_mult18);

                            let wire_pradd18 =
                                self.rc_wire(cell_mult[0], &format!("JC{i}_PRADD18"));
                            self.add_bel_wire(bcrd, format!("C{i}_PRADD18_0"), wire_pradd18);
                            self.claim_pip(wire_pradd18, wire_dsp);
                        }
                        if i < 9 {
                            let wire_mult9 = self.rc_wire(cell_mult[0], &format!("JC{i}_MULT9"));
                            self.add_bel_wire(bcrd, format!("C{i}_MULT9_0"), wire_mult9);
                            self.claim_pip(wire_mult9, wire_dsp);

                            let wire_ro_mult9 =
                                self.rc_wire(cell_mult[0], &format!("ROC{i}_MULT9"));
                            self.add_bel_wire(bcrd, format!("ROC{i}_MULT9_0"), wire_ro_mult9);

                            let wire_pradd9 = self.rc_wire(cell_mult[0], &format!("JC{i}_PRADD9"));
                            self.add_bel_wire(bcrd, format!("C{i}_PRADD9_0"), wire_pradd9);
                            self.claim_pip(wire_pradd9, wire_dsp);
                        }
                    } else {
                        let ii = i - 27;
                        let wire_alu = self.rc_wire(cell_alu54, &format!("JC{i}_ALU54"));
                        self.add_bel_wire(bcrd, format!("C{i}_ALU54"), wire_alu);
                        let bpin = self.xlat_int_wire(bcrd, wire_alu);

                        let wire_roc = self.rc_wire(cell_alu24, &format!("JROC{i}"));
                        self.add_bel_wire(bcrd, format!("ROC{i}"), wire_roc);

                        if ii < 18 {
                            let wire_mult18 = self.rc_wire(cell_mult[1], &format!("JC{ii}_MULT18"));
                            self.add_bel_wire(bcrd, format!("C{ii}_MULT18_1"), wire_mult18);
                            assert_eq!(self.xlat_int_wire(bcrd, wire_mult18), bpin);

                            let wire_ro_mult18 =
                                self.rc_wire(cell_mult[1], &format!("JROC{ii}_MULT18"));
                            self.add_bel_wire(bcrd, format!("ROC{ii}_MULT18_1"), wire_ro_mult18);
                            self.claim_pip(wire_roc, wire_ro_mult18);

                            let wire_pradd18 =
                                self.rc_wire(cell_mult[1], &format!("JC{ii}_PRADD18"));
                            self.add_bel_wire(bcrd, format!("C{ii}_PRADD18_1"), wire_pradd18);
                            assert_eq!(self.xlat_int_wire(bcrd, wire_pradd18), bpin);
                        }
                        if ii < 9 {
                            let wire_mult9 = self.rc_wire(cell_mult[1], &format!("JC{ii}_MULT9"));
                            self.add_bel_wire(bcrd, format!("C{ii}_MULT9_1"), wire_mult9);
                            assert_eq!(self.xlat_int_wire(bcrd, wire_mult9), bpin);

                            let wire_ro_mult9 =
                                self.rc_wire(cell_mult[1], &format!("JROC{ii}_MULT9"));
                            self.add_bel_wire(bcrd, format!("ROC{ii}_MULT9_1"), wire_ro_mult9);
                            self.claim_pip(wire_roc, wire_ro_mult9);

                            let wire_pradd9 = self.rc_wire(cell_mult[1], &format!("JC{ii}_PRADD9"));
                            self.add_bel_wire(bcrd, format!("C{ii}_PRADD9_1"), wire_pradd9);
                            assert_eq!(self.xlat_int_wire(bcrd, wire_pradd9), bpin);
                        }
                        if ii >= 18 {
                            assert_eq!(self.xlat_int_wire(bcrd, wire_roc), bpin);
                        }

                        bel.pins.insert(format!("C{i}"), bpin);
                    }
                }

                // output
                for mult_idx in 0..2 {
                    let mult_ab = ['A', 'B'][mult_idx];
                    for i in 0..36 {
                        let wire_mult18 =
                            self.rc_wire(cell_mult[mult_idx], &format!("JP{i}_MULT18"));
                        self.add_bel_wire(bcrd, format!("P{i}_MULT18_{mult_idx}"), wire_mult18);
                        let bpin = self.xlat_int_wire(bcrd, wire_mult18);

                        let wire_alu54 = self.rc_wire(cell_alu54, &format!("JM{mult_ab}{i}_ALU54"));
                        self.add_bel_wire(bcrd, format!("M{mult_ab}{i}_ALU54"), wire_alu54);
                        self.claim_pip(wire_alu54, wire_mult18);

                        if i < 18 {
                            let wire_mult9 =
                                self.rc_wire(cell_mult[mult_idx], &format!("JP{i}_MULT9"));
                            self.add_bel_wire(bcrd, format!("P{i}_MULT9_{mult_idx}"), wire_mult9);
                            assert_eq!(self.xlat_int_wire(bcrd, wire_mult9), bpin);

                            self.claim_pip(wire_alu54, wire_mult9);

                            let wire_alu24 =
                                self.rc_wire(cell_alu24, &format!("JM{mult_ab}{i}_ALU24"));
                            self.add_bel_wire(bcrd, format!("M{mult_ab}{i}_ALU24"), wire_alu24);
                            self.claim_pip(wire_alu24, wire_mult9);
                        }

                        bel.pins.insert(format!("P{i}_{mult_idx}"), bpin);
                    }
                }

                for i in 0..54 {
                    let wire_p = self.rc_wire(cell_alu24, &format!("JP{i}"));
                    self.add_bel_wire(bcrd, format!("P{i}"), wire_p);
                    let bpin = self.xlat_int_wire(bcrd, wire_p);
                    bel.pins.insert(format!("R{i}"), bpin);

                    let wire_r = self.rc_wire(cell_alu24, &format!("JR{i}"));
                    self.add_bel_wire(bcrd, format!("R{i}"), wire_r);
                    self.claim_pip(wire_p, wire_r);

                    let wire_co = self.rc_wire(cell_alu24, &format!("JCO{i}"));
                    self.add_bel_wire(bcrd, format!("CO{i}"), wire_co);
                    self.claim_pip(wire_p, wire_co);

                    let wire_r_alu54 = self.rc_wire(cell_alu54, &format!("JR{i}_ALU54"));
                    self.add_bel_wire(bcrd, format!("R{i}_ALU54"), wire_r_alu54);
                    self.claim_pip(wire_r, wire_r_alu54);

                    let wire_co_alu54 = self.rc_wire(cell_alu54, &format!("JCO{i}_ALU54"));
                    self.add_bel_wire(bcrd, format!("CO{i}_ALU54"), wire_co_alu54);
                    self.claim_pip(wire_co, wire_co_alu54);

                    if i < 24 {
                        let wire_r_alu24 = self.rc_wire(cell_alu24, &format!("JR{i}_ALU24"));
                        self.add_bel_wire(bcrd, format!("R{i}_ALU24"), wire_r_alu24);
                        self.claim_pip(wire_r, wire_r_alu24);

                        let wire_co_alu24 = self.rc_wire(cell_alu24, &format!("JCO{i}_ALU24"));
                        self.add_bel_wire(bcrd, format!("CO{i}_ALU24"), wire_co_alu24);
                        self.claim_pip(wire_co, wire_co_alu24);
                    }

                    let wire_cfb = self.rc_wire(cell_alu24, &format!("JCFB{i}"));
                    self.add_bel_wire(bcrd, format!("CFB{i}"), wire_cfb);
                    self.claim_pip(wire_cfb, wire_r);

                    if cell_next.is_some() {
                        let wire_nextr = self.rc_wire(cell_alu24, &format!("JNEXTR{i}"));
                        self.add_bel_wire(bcrd, format!("NEXTR{i}"), wire_nextr);
                        self.claim_pip(wire_cfb, wire_nextr);

                        if let Some(cell_next) = cell_next_alu54 {
                            let wire_r_alu54_next =
                                self.rc_wire(cell_next, &format!("JR{i}_ALU54"));
                            self.claim_pip(wire_nextr, wire_r_alu54_next);
                        }
                        if i < 24
                            && let Some(cell_next) = cell_next_alu24
                        {
                            let wire_r_alu24_next =
                                self.rc_wire(cell_next, &format!("JR{i}_ALU24"));
                            self.claim_pip(wire_nextr, wire_r_alu24_next);
                        }
                    }

                    let wire_cfb_alu54 = self.rc_wire(cell_alu54, &format!("JCFB{i}_ALU54"));
                    self.add_bel_wire(bcrd, format!("CFB{i}_ALU54"), wire_cfb_alu54);
                    self.claim_pip(wire_cfb_alu54, wire_cfb);

                    if i < 24 {
                        let wire_cfb_alu24 = self.rc_wire(cell_alu24, &format!("JCFB{i}_ALU24"));
                        self.add_bel_wire(bcrd, format!("CFB{i}_ALU24"), wire_cfb_alu24);
                        self.claim_pip(wire_cfb_alu24, wire_cfb);
                    }
                }

                // SRA
                for i in 0..18 {
                    for mult_idx in 0..2 {
                        for prim in ["MULT", "PRADD"] {
                            for (sz, osz) in [(9, 18), (18, 9)] {
                                if i < sz {
                                    let wire_sria = self.rc_wire(
                                        cell_mult[mult_idx],
                                        &format!("JSRIA{i}_{prim}{sz}"),
                                    );
                                    self.add_bel_wire(
                                        bcrd,
                                        format!("SRIA{i}_{prim}{sz}_{mult_idx}"),
                                        wire_sria,
                                    );
                                    let wire_sroa = self.rc_wire(
                                        cell_mult[mult_idx],
                                        &format!("JSROA{i}_{prim}{sz}"),
                                    );
                                    self.add_bel_wire(
                                        bcrd,
                                        format!("SROA{i}_{prim}{sz}_{mult_idx}"),
                                        wire_sroa,
                                    );
                                    self.claim_pip(wire_sroa, wire_sria);

                                    if mult_idx == 0 {
                                        if let Some(cell_prev) = cell_prev_alu24 {
                                            let wire_sroa_prev =
                                                self.rc_wire(cell_prev, &format!("JSROA{i}"));
                                            self.claim_pip(wire_sria, wire_sroa_prev);
                                        }
                                    } else {
                                        let wire_sroa_prev = self
                                            .rc_wire(cell_mult[0], &format!("JSROA{i}_{prim}{sz}"));
                                        self.claim_pip(wire_sria, wire_sroa_prev);
                                        if i < 9 {
                                            let wire_sroa_prev = self.rc_wire(
                                                cell_mult[0],
                                                &format!("JSROA{i}_{prim}{osz}"),
                                            );
                                            self.claim_pip(wire_sria, wire_sroa_prev);
                                        }
                                    }
                                }
                            }
                        }
                        if mult_idx == 1 {
                            let wire_sroa = self.rc_wire(cell_alu24, &format!("JSROA{i}"));
                            self.add_bel_wire(bcrd, format!("SROA{i}"), wire_sroa);
                            let bpin = self.xlat_int_wire(bcrd, wire_sroa);
                            bel.pins.insert(format!("SROA{i}"), bpin);

                            for (prim, mp) in [("MULT", 'M'), ("PRADD", 'P')] {
                                let wire_mpsroa =
                                    self.rc_wire(cell_alu24, &format!("J{mp}SROA{i}"));
                                self.add_bel_wire(bcrd, format!("{mp}SROA{i}"), wire_mpsroa);
                                self.claim_pip(wire_sroa, wire_mpsroa);

                                let wire_prim18_sroa = self
                                    .rc_wire(cell_mult[mult_idx], &format!("JSROA{i}_{prim}18"));
                                self.claim_pip(wire_mpsroa, wire_prim18_sroa);
                                if i < 9 {
                                    let wire_prim9_sroa = self
                                        .rc_wire(cell_mult[mult_idx], &format!("JSROA{i}_{prim}9"));
                                    self.claim_pip(wire_mpsroa, wire_prim9_sroa);
                                }
                            }
                        }
                    }
                }

                // SRB

                for i in 0..18 {
                    for mult_idx in 0..2 {
                        for (prim, cell_prev) in [
                            ("MULT", [cell_prev_mult[1], Some(cell_mult[0])][mult_idx]),
                            ("PRADD", [Some(cell_mult[1]), cell_next_mult[0]][mult_idx]),
                        ] {
                            for (sz, osz) in [(9, 18), (18, 9)] {
                                if i < sz {
                                    let wire_srib = self.rc_wire(
                                        cell_mult[mult_idx],
                                        &format!("JSRIB{i}_{prim}{sz}"),
                                    );
                                    self.add_bel_wire(
                                        bcrd,
                                        format!("SRIB{i}_{prim}{sz}_{mult_idx}"),
                                        wire_srib,
                                    );
                                    let wire_srob = self.rc_wire(
                                        cell_mult[mult_idx],
                                        &format!("JSROB{i}_{prim}{sz}"),
                                    );
                                    self.add_bel_wire(
                                        bcrd,
                                        format!("SROB{i}_{prim}{sz}_{mult_idx}"),
                                        wire_srob,
                                    );
                                    self.claim_pip(wire_srob, wire_srib);

                                    if let Some(cell_prev) = cell_prev {
                                        let wire_srob_prev = self
                                            .rc_wire(cell_prev, &format!("JSROB{i}_{prim}{sz}"));
                                        self.claim_pip(wire_srib, wire_srob_prev);
                                        if i < 9 {
                                            let wire_srob_prev = self.rc_wire(
                                                cell_prev,
                                                &format!("JSROB{i}_{prim}{osz}"),
                                            );
                                            self.claim_pip(wire_srib, wire_srob_prev);
                                        }
                                    }
                                }
                            }
                        }
                    }
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

                for i in 0..54 {
                    let wire = self.rc_wire(cell_alu54, &format!("JCIN{i}_ALU54"));
                    self.add_bel_wire(bcrd, format!("CIN{i}_ALU54"), wire);
                    if let Some(cell_prev_alu54) = cell_prev_alu54 {
                        let wire_src = self.rc_wire(cell_prev_alu54, &format!("JR{i}_ALU54"));
                        self.claim_pip(wire, wire_src);
                    }
                }
                for i in 0..24 {
                    let wire = self.rc_wire(cell_alu24, &format!("JCIN{i}_ALU24"));
                    self.add_bel_wire(bcrd, format!("CIN{i}_ALU24"), wire);
                    if let Some(cell_prev_alu24) = cell_prev_alu24 {
                        let wire_src = self.rc_wire(cell_prev_alu24, &format!("JR{i}_ALU24"));
                        self.claim_pip(wire, wire_src);
                    }
                }
                let wire = self.rc_wire(cell_alu54, "JSIGNEDR_ALU54");
                self.add_bel_wire(bcrd, "SIGNEDR_ALU54", wire);
                let wire = self.rc_wire(cell_alu54, "JSIGNEDCIN_ALU54");
                self.add_bel_wire(bcrd, "SIGNEDCIN_ALU54", wire);
                if let Some(cell_prev_alu54) = cell_prev_alu54 {
                    let wire_src = self.rc_wire(cell_prev_alu54, "JSIGNEDR_ALU54");
                    self.claim_pip(wire, wire_src);
                }

                self.insert_bel(bcrd, bel);
            }
        }
    }
}
