use crate::BelId;
use enum_map::Enum;
use prjcombine_entity::{entity_id, EntityMap, EntityPartVec, EntityVec};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Enum, Serialize, Deserialize)]
pub enum Dir {
    W,
    E,
    S,
    N,
}

impl Dir {
    pub const DIRS: [Dir; 4] = [Dir::W, Dir::E, Dir::S, Dir::N];
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
        write!(
            f,
            "{}",
            match self {
                Dir::W => "W",
                Dir::E => "E",
                Dir::S => "S",
                Dir::N => "N",
            }
        )
    }
}

entity_id! {
    pub id WireId u16, reserve 1;
    pub id NodeKindId u16, reserve 1;
    pub id TermKindId u16, reserve 1;
    pub id IntfKindId u16, reserve 1;
    pub id NodeNamingId u16, reserve 1;
    pub id TermNamingId u16, reserve 1;
    pub id IntfNamingId u16, reserve 1;
    pub id NodeTileId u16, reserve 1;
    pub id NodeRawTileId u16, reserve 1;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntDb {
    pub name: String,
    pub wires: EntityVec<WireId, WireInfo>,
    pub nodes: EntityMap<NodeKindId, String, NodeKind>,
    pub terms: EntityMap<TermKindId, String, TermKind>,
    pub intfs: EntityMap<IntfKindId, String, IntfKind>,
    pub node_namings: EntityMap<NodeNamingId, String, NodeNaming>,
    pub term_namings: EntityMap<TermNamingId, String, TermNaming>,
    pub intf_namings: EntityMap<IntfNamingId, String, IntfNaming>,
}

impl IntDb {
    #[track_caller]
    pub fn get_node(&self, name: &str) -> NodeKindId {
        self.nodes.get(name).unwrap().0
    }
    #[track_caller]
    pub fn get_term(&self, name: &str) -> TermKindId {
        self.terms.get(name).unwrap().0
    }
    #[track_caller]
    pub fn get_intf(&self, name: &str) -> IntfKindId {
        self.intfs.get(name).unwrap().0
    }
    #[track_caller]
    pub fn get_node_naming(&self, name: &str) -> NodeNamingId {
        self.node_namings.get(name).unwrap().0
    }
    #[track_caller]
    pub fn get_term_naming(&self, name: &str) -> TermNamingId {
        self.term_namings.get(name).unwrap().0
    }
    #[track_caller]
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
    CondAlias(NodeKindId, WireId),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeKind {
    pub tiles: EntityVec<NodeTileId, ()>,
    pub muxes: BTreeMap<NodeWireId, MuxInfo>,
    pub bels: EntityMap<BelId, String, BelInfo>,
}

pub type NodeWireId = (NodeTileId, WireId);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MuxInfo {
    pub kind: MuxKind,
    pub ins: BTreeSet<NodeWireId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum MuxKind {
    Plain,
    Inv,
    OptInv,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BelInfo {
    pub pins: BTreeMap<String, BelPin>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct BelPin {
    pub wires: BTreeSet<NodeWireId>,
    pub dir: PinDir,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PinDir {
    Input,
    Output,
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct NodeNaming {
    pub wires: BTreeMap<NodeWireId, String>,
    pub wire_bufs: BTreeMap<NodeWireId, NodeExtPipNaming>,
    pub ext_pips: BTreeMap<(NodeWireId, NodeWireId), NodeExtPipNaming>,
    pub bels: EntityVec<BelId, BelNaming>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeExtPipNaming {
    pub tile: NodeRawTileId,
    pub wire_to: String,
    pub wire_from: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BelNaming {
    pub tile: NodeRawTileId,
    pub pins: BTreeMap<String, BelPinNaming>,
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct BelPinNaming {
    pub name: String,
    pub name_far: String,
    pub pips: Vec<NodeExtPipNaming>,
    pub int_pips: BTreeMap<NodeWireId, NodeExtPipNaming>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TermKind {
    pub dir: Dir,
    pub wires: EntityPartVec<WireId, TermInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TermInfo {
    BlackHole,
    PassNear(WireId),
    PassFar(WireId),
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
