use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt::Write,
};

use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use prjcombine_entity::{EntityBundleItemIndex, EntityId};
use prjcombine_interconnect::db::{
    BelAttribute, BelAttributeId, BelAttributeType, BelClassId, BelInfo, BelInput, BelInputId,
    BelKind, BelSlotId, ConnectorSlotId, ConnectorWire, IntDb, PinDir, PolTileWireCoord, SwitchBox,
    SwitchBoxItem, TableId, TableValue, TileClass, TileClassId, TileWireCoord, WireKind,
};
use prjcombine_types::{
    bitvec::BitVec,
    bsdata::{FrameOrientation, PolTileBit, RectBitId, RectFrameId, TileBit},
};

use crate::DocgenContext;

fn gen_intdb_basics(ctx: &mut DocgenContext, dbname: &str, intdb: &IntDb) {
    let mut buf = String::new();

    writeln!(buf, "## Tile slots").unwrap();
    writeln!(buf).unwrap();
    writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
    writeln!(buf, r#"<caption>{dbname} tile slots</caption>"#).unwrap();
    writeln!(buf, r#"<thead>"#).unwrap();
    writeln!(
        buf,
        r#"<tr><th>Slot</th><th>Tiles</th><th>Bel slots</th></tr>"#
    )
    .unwrap();
    writeln!(buf, r#"</thead>"#).unwrap();
    writeln!(buf, r#"<tbody>"#).unwrap();
    for (id, name) in &intdb.tile_slots {
        let tiles = intdb
            .tile_classes
            .iter()
            .filter(|&(_, _, tcls)| tcls.slot == id)
            .map(|(_, name, _)| name.as_str())
            .join(", ");
        let bels = intdb
            .bel_slots
            .iter()
            .filter(|&(_, _, bslot)| bslot.tile_slot == id)
            .map(|(_, name, _)| name.as_str())
            .join(", ");
        writeln!(
            buf,
            r#"<tr><td>{name}</td><td>{tiles}</td><td>{bels}</td></tr>"#
        )
        .unwrap();
    }
    writeln!(buf, r#"</tbody>"#).unwrap();
    writeln!(buf, r#"</table></div>"#).unwrap();
    writeln!(buf).unwrap();

    writeln!(buf, "## Bel slots").unwrap();
    writeln!(buf).unwrap();
    writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
    writeln!(buf, r#"<caption>{dbname} bel slots</caption>"#).unwrap();
    writeln!(buf, r#"<thead>"#).unwrap();
    writeln!(
        buf,
        r#"<tr><th>Slot</th><th>Class</th><th>Tile slot</th><th>Tiles</th></tr>"#
    )
    .unwrap();
    writeln!(buf, r#"</thead>"#).unwrap();
    writeln!(buf, r#"<tbody>"#).unwrap();
    for (id, name, bslot) in &intdb.bel_slots {
        let class = match bslot.kind {
            BelKind::Routing => "routing",
            BelKind::Class(bcid) => intdb.bel_classes.key(bcid),
            BelKind::Legacy => "legacy",
        };
        let tiles = intdb
            .tile_classes
            .iter()
            .filter(|&(_, _, tcls)| tcls.bels.contains_id(id))
            .map(|(_, name, _)| name.as_str())
            .join(", ");
        let tslot = intdb.tile_slots[bslot.tile_slot].as_str();
        writeln!(
            buf,
            r#"<tr><td>{name}</td><td>{class}</td><td>{tslot}</td><td>{tiles}</td></tr>"#
        )
        .unwrap();
    }
    writeln!(buf, r#"</tbody>"#).unwrap();
    writeln!(buf, r#"</table></div>"#).unwrap();
    writeln!(buf).unwrap();

    writeln!(buf, "## Connector slots").unwrap();
    writeln!(buf).unwrap();
    writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
    writeln!(buf, r#"<caption>{dbname} connector slots</caption>"#).unwrap();
    writeln!(buf, r#"<thead>"#).unwrap();
    writeln!(
        buf,
        r#"<tr><th>Slot</th><th>Opposite</th><th>Connectors</th></tr>"#
    )
    .unwrap();
    writeln!(buf, r#"</thead>"#).unwrap();
    writeln!(buf, r#"<tbody>"#).unwrap();
    for (id, name, slot) in &intdb.conn_slots {
        let conns = intdb
            .conn_classes
            .iter()
            .filter(|&(_, _, ccls)| ccls.slot == id)
            .map(|(_, name, _)| name.as_str())
            .join(", ");
        let opposite = intdb.conn_slots.key(slot.opposite);
        writeln!(
            buf,
            r#"<tr><td>{name}</td><td>{opposite}</td><td>{conns}</td></tr>"#
        )
        .unwrap();
    }
    writeln!(buf, r#"</tbody>"#).unwrap();
    writeln!(buf, r#"</table></div>"#).unwrap();
    writeln!(buf).unwrap();

    writeln!(buf, "## Region slots").unwrap();
    writeln!(buf).unwrap();
    writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
    writeln!(buf, r#"<caption>{dbname} region slots</caption>"#).unwrap();
    writeln!(buf, r#"<thead>"#).unwrap();
    writeln!(buf, r#"<tr><th>Slot</th><th>Wires</th></tr>"#).unwrap();
    writeln!(buf, r#"</thead>"#).unwrap();
    writeln!(buf, r#"<tbody>"#).unwrap();
    for (id, name) in &intdb.region_slots {
        let wires = intdb
            .wires
            .iter()
            .filter(|&(_, _, wkind)| *wkind == WireKind::Regional(id))
            .map(|(_, name, _)| name.as_str())
            .join(", ");
        writeln!(buf, r#"<tr><td>{name}</td><td>{wires}</td></tr>"#).unwrap();
    }
    writeln!(buf, r#"</tbody>"#).unwrap();
    writeln!(buf, r#"</table></div>"#).unwrap();
    writeln!(buf).unwrap();

    writeln!(buf, "## Wires").unwrap();
    writeln!(buf).unwrap();
    writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
    writeln!(buf, r#"<caption>{dbname} wires</caption>"#).unwrap();
    writeln!(buf, r#"<thead>"#).unwrap();
    writeln!(buf, r#"<tr><th>Wire</th><th>Kind</th>"#).unwrap();
    writeln!(buf, r#"</tr>"#).unwrap();
    writeln!(buf, r#"</thead>"#).unwrap();
    writeln!(buf, r#"<tbody>"#).unwrap();
    for (_, name, kind) in &intdb.wires {
        let kind = kind.to_string(intdb);
        writeln!(buf, r#"<tr><td>{name}</td><td>{kind}</td></tr>"#).unwrap();
    }
    writeln!(buf, r#"</tbody>"#).unwrap();
    writeln!(buf, r#"</table></div>"#).unwrap();
    writeln!(buf).unwrap();

    for (csid, csname, _) in &intdb.conn_slots {
        writeln!(buf, "## Connectors — {csname}").unwrap();
        writeln!(buf).unwrap();
        writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
        writeln!(buf, r#"<caption>{dbname} wires</caption>"#).unwrap();
        writeln!(buf, r#"<thead>"#).unwrap();
        writeln!(buf, r#"<tr><th>Wire</th>"#).unwrap();
        for (_, name, ccls) in &intdb.conn_classes {
            if ccls.slot == csid {
                writeln!(buf, r#"<th>{name}</th>"#).unwrap();
            }
        }
        writeln!(buf, r#"</tr>"#).unwrap();
        writeln!(buf, r#"</thead>"#).unwrap();
        writeln!(buf, r#"<tbody>"#).unwrap();
        for (id, name, &kind) in &intdb.wires {
            if kind != WireKind::Branch(csid)
                && kind != WireKind::MultiBranch(csid)
                && kind != WireKind::MultiBranch(csid)
            {
                continue;
            }
            writeln!(buf, r#"<tr><td>{name}</td>"#).unwrap();
            for (_, _, ccls) in &intdb.conn_classes {
                if ccls.slot != csid {
                    continue;
                }
                if let Some(&cw) = ccls.wires.get(id) {
                    match cw {
                        ConnectorWire::BlackHole => {
                            writeln!(buf, r#"<td>[BLACKHOLE]</td>"#).unwrap();
                        }
                        ConnectorWire::Reflect(wid) => {
                            let wire = intdb.wires.key(wid);
                            writeln!(buf, r#"<td>← {wire}</td>"#).unwrap();
                        }
                        ConnectorWire::Pass(wid) => {
                            let wire = intdb.wires.key(wid);
                            writeln!(buf, r#"<td>→ {wire}</td>"#).unwrap();
                        }
                    }
                } else {
                    writeln!(buf, r#"<td>-</td>"#).unwrap();
                }
            }
            writeln!(buf, r#"</tr>"#).unwrap();
        }
        writeln!(buf, r#"</tbody>"#).unwrap();
        writeln!(buf, r#"</table></div>"#).unwrap();
        writeln!(buf).unwrap();
    }

    ctx.items.insert(format!("int-basics-{dbname}"), buf);
}

fn gen_switchbox_old(
    _ctx: &mut DocgenContext,
    buf: &mut String,
    dbname: &str,
    intdb: &IntDb,
    tcid: TileClassId,
    bslot: BelSlotId,
    sb: &SwitchBox,
) {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum PipKind {
        Mux(bool),
        PermaBuf(bool),
        ProgBuf(bool),
        Pass,
        BiPass,
        ProgInv,
        ProgDelay(bool, usize),
    }

    let tcls = &intdb.tile_classes[tcid];
    let tname = intdb.tile_classes.key(tcid).as_str();
    let bname = intdb.bel_slots.key(bslot).as_str();
    let mut pips: BTreeMap<TileWireCoord, BTreeSet<(PipKind, TileWireCoord)>> = BTreeMap::new();
    for item in &sb.items {
        match item {
            SwitchBoxItem::Mux(mux) => {
                for &src in mux.src.keys() {
                    pips.entry(mux.dst)
                        .or_default()
                        .insert((PipKind::Mux(src.inv), src.tw));
                }
            }
            SwitchBoxItem::ProgBuf(buf) => {
                pips.entry(buf.dst)
                    .or_default()
                    .insert((PipKind::ProgBuf(buf.src.inv), buf.src.tw));
            }
            SwitchBoxItem::PermaBuf(buf) => {
                pips.entry(buf.dst)
                    .or_default()
                    .insert((PipKind::PermaBuf(buf.src.inv), buf.src.tw));
            }
            SwitchBoxItem::Pass(pass) => {
                pips.entry(pass.dst)
                    .or_default()
                    .insert((PipKind::Pass, pass.src));
            }
            SwitchBoxItem::BiPass(pass) => {
                pips.entry(pass.a)
                    .or_default()
                    .insert((PipKind::BiPass, pass.b));
                pips.entry(pass.b)
                    .or_default()
                    .insert((PipKind::BiPass, pass.a));
            }
            SwitchBoxItem::ProgInv(inv) => {
                pips.entry(inv.dst)
                    .or_default()
                    .insert((PipKind::ProgInv, inv.src));
            }
            SwitchBoxItem::ProgDelay(delay) => {
                pips.entry(delay.dst).or_default().insert((
                    PipKind::ProgDelay(delay.src.inv, delay.steps.len()),
                    delay.src.tw,
                ));
            }
            SwitchBoxItem::Bidi(_) => unreachable!(),
        }
    }
    writeln!(buf, r#"### Switchbox {bname}"#).unwrap();
    writeln!(buf).unwrap();
    writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
    writeln!(
        buf,
        r#"<caption>{dbname} {tname} switchbox {bname}</caption>"#
    )
    .unwrap();
    writeln!(buf, r#"<thead>"#).unwrap();
    writeln!(
        buf,
        r#"<tr><th>Destination</th><th>Source</th><th>Kind</th></tr>"#
    )
    .unwrap();
    writeln!(buf, r#"</thead>"#).unwrap();
    writeln!(buf, r#"<tbody>"#).unwrap();
    for (dst, srcs) in &pips {
        let mut first = true;
        for &(kind, src) in srcs {
            write!(buf, r#"<tr>"#).unwrap();
            if first {
                write!(
                    buf,
                    r#"<td rowspan="{n}">{dst}</td>"#,
                    n = srcs.len(),
                    dst = dst.to_string(intdb, tcls)
                )
                .unwrap();
                first = false;
            }
            let k = match kind {
                PipKind::Mux(false) => "mux".to_string(),
                PipKind::Mux(true) => "inverted mux".to_string(),
                PipKind::PermaBuf(false) => "fixed buffer".to_string(),
                PipKind::PermaBuf(true) => "inverted fixed buffer".to_string(),
                PipKind::ProgBuf(false) => "buffer".to_string(),
                PipKind::ProgBuf(true) => "inverted buffer".to_string(),
                PipKind::Pass => "pass transistor".to_string(),
                PipKind::BiPass => "bidirectional pass transistor".to_string(),
                PipKind::ProgInv => "programmable inverter".to_string(),
                PipKind::ProgDelay(false, n) => format!("{n}-tap delay"),
                PipKind::ProgDelay(true, n) => format!("inverted {n}-tap delay"),
            };
            writeln!(
                buf,
                r#"<td>{src}</td><td>{k}</td>"#,
                src = src.to_string(intdb, tcls)
            )
            .unwrap();
            writeln!(buf, r#"</tr>"#).unwrap();
        }
    }
    writeln!(buf, r#"</tbody>"#).unwrap();
    writeln!(buf, r#"</table></div>"#).unwrap();
    writeln!(buf).unwrap();
}

#[derive(Copy, Clone)]
enum BitInfo {
    ProgBuf(BelSlotId, TileWireCoord, PolTileWireCoord, bool),
    Pass(BelSlotId, TileWireCoord, TileWireCoord, bool),
    BiPass(BelSlotId, TileWireCoord, TileWireCoord, bool),
    ProgInv(BelSlotId, TileWireCoord, TileWireCoord, bool),
    Mux(BelSlotId, TileWireCoord, usize),
    Bidi(BelSlotId, ConnectorSlotId, TileWireCoord, bool),
    BelInputInv(BelSlotId, BelInputId, bool),
    BelAttrBool(BelSlotId, BelAttributeId, bool),
    BelAttrBitVec(BelSlotId, BelAttributeId, usize, bool),
}

struct TileClassGen<'a, 'b, 'c> {
    ctx: &'b mut DocgenContext<'c>,
    dbname: &'a str,
    intdb: &'a IntDb,
    tcls: &'a TileClass,
    tname: &'a str,
    bits: HashMap<TileBit, Vec<BitInfo>>,
    wires: BTreeMap<TileWireCoord, Vec<(BelSlotId, String)>>,
}

impl<'a, 'b, 'c> TileClassGen<'a, 'b, 'c> {
    fn add_bit(&mut self, bit: TileBit, info: BitInfo) {
        self.bits.entry(bit).or_default().push(info);
    }

    fn anchor(&self, info: BitInfo) -> String {
        match info {
            BitInfo::ProgBuf(bslot, dst, src, _) => format!(
                "{dbname}-{tname}-{bname}-progbuf-{dst}-{src}",
                dbname = self.dbname,
                tname = self.tname,
                bname = self.intdb.bel_slots.key(bslot),
                dst = dst.to_string(self.intdb, self.tcls),
                src = src.to_string(self.intdb, self.tcls),
            ),
            BitInfo::Pass(bslot, dst, src, _) => format!(
                "{dbname}-{tname}-{bname}-pass-{dst}-{src}",
                dbname = self.dbname,
                tname = self.tname,
                bname = self.intdb.bel_slots.key(bslot),
                dst = dst.to_string(self.intdb, self.tcls),
                src = src.to_string(self.intdb, self.tcls),
            ),
            BitInfo::BiPass(bslot, a, b, _) => format!(
                "{dbname}-{tname}-{bname}-bipass-{a}-{b}",
                dbname = self.dbname,
                tname = self.tname,
                bname = self.intdb.bel_slots.key(bslot),
                a = a.to_string(self.intdb, self.tcls),
                b = b.to_string(self.intdb, self.tcls),
            ),
            BitInfo::ProgInv(bslot, dst, src, _) => format!(
                "{dbname}-{tname}-{bname}-proginv-{dst}-{src}",
                dbname = self.dbname,
                tname = self.tname,
                bname = self.intdb.bel_slots.key(bslot),
                dst = dst.to_string(self.intdb, self.tcls),
                src = src.to_string(self.intdb, self.tcls),
            ),
            BitInfo::Mux(bslot, dst, idx) => format!(
                "{dbname}-{tname}-{bname}-mux-{dst}-{idx}",
                dbname = self.dbname,
                tname = self.tname,
                bname = self.intdb.bel_slots.key(bslot),
                dst = dst.to_string(self.intdb, self.tcls),
            ),
            BitInfo::Bidi(bslot, conn, wire, _) => format!(
                "{dbname}-{tname}-{bname}-bidi-{conn}-{wire}",
                dbname = self.dbname,
                tname = self.tname,
                bname = self.intdb.bel_slots.key(bslot),
                conn = self.intdb.conn_slots.key(conn),
                wire = wire.to_string(self.intdb, self.tcls),
            ),
            BitInfo::BelInputInv(bslot, pid, _) => {
                let BelKind::Class(bcid) = self.intdb.bel_slots[bslot].kind else {
                    unreachable!()
                };
                let bcls = &self.intdb.bel_classes[bcid];
                let (pname, idx) = bcls.inputs.key(pid);
                match idx {
                    EntityBundleItemIndex::Single => format!(
                        "{dbname}-{tname}-{bname}-inpinv-{pname}",
                        dbname = self.dbname,
                        tname = self.tname,
                        bname = self.intdb.bel_slots.key(bslot),
                    ),
                    EntityBundleItemIndex::Array { index, .. } => format!(
                        "{dbname}-{tname}-{bname}-inpinv-{pname}[{index}]",
                        dbname = self.dbname,
                        tname = self.tname,
                        bname = self.intdb.bel_slots.key(bslot),
                    ),
                }
            }
            BitInfo::BelAttrBool(bslot, aid, _) => {
                let BelKind::Class(bcid) = self.intdb.bel_slots[bslot].kind else {
                    unreachable!()
                };
                let bcls = &self.intdb.bel_classes[bcid];
                format!(
                    "{dbname}-{tname}-{bname}-{attr}",
                    dbname = self.dbname,
                    tname = self.tname,
                    bname = self.intdb.bel_slots.key(bslot),
                    attr = bcls.attributes.key(aid),
                )
            }
            BitInfo::BelAttrBitVec(bslot, aid, idx, _) => {
                let BelKind::Class(bcid) = self.intdb.bel_slots[bslot].kind else {
                    unreachable!()
                };
                let bcls = &self.intdb.bel_classes[bcid];
                format!(
                    "{dbname}-{tname}-{bname}-{attr}[{idx}]",
                    dbname = self.dbname,
                    tname = self.tname,
                    bname = self.intdb.bel_slots.key(bslot),
                    attr = bcls.attributes.key(aid),
                )
            }
        }
    }

    fn link_bit(&self, bit: TileBit) -> String {
        let bit = self.tcls.dump_bit(bit);
        let anchor = format!(
            "{dbname}-{tname}-bit-{bit}",
            dbname = self.dbname,
            tname = self.tname
        );
        format!(r##"<a href="#{anchor}">{bit}</a>"##)
    }

    fn link_polbit(&self, bit: PolTileBit) -> String {
        let text = self.tcls.dump_polbit(bit);
        let bit = self.tcls.dump_bit(bit.bit);
        let anchor = format!(
            "{dbname}-{tname}-bit-{bit}",
            dbname = self.dbname,
            tname = self.tname
        );
        format!(r##"<a href="#{anchor}">{text}</a>"##)
    }

    fn classify_mux(&self, wire: TileWireCoord) -> String {
        let wname = self.intdb.wires.key(wire.wire);
        match self.dbname {
            "siliconblue" => {
                if wname.starts_with("GLOBAL_ROOT") {
                    "GLOBAL_ROOT".to_string()
                } else if wname.starts_with("GLOBAL_OUT") {
                    "GLOBAL_OUT".to_string()
                } else if wname.starts_with("GLOBAL") {
                    "GLOBAL".to_string()
                } else if wname.starts_with("QUAD_H") {
                    "QUAD_H".to_string()
                } else if wname.starts_with("QUAD_V") {
                    "QUAD_V".to_string()
                } else if wname.starts_with("LONG_H") {
                    "LONG_H".to_string()
                } else if wname.starts_with("LONG_V") {
                    "LONG_V".to_string()
                } else if wname.starts_with("LOCAL") {
                    "LOCAL".to_string()
                } else if wname.starts_with("IMUX_LC") {
                    "IMUX_LC".to_string()
                } else if wname.starts_with("IMUX_IO_DOUT") {
                    "IMUX_IO_DOUT".to_string()
                } else if wname.starts_with("IMUX_IO_OE") {
                    "IMUX_IO_OE".to_string()
                } else if wname.starts_with("IMUX_IO") && wname.ends_with("CLK") {
                    "IMUX_IO_CLK".to_string()
                } else {
                    wname.to_string()
                }
            }
            _ => wname.to_string(),
        }
    }
}

fn gen_switchbox(tcgen: &mut TileClassGen, buf: &mut String, bslot: BelSlotId, sb: &SwitchBox) {
    let tcls = tcgen.tcls;
    let intdb = tcgen.intdb;
    let dbname = tcgen.dbname;
    let tname = tcgen.tname;
    let bname = tcgen.intdb.bel_slots.key(bslot).as_str();

    writeln!(buf, r#"### Switchbox {bname}"#).unwrap();
    writeln!(buf).unwrap();

    if sb
        .items
        .iter()
        .any(|x| matches!(x, SwitchBoxItem::PermaBuf(_)))
    {
        writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
        writeln!(
            buf,
            r#"<caption>{dbname} {tname} switchbox {bname} permanent buffers</caption>"#
        )
        .unwrap();
        writeln!(buf, r#"<thead>"#).unwrap();
        writeln!(buf, r#"<tr><th>Destination</th><th>Source</th></tr>"#).unwrap();
        writeln!(buf, r#"</thead>"#).unwrap();
        writeln!(buf, r#"<tbody>"#).unwrap();
        for item in &sb.items {
            let SwitchBoxItem::PermaBuf(pbuf) = item else {
                continue;
            };
            writeln!(
                buf,
                r#"<tr><td>{dst}</td><td>{src}</td></tr>"#,
                dst = pbuf.dst.to_string(intdb, tcls),
                src = pbuf.src.to_string(intdb, tcls)
            )
            .unwrap();
        }
        writeln!(buf, r#"</tbody>"#).unwrap();
        writeln!(buf, r#"</table></div>"#).unwrap();
        writeln!(buf).unwrap();
    }

    if sb
        .items
        .iter()
        .any(|x| matches!(x, SwitchBoxItem::ProgBuf(_)))
    {
        writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
        writeln!(
            buf,
            r#"<caption>{dbname} {tname} switchbox {bname} programmable buffers</caption>"#
        )
        .unwrap();
        writeln!(buf, r#"<thead>"#).unwrap();
        writeln!(
            buf,
            r#"<tr><th>Destination</th><th>Source</th><th>Bit</th></tr>"#
        )
        .unwrap();
        writeln!(buf, r#"</thead>"#).unwrap();
        writeln!(buf, r#"<tbody>"#).unwrap();
        for item in &sb.items {
            let SwitchBoxItem::ProgBuf(pbuf) = item else {
                continue;
            };
            let bi = BitInfo::ProgBuf(bslot, pbuf.dst, pbuf.src, pbuf.bit.inv);
            writeln!(
                buf,
                r#"<tr id="{anchor}"><td>{dst}</td><td>{src}</td><td>{bit}</td></tr>"#,
                anchor = tcgen.anchor(bi),
                dst = pbuf.dst.to_string(intdb, tcls),
                src = pbuf.src.to_string(intdb, tcls),
                bit = tcgen.link_polbit(pbuf.bit),
            )
            .unwrap();
            tcgen.add_bit(pbuf.bit.bit, bi);
        }
        writeln!(buf, r#"</tbody>"#).unwrap();
        writeln!(buf, r#"</table></div>"#).unwrap();
        writeln!(buf).unwrap();
    }

    if sb.items.iter().any(|x| matches!(x, SwitchBoxItem::Pass(_))) {
        writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
        writeln!(
            buf,
            r#"<caption>{dbname} {tname} switchbox {bname} pass gates</caption>"#
        )
        .unwrap();
        writeln!(buf, r#"<thead>"#).unwrap();
        writeln!(
            buf,
            r#"<tr><th>Destination</th><th>Source</th><th>Bit</th></tr>"#
        )
        .unwrap();
        writeln!(buf, r#"</thead>"#).unwrap();
        writeln!(buf, r#"<tbody>"#).unwrap();
        for item in &sb.items {
            let SwitchBoxItem::Pass(pass) = item else {
                continue;
            };
            let bi = BitInfo::Pass(bslot, pass.dst, pass.src, pass.bit.inv);
            writeln!(
                buf,
                r#"<tr id="{anchor}"><td>{dst}</td><td>{src}</td><td>{bit}</td></tr>"#,
                anchor = tcgen.anchor(bi),
                dst = pass.dst.to_string(intdb, tcls),
                src = pass.src.to_string(intdb, tcls),
                bit = tcgen.link_polbit(pass.bit),
            )
            .unwrap();
            tcgen.add_bit(pass.bit.bit, bi);
        }
        writeln!(buf, r#"</tbody>"#).unwrap();
        writeln!(buf, r#"</table></div>"#).unwrap();
        writeln!(buf).unwrap();
    }

    if sb
        .items
        .iter()
        .any(|x| matches!(x, SwitchBoxItem::BiPass(_)))
    {
        writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
        writeln!(
            buf,
            r#"<caption>{dbname} {tname} switchbox {bname} bidirectional pass gates</caption>"#
        )
        .unwrap();
        writeln!(buf, r#"<thead>"#).unwrap();
        writeln!(
            buf,
            r#"<tr><th>Side A</th><th>Side B</th><th>Bit</th></tr>"#
        )
        .unwrap();
        writeln!(buf, r#"</thead>"#).unwrap();
        writeln!(buf, r#"<tbody>"#).unwrap();
        for item in &sb.items {
            let SwitchBoxItem::BiPass(pass) = item else {
                continue;
            };
            let bi = BitInfo::BiPass(bslot, pass.a, pass.b, pass.bit.inv);
            writeln!(
                buf,
                r#"<tr id="{anchor}"><td>{a}</td><td>{b}</td><td>{bit}</td></tr>"#,
                anchor = tcgen.anchor(bi),
                a = pass.a.to_string(intdb, tcls),
                b = pass.b.to_string(intdb, tcls),
                bit = tcgen.link_polbit(pass.bit),
            )
            .unwrap();
            tcgen.add_bit(pass.bit.bit, bi);
        }
        writeln!(buf, r#"</tbody>"#).unwrap();
        writeln!(buf, r#"</table></div>"#).unwrap();
        writeln!(buf).unwrap();
    }

    if sb
        .items
        .iter()
        .any(|x| matches!(x, SwitchBoxItem::ProgInv(_)))
    {
        writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
        writeln!(
            buf,
            r#"<caption>{dbname} {tname} switchbox {bname} programmable inverters</caption>"#
        )
        .unwrap();
        writeln!(buf, r#"<thead>"#).unwrap();
        writeln!(
            buf,
            r#"<tr><th>Destination</th><th>Source</th><th>Bit</th></tr>"#
        )
        .unwrap();
        writeln!(buf, r#"</thead>"#).unwrap();
        writeln!(buf, r#"<tbody>"#).unwrap();
        for item in &sb.items {
            let SwitchBoxItem::ProgInv(pbuf) = item else {
                continue;
            };
            let bi = BitInfo::ProgInv(bslot, pbuf.dst, pbuf.src, pbuf.bit.inv);
            writeln!(
                buf,
                r#"<tr id="{anchor}"><td>{dst}</td><td>{src}</td><td>{bit}</td></tr>"#,
                anchor = tcgen.anchor(bi),
                dst = pbuf.dst.to_string(intdb, tcls),
                src = pbuf.src.to_string(intdb, tcls),
                bit = tcgen.link_polbit(pbuf.bit),
            )
            .unwrap();
            tcgen.add_bit(pbuf.bit.bit, bi);
        }
        writeln!(buf, r#"</tbody>"#).unwrap();
        writeln!(buf, r#"</table></div>"#).unwrap();
        writeln!(buf).unwrap();
    }

    if sb.items.iter().any(|x| matches!(x, SwitchBoxItem::Bidi(_))) {
        writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
        writeln!(
            buf,
            r#"<caption>{dbname} {tname} switchbox {bname} bidi buffers</caption>"#
        )
        .unwrap();
        writeln!(buf, r#"<thead>"#).unwrap();
        writeln!(
            buf,
            r#"<tr><th>Connector</th><th>Wire</th><th>Bit</th></tr>"#
        )
        .unwrap();
        writeln!(buf, r#"</thead>"#).unwrap();
        writeln!(buf, r#"<tbody>"#).unwrap();
        for item in &sb.items {
            let SwitchBoxItem::Bidi(bidi) = item else {
                continue;
            };
            let bi = BitInfo::Bidi(bslot, bidi.conn, bidi.wire, bidi.bit_upstream.inv);
            writeln!(
                buf,
                r#"<tr id="{anchor}"><td>{conn}</td><td>{wire}</td><td>{bit}</td></tr>"#,
                anchor = tcgen.anchor(bi),
                conn = intdb.conn_slots.key(bidi.conn),
                wire = bidi.wire.to_string(intdb, tcls),
                bit = tcgen.link_polbit(bidi.bit_upstream),
            )
            .unwrap();
            tcgen.add_bit(bidi.bit_upstream.bit, bi);
        }
        writeln!(buf, r#"</tbody>"#).unwrap();
        writeln!(buf, r#"</table></div>"#).unwrap();
        writeln!(buf).unwrap();
    }

    let mut muxes: IndexMap<_, Vec<_>> = IndexMap::new();
    for item in &sb.items {
        let SwitchBoxItem::Mux(mux) = item else {
            continue;
        };
        let cat = tcgen.classify_mux(mux.dst);
        muxes.entry((cat, mux.bits.len())).or_default().push(mux);
    }

    for ((cat, nbits), muxes) in muxes {
        let mut slot_set = IndexSet::new();
        let mut mux_slot = vec![];
        for &mux in &muxes {
            let key = (&mux.src, &mux.bits_off);
            let (idx, _) = slot_set.insert_full(key);
            mux_slot.push(idx);
        }
        writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
        writeln!(
            buf,
            r#"<caption>{dbname} {tname} switchbox {bname} muxes {cat}</caption>"#
        )
        .unwrap();
        writeln!(buf, r#"<thead>"#).unwrap();
        writeln!(
            buf,
            r#"<tr><th colspan="{nbits}">Bits</th><th colspan="{nmuxes}">Destination</th></tr>"#,
            nmuxes = slot_set.len()
        )
        .unwrap();
        writeln!(buf, r#"</thead>"#).unwrap();
        writeln!(buf, r#"<tbody>"#).unwrap();
        let mut values = BTreeMap::new();
        for (idx, mux) in muxes.iter().copied().enumerate() {
            let slot = mux_slot[idx];
            write!(buf, r#"<tr>"#).unwrap();
            for (bidx, &bit) in mux.bits.iter().enumerate().rev() {
                let bi = BitInfo::Mux(bslot, mux.dst, bidx);
                tcgen.add_bit(bit, bi);
                write!(
                    buf,
                    r#"<td id="{anchor}">{bit}</td>"#,
                    anchor = tcgen.anchor(bi),
                    bit = tcgen.link_bit(bit),
                )
                .unwrap();
            }
            for _ in 0..slot {
                write!(buf, r#"<td>-</td>"#).unwrap();
            }
            write!(
                buf,
                r#"<td>{dst}</td>"#,
                dst = mux.dst.to_string(intdb, tcls),
            )
            .unwrap();
            for _ in (slot + 1)..slot_set.len() {
                write!(buf, r#"<td>-</td>"#).unwrap();
            }
            writeln!(buf, r#"</tr>"#).unwrap();
            for (src, val) in &mux.src {
                let val = BitVec::from_iter(val.iter().rev());
                values
                    .entry(val)
                    .or_insert_with(|| vec![None; slot_set.len()])[slot] =
                    Some(src.to_string(intdb, tcls));
            }
            if let Some(ref val) = mux.bits_off {
                let val = BitVec::from_iter(val.iter().rev());
                values
                    .entry(val)
                    .or_insert_with(|| vec![None; slot_set.len()])[slot] = Some("off".to_string());
            }
        }
        writeln!(
            buf,
            r#"<tr><th colspan="{nbits}"></th><th colspan="{nmuxes}">Source</th></tr>"#,
            nmuxes = slot_set.len()
        )
        .unwrap();
        for (value, srcs) in values {
            write!(buf, r#"<tr>"#).unwrap();
            for bit in value {
                write!(buf, r#"<td>{b}</td>"#, b = u8::from(bit)).unwrap();
            }
            for src in srcs {
                let src = src.unwrap_or_else(|| "-".to_string());
                write!(buf, r#"<td>{src}</td>"#).unwrap();
            }
            writeln!(buf, r#"</tr>"#).unwrap();
        }
        writeln!(buf, r#"</tbody>"#).unwrap();
        writeln!(buf, r#"</table></div>"#).unwrap();
        writeln!(buf).unwrap();
    }

    // TODO: prog delay
}

fn gen_bels(tcgen: &mut TileClassGen, buf: &mut String, bcid: BelClassId, bslots: &[BelSlotId]) {
    let tcls = tcgen.tcls;
    let intdb = tcgen.intdb;
    let dbname = tcgen.dbname;
    let tname = tcgen.tname;
    let bcls = &intdb.bel_classes[bcid];
    let bcname = intdb.bel_classes.key(bcid);
    let bels = Vec::from_iter(bslots.iter().map(|&slot| {
        let BelInfo::Bel(ref bel) = tcls.bels[slot] else {
            unreachable!()
        };
        bel
    }));
    writeln!(buf, r#"### Bels {bcname}"#).unwrap();
    writeln!(buf).unwrap();

    writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
    writeln!(
        buf,
        r#"<caption>{dbname} {tname} bel {bcname} pins</caption>"#
    )
    .unwrap();
    writeln!(buf, r#"<thead>"#).unwrap();
    write!(buf, r#"<tr><th>Pin</th><th>Direction</th>"#).unwrap();
    for &slot in bslots {
        write!(buf, r#"<th>{}</th>"#, intdb.bel_slots.key(slot)).unwrap();
    }
    writeln!(buf, r#"</tr>"#).unwrap();
    writeln!(buf, r#"</thead>"#).unwrap();
    writeln!(buf, r#"<tbody>"#).unwrap();
    for (pid, pname, idx, _cinp) in bcls.inputs.iter() {
        let pname = match idx {
            EntityBundleItemIndex::Single => pname.to_string(),
            EntityBundleItemIndex::Array { index, .. } => format!("{pname}[{index}]"),
        };
        if !bels.iter().any(|bel| bel.inputs.contains_id(pid)) {
            continue;
        }
        write!(buf, r#"<tr><td>{pname}</td><td>in</td>"#).unwrap();
        for (&bslot, bel) in bslots.iter().zip(&bels) {
            match bel.inputs.get(pid) {
                None => {
                    write!(buf, r#"<td>-</td>"#).unwrap();
                }
                Some(BelInput::Fixed(ptwc)) => {
                    tcgen
                        .wires
                        .entry(ptwc.tw)
                        .or_default()
                        .push((bslot, pname.clone()));
                    write!(
                        buf,
                        r#"<td>{wire}</td>"#,
                        wire = ptwc.to_string(intdb, tcls)
                    )
                    .unwrap();
                }
                Some(BelInput::Invertible(twc, bit)) => {
                    tcgen
                        .wires
                        .entry(*twc)
                        .or_default()
                        .push((bslot, pname.clone()));
                    let bi = BitInfo::BelInputInv(bslot, pid, bit.inv);
                    tcgen.add_bit(bit.bit, bi);
                    write!(
                        buf,
                        r#"<td id="{anchor}">{wire} invert by {bit}</td>"#,
                        anchor = tcgen.anchor(bi),
                        wire = twc.to_string(intdb, tcls),
                        bit = tcgen.link_polbit(*bit),
                    )
                    .unwrap();
                }
            };
        }
        writeln!(buf, r#"</tr>"#).unwrap();
    }
    for (pid, pname, idx, _cinp) in bcls.outputs.iter() {
        let pname = match idx {
            EntityBundleItemIndex::Single => pname.to_string(),
            EntityBundleItemIndex::Array { index, .. } => format!("{pname}[{index}]"),
        };
        if !bels.iter().any(|bel| bel.outputs.contains_id(pid)) {
            continue;
        }
        write!(buf, r#"<tr><td>{pname}</td><td>out</td>"#).unwrap();
        for (&bslot, bel) in bslots.iter().zip(&bels) {
            match bel.outputs.get(pid) {
                None => {
                    write!(buf, r#"<td>-</td>"#).unwrap();
                }
                Some(twcs) => {
                    for &twc in twcs {
                        tcgen
                            .wires
                            .entry(twc)
                            .or_default()
                            .push((bslot, pname.clone()));
                    }
                    let wires = twcs.iter().map(|twc| twc.to_string(intdb, tcls)).join(", ");
                    write!(buf, r#"<td>{wires}</td>"#).unwrap();
                }
            };
        }
        writeln!(buf, r#"</tr>"#).unwrap();
    }
    for (pid, pname, idx, _cinp) in bcls.bidirs.iter() {
        let pname = match idx {
            EntityBundleItemIndex::Single => pname.to_string(),
            EntityBundleItemIndex::Array { index, .. } => format!("{pname}[{index}]"),
        };
        if !bels.iter().any(|bel| bel.bidirs.contains_id(pid)) {
            continue;
        }
        write!(buf, r#"<tr><td>{pname}</td><td>bidir</td>"#).unwrap();
        for (&bslot, bel) in bslots.iter().zip(&bels) {
            match bel.bidirs.get(pid) {
                None => {
                    write!(buf, r#"<td>-</td>"#).unwrap();
                }
                Some(twc) => {
                    tcgen
                        .wires
                        .entry(*twc)
                        .or_default()
                        .push((bslot, pname.clone()));
                    let wire = twc.to_string(intdb, tcls);
                    write!(buf, r#"<td>{wire}</td>"#).unwrap();
                }
            };
        }
        writeln!(buf, r#"</tr>"#).unwrap();
    }
    writeln!(buf, r#"</tbody>"#).unwrap();
    writeln!(buf, r#"</table></div>"#).unwrap();
    writeln!(buf).unwrap();

    if !bcls.attributes.is_empty() {
        writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
        writeln!(
            buf,
            r#"<caption>{dbname} {tname} bel {bcname} attribute bits</caption>"#
        )
        .unwrap();
        writeln!(buf, r#"<thead>"#).unwrap();
        write!(buf, r#"<tr><th>Attribute</th>"#).unwrap();
        for &slot in bslots {
            write!(buf, r#"<th>{}</th>"#, intdb.bel_slots.key(slot)).unwrap();
        }
        writeln!(buf, r#"</tr>"#).unwrap();
        writeln!(buf, r#"</thead>"#).unwrap();
        writeln!(buf, r#"<tbody>"#).unwrap();

        let mut eattrs: IndexMap<_, Vec<_>> = IndexMap::new();

        for (aid, aname, attr) in &bcls.attributes {
            if !bels.iter().any(|bel| bel.attributes.contains_id(aid)) {
                continue;
            }

            match attr.typ {
                BelAttributeType::Enum(ecid) => {
                    write!(buf, r#"<tr><td>{aname}</td>"#).unwrap();
                    for (&bslot, bel) in bslots.iter().zip(&bels) {
                        match bel.attributes.get(aid) {
                            None => {
                                write!(buf, r#"<td>-</td>"#).unwrap();
                            }
                            Some(BelAttribute::Enum(edata)) => {
                                eattrs
                                    .entry((ecid, &edata.values))
                                    .or_default()
                                    .push((bslot, aid, edata));
                                write!(
                                    buf,
                                    r##"<td><a href="#{dbname}-{tname}-{bname}-{aname}">[enum: {ename}]</a></td>"##,
                                    bname = intdb.bel_slots.key(bslot),
                                    ename = intdb.enum_classes.key(ecid),
                                )
                                .unwrap();
                            }
                            _ => unreachable!(),
                        };
                    }
                    writeln!(buf, r#"</tr>"#).unwrap();
                }
                BelAttributeType::Bool => {
                    write!(buf, r#"<tr><td>{aname}</td>"#).unwrap();
                    for (&bslot, bel) in bslots.iter().zip(&bels) {
                        match bel.attributes.get(aid) {
                            None => {
                                write!(buf, r#"<td>-</td>"#).unwrap();
                            }
                            Some(BelAttribute::BitVec(bits)) => {
                                let bi = BitInfo::BelAttrBool(bslot, aid, bits[0].inv);
                                tcgen.add_bit(bits[0].bit, bi);
                                write!(
                                    buf,
                                    r#"<td id="{anchor}">{bit}</td>"#,
                                    anchor = tcgen.anchor(bi),
                                    bit = tcgen.link_polbit(bits[0])
                                )
                                .unwrap();
                            }
                            _ => unreachable!(),
                        };
                    }
                    writeln!(buf, r#"</tr>"#).unwrap();
                }
                BelAttributeType::Bitvec(width) => {
                    for idx in 0..width {
                        write!(buf, r#"<tr><td>{aname} bit {idx}</td>"#).unwrap();
                        for (&bslot, bel) in bslots.iter().zip(&bels) {
                            match bel.attributes.get(aid) {
                                None => {
                                    write!(buf, r#"<td>-</td>"#).unwrap();
                                }
                                Some(BelAttribute::BitVec(bits)) => {
                                    let bi = BitInfo::BelAttrBitVec(bslot, aid, idx, bits[idx].inv);
                                    tcgen.add_bit(bits[idx].bit, bi);
                                    write!(
                                        buf,
                                        r#"<td id="{anchor}">{bit}</td>"#,
                                        anchor = tcgen.anchor(bi),
                                        bit = tcgen.link_polbit(bits[idx])
                                    )
                                    .unwrap();
                                }
                                _ => unreachable!(),
                            };
                        }
                        writeln!(buf, r#"</tr>"#).unwrap();
                    }
                }
                BelAttributeType::BitvecArray(_, _) => todo!(),
            }
        }

        writeln!(buf, r#"</tbody>"#).unwrap();
        writeln!(buf, r#"</table></div>"#).unwrap();
        writeln!(buf).unwrap();

        for ((ecid, values), attrs) in eattrs {
            let ecls = &intdb.enum_classes[ecid];
            writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
            writeln!(
                buf,
                r#"<caption>{dbname} {tname} enum {ename}</caption>"#,
                ename = intdb.enum_classes.key(ecid),
            )
            .unwrap();
            writeln!(buf, r#"<thead>"#).unwrap();
            for (bslot, aid, attr) in attrs {
                write!(
                    buf,
                    r#"<tr id="{dbname}-{tname}-{bname}-{aname}"><th>{bname}.{aname}</th>"#,
                    bname = intdb.bel_slots.key(bslot),
                    aname = bcls.attributes.key(aid),
                )
                .unwrap();
                for (bidx, &bit) in attr.bits.iter().enumerate().rev() {
                    let bi = BitInfo::BelAttrBitVec(bslot, aid, bidx, false);
                    tcgen.add_bit(bit, bi);
                    write!(
                        buf,
                        r#"<td id="{anchor}">{bit}</td>"#,
                        anchor = tcgen.anchor(bi),
                        bit = tcgen.link_bit(bit),
                    )
                    .unwrap();
                }
                writeln!(buf, r#"</tr>"#).unwrap();
            }
            writeln!(buf, r#"</thead>"#).unwrap();
            writeln!(buf, r#"<tbody>"#).unwrap();

            for (vid, value) in values {
                write!(buf, r#"<tr><td>{vname}</td>"#, vname = ecls.values[vid]).unwrap();
                for bit in value.iter().rev() {
                    write!(buf, r#"<td>{b}</td>"#, b = u8::from(bit)).unwrap();
                }
                writeln!(buf, r#"</tr>"#).unwrap();
            }

            writeln!(buf, r#"</tbody>"#).unwrap();
            writeln!(buf, r#"</table></div>"#).unwrap();
            writeln!(buf).unwrap();
        }
    }
}

fn gen_bits(tcgen: &mut TileClassGen, buf: &mut String) {
    let tcls = tcgen.tcls;
    let intdb = tcgen.intdb;
    let dbname = tcgen.dbname;
    let tname = tcgen.tname;

    writeln!(buf, r#"### Bitstream"#).unwrap();
    writeln!(buf).unwrap();
    for (rect, rdata) in &tcls.bitrects {
        let frames = if rdata.geometry.rev_frames {
            Vec::from_iter((0..rdata.geometry.frames).map(RectFrameId::from_idx).rev())
        } else {
            Vec::from_iter((0..rdata.geometry.frames).map(RectFrameId::from_idx))
        };
        let bits = if rdata.geometry.rev_bits {
            Vec::from_iter((0..rdata.geometry.bits).map(RectBitId::from_idx).rev())
        } else {
            Vec::from_iter((0..rdata.geometry.bits).map(RectBitId::from_idx))
        };

        writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
        writeln!(
            buf,
            r#"<caption>{dbname} {tname} rect {rname}</caption>"#,
            rname = rdata.name
        )
        .unwrap();
        writeln!(buf, r#"<thead>"#).unwrap();
        match rdata.geometry.orientation {
            FrameOrientation::Horizontal => {
                writeln!(
                    buf,
                    r#"<tr><th rowspan="2">Frame</th><th colspan="{num_bits}">Bit</th></tr>"#,
                    num_bits = rdata.geometry.bits,
                )
                .unwrap();
                writeln!(buf, r#"<tr>"#).unwrap();
                for &bit in &bits {
                    writeln!(buf, r#"<th>{bit}</th>"#).unwrap();
                }
                writeln!(buf, r#"</tr>"#).unwrap();
            }
            FrameOrientation::Vertical => {
                writeln!(
                    buf,
                    r#"<tr><th rowspan="2">Bit</th><th colspan="{num_frames}">Frame</th></tr>"#,
                    num_frames = rdata.geometry.frames,
                )
                .unwrap();
                writeln!(buf, r#"<tr>"#).unwrap();
                for &frame in &frames {
                    writeln!(buf, r#"<th>{frame}</th>"#).unwrap();
                }
                writeln!(buf, r#"</tr>"#).unwrap();
            }
        }
        writeln!(buf, r#"</thead>"#).unwrap();
        writeln!(buf, r#"<tbody>"#).unwrap();

        let emit_bit = |buf: &mut String, tbit: TileBit| {
            if let Some(items) = tcgen.bits.get(&tbit) {
                writeln!(
                    buf,
                    r#"<td id="{dbname}-{tname}-bit-{bit}" title="{bit}">"#,
                    bit = tcls.dump_bit(tbit),
                )
                .unwrap();
                for &item in items {
                    let disp = match item {
                        BitInfo::ProgBuf(bslot, dst, src, inv) => {
                            format!(
                                "{bname}: {inv}buffer {dst} ← {src}",
                                bname = intdb.bel_slots.key(bslot),
                                inv = if inv { "!" } else { "" },
                                dst = dst.to_string(intdb, tcls),
                                src = src.to_string(intdb, tcls),
                            )
                        }
                        BitInfo::Pass(bslot, dst, src, inv) => {
                            format!(
                                "{bname}: {inv}pass {dst} ← {src}",
                                bname = intdb.bel_slots.key(bslot),
                                inv = if inv { "!" } else { "" },
                                dst = dst.to_string(intdb, tcls),
                                src = src.to_string(intdb, tcls),
                            )
                        }
                        BitInfo::BiPass(bslot, a, b, inv) => {
                            format!(
                                "{bname}: {inv}bipass {a} = {b}",
                                bname = intdb.bel_slots.key(bslot),
                                inv = if inv { "!" } else { "" },
                                a = a.to_string(intdb, tcls),
                                b = b.to_string(intdb, tcls),
                            )
                        }
                        BitInfo::ProgInv(bslot, dst, src, inv) => {
                            format!(
                                "{bname}: {inv}invert {dst} ← {src}",
                                bname = intdb.bel_slots.key(bslot),
                                inv = if inv { "!" } else { "" },
                                dst = dst.to_string(intdb, tcls),
                                src = src.to_string(intdb, tcls),
                            )
                        }
                        BitInfo::Mux(bslot, dst, idx) => {
                            format!(
                                "{bname}: mux {dst} bit {idx}",
                                bname = intdb.bel_slots.key(bslot),
                                dst = dst.to_string(intdb, tcls),
                            )
                        }
                        BitInfo::Bidi(bslot, conn, wire, inv) => {
                            format!(
                                "{bname}: {inv}bidi {conn} {wire}",
                                bname = intdb.bel_slots.key(bslot),
                                inv = if inv { "!" } else { "" },
                                conn = intdb.conn_slots.key(conn),
                                wire = wire.to_string(intdb, tcls),
                            )
                        }
                        BitInfo::BelInputInv(bslot, inp, inv) => {
                            let BelKind::Class(bcid) = intdb.bel_slots[bslot].kind else {
                                unreachable!()
                            };
                            let bcls = &intdb.bel_classes[bcid];
                            let (pname, idx) = bcls.inputs.key(inp);
                            match idx {
                                EntityBundleItemIndex::Single => format!(
                                    "{bname}: {inv}invert {pname}",
                                    bname = intdb.bel_slots.key(bslot),
                                    inv = if inv { "!" } else { "" },
                                ),
                                EntityBundleItemIndex::Array { index, .. } => format!(
                                    "{bname}: {inv}invert {pname}[{index}]",
                                    bname = intdb.bel_slots.key(bslot),
                                    inv = if inv { "!" } else { "" },
                                ),
                            }
                        }
                        BitInfo::BelAttrBool(bslot, aid, inv) => {
                            let BelKind::Class(bcid) = intdb.bel_slots[bslot].kind else {
                                unreachable!()
                            };
                            let bcls = &intdb.bel_classes[bcid];
                            format!(
                                "{bname}: {inv} {attr}",
                                bname = intdb.bel_slots.key(bslot),
                                inv = if inv { "!" } else { "" },
                                attr = bcls.attributes.key(aid),
                            )
                        }
                        BitInfo::BelAttrBitVec(bslot, aid, idx, inv) => {
                            let BelKind::Class(bcid) = intdb.bel_slots[bslot].kind else {
                                unreachable!()
                            };
                            let bcls = &intdb.bel_classes[bcid];
                            format!(
                                "{bname}: {inv} {attr} bit {idx}",
                                bname = intdb.bel_slots.key(bslot),
                                inv = if inv { "!" } else { "" },
                                attr = bcls.attributes.key(aid),
                            )
                        }
                    };
                    writeln!(
                        buf,
                        r##"<a href="#{anchor}">{disp}</a>"##,
                        anchor = tcgen.anchor(item),
                    )
                    .unwrap();
                }
                writeln!(buf, r#"</td>"#).unwrap();
            } else {
                writeln!(buf, r#"<td>-</td>"#).unwrap();
            }
        };
        match rdata.geometry.orientation {
            FrameOrientation::Horizontal => {
                for &frame in &frames {
                    writeln!(buf, r#"<tr><td>{frame}</td>"#).unwrap();
                    for &bit in &bits {
                        emit_bit(buf, TileBit { rect, frame, bit });
                    }
                    writeln!(buf, r#"</tr>"#).unwrap();
                }
            }
            FrameOrientation::Vertical => {
                for &bit in &bits {
                    writeln!(buf, r#"<tr><td>{bit}</td>"#).unwrap();
                    for &frame in &frames {
                        emit_bit(buf, TileBit { rect, frame, bit });
                    }
                    writeln!(buf, r#"</tr>"#).unwrap();
                }
            }
        }

        writeln!(buf, r#"</tbody>"#).unwrap();
        writeln!(buf, r#"</table></div>"#).unwrap();
    }
}

fn gen_tile(ctx: &mut DocgenContext, dbname: &str, intdb: &IntDb, tcid: TileClassId) {
    let tname = intdb.tile_classes.key(tcid);
    let tcls = &intdb[tcid];
    let mut buf = String::new();

    if matches!(dbname, "ultrascale" | "ultrascaleplus") && tcls.bels.iter().next().is_none() {
        return;
    }

    writeln!(buf, r#"## Tile {tname}"#).unwrap();
    writeln!(buf).unwrap();
    writeln!(buf, r#"Cells: {}"#, tcls.cells.len()).unwrap();
    writeln!(buf).unwrap();

    let mut tcgen = TileClassGen {
        ctx,
        dbname,
        intdb,
        tcls,
        tname,
        bits: Default::default(),
        wires: Default::default(),
    };

    let mut bels: IndexMap<_, Vec<_>> = IndexMap::new();

    for (slot, bel) in &tcls.bels {
        let bname = intdb.bel_slots.key(slot).as_str();
        match bel {
            BelInfo::SwitchBox(sb) => {
                if tcls.bitrects.is_empty() {
                    gen_switchbox_old(tcgen.ctx, &mut buf, dbname, intdb, tcid, slot, sb);
                } else {
                    gen_switchbox(&mut tcgen, &mut buf, slot, sb);
                }
            }
            BelInfo::Bel(_bel) => {
                let BelKind::Class(bcid) = intdb.bel_slots[slot].kind else {
                    unreachable!()
                };
                bels.entry(bcid).or_default().push(slot);
            }
            BelInfo::Legacy(bel) => {
                writeln!(buf, r#"### Bel {bname}"#).unwrap();
                writeln!(buf).unwrap();
                writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
                writeln!(buf, r#"<caption>{dbname} {tname} bel {bname}</caption>"#).unwrap();
                writeln!(buf, r#"<thead>"#).unwrap();
                writeln!(
                    buf,
                    r#"<tr><th>Pin</th><th>Direction</th><th>Wires</th></tr>"#
                )
                .unwrap();
                writeln!(buf, r#"</thead>"#).unwrap();
                writeln!(buf, r#"<tbody>"#).unwrap();
                for (pname, pin) in &bel.pins {
                    let wires = pin
                        .wires
                        .iter()
                        .map(|wire| wire.to_string(intdb, tcls))
                        .join(", ");
                    let dir = match pin.dir {
                        PinDir::Input => "input",
                        PinDir::Output => "output",
                        PinDir::Inout => "in-out",
                    };
                    writeln!(
                        buf,
                        r#"<tr><td>{pname}</td><td>{dir}</td><td>{wires}</td></tr>"#
                    )
                    .unwrap();
                    for &wire in &pin.wires {
                        tcgen
                            .wires
                            .entry(wire)
                            .or_default()
                            .push((slot, pname.to_string()));
                    }
                }
                writeln!(buf, r#"</tbody>"#).unwrap();
                writeln!(buf, r#"</table></div>"#).unwrap();
                writeln!(buf).unwrap();
            }
            BelInfo::TestMux(bel) => {
                writeln!(buf, r#"### Test mux {bname}"#).unwrap();
                writeln!(buf).unwrap();
                writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
                writeln!(buf, r#"<caption>{dbname} {tname} {bname} mux</caption>"#).unwrap();
                writeln!(buf, r#"<thead>"#).unwrap();
                writeln!(
                    buf,
                    r#"<tr><th>Destination</th><th>Primary source</th><th>Test sources</th></tr>"#
                )
                .unwrap();
                writeln!(buf, r#"</thead>"#).unwrap();
                writeln!(buf, r#"<tbody>"#).unwrap();
                for (dst, tmux) in &bel.wires {
                    let dst = dst.to_string(intdb, tcls);
                    let primary_src = tmux.primary_src.to_string(intdb, tcls);
                    let test_srcs = tmux
                        .test_src
                        .keys()
                        .map(|wsrc| wsrc.to_string(intdb, tcls))
                        .join(", ");
                    writeln!(
                        buf,
                        r#"<tr><td>{dst}</td><td>{primary_src}</td><td>{test_srcs}</td></tr>"#
                    )
                    .unwrap();
                }
                writeln!(buf, r#"</tbody>"#).unwrap();
                writeln!(buf, r#"</table></div>"#).unwrap();
                writeln!(buf).unwrap();
            }
            BelInfo::GroupTestMux(bel) => {
                writeln!(buf, r#"### Test mux {bname}"#).unwrap();
                writeln!(buf).unwrap();
                writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
                writeln!(buf, r#"<caption>{dbname} {tname} {bname} mux</caption>"#).unwrap();
                writeln!(buf, r#"<thead>"#).unwrap();
                writeln!(buf, r#"<tr><th>Destination</th><th>Primary source</th>"#).unwrap();
                for i in 0..bel.groups.len() {
                    writeln!(buf, r#"<th>Test source {i}</th>"#).unwrap();
                }
                writeln!(buf, r#"</tr>"#).unwrap();
                writeln!(buf, r#"</thead>"#).unwrap();
                writeln!(buf, r#"<tbody>"#).unwrap();
                for (dst, tmux) in &bel.wires {
                    let dst = dst.to_string(intdb, tcls);
                    let primary_src = tmux.primary_src.to_string(intdb, tcls);
                    writeln!(buf, r#"<tr><td>{dst}</td><td>{primary_src}</td>"#).unwrap();
                    for &src in &tmux.test_src {
                        if let Some(src) = src {
                            let src = src.to_string(intdb, tcls);
                            writeln!(buf, r#"<td>{src}</td>"#).unwrap();
                        } else {
                            writeln!(buf, r#"<td>-</td>"#).unwrap();
                        }
                    }
                    writeln!(buf, r#"</tr>"#).unwrap();
                }
                writeln!(buf, r#"</tbody>"#).unwrap();
                writeln!(buf, r#"</table></div>"#).unwrap();
                writeln!(buf).unwrap();
            }
        }
    }

    for (bcid, bslots) in bels {
        gen_bels(&mut tcgen, &mut buf, bcid, &bslots);
    }

    if !tcgen.wires.is_empty() {
        writeln!(buf, r#"### Bel wires"#).unwrap();
        writeln!(buf).unwrap();
        writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
        writeln!(buf, r#"<caption>{dbname} {tname} bel wires</caption>"#).unwrap();
        writeln!(buf, r#"<thead>"#).unwrap();
        writeln!(buf, r#"<tr><th>Wire</th><th>Pins</th></tr>"#).unwrap();
        writeln!(buf, r#"</thead>"#).unwrap();
        writeln!(buf, r#"<tbody>"#).unwrap();
        for (wire, pins) in &tcgen.wires {
            let pins = pins
                .iter()
                .map(|&(slot, ref pin)| {
                    let bel = intdb.bel_slots.key(slot).as_str();
                    format!("{bel}.{pin}")
                })
                .join(", ");
            writeln!(
                buf,
                r#"<tr><td>{wire}</td><td>{pins}</td></tr>"#,
                wire = wire.to_string(intdb, tcls)
            )
            .unwrap();
        }
        writeln!(buf, r#"</tbody>"#).unwrap();
        writeln!(buf, r#"</table></div>"#).unwrap();
        writeln!(buf).unwrap();
    }

    if !tcls.bitrects.is_empty() {
        gen_bits(&mut tcgen, &mut buf);
    }

    ctx.items.insert(format!("tile-{dbname}-{tname}"), buf);
}

fn gen_table(ctx: &mut DocgenContext, dbname: &str, intdb: &IntDb, tid: TableId) {
    let tname = intdb.tables.key(tid);
    let table = &intdb[tid];
    let mut buf = String::new();

    writeln!(buf, r#"## Table {tname}"#).unwrap();
    writeln!(buf).unwrap();
    writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
    writeln!(buf, r#"<caption>{dbname} table {tname}</caption>"#).unwrap();
    writeln!(buf, r#"<thead>"#).unwrap();
    writeln!(buf, r#"<tr><th>Row</th>"#).unwrap();
    for fname in table.fields.keys() {
        writeln!(buf, r#"<th>{fname}</th>"#).unwrap();
    }
    writeln!(buf, r#"</tr>"#).unwrap();
    writeln!(buf, r#"</thead>"#).unwrap();
    writeln!(buf, r#"<tbody>"#).unwrap();

    for (_, rname, row) in &table.rows {
        writeln!(buf, r#"<tr><td>{rname}</td>"#).unwrap();
        for (fid, _, &typ) in &table.fields {
            if let Some(value) = row.get(fid) {
                match value {
                    TableValue::BitVec(bv) => {
                        writeln!(buf, r#"<td>0b{bv}</td>"#).unwrap();
                    }
                    TableValue::Enum(vid) => {
                        let BelAttributeType::Enum(ecid) = typ else {
                            unreachable!()
                        };
                        writeln!(
                            buf,
                            r#"<td>{val}</td>"#,
                            val = intdb.enum_classes[ecid].values[*vid]
                        )
                        .unwrap();
                    }
                }
            } else {
                writeln!(buf, r#"<td>-</td>"#).unwrap();
            }
        }
        writeln!(buf, r#"</tr>"#).unwrap();
    }

    writeln!(buf, r#"</tbody>"#).unwrap();
    writeln!(buf, r#"</table></div>"#).unwrap();
    writeln!(buf).unwrap();

    ctx.items.insert(format!("table-{dbname}-{tname}"), buf);
}

pub fn gen_intdb(ctx: &mut DocgenContext, dbname: &str, intdb: &IntDb) {
    gen_intdb_basics(ctx, dbname, intdb);
    for id in intdb.tile_classes.ids() {
        gen_tile(ctx, dbname, intdb, id);
    }
    for id in intdb.tables.ids() {
        gen_table(ctx, dbname, intdb, id);
    }
}
