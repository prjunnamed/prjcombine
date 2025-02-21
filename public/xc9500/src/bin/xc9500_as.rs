use std::{
    error::Error,
    fs::{File, read_to_string},
    io::Write,
    path::{Path, PathBuf},
};

use bitvec::vec::BitVec;
use clap::Parser;
use prjcombine_xc9500::{Database, Device, DeviceKind, FbBitCoord, GlobalBitCoord};

use prjcombine_types::tiledb::{Tile, TileItemKind};

struct Bitstream {
    fbs: Vec<Vec<[u8; 15]>>,
    uim: Vec<Vec<Vec<[u8; 5]>>>,
}

impl Bitstream {
    fn new(dev: &Device) -> Self {
        let rows = if dev.kind == DeviceKind::Xc9500 {
            72
        } else {
            108
        };
        let fbs = (0..dev.fbs)
            .map(|_| {
                vec![
                    if dev.kind == DeviceKind::Xc9500 {
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
        let uim = if dev.kind == DeviceKind::Xc9500 {
            (0..dev.fbs)
                .map(|_| (0..dev.fbs).map(|_| vec![[0; 5]; 18]).collect())
                .collect()
        } else {
            vec![]
        };
        Bitstream { fbs, uim }
    }

    fn to_jed(&self) -> BitVec {
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
        res
    }

    fn put_bit(&mut self, fb: usize, row: usize, col: usize, bit: usize, val: bool) {
        if val {
            self.fbs[fb][row][col] |= 1 << bit;
        } else {
            self.fbs[fb][row][col] &= !(1 << bit);
        }
    }

    fn put_global(&mut self, crd: GlobalBitCoord, val: bool) {
        self.put_bit(
            crd.fb as usize,
            crd.row as usize,
            crd.column as usize,
            crd.bit as usize,
            val,
        );
    }

    fn put_fb(&mut self, fb: usize, crd: FbBitCoord, val: bool) {
        self.put_bit(
            fb,
            crd.row as usize,
            crd.column as usize,
            crd.bit as usize,
            val,
        );
    }

    fn put_mc(&mut self, fb: usize, mc: usize, row: usize, val: bool) {
        self.put_bit(fb, row, mc % 9, 6 + mc / 9, val);
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

fn set_tile_item<T: Copy>(
    tile: &Tile<T>,
    device: &Device,
    item: &str,
    mut put_bit: impl FnMut(T, bool),
) {
    let is_large = device.io_special.contains_key("GOE2");
    if let Some((name, val)) = item.split_once('=') {
        let item = tile.items.get(name).unwrap_or_else(|| {
            &tile.items[&format!("{}.{}", name, if is_large { "LARGE" } else { "SMALL" })]
        });
        match &item.kind {
            TileItemKind::Enum { values } => {
                let val = &values[val];
                for (k, v) in item.bits.iter().zip(val.iter()) {
                    put_bit(*k, *v);
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

#[derive(Parser)]
struct Args {
    dbdir: PathBuf,
    src: PathBuf,
    jed: PathBuf,
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
    let device = &db.devices[part.device];
    let mut bs = Bitstream::new(device);
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
        let mut fb_bits = db.fb_bits.clone();
        for (k, v) in device.imux_bits.clone().items {
            fb_bits.items.insert(k, v);
        }
        match pref[..] {
            ["GLOBAL"] => {
                for item in suf {
                    set_tile_item(&db.global_bits, device, item, |crd, val| {
                        bs.put_global(crd, val)
                    });
                }
            }
            ["FB", fb] => {
                let fb: usize = fb.parse()?;
                for item in suf {
                    set_tile_item(&fb_bits, device, item, |crd, val| bs.put_fb(fb, crd, val));
                }
            }
            ["MC", fb, mc] => {
                let fb: usize = fb.parse()?;
                let mc: usize = mc.parse()?;
                for item in suf {
                    set_tile_item(&db.mc_bits, device, item, |crd, val| {
                        bs.put_mc(fb, mc, crd as usize, val)
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
    let fuses = bs.to_jed();
    write_jed(args.jed, dev, &fuses)?;

    Ok(())
}
