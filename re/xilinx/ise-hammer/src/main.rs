#![recursion_limit = "1024"]

use clap::Parser;
use prjcombine_interconnect::dir::DirV;
use prjcombine_re_fpga_hammer::Collector;
use prjcombine_re_hammer::{Backend, Session};
use prjcombine_re_toolchain::Toolchain;
use prjcombine_re_xilinx_geom::{Device, ExpandedDevice, GeomDb};
use prjcombine_types::bitvec::BitVec;
use prjcombine_types::bsdata::BsData;
use prjcombine_xilinx_bitstream::Reg;
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;

mod backend;
mod collector;
mod generic;
mod spartan6;
mod virtex;
mod virtex2;
mod virtex4;
mod virtex5;
mod virtex6;
mod virtex7;
mod xc4000;
mod xc5200;

use backend::IseBackend;

use crate::collector::CollectorCtx;

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
    skip_gtz: bool,
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
    skip_gtz: bool,
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
        self.skip_gtz = true;
        self.skip_hard = true;
    }
}

fn run(tc: &Toolchain, db: &GeomDb, part: &Device, tiledb: &mut BsData, opts: &RunOpts) {
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
        tc,
        db,
        device: part,
        bs_geom: gedev.bs_geom(),
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
                if edev.chip.kind.is_xc4000() {
                    xc4000::int::add_fuzzers(&mut hammer, &backend);
                    xc4000::clb::add_fuzzers(&mut hammer, &backend);
                    xc4000::io::add_fuzzers(&mut hammer, &backend);
                    xc4000::misc::add_fuzzers(&mut hammer, &backend);
                } else {
                    xc5200::int::add_fuzzers(&mut hammer, &backend);
                    xc5200::clb::add_fuzzers(&mut hammer, &backend);
                    xc5200::io::add_fuzzers(&mut hammer, &backend);
                    xc5200::misc::add_fuzzers(&mut hammer, &backend);
                }
            }
        }
        ExpandedDevice::Virtex(_) => {
            if !opts.skip_core {
                virtex::int::add_fuzzers(&mut hammer, &backend);
                virtex::clb::add_fuzzers(&mut hammer, &backend);
                virtex::tbus::add_fuzzers(&mut hammer, &backend);
                virtex::clk::add_fuzzers(&mut hammer, &backend);
                virtex::bram::add_fuzzers(&mut hammer, &backend);
                virtex::misc::add_fuzzers(&mut hammer, &backend);
                virtex::io::add_fuzzers(&mut hammer, &backend);
                virtex::dll::add_fuzzers(&mut hammer, &backend);
            }
        }
        ExpandedDevice::Virtex2(ref edev) => {
            if !opts.skip_core {
                generic::int::add_fuzzers(&mut hammer, &backend);
                if edev.chip.kind.is_virtex2() {
                    virtex::tbus::add_fuzzers(&mut hammer, &backend);
                }
                virtex2::clb::add_fuzzers(&mut hammer, &backend);
                if edev.chip.kind != prjcombine_virtex2::chip::ChipKind::FpgaCore {
                    virtex2::bram::add_fuzzers(&mut hammer, &backend, false);
                }
                if edev.chip.kind == prjcombine_virtex2::chip::ChipKind::Spartan3ADsp {
                    virtex2::dsp::add_fuzzers(&mut hammer, &backend);
                }
            } else if !edev.chip.kind.is_virtex2()
                && edev.chip.kind != prjcombine_virtex2::chip::ChipKind::FpgaCore
            {
                virtex2::bram::add_fuzzers(&mut hammer, &backend, true);
            }
            if !opts.skip_clk {
                virtex2::clk::add_fuzzers(&mut hammer, &backend, false);
            } else if opts.devdata_only {
                virtex2::clk::add_fuzzers(&mut hammer, &backend, true);
            }
            if !opts.skip_misc {
                virtex2::misc::add_fuzzers(&mut hammer, &backend, opts.skip_io, false);
            } else if opts.devdata_only {
                virtex2::misc::add_fuzzers(&mut hammer, &backend, true, true);
            }
            if !opts.skip_io {
                if edev.chip.kind != prjcombine_virtex2::chip::ChipKind::FpgaCore {
                    virtex2::io::add_fuzzers(&mut hammer, &backend);
                } else {
                    virtex2::io_fpgacore::add_fuzzers(&mut hammer, &backend);
                }
            }
            if edev.chip.kind != prjcombine_virtex2::chip::ChipKind::FpgaCore {
                if !opts.skip_dcm {
                    if !edev.chip.kind.is_spartan3ea() {
                        virtex2::dcm_v2::add_fuzzers(&mut hammer, &backend, false);
                    } else {
                        virtex2::dcm_s3e::add_fuzzers(&mut hammer, &backend, false);
                    }
                } else if opts.devdata_only {
                    if !edev.chip.kind.is_spartan3ea() {
                        virtex2::dcm_v2::add_fuzzers(&mut hammer, &backend, true);
                    } else {
                        virtex2::dcm_s3e::add_fuzzers(&mut hammer, &backend, true);
                    }
                }
            }
            if !opts.skip_hard && edev.chip.kind.is_virtex2p() {
                virtex2::ppc::add_fuzzers(&mut hammer, &backend);
            }
            if !opts.skip_gt {
                if edev.chip.kind == prjcombine_virtex2::chip::ChipKind::Virtex2P {
                    virtex2::gt::add_fuzzers(&mut hammer, &backend);
                }
                if edev.chip.kind == prjcombine_virtex2::chip::ChipKind::Virtex2PX {
                    virtex2::gt10::add_fuzzers(&mut hammer, &backend);
                }
            }
        }
        ExpandedDevice::Spartan6(_) => {
            if !opts.skip_core {
                generic::int::add_fuzzers(&mut hammer, &backend);
                virtex5::clb::add_fuzzers(&mut hammer, &backend);
                spartan6::bram::add_fuzzers(&mut hammer, &backend);
                virtex2::dsp::add_fuzzers(&mut hammer, &backend);
            }
            if !opts.skip_clk {
                spartan6::clk::add_fuzzers(&mut hammer, &backend, false);
            } else if opts.devdata_only {
                spartan6::clk::add_fuzzers(&mut hammer, &backend, true);
            }
            if !opts.skip_misc {
                spartan6::misc::add_fuzzers(&mut hammer, &backend);
            }
            if !opts.skip_io {
                spartan6::io::add_fuzzers(&mut hammer, &backend);
            }
            if !opts.skip_dcm {
                spartan6::dcm::add_fuzzers(&mut hammer, &backend);
            }
            if !opts.skip_pll {
                spartan6::pll::add_fuzzers(&mut hammer, &backend);
            }
            if !opts.skip_hard {
                spartan6::mcb::add_fuzzers(&mut hammer, &backend);
                spartan6::pcie::add_fuzzers(&mut hammer, &backend);
            }
            if !opts.skip_gt {
                spartan6::gt::add_fuzzers(&mut hammer, &backend);
            }
        }
        ExpandedDevice::Virtex4(ref edev) => match edev.kind {
            prjcombine_virtex4::chip::ChipKind::Virtex4 => {
                if !opts.skip_core {
                    generic::int::add_fuzzers(&mut hammer, &backend);
                    virtex2::clb::add_fuzzers(&mut hammer, &backend);
                    virtex4::bram::add_fuzzers(&mut hammer, &backend);
                    virtex4::dsp::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_clk {
                    virtex4::clk::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_misc {
                    virtex4::misc::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_io {
                    virtex4::io::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_dcm {
                    virtex4::dcm::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_ccm {
                    virtex4::ccm::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_hard {
                    virtex4::ppc::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_gt {
                    virtex4::gt::add_fuzzers(&mut hammer, &backend);
                }
            }
            prjcombine_virtex4::chip::ChipKind::Virtex5 => {
                if !opts.skip_core {
                    generic::int::add_fuzzers(&mut hammer, &backend);
                    virtex5::clb::add_fuzzers(&mut hammer, &backend);
                    virtex5::bram::add_fuzzers(&mut hammer, &backend);
                    virtex5::dsp::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_clk {
                    virtex5::clk::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_misc {
                    virtex5::misc::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_io {
                    virtex5::io::add_fuzzers(&mut hammer, &backend, false);
                } else if opts.devdata_only {
                    virtex5::io::add_fuzzers(&mut hammer, &backend, true);
                }
                if !opts.skip_dcm || !opts.skip_pll {
                    virtex5::cmt::add_fuzzers(
                        &mut hammer,
                        &backend,
                        opts.skip_dcm,
                        opts.skip_pll,
                        false,
                    );
                } else if opts.devdata_only {
                    virtex5::cmt::add_fuzzers(&mut hammer, &backend, false, false, true);
                }
                if !opts.skip_hard {
                    virtex5::ppc::add_fuzzers(&mut hammer, &backend, false);
                    virtex5::emac::add_fuzzers(&mut hammer, &backend);
                    virtex5::pcie::add_fuzzers(&mut hammer, &backend);
                } else if opts.devdata_only {
                    virtex5::ppc::add_fuzzers(&mut hammer, &backend, true);
                }
                if !opts.skip_gt {
                    virtex5::gt::add_fuzzers(&mut hammer, &backend);
                }
            }
            prjcombine_virtex4::chip::ChipKind::Virtex6 => {
                if !opts.skip_core {
                    generic::int::add_fuzzers(&mut hammer, &backend);
                    virtex5::clb::add_fuzzers(&mut hammer, &backend);
                    virtex6::bram::add_fuzzers(&mut hammer, &backend);
                    virtex6::dsp::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_clk {
                    virtex6::clk::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_misc {
                    virtex6::misc::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_io {
                    virtex6::io::add_fuzzers(&mut hammer, &backend, false);
                } else if opts.devdata_only {
                    virtex6::io::add_fuzzers(&mut hammer, &backend, true);
                }
                if !opts.skip_pll {
                    virtex6::cmt::add_fuzzers(&mut hammer, &backend, false);
                } else if opts.devdata_only {
                    virtex6::cmt::add_fuzzers(&mut hammer, &backend, true);
                }
                if !opts.skip_hard {
                    virtex6::emac::add_fuzzers(&mut hammer, &backend);
                    virtex6::pcie::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_gt {
                    virtex6::gtx::add_fuzzers(&mut hammer, &backend);
                    virtex6::gth::add_fuzzers(&mut hammer, &backend);
                }
            }
            prjcombine_virtex4::chip::ChipKind::Virtex7 => {
                if !opts.skip_core {
                    generic::int::add_fuzzers(&mut hammer, &backend);
                    virtex5::clb::add_fuzzers(&mut hammer, &backend);
                    virtex6::bram::add_fuzzers(&mut hammer, &backend);
                    virtex6::dsp::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_clk {
                    virtex7::clk::add_fuzzers(&mut hammer, &backend, false);
                } else if opts.bali_only {
                    virtex7::clk::add_fuzzers(&mut hammer, &backend, true);
                }
                if !opts.skip_misc {
                    virtex7::misc::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_io {
                    virtex7::io::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_pll {
                    virtex7::cmt::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_hard {
                    virtex7::pcie::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_gt {
                    virtex7::gt::add_fuzzers(&mut hammer, &backend);
                }
                if !opts.skip_gtz {
                    virtex7::gtz::add_fuzzers(&mut hammer, &backend);
                }
            }
        },
        _ => panic!("unsupported device kind"),
    }
    if !opts.skip_core {
        generic::intf::add_fuzzers(&mut hammer, &backend);
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
        collector: Collector::new(&mut state, tiledb, gedev.db),
        empty_bs: &empty_bs,
    };
    match gedev {
        ExpandedDevice::Xc2000(ref edev) => {
            if !opts.skip_core {
                if edev.chip.kind.is_xc4000() {
                    xc4000::int::collect_fuzzers(&mut ctx);
                    xc4000::clb::collect_fuzzers(&mut ctx);
                    xc4000::io::collect_fuzzers(&mut ctx);
                    xc4000::misc::collect_fuzzers(&mut ctx);
                } else {
                    xc5200::int::collect_fuzzers(&mut ctx);
                    xc5200::clb::collect_fuzzers(&mut ctx);
                    xc5200::io::collect_fuzzers(&mut ctx);
                    xc5200::misc::collect_fuzzers(&mut ctx);
                }
            }
        }
        ExpandedDevice::Virtex(_) => {
            if !opts.skip_core {
                virtex::int::collect_fuzzers(&mut ctx);
                virtex::clb::collect_fuzzers(&mut ctx);
                virtex::tbus::collect_fuzzers(&mut ctx);
                virtex::clk::collect_fuzzers(&mut ctx);
                virtex::bram::collect_fuzzers(&mut ctx);
                virtex::misc::collect_fuzzers(&mut ctx);
                virtex::io::collect_fuzzers(&mut ctx);
                virtex::dll::collect_fuzzers(&mut ctx);
            }
        }
        ExpandedDevice::Virtex2(ref edev) => {
            if !opts.skip_core {
                generic::int::collect_fuzzers(&mut ctx);
                if edev.chip.kind.is_virtex2() {
                    virtex::tbus::collect_fuzzers(&mut ctx);
                }
                virtex2::clb::collect_fuzzers(&mut ctx);
                if edev.chip.kind != prjcombine_virtex2::chip::ChipKind::FpgaCore {
                    virtex2::bram::collect_fuzzers(&mut ctx, false);
                }
                if edev.chip.kind == prjcombine_virtex2::chip::ChipKind::Spartan3ADsp {
                    virtex2::dsp::collect_fuzzers(&mut ctx);
                }
            } else if !edev.chip.kind.is_virtex2()
                && edev.chip.kind != prjcombine_virtex2::chip::ChipKind::FpgaCore
            {
                virtex2::bram::collect_fuzzers(&mut ctx, true);
            }
            if !opts.skip_clk {
                virtex2::clk::collect_fuzzers(&mut ctx, false);
            } else if opts.devdata_only {
                virtex2::clk::collect_fuzzers(&mut ctx, true);
            }
            if !opts.skip_misc {
                virtex2::misc::collect_fuzzers(&mut ctx, opts.skip_io, false);
            } else if opts.devdata_only {
                virtex2::misc::collect_fuzzers(&mut ctx, true, true);
            }
            if !opts.skip_io {
                if edev.chip.kind != prjcombine_virtex2::chip::ChipKind::FpgaCore {
                    virtex2::io::collect_fuzzers(&mut ctx);
                } else {
                    virtex2::io_fpgacore::collect_fuzzers(&mut ctx);
                }
            }
            if edev.chip.kind != prjcombine_virtex2::chip::ChipKind::FpgaCore {
                if !opts.skip_dcm {
                    if !edev.chip.kind.is_spartan3ea() {
                        virtex2::dcm_v2::collect_fuzzers(&mut ctx, false);
                    } else {
                        virtex2::dcm_s3e::collect_fuzzers(&mut ctx, false);
                    }
                } else if opts.devdata_only {
                    if !edev.chip.kind.is_spartan3ea() {
                        virtex2::dcm_v2::collect_fuzzers(&mut ctx, true);
                    } else {
                        virtex2::dcm_s3e::collect_fuzzers(&mut ctx, true);
                    }
                }
            }
            if !opts.skip_hard && edev.chip.kind.is_virtex2p() {
                virtex2::ppc::collect_fuzzers(&mut ctx);
            }
            if !opts.skip_gt {
                if edev.chip.kind == prjcombine_virtex2::chip::ChipKind::Virtex2P {
                    virtex2::gt::collect_fuzzers(&mut ctx);
                }
                if edev.chip.kind == prjcombine_virtex2::chip::ChipKind::Virtex2PX {
                    virtex2::gt10::collect_fuzzers(&mut ctx);
                }
            }
        }
        ExpandedDevice::Spartan6(_) => {
            if !opts.skip_core {
                generic::int::collect_fuzzers(&mut ctx);
                virtex5::clb::collect_fuzzers(&mut ctx);
                spartan6::bram::collect_fuzzers(&mut ctx);
                virtex2::dsp::collect_fuzzers(&mut ctx);
            }
            if !opts.skip_clk {
                spartan6::clk::collect_fuzzers(&mut ctx, false);
            } else if opts.devdata_only {
                spartan6::clk::collect_fuzzers(&mut ctx, true);
            }
            if !opts.skip_misc {
                spartan6::misc::collect_fuzzers(&mut ctx);
            }
            if !opts.skip_io {
                spartan6::io::collect_fuzzers(&mut ctx);
            }
            if !opts.skip_dcm {
                spartan6::dcm::collect_fuzzers(&mut ctx);
            }
            if !opts.skip_pll {
                spartan6::pll::collect_fuzzers(&mut ctx, opts.skip_dcm);
            }
            if !opts.skip_hard {
                spartan6::mcb::collect_fuzzers(&mut ctx);
                spartan6::pcie::collect_fuzzers(&mut ctx);
            }
            if !opts.skip_gt {
                spartan6::gt::collect_fuzzers(&mut ctx);
            }
        }
        ExpandedDevice::Virtex4(ref edev) => match edev.kind {
            prjcombine_virtex4::chip::ChipKind::Virtex4 => {
                if !opts.skip_core {
                    generic::int::collect_fuzzers(&mut ctx);
                    virtex2::clb::collect_fuzzers(&mut ctx);
                    virtex4::bram::collect_fuzzers(&mut ctx);
                    virtex4::dsp::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_clk {
                    virtex4::clk::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_misc {
                    virtex4::misc::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_io {
                    virtex4::io::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_dcm {
                    virtex4::dcm::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_ccm {
                    virtex4::ccm::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_hard {
                    virtex4::ppc::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_gt {
                    virtex4::gt::collect_fuzzers(&mut ctx);
                }
            }
            prjcombine_virtex4::chip::ChipKind::Virtex5 => {
                if !opts.skip_core {
                    generic::int::collect_fuzzers(&mut ctx);
                    virtex5::clb::collect_fuzzers(&mut ctx);
                    virtex5::bram::collect_fuzzers(&mut ctx);
                    virtex5::dsp::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_clk {
                    virtex5::clk::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_misc {
                    virtex5::misc::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_io {
                    virtex5::io::collect_fuzzers(&mut ctx, false);
                } else if opts.devdata_only {
                    virtex5::io::collect_fuzzers(&mut ctx, true);
                }
                if !opts.skip_dcm || !opts.skip_pll {
                    virtex5::cmt::collect_fuzzers(&mut ctx, opts.skip_dcm, opts.skip_pll, false);
                } else if opts.devdata_only {
                    virtex5::cmt::collect_fuzzers(&mut ctx, true, true, true);
                }
                if !opts.skip_hard {
                    virtex5::ppc::collect_fuzzers(&mut ctx, false);
                    virtex5::emac::collect_fuzzers(&mut ctx);
                    virtex5::pcie::collect_fuzzers(&mut ctx);
                } else if opts.devdata_only {
                    virtex5::ppc::collect_fuzzers(&mut ctx, true);
                }
                if !opts.skip_gt {
                    virtex5::gt::collect_fuzzers(&mut ctx);
                }
            }
            prjcombine_virtex4::chip::ChipKind::Virtex6 => {
                if !opts.skip_core {
                    generic::int::collect_fuzzers(&mut ctx);
                    virtex5::clb::collect_fuzzers(&mut ctx);
                    virtex6::bram::collect_fuzzers(&mut ctx);
                    virtex6::dsp::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_clk {
                    virtex6::clk::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_misc {
                    virtex6::misc::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_io {
                    virtex6::io::collect_fuzzers(&mut ctx, false);
                } else if opts.devdata_only {
                    virtex6::io::collect_fuzzers(&mut ctx, true);
                }
                if !opts.skip_pll {
                    virtex6::cmt::collect_fuzzers(&mut ctx, false);
                } else if opts.devdata_only {
                    virtex6::cmt::collect_fuzzers(&mut ctx, true);
                }
                if !opts.skip_hard {
                    virtex6::emac::collect_fuzzers(&mut ctx);
                    virtex6::pcie::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_gt {
                    virtex6::gtx::collect_fuzzers(&mut ctx);
                    virtex6::gth::collect_fuzzers(&mut ctx);
                }
            }
            prjcombine_virtex4::chip::ChipKind::Virtex7 => {
                if !opts.skip_core {
                    generic::int::collect_fuzzers(&mut ctx);
                    virtex5::clb::collect_fuzzers(&mut ctx);
                    virtex6::bram::collect_fuzzers(&mut ctx);
                    virtex6::dsp::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_clk {
                    virtex7::clk::collect_fuzzers(&mut ctx, false);
                } else if opts.bali_only {
                    virtex7::clk::collect_fuzzers(&mut ctx, true);
                }
                if !opts.skip_misc {
                    virtex7::misc::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_io {
                    virtex7::io::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_pll {
                    virtex7::cmt::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_hard {
                    virtex7::pcie::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_gt {
                    virtex7::gt::collect_fuzzers(&mut ctx);
                }
                if !opts.skip_gtz {
                    virtex7::gtz::collect_fuzzers(&mut ctx);
                }
            }
        },
        _ => panic!("unsupported device kind"),
    }
    if !opts.skip_core {
        generic::intf::collect_fuzzers(&mut ctx);
    }
    for (die, dbs) in &ctx.empty_bs.die {
        if let Some(&val) = dbs.regs.get(&Reg::Idcode) {
            let mut idcode = BitVec::new();
            for i in 0..32 {
                idcode.push((val & 1 << i) != 0);
            }
            ctx.tiledb
                .insert_device_data(&part.name, format!("IDCODE:{die}"), idcode);
        }
    }
    for (&dir, gtzbs) in &ctx.empty_bs.gtz {
        let which = match dir {
            DirV::S => "GTZ_BOT",
            DirV::N => "GTZ_TOP",
        };
        let mut idcode = BitVec::new();
        for i in 0..32 {
            idcode.push((gtzbs.idcode & 1 << i) != 0);
        }
        ctx.tiledb
            .insert_device_data(&part.name, format!("IDCODE:{which}"), idcode);
    }

    for (key, data) in &ctx.state.features {
        println!("{key:?}: {diffs:?}", diffs = data.diffs);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let tc = Toolchain::from_file(&args.toolchain)?;
    let db = GeomDb::from_file(args.geomdb)?;
    let mut tiledb = BsData::new();
    let opts = RunOpts {
        skip_io: args.skip_io,
        skip_clk: args.skip_clk,
        skip_ccm: args.skip_ccm,
        skip_dcm: args.skip_dcm,
        skip_pll: args.skip_pll,
        skip_misc: args.skip_misc,
        skip_gt: args.skip_gt,
        skip_gtz: args.skip_gtz,
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
        match db.chips.first().unwrap() {
            prjcombine_re_xilinx_geom::Chip::Virtex(_) => {
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
            prjcombine_re_xilinx_geom::Chip::Virtex2(grid) => match grid.kind {
                prjcombine_virtex2::chip::ChipKind::Virtex2
                | prjcombine_virtex2::chip::ChipKind::Virtex2P
                | prjcombine_virtex2::chip::ChipKind::Virtex2PX => {
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
                prjcombine_virtex2::chip::ChipKind::Spartan3
                | prjcombine_virtex2::chip::ChipKind::Spartan3E
                | prjcombine_virtex2::chip::ChipKind::Spartan3A
                | prjcombine_virtex2::chip::ChipKind::Spartan3ADsp => {
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
                prjcombine_virtex2::chip::ChipKind::FpgaCore => {
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
            prjcombine_re_xilinx_geom::Chip::Spartan6(_) => {
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
            prjcombine_re_xilinx_geom::Chip::Virtex4(grid) => match grid.kind {
                prjcombine_virtex4::chip::ChipKind::Virtex4 => {
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
                prjcombine_virtex4::chip::ChipKind::Virtex5 => {
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
                prjcombine_virtex4::chip::ChipKind::Virtex6 => {
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
                prjcombine_virtex4::chip::ChipKind::Virtex7 => {
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
                        xopts.skip_gtz = opts.skip_gtz;
                        xopts.max_threads = Some(8);
                        run(&tc, &db, parts_dict[&"xc7vx1140t"], &mut tiledb, &xopts);
                    }
                    if !opts.skip_gtz {
                        // GTZ
                        let mut xopts = opts;
                        xopts.skip_all();
                        xopts.skip_gtz = opts.skip_gtz;
                        xopts.max_threads = Some(8);
                        run(&tc, &db, parts_dict[&"xc7vh870t"], &mut tiledb, &xopts);
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
