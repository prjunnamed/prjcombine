use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::path::Path;
use prjcombine_entity::{entity_id, EntitySet, EntityVec, EntityMap, EntityPartVec};

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone, Serialize, Deserialize)]
pub struct Coord {
    pub x: u16,
    pub y: u16,
}

entity_id!{
    pub id WireId u32, reserve 1;
    pub id NodeClassId u32, reserve 1;
    pub id SpeedId u32, reserve 1;
    pub id SlotKindId u16;
    pub id TileKindId u16;
    pub id TemplateId u32;
    pub id NodeId u32, reserve 1;
    pub id TkSiteId u32;
    pub id TkPipId u32;
    pub id TkWireId u32;
    pub id TkConnWireId u32;
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum Source {
    ISE,
    Vivado,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum TkSiteSlot {
    Single(SlotKindId),
    Indexed(SlotKindId, u8),
    Xy(SlotKindId, u8, u8),
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum TkSitePinDir {
    Input,
    Output,
    Bidir,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub struct TkSitePin {
    pub dir: TkSitePinDir,
    pub wire: Option<WireId>,
    pub speed: Option<SpeedId>,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct TkSite {
    pub kind: String,
    pub pins: HashMap<String, TkSitePin>,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum TkWire {
    Internal(Option<SpeedId>, Option<NodeClassId>),
    Connected(TkConnWireId),
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum TkPipInversion {
    Never,
    Always,
    Prog,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum TkPipDirection {
    Uni,
    BiFwd,
    BiBwd,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub struct TkPip {
    pub is_buf: bool,
    pub is_excluded: bool,
    pub is_test: bool,
    pub inversion: TkPipInversion,
    pub direction: TkPipDirection,
    pub speed: Option<SpeedId>,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct TileKind {
    pub sites: EntityMap<TkSiteId, TkSiteSlot, TkSite>,
    pub wires: EntityMap<TkWireId, WireId, TkWire>,
    pub conn_wires: EntityVec<TkConnWireId, WireId>,
    pub pips: EntityMap<TkPipId, (WireId, WireId), TkPip>,
    pub tiles: Vec<Coord>,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Tile {
    pub name: String,
    pub kind: TileKindId,
    pub sites: EntityPartVec<TkSiteId, String>,
    #[serde(skip)]
    pub conn_wires: EntityPartVec<TkConnWireId, NodeId>,
    pub pip_overrides: HashMap<TkPipId, (NodeClassId, NodeClassId)>,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub struct TkNode {
    pub base: Coord,
    pub template: TemplateId,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone, Serialize, Deserialize)]
pub struct TkNodeTemplateWire {
    pub delta: Coord,
    pub wire: WireId,
    pub speed: Option<SpeedId>,
    pub cls: Option<NodeClassId>,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct PartCombo {
    pub name: String,
    pub device: String,
    pub package: String,
    pub speed: String,
    pub temp: String,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Serialize, Deserialize)]
pub struct PkgPin {
    pub pad: Option<String>,
    pub pin: String,
    pub vref_bank: Option<u32>,
    pub vcco_bank: Option<u32>,
    pub func: String,
    pub tracelen_um: Option<u32>,
    pub delay_min_fs: Option<u32>,
    pub delay_max_fs: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Part {
    pub part: String,
    pub family: String,
    pub source: Source,
    pub width: u16,
    pub height: u16,
    pub tile_kinds: EntityMap<TileKindId, String, TileKind>,
    pub tiles: HashMap<Coord, Tile>,
    pub speeds: EntitySet<SpeedId, String>,
    pub node_classes: EntitySet<NodeClassId, String>,
    pub nodes: EntityVec<NodeId, TkNode>,
    pub templates: EntitySet<TemplateId, Vec<TkNodeTemplateWire>>,
    pub wires: EntitySet<WireId, String>,
    pub slot_kinds: EntitySet<SlotKindId, String>,
    pub packages: HashMap<String, Vec<PkgPin>>,
    pub combos: Vec<PartCombo>,
}

impl Part {
    pub fn post_deserialize(&mut self) {
        for (ni, node) in self.nodes.iter() {
            let template = &self.templates[node.template];
            for w in template {
                let coord = Coord {
                    x: node.base.x + w.delta.x,
                    y: node.base.y + w.delta.y,
                };
                let tile = self.tiles.get_mut(&coord).unwrap();
                let tk = &self.tile_kinds[tile.kind];
                let wire = tk.wires.get(&w.wire).unwrap().1;
                let idx = match *wire {
                    TkWire::Internal(_, _) => panic!("node on internal wire"),
                    TkWire::Connected(idx) => idx,
                };
                tile.conn_wires.insert(idx, ni);
            }
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let f = File::open(path)?;
        let cf = zstd::stream::Decoder::new(f)?;
        let mut res: Part = bincode::deserialize_from(cf).unwrap();
        res.post_deserialize();
        Ok(res)
    }

    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let f = File::create(path)?;
        let mut cf = zstd::stream::Encoder::new(f, 9)?;
        bincode::serialize_into(&mut cf, self).unwrap();
        cf.finish()?;
        Ok(())
    }

    pub fn tiles_by_kind_name(&self, name: &str) -> &[Coord] {
        if let Some((_, tk)) = self.tile_kinds.get(name) {
            &tk.tiles
        } else {
            &[]
        }
    }
}

impl Tile {
    pub fn has_wire(&self, tk: &TileKind, w: WireId) -> bool {
        match tk.wires.get(&w) {
            None => false,
            Some((_, &TkWire::Internal(_, _))) => true,
            Some((_, &TkWire::Connected(idx))) => self.conn_wires.get(idx).is_some(),
        }
    }
}
