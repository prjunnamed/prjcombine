use crate::int::*;
use crate::{ColId, RowId};
use serde::{Serialize, Deserialize};
use enum_map::{EnumMap, enum_map};
use prjcombine_entity::{EntityVec, EntityId, EntityIds};
use ndarray::Array2;

#[derive(Clone, Debug)]
pub struct ExpandedGrid<'a> {
    pub db: &'a IntDb,
    pub tie_kind: Option<String>,
    pub tie_pin_gnd: Option<String>,
    pub tie_pin_vcc: Option<String>,
    pub tie_pin_pullup: Option<String>,
    pub tiles: Array2<Option<ExpandedTile>>,
}

impl core::ops::Index<Coord> for ExpandedGrid<'_> {
    type Output = Option<ExpandedTile>;
    fn index(&self, xy: Coord) -> &Option<ExpandedTile> {
        &self.tiles[[xy.1.to_idx(), xy.0.to_idx()]]
    }
}

impl core::ops::IndexMut<Coord> for ExpandedGrid<'_> {
    fn index_mut(&mut self, xy: Coord) -> &mut Option<ExpandedTile> {
        &mut self.tiles[[xy.1.to_idx(), xy.0.to_idx()]]
    }
}

impl ExpandedGrid<'_> {
    pub fn tile(&self, xy: Coord) -> &ExpandedTile {
        self[xy].as_ref().unwrap()
    }

    pub fn tile_mut(&mut self, xy: Coord) -> &mut ExpandedTile {
        self[xy].as_mut().unwrap()
    }

    pub fn fill_tile(&mut self, xy: Coord, kind: &str, naming: &str, name: String) {
        assert!(self[xy].is_none());
        self[xy] = Some(ExpandedTile {
            kind: self.db.get_node(kind),
            name,
            tie_name: None,
            naming: self.db.get_naming(naming),
            special: false,
            intf: None,
            dirs: enum_map!(_ => ExpandedTileDir::None),
        });
    }

    pub fn fill_tile_special(&mut self, xy: Coord, kind: &str, naming: &str, name: String) {
        assert!(self[xy].is_none());
        self[xy] = Some(ExpandedTile {
            kind: self.db.get_node(kind),
            name,
            tie_name: None,
            naming: self.db.get_naming(naming),
            special: true,
            intf: None,
            dirs: enum_map!(_ => ExpandedTileDir::None),
        });
    }

    pub fn nuke_rect(&mut self, x: ColId, y: RowId, w: usize, h: usize) {
        for dx in 0..w {
            for dy in 0..h {
                self[(x + dx, y + dy)] = None;
            }
        }
    }

    pub fn rows(&self) -> EntityIds<RowId> {
        EntityIds::new(self.tiles.shape()[0])
    }

    pub fn cols(&self) -> EntityIds<ColId> {
        EntityIds::new(self.tiles.shape()[1])
    }

    pub fn fill_pass_pair(&mut self, fwd: ExpandedTilePass, bwd: ExpandedTilePass) {
        let a = bwd.target;
        let b = fwd.target;
        let dir = self.db.passes[fwd.kind].dir;
        assert_eq!(self.db.passes[bwd.kind].dir, !dir);
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
        self.tile_mut(a).dirs[dir] = ExpandedTileDir::Pass(fwd);
        self.tile_mut(b).dirs[!dir] = ExpandedTileDir::Pass(bwd);
    }

    pub fn fill_pass_anon(&mut self, a: Coord, b: Coord, fwd: PassKindId, bwd: PassKindId) {
        self.fill_pass_pair(ExpandedTilePass {
            target: b,
            kind: fwd,
            tile: None,
            naming_near: None,
            naming_far: None,
            tile_far: None,
            naming_far_out: None,
            naming_far_in: None,
        }, ExpandedTilePass {
            target: a,
            kind: bwd,
            tile: None,
            naming_near: None,
            naming_far: None,
            tile_far: None,
            naming_far_out: None,
            naming_far_in: None,
        });
    }

    pub fn fill_pass_buf(&mut self, a: Coord, b: Coord, fwd: PassKindId, bwd: PassKindId, tile: String, naming_a: NamingId, naming_b: NamingId) {
        self.fill_pass_pair(ExpandedTilePass {
            target: b,
            kind: fwd,
            tile: Some(tile.clone()),
            naming_near: Some(naming_a),
            naming_far: Some(naming_b),
            tile_far: None,
            naming_far_out: None,
            naming_far_in: None,
        }, ExpandedTilePass {
            target: a,
            kind: bwd,
            tile: Some(tile),
            naming_near: Some(naming_b),
            naming_far: Some(naming_a),
            tile_far: None,
            naming_far_out: None,
            naming_far_in: None,
        });
    }

    pub fn fill_pass_term(&mut self, a: Coord, b: Coord, fwd: PassKindId, bwd: PassKindId, tile_a: String, tile_b: String, naming_a: NamingId, naming_b: NamingId) {
        self.fill_pass_pair(ExpandedTilePass {
            target: b,
            kind: fwd,
            tile: Some(tile_a),
            naming_near: Some(naming_a),
            naming_far: None,
            tile_far: None,
            naming_far_out: None,
            naming_far_in: None,
        }, ExpandedTilePass {
            target: a,
            kind: bwd,
            tile: Some(tile_b),
            naming_near: Some(naming_b),
            naming_far: None,
            tile_far: None,
            naming_far_out: None,
            naming_far_in: None,
        });
    }

    pub fn fill_term_tile(&mut self, xy: Coord, kind: &str, naming: &str, naming_in: Option<&str>, tile: String) {
        let kind = self.db.get_term(kind);
        let naming = self.db.get_naming(naming);
        let naming_in = naming_in.map(|x| self.db.get_naming(x));
        let dir = self.db.terms[kind].dir;
        self.tile_mut(xy).dirs[dir] = ExpandedTileDir::Term(ExpandedTileTerm {
            kind,
            tile: Some(tile),
            naming: Some(naming),
            naming_in,
        });
    }

    pub fn fill_main_passes(&mut self) {
        let pass_w = self.db.get_pass("MAIN.W");
        let pass_e = self.db.get_pass("MAIN.E");
        let pass_s = self.db.get_pass("MAIN.S");
        let pass_n = self.db.get_pass("MAIN.N");
        // horizontal
        for row in self.rows() {
            let mut prev = None;
            for col in self.cols() {
                if self[(col, row)].is_none() {
                    continue;
                }
                if prev.is_some() && matches!(self.tile((col, row)).dirs[Dir::W], ExpandedTileDir::None) {
                    self.fill_pass_anon((prev.unwrap(), row), (col, row), pass_e, pass_w);
                }
                if matches!(self.tile((col, row)).dirs[Dir::E], ExpandedTileDir::None) {
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
                if self[(col, row)].is_none() {
                    continue;
                }
                if prev.is_some() && matches!(self.tile((col, row)).dirs[Dir::S], ExpandedTileDir::None) {
                    self.fill_pass_anon((col, prev.unwrap()), (col, row), pass_n, pass_s);
                }
                if matches!(self.tile((col, row)).dirs[Dir::N], ExpandedTileDir::None) {
                    prev = Some(row);
                } else {
                    prev = None;
                }
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedTile {
    pub kind: NodeKindId,
    pub name: String,
    pub tie_name: Option<String>,
    pub naming: NamingId,
    pub special: bool,
    pub intf: Option<ExpandedTileIntf>,
    pub dirs: EnumMap<Dir, ExpandedTileDir>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedTileIntf {
    pub kind: IntfKindId,
    pub name: String,
    pub naming_int: NamingId,
    pub naming_buf: Option<NamingId>,
    pub naming_site: Option<NamingId>,
    pub naming_delay: Option<NamingId>,
}

pub type Coord = (ColId, RowId);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExpandedTileDir {
    None,
    Term(ExpandedTileTerm),
    Pass(ExpandedTilePass),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedTileTerm {
    pub kind: TermKindId,
    pub tile: Option<String>,
    pub naming: Option<NamingId>,
    pub naming_in: Option<NamingId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedTilePass {
    pub target: Coord,
    pub kind: PassKindId,
    pub tile: Option<String>,
    pub naming_near: Option<NamingId>,
    pub naming_far: Option<NamingId>,
    pub tile_far: Option<String>,
    pub naming_far_out: Option<NamingId>,
    pub naming_far_in: Option<NamingId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedBel {
    pub name: String,
    pub tile_name: String,
    pub tiles: EntityVec<BelTileId, ExpandedBelTile>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedBelTile {
    pub coord: Coord,
    pub naming: (NamingId, NamingId),
    pub int_special_naming: Option<NamingId>,
}
