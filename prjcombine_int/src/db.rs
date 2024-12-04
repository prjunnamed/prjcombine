use enum_map::Enum;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use unnamed_entity::{entity_id, EntityMap, EntityPartVec, EntityVec};

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
    pub id NodeTileId u16, reserve 1;
    pub id NodeIriId u16, reserve 1;
    pub id BelId u16, reserve 1;
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntDb {
    pub wires: EntityMap<WireId, String, WireKind>,
    pub nodes: EntityMap<NodeKindId, String, NodeKind>,
    pub terms: EntityMap<TermKindId, String, TermKind>,
}

impl IntDb {
    #[track_caller]
    pub fn get_wire(&self, name: &str) -> WireId {
        self.wires
            .get(name)
            .unwrap_or_else(|| panic!("no wire {name}"))
            .0
    }
    #[track_caller]
    pub fn get_node(&self, name: &str) -> NodeKindId {
        self.nodes
            .get(name)
            .unwrap_or_else(|| panic!("no node {name}"))
            .0
    }
    #[track_caller]
    pub fn get_term(&self, name: &str) -> TermKindId {
        self.terms
            .get(name)
            .unwrap_or_else(|| panic!("no term {name}"))
            .0
    }
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

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
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
    Inout,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IntfInfo {
    OutputTestMux(BTreeSet<NodeWireId>),
    OutputTestMuxPass(BTreeSet<NodeWireId>, NodeWireId),
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TermKind {
    pub dir: Dir,
    pub wires: EntityPartVec<WireId, TermInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum TermInfo {
    BlackHole,
    PassNear(WireId),
    PassFar(WireId),
}

#[derive(Clone, Debug)]
pub struct IntDbIndex {
    pub nodes: EntityVec<NodeKindId, NodeIndex>,
    pub terms: EntityVec<TermKindId, TermIndex>,
    pub buf_ins: EntityVec<WireId, HashSet<WireId>>,
}

#[derive(Clone, Debug)]
pub struct NodeIndex {
    pub mux_ins: HashMap<NodeWireId, HashSet<NodeWireId>>,
    pub intf_ins: HashMap<NodeWireId, HashSet<NodeWireId>>,
    pub intf_ins_pass: HashMap<NodeWireId, HashSet<NodeWireId>>,
}

#[derive(Clone, Debug)]
pub struct TermIndex {
    pub wire_ins_far: EntityVec<WireId, HashSet<WireId>>,
    pub wire_ins_near: EntityVec<WireId, HashSet<WireId>>,
}

impl IntDbIndex {
    pub fn new(db: &IntDb) -> Self {
        let mut buf_ins: EntityVec<_, _> = db.wires.ids().map(|_| HashSet::new()).collect();
        for (w, _, wd) in &db.wires {
            if let WireKind::Buf(wi) = *wd {
                buf_ins[wi].insert(w);
            }
        }
        Self {
            nodes: db.nodes.values().map(NodeIndex::new).collect(),
            terms: db.terms.values().map(|t| TermIndex::new(t, db)).collect(),
            buf_ins,
        }
    }
}

impl NodeIndex {
    pub fn new(node: &NodeKind) -> Self {
        let mut mux_ins: HashMap<_, HashSet<_>> = HashMap::new();
        for (&wo, mux) in &node.muxes {
            for &wi in &mux.ins {
                mux_ins.entry(wi).or_default().insert(wo);
            }
        }

        let mut intf_ins: HashMap<_, HashSet<_>> = HashMap::new();
        let mut intf_ins_pass: HashMap<_, HashSet<_>> = HashMap::new();
        for (&wo, intf) in &node.intfs {
            match *intf {
                IntfInfo::OutputTestMux(ref ins) => {
                    for &wi in ins {
                        intf_ins.entry(wi).or_default().insert(wo);
                    }
                }
                IntfInfo::OutputTestMuxPass(ref ins, main_in) => {
                    for &wi in ins {
                        intf_ins.entry(wi).or_default().insert(wo);
                    }
                    intf_ins_pass.entry(main_in).or_default().insert(wo);
                }
                _ => (),
            }
        }

        NodeIndex {
            mux_ins,
            intf_ins,
            intf_ins_pass,
        }
    }
}

impl TermIndex {
    pub fn new(term: &TermKind, db: &IntDb) -> Self {
        let mut wire_ins_far: EntityVec<_, _> = db.wires.ids().map(|_| HashSet::new()).collect();
        let mut wire_ins_near: EntityVec<_, _> = db.wires.ids().map(|_| HashSet::new()).collect();
        for (wo, ti) in &term.wires {
            match *ti {
                TermInfo::BlackHole => (),
                TermInfo::PassNear(wi) => {
                    wire_ins_near[wi].insert(wo);
                }
                TermInfo::PassFar(wi) => {
                    wire_ins_far[wi].insert(wo);
                }
            }
        }
        TermIndex {
            wire_ins_far,
            wire_ins_near,
        }
    }
}

impl IntDb {
    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "wires": Vec::from_iter(self.wires.iter().map(|(_, name, wire)| {
                json!({
                    "name": name,
                    "kind": match wire {
                        WireKind::Tie0 => "TIE_0".into(),
                        WireKind::Tie1 => "TIE_1".into(),
                        WireKind::TiePullup => "TIE_PULLUP".into(),
                        WireKind::ClkOut(idx) => format!("CLKOUT{idx}"),
                        WireKind::MuxOut => "MUX_OUT".into(),
                        WireKind::LogicOut => "LOGIC_OUT".into(),
                        WireKind::TestOut => "TEST_OUT".into(),
                        WireKind::MultiOut => "MULTI_OUT".into(),
                        WireKind::PipOut => "PIP_OUT".into(),
                        WireKind::Buf(wire_id) => format!("BUF:{}", self.wires.key(*wire_id)),
                        WireKind::MultiBranch(dir) => format!("MULTI_BRANCH:{dir}"),
                        WireKind::PipBranch(dir) => format!("PIP_BRANCH:{dir}"),
                        WireKind::Branch(dir) => format!("BRANCH:{dir}"),
                    }
                })
            })),
            "nodes": serde_json::Map::from_iter(self.nodes.iter().map(|(_, name, node)| {
                (name.into(), json!({
                    "tiles": node.tiles.len(),
                    "muxes": serde_json::Map::from_iter(node.muxes.iter().map(|(wt, mux)| {
                        (
                            format!("{}:{}", wt.0, self.wires.key(wt.1)),
                            json!({
                                "kind": match mux.kind {
                                    MuxKind::Plain => "PLAIN",
                                    MuxKind::Inv => "INV",
                                    MuxKind::OptInv => "OPTINV",
                                },
                                "ins": Vec::from_iter(mux.ins.iter().map(|wf| format!(
                                    "{}:{}", wf.0, self.wires.key(wf.1)
                                ))),
                            })
                        )
                    })),
                    "iris": node.iris.len(),
                    "intfs": serde_json::Map::from_iter(node.intfs.iter().map(|(wt, intf)| {
                        (
                            format!("{}:{}", wt.0, self.wires.key(wt.1)),
                            match intf {
                                IntfInfo::OutputTestMux(ins) => json!({
                                    "kind": "OUTPUT_TEST_MUX",
                                    "ins": Vec::from_iter(ins.iter().map(|wf| format!(
                                        "{}:{}", wf.0, self.wires.key(wf.1)
                                    ))),
                                }),
                                IntfInfo::OutputTestMuxPass(ins, def) => json!({
                                    "kind": "OUTPUT_TEST_MUX_PASS",
                                    "ins": Vec::from_iter(ins.iter().map(|wf| format!(
                                        "{}:{}", wf.0, self.wires.key(wf.1)
                                    ))),
                                    "default": format!("{}:{}", def.0, self.wires.key(def.1)),
                                }),
                                IntfInfo::InputDelay => json!({
                                    "kind": "INPUT_DELAY",
                                }),
                                IntfInfo::InputIri(iri, pin) => json!({
                                    "kind": "INPUT_IRI",
                                    "iri": iri,
                                    "pin": match pin {
                                        IriPin::Clk => "CLK".to_string(),
                                        IriPin::Rst => "RST".to_string(),
                                        IriPin::Ce(i) => format!("CE{i}"),
                                        IriPin::Imux(i) => format!("IMUX{i}"),
                                    },
                                }),
                                IntfInfo::InputIriDelay(iri, pin) => json!({
                                    "kind": "INPUT_IRI_DELAY",
                                    "iri": iri,
                                    "pin": match pin {
                                        IriPin::Clk => "CLK".to_string(),
                                        IriPin::Rst => "RST".to_string(),
                                        IriPin::Ce(i) => format!("CE{i}"),
                                        IriPin::Imux(i) => format!("IMUX{i}"),
                                    },
                                }),
                            }
                        )
                    })),
                    "bels": Vec::from_iter(node.bels.iter().map(|(_, name, bel)| json!({
                        "name": name,
                        "pins": serde_json::Map::from_iter(bel.pins.iter().map(|(pname, pin)| (pname.to_string(), json!({
                            "wires": Vec::from_iter(pin.wires.iter().map(|wf| format!(
                                "{}:{}", wf.0, self.wires.key(wf.1)
                            ))),
                            "dir": match pin.dir {
                                PinDir::Input => "INPUT",
                                PinDir::Output => "OUTPUT",
                                PinDir::Inout => "INOUT",
                            },
                            "is_intf_in": pin.is_intf_in,
                        })))),
                    }))),
                }))
            })),
            "terms": serde_json::Map::from_iter(self.terms.iter().map(|(_, name, term)| {
                (name.into(), json!({
                    "dir": term.dir.to_string(),
                    "wires": serde_json::Map::from_iter(term.wires.iter().map(|(wire, ti)|
                        (self.wires.key(wire).to_string(), match *ti {
                            TermInfo::BlackHole => json!({
                                "kind": "BLACKHOLE",
                            }),
                            TermInfo::PassNear(wf) => json!({
                                "kind": "PASS_NEAR",
                                "wire": self.wires.key(wf),
                            }),
                            TermInfo::PassFar(wf) => json!({
                                "kind": "PASS_FAR",
                                "wire": self.wires.key(wf),
                            }),
                        })
                    ))
                }))
            })),
        })
    }
}
