use prjcombine_re_collector::{Diff, FeatureId, OcdMode, xlat_enum_ocd};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{TileBits, TileFuzzKV, TileFuzzerGen, TileKV},
};

pub mod virtex;
pub mod xc4000;
pub mod xc5200;

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    match backend.edev {
        ExpandedDevice::Xc2000(edev) => {
            if edev.chip.kind.is_xc4000() {
                xc4000::add_fuzzers(session, backend);
            } else {
                xc5200::add_fuzzers(session, backend);
            }
            return;
        }
        ExpandedDevice::Virtex(_) => {
            virtex::add_fuzzers(session, backend);
            return;
        }
        _ => (),
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
            TileBits::Main(0, 1)
        } else if name.starts_with("CLKB") || name.starts_with("CLKT") || name.starts_with("REG_") {
            TileBits::SpineEnd
        } else if matches!(&name[..], "TERM.S" | "TERM.N") {
            TileBits::BTTerm
        } else if matches!(&name[..], "TERM.E" | "TERM.W") {
            TileBits::LRTerm
        } else if name == "PPC.N" {
            TileBits::MainUp
        } else if name == "PPC.S" {
            TileBits::MainDown
        } else if name.starts_with("LLV") {
            TileBits::Llv
        } else if name.starts_with("LLH") {
            TileBits::Spine(0, 1)
        } else {
            panic!("UNK INT TILE: {name}");
        };
        for (&wire_to, mux) in &node.muxes {
            let mux_name = if node.tiles.len() == 1 {
                format!("MUX.{}", intdb.wires.key(wire_to.1))
            } else {
                format!("MUX.{}.{}", wire_to.0, intdb.wires.key(wire_to.1))
            };
            for &wire_from in &mux.ins {
                let in_name = if node.tiles.len() == 1 {
                    intdb.wires.key(wire_from.1).to_string()
                } else {
                    format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1))
                };
                let mut base = vec![
                    TileKV::NodeIntDistinct(wire_to, wire_from),
                    TileKV::NodeIntDstFilter(wire_to),
                    TileKV::NodeIntSrcFilter(wire_from),
                    TileKV::NodeMutexShared(wire_from),
                    TileKV::IntMutexShared("MAIN".to_string()),
                    TileKV::GlobalMutexNone("MISR_CLOCK".to_string()),
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
                if matches!(backend.edev, ExpandedDevice::Virtex2(_)) {
                    base.push(TileKV::NoGlobalOpt("TESTLL".to_string()));
                }
                if intdb.wires.key(wire_from.1) == "OUT.TBUS" {
                    base.push(TileKV::RowMutex("TBUF".to_string(), "INT".to_string()));
                }

                session.add_fuzzer(Box::new(TileFuzzerGen {
                    node: node_kind,
                    bits: bits.clone(),
                    feature: FeatureId {
                        tile: name.to_string(),
                        bel: "INT".to_string(),
                        attr: mux_name.to_string(),
                        val: in_name.to_string(),
                    },
                    base,
                    fuzz,
                    extras: vec![],
                }));
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    match ctx.edev {
        ExpandedDevice::Xc2000(edev) => {
            if edev.chip.kind.is_xc4000() {
                xc4000::collect_fuzzers(ctx);
            } else {
                xc5200::collect_fuzzers(ctx);
            }
            return;
        }
        ExpandedDevice::Virtex(_) => {
            virtex::collect_fuzzers(ctx);
            return;
        }
        _ => (),
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
                format!("MUX.{}", intdb.wires.key(wire_to.1))
            } else {
                format!("MUX.{}.{}", wire_to.0, intdb.wires.key(wire_to.1))
            };
            let mut inps = vec![];
            let mut got_empty = false;
            for &wire_from in &mux.ins {
                let in_name = if node.tiles.len() == 1 {
                    intdb.wires.key(wire_from.1).to_string()
                } else {
                    format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1))
                };
                let diff = ctx.state.get_diff(name, "INT", &mux_name, &in_name);
                if let ExpandedDevice::Virtex2(edev) = ctx.edev {
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
            let ti = xlat_enum_ocd(inps, OcdMode::Mux);
            if ti.bits.is_empty()
                && !(name == "INT.GT.CLKPAD"
                    && matches!(
                        &mux_name[..],
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
