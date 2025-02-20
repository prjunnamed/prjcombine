use bitvec::vec::BitVec;
use prjcombine_collector::FeatureId;
use prjcombine_hammer::Session;
use prjcombine_interconnect::{
    db::{BelId, NodeKindId},
    grid::NodeLoc,
};

use crate::{
    backend::{Key, Value, XactBackend},
    fgen::{
        BaseBelConfig, BaseBelMode, BaseBelMutex, BaseRaw, BondedIo, ExtraTile, FuzzBelConfig,
        FuzzBelConfigDiff, FuzzBelMode, FuzzBelPipBufg, FuzzBelPipPin, FuzzEquate, FuzzEquateFixed,
        FuzzRaw, PinMutexExclusive, Prop, XactFuzzerGen,
    },
};

pub struct FuzzCtx<'sm, 'a> {
    pub session: &'sm mut Session<'a, XactBackend<'a>>,
    pub backend: &'a XactBackend<'a>,
    pub node_kind: NodeKindId,
}

impl<'sm, 'a> FuzzCtx<'sm, 'a> {
    pub fn new(
        session: &'sm mut Session<'a, XactBackend<'a>>,
        backend: &'a XactBackend<'a>,
        tile: impl Into<String>,
    ) -> Self {
        let tile = tile.into();
        let node_kind = backend.egrid.db.get_node(&tile);
        Self {
            session,
            backend,
            node_kind,
        }
    }

    pub fn try_new(
        session: &'sm mut Session<'a, XactBackend<'a>>,
        backend: &'a XactBackend<'a>,
        tile: impl Into<String>,
    ) -> Option<Self> {
        let tile = tile.into();
        let node_kind = backend.egrid.db.get_node(&tile);
        if backend.egrid.node_index[node_kind].is_empty() {
            return None;
        }
        Some(Self {
            session,
            backend,
            node_kind,
        })
    }

    pub fn bel<'c>(&'c mut self, bel: impl Into<String>) -> FuzzCtxBel<'c, 'a> {
        let bel_name = bel.into();
        let bel = self.backend.egrid.db.nodes[self.node_kind]
            .bels
            .get(&bel_name)
            .unwrap()
            .0;
        FuzzCtxBel {
            session: &mut *self.session,
            backend: self.backend,
            node_kind: self.node_kind,
            bel,
        }
    }

    pub fn build<'nsm>(&'nsm mut self) -> FuzzBuilder<'nsm, 'a> {
        FuzzBuilder {
            session: &mut *self.session,
            backend: self.backend,
            node_kind: self.node_kind,
            props: vec![],
        }
    }

    pub fn test_global(&mut self, bel: &'static str, opt: &str, vals: &[&str]) {
        self.build().test_global(bel, opt, vals);
    }

    pub fn test_cfg4000(&mut self, bel: &'static str, opt: &str, vals: &[&str]) {
        self.build().test_cfg4000(bel, opt, vals);
    }

    pub fn test_cfg5200(&mut self, bel: &'static str, opt: &str, vals: &[&str]) {
        self.build().test_cfg5200(bel, opt, vals);
    }
}

pub struct FuzzBuilder<'sm, 'a> {
    pub session: &'sm mut Session<'a, XactBackend<'a>>,
    pub backend: &'a XactBackend<'a>,
    pub node_kind: NodeKindId,
    pub props: Vec<Box<dyn Prop>>,
}

impl<'sm, 'a> FuzzBuilder<'sm, 'a> {
    pub fn prop(mut self, prop: impl Prop + 'static) -> Self {
        self.props.push(Box::new(prop));
        self
    }

    pub fn raw(self, key: Key<'static>, val: impl Into<Value<'static>>) -> Self {
        self.prop(BaseRaw::new(key, val.into()))
    }

    pub fn global(self, opt: &str, val: &str) -> Self {
        self.raw(Key::GlobalOpt(opt.into()), val)
    }

    pub fn bel_out(self, bel: &'static str, pin: &str) -> Self {
        self.prop(BaseRaw::new(Key::BlockPin(bel, pin.into()), true.into()))
    }

    pub fn test_global(self, bel: &'static str, opt: &str, vals: &[&str]) {
        for &val in vals {
            let feature = FeatureId {
                tile: self.backend.egrid.db.nodes.key(self.node_kind).clone(),
                bel: bel.into(),
                attr: opt.into(),
                val: val.into(),
            };
            let mut props = Vec::from_iter(self.props.iter().map(|x| x.dyn_clone()));
            props.push(Box::new(FuzzRaw::new(
                Key::GlobalOpt(opt.into()),
                None.into(),
                val.into(),
            )));
            let fgen = XactFuzzerGen {
                node: self.node_kind,
                feature,
                props,
            };
            self.session.add_fuzzer(Box::new(fgen));
        }
    }

    pub fn test_cfg4000(self, bel: &'static str, opt: &str, vals: &[&str]) {
        for &val in vals {
            let feature = FeatureId {
                tile: self.backend.egrid.db.nodes.key(self.node_kind).clone(),
                bel: bel.into(),
                attr: opt.into(),
                val: val.into(),
            };
            let mut props = Vec::from_iter(self.props.iter().map(|x| x.dyn_clone()));
            props.push(Box::new(BaseRaw::new(
                Key::GlobalMutex(opt.into()),
                val.into(),
            )));
            props.push(Box::new(FuzzRaw::new(
                Key::BlockConfig("_cfg4000_", opt.into(), val.into()),
                false.into(),
                true.into(),
            )));
            let fgen = XactFuzzerGen {
                node: self.node_kind,
                feature,
                props,
            };
            self.session.add_fuzzer(Box::new(fgen));
        }
    }

    pub fn test_cfg5200(self, bel: &'static str, opt: &str, vals: &[&str]) {
        for &val in vals {
            let feature = FeatureId {
                tile: self.backend.egrid.db.nodes.key(self.node_kind).clone(),
                bel: bel.into(),
                attr: opt.into(),
                val: val.into(),
            };
            let mut props = Vec::from_iter(self.props.iter().map(|x| x.dyn_clone()));
            props.push(Box::new(BaseRaw::new(
                Key::GlobalMutex(opt.into()),
                val.into(),
            )));
            props.push(Box::new(FuzzRaw::new(
                Key::BlockConfig("_cfg5200_", opt.into(), val.into()),
                false.into(),
                true.into(),
            )));
            let fgen = XactFuzzerGen {
                node: self.node_kind,
                feature,
                props,
            };
            self.session.add_fuzzer(Box::new(fgen));
        }
    }

    pub fn test_manual(
        self,
        bel: &'static str,
        attr: impl AsRef<str>,
        val: impl AsRef<str>,
    ) -> FuzzBuilderTestManual<'sm, 'a> {
        let attr = attr.as_ref();
        let val = val.as_ref();
        let feature = FeatureId {
            tile: self.backend.egrid.db.nodes.key(self.node_kind).clone(),
            bel: bel.into(),
            attr: attr.into(),
            val: val.into(),
        };
        FuzzBuilderTestManual {
            session: self.session,
            node_kind: self.node_kind,
            props: self.props,
            feature,
        }
    }
}

pub struct FuzzBuilderTestManual<'sm, 'a> {
    pub session: &'sm mut Session<'a, XactBackend<'a>>,
    pub node_kind: NodeKindId,
    pub props: Vec<Box<dyn Prop>>,
    pub feature: FeatureId,
}

impl FuzzBuilderTestManual<'_, '_> {
    pub fn prop(mut self, prop: impl Prop + 'static) -> Self {
        self.props.push(Box::new(prop));
        self
    }

    pub fn raw_diff(
        self,
        key: Key<'static>,
        val0: impl Into<Value<'static>>,
        val1: impl Into<Value<'static>>,
    ) -> Self {
        self.prop(FuzzRaw::new(key, val0.into(), val1.into()))
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
        let fgen = XactFuzzerGen {
            node: self.node_kind,
            feature: self.feature,
            props: self.props,
        };
        self.session.add_fuzzer(Box::new(fgen));
    }
}

pub struct FuzzCtxBel<'sm, 'a> {
    pub session: &'sm mut Session<'a, XactBackend<'a>>,
    pub backend: &'a XactBackend<'a>,
    pub node_kind: NodeKindId,
    pub bel: BelId,
}

impl<'a> FuzzCtxBel<'_, 'a> {
    pub fn build<'sm>(&'sm mut self) -> FuzzBuilderBel<'sm, 'a> {
        FuzzBuilderBel {
            session: &mut *self.session,
            backend: self.backend,
            node_kind: self.node_kind,
            bel: self.bel,
            props: vec![],
        }
    }

    pub fn mode<'sm>(&'sm mut self, mode: impl Into<String>) -> FuzzBuilderBel<'sm, 'a> {
        self.build().mode(mode)
    }

    pub fn test_mode(&mut self, mode: impl Into<String>) {
        self.build().test_mode(mode)
    }
}

pub struct FuzzBuilderBel<'sm, 'a> {
    pub session: &'sm mut Session<'a, XactBackend<'a>>,
    pub backend: &'a XactBackend<'a>,
    pub node_kind: NodeKindId,
    pub bel: BelId,
    pub props: Vec<Box<dyn Prop>>,
}

impl<'sm, 'a> FuzzBuilderBel<'sm, 'a> {
    pub fn prop(mut self, prop: impl Prop + 'static) -> Self {
        self.props.push(Box::new(prop));
        self
    }

    pub fn extra_tile(
        self,
        nloc: NodeLoc,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> Self {
        self.prop(ExtraTile::new(nloc, bel.into(), attr.into(), val.into()))
    }

    pub fn mode(self, mode: impl Into<String>) -> Self {
        let prop = BaseBelMode::new(self.bel, mode.into());
        self.prop(prop)
    }

    pub fn test_mode(self, mode: impl Into<String>) {
        let mode = mode.into();
        let prop = FuzzBelMode::new(self.bel, mode.clone());
        self.test_manual("BASE", mode).prop(prop).commit();
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

    pub fn pin_mutex_exclusive(self, pin: impl Into<String>) -> Self {
        let prop = PinMutexExclusive::new(self.bel, pin.into());
        self.prop(prop)
    }

    pub fn test_enum(self, attr: impl AsRef<str>, vals: &[impl AsRef<str>]) {
        let attr = attr.as_ref();
        for val in vals {
            let val = val.as_ref();
            let feature = FeatureId {
                tile: self.backend.egrid.db.nodes.key(self.node_kind).clone(),
                bel: self.backend.egrid.db.nodes[self.node_kind]
                    .bels
                    .key(self.bel)
                    .clone(),
                attr: attr.into(),
                val: val.into(),
            };
            let mut props = Vec::from_iter(self.props.iter().map(|x| x.dyn_clone()));
            props.push(Box::new(FuzzBelConfig::new(
                self.bel,
                attr.into(),
                val.into(),
            )));
            props.push(Box::new(BaseBelMutex::new(
                self.bel,
                attr.into(),
                val.into(),
            )));
            let fgen = XactFuzzerGen {
                node: self.node_kind,
                feature,
                props,
            };
            self.session.add_fuzzer(Box::new(fgen));
        }
    }

    pub fn test_cfg(self, attr: impl AsRef<str>, val: impl AsRef<str>) {
        let attr = attr.as_ref();
        let val = val.as_ref();
        let prop = FuzzBelConfig::new(self.bel, attr.into(), val.into());
        self.test_manual(attr, val).prop(prop).commit();
    }

    pub fn test_equate(self, attr: impl AsRef<str>, inps: &'static [&'static str]) {
        let attr = attr.as_ref();
        let prop = FuzzEquate::new(self.bel, attr.into(), inps);
        self.test_manual(attr, "").prop(prop).commit();
    }

    pub fn test_equate_fixed(
        self,
        attr: impl AsRef<str>,
        val: impl AsRef<str>,
        inps: &'static [&'static str],
        bits: BitVec,
    ) {
        let attr = attr.as_ref();
        let val = val.as_ref();
        let prop = FuzzEquateFixed::new(self.bel, attr.into(), inps, bits);
        self.test_manual(attr, val).prop(prop).commit();
    }

    pub fn test_manual(
        self,
        attr: impl AsRef<str>,
        val: impl AsRef<str>,
    ) -> FuzzBuilderBelTestManual<'sm, 'a> {
        let attr = attr.as_ref();
        let val = val.as_ref();
        let feature = FeatureId {
            tile: self.backend.egrid.db.nodes.key(self.node_kind).clone(),
            bel: self.backend.egrid.db.nodes[self.node_kind]
                .bels
                .key(self.bel)
                .clone(),
            attr: attr.into(),
            val: val.into(),
        };
        FuzzBuilderBelTestManual {
            session: self.session,
            node_kind: self.node_kind,
            bel: self.bel,
            props: self.props,
            feature,
        }
    }
}

pub struct FuzzBuilderBelTestManual<'sm, 'a> {
    pub session: &'sm mut Session<'a, XactBackend<'a>>,
    pub node_kind: NodeKindId,
    pub bel: BelId,
    pub props: Vec<Box<dyn Prop>>,
    pub feature: FeatureId,
}

impl FuzzBuilderBelTestManual<'_, '_> {
    pub fn prop(mut self, prop: impl Prop + 'static) -> Self {
        self.props.push(Box::new(prop));
        self
    }

    pub fn cfg(self, attr: impl Into<String>, val: impl Into<String>) -> Self {
        let prop = FuzzBelConfig::new(self.bel, attr.into(), val.into());
        self.prop(prop)
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

    pub fn pip_bufg(self, key: impl Into<String>, buf: &'static str) -> Self {
        let prop = FuzzBelPipBufg::new(self.bel, key.into(), buf);
        self.prop(prop)
    }

    pub fn pip_pin(self, key: impl Into<String>, pin: impl Into<String>) -> Self {
        let prop = FuzzBelPipPin::new(self.bel, key.into(), pin.into());
        self.prop(prop)
    }

    pub fn commit(self) {
        let fgen = XactFuzzerGen {
            node: self.node_kind,
            feature: self.feature,
            props: self.props,
        };
        self.session.add_fuzzer(Box::new(fgen));
    }
}
