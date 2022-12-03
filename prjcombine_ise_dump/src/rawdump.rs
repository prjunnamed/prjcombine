use super::partgen::PartgenPkg;
use super::xdlrc::{Options, Parser, PipKind, Tile, Wire};
use indexmap::IndexSet;
use prjcombine_rawdump::{Coord, Part, Source, TkPipDirection, TkPipInversion};
use prjcombine_rdbuild::{PartBuilder, PbPip, PbSitePin};
use prjcombine_toolchain::Toolchain;
use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::error::Error;

fn is_buf_speed(speed: &Option<String>) -> bool {
    match speed {
        None => false,
        Some(s) => {
            if s.starts_with("B_") || s.starts_with("BSW_") {
                true
            } else if s.starts_with("R_") || s.starts_with("D_") || s.starts_with("CCMA1D8_") {
                false
            } else {
                panic!("funny speed {s}");
            }
        }
    }
}

struct NodeInfo {
    unseen: HashSet<(u32, u32)>,
    seen: HashMap<(u32, u32), Option<u32>>,
}

struct Nodes {
    nodes: Vec<NodeInfo>,
    freelist: Vec<usize>,
    wire2node: HashMap<(u32, u32), usize>,
}

fn intern(sp: &mut IndexSet<String>, s: &str) -> u32 {
    match sp.get_index_of(s) {
        Some(res) => res.try_into().unwrap(),
        None => sp.insert_full(s.to_string()).0.try_into().unwrap(),
    }
}

fn unintern(sp: &IndexSet<String>, idx: u32) -> &str {
    sp.get_index(idx as usize).unwrap()
}

impl Nodes {
    fn finish_node(&mut self, rd: &mut PartBuilder, sp: &mut IndexSet<String>, idx: usize) {
        let node = &mut self.nodes[idx];
        if node.seen.is_empty() && node.unseen.is_empty() {
            return;
        }
        let mut wires: Vec<(&str, &str, Option<&str>)> = Vec::new();
        for &(t, w) in node.unseen.iter() {
            wires.push((unintern(sp, t), unintern(sp, w), None));
            self.wire2node.remove(&(t, w));
        }
        for (&(t, w), &s) in node.seen.iter() {
            wires.push((
                unintern(sp, t),
                unintern(sp, w),
                match s {
                    None => None,
                    Some(s) => Some(unintern(sp, s)),
                },
            ));
            self.wire2node.remove(&(t, w));
        }
        rd.add_node(&wires);
        self.free_node(idx);
    }
    fn finish_all(&mut self, rd: &mut PartBuilder, sp: &mut IndexSet<String>) {
        for nidx in 0..self.nodes.len() {
            self.finish_node(rd, sp, nidx);
        }
    }
    fn free_node(&mut self, idx: usize) {
        let node = &mut self.nodes[idx];
        node.seen.clear();
        node.unseen.clear();
        self.freelist.push(idx);
    }
    fn process_wire(
        &mut self,
        rd: &mut PartBuilder,
        sp: &mut IndexSet<String>,
        t: &Tile,
        w: &Wire,
    ) {
        let mut wnodes: HashSet<usize> = HashSet::new();
        let mut wwires: HashSet<(u32, u32)> = HashSet::new();
        wwires.insert((intern(sp, &t.name), intern(sp, &w.name)));
        for (t, w) in &w.conns {
            wwires.insert((intern(sp, t), intern(sp, w)));
        }
        for k in wwires.iter() {
            if let Some(&i) = self.wire2node.get(k) {
                wnodes.insert(i);
            }
        }
        let mut wnodes: Vec<_> = wnodes.into_iter().collect();
        let mut nseen: HashMap<(u32, u32), Option<u32>> = HashMap::new();
        nseen.insert(
            (intern(sp, &t.name), intern(sp, &w.name)),
            w.speed.as_ref().map(|s| intern(sp, s)),
        );
        let nidx = match wnodes.pop() {
            None => match self.freelist.pop() {
                Some(i) => i,
                None => {
                    let i = self.nodes.len();
                    self.nodes.push(NodeInfo {
                        seen: HashMap::new(),
                        unseen: HashSet::new(),
                    });
                    i
                }
            },
            Some(i) => {
                for oi in wnodes {
                    let node = &mut self.nodes[oi];
                    for &w in node.unseen.iter() {
                        wwires.insert(w);
                    }
                    for (&w, &s) in node.seen.iter() {
                        nseen.insert(w, s);
                    }
                    self.free_node(oi);
                }
                i
            }
        };
        let node = &mut self.nodes[nidx];
        for &k in wwires.iter() {
            if !node.seen.contains_key(&k) && !nseen.contains_key(&k) {
                self.wire2node.insert(k, nidx);
                node.unseen.insert(k);
            }
        }
        for (&k, &v) in nseen.iter() {
            if node.unseen.contains(&k) {
                node.unseen.remove(&k);
            }
            self.wire2node.insert(k, nidx);
            node.seen.insert(k, v);
        }
        if node.unseen.is_empty() {
            self.finish_node(rd, sp, nidx);
        }
    }
}

pub fn get_rawdump(tc: &Toolchain, pkgs: &[PartgenPkg]) -> Result<Part, Box<dyn Error>> {
    let part = &pkgs[0];
    let device = &part.device;
    let partname = part.device.clone() + &part.package;
    let mut pinmap: HashMap<String, String> = part
        .pins
        .iter()
        .filter_map(|pin| {
            pin.pad
                .as_ref()
                .map(|pad| (pin.pin.to_string(), pad.to_string()))
        })
        .collect();

    let mut sp = IndexSet::new();

    let mut pips_non_test: HashSet<(u32, u32, u32)> = HashSet::new();
    {
        let mut parser = Parser::from_toolchain(
            tc,
            Options {
                part: partname.clone(),
                need_pips: true,
                need_conns: false,
                dump_test: false,
                dump_excluded: true,
            },
        )?;
        while let Some(t) = parser.get_tile()? {
            for pip in t.pips {
                pips_non_test.insert((
                    intern(&mut sp, &t.kind),
                    intern(&mut sp, &pip.wire_from),
                    intern(&mut sp, &pip.wire_to),
                ));
            }
        }
    }
    let mut pips_non_excl: HashSet<(u32, u32, u32)> = HashSet::new();
    {
        let mut parser = Parser::from_toolchain(
            tc,
            Options {
                part: partname.clone(),
                need_pips: true,
                need_conns: false,
                dump_test: true,
                dump_excluded: false,
            },
        )?;
        while let Some(t) = parser.get_tile()? {
            for pip in t.pips {
                pips_non_excl.insert((
                    intern(&mut sp, &t.kind),
                    intern(&mut sp, &pip.wire_from),
                    intern(&mut sp, &pip.wire_to),
                ));
            }
        }
    }
    let mut parser = Parser::from_toolchain(
        tc,
        Options {
            part: partname,
            need_pips: true,
            need_conns: true,
            dump_test: true,
            dump_excluded: true,
        },
    )?;
    let mut rd = PartBuilder::new(
        part.device.clone(),
        part.family.clone(),
        Source::ISE,
        parser.width() as u16,
        parser.height() as u16,
    );

    let mut nodes = Nodes {
        nodes: Vec::new(),
        freelist: Vec::new(),
        wire2node: HashMap::new(),
    };

    while let Some(t) = parser.get_tile()? {
        if part.family == "xc5200" {
            for p in &t.prims {
                if let Some(suf) = p.name.strip_prefix("UNB") {
                    pinmap.insert(p.name.clone(), format!("PAD{suf}"));
                }
            }
        }
        rd.add_tile(
            Coord {
                x: t.x as u16,
                y: (parser.height() - t.y - 1) as u16,
            },
            t.name.clone(),
            t.kind.clone(),
            &t.prims
                .iter()
                .map(|p| {
                    (
                        &pinmap.get(&p.name).unwrap_or(&p.name)[..],
                        &p.kind[..],
                        p.pinwires
                            .iter()
                            .map(|pw| PbSitePin {
                                name: &pw.name,
                                dir: pw.dir,
                                wire: Some(&pw.wire),
                                speed: None,
                            })
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>(),
            &t.wires
                .iter()
                .filter(|w| w.name != "SWBOX_STUB")
                .map(|w| (&w.name[..], w.speed.as_ref().map(|s| &s[..])))
                .collect::<Vec<_>>(),
            &t.pips
                .iter()
                .filter(|p| {
                    p.route_through.is_none()
                        && p.wire_from != "SWBOX_STUB"
                        && p.wire_to != "SWBOX_STUB"
                })
                .map(|p| PbPip {
                    wire_from: &p.wire_from,
                    wire_to: &p.wire_to,
                    is_buf: match p.kind {
                        PipKind::BiBuf => true,
                        PipKind::BiUniBuf => true, // hm.
                        PipKind::BiPass => false,
                        PipKind::Uni => is_buf_speed(&p.speed),
                    },
                    is_excluded: !pips_non_excl.contains(&(
                        intern(&mut sp, &t.kind),
                        intern(&mut sp, &p.wire_from),
                        intern(&mut sp, &p.wire_to),
                    )),
                    is_test: !pips_non_test.contains(&(
                        intern(&mut sp, &t.kind),
                        intern(&mut sp, &p.wire_from),
                        intern(&mut sp, &p.wire_to),
                    )),
                    inv: TkPipInversion::Never,
                    dir: match p.kind {
                        PipKind::BiBuf => TkPipDirection::BiFwd,
                        PipKind::BiUniBuf => TkPipDirection::BiFwd,
                        PipKind::BiPass => TkPipDirection::BiFwd,
                        PipKind::Uni => TkPipDirection::Uni,
                    },
                    speed: p.speed.as_ref().map(|s| &s[..]),
                })
                .collect::<Vec<_>>(),
        );
        for w in t.wires.iter() {
            if w.name == "SWBOX_STUB" {
                continue;
            }
            if w.conns.is_empty() {
                rd.add_node(&[(&t.name, &w.name, w.speed.as_ref().map(|s| &s[..]))]);
            } else {
                nodes.process_wire(&mut rd, &mut sp, &t, w);
            }
        }
    }
    nodes.finish_all(&mut rd, &mut sp);
    for pkg in pkgs {
        assert_eq!(pkg.device, *device);
        rd.add_package(pkg.package.clone(), pkg.pins.clone());
        for speed in pkg.speedgrades.iter() {
            rd.add_combo(
                pkg.device.clone() + &pkg.package + speed,
                pkg.device.clone(),
                pkg.package.clone(),
                speed.clone(),
                "".to_string(),
            );
        }
    }
    Ok(rd.finish())
}
