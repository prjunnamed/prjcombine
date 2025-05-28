use prjcombine_interconnect::{db::TileClassWire, grid::NodeLoc};
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
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let node = backend.egrid.tile(nloc);
        for &(col, row) in node.cells.values() {
            fuzzer = fuzzer.base(Key::IntMutex(nloc.0, col, row), self.val.clone());
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
        nloc: NodeLoc,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        Some((
            fuzzer.base(Key::RowMutex(self.key.clone(), nloc.2), self.val.clone()),
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
        nloc: NodeLoc,
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
        nloc: NodeLoc,
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
    pub wire: TileClassWire,
}

impl NodeMutexShared {
    pub fn new(wire: TileClassWire) -> Self {
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
        nloc: NodeLoc,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let node = backend.egrid.tile(nloc);
        let node = backend
            .egrid
            .resolve_wire((nloc.0, node.cells[self.wire.0], self.wire.1))?;
        Some((fuzzer.base(Key::NodeMutex(node), "SHARED"), false))
    }
}

#[derive(Clone, Debug)]
pub struct NodeMutexExclusive {
    pub wire: TileClassWire,
}

impl NodeMutexExclusive {
    pub fn new(wire: TileClassWire) -> Self {
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
        nloc: NodeLoc,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let node = backend.egrid.tile(nloc);
        let node = backend
            .egrid
            .resolve_wire((nloc.0, node.cells[self.wire.0], self.wire.1))?;
        Some((fuzzer.fuzz(Key::NodeMutex(node), None, "EXCLUSIVE"), false))
    }
}
