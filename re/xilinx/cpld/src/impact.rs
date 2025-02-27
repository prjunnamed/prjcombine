use std::{
    error::Error,
    fs::{File, read_to_string},
    io::Write,
    process::Stdio,
};

use bitvec::vec::BitVec;
use prjcombine_jed::JedFile;
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
    let jed = JedFile::new()
        .with_fuses(bits.clone())
        .with_note(format!(" DEVICE {dev}-{pkg}"));
    jed.emit_to_file(dir.path().join("t.jed"))?;

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
