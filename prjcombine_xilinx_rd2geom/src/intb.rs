use std::collections::{HashMap, HashSet};

use prjcombine_entity::EntityPartVec;
use prjcombine_xilinx_geom::int;
use prjcombine_xilinx_rawdump::{Part, Coord, TileKind, self as rawdump};

use enum_map::{EnumMap, enum_map};

struct NodeType<'a> {
    tkn: &'a str,
    tk: &'a TileKind,
    node: int::NodeKindId,
    naming: int::NamingId,
}

pub struct IntBuilder<'a> {
    rd: &'a Part,
    rdwi: HashMap<String, rawdump::WireIdx>,
    pub db: int::IntDb,
    main_passes: EnumMap<int::Dir, EntityPartVec<int::WireId, int::WireId>>,
    node_types: Vec<NodeType<'a>>,
    stub_outs: HashSet<String>,
    extra_names: HashMap<String, int::WireId>,
    extra_names_tile: HashMap<String, HashMap<String, int::WireId>>,
}

fn name_wire(db: &mut int::IntDb, naming: int::NamingId, wire: int::WireId, name: impl AsRef<str>) {
    let name = name.as_ref();
    let naming = &mut db.namings[naming];
    if !naming.contains_id(wire) {
        naming.insert(wire, name.to_string());
    } else {
        assert_eq!(naming[wire], name);
    }
}

impl<'a> IntBuilder<'a> {
    pub fn new(name: &str, rd: &'a Part) -> Self {
        let db = int::IntDb {
            name: name.to_string(),
            wires: Default::default(),
            nodes: Default::default(),
            terms: Default::default(),
            passes: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
            namings: Default::default(),
        };
        let rdwi = rd.all_wires().map(|wi| (rd.wire(wi).to_string(), wi)).collect();
        Self {
            rd,
            rdwi,
            db,
            main_passes: enum_map!(_ => Default::default()),
            node_types: vec![],
            stub_outs: Default::default(),
            extra_names: Default::default(),
            extra_names_tile: Default::default(),
        }
    }

    pub fn node_type(&mut self, tile_kind: &str, node: &str, naming: &str) {
        assert_eq!(self.db.wires.len(), 0);
        if let Some((tkn, tk)) = self.rd.tile_kinds.get_key_value(tile_kind) {
            let node = match self.db.nodes.get(node) {
                None => self.db.nodes.insert(node.to_string(), int::NodeKind {
                    muxes: Default::default(),
                }).0,
                Some((i, _)) => i,
            };
            let naming = self.make_naming(naming);
            self.node_types.push(NodeType {
                tkn,
                tk,
                node,
                naming,
            });
        }
    }

    pub fn make_naming(&mut self, name: impl AsRef<str>) -> int::NamingId {
        match self.db.namings.get(name.as_ref()) {
            None => self.db.namings.insert(name.as_ref().to_string(), Default::default()).0,
            Some((i, _)) => i,
        }
    }

    pub fn name_wire(&mut self, naming: int::NamingId, wire: int::WireId, name: impl AsRef<str>) {
        name_wire(&mut self.db, naming, wire, name);
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
        for nt in &self.node_types {
            for rn in raw_names {
                let rn = rn.as_ref();
                for &wi in nt.tk.wires.keys() {
                    if self.rd.wire(wi) == rn {
                        name_wire(&mut self.db, nt.naming, res, rn);
                    }
                }
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

    pub fn test_out(&mut self, name: impl Into<String>) -> int::WireId {
        self.wire(name, int::WireKind::TestOut, &[""])
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
        self.extra_names.insert(name.into(), wire);
    }

    pub fn extra_name_tile(&mut self, tile: impl Into<String>, name: impl Into<String>, wire: int::WireId) {
        self.extra_names_tile.entry(tile.into()).or_default().insert(name.into(), wire);
    }

    pub fn extract_nodes(&mut self) {
        for nt in &self.node_types {
            let naming = &self.db.namings[nt.naming];
            let rev_naming: HashMap<_, _> = naming.iter().map(|(k, v)| (v.to_string(), k)).collect();
            let mut names: HashMap<rawdump::WireIdx, int::WireId> = HashMap::new();
            for &wi in nt.tk.wires.keys() {
                let n = self.rd.wire(wi);
                if let Some(&w) = rev_naming.get(n) {
                    names.insert(wi, w);
                }
            }

            let node = &mut self.db.nodes[nt.node];

            for &(wfi, wti) in nt.tk.pips.keys() {
                if let Some(&wt) = names.get(&wti) {
                    match self.db.wires[wt].kind {
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
                            assert_eq!(wf, dwf);
                            continue;
                        }
                        _ => continue,
                    }
                    if let Some(&wf) = names.get(&wfi) {
                        // XXX
                        let kind = int::MuxKind::Plain;
                        if !node.muxes.contains_id(wt) {
                            node.muxes.insert(wt, int::MuxInfo {
                                kind,
                                ins: Default::default(),
                            });
                        }
                        let mux = &mut node.muxes[wt];
                        assert_eq!(mux.kind, kind);
                        mux.ins.insert(wf);
                    } else if self.stub_outs.contains(self.rd.wire(wfi)) {
                        // ignore
                    } else {
                        println!("UNEXPECTED INT MUX IN {} {} {}", nt.tkn, self.rd.wire(wti), self.rd.wire(wfi));
                    }
                }
            }
        }
        for (dir, wires) in &self.main_passes {
            self.db.passes.insert(format!("MAIN.{dir}"), int::PassKind {
                dir,
                wires: wires.iter().map(|(k, &v)| (k, int::PassInfo::Pass(int::PassWireIn::Far(v)))).collect()
            });
        }
    }

    fn get_int_naming(&self, int_xy: Coord) -> int::NamingId {
        let int_tile = &self.rd.tiles[&int_xy];
        self.node_types.iter().find_map(|nt| if nt.tkn == int_tile.kind { Some(nt.naming) } else { None }).unwrap()
    }

    fn get_int_rev_naming(&self, int_xy: Coord) -> HashMap<String, int::WireId> {
        let int_naming_id = self.get_int_naming(int_xy);
        let int_naming = &self.db.namings[int_naming_id];
        int_naming.iter().map(|(k, v)| (v.to_string(), k)).collect()
    }

    fn get_node(&self, tile: &rawdump::Tile, tk: &rawdump::TileKind, wi: rawdump::WireIdx) -> Option<u32> {
        if let Some(&rawdump::TkWire::Connected(idx)) = tk.wires.get(&wi) {
            if let rawdump::NodeOrClass::Node(nidx) = tile.get_conn_wire(idx) {
                return Some(nidx);
            }
        }
        None
    }

    fn get_int_node2wires(&self, int_xy: Coord) -> HashMap<u32, Vec<int::WireId>> {
        let int_tile = &self.rd.tiles[&int_xy];
        let int_tk = &self.rd.tile_kinds[&int_tile.kind];
        let int_rev_naming = self.get_int_rev_naming(int_xy);
        let mut res: HashMap<_, Vec<_>> = HashMap::new();
        for (&wi, &tkw) in &int_tk.wires {
            if let Some(&w) = int_rev_naming.get(self.rd.wire(wi)) {
                if let rawdump::TkWire::Connected(idx) = tkw {
                    if let rawdump::NodeOrClass::Node(nidx) = int_tile.get_conn_wire(idx) {
                        res.entry(nidx).or_default().push(w);
                    }
                }
            }
        }
        res
    }

    pub fn recover_names(&self, tile_xy: Coord, int_xy: Coord) -> HashMap<rawdump::WireIdx, Vec<int::WireId>> {
        if tile_xy == int_xy {
            let int_tile = &self.rd.tiles[&int_xy];
            let int_tk = &self.rd.tile_kinds[&int_tile.kind];
            let int_rev_naming = self.get_int_rev_naming(int_xy);
            let mut res = HashMap::new();
            for &wi in int_tk.wires.keys() {
                let n = self.rd.wire(wi);
                if let Some(&w) = int_rev_naming.get(n) {
                    res.insert(wi, vec![w]);
                }
            }
            res
        } else {
            let node2wires = self.get_int_node2wires(int_xy);
            let tile = &self.rd.tiles[&tile_xy];
            let tk = &self.rd.tile_kinds[&tile.kind];
            let mut res = HashMap::new();
            for (&wi, &tkw) in &tk.wires {
                if let Some(&w) = self.extra_names.get(self.rd.wire(wi)) {
                    res.insert(wi, vec![w]);
                } else if let Some(&w) = self.extra_names_tile.get(&tile.kind).and_then(|x| x.get(self.rd.wire(wi))) {
                    res.insert(wi, vec![w]);
                } else if let rawdump::TkWire::Connected(idx) = tkw {
                    if let rawdump::NodeOrClass::Node(nidx) = tile.get_conn_wire(idx) {
                        if let Some(w) = node2wires.get(&nidx) {
                            res.insert(wi, w.clone());
                        }
                    }
                }
            }
            res
        }
    }

    pub fn recover_names_cands(&self, tile_xy: Coord, int_xy: Coord, cands: &HashSet<int::WireId>) -> HashMap<rawdump::WireIdx, int::WireId> {
        self.recover_names(tile_xy, int_xy).into_iter().filter_map(|(k, v)| {
            let nv: Vec<_> = v.into_iter().filter(|x| cands.contains(x)).collect();
            if nv.len() == 1 {
                Some((k, nv[0]))
            } else {
                None
            }
        }).collect()
    }

    fn insert_term_merge(&mut self, name: impl AsRef<str>, term: int::TermKind) {
        match self.db.terms.get_mut(name.as_ref()) {
            None => {
                self.db.terms.insert(name.as_ref().to_string(), term);
            }
            Some((_, cterm)) => {
                assert_eq!(term.dir, cterm.dir);
                for k in self.db.wires.ids() {
                    let a = cterm.wires.get_mut(k);
                    let b = term.wires.get(k);
                    match (a, b) {
                        (Some(&mut int::TermInfo::Mux(ref mut al)), Some(&int::TermInfo::Mux(ref bl))) => {
                            for &w in bl.iter() {
                                al.insert(w);
                            }
                        }
                        (_, None) => (),
                        (None, Some(b)) => {
                            cterm.wires.insert(k, b.clone());
                        },
                        (a, b) => assert_eq!(a.map(|x| &*x), b),
                    }
                }
            }
        };
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
                        res.insert(wt, int::TermInfo::Pass(fwf));
                    }
                } else {
                    if wf.len() == 1 {
                        res.insert(wt, int::TermInfo::Pass(wf[0]));
                    }
                    if wf.len() > 1 {
                        println!("WHOOPS MULTI {} {:?}", self.db.wires[wt].name, wf.iter().map(|&x| &self.db.wires[x].name).collect::<Vec<_>>());
                    }
                }
            }
        }
        res
    }

    pub fn extract_term_tile(&mut self, name: impl AsRef<str>, dir: int::Dir, term_xy: Coord, naming: impl AsRef<str>, int_xy: Coord) {
        let cand_inps = self.get_pass_inps(!dir);
        let names = self.recover_names(term_xy, int_xy);
        let tile = &self.rd.tiles[&term_xy];
        let tk = &self.rd.tile_kinds[&tile.kind];
        let mut muxes: HashMap<int::WireId, Vec<int::WireId>> = HashMap::new();
        let naming_id = self.make_naming(naming);
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
                                println!("AMBIG TERM MUX IN {} {} {}", tile.kind, self.rd.wire(wti), self.rd.wire(wfi));
                                continue;
                            }
                        }
                        self.name_wire(naming_id, wt, self.rd.wire(wti));
                        self.name_wire(naming_id, wf, self.rd.wire(wfi));
                        muxes.entry(wt).or_default().push(wf);
                    } else {
                        println!("UNEXPECTED TERM MUX IN {} {} {}", tile.kind, self.rd.wire(wti), self.rd.wire(wfi));
                    }
                }
            }
        }
        let mut wires = self.extract_term_tile_conn(dir, int_xy, &Default::default());
        for (k, v) in muxes {
            if v.len() == 1 {
                wires.insert(k, int::TermInfo::Pass(v[0]));
            } else {
                wires.insert(k, int::TermInfo::Mux(v.into_iter().collect()));
            }
        }
        let term = int::TermKind {
            dir,
            wires,
        };
        self.insert_term_merge(name, term);
    }

    pub fn extract_term_buf_tile(&mut self, name: impl AsRef<str>, dir: int::Dir, term_xy: Coord, naming: impl AsRef<str>, int_xy: Coord, forced: &[(int::WireId, int::WireId)]) {
        let forced: HashMap<_, _> = forced.iter().copied().collect();
        let cand_inps = self.get_pass_inps(!dir);
        let naming = naming.as_ref();
        let names = self.recover_names(term_xy, int_xy);
        let tile = &self.rd.tiles[&term_xy];
        let tk = &self.rd.tile_kinds[&tile.kind];
        let mut wires = self.extract_term_tile_conn(dir, int_xy, &forced);
        let naming_in_id = self.make_naming(format!("{naming}.IN"));
        let naming_out_id = self.make_naming(format!("{naming}.OUT"));
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
                                    println!("AMBIG TERM MUX IN {} {} {}", tile.kind, self.rd.wire(wti), self.rd.wire(wfi));
                                    continue;
                                }
                            }
                        }
                        self.name_wire(naming_out_id, wt, self.rd.wire(wti));
                        self.name_wire(naming_in_id, wt, self.rd.wire(wfi));
                        if wires.contains_id(wt) {
                            println!("OOPS DUPLICATE TERM BUF {} {}", tile.kind, self.rd.wire(wti));
                        }
                        assert!(!wires.contains_id(wt));
                        wires.insert(wt, int::TermInfo::Pass(wf));
                    } else {
                        println!("UNEXPECTED TERM BUF IN {} {} {}", tile.kind, self.rd.wire(wti), self.rd.wire(wfi));
                    }
                }
            }
        }
        let term = int::TermKind {
            dir,
            wires,
        };
        self.insert_term_merge(name, term);
    }

    pub fn extract_term_conn_tile(&mut self, name: impl AsRef<str>, dir: int::Dir, int_xy: Coord, forced: &[(int::WireId, int::WireId)]) {
        let forced: HashMap<_, _> = forced.iter().copied().collect();
        let wires = self.extract_term_tile_conn(dir, int_xy, &forced);
        let term = int::TermKind {
            dir,
            wires,
        };
        self.insert_term_merge(name, term);
    }

    pub fn walk_to_int(&self, mut xy: Coord, dir: int::Dir) -> Option<Coord> {
        loop {
            let tile = &self.rd.tiles[&xy];
            if self.node_types.iter().any(|x| x.tkn == tile.kind) {
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

    pub fn extract_term(&mut self, name: impl AsRef<str>, dir: int::Dir, tkn: impl AsRef<str>, naming: impl AsRef<str>) {
        if let Some(tk) = self.rd.tile_kinds.get(tkn.as_ref()) {
            for &term_xy in &tk.tiles {
                if let Some(int_xy) = self.walk_to_int(term_xy, !dir) {
                    self.extract_term_tile(name.as_ref(), dir, term_xy, naming.as_ref(), int_xy);
                }
            }
        }
    }

    pub fn extract_term_buf(&mut self, name: impl AsRef<str>, dir: int::Dir, tkn: impl AsRef<str>, naming: impl AsRef<str>, forced: &[(int::WireId, int::WireId)]) {
        if let Some(tk) = self.rd.tile_kinds.get(tkn.as_ref()) {
            for &term_xy in &tk.tiles {
                if let Some(int_xy) = self.walk_to_int(term_xy, !dir) {
                    self.extract_term_buf_tile(name.as_ref(), dir, term_xy, naming.as_ref(), int_xy, forced);
                }
            }
        }
    }

    pub fn extract_term_conn(&mut self, name: impl AsRef<str>, dir: int::Dir, tkn: impl AsRef<str>, forced: &[(int::WireId, int::WireId)]) {
        if let Some(tk) = self.rd.tile_kinds.get(tkn.as_ref()) {
            for &term_xy in &tk.tiles {
                if let Some(int_xy) = self.walk_to_int(term_xy, !dir) {
                    self.extract_term_conn_tile(name.as_ref(), dir, int_xy, forced);
                }
            }
        }
    }

    fn get_bufs(&self, tk: &rawdump::TileKind) -> HashMap<rawdump::WireIdx, rawdump::WireIdx> {
        let mut mux_ins: HashMap<rawdump::WireIdx, Vec<rawdump::WireIdx>> = HashMap::new();
        for &(wfi, wti) in tk.pips.keys() {
            mux_ins.entry(wti).or_default().push(wfi);
        }
        mux_ins.into_iter().filter_map(|(k, v)| if v.len() == 1 {Some((k, v[0]))} else {None}).collect()
    }

    pub fn extract_pass_tile(&mut self, name: impl AsRef<str>, dir: int::Dir, int_xy: Coord, near: Option<(Coord, &str, Option<&str>)>, far: Option<(Coord, &str, &str)>, src_xy: Coord, force_pass: &[int::WireId]) {
        let cand_inps_far = self.get_pass_inps(dir);
        let int_tile = &self.rd.tiles[&int_xy];
        let int_tk = &self.rd.tile_kinds[&int_tile.kind];
        let int_naming = &self.db.namings[self.get_int_naming(int_xy)];
        let mut wires = EntityPartVec::new();
        let src_node2wires = self.get_int_node2wires(src_xy);
        if self.rd.family.starts_with("virtex2") {
            let tcwires = self.extract_term_tile_conn(dir, int_xy, &Default::default());
            for (wt, ti) in tcwires {
                if let int::TermInfo::Pass(wf) = ti {
                    wires.insert(wt, int::PassInfo::Pass(int::PassWireIn::Near(wf)));
                }
            }
        }
        for &wn in force_pass {
            if let Some(&wf) = self.main_passes[dir].get(wn) {
                wires.insert(wn, int::PassInfo::Pass(int::PassWireIn::Far(wf)));
            }
        }
        for wn in self.main_passes[dir].ids() {
            if let Some(wnn) = int_naming.get(wn) {
                let wni = self.rdwi[wnn];
                if let Some(nidx) = self.get_node(int_tile, int_tk, wni) {
                    if let Some(w) = src_node2wires.get(&nidx) {
                        if w.len() == 1 {
                            wires.insert(wn, int::PassInfo::Pass(int::PassWireIn::Far(w[0])));
                        }
                    }
                }
            }
        }

        if let Some((xy, naming, naming_far)) = near {
            let names = self.recover_names(xy, int_xy);
            let names_far = self.recover_names_cands(xy, src_xy, &cand_inps_far);
            let mut names_far_buf = HashMap::new();
            let tile = &self.rd.tiles[&xy];
            let tk = &self.rd.tile_kinds[&tile.kind];
            if let Some((far_xy, _, _)) = far {
                let far_tile = &self.rd.tiles[&far_xy];
                let far_tk = &self.rd.tile_kinds[&far_tile.kind];
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
            let mut muxes: HashMap<int::WireId, Vec<int::PassWireIn>> = HashMap::new();
            let naming = self.make_naming(naming);
            let naming_far = naming_far.map(|x| self.make_naming(x));
            let naming_far_out = far.map(|x| self.make_naming(x.1));
            let naming_far_in = far.map(|x| self.make_naming(x.2));
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
                        self.name_wire(naming, wt, self.rd.wire(wti));
                        if let Some(wfl) = names.get(&wfi) {
                            if wfl.len() != 1 {
                                println!("AMBIG PASS MUX IN {} {} {}", tile.kind, self.rd.wire(wti), self.rd.wire(wfi));
                                continue;
                            }
                            let wf = wfl[0];
                            self.name_wire(naming, wf, self.rd.wire(wfi));
                            muxes.entry(wt).or_default().push(int::PassWireIn::Near(wf));
                        } else if let Some(&wf) = names_far.get(&wfi) {
                            self.name_wire(naming_far.unwrap(), wf, self.rd.wire(wfi));
                            muxes.entry(wt).or_default().push(int::PassWireIn::Far(wf));
                        } else if let Some(&(wf, woi, wii)) = names_far_buf.get(&wfi) {
                            self.name_wire(naming_far.unwrap(), wf, self.rd.wire(wfi));
                            self.name_wire(naming_far_out.unwrap(), wf, self.rd.wire(woi));
                            self.name_wire(naming_far_in.unwrap(), wf, self.rd.wire(wii));
                            muxes.entry(wt).or_default().push(int::PassWireIn::Far(wf));
                        } else if self.stub_outs.contains(self.rd.wire(wfi)) {
                            // ignore
                        } else {
                            println!("UNEXPECTED PASS MUX IN {} {} {}", tile.kind, self.rd.wire(wti), self.rd.wire(wfi));
                        }
                    }
                }
            }
            for (k, v) in muxes {
                assert!(!wires.contains_id(k));
                if v.len() == 1 {
                    wires.insert(k, int::PassInfo::Pass(v[0]));
                } else {
                    wires.insert(k, int::PassInfo::Mux(v.into_iter().collect()));
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
                            println!("WEIRD SPLITTER {} {} {}", tile.kind, self.rd.wire(wti), self.rd.wire(wfi));
                        } else {
                            self.name_wire(naming, wt, self.rd.wire(wti));
                            self.name_wire(naming_far.unwrap(), wf, self.rd.wire(wfi));
                            wires.insert(wt, int::PassInfo::BiSplitter(int::PassWireIn::Far(wf)));
                        }
                    }
                }
            }
        }

        let pass = int::PassKind {
            dir,
            wires,
        };
        match self.db.passes.get(name.as_ref()) {
            None => {
                self.db.passes.insert(name.as_ref().to_string(), pass);
            }
            Some((_, cpass)) => {
                assert_eq!(*cpass, pass);
            }
        }
    }

    pub fn extract_pass_simple(&mut self, name: impl AsRef<str>, dir: int::Dir, tkn: impl AsRef<str>, force_pass: &[int::WireId]) {
        if let Some(tk) = self.rd.tile_kinds.get(tkn.as_ref()) {
            for xy in tk.tiles.iter().copied() {
                let int_fwd_xy = self.walk_to_int(xy, dir).unwrap();
                let int_bwd_xy = self.walk_to_int(xy, !dir).unwrap();
                self.extract_pass_tile(format!("{}.{}", name.as_ref(), dir), dir, int_bwd_xy, None, None, int_fwd_xy, force_pass);
                self.extract_pass_tile(format!("{}.{}", name.as_ref(), !dir), !dir, int_fwd_xy, None, None, int_bwd_xy, force_pass);
            }
        }
    }

    pub fn extract_pass_buf(&mut self, name: impl AsRef<str>, dir: int::Dir, tkn: impl AsRef<str>, naming: impl AsRef<str>) {
        if let Some(tk) = self.rd.tile_kinds.get(tkn.as_ref()) {
            for xy in tk.tiles.iter().copied() {
                let int_fwd_xy = self.walk_to_int(xy, dir).unwrap();
                let int_bwd_xy = self.walk_to_int(xy, !dir).unwrap();
                let naming_fwd = format!("{}.{}", naming.as_ref(), dir);
                let naming_bwd = format!("{}.{}", naming.as_ref(), !dir);
                self.extract_pass_tile(format!("{}.{}", name.as_ref(), dir), dir, int_bwd_xy, Some((xy, &naming_bwd, Some(&naming_fwd))), None, int_fwd_xy, &[]);
                self.extract_pass_tile(format!("{}.{}", name.as_ref(), !dir), !dir, int_fwd_xy, Some((xy, &naming_fwd, Some(&naming_bwd))), None, int_bwd_xy, &[]);
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

    pub fn extract_intf_tile(&mut self, name: impl AsRef<str>, xy: Coord, int_xy: Coord, naming: impl AsRef<str>, buf_naming: Option<&str>, site_naming: Option<&str>, delay_naming: Option<&str>) {
        let names = self.recover_names(xy, int_xy);
        let tile = &self.rd.tiles[&xy];
        let tk = &self.rd.tile_kinds[&tile.kind];
        let naming = self.make_naming(naming);
        let buf_naming = buf_naming.map(|x| self.make_naming(x));
        let site_naming = site_naming.map(|x| self.make_naming(x));
        let delay_naming = delay_naming.map(|x| self.make_naming(x));
        let mut out_muxes: HashMap<int::WireId, Vec<int::WireId>> = HashMap::new();
        let bufs = self.get_bufs(tk);
        let mut wires = EntityPartVec::new();
        let mut delayed = HashMap::new();
        if delay_naming.is_some() {
            for (&wdi, &wfi) in &bufs {
                if let Some(wfl) = names.get(&wfi) {
                    for &wf in wfl {
                        if !matches!(self.db.wires[wf].kind, int::WireKind::MuxOut) {
                            continue;
                        }
                        for &wti in tk.wires.keys() {
                            if tk.pips.contains_key(&(wfi, wti)) && tk.pips.contains_key(&(wdi, wti)) {
                                self.name_wire(naming, wf, self.rd.wire(wfi));
                                self.name_wire(site_naming.unwrap(), wf, self.rd.wire(wti));
                                self.name_wire(delay_naming.unwrap(), wf, self.rd.wire(wdi));
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
                    self.name_wire(naming, wt, self.rd.wire(wti));
                    let mut rwfi = wfi;
                    if buf_naming.is_some() && bufs.contains_key(&wfi) {
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
                                println!("AMBIG INTF MUX IN {} {} {}", tile.kind, self.rd.wire(wti), self.rd.wire(wfi));
                                continue;
                            }
                        }
                        self.name_wire(naming, wf, self.rd.wire(rwfi));
                        if rwfi != wfi {
                            self.name_wire(buf_naming.unwrap(), wf, self.rd.wire(wfi));
                        }
                        assert!(!wires.contains_id(wf));
                        out_muxes.entry(wt).or_default().push(wf);
                    } else if let Some(&wf) = delayed.get(&wfi) {
                        out_muxes.entry(wt).or_default().push(wf);
                    } else {
                        if let Some(sn) = site_naming {
                            out_muxes.entry(wt).or_default();
                            self.name_wire(sn, wt, self.rd.wire(wfi));
                        }
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

    pub fn extract_intf(&mut self, name: impl AsRef<str>, dir: int::Dir, tkn: impl AsRef<str>, naming: impl AsRef<str>, buf_naming: Option<&str>, site_naming: Option<&str>, delay_naming: Option<&str>) {
        if let Some(tk) = self.rd.tile_kinds.get(tkn.as_ref()) {
            for &xy in &tk.tiles {
                let int_xy = self.walk_to_int(xy, !dir).unwrap();
                self.extract_intf_tile(name.as_ref(), xy, int_xy, naming.as_ref(), buf_naming, site_naming, delay_naming);
            }
        }
    }

    pub fn build(self) -> int::IntDb {
        self.db
    }
}
