use std::collections::{BTreeMap, BTreeSet};

use prjcombine_ecp::{bels, chip::ChipKind, regions, tslots};
use prjcombine_interconnect::{
    db::{
        BelInfo, Buf, CellSlotId, ConnectorWire, Mux, ProgDelay, SwitchBox, SwitchBoxItem,
        TileClassId, TileWireCoord, WireId, WireKind,
    },
    grid::CellCoord,
};
use prjcombine_re_lattice_naming::WireName;
use unnamed_entity::EntityId;

use crate::ChipContext;

impl ChipContext<'_> {
    fn collect_term_pips(&mut self) -> BTreeSet<(WireName, WireName)> {
        let mut term_pips = BTreeSet::new();
        // collect expected term pips.
        let conn_e = self.intdb.get_conn_slot("E");
        let conn_w = self.intdb.get_conn_slot("W");
        let mut out_w = vec![];
        let mut out_e = vec![];
        if self.chip.kind.has_out_f_branch() {
            out_w.extend(["F0", "F1", "F2"]);
            out_e.extend(["F4", "F5", "F6", "F7"]);
        }
        if self.chip.kind.has_out_ofx_branch() {
            if matches!(self.chip.kind, ChipKind::Ecp5 | ChipKind::Crosslink) {
                out_w.push("F3");
            } else {
                out_w.push("OFX3");
            }
        }
        let out_w = Vec::from_iter(out_w.into_iter().map(|w| {
            (
                self.intdb.get_wire(&format!("OUT_{w}_W")),
                self.intdb.get_wire(&format!("OUT_{w}")),
            )
        }));
        let out_e = Vec::from_iter(out_e.into_iter().map(|w| {
            (
                self.intdb.get_wire(&format!("OUT_{w}_E")),
                self.intdb.get_wire(&format!("OUT_{w}")),
            )
        }));
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
        term_pips
    }

    #[allow(clippy::type_complexity)]
    fn collect_int_pips(
        &mut self,
        mut term_pips: BTreeSet<(WireName, WireName)>,
    ) -> (
        BTreeMap<CellCoord, BTreeSet<(WireId, WireId)>>,
        BTreeMap<TileClassId, BTreeSet<(WireId, WireId)>>,
    ) {
        let mut cell_pips: BTreeMap<CellCoord, BTreeSet<(WireId, WireId)>> = BTreeMap::new();
        let mut sb_pips: BTreeMap<TileClassId, BTreeSet<(WireId, WireId)>> = BTreeMap::new();

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
            if self.discard_nodes.contains(&wtn) && self.int_wires.contains_key(&wfn) {
                continue;
            }
            if let Some(wires_f) = self.int_wires.get(&wfn)
                && let Some(wires_t) = self.int_wires.get(&wtn)
                && let mut wt = wires_t[0]
                && let wtsn = self.intdb.wires.key(wt.slot)
                && !wtsn.starts_with("SCLK")
                && !(wtsn.starts_with("HSDCLK") && self.intdb.wires[wires_f[0].slot].is_tie())
                && !(wtsn.ends_with("_DELAY"))
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

        (cell_pips, sb_pips)
    }

    fn create_switchboxes(
        &mut self,
        mut cell_pips: BTreeMap<CellCoord, BTreeSet<(WireId, WireId)>>,
        sb_pips: BTreeMap<TileClassId, BTreeSet<(WireId, WireId)>>,
    ) {
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
            if self.chip.kind == ChipKind::Ecp4 && self.intdb.tile_classes.key(tcid) != "INT_PLC" {
                for i in 0..2 {
                    let clk = TileWireCoord {
                        cell: CellSlotId::from_idx(0),
                        wire: self.intdb.get_wire(&format!("IMUX_CLK{i}")),
                    };
                    let clk_delay = TileWireCoord {
                        cell: CellSlotId::from_idx(0),
                        wire: self.intdb.get_wire(&format!("IMUX_CLK{i}_DELAY")),
                    };
                    sb.items.push(SwitchBoxItem::ProgDelay(ProgDelay {
                        dst: clk_delay,
                        src: clk.pos(),
                        num_steps: 4,
                    }));
                }
            }
            self.bels.insert((tcid, bels::INT), BelInfo::SwitchBox(sb));
        }
    }

    pub(super) fn process_int_pips(&mut self) {
        let term_pips = self.collect_term_pips();
        let (cell_pips, sb_pips) = self.collect_int_pips(term_pips);
        self.create_switchboxes(cell_pips, sb_pips);
    }
}
