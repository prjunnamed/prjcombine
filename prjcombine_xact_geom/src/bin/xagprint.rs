use std::{error::Error, path::PathBuf};

use clap::Parser;
use prjcombine_xact_geom::GeomDb;

#[derive(Debug, Parser)]
#[command(name = "xagprint", about = "Dump xact geom file.")]
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
    let geom = GeomDb::from_file(args.file)?;
    if args.intdb {
        for (name, intdb) in &geom.ints {
            println!("INTDB {name}");
            intdb.print(&mut std::io::stdout())?;
        }
        for (name, ndb) in &geom.namings {
            println!("NAMINGDB {name}");
            ndb.print(&geom.ints[name], &mut std::io::stdout())?;
        }
    }
    if args.grids || args.devices {
        for (gid, grid) in &geom.grids {
            print!("GRID {gid}:");
            for dev in &geom.devices {
                if dev.grid == gid {
                    print!(" {dev}", dev = dev.name);
                }
            }
            println!();
            if args.grids {
                print!("{}", grid);
            }
        }
    }
    if args.pkgs || args.devices {
        for (bid, bond) in &geom.bonds {
            print!("BOND {bid}:");
            for dev in &geom.devices {
                for dbond in &dev.bonds {
                    if dbond.bond == bid {
                        print!(" {dev}-{pkg}", dev = dev.name, pkg = dbond.name);
                    }
                }
            }
            println!();
            if args.pkgs {
                print!("{}", bond);
            }
        }
    }
    if args.devices {
        for dev in &geom.devices {
            println!("DEVICE {n} GRIDS {g}", n = dev.name, g = dev.grid);
            for bond in &dev.bonds {
                println!("\tBOND {n}: {i}", n = bond.name, i = bond.bond);
            }
        }
    }
    Ok(())
}