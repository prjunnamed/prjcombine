use std::collections::{HashMap, HashSet};

use prjcombine_interconnect::{db::TileWireCoord, grid::TileCoord};
use prjcombine_re_fpga_hammer::{Diff, FuzzerProp, xlat_bit, xlat_enum, xlat_enum_default};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_re_xilinx_naming::db::{IntfWireInNaming, IntfWireOutNaming, RawTileId};
use unnamed_entity::EntityId;

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
};

use super::{
    fbuild::{FuzzBuilderBase, FuzzCtx},
    props::{
        DynProp,
        mutex::{IntMutex, NodeMutexExclusive, TileMutexExclusive},
    },
};

fn resolve_intf_test_pip<'a>(
    backend: &IseBackend<'a>,
    tcrd: TileCoord,
    wire_to: TileWireCoord,
    wire_from: TileWireCoord,
) -> Option<(&'a str, &'a str, &'a str)> {
    let nnode = &backend.ngrid.tiles[&tcrd];
    let intdb = backend.egrid.db;
    let ndb = backend.ngrid.db;
    let node_naming = &ndb.tile_class_namings[nnode.naming];
    backend
        .egrid
        .resolve_wire(backend.egrid.tile_wire(tcrd, wire_to))?;
    backend
        .egrid
        .resolve_wire(backend.egrid.tile_wire(tcrd, wire_from))?;
    if let ExpandedDevice::Virtex4(edev) = backend.edev {
        if edev.kind == prjcombine_virtex4::chip::ChipKind::Virtex5
            && ndb.tile_class_namings.key(nnode.naming) == "INTF.PPC_R"
            && intdb.wires.key(wire_from.wire).starts_with("TEST")
        {
            // ISE.
            return None;
        }
    }
    Some((
        &nnode.names[RawTileId::from_idx(0)],
        match node_naming.intf_wires_out.get(&wire_to)? {
            IntfWireOutNaming::Simple { name } => name,
            IntfWireOutNaming::Buf { name_out, .. } => name_out,
        },
        match node_naming.intf_wires_in.get(&wire_from)? {
            IntfWireInNaming::Simple { name } => name,
            IntfWireInNaming::Buf { name_in, .. } => name_in,
            IntfWireInNaming::TestBuf { name_out, .. } => name_out,
            IntfWireInNaming::Delay { name_out, .. } => name_out,
        },
    ))
}

#[derive(Clone, Debug)]
struct FuzzIntfTestPip {
    wire_to: TileWireCoord,
    wire_from: TileWireCoord,
}

impl FuzzIntfTestPip {
    pub fn new(wire_to: TileWireCoord, wire_from: TileWireCoord) -> Self {
        Self { wire_to, wire_from }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for FuzzIntfTestPip {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        if let ExpandedDevice::Virtex4(edev) = backend.edev {
            if edev.kind == prjcombine_virtex4::chip::ChipKind::Virtex4
                && backend
                    .egrid
                    .db
                    .wires
                    .key(self.wire_from.wire)
                    .starts_with("TEST")
                && tcrd.col == edev.col_cfg
            {
                // interference.
                return None;
            }
        }
        let (tile, wt, wf) = resolve_intf_test_pip(backend, tcrd, self.wire_to, self.wire_from)?;
        Some((fuzzer.fuzz(Key::Pip(tile, wf, wt), None, true), false))
    }
}

fn resolve_intf_delay<'a>(
    backend: &IseBackend<'a>,
    tcrd: TileCoord,
    wire: TileWireCoord,
) -> Option<(&'a str, &'a str, &'a str, &'a str)> {
    let nnode = &backend.ngrid.tiles[&tcrd];
    let ndb = backend.ngrid.db;
    let node_naming = &ndb.tile_class_namings[nnode.naming];
    backend
        .egrid
        .resolve_wire(backend.egrid.tile_wire(tcrd, wire))?;
    let IntfWireInNaming::Delay {
        name_out,
        name_in,
        name_delay,
    } = node_naming.intf_wires_in.get(&wire)?
    else {
        unreachable!()
    };
    Some((
        &nnode.names[RawTileId::from_idx(0)],
        name_in,
        name_delay,
        name_out,
    ))
}

#[derive(Clone, Debug)]
struct FuzzIntfDelay {
    wire: TileWireCoord,
    state: bool,
}

impl FuzzIntfDelay {
    pub fn new(wire: TileWireCoord, state: bool) -> Self {
        Self { wire, state }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for FuzzIntfDelay {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let (tile, wa, wb, wc) = resolve_intf_delay(backend, nloc, self.wire)?;
        let fuzzer = if self.state {
            fuzzer
                .fuzz(Key::Pip(tile, wa, wb), None, true)
                .fuzz(Key::Pip(tile, wb, wc), None, true)
        } else {
            fuzzer.fuzz(Key::Pip(tile, wa, wc), None, true)
        };
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.egrid.db;
    for (node_kind, tile, node) in &intdb.tile_classes {
        if node.intfs.is_empty() {
            continue;
        }
        if backend.egrid.tile_index[node_kind].is_empty() {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, tile);
        for (&wire, intf) in &node.intfs {
            match intf {
                prjcombine_interconnect::db::IntfInfo::OutputTestMux(inps) => {
                    let mux_name = if node.cells.len() == 1 {
                        format!("MUX.{}", intdb.wires.key(wire.wire))
                    } else {
                        format!("MUX.{:#}.{}", wire.cell, intdb.wires.key(wire.wire))
                    };
                    for &wire_from in inps {
                        let in_name = if node.cells.len() == 1 {
                            intdb.wires.key(wire_from.wire).to_string()
                        } else {
                            format!("{:#}.{}", wire_from.cell, intdb.wires.key(wire_from.wire))
                        };
                        ctx.build()
                            .prop(IntMutex::new("INTF".into()))
                            .test_manual("INTF", &mux_name, in_name)
                            .prop(TileMutexExclusive::new("INTF".into()))
                            .prop(NodeMutexExclusive::new(wire))
                            .prop(NodeMutexExclusive::new(wire_from))
                            .prop(FuzzIntfTestPip::new(wire, wire_from))
                            .commit();
                    }
                }
                prjcombine_interconnect::db::IntfInfo::InputDelay => {
                    assert_eq!(node.cells.len(), 1);
                    let del_name = format!("DELAY.{}", intdb.wires.key(wire.wire));
                    for val in ["0", "1"] {
                        ctx.build()
                            .prop(IntMutex::new("INTF".into()))
                            .test_manual("INTF", &del_name, val)
                            .prop(TileMutexExclusive::new("INTF".into()))
                            .prop(NodeMutexExclusive::new(wire))
                            .prop(FuzzIntfDelay::new(wire, val == "1"))
                            .commit();
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
    for (node_kind, name, node) in &intdb.tile_classes {
        if node.intfs.is_empty() {
            continue;
        }
        if egrid.tile_index[node_kind].is_empty() {
            continue;
        }
        let mut test_muxes = vec![];
        let mut test_bits: Option<HashMap<_, _>> = None;
        for (&wire, intf) in &node.intfs {
            match intf {
                prjcombine_interconnect::db::IntfInfo::OutputTestMux(inps) => {
                    let mux_name = if node.cells.len() == 1 {
                        format!("MUX.{}", intdb.wires.key(wire.wire))
                    } else {
                        format!("MUX.{:#}.{}", wire.cell, intdb.wires.key(wire.wire))
                    };
                    let mut mux_inps = vec![];
                    for &wire_from in inps {
                        let in_name = if node.cells.len() == 1 {
                            intdb.wires.key(wire_from.wire).to_string()
                        } else {
                            format!("{:#}.{}", wire_from.cell, intdb.wires.key(wire_from.wire))
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
                prjcombine_interconnect::db::IntfInfo::InputDelay => {
                    let del_name = format!("DELAY.{}", intdb.wires.key(wire.wire));
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
                prjcombine_virtex4::chip::ChipKind::Virtex4 => {
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
                prjcombine_virtex4::chip::ChipKind::Virtex6 => {
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
