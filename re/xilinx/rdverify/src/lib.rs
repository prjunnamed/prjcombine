#![allow(clippy::unnecessary_unwrap)]

use prjcombine_entity::{EntityBitVec, EntityId, EntityPartVec, EntityVec};
use prjcombine_interconnect::db::{
    BelInfo, BelSlotId, ConnectorWire, IntDb, LegacyBel, PinDir, SwitchBoxItem, TileClassId,
    TileWireCoord, WireKind, WireSlotId,
};
use prjcombine_interconnect::grid::{
    BelCoord, CellCoord, ColId, ConnectorCoord, DieId, ExpandedGrid, RowId, Tile, TileCoord,
    WireCoord,
};
use prjcombine_re_xilinx_naming::db::{
    BelNaming, ConnectorWireInFarNaming, ConnectorWireOutNaming, IntfWireInNaming, NamingDb,
    ProperBelNaming, RawTileId,
};
use prjcombine_re_xilinx_naming::grid::{ExpandedGridNaming, TileNaming};
use prjcombine_re_xilinx_rawdump::{self as rawdump, Coord, NodeOrWire, Part};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct BelContext<'a> {
    pub die: DieId,
    pub col: ColId,
    pub row: RowId,
    pub slot: BelSlotId,
    pub cell: CellCoord,
    pub bel: BelCoord,
    pub tile: &'a Tile,
    pub ntile: &'a TileNaming,
    pub tcls: &'a str,
    pub info: &'a LegacyBel,
    pub naming: &'a ProperBelNaming,
    pub name: Option<&'a str>,
    pub crds: EntityPartVec<RawTileId, Coord>,
}

impl<'a> BelContext<'a> {
    pub fn crd(&self) -> Coord {
        self.crds[self.naming.tile]
    }

    #[track_caller]
    pub fn wire(&self, name: &str) -> &'a str {
        &self.naming.pins[name].name
    }

    #[track_caller]
    pub fn wire_far(&self, name: &str) -> &'a str {
        &self.naming.pins[name].name_far
    }

    #[track_caller]
    pub fn fwire(&self, name: &str) -> (Coord, &'a str) {
        (self.crd(), self.wire(name))
    }

    #[track_caller]
    pub fn fwire_far(&self, name: &str) -> (Coord, &'a str) {
        (self.crd(), self.wire_far(name))
    }

    #[track_caller]
    pub fn pip(&self, pin: &str, idx: usize) -> (Coord, &'a str, &'a str) {
        let pip = &self.naming.pins[pin].pips[idx];
        (self.crds[pip.tile], &pip.wire_to, &pip.wire_from)
    }

    #[track_caller]
    pub fn pip_owire(&self, pin: &str, idx: usize) -> (Coord, &'a str) {
        let (crd, wire, _) = self.pip(pin, idx);
        (crd, wire)
    }

    #[track_caller]
    pub fn pip_iwire(&self, pin: &str, idx: usize) -> (Coord, &'a str) {
        let (crd, _, wire) = self.pip(pin, idx);
        (crd, wire)
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
    Inout,
}

pub struct Verifier<'a> {
    pub rd: &'a Part,
    pub db: &'a IntDb,
    pub ndb: &'a NamingDb,
    pub grid: &'a ExpandedGrid<'a>,
    pub ngrid: &'a ExpandedGridNaming<'a>,
    pub tile_lut: HashMap<String, Coord>,
    intf_int_aliases: HashMap<WireCoord, WireCoord>,
    dummy_in_nodes: HashSet<NodeOrWire>,
    dummy_out_nodes: HashSet<NodeOrWire>,
    claimed_nodes: EntityBitVec<rawdump::NodeId>,
    claimed_twires: HashMap<Coord, EntityBitVec<rawdump::TkWireId>>,
    claimed_pips: HashMap<Coord, EntityBitVec<rawdump::TkPipId>>,
    claimed_sites: HashMap<Coord, EntityBitVec<rawdump::TkSiteId>>,
    vcc_nodes: HashSet<NodeOrWire>,
    int_wire_data: HashMap<WireCoord, IntWireData>,
    node_used: EntityVec<TileClassId, TileClassUsedInfo>,
    skip_residual_sites: bool,
    skip_residual_pips: bool,
    skip_residual_nodes: bool,
    stub_outs: HashSet<rawdump::WireId>,
    stub_ins: HashSet<rawdump::WireId>,
    cond_stub_outs: HashSet<rawdump::WireId>,
    cond_stub_ins: HashSet<rawdump::WireId>,
    cond_stub_ins_tk: HashSet<(rawdump::TileKindId, rawdump::WireId)>,
    skip_bel_pins: HashSet<(BelCoord, &'static str)>,
}

#[derive(Debug, Default)]
struct IntWireData {
    used_o: bool,
    used_i: bool,
    node: Option<NodeOrWire>,
    has_intf_i: bool,
    intf_node: Option<NodeOrWire>,
    intf_missing: bool,
}

#[derive(Debug)]
struct TileClassUsedInfo {
    used_o: HashSet<TileWireCoord>,
    used_i: HashSet<TileWireCoord>,
}

fn prep_tile_class_used_info(db: &IntDb, tcid: TileClassId) -> TileClassUsedInfo {
    let tcls = &db[tcid];
    let mut used_o = HashSet::new();
    let mut used_i = HashSet::new();
    for bel in tcls.bels.values() {
        match bel {
            BelInfo::SwitchBox(sb) => {
                for item in &sb.items {
                    match item {
                        SwitchBoxItem::Mux(mux) => {
                            used_o.insert(mux.dst);
                            for &w in mux.src.keys() {
                                if !db[w.wire].is_tie() {
                                    used_i.insert(w.tw);
                                }
                            }
                        }
                        SwitchBoxItem::ProgBuf(buf) => {
                            used_o.insert(buf.dst);
                            if !db[buf.src.wire].is_tie() {
                                used_i.insert(buf.src.tw);
                            }
                        }
                        SwitchBoxItem::PermaBuf(buf) => {
                            used_o.insert(buf.dst);
                            used_i.insert(buf.src.tw);
                        }
                        SwitchBoxItem::Pass(pass) => {
                            used_o.insert(pass.dst);
                            used_i.insert(pass.src);
                        }
                        SwitchBoxItem::BiPass(pass) => {
                            used_o.insert(pass.a);
                            used_o.insert(pass.b);
                            used_i.insert(pass.a);
                            used_i.insert(pass.b);
                        }
                        SwitchBoxItem::ProgInv(inv) => {
                            used_o.insert(inv.dst);
                            used_i.insert(inv.src);
                        }
                        SwitchBoxItem::ProgDelay(delay) => {
                            used_o.insert(delay.dst);
                            used_i.insert(delay.src.tw);
                        }
                        SwitchBoxItem::Bidi(_) => unreachable!(),
                    }
                }
            }
            BelInfo::Bel(_bel) => {
                todo!();
            }
            BelInfo::Legacy(bel) => {
                for pin in bel.pins.values() {
                    for &w in &pin.wires {
                        match pin.dir {
                            PinDir::Input => {
                                used_i.insert(w);
                            }
                            PinDir::Output => {
                                used_o.insert(w);
                            }
                            PinDir::Inout => {
                                used_i.insert(w);
                                used_o.insert(w);
                            }
                        }
                    }
                }
            }
            BelInfo::TestMux(tm) => {
                for (&dst, tmux) in &tm.wires {
                    used_o.insert(dst);
                    used_i.insert(tmux.primary_src.tw);
                    for &src in tmux.test_src.keys() {
                        used_i.insert(src.tw);
                    }
                }
            }
            BelInfo::GroupTestMux(tm) => {
                for (&dst, tmux) in &tm.wires {
                    used_o.insert(dst);
                    used_i.insert(tmux.primary_src.tw);
                    for &src in &tmux.test_src {
                        if let Some(src) = src {
                            used_i.insert(src.tw);
                        }
                    }
                }
            }
        }
    }
    TileClassUsedInfo { used_o, used_i }
}

impl<'a> Verifier<'a> {
    fn new(rd: &'a Part, ngrid: &'a ExpandedGridNaming) -> Self {
        let mut node_used = EntityVec::new();
        for nid in ngrid.egrid.db.tile_classes.ids() {
            node_used.push(prep_tile_class_used_info(ngrid.egrid.db, nid));
        }
        Self {
            rd,
            db: ngrid.egrid.db,
            ndb: ngrid.db,
            grid: ngrid.egrid,
            ngrid,
            tile_lut: rd.tiles.iter().map(|(&c, t)| (t.name.clone(), c)).collect(),
            intf_int_aliases: HashMap::new(),
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
            vcc_nodes: HashSet::new(),
            node_used,
            int_wire_data: HashMap::new(),
            skip_residual_sites: false,
            skip_residual_pips: false,
            skip_residual_nodes: false,
            stub_outs: HashSet::new(),
            stub_ins: HashSet::new(),
            cond_stub_outs: HashSet::new(),
            cond_stub_ins: HashSet::new(),
            cond_stub_ins_tk: HashSet::new(),
            skip_bel_pins: HashSet::new(),
        }
    }

    pub fn alias_intf_int(&mut self, from: WireCoord, to: WireCoord) {
        self.intf_int_aliases.insert(from, to);
    }

    fn prep_int_wires(&mut self) {
        for (tcrd, tile) in self.grid.tiles() {
            let nui = &self.node_used[tile.class];
            let Some(ntile) = self.ngrid.tiles.get(&tcrd) else {
                continue;
            };
            let naming = &self.ndb.tile_class_namings[ntile.naming];
            for &w in &nui.used_i {
                let w = self.grid.tile_wire(tcrd, w);
                if let Some(w) = self.ngrid.resolve_wire_raw(w) {
                    self.int_wire_data.entry(w).or_default().used_i = true;
                }
            }
            for &w in &nui.used_o {
                let w = self.grid.tile_wire(tcrd, w);
                if let Some(w) = self.ngrid.resolve_wire_raw(w) {
                    self.int_wire_data.entry(w).or_default().used_o = true;
                }
            }
            for (&w, ini) in &naming.intf_wires_in {
                if let IntfWireInNaming::Buf { .. } = ini {
                    let wf = self.grid.tile_wire(tcrd, w);
                    if let Some(wf) = self.ngrid.resolve_wire_raw(wf) {
                        let iwd = self.int_wire_data.entry(wf).or_default();
                        iwd.used_i = true;
                        iwd.has_intf_i = true;
                    }
                }
            }
        }
        for (ccrd, conn) in self.grid.connectors() {
            if let Some(nt) = self.ngrid.conns.get(&ccrd) {
                let tn = &self.ndb.conn_class_namings[nt.naming];
                let ccls = &self.db[conn.class];
                for w in tn.wires_out.ids() {
                    let wt = ccrd.cell.wire(w);
                    if let Some(wt) = self.ngrid.resolve_wire_raw(wt) {
                        self.int_wire_data.entry(wt).or_default().used_o = true;
                    }
                    let wf = match ccls.wires[w] {
                        ConnectorWire::Reflect(wf) => ccrd.cell.wire(wf),
                        ConnectorWire::Pass(wf) => conn.target.unwrap().wire(wf),
                        _ => unreachable!(),
                    };
                    if let Some(wf) = self.ngrid.resolve_wire_raw(wf) {
                        self.int_wire_data.entry(wf).or_default().used_i = true;
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

    pub fn pin_int_wire(&mut self, crd: Coord, wire: &str, iw: WireCoord) -> bool {
        if let Some(cnw) = self.rd.lookup_wire(crd, wire) {
            let iwd = self.int_wire_data.get_mut(&iw).unwrap();
            if iwd.used_i && iwd.used_o {
                if iwd.node.is_none() {
                    iwd.node = Some(cnw);
                    self.claim_raw_node(cnw, crd, wire);
                } else if iwd.node != Some(cnw) {
                    let tname = &self.rd.tiles[&crd].name;
                    println!(
                        "INT NODE MISMATCH FOR {p} {tname} {wire} {iw}",
                        p = self.rd.part,
                        iw = iw.to_string(self.db),
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

    pub fn claim_dummy_in(&mut self, wire: (Coord, &str)) {
        let (crd, wn) = wire;
        if let Some(cnw) = self.rd.lookup_wire(crd, wn)
            && !self.dummy_in_nodes.contains(&cnw)
        {
            self.dummy_in_nodes.insert(cnw);
            self.claim_raw_node(cnw, crd, wn);
        }
    }

    pub fn claim_dummy_out(&mut self, wire: (Coord, &str)) {
        let (crd, wn) = wire;
        if let Some(cnw) = self.rd.lookup_wire(crd, wn)
            && !self.dummy_out_nodes.contains(&cnw)
        {
            self.dummy_out_nodes.insert(cnw);
            self.claim_raw_node(cnw, crd, wn);
        }
    }

    pub fn pin_int_intf_wire(&mut self, crd: Coord, wire: &str, iw: WireCoord) -> bool {
        if let Some(&iw) = self.intf_int_aliases.get(&iw) {
            return self.pin_int_wire(crd, wire, iw);
        }
        if let Some(cnw) = self.rd.lookup_wire(crd, wire) {
            let iwd = self.int_wire_data.get_mut(&iw).unwrap();
            if let Some(nw) = iwd.intf_node {
                if nw != cnw {
                    let tname = &self.rd.tiles[&crd].name;
                    println!(
                        "INT INTF NODE MISMATCH FOR {p} {tname} {wire} {iw}",
                        p = self.rd.part,
                        iw = iw.to_string(self.db)
                    );
                }
            } else if iwd.intf_missing {
                let tname = &self.rd.tiles[&crd].name;
                println!("INT INTF NODE PRESENT FOR {tname} {wire} BUT WAS MISSING PREVIOUSLY");
                iwd.intf_node = Some(cnw);
                self.claim_net(&[(crd, wire)]);
            } else {
                iwd.intf_node = Some(cnw);
                self.claim_net(&[(crd, wire)]);
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

    pub fn verify_net(&mut self, tiles: &[(Coord, &str)]) {
        let mut nw = None;
        for &(crd, wn) in tiles {
            let tile = &self.rd.tiles[&crd];
            let tname = &tile.name;
            if let Some(cnw) = self.rd.lookup_wire(crd, wn) {
                if let Some((pnw, pcrd, pwn)) = nw {
                    if pnw != cnw {
                        let ptile = &self.rd.tiles[&pcrd];
                        let ptname = &ptile.name;
                        println!(
                            "NODE MISMATCH FOR {p} {tname} {wn} != {ptname} {pwn}",
                            p = self.rd.part
                        );
                    }
                } else {
                    nw = Some((cnw, crd, wn));
                }
            } else {
                println!("MISSING WIRE {part} {tname} {wn}", part = self.rd.part);
            }
        }
    }

    pub fn is_claimed_raw(&mut self, crd: Coord, wire: rawdump::WireId) -> bool {
        let tile = &self.rd.tiles[&crd];
        if let Some(nw) = self.rd.lookup_wire_raw(crd, wire) {
            match nw {
                NodeOrWire::Node(nidx) => self.claimed_nodes[nidx],
                NodeOrWire::Wire(crd, widx) => self.claimed_twires.get_mut(&crd).unwrap()[widx],
            }
        } else {
            let tname = &tile.name;
            let wn = &self.rd.wires[wire];
            println!("MISSING NODE WIRE {part} {tname} {wn}", part = self.rd.part);
            false
        }
    }

    pub fn claim_net(&mut self, tiles: &[(Coord, &str)]) {
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
                println!("MISSING NODE WIRE {part} {tname} {wn}", part = self.rd.part);
            }
        }
    }

    pub fn claim_vcc_node(&mut self, node: (Coord, &str)) {
        let (crd, wn) = node;
        let tile = &self.rd.tiles[&crd];
        let tname = &tile.name;
        if let Some(cnw) = self.rd.lookup_wire(crd, wn) {
            if self.vcc_nodes.insert(cnw) {
                self.claim_raw_node(cnw, crd, wn);
            }
        } else {
            println!(
                "MISSING VCC NODE WIRE {part} {tname} {wn}",
                part = self.rd.part
            );
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
                        if act_wire.is_some() && pin.wire.is_none() {
                            self.claim_net(&[(crd, act_wire.unwrap())]);
                        } else if pin.wire != act_wire {
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
        tcrd: TileCoord,
    ) -> Option<EntityPartVec<RawTileId, rawdump::Coord>> {
        let mut crds = EntityPartVec::new();
        if let Some(nnode) = self.ngrid.tiles.get(&tcrd) {
            for (k, name) in &nnode.names {
                if let Some(c) = self.xlat_tile(name) {
                    crds.insert(k, c);
                } else {
                    println!("MISSING INT TILE {} {}", self.rd.part, name);
                    return None;
                }
            }
        }
        Some(crds)
    }

    fn print_nw(&self, nw: TileWireCoord) -> String {
        format!(
            "{t}.{w}",
            t = nw.cell.to_idx(),
            w = self.db.wires.key(nw.wire)
        )
    }

    fn print_w(&self, w: WireSlotId) -> String {
        self.db.wires.key(w).to_string()
    }

    fn handle_tile_tmux(&mut self, tcrd: TileCoord) {
        let tile = &self.grid[tcrd];
        let crds;
        if let Some(c) = self.get_node_crds(tcrd) {
            crds = c;
        } else {
            return;
        }
        let Some(ntile) = self.ngrid.tiles.get(&tcrd) else {
            return;
        };
        let def_rt = RawTileId::from_idx(0);
        let tcls = &self.db[tile.class];
        let naming = &self.ndb.tile_class_namings[ntile.naming];
        let nui = &self.node_used[tile.class];
        let mut wire_lut = HashMap::new();
        for &w in nui.used_i.iter().chain(nui.used_o.iter()) {
            let ww = self.grid.tile_wire(tcrd, w);
            wire_lut.insert(w, self.ngrid.resolve_wire_raw(ww));
        }
        for bel in tcls.bels.values() {
            match bel {
                BelInfo::TestMux(tm) => {
                    for (&wt, tmux) in &tm.wires {
                        let wti = self.grid.tile_wire(tcrd, wt);
                        let wtn = &naming.wires[&wt];
                        if !self.pin_int_wire(crds[def_rt], wtn, wti) {
                            let tname = &ntile.names[def_rt];
                            println!(
                                "INT NODE MISSING FOR {p} {tname} {wtn} {wn}",
                                p = self.rd.part,
                                wn = self.print_nw(wt),
                            );
                        }
                        if Some(&naming.wires[&wt]) == naming.wires.get(&tmux.primary_src.tw) {
                            let wfi = self.grid.tile_wire(tcrd, tmux.primary_src.tw);
                            let iwd = self.int_wire_data.get_mut(&wfi).unwrap();
                            iwd.node = Some(self.rd.lookup_wire(crds[def_rt], wtn).unwrap());
                        }
                    }
                }
                BelInfo::GroupTestMux(tm) => {
                    for (&wt, tmux) in &tm.wires {
                        let wti = self.grid.tile_wire(tcrd, wt);
                        let wtn = &naming.wires[&wt];
                        if !self.pin_int_wire(crds[def_rt], wtn, wti) {
                            let tname = &ntile.names[def_rt];
                            println!(
                                "INT NODE MISSING FOR {p} {tname} {wtn} {wn}",
                                p = self.rd.part,
                                wn = self.print_nw(wt),
                            );
                        }
                        if Some(&naming.wires[&wt]) == naming.wires.get(&tmux.primary_src.tw) {
                            let wfi = self.grid.tile_wire(tcrd, tmux.primary_src.tw);
                            let iwd = self.int_wire_data.get_mut(&wfi).unwrap();
                            iwd.node = Some(self.rd.lookup_wire(crds[def_rt], wtn).unwrap());
                        }
                    }
                }
                _ => (),
            }
        }
    }

    fn handle_tile(&mut self, tcrd: TileCoord) {
        let tile = &self.grid[tcrd];
        let crds;
        if let Some(c) = self.get_node_crds(tcrd) {
            crds = c;
        } else {
            return;
        }
        let Some(ntile) = self.ngrid.tiles.get(&tcrd) else {
            return;
        };
        let def_rt = RawTileId::from_idx(0);
        let tcls = &self.db[tile.class];
        let naming = &self.ndb.tile_class_namings[ntile.naming];
        let nui = &self.node_used[tile.class];
        let mut wire_lut = HashMap::new();
        for &w in nui.used_i.iter().chain(nui.used_o.iter()) {
            let ww = self.grid.tile_wire(tcrd, w);
            wire_lut.insert(w, self.ngrid.resolve_wire_raw(ww));
        }
        let mut wires_pinned = HashSet::new();
        let mut wires_missing = HashSet::new();
        let mut tie_pins_extra = HashMap::new();
        let mut pips = vec![];
        for bel in tcls.bels.values() {
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
            for item in &sb.items {
                match item {
                    SwitchBoxItem::Mux(mux) => {
                        for src in mux.src.keys() {
                            pips.push((mux.dst, src.tw));
                        }
                    }
                    SwitchBoxItem::ProgBuf(buf) => {
                        pips.push((buf.dst, buf.src.tw));
                    }
                    SwitchBoxItem::PermaBuf(buf) => {
                        pips.push((buf.dst, buf.src.tw));
                    }
                    SwitchBoxItem::Pass(pass) => {
                        pips.push((pass.dst, pass.src));
                    }
                    SwitchBoxItem::BiPass(pass) => {
                        pips.push((pass.a, pass.b));
                        pips.push((pass.b, pass.a));
                    }
                    SwitchBoxItem::ProgDelay(delay) => {
                        let Some(wti) = wire_lut[&delay.dst] else {
                            continue;
                        };
                        let Some(wfi) = wire_lut[&delay.src.tw] else {
                            continue;
                        };
                        let wtn = &naming.wires[&delay.dst];
                        let wdn = &naming.delay_wires[&delay.dst];
                        let wfn = &naming.wires[&delay.src];
                        if !self.pin_int_wire(crds[def_rt], wfn, wfi) {
                            let tname = &ntile.names[def_rt];
                            println!(
                                "INT NODE MISSING FOR {p} {tname} {wfn} {wn}",
                                p = self.rd.part,
                                wn = self.print_nw(delay.src.tw),
                            );
                        }
                        if !self.pin_int_wire(crds[def_rt], wtn, wti)
                            && self.int_wire_data[&wti].used_i
                        {
                            let tname = &ntile.names[def_rt];
                            println!(
                                "INT NODE MISSING FOR {p} {tname} {wtn} {wn}",
                                p = self.rd.part,
                                wn = self.print_nw(delay.dst),
                            );
                        }
                        self.claim_net(&[(crds[def_rt], wdn)]);
                        self.claim_pip(crds[def_rt], wtn, wfn);
                        self.claim_pip(crds[def_rt], wtn, wdn);
                        self.claim_pip(crds[def_rt], wdn, wfn);
                    }
                    _ => (),
                }
            }
        }
        for (wt, wf) in pips {
            let Some(wti) = wire_lut[&wt] else { continue };
            let wftie = self.db[wf.wire].is_tie();
            let pip_found;
            if let Some(en) = naming.ext_pips.get(&(wt, wf)) {
                if !crds.contains_id(en.tile) {
                    pip_found = false;
                } else if wftie {
                    if !wires_pinned.contains(&wf) {
                        wires_pinned.insert(wf);
                        self.claim_net(&[(crds[en.tile], &en.wire_from)]);
                        tie_pins_extra.insert(wf.wire, &en.wire_from);
                    }
                    pip_found = self.pin_int_wire(crds[en.tile], &en.wire_to, wti);
                    if pip_found {
                        self.claim_pip(crds[en.tile], &en.wire_to, &en.wire_from);
                    }
                } else {
                    let Some(wfi) = wire_lut[&wf] else { continue };
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
                    self.claim_net(&[(crds[def_rt], &naming.wires[&wf])]);
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
                            self.claim_net(&[
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
                        tile = ntile.names[def_rt],
                        wt = self.print_nw(wt),
                        wf = self.print_nw(wf)
                    );
                }
            }
        }
        if let Some(ref tn) = ntile.tie_name {
            let mut pins = vec![];
            for (&k, v) in &naming.wires {
                let pin = match self.db[k.wire] {
                    WireKind::Tie0 => self.ngrid.tie_pin_gnd.as_ref().unwrap(),
                    WireKind::Tie1 => self.ngrid.tie_pin_vcc.as_ref().unwrap(),
                    WireKind::TiePullup => self.ngrid.tie_pin_pullup.as_ref().unwrap(),
                    _ => continue,
                };
                if !wires_pinned.contains(&k) {
                    self.claim_net(&[(crds[ntile.tie_rt], v)]);
                }
                pins.push(SitePin {
                    dir: SitePinDir::Out,
                    pin,
                    wire: Some(v),
                });
            }
            for (k, v) in tie_pins_extra {
                let pin = match self.db[k] {
                    WireKind::Tie0 => self.ngrid.tie_pin_gnd.as_ref().unwrap(),
                    WireKind::Tie1 => self.ngrid.tie_pin_vcc.as_ref().unwrap(),
                    WireKind::TiePullup => self.ngrid.tie_pin_pullup.as_ref().unwrap(),
                    _ => continue,
                };
                pins.push(SitePin {
                    dir: SitePinDir::Out,
                    pin,
                    wire: Some(v),
                })
            }
            self.claim_site(
                crds[ntile.tie_rt],
                tn,
                self.ngrid.tie_kind.as_ref().unwrap(),
                &pins,
            );
        }

        for (slot, bel) in &tcls.bels {
            match bel {
                BelInfo::SwitchBox(_) => (),
                BelInfo::Bel(_bel) => todo!(),
                BelInfo::Legacy(bel) => {
                    let BelNaming::Bel(bn) = &naming.bels[slot] else {
                        unreachable!()
                    };
                    for (k, v) in &bel.pins {
                        if self.skip_bel_pins.contains(&(tcrd.bel(slot), k)) {
                            continue;
                        }
                        let n = &bn.pins[k];
                        let mut crd = crds[bn.tile];
                        let mut wn: &str = &n.name;
                        for pip in &n.pips {
                            let ncrd = crds[pip.tile];
                            wn = match v.dir {
                                PinDir::Input => {
                                    self.claim_net(&[(crd, wn), (ncrd, &pip.wire_to)]);
                                    self.claim_pip(ncrd, &pip.wire_to, &pip.wire_from);
                                    &pip.wire_from
                                }
                                PinDir::Output => {
                                    self.claim_net(&[(crd, wn), (ncrd, &pip.wire_from)]);
                                    self.claim_pip(ncrd, &pip.wire_to, &pip.wire_from);
                                    &pip.wire_to
                                }
                                PinDir::Inout => unreachable!(),
                            };
                            crd = ncrd;
                        }
                        if n.pips.is_empty() {
                            wn = &n.name_far;
                        }
                        let mut claim = true;
                        for &w in &v.wires {
                            let wire = self
                                .ngrid
                                .resolve_wire_raw(self.grid.tile_wire(tcrd, w))
                                .unwrap();
                            let wcrd;
                            let ww: &str;
                            if let Some(pip) = n.int_pips.get(&w) {
                                self.claim_pip(crds[pip.tile], &pip.wire_to, &pip.wire_from);
                                if v.dir == PinDir::Input {
                                    self.verify_net(&[(crd, wn), (crds[pip.tile], &pip.wire_to)]);
                                    wcrd = crds[pip.tile];
                                    ww = &pip.wire_from;
                                } else {
                                    self.verify_net(&[(crd, wn), (crds[pip.tile], &pip.wire_from)]);
                                    wcrd = crds[pip.tile];
                                    ww = &pip.wire_to;
                                }
                            } else {
                                wcrd = crd;
                                ww = wn;
                                claim = false;
                            }
                            if n.is_intf {
                                if !self.pin_int_intf_wire(wcrd, ww, wire) {
                                    println!(
                                        "MISSING BEL PIN INTF WIRE {part} {tile} {k} {wire}",
                                        part = self.rd.part,
                                        tile = ntile.names[def_rt],
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
                                            tile = ntile.names[def_rt],
                                            wire = n.name_far
                                        );
                                    }
                                }
                            }
                        }
                        if claim {
                            self.claim_net(&[(crd, wn)]);
                        }
                    }
                }
                BelInfo::TestMux(tm) => {
                    for (&wt, tmux) in &tm.wires {
                        let wti = self.grid.tile_wire(tcrd, wt);
                        let wtn = &naming.wires[&wt];
                        if !self.pin_int_wire(crds[def_rt], wtn, wti) {
                            let tname = &ntile.names[def_rt];
                            println!(
                                "INT NODE MISSING FOR {p} {tname} {wtn} {wn}",
                                p = self.rd.part,
                                wn = self.print_nw(wt),
                            );
                        }
                        if Some(&naming.wires[&wt]) != naming.wires.get(&tmux.primary_src.tw) {
                            let wf = tmux.primary_src.tw;
                            let wfi = self.grid.tile_wire(tcrd, wf);
                            if let Some(wfn) = naming.wires.get(&wf) {
                                if self.pin_int_wire(crds[def_rt], wfn, wfi) {
                                    self.claim_pip(crds[def_rt], wtn, wfn);
                                } else {
                                    let iwd = &self.int_wire_data[&wfi];
                                    if iwd.used_o {
                                        let tname = &ntile.names[def_rt];
                                        println!(
                                            "INT NODE MISSING FOR {p} {tname} {wfn} {wn}",
                                            p = self.rd.part,
                                            wn = self.print_nw(wf),
                                        );
                                    }
                                }
                            } else {
                                let iwd = &self.int_wire_data[&wfi];
                                if iwd.used_o {
                                    let tname = &ntile.names[def_rt];
                                    println!(
                                        "INTF INPUT MISSING FOR {p} {tname} {wn}",
                                        p = self.rd.part,
                                        wn = self.print_nw(wf),
                                    );
                                }
                            }
                        }
                        for &wf in tmux.test_src.keys() {
                            let wfi = self.grid.tile_wire(tcrd, wf.tw);
                            if let Some(iwi) = naming.intf_wires_in.get(&wf) {
                                let wfn = match *iwi {
                                    IntfWireInNaming::Simple { name: ref wfn } => {
                                        self.claim_pip(crds[def_rt], wtn, wfn);
                                        wfn
                                    }
                                    IntfWireInNaming::TestBuf {
                                        name_out: ref wfbn,
                                        name_in: ref wfn,
                                    } => {
                                        self.claim_pip(crds[def_rt], wtn, wfbn);
                                        wfn
                                    }
                                    IntfWireInNaming::Buf {
                                        name_in: ref wfn, ..
                                    } => {
                                        self.claim_pip(crds[def_rt], wtn, wfn);
                                        wfn
                                    }
                                };
                                if !self.pin_int_wire(crds[def_rt], wfn, wfi) {
                                    let iwd = &self.int_wire_data[&wfi];
                                    if iwd.used_o {
                                        let tname = &ntile.names[def_rt];
                                        println!(
                                            "INT NODE MISSING FOR {p} {tname} {wfn} {wn}",
                                            p = self.rd.part,
                                            wn = self.print_nw(wf.tw),
                                        );
                                    }
                                }
                            } else {
                                let iwd = &self.int_wire_data[&wfi];
                                if iwd.used_o {
                                    let tname = &ntile.names[def_rt];
                                    println!(
                                        "INTF INPUT MISSING FOR {p} {tname} {wn}",
                                        p = self.rd.part,
                                        wn = self.print_nw(wf.tw),
                                    );
                                }
                            }
                        }
                    }
                }
                BelInfo::GroupTestMux(tm) => {
                    for (&wt, tmux) in &tm.wires {
                        let wti = self.grid.tile_wire(tcrd, wt);
                        let wtn = &naming.wires[&wt];
                        if !self.pin_int_wire(crds[def_rt], wtn, wti) {
                            let tname = &ntile.names[def_rt];
                            println!(
                                "INT NODE MISSING FOR {p} {tname} {wtn} {wn}",
                                p = self.rd.part,
                                wn = self.print_nw(wt),
                            );
                        }
                        if Some(&naming.wires[&wt]) != naming.wires.get(&tmux.primary_src.tw) {
                            let wf = tmux.primary_src.tw;
                            let wfi = self.grid.tile_wire(tcrd, wf);
                            if let Some(wfn) = naming.wires.get(&wf) {
                                if self.pin_int_wire(crds[def_rt], wfn, wfi) {
                                    self.claim_pip(crds[def_rt], wtn, wfn);
                                } else {
                                    let iwd = &self.int_wire_data[&wfi];
                                    if iwd.used_o {
                                        let tname = &ntile.names[def_rt];
                                        println!(
                                            "INT NODE MISSING FOR {p} {tname} {wfn} {wn}",
                                            p = self.rd.part,
                                            wn = self.print_nw(wf),
                                        );
                                    }
                                }
                            } else {
                                let iwd = &self.int_wire_data[&wfi];
                                if iwd.used_o {
                                    let tname = &ntile.names[def_rt];
                                    println!(
                                        "INTF INPUT MISSING FOR {p} {tname} {wn}",
                                        p = self.rd.part,
                                        wn = self.print_nw(wf),
                                    );
                                }
                            }
                        }
                        for &wf in &tmux.test_src {
                            let Some(wf) = wf else { continue };
                            let wfi = self.grid.tile_wire(tcrd, wf.tw);
                            if let Some(iwi) = naming.intf_wires_in.get(&wf) {
                                let wfn = match *iwi {
                                    IntfWireInNaming::Simple { name: ref wfn } => {
                                        self.claim_pip(crds[def_rt], wtn, wfn);
                                        wfn
                                    }
                                    IntfWireInNaming::TestBuf {
                                        name_out: ref wfbn,
                                        name_in: ref wfn,
                                    } => {
                                        self.claim_pip(crds[def_rt], wtn, wfbn);
                                        wfn
                                    }
                                    IntfWireInNaming::Buf {
                                        name_in: ref wfn, ..
                                    } => {
                                        self.claim_pip(crds[def_rt], wtn, wfn);
                                        wfn
                                    }
                                };
                                if !self.pin_int_wire(crds[def_rt], wfn, wfi) {
                                    let iwd = &self.int_wire_data[&wfi];
                                    if iwd.used_o {
                                        let tname = &ntile.names[def_rt];
                                        println!(
                                            "INT NODE MISSING FOR {p} {tname} {wfn} {wn}",
                                            p = self.rd.part,
                                            wn = self.print_nw(wf.tw),
                                        );
                                    }
                                }
                            } else {
                                let iwd = &self.int_wire_data[&wfi];
                                if iwd.used_o {
                                    let tname = &ntile.names[def_rt];
                                    println!(
                                        "INTF INPUT MISSING FOR {p} {tname} {wn}",
                                        p = self.rd.part,
                                        wn = self.print_nw(wf.tw),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        for (&wf, iwin) in &naming.intf_wires_in {
            if let IntfWireInNaming::TestBuf { name_out, name_in } = iwin {
                self.claim_net(&[(crds[def_rt], name_out)]);
                self.claim_pip(crds[def_rt], name_out, name_in);
            }
            if let IntfWireInNaming::Buf { name_out, name_in } = iwin
                && self.pin_int_intf_wire(crds[def_rt], name_out, self.grid.tile_wire(tcrd, wf))
            {
                self.claim_pip(crds[def_rt], name_out, name_in);
            }
        }
    }

    pub fn handle_connector(&mut self, ccrd: ConnectorCoord) {
        let conn = &self.grid[ccrd];
        let Some(nconn) = &self.ngrid.conns.get(&ccrd) else {
            return;
        };
        let tn = &self.ndb.conn_class_namings[nconn.naming];
        let tk = &self.db[conn.class];
        let crd;
        if let Some(c) = self.xlat_tile(&nconn.tile) {
            crd = c;
        } else {
            println!("MISSING TERM TILE {n}", n = nconn.tile);
            return;
        }
        let crd_far;
        if let Some(n) = &nconn.tile_far {
            if let Some(c) = self.xlat_tile(n) {
                crd_far = Some(c);
            } else {
                println!("MISSING PASS TILE {n}");
                return;
            }
        } else {
            crd_far = None;
        }
        for (w, twn) in &tn.wires_out {
            let wt = self.ngrid.resolve_wire_raw(ccrd.cell.wire(w));
            if wt.is_none() {
                continue;
            }
            let wt = wt.unwrap();
            let tkw = &tk.wires[w];
            let wf = match *tkw {
                ConnectorWire::Reflect(wf) => ccrd.cell.wire(wf),
                ConnectorWire::Pass(wf) => conn.target.unwrap().wire(wf),
                _ => unreachable!(),
            };
            let wf = self.ngrid.resolve_wire_raw(wf);
            if wf.is_none() {
                continue;
            }
            let wf = wf.unwrap();
            let pip_found;
            match *twn {
                ConnectorWireOutNaming::Simple { name: ref wtn } => {
                    let wtf = self.pin_int_wire(crd, wtn, wt);
                    match *tkw {
                        ConnectorWire::Reflect(wfw) => {
                            let wfn = &tn.wires_in_near[wfw];
                            let wff = self.pin_int_wire(crd, wfn, wf);
                            pip_found = wtf && wff;
                            if pip_found {
                                self.claim_pip(crd, wtn, wfn);
                            }
                        }
                        ConnectorWire::Pass(wfw) => match tn.wires_in_far[wfw] {
                            ConnectorWireInFarNaming::Simple { name: ref wfn } => {
                                let wff = self.pin_int_wire(crd, wfn, wf);
                                pip_found = wtf && wff;
                                if pip_found {
                                    self.claim_pip(crd, wtn, wfn);
                                }
                            }
                            ConnectorWireInFarNaming::Buf {
                                name_out: ref wfn,
                                name_in: ref wfin,
                            } => {
                                let wff = self.pin_int_wire(crd, wfin, wf);
                                pip_found = wtf && wff;
                                if pip_found {
                                    self.claim_net(&[(crd, wfn)]);
                                    self.claim_pip(crd, wtn, wfn);
                                    self.claim_pip(crd, wfn, wfin);
                                }
                            }
                            ConnectorWireInFarNaming::BufFar {
                                name: ref wfn,
                                name_far_out: ref wffon,
                                name_far_in: ref wffin,
                            } => {
                                let wff = self.pin_int_wire(crd_far.unwrap(), wffin, wf);
                                pip_found = wtf && wff;
                                if pip_found {
                                    self.claim_net(&[(crd, wfn), (crd_far.unwrap(), wffon)]);
                                    self.claim_pip(crd_far.unwrap(), wffon, wffin);
                                    self.claim_pip(crd, wtn, wfn);
                                }
                            }
                        },
                        _ => unreachable!(),
                    }
                }
                ConnectorWireOutNaming::Buf {
                    name_out: ref wtn,
                    name_in: ref wfn,
                } => {
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
                        tile = nconn.tile,
                        wt = self.print_w(w)
                    );
                }
            }
        }
    }

    pub fn handle_int(&mut self) {
        for (tcrd, _) in self.grid.tiles() {
            self.handle_tile_tmux(tcrd);
        }
        for (tcrd, _) in self.grid.tiles() {
            self.handle_tile(tcrd);
        }
        for (ccrd, _) in self.grid.connectors() {
            self.handle_connector(ccrd);
        }
    }

    pub fn verify_bel_dummies(
        &mut self,
        bel: &BelContext<'_>,
        kind: &str,
        extras: &[(&str, SitePinDir)],
        skip: &[&str],
        dummies: &[&str],
    ) {
        let mut pins = Vec::new();
        for (k, v) in &bel.info.pins {
            if skip.contains(&&**k) {
                continue;
            }
            let n = &bel.naming.pins[k];
            pins.push(SitePin {
                dir: match v.dir {
                    PinDir::Input => SitePinDir::In,
                    PinDir::Output => SitePinDir::Out,
                    PinDir::Inout => SitePinDir::Inout,
                },
                pin: k,
                wire: Some(&n.name),
            });
        }
        for (pin, dir) in extras.iter().copied() {
            if dummies.contains(&pin) {
                pins.push(SitePin {
                    dir,
                    pin,
                    wire: None,
                });
            } else {
                if !bel.naming.pins.contains_key(pin) {
                    panic!(
                        "MISSING PIN NAME {slot} {pin}",
                        slot = self.grid.db.bel_slots.key(bel.bel.slot)
                    );
                }
                pins.push(SitePin {
                    dir,
                    pin,
                    wire: Some(&bel.naming.pins[pin].name),
                });
            }
        }
        if let Some(name) = bel.name {
            self.claim_site(bel.crd(), name, kind, &pins);
        } else {
            println!(
                "MISSING SITE NAME {tiles:?} {slot}",
                tiles = bel.tile.cells,
                slot = self.grid.db.bel_slots.key(bel.bel.slot)
            );
        }
    }

    pub fn verify_bel(
        &mut self,
        bel: &BelContext<'_>,
        kind: &str,
        extras: &[(&str, SitePinDir)],
        skip: &[&str],
    ) {
        self.verify_bel_dummies(bel, kind, extras, skip, &[]);
    }

    pub fn get_bel(&self, bel: BelCoord) -> BelContext<'a> {
        self.find_bel(bel)
            .unwrap_or_else(|| panic!("{}", bel.to_string(self.db)))
    }

    pub fn find_bel(&self, bel: BelCoord) -> Option<BelContext<'a>> {
        let tcrd = self.grid.find_tile_by_bel(bel)?;
        let tile = &self.grid[tcrd];
        let crds = self.get_node_crds(tcrd).unwrap();
        let nk = &self.db[tile.class];
        let ntile = &self.ngrid.tiles[&tcrd];
        let nn = &self.ndb.tile_class_namings[ntile.naming];
        let BelInfo::Legacy(info) = &nk.bels[bel.slot] else {
            unreachable!()
        };
        let BelNaming::Bel(naming) = &nn.bels[bel.slot] else {
            unreachable!()
        };
        Some(BelContext {
            die: bel.cell.die,
            col: bel.cell.col,
            row: bel.cell.row,
            cell: bel.cell,
            slot: bel.slot,
            bel,
            tile,
            ntile,
            tcls: self.db.tile_classes.key(tile.class),
            info,
            naming,
            crds,
            name: ntile.bels.get(bel.slot).map(|x| &**x),
        })
    }

    pub fn find_bel_delta(
        &self,
        bel: &BelContext<'_>,
        dx: isize,
        dy: isize,
        slot: BelSlotId,
    ) -> Option<BelContext<'a>> {
        let nc = bel.col.to_idx() as isize + dx;
        let nr = bel.row.to_idx() as isize + dy;
        if nc < 0 || nr < 0 {
            return None;
        }
        let nc = nc as usize;
        let nr = nr as usize;
        if nc >= self.grid.cols(bel.die).len() || nr >= self.grid.rows(bel.die).len() {
            return None;
        }
        self.find_bel(
            bel.bel
                .cell
                .with_cr(ColId::from_idx(nc), RowId::from_idx(nr))
                .bel(slot),
        )
    }

    pub fn find_bel_walk(
        &self,
        bel: &BelContext<'_>,
        dx: isize,
        dy: isize,
        slot: BelSlotId,
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
            if c >= self.grid.cols(bel.die).len() || r >= self.grid.rows(bel.die).len() {
                return None;
            }
            if let Some(x) = self.find_bel(
                bel.cell
                    .with_cr(ColId::from_idx(c), RowId::from_idx(r))
                    .bel(slot),
            ) {
                return Some(x);
            }
        }
    }

    #[track_caller]
    pub fn find_bel_sibling(&self, bel: &BelContext<'_>, slot: BelSlotId) -> BelContext<'a> {
        self.get_bel(bel.cell.bel(slot))
    }

    pub fn skip_residual_sites(&mut self) {
        self.skip_residual_sites = true;
    }

    pub fn skip_residual_pips(&mut self) {
        self.skip_residual_pips = true;
    }

    pub fn skip_residual_nodes(&mut self) {
        self.skip_residual_nodes = true;
    }

    pub fn skip_residual(&mut self) {
        self.skip_residual_sites();
        self.skip_residual_pips();
        self.skip_residual_nodes();
    }

    pub fn kill_stub_out(&mut self, name: &str) {
        if let Some(wi) = self.rd.wires.get(name) {
            self.stub_outs.insert(wi);
        }
    }

    pub fn kill_stub_out_cond(&mut self, name: &str) {
        if let Some(wi) = self.rd.wires.get(name) {
            self.cond_stub_outs.insert(wi);
        }
    }

    pub fn kill_stub_in(&mut self, name: &str) {
        if let Some(wi) = self.rd.wires.get(name) {
            self.stub_ins.insert(wi);
        }
    }

    pub fn kill_stub_in_cond(&mut self, name: &str) {
        if let Some(wi) = self.rd.wires.get(name) {
            self.cond_stub_ins.insert(wi);
        }
    }

    pub fn kill_stub_in_cond_tk(&mut self, tk: &str, name: &str) {
        if let Some((tki, _)) = self.rd.tile_kinds.get(tk)
            && let Some(wi) = self.rd.wires.get(name)
        {
            self.cond_stub_ins_tk.insert((tki, wi));
        }
    }

    pub fn skip_bel_pin(&mut self, bel: BelCoord, pin: &'static str) {
        self.skip_bel_pins.insert((bel, pin));
    }

    fn finish(mut self) {
        let mut cond_stub_outs = HashMap::new();
        let mut cond_stub_ins = HashMap::new();
        for (&crd, tile) in &self.rd.tiles {
            let tk = &self.rd.tile_kinds[tile.kind];
            for &w in tk.wires.keys() {
                if self.stub_outs.contains(&w) || self.stub_ins.contains(&w) {
                    self.claim_net(&[(crd, &self.rd.wires[w])]);
                }
                if let Some(nw) = self.rd.lookup_wire_raw(crd, w) {
                    if self.cond_stub_outs.contains(&w) && !self.is_claimed_raw(crd, w) {
                        cond_stub_outs.insert(nw, (crd, w));
                    }
                    if (self.cond_stub_ins.contains(&w)
                        || self.cond_stub_ins_tk.contains(&(tile.kind, w)))
                        && !self.is_claimed_raw(crd, w)
                    {
                        cond_stub_ins.insert(nw, (crd, w));
                    }
                }
            }
        }
        for (&nw, &(crd, w)) in &cond_stub_outs {
            self.claim_raw_node(nw, crd, &self.rd.wires[w]);
        }
        for (&nw, &(crd, w)) in &cond_stub_ins {
            self.claim_raw_node(nw, crd, &self.rd.wires[w]);
        }
        for (&crd, tile) in &self.rd.tiles {
            let tk = &self.rd.tile_kinds[tile.kind];
            for &(wf, wt) in tk.pips.keys() {
                if let Some(nwf) = self.rd.lookup_wire(crd, &self.rd.wires[wf])
                    && let Some(nwt) = self.rd.lookup_wire(crd, &self.rd.wires[wt])
                    && (self.stub_outs.contains(&wt)
                        || self.stub_ins.contains(&wf)
                        || cond_stub_outs.contains_key(&nwt)
                        || cond_stub_ins.contains_key(&nwf))
                {
                    self.claim_pip(crd, &self.rd.wires[wt], &self.rd.wires[wf]);
                }
            }
        }

        if self.skip_residual_sites && self.skip_residual_pips && self.skip_residual_nodes {
            return;
        }
        for (&crd, tile) in &self.rd.tiles {
            let tk = &self.rd.tile_kinds[tile.kind];
            if !self.skip_residual_sites {
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
            }
            if !self.skip_residual_pips {
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
            }
            if !self.skip_residual_nodes {
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
                            if let Some(&node) = tile.conn_wires.get(ci)
                                && !self.claimed_nodes[node]
                            {
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
    grid: &ExpandedGridNaming,
    extra_pre: impl FnOnce(&mut Verifier),
    bel_handler: impl Fn(&mut Verifier, &BelContext<'_>),
    extra: impl FnOnce(&mut Verifier),
) {
    let mut vrf = Verifier::new(rd, grid);
    extra_pre(&mut vrf);
    vrf.prep_int_wires();
    vrf.handle_int();
    for (tcrd, tile) in grid.egrid.tiles() {
        let tcls = &grid.egrid.db[tile.class];
        for (slot, bel) in &tcls.bels {
            if matches!(bel, BelInfo::Legacy(_)) {
                let ctx = vrf.get_bel(tcrd.bel(slot));
                bel_handler(&mut vrf, &ctx);
            }
        }
    }
    extra(&mut vrf);
    vrf.finish();
}
