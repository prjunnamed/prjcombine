use std::{
    collections::{BTreeMap, BTreeSet, btree_map},
    sync::LazyLock,
};

use itertools::Itertools;
use prjcombine_re_xilinx_geom::GeomDb;
use prjcombine_types::bsdata::BsData;
use prjcombine_virtex2::{
    bond::Bond,
    chip::{Chip, ChipKind},
    db::{Database, DeviceCombo, Part},
};
use regex::Regex;
use unnamed_entity::{EntityMap, EntitySet, EntityVec};

struct TmpPart<'a> {
    chip: &'a Chip,
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
    kind: ChipKind,
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

fn sort_key<'a>(name: &'a str, chip: &'a Chip) -> SortKey<'a> {
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
        kind: chip.kind,
        width: chip.columns.len(),
        height: chip.rows.len(),
        part_kind,
        name,
    }
}

pub fn finish(geom: GeomDb, tiledb: BsData) -> Database {
    let mut tmp_parts: BTreeMap<&str, _> = BTreeMap::new();
    for dev in &geom.devices {
        let prjcombine_re_xilinx_geom::Chip::Virtex2(ref chip) =
            geom.chips[*dev.chips.first().unwrap()]
        else {
            unreachable!()
        };
        let tpart = tmp_parts.entry(&dev.name).or_insert_with(|| TmpPart {
            chip,
            bonds: Default::default(),
            speeds: Default::default(),
            combos: Default::default(),
        });
        assert_eq!(tpart.chip, chip);
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
    let mut chips = EntitySet::new();
    let mut bonds = EntitySet::new();
    let mut parts = vec![];
    for (name, tpart) in tmp_parts
        .into_iter()
        .sorted_by_key(|(name, tpart)| sort_key(name, tpart.chip))
    {
        let chip = chips.insert(tpart.chip.clone()).0;
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
            chip,
            bonds: dev_bonds,
            speeds,
            combos,
        };
        parts.push(part);
    }
    let chips = chips.into_vec();
    let bonds = bonds.into_vec();

    assert_eq!(geom.ints.len(), 1);
    let int = geom.ints.into_values().next().unwrap();

    // TODO: resort int

    Database {
        chips,
        bonds,
        parts,
        int,
        bsdata: tiledb,
    }
}
