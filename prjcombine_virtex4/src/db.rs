use std::{collections::BTreeSet, error::Error, fs::File, path::Path};

use prjcombine_int::{db::IntDb, grid::DieId};
use prjcombine_types::tiledb::TileDb;
use serde::{Deserialize, Serialize};
use serde_json::json;
use unnamed_entity::{entity_id, EntityId, EntityMap, EntityVec};

use crate::{
    bond::Bond,
    grid::{DisabledPart, Grid, Interposer},
};

entity_id! {
    pub id GridId usize;
    pub id InterposerId usize;
    pub id BondId usize;
    pub id DevBondId usize;
    pub id DevSpeedId usize;
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeviceCombo {
    pub devbond: DevBondId,
    pub speed: DevSpeedId,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub name: String,
    pub grids: EntityVec<DieId, GridId>,
    pub interposer: Option<InterposerId>,
    pub bonds: EntityMap<DevBondId, String, BondId>,
    pub speeds: EntityVec<DevSpeedId, String>,
    pub combos: Vec<DeviceCombo>,
    pub disabled: BTreeSet<DisabledPart>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Database {
    pub grids: EntityVec<GridId, Grid>,
    pub interposers: EntityVec<InterposerId, Interposer>,
    pub bonds: EntityVec<BondId, Bond>,
    pub parts: Vec<Part>,
    pub int: IntDb,
    pub tiles: TileDb,
}

impl Database {
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

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "grids": Vec::from_iter(self.grids.values().map(|grid| grid.to_json())),
            "interposers": self.interposers,
            "bonds": Vec::from_iter(self.bonds.values().map(|bond| bond.to_json())),
            "parts": Vec::from_iter(self.parts.iter().map(|part| {
                json!({
                    "name": part.name,
                    "interposer": part.interposer,
                    "grids": part.grids,
                    "bonds": serde_json::Map::from_iter(part.bonds.iter().map(|(_, name, bond)| (name.clone(), bond.to_idx().into()))),
                    "speeds": part.speeds,
                    "combos": part.combos,
                    "disabled": Vec::from_iter(part.disabled.iter().map(|dis| match dis {
                        DisabledPart::Emac(row) => format!("EMAC:{row}"),
                        DisabledPart::GtxRow(reg) => format!("GTX:{reg}"),
                        DisabledPart::SysMon => "SYSMON".to_string(),
                        DisabledPart::Gtp => "GTP".to_string(),
                    })),
                })
            })),
            "int": self.int.to_json(),
            "tiles": self.tiles.to_json(),
        })
    }
}
