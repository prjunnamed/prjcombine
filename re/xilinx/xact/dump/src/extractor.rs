use std::{
    cmp::min,
    collections::{btree_map, BTreeMap, BTreeSet},
};

use ndarray::Array2;
use prjcombine_interconnect::{
    db::{BelId, Dir, IntDb, MuxInfo, MuxKind, NodeKindId, NodeWireId, WireId, WireKind},
    grid::{ColId, DieId, ExpandedGrid, IntWire, NodeLoc, RowId},
};
use prjcombine_re_xilinx_xact_data::die::{BoxId, Die, PrimId};
use prjcombine_re_xilinx_xact_naming::{
    db::{IntPipNaming, NamingDb, NodeNamingId, NodeRawTileId, PipNaming},
    grid::ExpandedGridNaming,
};
use unnamed_entity::{entity_id, EntityBitVec, EntityId, EntityMap, EntityPartVec, EntityVec};

entity_id! {
    pub id NetId u32, reserve 1;
}

pub struct PrimExtractor<'a> {
    pub name: &'a str,
    pub pins: BTreeMap<&'a str, NetId>,
}

impl<'a> PrimExtractor<'a> {
    pub fn get_pin(&mut self, name: &'a str) -> NetId {
        self.pins
            .remove(&name)
            .unwrap_or_else(|| panic!("prim {prim} has no pin {name}", prim = self.name))
    }
}

impl Drop for PrimExtractor<'_> {
    fn drop(&mut self) {
        for pin in self.pins.keys() {
            eprintln!("UMM pin {pin} unaccounted for in {prim}", prim = self.name);
        }
    }
}

pub struct Extractor<'a> {
    pub die: &'a Die,
    pub matrix: Array2<u16>,
    pub matrix_nets: Array2<MatrixCell>,
    pub nets: EntityVec<NetId, Net<'a>>,
    pub prims_by_name_a: BTreeMap<&'a str, PrimId>,
    pub prims_by_name_i: BTreeMap<&'a str, PrimId>,
    pub int_nets: BTreeMap<IntWire, NetId>,
    pub bel_nets: BTreeMap<(NodeLoc, BelId, &'a str), NetId>,
    pub egrid: &'a ExpandedGrid<'a>,
    pub ngrid: &'a ExpandedGridNaming<'a>,
    pub used_prims: EntityBitVec<PrimId>,
    pub box_owner: EntityPartVec<BoxId, NodeLoc>,
    pub pip_owner: BTreeMap<(NetId, NetId), NodeLoc>,
    pub tbuf_pseudos: BTreeSet<(NetId, NetId)>,
    pub int_pip_force_dst: BTreeMap<(NetId, NetId), NodeWireId>,
    pub used_pips: BTreeSet<(NetId, NetId)>,
    pub bel_pips: EntityVec<NodeNamingId, BTreeMap<(BelId, String), PipNaming>>,
    pub node_muxes: EntityPartVec<NodeKindId, BTreeMap<NodeWireId, MuxInfo>>,
    pub int_pips: EntityPartVec<NodeNamingId, BTreeMap<(NodeWireId, NodeWireId), IntPipNaming>>,
    pub net_by_tile_override: BTreeMap<(ColId, RowId), BTreeMap<NetId, WireId>>,
    pub junk_prim_names: BTreeSet<String>,
}

pub struct Finisher {
    pub bel_pips: EntityVec<NodeNamingId, BTreeMap<(BelId, String), PipNaming>>,
    pub node_muxes: EntityPartVec<NodeKindId, BTreeMap<NodeWireId, MuxInfo>>,
    pub int_pips: EntityPartVec<NodeNamingId, BTreeMap<(NodeWireId, NodeWireId), IntPipNaming>>,
}

#[derive(Debug)]
pub struct Net<'a> {
    pub root: (usize, usize, Dir),
    pub binding: NetBinding<'a>,
    pub pips_fwd: BTreeMap<NetId, (usize, usize)>,
    pub pips_bwd: BTreeMap<NetId, (usize, usize)>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NetBinding<'a> {
    None,
    Dummy,
    Int(IntWire),
    Bel(NodeLoc, BelId, &'a str),
}

#[derive(Copy, Clone, Debug)]
pub struct MatrixCell {
    pub net_l: Option<NetId>,
    pub net_b: Option<NetId>,
}

impl<'a> Extractor<'a> {
    pub fn new(die: &'a Die, egrid: &'a ExpandedGrid, ngrid: &'a ExpandedGridNaming) -> Self {
        let matrix = die.make_unified_matrix();
        let matrix_nets = Array2::from_elem(
            (matrix.dim().0 + 1, matrix.dim().1 + 1),
            MatrixCell {
                net_l: None,
                net_b: None,
            },
        );
        let mut res = Self {
            die,
            matrix,
            matrix_nets,
            nets: EntityVec::new(),
            int_nets: BTreeMap::new(),
            bel_nets: BTreeMap::new(),
            egrid,
            ngrid,
            used_prims: EntityBitVec::repeat(false, die.prims.len()),
            box_owner: EntityPartVec::new(),
            pip_owner: BTreeMap::new(),
            tbuf_pseudos: Default::default(),
            int_pip_force_dst: Default::default(),
            used_pips: Default::default(),
            prims_by_name_a: Default::default(),
            prims_by_name_i: Default::default(),
            bel_pips: ngrid
                .db
                .node_namings
                .ids()
                .map(|_| Default::default())
                .collect(),
            node_muxes: EntityPartVec::new(),
            int_pips: EntityPartVec::new(),
            net_by_tile_override: Default::default(),
            junk_prim_names: Default::default(),
        };
        res.build_nets();
        res.build_net_pips();
        res.build_prims();
        res
    }

    pub fn get_net(&self, col: usize, row: usize, dir: Dir) -> Option<NetId> {
        match dir {
            Dir::W => self.matrix_nets[(col, row)].net_l,
            Dir::E => self.matrix_nets[(col + 1, row)].net_l,
            Dir::S => self.matrix_nets[(col, row)].net_b,
            Dir::N => self.matrix_nets[(col, row + 1)].net_b,
        }
    }

    fn build_nets(&mut self) {
        for col in 0..self.matrix.dim().0 {
            for row in 0..self.matrix.dim().1 {
                let cv = usize::from(self.matrix[(col, row)] & 0xff);
                for dir in [Dir::N, Dir::E, Dir::S, Dir::W] {
                    if self.die.matrix_cells_fwd[dir][cv] == 0
                        && self.die.matrix_cells_bwd[dir][cv] == 0
                    {
                        continue;
                    }
                    if self.get_net(col, row, dir).is_some() {
                        continue;
                    }
                    let net = self.nets.push(Net {
                        root: (col, row, dir),
                        binding: NetBinding::None,
                        pips_fwd: Default::default(),
                        pips_bwd: Default::default(),
                    });
                    let mut queue = vec![(col, row, dir)];
                    for (i, odir) in [Dir::N, Dir::E, Dir::S, Dir::W].into_iter().enumerate() {
                        if (self.die.matrix_cells_fwd[dir][cv] >> i & 1) != 0 {
                            queue.push((col, row, odir));
                        }
                    }
                    while let Some((col, row, dir)) = queue.pop() {
                        let pnet = match dir {
                            Dir::W => &mut self.matrix_nets[(col, row)].net_l,
                            Dir::E => &mut self.matrix_nets[(col + 1, row)].net_l,
                            Dir::S => &mut self.matrix_nets[(col, row)].net_b,
                            Dir::N => &mut self.matrix_nets[(col, row + 1)].net_b,
                        };
                        if let Some(cnet) = *pnet {
                            panic!("hit already-filled net {cnet} while filling {net}");
                        }
                        *pnet = Some(net);
                        let (ncol, nrow) = match dir {
                            Dir::W => (col - 1, row),
                            Dir::E => (col + 1, row),
                            Dir::S => (col, row - 1),
                            Dir::N => (col, row + 1),
                        };
                        let ndir = !dir;
                        let cv = usize::from(self.matrix[(ncol, nrow)] & 0xff);
                        for (i, odir) in [Dir::N, Dir::E, Dir::S, Dir::W].into_iter().enumerate() {
                            if (self.die.matrix_cells_fwd[ndir][cv] >> i & 1) != 0 {
                                queue.push((ncol, nrow, odir));
                            }
                        }
                    }
                }
            }
        }
    }

    fn build_net_pips(&mut self) {
        for col in 0..self.matrix.dim().0 {
            for row in 0..self.matrix.dim().1 {
                let cv = usize::from(self.matrix[(col, row)] & 0xff);
                for dir in [Dir::N, Dir::E, Dir::S, Dir::W] {
                    if (self.die.matrix_cells_fwd[dir][cv] & 0xf0) == 0 {
                        continue;
                    }
                    let net_f = self.get_net(col, row, dir).unwrap();
                    for (i, odir) in [Dir::N, Dir::E, Dir::S, Dir::W].into_iter().enumerate() {
                        if (self.die.matrix_cells_fwd[dir][cv] >> i & 0x10) != 0 {
                            let net_t = self.get_net(col, row, odir).unwrap();
                            match self.nets[net_f].pips_fwd.entry(net_t) {
                                btree_map::Entry::Vacant(entry) => {
                                    entry.insert((col, row));
                                }
                                btree_map::Entry::Occupied(entry) => {
                                    assert_eq!(*entry.get(), (col, row));
                                }
                            }
                            match self.nets[net_t].pips_bwd.entry(net_f) {
                                btree_map::Entry::Vacant(entry) => {
                                    entry.insert((col, row));
                                }
                                btree_map::Entry::Occupied(entry) => {
                                    assert_eq!(*entry.get(), (col, row));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn build_prims(&mut self) {
        for (prim_id, prim) in &self.die.prims {
            if prim.name_a.is_empty() {
                self.prims_by_name_i.insert(&prim.name_i, prim_id);
            } else {
                self.prims_by_name_a.insert(&prim.name_a, prim_id);
            }
        }
    }

    pub fn box_net(&self, box_id: BoxId, pin: usize) -> NetId {
        let (x, y, dir) = self.die.box_pin(box_id, pin);
        self.get_net(x, y, dir).unwrap()
    }

    pub fn net_bel(&mut self, net_id: NetId, nloc: NodeLoc, bel: BelId, key: &'a str) {
        let net = &mut self.nets[net_id];
        let nbind = NetBinding::Bel(nloc, bel, key);
        if net.binding == NetBinding::None {
            match self.bel_nets.entry((nloc, bel, key)) {
                btree_map::Entry::Vacant(entry) => {
                    entry.insert(net_id);
                    net.binding = nbind;
                }
                btree_map::Entry::Occupied(entry) => {
                    let (nx, ny, nd) = net.root;
                    let (cx, cy, cd) = self.nets[*entry.get()].root;
                    eprintln!("BEL NET ALREADY USED: {nloc:?} {bel} {key} is {cx}.{cy}.{cd} setting {nx}.{ny}.{nd}")
                }
            }
        } else if net.binding != nbind {
            let (nx, ny, nd) = net.root;
            eprintln!(
                "NET {nx}.{ny}.{nd} ALREADY BOUND: is {bind:?} setting {nbind:?}",
                bind = net.binding,
            );
        }
    }

    pub fn net_int(&mut self, net_id: NetId, wire: IntWire) {
        let wire = self.egrid.resolve_wire_nobuf(wire).unwrap();
        let net = &mut self.nets[net_id];
        let nbind = NetBinding::Int(wire);
        if net.binding == NetBinding::None {
            match self.int_nets.entry(wire) {
                btree_map::Entry::Vacant(entry) => {
                    entry.insert(net_id);
                    net.binding = nbind;
                }
                btree_map::Entry::Occupied(entry) => {
                    let (nx, ny, nd) = net.root;
                    let (cx, cy, cd) = self.nets[*entry.get()].root;
                    eprintln!(
                        "INT NET ALREADY USED: {wire:?} is {cx}.{cy}.{cd} setting {nx}.{ny}.{nd}"
                    )
                }
            }
        } else if net.binding != nbind {
            let (nx, ny, nd) = net.root;
            eprintln!(
                "NET {nx}.{ny}.{nd} ALREADY BOUND: is {bind:?} setting {nbind:?}",
                bind = net.binding,
            );
        }
    }

    pub fn net_dummy(&mut self, net_id: NetId) {
        let net = &mut self.nets[net_id];
        let nbind = NetBinding::Dummy;
        if net.binding == NetBinding::None {
            net.binding = nbind;
        } else if net.binding != nbind {
            let (nx, ny, nd) = net.root;
            eprintln!(
                "NET {nx}.{ny}.{nd} ALREADY BOUND: is {bind:?} setting {nbind:?}",
                bind = net.binding,
            );
        }
    }

    pub fn net_bel_int(&mut self, net_id: NetId, nloc: NodeLoc, bel: BelId, pin: &'a str) {
        let node = &self.egrid.die(nloc.0)[(nloc.1, nloc.2)].nodes[nloc.3];
        for &wire in &self.egrid.db.nodes[node.kind].bels[bel].pins[pin].wires {
            let wire = (nloc.0, node.tiles[wire.0], wire.1);
            self.net_int(net_id, wire);
        }
    }

    pub fn get_int_net(&self, nloc: NodeLoc, nw: NodeWireId) -> NetId {
        let node = &self.egrid.die(nloc.0)[(nloc.1, nloc.2)].nodes[nloc.3];
        let w = self
            .egrid
            .resolve_wire_nobuf((nloc.0, node.tiles[nw.0], nw.1))
            .unwrap();
        self.int_nets[&w]
    }

    pub fn get_bel_int_net(&self, nloc: NodeLoc, bel: BelId, pin: &'a str) -> NetId {
        let node = &self.egrid.die(nloc.0)[(nloc.1, nloc.2)].nodes[nloc.3];
        let nw = *self.egrid.db.nodes[node.kind].bels[bel].pins[pin]
            .wires
            .iter()
            .next()
            .unwrap();
        self.get_int_net(nloc, nw)
    }

    #[track_caller]
    pub fn get_bel_net(&self, nloc: NodeLoc, bel: BelId, pin: &'a str) -> NetId {
        self.bel_nets[&(nloc, bel, pin)]
    }

    pub fn xlat_pip_loc(&self, nloc: NodeLoc, crd: (usize, usize)) -> PipNaming {
        let nnode = &self.ngrid.nodes[&nloc];
        for (rt, (xr, yr)) in &nnode.coords {
            if xr.contains(&crd.0) && yr.contains(&crd.1) {
                return PipNaming {
                    rt,
                    x: crd.0 - xr.start,
                    y: crd.1 - yr.start,
                };
            }
        }
        panic!("can't xlat pip {crd:?} in {nloc:?}");
    }

    pub fn use_pip(&mut self, net_t: NetId, net_f: NetId) -> (usize, usize) {
        let crd = self.nets[net_f].pips_fwd[&net_t];
        if !self.used_pips.insert((net_t, net_f)) {
            let (tx, ty, td) = self.nets[net_t].root;
            let (fx, fy, fd) = self.nets[net_f].root;
            let tb = self.nets[net_t].binding;
            let fb = self.nets[net_f].binding;
            eprintln!(
                "DOUBLE CLAIMED PIP at {crd:?} {tx}.{ty}.{td} [{tb:?}] <- {fx}.{fy}.{fd} [{fb:?}]"
            );
        }
        crd
    }

    pub fn consume_all_fwd(&mut self, net_id: NetId, nloc: NodeLoc) -> Vec<(NetId, PipNaming)> {
        let net = &self.nets[net_id];
        let mut res = vec![];
        for (net_t, crd) in net.pips_fwd.clone() {
            self.use_pip(net_t, net_id);
            let pip = self.xlat_pip_loc(nloc, crd);
            res.push((net_t, pip));
        }
        res
    }

    pub fn consume_all_bwd(&mut self, net_id: NetId, nloc: NodeLoc) -> Vec<(NetId, PipNaming)> {
        let net = &self.nets[net_id];
        let mut res = vec![];
        for (net_f, crd) in net.pips_bwd.clone() {
            self.use_pip(net_id, net_f);
            let pip = self.xlat_pip_loc(nloc, crd);
            res.push((net_f, pip));
        }
        res
    }

    pub fn consume_one_fwd(&mut self, net_id: NetId, nloc: NodeLoc) -> (NetId, PipNaming) {
        let list = self.consume_all_fwd(net_id, nloc);
        assert_eq!(list.len(), 1);
        list[0]
    }

    pub fn consume_one_bwd(&mut self, net_id: NetId, nloc: NodeLoc) -> (NetId, PipNaming) {
        let list = self.consume_all_bwd(net_id, nloc);
        assert_eq!(list.len(), 1);
        list[0]
    }

    fn grab_prim_id(&mut self, prim_id: PrimId) -> PrimExtractor<'a> {
        let prim = &self.die.prims[prim_id];
        let name = if prim.name_a.is_empty() {
            &prim.name_i
        } else {
            &prim.name_a
        };
        if self.used_prims[prim_id] {
            eprintln!("UMMM prim {name} double-used");
        }
        self.used_prims.set(prim_id, true);
        let mut pins: BTreeMap<&str, NetId> = BTreeMap::new();
        for (pin_id, pin_info) in &prim.pins {
            let pin_def = &self.die.primdefs[prim.primdef].pins[pin_id];
            pins.insert(
                &pin_def.name,
                self.get_net(pin_info.x, pin_info.y, pin_def.side).unwrap(),
            );
        }
        PrimExtractor { name, pins }
    }

    pub fn grab_prim_a(&mut self, name_a: &str) -> PrimExtractor<'a> {
        let prim_id = *self
            .prims_by_name_a
            .get(name_a)
            .unwrap_or_else(|| panic!("no bel {name_a}"));
        let prim = &self.die.prims[prim_id];
        assert_eq!(&prim.name_b, "");
        assert_eq!(&prim.name_i, "");
        self.grab_prim_id(prim_id)
    }

    pub fn grab_prim_ab(&mut self, name_a: &str, name_b: &str) -> PrimExtractor<'a> {
        let prim_id = *self
            .prims_by_name_a
            .get(name_a)
            .unwrap_or_else(|| panic!("no bel {name_a}"));
        let prim = &self.die.prims[prim_id];
        assert_eq!(&prim.name_b, name_b);
        assert_eq!(&prim.name_i, "");
        self.grab_prim_id(prim_id)
    }

    pub fn grab_prim_i(&mut self, name_i: &str) -> PrimExtractor<'a> {
        let prim_id = *self
            .prims_by_name_i
            .get(name_i)
            .unwrap_or_else(|| panic!("no bel {name_i}"));
        let prim = &self.die.prims[prim_id];
        assert_eq!(&prim.name_a, "");
        assert_eq!(&prim.name_b, "");
        self.grab_prim_id(prim_id)
    }

    pub fn bel_pip(
        &mut self,
        naming: NodeNamingId,
        bel: BelId,
        key: impl Into<String>,
        pip: PipNaming,
    ) {
        let key = key.into();
        match self.bel_pips[naming].entry((bel, key)) {
            btree_map::Entry::Vacant(entry) => {
                entry.insert(pip);
            }
            btree_map::Entry::Occupied(entry) => {
                assert_eq!(*entry.get(), pip);
            }
        }
    }

    pub fn own_box(&mut self, box_id: BoxId, nloc: NodeLoc) {
        assert!(!self.box_owner.contains_id(box_id));
        self.box_owner.insert(box_id, nloc);
    }

    pub fn own_mux(&mut self, wire: IntWire, nloc: NodeLoc) {
        let net = self.int_nets[&wire];
        for &net_f in self.nets[net].pips_bwd.keys() {
            if matches!(self.nets[net_f].binding, NetBinding::Int(_)) {
                assert_eq!(self.pip_owner.insert((net, net_f), nloc), None);
            }
        }
    }

    pub fn own_pip(&mut self, net_t: NetId, net_f: NetId, nloc: NodeLoc) {
        assert_eq!(self.pip_owner.insert((net_t, net_f), nloc), None);
    }

    pub fn mark_tbuf_pseudo(&mut self, net_t: NetId, net_f: NetId) {
        assert!(self.nets[net_t].pips_bwd.contains_key(&net_f));
        self.tbuf_pseudos.insert((net_t, net_f));
    }

    pub fn force_int_pip_dst(&mut self, net_t: NetId, net_f: NetId, nloc: NodeLoc, nw: NodeWireId) {
        self.pip_owner.insert((net_t, net_f), nloc);
        self.int_pip_force_dst.insert((net_t, net_f), nw);
    }

    fn extract_nodes(&mut self) {
        let mut node_boxes: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for (box_id, boxx) in &self.die.boxes {
            if let Some(&nloc) = self.box_owner.get(box_id) {
                node_boxes.entry(nloc).or_default().push(box_id);
            } else {
                eprintln!("box {name} not owned!", name = boxx.name);
            }
        }
        let die = self.egrid.die(DieId::from_idx(0));
        for col in die.cols() {
            for row in die.rows() {
                for (layer, _) in &die[(col, row)].nodes {
                    let nloc = (die.die, col, row, layer);
                    let nnode = &self.ngrid.nodes[&nloc];
                    if nnode.coords.is_empty() {
                        continue;
                    }
                    let rng = nnode.coords[NodeRawTileId::from_idx(0)].clone();

                    for x in rng.0 {
                        for y in rng.1.clone() {
                            let cv = usize::from(self.matrix[(x, y)] & 0xff);
                            for dir in [Dir::N, Dir::E, Dir::S, Dir::W] {
                                if (self.die.matrix_cells_fwd[dir][cv] & 0xf0) == 0 {
                                    continue;
                                }
                                let net_f = self.get_net(x, y, dir).unwrap();
                                for (i, odir) in
                                    [Dir::N, Dir::E, Dir::S, Dir::W].into_iter().enumerate()
                                {
                                    if (self.die.matrix_cells_fwd[dir][cv] >> i & 0x10) != 0 {
                                        let net_t = self.get_net(x, y, odir).unwrap();
                                        let key = (net_t, net_f);
                                        self.pip_owner.entry(key).or_insert(nloc);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        let mut node_pips: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for (&pip, &nloc) in &self.pip_owner {
            node_pips.entry(nloc).or_default().push(pip);
        }
        let mut net_by_tile: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
        for col in die.cols() {
            for row in die.rows() {
                for wire in self.egrid.db.wires.ids() {
                    let rw = self
                        .egrid
                        .resolve_wire_nobuf((die.die, (col, row), wire))
                        .unwrap();
                    if let Some(&net) = self.int_nets.get(&rw) {
                        match net_by_tile.entry((col, row)).or_default().entry(net) {
                            btree_map::Entry::Vacant(entry) => {
                                entry.insert(wire);
                            }
                            btree_map::Entry::Occupied(mut entry) => {
                                let p = entry.get_mut();
                                *p = min(*p, wire);
                            }
                        }
                    }
                }
                if let Some(nbto) = self.net_by_tile_override.get(&(col, row)) {
                    for (&net, &wire) in nbto {
                        net_by_tile.entry((col, row)).or_default().insert(net, wire);
                    }
                }
            }
        }
        for col in die.cols() {
            for row in die.rows() {
                for (layer, node) in &die[(col, row)].nodes {
                    let mut net_dict = BTreeMap::new();
                    for (tid, &(col, row)) in &node.tiles {
                        if let Some(nbt) = net_by_tile.get(&(col, row)) {
                            for (&net, &wire) in nbt {
                                net_dict.entry(net).or_insert((tid, wire));
                            }
                        }
                    }
                    let mut muxes = BTreeMap::new();
                    let mut int_pips = BTreeMap::new();
                    let nloc = (die.die, col, row, layer);
                    let nnode = &self.ngrid.nodes[&nloc];
                    if let Some(boxes) = node_boxes.get(&nloc) {
                        for &box_id in boxes {
                            let boxx = &self.die.boxes[box_id];
                            let boxdef = &self.die.boxdefs[boxx.boxdef];
                            for (i, pin) in boxdef.pins.iter().enumerate() {
                                for j in 0..boxdef.pins.len() {
                                    if pin.mask[j] {
                                        let nt = self.box_net(box_id, i);
                                        let nf = self.box_net(box_id, j);
                                        let Some(&nwt) = net_dict.get(&nt) else {
                                            continue;
                                        };
                                        let Some(&nwf) = net_dict.get(&nf) else {
                                            continue;
                                        };
                                        let (tx, ty, _) = self.die.box_pin(box_id, i);
                                        let (fx, fy, _) = self.die.box_pin(box_id, j);
                                        let pip_t = self.xlat_pip_loc(nloc, (tx, ty));
                                        let pip_f = self.xlat_pip_loc(nloc, (fx, fy));
                                        int_pips
                                            .insert((nwt, nwf), IntPipNaming::Box(pip_t, pip_f));
                                        muxes
                                            .entry(nwt)
                                            .or_insert(MuxInfo {
                                                kind: MuxKind::Plain,
                                                ins: Default::default(),
                                            })
                                            .ins
                                            .insert(nwf);
                                    }
                                }
                            }
                        }
                    }
                    if let Some(pips) = node_pips.get(&nloc) {
                        for &(nt, nf) in pips {
                            if self.used_pips.contains(&(nt, nf)) {
                                continue;
                            }
                            let nwt = if let Some(&n) = self.int_pip_force_dst.get(&(nt, nf)) {
                                n
                            } else if let Some(&n) = net_dict.get(&nt) {
                                n
                            } else {
                                continue;
                            };
                            let Some(&nwf) = net_dict.get(&nf) else {
                                continue;
                            };
                            let crd = self.nets[nt].pips_bwd[&nf];
                            let pip = self.xlat_pip_loc(nloc, crd);
                            int_pips.insert((nwt, nwf), IntPipNaming::Pip(pip));
                            self.use_pip(nt, nf);
                            let mut save = true;
                            if self.tbuf_pseudos.contains(&(nt, nf)) {
                                save = false;
                            }
                            if let WireKind::Buf(sw) = self.egrid.db.wires[nwt.1] {
                                assert_eq!(sw, nwf.1);
                                assert_eq!(nwt.0, nwf.0);
                                save = false;
                            }
                            if save {
                                muxes
                                    .entry(nwt)
                                    .or_insert(MuxInfo {
                                        kind: MuxKind::Plain,
                                        ins: Default::default(),
                                    })
                                    .ins
                                    .insert(nwf);
                            }
                        }
                    }
                    if !self.node_muxes.contains_id(node.kind) {
                        self.node_muxes.insert(node.kind, muxes);
                    } else {
                        assert_eq!(
                            self.node_muxes[node.kind],
                            muxes,
                            "fail merging node {}",
                            self.egrid.db.nodes.key(node.kind)
                        );
                    }
                    if !self.int_pips.contains_id(nnode.naming) {
                        self.int_pips.insert(nnode.naming, int_pips);
                    } else {
                        assert_eq!(
                            self.int_pips[nnode.naming],
                            int_pips,
                            "fail merging node naming {}",
                            self.ngrid.db.node_namings.key(nnode.naming)
                        );
                    }
                }
            }
        }
    }

    pub fn finish(mut self) -> Finisher {
        self.extract_nodes();
        for net in self.nets.values() {
            if net.binding == NetBinding::None {
                let (nx, ny, nd) = net.root;
                eprintln!("unknown net at {nx}.{ny}.{nd}");
            }
        }
        for (net_t, nd) in &self.nets {
            for (&net_f, &pip) in &nd.pips_bwd {
                if !self.used_pips.contains(&(net_t, net_f)) {
                    eprintln!("UNCLAIMED PIP at {pip:?}");
                }
            }
        }
        for (prim_id, prim) in &self.die.prims {
            if !self.used_prims[prim_id] {
                if self.junk_prim_names.contains(&prim.name_a) {
                    assert_eq!(prim.pins.len(), 0);
                    continue;
                }
                let pname = if prim.name_a.is_empty() {
                    &prim.name_i
                } else {
                    &prim.name_a
                };
                eprintln!("prim {pname} not used!");
            }
        }
        Finisher {
            bel_pips: self.bel_pips,
            int_pips: self.int_pips,
            node_muxes: self.node_muxes,
        }
    }
}

impl Finisher {
    pub fn finish(mut self, db: &mut IntDb, ndb: &mut NamingDb) {
        let mut new_node_namings = EntityMap::new();
        for (naming, name, mut node_naming) in core::mem::take(&mut ndb.node_namings) {
            if let Some(int_pips) = self.int_pips.remove(naming) {
                node_naming.int_pips = int_pips;
                node_naming.bel_pips = core::mem::take(&mut self.bel_pips[naming]);
                new_node_namings.insert(name, node_naming);
            }
        }
        ndb.node_namings = new_node_namings;
        let mut new_nodes = EntityMap::new();
        for (kind, name, mut node) in core::mem::take(&mut db.nodes) {
            if let Some(muxes) = self.node_muxes.remove(kind) {
                node.muxes = muxes;
                new_nodes.insert(name, node);
            }
        }
        db.nodes = new_nodes;
    }
}
