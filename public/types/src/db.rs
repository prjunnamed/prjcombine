use bincode::{Decode, Encode};
use jzon::JsonValue;
use unnamed_entity::{id::{EntityIdU16, EntityIdU8, EntityTag}, EntityId};

pub struct ChipTag;
pub struct SpeedTag;
pub struct BondTag;
pub struct InterposerTag;
pub struct DevBondTag;
pub struct DevSpeedTag;

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
impl EntityTag for DevBondTag {
    const PREFIX: &'static str = "DEVBOND";
}
impl EntityTag for DevSpeedTag {
    const PREFIX: &'static str = "DEVSPEED";
}

pub type ChipId = EntityIdU16<ChipTag>;
pub type SpeedId = EntityIdU16<SpeedTag>;
pub type BondId = EntityIdU16<BondTag>;
pub type InterposerId = EntityIdU16<InterposerTag>;
pub type DevBondId = EntityIdU8<DevBondTag>;
pub type DevSpeedId = EntityIdU8<DevSpeedTag>;

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct DeviceCombo {
    pub devbond: DevBondId,
    pub speed: DevSpeedId,
}

impl From<&DeviceCombo> for JsonValue {
    fn from(combo: &DeviceCombo) -> Self {
        jzon::object! {
            devbond: combo.devbond.to_idx(),
            speed: combo.speed.to_idx(),
        }
    }
}
