use bincode::{Decode, Encode};
use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::grid::{DieId, ExpandedGrid, TileCoord};
use prjcombine_re_xilinx_naming::db::NamingDb;
use prjcombine_re_xilinx_naming::grid::ExpandedGridNaming;
use prjcombine_types::db::{BondId, ChipId, DevBondId, DevSpeedId, InterposerId};
use prjcombine_virtex4::gtz::GtzDb;
use prjcombine_xilinx_bitstream::{BitTile, BitstreamGeom};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs::File;
use std::path::Path;
use prjcombine_entity::EntityVec;
use prjcombine_entity::id::{EntityIdU16, EntityTag};

impl EntityTag for DeviceNaming {
    const PREFIX: &'static str = "DEVNAMING";
}
pub type DeviceNamingId = EntityIdU16<DeviceNaming>;

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub enum Chip {
    Xc2000(prjcombine_xc2000::chip::Chip),
    Virtex(prjcombine_virtex::chip::Chip),
    Virtex2(prjcombine_virtex2::chip::Chip),
    Spartan6(prjcombine_spartan6::chip::Chip),
    Virtex4(prjcombine_virtex4::chip::Chip),
    Ultrascale(prjcombine_ultrascale::chip::Chip),
    Versal(prjcombine_versal::chip::Chip),
}

impl std::fmt::Display for Chip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Chip::Xc2000(chip) => write!(f, "{chip}"),
            Chip::Virtex(chip) => write!(f, "{chip}"),
            Chip::Virtex2(chip) => write!(f, "{chip}"),
            Chip::Spartan6(chip) => write!(f, "{chip}"),
            Chip::Virtex4(chip) => write!(f, "{chip}"),
            Chip::Ultrascale(chip) => write!(f, "{chip}"),
            Chip::Versal(chip) => write!(f, "{chip}"),
        }
    }
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct DeviceBond {
    pub name: String,
    pub bond: BondId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
pub enum DisabledPart {
    Virtex(prjcombine_virtex::chip::DisabledPart),
    Spartan6(prjcombine_spartan6::chip::DisabledPart),
    Virtex4(prjcombine_virtex4::chip::DisabledPart),
    Ultrascale(prjcombine_ultrascale::chip::DisabledPart),
    Versal(prjcombine_versal::chip::DisabledPart),
}

impl std::fmt::Display for DisabledPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisabledPart::Virtex(dis) => write!(f, "{dis}"),
            DisabledPart::Spartan6(dis) => write!(f, "{dis}"),
            DisabledPart::Virtex4(dis) => write!(f, "{dis}"),
            DisabledPart::Ultrascale(dis) => write!(f, "{dis}"),
            DisabledPart::Versal(dis) => write!(f, "{dis}"),
        }
    }
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct DeviceCombo {
    pub name: String,
    pub devbond_idx: DevBondId,
    pub speed_idx: DevSpeedId,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub enum Interposer {
    None,
    Virtex4(prjcombine_virtex4::chip::Interposer),
    Ultrascale(prjcombine_ultrascale::chip::Interposer),
    Versal(prjcombine_versal::chip::Interposer),
}

impl std::fmt::Display for Interposer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Interposer::None => writeln!(f, "\t[NONE]"),
            Interposer::Virtex4(ip) => write!(f, "{ip}"),
            Interposer::Ultrascale(ip) => write!(f, "{ip}"),
            Interposer::Versal(ip) => write!(f, "{ip}"),
        }
    }
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct Device {
    pub name: String,
    pub chips: EntityVec<DieId, ChipId>,
    pub interposer: InterposerId,
    pub bonds: EntityVec<DevBondId, DeviceBond>,
    pub speeds: EntityVec<DevSpeedId, String>,
    // valid (bond, speed) pairs
    pub combos: Vec<DeviceCombo>,
    pub disabled: BTreeSet<DisabledPart>,
    pub naming: DeviceNamingId,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
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
    pub fn expand(&self) -> ExpandedBond<'_> {
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

#[derive(Clone, Debug, Encode, Decode)]
pub struct GeomDb {
    pub chips: EntityVec<ChipId, Chip>,
    pub interposers: EntityVec<InterposerId, Interposer>,
    pub bonds: EntityVec<BondId, Bond>,
    pub dev_namings: EntityVec<DeviceNamingId, DeviceNaming>,
    pub devices: Vec<Device>,
    pub ints: BTreeMap<String, IntDb>,
    pub namings: BTreeMap<String, NamingDb>,
    pub gtz: GtzDb,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub enum DeviceNaming {
    Dummy,
    Ultrascale(prjcombine_re_xilinx_naming_ultrascale::DeviceNaming),
    Versal(prjcombine_re_xilinx_naming_versal::DeviceNaming),
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

    pub fn tile_bits(&self, tcrd: TileCoord) -> Vec<BitTile> {
        match self {
            ExpandedDevice::Xc2000(edev) => edev.tile_bits(tcrd),
            ExpandedDevice::Virtex(edev) => edev.tile_bits(tcrd),
            ExpandedDevice::Virtex2(edev) => edev.tile_bits(tcrd),
            ExpandedDevice::Spartan6(edev) => edev.tile_bits(tcrd),
            ExpandedDevice::Virtex4(edev) => edev.tile_bits(tcrd),
            ExpandedDevice::Ultrascale(_) => todo!(),
            ExpandedDevice::Versal(_) => todo!(),
        }
    }
}

impl<'a> std::ops::Deref for ExpandedDevice<'a> {
    type Target = ExpandedGrid<'a>;

    fn deref(&self) -> &Self::Target {
        match self {
            ExpandedDevice::Xc2000(edev) => edev,
            ExpandedDevice::Virtex(edev) => edev,
            ExpandedDevice::Virtex2(edev) => &edev.egrid,
            ExpandedDevice::Spartan6(edev) => &edev.egrid,
            ExpandedDevice::Virtex4(edev) => &edev.egrid,
            ExpandedDevice::Ultrascale(edev) => &edev.egrid,
            ExpandedDevice::Versal(edev) => &edev.egrid,
        }
    }
}

pub enum ExpandedNamedDevice<'a> {
    Xc2000(prjcombine_re_xilinx_naming_xc2000::ExpandedNamedDevice<'a>),
    Virtex(prjcombine_re_xilinx_naming_virtex::ExpandedNamedDevice<'a>),
    Virtex2(prjcombine_re_xilinx_naming_virtex2::ExpandedNamedDevice<'a>),
    Spartan6(prjcombine_re_xilinx_naming_spartan6::ExpandedNamedDevice<'a>),
    Virtex4(prjcombine_re_xilinx_naming_virtex4::ExpandedNamedDevice<'a>),
    Ultrascale(prjcombine_re_xilinx_naming_ultrascale::ExpandedNamedDevice<'a>),
    Versal(prjcombine_re_xilinx_naming_versal::ExpandedNamedDevice<'a>),
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
        let fchip = &self.chips[*dev.chips.first().unwrap()];
        match fchip {
            Chip::Xc2000(chip) => {
                let intdb = &self.ints[match chip.kind {
                    prjcombine_xc2000::chip::ChipKind::Xc2000 => "xc2000",
                    prjcombine_xc2000::chip::ChipKind::Xc3000 => "xc3000",
                    prjcombine_xc2000::chip::ChipKind::Xc3000A => "xc3000a",
                    prjcombine_xc2000::chip::ChipKind::Xc4000 => "xc4000",
                    prjcombine_xc2000::chip::ChipKind::Xc4000A => "xc4000a",
                    prjcombine_xc2000::chip::ChipKind::Xc4000H => "xc4000h",
                    prjcombine_xc2000::chip::ChipKind::Xc4000E => "xc4000e",
                    prjcombine_xc2000::chip::ChipKind::Xc4000Ex => "xc4000ex",
                    prjcombine_xc2000::chip::ChipKind::Xc4000Xla => "xc4000xla",
                    prjcombine_xc2000::chip::ChipKind::Xc4000Xv => "xc4000xv",
                    prjcombine_xc2000::chip::ChipKind::SpartanXl => "spartanxl",
                    prjcombine_xc2000::chip::ChipKind::Xc5200 => "xc5200",
                }];
                ExpandedDevice::Xc2000(chip.expand_grid(intdb))
            }
            Chip::Virtex(chip) => {
                let intdb = &self.ints["virtex"];
                let disabled = dev
                    .disabled
                    .iter()
                    .map(|&x| match x {
                        DisabledPart::Virtex(x) => x,
                        _ => unreachable!(),
                    })
                    .collect();
                ExpandedDevice::Virtex(chip.expand_grid(&disabled, intdb))
            }
            Chip::Virtex2(chip) => {
                let intdb = if chip.kind.is_virtex2() {
                    &self.ints["virtex2"]
                } else if chip.kind == prjcombine_virtex2::chip::ChipKind::FpgaCore {
                    &self.ints["fpgacore"]
                } else {
                    &self.ints["spartan3"]
                };
                ExpandedDevice::Virtex2(chip.expand_grid(intdb))
            }
            Chip::Spartan6(chip) => {
                let intdb = &self.ints["spartan6"];
                let disabled = dev
                    .disabled
                    .iter()
                    .map(|&x| match x {
                        DisabledPart::Spartan6(x) => x,
                        _ => unreachable!(),
                    })
                    .collect();
                ExpandedDevice::Spartan6(chip.expand_grid(intdb, &disabled))
            }
            Chip::Virtex4(chip) => {
                let intdb = &self.ints[match chip.kind {
                    prjcombine_virtex4::chip::ChipKind::Virtex4 => "virtex4",
                    prjcombine_virtex4::chip::ChipKind::Virtex5 => "virtex5",
                    prjcombine_virtex4::chip::ChipKind::Virtex6 => "virtex6",
                    prjcombine_virtex4::chip::ChipKind::Virtex7 => "virtex7",
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
                let chips = dev.chips.map_values(|&x| match self.chips[x] {
                    Chip::Virtex4(ref x) => x,
                    _ => unreachable!(),
                });
                ExpandedDevice::Virtex4(prjcombine_virtex4::expand_grid(
                    &chips, interposer, &disabled, intdb, &self.gtz,
                ))
            }
            Chip::Ultrascale(chip) => {
                let intdb = &self.ints[match chip.kind {
                    prjcombine_ultrascale::chip::ChipKind::Ultrascale => "ultrascale",
                    prjcombine_ultrascale::chip::ChipKind::UltrascalePlus => "ultrascaleplus",
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
                let chips = dev.chips.map_values(|&x| match self.chips[x] {
                    Chip::Ultrascale(ref x) => x,
                    _ => unreachable!(),
                });
                ExpandedDevice::Ultrascale(prjcombine_ultrascale::expand_grid(
                    &chips, interposer, &disabled, intdb,
                ))
            }
            Chip::Versal(_) => {
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
                let chips = dev.chips.map_values(|&x| match self.chips[x] {
                    Chip::Versal(ref x) => x,
                    _ => unreachable!(),
                });
                ExpandedDevice::Versal(prjcombine_versal::expand::expand_grid(
                    &chips, interposer, &disabled, intdb,
                ))
            }
        }
    }

    pub fn name<'a>(&'a self, dev: &Device, edev: &'a ExpandedDevice) -> ExpandedNamedDevice<'a> {
        match edev {
            ExpandedDevice::Xc2000(edev) => {
                let ndb = &self.namings[match edev.chip.kind {
                    prjcombine_xc2000::chip::ChipKind::Xc2000 => "xc2000",
                    prjcombine_xc2000::chip::ChipKind::Xc3000 => "xc3000",
                    prjcombine_xc2000::chip::ChipKind::Xc3000A => "xc3000a",
                    prjcombine_xc2000::chip::ChipKind::Xc4000 => "xc4000",
                    prjcombine_xc2000::chip::ChipKind::Xc4000A => "xc4000a",
                    prjcombine_xc2000::chip::ChipKind::Xc4000H => "xc4000h",
                    prjcombine_xc2000::chip::ChipKind::Xc4000E => "xc4000e",
                    prjcombine_xc2000::chip::ChipKind::Xc4000Ex => "xc4000ex",
                    prjcombine_xc2000::chip::ChipKind::Xc4000Xla => "xc4000xla",
                    prjcombine_xc2000::chip::ChipKind::Xc4000Xv => "xc4000xv",
                    prjcombine_xc2000::chip::ChipKind::SpartanXl => "spartanxl",
                    prjcombine_xc2000::chip::ChipKind::Xc5200 => "xc5200",
                }];
                ExpandedNamedDevice::Xc2000(prjcombine_re_xilinx_naming_xc2000::name_device(
                    edev, ndb,
                ))
            }
            ExpandedDevice::Virtex(edev) => {
                let ndb = &self.namings["virtex"];
                ExpandedNamedDevice::Virtex(prjcombine_re_xilinx_naming_virtex::name_device(
                    edev, ndb,
                ))
            }
            ExpandedDevice::Virtex2(edev) => {
                let ndb = if edev.chip.kind.is_virtex2() {
                    &self.namings["virtex2"]
                } else if edev.chip.kind == prjcombine_virtex2::chip::ChipKind::FpgaCore {
                    &self.namings["fpgacore"]
                } else {
                    &self.namings["spartan3"]
                };
                ExpandedNamedDevice::Virtex2(prjcombine_re_xilinx_naming_virtex2::name_device(
                    edev, ndb,
                ))
            }
            ExpandedDevice::Spartan6(edev) => {
                let ndb = &self.namings["spartan6"];
                ExpandedNamedDevice::Spartan6(prjcombine_re_xilinx_naming_spartan6::name_device(
                    edev, ndb,
                ))
            }
            ExpandedDevice::Virtex4(edev) => {
                let ndb = &self.namings[match edev.kind {
                    prjcombine_virtex4::chip::ChipKind::Virtex4 => "virtex4",
                    prjcombine_virtex4::chip::ChipKind::Virtex5 => "virtex5",
                    prjcombine_virtex4::chip::ChipKind::Virtex6 => "virtex6",
                    prjcombine_virtex4::chip::ChipKind::Virtex7 => "virtex7",
                }];
                ExpandedNamedDevice::Virtex4(prjcombine_re_xilinx_naming_virtex4::name_device(
                    edev, ndb,
                ))
            }
            ExpandedDevice::Ultrascale(edev) => {
                let ndb = &self.namings[match edev.kind {
                    prjcombine_ultrascale::chip::ChipKind::Ultrascale => "ultrascale",
                    prjcombine_ultrascale::chip::ChipKind::UltrascalePlus => "ultrascaleplus",
                }];
                let naming = match self.dev_namings[dev.naming] {
                    DeviceNaming::Ultrascale(ref x) => x,
                    _ => unreachable!(),
                };
                ExpandedNamedDevice::Ultrascale(
                    prjcombine_re_xilinx_naming_ultrascale::name_device(edev, ndb, naming),
                )
            }
            ExpandedDevice::Versal(edev) => {
                let ndb = &self.namings["versal"];
                let naming = match self.dev_namings[dev.naming] {
                    DeviceNaming::Versal(ref x) => x,
                    _ => unreachable!(),
                };
                ExpandedNamedDevice::Versal(prjcombine_re_xilinx_naming_versal::name_device(
                    edev, ndb, naming,
                ))
            }
        }
    }
}
