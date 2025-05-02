use std::{error::Error, path::PathBuf};

use clap::Parser;
use prjcombine_re_xilinx_cpld::speeddb::SpeedDb;

#[derive(Parser)]
struct Args {
    sdb: PathBuf,
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let db = SpeedDb::from_file(args.sdb)?;
    for part in db.parts {
        println!("DEV {d} {s}", d = part.dev_name, s = part.speed_name);
        print!("{}", part.speed);
    }
    Ok(())
}
