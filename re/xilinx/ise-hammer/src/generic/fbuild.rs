use prjcombine_interconnect::{
    db::{
        BelAttributeId, BelAttributeType, BelInputId, BelKind, BelSlotId, EnumValueId,
        PolTileWireCoord, TableRowId, TileClassId, TileWireCoord,
    },
    dir::DirV,
    grid::TileCoord,
};
use prjcombine_re_collector::diff::{DiffKey, FeatureId, SpecialId};
use prjcombine_re_fpga_hammer::{FpgaFuzzerGen, FuzzerProp};
use prjcombine_re_hammer::Session;
use prjcombine_types::bitvec::BitVec;
use prjcombine_xilinx_bitstream::Reg;

use crate::{
    backend::{IseBackend, Key, MultiValue, PinFromKind, Value},
    generic::{
        props::extra::{
            ExtraKeyBelAttrBits, ExtraKeyBelAttrValue, ExtraKeyBelSpecial, ExtraKeyBelSpecialRow,
            ExtraKeyLegacy, ExtraKeyLegacyAttr, ExtraKeyRouting,
        },
        utils::get_input_name,
    },
};

use super::props::{
    BaseRaw, DynProp, FuzzRaw, FuzzRawMulti, NullBits,
    bel::{
        BaseBelAttr, BaseBelMode, BaseBelNoPin, BaseBelPin, BaseBelPinFrom, BaseBelPinPips,
        BaseGlobalXy, BelMutex, ForceBelName, FuzzBelAttr, FuzzBelMode, FuzzBelMultiAttr,
        FuzzBelPin, FuzzBelPinFrom, FuzzBelPinIntPipsInput, FuzzBelPinPair, FuzzBelPinPips,
        FuzzGlobalXy, FuzzMultiGlobalXy, GlobalMutexHere, RowMutexHere,
    },
    extra::{ExtraGtz, ExtraReg, ExtraTile, ExtraTilesByBel, ExtraTilesByClass},
    mutex::{IntMutex, RowMutex, TileMutex, TileMutexExclusive},
    pip::{BasePip, BelIntoPipWire, FuzzPip},
    relation::{FixedRelation, HasRelated, NoopRelation, Related, TileRelation},
};

pub struct FuzzCtx<'sm, 'a> {
    pub session: &'sm mut Session<'a, IseBackend<'a>>,
    pub backend: &'a IseBackend<'a>,
    pub tile_class: Option<TileClassId>,
}

impl<'sm, 'b> FuzzCtx<'sm, 'b> {
    pub fn new(
        session: &'sm mut Session<'b, IseBackend<'b>>,
        backend: &'b IseBackend<'b>,
        tcid: TileClassId,
    ) -> Self {
        Self {
            session,
            backend,
            tile_class: Some(tcid),
        }
    }

    pub fn try_new(
        session: &'sm mut Session<'b, IseBackend<'b>>,
        backend: &'b IseBackend<'b>,
        tcid: TileClassId,
    ) -> Option<Self> {
        if backend.edev.tile_index[tcid].is_empty() {
            return None;
        }
        Some(Self {
            session,
            backend,
            tile_class: Some(tcid),
        })
    }

    pub fn new_null(
        session: &'sm mut Session<'b, IseBackend<'b>>,
        backend: &'b IseBackend<'b>,
    ) -> Self {
        Self {
            session,
            backend,
            tile_class: None,
        }
    }

    pub fn bel<'c>(&'c mut self, bel: BelSlotId) -> FuzzCtxBel<'c, 'b> {
        FuzzCtxBel {
            session: &mut *self.session,
            backend: self.backend,
            tile_class: self.tile_class.unwrap(),
            bel,
            sub: 0,
        }
    }

    pub fn test_manual_legacy<'nsm>(
        &'nsm mut self,
        bel: &'static str,
        attr: impl AsRef<str>,
        val: impl AsRef<str>,
    ) -> FuzzBuilderTestManual<'nsm, 'b> {
        self.build().test_manual_legacy(bel, attr, val)
    }

    pub fn test_reg_legacy<'nsm>(
        &'nsm mut self,
        reg: Reg,
        tile: impl Into<String>,
        bel: &'static str,
        attr: impl AsRef<str>,
        val: impl AsRef<str>,
    ) -> FuzzBuilderTestManual<'nsm, 'b> {
        self.build().test_reg(reg, tile, bel, attr, val)
    }

    pub fn build<'nsm>(&'nsm mut self) -> FuzzBuilder<'nsm, 'b> {
        FuzzBuilder {
            session: &mut *self.session,
            backend: self.backend,
            tile_class: self.tile_class,
            props: vec![],
        }
    }
}

pub trait FuzzBuilderBase<'b>: Sized {
    fn prop_box(self, prop: Box<DynProp<'b>>) -> Self;
    fn backend(&self) -> &'b IseBackend<'b>;

    fn prop(self, prop: impl FuzzerProp<'b, IseBackend<'b>> + 'b) -> Self {
        self.prop_box(Box::new(prop))
    }

    fn raw(self, key: Key<'b>, val: impl Into<Value<'b>>) -> Self {
        self.prop(BaseRaw::new(key, val.into()))
    }

    fn raw_diff(
        self,
        key: Key<'b>,
        val0: impl Into<Value<'b>>,
        val1: impl Into<Value<'b>>,
    ) -> Self {
        self.prop(FuzzRaw::new(key, val0.into(), val1.into()))
    }

    fn global(self, opt: impl Into<String>, val: impl Into<String>) -> Self {
        self.raw(Key::GlobalOpt(opt.into()), val.into())
    }

    fn no_global(self, opt: impl Into<String>) -> Self {
        self.raw(Key::GlobalOpt(opt.into()), None)
    }

    fn global_mutex(self, key: impl Into<String>, val: impl Into<Value<'b>>) -> Self {
        self.raw(Key::GlobalMutex(key.into()), val.into())
    }

    fn row_mutex(self, key: impl Into<String>, val: impl Into<String>) -> Self {
        let prop = RowMutex::new(key.into(), val.into());
        self.prop(prop)
    }

    fn tile_mutex(self, key: impl Into<String>, val: impl Into<Value<'b>>) -> Self {
        let prop = TileMutex::new(key.into(), val.into());
        self.prop(prop)
    }

    fn tile_mutex_exclusive(self, key: impl Into<String>) -> Self {
        let prop = TileMutexExclusive::new(key.into());
        self.prop(prop)
    }

    fn related_tile_mutex<R: TileRelation + 'b>(
        self,
        relation: R,
        key: impl Into<String>,
        val: impl Into<Value<'b>>,
    ) -> Self {
        let prop = Related::new(relation, TileMutex::new(key.into(), val.into()));
        self.prop(prop)
    }

    fn related_tile_mutex_exclusive<R: TileRelation + 'b>(
        self,
        relation: R,
        key: impl Into<String>,
    ) -> Self {
        let prop = Related::new(relation, TileMutexExclusive::new(key.into()));
        self.prop(prop)
    }

    fn maybe_prop(self, prop: Option<impl FuzzerProp<'b, IseBackend<'b>> + 'b>) -> Self {
        if let Some(prop) = prop {
            self.prop(prop)
        } else {
            self
        }
    }

    fn extra_tile_legacy<R: TileRelation + 'b>(self, relation: R, bel: impl Into<String>) -> Self {
        self.prop(ExtraTile::new(relation, ExtraKeyLegacy::new(bel.into())))
    }

    fn extra_tile_attr_legacy<R: TileRelation + 'b>(
        self,
        relation: R,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> Self {
        self.prop(ExtraTile::new(
            relation,
            ExtraKeyLegacyAttr::new(bel.into(), attr.into(), val.into()),
        ))
    }

    fn extra_tile_attr_fixed_legacy(
        self,
        tcrd: TileCoord,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> Self {
        self.extra_tile_attr_legacy(FixedRelation(tcrd), bel, attr, val)
    }

    fn extra_tiles_by_kind_legacy(self, kind: impl AsRef<str>, bel: impl Into<String>) -> Self {
        let kind = self.backend().edev.db.get_tile_class(kind.as_ref());
        self.prop(ExtraTilesByClass::new(
            kind,
            ExtraKeyLegacy::new(bel.into()),
        ))
    }

    fn extra_tiles_by_class_bel_special(
        self,
        tcid: TileClassId,
        bslot: BelSlotId,
        spec: SpecialId,
    ) -> Self {
        self.prop(ExtraTilesByClass::new(
            tcid,
            ExtraKeyBelSpecial::new(bslot, spec),
        ))
    }

    fn extra_tiles_by_bel_legacy(self, slot: BelSlotId, bel: impl Into<String>) -> Self {
        self.prop(ExtraTilesByBel::new(slot, ExtraKeyLegacy::new(bel.into())))
    }

    fn extra_tiles_by_bel_special(self, slot: BelSlotId, spec: SpecialId) -> Self {
        self.prop(ExtraTilesByBel::new(
            slot,
            ExtraKeyBelSpecial::new(slot, spec),
        ))
    }

    fn extra_tiles_by_bel_attr_bits(self, slot: BelSlotId, attr: BelAttributeId) -> Self {
        self.prop(ExtraTilesByBel::new(
            slot,
            ExtraKeyBelAttrBits::new(slot, attr, 0, true),
        ))
    }

    fn extra_tiles_by_bel_attr_val(
        self,
        slot: BelSlotId,
        attr: BelAttributeId,
        val: EnumValueId,
    ) -> Self {
        self.prop(ExtraTilesByBel::new(
            slot,
            ExtraKeyBelAttrValue::new(slot, attr, val),
        ))
    }

    fn extra_tile_reg(self, reg: Reg, tile: impl Into<String>, bel: impl Into<String>) -> Self {
        self.prop(ExtraReg::new(
            vec![reg],
            false,
            tile.into(),
            Some(bel.into()),
            None,
            None,
        ))
    }

    fn extra_tile_reg_present(
        self,
        reg: Reg,
        tile: impl Into<String>,
        bel: impl Into<String>,
    ) -> Self {
        self.prop(ExtraReg::new(
            vec![reg],
            true,
            tile.into(),
            Some(bel.into()),
            None,
            None,
        ))
    }

    fn extra_tile_reg_attr_legacy(
        self,
        reg: Reg,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> Self {
        self.prop(ExtraReg::new(
            vec![reg],
            false,
            tile.into(),
            Some(bel.into()),
            Some(attr.into()),
            Some(val.into()),
        ))
    }

    fn extra_tile_attr_bits<R: TileRelation + 'b>(
        self,
        relation: R,
        bslot: BelSlotId,
        attr: BelAttributeId,
    ) -> Self {
        self.prop(ExtraTile::new(
            relation,
            ExtraKeyBelAttrBits::new(bslot, attr, 0, true),
        ))
    }

    fn extra_tile_bel_special<R: TileRelation + 'b>(
        self,
        relation: R,
        bslot: BelSlotId,
        spec: SpecialId,
    ) -> Self {
        self.prop(ExtraTile::new(
            relation,
            ExtraKeyBelSpecial::new(bslot, spec),
        ))
    }

    fn extra_tile_routing<R: TileRelation + 'b>(
        self,
        relation: R,
        dst: TileWireCoord,
        src: PolTileWireCoord,
    ) -> Self {
        self.prop(ExtraTile::new(relation, ExtraKeyRouting::new(dst, src)))
    }

    fn extra_fixed_bel_attr_val(
        self,
        tcrd: TileCoord,
        bslot: BelSlotId,
        attr: BelAttributeId,
        val: EnumValueId,
    ) -> Self {
        self.prop(ExtraTile::new(
            FixedRelation(tcrd),
            ExtraKeyBelAttrValue::new(bslot, attr, val),
        ))
    }

    fn extra_fixed_bel_attr_bits(
        self,
        tcrd: TileCoord,
        bslot: BelSlotId,
        attr: BelAttributeId,
    ) -> Self {
        self.prop(ExtraTile::new(
            FixedRelation(tcrd),
            ExtraKeyBelAttrBits::new(bslot, attr, 0, true),
        ))
    }

    fn extra_fixed_bel_attr_bits_bi(
        self,
        tcrd: TileCoord,
        bslot: BelSlotId,
        attr: BelAttributeId,
        val: bool,
    ) -> Self {
        self.prop(ExtraTile::new(
            FixedRelation(tcrd),
            ExtraKeyBelAttrBits::new(bslot, attr, 0, val),
        ))
    }

    fn extra_fixed_bel_attr_bits_base_bi(
        self,
        tcrd: TileCoord,
        bslot: BelSlotId,
        attr: BelAttributeId,
        base: usize,
        val: bool,
    ) -> Self {
        self.prop(ExtraTile::new(
            FixedRelation(tcrd),
            ExtraKeyBelAttrBits::new(bslot, attr, base, val),
        ))
    }

    fn extra_fixed_bel_special(self, tcrd: TileCoord, bslot: BelSlotId, spec: SpecialId) -> Self {
        self.prop(ExtraTile::new(
            FixedRelation(tcrd),
            ExtraKeyBelSpecial::new(bslot, spec),
        ))
    }

    fn extra_fixed_bel_special_row(
        self,
        tcrd: TileCoord,
        bslot: BelSlotId,
        spec: SpecialId,
        row: TableRowId,
    ) -> Self {
        self.prop(ExtraTile::new(
            FixedRelation(tcrd),
            ExtraKeyBelSpecialRow::new(bslot, spec, row),
        ))
    }

    fn null_bits(self) -> Self {
        self.prop(NullBits)
    }

    fn no_related<R: TileRelation + 'b>(self, relation: R) -> Self {
        self.prop(HasRelated::new(relation, false))
    }

    fn has_related<R: TileRelation + 'b>(self, relation: R) -> Self {
        self.prop(HasRelated::new(relation, true))
    }
}

pub struct FuzzBuilder<'sm, 'b> {
    pub session: &'sm mut Session<'b, IseBackend<'b>>,
    pub backend: &'b IseBackend<'b>,
    pub tile_class: Option<TileClassId>,
    pub props: Vec<Box<DynProp<'b>>>,
}

impl<'b> FuzzBuilderBase<'b> for FuzzBuilder<'_, 'b> {
    fn prop_box(mut self, prop: Box<DynProp<'b>>) -> Self {
        self.props.push(prop);
        self
    }

    fn backend(&self) -> &'b IseBackend<'b> {
        self.backend
    }
}

impl<'sm, 'b> FuzzBuilder<'sm, 'b> {
    pub fn props(mut self, props: impl IntoIterator<Item = Box<DynProp<'b>>>) -> Self {
        self.props.extend(props);
        self
    }

    pub fn test_raw(self, key: DiffKey) -> FuzzBuilderTestManual<'sm, 'b> {
        FuzzBuilderTestManual {
            session: self.session,
            tile_class: self.tile_class,
            props: self.props,
            key,
        }
    }

    pub fn test_manual_legacy(
        self,
        bel: &'static str,
        attr: impl AsRef<str>,
        val: impl AsRef<str>,
    ) -> FuzzBuilderTestManual<'sm, 'b> {
        let attr = attr.as_ref();
        let val = val.as_ref();
        let key = DiffKey::Legacy(FeatureId {
            tile: if let Some(tile_class) = self.tile_class {
                self.backend.edev.db.tile_classes.key(tile_class).clone()
            } else {
                "NULL".into()
            },
            bel: bel.into(),
            attr: attr.into(),
            val: val.into(),
        });
        self.test_raw(key)
    }

    pub fn test_routing(
        self,
        wt: TileWireCoord,
        wf: PolTileWireCoord,
    ) -> FuzzBuilderTestManual<'sm, 'b> {
        let key = DiffKey::Routing(self.tile_class.unwrap(), wt, wf);
        self.test_raw(key)
    }

    pub fn test_global_special(self, spec: SpecialId) -> FuzzBuilderTestManual<'sm, 'b> {
        self.test_raw(DiffKey::GlobalSpecial(spec))
    }

    pub fn test_reg(
        self,
        reg: Reg,
        tile: impl Into<String>,
        bel: &'static str,
        attr: impl AsRef<str>,
        val: impl AsRef<str>,
    ) -> FuzzBuilderTestManual<'sm, 'b> {
        let attr = attr.as_ref();
        let val = val.as_ref();
        self.extra_tile_reg(reg, tile, bel)
            .test_manual_legacy(bel, attr, val)
    }

    pub fn test_gtz(
        self,
        dir: DirV,
        attr: impl AsRef<str>,
        val: impl AsRef<str>,
    ) -> FuzzBuilderTestManual<'sm, 'b> {
        self.prop(ExtraGtz(dir))
            .test_manual_legacy("GTZ", attr, val)
    }
}

#[must_use]
pub struct FuzzBuilderTestManual<'sm, 'b> {
    pub session: &'sm mut Session<'b, IseBackend<'b>>,
    pub tile_class: Option<TileClassId>,
    pub props: Vec<Box<DynProp<'b>>>,
    pub key: DiffKey,
}

impl<'b> FuzzBuilderTestManual<'_, 'b> {
    pub fn prop(mut self, prop: impl FuzzerProp<'b, IseBackend<'b>> + 'b) -> Self {
        self.props.push(Box::new(prop));
        self
    }

    pub fn prop_box(mut self, prop: Box<DynProp<'b>>) -> Self {
        self.props.push(prop);
        self
    }

    pub fn raw_diff(
        self,
        key: Key<'b>,
        val0: impl Into<Value<'b>>,
        val1: impl Into<Value<'b>>,
    ) -> Self {
        self.prop(FuzzRaw::new(key, val0.into(), val1.into()))
    }

    pub fn raw_multi(self, key: Key<'b>, val: MultiValue, width: usize) {
        self.prop(FuzzRawMulti::new(key, val, width)).commit();
    }

    pub fn global(self, opt: impl Into<String>, val: impl Into<String>) -> Self {
        self.raw_diff(Key::GlobalOpt(opt.into()), None, val.into())
    }

    pub fn global_diff(
        self,
        opt: impl Into<String>,
        val0: impl Into<String>,
        val1: impl Into<String>,
    ) -> Self {
        self.raw_diff(Key::GlobalOpt(opt.into()), val0.into(), val1.into())
    }

    pub fn commit(self) {
        let fgen = FpgaFuzzerGen {
            tile_class: self.tile_class,
            key: self.key,
            props: self.props,
        };
        self.session.add_fuzzer(Box::new(fgen));
    }

    pub fn multi_global(self, opt: impl Into<String>, val: MultiValue, width: usize) {
        self.raw_multi(Key::GlobalOpt(opt.into()), val, width);
    }
}

pub struct FuzzCtxBel<'sm, 'b> {
    pub session: &'sm mut Session<'b, IseBackend<'b>>,
    pub backend: &'b IseBackend<'b>,
    pub tile_class: TileClassId,
    pub bel: BelSlotId,
    pub sub: usize,
}

impl<'b> FuzzCtxBel<'_, 'b> {
    pub fn build<'sm>(&'sm mut self) -> FuzzBuilderBel<'sm, 'b> {
        FuzzBuilderBel {
            session: &mut *self.session,
            backend: self.backend,
            tile_class: self.tile_class,
            bel: self.bel,
            test_bel: self.bel,
            sub: self.sub,
            props: vec![],
        }
    }

    pub fn mode<'sm>(&'sm mut self, mode: impl Into<String>) -> FuzzBuilderBel<'sm, 'b> {
        self.build().mode(mode)
    }

    pub fn test_manual_legacy<'sm>(
        &'sm mut self,
        attr: impl AsRef<str>,
        val: impl AsRef<str>,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        self.build().test_manual_legacy(attr, val)
    }

    pub fn sub(mut self, sub: usize) -> Self {
        self.sub = sub;
        self
    }
}

pub struct FuzzBuilderBel<'sm, 'b> {
    pub session: &'sm mut Session<'b, IseBackend<'b>>,
    pub backend: &'b IseBackend<'b>,
    pub tile_class: TileClassId,
    pub bel: BelSlotId,
    pub test_bel: BelSlotId,
    pub sub: usize,
    pub props: Vec<Box<DynProp<'b>>>,
}

impl<'b> FuzzBuilderBase<'b> for FuzzBuilderBel<'_, 'b> {
    fn prop_box(mut self, prop: Box<DynProp<'b>>) -> Self {
        self.props.push(prop);
        self
    }

    fn backend(&self) -> &'b IseBackend<'b> {
        self.backend
    }
}

impl<'sm, 'b> FuzzBuilderBel<'sm, 'b> {
    // Note: this is not an implementation of the Clone trait because Clone::clone has a slightly
    // different signature.
    pub fn clone(&mut self) -> FuzzBuilderBel<'_, 'b> {
        FuzzBuilderBel {
            session: &mut *self.session,
            backend: self.backend,
            tile_class: self.tile_class,
            bel: self.bel,
            test_bel: self.test_bel,
            sub: self.sub,
            props: self.props.clone(),
        }
    }

    pub fn test_bel(mut self, bel: BelSlotId) -> Self {
        self.test_bel = bel;
        self
    }

    pub fn props(mut self, props: impl IntoIterator<Item = Box<DynProp<'b>>>) -> Self {
        self.props.extend(props);
        self
    }

    pub fn force_bel_name(self, bel_name: impl Into<String>) -> Self {
        self.prop(ForceBelName(bel_name.into()))
    }

    pub fn global_xy(self, opt: impl Into<String>, val: impl Into<String>) -> Self {
        let prop = BaseGlobalXy::new(self.bel, opt.into(), val.into());
        self.prop(prop)
    }

    pub fn mode(self, mode: impl Into<String>) -> Self {
        let bel = self.bel;
        let sub = self.sub;
        self.bel_sub_mode(bel, sub, mode)
    }

    pub fn bel_mode(self, bel: BelSlotId, mode: impl Into<String>) -> Self {
        self.bel_sub_mode(bel, 0, mode)
    }

    pub fn bel_sub_mode(self, bel: BelSlotId, sub: usize, mode: impl Into<String>) -> Self {
        let prop = BaseBelMode::new(bel, sub, mode.into());
        self.prop(IntMutex::new("MAIN".into())).prop(prop)
    }

    pub fn unused(self) -> Self {
        let bel = self.bel;
        let sub = self.sub;
        self.bel_sub_unused(bel, sub)
    }

    pub fn sub_unused(self, sub: usize) -> Self {
        let bel = self.bel;
        self.bel_sub_unused(bel, sub)
    }

    pub fn bel_unused(self, bel: BelSlotId) -> Self {
        self.bel_sub_unused(bel, 0)
    }

    pub fn bel_sub_unused(self, bel: BelSlotId, sub: usize) -> Self {
        let prop = BaseBelMode::new(bel, sub, "".into());
        self.prop(prop)
    }

    pub fn global_mutex_here(self, key: impl Into<String>) -> Self {
        let prop = GlobalMutexHere::new(self.bel, key.into());
        self.prop(prop)
    }

    pub fn row_mutex_here(self, key: impl Into<String>) -> Self {
        let prop = RowMutexHere::new(self.bel, key.into());
        self.prop(prop)
    }

    pub fn pin(self, pin: impl Into<String>) -> Self {
        let bel = self.bel;
        let sub = self.sub;
        self.bel_sub_pin(bel, sub, pin)
    }

    pub fn bel_pin(self, bel: BelSlotId, pin: impl Into<String>) -> Self {
        self.bel_sub_pin(bel, 0, pin)
    }

    pub fn bel_sub_pin(self, bel: BelSlotId, sub: usize, pin: impl Into<String>) -> Self {
        self.prop(BaseBelPin::new(bel, sub, pin.into()))
    }

    pub fn no_pin(self, pin: impl Into<String>) -> Self {
        let prop = BaseBelNoPin::new(self.bel, self.sub, pin.into());
        self.prop(prop)
    }

    pub fn pin_pips(self, pin: impl Into<String>) -> Self {
        let prop = BaseBelPinPips::new(self.bel, pin.into());
        self.prop(prop)
    }

    pub fn pin_from(self, pin: impl Into<String>, from: PinFromKind) -> Self {
        let prop = BaseBelPinFrom::new(self.bel, self.sub, pin.into(), from);
        self.prop(prop)
    }

    pub fn attr(self, attr: impl Into<String>, val: impl Into<String>) -> Self {
        let bel = self.bel;
        let sub = self.sub;
        self.bel_sub_attr(bel, sub, attr, val)
    }

    pub fn bel_attr(self, bel: BelSlotId, attr: impl Into<String>, val: impl Into<String>) -> Self {
        self.bel_sub_attr(bel, 0, attr, val)
    }

    pub fn bel_sub_attr(
        self,
        bel: BelSlotId,
        sub: usize,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> Self {
        let prop = BaseBelAttr::new(bel, sub, attr.into(), val.into());
        self.prop(prop)
    }

    pub fn mutex(self, key: impl Into<String>, val: impl Into<Value<'b>>) -> Self {
        let bel = self.bel;
        self.bel_mutex(bel, key, val)
    }

    pub fn bel_mutex(
        self,
        bel: BelSlotId,
        key: impl Into<String>,
        val: impl Into<Value<'b>>,
    ) -> Self {
        let prop = BelMutex::new(bel, key.into(), val.into());
        self.prop(prop)
    }

    pub fn pip(self, wire_to: impl BelIntoPipWire, wire_from: impl BelIntoPipWire) -> Self {
        self.related_pip(NoopRelation, wire_to, wire_from)
    }

    pub fn related_pip<R: TileRelation + 'b>(
        self,
        relation: R,
        wire_to: impl BelIntoPipWire,
        wire_from: impl BelIntoPipWire,
    ) -> Self {
        let wire_to = wire_to.into_pip_wire(self.backend, self.bel);
        let wire_from = wire_from.into_pip_wire(self.backend, self.bel);
        let prop = BasePip::new(relation, wire_to, wire_from);
        self.prop(prop)
    }

    pub fn test_enum_legacy(mut self, attr: impl AsRef<str>, vals: &[impl AsRef<str>]) {
        let attr = attr.as_ref();
        for val in vals {
            let val = val.as_ref();
            self.clone()
                .test_manual_legacy(attr, val)
                .attr(attr, val)
                .commit();
        }
    }

    pub fn test_enum_suffix_legacy(
        mut self,
        attr: impl AsRef<str>,
        suffix: impl AsRef<str>,
        vals: &[impl AsRef<str>],
    ) {
        let attr = attr.as_ref();
        let suffix = suffix.as_ref();
        for val in vals {
            let val = val.as_ref();
            self.clone()
                .test_manual_legacy(format!("{attr}.{suffix}"), val)
                .attr(attr, val)
                .commit();
        }
    }

    pub fn test_inv_legacy(self, pin: impl Into<String>) {
        let pin = pin.into();
        let pininv = format!("{pin}INV");
        let pin_b = format!("{pin}_B");
        self.pin(&pin).test_enum_legacy(pininv, &[pin, pin_b]);
    }

    pub fn test_inv_suffix(self, pin: impl Into<String>, suffix: impl AsRef<str>) {
        let pin = pin.into();
        let pininv = format!("{pin}INV");
        let pin_b = format!("{pin}_B");
        self.pin(&pin)
            .test_enum_suffix_legacy(pininv, suffix, &[pin, pin_b]);
    }

    pub fn test_multi_attr_bin_legacy(self, attr: impl Into<String>, width: usize) {
        let attr = attr.into();
        let prop = FuzzBelMultiAttr::new(self.bel, self.sub, attr.clone(), MultiValue::Bin, width);
        self.test_manual_legacy(attr, "").prop(prop).commit();
    }

    pub fn test_multi_attr_dec_legacy(self, attr: impl Into<String>, width: usize) {
        let attr = attr.into();
        let prop =
            FuzzBelMultiAttr::new(self.bel, self.sub, attr.clone(), MultiValue::Dec(0), width);
        self.test_manual_legacy(attr, "").prop(prop).commit();
    }

    pub fn test_multi_attr_dec_delta(self, attr: impl Into<String>, width: usize, delta: i32) {
        let attr = attr.into();
        let prop = FuzzBelMultiAttr::new(
            self.bel,
            self.sub,
            attr.clone(),
            MultiValue::Dec(delta),
            width,
        );
        self.test_manual_legacy(attr, "").prop(prop).commit();
    }

    pub fn test_multi_attr_hex_legacy(self, attr: impl Into<String>, width: usize) {
        let attr = attr.into();
        let prop =
            FuzzBelMultiAttr::new(self.bel, self.sub, attr.clone(), MultiValue::Hex(0), width);
        self.test_manual_legacy(attr, "").prop(prop).commit();
    }

    pub fn test_multi_attr_lut(self, attr: impl Into<String>, width: usize) {
        let attr = attr.into();
        let prop = FuzzBelMultiAttr::new(self.bel, self.sub, attr.clone(), MultiValue::Lut, width);
        self.test_manual_legacy(attr, "#LUT").prop(prop).commit();
    }

    pub fn test_bel_attr_multi(self, attr: BelAttributeId, value: MultiValue) {
        let BelKind::Class(bcid) = self.backend.edev.db.bel_slots[self.test_bel].kind else {
            unreachable!()
        };
        let BelAttributeType::BitVec(width) = self.backend.edev.db[bcid].attributes[attr].typ
        else {
            unreachable!()
        };
        let prop = FuzzBelMultiAttr::new(
            self.bel,
            self.sub,
            self.backend.edev.db[bcid].attributes.key(attr).to_string(),
            value,
            width,
        );
        self.test_bel_attr_bits(attr).prop(prop).commit();
    }

    pub fn test_raw(self, key: DiffKey) -> FuzzBuilderBelTestManual<'sm, 'b> {
        FuzzBuilderBelTestManual {
            session: self.session,
            backend: self.backend,
            tile_class: self.tile_class,
            bel: self.bel,
            sub: self.sub,
            props: self.props,
            key,
        }
    }

    pub fn test_manual_legacy(
        self,
        attr: impl AsRef<str>,
        val: impl AsRef<str>,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let attr = attr.as_ref();
        let val = val.as_ref();
        let key = DiffKey::Legacy(FeatureId {
            tile: self
                .backend
                .edev
                .db
                .tile_classes
                .key(self.tile_class)
                .clone(),
            bel: self.backend.edev.db.bel_slots.key(self.test_bel).clone(),
            attr: attr.into(),
            val: val.into(),
        });
        self.test_raw(key)
    }

    pub fn test_routing(
        self,
        wt: TileWireCoord,
        wf: PolTileWireCoord,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::Routing(self.tile_class, wt, wf);
        self.test_raw(key)
    }

    pub fn test_routing_special(
        self,
        wire: TileWireCoord,
        spec: SpecialId,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::RoutingSpecial(self.tile_class, wire, spec);
        self.test_raw(key)
    }

    pub fn test_routing_pair_special(
        self,
        wt: TileWireCoord,
        wf: PolTileWireCoord,
        spec: SpecialId,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::RoutingPairSpecial(self.tile_class, wt, wf, spec);
        self.test_raw(key)
    }

    pub fn test_bel_special(self, spec: SpecialId) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::BelSpecial(self.tile_class, self.test_bel, spec);
        self.test_raw(key)
    }

    pub fn test_bel_special_special(
        self,
        spec1: SpecialId,
        spec2: SpecialId,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::BelSpecialSpecial(self.tile_class, self.test_bel, spec1, spec2);
        self.test_raw(key)
    }

    pub fn test_bel_special_val(
        self,
        spec: SpecialId,
        val: EnumValueId,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::BelSpecialVal(self.tile_class, self.test_bel, spec, val);
        self.test_raw(key)
    }

    pub fn test_bel_special_bits(self, spec: SpecialId) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::BelSpecialBit(self.tile_class, self.test_bel, spec, 0);
        self.test_raw(key)
    }

    pub fn test_bel_special_u32(
        self,
        spec: SpecialId,
        val: u32,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::BelSpecialU32(self.tile_class, self.test_bel, spec, val);
        self.test_raw(key)
    }

    pub fn test_bel_special_row(
        self,
        spec: SpecialId,
        row: TableRowId,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::BelSpecialRow(self.tile_class, self.test_bel, spec, row);
        self.test_raw(key)
    }

    pub fn test_bel_sss_row(
        self,
        spec0: SpecialId,
        spec1: SpecialId,
        spec2: SpecialId,
        row: TableRowId,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::BelSpecialSpecialSpecialRow(
            self.tile_class,
            self.test_bel,
            spec0,
            spec1,
            spec2,
            row,
        );
        self.test_raw(key)
    }

    pub fn test_bel_attr_bits(self, attr: BelAttributeId) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::BelAttrBit(self.tile_class, self.test_bel, attr, 0, true);
        self.test_raw(key)
    }

    pub fn test_bel_attr_bits_bi(
        self,
        attr: BelAttributeId,
        val: bool,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::BelAttrBit(self.tile_class, self.test_bel, attr, 0, val);
        self.test_raw(key)
    }

    pub fn test_bel_attr_bits_base(
        self,
        attr: BelAttributeId,
        base: usize,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::BelAttrBit(self.tile_class, self.test_bel, attr, base, true);
        self.test_raw(key)
    }

    pub fn test_bel_attr_bits_base_bi(
        self,
        attr: BelAttributeId,
        base: usize,
        val: bool,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::BelAttrBit(self.tile_class, self.test_bel, attr, base, val);
        self.test_raw(key)
    }

    pub fn test_bel_attr_val(
        self,
        attr: BelAttributeId,
        val: EnumValueId,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::BelAttrValue(self.tile_class, self.test_bel, attr, val);
        self.test_raw(key)
    }

    pub fn test_bel_attr_bitvec(
        self,
        attr: BelAttributeId,
        val: BitVec,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::BelAttrBitVec(self.tile_class, self.test_bel, attr, val);
        self.test_raw(key)
    }

    pub fn test_bel_attr_bitvec_u32_width(
        self,
        attr: BelAttributeId,
        val: u32,
        width: usize,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let mut bv = BitVec::repeat(false, width);
        for i in 0..width {
            bv.set(i, (val & 1 << i) != 0);
        }
        self.test_bel_attr_bitvec(attr, bv)
    }

    pub fn test_bel_attr_bitvec_u32(
        self,
        attr: BelAttributeId,
        val: u32,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let BelKind::Class(bcid) = self.backend.edev.db.bel_slots[self.test_bel].kind else {
            unreachable!()
        };
        let BelAttributeType::BitVec(width) = self.backend.edev.db[bcid].attributes[attr].typ
        else {
            unreachable!()
        };
        self.test_bel_attr_bitvec_u32_width(attr, val, width)
    }

    pub fn test_bel_attr_u32(
        self,
        attr: BelAttributeId,
        val: u32,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::BelAttrU32(self.tile_class, self.test_bel, attr, val);
        self.test_raw(key)
    }

    pub fn test_bel_attr_special(
        self,
        attr: BelAttributeId,
        spec: SpecialId,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::BelAttrSpecial(self.tile_class, self.test_bel, attr, spec);
        self.test_raw(key)
    }

    pub fn test_bel_attr_special_bits(
        self,
        attr: BelAttributeId,
        special: SpecialId,
        base: usize,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let diff_key =
            DiffKey::BelAttrSpecialBit(self.tile_class, self.test_bel, attr, special, base, true);
        self.test_raw(diff_key)
    }

    pub fn test_bel_attr_special_bits_bi(
        self,
        attr: BelAttributeId,
        special: SpecialId,
        base: usize,
        val: bool,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let diff_key =
            DiffKey::BelAttrSpecialBit(self.tile_class, self.test_bel, attr, special, base, val);
        self.test_raw(diff_key)
    }

    pub fn test_bel_attr_special_val(
        self,
        attr: BelAttributeId,
        spec: SpecialId,
        val: EnumValueId,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::BelAttrSpecialValue(self.tile_class, self.test_bel, attr, spec, val);
        self.test_raw(key)
    }

    pub fn test_bel_attr_row(
        self,
        attr: BelAttributeId,
        row: TableRowId,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::BelAttrRow(self.tile_class, self.test_bel, attr, row);
        self.test_raw(key)
    }

    pub fn test_bel_attr_bool_rename(
        &mut self,
        rattr: impl Into<String>,
        attr: BelAttributeId,
        val0: impl Into<String>,
        val1: impl Into<String>,
    ) {
        let rattr = rattr.into();
        self.clone()
            .test_bel_attr_bits_bi(attr, false)
            .attr(&rattr, val0)
            .commit();
        self.clone()
            .test_bel_attr_bits_bi(attr, true)
            .attr(rattr, val1)
            .commit();
    }

    pub fn test_bel_attr_bool_auto(
        &mut self,
        attr: BelAttributeId,
        val0: impl Into<String>,
        val1: impl Into<String>,
    ) {
        let BelKind::Class(bcid) = self.backend.edev.db.bel_slots[self.test_bel].kind else {
            unreachable!()
        };
        let name = self.backend.edev.db[bcid].attributes.key(attr);
        self.test_bel_attr_bool_rename(name, attr, val0, val1);
    }

    pub fn test_bel_attr_bool_special_rename(
        &mut self,
        rattr: impl Into<String>,
        attr: BelAttributeId,
        spec: SpecialId,
        val0: impl Into<String>,
        val1: impl Into<String>,
    ) {
        let rattr = rattr.into();
        self.clone()
            .test_bel_attr_special_bits_bi(attr, spec, 0, false)
            .attr(&rattr, val0)
            .commit();
        self.clone()
            .test_bel_attr_special_bits_bi(attr, spec, 0, true)
            .attr(rattr, val1)
            .commit();
    }

    pub fn test_bel_attr_bool_special_auto(
        &mut self,
        attr: BelAttributeId,
        spec: SpecialId,
        val0: impl Into<String>,
        val1: impl Into<String>,
    ) {
        let BelKind::Class(bcid) = self.backend.edev.db.bel_slots[self.test_bel].kind else {
            unreachable!()
        };
        let name = self.backend.edev.db[bcid].attributes.key(attr);
        self.test_bel_attr_bool_special_rename(name, attr, spec, val0, val1);
    }

    pub fn test_global_attr_bool_rename(
        &mut self,
        opt: impl Into<String>,
        attr: BelAttributeId,
        val0: impl Into<String>,
        val1: impl Into<String>,
    ) {
        let opt = opt.into();
        self.clone()
            .test_bel_attr_bits_bi(attr, false)
            .global(&opt, val0)
            .commit();
        self.clone()
            .test_bel_attr_bits_bi(attr, true)
            .global(opt, val1)
            .commit();
    }

    pub fn test_global_attr_rename(&mut self, opt: impl Into<String>, attr: BelAttributeId) {
        let opt = opt.into();
        let BelKind::Class(bcid) = self.backend.edev.db.bel_slots[self.test_bel].kind else {
            unreachable!()
        };
        let BelAttributeType::Enum(ecid) = self.backend.edev.db[bcid].attributes[attr].typ else {
            unreachable!()
        };
        let ecls = &self.backend.edev.db[ecid];
        for (vid, val) in &ecls.values {
            let val = val.strip_prefix('_').unwrap_or(val);
            self.clone()
                .test_bel_attr_val(attr, vid)
                .global(&opt, val)
                .commit();
        }
    }

    pub fn test_bel_attr_rename(mut self, rattr: impl AsRef<str>, attr: BelAttributeId) {
        let rattr = rattr.as_ref();
        let BelKind::Class(bcid) = self.backend.edev.db.bel_slots[self.test_bel].kind else {
            unreachable!()
        };
        let BelAttributeType::Enum(ecid) = self.backend.edev.db[bcid].attributes[attr].typ else {
            unreachable!()
        };
        let ecls = &self.backend.edev.db[ecid];
        for (vid, val) in &ecls.values {
            let mut val = val.strip_prefix('_').unwrap_or(&val[..]);
            if val == "CONST_0" {
                val = "0";
            }
            if val == "CONST_1" {
                val = "1";
            }
            self.clone()
                .test_bel_attr_val(attr, vid)
                .attr(rattr, val)
                .commit();
        }
    }

    pub fn test_bel_attr_special_rename(
        mut self,
        rattr: impl AsRef<str>,
        attr: BelAttributeId,
        spec: SpecialId,
    ) {
        let rattr = rattr.as_ref();
        let BelKind::Class(bcid) = self.backend.edev.db.bel_slots[self.test_bel].kind else {
            unreachable!()
        };
        let BelAttributeType::Enum(ecid) = self.backend.edev.db[bcid].attributes[attr].typ else {
            unreachable!()
        };
        let ecls = &self.backend.edev.db[ecid];
        for (vid, val) in &ecls.values {
            let mut val = val.strip_prefix('_').unwrap_or(&val[..]);
            if val == "CONST_0" {
                val = "0";
            }
            if val == "CONST_1" {
                val = "1";
            }
            self.clone()
                .test_bel_attr_special_val(attr, spec, vid)
                .attr(rattr, val)
                .commit();
        }
    }

    pub fn test_bel_attr_default_rename(
        mut self,
        rattr: impl AsRef<str>,
        attr: BelAttributeId,
        default: EnumValueId,
    ) {
        let rattr = rattr.as_ref();
        let BelKind::Class(bcid) = self.backend.edev.db.bel_slots[self.test_bel].kind else {
            unreachable!()
        };
        let BelAttributeType::Enum(ecid) = self.backend.edev.db[bcid].attributes[attr].typ else {
            unreachable!()
        };
        let ecls = &self.backend.edev.db[ecid];
        for (vid, val) in &ecls.values {
            let val = val.strip_prefix('_').unwrap_or(&val[..]);
            if vid == default {
                continue;
            }
            self.clone()
                .test_bel_attr_val(attr, vid)
                .attr(rattr, val)
                .commit();
        }
    }

    pub fn test_bel_attr_subset_rename(
        mut self,
        rattr: impl AsRef<str>,
        attr: BelAttributeId,
        vals: &[EnumValueId],
    ) {
        let rattr = rattr.as_ref();
        let BelKind::Class(bcid) = self.backend.edev.db.bel_slots[self.test_bel].kind else {
            unreachable!()
        };
        let BelAttributeType::Enum(ecid) = self.backend.edev.db[bcid].attributes[attr].typ else {
            unreachable!()
        };
        let ecls = &self.backend.edev.db[ecid];
        for &vid in vals {
            let val = &ecls.values[vid];
            let val = val.strip_prefix('_').unwrap_or(&val[..]);
            self.clone()
                .test_bel_attr_val(attr, vid)
                .attr(rattr, val)
                .commit();
        }
    }

    pub fn test_bel_input_inv_enum(
        &mut self,
        rattr: impl Into<String>,
        pin: BelInputId,
        val0: impl Into<String>,
        val1: impl Into<String>,
    ) {
        let rattr = rattr.into();
        self.clone()
            .test_bel_input_inv(pin, false)
            .attr(&rattr, val0)
            .commit();
        self.clone()
            .test_bel_input_inv(pin, true)
            .attr(rattr, val1)
            .commit();
    }

    pub fn test_bel_input_inv_auto(self, pid: BelInputId) {
        let BelKind::Class(bcid) = self.backend.edev.db.bel_slots[self.bel].kind else {
            unreachable!()
        };
        let pname = get_input_name(self.backend.edev, bcid, pid);
        self.pin(&pname).test_bel_input_inv_enum(
            format!("{pname}INV"),
            pid,
            &pname,
            format!("{pname}_B"),
        );
    }

    pub fn test_bel_input_inv_special_auto(self, pid: BelInputId, spec: SpecialId) {
        let BelKind::Class(bcid) = self.backend.edev.db.bel_slots[self.bel].kind else {
            unreachable!()
        };
        let pname = get_input_name(self.backend.edev, bcid, pid);
        self.test_bel_input_inv_special_rename(&pname, pid, spec);
    }

    pub fn test_bel_input_inv_special_rename(
        mut self,
        pname: &str,
        pid: BelInputId,
        spec: SpecialId,
    ) {
        let rattr = format!("{pname}INV");
        self.clone()
            .pin(pname)
            .test_bel_input_inv_special(pid, spec, false)
            .attr(&rattr, pname)
            .commit();
        self.clone()
            .pin(pname)
            .test_bel_input_inv_special(pid, spec, true)
            .attr(rattr, format!("{pname}_B"))
            .commit();
    }

    pub fn test_bel_attr_auto(self, attr: BelAttributeId) {
        let BelKind::Class(bcid) = self.backend.edev.db.bel_slots[self.test_bel].kind else {
            unreachable!()
        };
        let rattr = self.backend.edev.db[bcid].attributes.key(attr);
        self.test_bel_attr_rename(rattr, attr)
    }

    pub fn test_bel_attr_special_auto(self, attr: BelAttributeId, spec: SpecialId) {
        let BelKind::Class(bcid) = self.backend.edev.db.bel_slots[self.test_bel].kind else {
            unreachable!()
        };
        let rattr = self.backend.edev.db[bcid].attributes.key(attr);
        self.test_bel_attr_special_rename(rattr, attr, spec)
    }

    pub fn test_bel_attr_auto_default(self, attr: BelAttributeId, val: EnumValueId) {
        let BelKind::Class(bcid) = self.backend.edev.db.bel_slots[self.test_bel].kind else {
            unreachable!()
        };
        let rattr = self.backend.edev.db[bcid].attributes.key(attr);
        self.test_bel_attr_default_rename(rattr, attr, val)
    }

    pub fn test_bel_input_inv(
        self,
        pin: BelInputId,
        val: bool,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::BelInputInv(self.tile_class, self.test_bel, pin, val);
        self.test_raw(key)
    }

    pub fn test_bel_input_inv_special(
        self,
        pin: BelInputId,
        spec: SpecialId,
        val: bool,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let key = DiffKey::BelInputInvSpecial(self.tile_class, self.test_bel, pin, spec, val);
        self.test_raw(key)
    }
}

#[must_use]
pub struct FuzzBuilderBelTestManual<'sm, 'b> {
    pub session: &'sm mut Session<'b, IseBackend<'b>>,
    pub backend: &'b IseBackend<'b>,
    pub tile_class: TileClassId,
    pub bel: BelSlotId,
    pub sub: usize,
    pub props: Vec<Box<DynProp<'b>>>,
    pub key: DiffKey,
}

impl<'b> FuzzBuilderBelTestManual<'_, 'b> {
    pub fn prop(mut self, prop: impl FuzzerProp<'b, IseBackend<'b>> + 'b) -> Self {
        self.props.push(Box::new(prop));
        self
    }

    pub fn raw_diff(
        self,
        key: Key<'b>,
        val0: impl Into<Value<'b>>,
        val1: impl Into<Value<'b>>,
    ) -> Self {
        self.prop(FuzzRaw::new(key, val0.into(), val1.into()))
    }

    pub fn raw_multi(self, key: Key<'b>, val: MultiValue, width: usize) {
        self.prop(FuzzRawMulti::new(key, val, width)).commit();
    }

    pub fn global(self, opt: impl Into<String>, val: impl Into<String>) -> Self {
        self.raw_diff(Key::GlobalOpt(opt.into()), None, val.into())
    }

    pub fn global_diff(
        self,
        opt: impl Into<String>,
        val0: impl Into<String>,
        val1: impl Into<String>,
    ) -> Self {
        self.raw_diff(Key::GlobalOpt(opt.into()), val0.into(), val1.into())
    }

    pub fn global_xy(self, opt: impl Into<String>, val: impl Into<String>) -> Self {
        let prop = FuzzGlobalXy::new(self.bel, opt.into(), None, Some(val.into()));
        self.prop(prop)
    }

    pub fn mode(self, mode: impl Into<String>) -> Self {
        let mode = mode.into();
        let prop = FuzzBelMode::new(self.bel, self.sub, "".into(), mode);
        self.prop(IntMutex::new("MAIN".into())).prop(prop)
    }

    pub fn mode_diff(self, mode0: impl Into<String>, mode1: impl Into<String>) -> Self {
        let bel = self.bel;
        let sub = self.sub;
        self.bel_sub_mode_diff(bel, sub, mode0, mode1)
    }

    pub fn bel_mode_diff(
        self,
        bel: BelSlotId,
        mode0: impl Into<String>,
        mode1: impl Into<String>,
    ) -> Self {
        self.bel_sub_mode_diff(bel, 0, mode0, mode1)
    }

    pub fn bel_sub_mode_diff(
        self,
        bel: BelSlotId,
        sub: usize,
        mode0: impl Into<String>,
        mode1: impl Into<String>,
    ) -> Self {
        let mode0 = mode0.into();
        let mode1 = mode1.into();
        let prop = FuzzBelMode::new(bel, sub, mode0, mode1);
        self.prop(IntMutex::new("MAIN".into())).prop(prop)
    }

    pub fn attr(self, attr: impl Into<String>, val: impl Into<String>) -> Self {
        let bel = self.bel;
        let sub = self.sub;
        self.bel_sub_attr(bel, sub, attr, val)
    }

    pub fn bel_attr(self, bel: BelSlotId, attr: impl Into<String>, val: impl Into<String>) -> Self {
        self.bel_sub_attr(bel, 0, attr, val)
    }

    pub fn bel_sub_attr(
        self,
        bel: BelSlotId,
        sub: usize,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> Self {
        let prop = FuzzBelAttr::new(bel, sub, attr.into(), "".into(), val.into());
        self.prop(prop)
    }

    pub fn attr_diff(
        self,
        attr: impl Into<String>,
        val_a: impl Into<String>,
        val_b: impl Into<String>,
    ) -> Self {
        let bel = self.bel;
        let sub = self.sub;
        self.bel_sub_attr_diff(bel, sub, attr, val_a, val_b)
    }

    pub fn bel_attr_diff(
        self,
        bel: BelSlotId,
        attr: impl Into<String>,
        val_a: impl Into<String>,
        val_b: impl Into<String>,
    ) -> Self {
        self.bel_sub_attr_diff(bel, 0, attr, val_a, val_b)
    }

    pub fn bel_sub_attr_diff(
        self,
        bel: BelSlotId,
        sub: usize,
        attr: impl Into<String>,
        val_a: impl Into<String>,
        val_b: impl Into<String>,
    ) -> Self {
        let prop = FuzzBelAttr::new(bel, sub, attr.into(), val_a.into(), val_b.into());
        self.prop(prop)
    }

    pub fn pin(self, pin: impl Into<String>) -> Self {
        let bel = self.bel;
        let sub = self.sub;
        self.bel_sub_pin(bel, sub, pin)
    }

    pub fn bel_pin(self, bel: BelSlotId, pin: impl Into<String>) -> Self {
        self.bel_sub_pin(bel, 0, pin)
    }

    pub fn bel_sub_pin(self, bel: BelSlotId, sub: usize, pin: impl Into<String>) -> Self {
        let prop = FuzzBelPin::new(bel, sub, pin.into());
        self.prop(prop)
    }

    pub fn pin_pips(self, pin: impl Into<String>) -> Self {
        let prop = FuzzBelPinPips::new(self.bel, pin.into());
        self.prop(prop)
    }

    pub fn pin_int_pips_input(self, pin: BelInputId) -> Self {
        let prop = FuzzBelPinIntPipsInput::new(self.bel, pin);
        self.prop(prop)
    }

    pub fn pin_from(self, pin: impl Into<String>, from0: PinFromKind, from1: PinFromKind) -> Self {
        let prop = FuzzBelPinFrom::new(self.bel, self.sub, pin.into(), from0, from1);
        self.prop(prop)
    }

    pub fn pip(self, wire_to: impl BelIntoPipWire, wire_from: impl BelIntoPipWire) -> Self {
        self.related_pip(NoopRelation, wire_to, wire_from)
    }

    pub fn pin_pair(
        self,
        pin_to: impl Into<String>,
        bel_from: BelSlotId,
        pin_from: impl Into<String>,
    ) -> Self {
        let prop = FuzzBelPinPair::new(
            self.bel,
            self.sub,
            pin_to.into(),
            bel_from,
            0,
            pin_from.into(),
        );
        self.prop(prop)
    }

    pub fn related_pip<R: TileRelation + 'b>(
        self,
        relation: R,
        wire_to: impl BelIntoPipWire,
        wire_from: impl BelIntoPipWire,
    ) -> Self {
        let wire_to = wire_to.into_pip_wire(self.backend, self.bel);
        let wire_from = wire_from.into_pip_wire(self.backend, self.bel);
        let prop = FuzzPip::new(relation, wire_to, wire_from);
        self.prop(prop)
    }

    pub fn commit(self) {
        let fgen = FpgaFuzzerGen {
            tile_class: Some(self.tile_class),
            key: self.key,
            props: self.props,
        };
        self.session.add_fuzzer(Box::new(fgen));
    }

    pub fn multi_global(self, opt: impl Into<String>, val: MultiValue, width: usize) {
        self.raw_multi(Key::GlobalOpt(opt.into()), val, width);
    }

    pub fn multi_attr(self, attr: impl Into<String>, val: MultiValue, width: usize) {
        let attr = attr.into();
        let prop = FuzzBelMultiAttr::new(self.bel, self.sub, attr, val, width);
        self.prop(prop).commit();
    }

    pub fn multi_global_xy(self, opt: impl Into<String>, val: MultiValue, width: usize) {
        let opt = opt.into();
        let prop = FuzzMultiGlobalXy::new(self.bel, opt, val, width);
        self.prop(prop).commit();
    }
}
