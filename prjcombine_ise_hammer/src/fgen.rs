use prjcombine_int::grid::{ColId, DieId, LayerId, RowId};
use prjcombine_virtex_bitstream::BitTile;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;

use prjcombine_hammer::{BatchValue, Fuzzer, FuzzerGen};
use prjcombine_int::db::{BelId, NodeKindId};

use crate::backend::{FuzzerInfo, IseBackend, Key, MultiValue, SimpleFeatureId, State};

type Loc = (DieId, ColId, RowId, LayerId);

#[derive(Debug)]
pub enum TileKV<'a> {
    SiteMode(BelId, &'a str),
    SiteAttr(BelId, &'a str, &'a str),
    SitePin(BelId, &'a str),
    #[allow(dead_code)]
    GlobalOpt(&'a str, &'a str),
}

impl<'a> TileKV<'a> {
    fn apply(
        &self,
        backend: &IseBackend<'a>,
        loc: Loc,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Fuzzer<IseBackend<'a>> {
        match *self {
            TileKV::SiteMode(bel, mode) => {
                let site = &backend.egrid.node(loc).bels[bel];
                fuzzer.base(Key::SiteMode(site), mode)
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
        }
    }
}

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum TileFuzzKV<'a> {
    #[allow(dead_code)]
    SiteMode(BelId, &'a str),
    SiteAttr(BelId, &'a str, &'a str),
    #[allow(dead_code)]
    SiteAttrDiff(BelId, &'a str, &'a str, &'a str),
}

impl<'a> TileFuzzKV<'a> {
    fn apply(
        &self,
        backend: &IseBackend<'a>,
        loc: Loc,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Fuzzer<IseBackend<'a>> {
        match *self {
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
        }
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
}

impl TileBits {
    fn get_bits(&self, backend: &IseBackend, loc: (DieId, ColId, RowId, LayerId)) -> Vec<BitTile> {
        let (die, col, row, _) = loc;
        match *self {
            TileBits::Main(n) => match backend.edev {
                prjcombine_xilinx_geom::ExpandedDevice::Xc4k(_) => todo!(),
                prjcombine_xilinx_geom::ExpandedDevice::Xc5200(_) => todo!(),
                prjcombine_xilinx_geom::ExpandedDevice::Virtex(edev) => {
                    (0..n).map(|idx| edev.btile_main(col, row + idx)).collect()
                }
                prjcombine_xilinx_geom::ExpandedDevice::Virtex2(edev) => {
                    (0..n).map(|idx| edev.btile_main(col, row + idx)).collect()
                }
                prjcombine_xilinx_geom::ExpandedDevice::Spartan6(edev) => {
                    (0..n).map(|idx| edev.btile_main(col, row + idx)).collect()
                }
                prjcombine_xilinx_geom::ExpandedDevice::Virtex4(edev) => (0..n)
                    .map(|idx| edev.btile_main(die, col, row + idx))
                    .collect(),
                prjcombine_xilinx_geom::ExpandedDevice::Ultrascale(_) => todo!(),
                prjcombine_xilinx_geom::ExpandedDevice::Versal(_) => todo!(),
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
            fuzzer = x.apply(backend, loc, fuzzer);
        }
        for x in &self.fuzz {
            fuzzer = x.apply(backend, loc, fuzzer);
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
            fuzzer = x.apply(backend, loc, fuzzer);
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
