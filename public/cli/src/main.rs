use clap::{Arg, ArgAction, Command, value_parser};
use prjcombine_types::db::DumpFlags;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let m = Command::new("prjcombine-cli")
        .subcommand_required(true)
        .subcommand(
            Command::new("dumpdb")
                .arg(
                    Arg::new("target")
                        .required(true)
                        .value_parser(value_parser!(String)),
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
                    Arg::new("bonds")
                        .short('b')
                        .long("bonds")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("speed")
                        .short('s')
                        .long("speed")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("bsdata")
                        .short('B')
                        .long("bsdata")
                        .action(ArgAction::SetTrue),
                ),
        )
        .get_matches();
    match m.subcommand() {
        Some(("dumpdb", m)) => {
            let arg_target = m.get_one::<String>("target").unwrap();
            let flag_intdb = m.get_flag("intdb");
            let flag_devices = m.get_flag("devices");
            let flag_chips = m.get_flag("chips");
            let flag_bonds = m.get_flag("bonds");
            let flag_speed = m.get_flag("speed");
            let flag_bsdata = m.get_flag("bsdata");

            let mut flags = DumpFlags {
                intdb: flag_intdb,
                chip: flag_chips,
                bond: flag_bonds,
                device: flag_devices,
                speed: flag_speed,
                bsdata: flag_bsdata,
            };
            if !flag_intdb
                && !flag_chips
                && !flag_bonds
                && !flag_devices
                && !flag_speed
                && !flag_bsdata
            {
                flags = DumpFlags::all();
            }
            match arg_target.as_str() {
                "xc2000" | "xc3000" | "xc3000a" | "xc4000" | "xc4000a" | "xc4000h" | "xc4000e"
                | "xc4000ex" | "xc4000xla" | "xc4000xv" | "spartanxl" | "xc5200" => {
                    let db = prjcombine_xc2000::db::Database::from_file(format!(
                        "../databases/{arg_target}.zstd"
                    ))?;
                    db.dump(&mut std::io::stdout(), flags)?;
                }
                "virtex" => {
                    let db =
                        prjcombine_virtex::db::Database::from_file("../databases/virtex.zstd")?;
                    db.dump(&mut std::io::stdout(), flags)?;
                }
                "virtex2" | "spartan3" | "fpgacore" => {
                    let db = prjcombine_virtex2::db::Database::from_file(format!(
                        "../databases/{arg_target}.zstd"
                    ))?;
                    db.dump(&mut std::io::stdout(), flags)?;
                }
                "spartan6" => {
                    let db =
                        prjcombine_spartan6::db::Database::from_file("../databases/spartan6.zstd")?;
                    db.dump(&mut std::io::stdout(), flags)?;
                }
                "virtex4" | "virtex5" | "virtex6" | "virtex7" => {
                    let db = prjcombine_virtex4::db::Database::from_file(format!(
                        "../databases/{arg_target}.zstd"
                    ))?;
                    db.dump(&mut std::io::stdout(), flags)?;
                }
                "ultrascale" | "ultrascaleplus" => {
                    let db = prjcombine_ultrascale::db::Database::from_file(format!(
                        "../databases/{arg_target}.zstd"
                    ))?;
                    db.dump(&mut std::io::stdout(), flags)?;
                }
                "versal" => {
                    todo!()
                }
                "siliconblue" => {
                    let db = prjcombine_siliconblue::db::Database::from_file(
                        "../databases/siliconblue.zstd",
                    )?;
                    db.dump(&mut std::io::stdout(), flags)?;
                }
                "ecp" | "xp" | "machxo" | "ecp2" | "ecp2m" | "xp2" | "ecp3" | "machxo2"
                | "ecp4" | "scm" | "ecp5" | "crosslink" => {
                    let db = prjcombine_ecp::db::Database::from_file(format!(
                        "../databases/{arg_target}.zstd"
                    ))?;
                    db.dump(&mut std::io::stdout(), flags)?;
                }
                "xc9500" | "xc9500xl" | "xc9500xv" => {
                    let db = prjcombine_xc9500::Database::from_file(format!(
                        "../databases/{arg_target}.zstd"
                    ))?;
                    db.dump(&mut std::io::stdout(), flags)?;
                }
                "xpla3" => {
                    let db = prjcombine_xpla3::Database::from_file("../databases/xpla3.zstd")?;
                    db.dump(&mut std::io::stdout(), flags)?;
                }
                "coolrunner2" => {
                    let db = prjcombine_coolrunner2::Database::from_file(
                        "../databases/coolrunner2.zstd",
                    )?;
                    db.dump(&mut std::io::stdout(), flags)?;
                }
                _ => panic!("unknown target {arg_target}"),
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}
