use std::{collections::HashMap, fmt::Write};

use indexmap::IndexSet;
use itertools::Itertools;
use prjcombine_coolrunner2::{BankId, BondPad, BsLayout, Database};
use prjcombine_types::cpld::IoCoord;
use unnamed_entity::{EntityId, EntityPartVec};

use crate::{
    DocgenContext,
    bsdata::{FrameDirection, TileOrientation, gen_tile},
    speed::{SpeedData, gen_speed},
    xpla3::gen_jed,
};

fn gen_devlist(ctx: &mut DocgenContext, db: &Database) {
    let mut buf = String::new();
    writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
    writeln!(buf, r#"<thead>"#).unwrap();
    writeln!(buf, r#"<tr>"#).unwrap();
    writeln!(buf, r#"<th>Device</th>"#).unwrap();
    writeln!(buf, r#"<th>IDCODE</th>"#).unwrap();
    writeln!(buf, r#"<th>Function blocks</th>"#).unwrap();
    writeln!(buf, r#"<th>I/O banks</th>"#).unwrap();
    writeln!(buf, r#"<th>Input pads</th>"#).unwrap();
    writeln!(buf, r#"<th>VREF</th>"#).unwrap();
    writeln!(buf, r#"<th>Data gate</th>"#).unwrap();
    writeln!(buf, r#"<th>Clock divider</th>"#).unwrap();
    writeln!(buf, r#"</tr>"#).unwrap();
    writeln!(buf, r#"</thead>"#).unwrap();
    writeln!(buf, r#"<tbody>"#).unwrap();
    for part in &db.parts {
        let chip = &db.chips[part.chip];
        writeln!(buf, r#"<tr>"#).unwrap();
        writeln!(buf, r#"<td>{}</td>"#, part.name).unwrap();
        writeln!(buf, r#"<td>0xX{:04x}093</td>"#, chip.idcode_part).unwrap();
        writeln!(buf, r#"<td>{}</td>"#, chip.blocks().len()).unwrap();
        writeln!(buf, r#"<td>{}</td>"#, chip.banks).unwrap();
        writeln!(buf, r#"<td>{}</td>"#, chip.ipads).unwrap();
        for cond in [
            chip.has_vref,
            chip.io_special.contains_key("DGE"),
            chip.io_special.contains_key("CDR"),
        ] {
            if cond {
                writeln!(buf, r#"<td>✅</td>"#).unwrap();
            } else {
                writeln!(buf, r#"<td>❌</td>"#).unwrap();
            }
        }
        writeln!(buf, r#"</tr>"#).unwrap();
    }
    writeln!(buf, r#"</tbody>"#).unwrap();
    writeln!(buf, r#"</table></div>"#).unwrap();
    ctx.items.insert("devlist-coolrunner2".into(), buf);
}

fn gen_devpkg(ctx: &mut DocgenContext, db: &Database) {
    let mut buf = String::new();
    let mut packages = IndexSet::new();
    for part in &db.parts {
        for pkg in part.packages.keys() {
            if !pkg.starts_with("di") {
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
    writeln!(buf, r#"<th>Bare die</th>"#).unwrap();
    writeln!(buf, r#"</tr>"#).unwrap();
    writeln!(buf, r#"</thead>"#).unwrap();
    writeln!(buf, r#"<tbody>"#).unwrap();
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
        let mut bare = None;
        for key in part.packages.keys() {
            if key.starts_with("di") {
                bare = Some(key);
            }
        }
        if let Some(bare) = bare {
            writeln!(buf, r#"<td>{bare}</td>"#).unwrap();
        } else {
            writeln!(buf, r#"<td>-</td>"#).unwrap();
        }
        writeln!(buf, r#"</tr>"#).unwrap();
    }
    writeln!(buf, r#"</tbody>"#).unwrap();
    writeln!(buf, r#"</table></div>"#).unwrap();
    ctx.items.insert("devpkg-coolrunner2".into(), buf);
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
        let to_bool = |val| if val { "✅" } else { "❌" };
        writeln!(
            buf,
            r#"|Block count|{blocks}|"#,
            blocks = chip.blocks().len()
        )
        .unwrap();
        writeln!(buf, r#"|I/O banks|{banks}|"#, banks = chip.banks).unwrap();
        writeln!(buf, r#"|Input-only pads|{ipads}|"#, ipads = chip.ipads).unwrap();
        writeln!(buf, r#"|Has VREF|{vref}|"#, vref = to_bool(chip.has_vref)).unwrap();
        writeln!(buf, r#"|BS columns|{cols}|"#, cols = chip.bs_cols).unwrap();
        writeln!(buf, r#"|IMUX width|{width}|"#, width = chip.imux_width).unwrap();
        writeln!(
            buf,
            r#"|BS layout|{layout}|"#,
            layout = match chip.bs_layout {
                BsLayout::Narrow => "narrow",
                BsLayout::Wide => "wide",
            }
        )
        .unwrap();
        writeln!(buf, r#"|Block rows|{rows}|"#, rows = chip.block_rows).unwrap();
        writeln!(buf, r#"|Macrocell width|{width}|"#, width = chip.mc_width).unwrap();
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
        for &bit in &chip.xfer_cols {
            items.push((bit, 1, "transfer".to_string()));
        }
        for (idx, &(mut col)) in chip.block_cols.iter().enumerate() {
            items.push((col, chip.mc_width, format!("Block column {idx} even MCs")));
            col += chip.mc_width;
            match chip.bs_layout {
                BsLayout::Narrow => {
                    items.push((col, 32, format!("Block column {idx} even PTs OR")));
                    col += 32;
                    items.push((col, 112, format!("Block column {idx} even PTs AND")));
                    col += 112;
                }
                BsLayout::Wide => {
                    items.push((col, 112, format!("Block column {idx} even PTs")));
                    col += 112;
                }
            }
            items.push((col, chip.imux_width * 2, format!("Block column {idx} IMUX")));
            col += chip.imux_width * 2;
            match chip.bs_layout {
                BsLayout::Narrow => {
                    items.push((col, 112, format!("Block column {idx} odd PTs AND")));
                    col += 112;
                    items.push((col, 32, format!("Block column {idx} odd PTs OR")));
                    col += 32;
                }
                BsLayout::Wide => {
                    items.push((col, 112, format!("Block column {idx} odd PTs")));
                    col += 112;
                }
            }
            items.push((col, chip.mc_width, format!("Block column {idx} odd MCs")));
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
        writeln!(buf, r#"<th>Bank</th>"#).unwrap();
        writeln!(buf, r#"<th>Pad distance</th>"#).unwrap();
        for bond in bonds.values() {
            let names = bond.names.join("<br>");
            writeln!(buf, r#"<th>{names}</th>"#).unwrap();
        }
        writeln!(buf, r#"</tr>"#).unwrap();
        writeln!(buf, r#"</thead>"#).unwrap();
        writeln!(buf, r#"<tbody>"#).unwrap();
        writeln!(buf, r#"<tr>"#).unwrap();
        writeln!(buf, r#"<td>IDCODE part</td>"#).unwrap();
        writeln!(buf, r#"<td></td>"#).unwrap();
        writeln!(buf, r#"<td></td>"#).unwrap();
        for bondid in bonds.ids() {
            let bond = &db.bonds[bondid];
            writeln!(buf, r#"<td>{:#06x}</td>"#, bond.idcode_part).unwrap();
        }
        writeln!(buf, r#"</tr>"#).unwrap();
        for (&io, io_data) in &chip.io {
            let pin = match io {
                IoCoord::Ipad(ipad) => BondPad::Ipad(ipad),
                IoCoord::Macrocell(mc) => BondPad::Iob(mc),
            };
            writeln!(buf, r#"<tr>"#).unwrap();
            writeln!(buf, r#"<td>{pin}</td>"#).unwrap();
            writeln!(buf, r#"<td>{bank}</td>"#, bank = io_data.bank).unwrap();
            writeln!(buf, r#"<td>{dist}</td>"#, dist = io_data.pad_distance).unwrap();
            for bond in bonds.values() {
                if let Some(pin_data) = bond.pins.get(&pin) {
                    let pins = pin_data.pins.join(", ");
                    if let Some(spec) = io_special_rev.get(&pin) {
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
        specs.extend([BondPad::VccAux, BondPad::Nc]);

        for pin in specs {
            writeln!(buf, r#"<tr>"#).unwrap();
            writeln!(buf, r#"<td>{pin}</td>"#).unwrap();
            let bank = match pin {
                BondPad::Tck | BondPad::Tms | BondPad::Tdi | BondPad::Tdo | BondPad::VccAux => {
                    "AUX".to_string()
                }
                BondPad::VccIo(bank) => bank.to_string(),
                _ => "-".to_string(),
            };
            writeln!(buf, r#"<td>{bank}</td>"#).unwrap();
            writeln!(buf, r#"<td></td>"#).unwrap();
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

        gen_tile(ctx, &parts[0].name, "mc", &chip.mc_bits, orientation);
        writeln!(buf, r#"## Macrocell bits"#).unwrap();
        writeln!(buf).unwrap();
        let item = ctx
            .items
            .remove(&format!("tile-{pname}-mc", pname = parts[0].name))
            .unwrap();
        buf.push_str(&item);
        writeln!(buf).unwrap();

        if chip.has_vref {
            gen_jed(
                ctx,
                &parts[0].name,
                "mc",
                &chip.mc_bits,
                "mc-iob",
                &db.jed_mc_bits_large_iob,
            );
            writeln!(buf, r#"### JED mapping (MCs with IOBs)"#).unwrap();
            writeln!(buf).unwrap();
            let item = ctx
                .items
                .remove(&format!("jed-{pname}-mc-iob", pname = parts[0].name))
                .unwrap();
            buf.push_str(&item);
            writeln!(buf).unwrap();

            gen_jed(
                ctx,
                &parts[0].name,
                "mc",
                &chip.mc_bits,
                "mc-buried",
                &db.jed_mc_bits_large_buried,
            );
            writeln!(buf, r#"### JED mapping (MCs without IOBs)"#).unwrap();
            writeln!(buf).unwrap();
            let item = ctx
                .items
                .remove(&format!("jed-{pname}-mc-buried", pname = parts[0].name))
                .unwrap();
            buf.push_str(&item);
            writeln!(buf).unwrap();
        } else {
            gen_jed(
                ctx,
                &parts[0].name,
                "mc",
                &chip.mc_bits,
                "mc",
                &db.jed_mc_bits_small,
            );
            writeln!(buf, r#"### JED mapping"#).unwrap();
            writeln!(buf).unwrap();
            let item = ctx
                .items
                .remove(&format!("jed-{pname}-mc", pname = parts[0].name))
                .unwrap();
            buf.push_str(&item);
            writeln!(buf).unwrap();
        }

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
            .entry("coolrunner2/devices/index.md".into())
            .or_default()
            .push((
                format!("coolrunner2/devices/{pname}.md", pname = parts[0].name),
                names,
                buf,
            ));
    }
}

pub fn gen_coolrunner2(ctx: &mut DocgenContext) {
    let db = prjcombine_coolrunner2::Database::from_file(
        ctx.ctx.root.join("../databases/coolrunner2.zstd"),
    )
    .unwrap();
    gen_devlist(ctx, &db);
    gen_devpkg(ctx, &db);
    gen_devices(ctx, &db);
}
