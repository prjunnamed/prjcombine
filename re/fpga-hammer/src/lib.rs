use core::fmt::Debug;
use std::collections::{BTreeMap, BTreeSet, HashMap, btree_map};

use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::{
    db::{TileClassId, TileSlotId},
    grid::{CellCoord, ColId, DieId, ExpandedGrid, RowId, TileCoord},
};
use prjcombine_re_hammer::{Backend, BatchValue, Fuzzer, FuzzerGen, FuzzerId};
use prjcombine_types::{
    bitrect::BitRect,
    bsdata::{BitRectId, TileBit},
};
use rand::seq::IndexedRandom;

use prjcombine_re_collector::diff::{Diff, DiffKey};

pub trait FpgaBackend: Backend<State = State, FuzzerInfo = FuzzerInfo<Self::BitRect>> {
    type BitRect: BitRect<BitPos = Self::BitPos>;

    fn tile_bits(&self, tcrd: TileCoord) -> EntityVec<BitRectId, Self::BitRect>;

    fn egrid(&self) -> &ExpandedGrid<'_>;
}

#[derive(Clone, Debug)]
pub struct FuzzerFeature<BitRect> {
    pub key: DiffKey,
    pub rects: EntityVec<BitRectId, BitRect>,
}

#[derive(Clone)]
pub struct FuzzerInfo<BitRect> {
    pub features: Vec<FuzzerFeature<BitRect>>,
}

impl<BitRect> std::fmt::Debug for FuzzerInfo<BitRect> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.features[0].key)
    }
}

pub trait FuzzerProp<'b, B: FpgaBackend>: Debug {
    fn dyn_clone(&self) -> Box<dyn FuzzerProp<'b, B> + 'b>;

    fn apply(&self, backend: &B, tcrd: TileCoord, fuzzer: Fuzzer<B>) -> Option<(Fuzzer<B>, bool)>;
}

impl<'a, B: FpgaBackend> Clone for Box<dyn FuzzerProp<'a, B> + 'a> {
    fn clone(&self) -> Self {
        self.dyn_clone()
    }
}

#[derive(Debug)]
pub struct FpgaFuzzerGen<'b, B: FpgaBackend> {
    pub tile_class: Option<TileClassId>,
    pub key: DiffKey,
    pub props: Vec<Box<dyn FuzzerProp<'b, B> + 'b>>,
}

impl<B: FpgaBackend> Clone for FpgaFuzzerGen<'_, B> {
    fn clone(&self) -> Self {
        Self {
            tile_class: self.tile_class,
            key: self.key.clone(),
            props: self.props.clone(),
        }
    }
}

impl<B: FpgaBackend> FpgaFuzzerGen<'_, B> {
    pub(crate) fn try_generate(
        &self,
        backend: &B,
        kv: &HashMap<B::Key, BatchValue<B>>,
        tcrd: TileCoord,
    ) -> Option<(Fuzzer<B>, BTreeSet<usize>)> {
        let rects = if self.tile_class.is_some() {
            backend.tile_bits(tcrd)
        } else {
            EntityVec::new()
        };
        let mut fuzzer = Fuzzer::new(FuzzerInfo {
            features: vec![FuzzerFeature {
                rects,
                key: self.key.clone(),
            }],
        });
        let mut sad_props = BTreeSet::new();
        for (idx, prop) in self.props.iter().enumerate() {
            let sad;
            (fuzzer, sad) = prop.apply(backend, tcrd, fuzzer)?;
            if sad {
                sad_props.insert(idx);
            }
        }
        if !fuzzer.is_ok(kv) {
            return None;
        }
        Some((fuzzer, sad_props))
    }
}

impl<'b, B: FpgaBackend> FuzzerGen<'b, B> for FpgaFuzzerGen<'b, B> {
    fn generate(
        &self,
        backend: &'b B,
        _state: &mut State,
        kv: &HashMap<B::Key, BatchValue<B>>,
    ) -> Option<(Fuzzer<B>, Option<Box<dyn FuzzerGen<'b, B> + 'b>>)> {
        let (res, sad_props) = if let Some(tile_class) = self.tile_class {
            let locs = &backend.egrid().tile_index[tile_class];
            let mut rng = rand::rng();
            'find: {
                if locs.len() > 20 {
                    for &loc in locs.choose_multiple(&mut rng, 20) {
                        if let Some(x) = self.try_generate(backend, kv, loc) {
                            break 'find x;
                        }
                    }
                }
                for &loc in locs.choose_multiple(&mut rng, locs.len()) {
                    if let Some(x) = self.try_generate(backend, kv, loc) {
                        break 'find x;
                    }
                }
                return None;
            }
        } else {
            let tcrd = CellCoord::new(DieId::from_idx(0), ColId::from_idx(0), RowId::from_idx(0))
                .tile(TileSlotId::from_idx(0));
            self.try_generate(backend, kv, tcrd)?
        };
        if !sad_props.is_empty() {
            return Some((
                res,
                Some(Box::new(FpgaFuzzerChainGen {
                    orig: self.clone(),
                    sad_props,
                })),
            ));
        }
        Some((res, None))
    }
}

#[derive(Debug)]
pub(crate) struct FpgaFuzzerChainGen<'b, B: FpgaBackend> {
    pub(crate) orig: FpgaFuzzerGen<'b, B>,
    pub(crate) sad_props: BTreeSet<usize>,
}

impl<'b, B: FpgaBackend> FuzzerGen<'b, B> for FpgaFuzzerChainGen<'b, B> {
    fn generate(
        &self,
        backend: &'b B,
        _state: &mut State,
        kv: &HashMap<B::Key, BatchValue<B>>,
    ) -> Option<(Fuzzer<B>, Option<Box<dyn FuzzerGen<'b, B> + 'b>>)> {
        let (res, mut sad_props) = if let Some(tile_class) = self.orig.tile_class {
            let locs = &backend.egrid().tile_index[tile_class];
            let mut rng = rand::rng();
            'find: {
                if locs.len() > 20 {
                    for &loc in locs.choose_multiple(&mut rng, 20) {
                        if let Some(x) = self.orig.try_generate(backend, kv, loc) {
                            for &prop in &self.sad_props {
                                if !x.1.contains(&prop) {
                                    break 'find x;
                                }
                            }
                        }
                    }
                }
                for &loc in locs.choose_multiple(&mut rng, locs.len()) {
                    if let Some(x) = self.orig.try_generate(backend, kv, loc) {
                        for &prop in &self.sad_props {
                            if !x.1.contains(&prop) {
                                break 'find x;
                            }
                        }
                    }
                }
                return None;
            }
        } else {
            let tcrd = CellCoord::new(DieId::from_idx(0), ColId::from_idx(0), RowId::from_idx(0))
                .tile(TileSlotId::from_idx(0));
            self.orig.try_generate(backend, kv, tcrd)?
        };
        sad_props.retain(|&idx| self.sad_props.contains(&idx));
        if !sad_props.is_empty() {
            return Some((
                res,
                Some(Box::new(FpgaFuzzerChainGen {
                    orig: self.orig.clone(),
                    sad_props,
                })),
            ));
        }
        Some((res, None))
    }
}

#[derive(Clone, Debug)]
pub struct FeatureData {
    pub diffs: Vec<Diff>,
    pub fuzzers: Vec<FuzzerId>,
}

#[derive(Debug, Default)]
pub struct State {
    pub features: BTreeMap<DiffKey, FeatureData>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_diff(
        &mut self,
        key: DiffKey,
        diffs: Vec<Diff>,
        fuzzers: Vec<FuzzerId>,
    ) -> Result<(), Vec<FuzzerId>> {
        if diffs.len() != 1
            && let DiffKey::BelAttrBit(tcid, bslot, attr, 0) = key
        {
            for (idx, diff) in diffs.into_iter().enumerate() {
                self.insert_diff(
                    DiffKey::BelAttrBit(tcid, bslot, attr, idx),
                    vec![diff],
                    fuzzers.clone(),
                )?;
            }
            return Ok(());
        }
        match self.features.entry(key) {
            btree_map::Entry::Occupied(mut e) => {
                let v = e.get();
                if v.diffs != diffs {
                    eprintln!(
                        "bits mismatch for {key:?}: {vbits:?} vs {diffs:?}",
                        key = e.key(),
                        vbits = v.diffs
                    );
                    return Err(v.fuzzers.clone());
                } else {
                    let v = e.get_mut();
                    v.fuzzers.extend(fuzzers);
                }
            }
            btree_map::Entry::Vacant(e) => {
                e.insert(FeatureData { diffs, fuzzers });
            }
        }
        Ok(())
    }

    pub fn return_fuzzer<P: Copy + Debug, T: BitRect<BitPos = P>>(
        &mut self,
        f: &FuzzerInfo<T>,
        fid: FuzzerId,
        bits: Vec<HashMap<P, bool>>,
    ) -> Option<Vec<FuzzerId>> {
        let mut fdiffs: Vec<_> = f
            .features
            .iter()
            .map(|_| vec![Diff::default(); bits.len()])
            .collect();
        for (bitidx, bbits) in bits.iter().enumerate() {
            'bits: for (&k, &v) in bbits {
                for (fidx, feat) in f.features.iter().enumerate() {
                    for (rid, rect) in &feat.rects {
                        if let Some(xk) = rect.xlat_pos_rev(k) {
                            fdiffs[fidx][bitidx].bits.insert(
                                TileBit {
                                    rect: rid,
                                    frame: xk.0,
                                    bit: xk.1,
                                },
                                v,
                            );
                            continue 'bits;
                        }
                    }
                }
                eprintln!("failed to xlat bit {k:?} [bits {bbits:?}] for {f:?}, candidates:");
                for feat in &f.features {
                    println!("{:?}: {:?}", feat.key, feat.rects);
                }
                return Some(vec![]);
            }
        }
        for (feat, xdiffs) in f.features.iter().zip(fdiffs) {
            // if self.debug >= 3 {
            //     eprintln!("RETURN {feat:?} {xdiffs:?}");
            // }
            if feat.rects.is_empty() {
                for diff in &xdiffs {
                    if !diff.bits.is_empty() {
                        eprintln!("null fuzzer {f:?} with bits: {xdiffs:?}");
                        return Some(vec![]);
                    }
                }
            } else {
                if let Err(err) = self.insert_diff(feat.key.clone(), xdiffs, vec![fid]) {
                    return Some(err);
                }
            }
        }
        None
    }
}
