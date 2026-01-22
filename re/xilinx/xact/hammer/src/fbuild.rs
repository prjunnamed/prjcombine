use prjcombine_interconnect::{
    db::{
        BelAttributeId, BelAttributeType, BelBidirId, BelInputId, BelKind, BelSlotId, EnumValueId,
        TileClassId,
    },
    grid::TileCoord,
};
use prjcombine_re_fpga_hammer::{DiffKey, FpgaFuzzerGen, FuzzerProp, SpecialId};
use prjcombine_re_hammer::Session;
use prjcombine_types::bitvec::BitVec;

use crate::{
    backend::{Key, Value, XactBackend},
    props::{
        BaseBelConfig, BaseBelMode, BaseBelMutex, BaseRaw, BidirMutexExclusive, BondedIo, DynProp,
        ExtraTile, FuzzBelConfig, FuzzBelConfigDiff, FuzzBelMode, FuzzBelPipPin, FuzzEquate,
        FuzzEquateFixed, FuzzRaw, InputMutexExclusive, NullBits,
    },
};

pub struct FuzzCtx<'sm, 'a> {
    pub session: &'sm mut Session<'a, XactBackend<'a>>,
    pub backend: &'a XactBackend<'a>,
    pub tile_class: TileClassId,
}

impl<'sm, 'a> FuzzCtx<'sm, 'a> {
    pub fn new(
        session: &'sm mut Session<'a, XactBackend<'a>>,
        backend: &'a XactBackend<'a>,
        tile_class: TileClassId,
    ) -> Self {
        Self {
            session,
            backend,
            tile_class,
        }
    }

    pub fn try_new(
        session: &'sm mut Session<'a, XactBackend<'a>>,
        backend: &'a XactBackend<'a>,
        tile_class: TileClassId,
    ) -> Option<Self> {
        if backend.edev.tile_index[tile_class].is_empty() {
            return None;
        }
        Some(Self {
            session,
            backend,
            tile_class,
        })
    }

    pub fn bel<'c>(&'c mut self, bel: BelSlotId) -> FuzzCtxBel<'c, 'a> {
        FuzzCtxBel {
            session: &mut *self.session,
            backend: self.backend,
            tile_class: self.tile_class,
            bel,
        }
    }

    pub fn build<'nsm>(&'nsm mut self) -> FuzzBuilder<'nsm, 'a> {
        FuzzBuilder {
            session: &mut *self.session,
            tile_class: self.tile_class,
            props: vec![],
        }
    }
}

pub struct FuzzBuilder<'sm, 'b> {
    pub session: &'sm mut Session<'b, XactBackend<'b>>,
    pub tile_class: TileClassId,
    pub props: Vec<Box<DynProp<'b>>>,
}

impl<'sm, 'b> FuzzBuilder<'sm, 'b> {
    pub fn prop(mut self, prop: impl FuzzerProp<'b, XactBackend<'b>> + 'static) -> Self {
        self.props.push(Box::new(prop));
        self
    }

    pub fn null_bits(self) -> Self {
        self.prop(NullBits)
    }

    pub fn test_raw(self, diff_key: DiffKey) -> FuzzBuilderTestManual<'sm, 'b> {
        FuzzBuilderTestManual {
            session: self.session,
            tile_class: self.tile_class,
            props: self.props,
            diff_key,
        }
    }
}

pub struct FuzzBuilderTestManual<'sm, 'b> {
    pub session: &'sm mut Session<'b, XactBackend<'b>>,
    pub tile_class: TileClassId,
    pub props: Vec<Box<DynProp<'b>>>,
    pub diff_key: DiffKey,
}

impl<'b> FuzzBuilderTestManual<'_, 'b> {
    pub fn prop(mut self, prop: impl FuzzerProp<'b, XactBackend<'b>> + 'static) -> Self {
        self.props.push(Box::new(prop));
        self
    }

    pub fn commit(self) {
        let fgen = FpgaFuzzerGen {
            tile_class: Some(self.tile_class),
            key: self.diff_key,
            props: self.props,
        };
        self.session.add_fuzzer(Box::new(fgen));
    }
}

pub struct FuzzCtxBel<'sm, 'a> {
    pub session: &'sm mut Session<'a, XactBackend<'a>>,
    pub backend: &'a XactBackend<'a>,
    pub tile_class: TileClassId,
    pub bel: BelSlotId,
}

impl<'a> FuzzCtxBel<'_, 'a> {
    pub fn build<'sm>(&'sm mut self) -> FuzzBuilderBel<'sm, 'a> {
        FuzzBuilderBel {
            session: &mut *self.session,
            backend: self.backend,
            tile_class: self.tile_class,
            bel: self.bel,
            props: vec![],
        }
    }

    pub fn mode<'sm>(&'sm mut self, mode: impl Into<String>) -> FuzzBuilderBel<'sm, 'a> {
        self.build().mode(mode)
    }

    pub fn test_attr_global_as(&mut self, opt: &str, attr: BelAttributeId) {
        self.build().test_attr_global_as(opt, attr);
    }

    pub fn test_attr_global(&mut self, attr: BelAttributeId) {
        self.build().test_attr_global(attr);
    }

    pub fn test_attr_global_enum_bool_as(
        &mut self,
        opt: &str,
        attr: BelAttributeId,
        val0: &str,
        val1: &str,
    ) {
        self.build()
            .test_attr_global_enum_bool_as(opt, attr, val0, val1);
    }
}

pub struct FuzzBuilderBel<'sm, 'b> {
    pub session: &'sm mut Session<'b, XactBackend<'b>>,
    pub backend: &'b XactBackend<'b>,
    pub tile_class: TileClassId,
    pub bel: BelSlotId,
    pub props: Vec<Box<DynProp<'b>>>,
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
            props: self.props.clone(),
        }
    }

    pub fn prop(mut self, prop: impl FuzzerProp<'b, XactBackend<'b>> + 'static) -> Self {
        self.props.push(Box::new(prop));
        self
    }

    pub fn raw(self, key: Key<'static>, val: impl Into<Value<'static>>) -> Self {
        self.prop(BaseRaw::new(key, val.into()))
    }

    pub fn null_bits(self) -> Self {
        self.prop(NullBits)
    }

    pub fn extra_fixed_bel_attr_val(
        self,
        tcrd: TileCoord,
        bslot: BelSlotId,
        attr: BelAttributeId,
        val: EnumValueId,
    ) -> Self {
        let tcid = self.backend.edev[tcrd].class;
        self.prop(ExtraTile::new(
            tcrd,
            DiffKey::BelAttrValue(tcid, bslot, attr, val),
        ))
    }

    pub fn extra_fixed_bel_attr_bits(
        self,
        tcrd: TileCoord,
        bslot: BelSlotId,
        attr: BelAttributeId,
    ) -> Self {
        let tcid = self.backend.edev[tcrd].class;
        self.prop(ExtraTile::new(
            tcrd,
            DiffKey::BelAttrBit(tcid, bslot, attr, 0),
        ))
    }

    pub fn mode(self, mode: impl Into<String>) -> Self {
        let prop = BaseBelMode::new(self.bel, mode.into());
        self.prop(prop)
    }

    pub fn mutex(self, attr: impl Into<String>, val: impl Into<String>) -> Self {
        let prop = BaseBelMutex::new(self.bel, attr.into(), val.into());
        self.prop(prop)
    }

    pub fn cfg(self, attr: impl Into<String>, val: impl Into<String>) -> Self {
        let prop = BaseBelConfig::new(self.bel, attr.into(), val.into());
        self.prop(prop)
    }

    pub fn bonded_io(self) -> Self {
        let prop = BondedIo::new(self.bel);
        self.prop(prop)
    }

    pub fn global(self, attr: impl Into<String>, val: impl Into<String>) -> Self {
        self.prop(BaseRaw::new(Key::GlobalOpt(attr.into()), val.into().into()))
    }

    pub fn bel_out(self, bel: &'static str, pin: &str) -> Self {
        self.prop(BaseRaw::new(Key::BlockPin(bel, pin.into()), true.into()))
    }

    pub fn input_mutex_exclusive(self, pin: BelInputId) -> Self {
        let prop = InputMutexExclusive::new(self.bel, pin);
        self.prop(prop)
    }

    pub fn bidir_mutex_exclusive(self, pin: BelBidirId) -> Self {
        let prop = BidirMutexExclusive::new(self.bel, pin);
        self.prop(prop)
    }

    pub fn test_other_bel_attr_as(
        mut self,
        bslot: BelSlotId,
        key: impl AsRef<str>,
        attr: BelAttributeId,
    ) {
        let key = key.as_ref();
        let BelKind::Class(bcid) = self.backend.edev.db.bel_slots[bslot].kind else {
            unreachable!()
        };
        let BelAttributeType::Enum(ecid) = self.backend.edev.db[bcid].attributes[attr].typ else {
            unreachable!()
        };
        let ecls = &self.backend.edev.db[ecid];
        for (vid, val) in &ecls.values {
            self.clone()
                .mutex(key, val)
                .test_other_bel_attr_val(bslot, attr, vid)
                .cfg(key, val)
                .commit();
        }
    }

    pub fn test_bel_attr_as(self, key: impl AsRef<str>, attr: BelAttributeId) {
        let bslot = self.bel;
        self.test_other_bel_attr_as(bslot, key, attr);
    }

    pub fn test_bel_attr_default_as(
        mut self,
        key: impl AsRef<str>,
        attr: BelAttributeId,
        default: EnumValueId,
    ) {
        let key = key.as_ref();
        let BelKind::Class(bcid) = self.backend.edev.db.bel_slots[self.bel].kind else {
            unreachable!()
        };
        let BelAttributeType::Enum(ecid) = self.backend.edev.db[bcid].attributes[attr].typ else {
            unreachable!()
        };
        let ecls = &self.backend.edev.db[ecid];
        for (vid, val) in &ecls.values {
            if vid == default {
                continue;
            }
            self.clone()
                .mutex(key, val)
                .test_bel_attr_val(attr, vid)
                .cfg(key, val)
                .commit();
        }
    }

    pub fn test_bel_attr_enum_bool_as(
        mut self,
        rattr: &str,
        attr: BelAttributeId,
        val0: &str,
        val1: &str,
    ) {
        self.clone()
            .mutex(rattr, val0)
            .test_bel_attr_enum_bool(attr, false)
            .cfg(rattr, val0)
            .commit();
        self.mutex(rattr, val1)
            .test_bel_attr_enum_bool(attr, true)
            .cfg(rattr, val1)
            .commit();
    }

    pub fn test_attr_global_as(mut self, opt: &str, attr: BelAttributeId) {
        let BelKind::Class(bcid) = self.backend.edev.db.bel_slots[self.bel].kind else {
            unreachable!()
        };
        let BelAttributeType::Enum(ecid) = self.backend.edev.db[bcid].attributes[attr].typ else {
            unreachable!()
        };
        let ecls = &self.backend.edev.db[ecid];
        for (vid, val) in &ecls.values {
            self.clone()
                .test_bel_attr_val(attr, vid)
                .global(opt, val)
                .commit();
        }
    }

    pub fn test_attr_global_default_as(
        mut self,
        opt: impl AsRef<str>,
        attr: BelAttributeId,
        default: EnumValueId,
    ) {
        let opt = opt.as_ref();
        let BelKind::Class(bcid) = self.backend.edev.db.bel_slots[self.bel].kind else {
            unreachable!()
        };
        let BelAttributeType::Enum(ecid) = self.backend.edev.db[bcid].attributes[attr].typ else {
            unreachable!()
        };
        let ecls = &self.backend.edev.db[ecid];
        for (vid, val) in &ecls.values {
            if vid == default {
                continue;
            }
            self.clone()
                .test_bel_attr_val(attr, vid)
                .global(opt, val)
                .commit();
        }
    }

    pub fn test_attr_global(self, attr: BelAttributeId) {
        let BelKind::Class(bcid) = self.backend.edev.db.bel_slots[self.bel].kind else {
            unreachable!()
        };
        let opt = self.backend.edev.db[bcid].attributes.key(attr);
        self.test_attr_global_as(opt, attr);
    }

    pub fn test_attr_global_enum_bool_as(
        mut self,
        opt: &str,
        attr: BelAttributeId,
        val0: &str,
        val1: &str,
    ) {
        self.clone()
            .test_bel_attr_enum_bool(attr, false)
            .global(opt, val0)
            .commit();
        self.test_bel_attr_enum_bool(attr, true)
            .global(opt, val1)
            .commit();
    }

    pub fn test_bel_attr_equate(
        self,
        attr: BelAttributeId,
        aname: impl AsRef<str>,
        inps: &'static [&'static str],
    ) {
        let aname = aname.as_ref();
        let prop = FuzzEquate::new(self.bel, aname.into(), inps);
        self.test_bel_attr_bits(attr).prop(prop).commit();
    }

    pub fn test_other_bel_attr_equate(
        self,
        bslot: BelSlotId,
        attr: BelAttributeId,
        aname: impl AsRef<str>,
        inps: &'static [&'static str],
    ) {
        let aname = aname.as_ref();
        let prop = FuzzEquate::new(self.bel, aname.into(), inps);
        self.test_other_bel_attr_bits(bslot, attr)
            .prop(prop)
            .commit();
    }

    pub fn test_raw(self, diff_key: DiffKey) -> FuzzBuilderBelTestManual<'sm, 'b> {
        FuzzBuilderBelTestManual {
            session: self.session,
            tile_class: self.tile_class,
            bel: self.bel,
            props: self.props,
            diff_key,
        }
    }

    pub fn test_bel_attr_val(
        self,
        attr: BelAttributeId,
        val: EnumValueId,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let diff_key = DiffKey::BelAttrValue(self.tile_class, self.bel, attr, val);
        self.test_raw(diff_key)
    }

    pub fn test_other_bel_attr_val(
        self,
        bslot: BelSlotId,
        attr: BelAttributeId,
        val: EnumValueId,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let diff_key = DiffKey::BelAttrValue(self.tile_class, bslot, attr, val);
        self.test_raw(diff_key)
    }

    pub fn test_bel_attr_enum_bool(
        self,
        attr: BelAttributeId,
        val: bool,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let diff_key = DiffKey::BelAttrEnumBool(self.tile_class, self.bel, attr, val);
        self.test_raw(diff_key)
    }

    pub fn test_other_bel_attr_enum_bool(
        self,
        bslot: BelSlotId,
        attr: BelAttributeId,
        val: bool,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let diff_key = DiffKey::BelAttrEnumBool(self.tile_class, bslot, attr, val);
        self.test_raw(diff_key)
    }

    pub fn test_bel_attr_bits(self, attr: BelAttributeId) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let diff_key = DiffKey::BelAttrBit(self.tile_class, self.bel, attr, 0);
        self.test_raw(diff_key)
    }

    pub fn test_other_bel_attr_bits(
        self,
        bslot: BelSlotId,
        attr: BelAttributeId,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let diff_key = DiffKey::BelAttrBit(self.tile_class, bslot, attr, 0);
        self.test_raw(diff_key)
    }

    pub fn test_bel_attr_special(
        self,
        attr: BelAttributeId,
        special: SpecialId,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let diff_key = DiffKey::BelAttrSpecial(self.tile_class, self.bel, attr, special);
        self.test_raw(diff_key)
    }

    pub fn test_bel_attr_special_bit(
        self,
        attr: BelAttributeId,
        special: SpecialId,
        bit: usize,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let diff_key = DiffKey::BelAttrSpecialBit(self.tile_class, self.bel, attr, special, bit);
        self.test_raw(diff_key)
    }

    pub fn test_bel_input_inv(
        self,
        inp: BelInputId,
        val: bool,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let diff_key = DiffKey::BelInputInv(self.tile_class, self.bel, inp, val);
        self.test_raw(diff_key)
    }

    pub fn test_other_bel_input_inv(
        self,
        bslot: BelSlotId,
        inp: BelInputId,
        val: bool,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let diff_key = DiffKey::BelInputInv(self.tile_class, bslot, inp, val);
        self.test_raw(diff_key)
    }

    pub fn test_bel_special(self, spec: SpecialId) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let diff_key = DiffKey::BelSpecial(self.tile_class, self.bel, spec);
        self.test_raw(diff_key)
    }

    pub fn test_other_bel_special(
        self,
        bslot: BelSlotId,
        spec: SpecialId,
    ) -> FuzzBuilderBelTestManual<'sm, 'b> {
        let diff_key = DiffKey::BelSpecial(self.tile_class, bslot, spec);
        self.test_raw(diff_key)
    }
}

pub struct FuzzBuilderBelTestManual<'sm, 'b> {
    pub session: &'sm mut Session<'b, XactBackend<'b>>,
    pub tile_class: TileClassId,
    pub bel: BelSlotId,
    pub props: Vec<Box<DynProp<'b>>>,
    pub diff_key: DiffKey,
}

impl<'b> FuzzBuilderBelTestManual<'_, 'b> {
    pub fn prop(mut self, prop: impl FuzzerProp<'b, XactBackend<'b>> + 'static) -> Self {
        self.props.push(Box::new(prop));
        self
    }

    pub fn mode(self, mode: impl Into<String>) -> Self {
        let mode = mode.into();
        let prop = FuzzBelMode::new(self.bel, mode);
        self.prop(prop)
    }

    pub fn cfg(self, attr: impl Into<String>, val: impl Into<String>) -> Self {
        let prop = FuzzBelConfig::new(self.bel, attr.into(), val.into());
        self.prop(prop)
    }

    pub fn cfg_excl(self, attr: impl Into<String>, val: impl Into<String>) -> Self {
        let attr = attr.into();
        let val = val.into();
        let prop0 = FuzzBelConfig::new(self.bel, attr.clone(), val.clone());
        let prop1 = BaseBelMutex::new(self.bel, attr, val);
        self.prop(prop0).prop(prop1)
    }

    pub fn cfg_diff(
        self,
        attr: impl Into<String>,
        val0: impl Into<String>,
        val1: impl Into<String>,
    ) -> Self {
        let prop = FuzzBelConfigDiff::new(self.bel, attr.into(), val0.into(), val1.into());
        self.prop(prop)
    }

    pub fn equate_fixed(
        self,
        attr: impl Into<String>,
        inps: &'static [&'static str],
        bits: BitVec,
    ) -> Self {
        let prop = FuzzEquateFixed::new(self.bel, attr.into(), inps, bits);
        self.prop(prop)
    }

    pub fn pip_pin(self, key: impl Into<String>, pin: impl Into<String>) -> Self {
        let prop = FuzzBelPipPin::new(self.bel, key.into(), pin.into());
        self.prop(prop)
    }

    pub fn raw_diff(
        self,
        key: Key<'static>,
        val0: impl Into<Value<'static>>,
        val1: impl Into<Value<'static>>,
    ) -> Self {
        self.prop(FuzzRaw::new(key, val0.into(), val1.into()))
    }

    pub fn global(self, opt: &str, val: &str) -> Self {
        self.raw_diff(Key::GlobalOpt(opt.into()), None, val)
    }

    pub fn global_diff(self, opt: &str, val0: &str, val1: &str) -> Self {
        self.raw_diff(Key::GlobalOpt(opt.into()), val0, val1)
    }

    pub fn bel_out(self, bel: &'static str, pin: &str) -> Self {
        self.prop(FuzzRaw::new(
            Key::BlockPin(bel, pin.into()),
            false.into(),
            true.into(),
        ))
    }

    pub fn commit(self) {
        let fgen = FpgaFuzzerGen {
            tile_class: Some(self.tile_class),
            key: self.diff_key,
            props: self.props,
        };
        self.session.add_fuzzer(Box::new(fgen));
    }
}
