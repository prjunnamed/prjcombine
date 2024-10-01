use bitvec::prelude::*;
use enum_map::{Enum, EnumMap};
use prjcombine_int::grid::DieId;
use std::collections::HashMap;
use unnamed_entity::EntityVec;

mod packet;
mod parse;
pub use parse::parse;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Enum)]
pub enum Reg {
    Idcode,
    Ctl0,
    Ctl1,
    Unk1C,
    Cor0,
    Cor1,
    Cor2,
    Key,
    WbStar,
    Timer,
    Trim0,
    Trim1,
    Trim2,
    Testmode,
    Bspi,
    Axss,
    RbCrcSw,
    CclkFrequency,
    Powerdown,
    EyeMask,
    HcOpt,
    PuGwe,
    PuGts,
    SeuOpt,
    ExpSign,
    Mode,
    General1,
    General2,
    General3,
    General4,
    General5,
    FakeLcAlignmentDone,
    FakeEarlyGhigh,
    FakeDoubleGrestore,
    FakeFreezeDciNops,
    FakeIgnoreCrc,
    FakeEncrypted,
    FakeDoubleCclkFrequency,
    FakeHasSwitch,
    FakeFallEdge,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum DeviceKind {
    Xc5200,
    Virtex,
    Virtex2,
    Spartan3A,
    Spartan6,
    Virtex4,
    Virtex5,
    Virtex6,
    Virtex7,
    Ultrascale,
    UltrascalePlus,
    Versal,
}

#[derive(Clone, Debug)]
pub enum KeyData {
    None,
    Des(KeyDataDes),
    Aes(KeyDataAes),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum KeySeq {
    First,
    Middle,
    Last,
    Single,
}

#[derive(Clone, Debug)]
pub struct KeyDataDes {
    pub key: [[u8; 7]; 6],
    pub keyseq: [KeySeq; 6],
}

#[derive(Clone, Debug)]
pub struct KeyDataAes {
    pub key: [u8; 32],
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum BitstreamMode {
    Plain,
    Debug,
    Compress,
    PerFrameCrc,
    Encrypt,
}

#[derive(Clone, Debug)]
pub struct BitstreamGeom {
    pub kind: DeviceKind,
    pub die: EntityVec<DieId, DieBitstreamGeom>,
    pub die_order: Vec<DieId>,
}

#[derive(Clone, Debug)]
pub struct DieBitstreamGeom {
    pub frame_len: usize,
    pub frame_info: Vec<FrameInfo>,
    // spartan 6 only
    pub bram_frame_len: usize,
    pub bram_frame_info: Vec<FrameInfo>,
    pub iob_frame_len: usize,
}

#[derive(Clone, Debug)]
pub struct Bitstream {
    pub kind: DeviceKind,
    pub die: EntityVec<DieId, DieBitstream>,
}

impl Bitstream {
    pub fn diff(a: &Bitstream, b: &Bitstream) -> HashMap<BitPos, bool> {
        assert_eq!(a.kind, b.kind);
        assert_eq!(a.die.len(), b.die.len());
        let mut res = HashMap::new();
        for ((die, da), db) in a.die.iter().zip(b.die.values()) {
            for (reg, &va) in &da.regs {
                if matches!(
                    reg,
                    Reg::RbCrcSw | Reg::Key | Reg::FakeEncrypted | Reg::FakeDoubleCclkFrequency
                ) {
                    continue;
                }
                let vb = db.regs[reg];
                if va.is_some() != vb.is_some() && reg != Reg::Testmode {
                    res.insert(BitPos::RegPresent(die, reg), vb.is_some());
                }
                let va = va.unwrap_or(0);
                let vb = vb.unwrap_or(0);
                if va != vb {
                    for j in 0..32 {
                        if (va >> j & 1) != (vb >> j & 1) {
                            res.insert(BitPos::Reg(die, reg, j), (vb >> j & 1) != 0);
                        }
                    }
                }
            }
            assert_eq!(da.frame_len, db.frame_len);
            assert_eq!(da.frame_info, db.frame_info);
            for i in 0..da.frame_info.len() {
                let fa = da.frame(i);
                let fb = db.frame(i);
                if fa == fb {
                    continue;
                }
                for j in 0..da.frame_len {
                    if fa[j] != fb[j] {
                        let is_ecc = match a.kind {
                            DeviceKind::Virtex4 | DeviceKind::Virtex5 => (640..652).contains(&j),
                            DeviceKind::Virtex6 => (1280..1293).contains(&j),
                            DeviceKind::Virtex7 => (1600..1613).contains(&j),
                            _ => false,
                        };
                        if !is_ecc {
                            res.insert(BitPos::Main(die, i, j), fb[j]);
                        }
                    }
                }
            }
            assert_eq!(da.bram_frame_len, db.bram_frame_len);
            assert_eq!(da.bram_frame_info, db.bram_frame_info);
            for i in 0..da.bram_frame_info.len() {
                let fa = da.bram_frame(i);
                let fb = db.bram_frame(i);
                if fa == fb {
                    continue;
                }
                for j in 0..da.bram_frame_len {
                    if fa[j] != fb[j] {
                        res.insert(BitPos::Bram(die, i, j), fb[j]);
                    }
                }
            }
            if da.iob != db.iob {
                assert_eq!(da.iob.len(), db.iob.len());
                for j in 0..da.iob.len() {
                    if da.iob[j] != db.iob[j] {
                        res.insert(BitPos::Iob(die, j), db.iob[j]);
                    }
                }
            }
            for k in da.frame_fixups.keys() {
                if !db.frame_fixups.contains_key(k) {
                    res.insert(BitPos::Fixup(die, k.0, k.1), false);
                }
            }
            for k in db.frame_fixups.keys() {
                if !da.frame_fixups.contains_key(k) {
                    res.insert(BitPos::Fixup(die, k.0, k.1), true);
                }
            }
        }
        res
    }

    pub fn get_bit(&self, bit: BitPos) -> bool {
        match bit {
            BitPos::Reg(die, reg, bit) => match self.die[die].regs[reg] {
                Some(val) => (val >> bit & 1) != 0,
                None => false,
            },
            BitPos::RegPresent(die, reg) => self.die[die].regs[reg].is_some(),
            BitPos::Main(die, frame, bit) => self.die[die].frame(frame)[bit],
            BitPos::Fixup(die, frame, bit) => {
                self.die[die].frame_fixups.contains_key(&(frame, bit))
            }
            BitPos::Bram(die, frame, bit) => self.die[die].bram_frame(frame)[bit],
            BitPos::Iob(die, bit) => self.die[die].iob[bit],
        }
    }
}

#[derive(Clone, Debug)]
pub struct DieBitstream {
    pub regs: EnumMap<Reg, Option<u32>>,
    pub mode: BitstreamMode,
    pub iv: Vec<u8>,
    pub frame_len: usize,
    pub frame_data: BitVec,
    pub frame_info: Vec<FrameInfo>,
    pub frame_present: BitVec,
    // spartan 6 only
    pub bram_frame_len: usize,
    pub bram_data: BitVec,
    pub bram_frame_info: Vec<FrameInfo>,
    pub bram_frame_present: BitVec,
    pub iob: BitVec,
    pub iob_present: bool,
    // frame idx, bit idx
    pub frame_fixups: HashMap<(usize, usize), bool>,
}

impl DieBitstream {
    pub fn frame_mut(&mut self, fi: usize) -> &mut BitSlice {
        let pos = fi * self.frame_len;
        &mut self.frame_data[pos..pos + self.frame_len]
    }

    pub fn frame(&self, fi: usize) -> &BitSlice {
        let pos = fi * self.frame_len;
        &self.frame_data[pos..pos + self.frame_len]
    }

    pub fn bram_frame_mut(&mut self, fi: usize) -> &mut BitSlice {
        let pos = fi * self.bram_frame_len;
        &mut self.bram_data[pos..pos + self.bram_frame_len]
    }

    pub fn bram_frame(&self, fi: usize) -> &BitSlice {
        let pos = fi * self.bram_frame_len;
        &self.bram_data[pos..pos + self.bram_frame_len]
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct FrameAddr {
    pub typ: u32,
    pub region: i32,
    pub major: u32,
    pub minor: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameInfo {
    pub addr: FrameAddr,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum BitPos {
    Reg(DieId, Reg, usize),
    RegPresent(DieId, Reg),
    // die, frame, bit
    Main(DieId, usize, usize),
    Fixup(DieId, usize, usize),
    Bram(DieId, usize, usize),
    Iob(DieId, usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum BitTile {
    Null,
    Reg(DieId, Reg),
    RegPresent(DieId, Reg),
    // die, frame, width, bit, height, flip
    Main(DieId, usize, usize, usize, usize, bool),
    Fixup(DieId, usize, usize, usize, usize, bool),
    // Spartan 6 horrible; single whole frame
    Bram(DieId, usize),
    // Spartan 6 horrible; bit, width
    Iob(DieId, usize, usize),
}

impl BitTile {
    pub fn xlat_pos_rev(&self, bit: BitPos) -> Option<(usize, usize)> {
        match (*self, bit) {
            (BitTile::Reg(die, reg), BitPos::Reg(bdie, breg, pos))
                if bdie == die && breg == reg =>
            {
                Some((0, pos))
            }
            (BitTile::RegPresent(die, reg), BitPos::RegPresent(bdie, breg))
                if bdie == die && breg == reg =>
            {
                Some((0, 0))
            }
            (
                BitTile::Main(die, frame, width, bit, height, flip),
                BitPos::Main(bdie, bframe, bbit),
            ) if die == bdie
                && bframe >= frame
                && bframe < frame + width
                && bbit >= bit
                && bbit < bit + height =>
            {
                Some((
                    bframe - frame,
                    if flip {
                        height - 1 - (bbit - bit)
                    } else {
                        bbit - bit
                    },
                ))
            }
            (
                BitTile::Fixup(die, frame, width, bit, height, flip),
                BitPos::Fixup(bdie, bframe, bbit),
            ) if die == bdie
                && bframe >= frame
                && bframe < frame + width
                && bbit >= bit
                && bbit < bit + height =>
            {
                Some((
                    bframe - frame,
                    if flip {
                        height - 1 - (bbit - bit)
                    } else {
                        bbit - bit
                    },
                ))
            }
            (BitTile::Bram(die, frame), BitPos::Bram(bdie, bframe, pos))
                if bdie == die && bframe == frame =>
            {
                Some((0, pos))
            }
            (BitTile::Iob(die, bit, height), BitPos::Iob(bdie, bbit))
                if die == bdie && bbit >= bit && bbit < bit + height =>
            {
                Some((0, bbit - bit))
            }
            _ => None,
        }
    }

    pub fn xlat_pos_fwd(&self, bit: (usize, usize)) -> BitPos {
        let (tframe, tbit) = bit;
        match *self {
            BitTile::Null => unreachable!(),
            BitTile::Reg(die, reg) => {
                assert_eq!(tframe, 0);
                BitPos::Reg(die, reg, tbit)
            }
            BitTile::RegPresent(die, reg) => {
                assert_eq!(tframe, 0);
                assert_eq!(tbit, 0);
                BitPos::RegPresent(die, reg)
            }
            BitTile::Main(die, frame, width, bit, height, flip) => {
                assert!(tframe < width);
                assert!(tbit < height);
                BitPos::Main(
                    die,
                    frame + tframe,
                    if flip {
                        bit + height - 1 - tbit
                    } else {
                        bit + tbit
                    },
                )
            }
            BitTile::Fixup(die, frame, width, bit, height, flip) => {
                assert!(tframe < width);
                assert!(tbit < height);
                BitPos::Fixup(
                    die,
                    frame + tframe,
                    if flip {
                        bit + height - 1 - tbit
                    } else {
                        bit + tbit
                    },
                )
            }
            BitTile::Bram(die, frame) => {
                assert_eq!(tframe, 0);
                BitPos::Bram(die, frame, tbit)
            }
            BitTile::Iob(die, bit, height) => {
                assert!(tbit < height);
                BitPos::Iob(die, bit + tbit)
            }
        }
    }

    pub fn to_fixup(self) -> BitTile {
        match self {
            BitTile::Main(die, frame, width, bit, height, flip) => {
                BitTile::Fixup(die, frame, width, bit, height, flip)
            }
            _ => unreachable!(),
        }
    }
}
