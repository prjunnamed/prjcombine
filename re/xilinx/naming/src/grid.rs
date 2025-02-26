use std::collections::{BTreeSet, HashMap, hash_map};

use prjcombine_interconnect::{
    db::{BelId, NodeIriId, NodeKind, NodeKindId, NodeTileId, TermInfo, TermSlotId, WireKind},
    grid::{ColId, DieId, ExpandedGrid, IntWire, LayerId, NodeLoc, NodePip, RowId, TracePip},
};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::db::{
    NamingDb, NodeNamingId, NodeRawTileId, TermNamingId, TermWireInFarNaming, TermWireOutNaming,
};

#[derive(Clone, Debug)]
pub struct ExpandedGridNaming<'a> {
    pub db: &'a NamingDb,
    pub egrid: &'a ExpandedGrid<'a>,
    pub tie_kind: Option<String>,
    pub tie_pin_gnd: Option<String>,
    pub tie_pin_vcc: Option<String>,
    pub tie_pin_pullup: Option<String>,
    pub nodes: HashMap<NodeLoc, GridNodeNaming>,
    pub terms: HashMap<(DieId, ColId, RowId, TermSlotId), GridTermNaming>,
}

#[derive(Clone, Debug)]
pub struct GridNodeNaming {
    pub names: EntityPartVec<NodeRawTileId, String>,
    pub tie_name: Option<String>,
    pub tie_rt: NodeRawTileId,
    pub iri_names: EntityVec<NodeIriId, String>,
    pub naming: NodeNamingId,
    pub bels: EntityPartVec<BelId, String>,
}

impl GridNodeNaming {
    pub fn add_bel(&mut self, idx: usize, name: String) {
        self.bels.insert(BelId::from_idx(idx), name);
    }
}

#[derive(Clone, Debug)]
pub struct GridTermNaming {
    pub naming: TermNamingId,
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

impl<'a> ExpandedGridNaming<'a> {
    pub fn new(db: &'a NamingDb, egrid: &'a ExpandedGrid<'a>) -> Self {
        ExpandedGridNaming {
            db,
            egrid,
            tie_kind: None,
            tie_pin_gnd: None,
            tie_pin_vcc: None,
            tie_pin_pullup: None,
            nodes: HashMap::new(),
            terms: HashMap::new(),
        }
    }

    pub fn resolve_wire_raw(&self, mut wire: IntWire) -> Option<IntWire> {
        let die = self.egrid.die(wire.0);
        loop {
            let tile = &die[wire.1];
            let wi = self.egrid.db.wires[wire.2];
            match wi {
                WireKind::ClkOut => {
                    wire.1 = tile.clkroot;
                    break;
                }
                WireKind::MultiBranch(slot)
                | WireKind::Branch(slot)
                | WireKind::PipBranch(slot) => {
                    if let Some(t) = tile.terms.get(slot) {
                        let term = &self.egrid.db.terms[t.kind];
                        match term.wires.get(wire.2) {
                            Some(&TermInfo::BlackHole) => return None,
                            Some(&TermInfo::PassNear(wf)) => {
                                if let Some(naming) =
                                    self.terms.get(&(wire.0, wire.1.0, wire.1.1, slot))
                                {
                                    let n = &self.db.term_namings[naming.naming];
                                    if n.wires_out.contains_id(wire.2) {
                                        break;
                                    }
                                }
                                wire.2 = wf;
                            }
                            Some(&TermInfo::PassFar(wf)) => {
                                if let Some(naming) =
                                    self.terms.get(&(wire.0, wire.1.0, wire.1.1, slot))
                                {
                                    let n = &self.db.term_namings[naming.naming];
                                    if n.wires_out.contains_id(wire.2) {
                                        break;
                                    }
                                }
                                wire.1 = t.target.unwrap();
                                wire.2 = wf;
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
        if let Some(&twire) = self.egrid.xdie_wires.get_by_left(&wire) {
            wire = twire;
        }
        if self.egrid.blackhole_wires.contains(&wire) {
            None
        } else {
            Some(wire)
        }
    }

    pub fn resolve_wire_trace(&self, mut wire: IntWire) -> Option<(IntWire, Vec<TracePip>)> {
        let die = self.egrid.die(wire.0);
        let mut trace = vec![];
        loop {
            let tile = die.tile(wire.1);
            let wi = self.egrid.db.wires[wire.2];
            match wi {
                WireKind::ClkOut => {
                    wire.1 = tile.clkroot;
                    break;
                }
                WireKind::MultiBranch(slot)
                | WireKind::Branch(slot)
                | WireKind::PipBranch(slot) => {
                    if let Some(t) = tile.terms.get(slot) {
                        let term = &self.egrid.db.terms[t.kind];
                        match term.wires.get(wire.2) {
                            Some(&TermInfo::BlackHole) => return None,
                            Some(&TermInfo::PassNear(wf)) => {
                                if let Some(naming) =
                                    self.terms.get(&(wire.0, wire.1.0, wire.1.1, slot))
                                {
                                    let n = &self.db.term_namings[naming.naming];
                                    match n.wires_out.get(wire.2) {
                                        None => (),
                                        Some(TermWireOutNaming::Simple { name }) => {
                                            trace.push(TracePip {
                                                tile: &naming.tile,
                                                wire_to: name,
                                                wire_from: &n.wires_in_near[wf],
                                            });
                                        }
                                        Some(TermWireOutNaming::Buf { name_out, name_in }) => {
                                            trace.push(TracePip {
                                                tile: &naming.tile,
                                                wire_to: name_out,
                                                wire_from: name_in,
                                            });
                                        }
                                    }
                                }
                                wire.2 = wf;
                            }
                            Some(&TermInfo::PassFar(wf)) => {
                                if let Some(naming) =
                                    self.terms.get(&(wire.0, wire.1.0, wire.1.1, slot))
                                {
                                    let n = &self.db.term_namings[naming.naming];
                                    match n.wires_out.get(wire.2) {
                                        None => (),
                                        Some(TermWireOutNaming::Simple { name: name_fout }) => {
                                            match n.wires_in_far[wf] {
                                                TermWireInFarNaming::Simple { ref name } => {
                                                    trace.push(TracePip {
                                                        tile: &naming.tile,
                                                        wire_to: name_fout,
                                                        wire_from: name,
                                                    });
                                                }
                                                TermWireInFarNaming::Buf {
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
                                                TermWireInFarNaming::BufFar {
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
                                            }
                                        }
                                        Some(TermWireOutNaming::Buf { name_out, name_in }) => {
                                            trace.push(TracePip {
                                                tile: &naming.tile,
                                                wire_to: name_out,
                                                wire_from: name_in,
                                            });
                                        }
                                    }
                                }
                                wire.1 = t.target.unwrap();
                                wire.2 = wf;
                            }
                            None => break,
                        }
                    } else {
                        break;
                    }
                }
                WireKind::Buf(wf) => {
                    let naming = &self.nodes[&(wire.0, wire.1.0, wire.1.1, LayerId::from_idx(0))];
                    let nn = &self.db.node_namings[naming.naming];
                    trace.push(TracePip {
                        tile: &naming.names[NodeRawTileId::from_idx(0)],
                        wire_to: &nn.wires[&(NodeTileId::from_idx(0), wire.2)],
                        wire_from: &nn.wires[&(NodeTileId::from_idx(0), wf)],
                    });
                    wire.2 = wf;
                }
                _ => break,
            }
        }
        if let Some(&twire) = self.egrid.xdie_wires.get_by_left(&wire) {
            wire = twire;
        }
        if self.egrid.blackhole_wires.contains(&wire) {
            None
        } else {
            Some((wire, trace))
        }
    }

    pub fn get_node_pip_naming(&self, np: NodePip) -> TracePip {
        let node = &self.nodes[&(np.node_die, np.node_crd.0, np.node_crd.1, np.node_layer)];
        let naming = &self.db.node_namings[node.naming];
        if let Some(pn) = naming.ext_pips.get(&(np.node_wire_out, np.node_wire_in)) {
            TracePip {
                tile: &node.names[pn.tile],
                wire_to: &pn.wire_to,
                wire_from: &pn.wire_from,
            }
        } else {
            TracePip {
                tile: &node.names[NodeRawTileId::from_idx(0)],
                wire_to: &naming.wires[&np.node_wire_out],
                wire_from: &naming.wires[&np.node_wire_in],
            }
        }
    }

    pub fn name_node(
        &mut self,
        nloc: NodeLoc,
        naming: &str,
        names: impl IntoIterator<Item = String>,
    ) -> &mut GridNodeNaming {
        let nnode = GridNodeNaming {
            names: names
                .into_iter()
                .enumerate()
                .map(|(k, v)| (NodeRawTileId::from_idx(k), v))
                .collect(),
            tie_name: None,
            tie_rt: NodeRawTileId::from_idx(0),
            iri_names: Default::default(),
            naming: self.db.get_node_naming(naming),
            bels: EntityPartVec::new(),
        };
        let hash_map::Entry::Vacant(entry) = self.nodes.entry(nloc) else {
            unreachable!()
        };
        entry.insert(nnode)
    }

    pub fn name_term_tile(
        &mut self,
        tloc: (DieId, ColId, RowId, TermSlotId),
        naming: &str,
        name: String,
    ) {
        let nterm = GridTermNaming {
            naming: self.db.get_term_naming(naming),
            tile: name,
            tile_far: None,
        };
        let hash_map::Entry::Vacant(entry) = self.terms.entry(tloc) else {
            unreachable!()
        };
        entry.insert(nterm);
    }

    pub fn name_term_pair(
        &mut self,
        tloc: (DieId, ColId, RowId, TermSlotId),
        naming: &str,
        name: String,
        name_far: String,
    ) {
        let nterm = GridTermNaming {
            naming: self.db.get_term_naming(naming),
            tile: name,
            tile_far: Some(name_far),
        };
        let hash_map::Entry::Vacant(entry) = self.terms.entry(tloc) else {
            unreachable!()
        };
        entry.insert(nterm);
    }

    pub fn bel_grid(&self, f: impl Fn(NodeKindId, &str, &NodeKind) -> bool) -> BelGrid {
        assert_eq!(self.egrid.die.len(), 1);
        let mut cols = BTreeSet::new();
        let mut rows = BTreeSet::new();
        for (kind, name, node) in &self.egrid.db.nodes {
            if f(kind, name, node) {
                for &nloc in &self.egrid.node_index[kind] {
                    cols.insert(nloc.1);
                    rows.insert(nloc.2);
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

    pub fn bel_multi_grid(&self, f: impl Fn(NodeKindId, &str, &NodeKind) -> bool) -> BelMultiGrid {
        let mut cols = BTreeSet::new();
        let mut rows = BTreeSet::new();
        for (kind, name, node) in &self.egrid.db.nodes {
            if f(kind, name, node) {
                for &nloc in &self.egrid.node_index[kind] {
                    cols.insert(nloc.1);
                    rows.insert((nloc.0, nloc.2));
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

    pub fn get_bel_name(&self, die: DieId, col: ColId, row: RowId, key: &str) -> Option<&str> {
        if let Some((layer, _, bel, _)) = self.egrid.find_bel(die, (col, row), key) {
            let nnode = &self.nodes[&(die, col, row, layer)];
            Some(&nnode.bels[bel])
        } else {
            None
        }
    }
}
