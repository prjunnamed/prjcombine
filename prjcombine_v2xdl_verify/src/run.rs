use prjcombine_xdl::Design;
use prjcombine_toolchain::Toolchain;
use std::process::Stdio;
use std::fs::{File, read_to_string};
use std::io::Write;
use tempfile;
use simple_error::bail;

pub fn run(tc: &Toolchain, part: &str, vlog: &str) -> Result<Design, Box<dyn std::error::Error>> {
    let dir = tempfile::Builder::new()
        .prefix("prjcombine_v2xdl")
        .tempdir()?;

    {
        let mut f_xst = File::create(dir.path().join("t.xst"))?;
        writeln!(f_xst, "run -ifn t.prj -p {part} -top top -ofn t")?;
        let mut f_prj = File::create(dir.path().join("t.prj"))?;
        writeln!(f_prj, "verilog work \"t.v\"")?;
        let mut f_v = File::create(dir.path().join("t.v"))?;
        f_v.write(vlog.as_bytes())?;
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
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    cmd.arg("t");
    let status = cmd.status()?;
    if !status.success() {
        bail!("non-zero ngdbuild status");
    }

    let mut cmd = tc.command("map");
    cmd.current_dir(dir.path().as_os_str());
    cmd.stdin(Stdio::null());
    cmd.arg("-u");
    cmd.arg("-w");
    cmd.arg("-c");
    cmd.arg("0");
    cmd.arg("t");
    let status = cmd.output()?;
    if !status.status.success() {
        let _ = std::io::stderr().write_all(&status.stdout);
        let _ = std::io::stderr().write_all(&status.stderr);
        bail!("non-zero map status");
    }

    let mut cmd = tc.command("xdl");
    cmd.current_dir(dir.path().as_os_str());
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    cmd.arg("-ncd2xdl");
    cmd.arg("t.ncd");
    let status = cmd.status()?;
    if !status.success() {
        bail!("non-zero xdl status");
    }

    let xdl = read_to_string(dir.path().join("t.xdl"))?;
    Ok(Design::parse(&xdl)?)
}
