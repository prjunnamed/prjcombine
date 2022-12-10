use enum_map::Enum;
use prjcombine_entity::{entity_id, EntityMap, EntityPartVec, EntityVec};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Enum, Serialize, Deserialize,
)]
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
    pub id NodeNamingId u16, reserve 1;
    pub id TermNamingId u16, reserve 1;
    pub id NodeTileId u16, reserve 1;
    pub id NodeRawTileId u16, reserve 1;
    pub id NodeIriId u16, reserve 1;
    pub id BelId u16, reserve 1;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntDb {
    pub name: String,
    pub wires: EntityVec<WireId, WireInfo>,
    pub nodes: EntityMap<NodeKindId, String, NodeKind>,
    pub terms: EntityMap<TermKindId, String, TermKind>,
    pub node_namings: EntityMap<NodeNamingId, String, NodeNaming>,
    pub term_namings: EntityMap<TermNamingId, String, TermNaming>,
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
    pub fn get_node_naming(&self, name: &str) -> NodeNamingId {
        self.node_namings.get(name).unwrap().0
    }
    #[track_caller]
    pub fn get_term_naming(&self, name: &str) -> TermNamingId {
        self.term_namings.get(name).unwrap().0
    }

    pub fn merge(&mut self, other: IntDb) {
        assert_eq!(self.wires, other.wires);
        macro_rules! merge_dicts {
            ($f:ident) => {
                for (_, k, v) in other.$f {
                    match self.$f.get(&k) {
                        None => {
                            self.$f.insert(k, v);
                        }
                        Some((_, v2)) => {
                            if v != *v2 {
                                println!("FAIL at {}", k);
                            }
                            assert_eq!(&v, v2);
                        }
                    }
                }
            };
        }
        merge_dicts!(nodes);
        merge_dicts!(terms);
        for (_, k, v) in other.node_namings {
            match self.node_namings.get_mut(&k) {
                None => {
                    self.node_namings.insert(k, v);
                }
                Some((_, v2)) => {
                    for (kk, vv) in v.wires {
                        match v2.wires.get(&kk) {
                            None => {
                                v2.wires.insert(kk, vv);
                            }
                            Some(vv2) => {
                                assert_eq!(&vv, vv2);
                            }
                        }
                    }
                    assert_eq!(v.wire_bufs, v2.wire_bufs);
                    assert_eq!(v.ext_pips, v2.ext_pips);
                    assert_eq!(v.bels, v2.bels);
                    for (kk, vv) in v.intf_wires_in {
                        match v2.intf_wires_in.get(&kk) {
                            None => {
                                v2.intf_wires_in.insert(kk, vv);
                            }
                            Some(vv2) => {
                                assert_eq!(&vv, vv2);
                            }
                        }
                    }
                    for (kk, vv) in v.intf_wires_out {
                        match v2.intf_wires_out.get(&kk) {
                            None => {
                                v2.intf_wires_out.insert(kk, vv);
                            }
                            Some(vv2 @ IntfWireOutNaming::Buf { name_out, .. }) => match vv {
                                IntfWireOutNaming::Buf { .. } => assert_eq!(&vv, vv2),
                                IntfWireOutNaming::Simple { name } => assert_eq!(name_out, &name),
                            },
                            Some(vv2 @ IntfWireOutNaming::Simple { name }) => {
                                if let IntfWireOutNaming::Buf { name_out, .. } = &vv {
                                    assert_eq!(name_out, name);
                                    v2.intf_wires_out.insert(kk, vv);
                                } else {
                                    assert_eq!(&vv, vv2);
                                }
                            }
                        }
                    }
                }
            }
        }
        merge_dicts!(term_namings);
    }

    pub fn get_wire(&self, n: &str) -> WireId {
        self.wires.iter().find(|(_, w)| w.name == n).unwrap().0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WireInfo {
    pub name: String,
    pub kind: WireKind,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
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

impl WireKind {
    pub fn is_tie(self) -> bool {
        matches!(self, WireKind::Tie0 | WireKind::Tie1 | WireKind::TiePullup)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeKind {
    pub tiles: EntityVec<NodeTileId, ()>,
    pub muxes: BTreeMap<NodeWireId, MuxInfo>,
    pub iris: EntityVec<NodeIriId, ()>,
    pub intfs: BTreeMap<NodeWireId, IntfInfo>,
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
    pub is_intf_in: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PinDir {
    Input,
    Output,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IntfInfo {
    OutputTestMux(BTreeSet<NodeWireId>),
    InputDelay,
    InputIri(NodeIriId, IriPin),
    InputIriDelay(NodeIriId, IriPin),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum IriPin {
    Clk,
    Rst,
    Ce(u32),
    Imux(u32),
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct NodeNaming {
    pub wires: BTreeMap<NodeWireId, String>,
    pub wire_bufs: BTreeMap<NodeWireId, NodeExtPipNaming>,
    pub ext_pips: BTreeMap<(NodeWireId, NodeWireId), NodeExtPipNaming>,
    pub bels: EntityVec<BelId, BelNaming>,
    pub iris: EntityVec<NodeIriId, IriNaming>,
    pub intf_wires_out: BTreeMap<NodeWireId, IntfWireOutNaming>,
    pub intf_wires_in: BTreeMap<NodeWireId, IntfWireInNaming>,
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
    pub is_intf_out: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IriNaming {
    pub tile: NodeRawTileId,
    pub kind: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IntfWireOutNaming {
    Simple { name: String },
    Buf { name_out: String, name_in: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IntfWireInNaming {
    Simple {
        name: String,
    },
    Buf {
        name_out: String,
        name_in: String,
    },
    TestBuf {
        name_out: String,
        name_in: String,
    },
    Delay {
        name_out: String,
        name_in: String,
        name_delay: String,
    },
    Iri {
        name_out: String,
        name_pin_out: String,
        name_pin_in: String,
        name_in: String,
    },
    IriDelay {
        name_out: String,
        name_delay: String,
        name_pre_delay: String,
        name_pin_out: String,
        name_pin_in: String,
        name_in: String,
    },
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
    Simple { name: String },
    Buf { name_out: String, name_in: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TermWireInFarNaming {
    Simple {
        name: String,
    },
    Buf {
        name_out: String,
        name_in: String,
    },
    BufFar {
        name: String,
        name_far_out: String,
        name_far_in: String,
    },
}
