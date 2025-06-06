use std::{collections::btree_map, path::PathBuf};

use clap::Parser;
use jzon::JsonValue;
use prjcombine_re_xilinx_geom::Chip;
use prjcombine_types::bsdata::BsData;

mod spartan6;
mod ultrascale;
mod virtex;
mod virtex2;
mod virtex4;
mod xc2000;

#[derive(Debug, Parser)]
struct Args {
    db: PathBuf,
    json: PathBuf,
    #[arg(long)]
    xact: Option<PathBuf>,
    #[arg(long)]
    geom: Option<PathBuf>,
    tiledb: Vec<PathBuf>,
}

fn merge_tiledb(tiledb: Vec<BsData>) -> BsData {
    let mut res = BsData::new();
    for db in tiledb {
        for (tile, tile_data) in db.tiles {
            let tile_dst = res.tiles.entry(tile).or_default();
            for (key, item) in tile_data.items {
                match tile_dst.items.entry(key) {
                    btree_map::Entry::Vacant(entry) => {
                        entry.insert(item);
                    }
                    btree_map::Entry::Occupied(entry) => {
                        // could make a little smarter?
                        assert_eq!(item, *entry.get());
                    }
                }
            }
        }
        for (device, data) in db.device_data {
            for (key, val) in data {
                res.insert_device_data(&device, key, val);
            }
        }
        for (key, val) in db.misc_data {
            res.insert_misc_data(key, val);
        }
    }
    res
}

fn main() {
    let args = Args::parse();
    if args.xact.is_none() && args.geom.is_none() {
        panic!("no geometry given");
    }
    let xact = args
        .xact
        .map(|f| prjcombine_re_xilinx_xact_geom::GeomDb::from_file(&f).unwrap());
    let geom = args
        .geom
        .map(|f| prjcombine_re_xilinx_geom::GeomDb::from_file(&f).unwrap());
    let tiledb: Vec<_> = args
        .tiledb
        .iter()
        .map(|f| BsData::from_file(f).unwrap())
        .collect();
    let tiledb = merge_tiledb(tiledb);
    if let Some(geom) = geom {
        let chip = geom.chips.first().unwrap();
        match chip {
            Chip::Xc2000(_) => {
                let db = xc2000::finish(xact, Some(geom), tiledb);
                db.to_file(&args.db).unwrap();
                std::fs::write(args.json, JsonValue::from(&db).to_string()).unwrap();
            }
            Chip::Virtex(_) => {
                let db = virtex::finish(geom, tiledb);
                db.to_file(&args.db).unwrap();
                std::fs::write(args.json, JsonValue::from(&db).to_string()).unwrap();
            }
            Chip::Virtex2(_) => {
                let db = virtex2::finish(geom, tiledb);
                db.to_file(&args.db).unwrap();
                std::fs::write(args.json, JsonValue::from(&db).to_string()).unwrap();
            }
            Chip::Spartan6(_) => {
                let db = spartan6::finish(geom, tiledb);
                db.to_file(&args.db).unwrap();
                std::fs::write(args.json, JsonValue::from(&db).to_string()).unwrap();
            }
            Chip::Virtex4(_) => {
                let db = virtex4::finish(geom, tiledb);
                db.to_file(&args.db).unwrap();
                std::fs::write(args.json, JsonValue::from(&db).to_string()).unwrap();
            }
            Chip::Ultrascale(_) => {
                let db = ultrascale::finish(geom, tiledb);
                db.to_file(&args.db).unwrap();
                std::fs::write(args.json, JsonValue::from(&db).to_string()).unwrap();
            }
            Chip::Versal(_) => todo!(),
        }
    } else {
        let db = xc2000::finish(xact, None, tiledb);
        db.to_file(&args.db).unwrap();
        std::fs::write(args.json, JsonValue::from(&db).to_string()).unwrap();
    }
}
