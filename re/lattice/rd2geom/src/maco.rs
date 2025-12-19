use std::collections::{BTreeSet, btree_map};

use prjcombine_ecp::{
    bels,
    chip::{ChipKind, RowKind},
    cslots,
};
use prjcombine_interconnect::{
    db::{
        Bel, BelInfo, BelPin, ConnectorWire, Mux, PinDir, SwitchBox, SwitchBoxItem, TileWireCoord,
    },
    dir::{DirH, DirV},
    grid::DieId,
};
use prjcombine_entity::{EntityId, EntityPartVec};

use crate::ChipContext;

impl ChipContext<'_> {
    fn gather_ebr_wires(&mut self) {
        for (row, rd) in &self.chip.rows {
            if rd.kind != RowKind::Ebr {
                continue;
            }
            if row == self.chip.row_n() - 12 {
                continue;
            }
            for cell in self.edev.row(DieId::from_idx(0), row) {
                if cell.col < self.chip.col_w() + 4 {
                    continue;
                }
                if cell.col >= self.chip.col_e() - 3 {
                    continue;
                }
                if cell.col.to_idx() % 2 == 1 {
                    continue;
                }
                for dir in [DirH::W, DirH::E] {
                    let ab = match dir {
                        DirH::W => 'B',
                        DirH::E => 'A',
                    };
                    let (has_0, has_1) = match dir {
                        DirH::W => (
                            cell.col < self.chip.col_e() - 13,
                            cell.col >= self.chip.col_w() + 14,
                        ),
                        DirH::E => (
                            cell.col >= self.chip.col_w() + 14,
                            cell.col < self.chip.col_e() - 13,
                        ),
                    };
                    let skip_mid = match dir {
                        DirH::W => {
                            cell.col >= self.chip.col_clk && cell.col < self.chip.col_clk + 10
                        }
                        DirH::E => {
                            cell.col >= self.chip.col_clk - 10 && cell.col < self.chip.col_clk
                        }
                    };
                    let num = match (dir, cell.col < self.chip.col_clk) {
                        (DirH::W, true) => 48,
                        (DirH::W, false) => 64,
                        (DirH::E, true) => 64,
                        (DirH::E, false) => 48,
                    };
                    if has_1 {
                        for i in 0..num {
                            let iwire_1 =
                                cell.wire(self.intdb.get_wire(&format!("EBR_{dir}{i}_1")));
                            let wire_1 =
                                self.rc_wire(cell.delta(1, 1), &format!("HMA{ab}SI{i:02}"));
                            self.ebr_wires.insert(iwire_1, wire_1);
                            if has_0 && !(skip_mid && (24..40).contains(&i)) {
                                let wire_0 = self.pips_fwd[&wire_1]
                                    .iter()
                                    .copied()
                                    .find(|wn| self.naming.strings[wn.suffix].starts_with("HMA"))
                                    .unwrap();
                                let iwire_0 =
                                    cell.wire(self.intdb.get_wire(&format!("EBR_{dir}{i}_0")));
                                self.ebr_wires.insert(iwire_0, wire_0);
                            }
                        }
                    }
                }
            }
        }
    }

    fn process_ebr_conns(&mut self) {
        for (ccrd, conn) in self.edev.connectors() {
            if ccrd.slot == cslots::EBR_W || ccrd.slot == cslots::EBR_E {
                let cell_to = ccrd.cell;
                let cell_from = conn.target.unwrap();
                let ccls = &self.intdb[conn.class];
                for (wt, &wf) in &ccls.wires {
                    let ConnectorWire::Pass(wf) = wf else {
                        unreachable!()
                    };
                    let wt = cell_to.wire(wt);
                    let wf = cell_from.wire(wf);
                    let Some(&wt) = self.ebr_wires.get(&wt) else {
                        println!(
                            "{name}: OOPS {wt}",
                            name = self.name,
                            wt = wt.to_string(self.intdb)
                        );
                        continue;
                    };
                    let Some(&wf) = self.ebr_wires.get(&wf) else {
                        println!(
                            "{name}: OOPS {wf}",
                            name = self.name,
                            wf = wf.to_string(self.intdb)
                        );
                        continue;
                    };
                    self.claim_pip(wt, wf);
                }
                if self.intdb.conn_classes.key(conn.class) == "PASS_EBR_E_W" {
                    for i in 48..64 {
                        let wire_1 = self.rc_wire(cell_to.delta(1, 1), &format!("HMAASI{i:02}"));
                        self.claim_node(wire_1);
                        let wire_0 = self.claim_single_in(wire_1);
                        self.claim_node(wire_0);
                        assert!(self.naming.strings[wire_0.suffix].starts_with("HMA"));
                    }
                }
                if self.intdb.conn_classes.key(conn.class) == "PASS_EBR_W_E" {
                    for i in 48..64 {
                        let wire_1 = self.rc_wire(cell_to.delta(1, 1), &format!("HMABSI{i:02}"));
                        self.claim_node(wire_1);
                        let wire_0 = self.claim_single_in(wire_1);
                        self.claim_node(wire_0);
                        assert!(self.naming.strings[wire_0.suffix].starts_with("HMA"));
                    }
                }
            }
        }
    }

    pub fn process_maco(&mut self) {
        if self.chip.kind != ChipKind::Scm {
            return;
        }
        self.gather_ebr_wires();
        self.process_ebr_conns();
        let mut maco_rows = EntityPartVec::new();
        let mut idx = 0;
        for (row, rd) in &self.chip.rows {
            if rd.kind == RowKind::Ebr && row >= self.chip.row_clk {
                maco_rows.insert(row, ('U', idx));
                idx += 1;
            }
        }
        let mut idx = 0;
        for (row, rd) in self.chip.rows.iter().rev() {
            if rd.kind == RowKind::Ebr && row < self.chip.row_clk {
                maco_rows.insert(row, ('L', idx));
                idx += 1;
            }
        }
        for (edge, tcname) in [(DirH::W, "MACO_W"), (DirH::E, "MACO_E")] {
            let lr = match edge {
                DirH::W => 'L',
                DirH::E => 'R',
            };
            let tcid = self.intdb.get_tile_class(tcname);
            let tcls = &self.intdb[tcid];
            for &tcrd in &self.edev.tile_index[tcid] {
                let bcrd = tcrd.bel(bels::MACO);
                let bcrd_int = tcrd.bel(bels::MACO_INT);
                let cell = bcrd.delta(0, 1);
                let (ul, idx) = maco_rows[bcrd.row];
                self.name_bel(bcrd, [format!("{lr}{ul}MACO{idx}")]);
                self.name_bel_null(bcrd_int);
                let mut bel = Bel::default();
                let mut sb = SwitchBox::default();

                for i in 0..3 {
                    for pin in ["CLK", "LSR", "CE"] {
                        for j in 0..4 {
                            if i == 0 && j >= 2 && pin == "LSR" {
                                continue;
                            }

                            let wire = self.rc_io_wire(cell, &format!("J{pin}_{i}_{j}_MACO"));
                            self.add_bel_wire(bcrd, format!("{pin}_{i}_{j}"), wire);
                            bel.pins
                                .insert(format!("{pin}_{i}_{j}"), self.xlat_int_wire(bcrd, wire));
                        }
                    }
                    for j in 0..32 {
                        let wire = self.rc_io_wire(cell, &format!("JCIBIN_{i}_{j}_MACO"));
                        self.add_bel_wire(bcrd, format!("CIBIN_{i}_{j}"), wire);
                        bel.pins
                            .insert(format!("CIBIN_{i}_{j}"), self.xlat_int_wire(bcrd, wire));
                    }
                    for j in 0..24 {
                        if i == 0 && j >= 22 {
                            continue;
                        }
                        let wire = self.rc_io_wire(cell, &format!("JCIBOUT_{i}_{j}_MACO"));
                        self.add_bel_wire(bcrd, format!("CIBOUT_{i}_{j}"), wire);
                        bel.pins
                            .insert(format!("CIBOUT_{i}_{j}"), self.xlat_int_wire(bcrd, wire));
                    }
                }

                for i in 0..10 {
                    for j in 0..64 {
                        let wire = self.rc_io_wire(cell, &format!("EBRO{i}{j:02}_MACO"));
                        self.add_bel_wire(bcrd, format!("EBRO{i}{j:02}"), wire);
                        let wire_out = self.rc_io_wire(cell, &format!("EBRO{i}{j:02}"));
                        self.add_bel_wire(bcrd, format!("EBRO{i}{j:02}_OUT"), wire_out);
                        self.claim_pip(wire_out, wire);
                        let wire_out_out = self.claim_single_out(wire_out);
                        self.add_bel_wire(bcrd, format!("EBRO{i}{j:02}_OUT_OUT"), wire_out_out);

                        let int_wire = TileWireCoord::new_idx(
                            15 - i,
                            self.intdb
                                .get_wire(&format!("EBR_{nedge}{j}_1", nedge = !edge)),
                        );
                        bel.pins
                            .insert(format!("EBRO{i}{j:02}"), BelPin::new_out(int_wire));
                        let wire_ebr = self.ebr_wires[&self.edev.tile_wire(tcrd, int_wire)];
                        self.claim_pip(wire_ebr, wire_out_out);
                    }
                    for j in 0..48 {
                        let wire = self.rc_io_wire(cell, &format!("EBRI{i}{j:02}_MACO"));
                        self.add_bel_wire(bcrd, format!("EBRI{i}{j:02}"), wire);
                        let wire_in = self.rc_io_wire(cell, &format!("EBRI{i}{j:02}"));
                        self.add_bel_wire(bcrd, format!("EBRI{i}{j:02}_IN"), wire_in);
                        self.claim_pip(wire, wire_in);
                        let wire_ebr = self.claim_single_in(wire_in);
                        let int_wire = TileWireCoord::new_idx(
                            6 + i,
                            self.intdb.get_wire(&format!("EBR_{edge}{j}_0")),
                        );
                        bel.pins
                            .insert(format!("EBRI{i}{j:02}"), BelPin::new_in(int_wire));
                        match self.ebr_wires.entry(self.edev.tile_wire(tcrd, int_wire)) {
                            btree_map::Entry::Vacant(e) => {
                                e.insert(wire_ebr);
                            }
                            btree_map::Entry::Occupied(e) => {
                                assert_eq!(*e.get(), wire_ebr);
                            }
                        }
                    }
                }

                for i in 0..4 {
                    let wire = self.rc_io_wire(cell, &format!("ECLK{i}_MACO"));
                    self.add_bel_wire(bcrd, format!("ECLK{i}"), wire);
                    let wire_in = self.rc_io_wire(cell, &format!("ECLK{i}"));
                    self.add_bel_wire(bcrd, format!("ECLK{i}_IN"), wire_in);
                    self.claim_pip(wire, wire_in);

                    let bank = match edge {
                        DirH::W => 7,
                        DirH::E => 2,
                    };
                    let bcrd_eclk = self.chip.bel_eclk_root_bank(bank);
                    for i in 0..8 {
                        let wire_eclk = self.naming.bel_wire(bcrd_eclk, &format!("ECLK{i}"));
                        self.claim_pip(wire_in, wire_eclk);
                    }
                }

                for dir in [DirV::S, DirV::N] {
                    let dir_io = match (dir, edge) {
                        (DirV::S, DirH::W) => DirH::E,
                        (DirV::S, DirH::E) => DirH::W,
                        (DirV::N, DirH::W) => DirH::W,
                        (DirV::N, DirH::E) => DirH::E,
                    };
                    for i in 0..8 {
                        for j in 0..32 {
                            let twire_ipo = TileWireCoord::new_idx(
                                0,
                                self.intdb.get_wire(&format!("OUT_MACO_IO_{dir}{i}_{j}")),
                            );
                            let wire_ipo =
                                self.rc_io_wire(cell, &format!("{dir}IPO{i}{j:02}_MACO"));
                            self.add_bel_wire(bcrd, format!("{dir}IPO{i}{j:02}"), wire_ipo);
                            bel.pins
                                .insert(format!("{dir}IPO{i}{j:02}"), BelPin::new_out(twire_ipo));

                            let wire_ipo_out = self.rc_io_wire(cell, &format!("{dir}IPO{i}{j:02}"));
                            self.claim_pip(wire_ipo_out, wire_ipo);
                            self.add_bel_wire(
                                bcrd_int,
                                twire_ipo.to_string(self.intdb, tcls),
                                wire_ipo_out,
                            );

                            let twire_out = TileWireCoord::new_idx(
                                match dir {
                                    DirV::S => 4,
                                    DirV::N => 5,
                                },
                                self.intdb
                                    .get_wire(&format!("IO_{dir_io}{j}_{seg}", seg = i + 1)),
                            );
                            let wire_out = self
                                .io_int_names
                                .get(&self.edev.tile_wire(tcrd, twire_out))
                                .copied();
                            if let Some(wire_out) = wire_out {
                                self.add_bel_wire_no_claim(
                                    bcrd_int,
                                    twire_out.to_string(self.intdb, tcls),
                                    wire_out,
                                );
                            }

                            let twire_in = TileWireCoord::new_idx(
                                match dir {
                                    DirV::S => 5,
                                    DirV::N => 4,
                                },
                                self.intdb.get_wire(&format!("IO_{dir_io}{j}_{i}")),
                            );
                            let wire_in = self
                                .io_int_names
                                .get(&self.edev.tile_wire(tcrd, twire_in))
                                .copied();
                            if let Some(wire_in) = wire_in {
                                self.add_bel_wire_no_claim(
                                    bcrd_int,
                                    twire_in.to_string(self.intdb, tcls),
                                    wire_in,
                                );
                            }

                            if let Some(wire_out) = wire_out {
                                self.claim_pip(wire_out, wire_ipo_out);
                                if let Some(wire_in) = wire_in {
                                    self.claim_pip(wire_out, wire_in);
                                }
                            }
                            sb.items.push(SwitchBoxItem::Mux(Mux {
                                dst: twire_out,
                                src: BTreeSet::from_iter([twire_in.pos(), twire_ipo.pos()]),
                            }));

                            let wire_ipi =
                                self.rc_io_wire(cell, &format!("{dir}IPI{i}{j:02}_MACO"));
                            self.add_bel_wire(bcrd, format!("{dir}IPI{i}{j:02}"), wire_ipi);
                            let wire_ipi_in = self.rc_io_wire(cell, &format!("{dir}IPI{i}{j:02}"));
                            self.add_bel_wire(bcrd, format!("{dir}IPI{i}{j:02}_IN"), wire_ipi_in);
                            self.claim_pip(wire_ipi, wire_ipi_in);
                            if let Some(wire_in) = wire_in {
                                self.claim_pip(wire_ipi_in, wire_in);
                            }
                            bel.pins
                                .insert(format!("{dir}IPI{i}{j:02}"), BelPin::new_out(twire_in));
                        }
                    }
                }

                let mut tips = vec![];
                for i in 0..2 {
                    let tip = self.rc_io_wire(cell, &format!("TIP{i:04}_MACO"));
                    self.add_bel_wire(bcrd, format!("TIP{i:04}"), tip);
                    let tip_out = self.rc_io_wire(cell, &format!("TIP{i:04}"));
                    self.add_bel_wire(bcrd, format!("TIP{i:04}_OUT"), tip_out);
                    self.claim_pip(tip_out, tip);
                    let tip_cib = self.rc_io_wire(cell, &format!("JTIP{i:04}_CIB"));
                    self.add_bel_wire(bcrd, format!("TIP{i:04}_CIB"), tip_cib);
                    self.claim_pip(tip_cib, tip_out);
                    let bpin = self.xlat_int_wire(bcrd, tip_cib);
                    assert_eq!(bpin.dir, PinDir::Output);
                    assert_eq!(bpin.wires.len(), 1);
                    let twire = bpin.wires.iter().copied().next().unwrap();
                    self.add_bel_wire_no_claim(
                        bcrd_int,
                        twire.to_string(self.intdb, tcls),
                        tip_out,
                    );
                    tips.push((twire, tip_out));
                    bel.pins.insert(format!("TIP{i:04}"), bpin);
                }

                for dir in [DirV::S, DirV::N] {
                    let dir_io = match (dir, edge) {
                        (DirV::S, DirH::W) => DirH::E,
                        (DirV::S, DirH::E) => DirH::W,
                        (DirV::N, DirH::W) => DirH::W,
                        (DirV::N, DirH::E) => DirH::E,
                    };
                    let wire = self.intdb.get_wire(&format!("IO_T_{dir_io}"));

                    let twire_out = TileWireCoord::new_idx(
                        match dir {
                            DirV::S => 4,
                            DirV::N => 5,
                        },
                        wire,
                    );
                    let wire_out = self.io_int_names[&self.edev.tile_wire(tcrd, twire_out)];
                    self.add_bel_wire_no_claim(
                        bcrd_int,
                        twire_out.to_string(self.intdb, tcls),
                        wire_out,
                    );

                    let twire_in = TileWireCoord::new_idx(
                        match dir {
                            DirV::S => 5,
                            DirV::N => 4,
                        },
                        wire,
                    );
                    let wire_in = self.io_int_names[&self.edev.tile_wire(tcrd, twire_in)];
                    self.add_bel_wire_no_claim(
                        bcrd_int,
                        twire_in.to_string(self.intdb, tcls),
                        wire_in,
                    );

                    let mut src = BTreeSet::from_iter([twire_in.pos()]);
                    self.claim_pip(wire_out, wire_in);

                    for &(twire_tip, wire_tip) in &tips {
                        src.insert(twire_tip.pos());
                        self.claim_pip(wire_out, wire_tip);
                    }

                    let wire_cib = self.rc_io_wire(cell, &format!("JVMT{dir}CIBI"));
                    let bpin = self.xlat_int_wire(bcrd, wire_cib);
                    assert_eq!(bpin.wires.len(), 1);
                    assert_eq!(bpin.dir, PinDir::Input);
                    let twire_cib = bpin.wires.into_iter().next().unwrap();
                    self.add_bel_wire(bcrd_int, twire_cib.to_string(self.intdb, tcls), wire_cib);
                    self.claim_pip(wire_out, wire_cib);
                    src.insert(twire_cib.pos());

                    sb.items.push(SwitchBoxItem::Mux(Mux {
                        dst: twire_out,
                        src,
                    }));
                }

                self.insert_bel(bcrd, bel);
                self.insert_bel_generic(bcrd_int, BelInfo::SwitchBox(sb));
            }
        }
    }
}
