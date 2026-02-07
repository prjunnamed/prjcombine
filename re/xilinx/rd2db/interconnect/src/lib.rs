#![allow(clippy::too_many_arguments)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, btree_map};

use prjcombine_entity::{EntityBundleIndex, EntityId, EntityPartVec, EntityVec};
use prjcombine_interconnect::{
    db::{
        Bel, BelInfo, BelInput, BelKind, BelPin, BelSlotId, BiPass, CellSlotId, ConnectorClass,
        ConnectorClassId, ConnectorSlotId, ConnectorWire, IntDb, LegacyBel, Mux, Pass, PermaBuf,
        PinDir, PolTileWireCoord, ProgBuf, ProgDelay, ProgInv, SwitchBox, SwitchBoxItem, TestMux,
        TestMuxWire, TileClassId, TileWireCoord, WireKind, WireSlotId,
    },
    dir::{Dir, DirMap},
};
use prjcombine_re_xilinx_naming::db::{
    BelNaming, BelPinNaming, ConnectorClassNamingId, ConnectorWireInFarNaming,
    ConnectorWireOutNaming, IntfWireInNaming, NamingDb, PipNaming, RawTileId, TileClassNaming,
    TileClassNamingId, WireNaming,
};
use prjcombine_re_xilinx_rawdump::{self as rawdump, Coord, NodeOrWire, Part};

use assert_matches::assert_matches;

use prjcombine_types::bsdata::PolTileBit;
use rawdump::TileKindId;

#[derive(Default, Debug)]
pub struct XTileResult {
    pub bels: Vec<(LegacyBel, BelNaming)>,
}

#[derive(Clone, Debug)]
struct ExtrBelInfoSub {
    slot: Option<rawdump::TkSiteSlot>,
    pin_renames: HashMap<String, String>,
    pins: HashMap<String, BelPinInfo>,
    raw_tile: usize,
}

#[derive(Clone, Debug)]
pub struct ExtrBelInfo {
    pub bel: BelSlotId,
    pub manual: bool,
    subs: Vec<ExtrBelInfoSub>,
}

#[derive(Clone, Debug)]
pub enum BelPinInfo {
    Int,
    NameOnly(usize),
    IntNudge(TileWireCoord),
    ForceInt(TileWireCoord, String),
    ExtraInt(PinDir, Vec<String>),
    ExtraIntForce(PinDir, TileWireCoord, String),
    ExtraWire(Vec<String>),
    ExtraWireForce(String, Vec<PipNaming>),
    Dummy,
}

#[derive(Debug)]
pub struct XTileRawTile {
    pub xy: Coord,
    pub tile_map: Option<EntityPartVec<CellSlotId, CellSlotId>>,
    pub extract_muxes: bool,
}

#[derive(Debug)]
pub struct XTileRef {
    pub xy: Coord,
    pub naming: Option<TileClassNamingId>,
    pub tile_map: EntityPartVec<CellSlotId, CellSlotId>,
}

pub struct XTileInfo<'a, 'b> {
    pub builder: &'b mut IntBuilder<'a>,
    pub tcid: TileClassId,
    pub naming: String,
    pub raw_tiles: Vec<XTileRawTile>,
    pub num_tiles: usize,
    pub refs: Vec<XTileRef>,
    pub tmux_bel: Option<BelSlotId>,
    pub intf_sb: Option<BelSlotId>,
    pub extract_delay: bool,
    pub has_intf_out_bufs: bool,
    pub skip_muxes: BTreeSet<WireSlotId>,
    pub optin_muxes: BTreeSet<WireSlotId>,
    pub optin_muxes_tile: BTreeSet<TileWireCoord>,
    pub bels: Vec<ExtrBelInfo>,
    pub force_names: HashMap<(usize, rawdump::WireId), (IntConnKind, TileWireCoord)>,
    pub force_skip_pips: HashSet<(TileWireCoord, TileWireCoord)>,
    pub force_pips: HashSet<(TileWireCoord, PolTileWireCoord)>,
    pub switchbox: Option<BelSlotId>,
    pub force_test_mux_in: bool,
    pub skip_edges: HashSet<(rawdump::WireId, rawdump::WireId)>,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum IntConnKind {
    Raw,
    IntfIn,
}

impl ExtrBelInfo {
    pub fn manual(mut self) -> Self {
        self.manual = true;
        self
    }

    pub fn pin_rename(mut self, name_from: impl Into<String>, name_to: impl Into<String>) -> Self {
        self.subs
            .last_mut()
            .unwrap()
            .pin_renames
            .insert(name_from.into(), name_to.into());
        self
    }

    pub fn pins_name_only(mut self, names: &[impl AsRef<str>]) -> Self {
        for name in names {
            self.subs
                .last_mut()
                .unwrap()
                .pins
                .insert(name.as_ref().to_string(), BelPinInfo::NameOnly(0));
        }
        self
    }

    pub fn pin_name_only(mut self, name: &str, buf_cnt: usize) -> Self {
        self.subs
            .last_mut()
            .unwrap()
            .pins
            .insert(name.to_string(), BelPinInfo::NameOnly(buf_cnt));
        self
    }

    pub fn pin_dummy(mut self, name: impl Into<String>) -> Self {
        self.subs
            .last_mut()
            .unwrap()
            .pins
            .insert(name.into(), BelPinInfo::Dummy);
        self
    }

    pub fn pin_force_int(
        mut self,
        name: &str,
        wire: TileWireCoord,
        wname: impl Into<String>,
    ) -> Self {
        self.subs
            .last_mut()
            .unwrap()
            .pins
            .insert(name.to_string(), BelPinInfo::ForceInt(wire, wname.into()));
        self
    }

    pub fn pin_int_nudge(mut self, name: &str, wire: TileWireCoord) -> Self {
        self.subs
            .last_mut()
            .unwrap()
            .pins
            .insert(name.to_string(), BelPinInfo::IntNudge(wire));
        self
    }

    pub fn extra_int_out(
        mut self,
        name: impl Into<String>,
        wire_names: &[impl AsRef<str>],
    ) -> Self {
        self.subs.last_mut().unwrap().pins.insert(
            name.into(),
            BelPinInfo::ExtraInt(
                PinDir::Output,
                wire_names.iter().map(|x| x.as_ref().to_string()).collect(),
            ),
        );
        self
    }

    pub fn extra_int_in(mut self, name: impl Into<String>, wire_names: &[impl AsRef<str>]) -> Self {
        self.subs.last_mut().unwrap().pins.insert(
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
        self.subs.last_mut().unwrap().pins.insert(
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
        wire: TileWireCoord,
        wire_name: impl Into<String>,
    ) -> Self {
        self.subs.last_mut().unwrap().pins.insert(
            name.into(),
            BelPinInfo::ExtraIntForce(PinDir::Output, wire, wire_name.into()),
        );
        self
    }

    pub fn extra_int_in_force(
        mut self,
        name: impl Into<String>,
        wire: TileWireCoord,
        wire_name: impl Into<String>,
    ) -> Self {
        self.subs.last_mut().unwrap().pins.insert(
            name.into(),
            BelPinInfo::ExtraIntForce(PinDir::Input, wire, wire_name.into()),
        );
        self
    }

    pub fn extra_wire(mut self, name: impl Into<String>, wire_names: &[impl AsRef<str>]) -> Self {
        self.subs.last_mut().unwrap().pins.insert(
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
        self.subs.last_mut().unwrap().pins.insert(
            name.into(),
            BelPinInfo::ExtraWireForce(wire_name.into(), vec![]),
        );
        self
    }

    pub fn extra_wire_force_pip(
        mut self,
        name: impl Into<String>,
        wire_name: impl Into<String>,
        pip: PipNaming,
    ) -> Self {
        self.subs.last_mut().unwrap().pins.insert(
            name.into(),
            BelPinInfo::ExtraWireForce(wire_name.into(), vec![pip]),
        );
        self
    }

    pub fn sub_indexed(mut self, rd: &rawdump::Part, slot: &str, idx: usize) -> Self {
        self.subs.push(ExtrBelInfoSub {
            slot: Some(rawdump::TkSiteSlot::Indexed(
                rd.slot_kinds.get(slot).unwrap(),
                idx as u8,
            )),
            pins: Default::default(),
            pin_renames: Default::default(),
            raw_tile: 0,
        });
        self
    }

    pub fn sub_xy(mut self, rd: &rawdump::Part, slot: &str, x: usize, y: usize) -> Self {
        self.subs.push(ExtrBelInfoSub {
            slot: Some(rawdump::TkSiteSlot::Xy(
                rd.slot_kinds.get(slot).expect("missing slot kind"),
                x as u8,
                y as u8,
            )),
            pins: Default::default(),
            pin_renames: Default::default(),
            raw_tile: 0,
        });
        self
    }

    pub fn sub_virtual(mut self) -> Self {
        self.subs.push(ExtrBelInfoSub {
            slot: None,
            pins: Default::default(),
            pin_renames: Default::default(),
            raw_tile: 0,
        });
        self
    }

    pub fn raw_tile(mut self, idx: usize) -> Self {
        self.subs.last_mut().unwrap().raw_tile = idx;
        self
    }
}

impl XTileInfo<'_, '_> {
    pub fn raw_tile(mut self, xy: Coord) -> Self {
        self.raw_tiles.push(XTileRawTile {
            xy,
            tile_map: None,
            extract_muxes: false,
        });
        self
    }

    pub fn raw_tile_single(mut self, xy: Coord, slot: usize) -> Self {
        self.raw_tiles.push(XTileRawTile {
            xy,
            tile_map: Some(
                [(CellSlotId::from_idx(0), CellSlotId::from_idx(slot))]
                    .into_iter()
                    .collect(),
            ),
            extract_muxes: false,
        });
        self
    }

    pub fn ref_int(mut self, xy: Coord, slot: usize) -> Self {
        self.refs.push(XTileRef {
            xy,
            naming: None,
            tile_map: [(CellSlotId::from_idx(0), CellSlotId::from_idx(slot))]
                .into_iter()
                .collect(),
        });
        self
    }

    pub fn ref_single(mut self, xy: Coord, slot: usize, naming: TileClassNamingId) -> Self {
        self.refs.push(XTileRef {
            xy,
            naming: Some(naming),
            tile_map: [(CellSlotId::from_idx(0), CellSlotId::from_idx(slot))]
                .into_iter()
                .collect(),
        });
        self
    }

    pub fn ref_xlat(
        mut self,
        xy: Coord,
        slots: &[Option<usize>],
        naming: TileClassNamingId,
    ) -> Self {
        self.refs.push(XTileRef {
            xy,
            naming: Some(naming),
            tile_map: slots
                .iter()
                .enumerate()
                .filter_map(|(i, x)| x.map(|x| (CellSlotId::from_idx(i), CellSlotId::from_idx(x))))
                .collect(),
        });
        self
    }

    pub fn switchbox(mut self, sb: BelSlotId) -> Self {
        self.switchbox = Some(sb);
        self
    }

    pub fn extract_muxes(mut self, sb: BelSlotId) -> Self {
        self.switchbox = Some(sb);
        self.raw_tiles[0].extract_muxes = true;
        self
    }

    pub fn extract_muxes_rt(mut self, sb: BelSlotId, rt: usize) -> Self {
        self.switchbox = Some(sb);
        self.raw_tiles[rt].extract_muxes = true;
        self
    }

    pub fn extract_delay(mut self) -> Self {
        self.extract_delay = true;
        self
    }

    pub fn extract_intfs(
        mut self,
        tmux_bel: BelSlotId,
        sb_bel: Option<BelSlotId>,
        has_out_bufs: bool,
    ) -> Self {
        self.tmux_bel = Some(tmux_bel);
        self.intf_sb = sb_bel;
        self.has_intf_out_bufs = has_out_bufs;
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

    pub fn skip_muxes<'a>(mut self, wires: impl IntoIterator<Item = &'a WireSlotId>) -> Self {
        self.skip_muxes.extend(wires.into_iter().copied());
        self
    }

    pub fn optin_muxes<'a>(mut self, wires: impl IntoIterator<Item = &'a WireSlotId>) -> Self {
        self.optin_muxes.extend(wires.into_iter().copied());
        self
    }

    pub fn optin_muxes_tile<'a>(
        mut self,
        wires: impl IntoIterator<Item = &'a TileWireCoord>,
    ) -> Self {
        self.optin_muxes_tile.extend(wires.into_iter().copied());
        self
    }

    pub fn num_cells(mut self, num: usize) -> Self {
        self.num_tiles = num;
        self
    }

    pub fn force_name(mut self, rti: usize, name: &str, wire: TileWireCoord) -> Self {
        if let Some(w) = self.builder.rd.wires.get(name) {
            self.force_names.insert((rti, w), (IntConnKind::Raw, wire));
        }
        self
    }

    pub fn force_skip_pip(mut self, wt: TileWireCoord, wf: TileWireCoord) -> Self {
        self.force_skip_pips.insert((wt, wf));
        self
    }

    pub fn force_pip(mut self, wt: TileWireCoord, wf: PolTileWireCoord) -> Self {
        self.force_pips.insert((wt, wf));
        self
    }

    pub fn force_test_mux_in(mut self) -> Self {
        self.force_test_mux_in = true;
        self
    }

    pub fn skip_edge(mut self, wt: &str, wf: &str) -> Self {
        if let Some(wti) = self.builder.rd.wires.get(wt)
            && let Some(wfi) = self.builder.rd.wires.get(wf)
        {
            self.skip_edges.insert((wti, wfi));
        }
        self
    }

    pub fn extract(self) -> XTileResult {
        let rd = self.builder.rd;

        let mut names: HashMap<NodeOrWire, (IntConnKind, TileWireCoord)> = HashMap::new();

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
                        w.cell = tile_map[w.cell];
                    } else if self.num_tiles == 1 {
                        w.cell = CellSlotId::from_idx(0);
                    }
                    names.entry(nw).or_insert((IntConnKind::Raw, w));
                }
            }
            for &(wfi, wti) in tk.pips.keys() {
                if self.skip_edges.contains(&(wti, wfi)) {
                    continue;
                }
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
                let naming = &self.builder.ndb.tile_class_namings[naming];
                for (&k, wn) in &naming.wires {
                    if round == 0
                        && matches!(
                            self.builder.db[k.wire],
                            WireKind::Branch(_) | WireKind::MultiBranch(_)
                        )
                    {
                        continue;
                    }
                    if let Some(&wf) = self.builder.test_mux_ins.get(&k.wire) {
                        let wf = TileWireCoord {
                            cell: k.cell,
                            wire: wf,
                        };
                        if let Some(wfn) = naming.wires.get(&wf)
                            && wfn.name == wn.name
                        {
                            continue;
                        }
                    }
                    if let Some(nw) = rd.lookup_wire(r.xy, &wn.name)
                        && let Some(&ti) = r.tile_map.get(k.cell)
                    {
                        names.entry(nw).or_insert((
                            IntConnKind::Raw,
                            TileWireCoord {
                                cell: ti,
                                wire: k.wire,
                            },
                        ));
                    }
                }
                for (&k, v) in &naming.intf_wires_in {
                    match v {
                        IntfWireInNaming::Simple { name: n }
                        | IntfWireInNaming::TestBuf { name_in: n, .. } => {
                            if let Some(nw) = rd.lookup_wire(r.xy, n) {
                                names.entry(nw).or_insert((
                                    IntConnKind::Raw,
                                    TileWireCoord {
                                        cell: r.tile_map[k.cell],
                                        wire: k.wire,
                                    },
                                ));
                            }
                        }
                        IntfWireInNaming::Buf { name_out: n, .. } => {
                            if let Some(nw) = rd.lookup_wire(r.xy, n) {
                                names.entry(nw).or_insert((
                                    IntConnKind::IntfIn,
                                    TileWireCoord {
                                        cell: r.tile_map[k.cell],
                                        wire: k.wire,
                                    },
                                ));
                            }
                        }
                        IntfWireInNaming::Anonymous => (),
                    }
                }

                for &wi in tk.wires.keys() {
                    if let Some(nw) = rd.lookup_wire_raw(r.xy, wi)
                        && let Some(w) = self.builder.get_wire_by_name(tile.kind, &rd.wires[wi])
                    {
                        if round == 0
                            && matches!(
                                self.builder.db[w.wire],
                                WireKind::Branch(_) | WireKind::MultiBranch(_)
                            )
                        {
                            continue;
                        }
                        if let Some(&t) = r.tile_map.get(w.cell) {
                            names.entry(nw).or_insert((
                                IntConnKind::Raw,
                                TileWireCoord {
                                    cell: t,
                                    wire: w.wire,
                                },
                            ));
                            continue;
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

        let mut extractor = XTileExtractor {
            rd: self.builder.rd,
            db: &self.builder.db,
            xtile: &self,
            names,
            buf_out,
            buf_in,
            int_out,
            int_in,
            bels: Default::default(),
            tcls_naming: TileClassNaming::default(),
            alt_names: Default::default(),
            alt_pips_to: Default::default(),
            alt_pips_from: Default::default(),
            pips: vec![],
            result: XTileResult::default(),
        };

        if self.raw_tiles.iter().any(|x| x.extract_muxes)
            || !self.optin_muxes.is_empty()
            || !self.optin_muxes_tile.is_empty()
        {
            extractor.extract_muxes();
        }

        extractor.extract_delay();
        if self.tmux_bel.is_some() {
            extractor.extract_tmux();
        }

        for bel in &self.bels {
            extractor.extract_bel(bel);
        }

        let mut tcls_naming = extractor.tcls_naming;

        for (wire, wn) in &mut tcls_naming.wires {
            wn.alt_name = extractor.alt_names.remove(wire);
            wn.alt_pips_to = extractor.alt_pips_to.remove(wire).unwrap_or_default();
            wn.alt_pips_from = extractor.alt_pips_from.remove(wire).unwrap_or_default();
        }

        let pips = extractor.pips;
        let result = extractor.result;
        for (bslot, bel) in extractor.bels {
            self.builder.insert_tcls_bel(self.tcid, bslot, bel);
        }
        for (bslot, pips) in pips {
            self.builder.insert_tcls_pips(self.tcid, bslot, pips);
        }
        self.builder.insert_tcls_naming(&self.naming, tcls_naming);

        result
    }

    pub fn name(&self) -> &str {
        self.builder.db.tile_classes.key(self.tcid)
    }
}

#[allow(clippy::type_complexity)]
struct XTileExtractor<'a, 'b, 'c> {
    rd: &'c Part,
    db: &'c IntDb,
    xtile: &'a XTileInfo<'b, 'c>,
    names: HashMap<NodeOrWire, (IntConnKind, TileWireCoord)>,
    buf_out: HashMap<NodeOrWire, (NodeOrWire, usize, rawdump::WireId, rawdump::WireId)>,
    buf_in: HashMap<NodeOrWire, (NodeOrWire, usize, rawdump::WireId, rawdump::WireId)>,
    int_out: HashMap<
        NodeOrWire,
        Vec<(
            IntConnKind,
            TileWireCoord,
            usize,
            rawdump::WireId,
            rawdump::WireId,
        )>,
    >,
    int_in: HashMap<
        NodeOrWire,
        (
            IntConnKind,
            TileWireCoord,
            usize,
            rawdump::WireId,
            rawdump::WireId,
        ),
    >,
    bels: BTreeMap<BelSlotId, BelInfo>,
    tcls_naming: TileClassNaming,
    alt_names: BTreeMap<TileWireCoord, String>,
    alt_pips_to: BTreeMap<TileWireCoord, BTreeSet<TileWireCoord>>,
    alt_pips_from: BTreeMap<TileWireCoord, BTreeSet<TileWireCoord>>,
    pips: Vec<(BelSlotId, Pips)>,
    result: XTileResult,
}

impl XTileExtractor<'_, '_, '_> {
    fn walk_to_int(
        &self,
        pin: &str,
        dir: PinDir,
        tile: usize,
        wire: rawdump::WireId,
    ) -> (
        IntConnKind,
        BTreeSet<TileWireCoord>,
        rawdump::WireId,
        Vec<PipNaming>,
        BTreeMap<TileWireCoord, PipNaming>,
    ) {
        let mut wn = wire;
        let mut nw = self
            .rd
            .lookup_wire_raw_force(self.xtile.raw_tiles[tile].xy, wire);
        let mut pips = Vec::new();
        loop {
            if let Some(&(ick, w)) = self.names.get(&nw) {
                return (ick, [w].into_iter().collect(), wn, pips, BTreeMap::new());
            }
            match dir {
                PinDir::Input => {
                    if let Some(&(ick, w, rt, wt, wf)) = self.int_in.get(&nw) {
                        pips.push(PipNaming {
                            tile: RawTileId::from_idx(rt),
                            wire_to: self.rd.wires[wt].clone(),
                            wire_from: self.rd.wires[wf].clone(),
                        });
                        if rt == tile {
                            wn = wf;
                        }
                        return (ick, [w].into_iter().collect(), wn, pips, BTreeMap::new());
                    }
                    if let Some(&(nnw, rt, wt, wf)) = self.buf_in.get(&nw) {
                        pips.push(PipNaming {
                            tile: RawTileId::from_idx(rt),
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
                        self.rd.part,
                        self.xtile.name(),
                        pin
                    );
                }
                PinDir::Output => {
                    if let Some(nxt) = self.int_out.get(&nw) {
                        if nxt.len() == 1 {
                            let (ick, mut w, rt, wt, wf) = nxt[0];
                            if self.xtile.force_test_mux_in
                                && let Some(&nw) = self.xtile.builder.test_mux_ins.get(&w.wire)
                            {
                                w.wire = nw;
                            }
                            pips.push(PipNaming {
                                tile: RawTileId::from_idx(rt),
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
                            for &(cick, mut w, rt, wt, wf) in nxt {
                                ick = Some(cick);
                                if self.xtile.force_test_mux_in
                                    && let Some(&nw) = self.xtile.builder.test_mux_ins.get(&w.wire)
                                {
                                    w.wire = nw;
                                }
                                wires.insert(w);
                                int_pips.insert(
                                    w,
                                    PipNaming {
                                        tile: RawTileId::from_idx(rt),
                                        wire_to: self.rd.wires[wt].clone(),
                                        wire_from: self.rd.wires[wf].clone(),
                                    },
                                );
                            }
                            return (ick.unwrap(), wires, wn, pips, int_pips);
                        }
                    }
                    if let Some(&(nnw, rt, wt, wf)) = self.buf_out.get(&nw) {
                        pips.push(PipNaming {
                            tile: RawTileId::from_idx(rt),
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
                        self.rd.part,
                        self.xtile.name(),
                        pin
                    );
                }
                PinDir::Inout => {
                    panic!(
                        "CANNOT WALK INOUT WIRE {} {} {}",
                        self.rd.part,
                        self.xtile.name(),
                        pin
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
    ) -> (rawdump::WireId, Vec<PipNaming>) {
        let mut wn = wire;
        let mut nw = self
            .rd
            .lookup_wire_raw_force(self.xtile.raw_tiles[tile].xy, wire);
        let mut pips = Vec::new();
        for _ in 0..cnt {
            match dir {
                PinDir::Input => {
                    if let Some(&(nnw, rt, wt, wf)) = self.buf_in.get(&nw) {
                        pips.push(PipNaming {
                            tile: RawTileId::from_idx(rt),
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
                        pips.push(PipNaming {
                            tile: RawTileId::from_idx(rt),
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
                self.rd.part,
                self.xtile.name(),
                pin
            );
        }
        (wn, pips)
    }

    fn extract_subbel(
        &mut self,
        bel: &mut LegacyBel,
        naming: &mut BelNaming,
        sub: &ExtrBelInfoSub,
    ) {
        let rt = RawTileId::from_idx(sub.raw_tile);
        naming.tiles.push(rt);
        let crd = self.xtile.raw_tiles[sub.raw_tile].xy;
        let tile = &self.rd.tiles[&crd];
        let tk = &self.rd.tile_kinds[tile.kind];
        if let Some(slot) = sub.slot {
            let tks = tk.sites.get(&slot).expect("missing site slot in tk").1;
            for (name, tksp) in &tks.pins {
                let name = sub.pin_renames.get(name).unwrap_or(name);
                match sub.pins.get(name).unwrap_or(&BelPinInfo::Int) {
                    &BelPinInfo::Int => {
                        let dir = match tksp.dir {
                            rawdump::TkSitePinDir::Input => PinDir::Input,
                            rawdump::TkSitePinDir::Output => PinDir::Output,
                            _ => panic!("bidir pin {name}"),
                        };
                        if tksp.wire.is_none() {
                            panic!(
                                "missing site wire for pin {name} tile {tile}",
                                tile = self.xtile.name()
                            );
                        }
                        let (ick, wires, wnf, pips, int_pips) =
                            self.walk_to_int(name, dir, sub.raw_tile, tksp.wire.unwrap());
                        naming.pins.insert(
                            name.clone(),
                            BelPinNaming {
                                tile: rt,
                                name: self.rd.wires[tksp.wire.unwrap()].clone(),
                                name_far: self.rd.wires[wnf].clone(),
                                pips,
                                int_pips,
                                is_intf: ick != IntConnKind::Raw,
                            },
                        );
                        bel.pins.insert(name.clone(), BelPin { wires, dir });
                    }
                    &BelPinInfo::IntNudge(wire) => {
                        let dir = match tksp.dir {
                            rawdump::TkSitePinDir::Input => PinDir::Input,
                            rawdump::TkSitePinDir::Output => PinDir::Output,
                            _ => panic!("bidir pin {name}"),
                        };
                        if tksp.wire.is_none() {
                            panic!(
                                "missing site wire for pin {name} tile {tile}",
                                tile = self.xtile.name()
                            );
                        }
                        let (ick, _wires, wnf, pips, int_pips) =
                            self.walk_to_int(name, dir, sub.raw_tile, tksp.wire.unwrap());
                        naming.pins.insert(
                            name.clone(),
                            BelPinNaming {
                                tile: rt,
                                name: self.rd.wires[tksp.wire.unwrap()].clone(),
                                name_far: self.rd.wires[wnf].clone(),
                                pips,
                                int_pips,
                                is_intf: ick != IntConnKind::Raw,
                            },
                        );
                        bel.pins.insert(
                            name.clone(),
                            BelPin {
                                wires: BTreeSet::from_iter([wire]),
                                dir,
                            },
                        );
                    }
                    &BelPinInfo::ForceInt(wire, ref wname) => {
                        let dir = match tksp.dir {
                            rawdump::TkSitePinDir::Input => PinDir::Input,
                            rawdump::TkSitePinDir::Output => PinDir::Output,
                            _ => panic!("bidir pin {name}"),
                        };
                        naming.pins.insert(
                            name.clone(),
                            BelPinNaming {
                                tile: rt,
                                name: self.rd.wires[tksp.wire.unwrap()].clone(),
                                name_far: wname.clone(),
                                pips: Vec::new(),
                                int_pips: BTreeMap::new(),
                                is_intf: false,
                            },
                        );
                        bel.pins.insert(
                            name.clone(),
                            BelPin {
                                wires: [wire].into_iter().collect(),
                                dir,
                            },
                        );
                    }
                    &BelPinInfo::NameOnly(buf_cnt) => {
                        if tksp.wire.is_none() {
                            panic!(
                                "missing site wire for pin {name} tile {tile}",
                                tile = self.xtile.name()
                            );
                        }
                        if buf_cnt == 0 {
                            naming.pins.insert(
                                name.clone(),
                                BelPinNaming {
                                    tile: rt,
                                    name: self.rd.wires[tksp.wire.unwrap()].clone(),
                                    name_far: self.rd.wires[tksp.wire.unwrap()].clone(),
                                    pips: Vec::new(),
                                    int_pips: BTreeMap::new(),
                                    is_intf: false,
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
                                sub.raw_tile,
                                tksp.wire.unwrap(),
                            );
                            naming.pins.insert(
                                name.clone(),
                                BelPinNaming {
                                    tile: rt,
                                    name: self.rd.wires[tksp.wire.unwrap()].clone(),
                                    name_far: self.rd.wires[wn].clone(),
                                    pips,
                                    int_pips: BTreeMap::new(),
                                    is_intf: false,
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
        for (name, pd) in &sub.pins {
            match *pd {
                BelPinInfo::ExtraInt(dir, ref names) => {
                    let mut wn = None;
                    for w in names {
                        if let Some(w) = self.rd.wires.get(w)
                            && tk.wires.contains_key(&w)
                        {
                            assert!(wn.is_none());
                            wn = Some(w);
                        }
                    }
                    if wn.is_none() {
                        println!("NOT FOUND: {name}");
                    }
                    let wn = wn.unwrap();
                    let (ick, wires, wnf, pips, int_pips) =
                        self.walk_to_int(name, dir, sub.raw_tile, wn);
                    naming.pins.insert(
                        name.clone(),
                        BelPinNaming {
                            tile: rt,
                            name: self.rd.wires[wn].clone(),
                            name_far: self.rd.wires[wnf].clone(),
                            pips,
                            int_pips,
                            is_intf: ick != IntConnKind::Raw,
                        },
                    );
                    bel.pins.insert(name.clone(), BelPin { wires, dir });
                }
                BelPinInfo::ExtraIntForce(dir, wire, ref wname) => {
                    naming.pins.insert(
                        name.clone(),
                        BelPinNaming {
                            tile: rt,
                            name: wname.clone(),
                            name_far: wname.clone(),
                            pips: vec![],
                            int_pips: BTreeMap::new(),
                            is_intf: false,
                        },
                    );
                    bel.pins.insert(
                        name.clone(),
                        BelPin {
                            wires: [wire].into_iter().collect(),
                            dir,
                        },
                    );
                }
                BelPinInfo::ExtraWire(ref names) => {
                    let mut wn = None;
                    for w in names {
                        if let Some(w) = self.rd.wires.get(w)
                            && tk.wires.contains_key(&w)
                        {
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
                    if wn.is_none() {
                        println!("NOT FOUND: {name}");
                    }
                    let wn = wn.unwrap();
                    naming.pins.insert(
                        name.clone(),
                        BelPinNaming {
                            tile: rt,
                            name: self.rd.wires[wn].clone(),
                            name_far: self.rd.wires[wn].clone(),
                            pips: Vec::new(),
                            int_pips: BTreeMap::new(),
                            is_intf: false,
                        },
                    );
                }
                BelPinInfo::ExtraWireForce(ref wname, ref pips) => {
                    naming.pins.insert(
                        name.clone(),
                        BelPinNaming {
                            tile: rt,
                            name: wname.clone(),
                            name_far: wname.clone(),
                            pips: pips.clone(),
                            int_pips: BTreeMap::new(),
                            is_intf: false,
                        },
                    );
                }
                _ => (),
            }
        }
    }

    fn extract_bel(&mut self, info: &ExtrBelInfo) {
        let mut bel = LegacyBel::default();
        let mut naming = BelNaming {
            tiles: vec![],
            pins: Default::default(),
        };
        for sub in &info.subs {
            self.extract_subbel(&mut bel, &mut naming, sub);
        }
        if info.manual {
            self.result.bels.push((bel, naming));
        } else {
            self.bels.insert(info.bel, BelInfo::Legacy(bel));
            self.tcls_naming.bels.insert(info.bel, naming);
        }
    }

    fn get_wire_by_name(&self, rti: usize, name: rawdump::WireId) -> Option<TileWireCoord> {
        let rt = &self.xtile.raw_tiles[rti];
        let tile = &self.rd.tiles[&rt.xy];
        if let Some(&(IntConnKind::Raw, res)) = self.xtile.force_names.get(&(rti, name)) {
            return Some(res);
        }
        if let Some(TileWireCoord { cell: t, wire: w }) = self
            .xtile
            .builder
            .get_wire_by_name(tile.kind, &self.rd.wires[name])
            && let Some(&xt) = rt.tile_map.as_ref().and_then(|x| x.get(t))
        {
            return Some(TileWireCoord { cell: xt, wire: w });
        }
        let nw = self.rd.lookup_wire_raw_force(rt.xy, name);
        if let Some(&(_, w)) = self.names.get(&nw) {
            return Some(w);
        }
        None
    }

    fn get_alt_by_name(&self, rti: usize, name: rawdump::WireId) -> Option<TileWireCoord> {
        let rt = &self.xtile.raw_tiles[rti];
        if let Some(&TileWireCoord { cell: t, wire: w }) =
            self.xtile.builder.alt_names.get(&self.rd.wires[name])
        {
            if let Some(ref tm) = rt.tile_map {
                if let Some(&xt) = tm.get(t) {
                    return Some(TileWireCoord { cell: xt, wire: w });
                }
            } else {
                return Some(TileWireCoord { cell: t, wire: w });
            }
        }
        None
    }

    fn extract_muxes(&mut self) {
        let mut pips = Pips::default();
        for &(wt, wf) in &self.xtile.force_pips {
            let mode = self.xtile.builder.pip_mode(wt.wire);
            pips.pips.insert((wt, wf), mode);
        }
        for (i, rt) in self.xtile.raw_tiles.iter().enumerate() {
            let tile = &self.rd.tiles[&rt.xy];
            let tk = &self.rd.tile_kinds[tile.kind];

            for &(wfi, wti) in tk.pips.keys() {
                if let Some(wt) = self.get_wire_by_name(i, wti) {
                    let mut pass = rt.extract_muxes
                        && !matches!(self.db[wt.wire], WireKind::BelOut)
                        && !self.xtile.skip_muxes.contains(&wt.wire);
                    if self.xtile.optin_muxes.contains(&wt.wire) {
                        pass = true;
                    }
                    if self.xtile.optin_muxes_tile.contains(&wt) {
                        pass = true;
                    }
                    if !pass {
                        continue;
                    }
                    if let Some(wf) = self.get_wire_by_name(i, wfi) {
                        if self.xtile.force_skip_pips.contains(&(wt, wf)) {
                            continue;
                        }
                        if i == 0 {
                            self.tcls_naming.wires.insert(
                                wt,
                                WireNaming {
                                    name: self.rd.wires[wti].to_string(),
                                    alt_name: None,
                                    alt_pips_to: Default::default(),
                                    alt_pips_from: Default::default(),
                                },
                            );
                            self.tcls_naming.wires.insert(
                                wf,
                                WireNaming {
                                    name: self.rd.wires[wfi].to_string(),
                                    alt_name: None,
                                    alt_pips_to: Default::default(),
                                    alt_pips_from: Default::default(),
                                },
                            );
                        } else {
                            self.tcls_naming.ext_pips.insert(
                                (wt, wf),
                                PipNaming {
                                    tile: RawTileId::from_idx(i),
                                    wire_to: self.rd.wires[wti].to_string(),
                                    wire_from: self.rd.wires[wfi].to_string(),
                                },
                            );
                        }
                        let mode = self.xtile.builder.pip_mode(wt.wire);
                        pips.pips.insert((wt, wf.pos()), mode);
                    } else if i == 0
                        && let Some(wf) = self.get_alt_by_name(i, wfi)
                    {
                        self.tcls_naming.wires.insert(
                            wt,
                            WireNaming {
                                name: self.rd.wires[wti].to_string(),
                                alt_name: None,
                                alt_pips_to: Default::default(),
                                alt_pips_from: Default::default(),
                            },
                        );
                        self.alt_names.insert(wf, self.rd.wires[wfi].to_string());
                        if wt == wf {
                            continue;
                        }
                        if self.xtile.force_skip_pips.contains(&(wt, wf)) {
                            continue;
                        }
                        self.alt_pips_from.entry(wf).or_default().insert(wt);
                        let mode = self.xtile.builder.pip_mode(wt.wire);
                        pips.pips.insert((wt, wf.pos()), mode);
                    } else if self.xtile.builder.stub_outs.contains(&self.rd.wires[wfi]) {
                        // ignore
                    } else {
                        println!(
                            "UNEXPECTED XTILE MUX IN {} {} {} {}",
                            self.rd.tile_kinds.key(tile.kind),
                            tile.name,
                            self.rd.wires[wti],
                            self.rd.wires[wfi]
                        );
                    }
                } else if i == 0
                    && let Some(wt) = self.get_alt_by_name(i, wti)
                {
                    let mut pass = rt.extract_muxes
                        && !matches!(self.db[wt.wire], WireKind::BelOut)
                        && !self.xtile.skip_muxes.contains(&wt.wire);
                    if self.xtile.optin_muxes.contains(&wt.wire) {
                        pass = true;
                    }
                    if self.xtile.optin_muxes_tile.contains(&wt) {
                        pass = true;
                    }
                    if !pass {
                        continue;
                    }
                    if let Some(wf) = self.get_wire_by_name(i, wfi) {
                        self.tcls_naming.wires.insert(
                            wf,
                            WireNaming {
                                name: self.rd.wires[wfi].to_string(),
                                alt_name: None,
                                alt_pips_to: Default::default(),
                                alt_pips_from: Default::default(),
                            },
                        );
                        self.alt_names.insert(wt, self.rd.wires[wti].to_string());
                        if wt == wf {
                            continue;
                        }
                        if self.xtile.force_skip_pips.contains(&(wt, wf)) {
                            continue;
                        }
                        self.alt_pips_to.entry(wt).or_default().insert(wf);
                        let mode = self.xtile.builder.pip_mode(wt.wire);
                        pips.pips.insert((wt, wf.pos()), mode);
                    }
                }
            }
        }
        self.pips.push((self.xtile.switchbox.unwrap(), pips));
    }

    fn extract_delay(&mut self) {
        let crd = self.xtile.raw_tiles[0].xy;
        let tile = &self.rd.tiles[&crd];
        let tk = &self.rd.tile_kinds[tile.kind];
        if let Some(sb) = self.xtile.intf_sb
            && self.xtile.extract_delay
        {
            let mut pips = Pips::default();
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
                let nwt = self.rd.lookup_wire_raw_force(crd, wti);
                if let Some(&(_, wf)) = self.names.get(&nwf) {
                    if !matches!(self.db[wf.wire], WireKind::MuxOut) {
                        continue;
                    }
                    let Some(&wtw) = self.xtile.builder.delay_wires.get(&wf.wire) else {
                        continue;
                    };
                    let wt = TileWireCoord {
                        cell: wf.cell,
                        wire: wtw,
                    };
                    assert_eq!(bwdi, wdi);
                    self.tcls_naming.wires.insert(
                        wf,
                        WireNaming {
                            name: self.rd.wires[wfi].clone(),
                            alt_name: None,
                            alt_pips_to: Default::default(),
                            alt_pips_from: Default::default(),
                        },
                    );
                    self.tcls_naming.wires.insert(
                        wt,
                        WireNaming {
                            name: self.rd.wires[wti].clone(),
                            alt_name: None,
                            alt_pips_to: Default::default(),
                            alt_pips_from: Default::default(),
                        },
                    );
                    self.tcls_naming
                        .delay_wires
                        .insert(wt, self.rd.wires[wdi].clone());
                    self.names.insert(nwt, (IntConnKind::Raw, wt));
                    pips.pips.insert((wt, wf.pos()), PipMode::Delay);
                }
            }
            self.pips.push((sb, pips));
        }
    }

    fn extract_tmux(&mut self) {
        let crd = self.xtile.raw_tiles[0].xy;
        let tile = &self.rd.tiles[&crd];
        let tk = &self.rd.tile_kinds[tile.kind];
        let mut out_muxes: BTreeMap<TileWireCoord, (Vec<TileWireCoord>, Option<TileWireCoord>)> =
            BTreeMap::new();
        for &(wfi, wti) in tk.pips.keys() {
            let nwt = self.rd.lookup_wire_raw_force(crd, wti);
            if let Some(&(_, wt)) = self.names.get(&nwt) {
                if !matches!(self.db[wt.wire], WireKind::BelOut) {
                    continue;
                }
                self.tcls_naming
                    .wires
                    .entry(wt)
                    .or_insert_with(|| WireNaming {
                        name: self.rd.wires[wti].clone(),
                        alt_name: None,
                        alt_pips_to: Default::default(),
                        alt_pips_from: Default::default(),
                    });
                if !self.xtile.has_intf_out_bufs {
                    let wf = TileWireCoord {
                        cell: wt.cell,
                        wire: self.xtile.builder.test_mux_ins[&wt.wire],
                    };
                    self.tcls_naming.wires.insert(
                        wf,
                        WireNaming {
                            name: self.rd.wires[wti].clone(),
                            alt_name: None,
                            alt_pips_to: Default::default(),
                            alt_pips_from: Default::default(),
                        },
                    );
                }
                let nwf = self.rd.lookup_wire_raw_force(crd, wfi);
                if let Some(&(_, mut wf)) = self.names.get(&nwf) {
                    if let Some(&w) = self.xtile.builder.optinvs.get(&wf.wire) {
                        wf.wire = w;
                    }
                    self.tcls_naming.intf_wires_in.insert(
                        wf,
                        IntfWireInNaming::Simple {
                            name: self.rd.wires[wfi].clone(),
                        },
                    );
                    if self.db[wf.wire] == WireKind::BelOut
                        || self.xtile.builder.test_mux_pass.contains(&wf.wire)
                    {
                        assert!(out_muxes.entry(wt).or_default().1.replace(wf).is_none());
                    } else {
                        out_muxes.entry(wt).or_default().0.push(wf);
                    }
                } else if let Some(&(_, mut wf, rt, bwti, bwfi)) = self.int_in.get(&nwf) {
                    if let Some(&w) = self.xtile.builder.optinvs.get(&wf.wire) {
                        wf.wire = w;
                    }
                    if !self.buf_in.contains_key(&nwf) {
                        assert!(!self.xtile.has_intf_out_bufs);
                        continue;
                    }
                    assert_eq!(rt, 0);
                    assert_eq!(bwti, wfi);
                    self.tcls_naming.intf_wires_in.insert(
                        wf,
                        IntfWireInNaming::TestBuf {
                            name_out: self.rd.wires[wfi].clone(),
                            name_in: self.rd.wires[bwfi].clone(),
                        },
                    );
                    out_muxes.entry(wt).or_default().0.push(wf);
                } else if self.xtile.has_intf_out_bufs {
                    out_muxes.entry(wt).or_default();
                    let wf = TileWireCoord {
                        cell: wt.cell,
                        wire: self.xtile.builder.test_mux_ins[&wt.wire],
                    };
                    self.tcls_naming.wires.insert(
                        wf,
                        WireNaming {
                            name: self.rd.wires[wfi].clone(),
                            alt_name: None,
                            alt_pips_to: Default::default(),
                            alt_pips_from: Default::default(),
                        },
                    );
                }
            }
        }
        let mut pips = Pips::default();
        for (&wt, (wfs, _)) in out_muxes.iter_mut() {
            if let Some(&nwf) = self.xtile.builder.test_mux_ins_test.get(&wt.wire) {
                let nwf = TileWireCoord {
                    cell: wt.cell,
                    wire: nwf,
                };
                self.tcls_naming
                    .intf_wires_in
                    .insert(nwf, IntfWireInNaming::Anonymous);
                for &wf in wfs.iter() {
                    pips.pips.insert((nwf, wf.pos()), PipMode::Mux);
                }
                *wfs = vec![nwf];
            }
            wfs.sort();
        }
        if !pips.pips.is_empty() {
            self.pips.push((self.xtile.intf_sb.unwrap(), pips));
        }
        let num_groups = out_muxes.values().map(|(wfs, _)| wfs.len()).max().unwrap();
        let mut tm = TestMux {
            bits: vec![],
            groups: vec![Default::default(); num_groups],
            bits_primary: Default::default(),
            wires: Default::default(),
        };
        for (wt, (wfs, pwf)) in out_muxes {
            let mut wfs = Vec::from_iter(wfs.into_iter().map(|x| Some(x.pos())));
            while wfs.len() < num_groups {
                wfs.push(None);
            }
            let pwf = pwf.unwrap_or_else(|| TileWireCoord {
                cell: wt.cell,
                wire: self.xtile.builder.test_mux_ins[&wt.wire],
            });
            tm.wires.insert(
                wt,
                TestMuxWire {
                    primary_src: pwf.pos(),
                    test_src: wfs,
                },
            );
        }
        self.bels
            .insert(self.xtile.tmux_bel.unwrap(), BelInfo::TestMux(tm));
    }
}

#[derive(Clone, Debug)]
struct IntType {
    tki: rawdump::TileKindId,
    naming: TileClassNamingId,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PipMode {
    Mux,
    Pass,
    Buf,
    PermaBuf,
    Delay,
}

#[derive(Clone, Debug, Default)]
pub struct Pips {
    pub pips: BTreeMap<(TileWireCoord, PolTileWireCoord), PipMode>,
    pub specials: BTreeSet<SwitchBoxItem>,
}

pub struct IntBuilder<'a> {
    pub rd: &'a Part,
    pub db: IntDb,
    pub ndb: NamingDb,
    pub term_slots: DirMap<ConnectorSlotId>,
    pub pips: BTreeMap<(TileClassId, BelSlotId), Pips>,
    permabuf_wires: BTreeSet<WireSlotId>,
    delay_wires: BTreeMap<WireSlotId, WireSlotId>,
    optinvs: BTreeMap<WireSlotId, WireSlotId>,
    is_mirror_square: bool,
    allow_mux_to_branch: bool,
    main_passes: DirMap<EntityPartVec<WireSlotId, WireSlotId>>,
    int_types: Vec<IntType>,
    injected_int_types: Vec<rawdump::TileKindId>,
    stub_outs: HashSet<String>,
    alt_names: HashMap<String, TileWireCoord>,
    extra_names: HashMap<String, TileWireCoord>,
    extra_names_tile: HashMap<rawdump::TileKindId, HashMap<String, TileWireCoord>>,
    test_mux_pass: HashSet<WireSlotId>,
    test_mux_ins: HashMap<WireSlotId, WireSlotId>,
    test_mux_ins_test: HashMap<WireSlotId, WireSlotId>,
    mux_to_ok: HashSet<WireSlotId>,
}

impl<'a> IntBuilder<'a> {
    pub fn new(rd: &'a Part, db: IntDb) -> Self {
        let term_slots = DirMap::from_fn(|dir| match dir {
            Dir::W => db.get_conn_slot("W"),
            Dir::E => db.get_conn_slot("E"),
            Dir::S => db.get_conn_slot("S"),
            Dir::N => db.get_conn_slot("N"),
        });

        let ndb = NamingDb::default();
        Self {
            rd,
            db,
            ndb,
            term_slots,
            permabuf_wires: Default::default(),
            delay_wires: Default::default(),
            optinvs: Default::default(),
            pips: Default::default(),
            is_mirror_square: false,
            allow_mux_to_branch: false,
            main_passes: Default::default(),
            int_types: vec![],
            injected_int_types: vec![],
            stub_outs: Default::default(),
            alt_names: Default::default(),
            extra_names: Default::default(),
            extra_names_tile: Default::default(),
            test_mux_pass: Default::default(),
            test_mux_ins: Default::default(),
            test_mux_ins_test: Default::default(),
            mux_to_ok: Default::default(),
        }
    }

    pub fn allow_mux_to_branch(&mut self) {
        self.allow_mux_to_branch = true;
    }

    pub fn allow_mux_to(&mut self, wire: WireSlotId) {
        self.mux_to_ok.insert(wire);
    }

    pub fn test_mux_pass(&mut self, wire: WireSlotId) {
        self.test_mux_pass.insert(wire);
    }

    pub fn set_mirror_square(&mut self) {
        self.is_mirror_square = true;
    }

    pub fn bel_virtual(&self, bel: BelSlotId) -> ExtrBelInfo {
        ExtrBelInfo {
            bel,
            manual: false,
            subs: vec![ExtrBelInfoSub {
                slot: None,
                pins: Default::default(),
                pin_renames: Default::default(),
                raw_tile: 0,
            }],
        }
    }

    pub fn bel_single(&self, bel: BelSlotId, slot: &str) -> ExtrBelInfo {
        ExtrBelInfo {
            bel,
            manual: false,
            subs: vec![ExtrBelInfoSub {
                slot: Some(rawdump::TkSiteSlot::Single(
                    self.rd.slot_kinds.get(slot).unwrap(),
                )),
                pins: Default::default(),
                pin_renames: Default::default(),
                raw_tile: 0,
            }],
        }
    }

    pub fn bel_indexed(&self, bel: BelSlotId, slot: &str, idx: usize) -> ExtrBelInfo {
        ExtrBelInfo {
            bel,
            manual: false,
            subs: vec![ExtrBelInfoSub {
                slot: Some(rawdump::TkSiteSlot::Indexed(
                    self.rd.slot_kinds.get(slot).unwrap(),
                    idx as u8,
                )),
                pins: Default::default(),
                pin_renames: Default::default(),
                raw_tile: 0,
            }],
        }
    }

    pub fn bel_xy(&self, bel: BelSlotId, slot: &str, x: usize, y: usize) -> ExtrBelInfo {
        ExtrBelInfo {
            bel,
            manual: false,
            subs: vec![ExtrBelInfoSub {
                slot: Some(rawdump::TkSiteSlot::Xy(
                    self.rd.slot_kinds.get(slot).expect("missing slot kind"),
                    x as u8,
                    y as u8,
                )),
                pins: Default::default(),
                pin_renames: Default::default(),
                raw_tile: 0,
            }],
        }
    }

    pub fn make_term_naming(&mut self, name: impl AsRef<str>) -> ConnectorClassNamingId {
        match self.ndb.conn_class_namings.get(name.as_ref()) {
            None => {
                self.ndb
                    .conn_class_namings
                    .insert(name.as_ref().to_string(), Default::default())
                    .0
            }
            Some((i, _)) => i,
        }
    }

    pub fn name_term_in_near_wire(
        &mut self,
        naming: ConnectorClassNamingId,
        wire: WireSlotId,
        name: impl AsRef<str>,
    ) {
        let name = name.as_ref();
        let naming = &mut self.ndb.conn_class_namings[naming];
        if !naming.wires_in_near.contains_id(wire) {
            naming.wires_in_near.insert(wire, name.to_string());
        } else {
            assert_eq!(naming.wires_in_near[wire], name);
        }
    }

    pub fn name_term_in_far_wire(
        &mut self,
        naming: ConnectorClassNamingId,
        wire: WireSlotId,
        name: impl AsRef<str>,
    ) {
        let name = name.as_ref();
        let naming = &mut self.ndb.conn_class_namings[naming];
        if !naming.wires_in_far.contains_id(wire) {
            naming.wires_in_far.insert(
                wire,
                ConnectorWireInFarNaming::Simple {
                    name: name.to_string(),
                },
            );
        } else {
            assert_matches!(&naming.wires_in_far[wire], ConnectorWireInFarNaming::Simple{name: n} if n == name);
        }
    }

    pub fn name_term_in_far_buf_wire(
        &mut self,
        naming: ConnectorClassNamingId,
        wire: WireSlotId,
        name_out: impl AsRef<str>,
        name_in: impl AsRef<str>,
    ) {
        let name_out = name_out.as_ref();
        let name_in = name_in.as_ref();
        let naming = &mut self.ndb.conn_class_namings[naming];
        if !naming.wires_in_far.contains_id(wire) {
            naming.wires_in_far.insert(
                wire,
                ConnectorWireInFarNaming::Buf {
                    name_out: name_out.to_string(),
                    name_in: name_in.to_string(),
                },
            );
        } else {
            assert_matches!(&naming.wires_in_far[wire], ConnectorWireInFarNaming::Buf{name_out: no, name_in: ni} if no == name_out && ni == name_in);
        }
    }

    pub fn name_term_in_far_buf_far_wire(
        &mut self,
        naming: ConnectorClassNamingId,
        wire: WireSlotId,
        name: impl AsRef<str>,
        name_out: impl AsRef<str>,
        name_in: impl AsRef<str>,
    ) {
        let name = name.as_ref();
        let name_out = name_out.as_ref();
        let name_in = name_in.as_ref();
        let naming = &mut self.ndb.conn_class_namings[naming];
        if !naming.wires_in_far.contains_id(wire) {
            naming.wires_in_far.insert(
                wire,
                ConnectorWireInFarNaming::BufFar {
                    name: name.to_string(),
                    name_far_out: name_out.to_string(),
                    name_far_in: name_in.to_string(),
                },
            );
        } else {
            assert_matches!(&naming.wires_in_far[wire], ConnectorWireInFarNaming::BufFar{name: n, name_far_out: no, name_far_in: ni} if n == name && no == name_out && ni == name_in);
        }
    }

    pub fn name_term_out_wire(
        &mut self,
        naming: ConnectorClassNamingId,
        wire: WireSlotId,
        name: impl AsRef<str>,
    ) {
        let name = name.as_ref();
        let naming = &mut self.ndb.conn_class_namings[naming];
        if !naming.wires_out.contains_id(wire) {
            naming.wires_out.insert(
                wire,
                ConnectorWireOutNaming::Simple {
                    name: name.to_string(),
                },
            );
        } else {
            assert_matches!(&naming.wires_out[wire], ConnectorWireOutNaming::Simple{name: n} if n == name);
        }
    }

    pub fn name_term_out_buf_wire(
        &mut self,
        naming: ConnectorClassNamingId,
        wire: WireSlotId,
        name_out: impl AsRef<str>,
        name_in: impl AsRef<str>,
    ) {
        let name_out = name_out.as_ref();
        let name_in = name_in.as_ref();
        let naming = &mut self.ndb.conn_class_namings[naming];
        if !naming.wires_out.contains_id(wire) {
            naming.wires_out.insert(
                wire,
                ConnectorWireOutNaming::Buf {
                    name_out: name_out.to_string(),
                    name_in: name_in.to_string(),
                },
            );
        } else {
            assert_matches!(&naming.wires_out[wire], ConnectorWireOutNaming::Buf{name_out: no, name_in: ni} if no == name_out && ni == name_in);
        }
    }

    pub fn mark_permabuf(&mut self, wire: WireSlotId) {
        self.permabuf_wires.insert(wire);
    }

    pub fn mark_delay(&mut self, wire: WireSlotId, delay: WireSlotId) {
        self.delay_wires.insert(wire, delay);
    }

    pub fn mark_test_mux_in(&mut self, tmin: WireSlotId, wire: WireSlotId) {
        self.test_mux_ins.insert(wire, tmin);
    }

    pub fn mark_test_mux_in_test(&mut self, tmin: WireSlotId, wire: WireSlotId) {
        self.test_mux_ins_test.insert(wire, tmin);
    }

    pub fn mark_optinv(&mut self, wire: WireSlotId, optinv: WireSlotId) {
        self.optinvs.insert(wire, optinv);
    }

    pub fn conn_branch(&mut self, src: WireSlotId, dir: Dir, dst: WireSlotId) {
        self.main_passes[!dir].insert(dst, src);
    }

    fn pip_mode(&self, dst: WireSlotId) -> PipMode {
        if self.permabuf_wires.contains(&dst) {
            PipMode::PermaBuf
        } else {
            PipMode::Mux
        }
    }

    pub fn stub_out(&mut self, name: impl Into<String>) {
        self.stub_outs.insert(name.into());
    }

    pub fn wire_names(&mut self, wire: WireSlotId, names: &[impl AsRef<str>]) {
        for name in names {
            let name = name.as_ref();
            if !name.is_empty() {
                self.extra_name(name, wire);
            }
        }
    }

    pub fn extra_name(&mut self, name: impl Into<String>, wire: WireSlotId) {
        self.extra_names
            .insert(name.into(), TileWireCoord::new_idx(0, wire));
    }

    pub fn extra_name_sub(&mut self, name: impl Into<String>, sub: usize, wire: WireSlotId) {
        self.extra_names
            .insert(name.into(), TileWireCoord::new_idx(sub, wire));
    }

    pub fn alt_name(&mut self, name: impl Into<String>, wire: WireSlotId) {
        self.alt_names
            .insert(name.into(), TileWireCoord::new_idx(0, wire));
    }

    pub fn alt_name_sub(&mut self, name: impl Into<String>, sub: usize, wire: WireSlotId) {
        self.alt_names
            .insert(name.into(), TileWireCoord::new_idx(sub, wire));
    }

    pub fn extra_name_tile(
        &mut self,
        tile: impl AsRef<str>,
        name: impl Into<String>,
        wire: WireSlotId,
    ) {
        if let Some((tki, _)) = self.rd.tile_kinds.get(tile.as_ref()) {
            self.extra_names_tile
                .entry(tki)
                .or_default()
                .insert(name.into(), TileWireCoord::new_idx(0, wire));
        }
    }

    pub fn extra_name_tile_sub(
        &mut self,
        tile: impl AsRef<str>,
        name: impl Into<String>,
        sub: usize,
        wire: WireSlotId,
    ) {
        if let Some((tki, _)) = self.rd.tile_kinds.get(tile.as_ref()) {
            self.extra_names_tile
                .entry(tki)
                .or_default()
                .insert(name.into(), TileWireCoord::new_idx(sub, wire));
        }
    }

    pub fn get_wire_by_name(&self, tki: TileKindId, name: &str) -> Option<TileWireCoord> {
        self.extra_names
            .get(name)
            .or_else(|| self.extra_names_tile.get(&tki).and_then(|m| m.get(name)))
            .copied()
    }

    pub fn inject_main_passes(&mut self, passes: DirMap<ConnectorClassId>) {
        for (dir, ccid) in passes {
            for (wt, &wf) in &self.db.conn_classes[ccid].wires {
                if let ConnectorWire::Pass(wf) = wf {
                    self.main_passes[dir].insert(wt, wf);
                }
            }
        }
    }

    fn extract_bels(
        &mut self,
        tcid: TileClassId,
        naming: &mut TileClassNaming,
        bels: &[ExtrBelInfo],
        tki: rawdump::TileKindId,
        names: &HashMap<rawdump::WireId, (IntConnKind, TileWireCoord)>,
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
                            pips.push(PipNaming {
                                tile: RawTileId::from_idx(0),
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
                                pips.push(PipNaming {
                                    tile: RawTileId::from_idx(0),
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
                                        PipNaming {
                                            tile: RawTileId::from_idx(0),
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
                            pips.push(PipNaming {
                                tile: RawTileId::from_idx(0),
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
                                pips.push(PipNaming {
                                    tile: RawTileId::from_idx(0),
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
        for info in bels {
            let mut bel = LegacyBel::default();
            let mut bnaming = BelNaming {
                tiles: vec![],
                pins: Default::default(),
            };
            for sub in &info.subs {
                let rt = RawTileId::from_idx(0);
                bnaming.tiles.push(rt);
                if let Some(slot) = sub.slot {
                    let tks = tk.sites.get(&slot).unwrap().1;
                    for (name, tksp) in &tks.pins {
                        match sub.pins.get(name).unwrap_or(&BelPinInfo::Int) {
                            &BelPinInfo::Int => {
                                let dir = match tksp.dir {
                                    rawdump::TkSitePinDir::Input => PinDir::Input,
                                    rawdump::TkSitePinDir::Output => PinDir::Output,
                                    _ => panic!("bidir pin {name}"),
                                };
                                let (ick, wires, wnf, pips, int_pips) =
                                    walk_to_int(dir, tksp.wire.unwrap());
                                bnaming.pins.insert(
                                    name.clone(),
                                    BelPinNaming {
                                        tile: rt,
                                        name: self.rd.wires[tksp.wire.unwrap()].clone(),
                                        name_far: self.rd.wires[wnf].clone(),
                                        pips,
                                        int_pips,
                                        is_intf: ick != IntConnKind::Raw,
                                    },
                                );
                                bel.pins.insert(name.clone(), BelPin { wires, dir });
                            }
                            &BelPinInfo::IntNudge(wire) => {
                                let dir = match tksp.dir {
                                    rawdump::TkSitePinDir::Input => PinDir::Input,
                                    rawdump::TkSitePinDir::Output => PinDir::Output,
                                    _ => panic!("bidir pin {name}"),
                                };
                                let (ick, _wires, wnf, pips, int_pips) =
                                    walk_to_int(dir, tksp.wire.unwrap());
                                bnaming.pins.insert(
                                    name.clone(),
                                    BelPinNaming {
                                        tile: rt,
                                        name: self.rd.wires[tksp.wire.unwrap()].clone(),
                                        name_far: self.rd.wires[wnf].clone(),
                                        pips,
                                        int_pips,
                                        is_intf: ick != IntConnKind::Raw,
                                    },
                                );
                                bel.pins.insert(
                                    name.clone(),
                                    BelPin {
                                        wires: BTreeSet::from_iter([wire]),
                                        dir,
                                    },
                                );
                            }
                            &BelPinInfo::ForceInt(wire, ref wname) => {
                                let dir = match tksp.dir {
                                    rawdump::TkSitePinDir::Input => PinDir::Input,
                                    rawdump::TkSitePinDir::Output => PinDir::Output,
                                    _ => panic!("bidir pin {name}"),
                                };
                                bnaming.pins.insert(
                                    name.clone(),
                                    BelPinNaming {
                                        tile: rt,
                                        name: self.rd.wires[tksp.wire.unwrap()].clone(),
                                        name_far: wname.clone(),
                                        pips: Vec::new(),
                                        int_pips: BTreeMap::new(),
                                        is_intf: false,
                                    },
                                );
                                bel.pins.insert(
                                    name.clone(),
                                    BelPin {
                                        wires: [wire].into_iter().collect(),
                                        dir,
                                    },
                                );
                            }
                            &BelPinInfo::NameOnly(buf_cnt) => {
                                if buf_cnt == 0 {
                                    bnaming.pins.insert(
                                        name.clone(),
                                        BelPinNaming {
                                            tile: rt,
                                            name: self.rd.wires[tksp.wire.unwrap()].clone(),
                                            name_far: self.rd.wires[tksp.wire.unwrap()].clone(),
                                            pips: Vec::new(),
                                            int_pips: BTreeMap::new(),
                                            is_intf: false,
                                        },
                                    );
                                } else {
                                    let dir = match tksp.dir {
                                        rawdump::TkSitePinDir::Input => PinDir::Input,
                                        rawdump::TkSitePinDir::Output => PinDir::Output,
                                        _ => panic!("bidir pin {name}"),
                                    };
                                    let (wn, pips) = walk_count(dir, tksp.wire.unwrap(), buf_cnt);
                                    bnaming.pins.insert(
                                        name.clone(),
                                        BelPinNaming {
                                            tile: rt,
                                            name: self.rd.wires[tksp.wire.unwrap()].clone(),
                                            name_far: self.rd.wires[wn].clone(),
                                            pips,
                                            int_pips: BTreeMap::new(),
                                            is_intf: false,
                                        },
                                    );
                                }
                            }
                            BelPinInfo::ExtraWireForce(_, _) => (),
                            _ => unreachable!(),
                        }
                    }
                }
                for (name, pd) in &sub.pins {
                    match *pd {
                        BelPinInfo::ExtraInt(dir, ref names) => {
                            let mut wn = None;
                            for w in names {
                                if let Some(w) = self.rd.wires.get(w)
                                    && tk.wires.contains_key(&w)
                                {
                                    assert!(wn.is_none());
                                    wn = Some(w);
                                }
                            }
                            if wn.is_none() {
                                println!("NOT FOUND: {name}");
                            }
                            let wn = wn.unwrap();
                            let (ick, wires, wnf, pips, int_pips) = walk_to_int(dir, wn);
                            bnaming.pins.insert(
                                name.clone(),
                                BelPinNaming {
                                    tile: rt,
                                    name: self.rd.wires[wn].clone(),
                                    name_far: self.rd.wires[wnf].clone(),
                                    pips,
                                    int_pips,
                                    is_intf: ick != IntConnKind::Raw,
                                },
                            );
                            bel.pins.insert(name.clone(), BelPin { wires, dir });
                        }
                        BelPinInfo::ExtraIntForce(dir, wire, ref wname) => {
                            bnaming.pins.insert(
                                name.clone(),
                                BelPinNaming {
                                    tile: rt,
                                    name: wname.clone(),
                                    name_far: wname.clone(),
                                    pips: vec![],
                                    int_pips: BTreeMap::new(),
                                    is_intf: false,
                                },
                            );
                            bel.pins.insert(
                                name.clone(),
                                BelPin {
                                    wires: [wire].into_iter().collect(),
                                    dir,
                                },
                            );
                        }
                        BelPinInfo::ExtraWire(ref names) => {
                            let mut wn = None;
                            for w in names {
                                if let Some(w) = self.rd.wires.get(w)
                                    && tk.wires.contains_key(&w)
                                {
                                    assert!(wn.is_none());
                                    wn = Some(w);
                                }
                            }
                            if wn.is_none() {
                                println!("NOT FOUND: {name}");
                            }
                            let wn = wn.unwrap();
                            bnaming.pins.insert(
                                name.clone(),
                                BelPinNaming {
                                    tile: rt,
                                    name: self.rd.wires[wn].clone(),
                                    name_far: self.rd.wires[wn].clone(),
                                    pips: Vec::new(),
                                    int_pips: BTreeMap::new(),
                                    is_intf: false,
                                },
                            );
                        }
                        BelPinInfo::ExtraWireForce(ref wname, ref pips) => {
                            bnaming.pins.insert(
                                name.clone(),
                                BelPinNaming {
                                    tile: rt,
                                    name: wname.clone(),
                                    name_far: wname.clone(),
                                    pips: pips.clone(),
                                    int_pips: BTreeMap::new(),
                                    is_intf: false,
                                },
                            );
                        }
                        _ => (),
                    }
                }
            }
            self.insert_tcls_bel(tcid, info.bel, BelInfo::Legacy(bel));
            naming.bels.insert(info.bel, bnaming);
        }
    }

    pub fn extract_int_id(
        &mut self,
        tcid: TileClassId,
        sb: BelSlotId,
        tile_kind: &str,
        naming: &str,
        bels: &[ExtrBelInfo],
    ) {
        if let Some((tki, _)) = self.rd.tile_kinds.get(tile_kind) {
            let tk = &self.rd.tile_kinds[tki];
            let tkn = self.rd.tile_kinds.key(tki);
            let mut pips = Pips::default();
            let mut tcls_naming = TileClassNaming::default();
            let mut names = HashMap::new();
            for &wi in tk.wires.keys() {
                if let Some(w) = self.get_wire_by_name(tki, &self.rd.wires[wi]) {
                    names.insert(wi, (IntConnKind::Raw, w));
                }
            }

            for (&k, &(_, v)) in &names {
                tcls_naming.wires.insert(
                    v,
                    WireNaming {
                        name: self.rd.wires[k].clone(),
                        alt_name: None,
                        alt_pips_to: Default::default(),
                        alt_pips_from: Default::default(),
                    },
                );
            }

            for &(wfi, wti) in tk.pips.keys() {
                if let Some(&(_, wt)) = names.get(&wti) {
                    match self.db[wt.wire] {
                        WireKind::MultiBranch(_) | WireKind::MultiRoot | WireKind::MuxOut => (),
                        WireKind::Branch(_) => {
                            if !self.allow_mux_to_branch {
                                continue;
                            }
                        }
                        _ => {
                            if !self.mux_to_ok.contains(&wt.wire) {
                                continue;
                            }
                        }
                    }
                    if let Some(&(_, wf)) = names.get(&wfi) {
                        let mode = self.pip_mode(wt.wire);
                        pips.pips.insert((wt, wf.pos()), mode);
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
            self.insert_tcls_pips(tcid, sb, pips);
            self.extract_bels(tcid, &mut tcls_naming, bels, tki, &names);

            let naming = self.insert_tcls_naming(naming, tcls_naming);
            self.int_types.push(IntType { tki, naming });
        }
    }

    pub fn extract_int_bels_id(
        &mut self,
        tcid: TileClassId,
        tile_kind: &str,
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

            let mut tcls_naming = TileClassNaming::default();
            self.extract_bels(tcid, &mut tcls_naming, bels, tki, &names);
            self.insert_tcls_naming(naming, tcls_naming);
        }
    }

    pub fn int_type_id(&mut self, tcid: TileClassId, sb: BelSlotId, tile_kind: &str, naming: &str) {
        self.extract_int_id(tcid, sb, tile_kind, naming, &[]);
    }

    pub fn inject_int_type(&mut self, tile_kind: &str) {
        if let Some((tki, _)) = self.rd.tile_kinds.get(tile_kind) {
            self.injected_int_types.push(tki);
        }
    }

    pub fn inject_int_type_naming(&mut self, tile_kind: &str, naming: TileClassNamingId) {
        if let Some((tki, _)) = self.rd.tile_kinds.get(tile_kind) {
            self.int_types.push(IntType { tki, naming });
        }
    }

    fn get_int_naming(&self, int_xy: Coord) -> Option<TileClassNamingId> {
        let int_tile = &self.rd.tiles[&int_xy];
        self.int_types.iter().find_map(|nt| {
            if nt.tki == int_tile.kind {
                Some(nt.naming)
            } else {
                None
            }
        })
    }

    fn get_int_rev_naming(&self, int_xy: Coord) -> HashMap<String, WireSlotId> {
        if let Some(int_naming_id) = self.get_int_naming(int_xy) {
            let int_naming = &self.ndb.tile_class_namings[int_naming_id];
            int_naming
                .wires
                .iter()
                .filter_map(|(k, v)| {
                    if k.cell.to_idx() == 0 {
                        Some((v.name.to_string(), k.wire))
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
        if let Some((_, &rawdump::TkWire::Connected(idx))) = tk.wires.get(&wi)
            && let Some(&nidx) = tile.conn_wires.get(idx)
        {
            return Some(nidx);
        }
        None
    }

    fn get_int_node2wires(&self, int_xy: Coord) -> HashMap<rawdump::NodeId, Vec<WireSlotId>> {
        let int_tile = &self.rd.tiles[&int_xy];
        let int_tk = &self.rd.tile_kinds[int_tile.kind];
        let int_rev_naming = self.get_int_rev_naming(int_xy);
        let mut res: HashMap<_, Vec<_>> = HashMap::new();
        for (_, &wi, &tkw) in &int_tk.wires {
            if let Some(&w) = int_rev_naming.get(&self.rd.wires[wi])
                && let rawdump::TkWire::Connected(idx) = tkw
                && let Some(&nidx) = int_tile.conn_wires.get(idx)
            {
                res.entry(nidx).or_default().push(w);
            }
        }
        res
    }

    pub fn recover_names(
        &self,
        tile_xy: Coord,
        int_xy: Coord,
    ) -> HashMap<rawdump::WireId, Vec<WireSlotId>> {
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
                    res.insert(wi, vec![w.wire]);
                } else if let rawdump::TkWire::Connected(idx) = tkw
                    && let Some(&nidx) = tile.conn_wires.get(idx)
                    && let Some(w) = node2wires.get(&nidx)
                {
                    res.insert(wi, w.clone());
                }
            }
            res
        }
    }

    pub fn recover_names_cands(
        &self,
        tile_xy: Coord,
        int_xy: Coord,
        cands: &HashSet<WireSlotId>,
    ) -> HashMap<rawdump::WireId, WireSlotId> {
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

    fn insert_tcls_pips(&mut self, tcid: TileClassId, bslot: BelSlotId, pips: Pips) {
        match self.pips.entry((tcid, bslot)) {
            btree_map::Entry::Vacant(e) => {
                e.insert(pips);
            }
            btree_map::Entry::Occupied(mut e) => {
                let cur_pips = e.get_mut();
                for ((wt, wf), mode) in pips.pips {
                    match cur_pips.pips.entry((wt, wf)) {
                        btree_map::Entry::Vacant(ee) => {
                            ee.insert(mode);
                        }
                        btree_map::Entry::Occupied(ee) => {
                            assert_eq!(*ee.get(), mode);
                        }
                    }
                }
            }
        }
    }

    fn convert_bel(&self, slot: BelSlotId, bel: BelInfo) -> BelInfo {
        match self.db.bel_slots[slot].kind {
            BelKind::Class(bcid) => {
                let bcls = &self.db[bcid];
                let mut bel = match bel {
                    BelInfo::Bel(_) => return bel,
                    BelInfo::Legacy(bel) => bel,
                    _ => unreachable!(),
                };
                let is_ultra = matches!(
                    self.rd.family.as_str(),
                    "ultrascale" | "ultrascaleplus" | "versal"
                );
                fn convert_input(pin: BelPin) -> BelInput {
                    assert_eq!(pin.dir, PinDir::Input);
                    assert_eq!(pin.wires.len(), 1);
                    let wire = pin.wires.into_iter().next().unwrap();
                    BelInput::Fixed(wire.pos())
                }
                fn convert_output(pin: BelPin) -> BTreeSet<TileWireCoord> {
                    assert_eq!(pin.dir, PinDir::Output);
                    pin.wires
                }
                fn convert_bidir(pin: BelPin) -> TileWireCoord {
                    assert_eq!(pin.wires.len(), 1);
                    pin.wires.into_iter().next().unwrap()
                }
                let name_index = |name, idx| {
                    if is_ultra {
                        format!("{name}_{idx}_")
                    } else {
                        format!("{name}{idx}")
                    }
                };
                let mut res = Bel::default();
                for (index, name, inp) in bcls.inputs.bundles() {
                    match index {
                        EntityBundleIndex::Single(id) => {
                            if let Some(pin) = bel.pins.remove(name) {
                                res.inputs.insert(id, convert_input(pin));
                            }
                        }
                        EntityBundleIndex::Array(range) => {
                            for (i, id) in range.into_iter().enumerate() {
                                let idx = inp.indexing.phys_to_virt(i);
                                if let Some(pin) = bel.pins.remove(&name_index(name, idx)) {
                                    res.inputs.insert(id, convert_input(pin));
                                }
                            }
                        }
                    }
                }
                for (index, name, outp) in bcls.outputs.bundles() {
                    match index {
                        EntityBundleIndex::Single(id) => {
                            if let Some(pin) = bel.pins.remove(name) {
                                res.outputs.insert(id, convert_output(pin));
                            }
                        }
                        EntityBundleIndex::Array(range) => {
                            for (i, id) in range.into_iter().enumerate() {
                                let idx = outp.indexing.phys_to_virt(i);
                                if let Some(pin) = bel.pins.remove(&name_index(name, idx)) {
                                    res.outputs.insert(id, convert_output(pin));
                                }
                            }
                        }
                    }
                }
                for (index, name, bidir) in bcls.bidirs.bundles() {
                    match index {
                        EntityBundleIndex::Single(id) => {
                            if let Some(pin) = bel.pins.remove(name) {
                                res.bidirs.insert(id, convert_bidir(pin));
                            }
                        }
                        EntityBundleIndex::Array(range) => {
                            for (i, id) in range.into_iter().enumerate() {
                                let idx = bidir.indexing.phys_to_virt(i);
                                if let Some(pin) = bel.pins.remove(&name_index(name, idx)) {
                                    res.bidirs.insert(id, convert_bidir(pin));
                                }
                            }
                        }
                    }
                }
                for pin in bel.pins.keys() {
                    println!("OOPS UNCONVERTED PIN {pin}");
                }
                BelInfo::Bel(res)
            }
            BelKind::Routing => {
                let BelInfo::Legacy(bel) = bel else {
                    return bel;
                };
                let mut dst = None;
                let mut src = None;
                for pin in bel.pins.into_values() {
                    assert_eq!(pin.wires.len(), 1);
                    let wire = pin.wires.into_iter().next().unwrap();
                    match pin.dir {
                        PinDir::Input => {
                            assert_eq!(src, None);
                            src = Some(wire);
                        }
                        PinDir::Output => {
                            assert_eq!(dst, None);
                            dst = Some(wire);
                        }
                        PinDir::Inout => unreachable!(),
                    }
                }
                let dst = dst.unwrap();
                let src = src.unwrap().pos();
                BelInfo::SwitchBox(SwitchBox {
                    items: vec![SwitchBoxItem::PermaBuf(PermaBuf { dst, src })],
                })
            }
            BelKind::Legacy => bel,
        }
    }

    pub fn insert_tcls_bel(&mut self, tcid: TileClassId, slot: BelSlotId, mut bel: BelInfo) {
        if let BelInfo::Legacy(ref mut info) = bel {
            for pin in info.pins.values_mut() {
                let wires = std::mem::take(&mut pin.wires);
                pin.wires = wires
                    .into_iter()
                    .map(|mut wire| {
                        if let Some(&w) = self.optinvs.get(&wire.wire) {
                            wire.wire = w;
                        }
                        wire
                    })
                    .collect();
            }
        }
        let bel = self.convert_bel(slot, bel);
        let tcls = &mut self.db.tile_classes[tcid];
        if !tcls.bels.contains_id(slot) {
            tcls.bels.insert(slot, bel);
        } else {
            let cur_bel = &mut tcls.bels[slot];
            assert_eq!(bel, *cur_bel);
        }
    }

    pub fn insert_tcls_naming(&mut self, name: &str, naming: TileClassNaming) -> TileClassNamingId {
        match self.ndb.tile_class_namings.get_mut(name) {
            None => {
                self.ndb
                    .tile_class_namings
                    .insert(name.to_string(), naming)
                    .0
            }
            Some((id, cnaming)) => {
                assert_eq!(naming.wire_bufs, cnaming.wire_bufs);
                assert_eq!(naming.delay_wires, cnaming.delay_wires);
                for (k, v) in naming.ext_pips {
                    match cnaming.ext_pips.get(&k) {
                        None => {
                            cnaming.ext_pips.insert(k, v);
                        }
                        Some(cv) => {
                            assert_eq!(v, *cv);
                        }
                    }
                }
                for (k, v) in naming.bels {
                    match cnaming.bels.get(k) {
                        None => {
                            cnaming.bels.insert(k, v);
                        }
                        Some(cv) => {
                            assert_eq!(v, *cv);
                        }
                    }
                }
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
                id
            }
        }
    }

    pub fn insert_bel_naming(&mut self, naming: &str, bslot: BelSlotId, bn: BelNaming) {
        if !self.ndb.tile_class_namings.contains_key(naming) {
            self.ndb
                .tile_class_namings
                .insert(naming.to_string(), Default::default());
        }
        let tn = self.ndb.tile_class_namings.get_mut(naming).unwrap().1;
        match tn.bels.get(bslot) {
            None => {
                tn.bels.insert(bslot, bn);
            }
            Some(cv) => {
                assert_eq!(bn, *cv);
            }
        }
    }

    pub fn insert_connector(&mut self, ccid: ConnectorClassId, ccls: ConnectorClass) {
        let cur_ccls = &mut self.db.conn_classes[ccid];
        assert_eq!(ccls.slot, cur_ccls.slot);
        for k in self.db.wires.ids() {
            let a = cur_ccls.wires.get_mut(k);
            let b = ccls.wires.get(k);
            match (a, b) {
                (_, None) => (),
                (None, Some(b)) => {
                    cur_ccls.wires.insert(k, *b);
                }
                (a, b) => assert_eq!(a.map(|x| &*x), b),
            }
        }
    }

    fn get_pass_inps(&self, dir: Dir) -> HashSet<WireSlotId> {
        self.main_passes[dir].values().copied().collect()
    }

    fn extract_term_tile_conn(
        &self,
        dir: Dir,
        int_xy: Coord,
        forced: &HashMap<WireSlotId, WireSlotId>,
    ) -> EntityPartVec<WireSlotId, ConnectorWire> {
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
                        res.insert(wt, ConnectorWire::Reflect(fwf));
                    }
                } else {
                    if wf.len() == 1 {
                        res.insert(wt, ConnectorWire::Reflect(wf[0]));
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

    pub fn extract_term_tile_id(
        &mut self,
        ccid: ConnectorClassId,
        tcls: Option<(TileClassId, BelSlotId)>,
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
        let mut muxes: HashMap<WireSlotId, Vec<WireSlotId>> = HashMap::new();
        let naming_id = self.make_term_naming(naming.as_ref());
        let mut tnames = EntityPartVec::new();
        for &(wfi, wti) in tk.pips.keys() {
            if let Some(wtl) = names.get(&wti) {
                for &wt in wtl {
                    match self.db[wt] {
                        WireKind::Branch(slot) => {
                            if slot != self.term_slots[dir] {
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
        let mut node_pips = Pips::default();
        let mut node_names = BTreeMap::new();
        let mut wires = self.extract_term_tile_conn(dir, int_xy, &Default::default());
        for (k, v) in muxes {
            if v.len() == 1 {
                self.name_term_out_wire(naming_id, k, tnames[k]);
                self.name_term_in_near_wire(naming_id, v[0], tnames[v[0]]);
                wires.insert(k, ConnectorWire::Reflect(v[0]));
            } else {
                let def_t = CellSlotId::from_idx(0);
                node_names.insert(
                    TileWireCoord::new_idx(0, k),
                    WireNaming {
                        name: tnames[k].to_string(),
                        alt_name: None,
                        alt_pips_to: Default::default(),
                        alt_pips_from: Default::default(),
                    },
                );
                for &x in &v {
                    node_names.insert(
                        TileWireCoord::new_idx(0, x),
                        WireNaming {
                            name: tnames[x].to_string(),
                            alt_name: None,
                            alt_pips_to: Default::default(),
                            alt_pips_from: Default::default(),
                        },
                    );
                }
                let wt = TileWireCoord {
                    cell: def_t,
                    wire: k,
                };
                for x in v {
                    let wf = TileWireCoord {
                        cell: def_t,
                        wire: x,
                    };
                    let mode = self.pip_mode(wt.wire);
                    node_pips.pips.insert((wt, wf.pos()), mode);
                }
            }
        }
        if let Some((tcid, sb)) = tcls {
            self.insert_tcls_pips(tcid, sb, node_pips);
            self.insert_tcls_naming(
                naming.as_ref(),
                TileClassNaming {
                    wires: node_names,
                    wire_bufs: Default::default(),
                    ext_pips: Default::default(),
                    delay_wires: Default::default(),
                    bels: Default::default(),
                    intf_wires_in: Default::default(),
                },
            );
        } else {
            assert!(node_pips.pips.is_empty());
        }
        let term = ConnectorClass {
            slot: self.term_slots[dir],
            wires,
        };
        self.insert_connector(ccid, term);
    }

    pub fn extract_term_buf_tile_id(
        &mut self,
        ccid: ConnectorClassId,
        dir: Dir,
        term_xy: Coord,
        naming: impl AsRef<str>,
        int_xy: Coord,
        forced: &[(WireSlotId, WireSlotId)],
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
                    match self.db[wt] {
                        WireKind::Branch(slot) => {
                            if slot != self.term_slots[dir] {
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
                        wires.insert(wt, ConnectorWire::Reflect(wf));
                    } else {
                        println!(
                            "UNEXPECTED TERM BUF IN {} {} {}",
                            tkn, self.rd.wires[wti], self.rd.wires[wfi]
                        );
                    }
                }
            }
        }
        let term = ConnectorClass {
            slot: self.term_slots[dir],
            wires,
        };
        self.insert_connector(ccid, term);
    }

    pub fn extract_term_conn_tile_id(
        &mut self,
        ccls: ConnectorClassId,
        dir: Dir,
        int_xy: Coord,
        forced: &[(WireSlotId, WireSlotId)],
    ) {
        let forced: HashMap<_, _> = forced.iter().copied().collect();
        let wires = self.extract_term_tile_conn(dir, int_xy, &forced);
        let term = ConnectorClass {
            slot: self.term_slots[dir],
            wires,
        };
        self.insert_connector(ccls, term);
    }

    pub fn walk_to_int(&self, mut xy: Coord, mut dir: Dir, mut step: bool) -> Option<Coord> {
        if self.is_mirror_square {
            if matches!(dir, Dir::E | Dir::W) && xy.x >= self.rd.width / 2 {
                dir = !dir;
            }
            if matches!(dir, Dir::S | Dir::N) && xy.y >= self.rd.height / 2 {
                dir = !dir;
            }
        }
        loop {
            if !step {
                let tile = &self.rd.tiles[&xy];
                if self.int_types.iter().any(|x| x.tki == tile.kind)
                    || self.injected_int_types.contains(&tile.kind)
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

    pub fn delta(&self, xy: Coord, mut dx: i32, mut dy: i32) -> Coord {
        if self.is_mirror_square {
            if xy.x >= self.rd.width / 2 {
                dx = -dx;
            }
            if xy.y >= self.rd.height / 2 {
                dy = -dy;
            }
        }
        xy.delta(dx, dy)
    }

    pub fn extract_term_id(
        &mut self,
        ccid: ConnectorClassId,
        tcls: Option<(TileClassId, BelSlotId)>,
        dir: Dir,
        tkn: impl AsRef<str>,
        naming: impl AsRef<str>,
    ) {
        for &term_xy in self.rd.tiles_by_kind_name(tkn.as_ref()) {
            if let Some(int_xy) = self.walk_to_int(term_xy, !dir, false) {
                self.extract_term_tile_id(ccid, tcls, dir, term_xy, naming.as_ref(), int_xy);
            }
        }
    }

    pub fn extract_term_buf_id(
        &mut self,
        ccls: ConnectorClassId,
        dir: Dir,
        tkn: impl AsRef<str>,
        naming: impl AsRef<str>,
        forced: &[(WireSlotId, WireSlotId)],
    ) {
        for &term_xy in self.rd.tiles_by_kind_name(tkn.as_ref()) {
            if let Some(int_xy) = self.walk_to_int(term_xy, !dir, false) {
                self.extract_term_buf_tile_id(ccls, dir, term_xy, naming.as_ref(), int_xy, forced);
            }
        }
    }

    pub fn extract_term_conn_id(
        &mut self,
        ccls: ConnectorClassId,
        dir: Dir,
        tkn: impl AsRef<str>,
        forced: &[(WireSlotId, WireSlotId)],
    ) {
        for &term_xy in self.rd.tiles_by_kind_name(tkn.as_ref()) {
            if let Some(int_xy) = self.walk_to_int(term_xy, !dir, false) {
                self.extract_term_conn_tile_id(ccls, dir, int_xy, forced);
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

    pub fn extract_pass_tile_id(
        &mut self,
        ccid: ConnectorClassId,
        dir: Dir,
        int_xy: Coord,
        near: Option<Coord>,
        far: Option<Coord>,
        naming: Option<&str>,
        tcls: Option<(TileClassId, BelSlotId, &str)>,
        splitter_tcls: Option<(TileClassId, BelSlotId, &str)>,
        src_xy: Coord,
        force_pass: &[WireSlotId],
    ) {
        let cand_inps_far = self.get_pass_inps(dir);
        let int_tile = &self.rd.tiles[&int_xy];
        let int_tk = &self.rd.tile_kinds[int_tile.kind];
        let int_naming = &self.ndb.tile_class_namings[self.get_int_naming(int_xy).unwrap()];
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
                wires.insert(wn, ConnectorWire::Pass(wf));
            }
        }
        for wn in self.main_passes[dir].ids() {
            if let Some(wnn) = int_naming.wires.get(&TileWireCoord::new_idx(0, wn)) {
                let wni = self.rd.wires.get(&wnn.name).unwrap();
                if let Some(nidx) = self.get_node(int_tile, int_tk, wni)
                    && let Some(w) = src_node2wires.get(&nidx)
                {
                    let w: Vec<_> = w
                        .iter()
                        .copied()
                        .filter(|x| cand_inps_far.contains(x))
                        .collect();
                    if w.len() == 1 {
                        wires.insert(wn, ConnectorWire::Pass(w[0]));
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
                        if let Some(&wf) = far_names.get(&wfi)
                            && let Some(nidx) = self.get_node(far_tile, far_tk, wti)
                        {
                            nodes.insert(nidx, (wf, wti, wfi));
                        }
                    }
                    for &wi in tk.wires.keys() {
                        if let Some(nidx) = self.get_node(tile, tk, wi)
                            && let Some(&x) = nodes.get(&nidx)
                        {
                            names_far_buf.insert(wi, x);
                        }
                    }
                }
            }
            #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
            enum WireIn {
                Near(WireSlotId),
                Far(WireSlotId),
            }
            #[derive(Clone, Copy)]
            enum FarNaming<'b> {
                Simple(&'b str),
                BufNear(&'b str, &'b str),
                BufFar(&'b str, &'b str, &'b str),
            }
            let mut muxes: HashMap<WireSlotId, Vec<WireIn>> = HashMap::new();
            let mut names_out = EntityPartVec::new();
            let mut names_in_near = EntityPartVec::new();
            let mut names_in_far = EntityPartVec::new();
            for &(wfi, wti) in tk.pips.keys() {
                if let Some(wtl) = names.get(&wti) {
                    for &wt in wtl {
                        match self.db[wt] {
                            WireKind::Branch(slot) => {
                                if slot != self.term_slots[dir] {
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
            let mut node_pips = Pips::default();
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
                            wires.insert(wt, ConnectorWire::Reflect(wf));
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
                            wires.insert(wt, ConnectorWire::Pass(wf));
                        }
                    }
                } else {
                    node_names.insert(
                        TileWireCoord {
                            cell: nt_near,
                            wire: wt,
                        },
                        names_out[wt].to_string(),
                    );
                    let mut ins = BTreeSet::new();
                    for &x in &v {
                        match x {
                            WireIn::Near(wf) => {
                                node_names.insert(
                                    TileWireCoord {
                                        cell: nt_near,
                                        wire: wf,
                                    },
                                    names_in_near[wf].to_string(),
                                );
                                ins.insert(TileWireCoord {
                                    cell: nt_near,
                                    wire: wf,
                                });
                            }
                            WireIn::Far(wf) => {
                                match names_in_far[wf] {
                                    FarNaming::Simple(n) => {
                                        node_names.insert(
                                            TileWireCoord {
                                                cell: nt_far,
                                                wire: wf,
                                            },
                                            n.to_string(),
                                        );
                                    }
                                    FarNaming::BufNear(no, ni) => {
                                        node_names.insert(
                                            TileWireCoord {
                                                cell: nt_far,
                                                wire: wf,
                                            },
                                            no.to_string(),
                                        );
                                        node_wire_bufs.insert(
                                            TileWireCoord {
                                                cell: nt_far,
                                                wire: wf,
                                            },
                                            PipNaming {
                                                tile: RawTileId::from_idx(0),
                                                wire_to: no.to_string(),
                                                wire_from: ni.to_string(),
                                            },
                                        );
                                    }
                                    FarNaming::BufFar(n, no, ni) => {
                                        node_names.insert(
                                            TileWireCoord {
                                                cell: nt_far,
                                                wire: wf,
                                            },
                                            n.to_string(),
                                        );
                                        node_wire_bufs.insert(
                                            TileWireCoord {
                                                cell: nt_far,
                                                wire: wf,
                                            },
                                            PipNaming {
                                                tile: RawTileId::from_idx(1),
                                                wire_to: no.to_string(),
                                                wire_from: ni.to_string(),
                                            },
                                        );
                                    }
                                }
                                ins.insert(TileWireCoord {
                                    cell: nt_far,
                                    wire: wf,
                                });
                            }
                        }
                    }
                    let wt = TileWireCoord {
                        cell: nt_near,
                        wire: wt,
                    };
                    for wf in ins {
                        let mode = self.pip_mode(wt.wire);
                        node_pips.pips.insert((wt, wf.pos()), mode);
                    }
                }
            }
            if let Some((tcid, sb, nnn)) = tcls {
                self.insert_tcls_pips(tcid, sb, node_pips);
                self.insert_tcls_naming(
                    nnn,
                    TileClassNaming {
                        wires: node_names
                            .into_iter()
                            .map(|(k, v)| {
                                (
                                    k,
                                    WireNaming {
                                        name: v,
                                        alt_name: None,
                                        alt_pips_to: Default::default(),
                                        alt_pips_from: Default::default(),
                                    },
                                )
                            })
                            .collect(),
                        wire_bufs: node_wire_bufs,
                        ext_pips: Default::default(),
                        delay_wires: Default::default(),
                        bels: Default::default(),
                        intf_wires_in: Default::default(),
                    },
                );
            } else {
                assert!(node_pips.pips.is_empty());
            }
            // splitters
            let mut snode_pips = Pips::default();
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
                        if self.db[wt] != WireKind::MultiBranch(self.term_slots[dir]) {
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
                            snode_names.insert(
                                TileWireCoord {
                                    cell: snt_near,
                                    wire: wt,
                                },
                                WireNaming {
                                    name: self.rd.wires[wti].clone(),
                                    alt_name: None,
                                    alt_pips_to: Default::default(),
                                    alt_pips_from: Default::default(),
                                },
                            );
                            snode_names.insert(
                                TileWireCoord {
                                    cell: snt_far,
                                    wire: wf,
                                },
                                WireNaming {
                                    name: self.rd.wires[wfi].clone(),
                                    alt_name: None,
                                    alt_pips_to: Default::default(),
                                    alt_pips_from: Default::default(),
                                },
                            );
                            let wt = TileWireCoord {
                                cell: snt_near,
                                wire: wt,
                            };
                            let wf = TileWireCoord {
                                cell: snt_far,
                                wire: wf,
                            };
                            snode_pips.pips.insert((wt, wf.pos()), PipMode::Buf);
                            snode_pips.pips.insert((wf, wt.pos()), PipMode::Buf);
                        }
                    }
                }
            }
            if let Some((tcid, sb, nnn)) = splitter_tcls {
                self.insert_tcls_pips(tcid, sb, snode_pips);
                self.insert_tcls_naming(
                    nnn,
                    TileClassNaming {
                        wires: snode_names,
                        wire_bufs: Default::default(),
                        ext_pips: Default::default(),
                        delay_wires: Default::default(),
                        bels: Default::default(),
                        intf_wires_in: Default::default(),
                    },
                );
            } else {
                assert!(snode_pips.pips.is_empty());
            }
        }

        let term = ConnectorClass {
            slot: self.term_slots[dir],
            wires,
        };
        self.insert_connector(ccid, term);
    }

    pub fn extract_pass_simple_id(
        &mut self,
        ccid_a: ConnectorClassId,
        ccid_b: ConnectorClassId,
        dir: Dir,
        tkn: impl AsRef<str>,
        force_pass: &[WireSlotId],
    ) {
        for &xy in self.rd.tiles_by_kind_name(tkn.as_ref()) {
            if let Some(int_fwd_xy) = self.walk_to_int(xy, dir, false)
                && let Some(int_bwd_xy) = self.walk_to_int(xy, !dir, false)
            {
                self.extract_pass_tile_id(
                    ccid_a, dir, int_bwd_xy, None, None, None, None, None, int_fwd_xy, force_pass,
                );
                self.extract_pass_tile_id(
                    ccid_b, !dir, int_fwd_xy, None, None, None, None, None, int_bwd_xy, force_pass,
                );
            }
        }
    }

    pub fn extract_pass_buf_id(
        &mut self,
        ccid_a: ConnectorClassId,
        ccid_b: ConnectorClassId,
        dir: Dir,
        tkn: impl AsRef<str>,
        naming_a: impl AsRef<str>,
        naming_b: impl AsRef<str>,
        force_pass: &[WireSlotId],
    ) {
        for &xy in self.rd.tiles_by_kind_name(tkn.as_ref()) {
            if let Some(int_fwd_xy) = self.walk_to_int(xy, dir, false)
                && let Some(int_bwd_xy) = self.walk_to_int(xy, !dir, false)
            {
                self.extract_pass_tile_id(
                    ccid_a,
                    dir,
                    int_bwd_xy,
                    Some(xy),
                    None,
                    Some(naming_a.as_ref()),
                    None,
                    None,
                    int_fwd_xy,
                    force_pass,
                );
                self.extract_pass_tile_id(
                    ccid_b,
                    !dir,
                    int_fwd_xy,
                    Some(xy),
                    None,
                    Some(naming_b.as_ref()),
                    None,
                    None,
                    int_bwd_xy,
                    force_pass,
                );
            }
        }
    }

    pub fn extract_intf_tile_multi_id(
        &mut self,
        tcid: TileClassId,
        xy: Coord,
        int_xy: &[Coord],
        naming: impl AsRef<str>,
        bel: BelSlotId,
        sb: Option<BelSlotId>,
        has_out_bufs: bool,
        extract_delay: bool,
    ) {
        let mut x = self
            .xtile_id(tcid, naming.as_ref(), xy)
            .num_cells(int_xy.len())
            .extract_intfs(bel, sb, has_out_bufs);
        if extract_delay {
            x = x.extract_delay();
        }
        for (i, &xy) in int_xy.iter().enumerate() {
            x = x.ref_int(xy, i);
        }
        x.extract();
    }

    pub fn extract_intf_tile_id(
        &mut self,
        tcid: TileClassId,
        xy: Coord,
        int_xy: Coord,
        naming: impl AsRef<str>,
        bel: BelSlotId,
        sb: Option<BelSlotId>,
        has_out_bufs: bool,
        extract_delay: bool,
    ) {
        self.extract_intf_tile_multi_id(
            tcid,
            xy,
            &[int_xy],
            naming,
            bel,
            sb,
            has_out_bufs,
            extract_delay,
        );
    }

    pub fn extract_intf_id(
        &mut self,
        tcid: TileClassId,
        dir: Dir,
        tkn: impl AsRef<str>,
        naming: impl AsRef<str>,
        bel: BelSlotId,
        sb: Option<BelSlotId>,
        has_out_bufs: bool,
        extract_delay: bool,
    ) {
        for &xy in self.rd.tiles_by_kind_name(tkn.as_ref()) {
            let int_xy = self.walk_to_int(xy, !dir, false).unwrap();
            self.extract_intf_tile_id(
                tcid,
                xy,
                int_xy,
                naming.as_ref(),
                bel,
                sb,
                has_out_bufs,
                extract_delay,
            );
        }
    }

    pub fn extract_xtile_id(
        &mut self,
        tcid: TileClassId,
        sb: BelSlotId,
        xy: Coord,
        buf_xy: &[Coord],
        int_xy: &[Coord],
        naming: &str,
        bels: &[ExtrBelInfo],
        skip_wires: &[WireSlotId],
    ) {
        let mut x = self
            .xtile_id(tcid, naming, xy)
            .num_cells(int_xy.len())
            .extract_muxes(sb)
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

    pub fn extract_xtile_bels_id(
        &mut self,
        tcid: TileClassId,
        xy: Coord,
        buf_xy: &[Coord],
        int_xy: &[Coord],
        naming: &str,
        bels: &[ExtrBelInfo],
        force_test_mux_in: bool,
    ) {
        let mut x = self.xtile_id(tcid, naming, xy).num_cells(int_xy.len());
        for &xy in buf_xy {
            x = x.raw_tile(xy);
        }
        for (i, &xy) in int_xy.iter().enumerate() {
            x = x.ref_int(xy, i);
        }
        for bel in bels {
            x = x.bel(bel.clone());
        }
        if force_test_mux_in {
            x = x.force_test_mux_in();
        }
        x.extract();
    }

    pub fn extract_xtile_bels_intf_id(
        &mut self,
        tcid: TileClassId,
        xy: Coord,
        buf_xy: &[Coord],
        int_xy: &[Coord],
        intf_xy: &[(Coord, TileClassNamingId)],
        naming: &str,
        bels: &[ExtrBelInfo],
    ) {
        let mut x = self
            .xtile_id(tcid, naming, xy)
            .num_cells(Ord::max(int_xy.len(), intf_xy.len()));
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

    pub fn xtile_id<'b>(
        &'b mut self,
        tcid: TileClassId,
        naming: impl Into<String>,
        tile: Coord,
    ) -> XTileInfo<'a, 'b> {
        XTileInfo {
            builder: self,
            tcid,
            naming: naming.into(),
            raw_tiles: vec![XTileRawTile {
                xy: tile,
                tile_map: None,
                extract_muxes: false,
            }],
            num_tiles: 1,
            refs: vec![],
            tmux_bel: None,
            intf_sb: None,
            extract_delay: false,
            has_intf_out_bufs: false,
            skip_muxes: BTreeSet::new(),
            optin_muxes: BTreeSet::new(),
            optin_muxes_tile: BTreeSet::new(),
            bels: vec![],
            force_names: HashMap::new(),
            force_skip_pips: HashSet::new(),
            force_pips: HashSet::new(),
            switchbox: None,
            force_test_mux_in: false,
            skip_edges: HashSet::new(),
        }
    }

    pub fn build(mut self) -> (IntDb, NamingDb) {
        for ((tcls, bslot), pips) in self.pips {
            let mut muxes: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
            let mut items = Vec::from_iter(pips.specials);
            let mut passes = BTreeSet::new();
            let mut invs = BTreeSet::new();
            for ((wt, wf), mode) in pips.pips {
                if self.optinvs.contains_key(&wt.wire) {
                    invs.insert(wt);
                }
                match mode {
                    PipMode::Mux => {
                        muxes.entry(wt).or_default().insert(wf);
                    }
                    PipMode::PermaBuf => {
                        items.push(SwitchBoxItem::PermaBuf(PermaBuf { dst: wt, src: wf }));
                    }
                    PipMode::Buf => {
                        items.push(SwitchBoxItem::ProgBuf(ProgBuf {
                            dst: wt,
                            src: wf,
                            bit: PolTileBit::DUMMY,
                        }));
                    }
                    PipMode::Pass => {
                        assert!(!wf.inv);
                        passes.insert((wt, wf.tw));
                    }
                    PipMode::Delay => {
                        items.push(SwitchBoxItem::ProgDelay(ProgDelay {
                            dst: wt,
                            src: wf,
                            bits: Default::default(),
                            steps: vec![Default::default(); 2],
                        }));
                    }
                }
            }
            for &(wt, wf) in &passes {
                if passes.contains(&(wf, wt)) {
                    if wt < wf {
                        items.push(SwitchBoxItem::BiPass(BiPass {
                            a: wt,
                            b: wf,
                            bit: PolTileBit::DUMMY,
                        }));
                    }
                } else {
                    items.push(SwitchBoxItem::Pass(Pass {
                        dst: wt,
                        src: wf,
                        bit: PolTileBit::DUMMY,
                    }));
                }
            }
            for (wt, wf) in muxes {
                items.push(SwitchBoxItem::Mux(Mux {
                    dst: wt,
                    bits: vec![],
                    src: wf.into_iter().map(|k| (k, Default::default())).collect(),
                    bits_off: None,
                }));
            }
            for src in invs {
                let dst = TileWireCoord {
                    cell: src.cell,
                    wire: self.optinvs[&src.wire],
                };
                items.push(SwitchBoxItem::ProgInv(ProgInv {
                    dst,
                    src,
                    bit: PolTileBit::DUMMY,
                }));
            }
            items.sort();
            self.db.tile_classes[tcls]
                .bels
                .insert(bslot, BelInfo::SwitchBox(SwitchBox { items }));
        }

        (self.db, self.ndb)
    }
}
