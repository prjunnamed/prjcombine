use std::{
    error::Error,
    ffi::OsStr,
    fs::create_dir_all,
    io::Write,
    os::unix::prelude::OsStrExt,
    process::Stdio,
};

use prjcombine_jed::{JedFile, JedParserOptions};
use prjcombine_re_toolchain::Toolchain;
use simple_error::bail;

use crate::vm6::Vm6;

pub fn run_hprep6(tc: &Toolchain, vm6: &Vm6, sig: Option<u32>) -> Result<JedFile, Box<dyn Error>> {
    let dir = tempfile::Builder::new()
        .prefix("prjcombine_xilinx_recpld_hprep6")
        .tempdir()?;

    let mut vs = String::new();
    vm6.write(&mut vs)?;
    std::fs::write(dir.path().join("t.vm6"), &vs)?;

    let mut cmd = tc.command("hprep6");
    cmd.current_dir(dir.path().as_os_str());
    cmd.stdin(Stdio::null());
    cmd.arg("-i");
    cmd.arg("t.vm6");
    if let Some(sig) = sig {
        cmd.arg("-n");
        cmd.arg(format!("0x{sig:08x}"));
    } else if vm6.family.contains("95") {
        cmd.arg("-n");
        cmd.arg(OsStr::from_bytes(b"\xff\xff\xff\xff"));
    }
    let status = cmd.output()?;
    if !status.status.success() {
        let _ = std::io::stderr().write_all(&status.stdout);
        let _ = std::io::stderr().write_all(&status.stderr);
        let _ = create_dir_all("crash");
        let fname = format!(
            "crash/{part}-{pid}-{r}.vm6",
            part = vm6.part,
            pid = std::process::id(),
            r = rand::random::<u64>(),
        );
        let _ = std::fs::write(fname, vs);
        std::mem::forget(dir);
        bail!("non-zero hprep6 status");
    }

    let jed = JedFile::parse_from_file(
        dir.path().join("t.jed"),
        &JedParserOptions::new().skip_design_spec(),
    )?;
    Ok(jed)
}
