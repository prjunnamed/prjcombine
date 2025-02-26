use clap::{Arg, ArgAction, Command, value_parser};
use prjcombine_siliconblue::db::Database;
use std::{error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let m = Command::new("sbprint")
        .arg(
            Arg::new("db")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("intdb")
                .short('i')
                .long("intdb")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("devices")
                .short('d')
                .long("devices")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("chips")
                .short('c')
                .long("chips")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("packages")
                .short('p')
                .long("packages")
                .action(ArgAction::SetTrue),
        )
        .get_matches();
    let arg_db = m.get_one::<PathBuf>("db").unwrap();
    let flag_intdb = m.get_flag("intdb");
    let flag_devices = m.get_flag("devices");
    let flag_chips = m.get_flag("chips");
    let flag_packages = m.get_flag("packages");

    let db = Database::from_file(arg_db)?;
    if flag_intdb {
        println!("INTDB");
        db.int.print(&mut std::io::stdout())?;
    }
    if flag_chips || flag_devices {
        for (gid, chip) in &db.chips {
            print!("GRID {gid}:");
            for dev in &db.parts {
                if dev.chip == gid {
                    print!(" {dev}", dev = dev.name);
                }
            }
            println!();
            if flag_chips {
                print!("{chip}");
            }
        }
    }
    if flag_packages || flag_devices {
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
            if flag_packages {
                print!("{bond}");
            }
        }
    }
    if flag_devices {
        for dev in &db.parts {
            println!("DEVICE {n} GRID {g}", n = dev.name, g = dev.chip);
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
