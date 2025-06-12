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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode)]
pub struct CellCoord {
    pub die: DieId,
    pub col: ColId,
    pub row: RowId,
}

impl CellCoord {
    pub fn new(die: DieId, col: ColId, row: RowId) -> Self {
        Self { die, col, row }
    }

    pub fn with_cr(self, col: ColId, row: RowId) -> CellCoord {
        CellCoord { col, row, ..self }
    }

    pub fn with_col(self, col: ColId) -> CellCoord {
        CellCoord { col, ..self }
    }

    pub fn with_row(self, row: RowId) -> CellCoord {
        CellCoord { row, ..self }
    }

    pub fn tile(self, slot: TileSlotId) -> TileCoord {
        TileCoord { cell: self, slot }
    }

    pub fn wire(self, slot: WireId) -> WireCoord {
        WireCoord { cell: self, slot }
    }

    pub fn bel(self, slot: BelSlotId) -> BelCoord {
        BelCoord { cell: self, slot }
    }

    pub fn connector(self, slot: ConnectorSlotId) -> ConnectorCoord {
        ConnectorCoord { cell: self, slot }
    }

    pub fn delta(&self, dx: i32, dy: i32) -> CellCoord {
        self.with_cr(self.col + dx, self.row + dy)
    }
}

impl std::fmt::Display for CellCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{die}{col}{row}",
            die = self.die,
            col = self.col,
            row = self.row
        )
    }
}

impl std::fmt::Debug for CellCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{die}{col}{row}",
            die = self.die,
            col = self.col,
            row = self.row
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode)]
pub struct TileCoord {
    pub cell: CellCoord,
    pub slot: TileSlotId,
}

impl TileCoord {
    pub fn to_string(&self, db: &IntDb) -> String {
        format!(
            "{cell}_{slot}",
            cell = self.cell,
            slot = db.tile_slots[self.slot]
        )
    }
}

impl std::ops::Deref for TileCoord {
    type Target = CellCoord;

    fn deref(&self) -> &Self::Target {
        &self.cell
    }
}

impl std::ops::DerefMut for TileCoord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cell
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode)]
pub struct ConnectorCoord {
    pub cell: CellCoord,
    pub slot: ConnectorSlotId,
}

impl ConnectorCoord {
    pub fn to_string(&self, db: &IntDb) -> String {
        format!(
            "{cell}_{slot}",
            cell = self.cell,
            slot = db.conn_slots.key(self.slot)
        )
    }
}

impl std::ops::Deref for ConnectorCoord {
    type Target = CellCoord;

    fn deref(&self) -> &Self::Target {
        &self.cell
    }
}

impl std::ops::DerefMut for ConnectorCoord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cell
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode)]
pub struct WireCoord {
    pub cell: CellCoord,
    pub slot: WireId,
}

impl WireCoord {
    pub fn to_string(&self, db: &IntDb) -> String {
        format!(
            "{cell}_{slot}",
            cell = self.cell,
            slot = db.wires.key(self.slot)
        )
    }
}

impl std::ops::Deref for WireCoord {
    type Target = CellCoord;

    fn deref(&self) -> &Self::Target {
        &self.cell
    }
}

impl std::ops::DerefMut for WireCoord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cell
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode)]
pub struct BelCoord {
    pub cell: CellCoord,
    pub slot: BelSlotId,
}

impl BelCoord {
    pub fn to_string(&self, db: &IntDb) -> String {
        format!(
            "{cell}_{slot}",
            cell = self.cell,
            slot = db.bel_slots.key(self.slot)
        )
    }
}

impl std::ops::Deref for BelCoord {
    type Target = CellCoord;

    fn deref(&self) -> &Self::Target {
        &self.cell
    }
}

impl std::ops::DerefMut for BelCoord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cell
    }
}

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
    pub tile_index: EntityVec<TileClassId, Vec<TileCoord>>,
}

#[derive(Clone, Debug)]
pub struct ExpandedDie {
    tiles: Array2<Cell>,
    pub region_root_cells:
        EntityVec<RegionSlotId, HashMap<(ColId, RowId), HashSet<(ColId, RowId)>>>,
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

    pub fn tile(&self, crd: TileCoord) -> &Tile {
        &self
            .die(crd.cell.die)
            .cell((crd.cell.col, crd.cell.row))
            .tiles[crd.slot]
    }

    pub fn connector(&self, crd: ConnectorCoord) -> &Connector {
        &self
            .die(crd.cell.die)
            .cell((crd.cell.col, crd.cell.row))
            .conns[crd.slot]
    }

    pub fn tile_wire(&self, tcrd: TileCoord, wire: TileWireCoord) -> WireCoord {
        WireCoord {
            cell: self.tile_cell(tcrd, wire.cell),
            slot: wire.wire,
        }
    }

    pub fn resolve_tile_wire_nobuf(
        &self,
        tcrd: TileCoord,
        wire: TileWireCoord,
    ) -> Option<WireCoord> {
        self.resolve_wire_nobuf(self.tile_wire(tcrd, wire))
    }

    pub fn cell(&self, ccrd: CellCoord) -> &Cell {
        self.die(ccrd.die).cell((ccrd.col, ccrd.row))
    }

    pub fn cells(&self) -> impl Iterator<Item = (CellCoord, &Cell)> + '_ {
        self.dies().flat_map(move |die| {
            die.cols().flat_map(move |col| {
                die.rows().map(move |row| {
                    let crd = CellCoord::new(die.die, col, row);
                    let cell = self.cell(crd);
                    (crd, cell)
                })
            })
        })
    }

    pub fn tiles(&self) -> impl Iterator<Item = (TileCoord, &Tile)> + '_ {
        self.cells().flat_map(|(crd, cell)| {
            cell.tiles
                .iter()
                .map(move |(tslot, tile)| (crd.tile(tslot), tile))
        })
    }

    pub fn connectors(&self) -> impl Iterator<Item = (ConnectorCoord, &Connector)> + '_ {
        self.cells().flat_map(|(crd, cell)| {
            cell.conns
                .iter()
                .map(move |(cslot, conn)| (crd.connector(cslot), conn))
        })
    }

    pub fn tile_cells(
        &self,
        tcrd: TileCoord,
    ) -> impl DoubleEndedIterator<Item = (CellSlotId, CellCoord)> + ExactSizeIterator + '_ {
        self.tile(tcrd)
            .cells
            .iter()
            .map(move |(slot, &(col, row))| (slot, tcrd.with_cr(col, row)))
    }

    pub fn tile_cell(&self, tcrd: TileCoord, slot: CellSlotId) -> CellCoord {
        let tile = self.tile(tcrd);
        CellCoord {
            die: tcrd.die,
            col: tile.cells[slot].0,
            row: tile.cells[slot].1,
        }
    }

    pub fn find_tile(&self, ccrd: CellCoord, f: impl Fn(&Tile) -> bool) -> Option<&Tile> {
        let cell = self.cell(ccrd);
        cell.tiles.values().find(|x| f(x))
    }

    pub fn get_tile(&self, tcrd: TileCoord) -> Option<&Tile> {
        self.cell(tcrd.cell).tiles.get(tcrd.slot)
    }

    pub fn cell_delta(&self, mut cell: CellCoord, dx: i32, dy: i32) -> Option<CellCoord> {
        if dx < 0 {
            if cell.col.to_idx() < (-dx) as usize {
                return None;
            }
            cell.col -= (-dx) as usize;
        } else {
            cell.col += dx as usize;
            if cell.col.to_idx() >= self.die(cell.die).cols().len() {
                return None;
            }
        }
        if dy < 0 {
            if cell.row.to_idx() < (-dy) as usize {
                return None;
            }
            cell.row -= (-dy) as usize;
        } else {
            cell.row += dy as usize;
            if cell.row.to_idx() >= self.die(cell.die).rows().len() {
                return None;
            }
        }
        Some(cell)
    }

    pub fn find_tile_by_class(
        &self,
        ccrd: CellCoord,
        f: impl Fn(&str) -> bool,
    ) -> Option<TileCoord> {
        for (slot, val) in &self.cell(ccrd).tiles {
            if f(self.db.tile_classes.key(val.class)) {
                return Some(TileCoord { cell: ccrd, slot });
            }
        }
        None
    }

    // TODO: kill
    pub fn get_tile_by_class(&self, ccrd: CellCoord, f: impl Fn(&str) -> bool) -> TileCoord {
        self.find_tile_by_class(ccrd, f).unwrap()
    }

    pub fn has_bel(&self, bel: BelCoord) -> bool {
        let cell = self.cell(bel.cell);
        let tslot = self.db.bel_slots[bel.slot].tile_slot;
        if let Some(tile) = cell.tiles.get(tslot) {
            let nk = &self.db.tile_classes[tile.class];
            if nk.bels.contains_id(bel.slot) {
                return true;
            }
        }
        false
    }

    pub fn find_tile_by_bel(&self, bel: BelCoord) -> Option<TileCoord> {
        if self.has_bel(bel) {
            Some(self.get_tile_by_bel(bel))
        } else {
            None
        }
    }

    pub fn get_tile_by_bel(&self, bel: BelCoord) -> TileCoord {
        bel.tile(self.db.bel_slots[bel.slot].tile_slot)
    }

    pub fn get_bel_pin(&self, bel: BelCoord, pin: &str) -> Vec<WireCoord> {
        let tcrd = self.get_tile_by_bel(bel);
        let tile = self.tile(tcrd);
        let pin_info = &self.db.tile_classes[tile.class].bels[bel.slot].pins[pin];
        pin_info
            .wires
            .iter()
            .map(|&wire| self.tile_wire(tcrd, wire))
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
                    for (slot, node) in &die[(col, row)].tiles {
                        self.tile_index[node.class].push(TileCoord {
                            cell: CellCoord {
                                die: dieid,
                                col,
                                row,
                            },
                            slot,
                        });
                    }
                }
            }
        }
        #[allow(unexpected_cfgs)]
        if cfg!(self_check_egrid) {
            println!("CHECK");
            for (cell, _) in self.cells() {
                for w in self.db.wires.ids() {
                    let wire = WireCoord { cell, slot: w };
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
                            "tree {rw} does not contain {ow}",
                            rw = rw.to_string(self.db),
                            ow = wire.to_string(self.db)
                        );
                    }
                }
            }
        }
    }
}

impl core::ops::Index<(ColId, RowId)> for ExpandedDie {
    type Output = Cell;
    fn index(&self, xy: (ColId, RowId)) -> &Cell {
        &self.tiles[[xy.1.to_idx(), xy.0.to_idx()]]
    }
}

impl core::ops::IndexMut<(ColId, RowId)> for ExpandedDie {
    fn index_mut(&mut self, xy: (ColId, RowId)) -> &mut Cell {
        &mut self.tiles[[xy.1.to_idx(), xy.0.to_idx()]]
    }
}

impl core::ops::Index<CellCoord> for ExpandedGrid<'_> {
    type Output = Cell;
    fn index(&self, xy: CellCoord) -> &Cell {
        &self.die[xy.die][(xy.col, xy.row)]
    }
}

impl core::ops::IndexMut<CellCoord> for ExpandedGrid<'_> {
    fn index_mut(&mut self, xy: CellCoord) -> &mut Cell {
        &mut self.die[xy.die][(xy.col, xy.row)]
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
    pub fn cell(&self, xy: (ColId, RowId)) -> &'a Cell {
        &self.grid.die[self.die].tiles[[xy.1.to_idx(), xy.0.to_idx()]]
    }
}

impl ExpandedDieRefMut<'_, '_> {
    pub fn add_tile(
        &mut self,
        crd: (ColId, RowId),
        kind: &str,
        cells: &[(ColId, RowId)],
    ) -> &mut Tile {
        let kind = self.grid.db.get_tile_class(kind);
        let cells: EntityVec<_, _> = cells.iter().copied().collect();
        let slot = self.grid.db.tile_classes[kind].slot;
        assert!(!self[crd].tiles.contains_id(slot));
        self[crd].tiles.insert(
            slot,
            Tile {
                class: kind,
                cells: cells.clone(),
            },
        );
        for (cid, ccrd) in cells {
            self[ccrd].tile_index.push((crd, slot, cid))
        }
        &mut self[crd].tiles[slot]
    }

    pub fn fill_tile(&mut self, xy: (ColId, RowId), kind: &str) -> &mut Tile {
        assert!(self[xy].tiles.iter().count() == 0);
        self.add_tile(xy, kind, &[xy])
    }

    pub fn fill_conn_pair(&mut self, a: (ColId, RowId), b: (ColId, RowId), fwd: &str, bwd: &str) {
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

    pub fn fill_conn_term(&mut self, xy: (ColId, RowId), kind: &str) {
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
                if self[(col, row)].tiles.iter().count() == 0 {
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
                if self[(col, row)].tiles.iter().count() == 0 {
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
pub struct TilePip {
    pub wire_out: WireCoord,
    pub wire_in: WireCoord,
    pub wire_out_raw: WireCoord,
    pub wire_in_raw: WireCoord,

    pub tile: TileCoord,
    pub tile_wire_out: TileWireCoord,
    pub tile_wire_in: TileWireCoord,
}

impl ExpandedGrid<'_> {
    pub fn resolve_wire(&self, mut wire: WireCoord) -> Option<WireCoord> {
        loop {
            let cell = self.cell(wire.cell);
            let wi = self.db.wires[wire.slot];
            match wi {
                WireKind::Regional(rslot) => {
                    (wire.cell.col, wire.cell.row) = cell.region_root[rslot];
                    break;
                }
                WireKind::MultiBranch(slot)
                | WireKind::Branch(slot)
                | WireKind::PipBranch(slot) => {
                    if let Some(t) = cell.conns.get(slot) {
                        let ccls = &self.db.conn_classes[t.class];
                        match ccls.wires.get(wire.slot) {
                            Some(&ConnectorWire::BlackHole) => return None,
                            Some(&ConnectorWire::Reflect(wf)) => {
                                wire.slot = wf;
                            }
                            Some(&ConnectorWire::Pass(wf)) => {
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
                    wire.slot = wf;
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
        loop {
            let cell = self.cell(wire.cell);
            let wi = self.db.wires[wire.slot];
            match wi {
                WireKind::Regional(rslot) => {
                    (wire.cell.col, wire.cell.row) = cell.region_root[rslot];
                    break;
                }
                WireKind::MultiBranch(slot)
                | WireKind::Branch(slot)
                | WireKind::PipBranch(slot) => {
                    if let Some(t) = cell.conns.get(slot) {
                        let ccls = &self.db.conn_classes[t.class];
                        match ccls.wires.get(wire.slot) {
                            Some(&ConnectorWire::BlackHole) => return None,
                            Some(&ConnectorWire::Reflect(wf)) => {
                                wire.slot = wf;
                            }
                            Some(&ConnectorWire::Pass(wf)) => {
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
            let die = self.die(wire.cell.die);
            let tile = self.cell(wire.cell);
            res.push(wire);
            if let WireKind::Regional(rslot) = self.db.wires[wire.slot] {
                if tile.region_root[rslot] == (wire.cell.col, wire.cell.row) {
                    for &crd in &die.region_root_cells[rslot][&(wire.cell.col, wire.cell.row)] {
                        if crd != (wire.cell.col, wire.cell.row) {
                            queue.push(wire.cell.with_cr(crd.0, crd.1).wire(wire.slot));
                        }
                    }
                }
            }
            for &wt in &self.db_index.buf_ins[wire.slot] {
                queue.push(wire.cell.wire(wt));
            }
            for (slot, conn) in &tile.conns {
                let oslot = self.db.conn_slots[slot].opposite;
                for &wt in &self.db_index.conn_classes[conn.class].wire_ins_near[wire.slot] {
                    queue.push(wire.cell.wire(wt));
                }
                if let Some(ocrd) = conn.target {
                    let ocrd = wire.cell.with_cr(ocrd.0, ocrd.1);
                    let oconn = &self.cell(ocrd).conns[oslot];
                    for &wt in &self.db_index.conn_classes[oconn.class].wire_ins_far[wire.slot] {
                        queue.push(ocrd.wire(wt));
                    }
                }
            }
        }
        res
    }

    pub fn wire_pips_bwd(&self, wire: WireCoord) -> Vec<TilePip> {
        let mut wires = vec![wire];
        if matches!(
            self.db.wires[wire.slot],
            WireKind::MultiOut
                | WireKind::MultiBranch(_)
                | WireKind::PipOut
                | WireKind::PipBranch(_)
        ) {
            wires = self.wire_tree(wire);
        }
        let mut res = vec![];
        for w in wires {
            for &(crd, tslot, tid) in &self.cell(w.cell).tile_index {
                let tcrd = w.cell.with_cr(crd.0, crd.1).tile(tslot);
                let tile = self.tile(tcrd);
                let tcls = &self.db.tile_classes[tile.class];
                let tcw = TileWireCoord {
                    cell: tid,
                    wire: w.slot,
                };
                if let Some(mux) = tcls.muxes.get(&tcw) {
                    for &tcwi in &mux.ins {
                        let wire_in_raw = self.tile_wire(tcrd, tcwi);
                        if let Some(wire_in) = self.resolve_wire(wire_in_raw) {
                            res.push(TilePip {
                                wire_out: wire,
                                wire_in,
                                wire_out_raw: w,
                                wire_in_raw,
                                tile: tcrd,
                                tile_wire_out: tcw,
                                tile_wire_in: tcwi,
                            });
                        }
                    }
                }
            }
        }
        res
    }

    pub fn wire_pips_fwd(&self, wire: WireCoord) -> Vec<TilePip> {
        let wires = self.wire_tree(wire);
        let mut res = vec![];
        for w in wires {
            for &(crd, tslot, tid) in &self.cell(w.cell).tile_index {
                let tcrd = w.cell.with_cr(crd.0, crd.1).tile(tslot);
                let tile = self.tile(tcrd);
                let tcls = &self.db_index.tile_classes[tile.class];
                let tcw = TileWireCoord {
                    cell: tid,
                    wire: w.slot,
                };
                if let Some(outs) = tcls.mux_ins.get(&tcw) {
                    for &tcwo in outs {
                        let wire_out_raw = self.tile_wire(tcrd, tcwo);
                        if let Some(wire_out) = self.resolve_wire(wire_out_raw) {
                            res.push(TilePip {
                                wire_out,
                                wire_in: wire,
                                wire_out_raw,
                                wire_in_raw: w,
                                tile: tcrd,
                                tile_wire_out: tcwo,
                                tile_wire_in: tcw,
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
    pub tiles: EntityPartVec<TileSlotId, Tile>,
    pub conns: EntityPartVec<ConnectorSlotId, Connector>,
    pub tile_index: Vec<((ColId, RowId), TileSlotId, CellSlotId)>,
    pub region_root: EntityVec<RegionSlotId, (ColId, RowId)>,
}

#[derive(Clone, Debug)]
pub struct Tile {
    pub class: TileClassId,
    pub cells: EntityVec<CellSlotId, (ColId, RowId)>,
}

#[derive(Clone, Debug)]
pub struct Connector {
    pub class: ConnectorClassId,
    pub target: Option<(ColId, RowId)>,
}
