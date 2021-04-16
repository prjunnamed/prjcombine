use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use crate::error::Error;
use serde::{Serialize, Deserialize};

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone, Serialize, Deserialize)]
pub struct Coord {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone, Serialize, Deserialize)]
pub struct WireIdx {
    idx: u32,
}

impl WireIdx {
    pub const NONE: WireIdx = WireIdx { idx: u32::MAX };
    pub fn from_raw(i: usize) -> WireIdx {
        assert!(i < u32::MAX as usize);
        WireIdx {idx: i as u32}
    }
    pub fn pack(v: Option<usize>) -> Self {
        match v {
            None => WireIdx::NONE,
            Some(idx) => WireIdx::from_raw(idx),
        }
    }
    pub fn unpack(&self) -> Option<usize> {
        if *self == WireIdx::NONE {
            None
        } else {
            Some(self.idx as usize)
        }
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone, Serialize, Deserialize)]
pub struct NodeClassIdx {
    idx: u32,
}

impl NodeClassIdx {
    pub const UNKNOWN: NodeClassIdx = NodeClassIdx { idx: u32::MAX };
    pub fn from_raw(i: usize) -> NodeClassIdx {
        assert!(i < u32::MAX as usize);
        NodeClassIdx {idx: i as u32}
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone, Serialize, Deserialize)]
pub struct SpeedIdx {
    idx: u32,
}

impl SpeedIdx {
    pub const NONE: SpeedIdx = SpeedIdx { idx: u32::MAX };
    pub const UNKNOWN: SpeedIdx = SpeedIdx { idx: u32::MAX - 1 };
    pub fn from_raw(i: usize) -> SpeedIdx {
        assert!(i < (u32::MAX - 1) as usize);
        SpeedIdx {idx: i as u32}
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum NodeOrClass {
    Node(u32),
    Pending(NodeClassIdx),
    None,
}

impl NodeOrClass {
    pub fn make_node(idx: usize) -> NodeOrClass {
        assert!(idx <= u32::MAX as usize);
        NodeOrClass::Node(idx as u32)
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum Source {
    ISE,
    Vivado,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum TkSiteSlot {
    Single(u16),
    Indexed(u16, u8),
    Xy(u16, u8, u8),
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
    pub wire: WireIdx,
    pub speed: SpeedIdx,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct TkSite {
    pub slot: TkSiteSlot,
    pub kind: String,
    pub pins: HashMap<String, TkSitePin>,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum TkWire {
    Internal(SpeedIdx, NodeClassIdx),
    Connected(usize),
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
    pub speed: SpeedIdx,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct TileKind {
    pub sites: Vec<TkSite>,
    pub sites_by_slot: HashMap<TkSiteSlot, usize>,
    pub wires: HashMap<WireIdx, TkWire>,
    pub conn_wires: Vec<WireIdx>,
    pub pips: HashMap<(WireIdx, WireIdx), TkPip>,
    pub tiles: Vec<Coord>,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Tile {
    pub name: String,
    pub kind: String,
    pub sites: Vec<Option<String>>,
    #[serde(skip)]
    pub conn_wires: Vec<NodeOrClass>,
    pub pip_overrides: HashMap<(WireIdx, WireIdx), (NodeClassIdx, NodeClassIdx)>,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub struct TkNode {
    pub base: Coord,
    pub template: u32,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone, Serialize, Deserialize)]
pub struct TkNodeTemplateWire {
    pub delta: Coord,
    pub wire: WireIdx,
    pub speed: SpeedIdx,
    pub cls: NodeClassIdx,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Serialize, Deserialize)]
pub struct TkNodeTemplate {
    pub wires: Vec<TkNodeTemplateWire>,
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
    pub tile_kinds: HashMap<String, TileKind>,
    pub tiles: HashMap<Coord, Tile>,
    pub speeds: Vec<String>,
    pub node_classes: Vec<String>,
    pub nodes: Vec<TkNode>,
    pub templates: Vec<TkNodeTemplate>,
    pub wires: Vec<String>,
    pub slot_kinds: Vec<String>,
    pub packages: HashMap<String, Vec<PkgPin>>,
    pub combos: Vec<PartCombo>,
}

impl Part {
    pub fn print_wire(&self, w: WireIdx) -> &str {
        if w == WireIdx::NONE {
            "[NONE]"
        } else {
            &self.wires[w.idx as usize]
        }
    }

    pub fn print_speed(&self, s: SpeedIdx) -> &str {
        if s == SpeedIdx::NONE {
            "[NONE]"
        } else if s == SpeedIdx::UNKNOWN {
            "[UNKNOWN]"
        } else {
            &self.speeds[s.idx as usize]
        }
    }

    pub fn print_slot_kind(&self, sk: u16) -> &str {
        &self.slot_kinds[sk as usize]
    }

    pub fn print_node_class(&self, nc: NodeClassIdx) -> &str {
        if nc == NodeClassIdx::UNKNOWN {
            "[UNKNOWN]"
        } else {
            &self.node_classes[nc.idx as usize]
        }
    }

    pub fn post_deserialize(&mut self) {
        for (i, node) in self.nodes.iter().enumerate() {
            let template = &self.templates[node.template as usize];
            for w in template.wires.iter() {
                let coord = Coord {x: node.base.x + w.delta.x, y: node.base.y + w.delta.y};
                let tile = self.tiles.get_mut(&coord).unwrap();
                let tk = self.tile_kinds.get(&tile.kind).unwrap();
                let wire = tk.wires.get(&w.wire).unwrap();
                let idx = match wire {
                    TkWire::Internal(_, _) => panic!("node on internal wire"),
                    TkWire::Connected(idx) => *idx,
                };
                tile.set_conn_wire(idx, NodeOrClass::make_node(i));
            }
        }
    }

    pub fn from_file<P: AsRef<Path>> (path: P) -> Result<Self, Error> {
        let f = File::open(path)?;
        let cf = zstd::stream::Decoder::new(f)?;
        let mut res: Part = bincode::deserialize_from(cf).unwrap();
        res.post_deserialize();
        Ok(res)
    }

    pub fn to_file<P: AsRef<Path>> (&self, path: P) -> Result<(), Error> {
        let f = File::create(path)?;
        let mut cf = zstd::stream::Encoder::new(f, 9)?;
        bincode::serialize_into(&mut cf, self).unwrap();
        cf.finish()?;
        Ok(())
    }

    pub fn all_wires(&self) -> impl Iterator<Item = WireIdx> {
        (0..self.wires.len()).map(WireIdx::from_raw)
    }
}

impl Tile {
    pub fn set_conn_wire(&mut self, idx: usize, val: NodeOrClass) {
        if self.conn_wires.len() <= idx {
            self.conn_wires.resize(idx + 1, NodeOrClass::None);
        }
        match (self.conn_wires[idx], val) {
            (NodeOrClass::Node(_), _) => panic!("conn wire double set {}", self.name),
            (_, NodeOrClass::None) => panic!("removing wire {}", self.name),
            (NodeOrClass::Pending(_), NodeOrClass::Pending(_)) => panic!("conn wire double pending {}", self.name),
            _ => (),
        }
        self.conn_wires[idx] = val;
    }
    pub fn get_conn_wire(&self, idx: usize) -> NodeOrClass {
        match self.conn_wires.get(idx) {
            None => NodeOrClass::None,
            Some(ni) => *ni,
        }
    }
    pub fn has_wire(&self, tk: &TileKind, w: WireIdx) -> bool {
        match tk.wires.get(&w) {
            None => false,
            Some(TkWire::Internal(_, _)) => true,
            Some(TkWire::Connected(idx)) => {
                match self.conn_wires.get(*idx) {
                    None => false,
                    Some(ni) => *ni != NodeOrClass::None,
                }
            }
        }
    }
}
