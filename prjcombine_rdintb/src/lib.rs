#![allow(clippy::too_many_arguments)]
#![allow(clippy::collapsible_else_if)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use prjcombine_entity::{EntityId, EntityMap, EntityPartVec, EntityVec};
use prjcombine_int::db::{
    BelInfo, BelNaming, BelPin, BelPinNaming, Dir, IntDb, IntfInfo, IntfWireInNaming,
    IntfWireOutNaming, MuxInfo, MuxKind, NodeExtPipNaming, NodeKind, NodeKindId, NodeNaming,
    NodeNamingId, NodeRawTileId, NodeTileId, NodeWireId, PinDir, TermInfo, TermKind, TermNamingId,
    TermWireInFarNaming, TermWireOutNaming, WireId, WireInfo, WireKind,
};
use prjcombine_rawdump::{self as rawdump, Coord, Part};

use assert_matches::assert_matches;

use enum_map::{enum_map, EnumMap};

#[derive(Clone, Debug)]
pub struct ExtrBelInfo {
    pub name: String,
    pub slot: Option<rawdump::TkSiteSlot>,
    pub pins: HashMap<String, BelPinInfo>,
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
}

struct BufInfo {
    jumps: HashMap<rawdump::WireId, rawdump::WireId>,
    bufs: HashMap<rawdump::WireId, (PinDir, rawdump::WireId, rawdump::WireId)>,
    int_wires: HashMap<rawdump::WireId, (IntConnKind, NodeWireId)>,
    int_outs: HashMap<rawdump::WireId, (IntConnKind, Vec<(rawdump::WireId, NodeWireId)>)>,
    int_inps: HashMap<rawdump::WireId, (IntConnKind, rawdump::WireId, NodeWireId)>,
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
}

#[derive(Clone, Debug)]
struct NodeType {
    tki: rawdump::TileKindId,
    naming: NodeNamingId,
}

pub struct IntBuilder<'a> {
    rd: &'a Part,
    pub db: IntDb,
    main_passes: EnumMap<Dir, EntityPartVec<WireId, WireId>>,
    node_types: Vec<NodeType>,
    stub_outs: HashSet<String>,
    extra_names: HashMap<String, NodeWireId>,
    extra_names_tile: HashMap<rawdump::TileKindId, HashMap<String, NodeWireId>>,
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
            main_passes: enum_map!(_ => Default::default()),
            node_types: vec![],
            stub_outs: Default::default(),
            extra_names: Default::default(),
            extra_names_tile: Default::default(),
        }
    }

    pub fn bel_virtual(&self, name: impl Into<String>) -> ExtrBelInfo {
        ExtrBelInfo {
            name: name.into(),
            slot: None,
            pins: Default::default(),
        }
    }

    pub fn bel_single(&self, name: impl Into<String>, slot: &str) -> ExtrBelInfo {
        ExtrBelInfo {
            name: name.into(),
            slot: Some(rawdump::TkSiteSlot::Single(
                self.rd.slot_kinds.get(slot).unwrap(),
            )),
            pins: Default::default(),
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
        }
    }

    pub fn bel_xy(&self, name: impl Into<String>, slot: &str, x: u8, y: u8) -> ExtrBelInfo {
        ExtrBelInfo {
            name: name.into(),
            slot: Some(rawdump::TkSiteSlot::Xy(
                self.rd.slot_kinds.get(slot).unwrap(),
                x,
                y,
            )),
            pins: Default::default(),
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
            naming
                .wires_in_far
                .insert(wire, TermWireInFarNaming::Simple(name.to_string()));
        } else {
            assert_matches!(&naming.wires_in_far[wire], TermWireInFarNaming::Simple(n) if n == name);
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
                TermWireInFarNaming::Buf(name_out.to_string(), name_in.to_string()),
            );
        } else {
            assert_matches!(&naming.wires_in_far[wire], TermWireInFarNaming::Buf(no, ni) if no == name_out && ni == name_in);
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
                TermWireInFarNaming::BufFar(
                    name.to_string(),
                    name_out.to_string(),
                    name_in.to_string(),
                ),
            );
        } else {
            assert_matches!(&naming.wires_in_far[wire], TermWireInFarNaming::BufFar(n, no, ni) if n == name && no == name_out && ni == name_in);
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
            naming
                .wires_out
                .insert(wire, TermWireOutNaming::Simple(name.to_string()));
        } else {
            assert_matches!(&naming.wires_out[wire], TermWireOutNaming::Simple(n) if n == name);
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
                TermWireOutNaming::Buf(name_out.to_string(), name_in.to_string()),
            );
        } else {
            assert_matches!(&naming.wires_out[wire], TermWireOutNaming::Buf(no, ni) if no == name_out && ni == name_in);
        }
    }

    pub fn find_wire(&mut self, name: impl AsRef<str>) -> WireId {
        for (k, v) in &self.db.wires {
            if v.name == name.as_ref() {
                return k;
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
        let res = self.db.wires.push(WireInfo {
            name: name.into(),
            kind,
        });
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
        bufs: &[BufInfo],
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
                for (i, bi) in bufs.iter().enumerate() {
                    if let Some(&(d, wt, wf)) = bi.bufs.get(&wn) {
                        if d == dir {
                            let wo = if d == PinDir::Output { wt } else { wf };
                            if let Some(&(ick, w)) = bi.int_wires.get(&wo) {
                                pips.push(NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(1 + i),
                                    wire_to: self.rd.wires[wt].clone(),
                                    wire_from: self.rd.wires[wf].clone(),
                                });
                                return (ick, [w].into_iter().collect(), wn, pips, BTreeMap::new());
                            }
                            if dir == PinDir::Output {
                                if let Some(&(ick, ref ws)) = bi.int_outs.get(&wo) {
                                    pips.push(NodeExtPipNaming {
                                        tile: NodeRawTileId::from_idx(1 + i),
                                        wire_to: self.rd.wires[wt].clone(),
                                        wire_from: self.rd.wires[wf].clone(),
                                    });
                                    if ws.len() == 1 {
                                        pips.push(NodeExtPipNaming {
                                            tile: NodeRawTileId::from_idx(1 + i),
                                            wire_to: self.rd.wires[ws[0].0].clone(),
                                            wire_from: self.rd.wires[wt].clone(),
                                        });
                                        return (
                                            ick,
                                            [ws[0].1].into_iter().collect(),
                                            wn,
                                            pips,
                                            BTreeMap::new(),
                                        );
                                    } else {
                                        let mut int_pips = BTreeMap::new();
                                        let mut int_wires = BTreeSet::new();
                                        for &(fwn, fw) in ws {
                                            int_wires.insert(fw);
                                            int_pips.insert(
                                                fw,
                                                NodeExtPipNaming {
                                                    tile: NodeRawTileId::from_idx(1 + i),
                                                    wire_to: self.rd.wires[fwn].clone(),
                                                    wire_from: self.rd.wires[wt].clone(),
                                                },
                                            );
                                        }
                                        return (ick, int_wires, wn, pips, int_pips);
                                    }
                                }
                            } else {
                                if let Some(&(ick, fwn, fw)) = bi.int_inps.get(&wo) {
                                    pips.push(NodeExtPipNaming {
                                        tile: NodeRawTileId::from_idx(1 + i),
                                        wire_to: self.rd.wires[wt].clone(),
                                        wire_from: self.rd.wires[wf].clone(),
                                    });
                                    pips.push(NodeExtPipNaming {
                                        tile: NodeRawTileId::from_idx(1 + i),
                                        wire_to: self.rd.wires[wf].clone(),
                                        wire_from: self.rd.wires[fwn].clone(),
                                    });
                                    return (
                                        ick,
                                        [fw].into_iter().collect(),
                                        wn,
                                        pips,
                                        BTreeMap::new(),
                                    );
                                }
                            }
                        }
                    }
                    if let Some(&wo) = bi.jumps.get(&wn) {
                        if dir == PinDir::Output {
                            if let Some(&(ick, ref ws)) = bi.int_outs.get(&wo) {
                                if ws.len() == 1 {
                                    pips.push(NodeExtPipNaming {
                                        tile: NodeRawTileId::from_idx(1 + i),
                                        wire_to: self.rd.wires[ws[0].0].clone(),
                                        wire_from: self.rd.wires[wo].clone(),
                                    });
                                    return (
                                        ick,
                                        [ws[0].1].into_iter().collect(),
                                        wn,
                                        pips,
                                        BTreeMap::new(),
                                    );
                                } else {
                                    let mut int_pips = BTreeMap::new();
                                    let mut int_wires = BTreeSet::new();
                                    for &(fwn, fw) in ws {
                                        int_wires.insert(fw);
                                        int_pips.insert(
                                            fw,
                                            NodeExtPipNaming {
                                                tile: NodeRawTileId::from_idx(1 + i),
                                                wire_to: self.rd.wires[fwn].clone(),
                                                wire_from: self.rd.wires[wo].clone(),
                                            },
                                        );
                                    }
                                    return (ick, int_wires, wn, pips, int_pips);
                                }
                            }
                        }
                    }
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
                }
            }
        };
        let walk_count = |dir, mut wn, cnt| {
            let mut pips = Vec::new();
            for left in (0..cnt).rev() {
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
                };
                if let Some(nwn) = nwn {
                    wn = nwn
                } else {
                    if left == 0 {
                        for (i, bi) in bufs.iter().enumerate() {
                            if let Some(&(d, wt, wf)) = bi.bufs.get(&wn) {
                                if d == dir {
                                    pips.push(NodeExtPipNaming {
                                        tile: NodeRawTileId::from_idx(1 + i),
                                        wire_to: self.rd.wires[wt].clone(),
                                        wire_from: self.rd.wires[wf].clone(),
                                    });
                                    return (wn, pips);
                                }
                            }
                        }
                    }
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
                                _ => panic!("bidir pin {}", name),
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
                                _ => panic!("bidir pin {}", name),
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
                                    _ => panic!("bidir pin {}", name),
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
                            println!("NOT FOUND: {}", name);
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
                            println!("NOT FOUND: {}", name);
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
                intfs: Default::default(),
            };
            let mut node_naming = NodeNaming::default();
            let mut names = HashMap::new();
            for &wi in tk.wires.keys() {
                if let Some(&w) = self.extra_names.get(&self.rd.wires[wi]) {
                    names.insert(wi, (IntConnKind::Raw, w));
                    continue;
                }
                if let Some(xn) = self.extra_names_tile.get(&tki) {
                    if let Some(&w) = xn.get(&self.rd.wires[wi]) {
                        names.insert(wi, (IntConnKind::Raw, w));
                        continue;
                    }
                }
            }

            for (&k, &(_, v)) in &names {
                node_naming.wires.insert(v, self.rd.wires[k].clone());
            }

            for &(wfi, wti) in tk.pips.keys() {
                if let Some(&(_, wt)) = names.get(&wti) {
                    match self.db.wires[wt.1].kind {
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

            self.extract_bels(&mut node, &mut node_naming, bels, tki, &names, &[]);

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
                if let Some(&w) = self.extra_names.get(&self.rd.wires[wi]) {
                    names.insert(wi, (IntConnKind::Raw, w));
                    continue;
                }
                if let Some(xn) = self.extra_names_tile.get(&tki) {
                    if let Some(&w) = xn.get(&self.rd.wires[wi]) {
                        names.insert(wi, (IntConnKind::Raw, w));
                        continue;
                    }
                }
            }

            let mut node = NodeKind {
                tiles: [()].into_iter().collect(),
                muxes: Default::default(),
                bels: Default::default(),
                intfs: Default::default(),
            };
            let mut node_naming = NodeNaming::default();
            self.extract_bels(&mut node, &mut node_naming, bels, tki, &names, &[]);

            self.insert_node_merge(kind, node);
            self.insert_node_naming(naming, node_naming);
        }
    }

    pub fn node_type(&mut self, tile_kind: &str, kind: &str, naming: &str) {
        self.extract_node(tile_kind, kind, naming, &[]);
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
                if let Some(&w) = self.extra_names.get(&self.rd.wires[wi]) {
                    res.insert(wi, vec![w.1]);
                } else if let Some(&w) = self
                    .extra_names_tile
                    .get(&tile.kind)
                    .and_then(|x| x.get(&self.rd.wires[wi]))
                {
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
                        IntfInfo::InputDelay => {
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
                        Some(cv @ IntfWireOutNaming::Buf(no, _)) => match v {
                            IntfWireOutNaming::Buf(_, _) => assert_eq!(&v, cv),
                            IntfWireOutNaming::Simple(ono) => assert_eq!(&ono, no),
                        },
                        Some(cv @ IntfWireOutNaming::Simple(n)) => {
                            if let IntfWireOutNaming::Buf(no, _) = &v {
                                assert_eq!(no, n);
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
                            self.db.wires[wt].name,
                            wf.iter()
                                .map(|&x| &self.db.wires[x].name)
                                .collect::<Vec<_>>()
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
                    match self.db.wires[wt].kind {
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
                    match self.db.wires[wt].kind {
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

    pub fn walk_to_int(&self, mut xy: Coord, dir: Dir) -> Option<Coord> {
        loop {
            let tile = &self.rd.tiles[&xy];
            if self.node_types.iter().any(|x| x.tki == tile.kind) {
                return Some(xy);
            }
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
            if let Some(int_xy) = self.walk_to_int(term_xy, !dir) {
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
            if let Some(int_xy) = self.walk_to_int(term_xy, !dir) {
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
            if let Some(int_xy) = self.walk_to_int(term_xy, !dir) {
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
                        match self.db.wires[wt].kind {
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
            // splitters
            let bufs = self.get_bufs(tk);
            for (&wti, &wfi) in bufs.iter() {
                if bufs.get(&wfi) != Some(&wti) {
                    continue;
                }
                if let Some(wtl) = names.get(&wti) {
                    for &wt in wtl {
                        if self.db.wires[wt].kind != WireKind::MultiBranch(dir) {
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
                            node_names.insert((nt_near, wt), self.rd.wires[wti].clone());
                            node_names.insert((nt_far, wf), self.rd.wires[wfi].clone());
                            node_muxes.insert(
                                (nt_near, wt),
                                MuxInfo {
                                    kind: MuxKind::Plain,
                                    ins: [(nt_far, wf)].into_iter().collect(),
                                },
                            );
                            node_muxes.insert(
                                (nt_far, wf),
                                MuxInfo {
                                    kind: MuxKind::Plain,
                                    ins: [(nt_near, wt)].into_iter().collect(),
                                },
                            );
                        }
                    }
                }
            }
            if let Some((nn, nnn)) = node {
                self.insert_node_merge(
                    nn,
                    NodeKind {
                        tiles: node_tiles,
                        muxes: node_muxes,
                        bels: Default::default(),
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
                        intf_wires_in: Default::default(),
                        intf_wires_out: Default::default(),
                    },
                );
            } else {
                assert!(node_muxes.is_empty());
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
            if let Some(int_fwd_xy) = self.walk_to_int(xy, dir) {
                if let Some(int_bwd_xy) = self.walk_to_int(xy, !dir) {
                    self.extract_pass_tile(
                        format!("{}.{}", name.as_ref(), dir),
                        dir,
                        int_bwd_xy,
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
            if let Some(int_fwd_xy) = self.walk_to_int(xy, dir) {
                if let Some(int_bwd_xy) = self.walk_to_int(xy, !dir) {
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

    /*
    pub fn name_intf_out_wire(
        &mut self,
        naming: IntfNamingId,
        wire: WireId,
        name: impl AsRef<str>,
    ) {
        let name = name.as_ref();
        let naming = &mut self.db.intf_namings[naming];
        if !naming.wires_out.contains_id(wire) {
            naming
                .wires_out
                .insert(wire, IntfWireOutNaming::Simple(name.to_string()));
        } else {
            assert_matches!(&naming.wires_out[wire], IntfWireOutNaming::Simple(n) | IntfWireOutNaming::Buf(n, _) if n == name);
        }
    }

    pub fn name_intf_out_wire_in(
        &mut self,
        naming: IntfNamingId,
        wire: WireId,
        name: impl AsRef<str>,
    ) {
        let name = name.as_ref();
        let naming = &mut self.db.intf_namings[naming];
        match naming.wires_out[wire] {
            IntfWireOutNaming::Simple(ref n) => {
                let n = n.clone();
                naming.wires_out[wire] = IntfWireOutNaming::Buf(n, name.to_string());
            }
            IntfWireOutNaming::Buf(_, ref n) => assert_eq!(n, name),
        }
    }
    */

    pub fn extract_intf_tile_multi(
        &mut self,
        name: impl AsRef<str>,
        xy: Coord,
        int_xy: &[Coord],
        naming: impl AsRef<str>,
        has_out_bufs: bool,
    ) {
        let names = self.extract_names(xy, int_xy);
        let tile = &self.rd.tiles[&xy];
        let tk = &self.rd.tile_kinds[tile.kind];
        let mut out_muxes: HashMap<NodeWireId, Vec<NodeWireId>> = HashMap::new();
        let bufs = self.get_bufs(tk);
        let mut intfs = BTreeMap::new();
        let mut delayed = HashMap::new();
        let mut intf_wires_in = BTreeMap::new();
        let mut intf_wires_out = BTreeMap::new();
        if has_out_bufs {
            for (&wdi, &wfi) in &bufs {
                if let Some(&(_, wf)) = names.get(&wfi) {
                    if !matches!(self.db.wires[wf.1].kind, WireKind::MuxOut) {
                        continue;
                    }
                    for &wti in tk.wires.keys() {
                        if tk.pips.contains_key(&(wfi, wti)) && tk.pips.contains_key(&(wdi, wti)) {
                            intf_wires_in.insert(
                                wf,
                                IntfWireInNaming::Delay(
                                    self.rd.wires[wti].clone(),
                                    self.rd.wires[wdi].clone(),
                                    self.rd.wires[wfi].clone(),
                                ),
                            );
                            delayed.insert(wti, wf);
                            intfs.insert(wf, IntfInfo::InputDelay);
                        }
                    }
                }
            }
        }
        for &(wfi, wti) in tk.pips.keys() {
            if let Some(&(_, wt)) = names.get(&wti) {
                if !matches!(self.db.wires[wt.1].kind, WireKind::LogicOut) {
                    continue;
                }
                intf_wires_out
                    .entry(wt)
                    .or_insert_with(|| IntfWireOutNaming::Simple(self.rd.wires[wti].clone()));
                let mut rwfi = wfi;
                if bufs.contains_key(&wfi) {
                    rwfi = bufs[&wfi];
                }
                if let Some(&(_, wf)) = names.get(&rwfi) {
                    if rwfi != wfi {
                        intf_wires_in.insert(
                            wf,
                            IntfWireInNaming::TestBuf(
                                self.rd.wires[wfi].clone(),
                                self.rd.wires[rwfi].clone(),
                            ),
                        );
                    } else {
                        intf_wires_in
                            .insert(wf, IntfWireInNaming::Simple(self.rd.wires[wfi].clone()));
                    }
                    assert!(!intfs.contains_key(&wf));
                    out_muxes.entry(wt).or_default().push(wf);
                } else if let Some(&wf) = delayed.get(&wfi) {
                    out_muxes.entry(wt).or_default().push(wf);
                } else if has_out_bufs {
                    out_muxes.entry(wt).or_default();
                    intf_wires_out.insert(
                        wt,
                        IntfWireOutNaming::Buf(
                            self.rd.wires[wti].clone(),
                            self.rd.wires[wfi].clone(),
                        ),
                    );
                }
            }
        }
        for (k, v) in out_muxes {
            intfs.insert(k, IntfInfo::OutputTestMux(v.into_iter().collect()));
        }
        let node = NodeKind {
            tiles: [()].into_iter().collect(),
            muxes: Default::default(),
            bels: Default::default(),
            intfs,
        };
        self.insert_node_merge(name.as_ref(), node);
        let node_naming = NodeNaming {
            wires: Default::default(),
            wire_bufs: Default::default(),
            ext_pips: Default::default(),
            bels: Default::default(),
            intf_wires_in,
            intf_wires_out,
        };
        self.insert_node_naming(naming.as_ref(), node_naming);
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
            let int_xy = self.walk_to_int(xy, !dir).unwrap();
            self.extract_intf_tile(name.as_ref(), xy, int_xy, naming.as_ref(), has_out_bufs);
        }
    }

    pub fn extract_names(
        &self,
        xy: Coord,
        int_xy: &[Coord],
    ) -> HashMap<rawdump::WireId, (IntConnKind, NodeWireId)> {
        let tile = &self.rd.tiles[&xy];
        let tk = &self.rd.tile_kinds[tile.kind];
        let mut names = HashMap::new();
        let node2wires: EntityVec<NodeTileId, _> = int_xy
            .iter()
            .copied()
            .map(|x| self.get_int_node2wires(x))
            .collect();
        for (_, &wi, &tkw) in tk.wires.iter() {
            if let Some(&w) = self.extra_names.get(&self.rd.wires[wi]) {
                names.insert(wi, (IntConnKind::Raw, w));
                continue;
            }
            if let Some(xn) = self.extra_names_tile.get(&tile.kind) {
                if let Some(&w) = xn.get(&self.rd.wires[wi]) {
                    names.insert(wi, (IntConnKind::Raw, w));
                    continue;
                }
            }
            if let rawdump::TkWire::Connected(idx) = tkw {
                if let Some(&nidx) = tile.conn_wires.get(idx) {
                    for (k, v) in &node2wires {
                        if let Some(w) = v.get(&nidx) {
                            if w.len() == 1 {
                                names.insert(wi, (IntConnKind::Raw, (k, w[0])));
                                break;
                            }
                            let fw: Vec<_> = w
                                .iter()
                                .copied()
                                .filter(|&x| !matches!(self.db.wires[x].kind, WireKind::Branch(_)))
                                .collect();
                            if fw.len() == 1 {
                                names.insert(wi, (IntConnKind::Raw, (k, fw[0])));
                                break;
                            }
                        }
                    }
                }
            }
        }
        names
    }

    pub fn extract_intf_names(
        &self,
        xy: Coord,
        intf_xy: &[(Coord, NodeNamingId)],
    ) -> HashMap<rawdump::WireId, (IntConnKind, NodeWireId)> {
        let mut node2wire = HashMap::new();
        for (i, &(intf_xy, nid)) in intf_xy.iter().enumerate() {
            let tid = NodeTileId::from_idx(i);
            let naming = &self.db.node_namings[nid];
            let mut name2wire = HashMap::new();
            for (k, v) in &naming.intf_wires_in {
                match v {
                    IntfWireInNaming::Simple(n) | IntfWireInNaming::TestBuf(_, n) => {
                        name2wire.insert(&n[..], (IntConnKind::Raw, (tid, k.1)));
                    }
                    IntfWireInNaming::Buf(n, _) | IntfWireInNaming::Delay(n, _, _) => {
                        name2wire.insert(&n[..], (IntConnKind::IntfIn, (tid, k.1)));
                    }
                }
            }
            for (k, v) in &naming.intf_wires_out {
                match v {
                    IntfWireOutNaming::Simple(n) => {
                        name2wire.insert(&n[..], (IntConnKind::Raw, (tid, k.1)));
                    }
                    IntfWireOutNaming::Buf(n, ns) => {
                        name2wire.insert(&n[..], (IntConnKind::Raw, (tid, k.1)));
                        name2wire.insert(&ns[..], (IntConnKind::IntfOut, (tid, k.1)));
                    }
                }
            }
            let intf_tile = &self.rd.tiles[&intf_xy];
            let intf_tk = &self.rd.tile_kinds[intf_tile.kind];
            for (_, &wi, &tkw) in intf_tk.wires.iter() {
                if let Some(&wd) = name2wire.get(&&self.rd.wires[wi][..]) {
                    if let rawdump::TkWire::Connected(idx) = tkw {
                        if let Some(&nidx) = intf_tile.conn_wires.get(idx) {
                            node2wire.insert(nidx, wd);
                        }
                    }
                }
            }
        }

        let tile = &self.rd.tiles[&xy];
        let tk = &self.rd.tile_kinds[tile.kind];
        let mut names = HashMap::new();

        for (_, &wi, &tkw) in tk.wires.iter() {
            if let Some(&w) = self.extra_names.get(&self.rd.wires[wi]) {
                names.insert(wi, (IntConnKind::Raw, w));
                continue;
            }
            if let Some(xn) = self.extra_names_tile.get(&tile.kind) {
                if let Some(&w) = xn.get(&self.rd.wires[wi]) {
                    names.insert(wi, (IntConnKind::Raw, w));
                    continue;
                }
            }
            if let rawdump::TkWire::Connected(idx) = tkw {
                if let Some(&nidx) = tile.conn_wires.get(idx) {
                    if let Some(&wd) = node2wire.get(&nidx) {
                        names.insert(wi, wd);
                    }
                }
            }
        }
        names
    }

    pub fn extract_buf_jumps(
        &self,
        xy: Coord,
        buf_xy: Coord,
    ) -> HashMap<rawdump::WireId, rawdump::WireId> {
        let mut names = HashMap::new();
        let buf_tile = &self.rd.tiles[&buf_xy];
        let buf_tk = &self.rd.tile_kinds[buf_tile.kind];
        let mut node2wires: HashMap<_, Vec<_>> = HashMap::new();
        for &w in buf_tk.wires.keys() {
            if let Some(node) = self.get_node(buf_tile, buf_tk, w) {
                node2wires.entry(node).or_default().push(w);
            }
        }
        let tile = &self.rd.tiles[&xy];
        let tk = &self.rd.tile_kinds[tile.kind];
        for &w in tk.wires.keys() {
            if let Some(node) = self.get_node(tile, tk, w) {
                if let Some(ws) = node2wires.get(&node) {
                    if ws.len() == 1 {
                        names.insert(w, ws[0]);
                    }
                }
            }
        }
        names
    }

    pub fn extract_buf_names(
        &self,
        xy: Coord,
        buf_xy: Coord,
    ) -> HashMap<rawdump::WireId, (PinDir, rawdump::WireId, rawdump::WireId)> {
        let mut names = HashMap::new();
        let buf_tile = &self.rd.tiles[&buf_xy];
        let buf_tk = &self.rd.tile_kinds[buf_tile.kind];
        let mut edges_in: HashMap<_, Vec<_>> = HashMap::new();
        let mut edges_out: HashMap<_, Vec<_>> = HashMap::new();
        for &(wfi, wti) in buf_tk.pips.keys() {
            edges_in.entry(wti).or_default().push(wfi);
            edges_out.entry(wfi).or_default().push(wti);
        }
        let mut buf_out = HashMap::new();
        for (wt, wfs) in edges_out {
            if let Some(node) = self.get_node(buf_tile, buf_tk, wt) {
                if wfs.len() == 1 {
                    buf_out.insert(node, (wt, wfs[0]));
                }
            }
        }
        let mut buf_in = HashMap::new();
        for (wt, wfs) in edges_in {
            if let Some(node) = self.get_node(buf_tile, buf_tk, wt) {
                if wfs.len() == 1 {
                    buf_in.insert(node, (wt, wfs[0]));
                }
            }
        }
        let tile = &self.rd.tiles[&xy];
        let tk = &self.rd.tile_kinds[tile.kind];
        for &w in tk.wires.keys() {
            if let Some(node) = self.get_node(tile, tk, w) {
                if let Some(&(wf, wt)) = buf_out.get(&node) {
                    names.insert(w, (PinDir::Output, wt, wf));
                }
                if let Some(&(wt, wf)) = buf_in.get(&node) {
                    names.insert(w, (PinDir::Input, wt, wf));
                }
            }
        }
        names
    }

    pub fn extract_int_outs(
        &self,
        xy: Coord,
        int_wires: &HashMap<rawdump::WireId, (IntConnKind, NodeWireId)>,
    ) -> HashMap<rawdump::WireId, (IntConnKind, Vec<(rawdump::WireId, NodeWireId)>)> {
        let tile = &self.rd.tiles[&xy];
        let tk = &self.rd.tile_kinds[tile.kind];
        let mut res = HashMap::new();
        let mut edges_out: HashMap<_, Vec<_>> = HashMap::new();
        for &(wfi, wti) in tk.pips.keys() {
            edges_out.entry(wfi).or_default().push(wti);
        }
        for (wf, wts) in edges_out {
            if wts.iter().all(|x| int_wires.contains_key(x)) {
                let ick = int_wires[&wts[0]].0;
                res.insert(
                    wf,
                    (ick, wts.into_iter().map(|x| (x, int_wires[&x].1)).collect()),
                );
            }
        }
        res
    }

    pub fn extract_int_inps(
        &self,
        xy: Coord,
        int_wires: &HashMap<rawdump::WireId, (IntConnKind, NodeWireId)>,
    ) -> HashMap<rawdump::WireId, (IntConnKind, rawdump::WireId, NodeWireId)> {
        let tile = &self.rd.tiles[&xy];
        let tk = &self.rd.tile_kinds[tile.kind];
        let mut res = HashMap::new();
        let mut edges_in: HashMap<_, Vec<_>> = HashMap::new();
        for &(wfi, wti) in tk.pips.keys() {
            edges_in.entry(wti).or_default().push(wfi);
        }
        for (wt, wfs) in edges_in {
            if wfs.len() == 1 && int_wires.contains_key(&wfs[0]) {
                res.insert(wt, (int_wires[&wfs[0]].0, wfs[0], int_wires[&wfs[0]].1));
            }
        }
        res
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
        let tile = &self.rd.tiles[&xy];
        let tk = &self.rd.tile_kinds[tile.kind];
        let names = self.extract_names(xy, int_xy);

        let mut node = NodeKind {
            tiles: int_xy.iter().map(|_| ()).collect(),
            muxes: Default::default(),
            bels: Default::default(),
            intfs: Default::default(),
        };
        let mut node_naming = NodeNaming::default();

        for &(wfi, wti) in tk.pips.keys() {
            if let Some(&(_, wt)) = names.get(&wti) {
                if skip_wires.contains(&wt.1) {
                    continue;
                }
                if matches!(self.db.wires[wt.1].kind, WireKind::LogicOut) {
                    continue;
                }
                if let Some(&(_, wf)) = names.get(&wfi) {
                    node_naming.wires.insert(wt, self.rd.wires[wti].to_string());
                    node_naming.wires.insert(wf, self.rd.wires[wfi].to_string());
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
                        "UNEXPECTED XNODE MUX IN {} {} {} {}",
                        self.rd.tile_kinds.key(tile.kind),
                        tile.name,
                        self.rd.wires[wti],
                        self.rd.wires[wfi]
                    );
                }
            }
        }

        let bufs: Vec<_> = buf_xy
            .iter()
            .map(|&x| {
                let int_wires = self.extract_names(x, int_xy);
                let int_outs = self.extract_int_outs(x, &int_wires);
                let int_inps = self.extract_int_inps(x, &int_wires);
                let jumps = self.extract_buf_jumps(xy, x);
                BufInfo {
                    jumps,
                    bufs: self.extract_buf_names(xy, x),
                    int_outs,
                    int_inps,
                    int_wires,
                }
            })
            .collect();
        self.extract_bels(&mut node, &mut node_naming, bels, tile.kind, &names, &bufs);

        self.insert_node_merge(name, node);
        self.insert_node_naming(naming, node_naming);
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
        let tile = &self.rd.tiles[&xy];
        let names = self.extract_names(xy, int_xy);

        let mut node = NodeKind {
            tiles: int_xy.iter().map(|_| ()).collect(),
            muxes: Default::default(),
            bels: Default::default(),
            intfs: Default::default(),
        };
        let mut node_naming = NodeNaming::default();

        let bufs: Vec<_> = buf_xy
            .iter()
            .map(|&x| {
                let int_wires = self.extract_names(x, int_xy);
                let int_outs = self.extract_int_outs(x, &int_wires);
                let int_inps = self.extract_int_inps(x, &int_wires);
                let jumps = self.extract_buf_jumps(xy, x);
                BufInfo {
                    jumps,
                    bufs: self.extract_buf_names(xy, x),
                    int_outs,
                    int_inps,
                    int_wires,
                }
            })
            .collect();
        self.extract_bels(&mut node, &mut node_naming, bels, tile.kind, &names, &bufs);

        self.insert_node_merge(name, node);
        self.insert_node_naming(naming, node_naming);
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
        let tile = &self.rd.tiles[&xy];
        let mut names = self.extract_names(xy, int_xy);
        names.extend(self.extract_intf_names(xy, intf_xy));

        let mut node = NodeKind {
            tiles: intf_xy.iter().map(|_| ()).collect(),
            muxes: Default::default(),
            bels: Default::default(),
            intfs: Default::default(),
        };
        let mut node_naming = NodeNaming::default();

        let bufs: Vec<_> = buf_xy
            .iter()
            .map(|&x| {
                let mut int_wires = self.extract_names(x, int_xy);
                int_wires.extend(self.extract_intf_names(x, intf_xy));
                let int_outs = self.extract_int_outs(x, &int_wires);
                let int_inps = self.extract_int_inps(x, &int_wires);
                let jumps = self.extract_buf_jumps(xy, x);
                BufInfo {
                    jumps,
                    bufs: self.extract_buf_names(xy, x),
                    int_outs,
                    int_inps,
                    int_wires,
                }
            })
            .collect();
        self.extract_bels(&mut node, &mut node_naming, bels, tile.kind, &names, &bufs);

        self.insert_node_merge(name, node);
        self.insert_node_naming(naming, node_naming);
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
            intfs: Default::default(),
        };
        let node_naming = NodeNaming {
            wires: Default::default(),
            wire_bufs: Default::default(),
            ext_pips: Default::default(),
            bels: naming_bels,
            intf_wires_in: Default::default(),
            intf_wires_out: Default::default(),
        };
        self.insert_node_merge(name, node);
        self.insert_node_naming(naming, node_naming);
    }

    pub fn build(self) -> IntDb {
        self.db
    }
}