#![allow(clippy::unnecessary_unwrap)]

use prjcombine_entity::{EntityBitVec, EntityBundleItemIndex, EntityId, EntityPartVec, EntityVec};
use prjcombine_interconnect::db::{
    BelBidirId, BelInfo, BelInput, BelInputId, BelKind, BelOutputId, BelSlotId, ConnectorWire,
    IntDb, LegacyBel, PinDir, SwitchBoxItem, TileClassId, TileWireCoord, WireKind, WireSlotId,
};
use prjcombine_interconnect::grid::{
    BelCoord, CellCoord, ColId, ConnectorCoord, DieId, ExpandedGrid, RowId, Tile, TileCoord,
    WireCoord,
};
use prjcombine_re_xilinx_naming::db::{
    BelNaming, ConnectorWireInFarNaming, ConnectorWireOutNaming, IntfWireInNaming, NamingDb,
    PipNaming, RawTileId,
};
use prjcombine_re_xilinx_naming::grid::{ExpandedGridNaming, TileNaming};
use prjcombine_re_xilinx_rawdump::{self as rawdump, Coord, NodeOrWire, Part};
use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct RawWireCoord<'a> {
    pub crd: Coord,
    pub wire: &'a str,
}

#[derive(Debug)]
pub struct LegacyBelContext<'a> {
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
    pub naming: &'a BelNaming,
    pub name: Option<&'a str>,
    pub crds: EntityPartVec<RawTileId, Coord>,
}

impl<'a> LegacyBelContext<'a> {
    pub fn crd(&self) -> Coord {
        self.crds[self.naming.tiles[0]]
    }

    #[track_caller]
    pub fn wire(&self, name: &str) -> RawWireCoord<'a> {
        RawWireCoord {
            crd: self.crd(),
            wire: &self.naming.pins[name].name,
        }
    }

    #[track_caller]
    pub fn wire_far(&self, name: &str) -> RawWireCoord<'a> {
        RawWireCoord {
            crd: self.crd(),
            wire: &self.naming.pins[name].name_far,
        }
    }

    #[track_caller]
    pub fn pip(&self, pin: &str, idx: usize) -> (RawWireCoord<'a>, RawWireCoord<'a>) {
        let pip = &self.naming.pins[pin].pips[idx];
        name_pip(&self.crds, pip)
    }

    #[track_caller]
    pub fn pip_owire(&self, pin: &str, idx: usize) -> RawWireCoord<'a> {
        self.pip(pin, idx).0
    }

    #[track_caller]
    pub fn pip_iwire(&self, pin: &str, idx: usize) -> RawWireCoord<'a> {
        self.pip(pin, idx).1
    }
}

pub struct BelVerifier<'a, 'b> {
    pub vrf: &'b mut Verifier<'a>,
    pub ntile: &'a TileNaming,
    pub naming: &'a BelNaming,
    pub crds: EntityPartVec<RawTileId, Coord>,
    bcrd: BelCoord,
    kind: String,
    extra_ins: Vec<(String, String)>,
    extra_outs: Vec<(String, String)>,
    bidir_dirs: EntityPartVec<BelBidirId, SitePinDir>,
    skip_ins: HashSet<BelInputId>,
    skip_outs: HashSet<BelOutputId>,
    rename_ins: EntityPartVec<BelInputId, String>,
    rename_outs: EntityPartVec<BelOutputId, String>,
    sub: usize,
}

impl<'a, 'b> BelVerifier<'a, 'b> {
    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = kind.into();
        self
    }

    pub fn extra_in(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.extra_ins.push((name.clone(), name));
        self
    }

    pub fn extra_out(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.extra_outs.push((name.clone(), name));
        self
    }

    pub fn extra_in_rename(mut self, pin: impl Into<String>, name: impl Into<String>) -> Self {
        let pin = pin.into();
        let name = name.into();
        self.extra_ins.push((pin, name));
        self
    }

    pub fn extra_out_rename(mut self, pin: impl Into<String>, name: impl Into<String>) -> Self {
        let pin = pin.into();
        let name = name.into();
        self.extra_outs.push((pin, name));
        self
    }

    pub fn crd(&self) -> Coord {
        self.crds[self.naming.tiles[self.sub]]
    }

    #[track_caller]
    pub fn wire(&self, name: &str) -> RawWireCoord<'a> {
        self.vrf.bel_wire(self.bcrd, name)
    }

    #[track_caller]
    pub fn wire_far(&self, name: &str) -> RawWireCoord<'a> {
        self.vrf.bel_wire_far(self.bcrd, name)
    }

    pub fn bel_wire(&self, bcrd: BelCoord, name: &str) -> RawWireCoord<'a> {
        self.vrf.bel_wire(bcrd, name)
    }

    pub fn bel_wire_far(&self, bcrd: BelCoord, name: &str) -> RawWireCoord<'a> {
        self.vrf.bel_wire_far(bcrd, name)
    }

    pub fn claim_net(&mut self, tiles: &[RawWireCoord]) {
        self.vrf.claim_net(tiles);
    }

    pub fn verify_net(&mut self, tiles: &[RawWireCoord]) {
        self.vrf.verify_net(tiles);
    }

    pub fn claim_pip(&mut self, wt: RawWireCoord, wf: RawWireCoord) {
        self.vrf.claim_pip(wt, wf);
    }

    pub fn bidir_dir(mut self, pin: BelBidirId, dir: SitePinDir) -> Self {
        self.bidir_dirs.insert(pin, dir);
        self
    }

    pub fn skip_in(mut self, pin: BelInputId) -> Self {
        self.skip_ins.insert(pin);
        self
    }

    pub fn skip_out(mut self, pin: BelOutputId) -> Self {
        self.skip_outs.insert(pin);
        self
    }

    pub fn rename_in(mut self, pin: BelInputId, arg: impl Into<String>) -> Self {
        self.rename_ins.insert(pin, arg.into());
        self
    }

    pub fn rename_out(mut self, pin: BelOutputId, arg: impl Into<String>) -> Self {
        self.rename_outs.insert(pin, arg.into());
        self
    }

    #[allow(clippy::should_implement_trait)]
    pub fn sub(mut self, sub: usize) -> Self {
        self.sub = sub;
        self
    }

    pub fn commit(self) {
        let db = self.vrf.db;
        let tcrd = self.vrf.grid.get_tile_by_bel(self.bcrd);
        let tile = &self.vrf.grid[tcrd];
        let tcls = &db[tile.class];
        let BelKind::Class(bcid) = db.bel_slots[self.bcrd.slot].kind else {
            unreachable!()
        };
        let BelInfo::Bel(ref bel) = tcls.bels[self.bcrd.slot] else {
            unreachable!()
        };
        let bcls = &db[bcid];
        let mut pins = vec![];
        for pid in bel.inputs.ids() {
            if self.skip_ins.contains(&pid) {
                continue;
            }
            let (name, idx) = bcls.inputs.key(pid);
            let name = self.vrf.pin_index(name, idx);
            let n = &self.naming.pins[&name];
            let pin = if let Some(pin) = self.rename_ins.get(pid) {
                pin.into()
            } else {
                name.into()
            };
            pins.push(SitePin {
                dir: SitePinDir::In,
                pin,
                wire: Some(&n.name),
            });
        }
        for pid in bel.outputs.ids() {
            if self.skip_outs.contains(&pid) {
                continue;
            }
            let (name, idx) = bcls.outputs.key(pid);
            let name = self.vrf.pin_index(name, idx);
            let n = &self.naming.pins[&name];
            let pin = if let Some(pin) = self.rename_outs.get(pid) {
                pin.into()
            } else {
                name.into()
            };
            pins.push(SitePin {
                dir: SitePinDir::Out,
                pin,
                wire: Some(&n.name),
            });
        }
        for pid in bel.bidirs.ids() {
            let (name, idx) = bcls.bidirs.key(pid);
            let name = self.vrf.pin_index(name, idx);
            let n = &self.naming.pins[&name];
            let dir = self
                .bidir_dirs
                .get(pid)
                .copied()
                .unwrap_or(SitePinDir::Inout);
            pins.push(SitePin {
                dir,
                pin: name.into(),
                wire: Some(&n.name),
            });
        }
        for (pin, name) in self.extra_ins {
            let wire = Some(self.naming.pins[&name].name.as_ref());
            pins.push(SitePin {
                dir: SitePinDir::In,
                pin: pin.into(),
                wire,
            });
        }
        for (pin, name) in self.extra_outs {
            let wire = Some(self.naming.pins[&name].name.as_ref());
            pins.push(SitePin {
                dir: SitePinDir::Out,
                pin: pin.into(),
                wire,
            });
        }
        if let Some(name) = self.vrf.ngrid.get_bel_name_sub(self.bcrd, self.sub) {
            let crd = self.vrf.bel_rcrd(self.bcrd);
            self.vrf.claim_site(crd, name, &self.kind, &pins);
        } else {
            println!(
                "MISSING SITE NAME {tiles:?} {slot}",
                tiles = tile.cells,
                slot = db.bel_slots.key(self.bcrd.slot)
            );
        }
    }
}

#[derive(Debug, Clone)]
pub struct SitePin<'a> {
    pub dir: SitePinDir,
    pub pin: Cow<'a, str>,
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
    wire_slot_aliases: HashMap<WireSlotId, WireSlotId>,
    intf_int_aliases: HashMap<WireCoord, WireCoord>,
    skip_tcls_pips: EntityPartVec<TileClassId, HashSet<(TileWireCoord, TileWireCoord)>>,
    inject_tcls_pips: EntityPartVec<TileClassId, HashSet<(TileWireCoord, TileWireCoord)>>,
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
    skip_sb: HashSet<BelSlotId>,
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
            BelInfo::Bel(bel) => {
                for &inp in bel.inputs.values() {
                    match inp {
                        BelInput::Fixed(wire) => {
                            used_i.insert(wire.tw);
                        }
                        BelInput::Invertible(wire, _) => {
                            used_i.insert(wire);
                        }
                    }
                }
                for wires in bel.outputs.values() {
                    for &wire in wires {
                        used_o.insert(wire);
                    }
                }
                for &wire in bel.bidirs.values() {
                    used_i.insert(wire);
                    used_o.insert(wire);
                }
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

fn name_pip<'a>(
    crds: &EntityPartVec<RawTileId, Coord>,
    pip: &'a PipNaming,
) -> (RawWireCoord<'a>, RawWireCoord<'a>) {
    let nwt = RawWireCoord {
        crd: crds[pip.tile],
        wire: &pip.wire_to,
    };
    let nwf = RawWireCoord {
        crd: crds[pip.tile],
        wire: &pip.wire_from,
    };
    (nwt, nwf)
}

impl<'a> Verifier<'a> {
    pub fn new(rd: &'a Part, ngrid: &'a ExpandedGridNaming) -> Self {
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
            wire_slot_aliases: HashMap::new(),
            intf_int_aliases: HashMap::new(),
            skip_tcls_pips: EntityPartVec::new(),
            inject_tcls_pips: EntityPartVec::new(),
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
            skip_sb: HashSet::new(),
        }
    }

    pub fn alias_wire_slot(&mut self, from: WireSlotId, to: WireSlotId) {
        self.wire_slot_aliases.insert(from, to);
    }

    pub fn alias_intf_int(&mut self, from: WireCoord, to: WireCoord) {
        self.intf_int_aliases.insert(from, to);
    }

    pub fn skip_tcls_pip(&mut self, tcid: TileClassId, dst: TileWireCoord, src: TileWireCoord) {
        if !self.skip_tcls_pips.contains_id(tcid) {
            self.skip_tcls_pips.insert(tcid, Default::default());
        }
        self.skip_tcls_pips[tcid].insert((dst, src));
    }

    pub fn inject_tcls_pip(&mut self, tcid: TileClassId, dst: TileWireCoord, src: TileWireCoord) {
        if !self.inject_tcls_pips.contains_id(tcid) {
            self.inject_tcls_pips.insert(tcid, Default::default());
        }
        self.inject_tcls_pips[tcid].insert((dst, src));
    }

    pub fn prep_int_wires(&mut self) {
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

    fn claim_raw_node(&mut self, nw: NodeOrWire, rw: RawWireCoord) {
        match nw {
            NodeOrWire::Node(nidx) => {
                if self.claimed_nodes[nidx] {
                    let tname = &self.rd.tiles[&rw.crd].name;
                    println!(
                        "DOUBLE CLAIMED NODE {part} {tname} {wn}",
                        part = self.rd.part,
                        wn = rw.wire,
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
                        part = self.rd.part,
                        wn = rw.wire,
                    );
                }
                ctw.set(widx, true);
            }
        }
    }

    pub fn pin_int_wire(&mut self, rw: RawWireCoord, mut iw: WireCoord) -> bool {
        if let Some(&nslot) = self.wire_slot_aliases.get(&iw.slot) {
            iw.slot = nslot;
        }
        if let Some(cnw) = self.rd.lookup_wire(rw.crd, rw.wire) {
            let iwd = self.int_wire_data.get_mut(&iw).unwrap();
            if iwd.used_i && iwd.used_o {
                if iwd.node.is_none() {
                    iwd.node = Some(cnw);
                    self.claim_raw_node(cnw, rw);
                } else if iwd.node != Some(cnw) {
                    let tname = &self.rd.tiles[&rw.crd].name;
                    println!(
                        "INT NODE MISMATCH FOR {p} {tname} {wire} {iw}",
                        wire = rw.wire,
                        p = self.rd.part,
                        iw = iw.to_string(self.db),
                    );
                }
            } else if iwd.used_o {
                if !self.dummy_out_nodes.contains(&cnw) {
                    self.dummy_out_nodes.insert(cnw);
                    self.claim_raw_node(cnw, rw);
                }
            } else {
                if !self.dummy_in_nodes.contains(&cnw) {
                    self.dummy_in_nodes.insert(cnw);
                    self.claim_raw_node(cnw, rw);
                }
            }
            true
        } else {
            false
        }
    }

    pub fn claim_dummy_in(&mut self, rw: RawWireCoord) {
        if let Some(cnw) = self.rd.lookup_wire(rw.crd, rw.wire)
            && !self.dummy_in_nodes.contains(&cnw)
        {
            self.dummy_in_nodes.insert(cnw);
            self.claim_raw_node(cnw, rw);
        }
    }

    pub fn claim_dummy_out(&mut self, rw: RawWireCoord) {
        if let Some(cnw) = self.rd.lookup_wire(rw.crd, rw.wire)
            && !self.dummy_out_nodes.contains(&cnw)
        {
            self.dummy_out_nodes.insert(cnw);
            self.claim_raw_node(cnw, rw);
        }
    }

    pub fn pin_int_intf_wire(&mut self, rw: RawWireCoord, iw: WireCoord) -> bool {
        if let Some(&iw) = self.intf_int_aliases.get(&iw) {
            return self.pin_int_wire(rw, iw);
        }
        if let Some(cnw) = self.rd.lookup_wire(rw.crd, rw.wire) {
            let iwd = self.int_wire_data.get_mut(&iw).unwrap();
            if let Some(nw) = iwd.intf_node {
                if nw != cnw {
                    let tname = &self.rd.tiles[&rw.crd].name;
                    println!(
                        "INT INTF NODE MISMATCH FOR {p} {tname} {wire} {iw}",
                        wire = rw.wire,
                        p = self.rd.part,
                        iw = iw.to_string(self.db)
                    );
                }
            } else if iwd.intf_missing {
                let tname = &self.rd.tiles[&rw.crd].name;
                println!(
                    "INT INTF NODE PRESENT FOR {tname} {wire} BUT WAS MISSING PREVIOUSLY",
                    wire = rw.wire,
                );
                iwd.intf_node = Some(cnw);
                self.claim_net(&[rw]);
            } else {
                iwd.intf_node = Some(cnw);
                self.claim_net(&[rw]);
            }
            true
        } else {
            let iwd = self.int_wire_data.get_mut(&iw).unwrap();
            if iwd.intf_node.is_some() {
                let tname = &self.rd.tiles[&rw.crd].name;
                println!(
                    "INT INTF NODE PRESENT FOR {tname} {wire} BUT WIRE NOT FOUND",
                    wire = rw.wire,
                );
            } else if iwd.intf_missing {
                let tname = &self.rd.tiles[&rw.crd].name;
                println!("INT INTF WIRE {tname} {wire} MISSING TWICE", wire = rw.wire,);
            } else {
                iwd.intf_missing = true;
            }
            false
        }
    }

    pub fn verify_net(&mut self, tiles: &[RawWireCoord]) {
        let mut nw = None;
        for &rw in tiles {
            let tile = &self.rd.tiles[&rw.crd];
            let tname = &tile.name;
            if let Some(cnw) = self.rd.lookup_wire(rw.crd, rw.wire) {
                if let Some((pnw, pcrd, pwn)) = nw {
                    if pnw != cnw {
                        let ptile = &self.rd.tiles[&pcrd];
                        let ptname = &ptile.name;
                        println!(
                            "NODE MISMATCH FOR {p} {tname} {wn} != {ptname} {pwn}",
                            p = self.rd.part,
                            wn = rw.wire,
                        );
                    }
                } else {
                    nw = Some((cnw, rw.crd, rw.wire));
                }
            } else {
                println!(
                    "MISSING WIRE {part} {tname} {wn}",
                    part = self.rd.part,
                    wn = rw.wire,
                );
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

    pub fn claim_net(&mut self, tiles: &[RawWireCoord]) {
        let mut nw = None;
        for &rw in tiles {
            let tile = &self.rd.tiles[&rw.crd];
            let tname = &tile.name;
            if let Some(cnw) = self.rd.lookup_wire(rw.crd, rw.wire) {
                if let Some(pnw) = nw {
                    if pnw != cnw {
                        println!(
                            "NODE MISMATCH FOR {p} {tname} {wn}",
                            p = self.rd.part,
                            wn = rw.wire
                        );
                    }
                } else {
                    nw = Some(cnw);
                    self.claim_raw_node(cnw, rw);
                }
            } else {
                println!(
                    "MISSING NODE WIRE {part} {tname} {wn}",
                    part = self.rd.part,
                    wn = rw.wire
                );
            }
        }
    }

    pub fn claim_vcc_node(&mut self, rw: RawWireCoord) {
        let tile = &self.rd.tiles[&rw.crd];
        let tname = &tile.name;
        if let Some(cnw) = self.rd.lookup_wire(rw.crd, rw.wire) {
            if self.vcc_nodes.insert(cnw) {
                self.claim_raw_node(cnw, rw);
            }
        } else {
            println!(
                "MISSING VCC NODE WIRE {part} {tname} {wn}",
                wn = rw.wire,
                part = self.rd.part
            );
        }
    }

    pub fn claim_pip(&mut self, wtn: RawWireCoord, wfn: RawWireCoord) {
        assert_eq!(wtn.crd, wfn.crd);
        let crd = wtn.crd;
        let wt = wtn.wire;
        let wf = wfn.wire;
        self.claim_pip_tri(crd, wt, wf);
    }

    pub fn claim_pip_tri(&mut self, crd: Coord, wt: &str, wf: &str) {
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
                    if let Some(tkp) = site.pins.get(pin.pin.as_ref()) {
                        extra_pins.remove(pin.pin.as_ref());
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
                            self.claim_net(&[RawWireCoord {
                                crd,
                                wire: act_wire.unwrap(),
                            }]);
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
        if let Some(ntile) = self.ngrid.tiles.get(&tcrd) {
            for (k, name) in &ntile.names {
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
                        let wtn = &naming.wires[&wt].name;
                        if !self.pin_int_wire(
                            RawWireCoord {
                                crd: crds[def_rt],
                                wire: wtn,
                            },
                            wti,
                        ) {
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
                        let wtn = &naming.wires[&wt].name;
                        if !self.pin_int_wire(
                            RawWireCoord {
                                crd: crds[def_rt],
                                wire: wtn,
                            },
                            wti,
                        ) {
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

    fn pin_index(&self, name: &str, idx: EntityBundleItemIndex) -> String {
        match idx {
            EntityBundleItemIndex::Single => name.to_string(),
            EntityBundleItemIndex::Array { index, .. } => {
                if matches!(
                    self.rd.family.as_str(),
                    "ultrascale" | "ultrascaleplus" | "versal"
                ) {
                    format!("{name}_{index}_")
                } else {
                    format!("{name}{index}")
                }
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
        let mut pips = BTreeSet::new();
        for (bslot, bel) in &tcls.bels {
            if self.skip_sb.contains(&bslot) {
                continue;
            }
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
            for item in &sb.items {
                match item {
                    SwitchBoxItem::Mux(mux) => {
                        for src in mux.src.keys() {
                            pips.insert((mux.dst, src.tw));
                        }
                    }
                    SwitchBoxItem::ProgBuf(buf) => {
                        pips.insert((buf.dst, buf.src.tw));
                    }
                    SwitchBoxItem::PermaBuf(buf) => {
                        pips.insert((buf.dst, buf.src.tw));
                    }
                    SwitchBoxItem::Pass(pass) => {
                        pips.insert((pass.dst, pass.src));
                    }
                    SwitchBoxItem::BiPass(pass) => {
                        pips.insert((pass.a, pass.b));
                        pips.insert((pass.b, pass.a));
                    }
                    SwitchBoxItem::ProgDelay(delay) => {
                        let Some(wti) = wire_lut[&delay.dst] else {
                            continue;
                        };
                        let Some(wfi) = wire_lut[&delay.src.tw] else {
                            continue;
                        };
                        let wtn = RawWireCoord {
                            crd: crds[def_rt],
                            wire: &naming.wires[&delay.dst].name,
                        };
                        let wdn = RawWireCoord {
                            crd: crds[def_rt],
                            wire: &naming.delay_wires[&delay.dst],
                        };
                        let wfn = RawWireCoord {
                            crd: crds[def_rt],
                            wire: &naming.wires[&delay.src].name,
                        };
                        if !self.pin_int_wire(wfn, wfi) {
                            let tname = &ntile.names[def_rt];
                            println!(
                                "INT NODE MISSING FOR {p} {tname} {wfn} {wn}",
                                p = self.rd.part,
                                wfn = wfn.wire,
                                wn = self.print_nw(delay.src.tw),
                            );
                        }
                        if !self.pin_int_wire(wtn, wti) && self.int_wire_data[&wti].used_i {
                            let tname = &ntile.names[def_rt];
                            println!(
                                "INT NODE MISSING FOR {p} {tname} {wtn} {wn}",
                                p = self.rd.part,
                                wtn = wtn.wire,
                                wn = self.print_nw(delay.dst),
                            );
                        }
                        self.claim_net(&[wdn]);
                        self.claim_pip(wtn, wfn);
                        self.claim_pip(wtn, wdn);
                        self.claim_pip(wdn, wfn);
                    }
                    _ => (),
                }
            }
        }
        if let Some(skip) = self.skip_tcls_pips.get(tile.class) {
            for pip in skip {
                pips.remove(pip);
            }
        }
        if let Some(inject) = self.inject_tcls_pips.get(tile.class) {
            for &pip in inject {
                pips.insert(pip);
            }
        }
        let mut alt_wires_dst = BTreeSet::new();
        let mut alt_wires_src = BTreeSet::new();
        for (wt, wf) in pips {
            if matches!(self.db[wf.wire], WireKind::Special) {
                continue;
            }
            if wt.cell == wf.cell
                && (self.wire_slot_aliases.get(&wt.wire) == Some(&wf.wire)
                    || self.wire_slot_aliases.get(&wf.wire) == Some(&wt.wire))
            {
                continue;
            }
            let Some(wti) = wire_lut[&wt] else { continue };
            let wftie = self.db[wf.wire].is_tie();
            let pip_found;
            if let Some(en) = naming.ext_pips.get(&(wt, wf)) {
                if !crds.contains_id(en.tile) {
                    pip_found = false;
                } else {
                    let wtn = RawWireCoord {
                        crd: crds[en.tile],
                        wire: &en.wire_to,
                    };
                    let wfn = RawWireCoord {
                        crd: crds[en.tile],
                        wire: &en.wire_from,
                    };
                    if wftie {
                        if !wires_pinned.contains(&wf) {
                            wires_pinned.insert(wf);
                            self.claim_net(&[wfn]);
                            tie_pins_extra.insert(wf.wire, &en.wire_from);
                        }
                        pip_found = self.pin_int_wire(wtn, wti);
                        if pip_found {
                            self.claim_pip(wtn, wfn);
                        }
                    } else {
                        let Some(wfi) = wire_lut[&wf] else { continue };
                        let wtf = self.pin_int_wire(wtn, wti);
                        let wff = self.pin_int_wire(wfn, wfi);
                        pip_found = wtf && wff;
                        if pip_found {
                            self.claim_pip(wtn, wfn);
                        }
                    }
                }
            } else if let Some(wn) = naming.wires.get(&wt)
                && wn.alt_pips_to.contains(&wf)
            {
                let wtn = RawWireCoord {
                    crd: crds[def_rt],
                    wire: wn.alt_name.as_ref().unwrap(),
                };
                if !alt_wires_dst.contains(&wt) && !alt_wires_src.contains(&wt) {
                    self.claim_net(&[wtn]);
                }
                if !alt_wires_dst.contains(&wt) {
                    let rwtn = RawWireCoord {
                        crd: crds[def_rt],
                        wire: &wn.name,
                    };
                    self.pin_int_wire(rwtn, wire_lut[&wt].unwrap());
                    self.claim_pip(rwtn, wtn);
                }
                alt_wires_dst.insert(wt);
                let wfn = RawWireCoord {
                    crd: crds[def_rt],
                    wire: &naming.wires[&wf].name,
                };
                pip_found = self.pin_int_wire(wfn, wire_lut[&wf].unwrap());
                self.claim_pip(wtn, wfn);
            } else if let Some(wn) = naming.wires.get(&wf)
                && wn.alt_pips_from.contains(&wt)
            {
                let wfn = RawWireCoord {
                    crd: crds[def_rt],
                    wire: wn.alt_name.as_ref().unwrap(),
                };
                if !alt_wires_dst.contains(&wf) && !alt_wires_src.contains(&wf) {
                    self.claim_net(&[wfn]);
                }
                if !alt_wires_src.contains(&wf) {
                    let rwfn = RawWireCoord {
                        crd: crds[def_rt],
                        wire: &wn.name,
                    };
                    self.pin_int_wire(rwfn, wire_lut[&wf].unwrap());
                    self.claim_pip(wfn, rwfn);
                }
                alt_wires_src.insert(wf);
                let wtn = RawWireCoord {
                    crd: crds[def_rt],
                    wire: &naming.wires[&wt].name,
                };
                pip_found = self.pin_int_wire(wtn, wire_lut[&wt].unwrap());
                self.claim_pip(wtn, wfn);
            } else {
                let wtf;
                if wires_pinned.contains(&wt) {
                    wtf = true;
                } else if wires_missing.contains(&wt) {
                    wtf = false;
                } else if let Some(wn) = naming.wires.get(&wt) {
                    let wtn = RawWireCoord {
                        crd: crds[def_rt],
                        wire: &wn.name,
                    };
                    wtf = self.pin_int_wire(wtn, wti);
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
                    let wfn = RawWireCoord {
                        crd: crds[def_rt],
                        wire: &naming.wires[&wf].name,
                    };
                    self.claim_net(&[wfn]);
                    wires_pinned.insert(wf);
                    wff = true;
                } else if let Some(wn) = naming.wires.get(&wf) {
                    let Some(wfi) = wire_lut[&wf] else {
                        continue;
                    };
                    if let Some(pip) = naming.wire_bufs.get(&wf) {
                        let (wtn, wfn) = name_pip(&crds, pip);
                        wff = self.pin_int_wire(wfn, wfi);
                        if wff {
                            self.claim_pip(wtn, wfn);
                            self.claim_net(&[
                                wtn,
                                RawWireCoord {
                                    crd: crds[def_rt],
                                    wire: &naming.wires[&wf].name,
                                },
                            ]);
                        }
                    } else {
                        let wfn = RawWireCoord {
                            crd: crds[def_rt],
                            wire: &wn.name,
                        };
                        wff = self.pin_int_wire(wfn, wfi);
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
                    let wtn = RawWireCoord {
                        crd: crds[def_rt],
                        wire: &naming.wires[&wt].name,
                    };
                    let wfn = RawWireCoord {
                        crd: crds[def_rt],
                        wire: &naming.wires[&wf].name,
                    };
                    self.claim_pip(wtn, wfn);
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
            for (&k, wn) in &naming.wires {
                let pin = match self.db[k.wire] {
                    WireKind::Tie0 => self.ngrid.tie_pin_gnd.as_ref().unwrap(),
                    WireKind::Tie1 => self.ngrid.tie_pin_vcc.as_ref().unwrap(),
                    WireKind::TiePullup => self.ngrid.tie_pin_pullup.as_ref().unwrap(),
                    _ => continue,
                };
                if !wires_pinned.contains(&k) {
                    self.claim_net(&[RawWireCoord {
                        crd: crds[ntile.tie_rt],
                        wire: &wn.name,
                    }]);
                }
                pins.push(SitePin {
                    dir: SitePinDir::Out,
                    pin: pin.into(),
                    wire: Some(&wn.name),
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
                    pin: pin.into(),
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
            let bcrd = tcrd.bel(slot);
            match bel {
                BelInfo::SwitchBox(_) => (),
                BelInfo::Bel(bel) => {
                    let bn = self.ngrid.get_bel_naming(bcrd);
                    let BelKind::Class(bcid) = self.db.bel_slots[slot].kind else {
                        unreachable!()
                    };
                    let bcls = &self.db[bcid];
                    for (pin, &inp) in &bel.inputs {
                        let (name, idx) = bcls.inputs.key(pin);
                        let pin = self.pin_index(name, idx);
                        let n = &bn.pins[&pin];
                        let mut wc = RawWireCoord {
                            crd: crds[n.tile],
                            wire: &n.name,
                        };
                        for pip in &n.pips {
                            let (wtn, wfn) = name_pip(&crds, pip);
                            self.claim_net(&[wc, wtn]);
                            self.claim_pip(wtn, wfn);
                            wc = wfn;
                        }
                        if n.pips.is_empty() {
                            wc.wire = &n.name_far;
                        }

                        let w = match inp {
                            BelInput::Fixed(wire) => wire.tw,
                            BelInput::Invertible(wire, _) => wire,
                        };

                        let wire = self
                            .ngrid
                            .resolve_wire_raw(self.grid.tile_wire(tcrd, w))
                            .unwrap();
                        let wwc;
                        if let Some(pip) = n.int_pips.get(&w) {
                            let (wtn, wfn) = name_pip(&crds, pip);
                            self.claim_pip(wtn, wfn);
                            self.verify_net(&[wc, wtn]);
                            self.claim_net(&[wc]);
                            wwc = wfn;
                        } else {
                            wwc = wc;
                        }
                        if n.is_intf {
                            if !self.pin_int_intf_wire(wwc, wire) {
                                println!(
                                    "MISSING BEL PIN INTF WIRE {part} {tile} {pin} {wire}",
                                    part = self.rd.part,
                                    tile = ntile.names[def_rt],
                                    wire = n.name_far
                                );
                            }
                        } else {
                            if !self.pin_int_wire(wwc, wire) {
                                let iwd = &self.int_wire_data[&wire];
                                if iwd.used_o {
                                    println!(
                                        "MISSING BEL PIN INT WIRE {part} {tile} {pin} {wire}",
                                        part = self.rd.part,
                                        tile = ntile.names[def_rt],
                                        wire = n.name_far
                                    );
                                }
                            }
                        }
                    }
                    for (pin, wires) in &bel.outputs {
                        let (name, idx) = bcls.outputs.key(pin);
                        let pin = self.pin_index(name, idx);
                        let n = &bn.pins[&pin];
                        let mut wc = RawWireCoord {
                            crd: crds[n.tile],
                            wire: &n.name,
                        };
                        for pip in &n.pips {
                            let (wtn, wfn) = name_pip(&crds, pip);
                            self.claim_net(&[wc, wfn]);
                            self.claim_pip(wtn, wfn);
                            wc = wtn;
                        }
                        if n.pips.is_empty() {
                            wc.wire = &n.name_far;
                        }

                        let mut claim = true;
                        for &w in wires {
                            let wire = self
                                .ngrid
                                .resolve_wire_raw(self.grid.tile_wire(tcrd, w))
                                .unwrap();
                            let wwc;
                            if let Some(pip) = n.int_pips.get(&w) {
                                let (wtn, wfn) = name_pip(&crds, pip);
                                self.claim_pip(wtn, wfn);
                                self.verify_net(&[wc, wfn]);
                                wwc = wtn;
                            } else {
                                wwc = wc;
                                claim = false;
                            }
                            if !self.pin_int_wire(wwc, wire) {
                                let iwd = &self.int_wire_data[&wire];
                                if iwd.used_i {
                                    println!(
                                        "MISSING BEL PIN INT WIRE {part} {tile} {pin} {wire}",
                                        part = self.rd.part,
                                        tile = ntile.names[def_rt],
                                        wire = n.name_far
                                    );
                                }
                            }
                        }
                        if claim {
                            self.claim_net(&[wc]);
                        }
                    }
                    for (pin, &w) in &bel.bidirs {
                        let (name, idx) = bcls.bidirs.key(pin);
                        let pin = self.pin_index(name, idx);
                        let n = &bn.pins[&pin];
                        let wn = RawWireCoord {
                            crd: crds[n.tile],
                            wire: &n.name_far,
                        };

                        let wire = self
                            .ngrid
                            .resolve_wire_raw(self.grid.tile_wire(tcrd, w))
                            .unwrap();
                        assert!(n.int_pips.is_empty());
                        if !self.pin_int_wire(wn, wire) {
                            println!(
                                "MISSING BEL PIN INT WIRE {part} {tile} {pin} {wire}",
                                part = self.rd.part,
                                tile = ntile.names[def_rt],
                                wire = n.name_far
                            );
                        }
                    }
                }
                BelInfo::Legacy(bel) => {
                    let bn = self.ngrid.get_bel_naming(bcrd);
                    for (k, v) in &bel.pins {
                        if self.skip_bel_pins.contains(&(tcrd.bel(slot), k)) {
                            continue;
                        }
                        let n = &bn.pins[k];
                        let mut wc = RawWireCoord {
                            crd: crds[n.tile],
                            wire: &n.name,
                        };
                        for pip in &n.pips {
                            let (wtn, wfn) = name_pip(&crds, pip);
                            wc = match v.dir {
                                PinDir::Input => {
                                    self.claim_net(&[wc, wtn]);
                                    self.claim_pip(wtn, wfn);
                                    wfn
                                }
                                PinDir::Output => {
                                    self.claim_net(&[wc, wfn]);
                                    self.claim_pip(wtn, wfn);
                                    wtn
                                }
                                PinDir::Inout => unreachable!(),
                            };
                        }
                        if n.pips.is_empty() {
                            wc.wire = &n.name_far;
                        }
                        let mut claim = true;
                        for &w in &v.wires {
                            let wire = self
                                .ngrid
                                .resolve_wire_raw(self.grid.tile_wire(tcrd, w))
                                .unwrap();
                            let wwc;
                            if let Some(pip) = n.int_pips.get(&w) {
                                let (wtn, wfn) = name_pip(&crds, pip);
                                self.claim_pip(wtn, wfn);
                                if v.dir == PinDir::Input {
                                    self.verify_net(&[wc, wtn]);
                                    wwc = wfn;
                                } else {
                                    self.verify_net(&[wc, wfn]);
                                    wwc = wtn;
                                }
                            } else {
                                wwc = wc;
                                claim = false;
                            }
                            if n.is_intf {
                                if !self.pin_int_intf_wire(wwc, wire) {
                                    println!(
                                        "MISSING BEL PIN INTF WIRE {part} {tile} {k} {wire}",
                                        part = self.rd.part,
                                        tile = ntile.names[def_rt],
                                        wire = n.name_far
                                    );
                                }
                            } else {
                                if !self.pin_int_wire(wwc, wire) {
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
                            self.claim_net(&[wc]);
                        }
                    }
                }
                BelInfo::TestMux(tm) => {
                    for (&wt, tmux) in &tm.wires {
                        let wti = self.grid.tile_wire(tcrd, wt);
                        let wtn = RawWireCoord {
                            crd: crds[def_rt],
                            wire: &naming.wires[&wt].name,
                        };
                        if !self.pin_int_wire(wtn, wti) {
                            let tname = &ntile.names[def_rt];
                            println!(
                                "INT NODE MISSING FOR {p} {tname} {wtn} {wn}",
                                p = self.rd.part,
                                wtn = wtn.wire,
                                wn = self.print_nw(wt),
                            );
                        }
                        if Some(&naming.wires[&wt]) != naming.wires.get(&tmux.primary_src.tw) {
                            let wf = tmux.primary_src.tw;
                            let wfi = self.grid.tile_wire(tcrd, wf);
                            if let Some(wfn) = naming.wires.get(&wf) {
                                let wfn = RawWireCoord {
                                    crd: crds[def_rt],
                                    wire: &wfn.name,
                                };
                                if self.pin_int_wire(wfn, wfi) {
                                    self.claim_pip(wtn, wfn);
                                } else {
                                    let iwd = &self.int_wire_data[&wfi];
                                    if iwd.used_o {
                                        let tname = &ntile.names[def_rt];
                                        println!(
                                            "INT NODE MISSING FOR {p} {tname} {wfn} {wn}",
                                            p = self.rd.part,
                                            wfn = wfn.wire,
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
                                        let wfn = RawWireCoord {
                                            crd: crds[def_rt],
                                            wire: wfn,
                                        };

                                        self.claim_pip(wtn, wfn);
                                        wfn
                                    }
                                    IntfWireInNaming::TestBuf {
                                        name_out: ref wfbn,
                                        name_in: ref wfn,
                                    } => {
                                        let wfn = RawWireCoord {
                                            crd: crds[def_rt],
                                            wire: wfn,
                                        };
                                        let wfbn = RawWireCoord {
                                            crd: crds[def_rt],
                                            wire: wfbn,
                                        };
                                        self.claim_pip(wtn, wfbn);
                                        wfn
                                    }
                                    IntfWireInNaming::Buf {
                                        name_in: ref wfn, ..
                                    } => {
                                        let wfn = RawWireCoord {
                                            crd: crds[def_rt],
                                            wire: wfn,
                                        };
                                        self.claim_pip(wtn, wfn);
                                        wfn
                                    }
                                };
                                if !self.pin_int_wire(wfn, wfi) {
                                    let iwd = &self.int_wire_data[&wfi];
                                    if iwd.used_o {
                                        let tname = &ntile.names[def_rt];
                                        println!(
                                            "INT NODE MISSING FOR {p} {tname} {wfn} {wn}",
                                            wfn = wfn.wire,
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
                        let wtn = RawWireCoord {
                            crd: crds[def_rt],
                            wire: &naming.wires[&wt].name,
                        };
                        if !self.pin_int_wire(wtn, wti) {
                            let tname = &ntile.names[def_rt];
                            println!(
                                "INT NODE MISSING FOR {p} {tname} {wtn} {wn}",
                                wtn = wtn.wire,
                                p = self.rd.part,
                                wn = self.print_nw(wt),
                            );
                        }
                        if Some(&naming.wires[&wt]) != naming.wires.get(&tmux.primary_src.tw) {
                            let wf = tmux.primary_src.tw;
                            let wfi = self.grid.tile_wire(tcrd, wf);
                            if let Some(wfn) = naming.wires.get(&wf) {
                                let wfn = RawWireCoord {
                                    crd: crds[def_rt],
                                    wire: &wfn.name,
                                };
                                if self.pin_int_wire(wfn, wfi) {
                                    self.claim_pip(wtn, wfn);
                                } else {
                                    let iwd = &self.int_wire_data[&wfi];
                                    if iwd.used_o {
                                        let tname = &ntile.names[def_rt];
                                        println!(
                                            "INT NODE MISSING FOR {p} {tname} {wfn} {wn}",
                                            wfn = wfn.wire,
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
                                        let wfn = RawWireCoord {
                                            crd: crds[def_rt],
                                            wire: wfn,
                                        };
                                        self.claim_pip(wtn, wfn);
                                        wfn
                                    }
                                    IntfWireInNaming::TestBuf {
                                        name_out: ref wfbn,
                                        name_in: ref wfn,
                                    } => {
                                        let wfbn = RawWireCoord {
                                            crd: crds[def_rt],
                                            wire: wfbn,
                                        };
                                        let wfn = RawWireCoord {
                                            crd: crds[def_rt],
                                            wire: wfn,
                                        };
                                        self.claim_pip(wtn, wfbn);
                                        wfn
                                    }
                                    IntfWireInNaming::Buf {
                                        name_in: ref wfn, ..
                                    } => {
                                        let wfn = RawWireCoord {
                                            crd: crds[def_rt],
                                            wire: wfn,
                                        };
                                        self.claim_pip(wtn, wfn);
                                        wfn
                                    }
                                };
                                if !self.pin_int_wire(wfn, wfi) {
                                    let iwd = &self.int_wire_data[&wfi];
                                    if iwd.used_o {
                                        let tname = &ntile.names[def_rt];
                                        println!(
                                            "INT NODE MISSING FOR {p} {tname} {wfn} {wn}",
                                            p = self.rd.part,
                                            wfn = wfn.wire,
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
                self.claim_net(&[RawWireCoord {
                    crd: crds[def_rt],
                    wire: name_out,
                }]);
                self.claim_pip_tri(crds[def_rt], name_out, name_in);
            }
            if let IntfWireInNaming::Buf { name_out, name_in } = iwin
                && self.pin_int_intf_wire(
                    RawWireCoord {
                        crd: crds[def_rt],
                        wire: name_out,
                    },
                    self.grid.tile_wire(tcrd, wf),
                )
            {
                self.claim_pip_tri(crds[def_rt], name_out, name_in);
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
                    let wtn = RawWireCoord { crd, wire: wtn };
                    let wtf = self.pin_int_wire(wtn, wt);
                    match *tkw {
                        ConnectorWire::Reflect(wfw) => {
                            let wfn = &tn.wires_in_near[wfw];
                            let wfn = RawWireCoord { crd, wire: wfn };
                            let wff = self.pin_int_wire(wfn, wf);
                            pip_found = wtf && wff;
                            if pip_found {
                                self.claim_pip(wtn, wfn);
                            }
                        }
                        ConnectorWire::Pass(wfw) => match tn.wires_in_far[wfw] {
                            ConnectorWireInFarNaming::Simple { name: ref wfn } => {
                                let wfn = RawWireCoord { crd, wire: wfn };
                                let wff = self.pin_int_wire(wfn, wf);
                                pip_found = wtf && wff;
                                if pip_found {
                                    self.claim_pip(wtn, wfn);
                                }
                            }
                            ConnectorWireInFarNaming::Buf {
                                name_out: ref wfn,
                                name_in: ref wfin,
                            } => {
                                let wfn = RawWireCoord { crd, wire: wfn };
                                let wfin = RawWireCoord { crd, wire: wfin };
                                let wff = self.pin_int_wire(wfin, wf);
                                pip_found = wtf && wff;
                                if pip_found {
                                    self.claim_net(&[wfn]);
                                    self.claim_pip(wtn, wfn);
                                    self.claim_pip(wfn, wfin);
                                }
                            }
                            ConnectorWireInFarNaming::BufFar {
                                name: ref wfn,
                                name_far_out: ref wffon,
                                name_far_in: ref wffin,
                            } => {
                                let wfn = RawWireCoord { crd, wire: wfn };
                                let wffin = RawWireCoord {
                                    crd: crd_far.unwrap(),
                                    wire: wffin,
                                };
                                let wffon = RawWireCoord {
                                    crd: crd_far.unwrap(),
                                    wire: wffon,
                                };
                                let wff = self.pin_int_wire(wffin, wf);
                                pip_found = wtf && wff;
                                if pip_found {
                                    self.claim_net(&[wfn, wffon]);
                                    self.claim_pip(wffon, wffin);
                                    self.claim_pip(wtn, wfn);
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
                    let wtn = RawWireCoord { crd, wire: wtn };
                    let wfn = RawWireCoord { crd, wire: wfn };
                    let wtf = self.pin_int_wire(wtn, wt);
                    let wff = self.pin_int_wire(wfn, wf);
                    pip_found = wtf && wff;
                    if pip_found {
                        self.claim_pip(wtn, wfn);
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
        bel: &LegacyBelContext<'_>,
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
                pin: k.into(),
                wire: Some(&n.name),
            });
        }
        for (pin, dir) in extras.iter().copied() {
            if dummies.contains(&pin) {
                pins.push(SitePin {
                    dir,
                    pin: pin.into(),
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
                    pin: pin.into(),
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

    pub fn bel_rcrd(&self, bcrd: BelCoord) -> Coord {
        self.bel_rcrd_sub(bcrd, 0)
    }

    pub fn bel_rcrd_sub(&self, bcrd: BelCoord, sub: usize) -> Coord {
        let naming = self.ngrid.get_bel_naming(bcrd);
        let tcrd = self.grid.get_tile_by_bel(bcrd);
        let crds = self.get_node_crds(tcrd).unwrap();
        crds[naming.tiles[sub]]
    }

    pub fn bel_wire(&self, bcrd: BelCoord, name: &str) -> RawWireCoord<'a> {
        let naming = self.ngrid.get_bel_naming(bcrd);
        RawWireCoord {
            crd: self.bel_rcrd(bcrd),
            wire: &naming.pins[name].name,
        }
    }

    pub fn bel_wire_far(&self, bcrd: BelCoord, name: &str) -> RawWireCoord<'a> {
        let naming = self.ngrid.get_bel_naming(bcrd);
        RawWireCoord {
            crd: self.bel_rcrd(bcrd),
            wire: &naming.pins[name].name_far,
        }
    }

    pub fn verify_bel<'b>(&'b mut self, bcrd: BelCoord) -> BelVerifier<'a, 'b> {
        let kind = if let BelKind::Class(bcid) = self.db.bel_slots[bcrd.slot].kind {
            self.db.bel_classes.key(bcid)
        } else {
            self.db.bel_slots.key(bcrd.slot)
        }
        .to_string();
        let tcrd = self.grid.get_tile_by_bel(bcrd);
        let ntile = &self.ngrid.tiles[&tcrd];
        let crds = self.get_node_crds(tcrd).unwrap();
        let naming = self.ngrid.get_bel_naming(bcrd);
        BelVerifier {
            vrf: self,
            naming,
            bcrd,
            kind,
            extra_ins: vec![],
            extra_outs: vec![],
            ntile,
            crds,
            bidir_dirs: Default::default(),
            skip_ins: Default::default(),
            skip_outs: Default::default(),
            rename_ins: Default::default(),
            rename_outs: Default::default(),
            sub: 0,
        }
    }

    pub fn verify_legacy_bel(
        &mut self,
        bel: &LegacyBelContext<'_>,
        kind: &str,
        extras: &[(&str, SitePinDir)],
        skip: &[&str],
    ) {
        self.verify_bel_dummies(bel, kind, extras, skip, &[]);
    }

    pub fn get_legacy_bel(&self, bel: BelCoord) -> LegacyBelContext<'a> {
        self.find_bel(bel)
            .unwrap_or_else(|| panic!("{}", bel.to_string(self.db)))
    }

    pub fn find_bel(&self, bel: BelCoord) -> Option<LegacyBelContext<'a>> {
        let tcrd = self.grid.find_tile_by_bel(bel)?;
        let tile = &self.grid[tcrd];
        let crds = self.get_node_crds(tcrd).unwrap();
        let nk = &self.db[tile.class];
        let ntile = &self.ngrid.tiles[&tcrd];
        let name = self.ngrid.get_bel_name(bel);
        let BelInfo::Legacy(info) = &nk.bels[bel.slot] else {
            unreachable!()
        };
        let naming = self.ngrid.get_bel_naming(bel);
        Some(LegacyBelContext {
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
            name,
        })
    }

    pub fn find_bel_delta(
        &self,
        bel: &LegacyBelContext<'_>,
        dx: isize,
        dy: isize,
        slot: BelSlotId,
    ) -> Option<LegacyBelContext<'a>> {
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
        bel: &LegacyBelContext<'_>,
        dx: isize,
        dy: isize,
        slot: BelSlotId,
    ) -> Option<LegacyBelContext<'a>> {
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
    pub fn find_bel_sibling(
        &self,
        bel: &LegacyBelContext<'_>,
        slot: BelSlotId,
    ) -> LegacyBelContext<'a> {
        self.get_legacy_bel(bel.cell.bel(slot))
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

    pub fn skip_sb(&mut self, slot: BelSlotId) {
        self.skip_sb.insert(slot);
    }

    pub fn finish(mut self) {
        let mut cond_stub_outs = HashMap::new();
        let mut cond_stub_ins = HashMap::new();
        for (&crd, tile) in &self.rd.tiles {
            let tk = &self.rd.tile_kinds[tile.kind];
            for &w in tk.wires.keys() {
                if self.stub_outs.contains(&w) || self.stub_ins.contains(&w) {
                    self.claim_net(&[RawWireCoord {
                        crd,
                        wire: &self.rd.wires[w],
                    }]);
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
            self.claim_raw_node(
                nw,
                RawWireCoord {
                    crd,
                    wire: &self.rd.wires[w],
                },
            );
        }
        for (&nw, &(crd, w)) in &cond_stub_ins {
            self.claim_raw_node(
                nw,
                RawWireCoord {
                    crd,
                    wire: &self.rd.wires[w],
                },
            );
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
                    self.claim_pip_tri(crd, &self.rd.wires[wt], &self.rd.wires[wf]);
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
    bel_handler: impl Fn(&mut Verifier, BelCoord),
    legacy_bel_handler: impl Fn(&mut Verifier, &LegacyBelContext<'_>),
    extra: impl FnOnce(&mut Verifier),
) {
    let mut vrf = Verifier::new(rd, grid);
    extra_pre(&mut vrf);
    vrf.prep_int_wires();
    vrf.handle_int();
    for (tcrd, tile) in grid.egrid.tiles() {
        let tcls = &grid.egrid.db[tile.class];
        for (slot, bel) in &tcls.bels {
            match bel {
                BelInfo::Bel(_) => {
                    bel_handler(&mut vrf, tcrd.bel(slot));
                }
                BelInfo::Legacy(_) => {
                    let ctx = vrf.get_legacy_bel(tcrd.bel(slot));
                    legacy_bel_handler(&mut vrf, &ctx);
                }
                _ => (),
            }
        }
    }
    extra(&mut vrf);
    vrf.finish();
}
