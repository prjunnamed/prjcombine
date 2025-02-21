use clap::Parser;
use prjcombine_re_xilinx_ise_dump::partgen::{get_pkgs, PartgenPkg};
use prjcombine_re_xilinx_ise_dump::rawdump::get_rawdump;
use prjcombine_re_toolchain::Toolchain;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use simple_error::{bail, SimpleError};
use std::collections::HashMap;
use std::error::Error;
use std::fs::create_dir_all;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "dump_ise_parts",
    about = "Dump ISE part geometry into rawdump files."
)]
struct Args {
    toolchain: String,
    target_directory: PathBuf,
    families: Vec<String>,
    #[arg(short, long, default_value = "0")]
    num_threads: usize,
}

fn dump_part(
    args: &Args,
    tc: &Toolchain,
    part: String,
    pkgs: Vec<PartgenPkg>,
) -> Result<(), Box<dyn Error>> {
    let fdir = args.target_directory.join(&pkgs[0].family);
    create_dir_all(&fdir)?;
    let path = fdir.join(part.clone() + ".zstd");
    if path.exists() {
        println!("skipping {part}");
    } else {
        println!("dumping {part}");
        let rd = get_rawdump(tc, &pkgs)?;
        rd.to_file(&path)?;
        println!("dumped {part}");
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    ThreadPoolBuilder::new()
        .num_threads(args.num_threads)
        .build_global()
        .unwrap();
    let tc = Toolchain::from_file(&args.toolchain)?;
    let mut ise_families: Vec<&'static str> = Vec::new();
    for family in args.families.iter() {
        ise_families.extend(match &family[..] {
            "xc3000a" => vec!["xc3000a", "xc3000l", "xc3100a", "xc3100l"],
            "xc4000e" => vec!["xc4000e", "xc4000l", "spartan"],
            "xc4000ex" => vec!["xc4000ex", "xc4000xl"],
            "xc4000xla" => vec!["xc4000xla"],
            "xc4000xv" => vec!["xc4000xv"],
            "xc5200" => vec!["xc5200"],
            "spartanxl" => vec!["spartanxl"],
            "virtex" => vec!["virtex", "qvirtex", "qrvirtex", "spartan2"],
            "virtexe" => vec!["virtexe", "qvirtexe", "spartan2e", "aspartan2e"],
            "virtex2" => vec!["virtex2", "qvirtex2", "qrvirtex2"],
            "virtex2p" => vec!["virtex2p", "qvirtex2p"],
            "spartan3" => vec!["spartan3", "aspartan3"],
            "fpgacore" => vec!["fpgacore"],
            "spartan3e" => vec!["spartan3e", "aspartan3e"],
            "spartan3a" => vec!["spartan3a", "aspartan3a"],
            "spartan3adsp" => vec!["spartan3adsp", "aspartan3adsp"],
            "spartan6" => vec![
                "spartan6",
                "spartan6l",
                "aspartan6",
                "qspartan6",
                "qspartan6l",
            ],
            "virtex4" => vec!["virtex4", "qvirtex4", "qrvirtex4"],
            "virtex5" => vec!["virtex5", "qvirtex5"],
            "virtex6" => vec!["virtex6", "virtex6l", "qvirtex6", "qvirtex6l"],
            "virtex7" => vec![
                "spartan7",
                "aspartan7",
                // "artix7",
                // "artix7l",
                // "aartix7",
                // "qartix7",
                // "kintex7",
                // "kintex7l",
                // "qkintex7",
                // "qkintex7l",
                // "virtex7",
                // "qvirtex7",
                // "zynq",
                // "azynq",
                // "qzynq",
            ],
            _ => bail!("unknown family {}", family),
        });
    }
    create_dir_all(&args.target_directory)?;
    let mut parts: HashMap<String, Vec<PartgenPkg>> = HashMap::new();
    for ise_fam in ise_families.iter() {
        println!("querying {ise_fam}");
    }
    let pkg_list: Vec<_> = ise_families
        .into_par_iter()
        .map(|ise_fam| get_pkgs(&tc, ise_fam).map_err(|x| SimpleError::new(x.to_string())))
        .collect();
    for pkgs in pkg_list {
        for pkg in pkgs? {
            match parts.get_mut(&pkg.device) {
                None => {
                    parts.insert(pkg.device.to_string(), vec![pkg]);
                }
                Some(v) => {
                    v.push(pkg);
                }
            }
        }
    }
    for (part, pkgs) in parts.iter() {
        println!(
            "device {} [{}]: {}",
            part,
            pkgs[0].family,
            pkgs.iter()
                .fold(String::new(), |acc, pkg| acc + &pkg.package + ", ")
        );
    }
    for res in parts
        .into_par_iter()
        .map(|(part, pkgs)| {
            dump_part(&args, &tc, part, pkgs).map_err(|x| SimpleError::new(x.to_string()))
        })
        .collect::<Vec<_>>()
    {
        res?;
    }
    Ok(())
}
