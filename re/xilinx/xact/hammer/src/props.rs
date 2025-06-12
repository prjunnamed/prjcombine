use prjcombine_interconnect::{db::BelSlotId, grid::TileCoord};
use prjcombine_re_fpga_hammer::{FeatureId, FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::Fuzzer;
use prjcombine_types::bitvec::BitVec;

use crate::backend::{Key, MultiValue, Value, XactBackend};

pub type DynProp<'b> = dyn FuzzerProp<'b, XactBackend<'b>>;

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

impl<'b> FuzzerProp<'b, XactBackend<'b>> for BaseRaw {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        _backend: &XactBackend<'a>,
        _tcrd: TileCoord,
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

impl<'b> FuzzerProp<'b, XactBackend<'b>> for FuzzRaw {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        _backend: &XactBackend<'a>,
        _tcrd: TileCoord,
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
    pub bel: BelSlotId,
    pub val: String,
}

impl BaseBelMode {
    pub fn new(bel: BelSlotId, val: String) -> Self {
        Self { bel, val }
    }
}

impl<'b> FuzzerProp<'b, XactBackend<'b>> for BaseBelMode {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let nnode = &backend.ngrid.tiles[&tcrd];
        Some((
            fuzzer.base(Key::BlockBase(&nnode.bels[self.bel][0]), self.val.clone()),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzBelMode {
    pub bel: BelSlotId,
    pub val: String,
}

impl FuzzBelMode {
    pub fn new(bel: BelSlotId, val: String) -> Self {
        Self { bel, val }
    }
}

impl<'b> FuzzerProp<'b, XactBackend<'b>> for FuzzBelMode {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let nnode = &backend.ngrid.tiles[&tcrd];
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
    pub bel: BelSlotId,
    pub attr: String,
    pub val: String,
}

impl BaseBelMutex {
    pub fn new(bel: BelSlotId, attr: String, val: String) -> Self {
        Self { bel, attr, val }
    }
}

impl<'b> FuzzerProp<'b, XactBackend<'b>> for BaseBelMutex {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        _backend: &XactBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        Some((
            fuzzer.base(
                Key::BelMutex(tcrd.bel(self.bel), self.attr.clone()),
                self.val.clone(),
            ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct BaseBelConfig {
    pub bel: BelSlotId,
    pub attr: String,
    pub val: String,
}

impl BaseBelConfig {
    pub fn new(bel: BelSlotId, attr: String, val: String) -> Self {
        Self { bel, attr, val }
    }
}

impl<'b> FuzzerProp<'b, XactBackend<'b>> for BaseBelConfig {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let nnode = &backend.ngrid.tiles[&tcrd];
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
    pub bel: BelSlotId,
    pub attr: String,
    pub val: String,
}

impl BaseBelNoConfig {
    pub fn new(bel: BelSlotId, attr: String, val: String) -> Self {
        Self { bel, attr, val }
    }
}

impl<'b> FuzzerProp<'b, XactBackend<'b>> for BaseBelNoConfig {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let nnode = &backend.ngrid.tiles[&tcrd];
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
    pub bel: BelSlotId,
    pub attr: String,
    pub val: String,
}

impl FuzzBelConfig {
    pub fn new(bel: BelSlotId, attr: String, val: String) -> Self {
        Self { bel, attr, val }
    }
}

impl<'b> FuzzerProp<'b, XactBackend<'b>> for FuzzBelConfig {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let nnode = &backend.ngrid.tiles[&tcrd];
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
    pub bel: BelSlotId,
    pub attr: String,
    pub val0: String,
    pub val1: String,
}

impl FuzzBelConfigDiff {
    pub fn new(bel: BelSlotId, attr: String, val0: String, val1: String) -> Self {
        Self {
            bel,
            attr,
            val0,
            val1,
        }
    }
}

impl<'b> FuzzerProp<'b, XactBackend<'b>> for FuzzBelConfigDiff {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let nnode = &backend.ngrid.tiles[&tcrd];
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
    pub bel: BelSlotId,
    pub attr: String,
    pub inps: &'static [&'static str],
}

impl FuzzEquate {
    pub fn new(bel: BelSlotId, attr: String, inps: &'static [&'static str]) -> Self {
        Self { bel, attr, inps }
    }
}

impl<'b> FuzzerProp<'b, XactBackend<'b>> for FuzzEquate {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let nnode = &backend.ngrid.tiles[&tcrd];
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
    pub bel: BelSlotId,
    pub attr: String,
    pub inps: &'static [&'static str],
    pub bits: BitVec,
}

impl FuzzEquateFixed {
    pub fn new(bel: BelSlotId, attr: String, inps: &'static [&'static str], bits: BitVec) -> Self {
        Self {
            bel,
            attr,
            inps,
            bits,
        }
    }
}

impl<'b> FuzzerProp<'b, XactBackend<'b>> for FuzzEquateFixed {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let nnode = &backend.ngrid.tiles[&tcrd];
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
    pub bel: BelSlotId,
    pub key: String,
    pub buf: &'static str,
}

impl FuzzBelPipBufg {
    pub fn new(bel: BelSlotId, key: String, buf: &'static str) -> Self {
        Self { bel, key, buf }
    }
}

impl<'b> FuzzerProp<'b, XactBackend<'b>> for FuzzBelPipBufg {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let crd = backend.ngrid.bel_pip(tcrd.bel(self.bel), &self.key);
        Some((
            fuzzer.fuzz(Key::Pip(crd), None, Value::FromPin(self.buf, "O".into())),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct PinMutexExclusive {
    pub bel: BelSlotId,
    pub pin: String,
}

impl PinMutexExclusive {
    pub fn new(bel: BelSlotId, pin: String) -> Self {
        Self { bel, pin }
    }
}

impl<'b> FuzzerProp<'b, XactBackend<'b>> for PinMutexExclusive {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        for wire in backend.egrid.get_bel_pin(tcrd.bel(self.bel), &self.pin) {
            let rw = backend.egrid.resolve_wire(wire)?;
            fuzzer = fuzzer.fuzz(Key::NodeMutex(rw), false, true);
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzBelPipPin {
    pub bel: BelSlotId,
    pub key: String,
    pub pin: String,
}

impl FuzzBelPipPin {
    pub fn new(bel: BelSlotId, key: String, pin: String) -> Self {
        Self { bel, key, pin }
    }
}

impl<'b> FuzzerProp<'b, XactBackend<'b>> for FuzzBelPipPin {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let nnode = &backend.ngrid.tiles[&tcrd];
        let bname = &nnode.bels[self.bel][0];
        let crd = backend.ngrid.bel_pip(tcrd.bel(self.bel), &self.key);
        Some((
            fuzzer.fuzz(Key::Pip(crd), None, Value::FromPin(bname, self.pin.clone())),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct BondedIo {
    pub bel: BelSlotId,
}

impl BondedIo {
    pub fn new(bel: BelSlotId) -> Self {
        Self { bel }
    }
}

impl<'b> FuzzerProp<'b, XactBackend<'b>> for BondedIo {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let io = backend.edev.chip.get_io_crd(tcrd.bel(self.bel));
        if backend.edev.chip.unbonded_io.contains(&io) {
            None
        } else {
            Some((fuzzer, false))
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExtraTile {
    pub tcrd: TileCoord,
    pub bel: String,
    pub attr: String,
    pub val: String,
}

impl ExtraTile {
    pub fn new(tcrd: TileCoord, bel: String, attr: String, val: String) -> Self {
        Self {
            tcrd,
            bel,
            attr,
            val,
        }
    }
}

impl<'b> FuzzerProp<'b, XactBackend<'b>> for ExtraTile {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let node = backend.egrid.tile(self.tcrd);
        let tile = backend.egrid.db.tile_classes.key(node.class);
        fuzzer.info.features.push(FuzzerFeature {
            id: FeatureId {
                tile: tile.into(),
                bel: self.bel.clone(),
                attr: self.attr.clone(),
                val: self.val.clone(),
            },
            tiles: backend.edev.tile_bits(self.tcrd),
        });
        Some((fuzzer, false))
    }
}
