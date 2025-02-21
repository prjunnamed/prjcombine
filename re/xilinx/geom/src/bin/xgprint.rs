use clap::Parser;
use prjcombine_re_xilinx_geom::{DeviceNaming, GeomDb};
use std::{error::Error, path::PathBuf};

#[derive(Debug, Parser)]
#[command(name = "xgprint", about = "Dump Xilinx geom file.")]
struct Args {
    file: PathBuf,
    #[arg(short, long)]
    intdb: bool,
    #[arg(short, long)]
    devices: bool,
    #[arg(short, long)]
    chips: bool,
    #[arg(short, long)]
    pkgs: bool,
    #[arg(short, long)]
    namings: bool,
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
        print!("{}", geom.gtz);
    }
    if args.chips || args.devices {
        for (gid, chip) in &geom.chips {
            print!("CHIPS {gid}:");
            for dev in &geom.devices {
                for (did, &die) in &dev.chips {
                    if die == gid {
                        if dev.chips.len() == 1 {
                            print!(" {dev}", dev = dev.name);
                        } else {
                            print!(" {dev}.{did}", dev = dev.name);
                        }
                    }
                }
            }
            println!();
            if args.chips {
                print!("{chip}");
            }
        }
        for (ipid, ip) in &geom.interposers {
            print!("INTERPOSER {ipid}:");
            for dev in &geom.devices {
                if dev.interposer == ipid {
                    print!(" {dev}", dev = dev.name);
                }
            }
            println!();
            if args.chips {
                print!("{ip}");
            }
        }
    }
    if args.pkgs || args.devices {
        for (bid, bond) in &geom.bonds {
            print!("BOND {bid}:");
            for dev in &geom.devices {
                for dbond in dev.bonds.values() {
                    if dbond.bond == bid {
                        print!(" {dev}-{pkg}", dev = dev.name, pkg = dbond.name);
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
        for dev in &geom.devices {
            print!(
                "DEVICE {n} INTERPOSER {ip} GRIDS",
                n = dev.name,
                ip = dev.interposer
            );
            for (_, &gid) in &dev.chips {
                print!(" {gid}");
            }
            println!();
            for disabled in &dev.disabled {
                println!("\tDISABLED {disabled:?}");
            }
            for bond in dev.bonds.values() {
                println!("\tBOND {n}: {i}", n = bond.name, i = bond.bond);
            }
            for combo in &dev.combos {
                println!(
                    "\tPART {n}: {bn} {sn}",
                    n = combo.name,
                    bn = dev.bonds[combo.devbond_idx].name,
                    sn = dev.speeds[combo.speed_idx]
                );
            }
            if geom.dev_namings[dev.naming] != DeviceNaming::Dummy {
                println!("\tNAMING {n}", n = dev.naming);
            }
        }
    }
    if args.devices || args.namings {
        for (dnid, dn) in geom.dev_namings {
            print!("NAMING {dnid}:");
            for dev in &geom.devices {
                if dev.naming == dnid {
                    print!(" {dev}", dev = dev.name);
                }
            }
            println!();
            if args.namings {
                // XXX pretty
                println!("{dn:#?}");
            }
        }
    }
    Ok(())
}
