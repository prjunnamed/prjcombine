#![allow(clippy::type_complexity)]

use clap::Parser;
use prjcombine_re_lattice_dump::parse_tiles;
use prjcombine_re_lattice_rawdump::{Db, Grid, Node, Part, Pip, Site};
use prjcombine_re_toolchain::Toolchain;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::fs::create_dir_all;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::OnceLock;
use unnamed_entity::{EntityMap, EntitySet, EntityVec};

struct FamilyInfo {
    name: &'static str,
    parts: &'static [(&'static str, &'static str)],
    pkgs: &'static [&'static str],
    speeds: &'static [&'static str],
}

const DIAMOND_FAMILIES: &[FamilyInfo] = &[
    FamilyInfo {
        name: "ecp",
        parts: &[
            ("ep5g00", "LFEC1E"),
            ("ep5g00", "LFEC3E"),
            ("ep5g00", "LFEC6E"),
            ("ep5g00", "LFEC10E"),
            ("ep5g00", "LFEC15E"),
            ("ep5g00", "LFEC20E"),
            ("ep5g00", "LFEC33E"),
            ("ep5g00p", "LFECP6E"),
            ("ep5g00p", "LFECP10E"),
            ("ep5g00p", "LFECP15E"),
            ("ep5g00p", "LFECP20E"),
            ("ep5g00p", "LFECP33E"),
        ],
        pkgs: &[
            "TQFP100", "TQFP144", "PQFP208", "FPBGA256", "FPBGA484", "FPBGA672",
        ],
        speeds: &["3", "4", "5"],
    },
    FamilyInfo {
        name: "xp",
        parts: &[
            ("mg5g00", "LFXP3C"),
            ("mg5g00", "LFXP3E"),
            ("mg5g00", "LFXP6C"),
            ("mg5g00", "LFXP6E"),
            ("mg5g00", "LFXP10C"),
            ("mg5g00", "LFXP10E"),
            ("mg5g00", "LFXP15C"),
            ("mg5g00", "LFXP15E"),
            ("mg5g00", "LFXP20C"),
            ("mg5g00", "LFXP20E"),
        ],
        pkgs: &[
            "TQFP100", "TQFP144", "PQFP208", "FPBGA256", "FPBGA388", "FPBGA484",
        ],
        speeds: &["3", "4", "5"],
    },
    FamilyInfo {
        name: "machxo",
        parts: &[
            ("mj5g00", "LAMXO256C"),
            ("mj5g00", "LAMXO256E"),
            ("mj5g00", "LCMXO256C"),
            ("mj5g00", "LCMXO256E"),
            ("mj5g00", "LAMXO640C"),
            ("mj5g00", "LAMXO640E"),
            ("mj5g00", "LCMXO640C"),
            ("mj5g00", "LCMXO640E"),
            ("mj5g00", "LAMXO1200E"),
            ("mj5g00", "LCMXO1200C"),
            ("mj5g00", "LCMXO1200E"),
            ("mj5g00", "LAMXO2280E"),
            ("mj5g00", "LCMXO2280C"),
            ("mj5g00", "LCMXO2280E"),
            ("mj5g00p", "LPTM10-1247"),
            ("mj5g00p", "LPTM10-12107"),
        ],
        pkgs: &[
            "TQFP100", "TQFP128", "TQFP144", "CSBGA100", "CSBGA132", "CABGA256", "FTBGA208",
            "FTBGA256", "FTBGA324",
        ],
        speeds: &["3", "4", "5"],
    },
    FamilyInfo {
        name: "scm",
        parts: &[
            ("slayer", "LFSC3GA15E"),
            ("slayer", "LFSC3GA25E"),
            ("slayer", "LFSC3GA40E"),
            ("slayer", "LFSC3GA80E"),
            ("slayer", "LFSC3GA115E"),
            ("or5s00", "LFSCM3GA15EP1"),
            ("or5s00", "LFSCM3GA25EP1"),
            ("or5s00", "LFSCM3GA40EP1"),
            ("or5s00", "LFSCM3GA80EP1"),
            ("or5s00", "LFSCM3GA115EP1"),
        ],
        pkgs: &[
            "FPBGA256",
            "FPBGA900",
            "FFBGA1020",
            "FFABGA1020",
            "FCBGA1152",
            "FFBGA1152",
            "FCBGA1704",
            "FFBGA1704",
        ],
        speeds: &["5", "6", "7"],
    },
    FamilyInfo {
        name: "ecp2",
        parts: &[
            ("ep5a00", "LFE2-6E"),
            ("ep5a00", "LFE2-12E"),
            ("ep5a00", "LFE2-20E"),
            ("ep5a00", "LFE2-35E"),
            ("ep5a00", "LFE2-50E"),
            ("ep5a00", "LFE2-70E"),
        ],
        pkgs: &[
            "TQFP144", "PQFP208", "FPBGA256", "FPBGA484", "FPBGA672", "FPBGA900",
        ],
        speeds: &["5", "6", "7"],
    },
    FamilyInfo {
        name: "ecp2m",
        parts: &[
            ("ep5m00", "LFE2M20E"),
            ("ep5m00", "LFE2M35E"),
            ("ep5m00", "LFE2M50E"),
            ("ep5m00", "LFE2M70E"),
            ("ep5m00", "LFE2M100E"),
        ],
        pkgs: &["FPBGA256", "FPBGA484", "FPBGA672", "FPBGA900", "FPBGA1152"],
        speeds: &["5", "6", "7"],
    },
    FamilyInfo {
        name: "xp2",
        parts: &[
            ("mg5a00", "LAXP2-5E"),
            ("mg5a00", "LFXP2-5E"),
            ("mg5a00", "LAXP2-8E"),
            ("mg5a00", "LFXP2-8E"),
            ("mg5a00", "LAXP2-17E"),
            ("mg5a00", "LFXP2-17E"),
            ("mg5a00", "LFXP2-30E"),
            ("mg5a00", "LFXP2-40E"),
        ],
        pkgs: &[
            "TQFP144", "PQFP208", "CSBGA132", "FTBGA256", "FPBGA484", "FPBGA672",
        ],
        speeds: &["5", "6", "7"],
    },
    FamilyInfo {
        name: "ecp3",
        parts: &[
            ("ep5c00", "LAE3-17EA"),
            ("ep5c00", "LFE3-17EA"),
            ("ep5c00", "LAE3-35EA"),
            ("ep5c00", "LFE3-35EA"),
            ("ep5c00", "LFE3-70E"),
            ("ep5c00", "LFE3-70EA"),
            ("ep5c00", "LFE3-95E"),
            ("ep5c00", "LFE3-95EA"),
            ("ep5c00", "LFE3-150EA"),
        ],
        pkgs: &["CSBGA328", "FTBGA256", "FPBGA484", "FPBGA672", "FPBGA1156"],
        speeds: &["6", "6L", "7", "7L", "8", "8L", "9"],
    },
    FamilyInfo {
        name: "ecp4",
        parts: &[
            ("ep5d00", "LFE4-30E"),
            ("ep5d00", "LFE4-50E"),
            ("ep5d00", "LFE4-95E"),
            ("ep5d00", "LFE4-130E"),
            ("ep5d00", "LFE4-190E"),
        ],
        pkgs: &[
            "FPBGA484",
            "FPBGA648",
            "FPBGA868",
            "FCBGA676",
            "FCBGA900",
            "FCBGA1152",
        ],
        speeds: &["7", "8", "9"],
    },
    FamilyInfo {
        name: "ecp5",
        parts: &[
            ("sa5p00", "LAE5U-12F"),
            ("sa5p00", "LFE5U-12F"),
            ("sa5p00", "LFE5U-25F"),
            ("sa5p00", "LFE5U-45F"),
            ("sa5p00", "LFE5U-85F"),
            ("sa5p00m", "LAE5UM-25F"),
            ("sa5p00m", "LFE5UM-25F"),
            ("sa5p00m", "LAE5UM-45F"),
            ("sa5p00m", "LFE5UM-45F"),
            ("sa5p00m", "LAE5UM-85F"),
            ("sa5p00m", "LFE5UM-85F"),
            ("sa5p00g", "LFE5UM5G-25F"),
            ("sa5p00g", "LFE5UM5G-45F"),
            ("sa5p00g", "LFE5UM5G-85F"),
        ],
        pkgs: &[
            "TQFP144",
            "CABGA256",
            "CABGA381",
            "CABGA554",
            "CABGA756",
            "CSFBGA285",
        ],
        speeds: &["6", "7", "8"],
    },
    FamilyInfo {
        name: "crosslink",
        parts: &[
            ("sn5w00", "LIA-MD6000"),
            ("sn5w00", "LIF-MD6000"),
            ("wi5s00", "LIA-MDF6000"),
            ("wi5s00", "LIF-MDF6000"),
        ],
        pkgs: &["WLCSP36", "UCFBGA64", "CKFBGA80", "CTFBGA80", "CSFBGA81"],
        speeds: &["6"],
    },
    FamilyInfo {
        name: "machxo2",
        parts: &[
            ("xo2c00", "LCMXO2-256HC"),
            ("xo2c00", "LCMXO2-256ZE"),
            ("xo2c00", "LCMXO2-640HC"),
            ("xo2c00", "LCMXO2-640ZE"),
            ("xo2c00", "LCMXO2-640UHC"),
            ("xo2c00", "LCMXO2-1200HC"),
            ("xo2c00", "LCMXO2-1200ZE"),
            ("xo2c00", "LCMXO2-1200UHC"),
            ("xo2c00", "LCMXO2-2000HC"),
            ("xo2c00", "LCMXO2-2000ZE"),
            ("xo2c00", "LCMXO2-2000UHC"),
            ("xo2c00", "LCMXO2-2000UHE"),
            ("xo2c00", "LCMXO2-4000HC"),
            ("xo2c00", "LCMXO2-4000HE"),
            ("xo2c00", "LCMXO2-4000ZE"),
            ("xo2c00", "LCMXO2-4000UHC"),
            ("xo2c00", "LCMXO2-7000HC"),
            ("xo2c00", "LCMXO2-7000HE"),
            ("xo2c00", "LCMXO2-7000ZE"),
            ("xo2c00", "LCMXO2-10000HC"),
            ("xo2c00", "LCMXO2-10000HE"),
            ("xo2c00", "LCMXO2-10000ZE"),
            ("xo2c00p", "LPTM21"),
            ("xo2c00p", "LPTM21L"),
            ("xo3c00a", "LCMXO3L-640E"),
            ("xo3c00a", "LCMXO3L-1300C"),
            ("xo3c00a", "LCMXO3L-1300E"),
            ("xo3c00a", "LCMXO3L-2100C"),
            ("xo3c00a", "LCMXO3L-2100E"),
            ("xo3c00a", "LCMXO3L-4300C"),
            ("xo3c00a", "LCMXO3L-4300E"),
            ("xo3c00a", "LCMXO3L-6900C"),
            ("xo3c00a", "LCMXO3L-6900E"),
            ("xo3c00a", "LCMXO3L-9400C"),
            ("xo3c00a", "LCMXO3L-9400E"),
            ("xo3c00f", "LCMXO3LF-640E"),
            ("xo3c00f", "LAMXO3LF-1300C"),
            ("xo3c00f", "LAMXO3LF-1300E"),
            ("xo3c00f", "LCMXO3LF-1300C"),
            ("xo3c00f", "LCMXO3LF-1300E"),
            ("xo3c00f", "LAMXO3LF-2100C"),
            ("xo3c00f", "LAMXO3LF-2100E"),
            ("xo3c00f", "LCMXO3LF-2100C"),
            ("xo3c00f", "LCMXO3LF-2100E"),
            ("xo3c00f", "LAMXO3LF-4300C"),
            ("xo3c00f", "LAMXO3LF-4300E"),
            ("xo3c00f", "LCMXO3LF-4300C"),
            ("xo3c00f", "LCMXO3LF-4300E"),
            ("xo3c00f", "LCMXO3LF-6900C"),
            ("xo3c00f", "LCMXO3LF-6900E"),
            ("xo3c00f", "LCMXO3LF-9400C"),
            ("xo3c00f", "LCMXO3LF-9400E"),
            ("xo3c00d", "LCMXO3LFP-4300HC"),
            ("xo3c00d", "LCMXO3LFP-6900HC"),
            ("xo3c00d", "LCMXO3LFP-9400HC"),
            ("se5c00", "LAMXO3D-4300HC"),
            ("se5c00", "LAMXO3D-4300ZC"),
            ("se5c00", "LCMXO3D-4300HC"),
            ("se5c00", "LCMXO3D-4300HE"),
            ("se5c00", "LCMXO3D-4300ZC"),
            ("se5c00", "LCMXO3D-4300ZE"),
            ("se5c00", "LAMXO3D-9400HE"),
            ("se5c00", "LAMXO3D-9400ZC"),
            ("se5c00", "LCMXO3D-9400HC"),
            ("se5c00", "LCMXO3D-9400HE"),
            ("se5c00", "LCMXO3D-9400ZC"),
            ("se5c00", "LCMXO3D-9400ZE"),
            ("se5r00", "LFMNX-50"),
        ],
        pkgs: &[
            // MachXO2
            "WLCSP25",
            "WLCSP36",
            "WLCSP49",
            "WLCSP69",
            "WLCSP81",
            "QFN32",
            "QFN48",
            "QFN84",
            "TQFP100",
            "TQFP144",
            "UCBGA64",
            "CSBGA132",
            "CSBGA184",
            "CABGA256",
            "CABGA332",
            "CABGA400",
            "FTBGA256",
            "FPBGA484",
            // MachXO3
            "QFN72",
            "CSFBGA121",
            "CSFBGA256",
            "CSFBGA324",
            "CABGA324",
            "CABGA484",
            // LPTM21
            "FTBGA237",
            "CABGA100",
            // Mach-NX
            "CBG256",
            "FBG484",
            "LBG484",
        ],
        speeds: &["1", "1A", "2", "3", "4", "5", "6"],
    },
];

#[derive(Debug, Parser)]
#[command(
    name = "dump_diamond_parts",
    about = "Dump Diamond part geometry into rawdump files."
)]
struct Args {
    toolchain: String,
    target_directory: PathBuf,
    families: Vec<String>,
    #[arg(short, long, default_value = "0")]
    num_threads: usize,
}

struct PrePart {
    pub arch: String,
    pub name: String,
    pub package: String,
    pub speeds: Vec<String>,
    pub grid: Grid,
    pub sites: Vec<Site>,
}

fn dump_pre(
    tc: &Toolchain,
    family: &FamilyInfo,
    arch: &str,
    part: &str,
    pkg: &str,
) -> Option<PrePart> {
    static SITE_RE: OnceLock<Regex> = OnceLock::new();
    static PIP_RE: OnceLock<Regex> = OnceLock::new();
    let site_re = SITE_RE.get_or_init(|| Regex::new(r"Site=([A-Za-z0-9_]+) type=XXX$").unwrap());
    let pip_re = PIP_RE.get_or_init(|| Regex::new(r"from ([A-Z0-9_]+) to ([A-Z0-9_]+)$").unwrap());

    let mut speeds = vec![];
    let mut grid = None;
    let mut sites = None;
    for speed in family.speeds {
        let dir = tempfile::Builder::new()
            .prefix("prjcombine_diamond_dump")
            .tempdir()
            .unwrap();
        let mut file = File::create(dir.path().join("top.ncl")).unwrap();
        write!(
            file,
            r#"
::FROM-WRITER;
design top
{{
   device
   {{
      architecture {arch};
      device {part};
      package {pkg};
      performance "{speed}";
   }}
}}
        "#
        )
        .unwrap();
        std::mem::drop(file);
        let mut cmd = tc.command("ncl2ncd");
        cmd.current_dir(dir.path().as_os_str());
        cmd.stdin(Stdio::null());
        cmd.arg("top.ncl");
        let status = cmd.output().unwrap();
        if !status.status.success() {
            let stdout = std::str::from_utf8(&status.stdout).unwrap();
            let stderr = std::str::from_utf8(&status.stderr).unwrap();
            if !stderr.contains("Invalid package name")
                && !stderr.contains("Invalid performance")
                && !stdout.contains("Invalid package name")
                && !stdout.contains("Invalid performance")
            {
                eprintln!("STDOUT");
                let _ = std::io::stderr().write_all(&status.stdout);
                eprintln!("STDERR");
                let _ = std::io::stderr().write_all(&status.stderr);
                panic!("non-zero ncl2ncd exit status");
            }
            continue;
        }
        speeds.push(speed.to_string());
        if grid.is_some() {
            continue;
        }
        let mut cmd = tc.command("bitgen");
        cmd.current_dir(dir.path().as_os_str());
        cmd.stdin(Stdio::null());
        cmd.arg("-d");
        cmd.arg("top.ncd");
        let status = cmd.output().unwrap();
        if !status.status.success() {
            let _ = std::io::stderr().write_all(&status.stdout);
            let _ = std::io::stderr().write_all(&status.stderr);
            panic!("non-zero bitgen exit status");
        }
        let mut cmd = tc.command("bstool");
        cmd.current_dir(dir.path().as_os_str());
        cmd.stdin(Stdio::null());
        cmd.arg("-t");
        cmd.arg("top.bit");
        let status = cmd.output().unwrap();
        if !status.status.success() {
            let _ = std::io::stderr().write_all(&status.stdout);
            let _ = std::io::stderr().write_all(&status.stderr);
            panic!("non-zero bstool exit status");
        }
        let tiles = parse_tiles(std::str::from_utf8(&status.stdout).unwrap(), family.name);
        File::create(dir.path().join("meow.prf")).unwrap();

        let mut cmd = tc.command("ispTcl");
        cmd.current_dir(dir.path().as_os_str());
        let mut file = File::create(dir.path().join("meow.tcl")).unwrap();
        write!(
            file,
            r#"
des_read_ncd top.ncd
des_read_prf meow.prf
basciCmdListSite 1 *
basciCmdListNode 1 nodes.out *
        "#
        )
        .unwrap();
        std::mem::drop(file);
        cmd.stdin(File::open(dir.path().join("meow.tcl")).unwrap());
        cmd.stdout(Stdio::null());
        let status = cmd.output().unwrap();
        if !status.status.success() {
            let _ = std::io::stderr().write_all(&status.stdout);
            let _ = std::io::stderr().write_all(&status.stderr);
            panic!("non-zero ispTcl exit status");
        }
        let mut nodes = EntityMap::new();
        let mut cur_sites = Vec::new();
        let mut pips = HashMap::new();
        let file = File::open(dir.path().join("nodes.out")).unwrap();
        for line in BufReader::new(file).lines() {
            let line = line.unwrap();
            nodes.insert(
                line,
                Node {
                    aliases: vec![],
                    typ: None,
                    pin: None,
                },
            );
        }
        let file = File::open(dir.path().join("ispTcl.log")).unwrap();
        for line in BufReader::new(file).lines() {
            let line = line.unwrap();
            if let Some(cap) = site_re.captures(&line) {
                cur_sites.push(Site {
                    name: cap[1].to_string(),
                    typ: None,
                });
            } else if line.starts_with("Site") {
                panic!("regex failed: {line:?}");
            }
        }
        for i in 0..32 {
            let mut cmd = tc.command("ispTcl");
            cmd.current_dir(dir.path().as_os_str());
            std::fs::remove_file(dir.path().join("ispTcl.log")).unwrap();
            let cmd_file = format!("pip{i}.tcl");
            let mut file = File::create(dir.path().join(&cmd_file)).unwrap();
            write!(
                file,
                r#"
    des_read_ncd top.ncd
    des_read_prf meow.prf
    basciCmdListArcByNodeType {i} -1 1000000000
            "#
            )
            .unwrap();
            std::mem::drop(file);
            cmd.stdin(File::open(dir.path().join(cmd_file)).unwrap());
            cmd.stdout(Stdio::null());
            let status = cmd.output().unwrap();
            if !status.status.success() {
                let _ = std::io::stderr().write_all(&status.stdout);
                let _ = std::io::stderr().write_all(&status.stderr);
                panic!("non-zero ispTcl exit status");
            }
            let file = File::open(dir.path().join("ispTcl.log")).unwrap();
            for line in BufReader::new(file).lines() {
                let line = line.unwrap();
                if let Some(cap) = pip_re.captures(&line) {
                    pips.insert(
                        (nodes.get(&cap[1]).unwrap().0, nodes.get(&cap[2]).unwrap().0),
                        Pip {
                            is_j: false,
                            buf: None,
                        },
                    );
                } else if line.starts_with("from ") {
                    panic!("regex failed: {line:?}");
                }
            }
        }

        grid = Some(Grid {
            tiles,
            nodes,
            pips,
            bufs: EntitySet::new(),
        });
        sites = Some(cur_sites);
    }
    if grid.is_some() {
        println!("dumped {part}-{pkg}");
    } else {
        println!("no {part}-{pkg}");
    }
    Some(PrePart {
        arch: arch.to_string(),
        name: part.to_string(),
        package: pkg.to_string(),
        speeds,
        grid: grid?,
        sites: sites?,
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    ThreadPoolBuilder::new()
        .num_threads(args.num_threads)
        .build_global()
        .unwrap();
    let tc = Toolchain::from_file(&args.toolchain)?;
    create_dir_all(&args.target_directory)?;
    for family in DIAMOND_FAMILIES {
        if args.families.iter().any(|x| x == family.name) {
            let pparts: Vec<_> = family
                .parts
                .into_par_iter()
                .flat_map(|&(arch, part)| {
                    family
                        .pkgs
                        .into_par_iter()
                        .map(move |&pkg| (arch, part, pkg))
                })
                .filter_map(|(arch, part, pkg)| dump_pre(&tc, family, arch, part, pkg))
                .collect();
            let mut grids = EntityVec::new();
            let mut parts = Vec::new();
            for part in pparts {
                let grid = 'grid: {
                    for (gid, grid) in &grids {
                        if *grid == part.grid {
                            break 'grid gid;
                        }
                    }
                    grids.push(part.grid)
                };
                parts.push(Part {
                    arch: part.arch,
                    name: part.name,
                    package: part.package,
                    speeds: part.speeds,
                    grid,
                    sites: part.sites,
                });
            }
            let db = Db {
                family: family.name.to_string(),
                grids,
                parts,
            };
            db.to_file(
                args.target_directory
                    .join(format!("{f}.zstd", f = family.name)),
            )
            .unwrap();
            println!(
                "dumped {f} [{n} grids]",
                f = family.name,
                n = db.grids.len()
            );
        }
    }
    Ok(())
}
