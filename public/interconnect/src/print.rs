use crate::db::{BelInfo, ConnectorWire, IntDb, PinDir, TileClass};
use prjcombine_entity::EntityId;
use std::collections::BTreeMap;

impl TileClass {
    pub fn print(&self, db: &IntDb, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        let mut wires: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for (slot, bel) in &self.bels {
            match bel {
                BelInfo::SwitchBox(sb) => {
                    writeln!(o, "\t\t{slot}: SWITCHBOX", slot = db.bel_slots.key(slot))?;
                    for item in &sb.items {
                        match item {
                            crate::db::SwitchBoxItem::Mux(mux) => {
                                write!(
                                    o,
                                    "\t\t\tMUX      {dst:20} <- ",
                                    dst = mux.dst.to_string(db, self)
                                )?;
                                for src in mux.src.keys() {
                                    write!(o, " {src:20}", src = src.to_string(db, self))?;
                                }
                                writeln!(o)?;
                            }
                            crate::db::SwitchBoxItem::ProgBuf(buf) => writeln!(
                                o,
                                "\t\t\tPROGBUF  {dst:20} <-  {src:20}",
                                dst = buf.dst.to_string(db, self),
                                src = buf.src.to_string(db, self),
                            )?,
                            crate::db::SwitchBoxItem::PermaBuf(buf) => writeln!(
                                o,
                                "\t\t\tPERMABUF {dst:20} <-  {src:20}",
                                dst = buf.dst.to_string(db, self),
                                src = buf.src.to_string(db, self),
                            )?,
                            crate::db::SwitchBoxItem::Pass(pass) => writeln!(
                                o,
                                "\t\t\tPASS     {dst:20} <-  {src:20}",
                                dst = pass.dst.to_string(db, self),
                                src = pass.src.to_string(db, self),
                            )?,
                            crate::db::SwitchBoxItem::BiPass(pass) => writeln!(
                                o,
                                "\t\t\tPASS     {a:20} <-> {b:20}",
                                a = pass.a.to_string(db, self),
                                b = pass.b.to_string(db, self),
                            )?,
                            crate::db::SwitchBoxItem::ProgInv(inv) => writeln!(
                                o,
                                "\t\t\tPROGINV  {dst:20} <-  {src:20}",
                                dst = inv.dst.to_string(db, self),
                                src = inv.src.to_string(db, self),
                            )?,
                            crate::db::SwitchBoxItem::ProgDelay(delay) => writeln!(
                                o,
                                "\t\t\tDELAY #{n} {dst:20} <-  {src:20}",
                                n = delay.steps.len(),
                                dst = delay.dst.to_string(db, self),
                                src = delay.src.to_string(db, self),
                            )?,
                        }
                    }
                }
                BelInfo::Bel(_bel) => {
                    todo!();
                }
                BelInfo::Legacy(bel) => {
                    writeln!(o, "\t\t{slot}: LEGACY BEL", slot = db.bel_slots.key(slot))?;
                    for (pn, pin) in &bel.pins {
                        write!(
                            o,
                            "\t\t\t{d} {pn:20}",
                            d = match pin.dir {
                                PinDir::Input => " INPUT",
                                PinDir::Output => "OUTPUT",
                                PinDir::Inout => " INOUT",
                            },
                        )?;
                        for &wi in &pin.wires {
                            wires.entry(wi).or_default().push((slot, pn));
                            write!(o, " {wire}", wire = wi.to_string(db, self))?;
                        }
                        writeln!(o)?;
                    }
                }
                BelInfo::TestMux(tmux) => {
                    writeln!(o, "\t\t{slot}: TEST_MUX", slot = db.bel_slots.key(slot))?;
                    for (dst, tmwire) in &tmux.wires {
                        write!(
                            o,
                            "\t\t\t{dst:20} <- {psrc:20} ||",
                            dst = dst.to_string(db, self),
                            psrc = tmwire.primary_src.to_string(db, self),
                        )?;
                        for src in &tmwire.test_src {
                            write!(o, " {src:20}", src = src.to_string(db, self))?;
                        }
                        writeln!(o)?;
                    }
                }
                BelInfo::GroupTestMux(tmux) => {
                    writeln!(
                        o,
                        "\t\t{slot}: GROUP_TEST_MUX {num_groups}",
                        slot = db.bel_slots.key(slot),
                        num_groups = tmux.num_groups
                    )?;
                    for (dst, tmwire) in &tmux.wires {
                        write!(
                            o,
                            "\t\t\t{dst:20} <- {psrc:20} || ",
                            dst = dst.to_string(db, self),
                            psrc = tmwire.primary_src.to_string(db, self),
                        )?;
                        for (i, src) in tmwire.test_src.iter().enumerate() {
                            if i != 0 {
                                write!(o, " | ")?;
                            }
                            if let Some(src) = src {
                                write!(o, " {src:20}", src = src.to_string(db, self))?;
                            } else {
                                write!(o, " {src:20}", src = "---")?;
                            }
                        }
                        writeln!(o)?;
                    }
                }
            }
        }
        for (wire, bels) in wires {
            write!(
                o,
                "\t\tWIRE {wt:3}.{wn:20}",
                wt = wire.cell.to_idx(),
                wn = db.wires.key(wire.wire)
            )?;
            for (bel, pin) in bels {
                write!(o, " {bel}.{pin}", bel = db.bel_slots.key(bel))?;
            }
            writeln!(o)?;
        }
        Ok(())
    }
}

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
                tile_slot = self.tile_slots[bslot.tile_slot]
            )?;
        }
        for (_, name, tcls) in &self.tile_classes {
            writeln!(
                o,
                "\tTILE CLASS {name} {slot} {nt}",
                slot = self.tile_slots[tcls.slot],
                nt = tcls.cells.len()
            )?;
            tcls.print(self, o)?;
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
