use std::collections::{BTreeMap, BTreeSet};

use prjcombine_ecp::{bels, chip::RowKind, cslots, tslots};
use prjcombine_interconnect::{
    db::{BelInfo, ConnectorWire, Mux, SwitchBox, SwitchBoxItem, TileWireCoord},
    dir::DirH,
    grid::{CellCoord, DieId},
};
use prjcombine_entity::{EntityId, EntityVec};

use crate::{ChipContext, chip::ChipExt};

impl ChipContext<'_> {
    pub(super) fn process_int_io_wires(&mut self) {
        let mut col_group = EntityVec::new();
        let mut idx = 0;
        for (col, cd) in &self.chip.columns {
            if cd.pclk_drive || col == self.chip.col_clk {
                idx += 1;
            }
            col_group.push(idx);
        }
        for cell in self.edev.die_cells(DieId::from_idx(0)) {
            let bcrd = cell.bel(bels::IO_INT);
            if self.edev.has_bel(bcrd) {
                self.name_bel_null(bcrd);
            }
        }
        for wn in self.unclaimed_nodes.clone() {
            let suffix = self.naming.strings[wn.suffix].as_str();
            if suffix.starts_with("HMAW")
                || suffix.starts_with("HMAE")
                || suffix.starts_with("VMAS")
                || suffix.starts_with("VMAN")
            {
                assert_eq!(suffix.len(), 8);
                let idx: usize = suffix[6..8].parse().unwrap();
                let mut seg: usize = if &suffix[4..6] == "O7" {
                    8
                } else {
                    suffix[4..6].parse().unwrap()
                };
                let mut cell = self.chip.xlat_rc_wire(wn);
                let dir = match &suffix[..4] {
                    "HMAW" => {
                        assert_eq!(cell.row, self.chip.row_s());
                        if cell.col < self.chip.col_w() + 2 {
                            cell.col = self.chip.col_w() + 2;
                        }
                        if cell.col > self.chip.col_e() - 2 {
                            cell.col = self.chip.col_e() - 2;
                            seg += 1;
                        }
                        DirH::W
                    }
                    "HMAE" => {
                        assert_eq!(cell.row, self.chip.row_s());
                        if cell.col < self.chip.col_w() + 2 {
                            cell.col = self.chip.col_w() + 2;
                            seg += 1;
                        }
                        if cell.col > self.chip.col_e() - 2 {
                            cell.col = self.chip.col_e() - 2;
                        }
                        DirH::E
                    }
                    "VMAS" => {
                        if cell.row == self.chip.row_s() {
                            cell.row += 1;
                        }
                        if self.chip.rows[cell.row].kind == RowKind::Ebr {
                            cell.row -= 1;
                            seg += 1;
                        }
                        if cell.row >= self.chip.row_n() - 12 {
                            cell.row = self.chip.row_n() - 13;
                            seg += 1;
                        }
                        if cell.col == self.chip.col_w() {
                            DirH::E
                        } else {
                            DirH::W
                        }
                    }
                    "VMAN" => {
                        if cell.row == self.chip.row_s() {
                            cell.row += 1;
                            seg += 1;
                        }
                        if self.chip.rows[cell.row].kind == RowKind::Ebr {
                            cell.row -= 1;
                        }
                        if cell.row >= self.chip.row_n() - 12 {
                            cell.row = self.chip.row_n() - 13;
                        }
                        if cell.col == self.chip.col_w() {
                            DirH::W
                        } else {
                            DirH::E
                        }
                    }
                    _ => unreachable!(),
                };
                while seg > 0
                    && let Some(conn) = self.edev[cell].conns.get(match dir {
                        DirH::W => cslots::IO_E,
                        DirH::E => cslots::IO_W,
                    })
                    && let target = conn.target.unwrap()
                    && (cell.row == self.chip.row_s()) == (target.row == self.chip.row_s())
                {
                    cell = target;
                    seg -= 1;
                }
                let mut wires = vec![];
                while seg <= 8 {
                    let wire = cell.wire(self.intdb.get_wire(&format!("IO_{dir}{idx}_{seg}")));
                    wires.push(wire);
                    if let Some(conn) = self.edev[cell].conns.get(match dir {
                        DirH::W => cslots::IO_W,
                        DirH::E => cslots::IO_E,
                    }) && let target = conn.target.unwrap()
                        && (cell.row == self.chip.row_s()) == (target.row == self.chip.row_s())
                    {
                        cell = target;
                        seg += 1;
                    } else {
                        break;
                    }
                }
                self.claim_node(wn);
                for &wire in &wires {
                    self.add_bel_wire_no_claim(
                        wire.cell.bel(bels::IO_INT),
                        self.intdb.wires.key(wire.slot),
                        wn,
                    );
                    self.io_int_names.insert(wire, wn);
                }
                self.io_int_wires.insert(wn, wires);
            } else if matches!(suffix, "HMTW0001" | "HMTE0001" | "VMTS0001" | "VMTN0001") {
                let mut cell = self.chip.xlat_rc_wire(wn);
                let dir = match suffix {
                    "HMTW0001" => {
                        assert_eq!(cell.row, self.chip.row_s());
                        DirH::W
                    }
                    "HMTE0001" => {
                        assert_eq!(cell.row, self.chip.row_s());
                        if cell.col < self.chip.col_w() + 2 {
                            cell.col = self.chip.col_w() + 2;
                        }
                        if cell.col > self.chip.col_e() - 2 {
                            cell.col = self.chip.col_e() - 2;
                        }
                        DirH::E
                    }
                    "VMTS0001" => {
                        if cell.col == self.chip.col_w() {
                            DirH::E
                        } else {
                            DirH::W
                        }
                    }
                    "VMTN0001" => {
                        if cell.col == self.chip.col_w() {
                            DirH::W
                        } else {
                            DirH::E
                        }
                    }
                    _ => unreachable!(),
                };
                while let Some(conn) = self.edev[cell].conns.get(match dir {
                    DirH::W => cslots::IO_E,
                    DirH::E => cslots::IO_W,
                }) && let target = conn.target.unwrap()
                    && (cell.row == self.chip.row_s()) == (target.row == self.chip.row_s())
                    && col_group[cell.col] == col_group[target.col]
                {
                    cell = target;
                }
                let mut wires = vec![];
                loop {
                    let wire = cell.wire(self.intdb.get_wire(&format!("IO_T_{dir}")));
                    wires.push(wire);
                    if let Some(conn) = self.edev[cell].conns.get(match dir {
                        DirH::W => cslots::IO_W,
                        DirH::E => cslots::IO_E,
                    }) && let target = conn.target.unwrap()
                        && (cell.row == self.chip.row_s()) == (target.row == self.chip.row_s())
                        && col_group[cell.col] == col_group[target.col]
                    {
                        cell = target;
                    } else {
                        break;
                    }
                }
                self.claim_node(wn);
                for &wire in &wires {
                    self.add_bel_wire_no_claim(
                        wire.cell.bel(bels::IO_INT),
                        self.intdb.wires.key(wire.slot),
                        wn,
                    );
                    self.io_int_names.insert(wire, wn);
                }
                self.io_int_wires.insert(wn, wires);
            } else if let Some(idx) = suffix.strip_prefix("JFMPIC_") {
                let idx: usize = idx.parse().unwrap();
                let wire = self.intdb.get_wire(&format!("OUT_IO{idx}"));
                let cell = self.chip.xlat_rc_wire(wn);
                self.add_bel_wire(cell.bel(bels::IO_INT), format!("OUT_IO{idx}"), wn);
                let wire = cell.wire(wire);
                self.io_int_wires.insert(wn, vec![wire]);
                self.io_int_names.insert(wire, wn);
            } else if let Some(idx) = suffix.strip_prefix("JTOPIC_") {
                let idx: usize = idx.parse().unwrap();
                let wire = self.intdb.get_wire(&format!("IMUX_IO{idx}"));
                let cell = self.chip.xlat_rc_wire(wn);
                self.add_bel_wire(cell.bel(bels::IO_INT), format!("IMUX_IO{idx}"), wn);
                let wire = cell.wire(wire);
                self.io_int_wires.insert(wn, vec![wire]);
                self.io_int_names.insert(wire, wn);
            } else if suffix.starts_with("JFMCIB_") {
                let cell = self.chip.xlat_rc_wire(wn);
                let wire_int = self.pips_bwd[&wn]
                    .iter()
                    .copied()
                    .find(|&w| self.int_wires.contains_key(&w))
                    .unwrap();
                let iwire = self.int_wires[&wire_int][0];
                assert!(self.intdb.wires.key(iwire.slot).starts_with("IMUX_"));
                self.claim_pip(wn, wire_int);
                assert_eq!(iwire.cell, cell);
                self.add_bel_wire(cell.bel(bels::IO_INT), self.intdb.wires.key(iwire.slot), wn);
                self.io_int_wires.insert(wn, vec![iwire]);
                self.io_int_names.insert(iwire, wn);
            } else if suffix.starts_with("JTOCIB_") {
                let cell = self.chip.xlat_rc_wire(wn);
                let wire_int = self.pips_fwd[&wn]
                    .iter()
                    .copied()
                    .find(|&w| self.int_wires.contains_key(&w))
                    .unwrap();
                let iwire = self.int_wires[&wire_int][0];
                assert!(self.intdb.wires.key(iwire.slot).starts_with("OUT_"));
                self.claim_pip(wire_int, wn);
                assert_eq!(iwire.cell, cell);
                self.add_bel_wire(cell.bel(bels::IO_INT), self.intdb.wires.key(iwire.slot), wn);
                self.io_int_wires.insert(wn, vec![iwire]);
                self.io_int_names.insert(iwire, wn);
            }
        }
    }

    pub(super) fn process_int_io_conn_hv(&mut self) {
        for (col, row, slot) in [
            (self.chip.col_w(), self.chip.row_s() + 1, cslots::IO_E),
            (self.chip.col_w() + 2, self.chip.row_s(), cslots::IO_W),
            (self.chip.col_e(), self.chip.row_s() + 1, cslots::IO_W),
            (self.chip.col_e() - 2, self.chip.row_s(), cslots::IO_E),
        ] {
            let cell = CellCoord::new(DieId::from_idx(0), col, row);
            let conn = &self.edev[cell].conns[slot];
            let ccls = &self.intdb[conn.class];
            let target = conn.target.unwrap();
            for (wt, &wf) in &ccls.wires {
                let ConnectorWire::Pass(wf) = wf else {
                    unreachable!()
                };
                let wt = self.io_int_names[&cell.wire(wt)];
                let wf = self.io_int_names[&target.wire(wf)];
                self.claim_pip(wt, wf);
            }
        }
        for (col, cd) in &self.chip.columns {
            if col == self.chip.col_clk || cd.pclk_drive {
                let cell_e = CellCoord::new(DieId::from_idx(0), col, self.chip.row_s());
                let cell_w = cell_e.delta(-1, 0);
                for (cell_to, cell_from, w) in
                    [(cell_w, cell_e, "IO_T_W"), (cell_e, cell_w, "IO_T_E")]
                {
                    let wire = self.intdb.get_wire(w);
                    let wt = self.io_int_names[&cell_to.wire(wire)];
                    let wf = self.io_int_names[&cell_from.wire(wire)];
                    self.claim_pip(wt, wf);
                }
            }
        }
    }

    pub(super) fn process_int_io_pips(&mut self) {
        for tcname in ["IO_INT_W", "IO_INT_E", "IO_INT_S"] {
            let tcid = self.intdb.get_tile_class(tcname);
            let mut sb_pips: BTreeMap<TileWireCoord, BTreeSet<TileWireCoord>> = BTreeMap::new();
            for &(wt, wf) in &self.unclaimed_pips {
                let Some(wires_t) = self.io_int_wires.get(&wt) else {
                    continue;
                };
                let Some(wires_f) = self.io_int_wires.get(&wf) else {
                    continue;
                };
                let wt = wires_t[0];
                let Some(wf) = wires_f.iter().copied().find(|w| w.cell == wt.cell) else {
                    continue;
                };
                if self.edev[wt.cell.tile(tslots::BEL)].class != tcid {
                    continue;
                }

                let wt = TileWireCoord::new_idx(0, wt.slot);
                let wf = TileWireCoord::new_idx(0, wf.slot);
                sb_pips.entry(wt).or_default().insert(wf);
            }
            let mut missing_dst = BTreeSet::new();
            let mut missing_src = BTreeSet::new();
            for &tcrd in &self.edev.tile_index[tcid] {
                let bcrd = tcrd.bel(bels::IO_INT);
                for (&wt, wfs) in &sb_pips {
                    let Some(wt) = self
                        .naming
                        .try_bel_wire(bcrd, self.intdb.wires.key(wt.wire))
                    else {
                        missing_dst.insert(bcrd.wire(wt.wire));
                        continue;
                    };
                    for &wf in wfs {
                        let Some(wf) = self
                            .naming
                            .try_bel_wire(bcrd, self.intdb.wires.key(wf.wire))
                        else {
                            missing_src.insert(bcrd.wire(wf.wire));
                            continue;
                        };
                        self.claim_pip(wt, wf);
                    }
                }
            }
            for wire in missing_src {
                if missing_dst.contains(&wire) {
                    println!(
                        "{name}: OOPS missing {wire}",
                        name = self.name,
                        wire = wire.to_string(self.intdb)
                    );
                }
            }
            let mut sb = SwitchBox::default();
            for (dst, src) in sb_pips {
                let mux = Mux {
                    dst,
                    src: src.into_iter().map(|w| w.pos()).collect(),
                };
                sb.items.push(SwitchBoxItem::Mux(mux));
            }
            self.bels
                .insert((tcid, bels::IO_INT), BelInfo::SwitchBox(sb));
        }
    }
}
