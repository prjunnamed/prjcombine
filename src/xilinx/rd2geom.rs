mod cfg;
mod builder;
mod part;
mod virtex2;
mod spartan3;
mod spartan6;
mod virtex4;
mod virtex5;
mod virtex6;
mod series7;

use crate::xilinx::rawdump;
use crate::xilinx::geomdb::GeomDb;
use crate::xilinx::geomraw::GeomRaw;

trait RdGeomMakerImpl {
    fn get_family(&self) -> &str;
    fn ingest(&mut self, rd: &rawdump::Part);
    fn finish(self: Box<Self>) -> (GeomDb, GeomRaw);
}

pub struct RdGeomMaker {
    maker: Option<Box<dyn RdGeomMakerImpl>>,
}

impl RdGeomMaker {
    pub fn new() -> Self {
        RdGeomMaker {
            maker: None,
        }
    }
    pub fn ingest(&mut self, rd: &rawdump::Part) {
        match &mut self.maker {
            None => {
                match &rd.family[..] {
                    // XXX xc4k
                    // XXX virtex
                    "virtex2" | "virtex2p" =>
                        self.maker = Some(Box::new(virtex2::Virtex2GeomMaker::new(&rd.family))),
                    "spartan3" | "spartan3e" | "spartan3a" | "spartan3adsp" =>
                        self.maker = Some(Box::new(spartan3::Spartan3GeomMaker::new(&rd.family))),
                    "spartan6" =>
                        self.maker = Some(Box::new(spartan6::Spartan6GeomMaker::new(&rd.family))),
                    "virtex4" =>
                        self.maker = Some(Box::new(virtex4::Virtex4GeomMaker::new(&rd.family))),
                    "virtex5" =>
                        self.maker = Some(Box::new(virtex5::Virtex5GeomMaker::new(&rd.family))),
                    "virtex6" =>
                        self.maker = Some(Box::new(virtex6::Virtex6GeomMaker::new(&rd.family))),
                    "7series" =>
                        self.maker = Some(Box::new(series7::Series7GeomMaker::new(&rd.family))),
                    // XXX ultrascale
                    // XXX ultrascaleplus
                    // XXX versal
                    _ =>
                        panic!("unknown family {}", rd.family),
                }
            },
            Some(m) => assert_eq!(m.get_family(), rd.family),
        }
        self.maker.as_mut().unwrap().ingest(rd);
    }
    pub fn finish(self) -> (GeomDb, GeomRaw) {
        self.maker.unwrap().finish()
    }
}

impl Default for RdGeomMaker {
    fn default() -> Self {
        Self::new()
    }
}
