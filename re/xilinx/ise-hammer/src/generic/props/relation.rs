use core::fmt::Debug;

use prjcombine_interconnect::grid::TileCoord;
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::Fuzzer;

use crate::backend::IseBackend;

use super::DynProp;

pub trait TileRelation: Clone + Debug {
    fn resolve(&self, backend: &IseBackend, nloc: TileCoord) -> Option<TileCoord>;
}

#[derive(Clone, Copy, Debug)]
pub struct NoopRelation;

impl TileRelation for NoopRelation {
    fn resolve(&self, _backend: &IseBackend, nloc: TileCoord) -> Option<TileCoord> {
        Some(nloc)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FixedRelation(pub TileCoord);

impl TileRelation for FixedRelation {
    fn resolve(&self, _backend: &IseBackend, _nloc: TileCoord) -> Option<TileCoord> {
        Some(self.0)
    }
}

#[derive(Clone, Debug)]
pub struct Delta {
    pub dx: i32,
    pub dy: i32,
    pub nodes: Vec<String>,
}

impl Delta {
    pub fn new(dx: i32, dy: i32, node: impl Into<String>) -> Self {
        Self {
            dx,
            dy,
            nodes: vec![node.into()],
        }
    }

    pub fn new_any(dx: i32, dy: i32, nodes: &[impl AsRef<str>]) -> Self {
        Self {
            dx,
            dy,
            nodes: nodes.iter().map(|x| x.as_ref().to_string()).collect(),
        }
    }
}

impl TileRelation for Delta {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let cell = backend.egrid.cell_delta(tcrd.cell, self.dx, self.dy)?;
        backend
            .egrid
            .find_tile_by_class(cell, |node| self.nodes.iter().any(|x| x == node))
    }
}

#[derive(Clone, Debug)]
pub struct Related<'b, R: TileRelation> {
    pub relation: R,
    pub prop: Box<DynProp<'b>>,
}

impl<'b, R: TileRelation> Related<'b, R> {
    pub fn new(relation: R, prop: impl FuzzerProp<'b, IseBackend<'b>> + 'b) -> Self {
        Self {
            relation,
            prop: Box::new(prop),
        }
    }

    pub fn new_boxed(relation: R, prop: Box<DynProp<'b>>) -> Self {
        Self { relation, prop }
    }
}

impl<'b, R: TileRelation + 'b> FuzzerProp<'b, IseBackend<'b>> for Related<'b, R> {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        nloc: TileCoord,
        fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let nloc = self.relation.resolve(backend, nloc)?;
        self.prop.apply(backend, nloc, fuzzer)
    }
}

#[derive(Clone, Debug)]
pub struct HasRelated<R: TileRelation> {
    pub relation: R,
    pub val: bool,
}

impl<R: TileRelation> HasRelated<R> {
    pub fn new(relation: R, val: bool) -> Self {
        Self { relation, val }
    }
}

impl<'b, R: TileRelation + 'b> FuzzerProp<'b, IseBackend<'b>> for HasRelated<R> {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        nloc: TileCoord,
        fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        if self.relation.resolve(backend, nloc).is_some() == self.val {
            Some((fuzzer, false))
        } else {
            None
        }
    }
}
