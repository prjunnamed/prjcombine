use crate::db::{ConnectorWire, IntDb, IntfInfo, IriPin, PinDir};
use std::collections::BTreeMap;
use unnamed_entity::EntityId;

impl IntDb {
    pub fn print(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        for (_, k, &w) in &self.wires {
            writeln!(o, "\tWIRE {k:14} {w}", w = w.to_string(self))?;
        }
        for slot in self.region_slots.values() {
            writeln!(o, "\tREGION SLOT {slot}")?;
        }
        for slot in self.tile_slots.values() {
            writeln!(o, "\tTILE SLOT {slot}")?;
        }
        for (_, name, bslot) in &self.bel_slots {
            writeln!(
                o,
                "\tBEL SLOT {name}: {tile_slot}",
                tile_slot = bslot.tile_slot
            )?;
        }
        for (_, name, tcls) in &self.tile_classes {
            writeln!(
                o,
                "\tTILE CLASS {name} {slot} {nt}",
                slot = self.tile_slots[tcls.slot],
                nt = tcls.cells.len()
            )?;
            for (&wo, mux) in &tcls.muxes {
                write!(
                    o,
                    "\t\tMUX {wot}.{won:14} <-",
                    wot = wo.cell.to_idx(),
                    won = self.wires.key(wo.wire)
                )?;
                for &wi in &mux.ins {
                    write!(
                        o,
                        " {wit}.{win:14}",
                        wit = wi.cell.to_idx(),
                        win = self.wires.key(wi.wire)
                    )?;
                }
                writeln!(o)?;
            }
            if !tcls.iris.is_empty() {
                writeln!(o, "\t\tIRI {n}", n = tcls.iris.len())?;
            }
            for (&wo, intf) in &tcls.intfs {
                match intf {
                    IntfInfo::OutputTestMux(ins) => {
                        write!(
                            o,
                            "\t\tINTF.TESTMUX {wot}.{won} <-",
                            wot = wo.cell.to_idx(),
                            won = self.wires.key(wo.wire)
                        )?;
                        for &wi in ins {
                            write!(
                                o,
                                " {wit}.{win}",
                                wit = wi.cell.to_idx(),
                                win = self.wires.key(wi.wire)
                            )?;
                        }
                        writeln!(o)?;
                    }
                    IntfInfo::OutputTestMuxPass(ins, wi) => {
                        write!(
                            o,
                            "\t\tINTF.TESTMUX.PASS {wot}.{won} <- {wit}.{win} | ",
                            wot = wo.cell.to_idx(),
                            won = self.wires.key(wo.wire),
                            wit = wi.cell.to_idx(),
                            win = self.wires.key(wi.wire)
                        )?;
                        for &wi in ins {
                            write!(
                                o,
                                " {wit}.{win}",
                                wit = wi.cell.to_idx(),
                                win = self.wires.key(wi.wire)
                            )?;
                        }
                        writeln!(o)?;
                    }
                    IntfInfo::InputDelay => {
                        writeln!(
                            o,
                            "\t\tINTF.DELAY {wot}.{won}",
                            wot = wo.cell.to_idx(),
                            won = self.wires.key(wo.wire)
                        )?;
                    }
                    IntfInfo::InputIri(iri, pin) => {
                        write!(
                            o,
                            "\t\tINTF.IRI {wot}.{won} IRI.{iri} ",
                            wot = wo.cell.to_idx(),
                            won = self.wires.key(wo.wire),
                            iri = iri.to_idx(),
                        )?;
                        match pin {
                            IriPin::Clk => writeln!(o, "CLK")?,
                            IriPin::Rst => writeln!(o, "RST")?,
                            IriPin::Ce(i) => writeln!(o, "CE{i}")?,
                            IriPin::Imux(i) => writeln!(o, "IMUX{i}")?,
                        }
                    }
                    IntfInfo::InputIriDelay(iri, pin) => {
                        write!(
                            o,
                            "\t\tINTF.IRI.DELAY {wot}.{won} IRI.{iri} ",
                            wot = wo.cell.to_idx(),
                            won = self.wires.key(wo.wire),
                            iri = iri.to_idx(),
                        )?;
                        match pin {
                            IriPin::Clk => writeln!(o, "CLK")?,
                            IriPin::Rst => writeln!(o, "RST")?,
                            IriPin::Ce(i) => writeln!(o, "CE{i}")?,
                            IriPin::Imux(i) => writeln!(o, "IMUX{i}")?,
                        }
                    }
                }
            }
            let mut wires: BTreeMap<_, Vec<_>> = BTreeMap::new();
            for (slot, bel) in &tcls.bels {
                writeln!(o, "\t\tBEL {slot}:", slot = self.bel_slots.key(slot))?;
                for (pn, pin) in &bel.pins {
                    write!(
                        o,
                        "\t\t\t{d}{intf} {pn:20}",
                        d = match pin.dir {
                            PinDir::Input => " INPUT",
                            PinDir::Output => "OUTPUT",
                            PinDir::Inout => " INOUT",
                        },
                        intf = if pin.is_intf_in { ".INTF" } else { "     " }
                    )?;
                    for &wi in &pin.wires {
                        wires.entry(wi).or_default().push((slot, pn));
                        write!(
                            o,
                            " {wit}.{win}",
                            wit = wi.cell.to_idx(),
                            win = self.wires.key(wi.wire)
                        )?;
                    }
                    writeln!(o)?;
                }
            }
            for (wire, bels) in wires {
                write!(
                    o,
                    "\t\tWIRE {wt:3}.{wn:20}",
                    wt = wire.cell.to_idx(),
                    wn = self.wires.key(wire.wire)
                )?;
                for (bel, pin) in bels {
                    write!(o, " {bel}.{pin}", bel = self.bel_slots.key(bel))?;
                }
                writeln!(o)?;
            }
        }
        for (_, name, slot) in &self.conn_slots {
            writeln!(
                o,
                "\tCONN SLOT {name}: opposite {oname}",
                oname = self.conn_slots.key(slot.opposite)
            )?;
        }
        for (_, name, term) in &self.conn_classes {
            writeln!(
                o,
                "\tCONN CLASS {name} {slot}",
                slot = self.conn_slots.key(term.slot)
            )?;
            for (w, ti) in &term.wires {
                let wn = &self.wires.key(w);
                match ti {
                    ConnectorWire::BlackHole => {
                        writeln!(o, "\t\tBLACKHOLE {wn}")?;
                    }
                    &ConnectorWire::Reflect(ow) => {
                        writeln!(o, "\t\tPASS NEAR {wn} <- {own}", own = self.wires.key(ow))?;
                    }
                    &ConnectorWire::Pass(ow) => {
                        writeln!(o, "\t\tPASS FAR {wn} <- {own}", own = self.wires.key(ow))?;
                    }
                }
            }
        }
        Ok(())
    }
}
