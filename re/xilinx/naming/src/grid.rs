use std::collections::{BTreeSet, HashMap, hash_map};

use prjcombine_interconnect::{
    db::{
        BelSlotId, CellSlotId, ConnectorWire, TileClass, TileClassId, TileIriId, TileWireCoord,
        WireKind,
    },
    grid::{
        BelCoord, ColId, ConnectorCoord, DieId, ExpandedGrid, RowId, TileCoord, TilePip, WireCoord,
    },
};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::db::{
    ConnectorClassNamingId, ConnectorWireInFarNaming, ConnectorWireOutNaming, NamingDb, RawTileId,
    TileClassNamingId,
};

#[derive(Clone, Debug)]
pub struct ExpandedGridNaming<'a> {
    pub db: &'a NamingDb,
    pub egrid: &'a ExpandedGrid<'a>,
    pub tie_kind: Option<String>,
    pub tie_pin_gnd: Option<String>,
    pub tie_pin_vcc: Option<String>,
    pub tie_pin_pullup: Option<String>,
    pub tiles: HashMap<TileCoord, TileNaming>,
    pub conns: HashMap<ConnectorCoord, ConnectorNaming>,
}

#[derive(Clone, Debug)]
pub struct TileNaming {
    pub names: EntityPartVec<RawTileId, String>,
    pub tie_name: Option<String>,
    pub tie_rt: RawTileId,
    pub iri_names: EntityVec<TileIriId, String>,
    pub naming: TileClassNamingId,
    pub bels: EntityPartVec<BelSlotId, String>,
}

impl TileNaming {
    pub fn add_bel(&mut self, slot: BelSlotId, name: String) {
        self.bels.insert(slot, name);
    }
}

#[derive(Clone, Debug)]
pub struct ConnectorNaming {
    pub naming: ConnectorClassNamingId,
    pub tile: String,
    pub tile_far: Option<String>,
}

#[derive(Clone, Debug)]
pub struct BelGrid {
    pub xlut: EntityPartVec<ColId, usize>,
    pub ylut: EntityPartVec<RowId, usize>,
}

#[derive(Clone, Debug)]
pub struct BelMultiGrid {
    pub xlut: EntityPartVec<ColId, usize>,
    pub ylut: EntityVec<DieId, EntityPartVec<RowId, usize>>,
}

#[derive(Copy, Clone, Debug)]
pub struct TracePip<'a> {
    pub tile: &'a str,
    pub wire_to: &'a str,
    pub wire_from: &'a str,
}

impl<'a> ExpandedGridNaming<'a> {
    pub fn new(db: &'a NamingDb, egrid: &'a ExpandedGrid<'a>) -> Self {
        ExpandedGridNaming {
            db,
            egrid,
            tie_kind: None,
            tie_pin_gnd: None,
            tie_pin_vcc: None,
            tie_pin_pullup: None,
            tiles: HashMap::new(),
            conns: HashMap::new(),
        }
    }

    pub fn resolve_wire_raw(&self, mut wire: WireCoord) -> Option<WireCoord> {
        loop {
            let cell = self.egrid.cell(wire.cell);
            let wi = self.egrid.db.wires[wire.slot];
            match wi {
                WireKind::Regional(rslot) => {
                    (wire.cell.col, wire.cell.row) = cell.region_root[rslot];
                    break;
                }
                WireKind::MultiBranch(slot)
                | WireKind::Branch(slot)
                | WireKind::PipBranch(slot) => {
                    if let Some(t) = cell.conns.get(slot) {
                        let ccls = &self.egrid.db.conn_classes[t.class];
                        match ccls.wires.get(wire.slot) {
                            Some(&ConnectorWire::BlackHole) => return None,
                            Some(&ConnectorWire::Reflect(wf)) => {
                                if let Some(naming) = self.conns.get(&wire.cell.connector(slot)) {
                                    let n = &self.db.conn_class_namings[naming.naming];
                                    if n.wires_out.contains_id(wire.slot) {
                                        break;
                                    }
                                }
                                wire.slot = wf;
                            }
                            Some(&ConnectorWire::Pass(wf)) => {
                                if let Some(naming) = self.conns.get(&wire.cell.connector(slot)) {
                                    let n = &self.db.conn_class_namings[naming.naming];
                                    if n.wires_out.contains_id(wire.slot) {
                                        break;
                                    }
                                }
                                (wire.cell.col, wire.cell.row) = t.target.unwrap();
                                wire.slot = wf;
                            }
                            None => break,
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        if let Some(&twire) = self.egrid.extra_conns.get_by_left(&wire) {
            wire = twire;
        }
        if self.egrid.blackhole_wires.contains(&wire) {
            None
        } else {
            Some(wire)
        }
    }

    pub fn resolve_wire_trace(
        &self,
        mut wire: WireCoord,
    ) -> Option<(WireCoord, Vec<TracePip<'_>>)> {
        let mut trace = vec![];
        loop {
            let tile = self.egrid.cell(wire.cell);
            let wi = self.egrid.db.wires[wire.slot];
            match wi {
                WireKind::Regional(rslot) => {
                    (wire.cell.col, wire.cell.row) = tile.region_root[rslot];
                    break;
                }
                WireKind::MultiBranch(slot)
                | WireKind::Branch(slot)
                | WireKind::PipBranch(slot) => {
                    if let Some(t) = tile.conns.get(slot) {
                        let term = &self.egrid.db.conn_classes[t.class];
                        match term.wires.get(wire.slot) {
                            Some(&ConnectorWire::BlackHole) => return None,
                            Some(&ConnectorWire::Reflect(wf)) => {
                                if let Some(naming) = self.conns.get(&wire.cell.connector(slot)) {
                                    let n = &self.db.conn_class_namings[naming.naming];
                                    match n.wires_out.get(wire.slot) {
                                        None => (),
                                        Some(ConnectorWireOutNaming::Simple { name }) => {
                                            trace.push(TracePip {
                                                tile: &naming.tile,
                                                wire_to: name,
                                                wire_from: &n.wires_in_near[wf],
                                            });
                                        }
                                        Some(ConnectorWireOutNaming::Buf { name_out, name_in }) => {
                                            trace.push(TracePip {
                                                tile: &naming.tile,
                                                wire_to: name_out,
                                                wire_from: name_in,
                                            });
                                        }
                                    }
                                }
                                wire.slot = wf;
                            }
                            Some(&ConnectorWire::Pass(wf)) => {
                                if let Some(naming) = self.conns.get(&wire.cell.connector(slot)) {
                                    let n = &self.db.conn_class_namings[naming.naming];
                                    match n.wires_out.get(wire.slot) {
                                        None => (),
                                        Some(ConnectorWireOutNaming::Simple {
                                            name: name_fout,
                                        }) => match n.wires_in_far[wf] {
                                            ConnectorWireInFarNaming::Simple { ref name } => {
                                                trace.push(TracePip {
                                                    tile: &naming.tile,
                                                    wire_to: name_fout,
                                                    wire_from: name,
                                                });
                                            }
                                            ConnectorWireInFarNaming::Buf {
                                                ref name_out,
                                                ref name_in,
                                            } => {
                                                trace.push(TracePip {
                                                    tile: &naming.tile,
                                                    wire_to: name_fout,
                                                    wire_from: name_out,
                                                });
                                                trace.push(TracePip {
                                                    tile: &naming.tile,
                                                    wire_to: name_out,
                                                    wire_from: name_in,
                                                });
                                            }
                                            ConnectorWireInFarNaming::BufFar {
                                                ref name,
                                                ref name_far_out,
                                                ref name_far_in,
                                            } => {
                                                trace.push(TracePip {
                                                    tile: &naming.tile,
                                                    wire_to: name_fout,
                                                    wire_from: name,
                                                });
                                                trace.push(TracePip {
                                                    tile: naming.tile_far.as_ref().unwrap(),
                                                    wire_to: name_far_out,
                                                    wire_from: name_far_in,
                                                });
                                            }
                                        },
                                        Some(ConnectorWireOutNaming::Buf { name_out, name_in }) => {
                                            trace.push(TracePip {
                                                tile: &naming.tile,
                                                wire_to: name_out,
                                                wire_from: name_in,
                                            });
                                        }
                                    }
                                }
                                (wire.cell.col, wire.cell.row) = t.target.unwrap();
                                wire.slot = wf;
                            }
                            None => break,
                        }
                    } else {
                        break;
                    }
                }
                WireKind::Buf(wf) => {
                    let slot = self
                        .egrid
                        .db
                        .tile_slots
                        .get("INT")
                        .unwrap_or(self.egrid.db.tile_slots.get("MAIN").unwrap());
                    let naming = &self.tiles[&wire.cell.tile(slot)];
                    let nn = &self.db.tile_class_namings[naming.naming];
                    trace.push(TracePip {
                        tile: &naming.names[RawTileId::from_idx(0)],
                        wire_to: &nn.wires[&TileWireCoord {
                            cell: CellSlotId::from_idx(0),
                            wire: wire.slot,
                        }],
                        wire_from: &nn.wires[&TileWireCoord {
                            cell: CellSlotId::from_idx(0),
                            wire: wf,
                        }],
                    });
                    wire.slot = wf;
                }
                _ => break,
            }
        }
        if let Some(&twire) = self.egrid.extra_conns.get_by_left(&wire) {
            wire = twire;
        }
        if self.egrid.blackhole_wires.contains(&wire) {
            None
        } else {
            Some((wire, trace))
        }
    }

    pub fn get_tile_pip_naming(&self, np: TilePip) -> TracePip<'_> {
        let tile = &self.tiles[&np.tile];
        let naming = &self.db.tile_class_namings[tile.naming];
        if let Some(pn) = naming.ext_pips.get(&(np.tile_wire_out, np.tile_wire_in)) {
            TracePip {
                tile: &tile.names[pn.tile],
                wire_to: &pn.wire_to,
                wire_from: &pn.wire_from,
            }
        } else {
            TracePip {
                tile: &tile.names[RawTileId::from_idx(0)],
                wire_to: &naming.wires[&np.tile_wire_out],
                wire_from: &naming.wires[&np.tile_wire_in],
            }
        }
    }

    pub fn name_tile(
        &mut self,
        nloc: TileCoord,
        naming: &str,
        names: impl IntoIterator<Item = String>,
    ) -> &mut TileNaming {
        let ntile = TileNaming {
            names: names
                .into_iter()
                .enumerate()
                .map(|(k, v)| (RawTileId::from_idx(k), v))
                .collect(),
            tie_name: None,
            tie_rt: RawTileId::from_idx(0),
            iri_names: Default::default(),
            naming: self.db.get_tile_class_naming(naming),
            bels: EntityPartVec::new(),
        };
        let hash_map::Entry::Vacant(entry) = self.tiles.entry(nloc) else {
            unreachable!()
        };
        entry.insert(ntile)
    }

    pub fn name_conn_tile(&mut self, ccrd: ConnectorCoord, naming: &str, name: String) {
        let nconn = ConnectorNaming {
            naming: self.db.get_conn_class_naming(naming),
            tile: name,
            tile_far: None,
        };
        let hash_map::Entry::Vacant(entry) = self.conns.entry(ccrd) else {
            unreachable!()
        };
        entry.insert(nconn);
    }

    pub fn name_conn_pair(
        &mut self,
        ccrd: ConnectorCoord,
        naming: &str,
        name: String,
        name_far: String,
    ) {
        let nconn = ConnectorNaming {
            naming: self.db.get_conn_class_naming(naming),
            tile: name,
            tile_far: Some(name_far),
        };
        let hash_map::Entry::Vacant(entry) = self.conns.entry(ccrd) else {
            unreachable!()
        };
        entry.insert(nconn);
    }

    pub fn bel_grid(&self, f: impl Fn(TileClassId, &str, &TileClass) -> bool) -> BelGrid {
        assert_eq!(self.egrid.die.len(), 1);
        let mut cols = BTreeSet::new();
        let mut rows = BTreeSet::new();
        for (kind, name, tcls) in &self.egrid.db.tile_classes {
            if f(kind, name, tcls) {
                for &tcrd in &self.egrid.tile_index[kind] {
                    cols.insert(tcrd.col);
                    rows.insert(tcrd.row);
                }
            }
        }
        let mut xlut = EntityPartVec::new();
        let mut ylut = EntityPartVec::new();
        for (i, col) in cols.into_iter().enumerate() {
            xlut.insert(col, i);
        }
        for (i, row) in rows.into_iter().enumerate() {
            ylut.insert(row, i);
        }
        BelGrid { xlut, ylut }
    }

    pub fn bel_multi_grid(
        &self,
        f: impl Fn(TileClassId, &str, &TileClass) -> bool,
    ) -> BelMultiGrid {
        let mut cols = BTreeSet::new();
        let mut rows = BTreeSet::new();
        for (kind, name, tcls) in &self.egrid.db.tile_classes {
            if f(kind, name, tcls) {
                for &tcrd in &self.egrid.tile_index[kind] {
                    cols.insert(tcrd.col);
                    rows.insert((tcrd.die, tcrd.row));
                }
            }
        }
        let mut xlut = EntityPartVec::new();
        let mut ylut: EntityVec<_, _> =
            self.egrid.die.ids().map(|_| EntityPartVec::new()).collect();
        for (i, col) in cols.into_iter().enumerate() {
            xlut.insert(col, i);
        }
        for (i, (die, row)) in rows.into_iter().enumerate() {
            ylut[die].insert(row, i);
        }
        BelMultiGrid { xlut, ylut }
    }

    pub fn get_bel_name(&self, bel: BelCoord) -> Option<&str> {
        if let Some(loc) = self.egrid.find_tile_by_bel(bel) {
            let ntile = &self.tiles[&loc];
            Some(&ntile.bels[bel.slot])
        } else {
            None
        }
    }
}
