use core::fmt::Debug;

use prjcombine_interconnect::{
    db::{BelSlotId, TileCellId, TileClassWire},
    grid::NodeLoc,
};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::Fuzzer;
use prjcombine_re_xilinx_naming::db::RawTileId;
use unnamed_entity::EntityId;

use crate::backend::{IseBackend, Key};

use super::{DynProp, NodeRelation};

pub struct PipInt;
pub struct PinFar;

pub trait BelIntoPipWire {
    fn into_pip_wire(self, backend: &IseBackend, slot: BelSlotId) -> PipWire;
}

impl BelIntoPipWire for &str {
    fn into_pip_wire(self, _backend: &IseBackend, slot: BelSlotId) -> PipWire {
        PipWire::BelPinNear(slot, self.into())
    }
}

impl BelIntoPipWire for &String {
    fn into_pip_wire(self, _backend: &IseBackend, slot: BelSlotId) -> PipWire {
        PipWire::BelPinNear(slot, self.clone())
    }
}

impl BelIntoPipWire for String {
    fn into_pip_wire(self, _backend: &IseBackend, slot: BelSlotId) -> PipWire {
        PipWire::BelPinNear(slot, self)
    }
}

impl BelIntoPipWire for (BelSlotId, &str) {
    fn into_pip_wire(self, _backend: &IseBackend, _slot: BelSlotId) -> PipWire {
        PipWire::BelPinNear(self.0, self.1.into())
    }
}

impl BelIntoPipWire for (BelSlotId, &String) {
    fn into_pip_wire(self, _backend: &IseBackend, _slot: BelSlotId) -> PipWire {
        PipWire::BelPinNear(self.0, self.1.clone())
    }
}

impl BelIntoPipWire for (BelSlotId, String) {
    fn into_pip_wire(self, _backend: &IseBackend, _slot: BelSlotId) -> PipWire {
        PipWire::BelPinNear(self.0, self.1)
    }
}

impl BelIntoPipWire for (PinFar, &str) {
    fn into_pip_wire(self, _backend: &IseBackend, slot: BelSlotId) -> PipWire {
        PipWire::BelPinFar(slot, self.1.into())
    }
}

impl BelIntoPipWire for (PinFar, String) {
    fn into_pip_wire(self, _backend: &IseBackend, slot: BelSlotId) -> PipWire {
        PipWire::BelPinFar(slot, self.1)
    }
}

impl BelIntoPipWire for (PinFar, BelSlotId, &str) {
    fn into_pip_wire(self, _backend: &IseBackend, _slot: BelSlotId) -> PipWire {
        PipWire::BelPinFar(self.1, self.2.into())
    }
}

impl BelIntoPipWire for (PinFar, BelSlotId, String) {
    fn into_pip_wire(self, _backend: &IseBackend, _slot: BelSlotId) -> PipWire {
        PipWire::BelPinFar(self.1, self.2)
    }
}

impl BelIntoPipWire for PipWire {
    fn into_pip_wire(self, _backend: &IseBackend, _slot: BelSlotId) -> PipWire {
        self
    }
}

impl BelIntoPipWire for (PipInt, usize, &str) {
    fn into_pip_wire(self, backend: &IseBackend, _slot: BelSlotId) -> PipWire {
        let tile = TileCellId::from_idx(self.1);
        let wire = backend.egrid.db.get_wire(self.2);
        PipWire::Int((tile, wire))
    }
}

impl BelIntoPipWire for (PipInt, usize, String) {
    fn into_pip_wire(self, backend: &IseBackend, _slot: BelSlotId) -> PipWire {
        let tile = TileCellId::from_idx(self.1);
        let wire = backend.egrid.db.get_wire(&self.2);
        PipWire::Int((tile, wire))
    }
}

#[derive(Clone, Debug)]
pub enum PipWire {
    Int(TileClassWire),
    BelPinNear(BelSlotId, String),
    BelPinFar(BelSlotId, String),
}

impl PipWire {
    pub fn resolve<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: NodeLoc,
    ) -> Option<(&'a str, &'a str)> {
        let node = backend.egrid.tile(nloc);
        let ndb = backend.ngrid.db;
        let nnode = &backend.ngrid.tiles[&nloc];
        let node_naming = &ndb.tile_class_namings[nnode.naming];
        Some(match self {
            PipWire::Int(wire) => {
                backend
                    .egrid
                    .resolve_wire((nloc.0, node.cells[wire.0], wire.1))?;
                (
                    &nnode.names[RawTileId::from_idx(0)],
                    node_naming.wires.get(wire)?,
                )
            }
            PipWire::BelPinNear(bel, pin) => {
                let bel_naming = &node_naming.bels[*bel];
                (
                    &nnode.names[bel_naming.tile],
                    &bel_naming
                        .pins
                        .get(pin)
                        .unwrap_or_else(|| {
                            panic!(
                                "missing pin {pin} in bel {bel} tile {tile}",
                                bel = backend.egrid.db.bel_slots[*bel],
                                tile = backend.egrid.db.tile_classes.key(node.class),
                            )
                        })
                        .name,
                )
            }
            PipWire::BelPinFar(bel, pin) => {
                let bel_naming = &node_naming.bels[*bel];
                (
                    &nnode.names[bel_naming.tile],
                    &bel_naming
                        .pins
                        .get(pin)
                        .unwrap_or_else(|| {
                            panic!(
                                "missing pin {pin} in bel {bel} tile {tile}",
                                bel = backend.egrid.db.bel_slots[*bel],
                                tile = backend.egrid.db.tile_classes.key(node.class),
                            )
                        })
                        .name_far,
                )
            }
        })
    }
}

#[derive(Clone, Debug)]
pub struct BasePip<R: NodeRelation> {
    pub relation: R,
    pub wire_to: PipWire,
    pub wire_from: PipWire,
}

impl<R: NodeRelation> BasePip<R> {
    pub fn new(relation: R, wire_to: PipWire, wire_from: PipWire) -> Self {
        Self {
            relation,
            wire_to,
            wire_from,
        }
    }
}

impl<'b, R: NodeRelation + 'b> FuzzerProp<'b, IseBackend<'b>> for BasePip<R> {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let nloc = self.relation.resolve(backend, nloc)?;
        let (tt, wt) = self.wire_to.resolve(backend, nloc)?;
        let (tf, wf) = self.wire_from.resolve(backend, nloc)?;
        assert_eq!(tt, tf);
        Some((fuzzer.base(Key::Pip(tt, wf, wt), true), false))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzPip<R: NodeRelation> {
    pub relation: R,
    pub wire_to: PipWire,
    pub wire_from: PipWire,
}

impl<R: NodeRelation> FuzzPip<R> {
    pub fn new(relation: R, wire_to: PipWire, wire_from: PipWire) -> Self {
        Self {
            relation,
            wire_to,
            wire_from,
        }
    }
}

impl<'b, R: NodeRelation + 'b> FuzzerProp<'b, IseBackend<'b>> for FuzzPip<R> {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let nloc = self.relation.resolve(backend, nloc)?;
        let (tt, wt) = self.wire_to.resolve(backend, nloc)?;
        let (tf, wf) = self.wire_from.resolve(backend, nloc)?;
        assert_eq!(tt, tf);
        Some((fuzzer.fuzz(Key::Pip(tt, wf, wt), false, true), false))
    }
}
