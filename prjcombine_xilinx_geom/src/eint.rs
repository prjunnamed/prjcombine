use crate::int::*;
use crate::{ColId, RowId, SlrId};
use serde::{Serialize, Deserialize};
use enum_map::{EnumMap, enum_map};
use prjcombine_entity::{EntityVec, EntityId, EntityIds};
use ndarray::Array2;

pub type Coord = (ColId, RowId);
pub type IntWire = (SlrId, Coord, WireId);

#[derive(Clone, Debug)]
pub struct ExpandedGrid<'a> {
    pub db: &'a IntDb,
    pub tie_kind: Option<String>,
    pub tie_pin_gnd: Option<String>,
    pub tie_pin_vcc: Option<String>,
    pub tie_pin_pullup: Option<String>,
    pub tiles: EntityVec<SlrId, Array2<Option<ExpandedTile>>>,
}

pub struct ExpandedSlrRef<'a, 'b> {
    pub grid: &'b ExpandedGrid<'a>,
    pub slr: SlrId,
}

pub struct ExpandedSlrRefMut<'a, 'b> {
    pub grid: &'b mut ExpandedGrid<'a>,
    pub slr: SlrId,
}

impl<'a> ExpandedGrid<'a> {
    pub fn slrs<'b>(&'b self) -> impl Iterator<Item=ExpandedSlrRef<'a, 'b>> {
        self.tiles.ids().map(|slr| self.slr(slr))
    }

    pub fn slr<'b>(&'b self, slr: SlrId) -> ExpandedSlrRef<'a, 'b> {
        ExpandedSlrRef {
            grid: self,
            slr,
        }
    }
    pub fn slr_mut<'b>(&'b mut self, slr: SlrId) -> ExpandedSlrRefMut<'a, 'b> {
        ExpandedSlrRefMut {
            grid: self,
            slr,
        }
    }
}

impl core::ops::Index<Coord> for ExpandedSlrRef<'_, '_> {
    type Output = Option<ExpandedTile>;
    fn index(&self, xy: Coord) -> &Option<ExpandedTile> {
        &self.grid.tiles[self.slr][[xy.1.to_idx(), xy.0.to_idx()]]
    }
}

impl core::ops::Index<Coord> for ExpandedSlrRefMut<'_, '_> {
    type Output = Option<ExpandedTile>;
    fn index(&self, xy: Coord) -> &Option<ExpandedTile> {
        &self.grid.tiles[self.slr][[xy.1.to_idx(), xy.0.to_idx()]]
    }
}

impl core::ops::IndexMut<Coord> for ExpandedSlrRefMut<'_, '_> {
    fn index_mut(&mut self, xy: Coord) -> &mut Option<ExpandedTile> {
        &mut self.grid.tiles[self.slr][[xy.1.to_idx(), xy.0.to_idx()]]
    }
}

impl ExpandedSlrRef<'_, '_> {
    pub fn tile(&self, xy: Coord) -> &ExpandedTile {
        self[xy].as_ref().unwrap()
    }

    pub fn rows(&self) -> EntityIds<RowId> {
        EntityIds::new(self.grid.tiles[self.slr].shape()[0])
    }

    pub fn cols(&self) -> EntityIds<ColId> {
        EntityIds::new(self.grid.tiles[self.slr].shape()[1])
    }
}

impl ExpandedSlrRefMut<'_, '_> {
    pub fn tile(&self, xy: Coord) -> &ExpandedTile {
        self[xy].as_ref().unwrap()
    }

    pub fn tile_mut(&mut self, xy: Coord) -> &mut ExpandedTile {
        self[xy].as_mut().unwrap()
    }

    pub fn fill_tile(&mut self, xy: Coord, kind: &str, naming: &str, name: String) {
        assert!(self[xy].is_none());
        self[xy] = Some(ExpandedTile {
            kind: self.grid.db.get_node(kind),
            name,
            tie_name: None,
            naming: self.grid.db.get_naming(naming),
            special: false,
            intf: None,
            dirs: enum_map!(_ => ExpandedTileDir::None),
        });
    }

    pub fn fill_tile_special(&mut self, xy: Coord, kind: &str, naming: &str, name: String) {
        assert!(self[xy].is_none());
        self[xy] = Some(ExpandedTile {
            kind: self.grid.db.get_node(kind),
            name,
            tie_name: None,
            naming: self.grid.db.get_naming(naming),
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
        EntityIds::new(self.grid.tiles[self.slr].shape()[0])
    }

    pub fn cols(&self) -> EntityIds<ColId> {
        EntityIds::new(self.grid.tiles[self.slr].shape()[1])
    }

    pub fn fill_pass_pair(&mut self, fwd: ExpandedTilePass, bwd: ExpandedTilePass) {
        let a = bwd.target;
        let b = fwd.target;
        let dir = self.grid.db.passes[fwd.kind].dir;
        assert_eq!(self.grid.db.passes[bwd.kind].dir, !dir);
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
        let kind = self.grid.db.get_term(kind);
        let naming = self.grid.db.get_naming(naming);
        let naming_in = naming_in.map(|x| self.grid.db.get_naming(x));
        let dir = self.grid.db.terms[kind].dir;
        self.tile_mut(xy).dirs[dir] = ExpandedTileDir::Term(ExpandedTileTerm {
            kind,
            tile: Some(tile),
            naming: Some(naming),
            naming_in,
        });
    }

    pub fn fill_term_anon(&mut self, xy: Coord, kind: &str) {
        let kind = self.grid.db.get_term(kind);
        let dir = self.grid.db.terms[kind].dir;
        self.tile_mut(xy).dirs[dir] = ExpandedTileDir::Term(ExpandedTileTerm {
            kind,
            tile: None,
            naming: None,
            naming_in: None,
        });
    }

    pub fn fill_main_passes(&mut self) {
        let pass_w = self.grid.db.get_pass("MAIN.W");
        let pass_e = self.grid.db.get_pass("MAIN.E");
        let pass_s = self.grid.db.get_pass("MAIN.S");
        let pass_n = self.grid.db.get_pass("MAIN.N");
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

impl ExpandedGrid<'_> {
    pub fn resolve_wire_raw(&self, mut wire: IntWire) -> Option<IntWire> {
        let slr = self.slr(wire.0);
        loop {
            let tile = slr.tile(wire.1);
            let wi = &self.db.wires[wire.2];
            match wi.kind {
                WireKind::MultiBranch(dir) | WireKind::Branch(dir) | WireKind::PipBranch(dir) => {
                    match &tile.dirs[dir] {
                        ExpandedTileDir::Pass(p) => {
                            let pass = &self.db.passes[p.kind];
                            match pass.wires.get(wire.2) {
                                Some(&PassInfo::BlackHole) => return None,
                                Some(&PassInfo::Pass(wf)) => {
                                    match wf {
                                        PassWireIn::Near(wf) => {
                                            if let Some(n) = p.naming_near {
                                                let n = &self.db.namings[n];
                                                if n.contains_id(wf) {
                                                    break;
                                                }
                                            }
                                            wire.2 = wf;
                                        }
                                        PassWireIn::Far(wf) => {
                                            if let Some(n) = p.naming_far {
                                                let n = &self.db.namings[n];
                                                if n.contains_id(wf) {
                                                    break;
                                                }
                                            }
                                            // horrible hack alert
                                            if self.db.nodes.key(slr.tile(p.target).kind) == "DCM.S3.DUMMY" &&
                                                self.db.wires[wf].name.starts_with("OMUX") &&
                                                matches!(self.db.wires[wf].kind, WireKind::MuxOut) {
                                                    break;
                                            }
                                            wire.1 = p.target;
                                            wire.2 = wf;
                                        }
                                    }
                                }
                                _ => break,
                            }
                        },
                        ExpandedTileDir::Term(t) => {
                            let term = &self.db.terms[t.kind];
                            match term.wires.get(wire.2) {
                                Some(&TermInfo::BlackHole) => return None,
                                Some(&TermInfo::Pass(wf)) => {
                                    if let Some(n) = t.naming {
                                        let n = &self.db.namings[n];
                                        if n.contains_id(wire.2) {
                                            break;
                                        }
                                    }
                                    wire.2 = wf;
                                }
                                None => {
                                    // horrible hack alert
                                    if self.db.terms.key(t.kind) == "N.PPC" && self.db.wires[wire.2].name == "IMUX.BYP4.BOUNCE.S" {
                                        wire.2 = WireId::from_idx(wire.2.to_idx() - 14);
                                        assert_eq!(self.db.wires[wire.2].name, "IMUX.BYP0.BOUNCE.S");
                                    }
                                    break;
                                }
                                _ => break,
                            }
                        },
                        _ => break,
                    }
                }
                _ => break,
            }
        }
        Some(wire)
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
