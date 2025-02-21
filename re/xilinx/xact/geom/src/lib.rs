use std::{collections::BTreeMap, error::Error, fs::File, path::Path};

use prjcombine_interconnect::db::IntDb;
use prjcombine_re_xilinx_xact_naming::db::NamingDb;
use prjcombine_re_xilinx_xact_xc2000::ExpandedNamedDevice;
use prjcombine_xc2000::{
    bond::Bond,
    chip::{Chip, ChipKind},
    expanded::ExpandedDevice,
};
use serde::{Deserialize, Serialize};
use unnamed_entity::{EntityVec, entity_id};

entity_id! {
    pub id GridId usize;
    pub id BondId usize;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceBond {
    pub name: String,
    pub bond: BondId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Device {
    pub name: String,
    pub grid: GridId,
    pub bonds: Vec<DeviceBond>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeomDb {
    pub grids: EntityVec<GridId, Chip>,
    pub bonds: EntityVec<BondId, Bond>,
    pub devices: Vec<Device>,
    pub ints: BTreeMap<String, IntDb>,
    pub namings: BTreeMap<String, NamingDb>,
}

impl GeomDb {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let f = File::open(path)?;
        let cf = zstd::stream::Decoder::new(f)?;
        Ok(bincode::deserialize_from(cf)?)
    }

    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let f = File::create(path)?;
        let mut cf = zstd::stream::Encoder::new(f, 9)?;
        bincode::serialize_into(&mut cf, self)?;
        cf.finish()?;
        Ok(())
    }

    pub fn expand_grid(&self, dev: &Device) -> ExpandedDevice {
        let grid = &self.grids[dev.grid];
        let intdb = &self.ints[match grid.kind {
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
        grid.expand_grid(intdb)
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
