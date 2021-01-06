use std::collections::{HashSet, HashMap};
use crate::xilinx::rawdump::{PartBuilder, Part, Source, Coord, TkPipInversion};
use crate::xilinx::toolchain::Toolchain;
use crate::error::Error;
use crate::stringpool::StringPool;
use super::xdlrc::{Parser, Options, PipKind, Tile, Wire};
use super::partgen::PartgenPkg;

fn is_buf_speed(speed: &Option<String>) -> bool {
    match speed {
        None => false,
        Some(s) => if s.starts_with("B_") {
            true
        } else if s.starts_with("BSW_") {
            true
        } else if s.starts_with("R_") {
            false
        } else if s.starts_with("D_") {
            // umm.
            false
        } else if s.starts_with("CCMA1D8_") {
            // umm.
            false
        } else {
            panic!("funny speed {}", s);
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

impl Nodes {
    fn finish_node(&mut self, rd: &mut PartBuilder, sp: &mut StringPool, idx: usize) {
        let node = &mut self.nodes[idx];
        if node.seen.is_empty() && node.unseen.is_empty() {
            return;
        }
        let mut wires: Vec<(&str, &str, Option<&str>)> = Vec::new();
        for (t, w) in node.unseen.iter() {
            wires.push((sp.get(*t), sp.get(*w), None));
            self.wire2node.remove(&(*t, *w));
        }
        for ((t, w), s) in node.seen.iter() {
            wires.push((sp.get(*t), sp.get(*w), match *s {
                None => None,
                Some(s) => Some(sp.get(s)),
            }));
            self.wire2node.remove(&(*t, *w));
        }
        rd.add_node(&wires);
        self.free_node(idx);
    }
    fn finish_all(&mut self, rd: &mut PartBuilder, sp: &mut StringPool) {
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
    fn process_wire(&mut self, rd: &mut PartBuilder, sp: &mut StringPool, t: &Tile, w: &Wire) {
        let mut wnodes: HashSet<usize> = HashSet::new();
        let mut wwires: HashSet<(u32, u32)> = HashSet::new();
        wwires.insert((sp.put(&t.name), sp.put(&w.name)));
        for (t, w) in &w.conns {
            wwires.insert((sp.put(t), sp.put(w)));
        }
        for k in wwires.iter() {
            if let Some(i) = self.wire2node.get(k) {
                wnodes.insert(*i);
            }
        }
        let mut wnodes : Vec<_> = wnodes.into_iter().collect();
        let mut nseen: HashMap<(u32, u32), Option<u32>> = HashMap::new();
        nseen.insert((sp.put(&t.name), sp.put(&w.name)), match &w.speed {
            None => None,
            Some(s) => Some(sp.put(s)),
        });
        let nidx = match wnodes.pop() {
            None => match self.freelist.pop() {
                Some(i) => i,
                None => {
                    let i = self.nodes.len();
                    self.nodes.push(NodeInfo { seen: HashMap::new(), unseen: HashSet::new() });
                    i
                }
            },
            Some(i) => {
                for oi in wnodes {
                    let node = &mut self.nodes[oi];
                    for w in node.unseen.iter() {
                        wwires.insert(*w);
                    }
                    for (w, s) in node.seen.iter() {
                        nseen.insert(*w, *s);
                    }
                    self.free_node(oi);
                }
                i
            }
        };
        let node = &mut self.nodes[nidx];
        for k in wwires.iter() {
            if !node.seen.contains_key(k) && !nseen.contains_key(k) {
                self.wire2node.insert(*k, nidx);
                node.unseen.insert(*k);
            }
        }
        for (k, v) in nseen.iter() {
            if node.unseen.contains(k) {
                node.unseen.remove(k);
            }
            self.wire2node.insert(*k, nidx);
            node.seen.insert(*k, *v);
        }
        if node.unseen.is_empty() {
            self.finish_node(rd, sp, nidx);
        }
    }
}


pub fn get_rawdump(tc: &Toolchain, pkgs: &[PartgenPkg]) -> Result<Part, Error> {
    let part = &pkgs[0];
    let device = &part.device;
    let partname = part.device.clone() + &part.package;
    let pinmap: HashMap<String, String> = part.pins.iter()
        .filter_map(|pin| match &pin.pad {
            None => None,
            Some(pad) => Some((pin.pin.to_string(), pad.to_string())),
        })
        .collect();

    let mut sp = StringPool::new();

    let mut pips_non_test: HashSet<(u32, u32, u32)> = HashSet::new();
    let mut pips_non_excl: HashSet<(u32, u32, u32)> = HashSet::new();
    let mut parser = Parser::from_toolchain(tc, Options {
        part: partname.clone(),
        need_pips: true,
        need_conns: false,
        dump_test: false,
        dump_excluded: true,
    })?;
    while let Some(t) = parser.get_tile()? {
        for pip in t.pips {
            pips_non_test.insert((sp.put(&t.name), sp.put(&pip.wire_from), sp.put(&pip.wire_to)));
        }
    }
    let mut parser = Parser::from_toolchain(tc, Options {
        part: partname.clone(),
        need_pips: true,
        need_conns: false,
        dump_test: true,
        dump_excluded: false,
    })?;
    while let Some(t) = parser.get_tile()? {
        for pip in t.pips {
            pips_non_excl.insert((sp.put(&t.name), sp.put(&pip.wire_from), sp.put(&pip.wire_to)));
        }
    }
    let mut parser = Parser::from_toolchain(tc, Options {
        part: partname,
        need_pips: true,
        need_conns: true,
        dump_test: true,
        dump_excluded: true,
    })?;
    let mut rd = PartBuilder::new(part.device.clone(), part.family.clone(), Source::ISE, parser.width() as u16, parser.height() as u16);

    let mut nodes = Nodes {
        nodes: Vec::new(),
        freelist: Vec::new(),
        wire2node: HashMap::new(),
    };

    while let Some(t) = parser.get_tile()? {
        rd.add_tile(
            Coord{x: t.x as u16, y: (parser.height()-t.y-1) as u16},
            t.name.clone(),
            t.kind.clone(),
            &t.prims.iter().map(|p| (
                &pinmap.get(&p.name).unwrap_or(&p.name)[..],
                &p.kind[..],
                p.pinwires.iter().map(|pw| (
                    &pw.name[..],
                    pw.dir,
                    Some(&pw.wire[..])
                )).collect::<Vec<_>>(),
            )).collect::<Vec<_>>(),
            &t.wires.iter().map(|w| (
                &w.name[..],
                match &w.speed {
                    None => None,
                    Some(s) => Some(&s[..]),
                }
            )).collect::<Vec<_>>(),
            &t.pips.iter().filter(|p| p.route_through.is_none()).map(|p| (
                &p.wire_from[..],
                &p.wire_to[..],
                match p.kind {
                    PipKind::BiBuf => true,
                    PipKind::BiUniBuf => true, // hm.
                    PipKind::BiPass => false,
                    PipKind::Uni => is_buf_speed(&p.speed),
                },
                !pips_non_excl.contains(&(sp.put(&t.name), sp.put(&p.wire_from), sp.put(&p.wire_to))),
                !pips_non_test.contains(&(sp.put(&t.name), sp.put(&p.wire_from), sp.put(&p.wire_to))),
                TkPipInversion::Never,
                match &p.speed {
                    None => None,
                    Some(s) => Some(&s[..]),
                }
            )).collect::<Vec<_>>(),
        );
        for w in t.wires.iter() {
            if w.conns.is_empty() {
                rd.add_node(&[(
                    &t.name,
                    &w.name,
                    match &w.speed {
                        None => None,
                        Some(s) => Some(&s),
                    }
                )]);
            } else {
                nodes.process_wire(&mut rd, &mut sp, &t, w);
            }
        }
    }
    nodes.finish_all(&mut rd, &mut sp);
    for pkg in pkgs {
        assert!(pkg.device == *device);
        rd.add_package(pkg.package.clone(), pkg.speedgrades.clone(), pkg.pins.clone());
    }
    Ok(rd.finish())
}
