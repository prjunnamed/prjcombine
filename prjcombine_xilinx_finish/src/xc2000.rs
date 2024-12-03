use std::collections::{btree_map, BTreeMap, BTreeSet};

use prjcombine_int::db::{BelInfo, NodeTileId};
use prjcombine_types::tiledb::TileDb;
use prjcombine_xc2000::{
    bond::Bond,
    db::{Database, DeviceCombo, Part},
    grid::Grid,
};
use unnamed_entity::{EntityId, EntityMap, EntitySet, EntityVec};

struct TmpPart<'a> {
    grid: &'a Grid,
    bonds: BTreeMap<&'a str, &'a Bond>,
    speeds: BTreeSet<&'a str>,
    combos: BTreeSet<(&'a str, &'a str)>,
}

pub fn finish(
    xact: Option<prjcombine_xact_geom::GeomDb>,
    geom: Option<prjcombine_xilinx_geom::GeomDb>,
    tiledb: TileDb,
) -> Database {
    let mut tmp_parts: BTreeMap<&str, _> = BTreeMap::new();
    if let Some(ref xact) = xact {
        for dev in &xact.devices {
            let tpart = tmp_parts.entry(&dev.name).or_insert_with(|| TmpPart {
                grid: &xact.grids[dev.grid],
                bonds: Default::default(),
                speeds: Default::default(),
                combos: Default::default(),
            });
            assert_eq!(tpart.grid, &xact.grids[dev.grid]);
            for bond in &dev.bonds {
                match tpart.bonds.entry(&bond.name) {
                    btree_map::Entry::Vacant(entry) => {
                        entry.insert(&xact.bonds[bond.bond]);
                    }
                    btree_map::Entry::Occupied(entry) => {
                        assert_eq!(*entry.get(), &xact.bonds[bond.bond]);
                    }
                }
            }
        }
    }
    if let Some(ref geom) = geom {
        for dev in &geom.devices {
            let prjcombine_xilinx_geom::Grid::Xc2000(ref grid) =
                geom.grids[*dev.grids.first().unwrap()]
            else {
                unreachable!()
            };
            let tpart = tmp_parts.entry(&dev.name).or_insert_with(|| TmpPart {
                grid,
                bonds: Default::default(),
                speeds: Default::default(),
                combos: Default::default(),
            });
            assert_eq!(tpart.grid, grid);
            for devbond in dev.bonds.values() {
                let prjcombine_xilinx_geom::Bond::Xc2000(ref bond) = geom.bonds[devbond.bond]
                else {
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
        };
        parts.push(part);
    }
    let grids = grids.into_vec();
    let bonds = bonds.into_vec();

    let int = match (xact, geom) {
        (Some(xact), None) => {
            assert_eq!(xact.ints.len(), 1);
            xact.ints.into_values().next().unwrap()
        }
        (None, Some(geom)) => {
            assert_eq!(geom.ints.len(), 1);
            geom.ints.into_values().next().unwrap()
        }
        (Some(xact), Some(geom)) => {
            assert_eq!(xact.ints.len(), 1);
            assert_eq!(geom.ints.len(), 1);
            let (key_x, mut int_x) = xact.ints.into_iter().next().unwrap();
            let (key_i, mut int_i) = geom.ints.into_iter().next().unwrap();
            assert_eq!(key_x, "xc5200");
            assert_eq!(key_i, "xc5200");
            let io_b = int_i.get_node("IO.B");
            let io_b = &mut int_i.nodes[io_b];
            io_b.bels.insert("SCANTEST".into(), BelInfo::default());
            let key = (NodeTileId::from_idx(0), int_i.get_wire("IMUX.BYPOSC.PUMP"));
            let imux_byposc_pump = int_i.nodes.get("CNR.TR").unwrap().1.muxes[&key].clone();
            int_x
                .nodes
                .get_mut("CNR.TR")
                .unwrap()
                .1
                .muxes
                .insert(key, imux_byposc_pump);
            assert_eq!(int_x, int_i);
            int_x
        }
        _ => unreachable!(),
    };

    // TODO: resort int

    Database {
        grids,
        bonds,
        parts,
        int,
        tiles: tiledb,
    }
}
