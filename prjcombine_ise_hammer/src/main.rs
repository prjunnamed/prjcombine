use bitvec::vec::BitVec;
use clap::Parser;
use itertools::Itertools;
use prjcombine_collector::Collector;
use prjcombine_hammer::{Backend, Session};
use prjcombine_toolchain::Toolchain;
use prjcombine_types::tiledb::TileDb;
use prjcombine_virtex_bitstream::Reg;
use prjcombine_xilinx_geom::{Device, ExpandedDevice, GeomDb};
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;

mod backend;
mod bram;
mod ccm;
mod clb;
mod clk;
mod cmt;
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
mod mcb;
mod misc;
mod pcie;
mod pll;
mod ppc;
mod tbus;

use backend::IseBackend;

use crate::diff::CollectorCtx;

#[derive(Debug, Parser)]
#[command(name = "ise_hammer", about = "Swing the Massive Hammer on ISE parts.")]
struct Args {
    toolchain: PathBuf,
    geomdb: PathBuf,
    tiledb: PathBuf,
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
    skip_pll: bool,
    #[arg(long)]
    skip_misc: bool,
    #[arg(long)]
    skip_gt: bool,
    #[arg(long)]
    skip_hard: bool,
    #[arg(long)]
    skip_core: bool,
    #[arg(long)]
    skip_devdata: bool,
    #[arg(long)]
    bali_only: bool,
    #[arg(long)]
    no_dup: bool,
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
    #[arg(long)]
    max_threads: Option<usize>,
}

#[derive(Copy, Clone, Debug)]
struct RunOpts {
    skip_core: bool,
    skip_io: bool,
    skip_clk: bool,
    skip_ccm: bool,
    skip_dcm: bool,
    skip_pll: bool,
    skip_misc: bool,
    skip_gt: bool,
    skip_hard: bool,
    bali_only: bool,
    devdata_only: bool,
    no_dup: bool,
    debug: u8,
    max_threads: Option<usize>,
}

impl RunOpts {
    fn skip_all(&mut self) {
        self.skip_core = true;
        self.skip_io = true;
        self.skip_clk = true;
        self.skip_ccm = true;
        self.skip_dcm = true;
        self.skip_pll = true;
        self.skip_misc = true;
        self.skip_gt = true;
        self.skip_hard = true;
    }
}

fn run(tc: &Toolchain, db: &GeomDb, part: &Device, tiledb: &mut TileDb, opts: &RunOpts) {
    println!("part {name}", name = part.name);
    let mut opts = *opts;
    let gedev = db.expand_grid(part);
    let gendev = db.name(part, &gedev);
    let mut ebonds = HashMap::new();
    for devbond in part.bonds.values() {
        let bond = &db.bonds[devbond.bond];
        ebonds.insert(devbond.name.clone(), bond.expand());
    }
    let backend = IseBackend {
        debug: opts.debug,
        tc,
        db,
        device: part,
        bs_geom: gedev.bs_geom(),
        egrid: gedev.egrid(),
        ngrid: gendev.ngrid(),
        edev: &gedev,
        endev: &gendev,
        ebonds: &ebonds,
    };
    let mut hammer = Session::new(&backend);
    hammer.debug = opts.debug;
    hammer.max_threads = opts.max_threads;
    if opts.no_dup {
        hammer.dup_factor = 1;
    }
    if !opts.skip_core {
        int::add_fuzzers(&mut hammer, &backend);
    }
    // sigh. Spartan 3AN cannot do VCCAUX == 2.5 and this causes a *ridiculously annoying*
    // problem in the I/O fuzzers, which cannot easily identify IBUF_MODE == CMOS_VCCO
    // without that. it could be fixed, but it's easier to just rely on the fuzzers being
    // run for plain Spartan 3A. it's the same die anyway.
    if part.name.starts_with("xc3s") && part.name.ends_with('n') {
        opts.skip_io = true;
    }
    // ISE just segfaults on this device under complex microarchitectural conditions,
    // the exact nature of which is unknown, but I/O-related.
    // fuck this shit and just skip the whole thing, I'm here to reverse FPGAs not ISE bugs.
    if part.name == "xc3s2000" {
        opts.skip_io = true;
    }
    // Apparently ISE applies virtex2 programming to those instead of virtex2p programming.
    // Just skip.
    if part.name.starts_with("xq2vp") {
        opts.skip_dcm = true;
    }
    match gedev {
        ExpandedDevice::Xc2000(ref edev) => {
            if !opts.skip_core {
                if edev.grid.kind.is_xc4000() {
                    clb::xc4000::add_fuzzers(&mut hammer, &backend);
                    io::xc4000::add_fuzzers(&mut hammer, &backend);
                    misc::xc4000::add_fuzzers(&mut hammer, &backend);
                } else {
                    clb::xc5200::add_fuzzers(&mut hammer, &backend);
                    io::xc5200::add_fuzzers(&mut hammer, &backend);
                    misc::xc5200::add_fuzzers(&mut hammer, &backend);
                }
            }
        }
        ExpandedDevice::Virtex(_) => {
            if !opts.skip_core {
                clb::virtex::add_fuzzers(&mut hammer, &backend);
                tbus::add_fuzzers(&mut hammer, &backend);
                clk::virtex::add_fuzzers(&mut hammer, &backend);
                bram::virtex::add_fuzzers(&mut hammer, &backend);
                misc::virtex::add_fuzzers(&mut hammer, &backend);
                io::virtex::add_fuzzers(&mut hammer, &backend);
                dcm::virtex::add_fuzzers(&mut hammer, &backend);
            }
        }
        ExpandedDevice::Virtex2(ref edev) => {
            if !opts.skip_core {
                if edev.grid.kind.is_virtex2() {
                    tbus::add_fuzzers(&mut hammer, &backend);
                }
                clb::virtex2::add_fuzzers(&mut hammer, &backend);
                if edev.grid.kind != prjcombine_virtex2::grid::GridKind::FpgaCore {
                    bram::virtex2::add_fuzzers(&mut hammer, &backend, false);
                }
                if edev.grid.kind == prjcombine_virtex2::grid::GridKind::Spartan3ADsp {
                    dsp::spartan3adsp::add_fuzzers(&mut hammer, &backend);
                }
            } else if !edev.grid.kind.is_virtex2()
                && edev.grid.kind != prjcombine_virtex2::grid::GridKind::FpgaCore
            {
                bram::virtex2::add_fuzzers(&mut hammer, &backend, true);
            }
            if !opts.skip_clk {
                clk::virtex2::add_fuzzers(&mut hammer, &backend, false);
            } else if opts.devdata_only {
                clk::virtex2::add_fuzzers(&mut hammer, &backend, true);
            }
            if !opts.skip_misc {
                misc::virtex2::add_fuzzers(&mut hammer, &backend, opts.skip_io, false);
            } else if opts.devdata_only {
                misc::virtex2::add_fuzzers(&mut hammer, &backend, true, true);
            }
            if !opts.skip_io {
                if edev.grid.kind != prjcombine_virtex2::grid::GridKind::FpgaCore {
                    io::virtex2::add_fuzzers(&mut hammer, &backend);
                } else {
                    io::fpgacore::add_fuzzers(&mut hammer, &backend);
                }
            }
            if edev.grid.kind != prjcombine_virtex2::grid::GridKind::FpgaCore {
                if !opts.skip_dcm {
                    if !edev.grid.kind.is_spartan3ea() {
                        dcm::virtex2::add_fuzzers(&mut hammer, &backend, false);
                    } else {
                        dcm::spartan3e::add_fuzzers(&mut hammer, &backend, false);
                    }
                } else if opts.devdata_only {
                    if !edev.grid.kind.is_spartan3ea() {
                        dcm::virtex2::add_fuzzers(&mut hammer, &backend, true);
                    } else {
                        dcm::spartan3e::add_fuzzers(&mut hammer, &backend, true);
                    }
                }
            }
            if !opts.skip_hard && edev.grid.kind.is_virtex2p() {
                ppc::virtex2::add_fuzzers(&mut hammer, &backend);
            }
            if !opts.skip_gt {
                if edev.grid.kind == prjcombine_virtex2::grid::GridKind::Virtex2P {
                    gt::virtex2p::add_fuzzers(&mut hammer, &backend);
                }
                if edev.grid.kind == prjcombine_virtex2::grid::GridKind::Virtex2PX {
                    gt::virtex2px::add_fuzzers(&mut hammer, &backend);
                }
            }
        }
        ExpandedDevice::Spartan6(_) => {
            if !opts.skip_core {
                clb::virtex5::add_fuzzers(&mut hammer, &backend);
                bram::spartan6::add_fuzzers(&mut hammer, &backend);
                dsp::spartan3adsp::add_fuzzers(&mut hammer, &backend);
            }
            if !opts.skip_clk {
                clk::spartan6::add_fuzzers(&mut hammer, &backend, false);
            } else if opts.devdata_only {
                clk::spartan6::add_fuzzers(&mut hammer, &backend, true);
            }
            if !opts.skip_misc {
                misc::spartan6::add_fuzzers(&mut hammer, &backend);
            }
            if !opts.skip_io {
                io::spartan6::add_fuzzers(&mut hammer, &backend);
            }
            if !opts.skip_dcm {
                dcm::spartan6::add_fuzzers(&mut hammer, &backend);
            }
            if !opts.skip_pll {
                pll::spartan6::add_fuzzers(&mut hammer, &backend);
            }
            if !opts.skip_hard {
                mcb::add_fuzzers(&mut hammer, &backend);
                pcie::spartan6::add_fuzzers(&mut hammer, &backend);
            }
            if !opts.skip_gt {
                gt::spartan6::add_fuzzers(&mut hammer, &backend);
            }
        }
        ExpandedDevice::Virtex4(ref edev) => match edev.kind {
            prjcombine_virtex4::grid::GridKind::Virtex4 => {
                if !opts.skip_core {
                    clb::virtex2::add_fuzzers(&mut hammer, &backend);
                    bram::virtex4::add_fuzzers(&mut hammer, &backend);
                    dsp::virtex4::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_clk {
                    clk::virtex4::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_misc {
                    misc::virtex4::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_io {
                    io::virtex4::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_dcm {
                    dcm::virtex4::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_ccm {
                    ccm::virtex4::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_hard {
                    ppc::virtex4::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_gt {
                    gt::virtex4::add_fuzzers(&mut hammer, &backend);
                }
            }
            prjcombine_virtex4::grid::GridKind::Virtex5 => {
                if !opts.skip_core {
                    clb::virtex5::add_fuzzers(&mut hammer, &backend);
                    bram::virtex5::add_fuzzers(&mut hammer, &backend);
                    dsp::virtex5::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_clk {
                    clk::virtex5::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_misc {
                    misc::virtex5::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_io {
                    io::virtex5::add_fuzzers(&mut hammer, &backend, false);
                } else if opts.devdata_only {
                    io::virtex5::add_fuzzers(&mut hammer, &backend, true);
                }
                if !opts.skip_dcm || !opts.skip_pll {
                    cmt::virtex5::add_fuzzers(
                        &mut hammer,
                        &backend,
                        opts.skip_dcm,
                        opts.skip_pll,
                        false,
                    );
                } else if opts.devdata_only {
                    cmt::virtex5::add_fuzzers(&mut hammer, &backend, false, false, true);
                }
                if !opts.skip_hard {
                    ppc::virtex5::add_fuzzers(&mut hammer, &backend, false);
                    emac::virtex5::add_fuzzers(&mut hammer, &backend);
                    pcie::virtex5::add_fuzzers(&mut hammer, &backend);
                } else if opts.devdata_only {
                    ppc::virtex5::add_fuzzers(&mut hammer, &backend, true);
                }
                if !opts.skip_gt {
                    gt::virtex5::add_fuzzers(&mut hammer, &backend);
                }
            }
            prjcombine_virtex4::grid::GridKind::Virtex6 => {
                if !opts.skip_core {
                    clb::virtex5::add_fuzzers(&mut hammer, &backend);
                    bram::virtex6::add_fuzzers(&mut hammer, &backend);
                    dsp::virtex6::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_clk {
                    clk::virtex6::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_misc {
                    misc::virtex6::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_io {
                    io::virtex6::add_fuzzers(&mut hammer, &backend, false);
                } else if opts.devdata_only {
                    io::virtex6::add_fuzzers(&mut hammer, &backend, true);
                }
                if !opts.skip_pll {
                    cmt::virtex6::add_fuzzers(&mut hammer, &backend, false);
                } else if opts.devdata_only {
                    cmt::virtex6::add_fuzzers(&mut hammer, &backend, true);
                }
                if !opts.skip_hard {
                    emac::virtex6::add_fuzzers(&mut hammer, &backend);
                    pcie::virtex6::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_gt {
                    gt::virtex6_gtx::add_fuzzers(&mut hammer, &backend);
                    gt::virtex6_gth::add_fuzzers(&mut hammer, &backend);
                }
            }
            prjcombine_virtex4::grid::GridKind::Virtex7 => {
                if !opts.skip_core {
                    clb::virtex5::add_fuzzers(&mut hammer, &backend);
                    bram::virtex6::add_fuzzers(&mut hammer, &backend);
                    dsp::virtex6::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_clk {
                    clk::virtex7::add_fuzzers(&mut hammer, &backend, false);
                } else if opts.bali_only {
                    clk::virtex7::add_fuzzers(&mut hammer, &backend, true);
                }
                if !opts.skip_misc {
                    misc::virtex7::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_io {
                    io::virtex7::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_pll {
                    cmt::virtex7::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_hard {
                    pcie::virtex7::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_gt {
                    gt::virtex7::add_fuzzers(&mut hammer, &backend);
                }
            }
        },
        _ => panic!("unsupported device kind"),
    }
    if !opts.skip_core {
        intf::add_fuzzers(&mut hammer, &backend);
    }
    let (empty_bs, mut state) = std::thread::scope(|s| {
        let empty_bs_t = s.spawn(|| backend.bitgen(&HashMap::new()));
        let state = hammer.run().unwrap();
        let empty_bs = empty_bs_t.join().unwrap();
        (empty_bs, state)
    });
    let mut ctx = CollectorCtx {
        device: part,
        edev: &gedev,
        db,
        collector: Collector {
            state: &mut state,
            tiledb,
        },
        empty_bs: &empty_bs,
    };
    if !opts.skip_core {
        int::collect_fuzzers(&mut ctx);
    }
    match gedev {
        ExpandedDevice::Xc2000(ref edev) => {
            if !opts.skip_core {
                if edev.grid.kind.is_xc4000() {
                    clb::xc4000::collect_fuzzers(&mut ctx);
                    io::xc4000::collect_fuzzers(&mut ctx);
                    misc::xc4000::collect_fuzzers(&mut ctx);
                } else {
                    clb::xc5200::collect_fuzzers(&mut ctx);
                    io::xc5200::collect_fuzzers(&mut ctx);
                    misc::xc5200::collect_fuzzers(&mut ctx);
                }
            }
        }
        ExpandedDevice::Virtex(_) => {
            if !opts.skip_core {
                clb::virtex::collect_fuzzers(&mut ctx);
                tbus::collect_fuzzers(&mut ctx);
                clk::virtex::collect_fuzzers(&mut ctx);
                bram::virtex::collect_fuzzers(&mut ctx);
                misc::virtex::collect_fuzzers(&mut ctx);
                io::virtex::collect_fuzzers(&mut ctx);
                dcm::virtex::collect_fuzzers(&mut ctx);
            }
        }
        ExpandedDevice::Virtex2(ref edev) => {
            if !opts.skip_core {
                if edev.grid.kind.is_virtex2() {
                    tbus::collect_fuzzers(&mut ctx);
                }
                clb::virtex2::collect_fuzzers(&mut ctx);
                if edev.grid.kind != prjcombine_virtex2::grid::GridKind::FpgaCore {
                    bram::virtex2::collect_fuzzers(&mut ctx, false);
                }
                if edev.grid.kind == prjcombine_virtex2::grid::GridKind::Spartan3ADsp {
                    dsp::spartan3adsp::collect_fuzzers(&mut ctx);
                }
            } else if !edev.grid.kind.is_virtex2()
                && edev.grid.kind != prjcombine_virtex2::grid::GridKind::FpgaCore
            {
                bram::virtex2::collect_fuzzers(&mut ctx, true);
            }
            if !opts.skip_clk {
                clk::virtex2::collect_fuzzers(&mut ctx, false);
            } else if opts.devdata_only {
                clk::virtex2::collect_fuzzers(&mut ctx, true);
            }
            if !opts.skip_misc {
                misc::virtex2::collect_fuzzers(&mut ctx, opts.skip_io, false);
            } else if opts.devdata_only {
                misc::virtex2::collect_fuzzers(&mut ctx, true, true);
            }
            if !opts.skip_io {
                if edev.grid.kind != prjcombine_virtex2::grid::GridKind::FpgaCore {
                    io::virtex2::collect_fuzzers(&mut ctx);
                } else {
                    io::fpgacore::collect_fuzzers(&mut ctx);
                }
            }
            if edev.grid.kind != prjcombine_virtex2::grid::GridKind::FpgaCore {
                if !opts.skip_dcm {
                    if !edev.grid.kind.is_spartan3ea() {
                        dcm::virtex2::collect_fuzzers(&mut ctx, false);
                    } else {
                        dcm::spartan3e::collect_fuzzers(&mut ctx, false);
                    }
                } else if opts.devdata_only {
                    if !edev.grid.kind.is_spartan3ea() {
                        dcm::virtex2::collect_fuzzers(&mut ctx, true);
                    } else {
                        dcm::spartan3e::collect_fuzzers(&mut ctx, true);
                    }
                }
            }
            if !opts.skip_hard && edev.grid.kind.is_virtex2p() {
                ppc::virtex2::collect_fuzzers(&mut ctx);
            }
            if !opts.skip_gt {
                if edev.grid.kind == prjcombine_virtex2::grid::GridKind::Virtex2P {
                    gt::virtex2p::collect_fuzzers(&mut ctx);
                }
                if edev.grid.kind == prjcombine_virtex2::grid::GridKind::Virtex2PX {
                    gt::virtex2px::collect_fuzzers(&mut ctx);
                }
            }
        }
        ExpandedDevice::Spartan6(_) => {
            if !opts.skip_core {
                clb::virtex5::collect_fuzzers(&mut ctx);
                bram::spartan6::collect_fuzzers(&mut ctx);
                dsp::spartan3adsp::collect_fuzzers(&mut ctx);
            }
            if !opts.skip_clk {
                clk::spartan6::collect_fuzzers(&mut ctx, false);
            } else if opts.devdata_only {
                clk::spartan6::collect_fuzzers(&mut ctx, true);
            }
            if !opts.skip_misc {
                misc::spartan6::collect_fuzzers(&mut ctx);
            }
            if !opts.skip_io {
                io::spartan6::collect_fuzzers(&mut ctx);
            }
            if !opts.skip_dcm {
                dcm::spartan6::collect_fuzzers(&mut ctx);
            }
            if !opts.skip_pll {
                pll::spartan6::collect_fuzzers(&mut ctx, opts.skip_dcm);
            }
            if !opts.skip_hard {
                mcb::collect_fuzzers(&mut ctx);
                pcie::spartan6::collect_fuzzers(&mut ctx);
            }
            if !opts.skip_gt {
                gt::spartan6::collect_fuzzers(&mut ctx);
            }
        }
        ExpandedDevice::Virtex4(ref edev) => match edev.kind {
            prjcombine_virtex4::grid::GridKind::Virtex4 => {
                if !opts.skip_core {
                    clb::virtex2::collect_fuzzers(&mut ctx);
                    bram::virtex4::collect_fuzzers(&mut ctx);
                    dsp::virtex4::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_clk {
                    clk::virtex4::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_misc {
                    misc::virtex4::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_io {
                    io::virtex4::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_dcm {
                    dcm::virtex4::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_ccm {
                    ccm::virtex4::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_hard {
                    ppc::virtex4::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_gt {
                    gt::virtex4::collect_fuzzers(&mut ctx);
                }
            }
            prjcombine_virtex4::grid::GridKind::Virtex5 => {
                if !opts.skip_core {
                    clb::virtex5::collect_fuzzers(&mut ctx);
                    bram::virtex5::collect_fuzzers(&mut ctx);
                    dsp::virtex5::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_clk {
                    clk::virtex5::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_misc {
                    misc::virtex5::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_io {
                    io::virtex5::collect_fuzzers(&mut ctx, false);
                } else if opts.devdata_only {
                    io::virtex5::collect_fuzzers(&mut ctx, true);
                }
                if !opts.skip_dcm || !opts.skip_pll {
                    cmt::virtex5::collect_fuzzers(&mut ctx, opts.skip_dcm, opts.skip_pll, false);
                } else if opts.devdata_only {
                    cmt::virtex5::collect_fuzzers(&mut ctx, true, true, true);
                }
                if !opts.skip_hard {
                    ppc::virtex5::collect_fuzzers(&mut ctx, false);
                    emac::virtex5::collect_fuzzers(&mut ctx);
                    pcie::virtex5::collect_fuzzers(&mut ctx);
                } else if opts.devdata_only {
                    ppc::virtex5::collect_fuzzers(&mut ctx, true);
                }
                if !opts.skip_gt {
                    gt::virtex5::collect_fuzzers(&mut ctx);
                }
            }
            prjcombine_virtex4::grid::GridKind::Virtex6 => {
                if !opts.skip_core {
                    clb::virtex5::collect_fuzzers(&mut ctx);
                    bram::virtex6::collect_fuzzers(&mut ctx);
                    dsp::virtex6::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_clk {
                    clk::virtex6::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_misc {
                    misc::virtex6::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_io {
                    io::virtex6::collect_fuzzers(&mut ctx, false);
                } else if opts.devdata_only {
                    io::virtex6::collect_fuzzers(&mut ctx, true);
                }
                if !opts.skip_pll {
                    cmt::virtex6::collect_fuzzers(&mut ctx, false);
                } else if opts.devdata_only {
                    cmt::virtex6::collect_fuzzers(&mut ctx, true);
                }
                if !opts.skip_hard {
                    emac::virtex6::collect_fuzzers(&mut ctx);
                    pcie::virtex6::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_gt {
                    gt::virtex6_gtx::collect_fuzzers(&mut ctx);
                    gt::virtex6_gth::collect_fuzzers(&mut ctx);
                }
            }
            prjcombine_virtex4::grid::GridKind::Virtex7 => {
                if !opts.skip_core {
                    clb::virtex5::collect_fuzzers(&mut ctx);
                    bram::virtex6::collect_fuzzers(&mut ctx);
                    dsp::virtex6::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_clk {
                    clk::virtex7::collect_fuzzers(&mut ctx, false);
                } else if opts.bali_only {
                    clk::virtex7::collect_fuzzers(&mut ctx, true);
                }
                if !opts.skip_misc {
                    misc::virtex7::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_io {
                    io::virtex7::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_pll {
                    cmt::virtex7::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_hard {
                    pcie::virtex7::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_gt {
                    gt::virtex7::collect_fuzzers(&mut ctx);
                }
            }
        },
        _ => panic!("unsupported device kind"),
    }
    if !opts.skip_core {
        intf::collect_fuzzers(&mut ctx);
    }
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

    for (feat, data) in ctx.state.features.iter().sorted_by_key(|&(k, _)| k) {
        println!(
            "{} {} {} {}: {:?}",
            feat.tile, feat.bel, feat.attr, feat.val, data.diffs
        );
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let tc = Toolchain::from_file(&args.toolchain)?;
    let db = GeomDb::from_file(args.geomdb)?;
    let mut tiledb = TileDb::new();
    let opts = RunOpts {
        skip_io: args.skip_io,
        skip_clk: args.skip_clk,
        skip_ccm: args.skip_ccm,
        skip_dcm: args.skip_dcm,
        skip_pll: args.skip_pll,
        skip_misc: args.skip_misc,
        skip_gt: args.skip_gt,
        skip_hard: args.skip_hard,
        skip_core: args.skip_core,
        bali_only: args.bali_only,
        devdata_only: false,
        no_dup: args.no_dup,
        debug: args.debug,
        max_threads: args.max_threads,
    };
    let parts_dict: HashMap<_, _> = db
        .devices
        .iter()
        .map(|part| (&part.name[..], part))
        .collect();
    if args.parts.is_empty() {
        match db.grids.first().unwrap() {
            prjcombine_xilinx_geom::Grid::Virtex(_) => {
                run(&tc, &db, parts_dict[&"xcv400"], &mut tiledb, &opts);
                run(&tc, &db, parts_dict[&"xc2s50e"], &mut tiledb, &opts);
                run(&tc, &db, parts_dict[&"xc2s50"], &mut tiledb, &opts);
                run(&tc, &db, parts_dict[&"xcv405e"], &mut tiledb, &opts);
                if !args.skip_devdata {
                    let mut xopts = opts;
                    xopts.skip_all();
                    xopts.devdata_only = true;
                    for part in &db.devices {
                        run(&tc, &db, part, &mut tiledb, &xopts);
                    }
                }
            }
            prjcombine_xilinx_geom::Grid::Virtex2(grid) => match grid.kind {
                prjcombine_virtex2::grid::GridKind::Virtex2
                | prjcombine_virtex2::grid::GridKind::Virtex2P
                | prjcombine_virtex2::grid::GridKind::Virtex2PX => {
                    run(&tc, &db, parts_dict[&"xc2v40"], &mut tiledb, &opts);
                    run(&tc, &db, parts_dict[&"xc2vp4"], &mut tiledb, &opts);
                    run(&tc, &db, parts_dict[&"xc2vpx20"], &mut tiledb, &opts);
                    if !args.skip_io {
                        let mut xopts = opts;
                        xopts.skip_all();
                        xopts.skip_io = opts.skip_io;
                        run(&tc, &db, parts_dict[&"xc2v250"], &mut tiledb, &xopts);
                    }
                    if !args.skip_devdata {
                        let mut xopts = opts;
                        xopts.skip_all();
                        xopts.devdata_only = true;
                        for part in &db.devices {
                            run(&tc, &db, part, &mut tiledb, &xopts);
                        }
                    }
                }
                prjcombine_virtex2::grid::GridKind::Spartan3
                | prjcombine_virtex2::grid::GridKind::Spartan3E
                | prjcombine_virtex2::grid::GridKind::Spartan3A
                | prjcombine_virtex2::grid::GridKind::Spartan3ADsp => {
                    run(&tc, &db, parts_dict[&"xc3s200"], &mut tiledb, &opts);
                    run(&tc, &db, parts_dict[&"xc3s100e"], &mut tiledb, &opts);
                    run(&tc, &db, parts_dict[&"xc3s250e"], &mut tiledb, &opts);
                    run(&tc, &db, parts_dict[&"xc3s500e"], &mut tiledb, &opts);
                    run(&tc, &db, parts_dict[&"xc3s1200e"], &mut tiledb, &opts);
                    run(&tc, &db, parts_dict[&"xc3s1600e"], &mut tiledb, &opts);
                    run(&tc, &db, parts_dict[&"xc3s50a"], &mut tiledb, &opts);
                    run(&tc, &db, parts_dict[&"xc3s700a"], &mut tiledb, &opts);
                    run(&tc, &db, parts_dict[&"xc3sd1800a"], &mut tiledb, &opts);
                    if !args.skip_core || !args.skip_io {
                        // dummy DCM int; more VREF
                        let mut xopts = opts;
                        xopts.skip_all();
                        xopts.skip_core = opts.skip_core;
                        xopts.skip_io = opts.skip_io;
                        run(&tc, &db, parts_dict[&"xc3s4000"], &mut tiledb, &xopts);
                    }
                    if !args.skip_io {
                        // more VREF
                        let mut xopts = opts;
                        xopts.skip_all();
                        xopts.skip_io = opts.skip_io;
                        run(&tc, &db, parts_dict[&"xc3s1000"], &mut tiledb, &xopts);
                    }
                    if !args.skip_devdata {
                        let mut xopts = opts;
                        xopts.skip_all();
                        xopts.devdata_only = true;
                        for part in &db.devices {
                            run(&tc, &db, part, &mut tiledb, &xopts);
                        }
                    }
                }
                prjcombine_virtex2::grid::GridKind::FpgaCore => {
                    run(&tc, &db, parts_dict[&"xcexf10"], &mut tiledb, &opts);
                    if !args.skip_devdata {
                        let mut xopts = opts;
                        xopts.skip_all();
                        xopts.devdata_only = true;
                        for part in &db.devices {
                            run(&tc, &db, part, &mut tiledb, &xopts);
                        }
                    }
                }
            },
            prjcombine_xilinx_geom::Grid::Spartan6(_) => {
                run(&tc, &db, parts_dict[&"xc6slx75t"], &mut tiledb, &opts);
                if !args.skip_devdata {
                    let mut xopts = opts;
                    xopts.skip_all();
                    xopts.devdata_only = true;
                    for part in &db.devices {
                        run(&tc, &db, part, &mut tiledb, &xopts);
                    }
                }
            }
            prjcombine_xilinx_geom::Grid::Virtex4(grid) => match grid.kind {
                prjcombine_virtex4::grid::GridKind::Virtex4 => {
                    run(&tc, &db, parts_dict[&"xc4vfx60"], &mut tiledb, &opts);
                    if !opts.skip_io {
                        let mut xopts = opts;
                        xopts.skip_all();
                        xopts.skip_io = opts.skip_io;
                        run(&tc, &db, parts_dict[&"xc4vlx60"], &mut tiledb, &xopts);
                        run(&tc, &db, parts_dict[&"xc4vlx100"], &mut tiledb, &xopts);
                    }
                    if !args.skip_devdata {
                        let mut xopts = opts;
                        xopts.skip_all();
                        xopts.devdata_only = true;
                        for part in &db.devices {
                            run(&tc, &db, part, &mut tiledb, &xopts);
                        }
                    }
                }
                prjcombine_virtex4::grid::GridKind::Virtex5 => {
                    run(&tc, &db, parts_dict[&"xc5vtx150t"], &mut tiledb, &opts);
                    if !opts.skip_gt {
                        let mut xopts = opts;
                        xopts.skip_all();
                        xopts.skip_gt = opts.skip_gt;
                        run(&tc, &db, parts_dict[&"xc5vlx30t"], &mut tiledb, &xopts);
                    }
                    if !opts.skip_hard {
                        let mut xopts = opts;
                        xopts.skip_all();
                        xopts.skip_hard = opts.skip_hard;
                        run(&tc, &db, parts_dict[&"xc5vfx70t"], &mut tiledb, &xopts);
                    }
                    if !args.skip_devdata {
                        let mut xopts = opts;
                        xopts.skip_all();
                        xopts.devdata_only = true;
                        for part in &db.devices {
                            run(&tc, &db, part, &mut tiledb, &xopts);
                        }
                    }
                }
                prjcombine_virtex4::grid::GridKind::Virtex6 => {
                    run(&tc, &db, parts_dict[&"xc6vlx75t"], &mut tiledb, &opts);
                    if !opts.skip_gt {
                        let mut xopts = opts;
                        xopts.skip_all();
                        xopts.skip_gt = opts.skip_gt;
                        run(&tc, &db, parts_dict[&"xc6vhx255t"], &mut tiledb, &xopts);
                    }
                    if !args.skip_devdata {
                        let mut xopts = opts;
                        xopts.skip_all();
                        xopts.devdata_only = true;
                        for part in &db.devices {
                            run(&tc, &db, part, &mut tiledb, &xopts);
                        }
                    }
                }
                prjcombine_virtex4::grid::GridKind::Virtex7 => {
                    run(&tc, &db, parts_dict[&"xc7k70t"], &mut tiledb, &opts);
                    if !opts.skip_gt {
                        // GTP
                        let mut xopts = opts;
                        xopts.skip_all();
                        xopts.skip_gt = opts.skip_gt;
                        run(&tc, &db, parts_dict[&"xc7a50t"], &mut tiledb, &xopts);
                        run(&tc, &db, parts_dict[&"xc7a200t"], &mut tiledb, &xopts);
                    }
                    if !opts.skip_clk || !opts.skip_hard {
                        // left PCIE
                        let mut xopts = opts;
                        xopts.skip_all();
                        xopts.bali_only = !opts.skip_clk;
                        xopts.skip_hard = opts.skip_hard;
                        xopts.skip_gt = opts.skip_gt;
                        xopts.max_threads = Some(12);
                        run(&tc, &db, parts_dict[&"xc7vx485t"], &mut tiledb, &xopts);
                    }
                    if !opts.skip_clk || !opts.skip_hard || !opts.skip_gt {
                        // GTH, CLK_BALI_REBUF, PCIE3
                        let mut xopts = opts;
                        xopts.skip_all();
                        xopts.bali_only = !opts.skip_clk;
                        xopts.skip_hard = opts.skip_hard;
                        xopts.skip_gt = opts.skip_gt;
                        xopts.max_threads = Some(6);
                        run(&tc, &db, parts_dict[&"xc7vx1140t"], &mut tiledb, &xopts);
                    }
                    if !args.skip_devdata {
                        let mut xopts = opts;
                        xopts.skip_all();
                        xopts.devdata_only = true;
                        for part in &db.devices {
                            if part.name.contains("7vh") {
                                // no. not yet.
                                continue;
                            }
                            run(&tc, &db, part, &mut tiledb, &xopts);
                        }
                    }
                }
            },
            _ => {
                for part in &db.devices {
                    run(&tc, &db, part, &mut tiledb, &opts);
                }
            }
        }
    } else {
        for pname in args.parts {
            let part = parts_dict[&&pname[..]];
            run(&tc, &db, part, &mut tiledb, &opts);
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
    tiledb.to_file(&args.tiledb)?;
    Ok(())
}
