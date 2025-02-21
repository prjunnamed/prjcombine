use std::{error::Error, fs::read_to_string, path::PathBuf};

use clap::Parser;
use prjcombine_re_toolchain::Toolchain;
use simple_error::bail;

use prjcombine_re_xilinx_cpld::device::DeviceKind;
use prjcombine_re_xilinx_cpld::{
    partgen::get_parts,
    v2vm6::{v2vm6, FitOpts, FitTerminate, FitUnused},
};

#[derive(Debug, Parser)]
struct Args {
    toolchain: PathBuf,
    part: String,
    vlog: PathBuf,
    #[arg(long)]
    localfbk: bool,
    #[arg(long)]
    noisp: bool,
    #[arg(long)]
    iostd: Option<String>,
    #[arg(long)]
    unused: Option<FitUnused>,
    #[arg(long)]
    terminate: Option<FitTerminate>,
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let tc = Toolchain::from_file(&args.toolchain)?;
    let kind = if args.part.starts_with("xc95") || args.part.starts_with("xa95") {
        if args.part.contains("xl") {
            DeviceKind::Xc9500Xl
        } else if args.part.contains("xv") {
            DeviceKind::Xc9500Xv
        } else {
            DeviceKind::Xc9500
        }
    } else if args.part.starts_with("xcr") {
        DeviceKind::Xpla3
    } else if args.part.starts_with("xc2c") || args.part.starts_with("xa2c") {
        DeviceKind::Coolrunner2
    } else {
        bail!("unknown family for part {p}", p = args.part)
    };
    let pkgs = get_parts(&tc, kind)?;
    let fitopts = FitOpts {
        localfbk: args.localfbk,
        noisp: args.noisp,
        iostd: args.iostd,
        unused: args.unused,
        terminate: args.terminate,
        ..FitOpts::default()
    };
    let vlog = read_to_string(args.vlog)?;
    for p in pkgs {
        if args.part == p.device || args.part == format!("{d}{p}", d = p.device, p = p.package) {
            eprintln!("PKG {} {} {:?}", p.device, p.package, p.speedgrades);
            let pname = format!(
                "{dev}{spd}-{pkg}",
                dev = p.device,
                spd = p.speedgrades[0],
                pkg = p.package
            );
            let (s, _) = v2vm6(&tc, &pname, &vlog, &fitopts)?;
            print!("{s}");
            return Ok(());
        }
    }
    bail!("no part found");
}
