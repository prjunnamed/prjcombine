use std::{error::Error, path::PathBuf};

use clap::{Arg, Command, value_parser};
use prjcombine_jed::{JedFile, JedParserOptions};
use prjcombine_types::{
    bitvec::BitVec,
    bsdata::{Tile, TileBit, TileItemKind},
};
use prjcombine_xc9500::{Chip, ChipKind, Database};

struct Bitstream {
    fbs: Vec<Vec<[u8; 15]>>,
    uim: Vec<Vec<Vec<[u8; 5]>>>,
}

impl Bitstream {
    fn from_jed(jed: &JedFile, chip: &Chip) -> Self {
        let fuses = jed.fuses.as_ref().unwrap();
        let mut fbs = vec![];
        let mut uim = vec![];
        let mut pos = 0;
        if chip.kind == ChipKind::Xc9500 {
            for _ in 0..chip.blocks {
                let mut rows = vec![];
                for _ in 0..72 {
                    let mut row = [0; 15];
                    for col in 0..15 {
                        let sz = if col < 9 { 8 } else { 6 };
                        for j in 0..sz {
                            if fuses[pos + j] {
                                row[col] |= 1 << j;
                            }
                        }
                        pos += sz;
                    }
                    rows.push(row);
                }
                fbs.push(rows);
                let mut uim_fb = vec![];
                for _ in 0..chip.blocks {
                    let mut rows = vec![];
                    for _ in 0..18 {
                        let mut row = [0; 5];
                        for col in 0..5 {
                            let sz = if col == 0 { 8 } else { 7 };
                            for j in 0..sz {
                                if fuses[pos + j] {
                                    row[col] |= 1 << j;
                                }
                            }
                            pos += sz;
                        }
                        rows.push(row);
                    }
                    uim_fb.push(rows);
                }
                uim.push(uim_fb);
            }
        } else {
            for _ in 0..chip.blocks {
                fbs.push(vec![[0; 15]; 108]);
            }
            for row in 0..108 {
                for col in 0..15 {
                    for fb in 0..chip.blocks {
                        let sz = if col < 9 { 8 } else { 6 };
                        for j in 0..sz {
                            if fuses[pos + j] {
                                fbs[fb][row][col] |= 1 << j;
                            }
                        }
                        pos += sz;
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
                    print!("{bits}");
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
                    print!(" {name}={bits}");
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
    for fb in 0..chip.blocks {
        print!("FB {fb}:");
        print_tile(&db.block_bits, chip, |crd| bs.get_fb(fb, crd));
        print_tile(&chip.imux_bits, chip, |crd| bs.get_fb(fb, crd));
        println!();
    }
}

fn print_uim(bs: &Bitstream, _db: &Database, chip: &Chip) {
    for fb in 0..chip.blocks {
        for imux in 0..36 {
            let found = (0..chip.blocks).any(|sfb| (0..18).any(|mc| bs.get_uim(fb, sfb, imux, mc)));
            if !found {
                continue;
            }
            print!("UIM {fb} {imux}:");
            for sfb in 0..chip.blocks {
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
    for fb in 0..chip.blocks {
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
    for fb in 0..chip.blocks {
        for mc in 0..18 {
            print!("MC {fb} {mc}:");
            print_tile(&db.mc_bits, chip, |crd| bs.get_mc(fb, mc, crd));
            println!();
        }
    }
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let m = Command::new("xc9500_dis")
        .arg(
            Arg::new("dbdir")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("jed")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .get_matches();
    let arg_dbdir = m.get_one::<PathBuf>("dbdir").unwrap();
    let arg_jed = m.get_one::<PathBuf>("jed").unwrap();
    let jed = JedFile::parse_from_file(arg_jed, &JedParserOptions::new().skip_design_spec())?;
    let mut device = None;
    for note in &jed.notes {
        if let Some(dev) = note.strip_prefix(" DEVICE ") {
            device = Some(dev.to_ascii_lowercase());
        }
    }
    let device = device.unwrap();
    let dev = if let Some(pos) = device.find('-') {
        &device[..pos]
    } else {
        &device[..]
    };
    let dbfn = if dev.ends_with("xv") {
        arg_dbdir.join("xc9500xv.zstd")
    } else if dev.ends_with("xl") {
        arg_dbdir.join("xc9500xl.zstd")
    } else {
        arg_dbdir.join("xc9500.zstd")
    };
    let db = Database::from_file(dbfn)?;
    let mut part = None;
    for p in &db.devices {
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
    let bs = Bitstream::from_jed(&jed, chip);
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
