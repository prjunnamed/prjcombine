use std::collections::{HashSet, HashMap};

use crate::namevec::NameVec;
use crate::xilinx::geomdb::{GeomDb, Grid, Part, Dir, Orient, TCWire, TileMux, TileMultiMux, TileTran, SiteSlot};
use crate::xilinx::geomdb::builder::GeomDbBuilder;
use crate::xilinx::geomraw::{GeomRaw, ExtractClass};
use super::cfg::{GeomBuilderConfig, IntTermInfo, IntBufInfo, IntDoubleBufInfo, RawSiteSlot};
use super::part::PartBuilder;

pub struct GeomIntBuilder {
    pub tslot_int: usize,
    pub tslot_int_h: Option<usize>,
    pub tslot_int_v: Option<usize>,
    pub pslot_int_n: usize,
    pub pslot_int_s: usize,
    pub pslot_int_e: usize,
    pub pslot_int_w: usize,
    pub pcls_int_n_pass: usize,
    pub pcls_int_s_pass: usize,
    pub pcls_int_e_pass: usize,
    pub pcls_int_w_pass: usize,
    pub pcls_buf: HashMap<&'static str, (usize, usize)>,
    pub pcls_dbuf: HashMap<&'static str, (usize, usize)>,
    pub tcls_dbuf: HashMap<&'static str, (usize, usize)>,
    pub pcls_pass: HashMap<&'static str, (usize, usize)>,
}

pub struct ExtractClassTmp {
    // one per raw tile involved
    pub wire_map: Vec<HashMap<String, TCWire>>,
    pub site_slots: Vec<(RawSiteSlot, usize)>,
}

pub struct ExtractInfo {
    pub cls: ExtractClass,
    pub wire_map: Vec<HashMap<String, TCWire>>,
    pub site_slots: Vec<(RawSiteSlot, usize)>,
    pub tile_muxes: Vec<TileMux>,
    pub tile_multimuxes: Vec<TileMultiMux>,
    pub tile_trans: Vec<TileTran>,
    pub tile_sites: Vec<SiteSlot>,
}

pub struct GeomBuilder {
    pub geomdb: GeomDb,
    pub raw: GeomRaw,
    pub cfg: GeomBuilderConfig,
    pub int: Option<GeomIntBuilder>,
    pub int_wires: HashSet<usize>,
    pub int_out_wires: HashSet<usize>,
    // raw_tile -> raw wire -> (delta, wire idx)
    pub int_wiremap: HashMap<String, usize>,
    pub wiremap: HashMap<String, HashMap<String, ((usize, usize), usize)>>,
    pub extract_tmp: Vec<ExtractClassTmp>,
    // rt kind -> raw wire -> list of raw wires
    pub transparent_pips: HashMap<String, HashMap<String, HashSet<String>>>,
    // XXX int if wire stuff
    // XXX set of extracted raw pips, sites I guess
    pub filled_tcls: HashSet<usize>,
    pub filled_pcls: HashSet<usize>,
}

impl GeomBuilder {
    pub fn new(family: String, cfg: GeomBuilderConfig) -> Self {
        GeomBuilder {
            geomdb: GeomDb {
                name: family,
                vert_bus: NameVec::new(),
                horiz_bus: NameVec::new(),
                wires: NameVec::new(),
                port_slots: NameVec::new(),
                ports: NameVec::new(),
                tile_slots: NameVec::new(),
                tiles: NameVec::new(),
                grids: NameVec::new(),
                parts: NameVec::new(),
            },
            raw: GeomRaw {
                extracts: NameVec::new(),
                port_extracts: NameVec::new(),
                parts: Vec::new(),
            },
            cfg,
            int: None,
            int_wires: HashSet::new(),
            int_out_wires: HashSet::new(),
            int_wiremap: HashMap::new(),
            wiremap: HashMap::new(),
            extract_tmp: Vec::new(),
            transparent_pips: HashMap::new(),
            filled_tcls: HashSet::new(),
            filled_pcls: HashSet::new(),
        }
    }

    pub fn get_int(&self) -> &GeomIntBuilder {
        self.int.as_ref().unwrap()
    }

    pub fn setup_int(&mut self) {
        // Main interconnect tile.
        let tslot_int = self.geomdb.make_tile_slot("INT");

        let mut vec_tcls = Vec::new();
        for t in self.cfg.int_tiles.iter() {
            if !vec_tcls.contains(&t.name) {
                vec_tcls.push(t.name);
            }
        }
        for k in vec_tcls {
            self.geomdb.make_tile_single(&k, tslot_int);
        }

        // The ports.
        let pslot_int_n = self.geomdb.make_port_slot("INT_N");
        let pslot_int_s = self.geomdb.make_port_slot("INT_S");
        let pslot_int_e = self.geomdb.make_port_slot("INT_E");
        let pslot_int_w = self.geomdb.make_port_slot("INT_W");
        let (pcls_int_n_pass, pcls_int_s_pass) = self.geomdb.make_port_pair(("INT_N_PASS", "INT_S_PASS"), (pslot_int_n, pslot_int_s));
        let (pcls_int_e_pass, pcls_int_w_pass) = self.geomdb.make_port_pair(("INT_E_PASS", "INT_W_PASS"), (pslot_int_e, pslot_int_w));
        // Those are not extracted, but filled via connect_int_wire — mark them finished already.
        self.mark_pcls_filled(pcls_int_n_pass);
        self.mark_pcls_filled(pcls_int_s_pass);
        self.mark_pcls_filled(pcls_int_e_pass);
        self.mark_pcls_filled(pcls_int_w_pass);

        // Create tile slots for terms/buffers if necessary.
        let mut need_tile_h = false;
        let mut need_tile_v = false;
        for term in self.cfg.int_terms.iter() {
            if term.needs_tile {
                match term.dir {
                    Dir::E | Dir::W => need_tile_h = true,
                    Dir::N | Dir::S => need_tile_v = true,
                }
            }
        }
        for buf in self.cfg.int_bufs.iter() {
            if buf.needs_tile {
                match buf.orient {
                    Orient::H => need_tile_h = true,
                    Orient::V => need_tile_v = true,
                }
            }
        }
        for dbuf in self.cfg.int_dbufs.iter() {
            if dbuf.needs_tile {
                match dbuf.orient {
                    Orient::H => need_tile_h = true,
                    Orient::V => need_tile_v = true,
                }
            }
        }
        let tslot_int_h = if need_tile_h {
            Some(self.geomdb.make_tile_slot("INT_H"))
        } else {
            None
        };
        let tslot_int_v = if need_tile_v {
            Some(self.geomdb.make_tile_slot("INT_V"))
        } else {
            None
        };

        // Terms.
        let mut vec_term_tcls = Vec::new();
        let mut map_term_tcls: HashMap<&'static str, Vec<&IntTermInfo>> = HashMap::new();
        for term in self.cfg.int_terms.iter() {
            match map_term_tcls.get_mut(&term.name) {
                Some(v) => v.push(term),
                None => {
                    vec_term_tcls.push(term.name);
                    map_term_tcls.insert(term.name, vec![term]);
                }
            }
        }
        for name in vec_term_tcls {
            let terms = &map_term_tcls[&name];
            let dir = terms[0].dir;
            let needs_tile = terms[0].needs_tile;
            let (pslot, tslot) = match dir {
                Dir::E => (pslot_int_e, tslot_int_h),
                Dir::W => (pslot_int_w, tslot_int_h),
                Dir::N => (pslot_int_n, tslot_int_v),
                Dir::S => (pslot_int_s, tslot_int_v),
            };
            for term in terms {
                assert!(term.dir == dir && term.needs_tile == needs_tile);
            }
            let mut raw = Vec::new();
            for term in terms {
                match term.span {
                    None => raw.push(term.raw_tile.to_string()),
                    Some((n, _)) => for i in 0..n {
                        raw.push(format!("{}[{}]", term.raw_tile, i));
                    },
                }
            }
            self.geomdb.make_port_term(name, pslot);
            if needs_tile {
                self.geomdb.make_tile_single(name, tslot.unwrap());
            }
        }

        // Buffers.
        let mut pcls_buf: HashMap<&'static str, (usize, usize)> = HashMap::new();
        let mut vec_buf_tcls = Vec::new();
        let mut map_buf_tcls: HashMap<&'static str, Vec<&IntBufInfo>> = HashMap::new();
        for buf in self.cfg.int_bufs.iter() {
            match map_buf_tcls.get_mut(&buf.name) {
                Some(v) => v.push(buf),
                None => {
                    vec_buf_tcls.push(buf.name);
                    map_buf_tcls.insert(buf.name, vec![buf]);
                }
            }
        }
        for name in vec_buf_tcls {
            let bufs = &map_buf_tcls[&name];
            let orient = bufs[0].orient;
            let needs_tile = bufs[0].needs_tile;
            let (pslot_a, pslot_b, tslot, a, b) = match orient {
                Orient::H => (pslot_int_e, pslot_int_w, tslot_int_h, "E", "W"),
                Orient::V => (pslot_int_n, pslot_int_s, tslot_int_v, "N", "S"),
            };
            for buf in bufs {
                assert!(buf.orient == orient && buf.needs_tile == needs_tile);
            }
            let mut raw = Vec::new();
            for buf in bufs {
                match buf.span {
                    None => raw.push(buf.raw_tile.to_string()),
                    Some((n, _)) => for i in 0..n {
                        raw.push(format!("{}[{}]", buf.raw_tile, i));
                    },
                }
            }
            let name_a = format!("{}_{}", name, a);
            let name_b = format!("{}_{}", name, b);
            pcls_buf.insert(name, self.geomdb.make_port_pair((&name_a, &name_b), (pslot_a, pslot_b)));
            if needs_tile {
                self.geomdb.make_tile_single(name, tslot.unwrap());
            }
        }

        // Double buffers.
        let mut pcls_dbuf: HashMap<&'static str, (usize, usize)> = HashMap::new();
        let mut tcls_dbuf: HashMap<&'static str, (usize, usize)> = HashMap::new();
        let mut vec_dbuf_tcls = Vec::new();
        let mut map_dbuf_tcls: HashMap<&'static str, Vec<&IntDoubleBufInfo>> = HashMap::new();
        for dbuf in self.cfg.int_dbufs.iter() {
            match map_dbuf_tcls.get_mut(&dbuf.name) {
                Some(v) => v.push(dbuf),
                None => {
                    vec_dbuf_tcls.push(dbuf.name);
                    map_dbuf_tcls.insert(dbuf.name, vec![dbuf]);
                }
            }
        }
        for name in vec_dbuf_tcls {
            let dbufs = &map_dbuf_tcls[&name];
            let orient = dbufs[0].orient;
            let needs_tile = dbufs[0].needs_tile;
            let (pslot_a, pslot_b, tslot, a, b) = match orient {
                Orient::H => (pslot_int_e, pslot_int_w, tslot_int_h, "E", "W"),
                Orient::V => (pslot_int_n, pslot_int_s, tslot_int_v, "N", "S"),
            };
            for dbuf in dbufs {
                assert!(dbuf.orient == orient && dbuf.needs_tile == needs_tile);
            }
            let mut raw = Vec::new();
            for dbuf in dbufs {
                match dbuf.span {
                    None => raw.push(format!("{}.{}", dbuf.raw_tile_a, dbuf.raw_tile_b)),
                    Some((n, _)) => for i in 0..n {
                        raw.push(format!("{}.{}[{}]", dbuf.raw_tile_a, dbuf.raw_tile_b, i));
                    },
                }
            }
            let name_a = format!("{}_{}", name, a);
            let name_b = format!("{}_{}", name, b);
            pcls_dbuf.insert(name, self.geomdb.make_port_pair((&name_a, &name_b), (pslot_a, pslot_b)));
            if needs_tile {
                let tcls_a = self.geomdb.make_tile_single(&name_a, tslot.unwrap());
                let tcls_b = self.geomdb.make_tile_single(&name_b, tslot.unwrap());
                tcls_dbuf.insert(name, (tcls_a, tcls_b));
            }
        }

        // Passes.
        let mut pcls_pass: HashMap<&'static str, (usize, usize)> = HashMap::new();
        let mut vec_pass_tcls = Vec::new();
        let mut map_pass_tcls: HashMap<&'static str, Orient> = HashMap::new();
        for pass in self.cfg.int_passes.iter() {
            if pass.empty {
                continue;
            }
            match map_pass_tcls.get(&pass.name) {
                Some(&v) => assert_eq!(v, pass.orient),
                None => {
                    vec_pass_tcls.push(pass.name);
                    map_pass_tcls.insert(pass.name, pass.orient);
                }
            }
        }
        for (src, &res) in self.cfg.int_pass_combine.iter() {
            let mut orient = None;
            for s in src {
                match orient {
                    None => orient = Some(map_pass_tcls[s]),
                    Some(cur) => assert!(cur == map_pass_tcls[s]),
                }
            }
            match map_pass_tcls.get(res) {
                Some(&v) => assert_eq!(v, orient.unwrap()),
                None => {
                    vec_pass_tcls.push(res);
                    map_pass_tcls.insert(res, orient.unwrap());
                }
            }
        }
        for name in vec_pass_tcls {
            let orient = map_pass_tcls[&name];
            let (pslot_a, pslot_b, a, b) = match orient {
                Orient::H => (pslot_int_e, pslot_int_w, "E", "W"),
                Orient::V => (pslot_int_n, pslot_int_s, "N", "S"),
            };
            let name_a = format!("{}_{}", name, a);
            let name_b = format!("{}_{}", name, b);
            pcls_pass.insert(name, self.geomdb.make_port_pair((&name_a, &name_b), (pslot_a, pslot_b)));
        }

        self.int = Some(GeomIntBuilder {
            tslot_int,
            tslot_int_h,
            tslot_int_v,
            pslot_int_n,
            pslot_int_s,
            pslot_int_e,
            pslot_int_w,
            pcls_int_n_pass,
            pcls_int_s_pass,
            pcls_int_e_pass,
            pcls_int_w_pass,
            pcls_buf,
            pcls_dbuf,
            tcls_dbuf,
            pcls_pass,
        });
    }

    pub fn setup_tiles(&mut self) {
        let mut slots = Vec::new();
        for t in self.cfg.tiles.iter() {
            if !slots.contains(&t.slot) {
                slots.push(t.slot);
            }
        }
        for slot in slots {
            self.geomdb.make_tile_slot(slot);
        }
        for t in self.cfg.tiles.iter() {
            let tslot = self.geomdb.tile_slots.idx(t.slot);
            let cells = t.cells.iter().copied().map(|(x, y)| (x, y, tslot)).collect::<Vec<_>>();
            if let Some(tcls) = self.geomdb.tiles.get_idx(t.name) {
                let tile = &mut self.geomdb.tiles[tcls];
                assert_eq!(cells, tile.cells);
            } else {
                self.geomdb.make_tile(t.name, &cells[..]);
            }
        }
    }

    pub fn register_int_wire(&mut self, wire: usize, raw_names: &[&str]) {
        self.int_wires.insert(wire);
        for n in raw_names.iter().copied() {
            if n.is_empty() {
                continue;
            }
            assert!(!self.int_wiremap.contains_key(n));
            self.int_wiremap.insert(n.to_string(), wire);
        }
    }

    pub fn register_int_out_wire(&mut self, wire: usize) {
        self.int_out_wires.insert(wire);
    }

    pub fn register_tile_wire(&mut self, wire: usize, raw_tiles: &[&str], raw_name: &str, delta: (usize, usize)) {
        for n in raw_tiles.iter().copied() {
            let twm = self.wiremap.entry(n.to_string()).or_default();
            assert!(!twm.contains_key(raw_name));
            twm.insert(raw_name.to_string(), (delta, wire));
        }
    }

    pub fn connect_int_wire(&mut self, wire: usize, from_dir: Dir, from_wire: usize) {
        let int = self.get_int();
        let (pcls_down, pcls_up) = match from_dir {
            Dir::W => (int.pcls_int_w_pass, int.pcls_int_e_pass),
            Dir::E => (int.pcls_int_e_pass, int.pcls_int_w_pass),
            Dir::S => (int.pcls_int_s_pass, int.pcls_int_n_pass),
            Dir::N => (int.pcls_int_n_pass, int.pcls_int_s_pass),
        };
        self.geomdb.make_simple_pconn(wire, from_wire, pcls_down, pcls_up);
    }

    pub fn make_int_wire(&mut self, name: &str, cls: &str, raw_names: &[&str]) -> usize {
        let res = self.geomdb.make_wire(name, cls, false);
        self.register_int_wire(res, raw_names);
        res
    }

    pub fn make_int_out_wire(&mut self, name: &str, cls: &str, raw_names: &[&str]) -> usize {
        let res = self.geomdb.make_wire(name, cls, false);
        self.register_int_wire(res, raw_names);
        self.register_int_out_wire(res);
        res
    }

    pub fn make_int_wire_cont(&mut self, name: &str, cls: &str, raw_names: &[&str], from_dir: Dir, from_wire: usize) -> usize {
        let res = self.geomdb.make_wire(name, cls, false);
        self.register_int_wire(res, raw_names);
        self.connect_int_wire(res, from_dir, from_wire);
        res
    }

    pub fn make_tile_wire(&mut self, name: &str, cls: &str, raw_tiles: &[&str], raw_name: &str, delta: (usize, usize)) -> usize {
        let res = self.geomdb.make_wire(name, cls, false);
        self.register_tile_wire(res, raw_tiles, raw_name, delta);
        res
    }

    pub fn mark_pcls_filled(&mut self, pcls: usize) {
        self.filled_pcls.insert(pcls);
    }

    pub fn extract(&mut self, info: ExtractInfo) -> usize {
        let tcls = info.cls.tcls;
        if !self.filled_tcls.contains(&tcls) {
            let tile = &mut self.geomdb.tiles[tcls];
            tile.muxes = info.tile_muxes;
            tile.multimuxes = info.tile_multimuxes;
            tile.trans = info.tile_trans;
            tile.sites = info.tile_sites;
            self.filled_tcls.insert(tcls);
        } else {
            let tile = &self.geomdb.tiles[tcls];
            assert_eq!(tile.muxes, info.tile_muxes);
            assert_eq!(tile.multimuxes, info.tile_multimuxes);
            assert_eq!(tile.trans, info.tile_trans);
            assert_eq!(tile.sites, info.tile_sites);
        }
        match self.raw.extracts.get_idx(&info.cls.name) {
            None => {
                let idx = self.raw.extracts.push(info.cls);
                self.extract_tmp.push(ExtractClassTmp {
                    wire_map: info.wire_map,
                    site_slots: info.site_slots,
                });
                idx
            },
            Some(idx) => {
                assert_eq!(self.raw.extracts[idx], info.cls);
                let tmp = &mut self.extract_tmp[idx];
                assert_eq!(tmp.wire_map.len(), info.wire_map.len());
                for (twm, iwm) in tmp.wire_map.iter_mut().zip(info.wire_map.into_iter()) {
                    for (rw, w) in iwm {
                        match twm.get(&rw) {
                            None => {twm.insert(rw, w);},
                            Some(&tw) => assert_eq!(tw, w),
                        }
                    }
                }
                assert_eq!(tmp.site_slots, info.site_slots);
                idx
            },
        }
    }

    pub fn ingest(&mut self, part: PartBuilder) {
        println!("PART {} GRID {} DIMS {}×{}", part.rd.part, part.grid.name, part.width(), part.height());
        let gidx = match self.geomdb.grids.get_idx(&part.grid.name) {
            None => self.geomdb.grids.push(part.grid),
            Some(idx) => {
                check_equiv_grids(&self.geomdb.grids[idx], &part.grid);
                idx
            }
        };
        match self.geomdb.parts.get_idx(&part.rd.part) {
            None => {
                self.geomdb.parts.push(Part {
                    name: part.rd.part.clone(),
                    grid: gidx,
                });
                self.raw.parts.push(part.raw);
            },
            Some(_idx) => {
                // XXX merge or sth
            },
        }
    }

    pub fn finish(self) -> (GeomDb, GeomRaw) {
        for i in 0..self.geomdb.tiles.len() {
            if !self.filled_tcls.contains(&i) {
                println!("WARNING: unfinished tile class {}", self.geomdb.tiles[i].name);
            }
        }
        for i in 0..self.geomdb.ports.len() {
            if !self.filled_pcls.contains(&i) {
                println!("WARNING: unfinished port class {}", self.geomdb.ports[i].name);
            }
        }
        (self.geomdb, self.raw)
    }
}

fn check_equiv_grids(a: &Grid, b: &Grid) {
    if a.grid.shape() != b.grid.shape() {
        panic!("grid {} shape mismatch {:?} {:?}", a.name, a.grid.shape(), b.grid.shape());
    }
    for ((crd, ca), cb) in a.grid.indexed_iter().zip(b.grid.iter()) {
        for (ta, tb) in ca.tiles.iter().copied().zip(cb.tiles.iter().copied()) {
            match (ta, tb) {
                (None, None) => (),
                (Some((tai, tac)), Some((tbi, tbc))) => {
                    if tac != tbc {
                        panic!("grid {} tile mismatch at {:?}", a.name, crd);
                    }
                    let tae = &a.tiles[tai];
                    let tbe = &b.tiles[tbi];
                    if tae.cls != tbe.cls || tae.origin != tbe.origin {
                        panic!("grid {} tile mismatch at {:?}", a.name, crd);
                    }
                }
                _ => {
                    panic!("grid {} tile mismatch at {:?}", a.name, crd);
                }
            }
        }
        for (pa, pb) in ca.ports.iter().zip(cb.ports.iter()) {
            match (pa, pb) {
                (None, None) => (),
                (Some(pae), Some(pbe)) => {
                    if pae.cls != pbe.cls || pae.other != pbe.other {
                        panic!("grid {} port mismatch at {:?}", a.name, crd);
                    }
                }
                _ => {
                    panic!("grid {} port mismatch at {:?}", a.name, crd);
                }
            }
        }
    }
    if a.columns != b.columns {
        panic!("grid {} columns mismatch", a.name);
    }
    if a.vert_bus != b.vert_bus {
        panic!("grid {} vert bus mismatch", a.name);
    }
    if a.horiz_bus != b.horiz_bus {
        panic!("grid {} horiz bus mismatch", a.name);
    }
}
