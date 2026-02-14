use core::fmt::Debug;

use prjcombine_interconnect::{
    db::{TileClassId, TileSlotId},
    grid::TileCoord,
};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::Fuzzer;

use crate::backend::IseBackend;

use super::DynProp;

pub trait TileRelation: Clone + Debug {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord>;
}

#[derive(Clone, Copy, Debug)]
pub struct NoopRelation;

impl TileRelation for NoopRelation {
    fn resolve(&self, _backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        Some(tcrd)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FixedRelation(pub TileCoord);

impl TileRelation for FixedRelation {
    fn resolve(&self, _backend: &IseBackend, _tcrd: TileCoord) -> Option<TileCoord> {
        Some(self.0)
    }
}

#[derive(Clone, Debug)]
pub struct Delta {
    pub dx: i32,
    pub dy: i32,
    pub tcids: Vec<TileClassId>,
}

impl Delta {
    pub fn new(dx: i32, dy: i32, tcid: TileClassId) -> Self {
        Self {
            dx,
            dy,
            tcids: vec![tcid],
        }
    }

    pub fn new_any(dx: i32, dy: i32, tcids: &[TileClassId]) -> Self {
        Self {
            dx,
            dy,
            tcids: tcids.to_vec(),
        }
    }
}

impl TileRelation for Delta {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let cell = backend.edev.cell_delta(tcrd.cell, self.dx, self.dy)?;
        for &tcid in &self.tcids {
            let tcrd = cell.tile(backend.edev.db[tcid].slot);
            if let Some(tile) = backend.edev.get_tile(tcrd)
                && tile.class == tcid
            {
                return Some(tcrd);
            }
        }
        None
    }
}

#[derive(Clone, Debug)]
pub struct DeltaSlot {
    pub dx: i32,
    pub dy: i32,
    pub slot: TileSlotId,
}

impl DeltaSlot {
    pub fn new(dx: i32, dy: i32, slot: TileSlotId) -> Self {
        Self { dx, dy, slot }
    }
}

impl TileRelation for DeltaSlot {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let cell = backend.edev.cell_delta(tcrd.cell, self.dx, self.dy)?;
        if backend.edev[cell].tiles.contains_id(self.slot) {
            Some(cell.tile(self.slot))
        } else {
            None
        }
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
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let tcrd = self.relation.resolve(backend, tcrd)?;
        self.prop.apply(backend, tcrd, fuzzer)
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
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        if self.relation.resolve(backend, tcrd).is_some() == self.val {
            Some((fuzzer, false))
        } else {
            None
        }
    }
}
