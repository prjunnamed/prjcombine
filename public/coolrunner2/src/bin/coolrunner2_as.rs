use std::{
    collections::BTreeMap,
    error::Error,
    fs::{File, read_to_string},
    io::Write,
    path::{Path, PathBuf},
};

use bitvec::vec::BitVec;
use clap::Parser;
use prjcombine_coolrunner2::{BitCoord, Chip, Database};
use prjcombine_types::{
    FbId, FbMcId, IoId,
    tiledb::{Tile, TileItemKind},
};
use unnamed_entity::EntityId;

struct Bitstream {
    fbs: Vec<FbData>,
    globals: BTreeMap<String, BitVec>,
}

struct FbData {
    imux: BTreeMap<String, BitVec>,
    mcs: [BTreeMap<String, BitVec>; 16],
    pla_and: [PTermData; 56],
    pla_or: [BitVec; 16],
}

struct PTermData {
    im_t: BitVec,
    im_f: BitVec,
}

fn init_tile(tile: &Tile<BitCoord>) -> BTreeMap<String, BitVec> {
    tile.items
        .iter()
        .map(|(k, v)| (k.clone(), BitVec::repeat(true, v.bits.len())))
        .collect()
}

impl Bitstream {
    fn new(chip: &Chip) -> Self {
        let fbs = (0..(chip.fb_rows as usize * chip.fb_cols.len() * 2))
            .map(|_| FbData {
                imux: init_tile(&chip.imux_bits),
                mcs: core::array::from_fn(|_| init_tile(&chip.mc_bits)),
                pla_and: core::array::from_fn(|_| PTermData {
                    im_t: BitVec::repeat(false, 40),
                    im_f: BitVec::repeat(false, 40),
                }),
                pla_or: core::array::from_fn(|_| BitVec::repeat(false, 56)),
            })
            .collect();
        Bitstream {
            fbs,
            globals: init_tile(&chip.global_bits),
        }
    }

    fn to_jed(&self, chip: &Chip, db: &Database) -> BitVec {
        let mut res = BitVec::new();
        for (fb, fbd) in self.fbs.iter().enumerate() {
            for i in 0..40 {
                let n = format!("IM[{i}].MUX");
                let val = &fbd.imux[&n];
                res.extend(val);
            }
            for i in 0..56 {
                let pt = &fbd.pla_and[i];
                for j in 0..40 {
                    res.push(!pt.im_t[j]);
                    res.push(!pt.im_f[j]);
                }
            }
            for i in 0..56 {
                for j in 0..16 {
                    res.push(!fbd.pla_or[j][i]);
                }
            }
            for mc in 0..16 {
                let iobful = chip
                    .io
                    .contains_key(&IoId::Mc((FbId::from_idx(fb), FbMcId::from_idx(mc))));
                let mcd = &fbd.mcs[mc];
                let jed_bits = if !chip.has_vref {
                    &db.jed_mc_bits_small
                } else if iobful {
                    &db.jed_mc_bits_large_iob
                } else {
                    &db.jed_mc_bits_large_buried
                };
                for (bn, bi) in jed_bits {
                    res.push(mcd[bn][*bi]);
                }
            }
        }
        for (bn, bi) in &chip.jed_global_bits {
            res.push(self.globals[bn][*bi]);
        }
        res
    }
}

fn set_tile_item(data: &mut BTreeMap<String, BitVec>, tile: &Tile<BitCoord>, item: &str) {
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
        match item.kind {
            TileItemKind::Enum { .. } => unreachable!(),
            TileItemKind::BitVec { ref invert } => {
                assert_eq!(item.bits.len(), 1);
                data.insert(name.to_string(), BitVec::repeat(val ^ invert[0], 1));
            }
        }
    }
}

#[derive(Parser)]
struct Args {
    db: PathBuf,
    src: PathBuf,
    jed: PathBuf,
}

fn write_jed(fname: impl AsRef<Path>, dev: &str, bits: &BitVec) -> Result<(), Box<dyn Error>> {
    let mut f = File::create(fname)?;
    writeln!(f, "\x02QF{n}*", n = bits.len())?;
    writeln!(f, "F0*")?;
    writeln!(f, "N DEVICE {dev}*")?;
    for (i, c) in bits.chunks(80).enumerate() {
        write!(f, "L{ii:06} ", ii = i * 80)?;
        for bit in c {
            write!(f, "{x}", x = u32::from(*bit))?;
        }
        writeln!(f, "*")?;
    }
    writeln!(f, "\x030000")?;
    Ok(())
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let src = read_to_string(args.src)?;
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
    let db = Database::from_file(args.db)?;
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
    let mut bs = Bitstream::new(chip);
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
                    set_tile_item(&mut bs.fbs[fb].imux, &chip.imux_bits, item);
                }
            }
            ["MC", fb, mc] => {
                let fb: usize = fb.parse()?;
                let mc: usize = mc.parse()?;
                for item in suf {
                    set_tile_item(&mut bs.fbs[fb].mcs[mc], &chip.mc_bits, item);
                }
            }
            ["PT", fb, pt] => {
                let fb: usize = fb.parse()?;
                let pt: usize = pt.parse()?;
                let pt = &mut bs.fbs[fb].pla_and[pt];
                for item in suf {
                    if let Some(idx) = item.strip_prefix('!') {
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
    let fuses = bs.to_jed(chip, &db);
    write_jed(args.jed, dev, &fuses)?;

    Ok(())
}
