use prjcombine_ecp::{
    bels,
    chip::{ChipKind, RowKind},
    tslots,
};
use prjcombine_interconnect::db::Bel;

use super::ChipContext;

mod pips;
mod wires;

impl ChipContext<'_> {
    fn process_pclk_cols(&mut self) {
        let mut ranges = vec![];
        let mut prev = self.chip.col_w();
        for (col, cd) in &self.chip.columns {
            if cd.pclk_break {
                ranges.push((prev, col));
                prev = col;
            }
        }
        ranges.push((prev, self.chip.col_e() + 1));
        for (col_w, col_e) in ranges {
            for _ in col_w.range(col_e) {
                self.pclk_cols.push((col_w, col_e));
            }
        }
    }

    fn process_cibtest(&mut self) {
        for (cell, cell_data) in self.edev.egrid.cells() {
            let Some(tile) = cell_data.tiles.get(tslots::INT) else {
                continue;
            };
            let tcname = self.intdb.tile_classes.key(tile.class);
            let has_cibtest = if self.chip.kind == ChipKind::MachXo {
                tcname == "INT_SIO_XW"
            } else {
                tcname != "INT_PLC"
            };
            if has_cibtest {
                let (r, c) = self.rc(cell);
                self.name_bel(cell.bel(bels::INT), [format!("CIBTEST_R{r}C{c}")]);
                if self.chip.kind == ChipKind::MachXo {
                    for w in [
                        "A0", "A1", "A2", "A3", "A6", "B0", "B1", "B2", "B3", "B6", "C0", "C1",
                        "C2", "C3", "C6", "D0", "D1", "D2", "D3", "D6", "M0", "M1", "CLK0", "CLK1",
                        "LSR0", "CE0",
                    ] {
                        let wt = self.rc_wire(cell, &format!("J{w}_CIBTEST"));
                        let wf = cell.wire(self.intdb.get_wire(&format!("IMUX_{w}")));
                        self.add_bel_wire(cell.bel(bels::INT), w, wt);
                        self.claim_pip_int_in(wt, wf);
                    }
                    for i in 0..12 {
                        let wf = self.rc_wire(cell, &format!("JTI{i}_CIBTEST"));
                        let wt = cell.wire(self.intdb.get_wire(&format!("OUT_TI{i}")));
                        self.add_bel_wire(cell.bel(bels::INT), format!("TI{i}"), wf);
                        self.claim_pip_int_out(wt, wf);
                    }
                    let bcrd = self.chip.bel_cibtest_sel();
                    let mut bel = Bel {
                        pins: Default::default(),
                    };
                    for pin in ["TSEL0", "TSEL1"] {
                        let wire = self.rc_wire(cell, &format!("J{pin}_CIBTEST"));
                        self.add_bel_wire(cell.bel(bels::INT), pin, wire);
                        let bpin = self.xlat_int_wire(bcrd, wire);
                        bel.pins.insert(pin.to_string(), bpin);
                    }
                    self.insert_bel(bcrd, bel);
                } else {
                    let num_clk = if matches!(
                        self.chip.kind,
                        ChipKind::Ecp4 | ChipKind::Ecp5 | ChipKind::Crosslink
                    ) {
                        2
                    } else {
                        4
                    };
                    for (l, n) in [
                        ("A", 8),
                        ("B", 8),
                        ("C", 8),
                        ("D", 8),
                        ("M", 8),
                        ("CLK", num_clk),
                        ("LSR", num_clk),
                        ("CE", 4),
                    ] {
                        for i in 0..n {
                            let wt = self.rc_wire(cell, &format!("J{l}{i}_CIBTEST"));
                            let wf = if self.chip.kind == ChipKind::Xp2
                                && l == "CLK"
                                && i == 2
                                && tcname.starts_with("INT_IO")
                                && !((cell.col == self.chip.col_w()
                                    || cell.col == self.chip.col_e())
                                    && (cell.row == self.chip.row_s()
                                        || cell.row == self.chip.row_n()))
                            {
                                cell.wire(self.intdb.get_wire("IMUX_CLK0"))
                            } else if self.chip.kind == ChipKind::Ecp4 && l == "CLK" {
                                cell.wire(self.intdb.get_wire(&format!("IMUX_{l}{i}_DELAY")))
                            } else {
                                cell.wire(self.intdb.get_wire(&format!("IMUX_{l}{i}")))
                            };
                            self.add_bel_wire(cell.bel(bels::INT), format!("{l}{i}"), wt);
                            self.claim_pip_int_in(wt, wf);
                        }
                    }
                    for (l, n) in [("F", 8), ("Q", 8), ("OFX", 8)] {
                        if l == "OFX"
                            && matches!(self.chip.kind, ChipKind::Ecp5 | ChipKind::Crosslink)
                        {
                            continue;
                        }
                        for i in 0..n {
                            let wf = self.rc_wire(cell, &format!("J{l}{i}_CIBTEST"));
                            let wt = cell.wire(self.intdb.get_wire(&format!("OUT_{l}{i}")));
                            self.add_bel_wire(cell.bel(bels::INT), format!("{l}{i}"), wf);
                            if matches!(
                                self.chip.kind,
                                ChipKind::Xp2
                                    | ChipKind::Ecp3
                                    | ChipKind::Ecp3A
                                    | ChipKind::MachXo2(_)
                                    | ChipKind::Ecp4
                                    | ChipKind::Ecp5
                                    | ChipKind::Crosslink
                            ) {
                                self.claim_pip_int_out(wt, wf);
                            } else {
                                self.claim_pip_int_in(wf, wt);
                            }
                        }
                    }
                }
            } else {
                self.name_bel_null(cell.bel(bels::INT));
            }
        }
    }

    fn process_int_misc(&mut self) {
        let conn_w = self.intdb.get_conn_slot("W");
        for (cell, cell_data) in self.edev.egrid.cells() {
            if !self.edev.egrid.has_bel(cell.bel(bels::INT)) {
                continue;
            }
            let has_cin = if matches!(self.chip.kind, ChipKind::MachXo | ChipKind::MachXo2(_)) {
                matches!(self.chip.rows[cell.row].kind, RowKind::Plc | RowKind::Fplc)
                    && cell.col != self.chip.col_e()
                    && cell.col > self.chip.col_w() + 1
            } else {
                cell_data.conns[conn_w].target.is_some()
            };
            if has_cin {
                let wire = self.rc_wire(cell, "HFIE0000");
                self.add_bel_wire(cell.bel(bels::INT), "FCI_IN", wire);
                if !self.chip.kind.has_out_ofx_branch() {
                    let wire = self.rc_wire(cell, "HL7W0001");
                    self.add_bel_wire(cell.bel(bels::INT), "SLICE2_FX_OUT", wire);
                }
            }
        }
    }

    fn process_int_delay(&mut self) {
        if self.chip.kind != ChipKind::Ecp4 {
            return;
        }
        for (cell, cell_data) in self.edev.egrid.cells() {
            let Some(tile) = cell_data.tiles.get(tslots::INT) else {
                continue;
            };
            let tcname = self.intdb.tile_classes.key(tile.class);
            if tcname == "INT_PLC" {
                continue;
            }
            let bcrd = cell.bel(bels::INT);
            for i in 0..2 {
                let clk = self.naming.interconnect
                    [&cell.wire(self.intdb.get_wire(&format!("IMUX_CLK{i}")))];
                let clk_delay = self.naming.interconnect
                    [&cell.wire(self.intdb.get_wire(&format!("IMUX_CLK{i}_DELAY")))];
                let del1 = self.rc_wire(cell, &format!("CLK{i}_DEL1"));
                let del2 = self.rc_wire(cell, &format!("CLK{i}_DEL2"));
                let del3 = self.rc_wire(cell, &format!("CLK{i}_DEL3"));
                self.add_bel_wire(bcrd, format!("CLK{i}_DEL1"), del1);
                self.add_bel_wire(bcrd, format!("CLK{i}_DEL2"), del2);
                self.add_bel_wire(bcrd, format!("CLK{i}_DEL3"), del3);
                self.claim_pip(del1, clk);
                self.claim_pip(del2, del1);
                self.claim_pip(del3, del2);
                self.claim_pip(clk_delay, clk);
                self.claim_pip(clk_delay, del1);
                self.claim_pip(clk_delay, del2);
                self.claim_pip(clk_delay, del3);
            }
        }
    }

    fn process_sclk_source(&mut self) {
        if !self.chip.kind.has_distributed_sclk() {
            return;
        }
        for (cell, cell_data) in self.edev.egrid.cells() {
            let idx = self.chip.col_sclk_idx(cell.col);
            let has_int = cell_data.tiles.contains_id(tslots::INT);
            let mut clocks = vec![(idx, 0, 0), (idx + 4, 4, 1)];
            if self.chip.kind.has_distributed_sclk_ecp3() {
                if cell.col == self.chip.col_w() {
                    let idx3 = (idx + 3) % 4;
                    let idx2 = (idx + 2) % 4;
                    clocks.extend([
                        (idx3, 1, 6),
                        (idx3 + 4, 5, 7),
                        (idx2, 2, 4),
                        (idx2 + 4, 6, 5),
                    ]);
                }
                if cell.col == self.chip.col_e() {
                    let idx1 = (idx + 1) % 4;
                    clocks.extend([(idx1, 1, 2), (idx1 + 4, 5, 3)]);
                }
            }
            if has_int {
                for (sclki, ti, vsdclki) in clocks {
                    let sclk_in = self.rc_wire(cell, &format!("HSBX0{ti}00"));
                    self.add_bel_wire(cell.bel(bels::INT), format!("SCLK{sclki}_IN"), sclk_in);
                    let sclk = cell.wire(self.intdb.get_wire(&format!("SCLK{sclki}")));
                    self.claim_pip_int_out(sclk, sclk_in);
                    let vsdclk = cell.wire(self.intdb.get_wire(&format!("VSDCLK{vsdclki}")));
                    self.claim_pip_int_in(sclk_in, vsdclk);
                }
            } else if self.chip.kind.has_distributed_sclk_ecp3() {
                for (sclki, _, vsdclki) in clocks {
                    let sclk = cell.wire(self.intdb.get_wire(&format!("SCLK{sclki}")));
                    if let Some(&sclk) = self.naming.interconnect.get(&sclk) {
                        let vsdclk = cell.wire(self.intdb.get_wire(&format!("VSDCLK{vsdclki}")));
                        self.claim_pip_int_in(sclk, vsdclk);
                    }
                }
            }
            if matches!(self.chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) {
                // ??!? special garbage wires
                let clocks =
                    if cell.row == self.chip.row_s() + 9 && cell.col == self.chip.col_clk - 1 {
                        vec![(0, 1), (4, 5)]
                    } else if cell.row == self.chip.row_s() + 9 && cell.col == self.chip.col_clk {
                        vec![(2, 2), (6, 6), (3, 1), (7, 5)]
                    } else {
                        vec![]
                    };
                for (sclki, ti) in clocks {
                    let sclk_in = self.rc_wire(cell, &format!("HSBX0{ti}00"));
                    self.add_bel_wire(cell.bel(bels::INT), format!("SCLK{sclki}_IN"), sclk_in);
                    let sclk = cell.wire(self.intdb.get_wire(&format!("SCLK{sclki}")));
                    self.claim_pip_int_out(sclk, sclk_in);
                }
            }
            if matches!(self.chip.kind, ChipKind::MachXo2(_))
                && matches!(self.chip.rows[cell.row].kind, RowKind::Io | RowKind::Ebr)
            {
                let keep = self.rc_wire(cell, "SOUTHKEEP");
                for i in 0..8 {
                    if matches!(i, 2..4) && cell.col != self.chip.col_e() {
                        continue;
                    }
                    if matches!(i, 4..8) && cell.col != self.chip.col_w() {
                        continue;
                    }
                    let vsdclk = cell.wire(self.intdb.get_wire(&format!("VSDCLK{i}")));
                    let vsdclk_n = cell.wire(self.intdb.get_wire(&format!("VSDCLK{i}_N")));
                    let idx = [2, 3, 6, 7, 10, 11, 6, 7][i];
                    let b2t = self.rc_wire(cell, &format!("VSTX{idx:02}00_B2T"));
                    let t2b = self.rc_wire(cell, &format!("VSTX{idx:02}00_T2B"));
                    self.add_bel_wire(cell.bel(bels::INT), format!("VSDCLK{i}_B2T"), b2t);
                    self.add_bel_wire(cell.bel(bels::INT), format!("VSDCLK{i}_T2B"), t2b);
                    self.claim_pip(t2b, keep);
                    self.claim_pip(b2t, keep);
                    if cell.row == self.chip.row_s() {
                        self.claim_pip_int_in(t2b, vsdclk);
                        self.claim_pip_int_out(vsdclk, b2t);
                    } else if cell.row == self.chip.row_n() {
                        self.claim_pip_int_in(b2t, vsdclk);
                        self.claim_pip_int_out(vsdclk, t2b);
                    } else {
                        self.claim_pip_int_in(t2b, vsdclk);
                        self.claim_pip_int_out(vsdclk, b2t);
                        self.claim_pip_int_in(b2t, vsdclk_n);
                        self.claim_pip_int_out(vsdclk_n, t2b);
                    }
                }
            }
        }
    }

    pub fn process_int(&mut self) {
        self.process_pclk_cols();
        self.process_int_wires();
        self.process_int_pips();
        self.process_cibtest();
        self.process_int_misc();
        self.process_int_delay();
        self.process_sclk_source();
    }
}
