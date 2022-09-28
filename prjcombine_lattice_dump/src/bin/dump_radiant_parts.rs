use prjcombine_lattice_dump::{dump_html, parse_tiles};
use prjcombine_toolchain::Toolchain;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use simple_error::bail;
use std::error::Error;
use std::fs::create_dir_all;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use structopt::StructOpt;

const RADIANT_PARTS: &[(&str, &str, &str, &str)] = &[
    ("lifcl", "LIFCL-17", "CABGA256", "7_High-Performance_1.0V"),
    ("lifcl", "LIFCL-33", "WLCSP84", "7_High-Performance_1.0V"),
    ("lifcl", "LIFCL-40", "CABGA256", "7_High-Performance_1.0V"),
    (
        "lfd2nx",
        "LFD2NX-17",
        "CSFBGA121",
        "7_High-Performance_1.0V",
    ),
    (
        "lfd2nx",
        "LFD2NX-40",
        "CSFBGA121",
        "7_High-Performance_1.0V",
    ),
    ("lfcpnx", "LFCPNX-100", "CBG256", "7_High-Performance_1.0V"),
    ("ut24c", "UT24C40", "CABGA256", "7_High-Performance_1.0V"),
    ("ut24cp", "UT24CP100", "BBG484", "8_High-Performance_1.0V"),
    ("lfmxo5", "LFMXO5-25", "BBG256", "7_High-Performance_1.0V"),
];

#[derive(Debug, StructOpt)]
#[structopt(
    name = "dump_radiant_parts",
    about = "Dump Radiant part geometry into rawdump files."
)]
struct Opt {
    toolchain: String,
    #[structopt(parse(from_os_str))]
    target_directory: PathBuf,
    #[structopt(short = "n", long, default_value = "0")]
    num_threads: usize,
}

fn dump_part(
    tc: &Toolchain,
    arch: &str,
    part: &str,
    package: &str,
    perf: &str,
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let dir = tempfile::Builder::new()
        .prefix("prjcombine_radiant_dump")
        .tempdir()?;
    let mut file = File::create(dir.path().join("top.v"))?;
    writeln!(
        file,
        r#"
(* \db:architecture ="{arch}", \db:device ="{part}", \db:package ="{package}", \db:speed ="{perf}", \db:timestamp =1576073342, \db:view ="physical" *)
module top();
(*keep*)
meow meow();
endmodule
module meow();
endmodule;
"#
    )?;
    std::mem::drop(file);
    let mut cmd = tc.command("sv2udb");
    cmd.current_dir(dir.path().as_os_str());
    cmd.stdin(Stdio::null());
    cmd.arg("-o");
    cmd.arg("top.udb");
    cmd.arg("top.v");
    let status = cmd.output()?;
    if !status.status.success() {
        let _ = std::io::stderr().write_all(&status.stdout);
        let _ = std::io::stderr().write_all(&status.stderr);
        bail!("non-zero sv2udb exit status");
    }
    let mut cmd = tc.command("bitgen");
    cmd.current_dir(dir.path().as_os_str());
    cmd.stdin(Stdio::null());
    cmd.arg("-b");
    cmd.arg("-ipeval");
    cmd.arg("top.udb");
    let status = cmd.output()?;
    if !status.status.success() {
        let _ = std::io::stderr().write_all(&status.stdout);
        let _ = std::io::stderr().write_all(&status.stderr);
        bail!("non-zero bitgen exit status");
    }

    let rbt = File::open(dir.path().join("top.rbt"))?;
    let mut orbt = File::create(dir.path().join("top2.rbt"))?;
    for line in BufReader::new(rbt).lines() {
        let line = line?;
        if !line.starts_with(['0', '1']) {
            writeln!(orbt, "{}", line)?;
        }
    }
    std::mem::drop(orbt);

    let mut cmd = tc.command("bstool");
    cmd.current_dir(dir.path().as_os_str());
    cmd.stdin(Stdio::null());
    cmd.arg("-t");
    cmd.arg("top2.rbt");
    let status = cmd.output()?;
    if !status.status.success() {
        let _ = std::io::stderr().write_all(&status.stdout);
        let _ = std::io::stderr().write_all(&status.stderr);
        bail!("non-zero bstool exit status");
    }
    let arch = match arch {
        "lifcl" => "nx",
        "lfd2nx" => "nx",
        "lfcpnx" => "nx",
        "ut24c" => "nx",
        "ut24cp" => "nx",
        "lfmxo5" => "nx",
        _ => panic!("unk arch {arch}"),
    };
    let tiles = parse_tiles(std::str::from_utf8(&status.stdout).unwrap(), arch);
    let pname = if part.contains("MXO3") {
        format!("{part}-{package}")
    } else {
        part.to_string()
    };
    dump_html(
        &output_dir.join(format!("{pname}.html")),
        arch,
        &pname,
        &tiles,
    )?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    ThreadPoolBuilder::new()
        .num_threads(opt.num_threads)
        .build_global()
        .unwrap();
    let tc = Toolchain::from_file(&opt.toolchain)?;
    create_dir_all(&opt.target_directory)?;
    RADIANT_PARTS
        .into_par_iter()
        .for_each(|(arch, part, package, grade)| {
            println!("dumping {}", part);
            dump_part(&tc, arch, part, package, grade, &opt.target_directory).unwrap();
            println!("dumped {}", part);
        });
    Ok(())
}
