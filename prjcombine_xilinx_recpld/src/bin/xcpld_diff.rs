use std::{error::Error, fs::read_to_string, path::PathBuf};

use clap::Parser;
use prjcombine_toolchain::Toolchain;
use prjcombine_vm6::Vm6;
use prjcombine_xilinx_recpld::hprep6::run_hprep6;

#[derive(Debug, Parser)]
struct Args {
    toolchain: PathBuf,
    f1: PathBuf,
    f2: PathBuf,
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let tc = Toolchain::from_file(&args.toolchain)?;
    let f1 = read_to_string(args.f1)?;
    let f1 = Vm6::parse(&f1)?;
    let f1 = run_hprep6(&tc, &f1, None)?;
    let f2 = read_to_string(args.f2)?;
    let f2 = Vm6::parse(&f2)?;
    let f2 = run_hprep6(&tc, &f2, None)?;
    assert_eq!(f1.len(), f2.len());
    for (i, b) in f1.into_iter().enumerate() {
        if f2[i] != b {
            println!("L{i}: {b} -> {b2}", b2 = f2[i]);
        }
    }
    Ok(())
}
