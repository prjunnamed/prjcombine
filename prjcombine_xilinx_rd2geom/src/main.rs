#![allow(clippy::collapsible_else_if)]

use prjcombine_rawdump::Part;
use rayon::prelude::*;
use simple_error::bail;
use std::error::Error;
use structopt::StructOpt;

mod db;
mod series7;
mod spartan6;
mod ultrascale;
mod versal;
mod virtex;
mod virtex2;
mod virtex4;
mod virtex5;
mod virtex6;
mod xc4k;
mod xc5200;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "prjcombine_xilinx_rd2geom",
    about = "Extract geometry information from rawdumps."
)]
struct Opt {
    dst: String,
    files: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    if opt.files.is_empty() {
        bail!("no files given");
    }
    let mut builder = db::DbBuilder::new();
    let pres: Vec<_> = opt
        .files
        .par_iter()
        .map(|file| {
            let rd = Part::from_file(file).unwrap();
            println!("INGEST {} {:?}", rd.part, rd.source);
            match &rd.family[..] {
                "xc4000e" | "xc4000ex" | "xc4000xla" | "xc4000xv" | "spartanxl" => {
                    xc4k::ingest(&rd)
                }
                "xc5200" => xc5200::ingest(&rd),
                "virtex" | "virtexe" => virtex::ingest(&rd),
                "virtex2" | "virtex2p" | "spartan3" | "spartan3e" | "spartan3a"
                | "spartan3adsp" => virtex2::ingest(&rd),
                "spartan6" => spartan6::ingest(&rd),
                "virtex4" => virtex4::ingest(&rd),
                "virtex5" => virtex5::ingest(&rd),
                "virtex6" => virtex6::ingest(&rd),
                "7series" => series7::ingest(&rd),
                "ultrascale" | "ultrascaleplus" => ultrascale::ingest(&rd),
                "versal" => versal::ingest(&rd),
                _ => panic!("unknown family {}", rd.family),
            }
        })
        .collect();
    for (pre, idb) in pres {
        builder.ingest(pre);
        if let Some(int_db) = idb {
            builder.ingest_int(int_db);
        }
    }
    let db = builder.finish();
    db.to_file(opt.dst)?;
    Ok(())
}
