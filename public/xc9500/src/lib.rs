use std::{collections::BTreeMap, error::Error, fs::File, path::Path};

use bincode::{Decode, Encode};
use itertools::Itertools;
use prjcombine_entity::{
    EntityVec,
    id::{EntityIdU8, EntityTag},
};
use prjcombine_types::{
    bsdata::Tile,
    cpld::MacrocellCoord,
    db::{BondId, ChipId, DumpFlags, SpeedId},
    speed::Speed,
};

pub struct BankTag;
impl EntityTag for BankTag {
    const PREFIX: &'static str = "BANK";
}
pub type BankId = EntityIdU8<BankTag>;

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub enum ChipKind {
    Xc9500,
    Xc9500Xl,
    Xc9500Xv,
}

impl std::fmt::Display for ChipKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChipKind::Xc9500 => write!(f, "xc9500"),
            ChipKind::Xc9500Xl => write!(f, "xc9500xl"),
            ChipKind::Xc9500Xv => write!(f, "xc9500xv"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Chip {
    pub kind: ChipKind,
    pub idcode: u32,
    pub blocks: usize,
    pub io: BTreeMap<MacrocellCoord, BankId>,
    pub banks: usize,
    pub tdo_bank: BankId,
    pub io_special: BTreeMap<String, MacrocellCoord>,
    pub imux_bits: Tile,
    pub uim_ibuf_bits: Option<Tile>,
    pub program_time: u32,
    pub erase_time: u32,
}

impl Chip {
    pub fn dump(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(o, "\tkind {};", self.kind)?;
        writeln!(o, "\tidcode 0x{:08x};", self.idcode)?;
        writeln!(o, "\tblocks {};", self.blocks)?;
        writeln!(o, "\tbanks {};", self.banks)?;
        for (k, v) in &self.io {
            writeln!(o, "\tio {k} = {v};")?;
        }
        writeln!(o, "\ttdo_bank {};", self.tdo_bank)?;
        for (k, v) in &self.io_special {
            writeln!(o, "\tio_special {k} = {v};")?;
        }
        writeln!(o, "\tprogram_time {};", self.program_time)?;
        writeln!(o, "\terase_time {};", self.erase_time)?;
        writeln!(o)?;
        writeln!(o, "\tbstile IMUX_BITS {{")?;
        self.imux_bits.dump(o)?;
        writeln!(o, "\t}}")?;
        if let Some(ref bits) = self.uim_ibuf_bits {
            writeln!(o)?;
            writeln!(o, "\tbstile UIM_IBUF_BITS {{")?;
            bits.dump(o)?;
            writeln!(o, "\t}}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Encode, Decode)]
pub enum BondPad {
    Nc,
    Gnd,
    VccInt,
    VccIo(BankId),
    Iob(MacrocellCoord),
    Tms,
    Tck,
    Tdi,
    Tdo,
}

impl std::fmt::Display for BondPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPad::Nc => write!(f, "NC"),
            BondPad::Gnd => write!(f, "GND"),
            BondPad::VccInt => write!(f, "VCCINT"),
            BondPad::VccIo(bank) => write!(f, "VCCIO{bank:#}"),
            BondPad::Iob(mc) => write!(f, "IOB_{mc}"),
            BondPad::Tms => write!(f, "TMS"),
            BondPad::Tck => write!(f, "TCK"),
            BondPad::Tdi => write!(f, "TDI"),
            BondPad::Tdo => write!(f, "TDO"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Bond {
    pub io_special_override: BTreeMap<String, MacrocellCoord>,
    pub pins: BTreeMap<String, BondPad>,
}

fn pad_sort_key(name: &str) -> (usize, &str, u32) {
    let pos = name.find(|x: char| x.is_ascii_digit()).unwrap();
    (pos, &name[..pos], name[pos..].parse().unwrap())
}

impl Bond {
    pub fn dump(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        for (k, v) in &self.io_special_override {
            writeln!(o, "\tio_special_override {k} = {v};")?;
        }
        for (pin, pad) in self.pins.iter().sorted_by_key(|(k, _)| pad_sort_key(k)) {
            writeln!(o, "\tpin {pin} = {pad};")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Device {
    pub name: String,
    pub chip: ChipId,
    pub bonds: BTreeMap<String, BondId>,
    pub speeds: BTreeMap<String, SpeedId>,
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Database {
    pub chips: EntityVec<ChipId, Chip>,
    pub bonds: EntityVec<BondId, Bond>,
    pub speeds: EntityVec<SpeedId, Speed>,
    pub devices: Vec<Device>,
    pub mc_bits: Tile,
    pub block_bits: Tile,
    pub global_bits: Tile,
}

impl Database {
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let f = File::create(path)?;
        let mut cf = zstd::stream::Encoder::new(f, 9)?;
        let config = bincode::config::standard();
        bincode::encode_into_std_write(self, &mut cf, config)?;
        cf.finish()?;
        Ok(())
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let f = File::open(path)?;
        let mut cf = zstd::stream::Decoder::new(f)?;
        let config = bincode::config::standard();
        Ok(bincode::decode_from_std_read(&mut cf, config)?)
    }

    pub fn dump(&self, o: &mut dyn std::io::Write, flags: DumpFlags) -> std::io::Result<()> {
        if flags.chip || flags.device {
            for (cid, chip) in &self.chips {
                write!(o, "//")?;
                for dev in &self.devices {
                    if cid == dev.chip {
                        write!(o, " {dev}", dev = dev.name)?;
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
                writeln!(o, "\tchip {cid};", cid = dev.chip)?;
                for (pkg, bond) in &dev.bonds {
                    writeln!(o, "\tbond {pkg} = {bond};")?;
                }
                for speed in dev.speeds.values() {
                    writeln!(o, "\tspeed {speed};")?;
                }
                writeln!(o, "}}")?;
                writeln!(o)?;
            }
        }

        if flags.bsdata {
            writeln!(o, "bstile MC_BITS {{")?;
            self.mc_bits.dump(o)?;
            writeln!(o, "}}")?;
            writeln!(o)?;
            writeln!(o, "bstile BLOCK_BITS {{")?;
            self.block_bits.dump(o)?;
            writeln!(o, "}}")?;
            writeln!(o)?;
            writeln!(o, "bstile GLOBAL_BITS {{")?;
            self.global_bits.dump(o)?;
            writeln!(o, "}}")?;
            writeln!(o)?;
        }
        Ok(())
    }
}
