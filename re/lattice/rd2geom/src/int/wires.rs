use std::collections::BTreeSet;

use prjcombine_ecp::{
    bels,
    chip::{ChipKind, RowKind},
    cslots,
};
use prjcombine_interconnect::{
    db::{BelPin, TileWireCoord, WireKind},
    dir::Dir,
    grid::{BelCoord, WireCoord},
};
use prjcombine_re_lattice_naming::WireName;
use unnamed_entity::EntityId;

use crate::{ChipContext, chip::ChipExt};

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
            let len: u8 = suffix[1..3].parse().unwrap();
            let mut idx: u8 = suffix[4..6].parse().unwrap();
            let mut seg: u8 = suffix[6..8].parse().unwrap();
            let dir = match (&suffix[0..1], &suffix[3..4]) {
                ("H", "W") => Dir::W,
                ("H", "E") => Dir::E,
                ("V", "S") => Dir::S,
                ("V", "N") => Dir::N,
                ("V", "B") if idx == 0 => {
                    idx = seg;
                    seg = 1;
                    Dir::N
                }
                ("V", "B") if idx == 1 => {
                    idx = seg;
                    seg = 0;
                    Dir::S
                }
                _ => unreachable!(),
            };
            assert!(seg <= len);
            let mut cell = self.chip.xlat_rc_wire(wn);
            let conn_fwd = self.intdb.get_conn_slot(&dir.to_string());
            let conn_bwd = self.intdb.get_conn_slot(&(!dir).to_string());
            if self.has_v01b.contains(&cell) && dir == Dir::N && len == 6 && seg == 4 {
                // hwaet.
                seg = 3;
            }
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
            let mut idx: u8 = suffix[4..6].parse().unwrap();
            if self.chip.kind == ChipKind::Crosslink {
                idx = match idx {
                    2..6 => idx - 2,
                    12..16 => idx - 12 + 4,
                    _ => unreachable!(),
                };
            }
            let is_sclk = suffix.starts_with("HSBX");
            let w = if is_sclk {
                format!("SCLK{idx}")
            } else {
                format!("PCLK{idx}")
            };
            if (is_sclk && self.chip.kind.has_distributed_sclk())
                || (!is_sclk && self.chip.kind == ChipKind::Scm)
            {
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
                ChipKind::Scm | ChipKind::Ecp4 => {
                    vec![cell.wire(wire)]
                }
                ChipKind::Ecp5 | ChipKind::Crosslink => {
                    let (col_start, col_end) = self.pclk_cols[cell.col];
                    col_start
                        .range(col_end)
                        .map(|col| cell.with_col(col).wire(wire))
                        .collect()
                }
            }
        } else if self.chip.kind == ChipKind::Scm
            && (suffix.starts_with("HSTX") || suffix.starts_with("HSTE"))
        {
            let mut cell = self.chip.xlat_rc_wire(wn);
            if suffix.starts_with("HSTE") {
                cell.col -= 1;
            }
            let idx: usize = suffix[4..6].parse().unwrap();
            assert_eq!(suffix.len(), 8);
            if suffix.starts_with("HSTE") {
                assert!(suffix.ends_with("01"));
            } else {
                assert!(suffix.ends_with("00"));
            }
            let wire = self.intdb.get_wire(&format!("SCLK{idx}"));
            let wire = cell.wire(wire);
            let mut res = vec![wire];
            if let Some(cell_w) = self.edev.egrid.cell_delta(cell, -1, 0)
                && self.edev.egrid.has_bel(cell_w.bel(bels::INT))
            {
                let wire = self.intdb.get_wire(&format!("SCLK{idx}_W"));
                let wire = cell_w.wire(wire);
                res.push(wire);
            }
            if let Some(cell_e) = self.edev.egrid.cell_delta(cell, 1, 0)
                && self.edev.egrid.has_bel(cell_e.bel(bels::INT))
            {
                let wire = self.intdb.get_wire(&format!("SCLK{idx}_E"));
                let wire = cell_e.wire(wire);
                res.push(wire);
            }
            res
        } else if (suffix.starts_with("VSBX") || suffix.starts_with("VSBB"))
            && self.chip.kind == ChipKind::Scm
        {
            assert_eq!(suffix.len(), 8);
            let idx: u8 = suffix[4..6].parse().unwrap();
            let mut seg: u8 = suffix[6..8].parse().unwrap();
            assert_eq!(idx, 0);
            if suffix.starts_with("VSBB") {
                assert_eq!(seg, 0);
                seg = 6;
            } else {
                assert!(seg < 6);
            }
            let mut cell = self.chip.xlat_rc_wire(wn);
            while seg != 0 {
                if let Some(conn) = self.edev.egrid[cell].conns.get(cslots::S)
                    && let Some(target) = conn.target
                {
                    cell = target;
                    seg -= 1;
                } else {
                    break;
                }
            }
            let mut result = vec![];
            while seg <= 6 {
                let wire = cell.wire(self.intdb.get_wire(&format!("VSDCLK{seg}")));
                result.push(wire);
                if let Some(conn) = self.edev.egrid[cell].conns.get(cslots::N)
                    && let Some(target) = conn.target
                {
                    cell = target;
                    seg += 1;
                } else {
                    break;
                }
            }
            result
        } else if (suffix.starts_with("HSSX") || suffix.starts_with("HSCX"))
            && self.chip.kind == ChipKind::Scm
        {
            assert_eq!(suffix.len(), 8);
            let idx: u8 = suffix[4..6].parse().unwrap();
            let seg: u8 = suffix[6..8].parse().unwrap();
            assert_eq!(idx, 0);
            if suffix.starts_with("HSCX") {
                assert!(matches!(seg, 1 | 2 | 4));
            } else {
                assert_eq!(seg, 3);
            }
            let cell = self.chip.xlat_rc_wire(wn);
            let wire = cell.wire(self.intdb.get_wire(&format!("HSDCLK{seg}")));
            let wire = self.edev.egrid.resolve_wire(wire).unwrap();
            self.edev.egrid.wire_tree(wire)
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
        } else if let Some(w) = match suffix.as_str() {
            "HL7W0000" if self.chip.kind.has_ecp_plc() && self.chip.kind.has_out_ofx_branch() => {
                Some("OUT_OFX3_W")
            }
            "HL7W0001" if !self.chip.kind.has_ecp_plc() && self.chip.kind.has_out_ofx_branch() => {
                if matches!(self.chip.kind, ChipKind::Ecp5 | ChipKind::Crosslink) {
                    Some("OUT_F3_W")
                } else {
                    Some("OUT_OFX3_W")
                }
            }
            "HF0W0000" if self.chip.kind.has_out_f_branch() => Some("OUT_F0_W"),
            "HF1W0000" if self.chip.kind.has_out_f_branch() => Some("OUT_F1_W"),
            "HF2W0000" if self.chip.kind.has_out_f_branch() => Some("OUT_F2_W"),
            _ => None,
        } {
            let cell = self.chip.xlat_rc_wire(wn);
            if cell.col == self.chip.col_w() {
                return vec![];
            }
            vec![cell.delta(-1, 0).wire(self.intdb.get_wire(w))]
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
                "JCLK0" if self.chip.kind == ChipKind::Ecp4 => "IMUX_CLK0_DELAY",
                "JCLK1" if self.chip.kind == ChipKind::Ecp4 => "IMUX_CLK1_DELAY",
                "MUXCLK0" => "IMUX_MUXCLK0",
                "MUXCLK1" => "IMUX_MUXCLK1",
                "MUXCLK2" => "IMUX_MUXCLK2",
                "MUXCLK3" => "IMUX_MUXCLK3",
                "MUXLSR0" => "IMUX_MUXLSR0",
                "MUXLSR1" => "IMUX_MUXLSR1",
                "MUXLSR2" => "IMUX_MUXLSR2",
                "MUXLSR3" => "IMUX_MUXLSR3",
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
                "HF4E0001" if self.chip.kind.has_out_f_branch() => "OUT_F4_E",
                "HF5E0001" if self.chip.kind.has_out_f_branch() => "OUT_F5_E",
                "HF6E0001" if self.chip.kind.has_out_f_branch() => "OUT_F6_E",
                "HF7E0001" if self.chip.kind.has_out_f_branch() => "OUT_F7_E",
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

    fn gather_v01b(&mut self) {
        let v01b = self.naming.strings.get("V01B0000").unwrap();
        for &wn in self.nodes.values() {
            if wn.suffix != v01b {
                continue;
            }
            let cell = self.chip.xlat_rc_wire(wn);
            self.has_v01b.insert(cell);
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

    pub(super) fn process_int_wires(&mut self) {
        if self.chip.kind == ChipKind::Scm {
            self.gather_v01b();
        }
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
                        | "HF0W0001"
                        | "HF1W0001"
                        | "HF2W0001"
                        | "HF4E0000"
                        | "HF5E0000"
                        | "HF6E0000"
                        | "HF7E0000"
                        | "HF4E0001"
                        | "HF5E0001"
                        | "HF6E0001"
                        | "HF7E0001"
                        | "HL7W0000"
                        | "H01M0101"
                        | "H01M0401"
                        | "V01M0101"
                        | "V01M0401"
                ) || (suffix == "HL7W0001" && self.chip.kind == ChipKind::Ecp4)
                {
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

    pub fn try_xlat_int_wire_inner(
        &mut self,
        bcrd: BelCoord,
        wire: WireName,
        filter: bool,
    ) -> Option<BelPin> {
        let tcrd = self.edev.egrid.get_tile_by_bel(bcrd);
        let tile = &self.edev.egrid[tcrd];
        for &wfn in self.pips_bwd.get(&wire).into_iter().flatten() {
            if let Some(wires_f) = self
                .int_wires
                .get(&wfn)
                .or_else(|| self.io_int_wires.get(&wfn))
            {
                let wf = wires_f[0];
                if !self.intdb.wires.key(wf.slot).starts_with("IMUX") {
                    continue;
                }
                let Some((cell, _)) = tile.cells.iter().find(|&(_cid, &cell)| cell == wf.cell)
                else {
                    if !filter {
                        println!(
                            "{name}: fail to xlat int wire {wf} in {bel}",
                            name = self.name,
                            wf = wf.to_string(self.intdb),
                            bel = bcrd.to_string(self.intdb)
                        );
                    }
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
        let mut out_wires = BTreeSet::new();
        for wtn in self.pips_fwd.get(&wire).cloned().into_iter().flatten() {
            if let Some(wires_t) = self
                .int_wires
                .get(&wtn)
                .or_else(|| self.io_int_wires.get(&wtn))
            {
                let wt = wires_t[0];
                if !self.intdb.wires.key(wt.slot).starts_with("OUT") {
                    continue;
                }
                let Some((cell, _)) = tile.cells.iter().find(|&(_cid, &cell)| cell == wt.cell)
                else {
                    if !filter {
                        println!(
                            "{name}: fail to xlat int wire {wt} in {bel}",
                            name = self.name,
                            wt = wt.to_string(self.intdb),
                            bel = bcrd.to_string(self.intdb)
                        );
                    }
                    continue;
                };
                self.claim_pip(wtn, wire);
                out_wires.insert(TileWireCoord {
                    cell,
                    wire: wt.slot,
                });
            }
        }
        if !out_wires.is_empty() {
            Some(BelPin::new_out_multi(out_wires))
        } else {
            None
        }
    }

    pub fn try_xlat_int_wire(&mut self, bcrd: BelCoord, wire: WireName) -> Option<BelPin> {
        self.try_xlat_int_wire_inner(bcrd, wire, false)
    }

    pub fn xlat_int_wire(&mut self, bcrd: BelCoord, wire: WireName) -> BelPin {
        self.try_xlat_int_wire(bcrd, wire).unwrap()
    }

    pub fn xlat_int_wire_filter(&mut self, bcrd: BelCoord, wire: WireName) -> BelPin {
        self.try_xlat_int_wire_inner(bcrd, wire, true).unwrap()
    }
}
