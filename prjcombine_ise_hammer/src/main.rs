use clap::Parser;
use prjcombine_hammer::{Backend, Session};
use prjcombine_toolchain::Toolchain;
use prjcombine_types::TileItemKind;
use prjcombine_xilinx_geom::{ExpandedDevice, GeomDb};
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use tiledb::TileDb;

mod backend;
mod bram;
mod clb;
mod clk;
mod diff;
mod dsp;
mod fgen;
mod fuzz;
mod gt;
mod int;
mod intf;
mod io;
mod misc;
mod ppc;
mod tiledb;

use backend::IseBackend;

use crate::diff::CollectorCtx;

#[derive(Debug, Parser)]
#[command(name = "ise_hammer", about = "Swing the Massive Hammer on ISE parts.")]
struct Args {
    toolchain: PathBuf,
    geomdb: PathBuf,
    json: PathBuf,
    parts: Vec<String>,
    #[arg(long)]
    skip_io: bool,
    #[arg(long)]
    no_dup: bool,
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
        let empty_bs = backend.bitgen(&HashMap::new());
        let mut hammer = Session::new(&backend);
        if args.no_dup {
            hammer.dup_factor = 1;
        }
        int::add_fuzzers(&mut hammer, &backend);
        let mut skip_io = args.skip_io;
        // sigh. Spartan 3AN cannot do VCCAUX == 2.5 and this causes a *ridiculously annoying*
        // problem in the I/O fuzzers, which cannot easily identify IBUF_MODE == CMOS_VCCO
        // without that. it could be fixed, but it's easier to just rely on the fuzzers being
        // run for plain Spartan 3A. it's the same die anyway.
        if part.name.starts_with("xc3s") && part.name.ends_with('n') {
            skip_io = true;
        }
        // ISE just segfaults on this device under complex microarchitectural conditions,
        // the exact nature of which is unknown, but I/O-related.
        // fuck this shit and just skip the whole thing, I'm here to reverse FPGAs not ISE bugs.
        if part.name == "xc3s2000" {
            skip_io = true;
        }
        match gedev {
            ExpandedDevice::Xc4k(_) => {}
            ExpandedDevice::Xc5200(_) => {}
            ExpandedDevice::Virtex(_) => {
                clb::virtex::add_fuzzers(&mut hammer, &backend);
                bram::virtex::add_fuzzers(&mut hammer, &backend);
            }
            ExpandedDevice::Virtex2(ref edev) => {
                clb::virtex2::add_fuzzers(&mut hammer, &backend);
                clk::virtex2::add_fuzzers(&mut hammer, &backend);
                bram::virtex2::add_fuzzers(&mut hammer, &backend);
                if edev.grid.kind == prjcombine_virtex2::grid::GridKind::Spartan3ADsp {
                    dsp::spartan3adsp::add_fuzzers(&mut hammer, &backend);
                }
                misc::virtex2::add_fuzzers(&mut hammer, &backend, skip_io);
                if !skip_io {
                    io::virtex2::add_fuzzers(&mut hammer, &backend);
                }
                if edev.grid.kind.is_virtex2p() {
                    ppc::virtex2::add_fuzzers(&mut hammer, &backend);
                }
                if edev.grid.kind == prjcombine_virtex2::grid::GridKind::Virtex2P {
                    gt::virtex2p::add_fuzzers(&mut hammer, &backend);
                }
                if edev.grid.kind == prjcombine_virtex2::grid::GridKind::Virtex2PX {
                    gt::virtex2px::add_fuzzers(&mut hammer, &backend);
                }
            }
            ExpandedDevice::Spartan6(_) => {
                clb::virtex5::add_fuzzers(&mut hammer, &backend);
                dsp::spartan3adsp::add_fuzzers(&mut hammer, &backend);
            }
            ExpandedDevice::Virtex4(ref edev) => match edev.kind {
                prjcombine_virtex4::grid::GridKind::Virtex4 => {
                    clb::virtex2::add_fuzzers(&mut hammer, &backend);
                    clk::virtex4::add_fuzzers(&mut hammer, &backend);
                    bram::virtex4::add_fuzzers(&mut hammer, &backend);
                    dsp::virtex4::add_fuzzers(&mut hammer, &backend);
                    ppc::virtex4::add_fuzzers(&mut hammer, &backend);
                    misc::virtex4::add_fuzzers(&mut hammer, &backend);
                }
                prjcombine_virtex4::grid::GridKind::Virtex5 => {
                    clb::virtex5::add_fuzzers(&mut hammer, &backend);
                    dsp::virtex5::add_fuzzers(&mut hammer, &backend);
                }
                prjcombine_virtex4::grid::GridKind::Virtex6 => {
                    clb::virtex5::add_fuzzers(&mut hammer, &backend);
                    dsp::virtex6::add_fuzzers(&mut hammer, &backend);
                }
                prjcombine_virtex4::grid::GridKind::Virtex7 => {
                    clb::virtex5::add_fuzzers(&mut hammer, &backend);
                    dsp::virtex6::add_fuzzers(&mut hammer, &backend);
                }
            },
            ExpandedDevice::Ultrascale(_) => panic!("ultrascale not supported by ISE"),
            ExpandedDevice::Versal(_) => panic!("versal not supported by ISE"),
        }
        intf::add_fuzzers(&mut hammer, &backend);
        let mut state = hammer.run().unwrap();
        let mut ctx = CollectorCtx {
            device: part,
            edev: &gedev,
            db: &db,
            state: &mut state,
            tiledb: &mut tiledb,
            empty_bs: &empty_bs,
        };
        int::collect_fuzzers(&mut ctx);
        match gedev {
            ExpandedDevice::Xc4k(_) => {}
            ExpandedDevice::Xc5200(_) => {}
            ExpandedDevice::Virtex(_) => {
                clb::virtex::collect_fuzzers(&mut ctx);
                bram::virtex::collect_fuzzers(&mut ctx);
            }
            ExpandedDevice::Virtex2(ref edev) => {
                clb::virtex2::collect_fuzzers(&mut ctx);
                clk::virtex2::collect_fuzzers(&mut ctx);
                bram::virtex2::collect_fuzzers(&mut ctx);
                if edev.grid.kind == prjcombine_virtex2::grid::GridKind::Spartan3ADsp {
                    dsp::spartan3adsp::collect_fuzzers(&mut ctx);
                }
                misc::virtex2::collect_fuzzers(&mut ctx, skip_io);
                if !skip_io {
                    io::virtex2::collect_fuzzers(&mut ctx);
                }
                if edev.grid.kind.is_virtex2p() {
                    ppc::virtex2::collect_fuzzers(&mut ctx);
                }
                if edev.grid.kind == prjcombine_virtex2::grid::GridKind::Virtex2P {
                    gt::virtex2p::collect_fuzzers(&mut ctx);
                }
                if edev.grid.kind == prjcombine_virtex2::grid::GridKind::Virtex2PX {
                    gt::virtex2px::collect_fuzzers(&mut ctx);
                }
            }
            ExpandedDevice::Spartan6(_) => {
                clb::virtex5::collect_fuzzers(&mut ctx);
                dsp::spartan3adsp::collect_fuzzers(&mut ctx)
            }
            ExpandedDevice::Virtex4(ref edev) => match edev.kind {
                prjcombine_virtex4::grid::GridKind::Virtex4 => {
                    clb::virtex2::collect_fuzzers(&mut ctx);
                    clk::virtex4::collect_fuzzers(&mut ctx);
                    bram::virtex4::collect_fuzzers(&mut ctx);
                    dsp::virtex4::collect_fuzzers(&mut ctx);
                    misc::virtex4::collect_fuzzers(&mut ctx);
                    ppc::virtex4::collect_fuzzers(&mut ctx);
                }
                prjcombine_virtex4::grid::GridKind::Virtex5 => {
                    clb::virtex5::collect_fuzzers(&mut ctx);
                    dsp::virtex5::collect_fuzzers(&mut ctx);
                }
                prjcombine_virtex4::grid::GridKind::Virtex6 => {
                    clb::virtex5::collect_fuzzers(&mut ctx);
                    dsp::virtex6::collect_fuzzers(&mut ctx);
                }
                prjcombine_virtex4::grid::GridKind::Virtex7 => {
                    clb::virtex5::collect_fuzzers(&mut ctx);
                    dsp::virtex6::collect_fuzzers(&mut ctx);
                }
            },
            ExpandedDevice::Ultrascale(_) => panic!("ultrascale not supported by ISE"),
            ExpandedDevice::Versal(_) => panic!("versal not supported by ISE"),
        }
        intf::collect_fuzzers(&mut ctx);

        for (feat, data) in &ctx.state.simple_features {
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
                if let TileItemKind::BitVec { ref invert } = item.kind {
                    if invert.iter().all(|x| *x) {
                        print!(" INVVEC");
                    } else if invert.iter().all(|x| !*x) {
                        print!(" VEC");
                    } else {
                        print!(" MIXVEC {:?}", Vec::from_iter(invert.iter().map(|x| *x)));
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
