use prjcombine_int::{
    db::{NodeRawTileId, NodeTileId, NodeWireId, WireKind},
    grid::{ColId, DieId, LayerId, RowId},
};
use prjcombine_virtex_bitstream::BitTile;
use prjcombine_xilinx_geom::ExpandedDevice;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;
use unnamed_entity::EntityId;

use prjcombine_hammer::{BatchValue, Fuzzer, FuzzerGen};
use prjcombine_int::db::{BelId, NodeKindId};

use crate::backend::{FuzzerInfo, IseBackend, Key, MultiValue, SimpleFeatureId, State};

pub type Loc = (DieId, ColId, RowId, LayerId);

#[derive(Debug, Copy, Clone)]
pub enum TileWire<'a> {
    BelPinNear(BelId, &'a str),
    IntWire(NodeWireId),
}

fn resolve_tile_wire<'a>(
    backend: &IseBackend<'a>,
    loc: Loc,
    wire: TileWire,
) -> Option<(&'a str, &'a str)> {
    let node = backend.egrid.node(loc);
    let intdb = backend.egrid.db;
    let node_naming = &intdb.node_namings[node.naming];
    Some(match wire {
        TileWire::BelPinNear(bel, pin) => {
            let bel_naming = &node_naming.bels[bel];
            (&node.names[bel_naming.tile], &bel_naming.pins[pin].name)
        }
        TileWire::IntWire(w) => {
            backend.egrid.resolve_wire((loc.0, node.tiles[w.0], w.1))?;
            (
                &node.names[NodeRawTileId::from_idx(0)],
                node_naming.wires.get(&w)?,
            )
        }
    })
}

#[derive(Debug)]
pub enum TileKV<'a> {
    SiteMode(BelId, &'a str),
    SiteUnused(BelId),
    SiteAttr(BelId, &'a str, &'a str),
    SitePin(BelId, &'a str),
    #[allow(dead_code)]
    GlobalOpt(&'a str, &'a str),
    GlobalMutexNone(&'a str),
    GlobalMutexSite(&'a str, BelId),
    RowMutexSite(&'a str, BelId),
    RowMutex(&'a str, &'a str),
    SiteMutex(BelId, &'a str, &'a str),
    Pip(TileWire<'a>, TileWire<'a>),
    NodeMutexShared(NodeWireId),
    NodeIntDstFilter(NodeWireId),
    NodeIntDistinct(NodeWireId, NodeWireId),
}

impl<'a> TileKV<'a> {
    fn apply(
        &self,
        backend: &IseBackend<'a>,
        loc: Loc,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<Fuzzer<IseBackend<'a>>> {
        Some(match *self {
            TileKV::SiteMode(bel, mode) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::SiteMode(site), mode)
            }
            TileKV::SiteUnused(bel) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::SiteMode(site), None)
            }
            TileKV::SiteAttr(bel, attr, val) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::SiteAttr(site, attr), val)
            }
            TileKV::SitePin(bel, pin) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::SitePin(site, pin), true)
            }
            TileKV::GlobalOpt(opt, val) => fuzzer.base(Key::GlobalOpt(opt), val),
            TileKV::GlobalMutexNone(name) => fuzzer.base(Key::GlobalMutex(name), None),
            TileKV::GlobalMutexSite(name, bel) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::GlobalMutex(name), &site[..])
            }
            TileKV::RowMutexSite(name, bel) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::RowMutex(name, loc.2), &site[..])
            }
            TileKV::RowMutex(name, val) => fuzzer.base(Key::RowMutex(name, loc.2), val),
            TileKV::SiteMutex(bel, name, val) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::SiteMutex(&site[..], name), val)
            }
            TileKV::Pip(wa, wb) => {
                let (ta, wa) = resolve_tile_wire(backend, loc, wa)?;
                let (tb, wb) = resolve_tile_wire(backend, loc, wb)?;
                assert_eq!(ta, tb);
                fuzzer.base(Key::Pip(ta, wa, wb), true)
            }
            TileKV::NodeMutexShared(wire) => {
                let node = backend.egrid.node(loc);
                let intdb = backend.egrid.db;
                let node_naming = &intdb.node_namings[node.naming];
                node_naming.wires.get(&wire)?;
                let node = backend
                    .egrid
                    .resolve_wire((loc.0, node.tiles[wire.0], wire.1))?;
                fuzzer.base(Key::NodeMutex(node), "SHARED")
            }
            TileKV::NodeIntDstFilter(wire) => {
                let intdb = backend.egrid.db;
                let wire_name = intdb.wires.key(wire.1);
                match backend.edev {
                    ExpandedDevice::Virtex2(edev) => {
                        let node = backend.egrid.node(loc);
                        if !edev.grid.kind.is_virtex2()
                            && backend.egrid.db.nodes.key(node.kind) == "INT.BRAM"
                            && (wire_name.starts_with("IMUX.CLK")
                                || wire_name.starts_with("IMUX.SR")
                                || wire_name.starts_with("IMUX.CE"))
                        {
                            let mut tgt = None;
                            for i in 0..4 {
                                if let Some(bram_node) =
                                    backend.egrid.find_node(loc.0, (loc.1, loc.2), |node| {
                                        intdb.nodes.key(node.kind).starts_with("BRAM")
                                    })
                                {
                                    tgt = Some((bram_node, i));
                                    break;
                                }
                            }
                            let (bram_node, idx) = tgt.unwrap();
                            let node_tile = NodeTileId::from_idx(idx);
                            let bram_node_kind = &intdb.nodes[bram_node.kind];
                            let mut found = false;
                            for bel in bram_node_kind.bels.values() {
                                for pin in bel.pins.values() {
                                    if pin.wires.contains(&(node_tile, wire.1)) {
                                        found = true;
                                        break;
                                    }
                                }
                            }
                            if !found {
                                return None;
                            }
                        }
                        if edev.grid.kind.is_spartan3a()
                            && backend.egrid.db.nodes.key(node.kind) == "INT.CLB"
                        {
                            // avoid SR in corners — it causes the inverter bit to be auto-set
                            let is_lr =
                                loc.1 == edev.grid.col_left() || loc.1 == edev.grid.col_right();
                            let is_bt =
                                loc.2 == edev.grid.row_bot() || loc.2 == edev.grid.row_top();
                            if intdb.wires.key(wire.1).starts_with("IMUX.SR") && is_lr && is_bt {
                                return None;
                            }
                        }
                        // TODO
                    }
                    ExpandedDevice::Virtex4(edev) => {
                        if edev.kind == prjcombine_virtex4::grid::GridKind::Virtex4 {
                            // avoid CLK in center column — using it on DCM tiles causes the inverter bit to be auto-set
                            if intdb.wires.key(wire.1).starts_with("IMUX.CLK")
                                && loc.1 == edev.col_clk
                            {
                                return None;
                            }
                        }
                    }
                    _ => (),
                }
                fuzzer
            }
            TileKV::NodeIntDistinct(a, b) => {
                let node = backend.egrid.node(loc);
                let a = backend.egrid.resolve_wire((loc.0, node.tiles[a.0], a.1))?;
                let b = backend.egrid.resolve_wire((loc.0, node.tiles[b.0], b.1))?;
                if a == b {
                    return None;
                }
                fuzzer
            }
        })
    }
}

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum TileFuzzKV<'a> {
    SiteMode(BelId, &'a str),
    SiteAttr(BelId, &'a str, &'a str),
    #[allow(dead_code)]
    SiteAttrDiff(BelId, &'a str, &'a str, &'a str),
    GlobalOpt(&'a str, &'a str),
    GlobalOptDiff(&'a str, &'a str, &'a str),
    Pip(TileWire<'a>, TileWire<'a>),
    NodeMutexExclusive(NodeWireId),
}

impl<'a> TileFuzzKV<'a> {
    fn apply(
        &self,
        backend: &IseBackend<'a>,
        loc: Loc,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<Fuzzer<IseBackend<'a>>> {
        Some(match *self {
            TileFuzzKV::SiteMode(bel, val) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.fuzz(Key::SiteMode(site), None, val)
            }
            TileFuzzKV::SiteAttr(bel, attr, val) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.fuzz(Key::SiteAttr(site, attr), None, val)
            }
            TileFuzzKV::SiteAttrDiff(bel, attr, va, vb) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.fuzz(Key::SiteAttr(site, attr), va, vb)
            }
            TileFuzzKV::GlobalOpt(opt, val) => fuzzer.fuzz(Key::GlobalOpt(opt), None, val),
            TileFuzzKV::GlobalOptDiff(opt, vala, valb) => {
                fuzzer.fuzz(Key::GlobalOpt(opt), vala, valb)
            }
            TileFuzzKV::Pip(wa, wb) => {
                let (ta, wa) = resolve_tile_wire(backend, loc, wa)?;
                let (tb, wb) = resolve_tile_wire(backend, loc, wb)?;
                assert_eq!(ta, tb);
                fuzzer.fuzz(Key::Pip(ta, wa, wb), None, true)
            }
            TileFuzzKV::NodeMutexExclusive(wire) => {
                let node = backend.egrid.node(loc);
                let intdb = backend.egrid.db;
                let node_naming = &intdb.node_namings[node.naming];
                node_naming.wires.get(&wire)?;
                let node = backend
                    .egrid
                    .resolve_wire((loc.0, node.tiles[wire.0], wire.1))?;
                fuzzer.fuzz(Key::NodeMutex(node), None, "EXCLUSIVE")
            }
        })
    }
}

#[derive(Debug)]
pub enum TileMultiFuzzKV<'a> {
    SiteAttr(BelId, &'a str, MultiValue),
}

impl<'a> TileMultiFuzzKV<'a> {
    fn apply(
        &self,
        backend: &IseBackend<'a>,
        loc: Loc,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Fuzzer<IseBackend<'a>> {
        match *self {
            TileMultiFuzzKV::SiteAttr(bel, attr, val) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.fuzz_multi(Key::SiteAttr(site, attr), val)
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum TileBits {
    Main(usize),
    Bram,
}

impl TileBits {
    fn get_bits(&self, backend: &IseBackend, loc: (DieId, ColId, RowId, LayerId)) -> Vec<BitTile> {
        let (die, col, row, _) = loc;
        match *self {
            TileBits::Main(n) => match backend.edev {
                ExpandedDevice::Xc4k(_) => todo!(),
                ExpandedDevice::Xc5200(_) => todo!(),
                ExpandedDevice::Virtex(edev) => {
                    (0..n).map(|idx| edev.btile_main(col, row + idx)).collect()
                }
                ExpandedDevice::Virtex2(edev) => {
                    (0..n).map(|idx| edev.btile_main(col, row + idx)).collect()
                }
                ExpandedDevice::Spartan6(edev) => {
                    (0..n).map(|idx| edev.btile_main(col, row + idx)).collect()
                }
                ExpandedDevice::Virtex4(edev) => (0..n)
                    .map(|idx| edev.btile_main(die, col, row + idx))
                    .collect(),
                ExpandedDevice::Ultrascale(_) => todo!(),
                ExpandedDevice::Versal(_) => todo!(),
            },
            TileBits::Bram => match backend.edev {
                ExpandedDevice::Xc4k(_) => unreachable!(),
                ExpandedDevice::Xc5200(_) => unreachable!(),
                ExpandedDevice::Virtex(edev) => {
                    vec![
                        edev.btile_main(col, row),
                        edev.btile_main(col, row + 1),
                        edev.btile_main(col, row + 2),
                        edev.btile_main(col, row + 3),
                        edev.btile_bram(col, row),
                    ]
                }
                ExpandedDevice::Virtex2(edev) => {
                    vec![
                        edev.btile_main(col, row),
                        edev.btile_main(col, row + 1),
                        edev.btile_main(col, row + 2),
                        edev.btile_main(col, row + 3),
                        edev.btile_bram(col, row),
                    ]
                }
                ExpandedDevice::Spartan6(edev) => {
                    todo!()
                }
                ExpandedDevice::Virtex4(edev) => {
                    if edev.kind == prjcombine_virtex4::grid::GridKind::Virtex4 {
                        vec![
                            edev.btile_main(die, col, row),
                            edev.btile_main(die, col, row + 1),
                            edev.btile_main(die, col, row + 2),
                            edev.btile_main(die, col, row + 3),
                            edev.btile_bram(die, col, row),
                        ]
                    } else {
                        vec![
                            edev.btile_main(die, col, row),
                            edev.btile_main(die, col, row + 1),
                            edev.btile_main(die, col, row + 2),
                            edev.btile_main(die, col, row + 3),
                            edev.btile_main(die, col, row + 4),
                            edev.btile_bram(die, col, row),
                        ]
                    }
                }
                ExpandedDevice::Ultrascale(_) => {
                    todo!()
                }
                ExpandedDevice::Versal(_) => {
                    todo!()
                }
            },
        }
    }
}

#[derive(Debug)]
pub struct TileFuzzerGen<'a> {
    pub node: NodeKindId,
    pub bits: TileBits,
    pub feature: SimpleFeatureId<'a>,
    pub base: Vec<TileKV<'a>>,
    pub fuzz: Vec<TileFuzzKV<'a>>,
}

impl<'b> TileFuzzerGen<'b> {
    fn try_gen(
        &self,
        backend: &IseBackend<'b>,
        kv: &HashMap<Key<'b>, BatchValue<IseBackend<'b>>>,
        loc: (DieId, ColId, RowId, LayerId),
    ) -> Option<Fuzzer<IseBackend<'b>>> {
        let bits = self.bits.get_bits(backend, loc);
        let mut fuzzer = Fuzzer::new(FuzzerInfo::Simple(bits, self.feature));
        for x in &self.base {
            fuzzer = x.apply(backend, loc, fuzzer)?;
        }
        for x in &self.fuzz {
            fuzzer = x.apply(backend, loc, fuzzer)?;
        }
        if fuzzer.is_ok(kv) {
            Some(fuzzer)
        } else {
            None
        }
    }
}

impl<'b> FuzzerGen<IseBackend<'b>> for TileFuzzerGen<'b> {
    fn gen<'a>(
        &self,
        backend: &'a IseBackend<'b>,
        _state: &mut State,
        kv: &HashMap<Key<'b>, BatchValue<IseBackend<'b>>>,
    ) -> Option<(
        Fuzzer<IseBackend<'b>>,
        Option<Box<dyn FuzzerGen<IseBackend<'b>> + 'a>>,
    )> {
        let locs = &backend.egrid.node_index[self.node];
        let mut rng = thread_rng();
        if locs.len() > 20 {
            for &loc in locs.choose_multiple(&mut rng, 20) {
                if let Some(res) = self.try_gen(backend, kv, loc) {
                    return Some((res, None));
                }
            }
        }
        for &loc in locs.choose_multiple(&mut rng, locs.len()) {
            if let Some(res) = self.try_gen(backend, kv, loc) {
                return Some((res, None));
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct TileMultiFuzzerGen<'a> {
    pub node: NodeKindId,
    pub bits: TileBits,
    pub feature: SimpleFeatureId<'a>,
    pub base: Vec<TileKV<'a>>,
    pub width: usize,
    pub fuzz: TileMultiFuzzKV<'a>,
}

impl<'b> TileMultiFuzzerGen<'b> {
    fn try_gen(
        &self,
        backend: &IseBackend<'b>,
        kv: &HashMap<Key<'b>, BatchValue<IseBackend<'b>>>,
        loc: (DieId, ColId, RowId, LayerId),
    ) -> Option<Fuzzer<IseBackend<'b>>> {
        let bits = self.bits.get_bits(backend, loc);
        let mut fuzzer = Fuzzer::new(FuzzerInfo::Simple(bits, self.feature));
        for x in &self.base {
            fuzzer = x.apply(backend, loc, fuzzer)?;
        }
        fuzzer = fuzzer.bits(self.width);
        fuzzer = self.fuzz.apply(backend, loc, fuzzer);
        if fuzzer.is_ok(kv) {
            Some(fuzzer)
        } else {
            None
        }
    }
}

impl<'b> FuzzerGen<IseBackend<'b>> for TileMultiFuzzerGen<'b> {
    fn gen<'a>(
        &self,
        backend: &'a IseBackend<'b>,
        _state: &mut State,
        kv: &HashMap<Key<'b>, BatchValue<IseBackend<'b>>>,
    ) -> Option<(
        Fuzzer<IseBackend<'b>>,
        Option<Box<dyn FuzzerGen<IseBackend<'b>> + 'a>>,
    )> {
        let locs = &backend.egrid.node_index[self.node];
        let mut rng = thread_rng();
        if locs.len() > 20 {
            for &loc in locs.choose_multiple(&mut rng, 20) {
                if let Some(res) = self.try_gen(backend, kv, loc) {
                    return Some((res, None));
                }
            }
        }
        for &loc in locs.choose_multiple(&mut rng, locs.len()) {
            if let Some(res) = self.try_gen(backend, kv, loc) {
                return Some((res, None));
            }
        }
        None
    }
}
