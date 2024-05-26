use prjcombine_hammer::Session;
use prjcombine_int::db::WireKind;

use crate::{
    backend::{IseBackend, SimpleFeatureId},
    diff::CollectorCtx,
    fgen::{TileBits, TileFuzzKV, TileFuzzerGen, TileKV, TileWire},
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let intdb = backend.egrid.db;
    for (node_kind, name, node) in &intdb.nodes {
        if node.muxes.is_empty() {
            continue;
        }
        if backend.egrid.node_index[node_kind].is_empty() {
            continue;
        }
        // TODO
        if name.starts_with("INT.BRAM") || name.starts_with("INT.IOI.S3A") || name.starts_with("INT.DCM") || name == "INT.IOI.S3E" {
            continue;
        }
        let bits = if name.starts_with("INT") {
            TileBits::Main(1)
        } else {
            println!("UNK INT TILE: {name}");
            continue;
        };
        for (&wire_to, mux) in &node.muxes {
            let mux_name = format!("MUX.{}.{}", wire_to.0, intdb.wires.key(wire_to.1)).leak();
            for &wire_from in &mux.ins {
                let in_name = format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1)).leak();
                let mut base = vec![
                    TileKV::NodeIntDistinct(wire_to, wire_from),
                    TileKV::NodeIntDstFilter(wire_to),
                    TileKV::NodeMutexShared(wire_from),
                ];
                let fuzz = vec![
                    TileFuzzKV::NodeMutexExclusive(wire_to),
                    TileFuzzKV::Pip(TileWire::IntWire(wire_from), TileWire::IntWire(wire_to)),
                ];
                if let Some(inmux) = node.muxes.get(&wire_from) {
                    if inmux.ins.contains(&wire_to) {
                        let mut wire_help = None;
                        for &help in &inmux.ins {
                            if let Some(helpmux) = node.muxes.get(&help) {
                                if helpmux.ins.contains(&wire_from) {
                                    continue;
                                }
                            }
                            // println!("HELP {} <- {} <- {}", intdb.wires.key(wire_to.1), intdb.wires.key(wire_from.1), intdb.wires.key(help.1));
                            wire_help = Some(help);
                            break;
                        }
                        let wire_help = wire_help.unwrap();
                        base.extend([
                            TileKV::Pip(TileWire::IntWire(wire_help), TileWire::IntWire(wire_from)),
                        ]);
                    }
                }
                if intdb.wires.key(wire_from.1) == "OUT.TBUS" {
                    base.push(TileKV::RowMutex("TBUF", "INT"));
                }

                session.add_fuzzer(Box::new(TileFuzzerGen {
                    node: node_kind,
                    bits,
                    feature: SimpleFeatureId {
                        tile: name,
                        bel: "INT",
                        attr: mux_name,
                        val: in_name,
                    },
                    base,
                    fuzz,
                }));
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let egrid = ctx.edev.egrid();
    let intdb = egrid.db;
    for (node_kind, name, node) in &intdb.nodes {
        if node.muxes.is_empty() {
            continue;
        }
        if egrid.node_index[node_kind].is_empty() {
            continue;
        }
        // TODO
        if name.starts_with("INT.BRAM") || name.starts_with("INT.IOI.S3A") || name.starts_with("INT.DCM") || name == "INT.IOI.S3E" {
            continue;
        }

        // TODO: remove
        if !name.starts_with("INT") {
            continue;
        }
        for (&wire_to, mux) in &node.muxes {
            let mux_name = format!("MUX.{}.{}", wire_to.0, intdb.wires.key(wire_to.1)).leak();
            let mut inps = vec![];
            let mut got_pullup = false;
            for &wire_from in &mux.ins {
                let in_name = &*format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1)).leak();
                inps.push(in_name);
                if matches!(intdb.wires[wire_from.1], WireKind::TiePullup) {
                    got_pullup = true;
                }
            }
            ctx.collect_enum(name, "INT", mux_name, &inps[..]);
        }
    }
}
