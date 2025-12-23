use clap::{Arg, ArgAction, Command, value_parser};
use prjcombine_siliconblue::db::Database;
use prjcombine_types::bsdata::TileItemKind;
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
        .arg(
            Arg::new("speed")
                .short('s')
                .long("speed")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("tiledb")
                .short('t')
                .long("tiledb")
                .action(ArgAction::SetTrue),
        )
        .get_matches();
    let arg_db = m.get_one::<PathBuf>("db").unwrap();
    let flag_intdb = m.get_flag("intdb");
    let flag_devices = m.get_flag("devices");
    let flag_chips = m.get_flag("chips");
    let flag_packages = m.get_flag("packages");
    let flag_speed = m.get_flag("speed");
    let flag_tiledb = m.get_flag("tiledb");

    let db = Database::from_file(arg_db)?;
    if flag_intdb {
        println!("INTDB");
        db.int.print(&mut std::io::stdout())?;
    }
    if flag_chips || flag_devices {
        for (cid, chip) in &db.chips {
            print!("CHIP {cid}:");
            for dev in &db.devices {
                if dev.chip == cid {
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
            for dev in &db.devices {
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
    if flag_speed || flag_devices {
        for (sid, speed) in &db.speeds {
            print!("SPEED {sid}:");
            for dev in &db.devices {
                for (sname, &dspeed) in &dev.speeds {
                    if dspeed == sid {
                        print!(" {dev}-{sname}", dev = dev.name);
                    }
                }
            }
            println!();
            if flag_speed {
                print!("{speed}");
            }
        }
    }
    if flag_devices {
        for dev in &db.devices {
            println!("DEVICE {n} GRID {g}", n = dev.name, g = dev.chip);
            for (pkg, bond) in &dev.bonds {
                println!("\tBOND {pkg}: {bond}");
            }
            for (speed, sid) in &dev.speeds {
                println!("\tSPEED {speed}: {sid}");
            }
            for temp in &dev.temps {
                println!("\tTEMP {temp}");
            }
        }
    }
    if flag_tiledb {
        for (tname, tile) in &db.bsdata.tiles {
            println!("tile {tname}:");
            for (key, item) in &tile.items {
                print!("\t{key}:");
                for bit in item.bits.iter().rev() {
                    print!(" {bit:?}");
                }
                match item.kind {
                    TileItemKind::Enum { ref values } => {
                        println!();
                        for (key, val) in values {
                            print!("\t\t");
                            for bit in val.iter().rev() {
                                print!("{}", usize::from(bit));
                            }
                            println!(": {key}");
                        }
                    }
                    TileItemKind::BitVec { ref invert } => {
                        print!(" inv ");
                        for bit in invert.iter().rev() {
                            print!("{}", usize::from(bit));
                        }
                        println!();
                    }
                }
            }
        }
    }
    Ok(())
}
