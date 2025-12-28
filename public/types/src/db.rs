use bincode::{Decode, Encode};
use prjcombine_entity::id::{EntityIdU8, EntityIdU16, EntityTag};

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

#[derive(Copy, Clone, Debug)]
pub struct DumpFlags {
    pub intdb: bool,
    pub chip: bool,
    pub bond: bool,
    pub speed: bool,
    pub device: bool,
    pub bsdata: bool,
}

impl DumpFlags {
    pub fn all() -> Self {
        DumpFlags {
            intdb: true,
            chip: true,
            bond: true,
            device: true,
            speed: true,
            bsdata: true,
        }
    }
}
