use std::collections::HashMap;

use bitvec::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum BitPos {
    // bank, frame, bit
    Main(usize, usize, usize),
    // bank, frame, bit
    Bram(usize, usize, usize),
    Speed(usize),
    CReg(usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum BitTile {
    Speed,
    CReg,
    // bank, frame, height, bit, width
    Main(usize, usize, usize, usize, usize),
    // bank, bit
    Bram(usize, usize),
}

impl BitTile {
    pub fn xlat_pos_rev(&self, bit: BitPos) -> Option<(usize, usize)> {
        match (*self, bit) {
            (BitTile::Speed, BitPos::Speed(pos)) => Some((0, pos)),
            (BitTile::CReg, BitPos::CReg(pos)) => Some((0, pos)),
            (BitTile::Main(bank, frame, height, bit, width), BitPos::Main(bbank, bframe, bbit))
                if bank == bbank
                    && bframe >= frame
                    && bframe < frame + height
                    && bbit >= bit
                    && bbit < bit + width =>
            {
                Some((
                    if (bank & 1) != 0 {
                        height - 1 - (bframe - frame)
                    } else {
                        bframe - frame
                    },
                    if (bank & 2) != 0 {
                        width - 1 - (bbit - bit)
                    } else {
                        bbit - bit
                    },
                ))
            }
            (BitTile::Bram(bank, bit), BitPos::Bram(bbank, bframe, bbit))
                if bank == bbank && bbit >= bit && bbit < bit + 0x10 =>
            {
                Some((bframe, bbit - bit))
            }
            _ => None,
        }
    }

    pub fn xlat_pos_fwd(&self, bit: (usize, usize)) -> BitPos {
        let (tframe, tbit) = bit;
        match *self {
            BitTile::Speed => {
                assert_eq!(tframe, 0);
                BitPos::Speed(tbit)
            }
            BitTile::CReg => {
                assert_eq!(tframe, 0);
                BitPos::CReg(tbit)
            }
            BitTile::Main(bank, frame, height, bit, width) => {
                assert!(tframe < height);
                assert!(tbit < width);
                BitPos::Main(
                    bank,
                    frame
                        + if (bank & 1) != 0 {
                            height - 1 - tframe
                        } else {
                            tframe
                        },
                    bit + if (bank & 2) != 0 {
                        width - 1 - tbit
                    } else {
                        tbit
                    },
                )
            }
            BitTile::Bram(bank, bit) => BitPos::Bram(bank, tframe, tbit + bit),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BitstreamBank {
    pub frame_len: usize,
    pub frame_data: BitVec,
    pub frame_present: BitVec,
}

impl BitstreamBank {
    fn empty() -> Self {
        Self {
            frame_len: 0,
            frame_data: bitvec![],
            frame_present: bitvec![],
        }
    }

    pub fn frame(&self, idx: usize) -> &BitSlice {
        &self.frame_data[idx * self.frame_len..(idx + 1) * self.frame_len]
    }

    pub fn frame_mut(&mut self, idx: usize) -> &mut BitSlice {
        &mut self.frame_data[idx * self.frame_len..(idx + 1) * self.frame_len]
    }
}

#[derive(Debug, Clone)]
pub struct Bitstream {
    pub cram: [BitstreamBank; 4],
    pub bram: [BitstreamBank; 4],
    pub speed: u8,
    pub creg: u16,
}

struct Crc {
    state: u16,
}

impl Crc {
    fn new() -> Self {
        Self { state: 0xffff }
    }

    fn feed(&mut self, data: &[u8]) {
        for &byte in data {
            for i in (0..8).rev() {
                let bit = ((byte >> i) & 1) != 0;
                let bit = bit ^ ((self.state >> 15) != 0);
                self.state <<= 1;
                if bit {
                    self.state ^= 0x1021;
                }
            }
        }
    }

    fn get(&self) -> u16 {
        self.state
    }
}

impl Bitstream {
    pub fn diff(a: &Bitstream, b: &Bitstream) -> HashMap<BitPos, bool> {
        let mut res = HashMap::new();
        for (bank, (ba, bb)) in (a.cram.iter()).zip(b.cram.iter()).enumerate() {
            assert_eq!(ba.frame_len, bb.frame_len);
            assert_eq!(ba.frame_present.len(), bb.frame_present.len());
            for i in 0..ba.frame_present.len() {
                let fa = ba.frame(i);
                let fb = bb.frame(i);
                if fa == fb {
                    continue;
                }
                for j in 0..ba.frame_len {
                    if fa[j] != fb[j] {
                        res.insert(BitPos::Main(bank, i, j), fb[j]);
                    }
                }
            }
        }
        for (bank, (ba, bb)) in (a.bram.iter()).zip(b.bram.iter()).enumerate() {
            assert_eq!(ba.frame_len, bb.frame_len);
            assert_eq!(ba.frame_present.len(), bb.frame_present.len());
            for i in 0..ba.frame_present.len() {
                let fa = ba.frame(i);
                let fb = bb.frame(i);
                if fa == fb {
                    continue;
                }
                for j in 0..ba.frame_len {
                    if fa[j] != fb[j] {
                        res.insert(BitPos::Bram(bank, i, j), fb[j]);
                    }
                }
            }
        }
        for j in 0..16 {
            if (a.creg >> j & 1) != (b.creg >> j & 1) {
                res.insert(BitPos::CReg(j), (b.creg >> j & 1) != 0);
            }
        }
        for j in 0..8 {
            if (a.speed >> j & 1) != (b.speed >> j & 1) {
                res.insert(BitPos::Speed(j), (b.speed >> j & 1) != 0);
            }
        }
        res
    }

    pub fn get(&self, bit: BitPos) -> bool {
        match bit {
            BitPos::Main(bank, frame, bit) => self.cram[bank].frame(frame)[bit],
            BitPos::Bram(bank, frame, bit) => self.bram[bank].frame(frame)[bit],
            BitPos::Speed(bit) => ((self.speed >> bit) & 1) != 0,
            BitPos::CReg(bit) => ((self.creg >> bit) & 1) != 0,
        }
    }

    pub fn parse(data: &[u8]) -> Self {
        assert_eq!(data[..4], [0x7e, 0xaa, 0x99, 0x7e]);
        let mut crc = Crc::new();
        let mut pos = 4;
        let mut speed = None;
        let mut creg = None;
        let mut bank_width = None;
        let mut bank_height = None;
        let mut bank_offset = None;
        let mut bank_idx = None;
        let mut cram = [None, None, None, None];
        let mut bram = [None, None, None, None];
        loop {
            let opcode = data[pos];
            match opcode & 0xf {
                1 => {
                    crc.feed(&data[pos..pos + 2]);
                    let payload = data[pos + 1];
                    pos += 2;
                    match opcode {
                        0x01 => match payload {
                            // write CRAM
                            0x01 => read_bank(
                                data,
                                &mut pos,
                                &mut crc,
                                &mut cram[bank_idx.unwrap()],
                                bank_width.unwrap(),
                                bank_height.unwrap(),
                                bank_offset.unwrap(),
                            ),
                            // write BRAM
                            0x03 => read_bank(
                                data,
                                &mut pos,
                                &mut crc,
                                &mut bram[bank_idx.unwrap()],
                                bank_width.unwrap(),
                                bank_height.unwrap(),
                                bank_offset.unwrap(),
                            ),
                            // CRC reset
                            0x05 => crc = Crc::new(),
                            // startup (end of bitstream)
                            0x06 => break,
                            _ => panic!("unk cmd {payload:02x}"),
                        },
                        0x11 => bank_idx = Some(usize::from(payload)),
                        0x51 => speed = Some(payload),
                        _ => panic!("unknown opcode {opcode:02x} {payload:02x}"),
                    }
                }
                2 => {
                    let b0 = data[pos + 1];
                    let b1 = data[pos + 2];
                    let payload = u16::from_be_bytes([b0, b1]);
                    crc.feed(&[opcode]);
                    match opcode {
                        0x22 => assert_eq!(crc.get(), payload),
                        0x62 => bank_width = Some(usize::from(payload) + 1),
                        0x72 => bank_height = Some(usize::from(payload)),
                        0x82 => bank_offset = Some(usize::from(payload)),
                        0x92 => creg = Some(payload),
                        _ => panic!("unknown opcode {opcode:02x} {payload:04x}"),
                    }
                    crc.feed(&data[pos + 1..pos + 3]);
                    pos += 3;
                }
                _ => panic!("unknown opcode {opcode:02x}"),
            }
        }
        Bitstream {
            cram: cram.map(Option::unwrap),
            bram: bram.map(|x| x.unwrap_or_else(BitstreamBank::empty)),
            speed: speed.unwrap(),
            creg: creg.unwrap(),
        }
    }
}

fn read_bank(
    data: &[u8],
    pos: &mut usize,
    crc: &mut Crc,
    bank: &mut Option<BitstreamBank>,
    width: usize,
    height: usize,
    offset: usize,
) {
    let nbits = width * height;
    assert_eq!(nbits % 8, 0);
    let nbytes = nbits / 8 + 2;
    crc.feed(&data[*pos..*pos + nbytes]);
    let bank = bank.get_or_insert_with(|| BitstreamBank {
        frame_len: width,
        frame_data: BitVec::new(),
        frame_present: BitVec::new(),
    });
    assert_eq!(bank.frame_len, width);
    while bank.frame_present.len() < height + offset {
        bank.frame_present.push(false);
        bank.frame_data.extend(bitvec![0; width]);
    }
    for i in 0..height {
        assert!(!bank.frame_present[offset + i]);
        bank.frame_present.set(offset + i, true);
        let frame = bank.frame_mut(offset + i);
        for j in 0..width {
            let bidx = i * width + j;
            let b = data[*pos + bidx / 8];
            let bit = ((b << (bidx % 8)) & 0x80) != 0;
            frame.set(j, bit);
        }
    }
    *pos += nbytes;
    assert_eq!(data[*pos - 2..*pos], [0, 0]);
}
