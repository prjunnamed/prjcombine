use clap::Parser;
use prjcombine_hammer::Session;
use prjcombine_toolchain::Toolchain;
use prjcombine_xilinx_geom::{ExpandedDevice, GeomDb};
use std::error::Error;
use std::path::PathBuf;

mod backend;
mod clb;
mod fgen;

use backend::IseBackend;

#[derive(Debug, Parser)]
#[command(name = "ise_hammer", about = "Swing the Massive Hammer on ISE parts.")]
struct Args {
    toolchain: PathBuf,
    geomdb: PathBuf,
    parts: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let tc = Toolchain::from_file(&args.toolchain)?;
    let db = GeomDb::from_file(args.geomdb)?;
    for part in &db.devices {
        if !args.parts.is_empty() && !args.parts.contains(&part.name) {
            continue;
        }
        println!("part {name}", name = part.name);
        let gedev = db.expand_grid(part);
        let backend = IseBackend {
            tc: &tc,
            db: &db,
            device: part,
            bs_geom: gedev.bs_geom(),
            egrid: gedev.egrid(),
            edev: &gedev,
        };
        let mut hammer = Session::new(&backend);
        match gedev {
            ExpandedDevice::Xc4k(_) => {}
            ExpandedDevice::Xc5200(_) => {}
            ExpandedDevice::Virtex(_) => {
                clb::virtex::add_fuzzers(&mut hammer, &backend);
            }
            ExpandedDevice::Virtex2(_) => {
                clb::virtex2::add_fuzzers(&mut hammer, &backend);
            }
            ExpandedDevice::Spartan6(_) => {
                clb::virtex5::add_fuzzers(&mut hammer, &backend);
            }
            ExpandedDevice::Virtex4(ref edev) => match edev.kind {
                prjcombine_virtex4::grid::GridKind::Virtex4 => {
                    clb::virtex2::add_fuzzers(&mut hammer, &backend);
                }
                prjcombine_virtex4::grid::GridKind::Virtex5 => {
                    clb::virtex5::add_fuzzers(&mut hammer, &backend);
                }
                prjcombine_virtex4::grid::GridKind::Virtex6 => {
                    clb::virtex5::add_fuzzers(&mut hammer, &backend);
                }
                prjcombine_virtex4::grid::GridKind::Virtex7 => {
                    clb::virtex5::add_fuzzers(&mut hammer, &backend);
                }
            },
            ExpandedDevice::Ultrascale(_) => panic!("ultrascale not supported by ISE"),
            ExpandedDevice::Versal(_) => panic!("versal not supported by ISE"),
        }
        let state = hammer.run().unwrap();
        println!("STATE {state:#?}");
    }
    Ok(())
}
