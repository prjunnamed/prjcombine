use prjcombine_toolchain::Toolchain;
use prjcombine_vivado_dump::parts::{get_parts, VivadoPart};
use prjcombine_vivado_dump::rawdump::get_rawdump;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use simple_error::SimpleError;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::create_dir_all;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "dump_vivado_parts",
    about = "Dump Vivado part geometry into rawdump files."
)]
struct Opt {
    toolchain: String,
    #[structopt(parse(from_os_str))]
    target_directory: PathBuf,
    families: Vec<String>,
    #[structopt(short = "n", long, default_value = "0")]
    num_threads: usize,
}

fn dump_part(
    opt: &Opt,
    tc: &Toolchain,
    dev: String,
    devparts: Vec<VivadoPart>,
) -> Result<(), Box<dyn Error>> {
    let fdir = opt.target_directory.join(&devparts[0].actual_family);
    create_dir_all(&fdir)?;
    let path = fdir.join(dev.clone() + ".zstd");
    if path.exists() {
        println!("skipping {}", dev);
    } else {
        println!("dumping {}", dev);
        let rd = get_rawdump(&tc, &devparts)?;
        rd.to_file(&path)?;
        println!("dumped {}", dev);
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    ThreadPoolBuilder::new()
        .num_threads(opt.num_threads)
        .build_global()
        .unwrap();
    let tc = Toolchain::from_file(&opt.toolchain)?;
    let families: HashSet<_> = opt.families.iter().map(|s| s.to_string()).collect();
    create_dir_all(&opt.target_directory)?;
    let mut parts: HashMap<String, Vec<VivadoPart>> = HashMap::new();
    for part in get_parts(&tc)? {
        if !families.contains(&part.actual_family) && !families.contains(&part.device) {
            continue;
        }
        match parts.get_mut(&part.device) {
            None => {
                parts.insert(part.device.to_string(), vec![part]);
            }
            Some(v) => {
                v.push(part);
            }
        }
    }
    for (dev, devparts) in parts.iter() {
        println!(
            "device {} [{}]: {}",
            dev,
            devparts[0].actual_family,
            devparts
                .iter()
                .fold(String::new(), |acc, dp| acc + &dp.name + ", ")
        );
    }
    for res in parts
        .into_par_iter()
        .map(|(dev, devparts)| {
            dump_part(&opt, &tc, dev, devparts).map_err(|x| SimpleError::new(x.to_string()))
        })
        .collect::<Vec<_>>()
    {
        res?;
    }
    Ok(())
}
