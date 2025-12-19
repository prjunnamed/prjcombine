use std::{collections::HashMap, fmt::Write};

use indexmap::IndexSet;
use itertools::Itertools;
use prjcombine_entity::{EntityId, EntityPartVec};
use prjcombine_types::{
    bsdata::{Tile, TileItemKind},
    cpld::{ClusterId, MacrocellCoord},
};
use prjcombine_xpla3::{BondPad, Database};

use crate::{
    DocgenContext,
    bsdata::{FrameDirection, TileOrientation, gen_tile},
    speed::{SpeedData, gen_speed},
};

fn gen_devlist(ctx: &mut DocgenContext, db: &Database) {
    let mut buf = String::new();
    writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
    writeln!(buf, r#"<thead>"#).unwrap();
    writeln!(buf, r#"<tr>"#).unwrap();
    writeln!(buf, r#"<th>Device</th>"#).unwrap();
    writeln!(buf, r#"<th>IDCODE</th>"#).unwrap();
    writeln!(buf, r#"<th>Function blocks</th>"#).unwrap();
    writeln!(buf, r#"</tr>"#).unwrap();
    writeln!(buf, r#"</thead>"#).unwrap();
    writeln!(buf, r#"<tbody>"#).unwrap();
    for part in &db.devices {
        let chip = &db.chips[part.chip];
        writeln!(buf, r#"<tr>"#).unwrap();
        writeln!(buf, r#"<td>{}</td>"#, part.name).unwrap();
        writeln!(buf, r#"<td>0xX{:04x}XXX</td>"#, chip.idcode_part).unwrap();
        writeln!(buf, r#"<td>{}</td>"#, chip.blocks().len()).unwrap();
        writeln!(buf, r#"</tr>"#).unwrap();
    }
    writeln!(buf, r#"</tbody>"#).unwrap();
    writeln!(buf, r#"</table></div>"#).unwrap();
    ctx.items.insert("devlist-xpla3".into(), buf);
}

fn gen_devpkg(ctx: &mut DocgenContext, db: &Database) {
    let mut buf = String::new();
    let mut packages = IndexSet::new();
    for part in &db.devices {
        for pkg in part.packages.keys() {
            packages.insert(pkg.clone());
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
    for part in &db.devices {
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
    writeln!(buf, r#"</tbody>"#).unwrap();
    writeln!(buf, r#"</table></div>"#).unwrap();
    ctx.items.insert("devpkg-xpla3".into(), buf);
}

pub fn gen_jed(
    ctx: &mut DocgenContext,
    dbname: &str,
    tname: &str,
    tile: &Tile,
    jname: &str,
    bits: &[(String, usize)],
) {
    let mut buf = String::new();
    writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
    writeln!(buf, r#"<thead>"#).unwrap();
    writeln!(buf, r#"<tr>"#).unwrap();
    writeln!(buf, r#"<th>JED offset</th>"#).unwrap();
    writeln!(buf, r#"<th>Bit</th>"#).unwrap();
    writeln!(buf, r#"</tr>"#).unwrap();
    writeln!(buf, r#"</thead>"#).unwrap();
    writeln!(buf, r#"<tbody>"#).unwrap();
    for (idx, &(ref name, bit)) in bits.iter().enumerate() {
        let item = &tile.items[name];
        let bname = if matches!(item.kind, TileItemKind::BitVec { .. }) && item.bits.len() == 1 {
            name.clone()
        } else {
            format!("{name}[{bit}]")
        };
        writeln!(buf, r#"<tr>"#).unwrap();
        writeln!(buf, r#"<td>{idx}</td>"#).unwrap();
        writeln!(
            buf,
            r##"<td><a href="#tile-{dbname}-{tname}-{name}">{bname}</a></td>"##
        )
        .unwrap();
        writeln!(buf, r#"</tr>"#).unwrap();
    }
    writeln!(buf, r#"</tbody>"#).unwrap();
    writeln!(buf, r#"</table></div>"#).unwrap();
    ctx.items.insert(format!("jed-{dbname}-{jname}"), buf);
}

fn gen_devices(ctx: &mut DocgenContext, db: &Database) {
    struct BondData {
        names: Vec<String>,
        pins: HashMap<BondPad, PinData>,
    }
    struct PinData {
        pins: Vec<String>,
    }

    let orientation = TileOrientation {
        frame_direction: FrameDirection::Horizontal,
        flip_frame: false,
        flip_bit: false,
    };

    for (chipid, chip) in &db.chips {
        let mut parts = vec![];
        let mut bonds = EntityPartVec::new();
        let mut speeds = EntityPartVec::new();
        let mut packages = IndexSet::new();
        for part in &db.devices {
            if part.chip != chipid {
                continue;
            }
            parts.push(part);
            for (pkg, &bondid) in &part.packages {
                packages.insert(pkg);
                let bond = &db.bonds[bondid];
                if !bonds.contains_id(bondid) {
                    let mut pins = HashMap::new();
                    for (k, &v) in &bond.pins {
                        pins.entry(v)
                            .or_insert_with(|| PinData { pins: vec![] })
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
        writeln!(
            buf,
            r#"|IDCODE part|{idcode:#06x}|"#,
            idcode = chip.idcode_part
        )
        .unwrap();
        writeln!(
            buf,
            r#"|Block count|{blocks}|"#,
            blocks = chip.blocks().len()
        )
        .unwrap();
        writeln!(buf, r#"|BS columns|{cols}|"#, cols = chip.bs_cols).unwrap();
        writeln!(buf, r#"|IMUX width|{width}|"#, width = chip.imux_width).unwrap();
        writeln!(buf, r#"|Block rows|{rows}|"#, rows = chip.block_rows).unwrap();
        writeln!(
            buf,
            r#"|Block columns|{cols}|"#,
            cols = chip.block_cols.len()
        )
        .unwrap();
        writeln!(buf).unwrap();

        writeln!(buf, r#"## Bitstream columns"#).unwrap();
        writeln!(buf).unwrap();
        writeln!(buf, r#"|Column range|Bits|"#).unwrap();
        writeln!(buf, r#"|-|-|"#).unwrap();
        let mut items = vec![];
        for (idx, bcol) in chip.block_cols.iter().enumerate() {
            items.push((
                bcol.imux_col,
                chip.imux_width,
                format!("Block column {idx} IMUX"),
            ));
            items.push((bcol.pt_col, 48, format!("Block column {idx} even PTs")));
            items.push((bcol.pt_col + 48, 48, format!("Block column {idx} odd PTs")));
            items.push((bcol.mc_col, 5, format!("Block column {idx} even MCs")));
            items.push((bcol.mc_col + 5, 5, format!("Block column {idx} odd MCs")));
        }
        items.sort();
        for (bit, width, item) in items {
            writeln!(buf, r#"|{bit}..{bit_end}|{item}|"#, bit_end = bit + width).unwrap();
        }
        writeln!(buf).unwrap();

        let io_special_rev: HashMap<_, _> =
            HashMap::from_iter(chip.io_special.iter().map(|(k, &mc)| (BondPad::Iob(mc), k)));

        writeln!(buf, r#"## I/O pins"#).unwrap();
        writeln!(buf).unwrap();
        writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
        writeln!(buf, r#"<thead>"#).unwrap();
        writeln!(buf, r#"<tr>"#).unwrap();
        writeln!(buf, r#"<th>Function</th>"#).unwrap();
        for bond in bonds.values() {
            let names = bond.names.join("<br>");
            writeln!(buf, r#"<th>{names}</th>"#).unwrap();
        }
        writeln!(buf, r#"</tr>"#).unwrap();
        writeln!(buf, r#"</thead>"#).unwrap();
        writeln!(buf, r#"<tbody>"#).unwrap();
        writeln!(buf, r#"<tr>"#).unwrap();
        writeln!(buf, r#"<td>IDCODE part</td>"#).unwrap();
        for bondid in bonds.ids() {
            let bond = &db.bonds[bondid];
            writeln!(buf, r#"<td>{:#06x}</td>"#, bond.idcode_part).unwrap();
        }
        writeln!(buf, r#"</tr>"#).unwrap();
        for block in chip.blocks() {
            for &mc in &chip.io_mcs {
                let mc = MacrocellCoord {
                    cluster: ClusterId::from_idx(0),
                    block,
                    macrocell: mc,
                };
                writeln!(buf, r#"<tr>"#).unwrap();
                writeln!(buf, r#"<td>IOB_{block}_{mc}</td>"#).unwrap();
                for bond in bonds.values() {
                    if let Some(pin) = bond.pins.get(&BondPad::Iob(mc)) {
                        let pins = pin.pins.join(", ");
                        if let Some(spec) = io_special_rev.get(&BondPad::Iob(mc)) {
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
        }
        for pin in [BondPad::PortEn, BondPad::Gnd, BondPad::Vcc, BondPad::Nc] {
            writeln!(buf, r#"<tr>"#).unwrap();
            writeln!(buf, r#"<td>{pin}</td>"#).unwrap();
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

        gen_tile(
            ctx,
            &parts[0].name,
            "global",
            &chip.global_bits,
            orientation,
        );
        writeln!(buf, r#"## Global bits"#).unwrap();
        writeln!(buf).unwrap();
        let item = ctx
            .items
            .remove(&format!("tile-{pname}-global", pname = parts[0].name))
            .unwrap();
        buf.push_str(&item);
        writeln!(buf).unwrap();

        gen_jed(
            ctx,
            &parts[0].name,
            "global",
            &chip.global_bits,
            "global",
            &chip.jed_global_bits,
        );
        writeln!(buf, r#"### JED mapping"#).unwrap();
        writeln!(buf).unwrap();
        let item = ctx
            .items
            .remove(&format!("jed-{pname}-global", pname = parts[0].name))
            .unwrap();
        buf.push_str(&item);
        writeln!(buf).unwrap();

        ctx.extra_docs
            .entry("xpla3/devices/index.md".into())
            .or_default()
            .push((
                format!("xpla3/devices/{pname}.md", pname = parts[0].name),
                names,
                buf,
            ));
    }
}

pub fn gen_xpla3(ctx: &mut DocgenContext) {
    let orientation = TileOrientation {
        frame_direction: FrameDirection::Horizontal,
        flip_frame: false,
        flip_bit: false,
    };
    let db = prjcombine_xpla3::Database::from_file(ctx.ctx.root.join("../databases/xpla3.zstd"))
        .unwrap();
    gen_tile(ctx, "xpla3", "mc", &db.mc_bits, orientation);
    gen_tile(ctx, "xpla3", "block", &db.block_bits, orientation);
    gen_jed(
        ctx,
        "xpla3",
        "mc",
        &db.mc_bits,
        "mc-iob",
        &db.jed_mc_bits_iob,
    );
    gen_jed(
        ctx,
        "xpla3",
        "mc",
        &db.mc_bits,
        "mc-buried",
        &db.jed_mc_bits_buried,
    );
    gen_jed(
        ctx,
        "xpla3",
        "block",
        &db.block_bits,
        "block",
        &db.jed_block_bits,
    );
    gen_devlist(ctx, &db);
    gen_devpkg(ctx, &db);
    gen_devices(ctx, &db);
}
