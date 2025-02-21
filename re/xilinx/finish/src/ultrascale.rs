use std::{
    collections::{BTreeMap, BTreeSet, btree_map},
    sync::LazyLock,
};

use itertools::Itertools;
use prjcombine_interconnect::grid::DieId;
use prjcombine_re_xilinx_geom::GeomDb;
use prjcombine_types::tiledb::TileDb;
use prjcombine_ultrascale::{
    bond::Bond,
    chip::{Chip, CleMKind, ColumnKindLeft, DisabledPart, HardRowKind, Interposer, IoRowKind},
    db::{Database, DeviceCombo, Part},
};
use regex::Regex;
use unnamed_entity::{EntityMap, EntitySet, EntityVec};

struct TmpPart<'a> {
    chips: EntityVec<DieId, &'a Chip>,
    interposer: &'a Interposer,
    bonds: BTreeMap<&'a str, &'a Bond>,
    speeds: BTreeSet<&'a str>,
    combos: BTreeSet<(&'a str, &'a str)>,
    disabled: BTreeSet<DisabledPart>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DeviceKind {
    Spartan,
    Virtex,
    VirtexSlr,
    VirtexHbm,
    Zynq,
    ZynqHsAdc,
    ZynqRfAdc,
    ZynqDfe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PsKind {
    Ev,
    Eg,
    Dr,
    Cg,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Suffix {
    None,
    Es1,
    Civ,
    Se,
    Lr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Prefix {
    Xc,
    Xa,
    Xq,
    Xqr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PartKind {
    Zynq,
    Virtex,
    Kintex,
    Artix,
    Spartan,
    Kria,
    Alveo,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SortKey<'a> {
    kind: DeviceKind,
    height: usize,
    width: usize,
    die_num: usize,
    part_kind: PartKind,
    size_neg: i32,
    ps: PsKind,
    suffix: Suffix,
    prefix: Prefix,
    name: &'a str,
}

static RE_ULTRASCALE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        "^(xc|xa|xq|xqr)(k|u|ux|au|su|ku|vu|zu)([0-9]+)t?(|p|c|n|eg|ev|cg|dr)(|-es1|_CIV|_SE|_LR)$",
    )
    .unwrap()
});

fn sort_key<'a>(name: &'a str, part: &TmpPart) -> SortKey<'a> {
    let chip = part.chips.first().unwrap();
    let width = chip.columns.len();
    let height = chip.regs;
    let captures = RE_ULTRASCALE
        .captures(name)
        .unwrap_or_else(|| panic!("ummm {name}?"));
    let prefix = match &captures[1] {
        "xc" => Prefix::Xc,
        "xq" => Prefix::Xq,
        "xqr" => Prefix::Xqr,
        "xa" => Prefix::Xa,
        _ => unreachable!(),
    };
    let part_kind = match &captures[2] {
        "su" => PartKind::Spartan,
        "au" => PartKind::Artix,
        "ku" => PartKind::Kintex,
        "vu" => PartKind::Virtex,
        "zu" => PartKind::Zynq,
        "u" | "ux" => PartKind::Alveo,
        "k" => PartKind::Kria,
        _ => unreachable!(),
    };
    let size: i32 = captures[3].parse().unwrap();
    let ps = match &captures[4] {
        "ev" => PsKind::Ev,
        "eg" => PsKind::Eg,
        "dr" => PsKind::Dr,
        "cg" => PsKind::Cg,
        _ => PsKind::None,
    };
    let suffix = match &captures[5] {
        "" => Suffix::None,
        "-es1" => Suffix::Es1,
        "_CIV" => Suffix::Civ,
        "_SE" => Suffix::Se,
        "_LR" => Suffix::Lr,
        _ => unreachable!(),
    };
    let kind = if chip.ps.is_some() {
        if chip
            .cols_hard
            .iter()
            .any(|hcol| hcol.regs.values().any(|&kind| kind == HardRowKind::DfeA))
        {
            DeviceKind::ZynqDfe
        } else if chip
            .cols_io
            .iter()
            .any(|iocol| iocol.regs.values().any(|&kind| kind == IoRowKind::RfAdc))
        {
            DeviceKind::ZynqRfAdc
        } else if chip
            .cols_io
            .iter()
            .any(|iocol| iocol.regs.values().any(|&kind| kind == IoRowKind::HsAdc))
        {
            DeviceKind::ZynqHsAdc
        } else {
            DeviceKind::Zynq
        }
    } else if chip.has_csec {
        DeviceKind::Spartan
    } else if chip.has_hbm {
        DeviceKind::VirtexHbm
    } else if chip
        .columns
        .values()
        .any(|col| col.l == ColumnKindLeft::CleM(CleMKind::Laguna))
    {
        DeviceKind::VirtexSlr
    } else {
        DeviceKind::Virtex
    };

    SortKey {
        kind,
        height,
        width,
        die_num: part.chips.len(),
        part_kind,
        size_neg: -size,
        ps,
        suffix,
        prefix,
        name,
    }
}

pub fn finish(geom: GeomDb, tiledb: TileDb) -> Database {
    let mut tmp_parts: BTreeMap<&str, _> = BTreeMap::new();
    for dev in &geom.devices {
        let chips = dev.chips.map_values(|&chip| {
            let prjcombine_re_xilinx_geom::Chip::Ultrascale(ref chip) = geom.chips[chip] else {
                unreachable!()
            };
            chip
        });
        let interposer = match geom.interposers[dev.interposer] {
            prjcombine_re_xilinx_geom::Interposer::Ultrascale(ref interposer) => interposer,
            _ => unreachable!(),
        };
        let disabled: BTreeSet<_> = dev
            .disabled
            .iter()
            .map(|&dis| {
                let prjcombine_re_xilinx_geom::DisabledPart::Ultrascale(dis) = dis else {
                    unreachable!()
                };
                dis
            })
            .collect();
        let tpart = tmp_parts.entry(&dev.name).or_insert_with(|| TmpPart {
            chips: chips.clone(),
            interposer,
            disabled: disabled.clone(),
            bonds: Default::default(),
            speeds: Default::default(),
            combos: Default::default(),
        });
        assert_eq!(tpart.chips, chips);
        assert_eq!(tpart.interposer, interposer);
        assert_eq!(tpart.disabled, disabled);
        for devbond in dev.bonds.values() {
            let prjcombine_re_xilinx_geom::Bond::Ultrascale(ref bond) = geom.bonds[devbond.bond]
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
    let mut interposers = EntitySet::new();
    let mut bonds = EntitySet::new();
    let mut parts = vec![];
    for (name, tpart) in tmp_parts
        .into_iter()
        .sorted_by_key(|(name, tpart)| sort_key(name, tpart))
    {
        let chips = tpart.chips.map_values(|&chip| chips.insert(chip.clone()).0);
        let interposer = interposers.insert(tpart.interposer.clone()).0;
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
            chips,
            interposer,
            bonds: dev_bonds,
            speeds,
            combos,
            disabled: tpart.disabled,
        };
        parts.push(part);
    }
    let chips = chips.into_vec();
    let interposers = interposers.into_vec();
    let bonds = bonds.into_vec();

    assert_eq!(geom.ints.len(), 1);
    let int = geom.ints.into_values().next().unwrap();

    // TODO: resort int

    Database {
        chips,
        interposers,
        bonds,
        parts,
        int,
        tiles: tiledb,
    }
}
