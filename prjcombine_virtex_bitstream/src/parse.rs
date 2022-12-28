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
            println!("GOT FIXUP");
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
                    if frames == 1 {
                        mfwr_frame = Some(val);
                    } else {
                        let fi = far_dict[&far];
                        for i in 0..(frames - 1) {
                            let pos = i * frame_bytes;
                            insert_spartan3a_frame(bs, fi + i, &val[pos..pos + frame_bytes]);
                        }
                        let pad = &val[(frames - 1) * frame_bytes..];
                        assert!(pad.iter().all(|&x| x == 0));
                        mfwr_frame = None;
                    }
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
    let packets = PacketParser::new(bs.kind, data);
    for packet in packets {
        if let Packet::Fdri(data) = packet {
            println!("PACKET FDRI {l}", l = data.len());
        } else {
            println!("PACKET {packet:#x?}");
        }
    }
    //todo!()
}

fn parse_virtex4_bitstream(bs: &mut Bitstream, data: &[u8]) {
    let packets = PacketParser::new(bs.kind, data);
    for packet in packets {
        if let Packet::Fdri(data) = packet {
            println!("PACKET FDRI {l}", l = data.len());
        } else {
            println!("PACKET {packet:#x?}");
        }
    }
    //todo!()
}

fn parse_virtex5_bitstream(bs: &mut Bitstream, data: &[u8]) {
    let packets = PacketParser::new(bs.kind, data);
    for packet in packets {
        if let Packet::Fdri(data) = packet {
            println!("PACKET FDRI {l}", l = data.len());
        } else {
            println!("PACKET {packet:#x?}");
        }
    }
    //todo!()
}

fn parse_virtex6_bitstream(bs: &mut Bitstream, data: &[u8]) {
    let packets = PacketParser::new(bs.kind, data);
    for packet in packets {
        if let Packet::Fdri(data) = packet {
            println!("PACKET FDRI {l}", l = data.len());
        } else {
            println!("PACKET {packet:#x?}");
        }
    }
    //todo!()
}

fn parse_series7_bitstream(bs: &mut Bitstream, data: &[u8]) {
    let packets = PacketParser::new(bs.kind, data);
    for packet in packets {
        if let Packet::Fdri(data) = packet {
            println!("PACKET FDRI {l}", l = data.len());
        } else {
            println!("PACKET {packet:#x?}");
        }
    }
    //todo!()
}

fn parse_ultrascale_bitstream(_bs: &mut Bitstream, _data: &[u8]) {
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
            bram_present: BitVec::repeat(false, dg.bram_frame_info.len()),
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
        DeviceKind::Virtex4 => parse_virtex4_bitstream(&mut res, data),
        DeviceKind::Virtex5 => parse_virtex5_bitstream(&mut res, data),
        DeviceKind::Virtex6 => parse_virtex6_bitstream(&mut res, data),
        DeviceKind::Series7 => parse_series7_bitstream(&mut res, data),
        DeviceKind::Ultrascale => parse_ultrascale_bitstream(&mut res, data),
        DeviceKind::UltrascalePlus => parse_ultrascaleplus_bitstream(&mut res, data),
        DeviceKind::Versal => panic!("versal bitstreams not supported through generic code"),
    }
    res
}
