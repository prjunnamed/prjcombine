use std::collections::{BTreeSet, HashMap};

use bitvec::vec::BitVec;
use prjcombine_re_collector::{FeatureId, State};
use prjcombine_re_hammer::{BatchValue, Fuzzer, FuzzerGen};
use prjcombine_interconnect::{
    db::{BelId, NodeKindId},
    grid::NodeLoc,
};
use prjcombine_xilinx_bitstream::BitTile;
use prjcombine_xc2000::grid::GridKind;
use rand::prelude::*;

use crate::backend::{FuzzerFeature, FuzzerInfo, Key, MultiValue, Value, XactBackend};

pub trait Prop: std::fmt::Debug {
    fn dyn_clone(&self) -> Box<dyn Prop>;

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)>;
}

#[derive(Clone, Debug)]
pub struct BaseRaw {
    pub key: Key<'static>,
    pub val: Value<'static>,
}

impl BaseRaw {
    pub fn new(key: Key<'static>, val: Value<'static>) -> Self {
        Self { key, val }
    }
}

impl Prop for BaseRaw {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        _backend: &XactBackend<'a>,
        _nloc: NodeLoc,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        Some((fuzzer.base(self.key.clone(), self.val.clone()), false))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzRaw {
    pub key: Key<'static>,
    pub val0: Value<'static>,
    pub val1: Value<'static>,
}

impl FuzzRaw {
    pub fn new(key: Key<'static>, val0: Value<'static>, val1: Value<'static>) -> Self {
        Self { key, val0, val1 }
    }
}

impl Prop for FuzzRaw {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        _backend: &XactBackend<'a>,
        _nloc: NodeLoc,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        Some((
            fuzzer.fuzz(self.key.clone(), self.val0.clone(), self.val1.clone()),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct BaseBelMode {
    pub bel: BelId,
    pub val: String,
}

impl BaseBelMode {
    pub fn new(bel: BelId, val: String) -> Self {
        Self { bel, val }
    }
}

impl Prop for BaseBelMode {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let nnode = &backend.ngrid.nodes[&nloc];
        Some((
            fuzzer.base(Key::BlockBase(&nnode.bels[self.bel][0]), self.val.clone()),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzBelMode {
    pub bel: BelId,
    pub val: String,
}

impl FuzzBelMode {
    pub fn new(bel: BelId, val: String) -> Self {
        Self { bel, val }
    }
}

impl Prop for FuzzBelMode {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let nnode = &backend.ngrid.nodes[&nloc];
        Some((
            fuzzer.fuzz(
                Key::BlockBase(&nnode.bels[self.bel][0]),
                None,
                self.val.clone(),
            ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct BaseBelMutex {
    pub bel: BelId,
    pub attr: String,
    pub val: String,
}

impl BaseBelMutex {
    pub fn new(bel: BelId, attr: String, val: String) -> Self {
        Self { bel, attr, val }
    }
}

impl Prop for BaseBelMutex {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        _backend: &XactBackend<'a>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        Some((
            fuzzer.base(
                Key::BelMutex(nloc, self.bel, self.attr.clone()),
                self.val.clone(),
            ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct BaseBelConfig {
    pub bel: BelId,
    pub attr: String,
    pub val: String,
}

impl BaseBelConfig {
    pub fn new(bel: BelId, attr: String, val: String) -> Self {
        Self { bel, attr, val }
    }
}

impl Prop for BaseBelConfig {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let nnode = &backend.ngrid.nodes[&nloc];
        Some((
            fuzzer.base(
                Key::BlockConfig(
                    &nnode.bels[self.bel][0],
                    self.attr.clone(),
                    self.val.clone(),
                ),
                true,
            ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct BaseBelNoConfig {
    pub bel: BelId,
    pub attr: String,
    pub val: String,
}

impl BaseBelNoConfig {
    pub fn new(bel: BelId, attr: String, val: String) -> Self {
        Self { bel, attr, val }
    }
}

impl Prop for BaseBelNoConfig {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let nnode = &backend.ngrid.nodes[&nloc];
        Some((
            fuzzer.base(
                Key::BlockConfig(
                    &nnode.bels[self.bel][0],
                    self.attr.clone(),
                    self.val.clone(),
                ),
                false,
            ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzBelConfig {
    pub bel: BelId,
    pub attr: String,
    pub val: String,
}

impl FuzzBelConfig {
    pub fn new(bel: BelId, attr: String, val: String) -> Self {
        Self { bel, attr, val }
    }
}

impl Prop for FuzzBelConfig {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let nnode = &backend.ngrid.nodes[&nloc];
        Some((
            fuzzer.fuzz(
                Key::BlockConfig(
                    &nnode.bels[self.bel][0],
                    self.attr.clone(),
                    self.val.clone(),
                ),
                false,
                true,
            ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzBelConfigDiff {
    pub bel: BelId,
    pub attr: String,
    pub val0: String,
    pub val1: String,
}

impl FuzzBelConfigDiff {
    pub fn new(bel: BelId, attr: String, val0: String, val1: String) -> Self {
        Self {
            bel,
            attr,
            val0,
            val1,
        }
    }
}

impl Prop for FuzzBelConfigDiff {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let nnode = &backend.ngrid.nodes[&nloc];
        Some((
            fuzzer
                .fuzz(
                    Key::BlockConfig(
                        &nnode.bels[self.bel][0],
                        self.attr.clone(),
                        self.val0.clone(),
                    ),
                    true,
                    false,
                )
                .fuzz(
                    Key::BlockConfig(
                        &nnode.bels[self.bel][0],
                        self.attr.clone(),
                        self.val1.clone(),
                    ),
                    false,
                    true,
                ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzEquate {
    pub bel: BelId,
    pub attr: String,
    pub inps: &'static [&'static str],
}

impl FuzzEquate {
    pub fn new(bel: BelId, attr: String, inps: &'static [&'static str]) -> Self {
        Self { bel, attr, inps }
    }
}

impl Prop for FuzzEquate {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let nnode = &backend.ngrid.nodes[&nloc];
        let bname = &nnode.bels[self.bel][0];
        for &inp in self.inps {
            fuzzer = fuzzer.base(
                Key::BlockConfig(bname, self.attr.clone(), inp.to_string()),
                true,
            );
        }
        Some((
            fuzzer
                .fuzz_multi(
                    Key::BlockEquate(bname, self.attr.clone()),
                    MultiValue::Lut(self.inps),
                )
                .bits(1 << self.inps.len()),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzEquateFixed {
    pub bel: BelId,
    pub attr: String,
    pub inps: &'static [&'static str],
    pub bits: BitVec,
}

impl FuzzEquateFixed {
    pub fn new(bel: BelId, attr: String, inps: &'static [&'static str], bits: BitVec) -> Self {
        Self {
            bel,
            attr,
            inps,
            bits,
        }
    }
}

impl Prop for FuzzEquateFixed {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let nnode = &backend.ngrid.nodes[&nloc];
        let bname = &nnode.bels[self.bel][0];
        for &inp in self.inps {
            fuzzer = fuzzer.fuzz(
                Key::BlockConfig(bname, self.attr.clone(), inp.to_string()),
                false,
                true,
            );
        }
        Some((
            fuzzer.fuzz(
                Key::BlockEquate(bname, self.attr.clone()),
                None,
                Value::Lut(self.inps, self.bits.clone()),
            ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzBelPipBufg {
    pub bel: BelId,
    pub key: String,
    pub buf: &'static str,
}

impl FuzzBelPipBufg {
    pub fn new(bel: BelId, key: String, buf: &'static str) -> Self {
        Self { bel, key, buf }
    }
}

impl Prop for FuzzBelPipBufg {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let crd = backend.ngrid.bel_pip(nloc, self.bel, &self.key);
        Some((
            fuzzer.fuzz(Key::Pip(crd), None, Value::FromPin(self.buf, "O".into())),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct PinMutexExclusive {
    pub bel: BelId,
    pub pin: String,
}

impl PinMutexExclusive {
    pub fn new(bel: BelId, pin: String) -> Self {
        Self { bel, pin }
    }
}

impl Prop for PinMutexExclusive {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let node = backend.egrid.node(nloc);
        let pin_info = &backend.egrid.db.nodes[node.kind].bels[self.bel].pins[&self.pin];
        for &wire in &pin_info.wires {
            let nw = (nloc.0, node.tiles[wire.0], wire.1);
            let rw = backend.egrid.resolve_wire(nw)?;
            fuzzer = fuzzer.fuzz(Key::NodeMutex(rw), false, true);
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzBelPipPin {
    pub bel: BelId,
    pub key: String,
    pub pin: String,
}

impl FuzzBelPipPin {
    pub fn new(bel: BelId, key: String, pin: String) -> Self {
        Self { bel, key, pin }
    }
}

impl Prop for FuzzBelPipPin {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let nnode = &backend.ngrid.nodes[&nloc];
        let bname = &nnode.bels[self.bel][0];
        let crd = backend.ngrid.bel_pip(nloc, self.bel, &self.key);
        Some((
            fuzzer.fuzz(Key::Pip(crd), None, Value::FromPin(bname, self.pin.clone())),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct BondedIo {
    pub bel: BelId,
}

impl BondedIo {
    pub fn new(bel: BelId) -> Self {
        Self { bel }
    }
}

impl Prop for BondedIo {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let io = backend.edev.grid.get_io_crd(nloc.1, nloc.2, self.bel);
        if backend.edev.grid.unbonded_io.contains(&io) {
            None
        } else {
            Some((fuzzer, false))
        }
    }
}

pub fn get_bits(backend: &XactBackend, nloc: NodeLoc) -> Vec<BitTile> {
    let edev = backend.edev;
    let node = backend.egrid.node(nloc);
    let kind = backend.egrid.db.nodes.key(node.kind);
    match backend.edev.grid.kind {
        GridKind::Xc2000 => {
            if kind.starts_with("BIDI") {
                todo!()
            } else {
                let mut res = vec![edev.btile_main(nloc.1, nloc.2)];
                if nloc.1 != edev.grid.col_rio()
                    && (nloc.2 == edev.grid.row_bio() || nloc.2 == edev.grid.row_tio())
                {
                    res.push(edev.btile_main(nloc.1 + 1, nloc.2));
                }
                res
            }
        }
        GridKind::Xc3000 | GridKind::Xc3000A => {
            if kind.starts_with("LLH") || (kind.starts_with("LLV") && kind.ends_with('S')) {
                vec![edev.btile_main(nloc.1, nloc.2)]
            } else if kind.starts_with("LLV") {
                vec![
                    edev.btile_llv(nloc.1, nloc.2),
                    edev.btile_main(nloc.1, nloc.2),
                ]
            } else {
                let mut res = vec![edev.btile_main(nloc.1, nloc.2)];
                if nloc.2 != edev.grid.row_tio() {
                    res.push(edev.btile_main(nloc.1, nloc.2 + 1));
                }
                res
            }
        }
        GridKind::Xc4000
        | GridKind::Xc4000A
        | GridKind::Xc4000H
        | GridKind::Xc4000E
        | GridKind::Xc4000Ex
        | GridKind::Xc4000Xla
        | GridKind::Xc4000Xv
        | GridKind::SpartanXl => {
            if kind.starts_with("LLH") {
                if nloc.2 == edev.grid.row_bio() {
                    vec![
                        edev.btile_llh(nloc.1, nloc.2),
                        edev.btile_main(nloc.1 - 1, nloc.2),
                    ]
                } else if nloc.2 == edev.grid.row_tio() {
                    vec![
                        edev.btile_llh(nloc.1, nloc.2),
                        edev.btile_llh(nloc.1, nloc.2 - 1),
                        edev.btile_main(nloc.1 - 1, nloc.2),
                    ]
                } else if nloc.2 == edev.grid.row_bio() + 1 {
                    vec![
                        edev.btile_llh(nloc.1, nloc.2),
                        edev.btile_llh(nloc.1, nloc.2 - 1),
                        edev.btile_main(nloc.1 - 1, nloc.2 - 1),
                    ]
                } else {
                    vec![
                        edev.btile_llh(nloc.1, nloc.2),
                        edev.btile_llh(nloc.1, nloc.2 - 1),
                    ]
                }
            } else if kind.starts_with("LLV") {
                if nloc.1 == edev.grid.col_lio() {
                    vec![
                        edev.btile_llv(nloc.1, nloc.2),
                        edev.btile_llv(nloc.1 + 1, nloc.2),
                    ]
                } else {
                    vec![edev.btile_llv(nloc.1, nloc.2)]
                }
            } else {
                if nloc.1 == edev.grid.col_lio() {
                    if nloc.2 == edev.grid.row_bio() {
                        // LL
                        vec![edev.btile_main(nloc.1, nloc.2)]
                    } else if nloc.2 == edev.grid.row_tio() {
                        // UL
                        vec![edev.btile_main(nloc.1, nloc.2)]
                    } else {
                        // LEFT
                        vec![
                            edev.btile_main(nloc.1, nloc.2),
                            edev.btile_main(nloc.1, nloc.2 - 1),
                        ]
                    }
                } else if nloc.1 == edev.grid.col_rio() {
                    if nloc.2 == edev.grid.row_bio() {
                        // LR
                        vec![edev.btile_main(nloc.1, nloc.2)]
                    } else if nloc.2 == edev.grid.row_tio() {
                        // UR
                        vec![
                            edev.btile_main(nloc.1, nloc.2),
                            edev.btile_main(nloc.1, nloc.2 - 1),
                            edev.btile_main(nloc.1 - 1, nloc.2),
                        ]
                    } else {
                        // RT
                        vec![
                            edev.btile_main(nloc.1, nloc.2),
                            edev.btile_main(nloc.1, nloc.2 - 1),
                            edev.btile_main(nloc.1 - 1, nloc.2),
                        ]
                    }
                } else {
                    if nloc.2 == edev.grid.row_bio() {
                        // BOT
                        vec![
                            edev.btile_main(nloc.1, nloc.2),
                            edev.btile_main(nloc.1 + 1, nloc.2),
                        ]
                    } else if nloc.2 == edev.grid.row_tio() {
                        // TOP
                        vec![
                            edev.btile_main(nloc.1, nloc.2),
                            edev.btile_main(nloc.1, nloc.2 - 1),
                            edev.btile_main(nloc.1 + 1, nloc.2),
                            edev.btile_main(nloc.1 - 1, nloc.2),
                        ]
                    } else {
                        // CLB
                        vec![
                            edev.btile_main(nloc.1, nloc.2),
                            edev.btile_main(nloc.1, nloc.2 - 1),
                            edev.btile_main(nloc.1 - 1, nloc.2),
                            edev.btile_main(nloc.1, nloc.2 + 1),
                            edev.btile_main(nloc.1 + 1, nloc.2),
                        ]
                    }
                }
            }
        }
        GridKind::Xc5200 => {
            if matches!(&kind[..], "CLKL" | "CLKR" | "CLKH") {
                vec![edev.btile_llv(nloc.1, nloc.2)]
            } else if matches!(&kind[..], "CLKB" | "CLKT" | "CLKV") {
                vec![edev.btile_llh(nloc.1, nloc.2)]
            } else {
                vec![edev.btile_main(nloc.1, nloc.2)]
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExtraTile {
    pub nloc: NodeLoc,
    pub bel: String,
    pub attr: String,
    pub val: String,
}

impl ExtraTile {
    pub fn new(nloc: NodeLoc, bel: String, attr: String, val: String) -> Self {
        Self {
            nloc,
            bel,
            attr,
            val,
        }
    }
}

impl Prop for ExtraTile {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        _nloc: NodeLoc,
        mut fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let node = backend.egrid.node(self.nloc);
        let tile = backend.egrid.db.nodes.key(node.kind);
        fuzzer.info.features.push(FuzzerFeature {
            id: FeatureId {
                tile: tile.into(),
                bel: self.bel.clone(),
                attr: self.attr.clone(),
                val: self.val.clone(),
            },
            tiles: get_bits(backend, self.nloc),
        });
        Some((fuzzer, false))
    }
}

#[derive(Debug)]
pub struct XactFuzzerGen {
    pub node: NodeKindId,
    pub feature: FeatureId,
    pub props: Vec<Box<dyn Prop>>,
}

impl Clone for XactFuzzerGen {
    fn clone(&self) -> Self {
        Self {
            node: self.node,
            feature: self.feature.clone(),
            props: self.props.iter().map(|x| x.dyn_clone()).collect(),
        }
    }
}

impl XactFuzzerGen {
    fn try_gen<'b>(
        &self,
        backend: &XactBackend<'b>,
        kv: &HashMap<Key<'b>, BatchValue<XactBackend<'b>>>,
        nloc: NodeLoc,
    ) -> Option<(Fuzzer<XactBackend<'b>>, BTreeSet<usize>)> {
        let tiles = get_bits(backend, nloc);
        let mut fuzzer = Fuzzer::new(FuzzerInfo {
            features: vec![FuzzerFeature {
                tiles,
                id: self.feature.clone(),
            }],
        });
        let mut sad_props = BTreeSet::new();
        for (idx, prop) in self.props.iter().enumerate() {
            let sad;
            (fuzzer, sad) = prop.apply(backend, nloc, fuzzer)?;
            if sad {
                sad_props.insert(idx);
            }
        }
        if !fuzzer.is_ok(kv) {
            return None;
        }
        Some((fuzzer, sad_props))
    }
}

impl<'b> FuzzerGen<XactBackend<'b>> for XactFuzzerGen {
    fn gen<'a>(
        &self,
        backend: &'a XactBackend<'b>,
        _state: &mut State,
        kv: &HashMap<Key<'b>, BatchValue<XactBackend<'b>>>,
    ) -> Option<(
        Fuzzer<XactBackend<'b>>,
        Option<Box<dyn FuzzerGen<XactBackend<'b>> + 'a>>,
    )> {
        let locs = &backend.egrid.node_index[self.node];
        let mut rng = rand::rng();
        let (res, sad_props) = 'find: {
            if locs.len() > 20 {
                for &loc in locs.choose_multiple(&mut rng, 20) {
                    if let Some(x) = self.try_gen(backend, kv, loc) {
                        break 'find x;
                    }
                }
            }
            for &loc in locs.choose_multiple(&mut rng, locs.len()) {
                if let Some(x) = self.try_gen(backend, kv, loc) {
                    break 'find x;
                }
            }
            return None;
        };
        if !sad_props.is_empty() {
            return Some((
                res,
                Some(Box::new(XactFuzzerChainGen {
                    orig: self.clone(),
                    sad_props,
                })),
            ));
        }
        Some((res, None))
    }
}

#[derive(Debug)]
struct XactFuzzerChainGen {
    orig: XactFuzzerGen,
    sad_props: BTreeSet<usize>,
}

impl<'b> FuzzerGen<XactBackend<'b>> for XactFuzzerChainGen {
    fn gen<'a>(
        &self,
        backend: &'a XactBackend<'b>,
        _state: &mut State,
        kv: &HashMap<Key<'b>, BatchValue<XactBackend<'b>>>,
    ) -> Option<(
        Fuzzer<XactBackend<'b>>,
        Option<Box<dyn FuzzerGen<XactBackend<'b>> + 'a>>,
    )> {
        let locs = &backend.egrid.node_index[self.orig.node];
        let mut rng = rand::rng();
        let (res, mut sad_props) = 'find: {
            if locs.len() > 20 {
                for &loc in locs.choose_multiple(&mut rng, 20) {
                    if let Some(x) = self.orig.try_gen(backend, kv, loc) {
                        for &prop in &self.sad_props {
                            if !x.1.contains(&prop) {
                                break 'find x;
                            }
                        }
                    }
                }
            }
            for &loc in locs.choose_multiple(&mut rng, locs.len()) {
                if let Some(x) = self.orig.try_gen(backend, kv, loc) {
                    for &prop in &self.sad_props {
                        if !x.1.contains(&prop) {
                            break 'find x;
                        }
                    }
                }
            }
            return None;
        };
        sad_props.retain(|&idx| self.sad_props.contains(&idx));
        if !sad_props.is_empty() {
            return Some((
                res,
                Some(Box::new(XactFuzzerChainGen {
                    orig: self.orig.clone(),
                    sad_props,
                })),
            ));
        }
        Some((res, None))
    }
}
