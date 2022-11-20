use crate::DeviceKind;
use arrayref::array_ref;

#[derive(Debug, Clone)]
pub struct PacketParser<'a> {
    kind: DeviceKind,
    data: &'a [u8],
    pos: usize,
    sync: bool,
    last_reg: Option<u32>,
    crc: u32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Packet<'a> {
    // unsynced
    DummyWord,
    WidthDetect,
    SyncWord,
    // synced
    Nop,
    CmdNull,
    CmdWcfg,
    CmdMfwr,
    CmdDGHigh,
    CmdStart,
    CmdRcrc,
    CmdSwitch,
    CmdGRestore,
    CmdDesynch,
    Crc,
    Flr(u32),
    Mask(u32),
    RbCrcSw(u32),
    Far(u32),
    Mfwr(usize),
    LoutDebug(u32),
    Cor0(u32),
    Cor1(u32),
    Cor2(u32),
    Ctl0(u32),
    Ctl1(u32),
    Idcode(u32),
    Timer(u32),
    Powerdown(u32),
    HcOpt(u32),
    Mode(u32),
    PuGwe(u32),
    PuGts(u32),
    CclkFrequency(u32),
    SeuOpt(u32),
    General1(u32),
    General2(u32),
    General3(u32),
    General4(u32),
    General5(u32),
    EyeMask(u32),
    WBStar(u32),
    Fdri(&'a [u8]),
    BugFdri(&'a [u8]),
}

impl<'a> PacketParser<'a> {
    pub fn new(kind: DeviceKind, data: &'a [u8]) -> Self {
        Self {
            kind,
            data,
            pos: 0,
            sync: false,
            last_reg: None,
            crc: 0,
        }
    }

    pub fn reset_crc(&mut self) {
        self.crc = 0;
    }

    pub fn update_crc(&mut self, reg: u32, val: u32) {
        match self.kind {
            DeviceKind::Spartan3A | DeviceKind::Spartan6 => {
                // Yup, rotates once per 22 bits. Really.
                let data = val | reg << 16;
                self.crc <<= 1;
                if (self.crc & 0x400000) != 0 {
                    self.crc ^= 0x409081;
                }
                self.crc ^= data;
            }
            _ => {
                let poly = match self.kind {
                    DeviceKind::Virtex | DeviceKind::Virtex2 => 0xa001,
                    _ => 0x82f63b78,
                };
                for i in 0..32 {
                    let bit = (val >> i & 1) ^ (self.crc & 1);
                    self.crc >>= 1;
                    if bit != 0 {
                        self.crc ^= poly;
                    }
                }
                let rw = if self.kind == DeviceKind::Virtex {
                    4
                } else {
                    5
                };
                for i in 0..rw {
                    let bit = (reg >> i & 1) ^ (self.crc & 1);
                    self.crc >>= 1;
                    if bit != 0 {
                        self.crc ^= poly;
                    }
                }
            }
        }
    }

    pub fn peek(&self) -> Option<Packet<'a>> {
        self.clone().next()
    }
}

impl<'a> Iterator for PacketParser<'a> {
    type Item = Packet<'a>;

    fn next(&mut self) -> Option<Packet<'a>> {
        if matches!(self.kind, DeviceKind::Spartan3A | DeviceKind::Spartan6) {
            let is_s6 = self.kind == DeviceKind::Spartan6;
            if self.pos + 2 > self.data.len() {
                None
            } else {
                let ph = u16::from_be_bytes(*array_ref!(self.data, self.pos, 2));
                self.pos += 2;
                if self.sync {
                    if ph == 0x2000 {
                        Some(Packet::Nop)
                    } else if (ph >> 11) == 6 {
                        let reg = ph >> 5 & 0x3f;
                        let num = (ph & 0x1f) as usize;
                        let dpos = self.pos;
                        self.pos += num * 2;
                        let epos = self.pos;
                        let get_val = |i: usize| {
                            u16::from_be_bytes(*array_ref!(self.data, dpos + i * 2, 2)) as u32
                        };
                        let get_val32 =
                            |i: usize| u32::from_be_bytes(*array_ref!(self.data, dpos + i * 2, 4));
                        if !matches!(reg, 0 | 9 | 0x12) {
                            for i in 0..num {
                                self.update_crc(reg as u32, get_val(i));
                            }
                        }
                        match (reg, num) {
                            (0, 2) => {
                                let val = get_val32(0);
                                if val != self.crc {
                                    println!("CRC MISMATCH {val:08x} {ecrc:08x}", ecrc = self.crc);
                                }
                                Some(Packet::Crc)
                            }
                            (1, 2) => Some(Packet::Far(get_val32(0))),
                            (5, 1) => match get_val(0) {
                                0 => Some(Packet::CmdNull),
                                1 => Some(Packet::CmdWcfg),
                                2 => Some(Packet::CmdMfwr),
                                3 => Some(Packet::CmdDGHigh),
                                5 => Some(Packet::CmdStart),
                                7 => {
                                    self.reset_crc();
                                    Some(Packet::CmdRcrc)
                                }
                                9 => Some(Packet::CmdSwitch),
                                10 => Some(Packet::CmdGRestore),
                                13 => Some(Packet::CmdDesynch),
                                val => panic!("unk cmd: {val}"),
                            },
                            (6, 1) => Some(Packet::Ctl0(get_val(0))),
                            (7, 1) => Some(Packet::Mask(get_val(0))),
                            (9, 2) => Some(Packet::LoutDebug(get_val32(0))),
                            (0xa, 1) => Some(Packet::Cor1(get_val(0))),
                            (0xb, 1) => Some(Packet::Cor2(get_val(0))),
                            (0xc, 1) => Some(Packet::Powerdown(get_val(0))),
                            (0xd, 1) => Some(Packet::Flr(get_val(0))),
                            (0xe, 2) => Some(Packet::Idcode(get_val32(0))),
                            (0xf, 1) if is_s6 => Some(Packet::Timer(get_val(0))),
                            (0x10, 1) => Some(Packet::HcOpt(get_val(0))),
                            (0x13, 1) => Some(Packet::General1(get_val(0))),
                            (0x14, 1) => Some(Packet::General2(get_val(0))),
                            (0x15, 1) if !is_s6 => Some(Packet::Mode(get_val(0))),
                            (0x16, 1) if !is_s6 => Some(Packet::PuGwe(get_val(0))),
                            (0x17, 1) if !is_s6 => Some(Packet::PuGts(get_val(0))),
                            (0x18, _) if !is_s6 => {
                                assert!(self.data[dpos..epos].iter().all(|&x| x == 0));
                                Some(Packet::Mfwr(num))
                            }
                            (0x19, 1) if !is_s6 => Some(Packet::CclkFrequency(get_val(0))),
                            (0x1a, 1) if !is_s6 => Some(Packet::SeuOpt(get_val(0))),
                            (0x1b, 2) if !is_s6 => Some(Packet::RbCrcSw(get_val32(0))),
                            (0x15, 1) if is_s6 => Some(Packet::General3(get_val(0))),
                            (0x16, 1) if is_s6 => Some(Packet::General4(get_val(0))),
                            (0x17, 1) if is_s6 => Some(Packet::General5(get_val(0))),
                            (0x18, 1) if is_s6 => Some(Packet::Mode(get_val(0))),
                            (0x19, 1) if is_s6 => Some(Packet::PuGwe(get_val(0))),
                            (0x1a, 1) if is_s6 => Some(Packet::PuGts(get_val(0))),
                            (0x1b, _) if is_s6 => {
                                assert!(self.data[dpos..epos].iter().all(|&x| x == 0));
                                Some(Packet::Mfwr(num))
                            }
                            (0x1c, 1) if is_s6 => Some(Packet::CclkFrequency(get_val(0))),
                            (0x1d, 1) if is_s6 => Some(Packet::SeuOpt(get_val(0))),
                            (0x1e, 2) if is_s6 => Some(Packet::RbCrcSw(get_val32(0))),
                            (0x21, 1) if is_s6 => Some(Packet::EyeMask(get_val(0))),
                            _ => panic!("unk write: {reg} times {num}"),
                        }
                    } else if (ph >> 11) == 0xa {
                        let reg = ph >> 5 & 0x3f;
                        let num = u32::from_be_bytes(*array_ref!(self.data, self.pos, 4)) as usize;
                        self.pos += 4;
                        let dpos = self.pos;
                        self.pos += num * 2;
                        let epos = self.pos;
                        let get_val = |i: usize| {
                            u16::from_be_bytes(*array_ref!(self.data, dpos + i * 2, 2)) as u32
                        };
                        for i in 0..num {
                            self.update_crc(reg as u32, get_val(i));
                        }
                        match reg {
                            3 => {
                                if self.kind == DeviceKind::Spartan6 {
                                    let crc = u32::from_be_bytes(*array_ref!(self.data, epos, 4));
                                    if crc != self.crc {
                                        println!(
                                            "AUTOCRC MISMATCH {crc:08x} {ecrc:08x}",
                                            ecrc = self.crc
                                        );
                                    }
                                    self.pos += 4;
                                }
                                Some(Packet::Fdri(&self.data[dpos..epos]))
                            }
                            _ => panic!("unk long write: {reg} times {num}"),
                        }
                    } else {
                        panic!("unk word: {ph:04x}")
                    }
                } else {
                    match ph {
                        0xffff => Some(Packet::DummyWord),
                        0xaa99 => {
                            if self.kind == DeviceKind::Spartan6 {
                                assert_eq!(
                                    u32::from_be_bytes(*array_ref!(self.data, self.pos - 2, 4)),
                                    0xaa995566
                                );
                                self.pos += 2;
                            }
                            self.sync = true;
                            Some(Packet::SyncWord)
                        }
                        _ => panic!("unk word while desyncd: {ph:04x}"),
                    }
                }
            }
        } else {
            let is_v4 = !matches!(self.kind, DeviceKind::Virtex | DeviceKind::Virtex2);
            if self.pos + 4 > self.data.len() {
                None
            } else {
                loop {
                    let ph = u32::from_be_bytes(*array_ref!(self.data, self.pos, 4));
                    self.pos += 4;
                    break if self.sync {
                        if ph == 0x00000000 && self.kind == DeviceKind::Virtex {
                            Some(Packet::Nop)
                        } else if ph == 0x00000000 && is_v4 {
                            // Not a valid packet, but a manifestation of a bitgen bug
                            // that emits broken debug bitstreams for Virtex4+.
                            // The packet header is missing, should be a long write to FDRI.
                            self.pos -= 4;
                            let dpos = self.pos;
                            let get_val = |i: usize| {
                                u32::from_be_bytes(*array_ref!(self.data, dpos + i * 4, 4))
                            };
                            let mut i = 0;
                            while get_val(i) == 0 {
                                i += 1;
                            }
                            let num = i;
                            self.pos += num * 4;
                            let epos = self.pos;
                            for i in 0..num {
                                self.update_crc(2, get_val(i));
                            }
                            Some(Packet::BugFdri(&self.data[dpos..epos]))
                        } else if ph == 0x20000000 {
                            Some(Packet::Nop)
                        } else if (ph >> 27) == 6 {
                            let reg = ph >> 13 & 0x3fff;
                            let num = (ph & 0x1fff) as usize;
                            let dpos = self.pos;
                            self.last_reg = Some(reg);
                            self.pos += num * 4;
                            let epos = self.pos;
                            let get_val = |i: usize| {
                                u32::from_be_bytes(*array_ref!(self.data, dpos + i * 4, 4))
                            };
                            let ecrc = self.crc;
                            if !matches!(reg, 8 | 0xf | 0x1e) {
                                for i in 0..num {
                                    self.update_crc(reg, get_val(i));
                                }
                            }
                            match (reg, num) {
                                (0, 1) => {
                                    let val = get_val(0);
                                    if val != ecrc {
                                        println!("CRC MISMATCH {val:08x} {ecrc:08x}");
                                    }
                                    Some(Packet::Crc)
                                }
                                (1, 1) => Some(Packet::Far(get_val(0))),
                                (2, 0) => continue,
                                (2, _) => {
                                    if self.kind == DeviceKind::Virtex2 {
                                        let crc =
                                            u32::from_be_bytes(*array_ref!(self.data, epos, 4));
                                        if crc != self.crc {
                                            println!(
                                                "AUTOCRC MISMATCH {crc:08x} {ecrc:08x}",
                                                ecrc = self.crc
                                            );
                                        }
                                        self.pos += 4;
                                        self.reset_crc();
                                    }
                                    Some(Packet::Fdri(&self.data[dpos..epos]))
                                }
                                (4, 1) => match get_val(0) {
                                    0 => Some(Packet::CmdNull),
                                    1 => Some(Packet::CmdWcfg),
                                    2 => Some(Packet::CmdMfwr),
                                    3 => Some(Packet::CmdDGHigh),
                                    5 => Some(Packet::CmdStart),
                                    7 => {
                                        self.reset_crc();
                                        Some(Packet::CmdRcrc)
                                    }
                                    9 => Some(Packet::CmdSwitch),
                                    10 => Some(Packet::CmdGRestore),
                                    13 => Some(Packet::CmdDesynch),
                                    val => panic!("unk cmd: {val}"),
                                },
                                (5, 1) => Some(Packet::Ctl0(get_val(0))),
                                (6, 1) => Some(Packet::Mask(get_val(0))),
                                (8, 1) => Some(Packet::LoutDebug(get_val(0))),
                                (9, 1) => Some(Packet::Cor0(get_val(0))),
                                (0xa, _) => {
                                    assert!(self.data[dpos..epos].iter().all(|&x| x == 0));
                                    Some(Packet::Mfwr(num))
                                }
                                (0xb, 1) if !is_v4 => Some(Packet::Flr(get_val(0))),
                                (0xe, 1) if !is_v4 => Some(Packet::Idcode(get_val(0))),
                                (0xc, 1) if is_v4 => Some(Packet::Idcode(get_val(0))),
                                (0xe, 1) if is_v4 => Some(Packet::Cor1(get_val(0))),
                                (0x10, 1) if is_v4 => Some(Packet::WBStar(get_val(0))),
                                (0x11, 1) if is_v4 => Some(Packet::Timer(get_val(0))),
                                (0x13, 1) if is_v4 => Some(Packet::RbCrcSw(get_val(0))),
                                (0x18, 1) if is_v4 => Some(Packet::Ctl1(get_val(0))),
                                _ => panic!("unk write: {reg} times {num}"),
                            }
                        } else if (ph >> 27) == 0xa {
                            let reg = self.last_reg.unwrap();
                            let num = (ph & 0x7ffffff) as usize;
                            let dpos = self.pos;
                            self.pos += num * 4;
                            let epos = self.pos;
                            let get_val = |i: usize| {
                                u32::from_be_bytes(*array_ref!(self.data, dpos + i * 4, 4))
                            };
                            for i in 0..num {
                                self.update_crc(reg, get_val(i));
                            }
                            match reg {
                                2 => {
                                    if self.kind == DeviceKind::Virtex2 {
                                        let crc =
                                            u32::from_be_bytes(*array_ref!(self.data, epos, 4));
                                        if crc != self.crc {
                                            println!(
                                                "AUTOCRC MISMATCH {crc:08x} {ecrc:08x}",
                                                ecrc = self.crc
                                            );
                                        }
                                        self.pos += 4;
                                        self.reset_crc();
                                    }
                                    Some(Packet::Fdri(&self.data[dpos..epos]))
                                }
                                _ => panic!("unk long write: {reg} times {num}"),
                            }
                        } else {
                            panic!("unk word: {ph:08x}")
                        }
                    } else {
                        match ph {
                            0xffffffff => Some(Packet::DummyWord),
                            0x000000bb => {
                                let w2 = u32::from_be_bytes(*array_ref!(self.data, self.pos, 4));
                                assert_eq!(w2, 0x11220044);
                                self.pos += 4;
                                Some(Packet::WidthDetect)
                            }
                            0xaa995566 => {
                                self.sync = true;
                                Some(Packet::SyncWord)
                            }
                            _ => panic!("unk word while desyncd: {ph:08x}"),
                        }
                    };
                }
            }
        }
    }
}
