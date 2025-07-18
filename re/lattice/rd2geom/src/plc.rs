use prjcombine_ecp::{bels, chip::ChipKind};

use super::ChipContext;

impl ChipContext<'_> {
    pub fn process_plc(&mut self) {
        for tcname in ["PLC", "FPLC"] {
            let tcid = self.intdb.get_tile_class(tcname);
            for &tcrd in &self.edev.egrid.tile_index[tcid] {
                let cell = tcrd.cell;
                let slices = [
                    cell.bel(bels::SLICE0),
                    cell.bel(bels::SLICE1),
                    cell.bel(bels::SLICE2),
                    cell.bel(bels::SLICE3),
                ];
                let (r, c) = self.rc(cell);
                self.name_bel(slices[0], [format!("R{r}C{c}A")]);
                self.name_bel(slices[1], [format!("R{r}C{c}B")]);
                self.name_bel(slices[2], [format!("R{r}C{c}C")]);
                self.name_bel(slices[3], [format!("R{r}C{c}D")]);
                for i in 0..4 {
                    let abcd = ['A', 'B', 'C', 'D'][i];

                    // plain inputs
                    for pin in [
                        "A0", "A1", "B0", "B1", "C0", "C1", "D0", "D1", "M0", "M1", "CLK", "LSR",
                        "CE",
                    ] {
                        let wire_int = self.edev.egrid.get_bel_pin(slices[i], pin)[0];
                        let wn = self
                            .intdb
                            .wires
                            .key(wire_int.slot)
                            .strip_prefix("IMUX_")
                            .unwrap();
                        let wire_slice = self.rc_wire(cell, &format!("{wn}_SLICE"));
                        self.add_bel_wire(slices[i], pin, wire_slice);
                        let wire_int = self.naming.interconnect[&wire_int];
                        self.claim_pip(wire_slice, wire_int);
                    }

                    let io_cell = if self.chip.kind == ChipKind::MachXo && i < 3 {
                        if cell.row == self.chip.row_s() + 1 {
                            Some(cell.delta(0, -1))
                        } else if cell.row == self.chip.row_n() - 1 {
                            Some(cell.delta(0, 1))
                        } else if cell.col == self.chip.col_w() + 1 {
                            Some(cell.delta(-1, 0))
                        } else if cell.col == self.chip.col_e() - 1 {
                            Some(cell.delta(1, 0))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    // plain outputs
                    for pin in ["F0", "F1", "Q0", "Q1"] {
                        let wire_int = self.edev.egrid.get_bel_pin(slices[i], pin)[0];
                        let wn = self
                            .intdb
                            .wires
                            .key(wire_int.slot)
                            .strip_prefix("OUT_")
                            .unwrap();
                        let wire_slice = self.rc_wire(
                            cell,
                            &if self.chip.kind == ChipKind::MachXo {
                                format!("J{wn}_SLICE")
                            } else {
                                format!("{wn}_SLICE")
                            },
                        );
                        self.add_bel_wire(slices[i], pin, wire_slice);
                        let wire_int = self.naming.interconnect[&wire_int];
                        self.claim_pip(wire_int, wire_slice);

                        if let Some(io_cell) = io_cell {
                            let wire_io = self.rc_wire(io_cell, &format!("JPLC{wn}"));
                            self.add_bel_wire(slices[i], format!("{pin}_IO"), wire_io);
                            self.claim_pip(wire_io, wire_slice);
                        }
                    }

                    // F5, FX
                    let ofx0_int = self.edev.egrid.get_bel_pin(slices[i], "OFX0")[0];
                    let ofx1_int = self.edev.egrid.get_bel_pin(slices[i], "OFX1")[0];
                    let ofx0_int = self.naming.interconnect[&ofx0_int];
                    let ofx1_int = self.naming.interconnect[&ofx1_int];
                    let ofx0_slice = self.rc_wire(cell, &format!("F5{abcd}_SLICE"));
                    let ofx1_slice = self.rc_wire(cell, &format!("FX{abcd}_SLICE"));
                    self.add_bel_wire(slices[i], "OFX0", ofx0_slice);
                    self.add_bel_wire(slices[i], "OFX1", ofx1_slice);
                    self.claim_pip(ofx0_int, ofx0_slice);
                    self.claim_pip(ofx1_int, ofx1_slice);

                    // FXA, FXB
                    let fxa = self.rc_wire(cell, &format!("FXA{abcd}"));
                    let fxb = self.rc_wire(cell, &format!("FXB{abcd}"));
                    let fxa_slice = self.rc_wire(cell, &format!("FXA{abcd}_SLICE"));
                    let fxb_slice = self.rc_wire(cell, &format!("FXB{abcd}_SLICE"));
                    self.add_bel_wire(slices[i], "FXA", fxa);
                    self.add_bel_wire(slices[i], "FXB", fxb);
                    self.add_bel_wire(slices[i], "FXA_SLICE", fxa_slice);
                    self.add_bel_wire(slices[i], "FXB_SLICE", fxb_slice);
                    self.claim_pip(fxa_slice, fxa);
                    self.claim_pip(fxb_slice, fxb);

                    let (ia, ib) = [(0, 2), (1, 5), (4, 6), (3, 3)][i];
                    let fxa_int = self.naming.interconnect
                        [&cell.wire(self.intdb.get_wire(&format!("OUT_OFX{ia}")))];
                    self.claim_pip(fxa, fxa_int);
                    if i == 3 {
                        if let Some(&fxb_int) = self
                            .naming
                            .interconnect
                            .get(&cell.wire(self.intdb.get_wire("OUT_OFX3_W")))
                        {
                            self.claim_pip(fxb, fxb_int);
                        }
                    } else {
                        let fxb_int = self.naming.interconnect
                            [&cell.wire(self.intdb.get_wire(&format!("OUT_OFX{ib}")))];
                        self.claim_pip(fxb, fxb_int);
                    }

                    // DI
                    let di0 = self.rc_wire(cell, &format!("DI{ii}", ii = 2 * i));
                    let di0_slice = self.rc_wire(cell, &format!("DI{ii}_SLICE", ii = 2 * i));
                    let di1 = self.rc_wire(cell, &format!("DI{ii}", ii = 2 * i + 1));
                    let di1_slice = self.rc_wire(cell, &format!("DI{ii}_SLICE", ii = 2 * i + 1));
                    self.add_bel_wire(slices[i], "DI0", di0);
                    self.add_bel_wire(slices[i], "DI1", di1);
                    self.add_bel_wire(slices[i], "DI0_SLICE", di0_slice);
                    self.add_bel_wire(slices[i], "DI1_SLICE", di1_slice);
                    let f0 = self.edev.egrid.get_bel_pin(slices[i], "F0")[0];
                    let f1 = self.edev.egrid.get_bel_pin(slices[i], "F1")[0];
                    let f0 = self.naming.interconnect[&f0];
                    let f1 = self.naming.interconnect[&f1];
                    self.claim_pip(di0_slice, di0);
                    self.claim_pip(di1_slice, di1);
                    self.claim_pip(di0, f0);
                    self.claim_pip(di0, ofx0_int);
                    self.claim_pip(di1, f1);
                    self.claim_pip(di1, ofx1_int);
                }

                for (i, pin, wire) in [
                    (0, "FCI", "FCI"),
                    (0, "FCI_SLICE", "FCI_SLICE"),
                    (1, "FCI_SLICE", "FCIB_SLICE"),
                    (2, "FCI_SLICE", "FCIC_SLICE"),
                    (3, "FCI_SLICE", "FCID_SLICE"),
                    (0, "FCO_SLICE", "FCOA_SLICE"),
                    (1, "FCO_SLICE", "FCOB_SLICE"),
                    (2, "FCO_SLICE", "FCOC_SLICE"),
                    (3, "FCO_SLICE", "FCO_SLICE"),
                    (3, "FCO", "FCO"),
                ] {
                    self.add_bel_wire(slices[i], pin, self.rc_wire(cell, wire));
                }
                for (wt, wf) in [
                    ("FCI_SLICE", "FCI"),
                    ("FCIB_SLICE", "FCOA_SLICE"),
                    ("FCIC_SLICE", "FCOB_SLICE"),
                    ("FCID_SLICE", "FCOC_SLICE"),
                    ("FCO", "FCO_SLICE"),
                ] {
                    let wt = self.rc_wire(cell, wt);
                    let wf = self.rc_wire(cell, wf);
                    self.claim_pip(wt, wf);
                }
                let fci_in = self.naming.strings.get("FCI_IN").unwrap();
                if let Some(naming) = self.naming.bels.get(&cell.bel(bels::INT))
                    && let Some(&wf) = naming.wires.get(&fci_in)
                {
                    let wt = self.rc_wire(cell, "FCI");
                    self.claim_pip(wt, wf);
                }
                if let Some(naming) = self.naming.bels.get(&cell.delta(1, 0).bel(bels::INT))
                    && let Some(&wt) = naming.wires.get(&fci_in)
                {
                    let wf = self.rc_wire(cell, "FCO");
                    self.claim_pip(wt, wf);
                }

                if tcname == "PLC" {
                    for (i, wt) in [
                        (0, "CLK20_SLICE"),
                        (1, "CLK21_SLICE"),
                        (2, "CLK22_SLICE"),
                        (3, "CLK23_SLICE"),
                    ] {
                        let wt = self.rc_wire(cell, wt);
                        self.add_bel_wire(slices[i], "CLK2", wt);
                        let wf = self.edev.egrid.get_bel_pin(slices[i ^ 1], "CLK")[0];
                        let wf = self.naming.interconnect[&wf];
                        self.claim_pip(wt, wf);
                    }
                    for (i, pin, wire) in [
                        (0, "DP64_SLICE", "DP64A_SLICE"),
                        (1, "DP64_SLICE", "DP64B_SLICE"),
                        (2, "DP64_SLICE", "DP64C_SLICE"),
                        (3, "DP64_SLICE", "DP64D_SLICE"),
                        (0, "DP_SLICE", "DPA_SLICE"),
                        (1, "DP_SLICE", "DPB_SLICE"),
                        (2, "DP_SLICE", "DPC_SLICE"),
                        (3, "DP_SLICE", "DPD_SLICE"),
                        (0, "DP", "DP01"),
                        (2, "DP", "DP23"),
                    ] {
                        self.add_bel_wire(slices[i], pin, self.rc_wire(cell, wire));
                    }
                    for (wt, wf) in [
                        ("DP64B_SLICE", "DP01"),
                        ("DP64D_SLICE", "DP23"),
                        ("DP01", "DPA_SLICE"),
                        ("DP23", "DPC_SLICE"),
                    ] {
                        let wt = self.rc_wire(cell, wt);
                        let wf = self.rc_wire(cell, wf);
                        self.claim_pip(wt, wf);
                    }
                }
            }
        }
    }
}
