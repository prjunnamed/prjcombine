use std::collections::{BTreeMap, BTreeSet, HashMap, btree_map};

use prjcombine_entity::{EntityBundleIndex, EntityVec};
use prjcombine_interconnect::{
    db::{
        BelInfo, BelInput, BelKind, BelSlotId, CellSlotId, IntDb, TileClass, TileClassId,
        TileWireCoord,
    },
    grid::{BelCoord, CellCoord},
};
use prjcombine_siliconblue::chip::{SpecialIoKey, SpecialTile};

use crate::{run::InstPin, sites::BelPins};

pub struct MiscTileBuilder<'a> {
    pub intdb: &'a IntDb,
    pub tcls: TileClass,
    pub io: BTreeMap<SpecialIoKey, BelCoord>,
    pub cells: EntityVec<CellSlotId, CellCoord>,
    pub cells_map: HashMap<CellCoord, CellSlotId>,
}

impl<'a> MiscTileBuilder<'a> {
    pub fn new(intdb: &'a IntDb, tcid: TileClassId, cells: &[CellCoord]) -> Self {
        let cells = EntityVec::from_iter(cells.iter().copied());
        let mut cells_map = HashMap::new();
        for (cell, &crd) in &cells {
            cells_map.insert(crd, cell);
        }
        let tcls = intdb.tile_classes[tcid].clone();
        assert_eq!(cells.len(), tcls.cells.len());
        Self {
            intdb,
            tcls,
            io: Default::default(),
            cells,
            cells_map,
        }
    }

    pub fn get_cell(&mut self, crd: CellCoord) -> CellSlotId {
        self.cells_map[&crd]
    }

    pub fn add_bel(&mut self, slot: BelSlotId, pins: &BelPins) {
        let BelKind::Class(bcid) = self.intdb.bel_slots[slot].kind else {
            unreachable!()
        };
        let bcls = &self.intdb.bel_classes[bcid];
        let BelInfo::Bel(mut bel) = self.tcls.bels[slot].clone() else {
            unreachable!()
        };
        for (pin, &wire) in &pins.ins {
            let cell = self.get_cell(wire.cell);
            let pid = match pin {
                InstPin::Simple(pname) => {
                    if let Some((bidx, _)) = bcls.inputs.get(pname) {
                        match bidx {
                            EntityBundleIndex::Single(pid) => pid,
                            EntityBundleIndex::Array(_) => unreachable!(),
                        }
                    } else {
                        let idx: usize = pname[pname.len() - 1..].parse().unwrap();
                        let pname = &pname[..pname.len() - 1];
                        let bidx = bcls.inputs.get(pname).unwrap().0;
                        match bidx {
                            EntityBundleIndex::Single(_) => unreachable!(),
                            EntityBundleIndex::Array(range) => range.index(idx),
                        }
                    }
                }
                InstPin::Indexed(pname, idx) => {
                    let bidx = bcls.inputs.get(pname).unwrap().0;
                    match bidx {
                        EntityBundleIndex::Single(_) => unreachable!(),
                        EntityBundleIndex::Array(range) => range.index(*idx),
                    }
                }
            };
            let inp = BelInput::Fixed(
                TileWireCoord {
                    cell,
                    wire: wire.slot,
                }
                .pos(),
            );
            if bel.inputs.contains_id(pid) {
                assert_eq!(bel.inputs[pid], inp);
            } else {
                bel.inputs.insert(pid, inp);
            }
        }
        for (pin, iwires) in &pins.outs {
            let mut wires = BTreeSet::new();
            let pid = match pin {
                InstPin::Simple(pname) => {
                    if let Some((bidx, _)) = bcls.outputs.get(pname) {
                        match bidx {
                            EntityBundleIndex::Single(pid) => pid,
                            EntityBundleIndex::Array(_) => unreachable!(),
                        }
                    } else {
                        let idx: usize = pname[pname.len() - 1..].parse().unwrap();
                        let pname = &pname[..pname.len() - 1];
                        let bidx = bcls.outputs.get(pname).unwrap().0;
                        match bidx {
                            EntityBundleIndex::Single(_) => unreachable!(),
                            EntityBundleIndex::Array(range) => range.index(idx),
                        }
                    }
                }
                InstPin::Indexed(pname, idx) => {
                    let bidx = bcls.outputs.get(pname).unwrap().0;
                    match bidx {
                        EntityBundleIndex::Single(_) => unreachable!(),
                        EntityBundleIndex::Array(range) => range.index(*idx),
                    }
                }
            };
            for &wire in iwires {
                let cell = self.get_cell(wire.cell);
                wires.insert(TileWireCoord {
                    cell,
                    wire: wire.slot,
                });
            }
            if bel.outputs.contains_id(pid) {
                assert_eq!(bel.outputs[pid], wires);
            } else {
                bel.outputs.insert(pid, wires);
            }
        }
        self.tcls.bels.insert(slot, BelInfo::Bel(bel));
    }

    pub fn insert_io(&mut self, key: SpecialIoKey, io: BelCoord) {
        match self.io.entry(key) {
            btree_map::Entry::Vacant(e) => {
                e.insert(io);
            }
            btree_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), io);
            }
        }
    }

    pub fn finish(self) -> (TileClass, SpecialTile) {
        (
            self.tcls,
            SpecialTile {
                io: self.io,
                cells: self.cells,
            },
        )
    }
}
