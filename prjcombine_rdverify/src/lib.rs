#![allow(clippy::collapsible_else_if)]

use prjcombine_entity::{EntityBitVec, EntityId, EntityPartVec};
use prjcombine_int::db::{
    BelId, BelInfo, BelNaming, IntDb, IntfInfo, IntfWireInNaming, IntfWireOutNaming, NodeRawTileId,
    NodeTileId, PinDir, TermInfo, TermWireInFarNaming, TermWireOutNaming, WireKind,
};
use prjcombine_int::grid::{
    ColId, DieId, ExpandedGrid, ExpandedTileIntf, ExpandedTileNode, ExpandedTileTerm, IntWire,
    RowId,
};
use prjcombine_rawdump::{self as rawdump, Coord, Part};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug)]
pub struct BelContext<'a> {
    pub die: DieId,
    pub col: ColId,
    pub row: RowId,
    pub node: &'a ExpandedTileNode,
    pub node_kind: &'a str,
    pub bid: BelId,
    pub key: &'a str,
    pub bel: &'a BelInfo,
    pub naming: &'a BelNaming,
    pub name: Option<&'a str>,
    pub crds: EntityPartVec<NodeRawTileId, Coord>,
}

impl<'a> BelContext<'a> {
    pub fn crd(&self) -> Coord {
        self.crds[self.naming.tile]
    }

    pub fn wire(&self, name: &str) -> &'a str {
        &self.naming.pins[name].name
    }

    pub fn wire_far(&self, name: &str) -> &'a str {
        &self.naming.pins[name].name_far
    }

    pub fn fwire(&self, name: &str) -> (Coord, &'a str) {
        (self.crd(), self.wire(name))
    }

    pub fn fwire_far(&self, name: &str) -> (Coord, &'a str) {
        (self.crd(), self.wire_far(name))
    }
}

pub struct Verifier<'a> {
    pub rd: &'a Part,
    pub db: &'a IntDb,
    pub grid: &'a ExpandedGrid<'a>,
    pub tile_lut: HashMap<String, Coord>,
    claimed_nodes: EntityBitVec<rawdump::NodeId>,
    claimed_twires: HashMap<Coord, EntityBitVec<rawdump::TkWireId>>,
    claimed_pips: HashMap<Coord, EntityBitVec<rawdump::TkPipId>>,
    claimed_sites: HashMap<Coord, EntityBitVec<rawdump::TkSiteId>>,
    int_wires: HashMap<IntWire, NodeOrWire>,
    int_site_wires: HashMap<IntWire, NodeOrWire>,
    missing_int_wires: HashSet<IntWire>,
    missing_int_site_wires: HashSet<IntWire>,
}

#[derive(Debug, Clone)]
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
    #[allow(dead_code)]
    Inout,
}

impl<'a> Verifier<'a> {
    pub fn new(rd: &'a Part, grid: &'a ExpandedGrid) -> Self {
        let mut res = Self {
            rd,
            db: grid.db,
            grid,
            tile_lut: rd.tiles.iter().map(|(&c, t)| (t.name.clone(), c)).collect(),
            claimed_nodes: EntityBitVec::repeat(false, rd.nodes.len()),
            claimed_twires: rd
                .tiles
                .iter()
                .map(|(&k, v)| {
                    (
                        k,
                        EntityBitVec::repeat(false, rd.tile_kinds[v.kind].wires.len()),
                    )
                })
                .collect(),
            claimed_pips: rd
                .tiles
                .iter()
                .map(|(&k, v)| {
                    (
                        k,
                        EntityBitVec::repeat(false, rd.tile_kinds[v.kind].pips.len()),
                    )
                })
                .collect(),
            claimed_sites: rd
                .tiles
                .iter()
                .map(|(&k, v)| {
                    (
                        k,
                        EntityBitVec::repeat(false, rd.tile_kinds[v.kind].sites.len()),
                    )
                })
                .collect(),
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
            (twi, rawdump::TkWire::Internal(_, _)) => Some(NodeOrWire::Wire(crd, twi)),
            (_, &rawdump::TkWire::Connected(idx)) => match tile.conn_wires.get(idx) {
                Some(&nidx) => Some(NodeOrWire::Node(nidx)),
                _ => None,
            },
        }
    }

    pub fn pin_int_wire(&mut self, crd: Coord, wire: &str, iw: IntWire) -> bool {
        if let Some(&nw) = self.int_wires.get(&iw) {
            if let Some(cnw) = self.lookup_wire(crd, wire) {
                if nw != cnw {
                    let tname = &self.rd.tiles[&crd].name;
                    println!(
                        "INT NODE MISMATCH FOR {p} {tname} {wire} {iw:?} {wn}",
                        p = self.rd.part,
                        wn = self.db.wires[iw.2].name
                    );
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
        if let Some(&nw) = self.int_site_wires.get(&iw) {
            if let Some(cnw) = self.lookup_wire(crd, wire) {
                if nw != cnw {
                    let tname = &self.rd.tiles[&crd].name;
                    println!(
                        "INT SITE NODE MISMATCH FOR {p} {tname} {wire} {iw:?} {wn}",
                        p = self.rd.part,
                        wn = self.db.wires[iw.2].name
                    );
                }
                true
            } else {
                let tname = &self.rd.tiles[&crd].name;
                println!("INT SITE NODE PRESENT FOR {tname} {wire} BUT WIRE NOT FOUND");
                false
            }
        } else if self.missing_int_site_wires.contains(&iw) {
            if let Some(cnw) = self.lookup_wire(crd, wire) {
                let tname = &self.rd.tiles[&crd].name;
                println!("INT SITE NODE PRESENT FOR {tname} {wire} BUT WAS MISSING PREVIOUSLY");
                self.claim_node(&[(crd, wire)]);
                self.int_site_wires.insert(iw, cnw);
                true
            } else {
                let tname = &self.rd.tiles[&crd].name;
                println!("INT SITEE WIRE {tname} {wire} MISSING TWICE");
                false
            }
        } else if let Some(cnw) = self.lookup_wire(crd, wire) {
            self.claim_node(&[(crd, wire)]);
            self.int_site_wires.insert(iw, cnw);
            true
        } else {
            self.missing_int_site_wires.insert(iw);
            false
        }
    }

    pub fn verify_node(&mut self, tiles: &[(Coord, &str)]) {
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
                }
            } else {
                println!("MISSING WIRE {tname} {wn}");
            }
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
                            if self.claimed_nodes[nidx] {
                                println!(
                                    "DOUBLE CLAIMED NODE {part} {tname} {wn}",
                                    part = self.rd.part
                                );
                            }
                            self.claimed_nodes.set(nidx, true);
                        }
                        NodeOrWire::Wire(crd, widx) => {
                            let ctw = self.claimed_twires.get_mut(&crd).unwrap();
                            if ctw[widx] {
                                println!(
                                    "DOUBLE CLAIMED NODE {part} {tname} {wn}",
                                    part = self.rd.part
                                );
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
            let ctp = self.claimed_pips.get_mut(&crd).unwrap();
            if ctp[idx] {
                println!("DOUBLE CLAIMED PIP {tname} {wt} <- {wf}");
            }
            ctp.set(idx, true);
        } else {
            println!("MISSING PIP {p} {tname} {wt} <- {wf}", p = self.rd.part);
        }
    }

    pub fn claim_site(&mut self, crd: Coord, name: &str, kind: &str, pins: &[SitePin<'_>]) {
        let tile = &self.rd.tiles[&crd];
        let tk = &self.rd.tile_kinds[tile.kind];
        for (i, n) in tile.sites.iter() {
            if n == name {
                let site = &tk.sites[i];
                let cts = self.claimed_sites.get_mut(&crd).unwrap();
                if cts[i] {
                    println!("DOUBLE CLAIMED SITE {name}");
                }
                cts.set(i, true);
                if site.kind != kind {
                    println!(
                        "MISMATCHED SITE KIND {} {} {} {} {}",
                        self.rd.part, tile.name, name, kind, site.kind
                    );
                }
                let mut extra_pins: HashSet<_> = site.pins.keys().map(|x| &x[..]).collect();
                for pin in pins {
                    if let Some(tkp) = site.pins.get(pin.pin) {
                        extra_pins.remove(pin.pin);
                        let exp_dir = match pin.dir {
                            SitePinDir::In => rawdump::TkSitePinDir::Input,
                            SitePinDir::Out => rawdump::TkSitePinDir::Output,
                            SitePinDir::Inout => rawdump::TkSitePinDir::Bidir,
                        };
                        if tkp.dir != exp_dir {
                            println!(
                                "PIN DIR MISMATCH {} {} {} {} {} {:?} {:?}",
                                self.rd.part, tile.name, name, kind, pin.pin, tkp.dir, exp_dir
                            );
                        }
                        let act_wire = tkp.wire.map(|x| &*self.rd.wires[x]);
                        if pin.wire != act_wire {
                            println!(
                                "PIN WIRE MISMATCH {} {} {} {} {} {:?} {:?}",
                                self.rd.part, tile.name, name, kind, pin.pin, act_wire, pin.wire
                            );
                        }
                        // XXX wire
                    } else {
                        println!(
                            "MISSING PIN {} {} {} {} {}",
                            self.rd.part, tile.name, name, kind, pin.pin
                        );
                    }
                }
                for pin in extra_pins {
                    println!(
                        "EXTRA PIN {} {} {} {} {}",
                        self.rd.part, tile.name, name, kind, pin
                    );
                }
                return;
            }
        }
        println!("MISSING SITE {} {} {}", self.rd.part, tile.name, name);
    }

    pub fn xlat_tile(&self, tname: &str) -> Option<Coord> {
        self.tile_lut.get(tname).copied()
    }

    pub fn get_node_crds(
        &self,
        node: &ExpandedTileNode,
    ) -> Option<EntityPartVec<NodeRawTileId, rawdump::Coord>> {
        let mut crds = EntityPartVec::new();
        for (k, name) in &node.names {
            if let Some(c) = self.xlat_tile(name) {
                crds.insert(k, c);
            } else {
                println!("MISSING INT TILE {} {}", self.rd.part, name);
                return None;
            }
        }
        Some(crds)
    }

    pub fn handle_int_node(&mut self, die: DieId, node: &ExpandedTileNode) {
        let crds;
        if let Some(c) = self.get_node_crds(node) {
            crds = c;
        } else {
            return;
        }
        let def_rt = NodeRawTileId::from_idx(0);
        let mut bh = HashSet::new();
        let mut missing = HashSet::new();
        let mut wires = BTreeSet::new();
        let mut missing_t = HashSet::new();
        let mut missing_f = HashSet::new();
        let mut found = HashSet::new();
        let kind = &self.db.nodes[node.kind];
        let naming = &self.db.node_namings[node.naming];
        for (&wt, wfs) in &kind.muxes {
            wires.insert(wt);
            for &wf in &wfs.ins {
                if !naming.ext_pips.contains_key(&(wt, wf)) {
                    wires.insert(wf);
                }
            }
        }
        for &w in &wires {
            match self.db.wires[w.1].kind {
                WireKind::Tie0 | WireKind::Tie1 | WireKind::TiePullup => {
                    if let Some(n) = naming.wires.get(&w) {
                        self.claim_node(&[(crds[def_rt], n)]);
                    }
                }
                _ => {
                    if let Some(wire) = self.grid.resolve_wire_raw((die, node.tiles[w.0], w.1)) {
                        if let Some(n) = naming.wires.get(&w) {
                            if let Some(en) = naming.wire_bufs.get(&w) {
                                if !self.pin_int_wire(crds[en.tile], &en.wire_from, wire) {
                                    missing.insert(w);
                                } else {
                                    self.claim_node(&[
                                        (crds[def_rt], n),
                                        (crds[en.tile], &en.wire_to),
                                    ]);
                                }
                            } else {
                                if !self.pin_int_wire(crds[def_rt], n, wire) {
                                    missing.insert(w);
                                }
                            }
                            found.insert(w);
                        } else {
                            missing.insert(w);
                        }
                    } else {
                        bh.insert(w);
                    }
                }
            }
        }
        for (&wt, wfs) in &kind.muxes {
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
                if let Some(en) = naming.ext_pips.get(&(wt, wf)) {
                    let wire_f = self
                        .grid
                        .resolve_wire_raw((die, node.tiles[wf.0], wf.1))
                        .unwrap();
                    let wire_t = self
                        .grid
                        .resolve_wire_raw((die, node.tiles[wt.0], wt.1))
                        .unwrap();
                    if !crds.contains_id(en.tile)
                        || !self.pin_int_wire(crds[en.tile], &en.wire_from, wire_f)
                    {
                        if found.contains(&wf) {
                            println!(
                                "MISSING EXT INT WIRE {} {}",
                                node.names[en.tile], en.wire_from
                            );
                        } else {
                            missing_f.insert(wf);
                        }
                        continue;
                    }
                    if !self.pin_int_wire(crds[en.tile], &en.wire_to, wire_t) {
                        if found.contains(&wt) {
                            println!(
                                "MISSING EXT INT WIRE {} {}",
                                node.names[en.tile], en.wire_to
                            );
                        } else {
                            missing_t.insert(wt);
                        }
                        continue;
                    }
                    self.claim_pip(crds[en.tile], &en.wire_to, &en.wire_from);
                } else {
                    self.claim_pip(crds[def_rt], &naming.wires[&wt], &naming.wires[&wf]);
                }
            }
        }
        for w in missing_t {
            if missing_f.contains(&w) {
                println!(
                    "MISSING INT WIRE {} {}",
                    node.names[def_rt], self.db.wires[w.1].name
                );
            }
        }
        if let Some(ref tn) = node.tie_name {
            let mut pins = vec![];
            for (&k, v) in &naming.wires {
                let wi = &self.db.wires[k.1];
                match wi.kind {
                    WireKind::Tie0 => {
                        pins.push(SitePin {
                            dir: SitePinDir::Out,
                            pin: self.grid.tie_pin_gnd.as_ref().unwrap(),
                            wire: Some(v),
                        });
                    }
                    WireKind::Tie1 => {
                        pins.push(SitePin {
                            dir: SitePinDir::Out,
                            pin: self.grid.tie_pin_vcc.as_ref().unwrap(),
                            wire: Some(v),
                        });
                    }
                    WireKind::TiePullup => {
                        pins.push(SitePin {
                            dir: SitePinDir::Out,
                            pin: self.grid.tie_pin_pullup.as_ref().unwrap(),
                            wire: Some(v),
                        });
                    }
                    _ => (),
                }
            }
            self.claim_site(
                crds[def_rt],
                tn,
                self.grid.tie_kind.as_ref().unwrap(),
                &pins,
            );
        }

        for (id, _, bel) in &kind.bels {
            let bn = &naming.bels[id];
            for (k, v) in &bel.pins {
                let n = &bn.pins[k];
                let mut crd = crds[bn.tile];
                let mut wn: &str = &n.name;
                for pip in &n.pips {
                    let ncrd = crds[pip.tile];
                    wn = match v.dir {
                        PinDir::Input => {
                            self.claim_node(&[(crd, wn), (ncrd, &pip.wire_to)]);
                            self.claim_pip(ncrd, &pip.wire_to, &pip.wire_from);
                            &pip.wire_from
                        }
                        PinDir::Output => {
                            self.claim_node(&[(crd, wn), (ncrd, &pip.wire_from)]);
                            self.claim_pip(ncrd, &pip.wire_to, &pip.wire_from);
                            &pip.wire_to
                        }
                    };
                    crd = ncrd;
                }
                if n.pips.is_empty() {
                    wn = &n.name_far;
                }
                let mut claim = true;
                for &w in &v.wires {
                    let wire = self
                        .grid
                        .resolve_wire_raw((die, node.tiles[w.0], w.1))
                        .unwrap();
                    let wcrd;
                    let ww: &str;
                    if let Some(pip) = n.int_pips.get(&w) {
                        self.claim_pip(crds[pip.tile], &pip.wire_to, &pip.wire_from);
                        if v.dir == PinDir::Input {
                            self.verify_node(&[(crd, wn), (crds[pip.tile], &pip.wire_to)]);
                            wcrd = crds[pip.tile];
                            ww = &pip.wire_from;
                        } else {
                            self.verify_node(&[(crd, wn), (crds[pip.tile], &pip.wire_from)]);
                            wcrd = crds[pip.tile];
                            ww = &pip.wire_to;
                        }
                    } else {
                        wcrd = crd;
                        ww = wn;
                        claim = false;
                    }
                    if v.is_intf_in || n.is_intf_out {
                        self.pin_int_site_wire(wcrd, ww, wire);
                    } else {
                        self.pin_int_wire(wcrd, ww, wire);
                    }
                }
                if claim {
                    self.claim_node(&[(crd, wn)]);
                }
            }
        }
    }

    pub fn handle_int_term(&mut self, die: DieId, col: ColId, row: RowId, term: &ExpandedTileTerm) {
        let crd;
        if let Some(n) = &term.tile {
            if let Some(c) = self.xlat_tile(n) {
                crd = c;
            } else {
                println!("MISSING TERM TILE {}", n);
                return;
            }
        } else {
            return;
        }
        let crd_far;
        if let Some(n) = &term.tile_far {
            if let Some(c) = self.xlat_tile(n) {
                crd_far = Some(c);
            } else {
                println!("MISSING PASS TILE {}", n);
                return;
            }
        } else {
            crd_far = None;
        }
        let naming = &self.db.term_namings[term.naming.unwrap()];
        for (wt, ti) in &self.db.terms[term.kind].wires {
            if let Some(wiret) = self.grid.resolve_wire_raw((die, (col, row), wt)) {
                match *ti {
                    TermInfo::PassNear(wf) => match naming.wires_out.get(wt) {
                        None => (),
                        Some(&TermWireOutNaming::Simple(ref wtn)) => {
                            if !self.pin_int_wire(crd, wtn, wiret) {
                                println!(
                                    "MISSING INT WIRE {} {}",
                                    term.tile.as_ref().unwrap(),
                                    wtn
                                );
                            }
                            if let Some(wiref) = self.grid.resolve_wire_raw((die, (col, row), wf)) {
                                let wfn = &naming.wires_in_near[wf];
                                if !self.pin_int_wire(crd, wfn, wiref) {
                                    println!(
                                        "MISSING INT WIRE {} {}",
                                        term.tile.as_ref().unwrap(),
                                        wfn
                                    );
                                }
                                self.claim_pip(crd, wtn, wfn);
                            }
                        }
                        Some(&TermWireOutNaming::Buf(ref wtn, ref wfn)) => {
                            if !self.pin_int_wire(crd, wtn, wiret) {
                                println!(
                                    "MISSING INT WIRE {} {}",
                                    term.tile.as_ref().unwrap(),
                                    wtn
                                );
                            }
                            if let Some(wiref) = self.grid.resolve_wire_raw((die, (col, row), wf)) {
                                if !self.pin_int_wire(crd, wfn, wiref) {
                                    println!(
                                        "MISSING INT WIRE {} {}",
                                        term.tile.as_ref().unwrap(),
                                        wfn
                                    );
                                }
                                self.claim_pip(crd, wtn, wfn);
                            }
                        }
                    },
                    TermInfo::PassFar(wf) => match naming.wires_out.get(wt) {
                        None => (),
                        Some(&TermWireOutNaming::Simple(ref wtn)) => {
                            if let Some(wiref) =
                                self.grid.resolve_wire_raw((die, term.target.unwrap(), wf))
                            {
                                if self.missing_int_wires.contains(&wiret)
                                    || self.missing_int_wires.contains(&wiref)
                                {
                                    continue;
                                }
                                match naming.wires_in_far[wf] {
                                    TermWireInFarNaming::Simple(ref wfn) => {
                                        if !self.pin_int_wire(crd, wfn, wiref) {
                                            continue;
                                        }
                                        if !self.pin_int_wire(crd, wtn, wiret) {
                                            continue;
                                        }
                                        self.claim_pip(crd, wtn, wfn);
                                    }
                                    TermWireInFarNaming::Buf(ref wfn, ref wfin) => {
                                        if !self.pin_int_wire(crd, wfin, wiref) {
                                            continue;
                                        }
                                        if !self.pin_int_wire(crd, wtn, wiret) {
                                            continue;
                                        }
                                        self.claim_pip(crd, wtn, wfn);
                                        self.claim_pip(crd, wfn, wfin);
                                    }
                                    TermWireInFarNaming::BufFar(ref wfn, ref wffon, ref wffin) => {
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
                        Some(&TermWireOutNaming::Buf(_, _)) => unreachable!(),
                    },
                    _ => (),
                }
            }
        }
    }

    pub fn handle_int_intf(&mut self, die: DieId, col: ColId, row: RowId, intf: &ExpandedTileIntf) {
        let crd;
        if let Some(c) = self.xlat_tile(&intf.name) {
            crd = c;
        } else {
            println!("MISSING INTF TILE {} {}", self.rd.part, intf.name);
            return;
        }
        let ik = &self.db.intfs[intf.kind];
        let naming = &self.db.intf_namings[intf.naming];
        for (wt, ii) in &ik.wires {
            match ii {
                IntfInfo::InputDelay => {
                    if let IntfWireInNaming::Delay(ref wton, ref wtdn, ref wtn) =
                        naming.wires_in[wt]
                    {
                        self.pin_int_wire(crd, wtn, (die, (col, row), wt));
                        if !self.pin_int_site_wire(crd, wton, (die, (col, row), wt)) {
                            let tname = &intf.name;
                            println!(
                                "INT SITE NODE MISSING FOR {p} {tname} {wton} {wn}",
                                p = self.rd.part,
                                wn = self.db.wires[wt].name
                            );
                        }
                        self.claim_node(&[(crd, wtdn)]);
                        self.claim_pip(crd, wtdn, wtn);
                        self.claim_pip(crd, wton, wtn);
                        self.claim_pip(crd, wton, wtdn);
                    } else {
                        unreachable!()
                    }
                }
                IntfInfo::OutputTestMux(wfs) => {
                    let wtn = match naming.wires_out[wt] {
                        IntfWireOutNaming::Simple(ref wtn) => {
                            self.pin_int_wire(crd, wtn, (die, (col, row), wt));
                            wtn
                        }
                        IntfWireOutNaming::Buf(ref wtn, ref wsn) => {
                            self.pin_int_wire(crd, wtn, (die, (col, row), wt));
                            if self.pin_int_site_wire(crd, wsn, (die, (col, row), wt)) {
                                self.claim_pip(crd, wtn, wsn);
                            }
                            wtn
                        }
                    };
                    for &wf in wfs {
                        match naming.wires_in[wf] {
                            IntfWireInNaming::Simple(ref wfn) => {
                                self.pin_int_wire(crd, wfn, (die, (col, row), wf));
                                self.claim_pip(crd, wtn, wfn);
                            }
                            IntfWireInNaming::TestBuf(ref wfbn, ref wfn) => {
                                self.pin_int_wire(crd, wfn, (die, (col, row), wf));
                                self.claim_pip(crd, wtn, wfbn);
                            }
                            IntfWireInNaming::Buf(_, ref wfn) => {
                                self.pin_int_wire(crd, wfn, (die, (col, row), wf));
                                self.claim_pip(crd, wtn, wfn);
                            }
                            IntfWireInNaming::Delay(ref wfon, _, ref wfn) => {
                                self.pin_int_wire(crd, wfn, (die, (col, row), wf));
                                self.claim_pip(crd, wtn, wfon);
                            }
                        }
                    }
                }
            }
        }
        for (wf, iwin) in &naming.wires_in {
            if let &IntfWireInNaming::TestBuf(ref wfbn, ref wfn) = iwin {
                self.claim_node(&[(crd, wfbn)]);
                self.claim_pip(crd, wfbn, wfn);
            }
            if let &IntfWireInNaming::Buf(ref wfbn, ref wfn) = iwin {
                if self.pin_int_site_wire(crd, wfbn, (die, (col, row), wf)) {
                    self.claim_pip(crd, wfbn, wfn);
                }
            }
        }
    }

    pub fn handle_int(&mut self) {
        for die in self.grid.dies() {
            for col in die.cols() {
                for row in die.rows() {
                    for node in &die[(col, row)].nodes {
                        self.handle_int_node(die.die, node);
                    }
                }
            }
            for col in die.cols() {
                for row in die.rows() {
                    let et = &die[(col, row)];
                    for t in et.terms.values().flatten() {
                        self.handle_int_term(die.die, col, row, t);
                    }
                    for intf in &et.intfs {
                        self.handle_int_intf(die.die, col, row, intf);
                    }
                }
            }
        }
    }

    pub fn verify_bel(
        &mut self,
        bel: &BelContext<'_>,
        kind: &str,
        extras: &[(&str, SitePinDir)],
        skip: &[&str],
    ) {
        let mut pins = Vec::new();
        for (k, v) in &bel.bel.pins {
            if skip.contains(&&**k) {
                continue;
            }
            let n = &bel.naming.pins[k];
            pins.push(SitePin {
                dir: match v.dir {
                    PinDir::Input => SitePinDir::In,
                    PinDir::Output => SitePinDir::Out,
                },
                pin: k,
                wire: Some(&n.name),
            });
        }
        for (pin, dir) in extras.iter().copied() {
            pins.push(SitePin {
                dir,
                pin,
                wire: Some(&bel.naming.pins[pin].name),
            });
        }
        if let Some(name) = bel.name {
            self.claim_site(bel.crd(), name, kind, &pins);
        } else {
            println!("MISSING SITE NAME {:?} {}", bel.node.tiles, bel.key);
        }
    }

    pub fn get_bel(&self, die: DieId, node: &'a ExpandedTileNode, bid: BelId) -> BelContext<'a> {
        let crds = self.get_node_crds(node).unwrap();
        let nk = &self.db.nodes[node.kind];
        let nn = &self.db.node_namings[node.naming];
        let bel = &nk.bels[bid];
        let naming = &nn.bels[bid];
        let key = &**nk.bels.key(bid);
        let (col, row) = node.tiles[NodeTileId::from_idx(0)];
        BelContext {
            die,
            col,
            row,
            node,
            node_kind: self.db.nodes.key(node.kind),
            bid,
            key,
            bel,
            naming,
            crds,
            name: node.bels.get(bid).map(|x| &**x),
        }
    }

    pub fn find_bel(&self, die: DieId, coord: (ColId, RowId), key: &str) -> Option<BelContext<'a>> {
        let die = self.grid.die(die);
        let tile = die.tile(coord);
        for node in &tile.nodes {
            let nk = &self.db.nodes[node.kind];
            if let Some((id, _)) = nk.bels.get(key) {
                return Some(self.get_bel(die.die, node, id));
            }
        }
        None
    }

    pub fn find_bel_delta(
        &self,
        bel: &BelContext<'_>,
        dx: isize,
        dy: isize,
        key: &str,
    ) -> Option<BelContext<'a>> {
        let nc = bel.col.to_idx() as isize + dx;
        let nr = bel.row.to_idx() as isize + dy;
        if nc < 0 || nr < 0 {
            return None;
        }
        let nc = nc as usize;
        let nr = nr as usize;
        let die = self.grid.die(bel.die);
        if nc >= die.cols().len() || nr >= die.rows().len() {
            return None;
        }
        self.find_bel(bel.die, (ColId::from_idx(nc), RowId::from_idx(nr)), key)
    }

    pub fn find_bel_walk(
        &self,
        bel: &BelContext<'_>,
        dx: isize,
        dy: isize,
        key: &str,
    ) -> Option<BelContext<'a>> {
        let mut c = bel.col.to_idx();
        let mut r = bel.row.to_idx();
        loop {
            let nc = c as isize + dx;
            let nr = r as isize + dy;
            if nc < 0 || nr < 0 {
                return None;
            }
            c = nc as usize;
            r = nr as usize;
            let die = self.grid.die(bel.die);
            if c >= die.cols().len() || r >= die.rows().len() {
                return None;
            }
            if let Some(x) = self.find_bel(bel.die, (ColId::from_idx(c), RowId::from_idx(r)), key) {
                return Some(x);
            }
        }
    }

    pub fn find_bel_sibling(&self, bel: &BelContext<'_>, key: &str) -> BelContext<'a> {
        self.find_bel(bel.die, (bel.col, bel.row), key).unwrap()
    }

    pub fn finish(self) {}
}

pub fn verify(
    rd: &rawdump::Part,
    grid: &ExpandedGrid,
    bel_handler: impl Fn(&mut Verifier, &BelContext<'_>),
) {
    let mut vrf = Verifier::new(rd, grid);
    for die in grid.dies() {
        for col in die.cols() {
            for row in die.rows() {
                for node in &die[(col, row)].nodes {
                    let nk = &grid.db.nodes[node.kind];
                    for id in nk.bels.ids() {
                        let ctx = vrf.get_bel(die.die, node, id);
                        bel_handler(&mut vrf, &ctx);
                    }
                }
            }
        }
    }
    vrf.finish();
}
