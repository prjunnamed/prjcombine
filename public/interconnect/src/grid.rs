#![allow(clippy::too_many_arguments)]

use crate::{db::*, dir::Dir};
use bimap::BiHashMap;
use bincode::{Decode, Encode};
use ndarray::Array2;
use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
};
use unnamed_entity::{
    EntityId, EntityIds, EntityPartVec, EntityVec,
    id::{EntityIdU8, EntityIdU16, EntityTag, EntityTagArith},
};

pub struct DieTag;
pub struct ColTag;
pub struct RowTag;

impl EntityTag for DieTag {
    const PREFIX: &'static str = "D";
}
impl EntityTag for ColTag {
    const PREFIX: &'static str = "X";
}
impl EntityTag for RowTag {
    const PREFIX: &'static str = "Y";
}

impl EntityTagArith for DieTag {}
impl EntityTagArith for ColTag {}
impl EntityTagArith for RowTag {}

pub type DieId = EntityIdU8<DieTag>;
pub type ColId = EntityIdU16<ColTag>;
pub type RowId = EntityIdU16<RowTag>;

pub struct LayerTag;
impl EntityTag for LayerTag {}
pub type LayerId = EntityIdU8<LayerTag>;

pub struct IobTag;
impl EntityTag for IobTag {
    const PREFIX: &'static str = "IOB";
}
pub type TileIobId = EntityIdU8<IobTag>;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
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
            EdgeIoCoord::W(row, iob) => write!(f, "IOB_W{row:#}_{iob:#}"),
            EdgeIoCoord::E(row, iob) => write!(f, "IOB_E{row:#}_{iob:#}"),
            EdgeIoCoord::S(col, iob) => write!(f, "IOB_S{col:#}_{iob:#}"),
            EdgeIoCoord::N(col, iob) => write!(f, "IOB_N{col:#}_{iob:#}"),
        }
    }
}

pub type Coord = (ColId, RowId);
pub type NodeLoc = (DieId, ColId, RowId, LayerId);
pub type WireCoord = (DieId, Coord, WireId);
pub type BelCoord = (DieId, Coord, BelSlotId);

#[derive(Copy, Clone, Debug)]
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
    pub extra_conns: BiHashMap<WireCoord, WireCoord>,
    pub blackhole_wires: HashSet<WireCoord>,
    pub tile_index: EntityVec<TileClassId, Vec<NodeLoc>>,
}

#[derive(Clone, Debug)]
pub struct ExpandedDie {
    tiles: Array2<Cell>,
    pub region_root_cells: EntityVec<RegionSlotId, HashMap<Coord, HashSet<Coord>>>,
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
            extra_conns: BiHashMap::new(),
            blackhole_wires: HashSet::new(),
            tile_index: db.tile_classes.ids().map(|_| vec![]).collect(),
        }
    }

    pub fn add_die<'b>(
        &'b mut self,
        width: usize,
        height: usize,
    ) -> (DieId, ExpandedDieRefMut<'a, 'b>) {
        let dieid = self.die.push(ExpandedDie {
            tiles: Array2::from_shape_fn([height, width], |(r, c)| Cell {
                tiles: Default::default(),
                conns: Default::default(),
                tile_index: vec![],
                region_root: self
                    .db
                    .region_slots
                    .ids()
                    .map(|_| (ColId::from_idx(c), RowId::from_idx(r)))
                    .collect(),
            }),
            region_root_cells: self.db.region_slots.ids().map(|_| HashMap::new()).collect(),
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

    pub fn tile(&self, loc: NodeLoc) -> &Tile {
        &self.die(loc.0).tile((loc.1, loc.2)).tiles[loc.3]
    }

    pub fn tile_wire(&self, loc: NodeLoc, wire: TileClassWire) -> WireCoord {
        let tile = self.tile(loc);
        (loc.0, tile.cells[wire.0], wire.1)
    }

    pub fn resolve_tile_wire_nobuf(&self, loc: NodeLoc, wire: TileClassWire) -> Option<WireCoord> {
        self.resolve_wire_nobuf(self.tile_wire(loc, wire))
    }

    pub fn find_tile(&self, die: DieId, coord: Coord, f: impl Fn(&Tile) -> bool) -> Option<&Tile> {
        let die = self.die(die);
        let tile = die.tile(coord);
        tile.tiles.values().find(|x| f(x))
    }

    pub fn find_tile_loc(
        &self,
        die: DieId,
        coord: Coord,
        f: impl Fn(&Tile) -> bool,
    ) -> Option<(LayerId, &Tile)> {
        let die = self.die(die);
        let tile = die.tile(coord);
        for (id, val) in &tile.tiles {
            if f(val) {
                return Some((id, val));
            }
        }
        None
    }

    pub fn find_tile_layer(
        &self,
        die: DieId,
        coord: Coord,
        f: impl Fn(&str) -> bool,
    ) -> Option<LayerId> {
        let die = self.die(die);
        let tile = die.tile(coord);
        for (layer, val) in &tile.tiles {
            if f(self.db.tile_classes.key(val.class)) {
                return Some(layer);
            }
        }
        None
    }

    pub fn find_tile_by_class(
        &self,
        die: DieId,
        coord: Coord,
        f: impl Fn(&str) -> bool,
    ) -> Option<NodeLoc> {
        let gdie = self.die(die);
        let tile = gdie.tile(coord);
        for (layer, val) in &tile.tiles {
            if f(self.db.tile_classes.key(val.class)) {
                return Some((die, coord.0, coord.1, layer));
            }
        }
        None
    }

    pub fn get_tile_by_class(&self, die: DieId, coord: Coord, f: impl Fn(&str) -> bool) -> NodeLoc {
        self.find_tile_by_class(die, coord, f).unwrap()
    }

    pub fn find_tile_by_bel(&self, bel: BelCoord) -> Option<NodeLoc> {
        let (die, coord, slot) = bel;
        let gdie = self.die(die);
        let tile = gdie.tile(coord);
        for (layer, tile) in &tile.tiles {
            let nk = &self.db.tile_classes[tile.class];
            if nk.bels.contains_id(slot) {
                return Some((die, coord.0, coord.1, layer));
            }
        }
        None
    }

    pub fn get_tile_by_bel(&self, bel: BelCoord) -> NodeLoc {
        self.find_tile_by_bel(bel).unwrap()
    }

    pub fn find_bel_layer(&self, bel: BelCoord) -> Option<LayerId> {
        self.find_tile_by_bel(bel).map(|(_, _, _, layer)| layer)
    }

    pub fn get_bel_pin(&self, bel: BelCoord, pin: &str) -> Vec<WireCoord> {
        let tloc = self.get_tile_by_bel(bel);
        let tile = self.tile(tloc);
        let pin_info = &self.db.tile_classes[tile.class].bels[bel.2].pins[pin];
        pin_info
            .wires
            .iter()
            .map(|&(cell, wire)| (tloc.0, tile.cells[cell], wire))
            .collect()
    }

    pub fn finish(&mut self) {
        for (dieid, die) in &mut self.die {
            for rslot in self.db.region_slots.ids() {
                let mut region_root_cells: HashMap<_, HashSet<_>> = HashMap::new();
                for col in die.cols() {
                    for row in die.rows() {
                        region_root_cells
                            .entry(die[(col, row)].region_root[rslot])
                            .or_default()
                            .insert((col, row));
                    }
                }
                die.region_root_cells[rslot] = region_root_cells;
            }
            for col in die.cols() {
                for row in die.rows() {
                    for (layer, node) in &die[(col, row)].tiles {
                        self.tile_index[node.class].push((dieid, col, row, layer));
                    }
                }
            }
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
    type Output = Cell;
    fn index(&self, xy: Coord) -> &Cell {
        &self.tiles[[xy.1.to_idx(), xy.0.to_idx()]]
    }
}

impl core::ops::IndexMut<Coord> for ExpandedDie {
    fn index_mut(&mut self, xy: Coord) -> &mut Cell {
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
    pub fn tile(&self, xy: Coord) -> &'a Cell {
        &self.grid.die[self.die].tiles[[xy.1.to_idx(), xy.0.to_idx()]]
    }
}

impl ExpandedDieRefMut<'_, '_> {
    pub fn add_tile(&mut self, crd: Coord, kind: &str, cells: &[Coord]) -> &mut Tile {
        let kind = self.grid.db.get_tile_class(kind);
        let cells: EntityVec<_, _> = cells.iter().copied().collect();
        let layer = self[crd].tiles.push(Tile {
            class: kind,
            cells: cells.clone(),
        });
        for (cid, ccrd) in cells {
            self[ccrd].tile_index.push((crd, layer, cid))
        }
        &mut self[crd].tiles[layer]
    }

    pub fn fill_tile(&mut self, xy: Coord, kind: &str) -> &mut Tile {
        assert!(self[xy].tiles.is_empty());
        self.add_tile(xy, kind, &[xy])
    }

    pub fn fill_conn_pair(&mut self, a: Coord, b: Coord, fwd: &str, bwd: &str) {
        let fwd = self.grid.db.get_conn_class(fwd);
        let bwd = self.grid.db.get_conn_class(bwd);
        let this = &mut *self;
        let fwd = Connector {
            target: Some(b),
            class: fwd,
        };
        let bwd = Connector {
            target: Some(a),
            class: bwd,
        };
        let a = bwd.target.unwrap();
        let b = fwd.target.unwrap();
        let fwd_slot = this.grid.db.conn_classes[fwd.class].slot;
        let bwd_slot = this.grid.db.conn_classes[bwd.class].slot;
        this[a].conns.insert(fwd_slot, fwd);
        this[b].conns.insert(bwd_slot, bwd);
    }

    pub fn fill_conn_term(&mut self, xy: Coord, kind: &str) {
        let ccls = self.grid.db.get_conn_class(kind);
        let slot = self.grid.db.conn_classes[ccls].slot;
        self[xy].conns.insert(
            slot,
            Connector {
                class: ccls,
                target: None,
            },
        );
    }

    pub fn fill_main_passes(&mut self) {
        let pass_w = "MAIN.W";
        let pass_e = "MAIN.E";
        let pass_s = "MAIN.S";
        let pass_n = "MAIN.N";
        let slot_w = self.grid.db.get_conn_slot("W");
        let slot_e = self.grid.db.get_conn_slot("E");
        let slot_s = self.grid.db.get_conn_slot("S");
        let slot_n = self.grid.db.get_conn_slot("N");
        // horizontal
        for row in self.rows() {
            let mut prev = None;
            for col in self.cols() {
                if self[(col, row)].tiles.is_empty() {
                    continue;
                }
                if let Some(prev) = prev {
                    if !self[(col, row)].conns.contains_id(slot_w) {
                        self.fill_conn_pair((prev, row), (col, row), pass_e, pass_w);
                    }
                }
                if !self[(col, row)].conns.contains_id(slot_e) {
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
                if self[(col, row)].tiles.is_empty() {
                    continue;
                }
                if let Some(prev) = prev {
                    if !self[(col, row)].conns.contains_id(slot_s) {
                        self.fill_conn_pair((col, prev), (col, row), pass_n, pass_s);
                    }
                }
                if !self[(col, row)].conns.contains_id(slot_n) {
                    prev = Some(row);
                } else {
                    prev = None;
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct NodePip {
    pub wire_out: WireCoord,
    pub wire_in: WireCoord,
    pub wire_out_raw: WireCoord,
    pub wire_in_raw: WireCoord,

    pub node_die: DieId,
    pub node_crd: Coord,
    pub node_layer: LayerId,
    pub node_wire_out: TileClassWire,
    pub node_wire_in: TileClassWire,
}

impl ExpandedGrid<'_> {
    pub fn resolve_wire(&self, mut wire: WireCoord) -> Option<WireCoord> {
        let die = self.die(wire.0);
        loop {
            let tile = die.tile(wire.1);
            let wi = self.db.wires[wire.2];
            match wi {
                WireKind::Regional(rslot) => {
                    wire.1 = tile.region_root[rslot];
                    break;
                }
                WireKind::MultiBranch(slot)
                | WireKind::Branch(slot)
                | WireKind::PipBranch(slot) => {
                    if let Some(t) = tile.conns.get(slot) {
                        let ccls = &self.db.conn_classes[t.class];
                        match ccls.wires.get(wire.2) {
                            Some(&ConnectorWire::BlackHole) => return None,
                            Some(&ConnectorWire::Reflect(wf)) => {
                                wire.2 = wf;
                            }
                            Some(&ConnectorWire::Pass(wf)) => {
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
        if let Some(&twire) = self.extra_conns.get_by_left(&wire) {
            wire = twire;
        }
        if self.blackhole_wires.contains(&wire) {
            None
        } else {
            Some(wire)
        }
    }

    pub fn resolve_wire_nobuf(&self, mut wire: WireCoord) -> Option<WireCoord> {
        let die = self.die(wire.0);
        loop {
            let tile = die.tile(wire.1);
            let wi = self.db.wires[wire.2];
            match wi {
                WireKind::Regional(rslot) => {
                    wire.1 = tile.region_root[rslot];
                    break;
                }
                WireKind::MultiBranch(slot)
                | WireKind::Branch(slot)
                | WireKind::PipBranch(slot) => {
                    if let Some(t) = tile.conns.get(slot) {
                        let ccls = &self.db.conn_classes[t.class];
                        match ccls.wires.get(wire.2) {
                            Some(&ConnectorWire::BlackHole) => return None,
                            Some(&ConnectorWire::Reflect(wf)) => {
                                wire.2 = wf;
                            }
                            Some(&ConnectorWire::Pass(wf)) => {
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
        if let Some(&twire) = self.extra_conns.get_by_left(&wire) {
            wire = twire;
        }
        if self.blackhole_wires.contains(&wire) {
            None
        } else {
            Some(wire)
        }
    }

    pub fn wire_tree(&self, wire: WireCoord) -> Vec<WireCoord> {
        if self.blackhole_wires.contains(&wire) {
            return vec![];
        }
        let mut res = vec![];
        let mut queue = vec![wire];
        if let Some(&twire) = self.extra_conns.get_by_right(&wire) {
            queue.push(twire);
        }
        while let Some(wire) = queue.pop() {
            let die = self.die(wire.0);
            let tile = &die[wire.1];
            res.push(wire);
            if let WireKind::Regional(rslot) = self.db.wires[wire.2] {
                if tile.region_root[rslot] == wire.1 {
                    for &crd in &die.region_root_cells[rslot][&wire.1] {
                        if crd != wire.1 {
                            queue.push((wire.0, crd, wire.2));
                        }
                    }
                }
            }
            for &wt in &self.db_index.buf_ins[wire.2] {
                queue.push((wire.0, wire.1, wt));
            }
            for (slot, conn) in &tile.conns {
                let oslot = self.db.conn_slots[slot].opposite;
                for &wt in &self.db_index.conn_classes[conn.class].wire_ins_near[wire.2] {
                    queue.push((wire.0, wire.1, wt));
                }
                if let Some(ocrd) = conn.target {
                    let oconn = &die[ocrd].conns[oslot];
                    for &wt in &self.db_index.conn_classes[oconn.class].wire_ins_far[wire.2] {
                        queue.push((wire.0, ocrd, wt));
                    }
                }
            }
        }
        res
    }

    pub fn wire_pips_bwd(&self, wire: WireCoord) -> Vec<NodePip> {
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
            for &(crd, layer, tid) in &self.die(w.0)[w.1].tile_index {
                let tile = &self.die(w.0)[crd].tiles[layer];
                let tcls = &self.db.tile_classes[tile.class];
                let tcw = (tid, w.2);
                if let Some(mux) = tcls.muxes.get(&tcw) {
                    for &tcwi in &mux.ins {
                        let wire_in_raw = (w.0, tile.cells[tcwi.0], tcwi.1);
                        if let Some(wire_in) = self.resolve_wire(wire_in_raw) {
                            res.push(NodePip {
                                wire_out: wire,
                                wire_in,
                                wire_out_raw: w,
                                wire_in_raw,
                                node_die: w.0,
                                node_crd: crd,
                                node_layer: layer,
                                node_wire_out: tcw,
                                node_wire_in: tcwi,
                            });
                        }
                    }
                }
            }
        }
        res
    }

    pub fn wire_pips_fwd(&self, wire: WireCoord) -> Vec<NodePip> {
        let wires = self.wire_tree(wire);
        let mut res = vec![];
        for w in wires {
            for &(crd, layer, tid) in &self.die(w.0)[w.1].tile_index {
                let tile = &self.die(w.0)[crd].tiles[layer];
                let tcls = &self.db_index.tile_classes[tile.class];
                let tcw = (tid, w.2);
                if let Some(outs) = tcls.mux_ins.get(&tcw) {
                    for &tcwo in outs {
                        let wire_out_raw = (w.0, tile.cells[tcwo.0], tcwo.1);
                        if let Some(wire_out) = self.resolve_wire(wire_out_raw) {
                            res.push(NodePip {
                                wire_out,
                                wire_in: wire,
                                wire_out_raw,
                                wire_in_raw: w,
                                node_die: w.0,
                                node_crd: crd,
                                node_layer: layer,
                                node_wire_out: tcwo,
                                node_wire_in: tcw,
                            });
                        }
                    }
                }
            }
        }
        res
    }
}

#[derive(Clone, Debug)]
pub struct Cell {
    pub tiles: EntityVec<LayerId, Tile>,
    pub conns: EntityPartVec<ConnectorSlotId, Connector>,
    pub tile_index: Vec<(Coord, LayerId, TileCellId)>,
    pub region_root: EntityVec<RegionSlotId, Coord>,
}

#[derive(Clone, Debug)]
pub struct Tile {
    pub class: TileClassId,
    pub cells: EntityVec<TileCellId, Coord>,
}

#[derive(Clone, Debug)]
pub struct Connector {
    pub class: ConnectorClassId,
    pub target: Option<Coord>,
}
