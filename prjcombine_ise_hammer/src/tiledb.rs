use std::collections::BTreeMap;

use prjcombine_types::{Tile, TileItem};
use serde_json::json;

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
        bel: impl Into<String>,
        name: impl Into<String>,
        item: TileItem<FeatureBit>,
    ) {
        let name = format!("{}:{}", bel.into(), name.into());
        let tile = self.tiles.entry(tile.into()).or_default();
        tile.insert(name, item, |_| false);
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::Map::from_iter(self.tiles.iter().map(|(name, tile)| {
            (
                name.clone(),
                tile.to_json(|crd| json!((crd.tile, crd.frame, crd.bit))),
            )
        }))
        .into()
    }
}
