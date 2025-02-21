use std::path::PathBuf;

use clap::Parser;
use prjcombine_re_xilinx_xact_data::{
    parts::{get_parts, PartKind},
    pkg::get_pkg,
};

#[derive(Parser)]
struct Args {
    xact: PathBuf,
}

fn main() {
    let args = Args::parse();
    let parts = get_parts(&args.xact);
    for part in parts {
        println!("{part:?}");
        if part.kind != PartKind::Xc7000 {
            let pkg = get_pkg(&args.xact, &part.pkg_file);
            println!("{pkg:?}");
        }
    }
}
