use std::{collections::BTreeMap, error::Error, fs::File, path::Path};

use bincode::{Decode, Encode};
use prjcombine_ecp::{bond::Bond, chip::Chip, db::Device};
use prjcombine_interconnect::{
    db::IntDb,
    grid::{BelCoord, WireCoord},
};
use prjcombine_types::db::{BondId, ChipId};
use unnamed_entity::{
    EntitySet, EntityVec,
    id::{EntityIdU32, EntityTag},
};

pub struct StringTag;

impl EntityTag for StringTag {}

pub type StringId = EntityIdU32<StringTag>;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Encode, Decode)]
pub struct WireName {
    pub r: u8,
    pub c: u8,
    pub suffix: StringId,
}

#[derive(Clone, Debug, Encode, Decode, Default)]
pub struct ChipNaming {
    pub strings: EntitySet<StringId, String>,
    pub interconnect: BTreeMap<WireCoord, WireName>,
    pub bels: BTreeMap<BelCoord, BelNaming>,
}
impl ChipNaming {
    pub fn bel_wire(&self, bel: BelCoord, wire: &str) -> WireName {
        let bel = &self.bels[&bel];
        let wire = self.strings.get(wire).unwrap();
        bel.wires[&wire]
    }
}

#[derive(Clone, Debug, Encode, Decode, Default)]
pub struct BelNaming {
    pub names: Vec<StringId>,
    pub wires: BTreeMap<StringId, WireName>,
}

#[derive(Clone, Debug, Encode, Decode, Default)]
pub struct Database {
    pub chips: EntityVec<ChipId, (Chip, ChipNaming)>,
    pub bonds: EntityVec<BondId, Bond>,
    pub devices: Vec<Device>,
    pub int: IntDb,
}

impl WireName {
    pub fn to_string(self, naming: &ChipNaming) -> String {
        format!(
            "R{r}C{c}_{suffix}",
            r = self.r,
            c = self.c,
            suffix = naming.strings[self.suffix]
        )
    }
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
}
