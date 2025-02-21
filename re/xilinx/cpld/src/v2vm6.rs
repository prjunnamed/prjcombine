use std::{
    fs::{read_to_string, File},
    io::Write,
    process::Stdio,
};

use clap::ValueEnum;
use prjcombine_re_toolchain::Toolchain;
use simple_error::bail;

use crate::vm6::Vm6;

#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum FitUnused {
    Float,
    Ground,
    Pullup,
    Keeper,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum FitTerminate {
    Float,
    Keeper,
    Pullup,
}

#[derive(Debug, Default)]
pub struct FitOpts {
    pub localfbk: bool,
    pub noisp: bool,
    pub iostd: Option<String>,
    pub unused: Option<FitUnused>,
    pub terminate: Option<FitTerminate>,
    pub blkfanin: Option<u32>,
    pub inputs: Option<u32>,
    pub silent: bool,
}

pub fn v2vm6(
    tc: &Toolchain,
    part: &str,
    vlog: &str,
    opts: &FitOpts,
) -> Result<(String, Vm6), Box<dyn std::error::Error>> {
    let dir = tempfile::Builder::new()
        .prefix("prjcombine_xilinx_recpld_v2vm6")
        .tempdir()?;

    {
        let mut f_xst = File::create(dir.path().join("t.xst"))?;
        writeln!(f_xst, "run -ifn t.prj -p {part} -top top -ofn t")?;
        let mut f_prj = File::create(dir.path().join("t.prj"))?;
        writeln!(f_prj, "verilog work \"t.v\"")?;
        std::fs::write(dir.path().join("t.v"), vlog)?;
    }

    let mut cmd = tc.command("xst");
    cmd.current_dir(dir.path().as_os_str());
    cmd.stdin(Stdio::null());
    cmd.arg("-ifn");
    cmd.arg("t.xst");
    let status = cmd.output()?;
    if !status.status.success() {
        let _ = std::io::stderr().write_all(&status.stdout);
        let _ = std::io::stderr().write_all(&status.stderr);
        bail!("non-zero xst status");
    }

    let mut cmd = tc.command("ngdbuild");
    cmd.current_dir(dir.path().as_os_str());
    cmd.stdin(Stdio::null());
    cmd.arg("t");
    let status = cmd.output()?;
    if !status.status.success() {
        let _ = std::io::stderr().write_all(&status.stdout);
        let _ = std::io::stderr().write_all(&status.stderr);
        bail!("non-zero ngdbuild status");
    }

    let mut cmd = tc.command("cpldfit");
    cmd.current_dir(dir.path().as_os_str());
    cmd.stdin(Stdio::null());
    cmd.arg("-p");
    cmd.arg(part);
    cmd.arg("t.ngd");
    if opts.localfbk {
        cmd.arg("-localfbk");
    }
    if opts.noisp {
        cmd.arg("-noisp");
    }
    if let Some(unused) = opts.unused {
        cmd.arg("-unused");
        cmd.arg(match unused {
            FitUnused::Float => "float",
            FitUnused::Keeper => "keeper",
            FitUnused::Pullup => "pullup",
            FitUnused::Ground => "ground",
        });
    }
    if let Some(terminate) = opts.terminate {
        cmd.arg("-terminate");
        cmd.arg(match terminate {
            FitTerminate::Float => "float",
            FitTerminate::Keeper => "keeper",
            FitTerminate::Pullup => "pullup",
        });
    }
    if let Some(ref iostd) = opts.iostd {
        cmd.arg("-iostd");
        cmd.arg(iostd);
    }
    if let Some(n) = opts.blkfanin {
        cmd.arg("-blkfanin");
        cmd.arg(n.to_string());
    }
    if let Some(n) = opts.inputs {
        cmd.arg("-inputs");
        cmd.arg(n.to_string());
    }
    let status = cmd.output()?;
    if !status.status.success() {
        if !opts.silent {
            let _ = std::io::stderr().write_all(&status.stdout);
            let _ = std::io::stderr().write_all(&status.stderr);
        }
        bail!("non-zero map status");
    }

    let vm6 = read_to_string(dir.path().join("t.vm6"))?;
    match Vm6::parse(&vm6) {
        Ok(res) => Ok((vm6, res)),
        Err(err) => {
            eprintln!("OOPS");
            eprint!("{vm6}");
            Err(err)?
        }
    }
}
