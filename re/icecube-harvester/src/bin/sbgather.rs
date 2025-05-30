use std::collections::btree_map;

use jzon::JsonValue;
use prjcombine_interconnect::db::IntDb;
use prjcombine_siliconblue::{
    bond::Bond,
    chip::Chip,
    db::{BondId, ChipId, Database, Part, SpeedId},
};
use prjcombine_types::{bsdata::BsData, speed::Speed};
use unnamed_entity::EntityVec;

fn merge_int(dst: &mut IntDb, src: &IntDb, dbname: &str) {
    if dst.wires.is_empty() {
        dst.wires = src.wires.clone();
        dst.bel_slots = src.bel_slots.clone();
        dst.region_slots = src.region_slots.clone();
        dst.conn_slots = src.conn_slots.clone();
    } else {
        assert_eq!(dst.wires, src.wires);
        assert_eq!(dst.bel_slots, src.bel_slots);
        assert_eq!(dst.region_slots, src.region_slots);
        assert_eq!(dst.conn_slots, src.conn_slots);
    }
    for (_, name, ccls) in &src.conn_classes {
        if let Some((_, dst_ccls)) = dst.conn_classes.get(name) {
            assert_eq!(dst_ccls, ccls);
        } else {
            dst.conn_classes.insert(name.clone(), ccls.clone());
        }
    }
    for (_, name, tcls) in &src.tile_classes {
        if let Some((_, dst_tcls)) = dst.tile_classes.get(name) {
            assert_eq!(dst_tcls, tcls, "FAIL when merging {dbname} TCLS {name}");
        } else {
            dst.tile_classes.insert(name.clone(), tcls.clone());
        }
    }
}

fn merge_bsdata(dst: &mut BsData, src: &BsData, dbname: &str) {
    for (k, v) in &src.misc_data {
        dst.insert_misc_data(k, v.clone());
    }
    for (name, tile) in &src.tiles {
        match dst.tiles.entry(name.clone()) {
            btree_map::Entry::Vacant(e) => {
                e.insert(tile.clone());
            }
            btree_map::Entry::Occupied(e) => {
                assert_eq!(e.get(), tile, "FAIL when merging {dbname} BSTILE {name}");
            }
        }
    }
}

fn merge_bonds(
    dst: &mut EntityVec<BondId, Bond>,
    src: &EntityVec<BondId, Bond>,
) -> EntityVec<BondId, BondId> {
    src.map_values(|bond| {
        for (id, dbond) in &*dst {
            if dbond == bond {
                return id;
            }
        }
        dst.push(bond.clone())
    })
}

fn merge_speeds(
    dst: &mut EntityVec<SpeedId, Speed>,
    src: &EntityVec<SpeedId, Speed>,
) -> EntityVec<SpeedId, SpeedId> {
    src.map_values(|speed| {
        for (id, dspeed) in &*dst {
            if dspeed == speed {
                return id;
            }
        }
        dst.push(speed.clone())
    })
}

fn merge_chips(
    dst: &mut EntityVec<ChipId, Chip>,
    src: &EntityVec<ChipId, Chip>,
) -> EntityVec<ChipId, ChipId> {
    src.map_values(|chip| {
        for (id, dchip) in &*dst {
            if dchip == chip {
                return id;
            }
        }
        dst.push(chip.clone())
    })
}

fn main() {
    let mut dst = Database {
        chips: Default::default(),
        bonds: Default::default(),
        speeds: Default::default(),
        parts: Default::default(),
        int: Default::default(),
        bsdata: Default::default(),
    };
    for fname in [
        "db/icecube/ice65l04.zstd",
        "db/icecube/ice65p04.zstd",
        "db/icecube/ice65l08.zstd",
        "db/icecube/ice65l01.zstd",
        "db/icecube/ice40p01.zstd",
        "db/icecube/ice40p08.zstd",
        "db/icecube/ice40p03.zstd",
        "db/icecube/ice40r04.zstd",
        "db/icecube/ice40t04.zstd",
        "db/icecube/ice40t05.zstd",
        "db/icecube/ice40t01.zstd",
    ] {
        let src = Database::from_file(fname).unwrap();
        merge_int(&mut dst.int, &src.int, fname);
        merge_bsdata(&mut dst.bsdata, &src.bsdata, fname);
        let bonds = merge_bonds(&mut dst.bonds, &src.bonds);
        let speeds = merge_speeds(&mut dst.speeds, &src.speeds);
        let chips = merge_chips(&mut dst.chips, &src.chips);
        for part in &src.parts {
            dst.parts.push(Part {
                name: part.name.clone(),
                chip: chips[part.chip],
                bonds: part
                    .bonds
                    .iter()
                    .map(|(k, &v)| (k.clone(), bonds[v]))
                    .collect(),
                speeds: part
                    .speeds
                    .iter()
                    .map(|(k, &v)| (k.clone(), speeds[v]))
                    .collect(),
                temps: part.temps.clone(),
            });
        }
    }
    dst.to_file("databases/siliconblue.zstd").unwrap();
    std::fs::write(
        "databases/siliconblue.json",
        JsonValue::from(&dst).to_string(),
    )
    .unwrap();
}
