use std::fs::File;
use std::path::Path;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::namevec::{NameVec, Named};
use crate::xilinx::geomdb::{GeomDb, TCWire};
use crate::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeomRaw {
    pub extracts: NameVec<ExtractClass>,
    pub port_extracts: NameVec<PortExtractClass>,
    pub parts: Vec<PartRaw>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct ExtractClass {
    pub name: String,
    pub tcls: usize,
    // (wire dst, wire src) -> list of raw pips
    pub pips: HashMap<(TCWire, TCWire), Vec<RawPip>>,
    pub sites: Vec<Option<ExtractSite>>,
    pub ties: HashMap<TCWire, ExtractTie>,
}

impl Named for ExtractClass {
    fn get_name(&self) -> &str { &self.name }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct ExtractSite {
    pub rtidx: usize,
    pub rsidx: usize,
    pub kind: String,
    pub pins: HashMap<String, ExtractSitePin>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct ExtractSitePin {
    pub pips: Vec<RawPip>,
    pub pad: Option<ExtractSitePinPad>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct ExtractSitePinPad {
    pub rtidx: usize,
    pub rsidx: usize,
    pub kind: String,
    pub pin: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct ExtractTie {
    pub rtidx: usize,
    pub rsidx: usize,
    pub kind: String,
    pub pin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortExtractClass {
    pub name: String,
    pub pcls: usize,
    pub conns: Vec<Vec<RawPip>>,
}

impl Named for PortExtractClass {
    fn get_name(&self) -> &str { &self.name }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RawPip {
    pub rtidx: usize,
    pub wire_out: String,
    pub wire_in: String,
    pub is_excl: bool,
    pub is_test: bool,
    pub direction: RawPipDirection,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum RawPipDirection {
    Uni,
    BiFwd,
    BiBwd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartRaw {
    // x, y, tile class -> Extract
    pub tiles: HashMap<((usize, usize), usize), Extract>,
    // x, y, port slot -> PortExtract
    pub ports: HashMap<((usize, usize), usize), PortExtract>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extract {
    pub cls: usize,
    pub raw_tiles: Vec<String>,
    pub raw_sites: Vec<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortExtract {
    pub cls: usize,
    pub raw_tiles: Vec<String>,
}

impl GeomRaw {
    pub fn from_file<P: AsRef<Path>> (path: P) -> Result<(GeomDb, Self), Error> {
        let f = File::open(path)?;
        let mut cf = zstd::stream::Decoder::new(f)?;
        let res_g = bincode::deserialize_from(&mut cf).unwrap();
        let res_r = bincode::deserialize_from(cf).unwrap();
        Ok((res_g, res_r))
    }

    pub fn to_file<P: AsRef<Path>> (&self, geomdb: &GeomDb, path: P) -> Result<(), Error> {
        let f = File::create(path)?;
        let mut cf = zstd::stream::Encoder::new(f, 9)?;
        bincode::serialize_into(&mut cf, geomdb).unwrap();
        bincode::serialize_into(&mut cf, self).unwrap();
        cf.finish()?;
        Ok(())
    }
}
