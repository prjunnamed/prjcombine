use clap::Parser;
use itertools::Itertools;
use prjcombine_rawdump::{Coord, Part};
use std::{error::Error, path::PathBuf};

#[derive(Debug, Parser)]
#[command(name = "dump_bli", about = "Dump Versal BLI structure from rawdump.")]
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
    for (_, tile) in rd.tiles.iter().sorted_by_key(|&(Coord { x, y }, _)| (y, x)) {
        let tk = &rd.tile_kinds[tile.kind];
        let tkn = rd.tile_kinds.key(tile.kind);
        if !tkn.starts_with("BLI_LS_CORE") && !tkn.starts_with("BLI_LS_MUX_CORE") {
            continue;
        }
        let name = &tile.name;
        println!("BLI {tkn} {name}");
        for (wi, ni) in tk
            .conn_wires
            .iter()
            .map(|(i, &wi)| (wi, tile.conn_wires.get(i).copied()))
            .sorted_by_key(|&(wi, _)| &rd.wires[wi])
        {
            let Some(ni) = ni else {continue;};
            let mut conns = vec![];
            let node = &rd.nodes[ni];
            let tpl = &rd.templates[node.template];
            for w in tpl {
                let tc = Coord {
                    x: node.base.x + w.delta.x,
                    y: node.base.y + w.delta.y,
                };
                let otile = &rd.tiles[&tc];
                let otkn = rd.tile_kinds.key(otile.kind);
                if otkn.starts_with("BLI_")
                    || otkn.starts_with("CLE_BC_")
                    || otkn.starts_with("INTF_")
                {
                    continue;
                }
                conns.push((&otile.name, w.wire));
            }
            if !conns.is_empty() {
                if conns.len() == 1 {
                    println!(
                        "\tWIRE {:35}: {:35} {}",
                        rd.wires[wi], conns[0].0, rd.wires[conns[0].1]
                    );
                } else {
                    println!("\tWIRE {}:", rd.wires[wi]);
                    for (t, w) in conns {
                        println!("\t\t\t\t\t\t  {t:35} {w}", w = rd.wires[w],);
                    }
                }
            }
        }
    }
    Ok(())
}
