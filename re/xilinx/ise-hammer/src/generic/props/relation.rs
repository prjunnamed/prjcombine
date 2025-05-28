use core::fmt::Debug;

use unnamed_entity::EntityId;

use prjcombine_interconnect::grid::NodeLoc;
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::Fuzzer;

use crate::backend::IseBackend;

use super::DynProp;

pub trait NodeRelation: Clone + Debug {
    fn resolve(&self, backend: &IseBackend, nloc: NodeLoc) -> Option<NodeLoc>;
}

#[derive(Clone, Copy, Debug)]
pub struct NoopRelation;

impl NodeRelation for NoopRelation {
    fn resolve(&self, _backend: &IseBackend, nloc: NodeLoc) -> Option<NodeLoc> {
        Some(nloc)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FixedRelation(pub NodeLoc);

impl NodeRelation for FixedRelation {
    fn resolve(&self, _backend: &IseBackend, _nloc: NodeLoc) -> Option<NodeLoc> {
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

impl NodeRelation for Delta {
    fn resolve(&self, backend: &IseBackend, mut nloc: NodeLoc) -> Option<NodeLoc> {
        if self.dx < 0 {
            if nloc.1.to_idx() < (-self.dx) as usize {
                return None;
            }
            nloc.1 -= (-self.dx) as usize;
        } else {
            nloc.1 += self.dx as usize;
            if nloc.1.to_idx() >= backend.egrid.die(nloc.0).cols().len() {
                return None;
            }
        }
        if self.dy < 0 {
            if nloc.2.to_idx() < (-self.dy) as usize {
                return None;
            }
            nloc.2 -= (-self.dy) as usize;
        } else {
            nloc.2 += self.dy as usize;
            if nloc.2.to_idx() >= backend.egrid.die(nloc.0).rows().len() {
                return None;
            }
        }
        let layer = backend
            .egrid
            .find_tile_layer(nloc.0, (nloc.1, nloc.2), |node| {
                self.nodes.iter().any(|x| x == node)
            })?;
        nloc.3 = layer;
        Some(nloc)
    }
}

#[derive(Clone, Debug)]
pub struct Related<'b, R: NodeRelation> {
    pub relation: R,
    pub prop: Box<DynProp<'b>>,
}

impl<'b, R: NodeRelation> Related<'b, R> {
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

impl<'b, R: NodeRelation + 'b> FuzzerProp<'b, IseBackend<'b>> for Related<'b, R> {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let nloc = self.relation.resolve(backend, nloc)?;
        self.prop.apply(backend, nloc, fuzzer)
    }
}

#[derive(Clone, Debug)]
pub struct HasRelated<R: NodeRelation> {
    pub relation: R,
    pub val: bool,
}

impl<R: NodeRelation> HasRelated<R> {
    pub fn new(relation: R, val: bool) -> Self {
        Self { relation, val }
    }
}

impl<'b, R: NodeRelation + 'b> FuzzerProp<'b, IseBackend<'b>> for HasRelated<R> {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        if self.relation.resolve(backend, nloc).is_some() == self.val {
            Some((fuzzer, false))
        } else {
            None
        }
    }
}
