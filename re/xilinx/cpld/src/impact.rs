use std::{
    error::Error,
    fs::{read_to_string, File},
    io::Write,
    process::Stdio,
};

use bitvec::vec::BitVec;
use prjcombine_re_toolchain::Toolchain;
use simple_error::bail;

#[allow(clippy::too_many_arguments)]
pub fn run_impact(
    tc: &Toolchain,
    dev: &str,
    pkg: &str,
    bits: &BitVec,
    usercode: Option<u32>,
    ues: Option<&[u8]>,
    rprot: bool,
    wprot: bool,
) -> Result<String, Box<dyn Error>> {
    let dir = tempfile::Builder::new()
        .prefix("prjcombine_xilinx_recpld_hprep6")
        .tempdir()?;
    let mut f = File::create(dir.path().join("t.jed"))?;
    writeln!(f, "\x02QF{n}*", n = bits.len())?;
    writeln!(f, "F0*")?;
    writeln!(f, "N DEVICE {dev}-{pkg}*")?;
    for (i, c) in bits.chunks(80).enumerate() {
        write!(f, "L{ii:06} ", ii = i * 80)?;
        for bit in c {
            write!(f, "{x}", x = u32::from(*bit))?;
        }
        writeln!(f, "*")?;
    }
    writeln!(f, "\x030")?;
    drop(f);

    let mut f = File::create(dir.path().join("t.batch"))?;
    writeln!(f, "setmode -bscan")?;
    writeln!(f, "setcable -p svf -file t.svf")?;
    writeln!(f, "adddevice -p 1 -part {dev}{pkg}")?;
    writeln!(f, "assignfile -p 1 -file t.jed")?;
    write!(f, "program -p 1")?;
    if rprot {
        write!(f, " -r")?;
    }
    if wprot {
        write!(f, " -w")?;
    }
    if let Some(x) = usercode {
        write!(f, " -u {x:08x}")?;
    }
    if let Some(x) = ues {
        write!(f, " -u ")?;
        f.write_all(x)?;
    }
    writeln!(f)?;
    writeln!(f, "quit")?;
    drop(f);

    let mut cmd = tc.command("impact");
    cmd.current_dir(dir.path().as_os_str());
    cmd.stdin(Stdio::null());
    cmd.arg("-batch");
    cmd.arg("t.batch");
    let status = cmd.output()?;
    if !status.status.success() {
        let _ = std::io::stderr().write_all(&status.stdout);
        let _ = std::io::stderr().write_all(&status.stderr);
        bail!("non-zero impact status");
    }

    let res = read_to_string(dir.path().join("t.svf"))?;
    Ok(res)
}
