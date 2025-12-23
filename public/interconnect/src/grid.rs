#![allow(clippy::too_many_arguments)]

use crate::{db::*, dir::Dir};
use bimap::BiHashMap;
use bincode::{Decode, Encode};
use prjcombine_entity::{
    EntityId, EntityPartVec, EntityVec,
    id::{EntityIdU8, EntityIdU16, EntityRange, EntityTag, EntityTagArith},
};
use std::collections::{HashMap, HashSet};

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

    pub fn wire(self, slot: WireSlotId) -> WireCoord {
        WireCoord { cell: self, slot }
    }

    pub fn bel(self, slot: BelSlotId) -> BelCoord {
        BelCoord { cell: self, slot }
    }

    pub fn connector(self, slot: ConnectorSlotId) -> ConnectorCoord {
        ConnectorCoord { cell: self, slot }
    }

    pub fn delta(self, dx: i32, dy: i32) -> CellCoord {
        self.with_cr(self.col + dx, self.row + dy)
    }

    pub fn cells_e_const<const N: usize>(self) -> [CellCoord; N] {
        core::array::from_fn(|i| self.delta(i as i32, 0))
    }

    pub fn cells_n_const<const N: usize>(self) -> [CellCoord; N] {
        core::array::from_fn(|i| self.delta(0, i as i32))
    }

    pub fn cells_e(self, num: usize) -> Vec<CellCoord> {
        (0..num).map(|i| self.delta(i as i32, 0)).collect()
    }

    pub fn cells_n(self, num: usize) -> Vec<CellCoord> {
        (0..num).map(|i| self.delta(0, i as i32)).collect()
    }

    pub fn rect(self, width: usize, height: usize) -> Rect {
        Rect {
            die: self.die,
            col_w: self.col,
            col_e: self.col + width,
            row_s: self.row,
            row_n: self.row + height,
        }
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
    pub slot: WireSlotId,
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
    pub die: DieId,
    pub col_w: ColId,
    pub col_e: ColId,
    pub row_s: RowId,
    pub row_n: RowId,
}

impl Rect {
    pub fn contains(&self, cell: CellCoord) -> bool {
        cell.die == self.die
            && cell.col >= self.col_w
            && cell.col < self.col_e
            && cell.row >= self.row_s
            && cell.row < self.row_n
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
    pub region_root_cells: EntityVec<RegionSlotId, HashMap<CellCoord, HashSet<CellCoord>>>,
}

#[derive(Clone, Debug)]
pub struct ExpandedDie {
    width: usize,
    height: usize,
    tiles: Vec<Cell>,
}

impl<'a> ExpandedGrid<'a> {
    pub fn new(db: &'a IntDb) -> Self {
        ExpandedGrid {
            db,
            db_index: IntDbIndex::new(db),
            die: EntityVec::new(),
            extra_conns: BiHashMap::new(),
            blackhole_wires: HashSet::new(),
            tile_index: EntityVec::new(),
            region_root_cells: EntityVec::new(),
        }
    }

    pub fn add_die(&mut self, width: usize, height: usize) -> DieId {
        let die = self.die.next_id();
        self.die.push(ExpandedDie {
            width,
            height,
            tiles: (0..width * height)
                .map(|idx| {
                    let c = idx % width;
                    let r = idx / width;
                    Cell {
                        tiles: Default::default(),
                        conns: Default::default(),
                        tile_index: vec![],
                        region_root: self
                            .db
                            .region_slots
                            .ids()
                            .map(|_| CellCoord::new(die, ColId::from_idx(c), RowId::from_idx(r)))
                            .collect(),
                    }
                })
                .collect(),
        });
        die
    }

    pub fn die(&self) -> EntityRange<DieId> {
        self.die.ids()
    }

    pub fn rows(&self, die: DieId) -> EntityRange<RowId> {
        EntityRange::new(0, self.die[die].height)
    }

    pub fn cols(&self, die: DieId) -> EntityRange<ColId> {
        EntityRange::new(0, self.die[die].width)
    }

    pub fn column(
        &self,
        die: DieId,
        col: ColId,
    ) -> impl DoubleEndedIterator<Item = CellCoord> + ExactSizeIterator + 'static {
        self.rows(die).map(move |row| CellCoord::new(die, col, row))
    }

    pub fn row(
        &self,
        die: DieId,
        row: RowId,
    ) -> impl DoubleEndedIterator<Item = CellCoord> + ExactSizeIterator + 'static {
        self.cols(die).map(move |col| CellCoord::new(die, col, row))
    }

    pub fn die_cells(&self, die: DieId) -> impl DoubleEndedIterator<Item = CellCoord> + 'static {
        let num_rows = self.rows(die).len();
        self.cols(die).into_iter().flat_map(move |col| {
            EntityRange::new(0, num_rows).map(move |row| CellCoord::new(die, col, row))
        })
    }

    pub fn tile_wire(&self, tcrd: TileCoord, wire: TileWireCoord) -> WireCoord {
        WireCoord {
            cell: self.tile_cell(tcrd, wire.cell),
            slot: wire.wire,
        }
    }

    pub fn resolve_tile_wire(&self, tcrd: TileCoord, wire: TileWireCoord) -> Option<WireCoord> {
        self.resolve_wire(self.tile_wire(tcrd, wire))
    }

    pub fn cells(&self) -> impl Iterator<Item = (CellCoord, &Cell)> + '_ {
        self.die().into_iter().flat_map(move |die| {
            self.cols(die).into_iter().flat_map(move |col| {
                self.rows(die).map(move |row| {
                    let crd = CellCoord::new(die, col, row);
                    let cell = &self[crd];
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
        self[tcrd]
            .cells
            .iter()
            .map(move |(slot, &cell)| (slot, cell))
    }

    pub fn tile_cell(&self, tcrd: TileCoord, slot: CellSlotId) -> CellCoord {
        self[tcrd].cells[slot]
    }

    pub fn find_tile(&self, ccrd: CellCoord, f: impl Fn(&Tile) -> bool) -> Option<&Tile> {
        self[ccrd].tiles.values().find(|x| f(x))
    }

    pub fn get_tile(&self, tcrd: TileCoord) -> Option<&Tile> {
        self[tcrd.cell].tiles.get(tcrd.slot)
    }

    pub fn cell_delta(&self, mut cell: CellCoord, dx: i32, dy: i32) -> Option<CellCoord> {
        if dx < 0 {
            if cell.col.to_idx() < (-dx) as usize {
                return None;
            }
            cell.col -= (-dx) as usize;
        } else {
            cell.col += dx as usize;
            if cell.col.to_idx() >= self.cols(cell.die).len() {
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
            if cell.row.to_idx() >= self.rows(cell.die).len() {
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
        for (slot, val) in &self[ccrd].tiles {
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
        let tslot = self.db.bel_slots[bel.slot].tile_slot;
        if let Some(tile) = self[bel.cell].tiles.get(tslot) {
            let nk = &self.db[tile.class];
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
        let tile = &self[tcrd];
        let BelInfo::Legacy(ref bel) = self.db[tile.class].bels[bel.slot] else {
            unreachable!()
        };
        let pin_info = &bel.pins[pin];
        pin_info
            .wires
            .iter()
            .map(|&wire| self.tile_wire(tcrd, wire))
            .collect()
    }

    pub fn finish(&mut self) {
        let mut region_root_cells: EntityVec<_, _> =
            self.db.region_slots.ids().map(|_| HashMap::new()).collect();
        for rslot in self.db.region_slots.ids() {
            let mut die_region_root_cells: HashMap<_, HashSet<_>> = HashMap::new();
            for die in self.die() {
                for col in self.cols(die) {
                    for row in self.rows(die) {
                        let cell = CellCoord::new(die, col, row);
                        die_region_root_cells
                            .entry(self[cell].region_root[rslot])
                            .or_default()
                            .insert(cell);
                    }
                }
            }
            region_root_cells[rslot] = die_region_root_cells;
        }
        self.region_root_cells = region_root_cells;

        let mut tile_index: EntityVec<_, _> = self.db.tile_classes.ids().map(|_| vec![]).collect();
        for die in self.die() {
            for col in self.cols(die) {
                for row in self.rows(die) {
                    let cell = CellCoord::new(die, col, row);
                    for (slot, tile) in &self[cell].tiles {
                        tile_index[tile.class].push(TileCoord {
                            cell: CellCoord { die, col, row },
                            slot,
                        });
                    }
                }
            }
        }
        self.tile_index = tile_index;

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
        &self.tiles[xy.1.to_idx() * self.width + xy.0.to_idx()]
    }
}

impl core::ops::IndexMut<(ColId, RowId)> for ExpandedDie {
    fn index_mut(&mut self, xy: (ColId, RowId)) -> &mut Cell {
        &mut self.tiles[xy.1.to_idx() * self.width + xy.0.to_idx()]
    }
}

impl core::ops::Index<CellCoord> for ExpandedGrid<'_> {
    type Output = Cell;
    fn index(&self, cell: CellCoord) -> &Cell {
        &self.die[cell.die][(cell.col, cell.row)]
    }
}

impl core::ops::IndexMut<CellCoord> for ExpandedGrid<'_> {
    fn index_mut(&mut self, cell: CellCoord) -> &mut Cell {
        &mut self.die[cell.die][(cell.col, cell.row)]
    }
}

impl core::ops::Index<TileCoord> for ExpandedGrid<'_> {
    type Output = Tile;
    fn index(&self, tcrd: TileCoord) -> &Tile {
        &self[tcrd.cell].tiles[tcrd.slot]
    }
}

impl core::ops::IndexMut<TileCoord> for ExpandedGrid<'_> {
    fn index_mut(&mut self, tcrd: TileCoord) -> &mut Tile {
        &mut self[tcrd.cell].tiles[tcrd.slot]
    }
}

impl core::ops::Index<ConnectorCoord> for ExpandedGrid<'_> {
    type Output = Connector;
    fn index(&self, ccrd: ConnectorCoord) -> &Connector {
        &self[ccrd.cell].conns[ccrd.slot]
    }
}

impl core::ops::IndexMut<ConnectorCoord> for ExpandedGrid<'_> {
    fn index_mut(&mut self, ccrd: ConnectorCoord) -> &mut Connector {
        &mut self[ccrd.cell].conns[ccrd.slot]
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TilePip {
    pub wire_out: WireCoord,
    pub wire_in: WireCoord,
    pub wire_out_raw: WireCoord,
    pub wire_in_raw: WireCoord,
    pub inv: bool,

    pub tile: TileCoord,
    pub tile_wire_out: TileWireCoord,
    pub tile_wire_in: TileWireCoord,
}

impl ExpandedGrid<'_> {
    pub fn resolve_wire(&self, mut wire: WireCoord) -> Option<WireCoord> {
        loop {
            let cell = &self[wire.cell];
            let wi = self.db[wire.slot];
            match wi {
                WireKind::Regional(rslot) => {
                    wire.cell = cell.region_root[rslot];
                    break;
                }
                WireKind::MultiBranch(slot) | WireKind::Branch(slot) => {
                    if let Some(conn) = cell.conns.get(slot) {
                        let ccls = &self.db[conn.class];
                        match ccls.wires.get(wire.slot) {
                            Some(&ConnectorWire::BlackHole) => return None,
                            Some(&ConnectorWire::Reflect(wf)) => {
                                wire.slot = wf;
                            }
                            Some(&ConnectorWire::Pass(wf)) => {
                                wire.cell = conn.target.unwrap();
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
            let tile = &self[wire.cell];
            res.push(wire);
            if let WireKind::Regional(rslot) = self.db[wire.slot]
                && tile.region_root[rslot] == wire.cell
            {
                for &cell in &self.region_root_cells[rslot][&wire.cell] {
                    if cell != wire.cell {
                        queue.push(cell.wire(wire.slot));
                    }
                }
            }
            for (slot, conn) in &tile.conns {
                let oslot = self.db.conn_slots[slot].opposite;
                for &wt in &self.db_index[conn.class].wire_ins_near[wire.slot] {
                    queue.push(wire.cell.wire(wt));
                }
                if let Some(ocrd) = conn.target {
                    let oconn = &self[ocrd].conns[oslot];
                    for &wt in &self.db_index[oconn.class].wire_ins_far[wire.slot] {
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
            self.db[wire.slot],
            WireKind::MultiOut | WireKind::MultiBranch(_)
        ) {
            wires = self.wire_tree(wire);
        }
        let mut res = vec![];
        for w in wires {
            for &(tcrd, cid) in &self[w.cell].tile_index {
                let tcls = &self.db_index[self[tcrd].class];
                let tcw = TileWireCoord {
                    cell: cid,
                    wire: w.slot,
                };
                if let Some(ins) = tcls.pips_bwd.get(&tcw) {
                    for &tcwi in ins {
                        let wire_in_raw = self.tile_wire(tcrd, tcwi.tw);
                        if let Some(wire_in) = self.resolve_wire(wire_in_raw) {
                            res.push(TilePip {
                                wire_out: wire,
                                wire_in,
                                wire_out_raw: w,
                                wire_in_raw,
                                tile: tcrd,
                                tile_wire_out: tcw,
                                tile_wire_in: tcwi.tw,
                                inv: tcwi.inv,
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
            for &(tcrd, cid) in &self[w.cell].tile_index {
                let tcls = &self.db_index[self[tcrd].class];
                let tcw = TileWireCoord {
                    cell: cid,
                    wire: w.slot,
                };
                if let Some(outs) = tcls.pips_fwd.get(&tcw) {
                    for &tcwo in outs {
                        let wire_out_raw = self.tile_wire(tcrd, tcwo.tw);
                        if let Some(wire_out) = self.resolve_wire(wire_out_raw) {
                            res.push(TilePip {
                                wire_out,
                                wire_in: wire,
                                wire_out_raw,
                                wire_in_raw: w,
                                tile: tcrd,
                                tile_wire_out: tcwo.tw,
                                tile_wire_in: tcw,
                                inv: tcwo.inv,
                            });
                        }
                    }
                }
            }
        }
        res
    }

    pub fn add_tile_id(
        &mut self,
        anchor: CellCoord,
        tcid: TileClassId,
        cells: &[CellCoord],
    ) -> &mut Tile {
        let tcls = &self.db[tcid];
        let tcrd = anchor.tile(tcls.slot);
        let cells: EntityVec<_, _> = cells.iter().copied().collect();
        assert_eq!(cells.len(), tcls.cells.len());
        let slot = tcls.slot;
        assert!(!self[anchor].tiles.contains_id(slot));
        self[anchor].tiles.insert(
            slot,
            Tile {
                class: tcid,
                cells: cells.clone(),
            },
        );
        for (cid, cell) in cells {
            self[cell].tile_index.push((tcrd, cid))
        }
        &mut self[anchor].tiles[slot]
    }

    pub fn add_tile(&mut self, anchor: CellCoord, kind: &str, cells: &[CellCoord]) -> &mut Tile {
        self.add_tile_id(anchor, self.db.get_tile_class(kind), cells)
    }

    pub fn add_tile_single_id(&mut self, cell: CellCoord, tcid: TileClassId) -> &mut Tile {
        self.add_tile_id(cell, tcid, &[cell])
    }

    pub fn add_tile_single(&mut self, cell: CellCoord, kind: &str) -> &mut Tile {
        self.add_tile(cell, kind, &[cell])
    }

    pub fn add_tile_e(&mut self, cell: CellCoord, kind: &str, num: usize) -> &mut Tile {
        self.add_tile(cell, kind, &cell.cells_e(num))
    }

    pub fn add_tile_n_id(&mut self, cell: CellCoord, tcid: TileClassId, num: usize) -> &mut Tile {
        self.add_tile_id(cell, tcid, &cell.cells_n(num))
    }

    pub fn add_tile_n(&mut self, cell: CellCoord, kind: &str, num: usize) -> &mut Tile {
        self.add_tile(cell, kind, &cell.cells_n(num))
    }

    pub fn add_tile_we(
        &mut self,
        cell: CellCoord,
        kind: &str,
        num_w: usize,
        num: usize,
    ) -> &mut Tile {
        self.add_tile(cell, kind, &cell.delta(-(num_w as i32), 0).cells_e(num))
    }

    pub fn add_tile_sn(
        &mut self,
        cell: CellCoord,
        kind: &str,
        num_s: usize,
        num: usize,
    ) -> &mut Tile {
        self.add_tile(cell, kind, &cell.delta(0, -(num_s as i32)).cells_n(num))
    }

    pub fn fill_conn_pair_id(
        &mut self,
        a: CellCoord,
        b: CellCoord,
        fwd: ConnectorClassId,
        bwd: ConnectorClassId,
    ) {
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
        let fwd_slot = this.db[fwd.class].slot;
        let bwd_slot = this.db[bwd.class].slot;
        this[a].conns.insert(fwd_slot, fwd);
        this[b].conns.insert(bwd_slot, bwd);
    }

    pub fn fill_conn_pair(&mut self, a: CellCoord, b: CellCoord, fwd: &str, bwd: &str) {
        let fwd = self.db.get_conn_class(fwd);
        let bwd = self.db.get_conn_class(bwd);
        self.fill_conn_pair_id(a, b, fwd, bwd);
    }

    pub fn fill_conn_term(&mut self, xy: CellCoord, kind: &str) {
        let ccls = self.db.get_conn_class(kind);
        let slot = self.db[ccls].slot;
        self[xy].conns.insert(
            slot,
            Connector {
                class: ccls,
                target: None,
            },
        );
    }

    pub fn fill_main_passes(&mut self, die: DieId) {
        let pass_w = "MAIN.W";
        let pass_e = "MAIN.E";
        let pass_s = "MAIN.S";
        let pass_n = "MAIN.N";
        let slot_w = self.db.get_conn_slot("W");
        let slot_e = self.db.get_conn_slot("E");
        let slot_s = self.db.get_conn_slot("S");
        let slot_n = self.db.get_conn_slot("N");
        // horizontal
        for row in self.rows(die) {
            let mut prev = None;
            for cell in self.row(die, row) {
                if self[cell].tiles.iter().count() == 0 {
                    continue;
                }
                if let Some(prev) = prev
                    && !self[cell].conns.contains_id(slot_w)
                {
                    self.fill_conn_pair(prev, cell, pass_e, pass_w);
                }
                if !self[cell].conns.contains_id(slot_e) {
                    prev = Some(cell);
                } else {
                    prev = None;
                }
            }
        }
        // vertical
        for col in self.cols(die) {
            let mut prev = None;
            for cell in self.column(die, col) {
                if self[cell].tiles.iter().count() == 0 {
                    continue;
                }
                if let Some(prev) = prev
                    && !self[cell].conns.contains_id(slot_s)
                {
                    self.fill_conn_pair(prev, cell, pass_n, pass_s);
                }
                if !self[cell].conns.contains_id(slot_n) {
                    prev = Some(cell);
                } else {
                    prev = None;
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Cell {
    pub tiles: EntityPartVec<TileSlotId, Tile>,
    pub conns: EntityPartVec<ConnectorSlotId, Connector>,
    pub tile_index: Vec<(TileCoord, CellSlotId)>,
    pub region_root: EntityVec<RegionSlotId, CellCoord>,
}

#[derive(Clone, Debug)]
pub struct Tile {
    pub class: TileClassId,
    pub cells: EntityVec<CellSlotId, CellCoord>,
}

#[derive(Clone, Debug)]
pub struct Connector {
    pub class: ConnectorClassId,
    pub target: Option<CellCoord>,
}
