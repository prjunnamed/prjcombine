use std::collections::{BTreeSet, BTreeMap};
use serde::{Serialize, Deserialize};
use enum_map::Enum;
use prjcombine_entity::{EntityVec, EntityPartVec, EntityMap, entity_id};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Enum, Serialize, Deserialize)]
pub enum Dir {
    W, E, S, N,
}

impl Dir {
    pub const DIRS: [Dir; 4] = [
        Dir::W,
        Dir::E,
        Dir::S,
        Dir::N,
    ];
}

impl core::ops::Not for Dir {
    type Output = Dir;
    fn not(self) -> Dir {
        match self {
            Dir::W => Dir::E,
            Dir::E => Dir::W,
            Dir::S => Dir::N,
            Dir::N => Dir::S,
        }
    }
}

impl std::fmt::Display for Dir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Dir::W => "W",
            Dir::E => "E",
            Dir::S => "S",
            Dir::N => "N",
        })
    }
}

entity_id! {
    pub id WireId u16, reserve 1;
    pub id NodeKindId u16, reserve 1;
    pub id TermKindId u16, reserve 1;
    pub id IntfKindId u16, reserve 1;
    pub id BelKindId u16, reserve 1;
    pub id NodeNamingId u16, reserve 1;
    pub id TermNamingId u16, reserve 1;
    pub id IntfNamingId u16, reserve 1;
    pub id BelTileId u16, reserve 1;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntDb {
    pub name: String,
    pub wires: EntityVec<WireId, WireInfo>,
    pub nodes: EntityMap<NodeKindId, String, NodeKind>,
    pub terms: EntityMap<TermKindId, String, TermKind>,
    pub intfs: EntityMap<IntfKindId, String, IntfKind>,
    pub bels: EntityMap<BelKindId, String, BelKind>,
    pub node_namings: EntityMap<NodeNamingId, String, EntityPartVec<WireId, String>>,
    pub term_namings: EntityMap<TermNamingId, String, TermNaming>,
    pub intf_namings: EntityMap<IntfNamingId, String, IntfNaming>,
}

impl IntDb {
    pub fn get_node(&self, name: &str) -> NodeKindId {
        self.nodes.get(name).unwrap().0
    }
    pub fn get_term(&self, name: &str) -> TermKindId {
        self.terms.get(name).unwrap().0
    }
    pub fn get_intf(&self, name: &str) -> IntfKindId {
        self.intfs.get(name).unwrap().0
    }
    pub fn get_node_naming(&self, name: &str) -> NodeNamingId {
        self.node_namings.get(name).unwrap().0
    }
    pub fn get_term_naming(&self, name: &str) -> TermNamingId {
        self.term_namings.get(name).unwrap().0
    }
    pub fn get_intf_naming(&self, name: &str) -> IntfNamingId {
        self.intf_namings.get(name).unwrap().0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WireInfo {
    pub name: String,
    pub kind: WireKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum WireKind {
    Tie0,
    Tie1,
    TiePullup,
    ClkOut(usize),
    MuxOut,
    LogicOut,
    TestOut,
    MultiOut,
    PipOut,
    Buf(WireId),
    MultiBranch(Dir),
    PipBranch(Dir),
    Branch(Dir),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeKind {
    pub muxes: EntityPartVec<WireId, MuxInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MuxInfo {
    pub kind: MuxKind,
    pub ins: BTreeSet<WireId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum MuxKind {
    Plain,
    Inv,
    OptInv,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TermKind {
    pub dir: Dir,
    pub wires: EntityPartVec<WireId, TermInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TermInfo {
    BlackHole,
    Pass(TermWireIn),
    BiSplitter(TermWireIn),
    Mux(BTreeSet<TermWireIn>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum TermWireIn {
    Near(WireId),
    Far(WireId),
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct TermNaming {
    pub wires_out: EntityPartVec<WireId, TermWireOutNaming>,
    pub wires_in_near: EntityPartVec<WireId, String>,
    pub wires_in_far: EntityPartVec<WireId, TermWireInFarNaming>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TermWireOutNaming {
    Simple(String),
    Buf(String, String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TermWireInFarNaming {
    Simple(String),
    Buf(String, String),
    BufFar(String, String, String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntfKind {
    pub wires: EntityPartVec<WireId, IntfInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IntfInfo {
    OutputTestMux(BTreeSet<WireId>),
    InputDelay,
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct IntfNaming {
    pub wires_out: EntityPartVec<WireId, IntfWireOutNaming>,
    pub wires_in: EntityPartVec<WireId, IntfWireInNaming>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IntfWireOutNaming {
    Simple(String),
    Buf(String, String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IntfWireInNaming {
    Simple(String),
    TestBuf(String, String),
    Delay(String, String, String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BelKind {
    pub ports: BTreeMap<String, BelPort>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BelPort {
    pub tile_idx: BelTileId,
    pub wire: WireId,
}
