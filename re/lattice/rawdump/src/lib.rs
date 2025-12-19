use ndarray::Array2;
use prjcombine_entity::{EntityMap, EntitySet, EntityVec, entity_id};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::path::Path;

entity_id! {
    pub id NodeId u32;
    pub id GridId u32;
    pub id BufferId u32, reserve 1;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Db {
    pub family: String,
    pub grids: EntityVec<GridId, Grid>,
    pub parts: Vec<Part>,
}

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct Grid {
    pub tiles: Array2<Tile>,
    pub nodes: EntityMap<NodeId, String, Node>,
    pub bufs: EntitySet<BufferId, String>,
    pub pips: HashMap<(NodeId, NodeId), Pip>,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug, Serialize, Deserialize)]
pub enum PinDir {
    Input,
    Output,
    Bidirectional,
}

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct Node {
    pub aliases: Vec<String>,
    pub typ: Option<u8>,
    pub pin: Option<(String, String, PinDir)>,
}

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct Pip {
    pub is_j: bool,
    pub buf: Option<BufferId>,
}

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct Tile {
    pub name: String,
    pub kind: String,
    pub width: usize,
    pub height: usize,
    pub x: usize,
    pub y: usize,
    pub sites: Vec<TileSite>,
}

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct TileSite {
    pub name: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct Site {
    pub name: String,
    pub typ: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Part {
    pub arch: String,
    pub name: String,
    pub package: String,
    pub speeds: Vec<String>,
    pub grid: GridId,
    pub sites: Vec<Site>,
}

impl Db {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let f = File::open(path)?;
        let mut cf = zstd::stream::Decoder::new(f)?;
        let config = bincode::config::legacy();
        Ok(bincode::serde::decode_from_std_read(&mut cf, config)?)
    }

    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let f = File::create(path)?;
        let mut cf = zstd::stream::Encoder::new(f, 9)?;
        let config = bincode::config::legacy();
        bincode::serde::encode_into_std_write(self, &mut cf, config)?;
        cf.finish()?;
        Ok(())
    }
}
