use std::{collections::BTreeMap, fmt::Write};

use itertools::Itertools;
use prjcombine_interconnect::db::{ConnectorWire, IntDb, IntfInfo, PinDir, TileClassId, WireKind};

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
                && kind != WireKind::PipBranch(csid)
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
    if !tcls.muxes.is_empty() {
        writeln!(buf, r#"### Muxes"#).unwrap();
        writeln!(buf).unwrap();
        writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
        writeln!(buf, r#"<caption>{dbname} {tname} muxes</caption>"#).unwrap();
        writeln!(buf, r#"<thead>"#).unwrap();
        writeln!(buf, r#"<tr><th>Destination</th><th>Sources</th></tr>"#).unwrap();
        writeln!(buf, r#"</thead>"#).unwrap();
        writeln!(buf, r#"<tbody>"#).unwrap();
        for (wdst, mux) in &tcls.muxes {
            let dst = if single_cell {
                intdb.wires.key(wdst.wire).to_string()
            } else {
                format!("{}:{}", wdst.cell, intdb.wires.key(wdst.wire))
            };
            let src = mux
                .ins
                .iter()
                .map(|wsrc| {
                    if single_cell {
                        intdb.wires.key(wsrc.wire).to_string()
                    } else {
                        format!("{}:{}", wsrc.cell, intdb.wires.key(wsrc.wire))
                    }
                })
                .join(", ");
            writeln!(buf, r#"<tr><td>{dst}</td><td>{src}</td></tr>"#).unwrap();
        }
        writeln!(buf, r#"</tbody>"#).unwrap();
        writeln!(buf, r#"</table></div>"#).unwrap();
        writeln!(buf).unwrap();
    }

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
