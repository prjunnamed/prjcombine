use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs::File,
    path::Path,
};

use bincode::{Decode, Encode};
use itertools::Itertools;
use prjcombine_entity::{
    EntityRange, EntityVec,
    id::{EntityIdU8, EntityTag},
};
use prjcombine_types::{
    bsdata::Tile,
    cpld::{BlockId, MacrocellCoord, MacrocellId},
    db::{BondId, ChipId, DumpFlags, SpeedId},
    speed::Speed,
};

pub struct GclkTag;
impl EntityTag for GclkTag {
    const PREFIX: &'static str = "GCLK";
}
pub type GclkId = EntityIdU8<GclkTag>;

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Chip {
    pub idcode_part: u32,
    pub bs_cols: usize,
    pub imux_width: usize,
    pub block_rows: usize,
    pub block_cols: Vec<FbColumn>,
    pub io_mcs: BTreeSet<MacrocellId>,
    pub io_special: BTreeMap<String, MacrocellCoord>,
    pub global_bits: Tile,
    pub jed_global_bits: Vec<(String, usize)>,
    pub imux_bits: Tile,
}

impl Chip {
    pub fn blocks(&self) -> EntityRange<BlockId> {
        EntityRange::new(0, self.block_rows * self.block_cols.len() * 2)
    }

    pub fn dump(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(o, "\tidcode_part 0x{:04x};", self.idcode_part)?;
        writeln!(o, "\tbs_cols {};", self.bs_cols)?;
        writeln!(o, "\timux_width {};", self.imux_width)?;
        writeln!(o, "\tblock_rows {};", self.block_rows)?;
        for col in &self.block_cols {
            writeln!(
                o,
                "\tblock_col pt {}, imux {}, mc {};",
                col.pt_col, col.imux_col, col.mc_col
            )?;
        }
        writeln!(
            o,
            "\tio_mcs {};",
            self.io_mcs.iter().map(|x| x.to_string()).join(", ")
        )?;
        for (k, v) in &self.io_special {
            writeln!(o, "\tio_special {k} = {v};")?;
        }

        writeln!(o)?;
        writeln!(o, "\tbstile GLOBAL_BITS {{")?;
        self.global_bits.dump(o)?;
        writeln!(o, "\t}}")?;

        writeln!(o)?;
        writeln!(o, "\tjedtile GLOBAL_BITS {{")?;
        for (name, idx) in &self.jed_global_bits {
            writeln!(o, "\t\t{name}[{idx}],")?;
        }
        writeln!(o, "\t}}")?;

        writeln!(o)?;
        writeln!(o, "\tbstile IMUX_BITS {{")?;
        self.imux_bits.dump(o)?;
        writeln!(o, "\t}}")?;

        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct FbColumn {
    pub pt_col: usize,
    pub imux_col: usize,
    pub mc_col: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Encode, Decode)]
pub enum BondPad {
    Nc,
    Gnd,
    Vcc,
    Gclk(GclkId),
    Iob(MacrocellCoord),
    PortEn,
}

impl std::fmt::Display for BondPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondPad::Nc => write!(f, "NC"),
            BondPad::Gnd => write!(f, "GND"),
            BondPad::Vcc => write!(f, "VCC"),
            BondPad::Iob(mc) => write!(f, "IOB_{mc}"),
            BondPad::Gclk(gclk) => write!(f, "{gclk}"),
            BondPad::PortEn => write!(f, "PORT_EN"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Bond {
    pub idcode_part: u32,
    pub pins: BTreeMap<String, BondPad>,
}

fn pad_sort_key(name: &str) -> (usize, &str, u32) {
    let pos = name.find(|x: char| x.is_ascii_digit()).unwrap();
    (pos, &name[..pos], name[pos..].parse().unwrap())
}

impl Bond {
    pub fn dump(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(o, "\tidcode_part 0x{:04x};", self.idcode_part)?;
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
    pub jed_mc_bits_iob: Vec<(String, usize)>,
    pub jed_mc_bits_buried: Vec<(String, usize)>,
    pub jed_block_bits: Vec<(String, usize)>,
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

            writeln!(o, "jedtile MC_BITS_IOB {{")?;
            for (name, idx) in &self.jed_mc_bits_iob {
                writeln!(o, "\t{name}[{idx}],")?;
            }
            writeln!(o, "}}")?;
            writeln!(o)?;
            writeln!(o, "jedtile MC_BITS_BURIED {{")?;
            for (name, idx) in &self.jed_mc_bits_buried {
                writeln!(o, "\t{name}[{idx}],")?;
            }
            writeln!(o, "}}")?;
            writeln!(o)?;
            writeln!(o, "jedtile BLOCK_BITS {{")?;
            for (name, idx) in &self.jed_block_bits {
                writeln!(o, "\t{name}[{idx}],")?;
            }
            writeln!(o, "}}")?;
            writeln!(o)?;
        }
        Ok(())
    }
}
