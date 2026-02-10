use std::collections::{HashMap, HashSet};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, TileWireCoord, WireSlotId},
    grid::TileCoord,
};
use prjcombine_re_collector::diff::{Diff, OcdMode, xlat_enum_raw};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_re_xilinx_naming::db::{IntfWireInNaming, RawTileId};
use prjcombine_types::bsdata::BitRectId;
use prjcombine_virtex2::defs::spartan3::{tcls as tcls_s3, wires as wires_s3};
use prjcombine_virtex4::defs::virtex4::{tcls as tcls_v4, wires as wires_v4};

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
        && ndb.tile_class_namings.key(ntile.naming) == "INTF_PPC_R"
        && intdb.wires.key(wire_from.wire).starts_with("TEST")
    {
        // ISE.
        return None;
    }
    Some((
        &ntile.names[RawTileId::from_idx(0)],
        &tile_naming.wires.get(&wire_to)?.name,
        match tile_naming.intf_wires_in.get(&wire_from)? {
            IntfWireInNaming::Simple { name } => name,
            IntfWireInNaming::Buf { name_in, .. } => name_in,
            IntfWireInNaming::TestBuf { name_out, .. } => name_out,
            IntfWireInNaming::Anonymous => unreachable!(),
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

fn is_anon_wire(edev: &ExpandedDevice, wire: WireSlotId) -> bool {
    match edev {
        ExpandedDevice::Virtex2(_) => false,
        ExpandedDevice::Spartan6(_) => prjcombine_spartan6::defs::wires::OUT_TEST.contains(wire),
        ExpandedDevice::Virtex4(edev) => match edev.kind {
            prjcombine_virtex4::chip::ChipKind::Virtex4 => {
                prjcombine_virtex4::defs::virtex4::wires::OUT_HALF0_TEST.contains(wire)
                    || prjcombine_virtex4::defs::virtex4::wires::OUT_HALF1_TEST.contains(wire)
            }
            prjcombine_virtex4::chip::ChipKind::Virtex5 => {
                prjcombine_virtex4::defs::virtex5::wires::OUT_TEST.contains(wire)
            }
            prjcombine_virtex4::chip::ChipKind::Virtex6 => {
                prjcombine_virtex4::defs::virtex6::wires::OUT_TEST.contains(wire)
            }
            prjcombine_virtex4::chip::ChipKind::Virtex7 => {
                prjcombine_virtex4::defs::virtex7::wires::OUT_TEST.contains(wire)
            }
        },
        _ => unreachable!(),
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.edev.db;
    for (tcid, _, tcls) in &intdb.tile_classes {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        for (slot, bel) in &tcls.bels {
            let BelInfo::TestMux(bel) = bel else {
                continue;
            };
            let mut bctx = ctx.bel(slot);
            for (&dst, tmux) in &bel.wires {
                for &src in tmux.test_src.iter().flatten() {
                    if is_anon_wire(backend.edev, src.wire) {
                        for &src in &backend.edev.db_index.tile_classes[tcid].pips_bwd[&src.tw] {
                            bctx.build()
                                .prop(IntMutex::new("INTF".into()))
                                .test_routing(dst, src)
                                .prop(TileMutexExclusive::new("INTF".into()))
                                .prop(WireMutexExclusive::new(dst))
                                .prop(WireMutexExclusive::new(src.tw))
                                .prop(FuzzIntfTestPip::new(dst, src.tw))
                                .commit();
                        }
                    } else {
                        bctx.build()
                            .prop(IntMutex::new("INTF".into()))
                            .test_routing(dst, src)
                            .prop(TileMutexExclusive::new("INTF".into()))
                            .prop(WireMutexExclusive::new(dst))
                            .prop(WireMutexExclusive::new(src.tw))
                            .prop(FuzzIntfTestPip::new(dst, src.tw))
                            .commit();
                    }
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let intdb = ctx.edev.db;
    for (tcid, _, tcls) in &intdb.tile_classes {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        for (bslot, bel) in &tcls.bels {
            let BelInfo::TestMux(bel) = bel else {
                continue;
            };
            let mut group_diffs = vec![(None, Diff::default())];
            for group in 0..bel.groups.len() {
                let mut test_muxes = vec![];
                let mut test_bits: Option<HashMap<_, _>> = None;
                for (&dst, tout) in &bel.wires {
                    let Some(tsrc) = tout.test_src[group] else {
                        continue;
                    };

                    let inps = if is_anon_wire(ctx.edev, tsrc.wire) {
                        Vec::from_iter(
                            ctx.edev.db_index.tile_classes[tcid].pips_bwd[&tsrc.tw]
                                .iter()
                                .copied(),
                        )
                    } else {
                        vec![tsrc]
                    };

                    let mut mux_diffs = vec![];
                    for src in inps {
                        let mut diff = ctx.get_diff_routing(tcid, dst, src);
                        if let ExpandedDevice::Virtex2(edev) = ctx.edev
                            && !edev.chip.kind.is_virtex2()
                            && (wires_s3::IMUX_SR_OPTINV.contains(src.wire)
                                || wires_s3::IMUX_CE_OPTINV.contains(src.wire))
                        {
                            let mut bit = ctx.sb_inv(
                                tcls_s3::INT_BRAM_S3ADSP,
                                TileWireCoord::new_idx(0, src.wire),
                            );
                            bit.bit.rect = BitRectId::from_idx(src.cell.to_idx());
                            diff.discard_bits(&[bit.bit]);
                        }
                        if let ExpandedDevice::Virtex4(edev) = ctx.edev
                            && edev.kind == prjcombine_virtex4::chip::ChipKind::Virtex4
                            && (wires_v4::IMUX_SR_OPTINV.contains(src.wire)
                                || wires_v4::IMUX_CE_OPTINV.contains(src.wire)
                                || wires_v4::IMUX_CLK_OPTINV.contains(src.wire))
                        {
                            diff.discard_bits(&[ctx.sb_inv(tcls_v4::INT, src.tw).bit]);
                        }

                        match test_bits {
                            Some(ref mut bits) => bits.retain(|bit, _| diff.bits.contains_key(bit)),
                            None => {
                                test_bits = Some(diff.bits.iter().map(|(&a, &b)| (a, b)).collect())
                            }
                        }

                        mux_diffs.push((src, diff));
                    }
                    test_muxes.push((tsrc, mux_diffs));
                }
                let test_diff = Diff {
                    bits: test_bits.unwrap(),
                };
                for (_, mux_inps) in &mut test_muxes {
                    for (_, diff) in mux_inps {
                        *diff = diff.combine(&!&test_diff);
                    }
                }

                group_diffs.push((Some(group), test_diff));

                if let ExpandedDevice::Virtex4(edev) = ctx.edev
                    && edev.kind == prjcombine_virtex4::chip::ChipKind::Virtex6
                {
                    let mut new_test_muxes = vec![];
                    let mut known_bits = HashSet::new();
                    for &(tsrc, ref mux_inps) in &test_muxes {
                        let (_, _, common) =
                            Diff::split(mux_inps[0].1.clone(), mux_inps[1].1.clone());
                        let mut new_mux_inps = vec![];
                        for &(src, ref diff) in mux_inps {
                            let (diff, empty, check_common) =
                                Diff::split(diff.clone(), common.clone());
                            assert_eq!(check_common, common);
                            empty.assert_empty();
                            for &bit in diff.bits.keys() {
                                known_bits.insert(bit);
                            }
                            new_mux_inps.push((src, diff));
                        }
                        new_test_muxes.push((tsrc, new_mux_inps));
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
                for (dst, srcs) in test_muxes {
                    let dst = dst.tw;
                    if srcs.len() == 1 && srcs[0].0.tw == dst {
                        srcs[0].1.assert_empty();
                    } else {
                        let has_empty = srcs.iter().any(|(_, diff)| diff.bits.is_empty());
                        let mut diffs = Vec::from_iter(srcs.into_iter().map(|(k, v)| (Some(k), v)));
                        if !has_empty {
                            diffs.push((None, Diff::default()));
                        }
                        let item = xlat_enum_raw(diffs, OcdMode::Mux);
                        ctx.insert_mux(tcid, dst, item);
                    }
                }
            }
            ctx.insert_tmux_group(tcid, bslot, xlat_enum_raw(group_diffs, OcdMode::ValueOrder));
        }
    }
}
