use std::collections::{btree_map, hash_map, BTreeMap, HashMap};

use bitvec::vec::BitVec;
use prjcombine_types::{TileItem, TileItemKind};
use prjcombine_xilinx_geom::{Device, ExpandedDevice};

use crate::{
    backend::{FeatureBit, State},
    tiledb::TileDb,
};

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct Diff {
    pub bits: HashMap<FeatureBit, bool>,
}

impl Diff {
    pub fn assert_empty(self) {
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

    pub fn discard_bits(&mut self, item: &TileItem<FeatureBit>) {
        for bit in item.bits.iter() {
            self.bits.remove(bit);
        }
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
}

impl core::ops::Not for Diff {
    type Output = Diff;

    fn not(self) -> Self::Output {
        Diff {
            bits: self.bits.into_iter().map(|(k, v)| (k, !v)).collect(),
        }
    }
}

pub struct CollectorCtx<'a, 'b>
where
    'b: 'a,
{
    pub device: &'a Device,
    pub edev: &'a ExpandedDevice<'a>,
    pub state: &'b mut State<'a>,
    pub tiledb: &'b mut TileDb,
}

pub fn xlat_bitvec(diffs: Vec<Diff>) -> TileItem<FeatureBit> {
    let mut invert = None;
    let mut bits = vec![];
    for diff in diffs {
        assert_eq!(diff.bits.len(), 1);
        for (k, v) in diff.bits {
            bits.push(k);
            if invert.is_none() {
                invert = Some(!v);
            } else {
                assert_eq!(invert, Some(!v));
            }
        }
    }
    TileItem {
        bits,
        kind: TileItemKind::BitVec {
            invert: invert.unwrap(),
        },
    }
}

pub fn xlat_enum(diffs: Vec<(String, Diff)>) -> TileItem<FeatureBit> {
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
    let bits_vec: Vec<_> = bits.iter().map(|(&f, _)| f).collect();
    let values = diffs
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                bits_vec
                    .iter()
                    .map(|&bit| bits[&bit] ^ !v.bits.contains_key(&bit))
                    .collect(),
            )
        })
        .collect();
    TileItem {
        bits: bits_vec,
        kind: TileItemKind::Enum { values },
    }
}

pub fn xlat_enum_default(
    diffs: Vec<(String, Diff)>,
    default: impl Into<String>,
) -> TileItem<FeatureBit> {
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
    let bits_vec: Vec<_> = bits.iter().map(|(&f, _)| f).collect();
    let mut values: BTreeMap<_, _> = diffs
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                bits_vec
                    .iter()
                    .map(|&bit| bits[&bit] ^ !v.bits.contains_key(&bit))
                    .collect(),
            )
        })
        .collect();
    values.insert(
        default.into(),
        bits_vec.iter().map(|&bit| !bits[&bit]).collect(),
    );
    TileItem {
        bits: bits_vec,
        kind: TileItemKind::Enum { values },
    }
}

pub fn xlat_bool(diff0: Diff, diff1: Diff) -> TileItem<FeatureBit> {
    let diff = if diff0.bits.is_empty() {
        diff0.assert_empty();
        diff1
    } else {
        diff1.assert_empty();
        !diff0
    };
    xlat_bitvec(vec![diff])
}

impl<'a, 'b: 'a> CollectorCtx<'a, 'b> {
    pub fn collect_bitvec(&mut self, tile: &'a str, bel: &'a str, attr: &'a str, val: &'a str) {
        let diffs = self.state.get_diffs(tile, bel, attr, val);
        let ti = xlat_bitvec(diffs);
        self.tiledb.insert(tile, bel, attr, ti);
    }

    pub fn collect_enum(&mut self, tile: &'a str, bel: &'a str, attr: &'a str, vals: &[&'a str]) {
        let diffs = vals
            .iter()
            .map(|val| (val.to_string(), self.state.get_diff(tile, bel, attr, val)))
            .collect();
        let ti = xlat_enum(diffs);
        self.tiledb.insert(tile, bel, attr, ti);
    }

    pub fn collect_enum_default(
        &mut self,
        tile: &'b str,
        bel: &'b str,
        attr: &'b str,
        vals: &[&'b str],
        default: &'b str,
    ) {
        let diffs = vals
            .iter()
            .map(|val| (val.to_string(), self.state.get_diff(tile, bel, attr, val)))
            .collect();
        let ti = xlat_enum_default(diffs, default);
        self.tiledb.insert(tile, bel, attr, ti);
    }

    pub fn extract_enum_bool(
        &mut self,
        tile: &'b str,
        bel: &'b str,
        attr: &'b str,
        val0: &'b str,
        val1: &'b str,
    ) -> TileItem<FeatureBit> {
        let d0 = self.state.get_diff(tile, bel, attr, val0);
        let d1 = self.state.get_diff(tile, bel, attr, val1);
        xlat_bool(d0, d1)
    }

    pub fn collect_enum_bool(
        &mut self,
        tile: &'b str,
        bel: &'b str,
        attr: &'b str,
        val0: &'b str,
        val1: &'b str,
    ) {
        let item = self.extract_enum_bool(tile, bel, attr, val0, val1);
        self.tiledb.insert(tile, bel, attr, item);
    }

    pub fn collect_inv(&mut self, tile: &'b str, bel: &'b str, pin: &'b str) {
        let pininv = format!("{pin}INV").leak();
        let pin_b = format!("{pin}_B").leak();
        self.collect_enum_bool(tile, bel, pininv, pin, pin_b);
    }
}

pub fn extract_bitvec_val(item: &TileItem<FeatureBit>, base: &BitVec, diff: Diff) -> BitVec {
    let TileItemKind::BitVec { invert } = item.kind else {
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
        assert_eq!(res[bitidx], !(val ^ invert));
        res.set(bitidx, val ^ invert);
    }
    res
}
