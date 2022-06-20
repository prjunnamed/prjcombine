use std::collections::{BTreeSet, BTreeMap};
use serde::{Serialize, Deserialize};
use prjcombine_entity::{EntityVec, EntityPartVec, EntityMap, entity_id};
use enum_map::{EnumMap, Enum};

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
    MultiOut,
    Buf(WireId),
    MultiBranch(Dir),
    Branch(Dir),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeKind {
    pub muxes: EntityPartVec<WireId, MuxInfo>,
    pub ptrans: BTreeSet<(WireId, WireId)>,
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


/*
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedInt {
    pub int_grid: Array2D<ExpandedTile>,
}
*/

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedTile {
    pub kind: NodeKindId,
    pub name: String,
    pub tie_name: Option<String>,
    pub wire_naming: NamingId,
    pub intf: Option<ExpandedTileIntf>,
    pub dirs: EnumMap<Dir, ExpandedTileDir>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedTileIntf {
    pub kind: IntfKindId,
    pub name: String,
    pub wire_naming_int: NamingId,
    pub wire_naming_delay: NamingId,
    pub wire_naming_site: NamingId,
}

pub type Coord = (u32, u32);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExpandedTileDir {
    None,
    TermAnon {
        kind: TermKindId,
    },
    Term {
        kind: TermKindId,
        name: String,
        wire_naming: NamingId,
    },
    TermBuf {
        kind: TermKindId,
        name: String,
        wire_naming_out: NamingId,
        wire_naming_in: NamingId,
    },
    PassAnon {
        target: Coord,
        kind: PassKindId,
    },
    PassSingle {
        target: Coord,
        kind: PassKindId,
        name: String,
        wire_naming_near: NamingId,
        wire_naming_far: NamingId,
    },
    PassDouble {
        target: Coord,
        kind: PassKindId,
        name_near: String,
        wire_naming_near_near: NamingId,
        wire_naming_near_far: NamingId,
        name_far: String,
        wire_naming_far_out: NamingId,
        wire_naming_far_in: NamingId,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedBel {
    pub name: String,
    pub tile_name: String,
    pub tiles: EntityVec<BelTileId, ExpandedBelTile>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedBelTile {
    pub coord: Coord,
    pub wire_naming: (NamingId, NamingId),
    pub int_special_naming: Option<NamingId>,
}
