use crate::packet::{Packet, PacketParser};
use crate::{Bitstream, BitstreamGeom, BitstreamMode, DeviceKind, DieBitstream, FrameAddr, Reg};
use arrayref::array_ref;
use bitvec::prelude::*;
use std::collections::HashMap;

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

fn parse_virtex_bitstream(bs: &mut Bitstream, data: &[u8]) {
    let mut packets = PacketParser::new(bs.kind, data);
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
    let _mask = match packets.next() {
        Some(Packet::Mask(val)) => val,
        p => panic!("expected mask got {p:?}"),
    };
    assert_eq!(packets.next(), Some(Packet::CmdSwitch));

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
                insert_virtex_frame(kind, bs, fi, val);
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
                    let last = &val[(frames - 1) * frame_bytes..];
                    last_frame = Some(last);
                }
                (Some(Packet::Mfwr(2)), State::Mfwr) => {
                    packets.next();
                    insert_virtex_frame(kind, bs, fi, last_frame.unwrap());
                }
                (Some(Packet::Far(_)), _) => continue,
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
    assert_eq!(packets.next(), Some(Packet::CmdDGHigh));

    if kind == DeviceKind::Virtex {
        match packets.next() {
            Some(Packet::Fdri(val)) => {
                assert_eq!(val.len(), frame_bytes);
                assert!(val.iter().all(|&x| x == 0));
            }
            p => panic!("expected fdri got {p:?}"),
        }
        if !bs.frame_present[fi] {
            insert_virtex_frame(kind, bs, fi, last_frame.unwrap());
        }
        assert!(bs.frame_present.all());
    } else {
        assert!(bs.frame_present.all());

        let mut nops = 0;
        while let Some(Packet::Nop) = packets.peek() {
            packets.next();
            nops += 1;
        }
        println!("NOPS {nops} FLR {flr}");

        if packets.peek() == Some(Packet::CmdWcfg) {
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
        }

        if packets.peek() == Some(Packet::CmdGRestore) {
            packets.next();
            println!("GOT DOUBLE GRESTORE");
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

fn parse_spartan3a_bitstream(bs: &mut Bitstream, data: &[u8]) {
    let mut packets = PacketParser::new(bs.kind, data);
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
                insert_spartan3a_frame(bs, fi, val);
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
                    mfwr_frame = Some(&val[(frames - 1) * frame_bytes..]);
                }
                (Some(Packet::Mfwr(4)), State::Mfwr) => {
                    let fi = far_dict[&far];
                    assert_ne!(bs.frame_info[fi].addr.typ, 1);
                    insert_spartan3a_frame(bs, fi, mfwr_frame.unwrap());
                }
                (Some(Packet::Mfwr(14)), State::Mfwr) => {
                    let fi = far_dict[&far];
                    assert_eq!(bs.frame_info[fi].addr.typ, 1);
                    insert_spartan3a_frame(bs, fi, mfwr_frame.unwrap());
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

fn parse_spartan6_bitstream(bs: &mut Bitstream, data: &[u8]) {
    let mut packets = PacketParser::new(bs.kind, data);
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
    for _ in 0..17 {
        assert_eq!(packets.next(), Some(Packet::Nop));
    }

    match packets.next() {
        Some(Packet::CclkFrequency(val)) => bs.regs[Reg::CclkFrequency] = Some(val),
        p => panic!("expected cclk frequency got {p:?}"),
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
    assert_eq!(packets.next(), Some(Packet::Nop));
    assert_eq!(packets.next(), Some(Packet::Nop));

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
    let mut last_frame = None;
    let mut state = State::None;
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
                insert_spartan3a_frame(bs, fi, last_frame.unwrap());
            }
            Some(Packet::Fdri(mut data)) => {
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
                                insert_spartan3a_frame(bs, fi, last_frame.unwrap());
                                frame = Frame::Main(fi + 1);
                                if fi == bs.frame_info.len() - 1
                                    || bs.frame_info[fi + 1].addr.region
                                        != bs.frame_info[fi].addr.region
                                {
                                    skip = 2;
                                }
                            }
                            last_frame = Some(cur);
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

fn parse_virtex4_bitstream(bs: &mut Bitstream, data: &[u8]) {
    let mut packets = PacketParser::new(bs.kind, data);
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
                    DeviceKind::Series7 => virtex7_far(f.addr),
                    _ => unreachable!(),
                },
                i,
            )
        })
        .collect();

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
        assert_eq!(packets.next(), Some(Packet::CmdSwitch));
        assert_eq!(packets.next(), Some(Packet::Nop));
        // XXX
        let _mask = match packets.next() {
            Some(Packet::Mask(val)) => val,
            p => panic!("expected mask got {p:?}"),
        };
        match packets.next() {
            Some(Packet::Ctl0(val)) => bs.regs[Reg::Ctl0] = Some(val),
            p => panic!("expected ctl0 got {p:?}"),
        }
        for _ in 0..1150 {
            assert_eq!(packets.next(), Some(Packet::Nop));
        }
        let _mask = match packets.next() {
            Some(Packet::Mask(val)) => val,
            p => panic!("expected mask got {p:?}"),
        };
        match packets.next() {
            Some(Packet::Ctl0(val)) => bs.regs[Reg::Ctl0] = Some(val),
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
        if kind == DeviceKind::Series7 {
            match packets.next() {
                Some(Packet::Timer(val)) => bs.regs[Reg::Timer] = Some(val),
                p => panic!("expected timer got {p:?}"),
            }
        }
        match packets.next() {
            Some(Packet::WBStar(val)) => bs.regs[Reg::WbStar] = Some(val),
            p => panic!("expected wbstar got {p:?}"),
        }
        assert_eq!(packets.next(), Some(Packet::CmdNull));
        assert_eq!(packets.next(), Some(Packet::Nop));
        assert_eq!(packets.next(), Some(Packet::CmdRcrc));
        assert_eq!(packets.next(), Some(Packet::Nop));
        assert_eq!(packets.next(), Some(Packet::Nop));
        if kind != DeviceKind::Series7 {
            match packets.next() {
                Some(Packet::Timer(val)) => bs.regs[Reg::Timer] = Some(val),
                p => panic!("expected timer got {p:?}"),
            }
        }
        match packets.next() {
            Some(Packet::RbCrcSw(val)) => bs.regs[Reg::RbCrcSw] = Some(val),
            p => panic!("expected rbcrcsw got {p:?}"),
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
        let _mask = match packets.next() {
            Some(Packet::Mask(val)) => val,
            p => panic!("expected mask got {p:?}"),
        };
        match packets.next() {
            Some(Packet::Ctl0(val)) => bs.regs[Reg::Ctl0] = Some(val),
            p => panic!("expected ctl0 got {p:?}"),
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
                if kind != DeviceKind::Series7 {
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
                    DeviceKind::Virtex6 | DeviceKind::Series7 => 12,
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
                let last = &val[(frames - 1) * frame_bytes..];
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
                        DeviceKind::Series7 =>
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
                insert_virtex4_frame(bs, fi, last_frame.unwrap());
                if kind == DeviceKind::Series7 && bs.frame_info[fi].addr.typ == 1 {
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
    assert_eq!(packets.next(), Some(Packet::Crc));
    if matches!(kind, DeviceKind::Virtex6 | DeviceKind::Series7) {
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
            for _ in 0..16 {
                assert_eq!(packets.next(), Some(Packet::Nop));
            }
            assert_eq!(packets.next(), None);
        }
        DeviceKind::Virtex5 | DeviceKind::Virtex6 | DeviceKind::Series7 => {
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
            let _mask = match packets.next() {
                Some(Packet::Mask(val)) => val,
                p => panic!("expected mask got {p:?}"),
            };
            match packets.next() {
                Some(Packet::Ctl0(val)) => bs.regs[Reg::Ctl0] = Some(val),
                p => panic!("expected ctl0 got {p:?}"),
            }
            assert_eq!(packets.next(), Some(Packet::Crc));
            if matches!(kind, DeviceKind::Virtex6 | DeviceKind::Series7) {
                assert_eq!(packets.next(), Some(Packet::Nop));
                assert_eq!(packets.next(), Some(Packet::Nop));
            }
            assert_eq!(packets.next(), Some(Packet::CmdDesynch));
            let num_nops = match kind {
                DeviceKind::Virtex5 => 61,
                DeviceKind::Virtex6 | DeviceKind::Series7 => 400,
                _ => unreachable!(),
            };
            for _ in 0..num_nops {
                assert_eq!(packets.next(), Some(Packet::Nop));
            }
            assert_eq!(packets.next(), None);
        }
        _ => unreachable!(),
    }
}

fn parse_ultrascale_bitstream(bs: &Bitstream, data: &[u8]) {
    let packets = PacketParser::new(bs.kind, data);
    for packet in packets {
        if let Packet::Fdri(data) = packet {
            println!("PACKET FDRI {l}", l = data.len());
        } else {
            println!("PACKET {packet:x?}");
        }
    }
    todo!()
}

fn parse_ultrascaleplus_bitstream(_bs: &mut Bitstream, _data: &[u8]) {
    todo!()
}

pub fn parse(geom: &BitstreamGeom, data: &[u8]) -> Bitstream {
    let mut res = Bitstream {
        kind: geom.kind,
        die: geom.die.map_values(|dg| DieBitstream {
            regs: Default::default(),
            mode: BitstreamMode::Plain,
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
        DeviceKind::Virtex => parse_virtex_bitstream(&mut res, data),
        DeviceKind::Virtex2 => parse_virtex_bitstream(&mut res, data),
        DeviceKind::Spartan3A => parse_spartan3a_bitstream(&mut res, data),
        DeviceKind::Spartan6 => parse_spartan6_bitstream(&mut res, data),
        DeviceKind::Virtex4 | DeviceKind::Virtex5 | DeviceKind::Virtex6 | DeviceKind::Series7 => {
            parse_virtex4_bitstream(&mut res, data)
        }
        DeviceKind::Ultrascale => parse_ultrascale_bitstream(&res, data),
        DeviceKind::UltrascalePlus => parse_ultrascaleplus_bitstream(&mut res, data),
        DeviceKind::Versal => panic!("versal bitstreams not supported through generic code"),
    }
    res
}
