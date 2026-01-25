use prjcombine_interconnect::{db::TileWireCoord, grid::TileCoord};
use prjcombine_re_fpga_hammer::backend::FuzzerProp;
use prjcombine_re_hammer::Fuzzer;

use crate::backend::{IseBackend, Key};

use super::DynProp;

#[derive(Clone, Debug)]
pub struct IntMutex {
    pub val: String,
}

impl IntMutex {
    pub fn new(val: String) -> Self {
        Self { val }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for IntMutex {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        for (_, cell) in backend.edev.tile_cells(tcrd) {
            fuzzer = fuzzer.base(Key::IntMutex(cell), self.val.clone());
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct RowMutex {
    pub key: String,
    pub val: String,
}

impl RowMutex {
    pub fn new(key: String, val: String) -> Self {
        Self { key, val }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for RowMutex {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        _backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        Some((
            fuzzer.base(Key::RowMutex(self.key.clone(), tcrd.row), self.val.clone()),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct TileMutex {
    pub key: String,
    pub val: String,
}

impl TileMutex {
    pub fn new(key: String, val: String) -> Self {
        Self { key, val }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for TileMutex {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        _backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        Some((
            fuzzer.base(Key::TileMutex(tcrd, self.key.clone()), self.val.clone()),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct TileMutexExclusive {
    pub key: String,
}

impl TileMutexExclusive {
    pub fn new(key: String) -> Self {
        Self { key }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for TileMutexExclusive {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        _backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        Some((
            fuzzer.fuzz(Key::TileMutex(tcrd, self.key.clone()), None, "EXCLUSIVE"),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct WireMutexShared {
    pub wire: TileWireCoord,
}

impl WireMutexShared {
    pub fn new(wire: TileWireCoord) -> Self {
        Self { wire }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for WireMutexShared {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let wire = backend
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, self.wire))?;
        Some((fuzzer.base(Key::WireMutex(wire), "SHARED"), false))
    }
}

#[derive(Clone, Debug)]
pub struct WireMutexExclusive {
    pub wire: TileWireCoord,
}

impl WireMutexExclusive {
    pub fn new(wire: TileWireCoord) -> Self {
        Self { wire }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for WireMutexExclusive {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let wire = backend
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, self.wire))?;
        Some((fuzzer.fuzz(Key::WireMutex(wire), None, "EXCLUSIVE"), false))
    }
}
