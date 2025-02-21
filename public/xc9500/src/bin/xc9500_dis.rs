use std::{error::Error, path::PathBuf};

use bitvec::vec::BitVec;
use clap::Parser;
use prjcombine_types::tiledb::{Tile, TileBit, TileItemKind};
use prjcombine_xc9500::{Chip, ChipKind, Database};

struct Bitstream {
    fbs: Vec<Vec<[u8; 15]>>,
    uim: Vec<Vec<Vec<[u8; 5]>>>,
}

impl Bitstream {
    fn from_jed(fuses: &BitVec, chip: &Chip) -> Self {
        let mut fbs = vec![];
        let mut uim = vec![];
        let mut pos = 0;
        if chip.kind == ChipKind::Xc9500 {
            for _ in 0..chip.fbs {
                let mut rows = vec![];
                for _ in 0..72 {
                    let mut row = [0; 15];
                    for col in 0..15 {
                        let sz = if col < 9 { 8 } else { 6 };
                        let f = &fuses[pos..pos + sz];
                        pos += sz;
                        for j in 0..sz {
                            if f[j] {
                                row[col] |= 1 << j;
                            }
                        }
                    }
                    rows.push(row);
                }
                fbs.push(rows);
                let mut uim_fb = vec![];
                for _ in 0..chip.fbs {
                    let mut rows = vec![];
                    for _ in 0..18 {
                        let mut row = [0; 5];
                        for col in 0..5 {
                            let sz = if col == 0 { 8 } else { 7 };
                            let f = &fuses[pos..pos + sz];
                            pos += sz;
                            for j in 0..sz {
                                if f[j] {
                                    row[col] |= 1 << j;
                                }
                            }
                        }
                        rows.push(row);
                    }
                    uim_fb.push(rows);
                }
                uim.push(uim_fb);
            }
        } else {
            for _ in 0..chip.fbs {
                fbs.push(vec![[0; 15]; 108]);
            }
            for row in 0..108 {
                for col in 0..15 {
                    for fb in 0..chip.fbs {
                        let sz = if col < 9 { 8 } else { 6 };
                        let f = &fuses[pos..pos + sz];
                        pos += sz;
                        for j in 0..sz {
                            if f[j] {
                                fbs[fb][row][col] |= 1 << j;
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(pos, fuses.len());
        Bitstream { fbs, uim }
    }

    fn get_bit(&self, fb: usize, row: usize, col: usize, bit: usize) -> bool {
        (self.fbs[fb][row][col] >> bit & 1) != 0
    }

    fn get_global(&self, crd: TileBit) -> bool {
        self.get_bit(crd.tile, crd.frame, crd.bit % 9, 6 + crd.bit / 9)
    }

    fn get_fb(&self, fb: usize, crd: TileBit) -> bool {
        self.get_bit(fb, crd.frame, crd.bit % 9, 6 + crd.bit / 9)
    }

    fn get_mc(&self, fb: usize, mc: usize, crd: TileBit) -> bool {
        self.get_bit(fb, crd.frame, mc % 9, 6 + mc / 9)
    }

    fn get_pt(&self, fb: usize, mc: usize, pt: usize, imux: usize, pol: bool) -> bool {
        self.get_bit(fb, imux * 2 + usize::from(pol), pt + (mc % 3) * 5, mc / 3)
    }

    fn get_uim(&self, fb: usize, sfb: usize, imux: usize, mc: usize) -> bool {
        (self.uim[fb][sfb][mc][imux % 5] >> (imux / 5) & 1) != 0
    }
}

#[derive(Parser)]
struct Args {
    dbdir: PathBuf,
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

fn print_tile(tile: &Tile, chip: &Chip, get_bit: impl Fn(TileBit) -> bool) {
    let is_large = chip.io_special.contains_key("GOE2");
    for (name, item) in &tile.items {
        let mut name = &name[..];
        if let Some(n) = name.strip_suffix(".SMALL") {
            if is_large {
                continue;
            }
            name = n;
        }
        if let Some(n) = name.strip_suffix(".LARGE") {
            if !is_large {
                continue;
            }
            name = n;
        }
        match &item.kind {
            TileItemKind::Enum { values } => {
                print!(" {name}=");
                let bits: BitVec = item.bits.iter().map(|&crd| get_bit(crd)).collect();
                let mut found = false;
                for (vn, val) in values {
                    if val == &bits {
                        print!("{vn}");
                        found = true;
                        break;
                    }
                }
                if !found {
                    for bit in bits.iter().rev() {
                        print!("{}", u8::from(*bit));
                    }
                }
            }
            TileItemKind::BitVec { invert } => {
                let bits: BitVec = item
                    .bits
                    .iter()
                    .enumerate()
                    .map(|(i, &crd)| get_bit(crd) ^ invert[i])
                    .collect();
                if item.bits.len() == 1 {
                    if bits[0] {
                        print!(" {name}");
                    } else {
                        print!(" !{name}");
                    }
                } else {
                    print!(" {name}=");
                    for bit in bits.iter().rev() {
                        print!("{}", u8::from(*bit));
                    }
                }
            }
        }
    }
}

fn print_globals(bs: &Bitstream, db: &Database, chip: &Chip) {
    print!("GLOBAL:");
    print_tile(&db.global_bits, chip, |crd| bs.get_global(crd));
    println!();
}

fn print_fb(bs: &Bitstream, db: &Database, chip: &Chip) {
    for fb in 0..chip.fbs {
        print!("FB {fb}:");
        print_tile(&db.fb_bits, chip, |crd| bs.get_fb(fb, crd));
        print_tile(&chip.imux_bits, chip, |crd| bs.get_fb(fb, crd));
        println!();
    }
}

fn print_uim(bs: &Bitstream, _db: &Database, chip: &Chip) {
    for fb in 0..chip.fbs {
        for imux in 0..36 {
            let found = (0..chip.fbs).any(|sfb| (0..18).any(|mc| bs.get_uim(fb, sfb, imux, mc)));
            if !found {
                continue;
            }
            print!("UIM {fb} {imux}:");
            for sfb in 0..chip.fbs {
                for mc in 0..18 {
                    if bs.get_uim(fb, sfb, imux, mc) {
                        print!(" {sfb}.{mc}");
                    }
                }
            }
            println!();
        }
    }
}

fn print_pt(bs: &Bitstream, _db: &Database, chip: &Chip) {
    let num_imux = if chip.kind == ChipKind::Xc9500 {
        36
    } else {
        54
    };
    for fb in 0..chip.fbs {
        for mc in 0..18 {
            for pt in 0..5 {
                let found = (0..num_imux)
                    .any(|i| bs.get_pt(fb, mc, pt, i, true) || bs.get_pt(fb, mc, pt, i, false));
                if !found {
                    continue;
                }
                print!("PT {fb} {mc} {pt}:");
                for i in 0..num_imux {
                    if bs.get_pt(fb, mc, pt, i, true) {
                        print!(" {i}");
                    }
                    if bs.get_pt(fb, mc, pt, i, false) {
                        print!(" !{i}");
                    }
                }
                println!();
            }
        }
    }
}

fn print_mc(bs: &Bitstream, db: &Database, chip: &Chip) {
    for fb in 0..chip.fbs {
        for mc in 0..18 {
            print!("MC {fb} {mc}:");
            print_tile(&db.mc_bits, chip, |crd| bs.get_mc(fb, mc, crd));
            println!();
        }
    }
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
    let dbfn = if dev.ends_with("xv") {
        args.dbdir.join("xc9500xv.zstd")
    } else if dev.ends_with("xl") {
        args.dbdir.join("xc9500xl.zstd")
    } else {
        args.dbdir.join("xc9500.zstd")
    };
    let db = Database::from_file(dbfn)?;
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
    let bs = Bitstream::from_jed(&fuses, chip);
    println!("DEVICE: {dev}");
    print_globals(&bs, &db, chip);
    // TODO: print UIM IBUF
    print_fb(&bs, &db, chip);
    if chip.kind == ChipKind::Xc9500 {
        print_uim(&bs, &db, chip);
    }
    print_pt(&bs, &db, chip);
    print_mc(&bs, &db, chip);
    Ok(())
}
