use std::{collections::BTreeSet, error::Error, fs::File, path::Path};

use bincode::{Decode, Encode};
use itertools::Itertools;
use prjcombine_entity::{EntityMap, EntityVec};
use prjcombine_interconnect::{db::IntDb, grid::DieId};
use prjcombine_types::{
    bsdata::BsData,
    db::{BondId, ChipId, DevBondId, DevSpeedId, DeviceCombo, DumpFlags, InterposerId},
};

use crate::{
    bond::Bond,
    chip::{Chip, DisabledPart, Interposer},
};

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Device {
    pub name: String,
    pub chips: EntityVec<DieId, ChipId>,
    pub interposer: InterposerId,
    pub bonds: EntityMap<DevBondId, String, BondId>,
    pub speeds: EntityVec<DevSpeedId, String>,
    pub combos: Vec<DeviceCombo>,
    pub disabled: BTreeSet<DisabledPart>,
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Database {
    pub chips: EntityVec<ChipId, Chip>,
    pub interposers: EntityVec<InterposerId, Interposer>,
    pub bonds: EntityVec<BondId, Bond>,
    pub devices: Vec<Device>,
    pub int: IntDb,
    pub bsdata: BsData,
}

impl Database {
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

    pub fn dump(&self, o: &mut dyn std::io::Write, flags: DumpFlags) -> std::io::Result<()> {
        if flags.chip || flags.device {
            for (cid, chip) in &self.chips {
                write!(o, "//")?;
                for dev in &self.devices {
                    for (did, &die) in &dev.chips {
                        if die == cid {
                            if dev.chips.len() == 1 {
                                write!(o, " {dev}", dev = dev.name)?;
                            } else {
                                write!(o, " {dev}.{did}", dev = dev.name)?;
                            }
                        }
                    }
                }
                writeln!(o)?;
                if flags.chip {
                    writeln!(o, "chip {cid} {{")?;
                    chip.dump(o)?;
                    writeln!(o, "}}")?;
                    writeln!(o)?;
                } else {
                    writeln!(o, "chip {cid};")?;
                }
            }

            for (ipid, ip) in &self.interposers {
                write!(o, "//")?;
                for dev in &self.devices {
                    if dev.interposer == ipid {
                        write!(o, " {dev}", dev = dev.name)?;
                    }
                }
                writeln!(o)?;
                if flags.chip {
                    writeln!(o, "interposer {ipid} {{")?;
                    ip.dump(o)?;
                    writeln!(o, "}}")?;
                    writeln!(o)?;
                } else {
                    writeln!(o, "interposer {ipid};")?;
                }
            }
        }
        if flags.device && !flags.chip {
            writeln!(o)?;
        }

        if flags.bond || flags.device {
            for (bid, bond) in &self.bonds {
                write!(o, "//")?;
                for dev in &self.devices {
                    for (_, pkg, &dbond) in &dev.bonds {
                        if dbond == bid {
                            write!(o, " {dev}-{pkg}", dev = dev.name)?;
                        }
                    }
                }
                writeln!(o)?;
                if flags.bond {
                    writeln!(o, "bond {bid} {{")?;
                    bond.dump(o)?;
                    writeln!(o, "}}")?;
                    writeln!(o)?;
                } else {
                    writeln!(o, "bond {bid};")?;
                }
            }
        }
        if flags.device && !flags.bond {
            writeln!(o)?;
        }

        if flags.device {
            for dev in &self.devices {
                writeln!(o, "device {n} {{", n = dev.name)?;
                writeln!(
                    o,
                    "\tchip {};",
                    dev.chips.ids().map(|x| x.to_string()).join(", ")
                )?;
                writeln!(o, "\tinterposer {ipid};", ipid = dev.interposer)?;
                for (_dpid, pkg, bond) in &dev.bonds {
                    writeln!(o, "\tbond {pkg} = {bond};")?;
                }
                for speed in dev.speeds.values() {
                    writeln!(o, "\tspeed {speed};")?;
                }
                for combo in &dev.combos {
                    writeln!(
                        o,
                        "\tcombo {pkg} {speed};",
                        pkg = dev.bonds.key(combo.devbond),
                        speed = dev.speeds[combo.speed]
                    )?;
                }
                for &dis in &dev.disabled {
                    writeln!(o, "\tdisabled {dis};")?;
                }
                writeln!(o, "}}")?;
                writeln!(o)?;
            }
        }

        if flags.intdb {
            self.int.dump(o)?;
            writeln!(o)?;
        }

        if flags.bsdata {
            self.bsdata.dump(o)?;
        }
        Ok(())
    }
}
