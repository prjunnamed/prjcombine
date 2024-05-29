#![allow(clippy::too_many_arguments)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use prjcombine_int::db::{
    BelInfo, BelNaming, BelPin, BelPinNaming, Dir, IntDb, IntfInfo, IntfWireInNaming,
    IntfWireOutNaming, IriNaming, IriPin, MuxInfo, MuxKind, NodeExtPipNaming, NodeIriId, NodeKind,
    NodeKindId, NodeNaming, NodeNamingId, NodeRawTileId, NodeTileId, NodeWireId, PinDir, TermInfo,
    TermKind, TermNamingId, TermWireInFarNaming, TermWireOutNaming, WireId, WireKind,
};
use prjcombine_rawdump::{self as rawdump, Coord, NodeOrWire, Part};
use unnamed_entity::{EntityId, EntityMap, EntityPartVec, EntityVec};

use assert_matches::assert_matches;

use enum_map::EnumMap;
use rawdump::TileKindId;

#[derive(Clone, Debug)]
pub struct ExtrBelInfo {
    pub name: String,
    pub slot: Option<rawdump::TkSiteSlot>,
    pub pins: HashMap<String, BelPinInfo>,
    pub raw_tile: usize,
}

#[derive(Clone, Debug)]
pub enum BelPinInfo {
    Int,
    NameOnly(usize),
    ForceInt(NodeWireId, String),
    ExtraInt(PinDir, Vec<String>),
    ExtraIntForce(PinDir, NodeWireId, String),
    ExtraWire(Vec<String>),
    ExtraWireForce(String, Vec<NodeExtPipNaming>),
    Dummy,
}

#[derive(Debug)]
pub struct XNodeRawTile {
    pub xy: Coord,
    pub tile_map: Option<EntityPartVec<NodeTileId, NodeTileId>>,
    pub extract_muxes: bool,
}

#[derive(Debug)]
pub struct XNodeRef {
    pub xy: Coord,
    pub naming: Option<NodeNamingId>,
    pub tile_map: EntityPartVec<NodeTileId, NodeTileId>,
}

pub struct XNodeInfo<'a, 'b> {
    pub builder: &'b mut IntBuilder<'a>,
    pub kind: String,
    pub naming: String,
    pub raw_tiles: Vec<XNodeRawTile>,
    pub num_tiles: usize,
    pub refs: Vec<XNodeRef>,
    pub extract_intfs: bool,
    pub has_intf_out_bufs: bool,
    pub iris: EntityVec<NodeIriId, rawdump::TkSiteSlot>,
    pub skip_muxes: BTreeSet<WireId>,
    pub optin_muxes: BTreeSet<WireId>,
    pub optin_muxes_tile: BTreeSet<NodeWireId>,
    pub bels: Vec<ExtrBelInfo>,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum IntConnKind {
    Raw,
    IntfIn,
    IntfOut,
}

impl ExtrBelInfo {
    pub fn pins_name_only(mut self, names: &[impl AsRef<str>]) -> Self {
        for name in names {
            self.pins
                .insert(name.as_ref().to_string(), BelPinInfo::NameOnly(0));
        }
        self
    }

    pub fn pin_name_only(mut self, name: &str, buf_cnt: usize) -> Self {
        self.pins
            .insert(name.to_string(), BelPinInfo::NameOnly(buf_cnt));
        self
    }

    pub fn pin_dummy(mut self, name: impl Into<String>) -> Self {
        self.pins.insert(name.into(), BelPinInfo::Dummy);
        self
    }

    pub fn pin_force_int(mut self, name: &str, wire: NodeWireId, wname: impl Into<String>) -> Self {
        self.pins
            .insert(name.to_string(), BelPinInfo::ForceInt(wire, wname.into()));
        self
    }

    pub fn extra_int_out(
        mut self,
        name: impl Into<String>,
        wire_names: &[impl AsRef<str>],
    ) -> Self {
        self.pins.insert(
            name.into(),
            BelPinInfo::ExtraInt(
                PinDir::Output,
                wire_names.iter().map(|x| x.as_ref().to_string()).collect(),
            ),
        );
        self
    }

    pub fn extra_int_in(mut self, name: impl Into<String>, wire_names: &[impl AsRef<str>]) -> Self {
        self.pins.insert(
            name.into(),
            BelPinInfo::ExtraInt(
                PinDir::Input,
                wire_names.iter().map(|x| x.as_ref().to_string()).collect(),
            ),
        );
        self
    }

    pub fn extra_int_inout(
        mut self,
        name: impl Into<String>,
        wire_names: &[impl AsRef<str>],
    ) -> Self {
        self.pins.insert(
            name.into(),
            BelPinInfo::ExtraInt(
                PinDir::Inout,
                wire_names.iter().map(|x| x.as_ref().to_string()).collect(),
            ),
        );
        self
    }
    pub fn extra_int_out_force(
        mut self,
        name: impl Into<String>,
        wire: NodeWireId,
        wire_name: impl Into<String>,
    ) -> Self {
        self.pins.insert(
            name.into(),
            BelPinInfo::ExtraIntForce(PinDir::Output, wire, wire_name.into()),
        );
        self
    }

    pub fn extra_int_in_force(
        mut self,
        name: impl Into<String>,
        wire: NodeWireId,
        wire_name: impl Into<String>,
    ) -> Self {
        self.pins.insert(
            name.into(),
            BelPinInfo::ExtraIntForce(PinDir::Input, wire, wire_name.into()),
        );
        self
    }

    pub fn extra_wire(mut self, name: impl Into<String>, wire_names: &[impl AsRef<str>]) -> Self {
        self.pins.insert(
            name.into(),
            BelPinInfo::ExtraWire(wire_names.iter().map(|x| x.as_ref().to_string()).collect()),
        );
        self
    }

    pub fn extra_wire_force(
        mut self,
        name: impl Into<String>,
        wire_name: impl Into<String>,
    ) -> Self {
        self.pins.insert(
            name.into(),
            BelPinInfo::ExtraWireForce(wire_name.into(), vec![]),
        );
        self
    }

    pub fn extra_wire_force_pip(
        mut self,
        name: impl Into<String>,
        wire_name: impl Into<String>,
        pip: NodeExtPipNaming,
    ) -> Self {
        self.pins.insert(
            name.into(),
            BelPinInfo::ExtraWireForce(wire_name.into(), vec![pip]),
        );
        self
    }

    pub fn raw_tile(mut self, idx: usize) -> Self {
        self.raw_tile = idx;
        self
    }
}

impl XNodeInfo<'_, '_> {
    pub fn raw_tile(mut self, xy: Coord) -> Self {
        self.raw_tiles.push(XNodeRawTile {
            xy,
            tile_map: None,
            extract_muxes: false,
        });
        self
    }

    pub fn raw_tile_single(mut self, xy: Coord, slot: usize) -> Self {
        self.raw_tiles.push(XNodeRawTile {
            xy,
            tile_map: Some(
                [(NodeTileId::from_idx(0), NodeTileId::from_idx(slot))]
                    .into_iter()
                    .collect(),
            ),
            extract_muxes: false,
        });
        self
    }

    pub fn ref_int(mut self, xy: Coord, slot: usize) -> Self {
        self.refs.push(XNodeRef {
            xy,
            naming: None,
            tile_map: [(NodeTileId::from_idx(0), NodeTileId::from_idx(slot))]
                .into_iter()
                .collect(),
        });
        self
    }

    pub fn ref_single(mut self, xy: Coord, slot: usize, naming: NodeNamingId) -> Self {
        self.refs.push(XNodeRef {
            xy,
            naming: Some(naming),
            tile_map: [(NodeTileId::from_idx(0), NodeTileId::from_idx(slot))]
                .into_iter()
                .collect(),
        });
        self
    }

    pub fn ref_xlat(mut self, xy: Coord, slots: &[Option<usize>], naming: NodeNamingId) -> Self {
        self.refs.push(XNodeRef {
            xy,
            naming: Some(naming),
            tile_map: slots
                .iter()
                .enumerate()
                .filter_map(|(i, x)| x.map(|x| (NodeTileId::from_idx(i), NodeTileId::from_idx(x))))
                .collect(),
        });
        self
    }

    pub fn extract_muxes(mut self) -> Self {
        self.raw_tiles[0].extract_muxes = true;
        self
    }

    pub fn extract_muxes_rt(mut self, rt: usize) -> Self {
        self.raw_tiles[rt].extract_muxes = true;
        self
    }

    pub fn extract_intfs(mut self, has_out_bufs: bool) -> Self {
        self.extract_intfs = true;
        self.has_intf_out_bufs = has_out_bufs;
        self
    }

    pub fn iris(mut self, iris: &[(&str, u8, u8)]) -> Self {
        assert!(self.iris.is_empty());
        for &(sn, sx, sy) in iris {
            self.iris.push(rawdump::TkSiteSlot::Xy(
                self.builder.rd.slot_kinds.get(sn).unwrap(),
                sx,
                sy,
            ));
        }
        self
    }

    pub fn bel(mut self, bel: ExtrBelInfo) -> Self {
        self.bels.push(bel);
        self
    }

    pub fn bels(mut self, bels: impl IntoIterator<Item = ExtrBelInfo>) -> Self {
        for bel in bels {
            self.bels.push(bel);
        }
        self
    }

    pub fn skip_muxes<'a>(mut self, wires: impl IntoIterator<Item = &'a WireId>) -> Self {
        self.skip_muxes.extend(wires.into_iter().copied());
        self
    }

    pub fn optin_muxes<'a>(mut self, wires: impl IntoIterator<Item = &'a WireId>) -> Self {
        self.optin_muxes.extend(wires.into_iter().copied());
        self
    }

    pub fn optin_muxes_tile<'a>(mut self, wires: impl IntoIterator<Item = &'a NodeWireId>) -> Self {
        self.optin_muxes_tile.extend(wires.into_iter().copied());
        self
    }

    pub fn num_tiles(mut self, num: usize) -> Self {
        self.num_tiles = num;
        self
    }

    pub fn extract(self) {
        let rd = self.builder.rd;

        let mut names: HashMap<NodeOrWire, (IntConnKind, NodeWireId)> = HashMap::new();

        let mut edges_in: HashMap<_, Vec<_>> = HashMap::new();
        let mut edges_out: HashMap<_, Vec<_>> = HashMap::new();

        for (i, rt) in self.raw_tiles.iter().enumerate() {
            let tile = &rd.tiles[&rt.xy];
            let tk = &rd.tile_kinds[tile.kind];
            for &wi in tk.wires.keys() {
                let nw = rd.lookup_wire_raw_force(rt.xy, wi);
                if let Some(w) = self.builder.get_wire_by_name(tile.kind, &rd.wires[wi]) {
                    let mut w = w;
                    if let Some(ref tile_map) = rt.tile_map {
                        w.0 = tile_map[w.0];
                    } else if self.num_tiles == 1 {
                        w.0 = NodeTileId::from_idx(0);
                    }
                    names.entry(nw).or_insert((IntConnKind::Raw, w));
                }
            }
            for &(wfi, wti) in tk.pips.keys() {
                let nwf = rd.lookup_wire_raw_force(rt.xy, wfi);
                let nwt = rd.lookup_wire_raw_force(rt.xy, wti);
                edges_in.entry(nwt).or_default().push((nwf, i, wti, wfi));
                edges_out.entry(nwf).or_default().push((nwt, i, wti, wfi));
            }
        }

        for round in [0, 1] {
            for r in &self.refs {
                let tile = &rd.tiles[&r.xy];
                let tk = &rd.tile_kinds[tile.kind];

                let naming = if let Some(n) = r.naming {
                    n
                } else if let Some(n) = self.builder.get_int_naming(r.xy) {
                    n
                } else {
                    continue;
                };
                let naming = &self.builder.db.node_namings[naming];
                for (&k, v) in &naming.wires {
                    if round == 0
                        && matches!(
                            self.builder.db.wires[k.1],
                            WireKind::Branch(_) | WireKind::MultiBranch(_) | WireKind::PipBranch(_)
                        )
                    {
                        continue;
                    }
                    if let Some(nw) = rd.lookup_wire(r.xy, v) {
                        if let Some(&ti) = r.tile_map.get(k.0) {
                            names.entry(nw).or_insert((IntConnKind::Raw, (ti, k.1)));
                        }
                    }
                }
                for (&k, v) in &naming.intf_wires_in {
                    match v {
                        IntfWireInNaming::Simple { name: n }
                        | IntfWireInNaming::TestBuf { name_in: n, .. } => {
                            if let Some(nw) = rd.lookup_wire(r.xy, n) {
                                names
                                    .entry(nw)
                                    .or_insert((IntConnKind::Raw, (r.tile_map[k.0], k.1)));
                            }
                        }
                        IntfWireInNaming::Buf { name_out: n, .. }
                        | IntfWireInNaming::Delay { name_out: n, .. }
                        | IntfWireInNaming::Iri { name_out: n, .. }
                        | IntfWireInNaming::IriDelay { name_out: n, .. } => {
                            if let Some(nw) = rd.lookup_wire(r.xy, n) {
                                names
                                    .entry(nw)
                                    .or_insert((IntConnKind::IntfIn, (r.tile_map[k.0], k.1)));
                            }
                        }
                    }
                }
                for (&k, v) in &naming.intf_wires_out {
                    match v {
                        IntfWireOutNaming::Simple { name } => {
                            if let Some(nw) = rd.lookup_wire(r.xy, name) {
                                names
                                    .entry(nw)
                                    .or_insert((IntConnKind::Raw, (r.tile_map[k.0], k.1)));
                            }
                        }
                        IntfWireOutNaming::Buf { name_out, name_in } => {
                            if let Some(nw) = rd.lookup_wire(r.xy, name_out) {
                                names
                                    .entry(nw)
                                    .or_insert((IntConnKind::Raw, (r.tile_map[k.0], k.1)));
                            }
                            if let Some(nw) = rd.lookup_wire(r.xy, name_in) {
                                names
                                    .entry(nw)
                                    .or_insert((IntConnKind::IntfOut, (r.tile_map[k.0], k.1)));
                            }
                        }
                    }
                }

                for &wi in tk.wires.keys() {
                    if let Some(nw) = rd.lookup_wire_raw(r.xy, wi) {
                        if let Some(w) = self.builder.get_wire_by_name(tile.kind, &rd.wires[wi]) {
                            if round == 0
                                && matches!(
                                    self.builder.db.wires[w.1],
                                    WireKind::Branch(_)
                                        | WireKind::MultiBranch(_)
                                        | WireKind::PipBranch(_)
                                )
                            {
                                continue;
                            }
                            if let Some(&t) = r.tile_map.get(w.0) {
                                names.entry(nw).or_insert((IntConnKind::Raw, (t, w.1)));
                                continue;
                            }
                        }
                    }
                }
            }
        }

        let buf_out: HashMap<_, _> = edges_out
            .iter()
            .filter_map(|(&wt, wfs)| {
                if wfs.len() == 1 {
                    Some((wt, wfs[0]))
                } else {
                    None
                }
            })
            .collect();

        let int_out: HashMap<_, _> = edges_out
            .iter()
            .filter_map(|(&wt, wfs)| {
                let filtered: Vec<_> = wfs
                    .iter()
                    .copied()
                    .filter_map(|(x, t, wt, wf)| {
                        if let Some(&(ick, w)) = names.get(&x) {
                            Some((ick, w, t, wt, wf))
                        } else {
                            None
                        }
                    })
                    .collect();
                if !filtered.is_empty() {
                    Some((wt, filtered))
                } else {
                    None
                }
            })
            .collect();

        let buf_in: HashMap<_, _> = edges_in
            .iter()
            .filter_map(|(&wt, wfs)| {
                if wfs.len() == 1 {
                    Some((wt, wfs[0]))
                } else {
                    None
                }
            })
            .collect();

        let int_in: HashMap<_, _> = edges_in
            .iter()
            .filter_map(|(&wt, wfs)| {
                let filtered: Vec<_> = wfs
                    .iter()
                    .copied()
                    .filter_map(|(x, t, wt, wf)| {
                        if let Some(&(ick, w)) = names.get(&x) {
                            Some((ick, w, t, wt, wf))
                        } else {
                            None
                        }
                    })
                    .collect();
                if filtered.len() == 1 {
                    Some((wt, filtered[0]))
                } else {
                    None
                }
            })
            .collect();

        let mut extractor = XNodeExtractor {
            rd: self.builder.rd,
            db: &self.builder.db,
            xnode: &self,
            names,
            buf_out,
            buf_in,
            int_out,
            int_in,
            node: NodeKind {
                tiles: (0..self.num_tiles).map(|_| ()).collect(),
                muxes: Default::default(),
                bels: Default::default(),
                iris: Default::default(),
                intfs: Default::default(),
            },
            node_naming: NodeNaming::default(),
        };

        if self.raw_tiles.iter().any(|x| x.extract_muxes)
            || !self.optin_muxes.is_empty()
            || !self.optin_muxes_tile.is_empty()
        {
            extractor.extract_muxes();
        }

        if self.extract_intfs {
            extractor.extract_intfs();
        }

        for bel in &self.bels {
            extractor.extract_bel(bel);
        }

        let node = extractor.node;
        let node_naming = extractor.node_naming;

        self.builder.insert_node_merge(&self.kind, node);
        self.builder.insert_node_naming(&self.naming, node_naming);
    }
}

#[allow(clippy::type_complexity)]
struct XNodeExtractor<'a, 'b, 'c> {
    rd: &'c Part,
    db: &'c IntDb,
    xnode: &'a XNodeInfo<'b, 'c>,
    names: HashMap<NodeOrWire, (IntConnKind, NodeWireId)>,
    buf_out: HashMap<NodeOrWire, (NodeOrWire, usize, rawdump::WireId, rawdump::WireId)>,
    buf_in: HashMap<NodeOrWire, (NodeOrWire, usize, rawdump::WireId, rawdump::WireId)>,
    int_out: HashMap<
        NodeOrWire,
        Vec<(
            IntConnKind,
            NodeWireId,
            usize,
            rawdump::WireId,
            rawdump::WireId,
        )>,
    >,
    int_in: HashMap<
        NodeOrWire,
        (
            IntConnKind,
            NodeWireId,
            usize,
            rawdump::WireId,
            rawdump::WireId,
        ),
    >,
    node: NodeKind,
    node_naming: NodeNaming,
}

impl XNodeExtractor<'_, '_, '_> {
    fn walk_to_int(
        &self,
        pin: &str,
        dir: PinDir,
        tile: usize,
        wire: rawdump::WireId,
    ) -> (
        IntConnKind,
        BTreeSet<NodeWireId>,
        rawdump::WireId,
        Vec<NodeExtPipNaming>,
        BTreeMap<NodeWireId, NodeExtPipNaming>,
    ) {
        let mut wn = wire;
        let mut nw = self
            .rd
            .lookup_wire_raw_force(self.xnode.raw_tiles[tile].xy, wire);
        let mut pips = Vec::new();
        loop {
            if let Some(&(ick, w)) = self.names.get(&nw) {
                return (ick, [w].into_iter().collect(), wn, pips, BTreeMap::new());
            }
            match dir {
                PinDir::Input => {
                    if let Some(&(ick, w, rt, wt, wf)) = self.int_in.get(&nw) {
                        pips.push(NodeExtPipNaming {
                            tile: NodeRawTileId::from_idx(rt),
                            wire_to: self.rd.wires[wt].clone(),
                            wire_from: self.rd.wires[wf].clone(),
                        });
                        if rt == tile {
                            wn = wf;
                        }
                        return (ick, [w].into_iter().collect(), wn, pips, BTreeMap::new());
                    }
                    if let Some(&(nnw, rt, wt, wf)) = self.buf_in.get(&nw) {
                        pips.push(NodeExtPipNaming {
                            tile: NodeRawTileId::from_idx(rt),
                            wire_to: self.rd.wires[wt].clone(),
                            wire_from: self.rd.wires[wf].clone(),
                        });
                        if rt == tile {
                            wn = wf;
                        }
                        nw = nnw;
                        continue;
                    }
                    panic!(
                        "CANNOT WALK INPUT WIRE {} {} {}",
                        self.rd.part, self.xnode.kind, pin
                    );
                }
                PinDir::Output => {
                    if let Some(nxt) = self.int_out.get(&nw) {
                        if nxt.len() == 1 {
                            let (ick, w, rt, wt, wf) = nxt[0];
                            pips.push(NodeExtPipNaming {
                                tile: NodeRawTileId::from_idx(rt),
                                wire_to: self.rd.wires[wt].clone(),
                                wire_from: self.rd.wires[wf].clone(),
                            });
                            if rt == tile {
                                wn = wt;
                            }
                            return (ick, [w].into_iter().collect(), wn, pips, BTreeMap::new());
                        } else {
                            let mut wires = BTreeSet::new();
                            let mut int_pips = BTreeMap::new();
                            let mut ick = None;
                            for &(cick, w, rt, wt, wf) in nxt {
                                ick = Some(cick);
                                wires.insert(w);
                                int_pips.insert(
                                    w,
                                    NodeExtPipNaming {
                                        tile: NodeRawTileId::from_idx(rt),
                                        wire_to: self.rd.wires[wt].clone(),
                                        wire_from: self.rd.wires[wf].clone(),
                                    },
                                );
                            }
                            return (ick.unwrap(), wires, wn, pips, int_pips);
                        }
                    }
                    if let Some(&(nnw, rt, wt, wf)) = self.buf_out.get(&nw) {
                        pips.push(NodeExtPipNaming {
                            tile: NodeRawTileId::from_idx(rt),
                            wire_to: self.rd.wires[wt].clone(),
                            wire_from: self.rd.wires[wf].clone(),
                        });
                        if rt == tile {
                            wn = wt;
                        }
                        nw = nnw;
                        continue;
                    }
                    panic!(
                        "CANNOT WALK OUTPUT WIRE {} {} {}",
                        self.rd.part, self.xnode.kind, pin
                    );
                }
                PinDir::Inout => {
                    panic!(
                        "CANNOT WALK INOUT WIRE {} {} {}",
                        self.rd.part, self.xnode.kind, pin
                    );
                }
            }
        }
    }

    fn walk_count(
        &self,
        pin: &str,
        dir: PinDir,
        cnt: usize,
        tile: usize,
        wire: rawdump::WireId,
    ) -> (rawdump::WireId, Vec<NodeExtPipNaming>) {
        let mut wn = wire;
        let mut nw = self
            .rd
            .lookup_wire_raw_force(self.xnode.raw_tiles[tile].xy, wire);
        let mut pips = Vec::new();
        for _ in 0..cnt {
            match dir {
                PinDir::Input => {
                    if let Some(&(nnw, rt, wt, wf)) = self.buf_in.get(&nw) {
                        pips.push(NodeExtPipNaming {
                            tile: NodeRawTileId::from_idx(rt),
                            wire_to: self.rd.wires[wt].clone(),
                            wire_from: self.rd.wires[wf].clone(),
                        });
                        if rt == tile {
                            wn = wf;
                        }
                        nw = nnw;
                        continue;
                    }
                }
                PinDir::Output => {
                    if let Some(&(nnw, rt, wt, wf)) = self.buf_out.get(&nw) {
                        pips.push(NodeExtPipNaming {
                            tile: NodeRawTileId::from_idx(rt),
                            wire_to: self.rd.wires[wt].clone(),
                            wire_from: self.rd.wires[wf].clone(),
                        });
                        if rt == tile {
                            wn = wt;
                        }
                        nw = nnw;
                        continue;
                    }
                }
                PinDir::Inout => (),
            }
            panic!(
                "CANNOT WALK WIRE {} {} {}",
                self.rd.part, self.xnode.kind, pin
            );
        }
        (wn, pips)
    }

    fn extract_bel(&mut self, bel: &ExtrBelInfo) {
        let crd = self.xnode.raw_tiles[bel.raw_tile].xy;
        let tile = &self.rd.tiles[&crd];
        let tk = &self.rd.tile_kinds[tile.kind];
        let mut pins = BTreeMap::new();
        let mut naming_pins = BTreeMap::new();
        if let Some(slot) = bel.slot {
            let tks = tk.sites.get(&slot).expect("missing site slot in tk").1;
            for (name, tksp) in &tks.pins {
                match bel.pins.get(name).unwrap_or(&BelPinInfo::Int) {
                    &BelPinInfo::Int => {
                        let dir = match tksp.dir {
                            rawdump::TkSitePinDir::Input => PinDir::Input,
                            rawdump::TkSitePinDir::Output => PinDir::Output,
                            _ => panic!("bidir pin {name}"),
                        };
                        if tksp.wire.is_none() {
                            panic!(
                                "missing site wire for pin {name} tile {tile}",
                                tile = self.xnode.kind
                            );
                        }
                        let (ick, wires, wnf, pips, int_pips) =
                            self.walk_to_int(name, dir, bel.raw_tile, tksp.wire.unwrap());
                        naming_pins.insert(
                            name.clone(),
                            BelPinNaming {
                                name: self.rd.wires[tksp.wire.unwrap()].clone(),
                                name_far: self.rd.wires[wnf].clone(),
                                pips,
                                int_pips,
                                is_intf_out: ick == IntConnKind::IntfOut,
                            },
                        );
                        pins.insert(
                            name.clone(),
                            BelPin {
                                wires,
                                dir,
                                is_intf_in: ick == IntConnKind::IntfIn,
                            },
                        );
                    }
                    &BelPinInfo::ForceInt(wire, ref wname) => {
                        let dir = match tksp.dir {
                            rawdump::TkSitePinDir::Input => PinDir::Input,
                            rawdump::TkSitePinDir::Output => PinDir::Output,
                            _ => panic!("bidir pin {name}"),
                        };
                        naming_pins.insert(
                            name.clone(),
                            BelPinNaming {
                                name: self.rd.wires[tksp.wire.unwrap()].clone(),
                                name_far: wname.clone(),
                                pips: Vec::new(),
                                int_pips: BTreeMap::new(),
                                is_intf_out: false,
                            },
                        );
                        pins.insert(
                            name.clone(),
                            BelPin {
                                wires: [wire].into_iter().collect(),
                                dir,
                                is_intf_in: false,
                            },
                        );
                    }
                    &BelPinInfo::NameOnly(buf_cnt) => {
                        if tksp.wire.is_none() {
                            panic!(
                                "missing site wire for pin {name} tile {tile}",
                                tile = self.xnode.kind
                            );
                        }
                        if buf_cnt == 0 {
                            naming_pins.insert(
                                name.clone(),
                                BelPinNaming {
                                    name: self.rd.wires[tksp.wire.unwrap()].clone(),
                                    name_far: self.rd.wires[tksp.wire.unwrap()].clone(),
                                    pips: Vec::new(),
                                    int_pips: BTreeMap::new(),
                                    is_intf_out: false,
                                },
                            );
                        } else {
                            let dir = match tksp.dir {
                                rawdump::TkSitePinDir::Input => PinDir::Input,
                                rawdump::TkSitePinDir::Output => PinDir::Output,
                                _ => panic!("bidir pin {name}"),
                            };
                            let (wn, pips) = self.walk_count(
                                name,
                                dir,
                                buf_cnt,
                                bel.raw_tile,
                                tksp.wire.unwrap(),
                            );
                            naming_pins.insert(
                                name.clone(),
                                BelPinNaming {
                                    name: self.rd.wires[tksp.wire.unwrap()].clone(),
                                    name_far: self.rd.wires[wn].clone(),
                                    pips,
                                    int_pips: BTreeMap::new(),
                                    is_intf_out: false,
                                },
                            );
                        }
                    }
                    BelPinInfo::Dummy => (),
                    BelPinInfo::ExtraWireForce(_, _) => (),
                    BelPinInfo::ExtraInt(_, _) => (),
                    BelPinInfo::ExtraWire(_) => (),
                    _ => unreachable!(),
                }
            }
        }
        for (name, pd) in &bel.pins {
            match *pd {
                BelPinInfo::ExtraInt(dir, ref names) => {
                    let mut wn = None;
                    for w in names {
                        if let Some(w) = self.rd.wires.get(w) {
                            if tk.wires.contains_key(&w) {
                                assert!(wn.is_none());
                                wn = Some(w);
                            }
                        }
                    }
                    if wn.is_none() {
                        println!("NOT FOUND: {name}");
                    }
                    let wn = wn.unwrap();
                    let (ick, wires, wnf, pips, int_pips) =
                        self.walk_to_int(name, dir, bel.raw_tile, wn);
                    naming_pins.insert(
                        name.clone(),
                        BelPinNaming {
                            name: self.rd.wires[wn].clone(),
                            name_far: self.rd.wires[wnf].clone(),
                            pips,
                            int_pips,
                            is_intf_out: ick == IntConnKind::IntfOut,
                        },
                    );
                    pins.insert(
                        name.clone(),
                        BelPin {
                            wires,
                            dir,
                            is_intf_in: ick == IntConnKind::IntfIn,
                        },
                    );
                }
                BelPinInfo::ExtraIntForce(dir, wire, ref wname) => {
                    naming_pins.insert(
                        name.clone(),
                        BelPinNaming {
                            name: wname.clone(),
                            name_far: wname.clone(),
                            pips: vec![],
                            int_pips: BTreeMap::new(),
                            is_intf_out: false,
                        },
                    );
                    pins.insert(
                        name.clone(),
                        BelPin {
                            wires: [wire].into_iter().collect(),
                            dir,
                            is_intf_in: false,
                        },
                    );
                }
                BelPinInfo::ExtraWire(ref names) => {
                    let mut wn = None;
                    for w in names {
                        if let Some(w) = self.rd.wires.get(w) {
                            if tk.wires.contains_key(&w) {
                                if let Some(wn) = wn {
                                    println!(
                                        "COLLISION {wn} {w}",
                                        wn = self.rd.wires[wn],
                                        w = self.rd.wires[w]
                                    );
                                }
                                assert!(wn.is_none());
                                wn = Some(w);
                            }
                        }
                    }
                    if wn.is_none() {
                        println!("NOT FOUND: {name}");
                    }
                    let wn = wn.unwrap();
                    naming_pins.insert(
                        name.clone(),
                        BelPinNaming {
                            name: self.rd.wires[wn].clone(),
                            name_far: self.rd.wires[wn].clone(),
                            pips: Vec::new(),
                            int_pips: BTreeMap::new(),
                            is_intf_out: false,
                        },
                    );
                }
                BelPinInfo::ExtraWireForce(ref wname, ref pips) => {
                    naming_pins.insert(
                        name.clone(),
                        BelPinNaming {
                            name: wname.clone(),
                            name_far: wname.clone(),
                            pips: pips.clone(),
                            int_pips: BTreeMap::new(),
                            is_intf_out: false,
                        },
                    );
                }
                _ => (),
            }
        }
        self.node.bels.insert(bel.name.clone(), BelInfo { pins });
        self.node_naming.bels.push(BelNaming {
            tile: NodeRawTileId::from_idx(bel.raw_tile),
            pins: naming_pins,
        });
    }

    fn get_wire_by_name(&self, rti: usize, name: rawdump::WireId) -> Option<NodeWireId> {
        let rt = &self.xnode.raw_tiles[rti];
        let tile = &self.rd.tiles[&rt.xy];
        if let Some((t, w)) = self
            .xnode
            .builder
            .get_wire_by_name(tile.kind, &self.rd.wires[name])
        {
            if let Some(&xt) = rt.tile_map.as_ref().and_then(|x| x.get(t)) {
                return Some((xt, w));
            }
        }
        let nw = self.rd.lookup_wire_raw_force(rt.xy, name);
        if let Some(&(_, w)) = self.names.get(&nw) {
            return Some(w);
        }
        None
    }

    fn extract_muxes(&mut self) {
        for (i, rt) in self.xnode.raw_tiles.iter().enumerate() {
            let tile = &self.rd.tiles[&rt.xy];
            let tk = &self.rd.tile_kinds[tile.kind];

            for &(wfi, wti) in tk.pips.keys() {
                if let Some(wt) = self.get_wire_by_name(i, wti) {
                    let mut pass = rt.extract_muxes
                        && !matches!(self.db.wires[wt.1], WireKind::LogicOut)
                        && !self.xnode.skip_muxes.contains(&wt.1);
                    if self.xnode.optin_muxes.contains(&wt.1) {
                        pass = true;
                    }
                    if self.xnode.optin_muxes_tile.contains(&wt) {
                        pass = true;
                    }
                    if !pass {
                        continue;
                    }
                    if let Some(wf) = self.get_wire_by_name(i, wfi) {
                        if i == 0 {
                            self.node_naming
                                .wires
                                .insert(wt, self.rd.wires[wti].to_string());
                            self.node_naming
                                .wires
                                .insert(wf, self.rd.wires[wfi].to_string());
                        } else {
                            self.node_naming.ext_pips.insert(
                                (wt, wf),
                                NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(i),
                                    wire_to: self.rd.wires[wti].to_string(),
                                    wire_from: self.rd.wires[wfi].to_string(),
                                },
                            );
                        }
                        if let WireKind::Buf(dwf) = self.db.wires[wt.1] {
                            assert_eq!(wf.1, dwf);
                            assert_eq!(wt.0, NodeTileId::from_idx(0));
                            assert_eq!(wf.0, NodeTileId::from_idx(0));
                            continue;
                        }
                        // XXX
                        let kind = MuxKind::Plain;
                        let mux = self.node.muxes.entry(wt).or_insert(MuxInfo {
                            kind,
                            ins: Default::default(),
                        });
                        assert_eq!(mux.kind, kind);
                        mux.ins.insert(wf);
                    } else if self.xnode.builder.stub_outs.contains(&self.rd.wires[wfi]) {
                        // ignore
                    } else {
                        println!(
                            "UNEXPECTED XNODE MUX IN {} {} {} {}",
                            self.rd.tile_kinds.key(tile.kind),
                            tile.name,
                            self.rd.wires[wti],
                            self.rd.wires[wfi]
                        );
                    }
                }
            }
        }
    }

    fn extract_intfs(&mut self) {
        let crd = self.xnode.raw_tiles[0].xy;
        let tile = &self.rd.tiles[&crd];
        let tk = &self.rd.tile_kinds[tile.kind];
        let mut proxied_in = HashMap::new();
        for (iri, slot) in &self.xnode.iris {
            let (_, site) = tk.sites.get(slot).unwrap();
            self.node.iris.push(());
            self.node_naming.iris.push(IriNaming {
                tile: NodeRawTileId::from_idx(0),
                kind: site.kind.clone(),
            });
            let mut pin_ins = HashMap::new();
            let mut pin_outs = HashMap::new();
            for (pname, spin) in &site.pins {
                let (pin, dir) = match &**pname {
                    "CLK" => (IriPin::Clk, PinDir::Input),
                    "CLK_O" => (IriPin::Clk, PinDir::Output),
                    "RST" => (IriPin::Rst, PinDir::Input),
                    "RST_O" => (IriPin::Rst, PinDir::Output),
                    _ => {
                        if let Some(pn) = pname.strip_prefix("CE") {
                            if let Some(pn) = pn.strip_suffix("_O") {
                                (IriPin::Ce(pn.parse().unwrap()), PinDir::Output)
                            } else {
                                (IriPin::Ce(pn.parse().unwrap()), PinDir::Input)
                            }
                        } else if let Some(pn) = pname.strip_prefix("IMUX_IN") {
                            (IriPin::Imux(pn.parse().unwrap()), PinDir::Input)
                        } else if let Some(pn) = pname.strip_prefix("IMUX_O") {
                            (IriPin::Imux(pn.parse().unwrap()), PinDir::Output)
                        } else {
                            unreachable!()
                        }
                    }
                };
                match dir {
                    PinDir::Input => {
                        pin_ins.insert(pin, spin.wire.unwrap());
                    }
                    PinDir::Output => {
                        pin_outs.insert(pin, spin.wire.unwrap());
                    }
                    PinDir::Inout => unreachable!(),
                }
            }
            assert_eq!(pin_ins.len(), pin_outs.len());
            for (pin, wire_pin_in) in pin_ins {
                let wire_pin_out = pin_outs[&pin];
                let npi = self.rd.lookup_wire_raw_force(crd, wire_pin_in);
                let npo = self.rd.lookup_wire_raw_force(crd, wire_pin_out);
                let &(_, rt, wire_out, wpo) = self.buf_out.get(&npo).unwrap();
                assert_eq!(rt, 0);
                assert_eq!(wpo, wire_pin_out);
                let &(_, rt, wpi, wire_in) = self.buf_in.get(&npi).unwrap();
                assert_eq!(rt, 0);
                assert_eq!(wpi, wire_pin_in);
                let ni = self.rd.lookup_wire_raw_force(crd, wire_in);
                if let Some(&(_, wf)) = self.names.get(&ni) {
                    proxied_in.insert(wire_out, wf);
                    self.node.intfs.insert(wf, IntfInfo::InputIri(iri, pin));
                    self.node_naming.intf_wires_in.insert(
                        wf,
                        IntfWireInNaming::Iri {
                            name_out: self.rd.wires[wire_out].to_string(),
                            name_pin_out: self.rd.wires[wire_pin_out].to_string(),
                            name_pin_in: self.rd.wires[wire_pin_in].to_string(),
                            name_in: self.rd.wires[wire_in].to_string(),
                        },
                    );
                } else {
                    println!(
                        "MISSING IRI {iri} {pin:?} {wo} {wpo} {wpi} {wi}",
                        iri = iri.to_idx(),
                        wo = self.rd.wires[wire_out],
                        wpo = self.rd.wires[wire_pin_out],
                        wpi = self.rd.wires[wire_pin_in],
                        wi = self.rd.wires[wire_in],
                    );
                }
            }
        }
        if self.xnode.has_intf_out_bufs {
            for &(wfi, wdi) in tk.pips.keys() {
                let nwf = self.rd.lookup_wire_raw_force(crd, wfi);
                let nwd = self.rd.lookup_wire_raw_force(crd, wdi);
                if !self.buf_in.contains_key(&nwd) {
                    continue;
                }
                let Some(&(_, rt, wti, bwdi)) = self.buf_out.get(&nwd) else {
                    continue;
                };
                if rt != 0 {
                    continue;
                }
                if !tk.pips.contains_key(&(wfi, wti)) {
                    continue;
                }
                if let Some(&(_, wf)) = self.names.get(&nwf) {
                    if !matches!(self.db.wires[wf.1], WireKind::MuxOut) {
                        continue;
                    }
                    assert_eq!(bwdi, wdi);
                    self.node_naming.intf_wires_in.insert(
                        wf,
                        IntfWireInNaming::Delay {
                            name_out: self.rd.wires[wti].clone(),
                            name_delay: self.rd.wires[wdi].clone(),
                            name_in: self.rd.wires[wfi].clone(),
                        },
                    );
                    proxied_in.insert(wti, wf);
                    self.node.intfs.insert(wf, IntfInfo::InputDelay);
                } else if let Some(&wf) = proxied_in.get(&wfi) {
                    let IntfInfo::InputIri(iri, pin) = self.node.intfs[&wf] else {
                        unreachable!();
                    };
                    self.node
                        .intfs
                        .insert(wf, IntfInfo::InputIriDelay(iri, pin));
                    let Some(IntfWireInNaming::Iri {
                        name_out,
                        name_pin_out,
                        name_pin_in,
                        name_in,
                    }) = self.node_naming.intf_wires_in.remove(&wf)
                    else {
                        unreachable!();
                    };
                    self.node_naming.intf_wires_in.insert(
                        wf,
                        IntfWireInNaming::IriDelay {
                            name_out: self.rd.wires[wti].clone(),
                            name_delay: self.rd.wires[wdi].clone(),
                            name_pre_delay: name_out,
                            name_pin_out,
                            name_pin_in,
                            name_in,
                        },
                    );
                    proxied_in.remove(&wfi);
                    proxied_in.insert(wti, wf);
                }
            }
        }
        let mut out_muxes: HashMap<NodeWireId, (Vec<NodeWireId>, Option<NodeWireId>)> =
            HashMap::new();
        for &(wfi, wti) in tk.pips.keys() {
            let nwt = self.rd.lookup_wire_raw_force(crd, wti);
            if let Some(&(_, wt)) = self.names.get(&nwt) {
                if !matches!(self.db.wires[wt.1], WireKind::LogicOut) {
                    continue;
                }
                self.node_naming
                    .intf_wires_out
                    .entry(wt)
                    .or_insert_with(|| IntfWireOutNaming::Simple {
                        name: self.rd.wires[wti].clone(),
                    });
                let nwf = self.rd.lookup_wire_raw_force(crd, wfi);
                if let Some(&(_, wf)) = self.names.get(&nwf) {
                    self.node_naming.intf_wires_in.insert(
                        wf,
                        IntfWireInNaming::Simple {
                            name: self.rd.wires[wfi].clone(),
                        },
                    );
                    assert!(!self.node.intfs.contains_key(&wf));
                    if self.db.wires[wf.1] == WireKind::LogicOut
                        || self.xnode.builder.test_mux_pass.contains(&wf.1)
                    {
                        assert!(out_muxes.entry(wt).or_default().1.replace(wf).is_none());
                    } else {
                        out_muxes.entry(wt).or_default().0.push(wf);
                    }
                } else if let Some(&wf) = proxied_in.get(&wfi) {
                    out_muxes.entry(wt).or_default().0.push(wf);
                } else if let Some(&(_, wf, rt, bwti, bwfi)) = self.int_in.get(&nwf) {
                    if !self.buf_in.contains_key(&nwf) {
                        assert!(!self.xnode.has_intf_out_bufs);
                        continue;
                    }
                    assert_eq!(rt, 0);
                    assert_eq!(bwti, wfi);
                    self.node_naming.intf_wires_in.insert(
                        wf,
                        IntfWireInNaming::TestBuf {
                            name_out: self.rd.wires[wfi].clone(),
                            name_in: self.rd.wires[bwfi].clone(),
                        },
                    );
                    assert!(!self.node.intfs.contains_key(&wf));
                    out_muxes.entry(wt).or_default().0.push(wf);
                } else if self.xnode.has_intf_out_bufs {
                    out_muxes.entry(wt).or_default();
                    self.node_naming.intf_wires_out.insert(
                        wt,
                        IntfWireOutNaming::Buf {
                            name_out: self.rd.wires[wti].clone(),
                            name_in: self.rd.wires[wfi].clone(),
                        },
                    );
                }
            }
        }
        for (wt, (wfs, pwf)) in out_muxes {
            let wfs = wfs.into_iter().collect();
            self.node.intfs.insert(
                wt,
                match pwf {
                    None => IntfInfo::OutputTestMux(wfs),
                    Some(pwf) => IntfInfo::OutputTestMuxPass(wfs, pwf),
                },
            );
        }
    }
}

#[derive(Clone, Debug)]
struct NodeType {
    tki: rawdump::TileKindId,
    naming: NodeNamingId,
}

pub struct IntBuilder<'a> {
    pub rd: &'a Part,
    pub db: IntDb,
    main_passes: EnumMap<Dir, EntityPartVec<WireId, WireId>>,
    node_types: Vec<NodeType>,
    injected_node_types: Vec<rawdump::TileKindId>,
    stub_outs: HashSet<String>,
    extra_names: HashMap<String, NodeWireId>,
    extra_names_tile: HashMap<rawdump::TileKindId, HashMap<String, NodeWireId>>,
    test_mux_pass: HashSet<WireId>,
}

impl<'a> IntBuilder<'a> {
    pub fn new(name: &str, rd: &'a Part) -> Self {
        let db = IntDb {
            name: name.to_string(),
            wires: Default::default(),
            nodes: Default::default(),
            terms: Default::default(),
            node_namings: Default::default(),
            term_namings: Default::default(),
        };
        Self {
            rd,
            db,
            main_passes: Default::default(),
            node_types: vec![],
            injected_node_types: vec![],
            stub_outs: Default::default(),
            extra_names: Default::default(),
            extra_names_tile: Default::default(),
            test_mux_pass: Default::default(),
        }
    }

    pub fn test_mux_pass(&mut self, wire: WireId) {
        self.test_mux_pass.insert(wire);
    }

    pub fn bel_virtual(&self, name: impl Into<String>) -> ExtrBelInfo {
        ExtrBelInfo {
            name: name.into(),
            slot: None,
            pins: Default::default(),
            raw_tile: 0,
        }
    }

    pub fn bel_single(&self, name: impl Into<String>, slot: &str) -> ExtrBelInfo {
        ExtrBelInfo {
            name: name.into(),
            slot: Some(rawdump::TkSiteSlot::Single(
                self.rd.slot_kinds.get(slot).unwrap(),
            )),
            pins: Default::default(),
            raw_tile: 0,
        }
    }

    pub fn bel_indexed(&self, name: impl Into<String>, slot: &str, idx: u8) -> ExtrBelInfo {
        ExtrBelInfo {
            name: name.into(),
            slot: Some(rawdump::TkSiteSlot::Indexed(
                self.rd.slot_kinds.get(slot).unwrap(),
                idx,
            )),
            pins: Default::default(),
            raw_tile: 0,
        }
    }

    pub fn bel_xy(&self, name: impl Into<String>, slot: &str, x: u8, y: u8) -> ExtrBelInfo {
        ExtrBelInfo {
            name: name.into(),
            slot: Some(rawdump::TkSiteSlot::Xy(
                self.rd.slot_kinds.get(slot).expect("missing slot kind"),
                x,
                y,
            )),
            pins: Default::default(),
            raw_tile: 0,
        }
    }

    pub fn make_term_naming(&mut self, name: impl AsRef<str>) -> TermNamingId {
        match self.db.term_namings.get(name.as_ref()) {
            None => {
                self.db
                    .term_namings
                    .insert(name.as_ref().to_string(), Default::default())
                    .0
            }
            Some((i, _)) => i,
        }
    }

    pub fn name_term_in_near_wire(
        &mut self,
        naming: TermNamingId,
        wire: WireId,
        name: impl AsRef<str>,
    ) {
        let name = name.as_ref();
        let naming = &mut self.db.term_namings[naming];
        if !naming.wires_in_near.contains_id(wire) {
            naming.wires_in_near.insert(wire, name.to_string());
        } else {
            assert_eq!(naming.wires_in_near[wire], name);
        }
    }

    pub fn name_term_in_far_wire(
        &mut self,
        naming: TermNamingId,
        wire: WireId,
        name: impl AsRef<str>,
    ) {
        let name = name.as_ref();
        let naming = &mut self.db.term_namings[naming];
        if !naming.wires_in_far.contains_id(wire) {
            naming.wires_in_far.insert(
                wire,
                TermWireInFarNaming::Simple {
                    name: name.to_string(),
                },
            );
        } else {
            assert_matches!(&naming.wires_in_far[wire], TermWireInFarNaming::Simple{name: n} if n == name);
        }
    }

    pub fn name_term_in_far_buf_wire(
        &mut self,
        naming: TermNamingId,
        wire: WireId,
        name_out: impl AsRef<str>,
        name_in: impl AsRef<str>,
    ) {
        let name_out = name_out.as_ref();
        let name_in = name_in.as_ref();
        let naming = &mut self.db.term_namings[naming];
        if !naming.wires_in_far.contains_id(wire) {
            naming.wires_in_far.insert(
                wire,
                TermWireInFarNaming::Buf {
                    name_out: name_out.to_string(),
                    name_in: name_in.to_string(),
                },
            );
        } else {
            assert_matches!(&naming.wires_in_far[wire], TermWireInFarNaming::Buf{name_out: no, name_in: ni} if no == name_out && ni == name_in);
        }
    }

    pub fn name_term_in_far_buf_far_wire(
        &mut self,
        naming: TermNamingId,
        wire: WireId,
        name: impl AsRef<str>,
        name_out: impl AsRef<str>,
        name_in: impl AsRef<str>,
    ) {
        let name = name.as_ref();
        let name_out = name_out.as_ref();
        let name_in = name_in.as_ref();
        let naming = &mut self.db.term_namings[naming];
        if !naming.wires_in_far.contains_id(wire) {
            naming.wires_in_far.insert(
                wire,
                TermWireInFarNaming::BufFar {
                    name: name.to_string(),
                    name_far_out: name_out.to_string(),
                    name_far_in: name_in.to_string(),
                },
            );
        } else {
            assert_matches!(&naming.wires_in_far[wire], TermWireInFarNaming::BufFar{name: n, name_far_out: no, name_far_in: ni} if n == name && no == name_out && ni == name_in);
        }
    }

    pub fn name_term_out_wire(
        &mut self,
        naming: TermNamingId,
        wire: WireId,
        name: impl AsRef<str>,
    ) {
        let name = name.as_ref();
        let naming = &mut self.db.term_namings[naming];
        if !naming.wires_out.contains_id(wire) {
            naming.wires_out.insert(
                wire,
                TermWireOutNaming::Simple {
                    name: name.to_string(),
                },
            );
        } else {
            assert_matches!(&naming.wires_out[wire], TermWireOutNaming::Simple{name: n} if n == name);
        }
    }

    pub fn name_term_out_buf_wire(
        &mut self,
        naming: TermNamingId,
        wire: WireId,
        name_out: impl AsRef<str>,
        name_in: impl AsRef<str>,
    ) {
        let name_out = name_out.as_ref();
        let name_in = name_in.as_ref();
        let naming = &mut self.db.term_namings[naming];
        if !naming.wires_out.contains_id(wire) {
            naming.wires_out.insert(
                wire,
                TermWireOutNaming::Buf {
                    name_out: name_out.to_string(),
                    name_in: name_in.to_string(),
                },
            );
        } else {
            assert_matches!(&naming.wires_out[wire], TermWireOutNaming::Buf{name_out: no, name_in: ni} if no == name_out && ni == name_in);
        }
    }

    pub fn find_wire(&mut self, name: impl AsRef<str>) -> WireId {
        for (i, k, _) in &self.db.wires {
            if k == name.as_ref() {
                return i;
            }
        }
        unreachable!();
    }

    pub fn wire(
        &mut self,
        name: impl Into<String>,
        kind: WireKind,
        raw_names: &[impl AsRef<str>],
    ) -> WireId {
        let res = self.db.wires.insert_new(name.into(), kind);
        for rn in raw_names {
            let rn = rn.as_ref();
            if !rn.is_empty() {
                self.extra_name(rn, res);
            }
        }
        res
    }

    pub fn mux_out(&mut self, name: impl Into<String>, raw_names: &[impl AsRef<str>]) -> WireId {
        self.wire(name, WireKind::MuxOut, raw_names)
    }

    pub fn logic_out(&mut self, name: impl Into<String>, raw_names: &[impl AsRef<str>]) -> WireId {
        self.wire(name, WireKind::LogicOut, raw_names)
    }

    pub fn multi_out(&mut self, name: impl Into<String>, raw_names: &[impl AsRef<str>]) -> WireId {
        self.wire(name, WireKind::MultiOut, raw_names)
    }

    pub fn test_out(&mut self, name: impl Into<String>, raw_names: &[impl AsRef<str>]) -> WireId {
        self.wire(name, WireKind::TestOut, raw_names)
    }

    pub fn buf(
        &mut self,
        src: WireId,
        name: impl Into<String>,
        raw_names: &[impl AsRef<str>],
    ) -> WireId {
        self.wire(name, WireKind::Buf(src), raw_names)
    }

    pub fn conn_branch(&mut self, src: WireId, dir: Dir, dst: WireId) {
        self.main_passes[!dir].insert(dst, src);
    }

    pub fn branch(
        &mut self,
        src: WireId,
        dir: Dir,
        name: impl Into<String>,
        raw_names: &[impl AsRef<str>],
    ) -> WireId {
        let res = self.wire(name, WireKind::Branch(!dir), raw_names);
        self.conn_branch(src, dir, res);
        res
    }

    pub fn multi_branch(
        &mut self,
        src: WireId,
        dir: Dir,
        name: impl Into<String>,
        raw_names: &[impl AsRef<str>],
    ) -> WireId {
        let res = self.wire(name, WireKind::MultiBranch(!dir), raw_names);
        self.conn_branch(src, dir, res);
        res
    }

    pub fn pip_branch(
        &mut self,
        src: WireId,
        dir: Dir,
        name: impl Into<String>,
        raw_names: &[impl AsRef<str>],
    ) -> WireId {
        let res = self.wire(name, WireKind::PipBranch(!dir), raw_names);
        self.conn_branch(src, dir, res);
        res
    }

    pub fn stub_out(&mut self, name: impl Into<String>) {
        self.stub_outs.insert(name.into());
    }

    pub fn extra_name(&mut self, name: impl Into<String>, wire: WireId) {
        self.extra_names
            .insert(name.into(), (NodeTileId::from_idx(0), wire));
    }

    pub fn extra_name_sub(&mut self, name: impl Into<String>, sub: usize, wire: WireId) {
        self.extra_names
            .insert(name.into(), (NodeTileId::from_idx(sub), wire));
    }

    pub fn extra_name_tile(
        &mut self,
        tile: impl AsRef<str>,
        name: impl Into<String>,
        wire: WireId,
    ) {
        if let Some((tki, _)) = self.rd.tile_kinds.get(tile.as_ref()) {
            self.extra_names_tile
                .entry(tki)
                .or_default()
                .insert(name.into(), (NodeTileId::from_idx(0), wire));
        }
    }

    pub fn extra_name_tile_sub(
        &mut self,
        tile: impl AsRef<str>,
        name: impl Into<String>,
        sub: usize,
        wire: WireId,
    ) {
        if let Some((tki, _)) = self.rd.tile_kinds.get(tile.as_ref()) {
            self.extra_names_tile
                .entry(tki)
                .or_default()
                .insert(name.into(), (NodeTileId::from_idx(sub), wire));
        }
    }

    pub fn get_wire_by_name(&self, tki: TileKindId, name: &str) -> Option<NodeWireId> {
        self.extra_names
            .get(name)
            .or_else(|| self.extra_names_tile.get(&tki).and_then(|m| m.get(name)))
            .copied()
    }

    pub fn extract_main_passes(&mut self) {
        for (dir, wires) in &self.main_passes {
            self.db.terms.insert(
                format!("MAIN.{dir}"),
                TermKind {
                    dir,
                    wires: wires
                        .iter()
                        .map(|(k, &v)| (k, TermInfo::PassFar(v)))
                        .collect(),
                },
            );
        }
    }

    fn extract_bels(
        &self,
        node: &mut NodeKind,
        naming: &mut NodeNaming,
        bels: &[ExtrBelInfo],
        tki: rawdump::TileKindId,
        names: &HashMap<rawdump::WireId, (IntConnKind, NodeWireId)>,
    ) {
        let tk = &self.rd.tile_kinds[tki];
        if bels.is_empty() {
            return;
        }
        let mut edges_in: HashMap<_, Vec<_>> = HashMap::new();
        let mut edges_out: HashMap<_, Vec<_>> = HashMap::new();
        for &(wfi, wti) in tk.pips.keys() {
            edges_in.entry(wti).or_default().push(wfi);
            edges_out.entry(wfi).or_default().push(wti);
        }
        let buf_out: HashMap<_, _> = edges_out
            .iter()
            .filter_map(|(&wt, wfs)| {
                if wfs.len() == 1 {
                    Some((wt, wfs.clone()))
                } else {
                    let filtered: Vec<_> = wfs
                        .iter()
                        .copied()
                        .filter(|x| names.contains_key(x))
                        .collect();
                    if !filtered.is_empty() {
                        Some((wt, filtered))
                    } else {
                        None
                    }
                }
            })
            .collect();
        let buf_in: HashMap<_, _> = edges_in
            .iter()
            .filter_map(|(&wt, wfs)| {
                if wfs.len() == 1 {
                    Some((wt, wfs[0]))
                } else {
                    let filtered: Vec<_> = wfs
                        .iter()
                        .copied()
                        .filter(|x| names.contains_key(x))
                        .collect();
                    if filtered.len() == 1 {
                        Some((wt, filtered[0]))
                    } else {
                        None
                    }
                }
            })
            .collect();
        let walk_to_int = |dir, mut wn| {
            let mut pips = Vec::new();
            loop {
                if let Some(&(ick, w)) = names.get(&wn) {
                    return (ick, [w].into_iter().collect(), wn, pips, BTreeMap::new());
                }
                match dir {
                    PinDir::Input => {
                        if let Some(&nwn) = buf_in.get(&wn) {
                            pips.push(NodeExtPipNaming {
                                tile: NodeRawTileId::from_idx(0),
                                wire_to: self.rd.wires[wn].clone(),
                                wire_from: self.rd.wires[nwn].clone(),
                            });
                            wn = nwn;
                            continue;
                        }
                        panic!(
                            "CANNOT WALK INPUT WIRE {} {} {}",
                            self.rd.part,
                            self.rd.tile_kinds.key(tki),
                            self.rd.wires[wn]
                        );
                    }
                    PinDir::Output => {
                        if let Some(nwn) = buf_out.get(&wn) {
                            if nwn.len() == 1 {
                                let nwn = nwn[0];
                                pips.push(NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(0),
                                    wire_to: self.rd.wires[nwn].clone(),
                                    wire_from: self.rd.wires[wn].clone(),
                                });
                                wn = nwn;
                                continue;
                            } else if nwn.iter().all(|x| names.contains_key(x)) {
                                let mut wires = BTreeSet::new();
                                let mut int_pips = BTreeMap::new();
                                let mut ick = None;
                                for &nwn in nwn {
                                    let (cick, w) = names[&nwn];
                                    ick = Some(cick);
                                    wires.insert(w);
                                    int_pips.insert(
                                        w,
                                        NodeExtPipNaming {
                                            tile: NodeRawTileId::from_idx(0),
                                            wire_to: self.rd.wires[nwn].clone(),
                                            wire_from: self.rd.wires[wn].clone(),
                                        },
                                    );
                                }
                                return (ick.unwrap(), wires, wn, pips, int_pips);
                            }
                        }
                        panic!(
                            "CANNOT WALK OUTPUT WIRE {} {} {}",
                            self.rd.part,
                            self.rd.tile_kinds.key(tki),
                            self.rd.wires[wn]
                        );
                    }
                    PinDir::Inout => {
                        panic!(
                            "CANNOT WALK INOUT WIRE {} {} {}",
                            self.rd.part,
                            self.rd.tile_kinds.key(tki),
                            self.rd.wires[wn]
                        );
                    }
                }
            }
        };
        let walk_count = |dir, mut wn, cnt| {
            let mut pips = Vec::new();
            for _ in 0..cnt {
                let nwn = match dir {
                    PinDir::Input => {
                        if let Some(&nwn) = buf_in.get(&wn) {
                            pips.push(NodeExtPipNaming {
                                tile: NodeRawTileId::from_idx(0),
                                wire_to: self.rd.wires[wn].clone(),
                                wire_from: self.rd.wires[nwn].clone(),
                            });
                            Some(nwn)
                        } else {
                            None
                        }
                    }
                    PinDir::Output => {
                        if let Some(nwn) = buf_out.get(&wn) {
                            if nwn.len() == 1 {
                                let nwn = nwn[0];
                                pips.push(NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(0),
                                    wire_to: self.rd.wires[nwn].clone(),
                                    wire_from: self.rd.wires[wn].clone(),
                                });
                                Some(nwn)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    PinDir::Inout => None,
                };
                if let Some(nwn) = nwn {
                    wn = nwn
                } else {
                    panic!(
                        "CANNOT WALK WIRE {} {} {}",
                        self.rd.part,
                        self.rd.tile_kinds.key(tki),
                        self.rd.wires[wn]
                    );
                }
            }
            (wn, pips)
        };
        for bel in bels {
            let mut pins = BTreeMap::new();
            let mut naming_pins = BTreeMap::new();
            if let Some(slot) = bel.slot {
                let tks = tk.sites.get(&slot).unwrap().1;
                for (name, tksp) in &tks.pins {
                    match bel.pins.get(name).unwrap_or(&BelPinInfo::Int) {
                        &BelPinInfo::Int => {
                            let dir = match tksp.dir {
                                rawdump::TkSitePinDir::Input => PinDir::Input,
                                rawdump::TkSitePinDir::Output => PinDir::Output,
                                _ => panic!("bidir pin {name}"),
                            };
                            let (ick, wires, wnf, pips, int_pips) =
                                walk_to_int(dir, tksp.wire.unwrap());
                            naming_pins.insert(
                                name.clone(),
                                BelPinNaming {
                                    name: self.rd.wires[tksp.wire.unwrap()].clone(),
                                    name_far: self.rd.wires[wnf].clone(),
                                    pips,
                                    int_pips,
                                    is_intf_out: ick == IntConnKind::IntfOut,
                                },
                            );
                            pins.insert(
                                name.clone(),
                                BelPin {
                                    wires,
                                    dir,
                                    is_intf_in: ick == IntConnKind::IntfIn,
                                },
                            );
                        }
                        &BelPinInfo::ForceInt(wire, ref wname) => {
                            let dir = match tksp.dir {
                                rawdump::TkSitePinDir::Input => PinDir::Input,
                                rawdump::TkSitePinDir::Output => PinDir::Output,
                                _ => panic!("bidir pin {name}"),
                            };
                            naming_pins.insert(
                                name.clone(),
                                BelPinNaming {
                                    name: self.rd.wires[tksp.wire.unwrap()].clone(),
                                    name_far: wname.clone(),
                                    pips: Vec::new(),
                                    int_pips: BTreeMap::new(),
                                    is_intf_out: false,
                                },
                            );
                            pins.insert(
                                name.clone(),
                                BelPin {
                                    wires: [wire].into_iter().collect(),
                                    dir,
                                    is_intf_in: false,
                                },
                            );
                        }
                        &BelPinInfo::NameOnly(buf_cnt) => {
                            if buf_cnt == 0 {
                                naming_pins.insert(
                                    name.clone(),
                                    BelPinNaming {
                                        name: self.rd.wires[tksp.wire.unwrap()].clone(),
                                        name_far: self.rd.wires[tksp.wire.unwrap()].clone(),
                                        pips: Vec::new(),
                                        int_pips: BTreeMap::new(),
                                        is_intf_out: false,
                                    },
                                );
                            } else {
                                let dir = match tksp.dir {
                                    rawdump::TkSitePinDir::Input => PinDir::Input,
                                    rawdump::TkSitePinDir::Output => PinDir::Output,
                                    _ => panic!("bidir pin {name}"),
                                };
                                let (wn, pips) = walk_count(dir, tksp.wire.unwrap(), buf_cnt);
                                naming_pins.insert(
                                    name.clone(),
                                    BelPinNaming {
                                        name: self.rd.wires[tksp.wire.unwrap()].clone(),
                                        name_far: self.rd.wires[wn].clone(),
                                        pips,
                                        int_pips: BTreeMap::new(),
                                        is_intf_out: false,
                                    },
                                );
                            }
                        }
                        BelPinInfo::ExtraWireForce(_, _) => (),
                        _ => unreachable!(),
                    }
                }
            }
            for (name, pd) in &bel.pins {
                match *pd {
                    BelPinInfo::ExtraInt(dir, ref names) => {
                        let mut wn = None;
                        for w in names {
                            if let Some(w) = self.rd.wires.get(w) {
                                if tk.wires.contains_key(&w) {
                                    assert!(wn.is_none());
                                    wn = Some(w);
                                }
                            }
                        }
                        if wn.is_none() {
                            println!("NOT FOUND: {name}");
                        }
                        let wn = wn.unwrap();
                        let (ick, wires, wnf, pips, int_pips) = walk_to_int(dir, wn);
                        naming_pins.insert(
                            name.clone(),
                            BelPinNaming {
                                name: self.rd.wires[wn].clone(),
                                name_far: self.rd.wires[wnf].clone(),
                                pips,
                                int_pips,
                                is_intf_out: ick == IntConnKind::IntfOut,
                            },
                        );
                        pins.insert(
                            name.clone(),
                            BelPin {
                                wires,
                                dir,
                                is_intf_in: ick == IntConnKind::IntfIn,
                            },
                        );
                    }
                    BelPinInfo::ExtraIntForce(dir, wire, ref wname) => {
                        naming_pins.insert(
                            name.clone(),
                            BelPinNaming {
                                name: wname.clone(),
                                name_far: wname.clone(),
                                pips: vec![],
                                int_pips: BTreeMap::new(),
                                is_intf_out: false,
                            },
                        );
                        pins.insert(
                            name.clone(),
                            BelPin {
                                wires: [wire].into_iter().collect(),
                                dir,
                                is_intf_in: false,
                            },
                        );
                    }
                    BelPinInfo::ExtraWire(ref names) => {
                        let mut wn = None;
                        for w in names {
                            if let Some(w) = self.rd.wires.get(w) {
                                if tk.wires.contains_key(&w) {
                                    assert!(wn.is_none());
                                    wn = Some(w);
                                }
                            }
                        }
                        if wn.is_none() {
                            println!("NOT FOUND: {name}");
                        }
                        let wn = wn.unwrap();
                        naming_pins.insert(
                            name.clone(),
                            BelPinNaming {
                                name: self.rd.wires[wn].clone(),
                                name_far: self.rd.wires[wn].clone(),
                                pips: Vec::new(),
                                int_pips: BTreeMap::new(),
                                is_intf_out: false,
                            },
                        );
                    }
                    BelPinInfo::ExtraWireForce(ref wname, ref pips) => {
                        naming_pins.insert(
                            name.clone(),
                            BelPinNaming {
                                name: wname.clone(),
                                name_far: wname.clone(),
                                pips: pips.clone(),
                                int_pips: BTreeMap::new(),
                                is_intf_out: false,
                            },
                        );
                    }
                    _ => (),
                }
            }
            node.bels.insert(bel.name.clone(), BelInfo { pins });
            naming.bels.push(BelNaming {
                tile: NodeRawTileId::from_idx(0),
                pins: naming_pins,
            });
        }
    }

    pub fn extract_node(
        &mut self,
        tile_kind: &str,
        kind: &str,
        naming: &str,
        bels: &[ExtrBelInfo],
    ) {
        if let Some((tki, _)) = self.rd.tile_kinds.get(tile_kind) {
            let tk = &self.rd.tile_kinds[tki];
            let tkn = self.rd.tile_kinds.key(tki);
            let mut node = NodeKind {
                tiles: [()].into_iter().collect(),
                muxes: Default::default(),
                bels: Default::default(),
                iris: Default::default(),
                intfs: Default::default(),
            };
            let mut node_naming = NodeNaming::default();
            let mut names = HashMap::new();
            for &wi in tk.wires.keys() {
                if let Some(w) = self.get_wire_by_name(tki, &self.rd.wires[wi]) {
                    names.insert(wi, (IntConnKind::Raw, w));
                }
            }

            for (&k, &(_, v)) in &names {
                node_naming.wires.insert(v, self.rd.wires[k].clone());
            }

            for &(wfi, wti) in tk.pips.keys() {
                if let Some(&(_, wt)) = names.get(&wti) {
                    match self.db.wires[wt.1] {
                        WireKind::PipBranch(_)
                        | WireKind::PipOut
                        | WireKind::MultiBranch(_)
                        | WireKind::MultiOut
                        | WireKind::MuxOut => (),
                        WireKind::Branch(_) => {
                            if self.db.name != "virtex" {
                                continue;
                            }
                        }
                        WireKind::Buf(dwf) => {
                            let wf = names[&wfi].1;
                            assert_eq!(wf, (wt.0, dwf));
                            continue;
                        }
                        _ => continue,
                    }
                    if let Some(&(_, wf)) = names.get(&wfi) {
                        // XXX
                        let kind = MuxKind::Plain;
                        node.muxes.entry(wt).or_insert(MuxInfo {
                            kind,
                            ins: Default::default(),
                        });
                        let mux = node.muxes.get_mut(&wt).unwrap();
                        assert_eq!(mux.kind, kind);
                        mux.ins.insert(wf);
                    } else if self.stub_outs.contains(&self.rd.wires[wfi]) {
                        // ignore
                    } else {
                        println!(
                            "UNEXPECTED INT MUX IN {} {} {}",
                            tkn, self.rd.wires[wti], self.rd.wires[wfi]
                        );
                    }
                }
            }

            self.extract_bels(&mut node, &mut node_naming, bels, tki, &names);

            self.insert_node_merge(kind, node);
            let naming = self.insert_node_naming(naming, node_naming);
            self.node_types.push(NodeType { tki, naming });
        }
    }

    pub fn extract_node_bels(
        &mut self,
        tile_kind: &str,
        kind: &str,
        naming: &str,
        bels: &[ExtrBelInfo],
    ) {
        if let Some((tki, _)) = self.rd.tile_kinds.get(tile_kind) {
            let tk = &self.rd.tile_kinds[tki];
            let mut names = HashMap::new();
            for &wi in tk.wires.keys() {
                if let Some(w) = self.get_wire_by_name(tki, &self.rd.wires[wi]) {
                    names.insert(wi, (IntConnKind::Raw, w));
                }
            }

            let mut node = NodeKind {
                tiles: [()].into_iter().collect(),
                muxes: Default::default(),
                bels: Default::default(),
                iris: Default::default(),
                intfs: Default::default(),
            };
            let mut node_naming = NodeNaming::default();
            self.extract_bels(&mut node, &mut node_naming, bels, tki, &names);

            self.insert_node_merge(kind, node);
            self.insert_node_naming(naming, node_naming);
        }
    }

    pub fn node_type(&mut self, tile_kind: &str, kind: &str, naming: &str) {
        self.extract_node(tile_kind, kind, naming, &[]);
    }

    pub fn inject_node_type(&mut self, tile_kind: &str) {
        if let Some((tki, _)) = self.rd.tile_kinds.get(tile_kind) {
            self.injected_node_types.push(tki);
        }
    }

    pub fn inject_node_type_naming(&mut self, tile_kind: &str, naming: NodeNamingId) {
        if let Some((tki, _)) = self.rd.tile_kinds.get(tile_kind) {
            self.node_types.push(NodeType { tki, naming });
        }
    }

    fn get_int_naming(&self, int_xy: Coord) -> Option<NodeNamingId> {
        let int_tile = &self.rd.tiles[&int_xy];
        self.node_types.iter().find_map(|nt| {
            if nt.tki == int_tile.kind {
                Some(nt.naming)
            } else {
                None
            }
        })
    }

    fn get_int_rev_naming(&self, int_xy: Coord) -> HashMap<String, WireId> {
        if let Some(int_naming_id) = self.get_int_naming(int_xy) {
            let int_naming = &self.db.node_namings[int_naming_id];
            int_naming
                .wires
                .iter()
                .filter_map(|(k, v)| {
                    if k.0.to_idx() == 0 {
                        Some((v.to_string(), k.1))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Default::default()
        }
    }

    fn get_node(
        &self,
        tile: &rawdump::Tile,
        tk: &rawdump::TileKind,
        wi: rawdump::WireId,
    ) -> Option<rawdump::NodeId> {
        if let Some((_, &rawdump::TkWire::Connected(idx))) = tk.wires.get(&wi) {
            if let Some(&nidx) = tile.conn_wires.get(idx) {
                return Some(nidx);
            }
        }
        None
    }

    fn get_int_node2wires(&self, int_xy: Coord) -> HashMap<rawdump::NodeId, Vec<WireId>> {
        let int_tile = &self.rd.tiles[&int_xy];
        let int_tk = &self.rd.tile_kinds[int_tile.kind];
        let int_rev_naming = self.get_int_rev_naming(int_xy);
        let mut res: HashMap<_, Vec<_>> = HashMap::new();
        for (_, &wi, &tkw) in &int_tk.wires {
            if let Some(&w) = int_rev_naming.get(&self.rd.wires[wi]) {
                if let rawdump::TkWire::Connected(idx) = tkw {
                    if let Some(&nidx) = int_tile.conn_wires.get(idx) {
                        res.entry(nidx).or_default().push(w);
                    }
                }
            }
        }
        res
    }

    pub fn recover_names(
        &self,
        tile_xy: Coord,
        int_xy: Coord,
    ) -> HashMap<rawdump::WireId, Vec<WireId>> {
        if tile_xy == int_xy {
            let int_tile = &self.rd.tiles[&int_xy];
            let int_tk = &self.rd.tile_kinds[int_tile.kind];
            let int_rev_naming = self.get_int_rev_naming(int_xy);
            let mut res = HashMap::new();
            for &wi in int_tk.wires.keys() {
                let n = &self.rd.wires[wi];
                if let Some(&w) = int_rev_naming.get(n) {
                    res.insert(wi, vec![w]);
                }
            }
            res
        } else {
            let node2wires = self.get_int_node2wires(int_xy);
            let tile = &self.rd.tiles[&tile_xy];
            let tk = &self.rd.tile_kinds[tile.kind];
            let mut res = HashMap::new();
            for (_, &wi, &tkw) in &tk.wires {
                if let Some(w) = self.get_wire_by_name(tile.kind, &self.rd.wires[wi]) {
                    res.insert(wi, vec![w.1]);
                } else if let rawdump::TkWire::Connected(idx) = tkw {
                    if let Some(&nidx) = tile.conn_wires.get(idx) {
                        if let Some(w) = node2wires.get(&nidx) {
                            res.insert(wi, w.clone());
                        }
                    }
                }
            }
            res
        }
    }

    pub fn recover_names_cands(
        &self,
        tile_xy: Coord,
        int_xy: Coord,
        cands: &HashSet<WireId>,
    ) -> HashMap<rawdump::WireId, WireId> {
        self.recover_names(tile_xy, int_xy)
            .into_iter()
            .filter_map(|(k, v)| {
                let nv: Vec<_> = v.into_iter().filter(|x| cands.contains(x)).collect();
                if nv.len() == 1 {
                    Some((k, nv[0]))
                } else {
                    None
                }
            })
            .collect()
    }

    fn insert_node_merge(&mut self, name: &str, node: NodeKind) -> NodeKindId {
        match self.db.nodes.get_mut(name) {
            None => self.db.nodes.insert(name.to_string(), node).0,
            Some((id, cnode)) => {
                assert_eq!(node.tiles, cnode.tiles);
                assert_eq!(node.bels, cnode.bels);
                for (k, v) in node.muxes {
                    match cnode.muxes.get_mut(&k) {
                        None => {
                            cnode.muxes.insert(k, v);
                        }
                        Some(cv) => {
                            assert_eq!(cv.kind, v.kind);
                            for x in v.ins {
                                cv.ins.insert(x);
                            }
                        }
                    }
                }
                for &k in cnode.intfs.keys() {
                    assert!(node.intfs.contains_key(&k));
                }
                for (k, v) in node.intfs {
                    let cv = cnode.intfs.get_mut(&k).unwrap();
                    match v {
                        IntfInfo::InputDelay
                        | IntfInfo::InputIri(..)
                        | IntfInfo::InputIriDelay(..) => {
                            assert_eq!(*cv, v);
                        }
                        IntfInfo::OutputTestMux(ref wfs) => {
                            if let IntfInfo::OutputTestMux(cwfs) = cv {
                                for &wf in wfs {
                                    cwfs.insert(wf);
                                }
                            } else {
                                assert_eq!(*cv, v);
                            }
                        }
                        IntfInfo::OutputTestMuxPass(ref wfs, pwf) => {
                            if let IntfInfo::OutputTestMuxPass(cwfs, cpwf) = cv {
                                assert_eq!(pwf, *cpwf);
                                for &wf in wfs {
                                    cwfs.insert(wf);
                                }
                            } else {
                                assert_eq!(*cv, v);
                            }
                        }
                    }
                }
                id
            }
        }
    }

    fn insert_node_naming(&mut self, name: &str, naming: NodeNaming) -> NodeNamingId {
        match self.db.node_namings.get_mut(name) {
            None => self.db.node_namings.insert(name.to_string(), naming).0,
            Some((id, cnaming)) => {
                assert_eq!(naming.ext_pips, cnaming.ext_pips);
                assert_eq!(naming.wire_bufs, cnaming.wire_bufs);
                assert_eq!(naming.bels, cnaming.bels);
                for (k, v) in naming.wires {
                    match cnaming.wires.get(&k) {
                        None => {
                            cnaming.wires.insert(k, v);
                        }
                        Some(cv) => {
                            assert_eq!(v, *cv);
                        }
                    }
                }
                for (k, v) in naming.intf_wires_in {
                    match cnaming.intf_wires_in.get(&k) {
                        None => {
                            cnaming.intf_wires_in.insert(k, v);
                        }
                        Some(cv) => {
                            assert_eq!(v, *cv);
                        }
                    }
                }
                for (k, v) in naming.intf_wires_out {
                    match cnaming.intf_wires_out.get(&k) {
                        None => {
                            cnaming.intf_wires_out.insert(k, v);
                        }
                        Some(cv @ IntfWireOutNaming::Buf { name_out, .. }) => match v {
                            IntfWireOutNaming::Buf { .. } => assert_eq!(&v, cv),
                            IntfWireOutNaming::Simple { name } => assert_eq!(&name, name_out),
                        },
                        Some(cv @ IntfWireOutNaming::Simple { name }) => {
                            if let IntfWireOutNaming::Buf { name_out, .. } = &v {
                                assert_eq!(name_out, name);
                                cnaming.intf_wires_out.insert(k, v);
                            } else {
                                assert_eq!(v, *cv);
                            }
                        }
                    }
                }
                id
            }
        }
    }

    pub fn insert_term_merge(&mut self, name: &str, term: TermKind) {
        match self.db.terms.get_mut(name) {
            None => {
                self.db.terms.insert(name.to_string(), term);
            }
            Some((_, cterm)) => {
                assert_eq!(term.dir, cterm.dir);
                for k in self.db.wires.ids() {
                    let a = cterm.wires.get_mut(k);
                    let b = term.wires.get(k);
                    match (a, b) {
                        (_, None) => (),
                        (None, Some(b)) => {
                            cterm.wires.insert(k, b.clone());
                        }
                        (a, b) => assert_eq!(a.map(|x| &*x), b),
                    }
                }
            }
        }
    }

    fn get_pass_inps(&self, dir: Dir) -> HashSet<WireId> {
        self.main_passes[dir].values().copied().collect()
    }

    fn extract_term_tile_conn(
        &self,
        dir: Dir,
        int_xy: Coord,
        forced: &HashMap<WireId, WireId>,
    ) -> EntityPartVec<WireId, TermInfo> {
        let mut res = EntityPartVec::new();
        let n2w = self.get_int_node2wires(int_xy);
        let cand_inps = self.get_pass_inps(!dir);
        for wl in n2w.into_values() {
            for &wt in &wl {
                if !self.main_passes[dir].contains_id(wt) {
                    continue;
                }
                let wf: Vec<_> = wl
                    .iter()
                    .copied()
                    .filter(|&wf| wf != wt && cand_inps.contains(&wf))
                    .collect();
                if let Some(&fwf) = forced.get(&wt) {
                    if wf.contains(&fwf) {
                        res.insert(wt, TermInfo::PassNear(fwf));
                    }
                } else {
                    if wf.len() == 1 {
                        res.insert(wt, TermInfo::PassNear(wf[0]));
                    }
                    if wf.len() > 1 {
                        println!(
                            "WHOOPS MULTI {} {:?}",
                            self.db.wires.key(wt),
                            wf.iter().map(|&x| self.db.wires.key(x)).collect::<Vec<_>>()
                        );
                    }
                }
            }
        }
        res
    }

    pub fn extract_term_tile(
        &mut self,
        name: impl AsRef<str>,
        node_name: Option<&str>,
        dir: Dir,
        term_xy: Coord,
        naming: impl AsRef<str>,
        int_xy: Coord,
    ) {
        let cand_inps = self.get_pass_inps(!dir);
        let names = self.recover_names(term_xy, int_xy);
        let tile = &self.rd.tiles[&term_xy];
        let tk = &self.rd.tile_kinds[tile.kind];
        let tkn = self.rd.tile_kinds.key(tile.kind);
        let mut muxes: HashMap<WireId, Vec<WireId>> = HashMap::new();
        let naming_id = self.make_term_naming(naming.as_ref());
        let mut tnames = EntityPartVec::new();
        for &(wfi, wti) in tk.pips.keys() {
            if let Some(wtl) = names.get(&wti) {
                for &wt in wtl {
                    match self.db.wires[wt] {
                        WireKind::Branch(d) => {
                            if d != dir {
                                continue;
                            }
                        }
                        _ => continue,
                    }
                    if let Some(wfl) = names.get(&wfi) {
                        let wf;
                        if wfl.len() == 1 {
                            wf = wfl[0];
                        } else {
                            let nwfl: Vec<_> = wfl
                                .iter()
                                .copied()
                                .filter(|x| cand_inps.contains(x))
                                .collect();
                            if nwfl.len() == 1 {
                                wf = nwfl[0];
                            } else {
                                println!(
                                    "AMBIG TERM MUX IN {} {} {}",
                                    tkn, self.rd.wires[wti], self.rd.wires[wfi]
                                );
                                continue;
                            }
                        }
                        if tnames.contains_id(wt) {
                            assert_eq!(tnames[wt], &self.rd.wires[wti]);
                        } else {
                            tnames.insert(wt, &self.rd.wires[wti]);
                        }
                        if tnames.contains_id(wf) {
                            assert_eq!(tnames[wf], &self.rd.wires[wfi]);
                        } else {
                            tnames.insert(wf, &self.rd.wires[wfi]);
                        }
                        muxes.entry(wt).or_default().push(wf);
                    } else {
                        println!(
                            "UNEXPECTED TERM MUX IN {} {} {}",
                            tkn, self.rd.wires[wti], self.rd.wires[wfi]
                        );
                    }
                }
            }
        }
        let mut node_muxes = BTreeMap::new();
        let mut node_names = BTreeMap::new();
        let mut wires = self.extract_term_tile_conn(dir, int_xy, &Default::default());
        for (k, v) in muxes {
            if v.len() == 1 {
                self.name_term_out_wire(naming_id, k, tnames[k]);
                self.name_term_in_near_wire(naming_id, v[0], tnames[v[0]]);
                wires.insert(k, TermInfo::PassNear(v[0]));
            } else {
                let def_t = NodeTileId::from_idx(0);
                node_names.insert((def_t, k), tnames[k].to_string());
                for &x in &v {
                    node_names.insert((def_t, x), tnames[x].to_string());
                }
                node_muxes.insert(
                    (def_t, k),
                    MuxInfo {
                        kind: MuxKind::Plain,
                        ins: v.into_iter().map(|x| (def_t, x)).collect(),
                    },
                );
            }
        }
        if let Some(nn) = node_name {
            self.insert_node_merge(
                nn,
                NodeKind {
                    tiles: [()].into_iter().collect(),
                    muxes: node_muxes,
                    bels: Default::default(),
                    iris: Default::default(),
                    intfs: Default::default(),
                },
            );
            self.insert_node_naming(
                naming.as_ref(),
                NodeNaming {
                    wires: node_names,
                    wire_bufs: Default::default(),
                    ext_pips: Default::default(),
                    bels: Default::default(),
                    iris: Default::default(),
                    intf_wires_in: Default::default(),
                    intf_wires_out: Default::default(),
                },
            );
        } else {
            assert!(node_muxes.is_empty());
        }
        let term = TermKind { dir, wires };
        self.insert_term_merge(name.as_ref(), term);
    }

    pub fn extract_term_buf_tile(
        &mut self,
        name: impl AsRef<str>,
        dir: Dir,
        term_xy: Coord,
        naming: impl AsRef<str>,
        int_xy: Coord,
        forced: &[(WireId, WireId)],
    ) {
        let forced: HashMap<_, _> = forced.iter().copied().collect();
        let cand_inps = self.get_pass_inps(!dir);
        let naming = naming.as_ref();
        let names = self.recover_names(term_xy, int_xy);
        let tile = &self.rd.tiles[&term_xy];
        let tk = &self.rd.tile_kinds[tile.kind];
        let tkn = self.rd.tile_kinds.key(tile.kind);
        let mut wires = self.extract_term_tile_conn(dir, int_xy, &forced);
        let naming_id = self.make_term_naming(naming);
        for &(wfi, wti) in tk.pips.keys() {
            if let Some(wtl) = names.get(&wti) {
                for &wt in wtl {
                    match self.db.wires[wt] {
                        WireKind::Branch(d) => {
                            if d != dir {
                                continue;
                            }
                        }
                        _ => continue,
                    }
                    if let Some(wfl) = names.get(&wfi) {
                        let wf;
                        if let Some(&fwf) = forced.get(&wt) {
                            if wfl.contains(&fwf) {
                                wf = fwf;
                            } else {
                                continue;
                            }
                        } else {
                            if wfl.len() == 1 {
                                wf = wfl[0];
                            } else {
                                let nwfl: Vec<_> = wfl
                                    .iter()
                                    .copied()
                                    .filter(|x| cand_inps.contains(x))
                                    .collect();
                                if nwfl.len() == 1 {
                                    wf = nwfl[0];
                                } else {
                                    println!(
                                        "AMBIG TERM MUX IN {} {} {}",
                                        tkn, self.rd.wires[wti], self.rd.wires[wfi]
                                    );
                                    continue;
                                }
                            }
                        }
                        self.name_term_out_buf_wire(
                            naming_id,
                            wt,
                            &self.rd.wires[wti],
                            &self.rd.wires[wfi],
                        );
                        if wires.contains_id(wt) {
                            println!("OOPS DUPLICATE TERM BUF {} {}", tkn, self.rd.wires[wti]);
                        }
                        assert!(!wires.contains_id(wt));
                        wires.insert(wt, TermInfo::PassNear(wf));
                    } else {
                        println!(
                            "UNEXPECTED TERM BUF IN {} {} {}",
                            tkn, self.rd.wires[wti], self.rd.wires[wfi]
                        );
                    }
                }
            }
        }
        let term = TermKind { dir, wires };
        self.insert_term_merge(name.as_ref(), term);
    }

    pub fn extract_term_conn_tile(
        &mut self,
        name: impl AsRef<str>,
        dir: Dir,
        int_xy: Coord,
        forced: &[(WireId, WireId)],
    ) {
        let forced: HashMap<_, _> = forced.iter().copied().collect();
        let wires = self.extract_term_tile_conn(dir, int_xy, &forced);
        let term = TermKind { dir, wires };
        self.insert_term_merge(name.as_ref(), term);
    }

    pub fn walk_to_int(&self, mut xy: Coord, dir: Dir, mut step: bool) -> Option<Coord> {
        loop {
            if !step {
                let tile = &self.rd.tiles[&xy];
                if self.node_types.iter().any(|x| x.tki == tile.kind)
                    || self.injected_node_types.contains(&tile.kind)
                {
                    return Some(xy);
                }
            }
            step = false;
            match dir {
                Dir::W => {
                    if xy.x == 0 {
                        return None;
                    }
                    xy.x -= 1;
                }
                Dir::E => {
                    if xy.x == self.rd.width - 1 {
                        return None;
                    }
                    xy.x += 1;
                }
                Dir::S => {
                    if xy.y == 0 {
                        return None;
                    }
                    xy.y -= 1;
                }
                Dir::N => {
                    if xy.y == self.rd.height - 1 {
                        return None;
                    }
                    xy.y += 1;
                }
            }
        }
    }

    pub fn extract_term(
        &mut self,
        name: impl AsRef<str>,
        node_name: Option<&str>,
        dir: Dir,
        tkn: impl AsRef<str>,
        naming: impl AsRef<str>,
    ) {
        for &term_xy in self.rd.tiles_by_kind_name(tkn.as_ref()) {
            if let Some(int_xy) = self.walk_to_int(term_xy, !dir, false) {
                self.extract_term_tile(
                    name.as_ref(),
                    node_name,
                    dir,
                    term_xy,
                    naming.as_ref(),
                    int_xy,
                );
            }
        }
    }

    pub fn extract_term_buf(
        &mut self,
        name: impl AsRef<str>,
        dir: Dir,
        tkn: impl AsRef<str>,
        naming: impl AsRef<str>,
        forced: &[(WireId, WireId)],
    ) {
        for &term_xy in self.rd.tiles_by_kind_name(tkn.as_ref()) {
            if let Some(int_xy) = self.walk_to_int(term_xy, !dir, false) {
                self.extract_term_buf_tile(
                    name.as_ref(),
                    dir,
                    term_xy,
                    naming.as_ref(),
                    int_xy,
                    forced,
                );
            }
        }
    }

    pub fn extract_term_conn(
        &mut self,
        name: impl AsRef<str>,
        dir: Dir,
        tkn: impl AsRef<str>,
        forced: &[(WireId, WireId)],
    ) {
        for &term_xy in self.rd.tiles_by_kind_name(tkn.as_ref()) {
            if let Some(int_xy) = self.walk_to_int(term_xy, !dir, false) {
                self.extract_term_conn_tile(name.as_ref(), dir, int_xy, forced);
            }
        }
    }

    fn get_bufs(&self, tk: &rawdump::TileKind) -> HashMap<rawdump::WireId, rawdump::WireId> {
        let mut mux_ins: HashMap<rawdump::WireId, Vec<rawdump::WireId>> = HashMap::new();
        for &(wfi, wti) in tk.pips.keys() {
            mux_ins.entry(wti).or_default().push(wfi);
        }
        mux_ins
            .into_iter()
            .filter_map(|(k, v)| if v.len() == 1 { Some((k, v[0])) } else { None })
            .collect()
    }

    pub fn extract_pass_tile(
        &mut self,
        name: impl AsRef<str>,
        dir: Dir,
        int_xy: Coord,
        near: Option<Coord>,
        far: Option<Coord>,
        naming: Option<&str>,
        node: Option<(&str, &str)>,
        splitter_node: Option<(&str, &str)>,
        src_xy: Coord,
        force_pass: &[WireId],
    ) {
        let cand_inps_far = self.get_pass_inps(dir);
        let int_tile = &self.rd.tiles[&int_xy];
        let int_tk = &self.rd.tile_kinds[int_tile.kind];
        let int_naming = &self.db.node_namings[self.get_int_naming(int_xy).unwrap()];
        let mut wires = EntityPartVec::new();
        let src_node2wires = self.get_int_node2wires(src_xy);
        if self.rd.family.starts_with("virtex2") {
            let tcwires = self.extract_term_tile_conn(dir, int_xy, &Default::default());
            for (wt, ti) in tcwires {
                wires.insert(wt, ti);
            }
        }
        for &wn in force_pass {
            if let Some(&wf) = self.main_passes[dir].get(wn) {
                wires.insert(wn, TermInfo::PassFar(wf));
            }
        }
        for wn in self.main_passes[dir].ids() {
            if let Some(wnn) = int_naming.wires.get(&(NodeTileId::from_idx(0), wn)) {
                let wni = self.rd.wires.get(wnn).unwrap();
                if let Some(nidx) = self.get_node(int_tile, int_tk, wni) {
                    if let Some(w) = src_node2wires.get(&nidx) {
                        let w: Vec<_> = w
                            .iter()
                            .copied()
                            .filter(|x| cand_inps_far.contains(x))
                            .collect();
                        if w.len() == 1 {
                            wires.insert(wn, TermInfo::PassFar(w[0]));
                        }
                    }
                }
            }
        }

        if let Some(xy) = near {
            let names = self.recover_names(xy, int_xy);
            let names_far = self.recover_names_cands(xy, src_xy, &cand_inps_far);
            let mut names_far_buf = HashMap::new();
            let tile = &self.rd.tiles[&xy];
            let tk = &self.rd.tile_kinds[tile.kind];
            let tkn = self.rd.tile_kinds.key(tile.kind);
            if let Some(far_xy) = far {
                let far_tile = &self.rd.tiles[&far_xy];
                let far_tk = &self.rd.tile_kinds[far_tile.kind];
                let far_names = self.recover_names_cands(far_xy, src_xy, &cand_inps_far);
                let far_bufs = self.get_bufs(far_tk);
                if far_xy == xy {
                    for (wti, wfi) in far_bufs {
                        if let Some(&wf) = far_names.get(&wfi) {
                            names_far_buf.insert(wti, (wf, wti, wfi));
                        }
                    }
                } else {
                    let mut nodes = HashMap::new();
                    for (wti, wfi) in far_bufs {
                        if let Some(&wf) = far_names.get(&wfi) {
                            if let Some(nidx) = self.get_node(far_tile, far_tk, wti) {
                                nodes.insert(nidx, (wf, wti, wfi));
                            }
                        }
                    }
                    for &wi in tk.wires.keys() {
                        if let Some(nidx) = self.get_node(tile, tk, wi) {
                            if let Some(&x) = nodes.get(&nidx) {
                                names_far_buf.insert(wi, x);
                            }
                        }
                    }
                }
            }
            #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
            enum WireIn {
                Near(WireId),
                Far(WireId),
            }
            #[derive(Clone, Copy)]
            enum FarNaming<'b> {
                Simple(&'b str),
                BufNear(&'b str, &'b str),
                BufFar(&'b str, &'b str, &'b str),
            }
            let mut muxes: HashMap<WireId, Vec<WireIn>> = HashMap::new();
            let mut names_out = EntityPartVec::new();
            let mut names_in_near = EntityPartVec::new();
            let mut names_in_far = EntityPartVec::new();
            for &(wfi, wti) in tk.pips.keys() {
                if let Some(wtl) = names.get(&wti) {
                    for &wt in wtl {
                        match self.db.wires[wt] {
                            WireKind::Branch(d) => {
                                if d != dir {
                                    continue;
                                }
                            }
                            _ => continue,
                        }
                        if wires.contains_id(wt) {
                            continue;
                        }
                        names_out.insert(wt, &self.rd.wires[wti]);
                        if let Some(wfl) = names.get(&wfi) {
                            if wfl.len() != 1 {
                                println!(
                                    "AMBIG PASS MUX IN {} {} {}",
                                    tkn, self.rd.wires[wti], self.rd.wires[wfi]
                                );
                                continue;
                            }
                            let wf = wfl[0];
                            names_in_near.insert(wf, &self.rd.wires[wfi]);
                            muxes.entry(wt).or_default().push(WireIn::Near(wf));
                        } else if let Some(&wf) = names_far.get(&wfi) {
                            names_in_far.insert(wf, FarNaming::Simple(&self.rd.wires[wfi]));
                            muxes.entry(wt).or_default().push(WireIn::Far(wf));
                        } else if let Some(&(wf, woi, wii)) = names_far_buf.get(&wfi) {
                            if xy == far.unwrap() {
                                names_in_far.insert(
                                    wf,
                                    FarNaming::BufNear(&self.rd.wires[woi], &self.rd.wires[wii]),
                                );
                            } else {
                                names_in_far.insert(
                                    wf,
                                    FarNaming::BufFar(
                                        &self.rd.wires[wfi],
                                        &self.rd.wires[woi],
                                        &self.rd.wires[wii],
                                    ),
                                );
                            }
                            muxes.entry(wt).or_default().push(WireIn::Far(wf));
                        } else if self.stub_outs.contains(&self.rd.wires[wfi]) {
                            // ignore
                        } else {
                            println!(
                                "UNEXPECTED PASS MUX IN {} {} {}",
                                tkn, self.rd.wires[wti], self.rd.wires[wfi]
                            );
                        }
                    }
                }
            }
            let mut node_muxes = BTreeMap::new();
            let mut node_tiles = EntityVec::new();
            let mut node_names = BTreeMap::new();
            let mut node_wire_bufs = BTreeMap::new();
            let nt_near = node_tiles.push(());
            let nt_far = node_tiles.push(());
            let naming = naming.map(|n| self.make_term_naming(n));
            for (wt, v) in muxes {
                assert!(!wires.contains_id(wt));
                if v.len() == 1 {
                    self.name_term_out_wire(naming.unwrap(), wt, names_out[wt]);
                    match v[0] {
                        WireIn::Near(wf) => {
                            self.name_term_in_near_wire(naming.unwrap(), wf, names_in_near[wf]);
                            wires.insert(wt, TermInfo::PassNear(wf));
                        }
                        WireIn::Far(wf) => {
                            match names_in_far[wf] {
                                FarNaming::Simple(n) => {
                                    self.name_term_in_far_wire(naming.unwrap(), wf, n)
                                }
                                FarNaming::BufNear(no, ni) => {
                                    self.name_term_in_far_buf_wire(naming.unwrap(), wf, no, ni)
                                }
                                FarNaming::BufFar(n, no, ni) => self.name_term_in_far_buf_far_wire(
                                    naming.unwrap(),
                                    wf,
                                    n,
                                    no,
                                    ni,
                                ),
                            }
                            wires.insert(wt, TermInfo::PassFar(wf));
                        }
                    }
                } else {
                    node_names.insert((nt_near, wt), names_out[wt].to_string());
                    let mut ins = BTreeSet::new();
                    for &x in &v {
                        match x {
                            WireIn::Near(wf) => {
                                node_names.insert((nt_near, wf), names_in_near[wf].to_string());
                                ins.insert((nt_near, wf));
                            }
                            WireIn::Far(wf) => {
                                match names_in_far[wf] {
                                    FarNaming::Simple(n) => {
                                        node_names.insert((nt_far, wf), n.to_string());
                                    }
                                    FarNaming::BufNear(no, ni) => {
                                        node_names.insert((nt_far, wf), no.to_string());
                                        node_wire_bufs.insert(
                                            (nt_far, wf),
                                            NodeExtPipNaming {
                                                tile: NodeRawTileId::from_idx(0),
                                                wire_to: no.to_string(),
                                                wire_from: ni.to_string(),
                                            },
                                        );
                                    }
                                    FarNaming::BufFar(n, no, ni) => {
                                        node_names.insert((nt_far, wf), n.to_string());
                                        node_wire_bufs.insert(
                                            (nt_far, wf),
                                            NodeExtPipNaming {
                                                tile: NodeRawTileId::from_idx(1),
                                                wire_to: no.to_string(),
                                                wire_from: ni.to_string(),
                                            },
                                        );
                                    }
                                }
                                ins.insert((nt_far, wf));
                            }
                        }
                    }
                    node_muxes.insert(
                        (nt_near, wt),
                        MuxInfo {
                            kind: MuxKind::Plain,
                            ins,
                        },
                    );
                }
            }
            if let Some((nn, nnn)) = node {
                self.insert_node_merge(
                    nn,
                    NodeKind {
                        tiles: node_tiles,
                        muxes: node_muxes,
                        bels: Default::default(),
                        iris: Default::default(),
                        intfs: Default::default(),
                    },
                );
                self.insert_node_naming(
                    nnn,
                    NodeNaming {
                        wires: node_names,
                        wire_bufs: node_wire_bufs,
                        ext_pips: Default::default(),
                        bels: Default::default(),
                        iris: Default::default(),
                        intf_wires_in: Default::default(),
                        intf_wires_out: Default::default(),
                    },
                );
            } else {
                assert!(node_muxes.is_empty());
            }
            // splitters
            let mut snode_muxes = BTreeMap::new();
            let mut snode_tiles = EntityVec::new();
            let mut snode_names = BTreeMap::new();
            let snt_far = snode_tiles.push(());
            let snt_near = snode_tiles.push(());
            let bufs = self.get_bufs(tk);
            for (&wti, &wfi) in bufs.iter() {
                if bufs.get(&wfi) != Some(&wti) {
                    continue;
                }
                if let Some(wtl) = names.get(&wti) {
                    for &wt in wtl {
                        if self.db.wires[wt] != WireKind::MultiBranch(dir) {
                            continue;
                        }
                        let wf = self.main_passes[dir][wt];
                        assert!(!wires.contains_id(wt));
                        if names_far.get(&wfi) != Some(&wf) {
                            println!(
                                "WEIRD SPLITTER {} {} {}",
                                tkn, self.rd.wires[wti], self.rd.wires[wfi]
                            );
                        } else {
                            snode_names.insert((snt_near, wt), self.rd.wires[wti].clone());
                            snode_names.insert((snt_far, wf), self.rd.wires[wfi].clone());
                            snode_muxes.insert(
                                (snt_near, wt),
                                MuxInfo {
                                    kind: MuxKind::Plain,
                                    ins: [(snt_far, wf)].into_iter().collect(),
                                },
                            );
                            snode_muxes.insert(
                                (snt_far, wf),
                                MuxInfo {
                                    kind: MuxKind::Plain,
                                    ins: [(snt_near, wt)].into_iter().collect(),
                                },
                            );
                        }
                    }
                }
            }
            if let Some((nn, nnn)) = splitter_node {
                self.insert_node_merge(
                    nn,
                    NodeKind {
                        tiles: snode_tiles,
                        muxes: snode_muxes,
                        bels: Default::default(),
                        iris: Default::default(),
                        intfs: Default::default(),
                    },
                );
                self.insert_node_naming(
                    nnn,
                    NodeNaming {
                        wires: snode_names,
                        wire_bufs: Default::default(),
                        ext_pips: Default::default(),
                        bels: Default::default(),
                        iris: Default::default(),
                        intf_wires_in: Default::default(),
                        intf_wires_out: Default::default(),
                    },
                );
            } else {
                assert!(snode_muxes.is_empty());
            }
        }

        let term = TermKind { dir, wires };
        self.insert_term_merge(name.as_ref(), term);
    }

    pub fn extract_pass_simple(
        &mut self,
        name: impl AsRef<str>,
        dir: Dir,
        tkn: impl AsRef<str>,
        force_pass: &[WireId],
    ) {
        for &xy in self.rd.tiles_by_kind_name(tkn.as_ref()) {
            if let Some(int_fwd_xy) = self.walk_to_int(xy, dir, false) {
                if let Some(int_bwd_xy) = self.walk_to_int(xy, !dir, false) {
                    self.extract_pass_tile(
                        format!("{}.{}", name.as_ref(), dir),
                        dir,
                        int_bwd_xy,
                        None,
                        None,
                        None,
                        None,
                        None,
                        int_fwd_xy,
                        force_pass,
                    );
                    self.extract_pass_tile(
                        format!("{}.{}", name.as_ref(), !dir),
                        !dir,
                        int_fwd_xy,
                        None,
                        None,
                        None,
                        None,
                        None,
                        int_bwd_xy,
                        force_pass,
                    );
                }
            }
        }
    }

    pub fn extract_pass_buf(
        &mut self,
        name: impl AsRef<str>,
        dir: Dir,
        tkn: impl AsRef<str>,
        naming: impl AsRef<str>,
        force_pass: &[WireId],
    ) {
        for &xy in self.rd.tiles_by_kind_name(tkn.as_ref()) {
            if let Some(int_fwd_xy) = self.walk_to_int(xy, dir, false) {
                if let Some(int_bwd_xy) = self.walk_to_int(xy, !dir, false) {
                    let naming_fwd = format!("{}.{}", naming.as_ref(), dir);
                    let naming_bwd = format!("{}.{}", naming.as_ref(), !dir);
                    self.extract_pass_tile(
                        format!("{}.{}", name.as_ref(), dir),
                        dir,
                        int_bwd_xy,
                        Some(xy),
                        None,
                        Some(&naming_bwd),
                        None,
                        None,
                        int_fwd_xy,
                        force_pass,
                    );
                    self.extract_pass_tile(
                        format!("{}.{}", name.as_ref(), !dir),
                        !dir,
                        int_fwd_xy,
                        Some(xy),
                        None,
                        Some(&naming_fwd),
                        None,
                        None,
                        int_bwd_xy,
                        force_pass,
                    );
                }
            }
        }
    }

    pub fn make_blackhole_term(&mut self, name: impl AsRef<str>, dir: Dir, wires: &[WireId]) {
        for &w in wires {
            assert!(self.main_passes[dir].contains_id(w));
        }
        let term = TermKind {
            dir,
            wires: wires.iter().map(|&w| (w, TermInfo::BlackHole)).collect(),
        };
        match self.db.terms.get_mut(name.as_ref()) {
            None => {
                self.db.terms.insert(name.as_ref().to_string(), term);
            }
            Some((_, cterm)) => {
                assert_eq!(term, *cterm);
            }
        };
    }

    pub fn extract_intf_tile_multi(
        &mut self,
        name: impl AsRef<str>,
        xy: Coord,
        int_xy: &[Coord],
        naming: impl AsRef<str>,
        has_out_bufs: bool,
    ) {
        let mut x = self
            .xnode(name.as_ref(), naming.as_ref(), xy)
            .num_tiles(int_xy.len())
            .extract_intfs(has_out_bufs);
        for (i, &xy) in int_xy.iter().enumerate() {
            x = x.ref_int(xy, i);
        }
        x.extract();
    }

    pub fn extract_intf_tile(
        &mut self,
        name: impl AsRef<str>,
        xy: Coord,
        int_xy: Coord,
        naming: impl AsRef<str>,
        has_out_bufs: bool,
    ) {
        self.extract_intf_tile_multi(name, xy, &[int_xy], naming, has_out_bufs);
    }

    pub fn extract_intf(
        &mut self,
        name: impl AsRef<str>,
        dir: Dir,
        tkn: impl AsRef<str>,
        naming: impl AsRef<str>,
        has_out_bufs: bool,
    ) {
        for &xy in self.rd.tiles_by_kind_name(tkn.as_ref()) {
            let int_xy = self.walk_to_int(xy, !dir, false).unwrap();
            self.extract_intf_tile(name.as_ref(), xy, int_xy, naming.as_ref(), has_out_bufs);
        }
    }

    pub fn extract_xnode(
        &mut self,
        name: &str,
        xy: Coord,
        buf_xy: &[Coord],
        int_xy: &[Coord],
        naming: &str,
        bels: &[ExtrBelInfo],
        skip_wires: &[WireId],
    ) {
        let mut x = self
            .xnode(name, naming, xy)
            .num_tiles(int_xy.len())
            .extract_muxes()
            .skip_muxes(skip_wires);
        for &xy in buf_xy {
            x = x.raw_tile(xy);
        }
        for (i, &xy) in int_xy.iter().enumerate() {
            x = x.ref_int(xy, i);
        }
        for bel in bels {
            x = x.bel(bel.clone());
        }
        x.extract();
    }

    pub fn extract_xnode_bels(
        &mut self,
        name: &str,
        xy: Coord,
        buf_xy: &[Coord],
        int_xy: &[Coord],
        naming: &str,
        bels: &[ExtrBelInfo],
    ) {
        let mut x = self.xnode(name, naming, xy).num_tiles(int_xy.len());
        for &xy in buf_xy {
            x = x.raw_tile(xy);
        }
        for (i, &xy) in int_xy.iter().enumerate() {
            x = x.ref_int(xy, i);
        }
        for bel in bels {
            x = x.bel(bel.clone());
        }
        x.extract();
    }

    pub fn extract_xnode_bels_intf(
        &mut self,
        name: &str,
        xy: Coord,
        buf_xy: &[Coord],
        int_xy: &[Coord],
        intf_xy: &[(Coord, NodeNamingId)],
        naming: &str,
        bels: &[ExtrBelInfo],
    ) {
        let mut x = self
            .xnode(name, naming, xy)
            .num_tiles(Ord::max(int_xy.len(), intf_xy.len()));
        for &xy in buf_xy {
            x = x.raw_tile(xy);
        }
        for (i, &xy) in int_xy.iter().enumerate() {
            x = x.ref_int(xy, i);
        }
        for (i, &(xy, naming)) in intf_xy.iter().enumerate() {
            x = x.ref_single(xy, i, naming);
        }
        for bel in bels {
            x = x.bel(bel.clone());
        }
        x.extract();
    }

    pub fn make_marker_bel(&mut self, name: &str, naming: &str, bel: &str, ntiles: usize) {
        let mut bels = EntityMap::new();
        bels.insert(
            bel.to_string(),
            BelInfo {
                pins: Default::default(),
            },
        );
        let mut naming_bels = EntityVec::new();
        naming_bels.push(BelNaming {
            tile: NodeRawTileId::from_idx(0),
            pins: Default::default(),
        });
        let node = NodeKind {
            tiles: (0..ntiles).map(|_| ()).collect(),
            muxes: Default::default(),
            bels,
            iris: Default::default(),
            intfs: Default::default(),
        };
        let node_naming = NodeNaming {
            wires: Default::default(),
            wire_bufs: Default::default(),
            ext_pips: Default::default(),
            bels: naming_bels,
            iris: Default::default(),
            intf_wires_in: Default::default(),
            intf_wires_out: Default::default(),
        };
        self.insert_node_merge(name, node);
        self.insert_node_naming(naming, node_naming);
    }

    pub fn xnode<'b>(
        &'b mut self,
        kind: impl Into<String>,
        naming: impl Into<String>,
        tile: Coord,
    ) -> XNodeInfo<'a, 'b> {
        XNodeInfo {
            builder: self,
            kind: kind.into(),
            naming: naming.into(),
            raw_tiles: vec![XNodeRawTile {
                xy: tile,
                tile_map: None,
                extract_muxes: false,
            }],
            num_tiles: 1,
            refs: vec![],
            extract_intfs: false,
            has_intf_out_bufs: false,
            iris: EntityVec::new(),
            skip_muxes: BTreeSet::new(),
            optin_muxes: BTreeSet::new(),
            optin_muxes_tile: BTreeSet::new(),
            bels: vec![],
        }
    }

    pub fn build(self) -> IntDb {
        self.db
    }
}
