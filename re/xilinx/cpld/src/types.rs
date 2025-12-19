use bincode::{Decode, Encode};
use enum_map::Enum;
use prjcombine_types::cpld::{IoCoord, MacrocellCoord, MacrocellId, ProductTermId};
use prjcombine_entity::id::{EntityIdU8, EntityTag};

macro_rules! entity_id_u8 {
    ($ty:ident, $tag:ident, $prefix:literal) => {
        pub struct $tag;
        impl EntityTag for $tag {
            const PREFIX: &'static str = $prefix;
        }
        pub type $ty = EntityIdU8<$tag>;
    };
}

entity_id_u8!(FbGroupId, FbGroupTag, "BG");
entity_id_u8!(ImuxId, ImuxTag, "IM");
entity_id_u8!(FbnId, FbnTag, "FBN");
entity_id_u8!(ClkPadId, ClkPadTag, "CLKPAD");
entity_id_u8!(FclkId, FclkTag, "FCLK");
entity_id_u8!(OePadId, OePadTag, "OEPAD");
entity_id_u8!(FoeId, FoeTag, "FOE");
entity_id_u8!(BankId, BankTag, "BANK");

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode, Enum)]
pub enum ExportDir {
    Up,
    Down,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode, Enum)]
pub enum Xc9500McPt {
    Clk,
    Oe,
    Rst,
    Set,
    Xor,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode, Enum)]
pub enum Ut {
    Clk,
    Oe,
    Rst,
    Set,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub enum ClkMuxVal {
    Pt,
    Fclk(FclkId),
    Ct(ProductTermId),
    Ut,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub enum SrMuxVal {
    Pt,
    Fsr,
    Ct(ProductTermId),
    Ut,
    Gnd,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub enum RegMode {
    Dff,
    Tff,
    Latch,
    DffCe,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub enum CeMuxVal {
    PtRst,
    PtSet,
    Pt,
    Ct(ProductTermId),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub enum XorMuxVal {
    Gnd,
    Vcc,
    Pt,
    PtInv,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub enum OeMuxVal {
    Gnd,
    Vcc,
    Pt,
    Foe(FoeId),
    Ct(ProductTermId),
    Ut,
    OpenDrain,
    Pullup,
    IsGround,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub enum TermMode {
    Pullup,
    Keeper,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub enum IBufMode {
    Plain,
    Schmitt,
    UseVref,
    IsVref,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub enum OeMode {
    Gnd,
    Vcc,
    McOe,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum ImuxInput {
    Ibuf(IoCoord),
    Fbk(MacrocellId),
    Mc(MacrocellCoord),
    Pup,
    Uim,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum Slew {
    Slow,
    Fast,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum FoeMuxVal {
    Ibuf,
    IbufInv,
    Mc,
}

impl core::ops::Not for ExportDir {
    type Output = ExportDir;

    fn not(self) -> Self::Output {
        match self {
            ExportDir::Up => ExportDir::Down,
            ExportDir::Down => ExportDir::Up,
        }
    }
}
