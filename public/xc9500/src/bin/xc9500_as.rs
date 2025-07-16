use std::{error::Error, fs::read_to_string, path::PathBuf};

use clap::{Arg, Command, value_parser};
use prjcombine_jed::JedFile;
use prjcombine_xc9500::{Chip, ChipKind, Database};

use prjcombine_types::{
    bitvec::BitVec,
    bsdata::{Tile, TileBit, TileItemKind},
};

struct Bitstream {
    fbs: Vec<Vec<[u8; 15]>>,
    uim: Vec<Vec<Vec<[u8; 5]>>>,
}

impl Bitstream {
    fn new(chip: &Chip) -> Self {
        let rows = if chip.kind == ChipKind::Xc9500 {
            72
        } else {
            108
        };
        let fbs = (0..chip.blocks)
            .map(|_| {
                vec![
                    if chip.kind == ChipKind::Xc9500 {
                        [
                            0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0, 0, 0, 0, 0, 0,
                        ]
                    } else {
                        [0; 15]
                    };
                    rows
                ]
            })
            .collect();
        let uim = if chip.kind == ChipKind::Xc9500 {
            (0..chip.blocks)
                .map(|_| (0..chip.blocks).map(|_| vec![[0; 5]; 18]).collect())
                .collect()
        } else {
            vec![]
        };
        Bitstream { fbs, uim }
    }

    fn to_jed(&self, device: &str) -> JedFile {
        let mut res = BitVec::new();
        if !self.uim.is_empty() {
            for fb in 0..self.fbs.len() {
                for row in 0..72 {
                    for col in 0..15 {
                        let sz = if col < 9 { 8 } else { 6 };
                        for j in 0..sz {
                            res.push((self.fbs[fb][row][col] >> j & 1) != 0);
                        }
                    }
                }
                for sfb in 0..self.fbs.len() {
                    for row in 0..18 {
                        for col in 0..5 {
                            let sz = if col == 0 { 8 } else { 7 };
                            for j in 0..sz {
                                res.push((self.uim[fb][sfb][row][col] >> j & 1) != 0);
                            }
                        }
                    }
                }
            }
        } else {
            for row in 0..108 {
                for col in 0..15 {
                    for fb in 0..self.fbs.len() {
                        let sz = if col < 9 { 8 } else { 6 };
                        for j in 0..sz {
                            res.push((self.fbs[fb][row][col] >> j & 1) != 0);
                        }
                    }
                }
            }
        }
        JedFile::new()
            .with_fuses(res)
            .with_note(format!(" DEVICE {device}"))
    }

    fn put_bit(&mut self, fb: usize, row: usize, col: usize, bit: usize, val: bool) {
        if val {
            self.fbs[fb][row][col] |= 1 << bit;
        } else {
            self.fbs[fb][row][col] &= !(1 << bit);
        }
    }

    fn put_global(&mut self, crd: TileBit, val: bool) {
        self.put_bit(crd.tile, crd.frame, crd.bit % 9, 6 + crd.bit / 9, val);
    }

    fn put_fb(&mut self, fb: usize, crd: TileBit, val: bool) {
        self.put_bit(fb, crd.frame, crd.bit % 9, 6 + crd.bit / 9, val);
    }

    fn put_mc(&mut self, fb: usize, mc: usize, crd: TileBit, val: bool) {
        self.put_bit(fb, crd.frame, mc % 9, 6 + mc / 9, val);
    }

    fn put_pt(&mut self, fb: usize, mc: usize, pt: usize, imux: usize, pol: bool, val: bool) {
        self.put_bit(
            fb,
            imux * 2 + usize::from(pol),
            pt + (mc % 3) * 5,
            mc / 3,
            val,
        );
    }

    fn put_uim(&mut self, fb: usize, sfb: usize, imux: usize, mc: usize, val: bool) {
        if val {
            self.uim[fb][sfb][mc][imux % 5] |= 1 << (imux / 5);
        } else {
            self.uim[fb][sfb][mc][imux % 5] &= !(1 << (imux / 5));
        }
    }
}

fn set_tile_item(tile: &Tile, chip: &Chip, item: &str, mut put_bit: impl FnMut(TileBit, bool)) {
    let is_large = chip.io_special.contains_key("GOE2");
    if let Some((name, val)) = item.split_once('=') {
        let item = tile.items.get(name).unwrap_or_else(|| {
            &tile.items[&format!("{}.{}", name, if is_large { "LARGE" } else { "SMALL" })]
        });
        match &item.kind {
            TileItemKind::Enum { values } => {
                let val = &values[val];
                for (k, v) in item.bits.iter().zip(val.iter()) {
                    put_bit(*k, v);
                }
            }
            TileItemKind::BitVec { invert } => {
                assert_eq!(val.len(), item.bits.len());
                for (i, (k, v)) in item.bits.iter().zip(val.chars().rev()).enumerate() {
                    put_bit(
                        *k,
                        match v {
                            '0' => false,
                            '1' => true,
                            _ => unreachable!(),
                        } ^ invert[i],
                    )
                }
            }
        }
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
                put_bit(item.bits[0], val ^ invert[0]);
            }
        }
    }
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let m = Command::new("xc9500_as")
        .arg(
            Arg::new("dbdir")
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
    let arg_dbdir = m.get_one::<PathBuf>("dbdir").unwrap();
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
        let mut block_bits = db.block_bits.clone();
        for (k, v) in chip.imux_bits.clone().items {
            block_bits.items.insert(k, v);
        }
        match pref[..] {
            ["GLOBAL"] => {
                for item in suf {
                    set_tile_item(&db.global_bits, chip, item, |crd, val| {
                        bs.put_global(crd, val)
                    });
                }
            }
            ["FB", fb] => {
                let fb: usize = fb.parse()?;
                for item in suf {
                    set_tile_item(&block_bits, chip, item, |crd, val| bs.put_fb(fb, crd, val));
                }
            }
            ["MC", fb, mc] => {
                let fb: usize = fb.parse()?;
                let mc: usize = mc.parse()?;
                for item in suf {
                    set_tile_item(&db.mc_bits, chip, item, |crd, val| {
                        bs.put_mc(fb, mc, crd, val)
                    });
                }
            }
            ["PT", fb, mc, pt] => {
                let fb: usize = fb.parse()?;
                let mc: usize = mc.parse()?;
                let pt: usize = pt.parse()?;
                for item in suf {
                    let (imux, pol) = if let Some(x) = item.strip_prefix('!') {
                        (x.parse()?, false)
                    } else {
                        (item.parse()?, true)
                    };
                    bs.put_pt(fb, mc, pt, imux, pol, true);
                }
            }
            ["UIM", fb, imux] => {
                let fb: usize = fb.parse()?;
                let imux: usize = imux.parse()?;
                for item in suf {
                    let (sfb, mc) = item.split_once('.').unwrap();
                    let sfb: usize = sfb.parse()?;
                    let mc: usize = mc.parse()?;
                    bs.put_uim(fb, sfb, imux, mc, true);
                }
            }

            _ => panic!("weird line {line}"),
        }
    }
    let jed = bs.to_jed(dev);
    jed.emit_to_file(arg_jed)?;

    Ok(())
}
