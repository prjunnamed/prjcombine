use std::env;
use std::io;
use std::path::Path;
use std::fs::create_dir_all;
use std::collections::{HashMap, HashSet};
use rayon::prelude::*;
use prjcombine::xilinx::vivado::rawdump::get_rawdump;
use prjcombine::xilinx::vivado::parts::{get_parts, VivadoPart};
use prjcombine::toolchain::Toolchain;

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = env::args().collect();
    let tc = Toolchain::from_file(&args[1])?;
    let families: HashSet<_> = args[3..].iter().map(|s| s.to_string()).collect();
    create_dir_all(&args[2])?;
    let mut parts: HashMap<String, Vec<VivadoPart>> = HashMap::new();
    for part in get_parts(&tc)? {
        if !families.contains(&part.actual_family) && !families.contains(&part.device) {
            continue;
        }
        match parts.get_mut(&part.device) {
            None => { parts.insert(part.device.to_string(), vec![part]); },
            Some(v) => { v.push(part); },
        }
    }
    for (dev, devparts) in parts.iter() {
        println!("device {} [{}]: {}", dev, devparts[0].actual_family, devparts.iter().fold(String::new(), |acc, dp| acc + &dp.name + ", "));
    }
    for res in parts.into_par_iter().map(|(dev, devparts)| -> Result<(), io::Error> {
        println!("dumping {}", dev);
        let fdir = Path::new(&args[2]).join(&devparts[0].actual_family);
        create_dir_all(&fdir)?;
        let rd = get_rawdump(&tc, &devparts)?;
        let path = fdir.join(dev.clone() + ".xz");
        rd.to_file(&path)?;
        println!("dumped {}", dev);
        Ok(())
    }).collect::<Vec<_>>() {
        res?;
    }
    Ok(())
}

