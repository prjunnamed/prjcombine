use std::collections::{btree_map, BTreeMap, BTreeSet};

use prjcombine_spartan6::{
    bond::Bond,
    db::{Database, DeviceCombo, Part},
    grid::{DisabledPart, Grid},
};
use prjcombine_types::tiledb::TileDb;
use prjcombine_xilinx_geom::GeomDb;
use unnamed_entity::{EntityMap, EntitySet, EntityVec};

struct TmpPart<'a> {
    grid: &'a Grid,
    bonds: BTreeMap<&'a str, &'a Bond>,
    speeds: BTreeSet<&'a str>,
    combos: BTreeSet<(&'a str, &'a str)>,
    disabled: BTreeSet<DisabledPart>,
}

pub fn finish(geom: GeomDb, tiledb: TileDb) -> Database {
    let mut tmp_parts: BTreeMap<&str, _> = BTreeMap::new();
    for dev in &geom.devices {
        let prjcombine_xilinx_geom::Grid::Spartan6(ref grid) =
            geom.grids[*dev.grids.first().unwrap()]
        else {
            unreachable!()
        };
        let disabled: BTreeSet<_> = dev
            .disabled
            .iter()
            .map(|&dis| {
                let prjcombine_xilinx_geom::DisabledPart::Spartan6(dis) = dis else {
                    unreachable!()
                };
                dis
            })
            .collect();
        let tpart = tmp_parts.entry(&dev.name).or_insert_with(|| TmpPart {
            grid,
            disabled: disabled.clone(),
            bonds: Default::default(),
            speeds: Default::default(),
            combos: Default::default(),
        });
        assert_eq!(tpart.grid, grid);
        assert_eq!(tpart.disabled, disabled);
        for devbond in dev.bonds.values() {
            let prjcombine_xilinx_geom::Bond::Spartan6(ref bond) = geom.bonds[devbond.bond] else {
                unreachable!()
            };
            match tpart.bonds.entry(&devbond.name) {
                btree_map::Entry::Vacant(entry) => {
                    entry.insert(bond);
                }
                btree_map::Entry::Occupied(entry) => {
                    assert_eq!(*entry.get(), bond);
                }
            }
        }
        for speed in dev.speeds.values() {
            tpart.speeds.insert(speed);
        }
        for combo in &dev.combos {
            tpart.combos.insert((
                &dev.bonds[combo.devbond_idx].name,
                &dev.speeds[combo.speed_idx],
            ));
        }
    }
    let mut grids = EntitySet::new();
    let mut bonds = EntitySet::new();
    let mut parts = vec![];
    for (name, tpart) in tmp_parts {
        let grid = grids.insert(tpart.grid.clone()).0;
        let mut dev_bonds = EntityMap::new();
        for (bname, bond) in tpart.bonds {
            let bond = bonds.insert(bond.clone()).0;
            dev_bonds.insert(bname.to_string(), bond);
        }
        let mut speeds = EntitySet::new();
        for speed in tpart.speeds {
            speeds.insert(speed.to_string());
        }
        let mut combos = vec![];
        for combo in tpart.combos {
            combos.push(DeviceCombo {
                devbond: dev_bonds.get(combo.0).unwrap().0,
                speed: speeds.get(combo.1).unwrap(),
            });
        }
        let speeds = EntityVec::from_iter(speeds.into_values());
        let part = Part {
            name: name.into(),
            grid,
            bonds: dev_bonds,
            speeds,
            combos,
            disabled: tpart.disabled,
        };
        parts.push(part);
    }
    let grids = grids.into_vec();
    let bonds = bonds.into_vec();

    assert_eq!(geom.ints.len(), 1);
    let int = geom.ints.into_values().next().unwrap();

    // TODO: resort int

    Database {
        grids,
        bonds,
        parts,
        int,
        tiles: tiledb,
    }
}