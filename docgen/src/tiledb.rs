use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use bitvec::vec::BitVec;
use indexmap::IndexMap;
use itertools::Itertools;
use prjcombine_types::bsdata::{DbValue, Tile, TileBit, BsData, TileItemKind};

use crate::DocgenContext;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FrameDirection {
    Horizontal,
    Vertical,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TileOrientation {
    pub frame_direction: FrameDirection,
    pub flip_frame: bool,
    pub flip_bit: bool,
}

pub fn gen_tile(
    ctx: &mut DocgenContext,
    dbname: &str,
    tname: &str,
    tile: &Tile,
    orientation: TileOrientation,
) {
    let mut dims: Vec<(usize, usize)> = vec![];
    let mut reverse: HashMap<_, Vec<_>> = HashMap::new();
    let mut buf = String::new();
    let mut item_data: IndexMap<_, Vec<_>> = IndexMap::new();
    for (iname, item) in &tile.items {
        for (bidx, &bit) in item.bits.iter().enumerate() {
            while bit.tile >= dims.len() {
                dims.push((0, 0));
            }
            dims[bit.tile].0 = std::cmp::max(dims[bit.tile].0, bit.frame + 1);
            dims[bit.tile].1 = std::cmp::max(dims[bit.tile].1, bit.bit + 1);
            let (bidx, invert) = if let TileItemKind::BitVec { ref invert } = item.kind {
                (
                    if invert.len() == 1 { None } else { Some(bidx) },
                    invert[bidx],
                )
            } else {
                (Some(bidx), false)
            };
            reverse.entry(bit).or_default().push((iname, bidx, invert));
        }
        item_data
            .entry(&item.kind)
            .or_default()
            .push((iname, &item.bits));
    }
    for (tidx, &(num_frames, num_bits)) in dims.iter().enumerate() {
        let frames = if orientation.flip_frame {
            Vec::from_iter((0..num_frames).rev())
        } else {
            Vec::from_iter(0..num_frames)
        };
        let bits = if orientation.flip_bit {
            Vec::from_iter((0..num_bits).rev())
        } else {
            Vec::from_iter(0..num_bits)
        };
        writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
        writeln!(buf, r#"<caption>{dbname} {tname} bittile {tidx}</caption>"#).unwrap();
        writeln!(buf, r#"<thead>"#).unwrap();
        match orientation.frame_direction {
            FrameDirection::Horizontal => {
                writeln!(
                    buf,
                    r#"<tr><th rowspan="2">Frame</th><th colspan="{num_bits}">Bit</th></tr>"#
                )
                .unwrap();
                writeln!(buf, r#"<tr>"#).unwrap();
                for &bit in &bits {
                    writeln!(buf, r#"<th>{bit}</th>"#).unwrap();
                }
                writeln!(buf, r#"</tr>"#).unwrap();
            }
            FrameDirection::Vertical => {
                writeln!(
                    buf,
                    r#"<tr><th rowspan="2">Bit</th><th colspan="{num_frames}">Frame</th></tr>"#
                )
                .unwrap();
                writeln!(buf, r#"<tr>"#).unwrap();
                for &frame in &frames {
                    writeln!(buf, r#"<th>{frame}</th>"#).unwrap();
                }
                writeln!(buf, r#"</tr>"#).unwrap();
            }
        }
        writeln!(buf, r#"</thead>"#).unwrap();
        writeln!(buf, r#"<tbody>"#).unwrap();
        let emit_bit = |buf: &mut String, tbit: TileBit| {
            let TileBit { tile, frame, bit } = tbit;
            if let Some(items) = reverse.get(&tbit) {
                writeln!(buf, r#"<td title="{tile}.{frame}.{bit}">"#).unwrap();
                for &(iname, bidx, invert) in items {
                    let inv = if invert { "~" } else { "" };
                    let bidx = if let Some(bidx) = bidx {
                        format!("[{bidx}]")
                    } else {
                        "".into()
                    };
                    writeln!(
                        buf,
                        r##"<a href="#tile-{dbname}-{tname}-{iname}">{inv}{iname}{bidx}</a>"##
                    )
                    .unwrap();
                }
                writeln!(buf, r#"</td>"#).unwrap();
            } else {
                writeln!(buf, r#"<td>-</td>"#).unwrap();
            }
        };
        match orientation.frame_direction {
            FrameDirection::Horizontal => {
                for &frame in &frames {
                    writeln!(buf, r#"<tr><td>{frame}</td>"#).unwrap();
                    for &bit in &bits {
                        emit_bit(
                            &mut buf,
                            TileBit {
                                tile: tidx,
                                frame,
                                bit,
                            },
                        );
                    }
                    writeln!(buf, r#"</tr>"#).unwrap();
                }
            }
            FrameDirection::Vertical => {
                for &bit in &bits {
                    writeln!(buf, r#"<tr><td>{bit}</td>"#).unwrap();
                    for &frame in &frames {
                        emit_bit(
                            &mut buf,
                            TileBit {
                                tile: tidx,
                                frame,
                                bit,
                            },
                        );
                    }
                    writeln!(buf, r#"</tr>"#).unwrap();
                }
            }
        }
        writeln!(buf, r#"</tbody>"#).unwrap();
        writeln!(buf, r#"</table></div>"#).unwrap();
    }
    for (kind, items) in item_data {
        writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
        writeln!(buf, r#"<thead>"#).unwrap();
        for (iname, ibits) in items {
            writeln!(
                buf,
                r#"<tr><th id="tile-{dbname}-{tname}-{iname}">{iname}</th>"#
            )
            .unwrap();
            for &bit in ibits.iter().rev() {
                writeln!(
                    buf,
                    "<th>{tile}.{frame}.{bit}</th>",
                    tile = bit.tile,
                    frame = bit.frame,
                    bit = bit.bit
                )
                .unwrap();
            }
            writeln!(buf, r#"</tr>"#).unwrap();
        }
        writeln!(buf, r#"</thead>"#).unwrap();
        writeln!(buf, r#"<tbody>"#).unwrap();
        match kind {
            TileItemKind::Enum { values } => {
                for (vname, value) in values
                    .iter()
                    .sorted_by_key(|&(_, val)| -> BitVec { BitVec::from_iter(val.iter().rev()) })
                {
                    writeln!(buf, r#"<tr><td>{vname}</td>"#).unwrap();
                    for vbit in value.iter().rev() {
                        let vbit = u8::from(*vbit);
                        writeln!(buf, r#"<td>{vbit}</td>"#).unwrap();
                    }
                    writeln!(buf, r#"</tr>"#).unwrap();
                }
            }
            TileItemKind::BitVec { invert } => {
                writeln!(buf, r#"<tr><td>"#).unwrap();
                if invert.all() {
                    writeln!(buf, r#"inverted"#).unwrap();
                } else if !invert.any() {
                    writeln!(buf, r#"non-inverted"#).unwrap();
                } else {
                    writeln!(buf, r#"mixed inversion"#).unwrap();
                }
                writeln!(buf, r#"</td>"#).unwrap();
                for (idx, inv) in invert.iter().enumerate().rev() {
                    if *inv {
                        writeln!(buf, r#"<td>~[{idx}]</td>"#).unwrap();
                    } else {
                        writeln!(buf, r#"<td>[{idx}]</td>"#).unwrap();
                    }
                }
                writeln!(buf, r#"</tr>"#).unwrap();
            }
        }
        writeln!(buf, r#"</tbody>"#).unwrap();
        writeln!(buf, r#"</table></div>"#).unwrap();
    }
    ctx.items.insert(format!("tile-{dbname}-{tname}"), buf);
}

pub fn gen_tiles(
    ctx: &mut DocgenContext,
    dbname: &str,
    tiledb: &BsData,
    orientation: impl Fn(&str) -> TileOrientation,
) {
    for (tname, tile) in &tiledb.tiles {
        gen_tile(ctx, dbname, tname, tile, orientation(tname));
    }
}

pub fn gen_misc_table(
    ctx: &mut DocgenContext,
    tiledb: &BsData,
    misc_used: &mut HashSet<String>,
    dbname: &str,
    tname: &str,
    prefixes: &[&str],
) {
    let mut kvs = vec![];
    let pref0 = format!("{}:", prefixes[0]);
    let mut lens = None;
    for name in tiledb.misc_data.keys() {
        let Some(name) = name.strip_prefix(&pref0) else {
            continue;
        };
        let mut data = vec![];
        let mut cur_lens = vec![];
        for &pref in prefixes {
            let full_name = format!("{pref}:{name}");
            let val = &tiledb.misc_data[&full_name];
            misc_used.insert(full_name);
            if let DbValue::BitVec(bv) = val {
                cur_lens.push(Some(bv.len()));
            } else {
                cur_lens.push(None);
            }
            data.push(val);
        }
        kvs.push((name, data));
        if let Some(ref lens) = lens {
            assert_eq!(*lens, cur_lens);
        } else {
            lens = Some(cur_lens)
        }
    }
    let lens = lens.unwrap();
    let mut buf = String::new();
    writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
    writeln!(buf, r#"<thead>"#).unwrap();
    writeln!(buf, r#"<tr><th rowspan="2">Name</th>"#).unwrap();
    for (&pref, &l) in prefixes.iter().zip(lens.iter()) {
        match l {
            None => {
                writeln!(buf, r#"<th rowspan="2">{pref}</th>"#).unwrap();
            }
            Some(l) => {
                writeln!(buf, r#"<th colspan="{l}">{pref}</th>"#).unwrap();
            }
        }
    }
    writeln!(buf, r#"</tr>"#).unwrap();
    writeln!(buf, r#"<tr>"#).unwrap();
    for &l in &lens {
        if let Some(l) = l {
            for i in (0..l).rev() {
                writeln!(buf, r#"<th>[{i}]</th>"#).unwrap();
            }
        }
    }
    writeln!(buf, r#"</tr>"#).unwrap();
    writeln!(buf, r#"</thead>"#).unwrap();
    writeln!(buf, r#"<tbody>"#).unwrap();
    for (name, data) in kvs {
        writeln!(buf, r#"<tr><td>{name}</td>"#).unwrap();
        for (val, &l) in data.into_iter().zip(lens.iter()) {
            match l {
                None => match val {
                    DbValue::String(v) => {
                        writeln!(buf, r#"<td>{v}</td>"#).unwrap();
                    }
                    DbValue::BitVec(_) => unreachable!(),
                    DbValue::Int(v) => {
                        writeln!(buf, r#"<td>{v}</td>"#).unwrap();
                    }
                },
                Some(l) => {
                    let DbValue::BitVec(bv) = val else {
                        unreachable!()
                    };
                    assert_eq!(bv.len(), l);
                    for bit in bv.iter().rev() {
                        let bit = u8::from(*bit);
                        writeln!(buf, r#"<td>{bit}</td>"#).unwrap();
                    }
                }
            }
        }
        writeln!(buf, r#"</tr>"#).unwrap();
    }
    writeln!(buf, r#"</tbody>"#).unwrap();
    writeln!(buf, r#"</table></div>"#).unwrap();
    ctx.items.insert(format!("misc-{dbname}-{tname}"), buf);
}

pub fn check_misc_data(tiledb: &BsData, dbname: &str, misc_used: &HashSet<String>) {
    for key in tiledb.misc_data.keys() {
        if !misc_used.contains(key) && !key.starts_with("INTF.DSP") {
            eprintln!("WARNING: unused misc data {dbname} {key}");
        }
    }
}

pub fn gen_devdata_table(
    ctx: &mut DocgenContext,
    tiledb: &BsData,
    part_names: &[&str],
    devdata_used: &mut HashSet<String>,
    dbname: &str,
    tname: &str,
    keys: &[&str],
) {
    let mut kvs = vec![];
    let mut lens = vec![None; keys.len()];
    for &key in keys {
        devdata_used.insert(key.into());
    }
    for (dev, devdata) in &tiledb.device_data {
        let mut data = vec![];
        for (idx, &key) in keys.iter().enumerate() {
            if let Some(val) = devdata.get(key) {
                let l = if let DbValue::BitVec(bv) = val {
                    Some(bv.len())
                } else {
                    None
                };
                if let Some(cur_l) = lens[idx] {
                    assert_eq!(l, cur_l);
                } else {
                    lens[idx] = Some(l);
                }
                data.push(Some(val));
            } else {
                data.push(None);
            }
        }
        kvs.push((dev, data));
    }
    kvs.sort_by_key(|&(dev, _)| part_names.iter().position(|&pn| pn == dev));
    let lens = Vec::from_iter(lens.into_iter().map(Option::unwrap));
    let mut buf = String::new();
    writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
    writeln!(buf, r#"<thead>"#).unwrap();
    writeln!(buf, r#"<tr><th rowspan="2">Device</th>"#).unwrap();
    for (&key, &l) in keys.iter().zip(lens.iter()) {
        match l {
            None => {
                writeln!(buf, r#"<th rowspan="2">{key}</th>"#).unwrap();
            }
            Some(l) => {
                writeln!(buf, r#"<th colspan="{l}">{key}</th>"#).unwrap();
            }
        }
    }
    writeln!(buf, r#"</tr>"#).unwrap();
    writeln!(buf, r#"<tr>"#).unwrap();
    for &l in &lens {
        if let Some(l) = l {
            for i in (0..l).rev() {
                writeln!(buf, r#"<th>[{i}]</th>"#).unwrap();
            }
        }
    }
    writeln!(buf, r#"</tr>"#).unwrap();
    writeln!(buf, r#"</thead>"#).unwrap();
    writeln!(buf, r#"<tbody>"#).unwrap();
    for (name, data) in kvs {
        writeln!(buf, r#"<tr><td>{name}</td>"#).unwrap();
        for (val, &l) in data.into_iter().zip(lens.iter()) {
            match l {
                None => match val {
                    Some(DbValue::String(v)) => {
                        writeln!(buf, r#"<td>{v}</td>"#).unwrap();
                    }
                    Some(DbValue::BitVec(_)) => unreachable!(),
                    Some(DbValue::Int(v)) => {
                        writeln!(buf, r#"<td>{v}</td>"#).unwrap();
                    }
                    None => {
                        writeln!(buf, r#"<td>-</td>"#).unwrap();
                    }
                },
                Some(l) => {
                    if let Some(DbValue::BitVec(bv)) = val {
                        assert_eq!(bv.len(), l);
                        for bit in bv.iter().rev() {
                            let bit = u8::from(*bit);
                            writeln!(buf, r#"<td>{bit}</td>"#).unwrap();
                        }
                    } else {
                        for _ in 0..l {
                            writeln!(buf, r#"<td>-</td>"#).unwrap();
                        }
                    }
                }
            }
        }
        writeln!(buf, r#"</tr>"#).unwrap();
    }
    writeln!(buf, r#"</tbody>"#).unwrap();
    writeln!(buf, r#"</table></div>"#).unwrap();
    ctx.items.insert(format!("devdata-{dbname}-{tname}"), buf);
}

pub fn check_devdata(tiledb: &BsData, dbname: &str, devdata_used: &HashSet<String>) {
    let mut warned = HashSet::new();
    for data in tiledb.device_data.values() {
        for key in data.keys() {
            // TODO: deal with IDCODE properly.
            if !devdata_used.contains(key) && !warned.contains(key) && !key.starts_with("IDCODE") {
                eprintln!("WARNING: unused devdata {dbname} {key}");
                warned.insert(key.clone());
            }
        }
    }
}
