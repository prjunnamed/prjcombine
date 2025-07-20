use std::collections::{BTreeMap, BTreeSet};

use prjcombine_ecp::{bels, chip::ChipKind, tslots};
use prjcombine_interconnect::{
    db::{
        Bel, BelInfo, BelPin, CellSlotId, ConnectorWire, Mux, PinDir, SwitchBox, SwitchBoxItem,
        TileClassId, TileWireCoord, WireId, WireKind,
    },
    dir::Dir,
    grid::{CellCoord, TileCoord, WireCoord},
};
use prjcombine_re_lattice_naming::WireName;
use unnamed_entity::EntityId;

use crate::chip::ChipExt;

use super::ChipContext;

impl ChipContext<'_> {
    fn classify_int_wire(&self, wn: WireName) -> Vec<WireCoord> {
        let suffix = &self.naming.strings[wn.suffix];
        if suffix.starts_with("H00") || suffix.starts_with("V00") {
            let mut result = vec![];
            assert_eq!(suffix.len(), 8);
            let dir = match (&suffix[0..1], &suffix[3..4]) {
                ("H", "L") => Dir::W,
                ("H", "R") => Dir::E,
                ("V", "B") => Dir::S,
                ("V", "T") => Dir::N,
                _ => unreachable!(),
            };
            let idx: u8 = suffix[4..6].parse().unwrap();
            let mut seg: u8 = suffix[6..8].parse().unwrap();
            assert!(seg <= 1);
            let mut cell = self.chip.xlat_rc_wire(wn);
            let conn_fwd = self.intdb.get_conn_slot(&dir.to_string());
            let conn_bwd = self.intdb.get_conn_slot(&(!dir).to_string());
            while seg != 0 {
                if let Some(conn) = self.edev.egrid[cell].conns.get(conn_bwd)
                    && let Some(target) = conn.target
                {
                    cell = target;
                    seg -= 1;
                } else {
                    break;
                }
            }
            while seg <= 1 {
                let wire = cell.wire(self.intdb.get_wire(&format!("X0_{dir}{idx}_{seg}")));
                result.push(wire);
                if let Some(conn) = self.edev.egrid[cell].conns.get(conn_fwd)
                    && let Some(target) = conn.target
                {
                    cell = target;
                    seg += 1;
                } else {
                    break;
                }
            }
            result
        } else if suffix.starts_with("H0") || suffix.starts_with("V0") {
            let mut result = vec![];
            assert_eq!(suffix.len(), 8);
            let dir = match (&suffix[0..1], &suffix[3..4]) {
                ("H", "W") => Dir::W,
                ("H", "E") => Dir::E,
                ("V", "S") => Dir::S,
                ("V", "N") => Dir::N,
                _ => unreachable!(),
            };
            let len: u8 = suffix[1..3].parse().unwrap();
            let idx: u8 = suffix[4..6].parse().unwrap();
            let mut seg: u8 = suffix[6..8].parse().unwrap();
            assert!(seg <= len);
            let mut cell = self.chip.xlat_rc_wire(wn);
            let conn_fwd = self.intdb.get_conn_slot(&dir.to_string());
            let conn_bwd = self.intdb.get_conn_slot(&(!dir).to_string());
            while seg != 0 {
                if let Some(conn) = self.edev.egrid[cell].conns.get(conn_bwd)
                    && let Some(target) = conn.target
                {
                    cell = target;
                    seg -= 1;
                } else {
                    break;
                }
            }
            while seg <= len {
                let wire = cell.wire(self.intdb.get_wire(&format!("X{len}_{dir}{idx}_{seg}")));
                result.push(wire);
                if let Some(conn) = self.edev.egrid[cell].conns.get(conn_fwd)
                    && let Some(target) = conn.target
                {
                    cell = target;
                    seg += 1;
                } else {
                    break;
                }
            }
            result
        } else if suffix.starts_with("HPBX") || suffix.starts_with("HSBX") {
            assert_eq!(suffix.len(), 8);
            assert!(suffix.ends_with("00"));
            let idx: u8 = suffix[4..6].parse().unwrap();
            let w = if suffix.starts_with("HPBX") {
                format!("PCLK{idx}")
            } else {
                format!("SCLK{idx}")
            };
            let wire = self.intdb.get_wire(&w);
            let cell = self.chip.xlat_rc_wire(wn);
            let wire = cell.wire(wire);
            let wire = self.edev.egrid.resolve_wire(wire).unwrap();
            self.edev.egrid.wire_tree(wire)
        } else if let Some(w) = match suffix.as_str() {
            "HL7W0000" => Some("OUT_OFX3_W"),
            "HF0W0000" => Some("OUT_F0_W"),
            "HF1W0000" => Some("OUT_F1_W"),
            "HF2W0000" => Some("OUT_F2_W"),
            _ => None,
        } {
            let cell = self.chip.xlat_rc_wire(wn).delta(-1, 0);
            vec![cell.wire(self.intdb.get_wire(w))]
        } else {
            let w = match suffix.as_str() {
                "A0" | "JA0" => "IMUX_A0",
                "A1" | "JA1" => "IMUX_A1",
                "A2" | "JA2" => "IMUX_A2",
                "A3" | "JA3" => "IMUX_A3",
                "A4" | "JA4" => "IMUX_A4",
                "A5" | "JA5" => "IMUX_A5",
                "A6" | "JA6" => "IMUX_A6",
                "A7" | "JA7" => "IMUX_A7",
                "B0" | "JB0" => "IMUX_B0",
                "B1" | "JB1" => "IMUX_B1",
                "B2" | "JB2" => "IMUX_B2",
                "B3" | "JB3" => "IMUX_B3",
                "B4" | "JB4" => "IMUX_B4",
                "B5" | "JB5" => "IMUX_B5",
                "B6" | "JB6" => "IMUX_B6",
                "B7" | "JB7" => "IMUX_B7",
                "C0" | "JC0" => "IMUX_C0",
                "C1" | "JC1" => "IMUX_C1",
                "C2" | "JC2" => "IMUX_C2",
                "C3" | "JC3" => "IMUX_C3",
                "C4" | "JC4" => "IMUX_C4",
                "C5" | "JC5" => "IMUX_C5",
                "C6" | "JC6" => "IMUX_C6",
                "C7" | "JC7" => "IMUX_C7",
                "D0" | "JD0" => "IMUX_D0",
                "D1" | "JD1" => "IMUX_D1",
                "D2" | "JD2" => "IMUX_D2",
                "D3" | "JD3" => "IMUX_D3",
                "D4" | "JD4" => "IMUX_D4",
                "D5" | "JD5" => "IMUX_D5",
                "D6" | "JD6" => "IMUX_D6",
                "D7" | "JD7" => "IMUX_D7",
                "M0" | "JM0" => "IMUX_M0",
                "M1" | "JM1" => "IMUX_M1",
                "M2" | "JM2" => "IMUX_M2",
                "M3" | "JM3" => "IMUX_M3",
                "M4" | "JM4" => "IMUX_M4",
                "M5" | "JM5" => "IMUX_M5",
                "M6" | "JM6" => "IMUX_M6",
                "M7" | "JM7" => "IMUX_M7",
                "CLK0" | "JCLK0" => "IMUX_CLK0",
                "CLK1" | "JCLK1" => "IMUX_CLK1",
                "CLK2" | "JCLK2" => "IMUX_CLK2",
                "CLK3" | "JCLK3" => "IMUX_CLK3",
                "LSR0" | "JLSR0" => "IMUX_LSR0",
                "LSR1" | "JLSR1" => "IMUX_LSR1",
                "LSR2" | "JLSR2" => "IMUX_LSR2",
                "LSR3" | "JLSR3" => "IMUX_LSR3",
                "CE0" | "JCE0" => "IMUX_CE0",
                "CE1" | "JCE1" => "IMUX_CE1",
                "CE2" | "JCE2" => "IMUX_CE2",
                "CE3" | "JCE3" => "IMUX_CE3",
                "F0" | "JF0" => "OUT_F0",
                "F1" | "JF1" => "OUT_F1",
                "F2" | "JF2" => "OUT_F2",
                "F3" | "JF3" => "OUT_F3",
                "F4" | "JF4" => "OUT_F4",
                "F5" | "JF5" => "OUT_F5",
                "F6" | "JF6" => "OUT_F6",
                "F7" | "JF7" => "OUT_F7",
                "Q0" | "JQ0" => "OUT_Q0",
                "Q1" | "JQ1" => "OUT_Q1",
                "Q2" | "JQ2" => "OUT_Q2",
                "Q3" | "JQ3" => "OUT_Q3",
                "Q4" | "JQ4" => "OUT_Q4",
                "Q5" | "JQ5" => "OUT_Q5",
                "Q6" | "JQ6" => "OUT_Q6",
                "Q7" | "JQ7" => "OUT_Q7",
                "OFX0" | "JOFX0" => "OUT_OFX0",
                "OFX1" | "JOFX1" => "OUT_OFX1",
                "OFX2" | "JOFX2" => "OUT_OFX2",
                "OFX3" | "JOFX3" => "OUT_OFX3",
                "OFX4" | "JOFX4" => "OUT_OFX4",
                "OFX5" | "JOFX5" => "OUT_OFX5",
                "OFX6" | "JOFX6" => "OUT_OFX6",
                "OFX7" | "JOFX7" => "OUT_OFX7",
                "JTI0" => "OUT_TI0",
                "JTI1" => "OUT_TI1",
                "JTI2" => "OUT_TI2",
                "JTI3" => "OUT_TI3",
                "JTI4" => "OUT_TI4",
                "JTI5" => "OUT_TI5",
                "JTI6" => "OUT_TI6",
                "JTI7" => "OUT_TI7",
                "JTI8" => "OUT_TI8",
                "JTI9" => "OUT_TI9",
                "JTI10" => "OUT_TI10",
                "JTI11" => "OUT_TI11",
                "WESTKEEP" => "KEEP_W",
                "EASTKEEP" => "KEEP_E",
                "SOUTHKEEP" => "KEEP_S0",
                "SOUTHKEEP1" => "KEEP_S1",
                "NORTHKEEP" => "KEEP_N0",
                "NORTHKEEP1" => "KEEP_N1",
                "HF4E0001" => "OUT_F4_E",
                "HF5E0001" => "OUT_F5_E",
                "HF6E0001" => "OUT_F6_E",
                "HF7E0001" => "OUT_F7_E",
                _ => return vec![],
            };
            let cell = self.chip.xlat_rc_wire(wn);
            vec![cell.wire(self.intdb.get_wire(w))]
        }
    }

    pub fn process_int_nodes(&mut self) {
        for &wn in self.nodes.values() {
            let wires = self.classify_int_wire(wn);
            if wires.is_empty() {
                self.unclaimed_nodes.insert(wn);
            } else {
                let rwire0 = self.edev.egrid.resolve_wire(wires[0]).unwrap();
                for &wire in &wires {
                    if let Some(prev) = self.naming.interconnect.insert(wire, wn) {
                        panic!(
                            "wire {w} has two names: {prev} {cur}",
                            w = wire.to_string(self.intdb),
                            prev = prev.to_string(&self.naming),
                            cur = wn.to_string(&self.naming),
                        );
                    }
                    let rwire = self.edev.egrid.resolve_wire(wire).unwrap();
                    assert_eq!(
                        rwire,
                        rwire0,
                        "wire resolve mismatch: {wn} {w0} is {rw0}; {w} is {rw}",
                        wn = wn.to_string(&self.naming),
                        w0 = wires[0].to_string(self.intdb),
                        rw0 = rwire0.to_string(self.intdb),
                        w = wire.to_string(self.intdb),
                        rw = rwire.to_string(self.intdb),
                    );
                }
                self.int_wires.insert(wn, wires);
            }
        }
    }

    pub fn process_int_pips(&mut self) {
        let mut cell_pips: BTreeMap<CellCoord, BTreeSet<(WireId, WireId)>> = BTreeMap::new();
        let mut sb_pips: BTreeMap<TileClassId, BTreeSet<(WireId, WireId)>> = BTreeMap::new();
        let mut term_pips = BTreeSet::new();
        // collect expected term pips.
        let conn_e = self.intdb.get_conn_slot("E");
        let conn_w = self.intdb.get_conn_slot("W");
        let out_w = ["F0", "F1", "F2", "OFX3"].map(|w| {
            (
                self.intdb.get_wire(&format!("OUT_{w}_W")),
                self.intdb.get_wire(&format!("OUT_{w}")),
            )
        });
        let out_e = ["F4", "F5", "F6", "F7"].map(|w| {
            (
                self.intdb.get_wire(&format!("OUT_{w}_E")),
                self.intdb.get_wire(&format!("OUT_{w}")),
            )
        });
        for (cell, cell_data) in self.edev.egrid.cells() {
            for (slot, conn) in &cell_data.conns {
                if let Some(target) = conn.target {
                    let cell_sio = self.chip.kind == ChipKind::MachXo
                        && (cell.col == self.chip.col_w()
                            || cell.col == self.chip.col_e()
                            || cell.row == self.chip.row_s()
                            || cell.row == self.chip.row_n());
                    let target_sio = self.chip.kind == ChipKind::MachXo
                        && (target.col == self.chip.col_w()
                            || target.col == self.chip.col_e()
                            || target.row == self.chip.row_s()
                            || target.row == self.chip.row_n());
                    if slot == conn_w && !target_sio && !cell_sio {
                        for (wt, wf) in out_w {
                            let wt = target.wire(wt);
                            let wf = cell.wire(wf);
                            let wtn = self.naming.interconnect[&wt];
                            let wfn = self.naming.interconnect[&wf];
                            term_pips.insert((wtn, wfn));
                        }
                    }
                    if slot == conn_e && !target_sio && !cell_sio {
                        for (wt, wf) in out_e {
                            let wt = target.wire(wt);
                            let wf = cell.wire(wf);
                            let wtn = self.naming.interconnect[&wt];
                            let wfn = self.naming.interconnect[&wf];
                            term_pips.insert((wtn, wfn));
                        }
                    }
                } else {
                    let ccls = &self.intdb.conn_classes[conn.class];
                    for (wt, &wf) in &ccls.wires {
                        let ConnectorWire::Reflect(wf) = wf else {
                            unreachable!()
                        };
                        let wt = cell.wire(wt);
                        let wf = cell.wire(wf);
                        let Some(&wtn) = self.naming.interconnect.get(&wt) else {
                            continue;
                        };
                        let Some(&wfn) = self.naming.interconnect.get(&wf) else {
                            continue;
                        };
                        term_pips.insert((wtn, wfn));
                    }
                }
            }
        }
        for &(nf, nt) in self.grid.pips.keys() {
            let wfn = self.nodes[nf];
            let wtn = self.nodes[nt];
            if term_pips.remove(&(wtn, wfn)) {
                continue;
            }
            if term_pips.remove(&(wfn, wtn)) {
                continue;
            }
            if let Some(wires_f) = self.int_wires.get(&wfn)
                && let Some(wires_t) = self.int_wires.get(&wtn)
            {
                let wt = wires_t[0];
                let wtsn = self.intdb.wires.key(wt.slot);
                if (wtsn.starts_with("X0") || wtsn.starts_with("X1"))
                    && self.intdb.wires.key(wires_f[0].slot).starts_with("KEEP")
                {
                    continue;
                }
                if matches!(self.intdb.wires[wt.slot], WireKind::Branch(_)) {
                    println!(
                        "{name}: extra term pip {wtn} / {wt} <- {wfn} / {wf}",
                        name = self.name,
                        wtn = wtn.to_string(&self.naming),
                        wt = wt.to_string(self.intdb),
                        wfn = wfn.to_string(&self.naming),
                        wf = wires_f[0].to_string(self.intdb),
                    );
                    continue;
                }
                let wf = if let WireKind::Regional(region) = self.intdb.wires[wires_f[0].slot] {
                    let wf_root = self.edev.egrid[wires_f[0].cell].region_root[region];
                    let wt_root = self.edev.egrid[wt.cell].region_root[region];
                    if wf_root == wt_root {
                        Some(wt.cell.wire(wires_f[0].slot))
                    } else {
                        None
                    }
                } else {
                    wires_f.iter().find(|w| w.cell == wt.cell).copied()
                };
                let Some(wf) = wf else {
                    println!(
                        "weird int pip {wtn} / {wt} <- {wfn} / {wf}",
                        wtn = wtn.to_string(&self.naming),
                        wt = wt.to_string(self.intdb),
                        wfn = wfn.to_string(&self.naming),
                        wf = wires_f[0].to_string(self.intdb),
                    );
                    continue;
                };
                cell_pips
                    .entry(wt.cell)
                    .or_default()
                    .insert((wt.slot, wf.slot));
                let tcid = self.edev.egrid[wt.cell.tile(tslots::INT)].class;
                sb_pips.entry(tcid).or_default().insert((wt.slot, wf.slot));
            } else {
                self.unclaimed_pips.insert((wtn, wfn));
                self.pips_bwd.entry(wtn).or_default().insert(wfn);
                self.pips_fwd.entry(wfn).or_default().insert(wtn);
            }
        }
        for (wtn, wfn) in term_pips {
            println!(
                "MISSING TERM PIP {wtn} <- {wfn}",
                wtn = wtn.to_string(&self.naming),
                wfn = wfn.to_string(&self.naming)
            );
        }
        let tc = CellSlotId::from_idx(0);
        for (tcid, pips) in sb_pips {
            for tcrd in &self.edev.egrid.tile_index[tcid] {
                let tile_pips = cell_pips.remove(&tcrd.cell).unwrap();
                for &(wt, wf) in &pips {
                    if tile_pips.contains(&(wt, wf)) {
                        // OK
                        continue;
                    }
                    let wt = tcrd.wire(wt);
                    let wf = tcrd.wire(wf);
                    if !self.naming.interconnect.contains_key(&wt)
                        && self.intdb.wires.key(wt.slot).starts_with("IMUX")
                    {
                        continue;
                    }
                    if !self.naming.interconnect.contains_key(&wf)
                        && self.intdb.wires.key(wf.slot).starts_with("OUT")
                    {
                        continue;
                    }
                    println!(
                        "MISSING PIP {wt} <- {wf}",
                        wt = wt.to_string(self.intdb),
                        wf = wf.to_string(self.intdb)
                    );
                }
            }
            let mut muxes = BTreeMap::new();
            for (wt, wf) in pips {
                let wt = TileWireCoord { cell: tc, wire: wt };
                let wf = TileWireCoord { cell: tc, wire: wf };
                muxes
                    .entry(wt)
                    .or_insert_with(|| Mux {
                        dst: wt,
                        src: BTreeSet::new(),
                    })
                    .src
                    .insert(wf.pos());
            }
            let sb = SwitchBox {
                items: muxes.into_values().map(SwitchBoxItem::Mux).collect(),
            };
            self.bels.insert((tcid, bels::INT), BelInfo::SwitchBox(sb));
        }
    }

    pub fn xlat_int_wire(&mut self, tcrd: TileCoord, wire: WireName) -> Option<BelPin> {
        let tile = &self.edev.egrid[tcrd];
        for &wfn in self.pips_bwd.get(&wire).into_iter().flatten() {
            if let Some(wires_f) = self.int_wires.get(&wfn) {
                let wf = wires_f[0];
                if !self.intdb.wires.key(wf.slot).starts_with("IMUX") {
                    continue;
                }
                let Some((cell, _)) = tile.cells.iter().find(|&(_cid, &cell)| cell == wf.cell)
                else {
                    println!(
                        "{name}: fail to xlat int wire {wf} in {tile}",
                        name = self.name,
                        wf = wf.to_string(self.intdb),
                        tile = tcrd.to_string(self.intdb)
                    );
                    continue;
                };
                self.claim_pip(wire, wfn);
                let wf = TileWireCoord {
                    cell,
                    wire: wf.slot,
                };
                return Some(BelPin {
                    wires: BTreeSet::from_iter([wf]),
                    dir: PinDir::Input,
                    is_intf_in: false,
                });
            }
        }
        for &wtn in self.pips_fwd.get(&wire).into_iter().flatten() {
            if let Some(wires_t) = self.int_wires.get(&wtn) {
                let wt = wires_t[0];
                if !self.intdb.wires.key(wt.slot).starts_with("OUT") {
                    continue;
                }
                let Some((cell, _)) = tile.cells.iter().find(|&(_cid, &cell)| cell == wt.cell)
                else {
                    println!(
                        "{name}: fail to xlat int wire {wt} in {tile}",
                        name = self.name,
                        wt = wt.to_string(self.intdb),
                        tile = tcrd.to_string(self.intdb)
                    );
                    continue;
                };
                self.claim_pip(wtn, wire);
                let wt = TileWireCoord {
                    cell,
                    wire: wt.slot,
                };
                return Some(BelPin {
                    wires: BTreeSet::from_iter([wt]),
                    dir: PinDir::Output,
                    is_intf_in: false,
                });
            }
        }
        None
    }

    pub fn process_cibtest(&mut self) {
        let conn_w = self.intdb.get_conn_slot("W");
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
                        let wf = self.intdb.get_wire(&format!("IMUX_{w}"));
                        let wf = self.naming.interconnect[&cell.wire(wf)];
                        self.add_bel_wire(cell.bel(bels::INT), w, wt);
                        self.claim_pip(wt, wf);
                    }
                    for i in 0..12 {
                        let wf = self.rc_wire(cell, &format!("JTI{i}_CIBTEST"));
                        let wt = self.intdb.get_wire(&format!("OUT_TI{i}"));
                        let wt = self.naming.interconnect[&cell.wire(wt)];
                        self.add_bel_wire(cell.bel(bels::INT), format!("TI{i}"), wf);
                        self.claim_pip(wt, wf);
                    }
                    let bel = self.chip.bel_cibtest_sel();
                    let tcrd = self.edev.egrid.get_tile_by_bel(bel);
                    let mut bel = Bel {
                        pins: Default::default(),
                    };
                    for pin in ["TSEL0", "TSEL1"] {
                        let wire = self.rc_wire(cell, &format!("J{pin}_CIBTEST"));
                        self.add_bel_wire(cell.bel(bels::INT), pin, wire);
                        let bpin = self.xlat_int_wire(tcrd, wire).unwrap();
                        bel.pins.insert(pin.to_string(), bpin);
                    }
                    self.insert_bel(tcrd.bel(bels::CIBTEST_SEL), bel);
                } else {
                    for (l, n) in [
                        ("A", 8),
                        ("B", 8),
                        ("C", 8),
                        ("D", 8),
                        ("M", 8),
                        ("CLK", 4),
                        ("LSR", 4),
                        ("CE", 4),
                    ] {
                        for i in 0..n {
                            let wt = self.rc_wire(cell, &format!("J{l}{i}_CIBTEST"));
                            let wf = self.intdb.get_wire(&format!("IMUX_{l}{i}"));
                            let wf = self.naming.interconnect[&cell.wire(wf)];
                            self.add_bel_wire(cell.bel(bels::INT), format!("{l}{i}"), wt);
                            self.claim_pip(wt, wf);
                        }
                    }
                    for (l, n) in [("F", 8), ("Q", 8), ("OFX", 8)] {
                        for i in 0..n {
                            let wt = self.rc_wire(cell, &format!("J{l}{i}_CIBTEST"));
                            let wf = self.intdb.get_wire(&format!("OUT_{l}{i}"));
                            let wf = self.naming.interconnect[&cell.wire(wf)];
                            self.add_bel_wire(cell.bel(bels::INT), format!("{l}{i}"), wt);
                            self.claim_pip(wt, wf);
                        }
                    }
                }
            } else {
                self.name_bel_null(cell.bel(bels::INT));
            }
            let has_cin = if self.chip.kind == ChipKind::MachXo {
                cell.row != self.chip.row_s()
                    && cell.row != self.chip.row_n()
                    && cell.col != self.chip.col_e()
                    && cell.col > self.chip.col_w() + 1
            } else {
                cell_data.conns[conn_w].target.is_some()
            };
            if has_cin {
                let wire = self.rc_wire(cell, "HFIE0000");
                self.add_bel_wire(cell.bel(bels::INT), "FCI_IN", wire);
            }
        }
    }
}
