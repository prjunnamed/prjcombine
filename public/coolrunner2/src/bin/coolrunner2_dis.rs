use std::{collections::BTreeMap, error::Error, path::PathBuf};

use bitvec::vec::BitVec;
use clap::{Arg, Command, value_parser};
use prjcombine_coolrunner2::{Chip, Database};
use prjcombine_jed::{JedFile, JedParserOptions};
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

impl Bitstream {
    fn from_jed(jed: &JedFile, chip: &Chip, db: &Database) -> Self {
        let fuses = jed.fuses.as_ref().unwrap();
        let mut fbs = vec![];
        let mut pos = 0;
        for fb in 0..(chip.fb_cols.len() * chip.fb_rows * 2) {
            let mut fbd = FbData {
                imux: BTreeMap::new(),
                mcs: core::array::from_fn(|_| BTreeMap::new()),
                pla_and: core::array::from_fn(|_| PTermData {
                    im_t: BitVec::new(),
                    im_f: BitVec::new(),
                }),
                pla_or: core::array::from_fn(|_| BitVec::new()),
            };
            for i in 0..40 {
                let n = format!("IM[{i}].MUX");
                let data = fuses[pos..(pos + chip.imux_width)].to_bitvec();
                pos += chip.imux_width;
                fbd.imux.insert(n, data);
            }
            for i in 0..56 {
                let pt = &mut fbd.pla_and[i];
                for _ in 0..40 {
                    pt.im_t.push(!fuses[pos]);
                    pos += 1;
                    pt.im_f.push(!fuses[pos]);
                    pos += 1;
                }
            }
            for _ in 0..56 {
                for j in 0..16 {
                    fbd.pla_or[j].push(!fuses[pos]);
                    pos += 1;
                }
            }
            for mc in 0..16 {
                let iobful = chip
                    .io
                    .contains_key(&IoId::Mc((FbId::from_idx(fb), FbMcId::from_idx(mc))));
                let mcd = &mut fbd.mcs[mc];
                let jed_bits = if !chip.has_vref {
                    &db.jed_mc_bits_small
                } else if iobful {
                    &db.jed_mc_bits_large_iob
                } else {
                    &db.jed_mc_bits_large_buried
                };
                for (bn, bi) in jed_bits {
                    let bits = mcd.entry(bn.clone()).or_insert_with(|| {
                        BitVec::repeat(false, chip.mc_bits.items[bn].bits.len())
                    });
                    bits.set(*bi, fuses[pos]);
                    pos += 1;
                }
            }
            fbs.push(fbd);
        }
        let mut globals = BTreeMap::new();
        for (bn, bi) in &chip.jed_global_bits {
            let bits = globals
                .entry(bn.clone())
                .or_insert_with(|| BitVec::repeat(false, chip.global_bits.items[bn].bits.len()));
            bits.set(*bi, fuses[pos]);
            pos += 1;
        }
        assert_eq!(pos, fuses.len());
        Bitstream { fbs, globals }
    }
}

fn print_tile(data: &BTreeMap<String, BitVec>, tile: &Tile) {
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
    let m = Command::new("coolrunner2_dis")
        .arg(
            Arg::new("db")
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
    let arg_jed = m.get_one::<PathBuf>("jed").unwrap();
    let jed = JedFile::parse_from_file(arg_jed, &JedParserOptions::new().skip_design_spec())?;
    let mut device = None;
    for note in &jed.notes {
        if let Some(dev) = note.strip_prefix(" DEVICE ") {
            device = Some(dev.to_ascii_lowercase());
        }
    }
    let device = device.unwrap();
    let device = device.to_ascii_lowercase();
    let dev = if let Some(pos) = device.find('-') {
        &device[..pos]
    } else {
        &device[..]
    };
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
    let bs = Bitstream::from_jed(&jed, chip, &db);
    println!("DEVICE: {dev}");
    print!("GLOBAL:");
    print_tile(&bs.globals, &chip.global_bits);
    for (i, fbd) in bs.fbs.iter().enumerate() {
        print!("FB {i}:");
        print_tile(&fbd.imux, &chip.imux_bits);
        for j in 0..16 {
            print!("MC {i} {j}:");
            print_tile(&fbd.mcs[j], &chip.mc_bits);
        }
        for (j, pt) in fbd.pla_and.iter().enumerate() {
            if pt.im_t.any() || pt.im_f.any() {
                print!("PT {i} {j}:");
                for k in 0..40 {
                    if pt.im_t[k] {
                        print!(" {k}");
                    }
                    if pt.im_f[k] {
                        print!(" !{k}");
                    }
                }
                println!();
            }
        }
        for (j, st) in fbd.pla_or.iter().enumerate() {
            if st.any() {
                print!("ST {i} {j}:");
                for k in 0..56 {
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
