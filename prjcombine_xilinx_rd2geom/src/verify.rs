use prjcombine_xilinx_rawdump::{Part, Coord, WireIdx, self as rawdump};
use prjcombine_xilinx_geom::{int, eint, eint::IntWire};
use std::collections::{HashMap, HashSet, BTreeSet};
use bitvec::vec::BitVec;
use indexmap::IndexSet;

pub struct Verifier<'a> {
    pub rd: &'a Part,
    pub db: &'a int::IntDb,
    pub grid: &'a eint::ExpandedGrid<'a>,
    pub wire_lut: HashMap<String, rawdump::WireIdx>,
    pub tile_lut: HashMap<String, Coord>,
    claimed_nodes: BitVec,
    claimed_twires: HashMap<Coord, BitVec>,
    claimed_pips: HashMap<Coord, BitVec>,
    int_wires: HashMap<IntWire, NodeOrWire>,
    int_site_wires: HashMap<IntWire, NodeOrWire>,
    missing_int_wires: HashSet<IntWire>,
    missing_int_site_wires: HashSet<IntWire>,
    tkinfo: HashMap<String, TkInfo>,
}

struct TkInfo {
    pips: IndexSet<(WireIdx, WireIdx)>,
    wires: IndexSet<WireIdx>,
}

pub struct SitePin<'a> {
    pub dir: SitePinDir,
    pub pin: &'a str,
    pub wire: Option<&'a str>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum NodeOrWire {
    Node(usize),
    Wire(Coord, usize),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SitePinDir {
    In,
    Out,
    Inout,
}

impl<'a> Verifier<'a> {
    pub fn new(rd: &'a Part, grid: &'a eint::ExpandedGrid) -> Self {
        let tkinfo = rd.tile_kinds.iter().map(|(n, tk)| {
            let pips = tk.pips.keys().copied().collect();
            let wires = tk.wires.iter().filter_map(|(&k, v)| if matches!(v, rawdump::TkWire::Internal(_, _)) {Some(k)} else {None}).collect();
            (n.clone(), TkInfo {
                pips,
                wires,
            })
        }).collect();
        let mut res = Self {
            rd,
            db: grid.db,
            grid,
            wire_lut: rd.wires.iter().enumerate().map(|(i, n)| (n.clone(), rawdump::WireIdx::from_raw(i))).collect(),
            tile_lut: rd.tiles.iter().map(|(&c, t)| (t.name.clone(), c)).collect(),
            claimed_nodes: BitVec::new(),
            claimed_twires: HashMap::new(),
            claimed_pips: HashMap::new(),
            int_wires: HashMap::new(),
            int_site_wires: HashMap::new(),
            missing_int_wires: HashSet::new(),
            missing_int_site_wires: HashSet::new(),
            tkinfo,
        };
        res.handle_int();
        res
    }

    fn lookup_wire(&self, crd: Coord, wire: &str) -> Option<NodeOrWire> {
        let tile = &self.rd.tiles[&crd];
        let tk = &self.rd.tile_kinds[&tile.kind];
        let widx = *self.wire_lut.get(wire)?;
        match *tk.wires.get(&widx)? {
            rawdump::TkWire::Internal(_, _) => {
                let widx = self.tkinfo[&tile.kind].wires.get_full(&widx).unwrap().0;
                Some(NodeOrWire::Wire(crd, widx))
            }
            rawdump::TkWire::Connected(idx) => {
                match tile.conn_wires.get(idx) {
                    Some(&rawdump::NodeOrClass::Node(nidx)) => Some(NodeOrWire::Node(nidx as usize)),
                    _ => None,
                }
            }
        }
    }

    pub fn pin_int_wire(&mut self, crd: Coord, wire: &str, iw: IntWire) -> bool {
        if let Some(&nw) = self.int_wires.get(&iw) {
            if let Some(cnw) = self.lookup_wire(crd, wire) {
                if nw != cnw {
                    let tname = &self.rd.tiles[&crd].name;
                    println!("INT NODE MISMATCH FOR {p} {tname} {wire} {iw:?} {wn}", p=self.rd.part, wn = self.db.wires[iw.2].name);
                }
                true
            } else {
                let tname = &self.rd.tiles[&crd].name;
                println!("INT NODE PRESENT FOR {tname} {wire} BUT WIRE NOT FOUND");
                false
            }
        } else if self.missing_int_wires.contains(&iw) {
            if let Some(cnw) = self.lookup_wire(crd, wire) {
                let tname = &self.rd.tiles[&crd].name;
                println!("INT NODE PRESENT FOR {tname} {wire} BUT WAS MISSING PREVIOUSLY");
                self.claim_node(&[(crd, wire)]);
                self.int_wires.insert(iw, cnw);
                true
            } else {
                let tname = &self.rd.tiles[&crd].name;
                println!("INT WIRE {tname} {wire} MISSING TWICE");
                false
            }
        } else if let Some(cnw) = self.lookup_wire(crd, wire) {
            self.claim_node(&[(crd, wire)]);
            self.int_wires.insert(iw, cnw);
            true
        } else {
            self.missing_int_wires.insert(iw);
            false
        }
    }

    pub fn pin_int_site_wire(&mut self, crd: Coord, wire: &str, iw: IntWire) -> bool {
        if self.int_site_wires.get(&iw).is_some() {
            let tname = &self.rd.tiles[&crd].name;
            println!("INT SITE NODE DOUBLE PIN FOR {p} {tname} {wire} {iw:?} {wn}", p=self.rd.part, wn = self.db.wires[iw.2].name);
            true
        } else if let Some(cnw) = self.lookup_wire(crd, wire) {
            self.claim_node(&[(crd, wire)]);
            self.int_site_wires.insert(iw, cnw);
            true
        } else {
            self.missing_int_site_wires.insert(iw);
            false
        }
    }

    pub fn claim_node(&mut self, tiles: &[(Coord, &str)]) {
        let mut nw = None;
        for &(crd, wn) in tiles {
            let tile = &self.rd.tiles[&crd];
            let tname = &tile.name;
            if let Some(cnw) = self.lookup_wire(crd, wn) {
                if let Some(pnw) = nw {
                    if pnw != cnw {
                        println!("NODE MISMATCH FOR {tname} {wn}");
                    }
                } else {
                    nw = Some(cnw);
                    match cnw {
                        NodeOrWire::Node(nidx) => {
                            let nidx = nidx as usize;
                            if nidx >= self.claimed_nodes.len() {
                                self.claimed_nodes.resize(nidx + 1, false);
                            }
                            if self.claimed_nodes[nidx] {
                                println!("DOUBLE CLAIMED NODE {tname} {wn}");
                            }
                            self.claimed_nodes.set(nidx, true);
                        }
                        NodeOrWire::Wire(crd, widx) => {
                            let ctw = self.claimed_twires.entry(crd).or_default();
                            if widx >= ctw.len() {
                                ctw.resize(widx + 1, false);
                            }
                            if ctw[widx] {
                                println!("DOUBLE CLAIMED NODE {tname} {wn}");
                            }
                            ctw.set(widx, true);
                        }
                    }
                }
            } else {
                println!("MISSING WIRE {tname} {wn}");
            }
        }
    }

    pub fn claim_pip(&mut self, crd: Coord, wt: &str, wf: &str) {
        let tile = &self.rd.tiles[&crd];
        let tname = &tile.name;
        let tkinfo = &self.tkinfo[&tile.kind];
        let wti = if let Some(&wti) = self.wire_lut.get(wt) {
            wti
        } else {
            println!("MISSING PIP DEST WIRE {tname} {wt}");
            return;
        };
        let wfi = if let Some(&wfi) = self.wire_lut.get(wf) {
            wfi
        } else {
            println!("MISSING PIP SRC WIRE {tname} {wf}");
            return;
        };
        if let Some((idx, _)) = tkinfo.pips.get_full(&(wfi, wti)) {
            let ctp = self.claimed_pips.entry(crd).or_default();
            if idx >= ctp.len() {
                ctp.resize(idx + 1, false);
            }
            if ctp[idx] {
                println!("DOUBLE CLAIMED PIP {tname} {wt} <- {wf}");
            }
            ctp.set(idx, true);
        } else {
            println!("MISSING PIP {p} {tname} {wt} <- {wf}", p=self.rd.part);
        }
    }

    pub fn claim_site(&mut self, crd: Coord, name: &str, kind: &str, pins: &[SitePin<'_>]) {
        let tile = &self.rd.tiles[&crd];
        let tk = &self.rd.tile_kinds[&tile.kind];
        for (i, n) in tile.sites.iter().enumerate() {
            if let Some(n) = n {
                if n == name {
                    let site = &tk.sites[i];
                    if site.kind != kind {
                        println!("MISMATCHED SITE KIND {} {} {} {} {}", self.rd.part, tile.name, name, kind, site.kind);
                    }
                    // XXX pins
                    return;
                }
            }
        }
        println!("MISSING SITE {} {} {}", self.rd.part, tile.name, name);
    }

    pub fn xlat_tile(&self, tname: &str) -> Option<Coord> {
        self.tile_lut.get(tname).copied()
    }

    pub fn handle_int(&mut self) {
        for slr in self.grid.slrs() {
            for col in slr.cols() {
                for row in slr.rows() {
                    if let Some(et) = &slr[(col, row)] {
                        let crd;
                        if let Some(c) = self.xlat_tile(&et.name) {
                            crd = c;
                        } else {
                            println!("MISSING INT TILE {}", et.name);
                            continue;
                        }
                        let mut bh = HashSet::new();
                        let mut missing = HashSet::new();
                        let mut wires = BTreeSet::new();
                        let mut missing_t = HashSet::new();
                        let mut missing_f = HashSet::new();
                        let node = &self.db.nodes[et.kind];
                        let naming = &self.db.namings[et.naming];
                        for (wt, wfs) in &node.muxes {
                            wires.insert(wt);
                            for &wf in &wfs.ins {
                                wires.insert(wf);
                            }
                        }
                        for w in wires {
                            if matches!(self.db.wires[w].kind, int::WireKind::ClkOut(_)) {
                                continue;
                            }
                            if let Some(wire) = self.grid.resolve_wire_raw((slr.slr, (col, row), w)) {
                                if let Some(n) = naming.get(w) {
                                    if !self.pin_int_wire(crd, n, wire) {
                                        missing.insert(w);
                                    }
                                } else {
                                    missing.insert(w);
                                }
                            } else {
                                bh.insert(w);
                            }
                        }
                        for (wt, wfs) in &node.muxes {
                            if bh.contains(&wt) {
                                continue;
                            }
                            if missing.contains(&wt) {
                                missing_t.insert(wt);
                                continue;
                            }
                            for &wf in &wfs.ins {
                                if bh.contains(&wf) {
                                    continue;
                                }
                                if missing.contains(&wf) {
                                    missing_f.insert(wf);
                                    continue;
                                }
                                self.claim_pip(crd, &naming[wt], &naming[wf]);
                            }
                        }
                        for w in missing_t {
                            if missing_f.contains(&w) {
                                println!("MISSING INT WIRE {} {}", et.name, self.db.wires[w].name);
                            }
                        }
                        if let Some(ref tn) = et.tie_name {
                            let mut pins = vec![];
                            for (w, wi) in &self.db.wires {
                                match wi.kind {
                                    int::WireKind::Tie0 => {
                                        pins.push(SitePin {
                                            dir: SitePinDir::Out,
                                            pin: self.grid.tie_pin_gnd.as_ref().unwrap(),
                                            wire: Some(&naming[w]),
                                        });
                                    }
                                    int::WireKind::Tie1 => {
                                        pins.push(SitePin {
                                            dir: SitePinDir::Out,
                                            pin: self.grid.tie_pin_vcc.as_ref().unwrap(),
                                            wire: Some(&naming[w]),
                                        });
                                    }
                                    int::WireKind::TiePullup => {
                                        pins.push(SitePin {
                                            dir: SitePinDir::Out,
                                            pin: self.grid.tie_pin_pullup.as_ref().unwrap(),
                                            wire: Some(&naming[w]),
                                        });
                                    }
                                    _ => (),
                                }
                            }
                            self.claim_site(crd, tn, self.grid.tie_kind.as_ref().unwrap(), &pins);
                        }
                    }
                }
            }
            for col in slr.cols() {
                for row in slr.rows() {
                    if let Some(et) = &slr[(col, row)] {
                        for t in et.terms.values() {
                            if let Some(t) = t {
                                let crd;
                                if let Some(n) = &t.tile {
                                    if let Some(c) = self.xlat_tile(n) {
                                        crd = c;
                                    } else {
                                        println!("MISSING TERM TILE {}", n);
                                        continue;
                                    }
                                } else {
                                    continue;
                                }
                                let crd_far;
                                if let Some(n) = &t.tile_far {
                                    if let Some(c) = self.xlat_tile(n) {
                                        crd_far = Some(c);
                                    } else {
                                        println!("MISSING PASS TILE {}", n);
                                        continue;
                                    }
                                } else {
                                    crd_far = None;
                                }
                                let naming_near = &self.db.namings[t.naming_near.unwrap()];
                                let naming_near_in = t.naming_near_in.map(|x| &self.db.namings[x]);
                                let naming_far = t.naming_far.map(|x| &self.db.namings[x]);
                                let naming_far_out = t.naming_far_out.map(|x| &self.db.namings[x]);
                                let naming_far_in = t.naming_far_in.map(|x| &self.db.namings[x]);
                                for (wt, ti) in &self.db.terms[t.kind].wires {
                                    if let Some(wiret) = self.grid.resolve_wire_raw((slr.slr, (col, row), wt)) {
                                        match *ti {
                                            int::TermInfo::Pass(wf) => {
                                                if naming_near.contains_id(wt) {
                                                    match wf {
                                                        int::TermWireIn::Near(wf) => {
                                                            if !self.pin_int_wire(crd, &naming_near[wt], wiret) {
                                                                println!("MISSING INT WIRE {} {}", t.tile.as_ref().unwrap(), naming_near[wt]);
                                                            }
                                                            if let Some(wiref) = self.grid.resolve_wire_raw((slr.slr, (col, row), wf)) {
                                                                if let Some(naming_near_in) = naming_near_in {
                                                                    if !self.pin_int_wire(crd, &naming_near_in[wt], wiref) {
                                                                        println!("MISSING INT WIRE {} {}", t.tile.as_ref().unwrap(), naming_near_in[wt]);
                                                                    }
                                                                    self.claim_pip(crd, &naming_near[wt], &naming_near_in[wt]);
                                                                } else {
                                                                    if !naming_near.contains_id(wf) {
                                                                        continue;
                                                                    }
                                                                    if !self.pin_int_wire(crd, &naming_near[wf], wiref) {
                                                                        println!("MISSING INT WIRE {} {}", t.tile.as_ref().unwrap(), naming_near[wf]);
                                                                    }
                                                                    self.claim_pip(crd, &naming_near[wt], &naming_near[wf]);
                                                                }
                                                            }
                                                        }
                                                        int::TermWireIn::Far(wf) => {
                                                            if let Some(wiref) = self.grid.resolve_wire_raw((slr.slr, t.target.unwrap(), wf)) {
                                                                if self.missing_int_wires.contains(&wiret) || self.missing_int_wires.contains(&wiref) {
                                                                    continue;
                                                                }
                                                                if !naming_far.unwrap().contains_id(wf) {
                                                                    continue;
                                                                }
                                                                if let Some(crd_far) = crd_far {
                                                                    if !self.pin_int_wire(crd_far, &naming_far_in.unwrap()[wf], wiref) {
                                                                        continue;
                                                                    }
                                                                    self.claim_node(&[
                                                                        (crd, &naming_far.unwrap()[wf]),
                                                                        (crd_far, &naming_far_out.unwrap()[wf]),
                                                                    ]);
                                                                    self.claim_pip(crd_far, &naming_far_out.unwrap()[wf], &naming_far_in.unwrap()[wf]);
                                                                } else {
                                                                    if !self.pin_int_wire(crd, &naming_far.unwrap()[wf], wiref) {
                                                                        continue;
                                                                    }
                                                                }
                                                                if !self.pin_int_wire(crd, &naming_near[wt], wiret) {
                                                                    continue;
                                                                }
                                                                self.claim_pip(crd, &naming_near[wt], &naming_far.unwrap()[wf]);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            int::TermInfo::Mux(ref wfs) => {
                                                if !self.pin_int_wire(crd, &naming_near[wt], wiret) {
                                                    println!("MISSING INT WIRE {} {}", t.tile.as_ref().unwrap(), naming_near[wt]);
                                                }
                                                for &wf in wfs {
                                                    match wf {
                                                        int::TermWireIn::Near(wf) => {
                                                            if let Some(wiref) = self.grid.resolve_wire_raw((slr.slr, (col, row), wf)) {
                                                                if !naming_near.contains_id(wf) {
                                                                    continue;
                                                                }
                                                                if !self.pin_int_wire(crd, &naming_near[wf], wiref) {
                                                                    continue;
                                                                }
                                                                self.claim_pip(crd, &naming_near[wt], &naming_near[wf]);
                                                            }
                                                        }
                                                        int::TermWireIn::Far(wf) => {
                                                            if let Some(wiref) = self.grid.resolve_wire_raw((slr.slr, t.target.unwrap(), wf)) {
                                                                if !naming_far.unwrap().contains_id(wf) {
                                                                    continue;
                                                                }
                                                                if let Some(crd_far) = crd_far {
                                                                    if !self.pin_int_wire(crd_far, &naming_far_in.unwrap()[wf], wiref) {
                                                                        println!("MISSING INT WIRE {} {}", t.tile_far.as_ref().unwrap(), naming_far_in.unwrap()[wf]);
                                                                    }
                                                                    self.claim_node(&[
                                                                        (crd, &naming_far.unwrap()[wf]),
                                                                        (crd_far, &naming_far_out.unwrap()[wf]),
                                                                    ]);
                                                                    self.claim_pip(crd_far, &naming_far_out.unwrap()[wf], &naming_far_in.unwrap()[wf]);
                                                                } else {
                                                                    if !self.pin_int_wire(crd, &naming_far.unwrap()[wf], wiref) {
                                                                        println!("MISSING INT WIRE {} {}", t.tile.as_ref().unwrap(), naming_far.unwrap()[wf]);
                                                                    }
                                                                }
                                                                self.claim_pip(crd, &naming_near[wt], &naming_far.unwrap()[wf]);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            _ => (),
                                        }
                                    }
                                }
                            }
                        }
                        for intf in &et.intfs {
                            if let Some(crd) = self.xlat_tile(&intf.name) {
                                let ik = &self.db.intfs[intf.kind];
                                let naming_int = &self.db.namings[intf.naming_int];
                                let naming_buf = intf.naming_buf.map(|x| &self.db.namings[x]);
                                let naming_site = intf.naming_site.map(|x| &self.db.namings[x]);
                                let naming_delay = intf.naming_delay.map(|x| &self.db.namings[x]);
                                let mut buf_wires = HashSet::new();
                                for (wt, ii) in &ik.wires {
                                    self.pin_int_wire(crd, &naming_int[wt], (slr.slr, (col, row), wt));
                                    match ii {
                                        int::IntfInfo::InputDelay => {
                                            let wire = &naming_site.unwrap()[wt];
                                            if !self.pin_int_site_wire(crd, wire, (slr.slr, (col, row), wt)) {
                                                let tname = &intf.name;
                                                println!("INT SITE NODE MISSING FOR {p} {tname} {wire} {wn}", p=self.rd.part, wn = self.db.wires[wt].name);
                                            }
                                            self.claim_node(&[(crd, &naming_delay.unwrap()[wt])]);
                                            self.claim_pip(crd, &naming_delay.unwrap()[wt], &naming_int[wt]);
                                            self.claim_pip(crd, &naming_site.unwrap()[wt], &naming_int[wt]);
                                            self.claim_pip(crd, &naming_site.unwrap()[wt], &naming_delay.unwrap()[wt]);
                                        }
                                        int::IntfInfo::OutputTestMux(wfs) => {
                                            if let Some(naming_site) = naming_site {
                                                if naming_site.contains_id(wt) {
                                                    if self.pin_int_site_wire(crd, &naming_site[wt], (slr.slr, (col, row), wt)) {
                                                        self.claim_pip(crd, &naming_int[wt], &naming_site[wt]);
                                                    }
                                                }
                                            }
                                            for &wf in wfs {
                                                self.pin_int_wire(crd, &naming_int[wf], (slr.slr, (col, row), wf));
                                                if matches!(ik.wires.get(wf), Some(&int::IntfInfo::InputDelay)) {
                                                    self.claim_pip(crd, &naming_int[wt], &naming_site.unwrap()[wf]);
                                                } else if let Some(naming_buf) = naming_buf {
                                                    if naming_buf.contains_id(wf) {
                                                        buf_wires.insert(wf);
                                                        self.claim_pip(crd, &naming_int[wt], &naming_buf[wf]);
                                                    } else {
                                                        self.claim_pip(crd, &naming_int[wt], &naming_int[wf]);
                                                    }
                                                } else {
                                                    self.claim_pip(crd, &naming_int[wt], &naming_int[wf]);
                                                }
                                            }
                                        }
                                    }
                                }
                                if let Some(naming_buf) = naming_buf {
                                    for w in buf_wires {
                                        self.claim_node(&[(crd, &naming_buf[w])]);
                                        self.claim_pip(crd, &naming_buf[w], &naming_int[w]);
                                    }
                                }
                            } else {
                                println!("MISSING INTF TILE {} {}", self.rd.part, intf.name);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn finish(self) {
    }
}
