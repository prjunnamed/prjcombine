use prjcombine_xilinx_rawdump::{Part, Coord, self as rawdump};
use prjcombine_xilinx_geom::{int, eint, eint::IntWire};
use std::collections::{HashMap, HashSet, BTreeSet};
use bitvec::vec::BitVec;
use prjcombine_entity::EntityId;

pub struct Verifier<'a> {
    pub rd: &'a Part,
    pub db: &'a int::IntDb,
    pub grid: &'a eint::ExpandedGrid<'a>,
    pub tile_lut: HashMap<String, Coord>,
    claimed_nodes: BitVec,
    claimed_twires: HashMap<Coord, BitVec>,
    claimed_pips: HashMap<Coord, BitVec>,
    int_wires: HashMap<IntWire, NodeOrWire>,
    int_site_wires: HashMap<IntWire, NodeOrWire>,
    missing_int_wires: HashSet<IntWire>,
    missing_int_site_wires: HashSet<IntWire>,
}

pub struct SitePin<'a> {
    pub dir: SitePinDir,
    pub pin: &'a str,
    pub wire: Option<&'a str>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum NodeOrWire {
    Node(rawdump::NodeId),
    Wire(Coord, rawdump::TkWireId),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SitePinDir {
    In,
    Out,
    Inout,
}

impl<'a> Verifier<'a> {
    pub fn new(rd: &'a Part, grid: &'a eint::ExpandedGrid) -> Self {
        let mut res = Self {
            rd,
            db: grid.db,
            grid,
            tile_lut: rd.tiles.iter().map(|(&c, t)| (t.name.clone(), c)).collect(),
            claimed_nodes: BitVec::new(),
            claimed_twires: HashMap::new(),
            claimed_pips: HashMap::new(),
            int_wires: HashMap::new(),
            int_site_wires: HashMap::new(),
            missing_int_wires: HashSet::new(),
            missing_int_site_wires: HashSet::new(),
        };
        res.handle_int();
        res
    }

    fn lookup_wire(&self, crd: Coord, wire: &str) -> Option<NodeOrWire> {
        let tile = &self.rd.tiles[&crd];
        let tk = &self.rd.tile_kinds[tile.kind];
        let widx = self.rd.wires.get(wire)?;
        match tk.wires.get(&widx)? {
            (twi, rawdump::TkWire::Internal(_, _)) => {
                Some(NodeOrWire::Wire(crd, twi))
            }
            (_, &rawdump::TkWire::Connected(idx)) => {
                match tile.conn_wires.get(idx) {
                    Some(&nidx) => Some(NodeOrWire::Node(nidx)),
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
                            let nidx = nidx.to_idx();
                            if nidx >= self.claimed_nodes.len() {
                                self.claimed_nodes.resize(nidx + 1, false);
                            }
                            if self.claimed_nodes[nidx] {
                                println!("DOUBLE CLAIMED NODE {tname} {wn}");
                            }
                            self.claimed_nodes.set(nidx, true);
                        }
                        NodeOrWire::Wire(crd, widx) => {
                            let widx = widx.to_idx();
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
        let tk = &self.rd.tile_kinds[tile.kind];
        let tname = &tile.name;
        let wti = if let Some(wti) = self.rd.wires.get(wt) {
            wti
        } else {
            println!("MISSING PIP DEST WIRE {tname} {wt}");
            return;
        };
        let wfi = if let Some(wfi) = self.rd.wires.get(wf) {
            wfi
        } else {
            println!("MISSING PIP SRC WIRE {tname} {wf}");
            return;
        };
        if let Some((idx, _)) = tk.pips.get(&(wfi, wti)) {
            let idx = idx.to_idx();
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
        let tk = &self.rd.tile_kinds[tile.kind];
        for (i, n) in tile.sites.iter() {
            if n == name {
                let site = &tk.sites[i];
                if site.kind != kind {
                    println!("MISMATCHED SITE KIND {} {} {} {} {}", self.rd.part, tile.name, name, kind, site.kind);
                }
                // XXX pins
                return;
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
                        let naming = &self.db.node_namings[et.naming];
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
                                let naming = &self.db.term_namings[t.naming.unwrap()];
                                for (wt, ti) in &self.db.terms[t.kind].wires {
                                    if let Some(wiret) = self.grid.resolve_wire_raw((slr.slr, (col, row), wt)) {
                                        match *ti {
                                            int::TermInfo::Pass(wf) => {
                                                match naming.wires_out.get(wt) {
                                                    None => (),
                                                    Some(&int::TermWireOutNaming::Simple(ref wtn)) => {
                                                        match wf {
                                                            int::TermWireIn::Near(wf) => {
                                                                if !self.pin_int_wire(crd, wtn, wiret) {
                                                                    println!("MISSING INT WIRE {} {}", t.tile.as_ref().unwrap(), wtn);
                                                                }
                                                                if let Some(wiref) = self.grid.resolve_wire_raw((slr.slr, (col, row), wf)) {
                                                                    let wfn = &naming.wires_in_near[wf];
                                                                    if !self.pin_int_wire(crd, wfn, wiref) {
                                                                        println!("MISSING INT WIRE {} {}", t.tile.as_ref().unwrap(), wfn);
                                                                    }
                                                                    self.claim_pip(crd, wtn, wfn);
                                                                }
                                                            }
                                                            int::TermWireIn::Far(wf) => {
                                                                if let Some(wiref) = self.grid.resolve_wire_raw((slr.slr, t.target.unwrap(), wf)) {
                                                                    if self.missing_int_wires.contains(&wiret) || self.missing_int_wires.contains(&wiref) {
                                                                        continue;
                                                                    }
                                                                    match naming.wires_in_far[wf] {
                                                                        int::TermWireInFarNaming::Simple(ref wfn) => {
                                                                            if !self.pin_int_wire(crd, wfn, wiref) {
                                                                                continue;
                                                                            }
                                                                            if !self.pin_int_wire(crd, wtn, wiret) {
                                                                                continue;
                                                                            }
                                                                            self.claim_pip(crd, wtn, wfn);
                                                                        }
                                                                        int::TermWireInFarNaming::Buf(ref wfn, ref wfin) => {
                                                                            if !self.pin_int_wire(crd, wfin, wiref) {
                                                                                continue;
                                                                            }
                                                                            if !self.pin_int_wire(crd, wtn, wiret) {
                                                                                continue;
                                                                            }
                                                                            self.claim_pip(crd, wtn, wfn);
                                                                            self.claim_pip(crd, wfn, wfin);
                                                                        }
                                                                        int::TermWireInFarNaming::BufFar(ref wfn, ref wffon, ref wffin) => {
                                                                            if !self.pin_int_wire(crd_far.unwrap(), wffin, wiref) {
                                                                                continue;
                                                                            }
                                                                            self.claim_node(&[(crd, wfn), (crd_far.unwrap(), wffon)]);
                                                                            self.claim_pip(crd_far.unwrap(), wffon, wffin);
                                                                            if !self.pin_int_wire(crd, wtn, wiret) {
                                                                                continue;
                                                                            }
                                                                            self.claim_pip(crd, wtn, wfn);
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    Some(&int::TermWireOutNaming::Buf(ref wtn, ref wfn)) => {
                                                        match wf {
                                                            int::TermWireIn::Near(wf) => {
                                                                if !self.pin_int_wire(crd, wtn, wiret) {
                                                                    println!("MISSING INT WIRE {} {}", t.tile.as_ref().unwrap(), wtn);
                                                                }
                                                                if let Some(wiref) = self.grid.resolve_wire_raw((slr.slr, (col, row), wf)) {
                                                                    if !self.pin_int_wire(crd, wfn, wiref) {
                                                                        println!("MISSING INT WIRE {} {}", t.tile.as_ref().unwrap(), wfn);
                                                                    }
                                                                    self.claim_pip(crd, wtn, wfn);
                                                                }
                                                            }
                                                            int::TermWireIn::Far(_) => unreachable!(),
                                                        }
                                                    }
                                                }
                                            }
                                            int::TermInfo::Mux(ref wfs) => {
                                                let wtn = match naming.wires_out[wt] {
                                                    int::TermWireOutNaming::Simple(ref wtn) => wtn,
                                                    _ => unreachable!(),
                                                };
                                                if !self.pin_int_wire(crd, wtn, wiret) {
                                                    println!("MISSING INT WIRE {} {}", t.tile.as_ref().unwrap(), wtn);
                                                }
                                                for &wf in wfs {
                                                    match wf {
                                                        int::TermWireIn::Near(wf) => {
                                                            if let Some(wiref) = self.grid.resolve_wire_raw((slr.slr, (col, row), wf)) {
                                                                if !naming.wires_in_near.contains_id(wf) {
                                                                    continue;
                                                                }
                                                                let wfn = &naming.wires_in_near[wf];
                                                                if !self.pin_int_wire(crd, wfn, wiref) {
                                                                    continue;
                                                                }
                                                                self.claim_pip(crd, wtn, wfn);
                                                            }
                                                        }
                                                        int::TermWireIn::Far(wf) => {
                                                            if let Some(wiref) = self.grid.resolve_wire_raw((slr.slr, t.target.unwrap(), wf)) {
                                                                match naming.wires_in_far[wf] {
                                                                    int::TermWireInFarNaming::Simple(ref wfn) => {
                                                                        if !self.pin_int_wire(crd, wfn, wiref) {
                                                                            println!("MISSING INT WIRE {} {}", t.tile.as_ref().unwrap(), wfn);
                                                                        }
                                                                        self.claim_pip(crd, wtn, wfn);
                                                                    }
                                                                    int::TermWireInFarNaming::Buf(ref wfn, ref wfin) => {
                                                                        if !self.pin_int_wire(crd, wfin, wiref) {
                                                                            println!("MISSING INT WIRE {} {}", t.tile.as_ref().unwrap(), wfin);
                                                                        }
                                                                        self.claim_pip(crd, wtn, wfn);
                                                                        self.claim_pip(crd, wfn, wfin);
                                                                    }
                                                                    int::TermWireInFarNaming::BufFar(ref wfn, ref wffon, ref wffin) => {
                                                                        if !self.pin_int_wire(crd_far.unwrap(), wffin, wiref) {
                                                                            println!("MISSING INT WIRE {} {}", t.tile_far.as_ref().unwrap(), wffin);
                                                                        }
                                                                        self.claim_node(&[(crd, wfn), (crd_far.unwrap(), wffon)]);
                                                                        self.claim_pip(crd_far.unwrap(), wffon, wffin);
                                                                        self.claim_pip(crd, wtn, wfn);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            // XXX BiSplitter
                                            _ => (),
                                        }
                                    }
                                }
                            }
                        }
                        for intf in &et.intfs {
                            if let Some(crd) = self.xlat_tile(&intf.name) {
                                let ik = &self.db.intfs[intf.kind];
                                let naming = &self.db.intf_namings[intf.naming];
                                for (wt, ii) in &ik.wires {
                                    match ii {
                                        int::IntfInfo::InputDelay => {
                                            if let int::IntfWireInNaming::Delay(ref wton, ref wtdn, ref wtn) = naming.wires_in[wt] {
                                                self.pin_int_wire(crd, wtn, (slr.slr, (col, row), wt));
                                                if !self.pin_int_site_wire(crd, wton, (slr.slr, (col, row), wt)) {
                                                    let tname = &intf.name;
                                                    println!("INT SITE NODE MISSING FOR {p} {tname} {wton} {wn}", p=self.rd.part, wn = self.db.wires[wt].name);
                                                }
                                                self.claim_node(&[(crd, wtdn)]);
                                                self.claim_pip(crd, wtdn, wtn);
                                                self.claim_pip(crd, wton, wtn);
                                                self.claim_pip(crd, wton, wtdn);
                                            } else {
                                                unreachable!()
                                            }
                                        }
                                        int::IntfInfo::OutputTestMux(wfs) => {
                                            let wtn = match naming.wires_out[wt] {
                                                int::IntfWireOutNaming::Simple(ref wtn) => {
                                                    self.pin_int_wire(crd, wtn, (slr.slr, (col, row), wt));
                                                    wtn
                                                }
                                                int::IntfWireOutNaming::Buf(ref wtn, ref wsn) => {
                                                    self.pin_int_wire(crd, wtn, (slr.slr, (col, row), wt));
                                                    if self.pin_int_site_wire(crd, wsn, (slr.slr, (col, row), wt)) {
                                                        self.claim_pip(crd, wtn, wsn);
                                                    }
                                                    wtn
                                                }
                                            };
                                            for &wf in wfs {
                                                match naming.wires_in[wf] {
                                                    int::IntfWireInNaming::Simple(ref wfn) => {
                                                        self.pin_int_wire(crd, wfn, (slr.slr, (col, row), wf));
                                                        self.claim_pip(crd, wtn, wfn);
                                                    }
                                                    int::IntfWireInNaming::TestBuf(ref wfbn, ref wfn) => {
                                                        self.pin_int_wire(crd, wfn, (slr.slr, (col, row), wf));
                                                        self.claim_pip(crd, wtn, wfbn);
                                                    }
                                                    int::IntfWireInNaming::Delay(ref wfon, _, ref wfn) => {
                                                        self.pin_int_wire(crd, wfn, (slr.slr, (col, row), wf));
                                                        self.claim_pip(crd, wtn, wfon);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                for (_, iwin) in &naming.wires_in {
                                    if let &int::IntfWireInNaming::TestBuf(ref wfbn, ref wfn) = iwin {
                                        self.claim_node(&[(crd, wfbn)]);
                                        self.claim_pip(crd, wfbn, wfn);
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
