use prjcombine_interconnect::{db::TileWireCoord, grid::TileCoord};
use prjcombine_re_fpga_hammer::FuzzerProp;
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
        for (_, cell) in backend.egrid.tile_cells(tcrd) {
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
        nloc: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        Some((
            fuzzer.base(Key::TileMutex(nloc, self.key.clone()), self.val.clone()),
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
        nloc: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        Some((
            fuzzer.fuzz(Key::TileMutex(nloc, self.key.clone()), None, "EXCLUSIVE"),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct NodeMutexShared {
    pub wire: TileWireCoord,
}

impl NodeMutexShared {
    pub fn new(wire: TileWireCoord) -> Self {
        Self { wire }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for NodeMutexShared {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let node = backend
            .egrid
            .resolve_wire(backend.egrid.tile_wire(tcrd, self.wire))?;
        Some((fuzzer.base(Key::NodeMutex(node), "SHARED"), false))
    }
}

#[derive(Clone, Debug)]
pub struct NodeMutexExclusive {
    pub wire: TileWireCoord,
}

impl NodeMutexExclusive {
    pub fn new(wire: TileWireCoord) -> Self {
        Self { wire }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for NodeMutexExclusive {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let node = backend
            .egrid
            .resolve_wire(backend.egrid.tile_wire(tcrd, self.wire))?;
        Some((fuzzer.fuzz(Key::NodeMutex(node), None, "EXCLUSIVE"), false))
    }
}
