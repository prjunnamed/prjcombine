use std::collections::{BTreeMap, BTreeSet};

use prjcombine_ecp::{
    bels,
    chip::{ChipKind, RowKind},
    regions, tslots,
};
use prjcombine_interconnect::{
    db::{
        Bel, BelInfo, BelPin, Buf, CellSlotId, ConnectorWire, Mux, SwitchBox, SwitchBoxItem,
        TileClassId, TileWireCoord, WireId, WireKind,
    },
    dir::Dir,
    grid::{BelCoord, CellCoord, WireCoord},
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
            let max = if self.chip.kind.has_x0_branch() { 1 } else { 0 };
            while seg <= max {
                let name = if self.chip.kind.has_x0_branch() {
                    format!("X0_{dir}{idx}_{seg}")
                } else {
                    format!("X0_{dir}{idx}")
                };
                let wire = cell.wire(self.intdb.get_wire(&name));
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
        } else if (suffix.starts_with("H0") || suffix.starts_with("V0")) && &suffix[3..4] != "M" {
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
            let idx: u8 = suffix[4..6].parse().unwrap();
            let is_sclk = suffix.starts_with("HSBX");
            let w = if is_sclk {
                format!("SCLK{idx}")
            } else {
                format!("PCLK{idx}")
            };
            if is_sclk && self.chip.kind.has_distributed_sclk() {
                if suffix.ends_with("00") {
                    return vec![];
                }
                assert!(suffix.ends_with("01"));
            } else {
                assert!(suffix.ends_with("00"));
            }
            let wire = self.intdb.get_wire(&w);
            let cell = self.chip.xlat_rc_wire(wn);
            match self.chip.kind {
                ChipKind::Ecp | ChipKind::Xp | ChipKind::MachXo => {
                    let wire = cell.wire(wire);
                    let wire = self.edev.egrid.resolve_wire(wire).unwrap();
                    self.edev.egrid.wire_tree(wire)
                }
                ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => {
                    if is_sclk {
                        let WireKind::Regional(region) = self.intdb.wires[wire] else {
                            unreachable!()
                        };
                        let root = self.edev.egrid[cell].region_root[region];
                        let mut cell_start = cell;
                        let mut cell_end = cell;
                        while let Some(cell_new) = self.edev.egrid.cell_delta(cell_start, -1, 0)
                            && self.edev.egrid.has_bel(cell_new.bel(bels::INT))
                            && self.edev.egrid[cell_new].region_root[region] == root
                        {
                            cell_start = cell_new;
                        }
                        while let Some(cell_new) = self.edev.egrid.cell_delta(cell_end, 1, 0)
                            && self.edev.egrid.has_bel(cell_new.bel(bels::INT))
                            && self.edev.egrid[cell_new].region_root[region] == root
                        {
                            cell_end = cell_new;
                        }
                        if self.chip.kind == ChipKind::Xp2
                            && self.chip.rows[cell.row].kind == RowKind::Ebr
                            && matches!(suffix.as_str(), "HSBX0101" | "HSBX0501")
                            && cell.col.to_idx() < 6
                        {
                            // toolchain bug or actual hardware issue? you tell me.
                            if cell.col.to_idx() < 2 {
                                cell_end.col -= 4;
                            } else {
                                cell_start.col += 2;
                            }
                        }
                        cell_start
                            .col
                            .range(cell_end.col + 1)
                            .map(|col| cell.with_col(col).wire(wire))
                            .collect()
                    } else {
                        let (col_start, col_end) = self.pclk_cols[cell.col];
                        col_start
                            .range(col_end)
                            .map(|col| cell.with_col(col).wire(wire))
                            .collect()
                    }
                }
                ChipKind::Ecp3 | ChipKind::Ecp3A | ChipKind::MachXo2(_) => {
                    let WireKind::Regional(region) = self.intdb.wires[wire] else {
                        unreachable!()
                    };
                    let root = self.edev.egrid[cell].region_root[region];
                    let mut cell_start = cell;
                    let mut cell_end = cell;
                    while let Some(cell_new) = self.edev.egrid.cell_delta(cell_start, -1, 0)
                        && self.edev.egrid[cell_new].region_root[region] == root
                    {
                        cell_start = cell_new;
                    }
                    while let Some(cell_new) = self.edev.egrid.cell_delta(cell_end, 1, 0)
                        && self.edev.egrid[cell_new].region_root[region] == root
                    {
                        cell_end = cell_new;
                    }
                    cell_start
                        .col
                        .range(cell_end.col + 1)
                        .map(|col| cell.with_col(col).wire(wire))
                        .collect()
                }
            }
        } else if suffix.starts_with("HSSX") && suffix.len() == 8 {
            assert!(suffix.ends_with("00"));
            let cell = self.hsdclk_locs[&wn];
            let idx: usize = suffix[4..6].parse().unwrap();
            let idx = if self.chip.kind.has_distributed_sclk_ecp3() {
                let dx = (idx + 4 - self.chip.col_sclk_idx(cell.col)) % 4;
                idx / 4 * 4 + dx
            } else {
                assert_eq!(idx % 4, self.chip.col_sclk_idx(cell.col));
                idx / 4 * 4
            };
            let wire = self.intdb.get_wire(&format!("HSDCLK{idx}"));
            let wire = cell.wire(wire);
            let wire = self.edev.egrid.resolve_wire(wire).unwrap();
            self.edev.egrid.wire_tree(wire)
        } else if suffix.starts_with("VSTX") && suffix.len() == 8 {
            assert!(suffix.ends_with("00"));
            let idx: usize = suffix[4..6].parse().unwrap();
            let cell = self.chip.xlat_rc_wire(wn);
            let idx = if self.chip.kind.has_distributed_sclk_ecp3() {
                let is_w = cell.col == self.chip.col_w();
                match idx {
                    2 => 0,
                    3 => 1,
                    6 if is_w => 6,
                    7 if is_w => 7,
                    6 => 2,
                    7 => 3,
                    10 => 4,
                    11 => 5,
                    _ => unreachable!(),
                }
            } else {
                match idx {
                    0 => 0,
                    1 => 1,
                    2 => 0,
                    3 => 1,
                    _ => unreachable!(),
                }
            };
            let wire = self.intdb.get_wire(&format!("VSDCLK{idx}"));
            let wire = cell.wire(wire);
            let wire = self.edev.egrid.resolve_wire(wire).unwrap();
            self.edev.egrid.wire_tree(wire)
        } else if self.chip.kind.has_x0_branch()
            && let Some(w) = match suffix.as_str() {
                "HL7W0000" if self.chip.kind.has_ecp_plc() => Some("OUT_OFX3_W"),
                "HL7W0001" if !self.chip.kind.has_ecp_plc() => Some("OUT_OFX3_W"),
                "HF0W0000" => Some("OUT_F0_W"),
                "HF1W0000" => Some("OUT_F1_W"),
                "HF2W0000" => Some("OUT_F2_W"),
                _ => None,
            }
        {
            let cell = self.chip.xlat_rc_wire(wn).delta(-1, 0);
            vec![cell.wire(self.intdb.get_wire(w))]
        } else if matches!(suffix.as_str(), "GND" | "VCC") {
            let w = if suffix == "VCC" { "TIE1" } else { "TIE0" };
            let cell = self.chip.xlat_rc_wire(wn);
            let w = self.intdb.get_wire(w);
            if matches!(self.chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A)
                && cell.row <= self.chip.row_s() + 9
            {
                if cell.col < self.chip.col_clk {
                    Vec::from_iter(
                        self.chip
                            .row_s()
                            .range(self.chip.row_s() + 10)
                            .flat_map(|row| self.edev.egrid.row(cell.die, row))
                            .filter(|cell| cell.col < self.chip.col_clk)
                            .map(|cell| cell.wire(w)),
                    )
                } else {
                    Vec::from_iter(
                        self.chip
                            .row_s()
                            .range(self.chip.row_s() + 10)
                            .flat_map(|row| self.edev.egrid.row(cell.die, row))
                            .filter(|cell| cell.col >= self.chip.col_clk)
                            .map(|cell| cell.wire(w)),
                    )
                }
            } else {
                if cell.col < self.chip.col_clk {
                    Vec::from_iter(
                        self.edev
                            .egrid
                            .row(cell.die, cell.row)
                            .filter(|cell| cell.col < self.chip.col_clk)
                            .map(|cell| cell.wire(w)),
                    )
                } else {
                    Vec::from_iter(
                        self.edev
                            .egrid
                            .row(cell.die, cell.row)
                            .filter(|cell| cell.col >= self.chip.col_clk)
                            .map(|cell| cell.wire(w)),
                    )
                }
            }
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
                "HF4E0001" if self.chip.kind.has_x0_branch() => "OUT_F4_E",
                "HF5E0001" if self.chip.kind.has_x0_branch() => "OUT_F5_E",
                "HF6E0001" if self.chip.kind.has_x0_branch() => "OUT_F6_E",
                "HF7E0001" if self.chip.kind.has_x0_branch() => "OUT_F7_E",
                "H01M0100" => "X1_H1",
                "H01M0400" => "X1_H4",
                "V01M0100" => "X1_V1",
                "V01M0400" => "X1_V4",
                _ => return vec![],
            };
            let cell = self.chip.xlat_rc_wire(wn);
            vec![cell.wire(self.intdb.get_wire(w))]
        }
    }

    fn classify_sdclk(&mut self) {
        for &(nf, nt) in self.grid.pips.keys() {
            let wfn = self.nodes[nf];
            let wtn = self.nodes[nt];
            if !self.naming.strings[wtn.suffix].starts_with("HSSX") {
                continue;
            }
            if !matches!(
                self.naming.strings[wfn.suffix].as_str(),
                "JF6" | "JCE1" | "JCLK2"
            ) {
                continue;
            }
            let cell = self.chip.xlat_rc_wire(wfn);
            self.hsdclk_locs.insert(wtn, cell);
        }
    }

    pub fn process_int_nodes(&mut self) {
        if self.chip.kind.has_distributed_sclk() {
            self.classify_sdclk();
        }
        for &wn in self.nodes.values() {
            let wires = self.classify_int_wire(wn);
            if wires.is_empty() {
                let suffix = self.naming.strings[wn.suffix].as_str();
                if matches!(
                    suffix,
                    "WESTKEEP"
                        | "EASTKEEP"
                        | "SOUTHKEEP"
                        | "SOUTHKEEP1"
                        | "NORTHKEEP"
                        | "NORTHKEEP1"
                        | "WBOUNCE"
                        | "EBOUNCE"
                        | "SBOUNCE"
                        | "NBOUNCE"
                ) {
                    self.keep_nodes.insert(wn);
                } else if matches!(
                    suffix,
                    "HF0W0000"
                        | "HF1W0000"
                        | "HF2W0000"
                        | "HF4E0001"
                        | "HF5E0001"
                        | "HF6E0001"
                        | "HF7E0001"
                        | "H01M0101"
                        | "H01M0401"
                        | "V01M0101"
                        | "V01M0401"
                ) {
                    self.discard_nodes.insert(wn);
                } else {
                    self.unclaimed_nodes.insert(wn);
                }
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
                    if !self.intdb.wires[wire.slot].is_tie() {
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
        let out_w = if self.chip.kind.has_x0_branch() {
            Vec::from_iter(["F0", "F1", "F2", "OFX3"].into_iter().map(|w| {
                (
                    self.intdb.get_wire(&format!("OUT_{w}_W")),
                    self.intdb.get_wire(&format!("OUT_{w}")),
                )
            }))
        } else {
            vec![]
        };
        let out_e = if self.chip.kind.has_x0_branch() {
            Vec::from_iter(["F4", "F5", "F6", "F7"].into_iter().map(|w| {
                (
                    self.intdb.get_wire(&format!("OUT_{w}_E")),
                    self.intdb.get_wire(&format!("OUT_{w}")),
                )
            }))
        } else {
            vec![]
        };
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
                        for &(wt, wf) in &out_w {
                            let wt = target.wire(wt);
                            let wf = cell.wire(wf);
                            let Some(&wtn) = self.naming.interconnect.get(&wt) else {
                                println!(
                                    "{name}: {wt} missing",
                                    name = self.name,
                                    wt = wt.to_string(self.intdb)
                                );
                                continue;
                            };
                            let Some(&wfn) = self.naming.interconnect.get(&wf) else {
                                println!(
                                    "{name}: {wf} missing",
                                    name = self.name,
                                    wf = wf.to_string(self.intdb)
                                );
                                continue;
                            };
                            term_pips.insert((wtn, wfn));
                        }
                    }
                    if slot == conn_e && !target_sio && !cell_sio {
                        for &(wt, wf) in &out_e {
                            let wt = target.wire(wt);
                            let wf = cell.wire(wf);
                            let Some(&wtn) = self.naming.interconnect.get(&wt) else {
                                println!(
                                    "{name}: {wt} missing",
                                    name = self.name,
                                    wt = wt.to_string(self.intdb)
                                );
                                continue;
                            };
                            let Some(&wfn) = self.naming.interconnect.get(&wf) else {
                                println!(
                                    "{name}: {wf} missing",
                                    name = self.name,
                                    wf = wf.to_string(self.intdb)
                                );
                                continue;
                            };
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
            if self.int_wires.contains_key(&wtn) && self.keep_nodes.contains(&wfn) {
                continue;
            }
            if self.discard_nodes.contains(&wtn) && self.keep_nodes.contains(&wfn) {
                continue;
            }
            if let Some(wires_f) = self.int_wires.get(&wfn)
                && let Some(wires_t) = self.int_wires.get(&wtn)
                && let mut wt = wires_t[0]
                && let wtsn = self.intdb.wires.key(wt.slot)
                && !wtsn.starts_with("SCLK")
                && !(wtsn.starts_with("HSDCLK") && self.intdb.wires[wires_f[0].slot].is_tie())
            {
                if matches!(self.intdb.wires[wt.slot], WireKind::Branch(_))
                    && !wtsn.starts_with("HSDCLK")
                {
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
                let wf = if wtsn.starts_with("HSDCLK") {
                    assert_eq!(wires_f.len(), 1);
                    wt = wires_t
                        .iter()
                        .find(|w| w.col == wires_f[0].col)
                        .copied()
                        .unwrap();
                    wires_f[0]
                } else if wtsn.starts_with("VSDCLK") {
                    let mut wf = wires_f.iter().find(|w| w.col == wt.col).copied().unwrap();
                    let wt_root = self.edev.egrid[wt.cell].region_root[regions::VSDCLK];
                    let wf_root = self.edev.egrid[wf.cell].region_root[regions::VSDCLK];
                    if wt_root == wf_root {
                        wt.cell = wf.cell;
                    } else {
                        assert_eq!(
                            self.edev.egrid[wf.cell.delta(0, -1)].region_root[regions::VSDCLK],
                            wt_root
                        );
                        wt.cell = wf.cell;
                        wt.slot = self.intdb.get_wire(&format!("{wtsn}_N"));
                    }
                    if matches!(self.chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A)
                        && self.intdb.wires[wf.slot].is_tie()
                        && wf.cell.row <= self.chip.row_s() + 9
                    {
                        wf.cell.row = self.chip.row_s();
                        if !self.edev.egrid.has_bel(wf.cell.bel(bels::INT)) {
                            wf.cell.row += 9;
                        }
                        wt.cell = wf.cell;
                    }
                    wf
                } else {
                    let wf = if let WireKind::Regional(region) = self.intdb.wires[wires_f[0].slot] {
                        let wf_root = self.edev.egrid[wires_f[0].cell].region_root[region];
                        let wt_root = self.edev.egrid[wt.cell].region_root[region];
                        let wfsn = self.intdb.wires.key(wires_f[0].slot);
                        if wf_root == wt_root {
                            Some(wt.cell.wire(wires_f[0].slot))
                        } else if wfsn.starts_with("VSDCLK")
                            && self.edev.egrid[wt.cell.delta(0, -1)].region_root[regions::VSDCLK]
                                == wf_root
                        {
                            Some(wt.cell.wire(self.intdb.get_wire(&format!("{wfsn}_N"))))
                        } else {
                            None
                        }
                    } else if self.intdb.wires[wires_f[0].slot].is_tie() {
                        Some(wt.cell.wire(wires_f[0].slot))
                    } else {
                        wires_f.iter().find(|w| w.cell == wt.cell).copied()
                    };
                    let Some(wf) = wf else {
                        println!(
                            "{name}: weird int pip {wtn} / {wt} <- {wfn} / {wf}",
                            name = self.name,
                            wtn = wtn.to_string(&self.naming),
                            wt = wt.to_string(self.intdb),
                            wfn = wfn.to_string(&self.naming),
                            wf = wires_f[0].to_string(self.intdb),
                        );
                        continue;
                    };
                    wf
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
                    if (matches!(
                        self.intdb.wires.key(wt.slot).as_str(),
                        "VSDCLK0_N" | "VSDCLK1_N"
                    ) || matches!(
                        self.intdb.wires.key(wf.slot).as_str(),
                        "VSDCLK0_N" | "VSDCLK1_N"
                    )) && tcrd.row != self.chip.row_s()
                        && !self.chip.rows[tcrd.row].sclk_break
                    {
                        continue;
                    }
                    if matches!(
                        self.intdb.wires.key(wt.slot).as_str(),
                        "VSDCLK2" | "VSDCLK3" | "VSDCLK2_N" | "VSDCLK3_N"
                    ) && tcrd.col != self.chip.col_e()
                    {
                        continue;
                    }
                    if matches!(
                        self.intdb.wires.key(wt.slot).as_str(),
                        "VSDCLK4"
                            | "VSDCLK5"
                            | "VSDCLK4_N"
                            | "VSDCLK5_N"
                            | "VSDCLK6"
                            | "VSDCLK7"
                            | "VSDCLK6_N"
                            | "VSDCLK7_N"
                    ) && tcrd.col != self.chip.col_w()
                    {
                        continue;
                    }
                    if matches!(self.intdb.wires.key(wf.slot).as_str(), "SCLK3" | "SCLK7") {
                        // ???? for some inscrutable reason these two devices are missing SCLK3
                        // in these locations in particular
                        if self.chip.kind == ChipKind::Ecp2
                            && self.chip.rows.len() == 46
                            && tcrd.col == self.chip.col_e()
                            && tcrd.row.to_idx() == 17
                        {
                            continue;
                        }
                        if self.chip.kind == ChipKind::Ecp2M
                            && self.chip.rows.len() == 75
                            && tcrd.col.to_idx() == 72
                            && tcrd.row == self.chip.row_s()
                        {
                            continue;
                        }
                    }
                    println!(
                        "{name}: MISSING PIP {wt} <- {wf}",
                        name = self.name,
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
            let mut sb = SwitchBox::default();
            for mux in muxes.into_values() {
                if mux.src.len() == 1 {
                    let src = mux.src.into_iter().next().unwrap();
                    let wtn = self.intdb.wires.key(mux.dst.wire).as_str();
                    let buf = Buf { dst: mux.dst, src };
                    if wtn.starts_with("X1") || wtn.starts_with("IMUX_M") {
                        sb.items.push(SwitchBoxItem::PermaBuf(buf));
                    } else {
                        sb.items.push(SwitchBoxItem::ProgBuf(buf));
                    }
                } else {
                    sb.items.push(SwitchBoxItem::Mux(mux));
                }
            }
            if matches!(self.chip.kind, ChipKind::MachXo2(_))
                && self.intdb.tile_classes.key(tcid) == "INT_EBR"
            {
                for i in 0..8 {
                    let vsdclk = TileWireCoord {
                        cell: CellSlotId::from_idx(0),
                        wire: self.intdb.get_wire(&format!("VSDCLK{i}")),
                    };
                    let vsdclk_n = TileWireCoord {
                        cell: CellSlotId::from_idx(0),
                        wire: self.intdb.get_wire(&format!("VSDCLK{i}_N")),
                    };
                    sb.items.push(SwitchBoxItem::ProgBuf(Buf {
                        dst: vsdclk,
                        src: vsdclk_n.pos(),
                    }));
                    sb.items.push(SwitchBoxItem::ProgBuf(Buf {
                        dst: vsdclk_n,
                        src: vsdclk.pos(),
                    }));
                }
            }
            self.bels.insert((tcid, bels::INT), BelInfo::SwitchBox(sb));
        }
    }

    pub fn try_xlat_int_wire(&mut self, bcrd: BelCoord, wire: WireName) -> Option<BelPin> {
        let tcrd = self.edev.egrid.get_tile_by_bel(bcrd);
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
                        "{name}: fail to xlat int wire {wf} in {bel}",
                        name = self.name,
                        wf = wf.to_string(self.intdb),
                        bel = bcrd.to_string(self.intdb)
                    );
                    continue;
                };
                self.claim_pip(wire, wfn);
                let wf = TileWireCoord {
                    cell,
                    wire: wf.slot,
                };
                return Some(BelPin::new_in(wf));
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
                        "{name}: fail to xlat int wire {wt} in {bel}",
                        name = self.name,
                        wt = wt.to_string(self.intdb),
                        bel = bcrd.to_string(self.intdb)
                    );
                    continue;
                };
                self.claim_pip(wtn, wire);
                let wt = TileWireCoord {
                    cell,
                    wire: wt.slot,
                };
                return Some(BelPin::new_out(wt));
            }
        }
        None
    }

    pub fn xlat_int_wire(&mut self, bcrd: BelCoord, wire: WireName) -> BelPin {
        self.try_xlat_int_wire(bcrd, wire).unwrap()
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
                            } else {
                                cell.wire(self.intdb.get_wire(&format!("IMUX_{l}{i}")))
                            };
                            self.add_bel_wire(cell.bel(bels::INT), format!("{l}{i}"), wt);
                            self.claim_pip_int_in(wt, wf);
                        }
                    }
                    for (l, n) in [("F", 8), ("Q", 8), ("OFX", 8)] {
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
                if !self.chip.kind.has_x0_branch() {
                    let wire = self.rc_wire(cell, "HL7W0001");
                    self.add_bel_wire(cell.bel(bels::INT), "SLICE2_FX_OUT", wire);
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
        if self.chip.kind.has_distributed_sclk() {
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
                            let vsdclk =
                                cell.wire(self.intdb.get_wire(&format!("VSDCLK{vsdclki}")));
                            self.claim_pip_int_in(sclk, vsdclk);
                        }
                    }
                }
                if matches!(self.chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) {
                    // ??!? special garbage wires
                    let clocks = if cell.row == self.chip.row_s() + 9
                        && cell.col == self.chip.col_clk - 1
                    {
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
            }
        }
    }
}
