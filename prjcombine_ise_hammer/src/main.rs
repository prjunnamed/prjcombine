use bitvec::vec::BitVec;
use clap::Parser;
use prjcombine_hammer::{Backend, Session};
use prjcombine_toolchain::Toolchain;
use prjcombine_virtex_bitstream::Reg;
use prjcombine_xilinx_geom::{ExpandedDevice, GeomDb};
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use tiledb::TileDb;

mod backend;
mod bram;
mod ccm;
mod clb;
mod clk;
mod dcm;
mod diff;
mod dsp;
mod emac;
mod fgen;
mod fuzz;
mod gt;
mod int;
mod intf;
mod io;
mod misc;
mod pcie;
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
    skip_clk: bool,
    #[arg(long)]
    skip_ccm: bool,
    #[arg(long)]
    skip_dcm: bool,
    #[arg(long)]
    skip_misc: bool,
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
        let mut hammer = Session::new(&backend);
        if args.no_dup {
            hammer.dup_factor = 1;
        }
        int::add_fuzzers(&mut hammer, &backend);
        let mut skip_io = args.skip_io;
        let mut skip_dcm = args.skip_dcm;
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
        // Apparently ISE applies virtex2 programming to those instead of virtex2p programming.
        // Just skip.
        if part.name.starts_with("xq2vp") {
            skip_dcm = true;
        }
        match gedev {
            ExpandedDevice::Xc4k(_) => {}
            ExpandedDevice::Xc5200(_) => {
                // TODO: int
                // TODO: clb
                // TODO: clk
                // TODO: misc
                // TODO: io
            }
            ExpandedDevice::Virtex(_) => {
                // TODO: int
                clb::virtex::add_fuzzers(&mut hammer, &backend);
                // TODO: clk
                bram::virtex::add_fuzzers(&mut hammer, &backend);
                // TODO: misc
                // TODO: io
                // TODO: dll
            }
            ExpandedDevice::Virtex2(ref edev) => {
                clb::virtex2::add_fuzzers(&mut hammer, &backend);
                clk::virtex2::add_fuzzers(&mut hammer, &backend);
                bram::virtex2::add_fuzzers(&mut hammer, &backend);
                if edev.grid.kind == prjcombine_virtex2::grid::GridKind::Spartan3ADsp {
                    dsp::spartan3adsp::add_fuzzers(&mut hammer, &backend);
                }
                if !args.skip_misc {
                    misc::virtex2::add_fuzzers(&mut hammer, &backend, skip_io);
                }
                if !skip_io {
                    io::virtex2::add_fuzzers(&mut hammer, &backend);
                }
                if !skip_dcm {
                    if !edev.grid.kind.is_spartan3ea() {
                        dcm::virtex2::add_fuzzers(&mut hammer, &backend);
                    } else {
                        dcm::spartan3e::add_fuzzers(&mut hammer, &backend);
                    }
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
                if !args.skip_clk {
                    clk::spartan6::add_fuzzers(&mut hammer, &backend);
                }
                bram::spartan6::add_fuzzers(&mut hammer, &backend);
                dsp::spartan3adsp::add_fuzzers(&mut hammer, &backend);
                // TODO: misc
                // TODO: io
                // TODO: mcb
                // TODO: dcm
                // TODO: pll
                pcie::spartan6::add_fuzzers(&mut hammer, &backend);
                // TODO: gtp
            }
            ExpandedDevice::Virtex4(ref edev) => match edev.kind {
                prjcombine_virtex4::grid::GridKind::Virtex4 => {
                    clb::virtex2::add_fuzzers(&mut hammer, &backend);
                    if !args.skip_clk {
                        clk::virtex4::add_fuzzers(&mut hammer, &backend);
                    }
                    bram::virtex4::add_fuzzers(&mut hammer, &backend);
                    dsp::virtex4::add_fuzzers(&mut hammer, &backend);
                    if !args.skip_misc {
                        misc::virtex4::add_fuzzers(&mut hammer, &backend);
                    }
                    // TODO: io
                    if !args.skip_dcm {
                        dcm::virtex4::add_fuzzers(&mut hammer, &backend);
                    }
                    if !args.skip_ccm {
                        ccm::virtex4::add_fuzzers(&mut hammer, &backend);
                    }
                    ppc::virtex4::add_fuzzers(&mut hammer, &backend);
                    // TODO: gt
                }
                prjcombine_virtex4::grid::GridKind::Virtex5 => {
                    clb::virtex5::add_fuzzers(&mut hammer, &backend);
                    if !args.skip_clk {
                        clk::virtex5::add_fuzzers(&mut hammer, &backend);
                    }
                    bram::virtex5::add_fuzzers(&mut hammer, &backend);
                    dsp::virtex5::add_fuzzers(&mut hammer, &backend);
                    if !args.skip_misc {
                        misc::virtex5::add_fuzzers(&mut hammer, &backend);
                    }
                    // TODO: io
                    // TODO: dcm
                    // TODO: pll
                    ppc::virtex5::add_fuzzers(&mut hammer, &backend);
                    emac::virtex5::add_fuzzers(&mut hammer, &backend);
                    pcie::virtex5::add_fuzzers(&mut hammer, &backend);
                    // TODO: gtp
                    // TODO: gtx
                }
                prjcombine_virtex4::grid::GridKind::Virtex6 => {
                    clb::virtex5::add_fuzzers(&mut hammer, &backend);
                    // TODO: clk
                    bram::virtex6::add_fuzzers(&mut hammer, &backend);
                    dsp::virtex6::add_fuzzers(&mut hammer, &backend);
                    // TODO: misc
                    // TODO: io
                    // TODO: pll
                    emac::virtex6::add_fuzzers(&mut hammer, &backend);
                    pcie::virtex6::add_fuzzers(&mut hammer, &backend);
                    // TODO: gtx
                    // TODO: gth
                }
                prjcombine_virtex4::grid::GridKind::Virtex7 => {
                    clb::virtex5::add_fuzzers(&mut hammer, &backend);
                    // TODO: clk
                    bram::virtex6::add_fuzzers(&mut hammer, &backend);
                    dsp::virtex6::add_fuzzers(&mut hammer, &backend);
                    // TODO: misc
                    // TODO: io
                    // TODO: io_fifo
                    // TODO: pll
                    pcie::virtex7::add_fuzzers(&mut hammer, &backend);
                    // TODO: gtp
                    // TODO: gtx
                    // TODO: gth
                }
            },
            ExpandedDevice::Ultrascale(_) => panic!("ultrascale not supported by ISE"),
            ExpandedDevice::Versal(_) => panic!("versal not supported by ISE"),
        }
        intf::add_fuzzers(&mut hammer, &backend);
        let (empty_bs, mut state) = std::thread::scope(|s| {
            let empty_bs_t = s.spawn(|| backend.bitgen(&HashMap::new()));
            let state = hammer.run().unwrap();
            let empty_bs = empty_bs_t.join().unwrap();
            (empty_bs, state)
        });
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
                if !args.skip_misc {
                    misc::virtex2::collect_fuzzers(&mut ctx, skip_io);
                }
                if !skip_io {
                    io::virtex2::collect_fuzzers(&mut ctx);
                }
                if !skip_dcm {
                    if !edev.grid.kind.is_spartan3ea() {
                        dcm::virtex2::collect_fuzzers(&mut ctx);
                    } else {
                        dcm::spartan3e::collect_fuzzers(&mut ctx);
                    }
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
                if !args.skip_clk {
                    clk::spartan6::collect_fuzzers(&mut ctx);
                }
                bram::spartan6::collect_fuzzers(&mut ctx);
                dsp::spartan3adsp::collect_fuzzers(&mut ctx);
                pcie::spartan6::collect_fuzzers(&mut ctx);
            }
            ExpandedDevice::Virtex4(ref edev) => match edev.kind {
                prjcombine_virtex4::grid::GridKind::Virtex4 => {
                    clb::virtex2::collect_fuzzers(&mut ctx);
                    if !args.skip_clk {
                        clk::virtex4::collect_fuzzers(&mut ctx);
                    }
                    bram::virtex4::collect_fuzzers(&mut ctx);
                    dsp::virtex4::collect_fuzzers(&mut ctx);
                    if !args.skip_misc {
                        misc::virtex4::collect_fuzzers(&mut ctx);
                    }
                    if !args.skip_dcm {
                        dcm::virtex4::collect_fuzzers(&mut ctx);
                    }
                    if !args.skip_ccm {
                        ccm::virtex4::collect_fuzzers(&mut ctx);
                    }
                    ppc::virtex4::collect_fuzzers(&mut ctx);
                }
                prjcombine_virtex4::grid::GridKind::Virtex5 => {
                    clb::virtex5::collect_fuzzers(&mut ctx);
                    if !args.skip_clk {
                        clk::virtex5::collect_fuzzers(&mut ctx);
                    }
                    bram::virtex5::collect_fuzzers(&mut ctx);
                    dsp::virtex5::collect_fuzzers(&mut ctx);
                    if !args.skip_misc {
                        misc::virtex5::collect_fuzzers(&mut ctx);
                    }
                    ppc::virtex5::collect_fuzzers(&mut ctx);
                    emac::virtex5::collect_fuzzers(&mut ctx);
                    pcie::virtex5::collect_fuzzers(&mut ctx);
                }
                prjcombine_virtex4::grid::GridKind::Virtex6 => {
                    clb::virtex5::collect_fuzzers(&mut ctx);
                    bram::virtex6::collect_fuzzers(&mut ctx);
                    dsp::virtex6::collect_fuzzers(&mut ctx);
                    emac::virtex6::collect_fuzzers(&mut ctx);
                    pcie::virtex6::collect_fuzzers(&mut ctx);
                }
                prjcombine_virtex4::grid::GridKind::Virtex7 => {
                    clb::virtex5::collect_fuzzers(&mut ctx);
                    bram::virtex6::collect_fuzzers(&mut ctx);
                    dsp::virtex6::collect_fuzzers(&mut ctx);
                    pcie::virtex7::collect_fuzzers(&mut ctx);
                }
            },
            ExpandedDevice::Ultrascale(_) => panic!("ultrascale not supported by ISE"),
            ExpandedDevice::Versal(_) => panic!("versal not supported by ISE"),
        }
        intf::collect_fuzzers(&mut ctx);
        for (die, dbs) in &ctx.empty_bs.die {
            if let Some(val) = dbs.regs[Reg::Idcode] {
                let mut idcode = BitVec::new();
                for i in 0..32 {
                    idcode.push((val & 1 << i) != 0);
                }
                ctx.tiledb
                    .insert_device_data(&part.name, format!("IDCODE:{die}"), idcode);
            }
        }

        for (feat, data) in &ctx.state.simple_features {
            println!(
                "{} {} {} {}: {:?}",
                feat.tile, feat.bel, feat.attr, feat.val, data.diffs
            );
        }
    }
    // inter-part fixups!
    for part in Vec::from_iter(tiledb.device_data.keys().cloned()) {
        if part.starts_with("xq2vp") {
            let xc_part = "xc".to_string() + &part[2..];
            if tiledb.device_data.contains_key(&xc_part) {
                for key in ["DCM:DESKEW_ADJUST", "DCM:VBG_SEL", "DCM:VBG_PD"] {
                    if let Some(val) = tiledb.device_data[&xc_part].get(key) {
                        let val = val.clone();
                        tiledb.insert_device_data(&part, key, val);
                    }
                }
            }
        }
    }
    if let Some(tile) = tiledb.tiles.get("INT.GT.CLKPAD") {
        let dcmclk0 = tile.items["INT:INV.IMUX.DCMCLK0"].clone();
        let dcmclk1 = tile.items["INT:INV.IMUX.DCMCLK1"].clone();
        let dcmclk2 = tile.items["INT:INV.IMUX.DCMCLK2"].clone();
        for tile in ["INT.DCM.V2", "INT.DCM.V2P"] {
            if tiledb.tiles.contains_key(tile) {
                tiledb.insert(tile, "INT", "INV.IMUX.DCMCLK0", dcmclk0.clone());
                tiledb.insert(tile, "INT", "INV.IMUX.DCMCLK1", dcmclk1.clone());
                tiledb.insert(tile, "INT", "INV.IMUX.DCMCLK2", dcmclk2.clone());
            }
        }
    }
    std::fs::write(args.json, tiledb.to_json().to_string())?;
    Ok(())
}
