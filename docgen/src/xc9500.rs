use std::{collections::HashMap, fmt::Write};

use indexmap::IndexSet;
use itertools::Itertools;
use prjcombine_xc9500::{BankId, BondPad, ChipKind, Database};
use unnamed_entity::{EntityId, EntityPartVec};

use crate::{
    DocgenContext,
    speed::{SpeedData, gen_speed},
    tiledb::{FrameDirection, TileOrientation, gen_tile},
};

fn gen_devlist(ctx: &mut DocgenContext, dbs: &[Database]) {
    let mut buf = String::new();
    writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
    writeln!(buf, r#"<thead>"#).unwrap();
    writeln!(buf, r#"<tr>"#).unwrap();
    writeln!(buf, r#"<th>Device</th>"#).unwrap();
    writeln!(buf, r#"<th>Variant</th>"#).unwrap();
    writeln!(buf, r#"<th>IDCODE</th>"#).unwrap();
    writeln!(buf, r#"<th>Function blocks</th>"#).unwrap();
    writeln!(buf, r#"<th>GOE pins / FOE networks</th>"#).unwrap();
    writeln!(buf, r#"<th>I/O banks</th>"#).unwrap();
    writeln!(buf, r#"<th>Notes</th>"#).unwrap();
    writeln!(buf, r#"</tr>"#).unwrap();
    writeln!(buf, r#"</thead>"#).unwrap();
    writeln!(buf, r#"<tbody>"#).unwrap();
    for db in dbs {
        for part in &db.parts {
            let chip = &db.chips[part.chip];
            let notes = if chip.kind == ChipKind::Xc9500 && chip.blocks == 2 {
                "Does not have block input feedback"
            } else if chip.uim_ibuf_bits.is_some() {
                "Has special input buffer enable fuses"
            } else if chip.blocks == 4 {
                "GOE mapping to pads varies with package"
            } else {
                ""
            };
            let goe_num = chip
                .io_special
                .keys()
                .filter(|key| key.starts_with("GOE"))
                .count();
            writeln!(buf, r#"<tr>"#).unwrap();
            writeln!(buf, r#"<td>{}</td>"#, part.name).unwrap();
            writeln!(buf, r#"<td>{}</td>"#, chip.kind).unwrap();
            writeln!(buf, r#"<td>{:#010x}</td>"#, chip.idcode).unwrap();
            writeln!(buf, r#"<td>{}</td>"#, chip.blocks).unwrap();
            writeln!(buf, r#"<td>{goe_num}</td>"#).unwrap();
            writeln!(buf, r#"<td>{}</td>"#, chip.banks).unwrap();
            writeln!(buf, r#"<td>{notes}</td>"#).unwrap();
            writeln!(buf, r#"</tr>"#).unwrap();
        }
    }
    writeln!(buf, r#"</tbody>"#).unwrap();
    writeln!(buf, r#"</table></div>"#).unwrap();
    ctx.items.insert("devlist-xc9500".into(), buf);
}

fn gen_devpkg(ctx: &mut DocgenContext, dbs: &[Database]) {
    let mut buf = String::new();
    let mut packages = IndexSet::new();
    for db in dbs {
        for part in &db.parts {
            for pkg in part.packages.keys() {
                packages.insert(pkg.clone());
            }
        }
    }
    writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
    writeln!(buf, r#"<thead>"#).unwrap();
    writeln!(buf, r#"<tr>"#).unwrap();
    writeln!(buf, r#"<th>Device</th>"#).unwrap();
    for pkg in &packages {
        writeln!(buf, r#"<th>{pkg}</th>"#).unwrap();
    }
    writeln!(buf, r#"</tr>"#).unwrap();
    writeln!(buf, r#"</thead>"#).unwrap();
    writeln!(buf, r#"<tbody>"#).unwrap();
    for db in dbs {
        for part in &db.parts {
            writeln!(buf, r#"<tr>"#).unwrap();
            writeln!(buf, r#"<td>{}</td>"#, part.name).unwrap();
            for pkg in &packages {
                if part.packages.contains_key(pkg) {
                    writeln!(buf, r#"<td>✅</td>"#).unwrap();
                } else {
                    writeln!(buf, r#"<td>❌</td>"#).unwrap();
                }
            }
            writeln!(buf, r#"</tr>"#).unwrap();
        }
    }
    writeln!(buf, r#"</tbody>"#).unwrap();
    writeln!(buf, r#"</table></div>"#).unwrap();
    ctx.items.insert("devpkg-xc9500".into(), buf);
}

fn gen_devices(ctx: &mut DocgenContext, dbs: &[Database]) {
    struct BondData {
        names: Vec<String>,
        pins: HashMap<BondPad, PinData>,
    }
    struct PinData {
        pins: Vec<String>,
        special: Option<String>,
    }

    let orientation = TileOrientation {
        frame_direction: FrameDirection::Horizontal,
        flip_frame: false,
        flip_bit: false,
    };

    for db in dbs {
        for (chipid, chip) in &db.chips {
            let mut parts = vec![];
            let mut bonds = EntityPartVec::new();
            let mut speeds = EntityPartVec::new();
            let mut packages = IndexSet::new();
            for part in &db.parts {
                if part.chip != chipid {
                    continue;
                }
                parts.push(part);
                for (pkg, &bondid) in &part.packages {
                    packages.insert(pkg);
                    let bond = &db.bonds[bondid];
                    if !bonds.contains_id(bondid) {
                        let mut pins = HashMap::new();
                        let mut io_special = chip.io_special.clone();
                        for (k, &v) in &bond.io_special_override {
                            io_special.insert(k.clone(), v);
                        }
                        let io_special_rev: HashMap<_, _> = HashMap::from_iter(
                            io_special.iter().map(|(k, &mc)| (BondPad::Iob(mc), k)),
                        );
                        for (k, &v) in &bond.pins {
                            pins.entry(v)
                                .or_insert_with(|| PinData {
                                    pins: vec![],
                                    special: io_special_rev.get(&v).copied().cloned(),
                                })
                                .pins
                                .push(k.clone());
                        }
                        bonds.insert(
                            bondid,
                            BondData {
                                names: vec![],
                                pins,
                            },
                        );
                    }
                    bonds[bondid]
                        .names
                        .push(format!("{pname}-{pkg}", pname = part.name));
                }

                for (sname, &speedid) in &part.speeds {
                    let speed = &db.speeds[speedid];
                    if !speeds.contains_id(speedid) {
                        speeds.insert(
                            speedid,
                            SpeedData {
                                names: vec![],
                                speed,
                            },
                        );
                    }
                    speeds[speedid]
                        .names
                        .push(format!("{pname}{sname}", pname = part.name));
                }
            }

            let mut buf = String::new();
            let names = parts
                .iter()
                .map(|part| &part.name)
                .map(|name| name.to_uppercase())
                .join(", ");
            writeln!(buf, r#"# {names}"#).unwrap();
            writeln!(buf).unwrap();
            writeln!(buf, r#"|Parameter|Value|"#).unwrap();
            writeln!(buf, r#"|-|-|"#).unwrap();
            writeln!(buf, r#"|IDCODE|{idcode:#010x}|"#, idcode = chip.idcode).unwrap();
            writeln!(buf, r#"|Block count|{blocks}|"#, blocks = chip.blocks).unwrap();
            writeln!(buf, r#"|I/O bank count|{banks}|"#, banks = chip.banks).unwrap();
            writeln!(buf, r#"|FPGM/FPGMI time|{time}|"#, time = chip.program_time).unwrap();
            writeln!(buf, r#"|FERASE/FBULK time|{time}|"#, time = chip.erase_time).unwrap();
            writeln!(buf).unwrap();

            writeln!(buf, r#"## I/O pins"#).unwrap();
            writeln!(buf).unwrap();
            writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
            writeln!(buf, r#"<thead>"#).unwrap();
            writeln!(buf, r#"<tr>"#).unwrap();
            writeln!(buf, r#"<th>Function</th>"#).unwrap();
            writeln!(buf, r#"<th>Bank</th>"#).unwrap();
            for bond in bonds.values() {
                let names = bond.names.join("<br>");
                writeln!(buf, r#"<th>{names}</th>"#).unwrap();
            }
            writeln!(buf, r#"</tr>"#).unwrap();
            writeln!(buf, r#"</thead>"#).unwrap();
            writeln!(buf, r#"<tbody>"#).unwrap();
            for (&mc, &bank) in &chip.io {
                writeln!(buf, r#"<tr>"#).unwrap();
                writeln!(buf, r#"<td>IOB_{mc}</td>"#).unwrap();
                writeln!(buf, r#"<td>{bank}</td>"#).unwrap();
                for bond in bonds.values() {
                    if let Some(pin) = bond.pins.get(&BondPad::Iob(mc)) {
                        let pins = pin.pins.join(", ");
                        if let Some(ref spec) = pin.special {
                            writeln!(buf, r#"<td>{pins} ({spec})</td>"#).unwrap();
                        } else {
                            writeln!(buf, r#"<td>{pins}</td>"#).unwrap();
                        }
                    } else {
                        writeln!(buf, r#"<td>-</td>"#).unwrap();
                    }
                }
                writeln!(buf, r#"</tr>"#).unwrap();
            }
            let mut specs = vec![
                BondPad::Tck,
                BondPad::Tms,
                BondPad::Tdi,
                BondPad::Tdo,
                BondPad::Gnd,
                BondPad::VccInt,
            ];
            for bank in 0..chip.banks {
                specs.push(BondPad::VccIo(BankId::from_idx(bank)));
            }
            specs.push(BondPad::Nc);
            for pin in specs {
                writeln!(buf, r#"<tr>"#).unwrap();
                writeln!(buf, r#"<td>{pin}</td>"#).unwrap();
                let bank = match pin {
                    BondPad::Tdo => Some(chip.tdo_bank),
                    BondPad::VccIo(bank) => Some(bank),
                    _ => None,
                };
                if let Some(bank) = bank {
                    writeln!(buf, r#"<td>{bank}</td>"#).unwrap();
                } else {
                    writeln!(buf, r#"<td>-</td>"#).unwrap();
                }
                for bond in bonds.values() {
                    if let Some(pin) = bond.pins.get(&pin) {
                        let pins = pin.pins.join("<br>");
                        writeln!(buf, r#"<td>{pins}</td>"#).unwrap();
                    } else {
                        writeln!(buf, r#"<td>-</td>"#).unwrap();
                    }
                }
                writeln!(buf, r#"</tr>"#).unwrap();
            }
            writeln!(buf, r#"</tbody>"#).unwrap();
            writeln!(buf, r#"</table></div>"#).unwrap();
            writeln!(buf).unwrap();

            gen_speed(ctx, &parts[0].name, &Vec::from_iter(speeds.into_values()));
            writeln!(buf, r#"## Speed data"#).unwrap();
            writeln!(buf).unwrap();
            let item = ctx
                .items
                .remove(&format!("speed-{pname}", pname = parts[0].name))
                .unwrap();
            buf.push_str(&item);
            writeln!(buf).unwrap();

            gen_tile(ctx, &parts[0].name, "imux", &chip.imux_bits, orientation);
            writeln!(buf, r#"## IMUX bits"#).unwrap();
            writeln!(buf).unwrap();
            let item = ctx
                .items
                .remove(&format!("tile-{pname}-imux", pname = parts[0].name))
                .unwrap();
            buf.push_str(&item);
            writeln!(buf).unwrap();

            if let Some(ref tile) = chip.uim_ibuf_bits {
                gen_tile(ctx, &parts[0].name, "uim-ibuf", tile, orientation);
                writeln!(buf, r#"## UIM IBUF bits"#).unwrap();
                writeln!(buf).unwrap();
                let item = ctx
                    .items
                    .remove(&format!("tile-{pname}-uim-ibuf", pname = parts[0].name))
                    .unwrap();
                buf.push_str(&item);
                writeln!(buf).unwrap();
            }

            ctx.extra_docs
                .entry("xc9500/devices/index.md".into())
                .or_default()
                .push((
                    format!("xc9500/devices/{pname}.md", pname = parts[0].name),
                    names,
                    buf,
                ));
        }
    }
}

pub fn gen_xc9500(ctx: &mut DocgenContext) {
    let orientation = TileOrientation {
        frame_direction: FrameDirection::Horizontal,
        flip_frame: false,
        flip_bit: false,
    };
    let mut dbs = vec![];
    for kind in ["xc9500", "xc9500xl", "xc9500xv"] {
        let db =
            Database::from_file(ctx.ctx.root.join(format!("../databases/{kind}.zstd"))).unwrap();
        gen_tile(ctx, kind, "mc", &db.mc_bits, orientation);
        gen_tile(ctx, kind, "block", &db.block_bits, orientation);
        gen_tile(ctx, kind, "global", &db.global_bits, orientation);
        dbs.push(db);
    }
    gen_devlist(ctx, &dbs);
    gen_devpkg(ctx, &dbs);
    gen_devices(ctx, &dbs);
}
