use std::io;
use std::path::PathBuf;
use std::fs::create_dir_all;
use std::collections::{HashMap, HashSet};
use structopt::StructOpt;
use rayon::prelude::*;
use prjcombine::xilinx::vivado::rawdump::get_rawdump;
use prjcombine::xilinx::vivado::parts::{get_parts, VivadoPart};
use prjcombine::toolchain::Toolchain;

#[derive(Debug, StructOpt)]
#[structopt(name = "dump_vivado_parts", about = "Dump Vivado part geometry into rawdump files.")]
struct Opt {
    toolchain: String,
    #[structopt(parse(from_os_str))]
    target_directory: PathBuf,
    families: Vec<String>,
}

fn main() -> Result<(), io::Error> {
    let opt = Opt::from_args();
    let tc = Toolchain::from_file(&opt.toolchain)?;
    let families: HashSet<_> = opt.families.iter().map(|s| s.to_string()).collect();
    create_dir_all(&opt.target_directory)?;
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
        let fdir = opt.target_directory.join(&devparts[0].actual_family);
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

