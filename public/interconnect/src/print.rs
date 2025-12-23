use crate::db::{
    BelAttributeType, BelInfo, BelKind, ConnectorWire, IntDb, PadKind, PinDir, TileClass,
};
use prjcombine_entity::{EntityBundleItemIndex, EntityId};
use std::collections::BTreeMap;

impl TileClass {
    pub fn print(&self, db: &IntDb, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        for name in self.cells.values() {
            writeln!(o, "\t\tcell {name}")?;
        }
        for rect in self.bitrects.values() {
            writeln!(
                o,
                "\t\tbitrect {name}: {orientation:?} ({rf}{frames}, {rb}{bits})",
                name = rect.name,
                orientation = rect.geometry.orientation,
                rf = if rect.geometry.rev_frames { "rev " } else { "" },
                rb = if rect.geometry.rev_bits { "rev " } else { "" },
                frames = rect.geometry.frames,
                bits = rect.geometry.bits,
            )?;
        }
        let mut wires: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for (slot, bel) in &self.bels {
            match bel {
                BelInfo::SwitchBox(sb) => {
                    writeln!(o, "\t\tswitchbox {slot}", slot = db.bel_slots.key(slot))?;
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
                BelInfo::Bel(bel) => {
                    let BelKind::Class(bcid) = db.bel_slots[slot].kind else {
                        unreachable!()
                    };
                    writeln!(
                        o,
                        "\t\tbel {slot}: {bcls}",
                        slot = db.bel_slots.key(slot),
                        bcls = db.bel_classes.key(bcid)
                    )?;
                    let bcls = &db.bel_classes[bcid];
                    for (pid, wire) in &bel.inputs {
                        let (pname, idx) = bcls.inputs.key(pid);
                        let pname = match idx {
                            EntityBundleItemIndex::Single => pname.to_string(),
                            EntityBundleItemIndex::Array { index, .. } => {
                                format!("{pname}[{index}]")
                            }
                        };
                        write!(o, "\t\t\tinput {pname} = ")?;
                        match wire {
                            crate::db::BelInput::Fixed(wire) => {
                                write!(o, "{wire}", wire = wire.to_string(db, self))?;
                            }
                            crate::db::BelInput::Invertible(wire, _) => {
                                write!(o, "^{wire}", wire = wire.to_string(db, self))?;
                            }
                        }
                        writeln!(o)?;
                    }
                    for (pid, pwires) in &bel.outputs {
                        let (pname, idx) = bcls.outputs.key(pid);
                        let pname = match idx {
                            EntityBundleItemIndex::Single => pname.to_string(),
                            EntityBundleItemIndex::Array { index, .. } => {
                                format!("{pname}[{index}]")
                            }
                        };
                        write!(o, "\t\t\toutput {pname} = ")?;
                        let mut first = true;
                        for &w in pwires {
                            if !first {
                                write!(o, ", ")?;
                            }
                            first = false;
                            wires.entry(w).or_default().push((slot, pname.clone()));
                            write!(o, "{wire}", wire = w.to_string(db, self))?;
                        }
                        writeln!(o)?;
                    }
                    for (pid, pwire) in &bel.bidirs {
                        let (pname, idx) = bcls.bidirs.key(pid);
                        let pname = match idx {
                            EntityBundleItemIndex::Single => pname.to_string(),
                            EntityBundleItemIndex::Array { index, .. } => {
                                format!("{pname}[{index}]")
                            }
                        };
                        write!(o, "\t\t\tbidir {pname} = ")?;
                        wires.entry(*pwire).or_default().push((slot, pname));
                        write!(o, "{wire}", wire = pwire.to_string(db, self))?;
                        writeln!(o)?;
                    }
                    // TODO attributes
                }
                BelInfo::Legacy(bel) => {
                    writeln!(o, "\t\tbel {slot}: legacy", slot = db.bel_slots.key(slot))?;
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
                            wires.entry(wi).or_default().push((slot, pn.to_string()));
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
        for (_, name, ecls) in &self.enum_classes {
            writeln!(o, "\tenum {name}")?;
            for val in ecls.values.values() {
                writeln!(o, "\t\t{val}")?;
            }
        }
        for (_, name, bcls) in &self.bel_classes {
            writeln!(o, "\tbel_class {name}")?;
            for (_, pname, pin) in bcls.inputs.bundles() {
                writeln!(
                    o,
                    "\t\t{nr}input {pname}",
                    nr = if pin.nonroutable { "nonroutable " } else { "" }
                )?;
            }
            for (_, pname, pin) in bcls.outputs.bundles() {
                writeln!(
                    o,
                    "\t\t{nr}output {pname}",
                    nr = if pin.nonroutable { "nonroutable " } else { "" }
                )?;
            }
            for (_, pname, pin) in bcls.bidirs.bundles() {
                writeln!(
                    o,
                    "\t\t{nr}bidir {pname}",
                    nr = if pin.nonroutable { "nonroutable " } else { "" }
                )?;
            }
            for (_, pname, pad) in bcls.pads.bundles() {
                writeln!(
                    o,
                    "\t\tpad {pname}: {kind}",
                    kind = match pad.kind {
                        PadKind::In => "input",
                        PadKind::Out => "output",
                        PadKind::Inout => "inout",
                        PadKind::Power => "power",
                        PadKind::Analog => "analog",
                    }
                )?;
            }
            for (_, aname, attr) in &bcls.attributes {
                write!(o, "\t\tattribute {aname}: ")?;
                match attr.typ {
                    BelAttributeType::Enum(eid) => {
                        write!(o, "{}", self.enum_classes.key(eid))?;
                    }
                    BelAttributeType::Bool => {
                        write!(o, "bool")?;
                    }
                    BelAttributeType::Bitvec(width) => {
                        write!(o, "bitvec[{width}]")?;
                    }
                    BelAttributeType::BitvecArray(width, depth) => {
                        write!(o, "bitvec[{width}][{depth}]")?;
                    }
                }
                writeln!(o)?;
            }
            // TODO
        }
        for (_, k, &w) in &self.wires {
            writeln!(o, "\twire {k:14} {w}", w = w.to_string(self))?;
        }
        for slot in self.region_slots.values() {
            writeln!(o, "\tregion_slot {slot}")?;
        }
        for slot in self.tile_slots.values() {
            writeln!(o, "\ttile_slot {slot}")?;
        }
        for (_, name, bslot) in &self.bel_slots {
            write!(
                o,
                "\tbel_slot {name}: {tile_slot}: ",
                tile_slot = self.tile_slots[bslot.tile_slot]
            )?;
            match bslot.kind {
                BelKind::Routing => write!(o, "routing")?,
                BelKind::Class(bcls) => write!(o, "{}", self.bel_classes.key(bcls))?,
                BelKind::Legacy => write!(o, "legacy")?,
            }
            writeln!(o)?
        }
        for (_, name, tcls) in &self.tile_classes {
            writeln!(
                o,
                "\ttile_class {name} {slot}",
                slot = self.tile_slots[tcls.slot],
            )?;
            tcls.print(self, o)?;
        }
        for (_, name, slot) in &self.conn_slots {
            writeln!(
                o,
                "\tconn_slot {name}: opposite {oname}",
                oname = self.conn_slots.key(slot.opposite)
            )?;
        }
        for (_, name, term) in &self.conn_classes {
            writeln!(
                o,
                "\tconn_class {name} {slot}",
                slot = self.conn_slots.key(term.slot)
            )?;
            for (w, ti) in &term.wires {
                let wn = &self.wires.key(w);
                match ti {
                    ConnectorWire::BlackHole => {
                        writeln!(o, "\t\tblackhole {wn}")?;
                    }
                    &ConnectorWire::Reflect(ow) => {
                        writeln!(o, "\t\treclect {wn} <- {own}", own = self.wires.key(ow))?;
                    }
                    &ConnectorWire::Pass(ow) => {
                        writeln!(o, "\t\tpass {wn} <- {own}", own = self.wires.key(ow))?;
                    }
                }
            }
        }
        Ok(())
    }
}
