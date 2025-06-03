use std::{error::Error, fs::File, path::Path};

use bincode::{Decode, Encode};
use prjcombine_types::bitvec::BitVec;

use crate::bits::{Bits, BitstreamMap};

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct FuzzDbPart {
    pub dev_name: String,
    pub pkg_name: String,
    pub bits: Bits,
    pub map: BitstreamMap,
    pub blank: BitVec,
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct FuzzDb {
    pub parts: Vec<FuzzDbPart>,
}

impl FuzzDb {
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
}
