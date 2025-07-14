use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
};

use itertools::Itertools;
use prjcombine_interconnect::db::{
    BelInfo, ConnectorWire, IntDb, IntfInfo, PinDir, SwitchBoxItem, TileClassId, TileWireCoord,
    WireKind,
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
        r#"<tr><th>Slot</th><th>Tile slot</th><th>Tiles</th></tr>"#
    )
    .unwrap();
    writeln!(buf, r#"</thead>"#).unwrap();
    writeln!(buf, r#"<tbody>"#).unwrap();
    for (id, name, bslot) in &intdb.bel_slots {
        let tiles = intdb
            .tile_classes
            .iter()
            .filter(|&(_, _, tcls)| tcls.bels.contains_id(id))
            .map(|(_, name, _)| name.as_str())
            .join(", ");
        let tslot = intdb.tile_slots[bslot.tile_slot].as_str();
        writeln!(
            buf,
            r#"<tr><td>{name}</td><td>{tslot}</td><td>{tiles}</td></tr>"#
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

fn gen_tile(ctx: &mut DocgenContext, dbname: &str, intdb: &IntDb, tcid: TileClassId) {
    let tname = intdb.tile_classes.key(tcid);
    let tcls = &intdb.tile_classes[tcid];
    let mut buf = String::new();

    writeln!(buf, r#"## Tile {tname}"#).unwrap();
    writeln!(buf).unwrap();
    writeln!(buf, r#"Cells: {}"#, tcls.cells.len()).unwrap();
    writeln!(buf).unwrap();

    let single_cell = tcls.cells.len() == 1;

    if !tcls.intfs.is_empty() {
        writeln!(buf, r#"### Intf"#).unwrap();
        writeln!(buf).unwrap();
        writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
        writeln!(buf, r#"<caption>{dbname} {tname} intfs</caption>"#).unwrap();
        writeln!(buf, r#"<thead>"#).unwrap();
        writeln!(buf, r#"<tr><th>Wire</th><th>Interface</th></tr>"#).unwrap();
        writeln!(buf, r#"</thead>"#).unwrap();
        writeln!(buf, r#"<tbody>"#).unwrap();
        for (wire, intf) in &tcls.intfs {
            let wire = if single_cell {
                intdb.wires.key(wire.wire).to_string()
            } else {
                format!("{}:{}", wire.cell, intdb.wires.key(wire.wire))
            };
            let intf = match intf {
                IntfInfo::OutputTestMux(srcs) => {
                    let srcs = srcs
                        .iter()
                        .map(|wsrc| {
                            if single_cell {
                                intdb.wires.key(wsrc.wire).to_string()
                            } else {
                                format!("{}:{}", wsrc.cell, intdb.wires.key(wsrc.wire))
                            }
                        })
                        .join(", ");
                    format!("TEST_MUX {srcs}")
                }
                IntfInfo::OutputTestMuxPass(srcs, base) => {
                    let srcs = srcs
                        .iter()
                        .map(|wsrc| {
                            if single_cell {
                                intdb.wires.key(wsrc.wire).to_string()
                            } else {
                                format!("{}:{}", wsrc.cell, intdb.wires.key(wsrc.wire))
                            }
                        })
                        .join(", ");
                    let base = if single_cell {
                        intdb.wires.key(base.wire).to_string()
                    } else {
                        format!("{}:{}", base.cell, intdb.wires.key(base.wire))
                    };
                    format!("TEST_MUX BASE {base} TEST {srcs}")
                }
                IntfInfo::InputDelay => "DELAY".to_string(),
            };
            writeln!(buf, r#"<tr><td>{wire}</td><td>{intf}</td></tr>"#).unwrap();
        }
        writeln!(buf, r#"</tbody>"#).unwrap();
        writeln!(buf, r#"</table></div>"#).unwrap();
        writeln!(buf).unwrap();
    }

    let mut wmap: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for (slot, bel) in &tcls.bels {
        let bname = intdb.bel_slots.key(slot).as_str();
        writeln!(buf, r#"### Bel {bname}"#).unwrap();
        writeln!(buf).unwrap();
        match bel {
            BelInfo::SwitchBox(sb) => {
                #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
                enum PipKind {
                    Mux(bool),
                    PermaBuf(bool),
                    ProgBuf(bool),
                    Pass,
                    BiPass,
                    ProgInv,
                    ProgDelay(bool, u8),
                }
                let mut pips: BTreeMap<TileWireCoord, BTreeSet<(PipKind, TileWireCoord)>> =
                    BTreeMap::new();
                for item in &sb.items {
                    match item {
                        SwitchBoxItem::Mux(mux) => {
                            for &src in &mux.src {
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
                                PipKind::ProgDelay(delay.src.inv, delay.num_steps),
                                delay.src.tw,
                            ));
                        }
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
            BelInfo::Bel(bel) => {
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
                        .map(|wire| {
                            if single_cell {
                                intdb.wires.key(wire.wire).to_string()
                            } else {
                                format!("{}:{}", wire.cell, intdb.wires.key(wire.wire))
                            }
                        })
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
                        wmap.entry(wire).or_default().push((slot, pname));
                    }
                }
                writeln!(buf, r#"</tbody>"#).unwrap();
                writeln!(buf, r#"</table></div>"#).unwrap();
                writeln!(buf).unwrap();
            }
        }
    }
    if !wmap.is_empty() {
        writeln!(buf, r#"### Bel wires"#).unwrap();
        writeln!(buf).unwrap();
        writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
        writeln!(buf, r#"<caption>{dbname} {tname} bel wires</caption>"#).unwrap();
        writeln!(buf, r#"<thead>"#).unwrap();
        writeln!(buf, r#"<tr><th>Wire</th><th>Pins</th></tr>"#).unwrap();
        writeln!(buf, r#"</thead>"#).unwrap();
        writeln!(buf, r#"<tbody>"#).unwrap();
        for (wire, pins) in wmap {
            let wire = if single_cell {
                intdb.wires.key(wire.wire).to_string()
            } else {
                format!("{}:{}", wire.cell, intdb.wires.key(wire.wire))
            };

            let pins = pins
                .into_iter()
                .map(|(slot, pin)| {
                    let bel = intdb.bel_slots.key(slot).as_str();
                    format!("{bel}.{pin}")
                })
                .join(", ");
            writeln!(buf, r#"<tr><td>{wire}</td><td>{pins}</td></tr>"#).unwrap();
        }
        writeln!(buf, r#"</tbody>"#).unwrap();
        writeln!(buf, r#"</table></div>"#).unwrap();
        writeln!(buf).unwrap();
    }

    ctx.items.insert(format!("tile-{dbname}-{tname}"), buf);
}

pub fn gen_intdb(ctx: &mut DocgenContext, dbname: &str, intdb: &IntDb) {
    gen_intdb_basics(ctx, dbname, intdb);
    for id in intdb.tile_classes.ids() {
        gen_tile(ctx, dbname, intdb, id);
    }
}
