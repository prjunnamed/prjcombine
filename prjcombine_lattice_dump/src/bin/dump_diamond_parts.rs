#![allow(clippy::type_complexity)]

use prjcombine_lattice_dump::{dump_html, parse_tiles};
use prjcombine_toolchain::Toolchain;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use simple_error::bail;
use std::error::Error;
use std::fs::create_dir_all;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use structopt::StructOpt;

const DIAMOND_PARTS: &[(&str, &[(&str, &str, &str, &str)])] = &[
    (
        "xp",
        &[
            ("mg5g00", "LFXP3C", "PQFP208", "3"),
            ("mg5g00", "LFXP3E", "PQFP208", "3"),
            ("mg5g00", "LFXP6C", "PQFP208", "3"),
            ("mg5g00", "LFXP6E", "PQFP208", "3"),
            ("mg5g00", "LFXP10C", "FPBGA256", "3"),
            ("mg5g00", "LFXP10E", "FPBGA256", "3"),
            ("mg5g00", "LFXP15C", "FPBGA256", "3"),
            ("mg5g00", "LFXP15E", "FPBGA256", "3"),
            ("mg5g00", "LFXP20C", "FPBGA256", "3"),
            ("mg5g00", "LFXP20E", "FPBGA256", "3"),
        ],
    ),
    (
        "ecp",
        &[
            ("ep5g00", "LFEC1E", "PQFP208", "3"),
            ("ep5g00", "LFEC3E", "PQFP208", "3"),
            ("ep5g00", "LFEC6E", "PQFP208", "3"),
            ("ep5g00", "LFEC10E", "PQFP208", "3"),
            ("ep5g00", "LFEC15E", "FPBGA484", "3"),
            ("ep5g00", "LFEC20E", "FPBGA484", "3"),
            ("ep5g00", "LFEC33E", "FPBGA484", "3"),
            ("ep5g00p", "LFECP6E", "PQFP208", "3"),
            ("ep5g00p", "LFECP10E", "PQFP208", "3"),
            ("ep5g00p", "LFECP15E", "FPBGA484", "3"),
            ("ep5g00p", "LFECP20E", "FPBGA484", "3"),
            ("ep5g00p", "LFECP33E", "FPBGA484", "3"),
        ],
    ),
    (
        "machxo",
        &[
            ("mj5g00", "LAMXO256C", "TQFP100", "3"),
            ("mj5g00", "LAMXO256E", "TQFP100", "3"),
            ("mj5g00", "LCMXO256C", "TQFP100", "3"),
            ("mj5g00", "LCMXO256E", "TQFP100", "3"),
            ("mj5g00", "LAMXO640C", "TQFP100", "3"),
            ("mj5g00", "LAMXO640E", "TQFP100", "3"),
            ("mj5g00", "LCMXO640C", "TQFP100", "3"),
            ("mj5g00", "LCMXO640E", "TQFP100", "3"),
            ("mj5g00", "LAMXO1200E", "TQFP100", "3"),
            ("mj5g00", "LCMXO1200C", "TQFP100", "3"),
            ("mj5g00", "LCMXO1200E", "TQFP100", "3"),
            ("mj5g00", "LAMXO2280E", "TQFP100", "3"),
            ("mj5g00", "LCMXO2280C", "TQFP100", "3"),
            ("mj5g00", "LCMXO2280E", "TQFP100", "3"),
            ("mj5g00p", "LPTM10-1247", "TQFP128", "3"),
            ("mj5g00p", "LPTM10-12107", "FTBGA208", "3"),
        ],
    ),
    (
        "scm",
        &[
            ("slayer", "LFSC3GA15E", "FPBGA900", "5"),
            ("slayer", "LFSC3GA25E", "FPBGA900", "5"),
            ("slayer", "LFSC3GA40E", "FFBGA1152", "5"),
            ("slayer", "LFSC3GA80E", "FFBGA1152", "5"),
            ("slayer", "LFSC3GA115E", "FFBGA1152", "5"),
            ("or5s00", "LFSCM3GA15EP1", "FPBGA900", "5"),
            ("or5s00", "LFSCM3GA25EP1", "FPBGA900", "5"),
            ("or5s00", "LFSCM3GA40EP1", "FFBGA1152", "5"),
            ("or5s00", "LFSCM3GA80EP1", "FFBGA1152", "5"),
            ("or5s00", "LFSCM3GA115EP1", "FFBGA1152", "5"),
        ],
    ),
    (
        "xp2",
        &[
            ("mg5a00", "LAXP2-5E", "FTBGA256", "5"),
            ("mg5a00", "LFXP2-5E", "FTBGA256", "5"),
            ("mg5a00", "LAXP2-8E", "FTBGA256", "5"),
            ("mg5a00", "LFXP2-8E", "FTBGA256", "5"),
            ("mg5a00", "LAXP2-17E", "FTBGA256", "5"),
            ("mg5a00", "LFXP2-17E", "FTBGA256", "5"),
            ("mg5a00", "LFXP2-30E", "FTBGA256", "5"),
            ("mg5a00", "LFXP2-40E", "FPBGA484", "5"),
        ],
    ),
    (
        "ecp2",
        &[
            ("ep5a00", "LFE2-6E", "FPBGA256", "5"),
            ("ep5a00", "LFE2-12E", "FPBGA256", "5"),
            ("ep5a00", "LFE2-20E", "FPBGA256", "5"),
            ("ep5a00", "LFE2-35E", "FPBGA672", "5"),
            ("ep5a00", "LFE2-50E", "FPBGA672", "5"),
            ("ep5a00", "LFE2-70E", "FPBGA672", "5"),
            ("ep5m00", "LFE2M20E", "FPBGA256", "5"),
            ("ep5m00", "LFE2M35E", "FPBGA672", "5"),
            ("ep5m00", "LFE2M50E", "FPBGA672", "5"),
            ("ep5m00", "LFE2M70E", "FPBGA900", "5"),
            ("ep5m00", "LFE2M100E", "FPBGA900", "5"),
        ],
    ),
    (
        "ecp3",
        &[
            ("ep5c00", "LAE3-17EA", "FTBGA256", "6"),
            ("ep5c00", "LFE3-17EA", "FTBGA256", "6"),
            ("ep5c00", "LAE3-35EA", "FTBGA256", "6"),
            ("ep5c00", "LFE3-35EA", "FTBGA256", "6"),
            ("ep5c00", "LFE3-70E", "FPBGA1156", "6"),
            ("ep5c00", "LFE3-70EA", "FPBGA1156", "6"),
            ("ep5c00", "LFE3-95E", "FPBGA1156", "6"),
            ("ep5c00", "LFE3-95EA", "FPBGA1156", "6"),
            ("ep5c00", "LFE3-150EA", "FPBGA1156", "6"),
        ],
    ),
    (
        "ecp4",
        &[
            ("ep5d00", "LFE4-30E", "FPBGA648", "8"),
            ("ep5d00", "LFE4-50E", "FPBGA648", "8"),
            ("ep5d00", "LFE4-95E", "FPBGA648", "8"),
            ("ep5d00", "LFE4-130E", "FPBGA648", "8"),
            ("ep5d00", "LFE4-190E", "FCBGA1152", "8"),
        ],
    ),
    (
        "ecp5",
        &[
            ("sa5p00", "LAE5U-12F", "CABGA381", "6"),
            ("sa5p00", "LFE5U-12F", "CABGA381", "6"),
            ("sa5p00", "LFE5U-25F", "CABGA381", "6"),
            ("sa5p00", "LFE5U-45F", "CABGA381", "6"),
            ("sa5p00", "LFE5U-85F", "CABGA381", "6"),
            ("sa5p00m", "LAE5UM-25F", "CABGA381", "6"),
            ("sa5p00m", "LFE5UM-25F", "CABGA381", "6"),
            ("sa5p00m", "LAE5UM-45F", "CABGA381", "6"),
            ("sa5p00m", "LFE5UM-45F", "CABGA381", "6"),
            ("sa5p00m", "LAE5UM-85F", "CABGA381", "6"),
            ("sa5p00m", "LFE5UM-85F", "CABGA381", "6"),
            ("sa5p00g", "LFE5UM5G-25F", "CABGA381", "8"),
            ("sa5p00g", "LFE5UM5G-45F", "CABGA381", "8"),
            ("sa5p00g", "LFE5UM5G-85F", "CABGA381", "8"),
        ],
    ),
    (
        "crosslink",
        &[
            ("sn5w00", "LIA-MD6000", "CKFBGA80", "6"),
            ("sn5w00", "LIF-MD6000", "CKFBGA80", "6"),
            ("wi5s00", "LIA-MDF6000", "CKFBGA80", "6"),
            ("wi5s00", "LIF-MDF6000", "CKFBGA80", "6"),
        ],
    ),
    (
        "machxo2",
        &[
            ("xo2c00", "LCMXO2-256HC", "CSBGA132", "4"),
            ("xo2c00", "LCMXO2-256ZE", "CSBGA132", "1"),
            ("xo2c00", "LCMXO2-640HC", "CSBGA132", "4"),
            ("xo2c00", "LCMXO2-640ZE", "CSBGA132", "1"),
            ("xo2c00", "LCMXO2-640UHC", "TQFP144", "4"),
            ("xo2c00", "LCMXO2-1200HC", "TQFP144", "4"),
            ("xo2c00", "LCMXO2-1200ZE", "TQFP144", "1"),
            ("xo2c00", "LCMXO2-1200UHC", "FTBGA256", "4"),
            ("xo2c00", "LCMXO2-2000HC", "FTBGA256", "4"),
            ("xo2c00", "LCMXO2-2000ZE", "FTBGA256", "1"),
            ("xo2c00", "LCMXO2-2000UHC", "FPBGA484", "4"),
            ("xo2c00", "LCMXO2-2000UHE", "FPBGA484", "4"),
            ("xo2c00", "LCMXO2-4000HC", "FPBGA484", "4"),
            ("xo2c00", "LCMXO2-4000HE", "FPBGA484", "4"),
            ("xo2c00", "LCMXO2-4000ZE", "FPBGA484", "1"),
            ("xo2c00", "LCMXO2-4000UHC", "CABGA400", "4"),
            ("xo2c00", "LCMXO2-7000HC", "FPBGA484", "4"),
            ("xo2c00", "LCMXO2-7000HE", "FPBGA484", "4"),
            ("xo2c00", "LCMXO2-7000ZE", "FPBGA484", "1"),
            ("xo2c00", "LCMXO2-10000HC", "FPBGA484", "4"),
            ("xo2c00", "LCMXO2-10000HE", "FPBGA484", "4"),
            ("xo2c00", "LCMXO2-10000ZE", "FPBGA484", "1"),
            ("xo2c00p", "LPTM21", "FTBGA237", "1A"),
            ("xo2c00p", "LPTM21L", "CABGA100", "1A"),
            ("xo3c00a", "LCMXO3L-640E", "CSFBGA121", "5"),
            ("xo3c00a", "LCMXO3L-1300C", "CABGA256", "5"),
            ("xo3c00a", "LCMXO3L-1300E", "WLCSP36", "5"),
            ("xo3c00a", "LCMXO3L-1300E", "CSFBGA121", "5"),
            ("xo3c00a", "LCMXO3L-1300E", "CSFBGA256", "5"),
            ("xo3c00a", "LCMXO3L-2100C", "CABGA256", "5"),
            ("xo3c00a", "LCMXO3L-2100C", "CABGA324", "5"),
            ("xo3c00a", "LCMXO3L-2100E", "WLCSP49", "5"),
            ("xo3c00a", "LCMXO3L-2100E", "CSFBGA121", "5"),
            ("xo3c00a", "LCMXO3L-2100E", "CSFBGA256", "5"),
            ("xo3c00a", "LCMXO3L-2100E", "CSFBGA324", "5"),
            ("xo3c00a", "LCMXO3L-4300C", "CABGA256", "5"),
            ("xo3c00a", "LCMXO3L-4300C", "CABGA324", "5"),
            ("xo3c00a", "LCMXO3L-4300C", "CABGA400", "5"),
            ("xo3c00a", "LCMXO3L-4300E", "WLCSP81", "5"),
            ("xo3c00a", "LCMXO3L-4300E", "CSFBGA121", "5"),
            ("xo3c00a", "LCMXO3L-4300E", "CSFBGA256", "5"),
            ("xo3c00a", "LCMXO3L-4300E", "CSFBGA324", "5"),
            ("xo3c00a", "LCMXO3L-6900C", "CABGA256", "5"),
            ("xo3c00a", "LCMXO3L-6900C", "CABGA324", "5"),
            ("xo3c00a", "LCMXO3L-6900C", "CABGA400", "5"),
            ("xo3c00a", "LCMXO3L-6900E", "CSFBGA256", "5"),
            ("xo3c00a", "LCMXO3L-6900E", "CSFBGA324", "5"),
            ("xo3c00a", "LCMXO3L-9400C", "CABGA256", "5"),
            ("xo3c00a", "LCMXO3L-9400C", "CABGA400", "5"),
            ("xo3c00a", "LCMXO3L-9400C", "CABGA484", "5"),
            ("xo3c00a", "LCMXO3L-9400E", "CABGA256", "5"),
            ("xo3c00a", "LCMXO3L-9400E", "CSFBGA256", "5"),
            ("xo3c00a", "LCMXO3L-9400E", "CABGA400", "5"),
            ("xo3c00a", "LCMXO3L-9400E", "CABGA484", "5"),
            ("xo3c00f", "LCMXO3LF-640E", "CSFBGA121", "5"),
            ("xo3c00f", "LAMXO3LF-1300C", "CABGA256", "5"),
            ("xo3c00f", "LAMXO3LF-1300E", "CABGA256", "5"),
            ("xo3c00f", "LAMXO3LF-1300E", "CSFBGA121", "5"),
            ("xo3c00f", "LCMXO3LF-1300C", "CABGA256", "5"),
            ("xo3c00f", "LCMXO3LF-1300E", "WLCSP36", "5"),
            ("xo3c00f", "LCMXO3LF-1300E", "CSFBGA121", "5"),
            ("xo3c00f", "LCMXO3LF-1300E", "CSFBGA256", "5"),
            ("xo3c00f", "LAMXO3LF-2100C", "CABGA256", "5"),
            ("xo3c00f", "LAMXO3LF-2100C", "CABGA324", "5"),
            ("xo3c00f", "LAMXO3LF-2100E", "CSFBGA121", "5"),
            ("xo3c00f", "LAMXO3LF-2100E", "CABGA256", "5"),
            ("xo3c00f", "LAMXO3LF-2100E", "CABGA324", "5"),
            ("xo3c00f", "LCMXO3LF-2100C", "CABGA256", "5"),
            ("xo3c00f", "LCMXO3LF-2100C", "CABGA324", "5"),
            ("xo3c00f", "LCMXO3LF-2100E", "WLCSP49", "5"),
            ("xo3c00f", "LCMXO3LF-2100E", "CSFBGA121", "5"),
            ("xo3c00f", "LCMXO3LF-2100E", "CSFBGA256", "5"),
            ("xo3c00f", "LCMXO3LF-2100E", "CSFBGA324", "5"),
            ("xo3c00f", "LAMXO3LF-4300C", "CABGA256", "5"),
            ("xo3c00f", "LAMXO3LF-4300C", "CABGA324", "5"),
            ("xo3c00f", "LAMXO3LF-4300E", "CSFBGA121", "5"),
            ("xo3c00f", "LAMXO3LF-4300E", "CABGA256", "5"),
            ("xo3c00f", "LAMXO3LF-4300E", "CABGA324", "5"),
            ("xo3c00f", "LCMXO3LF-4300C", "CABGA256", "5"),
            ("xo3c00f", "LCMXO3LF-4300C", "CABGA324", "5"),
            ("xo3c00f", "LCMXO3LF-4300C", "CABGA400", "5"),
            ("xo3c00f", "LCMXO3LF-4300E", "WLCSP81", "5"),
            ("xo3c00f", "LCMXO3LF-4300E", "CSFBGA121", "5"),
            ("xo3c00f", "LCMXO3LF-4300E", "CSFBGA256", "5"),
            ("xo3c00f", "LCMXO3LF-4300E", "CSFBGA324", "5"),
            ("xo3c00f", "LCMXO3LF-6900C", "CABGA256", "5"),
            ("xo3c00f", "LCMXO3LF-6900C", "CABGA324", "5"),
            ("xo3c00f", "LCMXO3LF-6900C", "CABGA400", "5"),
            ("xo3c00f", "LCMXO3LF-6900E", "CSFBGA256", "5"),
            ("xo3c00f", "LCMXO3LF-6900E", "CSFBGA324", "5"),
            ("xo3c00f", "LCMXO3LF-9400C", "CABGA256", "5"),
            ("xo3c00f", "LCMXO3LF-9400C", "CABGA400", "5"),
            ("xo3c00f", "LCMXO3LF-9400C", "CABGA484", "5"),
            ("xo3c00f", "LCMXO3LF-9400E", "CABGA256", "5"),
            ("xo3c00f", "LCMXO3LF-9400E", "CSFBGA256", "5"),
            ("xo3c00f", "LCMXO3LF-9400E", "CABGA400", "5"),
            ("xo3c00f", "LCMXO3LF-9400E", "CABGA484", "5"),
            ("xo3c00d", "LCMXO3LFP-4300HC", "QFN72", "5"),
            ("xo3c00d", "LCMXO3LFP-4300HC", "CABGA256", "5"),
            ("xo3c00d", "LCMXO3LFP-6900HC", "CABGA400", "5"),
            ("xo3c00d", "LCMXO3LFP-9400HC", "QFN72", "5"),
            ("xo3c00d", "LCMXO3LFP-9400HC", "CABGA256", "5"),
            ("xo3c00d", "LCMXO3LFP-9400HC", "CABGA400", "5"),
            ("xo3c00d", "LCMXO3LFP-9400HC", "CABGA484", "5"),
            ("se5c00", "LAMXO3D-4300HC", "CABGA256", "5"),
            ("se5c00", "LAMXO3D-4300ZC", "CABGA256", "2"),
            ("se5c00", "LAMXO3D-4300ZC", "QFN72", "2"),
            ("se5c00", "LCMXO3D-4300HC", "CABGA256", "5"),
            ("se5c00", "LCMXO3D-4300HC", "QFN72", "5"),
            ("se5c00", "LCMXO3D-4300HE", "CABGA256", "5"),
            ("se5c00", "LCMXO3D-4300ZC", "CABGA256", "2"),
            ("se5c00", "LCMXO3D-4300ZC", "QFN72", "2"),
            ("se5c00", "LCMXO3D-4300ZE", "CABGA256", "2"),
            ("se5c00", "LAMXO3D-9400HE", "CABGA256", "5"),
            ("se5c00", "LAMXO3D-9400HE", "CABGA484", "5"),
            ("se5c00", "LAMXO3D-9400ZC", "CABGA256", "2"),
            ("se5c00", "LAMXO3D-9400ZC", "CABGA484", "2"),
            ("se5c00", "LCMXO3D-9400HC", "QFN72", "5"),
            ("se5c00", "LCMXO3D-9400HC", "CABGA256", "5"),
            ("se5c00", "LCMXO3D-9400HC", "CABGA400", "5"),
            ("se5c00", "LCMXO3D-9400HC", "CABGA484", "5"),
            ("se5c00", "LCMXO3D-9400HE", "WLCSP69", "5"),
            ("se5c00", "LCMXO3D-9400HE", "CABGA484", "5"),
            ("se5c00", "LCMXO3D-9400ZC", "QFN72", "2"),
            ("se5c00", "LCMXO3D-9400ZC", "CABGA256", "2"),
            ("se5c00", "LCMXO3D-9400ZC", "CABGA400", "2"),
            ("se5c00", "LCMXO3D-9400ZE", "CABGA484", "2"),
            ("se5r00", "LFMNX-50", "CBG256", "5"),
        ],
    ),
];

#[derive(Debug, StructOpt)]
#[structopt(
    name = "dump_diamond_parts",
    about = "Dump Diamond part geometry into rawdump files."
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
    tc: &Toolchain,
    family: &str,
    arch: &str,
    part: &str,
    package: &str,
    perf: &str,
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let dir = tempfile::Builder::new()
        .prefix("prjcombine_diamond_dump")
        .tempdir()?;
    let mut file = File::create(dir.path().join("top.ncl"))?;
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
      package {package};
      performance "{perf}";
   }}
}}
    "#
    )?;
    std::mem::drop(file);
    let mut cmd = tc.command("ncl2ncd");
    cmd.current_dir(dir.path().as_os_str());
    cmd.stdin(Stdio::null());
    cmd.arg("top.ncl");
    let status = cmd.output()?;
    if !status.status.success() {
        let _ = std::io::stderr().write_all(&status.stdout);
        let _ = std::io::stderr().write_all(&status.stderr);
        bail!("non-zero ncl2ncd exit status");
    }
    let mut cmd = tc.command("bitgen");
    cmd.current_dir(dir.path().as_os_str());
    cmd.stdin(Stdio::null());
    cmd.arg("-d");
    cmd.arg("top.ncd");
    let status = cmd.output()?;
    if !status.status.success() {
        let _ = std::io::stderr().write_all(&status.stdout);
        let _ = std::io::stderr().write_all(&status.stderr);
        bail!("non-zero bitgen exit status");
    }
    let mut cmd = tc.command("bstool");
    cmd.current_dir(dir.path().as_os_str());
    cmd.stdin(Stdio::null());
    cmd.arg("-t");
    cmd.arg("top.bit");
    let status = cmd.output()?;
    if !status.status.success() {
        let _ = std::io::stderr().write_all(&status.stdout);
        let _ = std::io::stderr().write_all(&status.stderr);
        bail!("non-zero bstool exit status");
    }
    let tiles = parse_tiles(std::str::from_utf8(&status.stdout).unwrap(), family);
    let pname = if part.contains("MXO3") {
        format!("{part}-{package}")
    } else {
        part.to_string()
    };
    dump_html(
        &output_dir.join(format!("{pname}.html")),
        family,
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
    DIAMOND_PARTS.into_par_iter().for_each(|(family, parts)| {
        if opt.families.iter().any(|x| x == family) {
            parts
                .into_par_iter()
                .for_each(|(arch, part, package, grade)| {
                    println!("dumping {}", part);
                    dump_part(
                        &tc,
                        family,
                        arch,
                        part,
                        package,
                        grade,
                        &opt.target_directory,
                    )
                    .unwrap();
                    println!("dumped {}", part);
                });
        }
    });
    Ok(())
}
