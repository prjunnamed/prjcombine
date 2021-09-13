use itertools::Itertools;
use prjcombine_xilinx_rawdump::{
    Coord, NodeOrClass, Part, TkPipDirection, TkPipInversion, TkSiteSlot, TkWire,
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
    for (name, tt) in rd.tile_kinds.iter().sorted_by_key(|(name, _)| *name) {
        println!("TT {}", name);
        for site in tt.sites.iter().sorted_by_key(|site| site.slot) {
            let slot = match site.slot {
                TkSiteSlot::Single(sk) => rd.print_slot_kind(sk).to_string(),
                TkSiteSlot::Indexed(sk, idx) => format!("{}[{}]", rd.print_slot_kind(sk), idx),
                TkSiteSlot::Xy(sk, x, y) => format!("{}[{},{}]", rd.print_slot_kind(sk), x, y),
            };
            println!("\tSITE {} {}", site.kind, slot);
            for (name, pin) in site.pins.iter().sorted_by_key(|(name, _)| *name) {
                println!(
                    "\t\tPIN {} {:?} {} {}",
                    name,
                    pin.dir,
                    rd.print_wire(pin.wire),
                    rd.print_speed(pin.speed)
                );
            }
        }
        if opt.wires {
            for (wi, w) in tt.wires.iter().sorted_by_key(|(wi, _)| rd.print_wire(**wi)) {
                let wn = rd.print_wire(*wi);
                match *w {
                    TkWire::Internal(s, nc) => {
                        println!(
                            "\tWIRE {} {} {}",
                            wn,
                            rd.print_speed(s),
                            rd.print_node_class(nc)
                        );
                    }
                    TkWire::Connected(_) => {
                        println!("\tWIRE {} [connected]", wn);
                    }
                }
            }
            for ((wfi, wti), pip) in tt
                .pips
                .iter()
                .sorted_by_key(|((wfi, wti), _)| (rd.print_wire(*wti), rd.print_wire(*wfi)))
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
                let speed = rd.print_speed(pip.speed);
                println!(
                    "\tPIP {} {} {} {}",
                    rd.print_wire(*wti),
                    rd.print_wire(*wfi),
                    flags,
                    speed
                );
            }
        }
    }
    for (coord, tile) in rd.tiles.iter().sorted_by_key(|(coord, _)| *coord) {
        println!("TILE {} {} {} {}", coord.x, coord.y, tile.name, tile.kind);
        let tt = rd.tile_kinds.get(&tile.kind).unwrap();
        for (ts, tks) in tile
            .sites
            .iter()
            .zip(tt.sites.iter())
            .sorted_by_key(|(_, tks)| tks.slot)
        {
            let slot = match tks.slot {
                TkSiteSlot::Single(sk) => rd.print_slot_kind(sk).to_string(),
                TkSiteSlot::Indexed(sk, idx) => format!("{}[{}]", rd.print_slot_kind(sk), idx),
                TkSiteSlot::Xy(sk, x, y) => format!("{}[{},{}]", rd.print_slot_kind(sk), x, y),
            };
            println!(
                "\tSITE {} {}",
                slot,
                ts.as_ref().map_or("[none]".to_string(), |x| x.to_string())
            );
        }
        if opt.conns {
            for (wi, noc) in tt
                .conn_wires
                .iter()
                .copied()
                .zip(tile.conn_wires.iter().copied())
                .sorted_by_key(|(wi, _)| rd.print_wire(*wi))
            {
                match noc {
                    NodeOrClass::Node(ni) => {
                        println!("\tWIRE {}:", rd.print_wire(wi));
                        let node = &rd.nodes[ni as usize];
                        let tpl = &rd.templates[node.template as usize];
                        for w in tpl.wires.iter() {
                            let tc = Coord {
                                x: node.base.x + w.delta.x,
                                y: node.base.y + w.delta.y,
                            };
                            let otile = &rd.tiles[&tc];
                            println!(
                                "\t\t{} {} {} {}",
                                otile.name,
                                rd.print_wire(w.wire),
                                rd.print_speed(w.speed),
                                rd.print_node_class(w.cls)
                            );
                        }
                    }
                    NodeOrClass::None => {
                        println!("\tWIRE {}: MISSING", rd.print_wire(wi));
                    }
                    _ => panic!("WIRE {} PENDING", rd.print_wire(wi)),
                }
            }
        }
        // XXX pips with forced classes?
    }
    Ok(())
}
