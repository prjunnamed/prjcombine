use prjcombine_interconnect::db::PadKind;
use prjcombine_types::bsdata::BitRectGeometry;
use proc_macro::{Ident, Literal, Span};

#[derive(Debug)]
pub enum TopItem {
    Variant(Ident),
    BitRectClass(BitRectClass),
    EnumClass(EnumClass),
    BelClass(BelClass),
    TileSlot(TileSlot),
    RegionSlot(RegionSlot),
    ConnectorSlot(ConnectorSlot),
    Wire(Wire),
    ForLoop(ForLoop<TopItem>),
    If(If<TopItem>),
}

#[derive(Debug)]
pub struct ForLoop<T> {
    pub var: Ident,
    pub iterator: ForIterator,
    pub items: Vec<T>,
}

#[derive(Debug)]
pub enum ForIterator {
    Range(core::ops::Range<usize>),
    RangeInclusive(core::ops::RangeInclusive<usize>),
}

#[derive(Debug)]
pub struct If<T> {
    pub span: Span,
    pub branches: Vec<(IfCond, Vec<T>)>,
    pub else_items: Vec<T>,
}

#[derive(Debug)]
pub enum IfCond {
    Variant(Vec<Ident>),
    TileClass(Vec<TemplateId>),
    BelSlot(Vec<ArrayIdRef>),
}

#[derive(Debug)]
pub struct BitRectClass {
    pub name: Ident,
    pub geometry: BitRectGeometry,
}

#[derive(Debug)]
pub struct EnumClass {
    pub name: Ident,
    pub values: Vec<Ident>,
}

// region: BelClass

#[derive(Debug)]
pub struct BelClass {
    pub name: Ident,
    pub items: Vec<BelClassItem>,
}

#[derive(Debug)]
pub enum BelClassItem {
    Input(BelClassPin),
    Output(BelClassPin),
    Bidir(BelClassPin),
    Pad(BelClassPad),
    Attribute(BelClassAttribute),
    ForLoop(ForLoop<BelClassItem>),
    If(If<BelClassItem>),
}

#[derive(Debug)]
pub struct BelClassPin {
    pub names: Vec<ArrayIdDef>,
    pub nonroutable: bool,
}

#[derive(Debug)]
pub struct BelClassPad {
    pub names: Vec<ArrayIdDef>,
    pub kind: PadKind,
}

#[derive(Debug)]
pub struct BelClassAttribute {
    pub names: Vec<TemplateId>,
    pub typ: AttributeType,
}

#[derive(Debug)]
pub enum AttributeType {
    Bool,
    BitVec(usize),
    Enum(Ident),
}

// endregion

// region: Tile classes and bel slots

#[derive(Debug)]
pub struct TileSlot {
    pub name: TemplateId,
    pub items: Vec<TileSlotItem>,
}

#[derive(Debug)]
pub enum TileSlotItem {
    BelSlot(BelSlot),
    TileClass(TileClass),
    ForLoop(ForLoop<TileSlotItem>),
    If(If<TileSlotItem>),
}

#[derive(Debug)]
pub struct BelSlot {
    pub name: ArrayIdDef,
    pub kind: BelKind,
}

#[derive(Debug)]
pub enum BelKind {
    Routing,
    Class(Ident),
    Legacy,
}

#[derive(Debug)]
pub struct TileClass {
    pub names: Vec<TemplateId>,
    pub items: Vec<TileClassItem>,
}

#[derive(Debug)]
pub enum TileClassItem {
    Cell(Vec<ArrayIdDef>),
    BitRect(ArrayIdDef, Ident),
    SwitchBox(SwitchBox),
    Bel(Bel),
    ForLoop(ForLoop<TileClassItem>),
    If(If<TileClassItem>),
}

#[derive(Debug)]
pub struct SwitchBox {
    pub slot: ArrayIdRef,
    pub items: Vec<SwitchBoxItem>,
}

#[derive(Debug)]
pub enum SwitchBoxItem {
    PermaBuf(WireRef, PolWireRef),
    ProgBuf(WireRef, PolWireRef),
    ProgInv(WireRef, WireRef),
    Mux(WireRef, Vec<PolWireRef>),
    ForLoop(ForLoop<SwitchBoxItem>),
    If(If<SwitchBoxItem>),
}

#[derive(Debug)]
pub struct Bel {
    pub slot: ArrayIdRef,
    pub items: Vec<BelItem>,
}

#[derive(Debug)]
pub enum BelItem {
    Input(ArrayIdRef, PolWireRef),
    Output(ArrayIdRef, Vec<WireRef>),
    Bidir(ArrayIdRef, WireRef),
    ForLoop(ForLoop<BelItem>),
    If(If<BelItem>),
}

// endregion

// region: Wires and connectors

#[derive(Debug)]
pub struct RegionSlot {
    pub name: TemplateId,
}

#[derive(Debug)]
pub struct ConnectorSlot {
    pub name: TemplateId,
    pub items: Vec<ConnectorSlotItem>,
}

#[derive(Debug)]
pub enum ConnectorSlotItem {
    Opposite(TemplateId),
    ConnectorClass(ConnectorClass),
    ForLoop(ForLoop<ConnectorSlotItem>),
    If(If<ConnectorSlotItem>),
}

#[derive(Debug)]
pub struct ConnectorClass {
    pub names: Vec<TemplateId>,
    pub items: Vec<ConnectorClassItem>,
}

#[derive(Debug)]
pub enum ConnectorClassItem {
    Blackhole(ArrayIdRef),
    Pass(ArrayIdRef, ArrayIdRef),
    Reflect(ArrayIdRef, ArrayIdRef),
    ForLoop(ForLoop<ConnectorClassItem>),
    If(If<ConnectorClassItem>),
}

#[derive(Debug)]
pub struct Wire {
    pub name: ArrayIdDef,
    pub kind: WireKind,
}

#[derive(Debug)]
pub enum WireKind {
    Tie0,
    Tie1,
    TiePullup,
    Regional(TemplateId),
    Mux,
    Bel,
    Test,
    MultiRoot,
    MultiBranch(Ident),
    Branch(Ident),
}

// endregion

#[derive(Debug)]
pub enum TemplateId {
    Raw(Ident),
    String(Literal),
}

impl TemplateId {
    pub fn span(&self) -> Span {
        match self {
            TemplateId::Raw(ident) => ident.span(),
            TemplateId::String(literal) => literal.span(),
        }
    }
}

#[derive(Debug)]
pub enum ArrayIdRef {
    Plain(TemplateId),
    Indexed(TemplateId, Index),
}

impl ArrayIdRef {
    pub fn span(&self) -> Span {
        match self {
            ArrayIdRef::Plain(id) => id.span(),
            ArrayIdRef::Indexed(id, _) => id.span(),
        }
    }
}

#[derive(Debug)]
pub enum Index {
    Ident(Ident, usize),
    Literal(usize),
}

#[derive(Debug)]
pub enum ArrayIdDef {
    Plain(TemplateId),
    Array(TemplateId, usize),
}

#[derive(Debug)]
pub enum WireRef {
    Simple(ArrayIdRef),
    Qualified(ArrayIdRef, ArrayIdRef),
}

#[derive(Debug)]
pub enum PolWireRef {
    Pos(WireRef),
    Neg(WireRef),
}

macro_rules! impl_from {
    ($ty:ident) => {
        impl From<If<$ty>> for $ty {
            fn from(value: If<Self>) -> Self {
                Self::If(value)
            }
        }
        impl From<ForLoop<$ty>> for $ty {
            fn from(value: ForLoop<Self>) -> Self {
                Self::ForLoop(value)
            }
        }
    };
}

impl_from!(TopItem);
impl_from!(BelClassItem);
impl_from!(TileSlotItem);
impl_from!(TileClassItem);
impl_from!(SwitchBoxItem);
impl_from!(BelItem);
impl_from!(ConnectorSlotItem);
impl_from!(ConnectorClassItem);
