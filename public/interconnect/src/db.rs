use bincode::{Decode, Encode};
use prjcombine_entity::{
    EntityId, EntityMap, EntityPartVec, EntitySet, EntityVec,
    id::{EntityIdU8, EntityIdU16, EntityTag},
};
use std::collections::{BTreeMap, BTreeSet, HashSet};

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
pub type WireSlotId = EntityIdU16<WireKind>;
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
    pub wires: EntityMap<WireSlotId, String, WireKind>,
    pub tile_slots: EntitySet<TileSlotId, String>,
    pub bel_slots: EntityMap<BelSlotId, String, BelSlot>,
    pub region_slots: EntitySet<RegionSlotId, String>,
    pub tile_classes: EntityMap<TileClassId, String, TileClass>,
    pub conn_slots: EntityMap<ConnectorSlotId, String, ConnectorSlot>,
    pub conn_classes: EntityMap<ConnectorClassId, String, ConnectorClass>,
}

impl IntDb {
    #[track_caller]
    pub fn get_wire(&self, name: &str) -> WireSlotId {
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

    pub fn new(
        tslots: &[(TileSlotId, &str)],
        bslots: &[(BelSlotId, &str, TileSlotId)],
        rslots: &[(RegionSlotId, &str)],
        cslots: &[(ConnectorSlotId, &str, ConnectorSlotId)],
    ) -> Self {
        let mut result = IntDb::default();
        for &(id, name) in tslots {
            assert_eq!(result.tile_slots.insert(name.into()), (id, true));
        }
        for &(id, name, tslot) in bslots {
            assert_eq!(
                result
                    .bel_slots
                    .insert(name.into(), BelSlot { tile_slot: tslot }),
                (id, None)
            );
        }
        for &(id, name) in rslots {
            assert_eq!(result.region_slots.insert(name.into()), (id, true));
        }
        for &(id, name, opposite) in cslots {
            assert_eq!(
                result
                    .conn_slots
                    .insert(name.into(), ConnectorSlot { opposite }),
                (id, None)
            );
        }
        result
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

impl std::ops::Index<WireSlotId> for IntDb {
    type Output = WireKind;

    fn index(&self, index: WireSlotId) -> &Self::Output {
        &self.wires[index]
    }
}

impl std::ops::Index<TileClassId> for IntDb {
    type Output = TileClass;

    fn index(&self, index: TileClassId) -> &Self::Output {
        &self.tile_classes[index]
    }
}

impl std::ops::Index<ConnectorClassId> for IntDb {
    type Output = ConnectorClass;

    fn index(&self, index: ConnectorClassId) -> &Self::Output {
        &self.conn_classes[index]
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
    pub bels: EntityPartVec<BelSlotId, BelInfo>,
}

impl TileClass {
    pub fn new(slot: TileSlotId, num_cells: usize) -> Self {
        TileClass {
            slot,
            cells: EntityVec::from_iter(std::iter::repeat_n((), num_cells)),
            bels: Default::default(),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub struct TileWireCoord {
    pub cell: CellSlotId,
    pub wire: WireSlotId,
}

impl TileWireCoord {
    pub fn new_idx(cell_idx: usize, wire: WireSlotId) -> Self {
        TileWireCoord {
            cell: CellSlotId::from_idx(cell_idx),
            wire,
        }
    }

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
    TestMux(TestMux),
    GroupTestMux(GroupTestMux),
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

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode, Default)]
pub struct TestMux {
    pub wires: BTreeMap<TileWireCoord, TestMuxWire>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct TestMuxWire {
    pub primary_src: PolTileWireCoord,
    pub test_src: BTreeSet<PolTileWireCoord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode, Default)]
pub struct GroupTestMux {
    pub num_groups: usize,
    pub wires: BTreeMap<TileWireCoord, GroupTestMuxWire>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct GroupTestMuxWire {
    pub primary_src: PolTileWireCoord,
    pub test_src: Vec<Option<PolTileWireCoord>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct BelPin {
    pub wires: BTreeSet<TileWireCoord>,
    pub dir: PinDir,
}

impl BelPin {
    pub fn new_in(wire: TileWireCoord) -> BelPin {
        BelPin {
            wires: BTreeSet::from_iter([wire]),
            dir: PinDir::Input,
        }
    }

    pub fn new_out(wire: TileWireCoord) -> BelPin {
        BelPin {
            wires: BTreeSet::from_iter([wire]),
            dir: PinDir::Output,
        }
    }

    pub fn new_out_multi(wires: impl IntoIterator<Item = TileWireCoord>) -> BelPin {
        BelPin {
            wires: BTreeSet::from_iter(wires),
            dir: PinDir::Output,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum PinDir {
    Input,
    Output,
    Inout,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct ConnectorClass {
    pub slot: ConnectorSlotId,
    pub wires: EntityPartVec<WireSlotId, ConnectorWire>,
}

impl ConnectorClass {
    pub fn new(slot: ConnectorSlotId) -> Self {
        ConnectorClass {
            slot,
            wires: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum ConnectorWire {
    BlackHole,
    Reflect(WireSlotId),
    Pass(WireSlotId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct ConnectorSlot {
    pub opposite: ConnectorSlotId,
}

#[derive(Clone, Debug)]
pub struct IntDbIndex {
    pub tile_classes: EntityVec<TileClassId, TileClassIndex>,
    pub conn_classes: EntityVec<ConnectorClassId, ConnectorClassIndex>,
}

impl std::ops::Index<TileClassId> for IntDbIndex {
    type Output = TileClassIndex;

    fn index(&self, index: TileClassId) -> &Self::Output {
        &self.tile_classes[index]
    }
}

impl std::ops::Index<ConnectorClassId> for IntDbIndex {
    type Output = ConnectorClassIndex;

    fn index(&self, index: ConnectorClassId) -> &Self::Output {
        &self.conn_classes[index]
    }
}

#[derive(Clone, Debug)]
pub struct TileClassIndex {
    pub pips_fwd: BTreeMap<TileWireCoord, BTreeSet<PolTileWireCoord>>,
    pub pips_bwd: BTreeMap<TileWireCoord, BTreeSet<PolTileWireCoord>>,
}

#[derive(Clone, Debug)]
pub struct ConnectorClassIndex {
    pub wire_ins_far: EntityVec<WireSlotId, HashSet<WireSlotId>>,
    pub wire_ins_near: EntityVec<WireSlotId, HashSet<WireSlotId>>,
}

impl IntDbIndex {
    pub fn new(db: &IntDb) -> Self {
        Self {
            tile_classes: db.tile_classes.values().map(TileClassIndex::new).collect(),
            conn_classes: db
                .conn_classes
                .values()
                .map(|t| ConnectorClassIndex::new(t, db))
                .collect(),
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

        TileClassIndex { pips_fwd, pips_bwd }
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
