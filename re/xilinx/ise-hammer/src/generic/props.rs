use core::fmt::Debug;

use prjcombine_interconnect::grid::TileCoord;
use prjcombine_re_fpga_hammer::backend::FuzzerProp;
use prjcombine_re_hammer::Fuzzer;
use relation::TileRelation;

use crate::backend::{IseBackend, Key, MultiValue, Value};

pub mod bel;
pub mod extra;
pub mod mutex;
pub mod pip;
pub mod relation;

pub type DynProp<'b> = dyn FuzzerProp<'b, IseBackend<'b>> + 'b;

#[derive(Clone, Debug)]
pub struct BaseRaw<'b> {
    pub key: Key<'b>,
    pub val: Value<'b>,
}

impl<'b> BaseRaw<'b> {
    pub fn new(key: Key<'b>, val: Value<'b>) -> Self {
        Self { key, val }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for BaseRaw<'b> {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        _backend: &IseBackend<'b>,
        _tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        Some((fuzzer.base(self.key.clone(), self.val.clone()), false))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzRaw<'b> {
    pub key: Key<'b>,
    pub val0: Value<'b>,
    pub val1: Value<'b>,
}

impl<'b> FuzzRaw<'b> {
    pub fn new(key: Key<'b>, val0: Value<'b>, val1: Value<'b>) -> Self {
        Self { key, val0, val1 }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for FuzzRaw<'b> {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        _backend: &IseBackend<'b>,
        _tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        Some((
            fuzzer.fuzz(self.key.clone(), self.val0.clone(), self.val1.clone()),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzRawMulti<'b> {
    pub key: Key<'b>,
    pub val: MultiValue,
    pub width: usize,
}

impl<'b> FuzzRawMulti<'b> {
    pub fn new(key: Key<'b>, val: MultiValue, width: usize) -> Self {
        Self { key, val, width }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for FuzzRawMulti<'b> {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        _backend: &IseBackend<'b>,
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        fuzzer.bits = self.width;
        Some((fuzzer.fuzz_multi(self.key.clone(), self.val), false))
    }
}

#[derive(Clone, Debug)]
pub struct NullBits;

impl<'b> FuzzerProp<'b, IseBackend<'b>> for NullBits {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        _backend: &IseBackend<'a>,
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        fuzzer.info.features[0].rects.clear();
        Some((fuzzer, false))
    }
}
