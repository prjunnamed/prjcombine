use std::{collections::BTreeMap, error::Error, path::PathBuf};

use bitvec::vec::BitVec;
use clap::Parser;
use prjcombine_types::{
    tiledb::{Tile, TileItemKind},
    FbMcId,
};
use prjcombine_xpla3::{BitCoord, Database, Device};
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

impl Bitstream {
    fn from_jed(fuses: &BitVec, device: &Device, db: &Database) -> Self {
        let mut fbs = vec![];
        let mut pos = 0;
        for _ in 0..(device.fb_cols.len() * device.fb_rows as usize * 2) {
            let mut fbd = FbData {
                misc: BTreeMap::new(),
                mcs: core::array::from_fn(|_| BTreeMap::new()),
                pla_and: core::array::from_fn(|_| PTermData {
                    im_t: BitVec::new(),
                    im_f: BitVec::new(),
                    fbn: BitVec::new(),
                }),
                pla_or: core::array::from_fn(|_| BitVec::new()),
            };
            for i in 0..40 {
                let n = format!("IM[{i}].MUX");
                let data = fuses[pos..(pos + device.imux_width as usize)].to_bitvec();
                pos += device.imux_width as usize;
                fbd.misc.insert(n, data);
            }
            for i in 0..48 {
                let pt = &mut fbd.pla_and[i];
                for _ in 0..40 {
                    pt.im_t.push(!fuses[pos]);
                    pos += 1;
                    pt.im_f.push(!fuses[pos]);
                    pos += 1;
                }
                for _ in 0..8 {
                    pt.fbn.push(!fuses[pos]);
                    pos += 1;
                }
            }
            for _ in 0..48 {
                for j in 0..16 {
                    fbd.pla_or[j].push(!fuses[pos]);
                    pos += 1;
                }
            }
            for (bn, bi) in &db.jed_fb_bits {
                let bits = fbd
                    .misc
                    .entry(bn.clone())
                    .or_insert_with(|| BitVec::repeat(false, db.fb_bits.items[bn].bits.len()));
                bits.set(*bi, fuses[pos]);
                pos += 1;
            }
            for iobful in [true, false] {
                for mc in 0..16 {
                    if device.io_mcs.contains(&FbMcId::from_idx(mc)) != iobful {
                        continue;
                    }
                    let mcd = &mut fbd.mcs[mc];
                    let jed_bits = if iobful {
                        &db.jed_mc_bits_iob
                    } else {
                        &db.jed_mc_bits_buried
                    };
                    for (bn, bi) in jed_bits {
                        let bits = mcd.entry(bn.clone()).or_insert_with(|| {
                            BitVec::repeat(false, db.mc_bits.items[bn].bits.len())
                        });
                        bits.set(*bi, fuses[pos]);
                        pos += 1;
                    }
                }
            }
            fbs.push(fbd);
        }
        let mut globals = BTreeMap::new();
        for (bn, bi) in &device.jed_global_bits {
            let bits = globals
                .entry(bn.clone())
                .or_insert_with(|| BitVec::repeat(false, device.global_bits.items[bn].bits.len()));
            bits.set(*bi, fuses[pos]);
            pos += 1;
        }
        assert_eq!(pos, fuses.len());
        Bitstream { fbs, globals }
    }
}

#[derive(Parser)]
struct Args {
    db: PathBuf,
    jed: PathBuf,
}

fn parse_jed(jed: &str) -> (String, BitVec) {
    let stx = jed.find('\x02').unwrap();
    let etx = jed.find('\x03').unwrap();
    let mut res = None;
    let mut len = None;
    let mut device = None;
    for cmd in jed[stx + 1..etx].split('*') {
        let cmd = cmd.trim();
        if let Some(arg) = cmd.strip_prefix("QF") {
            assert!(len.is_none());
            let n: usize = arg.parse().unwrap();
            len = Some(n);
        } else if let Some(arg) = cmd.strip_prefix("N DEVICE ") {
            device = Some(arg.to_string())
        } else if let Some(arg) = cmd.strip_prefix('F') {
            assert!(res.is_none());
            let x: u32 = arg.parse().unwrap();
            let x = match x {
                0 => false,
                1 => true,
                _ => unreachable!(),
            };
            res = Some(BitVec::repeat(x, len.unwrap()));
        } else if let Some(arg) = cmd.strip_prefix('L') {
            let sp = arg.find(' ').unwrap();
            let mut pos: usize = arg[..sp].parse().unwrap();
            let v = res.as_mut().unwrap();
            for c in arg[sp..].chars() {
                match c {
                    '0' => {
                        v.set(pos, false);
                        pos += 1;
                    }
                    '1' => {
                        v.set(pos, true);
                        pos += 1;
                    }
                    ' ' => (),
                    _ => unreachable!(),
                }
            }
        }
    }
    (device.unwrap(), res.unwrap())
}

fn print_tile(data: &BTreeMap<String, BitVec>, tile: &Tile<BitCoord>) {
    for (k, v) in data {
        let item = &tile.items[k];
        print!(" {k}=");
        match &item.kind {
            TileItemKind::Enum { values } => {
                let mut found = false;
                for (vn, val) in values {
                    if val == v {
                        found = true;
                        print!("{vn}");
                    }
                }
                if !found {
                    for bit in v.iter().rev() {
                        print!("{}", u8::from(*bit));
                    }
                }
            }
            TileItemKind::BitVec { invert } => {
                for (i, bit) in v.iter().enumerate().rev() {
                    print!("{}", u8::from(*bit ^ invert[i]));
                }
            }
        }
    }
    println!();
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let jed = std::fs::read_to_string(args.jed)?;
    let (device, fuses) = parse_jed(&jed);
    let device = device.to_ascii_lowercase();
    let dev = if let Some(pos) = device.find('-') {
        &device[..pos]
    } else {
        &device[..]
    };
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
    let bs = Bitstream::from_jed(&fuses, device, &db);
    println!("DEVICE: {dev}");
    print!("GLOBAL:");
    print_tile(&bs.globals, &device.global_bits);
    for (i, fbd) in bs.fbs.iter().enumerate() {
        print!("FB {i}:");
        let mut bits = db.fb_bits.clone();
        for (k, v) in &device.imux_bits.items {
            bits.items.insert(k.clone(), v.clone());
        }
        print_tile(&fbd.misc, &bits);
        for j in 0..16 {
            print!("MC {i} {j}:");
            print_tile(&fbd.mcs[j], &db.mc_bits);
        }
        for (j, pt) in fbd.pla_and.iter().enumerate() {
            if pt.im_t.any() || pt.im_f.any() || pt.fbn.any() {
                print!("PT {i} {j}:");
                for k in 0..40 {
                    if pt.im_t[k] {
                        print!(" {k}");
                    }
                    if pt.im_f[k] {
                        print!(" !{k}");
                    }
                }
                for k in 0..8 {
                    if pt.fbn[k] {
                        print!(" FBN{k}");
                    }
                }
                println!();
            }
        }
        for (j, st) in fbd.pla_or.iter().enumerate() {
            if st.any() {
                print!("ST {i} {j}:");
                for k in 0..48 {
                    if st[k] {
                        print!(" {k}");
                    }
                }
                println!();
            }
        }
    }
    Ok(())
}
