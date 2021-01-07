use std::env;
use std::io;
use std::path::Path;
use std::fs::create_dir_all;
use std::collections::HashMap;
use rayon::prelude::*;
use prjcombine::xilinx::ise::rawdump::get_rawdump;
use prjcombine::xilinx::ise::partgen::{get_pkgs, PartgenPkg};
use prjcombine::xilinx::toolchain::Toolchain;

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = env::args().collect();
    let tc = Toolchain::from_file(&args[1])?;
    let mut ise_families: Vec<&'static str> = Vec::new();
    for family in &args[3..] {
        ise_families.extend(match &family[..] {
            "xc4000e" => vec!["xc4000e", "xc4000l", "spartan"],
            "xc4000ex" => vec!["xc4000ex", "xc4000xl"],
            "xc4000xla" => vec!["xc4000xla"],
            "xc4000xv" => vec!["xc4000xv"],
            "spartanxl" => vec!["spartanxl"],
            "virtex" => vec!["virtex", "qvirtex", "qrvirtex", "spartan2"],
            "virtexe" => vec!["virtexe", "qvirtexe", "spartan2e", "aspartan2e"],
            "virtex2" => vec!["virtex2", "qvirtex2", "qrvirtex2"],
            "virtex2p" => vec!["virtex2p", "qvirtex2p"],
            "spartan3" => vec!["spartan3", "aspartan3"],
            "spartan3e" => vec!["spartan3e", "aspartan3e"],
            "spartan3a" => vec!["spartan3a", "aspartan3a"],
            "spartan3adsp" => vec!["spartan3adsp", "aspartan3adsp"],
            "spartan6" => vec!["spartan6", "spartan6l", "aspartan6", "qspartan6", "qspartan6l"],
            "virtex4" => vec!["virtex4", "qvirtex4", "qrvirtex4"],
            "virtex5" => vec!["virtex5", "qvirtex5"],
            "virtex6" => vec!["virtex6", "virtex6l", "qvirtex6", "qvirtex6l"],
            "series7" => vec![
                "artix7", "artix7l", "aartix7", "qartix7",
                "kintex7", "kintex7l", "qkintex7", "qkintex7l",
                "virtex7", "qvirtex7",
                "zynq", "azynq", "qzynq",
            ],
            _ => return Err(io::Error::new(io::ErrorKind::Other, format!("unknown family {}", family))),
        });
    };
    create_dir_all(&args[2])?;
    let mut parts: HashMap<String, Vec<PartgenPkg>> = HashMap::new();
    for ise_fam in ise_families.iter() {
        println!("querying {}", ise_fam);
    }
    let pkg_list: Vec<_> = ise_families.into_par_iter().map(|ise_fam| get_pkgs(&tc, ise_fam)).collect();
    for pkgs in pkg_list {
        for pkg in pkgs? {
            match parts.get_mut(&pkg.device) {
                None => { parts.insert(pkg.device.to_string(), vec![pkg]); },
                Some(v) => { v.push(pkg); },
            }
        }
    }
    for (part, pkgs) in parts.iter() {
        println!("device {} [{}]: {}", part, pkgs[0].family, pkgs.iter().fold(String::new(), |acc, pkg| acc + &pkg.package + ", "));
    }
    for res in parts.into_par_iter().map(|(part, pkgs)| -> Result<(), io::Error> {
        println!("dumping {}", part);
        let fdir = Path::new(&args[2]).join(&pkgs[0].family);
        create_dir_all(&fdir)?;
        let rd = get_rawdump(&tc, &pkgs)?;
        let path = fdir.join(part.clone() + ".xz");
        rd.to_file(&path)?;
        println!("dumped {}", part);
        Ok(())
    }).collect::<Vec<_>>() {
        res?;
    }
    Ok(())
}
