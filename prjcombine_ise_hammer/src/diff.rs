use std::collections::{btree_map, hash_map, BTreeMap, HashMap};

use prjcombine_types::{TileItem, TileItemKind};

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
}

impl core::ops::Not for Diff {
    type Output = Diff;

    fn not(self) -> Self::Output {
        Diff {
            bits: self.bits.into_iter().map(|(k, v)| (k, !v)).collect(),
        }
    }
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

pub fn collect_bitvec<'a, 'b: 'a>(
    state: &mut State<'a>,
    tiledb: &mut TileDb,
    tile: &'b str,
    bel: &'b str,
    attr: &'b str,
    val: &'b str,
) {
    let diffs = state.get_diffs(tile, bel, attr, val);
    let ti = xlat_bitvec(diffs);
    tiledb.insert(tile, format!("{bel}.{attr}"), ti);
}

pub fn collect_enum<'a, 'b: 'a>(
    state: &mut State<'a>,
    tiledb: &mut TileDb,
    tile: &'b str,
    bel: &'b str,
    attr: &'b str,
    vals: &[&'b str],
) {
    let diffs = vals
        .iter()
        .map(|val| (val.to_string(), state.get_diff(tile, bel, attr, val)))
        .collect();
    let ti = xlat_enum(diffs);
    tiledb.insert(tile, format!("{bel}.{attr}"), ti);
}
