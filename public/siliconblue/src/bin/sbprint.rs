use clap::Parser;
use prjcombine_siliconblue::db::Database;
use std::{error::Error, path::PathBuf};

#[derive(Debug, Parser)]
#[command(name = "sbprint", about = "Dump SiliconBlue db file.")]
struct Args {
    file: PathBuf,
    #[arg(short, long)]
    intdb: bool,
    #[arg(short, long)]
    devices: bool,
    #[arg(short, long)]
    grids: bool,
    #[arg(short, long)]
    pkgs: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let db = Database::from_file(args.file)?;
    if args.intdb {
        println!("INTDB");
        db.int.print(&mut std::io::stdout())?;
    }
    if args.grids || args.devices {
        for (gid, grid) in &db.grids {
            print!("GRID {gid}:");
            for dev in &db.parts {
                if dev.grid == gid {
                    print!(" {dev}", dev = dev.name);
                }
            }
            println!();
            if args.grids {
                print!("{grid}");
            }
        }
    }
    if args.pkgs || args.devices {
        for (bid, bond) in &db.bonds {
            print!("BOND {bid}:");
            for dev in &db.parts {
                for (pkg, &dbond) in &dev.bonds {
                    if dbond == bid {
                        print!(" {dev}-{pkg}", dev = dev.name);
                    }
                }
            }
            println!();
            if args.pkgs {
                print!("{bond}");
            }
        }
    }
    if args.devices {
        for dev in &db.parts {
            println!("DEVICE {n} GRID {g}", n = dev.name, g = dev.grid);
            for (pkg, bond) in &dev.bonds {
                println!("\tBOND {pkg}: {bond}");
            }
            for speed in &dev.speeds {
                println!("\tSPEED {speed}");
            }
            for temp in &dev.temps {
                println!("\tTEMP {temp}");
            }
        }
    }
    Ok(())
}
