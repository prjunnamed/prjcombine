use std::{
    collections::{BTreeMap, BTreeSet, btree_map},
    sync::LazyLock,
};

use itertools::Itertools;
use prjcombine_entity::{EntityMap, EntitySet, EntityVec};
use prjcombine_re_collector::bitdata::CollectorData;
use prjcombine_re_xilinx_geom::GeomDb;
use prjcombine_types::db::DeviceCombo;
use prjcombine_virtex::{
    bond::Bond,
    chip::{Chip, ChipKind, DisabledPart},
    db::{Database, Device},
};
use regex::Regex;

struct TmpPart<'a> {
    chip: &'a Chip,
    bonds: BTreeMap<&'a str, &'a Bond>,
    speeds: BTreeSet<&'a str>,
    combos: BTreeSet<(&'a str, &'a str)>,
    disabled: BTreeSet<DisabledPart>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PartKind {
    Virtex,
    QVirtex,
    QRVirtex,
    Spartan2,
    ASpartan2,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SortKey<'a> {
    kind: ChipKind,
    width: usize,
    height: usize,
    part_kind: PartKind,
    name: &'a str,
}

static RE_VIRTEX: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xcv[0-9]+e?$").unwrap());
static RE_QVIRTEX: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xqv[0-9]+e?$").unwrap());
static RE_QRVIRTEX: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xqvr[0-9]+e?$").unwrap());
static RE_SPARTAN2: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xc2s[0-9]+e?$").unwrap());
static RE_ASPARTAN2: LazyLock<Regex> = LazyLock::new(|| Regex::new("^xa2s[0-9]+e?$").unwrap());

fn sort_key<'a>(name: &'a str, chip: &'a Chip) -> SortKey<'a> {
    let part_kind = if RE_VIRTEX.is_match(name) {
        PartKind::Virtex
    } else if RE_QVIRTEX.is_match(name) {
        PartKind::QVirtex
    } else if RE_QRVIRTEX.is_match(name) {
        PartKind::QRVirtex
    } else if RE_SPARTAN2.is_match(name) {
        PartKind::Spartan2
    } else if RE_ASPARTAN2.is_match(name) {
        PartKind::ASpartan2
    } else {
        panic!("ummm {name}?")
    };
    SortKey {
        kind: chip.kind,
        width: chip.columns,
        height: chip.rows,
        part_kind,
        name,
    }
}

pub fn finish(geom: GeomDb, mut bitdb: CollectorData) -> Database {
    let mut tmp_parts: BTreeMap<&str, _> = BTreeMap::new();
    for dev in &geom.devices {
        let prjcombine_re_xilinx_geom::Chip::Virtex(ref chip) =
            geom.chips[*dev.chips.first().unwrap()]
        else {
            unreachable!()
        };
        let disabled: BTreeSet<_> = dev
            .disabled
            .iter()
            .map(|&dis| {
                let prjcombine_re_xilinx_geom::DisabledPart::Virtex(dis) = dis else {
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
            let prjcombine_re_xilinx_geom::Bond::Virtex(ref bond) = geom.bonds[devbond.bond] else {
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
    let mut int = geom.ints.into_values().next().unwrap();

    let bsdata = std::mem::take(&mut bitdb.bsdata);
    bitdb.insert_into(&mut int, true);

    Database {
        chips,
        bonds,
        devices: parts,
        int,
        bsdata,
    }
}
