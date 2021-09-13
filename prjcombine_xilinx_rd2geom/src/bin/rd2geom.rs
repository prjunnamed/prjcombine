use prjcombine_xilinx_rawdump::Part;
use prjcombine_xilinx_rd2geom::xilinx::rd2geom::RdGeomMaker;
use std::error::Error;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "rd2geom", about = "Create geometry database out of rawdumps.")]
struct Opt {
    dst: String,
    files: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let mut r2g = RdGeomMaker::new();
    for file in opt.files.iter() {
        let rd = Part::from_file(file)?;
        println!("INGEST {} {:?}", rd.part, rd.source);
        r2g.ingest(&rd);
    }
    let (geomdb, raw) = r2g.finish();
    raw.to_file(&geomdb, opt.dst)?;
    Ok(())
}
