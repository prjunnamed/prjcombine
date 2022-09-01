#![allow(clippy::too_many_arguments)]
#![allow(clippy::collapsible_else_if)]

use crate::db::*;
use enum_map::EnumMap;
use ndarray::Array2;
use prjcombine_entity::{entity_id, EntityId, EntityIds, EntityPartVec, EntityVec};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

entity_id! {
    pub id DieId u16, reserve 1;
    pub id ColId u16, reserve 1;
    pub id RowId u16, reserve 1;
}

impl core::ops::Add<usize> for ColId {
    type Output = ColId;
    fn add(self, x: usize) -> ColId {
        ColId::from_idx(self.to_idx() + x)
    }
}

impl core::ops::AddAssign<usize> for ColId {
    fn add_assign(&mut self, x: usize) {
        *self = *self + x;
    }
}

impl core::ops::Sub<usize> for ColId {
    type Output = ColId;
    fn sub(self, x: usize) -> ColId {
        ColId::from_idx(self.to_idx() - x)
    }
}

impl core::ops::SubAssign<usize> for ColId {
    fn sub_assign(&mut self, x: usize) {
        *self = *self - x;
    }
}

impl core::ops::Add<usize> for RowId {
    type Output = RowId;
    fn add(self, x: usize) -> RowId {
        RowId::from_idx(self.to_idx() + x)
    }
}

impl core::ops::AddAssign<usize> for RowId {
    fn add_assign(&mut self, x: usize) {
        *self = *self + x;
    }
}

impl core::ops::Sub<usize> for RowId {
    type Output = RowId;
    fn sub(self, x: usize) -> RowId {
        RowId::from_idx(self.to_idx() - x)
    }
}

impl core::ops::SubAssign<usize> for RowId {
    fn sub_assign(&mut self, x: usize) {
        *self = *self - x;
    }
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

#[derive(Clone, Debug)]
pub struct ExpandedGrid<'a> {
    pub db: &'a IntDb,
    pub tie_kind: Option<String>,
    pub tie_pin_gnd: Option<String>,
    pub tie_pin_vcc: Option<String>,
    pub tie_pin_pullup: Option<String>,
    pub tiles: EntityVec<DieId, Array2<ExpandedTile>>,
    pub xdie_wires: HashMap<IntWire, IntWire>,
    pub blackhole_wires: HashSet<IntWire>,
    pub cursed_wires: HashSet<IntWire>,
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
            tie_kind: None,
            tie_pin_gnd: None,
            tie_pin_vcc: None,
            tie_pin_pullup: None,
            tiles: EntityVec::new(),
            xdie_wires: HashMap::new(),
            blackhole_wires: HashSet::new(),
            cursed_wires: HashSet::new(),
        }
    }

    pub fn add_die<'b>(
        &'b mut self,
        width: usize,
        height: usize,
    ) -> (DieId, ExpandedDieRefMut<'a, 'b>) {
        let dieid = self
            .tiles
            .push(Array2::from_shape_fn([height, width], |(r, c)| {
                ExpandedTile {
                    nodes: Default::default(),
                    intfs: Default::default(),
                    terms: Default::default(),
                    clkroot: (ColId::from_idx(c), RowId::from_idx(r)),
                }
            }));
        (dieid, self.die_mut(dieid))
    }

    pub fn dies<'b>(&'b self) -> impl Iterator<Item = ExpandedDieRef<'a, 'b>> {
        self.tiles.ids().map(|die| self.die(die))
    }

    pub fn die<'b>(&'b self, die: DieId) -> ExpandedDieRef<'a, 'b> {
        ExpandedDieRef { grid: self, die }
    }
    pub fn die_mut<'b>(&'b mut self, die: DieId) -> ExpandedDieRefMut<'a, 'b> {
        ExpandedDieRefMut { grid: self, die }
    }

    pub fn find_node(
        &self,
        die: DieId,
        coord: Coord,
        f: impl Fn(&ExpandedTileNode) -> bool,
    ) -> Option<&ExpandedTileNode> {
        let die = self.die(die);
        let tile = die.tile(coord);
        tile.nodes.iter().find(|x| f(x))
    }

    pub fn find_bel(
        &self,
        die: DieId,
        coord: Coord,
        key: &str,
    ) -> Option<(&ExpandedTileNode, BelId, &BelInfo, &BelNaming)> {
        let die = self.die(die);
        let tile = die.tile(coord);
        for node in &tile.nodes {
            let nk = &self.db.nodes[node.kind];
            let naming = &self.db.node_namings[node.naming];
            if let Some((id, bel)) = nk.bels.get(key) {
                return Some((node, id, bel, &naming.bels[id]));
            }
        }
        None
    }
}

impl core::ops::Index<Coord> for ExpandedDieRef<'_, '_> {
    type Output = ExpandedTile;
    fn index(&self, xy: Coord) -> &ExpandedTile {
        &self.grid.tiles[self.die][[xy.1.to_idx(), xy.0.to_idx()]]
    }
}

impl core::ops::Index<Coord> for ExpandedDieRefMut<'_, '_> {
    type Output = ExpandedTile;
    fn index(&self, xy: Coord) -> &ExpandedTile {
        &self.grid.tiles[self.die][[xy.1.to_idx(), xy.0.to_idx()]]
    }
}

impl core::ops::IndexMut<Coord> for ExpandedDieRefMut<'_, '_> {
    fn index_mut(&mut self, xy: Coord) -> &mut ExpandedTile {
        &mut self.grid.tiles[self.die][[xy.1.to_idx(), xy.0.to_idx()]]
    }
}

impl<'a> ExpandedDieRef<'_, 'a> {
    pub fn rows(&self) -> EntityIds<RowId> {
        EntityIds::new(self.grid.tiles[self.die].shape()[0])
    }

    pub fn cols(&self) -> EntityIds<ColId> {
        EntityIds::new(self.grid.tiles[self.die].shape()[1])
    }

    pub fn tile(&self, xy: Coord) -> &'a ExpandedTile {
        &self.grid.tiles[self.die][[xy.1.to_idx(), xy.0.to_idx()]]
    }
}

impl ExpandedDieRefMut<'_, '_> {
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
        self[xy].nodes.push(ExpandedTileNode {
            kind,
            tiles: [xy].into_iter().collect(),
            names: [(NodeRawTileId::from_idx(0), name)].into_iter().collect(),
            tie_name: None,
            naming,
            special: false,
            bels: Default::default(),
        });
        self[xy].nodes.last_mut().unwrap()
    }

    pub fn fill_tile_special(
        &mut self,
        xy: Coord,
        kind: &str,
        naming: &str,
        name: String,
    ) -> &mut ExpandedTileNode {
        assert!(self[xy].nodes.is_empty());
        let kind = self.grid.db.get_node(kind);
        let naming = self.grid.db.get_node_naming(naming);
        self[xy].nodes.push(ExpandedTileNode {
            kind,
            tiles: [xy].into_iter().collect(),
            names: [(NodeRawTileId::from_idx(0), name)].into_iter().collect(),
            tie_name: None,
            naming,
            special: true,
            bels: Default::default(),
        });
        self[xy].nodes.last_mut().unwrap()
    }

    pub fn nuke_rect(&mut self, x: ColId, y: RowId, w: usize, h: usize) {
        for dx in 0..w {
            for dy in 0..h {
                self[(x + dx, y + dy)].nodes.clear();
                self[(x + dx, y + dy)].intfs.clear();
                self[(x + dx, y + dy)].terms = Default::default();
            }
        }
    }

    pub fn rows(&self) -> EntityIds<RowId> {
        EntityIds::new(self.grid.tiles[self.die].shape()[0])
    }

    pub fn cols(&self) -> EntityIds<ColId> {
        EntityIds::new(self.grid.tiles[self.die].shape()[1])
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

impl ExpandedGrid<'_> {
    pub fn resolve_wire_raw(&self, mut wire: IntWire) -> Option<IntWire> {
        let owire = wire;
        let die = self.die(wire.0);
        loop {
            let tile = &die[wire.1];
            let wi = &self.db.wires[wire.2];
            match wi.kind {
                WireKind::ClkOut(_) => {
                    wire.1 = tile.clkroot;
                    break;
                }
                WireKind::CondAlias(node, wf) => {
                    if tile.nodes[0].kind != node {
                        break;
                    }
                    wire.2 = wf;
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
        if self.blackhole_wires.contains(&wire) {
            None
        } else if let Some(&twire) = self.xdie_wires.get(&wire) {
            if self.cursed_wires.contains(&twire) {
                Some(owire)
            } else {
                Some(twire)
            }
        } else {
            if self.cursed_wires.contains(&wire) {
                Some(owire)
            } else {
                Some(wire)
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedTile {
    pub nodes: Vec<ExpandedTileNode>,
    pub intfs: Vec<ExpandedTileIntf>,
    pub terms: EnumMap<Dir, Option<ExpandedTileTerm>>,
    pub clkroot: Coord,
}

impl ExpandedTile {
    pub fn insert_intf(
        &mut self,
        pos: usize,
        kind: IntfKindId,
        name: String,
        naming: IntfNamingId,
    ) {
        self.intfs
            .insert(pos, ExpandedTileIntf { kind, name, naming });
    }

    pub fn add_intf(&mut self, kind: IntfKindId, name: String, naming: IntfNamingId) {
        self.intfs.push(ExpandedTileIntf { kind, name, naming });
    }

    pub fn add_xnode(
        &mut self,
        kind: NodeKindId,
        names: &[&str],
        naming: NodeNamingId,
        coords: &[Coord],
    ) -> &mut ExpandedTileNode {
        let names: EntityVec<_, _> = names.iter().map(|x| x.to_string()).collect();
        let names = names.into_iter().collect();
        self.nodes.push(ExpandedTileNode {
            kind,
            tiles: coords.iter().copied().collect(),
            names,
            tie_name: None,
            naming,
            special: true,
            bels: Default::default(),
        });
        self.nodes.last_mut().unwrap()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedTileNode {
    pub kind: NodeKindId,
    pub tiles: EntityVec<NodeTileId, Coord>,
    pub names: EntityPartVec<NodeRawTileId, String>,
    pub tie_name: Option<String>,
    pub naming: NodeNamingId,
    pub special: bool,
    pub bels: EntityPartVec<BelId, String>,
}

impl ExpandedTileNode {
    pub fn add_bel(&mut self, idx: usize, name: String) {
        self.bels.insert(BelId::from_idx(idx), name);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedTileIntf {
    pub kind: IntfKindId,
    pub name: String,
    pub naming: IntfNamingId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedTileTerm {
    pub target: Option<Coord>,
    pub kind: TermKindId,
    pub tile: Option<String>,
    pub tile_far: Option<String>,
    pub naming: Option<TermNamingId>,
}
