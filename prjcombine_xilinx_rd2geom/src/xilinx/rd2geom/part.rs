use std::collections::{BTreeSet, HashMap, HashSet};
use std::iter;

use itertools::Itertools;
use ndarray::Array2;

use super::builder::{ExtractInfo, GeomBuilder};
use super::cfg::{GridSnap, IntTermInfo, RawSiteSlot, TileAnchor};
use crate::xilinx::geomdb::builder::GridBuilder;
use crate::xilinx::geomdb::{
    Dir, Grid, GridCell, GridRanges, Orient, PipInversion, TCWire, TileMux, TileMuxBranch,
    TileTran, WireConn,
};
use crate::xilinx::geomraw::{Extract, ExtractClass, ExtractTie, PartRaw, RawPip, RawPipDirection};
use prjcombine_xilinx_rawdump as rawdump;

pub struct PartBuilder<'a> {
    pub grid: Grid,
    pub raw: PartRaw,
    pub int_g2r_x: Vec<isize>,
    pub int_g2r_y: Vec<isize>,
    // maps raw x -> (grid col to left, grid col to right)
    // if x is a grid column, both values are equal
    // if x is to the left of leftmost grid col, or to right of rightmost grid col, result is (width-1, 0)
    pub int_r2g_x: Vec<(usize, usize)>,
    pub int_r2g_y: Vec<(usize, usize)>,
    pub rd: &'a rawdump::Part,
    pub slot_kind_lut: HashMap<String, u16>,
    pub cursed_tiles: HashSet<rawdump::Coord>,
    // coord -> ((x, y), tile class), rt idx
    pub extracted_tiles: HashMap<rawdump::Coord, Vec<(((usize, usize), usize), usize)>>,
    // (rt kind, wire dst, wire src)
    pub pips_extracted: HashSet<(&'a str, &'a str, &'a str)>,
}

impl<'a> PartBuilder<'a> {
    pub fn new(name: String, rd: &'a rawdump::Part, geob: &GeomBuilder) -> Self {
        // Prepare the grid.
        let mut int_tile_kinds = HashSet::new();
        for tt in geob.cfg.int_tiles.iter() {
            int_tile_kinds.insert(tt.raw_tile);
        }
        let mut int_x: HashSet<isize> = HashSet::new();
        let mut int_y: HashSet<isize> = HashSet::new();
        for (crd, tile) in rd.tiles.iter() {
            if int_tile_kinds.contains(&(&tile.kind[..])) {
                int_x.insert(crd.x as isize);
                int_y.insert(crd.y as isize);
            }
            if let Some(&cols) = geob.cfg.extra_cell_col_injectors.get(&(&tile.kind[..])) {
                for delta in cols {
                    int_x.insert((crd.x as isize) + delta);
                }
            }
        }
        let mut int_g2r_x = int_x.into_iter().collect::<Vec<_>>();
        int_g2r_x.sort_unstable();
        let mut int_g2r_y = int_y.into_iter().collect::<Vec<_>>();
        int_g2r_y.sort_unstable();
        let mut int_r2g_x = Vec::new();
        {
            let mut next = 0;
            let mut prev = int_g2r_x.len() - 1;
            while int_g2r_x[next] < 0 {
                prev = next;
                next += 1;
            }
            for i in 0..(rd.width as isize) {
                if int_g2r_x[next] == i {
                    int_r2g_x.push((next, next));
                    prev = next;
                    next += 1;
                    if next == int_g2r_x.len() {
                        next = 0;
                    }
                } else {
                    int_r2g_x.push((prev, next));
                }
            }
        }
        let mut int_r2g_y = Vec::new();
        {
            let mut next = 0;
            let mut prev = int_g2r_y.len() - 1;
            while int_g2r_y[next] < 0 {
                prev = next;
                next += 1;
            }
            for i in 0..(rd.height as isize) {
                if int_g2r_y[next] == i {
                    int_r2g_y.push((next, next));
                    prev = next;
                    next += 1;
                    if next == int_g2r_y.len() {
                        next = 0;
                    }
                } else {
                    int_r2g_y.push((prev, next));
                }
            }
        }
        let width = int_g2r_x.len();
        let height = int_g2r_y.len();

        PartBuilder {
            grid: Grid {
                name,
                grid: Array2::from_elem(
                    [width, height],
                    GridCell {
                        tiles: geob.geomdb.tile_slots.iter().map(|_| None).collect(),
                        ports: geob.geomdb.port_slots.iter().map(|_| None).collect(),
                    },
                ),
                columns: Vec::new(),
                vert_bus: (0..geob.geomdb.vert_bus.len())
                    .map(|_| Default::default())
                    .collect(),
                horiz_bus: (0..geob.geomdb.horiz_bus.len())
                    .map(|_| Default::default())
                    .collect(),
                tiles: Vec::new(),
            },
            raw: PartRaw {
                tiles: HashMap::new(),
                ports: HashMap::new(),
            },
            int_g2r_x,
            int_g2r_y,
            int_r2g_x,
            int_r2g_y,
            rd,
            slot_kind_lut: rd
                .slot_kinds
                .iter()
                .enumerate()
                .map(|(i, s)| (s.clone(), i as u16))
                .collect(),
            cursed_tiles: HashSet::new(),
            extracted_tiles: HashMap::new(),
            pips_extracted: HashSet::new(),
        }
    }

    pub fn width(&self) -> usize {
        self.grid.width()
    }

    pub fn height(&self) -> usize {
        self.grid.height()
    }

    pub fn curse_tile(&mut self, crd: rawdump::Coord) {
        self.cursed_tiles.insert(crd);
    }

    pub fn map_site_slot(&self, slot: &RawSiteSlot) -> rawdump::TkSiteSlot {
        match slot {
            &RawSiteSlot::Single(ref s) => rawdump::TkSiteSlot::Single(self.slot_kind_lut[s]),
            &RawSiteSlot::Indexed(ref s, i) => {
                rawdump::TkSiteSlot::Indexed(self.slot_kind_lut[s], i)
            }
            &RawSiteSlot::Xy(ref s, x, y) => rawdump::TkSiteSlot::Xy(self.slot_kind_lut[s], x, y),
        }
    }

    pub fn make_site_slot(&self, slot: rawdump::TkSiteSlot) -> RawSiteSlot {
        match slot {
            rawdump::TkSiteSlot::Single(s) => {
                RawSiteSlot::Single(self.rd.slot_kinds[s as usize].clone())
            }
            rawdump::TkSiteSlot::Indexed(s, i) => {
                RawSiteSlot::Indexed(self.rd.slot_kinds[s as usize].clone(), i)
            }
            rawdump::TkSiteSlot::Xy(s, x, y) => {
                RawSiteSlot::Xy(self.rd.slot_kinds[s as usize].clone(), x, y)
            }
        }
    }

    pub fn make_raw_pip(&self, rtidx: usize, rwa: &str, rwb: &str, pip: &rawdump::TkPip) -> RawPip {
        RawPip {
            rtidx,
            wire_out: rwa.to_string(),
            wire_in: rwb.to_string(),
            is_excl: pip.is_excluded,
            is_test: pip.is_test,
            direction: match pip.direction {
                rawdump::TkPipDirection::Uni => RawPipDirection::Uni,
                rawdump::TkPipDirection::BiFwd => RawPipDirection::BiFwd,
                rawdump::TkPipDirection::BiBwd => RawPipDirection::BiBwd,
            },
        }
    }

    pub fn extract(
        &mut self,
        geob: &mut GeomBuilder,
        info: ExtractInfo,
        tiles: Vec<((usize, usize), Vec<rawdump::Coord>)>,
    ) {
        let tcls = info.cls.tcls;
        let ecls = geob.extract(info);
        let site_slots = &geob.extract_tmp[ecls].site_slots;
        for (xy, crds) in tiles {
            self.grid.fill_tile(&geob.geomdb, xy, tcls);
            let raw_tiles: Vec<_> = crds.iter().map(|crd| &self.rd.tiles[crd]).collect();
            self.raw.tiles.insert(
                (xy, tcls),
                Extract {
                    cls: ecls,
                    raw_tiles: raw_tiles
                        .iter()
                        .copied()
                        .map(|tile| tile.name.clone())
                        .collect(),
                    raw_sites: site_slots
                        .iter()
                        .map(|&(ref rslot, rtidx)| {
                            let rt = raw_tiles[rtidx];
                            let rtk = &self.rd.tile_kinds[&rt.kind];
                            let slot = self.map_site_slot(rslot);
                            let sidx = rtk.sites_by_slot[&slot];
                            rt.sites[sidx].clone()
                        })
                        .collect(),
                },
            );
            for (i, crd) in crds.into_iter().enumerate() {
                self.extracted_tiles
                    .entry(crd)
                    .or_default()
                    .push(((xy, tcls), i));
            }
        }
    }

    pub fn fill_int_tiles(&mut self, geob: &mut GeomBuilder) {
        let mut extracts = Vec::new();
        let mut tmuxes: HashMap<usize, HashMap<TCWire, HashSet<TCWire>>> = HashMap::new();
        for tt in geob.cfg.int_tiles.iter() {
            let tcls = geob.geomdb.tiles.idx(tt.name);
            if let Some(tk) = self.rd.tile_kinds.get(tt.raw_tile) {
                let tiles: Vec<_> = tk
                    .tiles
                    .iter()
                    .copied()
                    .map(|crd| {
                        let (gx, gx_) = self.int_r2g_x[crd.x as usize];
                        let (gy, gy_) = self.int_r2g_y[crd.y as usize];
                        assert!(gx == gx_ && gy == gy_);
                        ((gx, gy), vec![crd])
                    })
                    .collect();

                let name = format!("{}.{}", tt.name, tt.raw_tile);

                let muxes = tmuxes.entry(tcls).or_default();
                let mut trans = HashSet::new();

                let mut pips = HashMap::new();
                let mut ties = HashMap::new();
                let mut wire_map: HashMap<String, TCWire> = HashMap::new();
                let mut site_slots = Vec::new();
                let mut tile_trans = Vec::new();

                for (&rwi, _) in tk.wires.iter() {
                    let rwire = self.rd.wire(rwi);
                    if let Some(&w) = geob.int_wiremap.get(rwire) {
                        wire_map.insert(rwire.to_string(), TCWire { cell: 0, wire: w });
                    } else if let Some(m) = geob.wiremap.get(&name) {
                        if let Some(&(d, w)) = m.get(rwire) {
                            assert_eq!(d, (0, 0));
                            wire_map.insert(rwire.to_string(), TCWire { cell: 0, wire: w });
                        }
                    }
                }

                // First, ties.
                for site in tk.sites.iter() {
                    for tie in geob.cfg.tie_sites.iter() {
                        if site.kind == tie.kind {
                            let rsidx = site_slots.len();
                            site_slots.push((self.make_site_slot(site.slot), 0));
                            for (pin, state) in tie.pins.iter().copied() {
                                let sp = &site.pins[pin];
                                assert_eq!(sp.dir, rawdump::TkSitePinDir::Output);
                                let rwire = self.rd.wire(sp.wire);
                                let wire = wire_map[rwire];
                                match geob.geomdb.wires[wire.wire].conn {
                                    WireConn::Tie(s) => assert_eq!(s, state),
                                    _ => panic!("tie site for non-tied wire"),
                                }
                                ties.insert(
                                    wire,
                                    ExtractTie {
                                        rtidx: 0,
                                        rsidx,
                                        kind: tie.kind.to_string(),
                                        pin: pin.to_string(),
                                    },
                                );
                            }
                        }
                    }
                }

                // XXX extract sites

                // XXX validate missing pips are not a problem

                // XXX permabufs?

                for (&(rwib, rwia), pip) in tk.pips.iter() {
                    let rwa = self.rd.wire(rwia);
                    let rwb = self.rd.wire(rwib);
                    let wa = match wire_map.get(rwa) {
                        None => {
                            continue;
                        }
                        Some(&w) => w,
                    };
                    if !geob.int_wires.contains(&wa.wire) {
                        continue;
                    }
                    if geob.int_out_wires.contains(&wa.wire) {
                        continue;
                    }
                    let wa_cls = &geob.geomdb.wires[wa.wire];
                    let has_up = match wa_cls.conn {
                        WireConn::Internal => false,
                        WireConn::Port { up, .. } => up.is_some(),
                        _ => true,
                    };
                    if has_up && !wa_cls.has_multicell_drive {
                        continue;
                    }
                    let wb = match wire_map.get(rwb) {
                        None => panic!("unknown int mux input {} {}", rwa, rwb),
                        Some(&w) => w,
                    };
                    if !pip.is_buf && pip.direction != rawdump::TkPipDirection::Uni {
                        trans.insert((wa, wb));
                        assert_eq!(pip.inversion, rawdump::TkPipInversion::Never);
                    } else {
                        muxes.entry(wa).or_default().insert(wb);
                        // XXX
                        assert_eq!(pip.inversion, rawdump::TkPipInversion::Never);
                    }
                    pips.insert((wa, wb), vec![self.make_raw_pip(0, rwa, rwb, pip)]);
                    self.pips_extracted.insert((tt.raw_tile, rwa, rwb));
                }

                for (a, b) in trans.iter().copied().sorted() {
                    if a < b {
                        assert!(trans.contains(&(b, a)));
                        tile_trans.push(TileTran {
                            wire_a: a,
                            wire_b: b,
                        });
                    }
                }

                extracts.push((
                    ExtractInfo {
                        cls: ExtractClass {
                            name: if self.rd.source == rawdump::Source::ISE {
                                format!("{}.I", name)
                            } else {
                                format!("{}.V", name)
                            },
                            tcls,
                            pips,
                            sites: Vec::new(),
                            ties,
                        },
                        wire_map: vec![wire_map],
                        site_slots,
                        tile_muxes: Vec::new(),
                        tile_multimuxes: Vec::new(),
                        tile_trans,
                        tile_sites: Vec::new(),
                    },
                    tiles,
                ));
            }
        }
        for (mut info, tiles) in extracts {
            let mut tile_muxes = Vec::new();
            for (&a, ins) in tmuxes[&info.cls.tcls].iter().sorted_by_key(|(&a, _)| a) {
                tile_muxes.push(TileMux {
                    wire_out: a,
                    branches: ins
                        .iter()
                        .copied()
                        .sorted()
                        .map(|b| TileMuxBranch {
                            wire_in: b,
                            // XXX inversion some day
                            inversion: PipInversion::Never,
                        })
                        .collect(),
                })
            }
            info.tile_muxes = tile_muxes;
            self.extract(geob, info, tiles);
        }
    }

    pub fn find_anchors(&self, anchor: &TileAnchor) -> Vec<(rawdump::Coord, (usize, usize))> {
        let mut res = Vec::new();
        for rt in anchor.raw_tiles.iter().copied() {
            if let Some(tk) = self.rd.tile_kinds.get(rt) {
                'outer: for crd in tk.tiles.iter().copied() {
                    for (dx, dy, ert) in anchor.extra_raw.iter().copied() {
                        let erx = (crd.x as isize) + dx;
                        let ery = (crd.y as isize) + dy;
                        if erx < 0
                            || ery < 0
                            || erx >= (self.rd.width as isize)
                            || ery >= (self.rd.height as isize)
                        {
                            continue 'outer;
                        }
                        let tile = &self.rd.tiles[&rawdump::Coord {
                            x: erx as u16,
                            y: ery as u16,
                        }];
                        if !ert.contains(&(&tile.kind[..])) {
                            continue 'outer;
                        }
                    }
                    let rx = (crd.x as isize) + anchor.raw_delta.0;
                    let ry = (crd.y as isize) + anchor.raw_delta.1;
                    let (gx_l, gx_r) = self.int_r2g_x[rx as usize];
                    let (gy_d, gy_u) = self.int_r2g_y[ry as usize];
                    let rgx = match anchor.grid_snap.0 {
                        GridSnap::Down => {
                            assert_ne!(gx_l, gx_r);
                            gx_l
                        }
                        GridSnap::None => {
                            assert_eq!(gx_l, gx_r);
                            gx_l
                        }
                        GridSnap::Up => {
                            assert_ne!(gx_l, gx_r);
                            gx_r
                        }
                    };
                    let rgy = match anchor.grid_snap.1 {
                        GridSnap::Down => {
                            assert_ne!(gy_d, gy_u);
                            gy_d
                        }
                        GridSnap::None => {
                            assert_eq!(gy_d, gy_u);
                            gy_d
                        }
                        GridSnap::Up => {
                            assert_ne!(gy_d, gy_u);
                            gy_u
                        }
                    };
                    let gx = ((rgx as isize) + anchor.grid_delta.0) as usize;
                    let gy = ((rgy as isize) + anchor.grid_delta.1) as usize;
                    res.push((crd, (gx, gy)));
                }
            }
        }
        res
    }

    pub fn find_anchor_gx_set(&self, anchor: &TileAnchor) -> HashSet<usize> {
        self.find_anchors(anchor)
            .into_iter()
            .map(|(_, (x, _))| x)
            .collect()
    }

    pub fn find_anchor_gy_set(&self, anchor: &TileAnchor) -> HashSet<usize> {
        self.find_anchors(anchor)
            .into_iter()
            .map(|(_, (_, y))| y)
            .collect()
    }

    pub fn walk_grid(
        &self,
        geob: &GeomBuilder,
        x: usize,
        y: usize,
        dx: isize,
        dy: isize,
    ) -> (usize, usize) {
        let int = geob.get_int();
        let mut ix = x as isize;
        let mut iy = y as isize;
        loop {
            let cell = &self.grid.grid[(ix as usize, iy as usize)];
            if cell.tiles[int.tslot_int].is_some() {
                return (ix as usize, iy as usize);
            }
            ix += dx;
            iy += dy;
            if ix < 0 || iy < 0 || ix >= self.width() as isize || iy >= self.height() as isize {
                panic!("walked out of grid {}.{}", x, y);
            }
        }
    }

    pub fn get_int_term_xy(
        &self,
        geob: &GeomBuilder,
        term: &IntTermInfo,
        crd: rawdump::Coord,
    ) -> Vec<((usize, usize), String)> {
        let (gx_l, gx_r) = self.int_r2g_x[crd.x as usize];
        let (gy_d, gy_u) = self.int_r2g_y[crd.y as usize];
        let mut tiles = Vec::new();
        match term.dir {
            Dir::E => match term.span {
                None => {
                    assert!(gy_d == gy_u);
                    let c = self.walk_grid(geob, gx_l, gy_d, -1, 0);
                    tiles.push((c, format!("{}.{}", term.name, term.raw_tile)));
                }
                Some((n, p)) => {
                    assert!(gy_u >= p);
                    for d in 0..n {
                        let y = gy_u - p + d;
                        let c = self.walk_grid(geob, gx_l, y, -1, 0);
                        tiles.push((c, format!("{}.{}.{}", term.name, term.raw_tile, d)));
                    }
                }
            },
            Dir::W => match term.span {
                None => {
                    assert!(gy_d == gy_u);
                    let c = self.walk_grid(geob, gx_r, gy_d, 1, 0);
                    tiles.push((c, format!("{}.{}", term.name, term.raw_tile)));
                }
                Some((n, p)) => {
                    assert!(gy_u >= p);
                    for d in 0..n {
                        let y = gy_u - p + d;
                        let c = self.walk_grid(geob, gx_r, y, 1, 0);
                        tiles.push((c, format!("{}.{}.{}", term.name, term.raw_tile, d)));
                    }
                }
            },
            Dir::N => match term.span {
                None => {
                    assert!(gx_l == gx_r);
                    let c = self.walk_grid(geob, gx_l, gy_d, 0, -1);
                    tiles.push((c, format!("{}.{}", term.name, term.raw_tile)));
                }
                Some((n, p)) => {
                    assert!(gx_r >= p);
                    for d in 0..n {
                        let x = gx_r - p + d;
                        let c = self.walk_grid(geob, x, gy_d, 0, -1);
                        tiles.push((c, format!("{}.{}.{}", term.name, term.raw_tile, d)));
                    }
                }
            },
            Dir::S => match term.span {
                None => {
                    assert!(gx_l == gx_r);
                    let c = self.walk_grid(geob, gx_l, gy_u, 0, 1);
                    tiles.push((c, format!("{}.{}", term.name, term.raw_tile)));
                }
                Some((n, p)) => {
                    assert!(gx_r >= p);
                    for d in 0..n {
                        let x = gx_r - p + d;
                        let c = self.walk_grid(geob, x, gy_u, 0, 1);
                        tiles.push((c, format!("{}.{}.{}", term.name, term.raw_tile, d)));
                    }
                }
            },
        }
        tiles
    }

    pub fn fill_int_terms(&mut self, geob: &GeomBuilder) {
        for term in geob.cfg.int_terms.iter() {
            if let Some(tk) = self.rd.tile_kinds.get(term.raw_tile) {
                let mut exts: HashMap<String, Vec<(rawdump::Coord, (usize, usize))>> =
                    HashMap::new();
                for crd in tk.tiles.iter().copied() {
                    if self.cursed_tiles.contains(&crd) {
                        continue;
                    }
                    for (xy, name) in self.get_int_term_xy(geob, term, crd) {
                        exts.entry(name).or_default().push((crd, xy));
                    }
                }
                for (name, tiles) in exts {
                    println!("TERM {}", name);

                    // XXX obtain int tile

                    // let mut wire_map: HashMap<String, TCWire> = HashMap::new();

                    // XXX wire map

                    // let mut muxes: HashMap<TCWire, HashSet<TCWire>> = HashMap::new();

                    // XXX muxes

                    // XXX straight thru wires

                    for (_crd, xy) in tiles {
                        // XXX extraction here?
                        self.grid.fill_port_term(
                            &geob.geomdb,
                            xy,
                            geob.geomdb.ports.idx(term.name),
                        );
                        if term.needs_tile {
                            self.grid
                                .fill_tile(&geob.geomdb, xy, geob.geomdb.tiles.idx(term.name));
                        }
                    }
                }
            }
        }
    }

    pub fn fill_int_bufs(&mut self, geob: &GeomBuilder) {
        let int = geob.get_int();
        for buf in geob.cfg.int_bufs.iter() {
            if let Some(tk) = self.rd.tile_kinds.get(buf.raw_tile) {
                for crd in tk.tiles.iter().copied() {
                    if self.cursed_tiles.contains(&crd) {
                        continue;
                    }
                    let (gx_l, gx_r) = self.int_r2g_x[crd.x as usize];
                    let (gy_d, gy_u) = self.int_r2g_y[crd.y as usize];
                    let mut tiles = Vec::new();
                    match buf.orient {
                        Orient::H => match buf.span {
                            None => {
                                let c_a = self.walk_grid(geob, gx_l, gy_u, -1, 0);
                                let c_b = self.walk_grid(geob, gx_r, gy_u, 1, 0);
                                tiles.push((c_a, c_b, buf.raw_tile.to_string()));
                            }
                            Some((n, p)) => {
                                assert!(gy_u >= p);
                                for d in 0..n {
                                    let y = gy_u - p + d;
                                    let c_a = self.walk_grid(geob, gx_l, y, -1, 0);
                                    let c_b = self.walk_grid(geob, gx_r, y, 1, 0);
                                    tiles.push((c_a, c_b, format!("{}[{}]", buf.raw_tile, d)));
                                }
                            }
                        },
                        Orient::V => match buf.span {
                            None => {
                                assert!(gx_l == gx_r);
                                let c_a = self.walk_grid(geob, gx_l, gy_d, 0, -1);
                                let c_b = self.walk_grid(geob, gx_l, gy_u, 0, 1);
                                tiles.push((c_a, c_b, buf.raw_tile.to_string()));
                            }
                            Some((n, p)) => {
                                assert!(gx_r >= p);
                                for d in 0..n {
                                    let x = gx_r - p + d;
                                    let c_a = self.walk_grid(geob, x, gy_d, 0, -1);
                                    let c_b = self.walk_grid(geob, x, gy_u, 0, 1);
                                    tiles.push((c_a, c_b, format!("{}[{}]", buf.raw_tile, d)));
                                }
                            }
                        },
                    }
                    for (xya, xyb, _raw) in tiles {
                        // XXX extraction here?
                        self.grid
                            .fill_port_pair(&geob.geomdb, xya, xyb, int.pcls_buf[buf.name]);
                        if buf.needs_tile {
                            self.grid
                                .fill_tile(&geob.geomdb, xyb, geob.geomdb.tiles.idx(buf.name));
                        }
                    }
                }
            }
        }
    }

    pub fn fill_int_dbufs(&mut self, geob: &GeomBuilder) {
        let mut int_tile_kinds = HashSet::new();
        for tt in geob.cfg.int_tiles.iter() {
            int_tile_kinds.insert(tt.raw_tile);
        }
        let int = geob.get_int();
        for dbuf in geob.cfg.int_dbufs.iter() {
            if let Some(tk) = self.rd.tile_kinds.get(dbuf.raw_tile_a) {
                'outer: for crd_a in tk.tiles.iter().copied() {
                    if self.cursed_tiles.contains(&crd_a) {
                        continue;
                    }

                    let mut crd_b;
                    match dbuf.orient {
                        Orient::H => {
                            let mut x = crd_a.x + 1;
                            loop {
                                if x == self.rd.width {
                                    continue 'outer;
                                }
                                crd_b = rawdump::Coord { x, y: crd_a.y };
                                let tile = &self.rd.tiles[&crd_b];
                                if int_tile_kinds.contains(&tile.kind[..]) {
                                    continue 'outer;
                                }
                                if tile.kind == dbuf.raw_tile_b {
                                    break;
                                }
                                x += 1;
                            }
                        }
                        Orient::V => {
                            let mut y = crd_a.y + 1;
                            loop {
                                if y == self.rd.height {
                                    continue 'outer;
                                }
                                crd_b = rawdump::Coord { x: crd_a.x, y };
                                let tile = &self.rd.tiles[&crd_b];
                                if int_tile_kinds.contains(&tile.kind[..]) {
                                    continue 'outer;
                                }
                                if tile.kind == dbuf.raw_tile_b {
                                    break;
                                }
                                y += 1;
                            }
                        }
                    }

                    let (gx_l, _) = self.int_r2g_x[crd_a.x as usize];
                    let (gy_d, _) = self.int_r2g_y[crd_a.y as usize];
                    let (_, gx_r) = self.int_r2g_x[crd_b.x as usize];
                    let (_, gy_u) = self.int_r2g_y[crd_b.y as usize];

                    let mut tiles = Vec::new();
                    match dbuf.orient {
                        Orient::H => match dbuf.span {
                            None => {
                                let c_a = self.walk_grid(geob, gx_l, gy_u, -1, 0);
                                let c_b = self.walk_grid(geob, gx_r, gy_u, 1, 0);
                                tiles.push((
                                    c_a,
                                    c_b,
                                    format!("{}.{}", dbuf.raw_tile_a, dbuf.raw_tile_b),
                                ));
                            }
                            Some((n, p)) => {
                                assert!(gy_u >= p);
                                for d in 0..n {
                                    let y = gy_u - p + d;
                                    let c_a = self.walk_grid(geob, gx_l, y, -1, 0);
                                    let c_b = self.walk_grid(geob, gx_r, y, 1, 0);
                                    tiles.push((
                                        c_a,
                                        c_b,
                                        format!("{}.{}[{}]", dbuf.raw_tile_a, dbuf.raw_tile_b, d),
                                    ));
                                }
                            }
                        },
                        Orient::V => match dbuf.span {
                            None => {
                                assert!(gx_l == gx_r);
                                let c_a = self.walk_grid(geob, gx_l, gy_d, 0, -1);
                                let c_b = self.walk_grid(geob, gx_l, gy_u, 0, 1);
                                tiles.push((
                                    c_a,
                                    c_b,
                                    format!("{}.{}", dbuf.raw_tile_a, dbuf.raw_tile_b),
                                ));
                            }
                            Some((n, p)) => {
                                assert!(gx_r >= p);
                                for d in 0..n {
                                    let x = gx_r - p + d;
                                    let c_a = self.walk_grid(geob, x, gy_d, 0, -1);
                                    let c_b = self.walk_grid(geob, x, gy_u, 0, 1);
                                    tiles.push((
                                        c_a,
                                        c_b,
                                        format!("{}.{}[{}]", dbuf.raw_tile_a, dbuf.raw_tile_b, d),
                                    ));
                                }
                            }
                        },
                    }
                    for (xya, xyb, _raw) in tiles {
                        // XXX extraction here?
                        self.grid
                            .fill_port_pair(&geob.geomdb, xya, xyb, int.pcls_dbuf[dbuf.name]);
                        if dbuf.needs_tile {
                            self.grid
                                .fill_tile(&geob.geomdb, xya, int.tcls_dbuf[dbuf.name].0);
                            self.grid
                                .fill_tile(&geob.geomdb, xyb, int.tcls_dbuf[dbuf.name].1);
                        }
                    }
                }
            }
        }
    }

    pub fn fill_int_passes(&mut self, geob: &GeomBuilder) {
        let int = geob.get_int();
        let mut specials: HashMap<((usize, usize), (usize, usize)), BTreeSet<&'static str>> =
            HashMap::new();
        let mut emptys: HashSet<((usize, usize), (usize, usize))> = HashSet::new();
        for pass in geob.cfg.int_passes.iter() {
            if let Some(tk) = self.rd.tile_kinds.get(pass.raw_tile) {
                for crd in tk.tiles.iter().copied() {
                    if self.cursed_tiles.contains(&crd) {
                        continue;
                    }
                    let (gx_l, gx_r) = self.int_r2g_x[crd.x as usize];
                    let (gy_d, gy_u) = self.int_r2g_y[crd.y as usize];
                    let mut tiles = Vec::new();
                    match pass.orient {
                        Orient::H => match pass.span {
                            None => {
                                let c_a = self.walk_grid(geob, gx_l, gy_u, -1, 0);
                                let c_b = self.walk_grid(geob, gx_r, gy_u, 1, 0);
                                tiles.push((c_a, c_b));
                            }
                            Some((n, p)) => {
                                assert!(gy_u >= p);
                                for d in 0..n {
                                    let y = gy_u - p + d;
                                    let c_a = self.walk_grid(geob, gx_l, y, -1, 0);
                                    let c_b = self.walk_grid(geob, gx_r, y, 1, 0);
                                    tiles.push((c_a, c_b));
                                }
                            }
                        },
                        Orient::V => match pass.span {
                            None => {
                                assert!(gx_l == gx_r);
                                let c_a = self.walk_grid(geob, gx_l, gy_d, 0, -1);
                                let c_b = self.walk_grid(geob, gx_l, gy_u, 0, 1);
                                tiles.push((c_a, c_b));
                            }
                            Some((n, p)) => {
                                assert!(gx_r >= p);
                                for d in 0..n {
                                    let x = gx_r - p + d;
                                    let c_a = self.walk_grid(geob, x, gy_d, 0, -1);
                                    let c_b = self.walk_grid(geob, x, gy_u, 0, 1);
                                    tiles.push((c_a, c_b));
                                }
                            }
                        },
                    }
                    for (xya, xyb) in tiles {
                        if pass.empty {
                            emptys.insert((xya, xyb));
                        } else {
                            specials.entry((xya, xyb)).or_default().insert(pass.name);
                        }
                    }
                }
            }
        }

        let mut all_pairs = Vec::new();
        for gx in 0..self.width() {
            for gy in 0..self.height() {
                let cell = &self.grid.grid[(gx, gy)];
                if cell.tiles[int.tslot_int].is_none() {
                    continue;
                }
                let mut nx = gx + 1;
                loop {
                    if nx >= self.width() {
                        break;
                    }
                    let ncell = &self.grid.grid[(nx, gy)];
                    if ncell.tiles[int.tslot_int].is_some() {
                        all_pairs.push(((gx, gy), (nx, gy)));
                        break;
                    }
                    nx += 1;
                }
                let mut ny = gy + 1;
                loop {
                    if ny >= self.height() {
                        break;
                    }
                    let ncell = &self.grid.grid[(gx, ny)];
                    if ncell.tiles[int.tslot_int].is_some() {
                        all_pairs.push(((gx, gy), (gx, ny)));
                        break;
                    }
                    ny += 1;
                }
            }
        }

        for (xya, xyb) in all_pairs {
            let ca = &self.grid.grid[xya];
            let cb = &self.grid.grid[xyb];
            if emptys.contains(&(xya, xyb)) {
                continue;
            }
            if xya.1 == xyb.1 {
                if ca.ports[int.pslot_int_e].is_some() {
                    continue;
                }
                if cb.ports[int.pslot_int_w].is_some() {
                    continue;
                }
            } else {
                if ca.ports[int.pslot_int_n].is_some() {
                    continue;
                }
                if cb.ports[int.pslot_int_s].is_some() {
                    continue;
                }
            }
            match specials.get(&(xya, xyb)) {
                None => {
                    let pcls = if xya.1 == xyb.1 {
                        (int.pcls_int_e_pass, int.pcls_int_w_pass)
                    } else {
                        (int.pcls_int_n_pass, int.pcls_int_s_pass)
                    };
                    self.grid.fill_port_pair(&geob.geomdb, xya, xyb, pcls);
                }
                Some(specs) => {
                    let spec = if specs.len() == 1 {
                        specs.iter().next().unwrap()
                    } else {
                        geob.cfg.int_pass_combine[specs]
                    };
                    // XXX save those up for extraction I guess
                    self.grid
                        .fill_port_pair(&geob.geomdb, xya, xyb, int.pcls_pass[spec]);
                }
            }
        }
    }

    pub fn fill_int(&mut self, geob: &mut GeomBuilder) {
        self.fill_int_tiles(geob);
        self.fill_int_terms(geob);
        self.fill_int_bufs(geob);
        self.fill_int_dbufs(geob);
        self.fill_int_passes(geob);
    }

    pub fn fill_bus(
        &mut self,
        geob: &GeomBuilder,
        orient: Orient,
        bus: &str,
        endpoints: Vec<usize>,
        midpoints: Vec<usize>,
    ) {
        let gr = GridRanges::new(endpoints, midpoints);
        match orient {
            Orient::H => {
                let hbus = geob.geomdb.horiz_bus.idx(bus);
                self.grid.horiz_bus[hbus] = gr;
            }
            Orient::V => {
                let vbus = geob.geomdb.vert_bus.idx(bus);
                self.grid.vert_bus[vbus] = gr;
            }
        }
    }

    pub fn fill_srcbrk_bus(
        &mut self,
        geob: &GeomBuilder,
        orient: Orient,
        bus: &str,
        src: impl Iterator<Item = usize>,
        brk: impl Iterator<Item = usize>,
    ) {
        let end = match orient {
            Orient::H => self.width(),
            Orient::V => self.height(),
        };
        let ccs: Vec<_> = src.sorted().dedup().collect();
        let cbs: Vec<_> = iter::once(0)
            .chain(brk.sorted())
            .chain(iter::once(end))
            .dedup()
            .collect();
        assert_eq!(cbs.len(), ccs.len() + 1);
        let mut endpoints = Vec::new();
        let mut midpoints = Vec::new();
        assert!(cbs[0] == 0);
        for i in 0..ccs.len() {
            endpoints.push(cbs[i]);
            midpoints.push(ccs[i]);
            assert!(cbs[i] < ccs[i]);
            assert!(ccs[i] < cbs[i + 1]);
        }
        endpoints.push(cbs[ccs.len()]);
        self.fill_bus(geob, orient, bus, endpoints, midpoints);
    }

    pub fn fill_srcbrk_bus_split(
        &mut self,
        geob: &GeomBuilder,
        orient: Orient,
        bus: &str,
        src: impl Iterator<Item = usize>,
        brk: impl Iterator<Item = usize>,
    ) {
        let end = match orient {
            Orient::H => self.width(),
            Orient::V => self.height(),
        };
        let ccs: Vec<_> = src.sorted().dedup().collect();
        let cbs: Vec<_> = iter::once(0)
            .chain(brk.sorted())
            .chain(iter::once(end))
            .dedup()
            .collect();
        assert_eq!(cbs.len(), ccs.len() + 1);
        let mut endpoints = Vec::new();
        let mut midpoints = Vec::new();
        assert!(cbs[0] == 0);
        for i in 0..ccs.len() {
            endpoints.push(cbs[i]);
            midpoints.push(ccs[i] - 1);
            endpoints.push(ccs[i]);
            midpoints.push(ccs[i]);
            assert!(cbs[i] < ccs[i]);
            assert!(ccs[i] < cbs[i + 1]);
        }
        endpoints.push(cbs[ccs.len()]);
        self.fill_bus(geob, orient, bus, endpoints, midpoints);
    }

    pub fn fill_tiles(&mut self, geob: &GeomBuilder) {
        for t in geob.cfg.tiles.iter() {
            let tcls = geob.geomdb.tiles.idx(t.name);
            for (_crd, gxy) in self.find_anchors(&t.anchor) {
                // XXX extraction here
                self.grid.fill_tile(&geob.geomdb, gxy, tcls);
            }
        }
    }
}
