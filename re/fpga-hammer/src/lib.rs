use core::fmt::Debug;
use core::hash::Hash;
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, btree_map, hash_map},
    ops::Range,
};

use itertools::Itertools;
use prjcombine_entity::{
    EntityId, EntityPartVec, EntityVec,
    id::{EntityIdU16, EntityTag},
};
use prjcombine_interconnect::{
    db::{
        BelAttribute, BelAttributeEnum, BelAttributeId, BelAttributeType, BelInfo, BelKind,
        BelSlotId, EnumValueId, IntDb, PolTileWireCoord, SwitchBoxItem, TableFieldId, TableId,
        TableRowId, TableValue, TileClassId, TileSlotId, TileWireCoord,
    },
    grid::{
        BelCoord, CellCoord, ColId, DieId, ExpandedGrid, PolWireCoord, RowId, TileCoord, WireCoord,
    },
};
use prjcombine_re_hammer::{Backend, BatchValue, Fuzzer, FuzzerGen, FuzzerId};
use prjcombine_types::bsdata::{BitRectId, BsData, PolTileBit, TileBit, TileItem, TileItemKind};
use prjcombine_types::{bitrect::BitRect, bitvec::BitVec};
use rand::seq::IndexedRandom;

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
    fn try_generate(
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

    pub fn apply_bitvec_diff_raw(&mut self, bits: &[PolTileBit], from: &BitVec, to: &BitVec) {
        for (idx, &pbit) in bits.iter().enumerate() {
            if from[idx] != to[idx] {
                match self.bits.entry(pbit.bit) {
                    hash_map::Entry::Occupied(e) => {
                        assert_eq!(*e.get(), from[idx] ^ pbit.inv);
                        e.remove();
                    }
                    hash_map::Entry::Vacant(e) => {
                        e.insert(to[idx] ^ pbit.inv);
                    }
                }
            }
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

    pub fn split_rects(&self, split: &[&EntityVec<BitRectId, BitRectId>]) -> Vec<Diff> {
        let mut res = vec![];
        let mut xlat = EntityPartVec::new();
        for (dst_idx, &dst) in split.iter().enumerate() {
            res.push(Diff::default());
            for (dst_rect, &src_rect) in dst {
                assert!(!xlat.contains_id(src_rect));
                xlat.insert(src_rect, (dst_idx, dst_rect));
            }
        }
        for (&bit, &val) in &self.bits {
            let (dst_idx, dst_rect) = xlat[bit.rect];
            let newbit = TileBit {
                rect: dst_rect,
                ..bit
            };
            res[dst_idx].bits.insert(newbit, val);
        }
        res
    }

    pub fn filter_rects(&self, filter: &EntityVec<BitRectId, BitRectId>) -> Diff {
        let mut res = Diff::default();
        let mut xlat = EntityPartVec::new();
        for (dst_rect, &src_rect) in filter {
            assert!(!xlat.contains_id(src_rect));
            xlat.insert(src_rect, dst_rect);
        }
        for (&bit, &val) in &self.bits {
            let Some(&dst_rect) = xlat.get(bit.rect) else {
                continue;
            };
            let newbit = TileBit {
                rect: dst_rect,
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

pub fn xlat_item_tile_fwd(item: TileItem, xlat: &EntityVec<BitRectId, BitRectId>) -> TileItem {
    TileItem {
        bits: item
            .bits
            .into_iter()
            .map(|bit| TileBit {
                rect: xlat[bit.rect],
                ..bit
            })
            .collect(),
        kind: item.kind,
    }
}

pub fn xlat_item_tile(item: TileItem, xlat: &EntityVec<BitRectId, BitRectId>) -> TileItem {
    let mut rxlat = EntityPartVec::new();
    for (dst_rect, &src_rect) in xlat {
        assert!(!rxlat.contains_id(src_rect));
        rxlat.insert(src_rect, dst_rect);
    }
    TileItem {
        bits: item
            .bits
            .into_iter()
            .map(|bit| TileBit {
                rect: rxlat[bit.rect],
                ..bit
            })
            .collect(),
        kind: item.kind,
    }
}

pub fn xlat_bit_raw(diff: Diff) -> PolTileBit {
    assert_eq!(diff.bits.len(), 1);
    let (&bit, &val) = diff.bits.iter().next().unwrap();
    PolTileBit { bit, inv: !val }
}

pub fn xlat_bitvec_raw(diffs: Vec<Diff>) -> Vec<PolTileBit> {
    diffs.into_iter().map(xlat_bit_raw).collect()
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

pub fn xlat_enum_raw<K: Clone + Debug + Eq + PartialEq + Ord + PartialOrd>(
    diffs: Vec<(K, Diff)>,
    ocd: OcdMode,
) -> EnumData<K> {
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
            if a.rect != b.rect {
                a.rect.cmp(&b.rect)
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
                        "MISMATCH FOR {n:?}: {cur:?} {value:?}",
                        n = e.key(),
                        cur = e.get(),
                    );
                    panic!("OOPS");
                }
            }
        }
    }
    (bits_vec, values)
}

pub fn xlat_enum_ocd(diffs: Vec<(impl Into<String>, Diff)>, ocd: OcdMode) -> TileItem {
    let (bits, values) = xlat_enum_raw(
        diffs
            .into_iter()
            .map(|(key, diff)| (key.into(), diff))
            .collect(),
        ocd,
    );
    TileItem {
        bits,
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

pub struct SpecialTag;
impl EntityTag for SpecialTag {
    const PREFIX: &'static str = "SPEC";
}

pub type SpecialId = EntityIdU16<SpecialTag>;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum DiffKey {
    Legacy(FeatureId),
    BelAttrBit(TileClassId, BelSlotId, BelAttributeId, usize),
    BelAttrValue(TileClassId, BelSlotId, BelAttributeId, EnumValueId),
    BelAttrSpecial(TileClassId, BelSlotId, BelAttributeId, SpecialId),
    BelSpecial(TileClassId, BelSlotId, SpecialId),
    BelSpecialString(TileClassId, BelSlotId, SpecialId, String),
    GlobalBelAttrBit(BelCoord, BelAttributeId, usize),
    GlobalBelAttrSpecial(BelCoord, BelAttributeId, SpecialId),
    GlobalRouting(WireCoord, PolWireCoord),
    Routing(TileClassId, TileWireCoord, PolTileWireCoord),
    RoutingInv(TileClassId, TileWireCoord),
    RoutingSpecial(TileClassId, TileWireCoord, SpecialId),
    RoutingPairSpecial(TileClassId, TileWireCoord, PolTileWireCoord, SpecialId),
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

    pub fn get_diffs_raw(&mut self, key: &DiffKey) -> Vec<Diff> {
        self.features
            .remove(key)
            .unwrap_or_else(|| panic!("NO DIFF: {key:?}"))
            .diffs
    }

    pub fn get_diff_raw(&mut self, key: &DiffKey) -> Diff {
        let mut res = self.get_diffs_raw(key);
        assert_eq!(res.len(), 1);
        res.pop().unwrap()
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
        let key = DiffKey::Legacy(FeatureId {
            tile,
            bel,
            attr,
            val,
        });
        self.get_diffs_raw(&key)
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
        let key = DiffKey::Legacy(FeatureId {
            tile,
            bel,
            attr,
            val,
        });
        &self
            .features
            .get(&key)
            .unwrap_or_else(|| panic!("NO DIFF: {key:?}"))
            .diffs
    }

    pub fn peek_diffs_raw(&self, key: &DiffKey) -> &Vec<Diff> {
        &self
            .features
            .get(key)
            .unwrap_or_else(|| panic!("NO DIFF: {key:?}"))
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

    pub fn peek_diff_raw(&self, key: &DiffKey) -> &Diff {
        let res = self.peek_diffs_raw(key);
        assert_eq!(res.len(), 1);
        &res[0]
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
                match self.features.entry(feat.key.clone()) {
                    btree_map::Entry::Occupied(mut e) => {
                        let v = e.get_mut();
                        if v.diffs != xdiffs {
                            eprintln!(
                                "bits mismatch for {f:?}/{fid:?}: {vbits:?} vs {xdiffs:?}",
                                fid = feat.key,
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

pub type EnumData<K> = (Vec<TileBit>, BTreeMap<K, BitVec>);

#[derive(Debug, Default)]
pub struct CollectorData {
    pub bel_attrs: HashMap<(TileClassId, BelSlotId, BelAttributeId), BelAttribute>,
    pub inv: HashMap<(TileClassId, TileWireCoord), PolTileBit>,
    pub buf: HashMap<(TileClassId, TileWireCoord, PolTileWireCoord), PolTileBit>,
    pub mux: HashMap<(TileClassId, TileWireCoord), EnumData<Option<PolTileWireCoord>>>,
    pub table_data: HashMap<(TableId, TableRowId, TableFieldId), TableValue>,
}

impl CollectorData {
    pub fn insert_into(mut self, intdb: &mut IntDb, missing_ok: bool) {
        for ((tcid, bslot, aid), attr) in self.bel_attrs {
            let BelInfo::Bel(ref mut bel) = intdb.tile_classes[tcid].bels[bslot] else {
                unreachable!()
            };
            if bel.attributes.contains_id(aid) {
                assert_eq!(bel.attributes[aid], attr);
            } else {
                bel.attributes.insert(aid, attr);
            }
        }

        for (tcid, _, tcls) in &mut intdb.tile_classes {
            for bel in tcls.bels.values_mut() {
                let BelInfo::SwitchBox(sbox) = bel else {
                    continue;
                };
                for item in &mut sbox.items {
                    match item {
                        SwitchBoxItem::Mux(mux) => {
                            let Some((bits, values)) = self.mux.remove(&(tcid, mux.dst)) else {
                                if missing_ok {
                                    continue;
                                }
                                let dst = mux.dst;
                                panic!(
                                    "can't find collect enum mux {tcname} {dst}",
                                    tcname = intdb.tile_classes.key(tcid),
                                    dst = dst.to_string(intdb, &intdb.tile_classes[tcid]),
                                )
                            };
                            mux.bits = bits;
                            let mut handled = BTreeSet::new();
                            for (src, val) in values {
                                if let Some(src) = src {
                                    *mux.src.get_mut(&src).unwrap() = val;
                                    handled.insert(src);
                                } else {
                                    mux.bits_off = Some(val);
                                }
                            }
                            for src in mux.src.keys() {
                                let src = *src;
                                if !handled.contains(&src) {
                                    let dst = mux.dst;
                                    panic!(
                                        "can't find mux input {tcname} {dst} {src}",
                                        tcname = intdb.tile_classes.key(tcid),
                                        dst = dst.to_string(intdb, &intdb.tile_classes[tcid]),
                                        src = src.to_string(intdb, &intdb.tile_classes[tcid]),
                                    );
                                }
                            }
                        }
                        SwitchBoxItem::ProgBuf(buf) => {
                            let Some(bit) = self.buf.remove(&(tcid, buf.dst, buf.src)) else {
                                if missing_ok {
                                    continue;
                                }
                                let dst = buf.dst;
                                let src = buf.src;
                                panic!(
                                    "can't find collect bit progbuf {tcname} {dst} {src}",
                                    tcname = intdb.tile_classes.key(tcid),
                                    dst = dst.to_string(intdb, &intdb.tile_classes[tcid]),
                                    src = src.to_string(intdb, &intdb.tile_classes[tcid])
                                )
                            };
                            buf.bit = bit;
                        }
                        SwitchBoxItem::PermaBuf(_) => (),
                        SwitchBoxItem::Pass(_pass) => {
                            // TODO
                        }
                        SwitchBoxItem::BiPass(_pass) => {
                            // TODO
                        }
                        SwitchBoxItem::ProgInv(inv) => {
                            let Some(bit) = self.inv.remove(&(tcid, inv.dst)) else {
                                if missing_ok {
                                    continue;
                                }
                                let twc = inv.dst;
                                panic!(
                                    "can't find collect bit proginv {tcname} {wire}",
                                    tcname = intdb.tile_classes.key(tcid),
                                    wire = twc.to_string(intdb, &intdb.tile_classes[tcid])
                                )
                            };
                            inv.bit = bit;
                        }
                        SwitchBoxItem::ProgDelay(_delay) => {
                            // TODO
                        }
                    }
                }
            }
        }

        for ((tcid, dst), data) in self.mux {
            println!(
                "uncollected enum: mux {tcls} {dst}: {data:?}",
                tcls = intdb.tile_classes.key(tcid),
                dst = dst.to_string(intdb, &intdb.tile_classes[tcid]),
            );
        }

        for ((tcid, dst, src), bit) in self.buf {
            println!(
                "uncollected bit: progbuf {tcls} {dst} {src}: {bit}",
                tcls = intdb.tile_classes.key(tcid),
                dst = dst.to_string(intdb, &intdb.tile_classes[tcid]),
                src = src.to_string(intdb, &intdb.tile_classes[tcid]),
                bit = intdb.tile_classes[tcid].dump_polbit(bit),
            );
        }

        for ((tcid, twc), bit) in self.inv {
            println!(
                "uncollected bit: proginv {tcls} {wire}: {bit}",
                tcls = intdb.tile_classes.key(tcid),
                wire = twc.to_string(intdb, &intdb.tile_classes[tcid]),
                bit = intdb.tile_classes[tcid].dump_polbit(bit),
            );
        }

        for ((tid, rid, fid), value) in self.table_data {
            let row = &mut intdb.tables[tid].rows[rid];
            if row.contains_id(fid) {
                assert_eq!(row[fid], value);
            } else {
                row.insert(fid, value);
            }
        }
    }
}

#[derive(Debug)]
pub struct Collector<'a, 'b> {
    pub state: &'a mut State,
    pub tiledb: &'a mut BsData,
    pub intdb: &'b IntDb,
    pub data: CollectorData,
}

impl<'a, 'b> Collector<'a, 'b> {
    pub fn new(state: &'a mut State, tiledb: &'a mut BsData, intdb: &'b IntDb) -> Self {
        Self {
            state,
            tiledb,
            intdb,
            data: Default::default(),
        }
    }

    pub fn insert_bel_attr_raw(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
        attr: BelAttribute,
    ) {
        match self.data.bel_attrs.entry((tcid, bslot, aid)) {
            hash_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), attr);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(attr);
            }
        }
    }

    pub fn insert_bel_attr_bool(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
        bit: PolTileBit,
    ) {
        self.insert_bel_attr_raw(tcid, bslot, aid, BelAttribute::BitVec(vec![bit]));
    }

    pub fn insert_bel_attr_bitvec(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
        bits: Vec<PolTileBit>,
    ) {
        self.insert_bel_attr_raw(tcid, bslot, aid, BelAttribute::BitVec(bits));
    }

    pub fn collect_bel_attr(&mut self, tcid: TileClassId, bslot: BelSlotId, aid: BelAttributeId) {
        let BelKind::Class(bcid) = self.intdb.bel_slots[bslot].kind else {
            unreachable!()
        };
        let bcattr = &self.intdb.bel_classes[bcid].attributes[aid];
        let attr = match bcattr.typ {
            BelAttributeType::Enum(ecid) => {
                let ecls = &self.intdb.enum_classes[ecid];
                let mut diffs = vec![];
                for vid in ecls.values.ids() {
                    diffs.push((
                        vid,
                        self.state
                            .get_diff_raw(&DiffKey::BelAttrValue(tcid, bslot, aid, vid)),
                    ));
                }
                let (bits, values) = xlat_enum_raw(diffs, OcdMode::ValueOrder);
                BelAttribute::Enum(BelAttributeEnum {
                    bits,
                    values: values.into_iter().collect(),
                })
            }
            BelAttributeType::Bool => BelAttribute::BitVec(vec![xlat_bit_raw(
                self.state
                    .get_diff_raw(&DiffKey::BelAttrBit(tcid, bslot, aid, 0)),
            )]),
            BelAttributeType::Bitvec(width) => BelAttribute::BitVec(xlat_bitvec_raw(
                (0..width)
                    .map(|idx| {
                        self.state
                            .get_diff_raw(&DiffKey::BelAttrBit(tcid, bslot, aid, idx))
                    })
                    .collect(),
            )),
            BelAttributeType::BitvecArray(_, _) => todo!(),
        };
        match self.data.bel_attrs.entry((tcid, bslot, aid)) {
            hash_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), attr);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(attr);
            }
        }
    }

    pub fn collect_bel_attr_default(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
        default: EnumValueId,
    ) {
        let BelKind::Class(bcid) = self.intdb.bel_slots[bslot].kind else {
            unreachable!()
        };
        let bcattr = &self.intdb.bel_classes[bcid].attributes[aid];
        let BelAttributeType::Enum(ecid) = bcattr.typ else {
            unreachable!()
        };

        let ecls = &self.intdb.enum_classes[ecid];
        let mut diffs = vec![(default, Diff::default())];
        for vid in ecls.values.ids() {
            if vid == default {
                continue;
            }
            diffs.push((
                vid,
                self.state
                    .get_diff_raw(&DiffKey::BelAttrValue(tcid, bslot, aid, vid)),
            ));
        }

        let (bits, values) = xlat_enum_raw(diffs, OcdMode::ValueOrder);
        let attr = BelAttribute::Enum(BelAttributeEnum {
            bits,
            values: values.into_iter().collect(),
        });
        match self.data.bel_attrs.entry((tcid, bslot, aid)) {
            hash_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), attr);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(attr);
            }
        }
    }

    pub fn bel_attr_bitvec(
        &self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
    ) -> &[PolTileBit] {
        let BelAttribute::BitVec(ref bits) = self.data.bel_attrs[&(tcid, bslot, aid)] else {
            unreachable!()
        };
        bits
    }

    pub fn collect_progbuf(
        &mut self,
        tcid: TileClassId,
        dst: TileWireCoord,
        src: PolTileWireCoord,
    ) {
        let diff = self.state.get_diff_raw(&DiffKey::Routing(tcid, dst, src));
        let bit = xlat_bit_raw(diff);
        match self.data.buf.entry((tcid, dst, src)) {
            hash_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), bit);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(bit);
            }
        }
    }

    pub fn collect_inv(&mut self, tcid: TileClassId, wire: TileWireCoord) {
        let diff = self.state.get_diff_raw(&DiffKey::RoutingInv(tcid, wire));
        let bit = xlat_bit_raw(diff);
        match self.data.inv.entry((tcid, wire)) {
            hash_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), bit);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(bit);
            }
        }
    }

    pub fn insert_table_bitvec(
        &mut self,
        tid: TableId,
        rid: TableRowId,
        fid: TableFieldId,
        val: BitVec,
    ) {
        let val = TableValue::BitVec(val);
        match self.data.table_data.entry((tid, rid, fid)) {
            hash_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), val);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(val);
            }
        }
    }

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

pub fn extract_bitvec_val_part_raw(bits: &[PolTileBit], base: &BitVec, diff: &mut Diff) -> BitVec {
    assert_eq!(bits.len(), base.len());
    let mut res = base.clone();
    let rev: HashMap<_, _> = bits
        .iter()
        .copied()
        .enumerate()
        .map(|(i, v)| (v.bit, i))
        .collect();
    diff.bits.retain(|&bit, &mut val| {
        if let Some(&bitidx) = rev.get(&bit) {
            assert_eq!(res[bitidx], !(val ^ bits[bitidx].inv));
            res.set(bitidx, val ^ bits[bitidx].inv);
            false
        } else {
            true
        }
    });
    res
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

#[macro_export]
macro_rules! specials {
    (@CONSTS@ ($idx:expr) $spec:ident, $($rest:ident,)*) => {
        pub const $spec: $crate::SpecialId = $crate::SpecialId::from_idx_const($idx);
        $crate::specials!(@CONSTS@ ($spec.to_idx_const() + 1) $($rest,)*);
    };
    (@CONSTS@ ($idx:expr)) => {};
    ($($spec:ident),* $(,)?) => {
        $crate::specials!(@CONSTS@ (0) $($spec,)*);
    };
}
