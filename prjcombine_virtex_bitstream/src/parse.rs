use crate::packet::{Packet, PacketParser};
use crate::{
    Bitstream, BitstreamGeom, BitstreamMode, DeviceKind, DieBitstream, FrameAddr, KeyData, Reg,
};
use arrayref::array_ref;
use bitvec::prelude::*;
use std::collections::HashMap;

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
        bs.regs[Reg::FakeLcAlignmentDone] = Some(1);
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
    // crc.crc = 0x468f;
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
    let frame_words = (frame_len + 31) / 32;
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
    let frame_words = (frame_len + 31) / 32;
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
    let frame_words = (frame_len + 31) / 32;
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
        (bs.frame_len + 31) / 32 + 1
    } else {
        bs.frame_len / 32
    };
    let frame_bytes = frame_words * 4;
    let flr = (frame_words - 1) as u32;
    let mut early_dghigh = false;
    if packets.peek() == Some(Packet::CmdDGHigh) {
        packets.next();
        early_dghigh = true;
        bs.regs[Reg::FakeEarlyGhigh] = Some(1);
        let mut nops = 0;
        while let Some(Packet::Nop) = packets.peek() {
            packets.next();
            nops += 1;
        }
        assert_eq!(nops, flr + 1);
    }
    assert_eq!(packets.next(), Some(Packet::Flr(flr)));
    match packets.next() {
        Some(Packet::Cor0(val)) => bs.regs[Reg::Cor0] = Some(val),
        p => panic!("expected cor0 got {p:?}"),
    }
    if kind != DeviceKind::Virtex {
        match packets.next() {
            Some(Packet::Idcode(val)) => bs.regs[Reg::Idcode] = Some(val),
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
            bs.regs[Reg::FakeHasSwitch] = Some(1);
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
                    bs.regs[Reg::Key] = Some(key);
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
                bs.regs[Reg::FakeFreezeDciNops] = Some(nops);
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
            bs.regs[Reg::FakeDoubleGrestore] = Some(1);
            packets.next();
        }
    }

    assert_eq!(packets.next(), Some(Packet::CmdStart));
    match packets.next() {
        Some(Packet::Ctl0(val)) => bs.regs[Reg::Ctl0] = Some(val),
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
        Some(Packet::Cor2(val)) => bs.regs[Reg::Cor2] = Some(val),
        p => panic!("expected cor2 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::CclkFrequency(val)) => bs.regs[Reg::CclkFrequency] = Some(val),
        p => panic!("expected cclk frequency got {p:?}"),
    }
    let frame_bytes = bs.frame_len / 8;
    let flr = (bs.frame_len / 16 - 1) as u32;
    assert_eq!(packets.next(), Some(Packet::Flr(flr)));
    match packets.next() {
        Some(Packet::Cor1(val)) => bs.regs[Reg::Cor1] = Some(val),
        p => panic!("expected cor1 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::Idcode(val)) => bs.regs[Reg::Idcode] = Some(val),
        p => panic!("expected idcode got {p:?}"),
    }
    let _mask = match packets.next() {
        Some(Packet::Mask(val)) => val,
        p => panic!("expected mask got {p:?}"),
    };
    match packets.next() {
        Some(Packet::Ctl0(val)) => bs.regs[Reg::Ctl0] = Some(val),
        p => panic!("expected ctl0 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::Powerdown(val)) => bs.regs[Reg::Powerdown] = Some(val),
        p => panic!("expected powerdown got {p:?}"),
    }
    match packets.next() {
        Some(Packet::HcOpt(val)) => bs.regs[Reg::HcOpt] = Some(val),
        p => panic!("expected hc opt got {p:?}"),
    }
    match packets.next() {
        Some(Packet::PuGwe(val)) => bs.regs[Reg::PuGwe] = Some(val),
        p => panic!("expected pu gwe got {p:?}"),
    }
    match packets.next() {
        Some(Packet::PuGts(val)) => bs.regs[Reg::PuGts] = Some(val),
        p => panic!("expected pu gts got {p:?}"),
    }
    match packets.next() {
        Some(Packet::Mode(val)) => {
            bs.regs[Reg::Mode] = Some(val);
            match packets.next() {
                Some(Packet::General1(val)) => bs.regs[Reg::General1] = Some(val),
                p => panic!("expected general1 got {p:?}"),
            }
            match packets.next() {
                Some(Packet::General2(val)) => bs.regs[Reg::General2] = Some(val),
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
        Some(Packet::SeuOpt(val)) => bs.regs[Reg::SeuOpt] = Some(val),
        p => panic!("expected seuopt got {p:?}"),
    }
    match packets.next() {
        Some(Packet::RbCrcSw(val)) => bs.regs[Reg::RbCrcSw] = Some(val),
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
        Some(Packet::Cor1(val)) => bs.regs[Reg::Cor1] = Some(val),
        p => panic!("expected cor1 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::Cor2(val)) => bs.regs[Reg::Cor2] = Some(val),
        p => panic!("expected cor2 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::Idcode(val)) => bs.regs[Reg::Idcode] = Some(val),
        p => panic!("expected idcode got {p:?}"),
    }
    let _mask = match packets.next() {
        Some(Packet::Mask(val)) => val,
        p => panic!("expected mask got {p:?}"),
    };
    match packets.next() {
        Some(Packet::Ctl0(val)) => bs.regs[Reg::Ctl0] = Some(val),
        p => panic!("expected ctl0 got {p:?}"),
    }
    for _ in 0..8 {
        assert_eq!(packets.next(), Some(Packet::Nop));
    }
    match packets.next() {
        Some(Packet::Cbc(_)) => {
            assert_eq!(bs.regs[Reg::Ctl0].unwrap() & 0x40, 0x40);
            bs.mode = BitstreamMode::Encrypt;
        }
        Some(Packet::Nop) => {
            assert_eq!(bs.regs[Reg::Ctl0].unwrap() & 0x40, 0);
            for _ in 0..8 {
                assert_eq!(packets.next(), Some(Packet::Nop));
            }
        }
        p => panic!("expected cbc or nop got {p:?}"),
    }
    match packets.next() {
        Some(Packet::CclkFrequency(val)) => bs.regs[Reg::CclkFrequency] = Some(val),
        p => panic!("expected cclk frequency got {p:?}"),
    }
    if let Some(Packet::CclkFrequency(val)) = packets.peek() {
        packets.next();
        bs.regs[Reg::CclkFrequency] = Some(val);
        bs.regs[Reg::FakeDoubleCclkFrequency] = Some(1);
    }
    match packets.next() {
        Some(Packet::Powerdown(val)) => bs.regs[Reg::Powerdown] = Some(val),
        p => panic!("expected powerdown got {p:?}"),
    }
    match packets.next() {
        Some(Packet::EyeMask(val)) => bs.regs[Reg::EyeMask] = Some(val),
        p => panic!("expected eyemask got {p:?}"),
    }
    match packets.next() {
        Some(Packet::HcOpt(val)) => bs.regs[Reg::HcOpt] = Some(val),
        p => panic!("expected hc opt got {p:?}"),
    }
    match packets.next() {
        Some(Packet::Timer(val)) => bs.regs[Reg::Timer] = Some(val),
        p => panic!("expected timer got {p:?}"),
    }
    match packets.next() {
        Some(Packet::PuGwe(val)) => bs.regs[Reg::PuGwe] = Some(val),
        p => panic!("expected pu gwe got {p:?}"),
    }
    match packets.next() {
        Some(Packet::PuGts(val)) => bs.regs[Reg::PuGts] = Some(val),
        p => panic!("expected pu gts got {p:?}"),
    }
    match packets.next() {
        Some(Packet::Mode(val)) => bs.regs[Reg::Mode] = Some(val),
        p => panic!("expected mode got {p:?}"),
    }
    match packets.next() {
        Some(Packet::General1(val)) => bs.regs[Reg::General1] = Some(val),
        p => panic!("expected general1 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::General2(val)) => bs.regs[Reg::General2] = Some(val),
        p => panic!("expected general2 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::General3(val)) => bs.regs[Reg::General3] = Some(val),
        p => panic!("expected general3 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::General4(val)) => bs.regs[Reg::General4] = Some(val),
        p => panic!("expected general4 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::General5(val)) => bs.regs[Reg::General5] = Some(val),
        p => panic!("expected general5 got {p:?}"),
    }
    match packets.next() {
        Some(Packet::SeuOpt(val)) => bs.regs[Reg::SeuOpt] = Some(val),
        p => panic!("expected seuopt got {p:?}"),
    }
    match packets.next() {
        Some(Packet::RbCrcSw(val)) => bs.regs[Reg::RbCrcSw] = Some(val),
        p => panic!("expected rbcrcsw got {p:?}"),
    }
    if let Some(Packet::Testmode(val)) = packets.peek() {
        packets.next();
        bs.regs[Reg::Testmode] = Some(val);
    } else if bs.regs[Reg::FakeDoubleCclkFrequency].is_none() {
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
        Some(Packet::Ctl0(val)) => bs.regs[Reg::Ctl0] = Some(val),
        p => panic!("expected ctl0 got {p:?}"),
    }
    assert_eq!(packets.next(), Some(Packet::Crc));
    assert_eq!(packets.next(), Some(Packet::CmdDesynch));
    for _ in 0..14 {
        assert_eq!(packets.next(), Some(Packet::Nop));
    }
    assert_eq!(packets.next(), None);
}

fn parse_virtex4_bitstream(bs: &mut Bitstream, data: &[u8], key: &KeyData) {
    let mut packets = PacketParser::new(bs.kind, data, key);
    let kind = bs.kind;
    let bs = bs.die.first_mut().unwrap();
    let far_dict: HashMap<_, _> = bs
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

    if kind == DeviceKind::Virtex4 {
        assert_eq!(packets.next(), Some(Packet::DummyWord));
        assert_eq!(packets.next(), Some(Packet::SyncWord));
        assert_eq!(packets.next(), Some(Packet::Nop));
        assert_eq!(packets.next(), Some(Packet::CmdRcrc));
        assert_eq!(packets.next(), Some(Packet::Nop));
        assert_eq!(packets.next(), Some(Packet::Nop));
        match packets.next() {
            Some(Packet::Cor0(val)) => bs.regs[Reg::Cor0] = Some(val),
            p => panic!("expected cor0 got {p:?}"),
        }
        match packets.next() {
            Some(Packet::Idcode(val)) => bs.regs[Reg::Idcode] = Some(val),
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
            bs.mode = BitstreamMode::Encrypt;
        }
        assert_eq!(packets.next(), Some(Packet::CmdSwitch));
        assert_eq!(packets.next(), Some(Packet::Nop));
        // XXX
        let mask = match packets.next() {
            Some(Packet::Mask(val)) => val,
            p => panic!("expected mask got {p:?}"),
        };
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
        if kind == DeviceKind::Virtex7 {
            match packets.next() {
                Some(Packet::Timer(val)) => bs.regs[Reg::Timer] = Some(val),
                p => panic!("expected timer got {p:?}"),
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
            for _ in 0..16 {
                assert_eq!(packets.next(), Some(Packet::Nop));
            }
            match packets.next() {
                Some(Packet::Cbc(val)) => bs.iv = val,
                p => panic!("expected fdri got {p:?}"),
            };

            match packets.next() {
                Some(Packet::Dwc(_)) => (),
                p => panic!("expected dwc got {p:?}"),
            };

            bs.regs[Reg::FakeEncrypted] = Some(1);
        }
        match packets.next() {
            Some(Packet::WBStar(val)) => bs.regs[Reg::WbStar] = Some(val),
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
                Some(Packet::Unk1c(val)) => bs.regs[Reg::Unk1C] = Some(val),
                Some(Packet::Trim(val)) => bs.regs[Reg::Trim] = Some(val),
                p => panic!("expected ctl2 or trim got {p:?}"),
            }
        }
        if kind != DeviceKind::Virtex5 && matches!(packets.peek(), Some(Packet::Testmode(_))) {
            match packets.next() {
                Some(Packet::Testmode(val)) => bs.regs[Reg::Testmode] = Some(val),
                p => panic!("expected testmode got {p:?}"),
            }
        }
        assert_eq!(packets.next(), Some(Packet::CmdRcrc));
        assert_eq!(packets.next(), Some(Packet::Nop));
        assert_eq!(packets.next(), Some(Packet::Nop));
        if kind != DeviceKind::Virtex7 {
            match packets.next() {
                Some(Packet::Timer(val)) => bs.regs[Reg::Timer] = Some(val),
                p => panic!("expected timer got {p:?}"),
            }
        }
        match packets.next() {
            Some(Packet::RbCrcSw(val)) => bs.regs[Reg::RbCrcSw] = Some(val),
            p => panic!("expected rbcrcsw got {p:?}"),
        }
        if kind == DeviceKind::Virtex5 && matches!(packets.peek(), Some(Packet::Testmode(_))) {
            match packets.next() {
                Some(Packet::Testmode(val)) => bs.regs[Reg::Testmode] = Some(val),
                p => panic!("expected testmode got {p:?}"),
            }
        }
        match packets.next() {
            Some(Packet::Cor0(val)) => bs.regs[Reg::Cor0] = Some(val),
            p => panic!("expected cor0 got {p:?}"),
        }
        match packets.next() {
            Some(Packet::Cor1(val)) => bs.regs[Reg::Cor1] = Some(val),
            p => panic!("expected cor1 got {p:?}"),
        }
        match packets.next() {
            Some(Packet::Idcode(val)) => bs.regs[Reg::Idcode] = Some(val),
            p => panic!("expected idcode got {p:?}"),
        }
        assert_eq!(packets.next(), Some(Packet::CmdSwitch));
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
            bs.mode = BitstreamMode::Encrypt;
        }
        let _mask = match packets.next() {
            Some(Packet::Mask(val)) => val,
            p => panic!("expected mask got {p:?}"),
        };
        match packets.next() {
            Some(Packet::Ctl1(val)) => bs.regs[Reg::Ctl1] = Some(val),
            p => panic!("expected ctl1 got {p:?}"),
        }
        for _ in 0..8 {
            assert_eq!(packets.next(), Some(Packet::Nop));
        }
    }

    // main loop
    let frame_bytes = bs.frame_len / 8;
    if bs.mode == BitstreamMode::Encrypt {
        let data;
        if kind == DeviceKind::Virtex4 {
            assert_eq!(packets.next(), Some(Packet::Far(0)));
            assert_eq!(packets.next(), Some(Packet::CmdWcfg));
            assert_eq!(packets.next(), Some(Packet::Nop));
            let init_iv = match packets.next() {
                Some(Packet::Cbc(val)) => val,
                p => panic!("expected fdri got {p:?}"),
            };
            bs.iv = init_iv.clone();
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
                Some(Packet::Cbc(val)) => bs.iv = val,
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
            insert_virtex4_frame(bs, fi, &data[pos..pos + frame_bytes]);
            let cur_reg = bs.frame_info[fi].addr.region;
            fi += 1;
            if fi >= bs.frame_info.len() || bs.frame_info[fi].addr.region != cur_reg {
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
                    bs.mode = BitstreamMode::Compress;
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
                        insert_virtex4_frame(bs, fi, &val[pos..pos + frame_bytes]);
                        let cur_reg = bs.frame_info[fi].addr.region;
                        fi += 1;
                        if fi >= bs.frame_info.len() || bs.frame_info[fi].addr.region != cur_reg {
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
                                if bs.frame_info[fi].addr.typ == 1 {
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
                    insert_virtex4_frame(bs, fi, last_frame.as_ref().unwrap());
                    if kind == DeviceKind::Virtex7 && bs.frame_info[fi].addr.typ == 1 {
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
            bs.regs[Reg::FakeIgnoreCrc] = Some(1);
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
    if kind != DeviceKind::Virtex4 && bs.mode == BitstreamMode::Compress {
        let _mask = match packets.next() {
            Some(Packet::Mask(val)) => val,
            p => panic!("expected mask got {p:?}"),
        };
        match packets.next() {
            Some(Packet::Ctl1(val)) => bs.regs[Reg::Ctl1] = Some(val),
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
            assert_eq!(packets.next(), None);
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
            if bs.regs[Reg::FakeIgnoreCrc].is_some() {
                assert_eq!(packets.next(), Some(Packet::CmdRcrc));
            } else {
                assert_eq!(packets.next(), Some(Packet::Crc));
            }
            if matches!(kind, DeviceKind::Virtex6 | DeviceKind::Virtex7) {
                assert_eq!(packets.next(), Some(Packet::Nop));
                assert_eq!(packets.next(), Some(Packet::Nop));
            }
            if bs.regs[Reg::FakeEncrypted].is_none() {
                assert_eq!(packets.next(), Some(Packet::CmdDesynch));
            }
            let mut num_nops = match kind {
                DeviceKind::Virtex5 => 61,
                DeviceKind::Virtex6 | DeviceKind::Virtex7 => 400,
                _ => unreachable!(),
            };
            if bs.regs[Reg::FakeEncrypted].is_some() {
                num_nops += 2; // desync
                num_nops -= 27; // mask+ctl+cbc+dwc
                num_nops -= 0x10; // encrypted header
                num_nops -= 0x78; // encrypted trailer
            }
            if bs.regs[Reg::Unk1C].is_some() {
                num_nops -= 4;
            }
            if bs.regs[Reg::Trim].is_some() {
                num_nops -= 4;
            }
            if bs.regs[Reg::Testmode].is_some() {
                num_nops -= 2;
            }
            for _ in 0..num_nops {
                assert_eq!(packets.next(), Some(Packet::Nop));
            }
            assert_eq!(packets.next(), None);
        }
        _ => unreachable!(),
    }
    bs.regs[Reg::Ctl0] = Some(ctl0);
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

pub fn parse(geom: &BitstreamGeom, data: &[u8], key: &KeyData) -> Bitstream {
    let mut res = Bitstream {
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
    };
    match res.kind {
        DeviceKind::Xc5200 => parse_xc5200_bitstream(&mut res, data),
        DeviceKind::Virtex | DeviceKind::Virtex2 => parse_virtex_bitstream(&mut res, data, key),
        DeviceKind::Spartan3A => parse_spartan3a_bitstream(&mut res, data, key),
        DeviceKind::Spartan6 => parse_spartan6_bitstream(&mut res, data, key),
        DeviceKind::Virtex4 | DeviceKind::Virtex5 | DeviceKind::Virtex6 | DeviceKind::Virtex7 => {
            parse_virtex4_bitstream(&mut res, data, key)
        }
        DeviceKind::Ultrascale | DeviceKind::UltrascalePlus => {
            parse_ultrascale_bitstream(&res, data, key)
        }
        DeviceKind::Versal => panic!("versal bitstreams not supported through generic code"),
    }
    res
}
