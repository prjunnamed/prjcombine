use core::fmt::Debug;
use std::{
    collections::{BTreeMap, btree_map},
    error::Error,
    fs::File,
    path::Path,
};

use bitvec::vec::BitVec;
use itertools::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct TileBit {
    pub tile: usize,
    pub frame: usize,
    pub bit: usize,
}

impl TileBit {
    pub fn new(tile: usize, frame: usize, bit: usize) -> Self {
        Self { tile, frame, bit }
    }
}

impl core::fmt::Debug for TileBit {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}", self.tile, self.frame, self.bit)
    }
}

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

    pub fn from_bit(bit: T, invert: bool) -> Self {
        Self {
            bits: vec![bit],
            kind: TileItemKind::BitVec {
                invert: BitVec::from_iter([invert]),
            },
        }
    }

    pub fn from_bitvec(bits: Vec<T>, invert: bool) -> Self {
        let invert = BitVec::repeat(invert, bits.len());
        Self {
            bits,
            kind: TileItemKind::BitVec { invert },
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum TileItemKind {
    Enum { values: BTreeMap<String, BitVec> },
    BitVec { invert: BitVec },
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum DbValue {
    String(String),
    BitVec(BitVec),
    Int(u32),
}

impl From<BitVec> for DbValue {
    fn from(value: BitVec) -> Self {
        Self::BitVec(value)
    }
}

impl<const N: usize> From<[bool; N]> for DbValue {
    fn from(value: [bool; N]) -> Self {
        Self::BitVec(BitVec::from_iter(value))
    }
}

impl From<String> for DbValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<u32> for DbValue {
    fn from(value: u32) -> Self {
        Self::Int(value)
    }
}

impl DbValue {
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            DbValue::String(s) => (&s[..]).into(),
            DbValue::Int(i) => (*i).into(),
            DbValue::BitVec(bv) => Vec::from_iter(bv.iter().map(|x| *x)).into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TileDb {
    pub tiles: BTreeMap<String, Tile<TileBit>>,
    pub device_data: BTreeMap<String, BTreeMap<String, DbValue>>,
    pub misc_data: BTreeMap<String, DbValue>,
}

impl TileDb {
    pub fn new() -> Self {
        Self {
            tiles: BTreeMap::new(),
            device_data: BTreeMap::new(),
            misc_data: BTreeMap::new(),
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let f = File::open(path)?;
        let cf = zstd::stream::Decoder::new(f)?;
        Ok(bincode::deserialize_from(cf)?)
    }

    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let f = File::create(path)?;
        let mut cf = zstd::stream::Encoder::new(f, 9)?;
        bincode::serialize_into(&mut cf, self)?;
        cf.finish()?;
        Ok(())
    }

    pub fn insert(
        &mut self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        name: impl Into<String>,
        item: TileItem<TileBit>,
    ) {
        let name = format!("{}:{}", bel.into(), name.into());
        let tile = self.tiles.entry(tile.into()).or_default();
        tile.insert(name, item, |_| false);
    }

    #[track_caller]
    pub fn item(&self, tile: &str, bel: &str, attr: &str) -> &TileItem<TileBit> {
        &self.tiles[tile].items[&format!("{bel}:{attr}")]
    }

    pub fn insert_misc_data(&mut self, key: impl Into<String>, val: impl Into<DbValue>) {
        let key = key.into();
        let val = val.into();
        match self.misc_data.entry(key) {
            btree_map::Entry::Vacant(e) => {
                e.insert(val);
            }
            btree_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), val);
            }
        }
    }

    pub fn insert_device_data(
        &mut self,
        device: &str,
        key: impl Into<String>,
        val: impl Into<DbValue>,
    ) {
        let dev = self.device_data.entry(device.into()).or_default();
        let key = key.into();
        let val = val.into();
        match dev.entry(key) {
            btree_map::Entry::Vacant(e) => {
                e.insert(val);
            }
            btree_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), val);
            }
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "tiles": serde_json::Map::from_iter(self.tiles.iter().map(|(name, tile)| {
                (
                    name.clone(),
                    tile.to_json(|crd| json!((crd.tile, crd.frame, crd.bit))),
                )
            })),
            "misc_data": serde_json::Map::from_iter(self.misc_data.iter().map(|(k, v)| {
                (k.clone(), v.to_json())
            })),
            "device_data": serde_json::Map::from_iter(self.device_data.iter().map(|(k, v)| {
                (k.clone(), serde_json::Map::from_iter(v.iter().map(|(kk, vv)| {
                    (kk.clone(), vv.to_json())
                })).into())
            })),
        })
    }
}

impl Default for TileDb {
    fn default() -> Self {
        Self::new()
    }
}
