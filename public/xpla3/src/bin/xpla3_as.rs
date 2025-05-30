use std::{collections::BTreeMap, error::Error, fs::read_to_string, path::PathBuf};

use bitvec::vec::BitVec;
use clap::{Arg, Command, value_parser};
use prjcombine_jed::JedFile;
use prjcombine_types::{
    FbMcId,
    bsdata::{Tile, TileItemKind},
};
use prjcombine_xpla3::{Chip, Database};
use unnamed_entity::EntityId;

struct Bitstream {
    fbs: Vec<FbData>,
    globals: BTreeMap<String, BitVec>,
}

struct FbData {
    misc: BTreeMap<String, BitVec>,
    mcs: [BTreeMap<String, BitVec>; 16],
    pla_and: [PTermData; 48],
    pla_or: [BitVec; 16],
}

struct PTermData {
    im_t: BitVec,
    im_f: BitVec,
    fbn: BitVec,
}

fn init_tile(tile: &Tile) -> BTreeMap<String, BitVec> {
    tile.items
        .iter()
        .map(|(k, v)| (k.clone(), BitVec::repeat(true, v.bits.len())))
        .collect()
}

impl Bitstream {
    fn new(chip: &Chip, db: &Database) -> Self {
        let fbs = (0..(chip.fb_rows * chip.fb_cols.len() * 2))
            .map(|_| {
                let mut misc = init_tile(&db.fb_bits);
                for i in 0..40 {
                    misc.insert(
                        format!("IM[{i}].MUX"),
                        BitVec::repeat(true, chip.imux_width),
                    );
                }
                FbData {
                    misc,
                    mcs: core::array::from_fn(|_| init_tile(&db.mc_bits)),
                    pla_and: core::array::from_fn(|_| PTermData {
                        im_t: BitVec::repeat(false, 40),
                        im_f: BitVec::repeat(false, 40),
                        fbn: BitVec::repeat(false, 8),
                    }),
                    pla_or: core::array::from_fn(|_| BitVec::repeat(false, 48)),
                }
            })
            .collect();
        Bitstream {
            fbs,
            globals: init_tile(&chip.global_bits),
        }
    }

    fn to_jed(&self, chip: &Chip, db: &Database, device: &str) -> JedFile {
        let mut res = BitVec::new();
        for fbd in &self.fbs {
            for i in 0..40 {
                let n = format!("IM[{i}].MUX");
                let val = &fbd.misc[&n];
                res.extend(val);
            }
            for i in 0..48 {
                let pt = &fbd.pla_and[i];
                for j in 0..40 {
                    res.push(!pt.im_t[j]);
                    res.push(!pt.im_f[j]);
                }
                for j in 0..8 {
                    res.push(!pt.fbn[j]);
                }
            }
            for i in 0..48 {
                for j in 0..16 {
                    res.push(!fbd.pla_or[j][i]);
                }
            }
            for (bn, bi) in &db.jed_fb_bits {
                res.push(fbd.misc[bn][*bi]);
            }
            for iobful in [true, false] {
                for mc in 0..16 {
                    if chip.io_mcs.contains(&FbMcId::from_idx(mc)) != iobful {
                        continue;
                    }
                    let mcd = &fbd.mcs[mc];
                    let jed_bits = if iobful {
                        &db.jed_mc_bits_iob
                    } else {
                        &db.jed_mc_bits_buried
                    };
                    for (bn, bi) in jed_bits {
                        res.push(mcd[bn][*bi]);
                    }
                }
            }
        }
        for (bn, bi) in &chip.jed_global_bits {
            res.push(self.globals[bn][*bi]);
        }
        JedFile::new()
            .with_fuses(res)
            .with_note(format!(" DEVICE {device}"))
    }
}

fn set_tile_item(data: &mut BTreeMap<String, BitVec>, tile: &Tile, item: &str) {
    if let Some((name, val)) = item.split_once('=') {
        let item = &tile.items[name];
        let val = match &item.kind {
            TileItemKind::Enum { values } => values[val].clone(),
            TileItemKind::BitVec { invert } => {
                assert_eq!(val.len(), item.bits.len());
                val.chars()
                    .rev()
                    .enumerate()
                    .map(|(i, x)| match x {
                        '0' => invert[i],
                        '1' => !invert[i],
                        _ => unreachable!(),
                    })
                    .collect()
            }
        };
        data.insert(name.to_string(), val);
    } else {
        let (name, val) = if let Some(name) = item.strip_prefix('!') {
            (name, false)
        } else {
            (item, true)
        };
        let item = &tile.items[name];
        match &item.kind {
            TileItemKind::Enum { .. } => unreachable!(),
            TileItemKind::BitVec { invert } => {
                assert_eq!(item.bits.len(), 1);
                data.insert(name.to_string(), BitVec::repeat(val ^ invert[0], 1));
            }
        }
    }
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let m = Command::new("xcpla3_as")
        .arg(
            Arg::new("db")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("src")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("jed")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .get_matches();
    let arg_db = m.get_one::<PathBuf>("db").unwrap();
    let arg_src = m.get_one::<PathBuf>("src").unwrap();
    let arg_jed = m.get_one::<PathBuf>("jed").unwrap();
    let src = read_to_string(arg_src)?;
    let mut lines = src.lines();
    let mut dev = None;
    for mut line in &mut lines {
        if let Some(pos) = line.find('#') {
            line = &line[..pos];
        }
        line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (pref, suf) = line.split_once(':').unwrap();
        let suf = suf.trim();
        assert_eq!(pref, "DEVICE");
        dev = Some(suf);
        break;
    }
    let dev = dev.unwrap();
    let db = Database::from_file(arg_db)?;
    let mut part = None;
    for p in &db.parts {
        if p.name == dev {
            part = Some(p);
            break;
        }
    }
    let Some(part) = part else {
        eprintln!("Unknown device {dev}");
        return Ok(());
    };
    let chip = &db.chips[part.chip];
    let mut bs = Bitstream::new(chip, &db);
    let mut fb_bits = db.fb_bits.clone();
    for (k, v) in chip.imux_bits.clone().items {
        fb_bits.items.insert(k, v);
    }
    for mut line in lines {
        if let Some(pos) = line.find('#') {
            line = &line[..pos];
        }
        line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (pref, suf) = line.split_once(':').unwrap();
        let pref: Vec<_> = pref.split_ascii_whitespace().collect();
        let suf: Vec<_> = suf.trim().split_ascii_whitespace().collect();
        match pref[..] {
            ["GLOBAL"] => {
                for item in suf {
                    set_tile_item(&mut bs.globals, &chip.global_bits, item);
                }
            }
            ["FB", fb] => {
                let fb: usize = fb.parse()?;
                for item in suf {
                    set_tile_item(&mut bs.fbs[fb].misc, &fb_bits, item);
                }
            }
            ["MC", fb, mc] => {
                let fb: usize = fb.parse()?;
                let mc: usize = mc.parse()?;
                for item in suf {
                    set_tile_item(&mut bs.fbs[fb].mcs[mc], &db.mc_bits, item);
                }
            }
            ["PT", fb, pt] => {
                let fb: usize = fb.parse()?;
                let pt: usize = pt.parse()?;
                let pt = &mut bs.fbs[fb].pla_and[pt];
                for item in suf {
                    if let Some(idx) = item.strip_prefix('F') {
                        let idx = idx.parse()?;
                        pt.fbn.set(idx, true);
                    } else if let Some(idx) = item.strip_prefix('!') {
                        let idx = idx.parse()?;
                        pt.im_f.set(idx, true);
                    } else {
                        let idx = item.parse()?;
                        pt.im_t.set(idx, true);
                    }
                }
            }
            ["ST", fb, mc] => {
                let fb: usize = fb.parse()?;
                let mc: usize = mc.parse()?;
                for item in suf {
                    let idx = item.parse()?;
                    bs.fbs[fb].pla_or[mc].set(idx, true);
                }
            }

            _ => panic!("weird line {line}"),
        }
    }
    let jed = bs.to_jed(chip, &db, dev);
    jed.emit_to_file(arg_jed)?;

    Ok(())
}
