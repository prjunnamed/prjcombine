use prjcombine_entity::{EntityBundleMap, EntityVec};
use prjcombine_interconnect::db::{
    BelAttributeId, BelBidirId, BelClassId, BelInputId, BelOutputId, BelPadId, BelSlotId,
    CellSlotId, ConnectorClassId, ConnectorSlotId, EnumClassId, EnumValueId, IntDb, RegionSlotId,
    TileClassId, TileSlotId, WireSlotId,
};
use proc_macro::Ident;

#[derive(Default)]
pub struct AnnotatedBelClass {
    pub input_id: EntityBundleMap<BelInputId, Ident>,
    pub output_id: EntityBundleMap<BelOutputId, Ident>,
    pub bidir_id: EntityBundleMap<BelBidirId, Ident>,
    pub pad_id: EntityBundleMap<BelPadId, Ident>,
    pub attr_id: EntityVec<BelAttributeId, Ident>,
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
    pub ccls_id: EntityVec<ConnectorClassId, Ident>,
    pub bcls_id: EntityVec<BelClassId, Ident>,
    pub bcls: EntityVec<BelClassId, AnnotatedBelClass>,
    pub wire_id: EntityBundleMap<WireSlotId, Ident>,
}
