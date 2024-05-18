use std::collections::BTreeMap;

use prjcombine_types::{Tile, TileItem};

use crate::backend::FeatureBit;

pub struct TileDb {
    pub tiles: BTreeMap<String, Tile<FeatureBit>>,
}

impl TileDb {
    pub fn new() -> Self {
        Self {
            tiles: BTreeMap::new(),
        }
    }

    pub fn insert(
        &mut self,
        tile: impl Into<String>,
        name: impl Into<String>,
        item: TileItem<FeatureBit>,
    ) {
        let tile = self
            .tiles
            .entry(tile.into())
            .or_insert_with(Default::default);
        tile.insert(name, item, |_| false);
    }
}
