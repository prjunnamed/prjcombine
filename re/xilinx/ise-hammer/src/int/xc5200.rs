use prjcombine_re_collector::{xlat_bit, xlat_enum_ocd, Diff, FeatureId, OcdMode};
use prjcombine_re_hammer::Session;
use prjcombine_interconnect::db::WireKind;
use prjcombine_types::tiledb::TileBit;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits, TileFuzzKV, TileFuzzerGen, TileKV},
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let intdb = backend.egrid.db;
    for (node_kind, name, node) in &intdb.nodes {
        if node.muxes.is_empty() {
            continue;
        }
        let bits = match &name[..] {
            "CLKL" | "CLKH" | "CLKR" => TileBits::Hclk,
            "CLKB" | "CLKV" | "CLKT" => TileBits::Spine(0, 1),
            _ => TileBits::Main(0, 1),
        };
        for (&wire_to, mux) in &node.muxes {
            let mux_name = if node.tiles.len() == 1 {
                format!("MUX.{}", intdb.wires.key(wire_to.1))
            } else {
                format!("MUX.{}.{}", wire_to.0, intdb.wires.key(wire_to.1))
            };
            for &wire_from in &mux.ins {
                let wire_from_name = intdb.wires.key(wire_from.1);
                let in_name = if node.tiles.len() == 1 {
                    wire_from_name.to_string()
                } else {
                    format!("{}.{}", wire_from.0, wire_from_name)
                };
                if (name == "IO.B" || name == "IO.T")
                    && mux_name.contains("IMUX.IO")
                    && mux_name.ends_with('O')
                    && in_name.contains("OMUX")
                {
                    continue;
                }
                let mut base = vec![
                    TileKV::NodeIntDistinct(wire_to, wire_from),
                    TileKV::NodeIntDstFilter(wire_to),
                    TileKV::NodeIntSrcFilter(wire_from),
                    TileKV::IntMutexShared("MAIN".to_string()),
                ];
                let mut fuzz = vec![
                    TileFuzzKV::NodeMutexExclusive(wire_to),
                    TileFuzzKV::NodeMutexExclusive(wire_from),
                    TileFuzzKV::IntPip(wire_from, wire_to),
                ];
                if let Some(inmux) = node.muxes.get(&wire_from) {
                    if inmux.ins.contains(&wire_to) {
                        if name.starts_with("CLK") || name.starts_with("CNR") {
                            if wire_from_name.starts_with("LONG.H") {
                                base.extend([TileKV::DriveLLH(wire_from)]);
                            } else if wire_from_name.starts_with("LONG.V") {
                                base.extend([TileKV::DriveLLV(wire_from)]);
                            } else {
                                panic!("AM HOUSECAT {name} {mux_name} {in_name}");
                            }
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
                            if let Some(wire_help) = wire_help {
                                base.extend([TileKV::IntPip(wire_help, wire_from)]);
                                fuzz.extend([
                                    TileFuzzKV::NodeMutexExclusive(wire_from),
                                    TileFuzzKV::NodeMutexExclusive(wire_help),
                                ]);
                            } else {
                                let mut wire_help_a = None;
                                let mut wire_help_b = None;
                                'help_ab: for &help_a in &inmux.ins {
                                    if help_a == wire_to {
                                        continue;
                                    }
                                    if let Some(helpmux_a) = node.muxes.get(&help_a) {
                                        for &help_b in &helpmux_a.ins {
                                            if help_b == wire_to || help_b == wire_from {
                                                continue;
                                            }
                                            if let Some(helpmux_b) = node.muxes.get(&help_b) {
                                                if helpmux_b.ins.contains(&help_a) {
                                                    continue;
                                                }
                                            }
                                            wire_help_a = Some(help_a);
                                            wire_help_b = Some(help_b);
                                            break 'help_ab;
                                        }
                                    }
                                }
                                if let (Some(wire_help_a), Some(wire_help_b)) =
                                    (wire_help_a, wire_help_b)
                                {
                                    base.extend([
                                        TileKV::IntPip(wire_help_a, wire_from),
                                        TileKV::IntPip(wire_help_b, wire_help_a),
                                    ]);
                                    fuzz.extend([
                                        TileFuzzKV::NodeMutexExclusive(wire_from),
                                        TileFuzzKV::NodeMutexExclusive(wire_help_a),
                                        TileFuzzKV::NodeMutexExclusive(wire_help_b),
                                    ]);
                                }
                            }
                        }
                    }
                }

                let mut extras = vec![];
                let mut bits = bits.clone();
                if mux_name.contains("LONG.V2") && (name == "CLKL" || name == "CLKR") {
                    bits = TileBits::Null;
                    extras.push(ExtraFeature::new(
                        ExtraFeatureKind::AllColumnIo,
                        name,
                        "INT",
                        mux_name.clone(),
                        in_name.clone(),
                    ));
                }

                session.add_fuzzer(Box::new(TileFuzzerGen {
                    node: node_kind,
                    bits,
                    feature: FeatureId {
                        tile: name.to_string(),
                        bel: "INT".to_string(),
                        attr: mux_name.to_string(),
                        val: in_name.to_string(),
                    },
                    base,
                    fuzz,
                    extras,
                }));
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let egrid = ctx.edev.egrid();
    let intdb = egrid.db;
    for (node_kind, tile, node) in &intdb.nodes {
        if node.muxes.is_empty() {
            continue;
        }
        if egrid.node_index[node_kind].is_empty() {
            continue;
        }

        for (&wire_to, mux) in &node.muxes {
            if intdb.wires[wire_to.1] != WireKind::MuxOut {
                let out_name = if node.tiles.len() == 1 {
                    intdb.wires.key(wire_to.1).to_string()
                } else {
                    format!("{}.{}", wire_to.0, intdb.wires.key(wire_to.1))
                };
                for &wire_from in &mux.ins {
                    let in_name = if node.tiles.len() == 1 {
                        intdb.wires.key(wire_from.1).to_string()
                    } else {
                        format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1))
                    };
                    let mut diff =
                        ctx.state
                            .get_diff(tile, "INT", format!("MUX.{out_name}"), &in_name);
                    // HORSEFUCKERS PISS SHIT FUCK
                    match (&tile[..], &out_name[..], &in_name[..]) {
                        ("CNR.BR", "LONG.V0", "OUT.STARTUP.DONEIN") => {
                            assert_eq!(diff.bits.len(), 2);
                            assert_eq!(diff.bits.remove(&TileBit::new(0, 6, 20)), Some(false));
                        }
                        ("CNR.BR", "LONG.V1", "OUT.STARTUP.DONEIN") => {
                            assert_eq!(diff.bits.len(), 0);
                            diff.bits.insert(TileBit::new(0, 6, 20), false);
                        }
                        _ => (),
                    }
                    let item = xlat_bit(diff);
                    let mut is_bidi = false;
                    if let Some(omux) = node.muxes.get(&wire_from) {
                        if omux.ins.contains(&wire_to) {
                            is_bidi = true;
                        }
                    }
                    let name = if !is_bidi {
                        format!("PASS.{out_name}.{in_name}")
                    } else if wire_from < wire_to {
                        format!("BIPASS.{in_name}.{out_name}")
                    } else {
                        format!("BIPASS.{out_name}.{in_name}")
                    };
                    ctx.tiledb.insert(tile, "INT", name, item);
                }
            } else {
                let out_name = if node.tiles.len() == 1 {
                    intdb.wires.key(wire_to.1).to_string()
                } else {
                    format!("{}.{}", wire_to.0, intdb.wires.key(wire_to.1))
                };
                let mux_name = format!("MUX.{out_name}");

                let mut inps = vec![];
                let mut got_empty = false;
                for &wire_from in &mux.ins {
                    let in_name = if node.tiles.len() == 1 {
                        intdb.wires.key(wire_from.1).to_string()
                    } else {
                        format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1))
                    };
                    if (tile == "IO.B" || tile == "IO.T")
                        && mux_name.contains("IMUX.IO")
                        && mux_name.ends_with('O')
                        && in_name.contains("OMUX")
                    {
                        continue;
                    }
                    let diff = ctx.state.get_diff(tile, "INT", &mux_name, &in_name);
                    if diff.bits.is_empty() {
                        got_empty = true;
                    }
                    inps.push((in_name.to_string(), diff));
                }
                for (rtile, rwire, rbel, rattr) in [
                    ("CNR.BR", "IMUX.STARTUP.GTS", "STARTUP", "ENABLE.GTS"),
                    ("CNR.BR", "IMUX.STARTUP.GRST", "STARTUP", "ENABLE.GR"),
                ] {
                    if tile == rtile && out_name == rwire {
                        let mut common = inps[0].1.clone();
                        for (_, diff) in &inps {
                            common.bits.retain(|bit, _| diff.bits.contains_key(bit));
                        }
                        assert_eq!(common.bits.len(), 1);
                        for (_, diff) in &mut inps {
                            *diff = diff.combine(&!&common);
                            if diff.bits.is_empty() {
                                got_empty = true;
                            }
                        }
                        assert!(got_empty);
                        ctx.tiledb.insert(tile, rbel, rattr, xlat_bit(common));
                    }
                }
                if !got_empty {
                    inps.push(("NONE".to_string(), Diff::default()));
                }
                let item = xlat_enum_ocd(inps, OcdMode::Mux);
                if item.bits.is_empty() {
                    println!("UMMM MUX {tile} {mux_name} is empty");
                }
                ctx.tiledb.insert(tile, "INT", mux_name, item);
            }
        }
    }
}
