use bincode::{Decode, Encode};
use prjcombine_entity::{
    EntityBundleMap, EntityId, EntityMap, EntityPartVec, EntitySet, EntityVec,
    id::{EntityIdU8, EntityIdU16, EntityTag},
};
use prjcombine_types::{
    bitvec::BitVec,
    bsdata::{BitRectGeometry, BitRectId, PolTileBit, TileBit},
};
use std::collections::{BTreeMap, BTreeSet, HashSet};

// region: top

#[derive(Clone, Debug, Default, PartialEq, Eq, Encode, Decode)]
pub struct IntDb {
    pub enum_classes: EntityMap<EnumClassId, String, EnumClass>,
    pub bel_classes: EntityMap<BelClassId, String, BelClass>,
    pub region_slots: EntitySet<RegionSlotId, String>,
    pub wires: EntityMap<WireSlotId, String, WireKind>,
    pub tile_slots: EntitySet<TileSlotId, String>,
    pub bel_slots: EntityMap<BelSlotId, String, BelSlot>,
    pub tile_classes: EntityMap<TileClassId, String, TileClass>,
    pub conn_slots: EntityMap<ConnectorSlotId, String, ConnectorSlot>,
    pub conn_classes: EntityMap<ConnectorClassId, String, ConnectorClass>,
    pub tables: EntityMap<TableId, String, Table>,
    pub devdata: EntityMap<DeviceDataId, String, BelAttributeType>,
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
                result.bel_slots.insert(
                    name.into(),
                    BelSlot {
                        tile_slot: tslot,
                        kind: BelKind::Legacy
                    }
                ),
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

impl std::ops::Index<EnumClassId> for IntDb {
    type Output = EnumClass;

    fn index(&self, index: EnumClassId) -> &Self::Output {
        &self.enum_classes[index]
    }
}

impl std::ops::Index<BelClassId> for IntDb {
    type Output = BelClass;

    fn index(&self, index: BelClassId) -> &Self::Output {
        &self.bel_classes[index]
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

impl std::ops::Index<TableId> for IntDb {
    type Output = Table;

    fn index(&self, index: TableId) -> &Self::Output {
        &self.tables[index]
    }
}

// endregion:

// region: enums and tables

impl EntityTag for EnumClass {
    const PREFIX: &'static str = "ECLS";
}
pub struct EnumValueTag;
impl EntityTag for EnumValueTag {
    const PREFIX: &'static str = "EV";
}
pub type EnumClassId = EntityIdU16<EnumClass>;
pub type EnumValueId = EntityIdU16<EnumValueTag>;

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct EnumClass {
    pub values: EntitySet<EnumValueId, String>,
}

impl EntityTag for Table {
    const PREFIX: &'static str = "TABLE";
}
pub type TableId = EntityIdU16<Table>;
pub struct TableFieldTag;
impl EntityTag for TableFieldTag {
    const PREFIX: &'static str = "FIELD";
}
pub type TableFieldId = EntityIdU8<TableFieldTag>;
pub struct TableRowTag;
impl EntityTag for TableRowTag {
    const PREFIX: &'static str = "ROW";
}
pub type TableRowId = EntityIdU16<TableRowTag>;

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct Table {
    pub fields: EntityMap<TableFieldId, String, BelAttributeType>,
    pub rows: EntityMap<TableRowId, String, EntityPartVec<TableFieldId, TableValue>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub enum TableValue {
    BitVec(BitVec),
    Enum(EnumValueId),
    U32(u32),
}

impl EntityTag for DeviceDataTag {
    const PREFIX: &'static str = "DD";
}
pub struct DeviceDataTag;
pub type DeviceDataId = EntityIdU16<DeviceDataTag>;

// endregion:

// region: bel classes and slots

impl EntityTag for BelClass {
    const PREFIX: &'static str = "BCLS";
}
impl EntityTag for BelClassInput {
    const PREFIX: &'static str = "BELIN";
}
impl EntityTag for BelClassOutput {
    const PREFIX: &'static str = "BELOUT";
}
impl EntityTag for BelClassBidir {
    const PREFIX: &'static str = "BELIO";
}
impl EntityTag for BelClassPad {
    const PREFIX: &'static str = "BELPAD";
}
impl EntityTag for BelClassAttribute {
    const PREFIX: &'static str = "BELATTR";
}
pub type BelClassId = EntityIdU16<BelClass>;
pub type BelInputId = EntityIdU16<BelClassInput>;
pub type BelOutputId = EntityIdU16<BelClassOutput>;
pub type BelBidirId = EntityIdU16<BelClassBidir>;
pub type BelPadId = EntityIdU16<BelClassPad>;
pub type BelAttributeId = EntityIdU16<BelClassAttribute>;

#[derive(Default, Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct BelClass {
    pub inputs: EntityBundleMap<BelInputId, BelClassInput>,
    pub outputs: EntityBundleMap<BelOutputId, BelClassOutput>,
    pub bidirs: EntityBundleMap<BelBidirId, BelClassBidir>,
    pub pads: EntityBundleMap<BelPadId, BelClassPad>,
    pub attributes: EntityMap<BelAttributeId, String, BelClassAttribute>,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct BelClassInput {
    pub nonroutable: bool,
    pub indexing: BelPinIndexing,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct BelClassOutput {
    pub nonroutable: bool,
    pub indexing: BelPinIndexing,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct BelClassBidir {
    pub nonroutable: bool,
    pub indexing: BelPinIndexing,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, Encode, Decode)]
pub struct BelPinIndexing {
    pub lsb_index: usize,
    pub wrong_endian: bool,
}

impl BelPinIndexing {
    pub fn try_virt_to_phys(self, index: usize) -> Option<usize> {
        if self.wrong_endian {
            if index > self.lsb_index {
                None
            } else {
                Some(self.lsb_index - index)
            }
        } else {
            if index < self.lsb_index {
                None
            } else {
                Some(index - self.lsb_index)
            }
        }
    }

    pub fn virt_to_phys(self, index: usize) -> usize {
        self.try_virt_to_phys(index).unwrap()
    }

    pub fn try_phys_to_virt(self, index: usize) -> Option<usize> {
        if self.wrong_endian {
            if index > self.lsb_index {
                None
            } else {
                Some(self.lsb_index - index)
            }
        } else {
            Some(self.lsb_index + index)
        }
    }

    pub fn phys_to_virt(self, index: usize) -> usize {
        self.try_phys_to_virt(index).unwrap()
    }

    pub fn msb_index(self, width: usize) -> usize {
        self.phys_to_virt(width - 1)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct BelClassPad {
    pub kind: PadKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct BelClassAttribute {
    pub typ: BelAttributeType,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub enum BelAttributeType {
    Enum(EnumClassId),
    Bool,
    BitVec(usize),
    BitVecArray(usize, usize),
    // for table / device data only
    U32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum PadKind {
    In,
    Out,
    Inout,
    Power,
    Analog,
}

impl EntityTag for BelSlot {
    const PREFIX: &'static str = "BEL";
}

pub type BelSlotId = EntityIdU16<BelSlot>;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct BelSlot {
    pub tile_slot: TileSlotId,
    pub kind: BelKind,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum BelKind {
    Routing,
    Class(BelClassId),
    Legacy,
}

// endregion:

// region: regions, wires, connectors

pub struct RegionSlotTag;
impl EntityTag for RegionSlotTag {
    const PREFIX: &'static str = "RSLOT";
}
pub type RegionSlotId = EntityIdU8<RegionSlotTag>;

impl EntityTag for WireKind {
    const PREFIX: &'static str = "WIRE";
}
pub type WireSlotId = EntityIdU16<WireKind>;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum WireKind {
    Tie0,
    Tie1,
    TiePullup,
    Regional(RegionSlotId),
    MuxOut,
    BelOut,
    TestOut,
    MultiRoot,
    MultiBranch(ConnectorSlotId),
    Branch(ConnectorSlotId),
    Special,
}

impl WireKind {
    pub fn to_string(&self, db: &IntDb) -> String {
        match self {
            WireKind::Tie0 => "tie 0".into(),
            WireKind::Tie1 => "tie 1".into(),
            WireKind::TiePullup => "pullup".into(),
            WireKind::Regional(slot) => format!("regional {}", db.region_slots[*slot]),
            WireKind::MuxOut => "mux".into(),
            WireKind::BelOut => "bel".into(),
            WireKind::TestOut => "test".into(),
            WireKind::MultiRoot => "multi_root".into(),
            WireKind::MultiBranch(slot) => {
                format!("multi_branch {slot}", slot = db.conn_slots.key(*slot))
            }
            WireKind::Branch(slot) => format!("branch {slot}", slot = db.conn_slots.key(*slot)),
            WireKind::Special => "special".into(),
        }
    }
}

impl WireKind {
    pub fn is_tie(self) -> bool {
        matches!(self, WireKind::Tie0 | WireKind::Tie1 | WireKind::TiePullup)
    }
}

impl EntityTag for ConnectorSlot {
    const PREFIX: &'static str = "CSLOT";
}
impl EntityTag for ConnectorClass {
    const PREFIX: &'static str = "CCLS";
}

pub type ConnectorSlotId = EntityIdU8<ConnectorSlot>;
pub type ConnectorClassId = EntityIdU16<ConnectorClass>;

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

// endregion:

// region: tiles

impl EntityTag for TileClass {
    const PREFIX: &'static str = "TCLS";
}
pub struct CellSlotTag;
impl EntityTag for CellSlotTag {
    const PREFIX: &'static str = "TCELL";
}
pub struct TileSlotTag;
impl EntityTag for TileSlotTag {
    const PREFIX: &'static str = "TSLOT";
}
pub type TileClassId = EntityIdU16<TileClass>;
pub type CellSlotId = EntityIdU16<CellSlotTag>;
pub type TileSlotId = EntityIdU8<TileSlotTag>;

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct TileClass {
    pub slot: TileSlotId,
    pub cells: EntityVec<CellSlotId, String>,
    pub bitrects: EntityVec<BitRectId, BitRectInfo>,
    pub bels: EntityPartVec<BelSlotId, BelInfo>,
}

impl TileClass {
    pub fn new(slot: TileSlotId, num_cells: usize) -> Self {
        TileClass {
            slot,
            cells: EntityVec::from_iter((0..num_cells).map(|i| format!("CELL{i}"))),
            bitrects: EntityVec::new(),
            bels: Default::default(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct BitRectInfo {
    pub name: String,
    pub geometry: BitRectGeometry,
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

    pub const fn pos(self) -> PolTileWireCoord {
        PolTileWireCoord {
            tw: self,
            inv: false,
        }
    }

    pub const fn neg(self) -> PolTileWireCoord {
        PolTileWireCoord {
            tw: self,
            inv: true,
        }
    }

    pub fn to_string(self, db: &IntDb, tcls: &TileClass) -> String {
        if tcls.cells.len() == 1 {
            db.wires.key(self.wire).clone()
        } else {
            format!(
                "{cell}.{wire}",
                cell = tcls.cells[self.cell],
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
    OldTestMux,
    Legacy(LegacyBel),
}

// endregion:

// region: bels

#[derive(Default, Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct Bel {
    pub inputs: EntityPartVec<BelInputId, BelInput>,
    pub outputs: EntityPartVec<BelOutputId, BTreeSet<TileWireCoord>>,
    pub bidirs: EntityPartVec<BelBidirId, TileWireCoord>,
    pub attributes: EntityPartVec<BelAttributeId, BelAttribute>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub enum BelInput {
    Fixed(PolTileWireCoord),
    Invertible(TileWireCoord, PolTileBit),
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub enum BelAttribute {
    BitVec(Vec<PolTileBit>),
    Enum(BelAttributeEnum),
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct BelAttributeEnum {
    pub bits: Vec<TileBit>,
    pub values: EntityPartVec<EnumValueId, BitVec>,
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Encode, Decode)]
pub struct LegacyBel {
    pub pins: BTreeMap<String, BelPin>,
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

// endregion:

// region: routing

#[derive(Clone, Debug, Eq, PartialEq, Default, Encode, Decode)]
pub struct SwitchBox {
    pub items: Vec<SwitchBoxItem>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub enum SwitchBoxItem {
    Mux(Mux),
    ProgBuf(ProgBuf),
    PermaBuf(PermaBuf),
    Pass(Pass),
    BiPass(BiPass),
    ProgInv(ProgInv),
    ProgDelay(ProgDelay),
    Bidi(Bidi),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct Mux {
    pub dst: TileWireCoord,
    pub bits: Vec<TileBit>,
    pub src: BTreeMap<PolTileWireCoord, BitVec>,
    pub bits_off: Option<BitVec>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct ProgBuf {
    pub dst: TileWireCoord,
    pub src: PolTileWireCoord,
    pub bit: PolTileBit,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct PermaBuf {
    pub dst: TileWireCoord,
    pub src: PolTileWireCoord,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct Pass {
    pub dst: TileWireCoord,
    pub src: TileWireCoord,
    pub bit: PolTileBit,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct BiPass {
    pub a: TileWireCoord,
    pub b: TileWireCoord,
    pub bit: PolTileBit,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct ProgInv {
    pub dst: TileWireCoord,
    pub src: TileWireCoord,
    pub bit: PolTileBit,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct ProgDelay {
    pub dst: TileWireCoord,
    pub src: PolTileWireCoord,
    pub bits: Vec<TileBit>,
    pub steps: Vec<BitVec>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct Bidi {
    pub conn: ConnectorSlotId,
    pub wire: TileWireCoord,
    // bit set iff driver upstream
    pub bit_upstream: PolTileBit,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode, Default)]
pub struct TestMux {
    pub bits: Vec<TileBit>,
    pub groups: Vec<BitVec>,
    pub bits_primary: BitVec,
    pub wires: BTreeMap<TileWireCoord, TestMuxWire>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct TestMuxWire {
    pub primary_src: PolTileWireCoord,
    pub test_src: Vec<Option<PolTileWireCoord>>,
}

// endregion:

// region: index

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
                            for &src in mux.src.keys() {
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
                        SwitchBoxItem::ProgDelay(ref delay) => {
                            pips_fwd
                                .entry(delay.src.tw)
                                .or_default()
                                .insert(PolTileWireCoord {
                                    tw: delay.dst,
                                    inv: delay.src.inv,
                                });
                            pips_bwd.entry(delay.dst).or_default().insert(delay.src);
                        }
                        SwitchBoxItem::Bidi(_) => (),
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

// endregion:
