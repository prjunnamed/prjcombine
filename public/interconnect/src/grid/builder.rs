use std::collections::{HashMap, HashSet};

use bimap::BiHashMap;
use prjcombine_entity::{EntityId, EntityVec};

use crate::{
    db::{ConnectorClassId, IntDb, IntDbIndex, TileClassId},
    grid::{Cell, CellCoord, ColId, Connector, DieId, ExpandedDie, ExpandedGrid, RowId, Tile},
};

pub struct GridBuilder<'db> {
    grid: ExpandedGrid<'db>,
}

impl<'db> GridBuilder<'db> {
    pub fn new(db: &'db IntDb) -> Self {
        Self {
            grid: ExpandedGrid {
                db,
                db_index: IntDbIndex::new(db),
                die: EntityVec::new(),
                extra_conns: BiHashMap::new(),
                blackhole_wires: HashSet::new(),
                tile_index: EntityVec::new(),
                region_root_cells: EntityVec::new(),
            },
        }
    }

    pub fn finish(mut self) -> ExpandedGrid<'db> {
        let mut region_root_cells: EntityVec<_, _> =
            self.db.region_slots.ids().map(|_| HashMap::new()).collect();
        for rslot in self.db.region_slots.ids() {
            let mut die_region_root_cells: HashMap<_, HashSet<_>> = HashMap::new();
            for (cell, _) in self.cells() {
                die_region_root_cells
                    .entry(self[cell].region_root[rslot])
                    .or_default()
                    .insert(cell);
            }
            region_root_cells[rslot] = die_region_root_cells;
        }
        self.region_root_cells = region_root_cells;

        let mut tile_index: EntityVec<_, _> = self.db.tile_classes.ids().map(|_| vec![]).collect();
        for (cell, _) in self.cells() {
            for (slot, tile) in &self[cell].tiles {
                tile_index[tile.class].push(cell.tile(slot));
            }
        }
        self.tile_index = tile_index;

        #[allow(unexpected_cfgs)]
        if cfg!(self_check_egrid) {
            println!("CHECK");
            for (cell, _) in self.cells() {
                for w in self.db.wires.ids() {
                    let wire = cell.wire(w);
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

        self.grid
    }

    pub fn add_die(&mut self, width: usize, height: usize) -> DieId {
        let die = self.die.next_id();
        let tiles = (0..width * height)
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
            .collect();
        self.die.push(ExpandedDie {
            width,
            height,
            tiles,
        });
        die
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

    pub fn add_tile_e_id(&mut self, cell: CellCoord, tcid: TileClassId, num: usize) -> &mut Tile {
        self.add_tile_id(cell, tcid, &cell.cells_e(num))
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

    pub fn add_tile_we_id(
        &mut self,
        cell: CellCoord,
        tcid: TileClassId,
        num_w: usize,
        num: usize,
    ) -> &mut Tile {
        self.add_tile_id(cell, tcid, &cell.delta(-(num_w as i32), 0).cells_e(num))
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

    pub fn add_tile_sn_id(
        &mut self,
        cell: CellCoord,
        tcid: TileClassId,
        num_s: usize,
        num: usize,
    ) -> &mut Tile {
        self.add_tile_id(cell, tcid, &cell.delta(0, -(num_s as i32)).cells_n(num))
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
        let fwd_slot = self.db[fwd].slot;
        let bwd_slot = self.db[bwd].slot;
        self[a].conns.insert(
            fwd_slot,
            Connector {
                target: Some(b),
                class: fwd,
            },
        );
        self[b].conns.insert(
            bwd_slot,
            Connector {
                target: Some(a),
                class: bwd,
            },
        );
    }

    pub fn fill_conn_pair(&mut self, a: CellCoord, b: CellCoord, fwd: &str, bwd: &str) {
        let fwd = self.db.get_conn_class(fwd);
        let bwd = self.db.get_conn_class(bwd);
        self.fill_conn_pair_id(a, b, fwd, bwd);
    }

    pub fn fill_conn_term_id(&mut self, xy: CellCoord, ccls: ConnectorClassId) {
        let slot = self.db[ccls].slot;
        self[xy].conns.insert(
            slot,
            Connector {
                class: ccls,
                target: None,
            },
        );
    }

    pub fn fill_conn_term(&mut self, xy: CellCoord, kind: &str) {
        let ccls = self.db.get_conn_class(kind);
        self.fill_conn_term_id(xy, ccls);
    }
}

impl<'db> core::ops::Deref for GridBuilder<'db> {
    type Target = ExpandedGrid<'db>;

    fn deref(&self) -> &Self::Target {
        &self.grid
    }
}

impl core::ops::DerefMut for GridBuilder<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.grid
    }
}
