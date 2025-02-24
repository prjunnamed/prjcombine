use crate::packet::{Crc, Packet, PacketParser};
use crate::{
    Bitstream, BitstreamGeom, BitstreamMode, DeviceKind, DieBitstream, FrameAddr, FrameMaskMode,
    GtzBitstream, KeyData, Reg,
};
use arrayref::array_ref;
use bitvec::prelude::*;
use prjcombine_interconnect::dir::Dir;
use std::collections::HashMap;

fn parse_xc2000_bitstream(bs: &mut Bitstream, data: &[u8]) {
    let bs = bs.die.first_mut().unwrap();
    let data: &BitSlice<u8, Msb0> = BitSlice::from_slice(data);
    assert_eq!(data[..12], bits![1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0]);
    let mut bitlen = 0;
    for j in 0..24 {
        if data[35 - j] {
            bitlen |= 1 << j;
        }
    }
    assert_eq!(data[36..40], bits![1, 1, 1, 1]);
    let mut pos = 40;
    let frame_len = bs.frame_len;
    let frames_num = bs.frame_info.len();
    for fi in 0..frames_num {
        assert!(!data[pos]);
        pos += 1;
        let fdata = &data[pos..(pos + frame_len)];
        let frame = bs.frame_mut(fi);
        for (i, bit) in fdata.iter().enumerate() {
            frame.set(i, *bit);
        }
        pos += frame_len;
        let stop = &data[pos..(pos + 3)];
        assert_eq!(stop, &bits![1, 1, 1]);
        pos += 3;
    }
    let post = &data[pos..(pos + 4)];
    assert_eq!(post, &bits![1, 1, 1, 1]);
    pos += 4;
    while pos % 8 != 0 {
        assert!(data[pos]);
        pos += 1;
    }
    assert_eq!(bitlen, pos + 1);
    let pad = &data[pos..(pos + 8)];
    assert_eq!(pad, &bits![1, 1, 1, 1, 1, 1, 1, 1]);
    pos += 8;
    assert_eq!(pos, data.len());
}

struct Xc4000Crc {
    crc: u16,
}

impl Xc4000Crc {
    fn new() -> Self {
        Self { crc: 0 }
    }

    fn feed_bit(&mut self, b: bool) {
        if !b {
            self.crc ^= 0x8000;
        }
        if (self.crc & 0x8000) != 0 {
            self.crc <<= 1;
            self.crc ^= 0x8005;
        } else {
            self.crc <<= 1;
        }
    }
}

fn parse_xc4000_bitstream(bs: &mut Bitstream, data: &[u8]) {
    let kind = bs.kind;
    let bs = bs.die.first_mut().unwrap();
    if data.starts_with(&[0xff, 0xff, 0xf2]) {
        let bitlen = (data[3] as u32) << 16 | (data[4] as u32) << 8 | (data[5] as u32);
        assert_eq!(data[6], 0xd2);
        let mut pos = 7;
        let frame_len = bs.frame_len;
        let frames_num = bs.frame_info.len();
        let start = if kind == DeviceKind::S40Xl {
            0xff
        } else {
            0xfe
        };
        let flen = frame_len.div_ceil(8);
        let pad = flen * 8 - frame_len;
        for fi in 0..frames_num {
            assert_eq!(data[pos], start);
            pos += 1;
            let fdata = &data[pos..pos + flen];
            pos += flen;
            let frame = bs.frame_mut(fi);
            for (i, &b) in fdata.iter().enumerate() {
                for bit in 0..8 {
                    let bv = (b >> bit & 1) != 0;
                    if kind == DeviceKind::S40Xl {
                        let bi = match bit {
                            0 => {
                                if i == 0 {
                                    assert!(!bv);
                                    continue;
                                }
                                bit * flen + i - 1
                            }
                            7 => {
                                if i == 0 {
                                    assert!(bv);
                                    continue;
                                }
                                bit * flen + i - 2
                            }
                            _ => bit * flen + i - 1,
                        };
                        assert!(bi < frame_len);
                        frame.set(bi, bv);
                    } else {
                        if bit < (8 - pad) {
                            let bi = bit * flen + i;
                            frame.set(bi, bv);
                        } else {
                            if i == 0 {
                                assert!(bv);
                            } else {
                                let bi = bit * (flen - 1) + (8 - pad) + (i - 1);
                                frame.set(bi, bv);
                            }
                        }
                    }
                }
            }
            assert_eq!(data[pos..pos + 6], [0xd2, 0xff, 0xd2, 0xff, 0xff, 0xff]);
            pos += 6;
        }
        assert_eq!(data[pos..pos + 10], [0xff; 10]);
        pos += 10;
        assert_eq!(pos, data.len());
        assert_eq!(bitlen as usize, pos * 8 - 7);
    } else {
        let mut crc = Xc4000Crc::new();
        let data: &BitSlice<u8, Msb0> = BitSlice::from_slice(data);
        assert_eq!(data[..12], bits![1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0]);
        let mut bitlen = 0;
        for j in 0..24 {
            if data[35 - j] {
                bitlen |= 1 << j;
            }
        }
        assert_eq!(data[36..40], bits![1, 1, 1, 1]);
        let mut pos = 40;
        let frame_len = bs.frame_len;
        let frames_num = bs.frame_info.len();
        let mut crc_enable = false;
        for fi in 0..frames_num {
            assert!(!data[pos]);
            if fi == 0 {
                crc.feed_bit(true);
            } else {
                crc.feed_bit(false);
            }
            pos += 1;
            let fdata = &data[pos..(pos + frame_len)];
            let frame = bs.frame_mut(fi);
            for (i, bit) in fdata.iter().enumerate() {
                frame.set(i, *bit);
                if fi == 0 && i < 2 {
                    // ??!?!?!?!?!??!
                    crc.feed_bit(fdata[0]);
                    if i == 1 {
                        crc_enable = !*bit;
                    }
                } else {
                    crc.feed_bit(*bit);
                }
            }
            pos += frame_len;
            let raw_crc = &data[pos..(pos + 4)];
            pos += 4;
            if crc_enable {
                for bit in raw_crc {
                    crc.feed_bit(*bit);
                }
                assert_eq!(crc.crc & 0xf, 0);
            } else {
                assert_eq!(raw_crc, bits![0, 1, 1, 0]);
            }
            if crc_enable && fi == frames_num - 1 {
                for i in (frame_len - 7)..frame_len {
                    frame.set(i, true);
                }
                assert_eq!(crc.crc & 0x7ff, 0);
            }
        }
        let post = &data[pos..(pos + 8)];
        assert_eq!(post, &bits![0, 1, 1, 1, 1, 1, 1, 1]);
        pos += 8;
        while pos % 8 != 0 {
            assert!(data[pos]);
            pos += 1;
        }
        assert_eq!(bitlen, pos + 1);
        let pad = &data[pos..(pos + 8)];
        assert_eq!(pad, &bits![1, 1, 1, 1, 1, 1, 1, 1]);
        pos += 8;
        assert_eq!(pos, data.len());
    }
}

struct Xc5200Crc {
    crc: u16,
}

impl Xc5200Crc {
    fn new() -> Self {
        Self { crc: 0 }
    }

    fn feed_byte(&mut self, b: u8) {
        for i in (0..8).rev() {
            if ((b >> i) & 1) == 0 {
                self.crc ^= 0x8000;
            }
            if (self.crc & 0x8000) != 0 {
                self.crc <<= 1;
                self.crc ^= 0x8005;
            } else {
                self.crc <<= 1;
            }
        }
    }
}

fn parse_xc5200_bitstream(bs: &mut Bitstream, data: &[u8]) {
    let bs = bs.die.first_mut().unwrap();
    let mut crc = Xc5200Crc::new();
    assert_eq!(data[0], 0xff);
    assert_eq!(data[1], 0xf2);
    let bit_length = (data[2] as usize) << 16 | (data[3] as usize) << 8 | (data[4] as usize);
    assert_eq!(data[5], 0xff);
    if bit_length == data.len() * 8 - 7 {
        // OK
    } else if bit_length == data.len() * 8 - 3 {
        bs.regs.insert(Reg::FakeLcAlignmentDone, 1);
    } else {
        panic!(
            "weird length {bit_length} [total {total}]",
            total = data.len() * 8
        );
    }
    let mut pos = 6;
    let frame_len = bs.frame_len;
    let mut crc_enable = false;
    let frames_num = bs.frame_info.len();
    let frame_bytes = frame_len.div_ceil(8);
    for fi in 0..frames_num {
        assert_eq!(data[pos], 0xfe);
        crc.feed_byte(data[pos]);
        pos += 1;
        let frame = bs.frame_mut(fi);
        for j in 0..(frame_bytes * 8) {
            let bit = ((data[pos + j / 8] << (j % 8)) & 0x80) != 0;
            if fi == 0 && j == 0 {
                crc_enable = bit;
            }
            if fi == frames_num - 1 && crc_enable && j >= frame_len - 12 {
                if j < frame_len {
                    frame.set(j, true);
                }
                if j < frame_bytes * 8 - 12 {
                    assert!(!bit);
                }
            } else if j < frame_len {
                frame.set(j, bit);
            } else {
                assert!(!bit);
            }
        }
        for &b in &data[pos..(pos + frame_bytes)] {
            crc.feed_byte(b);
        }
        pos += frame_bytes;
        let fcrc = data[pos] >> 4;
        if !crc_enable {
            assert_eq!(fcrc, 6);
        } else {
            assert_eq!(fcrc, (!crc.crc >> 12) as u8);
        }
        assert_eq!(data[pos] & 0xf, 0xf);
        assert_eq!(data[pos + 1], 0xff);
        assert_eq!(data[pos + 2], 0xff);
        assert_eq!(data[pos + 3], 0xff);
        for &b in &data[pos..(pos + 4)] {
            crc.feed_byte(b);
        }
        pos += 4;
    }
    if crc_enable {
        assert_eq!(crc.crc, 0);
    }
    assert_eq!(data[pos], 0xfe);
    pos += 1;
    for _ in 0..31 {
        assert_eq!(data[pos], 0xff);
        pos += 1;
    }
    assert_eq!(data.len(), pos);
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
enum State {
    None,
    Wcfg,
    Mfwr,
}

fn virtex_far(addr: FrameAddr) -> u32 {
    addr.minor << 9 | addr.major << 17 | addr.typ << 25
}

fn spartan3a_far(addr: FrameAddr) -> u32 {
    addr.minor | addr.major << 16 | addr.typ << 26
}

fn spartan6_far(addr: FrameAddr) -> u32 {
    if addr.typ == 1 {
        // BRAM
        addr.minor << 14 | addr.major << 16 | (addr.region as u32) << 24 | addr.typ << 28
    } else {
        addr.minor | addr.major << 16 | (addr.region as u32) << 24 | addr.typ << 28
    }
}

fn virtex4_far(addr: FrameAddr) -> u32 {
    let (row, bt) = if addr.region < 0 {
        ((-1 - addr.region) as u32, 1)
    } else {
        (addr.region as u32, 0)
    };
    addr.minor | addr.major << 6 | row << 14 | addr.typ << 19 | bt << 22
}

fn virtex5_far(addr: FrameAddr) -> u32 {
    let (row, bt) = if addr.region < 0 {
        ((-1 - addr.region) as u32, 1)
    } else {
        (addr.region as u32, 0)
    };
    addr.minor | addr.major << 7 | row << 15 | bt << 20 | addr.typ << 21
}

fn virtex7_far(addr: FrameAddr) -> u32 {
    let (row, bt) = if addr.region < 0 {
        ((-1 - addr.region) as u32, 1)
    } else {
        (addr.region as u32, 0)
    };
    addr.minor | addr.major << 7 | row << 17 | bt << 22 | addr.typ << 23
}

fn insert_virtex_frame(kind: DeviceKind, bs: &mut DieBitstream, fi: usize, data: &[u8]) {
    let frame_len = bs.frame_len;
    let frame_words = frame_len.div_ceil(32);
    if kind == DeviceKind::Virtex {
        assert_eq!(data.len(), (frame_words + 1) * 4);
    } else {
        assert_eq!(data.len(), frame_words * 4);
    }
    let frame = bs.frame_mut(fi);
    for i in 0..frame_words {
        let word = u32::from_be_bytes(*array_ref!(data, i * 4, 4));
        let bits: BitArray<u32, Lsb0> = BitArray::new(word);
        if i == frame_words - 1 {
            let pad = frame_words * 32 - frame_len;
            for j in pad..32 {
                frame.set(j - pad, bits[j]);
            }
        } else {
            let tgt = frame_len - (i + 1) * 32;
            for j in 0..32 {
                frame.set(tgt + j, bits[j]);
            }
        }
    }
    if bs.frame_present[fi] {
        panic!("FRAME {fi} SET TWICE");
    }
    bs.frame_present.set(fi, true);
}

fn fixup_virtex_frame(kind: DeviceKind, bs: &mut DieBitstream, fi: usize, data: &[u8]) {
    if !bs.frame_present[fi] {
        panic!("FIXUP ON NOT PRESENT FRAME {fi}");
    }
    let frame_len = bs.frame_len;
    let frame_words = frame_len.div_ceil(32);
    if kind == DeviceKind::Virtex {
        assert_eq!(data.len(), (frame_words + 1) * 4);
    } else {
        assert_eq!(data.len(), frame_words * 4);
    }
    let pos = fi * bs.frame_len;
    let frame = &bs.frame_data[pos..pos + bs.frame_len];
    for i in 0..frame_words {
        let word = u32::from_be_bytes(*array_ref!(data, i * 4, 4));
        let bits: BitArray<u32, Lsb0> = BitArray::new(word);
        if i == frame_words - 1 {
            let pad = frame_words * 32 - frame_len;
            for j in pad..32 {
                if frame[j - pad] != bits[j] {
                    bs.frame_fixups.insert((fi, j - pad), bits[j]);
                }
            }
        } else {
            let tgt = frame_len - (i + 1) * 32;
            for j in 0..32 {
                if frame[tgt + j] != bits[j] {
                    bs.frame_fixups.insert((fi, tgt + j), bits[j]);
                }
            }
        }
    }
}

fn insert_spartan3a_frame(bs: &mut DieBitstream, fi: usize, data: &[u8]) {
    let frame_len = bs.frame_len;
    assert_eq!(frame_len % 16, 0);
    let frame_words = frame_len / 16;
    assert_eq!(data.len(), frame_words * 2);
    let frame = bs.frame_mut(fi);
    for i in 0..frame_words {
        let word = u16::from_be_bytes(*array_ref!(data, i * 2, 2));
        let bits: BitArray<u16, Lsb0> = BitArray::new(word);
        let tgt = frame_len - (i + 1) * 16;
        for j in 0..16 {
            frame.set(tgt + j, bits[j]);
        }
    }
    if bs.frame_present[fi] {
        panic!("FRAME {fi} SET TWICE");
    }
    bs.frame_present.set(fi, true);
}

fn insert_spartan6_bram_frame(bs: &mut DieBitstream, fi: usize, data: &[u8]) {
    let frame_len = bs.bram_frame_len;
    assert_eq!(frame_len % 16, 0);
    let frame_words = frame_len / 16;
    assert_eq!(data.len(), frame_words * 2);
    let frame = bs.bram_frame_mut(fi);
    for i in 0..frame_words {
        let word = u16::from_be_bytes(*array_ref!(data, i * 2, 2));
        let bits: BitArray<u16, Lsb0> = BitArray::new(word);
        let tgt = frame_len - (i + 1) * 16;
        for j in 0..16 {
            frame.set(tgt + j, bits[j]);
        }
    }
    if bs.bram_frame_present[fi] {
        panic!("BRAM FRAME {fi} SET TWICE");
    }
    bs.bram_frame_present.set(fi, true);
}

fn insert_spartan6_iob_frame(bs: &mut DieBitstream, data: &[u8]) {
    let frame_len = bs.iob.len();
    assert_eq!(frame_len % 16, 0);
    let frame_words = frame_len / 16;
    assert_eq!(data.len(), frame_words * 2);
    for i in 0..frame_words {
        let word = u16::from_be_bytes(*array_ref!(data, i * 2, 2));
        let bits: BitArray<u16, Lsb0> = BitArray::new(word);
        let tgt = frame_len - (i + 1) * 16;
        for j in 0..16 {
            bs.iob.set(tgt + j, bits[j]);
        }
    }
    if bs.iob_present {
        panic!("IOB FRAME SET TWICE");
    }
    bs.iob_present = true;
}

fn fixup_spartan6_frame(bs: &mut DieBitstream, fi: usize, data: &[u8]) {
    if !bs.frame_present[fi] {
        panic!("FIXUP ON NOT PRESENT FRAME {fi}");
    }
    let frame_len = bs.frame_len;
    let frame_words = frame_len / 16;
    assert_eq!(data.len(), frame_words * 2);
    let pos = fi * bs.frame_len;
    let frame = &bs.frame_data[pos..pos + bs.frame_len];
    for i in 0..frame_words {
        let word = u16::from_be_bytes(*array_ref!(data, i * 2, 2));
        let bits: BitArray<u16, Lsb0> = BitArray::new(word);
        let tgt = frame_len - (i + 1) * 16;
        for j in 0..16 {
            if frame[tgt + j] != bits[j] {
                bs.frame_fixups.insert((fi, tgt + j), bits[j]);
            }
        }
    }
}

fn insert_virtex4_frame(bs: &mut DieBitstream, fi: usize, data: &[u8]) {
    let frame_len = bs.frame_len;
    let frame_words = frame_len.div_ceil(32);
    assert_eq!(data.len(), frame_words * 4);
    let frame = bs.frame_mut(fi);
    for i in 0..frame_words {
        let word = u32::from_be_bytes(*array_ref!(data, i * 4, 4));
        let bits: BitArray<u32, Lsb0> = BitArray::new(word);
        let tgt = i * 32;
        for j in 0..32 {
            frame.set(tgt + j, bits[j]);
        }
    }
    if bs.frame_present[fi] {
        panic!("FRAME {fi} SET TWICE");
    }
    bs.frame_present.set(fi, true);
}

fn parse_virtex_bitstream(bs: &mut Bitstream, data: &[u8], key: &KeyData) {
    let mut packets = PacketParser::new(bs.kind, data, key);
    let kind = bs.kind;
    let bs = bs.die.first_mut().unwrap();
    let far_dict: HashMap<_, _> = bs
        .frame_info
        .iter()
        .enumerate()
        .map(|(i, f)| (virtex_far(f.addr), i))
        .collect();

    assert_eq!(packets.next(), Some(Packet::DummyWord));
    assert_eq!(packets.next(), Some(Packet::SyncWord));
    if packets.peek() == Some(Packet::LoutDebug(0)) {
        packets.next();
        bs.mode = BitstreamMode::Debug;
    }
    assert_eq!(packets.next(), Some(Packet::CmdRcrc));
    let frame_words = if kind == DeviceKind::Virtex {
        bs.frame_len.div_ceil(32) + 1
    } else {
        bs.frame_len / 32
    };
    let frame_bytes = frame_words * 4;
    let flr = (frame_words - 1) as u32;
    let mut early_dghigh = false;
    if packets.peek() == Some(Packet::CmdDGHigh) {
        packets.next();
        early_dghigh = true;
        bs.regs.insert(Reg::FakeEarlyGhigh, 1);
        let mut nops = 0;
        while let Some(Packet::Nop) = packets.peek() {
            packets.next();
            nops += 1;
        }
        assert_eq!(nops, flr + 1);
    }
    assert_eq!(packets.next(), Some(Packet::Flr(flr)));
    match packets.next() {
        Some(Packet::Cor0(val)) => {
            bs.regs.insert(Reg::Cor0, val);
        }
        p => panic!("expected cor0 got {p:?}"),
    }
    if kind != DeviceKind::Virtex {
        match packets.next() {
            Some(Packet::Idcode(val)) => {
                bs.regs.insert(Reg::Idcode, val);
            }
            p => panic!("expected idcode got {p:?}"),
        }
    }
    // TODO: validate?
    let _mask = match packets.next() {
        Some(Packet::Mask(val)) => val,
        p => panic!("expected mask got {p:?}"),
    };
    match packets.next() {
        Some(Packet::CmdSwitch) => {
            bs.regs.insert(Reg::FakeHasSwitch, 1);
        }
        Some(Packet::CmdNull) => (),
        p => panic!("expected switch or null got {p:?}"),
    }

    // main loop
    let mut fi = 0;
    let mut last_frame = None;
    if bs.mode == BitstreamMode::Debug {
        assert_eq!(packets.next(), Some(Packet::Far(0)));
        assert_eq!(packets.next(), Some(Packet::CmdWcfg));
        loop {
            let val = match packets.next() {
                Some(Packet::Fdri(val)) => {
                    assert_eq!(val.len(), frame_bytes);
                    val
                }
                Some(Packet::Crc) if kind == DeviceKind::Virtex => {
                    fi -= 1;
                    break;
                }
                p => panic!("expected fdri got {p:?}"),
            };
            if kind == DeviceKind::Virtex {
                match packets.next() {
                    Some(Packet::Crc) => (),
                    Some(Packet::Far(far)) => {
                        assert_eq!(far_dict[&far], fi);
                        continue;
                    }
                    p => panic!("expected crc/far got {p:?}"),
                }
            }
            if let Some(Packet::LoutDebug(far)) = packets.peek() {
                assert_eq!(far_dict[&far], fi);
                packets.next();
                insert_virtex_frame(kind, bs, fi, &val);
                fi += 1;
            } else {
                if kind == DeviceKind::Virtex {
                    panic!("expected loutdebug got {p:?}", p = packets.next());
                }
                break;
            }
        }
    } else {
        let mut state = State::None;
        loop {
            match packets.peek() {
                Some(Packet::Far(far)) => {
                    packets.next();
                    fi = far_dict[&far];
                }
                Some(Packet::CmdWcfg) => {
                    packets.next();
                    assert_ne!(state, State::Wcfg);
                    state = State::Wcfg;
                    match packets.next() {
                        Some(Packet::Far(far)) => {
                            fi = far_dict[&far];
                        }
                        p => panic!("expected far got {p:?}"),
                    }
                }
                _ => break,
            }
            match packets.peek() {
                Some(Packet::CmdWcfg) => {
                    assert_ne!(state, State::Wcfg);
                    state = State::Wcfg;
                    packets.next();
                }
                Some(Packet::CmdMfwr) => {
                    assert_ne!(state, State::Mfwr);
                    bs.mode = BitstreamMode::Compress;
                    state = State::Mfwr;
                    packets.next();
                }
                _ => (),
            }
            match (packets.peek(), state) {
                (Some(Packet::Fdri(val)), State::Wcfg) => {
                    packets.next();
                    let frames = val.len() / frame_bytes;
                    assert_eq!(val.len() % frame_bytes, 0);
                    for i in 0..(frames - 1) {
                        let pos = i * frame_bytes;
                        insert_virtex_frame(kind, bs, fi, &val[pos..pos + frame_bytes]);
                        fi += 1;
                    }
                    let last = val[(frames - 1) * frame_bytes..].to_vec();
                    last_frame = Some(last);
                }
                (Some(Packet::Mfwr(2)), State::Mfwr) => {
                    packets.next();
                    insert_virtex_frame(kind, bs, fi, last_frame.as_ref().unwrap());
                }
                (Some(Packet::Far(_)), _) => continue,
                (Some(Packet::Key(key)), State::Wcfg) => {
                    bs.mode = BitstreamMode::Encrypt;
                    bs.regs.insert(Reg::Key, key);
                    packets.next();
                    assert_eq!(packets.next(), Some(Packet::Nop));
                    match packets.next() {
                        Some(Packet::Cbc(iv)) => {
                            bs.iv = iv;
                        }
                        p => panic!("expected iv got {p:?}"),
                    }
                    match packets.next() {
                        Some(Packet::EncFdri(val)) => {
                            let frames = val.len() / frame_bytes;
                            assert_eq!(val.len() % frame_bytes, 0);
                            for i in 0..(frames - 1) {
                                let pos = i * frame_bytes;
                                insert_virtex_frame(kind, bs, fi, &val[pos..pos + frame_bytes]);
                                fi += 1;
                            }
                            let last = val[(frames - 1) * frame_bytes..].to_vec();
                            last_frame = Some(last);
                        }
                        p => panic!("expected fdri got {p:?}"),
                    }
                    break;
                }
                (p, _) => {
                    panic!("UNEXPECTED PACKET IN STATE {state:?}: {p:#x?}");
                }
            }
        }
        if kind == DeviceKind::Virtex {
            assert_eq!(packets.next(), Some(Packet::Crc));
        }
    }

    if kind != DeviceKind::Virtex {
        assert_eq!(packets.next(), Some(Packet::CmdGRestore));
    }
    if !early_dghigh {
        assert_eq!(packets.next(), Some(Packet::CmdDGHigh));
    }

    if kind == DeviceKind::Virtex {
        match packets.next() {
            Some(Packet::Fdri(val)) => {
                assert_eq!(val.len(), frame_bytes);
                assert!(val.iter().all(|&x| x == 0));
            }
            p => panic!("expected fdri got {p:?}"),
        }
        if !bs.frame_present[fi] {
            insert_virtex_frame(kind, bs, fi, &last_frame.unwrap());
        }
        assert!(bs.frame_present.all());
    } else {
        assert!(bs.frame_present.all());

        if !early_dghigh {
            let mut nops = 0;
            while let Some(Packet::Nop) = packets.peek() {
                packets.next();
                nops += 1;
            }

            if packets.peek() == Some(Packet::CmdWcfg) {
                bs.regs.insert(Reg::FakeFreezeDciNops, nops);
                packets.next();
                while let Some(Packet::Far(far)) = packets.peek() {
                    packets.next();
                    match packets.next() {
                        Some(Packet::Fdri(val)) => {
                            let frames = val.len() / frame_bytes;
                            assert_eq!(val.len() % frame_bytes, 0);
                            fi = far_dict[&far];
                            for i in 0..(frames - 1) {
                                let pos = i * frame_bytes;
                                fixup_virtex_frame(kind, bs, fi, &val[pos..pos + frame_bytes]);
                                fi += 1;
                            }
                        }
                        p => panic!("expected fdri got {p:?}"),
                    }
                }
            } else {
                assert_eq!(nops, flr + 1);
            }
        }

        if packets.peek() == Some(Packet::CmdGRestore) {
            bs.regs.insert(Reg::FakeDoubleGrestore, 1);
            packets.next();
        }
    }

    assert_eq!(packets.next(), Some(Packet::CmdStart));
    match packets.next() {
        Some(Packet::Ctl0(val)) => {
            bs.regs.insert(Reg::Ctl0, val);
        }
        p => panic!("expected ctl0 got {p:?}"),
    }
    assert_eq!(packets.next(), Some(Packet::Crc));
    if kind != DeviceKind::Virtex {
        assert_eq!(packets.next(), Some(Packet::CmdDesynch));
    }
    for _ in 0..4 {
        assert_eq!(packets.next(), Some(Packet::Nop));
    }
    assert_eq!(packets.next(), None);
}

fn parse_spartan3a_bitstream(bs: &mut Bitstream, data: &[u8], key: &KeyData) {
    let mut packets = PacketParser::new(bs.kind, data, key);
    let bs = bs.die.first_mut().unwrap();
    let far_dict: HashMap<_, _> = bs
        .frame_info
        .iter()
        .enumerate()
        .map(|(i, f)| (spartan3a_far(f.addr), i))
        .collect();

    for _ in 0..16 {
        assert_eq!(packets.next(), Some(Packet::DummyWord));
    }
    assert_eq!(packets.next(), Some(Packet::SyncWord));
    if packets.peek() == Some(Packet::LoutDebug(0)) {
        packets.next();
        bs.mode = BitstreamMode::Debug;
    }
    assert_eq!(packets.next(), Some(Packet::CmdRcrc));
    assert_eq!(packets.next(), Some(Packet::Nop));
    match packets.next() {
        Some(Packet::Cor2(val)) => {
            bs.regs.insert(Reg::Cor2, val);
        }
        p => panic!("expected cor2 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::CclkFrequency(val)) => {
            bs.regs.insert(Reg::CclkFrequency, val);
        }
        p => panic!("expected cclk frequency got {p:?}"),
    }
    let frame_bytes = bs.frame_len / 8;
    let flr = (bs.frame_len / 16 - 1) as u32;
    assert_eq!(packets.next(), Some(Packet::Flr(flr)));
    match packets.next() {
        Some(Packet::Cor1(val)) => {
            bs.regs.insert(Reg::Cor1, val);
        }
        p => panic!("expected cor1 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::Idcode(val)) => {
            bs.regs.insert(Reg::Idcode, val);
        }
        p => panic!("expected idcode got {p:?}"),
    }
    let _mask = match packets.next() {
        Some(Packet::Mask(val)) => val,
        p => panic!("expected mask got {p:?}"),
    };
    match packets.next() {
        Some(Packet::Ctl0(val)) => {
            bs.regs.insert(Reg::Ctl0, val);
        }
        p => panic!("expected ctl0 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::Powerdown(val)) => {
            bs.regs.insert(Reg::Powerdown, val);
        }
        p => panic!("expected powerdown got {p:?}"),
    }
    match packets.next() {
        Some(Packet::HcOpt(val)) => {
            bs.regs.insert(Reg::HcOpt, val);
        }
        p => panic!("expected hc opt got {p:?}"),
    }
    match packets.next() {
        Some(Packet::PuGwe(val)) => {
            bs.regs.insert(Reg::PuGwe, val);
        }
        p => panic!("expected pu gwe got {p:?}"),
    }
    match packets.next() {
        Some(Packet::PuGts(val)) => {
            bs.regs.insert(Reg::PuGts, val);
        }
        p => panic!("expected pu gts got {p:?}"),
    }
    match packets.next() {
        Some(Packet::Mode(val)) => {
            bs.regs.insert(Reg::Mode, val);
            match packets.next() {
                Some(Packet::General1(val)) => {
                    bs.regs.insert(Reg::General1, val);
                }
                p => panic!("expected general1 got {p:?}"),
            }
            match packets.next() {
                Some(Packet::General2(val)) => {
                    bs.regs.insert(Reg::General2, val);
                }
                p => panic!("expected general2 got {p:?}"),
            }
        }
        Some(Packet::Nop) => {
            for _ in 0..5 {
                assert_eq!(packets.next(), Some(Packet::Nop));
            }
        }
        p => panic!("expected mode got {p:?}"),
    }
    match packets.next() {
        Some(Packet::SeuOpt(val)) => {
            bs.regs.insert(Reg::SeuOpt, val);
        }
        p => panic!("expected seuopt got {p:?}"),
    }
    match packets.next() {
        Some(Packet::RbCrcSw(val)) => {
            bs.regs.insert(Reg::RbCrcSw, val);
        }
        p => panic!("expected rbcrcsw got {p:?}"),
    }

    // main loop
    if bs.mode == BitstreamMode::Debug {
        assert_eq!(packets.next(), Some(Packet::Far(0)));
        assert_eq!(packets.next(), Some(Packet::CmdWcfg));
        let mut fi = 0;
        loop {
            let val = match packets.next() {
                Some(Packet::Fdri(val)) => {
                    assert_eq!(val.len(), frame_bytes);
                    val
                }
                p => panic!("expected fdri got {p:?}"),
            };
            if let Some(Packet::LoutDebug(far)) = packets.peek() {
                assert_eq!(far_dict[&far], fi);
                packets.next();
                insert_spartan3a_frame(bs, fi, &val);
                fi += 1;
            } else {
                assert!(val.iter().all(|&x| x == 0));
                break;
            }
        }
    } else {
        let mut state = State::None;
        let mut mfwr_frame = None;
        loop {
            let far = match packets.peek() {
                Some(Packet::Far(far)) => {
                    packets.next();
                    far
                }
                Some(Packet::CmdWcfg) => {
                    packets.next();
                    assert_ne!(state, State::Wcfg);
                    state = State::Wcfg;
                    match packets.next() {
                        Some(Packet::Far(far)) => far,
                        p => panic!("expected far got {p:?}"),
                    }
                }
                _ => break,
            };
            match packets.peek() {
                Some(Packet::CmdWcfg) => {
                    assert_ne!(state, State::Wcfg);
                    state = State::Wcfg;
                    packets.next();
                }
                Some(Packet::CmdMfwr) => {
                    assert_ne!(state, State::Mfwr);
                    bs.mode = BitstreamMode::Compress;
                    state = State::Mfwr;
                    packets.next();
                }
                _ => (),
            }
            match (packets.next(), state) {
                (Some(Packet::Fdri(val)), State::Wcfg) => {
                    let frames = val.len() / frame_bytes;
                    assert_eq!(val.len() % frame_bytes, 0);
                    if frames > 1 {
                        let fi = far_dict[&far];
                        for i in 0..(frames - 1) {
                            let pos = i * frame_bytes;
                            insert_spartan3a_frame(bs, fi + i, &val[pos..pos + frame_bytes]);
                        }
                    }
                    mfwr_frame = Some(val[(frames - 1) * frame_bytes..].to_vec());
                }
                (Some(Packet::Mfwr(4)), State::Mfwr) => {
                    let fi = far_dict[&far];
                    assert_ne!(bs.frame_info[fi].addr.typ, 1);
                    insert_spartan3a_frame(bs, fi, mfwr_frame.as_ref().unwrap());
                }
                (Some(Packet::Mfwr(14)), State::Mfwr) => {
                    let fi = far_dict[&far];
                    assert_eq!(bs.frame_info[fi].addr.typ, 1);
                    insert_spartan3a_frame(bs, fi, mfwr_frame.as_ref().unwrap());
                }
                p => {
                    panic!("UNEXPECTED PACKET IN STATE {state:?}: {p:#x?}");
                }
            }
        }
    }

    assert!(bs.frame_present.all());

    assert_eq!(packets.next(), Some(Packet::Crc));
    assert_eq!(packets.next(), Some(Packet::CmdGRestore));
    assert_eq!(packets.next(), Some(Packet::CmdDGHigh));
    for _ in 0..4 {
        assert_eq!(packets.next(), Some(Packet::Nop));
    }
    assert_eq!(packets.next(), Some(Packet::CmdStart));
    let _mask2 = match packets.next() {
        Some(Packet::Mask(val)) => val,
        p => panic!("expected mask got {p:?}"),
    };
    let _ctl0_2 = match packets.next() {
        Some(Packet::Ctl0(val)) => val,
        p => panic!("expected ctl0 got {p:?}"),
    };
    assert_eq!(packets.next(), Some(Packet::Crc));
    assert_eq!(packets.next(), Some(Packet::CmdDesynch));
    for _ in 0..16 {
        assert_eq!(packets.next(), Some(Packet::Nop));
    }
    assert_eq!(packets.next(), None);
}

fn parse_spartan6_bitstream(bs: &mut Bitstream, data: &[u8], key: &KeyData) {
    let mut packets = PacketParser::new(bs.kind, data, key);
    let bs = bs.die.first_mut().unwrap();
    let far_dict: HashMap<_, _> = bs
        .frame_info
        .iter()
        .enumerate()
        .map(|(i, f)| (spartan6_far(f.addr), i))
        .collect();
    let bram_far_dict: HashMap<_, _> = bs
        .bram_frame_info
        .iter()
        .enumerate()
        .map(|(i, f)| (spartan6_far(f.addr), i))
        .collect();

    for _ in 0..8 {
        assert_eq!(packets.next(), Some(Packet::DummyWord));
    }
    assert_eq!(packets.next(), Some(Packet::SyncWord));
    assert_eq!(packets.next(), Some(Packet::CmdRcrc));
    assert_eq!(packets.next(), Some(Packet::Nop));
    let flr = (bs.iob.len() / 16) as u32;
    assert_eq!(packets.next(), Some(Packet::Flr(flr)));
    match packets.next() {
        Some(Packet::Cor1(val)) => {
            bs.regs.insert(Reg::Cor1, val);
        }
        p => panic!("expected cor1 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::Cor2(val)) => {
            bs.regs.insert(Reg::Cor2, val);
        }
        p => panic!("expected cor2 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::Idcode(val)) => {
            bs.regs.insert(Reg::Idcode, val);
        }
        p => panic!("expected idcode got {p:?}"),
    }
    let _mask = match packets.next() {
        Some(Packet::Mask(val)) => val,
        p => panic!("expected mask got {p:?}"),
    };
    match packets.next() {
        Some(Packet::Ctl0(val)) => {
            bs.regs.insert(Reg::Ctl0, val);
        }
        p => panic!("expected ctl0 got {p:?}"),
    }
    for _ in 0..8 {
        assert_eq!(packets.next(), Some(Packet::Nop));
    }
    match packets.next() {
        Some(Packet::Cbc(_)) => {
            assert_eq!(bs.regs[&Reg::Ctl0] & 0x40, 0x40);
            bs.mode = BitstreamMode::Encrypt;
        }
        Some(Packet::Nop) => {
            assert_eq!(bs.regs[&Reg::Ctl0] & 0x40, 0);
            for _ in 0..8 {
                assert_eq!(packets.next(), Some(Packet::Nop));
            }
        }
        p => panic!("expected cbc or nop got {p:?}"),
    }
    match packets.next() {
        Some(Packet::CclkFrequency(val)) => {
            bs.regs.insert(Reg::CclkFrequency, val);
        }
        p => panic!("expected cclk frequency got {p:?}"),
    }
    if let Some(Packet::CclkFrequency(val)) = packets.peek() {
        packets.next();
        bs.regs.insert(Reg::CclkFrequency, val);
        bs.regs.insert(Reg::FakeDoubleCclkFrequency, 1);
    }
    match packets.next() {
        Some(Packet::Powerdown(val)) => {
            bs.regs.insert(Reg::Powerdown, val);
        }
        p => panic!("expected powerdown got {p:?}"),
    }
    match packets.next() {
        Some(Packet::EyeMask(val)) => {
            bs.regs.insert(Reg::EyeMask, val);
        }
        p => panic!("expected eyemask got {p:?}"),
    }
    match packets.next() {
        Some(Packet::HcOpt(val)) => {
            bs.regs.insert(Reg::HcOpt, val);
        }
        p => panic!("expected hc opt got {p:?}"),
    }
    match packets.next() {
        Some(Packet::Timer(val)) => {
            bs.regs.insert(Reg::Timer, val);
        }
        p => panic!("expected timer got {p:?}"),
    }
    match packets.next() {
        Some(Packet::PuGwe(val)) => {
            bs.regs.insert(Reg::PuGwe, val);
        }
        p => panic!("expected pu gwe got {p:?}"),
    }
    match packets.next() {
        Some(Packet::PuGts(val)) => {
            bs.regs.insert(Reg::PuGts, val);
        }
        p => panic!("expected pu gts got {p:?}"),
    }
    match packets.next() {
        Some(Packet::Mode(val)) => {
            bs.regs.insert(Reg::Mode, val);
        }
        p => panic!("expected mode got {p:?}"),
    }
    match packets.next() {
        Some(Packet::General1(val)) => {
            bs.regs.insert(Reg::General1, val);
        }
        p => panic!("expected general1 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::General2(val)) => {
            bs.regs.insert(Reg::General2, val);
        }
        p => panic!("expected general2 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::General3(val)) => {
            bs.regs.insert(Reg::General3, val);
        }
        p => panic!("expected general3 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::General4(val)) => {
            bs.regs.insert(Reg::General4, val);
        }
        p => panic!("expected general4 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::General5(val)) => {
            bs.regs.insert(Reg::General5, val);
        }
        p => panic!("expected general5 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::SeuOpt(val)) => {
            bs.regs.insert(Reg::SeuOpt, val);
        }
        p => panic!("expected seuopt got {p:?}"),
    }
    match packets.next() {
        Some(Packet::RbCrcSw(val)) => {
            bs.regs.insert(Reg::RbCrcSw, val);
        }
        p => panic!("expected rbcrcsw got {p:?}"),
    }
    if let Some(Packet::Testmode(val)) = packets.peek() {
        packets.next();
        bs.regs.insert(Reg::Testmode, val);
    } else if !bs.regs.contains_key(&Reg::FakeDoubleCclkFrequency) {
        assert_eq!(packets.next(), Some(Packet::Nop));
        assert_eq!(packets.next(), Some(Packet::Nop));
    }

    // main loop
    #[derive(Debug, Copy, Clone)]
    enum Frame {
        None,
        Main(usize),
        Bram(usize),
        Iob,
    }
    let frame_bytes = bs.frame_len / 8;
    let bram_frame_bytes = bs.bram_frame_len / 8;
    let iob_frame_bytes = bs.iob.len() / 8;
    let mut frame = Frame::None;
    let mut skip = 0;
    let mut last_frame: Option<Vec<u8>> = None;
    let mut state = State::None;
    if bs.mode != BitstreamMode::Encrypt {
        loop {
            match packets.peek() {
                Some(Packet::Far(far)) => {
                    frame = if let Some(&fi) = far_dict.get(&far) {
                        if bs.frame_present[fi] {
                            break;
                        }
                        Frame::Main(fi)
                    } else if let Some(&fi) = bram_far_dict.get(&far) {
                        Frame::Bram(fi)
                    } else if far == 0x20000000 {
                        Frame::Iob
                    } else {
                        panic!("weird FAR {far:08x}")
                    };
                    packets.next();
                    skip = 1;
                }
                Some(Packet::CmdWcfg) => {
                    packets.next();
                    assert_ne!(state, State::Wcfg);
                    state = State::Wcfg;
                }
                Some(Packet::CmdMfwr) => {
                    packets.next();
                    assert_ne!(state, State::Mfwr);
                    state = State::Mfwr;
                    for _ in 0..8 {
                        assert_eq!(packets.next(), Some(Packet::Nop));
                    }
                }
                Some(Packet::Mfwr(4)) => {
                    packets.next();
                    bs.mode = BitstreamMode::Compress;
                    assert_eq!(state, State::Mfwr);
                    let Frame::Main(fi) = frame else {
                        panic!("mfwr in weird frame {frame:?}");
                    };
                    insert_spartan3a_frame(bs, fi, last_frame.as_ref().unwrap());
                }
                Some(Packet::Fdri(orig_data)) => {
                    let mut data = &orig_data[..];
                    packets.next();
                    while !data.is_empty() {
                        match frame {
                            Frame::Main(fi) => {
                                let cur = &data[..frame_bytes];
                                data = &data[frame_bytes..];
                                if skip != 0 {
                                    skip -= 1;
                                    if fi == bs.frame_info.len() {
                                        frame = Frame::Bram(0);
                                    }
                                } else {
                                    insert_spartan3a_frame(bs, fi, last_frame.as_ref().unwrap());
                                    frame = Frame::Main(fi + 1);
                                    if fi == bs.frame_info.len() - 1
                                        || bs.frame_info[fi + 1].addr.region
                                            != bs.frame_info[fi].addr.region
                                    {
                                        skip = 2;
                                    }
                                }
                                last_frame = Some(cur.to_vec());
                            }
                            Frame::Bram(fi) => {
                                let cur = &data[..bram_frame_bytes];
                                data = &data[bram_frame_bytes..];
                                insert_spartan6_bram_frame(bs, fi, cur);
                                if fi == bs.bram_frame_info.len() - 1 {
                                    frame = Frame::Iob;
                                } else {
                                    frame = Frame::Bram(fi + 1)
                                }
                            }
                            Frame::Iob => {
                                assert_eq!(data.len(), iob_frame_bytes + 2);
                                insert_spartan6_iob_frame(bs, &data[..iob_frame_bytes]);
                                data = &[];
                                frame = Frame::None;
                            }
                            _ => panic!("fdri {l} with no frame", l = data.len()),
                        }
                    }
                }
                _ => break,
            }
        }
        if bs.mode != BitstreamMode::Compress {
            for _ in 0..24 {
                assert_eq!(packets.next(), Some(Packet::Nop));
            }
        }
    } else {
        assert_eq!(packets.next(), Some(Packet::Far(0)));
        assert_eq!(packets.next(), Some(Packet::CmdWcfg));
        match packets.next() {
            Some(Packet::EncFdri(orig_data)) => {
                let num = orig_data.len() / 2;
                let mut data = &orig_data[..];
                frame = Frame::Main(0);
                skip = 1;
                while !data.is_empty() {
                    match frame {
                        Frame::Main(fi) => {
                            let cur = &data[..frame_bytes];
                            data = &data[frame_bytes..];
                            if skip != 0 {
                                skip -= 1;
                                if fi == bs.frame_info.len() {
                                    frame = Frame::Bram(0);
                                }
                            } else {
                                insert_spartan3a_frame(bs, fi, last_frame.as_ref().unwrap());
                                frame = Frame::Main(fi + 1);
                                if fi == bs.frame_info.len() - 1
                                    || bs.frame_info[fi + 1].addr.region
                                        != bs.frame_info[fi].addr.region
                                {
                                    skip = 2;
                                }
                            }
                            last_frame = Some(cur.to_vec());
                        }
                        Frame::Bram(fi) => {
                            let cur = &data[..bram_frame_bytes];
                            data = &data[bram_frame_bytes..];
                            insert_spartan6_bram_frame(bs, fi, cur);
                            if fi == bs.bram_frame_info.len() - 1 {
                                frame = Frame::Iob;
                            } else {
                                frame = Frame::Bram(fi + 1)
                            }
                        }
                        Frame::Iob => {
                            assert_eq!(data.len(), iob_frame_bytes + 2);
                            insert_spartan6_iob_frame(bs, &data[..iob_frame_bytes]);
                            data = &[];
                            frame = Frame::None;
                        }
                        _ => panic!("fdri {l} with no frame", l = data.len()),
                    }
                }
                let mut real_num = num + 2;
                while real_num % 8 != 0 {
                    real_num += 1;
                }
                let exp_nops = 24 - (real_num - num - 2);
                for _ in 0..exp_nops {
                    assert_eq!(packets.next(), Some(Packet::Nop));
                }
            }
            p => panic!("expected enc fdri got {p:?}"),
        }
    }

    let mut first = true;
    while let Some(Packet::Far(far)) = packets.peek() {
        packets.next();
        let Some(&fi) = far_dict.get(&far) else {
            panic!("weird fixup FAR {far:08x}");
        };
        if first {
            assert_eq!(packets.next(), Some(Packet::CmdWcfg));
            first = false;
        }
        let data = match packets.next() {
            Some(Packet::Fdri(val)) => val,
            p => panic!("expected fdri got {p:?}"),
        };
        let mut fi = fi;
        let frames = data.len() / frame_bytes;
        assert_eq!(data.len() % frame_bytes, 0);
        for i in 0..(frames - 1) {
            let pos = i * frame_bytes;
            fixup_spartan6_frame(bs, fi, &data[pos..pos + frame_bytes]);
            fi += 1;
        }
    }

    assert_eq!(packets.next(), Some(Packet::CmdGRestore));
    assert_eq!(packets.next(), Some(Packet::CmdDGHigh));
    for _ in 0..4 {
        assert_eq!(packets.next(), Some(Packet::Nop));
    }
    assert_eq!(packets.next(), Some(Packet::CmdGRestore));
    assert_eq!(packets.next(), Some(Packet::CmdStart));
    let _mask = match packets.next() {
        Some(Packet::Mask(val)) => val,
        p => panic!("expected mask got {p:?}"),
    };
    match packets.next() {
        Some(Packet::Ctl0(val)) => {
            bs.regs.insert(Reg::Ctl0, val);
        }
        p => panic!("expected ctl0 got {p:?}"),
    }
    assert_eq!(packets.next(), Some(Packet::Crc));
    assert_eq!(packets.next(), Some(Packet::CmdDesynch));
    for _ in 0..14 {
        assert_eq!(packets.next(), Some(Packet::Nop));
    }
    assert_eq!(packets.next(), None);
}

fn parse_virtex4_bitstream(
    bs: &mut Bitstream,
    data: &[u8],
    key: &KeyData,
    geom: &BitstreamGeom,
    die_index: usize,
) {
    let die = geom.die_order[die_index];
    let mut packets = PacketParser::new(bs.kind, data, key);
    let kind = bs.kind;
    let diebs = &mut bs.die[die];
    let far_dict: HashMap<_, _> = diebs
        .frame_info
        .iter()
        .enumerate()
        .map(|(i, f)| {
            (
                match kind {
                    DeviceKind::Virtex4 => virtex4_far(f.addr),
                    DeviceKind::Virtex5 | DeviceKind::Virtex6 => virtex5_far(f.addr),
                    DeviceKind::Virtex7 => virtex7_far(f.addr),
                    _ => unreachable!(),
                },
                i,
            )
        })
        .collect();

    let mut ctl0 = 0;
    let mut trim_regs = 0;

    if kind == DeviceKind::Virtex4 {
        assert_eq!(packets.next(), Some(Packet::DummyWord));
        assert_eq!(packets.next(), Some(Packet::SyncWord));
        assert_eq!(packets.next(), Some(Packet::Nop));
        assert_eq!(packets.next(), Some(Packet::CmdRcrc));
        assert_eq!(packets.next(), Some(Packet::Nop));
        assert_eq!(packets.next(), Some(Packet::Nop));
        match packets.next() {
            Some(Packet::Cor0(val)) => {
                diebs.regs.insert(Reg::Cor0, val);
            }
            p => panic!("expected cor0 got {p:?}"),
        }
        match packets.next() {
            Some(Packet::Idcode(val)) => {
                diebs.regs.insert(Reg::Idcode, val);
            }
            p => panic!("expected idcode got {p:?}"),
        }
        if matches!(packets.peek(), Some(Packet::Mask(_))) {
            let mask = match packets.next() {
                Some(Packet::Mask(val)) => val,
                p => panic!("expected mask got {p:?}"),
            };
            match packets.next() {
                Some(Packet::Ctl0(val)) => {
                    assert_eq!(mask & 0x40, 0x40);
                    assert_eq!(val & 0x40, 0x40);
                    ctl0 = (ctl0 & !mask) | (val & mask);
                }
                p => panic!("expected ctl0 got {p:?}"),
            }
            diebs.mode = BitstreamMode::Encrypt;
        }
        assert_eq!(packets.next(), Some(Packet::CmdSwitch));
        assert_eq!(packets.next(), Some(Packet::Nop));
        match packets.next() {
            Some(Packet::CmdNull) => {
                // the following block is just missing in xqr4v?!?
            }
            Some(Packet::Mask(mask)) => {
                match packets.next() {
                    Some(Packet::Ctl0(val)) => ctl0 = (ctl0 & !mask) | (val & mask),
                    p => panic!("expected ctl0 got {p:?}"),
                }
                for _ in 0..1150 {
                    assert_eq!(packets.next(), Some(Packet::Nop));
                }
                let mask = match packets.next() {
                    Some(Packet::Mask(val)) => val,
                    p => panic!("expected mask got {p:?}"),
                };
                match packets.next() {
                    Some(Packet::Ctl0(val)) => ctl0 = (ctl0 & !mask) | (val & mask),
                    p => panic!("expected ctl0 got {p:?}"),
                }
                assert_eq!(packets.next(), Some(Packet::CmdNull));
            }
            p => panic!("expected mask got {p:?}"),
        };
        assert_eq!(packets.next(), Some(Packet::Nop));
    } else {
        for _ in 0..8 {
            assert_eq!(packets.next(), Some(Packet::DummyWord));
        }
        assert_eq!(packets.next(), Some(Packet::WidthDetect));
        assert_eq!(packets.next(), Some(Packet::DummyWord));
        assert_eq!(packets.next(), Some(Packet::DummyWord));
        assert_eq!(packets.next(), Some(Packet::SyncWord));
        assert_eq!(packets.next(), Some(Packet::Nop));
        if matches!(packets.peek(), Some(Packet::Bspi(_))) {
            match packets.next() {
                Some(Packet::Bspi(val)) => {
                    diebs.regs.insert(Reg::Bspi, val);
                    assert_eq!(packets.next(), Some(Packet::CmdBspiRead));
                    assert_eq!(packets.next(), Some(Packet::Nop));
                }
                p => panic!("expected bspi got {p:?}"),
            }
        }
        if matches!(packets.peek(), Some(Packet::Mask(_))) {
            let mask = match packets.next() {
                Some(Packet::Mask(val)) => val,
                p => panic!("expected mask got {p:?}"),
            };
            match packets.next() {
                Some(Packet::Ctl0(val)) => ctl0 = (ctl0 & !mask) | (val & mask),
                p => panic!("expected ctl0 got {p:?}"),
            }
            assert_eq!(ctl0 & 0x40, 0x40);
            if kind == DeviceKind::Virtex7 {
                match packets.next() {
                    Some(Packet::Cor1(val)) => {
                        diebs.regs.insert(Reg::Cor1, val);
                    }
                    p => panic!("expected cor1 got {p:?}"),
                }
                for _ in 0..14 {
                    assert_eq!(packets.next(), Some(Packet::Nop));
                }
            } else {
                for _ in 0..16 {
                    assert_eq!(packets.next(), Some(Packet::Nop));
                }
            }
            match packets.next() {
                Some(Packet::Cbc(val)) => diebs.iv = val,
                p => panic!("expected fdri got {p:?}"),
            };

            match packets.next() {
                Some(Packet::Dwc(_)) => (),
                p => panic!("expected dwc got {p:?}"),
            };

            diebs.regs.insert(Reg::FakeEncrypted, 1);
        }
        if kind == DeviceKind::Virtex7 {
            match packets.next() {
                Some(Packet::Timer(val)) => {
                    diebs.regs.insert(Reg::Timer, val);
                }
                p => panic!("expected timer got {p:?}"),
            }
        }
        match packets.next() {
            Some(Packet::WBStar(val)) => {
                diebs.regs.insert(Reg::WbStar, val);
            }
            p => panic!("expected wbstar got {p:?}"),
        }
        assert_eq!(packets.next(), Some(Packet::CmdNull));
        assert_eq!(packets.next(), Some(Packet::Nop));
        while matches!(packets.peek(), Some(Packet::Mask(_))) {
            let _mask = match packets.next() {
                Some(Packet::Mask(val)) => val,
                p => panic!("expected mask got {p:?}"),
            };
            match packets.next() {
                Some(Packet::Unk1c(val)) => {
                    diebs.regs.insert(Reg::Unk1C, val);
                }
                Some(Packet::Trim(val)) => {
                    diebs.regs.insert(Reg::Trim0, val);
                }
                p => panic!("expected ctl2 or trim got {p:?}"),
            }
        }
        while matches!(packets.peek(), Some(Packet::Cor1(_))) {
            let cor1 = match packets.next() {
                Some(Packet::Cor1(val)) => val,
                p => panic!("expected cor1 got {p:?}"),
            };
            let _mask = match packets.next() {
                Some(Packet::Mask(val)) => val,
                p => panic!("expected mask got {p:?}"),
            };
            let reg = match cor1 {
                0x1000 => Reg::Trim0,
                0x1400 => Reg::Trim1,
                0x1800 => Reg::Trim2,
                _ => panic!("unexpected cor1 val {cor1:#x}"),
            };
            match packets.next() {
                Some(Packet::Trim(val)) => {
                    diebs.regs.insert(reg, val);
                }
                p => panic!("expected trim got {p:?}"),
            }
            trim_regs += 1;
        }
        if kind != DeviceKind::Virtex5 && matches!(packets.peek(), Some(Packet::Testmode(_))) {
            match packets.next() {
                Some(Packet::Testmode(val)) => {
                    diebs.regs.insert(Reg::Testmode, val);
                }
                p => panic!("expected testmode got {p:?}"),
            }
        }
        assert_eq!(packets.next(), Some(Packet::CmdRcrc));
        assert_eq!(packets.next(), Some(Packet::Nop));
        assert_eq!(packets.next(), Some(Packet::Nop));
        if kind != DeviceKind::Virtex7 {
            match packets.next() {
                Some(Packet::Timer(val)) => {
                    diebs.regs.insert(Reg::Timer, val);
                }
                p => panic!("expected timer got {p:?}"),
            }
        }
        match packets.next() {
            Some(Packet::RbCrcSw(val)) => {
                diebs.regs.insert(Reg::RbCrcSw, val);
            }
            p => panic!("expected rbcrcsw got {p:?}"),
        }
        if kind == DeviceKind::Virtex5 && matches!(packets.peek(), Some(Packet::Testmode(_))) {
            match packets.next() {
                Some(Packet::Testmode(val)) => {
                    diebs.regs.insert(Reg::Testmode, val);
                }
                p => panic!("expected testmode got {p:?}"),
            }
        }
        match packets.next() {
            Some(Packet::Cor0(val)) => {
                diebs.regs.insert(Reg::Cor0, val);
            }
            p => panic!("expected cor0 got {p:?}"),
        }
        match packets.next() {
            Some(Packet::Cor1(val)) => {
                diebs.regs.insert(Reg::Cor1, val);
            }
            p => panic!("expected cor1 got {p:?}"),
        }
        match packets.next() {
            Some(Packet::Idcode(val)) => {
                diebs.regs.insert(Reg::Idcode, val);
            }
            p => panic!("expected idcode got {p:?}"),
        }
        match packets.next() {
            Some(Packet::CmdSwitch) => (),
            Some(Packet::CmdFallEdge) => {
                diebs.regs.insert(Reg::FakeFallEdge, 0);
                assert_eq!(packets.next(), Some(Packet::CmdSwitch));
            }
            p => panic!("expected switch got {p:?}"),
        }
        assert_eq!(packets.next(), Some(Packet::Nop));
        let mask = match packets.next() {
            Some(Packet::Mask(val)) => val,
            p => panic!("expected mask got {p:?}"),
        };
        match packets.next() {
            Some(Packet::Ctl0(val)) => ctl0 = (ctl0 & !mask) | (val & mask),
            p => panic!("expected ctl0 got {p:?}"),
        }
        if kind == DeviceKind::Virtex5 && (ctl0 & 0x40) != 0 {
            diebs.mode = BitstreamMode::Encrypt;
        }
        let _mask = match packets.next() {
            Some(Packet::Mask(val)) => val,
            p => panic!("expected mask got {p:?}"),
        };
        match packets.next() {
            Some(Packet::Ctl1(val)) => {
                diebs.regs.insert(Reg::Ctl1, val);
            }
            p => panic!("expected ctl1 got {p:?}"),
        }
        for _ in 0..8 {
            assert_eq!(packets.next(), Some(Packet::Nop));
        }
    }

    // main loop
    let frame_bytes = diebs.frame_len / 8;
    if diebs.mode == BitstreamMode::Encrypt {
        let data;
        if kind == DeviceKind::Virtex4 {
            assert_eq!(packets.next(), Some(Packet::Far(0)));
            assert_eq!(packets.next(), Some(Packet::CmdWcfg));
            assert_eq!(packets.next(), Some(Packet::Nop));
            let init_iv = match packets.next() {
                Some(Packet::Cbc(val)) => val,
                p => panic!("expected fdri got {p:?}"),
            };
            diebs.iv = init_iv.clone();
            let init_data = match packets.next() {
                Some(Packet::EncFdri(val)) => val,
                p => panic!("expected fdri got {p:?}"),
            };
            for _ in 0..9 {
                assert_eq!(packets.next(), Some(Packet::Nop));
            }
            assert_eq!(packets.next(), Some(Packet::Crc));
            assert_eq!(packets.next(), Some(Packet::CmdNull));
            assert_eq!(packets.next(), Some(Packet::Nop));
            assert_eq!(packets.next(), Some(Packet::Far(0)));
            assert_eq!(packets.next(), Some(Packet::CmdWcfg));
            assert_eq!(packets.next(), Some(Packet::Nop));
            assert_eq!(packets.next(), Some(Packet::Cbc(init_iv)));
            data = match packets.next() {
                Some(Packet::EncFdri(val)) => val,
                p => panic!("expected fdri got {p:?}"),
            };
            assert_eq!(init_data.len(), 12 * 41 * 4);
            assert!(data.starts_with(&init_data));
        } else {
            assert_eq!(packets.next(), Some(Packet::Far(0)));
            assert_eq!(packets.next(), Some(Packet::CmdWcfg));
            assert_eq!(packets.next(), Some(Packet::Nop));
            match packets.next() {
                Some(Packet::Cbc(val)) => diebs.iv = val,
                p => panic!("expected fdri got {p:?}"),
            };
            data = match packets.next() {
                Some(Packet::EncFdri(val)) => val,
                p => panic!("expected fdri got {p:?}"),
            };
        }
        let frames = data.len() / frame_bytes;
        let tail = data.len() % frame_bytes;
        assert!(tail < 16);
        let tail = &data[(data.len() - tail)..];
        assert!(tail.iter().all(|&x| x == 0));
        let mut skip = 0;
        let mut fi = 0;
        for i in 0..(frames - 1) {
            if skip != 0 {
                skip -= 1;
                continue;
            }
            let pos = i * frame_bytes;
            insert_virtex4_frame(diebs, fi, &data[pos..pos + frame_bytes]);
            let cur_reg = diebs.frame_info[fi].addr.region;
            let cur_typ = diebs.frame_info[fi].addr.typ;
            fi += 1;
            if fi >= diebs.frame_info.len()
                || diebs.frame_info[fi].addr.region != cur_reg
                || diebs.frame_info[fi].addr.typ != cur_typ
            {
                skip = 2;
            }
        }
        let last = data[(frames - 1) * frame_bytes..].to_vec();
        assert!(last.iter().all(|&x| x == 0));
        for _ in 0..9 {
            assert_eq!(packets.next(), Some(Packet::Nop));
        }
    } else {
        let mut fi = 0;
        let mut skip = 0;
        let mut state = State::None;
        let mut last_frame = None;
        let mut first_mf = false;
        loop {
            match packets.peek() {
                Some(Packet::Far(far)) => {
                    packets.next();
                    fi = far_dict[&far];
                    skip = 0;
                }
                Some(Packet::CmdWcfg) => {
                    packets.next();
                    assert_eq!(packets.next(), Some(Packet::Nop));
                    if kind != DeviceKind::Virtex7 {
                        assert_ne!(state, State::Wcfg);
                    }
                    state = State::Wcfg;
                    match packets.next() {
                        Some(Packet::Far(far)) => {
                            fi = far_dict[&far];
                            skip = 0;
                        }
                        p => panic!("expected far got {p:?}"),
                    }
                    assert_eq!(packets.next(), Some(Packet::Nop));
                }
                Some(Packet::CmdMfwr) => (),
                _ => break,
            }
            match packets.peek() {
                Some(Packet::CmdWcfg) => {
                    assert_ne!(state, State::Wcfg);
                    state = State::Wcfg;
                    packets.next();
                    assert_eq!(packets.next(), Some(Packet::Nop));
                }
                Some(Packet::CmdMfwr) => {
                    assert_ne!(state, State::Mfwr);
                    diebs.mode = BitstreamMode::Compress;
                    state = State::Mfwr;
                    packets.next();
                    let num_nops = match kind {
                        DeviceKind::Virtex4 | DeviceKind::Virtex5 => 1,
                        DeviceKind::Virtex6 | DeviceKind::Virtex7 => 12,
                        _ => unreachable!(),
                    };
                    for _ in 0..num_nops {
                        assert_eq!(packets.next(), Some(Packet::Nop));
                    }
                    first_mf = true;
                }
                Some(Packet::Nop) => {
                    packets.next();
                }
                _ => (),
            }
            match (packets.peek(), state) {
                (Some(Packet::CmdWcfg), State::Wcfg) => {
                    packets.next();
                    assert_eq!(packets.next(), Some(Packet::Nop));
                }
                (Some(Packet::Fdri(val)), State::Wcfg) => {
                    packets.next();
                    let frames = val.len() / frame_bytes;
                    assert_eq!(val.len() % frame_bytes, 0);
                    for i in 0..(frames - 1) {
                        if skip != 0 {
                            skip -= 1;
                            continue;
                        }
                        let pos = i * frame_bytes;
                        insert_virtex4_frame(diebs, fi, &val[pos..pos + frame_bytes]);
                        let cur_reg = diebs.frame_info[fi].addr.region;
                        let cur_typ = diebs.frame_info[fi].addr.typ;
                        fi += 1;
                        if fi >= diebs.frame_info.len()
                            || diebs.frame_info[fi].addr.region != cur_reg
                            || diebs.frame_info[fi].addr.typ != cur_typ
                        {
                            skip = 2;
                        }
                    }
                    let last = val[(frames - 1) * frame_bytes..].to_vec();
                    last_frame = Some(last);
                }
                (Some(Packet::Mfwr(mf)), State::Mfwr) => {
                    assert_eq!(
                        mf,
                        match kind {
                            DeviceKind::Virtex4 => 2,
                            DeviceKind::Virtex5 | DeviceKind::Virtex6 =>
                                if diebs.frame_info[fi].addr.typ == 1 {
                                    6
                                } else {
                                    2
                                },
                            DeviceKind::Virtex7 =>
                                if first_mf {
                                    8
                                } else {
                                    4
                                },
                            _ => unreachable!(),
                        },
                    );
                    first_mf = false;
                    packets.next();
                    insert_virtex4_frame(diebs, fi, last_frame.as_ref().unwrap());
                    if kind == DeviceKind::Virtex7 && diebs.frame_info[fi].addr.typ == 1 {
                        for _ in 0..8 {
                            assert_eq!(packets.next(), Some(Packet::Nop));
                        }
                    }
                }
                (p, _) => {
                    panic!("UNEXPECTED PACKET IN STATE {state:?}: {p:#x?}");
                }
            }
        }
    }
    match packets.next() {
        Some(Packet::Crc) => (),
        Some(Packet::CmdRcrc) => {
            diebs.regs.insert(Reg::FakeIgnoreCrc, 1);
        }
        p => panic!("expected CRC or RCRC got {p:?}"),
    }
    if matches!(kind, DeviceKind::Virtex6 | DeviceKind::Virtex7) {
        assert_eq!(packets.next(), Some(Packet::Nop));
        assert_eq!(packets.next(), Some(Packet::Nop));
    }
    assert_eq!(packets.next(), Some(Packet::CmdGRestore));
    assert_eq!(packets.next(), Some(Packet::Nop));
    assert_eq!(packets.next(), Some(Packet::CmdDGHigh));
    if kind != DeviceKind::Virtex4 && diebs.mode == BitstreamMode::Compress {
        let _mask = match packets.next() {
            Some(Packet::Mask(val)) => val,
            p => panic!("expected mask got {p:?}"),
        };
        match packets.next() {
            Some(Packet::Ctl1(val)) => {
                diebs.regs.insert(Reg::Ctl1, val);
            }
            p => panic!("expected ctl1 got {p:?}"),
        }
    }
    for _ in 0..100 {
        assert_eq!(packets.next(), Some(Packet::Nop));
    }
    if matches!(kind, DeviceKind::Virtex4 | DeviceKind::Virtex5) {
        assert_eq!(packets.next(), Some(Packet::CmdGRestore));
    }
    match kind {
        DeviceKind::Virtex4 => {
            assert_eq!(packets.next(), Some(Packet::Nop));
            assert_eq!(packets.next(), Some(Packet::CmdNull));
            assert_eq!(packets.next(), Some(Packet::Nop));
            let _final_far = match packets.next() {
                Some(Packet::Far(val)) => val,
                p => panic!("expected far got {p:?}"),
            };
            assert_eq!(packets.next(), Some(Packet::CmdStart));
            assert_eq!(packets.next(), Some(Packet::Nop));
            let mask = match packets.next() {
                Some(Packet::Mask(val)) => val,
                p => panic!("expected mask got {p:?}"),
            };
            match packets.next() {
                Some(Packet::Ctl0(val)) => ctl0 = (ctl0 & !mask) | (val & mask),
                p => panic!("expected ctl0 got {p:?}"),
            }
            assert_eq!(packets.next(), Some(Packet::Crc));
            assert_eq!(packets.next(), Some(Packet::CmdDesynch));
            for _ in 0..16 {
                assert_eq!(packets.next(), Some(Packet::Nop));
            }
        }
        DeviceKind::Virtex5 | DeviceKind::Virtex6 | DeviceKind::Virtex7 => {
            if kind == DeviceKind::Virtex5 {
                for _ in 0..30 {
                    assert_eq!(packets.next(), Some(Packet::Nop));
                }
            }
            assert_eq!(packets.next(), Some(Packet::CmdStart));
            assert_eq!(packets.next(), Some(Packet::Nop));
            let _final_far = match packets.next() {
                Some(Packet::Far(val)) => val,
                p => panic!("expected far got {p:?}"),
            };
            let mask = match packets.next() {
                Some(Packet::Mask(val)) => val,
                p => panic!("expected mask got {p:?}"),
            };
            match packets.next() {
                Some(Packet::Ctl0(val)) => ctl0 = (ctl0 & !mask) | (val & mask),
                p => panic!("expected ctl0 got {p:?}"),
            }
            if diebs.regs.contains_key(&Reg::FakeIgnoreCrc) {
                assert_eq!(packets.next(), Some(Packet::CmdRcrc));
            } else {
                assert_eq!(packets.next(), Some(Packet::Crc));
            }
            if matches!(kind, DeviceKind::Virtex6 | DeviceKind::Virtex7) {
                assert_eq!(packets.next(), Some(Packet::Nop));
                assert_eq!(packets.next(), Some(Packet::Nop));
            }
            if !diebs.regs.contains_key(&Reg::FakeEncrypted) {
                assert_eq!(packets.next(), Some(Packet::CmdDesynch));
            }
            let mut num_nops = match kind {
                DeviceKind::Virtex5 => 61,
                DeviceKind::Virtex6 | DeviceKind::Virtex7 => 400,
                _ => unreachable!(),
            };
            if diebs.regs.contains_key(&Reg::FakeEncrypted) {
                num_nops += 2; // desync
                num_nops -= 27; // mask+ctl+cbc+dwc
                num_nops -= 0x10; // encrypted header
                num_nops -= 0x78; // encrypted trailer
            }
            if diebs.regs.contains_key(&Reg::Unk1C) {
                num_nops -= 4;
            }
            if trim_regs == 0 && diebs.regs.contains_key(&Reg::Trim0) {
                num_nops -= 4;
            }
            if diebs.regs.contains_key(&Reg::Testmode) {
                num_nops -= 2;
            }
            if diebs.regs.contains_key(&Reg::Bspi) {
                num_nops -= 5;
            }
            if diebs.regs.contains_key(&Reg::FakeFallEdge) {
                num_nops -= 2;
            }
            num_nops -= 6 * trim_regs;
            for _ in 0..num_nops {
                assert_eq!(packets.next(), Some(Packet::Nop));
            }
        }
        _ => unreachable!(),
    }
    diebs.regs.insert(Reg::Ctl0, ctl0);
    if die_index != geom.die_order.len() - 1 {
        packets.desync();
        assert_eq!(packets.next(), Some(Packet::SyncWord));
        assert_eq!(packets.next(), Some(Packet::Nop));
        assert_eq!(packets.next(), Some(Packet::CmdShutdown));
        assert_eq!(packets.next(), Some(Packet::CmdRcrc));
        assert_eq!(packets.next(), Some(Packet::Nop));
        assert_eq!(packets.next(), Some(Packet::Nop));
        let subdata = match packets.next() {
            Some(Packet::Bout(data)) => data,
            p => panic!("expected bout got {p:?}"),
        };
        parse_virtex4_bitstream(bs, &subdata, key, geom, die_index + 1);
        assert_eq!(packets.next(), Some(Packet::Nop));
        assert_eq!(packets.next(), Some(Packet::Nop));
        assert_eq!(packets.next(), Some(Packet::CmdStart));
        assert_eq!(packets.next(), Some(Packet::CmdDesynch));
    }
    loop {
        match packets.next() {
            Some(Packet::Nop) => (),
            Some(Packet::DummyWord) => break,
            None => return,
            p => panic!("expected end got {p:?}"),
        }
    }
    assert_eq!(die_index, 0);
    assert!(geom.has_gtz_bot || geom.has_gtz_top);
    let loader = core::mem::replace(bs, empty(geom));
    bs.gtz_loader = Some(Box::new(loader));
    for _ in 0..7 {
        assert_eq!(packets.next(), Some(Packet::DummyWord));
    }
    assert_eq!(packets.next(), Some(Packet::WidthDetect));
    for _ in 0..2 {
        assert_eq!(packets.next(), Some(Packet::DummyWord));
    }
    assert_eq!(packets.next(), Some(Packet::SyncWord));
    assert_eq!(packets.next(), Some(Packet::Nop));
    loop {
        let pkt = packets.next().unwrap();
        if let Packet::Axss(data) = pkt {
            let gtz = parse_gtz_bitstream(&data);
            if geom.has_gtz_bot && !bs.gtz.contains_key(&Dir::S) {
                bs.gtz.insert(Dir::S, gtz);
            } else if geom.has_gtz_top && !bs.gtz.contains_key(&Dir::N) {
                bs.gtz.insert(Dir::N, gtz);
            } else {
                panic!("ummm too many GTZ?");
            }
        } else {
            assert_eq!(pkt, Packet::Crc);
            break;
        }
    }
    assert_eq!(packets.next(), Some(Packet::Nop));
    assert_eq!(packets.next(), Some(Packet::Nop));
    assert_eq!(packets.next(), Some(Packet::CmdDesynch));
    for _ in 0..8 {
        assert_eq!(packets.next(), Some(Packet::Nop));
    }
    consume_virtex4_shutdown(&mut packets, geom, die_index);
    consume_virtex4_aghigh(&mut packets, geom, die_index);
    for _ in 0..100 {
        assert_eq!(packets.next(), Some(Packet::Nop));
    }
    parse_virtex4_bitstream(bs, &data[packets.pos()..], key, geom, die_index)
}

fn parse_gtz_bitstream(data: &[u32]) -> GtzBitstream {
    let idcode = data[0];
    assert_eq!(idcode >> 28, 0);
    assert_eq!(data[1], 0x00010001);
    let data_len: usize = data[2].try_into().unwrap();
    let data_seg = &data[3..(data_len + 3)];
    let data_seg = data_seg.strip_suffix(&[0; 0x20]).unwrap();
    let data_crc = data_seg[data_seg.len() - 1];
    let data_seg = &data_seg[..data_seg.len() - 1];
    let mut crc = Crc::new(DeviceKind::Virtex7);
    for &w in &data[..3 + data_len - 0x21] {
        crc.update(0, w);
    }
    assert_eq!(data_crc, crc.get());

    assert_eq!(data[3 + data_len], idcode | 1 << 28);
    assert_eq!(data[3 + data_len + 1], 0x00010001);
    let code_len: usize = data[3 + data_len + 2].try_into().unwrap();
    let code_seg = &data[3 + data_len + 3..3 + data_len + 3 + code_len];
    assert_eq!(3 + data_len + 3 + code_len, data.len());
    let code_seg = code_seg.strip_suffix(&[0; 0x20]).unwrap();
    let code_crc = code_seg[code_seg.len() - 1];
    let code_seg = &code_seg[..code_seg.len() - 1];
    let mut crc = Crc::new(DeviceKind::Virtex7);
    for &w in &data[3 + data_len..3 + data_len + 3 + code_len - 0x21] {
        crc.update(0, w);
    }
    assert_eq!(code_crc, crc.get());
    GtzBitstream {
        idcode,
        data: data_seg.to_vec(),
        code: code_seg.to_vec(),
    }
}

fn consume_virtex4_shutdown(packets: &mut PacketParser, geom: &BitstreamGeom, die_index: usize) {
    for _ in 0..8 {
        assert_eq!(packets.next(), Some(Packet::DummyWord));
    }
    assert_eq!(packets.next(), Some(Packet::WidthDetect));
    for _ in 0..2 {
        assert_eq!(packets.next(), Some(Packet::DummyWord));
    }
    assert_eq!(packets.next(), Some(Packet::SyncWord));
    assert_eq!(packets.next(), Some(Packet::Nop));
    assert_eq!(packets.next(), Some(Packet::CmdShutdown));
    assert_eq!(packets.next(), Some(Packet::Nop));
    assert_eq!(packets.next(), Some(Packet::CmdRcrc));
    assert_eq!(packets.next(), Some(Packet::Nop));
    assert_eq!(packets.next(), Some(Packet::Nop));
    assert_eq!(packets.next(), Some(Packet::CmdDesynch));
    for _ in 0..8 {
        assert_eq!(packets.next(), Some(Packet::Nop));
    }
    packets.desync();
    if die_index != geom.die_order.len() - 1 {
        assert_eq!(packets.next(), Some(Packet::SyncWord));
        assert_eq!(packets.next(), Some(Packet::Nop));
        let subdata = match packets.next() {
            Some(Packet::Bout(data)) => data,
            p => panic!("expected bout got {p:?}"),
        };
        let mut subpackets = PacketParser::new(geom.kind, &subdata, &KeyData::None);
        consume_virtex4_shutdown(&mut subpackets, geom, die_index + 1);
        assert_eq!(subpackets.next(), None);
        assert_eq!(packets.next(), Some(Packet::Nop));
        assert_eq!(packets.next(), Some(Packet::Nop));
        assert_eq!(packets.next(), Some(Packet::CmdDesynch));
        for _ in 0..8 {
            assert_eq!(packets.next(), Some(Packet::Nop));
        }
    }
}

fn consume_virtex4_aghigh(packets: &mut PacketParser, geom: &BitstreamGeom, die_index: usize) {
    for _ in 0..8 {
        assert_eq!(packets.next(), Some(Packet::DummyWord));
    }
    assert_eq!(packets.next(), Some(Packet::WidthDetect));
    for _ in 0..2 {
        assert_eq!(packets.next(), Some(Packet::DummyWord));
    }
    assert_eq!(packets.next(), Some(Packet::SyncWord));
    assert_eq!(packets.next(), Some(Packet::Nop));
    assert_eq!(packets.next(), Some(Packet::CmdAGHigh));
    assert_eq!(packets.next(), Some(Packet::Nop));
    assert_eq!(packets.next(), Some(Packet::Nop));
    assert_eq!(packets.next(), Some(Packet::CmdDesynch));
    for _ in 0..8 {
        assert_eq!(packets.next(), Some(Packet::Nop));
    }
    packets.desync();
    if die_index != geom.die_order.len() - 1 {
        assert_eq!(packets.next(), Some(Packet::SyncWord));
        assert_eq!(packets.next(), Some(Packet::Nop));
        let subdata = match packets.next() {
            Some(Packet::Bout(data)) => data,
            p => panic!("expected bout got {p:?}"),
        };
        let mut subpackets = PacketParser::new(geom.kind, &subdata, &KeyData::None);
        consume_virtex4_aghigh(&mut subpackets, geom, die_index + 1);
        assert_eq!(subpackets.next(), None);
        assert_eq!(packets.next(), Some(Packet::Nop));
        assert_eq!(packets.next(), Some(Packet::Nop));
        assert_eq!(packets.next(), Some(Packet::CmdDesynch));
        for _ in 0..8 {
            assert_eq!(packets.next(), Some(Packet::Nop));
        }
    }
}

fn check_virtex4_ecc(bs: &Bitstream) {
    for (die, dbs) in &bs.die {
        for (fi, present) in dbs.frame_present.iter().enumerate() {
            if !*present {
                continue;
            }
            let fdata = dbs.frame(fi);
            let finfo = &dbs.frame_info[fi];
            let mut ecc: u32 = 0;
            let mut recc: u32 = 0;
            let flip = finfo.addr.region < 0;
            for (idx, bit) in fdata.iter().enumerate() {
                if !*bit {
                    continue;
                }
                let mask = match idx {
                    0..0x280 if !flip => finfo.mask_mode[idx / 0x140],
                    0..0x280 if flip => finfo.mask_mode[3 - idx / 0x140],
                    0x280..0x28c => {
                        recc ^= 1 << (idx - 0x280);
                        continue;
                    }
                    0x28c..0x2a0 => FrameMaskMode::None,
                    0x2a0..0x520 if !flip => finfo.mask_mode[(idx - 0x20) / 0x140],
                    0x2a0..0x520 if flip => finfo.mask_mode[3 - (idx - 0x20) / 0x140],
                    _ => unreachable!(),
                };
                let idx = idx as u32;
                match mask {
                    FrameMaskMode::None => (),
                    FrameMaskMode::BramV4 => {
                        let eidx = if flip { 0x520 - 1 - idx } else { idx };
                        let eidx = if eidx < 0x280 { eidx } else { eidx - 0x20 };
                        let eidx = eidx % 0x140;
                        if matches!(
                            eidx,
                            8 | 12
                                | 14
                                | 19
                                | 21
                                | 26
                                | 27
                                | 32
                                | 35
                                | 39
                                | 41
                                | 46
                                | 48
                                | 52
                                | 55
                                | 59
                                | 61
                                | 66
                                | 68
                                | 72
                                | 74
                                | 79
                                | 81
                                | 86
                                | 88
                                | 92
                                | 95
                                | 99
                                | 101
                                | 106
                                | 108
                                | 112
                                | 114
                                | 119
                                | 121
                                | 126
                                | 200
                                | 204
                                | 207
                                | 211
                                | 213
                                | 218
                                | 220
                                | 224
                                | 227
                                | 231
                                | 233
                                | 237
                                | 240
                                | 244
                                | 247
                                | 251
                                | 253
                                | 258
                                | 260
                                | 264
                                | 266
                                | 271
                                | 273
                                | 277
                                | 280
                                | 284
                                | 287
                                | 291
                                | 293
                                | 298
                                | 300
                                | 304
                                | 306
                                | 311
                                | 313
                                | 318
                        ) {
                            continue;
                        }
                    }
                    FrameMaskMode::DrpV4 => {
                        let eidx = if flip { 0x520 - 1 - idx } else { idx };
                        let eidx = if eidx < 0x280 { eidx } else { eidx - 0x20 };
                        if matches!(eidx % 20, 1..17) {
                            let midx = eidx / 20 * 20 + 18;
                            let midx = if midx < 0x280 { midx } else { midx + 0x20 };
                            let midx = if flip { 0x520 - 1 - midx } else { midx };
                            if fdata[midx as usize] {
                                continue;
                            }
                        }
                    }
                    FrameMaskMode::All => continue,
                    _ => todo!(),
                }
                let code = if idx < 0x140 {
                    0x2c0 + idx
                } else {
                    0x420 + (idx - 0x140)
                };
                ecc ^= 0x800 | code;
            }
            for i in 0..11 {
                if (ecc & (1 << i)) != 0 {
                    ecc ^= 0x800;
                }
            }
            if ecc != recc {
                eprintln!(
                    "ECC MISMATCH at frame {die}.{ft}.{fr}.{fmaj}.{fmin}: computed {ecc:04x} found {recc:04x}",
                    ft = finfo.addr.typ,
                    fr = finfo.addr.region,
                    fmaj = finfo.addr.major,
                    fmin = finfo.addr.minor
                );
            }
        }
    }
}

fn check_virtex5_ecc(bs: &Bitstream) {
    for (die, dbs) in &bs.die {
        for (fi, present) in dbs.frame_present.iter().enumerate() {
            if !*present {
                continue;
            }
            let fdata = dbs.frame(fi);
            let finfo = &dbs.frame_info[fi];
            let mut ecc: u32 = 0;
            let mut recc: u32 = 0;
            for (idx, bit) in fdata.iter().enumerate() {
                if !*bit {
                    continue;
                }
                let mask = match idx {
                    0..0x280 => finfo.mask_mode[0],
                    0x280..0x28c => {
                        recc ^= 1 << (idx - 0x280);
                        continue;
                    }
                    0x28c..0x2a0 => FrameMaskMode::None,
                    0x2a0..0x520 => finfo.mask_mode[1],
                    _ => unreachable!(),
                };
                let idx = idx as u32;
                match mask {
                    FrameMaskMode::None => (),
                    FrameMaskMode::DrpHclk(cframe, cbit) => {
                        let cfi = fi - (finfo.addr.minor as usize) + cframe;
                        if dbs.frame(cfi)[0x280 + cbit] {
                            continue;
                        }
                    }
                    FrameMaskMode::All => continue,
                    _ => todo!(),
                }
                let code = if idx < 0x140 {
                    0x2c0 + idx
                } else {
                    0x420 + (idx - 0x140)
                };
                ecc ^= 0x800 | code;
            }
            for i in 0..11 {
                if (ecc & (1 << i)) != 0 {
                    ecc ^= 0x800;
                }
            }
            if ecc != recc {
                eprintln!(
                    "ECC MISMATCH at frame {die}.{ft}.{fr}.{fmaj}.{fmin}: computed {ecc:04x} found {recc:04x}",
                    ft = finfo.addr.typ,
                    fr = finfo.addr.region,
                    fmaj = finfo.addr.major,
                    fmin = finfo.addr.minor
                );
            }
        }
    }
}

fn check_virtex6_ecc(bs: &Bitstream) {
    for (die, dbs) in &bs.die {
        for (fi, present) in dbs.frame_present.iter().enumerate() {
            if !*present {
                continue;
            }
            let fdata = dbs.frame(fi);
            let finfo = &dbs.frame_info[fi];
            let mut ecc: u32 = 0;
            let mut recc: u32 = 0;
            for (idx, bit) in fdata.iter().enumerate() {
                if !*bit {
                    continue;
                }
                let mask = match idx {
                    0..0x500 => finfo.mask_mode[0],
                    0x500..0x50d => {
                        recc ^= 1 << (idx - 0x500);
                        continue;
                    }
                    0x50d..0x520 => FrameMaskMode::None,
                    0x520..0xa20 => finfo.mask_mode[1],
                    _ => unreachable!(),
                };
                let idx = idx as u32;
                match mask {
                    FrameMaskMode::None => (),
                    FrameMaskMode::DrpHclk(cframe, cbit) => {
                        let cfi = fi - (finfo.addr.minor as usize) + cframe;
                        if dbs.frame(cfi)[0x500 + cbit] {
                            continue;
                        }
                    }
                    FrameMaskMode::CmtDrpHclk(cframe, cbit) => {
                        let cfi = fi - (finfo.addr.minor as usize) + cframe;
                        if dbs.frame(cfi)[0x500 + cbit]
                            && !matches!(idx, 0..0x80 | 0x480..0x5a0 | 0x9a0..0xa20)
                        {
                            continue;
                        }
                    }
                    FrameMaskMode::All => continue,
                    _ => todo!(),
                }
                let code = if idx < 0x240 {
                    0x5c0 + idx
                } else {
                    0x820 + (idx - 0x240)
                };
                ecc ^= 0x1000 | code;
            }
            for i in 0..12 {
                if (ecc & (1 << i)) != 0 {
                    ecc ^= 0x1000;
                }
            }
            if ecc != recc {
                eprintln!(
                    "ECC MISMATCH at frame {die}.{ft}.{fr}.{fmaj}.{fmin}: computed {ecc:04x} found {recc:04x}",
                    ft = finfo.addr.typ,
                    fr = finfo.addr.region,
                    fmaj = finfo.addr.major,
                    fmin = finfo.addr.minor
                );
            }
        }
    }
}

fn check_virtex7_ecc(bs: &Bitstream) {
    for (die, dbs) in &bs.die {
        for (fi, present) in dbs.frame_present.iter().enumerate() {
            if !*present {
                continue;
            }
            let fdata = dbs.frame(fi);
            let finfo = &dbs.frame_info[fi];
            let mut ecc: u32 = 0;
            let mut recc: u32 = 0;
            for (idx, bit) in fdata.iter().enumerate() {
                if !*bit {
                    continue;
                }
                let mask = match idx {
                    0..0x640 => finfo.mask_mode[0],
                    0x640..0x64d => {
                        recc ^= 1 << (idx - 0x640);
                        continue;
                    }
                    0x64d..0x660 => FrameMaskMode::None,
                    0x660..0xca0 => finfo.mask_mode[1],
                    _ => unreachable!(),
                };
                let idx = idx as u32;
                match mask {
                    FrameMaskMode::None => (),
                    FrameMaskMode::DrpHclk(cframe, cbit) => {
                        let cfi = fi - (finfo.addr.minor as usize) + cframe;
                        if dbs.frame(cfi)[0x640 + cbit] {
                            continue;
                        }
                    }
                    FrameMaskMode::PcieLeftDrpHclk(cframe, cbit) => {
                        let cfi = fi - (finfo.addr.minor as usize) - 28 - 2 * 36 + cframe;
                        assert_eq!(dbs.frame_info[cfi].addr.minor, cframe as u32);
                        if dbs.frame(cfi)[0x640 + cbit] {
                            continue;
                        }
                    }
                    FrameMaskMode::CmtDrpHclk(cframe, cbit) => {
                        let cfi = fi - (finfo.addr.minor as usize) + cframe;
                        if dbs.frame(cfi)[0x640 + cbit] && matches!(idx, 0..0x600 | 0x6a0..0xca0) {
                            continue;
                        }
                    }
                    FrameMaskMode::All => continue,
                    _ => todo!(),
                }
                let code = if idx < 0xe0 {
                    0x320 + idx
                } else if idx < 0x4c0 {
                    0x420 + (idx - 0xe0)
                } else {
                    0x820 + (idx - 0x4c0)
                };
                ecc ^= 0x1000 | code;
            }
            for i in 0..12 {
                if (ecc & (1 << i)) != 0 {
                    ecc ^= 0x1000;
                }
            }
            if ecc != recc {
                eprintln!(
                    "ECC MISMATCH at frame {die}.{ft}.{fr}.{fmaj}.{fmin}: computed {ecc:04x} found {recc:04x}",
                    ft = finfo.addr.typ,
                    fr = finfo.addr.region,
                    fmaj = finfo.addr.major,
                    fmin = finfo.addr.minor
                );
            }
        }
    }
}

fn parse_ultrascale_bitstream(bs: &Bitstream, data: &[u8], key: &KeyData) {
    let packets = PacketParser::new(bs.kind, data, key);
    for packet in packets {
        if let Packet::Fdri(data) = packet {
            println!("PACKET FDRI {l}", l = data.len());
        } else {
            println!("PACKET {packet:x?}");
        }
    }
    todo!()
}

fn empty(geom: &BitstreamGeom) -> Bitstream {
    Bitstream {
        kind: geom.kind,
        die: geom.die.map_values(|dg| DieBitstream {
            regs: Default::default(),
            mode: BitstreamMode::Plain,
            iv: vec![],
            frame_len: dg.frame_len,
            frame_data: BitVec::repeat(false, dg.frame_len * dg.frame_info.len()),
            frame_info: dg.frame_info.clone(),
            frame_present: BitVec::repeat(false, dg.frame_info.len()),
            bram_data: BitVec::repeat(false, dg.bram_frame_len * dg.bram_frame_info.len()),
            bram_frame_present: BitVec::repeat(false, dg.bram_frame_info.len()),
            bram_frame_len: dg.bram_frame_len,
            bram_frame_info: dg.bram_frame_info.clone(),
            iob: BitVec::repeat(false, dg.iob_frame_len),
            iob_present: false,
            frame_fixups: HashMap::new(),
        }),
        gtz: Default::default(),
        gtz_loader: None,
    }
}

pub fn parse(geom: &BitstreamGeom, data: &[u8], key: &KeyData) -> Bitstream {
    let mut res = empty(geom);
    match res.kind {
        DeviceKind::Xc2000 => parse_xc2000_bitstream(&mut res, data),
        DeviceKind::Xc4000 | DeviceKind::S40Xl => parse_xc4000_bitstream(&mut res, data),
        DeviceKind::Xc5200 => parse_xc5200_bitstream(&mut res, data),
        DeviceKind::Virtex | DeviceKind::Virtex2 => parse_virtex_bitstream(&mut res, data, key),
        DeviceKind::Spartan3A => parse_spartan3a_bitstream(&mut res, data, key),
        DeviceKind::Spartan6 => parse_spartan6_bitstream(&mut res, data, key),
        DeviceKind::Virtex4 => {
            parse_virtex4_bitstream(&mut res, data, key, geom, 0);
            check_virtex4_ecc(&res);
        }
        DeviceKind::Virtex5 => {
            parse_virtex4_bitstream(&mut res, data, key, geom, 0);
            check_virtex5_ecc(&res);
        }
        DeviceKind::Virtex6 => {
            parse_virtex4_bitstream(&mut res, data, key, geom, 0);
            check_virtex6_ecc(&res);
        }
        DeviceKind::Virtex7 => {
            parse_virtex4_bitstream(&mut res, data, key, geom, 0);
            check_virtex7_ecc(&res);
        }
        DeviceKind::Ultrascale | DeviceKind::UltrascalePlus => {
            parse_ultrascale_bitstream(&res, data, key)
        }
        DeviceKind::Versal => panic!("versal bitstreams not supported through generic code"),
    }
    res
}
