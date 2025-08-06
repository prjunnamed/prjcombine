use std::{
    collections::{HashMap, hash_map},
    ops::Range,
};

use prjcombine_interconnect::{
    db::{BelSlotId, TileWireCoord},
    grid::{BelCoord, ExpandedGrid, TileCoord},
};
use unnamed_entity::{EntityPartVec, EntityVec};

use crate::db::{IntPipNaming, NamingDb, TileNamingId, TileRawCellId};

#[derive(Clone, Debug)]
pub struct ExpandedGridNaming<'a> {
    pub db: &'a NamingDb,
    pub egrid: &'a ExpandedGrid<'a>,
    pub tiles: HashMap<TileCoord, GridTileNaming>,
    pub tie_pin_gnd: Option<String>,
}

#[derive(Clone, Debug)]
pub struct GridTileNaming {
    pub naming: TileNamingId,
    pub coords: EntityVec<TileRawCellId, (Range<usize>, Range<usize>)>,
    pub tie_names: Vec<String>,
    pub bels: EntityPartVec<BelSlotId, Vec<String>>,
}

impl GridTileNaming {
    pub fn add_bel(&mut self, slot: BelSlotId, names: Vec<String>) {
        self.bels.insert(slot, names);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PipCoords {
    Pip((usize, usize)),
    BoxPip((usize, usize), (usize, usize)),
}

impl<'a> ExpandedGridNaming<'a> {
    pub fn new(db: &'a NamingDb, egrid: &'a ExpandedGrid<'a>) -> Self {
        ExpandedGridNaming {
            db,
            egrid,
            tie_pin_gnd: None,
            tiles: HashMap::new(),
        }
    }

    pub fn name_tile(
        &mut self,
        tcrd: TileCoord,
        naming: &str,
        coords: impl IntoIterator<Item = (Range<usize>, Range<usize>)>,
    ) -> &mut GridTileNaming {
        let ntile = GridTileNaming {
            coords: coords.into_iter().collect(),
            naming: self.db.get_tile_naming(naming),
            tie_names: vec![],
            bels: EntityPartVec::new(),
        };
        let hash_map::Entry::Vacant(entry) = self.tiles.entry(tcrd) else {
            unreachable!()
        };
        entry.insert(ntile)
    }

    pub fn get_bel_name(&self, bel: BelCoord) -> Option<&str> {
        if let Some(tcrd) = self.egrid.find_tile_by_bel(bel) {
            let ntile = &self.tiles[&tcrd];
            Some(&ntile.bels[bel.slot][0])
        } else {
            None
        }
    }

    pub fn bel_pip(&self, bel: BelCoord, key: &str) -> PipCoords {
        let tcrd = self.egrid.get_tile_by_bel(bel);
        let ntile = &self.tiles[&tcrd];
        let naming = &self.db.tile_namings[ntile.naming].bel_pips[&(bel.slot, key.to_string())];
        PipCoords::Pip((
            naming.x + ntile.coords[naming.rt].0.start,
            naming.y + ntile.coords[naming.rt].1.start,
        ))
    }

    pub fn int_pip(
        &self,
        tcrd: TileCoord,
        wire_to: TileWireCoord,
        wire_from: TileWireCoord,
    ) -> PipCoords {
        let ntile = &self.tiles[&tcrd];
        let naming = &self.db.tile_namings[ntile.naming].int_pips[&(wire_to, wire_from)];
        match naming {
            IntPipNaming::Pip(pip) => PipCoords::Pip((
                pip.x + ntile.coords[pip.rt].0.start,
                pip.y + ntile.coords[pip.rt].1.start,
            )),
            IntPipNaming::Box(pip1, pip2) => PipCoords::BoxPip(
                (
                    pip1.x + ntile.coords[pip1.rt].0.start,
                    pip1.y + ntile.coords[pip1.rt].1.start,
                ),
                (
                    pip2.x + ntile.coords[pip2.rt].0.start,
                    pip2.y + ntile.coords[pip2.rt].1.start,
                ),
            ),
        }
    }
}
