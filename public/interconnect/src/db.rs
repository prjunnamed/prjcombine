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

/// Identifies a region slot.
///
/// A region is the set of cells in which a particular regional wire is present. If multiple wires
/// inhabit the same collection of cells, one region is used to describe all of them.
///
/// Since an FPGA can contain a rich variety of regional wires, a single cell will usually belong
/// to multiple regions. Moreover, since the structure of the interconnect is regular, each cell
/// will belong to the same number of regions. Thus, there is a fixed number of region slots,
/// constant between all cells.
///
/// Each type of regional wire is associated with a fixed region slot.
pub type RegionSlotId = EntityIdU8<RegionSlotTag>;

impl EntityTag for WireKind {
    const PREFIX: &'static str = "WIRE";
}
pub type WireSlotId = EntityIdU16<WireKind>;

pub trait WireSlotIdExt {
    fn cell(self, idx: usize) -> TileWireCoord;
}

impl WireSlotIdExt for WireSlotId {
    fn cell(self, idx: usize) -> TileWireCoord {
        TileWireCoord::new_idx(idx, self)
    }
}

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

/// An active element of a tile, identified by a `BelSlotId`.
///
/// Comes in several variants:
///
/// - a [`SwitchBox`], which is a container for a bunch of small generic interconnect elements
/// - a [`TestMux`], which is a special test-only multiplexer (not included in [`SwitchBox`] for
///   historical reasons)
/// - a [`Bel`], which is a target-specific block described by a schema in the form of [`BelClass`]
/// - a [`LegacyBel`], which is a deprecated variant of [`Bel`] that is stringly-typed instead of
///   being described by a schema; is being slowly removed from the codebase
#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub enum BelInfo {
    SwitchBox(SwitchBox),
    Bel(Bel),
    TestMux(TestMux),
    /// Leftover invalid placeholder variant, to be removed along with `Legacy`.  Only exists
    /// to keep serialization format stable.
    OldTestMux,
    Legacy(LegacyBel),
}

// endregion:

// region: bels

/// A target-specific block within a tile, described by a schema in the form of [`BelClass`].
///
/// Any pin or attribute described in the [`BelClass`] may be missing in a particular instance
/// of a [`Bel`].  This is often used for bel classes that have optional features (such as a LUT
/// bel class that is used to describe both LUTs with LUTRAM functionality and ones without it).
/// The meaning of this and circumstances when this happens are all target-dependent.
#[derive(Default, Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct Bel {
    /// The wires connected to bel inputs.
    pub inputs: EntityPartVec<BelInputId, BelInput>,
    /// The wires connected to bel outputs.  A bel output can have multiple wires connected
    /// to it — the output value is driven onto all of them simultanously.
    pub outputs: EntityPartVec<BelOutputId, BTreeSet<TileWireCoord>>,
    /// The wires connected to bel bidirectional pins.
    pub bidirs: EntityPartVec<BelBidirId, TileWireCoord>,
    /// The bitstream encodings of bel attributes.
    pub attributes: EntityPartVec<BelAttributeId, BelAttribute>,
}

/// Describes the connection of a [`Bel`] input pin.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub enum BelInput {
    /// The bel input is connected directly to this interconnect wire.
    Fixed(PolTileWireCoord),
    /// The bel input is connected to this intereconnect wire via a programmable inverter.
    /// If the specified bitstream bit is set, the input is the inverted value of the wire.
    /// Otherwise, it is directly the value of the wire.
    Invertible(TileWireCoord, PolTileBit),
}

impl BelInput {
    /// Returns the underlying interconnect wire, discarding any information about inversions,
    /// programmable or not.
    pub fn wire(self) -> TileWireCoord {
        match self {
            BelInput::Fixed(ptwc) => ptwc.tw,
            BelInput::Invertible(twc, _) => twc,
        }
    }
}

/// Describes the bitstream encoding of a [`Bel`] attribute.
///
/// The [`BelAttributeType`] associated with the attribute in the [`BelClass`] determines
/// what values are valid here:
///
/// - [`BelAttributeType::Bool`]: the value must be a [`BelAttribute::BitVec`] with one bit.
/// - [`BelAttributeType::BitVec`]: the value must be a [`BelAttribute::BitVec`] with matching length.
/// - [`BelAttributeType::BitVecArray`]: the value must be a [`BelAttribute::BitVec`] with length
///   equal to `depth * width`; bit `b` of array item `i` corresponds to index `i * width + b` in
///   this vector.
/// - [`BelAttributeType::Enum`]: the value must be a [`BelAttribute::Enum`]; however, it may be
///   the case that some values of the enum are missing (non-encodeable).  Once again, the usage
///   and meaning of this is target-specific.
#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub enum BelAttribute {
    BitVec(Vec<PolTileBit>),
    Enum(BelAttributeEnum),
}

/// Describes the bitstream encoding of an enum-typed [`Bel`] attribute.
#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct BelAttributeEnum {
    /// The bitstream bits encoding this attribute.
    pub bits: Vec<TileBit>,
    /// The mapping of enum values to bit patterns stored in the above bits.  The mapping may be
    /// partial, but all values present must have unique encodings.
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

/// A set of routing elements.
///
/// Describes all interconnect that logically belongs to a particular tile.
#[derive(Clone, Debug, Eq, PartialEq, Default, Encode, Decode)]
pub struct SwitchBox {
    pub items: Vec<SwitchBoxItem>,
}

/// A single routing element contained in a [`SwitchBox`].
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub enum SwitchBoxItem {
    /// A programmable multiplexer (drives one of several selectable source wires onto the destination wire).
    Mux(Mux),
    /// A programmable buffer (drives the source wire onto destination wire if enabled).
    ProgBuf(ProgBuf),
    /// A permanent buffer (always drives the source wire onto destination wire).
    PermaBuf(PermaBuf),
    /// A programmable unidirectional pass gate.
    Pass(Pass),
    /// A programmable bidirectional pass gate (connects two wires together if enabled).
    BiPass(BiPass),
    /// A programmable inverter (drives either the source wire or its negation onto the destination wire).
    ProgInv(ProgInv),
    /// A programmable delay (like a `PermaBuf`, but its delay can be selected from one of several options).
    ProgDelay(ProgDelay),
    /// An always-on buffer with configurable direction (an obscure, rarely-used element).
    Bidi(Bidi),
    /// Like `Mux`, but deals with two wires at once (rarely used).
    PairMux(PairMux),
    /// A set of bits that need to be set whenever some wires are in use (an obscure, rarely-used element).
    WireSupport(WireSupport),
}

/// A programmable interconnect multiplexer.
///
/// Drives the value of one of the selected source wires onto the destination wire.
/// The source is selectable via bitstream bits.  If `bits_off` is provided, the multiplexer
/// can also be turned off (and the wire may or may not be driven by another multiplexer, possibly
/// in another tile).
///
/// The connection may be buffered or not; we do not store that information here.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct Mux {
    /// The destination wire.
    pub dst: TileWireCoord,
    /// The bitstream bits controlling the multiplexer.
    pub bits: Vec<TileBit>,
    /// The selectable source wires, with associated bit patterns.
    pub src: BTreeMap<PolTileWireCoord, BitVec>,
    /// If specified, the bit pattern that turns off the multiplexer (and allows other multiplexers
    /// or programmable buffers to drive the wire).
    pub bits_off: Option<BitVec>,
}

/// A programmable interconnect buffer.
///
/// Drives the value of the source wire onto the destination wire when enabled.
/// Otherwise, the destination wire can be driven by another buffer or other interconnect element.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct ProgBuf {
    /// The destination wire.
    pub dst: TileWireCoord,
    /// The source wire.
    pub src: PolTileWireCoord,
    /// The bitstream bit which enables this buffer.
    pub bit: PolTileBit,
}

/// A permanent interconnect connection.
///
/// Always drives the value of the source wire onto the destination wire.
///
/// Can be used to represent actual interconnect buffers or wire aliasing in cases not covered by
/// other tools.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct PermaBuf {
    /// The destination wire.
    pub dst: TileWireCoord,
    /// The source wire.
    pub src: PolTileWireCoord,
}

/// A programmable unidirectional connection using a pass gate.
///
/// Passes the value of the source wire onto the destination wire iff the bitstream bit is set.
///
/// Even though the connection is implemented using a pass gate (which is bidirectional by nature),
/// it should only be used in the specified direction for one reason or another (possibly
/// the reverse connection is useless because of interconnect topology, or the signal strength is
/// too low to be useful).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct Pass {
    /// The destination wire.
    pub dst: TileWireCoord,
    /// The source wire.
    pub src: TileWireCoord,
    /// The bitstream bit.
    pub bit: PolTileBit,
}

/// A programmable bidirectional connection using a pass gate.
///
/// Connects the two wires together using a pass gate iff the bitstream bit is set.  The connection
/// works in both directions at once, and is unbuffered by nature.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct BiPass {
    /// The first wire.
    pub a: TileWireCoord,
    /// The second wire.
    pub b: TileWireCoord,
    /// The bitstream bit.
    pub bit: PolTileBit,
}

/// A programmable interconnect inverter.
///
/// If the bitstream bit is unset, drives the value of the source wire onto the destination wire.
/// If the bit is set, drives the inverted value of the source wire onto the destination wire.
///
/// This is a permanent connection (it is not possible to not drive the destination wire), only
/// the polarity of the connection is programmable.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct ProgInv {
    /// The destination wire.
    pub dst: TileWireCoord,
    /// The source wire.
    pub src: TileWireCoord,
    /// The bitstream bit.
    pub bit: PolTileBit,
}

/// A programmable interconnect delay.
///
/// Always drives the value of the source wire onto the destination wire.  The delay of
/// the connection is programmable and can be set to one of predefined "steps" via bitstream bits.
/// The exact values of the delays are specified elsewhere (as are interconnect delays in general).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct ProgDelay {
    /// The destination wire.
    pub dst: TileWireCoord,
    /// A transparent, always-on buffer with programmable direction.
    pub src: PolTileWireCoord,
    /// The bitstream bits controlling the delay.
    pub bits: Vec<TileBit>,
    /// The bit patterns for the delays.
    ///
    /// Each entry in this vector represents one delay step, and the value is the bit pattern that
    /// should be set in the bitstream to select it.  The delay steps are listed in order from
    /// the smallest.
    pub steps: Vec<BitVec>,
}

/// A transparent, always-on buffer with programmable direction.
///
/// This is a peculiar and rarely used item.  In most cases, when there are two physical wire
/// segments with programmable buffers involved, they would be described in the interconnect
/// database as two wires with `ProgBuf` connections, or something similar.  However, there are
/// cases where the FPGA has buffers in both directions between two physical wires, and either one
/// or the other is always enabled (which one depends on a bitstream bit).  In this case, there
/// is no way to use the two physical wires independently — they must always have the same value
/// because of the buffering.  Representing this situation as two `ProgBuf`s would be wrong, as it
/// would imply being able to switch the buffering off.  In this case, we choose to represent both
/// physical wires as a single wire in the database, and use `Bidi` to describe the connection.
/// Since both physical wires map to the same `WireCoord` in our model, we need to use a connector
/// to identify the exact spot where buffering takes place, so we know what to put in the bitstream.
///
/// Describes a bitstream bit that has to be set iff the driver of the given wire is *upstream*
/// of the given connector slot within the cell specified by the wire coordinate.  That is,
/// if reaching the segment of the wire that is driven from the specified cell requires traversing
/// the specified connector.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct Bidi {
    /// The connector slot used to determine whether the driver is upstream or downstream of
    /// the buffer.  This connector slot is relative to the cell specified in `wire`.
    pub conn: ConnectorSlotId,
    /// The reference segment of the wire being bufferred.
    pub wire: TileWireCoord,
    /// The bitstream bit, to be set if the driver is upstream of the connector.
    pub bit_upstream: PolTileBit,
}

/// A programmable interconnect multiplexer for two wires at once.
///
/// Has two destination wires, and has multiple selections of pairs of source wires controlled
/// by the bitstream.  The bitstream field controls what is driven to both destination wires
/// at once — the destination wires are not individually controllable.
///
/// The connection may be buffered or not.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct PairMux {
    /// The destination wires.
    pub dst: [TileWireCoord; 2],
    /// The bitstream bits controlling the multiplexer.
    pub bits: Vec<TileBit>,
    /// The possible selections of the two source wires, along with the corresponding bit pattern.
    /// One or both of the source wires may be `None`, which means that the value driven on
    /// the corresponding destination wire is undefined for this selection.  A `[None, None]`
    /// selection may be used to turn off the mux.
    pub src: BTreeMap<[Option<PolTileWireCoord>; 2], BitVec>,
}

/// A piece of interconnect that needs to be enabled for some wires to work.
///
/// Represents some unspecified kind of interconnect circuitry that can be enabled via bitstream,
/// and enabling it is necessary for some set of interconnect wires to work.
///
/// Has multiple bitstream bits and multiple associated wires.  All of the bitstream bits need to
/// be set if any of the associated wires is used in any capacity, including usage only in other
/// faraway tiles.  Otherwise, behavior is undefined.
///
/// It is mostly unknown what these bits do.  They may control power to interconnect circuitry, or
/// termination for long-distance differential transmission lines.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub struct WireSupport {
    /// The associated wires.
    pub wires: BTreeSet<TileWireCoord>,
    /// The bits to be set if any of the wires is in use.
    pub bits: Vec<PolTileBit>,
}

/// A wide multiplexer for testing purposes.
///
/// The multiplexer always drives a particular set of destination wires.  It has a set of bitstream
/// bits that select either the "primary" mode, or one of several test modes.  The selected mode
/// determines which set of source wires will be buffered onto the destination wires.
///
/// In normal operation, this multiplexer should always be set to the primary selection,
/// and the test mode inputs ignored.  The test mode settings are only used for the purpose
/// of testing the interconnect circuitry itself (at the factory).
#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode, Default)]
pub struct TestMux {
    /// The bits controlling the multiplexer.
    pub bits: Vec<TileBit>,
    /// The bit settings for the test modes.  Each entry in this vector corresponds to one test
    /// mode.
    pub groups: Vec<BitVec>,
    /// The bit setting for the primary mode.
    pub bits_primary: BitVec,
    /// The wires driven by this multiplexer and their sources.  The keys are destination wires,
    /// and the values describe the source wires.
    pub wires: BTreeMap<TileWireCoord, TestMuxWire>,
}

/// A set of source wires corresponding to a particular destination wire of a [`TestMux`].
#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct TestMuxWire {
    /// The source wire used for primary mode.
    pub primary_src: PolTileWireCoord,
    /// The source wires used for test modes, if any.  The indices of this vector correspond
    /// directly to the indices in the `groups` vector in the [`TestMux`], and the vectors must
    /// have the same length.
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
                        SwitchBoxItem::PairMux(ref mux) => {
                            for &src in mux.src.keys() {
                                for (wt, wf) in mux.dst.into_iter().zip(src) {
                                    let Some(wf) = wf else { continue };
                                    pips_fwd.entry(wf.tw).or_default().insert(PolTileWireCoord {
                                        tw: wt,
                                        inv: wf.inv,
                                    });
                                    pips_bwd.entry(wt).or_default().insert(wf);
                                }
                            }
                        }
                        SwitchBoxItem::WireSupport(_) => (),
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
