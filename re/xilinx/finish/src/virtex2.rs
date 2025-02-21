use std::{
    collections::{BTreeMap, BTreeSet, btree_map},
    sync::LazyLock,
};

use itertools::Itertools;
use prjcombine_re_xilinx_geom::GeomDb;
use prjcombine_types::tiledb::TileDb;
use prjcombine_virtex2::{
    bond::Bond,
    db::{Database, DeviceCombo, Part},
    grid::{Grid, GridKind},
};
use regex::Regex;
use unnamed_entity::{EntityMap, EntitySet, EntityVec};

struct TmpPart<'a> {
    grid: &'a Grid,
    bonds: BTreeMap<&'a str, &'a Bond>,
    speeds: BTreeSet<&'a str>,
    combos: BTreeSet<(&'a str, &'a str)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PartKind {
    Virtex2,
    QVirtex2,
    QRVirtex2,
    Spartan3,
    ASpartan3,
    Spartan3L,
    Spartan3N,
    FpgaCore,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SortKey<'a> {
    kind: GridKind,
    width: usize,
    height: usize,
    part_kind: PartKind,
    name: &'a str,
}

static RE_VIRTEX2: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc2v(|p|px)[0-9]+$").unwrap());
static RE_QVIRTEX2: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xq2v(|p|px)[0-9]+$").unwrap());
static RE_QRVIRTEX2: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^xqr2v(|p|px)[0-9]+$").unwrap());
static RE_SPARTAN3: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc3sd?[0-9]+(|e|a)$").unwrap());
static RE_ASPARTAN3: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^xa3sd?[0-9]+(|e|a)$").unwrap());
static RE_SPARTAN3L: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc3s[0-9]+l$").unwrap());
static RE_SPARTAN3N: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc3s[0-9]+an$").unwrap());
static RE_FPGACORE: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xcexf[0-9]+$").unwrap());

fn sort_key<'a>(name: &'a str, grid: &'a Grid) -> SortKey<'a> {
    let part_kind = if RE_VIRTEX2.is_match(name) {
        PartKind::Virtex2
    } else if RE_QVIRTEX2.is_match(name) {
        PartKind::QVirtex2
    } else if RE_QRVIRTEX2.is_match(name) {
        PartKind::QRVirtex2
    } else if RE_SPARTAN3.is_match(name) {
        PartKind::Spartan3
    } else if RE_ASPARTAN3.is_match(name) {
        PartKind::ASpartan3
    } else if RE_SPARTAN3L.is_match(name) {
        PartKind::Spartan3L
    } else if RE_SPARTAN3N.is_match(name) {
        PartKind::Spartan3N
    } else if RE_FPGACORE.is_match(name) {
        PartKind::FpgaCore
    } else {
        panic!("ummm {name}?")
    };
    SortKey {
        kind: grid.kind,
        width: grid.columns.len(),
        height: grid.rows.len(),
        part_kind,
        name,
    }
}

pub fn finish(geom: GeomDb, tiledb: TileDb) -> Database {
    let mut tmp_parts: BTreeMap<&str, _> = BTreeMap::new();
    for dev in &geom.devices {
        let prjcombine_re_xilinx_geom::Grid::Virtex2(ref grid) =
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
            let prjcombine_re_xilinx_geom::Bond::Virtex2(ref bond) = geom.bonds[devbond.bond]
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
