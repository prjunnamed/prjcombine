mod backend;
mod bitstream;
mod collect;
mod fuzzers;

use std::{error::Error, path::PathBuf};

use backend::reverse_cpld;
use bitstream::reverse_bitstream;
use clap::Parser;
use prjcombine_re_toolchain::Toolchain;
use prjcombine_re_xilinx_cpld::{
    db::Database,
    fuzzdb::{FuzzDb, FuzzDbPart},
    hprep6::run_hprep6,
    vm6_util::{insert_dummy_obuf, prep_vm6},
};

#[derive(Parser)]
struct Args {
    tc: PathBuf,
    db: PathBuf,
    fdb: PathBuf,
    device: Option<String>,
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let tc = Toolchain::from_file(args.tc)?;
    let db = Database::from_file(args.db)?;
    let mut parts = vec![];
    for part in &db.parts {
        if let Some(ref filter) = args.device {
            if *filter != part.dev_name
                && *filter != format!("{d}{p}", d = part.dev_name, p = part.pkg_name)
            {
                continue;
            }
        }
        let device = &db.devices[part.device];
        let package = &db.packages[part.package];
        let bits = reverse_cpld(&tc, part, device, package, args.debug);
        println!("MAIN RE DONE {d} {p}", d = part.dev_name, p = part.pkg_name);
        let mut vm6 = prep_vm6(part, &device.device, package, &part.speeds[0]);
        insert_dummy_obuf(&mut vm6);
        let blank = run_hprep6(&tc, &vm6, None)?;
        let map = reverse_bitstream(
            &tc,
            device.device.kind,
            &part.dev_name,
            &part.pkg_name,
            blank.len(),
        );
        println!("JED RE DONE {d} {p}", d = part.dev_name, p = part.pkg_name);
        parts.push(FuzzDbPart {
            dev_name: part.dev_name.clone(),
            pkg_name: part.pkg_name.clone(),
            bits,
            map,
            blank,
        });
    }
    let fdb = FuzzDb { parts };
    fdb.to_file(args.fdb)?;

    Ok(())
}
