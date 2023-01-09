use rayon::prelude::*;
use std::fs::{create_dir_all, File};
use std::{error::Error, path::PathBuf};
use structopt::StructOpt;

use prjcombine_xilinx_geom::{ExpandedDevice, GeomDb};

#[derive(Debug, StructOpt)]
#[structopt(name = "xg2svg", about = "Pretty-draw xilinx geometry.")]
struct Opt {
    file: String,
    dest_dir: PathBuf,
}

mod drawer;
mod spartan6;
mod ultrascale;
mod virtex2;
mod virtex4;

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let geom = GeomDb::from_file(opt.file)?;
    create_dir_all(&opt.dest_dir)?;
    geom.devices
        .par_iter()
        .try_for_each(|dev| -> Result<(), std::io::Error> {
            let drawer = match geom.expand_grid(dev) {
                ExpandedDevice::Virtex2(edev) => virtex2::draw_device(&dev.name, edev),
                ExpandedDevice::Virtex4(edev) => virtex4::draw_device(&dev.name, edev),
                ExpandedDevice::Spartan6(edev) => spartan6::draw_device(&dev.name, edev),
                ExpandedDevice::Ultrascale(edev) => ultrascale::draw_device(&dev.name, edev),
                _ => todo!(),
            };
            let fname = opt.dest_dir.join(format!("{n}.html", n = dev.name));
            let f = File::create(fname)?;
            drawer.emit(f)?;
            Ok(())
        })?;
    Ok(())
}
