use core::fmt::Debug;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, btree_map, hash_map};

use itertools::Itertools;
use prjcombine_entity::{
    EntityPartVec, EntityVec,
    id::{EntityIdU16, EntityTag},
};
use prjcombine_interconnect::{
    db::{
        BelAttributeEnum, BelAttributeId, BelInputId, BelSlotId, ConnectorSlotId, EnumValueId,
        PolTileWireCoord, TableRowId, TileClassId, TileWireCoord,
    },
    grid::{BelCoord, PolWireCoord, WireCoord},
};
use prjcombine_types::{
    bitvec::BitVec,
    bsdata::{BitRectId, EnumData, PolTileBit, TileBit},
};

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
    BelAttrBit(TileClassId, BelSlotId, BelAttributeId, usize, bool),
    BelAttrValue(TileClassId, BelSlotId, BelAttributeId, EnumValueId),
    BelAttrBitVec(TileClassId, BelSlotId, BelAttributeId, BitVec),
    BelAttrU32(TileClassId, BelSlotId, BelAttributeId, u32),
    BelAttrValueSpecial(
        TileClassId,
        BelSlotId,
        BelAttributeId,
        EnumValueId,
        SpecialId,
    ),
    BelAttrRow(TileClassId, BelSlotId, BelAttributeId, TableRowId),
    BelAttrSpecial(TileClassId, BelSlotId, BelAttributeId, SpecialId),
    BelAttrSpecialBit(
        TileClassId,
        BelSlotId,
        BelAttributeId,
        SpecialId,
        usize,
        bool,
    ),
    BelAttrSpecialValue(
        TileClassId,
        BelSlotId,
        BelAttributeId,
        SpecialId,
        EnumValueId,
    ),
    BelSpecial(TileClassId, BelSlotId, SpecialId),
    BelSpecialVal(TileClassId, BelSlotId, SpecialId, EnumValueId),
    BelSpecialBit(TileClassId, BelSlotId, SpecialId, usize),
    BelSpecialU32(TileClassId, BelSlotId, SpecialId, u32),
    BelSpecialRow(TileClassId, BelSlotId, SpecialId, TableRowId),
    BelSpecialSpecialSpecialRow(
        TileClassId,
        BelSlotId,
        SpecialId,
        SpecialId,
        SpecialId,
        TableRowId,
    ),
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
    ProgDelay(TileClassId, TileWireCoord, usize),
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

    pub fn discard_bits(&mut self, bits: &[TileBit]) {
        for bit in bits {
            self.bits.remove(bit);
        }
    }

    pub fn discard_polbits(&mut self, bits: &[PolTileBit]) {
        for bit in bits {
            self.bits.remove(&bit.bit);
        }
    }

    pub fn discard_bits_enum(&mut self, attr: &BelAttributeEnum) {
        self.discard_bits(&attr.bits);
    }

    pub fn apply_bitvec_diff(&mut self, bits: &[PolTileBit], from: &BitVec, to: &BitVec) {
        assert_eq!(bits.len(), from.len());
        assert_eq!(bits.len(), to.len());
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

    pub fn apply_bitvec_diff_int(&mut self, bits: &[PolTileBit], from: u64, to: u64) {
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
            bits,
            &to_bitvec(from, bits.len()),
            &to_bitvec(to, bits.len()),
        );
    }

    pub fn apply_bit_diff(&mut self, bit: PolTileBit, from: bool, to: bool) {
        self.apply_bitvec_diff(&[bit], &BitVec::from_iter([from]), &BitVec::from_iter([to]))
    }

    pub fn apply_enum_bits_raw(&mut self, bits: &[TileBit], from: &BitVec, to: &BitVec) {
        for (idx, &bit) in bits.iter().enumerate() {
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

    pub fn apply_enum_diff_raw<K: Ord>(&mut self, item: &EnumData<K>, from: &K, to: &K) {
        let from = &item.values[from];
        let to = &item.values[to];
        self.apply_enum_bits_raw(&item.bits, from, to);
    }

    pub fn apply_enum_diff(&mut self, item: &BelAttributeEnum, from: EnumValueId, to: EnumValueId) {
        let from = &item.values[from];
        let to = &item.values[to];
        self.apply_enum_bits_raw(&item.bits, from, to);
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

pub fn xlat_bit_wide(diff: Diff) -> Vec<PolTileBit> {
    let mut res = vec![];
    for (&bit, &val) in &diff.bits {
        res.push(PolTileBit { bit, inv: !val });
    }
    res.sort();
    res
}

pub fn xlat_bit(diff: Diff) -> PolTileBit {
    assert_eq!(diff.bits.len(), 1);
    let (&bit, &val) = diff.bits.iter().next().unwrap();
    PolTileBit { bit, inv: !val }
}

pub fn xlat_bitvec(diffs: Vec<Diff>) -> Vec<PolTileBit> {
    diffs.into_iter().map(xlat_bit).collect()
}

pub fn xlat_bit_bi_default(diff0: Diff, diff1: Diff) -> (PolTileBit, bool) {
    let (diff, res) = if diff0.bits.is_empty() {
        diff0.assert_empty();
        (diff1, false)
    } else {
        diff1.assert_empty();
        (!diff0, true)
    };
    (xlat_bit(diff), res)
}

pub fn xlat_bit_bi(diff0: Diff, diff1: Diff) -> PolTileBit {
    xlat_bit_bi_default(diff0, diff1).0
}

pub fn xlat_bit_wide_bi_default(diff0: Diff, diff1: Diff) -> (Vec<PolTileBit>, BitVec) {
    let mut bits = xlat_bit_wide(diff1.combine(&!&diff0));
    assert_eq!(bits.len(), diff0.bits.len() + diff1.bits.len());
    bits.sort();
    let default = BitVec::from_iter(bits.iter().map(|bit| diff0.bits.contains_key(&bit.bit)));
    (bits, default)
}

pub fn xlat_bit_wide_bi(diff0: Diff, diff1: Diff) -> Vec<PolTileBit> {
    xlat_bit_wide_bi_default(diff0, diff1).0
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

pub fn xlat_enum_attr_ocd(diffs: Vec<(EnumValueId, Diff)>, ocd: OcdMode) -> BelAttributeEnum {
    let edata = xlat_enum_raw(diffs, ocd);
    BelAttributeEnum {
        bits: edata.bits,
        values: edata.values.into_iter().collect(),
    }
}

pub fn xlat_enum_attr(diffs: Vec<(EnumValueId, Diff)>) -> BelAttributeEnum {
    xlat_enum_attr_ocd(diffs, OcdMode::ValueOrder)
}

pub fn xlat_bitvec_sparse(diffs: Vec<(BitVec, Diff)>) -> Vec<PolTileBit> {
    let width = diffs[0].0.len();
    let mut bits: Vec<Option<PolTileBit>> = vec![None; width];
    let mut xor = BitVec::repeat(false, width);
    for (val, diff) in &diffs {
        assert_eq!(val.len(), width);
        if diff.bits.is_empty() {
            xor = val.clone();
        }
    }
    fn strip_known(
        val: &BitVec,
        diff: &Diff,
        tgt: &BitVec,
        bits: &[Option<PolTileBit>],
    ) -> (BitVec, Diff) {
        let mut val = val.clone();
        let mut diff = diff.clone();
        for (i, &bit) in bits.iter().enumerate() {
            let Some(bit) = bit else { continue };
            if val[i] != tgt[i] {
                diff.apply_bit_diff(bit, val[i], tgt[i]);
                val.set(i, tgt[i]);
            }
        }
        (val, diff)
    }
    loop {
        let mut progress = false;
        let mut done = true;
        for (val, diff) in &diffs {
            let (val, diff) = strip_known(val, diff, &xor, &bits);
            let val_xor = &val ^ &xor;
            if !val_xor.any() {
                diff.assert_empty();
            } else if let Some(bidx) = val_xor.as_one_hot() {
                let mut bit = xlat_bit(diff);
                bit.inv ^= xor[bidx];
                bits[bidx] = Some(bit);
                progress = true;
            } else {
                done = false;
            }
        }
        if done {
            return Vec::from_iter(bits.iter().map(|bit| bit.unwrap()));
        }
        if !progress {
            'try_two: for (val_a, diff_a) in &diffs {
                let (val_a, diff_a) = strip_known(val_a, diff_a, &xor, &bits);
                for (val_b, diff_b) in &diffs {
                    let (val_b, diff_b) = strip_known(val_b, diff_b, &xor, &bits);
                    let val_xor = &val_a ^ &val_b;
                    if let Some(bidx) = val_xor.as_one_hot() {
                        assert!(bits[bidx].is_none());
                        let diff = if val_b[bidx] {
                            diff_b.combine(&!diff_a)
                        } else {
                            diff_a.combine(&!diff_b)
                        };
                        let bit = xlat_bit(diff);
                        bits[bidx] = Some(bit);
                        progress = true;
                        break 'try_two;
                    }
                }
            }
        }
        if !progress {
            panic!("NO PROGRESS: {bits:?} {diffs:?}")
        }
    }
}

pub fn xlat_bitvec_sparse_u32(diffs: Vec<(u32, Diff)>) -> Vec<PolTileBit> {
    let mut width = 0;
    for &(n, _) in &diffs {
        width = width.max(32 - n.leading_zeros());
    }
    let mut new_diffs = vec![];
    for (n, diff) in diffs {
        let bits = BitVec::from_iter((0..width).map(|bidx| (n & 1 << bidx) != 0));
        new_diffs.push((bits, diff));
    }
    xlat_bitvec_sparse(new_diffs)
}

pub fn extract_common_diff<K>(diffs: &mut [(K, Diff)]) -> Diff {
    let mut common = diffs[0].1.clone();
    for (_, diff) in &*diffs {
        common.bits.retain(|k, _| diff.bits.contains_key(k));
    }
    for (_, diff) in diffs {
        for (&k, &v) in &common.bits {
            assert_eq!(diff.bits.remove(&k), Some(v));
        }
    }
    common
}

pub fn extract_bitvec_val_part(bits: &[PolTileBit], base: &BitVec, diff: &mut Diff) -> BitVec {
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

pub fn extract_bitvec_val(bits: &[PolTileBit], base: &BitVec, diff: Diff) -> BitVec {
    assert_eq!(bits.len(), base.len());
    let mut res = base.clone();
    let rev: HashMap<_, _> = bits
        .iter()
        .copied()
        .enumerate()
        .map(|(i, v)| (v.bit, i))
        .collect();
    for (&bit, &val) in diff.bits.iter() {
        let bitidx = rev[&bit];
        assert_eq!(res[bitidx], !(val ^ bits[bitidx].inv));
        res.set(bitidx, val ^ bits[bitidx].inv);
    }
    res
}

pub fn enum_ocd_swap_bits<K: Ord>(item: &mut EnumData<K>, a: usize, b: usize) {
    item.bits.swap(a, b);
    for val in item.values.values_mut() {
        val.swap(a, b);
    }
}
