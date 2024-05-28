#![allow(clippy::too_many_arguments)]

use crate::db::*;
use bimap::BiHashMap;
use enum_map::EnumMap;
use ndarray::Array2;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
};
use unnamed_entity::{entity_id, EntityId, EntityIds, EntityPartVec, EntityVec};

entity_id! {
    pub id DieId u8, reserve 1, delta;
    pub id ColId u16, reserve 1, delta;
    pub id RowId u16, reserve 1, delta;
    pub id LayerId u8;
}

pub type Coord = (ColId, RowId);
pub type IntWire = (DieId, Coord, WireId);

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Rect {
    pub col_l: ColId,
    pub col_r: ColId,
    pub row_b: RowId,
    pub row_t: RowId,
}

impl Rect {
    pub fn contains(&self, col: ColId, row: RowId) -> bool {
        col >= self.col_l && col < self.col_r && row >= self.row_b && row < self.row_t
    }
}

#[derive(Clone, Debug)]
pub struct ExpandedGrid<'a> {
    pub db: &'a IntDb,
    pub db_index: IntDbIndex,
    pub tie_kind: Option<String>,
    pub tie_pin_gnd: Option<String>,
    pub tie_pin_vcc: Option<String>,
    pub tie_pin_pullup: Option<String>,
    pub die: EntityVec<DieId, ExpandedDie>,
    pub xdie_wires: BiHashMap<IntWire, IntWire>,
    pub blackhole_wires: HashSet<IntWire>,
    pub node_index: EntityVec<NodeKindId, Vec<(DieId, ColId, RowId, LayerId)>>,
}

#[derive(Clone, Debug)]
pub struct ExpandedDie {
    tiles: Array2<ExpandedTile>,
    pub clk_root_tiles: HashMap<Coord, HashSet<Coord>>,
}

pub struct ExpandedDieRef<'a, 'b> {
    pub grid: &'b ExpandedGrid<'a>,
    pub die: DieId,
}

pub struct ExpandedDieRefMut<'a, 'b> {
    pub grid: &'b mut ExpandedGrid<'a>,
    pub die: DieId,
}

impl<'a> ExpandedGrid<'a> {
    pub fn new(db: &'a IntDb) -> Self {
        ExpandedGrid {
            db,
            db_index: IntDbIndex::new(db),
            tie_kind: None,
            tie_pin_gnd: None,
            tie_pin_vcc: None,
            tie_pin_pullup: None,
            die: EntityVec::new(),
            xdie_wires: BiHashMap::new(),
            blackhole_wires: HashSet::new(),
            node_index: db.nodes.ids().map(|_| vec![]).collect(),
        }
    }

    pub fn add_die<'b>(
        &'b mut self,
        width: usize,
        height: usize,
    ) -> (DieId, ExpandedDieRefMut<'a, 'b>) {
        let dieid = self.die.push(ExpandedDie {
            tiles: Array2::from_shape_fn([height, width], |(r, c)| ExpandedTile {
                nodes: Default::default(),
                terms: Default::default(),
                node_index: vec![],
                clkroot: (ColId::from_idx(c), RowId::from_idx(r)),
            }),
            clk_root_tiles: HashMap::new(),
        });
        (dieid, self.die_mut(dieid))
    }

    pub fn dies<'b>(&'b self) -> impl Iterator<Item = ExpandedDieRef<'a, 'b>> {
        self.die.ids().map(|die| self.die(die))
    }

    pub fn die<'b>(&'b self, die: DieId) -> ExpandedDieRef<'a, 'b> {
        ExpandedDieRef { grid: self, die }
    }
    pub fn die_mut<'b>(&'b mut self, die: DieId) -> ExpandedDieRefMut<'a, 'b> {
        ExpandedDieRefMut { grid: self, die }
    }

    pub fn node(&self, loc: (DieId, ColId, RowId, LayerId)) -> &ExpandedTileNode {
        &self.die(loc.0).tile((loc.1, loc.2)).nodes[loc.3]
    }

    pub fn find_node(
        &self,
        die: DieId,
        coord: Coord,
        f: impl Fn(&ExpandedTileNode) -> bool,
    ) -> Option<&ExpandedTileNode> {
        let die = self.die(die);
        let tile = die.tile(coord);
        tile.nodes.values().find(|x| f(x))
    }

    pub fn find_bel(
        &self,
        die: DieId,
        coord: Coord,
        key: &str,
    ) -> Option<(&ExpandedTileNode, BelId, &BelInfo, &BelNaming)> {
        let die = self.die(die);
        let tile = die.tile(coord);
        for node in tile.nodes.values() {
            let nk = &self.db.nodes[node.kind];
            let naming = &self.db.node_namings[node.naming];
            if let Some((id, bel)) = nk.bels.get(key) {
                return Some((node, id, bel, &naming.bels[id]));
            }
        }
        None
    }

    pub fn finish(&mut self) {
        for (dieid, die) in &mut self.die {
            let mut clk_root_tiles: HashMap<_, HashSet<_>> = HashMap::new();
            for col in die.cols() {
                for row in die.rows() {
                    clk_root_tiles
                        .entry(die[(col, row)].clkroot)
                        .or_default()
                        .insert((col, row));
                    for (layer, node) in &die[(col, row)].nodes {
                        self.node_index[node.kind].push((dieid, col, row, layer));
                    }
                }
            }
            die.clk_root_tiles = clk_root_tiles;
        }
        #[allow(unexpected_cfgs)]
        if cfg!(self_check_egrid) {
            println!("CHECK");
            for die in self.dies() {
                for col in die.cols() {
                    for row in die.rows() {
                        for w in self.db.wires.ids() {
                            let wire = (die.die, (col, row), w);
                            let Some(rw) = self.resolve_wire(wire) else {
                                continue;
                            };
                            let tree = self.wire_tree(rw);
                            if rw == wire {
                                for &ow in &tree {
                                    assert_eq!(Some(wire), self.resolve_wire(ow));
                                }
                            }
                            if !tree.contains(&wire) {
                                panic!(
                                    "tree {rd}.{rc}.{rr}.{rw} does not contain {od}.{oc}.{or}.{ow}",
                                    rd = rw.0.to_idx(),
                                    rc = rw.1 .0.to_idx(),
                                    rr = rw.1 .1.to_idx(),
                                    rw = self.db.wires.key(rw.2),
                                    od = wire.0.to_idx(),
                                    oc = wire.1 .0.to_idx(),
                                    or = wire.1 .1.to_idx(),
                                    ow = self.db.wires.key(wire.2),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

impl core::ops::Index<Coord> for ExpandedDie {
    type Output = ExpandedTile;
    fn index(&self, xy: Coord) -> &ExpandedTile {
        &self.tiles[[xy.1.to_idx(), xy.0.to_idx()]]
    }
}

impl core::ops::IndexMut<Coord> for ExpandedDie {
    fn index_mut(&mut self, xy: Coord) -> &mut ExpandedTile {
        &mut self.tiles[[xy.1.to_idx(), xy.0.to_idx()]]
    }
}

impl Deref for ExpandedDieRef<'_, '_> {
    type Target = ExpandedDie;
    fn deref(&self) -> &Self::Target {
        &self.grid.die[self.die]
    }
}

impl Deref for ExpandedDieRefMut<'_, '_> {
    type Target = ExpandedDie;
    fn deref(&self) -> &Self::Target {
        &self.grid.die[self.die]
    }
}

impl DerefMut for ExpandedDieRefMut<'_, '_> {
    fn deref_mut(&mut self) -> &mut ExpandedDie {
        &mut self.grid.die[self.die]
    }
}

impl ExpandedDie {
    pub fn rows(&self) -> EntityIds<RowId> {
        EntityIds::new(self.tiles.shape()[0])
    }

    pub fn cols(&self) -> EntityIds<ColId> {
        EntityIds::new(self.tiles.shape()[1])
    }
}

impl<'a> ExpandedDieRef<'_, 'a> {
    pub fn tile(&self, xy: Coord) -> &'a ExpandedTile {
        &self.grid.die[self.die].tiles[[xy.1.to_idx(), xy.0.to_idx()]]
    }
}

impl ExpandedDieRefMut<'_, '_> {
    pub fn add_xnode(
        &mut self,
        crd: Coord,
        kind: NodeKindId,
        names: &[&str],
        naming: NodeNamingId,
        coords: &[Coord],
    ) -> &mut ExpandedTileNode {
        let names: EntityVec<_, _> = names.iter().map(|x| x.to_string()).collect();
        let names = names.into_iter().collect();
        let tiles: EntityVec<_, _> = coords.iter().copied().collect();
        let layer = self[crd].nodes.push(ExpandedTileNode {
            kind,
            tiles: tiles.clone(),
            names,
            tie_name: None,
            tie_rt: NodeRawTileId::from_idx(0),
            iri_names: Default::default(),
            naming,
            bels: Default::default(),
        });
        for (tid, tcrd) in tiles {
            self[tcrd].node_index.push((crd, layer, tid))
        }
        &mut self[crd].nodes[layer]
    }

    pub fn fill_tile(
        &mut self,
        xy: Coord,
        kind: &str,
        naming: &str,
        name: String,
    ) -> &mut ExpandedTileNode {
        assert!(self[xy].nodes.is_empty());
        let kind = self.grid.db.get_node(kind);
        let naming = self.grid.db.get_node_naming(naming);
        self.add_xnode(xy, kind, &[&name], naming, &[xy])
    }

    pub fn fill_term_pair(&mut self, fwd: ExpandedTileTerm, bwd: ExpandedTileTerm) {
        let a = bwd.target.unwrap();
        let b = fwd.target.unwrap();
        let dir = self.grid.db.terms[fwd.kind].dir;
        assert_eq!(self.grid.db.terms[bwd.kind].dir, !dir);
        match dir {
            Dir::W => {
                assert_eq!(a.1, b.1);
                assert!(a.0 > b.0);
            }
            Dir::E => {
                assert_eq!(a.1, b.1);
                assert!(a.0 < b.0);
            }
            Dir::S => {
                assert_eq!(a.0, b.0);
                assert!(a.1 > b.1);
            }
            Dir::N => {
                assert_eq!(a.0, b.0);
                assert!(a.1 < b.1);
            }
        }
        self[a].terms[dir] = Some(fwd);
        self[b].terms[!dir] = Some(bwd);
    }

    pub fn fill_term_pair_anon(&mut self, a: Coord, b: Coord, fwd: TermKindId, bwd: TermKindId) {
        self.fill_term_pair(
            ExpandedTileTerm {
                target: Some(b),
                kind: fwd,
                tile: None,
                tile_far: None,
                naming: None,
            },
            ExpandedTileTerm {
                target: Some(a),
                kind: bwd,
                tile: None,
                tile_far: None,
                naming: None,
            },
        );
    }

    pub fn fill_term_pair_buf(
        &mut self,
        a: Coord,
        b: Coord,
        fwd: TermKindId,
        bwd: TermKindId,
        tile: String,
        naming_a: TermNamingId,
        naming_b: TermNamingId,
    ) {
        self.fill_term_pair(
            ExpandedTileTerm {
                target: Some(b),
                kind: fwd,
                tile: Some(tile.clone()),
                tile_far: None,
                naming: Some(naming_a),
            },
            ExpandedTileTerm {
                target: Some(a),
                kind: bwd,
                tile: Some(tile),
                tile_far: None,
                naming: Some(naming_b),
            },
        );
    }

    pub fn fill_term_pair_bounce(
        &mut self,
        a: Coord,
        b: Coord,
        fwd: TermKindId,
        bwd: TermKindId,
        tile_a: String,
        tile_b: String,
        naming_a: TermNamingId,
        naming_b: TermNamingId,
    ) {
        self.fill_term_pair(
            ExpandedTileTerm {
                target: Some(b),
                kind: fwd,
                tile: Some(tile_a),
                tile_far: None,
                naming: Some(naming_a),
            },
            ExpandedTileTerm {
                target: Some(a),
                kind: bwd,
                tile: Some(tile_b),
                tile_far: None,
                naming: Some(naming_b),
            },
        );
    }

    pub fn fill_term_pair_dbuf(
        &mut self,
        a: Coord,
        b: Coord,
        fwd: TermKindId,
        bwd: TermKindId,
        tile_a: String,
        tile_b: String,
        naming_a: TermNamingId,
        naming_b: TermNamingId,
    ) {
        self.fill_term_pair(
            ExpandedTileTerm {
                target: Some(b),
                kind: fwd,
                tile: Some(tile_a.clone()),
                tile_far: Some(tile_b.clone()),
                naming: Some(naming_a),
            },
            ExpandedTileTerm {
                target: Some(a),
                kind: bwd,
                tile: Some(tile_b),
                tile_far: Some(tile_a),
                naming: Some(naming_b),
            },
        );
    }

    pub fn fill_term_tile(&mut self, xy: Coord, kind: &str, naming: &str, tile: String) {
        let kind = self.grid.db.get_term(kind);
        let naming = self.grid.db.get_term_naming(naming);
        let dir = self.grid.db.terms[kind].dir;
        self[xy].terms[dir] = Some(ExpandedTileTerm {
            target: None,
            kind,
            tile: Some(tile),
            tile_far: None,
            naming: Some(naming),
        });
    }

    pub fn fill_term_anon(&mut self, xy: Coord, kind: &str) {
        let kind = self.grid.db.get_term(kind);
        let dir = self.grid.db.terms[kind].dir;
        self[xy].terms[dir] = Some(ExpandedTileTerm {
            target: None,
            kind,
            tile: None,
            tile_far: None,
            naming: None,
        });
    }

    pub fn fill_main_passes(&mut self) {
        let pass_w = self.grid.db.get_term("MAIN.W");
        let pass_e = self.grid.db.get_term("MAIN.E");
        let pass_s = self.grid.db.get_term("MAIN.S");
        let pass_n = self.grid.db.get_term("MAIN.N");
        // horizontal
        for row in self.rows() {
            let mut prev = None;
            for col in self.cols() {
                if self[(col, row)].nodes.is_empty() {
                    continue;
                }
                if let Some(prev) = prev {
                    if self[(col, row)].terms[Dir::W].is_none() {
                        self.fill_term_pair_anon((prev, row), (col, row), pass_e, pass_w);
                    }
                }
                if self[(col, row)].terms[Dir::E].is_none() {
                    prev = Some(col);
                } else {
                    prev = None;
                }
            }
        }
        // vertical
        for col in self.cols() {
            let mut prev = None;
            for row in self.rows() {
                if self[(col, row)].nodes.is_empty() {
                    continue;
                }
                if let Some(prev) = prev {
                    if self[(col, row)].terms[Dir::S].is_none() {
                        self.fill_term_pair_anon((col, prev), (col, row), pass_n, pass_s);
                    }
                }
                if self[(col, row)].terms[Dir::N].is_none() {
                    prev = Some(row);
                } else {
                    prev = None;
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TracePip<'a> {
    pub tile: &'a str,
    pub wire_to: &'a str,
    pub wire_from: &'a str,
}

#[derive(Copy, Clone, Debug)]
pub struct NodePip {
    pub wire_out: IntWire,
    pub wire_in: IntWire,
    pub wire_out_raw: IntWire,
    pub wire_in_raw: IntWire,

    pub node_die: DieId,
    pub node_crd: Coord,
    pub node_layer: LayerId,
    pub node_wire_out: NodeWireId,
    pub node_wire_in: NodeWireId,
}

impl<'a> ExpandedGrid<'a> {
    pub fn resolve_wire_raw(&self, mut wire: IntWire) -> Option<IntWire> {
        let die = self.die(wire.0);
        loop {
            let tile = &die[wire.1];
            let wi = self.db.wires[wire.2];
            match wi {
                WireKind::ClkOut(_) => {
                    wire.1 = tile.clkroot;
                    break;
                }
                WireKind::MultiBranch(dir) | WireKind::Branch(dir) | WireKind::PipBranch(dir) => {
                    if let Some(t) = &tile.terms[dir] {
                        let term = &self.db.terms[t.kind];
                        match term.wires.get(wire.2) {
                            Some(&TermInfo::BlackHole) => return None,
                            Some(&TermInfo::PassNear(wf)) => {
                                if let Some(n) = t.naming {
                                    let n = &self.db.term_namings[n];
                                    if n.wires_out.contains_id(wire.2) {
                                        break;
                                    }
                                }
                                wire.2 = wf;
                            }
                            Some(&TermInfo::PassFar(wf)) => {
                                if let Some(n) = t.naming {
                                    let n = &self.db.term_namings[n];
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
        if let Some(&twire) = self.xdie_wires.get_by_left(&wire) {
            wire = twire;
        }
        if self.blackhole_wires.contains(&wire) {
            None
        } else {
            Some(wire)
        }
    }

    pub fn resolve_wire_trace(&self, mut wire: IntWire) -> Option<(IntWire, Vec<TracePip>)> {
        let die = self.die(wire.0);
        let mut trace = vec![];
        loop {
            let tile = die.tile(wire.1);
            let wi = self.db.wires[wire.2];
            match wi {
                WireKind::ClkOut(_) => {
                    wire.1 = tile.clkroot;
                    break;
                }
                WireKind::MultiBranch(dir) | WireKind::Branch(dir) | WireKind::PipBranch(dir) => {
                    if let Some(t) = &tile.terms[dir] {
                        let term = &self.db.terms[t.kind];
                        match term.wires.get(wire.2) {
                            Some(&TermInfo::BlackHole) => return None,
                            Some(&TermInfo::PassNear(wf)) => {
                                if let Some(n) = t.naming {
                                    let n = &self.db.term_namings[n];
                                    match n.wires_out.get(wire.2) {
                                        None => (),
                                        Some(TermWireOutNaming::Simple { name }) => {
                                            trace.push(TracePip {
                                                tile: t.tile.as_ref().unwrap(),
                                                wire_to: name,
                                                wire_from: &n.wires_in_near[wf],
                                            });
                                        }
                                        Some(TermWireOutNaming::Buf { name_out, name_in }) => {
                                            trace.push(TracePip {
                                                tile: t.tile.as_ref().unwrap(),
                                                wire_to: name_out,
                                                wire_from: name_in,
                                            });
                                        }
                                    }
                                }
                                wire.2 = wf;
                            }
                            Some(&TermInfo::PassFar(wf)) => {
                                if let Some(n) = t.naming {
                                    let n = &self.db.term_namings[n];
                                    match n.wires_out.get(wire.2) {
                                        None => (),
                                        Some(TermWireOutNaming::Simple { name: name_fout }) => {
                                            match n.wires_in_far[wf] {
                                                TermWireInFarNaming::Simple { ref name } => {
                                                    trace.push(TracePip {
                                                        tile: t.tile.as_ref().unwrap(),
                                                        wire_to: name_fout,
                                                        wire_from: name,
                                                    });
                                                }
                                                TermWireInFarNaming::Buf {
                                                    ref name_out,
                                                    ref name_in,
                                                } => {
                                                    trace.push(TracePip {
                                                        tile: t.tile.as_ref().unwrap(),
                                                        wire_to: name_fout,
                                                        wire_from: name_out,
                                                    });
                                                    trace.push(TracePip {
                                                        tile: t.tile.as_ref().unwrap(),
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
                                                        tile: t.tile.as_ref().unwrap(),
                                                        wire_to: name_fout,
                                                        wire_from: name,
                                                    });
                                                    trace.push(TracePip {
                                                        tile: t.tile_far.as_ref().unwrap(),
                                                        wire_to: name_far_out,
                                                        wire_from: name_far_in,
                                                    });
                                                }
                                            }
                                        }
                                        Some(TermWireOutNaming::Buf { name_out, name_in }) => {
                                            trace.push(TracePip {
                                                tile: t.tile.as_ref().unwrap(),
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
                    let node = tile.nodes.first().unwrap();
                    let nn = &self.db.node_namings[node.naming];
                    trace.push(TracePip {
                        tile: &node.names[NodeRawTileId::from_idx(0)],
                        wire_to: &nn.wires[&(NodeTileId::from_idx(0), wire.2)],
                        wire_from: &nn.wires[&(NodeTileId::from_idx(0), wf)],
                    });
                    wire.2 = wf;
                }
                _ => break,
            }
        }
        if let Some(&twire) = self.xdie_wires.get_by_left(&wire) {
            wire = twire;
        }
        if self.blackhole_wires.contains(&wire) {
            None
        } else {
            Some((wire, trace))
        }
    }

    pub fn resolve_wire(&self, mut wire: IntWire) -> Option<IntWire> {
        let die = self.die(wire.0);
        loop {
            let tile = die.tile(wire.1);
            let wi = self.db.wires[wire.2];
            match wi {
                WireKind::ClkOut(_) => {
                    wire.1 = tile.clkroot;
                    break;
                }
                WireKind::MultiBranch(dir) | WireKind::Branch(dir) | WireKind::PipBranch(dir) => {
                    if let Some(t) = &tile.terms[dir] {
                        let term = &self.db.terms[t.kind];
                        match term.wires.get(wire.2) {
                            Some(&TermInfo::BlackHole) => return None,
                            Some(&TermInfo::PassNear(wf)) => {
                                wire.2 = wf;
                            }
                            Some(&TermInfo::PassFar(wf)) => {
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
                    wire.2 = wf;
                }
                _ => break,
            }
        }
        if let Some(&twire) = self.xdie_wires.get_by_left(&wire) {
            wire = twire;
        }
        if self.blackhole_wires.contains(&wire) {
            None
        } else {
            Some(wire)
        }
    }

    pub fn wire_tree(&self, wire: IntWire) -> Vec<IntWire> {
        if self.blackhole_wires.contains(&wire) {
            return vec![];
        }
        let mut res = vec![];
        let mut queue = vec![wire];
        if let Some(&twire) = self.xdie_wires.get_by_right(&wire) {
            queue.push(twire);
        }
        while let Some(wire) = queue.pop() {
            let die = self.die(wire.0);
            let tile = &die[wire.1];
            res.push(wire);
            if matches!(self.db.wires[wire.2], WireKind::ClkOut(_)) && tile.clkroot == wire.1 {
                for &crd in &die.clk_root_tiles[&wire.1] {
                    if crd != wire.1 {
                        queue.push((wire.0, crd, wire.2));
                    }
                }
            }
            res.push(wire);
            for &wt in &self.db_index.buf_ins[wire.2] {
                queue.push((wire.0, wire.1, wt));
            }
            for dir in Dir::DIRS {
                if let Some(ref term) = tile.terms[dir] {
                    for &wt in &self.db_index.terms[term.kind].wire_ins_near[wire.2] {
                        queue.push((wire.0, wire.1, wt));
                    }
                    if let Some(ocrd) = term.target {
                        let oterm = die[ocrd].terms[!dir].as_ref().unwrap();
                        for &wt in &self.db_index.terms[oterm.kind].wire_ins_far[wire.2] {
                            queue.push((wire.0, ocrd, wt));
                        }
                    }
                }
            }
        }
        res
    }

    pub fn wire_pips_bwd(&self, wire: IntWire) -> Vec<NodePip> {
        let mut wires = vec![wire];
        if matches!(
            self.db.wires[wire.2],
            WireKind::MultiOut
                | WireKind::MultiBranch(_)
                | WireKind::PipOut
                | WireKind::PipBranch(_)
        ) {
            wires = self.wire_tree(wire);
        }
        let mut res = vec![];
        for w in wires {
            for &(crd, layer, tid) in &self.die(w.0)[w.1].node_index {
                let node = &self.die(w.0)[crd].nodes[layer];
                let nk = &self.db.nodes[node.kind];
                let nw = (tid, w.2);
                if let Some(mux) = nk.muxes.get(&nw) {
                    for &nwi in &mux.ins {
                        let wire_in_raw = (w.0, node.tiles[nwi.0], nwi.1);
                        if let Some(wire_in) = self.resolve_wire(wire_in_raw) {
                            res.push(NodePip {
                                wire_out: wire,
                                wire_in,
                                wire_out_raw: w,
                                wire_in_raw,
                                node_die: w.0,
                                node_crd: crd,
                                node_layer: layer,
                                node_wire_out: nw,
                                node_wire_in: nwi,
                            });
                        }
                    }
                }
            }
        }
        res
    }

    pub fn wire_pips_fwd(&self, wire: IntWire) -> Vec<NodePip> {
        let wires = self.wire_tree(wire);
        let mut res = vec![];
        for w in wires {
            for &(crd, layer, tid) in &self.die(w.0)[w.1].node_index {
                let node = &self.die(w.0)[crd].nodes[layer];
                let nki = &self.db_index.nodes[node.kind];
                let nw = (tid, w.2);
                if let Some(outs) = nki.mux_ins.get(&nw) {
                    for &nwo in outs {
                        let wire_out_raw = (w.0, node.tiles[nwo.0], nwo.1);
                        if let Some(wire_out) = self.resolve_wire(wire_out_raw) {
                            res.push(NodePip {
                                wire_out,
                                wire_in: wire,
                                wire_out_raw,
                                wire_in_raw: w,
                                node_die: w.0,
                                node_crd: crd,
                                node_layer: layer,
                                node_wire_out: nwo,
                                node_wire_in: nw,
                            });
                        }
                    }
                }
            }
        }
        res
    }

    pub fn get_node_pip_naming(&self, np: NodePip) -> TracePip {
        let node = &self.die(np.node_die).tile(np.node_crd).nodes[np.node_layer];
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedTile {
    pub nodes: EntityVec<LayerId, ExpandedTileNode>,
    pub terms: EnumMap<Dir, Option<ExpandedTileTerm>>,
    pub node_index: Vec<(Coord, LayerId, NodeTileId)>,
    pub clkroot: Coord,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedTileNode {
    pub kind: NodeKindId,
    pub tiles: EntityVec<NodeTileId, Coord>,
    pub names: EntityPartVec<NodeRawTileId, String>,
    pub tie_name: Option<String>,
    pub tie_rt: NodeRawTileId,
    pub iri_names: EntityVec<NodeIriId, String>,
    pub naming: NodeNamingId,
    pub bels: EntityPartVec<BelId, String>,
}

impl ExpandedTileNode {
    pub fn add_bel(&mut self, idx: usize, name: String) {
        self.bels.insert(BelId::from_idx(idx), name);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedTileTerm {
    pub target: Option<Coord>,
    pub kind: TermKindId,
    pub tile: Option<String>,
    pub tile_far: Option<String>,
    pub naming: Option<TermNamingId>,
}
