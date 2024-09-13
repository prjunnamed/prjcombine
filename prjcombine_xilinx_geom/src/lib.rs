use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{DieId, ExpandedGrid};
use prjcombine_virtex_bitstream::BitstreamGeom;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs::File;
use std::path::Path;
use unnamed_entity::{entity_id, EntityVec};

entity_id! {
    pub id GridId usize;
    pub id BondId usize;
    pub id DevBondId usize;
    pub id DevSpeedId usize;
    pub id DeviceNamingId usize;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Grid {
    Xc4k(prjcombine_xc4k::grid::Grid),
    Xc5200(prjcombine_xc5200::grid::Grid),
    Virtex(prjcombine_virtex::grid::Grid),
    Virtex2(prjcombine_virtex2::grid::Grid),
    Spartan6(prjcombine_spartan6::grid::Grid),
    Virtex4(prjcombine_virtex4::grid::Grid),
    Ultrascale(prjcombine_ultrascale::grid::Grid),
    Versal(prjcombine_versal::grid::Grid),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceBond {
    pub name: String,
    pub bond: BondId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DisabledPart {
    Virtex(prjcombine_virtex::grid::DisabledPart),
    Spartan6(prjcombine_spartan6::grid::DisabledPart),
    Virtex4(prjcombine_virtex4::grid::DisabledPart),
    Ultrascale(prjcombine_ultrascale::grid::DisabledPart),
    Versal(prjcombine_versal::grid::DisabledPart),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceCombo {
    pub name: String,
    pub devbond_idx: DevBondId,
    pub speed_idx: DevSpeedId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ExtraDie {
    Virtex4(prjcombine_virtex4::grid::ExtraDie),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Device {
    pub name: String,
    pub grids: EntityVec<DieId, GridId>,
    pub grid_master: DieId,
    pub extras: Vec<ExtraDie>,
    pub bonds: EntityVec<DevBondId, DeviceBond>,
    pub speeds: EntityVec<DevSpeedId, String>,
    // valid (bond, speed) pairs
    pub combos: Vec<DeviceCombo>,
    pub disabled: BTreeSet<DisabledPart>,
    pub naming: DeviceNamingId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Bond {
    Xc4k(prjcombine_xc4k::bond::Bond),
    Xc5200(prjcombine_xc5200::bond::Bond),
    Virtex(prjcombine_virtex::bond::Bond),
    Virtex2(prjcombine_virtex2::bond::Bond),
    Spartan6(prjcombine_spartan6::bond::Bond),
    Virtex4(prjcombine_virtex4::bond::Bond),
    Ultrascale(prjcombine_ultrascale::bond::Bond),
    Versal(prjcombine_versal::bond::Bond),
}

pub enum ExpandedBond<'a> {
    Xc4k(prjcombine_xc4k::bond::ExpandedBond<'a>),
    Xc5200(prjcombine_xc5200::bond::ExpandedBond<'a>),
    Virtex(prjcombine_virtex::bond::ExpandedBond<'a>),
    Virtex2(prjcombine_virtex2::bond::ExpandedBond<'a>),
    Spartan6(prjcombine_spartan6::bond::ExpandedBond<'a>),
    Virtex4(prjcombine_virtex4::bond::ExpandedBond<'a>),
    Ultrascale(prjcombine_ultrascale::bond::ExpandedBond<'a>),
    Versal(prjcombine_versal::bond::ExpandedBond<'a>),
}

impl Bond {
    pub fn expand(&self) -> ExpandedBond {
        match self {
            Bond::Xc4k(bond) => ExpandedBond::Xc4k(bond.expand()),
            Bond::Xc5200(bond) => ExpandedBond::Xc5200(bond.expand()),
            Bond::Virtex(bond) => ExpandedBond::Virtex(bond.expand()),
            Bond::Virtex2(bond) => ExpandedBond::Virtex2(bond.expand()),
            Bond::Spartan6(bond) => ExpandedBond::Spartan6(bond.expand()),
            Bond::Virtex4(bond) => ExpandedBond::Virtex4(bond.expand()),
            Bond::Ultrascale(bond) => ExpandedBond::Ultrascale(bond.expand()),
            Bond::Versal(bond) => ExpandedBond::Versal(bond.expand()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeomDb {
    pub grids: EntityVec<GridId, Grid>,
    pub bonds: EntityVec<BondId, Bond>,
    pub dev_namings: EntityVec<DeviceNamingId, DeviceNaming>,
    pub devices: Vec<Device>,
    pub ints: BTreeMap<String, IntDb>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DeviceNaming {
    Dummy,
    Ultrascale(prjcombine_ultrascale::grid::DeviceNaming),
    Versal(prjcombine_versal::naming::DeviceNaming),
}

pub enum ExpandedDevice<'a> {
    Xc4k(prjcombine_xc4k::expanded::ExpandedDevice<'a>),
    Xc5200(prjcombine_xc5200::expanded::ExpandedDevice<'a>),
    Virtex(prjcombine_virtex::expanded::ExpandedDevice<'a>),
    Virtex2(prjcombine_virtex2::expanded::ExpandedDevice<'a>),
    Spartan6(prjcombine_spartan6::expanded::ExpandedDevice<'a>),
    Virtex4(prjcombine_virtex4::expanded::ExpandedDevice<'a>),
    Ultrascale(prjcombine_ultrascale::expanded::ExpandedDevice<'a>),
    Versal(prjcombine_versal::expanded::ExpandedDevice<'a>),
}

impl<'a> ExpandedDevice<'a> {
    pub fn egrid(&self) -> &ExpandedGrid<'a> {
        match self {
            ExpandedDevice::Xc4k(edev) => &edev.egrid,
            ExpandedDevice::Xc5200(edev) => &edev.egrid,
            ExpandedDevice::Virtex(edev) => &edev.egrid,
            ExpandedDevice::Virtex2(edev) => &edev.egrid,
            ExpandedDevice::Spartan6(edev) => &edev.egrid,
            ExpandedDevice::Virtex4(edev) => &edev.egrid,
            ExpandedDevice::Ultrascale(edev) => &edev.egrid,
            ExpandedDevice::Versal(edev) => &edev.egrid,
        }
    }

    pub fn bs_geom(&self) -> &BitstreamGeom {
        match self {
            ExpandedDevice::Xc4k(_) => todo!(),
            ExpandedDevice::Xc5200(edev) => &edev.bs_geom,
            ExpandedDevice::Virtex(edev) => &edev.bs_geom,
            ExpandedDevice::Virtex2(edev) => &edev.bs_geom,
            ExpandedDevice::Spartan6(edev) => &edev.bs_geom,
            ExpandedDevice::Virtex4(edev) => &edev.bs_geom,
            ExpandedDevice::Ultrascale(_) => todo!(),
            ExpandedDevice::Versal(_) => todo!(),
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
        let grid = &self.grids[dev.grids[dev.grid_master]];
        match grid {
            Grid::Xc4k(grid) => {
                let intdb = &self.ints[match grid.kind {
                    prjcombine_xc4k::grid::GridKind::Xc4000 => "xc4000",
                    prjcombine_xc4k::grid::GridKind::Xc4000A => "xc4000a",
                    prjcombine_xc4k::grid::GridKind::Xc4000H => "xc4000h",
                    prjcombine_xc4k::grid::GridKind::Xc4000E => "xc4000e",
                    prjcombine_xc4k::grid::GridKind::Xc4000Ex => "xc4000ex",
                    prjcombine_xc4k::grid::GridKind::Xc4000Xla => "xc4000xla",
                    prjcombine_xc4k::grid::GridKind::Xc4000Xv => "xc4000xv",
                    prjcombine_xc4k::grid::GridKind::SpartanXl => "spartanxl",
                }];
                ExpandedDevice::Xc4k(grid.expand_grid(intdb))
            }
            Grid::Xc5200(grid) => {
                let intdb = &self.ints["xc5200"];
                ExpandedDevice::Xc5200(grid.expand_grid(intdb))
            }
            Grid::Virtex(grid) => {
                let intdb = &self.ints["virtex"];
                let disabled = dev
                    .disabled
                    .iter()
                    .map(|&x| match x {
                        DisabledPart::Virtex(x) => x,
                        _ => unreachable!(),
                    })
                    .collect();
                ExpandedDevice::Virtex(grid.expand_grid(&disabled, intdb))
            }
            Grid::Virtex2(grid) => {
                let intdb = if grid.kind.is_virtex2() {
                    &self.ints["virtex2"]
                } else if grid.kind == prjcombine_virtex2::grid::GridKind::FpgaCore {
                    &self.ints["fpgacore"]
                } else {
                    &self.ints["spartan3"]
                };
                ExpandedDevice::Virtex2(grid.expand_grid(intdb))
            }
            Grid::Spartan6(grid) => {
                let intdb = &self.ints["spartan6"];
                let disabled = dev
                    .disabled
                    .iter()
                    .map(|&x| match x {
                        DisabledPart::Spartan6(x) => x,
                        _ => unreachable!(),
                    })
                    .collect();
                ExpandedDevice::Spartan6(grid.expand_grid(intdb, &disabled))
            }
            Grid::Virtex4(ref grid) => {
                let intdb = &self.ints[match grid.kind {
                    prjcombine_virtex4::grid::GridKind::Virtex4 => "virtex4",
                    prjcombine_virtex4::grid::GridKind::Virtex5 => "virtex5",
                    prjcombine_virtex4::grid::GridKind::Virtex6 => "virtex6",
                    prjcombine_virtex4::grid::GridKind::Virtex7 => "virtex7",
                }];
                let disabled = dev
                    .disabled
                    .iter()
                    .map(|&x| match x {
                        DisabledPart::Virtex4(x) => x,
                        _ => unreachable!(),
                    })
                    .collect();
                let extras: Vec<_> = dev
                    .extras
                    .iter()
                    .map(|&x| match x {
                        ExtraDie::Virtex4(x) => x,
                    })
                    .collect();
                let grids = dev.grids.map_values(|&x| match self.grids[x] {
                    Grid::Virtex4(ref x) => x,
                    _ => unreachable!(),
                });
                ExpandedDevice::Virtex4(prjcombine_virtex4::expand_grid(
                    &grids,
                    dev.grid_master,
                    &extras,
                    &disabled,
                    intdb,
                ))
            }
            Grid::Ultrascale(ref grid) => {
                let intdb = &self.ints[match grid.kind {
                    prjcombine_ultrascale::grid::GridKind::Ultrascale => "ultrascale",
                    prjcombine_ultrascale::grid::GridKind::UltrascalePlus => "ultrascaleplus",
                }];
                let disabled = dev
                    .disabled
                    .iter()
                    .map(|&x| match x {
                        DisabledPart::Ultrascale(x) => x,
                        _ => unreachable!(),
                    })
                    .collect();
                let grids = dev.grids.map_values(|&x| match self.grids[x] {
                    Grid::Ultrascale(ref x) => x,
                    _ => unreachable!(),
                });
                let naming = match self.dev_namings[dev.naming] {
                    DeviceNaming::Ultrascale(ref x) => x,
                    _ => unreachable!(),
                };
                ExpandedDevice::Ultrascale(prjcombine_ultrascale::expand_grid(
                    &grids,
                    dev.grid_master,
                    &disabled,
                    naming,
                    intdb,
                ))
            }
            Grid::Versal(_) => {
                let intdb = &self.ints["versal"];
                let disabled = dev
                    .disabled
                    .iter()
                    .map(|&x| match x {
                        DisabledPart::Versal(x) => x,
                        _ => unreachable!(),
                    })
                    .collect();
                let grids = dev.grids.map_values(|&x| match self.grids[x] {
                    Grid::Versal(ref x) => x,
                    _ => unreachable!(),
                });
                let naming = match self.dev_namings[dev.naming] {
                    DeviceNaming::Versal(ref x) => x,
                    _ => unreachable!(),
                };
                ExpandedDevice::Versal(prjcombine_versal::expand::expand_grid(
                    &grids, &disabled, naming, intdb,
                ))
            }
        }
    }
}
