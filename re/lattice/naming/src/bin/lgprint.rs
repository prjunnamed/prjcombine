use clap::Parser;
use prjcombine_re_lattice_naming::Database;
use std::{error::Error, path::PathBuf};

#[derive(Debug, Parser)]
#[command(name = "lgprint", about = "Dump Lattice geom file.")]
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
    let db = Database::from_file(args.file)?;
    if args.intdb {
        println!("INTDB");
        db.int.print(&mut std::io::stdout())?;
    }
    if args.chips || args.devices || args.namings {
        for (cid, (chip, naming)) in &db.chips {
            print!("CHIP {cid}:");
            for dev in &db.devices {
                if dev.chip == cid {
                    print!(" {dev}", dev = dev.name);
                }
            }
            println!();
            if args.chips {
                print!("{chip}");
            }
            if args.namings {
                for (&wire, &name) in &naming.interconnect {
                    println!(
                        "\t{w}: {n}",
                        w = wire.to_string(&db.int),
                        n = name.to_string(naming)
                    );
                }
                for (&bel, bnaming) in &naming.bels {
                    print!("\tBEL {b}:", b = bel.to_string(&db.int));
                    for &name in &bnaming.names {
                        print!(" {}", naming.strings[name]);
                    }
                    println!();
                    for (&pin, &name) in &bnaming.wires {
                        println!(
                            "\t\t{p}: {n}",
                            p = naming.strings[pin],
                            n = name.to_string(naming)
                        );
                    }
                }
            }
        }
    }
    if args.pkgs || args.devices {
        for (bid, bond) in &db.bonds {
            print!("BOND {bid}:");
            for dev in &db.devices {
                for (_, pname, &dbond) in &dev.bonds {
                    if dbond == bid {
                        print!(" {dev}-{pkg}", dev = dev.name, pkg = pname);
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
        for dev in &db.devices {
            println!("DEVICE {n} CHIP {chip}", n = dev.name, chip = dev.chip,);
            for (_, pname, bond) in &dev.bonds {
                println!("\tBOND {pname}: {bond}");
            }
            for combo in &dev.combos {
                println!(
                    "\tPART: {bn} {sn}",
                    bn = dev.bonds.key(combo.devbond),
                    sn = dev.speeds[combo.speed]
                );
            }
        }
    }
    Ok(())
}
