use std::collections::HashSet;

use bitvec::{field::BitField, vec::BitVec};
use crate::device::DeviceKind;

#[derive(Clone, Debug)]
pub struct Bitstream {
    pub kind: DeviceKind,
    pub idcode: u32,
    pub idcode_mask: u32,
    pub abits: Option<usize>,
    pub words: Vec<(u32, BitVec)>,
    pub fixups: HashSet<(u32, usize)>,
}

struct ShiftLine {
    len: usize,
    tdi: Option<BitVec>,
    tdo: Option<BitVec>,
    mask: Option<BitVec>,
}

fn parse_hex(s: &str, l: usize) -> BitVec {
    let mut res = BitVec::repeat(false, l);
    let s = s.strip_prefix('(').unwrap();
    let s = s.strip_suffix(')').unwrap();
    for (i, c) in s.chars().rev().enumerate() {
        let n = c.to_digit(16).unwrap();
        for j in 0..4 {
            if i * 4 + j < l {
                res.set(i * 4 + j, (n & 1 << j) != 0);
            }
        }
    }
    res
}

fn parse_shift_line(line: &str) -> ShiftLine {
    let line: Vec<_> = line.split_ascii_whitespace().collect();
    let len: usize = line[1].parse().unwrap();
    let mut tdi = None;
    let mut tdo = None;
    let mut mask = None;
    let n = (line.len() - 2) / 2;
    for i in 0..n {
        let v = parse_hex(line[2 * i + 3], len);
        match line[2 * i + 2] {
            "TDI" => tdi = Some(v),
            "TDO" => tdo = Some(v),
            "MASK" => mask = Some(v),
            _ => (),
        }
    }
    ShiftLine {
        len,
        tdi,
        tdo,
        mask,
    }
}

fn ungray(n: u32, l: usize) -> u32 {
    let mut xor = 0;
    let mut res = 0;
    for i in 0..l {
        let b = n >> i & 1;
        xor ^= b;
        res |= xor << (l - i - 1);
    }
    res
}

fn parse_coolrunner_word(kind: DeviceKind, data: &BitVec) -> ((u32, BitVec), usize) {
    let alen = match (kind, data.len()) {
        (DeviceKind::Xpla3, 121) => 7,
        (DeviceKind::Xpla3, 131) => 8,
        (DeviceKind::Xpla3, 274) => 8,
        (DeviceKind::Xpla3, 313) => 9,
        (DeviceKind::Xpla3, 516) => 9,
        (DeviceKind::Xpla3, 765) => 9,
        (DeviceKind::Coolrunner2, 266) => 6,
        (DeviceKind::Coolrunner2, 281) => 7,
        (DeviceKind::Coolrunner2, 759) => 7,
        (DeviceKind::Coolrunner2, 1371) => 7,
        (DeviceKind::Coolrunner2, 1875) => 7,
        (DeviceKind::Coolrunner2, 1988) => 8,
        _ => unreachable!(),
    };
    let wlen = data.len() - alen;
    let word = data[..wlen].to_bitvec();
    let addr: u32 = data[wlen..].load_le();
    let addr = if kind == DeviceKind::Xpla3 {
        ungray(addr >> 1, alen - 1) | (addr & 1) << (alen - 1)
    } else {
        ungray(addr, alen)
    };
    ((addr, word), alen)
}

pub fn parse_svf(kind: DeviceKind, svf: &str) -> Bitstream {
    let mut lines = svf.lines();
    let idcode;
    let idcode_mask;
    // skip to IR shift
    loop {
        let line = lines.next().unwrap();
        if line.starts_with("SIR") {
            break;
        }
    }
    // IDCODE
    loop {
        let line = lines.next().unwrap();
        if line.starts_with("SDR") {
            let line = parse_shift_line(line);
            assert_eq!(line.len, 32);
            idcode = line.tdo.unwrap().load_le();
            idcode_mask = line.mask.unwrap().load_le();
            break;
        }
    }
    let num_skip = match kind {
        DeviceKind::Xc9500 | DeviceKind::Xc9500Xl | DeviceKind::Xc9500Xv => 3,
        DeviceKind::Xpla3 => 2,
        DeviceKind::Coolrunner2 => 5,
    };
    for _ in 0..num_skip {
        // skip to IR shift
        loop {
            let line = lines.next().unwrap();
            if line.starts_with("SIR") {
                break;
            }
        }
    }
    let mut words = vec![];
    let mut abits = None;
    let mut sdr_len = None;
    loop {
        let line = lines.next().unwrap();
        if line.starts_with("SIR") {
            break;
        }
        if line.starts_with("SDR") {
            let mut line = line.to_string();
            while !line.ends_with(';') {
                line.push_str(lines.next().unwrap());
            }
            let line = parse_shift_line(&line);
            let data = line.tdi.unwrap();
            sdr_len = Some(data.len());
            match kind {
                DeviceKind::Xc9500 => {
                    assert_eq!(data.len(), 27);
                    let word = data[2..10].to_bitvec();
                    let addr = data[10..].load_le();
                    words.push((addr, word));
                }
                DeviceKind::Xc9500Xl | DeviceKind::Xc9500Xv => {
                    let wlen = data.len() - 18;
                    assert!(matches!(wlen, 16 | 32 | 64 | 128));
                    let word = data[2..wlen + 2].to_bitvec();
                    let addr = data[wlen + 2..].load_le();
                    words.push((addr, word));
                }
                DeviceKind::Xpla3 | DeviceKind::Coolrunner2 => {
                    let (word, alen) = parse_coolrunner_word(kind, &data);
                    abits = Some(alen);
                    words.push(word);
                }
            }
        }
    }
    if kind == DeviceKind::Coolrunner2 {
        // xa2c* bug workaround
        let idx = words.len() - 2;
        let w = words[idx].1.clone();
        words[idx + 1].1 |= w;
    }
    let mut fixups = HashSet::new();
    loop {
        let Some(line) = lines.next() else {
            break;
        };
        if line.starts_with("SDR") {
            let mut line = line.to_string();
            while !line.ends_with(';') {
                line.push_str(lines.next().unwrap());
            }
            let line = parse_shift_line(&line);
            if line.len != sdr_len.unwrap() {
                continue;
            }
            let data = line.tdi.unwrap();
            match kind {
                DeviceKind::Xc9500 => {
                    assert_eq!(data.len(), 27);
                    let word = data[2..10].to_bitvec();
                    let addr = data[10..].load_le();
                    let idx = words.iter().position(|w| w.0 == addr).unwrap();
                    let oword = &words[idx].1;
                    for (i, b) in word.into_iter().enumerate() {
                        if oword[i] && !b {
                            fixups.insert((addr, i));
                        }
                    }
                }
                DeviceKind::Xc9500Xl | DeviceKind::Xc9500Xv => {
                    let wlen = data.len() - 18;
                    assert!(matches!(wlen, 16 | 32 | 64 | 128));
                    let word = data[2..wlen + 2].to_bitvec();
                    let addr = data[wlen + 2..].load_le();
                    let idx = words.iter().position(|w| w.0 == addr).unwrap();
                    let oword = &words[idx].1;
                    for (i, b) in word.into_iter().enumerate() {
                        if !oword[i] && b {
                            fixups.insert((addr, i));
                        }
                    }
                }
                DeviceKind::Xpla3 | DeviceKind::Coolrunner2 => {
                    let (word, _) = parse_coolrunner_word(kind, &data);
                    let idx = words.iter().position(|w| w.0 == word.0).unwrap();
                    let oword = &words[idx].1;
                    for (i, b) in word.1.into_iter().enumerate() {
                        if oword[i] && !b {
                            fixups.insert((word.0, i));
                        }
                    }
                }
            }
        }
    }
    if kind.is_xc9500() {
        let idx = words.len() - 2;
        assert_eq!(words[idx], words[idx + 1]);
        words.pop();
    }

    Bitstream {
        kind,
        idcode,
        idcode_mask,
        abits,
        words,
        fixups,
    }
}
