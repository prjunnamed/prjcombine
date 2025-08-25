use prjcombine_ecp::{bels, chip::ChipKind};

use super::ChipContext;

impl ChipContext<'_> {
    fn process_plc_ecp(&mut self) {
        for tcname in ["PLC", "FPLC"] {
            if tcname == "FPLC" && self.chip.kind == ChipKind::Scm {
                continue;
            }
            let tcid = self.intdb.get_tile_class(tcname);
            for &tcrd in &self.edev.tile_index[tcid] {
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
                        let wire_int = self.edev.get_bel_pin(slices[i], pin)[0];
                        let wn = self
                            .intdb
                            .wires
                            .key(wire_int.slot)
                            .strip_prefix("IMUX_")
                            .unwrap();
                        let wire_slice = self.rc_wire(cell, &format!("{wn}_SLICE"));
                        self.add_bel_wire(slices[i], pin, wire_slice);
                        self.claim_pip_int_in(wire_slice, wire_int);
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
                        let wire_int = self.edev.get_bel_pin(slices[i], pin)[0];
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
                        self.claim_pip_int_out(wire_int, wire_slice);

                        if let Some(io_cell) = io_cell {
                            let wire_io = self.rc_wire(io_cell, &format!("JPLC{wn}"));
                            self.add_bel_wire(slices[i], format!("{pin}_IO"), wire_io);
                            self.claim_pip(wire_io, wire_slice);
                        }
                    }

                    // F5, FX
                    let ofx0_int = self.edev.get_bel_pin(slices[i], "OFX0")[0];
                    let ofx1_int = self.edev.get_bel_pin(slices[i], "OFX1")[0];
                    let ofx0_slice = self.rc_wire(cell, &format!("F5{abcd}_SLICE"));
                    let ofx1_slice = self.rc_wire(cell, &format!("FX{abcd}_SLICE"));
                    self.add_bel_wire(slices[i], "OFX0", ofx0_slice);
                    self.add_bel_wire(slices[i], "OFX1", ofx1_slice);
                    self.claim_pip_int_out(ofx0_int, ofx0_slice);
                    self.claim_pip_int_out(ofx1_int, ofx1_slice);

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
                    let fxa_int = cell.wire(self.intdb.get_wire(&format!("OUT_OFX{ia}")));
                    self.claim_pip_int_in(fxa, fxa_int);
                    if i == 3 {
                        if let Some(&fxb_int) = self
                            .naming
                            .interconnect
                            .get(&cell.wire(self.intdb.get_wire("OUT_OFX3_W")))
                        {
                            self.claim_pip(fxb, fxb_int);
                        }
                    } else {
                        let fxb_int = cell.wire(self.intdb.get_wire(&format!("OUT_OFX{ib}")));
                        self.claim_pip_int_in(fxb, fxb_int);
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
                    let f0 = self.edev.get_bel_pin(slices[i], "F0")[0];
                    let f1 = self.edev.get_bel_pin(slices[i], "F1")[0];
                    self.claim_pip(di0_slice, di0);
                    self.claim_pip(di1_slice, di1);
                    self.claim_pip_int_in(di0, f0);
                    self.claim_pip_int_in(di0, ofx0_int);
                    self.claim_pip_int_in(di1, f1);
                    self.claim_pip_int_in(di1, ofx1_int);
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

                if self.chip.kind == ChipKind::Scm {
                    for (i, pin, wire) in
                        [(0, "FCO", "FCOA"), (1, "FCO", "FCOB"), (2, "FCO", "FCOC")]
                    {
                        self.add_bel_wire(slices[i], pin, self.rc_wire(cell, wire));
                    }
                    for (wt, wf) in [
                        ("FCOA", "FCOA_SLICE"),
                        ("FCOB", "FCOB_SLICE"),
                        ("FCOC", "FCOC_SLICE"),
                    ] {
                        let wt = self.rc_wire(cell, wt);
                        let wf = self.rc_wire(cell, wf);
                        self.claim_pip(wt, wf);
                    }
                    for (i, wf) in ["FCOA", "FCOB", "FCOC", "FCO"].into_iter().enumerate() {
                        let ofx1_int = self.edev.get_bel_pin(slices[i], "OFX1")[0];
                        let wf = self.rc_wire(cell, wf);
                        self.claim_pip_int_out(ofx1_int, wf);
                    }
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
                        let wf = self.edev.get_bel_pin(slices[i ^ 1], "CLK")[0];
                        self.claim_pip_int_in(wt, wf);
                    }
                    if self.chip.kind == ChipKind::Scm {
                        for (i, pin, wire) in [
                            (0, "DP64_SLICE", "DPI64A_SLICE"),
                            (1, "DP64_SLICE", "DPI64B_SLICE"),
                            (2, "DP64_SLICE", "DPI64C_SLICE"),
                            (3, "DP64_SLICE", "DPI64D_SLICE"),
                            (0, "DP_SLICE", "DPOA_SLICE"),
                            (1, "DP_SLICE", "DPOB_SLICE"),
                            (2, "DP_SLICE", "DPOC_SLICE"),
                            (3, "DP_SLICE", "DPOD_SLICE"),
                            (0, "DP", "DPI01"),
                            (2, "DP", "DPO23"),
                        ] {
                            self.add_bel_wire(slices[i], pin, self.rc_wire(cell, wire));
                        }
                        for (wt, wf) in [
                            ("DPI64B_SLICE", "DPI01"),
                            ("DPI64D_SLICE", "DPO23"),
                            ("DPI01", "DPOA_SLICE"),
                            ("DPO23", "DPOC_SLICE"),
                        ] {
                            let wt = self.rc_wire(cell, wt);
                            let wf = self.rc_wire(cell, wf);
                            self.claim_pip(wt, wf);
                        }
                    } else {
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

    fn process_plc_ecp2(&mut self) {
        let is_ecp3 = matches!(self.chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A);
        for tcname in ["PLC", "FPLC"] {
            let tcid = self.intdb.get_tile_class(tcname);
            for &tcrd in &self.edev.tile_index[tcid] {
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
                        if matches!(pin, "CLK" | "LSR" | "CE") && i == 3 {
                            continue;
                        }
                        let wire_int = self.edev.get_bel_pin(slices[i], pin)[0];
                        let wn = self
                            .intdb
                            .wires
                            .key(wire_int.slot)
                            .strip_prefix("IMUX_")
                            .unwrap();
                        let wire_slice = self.rc_wire(cell, &format!("{wn}_SLICE"));
                        self.add_bel_wire(slices[i], pin, wire_slice);
                        self.claim_pip_int_in(wire_slice, wire_int);
                    }

                    // plain outputs
                    for pin in ["F0", "F1", "Q0", "Q1"] {
                        if pin.starts_with('Q') && i == 3 {
                            continue;
                        }
                        let wire_int = self.edev.get_bel_pin(slices[i], pin)[0];
                        let wn = self
                            .intdb
                            .wires
                            .key(wire_int.slot)
                            .strip_prefix("OUT_")
                            .unwrap();
                        let wire_slice = self.rc_wire(cell, &format!("{wn}_SLICE"));
                        self.add_bel_wire(slices[i], pin, wire_slice);
                        self.claim_pip_int_out(wire_int, wire_slice);
                    }

                    // F5, FX
                    let ofx0_int = self.edev.get_bel_pin(slices[i], "OFX0")[0];
                    let ofx1_int = self.edev.get_bel_pin(slices[i], "OFX1")[0];
                    let ofx0_slice = self.rc_wire(cell, &format!("F5{abcd}_SLICE"));
                    let ofx1_slice = self.rc_wire(cell, &format!("FX{abcd}_SLICE"));
                    self.add_bel_wire(slices[i], "OFX0", ofx0_slice);
                    self.add_bel_wire(slices[i], "OFX1", ofx1_slice);
                    self.claim_pip_int_out(ofx0_int, ofx0_slice);
                    self.claim_pip_int_out(ofx1_int, ofx1_slice);
                    let out_idx = if is_ecp3 { 1 } else { 2 };
                    if i == out_idx && self.edev.has_bel(cell.delta(-1, 0).bel(bels::INT)) {
                        let fx_out = self.rc_wire(cell, "HL7W0001");
                        self.claim_pip_int_in(fx_out, ofx1_int);
                    }

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

                    if is_ecp3 {
                        let (ia, ib) = [(2, 0), (5, 1), (6, 4), (3, 3)][i];
                        if i == 3 {
                            let cell_src = cell.delta(1, 0);
                            if self.edev.has_bel(cell_src.bel(bels::INT)) {
                                let fx_out = self.rc_wire(cell_src, "HL7W0001");
                                self.claim_pip(fxa, fx_out);
                            }
                        } else {
                            let fxa_int = cell.wire(self.intdb.get_wire(&format!("OUT_OFX{ia}")));
                            self.claim_pip_int_in(fxa, fxa_int);
                        }
                        let fxb_int = cell.wire(self.intdb.get_wire(&format!("OUT_OFX{ib}")));
                        self.claim_pip_int_in(fxb, fxb_int);
                    } else {
                        let (ia, ib) = [(5, 5), (2, 0), (7, 3), (6, 4)][i];
                        if i == 0 {
                            let cell_src = cell.delta(1, 0);
                            if self.edev.has_bel(cell_src.bel(bels::INT)) {
                                let fx_out = self.rc_wire(cell_src, "HL7W0001");
                                self.claim_pip(fxa, fx_out);
                            }
                        } else {
                            let fxa_int = cell.wire(self.intdb.get_wire(&format!("OUT_OFX{ia}")));
                            self.claim_pip_int_in(fxa, fxa_int);
                        }
                        let fxb_int = cell.wire(self.intdb.get_wire(&format!("OUT_OFX{ib}")));
                        self.claim_pip_int_in(fxb, fxb_int);
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
                    self.claim_pip(di0_slice, di0);
                    self.claim_pip(di1_slice, di1);
                    if i != 3 {
                        let f0 = self.edev.get_bel_pin(slices[i], "F0")[0];
                        let f1 = self.edev.get_bel_pin(slices[i], "F1")[0];
                        self.claim_pip_int_in(di0, f0);
                        self.claim_pip_int_in(di0, ofx0_int);
                        self.claim_pip_int_in(di1, f1);
                        self.claim_pip_int_in(di1, ofx1_int);
                    }

                    if tcname == "PLC" {
                        for pin in [
                            "WRE", "WREO", "WCK", "WCKO", "WAD0", "WAD1", "WAD2", "WAD3", "WADO0",
                            "WADO1", "WADO2", "WADO3", "WD0", "WD1", "WDO0", "WDO1", "WDO2",
                            "WDO3",
                        ] {
                            let wire = self.rc_wire(cell, &format!("{pin}{abcd}_SLICE"));
                            self.add_bel_wire(slices[i], pin, wire);
                        }
                    }
                }

                for (i, pin, wire) in [
                    (0, "FCI", "FCI"),
                    (0, "FCI_SLICE", "FCI_SLICE"),
                    (1, "FCI_SLICE", "FCIB_SLICE"),
                    (2, "FCI_SLICE", "FCIC_SLICE"),
                    (0, "FCO_SLICE", "FCOA_SLICE"),
                    (1, "FCO_SLICE", "FCOB_SLICE"),
                    (2, "FCO_SLICE", "FCO_SLICE"),
                    (2, "FCO", "FCO"),
                    // dummy stuff
                    (3, "FCI_SLICE", "FCID_SLICE"),
                    (3, "FCO_SLICE", "FCOD_SLICE"),
                    (3, "CLK", "CLK3_SLICE"),
                    (3, "LSR", "LSR3_SLICE"),
                    (3, "CE", "CE3_SLICE"),
                    (3, "Q0", "Q6_SLICE"),
                    (3, "Q1", "Q7_SLICE"),
                ] {
                    self.add_bel_wire(slices[i], pin, self.rc_wire(cell, wire));
                }
                for (wt, wf) in [
                    ("FCI_SLICE", "FCI"),
                    ("FCIB_SLICE", "FCOA_SLICE"),
                    ("FCIC_SLICE", "FCOB_SLICE"),
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
                    for (i, pin, wire) in [
                        (1, "WCK_OUT", "WCK"),
                        (1, "WRE_OUT", "WRE"),
                        (1, "WD0_OUT", "WD0"),
                        (1, "WD1_OUT", "WD1"),
                        (1, "WD2_OUT", "WD2"),
                        (1, "WD3_OUT", "WD3"),
                        (1, "WAD0_OUT", "WAD0"),
                        (1, "WAD1_OUT", "WAD1"),
                        (1, "WAD2_OUT", "WAD2"),
                        (1, "WAD3_OUT", "WAD3"),
                    ] {
                        self.add_bel_wire(slices[i], pin, self.rc_wire(cell, wire));
                    }
                    if is_ecp3 {
                        for (wt, wf) in [
                            ("WCK", "WCKOC_SLICE"),
                            ("WCKA_SLICE", "WCK"),
                            ("WCKB_SLICE", "WCK"),
                            ("WRE", "WREOC_SLICE"),
                            ("WREA_SLICE", "WRE"),
                            ("WREB_SLICE", "WRE"),
                            ("WD0", "WDO0C_SLICE"),
                            ("WD1", "WDO1C_SLICE"),
                            ("WD2", "WDO2C_SLICE"),
                            ("WD3", "WDO3C_SLICE"),
                            ("WD0A_SLICE", "WD0"),
                            ("WD1A_SLICE", "WD1"),
                            ("WD0B_SLICE", "WD2"),
                            ("WD1B_SLICE", "WD3"),
                            ("WAD0", "WADO0C_SLICE"),
                            ("WAD1", "WADO1C_SLICE"),
                            ("WAD2", "WADO2C_SLICE"),
                            ("WAD3", "WADO3C_SLICE"),
                            ("WAD0A_SLICE", "WAD0"),
                            ("WAD1A_SLICE", "WAD1"),
                            ("WAD2A_SLICE", "WAD2"),
                            ("WAD3A_SLICE", "WAD3"),
                            ("WAD0B_SLICE", "WAD0"),
                            ("WAD1B_SLICE", "WAD1"),
                            ("WAD2B_SLICE", "WAD2"),
                            ("WAD3B_SLICE", "WAD3"),
                        ] {
                            let wt = self.rc_wire(cell, wt);
                            let wf = self.rc_wire(cell, wf);
                            self.claim_pip(wt, wf);
                        }
                    } else {
                        for (wt, wf) in [
                            ("WCK", "WCKOB_SLICE"),
                            ("WCKA_SLICE", "WCK"),
                            ("WCKC_SLICE", "WCK"),
                            ("WRE", "WREOB_SLICE"),
                            ("WREA_SLICE", "WRE"),
                            ("WREC_SLICE", "WRE"),
                            ("WD0", "WDO0B_SLICE"),
                            ("WD1", "WDO1B_SLICE"),
                            ("WD2", "WDO2B_SLICE"),
                            ("WD3", "WDO3B_SLICE"),
                            ("WD0A_SLICE", "WD0"),
                            ("WD1A_SLICE", "WD1"),
                            ("WD0C_SLICE", "WD2"),
                            ("WD1C_SLICE", "WD3"),
                            ("WAD0", "WADO0B_SLICE"),
                            ("WAD1", "WADO1B_SLICE"),
                            ("WAD2", "WADO2B_SLICE"),
                            ("WAD3", "WADO3B_SLICE"),
                            ("WAD0A_SLICE", "WAD0"),
                            ("WAD1A_SLICE", "WAD1"),
                            ("WAD2A_SLICE", "WAD2"),
                            ("WAD3A_SLICE", "WAD3"),
                            ("WAD0C_SLICE", "WAD0"),
                            ("WAD1C_SLICE", "WAD1"),
                            ("WAD2C_SLICE", "WAD2"),
                            ("WAD3C_SLICE", "WAD3"),
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

    fn process_plc_machxo2(&mut self) {
        let tcid = self.intdb.get_tile_class("PLC");
        let is_ecp5 = matches!(self.chip.kind, ChipKind::Ecp5 | ChipKind::Crosslink);
        for &tcrd in &self.edev.tile_index[tcid] {
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
                    "A0", "A1", "B0", "B1", "C0", "C1", "D0", "D1", "M0", "M1", "CE",
                ] {
                    let wire_int = self.edev.get_bel_pin(slices[i], pin)[0];
                    let wn = self
                        .intdb
                        .wires
                        .key(wire_int.slot)
                        .strip_prefix("IMUX_")
                        .unwrap();
                    let wire_slice = self.rc_wire(cell, &format!("{wn}_SLICE"));
                    self.add_bel_wire(slices[i], pin, wire_slice);
                    self.claim_pip_int_in(wire_slice, wire_int);
                }
                for pin in ["CLK", "LSR"] {
                    let wire_int = self.edev.get_bel_pin(slices[i], pin)[0];
                    let wire_slice = self.rc_wire(cell, &format!("{pin}{i}_SLICE"));
                    self.add_bel_wire(slices[i], pin, wire_slice);
                    self.claim_pip_int_in(wire_slice, wire_int);
                }

                // plain outputs
                for pin in ["F0", "F1", "Q0", "Q1"] {
                    let wire_int = self.edev.get_bel_pin(slices[i], pin)[0];
                    let wn = self
                        .intdb
                        .wires
                        .key(wire_int.slot)
                        .strip_prefix("OUT_")
                        .unwrap();
                    let wire_slice = self.rc_wire(cell, &format!("{wn}_SLICE"));
                    self.add_bel_wire(slices[i], pin, wire_slice);
                    self.claim_pip_int_out(wire_int, wire_slice);
                }

                // F5, FX
                let ofx0_int = self
                    .edev
                    .get_bel_pin(slices[i], if is_ecp5 { "F0" } else { "OFX0" })[0];
                let ofx1_int = self
                    .edev
                    .get_bel_pin(slices[i], if is_ecp5 { "F1" } else { "OFX1" })[0];
                let ofx0_slice = self.rc_wire(cell, &format!("F5{abcd}_SLICE"));
                let ofx1_slice = self.rc_wire(cell, &format!("FX{abcd}_SLICE"));
                self.add_bel_wire(slices[i], "OFX0", ofx0_slice);
                self.add_bel_wire(slices[i], "OFX1", ofx1_slice);
                self.claim_pip_int_out(ofx0_int, ofx0_slice);
                self.claim_pip_int_out(ofx1_int, ofx1_slice);

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

                let (ia, ib) = [(2, 0), (5, 1), (6, 4), (3, 3)][i];
                if i == 3 {
                    if let Some(&fxa_int) =
                        self.naming
                            .interconnect
                            .get(&cell.wire(self.intdb.get_wire(if is_ecp5 {
                                "OUT_F3_W"
                            } else {
                                "OUT_OFX3_W"
                            })))
                    {
                        self.claim_pip(fxa, fxa_int);
                    }
                } else {
                    let fxa_int = cell.wire(self.intdb.get_wire(&if is_ecp5 {
                        format!("OUT_F{ia}")
                    } else {
                        format!("OUT_OFX{ia}")
                    }));
                    self.claim_pip_int_in(fxa, fxa_int);
                }
                let fxb_int = cell.wire(self.intdb.get_wire(&if is_ecp5 {
                    format!("OUT_F{ib}")
                } else {
                    format!("OUT_OFX{ib}")
                }));
                self.claim_pip_int_in(fxb, fxb_int);

                // DI
                let di0 = self.rc_wire(cell, &format!("DI{ii}", ii = 2 * i));
                let di0_slice = self.rc_wire(cell, &format!("DI{ii}_SLICE", ii = 2 * i));
                let di1 = self.rc_wire(cell, &format!("DI{ii}", ii = 2 * i + 1));
                let di1_slice = self.rc_wire(cell, &format!("DI{ii}_SLICE", ii = 2 * i + 1));
                self.add_bel_wire(slices[i], "DI0", di0);
                self.add_bel_wire(slices[i], "DI1", di1);
                self.add_bel_wire(slices[i], "DI0_SLICE", di0_slice);
                self.add_bel_wire(slices[i], "DI1_SLICE", di1_slice);
                self.claim_pip(di0_slice, di0);
                self.claim_pip(di1_slice, di1);
                let f0 = self.edev.get_bel_pin(slices[i], "F0")[0];
                let f1 = self.edev.get_bel_pin(slices[i], "F1")[0];
                self.claim_pip_int_in(di0, f0);
                self.claim_pip_int_in(di1, f1);
                if !is_ecp5 {
                    self.claim_pip_int_in(di0, ofx0_int);
                    self.claim_pip_int_in(di1, ofx1_int);
                }

                for pin in [
                    "WAD0", "WAD1", "WAD2", "WAD3", "WADO0", "WADO1", "WADO2", "WADO3", "WD0",
                    "WD1", "WDO0", "WDO1", "WDO2", "WDO3",
                ] {
                    let wire = self.rc_wire(cell, &format!("{pin}{abcd}_SLICE"));
                    self.add_bel_wire(slices[i], pin, wire);
                }
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
                (0, "WCK", "WCK0_SLICE"),
                (1, "WCK", "WCK1_SLICE"),
                (2, "WCK", "WCK2_SLICE"),
                (3, "WCK", "WCK3_SLICE"),
                (0, "WRE", "WRE0_SLICE"),
                (1, "WRE", "WRE1_SLICE"),
                (2, "WRE", "WRE2_SLICE"),
                (3, "WRE", "WRE3_SLICE"),
                (2, "WD0_OUT", "WD0"),
                (2, "WD1_OUT", "WD1"),
                (2, "WD2_OUT", "WD2"),
                (2, "WD3_OUT", "WD3"),
                (2, "WAD0_OUT", "WAD0"),
                (2, "WAD1_OUT", "WAD1"),
                (2, "WAD2_OUT", "WAD2"),
                (2, "WAD3_OUT", "WAD3"),
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

            for (wt, wf) in [
                ("WD0", "WDO0C_SLICE"),
                ("WD1", "WDO1C_SLICE"),
                ("WD2", "WDO2C_SLICE"),
                ("WD3", "WDO3C_SLICE"),
                ("WD0A_SLICE", "WD0"),
                ("WD1A_SLICE", "WD1"),
                ("WD0B_SLICE", "WD2"),
                ("WD1B_SLICE", "WD3"),
                ("WAD0", "WADO0C_SLICE"),
                ("WAD1", "WADO1C_SLICE"),
                ("WAD2", "WADO2C_SLICE"),
                ("WAD3", "WADO3C_SLICE"),
                ("WAD0A_SLICE", "WAD0"),
                ("WAD1A_SLICE", "WAD1"),
                ("WAD2A_SLICE", "WAD2"),
                ("WAD3A_SLICE", "WAD3"),
                ("WAD0B_SLICE", "WAD0"),
                ("WAD1B_SLICE", "WAD1"),
                ("WAD2B_SLICE", "WAD2"),
                ("WAD3B_SLICE", "WAD3"),
            ] {
                let wt = self.rc_wire(cell, wt);
                let wf = self.rc_wire(cell, wf);
                self.claim_pip(wt, wf);
            }

            if is_ecp5 {
                for (wt, wf) in [
                    ("WCK0_SLICE", "WCK"),
                    ("WRE0_SLICE", "WRE"),
                    ("WCK1_SLICE", "WCK"),
                    ("WRE1_SLICE", "WRE"),
                ] {
                    let wt = self.rc_wire(cell, wt);
                    let wf = self.edev.get_bel_pin(slices[2], wf)[0];
                    self.claim_pip_int_in(wt, wf);
                }
            } else {
                for (wt, wf) in [
                    ("WCK0_SLICE", "CLK"),
                    ("WRE0_SLICE", "LSR"),
                    ("WCK1_SLICE", "CLK"),
                    ("WRE1_SLICE", "LSR"),
                ] {
                    let wt = self.rc_wire(cell, wt);
                    let wf = self.edev.get_bel_pin(slices[2], wf)[0];
                    self.claim_pip_int_in(wt, wf);
                }
            }
        }
    }

    pub fn process_plc(&mut self) {
        match self.chip.kind {
            ChipKind::Scm | ChipKind::Ecp | ChipKind::Xp | ChipKind::MachXo => {
                self.process_plc_ecp()
            }
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 | ChipKind::Ecp3 | ChipKind::Ecp3A => {
                self.process_plc_ecp2()
            }
            ChipKind::MachXo2(_) | ChipKind::Ecp4 | ChipKind::Ecp5 | ChipKind::Crosslink => {
                self.process_plc_machxo2()
            }
        }
    }
}
