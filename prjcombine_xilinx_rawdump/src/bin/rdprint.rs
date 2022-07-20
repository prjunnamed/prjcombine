use itertools::Itertools;
use prjcombine_xilinx_rawdump::{
    Coord, Part, TkPipDirection, TkPipInversion, TkSiteSlot, TkWire,
};
use std::error::Error;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "rdprint", about = "Dump rawdump file.")]
struct Opt {
    file: String,
    #[structopt(short, long)]
    package: bool,
    #[structopt(short, long)]
    wires: bool,
    #[structopt(short, long)]
    conns: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let rd = Part::from_file(opt.file)?;
    println!(
        "PART {} {} {:?} {}×{}",
        rd.part, rd.family, rd.source, rd.width, rd.height
    );
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
            println!("PACKAGE {}", pkg);
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
        println!("TT {}", name);
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
                        Some(w) => &rd.wires[w],
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
            for (_, wi, w) in tt.wires.iter().sorted_by_key(|&(_, &wi, _)| &rd.wires[wi]) {
                let wn = &rd.wires[*wi];
                match *w {
                    TkWire::Internal(s, nc) => {
                        println!(
                            "\tWIRE {} {} {}",
                            wn,
                            match s {
                                Some(s) => &rd.speeds[s],
                                None => "[NONE]",
                            },
                            match nc {
                                Some(nc) => &rd.node_classes[nc],
                                None => "[NONE]",
                            }
                        );
                    }
                    TkWire::Connected(_) => {
                        println!("\tWIRE {} [connected]", wn);
                    }
                }
            }
            for (_, &(wfi, wti), pip) in tt
                .pips
                .iter()
                .sorted_by_key(|&(_, &(wfi, wti), _)| (&rd.wires[wti], &rd.wires[wfi]))
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
                    "\tPIP {} {} {} {}",
                    rd.wires[wti],
                    rd.wires[wfi],
                    flags,
                    match pip.speed {
                        Some(s) => &rd.speeds[s],
                        None => "[NONE]",
                    },
                );
            }
        }
    }
    for (coord, tile) in rd.tiles.iter().sorted_by_key(|(coord, _)| *coord) {
        let tk = &rd.tile_kinds[tile.kind];
        println!("TILE {} {} {} {}", coord.x, coord.y, tile.name, rd.tile_kinds.key(tile.kind));
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
                .sorted_by_key(|&(wi, _)| &rd.wires[wi])
            {
                match ni {
                    Some(ni) => {
                        println!("\tWIRE {}:", rd.wires[wi]);
                        let node = &rd.nodes[ni];
                        let tpl = &rd.templates[node.template];
                        for w in tpl {
                            let tc = Coord {
                                x: node.base.x + w.delta.x,
                                y: node.base.y + w.delta.y,
                            };
                            let otile = &rd.tiles[&tc];
                            println!(
                                "\t\t{} {} {} {}",
                                otile.name,
                                rd.wires[w.wire],
                                match w.speed {
                                    Some(s) => &rd.speeds[s],
                                    None => "[NONE]",
                                },
                                match w.cls {
                                    Some(nc) => &rd.node_classes[nc],
                                    None => "[NONE]",
                                }
                            );
                        }
                    }
                    None => {
                        println!("\tWIRE {}: MISSING", rd.wires[wi]);
                    }
                }
            }
        }
        // XXX pips with forced classes?
    }
    Ok(())
}
