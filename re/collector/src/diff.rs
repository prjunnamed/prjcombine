use core::fmt::Debug;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, btree_map, hash_map};

use bincode::{Decode, Encode};
use itertools::Itertools;
use prjcombine_entity::{
    EntityPartVec, EntityVec,
    id::{EntityIdU16, EntityTag},
};
use prjcombine_interconnect::{
    db::{
        BelAttribute, BelAttributeEnum, BelAttributeId, BelInputId, BelSlotId, ConnectorSlotId,
        EnumValueId, PolTileWireCoord, TileClassId, TileWireCoord,
    },
    grid::{BelCoord, PolWireCoord, WireCoord},
};
use prjcombine_types::{
    bitvec::BitVec,
    bsdata::{BitRectId, PolTileBit, TileBit, TileItem, TileItemKind},
};

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub struct EnumData<K: Ord> {
    pub bits: Vec<TileBit>,
    pub values: BTreeMap<K, BitVec>,
}

pub struct SpecialTag;
impl EntityTag for SpecialTag {
    const PREFIX: &'static str = "SPEC";
}

pub type SpecialId = EntityIdU16<SpecialTag>;

#[macro_export]
macro_rules! specials {
    (@CONSTS@ ($idx:expr) $spec:ident, $($rest:ident,)*) => {
        pub const $spec: $crate::diff::SpecialId = $crate::diff::SpecialId::from_idx_const($idx);
        $crate::specials!(@CONSTS@ ($spec.to_idx_const() + 1) $($rest,)*);
    };
    (@CONSTS@ ($idx:expr)) => {};
    ($($spec:ident),* $(,)?) => {
        $crate::specials!(@CONSTS@ (0) $($spec,)*);
    };
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

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum DiffKey {
    Legacy(FeatureId),
    BelAttrBit(TileClassId, BelSlotId, BelAttributeId, usize),
    BelAttrValue(TileClassId, BelSlotId, BelAttributeId, EnumValueId),
    BelAttrValueSpecial(
        TileClassId,
        BelSlotId,
        BelAttributeId,
        EnumValueId,
        SpecialId,
    ),
    BelAttrEnumBool(TileClassId, BelSlotId, BelAttributeId, bool),
    BelAttrSpecial(TileClassId, BelSlotId, BelAttributeId, SpecialId),
    BelAttrSpecialBit(TileClassId, BelSlotId, BelAttributeId, SpecialId, usize),
    BelSpecial(TileClassId, BelSlotId, SpecialId),
    BelSpecialVal(TileClassId, BelSlotId, SpecialId, EnumValueId),
    BelSpecialString(TileClassId, BelSlotId, SpecialId, String),
    BelInputInv(TileClassId, BelSlotId, BelInputId, bool),
    GlobalSpecial(SpecialId),
    GlobalSpecialVal(SpecialId, EnumValueId),
    GlobalBelAttrBit(BelCoord, BelAttributeId, usize),
    GlobalBelAttrSpecial(BelCoord, BelAttributeId, SpecialId),
    GlobalRouting(WireCoord, PolWireCoord),
    Routing(TileClassId, TileWireCoord, PolTileWireCoord),
    RoutingVia(
        TileClassId,
        TileWireCoord,
        PolTileWireCoord,
        PolTileWireCoord,
    ),
    RoutingBidi(TileClassId, ConnectorSlotId, TileWireCoord, bool),
    RoutingInv(TileClassId, TileWireCoord, bool),
    RoutingSpecial(TileClassId, TileWireCoord, SpecialId),
    RoutingPairSpecial(TileClassId, TileWireCoord, PolTileWireCoord, SpecialId),
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

    pub fn discard_bits_raw(&mut self, bits: &[TileBit]) {
        for bit in bits {
            self.bits.remove(bit);
        }
    }

    pub fn discard_bits_enum(&mut self, attr: &BelAttributeEnum) {
        for bit in &attr.bits {
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

    pub fn apply_bitvec_diff_int_raw(&mut self, bits: &[PolTileBit], from: u64, to: u64) {
        fn to_bitvec(n: u64, len: usize) -> BitVec {
            let mut res = BitVec::repeat(false, len);
            for i in 0..64 {
                if (n & 1 << i) != 0 {
                    res.set(i, true);
                }
            }
            res
        }
        self.apply_bitvec_diff_raw(
            bits,
            &to_bitvec(from, bits.len()),
            &to_bitvec(to, bits.len()),
        );
    }

    pub fn apply_bit_diff(&mut self, item: &TileItem, from: bool, to: bool) {
        self.apply_bitvec_diff(item, &BitVec::from_iter([from]), &BitVec::from_iter([to]))
    }

    pub fn apply_bit_diff_raw(&mut self, bit: PolTileBit, from: bool, to: bool) {
        self.apply_bitvec_diff_raw(&[bit], &BitVec::from_iter([from]), &BitVec::from_iter([to]))
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

    pub fn apply_enum_diff_attr(
        &mut self,
        item: &BelAttributeEnum,
        from: EnumValueId,
        to: EnumValueId,
    ) {
        let from = &item.values[from];
        let to = &item.values[to];
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
            core::cmp::Ordering::Equal
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
    EnumData {
        bits: bits_vec,
        values,
    }
}

pub fn xlat_enum_attr_ocd(diffs: Vec<(EnumValueId, Diff)>, ocd: OcdMode) -> BelAttribute {
    let edata = xlat_enum_raw(diffs, ocd);
    BelAttribute::Enum(BelAttributeEnum {
        bits: edata.bits,
        values: edata.values.into_iter().collect(),
    })
}

pub fn xlat_enum_attr(diffs: Vec<(EnumValueId, Diff)>) -> BelAttribute {
    xlat_enum_attr_ocd(diffs, OcdMode::ValueOrder)
}

pub fn xlat_enum_ocd(diffs: Vec<(impl Into<String>, Diff)>, ocd: OcdMode) -> TileItem {
    let edata = xlat_enum_raw(
        diffs
            .into_iter()
            .map(|(key, diff)| (key.into(), diff))
            .collect(),
        ocd,
    );
    TileItem {
        bits: edata.bits,
        kind: TileItemKind::Enum {
            values: edata.values,
        },
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

pub fn xlat_bool_default_raw(diff0: Diff, diff1: Diff) -> (PolTileBit, bool) {
    let (diff, res) = if diff0.bits.is_empty() {
        diff0.assert_empty();
        (diff1, false)
    } else {
        diff1.assert_empty();
        (!diff0, true)
    };
    (xlat_bit_raw(diff), res)
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

pub fn xlat_bool_raw(diff0: Diff, diff1: Diff) -> PolTileBit {
    xlat_bool_default_raw(diff0, diff1).0
}

pub fn xlat_bool(diff0: Diff, diff1: Diff) -> TileItem {
    xlat_bool_default(diff0, diff1).0
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
