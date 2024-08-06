use std::collections::{HashMap, HashSet};

use prjcombine_hammer::Session;
use prjcombine_xilinx_geom::ExpandedDevice;

use crate::{
    backend::{FeatureId, IseBackend},
    diff::{xlat_bit, xlat_enum, xlat_enum_default, CollectorCtx, Diff},
    fgen::{TileBits, TileFuzzKV, TileFuzzerGen, TileKV},
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let intdb = backend.egrid.db;
    for (node_kind, name, node) in &intdb.nodes {
        if node.intfs.is_empty() {
            continue;
        }
        if backend.egrid.node_index[node_kind].is_empty() {
            continue;
        }
        let bits = TileBits::Main(node.tiles.len());
        for (&wire, intf) in &node.intfs {
            match intf {
                prjcombine_int::db::IntfInfo::OutputTestMux(inps) => {
                    let mux_name = if node.tiles.len() == 1 {
                        format!("MUX.{}", intdb.wires.key(wire.1))
                    } else {
                        format!("MUX.{}.{}", wire.0, intdb.wires.key(wire.1))
                    };
                    for &wire_from in inps {
                        let in_name = if node.tiles.len() == 1 {
                            intdb.wires.key(wire_from.1).to_string()
                        } else {
                            format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1))
                        };
                        session.add_fuzzer(Box::new(TileFuzzerGen {
                            node: node_kind,
                            bits: bits.clone(),
                            feature: FeatureId {
                                tile: name.to_string(),
                                bel: "INTF".into(),
                                attr: mux_name.clone(),
                                val: in_name,
                            },
                            base: vec![TileKV::IntMutexShared("INTF".into())],
                            fuzz: vec![
                                TileFuzzKV::TileMutexExclusive("INTF".into()),
                                TileFuzzKV::NodeMutexExclusive(wire),
                                TileFuzzKV::NodeMutexExclusive(wire_from),
                                TileFuzzKV::IntfTestPip(wire_from, wire),
                            ],
                            extras: vec![],
                        }));
                    }
                }
                prjcombine_int::db::IntfInfo::InputDelay => {
                    assert_eq!(node.tiles.len(), 1);
                    let del_name = format!("DELAY.{}", intdb.wires.key(wire.1));
                    for val in ["0", "1"] {
                        session.add_fuzzer(Box::new(TileFuzzerGen {
                            node: node_kind,
                            bits: bits.clone(),
                            feature: FeatureId {
                                tile: name.to_string(),
                                bel: "INTF".into(),
                                attr: del_name.clone(),
                                val: val.to_string(),
                            },
                            base: vec![TileKV::IntMutexShared("INTF".into())],
                            fuzz: vec![
                                TileFuzzKV::TileMutexExclusive("INTF".into()),
                                TileFuzzKV::NodeMutexExclusive(wire),
                                TileFuzzKV::IntfDelay(wire, val == "1"),
                            ],
                            extras: vec![],
                        }));
                    }
                }
                _ => unreachable!(),
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let egrid = ctx.edev.egrid();
    let intdb = egrid.db;
    for (node_kind, name, node) in &intdb.nodes {
        if node.intfs.is_empty() {
            continue;
        }
        if egrid.node_index[node_kind].is_empty() {
            continue;
        }
        let mut test_muxes = vec![];
        let mut test_bits: Option<HashMap<_, _>> = None;
        for (&wire, intf) in &node.intfs {
            match intf {
                prjcombine_int::db::IntfInfo::OutputTestMux(inps) => {
                    let mux_name = if node.tiles.len() == 1 {
                        format!("MUX.{}", intdb.wires.key(wire.1))
                    } else {
                        format!("MUX.{}.{}", wire.0, intdb.wires.key(wire.1))
                    };
                    let mut mux_inps = vec![];
                    for &wire_from in inps {
                        let in_name = if node.tiles.len() == 1 {
                            intdb.wires.key(wire_from.1).to_string()
                        } else {
                            format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1))
                        };
                        let diff = ctx.state.get_diff(name, "INTF", &mux_name, &in_name);

                        match test_bits {
                            Some(ref mut bits) => bits.retain(|bit, _| diff.bits.contains_key(bit)),
                            None => {
                                test_bits = Some(diff.bits.iter().map(|(&a, &b)| (a, b)).collect())
                            }
                        }

                        mux_inps.push((in_name, diff));
                    }
                    test_muxes.push((mux_name, mux_inps));
                }
                prjcombine_int::db::IntfInfo::InputDelay => {
                    let del_name = format!("DELAY.{}", intdb.wires.key(wire.1));
                    ctx.collect_enum_bool(name, "INTF", &del_name, "0", "1");
                }
                _ => unreachable!(),
            }
        }
        let Some(test_bits) = test_bits else { continue };
        if test_bits.is_empty() {
            let mut test_diffs = vec![];
            for (mux_name, mux_inps) in test_muxes {
                let mut mux_groups = HashSet::new();
                for (in_name, mut diff) in mux_inps {
                    if in_name.contains("IMUX.SR") || in_name.contains("IMUX.CE") {
                        let mut item = ctx
                            .tiledb
                            .item("INT.BRAM.S3ADSP", "INT", &format!("INV.{}", &in_name[2..]))
                            .clone();
                        assert_eq!(item.bits.len(), 1);
                        item.bits[0].tile = in_name[..1].parse().unwrap();
                        diff.discard_bits(&item);
                    }
                    assert_eq!(diff.bits.len(), 1);
                    let idx = test_diffs
                        .iter()
                        .position(|x| *x == diff)
                        .unwrap_or_else(|| {
                            let res = test_diffs.len();
                            test_diffs.push(diff);
                            res
                        });
                    ctx.tiledb.insert_misc_data(
                        format!("{name}:INTF_GROUP:{mux_name}:{in_name}"),
                        format!("{idx}"),
                    );
                    assert!(mux_groups.insert(idx));
                }
            }
            ctx.tiledb.insert(
                name,
                "INTF",
                "TEST_ENABLE",
                xlat_enum_default(
                    test_diffs
                        .into_iter()
                        .enumerate()
                        .map(|(i, diff)| (format!("{i}"), diff))
                        .collect(),
                    "NONE",
                ),
            );
            continue;
        }
        assert_eq!(test_bits.len(), 1);
        let test_diff = Diff { bits: test_bits };
        for (_, mux_inps) in &mut test_muxes {
            for (_, diff) in mux_inps {
                *diff = diff.combine(&!&test_diff);
            }
        }
        ctx.tiledb
            .insert(name, "INTF", "TEST_ENABLE", xlat_bit(test_diff));
        if let ExpandedDevice::Virtex4(edev) = ctx.edev {
            match edev.kind {
                prjcombine_virtex4::grid::GridKind::Virtex4 => {
                    for (_, mux_inps) in &mut test_muxes {
                        for (in_name, diff) in mux_inps {
                            if in_name.starts_with("IMUX.CLK")
                                || in_name.starts_with("IMUX.SR")
                                || in_name.starts_with("IMUX.CE")
                            {
                                diff.discard_bits(ctx.tiledb.item(
                                    "INT",
                                    "INT",
                                    &format!("INV.{in_name}"),
                                ));
                            }
                        }
                    }
                }
                prjcombine_virtex4::grid::GridKind::Virtex6 => {
                    let mut new_test_muxes = vec![];
                    let mut known_bits = HashSet::new();
                    for (mux_name, mux_inps) in &test_muxes {
                        let (_, _, common) =
                            Diff::split(mux_inps[0].1.clone(), mux_inps[1].1.clone());
                        let mut new_mux_inps = vec![];
                        for (in_name, diff) in mux_inps {
                            let (diff, empty, check_common) =
                                Diff::split(diff.clone(), common.clone());
                            assert_eq!(check_common, common);
                            empty.assert_empty();
                            for &bit in diff.bits.keys() {
                                known_bits.insert(bit);
                            }
                            new_mux_inps.push((in_name.clone(), diff));
                        }
                        new_test_muxes.push((mux_name.clone(), new_mux_inps));
                    }
                    for (_, mux_inps) in test_muxes {
                        for (_, diff) in mux_inps {
                            for bit in diff.bits.keys() {
                                assert!(known_bits.contains(bit));
                            }
                        }
                    }
                    test_muxes = new_test_muxes;
                }
                _ => (),
            }
        }
        for (mux_name, mut mux_inps) in test_muxes {
            if mux_inps.len() == 1 {
                mux_inps.pop().unwrap().1.assert_empty();
            } else {
                let has_empty = mux_inps.iter().any(|(_, diff)| diff.bits.is_empty());
                let diffs = mux_inps
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect();

                let item = if has_empty {
                    xlat_enum(diffs)
                } else {
                    xlat_enum_default(diffs, "NONE")
                };
                ctx.tiledb.insert(name, "INTF", mux_name, item);
            }
        }
    }
}
