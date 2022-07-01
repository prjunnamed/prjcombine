use crate::int::*;
use crate::{ColId, RowId, SlrId};
use serde::{Serialize, Deserialize};
use enum_map::{EnumMap, enum_map};
use prjcombine_entity::{EntityVec, EntityId, EntityIds};
use ndarray::Array2;
use std::collections::HashMap;

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
    pub slr_wires: HashMap<IntWire, IntWire>,
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
    pub fn new(db: &'a IntDb) -> Self {
        ExpandedGrid {
            db,
            tie_kind: None,
            tie_pin_gnd: None,
            tie_pin_vcc: None,
            tie_pin_pullup: None,
            tiles: EntityVec::new(),
            slr_wires: HashMap::new(),
        }
    }

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
            intfs: vec![],
            terms: enum_map!(_ => None),
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
            intfs: vec![],
            terms: enum_map!(_ => None),
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
        self.tile_mut(a).terms[dir] = Some(fwd);
        self.tile_mut(b).terms[!dir] = Some(bwd);
    }

    pub fn fill_term_pair_anon(&mut self, a: Coord, b: Coord, fwd: TermKindId, bwd: TermKindId) {
        self.fill_term_pair(ExpandedTileTerm {
            target: Some(b),
            kind: fwd,
            tile: None,
            naming_near: None,
            naming_near_in: None,
            naming_far: None,
            tile_far: None,
            naming_far_out: None,
            naming_far_in: None,
        }, ExpandedTileTerm {
            target: Some(a),
            kind: bwd,
            tile: None,
            naming_near: None,
            naming_near_in: None,
            naming_far: None,
            tile_far: None,
            naming_far_out: None,
            naming_far_in: None,
        });
    }

    pub fn fill_term_pair_buf(&mut self, a: Coord, b: Coord, fwd: TermKindId, bwd: TermKindId, tile: String, naming_a: NamingId, naming_b: NamingId) {
        self.fill_term_pair(ExpandedTileTerm {
            target: Some(b),
            kind: fwd,
            tile: Some(tile.clone()),
            naming_near: Some(naming_a),
            naming_near_in: None,
            naming_far: Some(naming_b),
            tile_far: None,
            naming_far_out: None,
            naming_far_in: None,
        }, ExpandedTileTerm {
            target: Some(a),
            kind: bwd,
            tile: Some(tile),
            naming_near: Some(naming_b),
            naming_near_in: None,
            naming_far: Some(naming_a),
            tile_far: None,
            naming_far_out: None,
            naming_far_in: None,
        });
    }

    pub fn fill_term_pair_bounce(&mut self, a: Coord, b: Coord, fwd: TermKindId, bwd: TermKindId, tile_a: String, tile_b: String, naming_a: NamingId, naming_b: NamingId) {
        self.fill_term_pair(ExpandedTileTerm {
            target: Some(b),
            kind: fwd,
            tile: Some(tile_a),
            naming_near: Some(naming_a),
            naming_near_in: None,
            naming_far: None,
            tile_far: None,
            naming_far_out: None,
            naming_far_in: None,
        }, ExpandedTileTerm {
            target: Some(a),
            kind: bwd,
            tile: Some(tile_b),
            naming_near: Some(naming_b),
            naming_near_in: None,
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
        self.tile_mut(xy).terms[dir] = Some(ExpandedTileTerm {
            target: None,
            kind,
            tile: Some(tile),
            naming_near: Some(naming),
            naming_near_in: naming_in,
            naming_far: None,
            tile_far: None,
            naming_far_out: None,
            naming_far_in: None,
        });
    }

    pub fn fill_term_anon(&mut self, xy: Coord, kind: &str) {
        let kind = self.grid.db.get_term(kind);
        let dir = self.grid.db.terms[kind].dir;
        self.tile_mut(xy).terms[dir] = Some(ExpandedTileTerm {
            target: None,
            kind,
            tile: None,
            naming_near: None,
            naming_near_in: None,
            naming_far: None,
            tile_far: None,
            naming_far_out: None,
            naming_far_in: None,
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
                if self[(col, row)].is_none() {
                    continue;
                }
                if prev.is_some() && self.tile((col, row)).terms[Dir::W].is_none() {
                    self.fill_term_pair_anon((prev.unwrap(), row), (col, row), pass_e, pass_w);
                }
                if self.tile((col, row)).terms[Dir::E].is_none() {
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
                if prev.is_some() && self.tile((col, row)).terms[Dir::S].is_none() {
                    self.fill_term_pair_anon((col, prev.unwrap()), (col, row), pass_n, pass_s);
                }
                if self.tile((col, row)).terms[Dir::N].is_none() {
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
                    if let Some(t) = &tile.terms[dir] {
                        let term = &self.db.terms[t.kind];
                        match term.wires.get(wire.2) {
                            Some(&TermInfo::BlackHole) => return None,
                            Some(&TermInfo::Pass(wf)) => {
                                match wf {
                                    TermWireIn::Near(wf) => {
                                        if let Some(n) = t.naming_near {
                                            let n = &self.db.namings[n];
                                            if let Some(ni) = t.naming_near_in {
                                                let ni = &self.db.namings[ni];
                                                if ni.contains_id(wire.2) {
                                                    break;
                                                }
                                            } else {
                                                if n.contains_id(wf) && n.contains_id(wire.2) {
                                                    break;
                                                }
                                            }
                                        }
                                        wire.2 = wf;
                                    }
                                    TermWireIn::Far(wf) => {
                                        if let Some(nf) = t.naming_far {
                                            let nn = &self.db.namings[t.naming_near.unwrap()];
                                            let nf = &self.db.namings[nf];
                                            if nn.contains_id(wire.2) && nf.contains_id(wf) {
                                                break;
                                            }
                                        }
                                        // horrible hack alert
                                        if self.db.nodes.key(slr.tile(t.target.unwrap()).kind) == "DCM.S3.DUMMY" &&
                                            self.db.wires[wf].name.starts_with("OMUX") &&
                                            matches!(self.db.wires[wf].kind, WireKind::MuxOut) {
                                                break;
                                        }
                                        wire.1 = t.target.unwrap();
                                        wire.2 = wf;
                                    }
                                }
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
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        if let Some(&twire) = self.slr_wires.get(&wire) {
            Some(twire)
        } else {
            Some(wire)
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
    pub intfs: Vec<ExpandedTileIntf>,
    pub terms: EnumMap<Dir, Option<ExpandedTileTerm>>,
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
pub struct ExpandedTileTerm {
    pub target: Option<Coord>,
    pub kind: TermKindId,
    pub tile: Option<String>,
    pub naming_near: Option<NamingId>,
    pub naming_near_in: Option<NamingId>,
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
