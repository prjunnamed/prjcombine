use std::{collections::btree_map, path::PathBuf};

use clap::Parser;
use prjcombine_ecp::db::Database;
use prjcombine_types::{bsdata::BsData, db::DumpFlags};

#[derive(Debug, Parser)]
struct Args {
    db: PathBuf,
    txt: PathBuf,
    geom: PathBuf,
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
    let geom = prjcombine_re_lattice_naming::Database::from_file(&args.geom).unwrap();
    let tiledb: Vec<_> = args
        .tiledb
        .iter()
        .map(|f| BsData::from_file(f).unwrap())
        .collect();
    let tiledb = merge_tiledb(tiledb);
    let db = Database {
        chips: geom.chips.into_map_values(|(chip, _)| chip),
        bonds: geom.bonds,
        devices: geom.devices,
        int: geom.int,
        bsdata: tiledb,
    };
    db.to_file(&args.db).unwrap();
    db.dump(
        &mut std::fs::File::create(&args.txt).unwrap(),
        DumpFlags::all(),
    )
    .unwrap();
}
