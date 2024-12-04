use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{DieId, ExpandedGrid};
use prjcombine_virtex_bitstream::BitstreamGeom;
use prjcombine_xilinx_naming::db::NamingDb;
use prjcombine_xilinx_naming::grid::ExpandedGridNaming;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs::File;
use std::path::Path;
use unnamed_entity::{entity_id, EntityVec};

entity_id! {
    pub id GridId usize;
    pub id InterposerId usize;
    pub id BondId usize;
    pub id DevBondId usize;
    pub id DevSpeedId usize;
    pub id DeviceNamingId usize;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Grid {
    Xc2000(prjcombine_xc2000::grid::Grid),
    Virtex(prjcombine_virtex::grid::Grid),
    Virtex2(prjcombine_virtex2::grid::Grid),
    Spartan6(prjcombine_spartan6::grid::Grid),
    Virtex4(prjcombine_virtex4::grid::Grid),
    Ultrascale(prjcombine_ultrascale::grid::Grid),
    Versal(prjcombine_versal::grid::Grid),
}

impl std::fmt::Display for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Grid::Xc2000(grid) => write!(f, "{grid}"),
            Grid::Virtex(grid) => write!(f, "{grid}"),
            Grid::Virtex2(grid) => write!(f, "{grid}"),
            Grid::Spartan6(grid) => write!(f, "{grid}"),
            Grid::Virtex4(grid) => write!(f, "{grid}"),
            Grid::Ultrascale(grid) => write!(f, "{grid}"),
            Grid::Versal(grid) => write!(f, "{grid}"),
        }
    }
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Interposer {
    None,
    Virtex4(prjcombine_virtex4::grid::Interposer),
    Ultrascale(prjcombine_ultrascale::grid::Interposer),
    Versal(prjcombine_versal::grid::Interposer),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Device {
    pub name: String,
    pub grids: EntityVec<DieId, GridId>,
    pub interposer: InterposerId,
    pub bonds: EntityVec<DevBondId, DeviceBond>,
    pub speeds: EntityVec<DevSpeedId, String>,
    // valid (bond, speed) pairs
    pub combos: Vec<DeviceCombo>,
    pub disabled: BTreeSet<DisabledPart>,
    pub naming: DeviceNamingId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Bond {
    Xc2000(prjcombine_xc2000::bond::Bond),
    Virtex(prjcombine_virtex::bond::Bond),
    Virtex2(prjcombine_virtex2::bond::Bond),
    Spartan6(prjcombine_spartan6::bond::Bond),
    Virtex4(prjcombine_virtex4::bond::Bond),
    Ultrascale(prjcombine_ultrascale::bond::Bond),
    Versal(prjcombine_versal::bond::Bond),
}

impl std::fmt::Display for Bond {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Bond::Xc2000(bond) => write!(f, "{bond}"),
            Bond::Virtex(bond) => write!(f, "{bond}"),
            Bond::Virtex2(bond) => write!(f, "{bond}"),
            Bond::Spartan6(bond) => write!(f, "{bond}"),
            Bond::Virtex4(bond) => write!(f, "{bond}"),
            Bond::Ultrascale(bond) => write!(f, "{bond}"),
            Bond::Versal(bond) => write!(f, "{bond}"),
        }
    }
}

pub enum ExpandedBond<'a> {
    Xc2000(prjcombine_xc2000::bond::ExpandedBond<'a>),
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
            Bond::Xc2000(bond) => ExpandedBond::Xc2000(bond.expand()),
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
    pub interposers: EntityVec<InterposerId, Interposer>,
    pub bonds: EntityVec<BondId, Bond>,
    pub dev_namings: EntityVec<DeviceNamingId, DeviceNaming>,
    pub devices: Vec<Device>,
    pub ints: BTreeMap<String, IntDb>,
    pub namings: BTreeMap<String, NamingDb>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DeviceNaming {
    Dummy,
    Ultrascale(prjcombine_ultrascale_naming::DeviceNaming),
    Versal(prjcombine_versal_naming::DeviceNaming),
}

pub enum ExpandedDevice<'a> {
    Xc2000(prjcombine_xc2000::expanded::ExpandedDevice<'a>),
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
            ExpandedDevice::Xc2000(edev) => &edev.egrid,
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
            ExpandedDevice::Xc2000(edev) => &edev.bs_geom,
            ExpandedDevice::Virtex(edev) => &edev.bs_geom,
            ExpandedDevice::Virtex2(edev) => &edev.bs_geom,
            ExpandedDevice::Spartan6(edev) => &edev.bs_geom,
            ExpandedDevice::Virtex4(edev) => &edev.bs_geom,
            ExpandedDevice::Ultrascale(_) => todo!(),
            ExpandedDevice::Versal(_) => todo!(),
        }
    }
}

pub enum ExpandedNamedDevice<'a> {
    Xc2000(prjcombine_xc2000_naming::ExpandedNamedDevice<'a>),
    Virtex(prjcombine_virtex_naming::ExpandedNamedDevice<'a>),
    Virtex2(prjcombine_virtex2_naming::ExpandedNamedDevice<'a>),
    Spartan6(prjcombine_spartan6_naming::ExpandedNamedDevice<'a>),
    Virtex4(prjcombine_virtex4_naming::ExpandedNamedDevice<'a>),
    Ultrascale(prjcombine_ultrascale_naming::ExpandedNamedDevice<'a>),
    Versal(prjcombine_versal_naming::ExpandedNamedDevice<'a>),
}

impl<'a> ExpandedNamedDevice<'a> {
    pub fn ngrid(&self) -> &ExpandedGridNaming<'a> {
        match self {
            ExpandedNamedDevice::Xc2000(endev) => &endev.ngrid,
            ExpandedNamedDevice::Virtex(endev) => &endev.ngrid,
            ExpandedNamedDevice::Virtex2(endev) => &endev.ngrid,
            ExpandedNamedDevice::Spartan6(endev) => &endev.ngrid,
            ExpandedNamedDevice::Virtex4(endev) => &endev.ngrid,
            ExpandedNamedDevice::Ultrascale(endev) => &endev.ngrid,
            ExpandedNamedDevice::Versal(endev) => &endev.ngrid,
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
        let fgrid = &self.grids[*dev.grids.first().unwrap()];
        match fgrid {
            Grid::Xc2000(grid) => {
                let intdb = &self.ints[match grid.kind {
                    prjcombine_xc2000::grid::GridKind::Xc2000 => "xc2000",
                    prjcombine_xc2000::grid::GridKind::Xc3000 => "xc3000",
                    prjcombine_xc2000::grid::GridKind::Xc3000A => "xc3000a",
                    prjcombine_xc2000::grid::GridKind::Xc4000 => "xc4000",
                    prjcombine_xc2000::grid::GridKind::Xc4000A => "xc4000a",
                    prjcombine_xc2000::grid::GridKind::Xc4000H => "xc4000h",
                    prjcombine_xc2000::grid::GridKind::Xc4000E => "xc4000e",
                    prjcombine_xc2000::grid::GridKind::Xc4000Ex => "xc4000ex",
                    prjcombine_xc2000::grid::GridKind::Xc4000Xla => "xc4000xla",
                    prjcombine_xc2000::grid::GridKind::Xc4000Xv => "xc4000xv",
                    prjcombine_xc2000::grid::GridKind::SpartanXl => "spartanxl",
                    prjcombine_xc2000::grid::GridKind::Xc5200 => "xc5200",
                }];
                ExpandedDevice::Xc2000(grid.expand_grid(intdb))
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
                let interposer = match self.interposers[dev.interposer] {
                    Interposer::None => None,
                    Interposer::Virtex4(ref ip) => Some(ip),
                    _ => unreachable!(),
                };
                let grids = dev.grids.map_values(|&x| match self.grids[x] {
                    Grid::Virtex4(ref x) => x,
                    _ => unreachable!(),
                });
                ExpandedDevice::Virtex4(prjcombine_virtex4::expand_grid(
                    &grids, interposer, &disabled, intdb,
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
                let interposer = match self.interposers[dev.interposer] {
                    Interposer::Ultrascale(ref ip) => ip,
                    _ => unreachable!(),
                };
                let grids = dev.grids.map_values(|&x| match self.grids[x] {
                    Grid::Ultrascale(ref x) => x,
                    _ => unreachable!(),
                });
                ExpandedDevice::Ultrascale(prjcombine_ultrascale::expand_grid(
                    &grids, interposer, &disabled, intdb,
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
                let interposer = match self.interposers[dev.interposer] {
                    Interposer::Versal(ref ip) => ip,
                    _ => unreachable!(),
                };
                let grids = dev.grids.map_values(|&x| match self.grids[x] {
                    Grid::Versal(ref x) => x,
                    _ => unreachable!(),
                });
                ExpandedDevice::Versal(prjcombine_versal::expand::expand_grid(
                    &grids, interposer, &disabled, intdb,
                ))
            }
        }
    }

    pub fn name<'a>(&'a self, dev: &Device, edev: &'a ExpandedDevice) -> ExpandedNamedDevice<'a> {
        match edev {
            ExpandedDevice::Xc2000(edev) => {
                let ndb = &self.namings[match edev.grid.kind {
                    prjcombine_xc2000::grid::GridKind::Xc2000 => "xc2000",
                    prjcombine_xc2000::grid::GridKind::Xc3000 => "xc3000",
                    prjcombine_xc2000::grid::GridKind::Xc3000A => "xc3000a",
                    prjcombine_xc2000::grid::GridKind::Xc4000 => "xc4000",
                    prjcombine_xc2000::grid::GridKind::Xc4000A => "xc4000a",
                    prjcombine_xc2000::grid::GridKind::Xc4000H => "xc4000h",
                    prjcombine_xc2000::grid::GridKind::Xc4000E => "xc4000e",
                    prjcombine_xc2000::grid::GridKind::Xc4000Ex => "xc4000ex",
                    prjcombine_xc2000::grid::GridKind::Xc4000Xla => "xc4000xla",
                    prjcombine_xc2000::grid::GridKind::Xc4000Xv => "xc4000xv",
                    prjcombine_xc2000::grid::GridKind::SpartanXl => "spartanxl",
                    prjcombine_xc2000::grid::GridKind::Xc5200 => "xc5200",
                }];
                ExpandedNamedDevice::Xc2000(prjcombine_xc2000_naming::name_device(edev, ndb))
            }
            ExpandedDevice::Virtex(edev) => {
                let ndb = &self.namings["virtex"];
                ExpandedNamedDevice::Virtex(prjcombine_virtex_naming::name_device(edev, ndb))
            }
            ExpandedDevice::Virtex2(edev) => {
                let ndb = if edev.grid.kind.is_virtex2() {
                    &self.namings["virtex2"]
                } else if edev.grid.kind == prjcombine_virtex2::grid::GridKind::FpgaCore {
                    &self.namings["fpgacore"]
                } else {
                    &self.namings["spartan3"]
                };
                ExpandedNamedDevice::Virtex2(prjcombine_virtex2_naming::name_device(edev, ndb))
            }
            ExpandedDevice::Spartan6(edev) => {
                let ndb = &self.namings["spartan6"];
                ExpandedNamedDevice::Spartan6(prjcombine_spartan6_naming::name_device(edev, ndb))
            }
            ExpandedDevice::Virtex4(edev) => {
                let ndb = &self.namings[match edev.kind {
                    prjcombine_virtex4::grid::GridKind::Virtex4 => "virtex4",
                    prjcombine_virtex4::grid::GridKind::Virtex5 => "virtex5",
                    prjcombine_virtex4::grid::GridKind::Virtex6 => "virtex6",
                    prjcombine_virtex4::grid::GridKind::Virtex7 => "virtex7",
                }];
                ExpandedNamedDevice::Virtex4(prjcombine_virtex4_naming::name_device(edev, ndb))
            }
            ExpandedDevice::Ultrascale(edev) => {
                let ndb = &self.namings[match edev.kind {
                    prjcombine_ultrascale::grid::GridKind::Ultrascale => "ultrascale",
                    prjcombine_ultrascale::grid::GridKind::UltrascalePlus => "ultrascaleplus",
                }];
                let naming = match self.dev_namings[dev.naming] {
                    DeviceNaming::Ultrascale(ref x) => x,
                    _ => unreachable!(),
                };
                ExpandedNamedDevice::Ultrascale(prjcombine_ultrascale_naming::name_device(
                    edev, ndb, naming,
                ))
            }
            ExpandedDevice::Versal(edev) => {
                let ndb = &self.namings["versal"];
                let naming = match self.dev_namings[dev.naming] {
                    DeviceNaming::Versal(ref x) => x,
                    _ => unreachable!(),
                };
                ExpandedNamedDevice::Versal(prjcombine_versal_naming::name_device(
                    edev, ndb, naming,
                ))
            }
        }
    }
}
