use std::{
    error::Error,
    ffi::OsStr,
    fs::{create_dir_all, read_to_string},
    io::Write,
    os::unix::prelude::OsStrExt,
    process::Stdio,
};

use bitvec::vec::BitVec;
use prjcombine_toolchain::Toolchain;
use prjcombine_vm6::Vm6;
use simple_error::bail;

pub fn run_hprep6(tc: &Toolchain, vm6: &Vm6, sig: Option<u32>) -> Result<BitVec, Box<dyn Error>> {
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

    let jed = read_to_string(dir.path().join("t.jed"))?;
    let jed = parse_jed(&jed);
    Ok(jed)
}

fn parse_jed(jed: &str) -> BitVec {
    let stx = jed.find('\x02').unwrap();
    let etx = jed.find('\x03').unwrap();
    let mut res = None;
    let mut len = None;
    for cmd in jed[stx + 1..etx].split('*') {
        let cmd = cmd.trim();
        if let Some(arg) = cmd.strip_prefix("QF") {
            assert!(len.is_none());
            let n: usize = arg.parse().unwrap();
            len = Some(n);
        } else if let Some(arg) = cmd.strip_prefix('F') {
            assert!(res.is_none());
            let x: u32 = arg.parse().unwrap();
            let x = match x {
                0 => false,
                1 => true,
                _ => unreachable!(),
            };
            res = Some(BitVec::repeat(x, len.unwrap()));
        } else if let Some(arg) = cmd.strip_prefix('L') {
            let sp = arg.find(' ').unwrap();
            let mut pos: usize = arg[..sp].parse().unwrap();
            let v = res.as_mut().unwrap();
            for c in arg[sp..].chars() {
                match c {
                    '0' => {
                        v.set(pos, false);
                        pos += 1;
                    }
                    '1' => {
                        v.set(pos, true);
                        pos += 1;
                    }
                    ' ' => (),
                    _ => unreachable!(),
                }
            }
        }
    }
    res.unwrap()
}
