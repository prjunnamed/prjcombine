use core::fmt::Debug;

use bincode::{Decode, Encode};
use unnamed_entity::{
    EntityId,
    id::{EntityIdU8, EntityTag},
};

pub struct ClusterTag;
pub struct BlockTag;
pub struct MacrocellTag;
pub struct ProductTermTag;
pub struct IpadTag;

impl EntityTag for ClusterTag {
    const PREFIX: &'static str = "C";
}
impl EntityTag for BlockTag {
    const PREFIX: &'static str = "B";
}
impl EntityTag for MacrocellTag {
    const PREFIX: &'static str = "MC";
}
impl EntityTag for ProductTermTag {
    const PREFIX: &'static str = "PT";
}
impl EntityTag for IpadTag {
    const PREFIX: &'static str = "IPAD";
}

pub type ClusterId = EntityIdU8<ClusterTag>;
pub type BlockId = EntityIdU8<BlockTag>;
pub type MacrocellId = EntityIdU8<MacrocellTag>;
pub type ProductTermId = EntityIdU8<ProductTermTag>;
pub type IpadId = EntityIdU8<IpadTag>;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode)]
pub struct MacrocellCoord {
    pub cluster: ClusterId,
    pub block: BlockId,
    pub macrocell: MacrocellId,
}

impl MacrocellCoord {
    pub fn simple(block: BlockId, macrocell: MacrocellId) -> Self {
        Self {
            cluster: ClusterId::from_idx(0),
            block,
            macrocell,
        }
    }
}

impl Debug for MacrocellCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl std::fmt::Display for MacrocellCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", self.cluster, self.block, self.macrocell)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum IoCoord {
    Ipad(IpadId),
    Macrocell(MacrocellCoord),
}

impl Debug for IoCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl std::fmt::Display for IoCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoCoord::Ipad(ipad) => write!(f, "{ipad}"),
            IoCoord::Macrocell(mc) => write!(f, "IOB_{mc}"),
        }
    }
}
