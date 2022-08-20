use std::collections::{HashMap, HashSet, BTreeMap, BTreeSet};

use prjcombine_entity::{EntityId, EntityVec, EntityPartVec};
use prjcombine_xilinx_geom::int;
use prjcombine_xilinx_rawdump::{Part, Coord, self as rawdump};

use assert_matches::assert_matches;

use enum_map::{EnumMap, enum_map};

#[derive(Clone, Debug)]
struct NodeType {
    tki: rawdump::TileKindId,
    naming: int::NodeNamingId,
}

pub struct IntBuilder<'a> {
    rd: &'a Part,
    pub db: int::IntDb,
    main_passes: EnumMap<int::Dir, EntityPartVec<int::WireId, int::WireId>>,
    node_types: Vec<NodeType>,
    stub_outs: HashSet<String>,
    extra_names: HashMap<String, int::NodeWireId>,
    extra_names_tile: HashMap<rawdump::TileKindId, HashMap<String, int::NodeWireId>>,
}

impl<'a> IntBuilder<'a> {
    pub fn new(name: &str, rd: &'a Part) -> Self {
        let db = int::IntDb {
            name: name.to_string(),
            wires: Default::default(),
            nodes: Default::default(),
            terms: Default::default(),
            intfs: Default::default(),
            node_namings: Default::default(),
            term_namings: Default::default(),
            intf_namings: Default::default(),
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

    pub fn make_term_naming(&mut self, name: impl AsRef<str>) -> int::TermNamingId {
        match self.db.term_namings.get(name.as_ref()) {
            None => self.db.term_namings.insert(name.as_ref().to_string(), Default::default()).0,
            Some((i, _)) => i,
        }
    }

    pub fn make_intf_naming(&mut self, name: impl AsRef<str>) -> int::IntfNamingId {
        match self.db.intf_namings.get(name.as_ref()) {
            None => self.db.intf_namings.insert(name.as_ref().to_string(), Default::default()).0,
            Some((i, _)) => i,
        }
    }

    pub fn name_term_in_near_wire(&mut self, naming: int::TermNamingId, wire: int::WireId, name: impl AsRef<str>) {
        let name = name.as_ref();
        let naming = &mut self.db.term_namings[naming];
        if !naming.wires_in_near.contains_id(wire) {
            naming.wires_in_near.insert(wire, name.to_string());
        } else {
            assert_eq!(naming.wires_in_near[wire], name);
        }
    }

    pub fn name_term_in_far_wire(&mut self, naming: int::TermNamingId, wire: int::WireId, name: impl AsRef<str>) {
        let name = name.as_ref();
        let naming = &mut self.db.term_namings[naming];
        if !naming.wires_in_far.contains_id(wire) {
            naming.wires_in_far.insert(wire, int::TermWireInFarNaming::Simple(name.to_string()));
        } else {
            assert_matches!(&naming.wires_in_far[wire], int::TermWireInFarNaming::Simple(n) if n == name);
        }
    }

    pub fn name_term_in_far_buf_wire(&mut self, naming: int::TermNamingId, wire: int::WireId, name_out: impl AsRef<str>, name_in: impl AsRef<str>) {
        let name_out = name_out.as_ref();
        let name_in = name_in.as_ref();
        let naming = &mut self.db.term_namings[naming];
        if !naming.wires_in_far.contains_id(wire) {
            naming.wires_in_far.insert(wire, int::TermWireInFarNaming::Buf(name_out.to_string(), name_in.to_string()));
        } else {
            assert_matches!(&naming.wires_in_far[wire], int::TermWireInFarNaming::Buf(no, ni) if no == name_out && ni == name_in);
        }
    }

    pub fn name_term_in_far_buf_far_wire(&mut self, naming: int::TermNamingId, wire: int::WireId, name: impl AsRef<str>, name_out: impl AsRef<str>, name_in: impl AsRef<str>) {
        let name = name.as_ref();
        let name_out = name_out.as_ref();
        let name_in = name_in.as_ref();
        let naming = &mut self.db.term_namings[naming];
        if !naming.wires_in_far.contains_id(wire) {
            naming.wires_in_far.insert(wire, int::TermWireInFarNaming::BufFar(name.to_string(), name_out.to_string(), name_in.to_string()));
        } else {
            assert_matches!(&naming.wires_in_far[wire], int::TermWireInFarNaming::BufFar(n, no, ni) if n == name && no == name_out && ni == name_in);
        }
    }

    pub fn name_term_out_wire(&mut self, naming: int::TermNamingId, wire: int::WireId, name: impl AsRef<str>) {
        let name = name.as_ref();
        let naming = &mut self.db.term_namings[naming];
        if !naming.wires_out.contains_id(wire) {
            naming.wires_out.insert(wire, int::TermWireOutNaming::Simple(name.to_string()));
        } else {
            assert_matches!(&naming.wires_out[wire], int::TermWireOutNaming::Simple(n) if n == name);
        }
    }

    pub fn name_term_out_buf_wire(&mut self, naming: int::TermNamingId, wire: int::WireId, name_out: impl AsRef<str>, name_in: impl AsRef<str>) {
        let name_out = name_out.as_ref();
        let name_in = name_in.as_ref();
        let naming = &mut self.db.term_namings[naming];
        if !naming.wires_out.contains_id(wire) {
            naming.wires_out.insert(wire, int::TermWireOutNaming::Buf(name_out.to_string(), name_in.to_string()));
        } else {
            assert_matches!(&naming.wires_out[wire], int::TermWireOutNaming::Buf(no, ni) if no == name_out && ni == name_in);
        }
    }

    pub fn name_intf_in_wire(&mut self, naming: int::IntfNamingId, wire: int::WireId, val: int::IntfWireInNaming) {
        let naming = &mut self.db.intf_namings[naming];
        if !naming.wires_in.contains_id(wire) {
            naming.wires_in.insert(wire, val);
        } else {
            assert_eq!(naming.wires_in[wire], val);
        }
    }

    pub fn name_intf_out_wire(&mut self, naming: int::IntfNamingId, wire: int::WireId, name: impl AsRef<str>) {
        let name = name.as_ref();
        let naming = &mut self.db.intf_namings[naming];
        if !naming.wires_out.contains_id(wire) {
            naming.wires_out.insert(wire, int::IntfWireOutNaming::Simple(name.to_string()));
        } else {
            assert_matches!(&naming.wires_out[wire], int::IntfWireOutNaming::Simple(n) | int::IntfWireOutNaming::Buf(n, _) if n == name);
        }
    }

    pub fn name_intf_out_wire_in(&mut self, naming: int::IntfNamingId, wire: int::WireId, name: impl AsRef<str>) {
        let name = name.as_ref();
        let naming = &mut self.db.intf_namings[naming];
        match naming.wires_out[wire] {
            int::IntfWireOutNaming::Simple(ref n) => {
                let n = n.clone();
                naming.wires_out[wire] = int::IntfWireOutNaming::Buf(n, name.to_string());
            }
            int::IntfWireOutNaming::Buf(_, ref n) => assert_eq!(n, name),
        }
    }

    pub fn find_wire(&mut self, name: impl AsRef<str>) -> int::WireId {
        for (k, v) in &self.db.wires {
            if v.name == name.as_ref() {
                return k;
            }
        }
        unreachable!();
    }

    pub fn wire(&mut self, name: impl Into<String>, kind: int::WireKind, raw_names: &[impl AsRef<str>]) -> int::WireId {
        let res = self.db.wires.push(int::WireInfo {
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

    pub fn mux_out(&mut self, name: impl Into<String>, raw_names: &[impl AsRef<str>]) -> int::WireId {
        self.wire(name, int::WireKind::MuxOut, raw_names)
    }

    pub fn logic_out(&mut self, name: impl Into<String>, raw_names: &[impl AsRef<str>]) -> int::WireId {
        self.wire(name, int::WireKind::LogicOut, raw_names)
    }

    pub fn multi_out(&mut self, name: impl Into<String>, raw_names: &[impl AsRef<str>]) -> int::WireId {
        self.wire(name, int::WireKind::MultiOut, raw_names)
    }

    pub fn test_out(&mut self, name: impl Into<String>, raw_names: &[impl AsRef<str>]) -> int::WireId {
        self.wire(name, int::WireKind::TestOut, raw_names)
    }

    pub fn buf(&mut self, src: int::WireId, name: impl Into<String>, raw_names: &[impl AsRef<str>]) -> int::WireId {
        self.wire(name, int::WireKind::Buf(src), raw_names)
    }

    pub fn conn_branch(&mut self, src: int::WireId, dir: int::Dir, dst: int::WireId) {
        self.main_passes[!dir].insert(dst, src);
    }

    pub fn branch(&mut self, src: int::WireId, dir: int::Dir, name: impl Into<String>, raw_names: &[impl AsRef<str>]) -> int::WireId {
        let res = self.wire(name, int::WireKind::Branch(!dir), raw_names);
        self.conn_branch(src, dir, res);
        res
    }

    pub fn multi_branch(&mut self, src: int::WireId, dir: int::Dir, name: impl Into<String>, raw_names: &[impl AsRef<str>]) -> int::WireId {
        let res = self.wire(name, int::WireKind::MultiBranch(!dir), raw_names);
        self.conn_branch(src, dir, res);
        res
    }

    pub fn pip_branch(&mut self, src: int::WireId, dir: int::Dir, name: impl Into<String>, raw_names: &[impl AsRef<str>]) -> int::WireId {
        let res = self.wire(name, int::WireKind::PipBranch(!dir), raw_names);
        self.conn_branch(src, dir, res);
        res
    }

    pub fn stub_out(&mut self, name: impl Into<String>) {
        self.stub_outs.insert(name.into());
    }

    pub fn extra_name(&mut self, name: impl Into<String>, wire: int::WireId) {
        self.extra_names.insert(name.into(), (int::NodeTileId::from_idx(0), wire));
    }

    pub fn extra_name_sub(&mut self, name: impl Into<String>, sub: usize, wire: int::WireId) {
        self.extra_names.insert(name.into(), (int::NodeTileId::from_idx(sub), wire));
    }

    pub fn extra_name_tile(&mut self, tile: impl AsRef<str>, name: impl Into<String>, wire: int::WireId) {
        if let Some((tki, _)) = self.rd.tile_kinds.get(tile.as_ref()) {
            self.extra_names_tile.entry(tki).or_default().insert(name.into(), (int::NodeTileId::from_idx(0), wire));
        }
    }

    pub fn extra_name_tile_sub(&mut self, tile: impl AsRef<str>, name: impl Into<String>, sub: usize, wire: int::WireId) {
        if let Some((tki, _)) = self.rd.tile_kinds.get(tile.as_ref()) {
            self.extra_names_tile.entry(tki).or_default().insert(name.into(), (int::NodeTileId::from_idx(sub), wire));
        }
    }

    pub fn extract_main_passes(&mut self) {
        for (dir, wires) in &self.main_passes {
            self.db.terms.insert(format!("MAIN.{dir}"), int::TermKind {
                dir,
                wires: wires.iter().map(|(k, &v)| (k, int::TermInfo::PassFar(v))).collect()
            });
        }
    }

    pub fn node_type(&mut self, tile_kind: &str, kind: &str, naming: &str) {
        if let Some((tki, _)) = self.rd.tile_kinds.get(tile_kind) {
            let tk = &self.rd.tile_kinds[tki];
            let tkn = self.rd.tile_kinds.key(tki);
            let mut node = int::NodeKind {
                tiles: [()].into_iter().collect(),
                muxes: Default::default(),
                bels: Default::default(),
            };
            let mut node_naming = int::NodeNaming::default();
            let mut names = HashMap::new();
            for &wi in tk.wires.keys() {
                if let Some(&w) = self.extra_names.get(&self.rd.wires[wi]) {
                    names.insert(wi, w);
                    continue;
                }
                if let Some(xn) = self.extra_names_tile.get(&tki) {
                    if let Some(&w) = xn.get(&self.rd.wires[wi]) {
                        names.insert(wi, w);
                        continue;
                    }
                }
            }

            for (&k, &v) in &names {
                node_naming.wires.insert(v, self.rd.wires[k].clone());
            }

            for &(wfi, wti) in tk.pips.keys() {
                if let Some(&wt) = names.get(&wti) {
                    match self.db.wires[wt.1].kind {
                        int::WireKind::PipBranch(_) |
                        int::WireKind::PipOut |
                        int::WireKind::MultiBranch(_) |
                        int::WireKind::MultiOut |
                        int::WireKind::MuxOut => (),
                        int::WireKind::Branch(_) => {
                            if self.db.name != "virtex" {
                                continue;
                            }
                        }
                        int::WireKind::Buf(dwf) => {
                            let wf = names[&wfi];
                            assert_eq!(wf, (wt.0, dwf));
                            continue;
                        }
                        _ => continue,
                    }
                    if let Some(&wf) = names.get(&wfi) {
                        // XXX
                        let kind = int::MuxKind::Plain;
                        if !node.muxes.contains_key(&wt) {
                            node.muxes.insert(wt, int::MuxInfo {
                                kind,
                                ins: Default::default(),
                            });
                        }
                        let mux = node.muxes.get_mut(&wt).unwrap();
                        assert_eq!(mux.kind, kind);
                        mux.ins.insert(wf);
                    } else if self.stub_outs.contains(&self.rd.wires[wfi]) {
                        // ignore
                    } else {
                        println!("UNEXPECTED INT MUX IN {} {} {}", tkn, self.rd.wires[wti], self.rd.wires[wfi]);
                    }
                }
            }
            self.insert_node_merge(kind, node);
            let naming = self.insert_node_naming(naming, node_naming);
            self.node_types.push(NodeType {
                tki,
                naming,
            });
        }
    }

    fn get_int_naming(&self, int_xy: Coord) -> Option<int::NodeNamingId> {
        let int_tile = &self.rd.tiles[&int_xy];
        self.node_types.iter().find_map(|nt| if nt.tki == int_tile.kind { Some(nt.naming) } else { None })
    }

    fn get_int_rev_naming(&self, int_xy: Coord) -> HashMap<String, int::WireId> {
        if let Some(int_naming_id) = self.get_int_naming(int_xy) {
            let int_naming = &self.db.node_namings[int_naming_id];
            int_naming.wires.iter().filter_map(|(k, v)| if k.0.to_idx() == 0 {Some((v.to_string(), k.1))} else {None}).collect()
        } else {
            Default::default()
        }
    }

    fn get_node(&self, tile: &rawdump::Tile, tk: &rawdump::TileKind, wi: rawdump::WireId) -> Option<rawdump::NodeId> {
        if let Some((_, &rawdump::TkWire::Connected(idx))) = tk.wires.get(&wi) {
            if let Some(&nidx) = tile.conn_wires.get(idx) {
                return Some(nidx);
            }
        }
        None
    }

    fn get_int_node2wires(&self, int_xy: Coord) -> HashMap<rawdump::NodeId, Vec<int::WireId>> {
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

    pub fn recover_names(&self, tile_xy: Coord, int_xy: Coord) -> HashMap<rawdump::WireId, Vec<int::WireId>> {
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
                } else if let Some(&w) = self.extra_names_tile.get(&tile.kind).and_then(|x| x.get(&self.rd.wires[wi])) {
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

    pub fn recover_names_cands(&self, tile_xy: Coord, int_xy: Coord, cands: &HashSet<int::WireId>) -> HashMap<rawdump::WireId, int::WireId> {
        self.recover_names(tile_xy, int_xy).into_iter().filter_map(|(k, v)| {
            let nv: Vec<_> = v.into_iter().filter(|x| cands.contains(x)).collect();
            if nv.len() == 1 {
                Some((k, nv[0]))
            } else {
                None
            }
        }).collect()
    }

    fn insert_node_merge(&mut self, name: &str, node: int::NodeKind) -> int::NodeKindId {
        match self.db.nodes.get_mut(name) {
            None => {
                self.db.nodes.insert(name.to_string(), node).0
            }
            Some((id, cnode)) => {
                assert_eq!(node.tiles, cnode.tiles);
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
                id
            }
        }
    }

    fn insert_node_naming(&mut self, name: &str, naming: int::NodeNaming) -> int::NodeNamingId {
        match self.db.node_namings.get_mut(name) {
            None => {
                self.db.node_namings.insert(name.to_string(), naming).0
            }
            Some((id, cnaming)) => {
                assert_eq!(naming.ext_pips, cnaming.ext_pips);
                assert_eq!(naming.wire_bufs, cnaming.wire_bufs);
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
                id
            }
        }
    }

    pub fn insert_term_merge(&mut self, name: &str, term: int::TermKind) {
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
                        },
                        (a, b) => assert_eq!(a.map(|x| &*x), b),
                    }
                }
            }
        }
    }

    fn get_pass_inps(&self, dir: int::Dir) -> HashSet<int::WireId> {
        self.main_passes[dir].values().copied().collect()
    }

    fn extract_term_tile_conn(&self, dir: int::Dir, int_xy: Coord, forced: &HashMap<int::WireId, int::WireId>) -> EntityPartVec<int::WireId, int::TermInfo> {
        let mut res = EntityPartVec::new();
        let n2w = self.get_int_node2wires(int_xy);
        let cand_inps = self.get_pass_inps(!dir);
        for wl in n2w.into_values() {
            for &wt in &wl {
                if !self.main_passes[dir].contains_id(wt) {
                    continue;
                }
                let wf: Vec<_> = wl.iter().copied().filter(|&wf| wf != wt && cand_inps.contains(&wf)).collect();
                if let Some(&fwf) = forced.get(&wt) {
                    if wf.contains(&fwf) {
                        res.insert(wt, int::TermInfo::PassNear(fwf));
                    }
                } else {
                    if wf.len() == 1 {
                        res.insert(wt, int::TermInfo::PassNear(wf[0]));
                    }
                    if wf.len() > 1 {
                        println!("WHOOPS MULTI {} {:?}", self.db.wires[wt].name, wf.iter().map(|&x| &self.db.wires[x].name).collect::<Vec<_>>());
                    }
                }
            }
        }
        res
    }

    pub fn extract_term_tile(&mut self, name: impl AsRef<str>, node_name: Option<&str>, dir: int::Dir, term_xy: Coord, naming: impl AsRef<str>, int_xy: Coord) {
        let cand_inps = self.get_pass_inps(!dir);
        let names = self.recover_names(term_xy, int_xy);
        let tile = &self.rd.tiles[&term_xy];
        let tk = &self.rd.tile_kinds[tile.kind];
        let tkn = self.rd.tile_kinds.key(tile.kind);
        let mut muxes: HashMap<int::WireId, Vec<int::WireId>> = HashMap::new();
        let naming_id = self.make_term_naming(naming.as_ref());
        let mut tnames = EntityPartVec::new();
        for &(wfi, wti) in tk.pips.keys() {
            if let Some(wtl) = names.get(&wti) {
                for &wt in wtl {
                    match self.db.wires[wt].kind {
                        int::WireKind::Branch(d) => if d != dir {
                            continue;
                        },
                        _ => continue,
                    }
                    if let Some(wfl) = names.get(&wfi) {
                        let wf;
                        if wfl.len() == 1 {
                            wf = wfl[0];
                        } else {
                            let nwfl: Vec<_> = wfl.iter().copied().filter(|x| cand_inps.contains(x)).collect();
                            if nwfl.len() == 1 {
                                wf = nwfl[0];
                            } else {
                                println!("AMBIG TERM MUX IN {} {} {}", tkn, self.rd.wires[wti], self.rd.wires[wfi]);
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
                        println!("UNEXPECTED TERM MUX IN {} {} {}", tkn, self.rd.wires[wti], self.rd.wires[wfi]);
                    }
                }
            }
        }
        let mut node_muxes = BTreeMap::new();
        let mut node_names = BTreeMap::new();
        let mut wires = self.extract_term_tile_conn(dir, int_xy, &Default::default());
        for (k, v) in muxes {
            if v.len() == 1 {
                self.name_term_out_wire(naming_id, k, tnames[k].to_string());
                self.name_term_in_near_wire(naming_id, v[0], tnames[v[0]].to_string());
                wires.insert(k, int::TermInfo::PassNear(v[0]));
            } else {
                let def_t = int::NodeTileId::from_idx(0);
                node_names.insert((def_t, k), tnames[k].to_string());
                for &x in &v {
                    node_names.insert((def_t, x), tnames[x].to_string());
                }
                node_muxes.insert((def_t, k), int::MuxInfo {
                    kind: int::MuxKind::Plain,
                    ins: v.into_iter().map(|x| (def_t, x)).collect()
                });
            }
        }
        if let Some(nn) = node_name {
            self.insert_node_merge(nn, int::NodeKind {
                tiles: [()].into_iter().collect(),
                muxes: node_muxes,
                bels: Default::default(),
            });
            self.insert_node_naming(naming.as_ref(), int::NodeNaming {
                wires: node_names,
                wire_bufs: Default::default(),
                ext_pips: Default::default(),
                bels: Default::default(),
            });
        } else {
            assert!(node_muxes.is_empty());
        }
        let term = int::TermKind {
            dir,
            wires,
        };
        self.insert_term_merge(name.as_ref(), term);
    }

    pub fn extract_term_buf_tile(&mut self, name: impl AsRef<str>, dir: int::Dir, term_xy: Coord, naming: impl AsRef<str>, int_xy: Coord, forced: &[(int::WireId, int::WireId)]) {
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
                        int::WireKind::Branch(d) => if d != dir {
                            continue;
                        },
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
                                let nwfl: Vec<_> = wfl.iter().copied().filter(|x| cand_inps.contains(x)).collect();
                                if nwfl.len() == 1 {
                                    wf = nwfl[0];
                                } else {
                                    println!("AMBIG TERM MUX IN {} {} {}", tkn, self.rd.wires[wti], self.rd.wires[wfi]);
                                    continue;
                                }
                            }
                        }
                        self.name_term_out_buf_wire(naming_id, wt, &self.rd.wires[wti], &self.rd.wires[wfi]);
                        if wires.contains_id(wt) {
                            println!("OOPS DUPLICATE TERM BUF {} {}", tkn, self.rd.wires[wti]);
                        }
                        assert!(!wires.contains_id(wt));
                        wires.insert(wt, int::TermInfo::PassNear(wf));
                    } else {
                        println!("UNEXPECTED TERM BUF IN {} {} {}", tkn, self.rd.wires[wti], self.rd.wires[wfi]);
                    }
                }
            }
        }
        let term = int::TermKind {
            dir,
            wires,
        };
        self.insert_term_merge(name.as_ref(), term);
    }

    pub fn extract_term_conn_tile(&mut self, name: impl AsRef<str>, dir: int::Dir, int_xy: Coord, forced: &[(int::WireId, int::WireId)]) {
        let forced: HashMap<_, _> = forced.iter().copied().collect();
        let wires = self.extract_term_tile_conn(dir, int_xy, &forced);
        let term = int::TermKind {
            dir,
            wires,
        };
        self.insert_term_merge(name.as_ref(), term);
    }

    pub fn walk_to_int(&self, mut xy: Coord, dir: int::Dir) -> Option<Coord> {
        loop {
            let tile = &self.rd.tiles[&xy];
            if self.node_types.iter().any(|x| x.tki == tile.kind) {
                return Some(xy);
            }
            match dir {
                int::Dir::W => {
                    if xy.x == 0 {
                        return None;
                    }
                    xy.x -= 1;
                },
                int::Dir::E => {
                    if xy.x == self.rd.width - 1 {
                        return None;
                    }
                    xy.x += 1;
                }
                int::Dir::S => {
                    if xy.y == 0 {
                        return None;
                    }
                    xy.y -= 1;
                },
                int::Dir::N => {
                    if xy.y == self.rd.height - 1 {
                        return None;
                    }
                    xy.y += 1;
                }
            }
        }
    }

    pub fn extract_term(&mut self, name: impl AsRef<str>, node_name: Option<&str>, dir: int::Dir, tkn: impl AsRef<str>, naming: impl AsRef<str>) {
        for &term_xy in self.rd.tiles_by_kind_name(tkn.as_ref()) {
            if let Some(int_xy) = self.walk_to_int(term_xy, !dir) {
                self.extract_term_tile(name.as_ref(), node_name, dir, term_xy, naming.as_ref(), int_xy);
            }
        }
    }

    pub fn extract_term_buf(&mut self, name: impl AsRef<str>, dir: int::Dir, tkn: impl AsRef<str>, naming: impl AsRef<str>, forced: &[(int::WireId, int::WireId)]) {
        for &term_xy in self.rd.tiles_by_kind_name(tkn.as_ref()) {
            if let Some(int_xy) = self.walk_to_int(term_xy, !dir) {
                self.extract_term_buf_tile(name.as_ref(), dir, term_xy, naming.as_ref(), int_xy, forced);
            }
        }
    }

    pub fn extract_term_conn(&mut self, name: impl AsRef<str>, dir: int::Dir, tkn: impl AsRef<str>, forced: &[(int::WireId, int::WireId)]) {
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
        mux_ins.into_iter().filter_map(|(k, v)| if v.len() == 1 {Some((k, v[0]))} else {None}).collect()
    }

    pub fn extract_pass_tile(&mut self, name: impl AsRef<str>, dir: int::Dir, int_xy: Coord, near: Option<Coord>, far: Option<Coord>, naming: Option<&str>, node: Option<(&str, &str)>, src_xy: Coord, force_pass: &[int::WireId]) {
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
                wires.insert(wn, int::TermInfo::PassFar(wf));
            }
        }
        for wn in self.main_passes[dir].ids() {
            if let Some(wnn) = int_naming.wires.get(&(int::NodeTileId::from_idx(0), wn)) {
                let wni = self.rd.wires.get(wnn).unwrap();
                if let Some(nidx) = self.get_node(int_tile, int_tk, wni) {
                    if let Some(w) = src_node2wires.get(&nidx) {
                        let w: Vec<_> = w.iter().copied().filter(|x| cand_inps_far.contains(x)).collect();
                        if w.len() == 1 {
                            wires.insert(wn, int::TermInfo::PassFar(w[0]));
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
                Near(int::WireId),
                Far(int::WireId),
            }
            #[derive(Clone, Copy)]
            enum FarNaming<'b> {
                Simple(&'b str),
                BufNear(&'b str, &'b str),
                BufFar(&'b str, &'b str, &'b str),
            }
            let mut muxes: HashMap<int::WireId, Vec<WireIn>> = HashMap::new();
            let mut names_out = EntityPartVec::new();
            let mut names_in_near = EntityPartVec::new();
            let mut names_in_far = EntityPartVec::new();
            for &(wfi, wti) in tk.pips.keys() {
                if let Some(wtl) = names.get(&wti) {
                    for &wt in wtl {
                        match self.db.wires[wt].kind {
                            int::WireKind::Branch(d) => if d != dir {
                                continue;
                            },
                            _ => continue,
                        }
                        if wires.contains_id(wt) {
                            continue;
                        }
                        names_out.insert(wt, &self.rd.wires[wti]);
                        if let Some(wfl) = names.get(&wfi) {
                            if wfl.len() != 1 {
                                println!("AMBIG PASS MUX IN {} {} {}", tkn, self.rd.wires[wti], self.rd.wires[wfi]);
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
                                names_in_far.insert(wf, FarNaming::BufNear(&self.rd.wires[woi], &self.rd.wires[wii]));
                            } else {
                                names_in_far.insert(wf, FarNaming::BufFar(&self.rd.wires[wfi], &self.rd.wires[woi], &self.rd.wires[wii]));
                            }
                            muxes.entry(wt).or_default().push(WireIn::Far(wf));
                        } else if self.stub_outs.contains(&self.rd.wires[wfi]) {
                            // ignore
                        } else {
                            println!("UNEXPECTED PASS MUX IN {} {} {}", tkn, self.rd.wires[wti], self.rd.wires[wfi]);
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
                    self.name_term_out_wire(naming.unwrap(), wt, names_out[wt].to_string());
                    match v[0] {
                        WireIn::Near(wf) => {
                            self.name_term_in_near_wire(naming.unwrap(), wf, names_in_near[wf].to_string());
                            wires.insert(wt, int::TermInfo::PassNear(wf));
                        }
                        WireIn::Far(wf) => {
                            match names_in_far[wf] {
                                FarNaming::Simple(n) =>
                                    self.name_term_in_far_wire(naming.unwrap(), wf, n.to_string()),
                                FarNaming::BufNear(no, ni) =>
                                    self.name_term_in_far_buf_wire(naming.unwrap(), wf, no.to_string(), ni.to_string()),
                                FarNaming::BufFar(n, no, ni) =>
                                    self.name_term_in_far_buf_far_wire(naming.unwrap(), wf, n.to_string(), no.to_string(), ni.to_string()),
                            }
                            wires.insert(wt, int::TermInfo::PassFar(wf));
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
                                        node_wire_bufs.insert((nt_far, wf), int::NodeExtPipNaming {
                                            tile: int::NodeRawTileId::from_idx(0),
                                            wire_to: no.to_string(),
                                            wire_from: ni.to_string(),
                                        });
                                    }
                                    FarNaming::BufFar(n, no, ni) => {
                                        node_names.insert((nt_far, wf), n.to_string());
                                        node_wire_bufs.insert((nt_far, wf), int::NodeExtPipNaming {
                                            tile: int::NodeRawTileId::from_idx(1),
                                            wire_to: no.to_string(),
                                            wire_from: ni.to_string(),
                                        });
                                    }
                                }
                                ins.insert((nt_far, wf));
                            }
                        }
                    }
                    node_muxes.insert((nt_near, wt), int::MuxInfo {
                        kind: int::MuxKind::Plain,
                        ins,
                    });
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
                        if self.db.wires[wt].kind != int::WireKind::MultiBranch(dir) {
                            continue;
                        }
                        let wf = self.main_passes[dir][wt];
                        assert!(!wires.contains_id(wt));
                        if names_far.get(&wfi) != Some(&wf) {
                            println!("WEIRD SPLITTER {} {} {}", tkn, self.rd.wires[wti], self.rd.wires[wfi]);
                        } else {
                            node_names.insert((nt_near, wt), self.rd.wires[wti].clone());
                            node_names.insert((nt_far, wf), self.rd.wires[wfi].clone());
                            node_muxes.insert((nt_near, wt), int::MuxInfo {
                                kind: int::MuxKind::Plain,
                                ins: [(nt_far, wf)].into_iter().collect(),
                            });
                            node_muxes.insert((nt_far, wf), int::MuxInfo {
                                kind: int::MuxKind::Plain,
                                ins: [(nt_near, wt)].into_iter().collect(),
                            });
                        }
                    }
                }
            }
            if let Some((nn, nnn)) = node {
                self.insert_node_merge(nn, int::NodeKind {
                    tiles: node_tiles,
                    muxes: node_muxes,
                    bels: Default::default(),
                });
                self.insert_node_naming(nnn, int::NodeNaming {
                    wires: node_names,
                    wire_bufs: node_wire_bufs,
                    ext_pips: Default::default(),
                    bels: Default::default(),
                });
            } else {
                assert!(node_muxes.is_empty());
            }
        }

        let term = int::TermKind {
            dir,
            wires,
        };
        self.insert_term_merge(name.as_ref(), term);
    }

    pub fn extract_pass_simple(&mut self, name: impl AsRef<str>, dir: int::Dir, tkn: impl AsRef<str>, force_pass: &[int::WireId]) {
        for &xy in self.rd.tiles_by_kind_name(tkn.as_ref()) {
            if let Some(int_fwd_xy) = self.walk_to_int(xy, dir) {
                if let Some(int_bwd_xy) = self.walk_to_int(xy, !dir) {
                    self.extract_pass_tile(format!("{}.{}", name.as_ref(), dir), dir, int_bwd_xy, None, None, None, None, int_fwd_xy, force_pass);
                    self.extract_pass_tile(format!("{}.{}", name.as_ref(), !dir), !dir, int_fwd_xy, None, None, None, None, int_bwd_xy, force_pass);
                }
            }
        }
    }

    pub fn extract_pass_buf(&mut self, name: impl AsRef<str>, dir: int::Dir, tkn: impl AsRef<str>, naming: impl AsRef<str>, force_pass: &[int::WireId]) {
        for &xy in self.rd.tiles_by_kind_name(tkn.as_ref()) {
            if let Some(int_fwd_xy) = self.walk_to_int(xy, dir) {
                if let Some(int_bwd_xy) = self.walk_to_int(xy, !dir) {
                    let naming_fwd = format!("{}.{}", naming.as_ref(), dir);
                    let naming_bwd = format!("{}.{}", naming.as_ref(), !dir);
                    self.extract_pass_tile(format!("{}.{}", name.as_ref(), dir), dir, int_bwd_xy, Some(xy), None, Some(&naming_bwd), None, int_fwd_xy, force_pass);
                    self.extract_pass_tile(format!("{}.{}", name.as_ref(), !dir), !dir, int_fwd_xy, Some(xy), None, Some(&naming_fwd), None, int_bwd_xy, force_pass);
                }
            }
        }
    }

    pub fn make_blackhole_term(&mut self, name: impl AsRef<str>, dir: int::Dir, wires: &[int::WireId]) {
        for &w in wires {
            assert!(self.main_passes[dir].contains_id(w));
        }
        let term = int::TermKind {
            dir,
            wires: wires.iter().map(|&w| (w, int::TermInfo::BlackHole)).collect(),
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

    pub fn extract_intf_tile(&mut self, name: impl AsRef<str>, xy: Coord, int_xy: Coord, naming: impl AsRef<str>, has_out_bufs: bool) {
        let names = self.recover_names(xy, int_xy);
        let tile = &self.rd.tiles[&xy];
        let tk = &self.rd.tile_kinds[tile.kind];
        let tkn = self.rd.tile_kinds.key(tile.kind);
        let naming = self.make_intf_naming(naming);
        let mut out_muxes: HashMap<int::WireId, Vec<int::WireId>> = HashMap::new();
        let bufs = self.get_bufs(tk);
        let mut wires = EntityPartVec::new();
        let mut delayed = HashMap::new();
        if has_out_bufs {
            for (&wdi, &wfi) in &bufs {
                if let Some(wfl) = names.get(&wfi) {
                    for &wf in wfl {
                        if !matches!(self.db.wires[wf].kind, int::WireKind::MuxOut) {
                            continue;
                        }
                        for &wti in tk.wires.keys() {
                            if tk.pips.contains_key(&(wfi, wti)) && tk.pips.contains_key(&(wdi, wti)) {
                                self.name_intf_in_wire(naming, wf, int::IntfWireInNaming::Delay(self.rd.wires[wti].clone(), self.rd.wires[wdi].clone(), self.rd.wires[wfi].clone()));
                                delayed.insert(wti, wf);
                                wires.insert(wf, int::IntfInfo::InputDelay);
                            }
                        }
                    }
                }
            }
        }
        for &(wfi, wti) in tk.pips.keys() {
            if let Some(wtl) = names.get(&wti) {
                for &wt in wtl {
                    if !matches!(self.db.wires[wt].kind, int::WireKind::LogicOut) {
                        continue;
                    }
                    self.name_intf_out_wire(naming, wt, &self.rd.wires[wti]);
                    let mut rwfi = wfi;
                    if bufs.contains_key(&wfi) {
                        rwfi = bufs[&wfi];
                    }
                    if let Some(wfl) = names.get(&rwfi) {
                        let wf;
                        if wfl.len() == 1 {
                            wf = wfl[0];
                        } else {
                            let nwfl: Vec<_> = wfl.iter().copied().filter(|&x| matches!(self.db.wires[x].kind, int::WireKind::MuxOut)).collect();
                            if nwfl.len() == 1 {
                                wf = nwfl[0];
                            } else {
                                println!("AMBIG INTF MUX IN {} {} {}", tkn, self.rd.wires[wti], self.rd.wires[wfi]);
                                continue;
                            }
                        }
                        if rwfi != wfi {
                            self.name_intf_in_wire(naming, wf, int::IntfWireInNaming::TestBuf(self.rd.wires[wfi].clone(), self.rd.wires[rwfi].clone()));
                        } else {
                            self.name_intf_in_wire(naming, wf, int::IntfWireInNaming::Simple(self.rd.wires[wfi].clone()));
                        }
                        assert!(!wires.contains_id(wf));
                        out_muxes.entry(wt).or_default().push(wf);
                    } else if let Some(&wf) = delayed.get(&wfi) {
                        out_muxes.entry(wt).or_default().push(wf);
                    } else if has_out_bufs {
                        out_muxes.entry(wt).or_default();
                        self.name_intf_out_wire_in(naming, wt, &self.rd.wires[wfi]);
                    }
                }
            }
        }
        for (k, v) in out_muxes {
            wires.insert(k, int::IntfInfo::OutputTestMux(v.into_iter().collect()));
        }
        let intf = int::IntfKind {
            wires,
        };
        match self.db.intfs.get_mut(name.as_ref()) {
            None => {
                self.db.intfs.insert(name.as_ref().to_string(), intf);
            }
            Some((_, cintf)) => {
                assert_eq!(intf, *cintf);
            }
        };
    }

    pub fn extract_intf(&mut self, name: impl AsRef<str>, dir: int::Dir, tkn: impl AsRef<str>, naming: impl AsRef<str>, has_out_bufs: bool) {
        for &xy in self.rd.tiles_by_kind_name(tkn.as_ref()) {
            let int_xy = self.walk_to_int(xy, !dir).unwrap();
            self.extract_intf_tile(name.as_ref(), xy, int_xy, naming.as_ref(), has_out_bufs);
        }
    }

    pub fn extract_xnode(&mut self, name: &str, xy: Coord, int_xy: &[Coord], naming: &str, skip_wires: &[int::WireId]) {
        let tile = &self.rd.tiles[&xy];
        let tk = &self.rd.tile_kinds[tile.kind];
        let mut names = HashMap::new();
        let node2wires: EntityVec<int::NodeTileId, _> = int_xy.into_iter().copied().map(|x| self.get_int_node2wires(x)).collect();
        for (_, &wi, &tkw) in tk.wires.iter() {
            if let Some(&w) = self.extra_names.get(&self.rd.wires[wi]) {
                names.insert(wi, w);
                continue;
            }
            if let Some(xn) = self.extra_names_tile.get(&tile.kind) {
                if let Some(&w) = xn.get(&self.rd.wires[wi]) {
                    names.insert(wi, w);
                    continue;
                }
            }
            if let rawdump::TkWire::Connected(idx) = tkw {
                if let Some(&nidx) = tile.conn_wires.get(idx) {
                    for (k, v) in &node2wires {
                        if let Some(w) = v.get(&nidx) {
                            if w.len() == 1 {
                                names.insert(wi, (k, w[0]));
                                break;
                            }
                        }
                    }
                }
            }
        }

        let mut node = int::NodeKind {
            tiles: node2wires.map_values(|_| ()),
            muxes: Default::default(),
            bels: Default::default(),
        };
        let mut node_naming = int::NodeNaming::default();

        for &(wfi, wti) in tk.pips.keys() {
            if let Some(&wt) = names.get(&wti) {
                if skip_wires.contains(&wt.1) {
                    continue;
                }
                if matches!(self.db.wires[wt.1].kind, int::WireKind::LogicOut) {
                    continue;
                }
                if let Some(&wf) = names.get(&wfi) {
                    node_naming.wires.insert(wt, self.rd.wires[wti].to_string());
                    node_naming.wires.insert(wf, self.rd.wires[wfi].to_string());
                    // XXX
                    let kind = int::MuxKind::Plain;
                    if !node.muxes.contains_key(&wt) {
                        node.muxes.insert(wt, int::MuxInfo {
                            kind,
                            ins: Default::default(),
                        });
                    }
                    let mux = node.muxes.get_mut(&wt).unwrap();
                    assert_eq!(mux.kind, kind);
                    mux.ins.insert(wf);
                } else if self.stub_outs.contains(&self.rd.wires[wfi]) {
                    // ignore
                } else {
                    println!("UNEXPECTED XNODE MUX IN {} {} {} {}", self.rd.tile_kinds.key(tile.kind), tile.name, self.rd.wires[wti], self.rd.wires[wfi]);
                }
            }
        }

        self.insert_node_merge(name, node);
        self.insert_node_naming(naming, node_naming);
    }

    pub fn build(self) -> int::IntDb {
        self.db
    }
}
