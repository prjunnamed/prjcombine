use std::collections::{BTreeMap, BTreeSet, btree_map};

use prjcombine_ecp::{bels, chip::ChipKind};
use prjcombine_interconnect::{
    db::{Bel, BelInfo, BelPin, Mux, PinDir, SwitchBox, SwitchBoxItem, TileWireCoord},
    dir::DirH,
};

use crate::ChipContext;

impl ChipContext<'_> {
    fn process_ebr_scm(&mut self) {
        let mut pins_out = vec![];
        let mut pins_in = vec![];
        for pin in ["DIA", "DIB"] {
            for i in 0..18 {
                pins_in.push(format!("{pin}{i}"));
            }
        }
        for pin in ["ADA", "ADB"] {
            for i in 3..14 {
                pins_in.push(format!("{pin}{i}"));
            }
        }
        for pin in ["CLKA", "CLKB", "RSTA", "RSTB", "CEA", "CEB", "WEA", "WEB"] {
            pins_in.push(pin.to_string());
        }
        for pin in ["DOA", "DOB"] {
            for i in 0..18 {
                pins_out.push(format!("{pin}{i}"));
            }
        }
        for pin in ["AE", "FF", "AF", "EF"] {
            pins_out.push(pin.to_string());
        }
        for i in 0..8 {
            pins_out.push(format!("EXTRA{i}"));
        }
        for tcname in ["EBR_W", "EBR_E"] {
            let tcid = self.intdb.get_tile_class(tcname);
            let tcls = &self.intdb.tile_classes[tcid];
            let mut ebr_from_int = BTreeMap::new();
            let mut ebr_to_int = BTreeMap::new();
            let mut x10_from_x10 = BTreeMap::new();
            let mut x10_to_ebr = BTreeMap::new();
            let mut x10_from_ebr = BTreeMap::new();
            for dir in [DirH::W, DirH::E] {
                let num = match (tcname, dir) {
                    ("EBR_W", DirH::W) => 48,
                    ("EBR_W", DirH::E) => 64,
                    ("EBR_E", DirH::W) => 64,
                    ("EBR_E", DirH::E) => 48,
                    _ => unreachable!(),
                };
                for i in 0..num {
                    let iwire_i = self.intdb.get_wire(&format!("EBR_{dir}{i}_1"));
                    let iwire_o = self.intdb.get_wire(&format!("EBR_{dir}{i}_0"));
                    let iwire_i = TileWireCoord::new_idx(0, iwire_i);
                    let iwire_o = TileWireCoord::new_idx(0, iwire_o);
                    x10_from_x10.insert(iwire_o, iwire_i);
                }
            }

            for &tcrd in &self.edev.tile_index[tcid] {
                let bcrd = tcrd.bel(bels::EBR0);
                let bcrd_int = tcrd.bel(bels::EBR_INT);
                let cell_w = bcrd.cell;
                let cell_e = bcrd.cell.delta(1, 0);
                let (r, c) = self.rc(cell_w);
                self.name_bel(bcrd, [format!("EBR_R{r}C{c}")]);
                self.name_bel_null(bcrd_int);

                let mut bel = Bel::default();

                let mut x10_out_wires = BTreeMap::new();
                let mut cur_x10_out = BTreeMap::new();
                let mut x10_in_wires = BTreeMap::new();
                let mut cur_x10_in = BTreeMap::new();

                for (&iwire_o, &iwire_i) in &x10_from_x10 {
                    let wire_i = self.ebr_wires.get(&cell_w.wire(iwire_i.wire)).copied();
                    let wire_o = self.ebr_wires.get(&cell_w.wire(iwire_o.wire)).copied();

                    if let Some(wire_i) = wire_i {
                        self.add_bel_wire(bcrd_int, iwire_i.to_string(self.intdb, tcls), wire_i);
                        x10_in_wires.insert(wire_i, iwire_i);
                        cur_x10_in.insert(iwire_i, vec![]);
                    }
                    if let Some(wire_o) = wire_o {
                        self.add_bel_wire(bcrd_int, iwire_o.to_string(self.intdb, tcls), wire_o);
                        x10_out_wires.insert(wire_o, iwire_o);
                        cur_x10_out.insert(iwire_o, vec![]);
                        if let Some(wire_i) = wire_i {
                            self.claim_pip(wire_o, wire_i);
                        }
                    }
                }

                for pin in &pins_out {
                    let wire_eint = self.rc_wire(cell_e, &format!("JCIB_{pin}"));
                    let bpin = self.xlat_int_wire(bcrd_int, wire_eint);
                    assert_eq!(bpin.dir, PinDir::Output);
                    assert_eq!(bpin.wires.len(), 1);
                    let int_wire = bpin.wires.into_iter().next().unwrap();
                    self.add_bel_wire(bcrd_int, int_wire.to_string(self.intdb, tcls), wire_eint);

                    for wn in self.pips_bwd.get(&wire_eint).cloned().into_iter().flatten() {
                        if let Some(&x10_iwire) = x10_in_wires.get(&wn) {
                            cur_x10_in.get_mut(&x10_iwire).unwrap().push(int_wire);
                            self.claim_pip(wire_eint, wn);
                        }
                    }

                    if !pin.starts_with("EXTRA") {
                        let out_wire = TileWireCoord::new_idx(
                            0,
                            self.intdb.get_wire(&format!("OUT_EBR_{pin}")),
                        );
                        bel.pins.insert(pin.into(), BelPin::new_out(out_wire));
                        match ebr_to_int.entry(out_wire) {
                            btree_map::Entry::Vacant(e) => {
                                e.insert(int_wire);
                            }
                            btree_map::Entry::Occupied(e) => {
                                assert_eq!(*e.get(), int_wire);
                            }
                        }

                        let wire = self.rc_wire(cell_w, &format!("J{pin}_EBR"));
                        self.add_bel_wire(bcrd, pin, wire);
                        let wire_out = self.rc_wire(cell_e, &format!("J{pin}"));
                        self.add_bel_wire(bcrd, format!("{pin}_OUT"), wire_out);
                        self.claim_pip(wire_out, wire);
                        self.claim_pip(wire_eint, wire_out);

                        for wn in self.pips_fwd[&wire_out].clone() {
                            if let Some(&x10_iwire) = x10_out_wires.get(&wn) {
                                cur_x10_out.get_mut(&x10_iwire).unwrap().push(out_wire);
                                self.claim_pip(wn, wire_out);
                            }
                        }
                    }
                }

                for pin in &pins_in {
                    let wire_eint = self.rc_wire(cell_e, &format!("JCIB_{pin}"));
                    let bpin = self.xlat_int_wire(bcrd_int, wire_eint);
                    assert_eq!(bpin.dir, PinDir::Input);
                    assert_eq!(bpin.wires.len(), 1);
                    let int_wire = bpin.wires.into_iter().next().unwrap();
                    self.add_bel_wire(bcrd_int, int_wire.to_string(self.intdb, tcls), wire_eint);

                    for wn in self.pips_fwd[&wire_eint].clone() {
                        if let Some(&x10_iwire) = x10_out_wires.get(&wn) {
                            cur_x10_out.get_mut(&x10_iwire).unwrap().push(int_wire);
                            self.claim_pip(wn, wire_eint);
                        }
                    }

                    let imux_wire =
                        TileWireCoord::new_idx(0, self.intdb.get_wire(&format!("IMUX_EBR_{pin}")));
                    bel.pins.insert(pin.into(), BelPin::new_out(imux_wire));
                    match ebr_from_int.entry(imux_wire) {
                        btree_map::Entry::Vacant(e) => {
                            e.insert(int_wire);
                        }
                        btree_map::Entry::Occupied(e) => {
                            assert_eq!(*e.get(), int_wire);
                        }
                    }

                    let wire = self.rc_wire(cell_w, &format!("J{pin}_EBR"));
                    self.add_bel_wire(bcrd, pin, wire);
                    let wire_in = self.rc_wire(cell_e, &format!("J{pin}"));
                    self.add_bel_wire(bcrd, format!("{pin}_IN"), wire_in);
                    self.claim_pip(wire, wire_in);
                    self.claim_pip(wire_in, wire_eint);

                    for wn in self.pips_bwd[&wire_in].clone() {
                        if let Some(&x10_iwire) = x10_in_wires.get(&wn) {
                            cur_x10_in.get_mut(&x10_iwire).unwrap().push(imux_wire);
                            self.claim_pip(wire_in, wn);
                        }
                    }
                }

                for pin in [
                    "ADA0", "ADA1", "ADA2", "ADB0", "ADB1", "ADB2", "CSA0", "CSA1", "CSA2", "CSB0",
                    "CSB1", "CSB2",
                ] {
                    let wire = self.rc_wire(cell_w, &format!("J{pin}_EBR"));
                    self.add_bel_wire(bcrd, pin, wire);
                    let wire_in = self.rc_wire(cell_e, &format!("J{pin}"));
                    self.add_bel_wire(bcrd, format!("{pin}_IN"), wire_in);
                    let wire_cib = self.rc_wire(cell_e, &format!("JCIB_{pin}"));
                    self.add_bel_wire(bcrd, format!("{pin}_CIB"), wire_cib);
                    self.claim_pip(wire, wire_in);
                    self.claim_pip(wire_in, wire_cib);
                    bel.pins
                        .insert(pin.into(), self.xlat_int_wire(bcrd, wire_cib));
                }

                for (iwire, wires) in cur_x10_in {
                    match x10_to_ebr.entry(iwire) {
                        btree_map::Entry::Vacant(e) => {
                            e.insert(wires);
                        }
                        btree_map::Entry::Occupied(e) => {
                            assert_eq!(*e.get(), wires);
                        }
                    }
                }
                for (iwire, wires) in cur_x10_out {
                    match x10_from_ebr.entry(iwire) {
                        btree_map::Entry::Vacant(e) => {
                            e.insert(wires);
                        }
                        btree_map::Entry::Occupied(e) => {
                            assert_eq!(*e.get(), wires);
                        }
                    }
                }

                self.insert_bel(bcrd, bel);
            }
            let mut sb = SwitchBox::default();
            let mut muxes: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
            for (wf, wt) in ebr_to_int {
                muxes.entry(wt).or_default().insert(wf.pos());
            }
            for (wt, wf) in ebr_from_int {
                muxes.entry(wt).or_default().insert(wf.pos());
            }
            for (wt, wf) in x10_from_x10 {
                muxes.entry(wt).or_default().insert(wf.pos());
            }
            for (wt, wfs) in x10_from_ebr {
                for wf in wfs {
                    muxes.entry(wt).or_default().insert(wf.pos());
                }
            }
            for (wf, wts) in x10_to_ebr {
                for wt in wts {
                    muxes.entry(wt).or_default().insert(wf.pos());
                }
            }
            for (dst, src) in muxes {
                let mux = Mux { dst, src };
                sb.items.push(SwitchBoxItem::Mux(mux));
            }
            self.bels
                .insert((tcid, bels::EBR_INT), BelInfo::SwitchBox(sb));
        }
    }

    fn process_ebr_ecp(&mut self) {
        let tiles = if matches!(self.chip.kind, ChipKind::MachXo2(_)) {
            ["EBR", "EBR_N"].as_slice()
        } else {
            ["EBR"].as_slice()
        };
        for &tcname in tiles {
            let tcid = self.intdb.get_tile_class(tcname);
            for &tcrd in &self.edev.tile_index[tcid] {
                let bcrd = tcrd.bel(bels::EBR0);
                let cell = if self.chip.kind == ChipKind::MachXo {
                    tcrd.cell.delta(0, 3)
                } else {
                    tcrd.cell
                };
                let (r, c) = self.rc(cell);
                self.name_bel(bcrd, [format!("EBR_R{r}C{c}")]);
                self.insert_simple_bel(bcrd, cell, "EBR");
            }
        }
    }

    fn process_ebr_ecp4(&mut self) {
        let tcid = self.intdb.get_tile_class("EBR");
        for &tcrd in &self.edev.tile_index[tcid] {
            for i in 0..4 {
                let bcrd = tcrd.bel(bels::EBR[i]);
                let cell = tcrd.delta(2 * (i as i32), 0);
                let (r, c) = self.rc(cell);
                self.name_bel(bcrd, [format!("EBR_R{r}C{c}")]);
                self.insert_simple_bel(bcrd, cell, "EBR");
            }
        }
    }

    pub fn process_ebr(&mut self) {
        match self.chip.kind {
            ChipKind::Scm => self.process_ebr_scm(),
            ChipKind::Ecp
            | ChipKind::Xp
            | ChipKind::MachXo
            | ChipKind::Ecp2
            | ChipKind::Ecp2M
            | ChipKind::Xp2
            | ChipKind::Ecp3
            | ChipKind::Ecp3A
            | ChipKind::MachXo2(_) => self.process_ebr_ecp(),
            ChipKind::Ecp4 | ChipKind::Ecp5 | ChipKind::Crosslink => self.process_ebr_ecp4(),
        }
    }
}
