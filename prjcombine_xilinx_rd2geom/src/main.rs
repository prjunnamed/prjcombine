#![allow(clippy::collapsible_else_if)]

use prjcombine_rawdump::Part;
use simple_error::bail;
use std::error::Error;
use std::sync::Mutex;
use std_semaphore::Semaphore;
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
    #[structopt(long)]
    no_verify: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    if opt.files.is_empty() {
        bail!("no files given");
    }
    let builder = Mutex::new(db::DbBuilder::new());
    let rb = &builder;
    let sema = Semaphore::new(std::thread::available_parallelism().unwrap().get() as isize);
    let verify = !opt.no_verify;
    std::thread::scope(|s| {
        for fname in opt.files {
            let guard = sema.access();
            let mut tname = &fname[..];
            if let Some((_, n)) = tname.rsplit_once('/') {
                tname = n;
            }
            if let Some((n, _)) = tname.split_once('.') {
                tname = n;
            }
            std::thread::Builder::new()
                .name(tname.to_string())
                .spawn_scoped(s, move || {
                    let rd = Part::from_file(fname).unwrap();
                    println!("INGEST {} {:?}", rd.part, rd.source);
                    let (pre, idb) = match &rd.family[..] {
                        "xc4000e" | "xc4000ex" | "xc4000xla" | "xc4000xv" | "spartanxl" => {
                            xc4k::ingest(&rd, verify)
                        }
                        "xc5200" => xc5200::ingest(&rd, verify),
                        "virtex" | "virtexe" => virtex::ingest(&rd, verify),
                        "virtex2" | "virtex2p" | "spartan3" | "spartan3e" | "spartan3a"
                        | "spartan3adsp" => virtex2::ingest(&rd, verify),
                        "spartan6" => spartan6::ingest(&rd, verify),
                        "virtex4" => virtex4::ingest(&rd, verify),
                        "virtex5" => virtex5::ingest(&rd, verify),
                        "virtex6" => virtex6::ingest(&rd, verify),
                        "7series" => series7::ingest(&rd, verify),
                        "ultrascale" | "ultrascaleplus" => ultrascale::ingest(&rd, verify),
                        "versal" => versal::ingest(&rd, verify),
                        _ => panic!("unknown family {}", rd.family),
                    };
                    let mut builder = rb.lock().unwrap();
                    builder.ingest(pre);
                    if let Some(int_db) = idb {
                        builder.ingest_int(int_db);
                    }
                    std::mem::drop(guard);
                })
                .unwrap();
        }
    });
    let db = builder.into_inner().unwrap().finish();
    db.to_file(opt.dst)?;
    Ok(())
}
