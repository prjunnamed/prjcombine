use std::collections::{HashMap, HashSet};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, TileWireCoord},
    grid::TileCoord,
};
use prjcombine_re_fpga_hammer::{Diff, FuzzerProp, xlat_bit, xlat_enum, xlat_enum_default};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_re_xilinx_naming::db::{IntfWireInNaming, RawTileId};
use prjcombine_types::bsdata::BitRectId;

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
};

use super::{
    fbuild::{FuzzBuilderBase, FuzzCtx},
    props::{
        DynProp,
        mutex::{IntMutex, TileMutexExclusive, WireMutexExclusive},
    },
};

fn resolve_intf_test_pip<'a>(
    backend: &IseBackend<'a>,
    tcrd: TileCoord,
    wire_to: TileWireCoord,
    wire_from: TileWireCoord,
) -> Option<(&'a str, &'a str, &'a str)> {
    let ntile = &backend.ngrid.tiles[&tcrd];
    let intdb = backend.edev.db;
    let ndb = backend.ngrid.db;
    let tile_naming = &ndb.tile_class_namings[ntile.naming];
    backend
        .edev
        .resolve_wire(backend.edev.tile_wire(tcrd, wire_to))?;
    backend
        .edev
        .resolve_wire(backend.edev.tile_wire(tcrd, wire_from))?;
    if let ExpandedDevice::Virtex4(edev) = backend.edev
        && edev.kind == prjcombine_virtex4::chip::ChipKind::Virtex5
        && ndb.tile_class_namings.key(ntile.naming) == "INTF.PPC_R"
        && intdb.wires.key(wire_from.wire).starts_with("TEST")
    {
        // ISE.
        return None;
    }
    Some((
        &ntile.names[RawTileId::from_idx(0)],
        tile_naming.wires.get(&wire_to)?,
        match tile_naming.intf_wires_in.get(&wire_from)? {
            IntfWireInNaming::Simple { name } => name,
            IntfWireInNaming::Buf { name_in, .. } => name_in,
            IntfWireInNaming::TestBuf { name_out, .. } => name_out,
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
        if let ExpandedDevice::Virtex4(edev) = backend.edev
            && edev.kind == prjcombine_virtex4::chip::ChipKind::Virtex4
            && backend
                .edev
                .db
                .wires
                .key(self.wire_from.wire)
                .starts_with("TEST")
            && tcrd.col == edev.col_cfg
        {
            // interference.
            return None;
        }
        let (tile, wt, wf) = resolve_intf_test_pip(backend, tcrd, self.wire_to, self.wire_from)?;
        Some((fuzzer.fuzz(Key::Pip(tile, wf, wt), None, true), false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.edev.db;
    for (_tcid, tcname, tcls) in &intdb.tile_classes {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcname) else {
            continue;
        };
        for (slot, bel) in &tcls.bels {
            match bel {
                BelInfo::TestMux(bel) => {
                    let mut bctx = ctx.bel(slot);
                    for (&dst, tmux) in &bel.wires {
                        let mux_name = if tcls.cells.len() == 1 {
                            format!("MUX.{}", intdb.wires.key(dst.wire))
                        } else {
                            format!("MUX.{:#}.{}", dst.cell, intdb.wires.key(dst.wire))
                        };
                        for &src in tmux.test_src.keys() {
                            let in_name = if tcls.cells.len() == 1 {
                                intdb.wires.key(src.wire).to_string()
                            } else {
                                format!("{:#}.{}", src.cell, intdb.wires.key(src.wire))
                            };
                            bctx.build()
                                .prop(IntMutex::new("INTF".into()))
                                .test_manual(&mux_name, in_name)
                                .prop(TileMutexExclusive::new("INTF".into()))
                                .prop(WireMutexExclusive::new(dst))
                                .prop(WireMutexExclusive::new(src.tw))
                                .prop(FuzzIntfTestPip::new(dst, src.tw))
                                .commit();
                        }
                    }
                }
                BelInfo::GroupTestMux(bel) => {
                    let mut bctx = ctx.bel(slot);
                    for (&dst, tmux) in &bel.wires {
                        let mux_name = if tcls.cells.len() == 1 {
                            format!("MUX.{}", intdb.wires.key(dst.wire))
                        } else {
                            format!("MUX.{:#}.{}", dst.cell, intdb.wires.key(dst.wire))
                        };
                        for &src in &tmux.test_src {
                            let Some(src) = src else {
                                continue;
                            };
                            let in_name = if tcls.cells.len() == 1 {
                                intdb.wires.key(src.wire).to_string()
                            } else {
                                format!("{:#}.{}", src.cell, intdb.wires.key(src.wire))
                            };
                            bctx.build()
                                .prop(IntMutex::new("INTF".into()))
                                .test_manual(&mux_name, in_name)
                                .prop(TileMutexExclusive::new("INTF".into()))
                                .prop(WireMutexExclusive::new(dst))
                                .prop(WireMutexExclusive::new(src.tw))
                                .prop(FuzzIntfTestPip::new(dst, src.tw))
                                .commit();
                        }
                    }
                }
                _ => (),
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let intdb = ctx.edev.db;
    for (_tcid, tcname, tcls) in &intdb.tile_classes {
        if !ctx.has_tile(tcname) {
            continue;
        }
        for (slot, bel) in &tcls.bels {
            match bel {
                BelInfo::TestMux(bel) => {
                    let bname = ctx.edev.db.bel_slots.key(slot);
                    let mut test_muxes = vec![];
                    let mut test_bits: Option<HashMap<_, _>> = None;
                    for (&dst, tmux) in &bel.wires {
                        let mux_name = if tcls.cells.len() == 1 {
                            format!("MUX.{}", intdb.wires.key(dst.wire))
                        } else {
                            format!("MUX.{:#}.{}", dst.cell, intdb.wires.key(dst.wire))
                        };
                        let mut mux_inps = vec![];
                        for &src in tmux.test_src.keys() {
                            let in_name = if tcls.cells.len() == 1 {
                                intdb.wires.key(src.wire).to_string()
                            } else {
                                format!("{:#}.{}", src.cell, intdb.wires.key(src.wire))
                            };
                            let diff = ctx.state.get_diff(tcname, bname, &mux_name, &in_name);

                            match test_bits {
                                Some(ref mut bits) => {
                                    bits.retain(|bit, _| diff.bits.contains_key(bit))
                                }
                                None => {
                                    test_bits =
                                        Some(diff.bits.iter().map(|(&a, &b)| (a, b)).collect())
                                }
                            }

                            mux_inps.push((in_name, diff));
                        }
                        test_muxes.push((mux_name, mux_inps));
                    }
                    let Some(test_bits) = test_bits else { continue };
                    assert_eq!(test_bits.len(), 1);
                    let test_diff = Diff { bits: test_bits };
                    for (_, mux_inps) in &mut test_muxes {
                        for (_, diff) in mux_inps {
                            *diff = diff.combine(&!&test_diff);
                        }
                    }
                    ctx.tiledb
                        .insert(tcname, bname, "TEST_ENABLE", xlat_bit(test_diff));
                    if let ExpandedDevice::Virtex4(edev) = ctx.edev {
                        match edev.kind {
                            prjcombine_virtex4::chip::ChipKind::Virtex4 => {
                                for (_, mux_inps) in &mut test_muxes {
                                    for (in_name, diff) in mux_inps {
                                        if in_name.starts_with("IMUX_CLK")
                                            || in_name.starts_with("IMUX_SR")
                                            || in_name.starts_with("IMUX_CE")
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
                            ctx.tiledb.insert(tcname, bname, mux_name, item);
                        }
                    }
                }
                BelInfo::GroupTestMux(bel) => {
                    let bname = ctx.edev.db.bel_slots.key(slot);
                    let mut diffs = vec![];
                    for (&dst, tmux) in &bel.wires {
                        let mux_name = if tcls.cells.len() == 1 {
                            format!("MUX.{}", intdb.wires.key(dst.wire))
                        } else {
                            format!("MUX.{:#}.{}", dst.cell, intdb.wires.key(dst.wire))
                        };
                        for (group, &src) in tmux.test_src.iter().enumerate() {
                            let Some(src) = src else { continue };
                            let in_name = if tcls.cells.len() == 1 {
                                intdb.wires.key(src.wire).to_string()
                            } else {
                                format!("{:#}.{}", src.cell, intdb.wires.key(src.wire))
                            };
                            let mut diff = ctx.state.get_diff(tcname, bname, &mux_name, &in_name);

                            if in_name.contains("IMUX_SR") || in_name.contains("IMUX_CE") {
                                let mut item = ctx
                                    .tiledb
                                    .item(
                                        "INT_BRAM_S3ADSP",
                                        "INT",
                                        &format!(
                                            "INV.{in_name}",
                                            in_name = intdb.wires.key(src.wire)
                                        ),
                                    )
                                    .clone();
                                for bit in &mut item.bits {
                                    bit.rect = BitRectId::from_idx(src.cell.to_idx());
                                }
                                diff.discard_bits(&item);
                            }

                            diffs.push((group.to_string(), diff));
                        }
                    }
                    ctx.tiledb.insert(
                        tcname,
                        bname,
                        "TEST_GROUP",
                        xlat_enum_default(diffs, "NONE"),
                    );
                }
                _ => (),
            }
        }
    }
}
