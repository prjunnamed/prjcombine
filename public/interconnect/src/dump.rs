use crate::db::{
    BelAttribute, BelAttributeType, BelInfo, BelKind, ConnectorWire, IntDb, PadKind, PinDir,
    SwitchBoxItem, TableValue, TileClass,
};
use prjcombine_entity::{EntityBundleIndex, EntityBundleItemIndex, EntityId};
use prjcombine_types::bsdata::{PolTileBit, TileBit};
use std::collections::BTreeMap;

impl TileClass {
    pub fn dump(&self, db: &IntDb, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        for name in self.cells.values() {
            writeln!(o, "\t\t\tcell {name};")?;
        }
        for rect in self.bitrects.values() {
            writeln!(
                o,
                "\t\t\tbitrect {name}: {orientation:?} ({rf}{frames}, {rb}{bits});",
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
            writeln!(o)?;
            match bel {
                BelInfo::SwitchBox(sb) => {
                    writeln!(
                        o,
                        "\t\t\tswitchbox {slot} {{",
                        slot = db.bel_slots.key(slot)
                    )?;
                    for item in &sb.items {
                        match item {
                            SwitchBoxItem::Mux(mux) => {
                                write!(o, "\t\t\t\tmux {dst}", dst = mux.dst.to_string(db, self))?;
                                if mux.bits.is_empty() {
                                    write!(o, " = ")?;
                                    let mut first = true;
                                    for src in mux.src.keys() {
                                        if !first {
                                            write!(o, " | ")?;
                                        }
                                        first = false;
                                        write!(o, "{src}", src = src.to_string(db, self))?;
                                    }
                                    writeln!(o, ";")?;
                                } else {
                                    write!(o, " @[")?;
                                    let mut first = true;
                                    for &bit in mux.bits.iter().rev() {
                                        if !first {
                                            write!(o, ", ")?;
                                        }
                                        first = false;
                                        write!(o, "{}", self.dump_bit(bit))?;
                                    }
                                    writeln!(o, "] {{")?;
                                    for (src, v) in &mux.src {
                                        writeln!(
                                            o,
                                            "\t\t\t\t\t{src} = 0b{v},",
                                            src = src.to_string(db, self)
                                        )?;
                                    }
                                    if let Some(ref v) = mux.bits_off {
                                        writeln!(o, "\t\t\t\t\toff = 0b{v},")?;
                                    }
                                    writeln!(o, "\t\t\t\t}}")?;
                                }
                            }
                            SwitchBoxItem::ProgBuf(buf) => writeln!(
                                o,
                                "\t\t\t\tprogbuf {dst} = {src} @{bit};",
                                dst = buf.dst.to_string(db, self),
                                src = buf.src.to_string(db, self),
                                bit = self.dump_polbit(buf.bit),
                            )?,
                            SwitchBoxItem::PermaBuf(buf) => writeln!(
                                o,
                                "\t\t\t\tpermabuf {dst} = {src};",
                                dst = buf.dst.to_string(db, self),
                                src = buf.src.to_string(db, self),
                            )?,
                            SwitchBoxItem::Pass(pass) => writeln!(
                                o,
                                "\t\t\t\tpass {dst} = {src} @{bit};",
                                dst = pass.dst.to_string(db, self),
                                src = pass.src.to_string(db, self),
                                bit = self.dump_polbit(pass.bit),
                            )?,
                            SwitchBoxItem::BiPass(pass) => writeln!(
                                o,
                                "\t\t\t\tbipass {a} = {b} @{bit};",
                                a = pass.a.to_string(db, self),
                                b = pass.b.to_string(db, self),
                                bit = self.dump_polbit(pass.bit),
                            )?,
                            SwitchBoxItem::ProgInv(inv) => writeln!(
                                o,
                                "\t\t\t\tproginv {dst} = {src} @{bit};",
                                dst = inv.dst.to_string(db, self),
                                src = inv.src.to_string(db, self),
                                bit = self.dump_polbit(inv.bit),
                            )?,
                            SwitchBoxItem::ProgDelay(delay) => {
                                write!(
                                    o,
                                    "\t\t\t\tprogdelay {dst} = {src}",
                                    dst = delay.dst.to_string(db, self),
                                    src = delay.src.to_string(db, self),
                                )?;
                                if delay.bits.is_empty() {
                                    writeln!(o, " #{n}", n = delay.steps.len())?;
                                } else {
                                    write!(o, " @[")?;
                                    let mut first = true;
                                    for &bit in delay.bits.iter().rev() {
                                        if !first {
                                            write!(o, ", ")?;
                                        }
                                        first = false;
                                        write!(o, "{}", self.dump_bit(bit))?;
                                    }
                                    writeln!(o, "] {{")?;
                                    for v in &delay.steps {
                                        writeln!(o, "\t\t\t\t\t0b{v},")?;
                                    }
                                    writeln!(o, "\t\t\t\t}}")?;
                                }
                            }
                            SwitchBoxItem::Bidi(bidi) => writeln!(
                                o,
                                "\t\t\t\tbidi {conn} {wire} @{bit};",
                                conn = db.conn_slots.key(bidi.conn),
                                wire = bidi.wire.to_string(db, self),
                                bit = self.dump_polbit(bidi.bit_upstream),
                            )?,
                            SwitchBoxItem::PairMux(mux) => {
                                write!(
                                    o,
                                    "\t\t\t\tpair_mux ({dst0}, {dst1})",
                                    dst0 = mux.dst[0].to_string(db, self),
                                    dst1 = mux.dst[1].to_string(db, self),
                                )?;
                                if mux.bits.is_empty() {
                                    write!(o, " = ")?;
                                    let mut first = true;
                                    for src in mux.src.keys() {
                                        if !first {
                                            write!(o, " | ")?;
                                        }
                                        first = false;
                                        write!(
                                            o,
                                            "({src0}, {src1})",
                                            src0 = if let Some(src) = src[0] {
                                                src.to_string(db, self)
                                            } else {
                                                "_".to_string()
                                            },
                                            src1 = if let Some(src) = src[1] {
                                                src.to_string(db, self)
                                            } else {
                                                "_".to_string()
                                            }
                                        )?;
                                    }
                                    writeln!(o, ";")?;
                                } else {
                                    write!(o, " @[")?;
                                    let mut first = true;
                                    for &bit in mux.bits.iter().rev() {
                                        if !first {
                                            write!(o, ", ")?;
                                        }
                                        first = false;
                                        write!(o, "{}", self.dump_bit(bit))?;
                                    }
                                    writeln!(o, "] {{")?;
                                    for (src, v) in &mux.src {
                                        writeln!(
                                            o,
                                            "\t\t\t\t\t({src0}, {src1}) = 0b{v},",
                                            src0 = if let Some(src) = src[0] {
                                                src.to_string(db, self)
                                            } else {
                                                "_".to_string()
                                            },
                                            src1 = if let Some(src) = src[1] {
                                                src.to_string(db, self)
                                            } else {
                                                "_".to_string()
                                            }
                                        )?;
                                    }
                                    writeln!(o, "\t\t\t\t}}")?;
                                }
                            }
                        }
                    }
                    writeln!(o, "\t\t\t}}")?;
                }
                BelInfo::Bel(bel) => {
                    let BelKind::Class(bcid) = db.bel_slots[slot].kind else {
                        unreachable!()
                    };
                    writeln!(o, "\t\t\tbel {slot} {{", slot = db.bel_slots.key(slot))?;
                    let bcls = &db.bel_classes[bcid];
                    for (pid, wire) in &bel.inputs {
                        let (pname, idx) = bcls.inputs.key(pid);
                        let indexing = bcls.inputs[pid].indexing;
                        let pname = match idx {
                            EntityBundleItemIndex::Single => pname.to_string(),
                            EntityBundleItemIndex::Array { index, .. } => {
                                let index = indexing.phys_to_virt(index);
                                format!("{pname}[{index}]")
                            }
                        };
                        write!(o, "\t\t\t\tinput {pname} = ")?;
                        match wire {
                            crate::db::BelInput::Fixed(wire) => {
                                write!(o, "{wire}", wire = wire.to_string(db, self))?;
                                wires.entry(wire.tw).or_default().push((slot, pname));
                            }
                            crate::db::BelInput::Invertible(wire, bit) => {
                                write!(
                                    o,
                                    "^{wire} @{bit}",
                                    wire = wire.to_string(db, self),
                                    bit = self.dump_polbit(*bit)
                                )?;
                                wires.entry(*wire).or_default().push((slot, pname));
                            }
                        }
                        writeln!(o, ";")?;
                    }
                    for (pid, pwires) in &bel.outputs {
                        let (pname, idx) = bcls.outputs.key(pid);
                        let indexing = bcls.outputs[pid].indexing;
                        let pname = match idx {
                            EntityBundleItemIndex::Single => pname.to_string(),
                            EntityBundleItemIndex::Array { index, .. } => {
                                let index = indexing.phys_to_virt(index);
                                format!("{pname}[{index}]")
                            }
                        };
                        write!(o, "\t\t\t\toutput {pname} = ")?;
                        let mut first = true;
                        for &w in pwires {
                            if !first {
                                write!(o, ", ")?;
                            }
                            first = false;
                            wires.entry(w).or_default().push((slot, pname.clone()));
                            write!(o, "{wire}", wire = w.to_string(db, self))?;
                        }
                        writeln!(o, ";")?;
                    }
                    for (pid, pwire) in &bel.bidirs {
                        let (pname, idx) = bcls.bidirs.key(pid);
                        let indexing = bcls.bidirs[pid].indexing;
                        let pname = match idx {
                            EntityBundleItemIndex::Single => pname.to_string(),
                            EntityBundleItemIndex::Array { index, .. } => {
                                let index = indexing.phys_to_virt(index);
                                format!("{pname}[{index}]")
                            }
                        };
                        write!(o, "\t\t\t\tbidir {pname} = ")?;
                        wires.entry(*pwire).or_default().push((slot, pname));
                        write!(o, "{wire}", wire = pwire.to_string(db, self))?;
                        writeln!(o, ";")?;
                    }
                    for (aid, attr) in &bel.attributes {
                        write!(
                            o,
                            "\t\t\t\tattribute {aname} ",
                            aname = bcls.attributes.key(aid)
                        )?;
                        let bcattr = &bcls.attributes[aid];
                        match attr {
                            BelAttribute::BitVec(bits) => match bcattr.typ {
                                BelAttributeType::Enum(_) => unreachable!(),
                                BelAttributeType::Bool => {
                                    assert_eq!(bits.len(), 1);
                                    writeln!(o, "@{};", self.dump_polbit(bits[0]))?;
                                }
                                BelAttributeType::BitVec(width) => {
                                    assert_eq!(bits.len(), width);
                                    write!(o, "@[")?;
                                    let mut first = true;
                                    for &bit in bits.iter().rev() {
                                        if !first {
                                            write!(o, ", ")?;
                                        }
                                        first = false;
                                        write!(o, "{}", self.dump_polbit(bit))?;
                                    }
                                    writeln!(o, "];")?;
                                }
                                BelAttributeType::BitVecArray(width, depth) => {
                                    assert_eq!(bits.len(), width * depth);
                                    writeln!(o, "@[")?;
                                    for i in 0..depth {
                                        write!(o, "\t\t\t\t\t[")?;
                                        let mut first = true;
                                        for &bit in bits[i * width..(i + 1) * width].iter().rev() {
                                            if !first {
                                                write!(o, ", ")?;
                                            }
                                            first = false;
                                            write!(o, "{}", self.dump_polbit(bit))?;
                                        }
                                        writeln!(o, "],")?;
                                    }
                                    writeln!(o, "\t\t\t\t];")?;
                                }
                                BelAttributeType::U32 => unreachable!(),
                            },
                            BelAttribute::Enum(ebits) => {
                                let BelAttributeType::Enum(eid) = bcattr.typ else {
                                    unreachable!()
                                };
                                let ecls = &db.enum_classes[eid];
                                write!(o, "@[")?;
                                let mut first = true;
                                for &bit in ebits.bits.iter().rev() {
                                    if !first {
                                        write!(o, ", ")?;
                                    }
                                    first = false;
                                    write!(o, "{}", self.dump_bit(bit))?;
                                }
                                writeln!(o, "] {{")?;
                                for (k, v) in &ebits.values {
                                    writeln!(o, "\t\t\t\t\t{k} = 0b{v},", k = ecls.values[k])?;
                                }
                                writeln!(o, "\t\t\t\t}}")?;
                            }
                        }
                    }
                    writeln!(o, "\t\t\t}}")?;
                }
                BelInfo::Legacy(bel) => {
                    writeln!(o, "\t\t\tbel {slot} {{", slot = db.bel_slots.key(slot))?;
                    for (pn, pin) in &bel.pins {
                        write!(
                            o,
                            "\t\t\t\t{d} {pn} = ",
                            d = match pin.dir {
                                PinDir::Input => "input",
                                PinDir::Output => "output",
                                PinDir::Inout => "inout",
                            },
                        )?;
                        let mut first = true;
                        for &wi in &pin.wires {
                            if !first {
                                write!(o, ", ")?;
                            }
                            first = false;
                            wires.entry(wi).or_default().push((slot, pn.to_string()));
                            write!(o, "{wire}", wire = wi.to_string(db, self))?;
                        }
                        writeln!(o, ";")?;
                    }
                    writeln!(o, "\t\t\t}}")?;
                }
                BelInfo::OldTestMux => unreachable!(),
                BelInfo::TestMux(tmux) => {
                    write!(o, "\t\t\ttest_mux {slot}", slot = db.bel_slots.key(slot),)?;
                    if tmux.bits.is_empty() {
                        writeln!(o, " #{n} {{", n = tmux.groups.len())?;
                    } else {
                        write!(o, " @[")?;
                        let mut first = true;
                        for &bit in tmux.bits.iter().rev() {
                            if !first {
                                write!(o, ", ")?;
                            }
                            first = false;
                            write!(o, "{}", self.dump_bit(bit))?;
                        }
                        writeln!(o, "] {{")?;
                        writeln!(o, "\t\t\t\tprimary = 0b{v},", v = tmux.bits_primary)?;
                        for (idx, v) in tmux.groups.iter().enumerate() {
                            writeln!(o, "\t\t\t\ttest_group {idx} = 0b{v},")?;
                        }
                        writeln!(o, "\t\t\t}} {{")?;
                    }
                    for (dst, tmwire) in &tmux.wires {
                        write!(
                            o,
                            "\t\t\t\t{dst} = {psrc} || [",
                            dst = dst.to_string(db, self),
                            psrc = tmwire.primary_src.to_string(db, self),
                        )?;
                        for (i, src) in tmwire.test_src.iter().enumerate() {
                            if i != 0 {
                                write!(o, ", ")?;
                            }
                            if let Some(src) = src {
                                write!(o, "{src}", src = src.to_string(db, self))?;
                            } else {
                                write!(o, "none")?;
                            }
                        }
                        writeln!(o, "];")?;
                    }
                    writeln!(o, "\t\t\t}}")?;
                }
            }
        }
        if !wires.is_empty() {
            writeln!(o)?;
        }
        for (wire, bels) in wires {
            write!(o, "\t\t\t// wire {wn:30}", wn = wire.to_string(db, self))?;
            for (bel, pin) in bels {
                write!(o, " {bel}.{pin}", bel = db.bel_slots.key(bel))?;
            }
            writeln!(o)?;
        }
        Ok(())
    }

    pub fn dump_bit(&self, bit: TileBit) -> String {
        if bit.rect.to_idx() >= self.bitrects.len() {
            format!(
                "XXX{r}[{f}][{b}]",
                r = bit.rect.to_idx(),
                f = bit.frame.to_idx(),
                b = bit.bit.to_idx()
            )
        } else {
            format!(
                "{r}[{f}][{b}]",
                r = self.bitrects[bit.rect].name,
                f = bit.frame.to_idx(),
                b = bit.bit.to_idx()
            )
        }
    }

    pub fn dump_polbit(&self, bit: PolTileBit) -> String {
        if bit.inv {
            format!("!{}", self.dump_bit(bit.bit))
        } else {
            self.dump_bit(bit.bit)
        }
    }
}

impl IntDb {
    pub fn dump_typ(&self, typ: BelAttributeType) -> String {
        match typ {
            BelAttributeType::Enum(eid) => self.enum_classes.key(eid).to_string(),
            BelAttributeType::Bool => "bool".to_string(),
            BelAttributeType::BitVec(width) => {
                format!("bitvec[{width}]")
            }
            BelAttributeType::BitVecArray(width, depth) => {
                format!("bitvec[{width}][{depth}]")
            }
            BelAttributeType::U32 => "u32".to_string(),
        }
    }

    pub fn dump_value(&self, typ: BelAttributeType, value: &TableValue) -> String {
        match (typ, value) {
            (BelAttributeType::Enum(eid), TableValue::Enum(vid)) => {
                self.enum_classes[eid].values[*vid].to_string()
            }
            (BelAttributeType::Bool, TableValue::BitVec(val)) => {
                assert_eq!(val.len(), 1);
                if val[0] {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            (BelAttributeType::BitVec(width), TableValue::BitVec(val)) => {
                assert_eq!(val.len(), width);
                format!("0b{val}")
            }
            (BelAttributeType::BitVecArray(width, depth), TableValue::BitVec(val)) => {
                assert_eq!(val.len(), width * depth);
                let mut res = "[".to_string();
                let mut first = true;
                for i in 0..depth {
                    if !first {
                        res.push_str(", ");
                    }
                    first = false;
                    res.push_str("0b");
                    for bidx in ((i * width)..((i + 1) * width)).rev() {
                        res.push(if val[bidx] { '1' } else { '0' });
                    }
                }
                res.push(']');
                res
            }
            (BelAttributeType::U32, TableValue::U32(val)) => val.to_string(),
            _ => unreachable!(),
        }
    }

    pub fn dump(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(o, "intdb {{")?;
        for (_, name, ecls) in &self.enum_classes {
            writeln!(o, "\tenum {name} {{")?;
            for val in ecls.values.values() {
                writeln!(o, "\t\t{val},")?;
            }
            writeln!(o, "\t}}")?;
            writeln!(o,)?;
        }
        for (_, name, bcls) in &self.bel_classes {
            writeln!(o, "\tbel_class {name} {{")?;
            for (index, pname, pin) in bcls.inputs.bundles() {
                write!(
                    o,
                    "\t\t{nr}input {pname}",
                    nr = if pin.nonroutable { "nonroutable " } else { "" }
                )?;
                match index {
                    EntityBundleIndex::Single(_) => writeln!(o, ";")?,
                    EntityBundleIndex::Array(range) => {
                        if pin.indexing == Default::default() {
                            writeln!(o, "[{n}];", n = range.len())?;
                        } else {
                            writeln!(
                                o,
                                "[{msb}:{lsb}];",
                                msb = pin.indexing.msb_index(range.len()),
                                lsb = pin.indexing.lsb_index
                            )?;
                        }
                    }
                }
            }
            for (index, pname, pin) in bcls.outputs.bundles() {
                write!(
                    o,
                    "\t\t{nr}output {pname}",
                    nr = if pin.nonroutable { "nonroutable " } else { "" }
                )?;
                match index {
                    EntityBundleIndex::Single(_) => writeln!(o, ";")?,
                    EntityBundleIndex::Array(range) => {
                        if pin.indexing == Default::default() {
                            writeln!(o, "[{n}];", n = range.len())?;
                        } else {
                            writeln!(
                                o,
                                "[{msb}:{lsb}];",
                                msb = pin.indexing.msb_index(range.len()),
                                lsb = pin.indexing.lsb_index
                            )?;
                        }
                    }
                }
            }
            for (index, pname, pin) in bcls.bidirs.bundles() {
                write!(
                    o,
                    "\t\t{nr}bidir {pname}",
                    nr = if pin.nonroutable { "nonroutable " } else { "" }
                )?;
                match index {
                    EntityBundleIndex::Single(_) => writeln!(o, ";")?,
                    EntityBundleIndex::Array(range) => {
                        if pin.indexing == Default::default() {
                            writeln!(o, "[{n}];", n = range.len())?;
                        } else {
                            writeln!(
                                o,
                                "[{msb}:{lsb}];",
                                msb = pin.indexing.msb_index(range.len()),
                                lsb = pin.indexing.lsb_index
                            )?;
                        }
                    }
                }
            }
            for (index, pname, pad) in bcls.pads.bundles() {
                write!(o, "\t\tpad {pname}")?;
                match index {
                    EntityBundleIndex::Single(_) => (),
                    EntityBundleIndex::Array(range) => write!(o, "[{n}]", n = range.len())?,
                }
                writeln!(
                    o,
                    ": {kind}",
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
                writeln!(
                    o,
                    "\t\tattribute {aname}: {typ};",
                    typ = self.dump_typ(attr.typ)
                )?;
            }
            writeln!(o, "\t}}")?;
            writeln!(o,)?;
        }
        for slot in self.region_slots.values() {
            writeln!(o, "\tregion_slot {slot};")?;
        }
        for (_, k, &w) in &self.wires {
            writeln!(o, "\twire {k}: {w};", w = w.to_string(self))?;
        }
        for (tslot, tsname) in &self.tile_slots {
            writeln!(o)?;
            writeln!(o, "\ttile_slot {tsname} {{")?;
            for (_, name, bslot) in &self.bel_slots {
                if bslot.tile_slot != tslot {
                    continue;
                }
                write!(o, "\t\tbel_slot {name}: ")?;
                match bslot.kind {
                    BelKind::Routing => write!(o, "routing")?,
                    BelKind::Class(bcls) => write!(o, "{}", self.bel_classes.key(bcls))?,
                    BelKind::Legacy => write!(o, "legacy")?,
                }
                writeln!(o, ";")?
            }
            for (_, name, tcls) in &self.tile_classes {
                if tcls.slot != tslot {
                    continue;
                }
                writeln!(o,)?;
                writeln!(o, "\t\ttile_class {name} {{")?;
                tcls.dump(self, o)?;
                writeln!(o, "\t\t}}")?;
            }
            writeln!(o, "\t}}")?;
        }
        for (csid, csname, cslot) in &self.conn_slots {
            writeln!(o,)?;
            writeln!(o, "\tconnector_slot {csname} {{")?;
            writeln!(
                o,
                "\t\topposite {oname};",
                oname = self.conn_slots.key(cslot.opposite)
            )?;
            for (_, name, ccls) in &self.conn_classes {
                if ccls.slot != csid {
                    continue;
                }
                writeln!(o,)?;
                writeln!(o, "\t\tconnector_class {name} {{",)?;
                for (w, ti) in &ccls.wires {
                    let wn = &self.wires.key(w);
                    match ti {
                        ConnectorWire::BlackHole => {
                            writeln!(o, "\t\t\tblackhole {wn};")?;
                        }
                        &ConnectorWire::Reflect(ow) => {
                            writeln!(o, "\t\t\treflect {wn} = {own};", own = self.wires.key(ow))?;
                        }
                        &ConnectorWire::Pass(ow) => {
                            writeln!(o, "\t\t\tpass {wn} = {own};", own = self.wires.key(ow))?;
                        }
                    }
                }
                writeln!(o, "\t\t}}")?;
            }
            writeln!(o, "\t}}")?;
        }

        for (_, tname, table) in &self.tables {
            writeln!(o)?;
            writeln!(o, "\ttable {tname} {{")?;
            for (_, fname, &typ) in &table.fields {
                writeln!(o, "\t\tfield {fname}: {typ};", typ = self.dump_typ(typ))?;
            }
            writeln!(o)?;
            for (_, rname, row) in &table.rows {
                if row.iter().next().is_some() {
                    writeln!(o, "\t\trow {rname} {{")?;
                    for (fid, value) in row {
                        writeln!(
                            o,
                            "\t\t\t{fname} = {value};",
                            fname = table.fields.key(fid),
                            value = self.dump_value(table.fields[fid], value)
                        )?;
                    }
                    writeln!(o, "\t\t}}")?;
                } else {
                    writeln!(o, "\t\trow {rname};")?;
                }
            }
            writeln!(o, "\t}}")?;
        }

        if !self.devdata.is_empty() {
            writeln!(o)?;
            for (_, name, &typ) in &self.devdata {
                writeln!(o, "\tdevice_data {name}: {typ};", typ = self.dump_typ(typ))?;
            }
        }

        writeln!(o, "}}")?;
        Ok(())
    }
}
