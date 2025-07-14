use bincode::{Decode, Encode};
use jzon::JsonValue;
use std::collections::{BTreeMap, BTreeSet, HashSet};
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
pub struct CellSlotTag;
impl EntityTag for CellSlotTag {
    const PREFIX: &'static str = "TCELL";
}
pub type WireId = EntityIdU16<WireKind>;
pub type TileClassId = EntityIdU16<TileClass>;
pub type RegionSlotId = EntityIdU8<RegionSlotTag>;
pub type ConnectorSlotId = EntityIdU8<ConnectorSlot>;
pub type ConnectorClassId = EntityIdU16<ConnectorClass>;
pub type CellSlotId = EntityIdU16<CellSlotTag>;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode)]
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode)]
pub struct TileSlotId(u8);

impl TileSlotId {
    pub const fn from_idx_const(idx: usize) -> Self {
        assert!(idx <= 0xff);
        Self(idx as u8)
    }

    pub const fn to_idx_const(self) -> usize {
        self.0 as usize
    }
}

impl EntityId for TileSlotId {
    fn from_idx(idx: usize) -> Self {
        Self(idx.try_into().unwrap())
    }

    fn to_idx(self) -> usize {
        self.0.into()
    }
}

impl std::fmt::Debug for TileSlotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TSLOT{}", self.0)
    }
}

impl std::fmt::Display for TileSlotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "{}", self.0)
        } else {
            write!(f, "TSLOT{}", self.0)
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Encode, Decode)]
pub struct IntDb {
    pub wires: EntityMap<WireId, String, WireKind>,
    pub tile_slots: EntitySet<TileSlotId, String>,
    pub bel_slots: EntityMap<BelSlotId, String, BelSlot>,
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
            .0
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

    pub fn init_slots(
        &mut self,
        tslots: &[(TileSlotId, &str)],
        bslots: &[(BelSlotId, &str, TileSlotId)],
    ) {
        for &(id, name) in tslots {
            assert_eq!(self.tile_slots.insert(name.into()), (id, true));
        }
        for &(id, name, tslot) in bslots {
            assert_eq!(
                self.bel_slots
                    .insert(name.into(), BelSlot { tile_slot: tslot }),
                (id, None)
            );
        }
    }

    pub fn validate(&self) {
        for (_, tcname, tcls) in &self.tile_classes {
            for bel in tcls.bels.ids() {
                let bname = self.bel_slots.key(bel);
                let bslot = &self.bel_slots[bel];
                assert_eq!(
                    tcls.slot,
                    bslot.tile_slot,
                    "mismatch on tile {tcname} bel {bname}: {tctslot} != {btslot}",
                    tctslot = self.tile_slots[tcls.slot],
                    btslot = self.tile_slots[bslot.tile_slot],
                );
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct BelSlot {
    pub tile_slot: TileSlotId,
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
    Buf(WireId),
    MultiBranch(ConnectorSlotId),
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
            WireKind::Buf(wire_id) => format!("BUF:{}", db.wires.key(*wire_id)),
            WireKind::MultiBranch(slot) => {
                format!("MULTI_BRANCH:{slot}", slot = db.conn_slots.key(*slot))
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
    pub slot: TileSlotId,
    pub cells: EntityVec<CellSlotId, ()>,
    pub intfs: BTreeMap<TileWireCoord, IntfInfo>,
    pub bels: EntityPartVec<BelSlotId, BelInfo>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub struct TileWireCoord {
    pub cell: CellSlotId,
    pub wire: WireId,
}

impl TileWireCoord {
    pub fn pos(self) -> PolTileWireCoord {
        PolTileWireCoord {
            tw: self,
            inv: false,
        }
    }

    pub fn to_string(self, db: &IntDb, tcls: &TileClass) -> String {
        if tcls.cells.len() == 1 {
            db.wires.key(self.wire).clone()
        } else {
            format!(
                "{cell}_{wire}",
                cell = self.cell,
                wire = db.wires.key(self.wire)
            )
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub struct PolTileWireCoord {
    pub tw: TileWireCoord,
    pub inv: bool,
}

impl PolTileWireCoord {
    pub fn to_string(self, db: &IntDb, tcls: &TileClass) -> String {
        let res = self.tw.to_string(db, tcls);
        if self.inv { format!("~{res}") } else { res }
    }
}

impl std::ops::Not for PolTileWireCoord {
    type Output = PolTileWireCoord;

    fn not(self) -> Self::Output {
        Self {
            inv: !self.inv,
            ..self
        }
    }
}

impl std::ops::Deref for PolTileWireCoord {
    type Target = TileWireCoord;

    fn deref(&self) -> &Self::Target {
        &self.tw
    }
}

impl std::ops::DerefMut for PolTileWireCoord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tw
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub enum BelInfo {
    SwitchBox(SwitchBox),
    Bel(Bel),
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Encode, Decode)]
pub struct Bel {
    pub pins: BTreeMap<String, BelPin>,
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Encode, Decode)]
pub struct SwitchBox {
    pub items: Vec<SwitchBoxItem>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub enum SwitchBoxItem {
    Mux(Mux),
    ProgBuf(Buf),
    PermaBuf(Buf),
    Pass(Pass),
    BiPass(BiPass),
    ProgInv(ProgInv),
    ProgDelay(ProgDelay),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct Mux {
    pub dst: TileWireCoord,
    pub src: BTreeSet<PolTileWireCoord>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct Buf {
    pub dst: TileWireCoord,
    pub src: PolTileWireCoord,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct Pass {
    pub dst: TileWireCoord,
    pub src: TileWireCoord,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct BiPass {
    pub a: TileWireCoord,
    pub b: TileWireCoord,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct ProgInv {
    pub dst: TileWireCoord,
    pub src: TileWireCoord,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct ProgDelay {
    pub dst: TileWireCoord,
    pub src: PolTileWireCoord,
    pub num_steps: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct BelPin {
    pub wires: BTreeSet<TileWireCoord>,
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
    OutputTestMux(BTreeSet<TileWireCoord>),
    OutputTestMuxPass(BTreeSet<TileWireCoord>, TileWireCoord),
    InputDelay,
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
    pub pips_fwd: BTreeMap<TileWireCoord, BTreeSet<PolTileWireCoord>>,
    pub pips_bwd: BTreeMap<TileWireCoord, BTreeSet<PolTileWireCoord>>,
    pub intf_ins: BTreeMap<TileWireCoord, BTreeSet<TileWireCoord>>,
    pub intf_ins_pass: BTreeMap<TileWireCoord, BTreeSet<TileWireCoord>>,
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
        let mut pips_fwd: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
        let mut pips_bwd: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
        for bel in tcls.bels.values() {
            if let BelInfo::SwitchBox(sb) = bel {
                for item in &sb.items {
                    match *item {
                        SwitchBoxItem::Mux(ref mux) => {
                            for &src in &mux.src {
                                pips_fwd
                                    .entry(src.tw)
                                    .or_default()
                                    .insert(PolTileWireCoord {
                                        tw: mux.dst,
                                        inv: src.inv,
                                    });
                                pips_bwd.entry(mux.dst).or_default().insert(src);
                            }
                        }
                        SwitchBoxItem::ProgBuf(buf) => {
                            pips_fwd
                                .entry(buf.src.tw)
                                .or_default()
                                .insert(PolTileWireCoord {
                                    tw: buf.dst,
                                    inv: buf.src.inv,
                                });
                            pips_bwd.entry(buf.dst).or_default().insert(buf.src);
                        }
                        SwitchBoxItem::PermaBuf(buf) => {
                            pips_fwd
                                .entry(buf.src.tw)
                                .or_default()
                                .insert(PolTileWireCoord {
                                    tw: buf.dst,
                                    inv: buf.src.inv,
                                });
                            pips_bwd.entry(buf.dst).or_default().insert(buf.src);
                        }
                        SwitchBoxItem::Pass(pass) => {
                            pips_fwd.entry(pass.src).or_default().insert(pass.dst.pos());
                            pips_bwd.entry(pass.dst).or_default().insert(pass.src.pos());
                        }
                        SwitchBoxItem::BiPass(pass) => {
                            pips_fwd.entry(pass.a).or_default().insert(pass.b.pos());
                            pips_fwd.entry(pass.b).or_default().insert(pass.a.pos());
                            pips_bwd.entry(pass.a).or_default().insert(pass.b.pos());
                            pips_bwd.entry(pass.b).or_default().insert(pass.a.pos());
                        }
                        SwitchBoxItem::ProgInv(inv) => {
                            pips_fwd.entry(inv.src).or_default().insert(inv.dst.pos());
                            pips_fwd.entry(inv.src).or_default().insert(!inv.dst.pos());
                            pips_bwd.entry(inv.dst).or_default().insert(inv.src.pos());
                            pips_bwd.entry(inv.dst).or_default().insert(!inv.src.pos());
                        }
                        SwitchBoxItem::ProgDelay(delay) => {
                            pips_fwd
                                .entry(delay.src.tw)
                                .or_default()
                                .insert(PolTileWireCoord {
                                    tw: delay.dst,
                                    inv: delay.src.inv,
                                });
                            pips_bwd.entry(delay.dst).or_default().insert(delay.src);
                        }
                    }
                }
            }
        }

        let mut intf_ins: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
        let mut intf_ins_pass: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
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
            pips_fwd,
            pips_bwd,
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

impl IntfInfo {
    pub fn to_json(&self, db: &IntDb) -> JsonValue {
        match self {
            IntfInfo::OutputTestMux(ins) => jzon::object! {
                kind: "OUTPUT_TEST_MUX",
                ins: Vec::from_iter(ins.iter().map(|wf| format!(
                    "{:#}:{}", wf.cell, db.wires.key(wf.wire)
                ))),
            },
            IntfInfo::OutputTestMuxPass(ins, def) => jzon::object! {
                kind: "OUTPUT_TEST_MUX_PASS",
                ins: Vec::from_iter(ins.iter().map(|wf| format!(
                    "{:#}:{}", wf.cell, db.wires.key(wf.wire)
                ))),
                default: format!("{:#}:{}", def.cell, db.wires.key(def.wire)),
            },
            IntfInfo::InputDelay => jzon::object! {
                kind: "INPUT_DELAY",
            },
        }
    }
}

impl BelPin {
    pub fn to_json(&self, db: &IntDb, tcls: &TileClass) -> JsonValue {
        jzon::object! {
            wires: Vec::from_iter(self.wires.iter().map(|wf| wf.to_string(db, tcls))),
            dir: match self.dir {
                PinDir::Input => "INPUT",
                PinDir::Output => "OUTPUT",
                PinDir::Inout => "INOUT",
            },
            is_intf_in: self.is_intf_in,
        }
    }
}

impl BelInfo {
    pub fn to_json(&self, db: &IntDb, tcls: &TileClass) -> JsonValue {
        match self {
            BelInfo::SwitchBox(sb) => sb.to_json(db, tcls),
            BelInfo::Bel(bel) => bel.to_json(db, tcls),
        }
    }
}

impl SwitchBox {
    pub fn to_json(&self, db: &IntDb, tcls: &TileClass) -> JsonValue {
        jzon::object! {
            kind: "switchbox",
            items: Vec::from_iter(self.items.iter().map(|item| item.to_json(db, tcls))),
        }
    }
}

impl SwitchBoxItem {
    pub fn to_json(&self, db: &IntDb, tcls: &TileClass) -> JsonValue {
        match self {
            SwitchBoxItem::Mux(mux) => jzon::object! {
                kind: "mux",
                dst: mux.dst.to_string(db, tcls),
                src: Vec::from_iter(mux.src.iter().map(|w| w.to_string(db, tcls))),
            },
            SwitchBoxItem::ProgBuf(buf) => jzon::object! {
                kind: "progbuf",
                dst: buf.dst.to_string(db, tcls),
                src: buf.src.to_string(db, tcls),
            },
            SwitchBoxItem::PermaBuf(buf) => jzon::object! {
                kind: "permabuf",
                dst: buf.dst.to_string(db, tcls),
                src: buf.src.to_string(db, tcls),
            },
            SwitchBoxItem::Pass(pass) => jzon::object! {
                kind: "pass",
                dst: pass.dst.to_string(db, tcls),
                src: pass.src.to_string(db, tcls),
            },
            SwitchBoxItem::BiPass(pass) => jzon::object! {
                kind: "pass",
                wires: [
                    pass.a.to_string(db, tcls),
                    pass.b.to_string(db, tcls),
                ],
            },
            SwitchBoxItem::ProgInv(inv) => jzon::object! {
                kind: "proginv",
                dst: inv.dst.to_string(db, tcls),
                src: inv.src.to_string(db, tcls),
            },
            SwitchBoxItem::ProgDelay(delay) => jzon::object! {
                kind: "progdelay",
                dst: delay.dst.to_string(db, tcls),
                src: delay.src.to_string(db, tcls),
                num_steps: delay.num_steps,
            },
        }
    }
}

impl Bel {
    pub fn to_json(&self, db: &IntDb, tcls: &TileClass) -> JsonValue {
        jzon::object! {
            kind: "bel",
            pins: jzon::object::Object::from_iter(self.pins.iter().map(|(pname, pin)| (pname.as_str(), pin.to_json(db, tcls)))),
        }
    }
}

impl TileClass {
    pub fn to_json(&self, db: &IntDb) -> JsonValue {
        jzon::object! {
            slot: db.tile_slots[self.slot].as_str(),
            cells: self.cells.len(),
            intfs: jzon::object::Object::from_iter(self.intfs.iter().map(|(wt, intf)| (
                format!("{:#}:{}", wt.cell, db.wires.key(wt.wire)),
                intf.to_json(db),
            ))),
            bels: jzon::object::Object::from_iter(self.bels.iter().map(|(slot, bel)| (
                db.bel_slots.key(slot).as_str(),
                bel.to_json(db, self),
            ))),
        }
    }
}

impl BelSlot {
    pub fn to_json(self, db: &IntDb) -> JsonValue {
        jzon::object! {
            tile_slot: db.tile_slots[self.tile_slot].as_str(),
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
            tile_slots: Vec::from_iter(db.tile_slots.values().map(|name| name.as_str())),
            bel_slots: jzon::object::Object::from_iter(db.bel_slots.iter().map(|(_, name, bslot)| {
                (name.as_str(), bslot.to_json(db))
            })),
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
