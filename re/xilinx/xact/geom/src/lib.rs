use std::{collections::BTreeMap, error::Error, fs::File, path::Path};

use bincode::{Decode, Encode};
use prjcombine_interconnect::db::IntDb;
use prjcombine_re_xilinx_xact_naming::db::NamingDb;
use prjcombine_re_xilinx_xact_xc2000::ExpandedNamedDevice;
use prjcombine_types::db::{BondId, ChipId};
use prjcombine_xc2000::{
    bond::Bond,
    chip::{Chip, ChipKind},
    expanded::ExpandedDevice,
};
use unnamed_entity::EntityVec;

#[derive(Clone, Debug, Encode, Decode)]
pub struct DeviceBond {
    pub name: String,
    pub bond: BondId,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct Device {
    pub name: String,
    pub chip: ChipId,
    pub bonds: Vec<DeviceBond>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct GeomDb {
    pub chips: EntityVec<ChipId, Chip>,
    pub bonds: EntityVec<BondId, Bond>,
    pub devices: Vec<Device>,
    pub ints: BTreeMap<String, IntDb>,
    pub namings: BTreeMap<String, NamingDb>,
}

impl GeomDb {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let f = File::open(path)?;
        let mut cf = zstd::stream::Decoder::new(f)?;
        let config = bincode::config::standard();
        Ok(bincode::decode_from_std_read(&mut cf, config)?)
    }

    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let f = File::create(path)?;
        let mut cf = zstd::stream::Encoder::new(f, 9)?;
        let config = bincode::config::standard();
        bincode::encode_into_std_write(self, &mut cf, config)?;
        cf.finish()?;
        Ok(())
    }

    pub fn expand_grid(&self, dev: &Device) -> ExpandedDevice<'_> {
        let chip = &self.chips[dev.chip];
        let intdb = &self.ints[match chip.kind {
            ChipKind::Xc2000 => "xc2000",
            ChipKind::Xc3000 => "xc3000",
            ChipKind::Xc3000A => "xc3000a",
            ChipKind::Xc4000 => "xc4000",
            ChipKind::Xc4000A => "xc4000a",
            ChipKind::Xc4000H => "xc4000h",
            ChipKind::Xc4000E => "xc4000e",
            ChipKind::Xc4000Ex => "xc4000ex",
            ChipKind::Xc4000Xla => "xc4000xla",
            ChipKind::Xc4000Xv => "xc4000xv",
            ChipKind::SpartanXl => "spartanxl",
            ChipKind::Xc5200 => "xc5200",
        }];
        chip.expand_grid(intdb)
    }

    pub fn name<'a>(&'a self, _dev: &Device, edev: &'a ExpandedDevice) -> ExpandedNamedDevice<'a> {
        let ndb = &self.namings[match edev.chip.kind {
            ChipKind::Xc2000 => "xc2000",
            ChipKind::Xc3000 => "xc3000",
            ChipKind::Xc3000A => "xc3000a",
            ChipKind::Xc4000 => "xc4000",
            ChipKind::Xc4000A => "xc4000a",
            ChipKind::Xc4000H => "xc4000h",
            ChipKind::Xc4000E => "xc4000e",
            ChipKind::Xc4000Ex => "xc4000ex",
            ChipKind::Xc4000Xla => "xc4000xla",
            ChipKind::Xc4000Xv => "xc4000xv",
            ChipKind::SpartanXl => "spartanxl",
            ChipKind::Xc5200 => "xc5200",
        }];
        prjcombine_re_xilinx_xact_xc2000::name_device(edev, ndb)
    }
}
