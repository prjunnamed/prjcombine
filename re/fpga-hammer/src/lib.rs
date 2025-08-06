use core::fmt::Debug;
use core::hash::Hash;
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, btree_map, hash_map},
    ops::Range,
};

use itertools::Itertools;
use prjcombine_interconnect::{
    db::{TileClassId, TileSlotId},
    grid::{CellCoord, ColId, DieId, ExpandedGrid, RowId, TileCoord},
};
use prjcombine_re_hammer::{Backend, BatchValue, Fuzzer, FuzzerGen, FuzzerId};
use prjcombine_types::bsdata::{BsData, TileBit, TileItem, TileItemKind};
use prjcombine_types::{bittile::BitTile, bitvec::BitVec};
use rand::seq::IndexedRandom;
use unnamed_entity::EntityId;

pub trait FpgaBackend: Backend<State = State, FuzzerInfo = FuzzerInfo<Self::BitTile>> {
    type BitTile: BitTile<BitPos = Self::BitPos>;

    fn tile_bits(&self, nloc: TileCoord) -> Vec<Self::BitTile>;

    fn egrid(&self) -> &ExpandedGrid<'_>;
}

#[derive(Clone, Debug)]
pub struct FuzzerFeature<BitTile> {
    pub id: FeatureId,
    pub tiles: Vec<BitTile>,
}

#[derive(Clone)]
pub struct FuzzerInfo<BitTile> {
    pub features: Vec<FuzzerFeature<BitTile>>,
}

impl<BitTile> std::fmt::Debug for FuzzerInfo<BitTile> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.features[0].id)
    }
}

pub trait FuzzerProp<'b, B: FpgaBackend>: Debug {
    fn dyn_clone(&self) -> Box<dyn FuzzerProp<'b, B> + 'b>;

    fn apply(&self, backend: &B, nloc: TileCoord, fuzzer: Fuzzer<B>) -> Option<(Fuzzer<B>, bool)>;
}

impl<'a, B: FpgaBackend> Clone for Box<dyn FuzzerProp<'a, B> + 'a> {
    fn clone(&self) -> Self {
        self.dyn_clone()
    }
}

#[derive(Debug)]
pub struct FpgaFuzzerGen<'b, B: FpgaBackend> {
    pub node_kind: Option<TileClassId>,
    pub feature: FeatureId,
    pub props: Vec<Box<dyn FuzzerProp<'b, B> + 'b>>,
}

impl<B: FpgaBackend> Clone for FpgaFuzzerGen<'_, B> {
    fn clone(&self) -> Self {
        Self {
            node_kind: self.node_kind,
            feature: self.feature.clone(),
            props: self.props.clone(),
        }
    }
}

impl<B: FpgaBackend> FpgaFuzzerGen<'_, B> {
    fn try_generate(
        &self,
        backend: &B,
        kv: &HashMap<B::Key, BatchValue<B>>,
        nloc: TileCoord,
    ) -> Option<(Fuzzer<B>, BTreeSet<usize>)> {
        let tiles = if self.node_kind.is_some() {
            backend.tile_bits(nloc)
        } else {
            vec![]
        };
        let mut fuzzer = Fuzzer::new(FuzzerInfo {
            features: vec![FuzzerFeature {
                tiles,
                id: self.feature.clone(),
            }],
        });
        let mut sad_props = BTreeSet::new();
        for (idx, prop) in self.props.iter().enumerate() {
            let sad;
            (fuzzer, sad) = prop.apply(backend, nloc, fuzzer)?;
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
        let (res, sad_props) = if let Some(node_kind) = self.node_kind {
            let locs = &backend.egrid().tile_index[node_kind];
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
            let nloc = CellCoord::new(DieId::from_idx(0), ColId::from_idx(0), RowId::from_idx(0))
                .tile(TileSlotId::from_idx(0));
            self.try_generate(backend, kv, nloc)?
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
struct FpgaFuzzerChainGen<'b, B: FpgaBackend> {
    orig: FpgaFuzzerGen<'b, B>,
    sad_props: BTreeSet<usize>,
}

impl<'b, B: FpgaBackend> FuzzerGen<'b, B> for FpgaFuzzerChainGen<'b, B> {
    fn generate(
        &self,
        backend: &'b B,
        _state: &mut State,
        kv: &HashMap<B::Key, BatchValue<B>>,
    ) -> Option<(Fuzzer<B>, Option<Box<dyn FuzzerGen<'b, B> + 'b>>)> {
        let (res, mut sad_props) = if let Some(node_kind) = self.orig.node_kind {
            let locs = &backend.egrid().tile_index[node_kind];
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
            let nloc = CellCoord::new(DieId::from_idx(0), ColId::from_idx(0), RowId::from_idx(0))
                .tile(TileSlotId::from_idx(0));
            self.orig.try_generate(backend, kv, nloc)?
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

#[derive(Clone, Eq, PartialEq, Default)]
pub struct Diff {
    pub bits: HashMap<TileBit, bool>,
}

impl std::fmt::Debug for Diff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (&k, &v) in self.bits.iter().sorted() {
            write!(f, "{k:?}:{v}, ")?;
        }
        write!(f, "]")?;
        Ok(())
    }
}

impl Diff {
    #[track_caller]
    pub fn assert_empty(&self) {
        assert!(self.bits.is_empty());
    }

    pub fn combine(&self, other: &Diff) -> Diff {
        let mut res = self.clone();
        for (k, v) in &other.bits {
            match res.bits.entry(*k) {
                hash_map::Entry::Occupied(e) => {
                    assert_eq!(*e.get(), !*v);
                    e.remove();
                }
                hash_map::Entry::Vacant(e) => {
                    e.insert(*v);
                }
            }
        }
        res
    }

    pub fn split(mut a: Diff, mut b: Diff) -> (Diff, Diff, Diff) {
        let mut common = Diff::default();
        a.bits.retain(|&k, av| match b.bits.remove(&k) {
            Some(bv) => {
                assert_eq!(*av, bv);
                common.bits.insert(k, bv);
                false
            }
            None => true,
        });
        (a, b, common)
    }

    pub fn split_bits_by(&mut self, mut f: impl FnMut(TileBit) -> bool) -> Diff {
        let mut res = Diff::default();
        self.bits.retain(|&k, &mut v| {
            if f(k) {
                res.bits.insert(k, v);
                false
            } else {
                true
            }
        });
        res
    }

    pub fn split_bits(&mut self, bits: &HashSet<TileBit>) -> Diff {
        self.split_bits_by(|bit| bits.contains(&bit))
    }

    pub fn discard_bits(&mut self, item: &TileItem) {
        for bit in item.bits.iter() {
            self.bits.remove(bit);
        }
    }

    pub fn apply_bitvec_diff(&mut self, item: &TileItem, from: &BitVec, to: &BitVec) {
        let TileItemKind::BitVec { ref invert } = item.kind else {
            unreachable!()
        };
        for (idx, &bit) in item.bits.iter().enumerate() {
            if from[idx] != to[idx] {
                match self.bits.entry(bit) {
                    hash_map::Entry::Occupied(e) => {
                        assert_eq!(*e.get(), from[idx] ^ invert[idx]);
                        e.remove();
                    }
                    hash_map::Entry::Vacant(e) => {
                        e.insert(to[idx] ^ invert[idx]);
                    }
                }
            }
        }
    }

    pub fn apply_bitvec_diff_int(&mut self, item: &TileItem, from: u64, to: u64) {
        fn to_bitvec(n: u64, len: usize) -> BitVec {
            let mut res = BitVec::repeat(false, len);
            for i in 0..64 {
                if (n & 1 << i) != 0 {
                    res.set(i, true);
                }
            }
            res
        }
        self.apply_bitvec_diff(
            item,
            &to_bitvec(from, item.bits.len()),
            &to_bitvec(to, item.bits.len()),
        );
    }

    pub fn apply_bit_diff(&mut self, item: &TileItem, from: bool, to: bool) {
        self.apply_bitvec_diff(item, &BitVec::from_iter([from]), &BitVec::from_iter([to]))
    }

    pub fn apply_enum_diff(&mut self, item: &TileItem, from: &str, to: &str) {
        let TileItemKind::Enum { ref values } = item.kind else {
            unreachable!()
        };
        let from = &values[from];
        let to = &values[to];
        for (idx, &bit) in item.bits.iter().enumerate() {
            if from[idx] != to[idx] {
                match self.bits.entry(bit) {
                    hash_map::Entry::Occupied(e) => {
                        assert_eq!(*e.get(), from[idx]);
                        e.remove();
                    }
                    hash_map::Entry::Vacant(e) => {
                        e.insert(to[idx]);
                    }
                }
            }
        }
    }

    pub fn split_tiles(&self, split: &[&[usize]]) -> Vec<Diff> {
        let mut res = vec![];
        let mut xlat = vec![];
        for (dstidx, &dst) in split.iter().enumerate() {
            res.push(Diff::default());
            for (tileidx, &srcidx) in dst.iter().enumerate() {
                while srcidx >= xlat.len() {
                    xlat.push(None);
                }
                assert!(xlat[srcidx].is_none());
                xlat[srcidx] = Some((dstidx, tileidx))
            }
        }
        for (&bit, &val) in &self.bits {
            let (dstidx, tileidx) = xlat[bit.tile].unwrap();
            let newbit = TileBit {
                tile: tileidx,
                ..bit
            };
            res[dstidx].bits.insert(newbit, val);
        }
        res
    }

    pub fn filter_tiles(&self, filter: &[usize]) -> Diff {
        let mut res = Diff::default();
        let mut xlat = vec![];
        for (tileidx, &srcidx) in filter.iter().enumerate() {
            while srcidx >= xlat.len() {
                xlat.push(None);
            }
            assert!(xlat[srcidx].is_none());
            xlat[srcidx] = Some(tileidx)
        }
        for (&bit, &val) in &self.bits {
            let Some(&Some(tileidx)) = xlat.get(bit.tile) else {
                continue;
            };
            let newbit = TileBit {
                tile: tileidx,
                ..bit
            };
            res.bits.insert(newbit, val);
        }
        res
    }

    pub fn from_bool_item(item: &TileItem) -> Self {
        assert_eq!(item.bits.len(), 1);
        let TileItemKind::BitVec { ref invert } = item.kind else {
            unreachable!()
        };
        let mut res = Diff::default();
        res.bits.insert(item.bits[0], !invert[0]);
        res
    }
}

impl core::ops::Not for Diff {
    type Output = Diff;

    fn not(self) -> Self::Output {
        Diff {
            bits: self.bits.into_iter().map(|(k, v)| (k, !v)).collect(),
        }
    }
}

impl core::ops::Not for &Diff {
    type Output = Diff;

    fn not(self) -> Self::Output {
        Diff {
            bits: self.bits.iter().map(|(&k, &v)| (k, !v)).collect(),
        }
    }
}

pub fn enum_ocd_swap_bits(item: &mut TileItem, a: usize, b: usize) {
    item.bits.swap(a, b);
    let TileItemKind::Enum { ref mut values } = item.kind else {
        unreachable!()
    };
    for val in values.values_mut() {
        val.swap(a, b);
    }
}

pub fn xlat_item_tile_fwd(item: TileItem, xlat: &[usize]) -> TileItem {
    TileItem {
        bits: item
            .bits
            .into_iter()
            .map(|bit| TileBit {
                tile: xlat[bit.tile],
                ..bit
            })
            .collect(),
        kind: item.kind,
    }
}

pub fn xlat_item_tile(item: TileItem, xlat: &[usize]) -> TileItem {
    let mut rxlat = vec![];
    for (idx, &tile) in xlat.iter().enumerate() {
        while tile >= rxlat.len() {
            rxlat.push(None);
        }
        assert!(rxlat[tile].is_none());
        rxlat[tile] = Some(idx);
    }
    TileItem {
        bits: item
            .bits
            .into_iter()
            .map(|bit| TileBit {
                tile: rxlat[bit.tile].unwrap(),
                ..bit
            })
            .collect(),
        kind: item.kind,
    }
}

pub fn xlat_bitvec(diffs: Vec<Diff>) -> TileItem {
    let mut invert = BitVec::new();
    let mut bits = vec![];
    for diff in diffs {
        assert_eq!(diff.bits.len(), 1);
        for (k, v) in diff.bits {
            bits.push(k);
            invert.push(!v);
        }
    }
    TileItem {
        bits,
        kind: TileItemKind::BitVec { invert },
    }
}

pub fn xlat_bit(diff: Diff) -> TileItem {
    xlat_bitvec(vec![diff])
}

pub fn xlat_bit_wide(diff: Diff) -> TileItem {
    let mut invert = BitVec::new();
    let mut bits = vec![];
    for (k, v) in diff.bits.into_iter().sorted() {
        bits.push(k);
        invert.push(!v);
    }
    assert!(invert.all() || !invert.any());
    TileItem {
        bits,
        kind: TileItemKind::BitVec { invert },
    }
}

pub fn concat_bitvec(vecs: impl IntoIterator<Item = TileItem>) -> TileItem {
    let mut invert = BitVec::new();
    let mut bits = vec![];
    for vec in vecs {
        let TileItemKind::BitVec { invert: cur_invert } = vec.kind else {
            unreachable!()
        };
        invert.extend(cur_invert);
        bits.extend(vec.bits);
    }
    TileItem {
        bits,
        kind: TileItemKind::BitVec { invert },
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum OcdMode<'a> {
    BitOrder,
    BitOrderDrpV6,
    ValueOrder,
    Mux,
    FixedOrder(&'a [TileBit]),
}

pub fn xlat_enum_ocd(diffs: Vec<(impl Into<String>, Diff)>, ocd: OcdMode) -> TileItem {
    let mut bits = BTreeMap::new();
    for (_, diff) in &diffs {
        for (&bit, &pol) in &diff.bits {
            match bits.entry(bit) {
                btree_map::Entry::Vacant(e) => {
                    e.insert(pol);
                }
                btree_map::Entry::Occupied(e) => {
                    assert_eq!(*e.get(), pol);
                }
            }
        }
    }
    let mut bits_vec: Vec<_> = bits.iter().map(|(&f, _)| f).collect();
    if let OcdMode::FixedOrder(fbits) = ocd {
        for bit in fbits {
            assert!(bits.contains_key(bit));
        }
        assert_eq!(bits.len(), fbits.len());
        bits_vec = fbits.to_vec();
    } else if ocd != OcdMode::BitOrder {
        bits_vec.sort_by(|a, b| {
            for (_, diff) in &diffs {
                let va = bits[a] ^ !diff.bits.contains_key(a);
                let vb = bits[b] ^ !diff.bits.contains_key(b);
                if va != vb {
                    return vb.cmp(&va);
                }
            }
            Ordering::Equal
        });
    }
    if ocd == OcdMode::BitOrderDrpV6 {
        bits_vec.sort_by(|a, b| {
            if a.tile != b.tile {
                a.tile.cmp(&b.tile)
            } else if a.bit != b.bit {
                a.bit.cmp(&b.bit)
            } else {
                a.frame.cmp(&b.frame)
            }
        })
    }
    if ocd == OcdMode::Mux {
        let tmp_values = Vec::from_iter(diffs.iter().map(|(_, v)| {
            Vec::from_iter(
                bits_vec
                    .iter()
                    .map(|&bit| bits[&bit] ^ !v.bits.contains_key(&bit)),
            )
        }));

        let mut en_bits = vec![];
        let mut onehot_groups = vec![];
        let mut taken = BTreeSet::new();

        for sbit in 0..bits_vec.len() {
            if taken.contains(&sbit) {
                continue;
            }
            let mut onehot_group = BTreeSet::new();
            onehot_group.insert(sbit);
            for nbit in (sbit + 1)..bits_vec.len() {
                if taken.contains(&nbit) {
                    continue;
                }
                let mut disjoint = true;
                for &cbit in &onehot_group {
                    for val in &tmp_values {
                        if val[nbit] && val[cbit] {
                            disjoint = false;
                            break;
                        }
                    }
                }
                if disjoint {
                    onehot_group.insert(nbit);
                }
            }
            let mut full = true;
            for val in &tmp_values {
                let mut cnt = 0;
                for &bit in &onehot_group {
                    if val[bit] {
                        cnt += 1;
                    }
                }
                assert!(cnt < 2);
                if cnt == 0 && val.iter().any(|&x| x) {
                    full = false;
                    break;
                }
            }
            if !full {
                continue;
            }
            for &bit in &onehot_group {
                taken.insert(bit);
            }
            if onehot_group.len() == 1 {
                // enable, not onehot_group.
                let bit = onehot_group.pop_first().unwrap();
                en_bits.push(bit);
            } else {
                onehot_groups.push(onehot_group);
            }
        }
        onehot_groups.sort_by_key(|group| std::cmp::Reverse(group.len()));
        let mut new_bits_vec = vec![];
        for bit in en_bits {
            new_bits_vec.push(bits_vec[bit]);
        }
        for group in onehot_groups {
            for bit in group {
                new_bits_vec.push(bits_vec[bit]);
            }
        }
        for bit in 0..bits_vec.len() {
            if !taken.contains(&bit) {
                new_bits_vec.push(bits_vec[bit]);
            }
        }
        bits_vec = new_bits_vec;
    }
    let mut values = BTreeMap::new();
    for (name, diff) in diffs {
        let name = name.into();
        let value: BitVec = bits_vec
            .iter()
            .map(|&bit| bits[&bit] ^ !diff.bits.contains_key(&bit))
            .collect();
        match values.entry(name) {
            btree_map::Entry::Vacant(e) => {
                e.insert(value);
            }
            btree_map::Entry::Occupied(e) => {
                if *e.get() != value {
                    eprintln!(
                        "MISMATCH FOR {n}: {cur:?} {value:?}",
                        n = e.key(),
                        cur = e.get(),
                    );
                    panic!("OOPS");
                }
            }
        }
    }
    TileItem {
        bits: bits_vec,
        kind: TileItemKind::Enum { values },
    }
}

pub fn xlat_enum(diffs: Vec<(impl Into<String>, Diff)>) -> TileItem {
    xlat_enum_ocd(diffs, OcdMode::ValueOrder)
}

pub fn xlat_enum_default(mut diffs: Vec<(String, Diff)>, default: impl Into<String>) -> TileItem {
    diffs.insert(0, (default.into(), Diff::default()));
    xlat_enum(diffs)
}

pub fn xlat_enum_default_ocd(
    mut diffs: Vec<(String, Diff)>,
    default: impl Into<String>,
    ocd: OcdMode,
) -> TileItem {
    diffs.insert(0, (default.into(), Diff::default()));
    xlat_enum_ocd(diffs, ocd)
}

pub fn xlat_enum_int(diffs: Vec<(u32, Diff)>) -> TileItem {
    let mut bits: Vec<Option<TileBit>> = vec![];
    let mut xor = 0;
    for (val, diff) in &diffs {
        if diff.bits.is_empty() {
            xor = *val;
        }
    }
    loop {
        let mut progress = false;
        let mut done = true;
        for &(mut val, ref diff) in &diffs {
            let mut mdiff = diff.clone();
            val ^= xor;
            for (i, &bit) in bits.iter().enumerate() {
                let Some(bit) = bit else { continue };
                if (val & 1 << i) != 0 {
                    val &= !(1 << i);
                    assert_eq!(mdiff.bits.remove(&bit), Some((xor >> i & 1) == 0));
                }
            }
            if val == 0 {
                mdiff.assert_empty();
            } else if val.is_power_of_two() {
                let bidx: usize = val.ilog2().try_into().unwrap();
                while bits.len() <= bidx {
                    bits.push(None);
                }
                assert_eq!(mdiff.bits.len(), 1);
                let (bit, pol) = mdiff.bits.into_iter().next().unwrap();
                bits[bidx] = Some(bit);
                assert_eq!(pol, (xor >> bidx & 1) == 0);
                progress = true;
            } else {
                done = false;
            }
        }
        if done {
            return TileItem {
                bits: bits.iter().map(|bit| bit.unwrap()).collect(),
                kind: TileItemKind::BitVec {
                    invert: BitVec::repeat(false, bits.len()),
                },
            };
        }
        if !progress {
            panic!("NO PROGRESS: {bits:?} {diffs:?}")
        }
    }
}

pub fn xlat_bool_default(diff0: Diff, diff1: Diff) -> (TileItem, bool) {
    let (diff, res) = if diff0.bits.is_empty() {
        diff0.assert_empty();
        (diff1, false)
    } else {
        diff1.assert_empty();
        (!diff0, true)
    };
    (xlat_bit(diff), res)
}

pub fn xlat_bool(diff0: Diff, diff1: Diff) -> TileItem {
    xlat_bool_default(diff0, diff1).0
}

#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct FeatureId {
    pub tile: String,
    pub bel: String,
    pub attr: String,
    pub val: String,
}

impl std::fmt::Debug for FeatureId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}:{}", self.tile, self.bel, self.attr, self.val)
    }
}

#[derive(Clone, Debug)]
pub struct FeatureData {
    pub diffs: Vec<Diff>,
    pub fuzzers: Vec<FuzzerId>,
}

#[derive(Debug, Default)]
pub struct State {
    pub features: BTreeMap<FeatureId, FeatureData>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_diffs(
        &mut self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> Vec<Diff> {
        let tile = tile.into();
        let bel = bel.into();
        let attr = attr.into();
        let val = val.into();
        let id = FeatureId {
            tile,
            bel,
            attr,
            val,
        };
        self.features
            .remove(&id)
            .unwrap_or_else(|| {
                panic!(
                    "NO DIFF: {tile} {bel} {attr} {val}",
                    tile = id.tile,
                    bel = id.bel,
                    attr = id.attr,
                    val = id.val
                )
            })
            .diffs
    }

    pub fn get_diff(
        &mut self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> Diff {
        let mut res = self.get_diffs(tile, bel, attr, val);
        assert_eq!(res.len(), 1);
        res.pop().unwrap()
    }

    pub fn peek_diffs(
        &self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> &Vec<Diff> {
        let tile = tile.into();
        let bel = bel.into();
        let attr = attr.into();
        let val = val.into();
        let id = FeatureId {
            tile,
            bel,
            attr,
            val,
        };
        &self
            .features
            .get(&id)
            .unwrap_or_else(|| {
                panic!(
                    "NO DIFF: {tile} {bel} {attr} {val}",
                    tile = id.tile,
                    bel = id.bel,
                    attr = id.attr,
                    val = id.val
                )
            })
            .diffs
    }

    pub fn peek_diff(
        &self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> &Diff {
        let res = self.peek_diffs(tile, bel, attr, val);
        assert_eq!(res.len(), 1);
        &res[0]
    }

    pub fn return_fuzzer<P: Copy + Debug, T: BitTile<BitPos = P>>(
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
                    for (i, t) in feat.tiles.iter().enumerate() {
                        if let Some(xk) = t.xlat_pos_rev(k) {
                            fdiffs[fidx][bitidx].bits.insert(
                                TileBit {
                                    tile: i,
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
                    println!("{:?}: {:?}", feat.id, feat.tiles);
                }
                return Some(vec![]);
            }
        }
        for (feat, xdiffs) in f.features.iter().zip(fdiffs) {
            // if self.debug >= 3 {
            //     eprintln!("RETURN {feat:?} {xdiffs:?}");
            // }
            if feat.tiles.is_empty() {
                for diff in &xdiffs {
                    if !diff.bits.is_empty() {
                        eprintln!("null fuzzer {f:?} with bits: {xdiffs:?}");
                        return Some(vec![]);
                    }
                }
            } else {
                match self.features.entry(feat.id.clone()) {
                    btree_map::Entry::Occupied(mut e) => {
                        let v = e.get_mut();
                        if v.diffs != xdiffs {
                            eprintln!(
                                "bits mismatch for {f:?}/{fid:?}: {vbits:?} vs {xdiffs:?}",
                                fid = feat.id,
                                vbits = v.diffs
                            );
                            return Some(v.fuzzers.clone());
                        } else {
                            v.fuzzers.push(fid);
                        }
                    }
                    btree_map::Entry::Vacant(e) => {
                        e.insert(FeatureData {
                            diffs: xdiffs,
                            fuzzers: vec![fid],
                        });
                    }
                }
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct Collector<'a> {
    pub state: &'a mut State,
    pub tiledb: &'a mut BsData,
}

impl Collector<'_> {
    #[must_use]
    pub fn extract_bitvec(&mut self, tile: &str, bel: &str, attr: &str, val: &str) -> TileItem {
        xlat_bitvec(self.state.get_diffs(tile, bel, attr, val))
    }

    pub fn collect_bitvec(&mut self, tile: &str, bel: &str, attr: &str, val: &str) {
        self.tiledb.insert(
            tile,
            bel,
            attr,
            xlat_bitvec(self.state.get_diffs(tile, bel, attr, val)),
        );
    }

    #[must_use]
    pub fn extract_enum(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
    ) -> TileItem {
        let diffs = vals
            .iter()
            .map(|val| {
                (
                    val.as_ref().to_string(),
                    self.state.get_diff(tile, bel, attr, val.as_ref()),
                )
            })
            .collect();
        xlat_enum(diffs)
    }

    #[must_use]
    pub fn extract_enum_ocd(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
        ocd: OcdMode,
    ) -> TileItem {
        let diffs = vals
            .iter()
            .map(|val| {
                (
                    val.as_ref().to_string(),
                    self.state.get_diff(tile, bel, attr, val.as_ref()),
                )
            })
            .collect();
        xlat_enum_ocd(diffs, ocd)
    }

    #[must_use]
    pub fn extract_enum_int(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: Range<u32>,
        delta: u32,
    ) -> TileItem {
        let diffs = vals
            .map(|val| {
                (
                    val,
                    self.state
                        .get_diff(tile, bel, attr, format!("{v}", v = val + delta)),
                )
            })
            .collect();
        xlat_enum_int(diffs)
    }

    pub fn collect_enum(&mut self, tile: &str, bel: &str, attr: &str, vals: &[impl AsRef<str>]) {
        let item = self.extract_enum(tile, bel, attr, vals);
        self.tiledb.insert(tile, bel, attr, item);
    }

    pub fn collect_enum_ocd(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
        ocd: OcdMode,
    ) {
        let item = self.extract_enum_ocd(tile, bel, attr, vals, ocd);
        self.tiledb.insert(tile, bel, attr, item);
    }

    pub fn collect_enum_int(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: Range<u32>,
        delta: u32,
    ) {
        let item = self.extract_enum_int(tile, bel, attr, vals, delta);
        self.tiledb.insert(tile, bel, attr, item);
    }

    #[must_use]
    pub fn extract_bit(&mut self, tile: &str, bel: &str, attr: &str, val: &str) -> TileItem {
        let diff = self.state.get_diff(tile, bel, attr, val);
        xlat_bit(diff)
    }

    #[must_use]
    pub fn extract_bit_wide(&mut self, tile: &str, bel: &str, attr: &str, val: &str) -> TileItem {
        let diff = self.state.get_diff(tile, bel, attr, val);
        xlat_bit_wide(diff)
    }

    pub fn collect_bit(&mut self, tile: &str, bel: &str, attr: &str, val: &str) {
        let item = self.extract_bit(tile, bel, attr, val);
        self.tiledb.insert(tile, bel, attr, item);
    }

    pub fn collect_bit_wide(&mut self, tile: &str, bel: &str, attr: &str, val: &str) {
        let item = self.extract_bit_wide(tile, bel, attr, val);
        self.tiledb.insert(tile, bel, attr, item);
    }

    #[must_use]
    pub fn extract_enum_default(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
        default: &str,
    ) -> TileItem {
        let diffs = vals
            .iter()
            .map(|val| {
                (
                    val.as_ref().to_string(),
                    self.state.get_diff(tile, bel, attr, val.as_ref()),
                )
            })
            .collect();
        xlat_enum_default(diffs, default)
    }

    #[must_use]
    pub fn extract_enum_default_ocd(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
        default: &str,
        ocd: OcdMode,
    ) -> TileItem {
        let diffs = vals
            .iter()
            .map(|val| {
                (
                    val.as_ref().to_string(),
                    self.state.get_diff(tile, bel, attr, val.as_ref()),
                )
            })
            .collect();
        xlat_enum_default_ocd(diffs, default, ocd)
    }

    pub fn collect_enum_default(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
        default: &str,
    ) {
        let item = self.extract_enum_default(tile, bel, attr, vals, default);
        self.tiledb.insert(tile, bel, attr, item);
    }

    pub fn collect_enum_default_ocd(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
        default: &str,
        ocd: OcdMode,
    ) {
        let item = self.extract_enum_default_ocd(tile, bel, attr, vals, default, ocd);
        self.tiledb.insert(tile, bel, attr, item);
    }

    #[must_use]
    pub fn extract_enum_bool_default(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) -> (TileItem, bool) {
        let d0 = self.state.get_diff(tile, bel, attr, val0);
        let d1 = self.state.get_diff(tile, bel, attr, val1);
        xlat_bool_default(d0, d1)
    }

    #[must_use]
    pub fn extract_enum_bool(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) -> TileItem {
        let d0 = self.state.get_diff(tile, bel, attr, val0);
        let d1 = self.state.get_diff(tile, bel, attr, val1);
        xlat_bool(d0, d1)
    }

    pub fn collect_enum_bool_default(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) -> bool {
        let (item, res) = self.extract_enum_bool_default(tile, bel, attr, val0, val1);
        self.tiledb.insert(tile, bel, attr, item);
        res
    }

    pub fn collect_enum_bool(&mut self, tile: &str, bel: &str, attr: &str, val0: &str, val1: &str) {
        let item = self.extract_enum_bool(tile, bel, attr, val0, val1);
        self.tiledb.insert(tile, bel, attr, item);
    }

    #[must_use]
    pub fn extract_enum_bool_wide(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) -> TileItem {
        let d0 = self.state.get_diff(tile, bel, attr, val0);
        let d1 = self.state.get_diff(tile, bel, attr, val1);
        let item = xlat_enum(vec![("0", d0), ("1", d1)]);
        let TileItemKind::Enum { values } = item.kind else {
            unreachable!()
        };
        let v0 = &values["0"];
        let v1 = &values["1"];
        let invert = if v1.all() && !v0.any() {
            false
        } else if v0.all() && !v1.any() {
            true
        } else {
            panic!("not a bool: {tile} {bel} {attr} {values:?}");
        };
        let invert = BitVec::from_iter(vec![invert; item.bits.len()]);
        TileItem {
            bits: item.bits,
            kind: TileItemKind::BitVec { invert },
        }
    }

    #[must_use]
    pub fn extract_enum_bool_wide_mixed(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) -> TileItem {
        let d0 = self.state.get_diff(tile, bel, attr, val0);
        let d1 = self.state.get_diff(tile, bel, attr, val1);
        let item = xlat_enum(vec![("0", d0), ("1", d1)]);
        let TileItemKind::Enum { values } = item.kind else {
            unreachable!()
        };
        let v0 = &values["0"];
        let v1 = &values["1"];
        for (b0, b1) in v0.iter().zip(v1) {
            assert_eq!(b0, !b1);
        }
        let invert = v0.clone();
        TileItem {
            bits: item.bits,
            kind: TileItemKind::BitVec { invert },
        }
    }

    pub fn collect_enum_bool_wide(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) {
        let item = self.extract_enum_bool_wide(tile, bel, attr, val0, val1);

        self.tiledb.insert(tile, bel, attr, item);
    }

    pub fn collect_enum_bool_wide_mixed(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) {
        let item = self.extract_enum_bool_wide_mixed(tile, bel, attr, val0, val1);

        self.tiledb.insert(tile, bel, attr, item);
    }
}

pub fn extract_bitvec_val_part(item: &TileItem, base: &BitVec, diff: &mut Diff) -> BitVec {
    let TileItemKind::BitVec { ref invert } = item.kind else {
        unreachable!()
    };
    assert_eq!(item.bits.len(), base.len());
    let mut res = base.clone();
    let rev: HashMap<_, _> = item
        .bits
        .iter()
        .copied()
        .enumerate()
        .map(|(i, v)| (v, i))
        .collect();
    diff.bits.retain(|&bit, &mut val| {
        if let Some(&bitidx) = rev.get(&bit) {
            assert_eq!(res[bitidx], !(val ^ invert[bitidx]));
            res.set(bitidx, val ^ invert[bitidx]);
            false
        } else {
            true
        }
    });
    res
}

pub fn extract_bitvec_val(item: &TileItem, base: &BitVec, diff: Diff) -> BitVec {
    let TileItemKind::BitVec { ref invert } = item.kind else {
        unreachable!()
    };
    assert_eq!(item.bits.len(), base.len());
    let mut res = base.clone();
    let rev: HashMap<_, _> = item
        .bits
        .iter()
        .copied()
        .enumerate()
        .map(|(i, v)| (v, i))
        .collect();
    for (&bit, &val) in diff.bits.iter() {
        let bitidx = rev[&bit];
        assert_eq!(res[bitidx], !(val ^ invert[bitidx]));
        res.set(bitidx, val ^ invert[bitidx]);
    }
    res
}
