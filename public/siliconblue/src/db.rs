use std::{collections::BTreeMap, error::Error, fs::File, path::Path};

use bincode::{Decode, Encode};
use prjcombine_entity::EntityVec;
use prjcombine_interconnect::db::IntDb;
use prjcombine_types::{
    bsdata::BsData,
    db::{BondId, ChipId, DumpFlags, SpeedId},
    speed::Speed,
};

use crate::{bond::Bond, chip::Chip};

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Device {
    pub name: String,
    pub chip: ChipId,
    pub bonds: BTreeMap<String, BondId>,
    pub speeds: BTreeMap<String, SpeedId>,
    pub temps: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Database {
    pub chips: EntityVec<ChipId, Chip>,
    pub bonds: EntityVec<BondId, Bond>,
    pub speeds: EntityVec<SpeedId, Speed>,
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
                    if dev.chip == cid {
                        write!(o, " {dev}", dev = dev.name)?;
                    }
                }
                writeln!(o)?;
                if flags.chip {
                    writeln!(o, "chip {cid} {{")?;
                    chip.dump(o, &self.int)?;
                    writeln!(o, "}}")?;
                    writeln!(o)?;
                } else {
                    writeln!(o, "chip {cid};")?;
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
                    for (pkg, &dbond) in &dev.bonds {
                        if dbond == bid {
                            write!(o, " {dev}-{pkg}", dev = dev.name)?;
                        }
                    }
                }
                writeln!(o)?;
                if flags.bond {
                    writeln!(o, "bond {bid} {{")?;
                    bond.dump(o, &self.int)?;
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

        if flags.speed || flags.device {
            for (sid, speed) in &self.speeds {
                write!(o, "//")?;
                for dev in &self.devices {
                    for (sname, &dspeed) in &dev.speeds {
                        if dspeed == sid {
                            write!(o, " {dev}-{sname}", dev = dev.name)?;
                        }
                    }
                }
                writeln!(o)?;
                if flags.speed {
                    writeln!(o, "speed {sid} {{")?;
                    write!(o, "{speed}")?;
                    writeln!(o, "}}")?;
                    writeln!(o)?;
                } else {
                    writeln!(o, "speed {sid};")?;
                }
            }
        }
        if flags.device && !flags.speed {
            writeln!(o)?;
        }

        if flags.device {
            for dev in &self.devices {
                writeln!(o, "device {n} {{", n = dev.name)?;
                writeln!(o, "\tchip {c};", c = dev.chip)?;
                for (pkg, bond) in &dev.bonds {
                    writeln!(o, "\tbond {pkg} = {bond};")?;
                }
                for (speed, sid) in &dev.speeds {
                    writeln!(o, "\tspeed {speed} = {sid};")?;
                }
                for temp in &dev.temps {
                    writeln!(o, "\ttemp {temp};")?;
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
