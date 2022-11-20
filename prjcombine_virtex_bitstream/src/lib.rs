#![allow(clippy::collapsible_else_if)]

use bitvec::prelude::*;
use enum_map::{Enum, EnumMap};
use prjcombine_entity::EntityVec;
use prjcombine_int::grid::DieId;
use std::collections::HashMap;

mod packet;
mod parse;
pub use parse::parse;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Enum)]
pub enum Reg {
    Idcode,
    Ctl0,
    Ctl1,
    Cor0,
    Cor1,
    Cor2,
    Key,
    WbStar,
    Timer,
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
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum DeviceKind {
    Virtex,
    Virtex2,
    Spartan3A,
    Spartan6,
    Virtex4,
    Virtex5,
    Virtex6,
    Series7,
    Ultrascale,
    UltrascalePlus,
    Versal,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum BitstreamMode {
    Plain,
    Debug,
    Compress,
    PerFrameCrc,
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
    pub bram_cols: usize,
    pub bram_regs: usize,
    pub iob_frame_len: usize,
}

#[derive(Clone, Debug)]
pub struct Bitstream {
    pub kind: DeviceKind,
    pub die: EntityVec<DieId, DieBitstream>,
}

#[derive(Clone, Debug)]
pub struct DieBitstream {
    pub regs: EnumMap<Reg, Option<u32>>,
    pub mode: BitstreamMode,
    pub frame_len: usize,
    pub frame_data: BitVec,
    pub frame_info: Vec<FrameInfo>,
    pub frame_present: BitVec,
    // spartan 6 only
    pub bram_data: BitVec,
    pub bram_cols: usize,
    pub bram_present: BitVec,
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
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct FrameAddr {
    pub typ: u32,
    pub region: i32,
    pub major: u32,
    pub minor: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct FrameInfo {
    pub addr: FrameAddr,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum BitPos {
    Reg(DieId, Reg, usize),
    // die, frame, bit
    Main(DieId, usize, usize),
    Fixup(DieId, usize, usize),
    Bram(DieId, usize, usize),
    Iob(DieId, usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum BitTile {
    Reg(DieId, Reg),
    // die, frame, width, bit, height, flip
    Main(DieId, usize, usize, usize, usize, bool),
    Fixup(DieId, usize, usize, usize, usize, bool),
    // single whole frame
    Bram(DieId, usize),
    // bit, width
    Iob(DieId, usize, usize),
}

impl BitTile {
    pub fn xlat_pos(&self, bit: BitPos) -> Option<(usize, usize)> {
        match (*self, bit) {
            (BitTile::Reg(die, reg), BitPos::Reg(bdie, breg, pos))
                if bdie == die && breg == reg =>
            {
                Some((0, pos))
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
}
