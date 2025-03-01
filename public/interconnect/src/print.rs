use crate::db::{IntDb, IntfInfo, IriPin, PinDir, TermInfo};
use std::collections::BTreeMap;
use unnamed_entity::EntityId;

impl IntDb {
    pub fn print(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        for (_, k, &w) in &self.wires {
            writeln!(o, "\tWIRE {k:14} {w}", w = w.to_string(self))?;
        }
        for slot in self.bel_slots.values() {
            writeln!(o, "\tBEL SLOT {slot}")?;
        }
        for (_, name, node) in &self.nodes {
            writeln!(o, "\tNODE {name} {nt}", nt = node.tiles.len())?;
            for (&wo, mux) in &node.muxes {
                write!(
                    o,
                    "\t\tMUX {wot}.{won:14} <-",
                    wot = wo.0.to_idx(),
                    won = self.wires.key(wo.1)
                )?;
                for &wi in &mux.ins {
                    write!(
                        o,
                        " {wit}.{win:14}",
                        wit = wi.0.to_idx(),
                        win = self.wires.key(wi.1)
                    )?;
                }
                writeln!(o)?;
            }
            if !node.iris.is_empty() {
                writeln!(o, "\t\tIRI {n}", n = node.iris.len())?;
            }
            for (&wo, intf) in &node.intfs {
                match intf {
                    IntfInfo::OutputTestMux(ins) => {
                        write!(
                            o,
                            "\t\tINTF.TESTMUX {wot}.{won} <-",
                            wot = wo.0.to_idx(),
                            won = self.wires.key(wo.1)
                        )?;
                        for &wi in ins {
                            write!(
                                o,
                                " {wit}.{win}",
                                wit = wi.0.to_idx(),
                                win = self.wires.key(wi.1)
                            )?;
                        }
                        writeln!(o)?;
                    }
                    IntfInfo::OutputTestMuxPass(ins, wi) => {
                        write!(
                            o,
                            "\t\tINTF.TESTMUX.PASS {wot}.{won} <- {wit}.{win} | ",
                            wot = wo.0.to_idx(),
                            won = self.wires.key(wo.1),
                            wit = wi.0.to_idx(),
                            win = self.wires.key(wi.1)
                        )?;
                        for &wi in ins {
                            write!(
                                o,
                                " {wit}.{win}",
                                wit = wi.0.to_idx(),
                                win = self.wires.key(wi.1)
                            )?;
                        }
                        writeln!(o)?;
                    }
                    IntfInfo::InputDelay => {
                        writeln!(
                            o,
                            "\t\tINTF.DELAY {wot}.{won}",
                            wot = wo.0.to_idx(),
                            won = self.wires.key(wo.1)
                        )?;
                    }
                    IntfInfo::InputIri(iri, pin) => {
                        write!(
                            o,
                            "\t\tINTF.IRI {wot}.{won} IRI.{iri} ",
                            wot = wo.0.to_idx(),
                            won = self.wires.key(wo.1),
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
                            wot = wo.0.to_idx(),
                            won = self.wires.key(wo.1),
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
            for (slot, bel) in &node.bels {
                writeln!(o, "\t\tBEL {slot}:", slot = self.bel_slots[slot])?;
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
                            wit = wi.0.to_idx(),
                            win = self.wires.key(wi.1)
                        )?;
                    }
                    writeln!(o)?;
                }
            }
            for (wire, bels) in wires {
                write!(
                    o,
                    "\t\tWIRE {wt:3}.{wn:20}",
                    wt = wire.0.to_idx(),
                    wn = self.wires.key(wire.1)
                )?;
                for (bel, pin) in bels {
                    write!(o, " {bel}.{pin}", bel = self.bel_slots[bel])?;
                }
                writeln!(o)?;
            }
        }
        for (_, name, slot) in &self.term_slots {
            writeln!(
                o,
                "\tTERM SLOT {name}: opposite {oname}",
                oname = self.term_slots.key(slot.opposite)
            )?;
        }
        for (_, name, term) in &self.terms {
            writeln!(
                o,
                "\tTERM {name} {slot}",
                slot = self.term_slots.key(term.slot)
            )?;
            for (w, ti) in &term.wires {
                let wn = &self.wires.key(w);
                match ti {
                    TermInfo::BlackHole => {
                        writeln!(o, "\t\tBLACKHOLE {wn}")?;
                    }
                    &TermInfo::PassNear(ow) => {
                        writeln!(o, "\t\tPASS NEAR {wn} <- {own}", own = self.wires.key(ow))?;
                    }
                    &TermInfo::PassFar(ow) => {
                        writeln!(o, "\t\tPASS FAR {wn} <- {own}", own = self.wires.key(ow))?;
                    }
                }
            }
        }
        Ok(())
    }
}
