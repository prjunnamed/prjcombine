use clap::{Parser, Subcommand};
use prjcombine_interconnect::db::IntDb;
use prjcombine_re_xilinx_geom::{Device, DeviceBond, GeomDb};
use prjcombine_re_xilinx_rawdump::Part;
use simple_error::bail;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Mutex;
use std_semaphore::Semaphore;

mod db;
mod spartan6;
mod ultrascale;
mod versal;
mod virtex;
mod virtex2;
mod virtex4;
mod virtex5;
mod virtex6;
mod virtex7;
mod xc4000;
mod xc5200;

#[derive(Debug, Parser)]
#[command(
    name = "prjcombine_xilinx_rd2geom",
    about = "Extract geometry information from rawdumps."
)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Convert {
        dst: PathBuf,
        files: Vec<PathBuf>,
        #[arg(long)]
        no_verify: bool,
    },
    Merge {
        dst: PathBuf,
        files: Vec<PathBuf>,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    match args.command {
        Command::Convert {
            dst,
            files,
            no_verify,
        } => {
            if files.is_empty() {
                bail!("no files given");
            }
            let builder = Mutex::new(db::DbBuilder::new());
            let rb = &builder;
            let sema = Semaphore::new(std::thread::available_parallelism().unwrap().get() as isize);
            let verify = !no_verify;
            std::thread::scope(|s| {
                for fname in files {
                    let guard = sema.access();
                    let tname = fname.file_stem().unwrap().to_str().unwrap();
                    std::thread::Builder::new()
                        .name(tname.to_string())
                        .spawn_scoped(s, move || {
                            let rd = Part::from_file(fname).unwrap();
                            println!("INGEST {} {:?}", rd.part, rd.source);
                            let pre = match &rd.family[..] {
                                "xc4000e" | "xc4000ex" | "xc4000xla" | "xc4000xv" | "spartanxl" => {
                                    xc4000::ingest(&rd, verify)
                                }
                                "xc5200" => xc5200::ingest(&rd, verify),
                                "virtex" | "virtexe" => virtex::ingest(&rd, verify),
                                "virtex2" | "virtex2p" | "spartan3" | "spartan3e" | "spartan3a"
                                | "spartan3adsp" | "fpgacore" => virtex2::ingest(&rd, verify),
                                "spartan6" => spartan6::ingest(&rd, verify),
                                "virtex4" => virtex4::ingest(&rd, verify),
                                "virtex5" => virtex5::ingest(&rd, verify),
                                "virtex6" => virtex6::ingest(&rd, verify),
                                "virtex7" => virtex7::ingest(&rd, verify),
                                "ultrascale" | "ultrascaleplus" => ultrascale::ingest(&rd, verify),
                                "versal" => versal::ingest(&rd, verify),
                                _ => panic!("unknown family {}", rd.family),
                            };
                            let mut builder = rb.lock().unwrap();
                            builder.ingest(pre);
                            std::mem::drop(guard);
                        })
                        .unwrap();
                }
            });
            let db = builder.into_inner().unwrap().finish();
            db.to_file(dst)?;
        }
        Command::Merge { dst, files } => {
            let mut builder = db::DbBuilder::new();
            for fname in files {
                let mut dbin = GeomDb::from_file(fname)?;
                let chip_xlat = dbin.chips.into_map_values(|chip| builder.insert_chip(chip));
                let bond_xlat = dbin.bonds.into_map_values(|bond| builder.insert_bond(bond));
                let interposer_xlat = dbin
                    .interposers
                    .into_map_values(|interposer| builder.insert_interposer(interposer));
                let dev_naming_xlat = dbin
                    .dev_namings
                    .into_map_values(|dev_naming| builder.insert_dev_naming(dev_naming));
                for dev in dbin.devices {
                    builder.devices.push(Device {
                        chips: dev.chips.map_values(|&c| chip_xlat[c]),
                        interposer: interposer_xlat[dev.interposer],
                        bonds: dev.bonds.map_values(|b| DeviceBond {
                            name: b.name.clone(),
                            bond: bond_xlat[b.bond],
                        }),
                        naming: dev_naming_xlat[dev.naming],
                        ..dev
                    });
                }
                for (key, intdb) in dbin.ints {
                    let naming = dbin.namings.remove(&key).unwrap();
                    let init = match key.as_str() {
                        "virtex" => {
                            bincode::decode_from_slice(
                                prjcombine_virtex::defs::INIT,
                                bincode::config::standard(),
                            )
                            .unwrap()
                            .0
                        }
                        "virtex2" => {
                            bincode::decode_from_slice(
                                prjcombine_virtex2::defs::virtex2::INIT,
                                bincode::config::standard(),
                            )
                            .unwrap()
                            .0
                        }
                        "spartan3" => {
                            bincode::decode_from_slice(
                                prjcombine_virtex2::defs::spartan3::INIT,
                                bincode::config::standard(),
                            )
                            .unwrap()
                            .0
                        }
                        "spartan6" => {
                            bincode::decode_from_slice(
                                prjcombine_spartan6::defs::INIT,
                                bincode::config::standard(),
                            )
                            .unwrap()
                            .0
                        }
                        "virtex4" => {
                            bincode::decode_from_slice(
                                prjcombine_virtex4::defs::virtex4::INIT,
                                bincode::config::standard(),
                            )
                            .unwrap()
                            .0
                        }
                        "virtex5" => {
                            bincode::decode_from_slice(
                                prjcombine_virtex4::defs::virtex5::INIT,
                                bincode::config::standard(),
                            )
                            .unwrap()
                            .0
                        }
                        "virtex6" => {
                            bincode::decode_from_slice(
                                prjcombine_virtex4::defs::virtex6::INIT,
                                bincode::config::standard(),
                            )
                            .unwrap()
                            .0
                        }
                        "virtex7" => {
                            bincode::decode_from_slice(
                                prjcombine_virtex4::defs::virtex7::INIT,
                                bincode::config::standard(),
                            )
                            .unwrap()
                            .0
                        }
                        _ => IntDb::default(),
                    };
                    builder.ingest_int(key, init, intdb, naming);
                }
                assert!(dbin.namings.is_empty());
                for (_, name, gtz) in dbin.gtz.gtz {
                    if let Some(ogtz) = builder.gtz.gtz.get(&name) {
                        assert_eq!(ogtz.1, &gtz);
                    } else {
                        builder.gtz.gtz.insert(name, gtz);
                    }
                }
            }
            let db = builder.finish();
            db.to_file(dst)?;
        }
    }
    Ok(())
}
