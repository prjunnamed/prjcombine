use std::collections::{HashSet, HashMap};
use super::rawdump::*;

struct PartBuilderIndex {
    speeds: Vec<String>,
    node_classes: Vec<String>,
    templates: Vec<TkNodeTemplate>,
    wires: Vec<String>,
    slot_kinds: Vec<String>,
    // the above are copied to part.
    speeds_by_name: HashMap<String, SpeedIdx>,
    node_classes_by_name: HashMap<String, NodeClassIdx>,
    templates_idx: HashMap<TkNodeTemplate, u32>,
    wires_by_name: HashMap<String, WireIdx>,
    slot_kinds_by_name: HashMap<String, u16>,
}

pub struct PartBuilder {
    pub part: Part,
    index: PartBuilderIndex,
    tiles_by_name: HashMap<String, Coord>,
    fixup_nodes_queue : Vec<(String, WireIdx, SpeedIdx, NodeClassIdx)>,
}

fn split_xy(s: &str) -> Option<(&str, u32, u32)> {
    let (l, r) = match s.rfind("_X") {
        None => return None,
        Some(pos) => (&s[..pos], &s[pos+2..]),
    };
    let (x, y) = match r.rfind("Y") {
        None => return None,
        Some(pos) => (&r[..pos], &r[pos+1..]),
    };
    let x = match x.parse::<u32>() {
        Err(_) => return None,
        Ok(x) => x,
    };
    let y = match y.parse::<u32>() {
        Err(_) => return None,
        Ok(y) => y,
    };
    Some((l, x, y))
}

#[cfg(test)]
mod tests {
    #[test]
    fn split_xy_test() {
        assert_eq!(super::split_xy("SLICE_X123Y456"), Some(("SLICE", 123, 456)));
    }
}

fn get_lastnum(s: &str) -> u8 {
    let mut num : Option<u8> = None;
    for c in s.chars() {
        if c.is_ascii_digit() {
            let v = c.to_digit(10).unwrap() as u8;
            num = Some(match num {
                None => v,
                Some(c) => c * 10 + v,
            })
        } else {
            num = None
        }
    }
    num.unwrap()
}

impl PartBuilder {
    pub fn new(part: String, family: String, source: Source, width: u16, height: u16) -> Self {
        PartBuilder {
            part: Part {
                part,
                family,
                source,
                width,
                height,
                tile_kinds: HashMap::new(),
                tiles: HashMap::new(),
                speeds: Vec::new(),
                node_classes: Vec::new(),
                nodes: Vec::new(),
                templates: Vec::new(),
                wires: Vec::new(),
                slot_kinds: Vec::new(),
                packages: HashMap::new(),
                combos: Vec::new(),
            },
            tiles_by_name: HashMap::new(),
            index: PartBuilderIndex {
                speeds: Vec::new(),
                node_classes: Vec::new(),
                templates: Vec::new(),
                wires: Vec::new(),
                slot_kinds: Vec::new(),
                speeds_by_name: HashMap::new(),
                node_classes_by_name: HashMap::new(),
                templates_idx: HashMap::new(),
                wires_by_name: HashMap::new(),
                slot_kinds_by_name: HashMap::new(),
            },
            fixup_nodes_queue: Vec::new(),
        }
    }

    fn slotify<'a>(&mut self, sites: &'a [(&'a str, &'a str, Vec<(&'a str, TkSitePinDir, Option<&'a str>, Option<&'a str>)>)]) -> HashMap<&'a str, TkSiteSlot> {
        fn from_pinnum(pins: &[(&str, TkSitePinDir, Option<&str>, Option<&str>)], pin: &str) -> u8 {
            for (n, _, w, _) in pins {
                if *n == pin {
                    return get_lastnum(w.unwrap());
                }
            }
            panic!("key pin {} not found", pin);
        }

        let mut res: HashMap<&'a str, TkSiteSlot> = HashMap::new();
        let mut minxy: HashMap<u16, (u32, u32)> = HashMap::new();
        for (n, _, _) in sites {
            if let Some((base, x, y)) = split_xy(n) {
                let base = self.index.slot_kind_to_idx(base);
                let e = minxy.entry(base).or_insert((x, y));
                if x < e.0 {
                    e.0 = x;
                }
                if y < e.1 {
                    e.1 = y;
                }
            }
        }
        let mut slots: HashSet<TkSiteSlot> = HashSet::new();
        for (n, k, p) in sites {
            let slot = if self.part.family == "xc4000e" || self.part.family == "xc4000ex" || self.part.family == "xc4000xla" || self.part.family == "xc4000xv" || self.part.family == "spartanxl" {
                if let Some(urpos) = n.find("_R") {
                    if let Some(dpos) = n.find(".") {
                        TkSiteSlot::Indexed(self.index.slot_kind_to_idx(&n[..urpos]), n[dpos+1..].parse::<u8>().unwrap())
                    } else {
                        TkSiteSlot::Single(self.index.slot_kind_to_idx(&n[..urpos]))
                    }
                } else if *k == "IOB" || *k == "CLKIOB" || *k == "FCLKIOB" {
                    TkSiteSlot::Indexed(self.index.slot_kind_to_idx("IOB"), from_pinnum(p, "O"))
                } else if *k == "CIN" || *k == "COUT" || *k == "BUFF" {
                    TkSiteSlot::Single(self.index.slot_kind_to_idx(k))
                } else if *k == "PRI-CLK" {
                    TkSiteSlot::Single(self.index.slot_kind_to_idx("BUFGP"))
                } else if *k == "SEC-CLK" {
                    TkSiteSlot::Single(self.index.slot_kind_to_idx("BUFGS"))
                } else if *k == "BUFG" || *k == "BUFGE" || *k == "BUFGLS" {
                    let pos = n.find("_").unwrap();
                    TkSiteSlot::Indexed(self.index.slot_kind_to_idx(&n[..pos]), match &n[pos..] {
                        "_WNW" => 0,
                        "_ENE" => 1,
                        "_NNE" => 2,
                        "_SSE" => 3,
                        "_ESE" => 4,
                        "_WSW" => 5,
                        "_SSW" => 6,
                        "_NNW" => 7,
                        _ => panic!("cannot match {}", n),
                    })
                } else {
                    TkSiteSlot::Single(self.index.slot_kind_to_idx(n))
                }
            } else if self.part.family == "virtex" || self.part.family == "virtexe" {
                match *k {
                    "IOB" | "EMPTYIOB" | "PCIIOB" | "DLLIOB" => TkSiteSlot::Indexed(self.index.slot_kind_to_idx("IOB"), from_pinnum(p, "I")),
                    "TBUF" => TkSiteSlot::Indexed(self.index.slot_kind_to_idx(k), from_pinnum(p, "O")),
                    "SLICE" => TkSiteSlot::Indexed(self.index.slot_kind_to_idx(k), from_pinnum(p, "CIN")),
                    "GCLKIOB" => TkSiteSlot::Indexed(self.index.slot_kind_to_idx(k), from_pinnum(p, "GCLKOUT")),
                    "GCLK" => TkSiteSlot::Indexed(self.index.slot_kind_to_idx(k), from_pinnum(p, "CE")),
                    "DLL" => TkSiteSlot::Indexed(self.index.slot_kind_to_idx(k), match *n {
                        "DLL0" => 0,
                        "DLL1" => 1,
                        "DLL2" => 2,
                        "DLL3" => 3,
                        "DLL0P" => 0,
                        "DLL1P" => 1,
                        "DLL2P" => 2,
                        "DLL3P" => 3,
                        "DLL0S" => 4,
                        "DLL1S" => 5,
                        "DLL2S" => 6,
                        "DLL3S" => 7,
                        _ => panic!("cannot match {}", n),
                    }),
                    _ => TkSiteSlot::Single(self.index.slot_kind_to_idx(k))
                }
            } else if *k == "TBUF" && self.part.family.starts_with("virtex2") {
                TkSiteSlot::Indexed(self.index.slot_kind_to_idx(k), from_pinnum(p, "O"))
            } else if (*k == "GTIPAD" || *k == "GTOPAD") && self.part.family == "virtex2p" {
                let idx : u8 = match n.as_bytes()[2] {
                    b'P' => 0,
                    b'N' => 1,
                    _ => panic!("weird GT pad"),
                };
                TkSiteSlot::Indexed(self.index.slot_kind_to_idx(k), idx)
            } else if let Some((base, x, y)) = split_xy(n) {
                let base = self.index.slot_kind_to_idx(base);
                let (bx, by) = *minxy.get(&base).unwrap();
                TkSiteSlot::Xy(base, (x - bx) as u8, (y - by) as u8)
            } else if (self.part.family.starts_with("virtex2") || self.part.family.starts_with("spartan3")) && (k.starts_with("IOB") || k.starts_with("IBUF") || k.starts_with("DIFF")) {
                TkSiteSlot::Indexed(self.index.slot_kind_to_idx("IOB"), from_pinnum(p, "T"))
            } else if ((self.part.family.starts_with("virtex2") || self.part.family == "spartan3") && k.starts_with("DCI")) || (self.part.family == "spartan3" && *k == "BUFGMUX") {
                TkSiteSlot::Indexed(self.index.slot_kind_to_idx(k), get_lastnum(n))
            } else if self.part.family.starts_with("virtex2") && *k == "BUFGMUX" {
                TkSiteSlot::Indexed(self.index.slot_kind_to_idx(k), n[7..8].parse::<u8>().unwrap())
            } else if self.part.family == "spartan6" && k.starts_with("IOB") {
                TkSiteSlot::Indexed(self.index.slot_kind_to_idx("IOB"), from_pinnum(p, "PADOUT"))
            } else {
                TkSiteSlot::Single(self.index.slot_kind_to_idx(n))
            };
            assert!(!slots.contains(&slot));
            slots.insert(slot);
            res.insert(n, slot);
        }
        res
    }

    pub fn add_tile(
        &mut self,
        coord: Coord,
        name: String,
        kind: String,
        sites: &[(&str, &str, Vec<(&str, TkSitePinDir, Option<&str>, Option<&str>)>)],
        wires: &[(&str, Option<&str>)],
        pips: &[(&str, &str, bool, bool, bool, TkPipInversion, TkPipDirection, Option<&str>)],
    ) {
        assert!(coord.x < self.part.width);
        assert!(coord.y < self.part.height);

        let mut w2nc : HashMap<WireIdx, NodeClassIdx> = HashMap::new();
        let mut cpips : Vec<(WireIdx, WireIdx, NodeClassIdx, NodeClassIdx)> = Vec::new();
        let pips : Vec<_> = pips.iter().copied().map(|(wf, wt, ib, ie, it, inv, dir, s)| {
            let wf = self.index.wire_to_idx(wf);
            let wt = self.index.wire_to_idx(wt);
            let s = match s {
                Some(s) => if s == "pip_FAKEPIP" {
                    SpeedIdx::NONE
                } else if s.starts_with("pip_") && s != "pip_OPTDLY_XIPHY" {
                    let ss : Vec<_> = s.split("__").collect();
                    match ss[..] {
                        [_pk, _pid, sid, did] => {
                            let (sid, did) = if dir == TkPipDirection::BiBwd {
                                (self.index.node_class_to_idx(did), self.index.node_class_to_idx(sid))
                            } else {
                                (self.index.node_class_to_idx(sid), self.index.node_class_to_idx(did))
                            };
                            let csid = w2nc.entry(wf).or_insert(sid);
                            if *csid != sid {
                                *csid = NodeClassIdx::UNKNOWN;
                            }
                            let cdid = w2nc.entry(wt).or_insert(did);
                            if *cdid != did {
                                *cdid = NodeClassIdx::UNKNOWN;
                            }
                            cpips.push((wf, wt, sid, did));
                        }
                        _ => panic!("weird pip string {:?}", s),
                    }
                    SpeedIdx::NONE
                } else {
                    self.index.speed_to_idx(Some(s))
                },
                None => SpeedIdx::NONE,
            };
            (wf, wt, ib, ie, it, inv, dir, s)
        }).collect();
        let mut pip_overrides : HashMap<(WireIdx, WireIdx), (NodeClassIdx, NodeClassIdx)> = HashMap::new();
        for (wf, wt, sid, did) in cpips {
            if w2nc[&wf] != sid || w2nc[&wt] != did {
                pip_overrides.insert((wf, wt), (sid, did));
            }
        }
        let wires : Vec<_> = wires.iter().map(|(n, s)| {
            let w = self.index.wire_to_idx(n);
            (w, self.index.speed_to_idx(*s), w2nc.get(&w).copied().unwrap_or(NodeClassIdx::UNKNOWN))
        }).collect();
        let slots = self.slotify(sites);
        let sites_raw : Vec<_> = sites.iter().map(|(n, k, p)| (
            *slots.get(n).unwrap(),
            *n,
            *k,
            p.iter().map(|(n, d, w, s)| (
                *n,
                *d,
                match w {Some(w) => self.index.wire_to_idx(w), None => WireIdx::NONE},
                self.index.speed_to_idx(*s),
            )).collect::<Vec<_>>()
        )).collect();

        let mut sites: Vec<Option<String>> = Vec::new();
        let mut conn_wires: Vec<NodeOrClass> = Vec::new();

        let mut set_site = |i, s| {
            if sites.len() <= i {
                sites.resize(i + 1, None);
            }
            sites[i] = Some(s);
        };

        let mut set_conn_wire = |i, ni| {
            if conn_wires.len() <= i {
                conn_wires.resize(i + 1, NodeOrClass::None);
            }
            conn_wires[i] = ni;
        };

        match self.part.tile_kinds.get_mut(&kind) {
            Some(tk) => {
                for (slot, name, kind, pins) in sites_raw {
                    match tk.sites_by_slot.get(&slot) {
                        Some(idx) => {
                            let site = &mut tk.sites[*idx];
                            for (n, _, w, s) in pins {
                                let pin = site.pins.get_mut(n).unwrap();
                                if w == WireIdx::NONE { continue; }
                                if pin.wire != WireIdx::NONE && pin.wire != w {
                                    panic!("pin wire mismatch");
                                }
                                pin.wire = w;
                                if pin.speed != s {
                                    panic!("pin speed mismatch");
                                }
                            }
                            set_site(*idx, name.to_string());
                        },
                        None => {
                            let i = tk.sites.len();
                            tk.sites.push(TkSite {
                                slot: slot,
                                kind: kind.to_string(),
                                pins: pins.iter().map(|(n, d, w, s)| (n.to_string(), TkSitePin {dir: *d, wire: *w, speed: *s})).collect(),
                            });
                            tk.sites_by_slot.insert(slot, i);
                            set_site(i, name.to_string());
                        },
                    }
                }

                // Process wires.
                let mut wire_set : HashSet<WireIdx> = HashSet::new();
                for (n, s, nc) in wires {
                    wire_set.insert(n);
                    match tk.wires.get(&n).copied() {
                        None => {
                            let i = tk.conn_wires.len();
                            tk.wires.insert(n, TkWire::Connected(i));
                            tk.conn_wires.push(n);
                            set_conn_wire(i, NodeOrClass::Pending(nc));
                        },
                        Some(TkWire::Internal(cs, cnc)) => {
                            if cs != s || cnc != nc {
                                let i = tk.conn_wires.len();
                                tk.wires.insert(n, TkWire::Connected(i));
                                tk.conn_wires.push(n);
                                for crd in &tk.tiles {
                                    self.part.tiles.get_mut(crd).unwrap().set_conn_wire(i, NodeOrClass::Pending(cnc));
                                }
                                set_conn_wire(i, NodeOrClass::Pending(nc));
                            }
                        },
                        Some(TkWire::Connected(i)) => {
                            set_conn_wire(i, NodeOrClass::Pending(nc));
                        },
                    }
                }
                for (k, v) in tk.wires.iter_mut() {
                    if !wire_set.contains(k) {
                        if let TkWire::Internal(_, cnc) = *v {
                            let i = tk.conn_wires.len();
                            *v = TkWire::Connected(i);
                            tk.conn_wires.push(*k);
                            for crd in &tk.tiles {
                                self.part.tiles.get_mut(crd).unwrap().set_conn_wire(i, NodeOrClass::Pending(cnc));
                            }
                        }
                    }
                }

                // Process pips.
                for (wf, wt, ib, ie, it, inv, dir, s) in pips {
                    let k = (wf, wt);
                    let pip = TkPip {
                        is_buf: ib,
                        is_excluded: ie,
                        is_test: it,
                        inversion: inv,
                        direction: dir,
                        speed: s,
                    };
                    let orig = *tk.pips.entry(k).or_insert(pip);
                    if orig != pip {
                        panic!("pip mismatch {} {} {} {} {:?} {:?}", name, kind, self.index.wires[wf.unpack().unwrap()], self.index.wires[wt.unpack().unwrap()], pip, orig);
                    }
                }

                // Add the current tile.
                tk.tiles.push(coord);
            },
            None => {
                self.part.tile_kinds.insert(kind.to_string(), TileKind {
                    sites: sites_raw.iter().map(|(slot, _, kind, pins)| TkSite {
                        slot: *slot,
                        kind: kind.to_string(),
                        pins: pins.iter().map(|(n, d, w, s)| (n.to_string(), TkSitePin {dir: *d, wire: *w, speed: *s})).collect(),
                    }).collect(),
                    sites_by_slot: sites_raw.iter().enumerate().map(|(idx, (slot, _, _, _))| (*slot, idx)).collect(),
                    wires: wires.iter().map(|(n, s, nc)| (
                        *n, TkWire::Internal(*s, *nc)
                    )).collect(),
                    conn_wires: Vec::new(),
                    pips: pips.iter().copied().map(|(wf, wt, ib, ie, it, inv, dir, s)| (
                        (wf, wt), TkPip {
                            is_buf: ib,
                            is_excluded: ie,
                            is_test: it,
                            inversion: inv,
                            direction: dir,
                            speed: s,
                        }
                    )).collect(),
                    tiles: vec![coord],
                });
                for (_, name, _, _) in sites_raw {
                    sites.push(Some(name.to_string()));
                }
            },
        }
        self.part.tiles.insert(coord, Tile {
            name: name.clone(),
            kind,
            sites,
            conn_wires,
            pip_overrides,
        });
        self.tiles_by_name.insert(name, coord);
    }

    pub fn add_node(&mut self, wires: &[(&str, &str, Option<&str>)]) {
        let wires: Vec<_> = wires.iter().copied().map(|(t, w, s)| (
            *self.tiles_by_name.get(t).unwrap(),
            self.index.wire_to_idx(w),
            self.index.speed_to_idx(s),
        )).collect();
        if wires.len() == 1 {
            let (coord, wire, speed) = wires[0];
            let tile = self.part.tiles.get(&coord).unwrap();
            let tk = self.part.tile_kinds.get(&tile.kind).unwrap();
            let w = tk.wires.get(&wire).unwrap();
            if let TkWire::Internal(s, _) = w {
                if *s == speed {
                    return;
                }
            }
        }
        let bx = wires.iter().map(|(t, _, _)| t.x).min().unwrap();
        let by = wires.iter().map(|(t, _, _)| t.y).min().unwrap();
        let mut twires: Vec<_> = wires.iter().copied().map(|(t, w, s)| TkNodeTemplateWire {
            delta: Coord{x: t.x - bx, y: t.y - by},
            wire: w,
            speed: s,
            cls: {
                let tile = self.part.tiles.get(&t).unwrap();
                let tk = self.part.tile_kinds.get(&tile.kind).unwrap();
                match *tk.wires.get(&w).unwrap() {
                    TkWire::Internal(_, nc) => nc,
                    TkWire::Connected(idx) => {
                        match tile.get_conn_wire(idx) {
                            NodeOrClass::Pending(nc) => nc,
                            _ => NodeClassIdx::UNKNOWN,
                        }
                    }
                }
            },
        }).collect();
        twires.sort();
        let template = TkNodeTemplate {
            wires: twires,
        };
        let tidx = self.index.template_to_idx(template);
        let node = NodeOrClass::make_node(self.part.nodes.len());
        self.part.nodes.push(TkNode {
            base: Coord{x: bx, y: by},
            template: tidx,
        });
        for (coord, wire, _) in wires {
            let tile = self.part.tiles.get(&coord).unwrap();
            let kind = tile.kind.clone();
            let tk = self.part.tile_kinds.get_mut(&kind).unwrap();
            let w = tk.wires.get_mut(&wire).unwrap();
            let idx = match *w {
                TkWire::Internal(s, nc) => {
                    let i = tk.conn_wires.len();
                    *w = TkWire::Connected(i);
                    tk.conn_wires.push(wire);
                    for crd in &tk.tiles {
                        let t = self.part.tiles.get_mut(&crd).unwrap();
                        t.set_conn_wire(i, NodeOrClass::Pending(nc));
                    }
                    self.fixup_nodes_queue.push((kind, wire, s, nc));
                    i
                },
                TkWire::Connected(i) => i,
            };
            self.part.tiles.get_mut(&coord).unwrap().set_conn_wire(idx, node);
        }
    }

    pub fn add_package(&mut self, name: String, pins: Vec<PkgPin>) {
        self.part.packages.insert(name, pins);
    }
    pub fn add_combo(&mut self, name: String, device: String, package: String, speed: String, temp: String) {
        self.part.combos.push(PartCombo {name, device, package, speed, temp});
    }

    pub fn finish(mut self) -> Part {
        for (kind, w, s, nc) in self.fixup_nodes_queue {
            let tk = self.part.tile_kinds.get(&kind).unwrap();
            let idx = match *tk.wires.get(&w).unwrap() {
                TkWire::Connected(i) => i,
                _ => unreachable!(),
            };
            let mut tidx : Option<u32> = None;
            for crd in &tk.tiles {
                let t = self.part.tiles.get_mut(&crd).unwrap();
                if let NodeOrClass::Pending(_) = t.get_conn_wire(idx) {
                    let ctidx = match tidx {
                        Some(i) => i,
                        None => {
                            let template = TkNodeTemplate {
                                wires: vec![
                                    TkNodeTemplateWire {
                                        delta: Coord{x:0, y:0},
                                        wire: w,
                                        speed: s,
                                        cls: nc,
                                    },
                                ],
                            };
                            let i = self.index.template_to_idx(template);
                            tidx = Some(i);
                            i
                        }
                    };
                    let node = NodeOrClass::make_node(self.part.nodes.len());
                    self.part.nodes.push(TkNode {
                        base: *crd,
                        template: ctidx,
                    });
                    t.set_conn_wire(idx, node);
                }
            }
        }
        self.part.speeds = self.index.speeds;
        self.part.node_classes = self.index.node_classes;
        self.part.templates = self.index.templates;
        self.part.wires = self.index.wires;
        self.part.slot_kinds = self.index.slot_kinds;
        self.part
    }
}

impl PartBuilderIndex {
    fn wire_to_idx(&mut self, s: &str) -> WireIdx {
        match self.wires_by_name.get(s) {
            None => {
                let i = WireIdx::from_raw(self.wires.len());
                self.wires.push(s.to_string());
                self.wires_by_name.insert(s.to_string(), i);
                i
            },
            Some(i) => *i
        }
    }

    fn speed_to_idx(&mut self, s: Option<&str>) -> SpeedIdx {
        match s {
            None => SpeedIdx::UNKNOWN,
            Some(s) => match self.speeds_by_name.get(s) {
                None => {
                    let i = SpeedIdx::from_raw(self.speeds.len());
                    self.speeds.push(s.to_string());
                    self.speeds_by_name.insert(s.to_string(), i);
                    i
                },
                Some(i) => *i
            }
        }
    }

    fn slot_kind_to_idx(&mut self, s: &str) -> u16 {
        match self.slot_kinds_by_name.get(s) {
            None => {
                let i = self.slot_kinds.len();
                if i > u16::MAX as usize {
                    panic!("out of slot kinds");
                }
                let i = i as u16;
                self.slot_kinds.push(s.to_string());
                self.slot_kinds_by_name.insert(s.to_string(), i);
                i
            },
            Some(i) => *i
        }
    }

    fn node_class_to_idx(&mut self, s: &str) -> NodeClassIdx {
        match self.node_classes_by_name.get(s) {
            None => {
                let i = NodeClassIdx::from_raw(self.node_classes.len());
                self.node_classes.push(s.to_string());
                self.node_classes_by_name.insert(s.to_string(), i);
                i
            },
            Some(i) => *i
        }
    }

    fn template_to_idx(&mut self, template: TkNodeTemplate) -> u32 {
        match self.templates_idx.get(&template) {
            None => {
                let i = self.templates.len();
                if i > u32::MAX as usize {
                    panic!("out of templates");
                }
                let i = i as u32;
                self.templates.push(template.clone());
                self.templates_idx.insert(template, i);
                i
            },
            Some(i) => *i
        }
    }
}
