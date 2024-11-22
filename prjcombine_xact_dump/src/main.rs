use std::{
    collections::{btree_map, BTreeMap},
    path::PathBuf,
};

use clap::Parser;
use prjcombine_xact_data::{
    die::Die,
    parts::{get_parts, PartKind},
    pkg::get_pkg,
};
use prjcombine_xact_geom::{Bond, Device, DeviceBond, ExpandedNamedDevice, GeomDb, Grid};

mod extractor;
mod xc2000;
mod xc4000;
mod xc5200;

#[derive(Parser)]
struct Args {
    dst: PathBuf,
    xact: PathBuf,
    family: String,
}

fn main() {
    let args = Args::parse();
    let parts = get_parts(&args.xact);
    let (family, styles) = match &args.family[..] {
        "xc2000" => (PartKind::Xc2000, &["XC2000", "XC2000L"][..]),
        "xc3000" => (PartKind::Xc3000, &["XC3000", "XC3100"][..]),
        "xc3000a" => (PartKind::Xc3000, &["XC3000A", "XC3000L", "XC3100A"][..]),
        "xc4000" => (PartKind::Xc4000, &["XC4000", "XC4000D"][..]),
        "xc4000a" => (PartKind::Xc4000, &["XC4000A"][..]),
        "xc4000h" => (PartKind::Xc4000, &["XC4000H"][..]),
        "xc5200" => (PartKind::Xc5200, &["XC5200"][..]),
        _ => panic!("ummm {}?", args.family),
    };
    let mut db = GeomDb {
        grids: Default::default(),
        bonds: Default::default(),
        devices: Default::default(),
        ints: Default::default(),
        namings: Default::default(),
    };
    let mut die_cache = BTreeMap::new();
    let mut devices = BTreeMap::new();
    for part in &parts {
        if part.kind != family {
            continue;
        }
        let style = &part.kv["STYLE"][0][..];
        if !styles.contains(&style) {
            continue;
        }
        println!("{} {}", part.name, part.package);
        let grid = match die_cache.entry(part.die_file.clone()) {
            btree_map::Entry::Vacant(entry) => {
                let die = Die::parse(&args.xact, &part.die_file);
                let (grid, intdb, ndb) = match family {
                    PartKind::Xc2000 => {
                        let (grid, intdb, ndb) = xc2000::dump_grid(&die);
                        (Grid::Xc2000(grid), intdb, ndb)
                    }
                    PartKind::Xc3000 => todo!(),
                    PartKind::Xc4000 => {
                        let (grid, intdb, ndb) = xc4000::dump_grid(&die);
                        (Grid::Xc4000(grid), intdb, ndb)
                    }
                    PartKind::Xc5200 => {
                        let (grid, intdb, ndb) = xc5200::dump_grid(&die);
                        (Grid::Xc5200(grid), intdb, ndb)
                    }
                    PartKind::Xc7000 => unreachable!(),
                };
                let grid = db.grids.push(grid);
                match db.ints.entry(args.family.clone()) {
                    btree_map::Entry::Vacant(entry) => {
                        entry.insert(intdb);
                    }
                    btree_map::Entry::Occupied(entry) => {
                        assert_eq!(*entry.get(), intdb);
                    }
                }
                match db.namings.entry(args.family.clone()) {
                    btree_map::Entry::Vacant(entry) => {
                        entry.insert(ndb);
                    }
                    btree_map::Entry::Occupied(entry) => {
                        assert_eq!(*entry.get(), ndb);
                    }
                }
                entry.insert(grid);
                grid
            }
            btree_map::Entry::Occupied(entry) => *entry.get(),
        };
        let name = format!("xc{}", part.name);
        let device = devices.entry(name.clone()).or_insert(Device {
            name,
            grid,
            bonds: Default::default(),
        });
        let pkg = get_pkg(&args.xact, &part.pkg_file);
        let edev = db.expand_grid(device);
        let endev = db.name(device, &edev);
        let bond = match family {
            PartKind::Xc2000 => {
                let ExpandedNamedDevice::Xc2000(ref endev) = endev else {
                    unreachable!()
                };
                let (bond, cfg_io) = xc2000::make_bond(endev, &part.package, &pkg);
                let pin_xtl1 = &part.kv["OSCIOB1"][0];
                let pin_xtl2 = &part.kv["OSCIOB2"][0];
                let io_xtl1 = bond.pins[pin_xtl1];
                let io_xtl2 = bond.pins[pin_xtl2];
                assert_eq!(
                    io_xtl1,
                    prjcombine_xc2000::bond::BondPin::Io(endev.grid.io_xtl1())
                );
                assert_eq!(
                    io_xtl2,
                    prjcombine_xc2000::bond::BondPin::Io(endev.grid.io_xtl2())
                );
                if !cfg_io.is_empty() {
                    let Grid::Xc2000(ref mut grid) = db.grids[grid] else {
                        unreachable!()
                    };
                    if grid.cfg_io.is_empty() {
                        grid.cfg_io = cfg_io;
                    } else {
                        assert_eq!(grid.cfg_io, cfg_io);
                    }
                }
                Bond::Xc2000(bond)
            }
            PartKind::Xc3000 => todo!(),
            PartKind::Xc4000 => {
                let ExpandedNamedDevice::Xc4000(ref endev) = endev else {
                    unreachable!()
                };
                let (bond, cfg_io) = xc4000::make_bond(endev, &part.package, &pkg);
                if !cfg_io.is_empty() {
                    let Grid::Xc4000(ref mut grid) = db.grids[grid] else {
                        unreachable!()
                    };
                    if grid.cfg_io.is_empty() {
                        grid.cfg_io = cfg_io;
                    } else {
                        assert_eq!(grid.cfg_io, cfg_io);
                    }
                }
                Bond::Xc4000(bond)
            }
            PartKind::Xc5200 => {
                let ExpandedNamedDevice::Xc5200(ref endev) = endev else {
                    unreachable!()
                };
                let (bond, cfg_io) = xc5200::make_bond(endev, &part.package, &pkg);
                if !cfg_io.is_empty() {
                    let Grid::Xc5200(ref mut grid) = db.grids[grid] else {
                        unreachable!()
                    };
                    if grid.cfg_io.is_empty() {
                        grid.cfg_io = cfg_io;
                    } else {
                        assert_eq!(grid.cfg_io, cfg_io);
                    }
                }
                Bond::Xc5200(bond)
            }
            PartKind::Xc7000 => unreachable!(),
        };
        let bond = 'bond: {
            for (bid, obond) in &db.bonds {
                if *obond == bond {
                    break 'bond bid;
                }
            }
            db.bonds.push(bond)
        };
        device.bonds.push(DeviceBond {
            name: part.package.clone(),
            bond,
        })
    }
    for dev in devices.into_values() {
        db.devices.push(dev);
    }
    db.to_file(args.dst).unwrap();
}
