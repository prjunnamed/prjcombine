#![allow(clippy::collapsible_else_if)]

use prjcombine_entity::{EntityBitVec, EntityId, EntityPartVec, EntityVec};
use prjcombine_int::db::{
    BelId, BelInfo, BelNaming, IntDb, IntfInfo, IntfWireInNaming, IntfWireOutNaming, NodeKindId,
    NodeRawTileId, NodeTileId, NodeWireId, PinDir, TermInfo, TermWireInFarNaming,
    TermWireOutNaming, WireId, WireKind,
};
use prjcombine_int::grid::{
    ColId, DieId, ExpandedGrid, ExpandedTileNode, ExpandedTileTerm, IntWire, RowId,
};
use prjcombine_rawdump::{self as rawdump, Coord, NodeOrWire, Part};
use std::collections::{HashMap, HashSet};

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

#[derive(Debug, Clone)]
pub struct SitePin<'a> {
    pub dir: SitePinDir,
    pub pin: &'a str,
    pub wire: Option<&'a str>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SitePinDir {
    In,
    Out,
    #[allow(dead_code)]
    Inout,
}

pub struct Verifier<'a> {
    pub rd: &'a Part,
    pub db: &'a IntDb,
    pub grid: &'a ExpandedGrid<'a>,
    pub tile_lut: HashMap<String, Coord>,
    dummy_in_nodes: HashSet<NodeOrWire>,
    dummy_out_nodes: HashSet<NodeOrWire>,
    claimed_nodes: EntityBitVec<rawdump::NodeId>,
    claimed_twires: HashMap<Coord, EntityBitVec<rawdump::TkWireId>>,
    claimed_pips: HashMap<Coord, EntityBitVec<rawdump::TkPipId>>,
    claimed_sites: HashMap<Coord, EntityBitVec<rawdump::TkSiteId>>,
    int_wire_data: HashMap<IntWire, IntWireData>,
    node_used: EntityVec<NodeKindId, NodeUsedInfo>,
    skip_residual: bool,
    stub_outs: HashSet<rawdump::WireId>,
    stub_ins: HashSet<rawdump::WireId>,
}

#[derive(Debug, Default)]
struct IntWireData {
    used_o: bool,
    used_i: bool,
    node: Option<NodeOrWire>,
    has_intf_i: bool,
    has_intf_o: bool,
    intf_node: Option<NodeOrWire>,
    intf_missing: bool,
}

#[derive(Debug)]
struct NodeUsedInfo {
    used_o: HashSet<NodeWireId>,
    used_i: HashSet<NodeWireId>,
}

fn prep_node_used_info(db: &IntDb, nid: NodeKindId) -> NodeUsedInfo {
    let node = &db.nodes[nid];
    let mut used_o = HashSet::new();
    let mut used_i = HashSet::new();
    for (&k, v) in &node.muxes {
        used_o.insert(k);
        for &w in &v.ins {
            if !db.wires[w.1].kind.is_tie() {
                used_i.insert(w);
            }
        }
    }
    for bel in node.bels.values() {
        for pin in bel.pins.values() {
            for &w in &pin.wires {
                match pin.dir {
                    PinDir::Input => {
                        used_i.insert(w);
                    }
                    PinDir::Output => {
                        used_o.insert(w);
                    }
                }
            }
        }
    }
    NodeUsedInfo { used_o, used_i }
}

impl<'a> Verifier<'a> {
    fn new(rd: &'a Part, grid: &'a ExpandedGrid) -> Self {
        let mut node_used = EntityVec::new();
        for nid in grid.db.nodes.ids() {
            node_used.push(prep_node_used_info(grid.db, nid));
        }
        Self {
            rd,
            db: grid.db,
            grid,
            tile_lut: rd.tiles.iter().map(|(&c, t)| (t.name.clone(), c)).collect(),
            dummy_in_nodes: HashSet::new(),
            dummy_out_nodes: HashSet::new(),
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
            node_used,
            int_wire_data: HashMap::new(),
            skip_residual: false,
            stub_outs: HashSet::new(),
            stub_ins: HashSet::new(),
        }
    }

    fn prep_int_wires(&mut self) {
        for die in self.grid.dies() {
            for col in die.cols() {
                for row in die.rows() {
                    let et = &die[(col, row)];
                    for node in &et.nodes {
                        let nui = &self.node_used[node.kind];
                        let nk = &self.db.nodes[node.kind];
                        let nn = &self.db.node_namings[node.naming];
                        for &w in &nui.used_i {
                            let w = (die.die, node.tiles[w.0], w.1);
                            if let Some(w) = self.grid.resolve_wire_raw(w) {
                                self.int_wire_data.entry(w).or_default().used_i = true;
                            }
                        }
                        for &w in &nui.used_o {
                            let w = (die.die, node.tiles[w.0], w.1);
                            if let Some(w) = self.grid.resolve_wire_raw(w) {
                                self.int_wire_data.entry(w).or_default().used_o = true;
                            }
                        }
                        let naming = &self.db.node_namings[node.naming];
                        for nt in node.tiles.ids() {
                            for (wt, wd) in &self.db.wires {
                                if let WireKind::Buf(wf) = wd.kind {
                                    let wt = (nt, wt);
                                    let wf = (nt, wf);
                                    if naming.wires.contains_key(&wt) {
                                        let w = (die.die, node.tiles[wf.0], wf.1);
                                        if let Some(w) = self.grid.resolve_wire_raw(w) {
                                            self.int_wire_data.entry(w).or_default().used_i = true;
                                        }
                                        let w = (die.die, node.tiles[wt.0], wt.1);
                                        if let Some(w) = self.grid.resolve_wire_raw(w) {
                                            self.int_wire_data.entry(w).or_default().used_o = true;
                                        }
                                    }
                                }
                            }
                        }
                        for (w, ii) in &nk.intfs {
                            match ii {
                                IntfInfo::InputDelay => {
                                    let wf = (die.die, node.tiles[w.0], w.1);
                                    if let Some(wf) = self.grid.resolve_wire_raw(wf) {
                                        let iwd = self.int_wire_data.entry(wf).or_default();
                                        iwd.used_i = true;
                                        iwd.has_intf_i = true;
                                    }
                                }
                                IntfInfo::OutputTestMux(ref wfs) => {
                                    let wt = (die.die, node.tiles[w.0], w.1);
                                    if let Some(wt) = self.grid.resolve_wire_raw(wt) {
                                        let iwd = self.int_wire_data.entry(wt).or_default();
                                        iwd.used_o = true;
                                        iwd.has_intf_o = true;
                                    }
                                    for &wf in wfs {
                                        let wf = (die.die, node.tiles[w.0], wf.1);
                                        if let Some(wf) = self.grid.resolve_wire_raw(wf) {
                                            self.int_wire_data.entry(wf).or_default().used_i = true;
                                        }
                                    }
                                }
                            }
                        }
                        for (w, ini) in &nn.intf_wires_in {
                            if let IntfWireInNaming::Buf(_, _) = ini {
                                let wf = (die.die, node.tiles[w.0], w.1);
                                if let Some(wf) = self.grid.resolve_wire_raw(wf) {
                                    let iwd = self.int_wire_data.entry(wf).or_default();
                                    iwd.used_i = true;
                                    iwd.has_intf_i = true;
                                }
                            }
                        }
                    }
                    for t in et.terms.values().flatten() {
                        if let Some(tn) = t.naming {
                            let tn = &self.db.term_namings[tn];
                            let tk = &self.db.terms[t.kind];
                            for w in tn.wires_out.ids() {
                                let wt = (die.die, (col, row), w);
                                if let Some(wt) = self.grid.resolve_wire_raw(wt) {
                                    self.int_wire_data.entry(wt).or_default().used_o = true;
                                }
                                let wf = match tk.wires[w] {
                                    TermInfo::PassNear(wf) => (die.die, (col, row), wf),
                                    TermInfo::PassFar(wf) => (die.die, t.target.unwrap(), wf),
                                    _ => unreachable!(),
                                };
                                if let Some(wf) = self.grid.resolve_wire_raw(wf) {
                                    self.int_wire_data.entry(wf).or_default().used_i = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn claim_raw_node(&mut self, nw: NodeOrWire, crd: rawdump::Coord, wn: &str) {
        match nw {
            NodeOrWire::Node(nidx) => {
                if self.claimed_nodes[nidx] {
                    let tname = &self.rd.tiles[&crd].name;
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
                    let tname = &self.rd.tiles[&crd].name;
                    println!(
                        "DOUBLE CLAIMED NODE {part} {tname} {wn}",
                        part = self.rd.part
                    );
                }
                ctw.set(widx, true);
            }
        }
    }

    pub fn pin_int_wire(&mut self, crd: Coord, wire: &str, iw: IntWire) -> bool {
        if let Some(cnw) = self.rd.lookup_wire(crd, wire) {
            let iwd = self.int_wire_data.get_mut(&iw).unwrap();
            if iwd.used_i && iwd.used_o {
                if iwd.node.is_none() {
                    iwd.node = Some(cnw);
                    self.claim_raw_node(cnw, crd, wire);
                } else if iwd.node != Some(cnw) {
                    let tname = &self.rd.tiles[&crd].name;
                    println!(
                        "INT NODE MISMATCH FOR {p} {tname} {wire} {iw:?} {wn}",
                        p = self.rd.part,
                        wn = self.db.wires[iw.2].name
                    );
                }
            } else if iwd.used_o {
                if !self.dummy_out_nodes.contains(&cnw) {
                    self.dummy_out_nodes.insert(cnw);
                    self.claim_raw_node(cnw, crd, wire);
                }
            } else {
                if !self.dummy_in_nodes.contains(&cnw) {
                    self.dummy_in_nodes.insert(cnw);
                    self.claim_raw_node(cnw, crd, wire);
                }
            }
            true
        } else {
            false
        }
    }

    pub fn pin_int_intf_wire(&mut self, crd: Coord, wire: &str, iw: IntWire) -> bool {
        if let Some(cnw) = self.rd.lookup_wire(crd, wire) {
            let iwd = self.int_wire_data.get_mut(&iw).unwrap();
            if let Some(nw) = iwd.intf_node {
                if nw != cnw {
                    let tname = &self.rd.tiles[&crd].name;
                    println!(
                        "INT INTF NODE MISMATCH FOR {p} {tname} {wire} {iw:?} {wn}",
                        p = self.rd.part,
                        wn = self.db.wires[iw.2].name
                    );
                }
            } else if iwd.intf_missing {
                let tname = &self.rd.tiles[&crd].name;
                println!("INT INTF NODE PRESENT FOR {tname} {wire} BUT WAS MISSING PREVIOUSLY");
                iwd.intf_node = Some(cnw);
                self.claim_node(&[(crd, wire)]);
            } else {
                iwd.intf_node = Some(cnw);
                self.claim_node(&[(crd, wire)]);
            }
            true
        } else {
            let iwd = self.int_wire_data.get_mut(&iw).unwrap();
            if iwd.intf_node.is_some() {
                let tname = &self.rd.tiles[&crd].name;
                println!("INT INTF NODE PRESENT FOR {tname} {wire} BUT WIRE NOT FOUND");
            } else if iwd.intf_missing {
                let tname = &self.rd.tiles[&crd].name;
                println!("INT INTF WIRE {tname} {wire} MISSING TWICE");
            } else {
                iwd.intf_missing = true;
            }
            false
        }
    }

    pub fn verify_node(&mut self, tiles: &[(Coord, &str)]) {
        let mut nw = None;
        for &(crd, wn) in tiles {
            let tile = &self.rd.tiles[&crd];
            let tname = &tile.name;
            if let Some(cnw) = self.rd.lookup_wire(crd, wn) {
                if let Some(pnw) = nw {
                    if pnw != cnw {
                        println!("NODE MISMATCH FOR {p} {tname} {wn}", p = self.rd.part);
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
            if let Some(cnw) = self.rd.lookup_wire(crd, wn) {
                if let Some(pnw) = nw {
                    if pnw != cnw {
                        println!("NODE MISMATCH FOR {p} {tname} {wn}", p = self.rd.part);
                    }
                } else {
                    nw = Some(cnw);
                    self.claim_raw_node(cnw, crd, wn);
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

    fn print_nw(&self, nw: NodeWireId) -> String {
        format!("{t}.{w}", t = nw.0.to_idx(), w = self.db.wires[nw.1].name)
    }

    fn print_w(&self, w: WireId) -> String {
        self.db.wires[w].name.to_string()
    }

    fn handle_int_node(&mut self, die: DieId, node: &ExpandedTileNode) {
        let crds;
        if let Some(c) = self.get_node_crds(node) {
            crds = c;
        } else {
            return;
        }
        let def_rt = NodeRawTileId::from_idx(0);
        let kind = &self.db.nodes[node.kind];
        let naming = &self.db.node_namings[node.naming];
        let nui = &self.node_used[node.kind];
        let mut wire_lut = HashMap::new();
        for &w in nui.used_i.iter().chain(nui.used_o.iter()) {
            let ww = (die, node.tiles[w.0], w.1);
            wire_lut.insert(w, self.grid.resolve_wire_raw(ww));
        }
        let mut wires_pinned = HashSet::new();
        let mut wires_missing = HashSet::new();
        for (&wt, wfs) in &kind.muxes {
            let wti = &wire_lut[&wt];
            if wti.is_none() {
                continue;
            }
            let wti = wti.unwrap();
            for &wf in &wfs.ins {
                let wftie = self.db.wires[wf.1].kind.is_tie();
                let pip_found;
                if let Some(en) = naming.ext_pips.get(&(wt, wf)) {
                    if !crds.contains_id(en.tile) {
                        pip_found = false;
                    } else {
                        let wfi = &wire_lut[&wf];
                        if wfi.is_none() {
                            continue;
                        }
                        let wfi = wfi.unwrap();
                        let wtf = self.pin_int_wire(crds[en.tile], &en.wire_to, wti);
                        let wff = self.pin_int_wire(crds[en.tile], &en.wire_from, wfi);
                        pip_found = wtf && wff;
                        if pip_found {
                            self.claim_pip(crds[en.tile], &en.wire_to, &en.wire_from);
                        }
                    }
                } else {
                    let wtf;
                    if wires_pinned.contains(&wt) {
                        wtf = true;
                    } else if wires_missing.contains(&wt) {
                        wtf = false;
                    } else if let Some(n) = naming.wires.get(&wt) {
                        wtf = self.pin_int_wire(crds[def_rt], n, wti);
                        if wtf {
                            wires_pinned.insert(wt);
                        } else {
                            wires_missing.insert(wt);
                        }
                    } else {
                        wtf = false;
                        wires_missing.insert(wt);
                    }
                    let wff;
                    if wires_pinned.contains(&wf) {
                        wff = true;
                    } else if wires_missing.contains(&wf) {
                        wff = false;
                    } else if wftie {
                        self.claim_node(&[(crds[def_rt], &naming.wires[&wf])]);
                        wires_pinned.insert(wf);
                        wff = true;
                    } else if let Some(n) = naming.wires.get(&wf) {
                        let wfi = &wire_lut[&wf];
                        if wfi.is_none() {
                            continue;
                        }
                        let wfi = wfi.unwrap();
                        if let Some(pip) = naming.wire_bufs.get(&wf) {
                            wff = self.pin_int_wire(crds[pip.tile], &pip.wire_from, wfi);
                            if wff {
                                self.claim_pip(crds[pip.tile], &pip.wire_to, &pip.wire_from);
                                self.claim_node(&[
                                    (crds[pip.tile], &pip.wire_to),
                                    (crds[def_rt], &naming.wires[&wf]),
                                ]);
                            }
                        } else {
                            wff = self.pin_int_wire(crds[def_rt], n, wfi);
                        }
                        if wff {
                            wires_pinned.insert(wf);
                        } else {
                            wires_missing.insert(wf);
                        }
                    } else {
                        wff = false;
                        wires_missing.insert(wf);
                    }

                    pip_found = wtf && wff;
                    if pip_found {
                        self.claim_pip(crds[def_rt], &naming.wires[&wt], &naming.wires[&wf]);
                    }
                }
                if !pip_found {
                    let wtu = self.int_wire_data[&wti].used_i;
                    let wfu = if wftie {
                        true
                    } else {
                        let wfi = &wire_lut[&wf];
                        let wfi = wfi.unwrap();
                        self.int_wire_data[&wfi].used_o
                    };
                    if wtu && wfu {
                        println!(
                            "MISSING PIP {part} {tile} {wt} {wf}",
                            part = self.rd.part,
                            tile = node.names[def_rt],
                            wt = self.print_nw(wt),
                            wf = self.print_nw(wf)
                        );
                    }
                }
            }
        }
        for (&wt, wtn) in &naming.wires {
            let wtk = &self.db.wires[wt.1].kind;
            if let &WireKind::Buf(wfw) = wtk {
                let wf = (wt.0, wfw);
                let wti = self
                    .grid
                    .resolve_wire_raw((die, node.tiles[wt.0], wt.1))
                    .unwrap();
                let wfi = self
                    .grid
                    .resolve_wire_raw((die, node.tiles[wf.0], wf.1))
                    .unwrap();
                let wff = self.pin_int_wire(crds[def_rt], &naming.wires[&wf], wfi);
                let wtf = self.pin_int_wire(crds[def_rt], wtn, wti);
                if wff && wtf {
                    self.claim_pip(crds[def_rt], wtn, &naming.wires[&wf]);
                } else {
                    let wtu = self.int_wire_data[&wti].used_i;
                    let wfu = self.int_wire_data[&wfi].used_o;
                    if wtu && wfu {
                        println!(
                            "MISSING BUF PIP {part} {tile} {wt} {wf}",
                            part = self.rd.part,
                            tile = node.names[def_rt],
                            wf = self.print_nw(wf),
                            wt = self.print_nw(wt)
                        );
                    }
                }
            }
        }
        if let Some(ref tn) = node.tie_name {
            let mut pins = vec![];
            for (&k, v) in &naming.wires {
                let wi = &self.db.wires[k.1];
                let pin = match wi.kind {
                    WireKind::Tie0 => self.grid.tie_pin_gnd.as_ref().unwrap(),
                    WireKind::Tie1 => self.grid.tie_pin_vcc.as_ref().unwrap(),
                    WireKind::TiePullup => self.grid.tie_pin_pullup.as_ref().unwrap(),
                    _ => continue,
                };
                if !wires_pinned.contains(&k) {
                    self.claim_node(&[(crds[def_rt], v)]);
                }
                pins.push(SitePin {
                    dir: SitePinDir::Out,
                    pin,
                    wire: Some(v),
                });
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
                        if !self.pin_int_intf_wire(wcrd, ww, wire) {
                            println!(
                                "MISSING BEL PIN INTF WIRE {part} {tile} {k} {wire}",
                                part = self.rd.part,
                                tile = node.names[def_rt],
                                wire = n.name_far
                            );
                        }
                    } else {
                        if !self.pin_int_wire(wcrd, ww, wire) {
                            let iwd = &self.int_wire_data[&wire];
                            if (v.dir == PinDir::Input && iwd.used_o)
                                || (v.dir == PinDir::Output && iwd.used_i)
                            {
                                println!(
                                    "MISSING BEL PIN INT WIRE {part} {tile} {k} {wire}",
                                    part = self.rd.part,
                                    tile = node.names[def_rt],
                                    wire = n.name_far
                                );
                            }
                        }
                    }
                }
                if claim {
                    self.claim_node(&[(crd, wn)]);
                }
            }
        }

        for (&wt, ii) in &kind.intfs {
            let wti = (die, node.tiles[wt.0], wt.1);
            match ii {
                IntfInfo::InputDelay => {
                    if let IntfWireInNaming::Delay(ref wton, ref wtdn, ref wtn) =
                        naming.intf_wires_in[&wt]
                    {
                        if !self.pin_int_wire(crds[def_rt], wtn, wti) {
                            let tname = &node.names[def_rt];
                            println!(
                                "INT NODE MISSING FOR {p} {tname} {wton} {wn}",
                                p = self.rd.part,
                                wn = self.print_nw(wt),
                            );
                        }
                        self.pin_int_intf_wire(crds[def_rt], wton, wti);
                        self.claim_node(&[(crds[def_rt], wtdn)]);
                        self.claim_pip(crds[def_rt], wtdn, wtn);
                        self.claim_pip(crds[def_rt], wton, wtn);
                        self.claim_pip(crds[def_rt], wton, wtdn);
                    } else {
                        unreachable!()
                    }
                }
                IntfInfo::OutputTestMux(wfs) => {
                    let wtn = match naming.intf_wires_out[&wt] {
                        IntfWireOutNaming::Simple(ref wtn) => wtn,
                        IntfWireOutNaming::Buf(ref wtn, ref wsn) => {
                            if self.pin_int_intf_wire(crds[def_rt], wsn, wti) {
                                self.claim_pip(crds[def_rt], wtn, wsn);
                            }
                            wtn
                        }
                    };
                    if !self.pin_int_wire(crds[def_rt], wtn, wti) {
                        let tname = &node.names[def_rt];
                        println!(
                            "INT NODE MISSING FOR {p} {tname} {wtn} {wn}",
                            p = self.rd.part,
                            wn = self.print_nw(wt),
                        );
                    }
                    for &wf in wfs {
                        let wfi = (die, node.tiles[wf.0], wf.1);
                        let wfn = match naming.intf_wires_in[&wf] {
                            IntfWireInNaming::Simple(ref wfn) => {
                                self.claim_pip(crds[def_rt], wtn, wfn);
                                wfn
                            }
                            IntfWireInNaming::TestBuf(ref wfbn, ref wfn) => {
                                self.claim_pip(crds[def_rt], wtn, wfbn);
                                wfn
                            }
                            IntfWireInNaming::Buf(_, ref wfn) => {
                                self.claim_pip(crds[def_rt], wtn, wfn);
                                wfn
                            }
                            IntfWireInNaming::Delay(ref wfon, _, ref wfn) => {
                                self.claim_pip(crds[def_rt], wtn, wfon);
                                wfn
                            }
                        };
                        if !self.pin_int_wire(crds[def_rt], wfn, wfi) {
                            let tname = &node.names[def_rt];
                            println!(
                                "INT NODE MISSING FOR {p} {tname} {wfn} {wn}",
                                p = self.rd.part,
                                wn = self.print_nw(wf),
                            );
                        }
                    }
                }
            }
        }
        for (wf, iwin) in &naming.intf_wires_in {
            if let &IntfWireInNaming::TestBuf(ref wfbn, ref wfn) = iwin {
                self.claim_node(&[(crds[def_rt], wfbn)]);
                self.claim_pip(crds[def_rt], wfbn, wfn);
            }
            if let &IntfWireInNaming::Buf(ref wfbn, ref wfn) = iwin {
                if self.pin_int_intf_wire(crds[def_rt], wfbn, (die, node.tiles[wf.0], wf.1)) {
                    self.claim_pip(crds[def_rt], wfbn, wfn);
                }
            }
        }
    }

    pub fn handle_int_term(&mut self, die: DieId, col: ColId, row: RowId, term: &ExpandedTileTerm) {
        if let Some(tn) = term.naming {
            let tn = &self.db.term_namings[tn];
            let tk = &self.db.terms[term.kind];
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
            for (w, twn) in &tn.wires_out {
                let wt = self.grid.resolve_wire_raw((die, (col, row), w));
                if wt.is_none() {
                    continue;
                }
                let wt = wt.unwrap();
                let tkw = &tk.wires[w];
                let wf = match *tkw {
                    TermInfo::PassNear(wf) => (die, (col, row), wf),
                    TermInfo::PassFar(wf) => (die, term.target.unwrap(), wf),
                    _ => unreachable!(),
                };
                let wf = self.grid.resolve_wire_raw(wf);
                if wf.is_none() {
                    continue;
                }
                let wf = wf.unwrap();
                let pip_found;
                match *twn {
                    TermWireOutNaming::Simple(ref wtn) => {
                        let wtf = self.pin_int_wire(crd, wtn, wt);
                        match *tkw {
                            TermInfo::PassNear(wfw) => {
                                let wfn = &tn.wires_in_near[wfw];
                                let wff = self.pin_int_wire(crd, wfn, wf);
                                pip_found = wtf && wff;
                                if pip_found {
                                    self.claim_pip(crd, wtn, wfn);
                                }
                            }
                            TermInfo::PassFar(wfw) => match tn.wires_in_far[wfw] {
                                TermWireInFarNaming::Simple(ref wfn) => {
                                    let wff = self.pin_int_wire(crd, wfn, wf);
                                    pip_found = wtf && wff;
                                    if pip_found {
                                        self.claim_pip(crd, wtn, wfn);
                                    }
                                }
                                TermWireInFarNaming::Buf(ref wfn, ref wfin) => {
                                    let wff = self.pin_int_wire(crd, wfin, wf);
                                    pip_found = wtf && wff;
                                    if pip_found {
                                        self.claim_node(&[(crd, wfn)]);
                                        self.claim_pip(crd, wtn, wfn);
                                        self.claim_pip(crd, wfn, wfin);
                                    }
                                }
                                TermWireInFarNaming::BufFar(ref wfn, ref wffon, ref wffin) => {
                                    let wff = self.pin_int_wire(crd_far.unwrap(), wffin, wf);
                                    pip_found = wtf && wff;
                                    if pip_found {
                                        self.claim_node(&[(crd, wfn), (crd_far.unwrap(), wffon)]);
                                        self.claim_pip(crd_far.unwrap(), wffon, wffin);
                                        self.claim_pip(crd, wtn, wfn);
                                    }
                                }
                            },
                            _ => unreachable!(),
                        }
                    }
                    TermWireOutNaming::Buf(ref wtn, ref wfn) => {
                        let wtf = self.pin_int_wire(crd, wtn, wt);
                        let wff = self.pin_int_wire(crd, wfn, wf);
                        pip_found = wtf && wff;
                        if pip_found {
                            self.claim_pip(crd, wtn, wfn);
                        }
                    }
                }
                if !pip_found {
                    let wtu = self.int_wire_data[&wt].used_i;
                    let wfu = self.int_wire_data[&wf].used_o;
                    if wtu && wfu {
                        println!(
                            "MISSING TERM PIP {part} {tile} {wt}",
                            part = self.rd.part,
                            tile = term.tile.as_ref().unwrap(),
                            wt = self.print_w(w)
                        );
                    }
                }
            }
        }
    }

    pub fn handle_int(&mut self) {
        for die in self.grid.dies() {
            for col in die.cols() {
                for row in die.rows() {
                    let et = &die[(col, row)];
                    for node in &et.nodes {
                        self.handle_int_node(die.die, node);
                    }
                    for t in et.terms.values().flatten() {
                        self.handle_int_term(die.die, col, row, t);
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

    pub fn skip_residual(&mut self) {
        self.skip_residual = true;
    }

    pub fn kill_stub_out(&mut self, name: &str) {
        if let Some(wi) = self.rd.wires.get(name) {
            self.stub_outs.insert(wi);
        }
    }

    pub fn kill_stub_in(&mut self, name: &str) {
        if let Some(wi) = self.rd.wires.get(name) {
            self.stub_ins.insert(wi);
        }
    }

    fn finish(mut self) {
        for (&crd, tile) in &self.rd.tiles {
            let tk = &self.rd.tile_kinds[tile.kind];
            for &(wf, wt) in tk.pips.keys() {
                let pip_present = self.rd.lookup_wire(crd, &self.rd.wires[wf]).is_some()
                    && self.rd.lookup_wire(crd, &self.rd.wires[wt]).is_some();
                if pip_present && (self.stub_outs.contains(&wt) || self.stub_ins.contains(&wf)) {
                    self.claim_pip(crd, &self.rd.wires[wt], &self.rd.wires[wf]);
                }
            }
            for &w in tk.wires.keys() {
                if self.stub_outs.contains(&w) || self.stub_ins.contains(&w) {
                    self.claim_node(&[(crd, &self.rd.wires[w])]);
                }
            }
        }

        if self.skip_residual {
            return;
        }
        for (&crd, tile) in &self.rd.tiles {
            let tk = &self.rd.tile_kinds[tile.kind];
            let claimed_sites = &self.claimed_sites[&crd];
            for (i, site) in &tile.sites {
                if !claimed_sites[i] {
                    println!(
                        "UNCLAIMED SITE {part} {tile} {site}",
                        part = self.rd.part,
                        tile = tile.name
                    );
                }
            }
            let claimed_pips = &self.claimed_pips[&crd];
            for (i, &(wf, wt), _) in &tk.pips {
                let pip_present = self.rd.lookup_wire(crd, &self.rd.wires[wf]).is_some()
                    && self.rd.lookup_wire(crd, &self.rd.wires[wt]).is_some();
                if !claimed_pips[i] && pip_present {
                    println!(
                        "UNCLAIMED PIP {part} {tile} {wt} <- {wf}",
                        part = self.rd.part,
                        tile = tile.name,
                        wt = self.rd.wires[wt],
                        wf = self.rd.wires[wf]
                    );
                }
            }
            let claimed_twires = &self.claimed_twires[&crd];
            for (i, &w, &wi) in &tk.wires {
                match wi {
                    rawdump::TkWire::Internal(_, _) => {
                        if !claimed_twires[i] {
                            println!(
                                "UNCLAIMED INTERNAL WIRE {part} {tile} {wire}",
                                part = self.rd.part,
                                tile = tile.name,
                                wire = self.rd.wires[w]
                            );
                        }
                    }
                    rawdump::TkWire::Connected(ci) => {
                        if let Some(&node) = tile.conn_wires.get(ci) {
                            if !self.claimed_nodes[node] {
                                println!(
                                    "UNCLAIMED CONN WIRE {part} {tile} {wire} {node}",
                                    part = self.rd.part,
                                    tile = tile.name,
                                    wire = self.rd.wires[w],
                                    node = node.to_idx()
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn verify(
    rd: &rawdump::Part,
    grid: &ExpandedGrid,
    bel_handler: impl Fn(&mut Verifier, &BelContext<'_>),
    extra: impl FnOnce(&mut Verifier),
) {
    let mut vrf = Verifier::new(rd, grid);
    vrf.prep_int_wires();
    vrf.handle_int();
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
    extra(&mut vrf);
    vrf.finish();
}
