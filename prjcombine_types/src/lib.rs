use core::fmt::Debug;
use std::collections::{btree_map, BTreeMap};

use bitvec::vec::BitVec;
use itertools::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use unnamed_entity::entity_id;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Tile<T> {
    pub items: BTreeMap<String, TileItem<T>>,
}

impl<T: Debug + Copy + Eq + Ord> Default for Tile<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Debug + Copy + Eq + Ord> Tile<T> {
    pub fn new() -> Self {
        Self {
            items: BTreeMap::new(),
        }
    }

    pub fn merge(&mut self, other: &Tile<T>, neutral: impl Fn(T) -> bool) {
        if self == other {
            return;
        }
        for (k, v) in &other.items {
            match self.items.entry(k.clone()) {
                btree_map::Entry::Vacant(e) => {
                    e.insert(v.clone());
                }
                btree_map::Entry::Occupied(mut e) => {
                    e.get_mut().merge(v, &neutral);
                }
            }
        }
    }

    pub fn insert(
        &mut self,
        name: impl Into<String>,
        item: TileItem<T>,
        neutral: impl Fn(T) -> bool,
    ) {
        match self.items.entry(name.into()) {
            btree_map::Entry::Vacant(e) => {
                e.insert(item);
            }
            btree_map::Entry::Occupied(mut e) => {
                e.get_mut().merge(&item, neutral);
            }
        }
    }

    pub fn to_json(&self, bit_to_json: impl Fn(T) -> serde_json::Value) -> serde_json::Value {
        serde_json::Map::from_iter(self.items.iter().map(|(k, v)| {
            (
                k.clone(),
                match &v.kind {
                    TileItemKind::Enum { values } => json!({
                        "bits": Vec::from_iter(v.bits.iter().copied().map(&bit_to_json)),
                        "values": serde_json::Map::from_iter(
                            values.iter().map(|(vk, vv)| {
                                (vk.clone(), Vec::from_iter(vv.iter().map(|x| *x)).into())
                            })
                        ),
                    }),
                    TileItemKind::BitVec { invert } => json!({
                        "bits": Vec::from_iter(v.bits.iter().copied().map(&bit_to_json)),
                        "invert": if invert.iter().all(|x| !*x) {
                            json!(false)
                        } else if invert.iter().all(|x| *x) {
                            json!(true)
                        } else {
                            json!(Vec::from_iter(invert.iter().map(|x| *x)))
                        },
                    }),
                },
            )
        }))
        .into()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TileItem<T> {
    pub bits: Vec<T>,
    pub kind: TileItemKind,
}

impl<T: Debug + Copy + Eq + Ord> TileItem<T> {
    pub fn merge(&mut self, other: &TileItem<T>, neutral: impl Fn(T) -> bool) {
        if self == other {
            return;
        }
        let TileItemKind::Enum { values: av } = &mut self.kind else {
            panic!("weird merge: {self:?} {other:?}");
        };
        let TileItemKind::Enum { values: bv } = &other.kind else {
            unreachable!()
        };
        let mut bits = self.bits.clone();
        for &bit in &other.bits {
            if !bits.contains(&bit) {
                bits.push(bit);
            }
        }
        bits.sort();
        let bit_map_a: Vec<_> = bits
            .iter()
            .map(|&x| self.bits.iter().find_position(|&&y| x == y).map(|x| x.0))
            .collect();
        let bit_map_b: Vec<_> = bits
            .iter()
            .map(|&x| other.bits.iter().find_position(|&&y| x == y).map(|x| x.0))
            .collect();
        self.bits = bits;
        for val in av.values_mut() {
            *val = bit_map_a
                .iter()
                .enumerate()
                .map(|(i, &x)| match x {
                    Some(idx) => val[idx],
                    None => neutral(self.bits[i]),
                })
                .collect();
        }
        for (key, val) in bv {
            let val: BitVec = bit_map_b
                .iter()
                .enumerate()
                .map(|(i, &x)| match x {
                    Some(idx) => val[idx],
                    None => neutral(self.bits[i]),
                })
                .collect();

            match av.entry(key.clone()) {
                btree_map::Entry::Vacant(e) => {
                    e.insert(val);
                }
                btree_map::Entry::Occupied(e) => {
                    if *e.get() != val {
                        panic!("tile merge failed at {key}: {cv} vs {val:?}", cv = e.get());
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum TileItemKind {
    Enum { values: BTreeMap<String, BitVec> },
    BitVec { invert: BitVec },
}

entity_id! {
    pub id FbId u8;
    pub id FbMcId u8;
    pub id IpadId u8;
}

pub type McId = (FbId, FbMcId);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum IoId {
    Ipad(IpadId),
    Mc(McId),
}
