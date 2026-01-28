use prjcombine_entity::{EntityBundleMap, EntityVec};
use prjcombine_interconnect::db::{
    BelAttributeId, BelBidirId, BelClassId, BelInputId, BelOutputId, BelPadId, BelPinIndexing,
    BelSlotId, CellSlotId, ConnectorClassId, ConnectorSlotId, EnumClassId, EnumValueId, IntDb,
    RegionSlotId, TableFieldId, TableId, TableRowId, TileClassId, TileSlotId, WireSlotId,
};
use prjcombine_types::bsdata::BitRectId;
use proc_macro::Ident;

#[derive(Default)]
pub struct AnnotatedBelClass {
    pub input_id: EntityBundleMap<BelInputId, (Ident, BelPinIndexing)>,
    pub output_id: EntityBundleMap<BelOutputId, (Ident, BelPinIndexing)>,
    pub bidir_id: EntityBundleMap<BelBidirId, (Ident, BelPinIndexing)>,
    pub pad_id: EntityBundleMap<BelPadId, Ident>,
    pub attr_id: EntityVec<BelAttributeId, Ident>,
}

#[derive(Default)]
pub struct AnnotatedTable {
    pub field_id: EntityVec<TableFieldId, Ident>,
    pub row_id: EntityVec<TableRowId, Ident>,
}

pub struct AnnotatedDb {
    pub name: Option<Ident>,
    pub db: IntDb,
    pub enum_id: EntityVec<EnumClassId, Ident>,
    pub eval_id: EntityVec<EnumClassId, EntityVec<EnumValueId, Ident>>,
    pub tslot_id: EntityVec<TileSlotId, Ident>,
    pub cslot_id: EntityVec<ConnectorSlotId, Ident>,
    pub bslot_id: EntityBundleMap<BelSlotId, Ident>,
    pub rslot_id: EntityVec<RegionSlotId, Ident>,
    pub tcls_id: EntityVec<TileClassId, Ident>,
    pub tcls_cell_id: EntityVec<TileClassId, EntityBundleMap<CellSlotId, Ident>>,
    pub tcls_bitrect_id: EntityVec<TileClassId, EntityBundleMap<BitRectId, Ident>>,
    pub ccls_id: EntityVec<ConnectorClassId, Ident>,
    pub bcls_id: EntityVec<BelClassId, Ident>,
    pub bcls: EntityVec<BelClassId, AnnotatedBelClass>,
    pub wire_id: EntityBundleMap<WireSlotId, Ident>,
    pub table_id: EntityVec<TableId, Ident>,
    pub table: EntityVec<TableId, AnnotatedTable>,
}
