use prjcombine_entity::{EntityMap, EntityPartVec, EntitySet, EntityVec};
use prjcombine_rawdump::*;
use std::collections::{HashMap, HashSet};

pub struct PartBuilder {
    pub part: Part,
    tiles_by_name: HashMap<String, Coord>,
    pending_wires: HashMap<Coord, EntityPartVec<TkConnWireId, Option<NodeClassId>>>,
    fixup_nodes_queue: Vec<(TileKindId, WireId, Option<SpeedId>, Option<NodeClassId>)>,
}

#[derive(Copy, Clone, Debug)]
pub struct PbPip<'a> {
    pub wire_from: &'a str,
    pub wire_to: &'a str,
    pub is_buf: bool,
    pub is_excluded: bool,
    pub is_test: bool,
    pub inv: TkPipInversion,
    pub dir: TkPipDirection,
    pub speed: Option<&'a str>,
}

pub struct PbSitePin<'a> {
    pub name: &'a str,
    pub dir: TkSitePinDir,
    pub wire: Option<&'a str>,
    pub speed: Option<&'a str>,
}

fn split_xy(s: &str) -> Option<(&str, u32, u32)> {
    let (l, r) = s.rsplit_once("_X")?;
    let (x, y) = r.rsplit_once('Y')?;
    let x = x.parse().ok()?;
    let y = y.parse().ok()?;
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
    let mut num: Option<u8> = None;
    let mut res = None;
    for c in s.chars() {
        if c.is_ascii_digit() {
            let v = c.to_digit(10).unwrap() as u8;
            num = Some(match num {
                None => v,
                Some(c) => c * 10 + v,
            });
            res = num;
        } else {
            num = None;
        }
    }
    res.unwrap()
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
                tile_kinds: EntityMap::new(),
                tiles: HashMap::new(),
                speeds: EntitySet::new(),
                node_classes: EntitySet::new(),
                nodes: EntityVec::new(),
                templates: EntitySet::new(),
                wires: EntitySet::new(),
                slot_kinds: EntitySet::new(),
                packages: HashMap::new(),
                combos: Vec::new(),
            },
            tiles_by_name: HashMap::new(),
            pending_wires: HashMap::new(),
            fixup_nodes_queue: Vec::new(),
        }
    }

    fn slotify<'a>(
        &mut self,
        sites: &'a [(&'a str, &'a str, Vec<PbSitePin<'a>>)],
    ) -> HashMap<&'a str, TkSiteSlot> {
        fn from_pinnum(pins: &[PbSitePin<'_>], refpin: &str) -> u8 {
            for pin in pins {
                if pin.name == refpin {
                    return get_lastnum(pin.wire.unwrap());
                }
            }
            panic!("key pin {refpin} not found");
        }

        let mut res: HashMap<&'a str, TkSiteSlot> = HashMap::new();
        let mut minxy: HashMap<SlotKindId, (u32, u32)> = HashMap::new();
        for (n, _, _) in sites {
            if let Some((base, x, y)) = split_xy(n) {
                let base = self.part.slot_kinds.get_or_insert(base);
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
        for (i, &(n, k, ref p)) in sites.iter().enumerate() {
            let slot = if self.part.family == "xc3000a" {
                match k {
                    "IOB" | "TBUF" | "PULLUP" => {
                        TkSiteSlot::Indexed(self.part.slot_kinds.get_or_insert(k), i as u8)
                    }
                    _ => TkSiteSlot::Single(self.part.slot_kinds.get_or_insert(k)),
                }
            } else if self.part.family == "xc4000e"
                || self.part.family == "xc4000ex"
                || self.part.family == "xc4000xla"
                || self.part.family == "xc4000xv"
                || self.part.family == "spartanxl"
            {
                if let Some(urpos) = n.find("_R") {
                    if let Some(dpos) = n.find('.') {
                        TkSiteSlot::Indexed(
                            self.part.slot_kinds.get_or_insert(&n[..urpos]),
                            n[dpos + 1..].parse::<u8>().unwrap(),
                        )
                    } else {
                        TkSiteSlot::Single(self.part.slot_kinds.get_or_insert(&n[..urpos]))
                    }
                } else {
                    match k {
                        "IOB" | "CLKIOB" | "FCLKIOB" => TkSiteSlot::Indexed(
                            self.part.slot_kinds.get_or_insert("IOB"),
                            from_pinnum(p, "O"),
                        ),
                        "CIN" | "COUT" | "BUFF" => {
                            TkSiteSlot::Single(self.part.slot_kinds.get_or_insert(k))
                        }
                        "PRI-CLK" => {
                            TkSiteSlot::Single(self.part.slot_kinds.get_or_insert("BUFGP"))
                        }
                        "SEC-CLK" => {
                            TkSiteSlot::Single(self.part.slot_kinds.get_or_insert("BUFGS"))
                        }
                        "BUFG" | "BUFGE" | "BUFGLS" => {
                            let pos = n.find('_').unwrap();
                            TkSiteSlot::Indexed(
                                self.part.slot_kinds.get_or_insert(&n[..pos]),
                                match &n[pos..] {
                                    "_WNW" => 0,
                                    "_ENE" => 1,
                                    "_NNE" => 2,
                                    "_SSE" => 3,
                                    "_ESE" => 4,
                                    "_WSW" => 5,
                                    "_SSW" => 6,
                                    "_NNW" => 7,
                                    _ => panic!("cannot match {n}"),
                                },
                            )
                        }
                        _ => TkSiteSlot::Single(self.part.slot_kinds.get_or_insert(n)),
                    }
                }
            } else if self.part.family == "xc5200" {
                if let Some(urpos) = n.find("_R") {
                    if let Some(dpos) = n.find('.') {
                        let end = &n[dpos + 1..];
                        let idx = if let Some(sfx) = end.strip_prefix("LC") {
                            sfx.parse::<u8>().unwrap()
                        } else {
                            end.parse::<u8>().unwrap()
                        };
                        TkSiteSlot::Indexed(self.part.slot_kinds.get_or_insert(&n[..urpos]), idx)
                    } else {
                        TkSiteSlot::Single(self.part.slot_kinds.get_or_insert(&n[..urpos]))
                    }
                } else {
                    match k {
                        "IOB" | "CLKIOB" => TkSiteSlot::Indexed(
                            self.part.slot_kinds.get_or_insert("IOB"),
                            from_pinnum(p, "O"),
                        ),
                        _ => TkSiteSlot::Single(self.part.slot_kinds.get_or_insert(n)),
                    }
                }
            } else if matches!(&self.part.family[..], "virtex" | "virtexe") {
                match k {
                    "IOB" | "EMPTYIOB" | "PCIIOB" | "DLLIOB" => TkSiteSlot::Indexed(
                        self.part.slot_kinds.get_or_insert("IOB"),
                        from_pinnum(p, "I"),
                    ),
                    "TBUF" => TkSiteSlot::Indexed(
                        self.part.slot_kinds.get_or_insert(k),
                        from_pinnum(p, "O"),
                    ),
                    "SLICE" => TkSiteSlot::Indexed(
                        self.part.slot_kinds.get_or_insert(k),
                        from_pinnum(p, "CIN"),
                    ),
                    "GCLKIOB" => TkSiteSlot::Indexed(
                        self.part.slot_kinds.get_or_insert(k),
                        from_pinnum(p, "GCLKOUT"),
                    ),
                    "GCLK" => TkSiteSlot::Indexed(
                        self.part.slot_kinds.get_or_insert(k),
                        from_pinnum(p, "CE"),
                    ),
                    "DLL" => TkSiteSlot::Indexed(
                        self.part.slot_kinds.get_or_insert(k),
                        match n {
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
                            _ => panic!("cannot match {n}"),
                        },
                    ),
                    _ => TkSiteSlot::Single(self.part.slot_kinds.get_or_insert(k)),
                }
            } else if k == "TBUF" && self.part.family.starts_with("virtex2") {
                TkSiteSlot::Indexed(self.part.slot_kinds.get_or_insert(k), from_pinnum(p, "O"))
            } else if matches!(k, "GTIPAD" | "GTOPAD") && self.part.family == "virtex2p" {
                let idx: u8 = match n.as_bytes()[2] {
                    b'P' => 0,
                    b'N' => 1,
                    _ => panic!("weird GT pad"),
                };
                TkSiteSlot::Indexed(self.part.slot_kinds.get_or_insert(k), idx)
            } else if let Some((base, x, y)) = split_xy(n) {
                let base = self.part.slot_kinds.get_or_insert(base);
                let (bx, by) = minxy[&base];
                TkSiteSlot::Xy(base, (x - bx) as u8, (y - by) as u8)
            } else if (self.part.family.starts_with("virtex2")
                || self.part.family.starts_with("spartan3"))
                && (k.starts_with("IOB") || k.starts_with("IBUF") || k.starts_with("DIFF"))
            {
                TkSiteSlot::Indexed(
                    self.part.slot_kinds.get_or_insert("IOB"),
                    from_pinnum(p, "T"),
                )
            } else if ((self.part.family.starts_with("virtex2") || self.part.family == "spartan3")
                && k.starts_with("DCI"))
                || (self.part.family == "spartan3" && k == "BUFGMUX")
            {
                TkSiteSlot::Indexed(self.part.slot_kinds.get_or_insert(k), get_lastnum(n))
            } else if self.part.family.starts_with("virtex2") && k == "BUFGMUX" {
                TkSiteSlot::Indexed(
                    self.part.slot_kinds.get_or_insert(k),
                    n[7..8].parse::<u8>().unwrap(),
                )
            } else if self.part.family == "spartan6" && k.starts_with("IOB") {
                TkSiteSlot::Indexed(
                    self.part.slot_kinds.get_or_insert("IOB"),
                    from_pinnum(p, "PADOUT"),
                )
            } else {
                TkSiteSlot::Single(self.part.slot_kinds.get_or_insert(n))
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
        sites: &[(&str, &str, Vec<PbSitePin<'_>>)],
        wires: &[(&str, Option<&str>)],
        pips: &[PbPip<'_>],
    ) {
        assert!(coord.x < self.part.width);
        assert!(coord.y < self.part.height);

        let mut w2nc: HashMap<WireId, Option<NodeClassId>> = HashMap::new();
        let mut cpips: Vec<(WireId, WireId, NodeClassId, NodeClassId)> = Vec::new();
        let pips: Vec<_> = pips
            .iter()
            .map(|pip| {
                let wf = self.part.wires.get_or_insert(pip.wire_from);
                let wt = self.part.wires.get_or_insert(pip.wire_to);
                let speed = match pip.speed {
                    Some(s) => {
                        if s == "pip_FAKEPIP" {
                            None
                        } else if s.starts_with("pip_") && s != "pip_OPTDLY_XIPHY" {
                            let ss: Vec<_> = s.split("__").collect();
                            match ss[..] {
                                [_pk, _pid, sid, did] => {
                                    let (sid, did) = if pip.dir == TkPipDirection::BiBwd {
                                        (
                                            self.part.node_classes.get_or_insert(did),
                                            self.part.node_classes.get_or_insert(sid),
                                        )
                                    } else {
                                        (
                                            self.part.node_classes.get_or_insert(sid),
                                            self.part.node_classes.get_or_insert(did),
                                        )
                                    };
                                    let csid = w2nc.entry(wf).or_insert(Some(sid));
                                    if *csid != Some(sid) {
                                        *csid = None;
                                    }
                                    let cdid = w2nc.entry(wt).or_insert(Some(did));
                                    if *cdid != Some(did) {
                                        *cdid = None;
                                    }
                                    cpips.push((wf, wt, sid, did));
                                }
                                _ => panic!("weird pip string {s:?}"),
                            }
                            None
                        } else {
                            Some(self.part.speeds.get_or_insert(s))
                        }
                    }
                    None => None,
                };
                (
                    (wf, wt),
                    TkPip {
                        is_buf: pip.is_buf,
                        is_excluded: pip.is_excluded,
                        is_test: pip.is_test,
                        inversion: pip.inv,
                        direction: pip.dir,
                        speed,
                    },
                )
            })
            .collect();
        let mut pip_overrides: HashMap<(WireId, WireId), (NodeClassId, NodeClassId)> =
            HashMap::new();
        for (wf, wt, sid, did) in cpips {
            if w2nc[&wf] != Some(sid) || w2nc[&wt] != Some(did) {
                pip_overrides.insert((wf, wt), (sid, did));
            }
        }
        let wires: Vec<_> = wires
            .iter()
            .map(|&(n, s)| {
                let w = self.part.wires.get_or_insert(n);
                (
                    w,
                    s.map(|x| self.part.speeds.get_or_insert(x)),
                    w2nc.get(&w).copied().unwrap_or(None),
                )
            })
            .collect();
        let slots = self.slotify(sites);
        let sites_raw: Vec<_> = sites
            .iter()
            .map(|&(n, k, ref p)| {
                (
                    slots[n],
                    n,
                    k,
                    p.iter()
                        .map(|pin| {
                            (
                                pin.name,
                                TkSitePin {
                                    dir: pin.dir,
                                    wire: pin.wire.map(|w| self.part.wires.get_or_insert(w)),
                                    speed: pin.speed.map(|x| self.part.speeds.get_or_insert(x)),
                                },
                            )
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect();

        let mut sites: EntityPartVec<TkSiteId, String> = EntityPartVec::new();
        let mut pending_wires: EntityPartVec<TkConnWireId, Option<NodeClassId>> =
            EntityPartVec::new();

        let tki = match self.part.tile_kinds.get_mut(&kind) {
            Some((tki, tk)) => {
                for (slot, name, kind, pins) in sites_raw {
                    let id = match tk.sites.get_mut(&slot) {
                        Some((id, site)) => {
                            for (n, tksp) in pins {
                                let pin = site.pins.get_mut(n).unwrap();
                                if tksp.wire.is_none() {
                                    continue;
                                }
                                if pin.wire.is_some() && pin.wire != tksp.wire {
                                    panic!("pin wire mismatch");
                                }
                                pin.wire = tksp.wire;
                                if pin.speed != tksp.speed {
                                    panic!("pin speed mismatch");
                                }
                                if pin.dir != tksp.dir {
                                    panic!("pin dir mismatch");
                                }
                            }
                            id
                        }
                        None => {
                            tk.sites
                                .insert(
                                    slot,
                                    TkSite {
                                        kind: kind.to_string(),
                                        pins: pins
                                            .iter()
                                            .map(|&(n, tksp)| (n.to_string(), tksp))
                                            .collect(),
                                    },
                                )
                                .0
                        }
                    };
                    sites.insert(id, name.to_string());
                }

                // Process wires.
                let mut wire_set: HashSet<WireId> = HashSet::new();
                for (n, s, nc) in wires {
                    wire_set.insert(n);
                    match tk.wires.get(&n) {
                        None => {
                            let i = tk.conn_wires.push(n);
                            tk.wires.insert(n, TkWire::Connected(i));
                            pending_wires.insert(i, nc);
                        }
                        Some((_, &TkWire::Internal(cs, cnc))) => {
                            if cs != s || cnc != nc {
                                let i = tk.conn_wires.push(n);
                                tk.wires.insert(n, TkWire::Connected(i));
                                for &crd in &tk.tiles {
                                    self.pending_wires.get_mut(&crd).unwrap().insert(i, cnc);
                                }
                                pending_wires.insert(i, nc);
                            }
                        }
                        Some((_, &TkWire::Connected(i))) => {
                            pending_wires.insert(i, nc);
                        }
                    }
                }
                for (_, &k, v) in tk.wires.iter_mut() {
                    if !wire_set.contains(&k) {
                        if let TkWire::Internal(_, cnc) = *v {
                            let i = tk.conn_wires.push(k);
                            *v = TkWire::Connected(i);
                            for &crd in &tk.tiles {
                                self.pending_wires.get_mut(&crd).unwrap().insert(i, cnc);
                            }
                        }
                    }
                }

                // Process pips.
                for (k, pip) in pips {
                    match tk.pips.get(&k) {
                        None => {
                            tk.pips.insert(k, pip);
                        }
                        Some((_, &orig)) => {
                            if orig != pip {
                                panic!(
                                    "pip mismatch {} {} {} {} {:?} {:?}",
                                    name,
                                    kind,
                                    self.part.wires[k.0],
                                    self.part.wires[k.1],
                                    pip,
                                    orig
                                );
                            }
                        }
                    }
                }

                // Add the current tile.
                tk.tiles.push(coord);
                tki
            }
            None => {
                let tk = TileKind {
                    sites: sites_raw
                        .iter()
                        .map(|&(slot, _, kind, ref pins)| {
                            (
                                slot,
                                TkSite {
                                    kind: kind.to_string(),
                                    pins: pins
                                        .iter()
                                        .map(|&(n, tksp)| (n.to_string(), tksp))
                                        .collect(),
                                },
                            )
                        })
                        .collect(),
                    wires: wires
                        .iter()
                        .map(|&(n, s, nc)| (n, TkWire::Internal(s, nc)))
                        .collect(),
                    conn_wires: EntityVec::new(),
                    pips: pips.into_iter().collect(),
                    tiles: vec![coord],
                };
                for (slot, name, _, _) in sites_raw {
                    let id = tk.sites.get(&slot).unwrap().0;
                    sites.insert(id, name.to_string());
                }
                self.part.tile_kinds.insert(kind.to_string(), tk).0
            }
        };
        let tk = &self.part.tile_kinds[tki];
        let pip_overrides = pip_overrides
            .into_iter()
            .map(|(k, v)| (tk.pips.get(&k).unwrap().0, v))
            .collect();
        self.part.tiles.insert(
            coord,
            Tile {
                name: name.clone(),
                kind: tki,
                sites,
                conn_wires: EntityPartVec::new(),
                pip_overrides,
            },
        );
        self.pending_wires.insert(coord, pending_wires);
        self.tiles_by_name.insert(name, coord);
    }

    pub fn ensure_conn_wire(&mut self, tki: TileKindId, wire: WireId) -> TkConnWireId {
        let tk = &mut self.part.tile_kinds[tki];
        let w = tk.wires.get_mut(&wire).unwrap().1;
        match *w {
            TkWire::Internal(s, nc) => {
                let i = tk.conn_wires.push(wire);
                *w = TkWire::Connected(i);
                for crd in &tk.tiles {
                    self.pending_wires.get_mut(crd).unwrap().insert(i, nc);
                }
                self.fixup_nodes_queue.push((tki, wire, s, nc));
                i
            }
            TkWire::Connected(i) => i,
        }
    }

    pub fn kill_wire(&mut self, tile: &str, wire: &str) {
        let crd = self.tiles_by_name[tile];
        let wire = self.part.wires.get(wire).unwrap();
        let idx = self.ensure_conn_wire(self.part.tiles[&crd].kind, wire);
        self.part
            .tiles
            .get_mut(&crd)
            .unwrap()
            .conn_wires
            .remove(idx);
        self.pending_wires.get_mut(&crd).unwrap().remove(idx);
    }

    pub fn add_node(&mut self, wires: &[(&str, &str, Option<&str>)]) {
        let wires: Vec<_> = wires
            .iter()
            .copied()
            .map(|(t, w, s)| {
                (
                    self.tiles_by_name[t],
                    self.part.wires.get_or_insert(w),
                    s.map(|x| self.part.speeds.get_or_insert(x)),
                )
            })
            .collect();
        if wires.len() == 1 {
            let (coord, wire, speed) = wires[0];
            let tile = &self.part.tiles[&coord];
            let tk = &self.part.tile_kinds[tile.kind];
            if let &TkWire::Internal(s, _) = tk.wires.get(&wire).unwrap().1 {
                if s == speed {
                    return;
                }
            }
        }
        let bx = wires.iter().map(|(t, _, _)| t.x).min().unwrap();
        let by = wires.iter().map(|(t, _, _)| t.y).min().unwrap();
        let mut twires: Vec<_> = wires
            .iter()
            .copied()
            .map(|(t, w, s)| TkNodeTemplateWire {
                delta: Coord {
                    x: t.x - bx,
                    y: t.y - by,
                },
                wire: w,
                speed: s,
                cls: {
                    let tile = &self.part.tiles[&t];
                    let tk = &self.part.tile_kinds[tile.kind];
                    match *tk.wires.get(&w).unwrap().1 {
                        TkWire::Internal(_, nc) => nc,
                        TkWire::Connected(idx) => match self.pending_wires[&t].get(idx) {
                            Some(&nc) => nc,
                            _ => None,
                        },
                    }
                },
            })
            .collect();
        twires.sort();
        let tidx = self.part.templates.insert(twires).0;
        let ni = self.part.nodes.push(TkNode {
            base: Coord { x: bx, y: by },
            template: tidx,
        });
        for (coord, wire, _) in wires {
            let tki = self.part.tiles[&coord].kind;
            let idx = self.ensure_conn_wire(tki, wire);
            self.part
                .tiles
                .get_mut(&coord)
                .unwrap()
                .conn_wires
                .insert(idx, ni);
            self.pending_wires.get_mut(&coord).unwrap().remove(idx);
        }
    }

    pub fn add_package(&mut self, name: String, pins: Vec<PkgPin>) {
        self.part.packages.insert(name, pins);
    }
    pub fn add_combo(
        &mut self,
        name: String,
        device: String,
        package: String,
        speed: String,
        temp: String,
    ) {
        self.part.combos.push(PartCombo {
            name,
            device,
            package,
            speed,
            temp,
        });
    }

    pub fn finish(mut self) -> Part {
        for (kind, w, s, nc) in self.fixup_nodes_queue {
            let tk = &self.part.tile_kinds[kind];
            let idx = match *tk.wires.get(&w).unwrap().1 {
                TkWire::Connected(i) => i,
                _ => unreachable!(),
            };
            let mut tidx: Option<TemplateId> = None;
            for &crd in &tk.tiles {
                let t = self.part.tiles.get_mut(&crd).unwrap();
                if self.pending_wires[&crd].contains_id(idx) {
                    let ctidx = match tidx {
                        Some(i) => i,
                        None => {
                            let template = vec![TkNodeTemplateWire {
                                delta: Coord { x: 0, y: 0 },
                                wire: w,
                                speed: s,
                                cls: nc,
                            }];
                            let i = self.part.templates.insert(template).0;
                            tidx = Some(i);
                            i
                        }
                    };
                    let ni = self.part.nodes.push(TkNode {
                        base: crd,
                        template: ctidx,
                    });
                    t.conn_wires.insert(idx, ni);
                }
            }
        }
        self.part
    }
}
