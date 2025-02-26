#![allow(clippy::too_many_arguments)]

use crate::{db::*, dir::Dir};
use bimap::BiHashMap;
use ndarray::Array2;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
};
use unnamed_entity::{EntityId, EntityIds, EntityPartVec, EntityVec, entity_id};

entity_id! {
    pub id DieId u8, reserve 1, delta;
    pub id ColId u16, reserve 1, delta;
    pub id RowId u16, reserve 1, delta;
    pub id LayerId u8;
    pub id TileIobId u8;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum EdgeIoCoord {
    W(RowId, TileIobId),
    E(RowId, TileIobId),
    S(ColId, TileIobId),
    N(ColId, TileIobId),
}

impl EdgeIoCoord {
    pub fn with_iob(self, iob: TileIobId) -> Self {
        match self {
            EdgeIoCoord::W(row, _) => EdgeIoCoord::W(row, iob),
            EdgeIoCoord::E(row, _) => EdgeIoCoord::E(row, iob),
            EdgeIoCoord::S(col, _) => EdgeIoCoord::S(col, iob),
            EdgeIoCoord::N(col, _) => EdgeIoCoord::N(col, iob),
        }
    }

    pub fn iob(self) -> TileIobId {
        match self {
            EdgeIoCoord::W(_, iob) => iob,
            EdgeIoCoord::E(_, iob) => iob,
            EdgeIoCoord::S(_, iob) => iob,
            EdgeIoCoord::N(_, iob) => iob,
        }
    }

    pub fn edge(&self) -> Dir {
        match self {
            EdgeIoCoord::W(..) => Dir::W,
            EdgeIoCoord::E(..) => Dir::E,
            EdgeIoCoord::S(..) => Dir::S,
            EdgeIoCoord::N(..) => Dir::N,
        }
    }
}

impl std::fmt::Display for EdgeIoCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EdgeIoCoord::W(row, iob) => write!(f, "IOB_W{row}_{iob}"),
            EdgeIoCoord::E(row, iob) => write!(f, "IOB_E{row}_{iob}"),
            EdgeIoCoord::S(col, iob) => write!(f, "IOB_S{col}_{iob}"),
            EdgeIoCoord::N(col, iob) => write!(f, "IOB_N{col}_{iob}"),
        }
    }
}

pub type Coord = (ColId, RowId);
pub type NodeLoc = (DieId, ColId, RowId, LayerId);
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
    pub die: EntityVec<DieId, ExpandedDie>,
    pub xdie_wires: BiHashMap<IntWire, IntWire>,
    pub blackhole_wires: HashSet<IntWire>,
    pub node_index: EntityVec<NodeKindId, Vec<NodeLoc>>,
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

    pub fn node(&self, loc: NodeLoc) -> &ExpandedTileNode {
        &self.die(loc.0).tile((loc.1, loc.2)).nodes[loc.3]
    }

    pub fn node_wire(&self, loc: NodeLoc, wire: NodeWireId) -> IntWire {
        let node = self.node(loc);
        (loc.0, node.tiles[wire.0], wire.1)
    }

    pub fn resolve_node_wire_nobuf(&self, loc: NodeLoc, wire: NodeWireId) -> Option<IntWire> {
        self.resolve_wire_nobuf(self.node_wire(loc, wire))
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

    pub fn find_node_loc(
        &self,
        die: DieId,
        coord: Coord,
        f: impl Fn(&ExpandedTileNode) -> bool,
    ) -> Option<(LayerId, &ExpandedTileNode)> {
        let die = self.die(die);
        let tile = die.tile(coord);
        for (id, val) in &tile.nodes {
            if f(val) {
                return Some((id, val));
            }
        }
        None
    }

    pub fn find_node_layer(
        &self,
        die: DieId,
        coord: Coord,
        f: impl Fn(&str) -> bool,
    ) -> Option<LayerId> {
        let die = self.die(die);
        let tile = die.tile(coord);
        for (layer, val) in &tile.nodes {
            if f(self.db.nodes.key(val.kind)) {
                return Some(layer);
            }
        }
        None
    }

    pub fn find_bel(
        &self,
        die: DieId,
        coord: Coord,
        key: &str,
    ) -> Option<(LayerId, &ExpandedTileNode, BelId, &BelInfo)> {
        let die = self.die(die);
        let tile = die.tile(coord);
        for (layer, node) in &tile.nodes {
            let nk = &self.db.nodes[node.kind];
            if let Some((id, bel)) = nk.bels.get(key) {
                return Some((layer, node, id, bel));
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
                                    rc = rw.1.0.to_idx(),
                                    rr = rw.1.1.to_idx(),
                                    rw = self.db.wires.key(rw.2),
                                    od = wire.0.to_idx(),
                                    oc = wire.1.0.to_idx(),
                                    or = wire.1.1.to_idx(),
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
    pub fn add_xnode(&mut self, crd: Coord, kind: &str, coords: &[Coord]) -> &mut ExpandedTileNode {
        let kind = self.grid.db.get_node(kind);
        let tiles: EntityVec<_, _> = coords.iter().copied().collect();
        let layer = self[crd].nodes.push(ExpandedTileNode {
            kind,
            tiles: tiles.clone(),
        });
        for (tid, tcrd) in tiles {
            self[tcrd].node_index.push((crd, layer, tid))
        }
        &mut self[crd].nodes[layer]
    }

    pub fn fill_tile(&mut self, xy: Coord, kind: &str) -> &mut ExpandedTileNode {
        assert!(self[xy].nodes.is_empty());
        self.add_xnode(xy, kind, &[xy])
    }

    pub fn fill_term_pair(&mut self, a: Coord, b: Coord, fwd: &str, bwd: &str) {
        let fwd = self.grid.db.get_term(fwd);
        let bwd = self.grid.db.get_term(bwd);
        let this = &mut *self;
        let fwd = ExpandedTileTerm {
            target: Some(b),
            kind: fwd,
        };
        let bwd = ExpandedTileTerm {
            target: Some(a),
            kind: bwd,
        };
        let a = bwd.target.unwrap();
        let b = fwd.target.unwrap();
        let fwd_slot = this.grid.db.terms[fwd.kind].slot;
        let bwd_slot = this.grid.db.terms[bwd.kind].slot;
        this[a].terms.insert(fwd_slot, fwd);
        this[b].terms.insert(bwd_slot, bwd);
    }

    pub fn fill_term(&mut self, xy: Coord, kind: &str) {
        let kind = self.grid.db.get_term(kind);
        let slot = self.grid.db.terms[kind].slot;
        self[xy]
            .terms
            .insert(slot, ExpandedTileTerm { target: None, kind });
    }

    pub fn fill_main_passes(&mut self) {
        let pass_w = "MAIN.W";
        let pass_e = "MAIN.E";
        let pass_s = "MAIN.S";
        let pass_n = "MAIN.N";
        let slot_w = self.grid.db.get_term_slot("W");
        let slot_e = self.grid.db.get_term_slot("E");
        let slot_s = self.grid.db.get_term_slot("S");
        let slot_n = self.grid.db.get_term_slot("N");
        // horizontal
        for row in self.rows() {
            let mut prev = None;
            for col in self.cols() {
                if self[(col, row)].nodes.is_empty() {
                    continue;
                }
                if let Some(prev) = prev {
                    if !self[(col, row)].terms.contains_id(slot_w) {
                        self.fill_term_pair((prev, row), (col, row), pass_e, pass_w);
                    }
                }
                if !self[(col, row)].terms.contains_id(slot_e) {
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
                    if !self[(col, row)].terms.contains_id(slot_s) {
                        self.fill_term_pair((col, prev), (col, row), pass_n, pass_s);
                    }
                }
                if !self[(col, row)].terms.contains_id(slot_n) {
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

impl ExpandedGrid<'_> {
    pub fn resolve_wire(&self, mut wire: IntWire) -> Option<IntWire> {
        let die = self.die(wire.0);
        loop {
            let tile = die.tile(wire.1);
            let wi = self.db.wires[wire.2];
            match wi {
                WireKind::ClkOut => {
                    wire.1 = tile.clkroot;
                    break;
                }
                WireKind::MultiBranch(slot)
                | WireKind::Branch(slot)
                | WireKind::PipBranch(slot) => {
                    if let Some(t) = tile.terms.get(slot) {
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

    pub fn resolve_wire_nobuf(&self, mut wire: IntWire) -> Option<IntWire> {
        let die = self.die(wire.0);
        loop {
            let tile = die.tile(wire.1);
            let wi = self.db.wires[wire.2];
            match wi {
                WireKind::ClkOut => {
                    wire.1 = tile.clkroot;
                    break;
                }
                WireKind::MultiBranch(slot)
                | WireKind::Branch(slot)
                | WireKind::PipBranch(slot) => {
                    if let Some(t) = tile.terms.get(slot) {
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
            if matches!(self.db.wires[wire.2], WireKind::ClkOut) && tile.clkroot == wire.1 {
                for &crd in &die.clk_root_tiles[&wire.1] {
                    if crd != wire.1 {
                        queue.push((wire.0, crd, wire.2));
                    }
                }
            }
            for &wt in &self.db_index.buf_ins[wire.2] {
                queue.push((wire.0, wire.1, wt));
            }
            for (slot, term) in &tile.terms {
                let oslot = self.db.term_slots[slot].opposite;
                for &wt in &self.db_index.terms[term.kind].wire_ins_near[wire.2] {
                    queue.push((wire.0, wire.1, wt));
                }
                if let Some(ocrd) = term.target {
                    let oterm = &die[ocrd].terms[oslot];
                    for &wt in &self.db_index.terms[oterm.kind].wire_ins_far[wire.2] {
                        queue.push((wire.0, ocrd, wt));
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedTile {
    pub nodes: EntityVec<LayerId, ExpandedTileNode>,
    pub terms: EntityPartVec<TermSlotId, ExpandedTileTerm>,
    pub node_index: Vec<(Coord, LayerId, NodeTileId)>,
    pub clkroot: Coord,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedTileNode {
    pub kind: NodeKindId,
    pub tiles: EntityVec<NodeTileId, Coord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedTileTerm {
    pub target: Option<Coord>,
    pub kind: TermKindId,
}
