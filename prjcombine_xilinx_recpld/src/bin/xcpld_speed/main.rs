mod extract;
mod vm6_emit;
mod xbr;
mod xc9500;
mod xpla3;

use std::{collections::HashSet, error::Error, path::PathBuf};

use clap::Parser;
use prjcombine_toolchain::Toolchain;
use prjcombine_xilinx_cpld::device::DeviceKind;
use prjcombine_xilinx_recpld::db::Database;

#[derive(Debug, Parser)]
struct Args {
    toolchain: PathBuf,
    db: PathBuf,
    device: Option<String>,
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let tc = Toolchain::from_file(&args.toolchain)?;
    let db = Database::from_file(args.db)?;
    let mut done = HashSet::new();
    for part in &db.parts {
        if let Some(ref filter) = args.device {
            if *filter != part.dev_name
                && *filter != format!("{d}{p}", d = part.dev_name, p = part.pkg_name)
            {
                continue;
            }
        }
        let device = &db.devices[part.device];
        let package = &db.packages[part.package];
        if done.contains(&part.dev_name) {
            continue;
        }
        done.insert(part.dev_name.clone());
        for spd in &part.speeds {
            println!("DEV {d} {p} {spd}", d = part.dev_name, p = part.pkg_name);
            match device.device.kind {
                DeviceKind::Xc9500 | DeviceKind::Xc9500Xl | DeviceKind::Xc9500Xv => {
                    let timing = xc9500::test_xc9500(&tc, part, &device.device, package, spd);
                    println!("TIMING: {timing:#?}");
                }
                DeviceKind::Xpla3 => {
                    let timing = xpla3::test_xpla3(&tc, part, &device.device, package, spd);
                    println!("TIMING: {timing:#?}");
                }
                DeviceKind::Coolrunner2 => {
                    let timing = xbr::test_xbr(&tc, part, &device.device, package, spd);
                    println!("TIMING: {timing:#?}");
                }
            }
        }
    }
    Ok(())
}
