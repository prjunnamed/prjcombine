use bincode::{Decode, Encode};
use jzon::JsonValue;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use unnamed_entity::{
    EntityId, EntityMap, EntityPartVec, EntitySet, EntityVec,
    id::{EntityIdU8, EntityIdU16, EntityTag},
};

impl EntityTag for WireKind {
    const PREFIX: &'static str = "WIRE";
}
impl EntityTag for TileClass {
    const PREFIX: &'static str = "TCLS";
}
impl EntityTag for ConnectorSlot {
    const PREFIX: &'static str = "CSLOT";
}
pub struct RegionSlotTag;
impl EntityTag for RegionSlotTag {
    const PREFIX: &'static str = "RSLOT";
}
impl EntityTag for ConnectorClass {
    const PREFIX: &'static str = "CCLS";
}
pub struct TileCellTag;
impl EntityTag for TileCellTag {
    const PREFIX: &'static str = "TCELL";
}
pub struct TileIriTag;
impl EntityTag for TileIriTag {
    const PREFIX: &'static str = "IRI";
}
pub type WireId = EntityIdU16<WireKind>;
pub type TileClassId = EntityIdU16<TileClass>;
pub type RegionSlotId = EntityIdU8<RegionSlotTag>;
pub type ConnectorSlotId = EntityIdU8<ConnectorSlot>;
pub type ConnectorClassId = EntityIdU16<ConnectorClass>;
pub type TileCellId = EntityIdU16<TileCellTag>;
pub type TileIriId = EntityIdU16<TileIriTag>;

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode,
)]
pub struct BelSlotId(u16);

impl BelSlotId {
    pub const fn from_idx_const(idx: usize) -> Self {
        assert!(idx <= 0xffff);
        Self(idx as u16)
    }

    pub const fn to_idx_const(self) -> usize {
        self.0 as usize
    }
}

impl EntityId for BelSlotId {
    fn from_idx(idx: usize) -> Self {
        Self(idx.try_into().unwrap())
    }

    fn to_idx(self) -> usize {
        self.0.into()
    }
}

impl std::fmt::Debug for BelSlotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BEL{}", self.0)
    }
}

impl std::fmt::Display for BelSlotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "{}", self.0)
        } else {
            write!(f, "BEL{}", self.0)
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Encode, Decode)]
pub struct IntDb {
    pub wires: EntityMap<WireId, String, WireKind>,
    pub bel_slots: EntitySet<BelSlotId, String>,
    pub region_slots: EntitySet<RegionSlotId, String>,
    pub tile_classes: EntityMap<TileClassId, String, TileClass>,
    pub conn_slots: EntityMap<ConnectorSlotId, String, ConnectorSlot>,
    pub conn_classes: EntityMap<ConnectorClassId, String, ConnectorClass>,
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
    pub fn get_bel_slot(&self, name: &str) -> BelSlotId {
        self.bel_slots
            .get(name)
            .unwrap_or_else(|| panic!("no bel slot {name}"))
    }
    #[track_caller]
    pub fn get_tile_class(&self, name: &str) -> TileClassId {
        self.tile_classes
            .get(name)
            .unwrap_or_else(|| panic!("no tile class {name}"))
            .0
    }
    #[track_caller]
    pub fn get_conn_class(&self, name: &str) -> ConnectorClassId {
        self.conn_classes
            .get(name)
            .unwrap_or_else(|| panic!("no connector class {name}"))
            .0
    }
    #[track_caller]
    pub fn get_conn_slot(&self, name: &str) -> ConnectorSlotId {
        self.conn_slots
            .get(name)
            .unwrap_or_else(|| panic!("no connector slot {name}"))
            .0
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum WireKind {
    Tie0,
    Tie1,
    TiePullup,
    Regional(RegionSlotId),
    MuxOut,
    LogicOut,
    TestOut,
    MultiOut,
    PipOut,
    Buf(WireId),
    MultiBranch(ConnectorSlotId),
    PipBranch(ConnectorSlotId),
    Branch(ConnectorSlotId),
}

impl WireKind {
    pub fn to_string(&self, db: &IntDb) -> String {
        match self {
            WireKind::Tie0 => "TIE_0".into(),
            WireKind::Tie1 => "TIE_1".into(),
            WireKind::TiePullup => "TIE_PULLUP".into(),
            WireKind::Regional(slot) => format!("REGIONAL:{}", db.region_slots[*slot]),
            WireKind::MuxOut => "MUX_OUT".into(),
            WireKind::LogicOut => "LOGIC_OUT".into(),
            WireKind::TestOut => "TEST_OUT".into(),
            WireKind::MultiOut => "MULTI_OUT".into(),
            WireKind::PipOut => "PIP_OUT".into(),
            WireKind::Buf(wire_id) => format!("BUF:{}", db.wires.key(*wire_id)),
            WireKind::MultiBranch(slot) => {
                format!("MULTI_BRANCH:{slot}", slot = db.conn_slots.key(*slot))
            }
            WireKind::PipBranch(slot) => {
                format!("PIP_BRANCH:{slot}", slot = db.conn_slots.key(*slot))
            }
            WireKind::Branch(slot) => format!("BRANCH:{slot}", slot = db.conn_slots.key(*slot)),
        }
    }
}

impl WireKind {
    pub fn is_tie(self) -> bool {
        matches!(self, WireKind::Tie0 | WireKind::Tie1 | WireKind::TiePullup)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct TileClass {
    pub cells: EntityVec<TileCellId, ()>,
    pub muxes: BTreeMap<TileClassWire, MuxInfo>,
    pub iris: EntityVec<TileIriId, ()>,
    pub intfs: BTreeMap<TileClassWire, IntfInfo>,
    pub bels: EntityPartVec<BelSlotId, BelInfo>,
}

pub type TileClassWire = (TileCellId, WireId);

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct MuxInfo {
    pub kind: MuxKind,
    pub ins: BTreeSet<TileClassWire>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum MuxKind {
    Plain,
    Inv,
    OptInv,
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Encode, Decode)]
pub struct BelInfo {
    pub pins: BTreeMap<String, BelPin>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct BelPin {
    pub wires: BTreeSet<TileClassWire>,
    pub dir: PinDir,
    pub is_intf_in: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum PinDir {
    Input,
    Output,
    Inout,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub enum IntfInfo {
    OutputTestMux(BTreeSet<TileClassWire>),
    OutputTestMuxPass(BTreeSet<TileClassWire>, TileClassWire),
    InputDelay,
    InputIri(TileIriId, IriPin),
    InputIriDelay(TileIriId, IriPin),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum IriPin {
    Clk,
    Rst,
    Ce(u32),
    Imux(u32),
}

impl std::fmt::Display for IriPin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IriPin::Clk => write!(f, "CLK"),
            IriPin::Rst => write!(f, "RST"),
            IriPin::Ce(i) => write!(f, "CE{i}"),
            IriPin::Imux(i) => write!(f, "IMUX{i}"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct ConnectorClass {
    pub slot: ConnectorSlotId,
    pub wires: EntityPartVec<WireId, ConnectorWire>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum ConnectorWire {
    BlackHole,
    Reflect(WireId),
    Pass(WireId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct ConnectorSlot {
    pub opposite: ConnectorSlotId,
}

#[derive(Clone, Debug)]
pub struct IntDbIndex {
    pub tile_classes: EntityVec<TileClassId, TileClassIndex>,
    pub conn_classes: EntityVec<ConnectorClassId, ConnectorClassIndex>,
    pub buf_ins: EntityVec<WireId, HashSet<WireId>>,
}

#[derive(Clone, Debug)]
pub struct TileClassIndex {
    pub mux_ins: HashMap<TileClassWire, HashSet<TileClassWire>>,
    pub intf_ins: HashMap<TileClassWire, HashSet<TileClassWire>>,
    pub intf_ins_pass: HashMap<TileClassWire, HashSet<TileClassWire>>,
}

#[derive(Clone, Debug)]
pub struct ConnectorClassIndex {
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
            tile_classes: db.tile_classes.values().map(TileClassIndex::new).collect(),
            conn_classes: db
                .conn_classes
                .values()
                .map(|t| ConnectorClassIndex::new(t, db))
                .collect(),
            buf_ins,
        }
    }
}

impl TileClassIndex {
    pub fn new(tcls: &TileClass) -> Self {
        let mut mux_ins: HashMap<_, HashSet<_>> = HashMap::new();
        for (&wo, mux) in &tcls.muxes {
            for &wi in &mux.ins {
                mux_ins.entry(wi).or_default().insert(wo);
            }
        }

        let mut intf_ins: HashMap<_, HashSet<_>> = HashMap::new();
        let mut intf_ins_pass: HashMap<_, HashSet<_>> = HashMap::new();
        for (&wo, intf) in &tcls.intfs {
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

        TileClassIndex {
            mux_ins,
            intf_ins,
            intf_ins_pass,
        }
    }
}

impl ConnectorClassIndex {
    pub fn new(term: &ConnectorClass, db: &IntDb) -> Self {
        let mut wire_ins_far: EntityVec<_, _> = db.wires.ids().map(|_| HashSet::new()).collect();
        let mut wire_ins_near: EntityVec<_, _> = db.wires.ids().map(|_| HashSet::new()).collect();
        for (wo, ti) in &term.wires {
            match *ti {
                ConnectorWire::BlackHole => (),
                ConnectorWire::Reflect(wi) => {
                    wire_ins_near[wi].insert(wo);
                }
                ConnectorWire::Pass(wi) => {
                    wire_ins_far[wi].insert(wo);
                }
            }
        }
        ConnectorClassIndex {
            wire_ins_far,
            wire_ins_near,
        }
    }
}

impl MuxInfo {
    pub fn to_json(&self, db: &IntDb) -> JsonValue {
        jzon::object! {
            kind: match self.kind {
                MuxKind::Plain => "PLAIN",
                MuxKind::Inv => "INV",
                MuxKind::OptInv => "OPTINV",
            },
            ins: Vec::from_iter(self.ins.iter().map(|wf| format!(
                "{:#}:{}", wf.0, db.wires.key(wf.1)
            ))),
        }
    }
}

impl IntfInfo {
    pub fn to_json(&self, db: &IntDb) -> JsonValue {
        match self {
            IntfInfo::OutputTestMux(ins) => jzon::object! {
                kind: "OUTPUT_TEST_MUX",
                ins: Vec::from_iter(ins.iter().map(|wf| format!(
                    "{:#}:{}", wf.0, db.wires.key(wf.1)
                ))),
            },
            IntfInfo::OutputTestMuxPass(ins, def) => jzon::object! {
                kind: "OUTPUT_TEST_MUX_PASS",
                ins: Vec::from_iter(ins.iter().map(|wf| format!(
                    "{:#}:{}", wf.0, db.wires.key(wf.1)
                ))),
                default: format!("{:#}:{}", def.0, db.wires.key(def.1)),
            },
            IntfInfo::InputDelay => jzon::object! {
                kind: "INPUT_DELAY",
            },
            IntfInfo::InputIri(iri, pin) => jzon::object! {
                kind: "INPUT_IRI",
                iri: iri.to_idx(),
                pin: pin.to_string(),
            },
            IntfInfo::InputIriDelay(iri, pin) => jzon::object! {
                kind: "INPUT_IRI_DELAY",
                iri: iri.to_idx(),
                pin: pin.to_string(),
            },
        }
    }
}

impl BelPin {
    pub fn to_json(&self, db: &IntDb) -> JsonValue {
        jzon::object! {
            wires: Vec::from_iter(self.wires.iter().map(|wf| format!(
                "{:#}:{}", wf.0, db.wires.key(wf.1)
            ))),
            dir: match self.dir {
                PinDir::Input => "INPUT",
                PinDir::Output => "OUTPUT",
                PinDir::Inout => "INOUT",
            },
            is_intf_in: self.is_intf_in,
        }
    }
}

impl TileClass {
    pub fn to_json(&self, db: &IntDb) -> JsonValue {
        jzon::object! {
            cells: self.cells.len(),
            muxes: jzon::object::Object::from_iter(self.muxes.iter().map(|(wt, mux)| (
                format!("{:#}:{}", wt.0, db.wires.key(wt.1)),
                mux.to_json(db),
            ))),
            iris: self.iris.len(),
            intfs: jzon::object::Object::from_iter(self.intfs.iter().map(|(wt, intf)| (
                format!("{:#}:{}", wt.0, db.wires.key(wt.1)),
                intf.to_json(db),
            ))),
            bels: jzon::object::Object::from_iter(self.bels.iter().map(|(slot, bel)| (
                db.bel_slots[slot].as_str(),
                jzon::object! {
                    pins: jzon::object::Object::from_iter(bel.pins.iter().map(|(pname, pin)| (pname.as_str(), pin.to_json(db)))),
                },
            ))),
        }
    }
}

impl ConnectorSlot {
    pub fn to_json(self, db: &IntDb) -> JsonValue {
        jzon::object! {
            opposite: db.conn_slots.key(self.opposite).as_str(),
        }
    }
}

impl ConnectorWire {
    pub fn to_json(self, db: &IntDb) -> JsonValue {
        match self {
            ConnectorWire::BlackHole => jzon::object! {
                kind: "BLACKHOLE",
            },
            ConnectorWire::Reflect(wf) => jzon::object! {
                kind: "REFLECT",
                wire: db.wires.key(wf).as_str(),
            },
            ConnectorWire::Pass(wf) => jzon::object! {
                kind: "PASS",
                wire: db.wires.key(wf).as_str(),
            },
        }
    }
}

impl ConnectorClass {
    pub fn to_json(&self, db: &IntDb) -> JsonValue {
        jzon::object! {
            slot: db.conn_slots.key(self.slot).as_str(),
            wires: jzon::object::Object::from_iter(self.wires.iter().map(|(wire, ti)|
                (db.wires.key(wire).to_string(), ti.to_json(db))
            ))
        }
    }
}

impl From<&IntDb> for JsonValue {
    fn from(db: &IntDb) -> Self {
        jzon::object! {
            wires: Vec::from_iter(db.wires.iter().map(|(_, name, wire)| {
                jzon::object! {
                    name: name.as_str(),
                    kind: wire.to_string(db),
                }
            })),
            region_slots: Vec::from_iter(db.region_slots.values().map(|name| name.as_str())),
            bel_slots: Vec::from_iter(db.bel_slots.values().map(|name| name.as_str())),
            tile_classes: jzon::object::Object::from_iter(db.tile_classes.iter().map(|(_, name, tcls)| {
                (name.as_str(), tcls.to_json(db))
            })),
            conn_slots: jzon::object::Object::from_iter(db.conn_slots.iter().map(|(_, name, cslot)| {
                (name.as_str(), cslot.to_json(db))
            })),
            conn_classes: jzon::object::Object::from_iter(db.conn_classes.iter().map(|(_, name, ccls)| {
                (name.as_str(), ccls.to_json(db))
            })),
        }
    }
}
