use std::{
    cmp::Ordering,
    collections::{btree_map, hash_map, BTreeMap, BTreeSet, HashMap, HashSet},
    fmt::Debug,
    ops::Range,
};

use bitvec::vec::BitVec;
use itertools::Itertools;
use prjcombine_types::{TileItem, TileItemKind};
use prjcombine_virtex_bitstream::Bitstream;
use prjcombine_xilinx_geom::{Device, ExpandedDevice, GeomDb};
use unnamed_entity::EntityId;

use crate::{
    backend::{FeatureBit, State},
    tiledb::TileDb,
};

#[derive(Clone, Eq, PartialEq, Default)]
pub struct Diff {
    pub bits: HashMap<FeatureBit, bool>,
}

impl Debug for Diff {
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

    pub fn split_bits_by(&mut self, mut f: impl FnMut(FeatureBit) -> bool) -> Diff {
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

    pub fn split_bits(&mut self, bits: &HashSet<FeatureBit>) -> Diff {
        self.split_bits_by(|bit| bits.contains(&bit))
    }

    pub fn discard_bits(&mut self, item: &TileItem<FeatureBit>) {
        for bit in item.bits.iter() {
            self.bits.remove(bit);
        }
    }

    pub fn apply_bitvec_diff(&mut self, item: &TileItem<FeatureBit>, from: &BitVec, to: &BitVec) {
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

    pub fn apply_bitvec_diff_int(&mut self, item: &TileItem<FeatureBit>, from: u64, to: u64) {
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

    pub fn apply_bit_diff(&mut self, item: &TileItem<FeatureBit>, from: bool, to: bool) {
        self.apply_bitvec_diff(item, &BitVec::from_iter([from]), &BitVec::from_iter([to]))
    }

    pub fn apply_enum_diff(&mut self, item: &TileItem<FeatureBit>, from: &str, to: &str) {
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
            let newbit = FeatureBit {
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
            let newbit = FeatureBit {
                tile: tileidx,
                ..bit
            };
            res.bits.insert(newbit, val);
        }
        res
    }

    pub fn from_bool_item(item: &TileItem<FeatureBit>) -> Self {
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

pub struct CollectorCtx<'a, 'b>
where
    'b: 'a,
{
    pub device: &'a Device,
    pub db: &'a GeomDb,
    pub edev: &'a ExpandedDevice<'a>,
    pub state: &'b mut State,
    pub tiledb: &'b mut TileDb,
    pub empty_bs: &'a Bitstream,
}

pub fn enum_ocd_swap_bits(item: &mut TileItem<FeatureBit>, a: usize, b: usize) {
    item.bits.swap(a, b);
    let TileItemKind::Enum { ref mut values } = item.kind else {
        unreachable!()
    };
    for val in values.values_mut() {
        val.swap(a, b);
    }
}

pub fn xlat_item_tile_fwd(item: TileItem<FeatureBit>, xlat: &[usize]) -> TileItem<FeatureBit> {
    TileItem {
        bits: item
            .bits
            .into_iter()
            .map(|bit| FeatureBit {
                tile: xlat[bit.tile],
                ..bit
            })
            .collect(),
        kind: item.kind,
    }
}

pub fn xlat_item_tile(item: TileItem<FeatureBit>, xlat: &[usize]) -> TileItem<FeatureBit> {
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
            .map(|bit| FeatureBit {
                tile: rxlat[bit.tile].unwrap(),
                ..bit
            })
            .collect(),
        kind: item.kind,
    }
}

pub fn xlat_bitvec(diffs: Vec<Diff>) -> TileItem<FeatureBit> {
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

pub fn xlat_bit(diff: Diff) -> TileItem<FeatureBit> {
    xlat_bitvec(vec![diff])
}

pub fn xlat_bit_wide(diff: Diff) -> TileItem<FeatureBit> {
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

pub fn concat_bitvec(vecs: impl IntoIterator<Item = TileItem<FeatureBit>>) -> TileItem<FeatureBit> {
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
    FixedOrder(&'a [FeatureBit]),
}

pub fn xlat_enum_ocd(diffs: Vec<(impl Into<String>, Diff)>, ocd: OcdMode) -> TileItem<FeatureBit> {
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
                        "MISMATCH FOR {n}: {cur:?} {new:?}",
                        n = e.key(),
                        cur = Vec::from_iter(e.get().iter().map(|x| *x)),
                        new = Vec::from_iter(value.into_iter())
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

pub fn xlat_enum(diffs: Vec<(impl Into<String>, Diff)>) -> TileItem<FeatureBit> {
    xlat_enum_ocd(diffs, OcdMode::ValueOrder)
}

pub fn xlat_enum_default(
    mut diffs: Vec<(String, Diff)>,
    default: impl Into<String>,
) -> TileItem<FeatureBit> {
    diffs.insert(0, (default.into(), Diff::default()));
    xlat_enum(diffs)
}

pub fn xlat_enum_default_ocd(
    mut diffs: Vec<(String, Diff)>,
    default: impl Into<String>,
    ocd: OcdMode,
) -> TileItem<FeatureBit> {
    diffs.insert(0, (default.into(), Diff::default()));
    xlat_enum_ocd(diffs, ocd)
}

pub fn xlat_enum_int(diffs: Vec<(u32, Diff)>) -> TileItem<FeatureBit> {
    let mut bits: Vec<Option<FeatureBit>> = vec![];
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

pub fn xlat_bool_default(diff0: Diff, diff1: Diff) -> (TileItem<FeatureBit>, bool) {
    let (diff, res) = if diff0.bits.is_empty() {
        diff0.assert_empty();
        (diff1, false)
    } else {
        diff1.assert_empty();
        (!diff0, true)
    };
    (xlat_bit(diff), res)
}

pub fn xlat_bool(diff0: Diff, diff1: Diff) -> TileItem<FeatureBit> {
    xlat_bool_default(diff0, diff1).0
}

impl<'a, 'b: 'a> CollectorCtx<'a, 'b> {
    pub fn has_tile(&self, tile: &str) -> bool {
        let egrid = self.edev.egrid();
        let node = egrid.db.get_node(tile);
        !egrid.node_index[node].is_empty()
    }

    #[must_use]
    pub fn extract_bitvec(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val: &str,
    ) -> TileItem<FeatureBit> {
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
    ) -> TileItem<FeatureBit> {
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
    ) -> TileItem<FeatureBit> {
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
    ) -> TileItem<FeatureBit> {
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
    pub fn extract_bit(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val: &str,
    ) -> TileItem<FeatureBit> {
        let diff = self.state.get_diff(tile, bel, attr, val);
        xlat_bit(diff)
    }

    #[must_use]
    pub fn extract_bit_wide(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val: &str,
    ) -> TileItem<FeatureBit> {
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
    ) -> TileItem<FeatureBit> {
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
    ) -> TileItem<FeatureBit> {
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
    ) -> (TileItem<FeatureBit>, bool) {
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
    ) -> TileItem<FeatureBit> {
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
    ) -> TileItem<FeatureBit> {
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

    pub fn extract_inv(&mut self, tile: &str, bel: &str, pin: &str) -> TileItem<FeatureBit> {
        let pininv = format!("{pin}INV");
        let pin_b = format!("{pin}_B");
        self.extract_enum_bool(tile, bel, &pininv, pin, &pin_b)
    }

    pub fn collect_inv(&mut self, tile: &str, bel: &str, pin: &str) {
        let item = self.extract_inv(tile, bel, pin);
        self.tiledb.insert(tile, bel, format!("INV.{pin}"), item);
    }

    pub fn insert_int_inv(
        &mut self,
        int_tiles: &[&str],
        tile: &str,
        bel: &str,
        pin: &str,
        mut item: TileItem<FeatureBit>,
    ) {
        let intdb = self.edev.egrid().db;
        let node = intdb.nodes.get(tile).unwrap().1;
        let bel = node.bels.get(bel).unwrap().1;
        let pin = &bel.pins[pin];
        assert_eq!(pin.wires.len(), 1);
        let wire = *pin.wires.first().unwrap();
        assert_eq!(item.bits.len(), 1);
        let bit = &mut item.bits[0];
        assert_eq!(wire.0.to_idx(), bit.tile);
        bit.tile = 0;
        let wire_name = intdb.wires.key(wire.1);
        self.tiledb.insert(
            int_tiles[wire.0.to_idx()],
            "INT",
            format!("INV.{wire_name}"),
            item,
        );
    }

    pub fn item_int_inv(
        &self,
        int_tiles: &[&str],
        tile: &str,
        bel: &str,
        pin: &str,
    ) -> TileItem<FeatureBit> {
        let intdb = self.edev.egrid().db;
        let node = intdb.nodes.get(tile).unwrap().1;
        let bel = node.bels.get(bel).unwrap().1;
        let pin = &bel.pins[pin];
        assert_eq!(pin.wires.len(), 1);
        let wire = *pin.wires.first().unwrap();
        let wire_name = intdb.wires.key(wire.1);
        let mut item = self
            .tiledb
            .item(
                int_tiles[wire.0.to_idx()],
                "INT",
                &format!("INV.{wire_name}"),
            )
            .clone();
        assert_eq!(item.bits.len(), 1);
        let bit = &mut item.bits[0];
        bit.tile = wire.0.to_idx();
        item
    }

    pub fn collect_int_inv(
        &mut self,
        int_tiles: &[&str],
        tile: &str,
        bel: &str,
        pin: &str,
        flip: bool,
    ) {
        let pininv = format!("{pin}INV");
        let pin_b = format!("{pin}_B");
        let item = self.extract_enum_bool(
            tile,
            bel,
            &pininv,
            if flip { &pin_b } else { pin },
            if flip { pin } else { &pin_b },
        );
        self.insert_int_inv(int_tiles, tile, bel, pin, item);
    }
}

pub fn extract_bitvec_val_part(
    item: &TileItem<FeatureBit>,
    base: &BitVec,
    diff: &mut Diff,
) -> BitVec {
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

pub fn extract_bitvec_val(item: &TileItem<FeatureBit>, base: &BitVec, diff: Diff) -> BitVec {
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
