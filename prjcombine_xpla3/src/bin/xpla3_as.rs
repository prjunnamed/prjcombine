use std::{
    collections::BTreeMap,
    error::Error,
    fs::{read_to_string, File},
    io::Write,
    path::{Path, PathBuf},
};

use bitvec::vec::BitVec;
use clap::Parser;
use prjcombine_xpla3::{BitCoord, Database, Device, FbMcId, Tile, TileItem};
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

fn init_tile(tile: &Tile<BitCoord>) -> BTreeMap<String, BitVec> {
    tile.items
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                match v {
                    TileItem::Enum(item) => BitVec::repeat(true, item.bits.len()),
                    TileItem::BitVec(item) => BitVec::repeat(true, item.bits.len()),
                },
            )
        })
        .collect()
}

impl Bitstream {
    fn new(device: &Device, db: &Database) -> Self {
        let fbs = (0..(device.fb_rows as usize * device.fb_cols.len() * 2))
            .map(|_| {
                let mut misc = init_tile(&db.fb_bits);
                for i in 0..40 {
                    misc.insert(
                        format!("IM[{i}].MUX"),
                        BitVec::repeat(true, device.imux_width as usize),
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
            globals: init_tile(&device.global_bits),
        }
    }

    fn to_jed(&self, device: &Device, db: &Database) -> BitVec {
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
                    if device.io_mcs.contains(&FbMcId::from_idx(mc)) != iobful {
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
        for (bn, bi) in &device.jed_global_bits {
            res.push(self.globals[bn][*bi]);
        }
        res
    }
}

fn set_tile_item(data: &mut BTreeMap<String, BitVec>, tile: &Tile<BitCoord>, item: &str) {
    if let Some((name, val)) = item.split_once('=') {
        let item = &tile.items[name];
        let val = match item {
            TileItem::Enum(item) => item.values[val].clone(),
            TileItem::BitVec(item) => {
                assert_eq!(val.len(), item.bits.len());
                val.chars()
                    .rev()
                    .map(|x| match x {
                        '0' => item.invert,
                        '1' => !item.invert,
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
        match item {
            TileItem::Enum(_) => unreachable!(),
            TileItem::BitVec(item) => {
                assert_eq!(item.bits.len(), 1);
                data.insert(name.to_string(), BitVec::repeat(val ^ item.invert, 1));
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
    let device = &db.devices[part.device];
    let mut bs = Bitstream::new(device, &db);
    let mut fb_bits = db.fb_bits.clone();
    for (k, v) in device.imux_bits.clone().items {
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
                    set_tile_item(&mut bs.globals, &device.global_bits, item);
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
    let fuses = bs.to_jed(device, &db);
    write_jed(args.jed, dev, &fuses)?;

    Ok(())
}
