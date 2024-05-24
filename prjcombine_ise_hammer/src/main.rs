use clap::Parser;
use prjcombine_hammer::Session;
use prjcombine_toolchain::Toolchain;
use prjcombine_types::TileItemKind;
use prjcombine_xilinx_geom::{ExpandedDevice, GeomDb};
use std::error::Error;
use std::path::PathBuf;
use tiledb::TileDb;

mod backend;
mod bram;
mod clb;
mod diff;
mod dsp;
mod fgen;
mod fuzz;
mod tiledb;

use backend::IseBackend;

#[derive(Debug, Parser)]
#[command(name = "ise_hammer", about = "Swing the Massive Hammer on ISE parts.")]
struct Args {
    toolchain: PathBuf,
    geomdb: PathBuf,
    json: PathBuf,
    parts: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let tc = Toolchain::from_file(&args.toolchain)?;
    let db = GeomDb::from_file(args.geomdb)?;
    let mut tiledb = TileDb::new();
    for part in &db.devices {
        if !args.parts.is_empty() && !args.parts.contains(&part.name) {
            continue;
        }
        println!("part {name}", name = part.name);
        let gedev = db.expand_grid(part);
        let backend = IseBackend {
            tc: &tc,
            db: &db,
            device: part,
            bs_geom: gedev.bs_geom(),
            egrid: gedev.egrid(),
            edev: &gedev,
        };
        let mut hammer = Session::new(&backend);
        match gedev {
            ExpandedDevice::Xc4k(_) => {}
            ExpandedDevice::Xc5200(_) => {}
            ExpandedDevice::Virtex(_) => {
                clb::virtex::add_fuzzers(&mut hammer, &backend);
                bram::virtex::add_fuzzers(&mut hammer, &backend);
            }
            ExpandedDevice::Virtex2(ref edev) => {
                clb::virtex2::add_fuzzers(&mut hammer, &backend);
                bram::virtex2::add_fuzzers(&mut hammer, &backend);
                if edev.grid.kind == prjcombine_virtex2::grid::GridKind::Spartan3ADsp {
                    dsp::spartan3adsp::add_fuzzers(
                        &mut hammer,
                        &backend,
                        dsp::spartan3adsp::Mode::Spartan3ADsp,
                    );
                }
            }
            ExpandedDevice::Spartan6(_) => {
                clb::virtex5::add_fuzzers(&mut hammer, &backend);
                dsp::spartan3adsp::add_fuzzers(
                    &mut hammer,
                    &backend,
                    dsp::spartan3adsp::Mode::Spartan6,
                );
            }
            ExpandedDevice::Virtex4(ref edev) => match edev.kind {
                prjcombine_virtex4::grid::GridKind::Virtex4 => {
                    clb::virtex2::add_fuzzers(&mut hammer, &backend);
                    dsp::virtex4::add_fuzzers(&mut hammer, &backend);
                }
                prjcombine_virtex4::grid::GridKind::Virtex5 => {
                    clb::virtex5::add_fuzzers(&mut hammer, &backend);
                    dsp::virtex5::add_fuzzers(&mut hammer, &backend);
                }
                prjcombine_virtex4::grid::GridKind::Virtex6 => {
                    clb::virtex5::add_fuzzers(&mut hammer, &backend);
                }
                prjcombine_virtex4::grid::GridKind::Virtex7 => {
                    clb::virtex5::add_fuzzers(&mut hammer, &backend);
                }
            },
            ExpandedDevice::Ultrascale(_) => panic!("ultrascale not supported by ISE"),
            ExpandedDevice::Versal(_) => panic!("versal not supported by ISE"),
        }
        let mut state = hammer.run().unwrap();
        match gedev {
            ExpandedDevice::Xc4k(_) => {}
            ExpandedDevice::Xc5200(_) => {}
            ExpandedDevice::Virtex(_) => {
                clb::virtex::collect_fuzzers(&mut state, &mut tiledb);
                bram::virtex::collect_fuzzers(backend.egrid, &mut state, &mut tiledb);
            }
            ExpandedDevice::Virtex2(ref edev) => {
                clb::virtex2::collect_fuzzers(
                    &mut state,
                    &mut tiledb,
                    if edev.grid.kind.is_virtex2() {
                        clb::virtex2::Mode::Virtex2
                    } else {
                        clb::virtex2::Mode::Spartan3
                    },
                );
                bram::virtex2::collect_fuzzers(part, &mut state, &mut tiledb, edev.grid.kind);
                if edev.grid.kind == prjcombine_virtex2::grid::GridKind::Spartan3ADsp {
                    dsp::spartan3adsp::collect_fuzzers(
                        &mut state,
                        &mut tiledb,
                        dsp::spartan3adsp::Mode::Spartan3ADsp,
                    )
                }
            }
            ExpandedDevice::Spartan6(_) => {
                clb::virtex5::collect_fuzzers(
                    &mut state,
                    &mut tiledb,
                    clb::virtex5::Mode::Spartan6,
                );
                dsp::spartan3adsp::collect_fuzzers(
                    &mut state,
                    &mut tiledb,
                    dsp::spartan3adsp::Mode::Spartan6,
                )
            }
            ExpandedDevice::Virtex4(ref edev) => match edev.kind {
                prjcombine_virtex4::grid::GridKind::Virtex4 => {
                    clb::virtex2::collect_fuzzers(
                        &mut state,
                        &mut tiledb,
                        clb::virtex2::Mode::Virtex4,
                    );
                    dsp::virtex4::collect_fuzzers(&mut state, &mut tiledb);
                }
                prjcombine_virtex4::grid::GridKind::Virtex5 => {
                    clb::virtex5::collect_fuzzers(
                        &mut state,
                        &mut tiledb,
                        clb::virtex5::Mode::Virtex5,
                    );
                    dsp::virtex5::collect_fuzzers(&mut state, &mut tiledb);
                }
                prjcombine_virtex4::grid::GridKind::Virtex6 => {
                    clb::virtex5::collect_fuzzers(
                        &mut state,
                        &mut tiledb,
                        clb::virtex5::Mode::Virtex6,
                    );
                }
                prjcombine_virtex4::grid::GridKind::Virtex7 => {
                    clb::virtex5::collect_fuzzers(
                        &mut state,
                        &mut tiledb,
                        clb::virtex5::Mode::Virtex7,
                    );
                }
            },
            ExpandedDevice::Ultrascale(_) => panic!("ultrascale not supported by ISE"),
            ExpandedDevice::Versal(_) => panic!("versal not supported by ISE"),
        }

        for (feat, data) in &state.simple_features {
            print!("{} {} {} {}: [", feat.tile, feat.bel, feat.attr, feat.val);
            for diff in &data.diffs {
                if data.diffs.len() != 1 {
                    print!("[");
                }
                for (bit, val) in &diff.bits {
                    print!("{}.{}.{}:{},", bit.tile, bit.frame, bit.bit, val);
                }
                if data.diffs.len() != 1 {
                    print!("], ");
                }
            }
            println!("]");
        }
    }
    std::fs::write(args.json, tiledb.to_json().to_string())?;
    if false {
        for (tname, tile) in &tiledb.tiles {
            for (name, item) in &tile.items {
                print!("ITEM {tname}.{name}:");
                if let TileItemKind::BitVec { invert } = item.kind {
                    if invert {
                        print!(" INVVEC");
                    } else {
                        print!(" VEC");
                    }
                } else {
                    print!(" ENUM");
                }
                for &bit in &item.bits {
                    print!(" {}.{}.{}", bit.tile, bit.frame, bit.bit);
                }
                println!();
                if let TileItemKind::Enum { ref values } = item.kind {
                    for (vname, val) in values {
                        print!("    {vname:10}: ");
                        for b in val {
                            if *b {
                                print!("1");
                            } else {
                                print!("0");
                            }
                        }
                        println!();
                    }
                }
            }
        }
    }
    Ok(())
}
