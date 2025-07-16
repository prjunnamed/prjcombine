mod extract;
mod vm6_emit;
mod xbr;
mod xc9500;
mod xpla3;

use std::sync::Mutex;
use std::{collections::HashSet, error::Error, path::PathBuf};

use clap::Parser;
use prjcombine_re_toolchain::Toolchain;
use prjcombine_re_xilinx_cpld::device::DeviceKind;
use prjcombine_re_xilinx_cpld::{
    db::Database,
    speeddb::{SpeedDb, SpeedDbPart},
};

#[derive(Debug, Parser)]
struct Args {
    toolchain: PathBuf,
    db: PathBuf,
    sdb: PathBuf,
    device: Option<String>,
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let tc = Toolchain::from_file(&args.toolchain)?;
    let db = Database::from_file(args.db)?;
    let parts = Mutex::new(vec![]);
    let mut done = HashSet::new();
    rayon::scope(|scope| {
        for part in &db.parts {
            if let Some(ref filter) = args.device
                && *filter != part.dev_name
                && *filter != format!("{d}{p}", d = part.dev_name, p = part.pkg_name)
            {
                continue;
            }
            let device = &db.devices[part.device];
            let package = &db.packages[part.package];
            if done.contains(&part.dev_name) {
                continue;
            }
            done.insert(part.dev_name.clone());
            for spd in &part.speeds {
                let tc = &tc;
                let parts = &parts;
                scope.spawn(move |_| {
                    println!("DEV {d} {p} {spd}", d = part.dev_name, p = part.pkg_name);
                    let speed = match device.device.kind {
                        DeviceKind::Xc9500 | DeviceKind::Xc9500Xl | DeviceKind::Xc9500Xv => {
                            xc9500::test_xc9500(tc, part, &device.device, package, spd)
                        }
                        DeviceKind::Xpla3 => {
                            xpla3::test_xpla3(tc, part, &device.device, package, spd)
                        }
                        DeviceKind::Coolrunner2 => {
                            xbr::test_xbr(tc, part, &device.device, package, spd)
                        }
                    };
                    parts.lock().unwrap().push(SpeedDbPart {
                        dev_name: part.dev_name.clone(),
                        speed_name: spd.clone(),
                        speed,
                    })
                });
            }
        }
    });
    let parts = parts.into_inner().unwrap();
    let sdb = SpeedDb { parts };
    sdb.to_file(args.sdb)?;
    Ok(())
}
