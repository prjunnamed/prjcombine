use std::{
    collections::{BTreeMap, BTreeSet, btree_map},
    sync::LazyLock,
};

use itertools::Itertools;
use prjcombine_re_xilinx_geom::GeomDb;
use prjcombine_spartan6::{
    bond::Bond,
    chip::{Chip, DisabledPart},
    db::{Database, Device},
};
use prjcombine_types::{bsdata::BsData, db::DeviceCombo};
use regex::Regex;
use prjcombine_entity::{EntityMap, EntitySet, EntityVec};

struct TmpPart<'a> {
    chip: &'a Chip,
    bonds: BTreeMap<&'a str, &'a Bond>,
    speeds: BTreeSet<&'a str>,
    combos: BTreeSet<(&'a str, &'a str)>,
    disabled: BTreeSet<DisabledPart>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PartKind {
    Spartan6T,
    ASpartan6T,
    QSpartan6T,
    Spartan6,
    ASpartan6,
    QSpartan6,
    Spartan6L,
    QSpartan6L,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SortKey<'a> {
    height: usize,
    width: usize,
    has_disabled_clb: bool,
    part_kind: PartKind,
    name: &'a str,
}

static RE_SPARTAN6: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc6slx[0-9]+$").unwrap());
static RE_ASPARTAN6: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xa6slx[0-9]+$").unwrap());
static RE_QSPARTAN6: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xq6slx[0-9]+$").unwrap());
static RE_SPARTAN6L: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc6slx[0-9]+l$").unwrap());
static RE_QSPARTAN6L: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xq6slx[0-9]+l$").unwrap());
static RE_SPARTAN6T: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc6slx[0-9]+t$").unwrap());
static RE_ASPARTAN6T: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xa6slx[0-9]+t$").unwrap());
static RE_QSPARTAN6T: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xq6slx[0-9]+t$").unwrap());

fn sort_key<'a>(name: &'a str, tpart: &TmpPart, chip: &Chip) -> SortKey<'a> {
    let part_kind = if RE_SPARTAN6.is_match(name) {
        PartKind::Spartan6
    } else if RE_ASPARTAN6.is_match(name) {
        PartKind::ASpartan6
    } else if RE_QSPARTAN6.is_match(name) {
        PartKind::QSpartan6
    } else if RE_SPARTAN6L.is_match(name) {
        PartKind::Spartan6L
    } else if RE_QSPARTAN6L.is_match(name) {
        PartKind::QSpartan6L
    } else if RE_SPARTAN6T.is_match(name) {
        PartKind::Spartan6T
    } else if RE_ASPARTAN6T.is_match(name) {
        PartKind::ASpartan6T
    } else if RE_QSPARTAN6T.is_match(name) {
        PartKind::QSpartan6T
    } else {
        panic!("ummm {name}?")
    };
    SortKey {
        width: chip.columns.len(),
        height: chip.rows.len(),
        has_disabled_clb: tpart
            .disabled
            .iter()
            .any(|x| matches!(x, DisabledPart::ClbColumn(_))),
        part_kind,
        name,
    }
}

pub fn finish(geom: GeomDb, tiledb: BsData) -> Database {
    let mut tmp_parts: BTreeMap<&str, _> = BTreeMap::new();
    for dev in &geom.devices {
        let prjcombine_re_xilinx_geom::Chip::Spartan6(ref chip) =
            geom.chips[*dev.chips.first().unwrap()]
        else {
            unreachable!()
        };
        let disabled: BTreeSet<_> = dev
            .disabled
            .iter()
            .map(|&dis| {
                let prjcombine_re_xilinx_geom::DisabledPart::Spartan6(dis) = dis else {
                    unreachable!()
                };
                dis
            })
            .collect();
        let tpart = tmp_parts.entry(&dev.name).or_insert_with(|| TmpPart {
            chip,
            disabled: disabled.clone(),
            bonds: Default::default(),
            speeds: Default::default(),
            combos: Default::default(),
        });
        assert_eq!(tpart.chip, chip);
        assert_eq!(tpart.disabled, disabled);
        for devbond in dev.bonds.values() {
            let prjcombine_re_xilinx_geom::Bond::Spartan6(ref bond) = geom.bonds[devbond.bond]
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
        .sorted_by_key(|(name, tpart)| sort_key(name, tpart, tpart.chip))
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
        let part = Device {
            name: name.into(),
            chip,
            bonds: dev_bonds,
            speeds,
            combos,
            disabled: tpart.disabled,
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
        devices: parts,
        int,
        bsdata: tiledb,
    }
}
