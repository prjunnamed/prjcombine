use unnamed_entity::id::{EntityIdU16, EntityTag};

pub struct ChipTag;
pub struct SpeedTag;
pub struct BondTag;
pub struct InterposerTag;
impl EntityTag for ChipTag {
    const PREFIX: &'static str = "CHIP";
}
impl EntityTag for SpeedTag {
    const PREFIX: &'static str = "SPEED";
}
impl EntityTag for BondTag {
    const PREFIX: &'static str = "BOND";
}
impl EntityTag for InterposerTag {
    const PREFIX: &'static str = "INTERPOSER";
}
pub type ChipId = EntityIdU16<ChipTag>;
pub type SpeedId = EntityIdU16<SpeedTag>;
pub type BondId = EntityIdU16<BondTag>;
pub type InterposerId = EntityIdU16<InterposerTag>;
