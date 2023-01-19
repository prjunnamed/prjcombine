use itertools::Itertools;
use prjcombine_entity::EntityPartVec;
use prjcombine_rawdump::{Coord, Part, TkPipDirection, TkPipInversion, TkSiteSlot, TkWire};
use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "rdprint", about = "Dump rawdump file.")]
struct Opt {
    file: PathBuf,
    #[structopt(short, long)]
    package: bool,
    #[structopt(short, long)]
    wires: bool,
    #[structopt(short, long)]
    conns: bool,
    #[structopt(short, long)]
    kinds: Vec<String>,
    #[structopt(long)]
    xlat: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let rd = Part::from_file(opt.file)?;
    println!(
        "PART {} {} {:?} {}×{}",
        rd.part, rd.family, rd.source, rd.width, rd.height
    );
    let mut xlat = EntityPartVec::new();
    if let Some(xf) = opt.xlat {
        let f = File::open(xf)?;
        let br = BufReader::new(f);
        for l in br.lines() {
            let l = l?;
            let l = l.trim();
            if l.is_empty() || l.starts_with('#') {
                continue;
            }
            let [k, v] = *l.split_ascii_whitespace().collect::<Vec<_>>() else {
                panic!("weird line {l}");
            };
            if let Some(w) = rd.wires.get(k) {
                xlat.insert(w, v.to_string());
            }
        }
    }
    let wire_name = |wi| -> &str { xlat.get(wi).unwrap_or(&rd.wires[wi]) };
    println!(
        "STAT {} {} {} {}",
        rd.tiles.len(),
        rd.tile_kinds.len(),
        rd.templates.len(),
        rd.nodes.len()
    );
    if opt.package {
        for combo in rd.combos.iter().sorted_by_key(|c| &c.name) {
            println!(
                "COMBO {} {} {} {} {}",
                combo.name, combo.device, combo.package, combo.speed, combo.temp
            );
        }
        for (pkg, pins) in rd.packages.iter().sorted_by_key(|(pkg, _)| *pkg) {
            println!("PACKAGE {pkg}");
            for pin in pins
                .iter()
                .sorted_by_key(|pin| (&pin.pad, &pin.func, &pin.pin))
            {
                println!(
                    "\tPIN {} {} {} {} {} {} {} {}",
                    pin.pad.as_ref().unwrap_or(&"[none]".to_string()),
                    pin.pin,
                    pin.vref_bank
                        .map_or("[none]".to_string(), |bank| bank.to_string()),
                    pin.vcco_bank
                        .map_or("[none]".to_string(), |bank| bank.to_string()),
                    pin.func,
                    pin.tracelen_um
                        .map_or("[none]".to_string(), |x| x.to_string()),
                    pin.delay_min_fs
                        .map_or("[none]".to_string(), |x| x.to_string()),
                    pin.delay_max_fs
                        .map_or("[none]".to_string(), |x| x.to_string()),
                );
            }
        }
    }
    for (_, name, tt) in rd.tile_kinds.iter().sorted_by_key(|(_, name, _)| *name) {
        if !opt.kinds.is_empty() && !opt.kinds.contains(name) {
            continue;
        }
        println!("TT {name}");
        for (_, &slot, site) in tt.sites.iter().sorted_by_key(|&(_, slot, _)| slot) {
            let slot = match slot {
                TkSiteSlot::Single(sk) => rd.slot_kinds[sk].clone(),
                TkSiteSlot::Indexed(sk, idx) => format!("{}[{}]", rd.slot_kinds[sk], idx),
                TkSiteSlot::Xy(sk, x, y) => format!("{}[{},{}]", rd.slot_kinds[sk], x, y),
            };
            println!("\tSITE {} {}", site.kind, slot);
            for (name, pin) in site.pins.iter().sorted_by_key(|(name, _)| *name) {
                println!(
                    "\t\tPIN {} {:?} {} {}",
                    name,
                    pin.dir,
                    match pin.wire {
                        Some(w) => wire_name(w),
                        None => "[NONE]",
                    },
                    match pin.speed {
                        Some(s) => &rd.speeds[s],
                        None => "[NONE]",
                    },
                );
            }
        }
        if opt.wires {
            for (_, &wi, &w) in tt.wires.iter().sorted_by_key(|&(_, &wi, _)| wire_name(wi)) {
                match w {
                    TkWire::Internal(s, nc) => {
                        println!(
                            "\tWIRE {wn} {speed} {nc}",
                            wn = wire_name(wi),
                            speed = match s {
                                Some(s) => &rd.speeds[s],
                                None => "[NONE]",
                            },
                            nc = match nc {
                                Some(nc) => &rd.node_classes[nc],
                                None => "[NONE]",
                            }
                        );
                    }
                    TkWire::Connected(_) => {
                        println!("\tWIRE {wn} [connected]", wn = wire_name(wi));
                    }
                }
            }
            for (_, &(wfi, wti), pip) in tt
                .pips
                .iter()
                .sorted_by_key(|&(_, &(wfi, wti), _)| (wire_name(wti), wire_name(wfi)))
            {
                let mut flags = String::new();
                flags.push(if pip.is_buf { 'B' } else { '-' });
                flags.push(if pip.is_excluded { 'E' } else { '-' });
                flags.push(if pip.is_test { 'T' } else { '-' });
                flags.push(match pip.inversion {
                    TkPipInversion::Never => '-',
                    TkPipInversion::Always => 'I',
                    TkPipInversion::Prog => 'i',
                });
                flags.push(match pip.direction {
                    TkPipDirection::Uni => '-',
                    TkPipDirection::BiFwd => '>',
                    TkPipDirection::BiBwd => '<',
                });
                println!(
                    "\tPIP {wtn} {wfn} {flags} {speed}",
                    wtn = wire_name(wti),
                    wfn = wire_name(wfi),
                    speed = match pip.speed {
                        Some(s) => &rd.speeds[s],
                        None => "[NONE]",
                    },
                );
            }
        }
    }
    for (coord, tile) in rd.tiles.iter().sorted_by_key(|(coord, _)| *coord) {
        let tk = &rd.tile_kinds[tile.kind];
        if !opt.kinds.is_empty() && !opt.kinds.contains(rd.tile_kinds.key(tile.kind)) {
            continue;
        }
        println!(
            "TILE {} {} {} {}",
            coord.x,
            coord.y,
            tile.name,
            rd.tile_kinds.key(tile.kind)
        );
        for (slot, ts) in tk
            .sites
            .iter()
            .map(|(i, &slot, _)| (slot, tile.sites.get(i)))
            .sorted_by_key(|&(slot, _)| slot)
        {
            let slot = match slot {
                TkSiteSlot::Single(sk) => rd.slot_kinds[sk].clone(),
                TkSiteSlot::Indexed(sk, idx) => format!("{}[{}]", rd.slot_kinds[sk], idx),
                TkSiteSlot::Xy(sk, x, y) => format!("{}[{},{}]", rd.slot_kinds[sk], x, y),
            };
            println!(
                "\tSITE {} {}",
                slot,
                ts.as_ref().map_or("[none]".to_string(), |x| x.to_string())
            );
        }
        if opt.conns {
            for (wi, ni) in tk
                .conn_wires
                .iter()
                .map(|(i, &wi)| (wi, tile.conn_wires.get(i).copied()))
                .sorted_by_key(|&(wi, _)| wire_name(wi))
            {
                match ni {
                    Some(ni) => {
                        println!("\tWIRE {wn}:", wn = wire_name(wi));
                        let node = &rd.nodes[ni];
                        let tpl = &rd.templates[node.template];
                        for w in tpl {
                            let tc = Coord {
                                x: node.base.x + w.delta.x,
                                y: node.base.y + w.delta.y,
                            };
                            let otile = &rd.tiles[&tc];
                            println!(
                                "\t\t{tn} {wn} {speed} {nc}",
                                tn = otile.name,
                                wn = wire_name(w.wire),
                                speed = match w.speed {
                                    Some(s) => &rd.speeds[s],
                                    None => "[NONE]",
                                },
                                nc = match w.cls {
                                    Some(nc) => &rd.node_classes[nc],
                                    None => "[NONE]",
                                }
                            );
                        }
                    }
                    None => {
                        println!("\tWIRE {}: MISSING", wire_name(wi));
                    }
                }
            }
        }
        // XXX pips with forced classes?
    }
    Ok(())
}
