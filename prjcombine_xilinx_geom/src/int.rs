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
    pub id PassKindId u16, reserve 1;
    pub id IntfKindId u16, reserve 1;
    pub id BelKindId u16, reserve 1;
    pub id NamingId u16, reserve 1;
    pub id BelTileId u16, reserve 1;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntDb {
    pub name: String,
    pub wires: EntityVec<WireId, WireInfo>,
    pub nodes: EntityMap<NodeKindId, String, NodeKind>,
    pub terms: EntityMap<TermKindId, String, TermKind>,
    pub passes: EntityMap<PassKindId, String, PassKind>,
    pub intfs: EntityMap<IntfKindId, String, IntfKind>,
    pub bels: EntityMap<BelKindId, String, BelKind>,
    pub namings: EntityMap<NamingId, String, EntityPartVec<WireId, String>>,
}

impl IntDb {
    pub fn get_node(&self, name: &str) -> NodeKindId {
        self.nodes.get(name).unwrap().0
    }
    pub fn get_term(&self, name: &str) -> TermKindId {
        self.terms.get(name).unwrap().0
    }
    pub fn get_pass(&self, name: &str) -> PassKindId {
        self.passes.get(name).unwrap().0
    }
    pub fn get_intf(&self, name: &str) -> IntfKindId {
        self.intfs.get(name).unwrap().0
    }
    pub fn get_naming(&self, name: &str) -> NamingId {
        self.namings.get(name).unwrap().0
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
    Pass(WireId),
    Mux(BTreeSet<WireId>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PassKind {
    pub dir: Dir,
    pub wires: EntityPartVec<WireId, PassInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PassInfo {
    BlackHole,
    Pass(PassWireIn),
    BiSplitter(PassWireIn),
    Mux(BTreeSet<PassWireIn>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum PassWireIn {
    Near(WireId),
    Far(WireId),
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BelKind {
    pub ports: BTreeMap<String, BelPort>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BelPort {
    pub tile_idx: BelTileId,
    pub wire: WireId,
}
