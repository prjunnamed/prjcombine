use core::fmt::Debug;

use prjcombine_interconnect::{
    db::{BelSlotId, TileWireCoord},
    grid::TileCoord,
};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::Fuzzer;
use prjcombine_re_xilinx_naming::db::{BelNaming, RawTileId};
use prjcombine_entity::EntityId;

use crate::backend::{IseBackend, Key};

use super::{DynProp, TileRelation};

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
        let wire = backend.edev.db.get_wire(self.2);
        PipWire::Int(TileWireCoord::new_idx(self.1, wire))
    }
}

impl BelIntoPipWire for (PipInt, usize, String) {
    fn into_pip_wire(self, backend: &IseBackend, _slot: BelSlotId) -> PipWire {
        let wire = backend.edev.db.get_wire(&self.2);
        PipWire::Int(TileWireCoord::new_idx(self.1, wire))
    }
}

#[derive(Clone, Debug)]
pub enum PipWire {
    Int(TileWireCoord),
    BelPinNear(BelSlotId, String),
    BelPinFar(BelSlotId, String),
}

impl PipWire {
    pub fn resolve<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
    ) -> Option<(&'a str, &'a str)> {
        let tile = &backend.edev[tcrd];
        let ndb = backend.ngrid.db;
        let ntile = &backend.ngrid.tiles[&tcrd];
        let tile_naming = &ndb.tile_class_namings[ntile.naming];
        Some(match self {
            PipWire::Int(wire) => {
                backend
                    .edev
                    .resolve_wire(backend.edev.tile_wire(tcrd, *wire))?;
                (
                    &ntile.names[RawTileId::from_idx(0)],
                    tile_naming.wires.get(wire)?,
                )
            }
            PipWire::BelPinNear(bel, pin) => {
                let BelNaming::Bel(bel_naming) = &tile_naming.bels[*bel] else {
                    unreachable!()
                };
                (
                    &ntile.names[bel_naming.tile],
                    &bel_naming
                        .pins
                        .get(pin)
                        .unwrap_or_else(|| {
                            panic!(
                                "missing pin {pin} in bel {bel} tile {tile}",
                                bel = backend.edev.db.bel_slots.key(*bel),
                                tile = backend.edev.db.tile_classes.key(tile.class),
                            )
                        })
                        .name,
                )
            }
            PipWire::BelPinFar(bel, pin) => {
                let BelNaming::Bel(bel_naming) = &tile_naming.bels[*bel] else {
                    unreachable!()
                };
                (
                    &ntile.names[bel_naming.tile],
                    &bel_naming
                        .pins
                        .get(pin)
                        .unwrap_or_else(|| {
                            panic!(
                                "missing pin {pin} in bel {bel} tile {tile}",
                                bel = backend.edev.db.bel_slots.key(*bel),
                                tile = backend.edev.db.tile_classes.key(tile.class),
                            )
                        })
                        .name_far,
                )
            }
        })
    }
}

#[derive(Clone, Debug)]
pub struct BasePip<R: TileRelation> {
    pub relation: R,
    pub wire_to: PipWire,
    pub wire_from: PipWire,
}

impl<R: TileRelation> BasePip<R> {
    pub fn new(relation: R, wire_to: PipWire, wire_from: PipWire) -> Self {
        Self {
            relation,
            wire_to,
            wire_from,
        }
    }
}

impl<'b, R: TileRelation + 'b> FuzzerProp<'b, IseBackend<'b>> for BasePip<R> {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let tcrd = self.relation.resolve(backend, tcrd)?;
        let (tt, wt) = self.wire_to.resolve(backend, tcrd)?;
        let (tf, wf) = self.wire_from.resolve(backend, tcrd)?;
        assert_eq!(tt, tf);
        Some((fuzzer.base(Key::Pip(tt, wf, wt), true), false))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzPip<R: TileRelation> {
    pub relation: R,
    pub wire_to: PipWire,
    pub wire_from: PipWire,
}

impl<R: TileRelation> FuzzPip<R> {
    pub fn new(relation: R, wire_to: PipWire, wire_from: PipWire) -> Self {
        Self {
            relation,
            wire_to,
            wire_from,
        }
    }
}

impl<'b, R: TileRelation + 'b> FuzzerProp<'b, IseBackend<'b>> for FuzzPip<R> {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let tcrd = self.relation.resolve(backend, tcrd)?;
        let (tt, wt) = self.wire_to.resolve(backend, tcrd)?;
        let (tf, wf) = self.wire_from.resolve(backend, tcrd)?;
        assert_eq!(tt, tf);
        Some((fuzzer.fuzz(Key::Pip(tt, wf, wt), false, true), false))
    }
}
