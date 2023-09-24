use enum_map::Enum;
use serde::{Deserialize, Serialize};
use unnamed_entity::entity_id;

entity_id! {
    pub id FbId u8;
    pub id FbGroupId u8;
    pub id FbMcId u8;
    pub id IpadId u8;
    pub id ImuxId u8;
    pub id PTermId u8;
    pub id FbnId u8;
    pub id ClkPadId u8;
    pub id FclkId u8;
    pub id OePadId u8;
    pub id FoeId u8;
    pub id BankId u8;
}

pub type McId = (FbId, FbMcId);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum IoId {
    Ipad(IpadId),
    Mc(McId),
}

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize, Enum,
)]
pub enum ExportDir {
    Up,
    Down,
}

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize, Enum,
)]
pub enum Xc9500McPt {
    Clk,
    Oe,
    Rst,
    Set,
    Xor,
}

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize, Enum,
)]
pub enum Ut {
    Clk,
    Oe,
    Rst,
    Set,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ClkMuxVal {
    Pt,
    Fclk(FclkId),
    Ct(PTermId),
    Ut,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum SrMuxVal {
    Pt,
    Fsr,
    Ct(PTermId),
    Ut,
    Gnd,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum RegMode {
    Dff,
    Tff,
    Latch,
    DffCe,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum CeMuxVal {
    PtRst,
    PtSet,
    Pt,
    Ct(PTermId),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum XorMuxVal {
    Gnd,
    Vcc,
    Pt,
    PtInv,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum OeMuxVal {
    Gnd,
    Vcc,
    Pt,
    Foe(FoeId),
    Ct(PTermId),
    Ut,
    OpenDrain,
    Pullup,
    IsGround,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum TermMode {
    Pullup,
    Keeper,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum IBufMode {
    Plain,
    Schmitt,
    UseVref,
    IsVref,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum OeMode {
    Gnd,
    Vcc,
    McOe,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ImuxInput {
    Ibuf(IoId),
    Fbk(FbMcId),
    Mc(McId),
    Pup,
    Uim,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Slew {
    Slow,
    Fast,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
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
