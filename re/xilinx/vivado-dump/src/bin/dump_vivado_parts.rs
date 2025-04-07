use clap::Parser;
use prjcombine_re_toolchain::Toolchain;
use prjcombine_re_xilinx_vivado_dump::parts::{VivadoPart, get_parts};
use prjcombine_re_xilinx_vivado_dump::rawdump::get_rawdump;
use rayon::ThreadPoolBuilder;
use std::collections::{HashMap, HashSet};
use std::fs::create_dir_all;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "dump_vivado_parts",
    about = "Dump Vivado part geometry into rawdump files."
)]
struct Args {
    toolchain: PathBuf,
    target_directory: PathBuf,
    families: Vec<String>,
    #[arg(long)]
    parts: Vec<String>,
    #[arg(short, long, default_value = "0")]
    num_threads: usize,
}

fn dump_part(args: &Args, tc: &Toolchain, dev: String, devparts: Vec<VivadoPart>) {
    let fdir = args.target_directory.join(&devparts[0].actual_family);
    create_dir_all(&fdir).unwrap();
    let path = fdir.join(dev.clone() + ".zstd");
    if path.exists() {
        println!("skipping {dev}");
    } else {
        println!("dumping {dev}");
        let rd = get_rawdump(tc, &devparts).unwrap();
        rd.to_file(&path).unwrap();
        println!("dumped {dev}");
    }
}

fn main() {
    let args = Args::parse();
    ThreadPoolBuilder::new()
        .num_threads(args.num_threads)
        .build_global()
        .unwrap();
    let tc = Toolchain::from_file(&args.toolchain).unwrap();
    let families: HashSet<_> = args.families.iter().map(|s| s.to_string()).collect();
    create_dir_all(&args.target_directory).unwrap();
    let mut parts: HashMap<String, Vec<VivadoPart>> = HashMap::new();
    for part in get_parts(&tc).unwrap() {
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
    for (dev, devparts) in parts {
        if !args.parts.is_empty() && !args.parts.contains(&dev) {
            continue;
        }
        dump_part(&args, &tc, dev, devparts);
    }
}
