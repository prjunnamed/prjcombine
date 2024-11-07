use clap::Parser;
use prjcombine_lattice_dump::parse_tiles;
use prjcombine_lattice_rawdump::{Db, Grid, Node, Part, PinDir, Pip, Site};
use prjcombine_toolchain::Toolchain;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::fs::create_dir_all;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader};
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

const RADIANT_FAMILIES: &[FamilyInfo] = &[FamilyInfo {
    name: "nx",
    parts: &[
        ("lifcl", "LIFCL-17"),
        ("lifcl", "LIFCL-33"),
        ("lifcl", "LIFCL-40"),
        ("lfd2nx", "LFD2NX-17"),
        ("lfd2nx", "LFD2NX-40"),
        ("lfcpnx", "LFCPNX-100"),
        ("ut24c", "UT24C40"),
        ("ut24cp", "UT24CP100"),
        ("lfmxo5", "LFMXO5-25"),
    ],
    pkgs: &[
        // LFD2NX, LIFCL, UT24C
        "QFN72",
        "WLCSP72",
        "WLCSP84",
        "CSFBGA121",
        "CABGA196",
        "CABGA256",
        "CABGA400",
        "CSBGA289",
        // LFCPNX, UT24CP
        "ASG256",
        "CBG256",
        "BBG484",
        "BFG484",
        "LFG672",
        // LFMXO5
        "BBG256",
        "BBG400",
    ],
    speeds: &[
        "7_High-Performance_1.0V",
        "7_Low-Power_1.0V",
        "8_High-Performance_1.0V",
        "8_Low-Power_1.0V",
        "9_High-Performance_1.0V",
        "9_Low-Power_1.0V",
    ],
}];

#[derive(Debug, Parser)]
#[command(
    name = "dump_radiant_parts",
    about = "Dump Radiant part geometry into rawdump files."
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
    static ALIAS_RE: OnceLock<Regex> = OnceLock::new();
    static TYPE_RE: OnceLock<Regex> = OnceLock::new();
    static PIN_RE: OnceLock<Regex> = OnceLock::new();
    static PIP_RE: OnceLock<Regex> = OnceLock::new();
    static SITE_RE: OnceLock<Regex> = OnceLock::new();
    let alias_re =
        ALIAS_RE.get_or_init(|| Regex::new(r"    Alias name = ([A-Za-z0-9_?]+)$").unwrap());
    let type_re = TYPE_RE.get_or_init(|| Regex::new(r"           Type : (\d+)$").unwrap());
    let pin_re = PIN_RE.get_or_init(|| {
        Regex::new(r"           Pin  : ([A-Za-z0-9_]+)/([A-Za-z0-9_]+) \(([a-z]+)\)$").unwrap()
    });
    let pip_re =
            PIP_RE.get_or_init(|| Regex::new(r"([A-Za-z0-9_]+) (-->|<--|<->) ([A-Za-z0-9_]+) \(Flags: ----([-j]), 0\) \(Buffer: ([A-Za-z0-9_]+)\)$").unwrap());
    let site_re = SITE_RE.get_or_init(|| {
        Regex::new(r"Site=([A-Za-z0-9_]+) id=\d+ type=([A-Za-z0-9_]+) X=-?\d+ Y=-?\d+$").unwrap()
    });

    let mut speeds = vec![];
    let mut grid = None;
    let mut sites = None;
    for speed in family.speeds {
        let dir = tempfile::Builder::new()
            .prefix("prjcombine_radiant_dump")
            .tempdir()
            .unwrap();
        let mut file = File::create(dir.path().join("top.v")).unwrap();
        writeln!(
            file,
            r#"
(* \db:architecture ="{arch}", \db:device ="{part}", \db:package ="{pkg}", \db:speed ="{speed}", \db:view ="physical" *)
module top();
(*keep*)
meow meow();
endmodule
module meow();
endmodule;
"#
        ).unwrap();
        std::mem::drop(file);
        let mut cmd = tc.command("sv2udb");
        cmd.current_dir(dir.path().as_os_str());
        cmd.stdin(Stdio::null());
        cmd.arg("-o");
        cmd.arg("top.udb");
        cmd.arg("top.v");
        let status = cmd.output().unwrap();
        let stderr = std::str::from_utf8(&status.stderr).unwrap();
        if stderr.contains("ERROR - Invalid package") {
            continue;
        }
        if !status.status.success() {
            let _ = std::io::stderr().write_all(&status.stdout);
            let _ = std::io::stderr().write_all(&status.stderr);
            panic!("non-zero sv2udb exit status");
        }
        speeds.push(speed.to_string());
        if grid.is_some() {
            continue;
        }
        println!("dumping {part}-{pkg}...");
        let mut cmd = tc.command("bitgen");
        cmd.current_dir(dir.path().as_os_str());
        cmd.stdin(Stdio::null());
        cmd.arg("-b");
        cmd.arg("-ipeval");
        cmd.arg("top.udb");
        let status = cmd.output().unwrap();
        if !status.status.success() {
            let _ = std::io::stderr().write_all(&status.stdout);
            let _ = std::io::stderr().write_all(&status.stderr);
            panic!("non-zero bitgen exit status");
        }

        let rbt = File::open(dir.path().join("top.rbt")).unwrap();
        let mut orbt = File::create(dir.path().join("top2.rbt")).unwrap();
        for line in BufReader::new(rbt).lines() {
            let line = line.unwrap();
            if !line.starts_with(['0', '1']) {
                writeln!(orbt, "{line}").unwrap();
            }
        }
        std::mem::drop(orbt);

        let mut cmd = tc.command("bstool");
        cmd.current_dir(dir.path().as_os_str());
        cmd.stdin(Stdio::null());
        cmd.arg("-t");
        cmd.arg("top2.rbt");
        let status = cmd.output().unwrap();
        if !status.status.success() {
            let _ = std::io::stderr().write_all(&status.stdout);
            let _ = std::io::stderr().write_all(&status.stderr);
            panic!("non-zero bstool exit status");
        }
        let tiles = parse_tiles(std::str::from_utf8(&status.stdout).unwrap(), family.name);

        let mut cmd = tc.command("lapie");
        cmd.current_dir(dir.path().as_os_str());
        let mut file = File::create(dir.path().join("meow.tcl")).unwrap();
        write!(
            file,
            r#"
des_read_udb top.udb
dev_report_site
dev_report_node -file nodes.out [get_nodes -re ".*"]
        "#
        )
        .unwrap();
        std::mem::drop(file);
        cmd.arg("meow.tcl");
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        let status = cmd.output().unwrap();
        if !status.status.success() {
            let _ = std::io::stderr().write_all(&status.stdout);
            let _ = std::io::stderr().write_all(&status.stderr);
            panic!("non-zero lapie exit status");
        }
        let mut nodes = EntityMap::new();
        let mut cur_sites = Vec::new();
        let mut pips = HashMap::new();
        let mut bufs = EntitySet::new();
        let file = File::open(dir.path().join("nodes.out")).unwrap();
        let mut aliases = vec![];
        let mut typ: Option<u8> = None;
        let mut pin = None;
        for line in BufReader::new(file).lines() {
            let line = line.unwrap();
            if line.starts_with("    Alias") {
                let cap = alias_re.captures(&line).unwrap();
                aliases.push(cap[1].to_string());
            } else if line.starts_with("           Type") {
                let cap = type_re.captures(&line).unwrap();
                typ = Some(cap[1].parse().unwrap());
            } else if line.starts_with("           Pin ") {
                let cap = pin_re.captures(&line).unwrap();
                pin = Some((
                    cap[1].to_string(),
                    cap[2].to_string(),
                    match &cap[3][..] {
                        "input" => PinDir::Input,
                        "output" => PinDir::Output,
                        "bidirectional" => PinDir::Bidirectional,
                        _ => panic!("weird dir {}", &cap[3]),
                    },
                ));
            } else if line == "           Flags: --- (0)"
                || line == "           Listing connected nodes:"
            {
                // skip
            } else if line == "           No connected nodes" {
                let name = aliases[0].clone();
                let node = Node { aliases, pin, typ };
                pin = None;
                typ = None;
                aliases = vec![];
                nodes.insert(name, node);
            } else if let Some(cap) = pip_re.captures(&line) {
                if !aliases.is_empty() {
                    let name = cap[1].to_string();
                    assert!(aliases.contains(&name));
                    let node = Node { aliases, pin, typ };
                    pin = None;
                    typ = None;
                    aliases = vec![];
                    nodes.insert(name, node);
                }
                let (wt, wf) = match &cap[2][..] {
                    "-->" => (cap[3].to_string(), cap[1].to_string()),
                    "<--" => (cap[1].to_string(), cap[3].to_string()),
                    "<->" => (cap[1].to_string(), cap[3].to_string()),
                    _ => unreachable!(),
                };
                if !nodes.contains_key(&wf) {
                    nodes.insert(
                        wf.to_string(),
                        Node {
                            aliases: vec![],
                            pin: None,
                            typ: None,
                        },
                    );
                }
                if !nodes.contains_key(&wt) {
                    nodes.insert(
                        wt.to_string(),
                        Node {
                            aliases: vec![],
                            pin: None,
                            typ: None,
                        },
                    );
                }
                let buf = bufs.get_or_insert(&cap[5]);
                if &cap[2] == "<->" {
                    pips.insert(
                        (nodes.get(&wt).unwrap().0, nodes.get(&wf).unwrap().0),
                        Pip {
                            is_j: &cap[4] == "j",
                            buf: Some(buf),
                        },
                    );
                }
                pips.insert(
                    (nodes.get(&wf).unwrap().0, nodes.get(&wt).unwrap().0),
                    Pip {
                        is_j: &cap[4] == "j",
                        buf: Some(buf),
                    },
                );
            } else {
                panic!("unk line {line}");
            }
        }
        let file = File::open(dir.path().join("lapie.log")).unwrap();
        for line in BufReader::new(file).lines() {
            let line = line.unwrap();
            if let Some(cap) = site_re.captures(&line) {
                cur_sites.push(Site {
                    name: cap[1].to_string(),
                    typ: Some(cap[2].to_string()),
                });
            } else if line.starts_with("Site") {
                panic!("regex failed: {line:?}");
            }
        }

        grid = Some(Grid {
            tiles,
            nodes,
            pips,
            bufs,
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
    for family in RADIANT_FAMILIES {
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
