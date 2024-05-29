use prjcombine_hammer::Session;
use prjcombine_xilinx_geom::ExpandedDevice;

use crate::{
    backend::{IseBackend, SimpleFeatureId},
    diff::{xlat_enum_inner, CollectorCtx, Diff, OcdMode},
    fgen::{TileBits, TileFuzzKV, TileFuzzerGen, TileKV},
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    if matches!(backend.edev, ExpandedDevice::Virtex(_)) {
        return;
    }
    let intdb = backend.egrid.db;
    for (node_kind, name, node) in &intdb.nodes {
        if node.muxes.is_empty() {
            continue;
        }
        if backend.egrid.node_index[node_kind].is_empty() {
            continue;
        }
        let bits = if name.starts_with("INT") || name == "PPC.E" || name == "PPC.W" {
            TileBits::Main(1)
        } else if name.starts_with("CLKB") || name.starts_with("CLKT") || name.starts_with("REG_") {
            TileBits::BTSpine
        } else if matches!(&name[..], "TERM.S" | "TERM.N") {
            TileBits::BTTerm
        } else if matches!(&name[..], "TERM.E" | "TERM.W") {
            TileBits::LRTerm
        } else if name == "PPC.N" {
            TileBits::MainUp
        } else if name == "PPC.S" {
            TileBits::MainDown
        } else if name.starts_with("LLV") {
            TileBits::LLV
        } else if name.starts_with("LLH") {
            TileBits::Spine
        } else {
            panic!("UNK INT TILE: {name}");
        };
        for (&wire_to, mux) in &node.muxes {
            let mux_name = if node.tiles.len() == 1 {
                &*format!("MUX.{}", intdb.wires.key(wire_to.1)).leak()
            } else {
                &*format!("MUX.{}.{}", wire_to.0, intdb.wires.key(wire_to.1)).leak()
            };
            for &wire_from in &mux.ins {
                let in_name = if node.tiles.len() == 1 {
                    &intdb.wires.key(wire_from.1)[..]
                } else {
                    format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1)).leak()
                };
                let mut base = vec![
                    TileKV::NodeIntDistinct(wire_to, wire_from),
                    TileKV::NodeIntDstFilter(wire_to),
                    TileKV::NodeIntSrcFilter(wire_from),
                    TileKV::NodeMutexShared(wire_from),
                ];
                let fuzz = vec![
                    TileFuzzKV::NodeMutexExclusive(wire_to),
                    TileFuzzKV::IntPip(wire_from, wire_to),
                ];
                if let Some(inmux) = node.muxes.get(&wire_from) {
                    if inmux.ins.contains(&wire_to) {
                        if name.starts_with("LLH") {
                            base.extend([TileKV::DriveLLH(wire_from)]);
                        } else if name.starts_with("LLV") {
                            base.extend([TileKV::DriveLLV(wire_from)]);
                        } else {
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
                            base.extend([TileKV::IntPip(wire_help, wire_from)]);
                        }
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
    if matches!(ctx.edev, ExpandedDevice::Virtex(_)) {
        return;
    }
    let egrid = ctx.edev.egrid();
    let intdb = egrid.db;
    for (node_kind, name, node) in &intdb.nodes {
        if node.muxes.is_empty() {
            continue;
        }
        if egrid.node_index[node_kind].is_empty() {
            continue;
        }

        for (&wire_to, mux) in &node.muxes {
            let mux_name = if node.tiles.len() == 1 {
                &*format!("MUX.{}", intdb.wires.key(wire_to.1)).leak()
            } else {
                &*format!("MUX.{}.{}", wire_to.0, intdb.wires.key(wire_to.1)).leak()
            };
            let mut inps = vec![];
            let mut got_empty = false;
            for &wire_from in &mux.ins {
                let in_name = if node.tiles.len() == 1 {
                    &intdb.wires.key(wire_from.1)[..]
                } else {
                    format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1)).leak()
                };
                let diff = ctx.state.get_diff(name, "INT", mux_name, in_name);
                if let ExpandedDevice::Virtex2(ref edev) = ctx.edev {
                    if edev.grid.kind == prjcombine_virtex2::grid::GridKind::Spartan3ADsp
                        && name == "INT.IOI.S3A.LR"
                        && mux_name == "MUX.IMUX.DATA3"
                        && in_name == "OMUX10.N"
                    {
                        // ISE is bad and should feel bad.
                        continue;
                    }
                }
                if diff.bits.is_empty() {
                    if intdb.wires.key(wire_to.1).starts_with("IMUX")
                        && !intdb.wires[wire_from.1].is_tie()
                    {
                        // suppress message on known offenders.
                        if name == "INT.BRAM.S3A.03"
                            && (mux_name.starts_with("MUX.IMUX.CLK")
                                || mux_name.starts_with("MUX.IMUX.CE"))
                        {
                            // these muxes don't actually exist.
                            continue;
                        }
                        if name.starts_with("INT.IOI.S3")
                            && mux_name.starts_with("MUX.IMUX.DATA")
                            && (in_name.starts_with("OUT.FAN")
                                || in_name.starts_with("IMUX.FAN")
                                || in_name.starts_with("OMUX"))
                        {
                            // ISE is kind of bad. fill these from INT.CLB and verify later?
                            continue;
                        }
                        println!("UMMMMM PIP {name} {mux_name} {in_name} is empty");
                        continue;
                    }
                    got_empty = true;
                }
                inps.push((in_name.to_string(), diff));
            }
            if !got_empty {
                inps.push(("NONE".to_string(), Diff::default()));
            }
            let ti = xlat_enum_inner(inps, OcdMode::Mux);
            if ti.bits.is_empty()
                && !(name == "INT.GT.CLKPAD"
                    && matches!(
                        mux_name,
                        "MUX.IMUX.CE0" | "MUX.IMUX.CE1" | "MUX.IMUX.TS0" | "MUX.IMUX.TS1"
                    ))
                && !(name == "INT.BRAM.S3A.03"
                    && (mux_name.starts_with("MUX.IMUX.CLK")
                        || mux_name.starts_with("MUX.IMUX.CE")))
            {
                println!("UMMM MUX {name} {mux_name} is empty");
            }
            ctx.tiledb.insert(name, "INT", mux_name, ti);
        }
    }
}
