use std::collections::{btree_map, BTreeMap};

use bitvec::vec::BitVec;
use prjcombine_types::{Tile, TileBit, TileItem};
use serde_json::json;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
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

#[derive(Clone, Debug)]
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
