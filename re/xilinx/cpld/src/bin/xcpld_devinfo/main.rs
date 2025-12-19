mod devinfo;
mod imux;

use clap::Parser;
use prjcombine_re_toolchain::Toolchain;
use prjcombine_re_xilinx_cpld::device::{Device, DeviceKind, Package};
use prjcombine_re_xilinx_cpld::{
    db::{Database, DeviceInfo, Part},
    partgen::get_parts,
};
use rayon::prelude::*;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::{error::Error, path::PathBuf};
use prjcombine_entity::EntityVec;

use crate::imux::gather_imux;

#[derive(Parser)]
struct Args {
    toolchain: PathBuf,
    family: String,
    db: PathBuf,
}

#[derive(Debug)]
pub struct DevInfo {
    dev: Device,
    pkg: Package,
    nds_version: String,
    vm6_family: String,
    vm6_dev: String,
    vm6_devpkg: String,
    vm6_part: String,
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let tc = Toolchain::from_file(&args.toolchain)?;
    let kind = match &*args.family {
        "xc9500" => DeviceKind::Xc9500,
        "xc9500xl" => DeviceKind::Xc9500Xl,
        "xc9500xv" => DeviceKind::Xc9500Xv,
        "xpla3" => DeviceKind::Xpla3,
        "xbr" => DeviceKind::Coolrunner2,
        _ => panic!("unknown family {}", args.family),
    };
    let pkgs = get_parts(&tc, kind)?;
    let mut max_dev: HashMap<usize, (usize, usize)> = HashMap::new();
    let mut max_dev_a: HashMap<(usize, bool), (usize, usize)> = HashMap::new();
    let devinfos: Vec<_> = pkgs
        .par_iter()
        .map(|p| {
            println!("PKG {} {} {:?}", p.device, p.package, p.speedgrades);
            let pname = format!(
                "{dev}{spd}-{pkg}",
                dev = p.device,
                spd = p.speedgrades[0],
                pkg = p.package
            );
            let devinfo = devinfo::get_devinfo(&tc, kind, p, &pname).unwrap();
            assert_eq!(
                devinfo.vm6_part,
                format!(
                    "{d}{s}-{p}",
                    d = p.device,
                    s = p.speedgrades[0],
                    p = p.package
                )
                .to_ascii_uppercase()
            );
            devinfo
        })
        .collect();
    for (i, devinfo) in devinfos.iter().enumerate() {
        let nio = devinfo.dev.io.len();
        match max_dev.entry(devinfo.dev.fbs) {
            Entry::Occupied(mut e) => {
                *e.get_mut() = (*e.get()).max((nio, i));
            }
            Entry::Vacant(e) => {
                e.insert((nio, i));
            }
        }
        match max_dev_a.entry((devinfo.dev.fbs, devinfo.vm6_dev.ends_with('A'))) {
            Entry::Occupied(mut e) => {
                *e.get_mut() = (*e.get()).max((nio, i));
            }
            Entry::Vacant(e) => {
                e.insert((nio, i));
            }
        }
    }
    let mut imux = HashMap::new();
    for (sz, (_, i)) in max_dev {
        let devinfo = &devinfos[i];
        let p = &pkgs[i];
        let pname = format!(
            "{dev}{spd}-{pkg}",
            dev = p.device,
            spd = p.speedgrades[0],
            pkg = p.package
        );
        println!("MAXPKG {} {} {:?}", p.device, p.package, p.speedgrades);
        imux.insert(sz, gather_imux(&tc, &pname, &devinfo.dev, &devinfo.pkg)?);
    }
    let mut devices = EntityVec::new();
    let mut packages = EntityVec::new();
    let mut parts = vec![];
    for (p, devinfo) in pkgs.iter().zip(devinfos.iter()) {
        let mut di = DeviceInfo {
            device: devinfo.dev.clone(),
            imux: imux[&devinfo.dev.fbs].clone(),
        };
        let maxidx = max_dev_a[&(devinfo.dev.fbs, devinfo.vm6_dev.ends_with('A'))].1;
        di.device.io = devinfos[maxidx].dev.io.clone();
        let device = 'a: {
            for (dev, cdi) in &devices {
                if *cdi == di {
                    break 'a dev;
                }
            }
            devices.push(di)
        };
        let package = 'a: {
            for (pkg, cur) in &packages {
                if *cur == devinfo.pkg {
                    break 'a pkg;
                }
            }
            packages.push(devinfo.pkg.clone())
        };
        parts.push(Part {
            device,
            package,
            dev_name: p.device.clone(),
            pkg_name: p.package.clone(),
            speeds: p.speedgrades.clone(),
            nds_version: devinfo.nds_version.clone(),
            vm6_family: devinfo.vm6_family.clone(),
            vm6_dev: devinfo.vm6_dev.clone(),
            vm6_devpkg: devinfo.vm6_devpkg.clone(),
        });
    }
    let db = Database {
        devices,
        packages,
        parts,
    };
    db.to_file(args.db)?;
    Ok(())
}
