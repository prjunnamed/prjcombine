use core::fmt::Debug;
use std::{
    collections::{BTreeMap, btree_map},
    error::Error,
    fs::File,
    path::Path,
};

use bincode::{Decode, Encode};
use itertools::*;
use prjcombine_entity::id::{EntityIdU16, EntityTag, EntityTagArith};

use crate::bitvec::BitVec;

pub struct BitRectTag;
impl EntityTag for BitRectTag {
    const PREFIX: &'static str = "R";
}

pub struct RectFrameTag;
impl EntityTag for RectFrameTag {
    const PREFIX: &'static str = "F";
}
impl EntityTagArith for RectFrameTag {}

pub struct RectBitTag;
impl EntityTag for RectBitTag {
    const PREFIX: &'static str = "B";
}
impl EntityTagArith for RectBitTag {}

pub type BitRectId = EntityIdU16<BitRectTag>;
pub type RectFrameId = EntityIdU16<RectFrameTag>;
pub type RectBitId = EntityIdU16<RectBitTag>;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub struct TileBit {
    pub rect: BitRectId,
    pub frame: RectFrameId,
    pub bit: RectBitId,
}

impl TileBit {
    pub const DUMMY: TileBit = TileBit::new(0xdead, 0xdead, 0xdead);

    pub const fn new(rect: usize, frame: usize, bit: usize) -> Self {
        Self {
            rect: BitRectId::from_idx_const(rect),
            frame: RectFrameId::from_idx_const(frame),
            bit: RectBitId::from_idx_const(bit),
        }
    }

    pub const fn pos(self) -> PolTileBit {
        PolTileBit {
            bit: self,
            inv: false,
        }
    }

    pub const fn neg(self) -> PolTileBit {
        PolTileBit {
            bit: self,
            inv: true,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub struct PolTileBit {
    pub bit: TileBit,
    pub inv: bool,
}

impl PolTileBit {
    pub const DUMMY: PolTileBit = TileBit::DUMMY.pos();
}

impl core::fmt::Debug for TileBit {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}", self.rect, self.frame, self.bit)
    }
}

impl core::fmt::Debug for PolTileBit {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.inv {
            write!(f, "~{:?}", self.bit)
        } else {
            write!(f, "{:?}", self.bit)
        }
    }
}

impl core::ops::Not for PolTileBit {
    type Output = PolTileBit;

    fn not(self) -> Self::Output {
        PolTileBit {
            bit: self.bit,
            inv: !self.inv,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Encode, Decode)]
pub struct BitRectGeometry {
    pub frames: usize,
    pub bits: usize,
    pub orientation: FrameOrientation,
    pub rev_frames: bool,
    pub rev_bits: bool,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Encode, Decode)]
pub enum FrameOrientation {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub struct EnumData<K: Ord> {
    pub bits: Vec<TileBit>,
    pub values: BTreeMap<K, BitVec>,
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode, Default)]
pub struct Tile {
    pub items: BTreeMap<String, TileItem>,
}

impl Tile {
    pub fn new() -> Self {
        Self {
            items: BTreeMap::new(),
        }
    }

    pub fn merge(&mut self, other: &Tile, neutral: impl Fn(TileBit) -> bool) {
        if self == other {
            return;
        }
        for (k, v) in &other.items {
            match self.items.entry(k.clone()) {
                btree_map::Entry::Vacant(e) => {
                    e.insert(v.clone());
                }
                btree_map::Entry::Occupied(mut e) => {
                    e.get_mut().merge(v, &neutral);
                }
            }
        }
    }

    pub fn insert(
        &mut self,
        name: impl Into<String>,
        item: TileItem,
        neutral: impl Fn(TileBit) -> bool,
    ) {
        match self.items.entry(name.into()) {
            btree_map::Entry::Vacant(e) => {
                e.insert(item);
            }
            btree_map::Entry::Occupied(mut e) => {
                e.get_mut().merge(&item, neutral);
            }
        }
    }

    pub fn dump(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        for (key, item) in &self.items {
            write!(o, "\t{key}:")?;
            for bit in item.bits.iter().rev() {
                write!(o, " {bit:?}")?;
            }
            match item.kind {
                TileItemKind::Enum { ref values } => {
                    writeln!(o,)?;
                    for (key, val) in values {
                        write!(o, "\t\t")?;
                        for bit in val.iter().rev() {
                            write!(o, "{}", usize::from(bit))?;
                        }
                        writeln!(o, ": {key}")?;
                    }
                }
                TileItemKind::BitVec { ref invert } => {
                    write!(o, " inv ")?;
                    for bit in invert.iter().rev() {
                        write!(o, "{}", usize::from(bit))?;
                    }
                    writeln!(o)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct TileItem {
    pub bits: Vec<TileBit>,
    pub kind: TileItemKind,
}

impl TileItem {
    pub fn merge(&mut self, other: &TileItem, neutral: impl Fn(TileBit) -> bool) {
        if self == other {
            return;
        }
        let TileItemKind::Enum { values: av } = &mut self.kind else {
            panic!("weird merge: {self:?} {other:?}");
        };
        let TileItemKind::Enum { values: bv } = &other.kind else {
            unreachable!()
        };
        let mut bits = self.bits.clone();
        for &bit in &other.bits {
            if !bits.contains(&bit) {
                bits.push(bit);
            }
        }
        bits.sort();
        let bit_map_a: Vec<_> = bits
            .iter()
            .map(|&x| self.bits.iter().find_position(|&&y| x == y).map(|x| x.0))
            .collect();
        let bit_map_b: Vec<_> = bits
            .iter()
            .map(|&x| other.bits.iter().find_position(|&&y| x == y).map(|x| x.0))
            .collect();
        self.bits = bits;
        for val in av.values_mut() {
            *val = bit_map_a
                .iter()
                .enumerate()
                .map(|(i, &x)| match x {
                    Some(idx) => val[idx],
                    None => neutral(self.bits[i]),
                })
                .collect();
        }
        for (key, val) in bv {
            let val: BitVec = bit_map_b
                .iter()
                .enumerate()
                .map(|(i, &x)| match x {
                    Some(idx) => val[idx],
                    None => neutral(self.bits[i]),
                })
                .collect();

            match av.entry(key.clone()) {
                btree_map::Entry::Vacant(e) => {
                    e.insert(val);
                }
                btree_map::Entry::Occupied(e) => {
                    if *e.get() != val {
                        panic!("tile merge failed at {key}: {cv} vs {val:?}", cv = e.get());
                    }
                }
            }
        }
    }

    pub fn from_bit_inv(bit: TileBit, invert: bool) -> Self {
        Self {
            bits: vec![bit],
            kind: TileItemKind::BitVec {
                invert: BitVec::from_iter([invert]),
            },
        }
    }

    pub fn from_bitvec_inv(bits: Vec<TileBit>, invert: bool) -> Self {
        let invert = BitVec::repeat(invert, bits.len());
        Self {
            bits,
            kind: TileItemKind::BitVec { invert },
        }
    }

    pub fn as_bitvec(&self) -> Vec<PolTileBit> {
        let TileItemKind::BitVec { ref invert } = self.kind else {
            unreachable!()
        };
        self.bits
            .iter()
            .zip(invert)
            .map(|(&bit, inv)| PolTileBit { bit, inv })
            .collect()
    }

    pub fn as_bit(&self) -> PolTileBit {
        assert_eq!(self.bits.len(), 1);
        let TileItemKind::BitVec { ref invert } = self.kind else {
            unreachable!()
        };
        PolTileBit {
            bit: self.bits[0],
            inv: invert[0],
        }
    }
}

impl From<PolTileBit> for TileItem {
    fn from(value: PolTileBit) -> Self {
        Self {
            bits: vec![value.bit],
            kind: TileItemKind::BitVec {
                invert: BitVec::from_iter([value.inv]),
            },
        }
    }
}

impl From<&[PolTileBit]> for TileItem {
    fn from(value: &[PolTileBit]) -> Self {
        Self {
            bits: value.iter().map(|pbit| pbit.bit).collect(),
            kind: TileItemKind::BitVec {
                invert: value.iter().map(|pbit| pbit.inv).collect(),
            },
        }
    }
}

impl From<Vec<PolTileBit>> for TileItem {
    fn from(value: Vec<PolTileBit>) -> Self {
        TileItem::from(value.as_slice())
    }
}

impl From<EnumData<String>> for TileItem {
    fn from(value: EnumData<String>) -> Self {
        TileItem {
            bits: value.bits,
            kind: TileItemKind::Enum {
                values: value.values,
            },
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Encode, Decode)]
pub enum TileItemKind {
    Enum { values: BTreeMap<String, BitVec> },
    BitVec { invert: BitVec },
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum DbValue {
    String(String),
    BitVec(BitVec),
    Int(u32),
}

impl std::fmt::Display for DbValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbValue::String(s) => write!(f, "\"{s}\""),
            DbValue::BitVec(v) => write!(f, "0b{v}"),
            DbValue::Int(v) => write!(f, "{v}"),
        }
    }
}

impl From<BitVec> for DbValue {
    fn from(value: BitVec) -> Self {
        Self::BitVec(value)
    }
}

impl<const N: usize> From<[bool; N]> for DbValue {
    fn from(value: [bool; N]) -> Self {
        Self::BitVec(BitVec::from_iter(value))
    }
}

impl From<String> for DbValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<u32> for DbValue {
    fn from(value: u32) -> Self {
        Self::Int(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct BsData {
    pub tiles: BTreeMap<String, Tile>,
    pub device_data: BTreeMap<String, BTreeMap<String, DbValue>>,
    pub misc_data: BTreeMap<String, DbValue>,
}

impl BsData {
    pub fn new() -> Self {
        Self {
            tiles: BTreeMap::new(),
            device_data: BTreeMap::new(),
            misc_data: BTreeMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty() && self.device_data.is_empty() && self.misc_data.is_empty()
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let f = File::open(path)?;
        let mut cf = zstd::stream::Decoder::new(f)?;
        let config = bincode::config::standard();
        Ok(bincode::decode_from_std_read(&mut cf, config)?)
    }

    pub fn to_file(&self, path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
        let f = File::create(path)?;
        let mut cf = zstd::stream::Encoder::new(f, 9)?;
        let config = bincode::config::standard();
        bincode::encode_into_std_write(self, &mut cf, config)?;
        cf.finish()?;
        Ok(())
    }

    pub fn insert(
        &mut self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        name: impl Into<String>,
        item: TileItem,
    ) {
        let name = format!("{}:{}", bel.into(), name.into());
        let tile = self.tiles.entry(tile.into()).or_default();
        tile.insert(name, item, |_| false);
    }

    #[track_caller]
    pub fn item(&self, tile: &str, bel: &str, attr: &str) -> &TileItem {
        &self.tiles[tile].items[&format!("{bel}:{attr}")]
    }

    pub fn insert_misc_data(&mut self, key: impl Into<String>, val: impl Into<DbValue>) {
        let key = key.into();
        let val = val.into();
        match self.misc_data.entry(key) {
            btree_map::Entry::Vacant(e) => {
                e.insert(val);
            }
            btree_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), val);
            }
        }
    }

    pub fn insert_device_data(
        &mut self,
        device: &str,
        key: impl Into<String>,
        val: impl Into<DbValue>,
    ) {
        let dev = self.device_data.entry(device.into()).or_default();
        let key = key.into();
        let val = val.into();
        match dev.entry(key) {
            btree_map::Entry::Vacant(e) => {
                e.insert(val);
            }
            btree_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), val);
            }
        }
    }

    pub fn dump(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        for (tname, tile) in &self.tiles {
            writeln!(o, "bstile {tname} {{")?;
            tile.dump(o)?;
            writeln!(o, "}}")?;
            writeln!(o)?;
        }
        for (name, value) in &self.misc_data {
            writeln!(o, "misc_data {name} = {value};")?;
        }
        if !self.misc_data.is_empty() {
            writeln!(o)?;
        }
        for (name, data) in &self.device_data {
            writeln!(o, "device_data {name} {{")?;
            for (name, value) in data {
                writeln!(o, "\t{name} = {value};")?;
            }
            writeln!(o, "}}")?;
            writeln!(o)?;
        }
        Ok(())
    }
}

impl Default for BsData {
    fn default() -> Self {
        Self::new()
    }
}
