use std::{
    collections::{BTreeMap, BTreeSet, btree_map},
    sync::LazyLock,
};

use itertools::Itertools;
use prjcombine_interconnect::grid::DieId;
use prjcombine_re_xilinx_geom::GeomDb;
use prjcombine_types::tiledb::TileDb;
use prjcombine_virtex4::{
    bond::Bond,
    chip::{Chip, DisabledPart, GtKind, Interposer},
    db::{Database, DeviceCombo, Part},
};
use regex::Regex;
use unnamed_entity::{EntityMap, EntitySet, EntityVec};

struct TmpPart<'a> {
    grids: EntityVec<DieId, &'a Chip>,
    interposer: Option<&'a Interposer>,
    bonds: BTreeMap<&'a str, &'a Bond>,
    speeds: BTreeSet<&'a str>,
    combos: BTreeSet<(&'a str, &'a str)>,
    disabled: BTreeSet<DisabledPart>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DeviceKind {
    // Virtex 4/5/6
    Lx,
    Lxt,
    Sx,
    Sxt,
    Fx,
    Fxt,
    Txt,
    // Virtex 7
    Spartan,
    Artix,
    Kintex,
    Virtex,
    VirtexSlr,
    VirtexGth,
    VirtexGthSlr,
    Zynq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Suffix {
    None,
    I,
    L,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Prefix {
    Xc,
    Xa,
    Xq,
    Xqr,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SortKey<'a> {
    kind: DeviceKind,
    height: usize,
    width: usize,
    has_gth: bool,
    is_spartan: bool,
    size_neg: i32,
    is_cx: bool,
    suffix: Suffix,
    prefix: Prefix,
    name: &'a str,
}

static RE_VIRTEX456: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^(xc|xq|xqr)[456]v(lx|sx|fx|tx|hx|cx)([0-9]+)(t?)(l?)$").unwrap());
static RE_VIRTEX7: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^(xc|xa|xq|xqr)7(s|a|k|v|vx|vh|z)([0-9]+)t?s?([il]?)$").unwrap());

fn sort_key<'a>(name: &'a str, grid: &Chip) -> SortKey<'a> {
    let width = grid.columns.len();
    let height = grid.regs;

    if let Some(captures) = RE_VIRTEX456.captures(name) {
        let prefix = match &captures[1] {
            "xc" => Prefix::Xc,
            "xq" => Prefix::Xq,
            "xqr" => Prefix::Xqr,
            _ => unreachable!(),
        };
        let kind = match (&captures[2], &captures[4]) {
            ("lx", "") => DeviceKind::Lx,
            ("lx", "t") => DeviceKind::Lxt,
            ("cx", "t") => DeviceKind::Lxt,
            ("sx", "") => DeviceKind::Sx,
            ("sx", "t") => DeviceKind::Sxt,
            ("fx", "") => DeviceKind::Fx,
            ("fx", "t") => DeviceKind::Fxt,
            ("tx" | "hx", "t") => DeviceKind::Txt,
            _ => panic!("ummm {name}?"),
        };
        let suffix = match &captures[5] {
            "" => Suffix::None,
            "l" => Suffix::L,
            _ => unreachable!(),
        };
        let size: i32 = captures[3].parse().unwrap();
        SortKey {
            kind,
            height,
            width,
            has_gth: grid
                .cols_gt
                .iter()
                .any(|gtcol| gtcol.regs.values().any(|&kind| kind == Some(GtKind::Gth))),
            is_spartan: false,
            size_neg: -size,
            is_cx: &captures[2] == "cx",
            suffix,
            prefix,
            name,
        }
    } else if let Some(captures) = RE_VIRTEX7.captures(name) {
        let prefix = match &captures[1] {
            "xc" => Prefix::Xc,
            "xa" => Prefix::Xa,
            "xq" => Prefix::Xq,
            "xqr" => Prefix::Xqr,
            _ => unreachable!(),
        };
        let size: i32 = captures[3].parse().unwrap();
        let suffix = match &captures[4] {
            "" => Suffix::None,
            "l" => Suffix::L,
            "i" => Suffix::I,
            _ => unreachable!(),
        };
        let kind = if grid.has_ps {
            DeviceKind::Zynq
        } else if grid
            .cols_gt
            .iter()
            .any(|gtcol| gtcol.regs.values().any(|&kind| kind == Some(GtKind::Gth)))
        {
            if grid.has_slr {
                DeviceKind::VirtexGthSlr
            } else {
                DeviceKind::VirtexGth
            }
        } else if grid.cols_io.len() == 2 && grid.cols_io[1].col != grid.columns.last_id().unwrap()
        {
            if grid.has_slr {
                DeviceKind::VirtexSlr
            } else {
                DeviceKind::Virtex
            }
        } else if grid
            .cols_gt
            .iter()
            .any(|gtcol| gtcol.regs.values().any(|&kind| kind == Some(GtKind::Gtx)))
        {
            DeviceKind::Kintex
        } else if grid
            .cols_gt
            .iter()
            .any(|gtcol| gtcol.regs.values().any(|&kind| kind == Some(GtKind::Gtp)))
        {
            DeviceKind::Artix
        } else {
            DeviceKind::Spartan
        };
        SortKey {
            kind,
            height,
            width,
            has_gth: false,
            is_spartan: &captures[2] == "s",
            size_neg: -size,
            is_cx: false,
            suffix,
            prefix,
            name,
        }
    } else {
        panic!("ummm {name}?")
    }
}

pub fn finish(geom: GeomDb, tiledb: TileDb) -> Database {
    let mut tmp_parts: BTreeMap<&str, _> = BTreeMap::new();
    for dev in &geom.devices {
        let grids = dev.grids.map_values(|&grid| {
            let prjcombine_re_xilinx_geom::Grid::Virtex4(ref grid) = geom.grids[grid] else {
                unreachable!()
            };
            grid
        });
        let interposer = match geom.interposers[dev.interposer] {
            prjcombine_re_xilinx_geom::Interposer::None => None,
            prjcombine_re_xilinx_geom::Interposer::Virtex4(ref interposer) => Some(interposer),
            _ => unreachable!(),
        };
        let disabled: BTreeSet<_> = dev
            .disabled
            .iter()
            .map(|&dis| {
                let prjcombine_re_xilinx_geom::DisabledPart::Virtex4(dis) = dis else {
                    unreachable!()
                };
                dis
            })
            .collect();
        let tpart = tmp_parts.entry(&dev.name).or_insert_with(|| TmpPart {
            grids: grids.clone(),
            interposer,
            disabled: disabled.clone(),
            bonds: Default::default(),
            speeds: Default::default(),
            combos: Default::default(),
        });
        assert_eq!(tpart.grids, grids);
        assert_eq!(tpart.interposer, interposer);
        assert_eq!(tpart.disabled, disabled);
        for devbond in dev.bonds.values() {
            let prjcombine_re_xilinx_geom::Bond::Virtex4(ref bond) = geom.bonds[devbond.bond]
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
    let mut interposers = EntitySet::new();
    let mut bonds = EntitySet::new();
    let mut parts = vec![];
    for (name, tpart) in tmp_parts
        .into_iter()
        .sorted_by_key(|(name, tpart)| sort_key(name, tpart.grids.first().unwrap()))
    {
        let grids = tpart.grids.map_values(|&grid| grids.insert(grid.clone()).0);
        let interposer = tpart
            .interposer
            .map(|interposer| interposers.insert(interposer.clone()).0);
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
            chips: grids,
            interposer,
            bonds: dev_bonds,
            speeds,
            combos,
            disabled: tpart.disabled,
        };
        parts.push(part);
    }
    let grids = grids.into_vec();
    let interposers = interposers.into_vec();
    let bonds = bonds.into_vec();

    assert_eq!(geom.ints.len(), 1);
    let int = geom.ints.into_values().next().unwrap();

    // TODO: resort int

    Database {
        chips: grids,
        interposers,
        bonds,
        parts,
        int,
        tiles: tiledb,
        gtz: geom.gtz,
    }
}
