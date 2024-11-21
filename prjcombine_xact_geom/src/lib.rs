use std::{collections::BTreeMap, error::Error, fs::File, path::Path};

use prjcombine_int::{db::IntDb, grid::ExpandedGrid};
use prjcombine_virtex_bitstream::BitstreamGeom;
use prjcombine_xact_naming::{db::NamingDb, grid::ExpandedGridNaming};
use serde::{Deserialize, Serialize};
use unnamed_entity::{entity_id, EntityVec};

entity_id! {
    pub id GridId usize;
    pub id BondId usize;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Grid {
    Xc4000(prjcombine_xc4000::grid::Grid),
    Xc5200(prjcombine_xc5200::grid::Grid),
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Bond {
    Xc4000(prjcombine_xc4000::bond::Bond),
    Xc5200(prjcombine_xc5200::bond::Bond),
}

pub enum ExpandedBond<'a> {
    Xc4000(prjcombine_xc4000::bond::ExpandedBond<'a>),
    Xc5200(prjcombine_xc5200::bond::ExpandedBond<'a>),
}

impl Bond {
    pub fn expand(&self) -> ExpandedBond {
        match self {
            Bond::Xc4000(bond) => ExpandedBond::Xc4000(bond.expand()),
            Bond::Xc5200(bond) => ExpandedBond::Xc5200(bond.expand()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeomDb {
    pub grids: EntityVec<GridId, Grid>,
    pub bonds: EntityVec<BondId, Bond>,
    pub devices: Vec<Device>,
    pub ints: BTreeMap<String, IntDb>,
    pub namings: BTreeMap<String, NamingDb>,
}

pub enum ExpandedDevice<'a> {
    Xc4000(prjcombine_xc4000::expanded::ExpandedDevice<'a>),
    Xc5200(prjcombine_xc5200::expanded::ExpandedDevice<'a>),
}

impl<'a> ExpandedDevice<'a> {
    pub fn egrid(&self) -> &ExpandedGrid<'a> {
        match self {
            ExpandedDevice::Xc4000(edev) => &edev.egrid,
            ExpandedDevice::Xc5200(edev) => &edev.egrid,
        }
    }

    pub fn bs_geom(&self) -> &BitstreamGeom {
        match self {
            ExpandedDevice::Xc4000(edev) => &edev.bs_geom,
            ExpandedDevice::Xc5200(edev) => &edev.bs_geom,
        }
    }
}

pub enum ExpandedNamedDevice<'a> {
    Xc4000(prjcombine_xc4000_xact::ExpandedNamedDevice<'a>),
    Xc5200(prjcombine_xc5200_xact::ExpandedNamedDevice<'a>),
}

impl<'a> ExpandedNamedDevice<'a> {
    pub fn ngrid(&self) -> &ExpandedGridNaming<'a> {
        match self {
            ExpandedNamedDevice::Xc4000(endev) => &endev.ngrid,
            ExpandedNamedDevice::Xc5200(endev) => &endev.ngrid,
        }
    }
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
        match grid {
            Grid::Xc4000(grid) => {
                let intdb = &self.ints[match grid.kind {
                    prjcombine_xc4000::grid::GridKind::Xc4000 => "xc4000",
                    prjcombine_xc4000::grid::GridKind::Xc4000A => "xc4000a",
                    prjcombine_xc4000::grid::GridKind::Xc4000H => "xc4000h",
                    prjcombine_xc4000::grid::GridKind::Xc4000E => "xc4000e",
                    prjcombine_xc4000::grid::GridKind::Xc4000Ex => "xc4000ex",
                    prjcombine_xc4000::grid::GridKind::Xc4000Xla => "xc4000xla",
                    prjcombine_xc4000::grid::GridKind::Xc4000Xv => "xc4000xv",
                    prjcombine_xc4000::grid::GridKind::SpartanXl => "spartanxl",
                }];
                ExpandedDevice::Xc4000(grid.expand_grid(intdb))
            }
            Grid::Xc5200(grid) => {
                let intdb = &self.ints["xc5200"];
                ExpandedDevice::Xc5200(grid.expand_grid(intdb))
            }
        }
    }

    pub fn name<'a>(&'a self, _dev: &Device, edev: &'a ExpandedDevice) -> ExpandedNamedDevice<'a> {
        match edev {
            ExpandedDevice::Xc4000(edev) => {
                let ndb = &self.namings[match edev.grid.kind {
                    prjcombine_xc4000::grid::GridKind::Xc4000 => "xc4000",
                    prjcombine_xc4000::grid::GridKind::Xc4000A => "xc4000a",
                    prjcombine_xc4000::grid::GridKind::Xc4000H => "xc4000h",
                    prjcombine_xc4000::grid::GridKind::Xc4000E => "xc4000e",
                    prjcombine_xc4000::grid::GridKind::Xc4000Ex => "xc4000ex",
                    prjcombine_xc4000::grid::GridKind::Xc4000Xla => "xc4000xla",
                    prjcombine_xc4000::grid::GridKind::Xc4000Xv => "xc4000xv",
                    prjcombine_xc4000::grid::GridKind::SpartanXl => "spartanxl",
                }];
                ExpandedNamedDevice::Xc4000(prjcombine_xc4000_xact::name_device(edev, ndb))
            }
            ExpandedDevice::Xc5200(edev) => {
                let ndb = &self.namings["xc5200"];
                ExpandedNamedDevice::Xc5200(prjcombine_xc5200_xact::name_device(edev, ndb))
            }
        }
    }
}
