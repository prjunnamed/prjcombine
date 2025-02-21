use clap::Parser;
use prjcombine_re_xilinx_rawdump::{Coord, Part, TkWire};
use std::collections::{hash_map, HashMap};
use std::error::Error;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "dump_noc", about = "Dump Versal NOC structure from rawdump.")]
struct Args {
    file: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let rd = Part::from_file(args.file)?;
    println!(
        "PART {} {} {:?} {}×{}",
        rd.part, rd.family, rd.source, rd.width, rd.height
    );
    let mut n2w: HashMap<_, Vec<_>> = HashMap::new();
    let mut pairs = vec![];
    for tkn in ["SLL", "SLL2"] {
        for &crd in rd.tiles_by_kind_name(tkn) {
            let tile = &rd.tiles[&crd];
            let tk = &rd.tile_kinds[tile.kind];
            for wn in ["UBUMP0", "UBUMP1", "UBUMP2", "UBUMP3", "UBUMP4", "UBUMP5"] {
                if rd.wires.get(wn).is_none() {
                    println!("OOPS {tkn} {wn}");
                }
                let wni = rd.wires.get(wn).unwrap();
                if tk.wires.get(&wni).is_none() {
                    println!("OOPS {tkn} {wn}");
                }
                let w = tk.wires.get(&wni).unwrap().1;
                if let TkWire::Connected(cwi) = *w {
                    if let Some(ni) = tile.conn_wires.get(cwi) {
                        match n2w.entry(ni) {
                            hash_map::Entry::Occupied(mut entry) => {
                                let list = entry.get_mut();
                                assert_eq!(list.len(), 1);
                                pairs.push((Some(list[0]), Some((crd, wn))));
                                pairs.push((Some((crd, wn)), Some(list[0])));
                                list.push((crd, wn));
                            }
                            hash_map::Entry::Vacant(entry) => {
                                entry.insert(vec![(crd, wn)]);
                            }
                        }
                    }
                }
            }
        }
    }
    for list in n2w.values() {
        if list.len() == 1 {
            pairs.push((Some(list[0]), None));
        }
    }
    pairs.sort_by_key(|&(pi, po)| {
        (
            pi.map(|(Coord { x, y }, w)| (y, x, w)),
            po.map(|(Coord { x, y }, w)| (y, x, w)),
        )
    });
    for (pi, po) in pairs {
        match pi {
            None => {
                print!("[NONE]                                                             <= ")
            }
            Some((ci, wi)) => print!("{ti:17} {wi:6} <=> ", ti = rd.tiles[&ci].name),
        }
        match po {
            None => println!("[NONE]"),
            Some((co, wo)) => println!("{to:17} {wo}", to = rd.tiles[&co].name),
        }
    }
    Ok(())
}
