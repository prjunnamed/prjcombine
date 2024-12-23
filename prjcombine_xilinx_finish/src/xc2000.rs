use std::{
    collections::{btree_map, BTreeMap, BTreeSet},
    sync::LazyLock,
};

use itertools::Itertools;
use prjcombine_int::db::{BelInfo, NodeTileId};
use prjcombine_types::tiledb::TileDb;
use prjcombine_xc2000::{
    bond::Bond,
    db::{Database, DeviceCombo, Part},
    grid::Grid,
};
use regex::Regex;
use unnamed_entity::{EntityId, EntityMap, EntitySet, EntityVec};

struct TmpPart<'a> {
    grid: &'a Grid,
    bonds: BTreeMap<&'a str, &'a Bond>,
    speeds: BTreeSet<&'a str>,
    combos: BTreeSet<(&'a str, &'a str)>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SortKey<'a> {
    width: usize,
    height: usize,
    part_kind: PartKind,
    name: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PartKind {
    Xc2000,
    Xc2000L,
    Xc3000,
    Xc3100,
    Xc3000A,
    Xc3000L,
    Xc3100A,
    Xc3100L,
    Xc4000,
    Xc4000D,
    Xc4000A,
    Xc4000H,
    Xc4000E,
    Xc4000L,
    Spartan,
    Xc4000Ex,
    Xc4000Xl,
    Xc4000Xla,
    Xc4000Xv,
    SpartanXl,
    Xc5200,
    Xc5200L,
}

static RE_2000: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc20[0-9]{2}$").unwrap());
static RE_2000L: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc20[0-9]{2}l$").unwrap());

static RE_3000: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc30[0-9]{2}$").unwrap());
static RE_3100: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc31[0-9]{2}$").unwrap());
static RE_3000A: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc30[0-9]{2}a$").unwrap());
static RE_3000L: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc30[0-9]{2}l$").unwrap());
static RE_3100A: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc31[0-9]{2}a$").unwrap());
static RE_3100L: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc31[0-9]{2}l$").unwrap());

static RE_4000: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc40[0-9]{2}$").unwrap());
static RE_4000D: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc40[0-9]{2}d$").unwrap());
static RE_4000A: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc40[0-9]{2}a$").unwrap());
static RE_4000H: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc40[0-9]{2}h$").unwrap());
static RE_4000E: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc40[0-9]{2}e$").unwrap());
static RE_4000L: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc40[0-9]{2}l$").unwrap());
static RE_SPARTAN: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xcs[0-9]{2}$").unwrap());
static RE_4000EX: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc40[0-9]{2}ex$").unwrap());
static RE_4000XL: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc40[0-9]{2}xl$").unwrap());
static RE_4000XLA: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc40[0-9]{2}xla$").unwrap());
static RE_4000XV: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc40[0-9]{3}xv$").unwrap());
static RE_SPARTANXL: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xcs[0-9]{2}xl$").unwrap());

static RE_5200: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc52[0-9]{2}$").unwrap());
static RE_5200L: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc52[0-9]{2}l$").unwrap());

fn sort_key<'a>(name: &'a str, grid: &'a Grid) -> SortKey<'a> {
    let part_kind = if RE_2000.is_match(name) {
        PartKind::Xc2000
    } else if RE_2000L.is_match(name) {
        PartKind::Xc2000L
    } else if RE_3000.is_match(name) {
        PartKind::Xc3000
    } else if RE_3100.is_match(name) {
        PartKind::Xc3100
    } else if RE_3000A.is_match(name) {
        PartKind::Xc3000A
    } else if RE_3000L.is_match(name) {
        PartKind::Xc3000L
    } else if RE_3100A.is_match(name) {
        PartKind::Xc3100A
    } else if RE_3100L.is_match(name) {
        PartKind::Xc3100L
    } else if RE_4000.is_match(name) {
        PartKind::Xc4000
    } else if RE_4000D.is_match(name) {
        PartKind::Xc4000D
    } else if RE_4000A.is_match(name) {
        PartKind::Xc4000A
    } else if RE_4000H.is_match(name) {
        PartKind::Xc4000H
    } else if RE_4000E.is_match(name) {
        PartKind::Xc4000E
    } else if RE_4000L.is_match(name) {
        PartKind::Xc4000L
    } else if RE_SPARTAN.is_match(name) {
        PartKind::Spartan
    } else if RE_4000EX.is_match(name) {
        PartKind::Xc4000Ex
    } else if RE_4000XL.is_match(name) {
        PartKind::Xc4000Xl
    } else if RE_4000XLA.is_match(name) {
        PartKind::Xc4000Xla
    } else if RE_4000XV.is_match(name) {
        PartKind::Xc4000Xv
    } else if RE_SPARTANXL.is_match(name) {
        PartKind::SpartanXl
    } else if RE_5200.is_match(name) {
        PartKind::Xc5200
    } else if RE_5200L.is_match(name) {
        PartKind::Xc5200L
    } else {
        panic!("ummm {name}?")
    };
    SortKey {
        width: grid.columns,
        height: grid.rows,
        part_kind,
        name,
    }
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
    for (name, tpart) in tmp_parts
        .into_iter()
        .sorted_by_key(|(name, tpart)| sort_key(name, tpart.grid))
    {
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
